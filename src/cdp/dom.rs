use std::collections::HashMap;

use chromiumoxide::Page;

use crate::observation::{ElementInfo, Rect, SimplifiedNode};

/// Extract simplified DOM from a page starting at an optional CSS selector.
pub async fn get_simplified_dom(
    page: &Page,
    root_selector: Option<&str>,
    max_depth: u32,
    include_hidden: bool,
    max_nodes: usize,
) -> anyhow::Result<SimplifiedNode> {
    let selector = root_selector.unwrap_or("body");

    // Use JS to extract the DOM tree
    let js = format!(
        r#"
        (function() {{
            function extractNode(el, depth, maxDepth, includeHidden, count, maxNodes) {{
                if (count[0] >= maxNodes) return null;
                count[0]++;
                
                var style = window.getComputedStyle ? window.getComputedStyle(el) : null;
                var visible = includeHidden || (style && style.display !== 'none' && style.visibility !== 'hidden');
                
                var attrs = {{}};
                var allowedAttrs = ['id','class','href','role','name','value','type','src','alt','placeholder',
                                    'aria-label','aria-hidden','aria-expanded','aria-selected','aria-checked',
                                    'aria-disabled','data-testid','for','action','method'];
                for (var i = 0; i < el.attributes.length; i++) {{
                    var a = el.attributes[i];
                    if (allowedAttrs.indexOf(a.name) !== -1 || a.name.startsWith('aria-')) {{
                        attrs[a.name] = a.value;
                    }}
                }}
                
                var rect = null;
                try {{
                    var br = el.getBoundingClientRect();
                    if (br.width > 0 || br.height > 0) {{
                        rect = {{ x: br.x, y: br.y, width: br.width, height: br.height }};
                    }}
                }} catch(e) {{}}
                
                var directText = null;
                for (var j = 0; j < el.childNodes.length; j++) {{
                    if (el.childNodes[j].nodeType === 3) {{
                        var t = el.childNodes[j].textContent.trim();
                        if (t) {{ directText = (directText || '') + t; }}
                    }}
                }}
                
                var children = [];
                if (depth < maxDepth) {{
                    for (var k = 0; k < el.children.length; k++) {{
                        var child = extractNode(el.children[k], depth+1, maxDepth, includeHidden, count, maxNodes);
                        if (child) children.push(child);
                        if (count[0] >= maxNodes) break;
                    }}
                }}
                
                return {{
                    tag: el.tagName.toLowerCase(),
                    attributes: attrs,
                    text: directText,
                    visible: visible,
                    bounding_box: rect,
                    children: children
                }};
            }}
            
            var root = document.querySelector({});
            if (!root) return null;
            var count = [0];
            return extractNode(root, 0, {}, {}, count, {});
        }})()
        "#,
        serde_json::json!(selector),
        max_depth,
        if include_hidden { "true" } else { "false" },
        max_nodes,
    );

    let result = page.evaluate(js).await?;
    let value = result.into_value::<serde_json::Value>()?;

    if value.is_null() {
        return Err(anyhow::anyhow!("Element not found: {}", selector));
    }

    parse_simplified_node(&value, 0)
}

fn parse_simplified_node(v: &serde_json::Value, backend_id: i64) -> anyhow::Result<SimplifiedNode> {
    let tag = v["tag"].as_str().unwrap_or("unknown").to_string();
    let mut attributes = HashMap::new();
    if let Some(attrs) = v["attributes"].as_object() {
        for (k, val) in attrs {
            if let Some(s) = val.as_str() {
                attributes.insert(k.clone(), s.to_string());
            }
        }
    }
    let text = v["text"].as_str().map(|s| s.to_string());
    let visible = v["visible"].as_bool().unwrap_or(true);

    let bounding_box = if let Some(bb) = v["bounding_box"].as_object() {
        Some(Rect {
            x: bb["x"].as_f64().unwrap_or(0.0),
            y: bb["y"].as_f64().unwrap_or(0.0),
            width: bb["width"].as_f64().unwrap_or(0.0),
            height: bb["height"].as_f64().unwrap_or(0.0),
        })
    } else {
        None
    };

    let children = if let Some(arr) = v["children"].as_array() {
        arr.iter()
            .filter_map(|c| parse_simplified_node(c, 0).ok())
            .collect()
    } else {
        vec![]
    };

    Ok(SimplifiedNode {
        backend_node_id: backend_id,
        tag,
        attributes,
        text,
        visible,
        bounding_box,
        children,
    })
}

/// Extract links from a page within an optional scope selector.
pub async fn extract_links(
    page: &Page,
    scope_selector: Option<&str>,
    same_origin_only: bool,
) -> anyhow::Result<Vec<crate::observation::LinkInfo>> {
    let scope = scope_selector.unwrap_or("document");
    let js = format!(
        r#"
        (function() {{
            var root = {};
            if (!root) root = document;
            var links = root.querySelectorAll ? root.querySelectorAll('a[href]') : document.querySelectorAll('a[href]');
            var result = [];
            var origin = window.location.origin;
            for (var i = 0; i < links.length; i++) {{
                var l = links[i];
                var href = l.href;
                if ({} && href && !href.startsWith(origin)) continue;
                var style = window.getComputedStyle ? window.getComputedStyle(l) : null;
                var visible = !style || (style.display !== 'none' && style.visibility !== 'hidden');
                result.push({{ href: href, text: l.textContent.trim(), visible: visible }});
            }}
            return result;
        }})()
        "#,
        if scope == "document" { "document".to_string() } else { format!("document.querySelector({})", serde_json::json!(scope)) },
        if same_origin_only { "true" } else { "false" },
    );

    let result = page.evaluate(js).await?;
    let value = result.into_value::<serde_json::Value>()?;

    let mut links = Vec::new();
    if let Some(arr) = value.as_array() {
        for item in arr {
            links.push(crate::observation::LinkInfo {
                href: item["href"].as_str().unwrap_or("").to_string(),
                text: item["text"].as_str().unwrap_or("").to_string(),
                visible: item["visible"].as_bool().unwrap_or(true),
            });
        }
    }
    Ok(links)
}

/// Query elements matching a CSS selector.
pub async fn query_elements(
    page: &Page,
    selector: &str,
    limit: usize,
) -> anyhow::Result<Vec<ElementInfo>> {
    let js = format!(
        r#"
        (function() {{
            var els = document.querySelectorAll({});
            var result = [];
            var limit = {};
            for (var i = 0; i < Math.min(els.length, limit); i++) {{
                var el = els[i];
                var attrs = {{}};
                var allowedAttrs = ['id','class','href','role','name','value','type','src','alt','placeholder'];
                for (var j = 0; j < el.attributes.length; j++) {{
                    var a = el.attributes[j];
                    if (allowedAttrs.indexOf(a.name) !== -1) attrs[a.name] = a.value;
                }}
                var style = window.getComputedStyle ? window.getComputedStyle(el) : null;
                var visible = !style || (style.display !== 'none' && style.visibility !== 'hidden');
                var rect = null;
                try {{
                    var br = el.getBoundingClientRect();
                    rect = {{ x: br.x, y: br.y, width: br.width, height: br.height }};
                }} catch(e) {{}}
                result.push({{
                    tag: el.tagName.toLowerCase(),
                    text: el.textContent.trim().substring(0, 200),
                    attributes: attrs,
                    visible: visible,
                    bounding_box: rect
                }});
            }}
            return result;
        }})()
        "#,
        serde_json::json!(selector),
        limit,
    );

    let result = page.evaluate(js).await?;
    let value = result.into_value::<serde_json::Value>()?;

    let mut elements = Vec::new();
    if let Some(arr) = value.as_array() {
        for (idx, item) in arr.iter().enumerate() {
            let mut attrs = HashMap::new();
            if let Some(a) = item["attributes"].as_object() {
                for (k, v) in a {
                    if let Some(s) = v.as_str() {
                        attrs.insert(k.clone(), s.to_string());
                    }
                }
            }
            let bounding_box = if let Some(bb) = item["bounding_box"].as_object() {
                Some(Rect {
                    x: bb["x"].as_f64().unwrap_or(0.0),
                    y: bb["y"].as_f64().unwrap_or(0.0),
                    width: bb["width"].as_f64().unwrap_or(0.0),
                    height: bb["height"].as_f64().unwrap_or(0.0),
                })
            } else {
                None
            };
            elements.push(ElementInfo {
                backend_node_id: idx as i64,
                tag: item["tag"].as_str().unwrap_or("unknown").to_string(),
                text: item["text"].as_str().map(|s| s.to_string()),
                attributes: attrs,
                bounding_box,
                visible: item["visible"].as_bool().unwrap_or(true),
            });
        }
    }
    Ok(elements)
}

use chromiumoxide::Page;

use crate::observation::AxNode;

/// Extract the accessibility tree from a page.
pub async fn get_ax_tree(
    page: &Page,
    _root_selector: Option<&str>,
    max_depth: u32,
) -> anyhow::Result<AxNode> {
    // Use JS to extract accessibility-relevant info since AX tree CDP domain may not be available
    let js = format!(
        r#"
        (function() {{
            function getRole(el) {{
                var role = el.getAttribute('role');
                if (role) return role;
                var tag = el.tagName.toLowerCase();
                var roleMap = {{
                    'a': 'link', 'button': 'button', 'input': 'textbox',
                    'select': 'combobox', 'textarea': 'textbox', 'img': 'image',
                    'nav': 'navigation', 'main': 'main', 'header': 'banner',
                    'footer': 'contentinfo', 'aside': 'complementary',
                    'section': 'region', 'article': 'article', 'form': 'form',
                    'h1': 'heading', 'h2': 'heading', 'h3': 'heading',
                    'h4': 'heading', 'h5': 'heading', 'h6': 'heading',
                    'ul': 'list', 'ol': 'list', 'li': 'listitem',
                    'table': 'table', 'tr': 'row', 'td': 'cell', 'th': 'columnheader',
                    'checkbox': 'checkbox', 'radio': 'radio',
                    'dialog': 'dialog', 'menu': 'menu', 'menuitem': 'menuitem',
                }};
                if (tag === 'input') {{
                    var type = (el.getAttribute('type') || 'text').toLowerCase();
                    if (type === 'checkbox') return 'checkbox';
                    if (type === 'radio') return 'radio';
                    if (type === 'button' || type === 'submit') return 'button';
                    return 'textbox';
                }}
                return roleMap[tag] || tag;
            }}
            
            function getName(el) {{
                return el.getAttribute('aria-label') || 
                       el.getAttribute('alt') || 
                       el.getAttribute('placeholder') ||
                       el.getAttribute('title') ||
                       (el.textContent ? el.textContent.trim().substring(0, 100) : null) ||
                       null;
            }}
            
            function extractAx(el, depth, maxDepth, id) {{
                if (depth > maxDepth) return null;
                
                var style = window.getComputedStyle ? window.getComputedStyle(el) : null;
                var hidden = style && (style.display === 'none' || style.visibility === 'hidden');
                var ariaHidden = el.getAttribute('aria-hidden') === 'true';
                if (hidden || ariaHidden) return null;
                
                var role = getRole(el);
                var name = getName(el);
                var value = el.value !== undefined ? String(el.value) : null;
                var focusable = el.tabIndex >= 0 || ['a','button','input','select','textarea'].indexOf(el.tagName.toLowerCase()) !== -1;
                var focused = document.activeElement === el;
                var disabled = el.disabled || el.getAttribute('aria-disabled') === 'true';
                
                var children = [];
                if (depth < maxDepth) {{
                    for (var i = 0; i < el.children.length; i++) {{
                        var child = extractAx(el.children[i], depth+1, maxDepth, id+'-'+i);
                        if (child) children.push(child);
                    }}
                }}
                
                // Only include nodes that are meaningful 
                var meaningful = focusable || role !== el.tagName.toLowerCase() || 
                                 (name && name.length > 0) || children.length > 0;
                if (!meaningful && depth > 2) return null;
                
                el.setAttribute('data-ax-node-id', id);
                
                return {{
                    ax_node_id: id,
                    role: role,
                    name: name,
                    value: value,
                    focusable: focusable,
                    focused: focused,
                    disabled: disabled,
                    children: children
                }};
            }}
            
            var root = document.body;
            return extractAx(root, 0, {}, 'root');
        }})()
        "#,
        max_depth,
    );

    let result = page.evaluate(js).await?;
    let value = result.into_value::<serde_json::Value>()?;

    if value.is_null() {
        return Ok(AxNode {
            ax_node_id: "root".to_string(),
            role: "document".to_string(),
            name: None,
            value: None,
            focusable: false,
            focused: false,
            disabled: false,
            children: vec![],
        });
    }

    parse_ax_node(&value)
}

fn parse_ax_node(v: &serde_json::Value) -> anyhow::Result<AxNode> {
    let ax_node_id = v["ax_node_id"].as_str().unwrap_or("unknown").to_string();
    let role = v["role"].as_str().unwrap_or("unknown").to_string();
    let name = v["name"].as_str().map(|s| s.to_string());
    let value = v["value"].as_str().map(|s| s.to_string());
    let focusable = v["focusable"].as_bool().unwrap_or(false);
    let focused = v["focused"].as_bool().unwrap_or(false);
    let disabled = v["disabled"].as_bool().unwrap_or(false);

    let children = if let Some(arr) = v["children"].as_array() {
        arr.iter().filter_map(|c| parse_ax_node(c).ok()).collect()
    } else {
        vec![]
    };

    Ok(AxNode {
        ax_node_id,
        role,
        name,
        value,
        focusable,
        focused,
        disabled,
        children,
    })
}

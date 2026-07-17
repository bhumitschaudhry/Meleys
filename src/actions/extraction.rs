use std::sync::Arc;

use anyhow::Result;

use crate::cdp::{ax_tree, dom};
use crate::error::MeleyError;
use crate::observation::{ActionResult, Observation};
use crate::selector::Selector;
use crate::session::SessionManager;

/// Get the text content of a page/element.
pub async fn get_text(
    session_manager: &Arc<SessionManager>,
    session_id: &str,
    tab_id: Option<&str>,
    selector: Option<&Selector>,
    max_chars: Option<usize>,
) -> Observation {
    let result: Result<(String, Option<String>, Option<String>, String)> = async {
        let session = session_manager.get_session(session_id).await?;
        let (actual_tab_id, page_lock) = if let Some(tid) = tab_id {
            let p = session.get_page(tid).await?;
            (tid.to_string(), p)
        } else {
            session.get_active_page().await?
        };
        let page = page_lock.lock().await;

        let sel_str = match selector {
            Some(Selector::Css(s)) => s.clone(),
            Some(Selector::XPath(xpath)) => {
                // Use JS to find via XPath
                let js = format!(
                    r#"(function() {{
                        var result = document.evaluate({}, document, null, XPathResult.FIRST_ORDERED_NODE_TYPE, null);
                        var el = result.singleNodeValue;
                        return el ? el.innerText || el.textContent : null;
                    }})()"#,
                    serde_json::json!(xpath)
                );
                let r = page.evaluate(js).await
                    .map_err(|e| anyhow::anyhow!(MeleyError::Internal(e.to_string())))?
                    .into_value::<Option<String>>()
                    .unwrap_or(None);
                let text = r.unwrap_or_default();
                let max = max_chars.unwrap_or(20000);
                let truncated: String = text.chars().take(max).collect();
                let url = page.url().await.ok().flatten().map(|u| u.to_string());
                let title = page.get_title().await.ok().flatten();
                return Ok((actual_tab_id, url, title, truncated));
            }
            None => "body".to_string(),
            _ => "body".to_string(),
        };

        let js = format!(
            r#"(function() {{
                var el = document.querySelector({});
                return el ? (el.innerText || el.textContent || '') : '';
            }})()"#,
            serde_json::json!(sel_str)
        );

        let text = page.evaluate(js).await
            .map_err(|e| anyhow::anyhow!(MeleyError::Internal(e.to_string())))?
            .into_value::<String>()
            .unwrap_or_default();

        let max = max_chars.unwrap_or(20000);
        let truncated: String = text.chars().take(max).collect();

        let url = page.url().await.ok().flatten().map(|u| u.to_string());
        let title = page.get_title().await.ok().flatten();
        Ok((actual_tab_id, url, title, truncated))
    }.await;

    match result {
        Ok((tid, url, title, text)) => {
            let mut obs =
                Observation::success(session_id, tid, "get_text", ActionResult::Text(text));
            obs.url = url;
            obs.title = title;
            obs
        }
        Err(e) => {
            let (code, retryable) = error_code(&e);
            Observation::failure(
                session_id,
                tab_id.unwrap_or(""),
                "get_text",
                code,
                e.to_string(),
                retryable,
            )
        }
    }
}

/// Get all links from a page.
#[allow(clippy::type_complexity)]
pub async fn get_links(
    session_manager: &Arc<SessionManager>,
    session_id: &str,
    tab_id: Option<&str>,
    scope_selector: Option<&str>,
    same_origin_only: bool,
) -> Observation {
    let result: Result<(
        String,
        Option<String>,
        Option<String>,
        Vec<crate::observation::LinkInfo>,
    )> = async {
        let session = session_manager.get_session(session_id).await?;
        let (actual_tab_id, page_lock) = if let Some(tid) = tab_id {
            let p = session.get_page(tid).await?;
            (tid.to_string(), p)
        } else {
            session.get_active_page().await?
        };
        let page = page_lock.lock().await;

        let links = dom::extract_links(&page, scope_selector, same_origin_only)
            .await
            .map_err(|e| anyhow::anyhow!(MeleyError::Internal(e.to_string())))?;

        let url = page.url().await.ok().flatten().map(|u| u.to_string());
        let title = page.get_title().await.ok().flatten();
        Ok((actual_tab_id, url, title, links))
    }
    .await;

    match result {
        Ok((tid, url, title, links)) => {
            let mut obs =
                Observation::success(session_id, tid, "get_links", ActionResult::Links(links));
            obs.url = url;
            obs.title = title;
            obs
        }
        Err(e) => {
            let (code, retryable) = error_code(&e);
            Observation::failure(
                session_id,
                tab_id.unwrap_or(""),
                "get_links",
                code,
                e.to_string(),
                retryable,
            )
        }
    }
}

/// Get simplified DOM.
pub async fn get_dom(
    session_manager: &Arc<SessionManager>,
    session_id: &str,
    tab_id: Option<&str>,
    selector: Option<&Selector>,
    max_depth: Option<u32>,
    include_hidden: Option<bool>,
    max_nodes: Option<usize>,
) -> Observation {
    let result: Result<(
        String,
        Option<String>,
        Option<String>,
        crate::observation::SimplifiedNode,
    )> = async {
        let session = session_manager.get_session(session_id).await?;
        let (actual_tab_id, page_lock) = if let Some(tid) = tab_id {
            let p = session.get_page(tid).await?;
            (tid.to_string(), p)
        } else {
            session.get_active_page().await?
        };
        let page = page_lock.lock().await;

        let max_n = max_nodes.unwrap_or(session_manager.config().limits.max_dom_nodes_per_call);
        let sel_str = selector.and_then(|s| {
            if let Selector::Css(c) = s {
                Some(c.as_str())
            } else {
                None
            }
        });

        let node = dom::get_simplified_dom(
            &page,
            sel_str,
            max_depth.unwrap_or(6),
            include_hidden.unwrap_or(false),
            max_n,
        )
        .await
        .map_err(|e| anyhow::anyhow!(MeleyError::Internal(e.to_string())))?;

        let url = page.url().await.ok().flatten().map(|u| u.to_string());
        let title = page.get_title().await.ok().flatten();
        Ok((actual_tab_id, url, title, node))
    }
    .await;

    match result {
        Ok((tid, url, title, node)) => {
            let mut obs = Observation::success(session_id, tid, "get_dom", ActionResult::Dom(node));
            obs.url = url;
            obs.title = title;
            obs
        }
        Err(e) => {
            let (code, retryable) = error_code(&e);
            Observation::failure(
                session_id,
                tab_id.unwrap_or(""),
                "get_dom",
                code,
                e.to_string(),
                retryable,
            )
        }
    }
}

/// Get accessibility tree.
pub async fn get_ax_tree(
    session_manager: &Arc<SessionManager>,
    session_id: &str,
    tab_id: Option<&str>,
    selector: Option<&Selector>,
    max_depth: Option<u32>,
) -> Observation {
    let result: Result<(
        String,
        Option<String>,
        Option<String>,
        crate::observation::AxNode,
    )> = async {
        let session = session_manager.get_session(session_id).await?;
        let (actual_tab_id, page_lock) = if let Some(tid) = tab_id {
            let p = session.get_page(tid).await?;
            (tid.to_string(), p)
        } else {
            session.get_active_page().await?
        };
        let page = page_lock.lock().await;

        let sel_str = selector.and_then(|s| {
            if let Selector::Css(c) = s {
                Some(c.as_str())
            } else {
                None
            }
        });

        let tree = ax_tree::get_ax_tree(&page, sel_str, max_depth.unwrap_or(8))
            .await
            .map_err(|e| anyhow::anyhow!(MeleyError::Internal(e.to_string())))?;

        let url = page.url().await.ok().flatten().map(|u| u.to_string());
        let title = page.get_title().await.ok().flatten();
        Ok((actual_tab_id, url, title, tree))
    }
    .await;

    match result {
        Ok((tid, url, title, tree)) => {
            let mut obs =
                Observation::success(session_id, tid, "get_ax_tree", ActionResult::AxTree(tree));
            obs.url = url;
            obs.title = title;
            obs
        }
        Err(e) => {
            let (code, retryable) = error_code(&e);
            Observation::failure(
                session_id,
                tab_id.unwrap_or(""),
                "get_ax_tree",
                code,
                e.to_string(),
                retryable,
            )
        }
    }
}

/// Query elements by CSS selector.
#[allow(clippy::type_complexity)]
pub async fn query_elements(
    session_manager: &Arc<SessionManager>,
    session_id: &str,
    tab_id: Option<&str>,
    selector: &Selector,
    limit: Option<usize>,
) -> Observation {
    let result: Result<(String, Option<String>, Option<String>, Vec<crate::observation::ElementInfo>)> = async {
        let session = session_manager.get_session(session_id).await?;
        let (actual_tab_id, page_lock) = if let Some(tid) = tab_id {
            let p = session.get_page(tid).await?;
            (tid.to_string(), p)
        } else {
            session.get_active_page().await?
        };
        let page = page_lock.lock().await;

        let css_sel = match selector {
            Selector::Css(s) => s.clone(),
            Selector::XPath(xpath) => {
                // Convert XPath to elements via JS
                let js = format!(
                    r#"(function() {{
                        var r = document.evaluate({}, document, null, XPathResult.ORDERED_NODE_SNAPSHOT_TYPE, null);
                        var result = [];
                        var lim = {};
                        for (var i=0; i<Math.min(r.snapshotLength, lim); i++) {{
                            var el = r.snapshotItem(i);
                            var attrs = {{}};
                            for (var j=0; j<el.attributes.length; j++) {{
                                attrs[el.attributes[j].name] = el.attributes[j].value;
                            }}
                            var style = window.getComputedStyle ? window.getComputedStyle(el) : null;
                            var visible = !style || (style.display !== 'none' && style.visibility !== 'hidden');
                            var rect = null;
                            try {{ var br = el.getBoundingClientRect(); rect = {{x:br.x,y:br.y,width:br.width,height:br.height}}; }} catch(e){{}}
                            result.push({{tag: el.tagName.toLowerCase(), text: el.textContent.trim().substring(0,200), attributes: attrs, visible: visible, bounding_box: rect}});
                        }}
                        return result;
                    }})()"#,
                    serde_json::json!(xpath),
                    limit.unwrap_or(50)
                );
                let val = page.evaluate(js).await
                    .map_err(|e| anyhow::anyhow!(MeleyError::Internal(e.to_string())))?
                    .into_value::<serde_json::Value>()
                    .unwrap_or(serde_json::Value::Array(vec![]));

                let elements = parse_elements_from_value(&val);
                let url = page.url().await.ok().flatten().map(|u| u.to_string());
                let title = page.get_title().await.ok().flatten();
                return Ok((actual_tab_id, url, title, elements));
            }
            _ => {
                return Err(anyhow::anyhow!(MeleyError::InvalidSelector(
                    "query_elements requires a CSS or XPath selector".to_string()
                )));
            }
        };

        let elements = dom::query_elements(&page, &css_sel, limit.unwrap_or(50)).await
            .map_err(|e| anyhow::anyhow!(MeleyError::Internal(e.to_string())))?;

        let url = page.url().await.ok().flatten().map(|u| u.to_string());
        let title = page.get_title().await.ok().flatten();
        Ok((actual_tab_id, url, title, elements))
    }.await;

    match result {
        Ok((tid, url, title, elements)) => {
            let mut obs = Observation::success(
                session_id,
                tid,
                "query_elements",
                ActionResult::Elements(elements),
            );
            obs.url = url;
            obs.title = title;
            obs
        }
        Err(e) => {
            let (code, retryable) = error_code(&e);
            Observation::failure(
                session_id,
                tab_id.unwrap_or(""),
                "query_elements",
                code,
                e.to_string(),
                retryable,
            )
        }
    }
}

/// Evaluate JavaScript.
pub async fn evaluate_js(
    session_manager: &Arc<SessionManager>,
    session_id: &str,
    tab_id: Option<&str>,
    expression: &str,
    allow_js: bool,
) -> Observation {
    if !allow_js {
        return Observation::failure(
            session_id,
            tab_id.unwrap_or(""),
            "evaluate_js",
            "JS_EVAL_DISABLED",
            "JavaScript evaluation is disabled. Set allow_evaluate_js=true in config.",
            false,
        );
    }

    let result: Result<(String, Option<String>, Option<String>, String)> = async {
        let session = session_manager.get_session(session_id).await?;
        let (actual_tab_id, page_lock) = if let Some(tid) = tab_id {
            let p = session.get_page(tid).await?;
            (tid.to_string(), p)
        } else {
            session.get_active_page().await?
        };
        let page = page_lock.lock().await;

        let val = page
            .evaluate(expression)
            .await
            .map_err(|e| anyhow::anyhow!(MeleyError::Internal(e.to_string())))?
            .into_value::<serde_json::Value>()
            .unwrap_or(serde_json::Value::Null);

        let text = serde_json::to_string(&val).unwrap_or_else(|_| "null".to_string());

        let url = page.url().await.ok().flatten().map(|u| u.to_string());
        let title = page.get_title().await.ok().flatten();
        Ok((actual_tab_id, url, title, text))
    }
    .await;

    match result {
        Ok((tid, url, title, text)) => {
            let mut obs =
                Observation::success(session_id, tid, "evaluate_js", ActionResult::Text(text));
            obs.url = url;
            obs.title = title;
            obs
        }
        Err(e) => {
            let (code, retryable) = error_code(&e);
            Observation::failure(
                session_id,
                tab_id.unwrap_or(""),
                "evaluate_js",
                code,
                e.to_string(),
                retryable,
            )
        }
    }
}

fn error_code(e: &anyhow::Error) -> (&'static str, bool) {
    if let Some(me) = e.downcast_ref::<MeleyError>() {
        (me.error_code(), me.is_retryable())
    } else {
        ("INTERNAL_ERROR", false)
    }
}

fn parse_elements_from_value(val: &serde_json::Value) -> Vec<crate::observation::ElementInfo> {
    use std::collections::HashMap;
    let mut result = Vec::new();
    if let Some(arr) = val.as_array() {
        for (idx, item) in arr.iter().enumerate() {
            let mut attrs = HashMap::new();
            if let Some(a) = item["attributes"].as_object() {
                for (k, v) in a {
                    if let Some(s) = v.as_str() {
                        attrs.insert(k.clone(), s.to_string());
                    }
                }
            }
            let bb = item["bounding_box"]
                .as_object()
                .map(|b| crate::observation::Rect {
                    x: b["x"].as_f64().unwrap_or(0.0),
                    y: b["y"].as_f64().unwrap_or(0.0),
                    width: b["width"].as_f64().unwrap_or(0.0),
                    height: b["height"].as_f64().unwrap_or(0.0),
                });
            result.push(crate::observation::ElementInfo {
                backend_node_id: idx as i64,
                tag: item["tag"].as_str().unwrap_or("unknown").to_string(),
                text: item["text"].as_str().map(|s| s.to_string()),
                attributes: attrs,
                bounding_box: bb,
                visible: item["visible"].as_bool().unwrap_or(true),
            });
        }
    }
    result
}

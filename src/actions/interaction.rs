use std::sync::Arc;

use anyhow::Result;
use chromiumoxide::layout::Point;
use tokio::time::Duration;

use crate::error::MeleyError;
use crate::observation::{ActionResult, Observation};
use crate::selector::Selector;
use crate::session::SessionManager;

/// Click on an element.
#[allow(clippy::too_many_arguments)]
pub async fn click(
    session_manager: &Arc<SessionManager>,
    session_id: &str,
    tab_id: Option<&str>,
    selector: &Selector,
    _button: Option<&str>,
    _click_count: Option<u32>,
    nth: Option<usize>,
    timeout_ms: Option<u64>,
) -> Observation {
    let timeout = Duration::from_millis(timeout_ms.unwrap_or(10000));

    let result: Result<(String, Option<String>, Option<String>)> =
        async {
            let session = session_manager.get_session(session_id).await?;
            let (actual_tab_id, page_lock) = if let Some(tid) = tab_id {
                let p = session.get_page(tid).await?;
                (tid.to_string(), p)
            } else {
                session.get_active_page().await?
            };
            let page = page_lock.lock().await;

            tokio::time::timeout(timeout, async {
                match selector {
                    Selector::Coordinates { x, y } => {
                        page.move_mouse(Point { x: *x, y: *y })
                            .await
                            .map_err(|e| anyhow::anyhow!(MeleyError::Internal(e.to_string())))?;
                        page.click(Point { x: *x, y: *y })
                            .await
                            .map_err(|e| anyhow::anyhow!(MeleyError::Internal(e.to_string())))?;
                    }
                    Selector::Css(css) => {
                        let nth_idx = nth.unwrap_or(0);
                        let elements = page.find_elements(css.as_str()).await.map_err(|e| {
                            anyhow::anyhow!(MeleyError::ElementNotFound(e.to_string()))
                        })?;
                        let el = elements.get(nth_idx).ok_or_else(|| {
                            anyhow::anyhow!(MeleyError::ElementNotFound(format!(
                                "No element at index {} for selector: {}",
                                nth_idx, css
                            )))
                        })?;
                        el.scroll_into_view()
                            .await
                            .map_err(|e| anyhow::anyhow!(MeleyError::Internal(e.to_string())))?;
                        el.click().await.map_err(|e| {
                            anyhow::anyhow!(MeleyError::ElementNotInteractable(e.to_string()))
                        })?;
                    }
                    Selector::XPath(xpath) => {
                        let nth_idx = nth.unwrap_or(0);
                        let elements = page.find_xpaths(xpath.as_str()).await.map_err(|e| {
                            anyhow::anyhow!(MeleyError::ElementNotFound(e.to_string()))
                        })?;
                        let el = elements.get(nth_idx).ok_or_else(|| {
                            anyhow::anyhow!(MeleyError::ElementNotFound(format!(
                                "No element at index {} for XPath: {}",
                                nth_idx, xpath
                            )))
                        })?;
                        el.scroll_into_view()
                            .await
                            .map_err(|e| anyhow::anyhow!(MeleyError::Internal(e.to_string())))?;
                        el.click().await.map_err(|e| {
                            anyhow::anyhow!(MeleyError::ElementNotInteractable(e.to_string()))
                        })?;
                    }
                    _ => {
                        // Fall back to JS-based click
                        let js = build_click_js(selector, nth.unwrap_or(0));
                        let result = page
                            .evaluate(js)
                            .await
                            .map_err(|e| anyhow::anyhow!(MeleyError::Internal(e.to_string())))?;
                        let found = result.into_value::<bool>().unwrap_or(false);
                        if !found {
                            return Err(anyhow::anyhow!(MeleyError::ElementNotFound(
                                selector.description()
                            )));
                        }
                    }
                }
                Ok::<(), anyhow::Error>(())
            })
            .await
            .map_err(|_| anyhow::anyhow!(MeleyError::Timeout("click timed out".to_string())))??;

            let url = page.url().await.ok().flatten();
            let title = page.get_title().await.ok().flatten();
            Ok((actual_tab_id, url, title))
        }
        .await;

    match result {
        Ok((tid, url, title)) => {
            let mut obs = Observation::success(session_id, tid, "click", ActionResult::Empty);
            obs.url = url;
            obs.title = title;
            obs
        }
        Err(e) => {
            let (code, retryable) = error_code(&e);
            Observation::failure(
                session_id,
                tab_id.unwrap_or(""),
                "click",
                code,
                e.to_string(),
                retryable,
            )
        }
    }
}

fn build_click_js(selector: &Selector, nth: usize) -> String {
    let find_expr = build_find_expr(selector, nth);
    format!(
        r#"(function() {{
            var el = {};
            if (!el) return false;
            el.scrollIntoView({{behavior: 'instant', block: 'center'}});
            el.click();
            return true;
        }})()"#,
        find_expr
    )
}

/// Type text into an element.
#[allow(clippy::too_many_arguments)]
pub async fn type_text(
    session_manager: &Arc<SessionManager>,
    session_id: &str,
    tab_id: Option<&str>,
    selector: &Selector,
    text: &str,
    clear_first: bool,
    delay_ms: Option<u64>,
    timeout_ms: Option<u64>,
) -> Observation {
    let timeout = Duration::from_millis(timeout_ms.unwrap_or(10000));

    let result: Result<(String, Option<String>, Option<String>)> = async {
        let session = session_manager.get_session(session_id).await?;
        let (actual_tab_id, page_lock) = if let Some(tid) = tab_id {
            let p = session.get_page(tid).await?;
            (tid.to_string(), p)
        } else {
            session.get_active_page().await?
        };
        let page = page_lock.lock().await;

        tokio::time::timeout(timeout, async {
            let element = match selector {
                Selector::Css(css) => page
                    .find_element(css.as_str())
                    .await
                    .map_err(|e| anyhow::anyhow!(MeleyError::ElementNotFound(e.to_string())))?,
                Selector::XPath(xpath) => page
                    .find_xpath(xpath.as_str())
                    .await
                    .map_err(|e| anyhow::anyhow!(MeleyError::ElementNotFound(e.to_string())))?,
                _ => {
                    // JS focus
                    let js = format!(
                        r#"(function() {{
                            var el = {};
                            if (!el) return false;
                            el.focus();
                            if ({}) {{
                                if ('value' in el) el.value = '';
                                else el.textContent = '';
                            }}
                            return true;
                        }})()"#,
                        build_find_expr(selector, 0),
                        if clear_first { "true" } else { "false" }
                    );
                    page.evaluate(js)
                        .await
                        .map_err(|e| anyhow::anyhow!(MeleyError::Internal(e.to_string())))?;
                    // Use keyboard type for fallback
                    use chromiumoxide::cdp::browser_protocol::input::{
                        DispatchKeyEventParams, DispatchKeyEventType,
                    };
                    for ch in text.chars() {
                        let key = ch.to_string();
                        page.execute(
                            DispatchKeyEventParams::builder()
                                .r#type(DispatchKeyEventType::KeyDown)
                                .text(key.clone())
                                .build()
                                .unwrap(),
                        )
                        .await
                        .ok();
                        page.execute(
                            DispatchKeyEventParams::builder()
                                .r#type(DispatchKeyEventType::Char)
                                .text(key.clone())
                                .build()
                                .unwrap(),
                        )
                        .await
                        .ok();
                        page.execute(
                            DispatchKeyEventParams::builder()
                                .r#type(DispatchKeyEventType::KeyUp)
                                .text(key)
                                .build()
                                .unwrap(),
                        )
                        .await
                        .ok();
                    }
                    return Ok::<(), anyhow::Error>(());
                }
            };

            element
                .focus()
                .await
                .map_err(|e| anyhow::anyhow!(MeleyError::Internal(e.to_string())))?;

            if clear_first {
                page.evaluate(
                    r#"(function() {
                        var el = document.activeElement;
                        if (el && 'value' in el) el.value = '';
                        return true;
                    })()"#,
                )
                .await
                .ok();
            }

            element
                .type_str(text)
                .await
                .map_err(|e| anyhow::anyhow!(MeleyError::ElementNotInteractable(e.to_string())))?;

            if let Some(delay) = delay_ms {
                if delay > 0 {
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }
            }

            Ok::<(), anyhow::Error>(())
        })
        .await
        .map_err(|_| anyhow::anyhow!(MeleyError::Timeout("type_text timed out".to_string())))??;

        let url = page.url().await.ok().flatten();
        let title = page.get_title().await.ok().flatten();
        Ok((actual_tab_id, url, title))
    }
    .await;

    match result {
        Ok((tid, url, title)) => {
            let mut obs = Observation::success(session_id, tid, "type_text", ActionResult::Empty);
            obs.url = url;
            obs.title = title;
            obs
        }
        Err(e) => {
            let (code, retryable) = error_code(&e);
            Observation::failure(
                session_id,
                tab_id.unwrap_or(""),
                "type_text",
                code,
                e.to_string(),
                retryable,
            )
        }
    }
}

/// Press a keyboard key.
pub async fn press_key(
    session_manager: &Arc<SessionManager>,
    session_id: &str,
    tab_id: Option<&str>,
    key: &str,
    selector: Option<&Selector>,
    timeout_ms: Option<u64>,
) -> Observation {
    let timeout = Duration::from_millis(timeout_ms.unwrap_or(10000));

    let result: Result<(String, Option<String>, Option<String>)> = async {
        let session = session_manager.get_session(session_id).await?;
        let (actual_tab_id, page_lock) = if let Some(tid) = tab_id {
            let p = session.get_page(tid).await?;
            (tid.to_string(), p)
        } else {
            session.get_active_page().await?
        };
        let page = page_lock.lock().await;

        tokio::time::timeout(timeout, async {
            if let Some(sel) = selector {
                // Focus element
                match sel {
                    Selector::Css(css) => {
                        if let Ok(el) = page.find_element(css.as_str()).await {
                            el.focus().await.ok();
                            el.press_key(key).await.map_err(|e| {
                                anyhow::anyhow!(MeleyError::Internal(e.to_string()))
                            })?;
                            return Ok::<(), anyhow::Error>(());
                        }
                    }
                    Selector::XPath(xpath) => {
                        if let Ok(el) = page.find_xpath(xpath.as_str()).await {
                            el.focus().await.ok();
                            el.press_key(key).await.map_err(|e| {
                                anyhow::anyhow!(MeleyError::Internal(e.to_string()))
                            })?;
                            return Ok::<(), anyhow::Error>(());
                        }
                    }
                    _ => {}
                }
            }

            // Send key to page
            use chromiumoxide::cdp::browser_protocol::input::{
                DispatchKeyEventParams, DispatchKeyEventType,
            };
            page.execute(
                DispatchKeyEventParams::builder()
                    .r#type(DispatchKeyEventType::KeyDown)
                    .key(key.to_string())
                    .build()
                    .unwrap(),
            )
            .await
            .map_err(|e| anyhow::anyhow!(MeleyError::Internal(e.to_string())))?;
            page.execute(
                DispatchKeyEventParams::builder()
                    .r#type(DispatchKeyEventType::KeyUp)
                    .key(key.to_string())
                    .build()
                    .unwrap(),
            )
            .await
            .map_err(|e| anyhow::anyhow!(MeleyError::Internal(e.to_string())))?;

            Ok::<(), anyhow::Error>(())
        })
        .await
        .map_err(|_| anyhow::anyhow!(MeleyError::Timeout("press_key timed out".to_string())))??;

        let url = page.url().await.ok().flatten();
        let title = page.get_title().await.ok().flatten();
        Ok((actual_tab_id, url, title))
    }
    .await;

    match result {
        Ok((tid, url, title)) => {
            let mut obs = Observation::success(session_id, tid, "press_key", ActionResult::Empty);
            obs.url = url;
            obs.title = title;
            obs
        }
        Err(e) => {
            let (code, retryable) = error_code(&e);
            Observation::failure(
                session_id,
                tab_id.unwrap_or(""),
                "press_key",
                code,
                e.to_string(),
                retryable,
            )
        }
    }
}

/// Hover over an element.
pub async fn hover(
    session_manager: &Arc<SessionManager>,
    session_id: &str,
    tab_id: Option<&str>,
    selector: &Selector,
    timeout_ms: Option<u64>,
) -> Observation {
    let timeout = Duration::from_millis(timeout_ms.unwrap_or(10000));

    let result: Result<(String, Option<String>, Option<String>)> =
        async {
            let session = session_manager.get_session(session_id).await?;
            let (actual_tab_id, page_lock) = if let Some(tid) = tab_id {
                let p = session.get_page(tid).await?;
                (tid.to_string(), p)
            } else {
                session.get_active_page().await?
            };
            let page = page_lock.lock().await;

            tokio::time::timeout(timeout, async {
                match selector {
                    Selector::Coordinates { x, y } => {
                        page.move_mouse(Point { x: *x, y: *y })
                            .await
                            .map_err(|e| anyhow::anyhow!(MeleyError::Internal(e.to_string())))?;
                    }
                    Selector::Css(css) => {
                        let el = page.find_element(css.as_str()).await.map_err(|e| {
                            anyhow::anyhow!(MeleyError::ElementNotFound(e.to_string()))
                        })?;
                        el.hover()
                            .await
                            .map_err(|e| anyhow::anyhow!(MeleyError::Internal(e.to_string())))?;
                    }
                    Selector::XPath(xpath) => {
                        let el = page.find_xpath(xpath.as_str()).await.map_err(|e| {
                            anyhow::anyhow!(MeleyError::ElementNotFound(e.to_string()))
                        })?;
                        el.hover()
                            .await
                            .map_err(|e| anyhow::anyhow!(MeleyError::Internal(e.to_string())))?;
                    }
                    _ => {
                        // JS hover
                        let js = format!(
                            r#"(function() {{
                            var el = {};
                            if (!el) return null;
                            var r = el.getBoundingClientRect();
                            el.dispatchEvent(new MouseEvent('mouseover', {{bubbles: true}}));
                            el.dispatchEvent(new MouseEvent('mouseenter', {{bubbles: true}}));
                            return {{ x: r.x + r.width/2, y: r.y + r.height/2 }};
                        }})()"#,
                            build_find_expr(selector, 0)
                        );
                        let val = page
                            .evaluate(js)
                            .await
                            .map_err(|e| anyhow::anyhow!(MeleyError::Internal(e.to_string())))?
                            .into_value::<serde_json::Value>()
                            .unwrap_or(serde_json::Value::Null);
                        if !val.is_null() {
                            let x = val["x"].as_f64().unwrap_or(0.0);
                            let y = val["y"].as_f64().unwrap_or(0.0);
                            page.move_mouse(Point { x, y }).await.map_err(|e| {
                                anyhow::anyhow!(MeleyError::Internal(e.to_string()))
                            })?;
                        }
                    }
                }
                Ok::<(), anyhow::Error>(())
            })
            .await
            .map_err(|_| anyhow::anyhow!(MeleyError::Timeout("hover timed out".to_string())))??;

            let url = page.url().await.ok().flatten();
            let title = page.get_title().await.ok().flatten();
            Ok((actual_tab_id, url, title))
        }
        .await;

    match result {
        Ok((tid, url, title)) => {
            let mut obs = Observation::success(session_id, tid, "hover", ActionResult::Empty);
            obs.url = url;
            obs.title = title;
            obs
        }
        Err(e) => {
            let (code, retryable) = error_code(&e);
            Observation::failure(
                session_id,
                tab_id.unwrap_or(""),
                "hover",
                code,
                e.to_string(),
                retryable,
            )
        }
    }
}

/// Scroll the page.
#[allow(clippy::too_many_arguments)]
pub async fn scroll(
    session_manager: &Arc<SessionManager>,
    session_id: &str,
    tab_id: Option<&str>,
    direction: Option<&str>,
    amount_px: Option<i64>,
    selector: Option<&Selector>,
    to_bottom: bool,
    timeout_ms: Option<u64>,
) -> Observation {
    let timeout = Duration::from_millis(timeout_ms.unwrap_or(10000));

    let result: Result<(String, Option<String>, Option<String>)> = async {
        let session = session_manager.get_session(session_id).await?;
        let (actual_tab_id, page_lock) = if let Some(tid) = tab_id {
            let p = session.get_page(tid).await?;
            (tid.to_string(), p)
        } else {
            session.get_active_page().await?
        };
        let page = page_lock.lock().await;

        tokio::time::timeout(timeout, async {
            if let Some(sel) = selector {
                // Scroll element into view
                match sel {
                    Selector::Css(css) => {
                        if let Ok(el) = page.find_element(css.as_str()).await {
                            el.scroll_into_view().await.ok();
                        }
                    }
                    _ => {
                        let js = format!(
                            r#"(function() {{
                                var el = {};
                                if (!el) return false;
                                el.scrollIntoView({{behavior: 'smooth', block: 'center'}});
                                return true;
                            }})()"#,
                            build_find_expr(sel, 0)
                        );
                        page.evaluate(js).await.ok();
                    }
                }
            } else {
                let js = if to_bottom {
                    "window.scrollTo(0, document.body.scrollHeight); true".to_string()
                } else {
                    let (dx, dy) = match direction.unwrap_or("down") {
                        "up" => (0, -(amount_px.unwrap_or(300))),
                        "down" => (0, amount_px.unwrap_or(300)),
                        "left" => (-(amount_px.unwrap_or(300)), 0),
                        "right" => (amount_px.unwrap_or(300), 0),
                        _ => (0, amount_px.unwrap_or(300)),
                    };
                    format!("window.scrollBy({}, {}); true", dx, dy)
                };
                page.evaluate(js)
                    .await
                    .map_err(|e| anyhow::anyhow!(MeleyError::Internal(e.to_string())))?;
            }
            Ok::<(), anyhow::Error>(())
        })
        .await
        .map_err(|_| anyhow::anyhow!(MeleyError::Timeout("scroll timed out".to_string())))??;

        let url = page.url().await.ok().flatten();
        let title = page.get_title().await.ok().flatten();
        Ok((actual_tab_id, url, title))
    }
    .await;

    match result {
        Ok((tid, url, title)) => {
            let mut obs = Observation::success(session_id, tid, "scroll", ActionResult::Empty);
            obs.url = url;
            obs.title = title;
            obs
        }
        Err(e) => {
            let (code, retryable) = error_code(&e);
            Observation::failure(
                session_id,
                tab_id.unwrap_or(""),
                "scroll",
                code,
                e.to_string(),
                retryable,
            )
        }
    }
}

/// Select an option in a <select> element.
pub async fn select_option(
    session_manager: &Arc<SessionManager>,
    session_id: &str,
    tab_id: Option<&str>,
    selector: &Selector,
    value: &str,
    timeout_ms: Option<u64>,
) -> Observation {
    let timeout = Duration::from_millis(timeout_ms.unwrap_or(10000));

    let result: Result<(String, Option<String>, Option<String>)> = async {
        let session = session_manager.get_session(session_id).await?;
        let (actual_tab_id, page_lock) = if let Some(tid) = tab_id {
            let p = session.get_page(tid).await?;
            (tid.to_string(), p)
        } else {
            session.get_active_page().await?
        };
        let page = page_lock.lock().await;

        tokio::time::timeout(timeout, async {
            let js = format!(
                r#"(function() {{
                    var el = {};
                    if (!el || el.tagName.toLowerCase() !== 'select') return false;
                    el.value = {};
                    el.dispatchEvent(new Event('change', {{bubbles: true}}));
                    return true;
                }})()"#,
                build_find_expr(selector, 0),
                serde_json::json!(value)
            );
            let ok = page
                .evaluate(js)
                .await
                .map_err(|e| anyhow::anyhow!(MeleyError::Internal(e.to_string())))?
                .into_value::<bool>()
                .unwrap_or(false);
            if !ok {
                return Err(anyhow::anyhow!(MeleyError::ElementNotFound(
                    selector.description()
                )));
            }
            Ok::<(), anyhow::Error>(())
        })
        .await
        .map_err(|_| {
            anyhow::anyhow!(MeleyError::Timeout("select_option timed out".to_string()))
        })??;

        let url = page.url().await.ok().flatten();
        let title = page.get_title().await.ok().flatten();
        Ok((actual_tab_id, url, title))
    }
    .await;

    match result {
        Ok((tid, url, title)) => {
            let mut obs =
                Observation::success(session_id, tid, "select_option", ActionResult::Empty);
            obs.url = url;
            obs.title = title;
            obs
        }
        Err(e) => {
            let (code, retryable) = error_code(&e);
            Observation::failure(
                session_id,
                tab_id.unwrap_or(""),
                "select_option",
                code,
                e.to_string(),
                retryable,
            )
        }
    }
}

/// Set a file input element's files.
pub async fn set_file_input(
    session_manager: &Arc<SessionManager>,
    session_id: &str,
    tab_id: Option<&str>,
    selector: &Selector,
    file_paths: Vec<String>,
    timeout_ms: Option<u64>,
) -> Observation {
    let timeout = Duration::from_millis(timeout_ms.unwrap_or(10000));

    let result: Result<(String, Option<String>, Option<String>)> = async {
        let session = session_manager.get_session(session_id).await?;
        let (actual_tab_id, page_lock) = if let Some(tid) = tab_id {
            let p = session.get_page(tid).await?;
            (tid.to_string(), p)
        } else {
            session.get_active_page().await?
        };
        let page = page_lock.lock().await;

        tokio::time::timeout(timeout, async {
            let css_sel = match selector {
                Selector::Css(s) => s.clone(),
                _ => {
                    return Err(anyhow::anyhow!(MeleyError::InvalidSelector(
                        "set_file_input requires a CSS selector".to_string()
                    )));
                }
            };

            // Use CDP DOM.setFileInputFiles via JS File API workaround
            // chromiumoxide Element doesn't have set_files, use CDP directly
            use chromiumoxide::cdp::browser_protocol::dom::{
                GetDocumentParams, QuerySelectorParams, SetFileInputFilesParams,
            };

            let doc = page
                .execute(GetDocumentParams::default())
                .await
                .map_err(|e| anyhow::anyhow!(MeleyError::Internal(e.to_string())))?;
            let node_id = doc.result.root.node_id;

            let query_result = page
                .execute(
                    QuerySelectorParams::builder()
                        .node_id(node_id)
                        .selector(css_sel.clone())
                        .build()
                        .unwrap(),
                )
                .await
                .map_err(|_| anyhow::anyhow!(MeleyError::ElementNotFound(css_sel.clone())))?;

            let el_node_id = query_result.result.node_id;

            page.execute(
                SetFileInputFilesParams::builder()
                    .files(file_paths.iter().map(|p| p.as_str()).collect::<Vec<_>>())
                    .node_id(el_node_id)
                    .build()
                    .unwrap(),
            )
            .await
            .map_err(|e| anyhow::anyhow!(MeleyError::Internal(e.to_string())))?;

            Ok::<(), anyhow::Error>(())
        })
        .await
        .map_err(|_| {
            anyhow::anyhow!(MeleyError::Timeout("set_file_input timed out".to_string()))
        })??;

        let url = page.url().await.ok().flatten();
        let title = page.get_title().await.ok().flatten();
        Ok((actual_tab_id, url, title))
    }
    .await;

    match result {
        Ok((tid, url, title)) => {
            let mut obs =
                Observation::success(session_id, tid, "set_file_input", ActionResult::Empty);
            obs.url = url;
            obs.title = title;
            obs
        }
        Err(e) => {
            let (code, retryable) = error_code(&e);
            Observation::failure(
                session_id,
                tab_id.unwrap_or(""),
                "set_file_input",
                code,
                e.to_string(),
                retryable,
            )
        }
    }
}

pub fn build_find_expr(selector: &Selector, nth: usize) -> String {
    match selector {
        Selector::Css(css) => format!(
            "document.querySelectorAll({})[{}]",
            serde_json::json!(css),
            nth
        ),
        Selector::XPath(xpath) => format!(
            r#"(function() {{
                var r = document.evaluate({}, document, null, XPathResult.ORDERED_NODE_SNAPSHOT_TYPE, null);
                return r.snapshotItem({});
            }})()"#,
            serde_json::json!(xpath),
            nth
        ),
        Selector::Text { exact, value } => {
            if *exact {
                format!(
                    r#"(function() {{
                        var all = document.querySelectorAll('*');
                        var matches = [];
                        for (var i=0;i<all.length;i++) {{
                            if (all[i].children.length === 0 && all[i].textContent.trim() === {}) matches.push(all[i]);
                        }}
                        return matches[{}] || null;
                    }})()"#,
                    serde_json::json!(value),
                    nth
                )
            } else {
                format!(
                    r#"(function() {{
                        var all = document.querySelectorAll('*');
                        var matches = [];
                        for (var i=0;i<all.length;i++) {{
                            if (all[i].textContent.trim().includes({})) matches.push(all[i]);
                        }}
                        return matches[{}] || null;
                    }})()"#,
                    serde_json::json!(value),
                    nth
                )
            }
        }
        Selector::Coordinates { x, y } => format!("document.elementFromPoint({}, {})", x, y),
        Selector::AxNodeId(id) => format!("document.querySelector('[data-ax-node-id=\"{}\"]')", id),
        Selector::BackendNodeId(id) => {
            format!("document.querySelector('[data-meleys-node-id=\"{}\"]')", id)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_find_expr_css() {
        let sel = Selector::Css("#main".into());
        let expr = build_find_expr(&sel, 0);
        assert!(expr.contains("document.querySelectorAll"));
        assert!(expr.contains("#main"));
        assert!(expr.contains("[0]"));
    }

    #[test]
    fn test_build_find_expr_css_nth() {
        let sel = Selector::Css(".item".into());
        let expr = build_find_expr(&sel, 3);
        assert!(expr.contains("[3]"));
    }

    #[test]
    fn test_build_find_expr_xpath() {
        let sel = Selector::XPath("//div".into());
        let expr = build_find_expr(&sel, 0);
        assert!(expr.contains("document.evaluate"));
        assert!(expr.contains("//div"));
        assert!(expr.contains("snapshotItem"));
    }

    #[test]
    fn test_build_find_expr_text_exact() {
        let sel = Selector::Text {
            exact: true,
            value: "Submit".into(),
        };
        let expr = build_find_expr(&sel, 0);
        assert!(expr.contains("textContent.trim() ==="));
        assert!(expr.contains("Submit"));
    }

    #[test]
    fn test_build_find_expr_text_partial() {
        let sel = Selector::Text {
            exact: false,
            value: "Sub".into(),
        };
        let expr = build_find_expr(&sel, 0);
        assert!(expr.contains("textContent.trim().includes"));
        assert!(expr.contains("Sub"));
    }

    #[test]
    fn test_build_find_expr_coordinates() {
        let sel = Selector::Coordinates { x: 50.0, y: 100.0 };
        let expr = build_find_expr(&sel, 0);
        assert!(expr.contains("document.elementFromPoint"));
        assert!(expr.contains("50"));
        assert!(expr.contains("100"));
    }

    #[test]
    fn test_build_find_expr_ax_node_id() {
        let sel = Selector::AxNodeId("node-42".into());
        let expr = build_find_expr(&sel, 0);
        assert!(expr.contains("data-ax-node-id"));
        assert!(expr.contains("node-42"));
    }

    #[test]
    fn test_build_find_expr_backend_node_id() {
        let sel = Selector::BackendNodeId(99);
        let expr = build_find_expr(&sel, 0);
        assert!(expr.contains("data-meleys-node-id"));
        assert!(expr.contains("99"));
    }
}

use std::sync::Arc;

use anyhow::Result;
use chromiumoxide::Page;
use tokio::time::Duration;

use crate::error::MeleyError;
use crate::observation::{ActionResult, Observation};
use crate::selector::Selector;
use crate::session::SessionManager;

/// Navigate to a URL.
pub async fn navigate(
    session_manager: &Arc<SessionManager>,
    session_id: &str,
    tab_id: Option<&str>,
    url: &str,
    wait_until: Option<&str>,
    timeout_ms: Option<u64>,
) -> Observation {
    let timeout = Duration::from_millis(timeout_ms.unwrap_or(30000));

    let result = async {
        let session = session_manager.get_session(session_id).await?;
        let (actual_tab_id, page_lock) = if let Some(tid) = tab_id {
            let p = session.get_page(tid).await?;
            (tid.to_string(), p)
        } else {
            session.get_active_page().await?
        };

        let page = page_lock.lock().await;

        tokio::time::timeout(timeout, async {
            page.goto(url).await
                .map_err(|e| anyhow::anyhow!(MeleyError::NavigationFailed(e.to_string())))?;
            page.wait_for_navigation().await
                .map_err(|e| anyhow::anyhow!(MeleyError::NavigationFailed(e.to_string())))?;
            Ok::<(), anyhow::Error>(())
        }).await
        .map_err(|_| anyhow::anyhow!(MeleyError::Timeout(format!("Navigation to {} timed out", url))))?
        .map_err(|e| e)?;

        let current_url = page.url().await.ok().flatten().map(|u| u.to_string());
        let title = page.get_title().await.ok().flatten();

        Ok::<(String, Option<String>, Option<String>), anyhow::Error>((actual_tab_id, current_url, title))
    }.await;

    match result {
        Ok((tid, current_url, title)) => {
            let mut obs = Observation::success(session_id, tid, "navigate", ActionResult::Empty);
            obs.url = current_url;
            obs.title = title;
            obs
        }
        Err(e) => {
            let (code, retryable) = error_info(&e);
            Observation::failure(session_id, tab_id.unwrap_or(""), "navigate", code, e.to_string(), retryable)
        }
    }
}

/// Go back in history.
pub async fn go_back(
    session_manager: &Arc<SessionManager>,
    session_id: &str,
    tab_id: Option<&str>,
    timeout_ms: Option<u64>,
) -> Observation {
    let timeout = Duration::from_millis(timeout_ms.unwrap_or(10000));

    let result = async {
        let session = session_manager.get_session(session_id).await?;
        let (actual_tab_id, page_lock) = if let Some(tid) = tab_id {
            let p = session.get_page(tid).await?;
            (tid.to_string(), p)
        } else {
            session.get_active_page().await?
        };

        let page = page_lock.lock().await;
        tokio::time::timeout(timeout, async {
            page.evaluate("history.go(-1)").await
                .map_err(|e| anyhow::anyhow!(MeleyError::NavigationFailed(e.to_string())))?;
            tokio::time::sleep(Duration::from_millis(500)).await;
            Ok::<(), anyhow::Error>(())
        }).await
        .map_err(|_| anyhow::anyhow!(MeleyError::Timeout("go_back timed out".to_string())))?
        .map_err(|e| e)?;

        let current_url = page.url().await.ok().flatten().map(|u| u.to_string());
        let title = page.get_title().await.ok().flatten();
        Ok::<(String, Option<String>, Option<String>), anyhow::Error>((actual_tab_id, current_url, title))
    }.await;

    match result {
        Ok((tid, url, title)) => {
            let mut obs = Observation::success(session_id, tid, "go_back", ActionResult::Empty);
            obs.url = url;
            obs.title = title;
            obs
        }
        Err(e) => {
            let (code, retryable) = error_info(&e);
            Observation::failure(session_id, tab_id.unwrap_or(""), "go_back", code, e.to_string(), retryable)
        }
    }
}

/// Go forward in history.
pub async fn go_forward(
    session_manager: &Arc<SessionManager>,
    session_id: &str,
    tab_id: Option<&str>,
    timeout_ms: Option<u64>,
) -> Observation {
    let timeout = Duration::from_millis(timeout_ms.unwrap_or(10000));

    let result = async {
        let session = session_manager.get_session(session_id).await?;
        let (actual_tab_id, page_lock) = if let Some(tid) = tab_id {
            let p = session.get_page(tid).await?;
            (tid.to_string(), p)
        } else {
            session.get_active_page().await?
        };

        let page = page_lock.lock().await;
        tokio::time::timeout(timeout, async {
            page.evaluate("history.go(1)").await
                .map_err(|e| anyhow::anyhow!(MeleyError::NavigationFailed(e.to_string())))?;
            tokio::time::sleep(Duration::from_millis(500)).await;
            Ok::<(), anyhow::Error>(())
        }).await
        .map_err(|_| anyhow::anyhow!(MeleyError::Timeout("go_forward timed out".to_string())))?
        .map_err(|e| e)?;

        let current_url = page.url().await.ok().flatten().map(|u| u.to_string());
        let title = page.get_title().await.ok().flatten();
        Ok::<(String, Option<String>, Option<String>), anyhow::Error>((actual_tab_id, current_url, title))
    }.await;

    match result {
        Ok((tid, url, title)) => {
            let mut obs = Observation::success(session_id, tid, "go_forward", ActionResult::Empty);
            obs.url = url;
            obs.title = title;
            obs
        }
        Err(e) => {
            let (code, retryable) = error_info(&e);
            Observation::failure(session_id, tab_id.unwrap_or(""), "go_forward", code, e.to_string(), retryable)
        }
    }
}

/// Reload the current page.
pub async fn reload(
    session_manager: &Arc<SessionManager>,
    session_id: &str,
    tab_id: Option<&str>,
    ignore_cache: bool,
    timeout_ms: Option<u64>,
) -> Observation {
    let timeout = Duration::from_millis(timeout_ms.unwrap_or(30000));

    let result = async {
        let session = session_manager.get_session(session_id).await?;
        let (actual_tab_id, page_lock) = if let Some(tid) = tab_id {
            let p = session.get_page(tid).await?;
            (tid.to_string(), p)
        } else {
            session.get_active_page().await?
        };

        let page = page_lock.lock().await;
        tokio::time::timeout(timeout, async {
            if ignore_cache {
                page.evaluate("location.reload(true)").await
                    .map_err(|e| anyhow::anyhow!(MeleyError::NavigationFailed(e.to_string())))?;
            } else {
                page.reload().await
                    .map_err(|e| anyhow::anyhow!(MeleyError::NavigationFailed(e.to_string())))?;
                page.wait_for_navigation().await
                    .map_err(|e| anyhow::anyhow!(MeleyError::NavigationFailed(e.to_string())))?;
            }
            Ok::<(), anyhow::Error>(())
        }).await
        .map_err(|_| anyhow::anyhow!(MeleyError::Timeout("reload timed out".to_string())))?
        .map_err(|e| e)?;

        let current_url = page.url().await.ok().flatten().map(|u| u.to_string());
        let title = page.get_title().await.ok().flatten();
        Ok::<(String, Option<String>, Option<String>), anyhow::Error>((actual_tab_id, current_url, title))
    }.await;

    match result {
        Ok((tid, url, title)) => {
            let mut obs = Observation::success(session_id, tid, "reload", ActionResult::Empty);
            obs.url = url;
            obs.title = title;
            obs
        }
        Err(e) => {
            let (code, retryable) = error_info(&e);
            Observation::failure(session_id, tab_id.unwrap_or(""), "reload", code, e.to_string(), retryable)
        }
    }
}

/// Wait for a condition.
pub async fn wait_for(
    session_manager: &Arc<SessionManager>,
    session_id: &str,
    tab_id: Option<&str>,
    condition: &str, // "selector", "navigation", "timeout"
    selector: Option<&str>,
    state: Option<&str>,
    timeout_ms: Option<u64>,
    idle_ms: Option<u64>,
    js_expr: Option<&str>,
    poll_ms: Option<u64>,
    sleep_ms: Option<u64>,
) -> Observation {
    let timeout = Duration::from_millis(timeout_ms.unwrap_or(10000));

    let result = async {
        let session = session_manager.get_session(session_id).await?;
        let (actual_tab_id, page_lock) = if let Some(tid) = tab_id {
            let p = session.get_page(tid).await?;
            (tid.to_string(), p)
        } else {
            session.get_active_page().await?
        };

        let page = page_lock.lock().await;

        match condition {
            "timeout" => {
                let ms = sleep_ms.unwrap_or(1000);
                tokio::time::sleep(Duration::from_millis(ms)).await;
            }
            "navigation" => {
                tokio::time::timeout(timeout, page.wait_for_navigation()).await
                    .map_err(|_| anyhow::anyhow!(MeleyError::Timeout("navigation wait timed out".to_string())))?
                    .map_err(|e| anyhow::anyhow!(MeleyError::NavigationFailed(e.to_string())))?;
            }
            "selector" => {
                let sel = selector.unwrap_or("body");
                let target_state = state.unwrap_or("attached");
                let poll = Duration::from_millis(poll_ms.unwrap_or(100));
                tokio::time::timeout(timeout, async {
                    loop {
                        let js = format!(
                            "!!document.querySelector({})",
                            serde_json::json!(sel)
                        );
                        let found = page.evaluate(js).await
                            .ok()
                            .and_then(|r| r.into_value::<bool>().ok())
                            .unwrap_or(false);

                        let done = match target_state {
                            "visible" | "attached" => found,
                            "hidden" | "detached" => !found,
                            _ => found,
                        };
                        if done { break; }
                        tokio::time::sleep(poll).await;
                    }
                    Ok::<(), anyhow::Error>(())
                }).await
                .map_err(|_| anyhow::anyhow!(MeleyError::Timeout("selector wait timed out".to_string())))?
                .map_err(|e| e)?;
            }
            "js_expression" => {
                let expr = js_expr.unwrap_or("true");
                let poll = Duration::from_millis(poll_ms.unwrap_or(200));
                tokio::time::timeout(timeout, async {
                    loop {
                        let truthy = page.evaluate(expr).await
                            .ok()
                            .and_then(|r| r.into_value::<serde_json::Value>().ok())
                            .map(|v| v.is_truthy())
                            .unwrap_or(false);
                        if truthy { break; }
                        tokio::time::sleep(poll).await;
                    }
                    Ok::<(), anyhow::Error>(())
                }).await
                .map_err(|_| anyhow::anyhow!(MeleyError::Timeout("js_expression wait timed out".to_string())))?
                .map_err(|e| e)?;
            }
            _ => {
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }

        let current_url = page.url().await.ok().flatten().map(|u| u.to_string());
        let title = page.get_title().await.ok().flatten();
        Ok::<(String, Option<String>, Option<String>), anyhow::Error>((actual_tab_id, current_url, title))
    }.await;

    match result {
        Ok((tid, url, title)) => {
            let mut obs = Observation::success(session_id, tid, "wait_for", ActionResult::Empty);
            obs.url = url;
            obs.title = title;
            obs
        }
        Err(e) => {
            let (code, retryable) = error_info(&e);
            Observation::failure(session_id, tab_id.unwrap_or(""), "wait_for", code, e.to_string(), retryable)
        }
    }
}

/// Helper to extract error code and retryability.
fn error_info(e: &anyhow::Error) -> (&'static str, bool) {
    if let Some(me) = e.downcast_ref::<MeleyError>() {
        (me.error_code(), me.is_retryable())
    } else {
        ("INTERNAL_ERROR", false)
    }
}

trait Truthiness {
    fn is_truthy(&self) -> bool;
}

impl Truthiness for serde_json::Value {
    fn is_truthy(&self) -> bool {
        match self {
            serde_json::Value::Null => false,
            serde_json::Value::Bool(b) => *b,
            serde_json::Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
            serde_json::Value::String(s) => !s.is_empty(),
            serde_json::Value::Array(a) => !a.is_empty(),
            serde_json::Value::Object(o) => !o.is_empty(),
        }
    }
}

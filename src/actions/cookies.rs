use std::sync::Arc;

use anyhow::Result;

use crate::error::MeleyError;
use crate::observation::{ActionResult, CookieInfo, Observation};
use crate::session::SessionManager;

/// Get cookies from the current session.
pub async fn get_cookies(
    session_manager: &Arc<SessionManager>,
    session_id: &str,
    tab_id: Option<&str>,
    urls: Option<Vec<String>>,
) -> Observation {
    let result: Result<Vec<CookieInfo>> = async {
        let session = session_manager.get_session(session_id).await?;
        session.get_cookies(urls).await
    }.await;

    match result {
        Ok(cookies) => {
            Observation::success(session_id, tab_id.unwrap_or(""), "get_cookies", ActionResult::Cookies(cookies))
        }
        Err(e) => {
            let (code, retryable) = error_code(&e);
            Observation::failure(session_id, tab_id.unwrap_or(""), "get_cookies", code, e.to_string(), retryable)
        }
    }
}

/// Set cookies.
pub async fn set_cookies(
    session_manager: &Arc<SessionManager>,
    session_id: &str,
    tab_id: Option<&str>,
    cookies: Vec<CookieInfo>,
) -> Observation {
    let result: Result<()> = async {
        let session = session_manager.get_session(session_id).await?;
        let (_, page_lock) = if let Some(tid) = tab_id {
            let p = session.get_page(tid).await?;
            (tid.to_string(), p)
        } else {
            session.get_active_page().await?
        };
        let page = page_lock.lock().await;

        let cdp_cookies: Vec<chromiumoxide::cdp::browser_protocol::network::CookieParam> = cookies.iter().map(|c| {
            chromiumoxide::cdp::browser_protocol::network::CookieParam::builder()
                .name(c.name.clone())
                .value(c.value.clone())
                .domain(c.domain.clone())
                .path(c.path.clone())
                .secure(c.secure)
                .http_only(c.http_only)
                .build()
                .unwrap()
        }).collect();

        page.set_cookies(cdp_cookies).await
            .map_err(|e| anyhow::anyhow!(MeleyError::Internal(e.to_string())))
            .map(|_| ())
    }.await;

    match result {
        Ok(()) => Observation::success(session_id, tab_id.unwrap_or(""), "set_cookies", ActionResult::Empty),
        Err(e) => {
            let (code, retryable) = error_code(&e);
            Observation::failure(session_id, tab_id.unwrap_or(""), "set_cookies", code, e.to_string(), retryable)
        }
    }
}

/// Clear all cookies.
pub async fn clear_cookies(
    session_manager: &Arc<SessionManager>,
    session_id: &str,
    tab_id: Option<&str>,
) -> Observation {
    let result: Result<(String,)> = async {
        let session = session_manager.get_session(session_id).await?;
        let (actual_tab_id, page_lock) = if let Some(tid) = tab_id {
            let p = session.get_page(tid).await?;
            (tid.to_string(), p)
        } else {
            session.get_active_page().await?
        };
        let page = page_lock.lock().await;

        // Use Browser-level clear_cookies
        // Since page.delete_cookies requires specific cookies, use JS fallback
        page.evaluate(
            r#"(function() {
                var cookies = document.cookie.split(';');
                for (var i = 0; i < cookies.length; i++) {
                    var cookie = cookies[i];
                    var eqPos = cookie.indexOf('=');
                    var name = eqPos > -1 ? cookie.substr(0, eqPos).trim() : cookie.trim();
                    document.cookie = name + '=;expires=Thu, 01 Jan 1970 00:00:00 GMT;path=/';
                }
                return true;
            })()"#
        ).await
        .map_err(|e| anyhow::anyhow!(MeleyError::Internal(e.to_string())))?;

        Ok((actual_tab_id,))
    }.await;

    match result {
        Ok((tid,)) => Observation::success(session_id, tid, "clear_cookies", ActionResult::Empty),
        Err(e) => {
            let (code, retryable) = error_code(&e);
            Observation::failure(session_id, tab_id.unwrap_or(""), "clear_cookies", code, e.to_string(), retryable)
        }
    }
}

/// Get localStorage content for an origin.
pub async fn get_local_storage(
    session_manager: &Arc<SessionManager>,
    session_id: &str,
    tab_id: Option<&str>,
    _origin: Option<&str>,
) -> Observation {
    let result: Result<(String, String)> = async {
        let session = session_manager.get_session(session_id).await?;
        let (actual_tab_id, page_lock) = if let Some(tid) = tab_id {
            let p = session.get_page(tid).await?;
            (tid.to_string(), p)
        } else {
            session.get_active_page().await?
        };
        let page = page_lock.lock().await;

        let js = r#"
        (function() {
            var result = {};
            try {
                for (var i = 0; i < localStorage.length; i++) {
                    var key = localStorage.key(i);
                    result[key] = localStorage.getItem(key);
                }
            } catch(e) {}
            return JSON.stringify(result);
        })()
        "#;

        let val = page.evaluate(js).await
            .map_err(|e| anyhow::anyhow!(MeleyError::Internal(e.to_string())))?
            .into_value::<String>()
            .unwrap_or_else(|_| "{}".to_string());

        Ok((actual_tab_id, val))
    }.await;

    match result {
        Ok((tid, json)) => Observation::success(session_id, tid, "get_local_storage", ActionResult::Text(json)),
        Err(e) => {
            let (code, retryable) = error_code(&e);
            Observation::failure(session_id, tab_id.unwrap_or(""), "get_local_storage", code, e.to_string(), retryable)
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

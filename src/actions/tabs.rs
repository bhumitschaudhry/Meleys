use std::sync::Arc;

use anyhow::Result;

use crate::error::MeleyError;
use crate::observation::{ActionResult, Observation, TabInfo};
use crate::session::SessionManager;

/// Create a new tab in a session.
pub async fn new_tab(
    session_manager: &Arc<SessionManager>,
    session_id: &str,
    url: Option<&str>,
) -> Observation {
    let result: Result<(String, Option<String>)> = async {
        let session = session_manager.get_session(session_id).await?;
        let tab_id = session.new_tab(url).await?;
        Ok((tab_id, url.map(|u| u.to_string())))
    }
    .await;

    match result {
        Ok((tab_id, url)) => {
            let tab_info = TabInfo {
                tab_id: tab_id.clone(),
                url,
                title: None,
                is_active: true,
                loading: false,
            };
            Observation::success(
                session_id,
                tab_id,
                "new_tab",
                ActionResult::Tabs(vec![tab_info]),
            )
        }
        Err(e) => {
            let (code, retryable) = error_code(&e);
            Observation::failure(session_id, "", "new_tab", code, e.to_string(), retryable)
        }
    }
}

/// Close a tab.
pub async fn close_tab(
    session_manager: &Arc<SessionManager>,
    session_id: &str,
    tab_id: &str,
) -> Observation {
    let result: Result<()> = async {
        let session = session_manager.get_session(session_id).await?;
        session.close_tab(tab_id).await?;
        Ok(())
    }
    .await;

    match result {
        Ok(()) => Observation::success(session_id, tab_id, "close_tab", ActionResult::Empty),
        Err(e) => {
            let (code, retryable) = error_code(&e);
            Observation::failure(
                session_id,
                tab_id,
                "close_tab",
                code,
                e.to_string(),
                retryable,
            )
        }
    }
}

/// List all tabs in a session.
pub async fn list_tabs(session_manager: &Arc<SessionManager>, session_id: &str) -> Observation {
    let result: Result<Vec<TabInfo>> = async {
        let session = session_manager.get_session(session_id).await?;
        session.list_tabs().await
    }
    .await;

    match result {
        Ok(tabs) => Observation::success(session_id, "", "list_tabs", ActionResult::Tabs(tabs)),
        Err(e) => {
            let (code, retryable) = error_code(&e);
            Observation::failure(session_id, "", "list_tabs", code, e.to_string(), retryable)
        }
    }
}

/// Switch active tab.
pub async fn switch_tab(
    session_manager: &Arc<SessionManager>,
    session_id: &str,
    tab_id: &str,
) -> Observation {
    let result: Result<TabInfo> = async {
        let session = session_manager.get_session(session_id).await?;
        session.switch_tab(tab_id).await?;

        let tabs = session.list_tabs().await?;
        tabs.into_iter()
            .find(|t| t.tab_id == tab_id)
            .ok_or_else(|| anyhow::anyhow!(MeleyError::TabNotFound(tab_id.to_string())))
    }
    .await;

    match result {
        Ok(tab_info) => Observation::success(
            session_id,
            tab_id,
            "switch_tab",
            ActionResult::Tabs(vec![tab_info]),
        ),
        Err(e) => {
            let (code, retryable) = error_code(&e);
            Observation::failure(
                session_id,
                tab_id,
                "switch_tab",
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

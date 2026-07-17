use std::sync::Arc;

use anyhow::Result;

use crate::engine::EnginePreference;
use crate::error::MeleyError;
use crate::observation::{ActionResult, Observation, SessionInfo};
use crate::session::SessionManager;

/// Create a new browser session.
pub async fn create_session(
    session_manager: &Arc<SessionManager>,
    profile_name: Option<String>,
    headless: Option<bool>,
    default_search_engine: Option<String>,
    engine_preference: Option<EnginePreference>,
) -> Observation {
    let result: Result<SessionInfo> = async {
        let session = session_manager
            .create_session(
                profile_name,
                headless,
                default_search_engine,
                engine_preference,
            )
            .await?;
        let tab_count = session.tab_count().await;
        let engine = session
            .get_default_search_engine()
            .await
            .unwrap_or_else(|| session_manager.default_search_engine().to_string());
        Ok(SessionInfo {
            session_id: session.session_id.clone(),
            profile_path: session.profile.path_str(),
            created_at: session.created_at.clone(),
            tab_count,
            default_search_engine: engine,
        })
    }
    .await;

    match result {
        Ok(info) => {
            let sid = info.session_id.clone();
            Observation::success(
                sid.clone(),
                "",
                "create_session",
                ActionResult::Sessions(vec![info]),
            )
        }
        Err(e) => {
            let (code, retryable) = error_code(&e);
            Observation::failure("", "", "create_session", code, e.to_string(), retryable)
        }
    }
}

/// Close a browser session.
pub async fn close_session(session_manager: &Arc<SessionManager>, session_id: &str) -> Observation {
    let result: Result<()> = async { session_manager.close_session(session_id).await }.await;

    match result {
        Ok(()) => Observation::success(session_id, "", "close_session", ActionResult::Empty),
        Err(e) => {
            let (code, retryable) = error_code(&e);
            Observation::failure(
                session_id,
                "",
                "close_session",
                code,
                e.to_string(),
                retryable,
            )
        }
    }
}

/// List all sessions.
pub async fn list_sessions(session_manager: &Arc<SessionManager>) -> Observation {
    let sessions = session_manager.list_sessions().await;
    Observation::success("", "", "list_sessions", ActionResult::Sessions(sessions))
}

fn error_code(e: &anyhow::Error) -> (&'static str, bool) {
    if let Some(me) = e.downcast_ref::<MeleyError>() {
        (me.error_code(), me.is_retryable())
    } else {
        ("INTERNAL_ERROR", false)
    }
}

use std::sync::Arc;

use anyhow::Result;

use crate::error::MeleyError;
use crate::observation::{ActionResult, Observation};
use crate::session::SessionManager;

pub mod capture;
pub mod cookies;
pub mod download;
pub mod extraction;
pub mod interaction;
pub mod navigation;
pub mod search;
pub mod session;
pub mod tabs;

/// Central action dispatcher.
pub struct ActionDispatcher {
    pub session_manager: Arc<SessionManager>,
}

impl ActionDispatcher {
    pub fn new(session_manager: Arc<SessionManager>) -> Self {
        Self { session_manager }
    }

    /// Helper: convert an error into a failure observation.
    pub fn error_obs(
        &self,
        session_id: impl Into<String>,
        tab_id: impl Into<String>,
        action: impl Into<String>,
        err: &anyhow::Error,
    ) -> Observation {
        // Try to downcast to our error type
        let (code, retryable) = if let Some(me) = err.downcast_ref::<MeleyError>() {
            (me.error_code().to_string(), me.is_retryable())
        } else {
            ("INTERNAL_ERROR".to_string(), false)
        };

        Observation::failure(session_id, tab_id, action, code, err.to_string(), retryable)
    }
}

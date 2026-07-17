use std::sync::Arc;

use crate::error::MeleyError;
use crate::observation::Observation;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn test_action_dispatcher_creation() {
        let config = Config::default();
        let manager = Arc::new(SessionManager::new(config));
        let dispatcher = ActionDispatcher::new(manager);
        assert_eq!(
            dispatcher.session_manager.default_search_engine(),
            "duckduckgo"
        );
    }

    #[test]
    fn test_error_obs_with_meley_error() {
        let config = Config::default();
        let manager = Arc::new(SessionManager::new(config));
        let dispatcher = ActionDispatcher::new(manager);

        let err = anyhow::anyhow!(MeleyError::SessionNotFound("s1".into()));
        let obs = dispatcher.error_obs("s1", "t1", "test_action", &err);
        assert!(!obs.success);
        assert_eq!(obs.session_id, "s1");
        assert_eq!(obs.tab_id, "t1");
        assert_eq!(obs.action, "test_action");
        let error = obs.error.as_ref().unwrap();
        assert_eq!(error.code, "SESSION_NOT_FOUND");
        assert!(!error.retryable);
    }

    #[test]
    fn test_error_obs_with_timeout_error() {
        let config = Config::default();
        let manager = Arc::new(SessionManager::new(config));
        let dispatcher = ActionDispatcher::new(manager);

        let err = anyhow::anyhow!(MeleyError::Timeout("30s".into()));
        let obs = dispatcher.error_obs("", "", "navigate", &err);
        assert!(!obs.success);
        let error = obs.error.as_ref().unwrap();
        assert_eq!(error.code, "TIMEOUT");
        assert!(error.retryable);
    }

    #[test]
    fn test_error_obs_with_generic_error() {
        let config = Config::default();
        let manager = Arc::new(SessionManager::new(config));
        let dispatcher = ActionDispatcher::new(manager);

        let err = anyhow::anyhow!("something unexpected");
        let obs = dispatcher.error_obs("", "", "test", &err);
        assert!(!obs.success);
        let error = obs.error.as_ref().unwrap();
        assert_eq!(error.code, "INTERNAL_ERROR");
        assert!(!error.retryable);
    }
}

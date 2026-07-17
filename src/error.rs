use thiserror::Error;

pub type Result<T> = std::result::Result<T, MeleyError>;

#[derive(Debug, Error)]
pub enum MeleyError {
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Tab not found: {0}")]
    TabNotFound(String),

    #[error("Element not found: {0}")]
    ElementNotFound(String),

    #[error("Element not interactable: {0}")]
    ElementNotInteractable(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Navigation failed: {0}")]
    NavigationFailed(String),

    #[error("Invalid selector: {0}")]
    InvalidSelector(String),

    #[error("Search engine parse failed: {0}")]
    SearchEngineParseFailed(String),

    #[error("Download failed: {0}")]
    DownloadFailed(String),

    #[error("CDP connection lost: {0}")]
    CdpConnectionLost(String),

    #[error("JS eval disabled")]
    JsEvalDisabled,

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Chromiumoxide error: {0}")]
    Chromiumoxide(#[from] chromiumoxide::error::CdpError),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Anyhow error: {0}")]
    Anyhow(#[from] anyhow::Error),
}

impl MeleyError {
    pub fn error_code(&self) -> &'static str {
        match self {
            MeleyError::SessionNotFound(_) => "SESSION_NOT_FOUND",
            MeleyError::TabNotFound(_) => "TAB_NOT_FOUND",
            MeleyError::ElementNotFound(_) => "ELEMENT_NOT_FOUND",
            MeleyError::ElementNotInteractable(_) => "ELEMENT_NOT_INTERACTABLE",
            MeleyError::Timeout(_) => "TIMEOUT",
            MeleyError::NavigationFailed(_) => "NAVIGATION_FAILED",
            MeleyError::InvalidSelector(_) => "INVALID_SELECTOR",
            MeleyError::SearchEngineParseFailed(_) => "SEARCH_ENGINE_PARSE_FAILED",
            MeleyError::DownloadFailed(_) => "DOWNLOAD_FAILED",
            MeleyError::CdpConnectionLost(_) => "CDP_CONNECTION_LOST",
            MeleyError::JsEvalDisabled => "JS_EVAL_DISABLED",
            MeleyError::Internal(_) => "INTERNAL_ERROR",
            MeleyError::Chromiumoxide(_) => "INTERNAL_ERROR",
            MeleyError::Json(_) => "INTERNAL_ERROR",
            MeleyError::Io(_) => "INTERNAL_ERROR",
            MeleyError::Anyhow(_) => "INTERNAL_ERROR",
        }
    }

    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            MeleyError::Timeout(_)
                | MeleyError::CdpConnectionLost(_)
                | MeleyError::NavigationFailed(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_not_found_error_code() {
        let err = MeleyError::SessionNotFound("abc".into());
        assert_eq!(err.error_code(), "SESSION_NOT_FOUND");
        assert!(!err.is_retryable());
        assert!(err.to_string().contains("abc"));
    }

    #[test]
    fn test_tab_not_found_error_code() {
        let err = MeleyError::TabNotFound("tab1".into());
        assert_eq!(err.error_code(), "TAB_NOT_FOUND");
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_element_not_found_error_code() {
        let err = MeleyError::ElementNotFound("#btn".into());
        assert_eq!(err.error_code(), "ELEMENT_NOT_FOUND");
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_element_not_interactable_error_code() {
        let err = MeleyError::ElementNotInteractable("disabled".into());
        assert_eq!(err.error_code(), "ELEMENT_NOT_INTERACTABLE");
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_timeout_is_retryable() {
        let err = MeleyError::Timeout("30s".into());
        assert_eq!(err.error_code(), "TIMEOUT");
        assert!(err.is_retryable());
    }

    #[test]
    fn test_navigation_failed_is_retryable() {
        let err = MeleyError::NavigationFailed("DNS".into());
        assert_eq!(err.error_code(), "NAVIGATION_FAILED");
        assert!(err.is_retryable());
    }

    #[test]
    fn test_invalid_selector_error_code() {
        let err = MeleyError::InvalidSelector("bad".into());
        assert_eq!(err.error_code(), "INVALID_SELECTOR");
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_search_engine_parse_failed_error_code() {
        let err = MeleyError::SearchEngineParseFailed("markup changed".into());
        assert_eq!(err.error_code(), "SEARCH_ENGINE_PARSE_FAILED");
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_download_failed_error_code() {
        let err = MeleyError::DownloadFailed("404".into());
        assert_eq!(err.error_code(), "DOWNLOAD_FAILED");
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_cdp_connection_lost_is_retryable() {
        let err = MeleyError::CdpConnectionLost("socket closed".into());
        assert_eq!(err.error_code(), "CDP_CONNECTION_LOST");
        assert!(err.is_retryable());
    }

    #[test]
    fn test_js_eval_disabled_error_code() {
        let err = MeleyError::JsEvalDisabled;
        assert_eq!(err.error_code(), "JS_EVAL_DISABLED");
        assert!(!err.is_retryable());
        assert!(err.to_string().contains("JS eval disabled"));
    }

    #[test]
    fn test_internal_error_error_code() {
        let err = MeleyError::Internal("something broke".into());
        assert_eq!(err.error_code(), "INTERNAL_ERROR");
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_json_error_maps_to_internal() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid").unwrap_err();
        let err = MeleyError::Json(json_err);
        assert_eq!(err.error_code(), "INTERNAL_ERROR");
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_io_error_maps_to_internal() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let err = MeleyError::Io(io_err);
        assert_eq!(err.error_code(), "INTERNAL_ERROR");
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_all_variants_have_error_codes() {
        let variants: Vec<MeleyError> = vec![
            MeleyError::SessionNotFound("".into()),
            MeleyError::TabNotFound("".into()),
            MeleyError::ElementNotFound("".into()),
            MeleyError::ElementNotInteractable("".into()),
            MeleyError::Timeout("".into()),
            MeleyError::NavigationFailed("".into()),
            MeleyError::InvalidSelector("".into()),
            MeleyError::SearchEngineParseFailed("".into()),
            MeleyError::DownloadFailed("".into()),
            MeleyError::CdpConnectionLost("".into()),
            MeleyError::JsEvalDisabled,
            MeleyError::Internal("".into()),
        ];
        for err in variants {
            let code = err.error_code();
            assert!(!code.is_empty(), "Error code empty for {:?}", err);
        }
    }

    #[test]
    fn test_result_type_alias() {
        let ok: Result<i32> = Ok(42);
        assert_eq!(ok.unwrap(), 42);

        let err: Result<i32> = Err(MeleyError::Internal("fail".into()));
        assert!(err.is_err());
    }
}

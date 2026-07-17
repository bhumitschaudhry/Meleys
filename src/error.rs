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

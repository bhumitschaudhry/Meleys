use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The universal return type from every Meleys action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub session_id: String,
    pub tab_id: String,
    pub action: String,
    pub success: bool,
    pub timestamp: String, // RFC3339
    pub url: Option<String>,
    pub title: Option<String>,
    pub status_code: Option<u16>,
    pub result: ActionResult,
    pub error: Option<ErrorInfo>,
    pub console_messages: Vec<ConsoleMessage>,
    pub network_summary: Option<NetworkSummary>,
}

impl Observation {
    pub fn success(
        session_id: impl Into<String>,
        tab_id: impl Into<String>,
        action: impl Into<String>,
        result: ActionResult,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            tab_id: tab_id.into(),
            action: action.into(),
            success: true,
            timestamp: chrono::Utc::now().to_rfc3339(),
            url: None,
            title: None,
            status_code: None,
            result,
            error: None,
            console_messages: vec![],
            network_summary: None,
        }
    }

    pub fn failure(
        session_id: impl Into<String>,
        tab_id: impl Into<String>,
        action: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
        retryable: bool,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            tab_id: tab_id.into(),
            action: action.into(),
            success: false,
            timestamp: chrono::Utc::now().to_rfc3339(),
            url: None,
            title: None,
            status_code: None,
            result: ActionResult::Empty,
            error: Some(ErrorInfo {
                code: code.into(),
                message: message.into(),
                retryable,
            }),
            console_messages: vec![],
            network_summary: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ActionResult {
    Empty,
    Text(String),
    Links(Vec<LinkInfo>),
    SearchResults(Vec<SearchResultItem>),
    Dom(SimplifiedNode),
    AxTree(AxNode),
    Screenshot {
        format: String,
        base64: String,
        width: u32,
        height: u32,
    },
    Download(DownloadInfo),
    Tabs(Vec<TabInfo>),
    Sessions(Vec<SessionInfo>),
    Elements(Vec<ElementInfo>),
    Cookies(Vec<CookieInfo>),
    SearchEngine {
        engine: String,
        scope: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorInfo {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleMessage {
    pub level: String,
    pub text: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSummary {
    pub requests: u32,
    pub failed: u32,
    pub bytes_received: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkInfo {
    pub href: String,
    pub text: String,
    pub visible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultItem {
    pub rank: u32,
    pub title: String,
    pub url: String,
    pub snippet: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimplifiedNode {
    pub backend_node_id: i64,
    pub tag: String,
    pub attributes: HashMap<String, String>,
    pub text: Option<String>,
    pub visible: bool,
    pub bounding_box: Option<Rect>,
    pub children: Vec<SimplifiedNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxNode {
    pub ax_node_id: String,
    pub role: String,
    pub name: Option<String>,
    pub value: Option<String>,
    pub focusable: bool,
    pub focused: bool,
    pub disabled: bool,
    pub children: Vec<AxNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadInfo {
    pub id: String,
    pub url: String,
    pub path: String,
    pub size_bytes: Option<u64>,
    pub state: String, // "in_progress" | "completed" | "failed"
    pub started_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabInfo {
    pub tab_id: String,
    pub url: Option<String>,
    pub title: Option<String>,
    pub is_active: bool,
    pub loading: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub profile_path: String,
    pub created_at: String,
    pub tab_count: usize,
    pub default_search_engine: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementInfo {
    pub backend_node_id: i64,
    pub tag: String,
    pub text: Option<String>,
    pub attributes: HashMap<String, String>,
    pub bounding_box: Option<Rect>,
    pub visible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookieInfo {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub secure: bool,
    pub http_only: bool,
    pub same_site: Option<String>,
    pub expires: Option<f64>,
}

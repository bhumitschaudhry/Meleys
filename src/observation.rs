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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_observation_success_construction() {
        let obs = Observation::success("s1", "t1", "navigate", ActionResult::Empty);
        assert!(obs.success);
        assert_eq!(obs.session_id, "s1");
        assert_eq!(obs.tab_id, "t1");
        assert_eq!(obs.action, "navigate");
        assert!(obs.error.is_none());
        assert!(!obs.timestamp.is_empty());
        assert!(obs.url.is_none());
        assert!(obs.title.is_none());
        assert!(obs.status_code.is_none());
        assert!(obs.console_messages.is_empty());
        assert!(obs.network_summary.is_none());
    }

    #[test]
    fn test_observation_failure_construction() {
        let obs = Observation::failure("s1", "t1", "click", "ELEMENT_NOT_FOUND", "no such element", true);
        assert!(!obs.success);
        assert_eq!(obs.session_id, "s1");
        assert_eq!(obs.tab_id, "t1");
        assert_eq!(obs.action, "click");
        let err = obs.error.as_ref().unwrap();
        assert_eq!(err.code, "ELEMENT_NOT_FOUND");
        assert_eq!(err.message, "no such element");
        assert!(err.retryable);
    }

    #[test]
    fn test_observation_failure_non_retryable() {
        let obs = Observation::failure("", "", "test", "INTERNAL_ERROR", "boom", false);
        let err = obs.error.as_ref().unwrap();
        assert!(!err.retryable);
    }

    #[test]
    fn test_observation_timestamp_is_rfc3339() {
        let obs = Observation::success("", "", "", ActionResult::Empty);
        chrono::DateTime::parse_from_rfc3339(&obs.timestamp)
            .expect("Timestamp should be valid RFC3339");
    }

    #[test]
    fn test_observation_json_roundtrip() {
        let mut obs = Observation::success("s1", "t1", "get_text", ActionResult::Text("hello".into()));
        obs.url = Some("https://example.com".into());
        obs.title = Some("Example".into());
        obs.status_code = Some(200);

        let json = serde_json::to_string(&obs).expect("Serialization failed");
        let deserialized: Observation = serde_json::from_str(&json).expect("Deserialization failed");
        assert_eq!(deserialized.session_id, "s1");
        assert_eq!(deserialized.tab_id, "t1");
        assert_eq!(deserialized.url, Some("https://example.com".into()));
        assert_eq!(deserialized.title, Some("Example".into()));
        assert_eq!(deserialized.status_code, Some(200));
    }

    #[test]
    fn test_action_result_text() {
        let obs = Observation::success("", "", "", ActionResult::Text("content".into()));
        match &obs.result {
            ActionResult::Text(t) => assert_eq!(t, "content"),
            _ => panic!("Expected Text variant"),
        }
    }

    #[test]
    fn test_action_result_empty() {
        let obs = Observation::success("", "", "", ActionResult::Empty);
        match obs.result {
            ActionResult::Empty => {}
            _ => panic!("Expected Empty variant"),
        }
    }

    #[test]
    fn test_action_result_search_results() {
        let items = vec![
            SearchResultItem {
                rank: 1,
                title: "Rust".into(),
                url: "https://rust-lang.org".into(),
                snippet: Some("A language...".into()),
            },
            SearchResultItem {
                rank: 2,
                title: "Cargo".into(),
                url: "https://crates.io".into(),
                snippet: None,
            },
        ];
        let obs = Observation::success("", "", "", ActionResult::SearchResults(items));
        match &obs.result {
            ActionResult::SearchResults(results) => {
                assert_eq!(results.len(), 2);
                assert_eq!(results[0].rank, 1);
                assert_eq!(results[0].title, "Rust");
                assert!(results[1].snippet.is_none());
            }
            _ => panic!("Expected SearchResults"),
        }
    }

    #[test]
    fn test_action_result_screenshot() {
        let obs = Observation::success(
            "", "", "",
            ActionResult::Screenshot {
                format: "png".into(),
                base64: "abc123".into(),
                width: 1280,
                height: 800,
            },
        );
        match &obs.result {
            ActionResult::Screenshot { format, base64, width, height } => {
                assert_eq!(format, "png");
                assert_eq!(base64, "abc123");
                assert_eq!(*width, 1280);
                assert_eq!(*height, 800);
            }
            _ => panic!("Expected Screenshot"),
        }
    }

    #[test]
    fn test_observation_with_console_messages() {
        let mut obs = Observation::success("", "", "", ActionResult::Empty);
        obs.console_messages = vec![
            ConsoleMessage {
                level: "error".into(),
                text: "Uncaught TypeError".into(),
                timestamp: "2025-01-01T00:00:00Z".into(),
            },
        ];
        let json = serde_json::to_string(&obs).unwrap();
        let d: Observation = serde_json::from_str(&json).unwrap();
        assert_eq!(d.console_messages.len(), 1);
        assert_eq!(d.console_messages[0].level, "error");
    }

    #[test]
    fn test_observation_with_network_summary() {
        let mut obs = Observation::success("", "", "", ActionResult::Empty);
        obs.network_summary = Some(NetworkSummary {
            requests: 10,
            failed: 1,
            bytes_received: 50000,
        });
        let json = serde_json::to_string(&obs).unwrap();
        let d: Observation = serde_json::from_str(&json).unwrap();
        let net = d.network_summary.unwrap();
        assert_eq!(net.requests, 10);
        assert_eq!(net.failed, 1);
        assert_eq!(net.bytes_received, 50000);
    }

    #[test]
    fn test_link_info_serialization() {
        let link = LinkInfo {
            href: "https://example.com".into(),
            text: "Example".into(),
            visible: true,
        };
        let json = serde_json::to_string(&link).unwrap();
        let d: LinkInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(d.href, "https://example.com");
        assert!(d.visible);
    }

    #[test]
    fn test_simplified_node_serialization() {
        let node = SimplifiedNode {
            backend_node_id: 42,
            tag: "div".into(),
            attributes: {
                let mut m = HashMap::new();
                m.insert("class".into(), "container".into());
                m
            },
            text: Some("hello".into()),
            visible: true,
            bounding_box: Some(Rect { x: 0.0, y: 0.0, width: 100.0, height: 50.0 }),
            children: vec![],
        };
        let json = serde_json::to_string(&node).unwrap();
        let d: SimplifiedNode = serde_json::from_str(&json).unwrap();
        assert_eq!(d.backend_node_id, 42);
        assert_eq!(d.tag, "div");
        assert_eq!(d.attributes.get("class").unwrap(), "container");
        assert!(d.bounding_box.is_some());
        assert_eq!(d.bounding_box.as_ref().unwrap().width, 100.0);
    }

    #[test]
    fn test_ax_node_serialization() {
        let node = AxNode {
            ax_node_id: "1".into(),
            role: "button".into(),
            name: Some("Submit".into()),
            value: None,
            focusable: true,
            focused: false,
            disabled: false,
            children: vec![],
        };
        let json = serde_json::to_string(&node).unwrap();
        let d: AxNode = serde_json::from_str(&json).unwrap();
        assert_eq!(d.role, "button");
        assert!(d.focusable);
        assert!(!d.disabled);
    }

    #[test]
    fn test_download_info_serialization() {
        let dl = DownloadInfo {
            id: "dl1".into(),
            url: "https://example.com/file.zip".into(),
            path: "/tmp/file.zip".into(),
            size_bytes: Some(1024),
            state: "completed".into(),
            started_at: "2025-01-01T00:00:00Z".into(),
            completed_at: Some("2025-01-01T00:00:01Z".into()),
        };
        let json = serde_json::to_string(&dl).unwrap();
        let d: DownloadInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(d.state, "completed");
        assert_eq!(d.size_bytes, Some(1024));
        assert!(d.completed_at.is_some());
    }

    #[test]
    fn test_tab_info_serialization() {
        let tab = TabInfo {
            tab_id: "t1".into(),
            url: Some("https://example.com".into()),
            title: Some("Example".into()),
            is_active: true,
            loading: false,
        };
        let json = serde_json::to_string(&tab).unwrap();
        let d: TabInfo = serde_json::from_str(&json).unwrap();
        assert!(d.is_active);
        assert!(!d.loading);
    }

    #[test]
    fn test_session_info_serialization() {
        let info = SessionInfo {
            session_id: "s1".into(),
            profile_path: "/tmp/profile".into(),
            created_at: "2025-01-01T00:00:00Z".into(),
            tab_count: 2,
            default_search_engine: "duckduckgo".into(),
        };
        let json = serde_json::to_string(&info).unwrap();
        let d: SessionInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(d.tab_count, 2);
        assert_eq!(d.default_search_engine, "duckduckgo");
    }

    #[test]
    fn test_element_info_serialization() {
        let el = ElementInfo {
            backend_node_id: 5,
            tag: "button".into(),
            text: Some("Click me".into()),
            attributes: {
                let mut m = HashMap::new();
                m.insert("id".into(), "btn".into());
                m
            },
            bounding_box: None,
            visible: true,
        };
        let json = serde_json::to_string(&el).unwrap();
        let d: ElementInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(d.tag, "button");
        assert!(d.visible);
    }

    #[test]
    fn test_cookie_info_serialization() {
        let cookie = CookieInfo {
            name: "session".into(),
            value: "abc123".into(),
            domain: ".example.com".into(),
            path: "/".into(),
            secure: true,
            http_only: true,
            same_site: Some("Lax".into()),
            expires: Some(1735689600.0),
        };
        let json = serde_json::to_string(&cookie).unwrap();
        let d: CookieInfo = serde_json::from_str(&json).unwrap();
        assert!(d.secure);
        assert!(d.http_only);
        assert_eq!(d.same_site.as_deref(), Some("Lax"));
    }
}

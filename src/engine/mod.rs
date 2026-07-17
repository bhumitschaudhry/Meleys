pub mod chromium;
pub mod fallback;
pub mod lightpanda;

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum EngineKind {
    Lightpanda,
    Chromium,
}

impl std::fmt::Display for EngineKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineKind::Lightpanda => write!(f, "lightpanda"),
            EngineKind::Chromium => write!(f, "chromium"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EnginePreference {
    LightpandaOnly,
    ChromiumOnly,
    #[default]
    LightpandaWithFallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EngineCapabilities {
    pub screenshot: bool,
    pub pdf_export: bool,
    pub javascript: bool,
    pub cors: bool,
    pub network_interception: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EngineTransition {
    pub from: EngineKind,
    pub to: EngineKind,
    pub reason: String,
    pub timestamp: String,
}

pub struct EnginePage {
    pub kind: EngineKind,
}

#[async_trait]
pub trait BrowserEngine: Send + Sync {
    fn kind(&self) -> EngineKind;

    async fn spawn(profile: &str, config: &str) -> anyhow::Result<Self>
    where
        Self: Sized;

    async fn new_page(&self) -> anyhow::Result<Arc<Mutex<EnginePage>>>;

    async fn close(&self) -> anyhow::Result<()>;

    fn capabilities(&self) -> EngineCapabilities;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_kind_display() {
        assert_eq!(EngineKind::Lightpanda.to_string(), "lightpanda");
        assert_eq!(EngineKind::Chromium.to_string(), "chromium");
    }

    #[test]
    fn test_engine_kind_equality() {
        assert_eq!(EngineKind::Lightpanda, EngineKind::Lightpanda);
        assert_ne!(EngineKind::Lightpanda, EngineKind::Chromium);
    }

    #[test]
    fn test_engine_kind_hash() {
        use std::collections::HashMap;
        let mut map = HashMap::new();
        map.insert(EngineKind::Lightpanda, "lp");
        map.insert(EngineKind::Chromium, "cr");
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn test_engine_kind_serde_roundtrip() {
        let kind = EngineKind::Lightpanda;
        let json = serde_json::to_string(&kind).unwrap();
        let deserialized: EngineKind = serde_json::from_str(&json).unwrap();
        assert_eq!(kind, deserialized);
    }

    #[test]
    fn test_engine_preference_default() {
        assert_eq!(
            EnginePreference::default(),
            EnginePreference::LightpandaWithFallback
        );
    }

    #[test]
    fn test_engine_capabilities() {
        let caps = EngineCapabilities {
            screenshot: true,
            pdf_export: false,
            javascript: true,
            cors: false,
            network_interception: true,
        };
        assert!(caps.screenshot);
        assert!(!caps.pdf_export);
        assert!(caps.javascript);
        assert!(!caps.cors);
        assert!(caps.network_interception);
    }

    #[test]
    fn test_engine_transition() {
        let transition = EngineTransition {
            from: EngineKind::Lightpanda,
            to: EngineKind::Chromium,
            reason: "missing feature".into(),
            timestamp: "2026-07-18T00:00:00Z".into(),
        };
        assert_eq!(transition.from, EngineKind::Lightpanda);
        assert_eq!(transition.to, EngineKind::Chromium);
    }

    #[test]
    fn test_engine_page_kind() {
        let page = EnginePage {
            kind: EngineKind::Chromium,
        };
        assert_eq!(page.kind, EngineKind::Chromium);
    }
}

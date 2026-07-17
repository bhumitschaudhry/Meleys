use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::{BrowserEngine, EngineCapabilities, EngineKind, EnginePage};

pub struct LightpandaEngine;

#[async_trait]
impl BrowserEngine for LightpandaEngine {
    fn kind(&self) -> EngineKind {
        EngineKind::Lightpanda
    }

    async fn spawn(_profile: &str, _config: &str) -> anyhow::Result<Self> {
        Ok(Self)
    }

    async fn new_page(&self) -> anyhow::Result<Arc<Mutex<EnginePage>>> {
        Err(anyhow::anyhow!("LightpandaEngine::new_page not yet wired"))
    }

    async fn close(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn capabilities(&self) -> EngineCapabilities {
        EngineCapabilities {
            screenshot: false,
            pdf_export: false,
            javascript: true,
            cors: false,
            network_interception: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_lightpanda_engine_kind() {
        let engine = LightpandaEngine;
        assert_eq!(engine.kind(), EngineKind::Lightpanda);
    }

    #[tokio::test]
    async fn test_lightpanda_engine_spawn() {
        let engine = LightpandaEngine::spawn("default", "").await.unwrap();
        assert_eq!(engine.kind(), EngineKind::Lightpanda);
    }

    #[tokio::test]
    async fn test_lightpanda_engine_new_page_returns_err() {
        let engine = LightpandaEngine;
        assert!(engine.new_page().await.is_err());
    }

    #[tokio::test]
    async fn test_lightpanda_engine_close() {
        let engine = LightpandaEngine;
        assert!(engine.close().await.is_ok());
    }

    #[tokio::test]
    async fn test_lightpanda_engine_capabilities() {
        let engine = LightpandaEngine;
        let caps = engine.capabilities();
        assert!(!caps.screenshot);
        assert!(!caps.pdf_export);
        assert!(caps.javascript);
        assert!(!caps.cors);
        assert!(caps.network_interception);
    }
}

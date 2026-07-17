use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::{BrowserEngine, EngineCapabilities, EngineKind, EnginePage};

pub struct ChromiumEngine;

#[async_trait]
impl BrowserEngine for ChromiumEngine {
    fn kind(&self) -> EngineKind {
        EngineKind::Chromium
    }

    async fn spawn(_profile: &str, _config: &str) -> anyhow::Result<Self> {
        Ok(Self)
    }

    async fn new_page(&self) -> anyhow::Result<Arc<Mutex<EnginePage>>> {
        Ok(Arc::new(Mutex::new(EnginePage {
            kind: EngineKind::Chromium,
        })))
    }

    async fn close(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn capabilities(&self) -> EngineCapabilities {
        EngineCapabilities {
            screenshot: true,
            pdf_export: true,
            javascript: true,
            cors: true,
            network_interception: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_chromium_engine_kind() {
        let engine = ChromiumEngine;
        assert_eq!(engine.kind(), EngineKind::Chromium);
    }

    #[tokio::test]
    async fn test_chromium_engine_spawn() {
        let engine = ChromiumEngine::spawn("default", "").await.unwrap();
        assert_eq!(engine.kind(), EngineKind::Chromium);
    }

    #[tokio::test]
    async fn test_chromium_engine_new_page() {
        let engine = ChromiumEngine;
        let page = engine.new_page().await.unwrap();
        let page = page.lock().await;
        assert_eq!(page.kind, EngineKind::Chromium);
    }

    #[tokio::test]
    async fn test_chromium_engine_close() {
        let engine = ChromiumEngine;
        assert!(engine.close().await.is_ok());
    }

    #[tokio::test]
    async fn test_chromium_engine_capabilities() {
        let engine = ChromiumEngine;
        let caps = engine.capabilities();
        assert!(caps.screenshot);
        assert!(caps.pdf_export);
        assert!(caps.javascript);
        assert!(caps.cors);
        assert!(caps.network_interception);
    }
}

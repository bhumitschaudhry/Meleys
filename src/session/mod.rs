pub mod browser_session;
pub mod profile;

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;

use crate::config::Config;
use crate::error::MeleyError;
use crate::observation::SessionInfo;
use crate::session::browser_session::BrowserSession;
use crate::session::profile::Profile;

/// Manages all active browser sessions.
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Arc<BrowserSession>>>>,
    config: Config,
}

impl SessionManager {
    pub fn new(config: Config) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Create a new browser session and return its ID.
    pub async fn create_session(
        &self,
        profile_name: Option<String>,
        headless: Option<bool>,
        default_search_engine: Option<String>,
    ) -> Result<Arc<BrowserSession>> {
        // Check session limit
        {
            let sessions = self.sessions.read().await;
            if sessions.len() >= self.config.limits.max_sessions {
                return Err(anyhow::anyhow!(
                    "Maximum session limit ({}) reached",
                    self.config.limits.max_sessions
                ));
            }
        }

        let session_id = uuid::Uuid::new_v4().to_string();

        // Create or open profile
        let profile = if let Some(ref name) = profile_name {
            Profile::open(&self.config.engine.chromium.profile_dir, name)?
        } else {
            Profile::temporary(&self.config.engine.chromium.profile_dir)?
        };

        // Launch browser
        let mut browser_config = self.config.engine.chromium.clone();
        if let Some(h) = headless {
            browser_config.headless = h;
        }

        let launch_result = crate::cdp::launcher::launch_browser(
            &browser_config,
            Some(profile.path_str().as_str()),
        )
        .await?;

        let session = Arc::new(BrowserSession::new(
            session_id.clone(),
            profile,
            launch_result.browser,
            launch_result.handler_task,
        ));

        // Set per-session default search engine
        if let Some(engine) = default_search_engine {
            session.set_default_search_engine(Some(engine)).await;
        }

        // Create initial tab
        session.new_tab(None).await?;

        // Store session
        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(session_id.clone(), session.clone());
        }

        tracing::info!("Created session {}", session_id);
        Ok(session)
    }

    /// Close a session.
    pub async fn close_session(&self, session_id: &str) -> Result<()> {
        let session = {
            let mut sessions = self.sessions.write().await;
            sessions
                .remove(session_id)
                .ok_or_else(|| MeleyError::SessionNotFound(session_id.to_string()))?
        };
        session.close().await?;
        tracing::info!("Closed session {}", session_id);
        Ok(())
    }

    /// Get a session by ID.
    pub async fn get_session(&self, session_id: &str) -> Result<Arc<BrowserSession>> {
        let sessions = self.sessions.read().await;
        sessions
            .get(session_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!(MeleyError::SessionNotFound(session_id.to_string())))
    }

    /// List all sessions.
    pub async fn list_sessions(&self) -> Vec<SessionInfo> {
        let sessions = self.sessions.read().await;
        let mut result = Vec::new();
        for (id, session) in sessions.iter() {
            let tab_count = session.tab_count().await;
            let engine = session
                .get_default_search_engine()
                .await
                .unwrap_or_else(|| self.config.search.default_engine.clone());
            result.push(SessionInfo {
                session_id: id.clone(),
                profile_path: session.profile.path_str(),
                created_at: session.created_at.clone(),
                tab_count,
                default_search_engine: engine,
            });
        }
        result
    }

    /// Close all sessions gracefully.
    pub async fn close_all(&self) {
        let sessions = {
            let mut sessions = self.sessions.write().await;
            std::mem::take(&mut *sessions)
        };
        for (id, session) in sessions {
            if let Err(e) = session.close().await {
                tracing::warn!("Failed to close session {}: {}", id, e);
            }
        }
    }

    /// Get the runtime-wide default search engine.
    pub fn default_search_engine(&self) -> &str {
        &self.config.search.default_engine
    }

    /// Get config.
    pub fn config(&self) -> &Config {
        &self.config
    }
}

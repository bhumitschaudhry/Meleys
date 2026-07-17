use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use chromiumoxide::browser::Browser;
use chromiumoxide::Page;
use tokio::sync::Mutex;

use crate::engine::{EngineKind, EngineTransition};
use crate::error::MeleyError;
use crate::observation::{CookieInfo, TabInfo};
use crate::session::profile::Profile;

/// A single tab within a browser session.
pub struct Tab {
    pub tab_id: String,
    pub page: Arc<Mutex<Page>>,
    pub is_active: bool,
}

/// A browser session: one Chromium process, one profile, multiple tabs.
pub struct BrowserSession {
    pub session_id: String,
    pub profile: Profile,
    pub browser: Arc<Mutex<Browser>>,
    pub tabs: Arc<Mutex<HashMap<String, Tab>>>,
    pub active_tab_id: Arc<Mutex<Option<String>>>,
    pub created_at: String,
    pub default_search_engine: Arc<Mutex<Option<String>>>,
    pub handler_task: Option<tokio::task::JoinHandle<()>>,
    pub engine_kind: EngineKind,
    pub engine_history: Vec<EngineTransition>,
}

impl BrowserSession {
    pub fn new(
        session_id: String,
        profile: Profile,
        browser: Browser,
        handler_task: tokio::task::JoinHandle<()>,
        engine_kind: EngineKind,
        engine_history: Vec<EngineTransition>,
    ) -> Self {
        Self {
            session_id,
            profile,
            browser: Arc::new(Mutex::new(browser)),
            tabs: Arc::new(Mutex::new(HashMap::new())),
            active_tab_id: Arc::new(Mutex::new(None)),
            created_at: chrono::Utc::now().to_rfc3339(),
            default_search_engine: Arc::new(Mutex::new(None)),
            handler_task: Some(handler_task),
            engine_kind,
            engine_history,
        }
    }

    /// Create a new tab and return its ID.
    pub async fn new_tab(&self, url: Option<&str>) -> Result<String> {
        let tab_id = uuid::Uuid::new_v4().to_string();
        let navigate_url = url.unwrap_or("about:blank");

        let page = {
            let browser = self.browser.lock().await;
            browser
                .new_page(navigate_url)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to create new tab: {}", e))?
        };

        // Check if this is the first tab
        let is_first = {
            let tabs = self.tabs.lock().await;
            tabs.is_empty()
        };

        {
            let mut tabs = self.tabs.lock().await;
            tabs.insert(
                tab_id.clone(),
                Tab {
                    tab_id: tab_id.clone(),
                    page: Arc::new(Mutex::new(page)),
                    is_active: is_first,
                },
            );
        }

        // Set as active if first tab
        if is_first {
            let mut active = self.active_tab_id.lock().await;
            *active = Some(tab_id.clone());
        }

        Ok(tab_id)
    }

    /// Get the page for a specific tab.
    pub async fn get_page(&self, tab_id: &str) -> Result<Arc<Mutex<Page>>> {
        let tabs = self.tabs.lock().await;
        tabs.get(tab_id)
            .map(|t| t.page.clone())
            .ok_or_else(|| anyhow::anyhow!(MeleyError::TabNotFound(tab_id.to_string())))
    }

    /// Get the active tab's page.
    pub async fn get_active_page(&self) -> Result<(String, Arc<Mutex<Page>>)> {
        let active = self.active_tab_id.lock().await;
        let tab_id = active
            .clone()
            .ok_or_else(|| anyhow::anyhow!(MeleyError::TabNotFound("no active tab".to_string())))?;
        drop(active);

        let tabs = self.tabs.lock().await;
        let page = tabs
            .get(&tab_id)
            .map(|t| t.page.clone())
            .ok_or_else(|| anyhow::anyhow!(MeleyError::TabNotFound(tab_id.clone())))?;

        Ok((tab_id, page))
    }

    /// Close a tab.
    pub async fn close_tab(&self, tab_id: &str) -> Result<()> {
        let page = self.get_page(tab_id).await?;
        {
            let page_guard = page.lock().await;
            // Clone the page to call close (which takes self)
            page_guard
                .clone()
                .close()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to close tab: {}", e))?;
        }

        let mut tabs = self.tabs.lock().await;
        tabs.remove(tab_id);

        // Update active tab
        let mut active = self.active_tab_id.lock().await;
        if *active == Some(tab_id.to_string()) {
            *active = tabs.keys().next().cloned();
        }

        Ok(())
    }

    /// Switch active tab.
    pub async fn switch_tab(&self, tab_id: &str) -> Result<()> {
        let tabs = self.tabs.lock().await;
        if !tabs.contains_key(tab_id) {
            return Err(anyhow::anyhow!(MeleyError::TabNotFound(tab_id.to_string())));
        }
        drop(tabs);

        let mut active = self.active_tab_id.lock().await;
        *active = Some(tab_id.to_string());
        Ok(())
    }

    /// List all tabs.
    pub async fn list_tabs(&self) -> Result<Vec<TabInfo>> {
        let tabs = self.tabs.lock().await;
        let active = self.active_tab_id.lock().await;
        let active_id = active.clone();
        drop(active);

        let mut result = Vec::new();
        for (id, tab) in tabs.iter() {
            let page = tab.page.lock().await;
            let url = page.url().await.ok().flatten();
            let title = page.get_title().await.ok().flatten();
            result.push(TabInfo {
                tab_id: id.clone(),
                url,
                title,
                is_active: active_id.as_deref() == Some(id.as_str()),
                loading: false,
            });
        }
        Ok(result)
    }

    /// Get tab count.
    pub async fn tab_count(&self) -> usize {
        self.tabs.lock().await.len()
    }

    /// Close all tabs and terminate the browser.
    pub async fn close(&self) -> Result<()> {
        let mut browser = self.browser.lock().await;
        let _ = browser.close().await;
        Ok(())
    }

    /// Get all cookies from the browser.
    pub async fn get_cookies(&self, urls: Option<Vec<String>>) -> Result<Vec<CookieInfo>> {
        let (_, page_arc) = self.get_active_page().await?;
        let page = page_arc.lock().await;

        let cookies = page
            .get_cookies()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get cookies: {}", e))?;

        Ok(cookies
            .into_iter()
            .filter(|c| {
                if let Some(ref filter_urls) = urls {
                    filter_urls.iter().any(|u| {
                        // simple domain-based filter
                        u.contains(&c.domain) || c.domain.contains(u.as_str())
                    })
                } else {
                    true
                }
            })
            .map(|c| CookieInfo {
                name: c.name,
                value: c.value,
                domain: c.domain,
                path: c.path,
                secure: c.secure,
                http_only: c.http_only,
                same_site: c.same_site.map(|s| format!("{:?}", s)),
                expires: Some(c.expires),
            })
            .collect())
    }

    pub async fn set_default_search_engine(&self, engine: Option<String>) {
        let mut se = self.default_search_engine.lock().await;
        *se = engine;
    }

    pub async fn get_default_search_engine(&self) -> Option<String> {
        self.default_search_engine.lock().await.clone()
    }
}

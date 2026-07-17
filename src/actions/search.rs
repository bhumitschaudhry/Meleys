use std::sync::Arc;
use std::sync::RwLock;

use anyhow::Result;

use crate::error::MeleyError;
use crate::observation::{ActionResult, Observation, SearchResultItem, SimplifiedNode};
use crate::selector::Selector;
use crate::session::SessionManager;

/// Trait for search engine adapters.
pub trait SearchEngineAdapter: Send + Sync {
    fn name(&self) -> &'static str;
    fn search_url(&self, query: &str) -> String;
    fn results_ready_selector(&self) -> &str;
    fn extract(&self, dom: &SimplifiedNode) -> Vec<SearchResultItem>;
}

/// DuckDuckGo adapter.
pub struct DuckDuckGoAdapter;

impl SearchEngineAdapter for DuckDuckGoAdapter {
    fn name(&self) -> &'static str { "duckduckgo" }

    fn search_url(&self, query: &str) -> String {
        let encoded = urlencoding(query);
        format!("https://html.duckduckgo.com/html/?q={}", encoded)
    }

    fn results_ready_selector(&self) -> &str {
        ".result, .result__title, #links"
    }

    fn extract(&self, dom: &SimplifiedNode) -> Vec<SearchResultItem> {
        let mut results = Vec::new();
        let mut rank = 1;
        extract_ddg_results(dom, &mut results, &mut rank);
        results
    }
}

fn extract_ddg_results(node: &SimplifiedNode, results: &mut Vec<SearchResultItem>, rank: &mut u32) {
    let tag = node.tag.as_str();
    let class = node.attributes.get("class").map(|s| s.as_str()).unwrap_or("");

    if class.contains("result__body") || class.contains("result ") || (tag == "div" && class.contains("result")) {
        // Find title and URL within this result
        let mut title = String::new();
        let mut url = String::new();
        let mut snippet = None;

        extract_ddg_result_parts(node, &mut title, &mut url, &mut snippet);

        if !title.is_empty() && !url.is_empty() {
            results.push(SearchResultItem {
                rank: *rank,
                title,
                url,
                snippet,
            });
            *rank += 1;
            return;
        }
    }

    for child in &node.children {
        extract_ddg_results(child, results, rank);
    }
}

fn extract_ddg_result_parts(node: &SimplifiedNode, title: &mut String, url: &mut String, snippet: &mut Option<String>) {
    let class = node.attributes.get("class").map(|s| s.as_str()).unwrap_or("");

    if class.contains("result__title") || class.contains("result__a") {
        if let Some(text) = &node.text {
            *title = text.trim().to_string();
        }
        if let Some(href) = node.attributes.get("href") {
            if href.starts_with("http") {
                *url = href.clone();
            }
        }
    } else if class.contains("result__snippet") {
        if let Some(text) = &node.text {
            *snippet = Some(text.trim().to_string());
        }
    }

    if node.tag == "a" && title.is_empty() {
        if let Some(text) = &node.text {
            if !text.trim().is_empty() {
                *title = text.trim().to_string();
            }
        }
        if let Some(href) = node.attributes.get("href") {
            if href.starts_with("http") && url.is_empty() {
                *url = href.clone();
            }
        }
    }

    for child in &node.children {
        extract_ddg_result_parts(child, title, url, snippet);
    }
}

/// Bing adapter.
pub struct BingAdapter;

impl SearchEngineAdapter for BingAdapter {
    fn name(&self) -> &'static str { "bing" }

    fn search_url(&self, query: &str) -> String {
        let encoded = urlencoding(query);
        format!("https://www.bing.com/search?q={}", encoded)
    }

    fn results_ready_selector(&self) -> &str {
        "#b_results, .b_algo"
    }

    fn extract(&self, dom: &SimplifiedNode) -> Vec<SearchResultItem> {
        let mut results = Vec::new();
        let mut rank = 1;
        extract_bing_results(dom, &mut results, &mut rank);
        results
    }
}

fn extract_bing_results(node: &SimplifiedNode, results: &mut Vec<SearchResultItem>, rank: &mut u32) {
    let class = node.attributes.get("class").map(|s| s.as_str()).unwrap_or("");

    if class.contains("b_algo") {
        let mut title = String::new();
        let mut url = String::new();
        let mut snippet = None;
        extract_bing_result_parts(node, &mut title, &mut url, &mut snippet, true);
        if !title.is_empty() && !url.is_empty() {
            results.push(SearchResultItem {
                rank: *rank,
                title,
                url,
                snippet,
            });
            *rank += 1;
            return;
        }
    }

    for child in &node.children {
        extract_bing_results(child, results, rank);
    }
}

fn extract_bing_result_parts(node: &SimplifiedNode, title: &mut String, url: &mut String, snippet: &mut Option<String>, first: bool) {
    if node.tag == "h2" && title.is_empty() {
        if let Some(text) = &node.text {
            *title = text.trim().to_string();
        }
        // Find link in h2
        for child in &node.children {
            if child.tag == "a" {
                if title.is_empty() {
                    if let Some(t) = &child.text { *title = t.trim().to_string(); }
                }
                if let Some(href) = child.attributes.get("href") {
                    if href.starts_with("http") && url.is_empty() {
                        *url = href.clone();
                    }
                }
            }
        }
    }

    if node.tag == "a" && first && url.is_empty() {
        if let Some(href) = node.attributes.get("href") {
            if href.starts_with("http") {
                *url = href.clone();
            }
        }
    }

    let class = node.attributes.get("class").map(|s| s.as_str()).unwrap_or("");
    if class.contains("b_caption") || class.contains("b_snippet") {
        if let Some(text) = &node.text {
            *snippet = Some(text.trim().to_string());
        }
    }

    for child in &node.children {
        extract_bing_result_parts(child, title, url, snippet, false);
    }
}

/// Google adapter.
pub struct GoogleAdapter;

impl SearchEngineAdapter for GoogleAdapter {
    fn name(&self) -> &'static str { "google" }

    fn search_url(&self, query: &str) -> String {
        let encoded = urlencoding(query);
        format!("https://www.google.com/search?q={}", encoded)
    }

    fn results_ready_selector(&self) -> &str {
        "#search, #rso, .g"
    }

    fn extract(&self, dom: &SimplifiedNode) -> Vec<SearchResultItem> {
        let mut results = Vec::new();
        let mut rank = 1;
        extract_google_results(dom, &mut results, &mut rank);
        results
    }
}

fn extract_google_results(node: &SimplifiedNode, results: &mut Vec<SearchResultItem>, rank: &mut u32) {
    let class = node.attributes.get("class").map(|s| s.as_str()).unwrap_or("");

    if class.contains(" g ") || class == "g" || class.starts_with("g ") {
        let mut title = String::new();
        let mut url = String::new();
        let mut snippet = None;
        extract_google_result_parts(node, &mut title, &mut url, &mut snippet, 0);
        if !title.is_empty() && !url.is_empty() {
            results.push(SearchResultItem {
                rank: *rank,
                title,
                url,
                snippet,
            });
            *rank += 1;
            return;
        }
    }

    for child in &node.children {
        extract_google_results(child, results, rank);
    }
}

fn extract_google_result_parts(node: &SimplifiedNode, title: &mut String, url: &mut String, snippet: &mut Option<String>, depth: u32) {
    if depth > 8 { return; }

    if (node.tag == "h3" || node.tag == "h2") && title.is_empty() {
        if let Some(text) = &node.text {
            *title = text.trim().to_string();
        }
    }

    if node.tag == "a" && url.is_empty() {
        if let Some(href) = node.attributes.get("href") {
            if href.starts_with("http") && !href.contains("google.com") {
                *url = href.clone();
            }
        }
    }

    let class = node.attributes.get("class").map(|s| s.as_str()).unwrap_or("");
    if (class.contains("VwiC3b") || class.contains("s3v9rd") || class.contains("st")) && snippet.is_none() {
        if let Some(text) = &node.text {
            if !text.trim().is_empty() {
                *snippet = Some(text.trim().to_string());
            }
        }
    }

    for child in &node.children {
        extract_google_result_parts(child, title, url, snippet, depth + 1);
    }
}

/// Search engine registry.
pub struct SearchRegistry {
    adapters: Vec<Box<dyn SearchEngineAdapter>>,
    default_engine: RwLock<String>,
}

impl SearchRegistry {
    pub fn new(default_engine: &str) -> Self {
        Self {
            adapters: vec![
                Box::new(DuckDuckGoAdapter),
                Box::new(BingAdapter),
                Box::new(GoogleAdapter),
            ],
            default_engine: RwLock::new(default_engine.to_string()),
        }
    }

    pub fn get(&self, name: &str) -> Option<&dyn SearchEngineAdapter> {
        self.adapters.iter().find(|a| a.name() == name).map(|a| a.as_ref())
    }

    pub fn default(&self) -> &dyn SearchEngineAdapter {
        let name = self.default_engine.read().unwrap().clone();
        self.get(&name).unwrap_or_else(|| self.adapters[0].as_ref())
    }

    pub fn set_default(&self, name: &str) {
        let mut d = self.default_engine.write().unwrap();
        *d = name.to_string();
    }

    pub fn default_name(&self) -> String {
        self.default_engine.read().unwrap().clone()
    }
}

/// Perform a web search.
pub async fn search_web(
    session_manager: &Arc<SessionManager>,
    session_id: &str,
    tab_id: Option<&str>,
    query: &str,
    engine: Option<&str>,
    num_results: Option<usize>,
    registry: &SearchRegistry,
    timeout_ms: Option<u64>,
) -> Observation {
    let timeout = tokio::time::Duration::from_millis(timeout_ms.unwrap_or(30000));

    let result: Result<(String, Option<String>, Option<String>, Vec<SearchResultItem>)> = async {
        let session = session_manager.get_session(session_id).await?;
        let (actual_tab_id, page_lock) = if let Some(tid) = tab_id {
            let p = session.get_page(tid).await?;
            (tid.to_string(), p)
        } else {
            session.get_active_page().await?
        };

        // Resolve engine
        let session_engine = session.get_default_search_engine().await;
        let engine_name = engine
            .map(|s| s.to_string())
            .or(session_engine)
            .unwrap_or_else(|| session_manager.default_search_engine().to_string());

        let adapter = registry.get(&engine_name)
            .ok_or_else(|| anyhow::anyhow!(MeleyError::SearchEngineParseFailed(format!("Unknown engine: {}", engine_name))))?;

        let search_url = adapter.search_url(query);
        let ready_selector = adapter.results_ready_selector().to_string();

        let page = page_lock.lock().await;

        // Navigate to search URL
        tokio::time::timeout(timeout, async {
            page.goto(&search_url).await
                .map_err(|e| anyhow::anyhow!(MeleyError::NavigationFailed(e.to_string())))?;
            page.wait_for_navigation().await
                .map_err(|e| anyhow::anyhow!(MeleyError::NavigationFailed(e.to_string())))?;
            Ok::<(), anyhow::Error>(())
        }).await
        .map_err(|_| anyhow::anyhow!(MeleyError::Timeout("Search navigation timed out".to_string())))?
        .map_err(|e| e)?;

        // Wait for results
        let wait_js = format!(
            r#"(function() {{
                var selectors = {};
                for (var i=0; i<selectors.length; i++) {{
                    if (document.querySelector(selectors[i])) return true;
                }}
                return false;
            }})()"#,
            serde_json::json!(ready_selector.split(',').map(|s| s.trim()).collect::<Vec<_>>())
        );

        let poll = tokio::time::Duration::from_millis(200);
        let wait_timeout = tokio::time::Duration::from_millis(10000);
        let _ = tokio::time::timeout(wait_timeout, async {
            loop {
                let found = page.evaluate(wait_js.clone()).await
                    .ok()
                    .and_then(|r| r.into_value::<bool>().ok())
                    .unwrap_or(false);
                if found { break; }
                tokio::time::sleep(poll).await;
            }
        }).await;

        // Extract DOM and parse results
        let dom = crate::cdp::dom::get_simplified_dom(
            &page, None, 8, false, 2000
        ).await
        .map_err(|e| anyhow::anyhow!(MeleyError::SearchEngineParseFailed(e.to_string())))?;

        let mut items = adapter.extract(&dom);
        let max = num_results.unwrap_or(10);
        items.truncate(max);

        if items.is_empty() {
            return Err(anyhow::anyhow!(MeleyError::SearchEngineParseFailed(
                format!("No results found for query: {}", query)
            )));
        }

        let url = page.url().await.ok().flatten().map(|u| u.to_string());
        let title = page.get_title().await.ok().flatten();
        Ok((actual_tab_id, url, title, items))
    }.await;

    match result {
        Ok((tid, url, title, items)) => {
            let mut obs = Observation::success(session_id, tid, "search_web", ActionResult::SearchResults(items));
            obs.url = url;
            obs.title = title;
            obs
        }
        Err(e) => {
            let (code, retryable) = error_code(&e);
            Observation::failure(session_id, tab_id.unwrap_or(""), "search_web", code, e.to_string(), retryable)
        }
    }
}

/// Set default search engine.
pub async fn set_default_search_engine(
    session_manager: &Arc<SessionManager>,
    session_id: Option<&str>,
    engine: &str,
    registry: &SearchRegistry,
) -> Observation {
    // Validate engine name
    if registry.get(engine).is_none() {
        return Observation::failure(
            session_id.unwrap_or(""),
            "",
            "set_default_search_engine",
            "INVALID_SELECTOR",
            format!("Unknown engine: {}", engine),
            false,
        );
    }

    if let Some(sid) = session_id {
        // Session-level override
        if let Ok(session) = session_manager.get_session(sid).await {
            session.set_default_search_engine(Some(engine.to_string())).await;
        }
    } else {
        // Runtime-wide
        registry.set_default(engine);
    }

    Observation::success(
        session_id.unwrap_or(""),
        "",
        "set_default_search_engine",
        ActionResult::Empty,
    )
}

/// Get the current default search engine.
pub async fn get_default_search_engine(
    session_manager: &Arc<SessionManager>,
    session_id: Option<&str>,
    registry: &SearchRegistry,
) -> Observation {
    let (engine, scope) = if let Some(sid) = session_id {
        if let Ok(session) = session_manager.get_session(sid).await {
            if let Some(e) = session.get_default_search_engine().await {
                (e, "session".to_string())
            } else {
                (registry.default_name(), "runtime".to_string())
            }
        } else {
            (registry.default_name(), "runtime".to_string())
        }
    } else {
        (registry.default_name(), "runtime".to_string())
    };

    Observation::success(
        session_id.unwrap_or(""),
        "",
        "get_default_search_engine",
        ActionResult::SearchEngine { engine, scope },
    )
}

fn urlencoding(s: &str) -> String {
    s.chars().map(|c| {
        match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            ' ' => '+'.to_string(),
            _ => {
                let bytes = c.to_string().into_bytes();
                bytes.iter().map(|b| format!("%{:02X}", b)).collect()
            }
        }
    }).collect()
}

fn error_code(e: &anyhow::Error) -> (&'static str, bool) {
    if let Some(me) = e.downcast_ref::<MeleyError>() {
        (me.error_code(), me.is_retryable())
    } else {
        ("INTERNAL_ERROR", false)
    }
}

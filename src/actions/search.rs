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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // ── URL encoding tests ────────────────────────────────────────

    #[test]
    fn test_urlencoding_ascii_letters() {
        assert_eq!(urlencoding("hello"), "hello");
    }

    #[test]
    fn test_urlencoding_digits() {
        assert_eq!(urlencoding("123"), "123");
    }

    #[test]
    fn test_urlencoding_unreserved_chars() {
        assert_eq!(urlencoding("-_.~"), "-_.~");
    }

    #[test]
    fn test_urlencoding_space() {
        assert_eq!(urlencoding("hello world"), "hello+world");
    }

    #[test]
    fn test_urlencoding_special_chars() {
        let encoded = urlencoding("a&b=c");
        assert!(encoded.contains("%26"));
        assert!(encoded.contains("%3D"));
    }

    #[test]
    fn test_urlencoding_empty() {
        assert_eq!(urlencoding(""), "");
    }

    #[test]
    fn test_urlencoding_unicode() {
        let encoded = urlencoding("café");
        assert!(encoded.contains("%"));
        assert!(!encoded.contains("é"));
    }

    #[test]
    fn test_urlencoding_plus_sign() {
        let encoded = urlencoding("a+b");
        assert!(encoded.contains("%2B"));
    }

    #[test]
    fn test_urlencoding_slash() {
        let encoded = urlencoding("path/to/file");
        assert!(encoded.contains("%2F"));
    }

    #[test]
    fn test_urlencoding_question_mark() {
        let encoded = urlencoding("what?");
        assert!(encoded.contains("%3F"));
    }

    #[test]
    fn test_urlencoding_hash() {
        let encoded = urlencoding("page#section");
        assert!(encoded.contains("%23"));
    }

    #[test]
    fn test_urlencoding_mixed() {
        let encoded = urlencoding("rust async runtime");
        assert!(encoded.contains('+'));
        assert!(!encoded.contains(' '));
    }

    // ── DuckDuckGo adapter tests ──────────────────────────────────

    #[test]
    fn test_ddg_adapter_name() {
        let adapter = DuckDuckGoAdapter;
        assert_eq!(adapter.name(), "duckduckgo");
    }

    #[test]
    fn test_ddg_search_url() {
        let adapter = DuckDuckGoAdapter;
        let url = adapter.search_url("rust lang");
        assert!(url.starts_with("https://html.duckduckgo.com/html/?q="));
        assert!(url.contains("rust"));
        assert!(url.contains("lang"));
    }

    #[test]
    fn test_ddg_results_ready_selector() {
        let adapter = DuckDuckGoAdapter;
        let sel = adapter.results_ready_selector();
        assert!(!sel.is_empty());
        assert!(sel.contains(".result"));
    }

    #[test]
    fn test_ddg_extract_single_result() {
        let adapter = DuckDuckGoAdapter;
        let dom = SimplifiedNode {
            backend_node_id: 0,
            tag: "div".into(),
            attributes: HashMap::new(),
            text: None,
            visible: true,
            bounding_box: None,
            children: vec![SimplifiedNode {
                backend_node_id: 1,
                tag: "div".into(),
                attributes: {
                    let mut m = HashMap::new();
                    m.insert("class".into(), "result".into());
                    m
                },
                text: None,
                visible: true,
                bounding_box: None,
                children: vec![
                    SimplifiedNode {
                        backend_node_id: 2,
                        tag: "a".into(),
                        attributes: {
                            let mut m = HashMap::new();
                            m.insert("class".into(), "result__a".into());
                            m.insert("href".into(), "https://example.com".into());
                            m
                        },
                        text: Some("Example Site".into()),
                        visible: true,
                        bounding_box: None,
                        children: vec![],
                    },
                    SimplifiedNode {
                        backend_node_id: 3,
                        tag: "td".into(),
                        attributes: {
                            let mut m = HashMap::new();
                            m.insert("class".into(), "result__snippet".into());
                            m
                        },
                        text: Some("An example snippet".into()),
                        visible: true,
                        bounding_box: None,
                        children: vec![],
                    },
                ],
            }],
        };

        let results = adapter.extract(&dom);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Example Site");
        assert_eq!(results[0].url, "https://example.com");
        assert_eq!(results[0].snippet.as_deref(), Some("An example snippet"));
        assert_eq!(results[0].rank, 1);
    }

    #[test]
    fn test_ddg_extract_no_results() {
        let adapter = DuckDuckGoAdapter;
        let dom = SimplifiedNode {
            backend_node_id: 0,
            tag: "div".into(),
            attributes: HashMap::new(),
            text: Some("No results".into()),
            visible: true,
            bounding_box: None,
            children: vec![],
        };
        let results = adapter.extract(&dom);
        assert!(results.is_empty());
    }

    #[test]
    fn test_ddg_extract_multiple_results() {
        let adapter = DuckDuckGoAdapter;
        let make_result = |id: i64, title: &str, url: &str, snippet: &str| -> SimplifiedNode {
            SimplifiedNode {
                backend_node_id: id,
                tag: "div".into(),
                attributes: {
                    let mut m = HashMap::new();
                    m.insert("class".into(), "result".into());
                    m
                },
                text: None,
                visible: true,
                bounding_box: None,
                children: vec![
                    SimplifiedNode {
                        backend_node_id: id + 1,
                        tag: "a".into(),
                        attributes: {
                            let mut m = HashMap::new();
                            m.insert("class".into(), "result__a".into());
                            m.insert("href".into(), url.into());
                            m
                        },
                        text: Some(title.into()),
                        visible: true,
                        bounding_box: None,
                        children: vec![],
                    },
                    SimplifiedNode {
                        backend_node_id: id + 2,
                        tag: "td".into(),
                        attributes: {
                            let mut m = HashMap::new();
                            m.insert("class".into(), "result__snippet".into());
                            m
                        },
                        text: Some(snippet.into()),
                        visible: true,
                        bounding_box: None,
                        children: vec![],
                    },
                ],
            }
        };

        let dom = SimplifiedNode {
            backend_node_id: 0,
            tag: "div".into(),
            attributes: HashMap::new(),
            text: None,
            visible: true,
            bounding_box: None,
            children: vec![
                make_result(1, "First", "https://first.com", "First snippet"),
                make_result(10, "Second", "https://second.com", "Second snippet"),
                make_result(20, "Third", "https://third.com", "Third snippet"),
            ],
        };

        let results = adapter.extract(&dom);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].rank, 1);
        assert_eq!(results[1].rank, 2);
        assert_eq!(results[2].rank, 3);
        assert_eq!(results[0].title, "First");
        assert_eq!(results[1].url, "https://second.com");
        assert_eq!(results[2].snippet.as_deref(), Some("Third snippet"));
    }

    #[test]
    fn test_ddg_extract_skips_empty_title_or_url() {
        let adapter = DuckDuckGoAdapter;
        let dom = SimplifiedNode {
            backend_node_id: 0,
            tag: "div".into(),
            attributes: HashMap::new(),
            text: None,
            visible: true,
            bounding_box: None,
            children: vec![SimplifiedNode {
                backend_node_id: 1,
                tag: "div".into(),
                attributes: {
                    let mut m = HashMap::new();
                    m.insert("class".into(), "result".into());
                    m
                },
                text: None,
                visible: true,
                bounding_box: None,
                children: vec![SimplifiedNode {
                    backend_node_id: 2,
                    tag: "a".into(),
                    attributes: {
                        let mut m = HashMap::new();
                        m.insert("class".into(), "result__a".into());
                        // No href = empty URL
                        m
                    },
                    text: Some("Title".into()),
                    visible: true,
                    bounding_box: None,
                    children: vec![],
                }],
            }],
        };
        let results = adapter.extract(&dom);
        // Should skip because URL is empty
        assert!(results.is_empty());
    }

    // ── Bing adapter tests ────────────────────────────────────────

    #[test]
    fn test_bing_adapter_name() {
        let adapter = BingAdapter;
        assert_eq!(adapter.name(), "bing");
    }

    #[test]
    fn test_bing_search_url() {
        let adapter = BingAdapter;
        let url = adapter.search_url("rust programming");
        assert!(url.starts_with("https://www.bing.com/search?q="));
        assert!(url.contains("rust"));
    }

    #[test]
    fn test_bing_results_ready_selector() {
        let adapter = BingAdapter;
        let sel = adapter.results_ready_selector();
        assert!(sel.contains("#b_results"));
        assert!(sel.contains(".b_algo"));
    }

    #[test]
    fn test_bing_extract_single_result() {
        let adapter = BingAdapter;
        let dom = SimplifiedNode {
            backend_node_id: 0,
            tag: "ol".into(),
            attributes: {
                let mut m = HashMap::new();
                m.insert("id".into(), "b_results".into());
                m
            },
            text: None,
            visible: true,
            bounding_box: None,
            children: vec![SimplifiedNode {
                backend_node_id: 1,
                tag: "li".into(),
                attributes: {
                    let mut m = HashMap::new();
                    m.insert("class".into(), "b_algo".into());
                    m
                },
                text: None,
                visible: true,
                bounding_box: None,
                children: vec![
                    SimplifiedNode {
                        backend_node_id: 2,
                        tag: "h2".into(),
                        attributes: HashMap::new(),
                        text: None,
                        visible: true,
                        bounding_box: None,
                        children: vec![SimplifiedNode {
                            backend_node_id: 3,
                            tag: "a".into(),
                            attributes: {
                                let mut m = HashMap::new();
                                m.insert("href".into(), "https://rust-lang.org".into());
                                m
                            },
                            text: Some("Rust Language".into()),
                            visible: true,
                            bounding_box: None,
                            children: vec![],
                        }],
                    },
                    SimplifiedNode {
                        backend_node_id: 4,
                        tag: "div".into(),
                        attributes: {
                            let mut m = HashMap::new();
                            m.insert("class".into(), "b_caption".into());
                            m
                        },
                        text: Some("Systems programming language".into()),
                        visible: true,
                        bounding_box: None,
                        children: vec![],
                    },
                ],
            }],
        };

        let results = adapter.extract(&dom);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Rust Language");
        assert_eq!(results[0].url, "https://rust-lang.org");
        assert_eq!(results[0].snippet.as_deref(), Some("Systems programming language"));
    }

    #[test]
    fn test_bing_extract_no_results() {
        let adapter = BingAdapter;
        let dom = SimplifiedNode {
            backend_node_id: 0,
            tag: "div".into(),
            attributes: HashMap::new(),
            text: Some("No results".into()),
            visible: true,
            bounding_box: None,
            children: vec![],
        };
        let results = adapter.extract(&dom);
        assert!(results.is_empty());
    }

    // ── Google adapter tests ──────────────────────────────────────

    #[test]
    fn test_google_adapter_name() {
        let adapter = GoogleAdapter;
        assert_eq!(adapter.name(), "google");
    }

    #[test]
    fn test_google_search_url() {
        let adapter = GoogleAdapter;
        let url = adapter.search_url("test query");
        assert!(url.starts_with("https://www.google.com/search?q="));
        assert!(url.contains("test"));
    }

    #[test]
    fn test_google_results_ready_selector() {
        let adapter = GoogleAdapter;
        let sel = adapter.results_ready_selector();
        assert!(sel.contains("#search"));
        assert!(sel.contains(".g"));
    }

    #[test]
    fn test_google_extract_single_result() {
        let adapter = GoogleAdapter;
        let dom = SimplifiedNode {
            backend_node_id: 0,
            tag: "div".into(),
            attributes: {
                let mut m = HashMap::new();
                m.insert("id".into(), "rso".into());
                m
            },
            text: None,
            visible: true,
            bounding_box: None,
            children: vec![SimplifiedNode {
                backend_node_id: 1,
                tag: "div".into(),
                attributes: {
                    let mut m = HashMap::new();
                    m.insert("class".into(), "g".into());
                    m
                },
                text: None,
                visible: true,
                bounding_box: None,
                children: vec![
                    SimplifiedNode {
                        backend_node_id: 2,
                        tag: "div".into(),
                        attributes: HashMap::new(),
                        text: None,
                        visible: true,
                        bounding_box: None,
                        children: vec![
                            SimplifiedNode {
                                backend_node_id: 3,
                                tag: "a".into(),
                                attributes: {
                                    let mut m = HashMap::new();
                                    m.insert("href".into(), "https://crates.io".into());
                                    m
                                },
                                text: None,
                                visible: true,
                                bounding_box: None,
                                children: vec![SimplifiedNode {
                                    backend_node_id: 4,
                                    tag: "h3".into(),
                                    attributes: HashMap::new(),
                                    text: Some("crates.io".into()),
                                    visible: true,
                                    bounding_box: None,
                                    children: vec![],
                                }],
                            },
                            SimplifiedNode {
                                backend_node_id: 5,
                                tag: "div".into(),
                                attributes: {
                                    let mut m = HashMap::new();
                                    m.insert("class".into(), "VwiC3b".into());
                                    m
                                },
                                text: Some("The Rust community's crate registry".into()),
                                visible: true,
                                bounding_box: None,
                                children: vec![],
                            },
                        ],
                    },
                ],
            }],
        };

        let results = adapter.extract(&dom);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "crates.io");
        assert_eq!(results[0].url, "https://crates.io");
        assert!(results[0].snippet.is_some());
    }

    #[test]
    fn test_google_extract_no_results() {
        let adapter = GoogleAdapter;
        let dom = SimplifiedNode {
            backend_node_id: 0,
            tag: "div".into(),
            attributes: HashMap::new(),
            text: Some("No results".into()),
            visible: true,
            bounding_box: None,
            children: vec![],
        };
        let results = adapter.extract(&dom);
        assert!(results.is_empty());
    }

    #[test]
    fn test_google_extract_skips_google_urls() {
        let adapter = GoogleAdapter;
        let dom = SimplifiedNode {
            backend_node_id: 0,
            tag: "div".into(),
            attributes: HashMap::new(),
            text: None,
            visible: true,
            bounding_box: None,
            children: vec![SimplifiedNode {
                backend_node_id: 1,
                tag: "div".into(),
                attributes: {
                    let mut m = HashMap::new();
                    m.insert("class".into(), "g".into());
                    m
                },
                text: None,
                visible: true,
                bounding_box: None,
                children: vec![SimplifiedNode {
                    backend_node_id: 2,
                    tag: "a".into(),
                    attributes: {
                        let mut m = HashMap::new();
                        m.insert("href".into(), "https://www.google.com/search?q=test".into());
                        m
                    },
                    text: Some("Google Link".into()),
                    visible: true,
                    bounding_box: None,
                    children: vec![SimplifiedNode {
                        backend_node_id: 3,
                        tag: "h3".into(),
                        attributes: HashMap::new(),
                        text: Some("Google Result".into()),
                        visible: true,
                        bounding_box: None,
                        children: vec![],
                    }],
                }],
            }],
        };
        let results = adapter.extract(&dom);
        // Should be empty because the URL is a google.com URL
        assert!(results.is_empty());
    }

    // ── SearchRegistry tests ──────────────────────────────────────

    #[test]
    fn test_registry_default_engine() {
        let registry = SearchRegistry::new("bing");
        assert_eq!(registry.default_name(), "bing");
    }

    #[test]
    fn test_registry_get_existing() {
        let registry = SearchRegistry::new("duckduckgo");
        assert!(registry.get("google").is_some());
        assert!(registry.get("bing").is_some());
        assert!(registry.get("duckduckgo").is_some());
    }

    #[test]
    fn test_registry_get_nonexistent() {
        let registry = SearchRegistry::new("duckduckgo");
        assert!(registry.get("yahoo").is_none());
        assert!(registry.get("").is_none());
    }

    #[test]
    fn test_registry_default_method() {
        let registry = SearchRegistry::new("google");
        let adapter = registry.default();
        assert_eq!(adapter.name(), "google");
    }

    #[test]
    fn test_registry_default_fallback_for_unknown() {
        let registry = SearchRegistry::new("unknown_engine");
        let adapter = registry.default();
        // Should fall back to first adapter (DuckDuckGo)
        assert_eq!(adapter.name(), "duckduckgo");
    }

    #[test]
    fn test_registry_set_default() {
        let registry = SearchRegistry::new("duckduckgo");
        registry.set_default("bing");
        assert_eq!(registry.default_name(), "bing");
        registry.set_default("google");
        assert_eq!(registry.default_name(), "google");
    }

    #[test]
    fn test_registry_all_adapters_present() {
        let registry = SearchRegistry::new("duckduckgo");
        let names: Vec<&str> = ["google", "bing", "duckduckgo"]
            .iter()
            .filter(|n| registry.get(n).is_some())
            .map(|s| *s)
            .collect();
        assert_eq!(names.len(), 3);
    }

    #[test]
    fn test_all_adapters_produce_valid_search_urls() {
        let adapters: Vec<Box<dyn SearchEngineAdapter>> = vec![
            Box::new(DuckDuckGoAdapter),
            Box::new(BingAdapter),
            Box::new(GoogleAdapter),
        ];
        for adapter in &adapters {
            let url = adapter.search_url("test");
            assert!(url.starts_with("https://"), "Adapter {} URL should start with https://", adapter.name());
            assert!(url.contains("test"), "Adapter {} URL should contain query", adapter.name());
            let ready = adapter.results_ready_selector();
            assert!(!ready.is_empty(), "Adapter {} should have a ready selector", adapter.name());
        }
    }
}

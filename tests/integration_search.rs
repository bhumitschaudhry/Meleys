use meleys::actions::search::{DuckDuckGoAdapter, SearchEngineAdapter, SearchRegistry};
use meleys::observation::SimplifiedNode;
use std::collections::HashMap;

#[test]
fn test_duckduckgo_adapter_extraction() {
    let adapter = DuckDuckGoAdapter;

    // Construct a mock DOM node for a DuckDuckGo result
    let result_node = SimplifiedNode {
        backend_node_id: 1,
        tag: "div".to_string(),
        attributes: {
            let mut m = HashMap::new();
            m.insert("class".to_string(), "result".to_string());
            m
        },
        text: None,
        visible: true,
        bounding_box: None,
        children: vec![
            SimplifiedNode {
                backend_node_id: 2,
                tag: "a".to_string(),
                attributes: {
                    let mut m = HashMap::new();
                    m.insert("class".to_string(), "result__a".to_string());
                    m.insert("href".to_string(), "https://rust-lang.org".to_string());
                    m
                },
                text: Some("Rust Programming Language".to_string()),
                visible: true,
                bounding_box: None,
                children: vec![],
            },
            SimplifiedNode {
                backend_node_id: 3,
                tag: "td".to_string(),
                attributes: {
                    let mut m = HashMap::new();
                    m.insert("class".to_string(), "result__snippet".to_string());
                    m
                },
                text: Some(
                    "A language empowering everyone to build reliable and efficient software."
                        .to_string(),
                ),
                visible: true,
                bounding_box: None,
                children: vec![],
            },
        ],
    };

    let parent_node = SimplifiedNode {
        backend_node_id: 0,
        tag: "div".to_string(),
        attributes: HashMap::new(),
        text: None,
        visible: true,
        bounding_box: None,
        children: vec![result_node],
    };

    let results = adapter.extract(&parent_node);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Rust Programming Language");
    assert_eq!(results[0].url, "https://rust-lang.org");
    assert_eq!(
        results[0].snippet.as_deref(),
        Some("A language empowering everyone to build reliable and efficient software.")
    );
}

#[test]
fn test_registry() {
    let registry = SearchRegistry::new("duckduckgo");
    assert_eq!(registry.default_name(), "duckduckgo");

    let engine = registry.get("google");
    assert!(engine.is_some());
    assert_eq!(engine.unwrap().name(), "google");
}

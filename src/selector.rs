use serde::{Deserialize, Serialize};

/// A selector that tells Meleys which element to act on.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum Selector {
    Css(String),
    XPath(String),
    AxNodeId(String),
    BackendNodeId(i64),
    Coordinates { x: f64, y: f64 },
    Text { exact: bool, value: String },
}

impl Selector {
    /// Convert this selector to a CSS selector string if possible.
    /// For XPath, wraps in a JS expression that would be evaluated.
    pub fn to_css_string(&self) -> Option<String> {
        match self {
            Selector::Css(s) => Some(s.clone()),
            _ => None,
        }
    }

    /// Returns a JS expression that finds the element and returns it.
    pub fn to_js_expression(&self) -> String {
        match self {
            Selector::Css(css) => {
                format!("document.querySelector({})", serde_json::json!(css))
            }
            Selector::XPath(xpath) => {
                format!(
                    "document.evaluate({}, document, null, XPathResult.FIRST_ORDERED_NODE_TYPE, null).singleNodeValue",
                    serde_json::json!(xpath)
                )
            }
            Selector::Text { exact, value } => {
                if *exact {
                    format!(
                        r#"(function() {{ 
                            var all = document.querySelectorAll('*');
                            for(var i=0;i<all.length;i++) {{
                                if(all[i].textContent.trim() === {}) return all[i];
                            }}
                            return null;
                        }})()"#,
                        serde_json::json!(value)
                    )
                } else {
                    format!(
                        r#"(function() {{ 
                            var all = document.querySelectorAll('*');
                            for(var i=0;i<all.length;i++) {{
                                if(all[i].textContent.trim().includes({})) return all[i];
                            }}
                            return null;
                        }})()"#,
                        serde_json::json!(value)
                    )
                }
            }
            Selector::Coordinates { .. } => {
                "null".to_string() // handled separately via mouse events
            }
            Selector::AxNodeId(id) => {
                format!(r#"document.querySelector('[data-ax-node-id="{}"]')"#, id)
            }
            Selector::BackendNodeId(_) => {
                "null".to_string() // handled via CDP DOM.resolveNode
            }
        }
    }

    /// Returns a human-readable description.
    pub fn description(&self) -> String {
        match self {
            Selector::Css(s) => format!("CSS({})", s),
            Selector::XPath(s) => format!("XPath({})", s),
            Selector::AxNodeId(s) => format!("AxNodeId({})", s),
            Selector::BackendNodeId(id) => format!("BackendNodeId({})", id),
            Selector::Coordinates { x, y } => format!("Coordinates({}, {})", x, y),
            Selector::Text { exact, value } => {
                format!("Text(exact={}, {:?})", exact, value)
            }
        }
    }
}

impl Default for Selector {
    fn default() -> Self {
        Selector::Css("body".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_selector_is_body_css() {
        let sel = Selector::default();
        match &sel {
            Selector::Css(s) => assert_eq!(s, "body"),
            _ => panic!("Default should be CSS"),
        }
    }

    #[test]
    fn test_css_to_css_string() {
        let sel = Selector::Css("#main".into());
        assert_eq!(sel.to_css_string(), Some("#main".into()));
    }

    #[test]
    fn test_xpath_to_css_string_returns_none() {
        let sel = Selector::XPath("//div".into());
        assert!(sel.to_css_string().is_none());
    }

    #[test]
    fn test_text_to_css_string_returns_none() {
        let sel = Selector::Text {
            exact: true,
            value: "Click".into(),
        };
        assert!(sel.to_css_string().is_none());
    }

    #[test]
    fn test_coordinates_to_css_string_returns_none() {
        let sel = Selector::Coordinates { x: 10.0, y: 20.0 };
        assert!(sel.to_css_string().is_none());
    }

    #[test]
    fn test_ax_node_id_to_css_string_returns_none() {
        let sel = Selector::AxNodeId("1".into());
        assert!(sel.to_css_string().is_none());
    }

    #[test]
    fn test_backend_node_id_to_css_string_returns_none() {
        let sel = Selector::BackendNodeId(42);
        assert!(sel.to_css_string().is_none());
    }

    #[test]
    fn test_css_to_js_expression() {
        let sel = Selector::Css(".button".into());
        let js = sel.to_js_expression();
        assert!(js.contains("document.querySelector"));
        assert!(js.contains(".button"));
    }

    #[test]
    fn test_xpath_to_js_expression() {
        let sel = Selector::XPath("//div[@class='test']".into());
        let js = sel.to_js_expression();
        assert!(js.contains("document.evaluate"));
        assert!(js.contains("//div[@class='test']"));
    }

    #[test]
    fn test_text_exact_to_js_expression() {
        let sel = Selector::Text {
            exact: true,
            value: "Submit".into(),
        };
        let js = sel.to_js_expression();
        assert!(js.contains("textContent.trim() ==="));
        assert!(js.contains("Submit"));
    }

    #[test]
    fn test_text_partial_to_js_expression() {
        let sel = Selector::Text {
            exact: false,
            value: "Sub".into(),
        };
        let js = sel.to_js_expression();
        assert!(js.contains("textContent.trim().includes"));
        assert!(js.contains("Sub"));
    }

    #[test]
    fn test_coordinates_to_js_expression() {
        let sel = Selector::Coordinates { x: 100.0, y: 200.0 };
        let js = sel.to_js_expression();
        assert_eq!(js, "null");
    }

    #[test]
    fn test_ax_node_id_to_js_expression() {
        let sel = Selector::AxNodeId("node-5".into());
        let js = sel.to_js_expression();
        assert!(js.contains("data-ax-node-id"));
        assert!(js.contains("node-5"));
    }

    #[test]
    fn test_backend_node_id_to_js_expression() {
        let sel = Selector::BackendNodeId(99);
        let js = sel.to_js_expression();
        assert_eq!(js, "null");
    }

    #[test]
    fn test_css_description() {
        let sel = Selector::Css("#app".into());
        assert_eq!(sel.description(), "CSS(#app)");
    }

    #[test]
    fn test_xpath_description() {
        let sel = Selector::XPath("//body".into());
        assert_eq!(sel.description(), "XPath(//body)");
    }

    #[test]
    fn test_text_exact_description() {
        let sel = Selector::Text {
            exact: true,
            value: "OK".into(),
        };
        assert_eq!(sel.description(), r#"Text(exact=true, "OK")"#);
    }

    #[test]
    fn test_text_partial_description() {
        let sel = Selector::Text {
            exact: false,
            value: "Ok".into(),
        };
        assert_eq!(sel.description(), r#"Text(exact=false, "Ok")"#);
    }

    #[test]
    fn test_coordinates_description() {
        let sel = Selector::Coordinates { x: 1.5, y: 2.5 };
        assert_eq!(sel.description(), "Coordinates(1.5, 2.5)");
    }

    #[test]
    fn test_ax_node_id_description() {
        let sel = Selector::AxNodeId("abc".into());
        assert_eq!(sel.description(), "AxNodeId(abc)");
    }

    #[test]
    fn test_backend_node_id_description() {
        let sel = Selector::BackendNodeId(7);
        assert_eq!(sel.description(), "BackendNodeId(7)");
    }

    #[test]
    fn test_selector_json_roundtrip_css() {
        let sel = Selector::Css("div > span".into());
        let json = serde_json::to_string(&sel).unwrap();
        let d: Selector = serde_json::from_str(&json).unwrap();
        assert_eq!(d.to_css_string(), Some("div > span".into()));
    }

    #[test]
    fn test_selector_json_roundtrip_xpath() {
        let sel = Selector::XPath("//a".into());
        let json = serde_json::to_string(&sel).unwrap();
        let d: Selector = serde_json::from_str(&json).unwrap();
        match d {
            Selector::XPath(s) => assert_eq!(s, "//a"),
            _ => panic!("Expected XPath"),
        }
    }

    #[test]
    fn test_selector_json_roundtrip_coordinates() {
        let sel = Selector::Coordinates { x: 10.0, y: 20.0 };
        let json = serde_json::to_string(&sel).unwrap();
        let d: Selector = serde_json::from_str(&json).unwrap();
        match d {
            Selector::Coordinates { x, y } => {
                assert_eq!(x, 10.0);
                assert_eq!(y, 20.0);
            }
            _ => panic!("Expected Coordinates"),
        }
    }

    #[test]
    fn test_selector_json_roundtrip_text() {
        let sel = Selector::Text {
            exact: false,
            value: "hello world".into(),
        };
        let json = serde_json::to_string(&sel).unwrap();
        let d: Selector = serde_json::from_str(&json).unwrap();
        match d {
            Selector::Text { exact, value } => {
                assert!(!exact);
                assert_eq!(value, "hello world");
            }
            _ => panic!("Expected Text"),
        }
    }

    #[test]
    fn test_selector_json_roundtrip_backend_node_id() {
        let sel = Selector::BackendNodeId(123);
        let json = serde_json::to_string(&sel).unwrap();
        let d: Selector = serde_json::from_str(&json).unwrap();
        match d {
            Selector::BackendNodeId(id) => assert_eq!(id, 123),
            _ => panic!("Expected BackendNodeId"),
        }
    }

    #[test]
    fn test_selector_clone() {
        let sel = Selector::Css("a".into());
        let cloned = sel.clone();
        assert_eq!(cloned.description(), sel.description());
    }
}

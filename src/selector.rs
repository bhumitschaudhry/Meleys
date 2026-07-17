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
                format!(
                    r#"document.querySelector('[data-ax-node-id="{}"]')"#,
                    id
                )
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

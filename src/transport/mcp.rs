/// MCP JSON-RPC 2.0 server over stdio.
/// 
/// Protocol:
/// - Client sends: `{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"navigate","arguments":{...}}}`
/// - Server responds: `{"jsonrpc":"2.0","id":1,"result":{...}}`
/// - Also handles: `initialize`, `tools/list`

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::actions::download::DownloadRegistry;
use crate::actions::search::SearchRegistry;
use crate::observation::{CookieInfo, Observation};
use crate::selector::Selector;
use crate::session::SessionManager;

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Value>,
}

impl JsonRpcResponse {
    fn ok(id: Option<Value>, result: Value) -> Self {
        Self { jsonrpc: "2.0".to_string(), id, result: Some(result), error: None }
    }

    fn err(id: Option<Value>, code: i64, message: &str) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(json!({"code": code, "message": message})),
        }
    }
}

fn tool_schema(name: &str, description: &str, properties: Value, required: Vec<&str>) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": {
            "type": "object",
            "properties": properties,
            "required": required,
        }
    })
}

fn tools_list() -> Value {
    json!([
        tool_schema("create_session", "Create a new browser session", json!({
            "profile_name": {"type": "string", "description": "Optional profile name for persistence"},
            "headless": {"type": "boolean", "description": "Run headless (default: true)"},
            "default_search_engine": {"type": "string", "enum": ["google", "bing", "duckduckgo"]}
        }), vec![]),
        tool_schema("close_session", "Close a browser session", json!({
            "session_id": {"type": "string"}
        }), vec!["session_id"]),
        tool_schema("list_sessions", "List all active sessions", json!({}), vec![]),
        tool_schema("new_tab", "Open a new tab", json!({
            "session_id": {"type": "string"},
            "url": {"type": "string"}
        }), vec!["session_id"]),
        tool_schema("close_tab", "Close a tab", json!({
            "session_id": {"type": "string"},
            "tab_id": {"type": "string"}
        }), vec!["session_id", "tab_id"]),
        tool_schema("list_tabs", "List tabs in a session", json!({
            "session_id": {"type": "string"}
        }), vec!["session_id"]),
        tool_schema("switch_tab", "Switch active tab", json!({
            "session_id": {"type": "string"},
            "tab_id": {"type": "string"}
        }), vec!["session_id", "tab_id"]),
        tool_schema("navigate", "Navigate to a URL", json!({
            "session_id": {"type": "string"},
            "tab_id": {"type": "string"},
            "url": {"type": "string"},
            "wait_until": {"type": "string", "enum": ["load", "domcontentloaded", "networkidle"]},
            "timeout_ms": {"type": "integer"}
        }), vec!["session_id", "url"]),
        tool_schema("go_back", "Go back in browser history", json!({
            "session_id": {"type": "string"},
            "tab_id": {"type": "string"},
            "timeout_ms": {"type": "integer"}
        }), vec!["session_id"]),
        tool_schema("go_forward", "Go forward in browser history", json!({
            "session_id": {"type": "string"},
            "tab_id": {"type": "string"},
            "timeout_ms": {"type": "integer"}
        }), vec!["session_id"]),
        tool_schema("reload", "Reload the current page", json!({
            "session_id": {"type": "string"},
            "tab_id": {"type": "string"},
            "ignore_cache": {"type": "boolean"},
            "timeout_ms": {"type": "integer"}
        }), vec!["session_id"]),
        tool_schema("wait_for", "Wait for a condition", json!({
            "session_id": {"type": "string"},
            "tab_id": {"type": "string"},
            "condition": {"type": "string", "enum": ["selector", "navigation", "timeout", "js_expression"]},
            "selector": {"type": "string"},
            "state": {"type": "string"},
            "timeout_ms": {"type": "integer"},
            "js_expr": {"type": "string"},
            "sleep_ms": {"type": "integer"}
        }), vec!["session_id", "condition"]),
        tool_schema("click", "Click on an element", json!({
            "session_id": {"type": "string"},
            "tab_id": {"type": "string"},
            "selector": {"type": "object", "description": "Selector object"},
            "button": {"type": "string"},
            "click_count": {"type": "integer"},
            "nth": {"type": "integer"}
        }), vec!["session_id", "selector"]),
        tool_schema("type_text", "Type text into an element", json!({
            "session_id": {"type": "string"},
            "tab_id": {"type": "string"},
            "selector": {"type": "object"},
            "text": {"type": "string"},
            "clear_first": {"type": "boolean"},
            "delay_ms": {"type": "integer"}
        }), vec!["session_id", "selector", "text"]),
        tool_schema("press_key", "Press a keyboard key", json!({
            "session_id": {"type": "string"},
            "tab_id": {"type": "string"},
            "key": {"type": "string"},
            "selector": {"type": "object"}
        }), vec!["session_id", "key"]),
        tool_schema("hover", "Hover over an element", json!({
            "session_id": {"type": "string"},
            "tab_id": {"type": "string"},
            "selector": {"type": "object"}
        }), vec!["session_id", "selector"]),
        tool_schema("scroll", "Scroll the page", json!({
            "session_id": {"type": "string"},
            "tab_id": {"type": "string"},
            "direction": {"type": "string", "enum": ["up", "down", "left", "right"]},
            "amount_px": {"type": "integer"},
            "to_bottom": {"type": "boolean"}
        }), vec!["session_id"]),
        tool_schema("select_option", "Select an option in a <select>", json!({
            "session_id": {"type": "string"},
            "tab_id": {"type": "string"},
            "selector": {"type": "object"},
            "value": {"type": "string"}
        }), vec!["session_id", "selector", "value"]),
        tool_schema("set_file_input", "Set file input files", json!({
            "session_id": {"type": "string"},
            "tab_id": {"type": "string"},
            "selector": {"type": "object"},
            "file_paths": {"type": "array", "items": {"type": "string"}}
        }), vec!["session_id", "selector", "file_paths"]),
        tool_schema("get_text", "Get text content", json!({
            "session_id": {"type": "string"},
            "tab_id": {"type": "string"},
            "selector": {"type": "object"},
            "max_chars": {"type": "integer"}
        }), vec!["session_id"]),
        tool_schema("get_links", "Get all links", json!({
            "session_id": {"type": "string"},
            "tab_id": {"type": "string"},
            "selector": {"type": "string"},
            "same_origin_only": {"type": "boolean"}
        }), vec!["session_id"]),
        tool_schema("get_dom", "Get simplified DOM", json!({
            "session_id": {"type": "string"},
            "tab_id": {"type": "string"},
            "selector": {"type": "object"},
            "max_depth": {"type": "integer"},
            "include_hidden": {"type": "boolean"},
            "max_nodes": {"type": "integer"}
        }), vec!["session_id"]),
        tool_schema("get_ax_tree", "Get accessibility tree", json!({
            "session_id": {"type": "string"},
            "tab_id": {"type": "string"},
            "selector": {"type": "object"},
            "max_depth": {"type": "integer"}
        }), vec!["session_id"]),
        tool_schema("query_elements", "Query elements by selector", json!({
            "session_id": {"type": "string"},
            "tab_id": {"type": "string"},
            "selector": {"type": "object"},
            "limit": {"type": "integer"}
        }), vec!["session_id", "selector"]),
        tool_schema("evaluate_js", "Evaluate JavaScript", json!({
            "session_id": {"type": "string"},
            "tab_id": {"type": "string"},
            "expression": {"type": "string"}
        }), vec!["session_id", "expression"]),
        tool_schema("screenshot", "Take a screenshot", json!({
            "session_id": {"type": "string"},
            "tab_id": {"type": "string"},
            "selector": {"type": "object"},
            "full_page": {"type": "boolean"},
            "format": {"type": "string", "enum": ["png", "jpeg"]}
        }), vec!["session_id"]),
        tool_schema("export_pdf", "Export page as PDF", json!({
            "session_id": {"type": "string"},
            "tab_id": {"type": "string"},
            "landscape": {"type": "boolean"}
        }), vec!["session_id"]),
        tool_schema("download_file", "Download a file", json!({
            "session_id": {"type": "string"},
            "tab_id": {"type": "string"},
            "url": {"type": "string"},
            "save_as": {"type": "string"}
        }), vec!["session_id", "url"]),
        tool_schema("list_downloads", "List downloads for session", json!({
            "session_id": {"type": "string"}
        }), vec!["session_id"]),
        tool_schema("get_cookies", "Get cookies", json!({
            "session_id": {"type": "string"},
            "tab_id": {"type": "string"},
            "urls": {"type": "array", "items": {"type": "string"}}
        }), vec!["session_id"]),
        tool_schema("set_cookies", "Set cookies", json!({
            "session_id": {"type": "string"},
            "tab_id": {"type": "string"},
            "cookies": {"type": "array"}
        }), vec!["session_id", "cookies"]),
        tool_schema("clear_cookies", "Clear all cookies", json!({
            "session_id": {"type": "string"},
            "tab_id": {"type": "string"}
        }), vec!["session_id"]),
        tool_schema("get_local_storage", "Get localStorage content", json!({
            "session_id": {"type": "string"},
            "tab_id": {"type": "string"},
            "origin": {"type": "string"}
        }), vec!["session_id"]),
        tool_schema("search_web", "Search the web", json!({
            "session_id": {"type": "string"},
            "tab_id": {"type": "string"},
            "query": {"type": "string"},
            "engine": {"type": "string", "enum": ["google", "bing", "duckduckgo"]},
            "num_results": {"type": "integer"}
        }), vec!["session_id", "query"]),
        tool_schema("set_default_search_engine", "Set default search engine", json!({
            "engine": {"type": "string", "enum": ["google", "bing", "duckduckgo"]},
            "session_id": {"type": "string"}
        }), vec!["engine"]),
        tool_schema("get_default_search_engine", "Get default search engine", json!({
            "session_id": {"type": "string"}
        }), vec![]),
    ])
}

/// Run the MCP stdio server.
pub async fn run_stdio_server(
    session_manager: Arc<SessionManager>,
    search_registry: Arc<SearchRegistry>,
    downloads: DownloadRegistry,
    allow_js: bool,
) -> anyhow::Result<()> {
    tracing::info!("Starting MCP stdio server");
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin);
    let mut stdout = tokio::io::BufWriter::new(stdout);
    let mut line = String::new();

    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            break; // EOF
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let response = match serde_json::from_str::<JsonRpcRequest>(trimmed) {
            Ok(req) => {
                handle_request(
                    req,
                    &session_manager,
                    &search_registry,
                    &downloads,
                    allow_js,
                ).await
            }
            Err(e) => {
                JsonRpcResponse::err(None, -32700, &format!("Parse error: {}", e))
            }
        };

        let mut response_str = serde_json::to_string(&response)?;
        response_str.push('\n');
        stdout.write_all(response_str.as_bytes()).await?;
        stdout.flush().await?;
    }

    Ok(())
}

async fn handle_request(
    req: JsonRpcRequest,
    session_manager: &Arc<SessionManager>,
    search_registry: &Arc<SearchRegistry>,
    downloads: &DownloadRegistry,
    allow_js: bool,
) -> JsonRpcResponse {
    let id = req.id.clone();

    match req.method.as_str() {
        "initialize" => {
            JsonRpcResponse::ok(id, json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "meleys",
                    "version": "0.1.0"
                }
            }))
        }
        "tools/list" => {
            JsonRpcResponse::ok(id, json!({ "tools": tools_list() }))
        }
        "tools/call" => {
            let params = req.params.unwrap_or(json!({}));
            let tool_name = params["name"].as_str().unwrap_or("").to_string();
            let args = params["arguments"].clone();

            let obs = dispatch_tool(
                &tool_name,
                args,
                session_manager,
                search_registry,
                downloads,
                allow_js,
            ).await;

            let result_val = serde_json::to_value(&obs).unwrap_or(json!({}));
            JsonRpcResponse::ok(id, json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&result_val).unwrap_or_default()
                }],
                "isError": !obs.success
            }))
        }
        "notifications/initialized" => {
            // Just acknowledge
            JsonRpcResponse::ok(id, json!({}))
        }
        _ => {
            JsonRpcResponse::err(id, -32601, &format!("Method not found: {}", req.method))
        }
    }
}

async fn dispatch_tool(
    name: &str,
    args: Value,
    session_manager: &Arc<SessionManager>,
    search_registry: &Arc<SearchRegistry>,
    downloads: &DownloadRegistry,
    allow_js: bool,
) -> Observation {
    let a = &args;

    let session_id = a["session_id"].as_str().unwrap_or("");
    let tab_id = a["tab_id"].as_str();

    match name {
        "create_session" => {
            crate::actions::session::create_session(
                session_manager,
                a["profile_name"].as_str().map(|s| s.to_string()),
                a["headless"].as_bool(),
                a["default_search_engine"].as_str().map(|s| s.to_string()),
            ).await
        }
        "close_session" => {
            crate::actions::session::close_session(session_manager, session_id).await
        }
        "list_sessions" => {
            crate::actions::session::list_sessions(session_manager).await
        }
        "new_tab" => {
            crate::actions::tabs::new_tab(session_manager, session_id, a["url"].as_str()).await
        }
        "close_tab" => {
            let tab = a["tab_id"].as_str().unwrap_or("");
            crate::actions::tabs::close_tab(session_manager, session_id, tab).await
        }
        "list_tabs" => {
            crate::actions::tabs::list_tabs(session_manager, session_id).await
        }
        "switch_tab" => {
            let tab = a["tab_id"].as_str().unwrap_or("");
            crate::actions::tabs::switch_tab(session_manager, session_id, tab).await
        }
        "navigate" => {
            let url = a["url"].as_str().unwrap_or("");
            crate::actions::navigation::navigate(
                session_manager, session_id, tab_id,
                url,
                a["wait_until"].as_str(),
                a["timeout_ms"].as_u64(),
            ).await
        }
        "go_back" => {
            crate::actions::navigation::go_back(
                session_manager, session_id, tab_id, a["timeout_ms"].as_u64(),
            ).await
        }
        "go_forward" => {
            crate::actions::navigation::go_forward(
                session_manager, session_id, tab_id, a["timeout_ms"].as_u64(),
            ).await
        }
        "reload" => {
            crate::actions::navigation::reload(
                session_manager, session_id, tab_id,
                a["ignore_cache"].as_bool().unwrap_or(false),
                a["timeout_ms"].as_u64(),
            ).await
        }
        "wait_for" => {
            crate::actions::navigation::wait_for(
                session_manager, session_id, tab_id,
                a["condition"].as_str().unwrap_or("timeout"),
                a["selector"].as_str(),
                a["state"].as_str(),
                a["timeout_ms"].as_u64(),
                a["idle_ms"].as_u64(),
                a["js_expr"].as_str(),
                a["poll_ms"].as_u64(),
                a["sleep_ms"].as_u64(),
            ).await
        }
        "click" => {
            let sel = parse_selector(&a["selector"]);
            crate::actions::interaction::click(
                session_manager, session_id, tab_id,
                &sel,
                a["button"].as_str(),
                a["click_count"].as_u64().map(|n| n as u32),
                a["nth"].as_u64().map(|n| n as usize),
                a["timeout_ms"].as_u64(),
            ).await
        }
        "type_text" => {
            let sel = parse_selector(&a["selector"]);
            crate::actions::interaction::type_text(
                session_manager, session_id, tab_id,
                &sel,
                a["text"].as_str().unwrap_or(""),
                a["clear_first"].as_bool().unwrap_or(false),
                a["delay_ms"].as_u64(),
                a["timeout_ms"].as_u64(),
            ).await
        }
        "press_key" => {
            let sel = if a["selector"].is_null() { None } else { Some(parse_selector(&a["selector"])) };
            crate::actions::interaction::press_key(
                session_manager, session_id, tab_id,
                a["key"].as_str().unwrap_or("Enter"),
                sel.as_ref(),
                a["timeout_ms"].as_u64(),
            ).await
        }
        "hover" => {
            let sel = parse_selector(&a["selector"]);
            crate::actions::interaction::hover(
                session_manager, session_id, tab_id, &sel, a["timeout_ms"].as_u64(),
            ).await
        }
        "scroll" => {
            let sel = if a["selector"].is_null() { None } else { Some(parse_selector(&a["selector"])) };
            crate::actions::interaction::scroll(
                session_manager, session_id, tab_id,
                a["direction"].as_str(),
                a["amount_px"].as_i64(),
                sel.as_ref(),
                a["to_bottom"].as_bool().unwrap_or(false),
                a["timeout_ms"].as_u64(),
            ).await
        }
        "select_option" => {
            let sel = parse_selector(&a["selector"]);
            crate::actions::interaction::select_option(
                session_manager, session_id, tab_id,
                &sel,
                a["value"].as_str().unwrap_or(""),
                a["timeout_ms"].as_u64(),
            ).await
        }
        "set_file_input" => {
            let sel = parse_selector(&a["selector"]);
            let paths = a["file_paths"].as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();
            crate::actions::interaction::set_file_input(
                session_manager, session_id, tab_id, &sel, paths, a["timeout_ms"].as_u64(),
            ).await
        }
        "get_text" => {
            let sel = if a["selector"].is_null() { None } else { Some(parse_selector(&a["selector"])) };
            crate::actions::extraction::get_text(
                session_manager, session_id, tab_id,
                sel.as_ref(),
                a["max_chars"].as_u64().map(|n| n as usize),
            ).await
        }
        "get_links" => {
            crate::actions::extraction::get_links(
                session_manager, session_id, tab_id,
                a["selector"].as_str(),
                a["same_origin_only"].as_bool().unwrap_or(false),
            ).await
        }
        "get_dom" => {
            let sel = if a["selector"].is_null() { None } else { Some(parse_selector(&a["selector"])) };
            crate::actions::extraction::get_dom(
                session_manager, session_id, tab_id,
                sel.as_ref(),
                a["max_depth"].as_u64().map(|n| n as u32),
                a["include_hidden"].as_bool(),
                a["max_nodes"].as_u64().map(|n| n as usize),
            ).await
        }
        "get_ax_tree" => {
            let sel = if a["selector"].is_null() { None } else { Some(parse_selector(&a["selector"])) };
            crate::actions::extraction::get_ax_tree(
                session_manager, session_id, tab_id,
                sel.as_ref(),
                a["max_depth"].as_u64().map(|n| n as u32),
            ).await
        }
        "query_elements" => {
            let sel = parse_selector(&a["selector"]);
            crate::actions::extraction::query_elements(
                session_manager, session_id, tab_id,
                &sel,
                a["limit"].as_u64().map(|n| n as usize),
            ).await
        }
        "evaluate_js" => {
            crate::actions::extraction::evaluate_js(
                session_manager, session_id, tab_id,
                a["expression"].as_str().unwrap_or(""),
                allow_js,
            ).await
        }
        "screenshot" => {
            let sel = if a["selector"].is_null() { None } else { Some(parse_selector(&a["selector"])) };
            crate::actions::capture::screenshot(
                session_manager, session_id, tab_id,
                sel.as_ref(),
                a["full_page"].as_bool().unwrap_or(false),
                a["format"].as_str(),
                a["timeout_ms"].as_u64(),
            ).await
        }
        "export_pdf" => {
            crate::actions::capture::export_pdf(
                session_manager, session_id, tab_id,
                a["landscape"].as_bool().unwrap_or(false),
                a["timeout_ms"].as_u64(),
            ).await
        }
        "download_file" => {
            crate::actions::download::download_file(
                session_manager, session_id, tab_id,
                a["url"].as_str().unwrap_or(""),
                a["save_as"].as_str(),
                downloads,
                a["timeout_ms"].as_u64(),
            ).await
        }
        "list_downloads" => {
            crate::actions::download::list_downloads(session_id, tab_id.unwrap_or(""), downloads).await
        }
        "get_cookies" => {
            let urls = a["urls"].as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect());
            crate::actions::cookies::get_cookies(session_manager, session_id, tab_id, urls).await
        }
        "set_cookies" => {
            let cookies: Vec<CookieInfo> = serde_json::from_value(a["cookies"].clone())
                .unwrap_or_default();
            crate::actions::cookies::set_cookies(session_manager, session_id, tab_id, cookies).await
        }
        "clear_cookies" => {
            crate::actions::cookies::clear_cookies(session_manager, session_id, tab_id).await
        }
        "get_local_storage" => {
            crate::actions::cookies::get_local_storage(
                session_manager, session_id, tab_id, a["origin"].as_str(),
            ).await
        }
        "search_web" => {
            crate::actions::search::search_web(
                session_manager, session_id, tab_id,
                a["query"].as_str().unwrap_or(""),
                a["engine"].as_str(),
                a["num_results"].as_u64().map(|n| n as usize),
                search_registry,
                a["timeout_ms"].as_u64(),
            ).await
        }
        "set_default_search_engine" => {
            crate::actions::search::set_default_search_engine(
                session_manager,
                if session_id.is_empty() { None } else { Some(session_id) },
                a["engine"].as_str().unwrap_or("duckduckgo"),
                search_registry,
            ).await
        }
        "get_default_search_engine" => {
            crate::actions::search::get_default_search_engine(
                session_manager,
                if session_id.is_empty() { None } else { Some(session_id) },
                search_registry,
            ).await
        }
        unknown => {
            Observation::failure("", "", unknown, "INTERNAL_ERROR",
                format!("Unknown tool: {}", unknown), false)
        }
    }
}

fn parse_selector(v: &Value) -> Selector {
    if let Some(s) = v.as_str() {
        return Selector::Css(s.to_string());
    }
    if let Some(obj) = v.as_object() {
        if let Some(t) = obj.get("type").and_then(|t| t.as_str()) {
            match t {
                "Css" => {
                    let val = obj.get("value").and_then(|v| v.as_str()).unwrap_or("body");
                    return Selector::Css(val.to_string());
                }
                "XPath" => {
                    let val = obj.get("value").and_then(|v| v.as_str()).unwrap_or("");
                    return Selector::XPath(val.to_string());
                }
                "Text" => {
                    if let Some(vobj) = obj.get("value").and_then(|v| v.as_object()) {
                        return Selector::Text {
                            exact: vobj.get("exact").and_then(|e| e.as_bool()).unwrap_or(false),
                            value: vobj.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        };
                    }
                }
                "Coordinates" => {
                    if let Some(vobj) = obj.get("value").and_then(|v| v.as_object()) {
                        return Selector::Coordinates {
                            x: vobj.get("x").and_then(|x| x.as_f64()).unwrap_or(0.0),
                            y: vobj.get("y").and_then(|y| y.as_f64()).unwrap_or(0.0),
                        };
                    }
                }
                "BackendNodeId" => {
                    if let Some(id) = obj.get("value").and_then(|v| v.as_i64()) {
                        return Selector::BackendNodeId(id);
                    }
                }
                _ => {}
            }
        }
        // Try direct css field
        if let Some(css) = obj.get("css").and_then(|c| c.as_str()) {
            return Selector::Css(css.to_string());
        }
    }
    Selector::Css("body".to_string())
}

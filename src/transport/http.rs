use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tower_http::trace::TraceLayer;

use crate::actions::download::DownloadRegistry;
use crate::actions::search::SearchRegistry;
use crate::observation::{CookieInfo, Observation};
use crate::selector::Selector;
use crate::session::SessionManager;

/// Shared application state for HTTP handlers.
#[derive(Clone)]
pub struct AppState {
    pub session_manager: Arc<SessionManager>,
    pub search_registry: Arc<SearchRegistry>,
    pub downloads: DownloadRegistry,
    pub allow_js: bool,
}

/// Build the axum router with all routes.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        // Sessions
        .route("/v1/sessions", get(list_sessions_handler).post(create_session_handler))
        .route("/v1/sessions/{session_id}", post(close_session_handler_post))
        .route("/v1/sessions/{session_id}/close", post(close_session_handler))
        .route("/v1/sessions/{session_id}/tabs", get(list_tabs_handler).post(new_tab_handler))
        .route("/v1/sessions/{session_id}/tabs/{tab_id}/close", post(close_tab_handler))
        .route("/v1/sessions/{session_id}/tabs/{tab_id}/switch", post(switch_tab_handler))
        // Navigation
        .route("/v1/sessions/{session_id}/tabs/{tab_id}/navigate", post(navigate_handler))
        .route("/v1/sessions/{session_id}/tabs/{tab_id}/go_back", post(go_back_handler))
        .route("/v1/sessions/{session_id}/tabs/{tab_id}/go_forward", post(go_forward_handler))
        .route("/v1/sessions/{session_id}/tabs/{tab_id}/reload", post(reload_handler))
        .route("/v1/sessions/{session_id}/tabs/{tab_id}/wait_for", post(wait_for_handler))
        // Interaction
        .route("/v1/sessions/{session_id}/tabs/{tab_id}/click", post(click_handler))
        .route("/v1/sessions/{session_id}/tabs/{tab_id}/type_text", post(type_text_handler))
        .route("/v1/sessions/{session_id}/tabs/{tab_id}/press_key", post(press_key_handler))
        .route("/v1/sessions/{session_id}/tabs/{tab_id}/hover", post(hover_handler))
        .route("/v1/sessions/{session_id}/tabs/{tab_id}/scroll", post(scroll_handler))
        .route("/v1/sessions/{session_id}/tabs/{tab_id}/select_option", post(select_option_handler))
        .route("/v1/sessions/{session_id}/tabs/{tab_id}/set_file_input", post(set_file_input_handler))
        // Extraction
        .route("/v1/sessions/{session_id}/tabs/{tab_id}/get_text", post(get_text_handler))
        .route("/v1/sessions/{session_id}/tabs/{tab_id}/get_links", post(get_links_handler))
        .route("/v1/sessions/{session_id}/tabs/{tab_id}/get_dom", post(get_dom_handler))
        .route("/v1/sessions/{session_id}/tabs/{tab_id}/get_ax_tree", post(get_ax_tree_handler))
        .route("/v1/sessions/{session_id}/tabs/{tab_id}/query_elements", post(query_elements_handler))
        .route("/v1/sessions/{session_id}/tabs/{tab_id}/evaluate_js", post(evaluate_js_handler))
        // Capture
        .route("/v1/sessions/{session_id}/tabs/{tab_id}/screenshot", post(screenshot_handler))
        .route("/v1/sessions/{session_id}/tabs/{tab_id}/export_pdf", post(export_pdf_handler))
        // Downloads
        .route("/v1/sessions/{session_id}/downloads", get(list_downloads_handler))
        .route("/v1/sessions/{session_id}/tabs/{tab_id}/download_file", post(download_file_handler))
        // Cookies
        .route("/v1/sessions/{session_id}/tabs/{tab_id}/cookies", get(get_cookies_handler).post(set_cookies_handler).delete(clear_cookies_handler))
        .route("/v1/sessions/{session_id}/tabs/{tab_id}/local_storage", get(get_local_storage_handler))
        // Search
        .route("/v1/sessions/{session_id}/tabs/{tab_id}/search_web", post(search_web_handler))
        .route("/v1/search_engine", get(get_search_engine_handler).post(set_search_engine_handler))
        .route("/v1/sessions/{session_id}/search_engine", get(get_session_search_engine_handler).post(set_session_search_engine_handler))
        // Health
        .route("/v1/health", get(health_handler))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

// ======================== Handler Types ========================

#[derive(Debug, Deserialize)]
struct CreateSessionRequest {
    profile_name: Option<String>,
    headless: Option<bool>,
    default_search_engine: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NewTabRequest {
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NavigateRequest {
    url: String,
    wait_until: Option<String>,
    timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct GoBackRequest {
    timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct GoForwardRequest {
    timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct ReloadRequest {
    ignore_cache: Option<bool>,
    timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct WaitForRequest {
    condition: String,
    selector: Option<String>,
    state: Option<String>,
    timeout_ms: Option<u64>,
    idle_ms: Option<u64>,
    js_expr: Option<String>,
    poll_ms: Option<u64>,
    sleep_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct ClickRequest {
    selector: Selector,
    button: Option<String>,
    click_count: Option<u32>,
    nth: Option<usize>,
    timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct TypeTextRequest {
    selector: Selector,
    text: String,
    clear_first: Option<bool>,
    delay_ms: Option<u64>,
    timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct PressKeyRequest {
    key: String,
    selector: Option<Selector>,
    timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct HoverRequest {
    selector: Selector,
    timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct ScrollRequest {
    direction: Option<String>,
    amount_px: Option<i64>,
    selector: Option<Selector>,
    to_bottom: Option<bool>,
    timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct SelectOptionRequest {
    selector: Selector,
    value: String,
    timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct SetFileInputRequest {
    selector: Selector,
    file_paths: Vec<String>,
    timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct GetTextRequest {
    selector: Option<Selector>,
    max_chars: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct GetLinksRequest {
    selector: Option<String>,
    same_origin_only: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct GetDomRequest {
    selector: Option<Selector>,
    max_depth: Option<u32>,
    include_hidden: Option<bool>,
    max_nodes: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct GetAxTreeRequest {
    selector: Option<Selector>,
    max_depth: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct QueryElementsRequest {
    selector: Selector,
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct EvaluateJsRequest {
    expression: String,
}

#[derive(Debug, Deserialize)]
struct ScreenshotRequest {
    selector: Option<Selector>,
    full_page: Option<bool>,
    format: Option<String>,
    timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct ExportPdfRequest {
    landscape: Option<bool>,
    timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct DownloadFileRequest {
    url: String,
    save_as: Option<String>,
    timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct GetCookiesRequest {
    urls: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct SetCookiesRequest {
    cookies: Vec<CookieInfo>,
}

#[derive(Debug, Deserialize)]
struct GetLocalStorageRequest {
    origin: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SearchWebRequest {
    query: String,
    engine: Option<String>,
    num_results: Option<usize>,
    timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct SetSearchEngineRequest {
    engine: String,
}

// ======================== Handlers ========================

async fn health_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "ok"}))
}

async fn list_sessions_handler(State(state): State<AppState>) -> Json<Observation> {
    let obs = crate::actions::session::list_sessions(&state.session_manager).await;
    Json(obs)
}

async fn create_session_handler(
    State(state): State<AppState>,
    Json(req): Json<CreateSessionRequest>,
) -> Json<Observation> {
    let obs = crate::actions::session::create_session(
        &state.session_manager,
        req.profile_name,
        req.headless,
        req.default_search_engine,
    ).await;
    Json(obs)
}

async fn close_session_handler(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Json<Observation> {
    let obs = crate::actions::session::close_session(&state.session_manager, &session_id).await;
    Json(obs)
}

async fn close_session_handler_post(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Json<Observation> {
    let obs = crate::actions::session::close_session(&state.session_manager, &session_id).await;
    Json(obs)
}

async fn list_tabs_handler(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Json<Observation> {
    let obs = crate::actions::tabs::list_tabs(&state.session_manager, &session_id).await;
    Json(obs)
}

async fn new_tab_handler(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Json(req): Json<NewTabRequest>,
) -> Json<Observation> {
    let obs = crate::actions::tabs::new_tab(&state.session_manager, &session_id, req.url.as_deref()).await;
    Json(obs)
}

async fn close_tab_handler(
    State(state): State<AppState>,
    Path((session_id, tab_id)): Path<(String, String)>,
) -> Json<Observation> {
    let obs = crate::actions::tabs::close_tab(&state.session_manager, &session_id, &tab_id).await;
    Json(obs)
}

async fn switch_tab_handler(
    State(state): State<AppState>,
    Path((session_id, tab_id)): Path<(String, String)>,
) -> Json<Observation> {
    let obs = crate::actions::tabs::switch_tab(&state.session_manager, &session_id, &tab_id).await;
    Json(obs)
}

async fn navigate_handler(
    State(state): State<AppState>,
    Path((session_id, tab_id)): Path<(String, String)>,
    Json(req): Json<NavigateRequest>,
) -> Json<Observation> {
    let obs = crate::actions::navigation::navigate(
        &state.session_manager,
        &session_id,
        Some(&tab_id),
        &req.url,
        req.wait_until.as_deref(),
        req.timeout_ms,
    ).await;
    Json(obs)
}

async fn go_back_handler(
    State(state): State<AppState>,
    Path((session_id, tab_id)): Path<(String, String)>,
    body: Option<Json<GoBackRequest>>,
) -> Json<Observation> {
    let timeout_ms = body.and_then(|b| b.timeout_ms);
    let obs = crate::actions::navigation::go_back(
        &state.session_manager, &session_id, Some(&tab_id), timeout_ms,
    ).await;
    Json(obs)
}

async fn go_forward_handler(
    State(state): State<AppState>,
    Path((session_id, tab_id)): Path<(String, String)>,
    body: Option<Json<GoForwardRequest>>,
) -> Json<Observation> {
    let timeout_ms = body.and_then(|b| b.timeout_ms);
    let obs = crate::actions::navigation::go_forward(
        &state.session_manager, &session_id, Some(&tab_id), timeout_ms,
    ).await;
    Json(obs)
}

async fn reload_handler(
    State(state): State<AppState>,
    Path((session_id, tab_id)): Path<(String, String)>,
    body: Option<Json<ReloadRequest>>,
) -> Json<Observation> {
    let (ignore_cache, timeout_ms) = body
        .map(|b| (b.ignore_cache.unwrap_or(false), b.timeout_ms))
        .unwrap_or((false, None));
    let obs = crate::actions::navigation::reload(
        &state.session_manager, &session_id, Some(&tab_id), ignore_cache, timeout_ms,
    ).await;
    Json(obs)
}

async fn wait_for_handler(
    State(state): State<AppState>,
    Path((session_id, tab_id)): Path<(String, String)>,
    Json(req): Json<WaitForRequest>,
) -> Json<Observation> {
    let obs = crate::actions::navigation::wait_for(
        &state.session_manager,
        &session_id,
        Some(&tab_id),
        &req.condition,
        req.selector.as_deref(),
        req.state.as_deref(),
        req.timeout_ms,
        req.idle_ms,
        req.js_expr.as_deref(),
        req.poll_ms,
        req.sleep_ms,
    ).await;
    Json(obs)
}

async fn click_handler(
    State(state): State<AppState>,
    Path((session_id, tab_id)): Path<(String, String)>,
    Json(req): Json<ClickRequest>,
) -> Json<Observation> {
    let obs = crate::actions::interaction::click(
        &state.session_manager,
        &session_id,
        Some(&tab_id),
        &req.selector,
        req.button.as_deref(),
        req.click_count,
        req.nth,
        req.timeout_ms,
    ).await;
    Json(obs)
}

async fn type_text_handler(
    State(state): State<AppState>,
    Path((session_id, tab_id)): Path<(String, String)>,
    Json(req): Json<TypeTextRequest>,
) -> Json<Observation> {
    let obs = crate::actions::interaction::type_text(
        &state.session_manager,
        &session_id,
        Some(&tab_id),
        &req.selector,
        &req.text,
        req.clear_first.unwrap_or(false),
        req.delay_ms,
        req.timeout_ms,
    ).await;
    Json(obs)
}

async fn press_key_handler(
    State(state): State<AppState>,
    Path((session_id, tab_id)): Path<(String, String)>,
    Json(req): Json<PressKeyRequest>,
) -> Json<Observation> {
    let obs = crate::actions::interaction::press_key(
        &state.session_manager,
        &session_id,
        Some(&tab_id),
        &req.key,
        req.selector.as_ref(),
        req.timeout_ms,
    ).await;
    Json(obs)
}

async fn hover_handler(
    State(state): State<AppState>,
    Path((session_id, tab_id)): Path<(String, String)>,
    Json(req): Json<HoverRequest>,
) -> Json<Observation> {
    let obs = crate::actions::interaction::hover(
        &state.session_manager,
        &session_id,
        Some(&tab_id),
        &req.selector,
        req.timeout_ms,
    ).await;
    Json(obs)
}

async fn scroll_handler(
    State(state): State<AppState>,
    Path((session_id, tab_id)): Path<(String, String)>,
    Json(req): Json<ScrollRequest>,
) -> Json<Observation> {
    let obs = crate::actions::interaction::scroll(
        &state.session_manager,
        &session_id,
        Some(&tab_id),
        req.direction.as_deref(),
        req.amount_px,
        req.selector.as_ref(),
        req.to_bottom.unwrap_or(false),
        req.timeout_ms,
    ).await;
    Json(obs)
}

async fn select_option_handler(
    State(state): State<AppState>,
    Path((session_id, tab_id)): Path<(String, String)>,
    Json(req): Json<SelectOptionRequest>,
) -> Json<Observation> {
    let obs = crate::actions::interaction::select_option(
        &state.session_manager,
        &session_id,
        Some(&tab_id),
        &req.selector,
        &req.value,
        req.timeout_ms,
    ).await;
    Json(obs)
}

async fn set_file_input_handler(
    State(state): State<AppState>,
    Path((session_id, tab_id)): Path<(String, String)>,
    Json(req): Json<SetFileInputRequest>,
) -> Json<Observation> {
    let obs = crate::actions::interaction::set_file_input(
        &state.session_manager,
        &session_id,
        Some(&tab_id),
        &req.selector,
        req.file_paths,
        req.timeout_ms,
    ).await;
    Json(obs)
}

async fn get_text_handler(
    State(state): State<AppState>,
    Path((session_id, tab_id)): Path<(String, String)>,
    body: Option<Json<GetTextRequest>>,
) -> Json<Observation> {
    let (selector, max_chars) = body.map(|b| (b.0.selector, b.0.max_chars)).unwrap_or((None, None));
    let obs = crate::actions::extraction::get_text(
        &state.session_manager,
        &session_id,
        Some(&tab_id),
        selector.as_ref(),
        max_chars,
    ).await;
    Json(obs)
}

async fn get_links_handler(
    State(state): State<AppState>,
    Path((session_id, tab_id)): Path<(String, String)>,
    body: Option<Json<GetLinksRequest>>,
) -> Json<Observation> {
    let (sel, same_origin) = body
        .map(|b| (b.0.selector, b.0.same_origin_only.unwrap_or(false)))
        .unwrap_or((None, false));
    let obs = crate::actions::extraction::get_links(
        &state.session_manager,
        &session_id,
        Some(&tab_id),
        sel.as_deref(),
        same_origin,
    ).await;
    Json(obs)
}

async fn get_dom_handler(
    State(state): State<AppState>,
    Path((session_id, tab_id)): Path<(String, String)>,
    body: Option<Json<GetDomRequest>>,
) -> Json<Observation> {
    let req = body.map(|b| b.0).unwrap_or(GetDomRequest {
        selector: None, max_depth: None, include_hidden: None, max_nodes: None,
    });
    let obs = crate::actions::extraction::get_dom(
        &state.session_manager,
        &session_id,
        Some(&tab_id),
        req.selector.as_ref(),
        req.max_depth,
        req.include_hidden,
        req.max_nodes,
    ).await;
    Json(obs)
}

async fn get_ax_tree_handler(
    State(state): State<AppState>,
    Path((session_id, tab_id)): Path<(String, String)>,
    body: Option<Json<GetAxTreeRequest>>,
) -> Json<Observation> {
    let req = body.map(|b| b.0).unwrap_or(GetAxTreeRequest { selector: None, max_depth: None });
    let obs = crate::actions::extraction::get_ax_tree(
        &state.session_manager,
        &session_id,
        Some(&tab_id),
        req.selector.as_ref(),
        req.max_depth,
    ).await;
    Json(obs)
}

async fn query_elements_handler(
    State(state): State<AppState>,
    Path((session_id, tab_id)): Path<(String, String)>,
    Json(req): Json<QueryElementsRequest>,
) -> Json<Observation> {
    let obs = crate::actions::extraction::query_elements(
        &state.session_manager,
        &session_id,
        Some(&tab_id),
        &req.selector,
        req.limit,
    ).await;
    Json(obs)
}

async fn evaluate_js_handler(
    State(state): State<AppState>,
    Path((session_id, tab_id)): Path<(String, String)>,
    Json(req): Json<EvaluateJsRequest>,
) -> Json<Observation> {
    let obs = crate::actions::extraction::evaluate_js(
        &state.session_manager,
        &session_id,
        Some(&tab_id),
        &req.expression,
        state.allow_js,
    ).await;
    Json(obs)
}

async fn screenshot_handler(
    State(state): State<AppState>,
    Path((session_id, tab_id)): Path<(String, String)>,
    body: Option<Json<ScreenshotRequest>>,
) -> Json<Observation> {
    let req = body.map(|b| b.0).unwrap_or(ScreenshotRequest {
        selector: None, full_page: None, format: None, timeout_ms: None,
    });
    let obs = crate::actions::capture::screenshot(
        &state.session_manager,
        &session_id,
        Some(&tab_id),
        req.selector.as_ref(),
        req.full_page.unwrap_or(false),
        req.format.as_deref(),
        req.timeout_ms,
    ).await;
    Json(obs)
}

async fn export_pdf_handler(
    State(state): State<AppState>,
    Path((session_id, tab_id)): Path<(String, String)>,
    body: Option<Json<ExportPdfRequest>>,
) -> Json<Observation> {
    let (landscape, timeout_ms) = body
        .map(|b| (b.0.landscape.unwrap_or(false), b.0.timeout_ms))
        .unwrap_or((false, None));
    let obs = crate::actions::capture::export_pdf(
        &state.session_manager,
        &session_id,
        Some(&tab_id),
        landscape,
        timeout_ms,
    ).await;
    Json(obs)
}

async fn list_downloads_handler(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Json<Observation> {
    let obs = crate::actions::download::list_downloads(&session_id, "", &state.downloads).await;
    Json(obs)
}

async fn download_file_handler(
    State(state): State<AppState>,
    Path((session_id, tab_id)): Path<(String, String)>,
    Json(req): Json<DownloadFileRequest>,
) -> Json<Observation> {
    let obs = crate::actions::download::download_file(
        &state.session_manager,
        &session_id,
        Some(&tab_id),
        &req.url,
        req.save_as.as_deref(),
        &state.downloads,
        req.timeout_ms,
    ).await;
    Json(obs)
}

async fn get_cookies_handler(
    State(state): State<AppState>,
    Path((session_id, tab_id)): Path<(String, String)>,
    body: Option<Json<GetCookiesRequest>>,
) -> Json<Observation> {
    let urls = body.and_then(|b| b.0.urls);
    let obs = crate::actions::cookies::get_cookies(
        &state.session_manager,
        &session_id,
        Some(&tab_id),
        urls,
    ).await;
    Json(obs)
}

async fn set_cookies_handler(
    State(state): State<AppState>,
    Path((session_id, tab_id)): Path<(String, String)>,
    Json(req): Json<SetCookiesRequest>,
) -> Json<Observation> {
    let obs = crate::actions::cookies::set_cookies(
        &state.session_manager,
        &session_id,
        Some(&tab_id),
        req.cookies,
    ).await;
    Json(obs)
}

async fn clear_cookies_handler(
    State(state): State<AppState>,
    Path((session_id, tab_id)): Path<(String, String)>,
) -> Json<Observation> {
    let obs = crate::actions::cookies::clear_cookies(
        &state.session_manager,
        &session_id,
        Some(&tab_id),
    ).await;
    Json(obs)
}

async fn get_local_storage_handler(
    State(state): State<AppState>,
    Path((session_id, tab_id)): Path<(String, String)>,
    body: Option<Json<GetLocalStorageRequest>>,
) -> Json<Observation> {
    let origin = body.and_then(|b| b.0.origin);
    let obs = crate::actions::cookies::get_local_storage(
        &state.session_manager,
        &session_id,
        Some(&tab_id),
        origin.as_deref(),
    ).await;
    Json(obs)
}

async fn search_web_handler(
    State(state): State<AppState>,
    Path((session_id, tab_id)): Path<(String, String)>,
    Json(req): Json<SearchWebRequest>,
) -> Json<Observation> {
    let obs = crate::actions::search::search_web(
        &state.session_manager,
        &session_id,
        Some(&tab_id),
        &req.query,
        req.engine.as_deref(),
        req.num_results,
        &state.search_registry,
        req.timeout_ms,
    ).await;
    Json(obs)
}

async fn get_search_engine_handler(State(state): State<AppState>) -> Json<Observation> {
    let obs = crate::actions::search::get_default_search_engine(
        &state.session_manager, None, &state.search_registry,
    ).await;
    Json(obs)
}

async fn set_search_engine_handler(
    State(state): State<AppState>,
    Json(req): Json<SetSearchEngineRequest>,
) -> Json<Observation> {
    let obs = crate::actions::search::set_default_search_engine(
        &state.session_manager, None, &req.engine, &state.search_registry,
    ).await;
    Json(obs)
}

async fn get_session_search_engine_handler(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Json<Observation> {
    let obs = crate::actions::search::get_default_search_engine(
        &state.session_manager, Some(&session_id), &state.search_registry,
    ).await;
    Json(obs)
}

async fn set_session_search_engine_handler(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Json(req): Json<SetSearchEngineRequest>,
) -> Json<Observation> {
    let obs = crate::actions::search::set_default_search_engine(
        &state.session_manager, Some(&session_id), &req.engine, &state.search_registry,
    ).await;
    Json(obs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::observation::ActionResult;
    use crate::session::SessionManager;

    fn test_state() -> AppState {
        let config = Config::default();
        AppState {
            session_manager: Arc::new(SessionManager::new(config.clone())),
            search_registry: Arc::new(SearchRegistry::new(&config.search.default_engine)),
            downloads: Arc::new(Mutex::new(Default::default())),
            allow_js: false,
        }
    }

    #[test]
    fn test_build_router_creates_routes() {
        let state = test_state();
        let router = build_router(state);
        // Verify the router was created successfully - if build_router
        // panics or has invalid routes, this test will fail.
        // We can't easily inspect axum routes, but creation succeeding
        // means all route registrations are valid.
        let _ = router;
    }

    #[tokio::test]
    async fn test_health_handler() {
        let response = health_handler().await;
        let json = response.0;
        assert_eq!(json["status"], "ok");
    }

    #[tokio::test]
    async fn test_list_sessions_empty() {
        let state = test_state();
        let Json(obs) = list_sessions_handler(State(state)).await;
        assert!(obs.success);
        match &obs.result {
            ActionResult::Sessions(sessions) => assert!(sessions.is_empty()),
            _ => panic!("Expected Sessions variant"),
        }
    }

    #[tokio::test]
    async fn test_close_nonexistent_session() {
        let state = test_state();
        let Json(obs) = close_session_handler(
            State(state),
            Path("nonexistent-id".into()),
        ).await;
        assert!(!obs.success);
        let err = obs.error.as_ref().unwrap();
        assert_eq!(err.code, "SESSION_NOT_FOUND");
    }

    #[tokio::test]
    async fn test_list_tabs_nonexistent_session() {
        let state = test_state();
        let Json(obs) = list_tabs_handler(
            State(state),
            Path("nonexistent-id".into()),
        ).await;
        assert!(!obs.success);
    }

    #[tokio::test]
    async fn test_new_tab_nonexistent_session() {
        let state = test_state();
        let Json(obs) = new_tab_handler(
            State(state),
            Path("nonexistent-id".into()),
            Json(NewTabRequest { url: None }),
        ).await;
        assert!(!obs.success);
    }

    #[tokio::test]
    async fn test_navigate_nonexistent_session() {
        let state = test_state();
        let Json(obs) = navigate_handler(
            State(state),
            Path(("nonexistent".into(), "tab1".into())),
            Json(NavigateRequest {
                url: "https://example.com".into(),
                wait_until: None,
                timeout_ms: None,
            }),
        ).await;
        assert!(!obs.success);
    }

    #[tokio::test]
    async fn test_click_nonexistent_session() {
        let state = test_state();
        let Json(obs) = click_handler(
            State(state),
            Path(("nonexistent".into(), "tab1".into())),
            Json(ClickRequest {
                selector: Selector::Css("#btn".into()),
                button: None,
                click_count: None,
                nth: None,
                timeout_ms: None,
            }),
        ).await;
        assert!(!obs.success);
    }

    #[tokio::test]
    async fn test_get_text_nonexistent_session() {
        let state = test_state();
        let Json(obs) = get_text_handler(
            State(state),
            Path(("nonexistent".into(), "tab1".into())),
            Some(Json(GetTextRequest {
                selector: None,
                max_chars: None,
            })),
        ).await;
        assert!(!obs.success);
    }

    #[tokio::test]
    async fn test_search_web_nonexistent_session() {
        let state = test_state();
        let Json(obs) = search_web_handler(
            State(state),
            Path(("nonexistent".into(), "tab1".into())),
            Json(SearchWebRequest {
                query: "test".into(),
                engine: None,
                num_results: None,
                timeout_ms: None,
            }),
        ).await;
        assert!(!obs.success);
    }

    #[tokio::test]
    async fn test_get_search_engine_default() {
        let state = test_state();
        let Json(obs) = get_search_engine_handler(State(state)).await;
        assert!(obs.success);
        match &obs.result {
            ActionResult::SearchEngine { engine, scope } => {
                assert_eq!(engine, "duckduckgo");
                assert_eq!(scope, "runtime");
            }
            _ => panic!("Expected SearchEngine variant"),
        }
    }

    #[tokio::test]
    async fn test_set_search_engine() {
        let state = test_state();
        let Json(obs) = set_search_engine_handler(
            State(state.clone()),
            Json(SetSearchEngineRequest { engine: "google".into() }),
        ).await;
        assert!(obs.success);

        // Verify it changed
        let Json(obs) = get_search_engine_handler(State(state)).await;
        match &obs.result {
            ActionResult::SearchEngine { engine, .. } => assert_eq!(engine, "google"),
            _ => panic!("Expected SearchEngine"),
        }
    }

    #[tokio::test]
    async fn test_set_invalid_search_engine() {
        let state = test_state();
        let Json(obs) = set_search_engine_handler(
            State(state),
            Json(SetSearchEngineRequest { engine: "yahoo".into() }),
        ).await;
        assert!(!obs.success);
    }

    #[tokio::test]
    async fn test_evaluate_js_disabled() {
        let state = test_state();
        let Json(obs) = evaluate_js_handler(
            State(state),
            Path(("s1".into(), "t1".into())),
            Json(EvaluateJsRequest {
                expression: "1+1".into(),
            }),
        ).await;
        assert!(!obs.success);
        let err = obs.error.as_ref().unwrap();
        assert_eq!(err.code, "JS_EVAL_DISABLED");
    }

    #[tokio::test]
    async fn test_screenshot_nonexistent_session() {
        let state = test_state();
        let Json(obs) = screenshot_handler(
            State(state),
            Path(("nonexistent".into(), "tab1".into())),
            None,
        ).await;
        assert!(!obs.success);
    }

    #[tokio::test]
    async fn test_download_file_nonexistent_session() {
        let state = test_state();
        let Json(obs) = download_file_handler(
            State(state),
            Path(("nonexistent".into(), "tab1".into())),
            Json(DownloadFileRequest {
                url: "https://example.com/file.zip".into(),
                save_as: None,
                timeout_ms: None,
            }),
        ).await;
        assert!(!obs.success);
    }

    #[tokio::test]
    async fn test_list_downloads_empty() {
        let state = test_state();
        let Json(obs) = list_downloads_handler(
            State(state),
            Path("nonexistent".into()),
        ).await;
        assert!(obs.success);
    }
}

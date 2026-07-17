# Meleys Multi-Engine Architecture Design

## Overview

Meleys transitions from a Chromium-only backend (`chromiumoxide`) to a **multi-engine architecture** with **Lightpanda as the primary/default engine** and **Chromium as automatic fallback**. Screenshot and PDF export are removed — Meleys becomes a pure text/DOM/AX-tree extraction runtime, exactly Lightpanda's strength.

## Decisions Made

- **Fallback granularity**: Session-level (not navigation-level)
- **Fallback direction**: One-directional per session lifetime (once on Chromium, stay there)
- **Process model**: One shared Lightpanda process per runtime, per-session Chromium on fallback
- **Default engine**: Lightpanda (`default = "lightpanda"`)

## Architecture

### 1. Engine Abstraction Layer

New `src/engine/mod.rs` with `BrowserEngine` trait:

```rust
#[async_trait]
pub trait BrowserEngine: Send + Sync {
    fn kind(&self) -> EngineKind;
    async fn spawn(profile: &Profile, config: &EngineConfig) -> Result<Self, MeleyError>
    where Self: Sized;
    async fn new_page(&self) -> Result<Arc<Mutex<EnginePage>>, MeleyError>;
    async fn close(&self) -> Result<(), MeleyError>;
    fn capabilities(&self) -> EngineCapabilities;
}

pub enum EngineKind { Lightpanda, Chromium }

pub struct EngineCapabilities {
    pub screenshot: bool,
    pub pdf_export: bool,
    pub javascript: bool,
    pub cors: bool,
    pub network_interception: bool,
}
```

- `ChromiumEngine` wraps existing `cdp::launcher.rs` logic
- `LightpandaEngine` spawns `lightpanda serve --host 127.0.0.1 --port <cfg>` subprocess
- `EnginePage` wraps the underlying page handle — both engines speak CDP, so the action layer continues calling `page.evaluate()`, `page.goto()`, etc. through a unified interface
- `MeleyError` gains: `EngineStartupFailed`, `EngineCapabilityUnsupported`, `EngineFallbackExhausted`

```rust
pub enum EnginePreference {
    LightpandaOnly,
    ChromiumOnly,
    LightpandaWithFallback, // default
}

pub struct EngineTransition {
    pub from: EngineKind,
    pub to: EngineKind,
    pub reason: String,
    pub timestamp: String,
}
```

### 2. Fallback Layer

Session-level fallback from Lightpanda → Chromium:

**Triggers:**
1. Lightpanda binary missing or fails healthcheck within `startup_timeout_ms`
2. CDP errors matching Lightpanda gaps (unimplemented JS API, CORS-blocked, unsupported domain)
3. Explicit capability request (screenshot/PDF — removed from core but checked)

**Mechanics:**
- `SessionManager::create_session` takes `preferred_engine: EnginePreference` (default: `LightpandaWithFallback`)
- On failure, restart entire `BrowserSession` on Chromium — caller re-navigates
- `BrowserSession.engine_history: Vec<EngineTransition>` records transitions
- `Observation` gains `engine_used: EngineKind` and optional `engine_fallback`
- One-directional per session, `max_engine_fallbacks_per_session = 1`

### 3. Action Set Changes

**Removed:**
- `src/actions/capture.rs` — deleted entirely (`screenshot`, `export_pdf`)
- `ActionResult::Screenshot` removed from `observation.rs`
- HTTP routes `/screenshot` and `/export-pdf` removed
- MCP tools `screenshot` and `export_pdf` removed

**Unchanged** (Lightpanda covers these):
- `navigation.rs` — navigate, back/forward, reload, wait_for
- `interaction.rs` — click, type_text, press_key, hover, scroll, select_option, set_file_input
- `extraction.rs` — get_text, get_links, get_dom, get_ax_tree, query_elements, evaluate_js
- `cookies.rs` — full support
- `tabs.rs` / `session.rs` — action surface stays

**At risk** (needs runtime capability check):
- `search.rs` — relies on parsing rendered HTML from search engines; should work since these are server-rendered, but validate against Lightpanda's JS/DOM coverage
- `download.rs` — Lightpanda's download handling unclear; fallback to Chromium if unsupported
- `evaluate_js` — gated by `allow_evaluate_js` config flag; Lightpanda's V8-based JS should support this

### 4. Configuration

New `EngineConfig` in `src/config.rs`:

```toml
[engine]
default = "lightpanda"
lightpanda_binary_path = ""
lightpanda_port = 9223
startup_timeout_ms = 5000
fallback_enabled = true
max_engine_fallbacks_per_session = 1
obey_robots = true

[engine.chromium]
# existing BrowserConfig fields (executable_path, headless, viewport, profile_dir)
```

- `BrowserConfig` becomes `chromium: BrowserConfig` nested under `EngineConfig`
- Backward-compatible via migration shim (old env vars log warning, map to new paths)

### 5. Process Model

- **Lightpanda**: One shared process per Meleys runtime. Each `BrowserSession` → browser context, each `Tab` → page within context.
- **Chromium**: Keep existing per-session process model (`chromiumoxide`/CDP launcher).
- `SessionManager` tracks two pools: shared Lightpanda handle + per-session Chromium handles (only on fallback).

### 6. CDP/Launcher Layer

`src/cdp/launcher.rs` changes:
- Add Lightpanda binary discovery: config → PATH → common install dirs
- New `spawn_lightpanda(config) -> Child` running `lightpanda serve --host 127.0.0.1 --port <cfg> [--obey-robots]`
- Healthcheck: poll CDP `/json/version` until ready or `startup_timeout_ms`
- Set `LIGHTPANDA_DISABLE_TELEMETRY=true` when spawning

### 7. Observability

- `tracing` spans tagged with `engine = "lightpanda" | "chromium"` throughout action layer
- `network_summary` includes whether `robots.txt` was obeyed
- New error variants: `EngineStartupFailed`, `EngineCapabilityUnsupported`, `EngineFallbackExhausted`

### 8. CI/CD Pipeline Updates

- Add Lightpanda binary download step in CI (Linux only)
- Add integration tests for engine selection and fallback
- Update MSRV if needed for Lightpanda CDP client dependencies
- Verify release build starts with both engines

## File Change Summary

| File | Action |
|------|--------|
| `src/engine/mod.rs` | NEW — trait, EngineKind, EngineCapabilities |
| `src/engine/chromium.rs` | NEW — ChromiumEngine |
| `src/engine/lightpanda.rs` | NEW — LightpandaEngine |
| `src/engine/fallback.rs` | NEW — fallback orchestration |
| `src/error.rs` | MODIFY — add new error variants |
| `src/config.rs` | MODIFY — EngineConfig, migrate BrowserConfig |
| `src/observation.rs` | MODIFY — remove Screenshot, add engine fields |
| `src/session/mod.rs` | MODIFY — shared Lightpanda pool, EnginePreference |
| `src/session/browser_session.rs` | MODIFY — hold Arc<dyn BrowserEngine>, engine_history |
| `src/actions/capture.rs` | DELETE |
| `src/actions/mod.rs` | MODIFY — remove capture module |
| `src/transport/http.rs` | MODIFY — remove screenshot/PDF routes |
| `src/transport/mcp.rs` | MODIFY — remove screenshot/PDF tools |
| `src/cdp/launcher.rs` | MODIFY — Lightpanda binary discovery + spawn |
| `.github/workflows/ci.yml` | MODIFY — Lightpanda setup, engine tests |

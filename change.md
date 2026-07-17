# Meleys × Lightpanda — Feature Spec

## 1. Summary

Meleys moves from a Chromium-only backend (`chromiumoxide`) to a **multi-engine architecture** with **Lightpanda as the primary/default engine** and **Chromium as an automatic fallback**. Screenshot and PDF export are removed from the core action set — Meleys becomes a pure text/DOM/AX-tree extraction runtime, which is exactly Lightpanda's strength.

Phase 2 (longer-term, separate spec) is a native, non-Chromium, non-Lightpanda engine written in Rust — this document only sets up the abstraction boundary that phase 2 will slot into.

**Non-goals for this phase:** no visual capture, no bot-evasion/stealth, no Lightpanda source modification (we consume it as a subprocess over CDP, not as a linked library — it's Zig, not Rust).

---

## 2. Why this works: Lightpanda already speaks CDP

Lightpanda ships a `lightpanda serve --port 9222` mode exposing a CDP WebSocket endpoint, the same protocol `chromiumoxide` already targets in Meleys. This means:

- No new wire protocol to implement — `chromiumoxide` (or a thin custom CDP client) can talk to Lightpanda largely as-is.
- The swap is primarily in `src/cdp/launcher.rs` (which binary to spawn, which flags) and `src/session/` (what a "browser" is), not in `src/actions/*.rs`.
- CDP domain **coverage differs**: Lightpanda supports DOM, JS/V8, Ajax (XHR/Fetch), cookies, forms, click, custom headers, network interception. It does **not** implement `Page.captureScreenshot`, `Page.printToPDF`, or any painted/graphical CDP domains, since there's no rendering engine at all.

---

## 3. Engine Abstraction Layer

Introduce a `BrowserEngine` trait so `SessionManager`/`BrowserSession` stop assuming "Chromium via chromiumoxide" and instead hold a pluggable engine.

```rust
#[async_trait]
pub trait BrowserEngine: Send + Sync {
    fn kind(&self) -> EngineKind; // Lightpanda | Chromium | (future) Native

    async fn spawn(profile: &Profile, config: &BrowserConfig) -> Result<Self, MeleyError>
    where
        Self: Sized;

    async fn new_page(&self) -> Result<Arc<Mutex<EnginePage>>, MeleyError>;
    async fn close(&self) -> Result<(), MeleyError>;

    /// Declares which ActionResult variants this engine can fulfill.
    /// Used by the dispatcher to short-circuit unsupported actions
    /// instead of attempting and failing at the CDP layer.
    fn capabilities(&self) -> EngineCapabilities;
}

pub struct EngineCapabilities {
    pub screenshot: bool,
    pub pdf_export: bool,
    pub javascript: bool,
    pub cors: bool,
    pub network_interception: bool,
}
```

- `LightpandaEngine` and `ChromiumEngine` both implement this trait.
- `EnginePage` becomes a thin wrapper so `actions/*.rs` code keeps calling `page.click(selector)` etc. regardless of backend — action-layer code should require **near-zero changes**.
- `MeleyError` gains a new variant: `EngineCapabilityUnsupported { engine: EngineKind, action: &'static str }`.

---

## 4. Fallback Layer (Lightpanda → Chromium)

Since Lightpanda is Beta with partial Web API coverage and no CORS support yet, sessions need automatic, transparent fallback rather than hard failure.

### 4.1 Fallback triggers

A session (or a single navigation within a session, per your call below) falls back to Chromium when:

1. **Startup failure** — Lightpanda binary missing/fails healthcheck within `startup_timeout_ms`.
2. **Navigation-level failure classes** — CDP errors matching known Lightpanda gaps: unimplemented JS API, CORS-blocked resource fetch, unsupported CDP domain call, parser crash/panic signal from the subprocess.
3. **Explicit capability request** — action requires `screenshot`/`pdf_export` and engine is Lightpanda (see §5 — but this is dropped from core actions in this phase, so this trigger mainly matters if a future action reintroduces it).

### 4.2 Fallback granularity — needs a decision

Two possible scopes, both worth spelling out in the spec since they have different cost/complexity:

| Scope | Behavior | Cost |
|---|---|---|
| **Session-level** | Whole `BrowserSession` restarts on Chromium, replaying nothing — caller must re-navigate/re-authenticate | Simple, cheap, but loses in-page state |
| **Navigation-level** | Only the failing `navigate()` call retries against a per-session Chromium instance; session keeps a "shadow" Chromium browser ready | More complex (two processes per session), preserves surrounding session state better |

Recommendation: **start with session-level fallback** for v1 (simpler, matches Lightpanda's process model), and note navigation-level as a fast-follow if telemetry shows fallback is frequent mid-session.

### 4.3 Fallback mechanics

- `SessionManager::create_session` takes a `preferred_engine: EnginePreference` (`LightpandaOnly | ChromiumOnly | LightpandaWithFallback` — default).
- On fallback, `BrowserSession.engine_history: Vec<EngineTransition>` records `{ from, to, reason, timestamp }` for observability.
- `Observation` gains an `engine_used: EngineKind` field and, when a fallback occurred, `engine_fallback: Option<EngineTransition>` — callers/agents can see this happened without digging into logs.
- Fallback is **one-directional per session lifetime** by default (once on Chromium, stay there) to avoid flapping; a `LimitsConfig.max_engine_fallbacks_per_session` (default 1) guards this.

---

## 5. Action Set Changes

### 5.1 Removed
- `capture.rs` module removed entirely: `screenshot`, `export_pdf` actions dropped.
- `ActionResult::Screenshot` and `ActionResult::Download`-adjacent PDF variants removed from `observation.rs`.
- MCP tool catalog and HTTP routes drop `POST /v1/.../screenshot` and `/export-pdf`.

### 5.2 Unaffected (Lightpanda covers these per its status table)
- `navigation.rs` — navigate, back/forward, reload, wait_for
- `interaction.rs` — click, type_text, press_key, hover, scroll, select_option, set_file_input
- `extraction.rs` — get_text, get_links, get_dom, get_ax_tree, query_elements, evaluate_js
- `cookies.rs` — full support (Lightpanda has cookies + custom headers)
- `tabs.rs` / `session.rs` — process/context model differs slightly (see §6) but action surface stays

### 5.3 At risk / needs runtime capability check
- `search.rs` adapters rely on parsing rendered HTML from DuckDuckGo/Bing/Google — should work since these are HTML-lite/server-rendered pages, but should be validated against Lightpanda's JS/DOM coverage rather than assumed.
- `download.rs` — Lightpanda's status list doesn't explicitly confirm download handling; needs a spike to confirm CDP `Page.downloadWillBegin`-equivalent support, or falls back to Chromium.
- `evaluate_js` — gated by existing `allow_evaluate_js` config flag either way; Lightpanda's V8-based JS execution should support this natively.

---

## 6. Session/Process Model Changes

- Lightpanda's `serve` mode is a single long-lived process exposing one CDP endpoint with multiple browser contexts (`browser.createBrowserContext()` per Puppeteer example) — closer to "one Lightpanda process, many sessions" rather than Meleys' current "one Chromium process per session."
- Proposed model: **one shared Lightpanda process per Meleys runtime** (not per session), with each `BrowserSession` mapped to a Lightpanda **browser context**, and each `Tab` to a **page** within that context — this is a meaningful topology change from `BrowserConfig`'s current one-process-per-session assumption.
- Chromium fallback sessions keep the existing per-session process model, since that's what `chromiumoxide`/CDP launcher already does.
- `SessionManager` needs to track two pools: a shared Lightpanda process handle + per-session Chromium process handles (only spun up on fallback).

---

## 7. Configuration Additions (`config.rs`)

```toml
[engine]
default = "lightpanda"          # "lightpanda" | "chromium"
lightpanda_binary_path = ""      # empty = auto-discover via PATH/known install dirs
lightpanda_port = 9223           # avoid colliding with a user's own 9222 instance
startup_timeout_ms = 5000
fallback_enabled = true
max_engine_fallbacks_per_session = 1
obey_robots = true               # maps to lightpanda's --obey-robots

[engine.chromium]
# existing BrowserConfig fields (executable_path, headless, viewport, profile_dir)
```

- New `EngineConfig` struct in `config.rs`; `BrowserConfig` becomes `chromium: BrowserConfig` nested under it, kept backward-compatible via a migration/deprecation shim (old flat `MELEYS_BROWSER_*` env vars log a warning and map to `MELEYS_ENGINE_CHROMIUM_*`).

---

## 8. CDP/Launcher Layer Changes (`src/cdp/`)

- `launcher.rs` gains Lightpanda binary discovery: check `lightpanda_binary_path` config → `PATH` → common install dirs (Homebrew cellar, `~/.local/bin`, AUR path) — mirroring the existing Windows/Linux Chromium discovery pattern.
- New `launcher::spawn_lightpanda(config) -> Child` analogous to existing Chromium spawn, running `lightpanda serve --host 127.0.0.1 --port <cfg> [--obey-robots]`.
- Healthcheck: poll CDP `/json/version`-equivalent endpoint until ready or `startup_timeout_ms` elapses, then fall back per §4.
- `dom.rs`/`ax_tree.rs` injected-JS extraction should work unmodified against Lightpanda's V8-backed DOM, but needs validation — Lightpanda's DOM APIs are marked complete but coverage of edge-case DOM methods isn't guaranteed at 100%.

---

## 9. Observability & Errors

- New `MeleyError` variants: `EngineStartupFailed`, `EngineCapabilityUnsupported`, `EngineFallbackExhausted`.
- `tracing` spans tagged with `engine = "lightpanda" | "chromium"` throughout the action layer for debugging which engine handled a given call.
- `network_summary` in `Observation` should include whether `robots.txt` was obeyed (relevant since Lightpanda supports `--obey-robots` natively and Chromium doesn't have an equivalent built-in).

---

## 10. Rollout Plan

| Stage | Scope |
|---|---|
| **Spike** | Validate: download handling, evaluate_js coverage, search-engine adapter pages, DOM/AX extraction parity vs Chromium on a fixed test-site corpus |
| **v0.2.0** | `BrowserEngine` trait + `ChromiumEngine` refactor (behavior-preserving, no Lightpanda yet) — de-risks the abstraction before adding a second backend |
| **v0.3.0** | `LightpandaEngine` + session-level fallback, `default = "chromium"` still (opt-in via config) |
| **v0.4.0** | Flip default to `lightpanda`, remove `capture.rs`/screenshot/PDF from action set + MCP tool catalog + HTTP routes |
| **Phase 2 (separate spec)** | Native Rust engine, replacing both — reuses the same `BrowserEngine` trait boundary |

---

## 11. Open Items to Track (not blocking, but flagged)

- CORS is unimplemented in Lightpanda (tracked upstream at lightpanda-io/browser#2015) — any action touching cross-origin `fetch`/XHR should surface a clear `EngineCapabilityUnsupported`-style error rather than a silent failure, pending upstream fix.
- Lightpanda has no native Windows binary (WSL2 only) — Meleys' Windows installer (WiX/MSI) needs either a bundled WSL dependency note or Chromium-only defaults on Windows until this changes upstream.
- Lightpanda telemetry is on by default; Meleys should set `LIGHTPANDA_DISABLE_TELEMETRY=true` when spawning the subprocess, consistent with Meleys' own "no auth/localhost-only" privacy posture.
- Both Lightpanda and Meleys are AGPLv3 — no new licensing conflict, but worth a one-line note in `LICENSING.md` since Meleys would now bundle/spawn a second AGPL binary.
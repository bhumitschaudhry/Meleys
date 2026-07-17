# SPEC.md — Meleys: Agent-First Browser Runtime

**Name:** Meleys
**Status:** Draft v1
**Owner:** Personal project

---

## 1. Purpose & Philosophy

`meleys` is a local Rust runtime that gives an LLM-powered CLI coding agent hands
on a real, persistent, headless Chromium browser. It exposes a small set of
**mechanical primitives** — navigate, click, type, scroll, screenshot, extract,
search, manage tabs/cookies — and returns **structured observations** about the
resulting page state.

The runtime performs **zero reasoning**. It never decides what to click, never
summarizes content for meaning, never plans multi-step tasks. All intelligence
lives in the calling agent. `meleys` is a deterministic, stateless-in-reasoning
executor: given command X, it performs X against the browser and reports what is
now true about the page. This separation is the core design constraint — every
feature decision in this document should be checked against it.

### 1.1 Goals

- Let a CLI agent browse the open web using **real search engines** (Google,
  Bing, DuckDuckGo) and real websites, with no third-party search API dependency.
- Fast startup, low memory/CPU overhead, minimal dependency surface.
- Persistent sessions: cookies, localStorage, and login state survive across
  agent turns and (optionally) across process restarts.
- Multiple concurrent tabs/sessions, addressable by ID.
- A stable, versioned action API that's easy for an agent (or a human) to call
  correctly on the first try.
- Runs entirely locally; no cloud dependency beyond whatever websites the agent
  visits.

### 1.2 Non-Goals

- No built-in page understanding, summarization, or semantic ranking of results.
- No autonomous task planning, retries-with-reasoning, or "smart" element
  selection (e.g. "click the login button" resolved by the runtime). The agent
  supplies exact selectors/coordinates/refs; the runtime resolves them
  mechanically.
- Not a general-purpose scraping framework or test-automation tool (though it
  overlaps with both).
- No multi-user/remote-hosting concerns in v1 — this is a single-user local tool.

---

## 2. High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        CLI Coding Agent                         │
│                  (LLM-driven, external process)                 │
└───────────────┬───────────────────────────────┬─────────────────┘
                │ MCP (stdio/SSE)                │ HTTP/JSON (REST)
┌───────────────▼───────────────────────────────▼─────────────────┐
│                      Transport Layer                             │
│         mcp_server.rs            │        http_server.rs         │
│   (tools = actions, 1:1 mapping) │   (axum, same actions as      │
│                                   │    REST endpoints)            │
└───────────────┬───────────────────────────────┬─────────────────┘
                │                                 │
                └───────────────┬─────────────────┘
                                │
                    ┌───────────▼────────────┐
                    │     Action Dispatcher    │
                    │  (validates + routes     │
                    │   Action → Session)      │
                    └───────────┬────────────┘
                                │
                    ┌───────────▼────────────┐
                    │    Session Manager       │
                    │ (owns N BrowserSessions,  │
                    │  lifecycle, GC, locking)  │
                    └───────────┬────────────┘
                                │
              ┌─────────────────┼─────────────────┐
    ┌─────────▼────────┐ ┌──────▼───────┐ ┌───────▼────────┐
    │ BrowserSession #1  │ │ Session #2   │ │  Session #N    │
    │ - CDP connection   │ │              │ │                │
    │ - Tab(s)           │ │              │ │                │
    │ - Cookie jar/profile│ │              │ │                │
    └─────────┬────────┘ └──────────────┘ └────────────────┘
              │
    ┌─────────▼─────────────┐
    │  CDP Client Layer       │
    │  (chromiumoxide, wraps  │
    │   raw Chrome DevTools   │
    │   Protocol)             │
    └─────────┬─────────────┘
              │
    ┌─────────▼─────────────┐
    │  Headless Chromium      │
    │  process (spawned,      │
    │  one per profile, or     │
    │  shared with tabs)       │
    └────────────────────────┘
```

### 2.1 Module Layout (crate structure)

```
meleys/
├── Cargo.toml
├── SPEC.md
├── src/
│   ├── main.rs                 # entrypoint: parse config, start transports
│   ├── config.rs               # Config struct, env/file loading
│   ├── error.rs                # BrowserdError, Result alias
│   │
│   ├── transport/
│   │   ├── mod.rs
│   │   ├── mcp.rs              # MCP server: tool registration + dispatch
│   │   └── http.rs             # axum HTTP server: route registration
│   │
│   ├── actions/
│   │   ├── mod.rs              # Action enum, ActionRequest/Response types
│   │   ├── navigation.rs       # navigate, back, forward, reload, wait_for
│   │   ├── search.rs           # search_web (engine-aware convenience action)
│   │   ├── interaction.rs      # click, type_text, scroll, hover, key_press
│   │   ├── extraction.rs       # get_text, get_dom, get_ax_tree, get_links
│   │   ├── capture.rs          # screenshot, pdf_export
│   │   ├── download.rs         # download_file, list_downloads
│   │   ├── tabs.rs             # new_tab, close_tab, list_tabs, switch_tab
│   │   └── session.rs          # create_session, close_session, list_sessions
│   │
│   ├── session/
│   │   ├── mod.rs              # SessionManager
│   │   ├── browser_session.rs  # BrowserSession (one Chromium instance/profile)
│   │   └── profile.rs          # on-disk profile dir, cookie persistence
│   │
│   ├── cdp/
│   │   ├── mod.rs              # thin wrapper over chromiumoxide
│   │   ├── launcher.rs         # spawns/attaches to Chromium process
│   │   ├── dom.rs              # DOM snapshot → SimplifiedNode tree
│   │   └── ax_tree.rs          # accessibility tree extraction
│   │
│   ├── observation.rs          # Observation struct (the universal return type)
│   └── selector.rs             # Selector enum: Css, XPath, AxNodeId, BackendNodeId, Coordinates
│
└── tests/
    ├── integration_navigation.rs
    ├── integration_search.rs
    └── integration_sessions.rs
```

---

## 3. Technology Stack

| Concern                  | Choice                                                            |
|---------------------------|-------------------------------------------------------------------|
| Language                 | Rust (2021 edition, stable toolchain)                              |
| Async runtime            | `tokio` (multi-threaded)                                           |
| CDP client                | `chromiumoxide` (async, tokio-native CDP bindings)                 |
| Browser binary            | System-installed Chromium/Chrome, or `chromiumoxide`'s fetcher     |
| HTTP transport            | `axum` + `tower`                                                   |
| MCP transport             | Official/community Rust MCP SDK (stdio + optional SSE)             |
| Serialization             | `serde` / `serde_json`                                             |
| Config                    | `figment` or plain `serde` + env vars, TOML file                   |
| Logging                  | `tracing` + `tracing-subscriber`                                   |
| Error handling             | `thiserror` (library errors) + `anyhow` (binary glue)              |
| Screenshot/image encoding | native CDP `Page.captureScreenshot` (base64 PNG/JPEG)               |

`chromiumoxide` is preferred over `fantoccini`/WebDriver because it speaks raw
CDP directly (lower overhead, access to `Accessibility.*` and `DOM.*` domains
needed for structured extraction) and is fully async/Tokio-native.

---

## 4. Core Data Model

### 4.1 `Observation` — the universal return type

Every action returns an `Observation`. The agent never receives raw HTML or a
rendered image without asking; it always gets a structured object it can log,
diff, or reason over.

```rust
struct Observation {
    session_id: String,
    tab_id: String,
    action: String,          // echo of the action that produced this
    success: bool,
    timestamp: String,       // RFC3339
    url: Option<String>,     // current URL after the action
    title: Option<String>,   // current page title
    status_code: Option<u16>,// last main-frame HTTP status, if known
    result: ActionResult,    // action-specific payload, see below
    error: Option<ErrorInfo>,
    console_messages: Vec<ConsoleMessage>, // new JS console output since last obs
    network_summary: Option<NetworkSummary>, // optional lightweight net info
}

enum ActionResult {
    Empty,
    Text(String),
    Links(Vec<LinkInfo>),
    SearchResults(Vec<SearchResultItem>),
    Dom(SimplifiedNode),
    AxTree(AxNode),
    Screenshot { format: String, base64: String, width: u32, height: u32 },
    Download(DownloadInfo),
    Tabs(Vec<TabInfo>),
    Sessions(Vec<SessionInfo>),
    Elements(Vec<ElementInfo>),  // e.g. result of a query_selector_all
}

struct ErrorInfo {
    code: String,      // machine-readable, e.g. "ELEMENT_NOT_FOUND", "TIMEOUT"
    message: String,   // human-readable detail
    retryable: bool,
}
```

### 4.2 Selectors

The agent must always tell the runtime *which* element to act on. No implicit
resolution. A `Selector` is one of:

```rust
enum Selector {
    Css(String),
    XPath(String),
    AxNodeId(String),        // from a prior get_ax_tree observation
    BackendNodeId(i64),      // from a prior get_dom observation
    Coordinates { x: f64, y: f64 },
    Text { exact: bool, value: String }, // matches element by visible text
}
```

If a selector matches multiple elements, the runtime acts on the first
DOM-order match and reports `matched_count` in the observation's metadata so
the agent can disambiguate on the next call (e.g. by switching to
`nth(css, index)` — see §5.3).

### 4.3 Simplified DOM node

```rust
struct SimplifiedNode {
    backend_node_id: i64,
    tag: String,
    attributes: HashMap<String, String>, // filtered: id, class, href, role, aria-*, name, value, type
    text: Option<String>,                // direct text content, trimmed
    visible: bool,
    bounding_box: Option<Rect>,
    children: Vec<SimplifiedNode>,
}
```

Depth and attribute inclusion are configurable per-call (`max_depth`,
`include_hidden`) to keep payloads small — full-page DOM trees can be huge, and
oversized responses defeat the "minimal" goal as much as an unbounded action
list would.

### 4.4 Accessibility tree node

```rust
struct AxNode {
    ax_node_id: String,
    role: String,
    name: Option<String>,
    value: Option<String>,
    focusable: bool,
    focused: bool,
    disabled: bool,
    children: Vec<AxNode>,
}
```

The AX tree is the **preferred extraction surface** for interactive pages
(forms, buttons, links) because it's already semantically flattened by the
browser and much smaller than raw DOM.

---

## 5. Action Catalog (v1)

Each action below is exposed identically as (a) an MCP tool and (b) an HTTP
endpoint. MCP tool names use `snake_case`; HTTP endpoints follow
`POST /v1/sessions/{session_id}/tabs/{tab_id}/{action}` unless noted.

### 5.1 Session & Tab Management

| Action | Params | Returns |
|---|---|---|
| `create_session` | `profile_name?: string`, `headless?: bool = true`, `viewport?: {w,h}`, `user_agent?: string`, `default_search_engine?: "google"\|"bing"\|"duckduckgo"` (defaults to the runtime-wide config value, see §5.5.2) | `SessionInfo { session_id, profile_path }` |
| `close_session` | `session_id` | `Empty` |
| `list_sessions` | — | `Sessions(Vec<SessionInfo>)` |
| `new_tab` | `session_id`, `url?: string` | `TabInfo { tab_id, url }` |
| `close_tab` | `session_id`, `tab_id` | `Empty` |
| `list_tabs` | `session_id` | `Tabs(Vec<TabInfo>)` |
| `switch_tab` | `session_id`, `tab_id` | `TabInfo` (becomes "active" default for calls that omit tab_id) |

`SessionInfo` includes `created_at`, `profile_path`, `tab_count`.
`TabInfo` includes `tab_id`, `url`, `title`, `is_active`, `loading: bool`.

### 5.2 Navigation

| Action | Params | Returns |
|---|---|---|
| `navigate` | `url`, `wait_until?: "load" \| "domcontentloaded" \| "networkidle" = "load"`, `timeout_ms? = 30000` | `Observation` (url, title, status_code) |
| `go_back` | — | `Observation` |
| `go_forward` | — | `Observation` |
| `reload` | `ignore_cache?: bool = false` | `Observation` |
| `wait_for` | `condition: WaitCondition`, `timeout_ms? = 10000` | `Observation` |

```rust
enum WaitCondition {
    Selector { selector: Selector, state: "visible"|"hidden"|"attached"|"detached" },
    Navigation,
    NetworkIdle { idle_ms: u64 },
    Timeout { ms: u64 },          // plain sleep, escape hatch
    JsExpression { expr: String, poll_ms: u64 }, // evaluated, truthy = done
}
```

### 5.3 Interaction

| Action | Params | Returns |
|---|---|---|
| `click` | `selector: Selector`, `button? = "left"`, `click_count? = 1`, `nth? = 0` | `Observation` |
| `type_text` | `selector: Selector`, `text: string`, `clear_first?: bool = false`, `delay_ms?: u64 = 0` | `Observation` |
| `press_key` | `key: string` (e.g. `"Enter"`, `"Tab"`), `selector?: Selector` | `Observation` |
| `hover` | `selector: Selector` | `Observation` |
| `scroll` | `direction?: "up"\|"down"\|"left"\|"right"`, `amount_px?: i64`, `selector?: Selector` (scroll into view), `to_bottom?: bool` | `Observation` |
| `select_option` | `selector: Selector`, `value: string` | `Observation` |
| `set_file_input` | `selector: Selector`, `file_paths: Vec<String>` | `Observation` |

`nth` disambiguates when a `Selector::Css`/`XPath` matches multiple elements.

### 5.4 Extraction

| Action | Params | Returns |
|---|---|---|
| `get_text` | `selector?: Selector` (default: `body`), `max_chars? = 20000` | `Text(String)` |
| `get_links` | `selector?: Selector` (scope), `same_origin_only?: bool = false` | `Links(Vec<LinkInfo>)` |
| `get_dom` | `selector?: Selector`, `max_depth? = 6`, `include_hidden? = false` | `Dom(SimplifiedNode)` |
| `get_ax_tree` | `selector?: Selector`, `max_depth? = 8` | `AxTree(AxNode)` |
| `query_elements` | `selector: Selector`, `limit? = 50` | `Elements(Vec<ElementInfo>)` |
| `evaluate_js` | `expression: string` | `Text(String)` (JSON-stringified result; **sandboxed, see §8**) |

`LinkInfo { href, text, visible }`. `ElementInfo { backend_node_id, tag,
text, attributes, bounding_box, visible }`.

### 5.5 Web Search (convenience action)

| Action | Params | Returns |
|---|---|---|
| `search_web` | `query: string`, `engine?: "google"\|"bing"\|"duckduckgo"` (omit to use the configured default engine, see §5.5.2), `num_results? = 10` | `SearchResults(Vec<SearchResultItem>)` |

Behavior:
1. Navigates the active tab to the engine's search URL for `query`.
2. Waits for the results container to attach (`WaitCondition::Selector`, engine-specific, see §5.5.1).
3. Runs a **fixed, hand-maintained CSS extraction** for that engine (no
   heuristic/ML parsing — this is mechanical, versioned, and expected to break
   when engines change markup; see §11 risks) to pull result entries.
4. Returns them as data; **does not** rank, dedupe across engines, or
   editorialize. The agent decides which result to open (typically via a
   follow-up `navigate` to the chosen `href`).

```rust
struct SearchResultItem {
    rank: u32,
    title: String,
    url: String,
    snippet: Option<String>,
}
```

#### 5.5.1 Engine adapters

Each engine gets a small adapter implementing:

```rust
trait SearchEngineAdapter {
    fn search_url(&self, query: &str) -> String;
    fn results_ready_selector(&self) -> Selector;
    fn extract(&self, dom: &SimplifiedNode) -> Vec<SearchResultItem>;
}
```

v1 ships adapters for Google, Bing, DuckDuckGo (DDG's HTML-lite endpoint
preferred where possible for stability/lower JS-rendering cost). Adapters live
in `actions/search.rs` behind a small registry, so adding a fourth engine is a
localized change — this is the main "modularity" seam called out in the
brief.

#### 5.5.2 Configurable default engine

The engine used when `search_web` is called without an explicit `engine`
param is a user setting, not a hardcoded constant:

- Set via `config.toml` (`[search] default_engine = "duckduckgo"`, see §9), or
- Overridden per-session at creation time (`create_session(..., default_search_engine?: string)`), or
- Updated live without a restart via a dedicated action:

| Action | Params | Returns |
|---|---|---|
| `set_default_search_engine` | `engine: "google"\|"bing"\|"duckduckgo"`, `session_id?: string` (omit = runtime-wide default; provide = session-level override) | `Empty` |
| `get_default_search_engine` | `session_id?: string` | result: `{ engine: string, scope: "runtime"\|"session" }` |

Precedence when resolving the engine for a given `search_web` call: explicit
`engine` param on the call &gt; session-level override &gt; runtime-wide
config default &gt; hardcoded fallback (`"duckduckgo"`, chosen as the fallback
since it needs the least JS rendering and is least prone to bot-challenges).

### 5.6 Capture & Downloads

| Action | Params | Returns |
|---|---|---|
| `screenshot` | `selector?: Selector` (element vs full page), `full_page?: bool = false`, `format? = "png"` | `Screenshot { base64, width, height }` |
| `export_pdf` | `landscape?: bool = false` | `Download { path }` (saved to disk, path returned — not inlined as base64 to keep payloads small) |
| `download_file` | `url: string`, `save_as?: string` | `Download(DownloadInfo)` |
| `list_downloads` | `session_id` | result: `Vec<DownloadInfo>` |

```rust
struct DownloadInfo { id, url, path, size_bytes, state: "in_progress"|"completed"|"failed", started_at, completed_at }
```

Downloads are written under `{profile_dir}/downloads/` by default,
configurable per session.

### 5.7 Cookies / Storage

| Action | Params | Returns |
|---|---|---|
| `get_cookies` | `urls?: Vec<String>` | result: `Vec<Cookie>` |
| `set_cookies` | `cookies: Vec<Cookie>` | `Empty` |
| `clear_cookies` | — | `Empty` |
| `get_local_storage` | `origin?: string` | `Text(String)` (JSON map) |

Cookies also persist automatically to the on-disk profile (see §6.3) — these
actions are for direct agent inspection/manipulation, not the primary
persistence mechanism.

---

## 6. Session & Persistence Model

### 6.1 BrowserSession

A `BrowserSession` wraps exactly one Chromium process (one user-data-dir /
"profile"). It may own multiple tabs (CDP targets). Sessions are the unit of:

- Cookie/localStorage isolation
- Login state
- Concurrency (one session = one independent browsing identity)

### 6.2 Concurrency model

- `SessionManager` holds `Arc<RwLock<HashMap<SessionId, Arc<BrowserSession>>>>`.
- Each `BrowserSession` serializes actions **per tab** via an internal
  `tokio::sync::Mutex` — two actions on the same tab never race, but actions
  on different tabs (same or different sessions) run concurrently.
- Long-running waits (`wait_for`, navigation) use `tokio::time::timeout` and
  return a structured `TIMEOUT` error rather than hanging the connection.

### 6.3 Profile persistence

- Each session gets a directory: `~/.local/share/meleys/profiles/{name}/`
  (or platform-equivalent via `dirs` crate), reused across runtime restarts if
  `profile_name` is passed again to `create_session`.
- This directory *is* the Chromium `--user-data-dir`; cookies, localStorage,
  and cache persist there natively — no custom cookie-jar re-implementation
  needed for the default case.
- `get_cookies`/`set_cookies` remain available for agent-driven inspection or
  cross-profile cookie transplant.

### 6.4 Lifecycle / cleanup

- Idle session timeout (configurable, default: none — sessions persist until
  explicitly closed or the runtime process exits) to match "persistent
  sessions" goal.
- On `close_session`, the Chromium process is terminated gracefully (CDP
  `Browser.close`) with a hard kill fallback after a grace period.
- On runtime shutdown (SIGINT/SIGTERM), all sessions are closed gracefully
  before exit.

---

## 7. Transport Layer

### 7.1 MCP server

- Each action in §5 is registered as an MCP tool with a JSON Schema generated
  from its Rust param struct (via `schemars`).
- Tool names mirror the action names exactly (`navigate`, `click`,
  `search_web`, etc.) for a 1:1 mental model between transports.
- Runs over stdio by default (simplest for a CLI agent to spawn as a
  subprocess); SSE/HTTP MCP transport is a stretch goal, not v1-blocking.

### 7.2 HTTP API

- `axum` router, JSON request/response bodies mirroring the same param/result
  structs.
- Route shape: `POST /v1/sessions`, `POST /v1/sessions/{id}/tabs`,
  `POST /v1/sessions/{id}/tabs/{tab_id}/navigate`, etc. Actions that aren't
  tab-scoped (session/tab management itself) live one level up.
- `GET /v1/sessions/{id}/tabs/{tab_id}/observation` — fetch current
  observation without performing an action (useful for polling after an
  async download or slow navigation).
- No auth in v1 (localhost-only binding by default, see §8).

### 7.3 Shared core

Both transports call into the same `ActionDispatcher::dispatch(Action) ->
Observation` function — the transport layer is pure (de)serialization plus
routing, guaranteeing MCP and HTTP behave identically.

---

## 8. Security & Safety Boundaries

- **Local-only by default.** HTTP server binds to `127.0.0.1` unless a config
  flag explicitly opens it to another interface.
- **No implicit code execution.** `evaluate_js` exists (useful escape hatch)
  but is off by default and must be enabled via config (`allow_evaluate_js =
  true`) — since arbitrary JS eval is the main foot-gun in a tool like this.
- **Download path containment.** Downloads are confined to the profile's
  `downloads/` dir unless the agent supplies an explicit `save_as` path, which
  is validated against a configurable allow-list of directories.
- **No credential storage/management beyond the browser's own cookie jar** —
  `meleys` does not itself hold or transmit secrets; it only persists what
  Chromium persists.
- **No reasoning about content safety.** `meleys` does not filter, block, or
  judge which URLs/content the agent asks it to fetch — that responsibility
  sits with the agent/its operator, consistent with the "no reasoning" design
  principle. This should be stated plainly in the README as an operator
  responsibility, not silently assumed.

---

## 9. Configuration

`config.toml` (overridable by env vars, e.g. `BROWSERD_HTTP_PORT`):

```toml
[server]
http_port = 8787
http_bind = "127.0.0.1"
mcp_transport = "stdio"   # or "sse"

[browser]
executable_path = ""       # empty = auto-detect system Chrome/Chromium
headless = true
default_viewport = { width = 1280, height = 800 }
profile_dir = "~/.local/share/meleys/profiles"

[search]
default_engine = "duckduckgo"   # "google" | "bing" | "duckduckgo"

[limits]
max_sessions = 8
default_action_timeout_ms = 30000
max_dom_nodes_per_call = 2000
allow_evaluate_js = false

[downloads]
dir = "~/.local/share/meleys/downloads"
allowed_save_dirs = []
```

---

## 10. Error Handling

Every failure path returns an `Observation` with `success = false` and a
populated `ErrorInfo` — the runtime never panics the process on a bad
selector, a closed tab, or a navigation timeout. Reserved `code` values (v1):

`SESSION_NOT_FOUND`, `TAB_NOT_FOUND`, `ELEMENT_NOT_FOUND`,
`ELEMENT_NOT_INTERACTABLE`, `TIMEOUT`, `NAVIGATION_FAILED`,
`INVALID_SELECTOR`, `SEARCH_ENGINE_PARSE_FAILED`, `DOWNLOAD_FAILED`,
`CDP_CONNECTION_LOST`, `JS_EVAL_DISABLED`, `INTERNAL_ERROR`.

`CDP_CONNECTION_LOST` triggers an automatic single reconnect attempt at the
session level before surfacing to the agent; it is not silently retried
indefinitely.

---

## 11. Known Risks / Open Design Questions

- **Search result markup drift.** Google/Bing/DDG change their HTML
  frequently; the `SearchEngineAdapter::extract` implementations will need
  periodic maintenance. Mitigated by isolating this logic per-adapter and
  returning `SEARCH_ENGINE_PARSE_FAILED` (rather than empty/garbage results)
  when extraction confidently fails, so the agent can fall back to a plain
  `navigate` + `get_text`/`get_dom` on the results page.
- **Bot detection.** Headless Chromium is sometimes challenged (CAPTCHAs,
  "unusual traffic" pages) by search engines. v1 does not attempt evasion
  (no stealth patches, no fingerprint spoofing) — this is a deliberate
  scope cut, flagged here for future revisiting if it blocks real usage.
- **DOM/AX payload size.** Even simplified, some pages produce very large
  trees. `max_depth`/`max_dom_nodes_per_call` caps exist, but truncation
  strategy (breadth-first vs depth-first cutoff) needs a concrete decision
  during implementation.
- **MCP SDK maturity in Rust.** The Rust MCP ecosystem is younger than
  Python/TS; may need to hand-roll the stdio JSON-RPC framing against the MCP
  spec rather than relying on a mature SDK. Worth a short spike before
  committing to a specific crate.

---

## 12. Milestones

1. **M0 — CDP plumbing:** launch Chromium via `chromiumoxide`, single session,
   single tab, `navigate` + `screenshot` + `get_text` working end-to-end over
   a bare HTTP server.
2. **M1 — Full action catalog (§5.1–5.4, 5.6–5.7):** interaction, extraction,
   tabs/sessions, downloads, cookies.
3. **M2 — Search adapters (§5.5):** DuckDuckGo first (simplest markup), then
   Bing, then Google.
4. **M3 — MCP transport:** tool schema generation, stdio server, parity
   tests against the HTTP surface.
5. **M4 — Hardening:** error taxonomy complete, timeouts everywhere, graceful
   shutdown, profile persistence across restarts verified.
6. **M5 (stretch):** SSE MCP transport, `evaluate_js` sandboxing options,
   basic stealth/anti-detection tuning if bot-blocking proves limiting in
   practice.

---

## 13. Example End-to-End Flow

```
agent -> create_session()                         -> session_id: "s1"
agent -> search_web("rust async cdp library", "duckduckgo")
      <- SearchResults([{rank:1, title:"chromiumoxide - crates.io", url:"...", snippet:"..."}, ...])
agent -> navigate(url = results[0].url)
      <- Observation{url, title, status_code: 200}
agent -> get_ax_tree(max_depth=6)
      <- AxTree(...)
agent -> click(selector = Text{value:"Docs"})
      <- Observation{url: ".../docs", ...}
agent -> get_text()
      <- Text("...")
agent -> close_session("s1")
```

At no point does `meleys` decide *which* result to open or *what* "Docs"
means — the agent made both calls explicitly from the structured data it was
given.

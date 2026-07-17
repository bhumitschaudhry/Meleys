# Meleys Architecture

## Overview

Meleys is a lightweight, agent-first browser runtime written in Rust. It gives LLM agents programmatic, persistent control over a headless Chromium browser through deterministic primitives (navigate, click, type, scroll, screenshot, extract) and returns structured observations (accessibility trees, DOM snapshots, search results) without performing any reasoning itself.

- **License**: GNU AGPL v3
- **Version**: 0.1.0 (Rust 2021 edition, MSRV 1.80.0)
- **Platforms**: Windows, macOS, Linux

---

## Tech Stack

| Concern | Technology |
|---|---|
| Language | Rust (2021 edition, stable toolchain) |
| Async Runtime | `tokio` (multi-threaded, full features) |
| Browser Control | `chromiumoxide` 0.7 (async CDP bindings, tokio-native) |
| HTTP Server | `axum` 0.8 + `tower-http` (with tracing layer) |
| Serialization | `serde` / `serde_json` |
| Configuration | `figment` 0.10 (TOML + env vars with `MELEYS_` prefix) |
| Error Handling | `thiserror` (library errors) + `anyhow` (binary glue) |
| Logging | `tracing` + `tracing-subscriber` (env-filter) |
| ID Generation | `uuid` v4 |
| Timestamps | `chrono` (RFC3339) |
| Image Encoding | `base64` 0.22 (for screenshots) |
| Path Resolution | `dirs` (platform-specific directories) |
| Windows Installer | WiX v3 (MSI) |

---

## Project Structure

```
Meleys/
  Cargo.toml          # Package manifest
  Cargo.lock          # Locked dependencies
  config.toml         # Default runtime configuration
  spec.md             # Full design specification
  README.md           # Project overview

  src/                # All Rust source code
    main.rs           # Binary entrypoint
    lib.rs            # Library root (re-exports all modules)
    config.rs         # Configuration structs + Figment loading
    error.rs          # MeleyError enum with machine-readable codes
    observation.rs    # Universal Observation response type + data models
    selector.rs       # Selector enum (Css, XPath, AxNodeId, etc.)
    actions/          # Browser action implementations
    cdp/              # Chrome DevTools Protocol wrappers
    session/          # Session + browser process management
    transport/        # HTTP and MCP transport layers
    setup/            # Agent integration CLI

  tests/              # Integration tests
  docs/               # User-facing documentation
  wix/                # Windows MSI installer
  .github/workflows/  # CI pipeline
```

---

## Core Data Types

### Observation (`src/observation.rs`)

The universal return type. Every action returns an `Observation` containing:

- `session_id`, `tab_id`, `action` (echo), `success`, `timestamp`
- `url`, `title`, `status_code` (post-action page state)
- `result: ActionResult` -- enum with variants: `Empty`, `Text`, `Links`, `SearchResults`, `Dom`, `AxTree`, `Screenshot`, `Download`, `Tabs`, `Sessions`, `Elements`, `Cookies`, `SearchEngine`
- `error: Option<ErrorInfo>` with `code`, `message`, `retryable`
- `console_messages`, `network_summary` (observability metadata)

### Selector (`src/selector.rs`)

A tagged enum for element targeting:

- `Css(String)`, `XPath(String)`, `AxNodeId(String)`, `BackendNodeId(i64)`, `Coordinates{x,y}`, `Text{exact,value}`
- Each variant can produce a JS expression for element resolution
- Supports `nth` indexing for multi-match disambiguation

### MeleyError (`src/error.rs`)

A comprehensive error enum with 16 variants covering domain errors (`SessionNotFound`, `TabNotFound`, `ElementNotFound`, `Timeout`, `NavigationFailed`, `SearchEngineParseFailed`, etc.) and infrastructure errors (wraps `CdpError`, `serde_json::Error`, `io::Error`, `anyhow::Error`).

---

## Configuration (`src/config.rs`)

Uses `figment` for layered configuration:

1. **Defaults** (hardcoded in code)
2. **TOML file** (`config.toml` in working directory)
3. **Environment variables** (prefixed `MELEYS_`)

Config sections:

- `ServerConfig`: `http_port` (8787), `http_bind` (127.0.0.1)
- `BrowserConfig`: `executable_path`, `headless` (true), `default_viewport` (1280x800), `profile_dir`
- `SearchConfig`: `default_engine` ("duckduckgo")
- `LimitsConfig`: `max_sessions` (8), `default_action_timeout_ms` (30000), `max_dom_nodes_per_call` (2000), `allow_evaluate_js` (false)
- `DownloadsConfig`: `dir`, `allowed_save_dirs`

---

## Module Architecture

### Action Layer (`src/actions/`)

Each action module is a standalone async function receiving `&Arc<SessionManager>` as the first argument, keeping action logic decoupled from transport and session internals.

| Module | Functions | Purpose |
|---|---|---|
| `navigation.rs` | `navigate`, `go_back`, `go_forward`, `reload`, `wait_for` | URL navigation and page state waits |
| `interaction.rs` | `click`, `type_text`, `press_key`, `hover`, `scroll`, `select_option`, `set_file_input` | Mouse/keyboard/form interactions |
| `extraction.rs` | `get_text`, `get_links`, `get_dom`, `get_ax_tree`, `query_elements`, `evaluate_js` | Page content extraction |
| `search.rs` | `search_web`, `set_default_search_engine`, `get_default_search_engine` | Web search via engine adapters |
| `capture.rs` | `screenshot`, `export_pdf` | Visual capture |
| `download.rs` | `download_file`, `list_downloads` | File download management |
| `cookies.rs` | `get_cookies`, `set_cookies`, `clear_cookies`, `get_local_storage` | Cookie/storage operations |
| `tabs.rs` | `new_tab`, `close_tab`, `list_tabs`, `switch_tab` | Tab lifecycle |
| `session.rs` | `create_session`, `close_session`, `list_sessions` | Session lifecycle |

**Action Pattern**: Every action function follows the same flow:
1. Resolve session and tab from IDs
2. Lock the page mutex
3. Execute with `tokio::time::timeout`
4. Return `Observation::success(...)` or `Observation::failure(...)`

### Session Layer (`src/session/`)

**`SessionManager`** -- The global session registry:
- Stores `Arc<RwLock<HashMap<String, Arc<BrowserSession>>>>`
- Enforces `max_sessions` limit
- Manages lifecycle: `create_session`, `close_session`, `close_all` (graceful shutdown)

**`BrowserSession`** -- One Chromium process, one profile, multiple tabs:
- Fields: `session_id`, `profile`, `browser`, `tabs`, `active_tab_id`, `created_at`, `default_search_engine`
- Tab management: `new_tab`, `get_page`, `get_active_page`, `close_tab`, `switch_tab`, `list_tabs`

**`Tab`** -- A single tab within a session with `tab_id`, `page: Arc<Mutex<Page>>`, `is_active`

**`Profile`** -- On-disk Chromium profile directory management with named or temporary profiles

### CDP Layer (`src/cdp/`)

**`launcher.rs`** -- Browser process spawning with cross-platform binary discovery (Windows: Program Files, LocalAppData, Playwright cache; Linux: standard paths, `which` fallback)

**`dom.rs`** -- Simplified DOM extraction via injected JavaScript that traverses the DOM, filtering to `SimplifiedNode` tree. Strips `<script>`, `<style>`, `<svg>`; preserves functional attributes.

**`ax_tree.rs`** -- Accessibility tree extraction via injected JavaScript that builds semantic AX tree with roles and accessible names.

### Transport Layer (`src/transport/`)

**`http.rs`** -- Axum-based REST API server with 30+ endpoints under `/v1/` prefix. Route pattern: `POST /v1/sessions/{session_id}/tabs/{tab_id}/{action}`

**`mcp.rs`** -- MCP JSON-RPC 2.0 stdio server implementing Model Context Protocol for LLM agent integration. Handles `initialize`, `tools/list`, `tools/call` with full tool catalog and JSON Schema definitions.

### Search Engine Adapters (`src/actions/search.rs`)

**`SearchEngineAdapter` trait**: `name()`, `search_url()`, `results_ready_selector()`, `extract()`

Three implementations:
- **DuckDuckGo** -- Uses HTML-lite endpoint, parses `.result`, `.result__a`, `.result__snippet` classes
- **Bing** -- Parses `#b_results`, `.b_algo`, `.b_caption` classes
- **Google** -- Parses `#rso`, `.g` containers, `h3`/`h2` titles, `.VwiC3b` snippets

**`SearchRegistry`**: Supports runtime-wide and per-session default engine switching. Resolution: explicit `engine` param > session override > runtime config > hardcoded fallback.

### Setup / Agent Integration (`src/setup/mod.rs`)

CLI commands (`meleys setup install|uninstall|list`) for auto-registering Meleys as an MCP server in coding agent configs:

| Agent | Config Path | Config Style |
|---|---|---|
| Claude Code | `~/.claude.json` + `~/.claude/settings.json` | `mcpServers` root + `permissions.deny` |
| Cline | `~/.cline/mcp.json` | `mcpServers` root |
| Cursor | `~/.cursor/mcp.json` | `mcpServers` root |
| VS Code / Copilot | `%APPDATA%/Code/User/settings.json` | `mcp.servers` nested |

All edits are idempotent and reversible.

---

## Entry Points

### Binary Entrypoint (`src/main.rs`)

```
meleys                    -> HTTP mode (default, port 8787)
meleys --mcp              -> MCP stdio mode
meleys setup install      -> Agent registration
```

Initialization sequence:
1. Initialize `tracing` logger with env-filter
2. Load `Config` via Figment (TOML + env vars)
3. Create shared resources: `SessionManager`, `SearchRegistry`, `DownloadRegistry`
4. Parse CLI args to select mode
5. For HTTP: build axum router, bind TCP listener, install graceful shutdown
6. For MCP: spawn stdio JSON-RPC loop with shutdown on ctrl+c

---

## Data Flow

```
Client/Agent
    |
    |-- HTTP (JSON REST) ----> axum Router
    |                              |
    |-- MCP (stdio JSON-RPC) -> dispatch_tool
                                   |
                                   v
                    Action function (actions/*.rs)
                    e.g. navigation::navigate()
                                   |
                    +-----------+--+
                    |           |
                    v           v
              SessionManager  SearchRegistry
              .get_session()  .get()
                    |
                    v
              BrowserSession
              .get_active_page()
                    |
                    v
              Arc<Mutex<Page>>
              .lock().await
                    |
                    v
              CDP operations
                    |
                    v
              Headless Chromium process
                    |
                    v
              Observation::success/failure()
                    |
                    v
              JSON response back to client
```

---

## Concurrency Model

- **Session Isolation**: Each session = one Chromium process + one `--user-data-dir`
- **Global Session Map**: `Arc<RwLock<HashMap<String, Arc<BrowserSession>>>>`
- **Tab Serialization**: Each `Page` wrapped in `Arc<Mutex<Page>>` -- same-tab actions serialized, cross-tab concurrent
- **Timeouts**: Every action uses `tokio::time::timeout` with configurable duration
- **Graceful Shutdown**: Signal handler drains all sessions via `session_manager.close_all()`

---

## Security Design

- **Local-only binding**: HTTP server defaults to `127.0.0.1`
- **JS execution disabled by default**: `allow_evaluate_js = false` must be explicitly enabled
- **Download path containment**: Downloads confined to profile's `downloads/` directory
- **Profile name sanitization**: All non-alphanumeric chars (except `-`, `_`) replaced with `_`
- **No stealth/bot evasion**: Deliberate scope cut -- calling agent manages browsing safety
- **No auth in v1**: Localhost-only trust model

---

## Build System & CI

### Cargo Configuration

- Binary target: `src/main.rs` (name: `meleys`)
- Library target: `src/lib.rs` (name: `meleys`)
- Release profile: `opt-level = 3`

### CI Pipeline (`.github/workflows/ci.yml`)

8 jobs:
1. **Rustfmt** -- Format check
2. **Clippy** -- Lint with `-D warnings`
3. **Build** -- Cross-platform build matrix (Ubuntu, macOS, Windows)
4. **Tests** -- Unit tests, doc tests, non-browser integration tests
5. **MSRV** -- Rust 1.80.0 minimum version check
6. **Security Audit** -- `rustsec/audit-check`
7. **Dependency Check** -- `cargo-machete` for unused dependencies
8. **Cross-compile** -- `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-gnu`
9. **Code Coverage** -- `cargo-llvm-cov` + Codecov upload
10. **Release Build** -- Builds release binary and verifies health endpoint

### Windows Installer (`wix/`)

WiX v3 MSI that installs `meleys.exe` + `config.toml` to `C:\Program Files\Meleys`, adds to system PATH, and auto-registers with coding agents via post-install custom action.

---

## Key Design Decisions

1. **Action functions, not methods**: All browser actions are standalone async functions taking `&Arc<SessionManager>`, decoupling transport from business logic completely.

2. **Universal `Observation` return type**: Every operation returns the same envelope type, making both HTTP and MCP transports handle responses uniformly.

3. **JS-based DOM extraction**: Rather than using CDP's synchronous `DOM.getDocument`, Meleys injects JavaScript functions that traverse the DOM and return structured JSON -- simpler, more controllable, and cleaner output.

4. **Search engine adapters as a trait**: Deterministic and versionable CSS extractors, but requires maintenance when engines change markup.

5. **MCP as first-class transport**: Full tool catalog with JSON Schema definitions, immediately usable with Claude Code, Cline, Cursor, and VS Code.

6. **No monorepo complexity**: Single-crate Rust project with both binary and library targets, keeping the build simple while allowing integration tests to import the library.

7. **Dual entrypoint architecture**: Library crate (`lib.rs`) exposes all modules publicly for testing; binary (`main.rs`) handles startup, config, and transport orchestration.

8. **Platform-aware browser discovery**: Launcher has platform-specific binary search paths, working out of the box on most developer machines.

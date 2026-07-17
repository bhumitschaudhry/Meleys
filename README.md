# Meleys — Agent-First Browser Runtime

Meleys is a local Rust runtime that gives an LLM-powered CLI coding agent hands on a real, persistent, headless Chromium browser via the Chrome DevTools Protocol (CDP). It exposes a small set of **mechanical primitives** and returns **structured observations** about the resulting page state.

**The runtime performs zero reasoning.** All intelligence lives in the calling agent.

## Quick Start

### Build
```bash
cargo build --release
```

### HTTP Mode (default)
```bash
./target/release/meleys
# Listening on http://127.0.0.1:8787
```

### MCP stdio Mode
```bash
./target/release/meleys --mcp
```

## Configuration

Create a `config.toml` in the working directory:

```toml
[server]
http_port = 8787
http_bind = "127.0.0.1"
mcp_transport = "stdio"

[browser]
executable_path = ""      # empty = auto-detect system Chrome/Chromium
headless = true
default_viewport = { width = 1280, height = 800 }
profile_dir = "~/.local/share/meleys/profiles"

[search]
default_engine = "duckduckgo"   # "google" | "bing" | "duckduckgo"

[limits]
max_sessions = 8
default_action_timeout_ms = 30000
max_dom_nodes_per_call = 2000
allow_evaluate_js = false     # off by default for security

[downloads]
dir = "~/.local/share/meleys/downloads"
allowed_save_dirs = []
```

### Environment Variable Overrides
Any config key can be overridden with `MELEYS_` prefix:
```bash
MELEYS_SERVER_HTTP_PORT=9000 ./target/release/meleys
```

## HTTP API Reference

All endpoints return an `Observation` JSON object.

### Session Management
```
POST /v1/sessions                          # create_session
GET  /v1/sessions                          # list_sessions
POST /v1/sessions/{session_id}/close       # close_session

GET  /v1/sessions/{session_id}/tabs        # list_tabs
POST /v1/sessions/{session_id}/tabs        # new_tab
POST /v1/sessions/{session_id}/tabs/{tab_id}/close   # close_tab
POST /v1/sessions/{session_id}/tabs/{tab_id}/switch  # switch_tab
```

### Navigation
```
POST /v1/sessions/{sid}/tabs/{tid}/navigate
POST /v1/sessions/{sid}/tabs/{tid}/go_back
POST /v1/sessions/{sid}/tabs/{tid}/go_forward
POST /v1/sessions/{sid}/tabs/{tid}/reload
POST /v1/sessions/{sid}/tabs/{tid}/wait_for
```

### Interaction
```
POST /v1/sessions/{sid}/tabs/{tid}/click
POST /v1/sessions/{sid}/tabs/{tid}/type_text
POST /v1/sessions/{sid}/tabs/{tid}/press_key
POST /v1/sessions/{sid}/tabs/{tid}/hover
POST /v1/sessions/{sid}/tabs/{tid}/scroll
POST /v1/sessions/{sid}/tabs/{tid}/select_option
POST /v1/sessions/{sid}/tabs/{tid}/set_file_input
```

### Extraction
```
POST /v1/sessions/{sid}/tabs/{tid}/get_text
POST /v1/sessions/{sid}/tabs/{tid}/get_links
POST /v1/sessions/{sid}/tabs/{tid}/get_dom
POST /v1/sessions/{sid}/tabs/{tid}/get_ax_tree
POST /v1/sessions/{sid}/tabs/{tid}/query_elements
POST /v1/sessions/{sid}/tabs/{tid}/evaluate_js
```

### Capture & Downloads
```
POST /v1/sessions/{sid}/tabs/{tid}/screenshot
POST /v1/sessions/{sid}/tabs/{tid}/export_pdf
POST /v1/sessions/{sid}/tabs/{tid}/download_file
GET  /v1/sessions/{sid}/downloads
```

### Cookies & Storage
```
GET    /v1/sessions/{sid}/tabs/{tid}/cookies
POST   /v1/sessions/{sid}/tabs/{tid}/cookies
DELETE /v1/sessions/{sid}/tabs/{tid}/cookies
GET    /v1/sessions/{sid}/tabs/{tid}/local_storage
```

### Search
```
POST /v1/sessions/{sid}/tabs/{tid}/search_web
GET  /v1/search_engine
POST /v1/search_engine
GET  /v1/sessions/{sid}/search_engine
POST /v1/sessions/{sid}/search_engine
```

## MCP Tool Reference

When running in `--mcp` mode, Meleys speaks JSON-RPC 2.0 over stdio.

**Initialize:**
```json
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}
```

**List tools:**
```json
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
```

**Call a tool:**
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "navigate",
    "arguments": {
      "session_id": "...",
      "url": "https://example.com"
    }
  }
}
```

## Example Flow

```bash
# 1. Create a session
curl -s -X POST http://localhost:8787/v1/sessions \
  -H 'Content-Type: application/json' \
  -d '{"profile_name": "my-agent"}' | jq .

# 2. Navigate
curl -s -X POST http://localhost:8787/v1/sessions/{sid}/tabs/{tid}/navigate \
  -H 'Content-Type: application/json' \
  -d '{"url": "https://duckduckgo.com"}' | jq .

# 3. Search
curl -s -X POST http://localhost:8787/v1/sessions/{sid}/tabs/{tid}/search_web \
  -H 'Content-Type: application/json' \
  -d '{"query": "rust async browser automation"}' | jq .

# 4. Screenshot
curl -s -X POST http://localhost:8787/v1/sessions/{sid}/tabs/{tid}/screenshot \
  -H 'Content-Type: application/json' \
  -d '{}' | jq .result.data.base64 | base64 -d > screenshot.png
```

## Selector Types

Selectors are JSON objects with a `type` field:

```json
{"type": "Css", "value": "#search-input"}
{"type": "XPath", "value": "//button[text()='Submit']"}
{"type": "Text", "value": {"exact": true, "value": "Sign In"}}
{"type": "Coordinates", "value": {"x": 100, "y": 200}}
{"type": "BackendNodeId", "value": 42}
```

## Error Codes

| Code | Meaning |
|------|---------|
| `SESSION_NOT_FOUND` | Session ID doesn't exist |
| `TAB_NOT_FOUND` | Tab ID doesn't exist |
| `ELEMENT_NOT_FOUND` | Selector matched no element |
| `ELEMENT_NOT_INTERACTABLE` | Element exists but can't be clicked/typed |
| `TIMEOUT` | Action exceeded timeout_ms |
| `NAVIGATION_FAILED` | Page navigation failed |
| `INVALID_SELECTOR` | Malformed selector |
| `SEARCH_ENGINE_PARSE_FAILED` | Couldn't extract results from search page |
| `DOWNLOAD_FAILED` | File download failed |
| `CDP_CONNECTION_LOST` | Lost connection to browser |
| `JS_EVAL_DISABLED` | evaluate_js called but allow_evaluate_js=false |
| `INTERNAL_ERROR` | Unexpected runtime error |

## Security Notes

- HTTP server binds to `127.0.0.1` by default (localhost only)
- `evaluate_js` is **disabled by default** — set `allow_evaluate_js = true` in config to enable
- Downloads are confined to the session's downloads directory
- No content filtering — the calling agent is responsible for content safety

## Architecture

```
CLI Agent (LLM)
    │
    ├── HTTP/JSON  ──► axum router ──► ActionDispatcher
    │
    └── MCP stdio ──► JSON-RPC 2.0 ──► ActionDispatcher
                                              │
                                       SessionManager
                                              │
                                    BrowserSession (chromiumoxide)
                                              │
                                       Headless Chromium
```
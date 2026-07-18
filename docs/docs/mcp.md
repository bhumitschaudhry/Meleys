---
id: mcp
title: MCP Reference
sidebar_position: 5
---

# Model Context Protocol (MCP) Reference

Meleys includes a native **Model Context Protocol (MCP)** server implementation. This allows LLM client applications (like Claude Desktop, Cline, or Cursor) to automatically discover and execute browser automation tools.

---

## Stdio JSON-RPC 2.0 Transport

When started with the `--mcp` command-line flag:
```bash
./target/release/meleys --mcp
```
Meleys runs an MCP stdio server. It listens for JSON-RPC 2.0 requests on standard input (`stdin`) and writes responses to standard output (`stdout`). Log output is safely redirected to `stderr` to prevent JSON-RPC frame corruption.

---

## Protocol Lifecycle

### 1. Initialization
The client initiates the connection by sending an `initialize` request.

**Client Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {}
}
```

**Meleys Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "protocolVersion": "2024-11-05",
    "capabilities": {},
    "serverInfo": {
      "name": "meleys",
      "version": "0.1.0"
    }
  }
}
```

---

### 2. Discovering Tools
To get a list of available browser tools, the client calls `tools/list`.

**Client Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/list",
  "params": {}
}
```

**Meleys Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "tools": [
      {
        "name": "create_session",
        "description": "Create a new browser session",
        "inputSchema": {
          "type": "object",
          "properties": {
            "profile_name": { "type": "string", "description": "Optional profile name for persistence" },
            "headless": { "type": "boolean", "description": "Run headless (default: true)" },
            "default_search_engine": { "type": "string", "enum": ["google", "bing", "duckduckgo"] },
            "engine_preference": { "type": "string", "enum": ["auto", "lightpanda", "chromium"], "description": "Preferred browser engine" }
          },
          "required": []
        }
      },
      ...
    ]
  }
}
```

---

### 3. Calling a Tool
The client executes a browser action by calling `tools/call`. Tool names and arguments map 1:1 with Meleys' HTTP API request parameters.

**Client Request (Navigate):**
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "navigate",
    "arguments": {
      "session_id": "session-123",
      "url": "https://example.com"
    }
  }
}
```

**Meleys Response:**
Every successful tool call returns the universal `Observation` object nested inside the standard MCP content structure.

```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\"session_id\":\"session-123\",\"tab_id\":\"00000000-0000-0000-0000-000000000000\",\"action\":\"navigate\",\"success\":true,\"timestamp\":\"2026-07-17T11:00:10Z\",\"url\":\"https://example.com/\",\"title\":\"Example Domain\",\"result\":{\"type\":\"Empty\"},\"error\":null,\"console_messages\":[],\"network_summary\":null}"
      }
    ]
  }
}
```

---

## Available MCP Tools Catalog

Below is the complete list of tools exposed by the Meleys MCP server:

| Tool Name | Purpose | Required Arguments | Optional Arguments |
|-----------|---------|--------------------|--------------------|
| `create_session` | Launches a persistent browser session. | - | `profile_name`, `headless`, `default_search_engine`, `engine_preference` |
| `close_session` | Closes browser and cleans up processes. | `session_id` | - |
| `list_sessions` | Lists currently active browser instances. | - | - |
| `new_tab` | Opens a new page/tab. | `session_id` | `url` |
| `close_tab` | Closes a tab. | `session_id`, `tab_id` | - |
| `list_tabs` | Lists tabs in a browser session. | `session_id` | - |
| `switch_tab` | Changes the default active tab. | `session_id`, `tab_id` | - |
| `navigate` | Moves a tab to a new URL. | `session_id`, `url` | `tab_id`, `wait_until`, `timeout_ms` |
| `go_back` | Navigates back in history. | `session_id` | `tab_id`, `timeout_ms` |
| `go_forward` | Navigates forward in history. | `session_id` | `tab_id`, `timeout_ms` |
| `reload` | Reloads the current tab. | `session_id` | `tab_id`, `ignore_cache`, `timeout_ms` |
| `wait_for` | Waits for elements, network idle, or JS. | `session_id`, `condition` | `tab_id`, `selector`, `state`, `timeout_ms`, `js_expr`, `sleep_ms` |
| `click` | Performs a click event on an element. | `session_id`, `selector` | `tab_id`, `button`, `click_count`, `nth` |
| `type_text` | Types string into form inputs. | `session_id`, `selector`, `text` | `tab_id`, `clear_first`, `delay_ms` |
| `press_key` | Presses keyboard keys (e.g. Enter). | `session_id`, `key` | `tab_id`, `selector` |
| `hover` | Hovers mouse over elements. | `session_id`, `selector` | `tab_id` |
| `scroll` | Scrolls the viewport. | `session_id` | `tab_id`, `direction`, `amount_px`, `to_bottom` |
| `select_option` | Selects `<option>` value in dropdowns. | `session_id`, `selector`, `value` | `tab_id` |
| `set_file_input` | Adds files to upload inputs. | `session_id`, `selector`, `file_paths` | `tab_id` |
| `get_text` | Gets page or element text content. | `session_id` | `tab_id`, `selector`, `max_chars` |
| `get_links` | Retrieves links inside page scope. | `session_id` | `tab_id`, `selector`, `same_origin_only` |
| `get_dom` | Returns simplified DOM hierarchy. | `session_id` | `tab_id`, `selector`, `max_depth`, `include_hidden`, `max_nodes` |
| `get_ax_tree` | Accessibility tree structural extraction. | `session_id` | `tab_id`, `selector`, `max_depth` |
| `query_elements` | Queries elements by CSS selector. | `session_id`, `selector` | `tab_id`, `limit` |
| `evaluate_js` | Evaluates raw JS. | `session_id`, `expression` | `tab_id` |
| `download_file` | Initiates browser asset download. | `session_id`, `url` | `tab_id`, `save_as` |
| `list_downloads` | Retrieves download file registries. | `session_id` | - |
| `get_cookies` | Gets cookie jar details. | `session_id` | `tab_id`, `urls` |
| `set_cookies` | Injects cookies manually. | `session_id`, `cookies` | `tab_id` |
| `clear_cookies` | Empties the cookie jar. | `session_id` | `tab_id` |
| `get_local_storage` | Fetches session local storage keys. | `session_id` | `tab_id`, `origin` |
| `search_web` | Searches Google/Bing/DDG. | `session_id`, `query` | `tab_id`, `engine`, `num_results` |
| `set_default_search_engine`| Updates search fallback configuration. | `engine` | `session_id` |
| `get_default_search_engine`| Queries search fallback configuration. | - | `session_id` |

---

## Client Integration & Automatic Setup

To simplify integrating Meleys into your daily development workflow, Meleys supports both **automatic tool registration** for major coding agents and **manual configuration** where needed.

### 1. Automatic Integration (`meleys setup`)

Meleys can automatically register itself as a stdio MCP server for supported coding agents using the `setup` CLI command. It detects your installed agents, registers the tool path, and configures environment parameters.

```bash
# Register Meleys for all detected coding agents
meleys setup install
```

#### Supported Agents & Automated Config Files
When you run `meleys setup install`, it edits the following configurations:

| Agent | Configuration File | Automatic Actions |
|-------|--------------------|-------------------|
| **Claude Code** | `%USERPROFILE%\.claude.json` | Registers the `meleys` MCP tool. Denies built-in `WebSearch` and `WebFetch` in `settings.json` to force Claude to route web actions through Meleys. |
| **Cline** | `%USERPROFILE%\.cline\mcp.json` | Appends `meleys` under the `mcpServers` registry. |
| **Cursor** | `%USERPROFILE%\.cursor\mcp.json` | Appends `meleys` under the `mcpServers` registry. |
| **VS Code / Copilot** | `%APPDATA%\Code\User\settings.json` | Registers Meleys as a stdio tool server under `mcp.servers.meleys`. |

#### Manual setup commands:
- **Register for select agents only**: `meleys setup install --agents claude,cursor`
- **Avoid disabling agent's built-in search**: `meleys setup install --no-disable-builtin`
- **View registration status**: `meleys setup list`
- **Remove Meleys registration**: `meleys setup uninstall`

---

### 2. Manual Integration Examples

#### Claude Desktop

Claude Desktop configuration is not modified by `meleys setup` and must be configured manually.

Edit your `claude_desktop_config.json` configuration file:
- **Windows**: `%APPDATA%\Claude\claude_desktop_config.json`
- **macOS/Linux**: `~/Library/Application Support/Claude/claude_desktop_config.json`

Add the following entry under `mcpServers`:

```json
{
  "mcpServers": {
    "meleys": {
      "command": "meleys",
      "args": ["--mcp"],
      "env": {
        "MELEYS_LIMITS__ALLOW_EVALUATE_JS": "true"
      }
    }
  }
}
```
*(Ensure the `meleys` binary is on your system `PATH`, or replace `"meleys"` with the absolute path to your compiled binary).*

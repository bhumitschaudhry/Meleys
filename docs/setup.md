# Meleys Installation & Setup Guide

This guide covers everything you need to get the Meleys browser runtime running locally.

## Prerequisites

Before building Meleys, ensure you have the following installed on your system:

1. **Rust Toolchain**: Rust stable (2021 edition) is required. You can install it via [rustup](https://rustup.rs/):
   ```bash
   rustup default stable
   ```
2. **Chrome or Chromium**: Meleys uses a headless browser to perform navigation, interaction, and extraction.
   - **Windows**: Google Chrome or Microsoft Edge (usually auto-detected if installed in standard locations).
   - **macOS**: Google Chrome (installed in `/Applications/Google Chrome.app`).
   - **Linux**: Chrome or Chromium packages (e.g. `chromium-browser` or `google-chrome-stable` via your package manager).
   - *Note*: If you install Chrome/Chromium in a non-standard location, you must configure its path in `config.toml` or set the `MELEYS_BROWSER_EXECUTABLE_PATH` environment variable.

---

## Building from Source

Clone this repository and compile the project using Cargo:

```bash
# Build the binary in release mode
cargo build --release
```

The compiled binary will be located at `target/release/meleys` (or `target/release/meleys.exe` on Windows).

---

## Running Meleys

Meleys supports two primary transport modes: **HTTP API** (default) and **Model Context Protocol (MCP)**.

### 1. HTTP Mode (Default)

Running without any command-line flags starts the HTTP server:

```bash
./target/release/meleys
```

By default, the server will:
- Listen on `http://127.0.0.1:8787` (localhost only).
- Automatically detect the system Chrome/Chromium installation.
- Log server events to stdout (with `meleys=info` logging).

You can verify that the server is running by sending a request to the health endpoint:

```bash
curl -i http://localhost:8787/v1/health
```

Expected response:
```http
HTTP/1.1 200 OK
content-type: application/json

{"status":"ok"}
```

### 2. MCP Stdio Mode

If you are using Meleys to give a command-line LLM agent access to a browser, start Meleys in Model Context Protocol (MCP) mode:

```bash
./target/release/meleys --mcp
```

In this mode, Meleys communicates over stdio using JSON-RPC 2.0. This is suitable for subprocess integration with client agents (such as Claude Desktop or Cline).

---

## Quick Start Example (HTTP API)

Here is a simple sequence using `curl` and `jq` to create a session, navigate to a page, and fetch its text.

### Step 1: Create a Session
```bash
curl -s -X POST http://localhost:8787/v1/sessions \
  -H "Content-Type: application/json" \
  -d '{"profile_name": "quick-start", "headless": true}'
```

Response:
```json
{
  "session_id": "quick-start",
  "tab_id": "00000000-0000-0000-0000-000000000000",
  "action": "create_session",
  "success": true,
  "timestamp": "2026-07-17T11:00:00Z",
  "url": "about:blank",
  "title": "",
  "result": {
    "type": "Empty"
  },
  "error": null,
  "console_messages": [],
  "network_summary": null
}
```
*Note the returned `session_id` and the default `tab_id`.*

### Step 2: Navigate to a Website
```bash
curl -s -X POST http://localhost:8787/v1/sessions/quick-start/tabs/00000000-0000-0000-0000-000000000000/navigate \
  -H "Content-Type: application/json" \
  -d '{"url": "https://example.com"}'
```

Response:
```json
{
  "session_id": "quick-start",
  "tab_id": "00000000-0000-0000-0000-000000000000",
  "action": "navigate",
  "success": true,
  "timestamp": "2026-07-17T11:00:05Z",
  "url": "https://example.com/",
  "title": "Example Domain",
  "result": {
    "type": "Empty"
  },
  "error": null,
  "console_messages": [],
  "network_summary": null
}
```

### Step 3: Extract Page Text
```bash
curl -s -X POST http://localhost:8787/v1/sessions/quick-start/tabs/00000000-0000-0000-0000-000000000000/get_text \
  -H "Content-Type: application/json" \
  -d '{}'
```

Response:
```json
{
  "session_id": "quick-start",
  "tab_id": "00000000-0000-0000-0000-000000000000",
  "action": "get_text",
  "success": true,
  "timestamp": "2026-07-17T11:00:06Z",
  "url": "https://example.com/",
  "title": "Example Domain",
  "result": {
    "type": "Text",
    "data": "Example Domain\nThis domain is for use in illustrative examples in documents..."
  },
  "error": null,
  "console_messages": [],
  "network_summary": null
}
```

### Step 4: Close the Session
```bash
curl -s -X POST http://localhost:8787/v1/sessions/quick-start/close
```

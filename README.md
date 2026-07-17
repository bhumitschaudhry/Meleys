# Meleys — Agent-First Browser Runtime

Meleys is a lightweight local browser runtime in Rust that gives LLM agents control over a persistent, headless Chromium browser. 

It exposes core primitives (navigation, click, type, scroll) and returns structured observations (accessibility trees, DOM snapshots, search results) without complex external dependencies.

> [!IMPORTANT]
> **Meleys performs zero reasoning.** It is a deterministic execution boundary. All planning and decision-making live in the calling agent.

---

## 📚 Documentation

- **[Installation & Setup](docs/setup.md)**: System prerequisites, compilation, and setup.
- **[Configuration](docs/configuration.md)**: Options for `config.toml` and environment overrides.
- **[HTTP REST API Reference](docs/api.md)**: API endpoints, request schemas, and error codes.
- **[MCP Reference](docs/mcp.md)**: JSON-RPC stdio protocol details and client config (e.g., Claude Desktop).
- **[Architecture & Design](docs/architecture.md)**: Process isolation, thread safety, and internal modules.

---

## ⚡ Quick Start

### 1. Build
```bash
cargo build --release
```

### 2. Run
* **HTTP API Mode** (port `8787`):
  ```bash
  ./target/release/meleys
  ```
* **MCP Stdio Mode**:
  ```bash
  ./target/release/meleys --mcp
  ```

### 3. Agent Integration (Windows)
To automatically register Meleys as a browser backend for Claude Code, Cline, Cursor, and VS Code:
```powershell
powershell -ExecutionPolicy Bypass -File wix/build.ps1
```
*(Or run `meleys setup install` manually. See [Setup](docs/setup.md).)*

### 4. Test
```bash
curl http://localhost:8787/v1/health
```

---

## 🛠️ Selector Types

Target elements explicitly using structured selectors:

- **CSS**: `{"type": "Css", "value": "#search-input"}`
- **XPath**: `{"type": "XPath", "value": "//button[text()='Submit']"}`
- **Accessibility Tree**: `{"type": "AxNodeId", "value": "ax-node-5"}`
- **DOM Node ID**: `{"type": "BackendNodeId", "value": 104}`
- **Text Match**: `{"type": "Text", "value": {"exact": true, "value": "Sign In"}}`
- **Coordinates**: `{"type": "Coordinates", "value": {"x": 200.5, "y": 450.0}}`

---

## 🏛️ Architecture

```
+--------------------------------------------------------+
|                    Client / Agent                      |
+-------------------+--------------------+---------------+
                    |                    |
                    | MCP (stdio)        | HTTP (JSON)
+-------------------v--------------------v---------------+
|                   Transport Layer                      |
|  Exposes actions 1:1 across HTTP Router & MCP Tools   |
+---------------------------+----------------------------+
                            | Action Enum
                            v
+--------------------------------------------------------+
|                  Action Dispatcher                     |
|  Translates actions and routes to the Session Manager |
+---------------------------+----------------------------+
                            |
                            v
+--------------------------------------------------------+
|                   Session Manager                      |
|  Manages lifecycle, isolation, and process registry   |
+---------------------------+----------------------------+
                            |
                            v
+--------------------------------------------------------+
|                   Browser Session                      |
|  Owns single Chrome process (profile directory) & tabs |
+---------------------------+----------------------------+
                            | chromiumoxide (CDP)
                            v
+--------------------------------------------------------+
|                  Headless Chromium                     |
+--------------------------------------------------------+
```

---

## 🔒 Security

- **Local-Only**: Binds to `127.0.0.1` by default.
- **No JS Execution**: `evaluate_js` is disabled unless `limits.allow_evaluate_js = true` is set.
- **Containment**: Downloads are restricted to the session's folder; path traversal is blocked.
- **Stealth & Safety**: No built-in spoofing or filtering. Calling agents must manage browsing safety.

---

## 📄 License

Apache 2.0. See [LICENSE](LICENSE) for details.
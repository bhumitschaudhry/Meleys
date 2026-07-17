# Meleys — Agent-First Browser Runtime

Meleys is a lightweight, local browser runtime written in Rust that provides LLM-driven agents (and other calling programs) with direct, programmable hands on a real, persistent, headless Chromium browser. 

By exposing a minimal set of **mechanical primitives** (navigation, clicking, typing, scrolling) and returning highly structured, optimized **observations** (Accessibility trees, simplified DOM snapshots, search engine results), Meleys allows LLMs to interact with the dynamic web without complex dependencies.

> [!IMPORTANT]
> **The runtime performs zero reasoning.** It does not decide what to click, does not summarize content, and does not plan tasks. All intelligence lives in the calling agent; Meleys is a deterministic execution boundary.

---

## 📚 Documentation Catalog

For detailed setup, configuration, and developer references, see the following guides:

- **[Installation & Setup Guide](docs/setup.md)**: System prerequisites, compilation instructions, quick-start guide, and running modes.
- **[Configuration Guide](docs/configuration.md)**: Detailed configuration option lists for `config.toml` and environment variable overrides.
- **[HTTP REST API Reference](docs/api.md)**: Comprehensive endpoints map, JSON payloads request schemas, observation return properties, and error codes.
- **[Model Context Protocol (MCP) Reference](docs/mcp.md)**: Stdio JSON-RPC protocol guidelines, schema declarations, and client configurations (e.g. Claude Desktop).
- **[Architecture & Internal Design](docs/architecture.md)**: Structural layout, multi-session process isolation, thread safety, and DOM/AX tree simplifications.

---

## ⚡ Quick Start

### 1. Compile Meleys
Ensure you have [Rust](https://rustup.rs/) installed, then run:
```bash
cargo build --release
```

### 2. Start the Runtime
* **HTTP API Mode (default)**:
  ```bash
  ./target/release/meleys
  # Server listening on http://127.0.0.1:8787
  ```
* **Model Context Protocol (MCP) stdio Mode**:
  ```bash
  ./target/release/meleys --mcp
  ```

### 3. Basic Test
In HTTP mode, verify the server is running by querying the health check:
```bash
curl -i http://localhost:8787/v1/health
```

---

## 🛠️ Selector Types Reference

Meleys uses structured selectors so the calling agent can target elements explicitly:

```json
// CSS selector
{"type": "Css", "value": "#search-input"}

// XPath selector
{"type": "XPath", "value": "//button[text()='Submit']"}

// Accessibility Tree Node ID
{"type": "AxNodeId", "value": "ax-node-5"}

// Backend DOM Node ID
{"type": "BackendNodeId", "value": 104}

// Screen Text Match
{"type": "Text", "value": {"exact": true, "value": "Sign In"}}

// Bounding Box Coordinates
{"type": "Coordinates", "value": {"x": 200.5, "y": 450.0}}
```

---

## 🏛️ Architecture Overview

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

## 🔒 Security Summary

- **Local-Only**: Bindings default to `127.0.0.1` to prevent unauthorized remote control.
- **No Arbitrary JS Execution**: `evaluate_js` is disabled by default for security. It can be enabled via `limits.allow_evaluate_js = true`.
- **Containment**: File downloads are written to the session's isolated downloads folder. Path traversal is blocked.
- **Stealth and Safety**: Meleys does not perform content filtering or stealth spoofing. The calling agent is responsible for browsing safety and operator consent.

---

## 📄 License

Meleys is licensed under the Apache 2.0 License. See the `LICENSE` file for details.
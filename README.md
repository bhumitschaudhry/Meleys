# Meleys — Agent-First Browser Runtime

Meleys is a lightweight local browser runtime in Rust that gives LLM agents control over persistent, headless browser engines (Lightpanda and Chromium).

It exposes core browser primitives (navigation, click, type, scroll) and returns structured observations (accessibility trees, DOM snapshots, search results) optimized for LLM token usage.

> [!IMPORTANT]
> **Meleys performs zero reasoning.** It is a deterministic execution engine. All planning lives in the calling agent.

---

## Features

- **Multi-Engine**: High-speed JS execution with Lightpanda, full DOM compatibility with Chromium, and automatic fallback.
- **Agent-Optimized**: Native Accessibility Tree (`AXTree`) and simplified DOM extractions to eliminate token bloat.
- **Dual Transports**: Native MCP stdio server for AI tools (Claude Code, Cline, Cursor, VS Code) and HTTP REST API.

---

## Documentation

- **[Installation & Setup](docs/setup.md)** — Prerequisites, compilation, and quick start.
- **[Configuration](docs/configuration.md)** — `config.toml` options and environment variables.
- **[HTTP REST API Reference](docs/api.md)** — Endpoints, request schemas, and error codes.
- **[MCP Reference](docs/mcp.md)** — Tool definitions and stdio protocol setup for coding agents.
- **[Architecture & Design](docs/architecture.md)** — Process isolation, thread safety, and engine abstraction.

---

## Quick Start

### 1. Build & Run
```bash
cargo build --release

# HTTP REST API (default port 8787)
./target/release/meleys

# MCP Stdio Mode (for coding agents)
./target/release/meleys --mcp
```

### 2. Auto-Register for Coding Agents (Windows)
```powershell
powershell -ExecutionPolicy Bypass -File wix/build.ps1
```
*(Or run `meleys setup install` manually. See [Setup](docs/setup.md).)*

---

## Selector Types

Target elements using structured selectors:

- **CSS**: `{"type": "Css", "value": "#search-input"}`
- **XPath**: `{"type": "XPath", "value": "//button[text()='Submit']"}`
- **Accessibility Tree**: `{"type": "AxNodeId", "value": "ax-node-5"}`
- **DOM Node ID**: `{"type": "BackendNodeId", "value": 104}`
- **Text Match**: `{"type": "Text", "value": {"exact": true, "value": "Sign In"}}`
- **Coordinates**: `{"type": "Coordinates", "value": {"x": 200.5, "y": 450.0}}`

---

## Architecture

```
                 +-------------------+
                 |   Client / Agent  |
                 +---------+---------+
                           |
                     MCP / REST API
                           v
                 +-------------------+
                 |  Session Manager  |
                 | (Fallback Engine) |
                 +---------+---------+
                           | BrowserEngine Trait
                   +-------+-------+
                   |               |
                   v               v
            +--------------+ +--------------+
            |  Lightpanda  | |   Chromium   |
            | (Fast JS)    | | (Full DOM)   |
            +--------------+ +--------------+
```

---

## Security Defaults

- **Local-Only**: Binds to `127.0.0.1` by default.
- **JS Disabled**: `evaluate_js` is disabled unless `limits.allow_evaluate_js = true`.
- **Sandboxed**: File downloads are restricted to session directories; path traversal is blocked.

---

## License

[Apache 2.0](LICENSE)
# Architecture & Internal Design

This document details the architecture, module layout, and internal execution model of the Meleys browser runtime.

---

## High-Level Architecture Overview

Meleys acts as a deterministic bridge between an external LLM agent (or client process) and persistent browser engine instances. The core architecture is composed of several decoupled layers:

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
|                   Session Manager                      |
|  Manages lifecycle, fallback, and engine preferences   |
+---------------------------+----------------------------+
                            |
                            v
+--------------------------------------------------------+
|                   Browser Session                      |
|  Tracks active engine & serializes page handle actions |
+---------------------------+----------------------------+
                            | BrowserEngine Trait
            +---------------+---------------+
            |                               |
            v                               v
+-----------------------+       +-----------------------+
|   Lightpanda Engine   |       |    Chromium Engine    |
|   (Fast execution)    |       |   (Full web compat)   |
+-----------------------+       +-----------------------+
```

---

## Module Directory Structure

The crate is structured as follows:

- `src/main.rs`: Entry point. Resolves configuration, initializes logger (`tracing`), selects transport mode (HTTP server or MCP stdio), and registers signal handlers for graceful shutdown.
- `src/config.rs`: Figment configuration provider merging `config.toml`, environment variables (`MELEYS_*`), and default values.
- `src/error.rs`: Centralizes the `MeleyError` enum, providing structured error mappings to standard machine-readable codes.
- `src/observation.rs`: Defines the `Observation` payload, which is the universal response envelope returned by all operations.
- `src/selector.rs`: Resolves how elements are queried by converting CSS, XPath, AxNodeId, and Text selectors into evaluated JavaScript or CDP operations.
- `src/engine/`:
  - `mod.rs`: Defines the core `BrowserEngine` trait, engine preference types, and engine selection logic.
  - `lightpanda.rs`: Lightpanda engine implementation providing lightweight, high-performance execution.
  - `chromium.rs`: Chromium engine implementation wrapping `chromiumoxide` CDP connections.
  - `fallback.rs`: Implements automatic fallback handling when an engine fails or encounters unsupported operations.
- `src/transport/`:
  - `http.rs`: Axum-based web router map translating REST request payloads into engine calls.
  - `mcp.rs`: Stdio JSON-RPC 2.0 server executing browser tools inside client process environments.
- `src/actions/`:
  - `navigation.rs`: Implements tab-scoped actions like `navigate`, `go_back`, `go_forward`, `reload`, and `wait_for`.
  - `interaction.rs`: Simulates mouse/keyboard operations (`click`, `type_text`, `press_key`, `hover`, `scroll`).
  - `extraction.rs`: Retrieves text content, links, AX trees, and DOM snapshots.
  - `search.rs`: Adapts web search result parsing.
  - `download.rs`: Handles background files download and tracks file states.
  - `cookies.rs`: Direct cookie jar and localStorage reading/writing.
  - `tabs.rs`: Lifecycle actions for tab allocation.
  - `session.rs`: Handles creation, closure, and active sessions list with engine selection.
- `src/session/`:
  - `browser_session.rs`: The core struct holding engine CDP connections, registry of tabs, engine tracking, and process lifecycle.
  - `profile.rs`: Platform-specific profile directory path management.
- `src/setup/`:
  - `mod.rs`: Implements `meleys setup` CLI commands, handling automatic config detection, JSON/JSONC configuration editing, and custom permission settings for third-party coding agents.
- `src/cdp/`:
  - `launcher.rs`: Discovers system Chrome / Chromium and Lightpanda binaries and launches browser engine processes.
  - `dom.rs`: Flattens and filters the CDP DOM representation into a light `SimplifiedNode` hierarchy.
  - `ax_tree.rs`: Extracts the Accessibility tree for semantic LLM consumption.
- `wix/`:
  - `meleys.wxs`: WiX installer configuration mapping file copy actions, registry keys, PATH environment variables, and post-installation command execution hooks.
  - `build.ps1`: PowerShell script automating compilation version reading, candle compilation, and light linking for MSI production.

---

## Concurrency & Thread-Safety Model

To ensure browser commands execute in a stable, crash-free environment:

1. **Session Isolation**: Each session maps to exactly one operating system Chromium subprocess and has its own `--user-data-dir`. The cookie jars, storage, cache, and state are isolated.
2. **Global Session Map**: `SessionManager` stores active sessions within a thread-safe map: `Arc<RwLock<HashMap<String, Arc<BrowserSession>>>>`.
3. **Tab Serialization**: Multiple operations on different tabs run concurrently. However, operations targetting the *same tab* must not execute out-of-order. Meleys serializes action execution per-tab using a `tokio::sync::Mutex` protecting the active page handles.

---

## CDP DOM & Accessibility Tree Extraction

Directly reading raw HTML is inefficient for LLMs due to the verbosity of boilerplate tags and script contents. Meleys solves this through two custom extractors:

### 1. Simplified DOM (`cdp/dom.rs`)
Iterates over node descriptors returned by the Chrome DevTools Protocol and constructs a cleaned-up `SimplifiedNode` tree. It:
- Strips `<script>`, `<style>`, and `<svg>` nodes.
- Discards elements with no visible content, text, or interactive attributes.
- Filters attributes, preserving only functional properties like `id`, `class`, `href`, `role`, `aria-*`, `name`, `value`, `type`.
- Computes element bounding boxes for coordinate-based click overrides.
- Caps tree traversal at `max_depth` (default: 6) and node counts to avoid token inflation.

### 2. Accessibility Tree (`cdp/ax_tree.rs`)
Extracts the browser's Accessibility tree (`AXTree`), which matches how screen readers interpret the web page. This yields a flattened, semantic representation where elements are described by their screen reader role (e.g. `button`, `heading`, `link`) and accessible name (e.g. "Submit"). This is the recommended extraction structure for LLM navigation.

---

## Search Engine Adapters (`actions/search.rs`)

To support web searching without reliance on paid APIs, Meleys utilizes direct parsing:
1. It navigates a browser page to Google, Bing, or DuckDuckGo.
2. It waits for the result element container to render.
3. It runs a hand-maintained CSS parser (implementing `SearchEngineAdapter`) against the DOM structure to extract clean result titles, URLs, and snippets.
4. If an engine's HTML layout drifts and parsing fails, it yields a `SEARCH_ENGINE_PARSE_FAILED` error, signaling the client agent to fall back to navigating the page directly and using standard DOM text extraction.

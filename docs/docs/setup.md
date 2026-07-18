---
id: setup
title: Installation & Setup
sidebar_position: 2
---

# Installation & Setup Guide

## Prerequisites

1. **Rust Toolchain**: Rust stable (2021 edition). Install via [rustup](https://rustup.rs/):
   ```bash
   rustup default stable
   ```
2. **Browser Engine**:
   - **Lightpanda**: Lightweight JS engine (auto-discovered on `PATH` or configured in `config.toml`).
   - **Chrome / Chromium**: Full browser engine (auto-detected on standard OS paths).

---

## Building & Running

### 1. Build from Source
```bash
cargo build --release
```
Binary location: `target/release/meleys` (or `meleys.exe` on Windows).

### 2. Run Modes
- **HTTP REST Server** (default port `8787`):
  ```bash
  ./target/release/meleys
  ```
- **MCP Stdio Server** (for LLM agents):
  ```bash
  ./target/release/meleys --mcp
  ```

### 3. Verify Server Health
```bash
curl http://localhost:8787/v1/health
```

---

## Quick Start (HTTP API)

```bash
# 1. Create session
curl -s -X POST http://localhost:8787/v1/sessions \
  -H "Content-Type: application/json" \
  -d '{"profile_name": "quick-start", "headless": true}'

# 2. Navigate
curl -s -X POST http://localhost:8787/v1/sessions/quick-start/tabs/<TAB_ID>/navigate \
  -H "Content-Type: application/json" \
  -d '{"url": "https://example.com"}'

# 3. Extract text
curl -s -X POST http://localhost:8787/v1/sessions/quick-start/tabs/<TAB_ID>/get_text \
  -H "Content-Type: application/json" -d '{}'

# 4. Close session
curl -s -X POST http://localhost:8787/v1/sessions/quick-start/close
```

---

## Coding Agent Integration (`meleys setup`)

Meleys can automatically register itself as a stdio MCP server for supported coding agents:

| Agent | Config File |
|-------|-------------|
| **Claude Code** | `%USERPROFILE%\.claude.json` + denies built-in `WebSearch`/`WebFetch` |
| **Cline** | `%USERPROFILE%\.cline\mcp.json` |
| **Cursor** | `%USERPROFILE%\.cursor\mcp.json` |
| **VS Code** | `%APPDATA%\Code\User\settings.json` |

### CLI Commands
```bash
# Register for all detected agents
meleys setup install

# Register specific agents without disabling built-in search
meleys setup install --agents claude,cursor --no-disable-builtin

# View registration status
meleys setup list

# Remove Meleys from agent configs
meleys setup uninstall
```

### Windows MSI Installer
Build the `.msi` package (requires WiX Toolset):
```powershell
powershell -ExecutionPolicy Bypass -File wix/build.ps1
```
The installer automatically runs `meleys setup install` post-installation.

# Configuration Guide

Meleys uses a layered configuration system powered by Figment, loading values from:
1. Internal defaults.
2. A `config.toml` file in the working directory (if present).
3. Environment variables prefixed with `MELEYS_`.

---

## Configuration File (`config.toml`)

Create a `config.toml` file in the directory from which you run Meleys to configure its behavior. Below is a fully commented example displaying all options and their defaults.

```toml
# ==========================================
# Server Settings
# ==========================================
[server]
# Port on which the HTTP server will listen.
http_port = 8787

# IP address or hostname to bind the HTTP server to.
# For security reasons, this defaults to localhost only (127.0.0.1).
http_bind = "127.0.0.1"

# The Model Context Protocol transport type ("stdio" or "sse").
# Stdio is default and recommended for local agent integrations.
mcp_transport = "stdio"

# ==========================================
# Browser Settings
# ==========================================
[browser]
# Path to the Chrome/Chromium executable. 
# Leave empty to automatically detect Chrome/Chromium on your system path.
executable_path = ""

# Whether to launch Chrome in headless mode. Set to false to see the browser window.
headless = true

# The default viewport dimensions for new tabs/pages.
default_viewport = { width = 1280, height = 800 }

# Directory where browser profiles (user data directories) are saved.
# This ensures cookie/login state persistence across agent sessions.
# Default: platform-specific local data directory (e.g. ~/.local/share/meleys/profiles).
profile_dir = "~/.local/share/meleys/profiles"

# ==========================================
# Search Settings
# ==========================================
[search]
# Default search engine to use for search_web calls when no engine parameter is specified.
# Supported engines: "google", "bing", "duckduckgo".
default_engine = "duckduckgo"

# ==========================================
# Limits & Security Policies
# ==========================================
[limits]
# Maximum number of concurrent active browser sessions allowed.
max_sessions = 8

# Default timeout in milliseconds for browser actions (navigation, clicks, etc.)
default_action_timeout_ms = 30000

# Maximum number of DOM nodes to return when calling get_dom.
# Prevents context exhaustion in LLM agents with very large pages.
max_dom_nodes_per_call = 2000

# Allow evaluate_js action. This is disabled by default for security, 
# preventing arbitrary JavaScript execution unless explicitly enabled.
allow_evaluate_js = false

# ==========================================
# Downloads Settings
# ==========================================
[downloads]
# Default path where downloaded files are saved.
# Default: platform-specific local data directory (e.g. ~/.local/share/meleys/downloads).
dir = "~/.local/share/meleys/downloads"

# Allow-list of directories to which files can be downloaded.
# If empty, files can only be saved to the default downloads directory.
allowed_save_dirs = []
```

---

## Environment Variables Override

Any configuration key can be overridden using environment variables prefixed with `MELEYS_`. Use double underscores (`__`) to delimit nested sections.

### Examples

- Override the HTTP server port:
  ```bash
  MELEYS_SERVER__HTTP_PORT=9000 ./target/release/meleys
  ```
- Enable JavaScript evaluation dynamically:
  ```bash
  MELEYS_LIMITS__ALLOW_EVALUATE_JS=true ./target/release/meleys
  ```
- Change the browser executable path:
  ```bash
  MELEYS_BROWSER__EXECUTABLE_PATH="/usr/bin/google-chrome-stable" ./target/release/meleys
  ```

---

## Resolution Precedence

Configuration parameters are resolved in the following priority order:

1. **Environment Variables**: E.g. `MELEYS_SERVER__HTTP_PORT` (highest priority).
2. **Configuration File**: Values defined in `config.toml`.
3. **Internal Defaults**: Hardcoded values used if neither of the above are provided.

# HTTP REST API Reference

Meleys exposes a standard REST HTTP API. Every operation (except health check) is structured around performing an **Action** on a persistent **Session** or **Tab** and returning a structured **Observation**.

---

## Universal Data Models

### 1. The `Observation` Object

All successful and failed actions return an HTTP `200 OK` with an `Observation` JSON response. If an action fails, `success` will be `false` and the `error` object will be populated.

```json
{
  "session_id": "string",
  "tab_id": "string",
  "action": "string",
  "success": true,
  "timestamp": "2026-07-17T11:00:00Z",
  "url": "string (current URL after action)",
  "title": "string (current page title after action)",
  "status_code": 200,
  "result": {
    "type": "ActionResultVariant",
    "data": {}
  },
  "error": null,
  "console_messages": [
    {
      "level": "info|warning|error",
      "text": "string",
      "timestamp": "string"
    }
  ],
  "network_summary": {
    "requests": 15,
    "failed": 0,
    "bytes_received": 102450
  }
}
```

### 2. Error Info (`Observation.error`)

When `success` is `false`, the `error` field contains:

```json
{
  "code": "ERROR_CODE",
  "message": "Human-readable error description.",
  "retryable": true
}
```

#### Error Codes Reference
| Code | Meaning |
|------|---------|
| `SESSION_NOT_FOUND` | The specified session ID does not exist or has been closed. |
| `TAB_NOT_FOUND` | The specified tab ID does not exist in this session. |
| `ELEMENT_NOT_FOUND` | The selector matched no elements on the page. |
| `ELEMENT_NOT_INTERACTABLE` | Element was found but could not be clicked or typed into. |
| `TIMEOUT` | The action exceeded the default or requested timeout. |
| `NAVIGATION_FAILED` | Chrome DevTools Protocol failed to navigate to the URL. |
| `INVALID_SELECTOR` | The selector object was malformed or unsupported. |
| `SEARCH_ENGINE_PARSE_FAILED` | Meleys could not extract structured search results from the engine's page. |
| `DOWNLOAD_FAILED` | File download failed to start or complete. |
| `CDP_CONNECTION_LOST` | The connection to the Chrome browser instance was severed. |
| `JS_EVAL_DISABLED` | `evaluate_js` was called but JS execution is disabled in limits. |
| `INTERNAL_ERROR` | An unexpected runtime or system error occurred. |

### 3. Selectors

Many interaction and extraction actions take a `selector` object. The `selector` object has a `type` parameter which dictates how the target element is identified:

- **CSS Selector**:
  ```json
  { "type": "Css", "value": "button.submit-btn" }
  ```
- **XPath Selector**:
  ```json
  { "type": "XPath", "value": "//div[contains(text(), 'Submit')]" }
  ```
- **Accessibility Tree Node ID**:
  ```json
  { "type": "AxNodeId", "value": "ax-node-123" }
  ```
- **Backend Node ID** (DOM Node ID):
  ```json
  { "type": "BackendNodeId", "value": 42 }
  ```
- **Visible Text Match**:
  ```json
  { "type": "Text", "value": { "exact": true, "value": "Click Me" } }
  ```
- **Pixel Coordinates**:
  ```json
  { "type": "Coordinates", "value": { "x": 150.5, "y": 300.0 } }
  ```

---

## Endpoint Catalog

### Session & Tab Management

#### Create Session
* **Route**: `POST /v1/sessions`
* **Request**:
  ```json
  {
    "profile_name": "string (optional, for persistent cookies/storage)",
    "headless": true,
    "default_search_engine": "duckduckgo|google|bing"
  }
  ```
* **Result Type**: `Empty` (The session ID is returned in the root `session_id` property).

#### List Sessions
* **Route**: `GET /v1/sessions`
* **Result Type**: `Sessions`
* **Result Data**:
  ```json
  [
    {
      "session_id": "my-profile",
      "profile_path": "/path/to/profiles/my-profile",
      "created_at": "2026-07-17T11:00:00Z",
      "tab_count": 1,
      "default_search_engine": "duckduckgo"
    }
  ]
  ```

#### Close Session
* **Route**: `POST /v1/sessions/:session_id/close` (or `POST /v1/sessions/:session_id`)
* **Result Type**: `Empty`

#### List Tabs
* **Route**: `GET /v1/sessions/:session_id/tabs`
* **Result Type**: `Tabs`
* **Result Data**:
  ```json
  [
    {
      "tab_id": "00000000-0000-0000-0000-000000000000",
      "url": "https://example.com/",
      "title": "Example Domain",
      "is_active": true,
      "loading": false
    }
  ]
  ```

#### Create Tab
* **Route**: `POST /v1/sessions/:session_id/tabs`
* **Request**:
  ```json
  {
    "url": "string (optional, starts navigation immediately)"
  }
  ```
* **Result Type**: `Tabs` (A list of tabs, containing the newly created tab).

#### Close Tab
* **Route**: `POST /v1/sessions/:session_id/tabs/:tab_id/close`
* **Result Type**: `Empty`

#### Switch Tab
* **Route**: `POST /v1/sessions/:session_id/tabs/:tab_id/switch`
* **Result Type**: `Tabs` (List of tabs indicating the new active tab).

---

### Navigation

#### Navigate
* **Route**: `POST /v1/sessions/:sid/tabs/:tid/navigate`
* **Request**:
  ```json
  {
    "url": "https://example.com",
    "wait_until": "load|domcontentloaded|networkidle",
    "timeout_ms": 30000
  }
  ```
* **Result Type**: `Empty`

#### Go Back
* **Route**: `POST /v1/sessions/:sid/tabs/:tid/go_back`
* **Request**:
  ```json
  {
    "timeout_ms": 30000
  }
  ```
* **Result Type**: `Empty`

#### Go Forward
* **Route**: `POST /v1/sessions/:sid/tabs/:tid/go_forward`
* **Request**:
  ```json
  {
    "timeout_ms": 30000
  }
  ```
* **Result Type**: `Empty`

#### Reload
* **Route**: `POST /v1/sessions/:sid/tabs/:tid/reload`
* **Request**:
  ```json
  {
    "ignore_cache": false,
    "timeout_ms": 30000
  }
  ```
* **Result Type**: `Empty`

#### Wait For
* **Route**: `POST /v1/sessions/:sid/tabs/:tid/wait_for`
* **Request**:
  ```json
  {
    "condition": "selector|navigation|networkidle|timeout|js_expression",
    "selector": "string (used with selector condition)",
    "state": "visible|hidden|attached|detached",
    "timeout_ms": 10000,
    "idle_ms": 500,
    "js_expr": "window.someFlag === true",
    "poll_ms": 100,
    "sleep_ms": 1000
  }
  ```
* **Result Type**: `Empty`

---

### Interaction

#### Click
* **Route**: `POST /v1/sessions/:sid/tabs/:tid/click`
* **Request**:
  ```json
  {
    "selector": { "type": "Css", "value": "a.link" },
    "button": "left|right|middle",
    "click_count": 1,
    "nth": 0,
    "timeout_ms": 30000
  }
  ```
* **Result Type**: `Empty`

#### Type Text
* **Route**: `POST /v1/sessions/:sid/tabs/:tid/type_text`
* **Request**:
  ```json
  {
    "selector": { "type": "Css", "value": "input[type='text']" },
    "text": "Hello World",
    "clear_first": false,
    "delay_ms": 0,
    "timeout_ms": 30000
  }
  ```
* **Result Type**: `Empty`

#### Press Key
* **Route**: `POST /v1/sessions/:sid/tabs/:tid/press_key`
* **Request**:
  ```json
  {
    "key": "Enter|Escape|Tab|Backspace",
    "selector": { "type": "Css", "value": "body" },
    "timeout_ms": 30000
  }
  ```
* **Result Type**: `Empty`

#### Hover
* **Route**: `POST /v1/sessions/:sid/tabs/:tid/hover`
* **Request**:
  ```json
  {
    "selector": { "type": "Css", "value": ".menu-item" },
    "timeout_ms": 30000
  }
  ```
* **Result Type**: `Empty`

#### Scroll
* **Route**: `POST /v1/sessions/:sid/tabs/:tid/scroll`
* **Request**:
  ```json
  {
    "direction": "up|down|left|right",
    "amount_px": 500,
    "selector": { "type": "Css", "value": ".scrollable-div" },
    "to_bottom": false,
    "timeout_ms": 30000
  }
  ```
* **Result Type**: `Empty`

#### Select Option
* **Route**: `POST /v1/sessions/:sid/tabs/:tid/select_option`
* **Request**:
  ```json
  {
    "selector": { "type": "Css", "value": "select#country" },
    "value": "US",
    "timeout_ms": 30000
  }
  ```
* **Result Type**: `Empty`

#### Set File Input
* **Route**: `POST /v1/sessions/:sid/tabs/:tid/set_file_input`
* **Request**:
  ```json
  {
    "selector": { "type": "Css", "value": "input[type='file']" },
    "file_paths": ["/absolute/path/to/file.txt"],
    "timeout_ms": 30000
  }
  ```
* **Result Type**: `Empty`

---

### Extraction

#### Get Text
* **Route**: `POST /v1/sessions/:sid/tabs/:tid/get_text`
* **Request**:
  ```json
  {
    "selector": { "type": "Css", "value": "body" },
    "max_chars": 20000
  }
  ```
* **Result Type**: `Text`
* **Result Data**: `"string of text"`

#### Get Links
* **Route**: `POST /v1/sessions/:sid/tabs/:tid/get_links`
* **Request**:
  ```json
  {
    "selector": ".content-area",
    "same_origin_only": false
  }
  ```
* **Result Type**: `Links`
* **Result Data**:
  ```json
  [
    {
      "href": "https://example.com/about",
      "text": "About Us",
      "visible": true
    }
  ]
  ```

#### Get DOM
* **Route**: `POST /v1/sessions/:sid/tabs/:tid/get_dom`
* **Request**:
  ```json
  {
    "selector": { "type": "Css", "value": "body" },
    "max_depth": 6,
    "include_hidden": false,
    "max_nodes": 2000
  }
  ```
* **Result Type**: `Dom`
* **Result Data**:
  ```json
  {
    "backend_node_id": 4,
    "tag": "div",
    "attributes": {
      "id": "container",
      "class": "main-content"
    },
    "text": "Hello",
    "visible": true,
    "bounding_box": {
      "x": 0.0,
      "y": 0.0,
      "width": 100.0,
      "height": 50.0
    },
    "children": []
  }
  ```

#### Get Accessibility Tree
* **Route**: `POST /v1/sessions/:sid/tabs/:tid/get_ax_tree`
* **Request**:
  ```json
  {
    "selector": { "type": "Css", "value": "body" },
    "max_depth": 8
  }
  ```
* **Result Type**: `AxTree`
* **Result Data**:
  ```json
  {
    "ax_node_id": "ax-node-1",
    "role": "RootWebArea",
    "name": "Example Page",
    "value": null,
    "focusable": true,
    "focused": false,
    "disabled": false,
    "children": []
  }
  ```

#### Query Elements
* **Route**: `POST /v1/sessions/:sid/tabs/:tid/query_elements`
* **Request**:
  ```json
  {
    "selector": { "type": "Css", "value": "input.form-control" },
    "limit": 50
  }
  ```
* **Result Type**: `Elements`
* **Result Data**:
  ```json
  [
    {
      "backend_node_id": 35,
      "tag": "input",
      "text": "",
      "attributes": { "type": "text", "name": "username" },
      "bounding_box": { "x": 10.0, "y": 20.0, "width": 200.0, "height": 30.0 },
      "visible": true
    }
  ]
  ```

#### Evaluate JavaScript
* **Route**: `POST /v1/sessions/:sid/tabs/:tid/evaluate_js`
* **Request**:
  ```json
  {
    "expression": "window.location.hostname"
  }
  ```
* **Result Type**: `Text`
* **Result Data**: `"\"example.com\""`
* *Note: Only functions if limits.allow_evaluate_js = true.*

---

### Capture & Downloads

#### Take Screenshot
* **Route**: `POST /v1/sessions/:sid/tabs/:tid/screenshot`
* **Request**:
  ```json
  {
    "selector": { "type": "Css", "value": "div#chart" },
    "full_page": false,
    "format": "png|jpeg",
    "timeout_ms": 30000
  }
  ```
* **Result Type**: `Screenshot`
* **Result Data**:
  ```json
  {
    "format": "png",
    "base64": "iVBORw0KGgo...",
    "width": 800,
    "height": 600
  }
  ```

#### Export PDF
* **Route**: `POST /v1/sessions/:sid/tabs/:tid/export_pdf`
* **Request**:
  ```json
  {
    "landscape": false,
    "timeout_ms": 30000
  }
  ```
* **Result Type**: `Download`
* **Result Data**:
  ```json
  {
    "id": "pdf-export-uuid",
    "url": "pdf_export://local",
    "path": "/path/to/downloads/pdf_export_123.pdf",
    "size_bytes": 102400,
    "state": "completed",
    "started_at": "2026-07-17T11:00:00Z",
    "completed_at": "2026-07-17T11:00:01Z"
  }
  ```

#### Download File
* **Route**: `POST /v1/sessions/:sid/tabs/:tid/download_file`
* **Request**:
  ```json
  {
    "url": "https://example.com/data.csv",
    "save_as": "data.csv",
    "timeout_ms": 60000
  }
  ```
* **Result Type**: `Download`
* **Result Data**:
  ```json
  {
    "id": "download-uuid",
    "url": "https://example.com/data.csv",
    "path": "/path/to/downloads/data.csv",
    "size_bytes": 450,
    "state": "completed|in_progress|failed",
    "started_at": "2026-07-17T11:00:00Z",
    "completed_at": "2026-07-17T11:00:02Z"
  }
  ```

#### List Downloads
* **Route**: `GET /v1/sessions/:session_id/downloads`
* **Result Type**: `Download` (returns an array of completed/in-progress downloads)

---

### Cookies & Local Storage

#### Get Cookies
* **Route**: `GET /v1/sessions/:sid/tabs/:tid/cookies`
* **Request** (Passed as request body):
  ```json
  {
    "urls": ["https://example.com"]
  }
  ```
* **Result Type**: `Cookies`
* **Result Data**:
  ```json
  [
    {
      "name": "session_token",
      "value": "xyz123",
      "domain": "example.com",
      "path": "/",
      "secure": true,
      "http_only": true,
      "same_site": "Lax",
      "expires": 1783220000.0
    }
  ]
  ```

#### Set Cookies
* **Route**: `POST /v1/sessions/:sid/tabs/:tid/cookies`
* **Request**:
  ```json
  {
    "cookies": [
      {
        "name": "my_cookie",
        "value": "123",
        "domain": "example.com",
        "path": "/",
        "secure": false,
        "http_only": false
      }
    ]
  }
  ```
* **Result Type**: `Empty`

#### Clear Cookies
* **Route**: `DELETE /v1/sessions/:sid/tabs/:tid/cookies`
* **Result Type**: `Empty`

#### Get Local Storage
* **Route**: `GET /v1/sessions/:sid/tabs/:tid/local_storage`
* **Request** (Passed as request body):
  ```json
  {
    "origin": "https://example.com"
  }
  ```
* **Result Type**: `Text`
* **Result Data**: `"{\"theme\":\"dark\",\"user_id\":\"42\"}"`

---

### Search Engine Settings

#### Search Web
* **Route**: `POST /v1/sessions/:sid/tabs/:tid/search_web`
* **Request**:
  ```json
  {
    "query": "Rust web scraping",
    "engine": "google|bing|duckduckgo",
    "num_results": 10
  }
  ```
* **Result Type**: `SearchResults`
* **Result Data**:
  ```json
  [
    {
      "rank": 1,
      "title": "Rust web scraping in 2026",
      "url": "https://example.org/blog/rust-scraping",
      "snippet": "A detailed guide about compiling and writing scraping programs..."
    }
  ]
  ```

#### Get Runtime Default Engine
* **Route**: `GET /v1/search_engine`
* **Result Type**: `SearchEngine`
* **Result Data**: `{ "engine": "duckduckgo", "scope": "runtime" }`

#### Set Runtime Default Engine
* **Route**: `POST /v1/search_engine`
* **Request**: `{ "engine": "google" }`
* **Result Type**: `Empty`

#### Get Session Default Engine
* **Route**: `GET /v1/sessions/:session_id/search_engine`
* **Result Type**: `SearchEngine`
* **Result Data**: `{ "engine": "google", "scope": "session" }`

#### Set Session Default Engine
* **Route**: `POST /v1/sessions/:session_id/search_engine`
* **Request**: `{ "engine": "bing" }`
* **Result Type**: `Empty`

---

### Health Check

#### Health
* **Route**: `GET /v1/health`
* **Response**:
  ```json
  {
    "status": "ok"
  }
  ```

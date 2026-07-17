You are implementing Task 9 of a multi-engine architecture for Meleys (a Rust browser runtime).

**Project context:** Meleys is at E:\Meleys. Tasks 1-8 created engine types, errors, config, observation fields, engines, fallback logic, and added engine tracking to BrowserSession. Now we need to refactor SessionManager to use the engine selection.

**Task 9: Refactor SessionManager**

Modify `src/session/mod.rs` to use `select_engine` from the fallback module.

**Current state of `session/mod.rs`:**
- `SessionManager::create_session` takes `profile_name`, `headless`, `default_search_engine`
- It calls `crate::cdp::launcher::launch_browser` directly
- It creates `BrowserSession::new(session_id, profile, launch_result.browser, launch_result.handler_task)`

**Changes to make:**

1. Add import:
```rust
use crate::engine::EnginePreference;
```

2. Update `create_session` signature to accept optional engine preference:
```rust
pub async fn create_session(
    &self,
    profile_name: Option<String>,
    headless: Option<bool>,
    default_search_engine: Option<String>,
    engine_preference: Option<EnginePreference>,
) -> Result<Arc<BrowserSession>> {
```

3. Inside `create_session`, before launching browser, call `select_engine`:
```rust
// Select engine based on preference
let preference = engine_preference.unwrap_or_default();
let engine_selection = crate::engine::fallback::select_engine(preference, &self.config.engine)?;
```

4. Pass `engine_selection.engine_kind` to `BrowserSession::new`:
```rust
let session = Arc::new(BrowserSession::new(
    session_id.clone(),
    profile,
    launch_result.browser,
    launch_result.handler_task,
    engine_selection.engine_kind,
));
```

5. Set the engine history on the session:
```rust
let mut session_mut = Arc::try_unwrap(session).unwrap_or_else(|arc| (*arc).clone());
session_mut.engine_history = engine_selection.engine_history;
let session = Arc::new(session_mut);
```

Actually, since `session` is created as `Arc::new(...)`, we can't mutate it after creation. Instead, modify `BrowserSession::new` to accept `engine_history` as a parameter, or set it before wrapping in Arc.

Better approach: Update `BrowserSession::new` to also accept `engine_history`:
```rust
pub fn new(
    session_id: String,
    profile: Profile,
    browser: Browser,
    handler_task: tokio::task::JoinHandle<()>,
    engine_kind: EngineKind,
    engine_history: Vec<EngineTransition>,
) -> Self {
```

6. Update all callers of `create_session` to pass `None` for the new parameter (HTTP handlers, MCP dispatch, tests).

7. Update the `list_sessions` method to include `engine_kind` in `SessionInfo` if needed (or leave for now).

8. Run `cargo test --lib session` to verify
9. Commit: `feat(session): SessionManager uses engine selection with fallback`

**Important:** The `select_engine` function checks if the Lightpanda binary exists. If it doesn't and fallback is enabled, it selects Chromium. The actual browser launch still uses `cdp::launcher::launch_browser` (Chromium). The Lightpanda process spawn will be wired in a future task.

Return: status, commits, test results, concerns.

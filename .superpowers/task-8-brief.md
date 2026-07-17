You are implementing Task 8 of a multi-engine architecture for Meleys (a Rust browser runtime).

**Project context:** Meleys is at E:\Meleys. Tasks 1-7 created engine types, errors, config, observation fields, engines, and added engine tracking to BrowserSession. Now we need fallback orchestration.

**Task 8: Fallback Orchestration**

Replace the placeholder `src/engine/fallback.rs` with real fallback logic.

**Current state:** `src/engine/fallback.rs` is a placeholder with `// placeholder`

**Implementation approach:**

Create an `EngineSelection` struct and `select_engine` function that determines which engine to use based on preference and config:

```rust
use crate::config::EngineConfig;
use crate::engine::{EngineKind, EnginePreference, EngineTransition};
use crate::error::MeleyError;

pub struct EngineSelection {
    pub engine_kind: EngineKind,
    pub engine_history: Vec<EngineTransition>,
}

pub fn select_engine(
    preference: EnginePreference,
    config: &EngineConfig,
) -> Result<EngineSelection, MeleyError> {
    let mut history = Vec::new();

    match preference {
        EnginePreference::LightpandaOnly => {
            // Verify Lightpanda binary exists
            let binary_path = config.lightpanda.binary_path.as_str();
            if crate::cdp::launcher::find_lightpanda_binary(binary_path).is_none() {
                return Err(MeleyError::EngineStartupFailed(
                    "Lightpanda binary not found and fallback disabled".into(),
                ));
            }
            Ok(EngineSelection {
                engine_kind: EngineKind::Lightpanda,
                engine_history: history,
            })
        }
        EnginePreference::ChromiumOnly => {
            Ok(EngineSelection {
                engine_kind: EngineKind::Chromium,
                engine_history: history,
            })
        }
        EnginePreference::LightpandaWithFallback => {
            // Try Lightpanda first
            let binary_path = config.lightpanda.binary_path.as_str();
            if crate::cdp::launcher::find_lightpanda_binary(binary_path).is_some() {
                Ok(EngineSelection {
                    engine_kind: EngineKind::Lightpanda,
                    engine_history: history,
                })
            } else if config.fallback_enabled {
                // Fallback to Chromium
                tracing::warn!(
                    "Lightpanda binary not found, falling back to Chromium"
                );
                history.push(EngineTransition {
                    from: EngineKind::Lightpanda,
                    to: EngineKind::Chromium,
                    reason: "Lightpanda binary not found".into(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                });
                Ok(EngineSelection {
                    engine_kind: EngineKind::Chromium,
                    engine_history: history,
                })
            } else {
                Err(MeleyError::EngineStartupFailed(
                    "Lightpanda binary not found and fallback disabled".into(),
                ))
            }
        }
    }
}
```

**Steps:**
1. Read `src/engine/fallback.rs` to see current placeholder
2. Replace with the implementation above
3. Run `cargo check` to verify compilation
4. Commit: `feat(engine): add engine selection with fallback logic`

Return: status, commits, test results, concerns.

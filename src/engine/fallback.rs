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
            let binary_path = config.lightpanda.binary_path.as_str();
            if crate::cdp::launcher::find_lightpanda_binary(binary_path).is_some() {
                Ok(EngineSelection {
                    engine_kind: EngineKind::Lightpanda,
                    engine_history: history,
                })
            } else if config.fallback_enabled {
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

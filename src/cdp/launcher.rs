use anyhow::Result;
use chromiumoxide::handler::viewport::Viewport;
use chromiumoxide::{Browser, BrowserConfig};
use futures::StreamExt;
use tokio::task::JoinHandle;

use crate::config::BrowserConfig as MeleysBrowserConfig;

/// Launch result containing the Browser instance and the handler task.
pub struct LaunchResult {
    pub browser: Browser,
    pub handler_task: JoinHandle<()>,
}

/// Launch a Chromium browser instance based on the given config.
pub async fn launch_browser(
    config: &MeleysBrowserConfig,
    user_data_dir: Option<&str>,
) -> Result<LaunchResult> {
    let mut builder = BrowserConfig::builder()
        .no_sandbox()
        .arg("--disable-dev-shm-usage")
        .arg("--disable-gpu")
        .arg("--disable-background-timer-throttling")
        .arg("--disable-renderer-backgrounding");

    // Set headless mode
    if config.headless {
        builder = builder.new_headless_mode();
    } else {
        builder = builder.with_head();
    }

    // Set executable path if configured
    if !config.executable_path.is_empty() {
        builder = builder.chrome_executable(&config.executable_path);
    } else if let Some(binary) = find_browser_binary() {
        builder = builder.chrome_executable(&binary);
    }

    // Set viewport size using chromiumoxide's Viewport type
    let viewport = Viewport {
        width: config.default_viewport.width,
        height: config.default_viewport.height,
        device_scale_factor: None,
        emulating_mobile: false,
        is_landscape: false,
        has_touch: false,
    };
    builder = builder.viewport(viewport);

    // Add user-data-dir if specified
    if let Some(udd) = user_data_dir {
        builder = builder.user_data_dir(udd);
    }

    let browser_config = builder
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build browser config: {}", e))?;

    let (browser, mut handler) = Browser::launch(browser_config)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to launch browser: {}", e))?;

    // Spawn handler task
    let handler_task = tokio::spawn(async move { while handler.next().await.is_some() {} });

    Ok(LaunchResult {
        browser,
        handler_task,
    })
}

/// Find the Chromium/Chrome binary on the system.
#[cfg(windows)]
pub fn find_browser_binary() -> Option<String> {
    let mut candidates = Vec::new();

    // Standard Program Files locations
    if let Ok(pf) = std::env::var("ProgramFiles") {
        candidates.push(
            std::path::PathBuf::from(pf.clone()).join("Google/Chrome/Application/chrome.exe"),
        );
        candidates.push(std::path::PathBuf::from(pf).join("Microsoft/Edge/Application/msedge.exe"));
    }
    if let Ok(pf86) = std::env::var("ProgramFiles(x86)") {
        candidates
            .push(std::path::PathBuf::from(pf86).join("Google/Chrome/Application/chrome.exe"));
    }
    // LocalAppData
    if let Ok(local_appdata) = std::env::var("LOCALAPPDATA") {
        let appdata_path = std::path::PathBuf::from(local_appdata);
        candidates.push(appdata_path.join("Google/Chrome/Application/chrome.exe"));
        candidates.push(appdata_path.join("Microsoft/Edge/Application/msedge.exe"));

        // Playwright cache on Windows is in LocalAppData/ms-playwright
        let playwright_dir = appdata_path.join("ms-playwright");
        if playwright_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(playwright_dir) {
                for entry in entries.flatten() {
                    if let Ok(file_type) = entry.file_type() {
                        if file_type.is_dir() {
                            let name = entry.file_name();
                            let name_str = name.to_string_lossy();
                            if name_str.starts_with("chromium-") {
                                let path = entry.path().join("chrome-win/chrome.exe");
                                if path.exists() {
                                    return Some(path.to_string_lossy().into_owned());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    for candidate in candidates {
        if candidate.exists() {
            return Some(candidate.to_string_lossy().into_owned());
        }
    }

    None
}

/// Find the Chromium/Chrome binary on the system.
#[cfg(not(windows))]
pub fn find_browser_binary() -> Option<String> {
    let candidates = [
        "/usr/bin/chromium",
        "/usr/bin/chromium-browser",
        "/usr/bin/google-chrome",
        "/usr/bin/google-chrome-stable",
        "/snap/bin/chromium",
        "/opt/google/chrome/chrome",
    ];

    for candidate in &candidates {
        if std::path::Path::new(candidate).exists() {
            return Some(candidate.to_string());
        }
    }

    // Try finding in Playwright cache directory
    if let Some(home) = dirs::home_dir() {
        let playwright_dir = home.join(".cache/ms-playwright");
        if playwright_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(playwright_dir) {
                for entry in entries.flatten() {
                    if let Ok(file_type) = entry.file_type() {
                        if file_type.is_dir() {
                            let name = entry.file_name();
                            let name_str = name.to_string_lossy();
                            if name_str.starts_with("chromium-") {
                                let path = entry.path().join("chrome-linux64/chrome");
                                if path.exists() {
                                    return Some(path.to_string_lossy().into_owned());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Try via `which`
    let which_candidates = [
        "chromium",
        "chromium-browser",
        "google-chrome",
        "google-chrome-stable",
    ];
    for candidate in &which_candidates {
        if let Ok(output) = std::process::Command::new("which").arg(candidate).output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return Some(path);
                }
            }
        }
    }
    None
}

/// Find the Lightpanda binary on the system.
pub fn find_lightpanda_binary(config_path: &str) -> Option<String> {
    // 1. Check config path
    if !config_path.is_empty() && std::path::Path::new(config_path).exists() {
        return Some(config_path.to_string());
    }
    // 2. Check PATH (use "where" on Windows, "which" on Unix)
    let which_cmd = if cfg!(windows) { "where" } else { "which" };
    if let Ok(output) = std::process::Command::new(which_cmd)
        .arg("lightpanda")
        .output()
    {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(path);
            }
        }
    }
    // 3. Common install dirs
    let mut candidates = vec![
        "/usr/local/bin/lightpanda".to_string(),
        "/opt/homebrew/bin/lightpanda".to_string(),
    ];
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".local/bin/lightpanda").to_string_lossy().into_owned());
    }
    for candidate in &candidates {
        if std::path::Path::new(candidate).exists() {
            return Some(candidate.clone());
        }
    }
    None
}

/// Spawn a Lightpanda subprocess.
pub fn spawn_lightpanda(
    config: &crate::config::LightpandaConfig,
) -> Result<std::process::Child, crate::error::MeleyError> {
    let binary = find_lightpanda_binary(&config.binary_path).ok_or_else(|| {
        crate::error::MeleyError::EngineStartupFailed("Lightpanda binary not found".into())
    })?;
    let mut cmd = std::process::Command::new(&binary);
    cmd.arg("serve")
        .arg("--host")
        .arg("127.0.0.1")
        .arg("--port")
        .arg(config.port.to_string())
        .env("LIGHTPANDA_DISABLE_TELEMETRY", "true");
    if config.obey_robots {
        cmd.arg("--obey-robots");
    }
    let child = cmd.spawn().map_err(|e| {
        crate::error::MeleyError::EngineStartupFailed(format!("Failed to spawn Lightpanda: {}", e))
    })?;
    Ok(child)
}

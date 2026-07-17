use figment::{providers::{Env, Format, Toml}, Figment};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub browser: BrowserConfig,
    #[serde(default)]
    pub search: SearchConfig,
    #[serde(default)]
    pub limits: LimitsConfig,
    #[serde(default)]
    pub downloads: DownloadsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub http_port: u16,
    pub http_bind: String,
    pub mcp_transport: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            http_port: 8787,
            http_bind: "127.0.0.1".to_string(),
            mcp_transport: "stdio".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewportConfig {
    pub width: u32,
    pub height: u32,
}

impl Default for ViewportConfig {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 800,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserConfig {
    pub executable_path: String,
    pub headless: bool,
    pub default_viewport: ViewportConfig,
    pub profile_dir: String,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        let profile_dir = dirs::data_local_dir()
            .map(|d| d.join("meleys").join("profiles"))
            .unwrap_or_else(|| std::path::PathBuf::from("~/.local/share/meleys/profiles"))
            .to_string_lossy()
            .to_string();
        Self {
            executable_path: String::new(),
            headless: true,
            default_viewport: ViewportConfig::default(),
            profile_dir,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    pub default_engine: String,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            default_engine: "duckduckgo".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitsConfig {
    pub max_sessions: usize,
    pub default_action_timeout_ms: u64,
    pub max_dom_nodes_per_call: usize,
    pub allow_evaluate_js: bool,
}

impl Default for LimitsConfig {
    fn default() -> Self {
        Self {
            max_sessions: 8,
            default_action_timeout_ms: 30000,
            max_dom_nodes_per_call: 2000,
            allow_evaluate_js: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadsConfig {
    pub dir: String,
    pub allowed_save_dirs: Vec<String>,
}

impl Default for DownloadsConfig {
    fn default() -> Self {
        let dir = dirs::data_local_dir()
            .map(|d| d.join("meleys").join("downloads"))
            .unwrap_or_else(|| std::path::PathBuf::from("~/.local/share/meleys/downloads"))
            .to_string_lossy()
            .to_string();
        Self {
            dir,
            allowed_save_dirs: vec![],
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            browser: BrowserConfig::default(),
            search: SearchConfig::default(),
            limits: LimitsConfig::default(),
            downloads: DownloadsConfig::default(),
        }
    }
}

impl Config {
    /// Load configuration from config.toml (if present) and environment variables.
    pub fn load() -> anyhow::Result<Self> {
        let config: Config = Figment::new()
            .merge(Toml::file("config.toml"))
            .merge(Env::prefixed("MELEYS_"))
            .extract()
            .unwrap_or_default();
        Ok(config)
    }

    /// Load from a specific path.
    pub fn load_from(path: &str) -> anyhow::Result<Self> {
        let config: Config = Figment::new()
            .merge(Toml::file(path))
            .merge(Env::prefixed("MELEYS_"))
            .extract()
            .unwrap_or_default();
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_server_config() {
        let server = ServerConfig::default();
        assert_eq!(server.http_port, 8787);
        assert_eq!(server.http_bind, "127.0.0.1");
        assert_eq!(server.mcp_transport, "stdio");
    }

    #[test]
    fn test_default_viewport_config() {
        let vp = ViewportConfig::default();
        assert_eq!(vp.width, 1280);
        assert_eq!(vp.height, 800);
    }

    #[test]
    fn test_default_browser_config() {
        let browser = BrowserConfig::default();
        assert!(browser.executable_path.is_empty());
        assert!(browser.headless);
        assert_eq!(browser.default_viewport.width, 1280);
        assert_eq!(browser.default_viewport.height, 800);
        assert!(browser.profile_dir.contains("meleys"));
        assert!(browser.profile_dir.contains("profiles"));
    }

    #[test]
    fn test_default_search_config() {
        let search = SearchConfig::default();
        assert_eq!(search.default_engine, "duckduckgo");
    }

    #[test]
    fn test_default_limits_config() {
        let limits = LimitsConfig::default();
        assert_eq!(limits.max_sessions, 8);
        assert_eq!(limits.default_action_timeout_ms, 30000);
        assert_eq!(limits.max_dom_nodes_per_call, 2000);
        assert!(!limits.allow_evaluate_js);
    }

    #[test]
    fn test_default_downloads_config() {
        let downloads = DownloadsConfig::default();
        assert!(downloads.dir.contains("meleys"));
        assert!(downloads.dir.contains("downloads"));
        assert!(downloads.allowed_save_dirs.is_empty());
    }

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.server.http_port, 8787);
        assert!(config.browser.headless);
        assert_eq!(config.search.default_engine, "duckduckgo");
        assert_eq!(config.limits.max_sessions, 8);
    }

    #[test]
    fn test_config_roundtrip_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string(&config).expect("Failed to serialize config");
        let deserialized: Config = toml::from_str(&toml_str).expect("Failed to deserialize config");
        assert_eq!(deserialized.server.http_port, config.server.http_port);
        assert_eq!(deserialized.server.http_bind, config.server.http_bind);
        assert_eq!(deserialized.browser.headless, config.browser.headless);
        assert_eq!(deserialized.search.default_engine, config.search.default_engine);
        assert_eq!(deserialized.limits.max_sessions, config.limits.max_sessions);
        assert_eq!(deserialized.limits.allow_evaluate_js, config.limits.allow_evaluate_js);
    }

    #[test]
    fn test_config_from_toml_string() {
        let toml_str = r#"
[server]
http_port = 9999
http_bind = "0.0.0.0"
mcp_transport = "sse"

[browser]
executable_path = ""
headless = false
default_viewport = { width = 1920, height = 1080 }
profile_dir = "/tmp/profiles"

[search]
default_engine = "google"

[limits]
max_sessions = 16
default_action_timeout_ms = 30000
max_dom_nodes_per_call = 2000
allow_evaluate_js = true

[downloads]
dir = "/tmp/downloads"
allowed_save_dirs = []
"#;
        let config: Config = toml::from_str(toml_str).expect("Failed to parse TOML");
        assert_eq!(config.server.http_port, 9999);
        assert_eq!(config.server.http_bind, "0.0.0.0");
        assert_eq!(config.server.mcp_transport, "sse");
        assert!(!config.browser.headless);
        assert_eq!(config.browser.default_viewport.width, 1920);
        assert_eq!(config.browser.default_viewport.height, 1080);
        assert_eq!(config.search.default_engine, "google");
        assert_eq!(config.limits.max_sessions, 16);
        assert!(config.limits.allow_evaluate_js);
    }

    #[test]
    fn test_config_partial_toml_uses_defaults() {
        let toml_str = r#"
[server]
http_port = 3000
http_bind = "127.0.0.1"
mcp_transport = "stdio"

[browser]
executable_path = ""
headless = true
default_viewport = { width = 1280, height = 800 }
profile_dir = "/tmp"

[search]
default_engine = "duckduckgo"

[limits]
max_sessions = 8
default_action_timeout_ms = 30000
max_dom_nodes_per_call = 2000
allow_evaluate_js = false

[downloads]
dir = "/tmp"
allowed_save_dirs = []
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.server.http_port, 3000);
        assert_eq!(config.server.http_bind, "127.0.0.1");
        assert!(config.browser.headless);
        assert_eq!(config.search.default_engine, "duckduckgo");
    }

    #[test]
    fn test_config_load_from_nonexistent_file() {
        let config = Config::load_from("/nonexistent/path/config.toml");
        assert!(config.is_ok());
    }

    #[test]
    fn test_config_json_serialization() {
        let config = Config::default();
        let json = serde_json::to_string(&config).expect("JSON serialization failed");
        let deserialized: Config = serde_json::from_str(&json).expect("JSON deserialization failed");
        assert_eq!(deserialized.server.http_port, 8787);
    }

    #[test]
    fn test_viewport_config_custom_values() {
        let vp = ViewportConfig { width: 1920, height: 1080 };
        assert_eq!(vp.width, 1920);
        assert_eq!(vp.height, 1080);
        let json = serde_json::to_string(&vp).unwrap();
        assert!(json.contains("1920"));
        assert!(json.contains("1080"));
    }
}

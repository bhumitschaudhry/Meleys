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

use std::path::PathBuf;
use anyhow::Result;

/// Manages an on-disk profile directory for a browser session.
#[derive(Debug, Clone)]
pub struct Profile {
    pub name: String,
    pub path: PathBuf,
}

impl Profile {
    /// Create or open an existing profile directory.
    pub fn open(profiles_base_dir: &str, name: &str) -> Result<Self> {
        let base = expand_path(profiles_base_dir);
        let path = base.join(sanitize_profile_name(name));
        std::fs::create_dir_all(&path)?;
        Ok(Self {
            name: name.to_string(),
            path,
        })
    }

    /// Create a temporary profile with a generated name.
    pub fn temporary(profiles_base_dir: &str) -> Result<Self> {
        let name = format!("tmp-{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("unknown"));
        Self::open(profiles_base_dir, &name)
    }

    pub fn downloads_dir(&self) -> PathBuf {
        let d = self.path.join("downloads");
        let _ = std::fs::create_dir_all(&d);
        d
    }

    pub fn path_str(&self) -> String {
        self.path.to_string_lossy().to_string()
    }
}

fn sanitize_profile_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

fn expand_path(path: &str) -> PathBuf {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(&path[2..]);
        }
    }
    PathBuf::from(path)
}

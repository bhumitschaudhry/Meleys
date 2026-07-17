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
        let sanitized_name = sanitize_profile_name(name);
        let path = base.join(&sanitized_name);
        std::fs::create_dir_all(&path)?;
        Ok(Self {
            name: sanitized_name,
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
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_sanitize_profile_name_clean() {
        assert_eq!(sanitize_profile_name("my-profile_1"), "my-profile_1");
    }

    #[test]
    fn test_sanitize_profile_name_special_chars() {
        let sanitized = sanitize_profile_name("my profile!@#$%");
        assert_eq!(sanitized.len(), "my profile!@#$%".len());
        assert!(sanitized.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_'));
        assert!(sanitized.starts_with("my"));
        assert!(sanitized.contains("profile"));
    }

    #[test]
    fn test_sanitize_profile_name_empty() {
        assert_eq!(sanitize_profile_name(""), "");
    }

    #[test]
    fn test_sanitize_profile_name_unicode() {
        // 'é' is alphanumeric in Unicode, so it's kept
        assert_eq!(sanitize_profile_name("café"), "café");
    }

    #[test]
    fn test_sanitize_profile_name_slashes() {
        let sanitized = sanitize_profile_name("../etc/passwd");
        // '/' and '.' are replaced with '_'
        assert!(sanitized.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_'));
        assert!(sanitized.contains("etc"));
        assert!(sanitized.contains("passwd"));
        assert!(!sanitized.contains('/'));
        assert!(!sanitized.contains('.'));
    }

    #[test]
    fn test_expand_path_no_tilde() {
        let path = expand_path("/absolute/path");
        assert_eq!(path, PathBuf::from("/absolute/path"));
    }

    #[test]
    fn test_expand_path_relative() {
        let path = expand_path("relative/path");
        assert_eq!(path, PathBuf::from("relative/path"));
    }

    #[test]
    fn test_expand_path_tilde() {
        let path = expand_path("~/Documents");
        if let Some(home) = dirs::home_dir() {
            assert_eq!(path, home.join("Documents"));
        } else {
            assert_eq!(path, PathBuf::from("~/Documents"));
        }
    }

    #[test]
    fn test_profile_open_creates_directory() {
        let tmp = std::env::temp_dir().join("meleys-test-profiles");
        let profile = Profile::open(tmp.to_str().unwrap(), "test-user");
        assert!(profile.is_ok());
        let p = profile.unwrap();
        assert_eq!(p.name, "test-user");
        assert!(p.path.exists());
        // Clean up
        let _ = fs::remove_dir_all(&p.path);
    }

    #[test]
    fn test_profile_open_existing_directory() {
        let tmp = std::env::temp_dir().join("meleys-test-profiles-existing");
        let name = format!("test-{}", uuid::Uuid::new_v4());
        let profile = Profile::open(tmp.to_str().unwrap(), &name).unwrap();
        // Open again - should succeed
        let profile2 = Profile::open(tmp.to_str().unwrap(), &name);
        assert!(profile2.is_ok());
        // Clean up
        let _ = fs::remove_dir_all(&profile.path);
    }

    #[test]
    fn test_profile_temporary_has_tmp_prefix() {
        let tmp = std::env::temp_dir().join("meleys-test-profiles-tmp");
        let profile = Profile::temporary(tmp.to_str().unwrap());
        assert!(profile.is_ok());
        let p = profile.unwrap();
        assert!(p.name.starts_with("tmp-"));
        assert!(p.path.exists());
        // Clean up
        let _ = fs::remove_dir_all(&p.path);
    }

    #[test]
    fn test_profile_downloads_dir() {
        let tmp = std::env::temp_dir().join("meleys-test-profiles-dl");
        let profile = Profile::open(tmp.to_str().unwrap(), "dl-test").unwrap();
        let dl_dir = profile.downloads_dir();
        assert!(dl_dir.exists());
        assert!(dl_dir.to_string_lossy().contains("downloads"));
        // Clean up
        let _ = fs::remove_dir_all(&profile.path);
    }

    #[test]
    fn test_profile_path_str() {
        let tmp = std::env::temp_dir().join("meleys-test-profiles-str");
        let profile = Profile::open(tmp.to_str().unwrap(), "path-test").unwrap();
        let path_str = profile.path_str();
        assert!(!path_str.is_empty());
        assert!(path_str.contains("path-test"));
        // Clean up
        let _ = fs::remove_dir_all(&profile.path);
    }

    #[test]
    fn test_profile_is_cloneable() {
        let tmp = std::env::temp_dir().join("meleys-test-profiles-clone");
        let profile = Profile::open(tmp.to_str().unwrap(), "clone-test").unwrap();
        let cloned = profile.clone();
        assert_eq!(profile.name, cloned.name);
        assert_eq!(profile.path, cloned.path);
        // Clean up
        let _ = fs::remove_dir_all(&profile.path);
    }

    #[test]
    fn test_profile_sanitizes_name_with_dots() {
        let tmp = std::env::temp_dir().join("meleys-test-profiles-dots");
        let profile = Profile::open(tmp.to_str().unwrap(), "../../etc").unwrap();
        // Profile stores original name but path is sanitized
        assert!(!profile.path.to_string_lossy().contains('/'));
        assert!(profile.path.exists());
        // Clean up
        let _ = fs::remove_dir_all(&profile.path);
    }
}

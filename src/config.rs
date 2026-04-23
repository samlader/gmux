use crate::error::{GmuxError, Result};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::PathBuf;

pub const DEFAULT_PR_TEMPLATE_NAME: &str = "pr_template.md";
pub const DEFAULT_CONFIG_DIR: &str = ".gmux";
pub const DEFAULT_CONFIG_FILE: &str = "config.json";
pub const GITHUB_TOKEN_ENV_VAR: &str = "GMUX_GITHUB_TOKEN";
const KEYRING_SERVICE: &str = "gmux";
const KEYRING_GITHUB_TOKEN_ACCOUNT: &str = "github-token";

pub const DEFAULT_PR_TEMPLATE: &str = r#"# {{ title }}

## Changes
{% for file in diff_files %}
- {{ file }}
{% endfor %}

## Repository
{{ repository_name }}
"#;

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(default, skip_serializing)]
    pub github_token: String,
    #[serde(default)]
    pub default_org: String,
    #[serde(default = "default_per_page")]
    pub per_page: u8,
    #[serde(default = "default_sort")]
    pub sort: String,
    #[serde(default = "default_direction")]
    pub direction: String,
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field(
                "github_token",
                &if self.github_token.is_empty() {
                    "<empty>"
                } else {
                    "<redacted>"
                },
            )
            .field("default_org", &self.default_org)
            .field("per_page", &self.per_page)
            .field("sort", &self.sort)
            .field("direction", &self.direction)
            .finish()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            github_token: String::new(),
            default_org: String::new(),
            per_page: default_per_page(),
            sort: default_sort(),
            direction: default_direction(),
        }
    }
}

fn default_per_page() -> u8 {
    100
}
fn default_sort() -> String {
    "updated".to_string()
}
fn default_direction() -> String {
    "desc".to_string()
}

impl Config {
    pub fn validate(&self) -> Result<()> {
        if self.github_token.is_empty() {
            return Err(GmuxError::Config("GitHub token is required".into()));
        }
        Ok(())
    }

    pub fn save(&self, path: &PathBuf) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
}

pub fn load_github_token_from_secure_store() -> Result<Option<String>> {
    if let Ok(token) = std::env::var(GITHUB_TOKEN_ENV_VAR) {
        if !token.trim().is_empty() {
            return Ok(Some(token));
        }
    }

    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_GITHUB_TOKEN_ACCOUNT)?;
    match entry.get_password() {
        Ok(token) if !token.trim().is_empty() => Ok(Some(token)),
        Ok(_) | Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(GmuxError::from(error)),
    }
}

pub fn save_github_token_to_secure_store(token: &str) -> Result<()> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_GITHUB_TOKEN_ACCOUNT)?;
    entry.set_password(token)?;
    Ok(())
}

fn load_config_file(path: &PathBuf) -> Result<Config> {
    if !path.exists() {
        return Ok(Config::default());
    }

    let content = fs::read_to_string(path)?;
    let config: Config = serde_json::from_str(&content)?;
    Ok(config)
}

pub fn load_config_for_setup(path: &PathBuf) -> Result<Config> {
    let mut config = load_config_file(path)?;
    if config.github_token.is_empty() {
        if let Some(token) = load_github_token_from_secure_store()? {
            config.github_token = token;
        }
    }
    Ok(config)
}

pub fn get_config_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("GMUX_CONFIG_DIR") {
        PathBuf::from(dir)
    } else {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(DEFAULT_CONFIG_DIR)
    }
}

pub fn get_config_path() -> PathBuf {
    get_config_dir().join(DEFAULT_CONFIG_FILE)
}

pub fn get_template_path() -> PathBuf {
    get_config_dir().join(DEFAULT_PR_TEMPLATE_NAME)
}

pub fn load_config(path: &PathBuf) -> Result<Config> {
    let mut config = load_config_file(path)?;
    if config.github_token.is_empty() {
        if let Some(token) = load_github_token_from_secure_store()? {
            config.github_token = token;
        }
    }
    config.validate()?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_config_validation() {
        let config = Config::default();
        assert!(config.validate().is_err());

        let config = Config {
            github_token: "test-token".to_string(),
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_load_save_config() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config_path = temp_dir.path().join("config.json");

        let config = Config {
            github_token: "test-token".to_string(),
            default_org: "test-org".to_string(),
            ..Default::default()
        };

        config.save(&config_path)?;
        let loaded_config = load_config_file(&config_path)?;

        assert_eq!(loaded_config.github_token, "");
        assert_eq!(loaded_config.default_org, "test-org");
        assert_eq!(loaded_config.per_page, 100);
        assert_eq!(loaded_config.sort, "updated");
        assert_eq!(loaded_config.direction, "desc");

        Ok(())
    }

    #[test]
    fn test_save_config_does_not_write_token() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config_path = temp_dir.path().join("config.json");

        let config = Config {
            github_token: "test-token".to_string(),
            default_org: "test-org".to_string(),
            ..Default::default()
        };

        config.save(&config_path)?;
        let content = fs::read_to_string(&config_path)?;

        assert!(!content.contains("test-token"));
        assert!(!content.contains("github_token"));

        Ok(())
    }
}

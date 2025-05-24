use crate::error::{GmuxError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

pub const DEFAULT_PR_TEMPLATE_NAME: &str = "pr_template.md";
pub const DEFAULT_CONFIG_DIR: &str = ".gmux";
pub const DEFAULT_CONFIG_FILE: &str = "config.json";

pub const DEFAULT_PR_TEMPLATE: &str = r#"# {{ title }}

## Changes
{% for file in diff_files %}
- {{ file }}
{% endfor %}

## Repository
{{ repository_name }}
"#;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
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
        if self.default_org.is_empty() {
            return Err(GmuxError::Config("Default organization is required".into()));
        }
        Ok(())
    }

    pub fn save(&self, path: &PathBuf) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
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
    if !path.exists() {
        return Ok(Config::default());
    }

    let content = fs::read_to_string(path)?;
    let config: Config = serde_json::from_str(&content)?;
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
            default_org: "test-org".to_string(),
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
        let loaded_config = load_config(&config_path)?;

        assert_eq!(loaded_config.github_token, "test-token");
        assert_eq!(loaded_config.default_org, "test-org");
        assert_eq!(loaded_config.per_page, 100);
        assert_eq!(loaded_config.sort, "updated");
        assert_eq!(loaded_config.direction, "desc");

        Ok(())
    }
}

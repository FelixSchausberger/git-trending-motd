use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub providers: ProvidersConfig,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub gitea: GiteaConfig,
    #[serde(default)]
    pub github: GitHubConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    #[serde(default = "default_max_per_provider")]
    pub max_per_provider: usize,
    #[serde(default)]
    pub github_max_entries: Option<usize>,
    #[serde(default)]
    pub gitlab_max_entries: Option<usize>,
    #[serde(default)]
    pub gitea_max_entries: Option<usize>,
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    #[serde(default = "default_cache_ttl_mins")]
    pub cache_ttl_mins: u64,
    #[serde(default)]
    pub language_filter: Vec<String>,
    #[serde(default = "default_github_timeout_secs")]
    pub github_timeout_secs: u64,
    #[serde(default = "default_gitlab_timeout_secs")]
    pub gitlab_timeout_secs: u64,
    #[serde(default = "default_gitea_timeout_secs")]
    pub gitea_timeout_secs: u64,
    #[serde(default)]
    pub ascii_only: bool,
    #[serde(default)]
    pub min_stars: Option<u32>,
    #[serde(default = "default_fast_network_timeout_secs")]
    pub fast_network_timeout_secs: u64,
    #[serde(default = "default_true")]
    pub show_starred_status: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvidersConfig {
    #[serde(default = "default_true")]
    pub github: bool,
    #[serde(default = "default_true")]
    pub gitlab: bool,
    #[serde(default = "default_true")]
    pub gitea: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[allow(clippy::struct_field_names)]
pub struct AuthConfig {
    pub github_token: Option<String>,
    pub gitlab_token: Option<String>,
    pub gitea_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiteaConfig {
    #[serde(default = "default_gitea_url")]
    pub base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GitHubConfig {
    #[serde(default)]
    pub exclude_topics: Vec<String>,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            max_per_provider: default_max_per_provider(),
            github_max_entries: None,
            gitlab_max_entries: None,
            gitea_max_entries: None,
            timeout_secs: default_timeout_secs(),
            cache_ttl_mins: default_cache_ttl_mins(),
            language_filter: vec![],
            github_timeout_secs: default_github_timeout_secs(),
            gitlab_timeout_secs: default_gitlab_timeout_secs(),
            gitea_timeout_secs: default_gitea_timeout_secs(),
            ascii_only: false,
            min_stars: None,
            fast_network_timeout_secs: default_fast_network_timeout_secs(),
            show_starred_status: true,
        }
    }
}

impl Default for ProvidersConfig {
    fn default() -> Self {
        Self {
            github: true,
            gitlab: true,
            gitea: true,
        }
    }
}

impl Default for GiteaConfig {
    fn default() -> Self {
        Self {
            base_url: default_gitea_url(),
        }
    }
}

fn default_max_per_provider() -> usize {
    2
}

fn default_timeout_secs() -> u64 {
    6
}

fn default_github_timeout_secs() -> u64 {
    15
}

fn default_gitlab_timeout_secs() -> u64 {
    10
}

fn default_gitea_timeout_secs() -> u64 {
    10
}

fn default_cache_ttl_mins() -> u64 {
    60
}

fn default_gitea_url() -> String {
    "https://gitea.com".to_string()
}

fn default_fast_network_timeout_secs() -> u64 {
    3
}

fn default_true() -> bool {
    true
}

impl Config {
    /// Load configuration from file, with XDG config directory support
    pub fn load() -> Result<Self> {
        // Try XDG config directory first, then current directory
        let config_paths = [
            dirs::config_dir().map(|p| p.join("git-trending-motd").join("config.toml")),
            Some(PathBuf::from("config.toml")),
        ];

        for path in config_paths.iter().flatten() {
            if path.exists() {
                let content = std::fs::read_to_string(path)
                    .with_context(|| format!("Failed to read config file: {}", path.display()))?;

                let mut config: Config = toml::from_str(&content)
                    .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

                // Convert empty token strings to None
                config.normalize_tokens();

                // Apply environment variable overrides
                config.apply_env_overrides();

                return Ok(config);
            }
        }

        // No config file found, create default and warn user
        Self::create_default_config_if_missing()?;

        let mut config = Config::default();
        config.apply_env_overrides();
        Ok(config)
    }

    /// Create a default config file in the XDG config directory if it doesn't exist
    fn create_default_config_if_missing() -> Result<()> {
        if let Some(config_dir) = dirs::config_dir() {
            let config_path = config_dir.join("git-trending-motd").join("config.toml");

            // Only create if it doesn't exist
            if !config_path.exists() {
                // Create directory if needed
                if let Some(parent) = config_path.parent() {
                    std::fs::create_dir_all(parent).with_context(|| {
                        format!("Failed to create config directory: {}", parent.display())
                    })?;
                }

                // Create default config
                let default_config = Config::default();
                let toml_content = toml::to_string_pretty(&default_config)
                    .context("Failed to serialize default config")?;

                std::fs::write(&config_path, toml_content).with_context(|| {
                    format!("Failed to write default config: {}", config_path.display())
                })?;

                eprintln!(
                    "ℹ No config file found. Created default configuration at: {}",
                    config_path.display()
                );
                eprintln!("  Edit this file to customize git-trending-motd settings.");
            }
        }

        Ok(())
    }

    /// Convert empty token strings to None
    fn normalize_tokens(&mut self) {
        if let Some(ref token) = self.auth.github_token {
            if token.trim().is_empty() {
                self.auth.github_token = None;
            }
        }
        if let Some(ref token) = self.auth.gitlab_token {
            if token.trim().is_empty() {
                self.auth.gitlab_token = None;
            }
        }
        if let Some(ref token) = self.auth.gitea_token {
            if token.trim().is_empty() {
                self.auth.gitea_token = None;
            }
        }
    }

    /// Apply environment variable overrides
    fn apply_env_overrides(&mut self) {
        if let Ok(val) = std::env::var("GIT_TRENDING_MOTD_MAX_PER_PROVIDER") {
            if let Ok(max) = val.parse() {
                self.general.max_per_provider = max;
            }
        }

        if let Ok(val) = std::env::var("GIT_TRENDING_MOTD_LANGUAGE_FILTER") {
            self.general.language_filter = val.split(',').map(|s| s.trim().to_string()).collect();
        }

        if let Ok(val) = std::env::var("GIT_TRENDING_MOTD_GITHUB_TIMEOUT_SECS") {
            if let Ok(timeout) = val.parse() {
                self.general.github_timeout_secs = timeout;
            }
        }

        if let Ok(val) = std::env::var("GIT_TRENDING_MOTD_GITLAB_TIMEOUT_SECS") {
            if let Ok(timeout) = val.parse() {
                self.general.gitlab_timeout_secs = timeout;
            }
        }

        if let Ok(val) = std::env::var("GIT_TRENDING_MOTD_GITEA_TIMEOUT_SECS") {
            if let Ok(timeout) = val.parse() {
                self.general.gitea_timeout_secs = timeout;
            }
        }

        if let Ok(val) = std::env::var("GIT_TRENDING_MOTD_GITEA_BASE_URL") {
            self.gitea.base_url = val;
        }

        if let Ok(val) = std::env::var("GIT_TRENDING_MOTD_GITHUB_TOKEN") {
            self.auth.github_token = Some(val);
        }

        if let Ok(val) = std::env::var("GIT_TRENDING_MOTD_GITLAB_TOKEN") {
            self.auth.gitlab_token = Some(val);
        }

        if let Ok(val) = std::env::var("GIT_TRENDING_MOTD_GITEA_TOKEN") {
            self.auth.gitea_token = Some(val);
        }

        if let Ok(val) = std::env::var("GIT_TRENDING_MOTD_MIN_STARS") {
            if let Ok(min) = val.parse() {
                self.general.min_stars = Some(min);
            }
        }

        if let Ok(val) = std::env::var("GIT_TRENDING_MOTD_GITHUB_EXCLUDE_TOPICS") {
            self.github.exclude_topics = val.split(',').map(|s| s.trim().to_string()).collect();
        }
    }

    /// Get list of enabled providers
    pub fn enabled_providers(&self) -> Vec<&str> {
        let mut providers = Vec::new();
        if self.providers.github {
            providers.push("github");
        }
        if self.providers.gitlab {
            providers.push("gitlab");
        }
        if self.providers.gitea {
            providers.push("gitea");
        }
        providers
    }

    /// Get the maximum number of entries for a specific provider
    pub fn get_max_entries(&self, provider: &str) -> usize {
        match provider {
            "github" => self
                .general
                .github_max_entries
                .unwrap_or(self.general.max_per_provider),
            "gitlab" => self
                .general
                .gitlab_max_entries
                .unwrap_or(self.general.max_per_provider),
            "gitea" => self
                .general
                .gitea_max_entries
                .unwrap_or(self.general.max_per_provider),
            _ => self.general.max_per_provider,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.general.max_per_provider, 2);
        assert_eq!(config.general.timeout_secs, 6);
        assert_eq!(config.general.cache_ttl_mins, 60);
        assert!(config.providers.github);
        assert!(config.providers.gitlab);
        assert!(config.providers.gitea);
        assert_eq!(config.gitea.base_url, "https://gitea.com");
    }

    #[test]
    fn test_enabled_providers() {
        let mut config = Config::default();
        config.providers.gitlab = false;
        let enabled = config.enabled_providers();
        assert_eq!(enabled, vec!["github", "gitea"]);
    }

    #[test]
    fn test_config_parsing() {
        let toml_str = r#"
            [general]
            max_per_provider = 5
            timeout_secs = 10
            cache_ttl_mins = 30
            language_filter = ["rust", "go"]

            [providers]
            github = true
            gitlab = false
            gitea = true

            [gitea]
            base_url = "https://codeberg.org"
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.general.max_per_provider, 5);
        assert_eq!(config.general.timeout_secs, 10);
        assert_eq!(config.general.language_filter, vec!["rust", "go"]);
        assert!(!config.providers.gitlab);
        assert_eq!(config.gitea.base_url, "https://codeberg.org");
    }

    #[test]
    fn test_get_max_entries_defaults() {
        let config = Config::default();
        // All providers should use the global default
        assert_eq!(config.get_max_entries("github"), 2);
        assert_eq!(config.get_max_entries("gitlab"), 2);
        assert_eq!(config.get_max_entries("gitea"), 2);
        assert_eq!(config.get_max_entries("unknown"), 2);
    }

    #[test]
    fn test_get_max_entries_overrides() {
        let mut config = Config::default();
        config.general.max_per_provider = 2;
        config.general.github_max_entries = Some(3);
        config.general.gitlab_max_entries = Some(1);
        // gitea_max_entries is None, should use default

        assert_eq!(config.get_max_entries("github"), 3);
        assert_eq!(config.get_max_entries("gitlab"), 1);
        assert_eq!(config.get_max_entries("gitea"), 2); // Falls back to default
        assert_eq!(config.get_max_entries("unknown"), 2); // Falls back to default
    }

    #[test]
    fn test_config_parsing_with_per_provider_limits() {
        let toml_str = r"
            [general]
            max_per_provider = 2
            github_max_entries = 3
            gitlab_max_entries = 1
            gitea_max_entries = 1
        ";

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.get_max_entries("github"), 3);
        assert_eq!(config.get_max_entries("gitlab"), 1);
        assert_eq!(config.get_max_entries("gitea"), 1);
    }
}

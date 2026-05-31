use git_trending_motd::{Config, Provider, Repository};
use std::path::PathBuf;
use tempfile::TempDir;

/// Creates a mock Config for testing
pub fn mock_config() -> Config {
    Config {
        providers: vec![Provider::GitHub],
        languages: vec![],
        ascii_only: false,
        token: None,
        cache_ttl_secs: 3600,
        max_repos: 5,
        format: git_trending_motd::OutputFormat::Motd,
        show_all: false,
        http_proxy: None,
        https_proxy: None,
    }
}

/// Creates a mock Config with GitHub token
pub fn mock_config_with_token(token: &str) -> Config {
    let mut config = mock_config();
    config.token = Some(token.to_string());
    config
}

/// Creates a test Repository
pub fn test_repo(name: &str, description: &str) -> Repository {
    Repository {
        name: name.to_string(),
        url: format!("https://github.com/{}", name),
        description: Some(description.to_string()),
        language: Some("Rust".to_string()),
        stars: 1000,
        forks: 100,
        stars_today: 50,
        is_starred: false,
    }
}

/// Creates a test Repository with starred status
pub fn test_starred_repo(name: &str, description: &str) -> Repository {
    let mut repo = test_repo(name, description);
    repo.is_starred = true;
    repo
}

/// Creates a temporary cache directory that auto-cleans on drop
pub fn temp_cache_dir() -> TempDir {
    tempfile::tempdir().expect("Failed to create temp directory")
}

/// Creates a temporary cache directory and returns the path
pub fn temp_cache_path() -> PathBuf {
    let dir = temp_cache_dir();
    let path = dir.path().to_path_buf();
    // Keep the directory alive by leaking it (will be cleaned up by OS)
    std::mem::forget(dir);
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_config_defaults() {
        let config = mock_config();
        assert_eq!(config.providers.len(), 1);
        assert_eq!(config.max_repos, 5);
        assert!(!config.show_all);
    }

    #[test]
    fn test_mock_config_with_token() {
        let config = mock_config_with_token("test_token");
        assert_eq!(config.token, Some("test_token".to_string()));
    }

    #[test]
    fn test_test_repo_creation() {
        let repo = test_repo("rust-lang/rust", "Rust compiler");
        assert_eq!(repo.name, "rust-lang/rust");
        assert_eq!(repo.url, "https://github.com/rust-lang/rust");
        assert!(!repo.is_starred);
    }

    #[test]
    fn test_starred_repo_creation() {
        let repo = test_starred_repo("rust-lang/rust", "Rust compiler");
        assert!(repo.is_starred);
    }

    #[test]
    fn test_temp_cache_dir_exists() {
        let dir = temp_cache_dir();
        assert!(dir.path().exists());
    }
}

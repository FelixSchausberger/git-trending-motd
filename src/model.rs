use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Normalized repository structure across all providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repo {
    pub provider: String,
    pub icon: String,
    pub name: String,
    pub language: Option<String>,
    pub description: Option<String>,
    pub url: String,
    pub stars_today: Option<u64>,
    pub stars_total: Option<u64>,
    pub last_activity: Option<DateTime<Utc>>,
    #[serde(default)]
    pub topics: Vec<String>,
    #[serde(default)]
    pub is_starred: bool,
}

/// Configuration for provider behavior
#[derive(Debug, Clone)]
pub struct ProviderCfg {
    #[allow(dead_code)]
    pub timeout_secs: u64,
    pub token: Option<String>,
    pub base_url: Option<String>,    // For Gitea
    pub exclude_topics: Vec<String>, // For GitHub
}

/// Language filter configuration
#[derive(Debug, Clone)]
pub struct LanguageFilter {
    pub languages: Vec<String>,
}

impl LanguageFilter {
    pub fn new(languages: Vec<String>) -> Self {
        Self { languages }
    }

    pub fn matches(&self, language: Option<&String>) -> bool {
        if self.languages.is_empty() {
            return true;
        }

        language.is_some_and(|lang| {
            self.languages
                .iter()
                .any(|filter| lang.eq_ignore_ascii_case(filter))
        })
    }
}

/// Provider trait for fetching trending repositories
#[async_trait]
pub trait Provider: Send + Sync {
    /// Provider identifier (e.g., "github", "gitlab", "gitea")
    fn id(&self) -> &'static str;

    /// Provider icon for display (e.g., "[GH]")
    fn icon(&self) -> &'static str;

    /// Fetch top repositories of the day with pagination support
    /// offset: Starting position in the trending list (0-indexed)
    /// limit: Number of repos to fetch from that position
    async fn top_today(
        &self,
        cfg: &ProviderCfg,
        offset: usize,
        limit: usize,
        langs: &LanguageFilter,
    ) -> anyhow::Result<Vec<Repo>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_filter_empty() {
        let filter = LanguageFilter::new(vec![]);
        assert!(filter.matches(Some(&"Rust".to_string())));
        assert!(filter.matches(Some(&"Go".to_string())));
        assert!(filter.matches(None));
    }

    #[test]
    fn test_language_filter_case_insensitive() {
        let filter = LanguageFilter::new(vec!["rust".to_string(), "go".to_string()]);
        assert!(filter.matches(Some(&"Rust".to_string())));
        assert!(filter.matches(Some(&"RUST".to_string())));
        assert!(filter.matches(Some(&"Go".to_string())));
        assert!(!filter.matches(Some(&"Python".to_string())));
        assert!(!filter.matches(None));
    }
}

use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;

use crate::http::HttpClient;
use crate::model::{LanguageFilter, Provider, ProviderCfg, Repo};

/// GitLab provider using explore API
pub struct GitLab {
    http: HttpClient,
}

#[derive(Debug, Deserialize)]
struct GitLabProject {
    #[allow(dead_code)]
    name: String,
    path_with_namespace: String,
    description: Option<String>,
    star_count: Option<u64>,
    web_url: String,
    #[serde(default)]
    topics: Vec<String>,
    last_activity_at: Option<String>,
}

impl GitLab {
    pub fn new(timeout_secs: u64) -> Result<Self> {
        Ok(Self {
            http: HttpClient::new(timeout_secs)?,
        })
    }

    /// Create a GitLab provider with a custom `HttpClient`
    #[allow(dead_code)]
    pub fn with_client(http: HttpClient) -> Self {
        Self { http }
    }

    /// Fetch recently active projects from GitLab
    async fn fetch_projects(&self, token: Option<&str>) -> Result<Vec<GitLabProject>> {
        // Get date from 7 days ago in ISO format
        let week_ago = (chrono::Utc::now() - chrono::Duration::days(7))
            .format("%Y-%m-%dT00:00:00Z")
            .to_string();

        // Search for projects with recent activity, sorted by activity date (descending)
        let url = format!(
            "https://gitlab.com/api/v4/projects?order_by=last_activity_at&sort=desc&last_activity_after={week_ago}&per_page=100"
        );

        let projects: Vec<GitLabProject> = self.http.get_json(&url, token).await?;

        // Filter to only repos with at least 10 stars (actually popular)
        Ok(projects
            .into_iter()
            .filter(|p| p.star_count.unwrap_or(0) >= 10)
            .collect())
    }

    /// Extract language from topics (GitLab uses topics, not a dedicated language field)
    fn extract_language(topics: &[String]) -> Option<String> {
        // Common programming language tags (lowercase for comparison)
        let languages = [
            "rust",
            "go",
            "golang",
            "python",
            "javascript",
            "typescript",
            "java",
            "c",
            "cpp",
            "c++",
            "csharp",
            "c#",
            "ruby",
            "php",
            "swift",
            "kotlin",
            "scala",
            "haskell",
            "elixir",
            "erlang",
        ];

        topics
            .iter()
            .find(|topic| {
                let lower = topic.to_lowercase();
                // Use exact match instead of contains to avoid false positives
                languages.iter().any(|&lang| lower == lang)
            })
            .map(|s| {
                // Normalize language name
                let lower = s.to_lowercase();
                match lower.as_str() {
                    "golang" => "Go".to_string(),
                    "c++" => "C++".to_string(),
                    "c#" => "C#".to_string(),
                    _ => {
                        // Capitalize first letter
                        let mut chars = s.chars();
                        match chars.next() {
                            None => String::new(),
                            Some(first) => first.to_uppercase().chain(chars).collect(),
                        }
                    }
                }
            })
    }
}

#[async_trait]
impl Provider for GitLab {
    fn id(&self) -> &'static str {
        "gitlab"
    }

    fn icon(&self) -> &'static str {
        "[GL]"
    }

    async fn top_today(
        &self,
        cfg: &ProviderCfg,
        offset: usize,
        limit: usize,
        langs: &LanguageFilter,
    ) -> Result<Vec<Repo>> {
        let projects = self.fetch_projects(cfg.token.as_deref()).await?;

        let repos = projects
            .into_iter()
            .map(|p| {
                let language = Self::extract_language(&p.topics);
                (p, language)
            })
            .filter(|(_, lang)| langs.matches(lang.as_ref()))
            .skip(offset)
            .take(limit)
            .map(|(p, language)| {
                let last_activity = p
                    .last_activity_at
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&chrono::Utc));

                Repo {
                    provider: self.id().to_string(),
                    icon: self.icon().to_string(),
                    name: p.path_with_namespace,
                    language,
                    description: p.description,
                    url: p.web_url,
                    stars_today: None, // GitLab API doesn't provide daily stars
                    stars_total: p.star_count,
                    last_activity,
                    topics: p.topics,
                    is_starred: false,
                }
            })
            .collect();

        Ok(repos)
    }
}

// Add chrono dependency for date handling
// Note: We need to add chrono to Cargo.toml

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gitlab_provider_metadata() {
        let gitlab = GitLab::new(6).unwrap();
        assert_eq!(gitlab.id(), "gitlab");
        assert_eq!(gitlab.icon(), "[GL]");
    }

    #[test]
    fn test_extract_language() {
        assert_eq!(
            GitLab::extract_language(&["rust".to_string(), "cli".to_string()]),
            Some("Rust".to_string())
        );
        assert_eq!(
            GitLab::extract_language(&["web".to_string(), "python".to_string()]),
            Some("Python".to_string())
        );
        assert_eq!(GitLab::extract_language(&["web".to_string()]), None);
    }

    #[tokio::test]
    async fn test_gitlab_api() {
        // Use max_retries(0) to avoid retry delays in tests
        let http = HttpClient::builder()
            .timeout_secs(10)
            .max_retries(0)
            .build()
            .unwrap();
        let gitlab = GitLab::with_client(http);
        let cfg = ProviderCfg {
            timeout_secs: 10,
            token: None,
            base_url: None,
            exclude_topics: vec![],
        };
        let filter = LanguageFilter::new(vec![]);

        // Try to fetch, but don't fail the test if API is down
        match gitlab.top_today(&cfg, 3, 3, &filter).await {
            Ok(repos) => {
                // Verify structure if API call succeeds
                for repo in repos {
                    assert_eq!(repo.provider, "gitlab");
                    assert_eq!(repo.icon, "[GL]");
                    assert!(repo.last_activity.is_some());
                }
            }
            Err(e) => {
                eprintln!("GitLab API test skipped: {e}");
            }
        }
    }
}

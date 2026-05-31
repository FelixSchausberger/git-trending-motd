use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;

use crate::http::HttpClient;
use crate::model::{LanguageFilter, Provider, ProviderCfg, Repo};

/// Gitea provider using search API with configurable base URL
pub struct Gitea {
    http: HttpClient,
}

#[derive(Debug, Deserialize)]
struct GiteaSearchResponse {
    data: Vec<GiteaRepository>,
}

#[derive(Debug, Deserialize)]
struct GiteaRepository {
    full_name: String,
    description: Option<String>,
    html_url: String,
    stars_count: Option<u64>,
    language: Option<String>,
    updated_at: Option<String>,
}

impl Gitea {
    pub fn new(timeout_secs: u64) -> Result<Self> {
        Ok(Self {
            http: HttpClient::new(timeout_secs)?,
        })
    }

    /// Create a Gitea provider with a custom `HttpClient`
    #[allow(dead_code)]
    pub fn with_client(http: HttpClient) -> Self {
        Self { http }
    }

    /// Fetch repositories from Gitea instance
    async fn fetch_repos(
        &self,
        base_url: &str,
        token: Option<&str>,
    ) -> Result<Vec<GiteaRepository>> {
        // Search for recently updated repos (sorted by update time)
        let url = format!("{base_url}/api/v1/repos/search?sort=updated&order=desc&limit=100");

        let response: GiteaSearchResponse = self.http.get_json(&url, token).await?;

        // Return repos with at least 1 star (Gitea has smaller community)
        let filtered: Vec<_> = response
            .data
            .into_iter()
            .filter(|repo| repo.stars_count.unwrap_or(0) >= 1)
            .collect();

        Ok(filtered)
    }
}

#[async_trait]
impl Provider for Gitea {
    fn id(&self) -> &'static str {
        "gitea"
    }

    fn icon(&self) -> &'static str {
        "[GE]"
    }

    async fn top_today(
        &self,
        cfg: &ProviderCfg,
        offset: usize,
        limit: usize,
        langs: &LanguageFilter,
    ) -> Result<Vec<Repo>> {
        let base_url = cfg.base_url.as_deref().unwrap_or("https://gitea.com");

        let repositories = self.fetch_repos(base_url, cfg.token.as_deref()).await?;

        let repos = repositories
            .into_iter()
            .filter(|r| langs.matches(r.language.as_ref()))
            .skip(offset)
            .take(limit)
            .map(|r| {
                let last_activity = r
                    .updated_at
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&chrono::Utc));

                Repo {
                    provider: self.id().to_string(),
                    icon: self.icon().to_string(),
                    name: r.full_name,
                    language: r.language,
                    description: r.description,
                    url: r.html_url,
                    stars_today: None, // Gitea API doesn't provide daily stars
                    stars_total: r.stars_count,
                    last_activity,
                    topics: vec![], // Gitea API doesn't provide topics in search
                    is_starred: false,
                }
            })
            .collect();

        Ok(repos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gitea_provider_metadata() {
        let gitea = Gitea::new(6).unwrap();
        assert_eq!(gitea.id(), "gitea");
        assert_eq!(gitea.icon(), "[GE]");
    }

    #[tokio::test]
    async fn test_gitea_api() {
        // Use max_retries(0) to avoid retry delays in tests
        let http = HttpClient::builder()
            .timeout_secs(10)
            .max_retries(0)
            .build()
            .unwrap();
        let gitea = Gitea::with_client(http);
        let cfg = ProviderCfg {
            timeout_secs: 10,
            token: None,
            base_url: Some("https://gitea.com".to_string()),
            exclude_topics: vec![],
        };
        let filter = LanguageFilter::new(vec![]);

        // Try to fetch, but don't fail the test if API is down
        match gitea.top_today(&cfg, 0, 3, &filter).await {
            Ok(repos) => {
                // Verify structure if API call succeeds
                for repo in repos {
                    assert_eq!(repo.provider, "gitea");
                    assert_eq!(repo.icon, "[GE]");
                    assert!(repo.last_activity.is_some());
                }
            }
            Err(e) => {
                eprintln!("Gitea API test skipped: {e}");
            }
        }
    }
}

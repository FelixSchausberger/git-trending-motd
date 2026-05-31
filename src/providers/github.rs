use anyhow::Result;
use async_trait::async_trait;
use scraper::{Html, Selector};
use serde::Deserialize;

use crate::http::HttpClient;
use crate::model::{LanguageFilter, Provider, ProviderCfg, Repo};

/// GitHub provider using HTML scraping of trending page
pub struct GitHub {
    http: HttpClient,
}

struct TrendingRepo {
    name: String,
    description: Option<String>,
    language: Option<String>,
    stars_today: Option<u64>,
    stars_total: Option<u64>,
    url: String,
    topics: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubSearchResponse {
    items: Vec<GitHubRepository>,
}

#[derive(Debug, Deserialize)]
struct GitHubRepository {
    full_name: String,
    description: Option<String>,
    html_url: String,
    stargazers_count: u64,
    language: Option<String>,
    topics: Vec<String>,
    updated_at: String,
}

impl GitHub {
    pub fn new(timeout_secs: u64) -> Result<Self> {
        Ok(Self {
            http: HttpClient::new(timeout_secs)?,
        })
    }

    /// Create a GitHub provider with a custom `HttpClient`
    #[allow(dead_code)]
    pub fn with_client(http: HttpClient) -> Self {
        Self { http }
    }

    /// Fetch trending repositories from GitHub using Search API (provides topics)
    async fn fetch_trending_api(&self, token: Option<&str>) -> Result<Vec<GitHubRepository>> {
        // Search for repos created/updated in the last 7 days, sorted by stars
        let week_ago = (chrono::Utc::now() - chrono::Duration::days(7))
            .format("%Y-%m-%d")
            .to_string();

        let url = format!(
            "https://api.github.com/search/repositories?q=created:>={week_ago}&sort=stars&order=desc&per_page=100"
        );

        let response: GitHubSearchResponse = self.http.get_json(&url, token).await?;
        Ok(response.items)
    }

    /// Fetch trending repositories from GitHub by scraping the trending page
    async fn fetch_trending(&self, language: Option<&str>) -> Result<Vec<TrendingRepo>> {
        let url = if let Some(lang) = language {
            format!("https://github.com/trending/{lang}?since=daily")
        } else {
            "https://github.com/trending?since=daily".to_string()
        };

        let html = self.http.get_html(&url).await?;
        Self::parse_trending_html(&html)
    }

    /// Parse HTML from GitHub trending page
    fn parse_trending_html(html: &str) -> Result<Vec<TrendingRepo>> {
        let document = Html::parse_document(html);

        // Selectors for extracting repository data
        let article_selector = Selector::parse("article.Box-row").unwrap();
        let name_selector = Selector::parse("h2 a").unwrap();
        let desc_selector = Selector::parse("p").unwrap();
        let lang_selector = Selector::parse("span[itemprop='programmingLanguage']").unwrap();
        let star_selector = Selector::parse("span.d-inline-block.float-sm-right").unwrap();

        let mut repos = Vec::new();

        for article in document.select(&article_selector) {
            // Extract repository name and URL
            let Some(name_elem) = article.select(&name_selector).next() else {
                continue;
            };

            let Some(href) = name_elem.value().attr("href") else {
                continue;
            };

            let name = name_elem
                .text()
                .collect::<String>()
                .trim()
                .replace('\n', "")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join("");

            let url = format!("https://github.com{href}");

            // Extract description
            let description = article
                .select(&desc_selector)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string())
                .filter(|s| !s.is_empty());

            // Extract language
            let language = article
                .select(&lang_selector)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string());

            // Extract stars (today and total)
            let stars_text: Vec<String> = article
                .select(&star_selector)
                .map(|e| e.text().collect::<String>().trim().to_string())
                .collect();

            let stars_total = stars_text
                .iter()
                .find(|s| !s.contains("stars today"))
                .and_then(|s| {
                    s.replace(',', "")
                        .split_whitespace()
                        .next()
                        .and_then(|n| n.parse().ok())
                });

            let stars_today = stars_text
                .iter()
                .find(|s| s.contains("stars today"))
                .and_then(|s| {
                    let parts: Vec<&str> = s.split_whitespace().collect();
                    parts.first().and_then(|n| n.replace(',', "").parse().ok())
                });

            repos.push(TrendingRepo {
                name,
                description,
                language,
                stars_today,
                stars_total,
                url,
                topics: vec![], // HTML scraping doesn't provide topics
            });
        }

        if repos.is_empty() {
            anyhow::bail!("Failed to parse any repositories from GitHub trending page");
        }

        Ok(repos)
    }
}

#[async_trait]
impl Provider for GitHub {
    fn id(&self) -> &'static str {
        "github"
    }

    fn icon(&self) -> &'static str {
        "[GH]"
    }

    async fn top_today(
        &self,
        cfg: &ProviderCfg,
        offset: usize,
        limit: usize,
        langs: &LanguageFilter,
    ) -> Result<Vec<Repo>> {
        // Use API if topic exclusion is configured (API provides topics)
        if !cfg.exclude_topics.is_empty() {
            let api_repos = self.fetch_trending_api(cfg.token.as_deref()).await?;

            let repos = api_repos
                .into_iter()
                .filter(|r| {
                    // Filter by language
                    if !langs.matches(r.language.as_ref()) {
                        return false;
                    }
                    // Filter by excluded topics
                    !r.topics.iter().any(|topic| {
                        cfg.exclude_topics
                            .iter()
                            .any(|excluded| topic.eq_ignore_ascii_case(excluded))
                    })
                })
                .skip(offset)
                .take(limit)
                .map(|r| {
                    let last_activity = chrono::DateTime::parse_from_rfc3339(&r.updated_at)
                        .ok()
                        .map(|dt| dt.with_timezone(&chrono::Utc));

                    Repo {
                        provider: self.id().to_string(),
                        icon: self.icon().to_string(),
                        name: r.full_name,
                        language: r.language,
                        description: r.description,
                        url: r.html_url,
                        stars_today: None, // API doesn't provide daily stars
                        stars_total: Some(r.stargazers_count),
                        last_activity,
                        topics: r.topics,
                        is_starred: false,
                    }
                })
                .collect();

            return Ok(repos);
        }

        // Fall back to HTML scraping (original behavior)
        let trending = if langs.languages.is_empty() {
            self.fetch_trending(None).await?
        } else {
            // Try fetching for each language filter and combine results
            let mut all_repos = Vec::new();
            for lang in &langs.languages {
                if let Ok(repos) = self.fetch_trending(Some(lang)).await {
                    all_repos.extend(repos);
                }
            }
            all_repos
        };

        let repos = trending
            .into_iter()
            .filter(|r| langs.matches(r.language.as_ref()))
            .skip(offset)
            .take(limit)
            .map(|r| Repo {
                provider: self.id().to_string(),
                icon: self.icon().to_string(),
                name: r.name,
                language: r.language,
                description: r.description,
                url: r.url,
                stars_today: r.stars_today,
                stars_total: r.stars_total,
                last_activity: Some(chrono::Utc::now()), // GitHub trending = active today
                topics: r.topics,
                is_starred: false,
            })
            .collect();

        Ok(repos)
    }
}

impl GitHub {
    /// Star a repository on GitHub
    pub async fn star_repo(&self, owner: &str, repo: &str, token: &str) -> Result<()> {
        let url = format!("https://api.github.com/user/starred/{owner}/{repo}");
        self.http.put(&url, Some(token)).await?;
        Ok(())
    }

    /// Check if a repository is starred
    #[allow(dead_code)]
    pub async fn check_starred(&self, owner: &str, repo: &str, token: &str) -> Result<bool> {
        let url = format!("https://api.github.com/user/starred/{owner}/{repo}");
        self.http.head(&url, Some(token)).await
    }

    /// Get all starred repositories for the authenticated user
    pub async fn get_user_stars(&self, token: &str) -> Result<Vec<String>> {
        #[derive(Deserialize)]
        struct StarredRepo {
            full_name: String,
        }

        let mut all_repos = Vec::new();
        let mut page = 1;
        let per_page = 100;

        loop {
            let url =
                format!("https://api.github.com/user/starred?per_page={per_page}&page={page}");

            let repos: Vec<StarredRepo> = self.http.get_json(&url, Some(token)).await?;

            if repos.is_empty() {
                break;
            }

            all_repos.extend(repos.into_iter().map(|r| r.full_name));
            page += 1;

            // Safety limit to avoid infinite loops
            if page > 50 {
                break;
            }
        }

        Ok(all_repos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_github_provider_metadata() {
        let github = GitHub::new(6).unwrap();
        assert_eq!(github.id(), "github");
        assert_eq!(github.icon(), "[GH]");
    }

    #[tokio::test]
    async fn test_github_trending_api() {
        // This is an integration test that requires network access
        // Use max_retries(0) to avoid retry delays in tests
        let http = HttpClient::builder()
            .timeout_secs(10)
            .max_retries(0)
            .build()
            .unwrap();
        let github = GitHub::with_client(http);
        let cfg = ProviderCfg {
            timeout_secs: 10,
            token: None,
            base_url: None,
            exclude_topics: vec![],
        };
        let filter = LanguageFilter::new(vec![]);

        // Try to fetch, but don't fail the test if API is down
        match github.top_today(&cfg, 0, 3, &filter).await {
            Ok(repos) => {
                // Verify structure if API call succeeds
                for repo in repos {
                    assert_eq!(repo.provider, "github");
                    assert_eq!(repo.icon, "[GH]");
                    assert!(!repo.name.is_empty());
                }
            }
            Err(e) => {
                // API might be down or rate-limited, that's okay for this test
                eprintln!("GitHub API test skipped: {e}");
            }
        }
    }
}

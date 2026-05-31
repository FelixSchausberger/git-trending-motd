use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::model::Repo;

/// Cache entry with timestamp
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    timestamp: u64,
    repos: Vec<Repo>,
}

/// Filesystem-based cache with TTL support
pub struct Cache {
    cache_dir: PathBuf,
    ttl_secs: u64,
}

impl Cache {
    /// Create a new cache instance
    pub fn new(ttl_mins: u64) -> Result<Self> {
        let cache_dir = dirs::cache_dir()
            .context("Failed to determine cache directory")?
            .join("git-trending-motd");

        Ok(Self {
            cache_dir,
            ttl_secs: ttl_mins * 60,
        })
    }

    /// Create a cache instance with a custom directory (for testing)
    #[cfg(test)]
    fn with_dir(cache_dir: PathBuf, ttl_mins: u64) -> Self {
        Self {
            cache_dir,
            ttl_secs: ttl_mins * 60,
        }
    }

    /// Get cache file path for a provider
    fn cache_file(&self, provider: &str) -> PathBuf {
        self.cache_dir.join(format!("{provider}.json"))
    }

    /// Get current timestamp in seconds
    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Load cached repositories for a provider
    pub async fn get(&self, provider: &str) -> Option<Vec<Repo>> {
        let cache_file = self.cache_file(provider);

        if !cache_file.exists() {
            return None;
        }

        let content = tokio::fs::read_to_string(&cache_file).await.ok()?;
        let entry: CacheEntry = serde_json::from_str(&content).ok()?;

        // Check if cache is still valid
        let age = Self::now().saturating_sub(entry.timestamp);
        if age > self.ttl_secs {
            return None;
        }

        Some(entry.repos)
    }

    /// Save repositories to cache for a provider
    pub async fn set(&self, provider: &str, repos: Vec<Repo>) -> Result<()> {
        // Ensure cache directory exists
        tokio::fs::create_dir_all(&self.cache_dir)
            .await
            .with_context(|| {
                format!(
                    "Failed to create cache directory: {}",
                    self.cache_dir.display()
                )
            })?;

        let entry = CacheEntry {
            timestamp: Self::now(),
            repos,
        };

        let content =
            serde_json::to_string_pretty(&entry).context("Failed to serialize cache entry")?;

        let cache_file = self.cache_file(provider);
        tokio::fs::write(&cache_file, content)
            .await
            .with_context(|| format!("Failed to write cache file: {}", cache_file.display()))?;

        Ok(())
    }

    /// Clear cache for a specific provider
    #[allow(dead_code)]
    pub async fn clear(&self, provider: &str) -> Result<()> {
        let cache_file = self.cache_file(provider);
        if cache_file.exists() {
            tokio::fs::remove_file(&cache_file).await.with_context(|| {
                format!("Failed to remove cache file: {}", cache_file.display())
            })?;
        }
        Ok(())
    }

    /// Clear all cached data
    #[allow(dead_code)]
    pub async fn clear_all(&self) -> Result<()> {
        if self.cache_dir.exists() {
            tokio::fs::remove_dir_all(&self.cache_dir)
                .await
                .with_context(|| {
                    format!(
                        "Failed to remove cache directory: {}",
                        self.cache_dir.display()
                    )
                })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_roundtrip() {
        // Use temporary directory for testing
        let temp_dir = std::env::temp_dir().join(format!("git-trending-motd-test-{}", Cache::now()));
        let cache = Cache::with_dir(temp_dir.clone(), 60);

        let test_repos = vec![Repo {
            provider: "github".to_string(),
            icon: "[GH]".to_string(),
            name: "test/repo".to_string(),
            language: Some("Rust".to_string()),
            description: Some("Test repository".to_string()),
            url: "https://github.com/test/repo".to_string(),
            stars_today: Some(10),
            stars_total: Some(100),
            last_activity: Some(chrono::Utc::now()),
            topics: vec!["rust".to_string(), "cli".to_string()],
            is_starred: false,
        }];

        // Clear any existing cache
        let _ = cache.clear("test-provider").await;

        // Initially no cache
        assert!(cache.get("test-provider").await.is_none());

        // Save to cache
        cache
            .set("test-provider", test_repos.clone())
            .await
            .unwrap();

        // Retrieve from cache
        let cached = cache.get("test-provider").await.unwrap();
        assert_eq!(cached.len(), 1);
        assert_eq!(cached[0].name, "test/repo");

        // Cleanup
        cache.clear("test-provider").await.unwrap();
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_cache_expiry() {
        // Use temporary directory for testing
        let temp_dir = std::env::temp_dir().join(format!("git-trending-motd-test-{}", Cache::now()));
        // Create cache with 0 minute TTL (expires immediately)
        let cache = Cache::with_dir(temp_dir.clone(), 0);

        let test_repos = vec![Repo {
            provider: "github".to_string(),
            icon: "[GH]".to_string(),
            name: "test/repo".to_string(),
            language: Some("Rust".to_string()),
            description: None,
            url: "https://github.com/test/repo".to_string(),
            stars_today: None,
            stars_total: Some(50),
            last_activity: Some(chrono::Utc::now()),
            topics: vec![],
            is_starred: false,
        }];

        // Clear any existing cache
        let _ = cache.clear("test-expiry").await;

        // Save to cache
        cache.set("test-expiry", test_repos).await.unwrap();

        // With 0 TTL, cache should be expired after at least 1 second
        // (cache uses seconds granularity, so we need to wait at least 1 second)
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        assert!(cache.get("test-expiry").await.is_none());

        // Cleanup
        let _ = cache.clear("test-expiry").await;
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}

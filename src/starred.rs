use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

/// Starred repositories cache with timestamp
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StarredEntry {
    timestamp: u64,
    starred_repos: HashSet<String>, // Set of "owner/repo" names
}

/// Filesystem-based starred status cache
pub struct StarredCache {
    cache_file: PathBuf,
    ttl_secs: u64,
}

impl StarredCache {
    /// Create a new starred cache instance (1 hour TTL)
    pub fn new() -> Result<Self> {
        let cache_dir = dirs::cache_dir()
            .context("Failed to determine cache directory")?
            .join("git-trending-motd");

        Ok(Self {
            cache_file: cache_dir.join("starred.json"),
            ttl_secs: 3600, // 1 hour cache
        })
    }

    /// Get current timestamp in seconds
    fn now() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Load starred repositories from cache
    pub async fn get_starred(&self) -> Option<HashSet<String>> {
        if !self.cache_file.exists() {
            return None;
        }

        let content = tokio::fs::read_to_string(&self.cache_file).await.ok()?;
        let entry: StarredEntry = serde_json::from_str(&content).ok()?;

        // Check if cache is still valid
        let age = Self::now().saturating_sub(entry.timestamp);
        if age > self.ttl_secs {
            return None;
        }

        Some(entry.starred_repos)
    }

    /// Save starred repositories to cache
    pub async fn save_starred(&self, starred_repos: HashSet<String>) -> Result<()> {
        // Ensure cache directory exists
        if let Some(parent) = self.cache_file.parent() {
            tokio::fs::create_dir_all(parent).await.with_context(|| {
                format!("Failed to create cache directory: {}", parent.display())
            })?;
        }

        let entry = StarredEntry {
            timestamp: Self::now(),
            starred_repos,
        };

        let content =
            serde_json::to_string_pretty(&entry).context("Failed to serialize starred entry")?;

        tokio::fs::write(&self.cache_file, content)
            .await
            .with_context(|| {
                format!(
                    "Failed to write starred file: {}",
                    self.cache_file.display()
                )
            })?;

        Ok(())
    }

    /// Check if a repository is starred
    #[cfg(test)]
    pub async fn is_starred(&self, repo_name: &str) -> bool {
        if let Some(starred) = self.get_starred().await {
            starred.contains(repo_name)
        } else {
            false
        }
    }

    /// Clear starred cache
    #[allow(dead_code)]
    pub async fn clear(&self) -> Result<()> {
        if self.cache_file.exists() {
            tokio::fs::remove_file(&self.cache_file)
                .await
                .with_context(|| {
                    format!(
                        "Failed to remove starred file: {}",
                        self.cache_file.display()
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
    async fn test_starred_cache_roundtrip() {
        let temp_dir =
            std::env::temp_dir().join(format!("git-trending-motd-starred-test-{}", StarredCache::now()));
        let cache = StarredCache {
            cache_file: temp_dir.join("starred.json"),
            ttl_secs: 3600,
        };

        // Initially no starred repos
        assert!(cache.get_starred().await.is_none());

        // Save some starred repos
        let mut starred = HashSet::new();
        starred.insert("owner1/repo1".to_string());
        starred.insert("owner2/repo2".to_string());
        cache.save_starred(starred.clone()).await.unwrap();

        // Should retrieve from cache
        let cached = cache.get_starred().await.unwrap();
        assert_eq!(cached.len(), 2);
        assert!(cached.contains("owner1/repo1"));
        assert!(cached.contains("owner2/repo2"));

        // Check is_starred
        assert!(cache.is_starred("owner1/repo1").await);
        assert!(!cache.is_starred("owner3/repo3").await);

        // Cleanup
        let _ = cache.clear().await;
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_starred_cache_expiry() {
        let temp_dir =
            std::env::temp_dir().join(format!("git-trending-motd-starred-expiry-{}", StarredCache::now()));
        let cache = StarredCache {
            cache_file: temp_dir.join("starred.json"),
            ttl_secs: 0, // Immediate expiry
        };

        // Save some starred repos
        let mut starred = HashSet::new();
        starred.insert("owner1/repo1".to_string());
        cache.save_starred(starred).await.unwrap();

        // Wait for expiry
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        // Should be expired
        assert!(cache.get_starred().await.is_none());

        // Cleanup
        let _ = cache.clear().await;
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_starred_cache_loads_after_save() {
        // This test verifies cache survives "process restart" simulation
        let temp_dir =
            std::env::temp_dir().join(format!("git-trending-motd-starred-persist-{}", StarredCache::now()));
        let cache_file = temp_dir.join("starred.json");

        // Create first cache instance and save data
        let cache1 = StarredCache {
            cache_file: cache_file.clone(),
            ttl_secs: 3600,
        };

        let mut starred = HashSet::new();
        starred.insert("rust-lang/rust".to_string());
        starred.insert("tokio-rs/tokio".to_string());
        cache1.save_starred(starred).await.unwrap();

        // Simulate process restart by creating new cache instance with same file
        let cache2 = StarredCache {
            cache_file,
            ttl_secs: 3600,
        };

        // Should load the data from file
        let loaded = cache2.get_starred().await;
        assert!(loaded.is_some(), "Cache should persist across instances");

        let starred_set = loaded.unwrap();
        assert_eq!(starred_set.len(), 2);
        assert!(starred_set.contains("rust-lang/rust"));
        assert!(starred_set.contains("tokio-rs/tokio"));

        // Cleanup
        let _ = cache2.clear().await;
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}

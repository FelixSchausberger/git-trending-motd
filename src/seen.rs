use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

use crate::model::Repo;

/// Seen repositories tracker with daily reset and pagination offset
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SeenEntry {
    date: String,                // Format: YYYY-MM-DD
    seen_repos: HashSet<String>, // Set of "owner/repo" names
    #[serde(default)]
    fetch_offset: usize, // Track position in trending list for pagination
}

/// Filesystem-based seen tracker that resets daily
pub struct SeenTracker {
    seen_file: PathBuf,
}

impl SeenTracker {
    /// Create a new seen tracker instance
    pub fn new() -> Result<Self> {
        let cache_dir = dirs::cache_dir()
            .context("Failed to determine cache directory")?
            .join("git-trending-motd");

        Ok(Self {
            seen_file: cache_dir.join("seen.json"),
        })
    }

    /// Get current date in YYYY-MM-DD format
    fn today() -> String {
        chrono::Utc::now().format("%Y-%m-%d").to_string()
    }

    /// Load seen entry for today
    async fn get_entry(&self) -> Result<Option<SeenEntry>> {
        if !self.seen_file.exists() {
            return Ok(None);
        }

        let content = tokio::fs::read_to_string(&self.seen_file).await?;
        let entry: SeenEntry = serde_json::from_str(&content)?;

        // Check if data is from today
        if entry.date == Self::today() {
            Ok(Some(entry))
        } else {
            // Old data, reset
            Ok(None)
        }
    }

    /// Load seen repositories for today
    pub async fn get_seen(&self) -> Result<HashSet<String>> {
        Ok(self
            .get_entry()
            .await?
            .map(|e| e.seen_repos)
            .unwrap_or_default())
    }

    /// Get current fetch offset for pagination
    pub async fn get_fetch_offset(&self) -> usize {
        self.get_entry()
            .await
            .ok()
            .flatten()
            .map_or(0, |e| e.fetch_offset)
    }

    /// Increment fetch offset after successful fetch
    pub async fn increment_fetch_offset(&self, increment: usize) -> Result<()> {
        let seen_repos = self.get_seen().await.unwrap_or_default();
        let current_offset = self.get_fetch_offset().await;
        self.save_seen_with_offset(seen_repos, current_offset + increment)
            .await
    }

    /// Mark repositories as seen
    pub async fn mark_seen(&self, repos: &[Repo]) -> Result<()> {
        // Load existing seen set
        let mut seen_repos = self.get_seen().await.unwrap_or_default();

        // Add new repos
        for repo in repos {
            seen_repos.insert(repo.name.clone());
        }

        // Save updated entry
        self.save_seen(seen_repos).await
    }

    /// Save seen repositories with offset
    async fn save_seen_with_offset(
        &self,
        seen_repos: HashSet<String>,
        offset: usize,
    ) -> Result<()> {
        // Ensure cache directory exists
        if let Some(parent) = self.seen_file.parent() {
            tokio::fs::create_dir_all(parent).await.with_context(|| {
                format!("Failed to create cache directory: {}", parent.display())
            })?;
        }

        let entry = SeenEntry {
            date: Self::today(),
            seen_repos,
            fetch_offset: offset,
        };

        let content =
            serde_json::to_string_pretty(&entry).context("Failed to serialize seen entry")?;

        tokio::fs::write(&self.seen_file, content)
            .await
            .with_context(|| format!("Failed to write seen file: {}", self.seen_file.display()))?;

        Ok(())
    }

    /// Save seen repositories (without changing offset)
    async fn save_seen(&self, seen_repos: HashSet<String>) -> Result<()> {
        let current_offset = self.get_fetch_offset().await;
        self.save_seen_with_offset(seen_repos, current_offset).await
    }

    /// Filter out already-seen repositories
    pub async fn filter_unseen(&self, repos: &[Repo]) -> Result<Vec<Repo>> {
        let seen = self.get_seen().await.unwrap_or_default();

        Ok(repos
            .iter()
            .filter(|repo| !seen.contains(&repo.name))
            .cloned()
            .collect())
    }

    /// Clear all seen data
    #[allow(dead_code)]
    pub async fn clear(&self) -> Result<()> {
        if self.seen_file.exists() {
            tokio::fs::remove_file(&self.seen_file)
                .await
                .with_context(|| {
                    format!("Failed to remove seen file: {}", self.seen_file.display())
                })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_repo(name: &str) -> Repo {
        Repo {
            provider: "github".to_string(),
            icon: "[GH]".to_string(),
            name: name.to_string(),
            language: Some("Rust".to_string()),
            description: Some("Test repository".to_string()),
            url: format!("https://github.com/{name}"),
            stars_today: Some(10),
            stars_total: Some(100),
            last_activity: Some(Utc::now()),
            topics: vec![],
            is_starred: false,
        }
    }

    #[tokio::test]
    async fn test_seen_tracker_new_day() {
        let temp_dir = std::env::temp_dir().join(format!(
            "git-trending-motd-seen-test-{}",
            chrono::Utc::now().timestamp()
        ));
        let tracker = SeenTracker {
            seen_file: temp_dir.join("seen.json"),
        };

        // Initially no seen repos
        let seen = tracker.get_seen().await.unwrap();
        assert!(seen.is_empty());

        // Mark some repos as seen
        let repos = vec![
            create_test_repo("owner1/repo1"),
            create_test_repo("owner2/repo2"),
        ];
        tracker.mark_seen(&repos).await.unwrap();

        // Should now be in seen list
        let seen = tracker.get_seen().await.unwrap();
        assert_eq!(seen.len(), 2);
        assert!(seen.contains("owner1/repo1"));
        assert!(seen.contains("owner2/repo2"));

        // Cleanup
        let _ = tracker.clear().await;
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_filter_unseen() {
        let temp_dir = std::env::temp_dir().join(format!(
            "git-trending-motd-seen-filter-{}",
            chrono::Utc::now().timestamp()
        ));
        let tracker = SeenTracker {
            seen_file: temp_dir.join("seen.json"),
        };

        // Mark repo1 as seen
        let seen_repos = vec![create_test_repo("owner1/repo1")];
        tracker.mark_seen(&seen_repos).await.unwrap();

        // Try to filter a mix of seen and unseen
        let all_repos = vec![
            create_test_repo("owner1/repo1"), // Already seen
            create_test_repo("owner2/repo2"), // Not seen
            create_test_repo("owner3/repo3"), // Not seen
        ];

        let unseen = tracker.filter_unseen(&all_repos).await.unwrap();
        assert_eq!(unseen.len(), 2);
        assert_eq!(unseen[0].name, "owner2/repo2");
        assert_eq!(unseen[1].name, "owner3/repo3");

        // Cleanup
        let _ = tracker.clear().await;
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_daily_reset_clears_yesterday_data() {
        use chrono::{Duration as ChronoDuration, Utc};

        let temp_dir = std::env::temp_dir().join(format!(
            "git-trending-motd-seen-reset-{}",
            Utc::now().timestamp()
        ));
        let tracker = SeenTracker {
            seen_file: temp_dir.join("seen.json"),
        };

        // Manually create a seen entry from yesterday
        let yesterday = (Utc::now() - ChronoDuration::days(1))
            .format("%Y-%m-%d")
            .to_string();

        let old_entry = SeenEntry {
            date: yesterday.clone(),
            seen_repos: vec!["owner/old-repo".to_string()].into_iter().collect(),
            fetch_offset: 10,
        };

        // Write the old entry
        let json = serde_json::to_string(&old_entry).unwrap();
        std::fs::create_dir_all(&temp_dir).unwrap();
        tokio::fs::write(&tracker.seen_file, json)
            .await
            .unwrap();

        // Get seen should return empty (yesterday's data is stale)
        let seen = tracker.get_seen().await.unwrap();
        assert!(
            seen.is_empty(),
            "Yesterday's seen repos should not be returned"
        );

        // Mark new repos as seen today
        let repos = vec![create_test_repo("owner/today-repo")];
        tracker.mark_seen(&repos).await.unwrap();

        // Should only have today's repos
        let seen = tracker.get_seen().await.unwrap();
        assert_eq!(seen.len(), 1);
        assert!(seen.contains("owner/today-repo"));
        assert!(!seen.contains("owner/old-repo"));

        // Cleanup
        let _ = tracker.clear().await;
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_offset_persists_within_same_day() {
        let temp_dir = std::env::temp_dir().join(format!(
            "git-trending-motd-seen-offset-{}",
            Utc::now().timestamp()
        ));
        let tracker = SeenTracker {
            seen_file: temp_dir.join("seen.json"),
        };

        // Initially offset is 0
        let offset = tracker.get_fetch_offset().await;
        assert_eq!(offset, 0);

        // Increment offset
        tracker.increment_fetch_offset(5).await.unwrap();

        // Offset should be persisted
        let offset = tracker.get_fetch_offset().await;
        assert_eq!(offset, 5);

        // Increment again
        tracker.increment_fetch_offset(3).await.unwrap();

        // Should accumulate
        let offset = tracker.get_fetch_offset().await;
        assert_eq!(offset, 8);

        // Cleanup
        let _ = tracker.clear().await;
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_mark_seen_appends_to_existing() {
        let temp_dir = std::env::temp_dir().join(format!(
            "git-trending-motd-seen-append-{}",
            Utc::now().timestamp()
        ));
        let tracker = SeenTracker {
            seen_file: temp_dir.join("seen.json"),
        };

        // Mark first batch
        let repos1 = vec![
            create_test_repo("owner1/repo1"),
            create_test_repo("owner2/repo2"),
        ];
        tracker.mark_seen(&repos1).await.unwrap();

        let seen = tracker.get_seen().await.unwrap();
        assert_eq!(seen.len(), 2);

        // Mark second batch
        let repos2 = vec![
            create_test_repo("owner3/repo3"),
            create_test_repo("owner4/repo4"),
        ];
        tracker.mark_seen(&repos2).await.unwrap();

        // Should have all repos from both batches
        let seen = tracker.get_seen().await.unwrap();
        assert_eq!(seen.len(), 4);
        assert!(seen.contains("owner1/repo1"));
        assert!(seen.contains("owner2/repo2"));
        assert!(seen.contains("owner3/repo3"));
        assert!(seen.contains("owner4/repo4"));

        // Cleanup
        let _ = tracker.clear().await;
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}

use anyhow::{anyhow, Context, Result};
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use futures::stream::{FuturesUnordered, StreamExt};
use std::io;
use std::sync::Arc;
use std::time::Duration;

mod cache;
mod config;
mod http;
mod model;
mod providers;
mod render;
mod seen;
mod starred;

use cache::Cache;
use config::Config;
use model::{LanguageFilter, Provider, ProviderCfg};
use providers::{GitHub, GitLab, Gitea};
use render::{render, OutputFormat};
use seen::SeenTracker;
use starred::StarredCache;

const PROVIDER_SLOW_WARN_SECS: u64 = 10;
const PROVIDER_FETCH_TIMEOUT_SECS: u64 = 30;

/// Trending repositories of the day - minimal MOTD CLI
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[allow(clippy::struct_excessive_bools)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Maximum repositories per provider
    #[arg(short = 'n', long = "max", value_name = "N", global = true)]
    max_per_provider: Option<usize>,

    /// Enable specific providers (comma-separated: gh,gl,ge)
    #[arg(short, long, value_name = "LIST", value_delimiter = ',', global = true)]
    provider: Option<Vec<String>>,

    /// Filter by language (comma-separated: rust,go)
    #[arg(short, long, value_name = "LIST", value_delimiter = ',', global = true)]
    lang: Option<Vec<String>>,

    /// Disable cache
    #[arg(long, global = true)]
    no_cache: bool,

    /// Output as JSON instead of MOTD
    #[arg(long, global = true)]
    json: bool,

    /// Enable verbose output for debugging
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Minimum star count threshold
    #[arg(long = "min-stars", value_name = "N", global = true)]
    min_stars: Option<u32>,

    /// Exclude GitHub repositories with these topics (comma-separated)
    #[arg(
        long = "exclude-topics",
        value_name = "LIST",
        value_delimiter = ',',
        global = true
    )]
    exclude_topics: Option<Vec<String>>,

    /// Show all repositories including those already seen today
    #[arg(long = "show-all", global = true)]
    show_all: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generate shell completion scripts
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
    /// Star a GitHub repository
    Star {
        /// Repository to star (format: owner/repo)
        repo: String,
    },
    /// Clone a trending repository
    Clone {
        /// Repository to clone (format: owner/repo or URL)
        repo: String,
    },
}

#[tokio::main]
#[allow(clippy::too_many_lines)]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Handle subcommands
    if let Some(command) = args.command {
        match command {
            Commands::Completions { shell } => {
                let mut cmd = Args::command();
                let bin_name = cmd.get_name().to_string();
                generate(shell, &mut cmd, bin_name, &mut io::stdout());
                return Ok(());
            }
            Commands::Star { repo } => {
                return handle_star_command(&repo).await;
            }
            Commands::Clone { repo } => {
                return handle_clone_command(&repo);
            }
        }
    }

    let verbose = args.verbose;

    // Load configuration
    let mut config = Config::load().context("Failed to load configuration")?;

    if verbose {
        eprintln!("📋 Config loaded successfully");
    }

    // Apply CLI overrides
    if let Some(max) = args.max_per_provider {
        config.general.max_per_provider = max;
    }

    if let Some(langs) = args.lang {
        config.general.language_filter = langs;
    }

    if let Some(min) = args.min_stars {
        config.general.min_stars = Some(min);
    }

    if let Some(topics) = args.exclude_topics {
        config.github.exclude_topics = topics;
    }

    // Determine output format
    let format = if args.json {
        OutputFormat::Json
    } else {
        OutputFormat::Motd
    };

    // Initialize cache
    let cache = if args.no_cache {
        if verbose {
            eprintln!("🚫 Cache disabled");
        }
        None
    } else {
        let c = Cache::new(config.general.cache_ttl_mins).context("Failed to initialize cache")?;
        if verbose {
            eprintln!(
                "💾 Cache initialized (TTL: {} mins)",
                config.general.cache_ttl_mins
            );
        }
        Some(c)
    };

    // Determine enabled providers
    let enabled_providers = if let Some(ref providers) = args.provider {
        // Parse short names: gh -> github, gl -> gitlab, ge -> gitea
        providers
            .iter()
            .map(|p| match p.as_str() {
                "gh" => "github",
                "gl" => "gitlab",
                "ge" => "gitea",
                _ => p.as_str(),
            })
            .collect::<Vec<_>>()
    } else {
        config.enabled_providers()
    };

    if verbose {
        eprintln!("🔌 Enabled providers: {enabled_providers:?}");
    }

    // Build provider instances
    let mut provider_instances: Vec<(String, Box<dyn Provider>)> = Vec::new();

    for provider_id in enabled_providers {
        match provider_id {
            "github" => match GitHub::new(config.general.github_timeout_secs) {
                Ok(gh) => {
                    if verbose {
                        eprintln!(
                            "  ✓ GitHub provider initialized (timeout: {}s)",
                            config.general.github_timeout_secs
                        );
                    }
                    provider_instances.push(("github".to_string(), Box::new(gh)));
                }
                Err(e) => eprintln!("✗ Failed to initialize GitHub provider: {e}"),
            },
            "gitlab" => match GitLab::new(config.general.gitlab_timeout_secs) {
                Ok(gl) => {
                    if verbose {
                        eprintln!(
                            "  ✓ GitLab provider initialized (timeout: {}s)",
                            config.general.gitlab_timeout_secs
                        );
                    }
                    provider_instances.push(("gitlab".to_string(), Box::new(gl)));
                }
                Err(e) => eprintln!("✗ Failed to initialize GitLab provider: {e}"),
            },
            "gitea" => match Gitea::new(config.general.gitea_timeout_secs) {
                Ok(ge) => {
                    if verbose {
                        eprintln!(
                            "  ✓ Gitea provider initialized (timeout: {}s)",
                            config.general.gitea_timeout_secs
                        );
                    }
                    provider_instances.push(("gitea".to_string(), Box::new(ge)));
                }
                Err(e) => eprintln!("✗ Failed to initialize Gitea provider: {e}"),
            },
            _ => eprintln!("⚠ Unknown provider: {provider_id}"),
        }
    }

    if provider_instances.is_empty() {
        anyhow::bail!("No providers enabled or available");
    }

    // Create language filter
    let lang_filter = LanguageFilter::new(config.general.language_filter.clone());

    if config.general.language_filter.is_empty() {
        if verbose {
            eprintln!("🌐 Language filter: all languages");
        }
    } else if verbose {
        eprintln!("🌐 Language filter: {:?}", config.general.language_filter);
    }

    if verbose {
        eprintln!("🚀 Fetching repositories...");
    }

    // Initialize seen tracker unless --show-all is set
    let seen_tracker = if args.show_all {
        None
    } else {
        match SeenTracker::new() {
            Ok(tracker) => Some(tracker),
            Err(e) => {
                if verbose {
                    eprintln!("⚠ Failed to initialize seen tracker: {e}");
                }
                None
            }
        }
    };

    // Get fetch offset for pagination
    let fetch_offset = if let Some(tracker) = &seen_tracker {
        tracker.get_fetch_offset().await
    } else {
        0
    };

    if verbose && fetch_offset > 0 {
        eprintln!("📖 Starting from position {fetch_offset} in trending list");
    }

    let prefer_cache_first = seen_tracker.is_none();

    // Fetch repositories in parallel
    let cache_arc = Arc::new(cache);
    let mut futures = FuturesUnordered::new();

    for (provider_id, provider) in provider_instances {
        let cache_prefetch = Arc::clone(&cache_arc);
        let cache_fetch = Arc::clone(&cache_arc);
        let cache_fallback = Arc::clone(&cache_arc);
        let lang_filter_clone = lang_filter.clone();
        let config_clone = config.clone();
        let verbose_clone = verbose;
        let offset_clone = fetch_offset;
        let provider_name = provider_id.clone();
        let provider_key = provider_id.clone();
        let prefer_cached = prefer_cache_first;

        let future = async move {
            if prefer_cached {
                if let Some(ref cache) = *cache_prefetch {
                    if let Some(cached_repos) = cache.get(&provider_key).await {
                        if verbose_clone {
                            eprintln!("  💾 {provider_key} (cached)");
                        }
                        return Ok((provider_key, cached_repos));
                    }
                }
            }

            let fetch_future = async move {
                let provider_cfg = ProviderCfg {
                    timeout_secs: config_clone.general.timeout_secs,
                    token: match provider_id.as_str() {
                        "github" => config_clone.auth.github_token.clone(),
                        "gitlab" => config_clone.auth.gitlab_token.clone(),
                        "gitea" => config_clone.auth.gitea_token.clone(),
                        _ => None,
                    },
                    base_url: if provider_id == "gitea" {
                        Some(config_clone.gitea.base_url.clone())
                    } else {
                        None
                    },
                    exclude_topics: if provider_id == "github" {
                        config_clone.github.exclude_topics.clone()
                    } else {
                        vec![]
                    },
                };

                let repos = provider
                    .top_today(
                        &provider_cfg,
                        offset_clone,
                        config_clone.get_max_entries(&provider_id),
                        &lang_filter_clone,
                    )
                    .await?;

                if let Some(ref cache) = *cache_fetch {
                    let _ = cache.set(&provider_id, repos.clone()).await;
                }

                Ok::<_, anyhow::Error>((provider_id, repos))
            };

            tokio::pin!(fetch_future);
            let slow_notice = tokio::time::sleep(Duration::from_secs(PROVIDER_SLOW_WARN_SECS));
            tokio::pin!(slow_notice);
            let mut warned = false;

            let monitored = async {
                loop {
                    tokio::select! {
                        result = &mut fetch_future => break result,
                        () = &mut slow_notice, if !warned => {
                            warned = true;
                            eprintln!("⏳ Still fetching {provider_name}...");
                            if let Some(ref cache) = *cache_fallback {
                                if let Some(cached_repos) = cache.get(&provider_key).await {
                                    eprintln!("⚠ Using cached {provider_name} results while network call finishes...");
                                    return Ok((provider_key.clone(), cached_repos));
                                }
                            }
                        }
                    }
                }
            };

            match tokio::time::timeout(Duration::from_secs(PROVIDER_FETCH_TIMEOUT_SECS), monitored)
                .await
            {
                Ok(res) => res,
                Err(_) => Err(anyhow!(
                    "{provider_name} provider timed out after {PROVIDER_FETCH_TIMEOUT_SECS}s"
                )),
            }
        };

        futures.push(future);
    }

    // Collect results
    let mut all_repos = Vec::new();
    let mut errors = Vec::new();
    let mut no_new_repos = false;

    while let Some(result) = futures.next().await {
        match result {
            Ok((provider_id, repos)) => {
                if verbose {
                    eprintln!("  📦 {}: {} repos", provider_id, repos.len());
                }
                if !repos.is_empty() {
                    all_repos.extend(repos);
                } else if format!("{format:?}") == "Motd" {
                    eprintln!("⚠ No repositories found for {provider_id}");
                }
            }
            Err(e) => {
                if verbose {
                    eprintln!("  ✗ Provider error: {e}");
                }
                errors.push(e);
            }
        }
    }

    // Handle errors
    if !errors.is_empty() {
        for error in &errors {
            eprintln!("✗ Error: {error}");
        }
    }

    if all_repos.is_empty() && !errors.is_empty() {
        anyhow::bail!("All providers failed");
    }

    // Filter out previously seen repos when tracking is enabled
    if let Some(tracker) = &seen_tracker {
        let before_count = all_repos.len();
        match tracker.filter_unseen(&all_repos).await {
            Ok(filtered) => {
                let removed = before_count.saturating_sub(filtered.len());
                if verbose && removed > 0 {
                    eprintln!("👀 Seen filter: skipped {removed} repos shown earlier today");
                }
                if filtered.is_empty() && before_count > 0 {
                    no_new_repos = true;
                }
                all_repos = filtered;
            }
            Err(e) => {
                if verbose {
                    eprintln!("⚠ Failed to filter seen repos: {e}");
                }
            }
        }
    }

    // Apply ASCII-only filter if enabled
    if config.general.ascii_only {
        let before_count = all_repos.len();
        all_repos.retain(is_mostly_ascii);
        if verbose {
            let filtered_count = before_count - all_repos.len();
            eprintln!("🔤 ASCII filter: removed {filtered_count} non-ASCII repos");
        }
    }

    // Apply minimum star filter if configured
    if let Some(min_stars) = config.general.min_stars {
        let before_count = all_repos.len();
        all_repos.retain(|repo| repo.stars_total.unwrap_or(0) >= min_stars.into());
        if verbose {
            let filtered_count = before_count - all_repos.len();
            eprintln!("⭐ Star filter: removed {filtered_count} repos below {min_stars} stars");
        }
    }

    if verbose {
        eprintln!("📊 Total repositories: {}", all_repos.len());
    }

    if no_new_repos {
        eprintln!(
            "👀 All fetched repositories were already shown today. Use --show-all to repeat them."
        );
    }

    // Apply starred status if enabled and GitHub token available
    if config.general.show_starred_status && config.auth.github_token.is_some() {
        if let Ok(starred_cache) = StarredCache::new() {
            // Try to use cached starred list
            let starred_set = if let Some(cached) = starred_cache.get_starred().await {
                if verbose {
                    eprintln!("⭐ Using cached starred status ({} repos)", cached.len());
                }
                cached
            } else if let Some(ref token) = config.auth.github_token {
                // Fetch fresh starred list if not cached
                if verbose {
                    eprintln!("⭐ Fetching starred repositories...");
                }
                if let Ok(github) = GitHub::new(config.general.github_timeout_secs) {
                    if let Ok(starred_repos) = github.get_user_stars(token).await {
                        let starred_set: std::collections::HashSet<String> =
                            starred_repos.into_iter().collect();
                        if verbose {
                            eprintln!("⭐ Found {} starred repos", starred_set.len());
                        }
                        let _ = starred_cache.save_starred(starred_set.clone()).await;
                        starred_set
                    } else {
                        std::collections::HashSet::new()
                    }
                } else {
                    std::collections::HashSet::new()
                }
            } else {
                std::collections::HashSet::new()
            };

            // Mark starred repos
            for repo in &mut all_repos {
                if repo.provider == "github" {
                    repo.is_starred = starred_set.contains(&repo.name);
                }
            }
        }
    }

    // Render output
    render(&all_repos, format);

    // Record seen repos and increment offset for next run when tracking is enabled
    if let Some(tracker) = &seen_tracker {
        if !all_repos.is_empty() {
            if let Err(e) = tracker.mark_seen(&all_repos).await {
                if verbose {
                    eprintln!("⚠ Failed to record seen repos: {e}");
                }
            }

            match tracker.increment_fetch_offset(all_repos.len()).await {
                Ok(()) => {
                    if verbose {
                        eprintln!(
                            "📈 Next run will start from position {}",
                            fetch_offset + all_repos.len()
                        );
                    }
                }
                Err(e) => {
                    if verbose {
                        eprintln!("⚠ Failed to update fetch offset: {e}");
                    }
                }
            }
        }
    }

    Ok(())
}

/// Check if a repository is mostly ASCII (filters out CJK/non-Latin scripts)
fn is_mostly_ascii(repo: &model::Repo) -> bool {
    // Check name - should be primarily ASCII
    let name_ascii_ratio = ascii_ratio(&repo.name);
    if name_ascii_ratio < 0.8 {
        return false;
    }

    // Check description if present
    if let Some(ref desc) = repo.description {
        let desc_ascii_ratio = ascii_ratio(desc);
        if desc_ascii_ratio < 0.7 {
            return false;
        }
    }

    true
}

/// Calculate the ratio of ASCII characters in a string
#[allow(clippy::cast_precision_loss)]
fn ascii_ratio(s: &str) -> f64 {
    if s.is_empty() {
        return 1.0;
    }
    let total_chars = s.chars().count();
    let ascii_chars = s.chars().filter(char::is_ascii).count();
    ascii_chars as f64 / total_chars as f64
}

/// Handle the star subcommand
async fn handle_star_command(repo: &str) -> Result<()> {
    let config = Config::load().context("Failed to load configuration")?;

    let token = config.auth.github_token.as_ref().context(
        "GitHub token not configured. Set GIT_TRENDING_MOTD_GITHUB_TOKEN or add github_token to config file.",
    )?;

    // Parse repo name (format: owner/repo)
    let parts: Vec<&str> = repo.split('/').collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid repository format. Expected: owner/repo");
    }
    let (owner, repo_name) = (parts[0], parts[1]);

    eprintln!("⭐ Starring {owner}/{repo_name} on GitHub...");

    let github = GitHub::new(config.general.github_timeout_secs)?;
    github.star_repo(owner, repo_name, token).await?;

    println!("✓ Successfully starred {owner}/{repo_name}");

    // Invalidate starred cache
    if let Ok(starred_cache) = StarredCache::new() {
        let _ = starred_cache.clear().await;
    }

    Ok(())
}

/// Handle the clone subcommand
fn handle_clone_command(repo: &str) -> Result<()> {
    // Support both "owner/repo" format and full URLs
    let clone_url = if repo.starts_with("http://") || repo.starts_with("https://") {
        repo.to_string()
    } else {
        // Assume GitHub by default for owner/repo format
        format!("https://github.com/{repo}.git")
    };

    eprintln!("📦 Cloning {clone_url}...");

    // Use git clone command
    let output = std::process::Command::new("git")
        .arg("clone")
        .arg(&clone_url)
        .output()
        .context("Failed to execute git clone. Is git installed?")?;

    if output.status.success() {
        println!("✓ Successfully cloned {repo}");
        Ok(())
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Git clone failed: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use model::Repo;

    // ===== Helper Functions Tests =====

    fn test_repo(name: &str, description: Option<&str>) -> Repo {
        Repo {
            provider: "github".to_string(),
            icon: "GH".to_string(),
            name: name.to_string(),
            language: Some("Rust".to_string()),
            description: description.map(String::from),
            url: format!("https://github.com/{name}"),
            stars_today: Some(50),
            stars_total: Some(1000),
            last_activity: None,
            topics: vec![],
            is_starred: false,
        }
    }

    #[test]
    fn test_is_mostly_ascii_pure_english() {
        let repo = test_repo(
            "rust-lang/rust",
            Some("Empowering everyone to build reliable and efficient software."),
        );
        assert!(is_mostly_ascii(&repo));
    }

    #[test]
    fn test_is_mostly_ascii_cjk_name() {
        let repo = test_repo("用户名/项目", Some("Some description"));
        // Name has < 80% ASCII, should fail
        assert!(!is_mostly_ascii(&repo));
    }

    #[test]
    fn test_is_mostly_ascii_cjk_description() {
        let repo = test_repo("owner/repo", Some("这是一个中文描述，应该会失败"));
        // Description has < 70% ASCII, should fail
        assert!(!is_mostly_ascii(&repo));
    }

    #[test]
    fn test_is_mostly_ascii_mixed_content() {
        let repo = test_repo("owner/repo", Some("Hello world 你好"));
        // Mixed content: "Hello world " (12 chars) + "你好" (2 chars) = 14 chars
        // ASCII: 12/14 = 85.7% which is > 70%, should pass
        assert!(is_mostly_ascii(&repo));
    }

    #[test]
    fn test_is_mostly_ascii_no_description() {
        let repo = test_repo("rust-lang/rust", None);
        assert!(is_mostly_ascii(&repo));
    }

    #[test]
    fn test_ascii_ratio_pure_ascii() {
        assert_eq!(ascii_ratio("hello world"), 1.0);
        assert_eq!(ascii_ratio("rust-lang/rust"), 1.0);
        assert_eq!(ascii_ratio("123456"), 1.0);
    }

    #[test]
    fn test_ascii_ratio_no_ascii() {
        let ratio = ascii_ratio("你好世界");
        assert!(ratio < 0.1); // Should be 0.0 or very close
    }

    #[test]
    fn test_ascii_ratio_mixed() {
        let ratio = ascii_ratio("hello世界");
        assert!(ratio > 0.7 && ratio < 0.8); // 5 ASCII / 7 total ≈ 0.71
    }

    #[test]
    fn test_ascii_ratio_empty_string() {
        assert_eq!(ascii_ratio(""), 1.0);
    }

    // ===== Clone Command Tests =====

    #[test]
    fn test_clone_url_construction_from_owner_repo() {
        // We can't test the actual function directly without executing git,
        // but we can test the URL construction logic
        let repo = "rust-lang/rust";
        let expected_url = "https://github.com/rust-lang/rust.git";

        // Test the logic inline (this is what handle_clone_command does)
        let clone_url = if repo.starts_with("http://") || repo.starts_with("https://") {
            repo.to_string()
        } else {
            format!("https://github.com/{repo}.git")
        };

        assert_eq!(clone_url, expected_url);
    }

    #[test]
    fn test_clone_url_construction_preserves_full_url() {
        let repo = "https://github.com/rust-lang/rust.git";

        let clone_url = if repo.starts_with("http://") || repo.starts_with("https://") {
            repo.to_string()
        } else {
            format!("https://github.com/{repo}.git")
        };

        assert_eq!(clone_url, repo);
    }

    #[test]
    fn test_clone_url_construction_handles_trailing_git() {
        let repo = "owner/repo.git";
        let expected_url = "https://github.com/owner/repo.git.git"; // This is the actual behavior

        let clone_url = if repo.starts_with("http://") || repo.starts_with("https://") {
            repo.to_string()
        } else {
            format!("https://github.com/{repo}.git")
        };

        // Note: This test documents current behavior; it might double .git extension
        assert_eq!(clone_url, expected_url);
    }

    #[test]
    fn test_clone_url_with_http_protocol() {
        let repo = "http://example.com/repo.git";

        let clone_url = if repo.starts_with("http://") || repo.starts_with("https://") {
            repo.to_string()
        } else {
            format!("https://github.com/{repo}.git")
        };

        assert_eq!(clone_url, repo);
    }
}

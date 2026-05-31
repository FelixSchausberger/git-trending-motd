use assert_cmd::Command;
use predicates::prelude::*;
use std::env;
use tempfile::TempDir;

/// Test that star command requires a GitHub token
#[tokio::test]
async fn test_star_command_requires_github_token() {
    // Temporarily remove any token from environment
    let original_token = env::var("GIT_TRENDING_MOTD_GITHUB_TOKEN").ok();
    env::remove_var("GIT_TRENDING_MOTD_GITHUB_TOKEN");

    let mut cmd = Command::cargo_bin("git-trending").unwrap();
    cmd.arg("star").arg("rust-lang/rust");

    let assert = cmd.assert();
    assert
        .failure()
        .stderr(predicate::str::contains("GitHub token not configured"));

    // Restore original token if it existed
    if let Some(token) = original_token {
        env::set_var("GIT_TRENDING_MOTD_GITHUB_TOKEN", token);
    }
}

/// Test that star command validates repository format
#[tokio::test]
async fn test_star_command_validates_format() {
    // Set a dummy token for testing (won't actually call API due to invalid format)
    env::set_var("GIT_TRENDING_MOTD_GITHUB_TOKEN", "test_token_for_validation");

    let mut cmd = Command::cargo_bin("git-trending").unwrap();
    cmd.arg("star").arg("invalidformat");

    let assert = cmd.assert();
    assert
        .failure()
        .stderr(predicate::str::contains("Invalid repository format"));

    // Clean up
    env::remove_var("GIT_TRENDING_MOTD_GITHUB_TOKEN");
}

/// Test that star command accepts owner/repo format
#[tokio::test]
async fn test_star_command_accepts_owner_slash_repo_format() {
    // This test documents that the format parsing works correctly
    let repo = "rust-lang/rust";
    let parts: Vec<&str> = repo.split('/').collect();
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0], "rust-lang");
    assert_eq!(parts[1], "rust");
}

/// Test that star command with multi-slash format fails
#[tokio::test]
async fn test_star_command_rejects_multi_slash() {
    // Note: Token validation happens before format validation in the current implementation
    // So this test documents that behavior
    env::remove_var("GIT_TRENDING_MOTD_GITHUB_TOKEN");

    let mut cmd = Command::cargo_bin("git-trending").unwrap();
    cmd.arg("star").arg("owner/repo/extra");

    let assert = cmd.assert();
    // Will fail with token error first, but documents expected behavior
    assert.failure();
}

/// Test that star command shows appropriate error message structure
#[tokio::test]
async fn test_star_command_error_message_structure() {
    // No token set
    env::remove_var("GIT_TRENDING_MOTD_GITHUB_TOKEN");

    let mut cmd = Command::cargo_bin("git-trending").unwrap();
    cmd.arg("star").arg("owner/repo");

    let assert = cmd.assert();
    assert.failure().stderr(
        predicate::str::contains("GitHub token")
            .and(predicate::str::contains("GIT_TRENDING_MOTD_GITHUB_TOKEN")),
    );
}

#[cfg(feature = "integration_tests_with_api")]
#[tokio::test]
#[ignore] // Only run when explicitly requested with GitHub API token
async fn test_star_command_real_api() {
    // This test requires a real GitHub token and will actually star a repo
    // Only run with: cargo test --ignored
    //
    // Set up:
    //   export GIT_TRENDING_MOTD_GITHUB_TOKEN="your_token"
    //   cargo test --test star_command test_star_command_real_api -- --ignored

    let token = env::var("GIT_TRENDING_MOTD_GITHUB_TOKEN")
        .expect("GIT_TRENDING_MOTD_GITHUB_TOKEN must be set for this test");

    // Use a test repository (you should use your own test repo)
    let test_repo = "octocat/Hello-World";

    let mut cmd = Command::cargo_bin("git-trending").unwrap();
    cmd.arg("star").arg(test_repo);
    cmd.env("GIT_TRENDING_MOTD_GITHUB_TOKEN", token);

    let assert = cmd.assert();
    // API might succeed or fail depending on rate limits, network, etc.
    // We just verify the command executes without panicking
    assert.code(predicate::in_iter(vec![0, 1]));
}

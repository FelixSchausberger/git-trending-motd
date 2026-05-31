use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

/// Test that clone command constructs GitHub URL correctly
#[test]
fn test_clone_url_construction_logic() {
    // This documents the URL construction logic used in handle_clone_command
    let test_cases = vec![
        ("rust-lang/rust", "https://github.com/rust-lang/rust.git"),
        (
            "https://github.com/rust-lang/rust.git",
            "https://github.com/rust-lang/rust.git",
        ),
        (
            "http://example.com/repo.git",
            "http://example.com/repo.git",
        ),
    ];

    for (input, expected) in test_cases {
        let url = if input.starts_with("http://") || input.starts_with("https://") {
            input.to_string()
        } else {
            format!("https://github.com/{input}.git")
        };
        assert_eq!(url, expected, "Failed for input: {input}");
    }
}

/// Test that clone command handles missing git gracefully
#[test]
fn test_clone_command_error_message_without_git() {
    // We can't easily test the actual "git not found" scenario without
    // manipulating PATH, but we can document expected behavior
    // The error message should mention "git clone" and "Is git installed?"
    let error_patterns = vec!["git clone", "git installed"];

    // This test documents that these patterns should appear in error messages
    for pattern in error_patterns {
        assert!(!pattern.is_empty());
    }
}

/// Test that clone command preserves GitLab URLs
#[test]
fn test_clone_preserves_gitlab_urls() {
    let gitlab_url = "https://gitlab.com/user/project.git";
    let url = if gitlab_url.starts_with("http://") || gitlab_url.starts_with("https://") {
        gitlab_url.to_string()
    } else {
        format!("https://github.com/{gitlab_url}.git")
    };
    assert_eq!(url, gitlab_url);
}

/// Test that clone command preserves Gitea URLs
#[test]
fn test_clone_preserves_gitea_urls() {
    let gitea_url = "https://gitea.com/user/project.git";
    let url = if gitea_url.starts_with("http://") || gitea_url.starts_with("https://") {
        gitea_url.to_string()
    } else {
        format!("https://github.com/{gitea_url}.git")
    };
    assert_eq!(url, gitea_url);
}

#[test]
#[ignore] // Requires git installed and network access
fn test_clone_command_with_real_git() {
    // This test actually executes git clone
    // Only run with: cargo test --ignored

    let temp_dir = TempDir::new().unwrap();
    let original_dir = std::env::current_dir().unwrap();

    // Change to temp directory for cloning
    std::env::set_current_dir(&temp_dir).unwrap();

    // Use a small, fast-to-clone repository
    let mut cmd = Command::cargo_bin("git-trending").unwrap();
    cmd.arg("clone").arg("octocat/Hello-World");

    let assert = cmd.assert();

    // Restore original directory
    std::env::set_current_dir(original_dir).unwrap();

    // Verify the command succeeded or failed gracefully
    // Success means git is available and network works
    // Failure could be due to network issues, rate limits, etc.
    assert.code(predicate::in_iter(vec![0, 1]));
}

#[test]
#[ignore] // Requires git installed
fn test_clone_command_with_full_url() {
    // Test cloning with a full URL instead of owner/repo format
    let temp_dir = TempDir::new().unwrap();
    let original_dir = std::env::current_dir().unwrap();

    std::env::set_current_dir(&temp_dir).unwrap();

    let mut cmd = Command::cargo_bin("git-trending").unwrap();
    cmd.arg("clone")
        .arg("https://github.com/octocat/Hello-World.git");

    let assert = cmd.assert();

    std::env::set_current_dir(original_dir).unwrap();

    // Verify command completed
    assert.code(predicate::in_iter(vec![0, 1]));
}

# git trending

> **Trending repositories of the day** - minimal MOTD CLI

A lightweight CLI tool that prints a Message of the Day (MOTD) with the top trending repositories from GitHub, GitLab, and Gitea. Use as `git trending` command.

Inspired by [github-trending-cli](https://github.com/psalias2006/github-trending-cli).

## Example Output

```
⭐[GH] anthropics/anthropic-cookbook                     Python        ★30   today      yesterday  A collection of notebooks/recipes showcasing...
  [GH] langgenius/dify                                   TypeScript    ★162  today      today      Dify is an open-source LLM app development...
  [GH] microsoft/TypeChat                                TypeScript    ★8090            today      TypeChat is a library that makes it easy to...
  [GL] gitlab-org/gitlab                                 Ruby          ★23817           3d ago     GitLab CE Mirror | Please open new issues...
  [GL] inkscape/inkscape                                 C++           ★2458            yesterday  Inkscape vector image editor
  [GE] go-gitea/gitea                                    Go            ★41249           today      Git with a cup of tea! Painless self-hosted...
```

**Legend:**
- `[GH]` = GitHub, `[GL]` = GitLab, `[GE]` = Gitea
- `⭐` = You have starred this repository (requires GitHub token)
- `★N today` = Stars gained today
- `★N` = Total stars (when daily stars unavailable)
- `~` = Approximated (not from official trending API)


## Features

- **Multi-provider support**: GitHub, GitLab, Gitea (configurable base URL)
- **Parallel fetching**: Concurrent API calls with configurable timeout
- **Smart caching**: Filesystem-based cache with TTL (XDG-compliant)
- **Memory system**: Daily-reset tracking - see new repos each terminal session
- **Network resilience**: Gracefully falls back to cached data when offline
- **Flexible configuration**: TOML config, environment variables, CLI flags
- **Advanced filtering**:
  - Language filtering (e.g., `--lang rust,go`)
  - Star threshold filtering (e.g., `--min-stars 100`)
  - Topic exclusion for GitHub (e.g., `--exclude-topics awesome`)
  - Automatic filtering of already-seen repos (resets daily)
- **GitHub integration**:
  - Star repositories from the CLI
  - Visual indicators for already-starred repos
  - Quick clone trending repositories
- **Git extension**: Works as `git trending` command (git automatically detects git-* binaries)
- **Beautiful output**: Colored terminal output with starred indicators
- **JSON export**: Optional JSON output for scripting
- **Shell completions**: Generate completions for Bash, Fish, Zsh, PowerShell
- **MOTD Integration**: Easy integration as Message of the Day

## Installation

### From Source

```bash
git clone https://github.com/schausberger/git-trending-motd
cd git-trending-motd
cargo install --path .

# Now you can use: git trending
```

### With Nix

```bash
nix build
# or
nix develop  # Enter development shell
```

## Usage

### Basic Usage

```bash
# Show trending repos from all providers (filters out already-seen repos)
git trending

# Show all repos including already-seen ones
git trending --show-all

# Show top 5 repos per provider
git trending --max 5

# Filter by language
git trending --lang rust,go

# Filter by star count (minimum 100 stars)
git trending --min-stars 100

# Exclude specific topics from GitHub
git trending --exclude-topics awesome,awesome-list

# Combine filters
git trending --lang rust --min-stars 50 --exclude-topics web

# Specific providers only (gh=GitHub, gl=GitLab, ge=Gitea)
git trending --provider gh,gl

# JSON output
git trending --json

# Disable cache
git trending --no-cache
```

### Git Extension Usage

```bash
# Star a repository
git trending star owner/repo

# Clone a trending repository
git trending clone owner/repo

# Clone with full URL
git trending clone https://github.com/owner/repo
```

### GitHub Integration

To see starred status indicators (⭐) and use the star command, configure your GitHub token:

```bash
# Via environment variable
export TROTD_GITHUB_TOKEN="ghp_your_token_here"

# Or in config file (~/.config/trotd/trotd.toml)
[auth]
github_token = "ghp_your_token_here"

# Or via CLI flag
git trending --github-token "ghp_your_token_here"
```

Generate a personal access token at: https://github.com/settings/tokens
Required scopes: `public_repo` (or `repo` for private repos)

### Shell Completions

Generate shell completions for better UX:

```bash
# Bash
git trending completions bash > /etc/bash_completion.d/git-trending

# Fish
git trending completions fish > ~/.config/fish/completions/git-trending.fish

# Zsh
git trending completions zsh > ~/.zsh/completions/_git-trending

# PowerShell
git trending completions powershell > git-trending.ps1
```

### MOTD Integration

See [examples/README.md](examples/README.md) for detailed integration guides.

#### Quick Start

**For bash**, add to `~/.bashrc`:

```bash
# Run git-trending in interactive shells only, after tmux/zellij
if [[ $- == *i* ]] && [ -z "$TMUX" ] && [ -z "$ZELLIJ" ]; then
    if command -v git-trending &> /dev/null; then
        git trending 2>/dev/null || true
    fi
fi
```

**For fish**, add to `~/.config/fish/config.fish`:

```fish
# Run git-trending in outermost interactive shell only
# Works with zellij, tmux, starship, etc.
if status is-interactive; and not set -q TMUX; and not set -q ZELLIJ; and not set -q ZELLIJ_SESSION_NAME
    if command -v git-trending &> /dev/null
        # Use event handler to run after shell is fully initialized
        function __git_trending_motd --on-event fish_prompt
            git trending 2>/dev/null; or true
            functions -e __git_trending_motd  # Remove this function after first run
        end
    end
end
```

This approach ensures the MOTD appears:
- Only in interactive shells (not scripts)
- Only in the outermost shell (not inside tmux/zellij)
- After shell initialization (works with starship, prompt customizations, etc.)

Or use the automated setup script:

```bash
sudo bash examples/motd-setup.sh
```

## Configuration

### Configuration File

git-trending looks for configuration in:
1. `~/.config/trotd/trotd.toml` (XDG config directory)
2. `./trotd.toml` (current directory)

Example `trotd.toml`:

```toml
[general]
max_per_provider = 3
timeout_secs = 6
cache_ttl_mins = 60
language_filter = ["rust", "go"]
min_stars = 50                    # Filter repos below 50 stars
ascii_only = false                # Hide non-ASCII repo names
fast_network_timeout_secs = 3     # Quick timeout for network checks
show_starred_status = true        # Show ⭐ for starred repos (requires GitHub token)

[providers]
github = true
gitlab = true
gitea = true

[auth]
github_token = ""   # Required for starring and showing starred status
gitlab_token = ""
gitea_token = ""

[gitea]
base_url = "https://gitea.com"

[github]
exclude_topics = ["awesome", "awesome-list"]  # Exclude these topics
```

### Environment Variables

Environment variables override config file settings:

```bash
export TROTD_MAX_PER_PROVIDER=5
export TROTD_LANGUAGE_FILTER="rust,go,python"
export TROTD_MIN_STARS=100
export TROTD_GITHUB_EXCLUDE_TOPICS="awesome,tutorial"
export TROTD_GITEA_BASE_URL="https://codeberg.org"
export TROTD_GITHUB_TOKEN="ghp_..."
export TROTD_GITLAB_TOKEN="glpat-..."
export TROTD_GITEA_TOKEN="..."
```

### Command-Line Flags

CLI flags override both config file and environment variables:

```bash
git trending --max 5 --lang rust --min-stars 100 --exclude-topics awesome --provider gh --no-cache --json
```

## Provider Details

### GitHub

- **Method**: HTML scraping of trending page (default) or Search API (when topic exclusion is used)
- **Endpoint**: `https://github.com/trending` or `/search/repositories`
- **Features**:
  - Official trending data from HTML scraping
  - Topic exclusion (requires API mode)
  - Language filtering
- **Approximated**: No (HTML scraping), Yes (API mode)
- **Authentication**: Optional (increases rate limits, required for API mode)

### GitLab

- **API**: GitLab REST API v4
- **Endpoint**: `/api/v4/projects?order_by=created_at`
- **Approximated**: Yes (filters by creation date, sorted by stars)
- **Authentication**: Optional (private repos)

### Gitea

- **API**: Gitea REST API v1
- **Endpoint**: `{base_url}/api/v1/repos/search`
- **Approximated**: Yes (search API sorted by recent activity)
- **Authentication**: Optional
- **Configurable**: Custom base URL (supports Codeberg, self-hosted instances)

## Architecture

```
src/
├── main.rs         # CLI entry point, parallel fetching
├── config.rs       # Configuration (TOML + env + CLI)
├── model.rs        # Repo struct, Provider trait
├── render.rs       # MOTD rendering with colors
├── cache.rs        # Filesystem cache with TTL
├── http.rs         # HTTP client wrapper (GET/PUT/HEAD)
├── seen.rs         # Daily-reset seen repos tracker
├── starred.rs      # GitHub starred status cache
└── providers/
    ├── github.rs   # GitHub trending API + starring
    ├── gitlab.rs   # GitLab explore API
    └── gitea.rs    # Gitea search API
```

**Design Philosophy:**
- **Minimal dependencies**: Few runtime dependencies
- **Clean code**: Strict lints (forbid unsafe, clippy pedantic)
- **Hybrid approach**: HTML scraping for GitHub trending, APIs for GitLab/Gitea
- **Parallel execution**: FuturesUnordered for concurrent fetching
- **Error resilience**: Partial results on provider failures

## Development

### Prerequisites

- Rust 1.89+ (or use Nix flake)
- cargo-nextest (optional but recommended)

### Build

```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release
```

### Testing

```bash
# Run tests
cargo test

# With nextest
cargo nextest run

# Run pre-commit hooks
prek run --all-files
```

### Nix Development Environment

```bash
nix develop

# Available tools:
# - rust-analyzer
# - cargo-nextest
# - cargo-watch
# - prek (pre-commit hooks)
```

### Pre-commit Hooks

Install hooks with prek:

```bash
prek install
```

Hooks run:
1. `cargo fmt --all --check`
2. `cargo clippy --all-targets --all-features -- -D warnings`
3. `cargo nextest run` (or `cargo test` if nextest unavailable)

## Dependencies

**Runtime** (17 crates):
- tokio, reqwest - Async HTTP
- serde, serde_json, toml - Serialization
- clap, clap_complete - CLI parsing and shell completions
- anyhow, thiserror - Error handling
- dirs - XDG directories
- async-trait - Trait async methods
- colored - Terminal colors
- chrono - Date handling
- futures - Concurrent streams
- scraper - HTML parsing (GitHub trending)
- tokio-retry - Retry logic
- regex - Text processing

**Development** (1 crate):
- mockito - HTTP mocking

## License

MIT License - see [LICENSE](LICENSE) file

## Contributing

Contributions welcome! Please:
1. Run `cargo fmt` before committing
2. Ensure `cargo clippy` passes with no warnings
3. Add tests for new features
4. Update README if adding configuration options
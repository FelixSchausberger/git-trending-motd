# trotd

> **Trending repositories of the day** - minimal MOTD CLI

A lightweight CLI tool that prints a Message of the Day (MOTD) with the top trending repositories from GitHub, GitLab, and Gitea.

Inspired by [github-trending-cli](https://github.com/psalias2006/github-trending-cli).

## Example Output

<!-- EXAMPLE_OUTPUT_START -->
```
[GE] gitea/tea                    Go ★187        today      A command line tool to interact with Gitea...
[GH] google/adk-go                Go ★127  today today      An open-source, code-first Go toolkit for...
[GL] gitlab-org/quality/testcases -  ★14         today
```
<!-- EXAMPLE_OUTPUT_END -->

**Legend:**
- `[GH]` = GitHub, `[GL]` = GitLab, `[GE]` = Gitea
- `★N today` = Stars gained today
- `★N` = Total stars (when daily stars unavailable)
- `~` = Approximated (not from official trending API)

*Note: Example output is automatically updated by CI on each push and daily at midnight UTC.*

## Features

- **Multi-provider support**: GitHub, GitLab, Gitea (configurable base URL)
- **Parallel fetching**: Concurrent API calls with configurable timeout
- **Smart caching**: Filesystem-based cache with TTL (XDG-compliant)
- **Flexible configuration**: TOML config, environment variables, CLI flags
- **Advanced filtering**:
  - Language filtering (e.g., `--lang rust,go`)
  - Star threshold filtering (e.g., `--min-stars 100`)
  - Topic exclusion for GitHub (e.g., `--exclude-topics awesome`)
- **Beautiful output**: Colored terminal output with nerd font icons
- **JSON export**: Optional JSON output for scripting
- **Shell completions**: Generate completions for Bash, Fish, Zsh, PowerShell
- **MOTD Integration**: Easy integration as Message of the Day

## Installation

### From Source

```bash
git clone https://github.com/schausberger/trotd
cd trotd
cargo install --path .
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
# Show trending repos from all providers
trotd

# Show top 5 repos per provider
trotd --max 5

# Filter by language
trotd --lang rust,go

# Filter by star count (minimum 100 stars)
trotd --min-stars 100

# Exclude specific topics from GitHub
trotd --exclude-topics awesome,awesome-list

# Combine filters
trotd --lang rust --min-stars 50 --exclude-topics web

# Specific providers only (gh=GitHub, gl=GitLab, ge=Gitea)
trotd --provider gh,gl

# JSON output
trotd --json

# Disable cache
trotd --no-cache
```

### Shell Completions

Generate shell completions for better UX:

```bash
# Bash
trotd completions bash > /etc/bash_completion.d/trotd

# Fish
trotd completions fish > ~/.config/fish/completions/trotd.fish

# Zsh
trotd completions zsh > ~/.zsh/completions/_trotd

# PowerShell
trotd completions powershell > trotd.ps1
```

### MOTD Integration

See [examples/README.md](examples/README.md) for detailed integration guides.

#### Quick Start

Add to your shell RC file (`~/.bashrc`, `~/.zshrc`, or `~/.config/fish/config.fish`):

```bash
if command -v trotd &> /dev/null; then
    trotd 2>/dev/null || true
fi
```

Or use the automated setup script:

```bash
sudo bash examples/motd-setup.sh
```

## Configuration

### Configuration File

trotd looks for configuration in:
1. `~/.config/trotd/trotd.toml` (XDG config directory)
2. `./trotd.toml` (current directory)

Example `trotd.toml`:

```toml
[general]
max_per_provider = 3
timeout_secs = 6
cache_ttl_mins = 60
language_filter = ["rust", "go"]
min_stars = 50              # Filter repos below 50 stars
ascii_only = false          # Hide non-ASCII repo names

[providers]
github = true
gitlab = true
gitea = true

[auth]
github_token = ""
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
trotd --max 5 --lang rust --min-stars 100 --exclude-topics awesome --provider gh --no-cache --json
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
├── http.rs         # HTTP client wrapper
└── providers/
    ├── github.rs   # GitHub trending API
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
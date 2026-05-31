# git-trending Integration Examples

This directory contains examples of how to integrate `git trending` into various workflows.

## MOTD (Message of the Day)

### Automatic Setup Script

Run the automated setup script:

```bash
sudo bash examples/motd-setup.sh
```

This will configure your system to show trending repos on login.

### Manual Shell Integration

**For bash**, add to `~/.bashrc`:

```bash
# Run git-trending in interactive shells only, after tmux/zellij
if [[ $- == *i* ]] && [ -z "$TMUX" ] && [ -z "$ZELLIJ" ]; then
    if command -v git-trending &> /dev/null; then
        git trending 2>/dev/null || true
    fi
fi
```

**For zsh**, add to `~/.zshrc`:

```zsh
# Run git-trending in interactive shells only, after tmux/zellij
if [[ -o interactive ]] && [ -z "$TMUX" ] && [ -z "$ZELLIJ" ]; then
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

**Why this approach?**
- **Interactive check**: Only runs in interactive shells (not scripts)
- **Multiplexer detection**: Skips inside tmux/zellij sessions (shows only on outer shell)
- **Fish event handler**: Uses `fish_prompt` event to run after shell initialization (starship, etc.)
- **Self-cleanup**: Fish function removes itself after first run

### Advanced: Show Once Per Day

**For bash/zsh**:

```bash
MOTD_CACHE="$HOME/.cache/git-trending-motd/.shown_today"
TODAY=$(date +%Y-%m-%d)

if [ ! -f "$MOTD_CACHE" ] || [ "$(cat $MOTD_CACHE)" != "$TODAY" ]; then
    if command -v git-trending &> /dev/null; then
        git trending && echo "$TODAY" > "$MOTD_CACHE"
    fi
fi
```

**For fish**:

```fish
if status is-interactive; and not set -q TMUX; and not set -q ZELLIJ
    set -l cache_file "$HOME/.cache/git-trending-motd/.shown_today"
    set -l today (date +%Y-%m-%d)

    if not test -f "$cache_file"; or test (cat "$cache_file" 2>/dev/null) != "$today"
        if command -v git-trending &> /dev/null
            function __git_trending_motd --on-event fish_prompt
                git trending 2>/dev/null && echo $today > $cache_file
                functions -e __git_trending_motd
            end
        end
    end
end
```

## Systemd Timer (Auto-Refresh)

Keep your cache always fresh with a systemd user timer:

```bash
bash examples/systemd-timer-refresh.sh
```

This creates a timer that refreshes the cache every hour in the background.

## Shell Completions

Generate shell completions for better UX:

### Bash

```bash
git trending completions bash > /etc/bash_completion.d/git-trending
```

Or for user-only:

```bash
mkdir -p ~/.local/share/bash-completion/completions
git trending completions bash > ~/.local/share/bash-completion/completions/git-trending
```

### Fish

```bash
git trending completions fish > ~/.config/fish/completions/git-trending.fish
```

### Zsh

```bash
mkdir -p ~/.zsh/completions
git trending completions zsh > ~/.zsh/completions/_git-trending
```

Then add to `~/.zshrc`:

```zsh
fpath=(~/.zsh/completions $fpath)
autoload -Uz compinit && compinit
```

## Tips & Tricks

### Filter by Language

Show only Rust and Go repositories:

```bash
git trending --lang rust,go
```

### Filter by Star Count

Show only popular repos (100+ stars):

```bash
git trending --min-stars 100
```

### Exclude Topics (GitHub Only)

Exclude certain topics from GitHub results:

```bash
git trending --exclude-topics awesome,awesome-list
```

### Combine Filters

```bash
git trending --lang rust --min-stars 50 --exclude-topics web
```

### Use in Scripts

```bash
#!/bin/bash
# Daily digest email

{
    echo "Subject: Daily Trending Repos"
    echo ""
    git trending
} | sendmail user@example.com
```

### JSON Output for Custom Processing

```bash
# Get just the repo names
git trending --json | jq -r '.[].name'

# Get repos with their star counts
git trending --json | jq -r '.[] | "\(.name): \(.stars_total) stars"'

# Filter JSON output
git trending --json | jq '.[] | select(.stars_total > 100)'
```

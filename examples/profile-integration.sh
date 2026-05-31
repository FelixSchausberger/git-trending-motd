#!/bin/bash
# Example script showing how to integrate git-trending into your shell profile
# This will show trending repos every time you open a new terminal

# For bash, add to ~/.bashrc:
# ----------------------------------------
# # Run git-trending in interactive shells only, after tmux/zellij
# if [[ $- == *i* ]] && [ -z "$TMUX" ] && [ -z "$ZELLIJ" ]; then
#     if command -v git-trending &> /dev/null; then
#         git trending 2>/dev/null || true
#     fi
# fi

# For zsh, add to ~/.zshrc:
# ----------------------------------------
# # Run git-trending in interactive shells only, after tmux/zellij
# if [[ -o interactive ]] && [ -z "$TMUX" ] && [ -z "$ZELLIJ" ]; then
#     if command -v git-trending &> /dev/null; then
#         git trending 2>/dev/null || true
#     fi
# fi

# For fish, add to ~/.config/fish/config.fish:
# ----------------------------------------
# # Run git-trending in outermost interactive shell only
# # Works with zellij, tmux, starship, etc.
# if status is-interactive; and not set -q TMUX; and not set -q ZELLIJ; and not set -q ZELLIJ_SESSION_NAME
#     if command -v git-trending &> /dev/null
#         # Use function to run after shell is fully initialized
#         function __git_trending_motd --on-event fish_prompt
#             git trending 2>/dev/null; or true
#             functions -e __git_trending_motd  # Remove this function after first run
#         end
#     end
# end

# Advanced: Only show once per day
# ----------------------------------------
# MOTD_CACHE="$HOME/.cache/git-trending-motd/.shown_today"
# TODAY=$(date +%Y-%m-%d)
#
# if [ ! -f "$MOTD_CACHE" ] || [ "$(cat $MOTD_CACHE)" != "$TODAY" ]; then
#     if command -v git-trending &> /dev/null; then
#         git trending && echo "$TODAY" > "$MOTD_CACHE"
#     fi
# fi

# Alternative for fish (once per day):
# ----------------------------------------
# if status is-interactive; and not set -q TMUX; and not set -q ZELLIJ
#     set -l cache_file "$HOME/.cache/git-trending-motd/.shown_today"
#     set -l today (date +%Y-%m-%d)
#
#     if not test -f "$cache_file"; or test (cat "$cache_file" 2>/dev/null) != "$today"
#         if command -v git-trending &> /dev/null
#             function __git_trending_motd --on-event fish_prompt
#                 git trending 2>/dev/null && echo $today > $cache_file
#                 functions -e __git_trending_motd
#             end
#         end
#     end
# end

echo "Copy the appropriate snippet above to your shell configuration file!"

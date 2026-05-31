#!/bin/bash
# Example systemd user timer to refresh git-trending-motd cache periodically
# This ensures the cache is always fresh when you open a terminal

# Create the service file at: ~/.config/systemd/user/git-trending-motd-refresh.service
cat > ~/.config/systemd/user/git-trending-motd-refresh.service <<'EOF'
[Unit]
Description=Refresh git-trending-motd cache
After=network.target

[Service]
Type=oneshot
ExecStart=/usr/bin/env bash -c 'git-trending --no-cache > /dev/null 2>&1'

[Install]
WantedBy=default.target
EOF

# Create the timer file at: ~/.config/systemd/user/git-trending-motd-refresh.timer
cat > ~/.config/systemd/user/git-trending-motd-refresh.timer <<'EOF'
[Unit]
Description=Refresh git-trending-motd cache every hour

[Timer]
OnBootSec=5min
OnUnitActiveSec=1h
Persistent=true

[Install]
WantedBy=timers.target
EOF

# Reload systemd and enable the timer
systemctl --user daemon-reload
systemctl --user enable git-trending-motd-refresh.timer
systemctl --user start git-trending-motd-refresh.timer

echo "✅ Systemd timer created and enabled!"
echo "The cache will be refreshed every hour."
echo ""
echo "Useful commands:"
echo "  systemctl --user status git-trending-motd-refresh.timer  - Check timer status"
echo "  systemctl --user stop git-trending-motd-refresh.timer    - Stop the timer"
echo "  systemctl --user disable git-trending-motd-refresh.timer - Disable the timer"
echo "  journalctl --user -u git-trending-motd-refresh.service   - View logs"

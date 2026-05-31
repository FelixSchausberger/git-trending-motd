#!/bin/bash
# Setup script to configure git-trending-motd as Message of the Day (MOTD)
# This script will configure your system to show git-trending-motd output when you log in

set -e

echo "🚀 Setting up git-trending-motd as MOTD..."

# Detect the Linux distribution
if [ -f /etc/debian_version ]; then
    DISTRO="debian"
elif [ -f /etc/redhat-release ]; then
    DISTRO="redhat"
elif [ -f /etc/arch-release ]; then
    DISTRO="arch"
else
    DISTRO="other"
fi

# Check if git-trending is installed
if ! command -v git-trending &> /dev/null; then
    echo "❌ Error: git-trending is not installed or not in PATH"
    echo "Please install git-trending-motd first: cargo install --path ."
    exit 1
fi

# Create MOTD script
MOTD_SCRIPT="/etc/update-motd.d/99-git-trending-motd"

echo "📝 Creating MOTD script at $MOTD_SCRIPT"

if [ "$DISTRO" = "debian" ]; then
    # Debian/Ubuntu uses update-motd.d
    echo "Detected Debian/Ubuntu system"

    if [ ! -d "/etc/update-motd.d" ]; then
        echo "Creating /etc/update-motd.d directory..."
        sudo mkdir -p /etc/update-motd.d
    fi

    sudo tee "$MOTD_SCRIPT" > /dev/null <<'EOF'
#!/bin/bash
# Display trending repositories using git-trending-motd

/usr/local/bin/git-trending || true
EOF

    sudo chmod +x "$MOTD_SCRIPT"

    # Disable other MOTD scripts if desired
    read -p "Disable other MOTD scripts? (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        for script in /etc/update-motd.d/*; do
            if [ "$script" != "$MOTD_SCRIPT" ]; then
                sudo chmod -x "$script"
            fi
        done
        echo "✅ Disabled other MOTD scripts"
    fi

else
    # For other systems, add to /etc/profile.d/
    PROFILE_SCRIPT="/etc/profile.d/99-git-trending-motd.sh"

    echo "Creating profile script at $PROFILE_SCRIPT"

    sudo tee "$PROFILE_SCRIPT" > /dev/null <<'EOF'
#!/bin/bash
# Display trending repositories using git-trending-motd (only for interactive shells)

if [ -n "$PS1" ]; then
    /usr/local/bin/git-trending 2>/dev/null || true
fi
EOF

    sudo chmod +x "$PROFILE_SCRIPT"
fi

echo ""
echo "✅ git-trending-motd MOTD setup complete!"
echo ""
echo "The trending repositories will now be displayed when you log in."
echo "To test it now, run: git trending"
echo ""
echo "Configuration tips:"
echo "  - Edit ~/.config/git-trending-motd/config.toml to customize settings"
echo "  - Use --no-cache flag for fresh data"
echo "  - Use --lang rust,go to filter by language"
echo "  - Use --min-stars 100 to filter by star count"
echo ""

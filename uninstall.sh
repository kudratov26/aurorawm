#!/bin/bash
set -e

echo "Uninstalling AuroraWM..."

# Check if running as root
if [ "$EUID" -ne 0 ]; then 
    echo "Please run as root or use sudo"
    exit 1
fi

# Remove binary
echo "Removing binary..."
rm -f /usr/local/bin/aurorawm

# Remove desktop entry
echo "Removing desktop entry..."
rm -f /usr/share/wayland-sessions/aurorawm.desktop

# Remove systemd service
echo "Removing systemd service..."
systemctl disable aurorawm 2>/dev/null || true
rm -f /usr/lib/systemd/system/aurorawm.service
systemctl daemon-reload

# Remove config directory (optional)
read -p "Remove configuration directory? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo "Removing configuration directory..."
    rm -rf /etc/aurorawm
    echo "Note: User configs in ~/.config/aurorawm are not removed."
fi

echo "Uninstallation complete!"

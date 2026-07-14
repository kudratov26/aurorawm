#!/bin/bash
set -e

echo "Installing AuroraWM..."

# Check if running as root
if [ "$EUID" -ne 0 ]; then 
    echo "Please run as root or use sudo"
    exit 1
fi

# Build the project
echo "Building AuroraWM..."
cargo build --release

# Install binary
echo "Installing binary to /usr/local/bin..."
install -Dm755 target/release/aurorawm /usr/local/bin/aurorawm

# Install session script
echo "Installing session script..."
install -Dm755 aurorawm-session /usr/local/bin/aurorawm-session

# Install desktop entry
echo "Installing desktop entry..."
mkdir -p /usr/share/wayland-sessions
install -Dm644 aurorawm.desktop /usr/share/wayland-sessions/aurorawm.desktop

# Install systemd service (optional)
read -p "Install systemd service? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo "Installing systemd service..."
    install -Dm644 aurorawm.service /usr/lib/systemd/system/aurorawm.service
    systemctl daemon-reload
    echo "Systemd service installed. Enable with: systemctl --user enable aurorawm"
fi

echo "Installation complete!"
echo "You can now start AuroraWM from your display manager or by running 'aurorawm' from a TTY."

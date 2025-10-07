#!/bin/bash

# Photo Booth Package Installation and Setup Script
# This script installs all required packages and configures the v4l2loopback device
# for the Canon Test Cam photo booth application on Debian Bookworm

set -e

echo "============================================"
echo "Photo Booth System Setup"
echo "============================================"
echo ""

# Check if running as root or with sudo
if [ "$EUID" -eq 0 ]; then
   echo "Running as root"
   SUDO=""
else
   echo "Running as user, will use sudo for privileged commands"
   SUDO="sudo"
fi

echo ""
echo "============================================"
echo "Installing System Packages"
echo "============================================"
echo ""

# Update package lists
echo "Updating package lists..."
$SUDO apt-get update

# Build essentials for compilation
echo ""
echo "Installing build essentials..."
$SUDO apt-get install -y \
    build-essential \
    pkg-config \
    curl \
    ca-certificates \
    git \
    clang \
    libclang-dev

# Camera support packages
echo ""
echo "Installing camera support packages..."
$SUDO apt-get install -y \
    gphoto2 \
    libgphoto2-dev \
    libgphoto2-port12 \
    v4l-utils \
    v4l2loopback-dkms \
    v4l2loopback-utils \
    libv4l-dev

# Printing support packages
echo ""
echo "Installing printing support packages..."
$SUDO apt-get install -y \
    cups \
    cups-client \
    cups-bsd \
    libcups2-dev

# Display and kiosk mode packages
echo ""
echo "Installing display and kiosk mode packages..."
# Try to install chromium (Debian 13+) or chromium-browser (older versions)
if apt-cache show chromium &>/dev/null; then
    echo "Installing chromium package (Debian 13+)..."
    $SUDO apt-get install -y chromium
else
    echo "Installing chromium-browser package..."
    $SUDO apt-get install -y chromium-browser
fi
$SUDO apt-get install -y \
    xorg \
    xinit \
    openbox \
    lightdm

# System utilities
echo ""
echo "Installing system utilities..."
$SUDO apt-get install -y \
    systemd \
    udev \
    ssh \
    sudo \
    htop

# FFmpeg for video processing (required for gphoto2 streaming)
echo ""
echo "Installing video processing tools..."
$SUDO apt-get install -y \
    ffmpeg

# Additional dependencies for Rust applications
echo ""
echo "Installing Rust application dependencies..."
$SUDO apt-get install -y \
    libssl-dev \
    libsqlite3-dev

# Linux headers for v4l2loopback
echo ""
echo "Installing Linux headers for v4l2loopback..."
$SUDO apt-get install -y \
    linux-headers-$(uname -r)

echo ""
echo "============================================"
echo "Configuring V4L2 Loopback Device"
echo "============================================"
echo ""

# Remove v4l2loopback module if already loaded
echo "Removing existing v4l2loopback module if present..."
$SUDO modprobe -r v4l2loopback 2>/dev/null || true

# Load v4l2loopback module with proper parameters
echo "Loading v4l2loopback module..."
$SUDO modprobe v4l2loopback \
    exclusive_caps=1 \
    max_buffers=2 \
    card_label="Canon EOS Rebel T7"

# Make v4l2loopback persistent across reboots
echo "Making v4l2loopback persistent..."
echo "v4l2loopback" | $SUDO tee /etc/modules-load.d/v4l2loopback.conf > /dev/null

# Configure v4l2loopback module options
echo "options v4l2loopback exclusive_caps=1 max_buffers=2 card_label=\"Canon EOS Rebel T7\"" | \
    $SUDO tee /etc/modprobe.d/v4l2loopback.conf > /dev/null

# Create udev rules for Canon camera
echo "Creating udev rules for Canon EOS Rebel T7..."
cat << 'EOF' | $SUDO tee /etc/udev/rules.d/99-canon-eos.rules > /dev/null
# Canon EOS Rebel T7 / EOS 2000D
SUBSYSTEM=="usb", ATTR{idVendor}=="04a9", ATTR{idProduct}=="32da", MODE="0666", GROUP="plugdev"
SUBSYSTEM=="usb", ATTR{idVendor}=="04a9", MODE="0666", GROUP="plugdev"
EOF

# Reload udev rules
echo "Reloading udev rules..."
$SUDO udevadm control --reload-rules
$SUDO udevadm trigger

# Add current user to required groups
if [ "$EUID" -ne 0 ]; then
    echo "Adding user to required groups..."
    $SUDO usermod -a -G video,plugdev,lpadmin $USER
    echo "Note: You may need to log out and back in for group changes to take effect"
fi

# Verify v4l2loopback device creation
echo ""
echo "Checking v4l2loopback devices..."
ls -la /dev/video* 2>/dev/null || echo "No video devices found yet"

echo ""
echo "============================================"
echo "Setup Complete"
echo "============================================"
echo ""
echo "Next steps:"
echo "1. Download and install TurboPrint driver for Epson XP-8700:"
echo "   wget https://www.zedonet.com/download/tp2/arm/turboprint-2.55-1.arm64.tgz"
echo "   tar -xzf turboprint-2.55-1.arm64.tgz"
echo "   cd turboprint-2.55-1"
echo "   sudo ./setup"
echo "   tpsetup --install turboprint2.tpkey"
echo ""
echo "2. Configure printer with TurboPrint:"
echo "   sudo tpconfig"
echo ""
echo "3. If you added the user to groups, log out and back in"
echo ""
echo "4. Connect Canon EOS Rebel T7 and verify detection:"
echo "   gphoto2 --auto-detect"
echo ""
echo "5. Deploy the application using ./deploy.sh from your Mac"
echo ""

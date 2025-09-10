#!/bin/bash

# Startup script for running the photo booth application
# This script ensures proper setup and configuration for Canon EOS Rebel T7 via GPhoto2

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
BINARY_PATH="${BINARY_PATH:-/home/prospero/photobooth/cam_test}"
V4L2_DEVICE="${V4L2_LOOPBACK_DEVICE:-/dev/video2}"
V4L2_VIDEO_NR="2"  # The number for video device (video2 = 2)
LOG_FILE="/tmp/photobooth.log"

# Function to print colored messages
print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

print_info() {
    echo -e "${YELLOW}ℹ${NC} $1"
}

# Function to cleanup on exit
cleanup() {
    print_info "Cleaning up..."
    pkill -f gphoto2 2>/dev/null || true
    pkill -f cam_test 2>/dev/null || true
}

# Set trap for cleanup on exit
trap cleanup EXIT

echo "==================================="
echo "Photo Booth Startup Script"
echo "==================================="
echo ""

# Step 1: Kill any existing processes
print_info "Stopping any existing processes..."
pkill -f gphoto2 2>/dev/null || true
pkill -f cam_test 2>/dev/null || true
sleep 1

# Step 2: Check if camera is connected
print_info "Checking camera connection..."
if gphoto2 --auto-detect 2>&1 | grep -q "usb:"; then
    print_success "Camera detected"
else
    print_error "No camera detected. Please check:"
    echo "  - Camera is connected via USB"
    echo "  - Camera is turned on"
    echo "  - Camera is in the correct mode (not Mass Storage)"
    exit 1
fi

# Step 3: Load v4l2loopback module if needed
print_info "Checking v4l2loopback module..."
if ! lsmod | grep -q v4l2loopback; then
    print_info "Loading v4l2loopback module..."
    sudo modprobe v4l2loopback devices=1 video_nr=$V4L2_VIDEO_NR card_label="Canon EOS Preview" exclusive_caps=1
    sleep 1
fi

# Step 4: Verify v4l2 device exists
if [ -e "$V4L2_DEVICE" ]; then
    print_success "v4l2loopback device ready: $V4L2_DEVICE"
else
    print_error "v4l2loopback device not found: $V4L2_DEVICE"
    exit 1
fi

# Step 5: Set permissions on v4l2 device
print_info "Setting permissions on $V4L2_DEVICE..."
sudo chmod 666 "$V4L2_DEVICE" 2>/dev/null || true

# Step 6: Create necessary directories
print_info "Creating necessary directories..."
mkdir -p /home/prospero/photobooth/images
mkdir -p /home/prospero/photobooth/static
chmod 755 /home/prospero/photobooth/images
chmod 755 /home/prospero/photobooth/static

# Step 7: Export environment variables
print_info "Setting environment variables..."
export V4L2_LOOPBACK_DEVICE="$V4L2_DEVICE"
export RUST_LOG=info
export RUST_BACKTRACE=1

# Step 8: Check if binary exists
if [ ! -f "$BINARY_PATH" ]; then
    print_error "Binary not found at: $BINARY_PATH"
    exit 1
fi

# Step 9: Make binary executable
chmod +x "$BINARY_PATH"

# Step 10: Display configuration
echo ""
echo "Configuration:"
echo "  Binary: $BINARY_PATH"
echo "  V4L2 Device: $V4L2_DEVICE"
echo "  Camera: Canon EOS (GPhoto2)"
echo "  Log Level: info"
echo ""

# Step 11: Run the application
print_success "Starting photo booth application..."
echo "Logs will be written to: $LOG_FILE"
echo ""
echo "Press Ctrl+C to stop"
echo "==================================="
echo ""

# Run the application with logging
exec "$BINARY_PATH" 2>&1 | tee "$LOG_FILE"

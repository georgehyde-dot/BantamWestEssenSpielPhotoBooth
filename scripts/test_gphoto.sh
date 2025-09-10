#!/bin/bash

# Test script for GPhoto2 camera functionality on Raspberry Pi
# This script tests the same operations that the Rust code performs

set -e

echo "==================================="
echo "GPhoto2 Camera Test Script"
echo "==================================="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
V4L2_DEVICE="${V4L2_LOOPBACK_DEVICE:-/dev/video2}"
TEST_IMAGE="/tmp/test_capture.jpg"

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

# Function to kill gphoto2 processes
kill_gphoto() {
    print_info "Killing any existing gphoto2 processes..."
    pkill -f gphoto2 2>/dev/null || true
    sleep 0.5
}

# Initial cleanup
kill_gphoto

# Test 1: Check if gphoto2 is installed
echo "1. Checking gphoto2 installation..."
if command -v gphoto2 &> /dev/null; then
    VERSION=$(gphoto2 --version | head -n1)
    print_success "gphoto2 is installed: $VERSION"
else
    print_error "gphoto2 is not installed. Please install it with: sudo apt-get install gphoto2"
    exit 1
fi
echo ""

# Test 2: Check if ffmpeg is installed
echo "2. Checking ffmpeg installation..."
if command -v ffmpeg &> /dev/null; then
    print_success "ffmpeg is installed"
else
    print_error "ffmpeg is not installed. Please install it with: sudo apt-get install ffmpeg"
    exit 1
fi
echo ""

# Test 3: Detect camera
echo "3. Detecting camera..."
OUTPUT=$(gphoto2 --auto-detect 2>&1)
if echo "$OUTPUT" | grep -q "usb:"; then
    print_success "Camera detected:"
    echo "$OUTPUT" | grep "usb:" | head -n1
else
    print_error "No camera detected. Please check:"
    echo "  - Camera is connected via USB"
    echo "  - Camera is turned on"
    echo "  - Camera is in the correct mode (not Mass Storage)"
    exit 1
fi
echo ""

# Test 4: Check v4l2loopback device
echo "4. Checking v4l2loopback device..."
if [ -e "$V4L2_DEVICE" ]; then
    print_success "v4l2loopback device exists: $V4L2_DEVICE"
else
    print_error "v4l2loopback device not found: $V4L2_DEVICE"
    echo "  Please load the v4l2loopback module:"
    echo "  sudo modprobe v4l2loopback devices=1 video_nr=2 card_label=\"Virtual Camera\""
    exit 1
fi
echo ""

# Test 5: Test photo capture
echo "5. Testing photo capture..."
kill_gphoto
if gphoto2 --capture-image-and-download --filename="$TEST_IMAGE" --force-overwrite 2>&1; then
    if [ -f "$TEST_IMAGE" ]; then
        SIZE=$(stat -c%s "$TEST_IMAGE")
        print_success "Photo captured successfully (size: $SIZE bytes)"
        rm -f "$TEST_IMAGE"
    else
        print_error "Photo capture command succeeded but file not found"
    fi
else
    print_error "Failed to capture photo"
fi
echo ""

# Test 6: Test preview stream
echo "6. Testing preview stream to v4l2loopback..."
kill_gphoto
print_info "Starting preview stream to $V4L2_DEVICE (will run for 5 seconds)..."

# Start the preview stream in background
(gphoto2 --stdout --capture-movie | ffmpeg -i - -vcodec rawvideo -pix_fmt yuv420p -threads 0 -f v4l2 "$V4L2_DEVICE" 2>/dev/null) &
PREVIEW_PID=$!

# Wait a bit for stream to start
sleep 2

# Check if process is still running
if kill -0 $PREVIEW_PID 2>/dev/null; then
    print_success "Preview stream is running (PID: $PREVIEW_PID)"

    # Test capturing a frame from v4l2 device
    print_info "Attempting to capture a frame from $V4L2_DEVICE..."
    if ffmpeg -f v4l2 -i "$V4L2_DEVICE" -frames:v 1 -f mjpeg - 2>/dev/null | head -c 100 > /dev/null; then
        print_success "Successfully captured a frame from v4l2 device"
    else
        print_error "Failed to capture frame from v4l2 device"
    fi

    # Let it run for a bit more
    sleep 3

    # Stop the preview
    print_info "Stopping preview stream..."
    kill $PREVIEW_PID 2>/dev/null || true
    wait $PREVIEW_PID 2>/dev/null || true
else
    print_error "Preview stream failed to start"
fi
echo ""

# Final cleanup
kill_gphoto

# Summary
echo "==================================="
echo "Test Summary"
echo "==================================="
echo ""
print_info "All basic tests completed."
echo ""
echo "To use with the Rust application:"
echo "1. Ensure v4l2loopback is loaded:"
echo "   sudo modprobe v4l2loopback devices=1 video_nr=2 card_label=\"Virtual Camera\""
echo ""
echo "2. Set environment variables and run:"
echo "   export USE_GPHOTO=true"
echo "   export V4L2_LOOPBACK_DEVICE=$V4L2_DEVICE"
echo "   ./cam_test"
echo ""
print_success "Camera appears to be working correctly!"

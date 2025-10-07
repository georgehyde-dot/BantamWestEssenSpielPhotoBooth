#!/bin/bash

# V4L2 Loopback Testing and Diagnostic Script
# This script tests and diagnoses v4l2 loopback issues for the photo booth

set -e

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
print_header() {
    echo ""
    echo -e "${BLUE}============================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}============================================${NC}"
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

print_info() {
    echo -e "  $1"
}

# Start diagnostics
echo -e "${BLUE}V4L2 Loopback Diagnostic and Testing${NC}"
echo -e "${BLUE}$(date)${NC}"

# Check if running as root for some operations
if [ "$EUID" -eq 0 ]; then
   SUDO=""
else
   SUDO="sudo"
fi

# Step 1: Check v4l2loopback module
print_header "V4L2 Loopback Module Status"

if lsmod | grep -q v4l2loopback; then
    print_success "v4l2loopback module is loaded"

    # Get module info
    echo ""
    print_info "Module parameters:"
    if [ -d /sys/module/v4l2loopback/parameters ]; then
        for param in /sys/module/v4l2loopback/parameters/*; do
            if [ -f "$param" ]; then
                param_name=$(basename "$param")
                param_value=$(cat "$param" 2>/dev/null || echo "unknown")
                print_info "  $param_name: $param_value"
            fi
        done
    fi
else
    print_error "v4l2loopback module is NOT loaded"
    print_info "Loading module..."
    $SUDO modprobe v4l2loopback exclusive_caps=1 max_buffers=2 card_label="Canon EOS Rebel T7"

    if lsmod | grep -q v4l2loopback; then
        print_success "Module loaded successfully"
    else
        print_error "Failed to load module"
        exit 1
    fi
fi

# Step 2: List video devices
print_header "Video Devices"

for device in /dev/video*; do
    if [ -e "$device" ]; then
        print_info "Device: $device"

        # Get device capabilities
        if command -v v4l2-ctl >/dev/null 2>&1; then
            # Get device name
            device_name=$(v4l2-ctl --device=$device --info 2>/dev/null | grep "Card type" | cut -d: -f2 | xargs || echo "Unknown")
            print_info "  Name: $device_name"

            # Check if it's a loopback device
            if echo "$device_name" | grep -qi "loopback\|Canon EOS"; then
                print_success "  This appears to be the loopback device"
                LOOPBACK_DEVICE="$device"
            fi

            # Get formats
            formats=$(v4l2-ctl --device=$device --list-formats 2>/dev/null | grep "Pixel Format" | head -n 3 || echo "")
            if [ ! -z "$formats" ]; then
                print_info "  Supported formats:"
                echo "$formats" | while read line; do
                    print_info "    $line"
                done
            fi
        fi

        # Check if device is in use
        if lsof "$device" 2>/dev/null | grep -q "$device"; then
            print_warning "  Device is currently in use by:"
            lsof "$device" 2>/dev/null | tail -n +2 | while read line; do
                print_info "    $line"
            done
        else
            print_info "  Device is not in use"
        fi
    fi
done

if [ -z "$LOOPBACK_DEVICE" ]; then
    # Try to find the highest numbered video device (usually the loopback)
    LOOPBACK_DEVICE=$(ls /dev/video* 2>/dev/null | sort -V | tail -n1)
    print_warning "Could not identify loopback device, assuming: $LOOPBACK_DEVICE"
fi

# Step 3: Check camera detection
print_header "Camera Detection"

if command -v gphoto2 >/dev/null 2>&1; then
    camera_output=$(gphoto2 --auto-detect 2>&1)

    if echo "$camera_output" | grep -q "usb:"; then
        print_success "Camera detected via USB"
        echo "$camera_output" | grep "usb:" | while read line; do
            print_info "$line"
        done
    else
        print_error "No camera detected via USB"
        print_info "Cannot test streaming without a camera"
    fi
else
    print_error "gphoto2 is not installed"
fi

# Step 4: Kill any existing streaming processes
print_header "Cleaning Up Existing Processes"

# Kill any existing gphoto2 processes
if pgrep -f gphoto2 >/dev/null 2>&1; then
    print_warning "Found existing gphoto2 processes, killing them..."
    pkill -f gphoto2 || true
    sleep 1
    pkill -9 -f gphoto2 2>/dev/null || true
    print_success "Killed gphoto2 processes"
fi

# Kill any ffmpeg processes using v4l2
if pgrep -f "ffmpeg.*v4l2" >/dev/null 2>&1; then
    print_warning "Found existing ffmpeg processes, killing them..."
    pkill -f "ffmpeg.*v4l2" || true
    sleep 1
    print_success "Killed ffmpeg processes"
fi

# Step 5: Test gphoto2 streaming
print_header "Testing GPhoto2 Streaming"

if [ ! -z "$LOOPBACK_DEVICE" ] && command -v gphoto2 >/dev/null 2>&1; then
    print_info "Starting test stream to $LOOPBACK_DEVICE for 5 seconds..."
    print_info "Command: gphoto2 --stdout --capture-movie | ffmpeg -i - -vcodec rawvideo -pix_fmt yuv420p -f v4l2 $LOOPBACK_DEVICE"

    # Start the streaming in background
    timeout 5 bash -c "gphoto2 --stdout --capture-movie 2>/tmp/gphoto2_err.log | ffmpeg -i - -vcodec rawvideo -pix_fmt yuv420p -f v4l2 $LOOPBACK_DEVICE 2>/tmp/ffmpeg_err.log" &
    STREAM_PID=$!

    # Wait a moment for stream to start
    sleep 2

    # Check if stream is running
    if kill -0 $STREAM_PID 2>/dev/null; then
        print_success "Stream appears to be running"

        # Try to capture a frame
        if command -v ffmpeg >/dev/null 2>&1; then
            print_info "Attempting to capture a test frame..."
            timeout 2 ffmpeg -f v4l2 -i $LOOPBACK_DEVICE -frames:v 1 /tmp/test_frame.jpg -y 2>/dev/null

            if [ -f /tmp/test_frame.jpg ]; then
                frame_size=$(du -h /tmp/test_frame.jpg | cut -f1)
                print_success "Successfully captured test frame (size: $frame_size)"
                rm /tmp/test_frame.jpg
            else
                print_warning "Could not capture test frame"
            fi
        fi
    else
        print_error "Stream failed to start"
    fi

    # Wait for timeout
    wait $STREAM_PID 2>/dev/null || true

    # Check error logs
    if [ -f /tmp/gphoto2_err.log ] && [ -s /tmp/gphoto2_err.log ]; then
        print_warning "GPhoto2 errors:"
        head -n 5 /tmp/gphoto2_err.log | while read line; do
            print_info "$line"
        done
    fi

    if [ -f /tmp/ffmpeg_err.log ] && [ -s /tmp/ffmpeg_err.log ]; then
        print_warning "FFmpeg errors:"
        grep -v "frame=" /tmp/ffmpeg_err.log | head -n 5 | while read line; do
            print_info "$line"
        done
    fi

    # Clean up
    rm -f /tmp/gphoto2_err.log /tmp/ffmpeg_err.log
else
    print_warning "Cannot test streaming (missing camera or gphoto2)"
fi

# Step 6: Test direct v4l2 device reading
print_header "Testing V4L2 Device Reading"

if [ ! -z "$LOOPBACK_DEVICE" ]; then
    print_info "Testing if $LOOPBACK_DEVICE can be read..."

    # Try with v4l2-ctl
    if command -v v4l2-ctl >/dev/null 2>&1; then
        if timeout 1 v4l2-ctl --device=$LOOPBACK_DEVICE --stream-mmap --stream-count=1 2>/dev/null; then
            print_success "v4l2-ctl can read from device"
        else
            print_warning "v4l2-ctl cannot read from device (may need active stream)"
        fi
    fi

    # Try with ffmpeg
    if command -v ffmpeg >/dev/null 2>&1; then
        print_info "Testing ffmpeg read (will timeout if no stream)..."
        if timeout 2 ffmpeg -f v4l2 -i $LOOPBACK_DEVICE -frames:v 1 -f null - 2>/dev/null; then
            print_success "FFmpeg can read from device"
        else
            print_info "FFmpeg cannot read (expected if no active stream)"
        fi
    fi
fi

# Step 7: Check common issues
print_header "Common Issues Check"

# Check permissions
if [ ! -z "$LOOPBACK_DEVICE" ]; then
    perms=$(ls -l $LOOPBACK_DEVICE | awk '{print $1, $3, $4}')
    print_info "Device permissions: $perms"

    if [ ! -r "$LOOPBACK_DEVICE" ] || [ ! -w "$LOOPBACK_DEVICE" ]; then
        print_warning "Current user may not have read/write access"
        print_info "Fix with: sudo chmod 666 $LOOPBACK_DEVICE"
    fi
fi

# Check if user is in video group
if groups | grep -q video; then
    print_success "User is in video group"
else
    print_warning "User is NOT in video group"
    print_info "Fix with: sudo usermod -a -G video $USER"
    print_info "(You'll need to log out and back in)"
fi

# Step 8: Recommendations
print_header "Recommendations"

echo ""
if [ ! -z "$LOOPBACK_DEVICE" ]; then
    print_info "1. The v4l2 loopback device is: $LOOPBACK_DEVICE"
    print_info "   Update your application config if needed:"
    print_info "   export VIDEO_DEVICE=$LOOPBACK_DEVICE"
    echo ""
fi

print_info "2. To manually test the streaming pipeline:"
print_info "   # Terminal 1 - Start stream:"
print_info "   gphoto2 --stdout --capture-movie | ffmpeg -i - -vcodec rawvideo -pix_fmt yuv420p -f v4l2 $LOOPBACK_DEVICE"
print_info ""
print_info "   # Terminal 2 - View stream:"
print_info "   ffplay -f v4l2 -video_size 1920x1080 -i $LOOPBACK_DEVICE"
print_info "   # Or with ffmpeg:"
print_info "   ffmpeg -f v4l2 -video_size 1920x1080 -i $LOOPBACK_DEVICE -f mjpeg -q:v 5 -r 30 - | ffplay -"
echo ""

print_info "3. If the stream immediately stops:"
print_info "   - The v4l2 device might be wrong (try /dev/video0, /dev/video1, /dev/video2)"
print_info "   - The gphoto2 stream might not be outputting data"
print_info "   - Try running gphoto2 --stdout --capture-movie alone to test camera"
echo ""

print_info "4. To fix v4l2loopback permanently:"
print_info "   echo 'options v4l2loopback exclusive_caps=1 max_buffers=2 card_label=\"Canon EOS Rebel T7\"' | sudo tee /etc/modprobe.d/v4l2loopback.conf"
print_info "   echo 'v4l2loopback' | sudo tee -a /etc/modules-load.d/v4l2loopback.conf"

# Summary
print_header "Summary"

if [ ! -z "$LOOPBACK_DEVICE" ]; then
    echo -e "${GREEN}Loopback device found: $LOOPBACK_DEVICE${NC}"
    echo ""
    echo "To use in your application:"
    echo "  export VIDEO_DEVICE=$LOOPBACK_DEVICE"
    echo "  ./cam_test"
else
    echo -e "${RED}Could not identify loopback device${NC}"
    echo "Please check v4l2loopback module installation"
fi

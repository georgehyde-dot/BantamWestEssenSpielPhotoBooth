#!/bin/bash

# Fix V4L2 Device Configuration
# This script identifies and configures the correct video device
# Supports both HD Pro Webcam C920 (direct) and Canon EOS (via loopback)

set -e

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}V4L2 Device Configuration${NC}"
echo "========================================"
echo ""

# Step 1: Check for HD Pro Webcam C920
echo "Checking for HD Pro Webcam C920..."
C920_DEVICE=""
for device in /dev/video*; do
    if [ -e "$device" ]; then
        # Check if this is the C920
        device_info=$(v4l2-ctl --device=$device --info 2>/dev/null || true)
        if echo "$device_info" | grep -qi "HD Pro Webcam C920"; then
            # Get the first video device for the C920 (usually the capture device)
            if [ -z "$C920_DEVICE" ]; then
                C920_DEVICE="$device"
                echo -e "${GREEN}✓${NC} Found HD Pro Webcam C920: $device"
            fi
        fi
    fi
done

# Determine which device to use
if [ -n "$C920_DEVICE" ]; then
    echo -e "${GREEN}Using HD Pro Webcam C920 directly (no loopback needed)${NC}"
    VIDEO_DEVICE="$C920_DEVICE"
    DEVICE_TYPE="webcam"
else
    echo -e "${YELLOW}HD Pro Webcam C920 not found, setting up Canon camera with loopback${NC}"
    DEVICE_TYPE="loopback"

    # Check if v4l2loopback is loaded
    echo ""
    echo "Checking v4l2loopback module..."
    if ! lsmod | grep -q v4l2loopback; then
        echo -e "${YELLOW}Loading v4l2loopback module...${NC}"
        sudo modprobe v4l2loopback exclusive_caps=1 max_buffers=2 card_label="Canon EOS Rebel T7"
        sleep 1
    fi

    # Find the loopback device
    echo ""
    echo "Scanning for loopback device..."
    LOOPBACK_DEVICE=""

    for device in /dev/video*; do
        if [ -e "$device" ]; then
            # Check if this is the loopback device
            if v4l2-ctl --device=$device --info 2>/dev/null | grep -qi "Canon EOS\|loopback"; then
                LOOPBACK_DEVICE="$device"
                echo -e "${GREEN}✓${NC} Found loopback device: $device"
                break
            fi
        fi
    done

    # If no loopback found, try the highest numbered device
    if [ -z "$LOOPBACK_DEVICE" ]; then
        LOOPBACK_DEVICE=$(ls /dev/video* 2>/dev/null | sort -V | tail -n1)
        echo -e "${YELLOW}⚠${NC} Could not identify loopback device by name"
        echo -e "${YELLOW}⚠${NC} Using highest numbered device: $LOOPBACK_DEVICE"
    fi

    if [ -z "$LOOPBACK_DEVICE" ]; then
        echo -e "${RED}✗${NC} No video devices found for loopback!"
        echo "Please ensure v4l2loopback module is loaded"
        exit 1
    fi

    VIDEO_DEVICE="$LOOPBACK_DEVICE"
fi

# Step 2: Test the device
echo ""
echo "Testing device: $VIDEO_DEVICE"

# Check permissions
if [ -r "$VIDEO_DEVICE" ] && [ -w "$VIDEO_DEVICE" ]; then
    echo -e "${GREEN}✓${NC} Device is readable and writable"
else
    echo -e "${YELLOW}⚠${NC} Fixing device permissions..."
    sudo chmod 666 $VIDEO_DEVICE
    echo -e "${GREEN}✓${NC} Permissions fixed"
fi

# Step 3: Update environment file
ENV_FILE="$HOME/.photobooth.env"
echo ""
echo "Updating environment configuration..."

if [ -f "$ENV_FILE" ]; then
    # Backup existing file
    cp "$ENV_FILE" "${ENV_FILE}.backup"
    echo "  Backed up existing config to ${ENV_FILE}.backup"

    # Update or add VIDEO_DEVICE and V4L2_LOOPBACK_DEVICE
    if grep -q "VIDEO_DEVICE" "$ENV_FILE"; then
        sed -i "s|VIDEO_DEVICE=.*|VIDEO_DEVICE=$VIDEO_DEVICE|" "$ENV_FILE"
        echo -e "${GREEN}✓${NC} Updated VIDEO_DEVICE in $ENV_FILE"
    else
        echo "export VIDEO_DEVICE=$VIDEO_DEVICE" >> "$ENV_FILE"
        echo -e "${GREEN}✓${NC} Added VIDEO_DEVICE to $ENV_FILE"
    fi

    if grep -q "V4L2_LOOPBACK_DEVICE" "$ENV_FILE"; then
        sed -i "s|V4L2_LOOPBACK_DEVICE=.*|V4L2_LOOPBACK_DEVICE=$VIDEO_DEVICE|" "$ENV_FILE"
        echo -e "${GREEN}✓${NC} Updated V4L2_LOOPBACK_DEVICE in $ENV_FILE"
    else
        echo "export V4L2_LOOPBACK_DEVICE=$VIDEO_DEVICE" >> "$ENV_FILE"
        echo -e "${GREEN}✓${NC} Added V4L2_LOOPBACK_DEVICE to $ENV_FILE"
    fi

    # Add DEVICE_TYPE for app to know which mode to use
    if grep -q "CAMERA_DEVICE_TYPE" "$ENV_FILE"; then
        sed -i "s|CAMERA_DEVICE_TYPE=.*|CAMERA_DEVICE_TYPE=$DEVICE_TYPE|" "$ENV_FILE"
    else
        echo "export CAMERA_DEVICE_TYPE=$DEVICE_TYPE" >> "$ENV_FILE"
    fi
else
    # Create new environment file
    cat > "$ENV_FILE" << EOF
# Photo Booth Environment Configuration
export VIDEO_DEVICE=$VIDEO_DEVICE
export V4L2_LOOPBACK_DEVICE=$VIDEO_DEVICE
export CAMERA_DEVICE_TYPE=$DEVICE_TYPE
export HOST=0.0.0.0
export PORT=8080
export STORAGE_PATH=/usr/local/share/photo_booth
export PRINTER_NAME=EPSON_XP_8700_Series_USB
export USE_MOCK_PRINTER=false
export RUST_LOG=info
EOF
    echo -e "${GREEN}✓${NC} Created new environment file: $ENV_FILE"
fi

# Step 4: Clean up existing processes
echo ""
echo "Cleaning up existing processes..."
pkill -f gphoto2 2>/dev/null || true
pkill -f "ffmpeg.*v4l2" 2>/dev/null || true
sleep 1
echo -e "${GREEN}✓${NC} Cleaned up existing processes"

# Step 5: Test the device based on type
echo ""
if [ "$DEVICE_TYPE" = "webcam" ]; then
    echo "Testing HD Pro Webcam C920..."

    # Try to capture a test frame
    if timeout 2 ffmpeg -f v4l2 -i $VIDEO_DEVICE -frames:v 1 -f null - 2>/dev/null; then
        echo -e "${GREEN}✓${NC} Successfully read from webcam"
    else
        echo -e "${YELLOW}⚠${NC} Could not read from webcam - may need to configure format"
    fi

    echo ""
    echo "Webcam device info:"
    v4l2-ctl --device=$VIDEO_DEVICE --list-formats-ext | head -n 20

else
    echo "Testing Canon camera loopback pipeline..."
    echo "Starting 5-second test stream..."

    # Run test stream in background
    timeout 5 bash -c "gphoto2 --stdout --capture-movie 2>/tmp/gphoto_test.err | ffmpeg -i - -vcodec rawvideo -pix_fmt yuv420p -f v4l2 $VIDEO_DEVICE 2>/tmp/ffmpeg_test.err" &
    TEST_PID=$!

    # Wait for stream to start
    sleep 2

    if kill -0 $TEST_PID 2>/dev/null; then
        echo -e "${GREEN}✓${NC} Test stream is running"

        # Try to read a frame
        if timeout 1 ffmpeg -f v4l2 -i $VIDEO_DEVICE -frames:v 1 -f null - 2>/dev/null; then
            echo -e "${GREEN}✓${NC} Successfully read from loopback device"
        else
            echo -e "${YELLOW}⚠${NC} Could not read from loopback device"
        fi
    else
        echo -e "${RED}✗${NC} Test stream failed to start"

        # Show errors if any
        if [ -s /tmp/gphoto_test.err ]; then
            echo "GPhoto2 errors:"
            head -n 3 /tmp/gphoto_test.err
        fi
        if [ -s /tmp/ffmpeg_test.err ]; then
            echo "FFmpeg errors:"
            grep -v "frame=" /tmp/ffmpeg_test.err | head -n 3
        fi
    fi

    # Wait for test to complete
    wait $TEST_PID 2>/dev/null || true
    rm -f /tmp/gphoto_test.err /tmp/ffmpeg_test.err
fi

# Step 6: Summary and recommendations
echo ""
echo -e "${BLUE}============================================${NC}"
echo -e "${BLUE}Configuration Summary${NC}"
echo -e "${BLUE}============================================${NC}"
echo ""
echo -e "Device Type: ${GREEN}$DEVICE_TYPE${NC}"
echo -e "Video Device: ${GREEN}$VIDEO_DEVICE${NC}"
echo ""
echo "Next steps:"
echo "1. Source the environment file:"
echo "   source ~/.photobooth.env"
echo ""
echo "2. Run the application:"
echo "   ./cam_test"
echo ""

if [ "$DEVICE_TYPE" = "webcam" ]; then
    echo "Using HD Pro Webcam C920 directly."
    echo "The application will read from $VIDEO_DEVICE without loopback."
else
    echo "Using Canon camera with loopback."
    echo ""
    echo "If preview doesn't work, try:"
    echo "1. Check camera connection:"
    echo "   gphoto2 --auto-detect"
    echo ""
    echo "2. Test manual streaming:"
    echo "   # Terminal 1:"
    echo "   gphoto2 --stdout --capture-movie | ffmpeg -i - -vcodec rawvideo -pix_fmt yuv420p -f v4l2 $VIDEO_DEVICE"
    echo "   # Terminal 2:"
    echo "   ffplay -f v4l2 -i $VIDEO_DEVICE"
fi

echo ""
echo "3. Check application logs:"
echo "   tail -f ~/photobooth.log"
echo ""
echo -e "${GREEN}Configuration complete!${NC}"

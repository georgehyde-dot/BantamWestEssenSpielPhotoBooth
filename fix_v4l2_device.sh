#!/bin/bash

# Fix V4L2 Device Configuration
# This script identifies and configures the correct video device
# Priority: 1. Canon EOS (via loopback), 2. HD Pro Webcam C920 (direct)

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

# Initialize variables
VIDEO_DEVICE=""
DEVICE_TYPE=""
CANON_FOUND=false
C920_DEVICE=""

# Step 1: Check for Canon camera first
echo "Checking for Canon EOS camera..."
if gphoto2 --auto-detect 2>/dev/null | grep -qi "Canon"; then
    echo -e "${GREEN}✓${NC} Canon camera detected via USB"
    CANON_FOUND=true

    # Set up loopback for Canon
    echo "Setting up v4l2loopback for Canon camera..."

    # Check if v4l2loopback is loaded
    if ! lsmod | grep -q v4l2loopback; then
        echo -e "${YELLOW}Loading v4l2loopback module...${NC}"
        sudo modprobe v4l2loopback exclusive_caps=1 max_buffers=2 card_label="Canon EOS Rebel T7"
        sleep 1
    fi

    # Find the loopback device
    echo "Scanning for loopback device..."
    LOOPBACK_DEVICE=""

    for device in /dev/video*; do
        if [ -e "$device" ]; then
            # Check if this is the loopback device
            if v4l2-ctl --device=$device --info 2>/dev/null | grep -qi "Canon EOS\|loopback\|v4l2 loopback"; then
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

    if [ -n "$LOOPBACK_DEVICE" ]; then
        VIDEO_DEVICE="$LOOPBACK_DEVICE"
        DEVICE_TYPE="loopback"
        echo -e "${GREEN}Canon camera will use loopback device: $VIDEO_DEVICE${NC}"
    else
        echo -e "${RED}✗${NC} Canon camera found but no loopback device available"
        CANON_FOUND=false
    fi
else
    echo -e "${YELLOW}Canon camera not detected${NC}"
fi

# Step 2: If no Canon, check for HD Pro Webcam C920
if [ "$CANON_FOUND" = false ]; then
    echo ""
    echo "Checking for HD Pro Webcam C920..."

    # Find all video devices and check for C920
    for device in /dev/video*; do
        if [ -e "$device" ]; then
            device_info=$(v4l2-ctl --device=$device --info 2>/dev/null || true)
            if echo "$device_info" | grep -qi "HD Pro Webcam C920"; then
                # Check if this is a capture device (not metadata)
                if v4l2-ctl --device=$device --list-formats 2>/dev/null | grep -q "MJPG\|YUYV"; then
                    C920_DEVICE="$device"
                    echo -e "${GREEN}✓${NC} Found HD Pro Webcam C920 capture device: $device"

                    # List device details
                    echo "  Device capabilities:"
                    v4l2-ctl --device=$device --list-formats-ext 2>/dev/null | head -n 10 | sed 's/^/    /'
                    break
                fi
            fi
        fi
    done

    if [ -n "$C920_DEVICE" ]; then
        VIDEO_DEVICE="$C920_DEVICE"
        DEVICE_TYPE="webcam"
        echo -e "${GREEN}Using HD Pro Webcam C920 at: $VIDEO_DEVICE${NC}"
    fi
fi

# Step 3: Handle no camera found
if [ -z "$VIDEO_DEVICE" ]; then
    echo ""
    echo -e "${RED}✗ No supported camera found!${NC}"
    echo "Neither Canon camera nor HD Pro Webcam C920 detected."
    echo ""

    # Use a fallback device or create a dummy
    # Try to find any video device
    FALLBACK_DEVICE=$(ls /dev/video* 2>/dev/null | head -n1)
    if [ -n "$FALLBACK_DEVICE" ]; then
        VIDEO_DEVICE="$FALLBACK_DEVICE"
        DEVICE_TYPE="unknown"
        echo -e "${YELLOW}Using fallback device: $VIDEO_DEVICE${NC}"
        echo "This may not work correctly!"
    else
        # No video devices at all
        VIDEO_DEVICE="/dev/video0"
        DEVICE_TYPE="none"
        echo -e "${RED}No video devices found at all!${NC}"
        echo "Setting VIDEO_DEVICE=$VIDEO_DEVICE as placeholder"
    fi
fi

# Step 4: Test the device (if not "none")
if [ "$DEVICE_TYPE" != "none" ] && [ -e "$VIDEO_DEVICE" ]; then
    echo ""
    echo "Testing device: $VIDEO_DEVICE"

    # Check permissions
    if [ -r "$VIDEO_DEVICE" ] && [ -w "$VIDEO_DEVICE" ]; then
        echo -e "${GREEN}✓${NC} Device is readable and writable"
    else
        echo -e "${YELLOW}⚠${NC} Fixing device permissions..."
        sudo chmod 666 $VIDEO_DEVICE 2>/dev/null || true
        echo -e "${GREEN}✓${NC} Permissions updated"
    fi

    # Quick test based on device type
    if [ "$DEVICE_TYPE" = "webcam" ] || [ "$DEVICE_TYPE" = "unknown" ]; then
        echo "Testing direct capture..."
        if timeout 2 ffmpeg -f v4l2 -i $VIDEO_DEVICE -frames:v 1 -f null - 2>/dev/null; then
            echo -e "${GREEN}✓${NC} Successfully read from device"
        else
            echo -e "${YELLOW}⚠${NC} Could not read from device - may need format configuration"
        fi
    elif [ "$DEVICE_TYPE" = "loopback" ]; then
        echo "Loopback device configured for Canon camera"
        echo "Preview stream will be started by the application"
    fi
fi

# Step 5: Kill any existing streaming processes
echo ""
echo "Cleaning up existing processes..."
pkill -f gphoto2 2>/dev/null || true
pkill -f "ffmpeg.*v4l2" 2>/dev/null || true
sleep 1
echo -e "${GREEN}✓${NC} Cleaned up existing processes"

# Step 6: Update environment file
ENV_FILE="$HOME/.photobooth.env"
echo ""
echo "Updating environment configuration..."

# Create backup if file exists
if [ -f "$ENV_FILE" ]; then
    cp "$ENV_FILE" "${ENV_FILE}.backup"
    echo "  Backed up existing config to ${ENV_FILE}.backup"
fi

# Write new configuration
cat > "$ENV_FILE" << EOF
# Photo Booth Environment Configuration
# Generated on $(date)
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

echo -e "${GREEN}✓${NC} Updated environment file: $ENV_FILE"

# Step 7: Test streaming pipeline for Canon
if [ "$DEVICE_TYPE" = "loopback" ] && [ "$CANON_FOUND" = true ]; then
    echo ""
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
            grep -v "frame=" /tmp/ffmpeg_test.err 2>/dev/null | head -n 3
        fi
    fi

    # Wait for test to complete
    wait $TEST_PID 2>/dev/null || true
    rm -f /tmp/gphoto_test.err /tmp/ffmpeg_test.err
fi

# Step 8: Summary and recommendations
echo ""
echo -e "${BLUE}============================================${NC}"
echo -e "${BLUE}Configuration Summary${NC}"
echo -e "${BLUE}============================================${NC}"
echo ""
echo -e "Device Type:  ${GREEN}$DEVICE_TYPE${NC}"
echo -e "Video Device: ${GREEN}$VIDEO_DEVICE${NC}"
echo ""

case "$DEVICE_TYPE" in
    "loopback")
        echo "✓ Canon camera configured with v4l2loopback"
        echo "  The application will handle the streaming pipeline"
        ;;
    "webcam")
        echo "✓ HD Pro Webcam C920 configured for direct capture"
        echo "  No loopback required"
        ;;
    "unknown")
        echo "⚠ Unknown camera device detected"
        echo "  Basic V4L2 capture will be attempted"
        ;;
    "none")
        echo "✗ No camera available"
        echo "  The application will handle this gracefully"
        ;;
esac

echo ""
echo "Next steps:"
echo "1. Source the environment file:"
echo "   source ~/.photobooth.env"
echo ""
echo "2. Verify settings:"
echo "   echo \$VIDEO_DEVICE"
echo "   echo \$CAMERA_DEVICE_TYPE"
echo ""
echo "3. Run the application:"
echo "   ./cam_test"
echo ""

if [ "$DEVICE_TYPE" = "loopback" ]; then
    echo "For manual Canon testing:"
    echo "  Terminal 1: gphoto2 --stdout --capture-movie | ffmpeg -i - -vcodec rawvideo -pix_fmt yuv420p -f v4l2 $VIDEO_DEVICE"
    echo "  Terminal 2: ffplay -f v4l2 -i $VIDEO_DEVICE"
elif [ "$DEVICE_TYPE" = "webcam" ]; then
    echo "For manual C920 testing:"
    echo "  Preview: ffplay -f v4l2 -i $VIDEO_DEVICE"
    echo "  Capture: ffmpeg -f v4l2 -i $VIDEO_DEVICE -frames:v 1 test.jpg"
fi

echo ""
echo -e "${GREEN}Configuration complete!${NC}"

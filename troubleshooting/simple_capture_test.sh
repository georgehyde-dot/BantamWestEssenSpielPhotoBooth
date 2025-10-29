#!/bin/bash

# Simple Camera Capture Test
# Tests different approaches to capture a photo with gphoto2
# Helps diagnose PTP timeout and device busy issues

set -e

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}Simple Camera Capture Test${NC}"
echo "========================================"
echo ""

# Test counter
TEST_NUM=0
SUCCESS_COUNT=0
FAIL_COUNT=0

# Function to run a test
run_test() {
    local description="$1"
    local command="$2"
    TEST_NUM=$((TEST_NUM + 1))

    echo -e "${BLUE}Test $TEST_NUM: $description${NC}"
    echo "Command: $command"

    if eval "$command" 2>/tmp/gphoto_error.log; then
        echo -e "${GREEN}✓ SUCCESS${NC}"
        SUCCESS_COUNT=$((SUCCESS_COUNT + 1))

        # Check if file was created
        if [ -f "/tmp/test_capture_${TEST_NUM}.jpg" ]; then
            local size=$(du -h "/tmp/test_capture_${TEST_NUM}.jpg" | cut -f1)
            echo "  File size: $size"
            rm -f "/tmp/test_capture_${TEST_NUM}.jpg"
        fi
        return 0
    else
        echo -e "${RED}✗ FAILED${NC}"
        FAIL_COUNT=$((FAIL_COUNT + 1))

        # Show error
        if [ -s /tmp/gphoto_error.log ]; then
            echo "  Error:"
            head -n 3 /tmp/gphoto_error.log | sed 's/^/    /'
        fi
        return 1
    fi
}

# Kill any existing processes
cleanup_processes() {
    pkill -f gphoto2 2>/dev/null || true
    pkill -f "ffmpeg.*v4l2" 2>/dev/null || true
    pkill -f PTPCamera 2>/dev/null || true
    pkill -f gvfsd-gphoto2 2>/dev/null || true
    pkill -f gvfs-gphoto2 2>/dev/null || true
}

echo "Cleaning up any existing camera processes..."
cleanup_processes
sleep 1
echo -e "${GREEN}✓${NC} Processes cleaned"
echo ""

# Step 1: Camera Detection
echo -e "${BLUE}Camera Detection${NC}"
echo "----------------------------------------"
camera_output=$(gphoto2 --auto-detect 2>&1)

if echo "$camera_output" | grep -q "usb:"; then
    camera_model=$(echo "$camera_output" | grep "usb:" | head -n1 | awk -F'usb:' '{print $1}' | xargs)
    camera_port=$(echo "$camera_output" | grep "usb:" | head -n1 | awk '{print $NF}')
    echo -e "${GREEN}✓${NC} Camera detected: $camera_model"
    echo "  Port: $camera_port"
else
    echo -e "${RED}✗${NC} No camera detected"
    echo "Cannot proceed without camera"
    exit 1
fi
echo ""

# Step 2: Test Different Capture Methods
echo -e "${BLUE}Testing Capture Methods${NC}"
echo "----------------------------------------"

# Test 1: Basic capture with minimal delay
cleanup_processes
sleep 1
run_test "Basic capture (1s delay)" \
    "gphoto2 --capture-image-and-download --filename /tmp/test_capture_1.jpg --force-overwrite"
echo ""

# Test 2: Capture with longer delay
cleanup_processes
sleep 2
run_test "Capture with 2s delay" \
    "gphoto2 --capture-image-and-download --filename /tmp/test_capture_2.jpg --force-overwrite"
echo ""

# Test 3: Capture with explicit port
cleanup_processes
sleep 1
run_test "Capture with explicit port" \
    "gphoto2 --port '$camera_port' --capture-image-and-download --filename /tmp/test_capture_3.jpg --force-overwrite"
echo ""

# Test 4: Reset USB and capture
cleanup_processes
echo "Attempting USB reset..."
if gphoto2 --reset 2>/dev/null; then
    echo "  USB reset completed"
else
    echo "  USB reset not supported/failed"
fi
sleep 2
run_test "Capture after USB reset" \
    "gphoto2 --capture-image-and-download --filename /tmp/test_capture_4.jpg --force-overwrite"
echo ""

# Test 5: Kill gvfs and capture
cleanup_processes
echo "Stopping gvfs services..."
systemctl --user stop gvfs-daemon 2>/dev/null || true
systemctl --user stop gvfs-gphoto2-volume-monitor 2>/dev/null || true
sleep 2
run_test "Capture with gvfs stopped" \
    "gphoto2 --capture-image-and-download --filename /tmp/test_capture_5.jpg --force-overwrite"
echo ""

# Test 6: Capture to memory only (no download)
cleanup_processes
sleep 1
run_test "Capture to camera memory only" \
    "gphoto2 --capture-image"
echo ""

# Test 7: Wait and capture to SD card
cleanup_processes
sleep 1
run_test "Capture and keep on camera" \
    "gphoto2 --capture-image-and-download --keep --filename /tmp/test_capture_7.jpg --force-overwrite"
echo ""

# Test 8: Trigger capture only (no wait)
cleanup_processes
sleep 1
run_test "Trigger capture (no wait)" \
    "gphoto2 --trigger-capture"
echo ""

# Test 9: Set capture target to memory card first
cleanup_processes
sleep 1
echo "Setting capture target to memory card..."
gphoto2 --set-config capturetarget=1 2>/dev/null || echo "  Could not set capture target"
sleep 1
run_test "Capture with target set to card" \
    "gphoto2 --capture-image-and-download --filename /tmp/test_capture_9.jpg --force-overwrite"
echo ""

# Test 10: Long delay before capture
cleanup_processes
echo "Waiting 5 seconds before capture..."
sleep 5
run_test "Capture with 5s delay" \
    "gphoto2 --capture-image-and-download --filename /tmp/test_capture_10.jpg --force-overwrite"
echo ""

# Summary
echo ""
echo -e "${BLUE}=======================================${NC}"
echo -e "${BLUE}Test Summary${NC}"
echo -e "${BLUE}=======================================${NC}"
echo ""
echo "Total tests: $TEST_NUM"
echo -e "${GREEN}Successful: $SUCCESS_COUNT${NC}"
echo -e "${RED}Failed: $FAIL_COUNT${NC}"
echo ""

if [ $SUCCESS_COUNT -gt 0 ]; then
    echo -e "${GREEN}At least one capture method worked!${NC}"
    echo ""
    echo "Recommendations:"
    echo "- Use the approach that worked consistently"
    echo "- Add appropriate delays in your application"
    echo "- Consider stopping gvfs services on boot if needed"
else
    echo -e "${RED}All capture methods failed${NC}"
    echo ""
    echo "Troubleshooting steps:"
    echo "1. Check camera settings (not in Mass Storage mode)"
    echo "2. Try a different USB cable or port"
    echo "3. Update camera firmware if available"
    echo "4. Try: gphoto2 --debug --debug-logfile=debug.txt --capture-image-and-download"
fi

# Cleanup
cleanup_processes
rm -f /tmp/test_capture_*.jpg 2>/dev/null
rm -f /tmp/gphoto_error.log 2>/dev/null

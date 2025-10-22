#!/bin/bash

# Canon EOS 250D Diagnostic Script
# Diagnoses PTP Device Busy issues specific to Canon EOS 250D
# Tests various camera configurations and modes

set -e

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}Canon EOS 250D Diagnostic${NC}"
echo "========================================"
echo ""
echo "This script will diagnose why your Canon EOS 250D"
echo "is showing 'PTP Device Busy' errors."
echo ""

# Step 1: Basic Detection
echo -e "${BLUE}Step 1: Camera Detection${NC}"
echo "----------------------------------------"

camera_output=$(gphoto2 --auto-detect 2>&1)
if echo "$camera_output" | grep -q "Canon EOS 250D"; then
    camera_port=$(echo "$camera_output" | grep "Canon EOS 250D" | awk '{print $NF}')
    echo -e "${GREEN}✓${NC} Canon EOS 250D detected"
    echo "  Port: $camera_port"
else
    echo -e "${RED}✗${NC} Canon EOS 250D not detected"
    exit 1
fi
echo ""

# Step 2: Check Camera Abilities
echo -e "${BLUE}Step 2: Camera Abilities${NC}"
echo "----------------------------------------"
echo "Checking what the camera reports it can do..."

if gphoto2 --abilities 2>/tmp/abilities.err | head -20; then
    echo -e "${GREEN}✓${NC} Abilities retrieved"
else
    echo -e "${RED}✗${NC} Could not get abilities"
    cat /tmp/abilities.err | head -5
fi
echo ""

# Step 3: Check Current Camera Configuration
echo -e "${BLUE}Step 3: Camera Configuration${NC}"
echo "----------------------------------------"
echo "Getting current camera settings..."

# Try to list configuration
echo "Attempting to list configuration..."
if gphoto2 --list-config 2>/tmp/config_err.txt | head -20; then
    echo -e "${GREEN}✓${NC} Configuration accessible"
    echo ""

    # Check specific important settings
    echo "Checking important settings:"

    # Check capture target
    echo -n "  Capture target: "
    gphoto2 --get-config capturetarget 2>/dev/null | grep "Current:" | sed 's/Current: //' || echo "unknown"

    # Check image format
    echo -n "  Image format: "
    gphoto2 --get-config imageformat 2>/dev/null | grep "Current:" | sed 's/Current: //' || echo "unknown"

    # Check drive mode
    echo -n "  Drive mode: "
    gphoto2 --get-config drivemode 2>/dev/null | grep "Current:" | sed 's/Current: //' || echo "unknown"

    # Check auto power off
    echo -n "  Auto power off: "
    gphoto2 --get-config autopoweroff 2>/dev/null | grep "Current:" | sed 's/Current: //' || echo "unknown"

else
    echo -e "${RED}✗${NC} Cannot access configuration"
    echo "  This might indicate the camera is locked"
    echo "  Error:"
    head -3 /tmp/config_err.txt | sed 's/^/    /'
fi
echo ""

# Step 4: Try Different Capture Targets
echo -e "${BLUE}Step 4: Testing Capture Targets${NC}"
echo "----------------------------------------"

# Kill any processes first
pkill -9 -f gphoto2 2>/dev/null || true
sleep 1

# Try setting to internal RAM
echo "Setting capture target to internal RAM..."
if gphoto2 --set-config capturetarget=0 2>/dev/null; then
    echo -e "${GREEN}✓${NC} Set to internal RAM"
    echo "  Testing capture to RAM..."
    if gphoto2 --capture-image 2>/tmp/ram_capture.err; then
        echo -e "${GREEN}✓${NC} Capture to RAM works!"
    else
        echo -e "${RED}✗${NC} Capture to RAM failed"
        grep "Error" /tmp/ram_capture.err | head -2 | sed 's/^/    /'
    fi
else
    echo -e "${YELLOW}⚠${NC} Could not set capture target to RAM"
fi
echo ""

# Try setting to memory card
echo "Setting capture target to memory card..."
if gphoto2 --set-config capturetarget=1 2>/dev/null; then
    echo -e "${GREEN}✓${NC} Set to memory card"
    echo "  Testing capture to card..."
    if gphoto2 --capture-image 2>/tmp/card_capture.err; then
        echo -e "${GREEN}✓${NC} Capture to card works!"
    else
        echo -e "${RED}✗${NC} Capture to card failed"
        grep "Error" /tmp/card_capture.err | head -2 | sed 's/^/    /'
    fi
else
    echo -e "${YELLOW}⚠${NC} Could not set capture target to card"
fi
echo ""

# Step 5: Test Preview Mode
echo -e "${BLUE}Step 5: Testing Preview/Live View${NC}"
echo "----------------------------------------"

# Kill any processes
pkill -9 -f gphoto2 2>/dev/null || true
sleep 1

echo "Testing if camera can enter preview mode..."
timeout 3 gphoto2 --capture-preview 2>/tmp/preview.err
if [ $? -eq 124 ]; then
    echo -e "${GREEN}✓${NC} Preview mode works (timed out as expected)"
elif [ $? -eq 0 ]; then
    echo -e "${GREEN}✓${NC} Preview capture successful"
else
    echo -e "${RED}✗${NC} Preview mode failed"
    head -3 /tmp/preview.err | sed 's/^/    /'
fi
echo ""

# Step 6: Check Camera Mode
echo -e "${BLUE}Step 6: Camera Mode Check${NC}"
echo "----------------------------------------"
echo "IMPORTANT: Check your camera's physical mode dial!"
echo ""
echo "The Canon EOS 250D should be set to:"
echo "  • M (Manual) mode"
echo "  • Av (Aperture Priority) mode"
echo "  • Tv (Shutter Priority) mode"
echo "  • P (Program) mode"
echo ""
echo "It should NOT be in:"
echo "  • AUTO mode"
echo "  • Scene modes"
echo "  • Video mode"
echo ""
read -p "Is your camera in M, Av, Tv, or P mode? (y/n): " -n 1 -r
echo ""
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo -e "${RED}✗${NC} Please set camera to M, Av, Tv, or P mode and try again"
    exit 1
fi
echo ""

# Step 7: Test with wait-event
echo -e "${BLUE}Step 7: Testing with wait-event${NC}"
echo "----------------------------------------"
echo "Testing if camera responds to wait-event..."

# Kill processes
pkill -9 -f gphoto2 2>/dev/null || true
sleep 1

if timeout 2 gphoto2 --wait-event=1s 2>&1 | grep -q "UNKNOWN"; then
    echo -e "${GREEN}✓${NC} Camera is responding to events"
else
    echo -e "${YELLOW}⚠${NC} Camera may not be sending events"
fi
echo ""

# Step 8: Try trigger capture
echo -e "${BLUE}Step 8: Testing Trigger Capture${NC}"
echo "----------------------------------------"
echo "Testing trigger-only capture (no download)..."

# Kill processes
pkill -9 -f gphoto2 2>/dev/null || true
sleep 1

if gphoto2 --trigger-capture 2>/tmp/trigger.err; then
    echo -e "${GREEN}✓${NC} Trigger capture works!"
    echo "  The camera took a photo but didn't download it"
else
    echo -e "${RED}✗${NC} Trigger capture failed"
    head -3 /tmp/trigger.err | sed 's/^/    /'
fi
echo ""

# Step 9: Summary and Recommendations
echo -e "${BLUE}======================================${NC}"
echo -e "${BLUE}Diagnosis Summary${NC}"
echo -e "${BLUE}======================================${NC}"
echo ""

echo "Based on the tests above:"
echo ""

# Check if any capture method worked
if grep -q "✓.*works" /dev/stdout 2>/dev/null; then
    echo -e "${GREEN}Some capture methods are working!${NC}"
    echo ""
    echo "Recommendations:"
    echo "1. Use the capture method that worked"
    echo "2. If only trigger-capture works, use that"
    echo "3. If preview works, try capture-movie mode"
else
    echo -e "${RED}No capture methods are working${NC}"
    echo ""
    echo "Try these fixes:"
    echo ""
    echo "1. CAMERA SETTINGS:"
    echo "   - Turn off 'Auto Power Off' in camera menu"
    echo "   - Turn off 'Wi-Fi/Bluetooth' in camera menu"
    echo "   - Set 'USB Connection' to 'PC Remote' (not Mass Storage)"
    echo "   - Format the memory card in camera"
    echo ""
    echo "2. PHYSICAL CHECKS:"
    echo "   - Use a USB 2.0 port (not 3.0)"
    echo "   - Try a different/shorter USB cable"
    echo "   - Remove and reinsert battery"
    echo "   - Let camera sit powered off for 30 seconds"
    echo ""
    echo "3. SOFTWARE WORKAROUND:"
    echo "   - Use capture-preview in a loop instead of capture-image"
    echo "   - Use trigger-capture if shutter fires but no download"
fi

echo ""
echo "For more details, run:"
echo "  gphoto2 --debug --debug-logfile=debug.log --capture-image-and-download"
echo ""
echo -e "${GREEN}Diagnostic complete!${NC}"

# Cleanup
rm -f /tmp/*.err 2>/dev/null || true

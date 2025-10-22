#!/bin/bash

# Canon EOS 250D Settings Check and Configuration
# Checks all accessible settings via gphoto2 and provides instructions for physical settings

set -e

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}Canon EOS 250D Settings Check${NC}"
echo "========================================"
echo ""

# Kill any existing processes first
pkill -f gphoto2 2>/dev/null || true
sleep 1

# Verify camera is connected
echo -e "${BLUE}Camera Detection${NC}"
echo "----------------------------------------"
if gphoto2 --auto-detect | grep -q "Canon EOS 250D"; then
    echo -e "${GREEN}✓${NC} Canon EOS 250D detected"
else
    echo -e "${RED}✗${NC} Canon EOS 250D not detected"
    exit 1
fi
echo ""

# Function to get a config value
get_config() {
    local config="$1"
    local value=$(gphoto2 --get-config "$config" 2>/dev/null | grep "Current:" | sed 's/Current: //')
    echo "$value"
}

# Function to set a config value
set_config() {
    local config="$1"
    local value="$2"
    local description="$3"

    echo -n "  Setting $description to $value... "
    if gphoto2 --set-config "$config=$value" 2>/dev/null; then
        echo -e "${GREEN}✓${NC}"
        return 0
    else
        echo -e "${RED}✗${NC} (may need physical change)"
        return 1
    fi
}

# Check and display all important settings
echo -e "${BLUE}Current Camera Settings${NC}"
echo "----------------------------------------"

echo "CAPTURE SETTINGS:"
echo -n "  Capture target: "
capturetarget=$(get_config "capturetarget")
echo "${capturetarget:-unknown}"

echo -n "  Image format: "
imageformat=$(get_config "imageformat")
echo "${imageformat:-unknown}"

echo -n "  Image format SD: "
imageformatsd=$(get_config "imageformatsd")
echo "${imageformatsd:-unknown}"

echo -n "  Drive mode: "
drivemode=$(get_config "drivemode")
echo "${drivemode:-unknown}"

echo ""
echo "POWER SETTINGS:"
echo -n "  Auto power off: "
autopoweroff=$(get_config "autopoweroff")
echo "${autopoweroff:-unknown} seconds"

echo -n "  Battery level: "
batterylevel=$(get_config "batterylevel")
echo "${batterylevel:-unknown}"

echo ""
echo "FOCUS SETTINGS:"
echo -n "  Focus mode: "
focusmode=$(get_config "focusmode")
echo "${focusmode:-unknown}"

echo -n "  AF method: "
afmethod=$(get_config "afmethod")
echo "${afmethod:-unknown}"

echo ""
echo "EVF/DISPLAY SETTINGS:"
echo -n "  EVF mode: "
evfmode=$(get_config "evfmode")
echo "${evfmode:-unknown}"

echo -n "  Viewfinder: "
viewfinder=$(get_config "viewfinder")
echo "${viewfinder:-unknown}"

echo ""
echo "CONNECTIVITY:"
echo -n "  PTP mode: "
ptpmode=$(get_config "ptpmode")
echo "${ptpmode:-unknown}"

echo ""

# Try to fix problematic settings
echo -e "${BLUE}Attempting to Fix Settings${NC}"
echo "----------------------------------------"

# Disable auto power off
if [[ "$autopoweroff" != "0" ]]; then
    set_config "autopoweroff" "0" "Auto power off"
fi

# Set capture target to internal RAM
if [[ "$capturetarget" != "Internal RAM" ]]; then
    set_config "capturetarget" "0" "Capture target"
fi

# Enable viewfinder if disabled
if [[ "$viewfinder" == "0" ]]; then
    set_config "viewfinder" "1" "Viewfinder"
fi

echo ""

# Physical camera settings checklist
echo -e "${BLUE}======================================${NC}"
echo -e "${BLUE}PHYSICAL CAMERA SETTINGS CHECKLIST${NC}"
echo -e "${BLUE}======================================${NC}"
echo ""
echo "Please ask your coworker to check these settings on the camera:"
echo ""
echo -e "${YELLOW}1. MODE DIAL:${NC}"
echo "   [ ] Set to M (Manual), Av, Tv, or P mode"
echo "   [ ] NOT in AUTO, Scene, or Video mode"
echo ""
echo -e "${YELLOW}2. CAMERA MENU - WRENCH ICON 1:${NC}"
echo "   [ ] Auto power off: DISABLE or set to maximum"
echo "   [ ] Wi-Fi: DISABLE"
echo "   [ ] Bluetooth: DISABLE"
echo ""
echo -e "${YELLOW}3. CAMERA MENU - WRENCH ICON 2:${NC}"
echo "   [ ] Communication settings > USB connection: PC Remote (NOT Mass Storage)"
echo ""
echo -e "${YELLOW}4. SHOOTING SETTINGS:${NC}"
echo "   [ ] Drive mode: Single shooting (not continuous)"
echo "   [ ] Image quality: JPEG (not RAW or RAW+JPEG for faster transfer)"
echo "   [ ] Auto review: 2 sec or OFF"
echo ""
echo -e "${YELLOW}5. PHYSICAL SWITCHES:${NC}"
echo "   [ ] Lens: Set to AF (autofocus)"
echo "   [ ] Mode switch: Not in video mode"
echo "   [ ] Live View switch: Try both positions"
echo ""
echo -e "${YELLOW}6. MEMORY CARD:${NC}"
echo "   [ ] Card is inserted and not full"
echo "   [ ] Card write-protect switch is OFF"
echo "   [ ] Format card in camera if possible"
echo ""

# Test with new settings
echo -e "${BLUE}Quick Capture Test${NC}"
echo "----------------------------------------"
echo "Testing capture with current settings..."

# Clean processes
pkill -f gphoto2 2>/dev/null || true
sleep 2

# Try a simple capture
if gphoto2 --capture-image 2>/tmp/test.err; then
    echo -e "${GREEN}✓${NC} Capture successful!"
else
    echo -e "${RED}✗${NC} Capture still failing"
    echo ""
    echo "Error details:"
    grep -E "Error|error|failed" /tmp/test.err | head -3 | sed 's/^/  /'
fi

echo ""
echo -e "${BLUE}Additional Troubleshooting${NC}"
echo "----------------------------------------"
echo ""
echo "If capture still fails after checking all settings:"
echo ""
echo "1. POWER CYCLE:"
echo "   - Turn camera OFF"
echo "   - Remove battery for 10 seconds"
echo "   - Reinsert battery and turn ON"
echo ""
echo "2. RESET CAMERA:"
echo "   - Menu > Wrench 5 > Clear all camera settings"
echo "   - Then reconfigure settings above"
echo ""
echo "3. USB CONNECTION:"
echo "   - Try a different USB cable (USB 2.0 preferred)"
echo "   - Try a different USB port"
echo "   - Connect directly to Pi (no hubs)"
echo ""
echo "4. FIRMWARE:"
echo "   - Check Canon website for firmware updates"
echo "   - Current firmware can be seen in Menu > Wrench 4 > Firmware"
echo ""
echo "5. ALTERNATIVE CAPTURE METHODS:"
echo "   If normal capture fails, we can try:"
echo "   - Live view capture (capture-preview repeatedly)"
echo "   - Tethered shooting mode"
echo "   - Remote control mode"
echo ""

# Get debug info for support
echo -e "${BLUE}Debug Information${NC}"
echo "----------------------------------------"
echo "If you need to report this issue, run:"
echo ""
echo "  gphoto2 --debug --debug-logfile=canon_debug.log --capture-image-and-download"
echo ""
echo "And share the canon_debug.log file"
echo ""

echo -e "${GREEN}Settings check complete!${NC}"

# Cleanup
rm -f /tmp/test.err 2>/dev/null || true

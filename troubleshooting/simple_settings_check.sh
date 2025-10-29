#!/bin/bash

# Simple Canon EOS 250D Settings Check
# Checks camera settings without terminal issues

set -e

# No color codes to avoid terminal issues
echo "Canon EOS 250D Settings Check"
echo "========================================"
echo ""

# Kill any existing processes first
pkill -f gphoto2 2>/dev/null || true
sleep 1

# Verify camera is connected
echo "Camera Detection:"
if gphoto2 --auto-detect 2>/dev/null | grep -q "Canon EOS 250D"; then
    echo "  [OK] Canon EOS 250D detected"
else
    echo "  [ERROR] Canon EOS 250D not detected"
    exit 1
fi
echo ""

# Simple function to get config values
check_setting() {
    local config="$1"
    local name="$2"

    echo -n "  $name: "

    # Redirect stderr to avoid escape sequences
    result=$(gphoto2 --get-config "$config" 2>/dev/null | grep "Current:" | sed 's/Current: //' || echo "unknown")

    # Clean any escape sequences that might have gotten through
    result=$(echo "$result" | sed 's/\x1b\[[0-9;]*m//g' | sed 's/\x1b\[[0-9;]*[a-zA-Z]//g' | tr -d '\033' | tr -d '\177')

    echo "$result"
}

echo "CHECKING CAMERA SETTINGS:"
echo "--------------------------"
echo ""

echo "Critical Settings:"
check_setting "capturetarget" "Capture target"
check_setting "autopoweroff" "Auto power off"
check_setting "imageformat" "Image format"
check_setting "drivemode" "Drive mode"

echo ""
echo "Focus Settings:"
check_setting "focusmode" "Focus mode"
check_setting "afmethod" "AF method"

echo ""
echo "Display Settings:"
check_setting "viewfinder" "Viewfinder"
check_setting "evfmode" "EVF mode"

echo ""
echo "Battery Status:"
check_setting "batterylevel" "Battery level"

echo ""
echo "ATTEMPTING TO CHANGE SETTINGS:"
echo "-------------------------------"
echo ""

# Try to disable auto power off
echo -n "  Disabling auto power off... "
if gphoto2 --set-config autopoweroff=0 2>/dev/null; then
    echo "[OK]"
else
    echo "[FAILED - May need physical change]"
fi

# Try to set capture target to internal RAM
echo -n "  Setting capture to Internal RAM... "
if gphoto2 --set-config capturetarget=0 2>/dev/null; then
    echo "[OK]"
else
    echo "[FAILED - May need physical change]"
fi

echo ""
echo "TESTING CAPTURE:"
echo "----------------"
echo ""

# Kill processes and wait
pkill -f gphoto2 2>/dev/null || true
sleep 2

# Test basic capture
echo -n "  Testing capture to internal RAM... "
if gphoto2 --capture-image 2>/dev/null; then
    echo "[SUCCESS]"
else
    echo "[FAILED]"

    # Try trigger only
    echo -n "  Testing trigger capture... "
    if gphoto2 --trigger-capture 2>/dev/null; then
        echo "[SUCCESS - Shutter fires]"
    else
        echo "[FAILED]"
    fi
fi

echo ""
echo "========================================"
echo "PHYSICAL CAMERA SETTINGS TO CHECK:"
echo "========================================"
echo ""
echo "Please have your coworker check these on the camera:"
echo ""
echo "1. TOP DIAL:"
echo "   - Set to M, Av, Tv, or P (NOT Auto or Scene modes)"
echo ""
echo "2. MENU -> WRENCH ICON 1:"
echo "   [ ] Auto power off: DISABLE"
echo "   [ ] Wi-Fi: DISABLE"
echo "   [ ] Bluetooth: DISABLE"
echo ""
echo "3. MENU -> WRENCH ICON 2:"
echo "   [ ] Communication settings:"
echo "       -> USB connection: PC Remote (NOT Mass Storage)"
echo "       ^^ THIS IS CRITICAL FOR CANON 250D ^^"
echo ""
echo "4. BASIC SETTINGS:"
echo "   [ ] Memory card inserted and not full"
echo "   [ ] Battery charged"
echo "   [ ] Lens set to AF"
echo ""
echo "5. IF STILL FAILING:"
echo "   - Turn camera OFF"
echo "   - Remove battery for 10 seconds"
echo "   - Reinsert battery"
echo "   - Turn camera ON"
echo "   - Try a different USB cable"
echo ""
echo "========================================"
echo ""

# Which settings can be changed via gphoto2
echo "SETTINGS CHANGEABLE VIA GPHOTO2:"
echo "---------------------------------"
echo ""
echo "YES - Can change remotely:"
echo "  - capturetarget (RAM vs Card)"
echo "  - imageformat (JPEG quality)"
echo "  - autopoweroff (disable/enable)"
echo "  - datetime (sync time)"
echo "  - owner name"
echo "  - artist name"
echo ""
echo "NO - Must change on camera:"
echo "  - USB Connection Mode (PC Remote vs Mass Storage)"
echo "  - Wi-Fi/Bluetooth settings"
echo "  - Shooting mode (M/Av/Tv/P)"
echo "  - Focus mode (if in manual)"
echo "  - Card format"
echo ""
echo "Settings check complete!"

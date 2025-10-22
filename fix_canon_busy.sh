#!/bin/bash

# Fix Canon PTP Device Busy Issues
# Aggressively clears all processes and services that might be holding the camera
# Specifically for Canon EOS cameras with PTP timeout/busy errors

set -e

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}Canon PTP Device Busy Fix${NC}"
echo "========================================"
echo ""

# Check if running as root for some operations
if [ "$EUID" -eq 0 ]; then
   SUDO=""
else
   SUDO="sudo"
fi

# Step 1: Kill ALL camera-related processes
echo -e "${BLUE}Step 1: Killing all camera-related processes${NC}"
echo "----------------------------------------"

# Kill gphoto2 processes
pkill -9 -f gphoto2 2>/dev/null && echo "  Killed gphoto2 processes" || true
pkill -9 -f gphoto 2>/dev/null || true

# Kill PTP processes
pkill -9 -f PTPCamera 2>/dev/null && echo "  Killed PTPCamera" || true
pkill -9 -f ptpcam 2>/dev/null || true

# Kill gvfs processes that handle cameras
pkill -9 -f gvfsd-gphoto2 2>/dev/null && echo "  Killed gvfsd-gphoto2" || true
pkill -9 -f gvfs-gphoto2 2>/dev/null && echo "  Killed gvfs-gphoto2" || true
pkill -9 -f gvfsd-mtp 2>/dev/null && echo "  Killed gvfsd-mtp" || true

# Kill any ffmpeg processes using v4l2
pkill -9 -f "ffmpeg.*v4l2" 2>/dev/null && echo "  Killed ffmpeg v4l2 processes" || true

# Kill colord which sometimes locks cameras
pkill -9 colord 2>/dev/null && echo "  Killed colord" || true

echo -e "${GREEN}✓${NC} Processes killed"
echo ""

# Step 2: Unmount any camera mounts
echo -e "${BLUE}Step 2: Unmounting camera filesystems${NC}"
echo "----------------------------------------"

# Find and unmount gvfs mounts
for mount in /run/user/*/gvfs/*; do
    if [ -d "$mount" ]; then
        echo "  Unmounting: $mount"
        fusermount -u "$mount" 2>/dev/null || $SUDO umount -f "$mount" 2>/dev/null || true
    fi
done

# Also check media mounts
for mount in /media/*/CANON*; do
    if [ -d "$mount" ]; then
        echo "  Unmounting: $mount"
        $SUDO umount -f "$mount" 2>/dev/null || true
    fi
done

echo -e "${GREEN}✓${NC} Unmounts complete"
echo ""

# Step 3: Stop interfering services
echo -e "${BLUE}Step 3: Stopping interfering services${NC}"
echo "----------------------------------------"

# Stop gvfs services
systemctl --user stop gvfs-daemon.service 2>/dev/null && echo "  Stopped gvfs-daemon" || true
systemctl --user stop gvfs-gphoto2-volume-monitor.service 2>/dev/null && echo "  Stopped gphoto2 monitor" || true
systemctl --user stop gvfs-mtp-volume-monitor.service 2>/dev/null && echo "  Stopped MTP monitor" || true

# Mask them temporarily
systemctl --user mask gvfs-daemon.service 2>/dev/null || true
systemctl --user mask gvfs-gphoto2-volume-monitor.service 2>/dev/null || true

echo -e "${GREEN}✓${NC} Services stopped"
echo ""

# Step 4: Reset USB for Canon camera
echo -e "${BLUE}Step 4: USB Reset${NC}"
echo "----------------------------------------"

# Find Canon USB device
canon_device=$(lsusb | grep -i "canon" | head -n1)
if [ -n "$canon_device" ]; then
    echo "  Found: $canon_device"

    # Extract bus and device numbers
    bus=$(echo "$canon_device" | sed 's/Bus \([0-9]*\).*/\1/')
    device=$(echo "$canon_device" | sed 's/.*Device \([0-9]*\).*/\1/')

    # Try to reset using gphoto2
    echo "  Attempting gphoto2 USB reset..."
    if gphoto2 --reset 2>/dev/null; then
        echo -e "${GREEN}✓${NC} USB reset via gphoto2 succeeded"
    else
        echo -e "${YELLOW}⚠${NC} gphoto2 reset not available"
    fi

    # Try usbreset if available
    if command -v usbreset >/dev/null 2>&1; then
        echo "  Attempting usbreset..."
        $SUDO usbreset "/dev/bus/usb/$bus/$device" 2>/dev/null && echo -e "${GREEN}✓${NC} USB reset succeeded" || true
    fi
else
    echo -e "${YELLOW}⚠${NC} No Canon device found via lsusb"
fi
echo ""

# Step 5: Wait for camera to stabilize
echo -e "${BLUE}Step 5: Waiting for camera to stabilize${NC}"
echo "----------------------------------------"
echo "  Waiting 3 seconds..."
sleep 3
echo -e "${GREEN}✓${NC} Wait complete"
echo ""

# Step 6: Verify camera is available
echo -e "${BLUE}Step 6: Camera Detection${NC}"
echo "----------------------------------------"

camera_output=$(gphoto2 --auto-detect 2>&1)
if echo "$camera_output" | grep -q "usb:"; then
    camera_model=$(echo "$camera_output" | grep "usb:" | head -n1 | awk -F'usb:' '{print $1}' | xargs)
    camera_port=$(echo "$camera_output" | grep "usb:" | head -n1 | awk '{print $NF}')
    echo -e "${GREEN}✓${NC} Camera detected: $camera_model"
    echo "  Port: $camera_port"
else
    echo -e "${RED}✗${NC} No camera detected after reset"
    echo ""
    echo "Try:"
    echo "1. Unplug and replug the USB cable"
    echo "2. Turn the camera off and on"
    echo "3. Check camera is not in Mass Storage mode"
    exit 1
fi
echo ""

# Step 7: Test capture
echo -e "${BLUE}Step 7: Test Capture${NC}"
echo "----------------------------------------"

echo "Attempting test capture..."
if gphoto2 --capture-image-and-download --filename /tmp/test_fix.jpg --force-overwrite 2>/tmp/capture_error.log; then
    echo -e "${GREEN}✓${NC} Capture SUCCESSFUL!"
    if [ -f /tmp/test_fix.jpg ]; then
        size=$(du -h /tmp/test_fix.jpg | cut -f1)
        echo "  File size: $size"
        rm -f /tmp/test_fix.jpg
    fi
else
    echo -e "${RED}✗${NC} Capture failed"
    echo "  Error:"
    head -n 5 /tmp/capture_error.log | sed 's/^/    /'
    echo ""
    echo "Additional fixes to try:"
    echo "1. Power cycle the camera"
    echo "2. Remove and reinsert camera battery"
    echo "3. Check camera firmware updates"
    echo "4. Try a different USB cable/port"
fi
echo ""

# Step 8: Re-enable services (optional)
echo -e "${BLUE}Step 8: Service Management${NC}"
echo "----------------------------------------"
echo "Services have been masked to prevent interference."
echo ""
echo "To re-enable them later (if needed):"
echo "  systemctl --user unmask gvfs-daemon.service"
echo "  systemctl --user unmask gvfs-gphoto2-volume-monitor.service"
echo ""
echo "Or keep them disabled for photo booth operation."
echo ""

echo -e "${GREEN}Fix process complete!${NC}"
echo ""
echo "If capture now works, the issue was likely:"
echo "- gvfs/GVFS interference"
echo "- Stale process holding camera"
echo "- USB connection needed reset"
echo ""
echo "For permanent fix, add to startup:"
echo "  systemctl --user disable gvfs-daemon.service"
echo "  systemctl --user disable gvfs-gphoto2-volume-monitor.service"

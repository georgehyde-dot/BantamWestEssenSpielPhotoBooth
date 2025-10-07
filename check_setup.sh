#!/bin/bash

# Photo Booth System Diagnostic Script
# This script checks the status of all required components for the photo booth

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
echo -e "${BLUE}Photo Booth System Diagnostics${NC}"
echo -e "${BLUE}$(date)${NC}"

# System Information
print_header "System Information"
print_info "Hostname: $(hostname)"
print_info "OS: $(lsb_release -d 2>/dev/null | cut -f2 || cat /etc/os-release | grep PRETTY_NAME | cut -d'"' -f2)"
print_info "Kernel: $(uname -r)"
print_info "Architecture: $(uname -m)"
print_info "User: $(whoami)"
print_info "Groups: $(groups)"

# V4L2 Loopback Check
print_header "V4L2 Loopback Device Status"

# Check if v4l2loopback module is loaded
if lsmod | grep -q v4l2loopback; then
    print_success "v4l2loopback module is loaded"

    # Get module info
    echo -e "\n  Module parameters:"
    cat /sys/module/v4l2loopback/parameters/* 2>/dev/null | while read param; do
        print_info "$param"
    done

    # List video devices
    echo -e "\n  Video devices:"
    for device in /dev/video*; do
        if [ -e "$device" ]; then
            # Try to get device name
            device_name=$(v4l2-ctl --device=$device --info 2>/dev/null | grep "Card type" | cut -d: -f2 | xargs || echo "Unknown")
            print_info "$device: $device_name"
        fi
    done
else
    print_error "v4l2loopback module is NOT loaded"
    print_info "Run: sudo modprobe v4l2loopback exclusive_caps=1 max_buffers=2"
fi

# Check v4l2loopback persistence configuration
if [ -f /etc/modules-load.d/v4l2loopback.conf ]; then
    print_success "v4l2loopback is configured to load at boot"
else
    print_warning "v4l2loopback is NOT configured to load at boot"
fi

# GPhoto2 Camera Check
print_header "Camera Detection (GPhoto2)"

# Check if gphoto2 is installed
if command -v gphoto2 >/dev/null 2>&1; then
    print_success "gphoto2 is installed"
    print_info "Version: $(gphoto2 --version | head -n1)"

    # Detect cameras
    echo -e "\n  Detecting cameras..."
    camera_output=$(gphoto2 --auto-detect 2>&1)

    if echo "$camera_output" | grep -q "usb:"; then
        print_success "Camera detected via USB"
        echo "$camera_output" | grep "usb:" | while read line; do
            print_info "$line"
        done

        # Try to get camera summary
        echo -e "\n  Camera information:"
        gphoto2 --summary 2>/dev/null | head -n 20 | while read line; do
            [ ! -z "$line" ] && print_info "$line"
        done

        # Check camera abilities
        echo -e "\n  Camera abilities:"
        abilities=$(gphoto2 --abilities 2>/dev/null | grep -E "capture_image|capture_preview" | head -n 5)
        if [ ! -z "$abilities" ]; then
            echo "$abilities" | while read line; do
                print_info "$line"
            done
        fi
    else
        print_error "No camera detected via USB"
        print_info "Make sure camera is:"
        print_info "  - Connected via USB"
        print_info "  - Powered on"
        print_info "  - Not being used by another application"
    fi
else
    print_error "gphoto2 is NOT installed"
    print_info "Install with: sudo apt-get install gphoto2"
fi

# Check for interfering processes
echo -e "\n  Checking for interfering processes:"
if pgrep -f gphoto2 >/dev/null 2>&1; then
    print_warning "gphoto2 processes are running:"
    ps aux | grep -v grep | grep gphoto2 | while read line; do
        print_info "$line"
    done
else
    print_info "No gphoto2 processes running"
fi

# USB Device Check
print_header "USB Devices"

# List all USB devices
echo "  All USB devices:"
lsusb | while read line; do
    print_info "$line"
done

# Look specifically for Canon devices
echo -e "\n  Canon devices:"
canon_devices=$(lsusb | grep -i canon)
if [ ! -z "$canon_devices" ]; then
    echo "$canon_devices" | while read line; do
        print_success "$line"
    done
else
    print_warning "No Canon USB devices found"
fi

# Look specifically for Epson devices
echo -e "\n  Epson devices:"
epson_devices=$(lsusb | grep -i epson)
if [ ! -z "$epson_devices" ]; then
    echo "$epson_devices" | while read line; do
        print_success "$line"
    done
else
    print_warning "No Epson USB devices found"
fi

# CUPS Printer Check
print_header "Printer Configuration (CUPS)"

# Check if CUPS is installed and running
if systemctl is-active --quiet cups; then
    print_success "CUPS service is running"

    # Get CUPS version
    if command -v cups-config >/dev/null 2>&1; then
        print_info "Version: $(cups-config --version)"
    fi

    # List all configured printers
    echo -e "\n  Configured printers:"
    printer_list=$(lpstat -p 2>/dev/null)
    if [ ! -z "$printer_list" ]; then
        echo "$printer_list" | while read line; do
            print_info "$line"
        done

        # Get default printer
        default_printer=$(lpstat -d 2>/dev/null | cut -d: -f2 | xargs)
        if [ ! -z "$default_printer" ]; then
            print_success "Default printer: $default_printer"
        else
            print_warning "No default printer set"
        fi

        # Check for TurboPrint printers
        echo -e "\n  TurboPrint printers:"
        for printer in $(lpstat -p 2>/dev/null | awk '{print $2}'); do
            if grep -qi turboprint /etc/cups/ppd/${printer}.ppd 2>/dev/null; then
                print_success "$printer uses TurboPrint driver"
                # Get driver details
                grep -E "NickName|ModelName" /etc/cups/ppd/${printer}.ppd 2>/dev/null | while read line; do
                    print_info "$line"
                done
            fi
        done
    else
        print_warning "No printers configured in CUPS"
    fi

    # Check printer queue
    echo -e "\n  Print queue status:"
    queue_status=$(lpstat -o 2>/dev/null)
    if [ ! -z "$queue_status" ]; then
        print_warning "Jobs in queue:"
        echo "$queue_status" | while read line; do
            print_info "$line"
        done
    else
        print_info "Print queue is empty"
    fi
else
    print_error "CUPS service is NOT running"
    print_info "Start with: sudo systemctl start cups"
fi

# Check TurboPrint installation
print_header "TurboPrint Status"

if command -v tpconfig >/dev/null 2>&1; then
    print_success "TurboPrint is installed"
    print_info "Version: $(tpconfig --version 2>/dev/null | head -n1)"

    # Check TurboPrint status
    if command -v tpstatus >/dev/null 2>&1; then
        echo -e "\n  TurboPrint printer status:"
        tp_status=$(sudo tpstatus 2>/dev/null | head -n 20)
        if [ ! -z "$tp_status" ]; then
            echo "$tp_status" | while read line; do
                print_info "$line"
            done
        fi
    fi
else
    print_warning "TurboPrint is NOT installed"
    print_info "Download from: https://www.turboprint.de/downloads/turboprint-2.55-1.arm64.tgz"
fi

# Check for USB printer connections
print_header "USB Printer Connections"

# Use lpinfo to detect USB printers
echo "  Detected USB printers:"
usb_printers=$(sudo lpinfo -v 2>/dev/null | grep "usb://")
if [ ! -z "$usb_printers" ]; then
    echo "$usb_printers" | while read line; do
        print_success "$line"
    done
else
    print_warning "No USB printers detected by CUPS"
fi

# Application-specific checks
print_header "Application Environment"

# Check for database file
DB_PATH="/usr/local/share/photo_booth/photo_booth.db"
if [ -f "$DB_PATH" ]; then
    print_success "Database file exists: $DB_PATH"
    print_info "Permissions: $(ls -la $DB_PATH | awk '{print $1, $3, $4}')"
    print_info "Size: $(du -h $DB_PATH | cut -f1)"
else
    print_warning "Database file does NOT exist: $DB_PATH"
fi

# Check for static assets directory
ASSETS_DIR="/usr/local/share/photo_booth/static"
if [ -d "$ASSETS_DIR" ]; then
    print_success "Static assets directory exists"
    print_info "Contents: $(ls -1 $ASSETS_DIR 2>/dev/null | wc -l) items"

    # Check for background image
    if [ -f "$ASSETS_DIR/background.png" ]; then
        print_success "Template background image exists"
    else
        print_warning "Template background image missing"
    fi

    # Check for selection images
    if [ -d "$ASSETS_DIR/resized_output" ]; then
        image_count=$(ls -1 $ASSETS_DIR/resized_output/*.jpg 2>/dev/null | wc -l)
        if [ "$image_count" -eq 12 ]; then
            print_success "All 12 selection images present"
        else
            print_warning "Selection images: $image_count/12 found"
        fi
    else
        print_warning "Selection images directory missing"
    fi
else
    print_warning "Static assets directory does NOT exist"
fi

# Check environment file
ENV_FILE="$HOME/.photobooth.env"
if [ -f "$ENV_FILE" ]; then
    print_success "Environment file exists: $ENV_FILE"
    print_info "Key variables:"
    grep -E "PRINTER_NAME|VIDEO_DEVICE|STORAGE_PATH" $ENV_FILE 2>/dev/null | while read line; do
        print_info "$line"
    done
else
    print_warning "Environment file does NOT exist: $ENV_FILE"
fi

# Check if application binary exists
APP_BINARY="$HOME/cam_test"
if [ -f "$APP_BINARY" ]; then
    print_success "Application binary exists: $APP_BINARY"
    print_info "Executable: $([ -x "$APP_BINARY" ] && echo "Yes" || echo "No")"
    print_info "Size: $(du -h $APP_BINARY | cut -f1)"
else
    print_warning "Application binary does NOT exist: $APP_BINARY"
fi

# Summary
print_header "System Readiness Summary"

# Count issues
errors=0
warnings=0

# Check critical components
echo "  Critical components:"

# Camera
if gphoto2 --auto-detect 2>&1 | grep -q "usb:"; then
    print_success "Camera: Ready"
else
    print_error "Camera: Not detected"
    errors=$((errors + 1))
fi

# V4L2 Loopback
if lsmod | grep -q v4l2loopback; then
    print_success "V4L2 Loopback: Ready"
else
    print_error "V4L2 Loopback: Not loaded"
    errors=$((errors + 1))
fi

# CUPS
if systemctl is-active --quiet cups; then
    print_success "CUPS: Running"
else
    print_error "CUPS: Not running"
    errors=$((errors + 1))
fi

# Printer
if lpstat -p 2>/dev/null | grep -q "XP8700\|XP-8700"; then
    print_success "Epson XP-8700: Configured"
else
    print_warning "Epson XP-8700: Not found"
    warnings=$((warnings + 1))
fi

# Database
if [ -f "$DB_PATH" ]; then
    print_success "Database: Ready"
else
    print_warning "Database: Missing"
    warnings=$((warnings + 1))
fi

# Application
if [ -x "$APP_BINARY" ]; then
    print_success "Application: Ready"
else
    print_warning "Application: Not found"
    warnings=$((warnings + 1))
fi

# Final status
echo ""
if [ $errors -eq 0 ] && [ $warnings -eq 0 ]; then
    echo -e "${GREEN}✓ System is ready for photo booth operation!${NC}"
elif [ $errors -eq 0 ]; then
    echo -e "${YELLOW}⚠ System has $warnings warning(s) but can run${NC}"
else
    echo -e "${RED}✗ System has $errors error(s) that must be fixed${NC}"
fi

echo ""
echo "Run './cam_test' to start the application"
echo "Run 'sudo systemctl status photobooth-kiosk' to check kiosk mode"

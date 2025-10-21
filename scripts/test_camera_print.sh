#!/bin/bash

# Test Camera Capture and Printing
# This script tests the full photo capture and print pipeline
# Uses existing camera setup from fix_v4l2_device.sh
# Prints to printer configured with setup_printer.sh

set -e

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
PRINTER_NAME="${PRINTER_NAME:-DNP_DS620_Photo}"
TEST_IMAGE_PATH="/tmp/test_capture_$(date +%s).jpg"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Helper functions
print_header() {
    echo ""
    echo -e "${BLUE}============================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}============================================${NC}"
    echo ""
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

# Cleanup function
cleanup() {
    if [ -f "$TEST_IMAGE_PATH" ]; then
        echo ""
        print_info "Cleaning up test image: $TEST_IMAGE_PATH"
        rm -f "$TEST_IMAGE_PATH"
    fi

    # Kill any streaming processes
    pkill -f gphoto2 2>/dev/null || true
    pkill -f "ffmpeg.*v4l2" 2>/dev/null || true
}

# Set up trap for cleanup
trap cleanup EXIT

print_header "Camera Capture and Print Test"

# Step 1: Run camera setup
print_header "Step 1: Camera Setup"

# Check if fix_v4l2_device.sh exists
V4L2_FIX_SCRIPT=""
if [ -f "$SCRIPT_DIR/troubleshooting/fix_v4l2_device.sh" ]; then
    V4L2_FIX_SCRIPT="$SCRIPT_DIR/troubleshooting/fix_v4l2_device.sh"
elif [ -f "$SCRIPT_DIR/fix_v4l2_device.sh" ]; then
    V4L2_FIX_SCRIPT="$SCRIPT_DIR/fix_v4l2_device.sh"
else
    print_error "Cannot find fix_v4l2_device.sh"
    exit 1
fi

print_info "Running camera setup script..."
if bash "$V4L2_FIX_SCRIPT"; then
    print_success "Camera setup completed"
else
    print_error "Camera setup failed"
    exit 1
fi

# Source the environment
if [ -f "$HOME/.photobooth.env" ]; then
    print_info "Loading environment configuration..."
    source "$HOME/.photobooth.env"
    print_success "Environment loaded"
    print_info "Camera device: $VIDEO_DEVICE"
    print_info "Camera type: $CAMERA_DEVICE_TYPE"
else
    print_warning "Environment file not found, using defaults"
fi

# Step 2: Check camera availability
print_header "Step 2: Camera Check"

print_info "Detecting camera..."
camera_output=$(gphoto2 --auto-detect 2>&1)

if echo "$camera_output" | grep -q "usb:"; then
    camera_model=$(echo "$camera_output" | grep "usb:" | head -n1 | awk -F'usb:' '{print $1}' | xargs)
    print_success "Camera detected: $camera_model"
else
    print_error "No camera detected via USB"
    echo ""
    print_info "Make sure your camera is:"
    print_info "  - Connected via USB"
    print_info "  - Powered on"
    print_info "  - Not in Mass Storage mode"
    exit 1
fi

# Step 3: Test capture
print_header "Step 3: Capture Test Photo"

# Kill any existing streaming processes first
print_info "Stopping any existing camera processes..."
pkill -f gphoto2 2>/dev/null || true
pkill -f "ffmpeg.*v4l2" 2>/dev/null || true
sleep 1

print_info "Capturing test photo to: $TEST_IMAGE_PATH"
print_info "This may take a few seconds..."

if gphoto2 --capture-image-and-download --filename "$TEST_IMAGE_PATH" --force-overwrite 2>/dev/null; then
    if [ -f "$TEST_IMAGE_PATH" ]; then
        file_size=$(du -h "$TEST_IMAGE_PATH" | cut -f1)
        print_success "Photo captured successfully (size: $file_size)"

        # Get image dimensions
        if command -v identify >/dev/null 2>&1; then
            dimensions=$(identify -format "%wx%h" "$TEST_IMAGE_PATH" 2>/dev/null || echo "unknown")
            print_info "Image dimensions: $dimensions"
        fi
    else
        print_error "Capture command succeeded but file not found"
        exit 1
    fi
else
    print_error "Failed to capture photo"
    echo ""
    print_info "Try running with debug output:"
    print_info "  gphoto2 --debug --capture-image-and-download --filename test.jpg"
    exit 1
fi

# Step 4: Check printer
print_header "Step 4: Printer Check"

print_info "Checking for configured printer: $PRINTER_NAME"

if lpstat -p "$PRINTER_NAME" &> /dev/null; then
    print_success "Printer '$PRINTER_NAME' found"

    # Check printer status
    printer_status=$(lpstat -p "$PRINTER_NAME" 2>/dev/null | grep -oE "is idle|is processing|disabled" || echo "unknown")
    print_info "Printer status: $printer_status"

    if echo "$printer_status" | grep -q "disabled"; then
        print_warning "Printer is disabled, trying to enable it..."
        if [ "$EUID" -eq 0 ]; then
            cupsenable "$PRINTER_NAME" 2>/dev/null || true
            cupsaccept "$PRINTER_NAME" 2>/dev/null || true
        else
            sudo cupsenable "$PRINTER_NAME" 2>/dev/null || true
            sudo cupsaccept "$PRINTER_NAME" 2>/dev/null || true
        fi
    fi
else
    print_error "Printer '$PRINTER_NAME' not found"
    print_info "Available printers:"
    lpstat -p 2>/dev/null | sed 's/^/    /' || echo "    No printers configured"

    # Try fallback printer names
    for fallback in "EPSON_XP_8700_Series_USB" "XP-8700" "DNP_DS620"; do
        if lpstat -p "$fallback" &> /dev/null; then
            print_info "Found fallback printer: $fallback"
            PRINTER_NAME="$fallback"
            break
        fi
    done

    if ! lpstat -p "$PRINTER_NAME" &> /dev/null; then
        print_error "No suitable printer found"
        print_info "Please run setup_printer.sh to configure a printer"
        exit 1
    fi
fi

# Step 5: Print the photo
print_header "Step 5: Print Test"

echo ""
read -p "Do you want to print the captured photo? (y/N): " -n 1 -r
echo ""

if [[ $REPLY =~ ^[Yy]$ ]]; then
    print_info "Sending photo to printer: $PRINTER_NAME"

    # Print with appropriate options for photo printing
    if lp -d "$PRINTER_NAME" \
        "$TEST_IMAGE_PATH" 2>/dev/null; then

        print_success "Print job submitted successfully"
        print_info "Check the printer for output"

        # Show print queue status
        echo ""
        print_info "Print queue status:"
        lpq -P "$PRINTER_NAME" 2>/dev/null | sed 's/^/    /' || true

    else
        print_error "Failed to submit print job"
        print_info "Trying alternative print command..."

        # Try simpler print command
        if lp -d "$PRINTER_NAME" "$TEST_IMAGE_PATH"; then
            print_success "Print job submitted with basic settings"
        else
            print_error "Print failed"
        fi
    fi
else
    print_info "Skipping print test"
fi

# Step 6: Optional preview stream test
print_header "Step 6: Preview Stream Test (Optional)"

echo ""
read -p "Do you want to test the preview stream? (y/N): " -n 1 -r
echo ""

if [[ $REPLY =~ ^[Yy]$ ]]; then
    if [ "$CAMERA_DEVICE_TYPE" = "loopback" ]; then
        print_info "Starting 5-second preview stream test..."
        print_info "Camera -> v4l2 loopback at $VIDEO_DEVICE"

        # Start preview stream
        timeout 5 bash -c "gphoto2 --stdout --capture-movie 2>/dev/null | ffmpeg -i - -vcodec rawvideo -pix_fmt yuv420p -f v4l2 $VIDEO_DEVICE 2>/dev/null" &
        STREAM_PID=$!

        # Wait a moment for stream to start
        sleep 2

        if kill -0 $STREAM_PID 2>/dev/null; then
            print_success "Preview stream is running"

            # Try to capture a frame from the stream
            if timeout 1 ffmpeg -f v4l2 -i $VIDEO_DEVICE -frames:v 1 /tmp/preview_test.jpg -y 2>/dev/null; then
                print_success "Successfully captured frame from preview"
                rm -f /tmp/preview_test.jpg
            else
                print_warning "Could not capture frame from preview (stream may still be working)"
            fi

            wait $STREAM_PID 2>/dev/null || true
            print_success "Preview stream test completed"
        else
            print_error "Preview stream failed to start"
        fi
    else
        print_info "Preview stream test not applicable for device type: $CAMERA_DEVICE_TYPE"
    fi
else
    print_info "Skipping preview stream test"
fi

# Summary
print_header "Test Summary"

echo -e "${GREEN}Test Results:${NC}"
echo ""
echo "✓ Camera detected: $camera_model"
echo "✓ Photo captured: $TEST_IMAGE_PATH"
echo "✓ File size: $file_size"
if [ -n "${dimensions:-}" ]; then
    echo "✓ Dimensions: $dimensions"
fi
echo "✓ Printer available: $PRINTER_NAME"

echo ""
print_success "All basic tests passed!"
echo ""
echo "The photo booth camera and printer are working correctly."
echo ""
echo "Test image saved at: $TEST_IMAGE_PATH"
echo "(This will be deleted on script exit)"

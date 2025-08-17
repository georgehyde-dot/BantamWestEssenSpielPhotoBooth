#!/usr/bin/env bash
# Debug script for CUPS printing permissions and file access issues
# Run this on the Raspberry Pi to diagnose and fix printing problems

set -euo pipefail

# Configuration
CAPTURES_DIR="/usr/local/share/photo_booth"
TEST_IMAGE="/usr/local/share/photo_booth/test.jpg"
PRINTER_NAME="EPSON_XP_8700_Series_USB"
APP_USER="prospero"
CUPS_USER="lp"

echo "=============================================="
echo "CUPS Printing Debug Script"
echo "=============================================="

# Function to check if running as root
check_root() {
    if [[ $EUID -eq 0 ]]; then
        echo "✓ Running as root (required for some fixes)"
    else
        echo "⚠ Not running as root - some fixes may not work"
        echo "  Run with: sudo ./debug_print.sh"
    fi
}

# Function to check file permissions
check_permissions() {
    echo
    echo "--- File Permissions Check ---"

    if [[ -d "$CAPTURES_DIR" ]]; then
        echo "Captures directory: $CAPTURES_DIR"
        ls -la "$CAPTURES_DIR/" | head -10
        echo

        # Check if lp user can read the directory
        if sudo -u lp test -r "$CAPTURES_DIR" 2>/dev/null; then
            echo "✓ lp user can read captures directory"
        else
            echo "✗ lp user CANNOT read captures directory"
        fi

        # Check individual files
        for file in "$CAPTURES_DIR"/*.jpg; do
            if [[ -f "$file" ]]; then
                echo "Checking file: $(basename "$file")"
                ls -la "$file"
                if sudo -u lp test -r "$file" 2>/dev/null; then
                    echo "  ✓ lp user can read this file"
                else
                    echo "  ✗ lp user CANNOT read this file"
                fi
                break  # Just check the first one
            fi
        done
    else
        echo "✗ Captures directory does not exist: $CAPTURES_DIR"
    fi
}

# Function to check CUPS status
check_cups_status() {
    echo
    echo "--- CUPS Status Check ---"

    systemctl is-active cups || echo "CUPS service status issue"

    echo "Available printers:"
    lpstat -p

    echo
    echo "Printer $PRINTER_NAME details:"
    lpstat -l -p "$PRINTER_NAME" || echo "Printer not found"

    echo
    echo "Recent CUPS error log:"
    tail -20 /var/log/cups/error_log || echo "Cannot read CUPS error log"
}

# Function to test MIME type detection
test_mime_detection() {
    echo
    echo "--- MIME Type Detection Test ---"

    for file in "$CAPTURES_DIR"/*.jpg; do
        if [[ -f "$file" ]]; then
            echo "Testing file: $(basename "$file")"
            echo "  File command: $(file "$file")"
            echo "  MIME type: $(file -b --mime-type "$file")"

            # Test as lp user
            echo "  As lp user:"
            sudo -u lp file "$file" 2>/dev/null || echo "    ✗ lp user cannot access file"
            sudo -u lp file -b --mime-type "$file" 2>/dev/null || echo "    ✗ lp user cannot detect MIME type"
            break
        fi
    done
}

# Function to fix permissions
fix_permissions() {
    echo
    echo "--- Fixing Permissions ---"

    if [[ $EUID -ne 0 ]]; then
        echo "⚠ Need root privileges to fix permissions"
        return 1
    fi

    echo "Setting proper permissions for captures directory..."

    # Make sure the captures directory exists and is readable by lp
    mkdir -p "$CAPTURES_DIR"
    chown -R "$APP_USER:$APP_USER" "$CAPTURES_DIR"
    chmod 755 "$CAPTURES_DIR"

    # Make all existing files readable by lp user
    find "$CAPTURES_DIR" -name "*.jpg" -exec chmod 644 {} \;

    # Add lp user to the app user's group for future access
    usermod -a -G "$APP_USER" lp || echo "Could not add lp to $APP_USER group"

    echo "✓ Permissions fixed"

    # Test again
    echo "Testing after fix..."
    if sudo -u lp test -r "$CAPTURES_DIR" 2>/dev/null; then
        echo "✓ lp user can now read captures directory"
    else
        echo "✗ lp user still cannot read captures directory"
    fi
}

# Function to create test image
create_test_image() {
    echo
    echo "--- Creating Test Image ---"

    mkdir -p "$CAPTURES_DIR"

    # Create a simple test JPEG using ImageMagick if available
    if command -v convert &> /dev/null; then
        convert -size 1280x720 canvas:blue -pointsize 72 -fill white -gravity center \
                -annotate +0+0 "Test Photo\n$(date)" "$TEST_IMAGE"
        chmod 644 "$TEST_IMAGE"
        echo "✓ Test image created: $TEST_IMAGE"
    else
        echo "⚠ ImageMagick not available, cannot create test image"
        return 1
    fi
}

# Function to test direct CUPS printing
test_cups_printing() {
    echo
    echo "--- Testing Direct CUPS Printing ---"

    # Find a test image
    local test_file=""
    if [[ -f "$TEST_IMAGE" ]]; then
        test_file="$TEST_IMAGE"
    else
        for file in "$CAPTURES_DIR"/*.jpg; do
            if [[ -f "$file" ]]; then
                test_file="$file"
                break
            fi
        done
    fi

    if [[ -z "$test_file" ]]; then
        echo "✗ No test image found"
        return 1
    fi

    echo "Using test file: $test_file"

    # Test with lp command
    echo "Testing with lp command..."
    lp -d "$PRINTER_NAME" \
       -o PageSize=4x6.Borderless \
       -o InputSlot=Photo \
       -o MediaType=PhotographicSemiGloss \
       -o copies=1 \
       "$test_file" && echo "✓ Print job submitted" || echo "✗ Print job failed"

    # Show job status
    echo
    echo "Print job status:"
    lpstat -o

    echo
    echo "Recent access log:"
    tail -10 /var/log/cups/access_log || echo "Cannot read access log"
}

# Function to test file conversion
test_file_conversion() {
    echo
    echo "--- Testing File Conversion ---"

    for file in "$CAPTURES_DIR"/*.jpg; do
        if [[ -f "$file" ]]; then
            local temp_file="/tmp/cups_test_$(date +%s).jpg"
            echo "Converting $file to $temp_file"

            if command -v convert &> /dev/null; then
                convert "$file" -quality 95 -strip "$temp_file"
                chmod 644 "$temp_file"

                echo "Original file:"
                ls -la "$file"
                file "$file"

                echo "Converted file:"
                ls -la "$temp_file"
                file "$temp_file"

                # Test as lp user
                echo "Testing lp user access to converted file:"
                sudo -u lp file "$temp_file" || echo "lp cannot access converted file"

                # Clean up
                rm -f "$temp_file"
            else
                echo "ImageMagick not available for conversion test"
            fi
            break
        fi
    done
}

# Function to test different paper settings
test_paper_settings() {
    echo
    echo "--- Testing Different Paper Settings ---"

    local test_file=""
    if [[ -f "$TEST_IMAGE" ]]; then
        test_file="$TEST_IMAGE"
    else
        for file in "$CAPTURES_DIR"/*.jpg; do
            if [[ -f "$file" ]]; then
                test_file="$file"
                break
            fi
        done
    fi

    if [[ -z "$test_file" ]]; then
        echo "✗ No test image found - creating one first..."
        create_test_image
        test_file="$TEST_IMAGE"
    fi

    echo "Using test file: $test_file"
    echo

    # Test 1: Default settings (should work according to lpoptions)
    echo "=== Test 1: Default 4x6 Borderless Photo Settings ==="
    lp -d "$PRINTER_NAME" \
       -o PageSize=4x6.Borderless \
       -o InputSlot=Photo \
       -o MediaType=PhotographicSemiGloss \
       -o copies=1 \
       "$test_file" && echo "✓ Test 1 job submitted" || echo "✗ Test 1 job failed"

    echo
    sleep 2

    # Test 2: Try Main tray instead of Photo tray
    echo "=== Test 2: Same settings but Main tray ==="
    lp -d "$PRINTER_NAME" \
       -o PageSize=4x6.Borderless \
       -o InputSlot=Main \
       -o MediaType=PhotographicSemiGloss \
       -o copies=1 \
       "$test_file" && echo "✓ Test 2 job submitted" || echo "✗ Test 2 job failed"

    echo
    sleep 2

    # Test 3: Try regular 4x6 (non-borderless)
    echo "=== Test 3: Regular 4x6 (non-borderless) ==="
    lp -d "$PRINTER_NAME" \
       -o PageSize=4x6 \
       -o InputSlot=Photo \
       -o MediaType=PhotographicSemiGloss \
       -o copies=1 \
       "$test_file" && echo "✓ Test 3 job submitted" || echo "✗ Test 3 job failed"

    echo
    sleep 2

    # Test 4: Try high gloss paper type
    echo "=== Test 4: High Gloss Paper ==="
    lp -d "$PRINTER_NAME" \
       -o PageSize=4x6.Borderless \
       -o InputSlot=Photo \
       -o MediaType=PhotographicHighGloss \
       -o copies=1 \
       "$test_file" && echo "✓ Test 4 job submitted" || echo "✗ Test 4 job failed"

    echo
    sleep 2

    # Test 5: Try auto tray selection
    echo "=== Test 5: Auto Tray Selection ==="
    lp -d "$PRINTER_NAME" \
       -o PageSize=4x6.Borderless \
       -o InputSlot=Auto \
       -o MediaType=PhotographicSemiGloss \
       -o copies=1 \
       "$test_file" && echo "✓ Test 5 job submitted" || echo "✗ Test 5 job failed"

    echo
    echo "=== All paper setting tests completed ==="
    echo "Check printer for physical output from any of these tests"
    echo "Recent job queue:"
    lpstat -o | tail -5
}

# Function to show systemd service logs
check_app_logs() {
    echo
    echo "--- Application Logs ---"
    echo "Recent cam_test logs:"
    journalctl -u cam_test.service -n 20 --no-pager || echo "No cam_test service logs found"
}

# Main execution
main() {
    check_root
    check_permissions
    check_cups_status
    test_mime_detection

    echo
    echo "=============================================="
    echo "Would you like to:"
    echo "1. Fix permissions (requires root)"
    echo "2. Create test image"
    echo "3. Test CUPS printing"
    echo "4. Test file conversion"
    echo "5. Check app logs"
    echo "6. Test different paper settings"
    echo "7. All of the above"
    echo "=============================================="

    if [[ $# -eq 0 ]]; then
        echo "Usage: $0 [1-7|fix|test|convert|logs|paper|all]"
        echo "Running basic checks only..."
        return 0
    fi

    case "${1:-}" in
        "1"|"fix")
            fix_permissions
            ;;
        "2"|"create")
            create_test_image
            ;;
        "3"|"test")
            test_cups_printing
            ;;
        "4"|"convert")
            test_file_conversion
            ;;
        "5"|"logs")
            check_app_logs
            ;;
        "6"|"paper")
            test_paper_settings
            ;;
        "7"|"all")
            fix_permissions
            create_test_image
            test_file_conversion
            test_cups_printing
            check_app_logs
            ;;
        *)
            echo "Invalid option: $1"
            ;;
    esac
}

main "$@"

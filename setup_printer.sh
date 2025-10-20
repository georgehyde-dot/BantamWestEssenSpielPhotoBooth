#!/bin/bash

# DNP DS620 Printer Setup Script for Photo Booth
# This script configures the DNP DS620 photo printer with Gutenprint driver
# Run after setup_packages.sh has installed the base system

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Printer configuration
DNP_PRINTER_PATTERN="DS620|DNP_DS620|DNP-DS620"
DNP_PRINTER_NAME="DNP_DS620_Photo"
# Default driver - will be detected dynamically if available
DNP_DRIVER_PPD="gutenprint.5.3://dnp-ds620/expert"

# Log functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[✓]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_header() {
    echo ""
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}  $1${NC}"
    echo -e "${GREEN}========================================${NC}"
    echo ""
}

# Check if running as root
check_root() {
    if [[ $EUID -ne 0 ]]; then
        log_error "This script must be run with sudo privileges"
        echo "Please run: sudo $0"
        exit 1
    fi
}

# Install printer driver if needed
install_gutenprint() {
    log_header "Checking Gutenprint Driver"

    if dpkg -l | grep -q printer-driver-gutenprint; then
        log_success "Gutenprint driver already installed"
    else
        log_info "Installing Gutenprint driver..."
        apt-get update
        apt-get install -y printer-driver-gutenprint
        log_success "Gutenprint driver installed"
    fi
}

# Setup CUPS service
setup_cups() {
    log_header "Setting up CUPS Service"

    # Check if CUPS is running
    if systemctl is-active --quiet cups; then
        log_success "CUPS service is running"
    else
        log_info "Starting CUPS service..."
        systemctl start cups
        systemctl enable cups
        log_success "CUPS service started and enabled"
    fi
}

# Find and configure DNP DS620 printer
configure_printer() {
    log_header "Configuring DNP DS620 Printer"

    # Check if printer is already configured
    if lpstat -p "$DNP_PRINTER_NAME" &> /dev/null; then
        log_success "Printer '$DNP_PRINTER_NAME' is already configured"

        # Update default options anyway
        log_info "Updating printer default options..."
        lpoptions -p "$DNP_PRINTER_NAME" \
            -o PageSize=w288h432 \
            -o StpiShrinkOutput=Expand \
            -o Resolution=300x300dpi \
            -o StpLaminate=Glossy \
            -o StpImageType=Photo

        log_success "Printer options updated"
        return 0
    fi

    # Search for DNP printer
    log_info "Searching for DNP DS620 printer..."
    local device_uri=$(lpinfo -v 2>/dev/null | grep -iE "$DNP_PRINTER_PATTERN" | awk '{print $2}' | head -1)

    if [ -z "$device_uri" ]; then
        log_warning "DNP DS620 not detected via auto-discovery"
        log_info "Checking USB devices for DNP printer..."
        device_uri=$(lpinfo -v 2>/dev/null | grep -i "usb" | grep -iE "dnp|ds620" | awk '{print $2}' | head -1)
    fi

    if [ -z "$device_uri" ]; then
        log_error "No DNP DS620 printer found"
        log_info "Available printer connections:"
        lpinfo -v 2>/dev/null | grep -E "usb|direct" | head -10 || true
        return 1
    fi

    log_success "DNP printer found at: $device_uri"

    # Detect the correct Gutenprint driver version
    local driver_ppd="$DNP_DRIVER_PPD"
    local available_driver=$(lpinfo -m 2>/dev/null | grep -i "dnp.*ds620.*expert" | head -1 | awk '{print $1}')

    if [ -n "$available_driver" ]; then
        log_info "Found driver: $available_driver"
        driver_ppd="$available_driver"
    else
        log_warning "Using default driver: $driver_ppd"
    fi

    log_info "Adding printer queue..."

    # Add the printer queue
    lpadmin -p "$DNP_PRINTER_NAME" \
        -E \
        -v "$device_uri" \
        -m "$driver_ppd"

    if [ $? -eq 0 ]; then
        log_success "Printer queue added successfully"

        # Set default options for photo printing
        log_info "Setting default print options..."
        lpoptions -p "$DNP_PRINTER_NAME" \
            -o PageSize=w288h432 \
            -o StpiShrinkOutput=Expand \
            -o Resolution=300x300dpi \
            -o StpLaminate=Glossy \
            -o StpImageType=Photo

        # Set as system default printer
        lpadmin -d "$DNP_PRINTER_NAME"

        log_success "DNP DS620 printer configured and set as default"
        return 0
    else
        log_error "Failed to add printer queue"
        return 1
    fi
}

# Verify printer setup
verify_printer() {
    log_header "Verifying Printer Setup"

    if lpstat -p "$DNP_PRINTER_NAME" &> /dev/null; then
        log_success "Printer '$DNP_PRINTER_NAME' found in CUPS"

        # Show printer status
        log_info "Printer status:"
        lpstat -p "$DNP_PRINTER_NAME" -l 2>/dev/null || true

        # Check if it's the default
        local default_printer=$(lpstat -d 2>/dev/null | awk '{print $4}')
        if [[ "$default_printer" == "$DNP_PRINTER_NAME" ]]; then
            log_success "Printer is set as system default"
        else
            log_warning "Printer is not the system default"
        fi

        # Check driver details
        if [ -f "/etc/cups/ppd/${DNP_PRINTER_NAME}.ppd" ]; then
            local driver_info=$(grep -E "NickName|ModelName" "/etc/cups/ppd/${DNP_PRINTER_NAME}.ppd" | head -1)
            log_info "Driver: $driver_info"

            if grep -q "Gutenprint" "/etc/cups/ppd/${DNP_PRINTER_NAME}.ppd" 2>/dev/null; then
                log_success "Using Gutenprint driver"
            fi
        fi

        return 0
    else
        log_error "Printer '$DNP_PRINTER_NAME' not found in CUPS"
        log_info "Available printers:"
        lpstat -p 2>/dev/null || echo "  No printers configured"
        return 1
    fi
}

# Test print functionality
test_print() {
    log_header "Testing Printer"

    read -p "Do you want to print a test page? (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        log_info "Printing test page..."
        echo "Photo Booth Test Page - $(date)" | lp -d "$DNP_PRINTER_NAME"
        if [ $? -eq 0 ]; then
            log_success "Test print job submitted"
            log_info "Check printer for output"
        else
            log_error "Failed to submit test print job"
        fi
    fi
}

# Main setup flow
main() {
    log_header "DNP DS620 Photo Printer Setup"

    check_root
    install_gutenprint
    setup_cups

    if configure_printer; then
        verify_printer
        test_print

        log_header "Setup Complete"
        log_success "DNP DS620 printer is ready for photo booth operation"

        echo ""
        echo "Printer name: $DNP_PRINTER_NAME"
        echo "Default settings:"
        echo "  - Paper size: 4x6 inches (w288h432)"
        echo "  - Resolution: 300x300 DPI"
        echo "  - Output: Glossy photo"
        echo ""
        echo "Test printing from command line:"
        echo "  echo 'Test' | lp -d $DNP_PRINTER_NAME"
        echo ""
    else
        log_header "Setup Failed"
        log_error "Could not configure DNP DS620 printer"
        echo ""
        echo "Troubleshooting:"
        echo "1. Ensure the DNP DS620 is connected via USB"
        echo "2. Check that it's powered on"
        echo "3. Try running: lpinfo -v"
        echo "4. Check CUPS web interface: http://localhost:631"
        exit 1
    fi
}

# Run main function
main "$@"

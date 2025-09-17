#!/usr/bin/env bash
# Printer Management Utilities for Epson XP-8700 with TurboPrint
# This script provides helpful utilities for managing the photo booth printer

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
PRINTER_PATTERN="XP-8700|XP8700"
TURBOPRINT_PPD_PATH="/usr/share/turboprint/ppd/Epson"
CUPS_PPD_PATH="/etc/cups/ppd"

# Helper functions
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

show_header() {
    echo -e "\n${GREEN}=== $1 ===${NC}\n"
}

# Check if running as root when needed
check_root() {
    if [ "$EUID" -ne 0 ]; then
        log_error "This command requires sudo privileges"
        echo "Please run: sudo $0 $@"
        exit 1
    fi
}

# Function to show printer status
status() {
    show_header "Printer Status"

    # Check USB devices
    echo -e "${BLUE}USB Devices:${NC}"
    if lsusb | grep -qi "epson"; then
        log_success "Epson device detected on USB"
        lsusb | grep -i epson
    else
        log_warning "No Epson device detected on USB"
    fi
    echo

    # Check CUPS status
    echo -e "${BLUE}CUPS Service:${NC}"
    if systemctl is-active --quiet cups; then
        log_success "CUPS is running"
    else
        log_error "CUPS is not running"
        echo "  Start with: sudo systemctl start cups"
    fi
    echo

    # List configured printers
    echo -e "${BLUE}Configured Printers:${NC}"
    local printers=$(lpstat -p 2>/dev/null | grep -Ei "$PRINTER_PATTERN" || true)
    if [ -n "$printers" ]; then
        echo "$printers"
        echo

        # Check driver for each printer
        while IFS= read -r line; do
            local printer_name=$(echo "$line" | awk '{print $2}')
            if [ -n "$printer_name" ]; then
                echo -e "${BLUE}Driver for $printer_name:${NC}"
                if [ -f "$CUPS_PPD_PATH/${printer_name}.ppd" ]; then
                    local driver=$(grep -E "^\*NickName" "$CUPS_PPD_PATH/${printer_name}.ppd" | cut -d'"' -f2)
                    if echo "$driver" | grep -qi "turboprint"; then
                        log_success "Using TurboPrint driver"
                        echo "  $driver"
                    else
                        log_warning "Using standard CUPS driver"
                        echo "  $driver"
                    fi
                fi
            fi
        done <<< "$printers"
    else
        log_warning "No Epson XP-8700 printer configured"
        echo "  Add printer with: $0 add-printer"
    fi
    echo

    # Check TurboPrint status
    echo -e "${BLUE}TurboPrint Status:${NC}"
    if command -v tpstatus >/dev/null 2>&1; then
        log_success "TurboPrint is installed"
        tpconfig --version 2>/dev/null | head -1 || true

        # Check license
        if sudo tpconfig --license 2>&1 | grep -qi "valid"; then
            log_success "TurboPrint license is valid"
        else
            log_warning "TurboPrint license may need attention"
        fi
    else
        log_warning "TurboPrint is not installed"
        echo "  Install instructions: $0 install-turboprint"
    fi
    echo

    # Check print queue
    echo -e "${BLUE}Print Queue:${NC}"
    local jobs=$(lpstat -o 2>/dev/null | grep -Ei "$PRINTER_PATTERN" || true)
    if [ -n "$jobs" ]; then
        echo "$jobs"
    else
        echo "  No pending print jobs"
    fi
}

# Function to list available printers
list-available() {
    show_header "Available Printer Connections"

    echo -e "${BLUE}USB Printers:${NC}"
    sudo lpinfo -v 2>/dev/null | grep -E "usb://" | grep -i epson || echo "  No Epson USB printers detected"
    echo

    echo -e "${BLUE}Network Printers:${NC}"
    sudo lpinfo -v 2>/dev/null | grep -E "socket://|ipp://|dnssd://" | grep -i epson || echo "  No Epson network printers detected"
    echo

    echo -e "${BLUE}All Available Connections:${NC}"
    sudo lpinfo -v 2>/dev/null | head -20
}

# Function to add printer with TurboPrint
add-printer() {
    check_root
    show_header "Add Printer with TurboPrint"

    # Check if TurboPrint is installed
    if ! command -v tpconfig >/dev/null 2>&1; then
        log_error "TurboPrint is not installed"
        echo "Install TurboPrint first with: $0 install-turboprint"
        exit 1
    fi

    # Run TurboPrint configuration
    log_info "Launching TurboPrint configuration..."
    tpconfig

    log_success "TurboPrint configuration completed"
    echo "Your printer should now be configured. Verify with: $0 status"
}

# Function to update existing printer to use TurboPrint
update-driver() {
    check_root
    show_header "Update Printer to Use TurboPrint Driver"

    # Find existing printer
    local printer_name=$(lpstat -p 2>/dev/null | grep -Ei "$PRINTER_PATTERN" | awk '{print $2}' | head -1)

    if [ -z "$printer_name" ]; then
        log_error "No Epson XP-8700 printer found"
        echo "Add a printer first with: $0 add-printer"
        exit 1
    fi

    log_info "Found printer: $printer_name"

    # Check if TurboPrint is installed
    if ! command -v tpconfig >/dev/null 2>&1; then
        log_error "TurboPrint is not installed"
        echo "Install TurboPrint first with: $0 install-turboprint"
        exit 1
    fi

    # Find TurboPrint PPD
    local ppd_file=$(find "$TURBOPRINT_PPD_PATH" -name "*XP-8700*.ppd" 2>/dev/null | head -1)

    if [ -z "$ppd_file" ]; then
        log_error "TurboPrint PPD for XP-8700 not found"
        echo "Generate PPD with: sudo /usr/share/turboprint/lib/tpsetup --genppd 'Epson XP-8700'"
        exit 1
    fi

    log_info "Using PPD: $ppd_file"

    # Update printer driver
    log_info "Updating printer driver..."
    lpadmin -p "$printer_name" -P "$ppd_file"

    # Restart CUPS
    systemctl restart cups

    log_success "Printer driver updated to TurboPrint"

    # Verify
    if grep -qi turboprint "$CUPS_PPD_PATH/${printer_name}.ppd" 2>/dev/null; then
        log_success "Verification: Printer is now using TurboPrint driver"
    else
        log_warning "Verification failed - please check printer configuration"
    fi
}

# Function to test print
test-print() {
    show_header "Test Print"

    # Find printer
    local printer_name=$(lpstat -p 2>/dev/null | grep -Ei "$PRINTER_PATTERN" | awk '{print $2}' | head -1)

    if [ -z "$printer_name" ]; then
        log_error "No Epson XP-8700 printer found"
        echo "Add a printer first with: $0 add-printer"
        exit 1
    fi

    log_info "Using printer: $printer_name"

    # Create test page
    local test_file="/tmp/photobooth_test_$(date +%s).txt"
    {
        echo "Photo Booth Printer Test Page"
        echo "================================"
        echo ""
        echo "Printer: $printer_name"
        echo "Date: $(date)"
        echo "Host: $(hostname)"
        echo ""
        echo "Driver Information:"
        grep -E "^\*NickName" "$CUPS_PPD_PATH/${printer_name}.ppd" 2>/dev/null | cut -d'"' -f2 || echo "Unknown"
        echo ""
        echo "This is a test print from the photo booth system."
        echo "If you can read this, basic printing is working."
        echo ""
        echo "For photo printing, ensure:"
        echo "- 4x6 paper is loaded"
        echo "- Photo quality is selected"
        echo "- Borderless printing is enabled"
    } > "$test_file"

    # Send test print
    log_info "Sending test print..."
    local job_id=$(lp -d "$printer_name" "$test_file" 2>&1 | grep -oP 'request id is \K[^ ]+')

    if [ -n "$job_id" ]; then
        log_success "Test print submitted (Job ID: $job_id)"
        echo "Monitor with: lpstat -o $job_id"
    else
        log_error "Failed to submit test print"
    fi

    # Clean up
    rm -f "$test_file"
}

# Function to clear print queue
clear-queue() {
    check_root
    show_header "Clear Print Queue"

    # Check for pending jobs
    local jobs=$(lpstat -o 2>/dev/null | grep -Ei "$PRINTER_PATTERN" || true)

    if [ -z "$jobs" ]; then
        log_info "No pending print jobs"
        return
    fi

    echo "Pending jobs:"
    echo "$jobs"
    echo

    read -p "Clear all print jobs? (y/N): " -n 1 -r
    echo

    if [[ $REPLY =~ ^[Yy]$ ]]; then
        cancel -a
        log_success "All print jobs cancelled"

        # Also clear CUPS spool if needed
        systemctl stop cups
        rm -f /var/spool/cups/c* 2>/dev/null || true
        rm -f /var/spool/cups/d* 2>/dev/null || true
        systemctl start cups

        log_success "CUPS spool cleared"
    else
        log_info "Cancelled"
    fi
}

# Function to show TurboPrint installation instructions
install-turboprint() {
    show_header "TurboPrint Installation Instructions"

    echo "To install TurboPrint 2.55-1 for ARM64:"
    echo
    echo "1. Download the package:"
    echo "   ${GREEN}wget https://www.turboprint.de/downloads/turboprint-2.55-1.arm64.tgz${NC}"
    echo
    echo "2. Extract the archive:"
    echo "   ${GREEN}tar -xzf turboprint-2.55-1.arm64.tgz${NC}"
    echo
    echo "3. Run the installer:"
    echo "   ${GREEN}cd turboprint-2.55-1${NC}"
    echo "   ${GREEN}sudo ./setup${NC}"
    echo
    echo "4. Configure your printer:"
    echo "   ${GREEN}sudo tpconfig${NC}"
    echo
    echo "5. Enter license (if you have one):"
    echo "   ${GREEN}sudo tpconfig --enter-license${NC}"
    echo
    echo "After installation, use this script to configure the printer:"
    echo "   ${GREEN}sudo $0 add-printer${NC}"
}

# Function to diagnose printer issues
diagnose() {
    show_header "Printer Diagnostics"

    echo -e "${BLUE}Running comprehensive diagnostics...${NC}\n"

    # Check USB
    echo "1. USB Detection:"
    if lsusb | grep -qi "epson"; then
        log_success "Epson USB device found"
        lsusb | grep -i epson
    else
        log_warning "No Epson USB device found"
    fi
    echo

    # Check CUPS
    echo "2. CUPS Service:"
    systemctl status cups --no-pager | head -5
    echo

    # Check configured printers
    echo "3. Configured Printers:"
    lpstat -t 2>/dev/null | grep -Ei "$PRINTER_PATTERN" || echo "  None found"
    echo

    # Check drivers
    echo "4. Installed Drivers:"
    if [ -d "$TURBOPRINT_PPD_PATH" ]; then
        local tp_drivers=$(find "$TURBOPRINT_PPD_PATH" -name "*XP-8700*.ppd" 2>/dev/null | wc -l)
        echo "  TurboPrint XP-8700 drivers: $tp_drivers"
    fi
    local cups_drivers=$(lpinfo -m 2>/dev/null | grep -ci "xp-8700" || echo "0")
    echo "  CUPS XP-8700 drivers: $cups_drivers"
    echo

    # Check logs
    echo "5. Recent CUPS Errors:"
    sudo tail -5 /var/log/cups/error_log 2>/dev/null || echo "  No error log found"
    echo

    # Check permissions
    echo "6. User Permissions:"
    if groups | grep -q lpadmin; then
        log_success "User is in lpadmin group"
    else
        log_warning "User is not in lpadmin group"
        echo "  Fix with: sudo usermod -a -G lpadmin $USER"
    fi
    echo

    # Network connectivity (if network printer)
    echo "7. Network Printers:"
    avahi-browse -art 2>/dev/null | grep -A 3 "Internet Printer" | head -10 || echo "  No network printers found"
    echo

    # Summary
    echo -e "${GREEN}Diagnosis Complete${NC}"
    echo "If issues persist, check:"
    echo "- USB cable connection"
    echo "- Printer power status"
    echo "- CUPS web interface: http://localhost:631"
    echo "- Run with debug: RUST_LOG=debug ./cam_test"
}

# Function to show help
show_help() {
    echo "Photo Booth Printer Management Utility"
    echo "======================================"
    echo
    echo "Usage: $0 <command>"
    echo
    echo "Commands:"
    echo "  status           - Show printer and TurboPrint status"
    echo "  list-available   - List available printer connections"
    echo "  add-printer      - Add printer using TurboPrint (requires sudo)"
    echo "  update-driver    - Update existing printer to use TurboPrint (requires sudo)"
    echo "  test-print       - Send a test print"
    echo "  clear-queue      - Clear all print jobs (requires sudo)"
    echo "  diagnose         - Run comprehensive diagnostics"
    echo "  install-turboprint - Show TurboPrint installation instructions"
    echo "  help             - Show this help message"
    echo
    echo "Examples:"
    echo "  $0 status                  # Check printer status"
    echo "  sudo $0 add-printer        # Add new printer with TurboPrint"
    echo "  sudo $0 update-driver      # Switch existing printer to TurboPrint"
    echo "  $0 test-print              # Send a test page"
    echo
    echo "For detailed setup instructions, see PRINTER_SETUP.md"
}

# Main command dispatcher
case "${1:-help}" in
    status)
        status
        ;;
    list-available)
        list-available
        ;;
    add-printer)
        add-printer
        ;;
    update-driver)
        update-driver
        ;;
    test-print)
        test-print
        ;;
    clear-queue)
        clear-queue
        ;;
    diagnose)
        diagnose
        ;;
    install-turboprint)
        install-turboprint
        ;;
    help|--help|-h)
        show_help
        ;;
    *)
        log_error "Unknown command: $1"
        echo
        show_help
        exit 1
        ;;
esac

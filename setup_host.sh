#!/usr/bin/env bash
# Photo Booth Host Setup Script for Debian Bookworm
# This script prepares a Debian 12 (Bookworm) system to run the photo booth application
# with Canon EOS Rebel T7 camera and Epson XP-8700 printer

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
PHOTO_BOOTH_USER="${PHOTO_BOOTH_USER:-${USER}}"
STORAGE_PATH="/usr/local/share/photo_booth"
CANON_VENDOR_ID="04a9"
CANON_PRODUCT_ID="32da"  # Canon EOS Rebel T7 / EOS 2000D
EPSON_PRINTER_PATTERN="XP-8700|XP8700"

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
    if [ "$EUID" -ne 0 ]; then
        log_error "This script must be run with sudo privileges"
        echo "Please run: sudo $0"
        exit 1
    fi
}

# Check OS version
check_os() {
    log_header "Checking Operating System"

    if [ -f /etc/os-release ]; then
        . /etc/os-release
        if [[ "$ID" == "debian" && "$VERSION_ID" == "12" ]]; then
            log_success "Debian 12 (Bookworm) detected"
        else
            log_warning "This script is designed for Debian 12 (Bookworm)"
            log_warning "Detected: $PRETTY_NAME"
            read -p "Continue anyway? (y/N): " -n 1 -r
            echo
            if [[ ! $REPLY =~ ^[Yy]$ ]]; then
                exit 1
            fi
        fi
    else
        log_error "Cannot determine OS version"
        exit 1
    fi
}

# Update package lists
update_packages() {
    log_header "Updating Package Lists"

    log_info "Running apt-get update..."
    apt-get update || {
        log_error "Failed to update package lists"
        exit 1
    }
    log_success "Package lists updated"
}

# Install required packages
install_packages() {
    log_header "Installing Required Packages"

    local packages=(
        # Build essentials
        build-essential
        pkg-config
        curl
        ca-certificates
        git
        clang
        libclang-dev

        # Camera support
        gphoto2
        libgphoto2-dev
        libgphoto2-port12
        v4l-utils
        v4l2loopback-dkms
        v4l2loopback-utils
        libv4l-dev
        ffmpeg

        # Printing support
        cups
        cups-client
        cups-bsd
        libcups2-dev

        # Display and kiosk mode
        chromium-browser
        xorg
        xinit
        openbox
        lightdm

        # System utilities
        systemd
        udev
        openssh-server
        sudo
        htop
        usbutils
        lsof
    )

    log_info "Installing packages..."
    local failed_packages=()

    for package in "${packages[@]}"; do
        if dpkg -l | grep -q "^ii  $package"; then
            log_success "$package already installed"
        else
            log_info "Installing $package..."
            if apt-get install -y "$package" >/dev/null 2>&1; then
                log_success "$package installed"
            else
                log_warning "Failed to install $package"
                failed_packages+=("$package")
            fi
        fi
    done

    if [ ${#failed_packages[@]} -gt 0 ]; then
        log_warning "Some packages failed to install:"
        for package in "${failed_packages[@]}"; do
            echo "  - $package"
        done
        log_warning "You may need to install these manually"
    else
        log_success "All packages installed successfully"
    fi
}

# Setup user permissions
setup_user_permissions() {
    log_header "Setting Up User Permissions"

    local groups=(video plugdev lpadmin dialout)

    for group in "${groups[@]}"; do
        if getent group "$group" >/dev/null; then
            if id -nG "$PHOTO_BOOTH_USER" | grep -qw "$group"; then
                log_success "User $PHOTO_BOOTH_USER already in group $group"
            else
                usermod -a -G "$group" "$PHOTO_BOOTH_USER"
                log_success "Added user $PHOTO_BOOTH_USER to group $group"
            fi
        else
            log_warning "Group $group does not exist"
        fi
    done

    log_info "User may need to log out and back in for group changes to take effect"
}

# Setup USB rules for Canon camera
# setup_usb_rules() {
#     log_header "Setting Up USB Rules"

#     local rules_file="/etc/udev/rules.d/99-canon-eos.rules"

#     cat > "$rules_file" << 'EOF'
# # Canon EOS Rebel T7 / EOS 2000D
# SUBSYSTEM=="usb", ATTR{idVendor}=="04a9", ATTR{idProduct}=="32da", MODE="0666", GROUP="plugdev"
# SUBSYSTEM=="usb", ATTR{idVendor}=="04a9", MODE="0666", GROUP="plugdev"

# # Prevent gvfs from mounting the camera
# ENV{ID_GPHOTO2}=="1", ENV{UDISKS_IGNORE}="1", ENV{UDISKS_PRESENTATION_HIDE}="1"
# EOF

#     log_success "USB rules created at $rules_file"

#     log_info "Reloading udev rules..."
#     udevadm control --reload-rules
#     udevadm trigger
#     log_success "USB rules reloaded"
# }

# Setup V4L2 loopback
setup_v4l2_loopback() {
    log_header "Setting Up V4L2 Loopback"

    # Check if module is already loaded
    if lsmod | grep -q v4l2loopback; then
        log_info "v4l2loopback module already loaded, removing..."
        modprobe -r v4l2loopback
    fi

    log_info "Loading v4l2loopback module..."
    modprobe v4l2loopback exclusive_caps=1 max_buffers=2 card_label="Canon EOS Rebel T7"

    if lsmod | grep -q v4l2loopback; then
        log_success "v4l2loopback module loaded"
    else
        log_error "Failed to load v4l2loopback module"
        return 1
    fi

    # Make persistent
    if ! grep -q "v4l2loopback" /etc/modules; then
        echo "v4l2loopback" >> /etc/modules
        log_success "Added v4l2loopback to /etc/modules"
    fi

    cat > /etc/modprobe.d/v4l2loopback.conf << 'EOF'
options v4l2loopback exclusive_caps=1 max_buffers=2 card_label="Canon EOS Rebel T7"
EOF
    log_success "Created /etc/modprobe.d/v4l2loopback.conf"

    # Check for video device
    if [ -e /dev/video0 ]; then
        log_success "Video device /dev/video0 exists"
    else
        log_warning "Video device /dev/video0 not found - will be created when camera is connected"
    fi
}

# Check for Canon camera
check_canon_camera() {
    log_header "Checking for Canon EOS Rebel T7"

    # Kill any processes that might be using the camera
    log_info "Stopping any processes using the camera..."
    pkill -f gphoto2 2>/dev/null || true
    pkill -f PTPCamera 2>/dev/null || true
    sleep 2

    # Check USB connection
    if lsusb | grep -qi "canon"; then
        log_success "Canon device detected via USB"
        lsusb | grep -i canon
    else
        log_warning "No Canon device detected via USB"
        log_info "Please ensure the Canon EOS Rebel T7 is:"
        echo "  1. Connected via USB cable"
        echo "  2. Powered on"
        echo "  3. Set to appropriate shooting mode"
        return 1
    fi

    # Check gphoto2 detection
    log_info "Checking gphoto2 camera detection..."
    if gphoto2 --auto-detect | grep -qi "canon"; then
        log_success "Canon camera detected by gphoto2:"
        gphoto2 --auto-detect | grep -i canon

        # Try to get camera info
        log_info "Getting camera information..."
        gphoto2 --summary 2>/dev/null | head -n 10 || true
    else
        log_warning "Camera not detected by gphoto2"
        log_info "This might be resolved by reconnecting the camera"
        return 1
    fi
}

# Check for Epson printer
check_epson_printer() {
    log_header "Checking for Epson XP-8700 Printer"

    # Check if CUPS is running
    if systemctl is-active --quiet cups; then
        log_success "CUPS service is running"
    else
        log_info "Starting CUPS service..."
        systemctl start cups
        systemctl enable cups
        log_success "CUPS service started and enabled"
    fi

    # Check USB connection first
    log_info "Checking USB devices..."
    if lsusb | grep -qi "epson"; then
        log_success "Epson device detected via USB"
        lsusb | grep -i epson
    else
        log_info "No Epson device found on USB"
    fi

    # # Check for network printers
    # log_info "Checking for network printers..."
    # lpinfo -v 2>/dev/null | grep -E "socket|ipp|dnssd" | grep -i epson || true

    # Check for Epson printer in CUPS
    log_info "Checking CUPS configuration..."
    local printer_found=false
    local printer_name=""

    if lpstat -p 2>/dev/null | grep -Ei "$EPSON_PRINTER_PATTERN"; then
        log_success "Epson XP-8700 series printer found in CUPS:"
        lpstat -p | grep -Ei "$EPSON_PRINTER_PATTERN"
        printer_found=true
        printer_name=$(lpstat -p | grep -Ei "$EPSON_PRINTER_PATTERN" | awk '{print $2}' | head -1)

        # Check driver details
        if [ -n "$printer_name" ]; then
            log_info "Printer configuration for $printer_name:"
            lpstat -p "$printer_name" -l 2>/dev/null || true

            # Check if using TurboPrint
            if [ -f "/etc/cups/ppd/${printer_name}.ppd" ]; then
                local driver_info=$(grep -E "NickName|ModelName" "/etc/cups/ppd/${printer_name}.ppd" | head -1)
                log_info "Current driver: $driver_info"
            fi
        fi
    else
        log_warning "No Epson XP-8700 printer found in CUPS"
        log_info "Available printers:"
        lpstat -p 2>/dev/null || echo "  No printers configured"

        # Show available connections
        log_info "Available printer connections:"
        lpinfo -v 2>/dev/null | head -10 || true
    fi

    if [ "$printer_found" = false ]; then
        log_info "To add the Epson XP-8700:"
        echo "  For USB: sudo lpadmin -p XP8700 -E -v usb://EPSON/XP-8700 -m everywhere"
        echo "  For Network: sudo lpadmin -p XP8700 -E -v socket://PRINTER_IP:9100 -m everywhere"
        echo "  Or use CUPS web interface: http://localhost:631"
        return 1
    fi
}


# Setup directory structure
setup_directories() {
    log_header "Setting Up Directory Structure"

    # Create main storage directory
    if [ ! -d "$STORAGE_PATH" ]; then
        mkdir -p "$STORAGE_PATH"
        log_success "Created $STORAGE_PATH"
    else
        log_success "$STORAGE_PATH already exists"
    fi

    # Create subdirectories
    local subdirs=(
        "static"
        "static/resized_output"
        "captured"
        "previews"
    )

    for subdir in "${subdirs[@]}"; do
        local full_path="$STORAGE_PATH/$subdir"
        if [ ! -d "$full_path" ]; then
            mkdir -p "$full_path"
            log_success "Created $full_path"
        else
            log_success "$full_path already exists"
        fi
    done

    # Set ownership
    chown -R "$PHOTO_BOOTH_USER:$PHOTO_BOOTH_USER" "$STORAGE_PATH"
    chmod -R 755 "$STORAGE_PATH"
    log_success "Set permissions for $STORAGE_PATH"

    # Create user directories
    local user_home="/home/$PHOTO_BOOTH_USER"
    if [ ! -d "$user_home/operations" ]; then
        log_warning "Operations directory not found at $user_home/operations"
        log_info "This will be created when you deploy the application"
    fi
}

# Setup system optimizations
# setup_optimizations() {
#     log_header "Applying System Optimizations"

#     # Increase USB buffer size
#     log_info "Increasing USB buffer size..."
#     echo 1000 > /sys/module/usbcore/parameters/usbfs_memory_mb 2>/dev/null || true

#     if ! grep -q "usbcore.usbfs_memory_mb" /etc/modprobe.d/usbcore.conf 2>/dev/null; then
#         echo "options usbcore usbfs_memory_mb=1000" > /etc/modprobe.d/usbcore.conf
#         log_success "USB buffer size increased"
#     fi

#     # Disable unnecessary services (optional)
#     log_info "Optimizing system services..."
#     local services_to_disable=(bluetooth avahi-daemon)

#     for service in "${services_to_disable[@]}"; do
#         if systemctl list-unit-files | grep -q "^$service"; then
#             systemctl disable "$service" 2>/dev/null || true
#             systemctl stop "$service" 2>/dev/null || true
#             log_success "Disabled $service"
#         fi
#     done
# }

# Create environment file
create_env_file() {
    log_header "Creating Environment Configuration"

    local env_file="/home/$PHOTO_BOOTH_USER/.photobooth.env"

    cat > "$env_file" << EOF
# Photo Booth Environment Configuration
# Source this file before running the application

# Server configuration
export HOST=0.0.0.0
export PORT=8080

# Camera configuration
export VIDEO_DEVICE=/dev/video0
export VIDEO_WIDTH=1920
export VIDEO_HEIGHT=1080
export VIDEO_FORMAT=MJPG

# Storage configuration
export STORAGE_PATH=$STORAGE_PATH

# Printer configuration
export PRINTER_NAME=XP8700series-TurboPrint
export USE_MOCK_PRINTER=false

# Template configuration
export TEMPLATE_HEADER="Photo Booth"
export TEMPLATE_NAME="NAME HERE"
export TEMPLATE_HEADLINE="HEADLINE"
export TEMPLATE_STORY="STORY HERE"
export TEMPLATE_BACKGROUND=background.png

# Logging
export RUST_LOG=info

# Display (for kiosk mode)
export DISPLAY=:0
export XAUTHORITY=/home/$PHOTO_BOOTH_USER/.Xauthority
EOF

    chown "$PHOTO_BOOTH_USER:$PHOTO_BOOTH_USER" "$env_file"
    chmod 644 "$env_file"
    log_success "Created environment file at $env_file"
    log_info "Source this file before running: source $env_file"
}

# Generate summary report
generate_report() {
    log_header "Setup Summary"

    local report_file="/home/$PHOTO_BOOTH_USER/photobooth_setup_report.txt"

    {
        echo "Photo Booth Setup Report"
        echo "Generated: $(date)"
        echo "========================"
        echo ""
        echo "System Information:"
        echo "  OS: $(lsb_release -ds 2>/dev/null || cat /etc/os-release | grep PRETTY_NAME | cut -d= -f2)"
        echo "  Kernel: $(uname -r)"
        echo "  Architecture: $(uname -m)"
        echo "  User: $PHOTO_BOOTH_USER"
        echo ""
        echo "Camera Status:"
        if gphoto2 --auto-detect | grep -qi "canon"; then
            echo "  ✓ Canon camera detected"
            gphoto2 --auto-detect | grep -i canon | head -1
        else
            echo "  ✗ Canon camera not detected"
        fi
        echo ""
        echo "Printer Status:"
        if lpstat -p 2>/dev/null | grep -Ei "$EPSON_PRINTER_PATTERN"; then
            echo "  ✓ Epson XP-8700 detected in CUPS"
            local printer_name=$(lpstat -p | grep -Ei "$EPSON_PRINTER_PATTERN" | awk '{print $2}' | head -1)
            if [ -n "$printer_name" ]; then
                if grep -qi turboprint /etc/cups/ppd/${printer_name}.ppd 2>/dev/null; then
                    echo "    Using TurboPrint driver"
                else
                    echo "    Using standard CUPS driver"
                fi
            fi
        else
            echo "  ✗ Epson XP-8700 not detected in CUPS"
        fi
        if lsusb | grep -qi "epson"; then
            echo "  ✓ Epson device detected on USB"
        fi
        echo ""
        echo "TurboPrint Status:"
        if command -v tpstatus >/dev/null 2>&1; then
            echo "  ✓ TurboPrint installed"
            tpconfig --version 2>/dev/null | head -1 || true
        else
            echo "  ✗ TurboPrint not installed"
            echo "    Download: turboprint-2.55-1.arm64.tgz"
        fi
        echo ""
        echo "Video Devices:"
        ls -la /dev/video* 2>/dev/null || echo "  No video devices found"
        echo ""
        echo "Directory Structure:"
        echo "  Storage Path: $STORAGE_PATH"
        ls -la "$STORAGE_PATH" 2>/dev/null || echo "  Not created"
        echo ""
        echo "Next Steps:"
        echo "1. Deploy the application using ./deploy.sh from your development machine"
        echo "2. Test the camera: gphoto2 --capture-preview"
        echo "3. Configure printer with TurboPrint: sudo tpconfig (if installed)"
        echo "4. Test printing: echo 'Test' | lp -d XP8700series-TurboPrint"
        echo "5. Run the application: source ~/.photobooth.env && ./cam_test"
        echo "6. Set up kiosk mode: sudo ./operations/setup-kiosk.sh"
        echo ""
        echo "For detailed printer setup, see: PRINTER_SETUP.md"
    } | tee "$report_file"

    chown "$PHOTO_BOOTH_USER:$PHOTO_BOOTH_USER" "$report_file"
    log_success "Report saved to $report_file"
}

# Main execution
main() {
    log_header "Photo Booth Host Setup Script"
    echo "Setting up host for user: $PHOTO_BOOTH_USER"
    echo ""

    check_root
    check_os
    update_packages
    install_packages
    setup_user_permissions
    setup_usb_rules
    setup_v4l2_loopback
    setup_directories
    setup_optimizations
    create_env_file

    # Check hardware
    local camera_ok=false
    local printer_ok=false
    local turboprint_ok=false

    if check_canon_camera; then
        camera_ok=true
    fi

    if check_epson_printer; then
        printer_ok=true
    fi

    if check_turboprint; then
        turboprint_ok=true
    fi

    # Generate report
    generate_report

    # Final status
    log_header "Setup Complete"

    if [ "$camera_ok" = true ] && [ "$printer_ok" = true ] && [ "$turboprint_ok" = true ]; then
        log_success "All components detected and configured!"
        log_info "The system is ready for photo booth deployment"
    else
        log_warning "Some components need attention:"
        [ "$camera_ok" = false ] && echo "  - Connect and configure Canon EOS Rebel T7"
        [ "$printer_ok" = false ] && echo "  - Connect and configure Epson XP-8700"
        [ "$turboprint_ok" = false ] && echo "  - Install TurboPrint driver (optional but recommended)"
        echo ""
        log_info "You can still deploy the application and address these issues later"
    fi

    echo ""
    log_info "Next steps:"
    echo "  1. Review the setup report: cat /home/$PHOTO_BOOTH_USER/photobooth_setup_report.txt"
    echo "  2. Deploy the application from your development machine using ./deploy.sh"
    echo "  3. Configure the target host in deploy.sh with appropriate SSH settings"
    echo ""
    log_success "Setup script completed!"
}

# Run main function
main "$@"

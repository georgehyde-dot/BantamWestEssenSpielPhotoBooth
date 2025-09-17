# Photo Booth System Requirements

## Overview
This document outlines the complete system requirements and setup procedures for deploying the Canon Test Cam photo booth application to Debian Bookworm hosts.

## Host System Requirements

### Operating System
- **Required:** Debian 12 (Bookworm) or compatible
- **Architecture:** ARM64 (aarch64) for Raspberry Pi
- **Kernel:** Linux 5.10 or newer with V4L2 support

### Hardware Requirements

#### Minimum System Specifications
- **CPU:** ARM Cortex-A72 (Raspberry Pi 4) or equivalent
- **RAM:** 2GB minimum, 4GB recommended
- **Storage:** 16GB minimum, 32GB recommended
- **Network:** Ethernet or WiFi for remote deployment
- **Display:** HDMI output for kiosk mode (1920x1080 recommended) (currently designed for touch screen use)
- **USB:** At least 2 available USB - B ports (camera and printer)

#### Required Peripherals
1. **Camera:** Canon EOS Rebel T7 (EOS 2000D)
   - Connected via USB cable
   - Set to appropriate shooting mode
   - Disable auto power-off in camera settings

2. **Printer:** Epson XP-8700
   - Connected via USB or network
   - Configured for borderless 4x6" printing
   - Configured for Turboprint XP-8700 driver (license required)

## Software Dependencies

### System Packages
These packages must be installed on the host system:

#### Core Dependencies
```bash
# Build essentials (if building on host)
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
ssh
sudo
htop
```

### TurboPrint Driver
- **Version:** 2.55-1 or newer
- **Package:** turboprint-2.55-1.arm64.tgz (for ARM64 systems)
- **License:** Required for full functionality
- **Download:** https://www.turboprint.de/downloads/turboprint-2.55-1.arm64.tgz
- **Support:** Epson XP-8700 Series
- **Installation:** Extract tarball and run ./setup script

### GPhoto2 Configuration
Required for Canon EOS Rebel T7 support:
- **Version:** 2.5.27 or newer
- **libgphoto2:** 2.5.29 or newer
- **PTP2 support:** Enabled

## Network Configuration
- **Local use only** Currently built for running on localhost with a single screen output.

### Firewall Rules
```bash
# Allow web interface
sudo ufw allow 8080/tcp

# Allow SSH (if needed)
sudo ufw allow 22/tcp
```

## File System Structure

### Required Directories
```
/home/${USER}/
├── cam_test                    # Main application binary
├── operations/                  # Service scripts
│   ├── photobooth-kiosk.service
│   ├── run-kiosk.sh
│   ├── setup-kiosk.sh
│   ├── start-kiosk.sh
│   └── stop-kiosk.sh
├── scripts/                     # Utility scripts
├── photobooth.log              # Application log
└── .ssh/                       # SSH keys for deployment

/usr/local/share/photo_booth/   # Photo storage
├── static/                     # Static assets
│   ├── background.png         # Template background
│   └── resized_output/        # Selection images
│       ├── class_1.jpg
│       ├── choice_1.jpg
│       └── ... (12 total) (format <type>_<#1-4>.jpg)
├── captured/                   # Captured photos
└── previews/                   # Generated previews
```

### Permissions
```bash
# Application directories
chmod 755 /home/${USER}/cam_test
chmod 755 /home/${USER}/operations/
chmod 755 /home/${USER}/scripts/

# Storage directories
sudo mkdir -p /usr/local/share/photo_booth/{static,captured,previews}
sudo chown -R ${USER}:${USER} /usr/local/share/photo_booth
chmod 755 /usr/local/share/photo_booth
chmod 755 /usr/local/share/photo_booth/*
```

## USB Device Configuration

### Canon EOS Rebel T7 USB Rules
Create `/etc/udev/rules.d/99-canon-eos.rules`:
```
# Canon EOS Rebel T7 / EOS 2000D
SUBSYSTEM=="usb", ATTR{idVendor}=="04a9", ATTR{idProduct}=="32da", MODE="0666", GROUP="plugdev"
SUBSYSTEM=="usb", ATTR{idVendor}=="04a9", MODE="0666", GROUP="plugdev"
```

### V4L2 Loopback for GPhoto2
```bash
# Load v4l2loopback module
sudo modprobe v4l2loopback exclusive_caps=1 max_buffers=2 card_label="Canon EOS Rebel T7"

# Make persistent
echo "v4l2loopback" | sudo tee -a /etc/modules
echo "options v4l2loopback exclusive_caps=1 max_buffers=2 card_label='Canon EOS Rebel T7'" | sudo tee /etc/modprobe.d/v4l2loopback.conf
```

## Environment Variables

### Required Configuration
```bash
# Server configuration
export HOST=0.0.0.0
export PORT=8080

# Camera configuration
export VIDEO_DEVICE=/dev/video0
export VIDEO_WIDTH=1920
export VIDEO_HEIGHT=1080
export VIDEO_FORMAT=MJPG

# Storage configuration
export STORAGE_PATH=/usr/local/share/photo_booth

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
```

### Kiosk Mode Environment
Additional variables for kiosk operation:
```bash
export DISPLAY=:0
export XAUTHORITY=/home/${USER}/.Xauthority
```

## Service Configuration

### Systemd Service
The application runs as a systemd service for kiosk mode:
- **Service Name:** photobooth-kiosk.service
- **User:** Non-root user with appropriate permissions
- **Type:** Simple
- **Restart Policy:** on-failure

### Auto-start Configuration
```bash
# Enable auto-start on boot
sudo systemctl enable photobooth-kiosk.service

# Enable graphical target as default
sudo systemctl set-default graphical.target
```

## Security Considerations

### User Permissions
- Application runs as non-root user
- User must be member of these groups:
  ```bash
  sudo usermod -a -G video,plugdev,lpadmin ${USER}
  ```

### SSH Access
For remote deployment:
- SSH key authentication required
- Password authentication disabled (recommended)
- Fail2ban installed (recommended)


## Verification Checklist

Before deployment, verify:

1. **System**
   - [ ] Debian Bookworm installed and updated
   - [ ] All required packages installed
   - [ ] User created with appropriate permissions

2. **Camera**
   - [ ] Canon EOS Rebel T7 detected by gphoto2
   - [ ] V4L2 loopback device created
   - [ ] Camera accessible at /dev/video0

3. **Printer**
   - [ ] Epson XP-8700 detected by CUPS
   - [ ] TurboPrint driver installed and licensed
   - [ ] Test print successful through CUPS

4. **Storage**
   - [ ] Directory structure created
   - [ ] Correct permissions set
   - [ ] Adequate free space available

5. **Network**
   - [ ] Port 8080 accessible
   - [ ] SSH access working (if remote)

6. **Display** (for kiosk mode)
   - [ ] X server running
   - [ ] Chromium browser installed
   - [ ] Display connected and working

## Troubleshooting

### Common Issues

#### Camera Not Detected
```bash
# Check USB connection
lsusb | grep Canon

# Kill any processes using the camera
pkill -f gphoto2
pkill -f PTPCamera

# Check gphoto2 detection
gphoto2 --auto-detect
```

#### Printer Not Working
- Access CUPS admin interface at localhost:631
```bash
# Check CUPS status
sudo systemctl status cups

# List available printers
lpstat -p -d

# Check TurboPrint status
tpstatus
```

#### V4L2 Device Missing
```bash
# Check if module is loaded
lsmod | grep v4l2loopback

# Reload module
sudo modprobe -r v4l2loopback
sudo modprobe v4l2loopback exclusive_caps=1 max_buffers=2

# Start gphoto2 webcam mode
gphoto2 --stdout --capture-movie | ffmpeg -i - -vcodec rawvideo -pix_fmt yuv420p -f v4l2 /dev/video0
```

## Support Resources

- **GPhoto2 Documentation:** http://gphoto.org/doc/
- **TurboPrint Support:** https://www.turboprint.de/support.html
- **CUPS Documentation:** https://www.cups.org/documentation.html
- **Debian Wiki:** https://wiki.debian.org/

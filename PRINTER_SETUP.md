# Epson XP-8700 Printer Setup Guide

## Overview
This guide provides detailed instructions for setting up the Epson XP-8700 printer with TurboPrint driver on Debian Bookworm (ARM64) systems.

## Prerequisites
- Debian 12 (Bookworm) ARM64 system
- CUPS installed and running
- Epson XP-8700 printer connected via USB or network
- Internet connection for driver download

## Part 1: TurboPrint Installation

### 1.1 Download TurboPrint

```bash
# Create a temporary directory for installation
mkdir -p ~/turboprint_install
cd ~/turboprint_install

# Download TurboPrint for ARM64
wget https://www.turboprint.de/downloads/turboprint-2.55-1.arm64.tgz

# Alternative download location if needed
# wget https://www.zedonet.com/downloads/turboprint-2.55-1.arm64.tgz
```

### 1.2 Extract and Install TurboPrint

```bash
# Extract the archive
tar -xzf turboprint-2.55-1.arm64.tgz

# Navigate to extracted directory
cd turboprint-2.55-1

# Run the installer (requires root)
sudo ./setup

# Follow the interactive installer:
# - Accept the license agreement
# - Choose installation type (typically "Full")
# - Confirm installation path (default: /usr/share/turboprint)
```

### 1.3 Verify TurboPrint Installation

```bash
# Check if TurboPrint is installed
which tpconfig
which tpstatus

# Check TurboPrint version
tpconfig --version

# Check TurboPrint status
sudo tpstatus
```

## Part 2: Printer Detection and Information

### 2.1 USB Connected Printer

```bash
# Check if printer is detected via USB
lsusb | grep -i epson

# Expected output example:
# Bus 001 Device 004: ID 04b8:1143 Seiko Epson Corp. XP-8700 Series

# Get detailed USB information
sudo lsusb -v -d 04b8: | grep -E "idVendor|idProduct|bcdDevice|iProduct"

# Check kernel messages for printer detection
dmesg | grep -i epson
dmesg | grep -i "usb.*printer"

# Check if usblp module is loaded
lsmod | grep usblp
```
<!--
### 2.2 Network Connected Printer

```bash
# Scan for network printers
sudo lpinfo -v | grep -E "socket|ipp|dnssd"

# Use avahi to discover network printers
avahi-browse -art | grep -A 5 "Internet Printer"

# Scan specific IP range for printers (adjust IP range as needed)
nmap -p 9100,515,631 192.168.1.0/24

# Test connection to specific printer IP
ping printer-ip-address
nc -zv printer-ip-address 9100
```-->

## Part 3: CUPS Configuration

### 3.1 Check Current CUPS Status

```bash
# Check if CUPS is running
sudo systemctl status cups

# Start CUPS if not running
sudo systemctl start cups
sudo systemctl enable cups

# List all configured printers
lpstat -p -d

# List printer drivers available in CUPS
lpinfo -m | grep -i epson
lpinfo -m | grep -i xp-8700

# List available printer connections
sudo lpinfo -v
```

### 3.2 Check Current Printer Configuration

```bash
# Get detailed info about specific printer
lpstat -p "printer-name" -l

# Check which driver a printer is using
lpoptions -p "printer-name" -l | grep -i driver

# Alternative method to check driver
cat /etc/cups/ppd/printer-name.ppd | grep -E "NickName|ModelName|Manufacturer"

# List all printers with their drivers
for printer in $(lpstat -p | awk '{print $2}'); do
    echo "Printer: $printer"
    grep -E "NickName|ModelName" /etc/cups/ppd/${printer}.ppd 2>/dev/null || echo "  PPD not found"
    echo ""
done
```

## Part 4: Adding/Updating Printer with TurboPrint

### 4.1 Remove Existing Printer (if needed)

```bash
# List current printers
lpstat -p

# Remove existing printer configuration
sudo lpadmin -x "old-printer-name"

# Verify removal
lpstat -p
```

### 4.2 Add Printer Using TurboPrint

#### Method 1: Using TurboPrint Configuration Tool (Recommended)

```bash
# Run TurboPrint configuration
sudo tpconfig

# In the TurboPrint configuration interface:
# 1. Select "Add new printer"
# 2. Choose connection type (USB or Network)
# 3. Select "Epson XP-8700 Series" from the list
# 4. Configure print settings:
#    - Paper size: 4x6" (for photo booth)
#    - Quality: High/Photo
#    - Borderless: Yes
# 5. Set printer name: "XP8700series-TurboPrint"
# 6. Test the printer
```

#### Method 2: Command Line with TurboPrint PPD

```bash
# First, generate TurboPrint PPD for Epson XP-8700
sudo /usr/share/turboprint/lib/tpsetup --genppd "Epson XP-8700"

# Find the generated PPD file
find /usr/share/turboprint -name "*XP-8700*.ppd" 2>/dev/null

# For USB connected printer
sudo lpadmin -p "XP8700series-TurboPrint" \
    -E \
    -v "usb://EPSON/XP-8700" \
    -P "/usr/share/turboprint/ppd/Epson/Epson_XP-8700_Series_TP.ppd" \
    -D "Epson XP-8700 with TurboPrint" \
    -L "Photo Booth Printer"

# For network connected printer (adjust IP)
sudo lpadmin -p "XP8700series-TurboPrint" \
    -E \
    -v "socket://192.168.1.100:9100" \
    -P "/usr/share/turboprint/ppd/Epson/Epson_XP-8700_Series_TP.ppd" \
    -D "Epson XP-8700 with TurboPrint" \
    -L "Photo Booth Printer"
```

### 4.3 Verify TurboPrint Driver is Active

```bash
# Check if printer is using TurboPrint driver
lpstat -p "XP8700series-TurboPrint" -l

# Verify PPD contains TurboPrint
grep -i turboprint /etc/cups/ppd/XP8700series-TurboPrint.ppd

# Check TurboPrint printer status
sudo tpstatus | grep -A 5 "XP-8700"

# Get TurboPrint printer info
sudo tpconfig --info "XP8700series-TurboPrint"
```

## Part 5: Printer Configuration and Testing

### 5.1 Set Default Printer Options

```bash
# Set as default printer
sudo lpadmin -d "XP8700series-TurboPrint"

# Configure for borderless 4x6" photos
sudo lpoptions -p "XP8700series-TurboPrint" -o PageSize=4x6
sudo lpoptions -p "XP8700series-TurboPrint" -o BorderlessMode=True
sudo lpoptions -p "XP8700series-TurboPrint" -o MediaType=PhotoPaper
sudo lpoptions -p "XP8700series-TurboPrint" -o PrintQuality=Photo

# List all available options
lpoptions -p "XP8700series-TurboPrint" -l
```

### 5.2 Test Printing

```bash
# Create a test file
echo "Photo Booth Printer Test Page" > /tmp/test.txt
echo "Printer: Epson XP-8700" >> /tmp/test.txt
echo "Driver: TurboPrint" >> /tmp/test.txt
echo "Date: $(date)" >> /tmp/test.txt

# Test print with text
lp -d "XP8700series-TurboPrint" /tmp/test.txt

# Check print job status
lpstat -o

# Test with an image (if available)
# Download a test image
wget -O /tmp/test_image.jpg https://via.placeholder.com/1200x1800

# Print test image at 4x6"
lp -d "XP8700series-TurboPrint" -o PageSize=4x6 -o fit-to-page /tmp/test_image.jpg

# Monitor print queue
watch -n 1 lpstat -o
```

## Part 6: Troubleshooting

### 6.1 Printer Not Detected

```bash
# Check USB connection
sudo dmesg | tail -20
lsusb -t

# Reset USB subsystem
sudo modprobe -r usblp
sudo modprobe usblp

# Restart CUPS
sudo systemctl restart cups

# Check for conflicting services
sudo systemctl status ipp-usb
sudo systemctl stop ipp-usb  # If running, might conflict
```

### 6.2 Wrong Driver Being Used

```bash
# Check current driver
current_driver=$(grep -E "NickName" /etc/cups/ppd/XP8700series-TurboPrint.ppd 2>/dev/null)
echo "Current driver: $current_driver"

# If not TurboPrint, update the PPD
sudo lpadmin -p "XP8700series-TurboPrint" \
    -P "/usr/share/turboprint/ppd/Epson/Epson_XP-8700_Series_TP.ppd"

# Restart CUPS to apply changes
sudo systemctl restart cups

# Verify change
grep -i turboprint /etc/cups/ppd/XP8700series-TurboPrint.ppd
```

### 6.3 Print Jobs Stuck

```bash
# View print queue
lpstat -o

# Cancel all print jobs
cancel -a

# Cancel specific job
cancel job-id

# Clear and restart print system
sudo systemctl stop cups
sudo rm -f /var/spool/cups/c*
sudo rm -f /var/spool/cups/d*
sudo systemctl start cups

# Check printer status
lpstat -p "XP8700series-TurboPrint" -t
```

### 6.4 TurboPrint License Issues

```bash
# Check license status
sudo tpconfig --license

# Enter license key if needed
sudo tpconfig --enter-license

# Verify printer is licensed
sudo tpstatus --check-license "XP8700series-TurboPrint"
```

## Part 7: Application Integration Checklist

After setting up the printer, verify it works with the photo booth application:

### 7.1 Environment Variables

```bash
# Add to ~/.photobooth.env
export PRINTER_NAME="XP8700series-TurboPrint"
export PRINTER_FALLBACK="EPSON_XP_8700_Series_USB,XP-8700"
export USE_MOCK_PRINTER=false
```

### 7.2 Test from Application

```bash
# Source environment
source ~/.photobooth.env

# Run application
./cam_test

# Test print endpoint
curl -X POST http://localhost:8080/print \
    -H "Content-Type: application/json" \
    -d '{"filename": "test.png"}'
```

### 7.3 Verify Logs

```bash
# Check application logs for printer errors
tail -f ~/photobooth.log | grep -i print

# Check CUPS error log
sudo tail -f /var/log/cups/error_log

# Check TurboPrint logs
sudo tail -f /var/log/turboprint/turboprint.log
```

## Diagnostic Commands Summary

```bash
# Quick printer status check
echo "=== USB Detection ==="
lsusb | grep -i epson

echo "=== CUPS Printers ==="
lpstat -p -d

echo "=== TurboPrint Status ==="
sudo tpstatus

echo "=== Current Driver ==="
grep -E "NickName" /etc/cups/ppd/*.ppd | grep -i turboprint

echo "=== Print Queue ==="
lpstat -o

echo "=== Recent CUPS Errors ==="
sudo tail -5 /var/log/cups/error_log
```

## Common Issues and Solutions

| Issue | Solution |
|-------|----------|
| Printer not detected | Check USB cable, restart CUPS, check dmesg |
| Wrong driver in use | Remove printer, re-add with TurboPrint PPD |
| Print quality issues | Check media type settings, use Photo quality |
| Borderless not working | Ensure TurboPrint driver is active, set BorderlessMode=True |
| Jobs stuck in queue | Cancel jobs, restart CUPS, check printer status |
| License expired | Run `sudo tpconfig --enter-license` with valid key |

## Support Resources

- **TurboPrint Documentation**: https://www.turboprint.de/english.html
- **CUPS Documentation**: https://www.cups.org/doc/admin.html
- **Epson Support**: https://epson.com/support/xp-8700
- **Application Logs**: `~/photobooth.log`
- **CUPS Web Interface**: http://localhost:631

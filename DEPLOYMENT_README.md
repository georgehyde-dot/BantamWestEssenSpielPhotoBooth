# Photo Booth Deployment Guide

This guide explains how to deploy the Canon Test Cam photo booth application to different Debian Bookworm hosts.

## Prerequisites

### Development Machine (Your Mac)
- Docker installed and running
- SSH key pair generated
- Access to target host(s) via SSH

### Target Host Requirements
- Debian 12 (Bookworm) or compatible OS
- Network connectivity (for deployment)
- Canon EOS Rebel T7 camera (USB connected)
- Epson XP-8700 printer (USB or network connected)
- Display for kiosk mode (optional)

## Deployment Overview

The deployment process consists of:
1. Preparing the target host with required dependencies
2. Building the application using Docker
3. Deploying the binary and assets via SSH
4. Configuring and starting the application

## Step 1: Prepare Target Host

### 1.1 Copy Setup Script to Target Host

First, copy the setup script to your target host:

```bash
# From your Mac, in the project directory
scp canon_test_cam/setup_host.sh user@target-host:~/
scp canon_test_cam/SYSTEM_REQUIREMENTS.md user@target-host:~/
```

### 1.2 Run Setup Script on Target Host

SSH into the target host and run the setup script:

```bash
# Connect to target host
ssh user@target-host

# Make script executable
chmod +x setup_host.sh

# Run setup script (requires sudo)
sudo ./setup_host.sh

# Review the generated report
cat ~/photobooth_setup_report.txt
```

The setup script will:
- Install all required system packages
- Configure USB rules for the Canon camera
- Set up V4L2 loopback for camera streaming
- Configure CUPS for printing
- Check for connected hardware
- Create necessary directories and permissions
- Generate environment configuration file

### 1.3 Install TurboPrint (Optional but Recommended)

If TurboPrint is not detected, install it manually:

```bash
# Download TurboPrint for ARM64
wget https://www.turboprint.de/downloads/turboprint-2.55-1.arm64.tgz

# Extract and install
tar -xzf turboprint-2.55-1.arm64.tgz
cd turboprint-2.55-1
sudo ./setup

# Configure printer
sudo tpconfig
```

**For detailed printer setup instructions, see [PRINTER_SETUP.md](PRINTER_SETUP.md)**

## Step 2: Configure Deployment Script

### 2.1 Edit deploy.sh Variables

Modify the deployment script to target your specific host:

```bash
# Edit deploy.sh and set these variables at the top:
PI_USER="your-target-username"           # Username on target host
PI_HOST="your-target-hostname"           # Hostname or IP address
SSH_KEY_PATH="$HOME/.ssh/your-key"      # Path to your SSH private key
BINARY_NAME="cam_test"                   # Keep as is
REMOTE_DEST_PATH="/home/${PI_USER}/cam_test"  # Deployment path
```

### 2.2 Example Configurations

#### For Local Network Raspberry Pi:
```bash
export PI_USER="pi"
export PI_HOST="192.168.1.100"
export SSH_KEY_PATH="$HOME/.ssh/id_rsa"
./deploy.sh
```

#### For Remote Server:
```bash
export PI_USER="photobooth"
export PI_HOST="photobooth.example.com"
export SSH_KEY_PATH="$HOME/.ssh/photobooth_key"
./deploy.sh
```

#### For Multiple Hosts:
Create wrapper scripts for each host:

```bash
# deploy_to_booth1.sh
#!/bin/bash
export PI_USER="booth1"
export PI_HOST="booth1.local"
export SSH_KEY_PATH="$HOME/.ssh/booth1_key"
./deploy.sh

# deploy_to_booth2.sh
#!/bin/bash
export PI_USER="booth2"
export PI_HOST="booth2.local"
export SSH_KEY_PATH="$HOME/.ssh/booth2_key"
./deploy.sh
```

## Step 3: Deploy Application

### 3.1 Build and Deploy

From your Mac, in the project root directory:

```bash
# Navigate to the project directory
cd canon_test_cam

# Run deployment (will build in Docker and copy to target)
./deploy.sh
```

The deployment script will:
1. Build the ARM64 binary using Docker
2. Extract the binary from the Docker container
3. Copy the binary to the target host via SCP
4. Copy operation scripts for kiosk mode
5. Set appropriate permissions

### 3.2 Verify Deployment

After deployment, the script will show commands to verify:

```bash
# Test if the binary was deployed
ssh -i ~/.ssh/your-key user@target-host "ls -la ~/cam_test"

# Check if operations scripts were copied
ssh -i ~/.ssh/your-key user@target-host "ls -la ~/operations/"
```

## Step 4: Configure and Run Application

### 4.1 Initial Test Run

SSH into the target host and test the application:

```bash
# Connect to target
ssh -i ~/.ssh/your-key user@target-host

# Load environment variables
source ~/.photobooth.env

# Run the application
./cam_test
```

### 4.2 Test Camera Connection

```bash
# Check if camera is detected
gphoto2 --auto-detect

# Test capture
gphoto2 --capture-preview

# Check video device
ls -la /dev/video*
```

### 4.3 Test Printer

```bash
# List available printers
lpstat -p

# Check if using TurboPrint driver
grep -i turboprint /etc/cups/ppd/*.ppd

# Test print
echo "Test Page" | lp -d "XP8700series-TurboPrint"
```

For comprehensive printer setup and troubleshooting, see [PRINTER_SETUP.md](PRINTER_SETUP.md)

## Step 5: Setup Kiosk Mode (Optional)

For unattended photo booth operation:

### 5.1 Install Kiosk Service

```bash
# On the target host
sudo ~/operations/setup-kiosk.sh
```

### 5.2 Start Kiosk Mode

```bash
# Start the service
sudo systemctl start photobooth-kiosk.service

# Enable auto-start on boot
sudo systemctl enable photobooth-kiosk.service

# Check status
sudo systemctl status photobooth-kiosk.service
```

### 5.3 Stop Kiosk Mode

```bash
sudo systemctl stop photobooth-kiosk.service
```

## Deployment Checklist

Before deployment:
- [ ] Target host running Debian Bookworm
- [ ] SSH access configured with key authentication
- [ ] Docker installed on development machine

On target host:
- [ ] Run setup_host.sh script
- [ ] Canon EOS Rebel T7 connected and detected
- [ ] Epson XP-8700 configured in CUPS
- [ ] TurboPrint 2.55-1 installed (optional)
- [ ] Printer using TurboPrint driver (verify with PRINTER_SETUP.md)
- [ ] Environment file created (~/.photobooth.env)

Deployment:
- [ ] Configure deploy.sh with target host details
- [ ] Run ./deploy.sh from development machine
- [ ] Verify binary deployed to target
- [ ] Test application startup

Post-deployment:
- [ ] Camera preview working (/stream endpoint)
- [ ] Photo capture working
- [ ] Printing functional
- [ ] Kiosk mode configured (if needed)

## Troubleshooting

### SSH Connection Issues
```bash
# Test SSH connection
ssh -v -i ~/.ssh/your-key user@target-host

# Fix permissions
chmod 600 ~/.ssh/your-key
chmod 700 ~/.ssh
```

### Camera Not Working
```bash
# On target host
# Kill interfering processes
sudo pkill -f gphoto2
sudo pkill -f PTPCamera

# Reload v4l2loopback
sudo modprobe -r v4l2loopback
sudo modprobe v4l2loopback exclusive_caps=1 max_buffers=2

# Test with gphoto2
gphoto2 --stdout --capture-movie | ffmpeg -i - -vcodec rawvideo -pix_fmt yuv420p -f v4l2 /dev/video0
```

### Application Won't Start
```bash
# Check for port conflicts
sudo lsof -i :8080

# Check logs
tail -f ~/photobooth.log

# Run with debug logging
RUST_LOG=debug ./cam_test
```

### Printer Not Found
```bash
# Check USB connection
lsusb | grep -i epson

# Restart CUPS
sudo systemctl restart cups

# Re-detect printers
sudo lpinfo -v

# Add printer with TurboPrint (if installed)
sudo tpconfig

# Or add manually with CUPS
sudo lpadmin -p "XP8700series-TurboPrint" -E -v "usb://EPSON/XP-8700" -m everywhere
```

See [PRINTER_SETUP.md](PRINTER_SETUP.md) for detailed printer configuration steps.

## Managing Multiple Deployments

### Using Configuration Files

Create a deployment configuration for each host:

```bash
# config/booth1.conf
PI_USER=booth1
PI_HOST=192.168.1.101
SSH_KEY_PATH=$HOME/.ssh/booth1_key
BINARY_NAME=cam_test
REMOTE_DEST_PATH=/home/booth1/cam_test

# config/booth2.conf
PI_USER=booth2
PI_HOST=192.168.1.102
SSH_KEY_PATH=$HOME/.ssh/booth2_key
BINARY_NAME=cam_test
REMOTE_DEST_PATH=/home/booth2/cam_test
```

Deploy using configuration:
```bash
# Load config and deploy
source config/booth1.conf && ./deploy.sh
```

### Batch Deployment Script

Create a script to deploy to multiple hosts:

```bash
#!/bin/bash
# deploy_all.sh

HOSTS=("booth1.local" "booth2.local" "booth3.local")
USER="photobooth"
KEY="$HOME/.ssh/photobooth_key"

for host in "${HOSTS[@]}"; do
    echo "Deploying to $host..."
    PI_USER="$USER" PI_HOST="$host" SSH_KEY_PATH="$KEY" ./deploy.sh
    
    if [ $? -eq 0 ]; then
        echo "✓ Successfully deployed to $host"
    else
        echo "✗ Failed to deploy to $host"
    fi
done
```

## Security Considerations

1. **SSH Keys**: Use unique SSH keys for each deployment
2. **Firewall**: Configure firewall on target hosts
   ```bash
   sudo ufw allow 8080/tcp  # Web interface
   sudo ufw allow 22/tcp    # SSH (consider changing port)
   sudo ufw enable
   ```
3. **User Permissions**: Run application as non-root user
4. **Network Isolation**: Consider network segmentation for kiosks

## Maintenance

### Updating the Application

```bash
# On development machine
git pull origin main
./deploy.sh  # Redeploy to target
```

### Backing Up Photos

```bash
# From development machine
rsync -avz user@target-host:/usr/local/share/photo_booth/captured/ ./backups/
```

### Monitoring

```bash
# Check application logs
ssh user@target-host "tail -f ~/photobooth.log"

# Check system resources
ssh user@target-host "htop"

# Check disk space
ssh user@target-host "df -h"
```

## Support

For issues specific to:
- **Camera**: Check gphoto2 documentation and camera USB connection
- **Printer**: See [PRINTER_SETUP.md](PRINTER_SETUP.md) for detailed configuration
- **Deployment**: Review Docker build logs and SSH connectivity
- **Application**: Check application logs and environment variables

Remember to test your deployment on a single host before deploying to multiple locations.
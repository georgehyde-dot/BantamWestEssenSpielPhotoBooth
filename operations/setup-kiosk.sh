#!/usr/bin/env bash
# One-time setup script to install the photo booth kiosk service

set -euo pipefail

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Photo Booth Kiosk - Setup${NC}"
echo "=========================="
echo ""

# Check if running as root or with sudo
if [ "$EUID" -ne 0 ]; then
   echo -e "${RED}This script needs sudo privileges${NC}"
   echo "Please run: sudo $0"
   exit 1
fi

# Service name and paths
SERVICE_NAME="photobooth-kiosk.service"
SERVICE_FILE="/home/prospero/operations/${SERVICE_NAME}"
SYSTEMD_PATH="/etc/systemd/system/${SERVICE_NAME}"
RUN_SCRIPT="/home/prospero/operations/run-kiosk.sh"

# Check if service file exists
if [ ! -f "${SERVICE_FILE}" ]; then
    echo -e "${RED}Error: Service file not found at ${SERVICE_FILE}${NC}"
    echo "Make sure you've deployed the operations directory"
    exit 1
fi

# Check if run script exists
if [ ! -f "${RUN_SCRIPT}" ]; then
    echo -e "${RED}Error: Run script not found at ${RUN_SCRIPT}${NC}"
    echo "Make sure you've deployed the operations directory"
    exit 1
fi

# Check if the photo booth binary exists
if [ ! -f "/home/prospero/cam_test" ]; then
    echo -e "${RED}Error: Photo booth binary not found at /home/prospero/cam_test${NC}"
    echo "Please deploy the application first using ./deploy.sh"
    exit 1
fi

# Make run script executable
echo "Making run script executable..."
chmod +x "${RUN_SCRIPT}"

# Copy service file to systemd
echo "Installing service file..."
cp "${SERVICE_FILE}" "${SYSTEMD_PATH}"

# Reload systemd daemon
echo "Reloading systemd daemon..."
systemctl daemon-reload

echo -e "${GREEN}✓ Service installed successfully!${NC}"
echo ""

# Ask if user wants to enable auto-start
read -p "Enable auto-start on boot? (y/N): " -n 1 -r
echo ""
if [[ $REPLY =~ ^[Yy]$ ]]; then
    systemctl enable ${SERVICE_NAME}
    echo -e "${GREEN}✓ Auto-start enabled${NC}"
else
    echo "Auto-start not enabled"
    echo "You can enable it later with: sudo systemctl enable ${SERVICE_NAME}"
fi

echo ""
echo -e "${GREEN}Setup complete!${NC}"
echo ""
echo "Commands:"
echo "  Start:   sudo systemctl start ${SERVICE_NAME}"
echo "  Stop:    sudo systemctl stop ${SERVICE_NAME}"
echo "  Status:  sudo systemctl status ${SERVICE_NAME}"
echo "  Logs:    sudo journalctl -u ${SERVICE_NAME} -f"

#!/usr/bin/env bash
# Start the photo booth in kiosk mode

set -euo pipefail

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Check if running as root or with sudo
if [ "$EUID" -ne 0 ]; then
   echo -e "${RED}This script needs sudo privileges${NC}"
   echo "Please run: sudo $0"
   exit 1
fi

# Service name
SERVICE_NAME="photobooth-kiosk.service"

# Start the service
echo "Starting photo booth kiosk service..."
systemctl start ${SERVICE_NAME}

# Check if it started successfully
sleep 2
if systemctl is-active --quiet ${SERVICE_NAME}; then
    echo -e "${GREEN}✓ Photo booth kiosk service started successfully!${NC}"
    echo ""
    echo "Commands:"
    echo "  Status:  sudo systemctl status ${SERVICE_NAME}"
    echo "  Logs:    sudo journalctl -u ${SERVICE_NAME} -f"
    echo "  Stop:    sudo systemctl stop ${SERVICE_NAME}"
else
    echo -e "${RED}✗ Failed to start the service${NC}"
    echo "Check logs with: sudo journalctl -u ${SERVICE_NAME} -n 50"
    exit 1
fi

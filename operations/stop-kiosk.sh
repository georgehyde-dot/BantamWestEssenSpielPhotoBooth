#!/usr/bin/env bash
# Stop the photo booth kiosk mode

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

# Stop the service
echo "Stopping photo booth kiosk service..."
systemctl stop ${SERVICE_NAME}

# Check if it stopped successfully
sleep 1
if ! systemctl is-active --quiet ${SERVICE_NAME}; then
    echo -e "${GREEN}✓ Photo booth kiosk service stopped successfully!${NC}"
else
    echo -e "${RED}✗ Service may still be running${NC}"
    echo "Check status with: sudo systemctl status ${SERVICE_NAME}"
    exit 1
fi

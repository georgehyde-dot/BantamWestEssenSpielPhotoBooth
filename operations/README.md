# Photo Booth Kiosk Mode

Simple systemd service to run the photo booth application with Chromium in kiosk mode on a Raspberry Pi.

## Quick Start

### First Time Setup
```bash
# Install the service (one time only)
sudo /home/prospero/operations/setup-kiosk.sh
```

### Daily Usage
```bash
# Start the kiosk
sudo systemctl start photobooth-kiosk.service

# Stop the kiosk
sudo systemctl stop photobooth-kiosk.service

# Check status
sudo systemctl status photobooth-kiosk.service

# View logs
sudo journalctl -u photobooth-kiosk.service -f
```

## Files

- `photobooth-kiosk.service` - Systemd service definition
- `run-kiosk.sh` - Wrapper script that runs both app and browser
- `setup-kiosk.sh` - One-time installation script
- `start-kiosk.sh` - Shortcut for `systemctl start`
- `stop-kiosk.sh` - Shortcut for `systemctl stop`

## How It Works

1. The service runs `run-kiosk.sh` which:
   - Starts the photo booth application (`/home/prospero/cam_test`)
   - Waits for it to be ready on port 8080
   - Launches Chromium in full-screen kiosk mode
   - Cleans up when either process exits

2. Application logs go to `/home/prospero/photobooth.log`

## Auto-Start on Boot

```bash
# Enable auto-start
sudo systemctl enable photobooth-kiosk.service

# Disable auto-start
sudo systemctl disable photobooth-kiosk.service
```

## Troubleshooting

### Check if service is running
```bash
sudo systemctl is-active photobooth-kiosk.service
```

### Check application logs
```bash
tail -f /home/prospero/photobooth.log
```

### Reset failed service
```bash
sudo systemctl reset-failed photobooth-kiosk.service
sudo systemctl start photobooth-kiosk.service
```

### Check if app is listening
```bash
curl http://localhost:8080/
```

## Requirements

- Chromium browser installed
- Display connected to Pi
- Photo booth binary at `/home/prospero/cam_test`

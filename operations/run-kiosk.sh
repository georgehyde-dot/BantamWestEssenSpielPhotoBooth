#!/usr/bin/env bash
# Simple wrapper script to run photo booth app and Chromium kiosk

# Cleanup function
cleanup() {
    echo "Stopping photo booth and browser..."
    pkill -f chromium || true
    pkill -f cam_test || true
    exit 0
}

# Trap signals to ensure cleanup
trap cleanup EXIT INT TERM

# Start the photo booth application
echo "Starting photo booth application..."
/home/prospero/cam_test > /home/prospero/photobooth.log 2>&1 &
APP_PID=$!

# Wait for the app to be ready
echo "Waiting for application to be ready..."
for i in {1..30}; do
    if curl -s http://localhost:8080/ > /dev/null 2>&1; then
        echo "Application is ready!"
        break
    fi
    sleep 1
done

# Detect chromium command (Debian 13+ uses 'chromium', older versions use 'chromium-browser')
if command -v chromium &> /dev/null; then
    CHROMIUM_CMD="chromium"
elif command -v chromium-browser &> /dev/null; then
    CHROMIUM_CMD="chromium-browser"
else
    echo "Error: Chromium browser not found!"
    echo "Install with: sudo apt-get install chromium"
    cleanup
    exit 1
fi

# Start Chromium in kiosk mode
echo "Starting Chromium in kiosk mode (using $CHROMIUM_CMD)..."
$CHROMIUM_CMD \
    --kiosk \
    --no-sandbox \
    --disable-setuid-sandbox \
    --noerrdialogs \
    --disable-infobars \
    --disable-session-crashed-bubble \
    --check-for-update-interval=31536000 \
    --disable-component-update \
    --autoplay-policy=no-user-gesture-required \
    --start-fullscreen \
    --window-position=0,0 \
    --disable-pinch \
    --overscroll-history-navigation=0 \
    --enable-features=VirtualKeyboard \
    --touch-events=enabled \
    --enable-touch-drag-drop \
    --enable-touch-editing \
    http://localhost:8080/ &
BROWSER_PID=$!

# Wait for either process to exit
wait -n $APP_PID $BROWSER_PID

# If we get here, something exited - clean up everything
cleanup

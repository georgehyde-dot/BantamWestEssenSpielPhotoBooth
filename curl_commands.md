# Photo Booth Curl Commands Reference

## Basic Health Checks

### Check if server is running
```bash
curl -I http://localhost:8080/
```

### Check preview stream headers
```bash
curl -I http://localhost:8080/preview
```

### Get a few bytes of preview stream to verify MJPEG
```bash
curl -s http://localhost:8080/preview | head -c 1000 | file -
```

## Image Capture

### Capture a photo
```bash
curl -X POST http://localhost:8080/capture
```

### Capture and save response to variable
```bash
RESPONSE=$(curl -s -X POST http://localhost:8080/capture)
echo $RESPONSE
FILENAME=$(echo $RESPONSE | jq -r '.file')
echo "Captured: $FILENAME"
```

### Capture without jq (using grep/sed)
```bash
RESPONSE=$(curl -s -X POST http://localhost:8080/capture)
FILENAME=$(echo $RESPONSE | grep -o '"file":"[^"]*"' | sed 's/"file":"\([^"]*\)"/\1/')
echo "Captured: $FILENAME"
```

## Viewing Images

### List captured images
```bash
curl http://localhost:8080/images/
```

### Download a specific image
```bash
# Replace cap_XXXXXXXXXX.png with actual filename
curl -O http://localhost:8080/images/cap_1234567890.png
```

### View photo page
```bash
# Replace with actual filename
curl http://localhost:8080/photo?file=cap_1234567890.png
```

## Printing

### Print a captured photo
```bash
# Replace with actual filename
curl -X POST http://localhost:8080/print \
  -H "Content-Type: application/json" \
  -d '{"filename":"cap_1234567890.png"}'
```

### Print with formatted output
```bash
curl -s -X POST http://localhost:8080/print \
  -H "Content-Type: application/json" \
  -d '{"filename":"cap_1234567890.png"}' | jq .
```

## Complete Workflow Example

### Capture and print in one command chain
```bash
# Capture image
RESPONSE=$(curl -s -X POST http://localhost:8080/capture)
FILENAME=$(echo $RESPONSE | grep -o '"file":"[^"]*"' | sed 's/"file":"\([^"]*\)"/\1/')

# Print the captured image
if [ -n "$FILENAME" ]; then
    echo "Captured: $FILENAME"
    curl -X POST http://localhost:8080/print \
      -H "Content-Type: application/json" \
      -d "{\"filename\":\"$FILENAME\"}"
else
    echo "Capture failed"
fi
```

## Testing Error Handling

### Test with invalid filename (security check)
```bash
curl -X POST http://localhost:8080/print \
  -H "Content-Type: application/json" \
  -d '{"filename":"../../../etc/passwd"}'
```

### Test with missing filename
```bash
curl -X POST http://localhost:8080/print \
  -H "Content-Type: application/json" \
  -d '{}'
```

### Test with non-existent file
```bash
curl -X POST http://localhost:8080/print \
  -H "Content-Type: application/json" \
  -d '{"filename":"does_not_exist.png"}'
```

## Monitoring

### Watch preview stream data (first 50 lines)
```bash
curl -N http://localhost:8080/preview 2>/dev/null | xxd | head -50
```

### Check server response time
```bash
curl -w "\nTotal time: %{time_total}s\n" -o /dev/null -s http://localhost:8080/
```

### Check all endpoints response codes
```bash
echo "Main page: $(curl -s -o /dev/null -w "%{http_code}" http://localhost:8080/)"
echo "Preview: $(curl -s -o /dev/null -w "%{http_code}" http://localhost:8080/preview)"
echo "Images: $(curl -s -o /dev/null -w "%{http_code}" http://localhost:8080/images/)"
```

## System Checks

### Check photo storage directory
```bash
ls -la /usr/local/share/photo_booth/
```

### Check recent captures
```bash
ls -lt /usr/local/share/photo_booth/*.png | head -10
```

### Check file permissions
```bash
stat -c "%a %n" /usr/local/share/photo_booth/*.png | head -5
```

### Monitor CUPS print queue
```bash
lpstat -o
```

### Check printer status
```bash
lpstat -p -d
```

## Debugging

### Follow application output (if running in foreground)
```bash
# In the terminal running the app, you'll see the output
```

### Check CUPS error log
```bash
sudo tail -f /var/log/cups/error_log
```

### Test network connectivity from another device
```bash
# From another computer on the network
curl http://<raspberry-pi-ip>:8080/
```

## Quick Test Script

Save this as `quick_test.sh`:

```bash
#!/bin/bash
echo "Testing photo booth..."
echo -n "Server check: "
curl -s -o /dev/null -w "%{http_code}\n" http://localhost:8080/

echo -n "Capturing image... "
RESPONSE=$(curl -s -X POST http://localhost:8080/capture)
if echo $RESPONSE | grep -q '"ok":true'; then
    echo "OK"
    FILENAME=$(echo $RESPONSE | grep -o '"file":"[^"]*"' | sed 's/"file":"\([^"]*\)"/\1/')
    echo "Captured: $FILENAME"
else
    echo "FAILED"
    echo $RESPONSE
fi
```

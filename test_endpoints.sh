#!/bin/bash

# Photo Booth API Test Script
# Run this on the Raspberry Pi to test endpoints without GUI

BASE_URL="http://localhost:8080"

echo "=== Photo Booth Endpoint Tests ==="
echo "Base URL: $BASE_URL"
echo

# Test 1: Check if server is running
echo "1. Testing main page (/)..."
if curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/" | grep -q "200"; then
    echo "   ✓ Main page is accessible"
else
    echo "   ✗ Main page failed"
    exit 1
fi
echo

# Test 2: Check preview stream headers
echo "2. Testing preview stream (/preview)..."
PREVIEW_HEADERS=$(curl -s -I -X GET "$BASE_URL/preview" | head -n 20)
if echo "$PREVIEW_HEADERS" | grep -q "multipart/x-mixed-replace"; then
    echo "   ✓ Preview stream is running (MJPEG stream detected)"
else
    echo "   ✗ Preview stream not working properly"
    echo "   Headers received:"
    echo "$PREVIEW_HEADERS"
fi
echo

# Test 3: Get a single frame from preview (first few bytes to confirm it's JPEG)
echo "3. Testing preview frame data..."
FRAME_DATA=$(curl -s "$BASE_URL/preview" | head -c 1000 | xxd -p | head -n 1)
if echo "$FRAME_DATA" | grep -q "ffd8ff"; then
    echo "   ✓ Preview is returning JPEG data (FFD8FF header found)"
else
    echo "   ✗ Preview data doesn't look like JPEG"
fi
echo

# Test 4: Capture an image
echo "4. Testing image capture (/capture)..."
CAPTURE_RESPONSE=$(curl -s -X POST "$BASE_URL/capture")
echo "   Response: $CAPTURE_RESPONSE"

# Extract filename from response
FILENAME=$(echo "$CAPTURE_RESPONSE" | grep -o '"file":"[^"]*"' | sed 's/"file":"\([^"]*\)"/\1/')
if [ -n "$FILENAME" ]; then
    echo "   ✓ Image captured successfully: $FILENAME"

    # Test 5: Check if captured image is accessible
    echo
    echo "5. Testing image access (/images/$FILENAME)..."
    IMAGE_STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/images/$FILENAME")
    if [ "$IMAGE_STATUS" = "200" ]; then
        echo "   ✓ Captured image is accessible"

        # Get image size
        IMAGE_SIZE=$(curl -s -I "$BASE_URL/images/$FILENAME" | grep -i "content-length" | awk '{print $2}' | tr -d '\r')
        echo "   Image size: $IMAGE_SIZE bytes"
    else
        echo "   ✗ Cannot access captured image (HTTP $IMAGE_STATUS)"
    fi

    # Test 6: Test photo page
    echo
    echo "6. Testing photo page (/photo?file=$FILENAME)..."
    if curl -s "$BASE_URL/photo?file=$FILENAME" | grep -q "Captured Image"; then
        echo "   ✓ Photo page loads correctly"
    else
        echo "   ✗ Photo page failed to load"
    fi

    # Test 7: Test print endpoint (without actually printing)
    echo
    echo "7. Testing print endpoint (/print)..."
    PRINT_RESPONSE=$(curl -s -X POST \
        -H "Content-Type: application/json" \
        -d "{\"filename\":\"$FILENAME\"}" \
        "$BASE_URL/print")
    echo "   Response: $PRINT_RESPONSE"

    if echo "$PRINT_RESPONSE" | grep -q '"ok":true'; then
        echo "   ✓ Print endpoint accepted the request"
        JOB_ID=$(echo "$PRINT_RESPONSE" | grep -o '"job_id":"[^"]*"' | sed 's/"job_id":"\([^"]*\)"/\1/')
        echo "   Print job ID: $JOB_ID"
    else
        echo "   ✗ Print endpoint failed"
    fi
else
    echo "   ✗ Image capture failed"
fi

echo
echo "=== Additional Tests ==="

# Test invalid filename for security
echo
echo "8. Testing filename validation (security check)..."
INVALID_RESPONSE=$(curl -s -X POST \
    -H "Content-Type: application/json" \
    -d '{"filename":"../../../etc/passwd"}' \
    "$BASE_URL/print")
if echo "$INVALID_RESPONSE" | grep -q "Invalid filename"; then
    echo "   ✓ Path traversal protection is working"
else
    echo "   ✗ WARNING: Path traversal might not be properly protected"
fi

# Test missing filename
echo
echo "9. Testing missing filename handling..."
MISSING_RESPONSE=$(curl -s -X POST \
    -H "Content-Type: application/json" \
    -d '{}' \
    "$BASE_URL/print")
if echo "$MISSING_RESPONSE" | grep -q "filename is required"; then
    echo "   ✓ Missing filename is properly handled"
else
    echo "   ✗ Missing filename not handled correctly"
fi

echo
echo "=== System Information ==="
echo
echo "10. Checking captured images directory..."
if [ -d "/usr/local/share/photo_booth" ]; then
    echo "   ✓ Photo booth directory exists"
    FILE_COUNT=$(ls -1 /usr/local/share/photo_booth/*.png 2>/dev/null | wc -l)
    echo "   Total PNG files: $FILE_COUNT"

    # Show last 3 captured files
    if [ $FILE_COUNT -gt 0 ]; then
        echo "   Recent captures:"
        ls -lt /usr/local/share/photo_booth/*.png 2>/dev/null | head -3 | awk '{print "     " $9 " (" $5 " bytes)"}'
    fi
else
    echo "   ✗ Photo booth directory not found"
fi

echo
echo "=== Test Summary ==="
echo "All endpoint tests completed. Check the output above for any failures."
echo
echo "To monitor the application logs, run:"
echo "  journalctl -u photo-booth -f    (if running as systemd service)"
echo "  or check the terminal where the app is running"
echo
echo "To continuously test the preview stream:"
echo "  curl -N $BASE_URL/preview | xxd | head -n 100"
echo
echo "To download a captured image:"
echo "  curl -O $BASE_URL/images/\$FILENAME"

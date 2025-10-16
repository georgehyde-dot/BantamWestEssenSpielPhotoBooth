#!/bin/bash

# Font Installation Script for Photo Booth
# Installs TTF fonts from the application's static directory to the system

set -e

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Photo Booth Font Installation${NC}"
echo "=============================="
echo ""

# Define paths
STATIC_DIR="/usr/local/share/photo_booth/static"
FONTS_DIR="/usr/local/share/fonts"
LOCAL_STATIC="$HOME/cam_test/static"

# Check which static directory exists
if [ -d "$STATIC_DIR" ]; then
    SOURCE_DIR="$STATIC_DIR"
    echo "Using fonts from: $STATIC_DIR"
elif [ -d "$LOCAL_STATIC" ]; then
    SOURCE_DIR="$LOCAL_STATIC"
    echo "Using fonts from: $LOCAL_STATIC"
else
    echo -e "${RED}✗${NC} No static directory found!"
    echo "Expected locations:"
    echo "  - $STATIC_DIR"
    echo "  - $LOCAL_STATIC"
    exit 1
fi

# Find TTF files
echo ""
echo "Searching for font files..."
FONT_FILES=$(find "$SOURCE_DIR" -name "*.ttf" -o -name "*.TTF" 2>/dev/null || true)

if [ -z "$FONT_FILES" ]; then
    echo -e "${YELLOW}⚠${NC} No TTF font files found in $SOURCE_DIR"
    echo "Please ensure font files are in the static directory"
    exit 0
fi

# Count fonts
FONT_COUNT=$(echo "$FONT_FILES" | wc -l)
echo -e "${GREEN}✓${NC} Found $FONT_COUNT font file(s):"
echo "$FONT_FILES" | while read font; do
    if [ -f "$font" ]; then
        basename "$font"
    fi
done

# Create fonts directory if it doesn't exist
echo ""
echo "Creating system fonts directory..."
sudo mkdir -p "$FONTS_DIR"

# Copy fonts
echo ""
echo "Installing fonts to system..."
echo "$FONT_FILES" | while read font; do
    if [ -f "$font" ]; then
        font_name=$(basename "$font")
        echo "  Installing: $font_name"
        sudo cp "$font" "$FONTS_DIR/"
        sudo chmod 644 "$FONTS_DIR/$font_name"
    fi
done

echo -e "${GREEN}✓${NC} Fonts copied to $FONTS_DIR"

# Update font cache
echo ""
echo "Updating font cache..."
if command -v fc-cache >/dev/null 2>&1; then
    sudo fc-cache -f -v "$FONTS_DIR" 2>&1 | grep -E "caching|succeeded" || true
    echo -e "${GREEN}✓${NC} Font cache updated"
else
    echo -e "${YELLOW}⚠${NC} fc-cache not found, fonts may not be immediately available"
    echo "Install fontconfig: sudo apt-get install fontconfig"
fi

# Verify installation
echo ""
echo "Verifying installation..."
INSTALLED_FONTS=$(ls "$FONTS_DIR"/*.ttf "$FONTS_DIR"/*.TTF 2>/dev/null | wc -l || echo "0")
echo -e "${GREEN}✓${NC} $INSTALLED_FONTS font(s) installed in $FONTS_DIR"

# List installed fonts
if command -v fc-list >/dev/null 2>&1; then
    echo ""
    echo "Installed fonts from this script:"
    echo "$FONT_FILES" | while read font; do
        if [ -f "$font" ]; then
            font_name=$(basename "$font" .ttf)
            font_name=$(basename "$font_name" .TTF)
            fc-list | grep -i "$font_name" | head -n1 || true
        fi
    done
fi

# Create font test HTML (optional)
TEST_HTML="/tmp/font_test.html"
cat > "$TEST_HTML" << 'EOF'
<!DOCTYPE html>
<html>
<head>
    <title>Font Test</title>
    <style>
        @font-face {
            font-family: 'CustomFont1';
            src: url('file:///usr/local/share/fonts/') format('truetype');
        }
        body {
            font-family: Arial, sans-serif;
            padding: 20px;
        }
        .test {
            font-size: 24px;
            margin: 10px 0;
            padding: 10px;
            border: 1px solid #ccc;
        }
    </style>
</head>
<body>
    <h1>Font Test Page</h1>
    <div class="test" style="font-family: Arial;">Default: The quick brown fox jumps over the lazy dog</div>
EOF

# Add each font to test page
echo "$FONT_FILES" | while read font; do
    if [ -f "$font" ]; then
        font_name=$(basename "$font" .ttf)
        font_name=$(basename "$font_name" .TTF)
        echo "    <div class=\"test\" style=\"font-family: '$font_name', Arial;\">$font_name: The quick brown fox jumps over the lazy dog</div>" >> "$TEST_HTML"
    fi
done

echo "</body></html>" >> "$TEST_HTML"

echo ""
echo -e "${BLUE}============================================${NC}"
echo -e "${BLUE}Font Installation Complete${NC}"
echo -e "${BLUE}============================================${NC}"
echo ""
echo "Fonts installed to: $FONTS_DIR"
echo "Test page created: $TEST_HTML"
echo ""
echo "To verify fonts are working:"
echo "  1. List system fonts: fc-list | grep -i ttf"
echo "  2. Open test page: firefox $TEST_HTML"
echo ""
echo "If fonts don't appear:"
echo "  1. Restart your application"
echo "  2. Clear font cache: sudo fc-cache -r"
echo "  3. Reboot the system (last resort)"

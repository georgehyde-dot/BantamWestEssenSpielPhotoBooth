#!/bin/bash

# Configure Printer for 4x6 Photo Printing
# This script automatically configures the EPSON XP-8700 for 4x6 borderless printing

set -e

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Automatic Printer Configuration for 4x6 Photos${NC}"
echo "=============================================="
echo ""

# Check if CUPS is running
if ! systemctl is-active --quiet cups; then
    echo -e "${YELLOW}Starting CUPS service...${NC}"
    sudo systemctl start cups
    sleep 2
fi

# Find EPSON printer
echo "Searching for EPSON printer..."
PRINTER_NAME=""
PRINTER_URI=""

# Check for USB connected EPSON printer
USB_PRINTER=$(sudo lpinfo -v 2>/dev/null | grep -i "usb.*epson" | head -n1)
if [ ! -z "$USB_PRINTER" ]; then
    PRINTER_URI=$(echo "$USB_PRINTER" | awk '{print $2}')
    echo -e "${GREEN}✓${NC} Found USB EPSON printer: $PRINTER_URI"
else
    echo -e "${YELLOW}⚠${NC} No USB EPSON printer detected"
    echo "Make sure printer is connected and powered on"
fi

# Check if printer is already configured
EXISTING_PRINTERS=$(lpstat -p 2>/dev/null | grep -i epson || true)
if [ ! -z "$EXISTING_PRINTERS" ]; then
    echo ""
    echo "Existing EPSON printers:"
    echo "$EXISTING_PRINTERS"

    # Get the first EPSON printer name
    PRINTER_NAME=$(lpstat -p 2>/dev/null | grep -i epson | head -n1 | awk '{print $2}')
    echo ""
    echo -e "${GREEN}✓${NC} Using existing printer: $PRINTER_NAME"
else
    # Add new printer if not exists
    if [ ! -z "$PRINTER_URI" ]; then
        PRINTER_NAME="EPSON_XP_8700_Series_USB"
        echo ""
        echo "Adding printer to CUPS..."

        # Try to add with generic driver first (usually works for basic printing)
        sudo lpadmin -p "$PRINTER_NAME" \
            -E \
            -v "$PRINTER_URI" \
            -m everywhere \
            -D "EPSON XP-8700 Series" \
            -L "Photo Booth Printer" 2>/dev/null || {
                # If that fails, try with raw driver
                echo -e "${YELLOW}Trying alternative driver...${NC}"
                sudo lpadmin -p "$PRINTER_NAME" \
                    -E \
                    -v "$PRINTER_URI" \
                    -m raw \
                    -D "EPSON XP-8700 Series" \
                    -L "Photo Booth Printer"
            }

        echo -e "${GREEN}✓${NC} Printer added: $PRINTER_NAME"
    else
        echo -e "${RED}✗${NC} Cannot add printer - no USB printer detected"
        exit 1
    fi
fi

# Configure printer for 4x6 borderless printing
if [ ! -z "$PRINTER_NAME" ]; then
    echo ""
    echo "Configuring printer for 4x6 borderless photos..."

    # Set as default printer (optional)
    # sudo lpadmin -d "$PRINTER_NAME"

    # Get available options
    echo "Checking available paper sizes..."
    PAPER_SIZES=$(lpoptions -p "$PRINTER_NAME" -l 2>/dev/null | grep -i "PageSize" || true)

    # Look for 4x6 paper size option
    if echo "$PAPER_SIZES" | grep -qi "4x6\|10x15cm\|postcard"; then
        # Try different 4x6 size names
        for size in "4x6" "4x6.bl" "4x6.Borderless" "10x15cm" "10x15cm.Borderless" "Postcard" "Postcard.Borderless"; do
            if lpoptions -p "$PRINTER_NAME" -l 2>/dev/null | grep -qi "PageSize.*$size"; then
                echo "  Setting paper size to: $size"
                sudo lpoptions -p "$PRINTER_NAME" -o PageSize=$size 2>/dev/null && break
            fi
        done
    else
        echo -e "${YELLOW}⚠${NC} 4x6 paper size not found, using default"
    fi

    # Set print quality options
    echo "Setting print quality options..."

    # Try to set photo quality
    sudo lpoptions -p "$PRINTER_NAME" -o PrintQuality=High 2>/dev/null || \
    sudo lpoptions -p "$PRINTER_NAME" -o Quality=Photo 2>/dev/null || \
    sudo lpoptions -p "$PRINTER_NAME" -o Resolution=600dpi 2>/dev/null || true

    # Try to set borderless mode
    sudo lpoptions -p "$PRINTER_NAME" -o BorderlessMode=True 2>/dev/null || \
    sudo lpoptions -p "$PRINTER_NAME" -o Borderless=True 2>/dev/null || \
    sudo lpoptions -p "$PRINTER_NAME" -o StpBorderless=True 2>/dev/null || true

    # Try to set media type to photo paper
    sudo lpoptions -p "$PRINTER_NAME" -o MediaType=PhotoPaper 2>/dev/null || \
    sudo lpoptions -p "$PRINTER_NAME" -o Media=PhotoPaper 2>/dev/null || \
    sudo lpoptions -p "$PRINTER_NAME" -o MediaType=Glossy 2>/dev/null || true

    # Set scaling to fit
    sudo lpoptions -p "$PRINTER_NAME" -o fit-to-page=True 2>/dev/null || true
    sudo lpoptions -p "$PRINTER_NAME" -o scaling=100 2>/dev/null || true

    echo -e "${GREEN}✓${NC} Printer configured"

    # Show current configuration
    echo ""
    echo "Current printer settings:"
    echo "========================="
    lpoptions -p "$PRINTER_NAME" | tr ' ' '\n' | grep -E "PageSize|Quality|Border|Media|Resolution" || true

    # Update environment file
    ENV_FILE="$HOME/.photobooth.env"
    echo ""
    echo "Updating environment configuration..."

    if [ -f "$ENV_FILE" ]; then
        # Update or add PRINTER_NAME
        if grep -q "PRINTER_NAME" "$ENV_FILE"; then
            sed -i "s|PRINTER_NAME=.*|PRINTER_NAME=$PRINTER_NAME|" "$ENV_FILE"
        else
            echo "export PRINTER_NAME=$PRINTER_NAME" >> "$ENV_FILE"
        fi
    else
        echo "export PRINTER_NAME=$PRINTER_NAME" > "$ENV_FILE"
    fi
    echo -e "${GREEN}✓${NC} Updated environment file with printer name"

    # Create a test print function
    echo ""
    echo "Testing printer configuration..."

    # Create a test image with ImageMagick (if available) or use existing
    if command -v convert >/dev/null 2>&1; then
        echo "Creating test image..."
        convert -size 1800x1200 xc:white \
            -fill black -pointsize 72 -gravity center \
            -annotate +0+0 "4x6 Test Print\n$(date)" \
            /tmp/test_4x6.jpg 2>/dev/null

        echo "Sending test print (4x6)..."
        lp -d "$PRINTER_NAME" -o PageSize=4x6 -o fit-to-page /tmp/test_4x6.jpg 2>/dev/null || \
        lp -d "$PRINTER_NAME" -o fit-to-page /tmp/test_4x6.jpg 2>/dev/null || \
        echo -e "${YELLOW}⚠${NC} Test print queued (check printer)"

        rm -f /tmp/test_4x6.jpg
    else
        echo "ImageMagick not installed, skipping test print"
    fi

    # Show print queue status
    echo ""
    echo "Print queue status:"
    lpstat -o "$PRINTER_NAME" 2>/dev/null || echo "Queue empty"

    echo ""
    echo -e "${BLUE}============================================${NC}"
    echo -e "${BLUE}Printer Configuration Complete${NC}"
    echo -e "${BLUE}============================================${NC}"
    echo ""
    echo "Printer Name: $PRINTER_NAME"
    echo "Configuration: 4x6 borderless photos"
    echo ""
    echo "To use in application:"
    echo "  export PRINTER_NAME=$PRINTER_NAME"
    echo "  ./cam_test"
    echo ""
    echo "To test printing manually:"
    echo "  echo 'Test' | lp -d $PRINTER_NAME"
    echo ""
    echo "To check printer status:"
    echo "  lpstat -p $PRINTER_NAME -t"

else
    echo -e "${RED}✗${NC} No printer configured"
    exit 1
fi

# Create helper script for 4x6 printing
HELPER_SCRIPT="$HOME/print_4x6.sh"
cat > "$HELPER_SCRIPT" << EOF
#!/bin/bash
# Helper script to print 4x6 photos
if [ -z "\$1" ]; then
    echo "Usage: \$0 <image_file>"
    exit 1
fi

lp -d "$PRINTER_NAME" \\
    -o PageSize=4x6 \\
    -o fit-to-page \\
    -o BorderlessMode=True \\
    -o MediaType=PhotoPaper \\
    -o PrintQuality=High \\
    "\$1"

echo "Print job sent for: \$1"
lpstat -o "$PRINTER_NAME"
EOF

chmod +x "$HELPER_SCRIPT"
echo ""
echo -e "${GREEN}✓${NC} Created helper script: $HELPER_SCRIPT"
echo "   Usage: ~/print_4x6.sh <image_file>"

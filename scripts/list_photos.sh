#!/bin/bash

# Script to list all captured photos and templates in the photo booth storage

echo "========================================="
echo "Photo Booth Storage Inspection"
echo "========================================="
echo

# Check for custom storage path or use default
STORAGE_PATH=${STORAGE_PATH:-"/usr/local/share/photo_booth"}
echo "Storage path: $STORAGE_PATH"
echo

# Check if directory exists
if [ ! -d "$STORAGE_PATH" ]; then
    echo "WARNING: Storage directory does not exist!"
    echo "Creating directory..."
    sudo mkdir -p "$STORAGE_PATH"
    sudo chown -R $USER:$USER "$STORAGE_PATH"
fi

# Count files
echo "File Statistics:"
echo "----------------"
CAPTURE_COUNT=$(find "$STORAGE_PATH" -name "cap_*.jpg" 2>/dev/null | wc -l)
PRINT_COUNT=$(find "$STORAGE_PATH" -name "print_*.png" 2>/dev/null | wc -l)
PREVIEW_COUNT=$(find "$STORAGE_PATH" -name "preview_*.png" 2>/dev/null | wc -l)

echo "Captured photos (cap_*.jpg): $CAPTURE_COUNT"
echo "Templated prints (print_*.png): $PRINT_COUNT"
echo "Preview images (preview_*.png): $PREVIEW_COUNT"
echo

# List captured photos
echo "Captured Photos (Raw):"
echo "----------------------"
if [ $CAPTURE_COUNT -gt 0 ]; then
    ls -lh "$STORAGE_PATH"/cap_*.jpg 2>/dev/null | tail -10
    if [ $CAPTURE_COUNT -gt 10 ]; then
        echo "... and $((CAPTURE_COUNT - 10)) more"
    fi
else
    echo "No captured photos found"
fi
echo

# List templated prints
echo "Templated Prints:"
echo "-----------------"
if [ $PRINT_COUNT -gt 0 ]; then
    ls -lh "$STORAGE_PATH"/print_*.png 2>/dev/null | tail -10
    if [ $PRINT_COUNT -gt 10 ]; then
        echo "... and $((PRINT_COUNT - 10)) more"
    fi
else
    echo "No templated prints found"
fi
echo

# List preview images
echo "Preview Images:"
echo "---------------"
if [ $PREVIEW_COUNT -gt 0 ]; then
    ls -lh "$STORAGE_PATH"/preview_*.png 2>/dev/null | tail -10
    if [ $PREVIEW_COUNT -gt 10 ]; then
        echo "... and $((PREVIEW_COUNT - 10)) more"
    fi
else
    echo "No preview images found"
fi
echo

# Show disk usage
echo "Storage Usage:"
echo "--------------"
if [ -d "$STORAGE_PATH" ]; then
    du -sh "$STORAGE_PATH" 2>/dev/null
    df -h "$STORAGE_PATH" 2>/dev/null | tail -1
fi
echo

# Check for other image files
echo "Other Image Files:"
echo "------------------"
OTHER_COUNT=$(find "$STORAGE_PATH" -type f \( -name "*.jpg" -o -name "*.jpeg" -o -name "*.png" \) ! -name "cap_*" ! -name "print_*" ! -name "preview_*" 2>/dev/null | wc -l)
if [ $OTHER_COUNT -gt 0 ]; then
    echo "Found $OTHER_COUNT other image files:"
    find "$STORAGE_PATH" -type f \( -name "*.jpg" -o -name "*.jpeg" -o -name "*.png" \) ! -name "cap_*" ! -name "print_*" ! -name "preview_*" 2>/dev/null | head -10
    if [ $OTHER_COUNT -gt 10 ]; then
        echo "... and $((OTHER_COUNT - 10)) more"
    fi
else
    echo "No other image files found"
fi
echo

# Show most recent files
echo "Most Recent Files (last 5):"
echo "----------------------------"
ls -lt "$STORAGE_PATH" 2>/dev/null | grep -E "\.(jpg|png)" | head -5
echo

# Check database for session records
echo "Database Session Info:"
echo "----------------------"
DB_PATH=${DATABASE_PATH:-"$STORAGE_PATH/photo_booth.db"}
if [ -f "$DB_PATH" ]; then
    echo "Database found at: $DB_PATH"
    echo "Recent sessions with photos:"
    sqlite3 "$DB_PATH" "SELECT id, photo_path, created_at FROM sessions WHERE photo_path IS NOT NULL ORDER BY created_at DESC LIMIT 5;" 2>/dev/null || echo "Could not read database"
else
    echo "Database not found at: $DB_PATH"
fi
echo

echo "========================================="
echo "End of Storage Inspection"
echo "========================================="

#!/bin/bash

# Fix Photo Booth Database Permissions
# This script fixes the permissions for the database and its directory

set -e

# Color codes for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Fixing Photo Booth Database Permissions${NC}"
echo "========================================"
echo ""

# Define paths
DB_DIR="/usr/local/share/photo_booth"
DB_FILE="${DB_DIR}/photo_booth.db"
CURRENT_USER=$(whoami)

echo "User: ${CURRENT_USER}"
echo "Database directory: ${DB_DIR}"
echo "Database file: ${DB_FILE}"
echo ""

# Check current permissions
echo "Current permissions:"
if [ -d "$DB_DIR" ]; then
    echo "Directory: $(ls -ld $DB_DIR)"
else
    echo "Directory does not exist"
fi

if [ -f "$DB_FILE" ]; then
    echo "Database: $(ls -l $DB_FILE)"
else
    echo "Database file does not exist"
fi
echo ""

# Fix directory permissions
echo "Fixing directory permissions..."
if [ ! -d "$DB_DIR" ]; then
    sudo mkdir -p "$DB_DIR"
    echo "Created directory: $DB_DIR"
fi

# Set ownership and permissions for directory
sudo chown ${CURRENT_USER}:${CURRENT_USER} "$DB_DIR"
sudo chmod 755 "$DB_DIR"
echo "Set directory ownership to ${CURRENT_USER}:${CURRENT_USER}"
echo "Set directory permissions to 755"

# Fix database file permissions
echo ""
echo "Fixing database file permissions..."
if [ ! -f "$DB_FILE" ]; then
    touch "$DB_FILE"
    echo "Created database file: $DB_FILE"
fi

# Set ownership and permissions for database file
sudo chown ${CURRENT_USER}:${CURRENT_USER} "$DB_FILE"
chmod 664 "$DB_FILE"
echo "Set database ownership to ${CURRENT_USER}:${CURRENT_USER}"
echo "Set database permissions to 664"

# Also ensure the subdirectories have proper permissions
for subdir in static captured previews; do
    subdir_path="${DB_DIR}/${subdir}"
    if [ -d "$subdir_path" ]; then
        sudo chown -R ${CURRENT_USER}:${CURRENT_USER} "$subdir_path"
        chmod 755 "$subdir_path"
        echo "Fixed permissions for: $subdir_path"
    fi
done

# Verify final permissions
echo ""
echo -e "${GREEN}Final permissions:${NC}"
echo "Directory: $(ls -ld $DB_DIR)"
echo "Database: $(ls -l $DB_FILE)"

# Check if SQLite can write to the database
echo ""
echo "Testing database write access..."
if sqlite3 "$DB_FILE" "CREATE TABLE IF NOT EXISTS test_permissions (id INTEGER); DROP TABLE IF EXISTS test_permissions;" 2>/dev/null; then
    echo -e "${GREEN}✓ Database is writable${NC}"
else
    echo -e "${YELLOW}⚠ Warning: Could not write to database${NC}"
    echo "You may need to check if another process has the database locked"
fi

echo ""
echo -e "${GREEN}Permissions fixed!${NC}"
echo "You can now run the photo booth application"

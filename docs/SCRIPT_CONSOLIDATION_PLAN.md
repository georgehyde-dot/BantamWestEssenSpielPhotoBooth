# Shell Script Consolidation Plan

## Current State Analysis

### 🔴 Major Redundancies Identified

1. **System Setup Scripts (3 overlapping scripts)**
   - `setup_packages.sh` - Base system setup (KEEP - Primary)
   - `setup_host.sh` - Full system + printer setup (REMOVE - Now setup_printer.sh)
   - Multiple package installation routines

2. **Printer Configuration Scripts (4 overlapping scripts)**
   - `setup_printer.sh` - DNP DS620 setup (KEEP - Primary)
   - `operations/printer_setup.sh` - DNP DS620 setup (REMOVE - Duplicate)
   - `configure_printer_4x6.sh` - Printer options (MERGE into setup_printer.sh)
   - `printer_utils.sh` - Printer utilities (REVIEW for useful functions)

3. **Font Installation Scripts (2 scripts)**
   - `install_fonts.sh` - Font installation (KEEP)
   - Font installation code in deploy.sh (KEEP in deploy)

4. **V4L2/Camera Scripts (3 scripts)**
   - `fix_v4l2_device.sh` - V4L2 fixes (KEEP)
   - `test_v4l2_loopback.sh` - V4L2 testing (KEEP)
   - V4L2 setup in setup_packages.sh (KEEP in setup_packages)

5. **Kiosk Mode Scripts (4 scripts in operations/)**
   - All kiosk scripts are focused and non-redundant (KEEP ALL)

## Proposed Consolidated Structure

### 📁 Root Level Scripts (Deployment & Primary Setup)

```
canon_test_cam/
├── deploy.sh                 # KEEP - Deployment script
├── setup_packages.sh         # KEEP - Base system setup
├── setup_printer.sh          # KEEP - DNP DS620 printer setup (focused)
├── check_setup.sh           # KEEP - System diagnostics
└── install_fonts.sh         # KEEP - Font installation
```

### 📁 operations/ (Runtime Operations)

```
operations/
├── setup-kiosk.sh           # KEEP - Kiosk mode setup
├── start-kiosk.sh           # KEEP - Start kiosk
├── stop-kiosk.sh            # KEEP - Stop kiosk
└── run-kiosk.sh             # KEEP - Run kiosk application
```

### 📁 troubleshooting/ (Diagnostic & Fix Scripts)

```
troubleshooting/
├── fix_v4l2_device.sh       # MOVE HERE - V4L2 fixes
├── test_v4l2_loopback.sh    # MOVE HERE - V4L2 testing
└── fix_db_permissions.sh    # MOVE HERE - Database fixes
```

### 📁 scripts/ (Application Runtime Scripts)

```
scripts/
└── list_photos.sh           # KEEP - Photo listing utility
```

## Action Plan

### Phase 1: Remove Duplicates
```bash
# Remove redundant printer setup
rm operations/printer_setup.sh

# Remove old printer config (after merging useful parts)
rm configure_printer_4x6.sh

# Remove printer utils if no unique functionality
rm printer_utils.sh
```

### Phase 2: Reorganize Scripts
```bash
# Create troubleshooting directory
mkdir -p troubleshooting

# Move diagnostic/fix scripts
mv fix_v4l2_device.sh troubleshooting/
mv test_v4l2_loopback.sh troubleshooting/
mv fix_db_permissions.sh troubleshooting/
```

### Phase 3: Update Scripts

#### 1. `setup_packages.sh` - Make it comprehensive for base system
- ✅ Keep all package installation
- ✅ Keep V4L2 loopback setup
- ✅ Keep udev rules
- ✅ Keep user permissions
- ❌ Remove any printer-specific setup
- ❌ Remove Epson/TurboPrint references

#### 2. `setup_printer.sh` - Focus only on DNP DS620
- ✅ Install gutenprint if needed
- ✅ Configure DNP DS620 printer
- ✅ Set default print options
- ✅ Test print capability
- ❌ Remove all system setup
- ❌ Remove camera setup
- ❌ Remove directory creation

#### 3. `check_setup.sh` - Comprehensive diagnostics
- ✅ Check all system components
- ✅ Check camera connection
- ✅ Check printer status
- ✅ Check V4L2 devices
- ✅ Generate status report

### Phase 4: Update deploy.sh

Update the deployment script to reference the new structure:

```bash
# Copy main setup scripts
scp setup_packages.sh user@host:~/
scp setup_printer.sh user@host:~/
scp check_setup.sh user@host:~/
scp install_fonts.sh user@host:~/

# Copy troubleshooting tools
scp -r troubleshooting/ user@host:~/

# Copy operations scripts
scp -r operations/ user@host:~/
```

## Script Purposes After Consolidation

| Script | Purpose | When to Run |
|--------|---------|-------------|
| `setup_packages.sh` | Install system packages, configure V4L2, set permissions | Once on new system |
| `setup_printer.sh` | Configure DNP DS620 printer with gutenprint | Once after system setup |
| `check_setup.sh` | Verify all components are working | After setup or for diagnostics |
| `install_fonts.sh` | Install custom fonts for photo booth | If using custom fonts |
| `troubleshooting/fix_v4l2_device.sh` | Fix V4L2 loopback issues | If camera preview not working |
| `troubleshooting/test_v4l2_loopback.sh` | Test V4L2 functionality | Debugging camera issues |
| `troubleshooting/fix_db_permissions.sh` | Fix database permissions | If database access fails |
| `operations/setup-kiosk.sh` | Configure kiosk mode | Once for kiosk setup |
| `operations/start-kiosk.sh` | Start kiosk mode | To run in kiosk mode |

## Benefits of Consolidation

1. **Reduced Redundancy**: From 16 scripts to ~12 focused scripts
2. **Clear Organization**: Scripts grouped by purpose
3. **Easier Maintenance**: No duplicate code to maintain
4. **Faster Setup**: No redundant operations
5. **Clear Documentation**: Each script has one clear purpose

## Migration Commands

```bash
# 1. Backup existing scripts
tar -czf scripts_backup_$(date +%Y%m%d).tar.gz *.sh operations/ scripts/

# 2. Create new structure
mkdir -p troubleshooting

# 3. Move and reorganize
mv fix_v4l2_device.sh troubleshooting/
mv test_v4l2_loopback.sh troubleshooting/
mv fix_db_permissions.sh troubleshooting/

# 4. Remove duplicates (after verifying backups)
rm operations/printer_setup.sh
rm configure_printer_4x6.sh
rm printer_utils.sh

# 5. Update setup_printer.sh (already done)
# 6. Update deploy.sh to match new structure
```

## Testing After Consolidation

1. **Fresh System Test**
   ```bash
   sudo ./setup_packages.sh
   sudo ./setup_printer.sh
   ./check_setup.sh
   ```

2. **Printer Test**
   ```bash
   echo "Test Page" | lp -d DNP_DS620_Photo
   ```

3. **Camera Test**
   ```bash
   gphoto2 --auto-detect
   ./troubleshooting/test_v4l2_loopback.sh
   ```

4. **Kiosk Test**
   ```bash
   sudo ./operations/setup-kiosk.sh
   sudo systemctl start photobooth-kiosk.service
   ```

# Photo Booth Camera Application

A Rust-based photo booth application designed for Raspberry Pi with Canon camera support, featuring live preview, capture, templating, and direct printing capabilities.

## Features

- **Live Camera Preview**: Real-time MJPEG stream from V4L2-compatible cameras
- **Photo Capture**: High-quality image capture with automatic file management
- **Template System**: Customizable print templates with text overlays and background images
- **Direct Printing**: CUPS integration with TurboPrint driver support for Epson printers
- **Web Interface**: Responsive web UI for camera control and photo management
- **Configuration**: Environment-based configuration for easy deployment

## Requirements

### Hardware
- Raspberry Pi (tested on Pi 4)
- V4L2-compatible camera (Canon DSLRs via gphoto2, webcams)
- Epson XP-8700 printer (or compatible)

### Software
- Linux (Raspberry Pi OS recommended)
- Rust 1.70+
- CUPS printing system
- TurboPrint driver (for borderless printing)
- V4L2 drivers for your camera

## Configuration

The application uses environment variables for configuration:

### Server Configuration
- `HOST`: Server bind address (default: `0.0.0.0`)
- `PORT`: Server port (default: `8080`)

### Camera Configuration
- `VIDEO_DEVICE`: V4L2 device path (default: `/dev/video0`)
- `VIDEO_WIDTH`: Capture width in pixels (default: `1920`)
- `VIDEO_HEIGHT`: Capture height in pixels (default: `1080`)
- `VIDEO_FORMAT`: Video format (default: `MJPG`)

### Storage Configuration
- `STORAGE_PATH`: Base path for photo storage (default: `/usr/local/share/photo_booth`)

### Printer Configuration
- `PRINTER_NAME`: Primary printer name (default: `XP8700series-TurboPrint`)
- `PRINTER_FALLBACK`: Comma-separated fallback printer names (default: `EPSON_XP_8700_Series_USB,XP-8700`)
- `USE_MOCK_PRINTER`: Use mock printer for testing (default: `false`)

### Template Configuration
- `TEMPLATE_HEADER`: Header text (default: `Photo Booth`)
- `TEMPLATE_NAME`: Name placeholder text (default: `NAME HERE`)
- `TEMPLATE_HEADLINE`: Headline placeholder text (default: `HEADLINE`)
- `TEMPLATE_STORY`: Story placeholder text (default: `STORY HERE`)
- `TEMPLATE_BACKGROUND`: Background image filename (default: `background.png`)

### Logging
- `RUST_LOG`: Log level (default: `info`, options: `error`, `warn`, `info`, `debug`, `trace`)

## Installation

### 1. Build for Raspberry Pi

Using the provided Docker build system:

```bash
./deploy.sh
```

This will:
- Build the ARM64 binary in Docker
- Deploy to your Raspberry Pi via SSH
- Set up the binary with correct permissions

### 2. Set up the storage directory

On the Raspberry Pi:

```bash
sudo mkdir -p /usr/local/share/photo_booth/static
sudo chmod 755 /usr/local/share/photo_booth
```

### 3. Add background image

Place your background image at:
```bash
/usr/local/share/photo_booth/static/background.png
```

### 4. Configure the printer

Ensure your printer is set up in CUPS:
```bash
lpstat -p  # List available printers
```

## Usage

### Running the Application

```bash
VIDEO_DEVICE=/dev/video0 VIDEO_WIDTH=1920 VIDEO_HEIGHT=1080 ./cam_test
```

### Web Interface

Navigate to `http://raspberry-pi-ip:8080` to access the photo booth interface.

## API Endpoints

### GET `/`
Main photo booth interface

### GET `/stream`
Live MJPEG camera preview stream

### POST `/capture`
Capture a photo from the camera

**Response:**
```json
{
  "ok": true,
  "path": "/images/cap_1234567890.png",
  "file": "cap_1234567890.png",
  "redirect": "/photo?file=cap_1234567890.png"
}
```

### GET `/photo`
Display captured photo page

**Query Parameters:**
- `file`: Filename of the captured photo

### POST `/print`
Print a photo with template

**Request:**
```json
{
  "filename": "cap_1234567890.png"
}
```

**Response:**
```json
{
  "ok": true,
  "job_id": "XP8700series-TurboPrint-123",
  "message": "Print job submitted successfully. Job ID: XP8700series-TurboPrint-123"
}
```

### POST `/preview`
Generate a preview of the templated print

**Request:**
```json
{
  "filename": "cap_1234567890.png"
}
```

**Response:**
```json
{
  "ok": true,
  "preview_url": "/images/preview_1234567890.png"
}
```

### GET `/images/*`
Serve captured photos and previews

### GET `/static/*`
Serve static assets (background images, etc.)

## Template System

The template system creates 4x6" prints at 300 DPI (1200x1800 pixels) with:

1. **Background**: Custom image or solid color
2. **Header Section**: Title text at top
3. **Photo**: Positioned 1/3 from top
4. **Text Sections**:
   - Name (large text below photo)
   - Headline (medium text)
   - Story section with semi-transparent overlay

### Customizing Templates

Templates can be customized by:
1. Changing environment variables for text
2. Replacing the background image
3. Modifying the `templates.rs` module for layout changes

## Architecture

The application follows clean architecture principles:

- **Configuration Module**: Centralized configuration management
- **Camera Module**: Abstracted camera interface
- **Printer Module**: Printer abstraction with CUPS implementation
- **Template Module**: Composable template system
- **Error Handling**: Type-safe error handling with `thiserror`
- **Logging**: Structured logging with `tracing`

## Development

### Running Tests

```bash
cargo test
```

### Building Locally

```bash
cargo build --release
```

### Cross-Compilation

The project includes a Dockerfile for cross-compilation to ARM64:

```bash
docker build -f canon_test_cam/Dockerfile -t cam-test-pi-builder .
```

## Troubleshooting

### Camera Not Found
- Check `VIDEO_DEVICE` environment variable
- Verify camera is connected: `ls -la /dev/video*`
- Check V4L2 compatibility: `v4l2-ctl --list-devices`

### Printer Issues
- Verify printer in CUPS: `lpstat -p`
- Check printer status: `lpstat -p PRINTER_NAME`
- Test print: `lp -d PRINTER_NAME test.png`

### Permission Errors
- Ensure user has access to video devices: `sudo usermod -a -G video $USER`
- Check storage directory permissions

## License

[Your License Here]

## Contributing

Contributions are welcome! Please follow Rust best practices and include tests for new features.
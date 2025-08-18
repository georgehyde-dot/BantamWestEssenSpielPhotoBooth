# Canon Photo Booth Application

A web-based photo booth application designed for Raspberry Pi with Canon EOS camera support and automatic printing capabilities.

## Features

- Live camera preview via MJPEG streaming
- Web-based interface accessible from any device on the network
- 3-second countdown timer before capture
- Automatic photo printing to Epson printers via CUPS
- Image capture at 1920x1080 resolution
- PNG format output for better print quality

## Requirements

- Raspberry Pi running Linux
- Canon EOS camera (tested with T7) connected via USB
- Epson printer (tested with XP-8700) configured in CUPS
- Rust toolchain for compilation

## Hardware Setup

1. Connect Canon EOS camera to Raspberry Pi via USB
2. Set camera to appropriate capture mode
3. Connect and configure Epson printer via USB or network
4. Ensure printer is configured in CUPS with name `EPSON_XP_8700_Series_USB`

## Software Dependencies

The application uses the following Rust crates:
- `actix-web` - Web framework
- `v4l` - Video4Linux interface for camera access
- `image` - Image processing and format conversion
- `printers` - CUPS integration for printing

## Configuration

### Environment Variables

- `VIDEO_DEVICE` - Camera device path (default: `/dev/video1`)
- `VIDEO_WIDTH` - Capture width (default: `1920`)
- `VIDEO_HEIGHT` - Capture height (default: `1080`)

### File Storage

Captured images are stored in `/usr/local/share/photo_booth/` with appropriate permissions for CUPS access.

## Building and Running

1. Clone the repository
2. Build the application:
   ```bash
   cargo build --release
   ```
3. Run the application:
   ```bash
   sudo cargo run --release
   ```

The web interface will be available at `http://<raspberry-pi-ip>:8080`

## Usage

1. Navigate to the web interface
2. The live camera preview will be displayed
3. Click "Start Countdown" to begin the 3-second timer
4. After capture, you'll be redirected to the photo page
5. From the photo page you can:
   - Download the image
   - Copy the image URL
   - Print the photo (4x6 borderless format)
   - Return to start for another capture

## Architecture

### Main Components

- **Camera Module**: Uses V4L2 to interface with the Canon camera, capturing MJPEG frames
- **Web Server**: Actix-web server provides HTTP endpoints for preview, capture, and printing
- **Image Processing**: Converts JPEG captures to PNG format for better print quality
- **Printer Interface**: CUPS integration for direct printing to Epson printers

### API Endpoints

- `GET /` - Main interface with live preview
- `GET /preview` - MJPEG stream endpoint
- `POST /capture` - Captures current frame and saves as PNG
- `GET /photo?file=<filename>` - Displays captured photo
- `POST /print` - Sends photo to printer
- `GET /images/<filename>` - Static file serving for captured images

### Print Settings

Photos are printed with the following settings:
- Paper Size: 4x6 Borderless
- Input Slot: Photo
- Media Type: Photographic Semi-Gloss
- Quality: High

## Code Overview

The application is organized in a modular structure with separate concerns:
- `main.rs` - Application entry point and web server
- `lib.rs` - Library module organization
- `printers.rs` - All printer-related functionality
- `camera.rs` - Camera interface and V4L2 implementation
- HTML files embedded at compile time for the web interface

The application has been cleaned up to remove unnecessary debug logging while maintaining essential functionality:

### Key Components

1. **Main Application (`main.rs`)**
   - Web server setup using Actix-web
   - Camera interface using V4L2
   - Image capture and format conversion
   - HTTP endpoint handlers

2. **Library Module (`lib.rs`)**
   - Module organization and re-exports
   - Public API for library usage

3. **Printer Module (`printers.rs`)**
   - `EpsonPrinter` - CUPS integration for Epson printers
   - `MockPrinter` - Fallback when no printer is available
   - Print job configuration with proper paper settings
   - `Printer` trait for abstraction
   - Platform-specific implementations

4. **Camera Module (`camera.rs`)**
   - `Camera` - Main camera interface
   - `CameraConfig` - Camera configuration from environment variables
   - V4L2 device management
   - MJPEG streaming support (userptr mode)
   - Frame capture and buffering
   - Platform-specific stubs for non-Linux systems

5. **Web Endpoints**
   - `/` - Main interface with embedded HTML
   - `/preview` - Live MJPEG stream
   - `/capture` - Photo capture endpoint
   - `/photo` - Photo display page
   - `/print` - Print submission endpoint
   - `/images/*` - Static file serving

### Key Functions

- `preview_loop()` - Manages camera streaming
- `capture_image()` - Captures and saves photos
- `print_photo()` - Handles print job submission
- `new_printer()` - Factory function for printer instances (in `printers.rs`)
- `Camera::start_preview_stream()` - Starts continuous MJPEG streaming (in `camera.rs`)
- `Camera::capture_frame()` - Gets the latest captured frame (in `camera.rs`)

### File Storage

Photos are stored in `/usr/local/share/photo_booth/` with:
- PNG format for better print quality
- 644 permissions for CUPS access
- Timestamp-based naming

### Error Handling

The application includes proper error handling for:
- Camera connection issues
- Printer availability
- File permissions
- Image format validation

## Troubleshooting

### Camera Not Found
- Check camera connection and power
- Verify device path with `ls /dev/video*`
- Ensure camera is in appropriate mode

### Printing Issues
- Verify printer name matches in CUPS: `lpstat -p`
- Check CUPS permissions for the application user
- Ensure photo paper is loaded in the photo tray
- Review CUPS logs: `/var/log/cups/error_log`

### Permission Issues
- The application creates necessary directories with appropriate permissions
- Files are saved with 644 permissions for CUPS access
- Directory permissions are set to 755

## Security Considerations

- File uploads are not supported (capture only)
- Filenames are validated to prevent directory traversal
- Web interface binds to all interfaces (0.0.0.0) - consider firewall rules for production use

## License

[Add your license information here]

## Contributing

[Add contribution guidelines if applicable]
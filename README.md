# Rust Photo Booth

A production-ready photo booth application built in Rust for Raspberry Pi, featuring Canon DSLR camera control, automated printing, and web-based interface.

## Background

I created this photo booth as a project for friends who run a board game company and needed an interactive booth for conventions. The goal was to build a reliable, self-contained system that could capture high-quality photos, add custom branding/templates, and print on-site without requiring constant supervision.

### Why I Chose Rust

After experimenting with several languages and approaches:

- **Python**: Initial prototype was too slow for real-time camera preview and had reliability issues with long-running processes
- **C++**: Better performance but memory management complexity and dependency hell made it difficult to maintain
- **Go**: Good middle ground but lacked mature libraries for camera control and image processing at the time

I settled on Rust for several key reasons:
- **Memory Safety**: No memory leaks or segfaults during long convention days
- **Performance**: Native performance for real-time camera streaming and image processing
- **Reliability**: The type system catches many issues at compile time
- **Cross-compilation**: Easy to build for ARM targets from my development machine
- **Ecosystem**: Great libraries for web servers (actix-web), image processing, and async operations

## Development

### Development Journey

The project evolved through several phases to reach production readiness:

#### Phase 1: Camera Control
Started with basic USB camera support, then moved to Canon DSLR control via gphoto2 for professional image quality. The Canon EOS Rebel T7 was chosen for its:
- Excellent image quality at a reasonable price point
- Reliable USB tethering support
- Fast autofocus and capture times
- Good low-light performance for indoor venues

#### Phase 2: Preview System
Implementing real-time preview was challenging. The solution uses:
- **v4l2loopback**: Virtual video device for streaming
- **gphoto2**: Captures live view from Canon camera
- **FFmpeg**: Pipes the stream to the loopback device
- **MJPEG streaming**: Serves preview to web browsers

Key timing considerations:
- Stop preview → Wait 500ms → Capture → Resume preview
- Process cleanup between operations to prevent "device busy" errors
- Careful process group management to ensure child processes are properly terminated

#### Phase 3: Printer Integration
Selected the EPSON XP-8700 printer with TurboPrint driver for:
- Borderless 4x6" photo printing
- Fast print speeds (under 30 seconds per photo)
- Reliable CUPS integration
- Good color reproduction

#### Phase 4: Remote Deployment
Implemented Tailscale-based deployment for:
- Secure remote access to production devices
- Easy updates without physical access
- Remote troubleshooting during events
- Network-independent connectivity

#### Phase 5: Frontend Evolution
The web interface went through multiple iterations:
1. Basic HTML forms → Interactive wizard flow
2. Added session management for multi-step process
3. Integrated AI story generation for personalized prints
4. Responsive design for tablet/phone operation

### Technical Highlights

#### Camera Timing & Reliability
The most critical aspect was getting camera timing right:
```rust
// Stop preview before capture
self.stop_preview().await?;
// Critical delay for camera state transition
tokio::time::sleep(Duration::from_millis(500)).await;
// Capture photo
let jpeg_data = self.capture_photo(output_path).await?;
// Resume preview after capture
self.start_preview_stream().await?;
```

#### v4l2loopback Setup
The system uses a virtual video device for preview:
```bash
# Load v4l2loopback module
sudo modprobe v4l2loopback devices=1 video_nr=10 card_label="Photo Booth" exclusive_caps=1
# Stream from camera to loopback
gphoto2 --stdout --capture-movie | ffmpeg -i - -vcodec rawvideo -pix_fmt yuv420p -f v4l2 /dev/video10
```

#### Process Management
Robust process cleanup prevents camera lockups:
- Uses process groups for managing child processes
- SIGTERM followed by SIGKILL for graceful shutdown
- Explicit cleanup in Drop implementations

## Deployment

### Deployment System Overview

The deployment system uses Docker for cross-compilation and SSH for distribution:

1. **Build Phase**: Docker container cross-compiles for ARM64
2. **Distribution**: Binary and assets deployed via SCP
3. **Configuration**: Environment-based configuration for different venues
4. **Monitoring**: Tailscale for remote access and troubleshooting

### Docker Build System

The Dockerfile implements a multi-stage build:
```dockerfile
# Build stage: Debian Bookworm with cross-compilation toolchain
FROM debian:bookworm AS builder
# Install Rust and ARM64 cross-compilation tools
# Build for aarch64-unknown-linux-gnu target
# Output minimal binary artifact
```

Key features:
- Debian Bookworm base for compatibility with Raspberry Pi OS
- Cross-compilation from x86_64 to ARM64
- Dependency caching for fast rebuilds
- Minimal final artifact size

### Deployment Script

The `deploy.sh` script handles the complete deployment:

```bash
./deploy.sh [environment] [deploy_all]
# Examples:
./deploy.sh dev        # Deploy to development Pi
./deploy.sh prod       # Deploy to production Pi
./deploy.sh dev true   # Deploy with all setup scripts
```

Features:
- Environment-specific deployment (dev/prod)
- Intelligent file change detection (only copies modified files)
- Setup script distribution
- Database initialization
- Font installation
- Permission management

### Setup Scripts

The deployment includes several setup scripts:

- **setup_packages.sh**: Installs system dependencies (gphoto2, ffmpeg, CUPS, etc.)
- **setup_printer.sh**: Configures CUPS and TurboPrint driver
- **configure_printer_4x6.sh**: Sets printer defaults for photo printing
- **check_setup.sh**: Diagnostic script for troubleshooting
- **install_fonts.sh**: Installs custom fonts for template rendering

### Production Configuration

Environment variables for production:
```bash
# Server
HOST=0.0.0.0
PORT=8080

# Camera
V4L2_LOOPBACK_DEVICE=/dev/video10

# Storage
STORAGE_PATH=/usr/local/share/photo_booth
DATABASE_URL=sqlite:///usr/local/share/photo_booth/photo_booth.db

# Printer
PRINTER_NAME=XP8700series-TurboPrint
PRINTER_FALLBACK=EPSON_XP_8700_Series_USB,XP-8700

# Logging
RUST_LOG=info
```

### Troubleshooting

Common issues and solutions:

#### Camera Issues
- **Device Busy**: Increase delay after stopping preview
- **No Camera Found**: Check USB connection and `gphoto2 --auto-detect`
- **Preview Frozen**: Restart v4l2loopback module

#### Printer Issues
- **Jobs Stuck**: Check CUPS queue with `lpstat -o`
- **Wrong Size**: Verify media settings in CUPS
- **No Borderless**: Ensure TurboPrint driver is active

#### Deployment Issues
- **Permission Denied**: Check SSH key permissions
- **Build Fails**: Ensure Docker has enough memory
- **Binary Won't Run**: Verify ARM64 architecture match

### Monitoring & Maintenance

Production monitoring setup:
- Tailscale for secure remote access
- SystemD service for automatic startup
- Log rotation for long-running deployments
- Database backups before events

## Project Structure

```
canon_test_cam/
├── src/
│   ├── main.rs              # Application entry point
│   ├── gphoto_camera.rs     # Canon camera control
│   ├── routes/              # HTTP endpoint handlers
│   ├── templates.rs         # Print template generation
│   ├── config.rs            # Configuration management
│   └── printers/            # Printer abstraction
├── migrations/              # Database schema migrations
├── static/                  # Frontend assets
├── deploy.sh               # Deployment script
├── Dockerfile              # Cross-compilation container
└── operations/             # Setup and maintenance scripts
```

## Acknowledgments

Special thanks to:
- The Bantam team for trusting me with their convention booth needs
- The Rust community for excellent libraries and documentation
- The gphoto2 project for reliable camera control
- Everyone who tested the booth at conventions and provided feedback

## Online Demo

An online version (without camera/printer functionality) is available for testing the user flow and template system. This helped iterate on the UI before deploying to hardware.

## Future Improvements

- [ ] Multiple camera support for different angles
- [ ] Cloud backup of photos
- [ ] QR code for digital photo delivery
- [ ] More template customization options
- [ ] Analytics dashboard for event organizers

## License

MIT License - See LICENSE file for details

## Contributing

Contributions are welcome! The project follows Rust best practices as outlined in "Zero to Production in Rust". Please ensure:
- All tests pass
- Code follows Rust idioms
- Changes are tested on actual hardware
- Documentation is updated

For questions or issues, please open a GitHub issue or reach out via the project repository.
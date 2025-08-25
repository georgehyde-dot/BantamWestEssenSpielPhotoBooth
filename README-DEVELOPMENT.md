# Photo Booth Development Environment

This project is designed to run on Linux systems. To facilitate development on macOS (or other platforms), we provide a Docker-based development environment that mirrors the production Linux environment.

## Prerequisites

- Docker Desktop installed and running
- Docker Compose (usually included with Docker Desktop)
- Git

## Quick Start

```bash
# Clone the repository
git clone <repository-url>
cd canon_test_cam

# Make the development script executable
chmod +x dev.sh

# Start the development environment
./dev.sh up

# The application will be available at http://localhost:8080
```

## Development Workflow

### Using the Helper Script

The `dev.sh` script provides convenient commands for development:

```bash
# Start development environment with hot reload
./dev.sh up

# View logs
./dev.sh logs

# Run tests
./dev.sh test

# Run cargo commands
./dev.sh cargo build
./dev.sh cargo check

# Enter container shell for debugging
./dev.sh shell

# Format code
./dev.sh fmt

# Run linter
./dev.sh clippy

# Stop environment
./dev.sh down
```

### Hot Reload Development

The development container includes `cargo-watch` for automatic rebuilding:

```bash
# Start with hot reload (default when using ./dev.sh up)
./dev.sh watch
```

Edit files on your host machine, and the application will automatically rebuild and restart.

### Manual Development

If you prefer to work directly in the container:

```bash
# Enter the container
./dev.sh shell

# Now you're in the Linux environment
cargo build
cargo run
cargo test
```

## Container Architecture

### Development Image (`Dockerfile.dev`)

- Based on Rust official image with Debian Bookworm
- Includes all Linux dependencies:
  - V4L2 for camera support
  - CUPS for printing
  - SQLite for database
  - Image processing libraries
  - Development tools

### Volume Mounts

- Source code: `.:/workspace` (for hot reload)
- Application data: `photo_booth_data:/usr/local/share/photo_booth`

### Environment Variables

The container is pre-configured with development defaults:

- `RUST_LOG=debug` - Verbose logging
- `USE_MOCK_PRINTER=true` - Mock printer for development
- `HOST=0.0.0.0` - Listen on all interfaces
- `PORT=8080` - Web server port

## Camera Development

### Without Physical Camera

The default configuration uses mock devices. The application will work without a physical camera attached.

### With USB Camera

To use a physical USB camera:

1. Uncomment the device mapping in `docker-compose.dev.yml`:
   ```yaml
   devices:
     - /dev/video0:/dev/video0
   ```

2. Ensure your camera is connected before starting the container

3. You may need to run with elevated permissions:
   ```bash
   # Edit docker-compose.dev.yml and uncomment:
   # privileged: true
   ```

## Database Development

The SQLite database is automatically created and migrated on first run.

To manually run migrations:
```bash
./dev.sh init-db
```

Database file location: `/usr/local/share/photo_booth/photo_booth.db`

## Troubleshooting

### Container won't start

```bash
# Check Docker is running
docker info

# Clean up and rebuild
./dev.sh down
docker-compose -f docker-compose.dev.yml build --no-cache
./dev.sh up
```

### Permission issues

```bash
# Reset the environment including volumes
./dev.sh reset

# Rebuild
./dev.sh build
```

### Camera not detected

1. Check camera is connected: `ls -la /dev/video*`
2. Ensure device mapping is uncommented in docker-compose.dev.yml
3. May need to run container in privileged mode

### Port already in use

Change the port mapping in `docker-compose.dev.yml`:
```yaml
ports:
  - "8081:8080"  # Change 8081 to any free port
```

## Production Build

To build for production (ARM64/Raspberry Pi):

```bash
# Use the production Dockerfile
docker build -f Dockerfile -t photo-booth-arm64 .

# Extract the binary
docker create --name temp photo-booth-arm64
docker cp temp:/cam_test ./canon_test_cam
docker rm temp
```

## Additional Resources

- [Docker Documentation](https://docs.docker.com/)
- [Rust in Docker](https://hub.docker.com/_/rust)
- [V4L2 Documentation](https://www.kernel.org/doc/html/v4.9/media/uapi/v4l/v4l2.html)

## Notes

- All development happens in the Linux container environment
- The container includes all necessary Linux-specific dependencies
- Changes to Dockerfile.dev require rebuilding: `./dev.sh build`
- The development environment closely mirrors production Raspberry Pi setup
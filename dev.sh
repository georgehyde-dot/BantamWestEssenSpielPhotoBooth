#!/bin/bash

# Photo Booth Development Helper Script
# This script helps manage the Docker-based development environment

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
PROJECT_NAME="photo_booth"
DEV_CONTAINER_NAME="${PROJECT_NAME}_dev"
COMPOSE_FILE="docker-compose.dev.yml"

# Helper functions
print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

print_error() {
    echo -e "${RED}✗ $1${NC}"
}

print_info() {
    echo -e "${YELLOW}→ $1${NC}"
}

# Check if Docker is running
check_docker() {
    if ! docker info > /dev/null 2>&1; then
        print_error "Docker is not running. Please start Docker first."
        exit 1
    fi
    print_success "Docker is running"
}

# Build development image
build() {
    print_info "Building development image..."
    docker-compose -f $COMPOSE_FILE build
    print_success "Development image built successfully"
}

# Start development environment
up() {
    print_info "Starting development environment..."
    docker-compose -f $COMPOSE_FILE up -d
    print_success "Development environment started"
    print_info "Application available at http://localhost:8080"
    print_info "Logs: ./dev.sh logs"
}

# Stop development environment
down() {
    print_info "Stopping development environment..."
    docker-compose -f $COMPOSE_FILE down
    print_success "Development environment stopped"
}

# Restart development environment
restart() {
    down
    up
}

# Show logs
logs() {
    docker-compose -f $COMPOSE_FILE logs -f
}

# Run cargo command in container
cargo() {
    print_info "Running cargo $@..."
    docker-compose -f $COMPOSE_FILE exec photo-booth-dev cargo "$@"
}

# Run tests
test() {
    print_info "Running tests..."
    docker-compose -f $COMPOSE_FILE exec photo-booth-dev cargo test "$@"
}

# Run clippy
clippy() {
    print_info "Running clippy..."
    docker-compose -f $COMPOSE_FILE exec photo-booth-dev cargo clippy -- -D warnings
}

# Format code
fmt() {
    print_info "Formatting code..."
    docker-compose -f $COMPOSE_FILE exec photo-booth-dev cargo fmt
}

# Check code (format + clippy + test)
check() {
    fmt
    clippy
    test
}

# Enter shell in container
shell() {
    print_info "Entering container shell..."
    docker-compose -f $COMPOSE_FILE exec photo-booth-dev /bin/bash
}

# Run the application with live reload
watch() {
    print_info "Starting with live reload..."
    docker-compose -f $COMPOSE_FILE exec photo-booth-dev cargo watch -x run
}

# Clean build artifacts
clean() {
    print_info "Cleaning build artifacts..."
    docker-compose -f $COMPOSE_FILE exec photo-booth-dev cargo clean
    print_success "Build artifacts cleaned"
}

# Reset everything (including volumes)
reset() {
    print_info "Resetting development environment..."
    docker-compose -f $COMPOSE_FILE down -v
    print_success "Development environment reset"
}

# Initialize database
init-db() {
    print_info "Initializing database..."
    docker-compose -f $COMPOSE_FILE exec photo-booth-dev cargo sqlx migrate run
    print_success "Database initialized"
}

# Show help
help() {
    echo "Photo Booth Development Helper"
    echo ""
    echo "Usage: ./dev.sh [command]"
    echo ""
    echo "Commands:"
    echo "  build       Build the development Docker image"
    echo "  up          Start the development environment"
    echo "  down        Stop the development environment"
    echo "  restart     Restart the development environment"
    echo "  logs        Show application logs (follow mode)"
    echo "  cargo       Run cargo command in container"
    echo "  test        Run tests"
    echo "  clippy      Run clippy linter"
    echo "  fmt         Format code"
    echo "  check       Run fmt, clippy, and tests"
    echo "  shell       Enter container shell"
    echo "  watch       Run with live reload"
    echo "  clean       Clean build artifacts"
    echo "  reset       Reset environment (including data)"
    echo "  init-db     Initialize/migrate database"
    echo "  help        Show this help message"
    echo ""
    echo "Examples:"
    echo "  ./dev.sh up                    # Start development environment"
    echo "  ./dev.sh cargo build           # Build the project"
    echo "  ./dev.sh test                  # Run all tests"
    echo "  ./dev.sh shell                 # Enter container for debugging"
}

# Main script logic
check_docker

case "$1" in
    build)
        build
        ;;
    up)
        build
        up
        ;;
    down)
        down
        ;;
    restart)
        restart
        ;;
    logs)
        logs
        ;;
    cargo)
        shift
        cargo "$@"
        ;;
    test)
        shift
        test "$@"
        ;;
    clippy)
        clippy
        ;;
    fmt)
        fmt
        ;;
    check)
        check
        ;;
    shell)
        shell
        ;;
    watch)
        watch
        ;;
    clean)
        clean
        ;;
    reset)
        reset
        ;;
    init-db)
        init-db
        ;;
    help|"")
        help
        ;;
    *)
        print_error "Unknown command: $1"
        echo ""
        help
        exit 1
        ;;
esac

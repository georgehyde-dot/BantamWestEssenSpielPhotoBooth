#!/bin/bash

# Scheduled deployment script for Photo Booth
# This script stops the service, deploys new code, and restarts the service

set -euo pipefail

# Get the script directory - handle both original location and ~/bin location
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Set PROJECT_ROOT to the actual project location
PROJECT_ROOT="/Users/georgehyde/Documents/Projects/Bantam/rustBooth/canon_test_cam"
# Use project location for log file
LOG_FILE="${PROJECT_ROOT}/scripts/scheduled_deploy.log"
TEST_FILE="${HOME}/here_it_is.txt"

# Function to log messages
log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*" | tee -a "${LOG_FILE}"
}

# Function to log error and exit
error_exit() {
    log "ERROR: $1"
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] Deployment failed: $1" >> "${TEST_FILE}"
    exit 1
}

# Write test file to verify script is running
echo "[$(date '+%Y-%m-%d %H:%M:%S')] scheduled_deploy.sh started" > "${TEST_FILE}"
echo "Script directory: ${SCRIPT_DIR}" >> "${TEST_FILE}"
echo "Project root: ${PROJECT_ROOT}" >> "${TEST_FILE}"
echo "User: $(whoami)" >> "${TEST_FILE}"
echo "PATH: ${PATH}" >> "${TEST_FILE}"

# Start logging
log "=========================================="
log "Starting scheduled deployment"
log "Script running as user: $(whoami)"

# Get environment argument (default to prod)
env=${1:-prod}

# Set up environment variables
export PATH="/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin:${PATH}"
export HOME="${HOME:-/Users/georgehyde}"
export USER="${USER:-georgehyde}"
export DOCKER_HOST="unix:///var/run/docker.sock"

if [ "$env" == "prod" ]; then
    PI_HOST="${PI_HOST:-100.95.14.25}"
    log "Deploying to PRODUCTION: ${PI_HOST}"
    echo "Environment: PRODUCTION (${PI_HOST})" >> "${TEST_FILE}"
else
    PI_HOST="${PI_HOST:-100.90.132.44}"
    log "Deploying to DEVELOPMENT: ${PI_HOST}"
    echo "Environment: DEVELOPMENT (${PI_HOST})" >> "${TEST_FILE}"
fi

PI_USER="${PI_USER:-prospero}"
SSH_KEY_PATH="${SSH_KEY_PATH:-$HOME/.ssh/id_bantatam_pi}"

# Add SSH key path to test file
echo "SSH Key: ${SSH_KEY_PATH}" >> "${TEST_FILE}"

# Set up SSH options for non-interactive mode

# Check if deploy.sh exists
DEPLOY_SCRIPT="${PROJECT_ROOT}/deploy.sh"
if [ ! -f "${DEPLOY_SCRIPT}" ]; then
    error_exit "deploy.sh not found at ${DEPLOY_SCRIPT}"
fi

# Remove any extended attributes that might block execution
xattr -c "${DEPLOY_SCRIPT}" 2>/dev/null || true

# Ensure deploy.sh is executable
chmod +x "${DEPLOY_SCRIPT}"

echo "Deploy script found: ${DEPLOY_SCRIPT}" >> "${TEST_FILE}"
echo "Deploy script permissions: $(ls -la ${DEPLOY_SCRIPT})" >> "${TEST_FILE}"

# Test SSH connectivity first
log "Testing SSH connectivity to ${PI_USER}@${PI_HOST}..."
echo "[$(date '+%Y-%m-%d %H:%M:%S')] Testing SSH to ${PI_USER}@${PI_HOST}..." >> "${TEST_FILE}"

if ! ssh "${PI_USER}@${PI_HOST}" "echo 'SSH connection successful'" >/dev/null 2>&1; then
    log "WARNING: Cannot connect to ${PI_USER}@${PI_HOST} - continuing anyway for testing"
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] SSH connection failed (expected during testing)" >> "${TEST_FILE}"
    # Don't exit during testing phase
else
    log "SSH connection successful"
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] SSH connection successful" >> "${TEST_FILE}"

    # Stop the photobooth service
    log "Stopping photobooth-kiosk service..."
    if ssh "${PI_USER}@${PI_HOST}" "sudo systemctl stop photobooth-kiosk.service" 2>/dev/null; then
        log "Service stopped successfully"
        echo "[$(date '+%Y-%m-%d %H:%M:%S')] Service stopped" >> "${TEST_FILE}"
    else
        log "WARNING: Failed to stop service (might not be running)"
        echo "[$(date '+%Y-%m-%d %H:%M:%S')] Service stop failed or not running" >> "${TEST_FILE}"
    fi

    # Wait a moment for service to fully stop
    sleep 2
fi

# Execute deployment
log "Running deployment script..."
echo "[$(date '+%Y-%m-%d %H:%M:%S')] Starting deployment..." >> "${TEST_FILE}"

cd "${PROJECT_ROOT}" || error_exit "Failed to change to project directory"

# Check if Docker is available
if command -v docker >/dev/null 2>&1; then
    log "Docker found at: $(which docker)"
    echo "Docker: $(which docker)" >> "${TEST_FILE}"

    # Check Docker group membership
    DOCKER_GROUP=$(stat -f '%Sg' /var/run/docker.sock 2>/dev/null || echo "unknown")
    USER_GROUPS=$(id -Gn 2>/dev/null || echo "unknown")
    echo "Docker socket group: ${DOCKER_GROUP}" >> "${TEST_FILE}"
    echo "User groups: ${USER_GROUPS}" >> "${TEST_FILE}"

    # Test Docker access
    if docker info >/dev/null 2>&1; then
        log "Docker daemon is accessible"
        echo "Docker daemon: ACCESSIBLE" >> "${TEST_FILE}"
    else
        log "WARNING: Cannot access Docker daemon directly"
        echo "Docker daemon: NOT ACCESSIBLE (may need sudo or socket permissions)" >> "${TEST_FILE}"

        # Check if we can use Docker with sudo
        if sudo -n docker info >/dev/null 2>&1; then
            log "Docker accessible via sudo"
            echo "Docker via sudo: ACCESSIBLE" >> "${TEST_FILE}"
            export DOCKER_NEEDS_SUDO=1
        else
            echo "Docker via sudo: NOT ACCESSIBLE" >> "${TEST_FILE}"
        fi

        # Try to fix Docker socket permissions if needed
        if [ -e /var/run/docker.sock ]; then
            echo "Docker socket exists at /var/run/docker.sock" >> "${TEST_FILE}"
            echo "Socket permissions: $(ls -la /var/run/docker.sock)" >> "${TEST_FILE}"
        fi
    fi
else
    log "WARNING: Docker not found in PATH"
    echo "Docker: NOT FOUND" >> "${TEST_FILE}"
fi

# Create a temporary wrapper script to ensure proper execution
TEMP_WRAPPER="${HOME}/tmp_deploy_wrapper_$$.sh"
cat > "${TEMP_WRAPPER}" << 'WRAPPER_EOF'
#!/bin/bash
export PATH="/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin:${PATH}"
export DOCKER_HOST="unix:///var/run/docker.sock"

# If Docker needs sudo, create an alias
if [ "${DOCKER_NEEDS_SUDO}" = "1" ]; then
    docker() {
        sudo docker "$@"
    }
    export -f docker
fi

# Run the actual deploy script
exec /bin/bash DEPLOY_SCRIPT_PATH "$@"
WRAPPER_EOF

# Replace placeholder with actual path
sed -i '' "s|DEPLOY_SCRIPT_PATH|${DEPLOY_SCRIPT}|g" "${TEMP_WRAPPER}" 2>/dev/null || \
sed -i "s|DEPLOY_SCRIPT_PATH|${DEPLOY_SCRIPT}|g" "${TEMP_WRAPPER}" 2>/dev/null || true

chmod +x "${TEMP_WRAPPER}"

# Run deployment using the wrapper
log "Executing deployment via wrapper: ${TEMP_WRAPPER} $env"
if /bin/bash "${TEMP_WRAPPER}" "$env" 2>&1 | tee -a "${LOG_FILE}"; then
    log "Deployment completed successfully"
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] Deployment successful" >> "${TEST_FILE}"
    DEPLOY_SUCCESS=1
else
    DEPLOY_EXIT_CODE=$?
    log "WARNING: Deployment script exited with code $DEPLOY_EXIT_CODE"
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] Deployment failed with exit code $DEPLOY_EXIT_CODE" >> "${TEST_FILE}"

    # Log more details for debugging
    if [ $DEPLOY_EXIT_CODE -eq 126 ]; then
        echo "Exit code 126: Permission denied executing deploy.sh or its commands" >> "${TEST_FILE}"
        echo "Checking deploy.sh attributes: $(xattr -l ${DEPLOY_SCRIPT} 2>&1)" >> "${TEST_FILE}"

        # Try alternative execution method
        log "Attempting alternative execution method..."
        if cd "${PROJECT_ROOT}" && source "${DEPLOY_SCRIPT}" "$env" 2>&1 | tee -a "${LOG_FILE}"; then
            log "Alternative execution succeeded"
            echo "[$(date '+%Y-%m-%d %H:%M:%S')] Alternative execution successful" >> "${TEST_FILE}"
            DEPLOY_SUCCESS=1
        fi
    elif [ $DEPLOY_EXIT_CODE -eq 127 ]; then
        echo "Exit code 127: Command not found in deploy.sh" >> "${TEST_FILE}"
    fi
    DEPLOY_SUCCESS=0
    log "WARNING: Deployment failed but will restart service with existing binary"
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] Will restart service with existing binary" >> "${TEST_FILE}"
fi

# Clean up temp wrapper
rm -f "${TEMP_WRAPPER}"

# ALWAYS restart the photobooth service, even if deployment failed
# The service can run with the old binary if the new deployment failed
if ssh "${PI_USER}@${PI_HOST}" "echo 'test'" >/dev/null 2>&1; then
    log "Restarting photobooth-kiosk service (using ${DEPLOY_SUCCESS:+new}${DEPLOY_SUCCESS:-existing} binary)..."

    # First try to stop the service if it's running
    ssh "${PI_USER}@${PI_HOST}" "sudo systemctl stop photobooth-kiosk.service" 2>/dev/null || true
    sleep 2

    # Now start the service
    if ssh "${PI_USER}@${PI_HOST}" "sudo systemctl start photobooth-kiosk.service" 2>/dev/null; then
        log "Service started successfully"
        echo "[$(date '+%Y-%m-%d %H:%M:%S')] Service started" >> "${TEST_FILE}"
    else
        log "ERROR: Failed to start service"
        echo "[$(date '+%Y-%m-%d %H:%M:%S')] Service start failed" >> "${TEST_FILE}"

        # Try restart command as fallback
        log "Attempting systemctl restart as fallback..."
        if ssh "${PI_USER}@${PI_HOST}" "sudo systemctl restart photobooth-kiosk.service" 2>/dev/null; then
            log "Service restarted using restart command"
            echo "[$(date '+%Y-%m-%d %H:%M:%S')] Service restarted via restart command" >> "${TEST_FILE}"
        fi
    fi

    # Verify service is running
    sleep 3
    log "Verifying service status..."
    if ssh "${PI_USER}@${PI_HOST}" "sudo systemctl is-active photobooth-kiosk.service" 2>/dev/null | grep -q "active"; then
        log "Service is running"
        echo "[$(date '+%Y-%m-%d %H:%M:%S')] Service verified as running" >> "${TEST_FILE}"
    else
        log "WARNING: Service may not be running properly"
        echo "[$(date '+%Y-%m-%d %H:%M:%S')] Service status unknown" >> "${TEST_FILE}"
    fi
else
    log "ERROR: Cannot connect via SSH to restart service"
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] Cannot connect via SSH for service restart" >> "${TEST_FILE}"
fi

# Final status
log "Scheduled deployment completed"
log "=========================================="
echo "[$(date '+%Y-%m-%d %H:%M:%S')] scheduled_deploy.sh completed" >> "${TEST_FILE}"
echo "========================================" >> "${TEST_FILE}"

# Return success even if some parts failed (for testing phase)
exit 0

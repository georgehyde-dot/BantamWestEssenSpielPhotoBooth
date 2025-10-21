#!/usr/bin/env bash
# Build the aarch64 (Raspberry Pi) binary in Docker and deploy it via scp.
# You can override any of these with environment variables before running.
#
# Example:
#   PI_HOST="raspberrypi.local" ./deploy.sh

set -euo pipefail

# Get the environment argument (default to "dev" if not provided)
env=${1:-dev}
deploy_all=${2:-false}

if [ "$env" == "prod" ]; then
    PI_HOST="${PI_HOST:-100.95.14.25}"
    echo "Deploying to PRODUCTION: ${PI_HOST}"
else
    PI_HOST="${PI_HOST:-100.90.132.44}"
    echo "Deploying to DEVELOPMENT: ${PI_HOST}"
fi

# Defaults (can be overridden via env)
PI_USER="${PI_USER:-prospero}"
BINARY_NAME="${BINARY_NAME:-cam_test}"
REMOTE_DEST_PATH="${REMOTE_DEST_PATH:-/home/${PI_USER}/cam_test}"
DOCKER_IMAGE_NAME="${DOCKER_IMAGE_NAME:-cam-test-pi-builder}"
SSH_KEY_PATH="${SSH_KEY_PATH:-$HOME/.ssh/id_bantatam_pi}"

# Resolve important paths
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
DOCKERFILE_PATH="${SCRIPT_DIR}/Dockerfile"
OUT_DIR="${SCRIPT_DIR}/dist"
LOCAL_BIN="${OUT_DIR}/${BINARY_NAME}"
REMOTE_ASSETS_DIR="/usr/local/share/photo_booth"

echo "------------------------------------------------------------------"
echo "Build and Deploy Configuration:"
echo "  PI_USER          = ${PI_USER}"
echo "  PI_HOST          = ${PI_HOST}"
echo "  BINARY_NAME      = ${BINARY_NAME}"
echo "  REMOTE_DEST_PATH = ${REMOTE_DEST_PATH}"
echo "  DOCKER_IMAGE_NAME= ${DOCKER_IMAGE_NAME}"
echo "  SSH_KEY_PATH     = ${SSH_KEY_PATH}"
echo "  PROJECT_ROOT     = ${PROJECT_ROOT}"
echo "  DOCKERFILE_PATH  = ${DOCKERFILE_PATH}"
echo "  LOCAL_BIN        = ${LOCAL_BIN}"
echo "------------------------------------------------------------------"

# Basic checks
command -v docker >/dev/null 2>&1 || { echo "ERROR: docker not found in PATH"; exit 1; }
[ -f "${DOCKERFILE_PATH}" ] || { echo "ERROR: Dockerfile not found at ${DOCKERFILE_PATH}"; exit 1; }
[ -f "${SSH_KEY_PATH}" ] || { echo "ERROR: SSH key not found at ${SSH_KEY_PATH}"; exit 1; }

mkdir -p "${OUT_DIR}"

# Build image (final stage contains /cam_test artifact)
echo ">> Building Docker image '${DOCKER_IMAGE_NAME}' (context: ${PROJECT_ROOT})..."
DOCKER_BUILDKIT=1 docker build -t "${DOCKER_IMAGE_NAME}" -f "${DOCKERFILE_PATH}" "${PROJECT_ROOT}"

# Extract artifact from the final image stage
echo ">> Extracting binary from Docker image..."
CID="$(docker create "${DOCKER_IMAGE_NAME}")"
cleanup() {
  docker rm -f "${CID}" >/dev/null 2>&1 || true
}
trap cleanup EXIT

docker cp "${CID}:/cam_test" "${LOCAL_BIN}"
chmod +x "${LOCAL_BIN}"
echo ">> Binary extracted to ${LOCAL_BIN}"

# Ensure remote directory exists, then copy binary
REMOTE_DIR="$(dirname "${REMOTE_DEST_PATH}")"
echo ">> Ensuring remote directory exists: ${PI_USER}@${PI_HOST}:${REMOTE_DIR}"
ssh "${PI_USER}@${PI_HOST}" "mkdir -p '${REMOTE_DIR}'"

echo ">> Copying '${LOCAL_BIN}' to '${PI_USER}@${PI_HOST}:${REMOTE_DEST_PATH}'..."
scp "${LOCAL_BIN}" "${PI_USER}@${PI_HOST}:${REMOTE_DEST_PATH}"

# Optionally set executable bit on remote
echo ">> Marking remote binary as executable..."
ssh "${PI_USER}@${PI_HOST}" "chmod +x '${REMOTE_DEST_PATH}'"

# Function to check if a file/directory has changes
has_git_changes() {
    local path="$1"
    # Check if path exists
    [ -e "$path" ] || return 1

    # Check for uncommitted changes or untracked files
    if git diff HEAD --name-only | grep -q "^$(basename "$path")$"; then
        return 0
    fi
    if git status --porcelain | grep -q "$(basename "$path")"; then
        return 0
    fi

    # For directories, check if any file inside has changes
    if [ -d "$path" ]; then
        local rel_path=$(realpath --relative-to="$PROJECT_ROOT" "$path" 2>/dev/null || basename "$path")
        if git diff HEAD --name-only | grep -q "^$rel_path/"; then
            return 0
        fi
        if git status --porcelain | grep -q " $rel_path/"; then
            return 0
        fi
    fi

    return 1
}

# Determine if we should copy setup files
if [ "$deploy_all" == "true" ]; then
    echo ">> Deploy all flag is set - copying all setup files"
    should_copy_setup=true
else
    echo ">> Checking for changes in setup files..."
    should_copy_setup=false
fi


# Copy camera print test script if it exists and has changes or deploy_all is true
if [ -f "${SCRIPT_DIR}/test_camera_print.sh" ]; then
    if [ "$should_copy_setup" == "true" ] || has_git_changes "${SCRIPT_DIR}/test_camera_print.sh"; then
        echo ">> Copying test_camera_print.sh (changes detected or deploy_all)"
        scp "${SCRIPT_DIR}/test_camera_print.sh" "${PI_USER}@${PI_HOST}:${REMOTE_DIR}/test_camera_print.sh"
        ssh "${PI_USER}@${PI_HOST}" "chmod +x '${REMOTE_DIR}/test_camera_print.sh'"
    else
        echo ">> Skipping test_camera_print.sh (no changes)"
    fi
fi

# Copy printer setup script if it exists and has changes or deploy_all is true
if [ -f "${SCRIPT_DIR}/setup_printer.sh" ]; then
    if [ "$should_copy_setup" == "true" ] || has_git_changes "${SCRIPT_DIR}/setup_printer.sh"; then
        echo ">> Copying setup_printer.sh (changes detected or deploy_all)"
        scp "${SCRIPT_DIR}/setup_printer.sh" "${PI_USER}@${PI_HOST}:${REMOTE_DIR}/setup_printer.sh"
        ssh "${PI_USER}@${PI_HOST}" "chmod +x '${REMOTE_DIR}/setup_printer.sh'"
    else
        echo ">> Skipping setup_printer.sh (no changes)"
    fi
fi

# Copy setup script if it exists and has changes or deploy_all is true
if [ -f "${SCRIPT_DIR}/setup_packages.sh" ]; then
    if [ "$should_copy_setup" == "true" ] || has_git_changes "${SCRIPT_DIR}/setup_packages.sh"; then
        echo ">> Copying setup_packages.sh (changes detected or deploy_all)"
        scp "${SCRIPT_DIR}/setup_packages.sh" "${PI_USER}@${PI_HOST}:${REMOTE_DIR}/setup_packages.sh"
        ssh "${PI_USER}@${PI_HOST}" "chmod +x '${REMOTE_DIR}/setup_packages.sh'"
    else
        echo ">> Skipping setup_packages.sh (no changes)"
    fi
fi

# Copy diagnostic script if it exists and has changes or deploy_all is true
if [ -f "${SCRIPT_DIR}/check_setup.sh" ]; then
    if [ "$should_copy_setup" == "true" ] || has_git_changes "${SCRIPT_DIR}/check_setup.sh"; then
        echo ">> Copying check_setup.sh (changes detected or deploy_all)"
        scp "${SCRIPT_DIR}/check_setup.sh" "${PI_USER}@${PI_HOST}:${REMOTE_DIR}/check_setup.sh"
        ssh "${PI_USER}@${PI_HOST}" "chmod +x '${REMOTE_DIR}/check_setup.sh'"
    else
        echo ">> Skipping check_setup.sh (no changes)"
    fi
fi

# Copy troubleshooting directory if it exists and has changes or deploy_all is true
if [ -d "${SCRIPT_DIR}/troubleshooting" ]; then
    if [ "$should_copy_setup" == "true" ] || has_git_changes "${SCRIPT_DIR}/troubleshooting"; then
        echo ">> Copying troubleshooting directory (changes detected or deploy_all)"
        scp -r "${SCRIPT_DIR}/troubleshooting" "${PI_USER}@${PI_HOST}:${REMOTE_DIR}/"
        ssh "${PI_USER}@${PI_HOST}" "chmod +x '${REMOTE_DIR}/troubleshooting/'*.sh 2>/dev/null || true"
    else
        echo ">> Skipping troubleshooting directory (no changes)"
    fi
fi

# Copy scripts directory if it exists and has changes or deploy_all is true
if [ -d "${SCRIPT_DIR}/scripts" ]; then
    if [ "$should_copy_setup" == "true" ] || has_git_changes "${SCRIPT_DIR}/scripts"; then
        echo ">> Copying scripts directory (changes detected or deploy_all)"
        scp -r "${SCRIPT_DIR}/scripts" "${PI_USER}@${PI_HOST}:${REMOTE_DIR}/"
        ssh "${PI_USER}@${PI_HOST}" "chmod +x '${REMOTE_DIR}/scripts/'*.sh 2>/dev/null || true"
    else
        echo ">> Skipping scripts directory (no changes)"
    fi
fi

# Copy operations directory if it exists and has changes or deploy_all is true
if [ -d "${SCRIPT_DIR}/operations" ]; then
    if [ "$should_copy_setup" == "true" ] || has_git_changes "${SCRIPT_DIR}/operations"; then
        echo ">> Copying operations directory (changes detected or deploy_all)"
        scp -r "${SCRIPT_DIR}/operations" "${PI_USER}@${PI_HOST}:${REMOTE_DIR}/"
        ssh "${PI_USER}@${PI_HOST}" "chmod +x '${REMOTE_DIR}/operations/'*.sh 2>/dev/null || true"
    else
        echo ">> Skipping operations directory (no changes)"
    fi
fi

# Copy static directory if it exists and has changes or deploy_all is true
if [ -d "${SCRIPT_DIR}/static" ]; then
    if [ "$should_copy_setup" == "true" ] || has_git_changes "${SCRIPT_DIR}/static"; then
        echo ">> Ensuring remote assets directory exists: ${REMOTE_ASSETS_DIR}"
        ssh "${PI_USER}@${PI_HOST}" "sudo mkdir -p '${REMOTE_ASSETS_DIR}'"
        echo ">> Copying static directory (changes detected or deploy_all)"
        scp -r "${SCRIPT_DIR}/static" "${PI_USER}@${PI_HOST}:/tmp/"
        ssh "${PI_USER}@${PI_HOST}" "sudo rm -rf '${REMOTE_ASSETS_DIR}/static' && sudo mv /tmp/static '${REMOTE_ASSETS_DIR}/' && sudo chown -R ${PI_USER}:${PI_USER} '${REMOTE_ASSETS_DIR}/static'"

        # Copy font files to system fonts directory
        echo ">> Installing font files..."
        ssh "${PI_USER}@${PI_HOST}" "
            sudo mkdir -p /usr/local/share/fonts
            if ls '${REMOTE_ASSETS_DIR}/static/'*.ttf 1> /dev/null 2>&1; then
                sudo cp '${REMOTE_ASSETS_DIR}/static/'*.ttf /usr/local/share/fonts/
                sudo fc-cache -f -v /usr/local/share/fonts/
                echo 'Fonts installed successfully'
            else
                echo 'No font files found in static directory'
            fi
        "
    else
        echo ">> Skipping static directory (no changes)"
    fi
fi

# Copy printer configuration script if it exists and has changes or deploy_all is true
if [ -f "${SCRIPT_DIR}/configure_printer_4x6.sh" ]; then
    if [ "$should_copy_setup" == "true" ] || has_git_changes "${SCRIPT_DIR}/configure_printer_4x6.sh"; then
        echo ">> Copying configure_printer_4x6.sh (changes detected or deploy_all)"
        scp "${SCRIPT_DIR}/configure_printer_4x6.sh" "${PI_USER}@${PI_HOST}:${REMOTE_DIR}/configure_printer_4x6.sh"
        ssh "${PI_USER}@${PI_HOST}" "chmod +x '${REMOTE_DIR}/configure_printer_4x6.sh'"
    else
        echo ">> Skipping configure_printer_4x6.sh (no changes)"
    fi
fi

# Copy font installation script if it exists and has changes or deploy_all is true
if [ -f "${SCRIPT_DIR}/install_fonts.sh" ]; then
    if [ "$should_copy_setup" == "true" ] || has_git_changes "${SCRIPT_DIR}/install_fonts.sh"; then
        echo ">> Copying install_fonts.sh (changes detected or deploy_all)"
        scp "${SCRIPT_DIR}/install_fonts.sh" "${PI_USER}@${PI_HOST}:${REMOTE_DIR}/install_fonts.sh"
        ssh "${PI_USER}@${PI_HOST}" "chmod +x '${REMOTE_DIR}/install_fonts.sh'"
    else
        echo ">> Skipping install_fonts.sh (no changes)"
    fi
fi

# Create database file with proper permissions
echo ">> Ensuring database file exists with proper permissions..."
ssh "${PI_USER}@${PI_HOST}" "
    sudo mkdir -p '${REMOTE_ASSETS_DIR}'
    sudo chown ${PI_USER}:${PI_USER} '${REMOTE_ASSETS_DIR}'
    sudo chmod 755 '${REMOTE_ASSETS_DIR}'
    touch '${REMOTE_ASSETS_DIR}/photo_booth.db'
    chmod 664 '${REMOTE_ASSETS_DIR}/photo_booth.db'
    echo 'Database file created at ${REMOTE_ASSETS_DIR}/photo_booth.db'
    echo 'Directory permissions: \$(ls -ld ${REMOTE_ASSETS_DIR})'
    echo 'Database permissions: \$(ls -l ${REMOTE_ASSETS_DIR}/photo_booth.db)'
"

echo "------------------------------------------------------------------"
echo "Deploy complete."

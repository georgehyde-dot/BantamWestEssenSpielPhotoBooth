#!/usr/bin/env bash
# Build the aarch64 (Raspberry Pi) binary in Docker and deploy it via scp.
# You can override any of these with environment variables before running.
#
# Example:
#   PI_HOST="raspberrypi.local" ./deploy.sh

set -euo pipefail

# Defaults (can be overridden via env)
PI_USER="${PI_USER:-prospero}"
PI_HOST="${PI_HOST:-BantamPhotoShop.local}"
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
ssh -i "${SSH_KEY_PATH}" -o StrictHostKeyChecking=accept-new "${PI_USER}@${PI_HOST}" "mkdir -p '${REMOTE_DIR}'"

echo ">> Copying '${LOCAL_BIN}' to '${PI_USER}@${PI_HOST}:${REMOTE_DEST_PATH}'..."
scp -i "${SSH_KEY_PATH}" -o StrictHostKeyChecking=accept-new "${LOCAL_BIN}" "${PI_USER}@${PI_HOST}:${REMOTE_DEST_PATH}"

# Optionally set executable bit on remote
echo ">> Marking remote binary as executable..."
ssh -i "${SSH_KEY_PATH}" -o StrictHostKeyChecking=accept-new "${PI_USER}@${PI_HOST}" "chmod +x '${REMOTE_DEST_PATH}'"

# Copy debug script for troubleshooting print issues
DEBUG_SCRIPT="${SCRIPT_DIR}/debug_print.sh"
REMOTE_DEBUG_PATH="/home/${PI_USER}/debug_print.sh"
if [[ -f "${DEBUG_SCRIPT}" ]]; then
    echo ">> Copying debug script..."
    scp -i "${SSH_KEY_PATH}" -o StrictHostKeyChecking=accept-new "${DEBUG_SCRIPT}" "${PI_USER}@${PI_HOST}:${REMOTE_DEBUG_PATH}"
    ssh -i "${SSH_KEY_PATH}" -o StrictHostKeyChecking=accept-new "${PI_USER}@${PI_HOST}" "chmod +x '${REMOTE_DEBUG_PATH}'"
    echo ">> Debug script deployed to: ${REMOTE_DEBUG_PATH}"
else
    echo ">> Debug script not found at ${DEBUG_SCRIPT}, skipping..."
fi

echo "------------------------------------------------------------------"
echo "Deploy complete."
echo "Remote binary: ${PI_USER}@${PI_HOST}:${REMOTE_DEST_PATH}"
if [[ -f "${DEBUG_SCRIPT}" ]]; then
    echo "Debug script: ${PI_USER}@${PI_HOST}:${REMOTE_DEBUG_PATH}"
fi
echo
echo "Run on the Pi (example):"
echo "  ssh -i '${SSH_KEY_PATH}' ${PI_USER}@${PI_HOST} \"VIDEO_DEVICE=/dev/video0 VIDEO_WIDTH=1280 VIDEO_HEIGHT=720 '${REMOTE_DEST_PATH}'\""
echo
echo "Troubleshooting print issues:"
echo "  # Run debug script to check CUPS permissions:"
echo "  ssh -i '${SSH_KEY_PATH}' ${PI_USER}@${PI_HOST} \"sudo ./debug_print.sh all\""
echo
echo "  # Check CUPS status and logs:"
echo "  ssh -i '${SSH_KEY_PATH}' ${PI_USER}@${PI_HOST} \"lpstat -p && tail -20 /var/log/cups/error_log\""
echo
echo "  # Fix common permission issues:"
echo "  ssh -i '${SSH_KEY_PATH}' ${PI_USER}@${PI_HOST} \"sudo ./debug_print.sh fix\""
echo "------------------------------------------------------------------"

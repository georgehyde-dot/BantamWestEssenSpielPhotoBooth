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
ssh -i "${SSH_KEY_PATH}" -o StrictHostKeyChecking=accept-new "${PI_USER}@${PI_HOST}" "mkdir -p '${REMOTE_DIR}'"

echo ">> Copying '${LOCAL_BIN}' to '${PI_USER}@${PI_HOST}:${REMOTE_DEST_PATH}'..."
scp -i "${SSH_KEY_PATH}" -o StrictHostKeyChecking=accept-new "${LOCAL_BIN}" "${PI_USER}@${PI_HOST}:${REMOTE_DEST_PATH}"

# Optionally set executable bit on remote
echo ">> Marking remote binary as executable..."
ssh -i "${SSH_KEY_PATH}" -o StrictHostKeyChecking=accept-new "${PI_USER}@${PI_HOST}" "chmod +x '${REMOTE_DEST_PATH}'"

# Copy test scripts if they exist
if [ -f "${SCRIPT_DIR}/test_endpoints.sh" ]; then
    echo ">> Copying test_endpoints.sh..."
    scp -i "${SSH_KEY_PATH}" -o StrictHostKeyChecking=accept-new "${SCRIPT_DIR}/test_endpoints.sh" "${PI_USER}@${PI_HOST}:${REMOTE_DIR}/test_endpoints.sh"
    ssh -i "${SSH_KEY_PATH}" -o StrictHostKeyChecking=accept-new "${PI_USER}@${PI_HOST}" "chmod +x '${REMOTE_DIR}/test_endpoints.sh'"
fi

# Copy scripts directory if it exists
if [ -d "${SCRIPT_DIR}/scripts" ]; then
    echo ">> Copying scripts directory..."
    scp -i "${SSH_KEY_PATH}" -o StrictHostKeyChecking=accept-new -r "${SCRIPT_DIR}/scripts" "${PI_USER}@${PI_HOST}:${REMOTE_DIR}/"
    ssh -i "${SSH_KEY_PATH}" -o StrictHostKeyChecking=accept-new "${PI_USER}@${PI_HOST}" "chmod +x '${REMOTE_DIR}/scripts/'*.sh 2>/dev/null || true"
fi

# Copy operations directory if it exists
if [ -d "${SCRIPT_DIR}/operations" ]; then
    echo ">> Copying operations directory..."
    scp -i "${SSH_KEY_PATH}" -o StrictHostKeyChecking=accept-new -r "${SCRIPT_DIR}/operations" "${PI_USER}@${PI_HOST}:${REMOTE_DIR}/"
    ssh -i "${SSH_KEY_PATH}" -o StrictHostKeyChecking=accept-new "${PI_USER}@${PI_HOST}" "chmod +x '${REMOTE_DIR}/operations/'*.sh 2>/dev/null || true"
fi

# Copy static directory if it exists (contains background images, etc.)
if [ -d "${SCRIPT_DIR}/static" ]; then
    echo ">> Ensuring remote assets directory exists: ${REMOTE_ASSETS_DIR}"
    ssh -i "${SSH_KEY_PATH}" -o StrictHostKeyChecking=accept-new "${PI_USER}@${PI_HOST}" "sudo mkdir -p '${REMOTE_ASSETS_DIR}'"
    echo ">> Copying static directory..."
    scp -i "${SSH_KEY_PATH}" -o StrictHostKeyChecking=accept-new -r "${SCRIPT_DIR}/static" "${PI_USER}@${PI_HOST}:/tmp/"
    ssh -i "${SSH_KEY_PATH}" -o StrictHostKeyChecking=accept-new "${PI_USER}@${PI_HOST}" "sudo rm -rf '${REMOTE_ASSETS_DIR}/static' && sudo mv /tmp/static '${REMOTE_ASSETS_DIR}/' && sudo chown -R ${PI_USER}:${PI_USER} '${REMOTE_ASSETS_DIR}/static'"
fi

echo "------------------------------------------------------------------"
echo "Deploy complete."
echo "Remote binary: ${PI_USER}@${PI_HOST}:${REMOTE_DEST_PATH}"
echo
echo "Run on the Pi with Canon EOS camera:"
echo "  ssh -i '${SSH_KEY_PATH}' ${PI_USER}@${PI_HOST} \"cd ${REMOTE_DIR} && ./scripts/run.sh\""
echo
echo "Test GPhoto2 functionality:"
echo "  ssh -i '${SSH_KEY_PATH}' ${PI_USER}@${PI_HOST} \"cd ${REMOTE_DIR} && ./scripts/test_gphoto.sh\""
echo
echo "Run directly (without startup script):"
echo "  ssh -i '${SSH_KEY_PATH}' ${PI_USER}@${PI_HOST} \"'${REMOTE_DEST_PATH}'\""
echo
echo "Setup kiosk mode (first time only):"
echo "  ssh -i '${SSH_KEY_PATH}' ${PI_USER}@${PI_HOST} \"sudo /home/${PI_USER}/operations/setup-kiosk.sh\""
echo
echo "Start kiosk mode:"
echo "  ssh -i '${SSH_KEY_PATH}' ${PI_USER}@${PI_HOST} \"sudo systemctl start photobooth-kiosk.service\""
echo
echo "Stop kiosk mode:"
echo "  ssh -i '${SSH_KEY_PATH}' ${PI_USER}@${PI_HOST} \"sudo systemctl stop photobooth-kiosk.service\""
echo
echo "Check kiosk status:"
echo "  ssh -i '${SSH_KEY_PATH}' ${PI_USER}@${PI_HOST} \"sudo systemctl status photobooth-kiosk.service\""
echo

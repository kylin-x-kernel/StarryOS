#!/bin/sh
set -e

# This script automates the creation of a disk image for StarryOS.
# It can be run from either the project root or the scripts directory.

# Determine project root directory
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
if [ "$(basename "$SCRIPT_DIR")" = "scripts" ]; then
    PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
else
    PROJECT_ROOT="$SCRIPT_DIR"
fi

echo "Project root: $PROJECT_ROOT"

# --- Configuration ---
IMAGE_NAME="alpine.img"
IMAGE_SIZE_MB=1024
MOUNT_POINT="/mnt"
ROOTFS_DIR="alpine-rootfs-strace"
DEST_DIR="$PROJECT_ROOT/arceos"
DEST_IMAGE_NAME="disk.img"
STRACE_BIN="$PROJECT_ROOT/userspace/strace/target/aarch64-unknown-linux-musl/release/strace"
ALPINE_TARBALL="$PROJECT_ROOT/alpine-minirootfs.tar.gz"
# --- End Configuration ---

# Build strace as regular user before requiring sudo
if [ "$(id -u)" -eq 0 ]; then
  echo "Error: Do not run this script with sudo directly." >&2
  echo "The script will request sudo when needed." >&2
  exit 1
fi

# Check if the target is installed, install if needed
echo "Checking for aarch64-unknown-linux-musl target..."
if ! rustup target list --installed | grep -q "aarch64-unknown-linux-musl"; then
  echo "Target not found, installing aarch64-unknown-linux-musl..."
  rustup target add aarch64-unknown-linux-musl
else
  echo "Target aarch64-unknown-linux-musl is already installed."
fi

echo "Compiling strace as regular user..."
cd "$PROJECT_ROOT/userspace/strace"
cargo build --release --target aarch64-unknown-linux-musl

echo "Strace compiled."

# Check if strace binary was built successfully
if [ ! -f "$STRACE_BIN" ]; then
  echo "Error: strace binary not found at $STRACE_BIN" >&2
  exit 1
fi

# Change back to project root for remaining operations
cd "$PROJECT_ROOT"

echo "Preparing root filesystem directory..."
# Clean up any existing rootfs directory
rm -rf "$ROOTFS_DIR"
mkdir "$ROOTFS_DIR"

# check for existing minirootfs tarball, download if not present
if [ ! -f "$ALPINE_TARBALL" ]; then
    echo "Alpine minirootfs tarball not found, downloading..."
    wget https://dl-cdn.alpinelinux.org/alpine/v3.22/releases/aarch64/alpine-minirootfs-3.22.2-aarch64.tar.gz -O "$ALPINE_TARBALL"
else
    echo "Using existing alpine-minirootfs.tar.gz"
fi

echo "Extracting Alpine Linux minirootfs..."
tar -xzf "$ALPINE_TARBALL" -C "$ROOTFS_DIR"

echo "Copying strace binary into rootfs directory..."
mkdir -p "${ROOTFS_DIR}/usr/bin"
cp "$STRACE_BIN" "${ROOTFS_DIR}/usr/bin/"

# 1. Create a 1GB disk image file
echo "Creating ${IMAGE_SIZE_MB}MB disk image..."
dd if=/dev/zero of="$IMAGE_NAME" bs=1M count="$IMAGE_SIZE_MB"

# 2. Format the image with the ext4 filesystem
echo "Formatting image with ext4 (requires sudo)..."
sudo mkfs.ext4 "$IMAGE_NAME"

# 3. Mount the image to the mount point
echo "Mounting image to $MOUNT_POINT (requires sudo)..."
sudo mount -o loop "$IMAGE_NAME" "$MOUNT_POINT"

# 4. Copy the root filesystem into the image
echo "Copying rootfs from ./${ROOTFS_DIR}..."
sudo cp -a "${ROOTFS_DIR}/"* "$MOUNT_POINT"

# 5. Unmount the image
echo "Unmounting image..."
sudo umount "$MOUNT_POINT"

# 6. Move and rename the image to the destination directory
echo "Moving image to ${DEST_DIR}/${DEST_IMAGE_NAME}..."
mv "$IMAGE_NAME" "${DEST_DIR}/${DEST_IMAGE_NAME}"
echo "Cleaning up rootfs directory..."
rm -rf "$ROOTFS_DIR"

echo "Done. Disk image created at ${DEST_DIR}/${DEST_IMAGE_NAME}"
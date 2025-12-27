#!/bin/sh

MOUNTPOINT="/mnt/world"

cleanup() {
    echo "Received shutdown signal, unmounting..."
    # Kill the FUSE process first
    kill "$FUSE_PID" 2>/dev/null || true
    wait "$FUSE_PID" 2>/dev/null || true
    # Then unmount
    fusermount -uz "$MOUNTPOINT" 2>/dev/null || true
    exit 0
}

# Signal numbers: 15=SIGTERM, 2=SIGINT, 3=SIGQUIT
trap cleanup 15 2 3

# Run the FUSE driver in background
mc-anvil-db "$@" &
FUSE_PID=$!

# Wait for it (this allows trap to work)
wait "$FUSE_PID"

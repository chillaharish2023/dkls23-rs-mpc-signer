#!/bin/bash
# Run message relay service
# Usage: ./scripts/run-relay.sh [port]

set -e

PORT=${1:-8080}
RUST_LOG=${RUST_LOG:-info}

echo "=== Starting Message Relay Service ==="
echo "Port: $PORT"
echo "Log level: $RUST_LOG"
echo ""

RUST_LOG=$RUST_LOG cargo run -p msg-relay-svc --release -- \
    --listen "0.0.0.0:$PORT"

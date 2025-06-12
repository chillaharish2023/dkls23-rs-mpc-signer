#!/bin/bash
# Distributed Key Generation Script
# Usage: ./scripts/dkg.sh <n_parties> <threshold>

set -e

N=${1:-3}
T=${2:-2}
DEST=${DEST:-./data}
RELAY_URL=${RELAY_URL:-http://127.0.0.1:8080}

echo "=== DKLs23 Distributed Key Generation ==="
echo "Parties: $N"
echo "Threshold: $T"
echo "Data directory: $DEST"
echo "Relay URL: $RELAY_URL"
echo ""

mkdir -p "$DEST"

# Start all parties in parallel
for i in $(seq 0 $((N-1))); do
    echo "Starting party $i..."
    PARTY_ID=$i DEST=$DEST RELAY_URL=$RELAY_URL \
        cargo run -p dkls-party --release -q -- \
        --party-id $i \
        --dest "$DEST" \
        --relay "$RELAY_URL" \
        keygen --n $N --t $T &
done

# Wait for all parties to complete
wait

echo ""
echo "=== DKG Complete ==="
echo "Key shares saved to: $DEST"
ls -la "$DEST"/keyshare.*.json 2>/dev/null || echo "No key shares found"

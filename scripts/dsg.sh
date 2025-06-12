#!/bin/bash
# Distributed Signature Generation Script
# Usage: ./scripts/dsg.sh <message_hex> <party_ids...>
# Example: ./scripts/dsg.sh "abcd1234..." 0 1 2

set -e

MESSAGE=$1
shift
PARTIES="$@"

DEST=${DEST:-./data}
RELAY_URL=${RELAY_URL:-http://127.0.0.1:8080}

# Convert party IDs to comma-separated string
PARTIES_STR=$(echo "$PARTIES" | tr ' ' ',')

echo "=== DKLs23 Distributed Signature Generation ==="
echo "Message: $MESSAGE"
echo "Parties: $PARTIES_STR"
echo "Data directory: $DEST"
echo "Relay URL: $RELAY_URL"
echo ""

# Start signing for each party in parallel
for party_id in $PARTIES; do
    echo "Starting party $party_id..."
    PARTY_ID=$party_id DEST=$DEST RELAY_URL=$RELAY_URL \
        cargo run -p dkls-party --release -q -- \
        --party-id $party_id \
        --dest "$DEST" \
        --relay "$RELAY_URL" \
        sign --message "$MESSAGE" --parties "$PARTIES_STR" &
done

# Wait for all parties to complete
wait

echo ""
echo "=== Signing Complete ==="

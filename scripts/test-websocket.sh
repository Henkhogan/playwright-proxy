#!/bin/bash
# Test script for WebSocket functionality

set -e

echo "=== WebSocket Connection Test ==="
echo ""

# Check if websocat is installed
if ! command -v websocat &> /dev/null; then
    echo "Error: websocat is not installed"
    echo "Install it with:"
    echo "  Linux: wget https://github.com/vi/websocat/releases/latest/download/websocat.x86_64-unknown-linux-musl"
    echo "  macOS: brew install websocat"
    exit 1
fi

# Get the service endpoint
if [ -n "$1" ]; then
    ENDPOINT="$1"
else
    echo "Usage: $0 <endpoint>"
    echo "Example: $0 ws://localhost:3128/logs"
    echo "         $0 ws://playwright-proxy.local/logs"
    exit 1
fi

echo "Testing WebSocket connection to: $ENDPOINT"
echo ""

# Test the connection with a timeout
timeout 5s websocat "$ENDPOINT" &
WS_PID=$!

# Give it a moment to connect
sleep 1

# Make a test request to generate logs
echo "Generating test logs with a proxy request..."
HOST=$(echo "$ENDPOINT" | sed -E 's|ws://([^:/]+).*|\1|')
PORT=$(echo "$ENDPOINT" | sed -E 's|ws://[^:]+:([0-9]+).*|\1|')

if [[ "$ENDPOINT" == *":3128"* ]]; then
    curl -s "http://${HOST}:${PORT}/health" > /dev/null && echo "Health check successful"
    curl -s "http://${HOST}:${PORT}/https://example.com" > /dev/null && echo "Proxy request sent"
elif [[ "$ENDPOINT" == *".local"* ]]; then
    curl -s "http://${HOST}/health" > /dev/null && echo "Health check successful"
    curl -s "http://${HOST}/https://example.com" > /dev/null && echo "Proxy request sent"
fi

# Wait for websocat to finish or timeout
wait $WS_PID 2>/dev/null || true

echo ""
echo "=== Test Complete ==="
echo "If you saw log messages above, WebSocket is working!"
echo "If not, check:"
echo "  1. Is the service running?"
echo "  2. Is the endpoint correct?"
echo "  3. Are there firewall/ingress issues?"

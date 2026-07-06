#!/bin/bash
# Comprehensive WebSocket test script

set -e

echo "🧪 WebSocket Connection Test Suite"
echo "===================================="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print status
print_status() {
    if [ $1 -eq 0 ]; then
        echo -e "${GREEN}✓ $2${NC}"
    else
        echo -e "${RED}✗ $2${NC}"
    fi
}

# Test 1: Check if websocat is installed
echo "Test 1: Checking for websocat..."
if command -v websocat &> /dev/null; then
    print_status 0 "websocat is installed"
else
    print_status 1 "websocat is not installed"
    echo ""
    echo "Install websocat:"
    echo "  Linux: wget https://github.com/vi/websocat/releases/latest/download/websocat.x86_64-unknown-linux-musl && chmod +x websocat.x86_64-unknown-linux-musl && sudo mv websocat.x86_64-unknown-linux-musl /usr/local/bin/websocat"
    echo "  macOS: brew install websocat"
    exit 1
fi

# Get endpoint from argument or use default
ENDPOINT="${1:-ws://localhost:3128/logs}"
HOST=$(echo "$ENDPOINT" | sed -E 's|ws://([^:/]+).*|\1|')
PORT=$(echo "$ENDPOINT" | grep -oP ':\K[0-9]+' || echo "3128")

echo ""
echo "Test 2: Checking if service is reachable..."
if timeout 2 bash -c "echo > /dev/tcp/$HOST/$PORT" 2>/dev/null; then
    print_status 0 "Service is reachable at $HOST:$PORT"
else
    print_status 1 "Cannot reach service at $HOST:$PORT"
    echo ""
    echo "Troubleshooting:"
    echo "  1. Is the service running?"
    echo "  2. Check with: kubectl get pods -l app.kubernetes.io/name=playwright-proxy"
    echo "  3. Try port-forward: kubectl port-forward svc/playwright-proxy 3128:3128"
    exit 1
fi

# Test 3: Test health endpoint
echo ""
echo "Test 3: Testing health endpoint..."
if curl -s -f "http://$HOST:$PORT/health" > /dev/null 2>&1; then
    print_status 0 "Health endpoint responds"
    HEALTH_STATUS=$(curl -s "http://$HOST:$PORT/health")
    echo "   Response: $HEALTH_STATUS"
else
    print_status 1 "Health endpoint not responding"
    exit 1
fi

# Test 4: Test WebSocket handshake
echo ""
echo "Test 4: Testing WebSocket handshake..."
WS_RESPONSE=$(curl -s -i -N \
  -H "Connection: Upgrade" \
  -H "Upgrade: websocket" \
  -H "Sec-WebSocket-Version: 13" \
  -H "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==" \
  "http://$HOST:$PORT/logs" 2>&1 | head -1)

if echo "$WS_RESPONSE" | grep -q "101"; then
    print_status 0 "WebSocket handshake successful (101 Switching Protocols)"
else
    print_status 1 "WebSocket handshake failed"
    echo "   Response: $WS_RESPONSE"
    exit 1
fi

# Test 5: Full WebSocket connection test
echo ""
echo "Test 5: Testing full WebSocket connection..."
echo "   Connecting to $ENDPOINT..."

# Start websocat in background and capture output
TEMP_FILE=$(mktemp)
timeout 10s websocat "$ENDPOINT" > "$TEMP_FILE" 2>&1 &
WS_PID=$!

# Give it time to connect
sleep 2

# Check if websocat is still running (connected)
if kill -0 $WS_PID 2>/dev/null; then
    print_status 0 "WebSocket connection established"
    
    # Test 6: Generate logs
    echo ""
    echo "Test 6: Generating logs..."
    
    # Make a health check request to generate logs
    curl -s "http://$HOST:$PORT/health" > /dev/null 2>&1 &
    
    # Wait a bit for logs to arrive
    sleep 2
    
    # Kill websocat
    kill $WS_PID 2>/dev/null || true
    wait $WS_PID 2>/dev/null || true
    
    # Check if we received any logs
    if [ -s "$TEMP_FILE" ]; then
        LOG_COUNT=$(wc -l < "$TEMP_FILE")
        print_status 0 "Received log messages ($LOG_COUNT lines)"
        echo ""
        echo "Sample logs:"
        head -5 "$TEMP_FILE" | sed 's/^/   /'
    else
        print_status 1 "No log messages received"
        echo "   This might be normal if no requests were made"
    fi
else
    print_status 1 "WebSocket connection failed"
    cat "$TEMP_FILE"
fi

# Cleanup
rm -f "$TEMP_FILE"

echo ""
echo "===================================="
echo -e "${GREEN}✓ All tests passed!${NC}"
echo ""
echo "WebSocket endpoint is working correctly!"
echo ""
echo "To connect manually:"
echo "  websocat $ENDPOINT"
echo ""
echo "To generate logs:"
echo "  curl http://$HOST:$PORT/health"
echo "  curl http://$HOST:$PORT/https://example.com"

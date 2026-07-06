# Quick Start: WebSocket on k3s

## TL;DR

**Option 1: Gateway API (Recommended)**
```bash
# Deploy with Gateway API HTTPRoute
helm install playwright-proxy ./helm/playwright-proxy \
  -f helm/playwright-proxy/examples/k3s-gateway-api.yaml

# Add to /etc/hosts (replace with your k3s node IP)
echo "192.168.1.100 playwright-proxy.local" | sudo tee -a /etc/hosts

# Test WebSocket connection
websocat ws://playwright-proxy.local/logs
```

**Option 2: Standard Ingress**
```bash
# Deploy with Traefik Ingress
helm install playwright-proxy ./helm/playwright-proxy \
  -f helm/playwright-proxy/examples/k3s-traefik.yaml

# Add to /etc/hosts (replace with your k3s node IP)
echo "192.168.1.100 playwright-proxy.local" | sudo tee -a /etc/hosts

# Test WebSocket connection
websocat ws://playwright-proxy.local/logs
```

## What Was Fixed

The WebSocket connection to `/logs` wasn't working on k3s because:
1. The code used `peek()` which doesn't work through proxies
2. Missing Ingress annotations for WebSocket support
3. Case-sensitive header detection

All fixed now! ✅

## Deployment Methods

### Method 1: Gateway API HTTPRoute (Recommended)

Gateway API provides native WebSocket support without special configuration:

```bash
helm install playwright-proxy ./helm/playwright-proxy \
  -f helm/playwright-proxy/examples/k3s-gateway-api.yaml

# Wait for deployment
kubectl wait --for=condition=available --timeout=60s \
  deployment/playwright-proxy

# Verify HTTPRoute is created
kubectl get httproute playwright-proxy

# Test
websocat ws://playwright-proxy.local/logs
```

**Benefits:**
- Native WebSocket support (no annotations needed)
- Modern Kubernetes API (successor to Ingress)
- Automatic protocol upgrade handling
- Better routing capabilities

### Method 2: Traefik Ingress (Legacy)

```bash
helm install playwright-proxy ./helm/playwright-proxy \
  -f helm/playwright-proxy/examples/k3s-traefik.yaml

# Wait for deployment
kubectl wait --for=condition=available --timeout=60s \
  deployment/playwright-proxy

# Test
websocat ws://playwright-proxy.local/logs
```

### Method 3: NodePort (Development)

```bash
helm install playwright-proxy ./helm/playwright-proxy \
  --set service.type=NodePort \
  --set ingress.enabled=false

# Get the port
kubectl get svc playwright-proxy

# Connect (replace 31234 with your NodePort)
websocat ws://localhost:31234/logs
```

### Method 4: Port-Forward (Quick Test)

```bash
helm install playwright-proxy ./helm/playwright-proxy

# Forward port
kubectl port-forward svc/playwright-proxy 3128:3128

# In another terminal
websocat ws://localhost:3128/logs
```

## Testing

Generate logs to verify WebSocket is working:

```bash
# Terminal 1: Watch logs
websocat ws://playwright-proxy.local/logs

# Terminal 2: Make a request
curl http://playwright-proxy.local/https://example.com
```

You should see logs appear in Terminal 1.

## Files Changed

- `src/main.rs` - Fixed WebSocket detection and buffering
- `helm/playwright-proxy/values.yaml` - Added WebSocket config for both Ingress and HTTPRoute
- `helm/playwright-proxy/templates/ingress.yaml` - Added annotations
- `helm/playwright-proxy/templates/httproute.yaml` - Added WebSocket support
- `helm/playwright-proxy/examples/k3s-traefik.yaml` - Ingress example
- `helm/playwright-proxy/examples/k3s-gateway-api.yaml` - Gateway API example

## Full Documentation

- **Detailed guide**: [WEBSOCKET_K3S.md](WEBSOCKET_K3S.md)
- **All changes**: [WEBSOCKET_FIX_SUMMARY.md](WEBSOCKET_FIX_SUMMARY.md)
- **Main docs**: [README.md](README.md)

## Troubleshooting

**Connection refused?**
```bash
# Check pod status
kubectl get pods -l app.kubernetes.io/name=playwright-proxy

# Check pod logs
kubectl logs -l app.kubernetes.io/name=playwright-proxy

# If using Gateway API, check HTTPRoute
kubectl describe httproute playwright-proxy

# If using Ingress, check Ingress
kubectl describe ingress playwright-proxy

# Try port-forward to bypass routing
kubectl port-forward svc/playwright-proxy 3128:3128
websocat ws://localhost:3128/logs
```

**No logs appearing?**
```bash
# Make a request to generate logs
curl http://playwright-proxy.local/health

# Check if logs work in pod directly
kubectl logs -l app.kubernetes.io/name=playwright-proxy -f
```

Need more help? See [WEBSOCKET_K3S.md](WEBSOCKET_K3S.md) for detailed troubleshooting.

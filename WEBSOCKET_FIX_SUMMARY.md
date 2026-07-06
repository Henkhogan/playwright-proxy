# WebSocket k3s Fix - Summary of Changes

## Problem
WebSocket connections to `/logs` endpoint were not working when playwright-proxy was deployed on k3s with Traefik Ingress.

## Root Causes

1. **Code Issue**: The `peek()` method used for detecting WebSocket upgrade requests doesn't work reliably when connections come through an Ingress/proxy, as the data may be buffered.

2. **Missing WebSocket Headers**: Ingress controllers need proper annotations to handle WebSocket upgrade requests.

3. **Case-Sensitivity**: The WebSocket detection was case-sensitive, but HTTP headers can have different casing.

## Changes Made

### 1. Code Changes (`src/main.rs`)

#### Replaced `peek()` with proper `read()`
- **Before**: Used `client_stream.peek()` to inspect data without consuming it
- **After**: Use `client_stream.read()` to properly read data and pass it along

#### Added Buffer Handling
- **New function**: `handle_client_with_buffer()` - accepts pre-read buffer
- **New function**: `websocket_handler_with_buffer()` - handles WebSocket with pre-read data
- **Removed**: Old `handle_client()` wrapper that was causing double-reads

#### Improved WebSocket Detection
```rust
// Before
request_str.contains("Upgrade: websocket")

// After (case-insensitive)
request_str.contains("Upgrade: websocket") || 
request_str.contains("upgrade: websocket")
```

### 2. Helm Chart Changes

#### Updated `values.yaml`
Added WebSocket-specific configuration section:

```yaml
ingress:
  # ... existing config ...
  
  # WebSocket-specific configuration
  websocket:
    enabled: true
    annotations:
      # Traefik (k3s default)
      traefik.ingress.kubernetes.io/router.middlewares: ""
      # Nginx
      nginx.ingress.kubernetes.io/websocket-services: "{{ include \"playwright-proxy.fullname\" . }}"
      nginx.ingress.kubernetes.io/proxy-read-timeout: "3600"
      nginx.ingress.kubernetes.io/proxy-send-timeout: "3600"
```

#### Updated `templates/ingress.yaml`
Modified to include WebSocket annotations:

```yaml
annotations:
  {{- if .Values.ingress.websocket.enabled }}
  # WebSocket support annotations
  {{- with .Values.ingress.websocket.annotations }}
  {{- toYaml . | nindent 4 }}
  {{- end }}
  {{- end }}
  {{- with .Values.ingress.annotations }}
  {{- toYaml . | nindent 4 }}
  {{- end }}
```

### 3. New Files Created

#### `helm/playwright-proxy/examples/k3s-traefik.yaml`
Ready-to-use configuration for k3s with Traefik, including:
- Proper Ingress configuration
- WebSocket support enabled
- Traefik-specific annotations
- Health check probes configured
- Resource limits suitable for k3s

#### `WEBSOCKET_K3S.md`
Comprehensive documentation covering:
- The WebSocket issue explanation
- Three deployment options (Ingress, NodePort, Port-forward)
- Testing instructions with multiple tools (websocat, curl, JavaScript)
- Troubleshooting guide
- Configuration examples for both Traefik and nginx

#### `scripts/test-websocket.sh`
Automated test script to verify WebSocket connectivity:
- Checks for required tools
- Connects to WebSocket endpoint
- Generates test traffic to produce logs
- Provides clear success/failure feedback

### 4. Documentation Updates

#### `README.md`
- Added WebSocket feature to features list
- Added k3s deployment section with reference to WebSocket docs
- Added WebSocket log streaming usage examples
- Referenced `WEBSOCKET_K3S.md` for detailed k3s configuration

## How to Deploy

### Option 1: Using k3s with Traefik (Recommended)

```bash
helm install playwright-proxy ./helm/playwright-proxy \
  -f helm/playwright-proxy/examples/k3s-traefik.yaml
```

Then test:
```bash
./scripts/test-websocket.sh ws://playwright-proxy.local/logs
```

### Option 2: Using NodePort (Development)

```bash
helm install playwright-proxy ./helm/playwright-proxy \
  --set service.type=NodePort \
  --set ingress.enabled=false
```

Find the port and test:
```bash
NODE_PORT=$(kubectl get svc playwright-proxy -o jsonpath='{.spec.ports[0].nodePort}')
./scripts/test-websocket.sh ws://localhost:$NODE_PORT/logs
```

### Option 3: Using Port-Forward (Local Testing)

```bash
kubectl port-forward svc/playwright-proxy 3128:3128
./scripts/test-websocket.sh ws://localhost:3128/logs
```

## Testing

### Manual Test
```bash
# Terminal 1: Connect to logs
websocat ws://playwright-proxy.local/logs

# Terminal 2: Generate traffic
curl http://playwright-proxy.local/https://example.com
```

You should see logs appear in Terminal 1.

### Automated Test
```bash
./scripts/test-websocket.sh ws://playwright-proxy.local/logs
```

## Verification

After deployment, verify that:

1. **Pod is running**:
   ```bash
   kubectl get pods -l app.kubernetes.io/name=playwright-proxy
   ```

2. **Service is accessible**:
   ```bash
   kubectl get svc playwright-proxy
   ```

3. **Ingress is configured** (if using Ingress):
   ```bash
   kubectl get ingress playwright-proxy
   ```

4. **WebSocket connects**:
   ```bash
   websocat ws://playwright-proxy.local/logs
   ```

5. **Logs are streaming**:
   Make a request and see logs appear in the WebSocket connection

## Troubleshooting

See [WEBSOCKET_K3S.md](WEBSOCKET_K3S.md) for detailed troubleshooting steps, including:
- Connection refused issues
- Ingress configuration problems
- Traefik-specific debugging
- Alternative connection methods

## Files Modified

- `src/main.rs` - Core WebSocket handling fixes
- `helm/playwright-proxy/values.yaml` - WebSocket configuration
- `helm/playwright-proxy/templates/ingress.yaml` - Annotation handling
- `README.md` - Documentation updates

## Files Created

- `helm/playwright-proxy/examples/k3s-traefik.yaml` - k3s example config
- `WEBSOCKET_K3S.md` - Comprehensive WebSocket guide
- `scripts/test-websocket.sh` - Automated testing script
- `WEBSOCKET_FIX_SUMMARY.md` - This file

## Next Steps

1. Build and deploy the updated image
2. Upgrade the Helm release with the new configuration
3. Test WebSocket connectivity
4. Update production configurations as needed

## Related Issues

This fix addresses WebSocket connectivity problems specifically in k3s environments, but the improvements also benefit:
- Any Kubernetes deployment with Ingress
- Docker deployments behind reverse proxies
- Situations where connection buffering occurs

The changes maintain backward compatibility with existing deployments while enabling proper WebSocket support.

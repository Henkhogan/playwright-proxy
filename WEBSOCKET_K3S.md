# WebSocket Support in k3s

This document explains how to enable WebSocket connections for the playwright-proxy when running in k3s.

## The Issue

When running playwright-proxy in k3s, WebSocket connections to `/logs` may fail because:

1. **Ingress controllers need WebSocket-specific configuration**: By default, some ingress controllers don't automatically handle WebSocket upgrade requests.
2. **Connection buffering**: Reverse proxies may buffer connections, breaking the WebSocket upgrade handshake.
3. **Timeout settings**: Default timeouts may be too short for long-lived WebSocket connections.

## Solution

The playwright-proxy now includes built-in WebSocket support with proper configuration for k3s and Traefik (the default ingress controller in k3s).

### Changes Made

1. **Code improvements** (`src/main.rs`):
   - Replaced `peek()` with proper `read()` to handle buffered connections from ingress
   - More reliable WebSocket detection (case-insensitive header matching)
   - Proper buffer handling for pre-read data

2. **Helm chart improvements**:
   - Added WebSocket-specific configuration in `values.yaml`
   - Updated Ingress template to include WebSocket annotations
   - Created k3s-specific example configuration

### Deployment Options

#### Option 1: Using Gateway API HTTPRoute (Recommended for Modern k3s)

Gateway API provides native WebSocket support without requiring special annotations:

```bash
helm install playwright-proxy ./helm/playwright-proxy \
  -f helm/playwright-proxy/examples/k3s-gateway-api.yaml
```

Then update the hostname in your `/etc/hosts` or DNS:
```
<k3s-node-ip> playwright-proxy.local
```

Test the WebSocket connection:
```bash
# Test health endpoint
curl http://playwright-proxy.local/health

# Test WebSocket logs (using websocat or similar tool)
websocat ws://playwright-proxy.local/logs
```

**Why Gateway API is better:**
- Native WebSocket support (no special annotations needed)
- Modern Kubernetes API (successor to Ingress)
- Better routing capabilities
- Protocol upgrades handled automatically

#### Option 2: Using Traefik Ingress

Use the provided k3s example configuration:

```bash
helm install playwright-proxy ./helm/playwright-proxy \
  -f helm/playwright-proxy/examples/k3s-traefik.yaml
```

Then update the hostname in your `/etc/hosts` or DNS:
```
<k3s-node-ip> playwright-proxy.local
```

Test the WebSocket connection:
```bash
# Test health endpoint
curl http://playwright-proxy.local/health

# Test WebSocket logs (using websocat or similar tool)
websocat ws://playwright-proxy.local/logs
```

#### Option 3: Using NodePort (Direct Access)

For development or testing without Ingress:

```bash
helm install playwright-proxy ./helm/playwright-proxy \
  --set service.type=NodePort \
  --set ingress.enabled=false
```

Find the NodePort:
```bash
kubectl get svc playwright-proxy
```

Connect directly:
```bash
# If NodePort is 31234
websocat ws://<k3s-node-ip>:31234/logs
```

#### Option 4: Using Port-Forward (Local Testing)

```bash
kubectl port-forward svc/playwright-proxy 3128:3128
```

Then connect to:
```bash
websocat ws://localhost:3128/logs
```

### Custom Values Configuration

#### Gateway API HTTPRoute (Recommended)

Gateway API provides better WebSocket support out of the box:

```yaml
# Disable Ingress, enable HTTPRoute
ingress:
  enabled: false

httproute:
  enabled: true
  gatewayName: traefik  # or your gateway name
  gatewayNamespace: traefik
  hostnames:
    - playwright-proxy.local
  websocket:
    enabled: true  # WebSocket support is automatic with Gateway API
```

#### Standard Ingress (Legacy)

If you need to customize the WebSocket configuration for standard Ingress, update your values file:

```yaml
ingress:
  enabled: true
  className: traefik
  websocket:
    enabled: true
    annotations:
      # Add custom annotations here if needed
      traefik.ingress.kubernetes.io/router.middlewares: ""
```

For nginx ingress controller (not default in k3s):
```yaml
ingress:
  enabled: true
  className: nginx
  websocket:
    enabled: true
    annotations:
      nginx.ingress.kubernetes.io/websocket-services: "playwright-proxy"
      nginx.ingress.kubernetes.io/proxy-read-timeout: "3600"
      nginx.ingress.kubernetes.io/proxy-send-timeout: "3600"
```

### Testing WebSocket Connection

#### Using websocat

Install websocat:
```bash
# Linux
wget https://github.com/vi/websocat/releases/latest/download/websocat.x86_64-unknown-linux-musl
chmod +x websocat.x86_64-unknown-linux-musl
sudo mv websocat.x86_64-unknown-linux-musl /usr/local/bin/websocat

# macOS
brew install websocat
```

Connect to logs:
```bash
websocat ws://playwright-proxy.local/logs
```

#### Using JavaScript/Browser

```html
<!DOCTYPE html>
<html>
<head>
    <title>Playwright Proxy Logs</title>
</head>
<body>
    <h1>Live Logs</h1>
    <div id="logs" style="font-family: monospace; white-space: pre-wrap;"></div>
    <script>
        const ws = new WebSocket('ws://playwright-proxy.local/logs');
        const logsDiv = document.getElementById('logs');
        
        ws.onopen = () => {
            console.log('Connected to log stream');
            logsDiv.innerHTML += 'Connected to log stream\n';
        };
        
        ws.onmessage = (event) => {
            logsDiv.innerHTML += event.data;
            logsDiv.scrollTop = logsDiv.scrollHeight;
        };
        
        ws.onerror = (error) => {
            console.error('WebSocket error:', error);
            logsDiv.innerHTML += 'Error: ' + error + '\n';
        };
        
        ws.onclose = () => {
            console.log('Disconnected from log stream');
            logsDiv.innerHTML += 'Disconnected from log stream\n';
        };
    </script>
</body>
</html>
```

#### Using curl (for upgrade request test)

```bash
curl -i -N \
  -H "Connection: Upgrade" \
  -H "Upgrade: websocket" \
  -H "Sec-WebSocket-Version: 13" \
  -H "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==" \
  http://playwright-proxy.local/logs
```

You should see:
```
HTTP/1.1 101 Switching Protocols
Upgrade: websocket
Connection: Upgrade
...
```

### Troubleshooting

#### WebSocket connection refused or times out

1. **Check pod logs**:
   ```bash
   kubectl logs -l app.kubernetes.io/name=playwright-proxy
   ```

2. **Verify service is running**:
   ```bash
   kubectl get pods -l app.kubernetes.io/name=playwright-proxy
   kubectl get svc playwright-proxy
   ```

3. **Test without Ingress** using port-forward:
   ```bash
   kubectl port-forward svc/playwright-proxy 3128:3128
   websocat ws://localhost:3128/logs
   ```
   
   If this works, the issue is with Ingress configuration.

#### Connection established but no logs appear

1. **Generate some logs** by making a proxy request:
   ```bash
   curl http://playwright-proxy.local/https://example.com
   ```

2. **Check if logging is working** in pod logs:
   ```bash
   kubectl logs -l app.kubernetes.io/name=playwright-proxy -f
   ```

#### 404 or 400 errors

1. **Verify the path** - it should be `/logs` not `/log` or other variations
2. **Check Ingress configuration**:
   ```bash
   kubectl describe ingress playwright-proxy
   kubectl get ingress playwright-proxy -o yaml
   ```

#### Gateway API HTTPRoute issues

1. **Check HTTPRoute status**:
   ```bash
   kubectl describe httproute playwright-proxy
   kubectl get httproute playwright-proxy -o yaml
   ```

2. **Verify Gateway is ready**:
   ```bash
   kubectl get gateway -n traefik
   kubectl describe gateway -n traefik
   ```

3. **Check if Gateway API CRDs are installed**:
   ```bash
   kubectl get crd | grep gateway
   # Should show: gateways.gateway.networking.k8s.io
   #              httproutes.gateway.networking.k8s.io
   ```

4. **Check Gateway logs** (if using Traefik):
   ```bash
   kubectl logs -n traefik -l app.kubernetes.io/name=traefik
   ```

#### Traefik-specific issues

1. **Check Traefik logs**:
   ```bash
   kubectl logs -n kube-system -l app.kubernetes.io/name=traefik
   # or
   kubectl logs -n traefik -l app.kubernetes.io/name=traefik
   ```

2. **Verify Traefik is handling routes**:
   ```bash
   # For Ingress
   kubectl get ingressroute -A
   
   # For Gateway API
   kubectl get httproute -A
   ```

### Additional Resources

- [Gateway API Documentation](https://gateway-api.sigs.k8s.io/)
- [Gateway API WebSocket Support](https://gateway-api.sigs.k8s.io/guides/http-routing/#websockets)
- [Traefik Gateway API Documentation](https://doc.traefik.io/traefik/routing/providers/kubernetes-gateway/)
- [Traefik WebSocket Documentation](https://doc.traefik.io/traefik/routing/routers/#websocket)
- [Kubernetes Ingress Documentation](https://kubernetes.io/docs/concepts/services-networking/ingress/)
- [k3s Traefik Documentation](https://docs.k3s.io/networking#traefik-ingress-controller)

### Support

If you continue to experience issues:

1. Check the pod logs for WebSocket connection attempts
2. Verify your gateway/ingress controller version and configuration
3. Try the NodePort or port-forward methods to isolate the issue
4. For Gateway API: Ensure CRDs are installed and gateway is ready
5. Open an issue on the GitHub repository with your configuration and logs

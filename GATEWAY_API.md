# Gateway API with WebSocket Support

This guide covers deploying playwright-proxy using the Kubernetes Gateway API with WebSocket support.

## Why Gateway API?

The Gateway API is the next-generation Kubernetes networking API and is superior to traditional Ingress:

- **Native WebSocket support**: Protocol upgrades (including WebSocket) are handled automatically
- **Better separation of concerns**: Clear roles between infrastructure provider and application developer
- **More expressive**: Supports more routing capabilities than Ingress
- **Type-safe**: Stronger validation and better error reporting
- **Future-proof**: Actively developed as the successor to Ingress API

## Prerequisites

### 1. Check if Gateway API is Installed

```bash
# Check for Gateway API CRDs
kubectl get crd | grep gateway

# You should see:
# gateways.gateway.networking.k8s.io
# httproutes.gateway.networking.k8s.io
# gatewayclasses.gateway.networking.k8s.io
```

### 2. Install Gateway API CRDs (if not present)

```bash
# Install the standard channel (recommended)
kubectl apply -f https://github.com/kubernetes-sigs/gateway-api/releases/download/v1.0.0/standard-install.yaml

# Or experimental channel (for cutting-edge features)
kubectl apply -f https://github.com/kubernetes-sigs/gateway-api/releases/download/v1.0.0/experimental-install.yaml
```

### 3. Verify Gateway Exists

k3s with Traefik should have a Gateway configured by default:

```bash
# Check for gateways
kubectl get gateway -A

# If using Traefik, check in traefik namespace
kubectl get gateway -n traefik

# Describe the gateway to see its configuration
kubectl describe gateway -n traefik
```

If no Gateway exists, you may need to create one or configure your Traefik installation to support Gateway API.

## Deployment

### Quick Start

```bash
# Deploy with Gateway API
helm install playwright-proxy ./helm/playwright-proxy \
  -f helm/playwright-proxy/examples/k3s-gateway-api.yaml

# Verify HTTPRoute is created
kubectl get httproute playwright-proxy

# Check status
kubectl describe httproute playwright-proxy
```

### Custom Configuration

Create your own values file:

```yaml
# my-gateway-values.yaml
replicaCount: 1

image:
  registry: ghcr.io
  repository: henkhogan/playwright-proxy
  tag: latest

service:
  type: ClusterIP
  port: 3128

# Disable Ingress
ingress:
  enabled: false

# Enable Gateway API HTTPRoute
httproute:
  enabled: true
  gatewayName: traefik  # Your gateway name
  gatewayNamespace: traefik  # Your gateway namespace
  
  # Optional: Target a specific listener/port
  # sectionName: websecure
  
  hostnames:
    - playwright-proxy.example.com
  
  # WebSocket support (enabled by default)
  websocket:
    enabled: true
  
  # Optional annotations
  annotations:
    # Add any custom annotations here
    # traefik.io/router.middlewares: my-middleware@kubernetescrd

resources:
  limits:
    cpu: 1000m
    memory: 1024Mi
  requests:
    cpu: 500m
    memory: 512Mi

env:
  - name: PROXY_PORT
    value: "3128"
```

Deploy:
```bash
helm install playwright-proxy ./helm/playwright-proxy \
  -f my-gateway-values.yaml
```

## WebSocket Support

### How it Works

Gateway API HTTPRoute **automatically supports WebSocket** through the HTTP/1.1 Upgrade mechanism. No special configuration is needed!

When a client sends a WebSocket upgrade request:
1. Client sends `GET /logs` with `Upgrade: websocket` header
2. Gateway API routes the request to the backend service
3. Backend (playwright-proxy) responds with `101 Switching Protocols`
4. Connection is upgraded to WebSocket
5. Bidirectional communication begins

### Configuration

WebSocket support is enabled by default in the HTTPRoute configuration:

```yaml
httproute:
  websocket:
    enabled: true  # This adds informational annotations
```

The `websocket.enabled` setting adds helpful annotations to the HTTPRoute, but **WebSocket support works even without it** because Gateway API handles protocol upgrades automatically.

## Testing

### 1. Verify Deployment

```bash
# Check pod
kubectl get pods -l app.kubernetes.io/name=playwright-proxy

# Check service
kubectl get svc playwright-proxy

# Check HTTPRoute
kubectl get httproute playwright-proxy

# Check HTTPRoute status and parents
kubectl describe httproute playwright-proxy
```

Look for the status section showing the parent gateway is accepted:
```yaml
Status:
  Parents:
    Conditions:
      Status: True
      Type: Accepted
      Reason: Accepted
```

### 2. Test HTTP Endpoint

```bash
# Add to /etc/hosts if needed
echo "<k3s-node-ip> playwright-proxy.local" | sudo tee -a /etc/hosts

# Test health endpoint
curl http://playwright-proxy.local/health
```

### 3. Test WebSocket Connection

Using websocat:
```bash
# Install websocat if needed
# Linux:
wget https://github.com/vi/websocat/releases/latest/download/websocat.x86_64-unknown-linux-musl
chmod +x websocat.x86_64-unknown-linux-musl
sudo mv websocat.x86_64-unknown-linux-musl /usr/local/bin/websocat

# macOS:
brew install websocat

# Connect to logs
websocat ws://playwright-proxy.local/logs
```

### 4. Generate Logs

In another terminal:
```bash
# Make a proxy request to generate logs
curl http://playwright-proxy.local/https://example.com
```

You should see logs appear in the websocat terminal.

## Troubleshooting

### HTTPRoute Not Working

**Check HTTPRoute status:**
```bash
kubectl describe httproute playwright-proxy
```

Look for the `Status` section. Common issues:

1. **Parent not accepted:**
   ```yaml
   Status:
     Parents:
       Conditions:
         Status: False
         Type: Accepted
         Reason: NoMatchingParent
   ```
   
   **Solution:** Check that `gatewayName` and `gatewayNamespace` match your Gateway:
   ```bash
   kubectl get gateway -A
   ```

2. **No matching listener:**
   ```yaml
   Status:
     Parents:
       Conditions:
         Status: False
         Type: Accepted
         Reason: NoMatchingListener
   ```
   
   **Solution:** The Gateway doesn't have a listener for your hostname/port. Check Gateway listeners:
   ```bash
   kubectl get gateway -n traefik -o yaml
   ```

### WebSocket Connection Fails

**1. Verify HTTPRoute is working for HTTP:**
```bash
curl http://playwright-proxy.local/health
```

If HTTP doesn't work, WebSocket won't work either. Fix HTTP routing first.

**2. Test WebSocket upgrade manually:**
```bash
curl -i -N \
  -H "Connection: Upgrade" \
  -H "Upgrade: websocket" \
  -H "Sec-WebSocket-Version: 13" \
  -H "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==" \
  http://playwright-proxy.local/logs
```

Should return:
```
HTTP/1.1 101 Switching Protocols
Upgrade: websocket
Connection: Upgrade
...
```

**3. Bypass Gateway API to test backend:**
```bash
# Port-forward directly to service
kubectl port-forward svc/playwright-proxy 3128:3128

# Test WebSocket
websocat ws://localhost:3128/logs
```

If this works, the issue is with Gateway API routing, not the backend.

**4. Check Gateway logs:**

For Traefik:
```bash
kubectl logs -n traefik -l app.kubernetes.io/name=traefik -f
```

Look for errors related to route configuration or WebSocket upgrades.

### Gateway API CRDs Not Found

```bash
# Error: no matches for kind "HTTPRoute"
# Solution: Install Gateway API CRDs

kubectl apply -f https://github.com/kubernetes-sigs/gateway-api/releases/download/v1.0.0/standard-install.yaml
```

### No Gateway Found

If `kubectl get gateway -A` returns nothing:

**Option 1:** Configure Traefik to use Gateway API

Check Traefik documentation for enabling Gateway API provider.

**Option 2:** Use standard Ingress instead

```bash
helm install playwright-proxy ./helm/playwright-proxy \
  -f helm/playwright-proxy/examples/k3s-traefik.yaml
```

## Comparison: Gateway API vs Ingress

| Feature | Gateway API HTTPRoute | Standard Ingress |
|---------|----------------------|------------------|
| WebSocket Support | ✅ Native (automatic) | ⚠️ Requires annotations |
| Configuration | Simpler | More annotations needed |
| Protocol Upgrades | ✅ Automatic | ⚠️ Controller-dependent |
| Future Support | ✅ Active development | ⚠️ Maintenance mode |
| Maturity | Standard (v1.0+) | Stable |
| Compatibility | Requires CRDs | Built-in to K8s |

**Recommendation:** Use Gateway API if your cluster supports it (k3s with recent Traefik versions). Otherwise, use standard Ingress.

## Migration from Ingress to Gateway API

If you're currently using Ingress:

1. **Install Gateway API CRDs** (if not present)
2. **Verify Gateway exists** and is configured
3. **Update values:**
   ```yaml
   ingress:
     enabled: false
   
   httproute:
     enabled: true
     gatewayName: traefik
     gatewayNamespace: traefik
     hostnames:
       - your-hostname.local
   ```
4. **Upgrade Helm release:**
   ```bash
   helm upgrade playwright-proxy ./helm/playwright-proxy \
     -f your-values.yaml
   ```
5. **Verify:** HTTPRoute should be created and working

## Advanced Configuration

### Multiple Hostnames

```yaml
httproute:
  enabled: true
  hostnames:
    - playwright-proxy.local
    - playwright-proxy.example.com
    - proxy.example.com
```

### Specific Gateway Listener

Target a specific listener (e.g., HTTPS only):

```yaml
httproute:
  enabled: true
  gatewayName: traefik
  gatewayNamespace: traefik
  sectionName: websecure  # Target the HTTPS listener only
  hostnames:
    - playwright-proxy.local
```

### Path-based Routing

The default configuration routes all paths (`/`) to the service. Gateway API supports more complex routing, but the current template uses a simple prefix match which is appropriate for this use case.

## Additional Resources

- [Gateway API Official Documentation](https://gateway-api.sigs.k8s.io/)
- [Gateway API HTTP Routing Guide](https://gateway-api.sigs.k8s.io/guides/http-routing/)
- [Gateway API WebSocket Support](https://gateway-api.sigs.k8s.io/guides/http-routing/#websockets)
- [Traefik Gateway API Provider](https://doc.traefik.io/traefik/routing/providers/kubernetes-gateway/)
- [k3s Gateway API](https://docs.k3s.io/networking)

## Summary

Gateway API provides the best WebSocket support for playwright-proxy on k3s:
- ✅ No special annotations needed
- ✅ Automatic protocol upgrade handling
- ✅ Cleaner configuration
- ✅ Future-proof API

Use the `k3s-gateway-api.yaml` example to get started quickly!

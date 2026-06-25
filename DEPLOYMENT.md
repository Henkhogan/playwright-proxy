# Kubernetes Deployment Guide

This guide provides comprehensive instructions for deploying Playwright Proxy to Kubernetes using Helm.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Quick Start](#quick-start)
- [Installation](#installation)
- [Configuration](#configuration)
- [Examples](#examples)
- [Monitoring](#monitoring)
- [Scaling](#scaling)
- [Troubleshooting](#troubleshooting)
- [Uninstalling](#uninstalling)

## Prerequisites

- Kubernetes cluster 1.19 or higher
- `kubectl` configured to access your cluster
- Helm 3.0 or higher installed
- Container registry credentials (for private registries)
- At least 2GB of free memory per Pod (for Playwright browser)

## Quick Start

The fastest way to deploy Playwright Proxy to Kubernetes:

```bash
# Add Helm repository (if applicable)
helm repo add playwright-proxy https://github.com/Henkhogan/playwright-proxy/releases/download/helm-latest/
helm repo update

# Install with default configuration
helm install playwright-proxy playwright-proxy/playwright-proxy \
  --namespace default \
  --create-namespace

# Verify the deployment
kubectl get pods -l app.kubernetes.io/name=playwright-proxy
kubectl get svc -l app.kubernetes.io/name=playwright-proxy
```

## Installation

### From local chart

```bash
# Clone the repository
git clone https://github.com/Henkhogan/playwright-proxy.git
cd playwright-proxy

# Install using local chart
helm install my-release ./helm/playwright-proxy \
  --namespace default \
  --create-namespace
```

### With custom namespace

```bash
helm install my-release ./helm/playwright-proxy \
  --namespace playwright-proxy \
  --create-namespace \
  --set replicaCount=3
```

### With custom values file

```bash
helm install my-release ./helm/playwright-proxy \
  --values custom-values.yaml
```

## Configuration

### Basic Configuration Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| `replicaCount` | `2` | Number of Pod replicas |
| `image.tag` | `latest` | Container image tag |
| `image.pullPolicy` | `IfNotPresent` | Image pull policy |
| `service.type` | `ClusterIP` | Kubernetes service type |
| `service.port` | `8000` | Service port |
| `resources.requests.cpu` | `500m` | CPU request per Pod |
| `resources.requests.memory` | `512Mi` | Memory request per Pod |
| `resources.limits.cpu` | `1000m` | CPU limit per Pod |
| `resources.limits.memory` | `1024Mi` | Memory limit per Pod |

### Updating Configuration

```bash
# Update via command line
helm upgrade my-release ./helm/playwright-proxy \
  --set replicaCount=5 \
  --set resources.limits.memory=2048Mi

# Update via values file
helm upgrade my-release ./helm/playwright-proxy \
  --values production-values.yaml
```

## Examples

### Example 1: Development Environment

Minimal configuration for development:

```bash
helm install playwright-proxy ./helm/playwright-proxy \
  --values ./helm/playwright-proxy/examples/development.yaml
```

**Features:**
- Single Pod replica
- NodePort service
- No autoscaling
- Minimal resource requests

### Example 2: Staging Environment

Balanced configuration with autoscaling:

```bash
helm install playwright-proxy ./helm/playwright-proxy \
  --values ./helm/playwright-proxy/examples/staging.yaml
```

**Features:**
- 2 Pod replicas
- ClusterIP service
- Ingress enabled
- Horizontal Pod Autoscaler (2-10 replicas)
- Health checks enabled

### Example 3: Production Environment

High-availability configuration:

```bash
helm install playwright-proxy ./helm/playwright-proxy \
  --values ./helm/playwright-proxy/examples/production.yaml
```

**Features:**
- 3 Pod replicas
- ClusterIP service
- Ingress with TLS
- Horizontal Pod Autoscaler (3-20 replicas)
- Pod anti-affinity for distribution
- Node affinity for compute-intensive nodes
- Comprehensive monitoring annotations

### Example 4: Custom Configuration

```bash
helm install playwright-proxy ./helm/playwright-proxy \
  --set ingress.enabled=true \
  --set ingress.className=nginx \
  --set ingress.hosts[0].host=proxy.example.com \
  --set ingress.hosts[0].paths[0].path=/ \
  --set ingress.hosts[0].paths[0].pathType=Prefix \
  --set autoscaling.enabled=true \
  --set autoscaling.minReplicas=2 \
  --set autoscaling.maxReplicas=10 \
  --set autoscaling.targetCPUUtilizationPercentage=70
```

## Monitoring

### Health Checks

The Helm chart includes liveness and readiness probes:

```yaml
livenessProbe:
  enabled: true
  httpGet:
    path: /health
    port: 8000
  initialDelaySeconds: 30
  periodSeconds: 10
  failureThreshold: 3

readinessProbe:
  enabled: true
  httpGet:
    path: /health
    port: 8000
  initialDelaySeconds: 10
  periodSeconds: 5
  failureThreshold: 3
```

### Viewing Logs

```bash
# View logs from a specific Pod
kubectl logs -l app.kubernetes.io/name=playwright-proxy -f

# View logs from a specific deployment
kubectl logs deployment/playwright-proxy -f

# View logs from the last 100 lines
kubectl logs -l app.kubernetes.io/name=playwright-proxy --tail=100
```

### Accessing the Service

```bash
# Port-forward for local testing
kubectl port-forward svc/playwright-proxy 8000:8000

# From another terminal, test the proxy
curl --proxy "http://localhost:8000" https://example.com
```

## Scaling

### Manual Scaling

```bash
# Scale to a specific number of replicas
kubectl scale deployment playwright-proxy --replicas=5

# Or via Helm
helm upgrade my-release ./helm/playwright-proxy --set replicaCount=5
```

### Autoscaling

Enable Horizontal Pod Autoscaler:

```bash
helm install playwright-proxy ./helm/playwright-proxy \
  --set autoscaling.enabled=true \
  --set autoscaling.minReplicas=2 \
  --set autoscaling.maxReplicas=20 \
  --set autoscaling.targetCPUUtilizationPercentage=70
```

### Monitoring Autoscaling

```bash
# View HPA status
kubectl get hpa

# Describe HPA for detailed information
kubectl describe hpa playwright-proxy

# Watch HPA in action
kubectl get hpa -w
```

## Troubleshooting

### Pod not starting

```bash
# Check Pod status
kubectl describe pod -l app.kubernetes.io/name=playwright-proxy

# Check logs
kubectl logs -l app.kubernetes.io/name=playwright-proxy

# Check events
kubectl get events --sort-by='.lastTimestamp'
```

### ImagePullBackOff error

```bash
# Check if the image exists
kubectl describe pod <pod-name>

# Create image pull secret if using private registry
kubectl create secret docker-registry regcred \
  --docker-server=ghcr.io \
  --docker-username=<username> \
  --docker-password=<token> \
  --docker-email=<email>

# Add to values
helm install playwright-proxy ./helm/playwright-proxy \
  --set imagePullSecrets[0].name=regcred
```

### Out of Memory errors

```bash
# Increase memory limits
helm upgrade my-release ./helm/playwright-proxy \
  --set resources.limits.memory=2048Mi \
  --set resources.requests.memory=1536Mi

# Monitor memory usage
kubectl top pods -l app.kubernetes.io/name=playwright-proxy
```

### Connection timeout

```bash
# Check if service is accessible
kubectl get svc playwright-proxy

# Test connectivity from another Pod
kubectl run -it debug --image=curlimages/curl --restart=Never -- \
  curl http://playwright-proxy:8000
```

## Uninstalling

```bash
# Uninstall the release
helm uninstall my-release

# Uninstall from specific namespace
helm uninstall my-release --namespace playwright-proxy

# Verify uninstallation
helm list
kubectl get pods -l app.kubernetes.io/name=playwright-proxy
```

## Advanced Configuration

### Using a custom image repository

```bash
helm install playwright-proxy ./helm/playwright-proxy \
  --set image.registry=docker.io \
  --set image.repository=myorg/playwright-proxy \
  --set image.tag=v1.0.0
```

### Adding environment variables

```bash
helm install playwright-proxy ./helm/playwright-proxy \
  --set env.PROXY_PORT=9000 \
  --set env.LOG_LEVEL=debug
```

### Node affinity and tolerations

```bash
helm install playwright-proxy ./helm/playwright-proxy \
  --set nodeSelector.workload=compute-intensive \
  --set tolerations[0].key=compute-intensive \
  --set tolerations[0].operator=Equal \
  --set tolerations[0].value=true \
  --set tolerations[0].effect=NoSchedule
```

## Resources and Best Practices

### Resource Recommendations

**Minimum:**
- CPU: 250m per Pod
- Memory: 512Mi per Pod

**Recommended:**
- CPU: 500m-1000m per Pod
- Memory: 512Mi-1024Mi per Pod

**High Load:**
- CPU: 1000m-2000m per Pod
- Memory: 1024Mi-2048Mi per Pod

### Best Practices

1. **Set resource requests and limits** to ensure fair resource distribution
2. **Enable autoscaling** for production environments
3. **Use readiness and liveness probes** for better reliability
4. **Set pod anti-affinity** to distribute Pods across nodes
5. **Use ingress** for external access instead of NodePort
6. **Monitor metrics** using Prometheus or similar tools
7. **Use separate namespaces** for different environments
8. **Enable RBAC** for security
9. **Regularly update** the container image
10. **Test configuration changes** in staging before production

## Support

For issues or questions:
- Check the [main README](../README.md)
- Review the [chart README](./README.md)
- Open an issue on [GitHub](https://github.com/Henkhogan/playwright-proxy/issues)

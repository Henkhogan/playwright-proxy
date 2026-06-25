# Playwright Proxy Helm Chart

This Helm chart deploys the Playwright Proxy application to a Kubernetes cluster.

## Prerequisites

- Kubernetes 1.19+
- Helm 3.0+

## Installation

### Add the Helm repository (if applicable)

```bash
helm repo add playwright-proxy https://example.com/charts
helm repo update
```

### Install the chart with default values

```bash
helm install my-release ./helm/playwright-proxy
```

### Install with custom values

```bash
helm install my-release ./helm/playwright-proxy -f values.yaml
```

## Configuration

The following table lists the configurable parameters of the Playwright Proxy chart and their default values:

| Parameter | Description | Default |
|-----------|-------------|---------|
| `replicaCount` | Number of replicas | `2` |
| `image.registry` | Image registry | `ghcr.io` |
| `image.repository` | Image repository | `henkhogan/playwright-proxy` |
| `image.tag` | Image tag | `latest` |
| `image.pullPolicy` | Image pull policy | `IfNotPresent` |
| `service.type` | Kubernetes service type | `ClusterIP` |
| `service.port` | Kubernetes service port | `8000` |
| `ingress.enabled` | Enable ingress | `false` |
| `autoscaling.enabled` | Enable HPA | `false` |
| `autoscaling.minReplicas` | Minimum replicas for HPA | `2` |
| `autoscaling.maxReplicas` | Maximum replicas for HPA | `10` |
| `resources.limits.cpu` | CPU limit | `1000m` |
| `resources.limits.memory` | Memory limit | `1024Mi` |
| `resources.requests.cpu` | CPU request | `500m` |
| `resources.requests.memory` | Memory request | `512Mi` |

## Examples

### Deploy with autoscaling enabled

```bash
helm install my-release ./helm/playwright-proxy \
  --set autoscaling.enabled=true \
  --set autoscaling.minReplicas=2 \
  --set autoscaling.maxReplicas=10
```

### Deploy with ingress

```bash
helm install my-release ./helm/playwright-proxy \
  --set ingress.enabled=true \
  --set ingress.className=nginx \
  --set ingress.hosts[0].host=playwright-proxy.example.com \
  --set ingress.hosts[0].paths[0].path=/ \
  --set ingress.hosts[0].paths[0].pathType=Prefix
```

### Deploy with custom image tag

```bash
helm install my-release ./helm/playwright-proxy \
  --set image.tag=v0.2.0
```

## Uninstall

```bash
helm uninstall my-release
```

## Upgrade

```bash
helm upgrade my-release ./helm/playwright-proxy
```

## License

MIT

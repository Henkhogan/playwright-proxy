# Changelog

## [0.3.0] - 2026-07-06

### Added
- **Gateway API HTTPRoute support** for modern Kubernetes networking
  - Native WebSocket support without special configuration
  - Better routing capabilities than traditional Ingress
  - New example configuration: `examples/k3s-gateway-api.yaml`
- **Comprehensive WebSocket documentation**
  - `GATEWAY_API.md`: Complete guide for Gateway API deployment
  - `WEBSOCKET_K3S.md`: Detailed WebSocket configuration and troubleshooting
  - `QUICKSTART_K3S_WEBSOCKET.md`: Quick start guide for k3s users
- **WebSocket configuration in values.yaml**
  - Support for both Ingress and HTTPRoute
  - Automatic annotation injection for WebSocket support
  - Traefik and nginx ingress controller compatibility

### Fixed
- **WebSocket connectivity through k3s Ingress**
  - Replaced `peek()` with proper `read()` for reliable connection handling
  - Added buffer management for pre-read data
  - Case-insensitive WebSocket upgrade detection
  - Proper handling of connections through reverse proxies
- **WebSocket handshake completion error**
  - Implemented `BufferedStream` wrapper to handle pre-read data
  - Fixed "Handshake not finished" protocol error
  - WebSocket upgrade now works correctly after protocol detection

### Changed
- Updated Helm chart to support both Ingress and Gateway API configurations
- Improved HTTPRoute template with WebSocket annotations
- Enhanced README with Gateway API deployment options

### Documentation
- Added `GATEWAY_API.md` - Complete Gateway API guide
- Added `WEBSOCKET_K3S.md` - WebSocket troubleshooting guide
- Added `QUICKSTART_K3S_WEBSOCKET.md` - Quick start reference
- Added `WEBSOCKET_FIX_SUMMARY.md` - Technical change summary
- Updated README.md with WebSocket features and deployment options

## [0.2.0] - Previous Release

- Initial Helm chart implementation
- Basic Ingress support
- Health check endpoints
- Docker image configuration

## [0.1.0] - Initial Release

- Core Playwright proxy functionality
- HTTP/HTTPS proxy support
- JavaScript rendering
- Basic Kubernetes deployment

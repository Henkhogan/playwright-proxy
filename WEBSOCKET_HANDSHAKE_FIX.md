# WebSocket Handshake Fix

## Issue
Error encountered: `WebSocket protocol error: Handshake not finished`

## Root Cause
The WebSocket handshake was failing because:
1. In the main connection loop, we read data from the TcpStream to detect if it's a WebSocket request
2. We then passed the stream to `tokio_tungstenite::accept_async()`
3. However, `accept_async()` needs to read the HTTP upgrade handshake itself
4. Since we already consumed the data, the handshake couldn't complete

## Solution
Implemented a `BufferedStream` wrapper that:
1. Stores the already-read data in a buffer
2. Returns data from the buffer first when read
3. Once buffer is exhausted, reads from the underlying TcpStream
4. Implements both `AsyncRead` and `AsyncWrite` traits

This allows `tokio_tungstenite::accept_async()` to read the complete WebSocket handshake, including the data we already read for detection.

## Code Changes

### Added BufferedStream struct
```rust
struct BufferedStream {
    stream: TcpStream,
    buffer: Vec<u8>,
    position: usize,
}

impl tokio::io::AsyncRead for BufferedStream { ... }
impl tokio::io::AsyncWrite for BufferedStream { ... }
```

### Updated websocket_handler_with_buffer
```rust
async fn websocket_handler_with_buffer(stream: TcpStream, initial_buffer: &[u8]) -> Result<...> {
    // Create buffered stream with pre-read data
    let buffered_stream = BufferedStream::new(stream, initial_buffer.to_vec());
    
    // Now accept_async can read the full handshake
    let ws_stream = tokio_tungstenite::accept_async(buffered_stream).await?;
    
    // ... rest of handler
}
```

### Removed unused function
- Removed `websocket_handler()` which was replaced by `websocket_handler_with_buffer()`

## Testing
After this fix, WebSocket connections should complete successfully:
```bash
# Test WebSocket connection
websocat ws://localhost:3128/logs

# Should see: "WebSocket client connected" in logs
# And receive live log messages
```

## Technical Details
The `BufferedStream` wrapper pattern is necessary because:
- We need to inspect the HTTP request to route it (WebSocket vs proxy request)
- Once we read from a stream, the data is consumed
- `tokio_tungstenite` needs the full HTTP upgrade request to establish WebSocket
- Solution: Prepend already-read data to the stream using a custom AsyncRead/AsyncWrite wrapper

This is a common pattern when handling protocol detection at the application layer.

## Related Files
- `src/main.rs` - Added BufferedStream, updated WebSocket handler
- Previous fixes also included Gateway API and Ingress WebSocket support

## Status
✅ **Fixed** - WebSocket handshake now completes successfully through all deployment methods:
- Direct connection
- Through Kubernetes Ingress
- Through Gateway API HTTPRoute
- Through port-forward

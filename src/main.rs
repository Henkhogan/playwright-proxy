use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::net::SocketAddr;
use std::io;
use std::sync::Arc;
use rcgen::generate_simple_self_signed;
use tokio_rustls::{TlsAcceptor, rustls::ServerConfig};
use tokio_rustls::rustls::pki_types::CertificateDer;
use playwright::Playwright;
use tokio::sync::broadcast;
use futures::{StreamExt, SinkExt};
use tracing::{info, warn, error};
use tracing_subscriber::fmt::MakeWriter;
use std::io::Write;

// Custom writer that broadcasts to WebSocket and writes to stderr
struct BroadcastWriter {
    tx: broadcast::Sender<String>,
}

impl Write for BroadcastWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if let Ok(s) = std::str::from_utf8(buf) {
            let _ = self.tx.send(s.to_string());
            io::stderr().write(buf)
        } else {
            Ok(buf.len())
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        io::stderr().flush()
    }
}

impl<'a> MakeWriter<'a> for BroadcastWriter {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        BroadcastWriter {
            tx: self.tx.clone(),
        }
    }
}

// Global state for Playwright
lazy_static::lazy_static! {
    static ref PLAYWRIGHT: tokio::sync::Mutex<Option<Playwright>> = tokio::sync::Mutex::new(None);
    static ref PROXY_READY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    static ref LOG_TX: broadcast::Sender<String> = {
        let (tx, _) = broadcast::channel(100);
        tx
    };
}

// Generate a wildcard certificate for intercepting HTTPS - returns (cert_der, key_der)
fn generate_wildcard_cert() -> (Vec<u8>, Vec<u8>) {
    let cert_key = generate_simple_self_signed(vec!["*.example.com".to_string()])
        .expect("Failed to generate certificate");
    
    // Get DER format directly
    let cert_der = cert_key.cert.der().to_vec();
    let key_der = cert_key.key_pair.serialize_der();
    
    (cert_der, key_der)
}

// Parse HTTP request line
fn parse_http_request(buffer: &[u8]) -> Option<(String, String, String)> {
    let request = String::from_utf8_lossy(buffer);
    let lines: Vec<&str> = request.lines().collect();
    if lines.is_empty() {
        return None;
    }
    
    let parts: Vec<&str> = lines[0].split_whitespace().collect();
    if parts.len() < 3 {
        return None;
    }
    
    Some((
        parts[0].to_string(),      // method
        parts[1].to_string(),      // path
        parts[2].to_string(),      // version
    ))
}

async fn handle_client(mut client_stream: TcpStream, cert_pem: Vec<u8>, key_pem: Vec<u8>) -> io::Result<()> {
    // Read the first line to determine if it's a health check or proxy request
    let mut buffer = vec![0; 4096];
    let n = client_stream.read(&mut buffer).await?;
    
    if let Some((method, target, _)) = parse_http_request(&buffer[..n]) {
        info!("Received request: {} {}", method, target);
        
        // Handle health check requests
        if (method == "GET" || method == "HEAD") && target == "/health" {
            let status = if PROXY_READY.load(std::sync::atomic::Ordering::SeqCst) {
                "200 OK"
            } else {
                "503 Service Unavailable"
            };
            
            info!("Health check: {}", status);
            
            let html = if status.starts_with("200") {
                "<html><body><h1>OK</h1></body></html>".to_string()
            } else {
                "<html><body><h1>NOT_READY</h1></body></html>".to_string()
            };
            
            let response = format!(
                "HTTP/1.1 {}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n\r\n{}",
                status,
                html.len(),
                html
            );
            client_stream.write_all(response.as_bytes()).await?;
            return Ok(());
        }
        
        if method == "CONNECT" {
            // Parse the host and port from CONNECT request
            let (host, _port) = if let Some(colon_pos) = target.rfind(':') {
                (target[..colon_pos].to_string(), target[colon_pos + 1..].parse::<u16>().unwrap_or(443))
            } else {
                (target.clone(), 443)
            };
            
            info!("CONNECT request to: {}", host);
            
            // Send 200 Connection Established response
            let response = "HTTP/1.1 200 Connection Established\r\n\r\n";
            client_stream.write_all(response.as_bytes()).await?;
            
            // Set up TLS with the generated certificate (already in DER format)
            let cert_der = CertificateDer::from(cert_pem);
            
            // key_pem is already in DER format from generate_wildcard_cert
            let key_der = rustls::pki_types::PrivateKeyDer::Pkcs8(
                rustls::pki_types::PrivatePkcs8KeyDer::from(key_pem)
            );
            
            let config = ServerConfig::builder()
                .with_no_client_auth()
                .with_single_cert(vec![cert_der], key_der)
                .expect("Failed to create TLS config");
            
            let acceptor = TlsAcceptor::from(Arc::new(config));
            
            // Upgrade connection to TLS
            let tls_stream = acceptor.accept(client_stream).await?;
            
            // Now read the actual HTTP request over TLS
            let (mut reader, mut writer) = tokio::io::split(tls_stream);
            let mut tls_buffer = vec![0; 4096];
            let tls_n = reader.read(&mut tls_buffer).await?;
            
            if let Some((_, http_path, _)) = parse_http_request(&tls_buffer[..tls_n]) {
                // Construct the full target URL
                let target_url = if http_path.starts_with("http") {
                    http_path
                } else {
                    format!("https://{}{}", host, http_path)
                };
                
                info!("Rendering page: {}", target_url);
                
                // Fetch and render the page with Playwright
                match render_page(&target_url).await {
                    Ok(html) => {
                        info!("Successfully rendered: {} ({} bytes)", target_url, html.len());
                        let response = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
                            html.len(),
                            html
                        );
                        let _ = writer.write_all(response.as_bytes()).await;
                    }
                    Err(e) => {
                        let response = format!("HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\n\r\n");
                        let _ = writer.write_all(response.as_bytes()).await;
                        error!("Rendering error for {}: {}", target_url, e);
                    }
                }
            }
        } else if method == "GET" || method == "POST" || method == "HEAD" {
            // Handle direct HTTP requests with target URL as path
            // e.g., GET /https://www.example.com/path HTTP/1.1
            let target_url = if target.starts_with('/') {
                target[1..].to_string()  // Remove leading /
            } else {
                target
            };
            
            // Only process if it looks like a URL
            if target_url.starts_with("http://") || target_url.starts_with("https://") {
                info!("Direct proxy request: {}", target_url);
                
                match render_page(&target_url).await {
                    Ok(html) => {
                        info!("Successfully rendered: {} ({} bytes)", target_url, html.len());
                        let response = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
                            html.len(),
                            html
                        );
                        let _ = client_stream.write_all(response.as_bytes()).await;
                    }
                    Err(e) => {
                        let response = format!("HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\n\r\n");
                        let _ = client_stream.write_all(response.as_bytes()).await;
                        error!("Rendering error for {}: {}", target_url, e);
                    }
                }
            } else {
                warn!("Invalid URL format: {}", target_url);
                // Invalid request
                let response = "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html\r\nContent-Length: 46\r\n\r\n<html><body>Invalid URL format</body></html>";
                let _ = client_stream.write_all(response.as_bytes()).await;
            }
        }
    }
    
    Ok(())
}

async fn render_page(url: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    info!("Starting browser for: {}", url);
    
    // Initialize Playwright if needed
    let mut pw_guard = PLAYWRIGHT.lock().await;
    if pw_guard.is_none() {
        info!("Initializing Playwright...");
        let pw = Playwright::initialize().await?;
        pw.prepare()?;
        *pw_guard = Some(pw);
        info!("Playwright initialized successfully");
    }
    
    let pw = pw_guard.as_ref().unwrap();
    
    // Launch browser
    info!("Launching Chromium browser...");
    let browser = pw
        .chromium()
        .launcher()
        .headless(true)
        .launch()
        .await?;
    
    // Create context and page
    let context = browser.context_builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .build()
        .await?;
    
    let page = context.new_page().await?;
    info!("Navigating to: {}", url);
    
    // Navigate to URL
    page.goto_builder(url)
        .timeout(60000.0)
        .goto()
        .await?;
    
    info!("Waiting for content to load...");
    // Wait for content to load
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    
    info!("Extracting HTML content...");
    // Get rendered HTML
    let html = page.content().await?;
    
    // Close context
    context.close().await?;
    info!("Page rendering complete: {} characters", html.len());
    
    Ok(html)
}

// WebSocket handler for log streaming
async fn websocket_handler(stream: TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .map_err(|e| format!("WebSocket error: {}", e))?;
    
    info!("WebSocket client connected");
    
    let mut rx = LOG_TX.subscribe();
    let (mut ws_sender, _ws_receiver) = ws_stream.split();
    
    loop {
        match rx.recv().await {
            Ok(msg) => {
                let ws_msg = tokio_tungstenite::tungstenite::Message::Text(msg);
                if let Err(e) = ws_sender.send(ws_msg).await {
                    error!("Failed to send WebSocket message: {}", e);
                    break;
                }
            }
            Err(_) => {
                // Sender has been dropped
                break;
            }
        }
    }
    
    info!("WebSocket client disconnected");
    Ok(())
}

#[tokio::main]
async fn main() -> io::Result<()> {
    // Initialize tracing subscriber with BroadcastWriter
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(false)
        .with_level(true)
        .with_writer(BroadcastWriter { tx: LOG_TX.clone() })
        .init();
    
    info!("Starting Playwright Proxy...");
    
    // Generate wildcard certificate (in DER format)
    info!("Generating wildcard certificate...");
    let (cert_der, key_der) = generate_wildcard_cert();
    info!("Certificate generated successfully");
    
    // Parse port from command-line argument or environment variable, default to 3128
    let port = std::env::args()
        .nth(1)
        .or_else(|| std::env::var("PROXY_PORT").ok())
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(3128);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(addr).await?;
    
    info!("Proxy and health check listening on {}", addr);
    println!("Proxy and health check listening on {}", addr);
    println!("  Health check: http://localhost:{}/health", port);
    println!("  Direct proxy: http://localhost:{}/https://example.com", port);
    println!("  CONNECT proxy: curl --proxy http://localhost:{} https://example.com", port);
    println!("  WebSocket logs: ws://localhost:{}/logs", port);
    
    // Mark proxy as ready
    PROXY_READY.store(true, std::sync::atomic::Ordering::SeqCst);
    info!("Proxy is ready to accept connections");
    
    loop {
        let (client_stream, _) = listener.accept().await?;
        let cert = cert_der.clone();
        let key = key_der.clone();
        
        tokio::spawn(async move {
            // Try to detect WebSocket upgrade request
            let mut buffer = vec![0; 4096];
            let n = match client_stream.peek(&mut buffer).await {
                Ok(n) => n,
                Err(_) => 0,
            };
            
            let is_websocket = n > 0 && {
                let request_str = String::from_utf8_lossy(&buffer[..n.min(1024)]);
                request_str.contains("GET /logs") && request_str.contains("Upgrade: websocket")
            };
            
            if is_websocket {
                // Handle WebSocket connection
                if let Err(e) = websocket_handler(client_stream).await {
                    error!("WebSocket error: {}", e);
                }
            } else {
                // Handle proxy request
                if let Err(e) = handle_client(client_stream, cert, key).await {
                    error!("Error handling client: {}", e);
                }
            }
        });
    }
}

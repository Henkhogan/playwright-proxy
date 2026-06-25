use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::net::SocketAddr;
use std::io;
use std::sync::Arc;
use rcgen::generate_simple_self_signed;
use tokio_rustls::{TlsAcceptor, rustls::ServerConfig};
use tokio_rustls::rustls::pki_types::CertificateDer;
use playwright::Playwright;

// Global state for Playwright
lazy_static::lazy_static! {
    static ref PLAYWRIGHT: tokio::sync::Mutex<Option<Playwright>> = tokio::sync::Mutex::new(None);
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
    // Read the first line to determine if it's a CONNECT request
    let mut buffer = vec![0; 4096];
    let n = client_stream.read(&mut buffer).await?;
    
    if let Some((method, target, _)) = parse_http_request(&buffer[..n]) {
        if method == "CONNECT" {
            // Parse the host and port from CONNECT request
            let (host, _port) = if let Some(colon_pos) = target.rfind(':') {
                (target[..colon_pos].to_string(), target[colon_pos + 1..].parse::<u16>().unwrap_or(443))
            } else {
                (target.clone(), 443)
            };
            
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
                
                // Fetch and render the page with Playwright
                match render_page(&target_url).await {
                    Ok(html) => {
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
                        eprintln!("Rendering error: {}", e);
                    }
                }
            }
        }
    }
    
    Ok(())
}

async fn render_page(url: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // Initialize Playwright if needed
    let mut pw_guard = PLAYWRIGHT.lock().await;
    if pw_guard.is_none() {
        let pw = Playwright::initialize().await?;
        pw.prepare()?;
        *pw_guard = Some(pw);
    }
    
    let pw = pw_guard.as_ref().unwrap();
    
    // Launch browser
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
    
    // Navigate to URL
    page.goto_builder(url)
        .timeout(60000.0)
        .goto()
        .await?;
    
    // Wait for content to load
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    
    // Get rendered HTML
    let html = page.content().await?;
    
    // Close context
    context.close().await?;
    
    Ok(html)
}

#[tokio::main]
async fn main() -> io::Result<()> {
    // Generate wildcard certificate (in DER format)
    let (cert_der, key_der) = generate_wildcard_cert();
    
    // Parse port from command-line argument or environment variable, default to 9000
    let port = std::env::args()
        .nth(1)
        .or_else(|| std::env::var("PROXY_PORT").ok())
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(9000);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(addr).await?;
    
    println!("HTTPS Intercepting Proxy listening on {}", addr);
    println!("Usage: curl --proxy http://localhost:{} https://example.com", port);
    
    loop {
        let (client_stream, _) = listener.accept().await?;
        let cert = cert_der.clone();
        let key = key_der.clone();
        
        tokio::spawn(async move {
            if let Err(e) = handle_client(client_stream, cert, key).await {
                eprintln!("Error handling client: {}", e);
            }
        });
    }
}

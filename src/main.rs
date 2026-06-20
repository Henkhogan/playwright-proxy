use axum::{
    Router, Extension, 
    http::{StatusCode, HeaderMap, Uri},
    response::IntoResponse,
};
use playwright::Playwright;
use std::net::SocketAddr;
use std::sync::Arc;
use std::collections::HashMap;
use std::env;

// Global shared state containing our launched browser instance
struct AppState {
    browser: playwright::api::Browser,
}

async fn proxy_handler(
    Extension(state): Extension<Arc<AppState>>,
    headers: HeaderMap,
    uri: Uri,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Extract the target URL from the request
    // In HTTP proxy mode, the request line contains the full URL
    // e.g., GET http://www.google.com/ HTTP/1.1
    let uri_str = uri.to_string();
    
    // Remove any leading slash and decode the URL
    let path_str = if uri_str.starts_with("/") {
        &uri_str[1..]
    } else {
        &uri_str
    };
    
    let target_url = if path_str.starts_with("http://") || path_str.starts_with("https://") {
        // Already a full URL
        path_str.to_string()
    } else if !path_str.is_empty() {
        // Assume it's a domain, add https:// prefix
        format!("https://{}", path_str)
    } else {
        return Err((StatusCode::BAD_REQUEST, "Invalid target URL".to_string()));
    };

    // Extract playwright-prefixed headers and parse them
    let mut _playwright_options = HashMap::new();
    for (key, value) in headers.iter() {
        if let Some(key_str) = key.as_str().strip_prefix("playwright-") {
            if let Ok(value_str) = value.to_str() {
                _playwright_options.insert(key_str.to_string(), value_str.to_string());
            }
        }
    }
    
    // 1. Create a new browser context with proper headers
    let mut context_builder = state.browser.context_builder();
    
    // Set a realistic user-agent to avoid being blocked by sites like Instagram
    let user_agent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
    context_builder = context_builder.user_agent(user_agent);
    
    let context = context_builder
        .build()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create context: {}", e)))?;
    
    // 2. Open a new page within that context
    let page = context.new_page()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to open page: {}", e)))?;

    // 3. Navigate to the target URL with extended timeout for sites like Instagram
    page.goto_builder(&target_url)
        .timeout(60000.0) // 60 second timeout
        .goto()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Navigation failed: {}", e)))?;

    // 4. Wait for any dynamic content to load (Instagram might load content dynamically)
    // Add a delay to ensure all JavaScript-rendered content is visible
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // 5. Extract the fully executed and rendered DOM tree
    let html_content = page.content()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to fetch page source: {}", e)))?;

    // 6. Close the context (which also closes pages) to release resources
    let _ = context.close().await;

    Ok(html_content)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // Parse port from command-line argument or environment variable, default to 8000
    let port = env::args()
        .nth(1)
        .or_else(|| env::var("PROXY_PORT").ok())
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(8000);

    println!("Initializing Playwright engine...");
    // Initialize the core Playwright driver system
    let playwright = Playwright::initialize().await.expect("Failed to initialize Playwright");
    
    // Ensure the Chromium binaries are present
    playwright.prepare().expect("Failed to install browser binaries");

    // Launch a single background Chromium process
    let browser = playwright
        .chromium()
        .launcher()
        .headless(true)
        .launch()
        .await
        .expect("Failed to launch Chromium instance");

    let shared_state = Arc::new(AppState { browser });

    // Build router to catch all requests and proxy them
    let app = Router::new()
        .fallback(proxy_handler)
        .layer(Extension(shared_state));

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("Playwright proxy listening on {}", addr);
    println!("Usage: http://localhost:{port}/<target_url>");
    println!("Example: http://localhost:{port}/https://example.com");
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

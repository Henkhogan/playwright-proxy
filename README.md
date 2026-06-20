This project is supposed to provide a web proxy that renders the destination website wit playwright and returns the rendered html. 

Any specific arguments are supposed to be passed as headers prefixed with "playwright-" which are stripped before the request is send to the destination

## Features

- **Full JavaScript rendering**: Executes all JavaScript on the page before returning HTML
- **HTTP/HTTPS proxy**: Works as a standard HTTP proxy compatible with `curl`, `wget`, and other tools
- **Configurable port**: Use command-line arguments or environment variables
- **Extended timeouts**: Handles slow-loading sites with 60-second navigation timeout
- **Realistic browser**: Uses a Chrome user-agent to avoid being blocked by anti-bot systems

## Prerequisites

- Docker (for containerized deployment)
- Rust 1.96+ (for local development)

## Building

### Local build:
```bash
cargo build --release
```

### Docker build:
```bash
docker build -t playwright-proxy .
```

## Configuration

The proxy port can be configured using either:
- **Command-line argument**: `./playwright-proxy 3000`
- **Environment variable**: `PROXY_PORT=3000 ./playwright-proxy`
- **Default**: 8000 (if no argument or environment variable is provided)

## Usage

### Starting the proxy

Local execution:
```bash
./playwright-proxy 9000
```

Docker execution:
```bash
docker run -p 9000:9000 playwright-proxy 9000
```

### Making requests

Use as an HTTP proxy with curl:
```bash
# Render a website with JavaScript executed
curl --proxy "http://localhost:9000" http://www.google.com

# HTTPS URLs
curl --proxy "http://localhost:9000" https://example.com

# Instagram (note: may have limitations due to anti-bot measures)
curl --proxy "http://localhost:9000" https://www.instagram.com/username
```

You can also use it with other tools that support HTTP proxies:
```bash
# wget with proxy
wget -e use_proxy=yes -e http_proxy=http://localhost:9000 http://www.google.com

# Python requests
python -c "import requests; print(requests.get('http://www.google.com', proxies={'http': 'http://localhost:9000'}).text)"

# Set environment variables for system-wide proxy usage
export http_proxy=http://localhost:9000
export https_proxy=http://localhost:9000
curl https://example.com
```

### Output

The proxy returns the fully rendered HTML as plain text. Redirect to a file to save it:
```bash
curl --proxy "http://localhost:9000" https://example.com > rendered.html
```

### Path-based usage (legacy)
```bash
# The old format still works for backwards compatibility
curl http://localhost:9000/https://example.com
```

## How it works

1. Receives an HTTP request with a target URL
2. Launches a Chromium browser context via Playwright
3. Navigates to the target URL and waits for page load
4. Waits 3 seconds for any additional dynamic content to render
5. Extracts the full HTML content
6. Returns the rendered HTML to the client
7. Closes the browser context to free resources

## Limitations

- **Performance**: Each request creates a new browser context, so the proxy is slower than traditional proxies
- **Resource usage**: Requires significant memory and CPU for browser automation
- **Anti-bot detection**: Some sites (like Instagram) actively block automated access despite realistic user-agents
- **JavaScript complexity**: May not handle extremely complex or obfuscated JavaScript perfectly
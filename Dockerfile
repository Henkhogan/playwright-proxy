# --- Stage 1: Build the Rust binary ---
FROM rust:1.96-slim-bookworm AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY . .
RUN cargo build --release

# --- Stage 2: Minimal Playwright Runtime ---
# Using the official Playwright image completely avoids missing library/font errors
FROM mcr.microsoft.com/playwright:v1.61.0-noble

WORKDIR /app

# Copy the compiled native binary from the builder stage
COPY --from=builder /app/target/release/playwright-proxy /app/playwright-proxy

EXPOSE 3128

# Start the API directly (Playwright driver starts implicitly inside the Rust process)
CMD ["/app/playwright-proxy"]

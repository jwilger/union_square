# Build stage
FROM rust:1.83-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Build dependencies - this is the caching Docker layer!
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy source code
COPY . .

# Build application
RUN touch src/main.rs && \
    cargo build --release --bin union_square

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1001 -s /bin/bash union_square

# Copy binary from builder
COPY --from=builder /app/target/release/union_square /usr/local/bin/union_square

# Set ownership
RUN chown -R union_square:union_square /usr/local/bin/union_square && \
    chmod +x /usr/local/bin/union_square

# Switch to non-root user
USER union_square

# Expose default port (adjust as needed)
EXPOSE 8080

# Set entrypoint
ENTRYPOINT ["/usr/local/bin/union_square"]

# Multi-stage Docker build for netwatch

# Build stage
FROM rust:1.75-slim as builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy all necessary files for the build
COPY . .

# Build for release
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -s /bin/false netwatch

# Copy the binary from builder stage
COPY --from=builder /app/target/release/netwatch /usr/local/bin/netwatch

# Set ownership and permissions
RUN chown root:root /usr/local/bin/netwatch && \
    chmod 755 /usr/local/bin/netwatch

# Switch to non-root user
USER netwatch

# Set default command
ENTRYPOINT ["netwatch"]
CMD ["--help"]

# Metadata
LABEL maintainer="netwatch contributors"
LABEL description="A modern network traffic monitor for Unix systems"
LABEL version="0.1.9"
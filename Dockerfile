# Build stage
FROM rust:alpine AS builder
WORKDIR /myapp

# Install system dependencies for building
RUN apk add --no-cache \
    musl-dev \
    openssl-dev

RUN rustup default nightly
RUN rustup target add x86_64-unknown-linux-musl
RUN rustup install nightly

# Create app directory
WORKDIR /usr/src/app

# Copy manifest files
COPY Cargo.toml Cargo.lock* ./

# Create a dummy main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies (this step will be cached unless Cargo.toml changes)
RUN cargo build --release && rm src/main.rs

# Copy source code
COPY src ./src

# Build the actual application
RUN cargo build --locked --release

# Runtime stage
FROM alpine:latest

# Install runtime dependencies
RUN apk add --no-cache \
    ca-certificates

# Create a non-root user
RUN adduser -D -s /bin/false -u 1000 colabri

# Set working directory in the container
WORKDIR /app

# Copy the binary from builder stage
COPY --from=builder /usr/src/app/target/release/colabri-doc /app/colabri-doc

# Change ownership of files to the colabri user
RUN chown -R colabri:colabri /app

# Switch to the non-root user
USER colabri

# Expose port 3000
EXPOSE 3000

# Execute the binary
CMD ["/app/colabri-doc"]

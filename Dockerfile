# Build stage
FROM rust:1-bookworm AS builder

# Install nightly toolchain
RUN rustup toolchain install nightly && rustup default nightly

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
RUN rm -f target/release/deps/colabri_doc* && cargo build --locked --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user
RUN useradd -m -u 1000 colabri

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

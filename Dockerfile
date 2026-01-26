# Build stage
FROM rust:1.82-bookworm as builder

WORKDIR /app

# Copy manifests
COPY backend/Cargo.toml backend/Cargo.lock ./

# Create a dummy main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Verify Rust version and show cargo details
RUN rustc --version && cargo --version

# Build dependencies (cached layer)
RUN cargo build --release
RUN rm src/main.rs

# Copy source code
COPY backend/src ./src
COPY backend/words.json ./words.json
COPY backend/schema.sql ./schema.sql

# Build the actual application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install required libraries
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder /app/target/release/chain_verse /app/chain_verse
COPY --from=builder /app/words.json /app/words.json
COPY --from=builder /app/schema.sql /app/schema.sql

# Create directory for database
RUN mkdir -p /app/data

# Set environment variables
ENV DATABASE_URL=sqlite:///app/data/chain_verse.db
ENV RUST_LOG=info

# Expose port (Railway will set PORT env var)
EXPOSE 3000

# Run the application in full mode (collector + API)
CMD ["./chain_verse", "full"]

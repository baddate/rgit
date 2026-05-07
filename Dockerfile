FROM rust:1.88-bookworm AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    clang \
    libclang-dev \
    libsnappy-dev \
    && rm -rf /var/lib/apt/lists/*

# Cache dependency compilation
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src && echo "fn main() {}" > src/main.rs
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release 2>/dev/null || true

# Build real binary, then copy out of cache mount into Docker layer
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release && \
    cp target/release/rgit /app/rgit

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    git \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/rgit /app/bin/rgit

ENTRYPOINT ["/app/bin/rgit"]

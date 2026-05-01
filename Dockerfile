FROM rust:1.88-bookworm AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    cmake \
    clang \
    libclang-dev \
    libsnappy-dev \
    git \
    && rm -rf /var/lib/apt/lists/*

# Cache dependencies via dummy build
COPY Cargo.toml Cargo.lock ./
COPY tree-sitter-grammar-repository/Cargo.toml ./tree-sitter-grammar-repository/
COPY tree-sitter-grammar-repository/build.rs ./tree-sitter-grammar-repository/
RUN mkdir -p src && echo "fn main() {}" > src/main.rs
RUN mkdir -p tree-sitter-grammar-repository/src && echo "" > tree-sitter-grammar-repository/src/lib.rs
RUN cargo build --release 2>/dev/null || true

# Build actual binary
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim

WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    git \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/rgit /app/bin/rgit

ENTRYPOINT ["/app/bin/rgit"]

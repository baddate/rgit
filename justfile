# rgit development commands
# Usage: just <recipe>

# set env
export LIBCLANG_PATH := "/opt/homebrew/opt/llvm/lib"
export DYLD_FALLBACK_LIBRARY_PATH := "/opt/homebrew/opt/llvm/lib"

# Default: list available recipes
default:
    @just --list

# ── Build ─────────────────────────────────────────────────────────────────────

# Build in debug mode
build:
    cargo build

# Build in release mode
build-release:
    cargo build --release

# Build with zlib-ng feature (faster zlib)
build-zlib-ng:
    cargo build --release --features zlib-ng

# ── Run ───────────────────────────────────────────────────────────────────────

# Run the dev server (requires SCAN_PATH and DB_PATH)
# Example: just run /path/to/git/repos
run scan_path="." db_path="/tmp/rgit-dev.db" bind="[::1]:8000":
    cargo run -- {{ bind }} {{ scan_path }} -d {{ db_path }}

# Run with a custom refresh interval (e.g. 30s, 5m)
run-watch scan_path="." db_path="/tmp/rgit-dev.db" interval="30s":
    cargo run -- [::1]:8000 {{ scan_path }} -d {{ db_path }} --refresh-interval {{ interval }}

# ── Test & Check ──────────────────────────────────────────────────────────────

# Run tests
test:
    cargo test

# Run clippy lints
lint:
    cargo clippy -- -D warnings

# Check formatting
fmt-check:
    cargo fmt --check

# Apply formatting
fmt:
    cargo fmt

# Run all checks (fmt + lint + test)
check: fmt-check lint test

# ── Docker ────────────────────────────────────────────────────────────────────

# Build the Docker image
docker-build tag="rgit:dev":
    docker build -t {{ tag }} .

# Run via docker-compose
docker-up:
    docker compose up

# Run in background
docker-up-detached:
    docker compose up -d

# Stop docker-compose services
docker-down:
    docker compose down

# Tail docker-compose logs
docker-logs:
    docker compose logs -f

# ── Assets ────────────────────────────────────────────────────────────────────

# Compile SCSS to CSS (via cargo build, since rsass runs in build.rs)
css:
    cargo build -q 2>&1 | grep -v "^$" || true
    @echo "CSS compiled via build.rs (check target/*/build/rgit-*/out/statics/css/style.css)"

# ── Release ───────────────────────────────────────────────────────────────────

# Generate changelog since last tag using git-cliff
changelog:
    git cliff --output CHANGELOG.md

# Show what would go into the next changelog entry
changelog-preview:
    git cliff --unreleased

# ── Housekeeping ──────────────────────────────────────────────────────────────

# Remove build artifacts
clean:
    cargo clean

# Remove the dev database
clean-db db_path="/tmp/rgit-dev.db":
    rm -rf {{ db_path }}
    @echo "Removed {{ db_path }}"

# Check for known-vulnerable dependencies
audit:
    cargo audit

# Update dependencies
update:
    cargo update

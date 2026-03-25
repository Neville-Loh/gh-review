set dotenv-load

default:
    @just --list

# ── Development ──────────────────────────────────────────────

# Run the app with arguments (e.g. just run octocat/hello-world 42)
run *ARGS:
    cargo run -- {{ARGS}}

# Build debug binary
build:
    cargo build

# Build optimized release binary
build-release:
    cargo build --release

# Run all tests
test:
    cargo test

# Run clippy lints
lint:
    cargo clippy -- -D warnings

# Format code
fmt:
    cargo fmt

# Check formatting without modifying
fmt-check:
    cargo fmt -- --check

# Run all checks (format, lint, test)
check: fmt-check lint test

# ── Utilities ────────────────────────────────────────────────

# Install locally from source
install:
    cargo install --path .

# Clean build artifacts
clean:
    cargo clean

# Open the repo in the browser
open:
    open "https://github.com/Neville-Loh/gh-review"
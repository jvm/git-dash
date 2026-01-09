# List available recipes
default:
    @just --list

# Run the TUI application
run *ARGS:
    cargo run -- {{ARGS}}

# Run with debug logging
debug *ARGS:
    cargo run -- --debug {{ARGS}}

# Build in release mode
build:
    cargo build --release

# Run all tests
test:
    cargo test

# Run specific test
test-one TEST:
    cargo test {{TEST}}

# Format code
fmt:
    cargo fmt

# Check formatting without modifying
fmt-check:
    cargo fmt --check

# Run clippy linter
lint:
    cargo clippy --all-targets --all-features

# Run all quality checks (pre-commit)
check: fmt lint test build
    @echo "✓ All checks passed!"

# Clean build artifacts
clean:
    cargo clean
    rm -f git-dash-debug.log

# Install the binary locally
install:
    cargo install --path .

# Update dependencies
update:
    cargo update

# Show dependency tree
deps:
    cargo tree

# Check for dependency issues (requires cargo-deny)
deny:
    cargo deny check

# Run benchmarks (when added)
bench:
    cargo bench

# Generate and open documentation
doc:
    cargo doc --open

# Watch for changes and run tests
watch-test:
    cargo watch -x test

# Watch for changes and run the app
watch-run:
    cargo watch -x run

# Build release binaries for Homebrew (requires both targets installed)
build-release VERSION="0.1.0":
    ./scripts/build-release.sh {{VERSION}}

# Create a new git tag for release
tag VERSION:
    git tag -a v{{VERSION}} -m "Release v{{VERSION}}"
    git push origin v{{VERSION}}

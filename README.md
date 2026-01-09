# git-dash

A fast TUI dashboard for discovering and managing multiple Git repositories from a single interface.

## Features

- 🔍 **Automatic Discovery**: Recursively scans directories for Git repositories
- 📊 **Rich Status View**: 7-column table showing repository state at a glance
- ⚡ **Parallel Operations**: Fast status fetching using concurrent threads
- 🎯 **Safe Actions**: Remote validation and confirmation prompts for push/pull
- 📈 **Progress Indicators**: Real-time progress bar during repository scanning
- 🐛 **Debug Logging**: Optional detailed logging for troubleshooting
- ✅ **Well Tested**: 17 unit and integration tests

## Installation

### Homebrew (macOS)

```sh
# Add the tap
brew tap jvm/tap

# Install (or in one command)
brew install jvm/tap/git-dash
```

### From Source

```sh
cargo install --path .
```

### From crates.io (when published)

```sh
cargo install git-dash
```

### Build and Run

Using [Just](https://github.com/casey/just) (recommended):
```sh
# Install just
brew install just  # macOS
# or
cargo install just

# Common commands
just run           # Run from current directory
just debug         # Run with debug logging
just test          # Run all tests
just check         # Run all quality checks
just build         # Build release binary
just install       # Install locally
```

Using Cargo directly:
```sh
# Run from current directory
cargo run

# Run with debug logging
cargo run -- --debug

# Scan a specific directory
cargo run -- ~/repos

# Release build for better performance
cargo build --release
./target/release/git-dash
```

## Usage

```
git-dash [OPTIONS] [PATH]

ARGS:
    path    Optional directory to scan (defaults to current directory)

OPTIONS:
    -d, --debug    Enable debug logging to git-dash-debug.log
    -h, --help     Print help information
```

## Display Columns

The main view shows a table with the following columns:

1. **Repository**: Name of the repository
2. **Branch**: Current branch (or "DETACHED" for detached HEAD)
3. **Dirty**: Working tree status (clean/dirty with color coding)
4. **Ahead/Behind**: Commits ahead/behind upstream (+2/-1 format)
5. **Changes**: Summary of changes by type (M:2 A:1 D:1 format)
6. **Remote**: Simplified remote URL (e.g., github.com/user/repo)
7. **Last Fetch**: Time since last fetch (5m, 2h, 3d format)

## Keybindings

### Navigation
- `j` / `k` or `↓` / `↑`: Move selection up/down
- `PageDown` / `PageUp`: Jump 10 repositories at a time

### Actions
- `p`: Pull selected repository (prompts for confirmation)
- `u`: Push selected repository (prompts for confirmation)
- `r`: Refresh status for all repositories

### Confirmation Prompts
- `y`: Confirm action
- `n` or `Esc`: Cancel action

### Exit
- `q`: Quit application
- `Ctrl+C`: Force quit

## Features in Detail

### Repository Discovery

- Recursively scans the specified directory for `.git` folders or files
- Handles both regular repositories and worktrees/submodules (gitdir files)
- Stops at nested repositories (doesn't traverse into subdirectories of found repos)
- Shows animated progress bar during scanning

### Status Information

- Parses Git's porcelain v2 format for accurate status information
- Displays change types: Modified (M), Added (A), Deleted (D), Untracked (?)
- Simplifies remote URLs for better readability
- Shows human-readable last fetch timestamps
- Inline error messages when Git operations fail

### Safety Features

- Validates remote configuration before attempting push/pull
- Requires explicit confirmation (y/n) for all network operations
- Fast-forward only pulls to prevent accidental merge commits
- Timeouts for long-running Git operations (30s for push/pull, 5s for status)

### Performance

- Parallel status fetching: All repositories checked concurrently
- Two-phase scanning: 40% for discovery, 60% for parallel status
- Non-blocking UI: All Git operations run in background worker thread
- Optimized porcelain parsing for minimal overhead

## Debug Logging

Enable debug logging to troubleshoot issues or understand performance:

```sh
cargo run -- --debug ~/repos
```

Debug logs include:
- Timestamp with millisecond precision
- Repository scan progress
- Git command execution times
- Success/failure status for all operations
- Performance metrics for status fetching

Logs are written to `git-dash-debug.log` in the current directory.

## Testing

Run the test suite:

```sh
cargo test
```

The project includes:
- 14 unit tests in `src/status.rs` (parsing, formatting, URL simplification)
- 3 integration tests in `tests/repo_discovery.rs` (discovery, nested repos, gitdir files)

## Development

### Requirements

- Rust 1.92.0 or later (specified in `rust-toolchain.toml`)
- [Just](https://github.com/casey/just) command runner (optional but recommended)

### Quality Checks

Before committing, run:
```sh
just check  # Format, lint, test, and build
```

Or individually:
```sh
cargo fmt              # Format code
cargo clippy --all-targets --all-features  # Lint
cargo test             # Run tests
cargo build --release  # Build
```

### Optional Tools

**cargo-deny** - Check for security vulnerabilities and license issues:
```sh
cargo install cargo-deny
cargo deny check  # or: just deny
```

**cargo-watch** - Auto-rebuild on file changes:
```sh
cargo install cargo-watch
cargo watch -x test  # or: just watch-test
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development guidelines.

## License

Licensed under the [MIT License](LICENSE).

Copyright (c) 2025 Jose Mocito

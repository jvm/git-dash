# Repository Guidelines

## Project Structure & Module Organization

This repository now has a Rust crate scaffold. Keep the standard layout to stay predictable:

- `src/` for application code (e.g., `src/main.rs`, `src/app/`, `src/ui/`).
- `tests/` for integration tests.
- `assets/` for static resources (e.g., icons, fixtures).

Keep modules focused: Git discovery, status parsing, UI rendering, and background workers should live in separate modules.

## Build, Test, and Development Commands

### Using Just (Recommended)

This project uses [Just](https://github.com/casey/just) as a command runner. Install with:
```sh
# macOS
brew install just

# Or via cargo
cargo install just
```

Common commands:
```sh
just              # List all available recipes
just run          # Run the TUI application
just debug        # Run with debug logging
just test         # Run all tests
just check        # Run all quality checks (format, lint, test, build)
just build        # Build release binary
just install      # Install locally
just clean        # Clean build artifacts
```

See `justfile` for all available commands.

### Using Cargo Directly

Alternatively, use conventional Cargo commands:

- `cargo run` to start the TUI locally
- `cargo build` for a release or debug build
- `cargo test` to run unit and integration tests

### Rust Toolchain

This project uses Rust 1.92.0 (specified in `rust-toolchain.toml`). The toolchain will be automatically installed when you run cargo commands.

Required components:
- `rustfmt` - Code formatting
- `clippy` - Linting

## Coding Style & Naming Conventions

- Use default `rustfmt` formatting (4-space indentation with tabs handled by rustfmt).
- Prefer `snake_case` for functions and modules, `UpperCamelCase` for types, and `SCREAMING_SNAKE_CASE` for constants.
- Keep async or threaded operations isolated in worker modules; avoid shared mutable state across threads, per the spec.

## Code Quality Standards

All code must meet these quality standards before commit:

**When any code changes are made**, always run the full quality gate sequence before handing off:
```sh
cargo fmt
cargo clippy --all-targets --all-features
cargo test
cargo build --release
```

### 1. Formatting
```sh
cargo fmt
```
All code must be formatted with rustfmt. Run `cargo fmt` before committing. Verify with `cargo fmt --check`.

### 2. Linting
```sh
cargo clippy --all-targets --all-features
```
Code must pass clippy with **zero warnings**. Fix all clippy suggestions before committing. Common fixes:
- Use `split_once()` instead of `splitn(2, ...)`
- Use `is_multiple_of()` instead of `% n == 0`
- Use `for` loops instead of `while let` on iterators
- Prefer idiomatic Rust patterns over manual implementations

### 3. Compilation
```sh
cargo build --release
```
Code must compile with **zero warnings** in both debug and release modes. Use `#[allow(...)]` sparingly and only when justified.

### 4. Testing
```sh
cargo test
```
All tests must pass. The current test suite includes:
- 14 unit tests in `src/status.rs` (parsing, formatting, URL simplification)
- 3 integration tests in `tests/repo_discovery.rs` (discovery, nested repos, gitdir files)

When adding new functionality:
- Add unit tests for pure functions (parsing, formatting, transformations)
- Add integration tests for complex behaviors (repository discovery, Git operations)
- Aim for comprehensive coverage of edge cases (paths with spaces, detached HEAD, etc.)

### 5. Dependency Security & Licensing (Optional)

Install [cargo-deny](https://github.com/EmbarkStudios/cargo-deny):
```sh
cargo install cargo-deny
```

Check dependencies for security vulnerabilities, license issues, and bans:
```sh
cargo deny check
# or
just deny
```

The `deny.toml` configuration enforces:
- No security vulnerabilities (deny)
- Acceptable licenses (MIT, Apache-2.0, BSD variants)
- Trusted sources (crates.io only)
- Warnings for unmaintained or yanked crates

## Testing Guidelines

The project uses Rust's built-in test framework:

- **Unit tests**: Located in `#[cfg(test)]` modules within source files (e.g., `src/status.rs`)
- **Integration tests**: Located in `tests/` directory (e.g., `tests/repo_discovery.rs`)

### Test Organization
- Unit tests cover pure functions: parsing, formatting, transformations
- Integration tests cover complex behaviors: repository discovery, Git worktrees, nested repos
- Tests validate edge cases: paths with spaces, detached HEAD, missing remotes

### Running Tests
```sh
# Run all tests
cargo test

# Run specific test
cargo test test_parse_status_line

# Run with output
cargo test -- --nocapture
```

### Writing New Tests
- Name tests descriptively: `test_<function>_<scenario>` (e.g., `test_parse_status_line_with_spaces_in_path`)
- Test both success and error cases
- Use temporary directories for integration tests (clean up in test)
- Keep tests focused on a single behavior

## Pre-Commit Checklist

Before committing, ensure all checks pass:

```sh
# 1. Format code
cargo fmt

# 2. Run linter (must have zero warnings)
cargo clippy --all-targets --all-features

# 3. Run tests (must all pass)
cargo test

# 4. Verify release build (must have zero warnings)
cargo build --release
```

**All four steps must succeed with zero warnings/errors before committing.**

## Commit & Pull Request Guidelines

Use short, imperative commit summaries (e.g., "Add repo scanner", "Fix status parsing for paths with spaces").

Commit messages should:
- Start with a verb in imperative mood (Add, Fix, Update, Remove, Refactor)
- Be concise (50 chars or less for first line)
- Explain "what" and "why", not "how"

Pull requests should include:
- A concise description of behavior changes
- Linked issues (if any)
- Screenshots or terminal captures for UI changes
- Confirmation that pre-commit checklist passed

## Release Process

Use `RELEASE.md` as the source of truth. The high-level flow:

1. Bump `Cargo.toml` version and update `Cargo.lock`.
2. Run `just check`.
3. Build release artifacts (`just build-release VERSION=x.y.z`).
4. Create and push tag (`vX.Y.Z`).
5. Create a GitHub release and upload the tarballs.
6. Trigger the Homebrew tap workflow and merge the PR.
7. Publish to crates.io.
8. Remove local release artifacts.

### Release Prerequisites

- Authenticated GitHub CLI (`gh auth status`).
- `cargo login` with a verified crates.io email.
- Rust targets installed for macOS releases:
  - `rustup target add x86_64-apple-darwin`
  - `rustup target add aarch64-apple-darwin`

### Toolchain Note

If Homebrew `cargo`/`rustc` are ahead in `PATH`, prefer rustup for release builds:
```sh
rustup run 1.92.0 cargo build --release --target x86_64-apple-darwin
```

## Security & Configuration Notes

This tool is local-first. Do not introduce telemetry or network access beyond invoking Git. Configuration is currently out of scope; if you add it later, document defaults and safe handling of paths and credentials.

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   bd sync
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds

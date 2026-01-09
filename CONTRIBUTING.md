# Contributing

Thanks for contributing to git-dash. Keep changes small, focused, and aligned to the spec.

## Project Structure

- `src/` for Rust application code.
- `tests/` for integration tests.
- `assets/` for static resources (icons, fixtures).

Suggested module splits: discovery, status parsing, UI rendering, and background workers.

## Development Commands

- `just run` or `cargo run` to start the TUI locally.
- `just build` or `cargo build --release` for a release build.
- `just test` or `cargo test` to run tests.

## Style & Conventions

- Use `rustfmt` defaults.
- `snake_case` for functions/modules, `UpperCamelCase` for types, `SCREAMING_SNAKE_CASE` for constants.
- Avoid shared mutable state across threads; use message passing for background Git operations.

## Testing

- Use Rust’s built-in test framework (`#[test]`).
- Name integration tests by behavior, e.g., `tests/repo_discovery.rs`.

## Commits & PRs

- Use short, imperative commit summaries (e.g., “Add repo scanner”).
- PRs should include a concise description, linked issues when relevant, and screenshots or terminal captures for UI changes.

## Security & Configuration

The tool is local-first. Do not add telemetry or network access beyond invoking Git. If configuration is introduced later, document safe defaults and avoid handling credentials directly.

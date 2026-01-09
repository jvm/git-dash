# Release Checklist for git-dash

Follow these steps to create a new release.

## Prerequisites

- `gh auth status` is logged in.
- `cargo login` is done and your crates.io email is verified.
- macOS targets installed:
  ```sh
  rustup target add x86_64-apple-darwin
  rustup target add aarch64-apple-darwin
  ```

## Pre-Release Checklist

- [ ] Version updated in `Cargo.toml`
- [ ] `Cargo.lock` updated
- [ ] All tests passing (`just check`)
- [ ] Documentation updated (README/RELEASE as needed)
- [ ] CHANGELOG updated (if exists)

## Release Steps (Canonical)

1. **Run quality checks**
   ```sh
   just check
   ```

2. **Build release artifacts**
   ```sh
   just build-release VERSION=0.1.1
   ```

3. **Create and push tag**
   ```sh
   git tag -a v0.1.1 -m "Release v0.1.1"
   git push origin v0.1.1
   ```

4. **Create GitHub release and upload tarballs**
   ```sh
   gh release create v0.1.1 \
     git-dash-v0.1.1-aarch64-apple-darwin.tar.gz \
     git-dash-v0.1.1-x86_64-apple-darwin.tar.gz \
     -t "v0.1.1" \
     -n "Release v0.1.1."
   ```

5. **Update Homebrew tap**
   ```sh
   gh workflow run update-homebrew-tap.yml -f version=0.1.1
   ```
   Merge the resulting PR in `jvm/homebrew-tap`.

6. **Publish to crates.io**
   ```sh
   cargo publish --dry-run
   cargo publish
   ```

7. **Cleanup local artifacts**
   ```sh
   rm -f git-dash-v0.1.1-*-apple-darwin.tar.gz
   ```

## Building Release Binaries (Details)

The canonical path is `just build-release`, which runs `scripts/build-release.sh`.

Manual fallback:
```sh
cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin
```

## Testing the Release

```sh
brew uninstall git-dash  # if already installed
brew update
brew install jvm/tap/git-dash
git-dash --help
git-dash ~/repos
```

## Post-Release

- [ ] Announce on relevant channels
- [ ] Update documentation if needed
- [ ] Close related issues
- [ ] Update roadmap

## Troubleshooting

**Release assets 404 in the tap workflow:**
- Ensure the GitHub release exists before triggering the workflow.

**Homebrew PR already exists:**
- The workflow will force-update the branch; just review and merge.

**Cross-compile failures with Homebrew Rust:**
- Prefer rustup: `rustup run 1.92.0 cargo build --release --target x86_64-apple-darwin`

## Example Release Notes Template

```markdown
## What's New

- Feature 1
- Feature 2
- Bug fix 1

## Installation

### Homebrew (macOS)
```sh
brew tap jvm/tap
brew install git-dash
```

### Cargo
```sh
cargo install git-dash
```

## Full Changelog

See [CHANGELOG.md](CHANGELOG.md) for details.
```

## Troubleshooting

**Formula won't install:**
- Verify checksums match actual file checksums
- Ensure release artifacts are publicly accessible
- Check URLs are correct in formula

**Cross-compilation issues:**
- May need Xcode Command Line Tools
- May need to install target-specific linkers

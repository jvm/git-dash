# Release Checklist for git-dash

Follow these steps to create a new release.

## Pre-Release Checklist

- [ ] All tests passing (`just check`)
- [ ] Version updated in `Cargo.toml`
- [ ] Version updated in `homebrew/git-dash.rb`
- [ ] CHANGELOG updated (if exists)
- [ ] Documentation updated

## Building Release Binaries

### Option 1: Just (Recommended)

Build both macOS targets, create tarballs, and print SHA256 checksums:
```sh
just build-release VERSION=0.1.0
```

This runs `scripts/build-release.sh` under the hood.

### Option 2: Manual Build (if you have both architectures)

**On Intel Mac:**
```sh
cargo build --release --target x86_64-apple-darwin
cd target/x86_64-apple-darwin/release
tar -czf git-dash-v0.1.0-x86_64-apple-darwin.tar.gz git-dash
shasum -a 256 git-dash-v0.1.0-x86_64-apple-darwin.tar.gz
```

**On Apple Silicon Mac:**
```sh
cargo build --release --target aarch64-apple-darwin
cd target/aarch64-apple-darwin/release
tar -czf git-dash-v0.1.0-aarch64-apple-darwin.tar.gz git-dash
shasum -a 256 git-dash-v0.1.0-aarch64-apple-darwin.tar.gz
```

### Option 3: Cross-Compilation (Advanced)

Install cross-compilation tools:
```sh
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin
```

Then build both:
```sh
cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin
```

### Option 4: GitHub Actions (Recommended - TODO)

Create `.github/workflows/release.yml` to automate builds on tag push.

## Creating the GitHub Release

1. **Create and push a tag:**
   ```sh
   git tag -a v0.1.0 -m "Release v0.1.0"
   git push origin v0.1.0
   ```

2. **Create release on GitHub:**
   - Go to https://github.com/jvm/git-dash/releases/new
   - Select tag: `v0.1.0`
   - Release title: `v0.1.0`
   - Description: Release notes
   - Upload both `.tar.gz` files

3. **Note the SHA256 checksums** from the build output

## Updating Homebrew Tap

The Homebrew formula lives in a separate repository: https://github.com/jvm/homebrew-tap

1. **Clone or update the tap repository:**
   ```sh
   git clone https://github.com/jvm/homebrew-tap.git
   cd homebrew-tap
   ```

2. **Update the formula** (`Formula/git-dash.rb`):
   - Update `version` line
   - Update URLs with new version number
   - Replace SHA256 checksums with actual values from the build output

3. **Commit and push:**
   ```sh
   git add Formula/git-dash.rb
   git commit -m "Update git-dash to v0.1.0"
   git push
   ```

See the tap repository for formula maintenance and documentation.

## Testing the Release

```sh
# Test Homebrew installation
brew uninstall git-dash  # if already installed
brew update
brew install jvm/tap/git-dash

# Verify version
git-dash --help

# Test basic functionality
git-dash ~/repos
```

## Publishing to crates.io (Optional)

```sh
# Dry run first
cargo publish --dry-run

# Publish
cargo publish
```

## Post-Release

- [ ] Announce on relevant channels
- [ ] Update documentation if needed
- [ ] Close related issues
- [ ] Update roadmap

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

# YaoXiang Release Guide

This document outlines the release process for the YaoXiang programming language.

## Release Checklist

### 1. Pre-Release Preparation

- [ ] Ensure all tests pass: `cargo test --all`
- [ ] Run benchmarks: `cargo bench`
- [ ] Check code formatting: `cargo fmt --check`
- [ ] Run clippy: `cargo clippy --all-targets --all-features`
- [ ] Update version in [`Cargo.toml`](Cargo.toml)
- [ ] Update CHANGELOG.md with all changes since last release

### 2. Version Management

YaoXiang follows [Semantic Versioning](https://semver.org/):
- **MAJOR**: Breaking changes
- **MINOR**: New features (backward compatible)
- **PATCH**: Bug fixes

Update version in Cargo.toml:
```toml
[package]
version = "X.Y.Z"
```

### 3. Changelog Generation

Update `CHANGELOG.md` with the following sections:

```markdown
# Changelog

## [X.Y.Z] - YYYY-MM-DD

### Added
- New features added

### Changed
- Changes to existing features

### Deprecated
- Features marked for removal

### Removed
- Features that have been removed

### Fixed
- Bug fixes

### Security
- Security fixes
```

### 4. Release Types

| Type | Branch | Description |
|------|--------|-------------|
| Release | `main` | Stable release |
| Beta | `beta` | Pre-release testing |
| Nightly | `nightly` | Latest development build |

### 5. Building Release Artifacts

```bash
# Build release binary
cargo build --release

# Build for all targets (cross-compilation)
cargo build --release --target x86_64-unknown-linux-gnu
cargo build --release --target x86_64-pc-windows-gnu
cargo build --release --target x86_64-apple-darwin
```

### 6. Creating a Release on GitHub

1. Go to [GitHub Releases](https://github.com/yaoxiang-lang/yaoxiang/releases)
2. Click "Draft a new release"
3. Select the tag version (e.g., `vX.Y.Z`)
4. Target branch: `main`
5. Fill in release notes from CHANGELOG
6. Upload artifacts:
   - `yaoxiang-X.Y.Z-x86_64-unknown-linux-gnu.tar.gz`
   - `yaoxiang-X.Y.Z-x86_64-pc-windows-gnu.zip`
   - `yaoxiang-X.Y.Z-x86_64-apple-darwin.tar.gz`

### 7. Publishing to Crates.io

```bash
# Login to crates.io
cargo login

# Publish
cargo publish
```

### 8. Post-Release Tasks

- [ ] Merge release branch back to main (if applicable)
- [ ] Update documentation website
- [ ] Announce release on social media
- [ ] Update README.md version badge

## Release Templates

### Version Bump Script

```bash
#!/bin/bash
# bump_version.sh - Bump version and create git tag

VERSION_TYPE=$1  # major, minor, or patch

if [ -z "$VERSION_TYPE" ]; then
    echo "Usage: ./bump_version.sh <major|minor|patch>"
    exit 1
fi

# Get current version
CURRENT_VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')

# Bump version using semver
NEW_VERSION=$(cargo semver $VERSION_TYPE $CURRENT_VERSION)

# Update Cargo.toml
sed -i "s/version = \"$CURRENT_VERSION\"/version = \"$NEW_VERSION\"/" Cargo.toml

# Create git tag
git add Cargo.toml
git commit -m "chore: bump version to $NEW_VERSION"
git tag -a "v$NEW_VERSION" -m "Release v$NEW_VERSION"

echo "Version bumped to $NEW_VERSION and tag created"
```

### Release Note Template

```markdown
## What's Changed

### Breaking Changes
- List breaking changes here

### New Features
- List new features here

### Bug Fixes
- List bug fixes here

### Performance Improvements
- List performance improvements here

### Documentation
- List documentation changes here

### Other Changes
- List other changes here

**Full Changelog**: https://github.com/yaoxiang-lang/yaoxiang/compare/vPREV_VERSION...vNEW_VERSION
```

## Troubleshooting

### Common Issues

1. **Tests failing**: Run `cargo test --release` to catch release-specific bugs
2. **Build errors**: Ensure you have the latest Rust stable: `rustup update stable`
3. **Clippy warnings**: Address all clippy warnings before release

### Getting Help

- Check existing [issues](https://github.com/yaoxiang-lang/yaoxiang/issues)
- Open a new issue if problems persist

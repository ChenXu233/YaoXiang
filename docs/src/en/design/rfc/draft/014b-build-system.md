---
title: "RFC-014b: Build System and Binary Distribution"
status: "Draft"
author: "Chenxu"
created: "2026-06-11"
updated: "2026-06-11"
group: "rfc-014"
---

# RFC-014b: Build System and Binary Distribution

> This RFC is a sub-RFC of [RFC-014: Package Management System Design](../accepted/014-package-manager.md).

## Summary

Define the build mechanism for the YaoXiang package management system: declarative build configuration, build strategies (cargo/cmake/custom/none), precompiled binary distribution, and system dependency checks.

## Motivation

Some packages are pure `.yx` code and require no build step. Others need to compile FFI bindings (calling Cargo, CMake, etc.). A unified mechanism is needed so that package authors can declare build requirements and have the package manager handle them automatically.

### Current Problems

- No declarative build configuration (no `[build]` section in `yaoxiang.toml`)
- No precompiled binary distribution mechanism
- FFI package builds rely entirely on manual user operations
- No system dependency checks

## Proposal

### Core Design: Declarative Build + Precompiled-First

Package authors declare build requirements in `yaoxiang.toml`, and the package manager makes decisions automatically based on the declarations.

### Build Strategies

```rust
enum BuildStrategy {
    None,          // Pure .yx package, no build needed
    Cargo,         // Invoke cargo build
    Cmake,         // Invoke cmake
    Custom,        // Execute build.yx script
    Precompiled,   // Use precompiled artifacts directly
}
```

### Build Declaration in yaoxiang.toml

```toml
[package]
name = "native-foo"
version = "1.0.0"

[build]
strategy = "cargo"              # Build strategy
script = "build.yx"             # Only used when strategy = "custom"

[build.cargo]
features = ["ffi"]              # cargo build --features ffi
target = "release"              # cargo build --release

[build.requirements]
cargo = ">= 1.70"               # Tools needed at build time
cmake = ">= 3.20"

[build.platforms]               # Platform-specific overrides
"linux-x86_64" = { cargo-features = ["linux-ffi"] }
"windows-x86_64" = { cargo-features = ["win-ffi"] }
"aarch64-apple-darwin" = { cargo-features = ["mac-ffi"] }
```

### Install Decision Tree

```
yaoxiang install foo
    │
    ├─ 1. Check Registry/GitHub Release for precompiled artifact for current platform?
    │     → Yes: Download, verify SHA-256, install directly (skip build)
    │     → No:  Continue
    │
    ├─ 2. Download source package
    │
    ├─ 3. Read [build] section of yaoxiang.toml
    │     → strategy = "none": Install directly
    │     → Other: Check requirements, execute build
    │
    └─ 4. Install to vendor/
```

**Precompiled first, source as fallback.** Similar to Python's wheel vs sdist.

### Precompiled Binary Declaration

```toml
# yaoxiang.toml
[binaries]
"linux-x86_64" = { url = "releases/download/v1.0.0/foo-linux-x64.tar.gz", sha256 = "abc123" }
"windows-x86_64" = { url = "releases/download/v1.0.0/foo-win-x64.tar.gz", sha256 = "def456" }
"aarch64-apple-darwin" = { url = "releases/download/v1.0.0/foo-mac-arm.tar.gz", sha256 = "ghi789" }
```

**Conditions to skip the build:**
1. The `[binaries]` section has an entry for the current platform
2. SHA-256 verification passes
3. Download succeeds

All three conditions met → skip the build. Otherwise → fall back to source build.

### build.yx Build Script

Executed when `strategy = "custom"`:

```yx
# build.yx — Package build script
use std.os
use std.io

fn main() {
    let platform = os.platform()       # "linux", "windows", "macos"
    let arch = os.arch()               # "x86_64", "aarch64"

    if os.file_exists("Cargo.toml") {
        io.println("Building native extension via Cargo...")
        os.exec("cargo build --release")

        os.copy(
            "target/release/libfoo.so",
            "build/native/${platform}-${arch}/libfoo.so"
        )
    }

    io.println("Build complete!")
}
```

### System Dependency Check

All `[build.requirements]` are automatically checked before installation; an error is reported if any are not satisfied:

```
Error: Build requirement not satisfied
  cargo >= 1.70 required, but cargo is not installed
  Install: https://rustup.rs
```

### Integration with Cargo Workspace

If a package contains FFI code, a Cargo workspace can be defined alongside it:

```
my-package/
├── yaoxiang.toml          # YaoXiang package config
├── Cargo.toml             # Cargo workspace (FFI part)
├── src/
│   └── lib.yx             # YaoXiang code
└── native/
    ├── Cargo.toml          # Rust FFI code
    └── src/
        └── lib.rs
```

`yaoxiang build` automatically detects and invokes `cargo build` to compile the native part.

## Detailed Design

### Platform Identifiers

Use the `{os}-{arch}` format:

| Platform Identifier | OS | Arch |
|---------------------|----|------|
| `linux-x86_64` | Linux | x86_64 |
| `linux-aarch64` | Linux | ARM64 |
| `windows-x86_64` | Windows | x86_64 |
| `aarch64-apple-darwin` | macOS | ARM64 (Apple Silicon) |
| `x86_64-apple-darwin` | macOS | x86_64 |

### Build Artifact Directory Structure

```
build/
└── native/
    ├── linux-x86_64/
    │   └── libfoo.so
    ├── windows-x86_64/
    │   └── foo.dll
    └── aarch64-apple-darwin/
        └── libfoo.dylib
```

### Full Lifecycle of a Precompiled Package

```
Developer:
  1. Write .yx code + FFI bindings
  2. Declare [build] + [binaries] in yaoxiang.toml
  3. yaoxiang publish
     → Automatically build multi-platform binaries on CI
     → Upload source + precompiled artifacts

User:
  yaoxiang add native-foo
    → Precompiled artifact detected → download directly (seconds)
    → No precompiled artifact → download source + execute build (minutes)
```

## Trade-offs

### Advantages

- Declarative configuration; users don't need to understand build details
- Precompiled-first; installation is extremely fast
- Multi-platform support with automatic selection
- Seamless integration with the Cargo ecosystem

### Disadvantages

- Precompiled artifacts require CI support
- Multi-platform builds add release complexity
- build.yx scripts require a sandbox security mechanism

## Alternatives

| Alternative | Why Not Chosen |
|-------------|----------------|
| Pure source distribution | Users must install the build toolchain; high barrier to entry |
| Binary format similar to Python wheels | Too complex; not needed in the early YaoXiang ecosystem |
| No FFI build support | Would limit the language's extensibility |

## Implementation Strategy

### Phases

| Phase | Content |
|-------|---------|
| Phase 5a | `[build]` config parsing + `BuildStrategy` enum |
| Phase 5b | System dependency checks |
| Phase 5c | Cargo build integration |
| Phase 5d | Precompiled binary download + verification |
| Phase 5e | build.yx script execution |

### Dependencies

- Depends on RFC-014a (Registry protocol, used for downloading precompiled artifacts)
- Depends on the `sha2` crate (integrity verification)

## Open Questions

- [ ] Does build.yx need sandbox isolation?
- [ ] Maximum size limit for build artifacts?
- [ ] Is cross-compilation supported (building Windows artifacts on Linux)?
- [ ] How to handle Cargo version incompatibilities?

---

## References

- [Rust build.rs](https://doc.rust-lang.org/cargo/reference/build-scripts.html)
- [Python wheels](https://packaging.python.org/en/latest/guides/distributing-packages-using-setuptools/#wheels)
- [Go build constraints](https://pkg.go.dev/cmd/go#hdr-Build_constraints)
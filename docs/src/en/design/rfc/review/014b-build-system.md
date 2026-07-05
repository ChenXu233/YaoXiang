---
title: "RFC-014b: Build System and Binary Distribution"
status: "Under Review"
author: "晨煦"
created: "2026-06-11"
updated: "2026-07-05"
group: "rfc-014"
issue: "#91"
impl: "0%"
impl_status: "not-started"
---

# RFC-014b: Build System and Binary Distribution

> This RFC is a sub-RFC of [RFC-014: Package Management System Design](../accepted/014-package-manager.md).

## Summary

Defines the build mechanism for the YaoXiang package management system: declarative build configuration, build strategies (cargo/cmake/custom/none), precompiled binary distribution, and system dependency checks.

## Motivation

Some packages are pure `.yx` code and require no build. Some need compiled FFI bindings (calling Cargo, CMake, etc.). A unified mechanism is needed so that package authors can declare build requirements and the package manager can handle them automatically.

### Current Problems

- No build configuration declaration (no `[build]` section in `yaoxiang.toml`)
- No precompiled binary distribution mechanism
- FFI package builds depend entirely on manual user operation
- No system dependency check

## Proposal

### Core Design: Declarative Build + Precompiled Priority

Package authors declare build requirements in `yaoxiang.toml`, and the package manager makes decisions automatically based on the declaration.

### Build Strategy

```rust
enum BuildStrategy {
    None,          // Pure .yx package, no build required
    Cargo,         // Invoke cargo build, read [build.cargo] configuration
    Cmake,         // Invoke cmake
    Custom,        // Execute build.yx script
}
```

Note: The `Precompiled` variant has been removed. The presence of `[binaries]` automatically triggers precompiled-priority behavior; no explicit strategy declaration is needed.

### Build Declaration in yaoxiang.toml

```toml
[package]
name = "native-foo"
version = "1.0.0"

[build]
strategy = "cargo"              # Build strategy
headers = ["include/sqlite3.h"] # Optional: C header files handled by yx-bindgen

[build.cargo]
features = ["ffi"]             # cargo build --features ffi
target = "release"             # cargo build --release

[build.requirements]
cargo = ">= 1.70"              # Tools required at build time
cmake = ">= 3.20"

[build.platforms]              # Platform-specific overrides
"x86_64-unknown-linux-gnu" = { cargo-features = ["linux-ffi"] }
"x86_64-pc-windows-msvc" = { cargo-features = ["win-ffi"] }
"aarch64-apple-darwin" = { cargo-features = ["mac-ffi"] }
```

### Installation Decision Tree

```
yaoxiang install foo
    │
    ├─ 1. [binaries] has entry for current platform?
    │     → Yes: download, verify SHA-256, install directly (skip build)
    │     → No: continue
    │
    ├─ 2. Download source package
    │
    ├─ 3. [build].headers has value?
    │     → Yes: automatically run yx-bindgen to generate binding files
    │
    ├─ 4. Read [build].strategy
    │     → "none": install directly
    │     → "cargo": read [build.cargo] configuration, assemble cargo build command
    │     → "cmake": invoke cmake
    │     → "custom": execute build.yx script
    │
    └─ 5. Install to vendor/
```

**Precompiled priority, source code as fallback.** The presence of `[binaries]` automatically triggers precompiled check; no explicit strategy is needed.

### Cargo Strategy Details

When `strategy = "cargo"`, read `[build.cargo]` configuration to assemble the command:

```toml
[build]
strategy = "cargo"

[build.cargo]
features = ["ffi"]             # → cargo build --features ffi
target = "release"             # → cargo build --release

[build.platforms]              # Platform overrides
"x86_64-unknown-linux-gnu" = { cargo-features = ["linux-ffi"] }
"x86_64-pc-windows-msvc" = { cargo-features = ["win-ffi"] }
"aarch64-apple-darwin" = { cargo-features = ["mac-ffi"] }
```

Actual command executed:

```bash
# Base
cargo build --release --features ffi

# With platform override (Linux example)
cargo build --release --features ffi,linux-ffi
```

### Precompiled Binary Declaration

```toml
# yaoxiang.toml
[binaries]
"x86_64-unknown-linux-gnu" = { url = "releases/download/v1.0.0/foo-linux-x86_64.tar.gz", sha256 = "abc123" }
"x86_64-pc-windows-msvc" = { url = "https://example.com/foo-win-x86_64.tar.gz", sha256 = "def456" }
"aarch64-apple-darwin" = { url = "releases/download/v1.0.0/foo-macos-aarch64.tar.gz", sha256 = "ghi789" }
```

**URL format:** Supports both absolute URLs and relative paths. Relative paths are resolved against the package's repository address (GitHub repo URL or Registry root URL).

**Conditions to skip build:**
1. `[binaries]` has an entry for the current platform
2. SHA-256 verification passes
3. Download succeeds

All three conditions satisfied → skip build. Otherwise → fall back to source build.

### build.yx Build Script

When `strategy = "custom"`, execute `build.yx`.

**Execution model (minimal specification):**
- Script is regular `.yx` code with full `std` access
- Working directory: package root directory (`vendor/<pkg>-<ver>/`)
- Success: exit code 0
- Failure: non-zero exit code, installation aborted
- Package manager does not constrain script behavior, only checks exit code

```yx
# build.yx — package build script
use std.os
use std.io

fn main() {
    let platform = os.platform()
    let arch = os.arch()

    if os.file_exists("Cargo.toml") {
        io.println("Building native extension via Cargo...")
        let result = os.exec("cargo build --release")
        if result.exit_code != 0 {
            io.println("Build failed!")
            os.exit(1)
        }
    }

    io.println("Build complete!")
}
```

### System Dependency Check

All `[build.requirements]` are automatically checked before installation; an error is reported if not satisfied:

```
Error: Build requirement not satisfied
  cargo >= 1.70 required, but cargo is not installed
  Install: https://rustup.rs
```

### yx-bindgen Integration (headers field)

`[build].headers` declares C header files that need to be processed by yx-bindgen. The build system automatically runs yx-bindgen to generate `.yx` binding files.

```toml
[build]
strategy = "cargo"
headers = ["include/sqlite3.h", "include/json.h"]
```

Build flow:

```
1. [binaries] has precompiled? → skip all builds
2. [build].headers has value? → yx-bindgen automatically generates bindings
3. Execute [build].strategy (cargo/cmake/custom)
4. Install
```

yx-bindgen parses function signatures and type definitions from C header files (`.h`) and automatically generates `.yx` binding declarations. Users do not need to run it manually — the build system handles it automatically when it detects the `headers` configuration.

**Relationship with RFC-026:** RFC-026 defines the language-level semantics of `yx-bindgen` (`native("symbol")` syntax, unsafe type). RFC-014b defines its integration into the build flow (`headers` configuration). The two are complementary.

### Integration with Cargo Workspace

If the package contains FFI code, a Cargo workspace can be defined simultaneously:

```
my-package/
├── yaoxiang.toml          # YaoXiang package configuration
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

Use Rust target triple format (`arch-vendor-os-env`):

| Platform | Identifier |
|------|------|
| Linux x86_64 (glibc) | `x86_64-unknown-linux-gnu` |
| Linux x86_64 (musl) | `x86_64-unknown-linux-musl` |
| Linux ARM64 | `aarch64-unknown-linux-gnu` |
| Windows x86_64 (MSVC) | `x86_64-pc-windows-msvc` |
| Windows x86_64 (MinGW) | `x86_64-pc-windows-gnu` |
| macOS ARM64 | `aarch64-apple-darwin` |
| macOS x86_64 | `x86_64-apple-darwin` |

Rust target triples are used instead of simplified formats because:
1. They distinguish different ABIs on the same OS (gnu vs musl, msvc vs gnu)
2. They align with the Rust/Cargo ecosystem, reducing mapping errors
3. Future extensions require no format changes

### Build Artifact Directory Structure

```
build/
└── native/
    ├── x86_64-unknown-linux-gnu/
    │   └── libfoo.so
    ├── x86_64-pc-windows-msvc/
    │   └── foo.dll
    └── aarch64-apple-darwin/
        └── libfoo.dylib
```

### Complete Lifecycle of a Precompiled Package

```
Developer:
  1. Write .yx code + FFI bindings
  2. Declare [build] + [binaries] in yaoxiang.toml
  3. yaoxiang publish
     → Automatically build multi-platform binaries on CI
     → Upload source code + precompiled artifacts

User:
  yaoxiang add native-foo
    → Detects precompiled artifact → download directly (seconds)
    → No precompiled artifact → download source + execute build (minutes)
```

## Trade-offs

### Advantages

- Declarative configuration; users do not need to understand build details
- Precompiled priority; installation is extremely fast
- Multi-platform support with automatic selection
- Seamless integration with the Cargo ecosystem

### Disadvantages

- Precompiled artifacts require CI support
- Multi-platform builds increase release complexity
- build.yx scripts require sandbox security mechanisms

## Alternatives

| Alternative | Why Not Chosen |
|------|-----------|
| Pure source distribution | Users need to install build toolchain, high barrier to entry |
| Binary format like Python wheels | Too complex; not needed in the early YaoXiang ecosystem |
| No FFI build support | Limits the language's extensibility |

## Implementation Strategy

### Phase Division

| Phase | Content |
|------|------|
| Phase 5a | `[build]` configuration parsing + `BuildStrategy` enum |
| Phase 5b | System dependency check |
| Phase 5c | Cargo build integration (read `[build.cargo]` to assemble command) |
| Phase 5d | Precompiled binary download + verification |
| Phase 5e | build.yx script execution |
| Phase 5f | yx-bindgen integration (`headers` field) |

### Dependencies

- Depends on RFC-014a (Registry protocol, for downloading precompiled artifacts)
- Depends on the `sha2` crate (integrity verification)

## Open Questions

- [ ] Does the build.yx script require sandbox isolation?
- [ ] Maximum size limit for build artifacts?
- [ ] Is cross-compilation supported (building Windows artifacts on Linux)?
- [ ] How to handle Cargo version incompatibilities?

---

## References

- [Rust build.rs](https://doc.rust-lang.org/cargo/reference/build-scripts.html)
- [Python wheels](https://packaging.python.org/en/latest/guides/distributing-packages-using-setuptools/#wheels)
- [Go build constraints](https://pkg.go.dev/cmd/go#hdr-Build_constraints)
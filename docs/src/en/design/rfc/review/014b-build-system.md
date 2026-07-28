---
title: 'RFC-014b: Build System and Binary Distribution'
status: 'Under Review'
author: 'Chen Xu'
created: '2026-06-11'
updated: '2026-07-05'
group: 'rfc-014'
issue: '#91'
impl: '0%'
impl_status: 'not-started'
---

# RFC-014b: Build System and Binary Distribution

> This RFC is a sub-RFC of [RFC-014: Package Manager Design](../accepted/014-package-manager.md).

## Summary

Define the build mechanism for the YaoXiang package manager: declarative build configuration, build
strategies (cargo/cmake/custom/none), precompiled binary distribution, and system dependency
checking.

## Motivation

Some packages are pure `.yx` code that don't require building. Some need to compile FFI bindings
(calling Cargo, CMake, etc.). A unified mechanism is needed to allow package authors to declare
build requirements, letting the package manager handle them automatically.

### Current Problems

- No build configuration declaration (`yaoxiang.toml` lacks a `[build]` section)
- No precompiled binary distribution mechanism
- FFI package builds rely entirely on manual user operations
- No system dependency checking

## Proposal

### Core Design: Declarative Build + Precompiled First

Package authors declare build requirements in `yaoxiang.toml`, and the package manager automatically
makes decisions based on these declarations.

### Build Strategy

```rust
enum BuildStrategy {
    None,          // Pure .yx package, no build needed
    Cargo,         // Call cargo build, read [build.cargo] configuration
    Cmake,         // Call cmake
    Custom,        // Execute build.yx script
}
```

Note: The `Precompiled` variant has been removed. The existence of `[binaries]` automatically
triggers precompiled-first behavior, without requiring an explicit strategy declaration.

### Build Declaration in yaoxiang.toml

```toml
[package]
name = "native-foo"
version = "1.0.0"

[build]
strategy = "cargo"              # Build strategy
headers = ["include/sqlite3.h"] # Optional: C header files for yx-bindgen auto-processing

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
    ├─ 1. Does [binaries] have current platform entry?
    │     → Yes: Download, verify SHA-256, install directly (skip build)
    │     → No: Continue
    │
    ├─ 2. Download source package
    │
    ├─ 3. Does [build].headers have a value?
    │     → Yes: Auto-run yx-bindgen to generate binding files
    │
    ├─ 4. Read [build].strategy
    │     → "none": Install directly
    │     → "cargo": Read [build.cargo] config, construct cargo build command
    │     → "cmake": Call cmake
    │     → "custom": Execute build.yx script
    │
    └─ 5. Install to vendor/
```

**Precompiled first, source as fallback.** The existence of `[binaries]` automatically triggers
precompiled checking without requiring an explicit strategy.

### Cargo Strategy Details

When `strategy = "cargo"`, read `[build.cargo]` configuration to construct the command:

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

Actual commands executed:

```bash
# Basic
cargo build --release --features ffi

# With platform override (e.g., Linux)
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

**URL Format:** Supports both absolute URLs and relative paths. Relative paths are resolved relative
to the package's repository address (GitHub repo URL or Registry root URL).

**Conditions to skip build:**

1. `[binaries]` has an entry for the current platform
2. SHA-256 verification passes
3. Download succeeds

All three conditions met → Skip build. Otherwise → Fallback to source build.

### build.yx Build Script

Execute `build.yx` when `strategy = "custom"`.

**Execution Model (minimal specification):**

- Script is regular `.yx` code with full `std` access
- Working directory: package root (`vendor/<pkg>-<ver>/`)
- Success: exit code 0
- Failure: non-zero exit code, installation aborts
- Package manager does not constrain script behavior, only checks exit code

```yx
# build.yx — Package build script
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

### System Dependency Checking

Automatically check all `[build.requirements]` before installation, error if not satisfied:

```
Error: Build requirement not satisfied
  cargo >= 1.70 required, but cargo is not installed
  Install: https://rustup.rs
```

### yx-bindgen Integration (headers field)

`[build].headers` declares C header files that need yx-bindgen processing. The build system
automatically runs yx-bindgen to generate `.yx` binding files.

```toml
[build]
strategy = "cargo"
headers = ["include/sqlite3.h", "include/json.h"]
```

Build flow:

```
1. Does [binaries] have precompiled? → Skip all build
2. Does [build].headers have value? → yx-bindgen auto-generates bindings
3. Execute [build].strategy (cargo/cmake/custom)
4. Install
```

yx-bindgen parses function signatures and type definitions from C header files (`.h`), automatically
generating `.yx` binding declarations. Users don't need to run it manually—the build system handles
it automatically when `headers` configuration is detected.

**Relationship with RFC-026:** RFC-026 defines the language-level semantics of `yx-bindgen`
(`native("symbol")` syntax, unsafe types). RFC-014b defines its integration into the build flow
(`headers` configuration). The two are complementary.

### Integration with Cargo Workspace

If a package contains FFI code, you can define a Cargo workspace alongside it:

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

`yaoxiang build` automatically detects and calls `cargo build` to compile the native part.

## Detailed Design

### Platform Identifiers

Use Rust target triple format (`arch-vendor-os-env`):

| Platform               | Identifier                  |
| ---------------------- | --------------------------- |
| Linux x86_64 (glibc)   | `x86_64-unknown-linux-gnu`  |
| Linux x86_64 (musl)    | `x86_64-unknown-linux-musl` |
| Linux ARM64            | `aarch64-unknown-linux-gnu` |
| Windows x86_64 (MSVC)  | `x86_64-pc-windows-msvc`    |
| Windows x86_64 (MinGW) | `x86_64-pc-windows-gnu`     |
| macOS ARM64            | `aarch64-apple-darwin`      |
| macOS x86_64           | `x86_64-apple-darwin`       |

Using Rust target triples instead of simplified formats because:

1. Distinguishes different ABIs on the same OS (gnu vs musl, msvc vs gnu)
2. Aligns with Rust/Cargo ecosystem, reducing mapping errors
3. Future extensions don't require format changes

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

### Complete Lifecycle of Precompiled Packages

```
Developer:
  1. Write .yx code + FFI bindings
  2. Declare [build] + [binaries] in yaoxiang.toml
  3. yaoxiang publish
     → Auto build multi-platform binaries on CI
     → Upload source + precompiled artifacts

User:
  yaoxiang add native-foo
    → Detects precompiled artifacts → Direct download (seconds)
    → No precompiled artifacts → Download source + execute build (minutes)
```

## Trade-offs

### Advantages

- Declarative configuration, users don't need to understand build details
- Precompiled first, extremely fast installation
- Multi-platform support, automatic selection
- Seamless integration with Cargo ecosystem

### Disadvantages

- Precompiled artifacts require CI support
- Multi-platform builds increase release complexity
- build.yx scripts need sandbox security mechanisms

## Alternative Approaches

| Approach                        | Why Not Chosen                                                 |
| ------------------------------- | -------------------------------------------------------------- |
| Pure source distribution        | Users need to install build toolchain, high barrier            |
| Python wheel-like binary format | Too complex, not needed for YaoXiang ecosystem in early stages |
| No FFI build support            | Limits language extensibility                                  |

## Implementation Strategy

### Phase Breakdown

| Phase    | Content                                                            |
| -------- | ------------------------------------------------------------------ |
| Phase 5a | `[build]` configuration parsing + `BuildStrategy` enum             |
| Phase 5b | System dependency checking                                         |
| Phase 5c | Cargo build integration (read `[build.cargo]`, construct commands) |
| Phase 5d | Precompiled binary download + verification                         |
| Phase 5e | build.yx script execution                                          |
| Phase 5f | yx-bindgen integration (`headers` field)                           |

### Dependencies

- Depends on RFC-014a (Registry protocol, for downloading precompiled artifacts)
- Depends on `sha2` crate (integrity verification)

## Open Questions

- [ ] Does build.yx script need sandbox isolation?
- [ ] Maximum size limit for build artifacts?
- [ ] Cross-compilation support (build Windows artifacts on Linux)?
- [ ] How to handle incompatible Cargo versions?

---

## References

- [Rust build.rs](https://doc.rust-lang.org/cargo/reference/build-scripts.html)
- [Python wheels](https://packaging.python.org/en/latest/guides/distributing-packages-using-setuptools/#wheels)
- [Go build constraints](https://pkg.go.dev/cmd/go#hdr-Build_constraints)

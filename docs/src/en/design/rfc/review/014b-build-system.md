---
title: 'RFC-014b: Build System and Binary Distribution'
status: 'Under Review'
author: 'Chenxu'
created: '2026-06-11'
updated: '2026-09-15'
group: 'rfc-014'
issue: '#91'
impl: '0%'
impl_status: 'not-started'
---

# RFC-014b: Build System and Binary Distribution

> This RFC is a sub-RFC of
> [RFC-014: Package Management System Design](../accepted/014-package-manager.md).

## 2026-09-15 Review Resolution

The following resolutions were finalized by the owner on 2026-09-15:

1. **build.yx trust gate (grounding the "sandbox" open question)**: `custom` strategy executing
   arbitrary code is the largest supply chain attack surface for the package
   manager—`yaoxiang add`ing a malicious package is equivalent to handing over full `std.os`
   permissions. Minimum mandatory baseline:
   - Interactive confirmation must be required before first execution of a package's `build.yx`;
   - `yaoxiang add --trust <pkg>` / `install --trust` persists the trust record to
     `~/.yaoxiang/config.toml` (`[trust] build-scripts = ["name@version"]`);
   - Non-interactive environments (CI) reject `custom` builds by default, unless explicitly
     `--trust`;
   - Full sandbox mechanisms continue as an open question for research, but the trust gate is a
     non-omittable floor.
2. **Phase reordering**:
   `5a → 5b → 5c (cargo) → 5d ([binaries]) → 5f (bindgen, depends on RFC-026b) → 5e (custom last, with trust gate)`.
   Declarative path (cargo) first, arbitrary code execution last.
3. **Unified binary distribution**: `[binaries]` is the sole binary distribution mechanism
   (corresponding to RFC-014a resolution 3—.yxpkg only contains source).
4. **Cross-compilation**: Not supported in the initial phase; multi-platform artifacts are produced
   by CI on multiple hosts (aligned with RFC-037 cargo-dist approach).
5. **Build artifact size**: No upper limit; managed by the distribution channel.
6. **Cargo version incompatibility**: Handled via `[build.requirements]` pre-check error +
   installation guidance (defined in the body, no additional mechanism).

## Summary

Defines the build mechanism for the YaoXiang package management system: declarative build
configuration, build strategies (cargo/cmake/custom/none), pre-compiled binary distribution, and
system dependency checking.

## Motivation

Some packages are pure `.yx` code requiring no build. Some require compiling FFI bindings (calling
Cargo, CMake, etc.). A unified mechanism is needed for package authors to declare build
requirements, with the package manager handling them automatically.

### Current Problems

- No build configuration declaration (no `[build]` section in `yaoxiang.toml`)
- No pre-compiled binary distribution mechanism
- FFI package builds rely entirely on manual user operation
- No system dependency checking

## Proposal

### Core Design: Declarative Build + Pre-compiled Priority

Package authors declare build requirements in `yaoxiang.toml`, and the package manager makes
automatic decisions based on the declaration.

### Build Strategies

```rust
enum BuildStrategy {
    None,          // Pure .yx package, no build required
    Cargo,         // Invokes cargo build, reads [build.cargo] configuration
    Cmake,         // Invokes cmake
    Custom,        // Executes build.yx script
}
```

Note: The `Precompiled` variant has been removed. The presence of `[binaries]` automatically
triggers pre-compiled priority behavior, no explicit strategy declaration required.

### Build Declaration in yaoxiang.toml

```toml
[package]
name = "native-foo"
version = "1.0.0"

[build]
strategy = "cargo"              # Build strategy
headers = ["include/sqlite3.h"] # Optional: C headers auto-handled by yx-bindgen

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
    ├─ 1. Entry for current platform in [binaries]?
    │     → Yes: download, verify SHA-256, install directly (skip build)
    │     → No: continue
    │
    ├─ 2. Download source package
    │
    ├─ 3. [build].headers has values?
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

**Pre-compiled priority, source as fallback.** The presence of `[binaries]` automatically triggers
pre-compiled checking, no explicit strategy required.

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

Commands actually executed:

```bash
# Base
cargo build --release --features ffi

# With platform override (Linux example)
cargo build --release --features ffi,linux-ffi
```

### Pre-compiled Binary Declaration

```toml
# yaoxiang.toml
[binaries]
"x86_64-unknown-linux-gnu" = { url = "releases/download/v1.0.0/foo-linux-x86_64.tar.gz", sha256 = "abc123" }
"x86_64-pc-windows-msvc" = { url = "https://example.com/foo-win-x86_64.tar.gz", sha256 = "def456" }
"aarch64-apple-darwin" = { url = "releases/download/v1.0.0/foo-macos-aarch64.tar.gz", sha256 = "ghi789" }
```

**URL format:** Supports both absolute URLs and relative paths. Relative paths are relative to the
package's repository address (GitHub repo URL or Registry root URL).

**Conditions for skipping build:**

1. Current platform has an entry in `[binaries]`
2. SHA-256 verification passes
3. Download succeeds

All three conditions met → skip build. Otherwise → fallback to source build.

### build.yx Build Script

When `strategy = "custom"`, `build.yx` is executed.

**Execution model (minimum specification):**

- The script is ordinary `.yx` code with full `std` access permissions
- **Trust gate (2026-09-15 resolution, mandatory)**: Interactive confirmation required before first
  execution; non-interactive environments reject by default (see resolution 1 above)
- Working directory: package root directory (`vendor/<pkg>-<ver>/`)
- Success: exit code 0
- Failure: non-zero exit code, installation aborts
- The package manager does not constrain script behavior, only checks the exit code

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

### System Dependency Checking

All `[build.requirements]` are automatically checked before installation; if not satisfied, an error
is reported:

```
Error: Build requirement not satisfied
  cargo >= 1.70 required, but cargo is not installed
  Install: https://rustup.rs
```

### yx-bindgen Integration (headers field)

`[build].headers` declares C header files that need to be processed by yx-bindgen. The build system
automatically runs yx-bindgen to generate `.yx` binding files.

```toml
[build]
strategy = "cargo"
headers = ["include/sqlite3.h", "include/json.h"]
```

Build flow:

```
1. [binaries] has pre-compiled? → skip entire build
2. [build].headers has values? → yx-bindgen auto-generates bindings
3. Execute [build].strategy (cargo/cmake/custom)
4. Install
```

yx-bindgen parses function signatures and type definitions from C header files (`.h`), automatically
generating `.yx` binding declarations. Users do not need to run it manually—the build system
automatically handles this when it detects the `headers` configuration.

**Relationship with RFC-026:** RFC-026 defines the language-level semantics of `yx-bindgen`
(`native("symbol")` syntax, unsafe types). RFC-014b defines its integration into the build flow
(`headers` configuration). The two are complementary.

### Integration with Cargo Workspace

If the package contains FFI code, a Cargo workspace can be defined concurrently:

```
my-package/
├── yaoxiang.toml          # YaoXiang package configuration
├── Cargo.toml             # Cargo workspace (FFI portion)
├── src/
│   └── lib.yx             # YaoXiang code
└── native/
    ├── Cargo.toml          # Rust FFI code
    └── src/
        └── lib.rs
```

`yaoxiang build` automatically detects and calls `cargo build` to compile the native portion.

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

Rust target triples are used instead of a simplified format because:

1. To distinguish different ABIs on the same OS (gnu vs musl, msvc vs gnu)
2. To align with the Rust/Cargo ecosystem, reducing mapping errors
3. Future extensions do not require format changes

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

### Complete Lifecycle of a Pre-compiled Package

```
Developer:
  1. Write .yx code + FFI bindings
  2. Declare [build] + [binaries] in yaoxiang.toml
  3. yaoxiang publish
     → Automatically build multi-platform binaries on CI
     → Upload source + pre-compiled artifacts

User:
  yaoxiang add native-foo
    → Detects pre-compiled artifacts → direct download (seconds)
    → No pre-compiled artifacts → download source + execute build (minutes)
```

## Trade-offs

### Advantages

- Declarative configuration; users need not understand build details
- Pre-compiled priority; extremely fast installation
- Multi-platform support with automatic selection
- Seamless integration with the Cargo ecosystem

### Disadvantages

- Pre-compiled artifacts require CI support
- Multi-platform builds increase release complexity
- build.yx scripts require sandbox security mechanisms

## Alternatives

| Approach                        | Why Not Chosen                                       |
| ------------------------------- | ---------------------------------------------------- |
| Pure source distribution        | Users need to install build toolchain, high barrier  |
| Python wheel-like binary format | Too complex, unnecessary in early YaoXiang ecosystem |
| No FFI build support            | Limits the language's extension capabilities         |

## Implementation Strategy

### Phase Division

| Phase    | Content                                                              |
| -------- | -------------------------------------------------------------------- |
| Phase 5a | `[build]` configuration parsing + `BuildStrategy` enum               |
| Phase 5b | System dependency checking                                           |
| Phase 5c | Cargo build integration (reads `[build.cargo]` to assemble commands) |
| Phase 5d | Pre-compiled binary download + verification                          |
| Phase 5f | yx-bindgen integration (`headers` field, depends on RFC-026b)        |
| Phase 5e | build.yx script execution (**implemented last**, with trust gate)    |

Execution order (2026-09-15 resolution 2): `5a → 5b → 5c → 5d → 5f → 5e`. Declarative build first,
arbitrary code execution last.

### Dependencies

- Depends on RFC-014a (Registry protocol, for downloading pre-compiled artifacts)
- Depends on `sha2` crate (integrity verification)

## Open Questions

- [x] Does build.yx script require sandbox isolation? → Trust gate as mandatory floor (2026-09-15
      resolution 1); full sandbox continues research
- [x] Maximum size limit for build artifacts? → No upper limit, managed by channel (2026-09-15
      resolution 5)
- [x] Support cross-compilation (build Windows artifacts on Linux)? → Not supported initially; CI
      multi-host production (2026-09-15 resolution 4)
- [x] How to handle Cargo version incompatibility? → `[build.requirements]` pre-check error +
      installation guidance (2026-09-15 resolution 6)

---

## References

- [Rust build.rs](https://doc.rust-lang.org/cargo/reference/build-scripts.html)
- [Python wheels](https://packaging.python.org/en/latest/guides/distributing-packages-using-setuptools/#wheels)
- [Go build constraints](https://pkg.go.dev/cmd/go#hdr-Build_constraints)

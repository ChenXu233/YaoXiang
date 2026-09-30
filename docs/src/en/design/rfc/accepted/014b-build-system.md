---
title: 'RFC-014b: Build System and Binary Distribution'
status: 'Accepted'
author: 'Chenxu'
created: '2026-06-11'
updated: '2026-09-30'
group: 'rfc-014'
issue: '#91'
impl: '90%'
impl_status: 'in-progress'
---

# RFC-014b: Build System and Binary Distribution

> This RFC is a sub-RFC of [RFC-014: Package Manager Design](../accepted/014-package-manager.md).

## 2026-09-15 Review Decision

The following decisions were finalized by the owner on 2026-09-15:

1. **`build.yx` trust gate (resolution to the "sandbox" open question)**: executing arbitrary code
   under the `custom` strategy is the largest supply-chain attack surface in the package
   manager—`yaoxiang add` on a malicious package equals handing over full `std.os` permissions.
   Minimum mandatory baseline:
   - Interactive confirmation is required before the first execution of a package's `build.yx`;
   - `yaoxiang add --trust <pkg>` / `install --trust` persists the trust record to
     `~/.yaoxiang/config.toml` (`[trust] build-scripts = ["name@version"]`);
   - Non-interactive environments (CI) reject `custom` builds by default unless `--trust` is
     explicit;
   - A full sandbox mechanism continues to be researched as an open question, but the trust gate is
     an indispensable floor.
2. **Phase reordering**:
   `5a → 5b → 5c (cargo) → 5d ([binaries]) → 5f (bindgen, depends on RFC-026b) → 5e (custom last, with trust gate)`.
   Declarative path (cargo) first, arbitrary code execution last.
3. **Binary distribution unification**: `[binaries]` is the only binary distribution mechanism
   (corresponding to RFC-014a decision 3—`.yxpkg` only contains source code).
4. **Cross-compilation**: Not supported initially; multi-platform artifacts are produced by CI on
   multiple hosts (aligned with RFC-037 cargo-dist thinking).
5. **Build artifact size**: No upper limit is set; the distribution channel manages it.
6. **Cargo version incompatibility**: Use `[build.requirements]` pre-check error + installation
   guide (defined in the main text, no additional mechanism).

## Summary

Defines the build mechanism for the YaoXiang package management system: declarative build
configuration, build strategies (cargo/cmake/custom/none), pre-compiled binary distribution, and
system dependency checks.

## Motivation

Some packages are pure `.yx` code and require no building. Some require compiled FFI bindings
(calling Cargo, CMake, etc.). A unified mechanism is needed to let package authors declare build
requirements and let the package manager handle them automatically.

### Current Problems

- No build configuration declaration (no `[build]` section in `yaoxiang.toml`)
- No pre-compiled binary distribution mechanism
- FFI package builds depend entirely on manual user operations
- No system dependency check

## Proposal

### Core Design: Declarative Build + Pre-compiled First

Package authors declare build requirements in `yaoxiang.toml`, and the package manager makes
decisions automatically based on the declarations.

### Build Strategy

```rust
enum BuildStrategy {
    None,          // Pure .yx package, no build needed
    Cargo,         // Invoke cargo build, read [build.cargo] config
    Cmake,         // Invoke cmake
    Custom,        // Execute build.yx script
}
```

Note: The `Precompiled` variant has been removed. The presence of `[binaries]` automatically
triggers pre-compiled-first behavior; no explicit strategy declaration is needed.

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
    ├─ 1. [binaries] has entry for current platform?
    │     → Yes: download, verify SHA-256, install directly (skip build)
    │     → No: continue
    │
    ├─ 2. Download source package
    │
    ├─ 3. [build].headers has value?
    │     → Yes: auto-run yx-bindgen to generate binding files
    │
    ├─ 4. Read [build].strategy
    │     → "none": install directly
    │     → "cargo": read [build.cargo] config, assemble cargo build command
    │     → "cmake": invoke cmake
    │     → "custom": execute build.yx script
    │
    └─ 5. Install to vendor/
```

**Pre-compiled first, source as fallback.** The presence of `[binaries]` automatically triggers the
pre-compiled check; no explicit strategy is needed.

### Cargo Strategy in Detail

When `strategy = "cargo"`, read the `[build.cargo]` config to assemble the command:

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

Actual executed command:

```bash
# Basic
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

**URL format:** Both absolute URLs and relative paths are supported. Relative paths are relative to
the package's repository address (GitHub repo URL or Registry root URL).

**Conditions to skip building:**

1. `[binaries]` has an entry for the current platform
2. SHA-256 verification passes
3. Download succeeds

All three conditions met → skip build. Otherwise → fallback to source build.

### build.yx Build Script

When `strategy = "custom"`, execute `build.yx`.

**Execution model (minimal specification):**

- The script is normal `.yx` code with full `std` access permissions
- **Trust gate (2026-09-15 decision, mandatory)**: interactive confirmation is required before first
  execution; non-interactive environments reject by default (see decision 1 above)
- Working directory: package root directory (`vendor/<pkg>-<ver>/`)
- Success: exit code 0
- Failure: non-zero exit code, installation aborts
- The package manager does not constrain script behavior; only checks the exit code

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

### System Dependency Check

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
1. [binaries] has pre-compiled? → skip all building
2. [build].headers has value? → yx-bindgen auto-generates bindings
3. Execute [build].strategy (cargo/cmake/custom)
4. Install
```

yx-bindgen parses function signatures and type definitions from C header files (`.h`) and
automatically generates `.yx` binding declarations. Users don't need to run it manually—the build
system handles it automatically when `headers` configuration is detected.

**Relationship to RFC-026:** RFC-026 defines the language-level semantics of `yx-bindgen`
(`native("symbol")` syntax, unsafe types). RFC-014b defines its integration into the build flow
(`headers` configuration). The two are complementary.

### Integration with Cargo Workspace

If the package contains FFI code, a Cargo workspace can be defined simultaneously:

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

Uses the Rust target triple format (`arch-vendor-os-env`):

| Platform               | Identifier                  |
| ---------------------- | --------------------------- |
| Linux x86_64 (glibc)   | `x86_64-unknown-linux-gnu`  |
| Linux x86_64 (musl)    | `x86_64-unknown-linux-musl` |
| Linux ARM64            | `aarch64-unknown-linux-gnu` |
| Windows x86_64 (MSVC)  | `x86_64-pc-windows-msvc`    |
| Windows x86_64 (MinGW) | `x86_64-pc-windows-gnu`     |
| macOS ARM64            | `aarch64-apple-darwin`      |
| macOS x86_64           | `x86_64-apple-darwin`       |

Using the Rust target triple instead of a simplified format because:

1. Distinguishes different ABIs on the same OS (gnu vs musl, msvc vs gnu)
2. Aligns with the Rust/Cargo ecosystem, reducing mapping errors
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

### Full Lifecycle of a Pre-compiled Package

```
Developer:
  1. Write .yx code + FFI bindings
  2. Declare [build] + [binaries] in yaoxiang.toml
  3. yaoxiang publish
     → Auto-build multi-platform binaries on CI
     → Upload source + pre-compiled artifacts

User:
  yaoxiang add native-foo
    → Detects pre-compiled artifacts → download directly (seconds)
    → No pre-compiled artifacts → download source + execute build (minutes)
```

## Trade-offs

### Advantages

- Declarative configuration; users don't need to understand build details
- Pre-compiled first; extremely fast installation
- Multi-platform support; automatic selection
- Seamless integration with the Cargo ecosystem

### Disadvantages

- Pre-compiled artifacts require CI support
- Multi-platform builds increase release complexity
- build.yx scripts need sandbox security mechanisms

## Alternatives

| Option                           | Why not chosen                                        |
| -------------------------------- | ----------------------------------------------------- |
| Pure source distribution         | Users need to install build toolchains; high barrier  |
| Binary format like Python wheels | Too complex; not needed in YaoXiang's early ecosystem |
| No FFI build support             | Limits the language's extensibility                   |

## Implementation Strategy

### Phase Division

| Phase    | Content                                                                                                                | Status                                                |
| -------- | ---------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------- |
| Phase 5a | `[build]` config parsing + `BuildStrategy` enum (including `[binaries]` declaration, platform triples)                 | ✅ Completed                                          |
| Phase 5b | System dependency check (`<tool> --version` detection + comparator operand completion + installation guide)            | ✅ Completed                                          |
| Phase 5c | Cargo build integration (read `[build.cargo]` to assemble command + platform override merging + scratch isolation)     | ✅ Completed                                          |
| Phase 5d | Pre-compiled binary download + verification (whole-package SHA-256 + safe unpacking + fallback semantics)              | ✅ Completed                                          |
| Phase 5f | yx-bindgen integration (`headers` field config pipeline; RFC-026b is still a draft, clear error reported at execution) | ✅ Config pipeline completed (generator follows 026b) |
| Phase 5e | build.yx script execution (**implemented last**, with trust gate)                                                      | ✅ Completed                                          |

Execution order (2026-09-15 decision 2): `5a → 5b → 5c → 5d → 5f → 5e`. Declarative build first,
arbitrary code execution last.

**Implementation notes (2026-09-30, commit see feat/rfc014)**:

- **Two-layer separation of artifacts**: cargo scratch (target/) is pointed to the project
  `.yaoxiang/build/cargo/<pkg>/` via `CARGO_TARGET_DIR`—it does not land in the vendor package
  directory, otherwise the directory integrity checksum is bloated by incremental build artifacts;
  FFI-consumable library files (.so/.dll/.dylib/.a) are copied to the vendor package directory
  `build/native/<triple>/`. The vendor integrity check semantics are thus clarified as **source tree
  integrity** (`build/` derived artifacts are not included in the checksum).
- **`[binaries]` fallback semantics**: when the current platform has an entry but any of "sha256 not
  declared / download failed / checksum mismatch" occurs → stderr message + fallback to source build
  (RFC "otherwise fallback"), clean up half-finished products before fallback. After the
  whole-package verification passes, go through safe unpacking (path traversal protection +
  unpacking total size guard 512 MiB; artifact size has no hard upper limit—decision 5).
- **Trust gate (decision 1) implementation**: trust records follow the user configuration system
  (`[trust] build-scripts` in `~/.config/yaoxiang/config.toml`; the `~/.yaoxiang/config.toml`
  mentioned in the RFC draft never existed, merged in like `[cache] dir`). Three allow paths:
  already recorded / this invocation's `--trust` (**allow means persist**, fulfilling "add/install
  --trust persists the trust record") / interactive confirmation (confirmation means persist).
  Non-interactive environments (stdin not a terminal) reject by default; only `--trust` can pass.
- **build.yx execution model**: ordinary .yx script, **top-level statements are the build logic**
  (Script/eval semantics; the `fn main()` example above is draft-stage pseudocode), executed
  in-process, working directory temporarily switched to package root (global lock serializes the
  flip window). The current std has no exec/exit API—scripts can write generation/file-type logic;
  `os.exec`-style external tool invocation follows std evolution.
- **cmake strategy**: enum and config parsing are ready, execution is not yet implemented (explicit
  error reported)—the RFC phase table does not have a separate cmake phase, scheduled when there is
  a real-need package.
- **publish pre-release test** (014a validation 3) is wired in with this Phase: by default runs
  tests discovered by `[tool.test]` (RFC-036 mechanism), failure aborts release, `--no-test` skips.

### Dependencies

- Depends on RFC-014a (Registry protocol, for downloading pre-compiled artifacts)
- Depends on the `sha2` crate (integrity verification)

## Open Questions

- [x] Does the build.yx script need sandbox isolation? → Trust gate is the mandatory floor
      (2026-09-15 decision 1); full sandbox continues to be researched
- [x] Maximum size limit for build artifacts? → No upper limit, channel self-manages (2026-09-15
      decision 5)
- [x] Support cross-compilation (build Windows artifacts on Linux)? → Not supported initially, CI
      multi-host output (2026-09-15 decision 4)
- [x] How to handle Cargo version incompatibility? → `[build.requirements]` pre-check error +
      installation guide (2026-09-15 decision 6)

---

## References

- [Rust build.rs](https://doc.rust-lang.org/cargo/reference/build-scripts.html)
- [Python wheels](https://packaging.python.org/en/latest/guides/distributing-packages-using-setuptools/#wheels)
- [Go build constraints](https://pkg.go.dev/cmd/go#hdr-Build_constraints)

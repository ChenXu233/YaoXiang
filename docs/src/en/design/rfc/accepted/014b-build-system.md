---
title: 'RFC-014b: Build System and Binary Distribution'
status: 'Accepted'
author: 'Chenxu'
created: '2026-06-11'
updated: '2026-10-03'
group: 'rfc-014'
issue: '#113'
impl: '100%'
impl_status: 'complete'
---

# RFC-014b: Build System and Binary Distribution

> This RFC is a sub-RFC of
> [RFC-014: Package Management System Design](../accepted/014-package-manager.md).

## 2026-09-15 Review Decision

The following decisions were finalized by the owner on 2026-09-15:

1. **build.yx trust gate (resolution of the "sandbox" open question)**: executing arbitrary code
   under the `custom` strategy is the largest supply-chain attack surface in the package manager —
   `yaoxiang add` on a malicious package is equivalent to handing over full `std.os` privileges.
   Minimum mandatory baseline:
   - Interactive confirmation is required before executing a package's `build.yx` for the first
     time;
   - `yaoxiang add --trust <pkg>` / `install --trust` persists the trust record to
     `~/.yaoxiang/config.toml` (`[trust] build-scripts = ["name@version"]`);
   - Non-interactive environments (CI) reject `custom` builds by default, unless `--trust` is
     explicitly given;
   - A complete sandbox mechanism continues to be researched as an open question, but the trust gate
     is a non-omittable floor.
2. **Phase reordering**:
   `5a → 5b → 5c (cargo) → 5d ([binaries]) → 5f (bindgen, depends on RFC-026b) → 5e (custom last, with trust gate)`.
   The declarative path (cargo) first, arbitrary code execution last.
3. **Unified binary distribution**: `[binaries]` is the sole binary distribution mechanism
   (corresponding to RFC-014a decision 3 — `.yxpkg` contains only source code).
4. **Cross-compilation**: Not supported initially; multi-platform artifacts are produced by CI on
   multiple hosts (aligned with the RFC-037 cargo-dist approach).
5. **Build artifact size**: No upper limit; managed by the distribution channel itself.
6. **Cargo version incompatibility**: Use `[build.requirements]` pre-check error reporting +
   installation guidance (defined in the main text, no additional mechanism).

## Summary

Defines the build mechanism for the YaoXiang package management system: declarative build
configuration, build strategies (cargo/cmake/custom/none), pre-compiled binary distribution, and
system dependency checking.

## Motivation

Some packages are pure `.yx` code and need no build. Others require compiling FFI bindings (calling
Cargo, CMake, etc.). A unified mechanism is needed to let package authors declare build requirements
and let the package manager handle them automatically.

### Current Problems

- No build configuration declaration (no `[build]` section in `yaoxiang.toml`)
- No pre-compiled binary distribution mechanism
- FFI package builds rely entirely on manual user operations
- No system dependency checking

## Proposal

### Core Design: Declarative Build + Pre-compiled First

Package authors declare build requirements in `yaoxiang.toml`, and the package manager makes
decisions automatically based on the declaration.

### Build Strategy

```rust
enum BuildStrategy {
    None,          // Pure .yx package, no build needed
    Cargo,         // Call cargo build, read [build.cargo] configuration
    Cmake,         // Call cmake
    Custom,        // Execute build.yx script
}
```

Note: The `Precompiled` variant has been removed. The presence of `[binaries]` automatically
triggers pre-compiled-first behavior, without an explicit strategy declaration.

### Build Declaration in yaoxiang.toml

```toml
[package]
name = "native-foo"
version = "1.0.0"

[build]
strategy = "cargo"              # build strategy
headers = ["include/sqlite3.h"] # optional: C headers handled automatically by yx-bindgen

[build.cargo]
features = ["ffi"]             # cargo build --features ffi
target = "release"             # cargo build --release

[build.requirements]
cargo = ">= 1.70"              # tools required at build time
cmake = ">= 3.20"

[build.platforms]              # platform-specific overrides
"x86_64-unknown-linux-gnu" = { cargo-features = ["linux-ffi"] }
"x86_64-pc-windows-msvc" = { cargo-features = ["win-ffi"] }
"aarch64-apple-darwin" = { cargo-features = ["mac-ffi"] }
```

### Install Decision Tree

```
yaoxiang install foo
    │
    ├─ 1. [binaries] has an entry for the current platform?
    │     → Yes: download, verify SHA-256, install directly (skip build)
    │     → No: continue
    │
    ├─ 2. Download the source package
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

**Pre-compiled first, source as fallback.** The presence of `[binaries]` automatically triggers the
pre-compiled check, no explicit strategy is required.

### Cargo Strategy Details

When `strategy = "cargo"`, read the `[build.cargo]` configuration and assemble the command:

```toml
[build]
strategy = "cargo"

[build.cargo]
features = ["ffi"]             # → cargo build --features ffi
target = "release"             # → cargo build --release

[build.platforms]              # platform override
"x86_64-unknown-linux-gnu" = { cargo-features = ["linux-ffi"] }
"x86_64-pc-windows-msvc" = { cargo-features = ["win-ffi"] }
"aarch64-apple-darwin" = { cargo-features = ["mac-ffi"] }
```

Actual executed commands:

```bash
# Basic
cargo build --release --features ffi

# With platform override (linux example)
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

1. There is an entry for the current platform in `[binaries]`
2. SHA-256 check passes
3. Download succeeds

All three conditions met → skip build. Otherwise → fall back to source build.

### build.yx Build Script

When `strategy = "custom"`, execute `build.yx`.

**Execution model (minimal specification):**

- The script is ordinary `.yx` code with full `std` access
- **Trust gate (2026-09-15 decision, mandatory)**: Interactive confirmation is required before first
  execution; non-interactive environments reject by default (see decision 1 above)
- Working directory: package root (`vendor/<pkg>-<ver>/`)
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

### System Dependency Check

All `[build.requirements]` are automatically checked before installation; if unsatisfied, an error
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
1. [binaries] has pre-compiled? → skip all builds
2. [build].headers has values? → yx-bindgen automatically generates bindings
3. Execute [build].strategy (cargo/cmake/custom)
4. Install
```

yx-bindgen parses function signatures and type definitions from C header files (`.h`) and
automatically generates `.yx` binding declarations. Users do not need to run it manually — the build
system handles it automatically when `headers` configuration is detected.

**Relationship with RFC-026:** RFC-026 defines the language-level semantics of `yx-bindgen`
(`native("symbol")` syntax, unsafe types). RFC-014b defines its integration into the build flow
(`headers` configuration). The two complement each other.

### Integration with Cargo Workspace

If a package contains FFI code, a Cargo workspace can also be defined:

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

### Platform Identifier

Use the Rust target triple format (`arch-vendor-os-env`):

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

1. They distinguish different ABIs on the same OS (gnu vs musl, msvc vs gnu)
2. They align with the Rust/Cargo ecosystem, reducing mapping errors
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

### Full Lifecycle of a Pre-compiled Package

```
Developer:
  1. Write .yx code + FFI bindings
  2. Declare [build] + [binaries] in yaoxiang.toml
  3. yaoxiang publish
     → Automatically build multi-platform binaries on CI
     → Upload source + pre-compiled artifacts

User:
  yaoxiang add native-foo
    → Detected pre-compiled artifact → download directly (seconds)
    → No pre-compiled artifact → download source + execute build (minutes)
```

## Trade-offs

### Advantages

- Declarative configuration, users don't need to understand build details
- Pre-compiled first, extremely fast installation
- Multi-platform support, automatic selection
- Seamless integration with the Cargo ecosystem

### Disadvantages

- Pre-compiled artifacts require CI support
- Multi-platform builds increase release complexity
- build.yx scripts require sandbox security mechanisms

## Alternatives

| Approach                        | Why Not Chosen                                        |
| ------------------------------- | ----------------------------------------------------- |
| Pure source distribution        | Users need to install toolchains, high barrier        |
| Python wheel-like binary format | Too complex, unnecessary for early YaoXiang ecosystem |
| No FFI build support            | Limits the language's extensibility                   |

## Implementation Strategy

### Phase Breakdown

| Phase    | Content                                                                                                            | Status                                                      |
| -------- | ------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------- |
| Phase 5a | `[build]` configuration parsing + `BuildStrategy` enum (including `[binaries]` declaration, platform triples)      | ✅ Completed                                                |
| Phase 5b | System dependency check (`<tool> --version` probing + comparator operand completion + installation guidance)       | ✅ Completed                                                |
| Phase 5c | Cargo build integration (read `[build.cargo]` to assemble command + platform override merging + scratch isolation) | ✅ Completed                                                |
| Phase 5d | Pre-compiled binary download + verification (whole-package SHA-256 + safe extraction + fallback semantics)         | ✅ Completed                                                |
| Phase 5f | yx-bindgen integration (`headers` field configuration pipeline; RFC-026b still draft, explicit error at execution) | ✅ Configuration pipeline complete (generator follows 026b) |
| Phase 5e | build.yx script execution (**implemented last**, with trust gate)                                                  | ✅ Completed                                                |

Execution order (2026-09-15 decision 2): `5a → 5b → 5c → 5d → 5f → 5e`. Declarative build first,
arbitrary code execution last.

**Implementation notes (2026-09-30, see commit feat/rfc014)**:

- **Two-layer artifact separation**: cargo scratch (target/) is pointed via `CARGO_TARGET_DIR` to
  the project `.yaoxiang/build/cargo/<pkg>/` — it does not land in the vendor package directory,
  otherwise directory integrity checksums get bloated by incremental build artifacts; FFI-consumable
  library files (.so/.dll/.dylib/.a) are copied to the vendor package directory
  `build/native/<triple>/`. The vendor integrity check semantics are accordingly clarified as
  **source tree integrity** (`build/` derived artifacts are not included in checksums).
- **`[binaries]` fallback semantics**: When the current platform has an entry but any of "sha256 not
  declared / download failed / checksum mismatch" occurs → stderr warning + fall back to source
  build (per the RFC "otherwise fallback"), clean up half-finished artifacts before falling back.
  After the whole-package check passes, safe extraction is performed (path traversal protection +
  extraction total size guardrail of 512 MiB; artifact size has no hard upper limit — decision 5).
- **Trust gate (decision 1) implementation**: The trust record follows the user configuration system
  (`~/.config/yaoxiang/config.toml`'s `[trust] build-scripts`; the `~/.yaoxiang/config.toml`
  mentioned during RFC drafting never existed, merged like `[cache] dir`). Three pass-through paths:
  already recorded / this run's `--trust` (**pass-through is persisted**, fulfilling "add/install
  --trust persists the trust record") / interactive confirmation (confirmation is persisted).
  Non-interactive environments (stdin not a terminal) reject by default; only `--trust` can pass.
- **build.yx execution model**: Ordinary .yx script, **top-level statements are the build logic**
  (Script/eval semantics; the `fn main()` example above is draft-stage pseudo-code), executed
  in-process, working directory temporarily switched to the package root (global lock serializes the
  flip window). The current std has no exec/exit API — the script can write generation/file-style
  logic; `os.exec`-style calls to external tools will follow std evolution.
- **cmake strategy**: Enum and configuration parsing are ready, execution is not yet implemented
  (explicit error reported) — the RFC phase table does not list a separate cmake phase; will be
  scheduled when a real-needs package appears.
- **publish pre-release testing** (014a validation 3) has been wired up with this phase: by default,
  tests discovered by `[tool.test]` are run (RFC-036 mechanism), failure aborts the release;
  `--no-test` skips.

### Dependencies

- Depends on RFC-014a (Registry protocol, for downloading pre-compiled artifacts)
- Depends on the `sha2` crate (integrity verification)

## Open Questions

- [x] Does build.yx script need sandbox isolation? → Trust gate as mandatory floor (2026-09-15
      decision 1); full sandbox continues to be researched
- [x] Maximum size limit for build artifacts? → No upper limit, channel manages itself (2026-09-15
      decision 5)
- [x] Is cross-compilation supported (build Windows artifacts on Linux)? → Not supported initially,
      CI multi-host production (2026-09-15 decision 4)
- [x] How to handle Cargo version incompatibility? → `[build.requirements]` pre-check error +
      installation guidance (2026-09-15 decision 6)

---

## References

- [Rust build.rs](https://doc.rust-lang.org/cargo/reference/build-scripts.html)
- [Python wheels](https://packaging.python.org/en/latest/guides/distributing-packages-using-setuptools/#wheels)
- [Go build constraints](https://pkg.go.dev/cmd/go#hdr-Build_constraints)

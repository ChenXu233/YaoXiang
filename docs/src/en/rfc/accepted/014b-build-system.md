---
title: 'RFC-014b: Build System and Binary Distribution'
status: 'Accepted'
author: 'Morning Glow'
created: '2026-06-11'
updated: '2026-10-03'
group: 'rfc-014'
issue: '#113'
impl: '100%'
impl_status: 'complete'
---

# RFC-014b: Build System and Binary Distribution

> This RFC is a sub-RFC of [RFC-014: Package Management System Design](014-package-manager.md).

## 2026-09-15 Review Resolution

The following resolutions were decided by the owner on 2026-09-15:

1. **build.yx trust gate (landing for the "sandbox" open question)**: The `custom` strategy
   executing arbitrary code is the largest supply chain attack surface in the package
   manager—`yaoxiang add`ing a malicious package equals handing over full `std.os` permissions. The
   minimum mandatory baseline:
   - Interactive confirmation is required before the first execution of a package's `build.yx`;
   - `yaoxiang add --trust <pkg>` / `install --trust` persists the trust record to
     `~/.yaoxiang/config.toml` (`[trust] build-scripts = ["name@version"]`);
   - Non-interactive environments (CI) reject `custom` builds by default, unless `--trust` is
     explicitly given;
   - The full sandbox mechanism continues to be researched as an open question, but the trust gate
     is an indispensable lower bound.
2. **Phase reordering**:
   `5a → 5b → 5c (cargo) → 5d ([binaries]) → 5f (bindgen, depends on RFC-026b) → 5e (custom last, with trust gate)`.
   The declarative path (cargo) comes first, arbitrary code execution comes last.
3. **Unified binary distribution**: `[binaries]` is the sole binary distribution mechanism
   (corresponding to RFC-014a resolution 3—`.yxpkg` only contains source code).
4. **Cross-compilation**: Not supported initially; multi-platform artifacts are produced by CI on
   multiple hosts (aligned with the RFC-037 cargo-dist approach).
5. **Build artifact size**: No upper limit; managed by distribution channels themselves.
6. **Cargo version incompatibility**: Go through `[build.requirements]` pre-check error +
   installation guide (defined in the main text, no additional mechanism).

## Summary

Defines the build mechanism for the YaoXiang package management system: declarative build
configuration, build strategies (cargo/cmake/custom/none), precompiled binary distribution, and
system dependency checking.

## Motivation

Some packages are pure `.yx` code and need no build. Some require compiled FFI bindings (calling
Cargo, CMake, etc.). A unified mechanism is needed to let package authors declare build
requirements, and let the package manager handle them automatically.

### Current Problems

- No build configuration declaration (no `[build]` section in `yaoxiang.toml`)
- No precompiled binary distribution mechanism
- Build of FFI packages completely depends on manual user operations
- No system dependency checking

## Proposal

### Core Design: Declarative Build + Precompiled First

Package authors declare build requirements in `yaoxiang.toml`, and the package manager makes
decisions automatically based on the declarations.

### Build Strategies

```rust
enum BuildStrategy {
    None,          // Pure .yx package, no build needed
    Cargo,         // Call cargo build, read [build.cargo] configuration
    Cmake,         // Call cmake
    Custom,        // Execute build.yx script
}
```

Note: The `Precompiled` variant has been removed. The presence of `[binaries]` automatically
triggers precompiled-first behavior, no need to explicitly declare strategy.

### Build Declaration in yaoxiang.toml

```toml
[package]
name = "native-foo"
version = "1.0.0"

[build]
strategy = "cargo"              # Build strategy
headers = ["include/sqlite3.h"] # Optional: C header files to be processed automatically by yx-bindgen

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
    ├─ 1. Does [binaries] have an entry for the current platform?
    │     → Yes: download, verify SHA-256, install directly (skip build)
    │     → No: continue
    │
    ├─ 2. Download the source package
    │
    ├─ 3. Does [build].headers have a value?
    │     → Yes: automatically run yx-bindgen to generate binding files
    │
    ├─ 4. Read [build].strategy
    │     → "none": install directly
    │     → "cargo": read [build.cargo] configuration, assemble cargo build command
    │     → "cmake": call cmake
    │     → "custom": execute build.yx script
    │
    └─ 5. Install to vendor/
```

**Precompiled first, source as fallback.** The presence of `[binaries]` automatically triggers the
precompiled check, no explicit strategy is needed.

### Cargo Strategy Details

When `strategy = "cargo"`, read the `[build.cargo]` configuration to assemble the command:

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
# Base
cargo build --release --features ffi

# When platform overrides exist (using linux as example)
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

**URL format:** Supports both absolute URLs and relative paths. Relative paths are relative to the
package's repository address (GitHub repo URL or Registry root URL).

**Conditions for skipping build:**

1. `[binaries]` has an entry for the current platform
2. SHA-256 verification passes
3. Download succeeds

All three conditions met → skip build. Otherwise → fall back to source build.

### build.yx Build Script

When `strategy = "custom"`, execute `build.yx`.

**Execution model (minimal specification):**

- The script is regular `.yx` code with full `std` access permissions
- **Trust gate (2026-09-15 resolution, mandatory)**: Interactive confirmation is required before
  first execution; non-interactive environments reject by default (see resolution 1 above)
- Working directory: package root directory (`vendor/<pkg>-<ver>/`)
- Success: exit code 0
- Failure: non-zero exit code, installation aborted
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

All `[build.requirements]` are automatically checked before installation; if not satisfied, report
an error:

```
Error: Build requirement not satisfied
  cargo >= 1.70 required, but cargo is not installed
  Install: https://rustup.rs
```

### yx-bindgen Integration (headers field)

`[build].headers` declares the C header files to be processed by yx-bindgen. The build system
automatically runs yx-bindgen to generate `.yx` binding files.

```toml
[build]
strategy = "cargo"
headers = ["include/sqlite3.h", "include/json.h"]
```

Build flow:

```
1. Does [binaries] have precompiled? → skip entire build
2. Does [build].headers have a value? → yx-bindgen automatically generates bindings
3. Execute [build].strategy (cargo/cmake/custom)
4. Install
```

yx-bindgen parses function signatures and type definitions from C header files (`.h`) and
automatically generates `.yx` binding declarations. Users do not need to run it manually—the build
system handles it automatically when detecting the `headers` configuration.

**Relationship with RFC-026:** RFC-026 defines the language-level semantics of `yx-bindgen`
(`native("symbol")` syntax, unsafe types). RFC-014b defines its integration in the build flow
(`headers` configuration). The two are complementary.

### Integration with Cargo Workspace

If a package contains FFI code, a Cargo workspace can be defined simultaneously:

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

Using Rust target triples rather than a simplified format because:

1. Distinguishes different ABIs on the same OS (gnu vs musl, msvc vs gnu)
2. Aligns with the Rust/Cargo ecosystem, reducing mapping errors
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

### Complete Lifecycle of Precompiled Packages

```
Developer:
  1. Write .yx code + FFI bindings
  2. Declare [build] + [binaries] in yaoxiang.toml
  3. yaoxiang publish
     → Automatically build multi-platform binaries on CI
     → Upload source + precompiled artifacts

User:
  yaoxiang add native-foo
    → Detected precompiled artifacts → download directly (seconds)
    → No precompiled artifacts → download source + execute build (minutes)
```

## Trade-offs

### Advantages

- Declarative configuration; users do not need to understand build details
- Precompiled first, extremely fast installation
- Multi-platform support, automatic selection
- Seamless integration with the Cargo ecosystem

### Disadvantages

- Precompiled artifacts require CI support
- Multi-platform builds increase release complexity
- build.yx scripts require sandbox security mechanisms

## Alternatives

| Option                          | Why not chosen                                          |
| ------------------------------- | ------------------------------------------------------- |
| Pure source distribution        | Users need to install the build toolchain, high barrier |
| Binary format like Python wheel | Too complex, not needed in early YaoXiang ecosystem     |
| No FFI build support            | Limits the language's extensibility                     |

## Implementation Strategy

### Phase Division

| Phase    | Content                                                                                                             | Status                                                       |
| -------- | ------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------ |
| Phase 5a | `[build]` configuration parsing + `BuildStrategy` enum (including `[binaries]` declaration, platform triples)       | ✅ Completed                                                 |
| Phase 5b | System dependency check (`<tool> --version` detection + comparator operand completion + installation guide)         | ✅ Completed                                                 |
| Phase 5c | Cargo build integration (read `[build.cargo]` to assemble commands + platform override merging + scratch isolation) | ✅ Completed                                                 |
| Phase 5d | Precompiled binary download + verification (whole-package SHA-256 + safe unpacking + fallback semantics)            | ✅ Completed                                                 |
| Phase 5f | yx-bindgen integration (`headers` field configuration pipeline; RFC-026b is still draft, explicit error at runtime) | ✅ Configuration pipeline completed (generator follows 026b) |
| Phase 5e | build.yx script execution (**implemented last**, with trust gate)                                                   | ✅ Completed                                                 |

Execution order (2026-09-15 resolution 2): `5a → 5b → 5c → 5d → 5f → 5e`. Declarative build comes
first, arbitrary code execution comes last.

**Implementation notes (2026-09-30, see commit feat/rfc014)**:

- **Two-layer artifact separation**: cargo scratch (`target/`) is pointed to the project's
  `.yaoxiang/build/cargo/<pkg>/` via `CARGO_TARGET_DIR`—not into the vendor package directory,
  otherwise the directory integrity checksum would be inflated by incremental build artifacts;
  FFI-consumable library files (`.so/.dll/.dylib/.a`) are copied to the vendor package directory's
  `build/native/<triple>/`. The vendor integrity check semantics are thereby clarified as **source
  tree integrity** (`build/` derived artifacts do not enter the checksum).
- **`[binaries]` fallback semantics**: When the current platform has an entry but any of "sha256 not
  declared / download failed / checksum mismatch" applies → stderr hint + fall back to source build
  (RFC "otherwise fallback"), clean up half-finished artifacts before falling back. After
  whole-package verification passes, go through safe unpacking (path escape protection + unpacking
  total guardrail 512 MiB, artifact size has no hard upper limit—resolution 5).
- **Trust gate (resolution 1) implementation**: Trust records follow the user configuration system
  (`~/.config/yaoxiang/config.toml`'s `[trust] build-scripts`; the `~/.yaoxiang/config.toml` in the
  RFC draft never existed, merged in with `[cache] dir`). Three paths to pass: already recorded /
  `--trust` this time (**passing means persisted**, fulfilling "add/install --trust persists the
  trust record") / interactive confirmation (confirmation means persisted). Non-interactive
  environments (stdin not a terminal) reject by default, only `--trust` can pass.
- **build.yx execution model**: Plain .yx script, **top-level statements are the build logic**
  (Script/eval semantics; the `fn main()` example above is draft-period pseudocode), executed
  in-process, working directory temporarily switched to the package root (global lock serializes the
  flip window). The current std has no exec/exit API—scripts can write generation/file-type logic;
  `os.exec`-style calls to external tools evolve with std.
- **cmake strategy**: Enum and configuration parsing ready, execution not yet implemented (explicit
  error)—the RFC phase table does not list cmake as a separate phase, scheduling pending real-need
  packages.
- **Pre-publish test** (014a validation 3) is wired in with this Phase: by default run tests
  discovered by `[tool.test]` (RFC-036 mechanism), failure aborts publish, `--no-test` skips.

### Dependencies

- Depends on RFC-014a (Registry protocol, for downloading precompiled artifacts)
- Depends on `sha2` crate (integrity verification)

## Open Questions

- [x] Does build.yx script need sandbox isolation? → Trust gate as the mandatory lower bound
      (2026-09-15 resolution 1); full sandbox continues to be researched
- [x] Maximum size limit for build artifacts? → No upper limit, channels self-manage (2026-09-15
      resolution 5)
- [x] Is cross-compilation supported (building Windows artifacts on Linux)? → Not supported
      initially, CI multi-host production (2026-09-15 resolution 4)
- [x] How to handle Cargo version incompatibility? → `[build.requirements]` pre-check error +
      installation guide (2026-09-15 resolution 6)

---

## References

- [Rust build.rs](https://doc.rust-lang.org/cargo/reference/build-scripts.html)
- [Python wheels](https://packaging.python.org/en/latest/guides/distributing-packages-using-setuptools/#wheels)
- [Go build constraints](https://pkg.go.dev/cmd/go#hdr-Build_constraints)

---
title: 'RFC-037: Industrial Distribution Solution — Compiler/Toolchain Packaging Based on cargo-dist'
author: 'ChenXu233'
created: '2026-07-26'
updated: '2026-07-27'
issue: '#230'
---

# RFC-037: Industrial Distribution Solution — Compiler/Toolchain Packaging Based on cargo-dist

> This RFC complements [RFC-014b: Build System and Binary Distribution](../review/014b-build-system.md). RFC-014b defines how the
> **YaoXiang package manager** builds and distributes third-party packages; this RFC defines how
> **the YaoXiang compiler/toolchain itself** is packaged and distributed.

## Summary

Replace the existing hand-written CI build/packaging logic with
`cargo-dist` (the standard binary distribution tool in the Rust ecosystem), enabling cross-platform automated releases. Solves
problems such as missing `libz3.dll`, standard library interface files not packaged, messy directory structure, and repeated maintenance of CI scripts.

## Motivation

### Why is this feature needed?

Users who download YaoXiang should be able to **use it out of the box** without any additional steps.

### Current Problems

#### Problem 1: Windows users can't run it after downloading

Current releases only upload `yaoxiang.exe`, but `libz3.dll` is not packaged. Users double-clicking on Windows will get an error:

```
The code execution cannot proceed because libz3.dll was not found.
```

This is a **breaking bug** — users can't even get past the first step.

#### Problem 2: Release artifacts only contain a single exe file

```
yaoxiang-v0.7.10-x86_64-pc-windows-msvc.zip
└── yaoxiang.exe
```

Standard library interface files (`.yx` files, needed by LSP) are not included in the distribution. Users need to run `yaoxiang package init`
to generate them. The industry-standard approach is to include the standard library in the distribution.

#### Problem 3: CI hand-written scripts require repeated maintenance

Currently maintaining 4 build pipelines:

| File                      | Responsibility         | Lines         |
| ------------------------- | ---------------------- | ------------- |
| `_build-platforms.yml`    | Cross-platform build   | ~255 lines    |
| `release.yml`             | Version release        | ~176 lines    |
| `nightly.yml`             | Daily build            | ~145 lines    |
| `_build-wasm.yml`         | Wasm build             | ~75 lines     |
| `scripts/build/setup.iss` | Inno Setup installer   | ~250 lines    |
| **Total**                 |                        | **~900 lines** |

Most of it is repetitive (install Rust → cache → build → rename → upload), written once for each platform. `cargo-dist`
can generate equivalent pipelines with a single command.

#### Problem 4: Inno Setup version number hardcoded

`MyAppVersion` in `setup.iss` is hardcoded as `0.7.0`, replaced via `sed` at build time. It's only a matter of time before something breaks.

#### Problem 5: Blurry boundary with RFC-014b

RFC-014b defines "YaoXiang package build and distribution mechanisms" (i.e., `[build]` and `[binaries]`
config in `yaoxiang.toml`), but **doesn't cover "how to publish the YaoXiang compiler itself"**. This RFC fills that gap.

## Proposal

### Core Design

Adopt **cargo-dist** as the release pipeline backbone, with custom post-build scripts for package structure and additional files.

```
cargo-dist responsibilities:
  ├── Cross-platform compilation (6 targets)
  ├── Generate CI pipelines (replace ~900 lines of hand-written YAML)
  ├── Generate installers (MSI / shell / powershell / homebrew)
  ├── npm publishing (@yaoxiang/cli — binary download wrapper)
  ├── checksum + signing
  └── Upload to GitHub Release

build.rs continues to handle:
  └── Z3 download/linking (existing logic, switch to cross-platform dynamic linking)

YaoXiang custom scripts (package-dist.sh) handle:
  ├── Post-build reorganization of zip structure (bin/ + lib/)
  ├── Include shared libraries (libz3.so / dylib / dll)
  └── Pre-generate standard library .yx interface files
```

### Release Directory Structure

Each platform's release archive, reorganized by `package-dist.sh` after cargo-dist builds:

```
yaoxiang-{version}-{target}.tar.gz / .zip
├── bin/
│   ├── yaoxiang                      # or yaoxiang.exe
│   └── libz3.so / libz3.dylib / libz3.dll
├── lib/
│   └── yaoxiang/
│       └── std/                      # Pre-generated standard library interface files
│           ├── io.yx
│           ├── math.yx
│           ├── string.yx
│           ├── ...
│           └── mod.yx
├── README.md
└── LICENSE
```

cargo-dist's default zip is flat (binary + auto-included README/LICENSE all at root level). This isn't a problem — clear division of labor: cargo-dist handles compilation+CI+installers, YaoXiang uses a 50-line
`package-dist.sh` to handle zip structure.

### Platform Support

| Platform          | target triple               | Description         |
| -------------- | --------------------------- | ----------------- |
| Linux x86_64   | `x86_64-unknown-linux-gnu`  | Primary platform  |
| Linux ARM64    | `aarch64-unknown-linux-gnu` | Cross-compiled on CI |
| macOS x86_64   | `x86_64-apple-darwin`       | Intel Mac         |
| macOS ARM64    | `aarch64-apple-darwin`      | Apple Silicon     |
| Windows x86_64 | `x86_64-pc-windows-msvc`    | Primary platform  |

Windows ARM64 not yet supported (Z3 has no official pre-built ARM64 packages).

### Z3 Distribution Strategy

**Unified to cross-platform dynamic linking.**

| Platform    | Change                | Artifact         |
| ------- | ------------------- | ------------- |
| Linux   | **Changed from static to dynamic** | `libz3.so`    |
| macOS   | **Changed from static to dynamic** | `libz3.dylib` |
| Windows | Unchanged                | `libz3.dll`   |
| wasm32  | Unchanged (static linking)    | embedded `.a`     |

Reasons:

- **Consistency** — Unified behavior across all three platforms, no special cases
- **This is an external library, it should be distributed as a shared library**. Python (`python3.dll`+`DLLs/lib*.dll`), Node (`node`+`lib/`) all do this
- **Users can upgrade Z3 without waiting for compiler version** — just swap out the `.so`/`.dylib`/`.dll`
- **Smaller binary size** — Z3 is not small, static linking bloats the exe by several MB

Corresponding `build.rs` modification:

```rust
// Unified dynamic linking
fn link_z3(z3_dir: &Path) {
    println!("cargo:rustc-link-lib=z3");     // No longer distinguishes Windows/non-Windows
    // Keep C++ standard library linking unchanged
    let cxx = if target_os == "macos" { "c++" } else { "stdc++" };
    println!("cargo:rustc-link-lib={}", cxx);
}
```

**"Cross-platform static linking" is no longer the goal.**
This is not eliminating special cases, it's eliminating a reasonable situation in the wrong way. Shared libraries are the normal distribution method for external libraries.

### Installer Support

| Installer           | Status              | Description                           |
| ---------------- | ----------------- | ---------------------------------- |
| zip / tar.gz     | ✅ Default           | All platforms, manual download             |
| shell script       | ✅ cargo-dist     | Unix: `curl ...` pipe `sh` |
| powershell script  | ✅ cargo-dist     | Windows: `irm ...` pipe `iex` |
| Homebrew formula | ✅ cargo-dist     | macOS: `brew install yaoxiang` |
| Windows MSI      | ✅ cargo-dist     | Based on WiX, primary Windows installer    |
| **Inno Setup**   | **✅ Kept as secondary** | Domestic user alternative, won't be deleted           |

**Rationale for keeping Inno Setup:**

- Domestic Windows users are more accustomed to exe installation wizards (Next → Next → Finish)
- MSI may be blocked in certain enterprise/school network environments
- Maintaining one extra `setup.iss` costs far less than losing a portion of users

### Standard Library Interface File Generation

Subcommand name: **`yaoxiang package gen-std`** (in the same system as existing `package init`/`add`/`install`)

Current `src/std/gen_interfaces.rs`
already has complete implementation (`generate_all_interfaces()`, `write_interfaces_to_dir()`), just need to add a subcommand entry in `main.rs`,
then call it in `package-dist.sh`:

```bash
yaoxiang package gen-std --out-dir "$PKG_ROOT/lib/yaoxiang/std/"
```

### Wasm Build

**Remains independent, not migrated into cargo-dist.**

cargo-dist manages "sending the compiler to users", wasm is "online playground embedded in documentation website" — two completely different deliverables.

| Aspect        | Approach                                   |
| ----------- | -------------------------------------- |
| Build tool    | Keep `wasm-pack build`                 |
| CI workflow | Keep `_build-wasm.yml` as independent job        |
| Trigger timing    | Same push as release, parallel independent job |
| Publish target    | `docs/public/wasm/` → GitHub Pages     |

### npm Publishing

Two different npm packages, each independent:

| Package                     | Content                       | Tool                      | Status                |
| ---------------------- | -------------------------- | ------------------------- | ------------------- |
| `@yaoxiang/cli`        | Download CLI binary (wrapper) | cargo-dist native generation       | Ready with cargo-dist config |
| `@yaoxiang/playground` | wasm library (JS + .wasm)      | wasm-pack + `npm publish` | Optional, currently only publishes to docs |

They don't conflict with each other, and the names don't conflict either.

### Nightly Publishing

cargo-dist has no native nightly support ([#1143](https://github.com/axodotdev/cargo-dist/issues/1143), still an open
feature request).

**Keep the existing cron + tag approach**, replacing the build portion with cargo-dist:

```yaml
# nightly.yml (after migration, ~50 lines)
on: schedule: "17 22 * * *"
jobs:
  build:
    # Reuse cargo-dist build capability, but don't use its release flow
    uses: ./.github/workflows/release.yml  # cargo-dist generated build job
  publish:
    # Continue using existing: tag nightly → overwrite GitHub Pre-release
```

### cargo-dist Configuration (Draft)

After running `cargo dist init`, the initial configuration generated, core parts expected:

```toml
[workspace]
members = ["cargo:."]

[dist]
targets = [
  "x86_64-unknown-linux-gnu",
  "aarch64-unknown-linux-gnu",
  "x86_64-apple-darwin",
  "aarch64-apple-darwin",
  "x86_64-pc-windows-msvc",
]
installers = [
  "shell",
  "powershell",
  "homebrew",
  "msi",
]
```

Specific configuration items to be determined by what `cargo dist init` actually generates.

### package-dist.sh (Draft)

```bash
#!/bin/bash
# Executes after cargo-dist builds, reorganizes release package structure
# Called by cargo-dist's extra-artifacts or independent CI step
set -euo pipefail

VERSION="$1"
TARGET="$2"
DIST_DIR="target/distrib"
PKG_ROOT="$DIST_DIR/yaoxiang-$VERSION-$TARGET"

mkdir -p "$PKG_ROOT/bin" "$PKG_ROOT/lib/yaoxiang/std"

# binary
mv "$DIST_DIR/yaoxiang" "$PKG_ROOT/bin/"

# shared library
Z3_DIR=".z3/z3-4.16.0-..."
case "$TARGET" in
  *windows*)   cp "$Z3_DIR/bin/libz3.dll"   "$PKG_ROOT/bin/" ;;
  *linux*)     cp "$Z3_DIR/lib/libz3.so"    "$PKG_ROOT/bin/" ;;
  *apple*)     cp "$Z3_DIR/lib/libz3.dylib" "$PKG_ROOT/bin/" ;;
esac

# standard library interface files
yaoxiang package gen-std --out-dir "$PKG_ROOT/lib/yaoxiang/std/"

# README + LICENSE
cp README.md LICENSE "$PKG_ROOT/"

# repackage
cd "$DIST_DIR"
tar czf "yaoxiang-$VERSION-$TARGET.tar.gz" "yaoxiang-$VERSION-$TARGET"
```

### Standard Library Interface File Generation

Current `src/std/gen_interfaces.rs` already implements generating `.yx`
interface files (`write_interfaces_to_dir`), and the `package init` command also calls it.

Just need to add a subcommand entry in `main.rs`, then call it in the packaging script.

### Deprecated Hand-written CI

Delete the following files after migration:

| File                                     | Lines         | Replaced by                           |
| ---------------------------------------- | ----------- | ------------------------------ |
| `.github/workflows/_build-platforms.yml` | 255         | auto-generated by cargo-dist            |
| `.github/workflows/release.yml`          | 176         | auto-generated by cargo-dist            |
| `.github/workflows/nightly.yml`          | 145         | cargo-dist build + keep publish logic |
| `scripts/build/setup.iss`                | ~250        | **Keep** (for domestic use)             |
| **Total reduction**                             | **~600 lines** |                                |

Keep:

- `ci.yml` (daily fmt + clippy + test + MSRV, not part of release process)
- `nightly.yml` (publish logic portion kept)
- `_build-wasm.yml` (independent build flow)
- `_build-z3-wasm.yml` (wasm-specific Z3)
- `setup.iss` (domestic auxiliary installer)
- `docs-deploy.yml` (documentation deployment)

## Tradeoffs

### Advantages

- **Out of the box** — Users download and extract, then run directly, no missing DLL issues
- **Reduced maintenance cost** — Delete ~600 lines of hand-written CI YAML, cargo-dist auto-maintains
- **Standardized** — Industry-standard tool, validated by hundreds of projects
- **Cross-platform consistency** — Cross-platform dynamic linking, unified behavior
- **Installer coverage** — shell/powershell/homebrew/msi/inno setup all supported

### Disadvantages

- **Learning cargo-dist configuration** — Team needs to learn a new tool
- **Custom packaging scripts still have maintenance cost** — Package structure and standard library interface file scripts need maintenance
- **cargo-dist version iteration** — Need to monitor upstream changes
- **cargo-dist has no native nightly** — Nightly publishing portion still requires hand-written code

### Relationship with RFC-014b

|          | RFC-014b                              | RFC-037                  |
| -------- | ------------------------------------- | ------------------------ |
| **Scope** | Third-party package build and distribution                  | Compiler self-packaging and distribution   |
| **Tool** | `yaoxiang build` / `yaoxiang publish` | `cargo-dist`             |
| **Output** | Third-party package FFI libraries                     | Compiler + standard library + toolchain |
| **Mutually exclusive** | No, complementary                              | No, complementary                 |

## Alternative Approaches

| Approach                       | Why not chosen                                     |
| -------------------------- | ---------------------------------------------- |
| **Continue hand-writing CI**            | Already hand-wrote ~900 lines, repetitive work, easy to miss DLL     |
| **Write own packaging tool**         | Don't reinvent the wheel, cargo-dist is already mature          |
| **Only use tar.gz, no installers** | Users need more user-friendly installation methods (Homebrew/MSI)       |
| **Docker distribution**            | Compilers and language toolchains need native binaries, not container scenarios |
| **Fully static link Z3**          | External libraries should normally be distributed as shared libraries, don't pursue static         |
| **Deprecate Inno Setup**        | Domestic users have different habits, maintenance cost is extremely low                 |

## Implementation Strategy

### Phase One: build.rs modification + gen-std subcommand (P0)

1. Modify `build.rs`: unified cross-platform dynamic linking, expand `copy_dll()` to `copy_shared_lib()`
2. Add `yaoxiang package gen-std` subcommand in `main.rs` (reuse `gen_interfaces.rs`)

### Phase Two: cargo-dist integration (P0)

1. Run `cargo dist init` to generate initial configuration
2. Write `package-dist.sh` packaging script
3. Integrate in `release.yml`: cargo-dist build → `package-dist.sh` reorganization → upload
4. Verify the generated archive structure and content are correct

### Phase Three: Old CI decommission (P1)

1. Run new and old CI in parallel, compare artifacts
2. After confirmation, delete `_build-platforms.yml`
3. Streamline `nightly.yml` (replace build portion with cargo-dist)
4. Confirm `setup.iss` still works

### Phase Four: Installer enablement (P2)

1. Configure Homebrew tap auto-publishing
2. Configure MSI installer generation
3. Configure npm publishing (`@yaoxiang/cli`)

## Open Issues (Closed)

The following issues have been resolved in design discussion:

- ~~Feasibility of static linking Z3 on Windows?~~ → **Don't do static linking, cross-platform dynamic**
- ~~gen-std-interfaces subcommand naming?~~ → **`yaoxiang package gen-std`**
- ~~Keep Inno Setup?~~ → **Keep**
- ~~cargo-dist extra-artifacts conditional execution?~~ → **Handle with `package-dist.sh` script, use shell case branches**
- ~~Standard library interface version compatibility?~~ → **Released with compiler version, same archive**

## References

- [cargo-dist Official Documentation](https://axodotdev.github.io/cargo-dist/)
- [cargo-dist GitHub](https://github.com/axodotdev/cargo-dist)
- [RFC-014b: Build System and Binary Distribution](../review/014b-build-system.md)
- [cargo-dist nightly feature request](https://github.com/axodotdev/cargo-dist/issues/1143)
- [Z3 Build Configuration — CMakeLists.txt](https://github.com/Z3Prover/z3/blob/master/src/CMakeLists.txt)

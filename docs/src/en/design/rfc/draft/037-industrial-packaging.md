---
title: 'RFC-037: Industrial Distribution Plan — Compiler/Toolchain Packaging Based on cargo-dist'
author: 'ChenXu233'
created: '2026-07-26'
updated: '2026-07-27'
issue: '#230'
---

# RFC-037: Industrial Distribution Plan — Compiler/Toolchain Packaging Based on cargo-dist

> This RFC complements
> [RFC-014b: Build System and Binary Distribution](../review/014b-build-system.md). RFC-014b defines
> how the **YaoXiang package manager** builds and distributes third-party packages; this RFC defines
> how the **YaoXiang compiler/toolchain itself** is packaged and distributed.

## Summary

Replace the current hand-written CI build/packaging logic with `cargo-dist` (the standard binary
distribution tool in the Rust ecosystem) to achieve cross-platform automated releases. This
addresses issues such as missing `libz3.dll`, unpackaged standard library interface files, messy
directory structure, and duplicated CI script maintenance.

## Motivation

### Why is this feature needed?

Users who download YaoXiang should be able to **use it out of the box**, without any additional
steps.

### Current Problems

#### Problem 1: Windows users can't run it after download

The current Release only uploads `yaoxiang.exe`, but `libz3.dll` is not packaged in. Double-clicking
to run on Windows will produce an error:

```
The code execution cannot proceed because libz3.dll was not found.
```

This is a **blocking bug** — users can't even get past the first step.

#### Problem 2: Release artifacts contain only a single exe file

```
yaoxiang-v0.7.10-x86_64-pc-windows-msvc.zip
└── yaoxiang.exe
```

Standard library interface files (`.yx` files, needed by LSP) are not included in the release
package. Users need to run `yaoxiang package init` to generate them. The industrial approach is for
the release package to include the standard library.

#### Problem 3: Duplicated maintenance of hand-written CI scripts

Currently maintaining 4 sets of build pipelines:

| File                      | Responsibility        | Lines          |
| ------------------------- | --------------------- | -------------- |
| `_build-platforms.yml`    | Cross-platform builds | ~255 lines     |
| `release.yml`             | Version release       | ~176 lines     |
| `nightly.yml`             | Daily builds          | ~145 lines     |
| `_build-wasm.yml`         | Wasm builds           | ~75 lines      |
| `scripts/build/setup.iss` | Inno Setup installer  | ~250 lines     |
| **Total**                 |                       | **~900 lines** |

Most of it is repetitive (install Rust → cache → build → rename → upload), and has to be written
once for each platform. `cargo-dist` can generate equivalent pipelines with a single command.

#### Problem 4: Inno Setup version number is hardcoded

`MyAppVersion` is hardcoded to `0.7.0` in `setup.iss`, and the build relies on `sed` for
substitution. This will eventually fail.

#### Problem 5: Ambiguous boundary with RFC-014b

RFC-014b defines the "YaoXiang package build and distribution mechanism" (i.e., the `[build]` and
`[binaries]` configuration in `yaoxiang.toml`), but **does not cover "how the YaoXiang compiler
itself is published"**. This RFC fills that gap.

## Proposal

### Core Design

Adopt **cargo-dist** as the release pipeline skeleton, combined with custom post-build scripts to
handle package structure and additional files.

```
cargo-dist responsibilities:
  ├── Cross-platform compilation (6 targets)
  ├── Generate CI pipeline (replaces hand-written ~900 lines of YAML)
  ├── Generate installers (MSI / shell / powershell / homebrew)
  ├── npm publish (@yaoxiang/cli — binary download wrapper)
  ├── Checksum + signing
  └── Upload to GitHub Release

build.rs continues to handle:
  └── Z3 download/link (existing logic, switched to dynamic linking on all platforms)

YaoXiang custom script (package-dist.sh) handles:
  ├── Reorganize zip structure after build (bin/ + lib/)
  ├── Include shared libraries (libz3.so / dylib / dll)
  └── Pre-generate standard library .yx interface files
```

### Release Directory Structure

Each platform release archive, reorganized by `package-dist.sh` after cargo-dist build:

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

The default cargo-dist zip is flat (binary + auto-included README/LICENSE are all in the root
directory). This is not a problem — clear division of labor: cargo-dist handles compilation + CI +
installers, and YaoXiang uses a 50-line `package-dist.sh` to handle zip structure.

### Platform Support

| Platform       | target triple               | Notes               |
| -------------- | --------------------------- | ------------------- |
| Linux x86_64   | `x86_64-unknown-linux-gnu`  | Primary platform    |
| Linux ARM64    | `aarch64-unknown-linux-gnu` | Cross-compile on CI |
| macOS x86_64   | `x86_64-apple-darwin`       | Intel Mac           |
| macOS ARM64    | `aarch64-apple-darwin`      | Apple Silicon       |
| Windows x86_64 | `x86_64-pc-windows-msvc`    | Primary platform    |

Windows ARM64 is not currently supported (Z3 has no official prebuilt ARM64 package).

### Z3 Distribution Strategy

**Unified to dynamic linking on all platforms.**

| Platform | Change                                 | Output        |
| -------- | -------------------------------------- | ------------- |
| Linux    | **Original static→changed to dynamic** | `libz3.so`    |
| macOS    | **Original static→changed to dynamic** | `libz3.dylib` |
| Windows  | Unchanged                              | `libz3.dll`   |
| wasm32   | Unchanged (static link)                | Embedded `.a` |

Reasons:

- **Consistency** — Unified behavior across three platforms, no more platform-specific exceptions
- **This is an external library and should be distributed as a shared library**. Python
  (`python3.dll`+`DLLs/lib*.dll`), Node (`node`+`lib/`) all do this
- **Users don't need to wait for a new compiler version to upgrade Z3** — just replace the
  `.so`/`.dylib`/`.dll`
- **Smaller binary size** — Z3 is not small, and static linking will bloat the exe by several MB

Corresponding `build.rs` modifications:

```rust
// Unified dynamic linking
fn link_z3(z3_dir: &Path) {
    println!("cargo:rustc-link-lib=z3");     // No longer distinguish Windows/non-Windows
    // Keep C++ standard library linking unchanged
    let cxx = if target_os == "macos" { "c++" } else { "stdc++" };
    println!("cargo:rustc-link-lib={}", cxx);
}
```

**"Static linking Z3 on all platforms" is no longer a goal.** This is not eliminating special cases;
it's eliminating a reasonable case in the wrong way. Shared libraries are the normal distribution
method for external libraries.

### Installer Support

| Installer         | Status                   | Notes                                  |
| ----------------- | ------------------------ | -------------------------------------- |
| zip / tar.gz      | ✅ Default               | All platforms, manual download         |
| shell script      | ✅ cargo-dist            | Unix: `curl ... \| sh`                 |
| powershell script | ✅ cargo-dist            | Windows: `irm ... \| iex`              |
| Homebrew formula  | ✅ cargo-dist            | macOS: `brew install yaoxiang`         |
| Windows MSI       | ✅ cargo-dist            | Based on WiX, main Windows installer   |
| **Inno Setup**    | **✅ Kept as auxiliary** | Backup for domestic users, not deleted |

**Reasons for keeping Inno Setup:**

- Domestic Windows users are more accustomed to exe installation wizards (Next → Next → Finish)
- MSI is blocked in some enterprise/school network environments
- The cost of maintaining an extra `setup.iss` is far less than losing a portion of users

### Standard Library Interface File Generation

Subcommand name: **`yaoxiang package gen-std`** (in the same family as the existing
`package init`/`add`/`install`)

The current `src/std/gen_interfaces.rs` already has a complete implementation
(`generate_all_interfaces()`, `write_interfaces_to_dir()`); just add the subcommand entry in
`main.rs` and call it in `package-dist.sh`:

```bash
yaoxiang package gen-std --out-dir "$PKG_ROOT/lib/yaoxiang/std/"
```

### Wasm Build

**Remain independent, not migrated into cargo-dist.**

cargo-dist handles "delivering the compiler to users"; wasm handles "embedding the online playground
in the documentation website" — two completely different deliverables.

| Aspect         | Approach                                       |
| -------------- | ---------------------------------------------- |
| Build tool     | Keep `wasm-pack build`                         |
| CI workflow    | Keep `_build-wasm.yml` as an independent job   |
| Trigger timing | Same push as release, parallel independent job |
| Publish target | `docs/public/wasm/` → GitHub Pages             |

### npm Publish

Two different npm packages, each independent:

| Package                | Contents                      | Tool                         | Status                                  |
| ---------------------- | ----------------------------- | ---------------------------- | --------------------------------------- |
| `@yaoxiang/cli`        | Download CLI binary (wrapper) | cargo-dist native generation | cargo-dist config out-of-the-box        |
| `@yaoxiang/playground` | wasm library (JS + .wasm)     | wasm-pack + `npm publish`    | Optional, currently only publishes docs |

The two do not conflict, nor do their names.

### Nightly Publish

cargo-dist has no native nightly support
([#1143](https://github.com/axodotdev/cargo-dist/issues/1143), still an open feature request).

**Keep the current cron + tag approach**, and replace the build portion with cargo-dist:

```yaml
# nightly.yml (after migration, ~50 lines)
on: schedule: "17 22 * * *"
jobs:
  build:
    # Reuse cargo-dist build capability, but not its release flow
    uses: ./.github/workflows/release.yml  # cargo-dist-generated build job
  publish:
    # Continue with the existing approach: tag nightly → overwrite GitHub Pre-release
```

### cargo-dist Configuration (Draft)

After running `cargo dist init`, the initial configuration is generated, with the core expected to
be:

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

Specific configuration items are subject to the actual output of `cargo dist init`.

### package-dist.sh (Draft)

```bash
#!/bin/bash
# Executed after cargo-dist build, reorganizes release package structure
# Called by cargo-dist's extra-artifacts or an independent CI step
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

# re-package
cd "$DIST_DIR"
tar czf "yaoxiang-$VERSION-$TARGET.tar.gz" "yaoxiang-$VERSION-$TARGET"
```

### Standard Library Interface File Generation

The current `src/std/gen_interfaces.rs` already implements the functionality for generating `.yx`
interface files (`write_interfaces_to_dir`), and the `package init` command also calls it.

Just add the subcommand entry in `main.rs` and call it in the packaging script.

### Deprecated Hand-Written CI

Delete the following files after migration is complete:

| File                                     | Lines          | Replacement                           |
| ---------------------------------------- | -------------- | ------------------------------------- |
| `.github/workflows/_build-platforms.yml` | 255            | Auto-generated by cargo-dist          |
| `.github/workflows/release.yml`          | 176            | Auto-generated by cargo-dist          |
| `.github/workflows/nightly.yml`          | 145            | cargo-dist build + keep publish logic |
| `scripts/build/setup.iss`                | ~250           | **Keep** (for domestic users)         |
| **Total reduction**                      | **~600 lines** |                                       |

Kept:

- `ci.yml` (daily fmt + clippy + test + MSRV, not part of the release process)
- `nightly.yml` (keep the publish logic portion)
- `_build-wasm.yml` (independent build flow)
- `_build-z3-wasm.yml` (wasm-specific Z3)
- `setup.iss` (domestic auxiliary installer)
- `docs-deploy.yml` (documentation deployment)

## Trade-offs

### Advantages

- **Out of the box** — Users can run it directly after downloading and extracting, no missing DLL
  issues
- **Reduced maintenance cost** — Delete ~600 lines of hand-written CI YAML, cargo-dist
  auto-maintains
- **Standardization** — Industry-standard tool, validated by hundreds of projects
- **Cross-platform consistency** — Dynamic linking on all platforms, unified behavior
- **Installer coverage** — Full support for shell/powershell/homebrew/msi/inno setup

### Disadvantages

- **Learning the cargo-dist configuration** — The team needs to learn a new tool
- **Custom packaging script still has maintenance cost** — The script for package structure and
  standard library interface files needs maintenance
- **cargo-dist version iteration** — Need to follow upstream changes
- **cargo-dist has no native nightly** — The nightly release portion still needs to be hand-written

### Relationship with RFC-014b

|                        | RFC-014b                                   | RFC-037                                    |
| ---------------------- | ------------------------------------------ | ------------------------------------------ |
| **Scope**              | Third-party package build and distribution | Compiler packaging and distribution itself |
| **Tool**               | `yaoxiang build` / `yaoxiang publish`      | `cargo-dist`                               |
| **Output**             | Third-party package FFI libraries          | Compiler + standard library + toolchain    |
| **Mutually exclusive** | No, complementary                          | No, complementary                          |

## Alternatives

| Approach                               | Why not chosen                                                                                   |
| -------------------------------------- | ------------------------------------------------------------------------------------------------ |
| **Continue hand-writing CI**           | Already hand-wrote ~900 lines, repetitive work, easy to miss DLLs                                |
| **Write our own packaging tool**       | Don't reinvent the wheel, cargo-dist is already mature                                           |
| **Use only tar.gz without installers** | Users need more friendly installation methods (Homebrew/MSI)                                     |
| **Docker distribution**                | Compilers and language toolchains require native binaries, not a container scenario              |
| **Fully static linking Z3**            | External libraries should normally be distributed as shared libraries, not pursue static linking |
| **Deprecate Inno Setup**               | Domestic users have different habits, the cost of keeping it is very low                         |

## Implementation Strategy

### Phase 1: build.rs modifications + gen-std subcommand (P0)

1. Modify `build.rs`: unified dynamic linking on all platforms, extend `copy_dll()` to
   `copy_shared_lib()`
2. Add `yaoxiang package gen-std` subcommand in `main.rs` (reuse `gen_interfaces.rs`)

### Phase 2: cargo-dist integration (P0)

1. Run `cargo dist init` to generate initial configuration
2. Write `package-dist.sh` packaging script
3. Integrate in `release.yml`: cargo-dist build → `package-dist.sh` reorganize → upload
4. Verify the generated archive structure and contents are correct

### Phase 3: Decommission old CI (P1)

1. Run new and old CI in parallel, compare outputs
2. Delete `_build-platforms.yml` after confirmation
3. Simplify `nightly.yml` (replace the build portion with cargo-dist)
4. Confirm `setup.iss` still works

### Phase 4: Enable installers (P2)

1. Configure Homebrew tap auto-publish
2. Configure MSI installer generation
3. Configure npm publish (`@yaoxiang/cli`)

## Open Questions (Closed)

The following questions have been resolved during design discussion:

- ~~Feasibility of static linking Z3 on Windows?~~ → **No static linking, dynamic on all platforms**
- ~~gen-std-interfaces subcommand naming?~~ → **`yaoxiang package gen-std`**
- ~~Should Inno Setup be kept?~~ → **Keep it**
- ~~Conditional execution of cargo-dist extra-artifacts?~~ → **Handle with `package-dist.sh` script,
  use shell case branches**
- ~~Standard library interface version compatibility?~~ → **Published together with compiler
  version, in the same archive**

## References

- [cargo-dist official documentation](https://axodotdev.github.io/cargo-dist/)
- [cargo-dist GitHub](https://github.com/axodotdev/cargo-dist)
- [RFC-014b: Build System and Binary Distribution](../review/014b-build-system.md)
- [cargo-dist nightly feature request](https://github.com/axodotdev/cargo-dist/issues/1143)
- [Z3 build configuration — CMakeLists.txt](https://github.com/Z3Prover/z3/blob/master/src/CMakeLists.txt)

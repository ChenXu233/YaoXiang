---
title: "RFC-037: Industrial Distribution Plan — Compiler/Toolchain Packaging Based on cargo-dist"
author: "ChenXu233"
created: "2026-07-26"
updated: "2026-07-26"
issue: "#230"
---

# RFC-037: Industrial Distribution Plan — Compiler/Toolchain Packaging Based on cargo-dist

> This RFC is complementary to [RFC-014b: Build System and Binary Distribution](../review/014b-build-system.md).
> RFC-014b defines how the **YaoXiang package manager** builds and distributes third-party packages;
> this RFC defines how the **YaoXiang compiler/toolchain itself** is packaged and distributed.

## Summary

Replace the existing hand-written CI build/packaging logic with `cargo-dist` (the standard binary distribution tool in the Rust ecosystem) to achieve automated cross-platform releases. This addresses issues such as missing `libz3.dll`, unpackaged standard library interface files, messy directory structure, and repetitive maintenance of CI scripts.

## Motivation

### Why is this feature needed?

Users who download YaoXiang should be able to use it **out of the box** without any additional steps.

### Current Problems

#### Problem 1: Windows users can't run after downloading

The current Release only uploads `yaoxiang.exe`, but `libz3.dll` is not packaged inside. When users double-click to run on Windows, they get an error:

```
The code execution cannot proceed because libz3.dll was not found.
```

This is a **blocking bug** — users can't even get past the first step.

#### Problem 2: Release artifact is only a single-file exe

```
yaoxiang-v0.7.10-x86_64-pc-windows-msvc.zip
└── yaoxiang.exe
```

The standard library interface files (`.yx` files, required by LSP) are not included in the distribution package, and users have to run `yaoxiang package init` to generate them. The industrial approach should be:

```
yaoxiang-0.7.10-x86_64-pc-windows-msvc.zip
├── bin/
│   ├── yaoxiang.exe
│   └── libz3.dll
└── lib/
    └── std/
        ├── io.yx
        ├── math.yx
        ├── ...
        └── mod.yx
```

#### Problem 3: Repetitive maintenance of hand-written CI scripts

Currently 3 build pipelines are maintained:

| File | Responsibility | Lines |
|------|------|------|
| `_build-platforms.yml` | Cross-platform builds (Linux/macOS/Windows) | ~250 lines |
| `release.yml` | Version release process | ~170 lines |
| `nightly.yml` | Daily builds | ~170 lines |

**Total ~600 lines of hand-written YAML.** Most of these scripts are repetitive (install Rust → cache → build → rename → upload), and need to be written once per platform. `cargo-dist` can generate equivalent pipelines with a single command.

#### Problem 4: Inno Setup has hardcoded version number

`MyAppVersion` in `setup.iss` is hardcoded to `0.7.0`, and relies on `sed` substitution at build time. It will eventually fail.

#### Problem 5: Ambiguous boundary with RFC-014b

RFC-014b defines "YaoXiang package build and distribution mechanism" (i.e., `[build]` and `[binaries]` configuration in `yaoxiang.toml`), but **does not cover "how the YaoXiang compiler itself is released"**. This RFC fills that gap.

## Proposal

### Core Design

Adopt **cargo-dist** as the release pipeline, combined with custom post-build scripts to handle Z3 DLL and standard library interface files.

```
cargo-dist handles:
  ├── Cross-platform builds (6 platforms)
  ├── Generate tar.gz/zip archives
  ├── Generate install scripts (shell/powershell)
  ├── Generate Windows MSI installer
  ├── Automatic GitHub Release publishing
  └── Automatic changelog generation

build.rs continues to handle:
  └── Z3 download/link (existing logic, only minor adjustments needed)

Custom scripts handle:
  ├── Copy libz3.dll to packaging directory after build
  └── Pre-generate standard library .yx interface files
```

### Release Directory Structure

Each platform release archive:

```
yaoxiang-{version}-{target}.tar.gz / .zip
├── bin/
│   ├── yaoxiang                      # or yaoxiang.exe
│   └── libz3.dll                     # Windows only; other platforms statically link
├── lib/
│   └── std/                          # Pre-generated standard library interface files
│       ├── io.yx
│       ├── math.yx
│       ├── string.yx
│       ├── ...
│       └── mod.yx
└── README.md                         # Brief installation instructions
```

### Platform Support

| Platform | target triple | Notes |
|------|-------------|------|
| Linux x86_64 | `x86_64-unknown-linux-gnu` | Main platform |
| Linux ARM64 | `aarch64-unknown-linux-gnu` | Cross-compiled in CI |
| macOS x86_64 | `x86_64-apple-darwin` | Intel Mac |
| macOS ARM64 | `aarch64-apple-darwin` | Apple Silicon |
| Windows x86_64 | `x86_64-pc-windows-msvc` | Main platform |
| Windows ARM64 | `aarch64-pc-windows-msvc` | Optional, future support |

### Z3 Distribution Strategy

| Platform | Strategy | Reason |
|------|------|------|
| Linux | Static link `libz3.a` | Already in place, keep it |
| macOS | Static link `libz3.a` | Already in place, keep it |
| Windows | Package `libz3.dll` | Z3 official Windows prebuilt is DLL only |
| wasm32 | Static link `libz3.a` | Already in place, keep it |

**The Windows `libz3.dll` is downloaded by build.rs at build time to the `.z3/` directory, then packaged into the archive via cargo-dist's `extra-artifacts` mechanism.**

Long-term goal: Self-build Z3's Windows static library (`-DZ3_BUILD_LIBZ3_SHARED=OFF`), achieving single-file distribution with full-platform static linking.

### Installer Support

| Installer | Supported | Notes |
|--------|------|------|
| tar.gz / zip | ✅ Default | All platforms |
| Shell install script | ✅ cargo-dist built-in | Unix platforms |
| PowerShell install script | ✅ cargo-dist built-in | Windows platforms |
| Homebrew formula | ✅ cargo-dist built-in | macOS |
| Windows MSI | ✅ cargo-dist built-in | Replaces Inno Setup |
| Inno Setup | ❌ Deprecated | Migrate to cargo-dist MSI |

## Detailed Design

### cargo-dist Configuration

Create `dist-workspace.toml` in the project root:

```toml
[workspace]
# Points to Cargo workspace; all binary packages are auto-discovered
members = ["cargo:."]

[dist]
# Build artifacts
package-libraries = ["cdylib"]

# Installers
installers = [
    "shell",           # Unix shell install script
    "powershell",      # Windows powershell install script
    "homebrew",        # macOS Homebrew
    "msi",             # Windows MSI installer
]

# Post-build extra-processing scripts
extra-artifacts = [
    "scripts/build/package-z3.sh",   # Copy libz3.dll + standard library interface files
]

# CI configuration
ci = "github"
ci.github.create-release = true
ci.github.pr-run-mode = "plan"
```

### Post-build Processing Script

`scripts/build/package-z3.sh` (cross-platform, executed after cargo-dist build):

```bash
#!/bin/bash
# After cargo-dist build, copy libz3.dll and standard library interface files to the packaging directory

set -euo pipefail

# 1. Copy Z3 DLL (Windows only)
if [ "$CARGO_DIST_TARGET" = "x86_64-pc-windows-msvc" ]; then
    Z3_DIR=".z3/z3-4.16.0-x64-win"
    cp "$Z3_DIR/bin/libz3.dll" "$DIST_DIR/bin/"
fi

# 2. Generate standard library interface files
yaoxiang package gen-std-interfaces --out-dir "$DIST_DIR/lib/std/"
```

### CI Pipeline Changes

#### Before Migration (Current):

```
release.yml (170 lines) ──→ _build-platforms.yml (250 lines) ──→ Build → Upload
                  └──→ wasm build
                  └──→ Security audit
                  └──→ Test
                  └──→ Release

nightly.yml (170 lines) ──→ Same as above (duplicated)
```

#### After Migration:

```
release.yml generated by cargo-dist (~100 lines, auto-maintained)
  └──→ 6 platforms parallel build
  └──→ Generate archives + install scripts
  └──→ Create GitHub Release
  └──→ Upload to Homebrew / npm

pr.yml generated by cargo-dist (~50 lines, auto-maintained)
  └──→ Run dist plan check on PR
```

### Standard Library Interface File Generation

The current `src/std/gen_interfaces.rs` already implements the function to generate `.yx` interface files (`write_interfaces_to_dir`), and the `package init` command also calls it.

All that's needed is:

1. Add a new subcommand `yaoxiang package gen-std-interfaces` (or a standalone script) in `main.rs`
2. Call this command in the packaging script to generate to `lib/std/`

### Deprecated Hand-written CI

After migration, delete the following files:

| File | Replacement |
|------|------|
| `.github/workflows/_build-platforms.yml` | Auto-generated by cargo-dist |
| `.github/workflows/release.yml` | Auto-generated by cargo-dist |
| `.github/workflows/nightly.yml` | Schedule trigger from cargo-dist |
| `scripts/build/setup.iss` | cargo-dist MSI installer |
| `scripts/build/ChineseSimplified.isl` | Same as above |

## Trade-offs

### Pros

- **Out of the box** — Users can run directly after downloading and extracting the archive, with no missing DLL issues
- **Reduced maintenance cost** — Delete ~600 lines of hand-written CI YAML; cargo-dist maintains it automatically
- **Standardization** — Industry-standard tool, validated by hundreds of projects
- **Cross-platform consistency** — Same pipeline across 6 platforms
- **Automatic changelog** — Built-in changelog generation and release notes
- **Installer coverage** — Full support for shell/powershell/homebrew/msi

### Cons

- **Learning cargo-dist configuration** — Team needs to learn the new tool
- **Custom processing still has maintenance cost** — Scripts for Z3 DLL and standard library interface files need maintenance
- **cargo-dist version iterations** — Need to follow upstream updates
- **Windows ARM64 support** — cargo-dist supports it by default, but Z3 may not have prebuilt ARM64 DLLs

### Relationship with RFC-014b

| | RFC-014b | RFC-037 |
|--|----------|---------|
| **Scope** | Third-party package build and distribution | Compiler itself packaging and distribution |
| **Tool** | `yaoxiang build` / `yaoxiang publish` | `cargo-dist` |
| **Artifact** | Third-party package FFI libraries | Compiler + standard library + toolchain |
| **Mutually exclusive** | No, complementary | No, complementary |

## Alternatives

| Plan | Why not chosen |
|------|-----------|
| **Continue hand-written CI** | Already hand-written ~600 lines, repetitive work, easy to miss DLL |
| **Write own packaging tool** | Don't reinvent the wheel; cargo-dist is already mature |
| **Only use tar.gz without installers** | Users need friendlier installation methods (Homebrew/MSI) |
| **Docker distribution** | Compilers and language toolchains require native binaries, not a container scenario |
| **Fully statically link Z3** | Ideal solution, but statically compiling Z3 on Windows requires additional CI steps; can be optimized later |

## Implementation Strategy

### Phase 1: Basic Migration (High Priority)

1. Research and confirm the latest version and configuration format of cargo-dist
2. Install cargo-dist, run `dist init` to generate initial configuration
3. Configure `dist-workspace.toml`, specify target platforms
4. Use `cc` crate to replace build.rs's external Z3 download logic (optional)

### Phase 2: Custom Packaging (Medium Priority)

1. Write `package-z3.sh` post-build processing script
2. Add `gen-std-interfaces` subcommand in `main.rs`
3. Call it in the packaging script to generate standard library interface files
4. Verify the generated archive structure is correct

### Phase 3: Deprecate Old CI (High Priority)

1. Integrate cargo-dist pipeline in `release.yml`
2. Run old and new CI in parallel, compare artifact consistency
3. Delete old CI files after confirmation
4. Delete `setup.iss` and related scripts

### Phase 4: Optimization (Low Priority)

1. Research the feasibility of statically compiling Z3 on Windows
2. Add Homebrew formula auto-publish
3. Add MSI installer
4. Consider ARM64 Windows support

### Dependencies

- No external toolchain dependencies (cargo-dist installed via cargo install)
- Requires GitHub Actions to run CI
- Requires Homebrew maintainer account (optional)

### Risks

- **cargo-dist version upgrades**: Configuration format may change, need to watch changelog
- **Z3 official release changes**: Location or format of Z3 prebuilt packages may change
- **Windows static linking**: Z3 static libraries on Windows may require additional handling (e.g., C++ runtime dependencies)

## Open Questions

- [ ] Feasibility of statically linking Z3 on Windows? Need to test `-DZ3_BUILD_LIBZ3_SHARED=OFF` behavior under MSVC
- [ ] Specific naming and interface design of the `gen-std-interfaces` subcommand?
- [ ] Should Inno Setup installer be retained as a supplement to MSI? Domestic users may be more accustomed to exe install wizard
- [ ] Does cargo-dist's `extra-artifacts` support cross-platform conditional execution (e.g., only copy DLL on Windows)?
- [ ] Do standard library interface files have version compatibility guarantees? Should they be released together with the compiler version?

## References

- [cargo-dist Official Documentation](https://axodotdev.github.io/cargo-dist/)
- [cargo-dist GitHub](https://github.com/axodotdev/cargo-dist)
- [RFC-014b: Build System and Binary Distribution](../review/014b-build-system.md)
- [Rust Compiler Distribution Process — bootstrap dist](https://doc.rust-lang.org/stable/nightly-rustc/bootstrap/core/build_steps/dist/index.html)
- [Go Toolchain Distribution — Go Toolchains](https://go.dev/doc/toolchain)
- [Z3 Build Configuration — CMakeLists.txt](https://github.com/Z3Prover/z3/blob/master/src/CMakeLists.txt)
- [Z3 Windows Distribution Script](https://github.com/Z3Prover/z3/blob/master/scripts/mk_win_dist_cmake.py)
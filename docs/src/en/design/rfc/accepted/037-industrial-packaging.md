---
title: 'RFC-037: Industrial Distribution Plan — Compiler/Toolchain Packaging Based on cargo-dist'
author: 'ChenXu233'
created: '2026-07-26'
updated: '2026-09-10'
accepted: '2026-09-09'
issue: '#230'
status: 'Accepted'
---

# RFC-037: Industrial Distribution Plan — Compiler/Toolchain Packaging Based on cargo-dist

> This RFC is complementary to
> [RFC-014b: Build System and Binary Distribution](../review/014b-build-system.md). RFC-014b defines
> how the **YaoXiang package manager** builds and distributes third-party packages; this RFC defines
> how the **YaoXiang compiler/toolchain itself** is packaged and distributed.

## Summary

Use `cargo-dist` (a binary distribution tool from the Rust ecosystem) to handle cross-platform build
orchestration, while in-house scripts handle the distribution package structure. Two core
commitments: the distribution package **physically carries the standard library source directory**
(users can read it directly, just like Python's `Lib/`), and Z3 shared libraries for dynamic linking
on all platforms ship with the package. The command model follows a **front door / engine
separation**: the common command is `yx` (a small front door with built-in version management —
analogous to rustup / Go's GOTOOLCHAIN), and the engine is `yaoxiang-rs` (renamed from the current
monolithic `yaoxiang`), avoiding the ecosystem fragmentation that Python/Node suffered from by
belatedly adopting nvm/pdm. The installation approach has two layers: the standard channel aligns
with Go/Zig — **the distribution package is the product**, just extract it and add to PATH; the
foolproof channel is a one-line install command (Linux `apt` / `curl | sh`, Windows `irm | iex` /
Inno exe wizard). This solves problems such as missing `libz3.dll`, an invisible standard library
for users, and repeated CI script maintenance.

## Motivation

### Why is this feature needed?

Users who download YaoXiang should be able to **use it out of the box** without any additional
steps; the standard library should be **directly readable** by users, not hidden as a black box
inside a binary.

### Current Problems

#### Problem 1: Windows users can't run it after downloading

The current Release only uploads `yaoxiang.exe`, but `libz3.dll` is not bundled. When users
double-click to run it on Windows, they get an error:

```
The code execution cannot proceed because libz3.dll was not found.
```

This is a **blocking bug** — users can't even get past the first step.

#### Problem 2: Release artifacts are only a single-file exe; the standard library is invisible to users

The current state is a triple break:

- Release artifacts are bare binaries; the standard library is not distributed
- The LSP's interface file lookup chain is nearly disconnected: when calling
  `find_std_interface_file`, the project directory is not passed (only the global `~/.yaoxiang/std/`
  is checked, and nothing populates it); `package init` writes to `.yaoxiang/std`, which is not on
  the lookup chain
- The standard library source (`.yx` layer) and interface view (native layer) are completely
  black-boxed for users

The industrial approach: users can directly open the standard library directory to read the source —
just like Python's `Lib/`. **The distribution package physically carrying the std directory is a
hard requirement of this plan** (already decided).

#### Problem 3: Repeated maintenance of hand-written CI scripts

Currently, multiple build pipelines are maintained:

| File                      | Responsibility       | Lines          |
| ------------------------- | -------------------- | -------------- |
| `_build-platforms.yml`    | Cross-platform build | ~255 lines     |
| `release.yml`             | Version release      | ~189 lines     |
| `nightly.yml`             | Daily build          | ~173 lines     |
| `scripts/build/setup.iss` | Inno Setup installer | ~250 lines     |
| **Total**                 |                      | **~870 lines** |

Most of it is repetitive (install Rust → cache → build → rename → upload), written once per
platform.

#### Problem 4: Hard-coded version number in Inno Setup

`setup.iss` has `MyAppVersion` hard-coded as `0.7.0`, and relies on `sed` substitution at build
time. This will eventually fail.

#### Problem 5: Ambiguous boundary with RFC-014b

RFC-014b defines "how YaoXiang packages are built and distributed" (i.e., the `[build]` and
`[binaries]` configuration in `yaoxiang.toml`), but **does not cover "how the YaoXiang compiler
itself is published"**. This RFC fills that gap.

## Proposal

### Core Design

cargo-dist is only responsible for the **build orchestration layer**; the package structure and
installers are all in-house. Division of responsibilities:

```
cargo-dist responsibilities (build orchestration layer):
  ├── Cross-platform compilation (5 targets)
  └── Generate compressed archives and checksums
  (Native installers and npm wrapper are deprecated — their flat-binary assumption conflicts with the bin/+lib/ structure)

build.rs continues to handle:
  └── Z3 download/link (dynamic linking on all platforms + rpath)

YaoXiang's in-house scripts:
  ├── package-dist.sh — Reorganize the package structure (bin/ + lib/), include shared libraries,
  │   populate the std directory (pre-generated interface views from the repo + .yx layer source), recompute checksums
  └── Inno Setup — Windows installation wizard (existing asset; lays out the complete directory structure)

Command model (front door / engine separation):
  ├── yx — Front door (new small crate): version resolution + dispatch; only toolchain/self verbs are retained, others are passed through
  └── yaoxiang-rs — Engine (renamed from the current monolithic yaoxiang): compile/run/package management/fmt/lsp subcommands

Installation method (two layers):
  ├── Standard channel (Go/Zig model): The distribution package is the product, extract + PATH
  └── Foolproof channel (Rust model): One-line command install + version management (built into the yx front door)
      ├── Linux: apt (self-hosted deb repo, system-level flat install) / curl … | sh (one-liner full package install)
      ├── Windows: irm … | iex (one-liner full package install) / Inno Setup wizard (existing asset, system-level flat install)
      └── macOS: curl … | sh (brew waits for the homebrew-core community)
```

### Release Directory Structure (Decided: Physically Carry the Standard Library Source)

Users must be able to read the standard library directly, just like Python's `Lib/` — having a
self-contained std directory in the distribution package is a hard requirement, not a packaging
detail. Same logic for Z3: as an external system, directory-style shared library distribution is its
natural form — stuffing a `.so` into an exe versus keeping it external for dynamic linking is
equivalent in terms of "having to ship with the package", but the latter preserves replaceability.

For each platform's distribution package, `package-dist.sh` reorganizes after the cargo-dist build:

```
yaoxiang-{version}-{target}.tar.gz / .zip     (Portable ready-to-use: run directly from bin/ after extraction)
├── bin/
│   ├── yx                            # Front door (or yx.exe)
│   ├── yaoxiang-rs                   # Engine (or yaoxiang-rs.exe)
│   └── libz3.so / libz3.dylib / libz3.dll
├── lib/
│   └── yaoxiang/
│       └── std/                      # Directly readable by users (Python Lib/ model)
│           ├── io.yx                 # native module: pre-generated interface view from the repo
│           ├── math.yx
│           ├── test.yx               # .yx layer: real source files copied as-is from the repo
│           └── ...
├── README.md
└── LICENSE
```

Installation = extract to any directory + add `bin/` to PATH (Go's `/usr/local/go/bin` model;
extracting to `~/.yaoxiang/` is a common choice). On Windows, the Inno Setup wizard does the same
thing (default Program Files). For portable extraction, the `yx` front door has no `~/.yaoxiang`
state and falls back to the adjacent `yaoxiang-rs` — consistent with managed installation behavior;
the engine's rpath and exe-relative std lookup are not affected by the presence of the front door.

### Platform Support

| Platform       | target triple               | Notes                |
| -------------- | --------------------------- | -------------------- |
| Linux x86_64   | `x86_64-unknown-linux-gnu`  | Primary platform     |
| Linux ARM64    | `aarch64-unknown-linux-gnu` | Cross-compiled in CI |
| macOS x86_64   | `x86_64-apple-darwin`       | Intel Mac            |
| macOS ARM64    | `aarch64-apple-darwin`      | Apple Silicon        |
| Windows x86_64 | `x86_64-pc-windows-msvc`    | Primary platform     |

5 targets in total. Windows ARM64 is not supported for now (Z3 has no official pre-built ARM64
packages).

### Z3 Distribution Strategy

**Dynamic linking on all platforms** (review upheld):

| Platform | Change               | Artifact      |
| -------- | -------------------- | ------------- |
| Linux    | **Static → Dynamic** | `libz3.so`    |
| macOS    | **Static → Dynamic** | `libz3.dylib` |
| Windows  | No change            | `libz3.dll`   |
| wasm32   | No change (static)   | Embedded `.a` |

Rationale:

- **Consistency** — Three platforms behave uniformly, no more per-platform exceptions
- **This is an external library; it should be distributed as a shared library.** Python
  (`python3.dll`+`DLLs/lib*.dll`), Node (`node`+`lib/`) all do this
- **Users can upgrade Z3 without waiting for a compiler version** — just swap out a
  `.so`/`.dylib`/`.dll`
- **Smaller binary size** — Z3 is not small; static linking bloats the exe by several MB

Dynamic linking has one **necessary accompaniment**: Linux/macOS dynamic linkers don't search the
binary's directory by default, so rpath must be injected, otherwise "extract and use" doesn't hold
(Windows searches the exe directory by default, so no handling needed). The corresponding `build.rs`
modification:

```rust
// Unified dynamic linking + rpath
fn link_z3(z3_dir: &Path) {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    // Z3 distribution package layout is not uniform; try both lib/bin directories
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    // RFC-037: Dynamic linking on all platforms. Shared libraries ship with bin/ in the distribution package; users can replace them wholesale to upgrade Z3
    if target_os == "windows" {
        // MSVC import lib is named libz3.lib
        println!("cargo:rustc-link-lib=libz3");
    } else {
        println!("cargo:rustc-link-lib=z3");
        // Dynamic linkers don't search the binary's directory by default; rpath must be injected for "extract and use" to hold
        // (In the distribution package, exe and libz3 are both in bin/; Windows searches the exe directory by default, so no handling needed)
        match target_os.as_str() {
            "linux" => println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN"),
            "macos" => println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path"),
            _ => {}
        }
        let cxx = if target_os == "macos" {
            "c++".to_string()
        } else {
            env::var("CXXSTDLIB").unwrap_or_else(|_| "stdc++".into())
        };
        println!("cargo:rustc-link-lib={}", cxx);
    }
}
```

**"Static linking on all platforms" is not a goal.** This is not about eliminating special cases;
it's eliminating a reasonable case the wrong way. Shared libraries are the normal distribution
method for external libraries.

### Installer Support

A survey of distribution methods for mainstream language toolchains (2026-09):

| Language | Official Distribution        | Official Install Method                       | Installer Maintainer            |
| -------- | ---------------------------- | --------------------------------------------- | ------------------------------- |
| Go       | `go/{bin,src,pkg}` tarball   | Official docs are "download → extract → PATH" | None (brew/apt by community)    |
| Zig      | `zig/{bin,lib/std}` tarball  | Same as above, no official install script     | None (homebrew-core community)  |
| Node     | `{bin,lib,include}` tarball  | tar + official pkg/msi                        | Team-written                    |
| Rust     | Multi-component tarball      | rustup                                        | Team-written                    |
| Crystal  | `{bin,src,embedded}` tarball | deb/rpm/tar                                   | Team + brew community           |
| Deno/Bun | Single-binary zip            | Official curl script                          | Team-written (scripts are tiny) |
| Gleam    | cargo-dist single-binary     | cargo-dist generated scripts                  | cargo-dist                      |

Three rules:

- **No multi-file toolchain uses a third-party generator for installers** — cargo-dist's installers
  only fit the single-binary scenario (Gleam can use it precisely because it's a dependency-free
  single Rust binary)
- The simplest model is **Go/Zig's "distribution package is the product"**: the official install
  guide is just extract + PATH, with zero installer code; distribution packages self-contained with
  readable std source (Go's `src/`, Zig's `lib/std/`, Crystal's `src/`) is the norm
- Those that want curl one-liner install (Deno/Bun/rustup) all use **self-written scripts** that
  barely evolve; brew formulas are all community-maintained in homebrew-core, language teams don't
  build their own tap (Crystal's team explicitly stated the formula is community's)

YaoXiang adopts a two-layer model:

| Channel                                                 | Layer     | Status | Description                                                                                                                 |
| ------------------------------------------------------- | --------- | ------ | --------------------------------------------------------------------------------------------------------------------------- |
| zip / tar.gz                                            | Standard  | ✅     | Extract and use (rpath + same-directory shared library), extract + PATH is the official guide                               |
| `yx` (front door, built-in version management)          | Foolproof | ✅     | rustup/Go GOTOOLCHAIN analog: multi-version install/switch/update + project pin                                             |
| `curl ... \| sh` (install.sh)                           | Foolproof | ✅     | Linux / macOS: download reorganized package into `versions/`, place `bin/yx` at the install root, write the default version |
| `irm ... \| iex` (install.ps1)                          | Foolproof | ✅     | Windows: same logic                                                                                                         |
| `apt install yaoxiang`                                  | Foolproof | ✅     | `.deb` (amd64/arm64) + GitHub Pages static apt repo; system-level flat install, `apt upgrade` follows versions              |
| Inno Setup exe                                          | Foolproof | ✅     | Windows wizard (existing asset), system-level flat install, lays out the complete bin/+lib/ structure                       |
| winget / `.rpm` / homebrew-core / npm                   | —         | ⏸      | Optional follow-up: winget and brew-core are community-maintained; rpm mirrors deb structure                                |
| MSI / cargo-dist native installer / self-built brew tap | —         | ❌     | See "Alternative Approaches"                                                                                                |

**The foolproof channel references Rust, with version management built into the front door.** Rust's
decomposition is "bootstrap script (sh.rustup.rs) → rustup → toolchain packages" — rustup manages
multi-version, switching, and updates from the start; Python/Node's official installer didn't do
this layer, so the ecosystem grew pyenv/nvm/pdm as afterthoughts, each doing its own thing. YaoXiang
adopts **front door / engine separation** (Go's `go` front door + GOTOOLCHAIN, rustup's proxy
dispatch — both are isomorphic): the common command is `yx`, the engine is `yaoxiang-rs` —

```
~/.yaoxiang/
├── bin/yx                  # Front door: small binary, version resolution + dispatch (project pin > default > adjacent engine)
├── settings.toml           # Default version, mirror sources
└── versions/               # Version is a first-class concept; <ver>/ is the extraction root of that version's distribution package
    └── 0.7.14/
        ├── bin/
        │   ├── yaoxiang-rs      # Engine: compile/run/package management/fmt/lsp subcommands
        │   └── libz3.so
        └── lib/yaoxiang/std/
```

- The install root `~/.yaoxiang/` aligns with industry convention (pyenv `~/.pyenv`, nvm `~/.nvm`,
  deno `~/.deno`, bun `~/.bun`, volta `~/.volta` all have a single root; rustup's
  `~/.cargo`+`~/.rustup` double root is a historical artifact of cargo predating rustup, not
  emulated), and `~/.yaoxiang` is already the codebase's existing namespace (std global fallback
  slot); supports `YAOXIANG_HOME` environment variable override (precedent: RUSTUP_HOME /
  DENO_INSTALL, serves CI and container scenarios); on Windows it's `%USERPROFILE%\.yaoxiang`
- **Version directory = distribution package extraction root**: `versions/<ver>/` is fully
  isomorphic to portable extraction and deb install trees — installing a version is just extracting
  a distribution package, with zero structural fork between three channels
- Command surface (rustup analog): `yx toolchain install / default / update / list / uninstall`,
  including `yx self update`; other verbs are transparently passed through to the engine
- **Version locking is a structural guarantee**: tools like fmt evolve in lockstep with syntax (old
  fmt doesn't recognize new syntax), version resolution is done once at the front door, the whole
  group is switched — there is no combination space for "new engine paired with old fmt"; if fmt/LSP
  are later split into independent binaries, they will also live in the same version's `bin/`
- Project-level pin: `yx-toolchain.toml` (precedent: rust-toolchain.toml, follows the command name;
  not in `yaoxiang.toml` — the package manifest shouldn't force toolchain version on library users)
- The bootstrap entry (`curl | sh` / `irm | iex`) installs the latest stable full package in one go:
  extract into `versions/`, place `bin/yx` at the install root, write the default version to
  `settings.toml` (the equivalent convergence of rustup's "bootstrap installs the manager"
  decomposition — the front door and engine are in the same package, no two-step)
- Mirror sources are configurable (settings.toml), continuing consideration for domestic users (same
  network issues as Z3 download)
- **Version management doesn't break the self-containment invariant**: each version is a complete
  distribution tree, rpath and exe-relative std lookup are self-consistent within the tree, the
  front door only dispatches and doesn't change the structure

The `.deb` and Inno are **system-level flat install** channels (root / Program Files single version,
updated by `apt upgrade` / Control Panel), serving servers, CI, and pure novice scenarios;
coexistence with the manager relies on PATH order (precedent: apt's rustc and rustup coexist). All
channels share the same product tree.

The `.deb` layout reuses the same directory tree: `/usr/lib/yaoxiang/` (the complete distribution
tree: `bin/{yx,yaoxiang-rs,libz3.so}` + `lib/yaoxiang/std/`) + a `/usr/bin/yx` symlink pointing to
`/usr/lib/yaoxiang/bin/yx` — `$ORIGIN` is computed by the **resolved real path** after symlink,
still hitting `libz3.so` in the same directory, isomorphic to the extracted package structure. The
brand name stays in the package name and product name (`apt install yaoxiang`, Inno product name
YaoXiang), the command surface is unified as `yx` — same as Go: package name `golang-go`, command
`go`. The apt repo is statically hosted on GitHub Pages (Packages/Release/InRelease metadata with
GPG signatures, published by the release CI); a long-term goal is to apply for inclusion in the
official Debian/Ubuntu repos (long cycle, lagging versions, not the main path).

### Standard Library Directory

The content of `lib/yaoxiang/std/` all comes from static files in the repo; packaging is **pure
copy**, with no runtime generation entry:

| Layer                             | Source                                                                   | Nature                                                     |
| --------------------------------- | ------------------------------------------------------------------------ | ---------------------------------------------------------- |
| native modules (io/math/…)        | Pre-generated interface views from `src/std/interfaces/*.yx` in the repo | Interface signature view (implementation is in the binary) |
| .yx layer modules (test/…growing) | Original files copied from `src/std/*.yx` in the repo                    | Real source code                                           |

Pre-generated views are derived from `StdModule::exports()` (`generate_all_interfaces()` in
`src/std/gen_interfaces.rs`), **no runtime generation subcommand is set up** (decision 2026-09-10:
after packaging is mature, the subcommand is a redundant interface surface). Synchronization uses
the "generated artifacts committed + test gate" pattern (same as RFC-013 code tables):
`test_committed_interface_files_match_generation` byte-compares pre-generated files with the current
generation, drift fails the test; healing goes through the bless entry
`cargo test update_committed_interface_files -- --ignored`. The generation logic depends on the
crate's internal `StdModule` implementation and cannot be moved to build.rs, so the gate is at the
test level rather than the build level.

**Runtime lookup chain** — `find_std_interface_file` adds an exe-relative lookup level:

1. Project `.yaoxiang/vendor/std/<name>.yx` (project override, current)
2. **`<exe directory>/../lib/yaoxiang/std/<name>.yx` (new)**: portable extraction, managed install
   (`versions/<ver>/`), deb flat install all hit this uniformly
3. `~/.yaoxiang/std/<name>.yx` (global fallback, kept as a manual override slot)

The current chain is nearly disconnected (LSP calls don't pass the project directory, `package init`
writes to `.yaoxiang/std` which isn't on the chain); this RFC picks up that loose end and unifies
`package init` output to `.yaoxiang/vendor/std` (consistent with the package manager's vendor
directory).

**Compile authority doesn't change**: the `.yx` layer still uses `include_str!` embedding (RFC-036's
"std version strictly bound to binary" invariant is maintained). The distribution directory is
positioned as a **readable view + LSP resolution source**, not a compile input — user edits to `.yx`
files in the distribution directory won't be picked up by the compiler (whether to open Python-style
"edit `Lib/` and it takes effect" semantics, see Open Questions).

### Wasm Build

**Remain independent, not migrated to cargo-dist.**

cargo-dist handles "shipping the compiler to users"; wasm is "embedding an online playground in the
documentation website" — two completely different deliverables.

| Aspect         | Approach                                           |
| -------------- | -------------------------------------------------- |
| Build tool     | Keep `wasm-pack build`                             |
| CI workflow    | Keep `_build-wasm.yml` as a separate job           |
| Trigger        | Same tag push as release, parallel independent job |
| Publish target | `docs/public/wasm/` → GitHub Pages                 |

### npm Publishing

| Package                | Content                                         | Status                                                                                                                                                                                                                                                       |
| ---------------------- | ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `@yaoxiang/cli`        | Wrapper that downloads the distribution package | Deferred: cargo-dist's npm wrapper is also based on the flat-artifact assumption, deprecated along with the installer; if an npm channel is needed, write your own wrapper (download and extract the reorganized package, same logic as extract-and-install) |
| `@yaoxiang/playground` | wasm library (JS + .wasm)                       | Optional, currently only published to docs                                                                                                                                                                                                                   |

The two don't conflict, and the names don't conflict.

### Integration with the Existing Release Process

Current `release.yml`: push main → check-version (only proceed if `v{version}` tag doesn't exist) →
four parallel jobs: build / build-wasm / security / test → release job (tag + push +
`generate-commit-list.ts` generates body with @mentions + upload artifacts).

The pipeline generated by cargo-dist is tag-driven, comes with announce/publish, and doesn't include
fmt/clippy/test/audit gates, and the release notes format can't carry merge commit changelogs. **A
direct wholesale replacement would break the current release ceremony** (PR → all CI green → bump →
merge commit = changelog).

Integration principle: **keep the trigger and gates as they are, hand over building to
`cargo dist build`, keep publishing as it is.**

1. The check-version / security / test jobs remain as they are (push main triggered, gates before
   tagging)
2. After all pass, the release job creates and pushes the `v{version}` tag (unchanged)
3. Tag push triggers the new `dist-release.yml`: plan job computes the runner/system dependency
   matrix from dist → `cargo dist build` (5 targets) → `package-dist.sh` reorganizes per target →
   Inno Setup job (consumes the Windows reorganized package to build the wizard, injects version via
   `/DMyAppVersion=`, no secondary compilation) → `_build-wasm.yml` (parallel job)
4. Publish job: `generate-commit-list.ts` generates the body (existing script reused) → additionally
   uploads reorganized packages + `.sha256` + `.deb` + wasm + Setup exe; a separate `publish-apt`
   job publishes the GitHub Pages apt repo metadata (auto-skips when `secrets.APT_GPG_KEY` isn't
   configured, doesn't affect other channels)

### Nightly Publishing

cargo-dist has no native nightly support
([axodotdev#1143](https://github.com/axodotdev/cargo-dist/issues/1143), still an open feature
request).

Keep the existing cron + tag-override approach; replace the build portion from
`_build-platforms.yml` with `cargo dist build` — it's essentially a cargo command and can be called
directly in nightly.yml. No workflow reuse (the envisioned `uses: ./release.yml` isn't feasible: the
reuse target needs a `workflow_call` trigger, and the cargo-dist workflow is tag-driven with build
and publish coupled):

```yaml
# nightly.yml (after migration)
on:
  schedule:
    - cron: '17 22 * * *'
jobs:
  build: # cargo dist build + package-dist.sh (same set as the formal release)
  publish: # Kept as-is: create/remove nightly tag → overwrite GitHub Pre-release
```

### cargo-dist Configuration (Committed to dist-workspace.toml)

```toml
[workspace]
members = ["cargo:.", "cargo:tools/yx"]

# Config for 'dist'
[dist]
# Pin dist version (Cargo.toml SemVer syntax)
cargo-dist-version = "0.32.0"
ci = "github"
# Installers are all in-house; cargo-dist only does build + compressed archives + checksums
installers = []
targets = ["aarch64-apple-darwin", "aarch64-unknown-linux-gnu", "x86_64-apple-darwin", "x86_64-unknown-linux-gnu", "x86_64-pc-windows-msvc"]
# The generated workflow has been intentionally modified (package-dist.sh reorganization + in-house publish section, RFC-037),
# not refusing to build due to drift from the generate template
allow-dirty = ["ci"]
```

This is the actual configuration vendored in the repo (generated by `cargo dist init` and then
revised as needed); `cargo-dist-version` is pinned to 0.32.0, the generated workflow is vendored
into the repo for review, not fetched at runtime. The build profile is injected into the root
Cargo.toml by init as `[profile.dist]` (inherits release, lto=thin), and the two binaries land in
`target/<triple>/dist/` for the reorganization script to pick up.

### package-dist.sh (Committed)

Refer to the repo's `scripts/release/package-dist.sh` for the source of truth; key points:

- The two binaries are taken directly from `cargo dist build`'s build output directory
  `target/<triple>/dist/` (profile=dist) — cargo-dist's own self-produced flat single-binary archive
  is not a deliverable; the same-named reorganized package in `target/distrib/` is overwritten
  directly
- The Z3 shared library is also taken from the build output directory — `build.rs` selects the
  appropriate platform's shared library and copies it to disk during linking (`copy_shared_lib`),
  **single source of truth**: the packaging script doesn't know and doesn't need to know the Z3
  version and platform directory naming; Z3 license text is dropped alongside the library and
  carried by the package (MIT distribution obligation), and on the macOS side during packaging the
  dylib's install_name is normalized to `@rpath` and the binary is ad-hoc re-signed
- The std directory is pure copy: `src/std/interfaces/*.yx` (pre-generated interface views) +
  `src/std/*.yx` (real source for the .yx layer)
- Include README/LICENSE; re-package (Windows zip / others tar.gz; three-level fallback for when Git
  Bash has no zip: zip → System32 bsdtar → PowerShell) and recompute `.sha256`
- When on Linux and `dpkg-deb` is available, also call `build-deb.sh` to produce `.deb`
  (`/usr/lib/yaoxiang` flat install tree + `/usr/bin/yx` symlink)

### Deprecated Hand-Written CI

Files to adjust after migration:

| File                                     | Lines          | Disposition                                                            |
| ---------------------------------------- | -------------- | ---------------------------------------------------------------------- |
| `.github/workflows/_build-platforms.yml` | 254            | Delete (cargo-dist build matrix replaces it)                           |
| `.github/workflows/release.yml`          | 189            | Shrinks to gates + tagging (build/publish moved to dist-release.yml)   |
| `.github/workflows/nightly.yml`          | 173            | Build segment replaced with `cargo dist build`, publish logic retained |
| `scripts/build/setup.iss`                | ~250           | **Retained and promoted** (Windows wizard)                             |
| **Total reduction**                      | **~600 lines** |                                                                        |

Retained:

- `ci.yml` (daily fmt + clippy + test + MSRV, not part of the release process)
- `_build-wasm.yml` (independent build flow, attached as a parallel job to dist-release.yml)
- `_build-z3-wasm.yml` (wasm-specific Z3)
- `docs-deploy.yml` (documentation deployment)

### Acceptance Criteria

"Out of the box" is testable; the migration is judged complete not by "old and new artifacts are
identical" but by all of the following passing:

- On a clean machine (no Rust / no Z3 / no `~/.yaoxiang`), extract any platform's compressed archive
  and directly execute `bin/yaoxiang-rs --version` successfully — no `LD_LIBRARY_PATH` needed (rpath
  takes effect)
- All `lib/yaoxiang/std/*.yx` in the extracted directory are readable: native modules are signature
  interface views, the `.yx` layer is real source
- Start LSP on a sample project inside the extracted directory, std member completion /
  go-to-definition is available (exe-relative lookup works)
- Following the official guide, extract to `/usr/local` (or `~/.yaoxiang`) and add to PATH,
  `yx --version` succeeds in any directory
- After `apt install yaoxiang` (self-hosted repo) it works, `apt upgrade` can follow versions; the
  engine's `$ORIGIN` (resolved by real path) under the `/usr/bin/yx` symlink hits `bin/libz3.so`
- After running `curl ... | sh` and `irm ... | iex` in a clean environment, `yx` works and PATH is
  set
- `yx toolchain install <ver>` / `default` / `update` takes effect: multiple versions coexist,
  `yx-toolchain.toml` project pin takes precedence over default version, front door dispatches to
  the correct version (rpath and std lookup within the version tree are self-consistent, no "new
  engine with old fmt" combination)
- After portable extraction, `yx` falls back to the adjacent `yaoxiang-rs`, behavior consistent with
  managed installation
- After Inno Setup installation, the directory structure is complete, PATH takes effect,
  uninstallable
- Release assets are complete: 5 platform reorganized packages + `.sha256` matches actual content
- Release body is `generate-commit-list.ts` output (merge commit changelog complete)
- Nightly artifacts are Pre-release, not affecting the latest formal tag

## Trade-offs

### Advantages

- **Out of the box** — Portable extract and use (rpath + same-directory shared library), installers
  lay out the complete directory
- **Readable standard library** — Users can read std directly, like Python's `Lib/` (hard
  requirement met)
- **Reduced maintenance cost** — ~600 lines of hand-written build YAML replaced by cargo-dist + ~80
  lines of in-house scripts
- **Cross-platform consistency** — Dynamic linking on all platforms + same-directory shared library,
  no exceptions
- **Two-layer installation** — Standard channel has zero new code (Go/Zig model); foolproof channel
  references Rust, apt / curl / iex / exe four entry points share the same product structure
- **Built-in version management** — Front door `yx` mirrors rustup/Go GOTOOLCHAIN, avoiding the
  ecosystem fragmentation that Python/Node suffered from by relying on pyenv/nvm/pdm as
  afterthoughts; tool version locking is a structural guarantee, not a convention

### Disadvantages and Risks

- **Foolproof channel maintenance surface** — install.sh / install.ps1 (one-liner scripts, barely
  evolving) + `.deb` and apt repo metadata publishing (automated by release CI) + `yx` front door
  crate
- **Engine rename blast radius** — `yaoxiang` → `yaoxiang-rs` requires a one-time migration of CI
  artifact names, Inno, tests, and documentation (completed within Phase Five)
- **Learning cost** — The team needs to learn cargo-dist configuration
- **cargo-dist upstream risk** — Briefly stalled in mid-2025 with Axo, but the original author
  revived it in September of the same year and continued releasing versions (0.29 → 0.32+);
  mitigated by `dist-version` pinning + vendoring generated artifacts into the repo for review
- **cargo-dist has no native nightly** — Nightly publishing still requires hand-written parts

### Relationship with RFC-014b

|                        | RFC-014b                                   | RFC-037                                           |
| ---------------------- | ------------------------------------------ | ------------------------------------------------- |
| **Scope**              | Third-party package build and distribution | Packaging and distribution of the compiler itself |
| **Tools**              | `yaoxiang build` / `yaoxiang publish`      | `cargo-dist` + in-house scripts                   |
| **Artifact**           | FFI libraries for third-party packages     | Compiler + standard library + toolchain           |
| **Mutually exclusive** | No, complementary                          | No, complementary                                 |

## Alternative Approaches

| Approach                                    | Why not chosen                                                                                                                                                                                                                                                  |
| ------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Continue hand-written CI**                | Already ~870 lines hand-written, repetitive work, easy to miss DLLs                                                                                                                                                                                             |
| **Write your own packaging tool**           | Don't reinvent the wheel; cargo-dist is already mature                                                                                                                                                                                                          |
| **Only tar.gz, no installer**               | The only official channel is extract + PATH; Inno only serves the Windows wizard habit (retained per decision for domestic users)                                                                                                                               |
| **Docker distribution**                     | Compilers and language toolchains need native binaries, not container scenarios                                                                                                                                                                                 |
| **Self-hosted Homebrew tap**                | Taps are all community-maintained (homebrew-core), self-hosting is a premature need; the macOS entry point for the foolproof channel is the curl script                                                                                                         |
| **Independent `yaoxiangup` manager binary** | The rustup-as-is precedent is workable; but creates a second user-facing verb, and raises the symmetric question of "should package manager/fmt also be independent" — front door/engine separation resolves this in one go (rejected in 2026-09-09 discussion) |
| **Only self-update, no multi-version**      | Single-version self-update doesn't handle the need for different versions pinned by different projects; Python/Node's lack of official version management forced the ecosystem to grow pyenv/nvm/pdm — decided to be built-in (2026-09-09)                      |
| **Statically link Z3 on all platforms**     | Decision rejected — for an external system, directory-style shared library distribution is its natural form; stuffing into an exe versus keeping external is equivalent in "having to ship with the package", but the latter loses replaceability               |
| **Deprecate Inno Setup**                    | Decision rejected — retained as Windows wizard (additional channel)                                                                                                                                                                                             |
| **cargo-dist native installer**             | The flat-binary assumption conflicts with the bin/+lib/ structure; installation result is missing libraries                                                                                                                                                     |
| **Self-maintained WiX/MSI**                 | Without cargo-dist's generator, the cost is higher than the value Inno already covers                                                                                                                                                                           |

## Implementation Strategy

### Phase One: Language-Side Changes (P0)

1. `build.rs`: Unified dynamic linking on all platforms + rpath link-arg; extend `copy_dll()` to
   `copy_shared_lib()` (so/dylib/dll)
2. Pre-generate native interface views in the repo (`src/std/interfaces/`), test gate enforces sync
   with `StdModule::exports()` (gen-std subcommand already decided to be cancelled)
3. Add an exe-relative lookup branch to `find_std_interface_file`; unify `package init` output path
   to `.yaoxiang/vendor/std`

### Phase Two: cargo-dist Integration (P0)

1. Run `cargo dist init` to generate the initial configuration (`installers = []`, pin dist-version)
2. Write `package-dist.sh` (reorganization + .yx source copy + checksum recompute)
3. Create `dist-release.yml` (tag-driven: dist build → reorganize → wasm parallel → in-house
   publish); shrink `release.yml` to gates + tagging
4. Run old and new pipelines in parallel, verify each item against the acceptance criteria

### Phase Three: Old CI Decommission (P1)

1. Delete `_build-platforms.yml` after verification
2. Replace `nightly.yml` build segment with `cargo dist build`
3. Connect `setup.iss` to the new artifact structure (Inno promoted; version number injected from
   Cargo.toml, eliminating sed substitution)

### Phase Four: Foolproof Channel (P2)

1. `install.sh` / `install.ps1` (detect platform → download latest reorganized package → extract
   into `versions/` → place `bin/yx` at install root → write `settings.toml` default version → PATH
   prompt/write; lands in the same round as Phase Five, one-shot to the final state)
2. `.deb` packaging (reuse the same directory tree as `package-dist.sh` + `/usr/bin` symlink) +
   GitHub Pages static apt repo (metadata GPG-signed, published by release CI)

### Phase Five: Front Door yx and Engine Rename (P2)

1. Rename the current monolith to `yaoxiang-rs` (go through all CI artifact names, Inno, tests, and
   documentation in one pass, one-time migration)
2. Add a new front door small crate `yx` (workspace member): version resolution, download release
   artifacts, tar/zip extraction, settings.toml, dispatch (only toolchain/self verbs are retained,
   others are passed through; hand-written dispatch without clap to guarantee verbatim argument
   forwarding); the version index comes from the GitHub Releases latest API (mirror source is a
   ghproxy-style prefix concatenation); the `yx` command name has gone through a name collision
   check (no commonly used command with the same name in mainstream distros/Homebrew, only a niche
   tool uses yx as an alias)
3. `yx toolchain install/default/update/list/uninstall` + `yx-toolchain.toml` project pin + mirror
   sources + `yx self update`
4. Bootstrap scripts: `curl | sh` / `irm | iex` → download the reorganized package to install the
   front door and default stable in one go (lands in the same round as Phase Four as the final
   state)

### Phase Six: Optional Follow-Ups (None Blocking)

1. winget submission (points to Inno exe, community-maintained, symmetric with homebrew-core model)
2. `.rpm` (dnf users, isomorphic to `.deb`)
3. Homebrew: submitted by the community after reaching the homebrew-core admission threshold
4. npm `@yaoxiang/cli` self-written wrapper (the name is currently unregistered)

## Open Questions

### Unresolved

- **Should the .yx layer provide Python-style semantics where "manually editing the distribution
  directory is picked up by compilation"?** Default is no — compile authority keeps RFC-036
  embedding (std version strictly bound to binary), the distribution directory is positioned as a
  readable view + LSP resolution source. If opened in the future, the version-binding invariant
  needs to be re-examined.

### Closed

The following questions were resolved during design discussion:

- ~~Feasibility of Z3 static linking on Windows?~~ → **No static linking, dynamic on all platforms**
  (review upheld 2026-09-09)
- ~~gen-std-interfaces subcommand naming?~~ → **No subcommand** (decision 2026-09-10: after normal
  packaging is mature the subcommand surface is redundant; native interface views switched to repo
  pre-generated `src/std/interfaces/` + test gate synchronization, packaging is pure copy)
- ~~Keep Inno Setup?~~ → **Retained as Windows wizard (additional channel)**
- ~~Does the distribution package structure physically carry standard library source?~~ → **Yes,
  must** (user readability aligns with Python `Lib/`, decided 2026-09-09)
- ~~cargo-dist native installers (shell/powershell/homebrew/msi/npm)?~~ → **All deprecated**,
  installers are in-house (flat assumption conflicts with bin/+lib/)
- ~~Installer strategy?~~ → **Two layers** (final decision 2026-09-09): standard channel =
  distribution package is the product (Go/Zig model, extract + PATH); foolproof channel references
  Rust one-liner install — Linux `apt` (self-hosted deb repo) / `curl | sh`, Windows `irm | iex` /
  Inno exe. Not doing: MSI, cargo-dist native installer, self-built brew tap
- ~~Is a version manager included?~~ → **Yes, belongs to the installer system** (decided 2026-09-09,
  overturning the same-day earlier "long-term separate RFC" boundary): motivated by the Python/Node
  ecosystem fragmentation caused by the lack of official version management that forced
  pyenv/nvm/pdm as afterthoughts
- ~~Version manager form: independent binary or subcommand?~~ → **Front door / engine separation**
  (final decision 2026-09-09, three rounds of convergence A→C→name reversal): the common command
  `yx` = front door (small binary, built-in toolchain/self verbs), engine `yaoxiang-rs` = current
  `yaoxiang` monolith renamed; directory structure `versions/<ver>/` — version is a first-class
  concept, version directory is the distribution package extraction root, tools are locked together
  with the version (old fmt doesn't recognize new syntax; the envisioned inner `toolchains/` was
  cancelled as redundant). Precedent: Go's `go` front door + GOTOOLCHAIN, rustup's proxy dispatch
- ~~Conditional execution of cargo-dist extra-artifacts?~~ → **Handled by `package-dist.sh` script,
  with shell case branches**
- ~~Standard library interface version compatibility?~~ → **Released together with the compiler
  version, in the same compressed package**

## References

- [cargo-dist Official Documentation](https://axodotdev.github.io/cargo-dist/)
- [cargo-dist GitHub](https://github.com/axodotdev/cargo-dist)
- [RFC-014b: Build System and Binary Distribution](../review/014b-build-system.md)
- [cargo-dist nightly feature request](https://github.com/axodotdev/cargo-dist/issues/1143)
- [Z3 Build Configuration — CMakeLists.txt](https://github.com/Z3Prover/z3/blob/master/src/CMakeLists.txt)

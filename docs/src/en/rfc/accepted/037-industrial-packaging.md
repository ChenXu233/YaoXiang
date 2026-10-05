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
> [RFC-014b: Build System and Binary Distribution](014b-build-system.md). RFC-014b defines how the
> **YaoXiang package manager** builds and distributes third-party packages; this RFC defines how the
> **YaoXiang compiler/toolchain itself** is packaged and distributed.

## Summary

Use `cargo-dist` (a binary distribution tool from the Rust ecosystem) to handle cross-platform build
orchestration, with proprietary scripts responsible for the distribution package structure. Two core
commitments: the distribution package **physically carries the standard library source directory**
(users read it directly, just like Python's `Lib/`), and the Z3 shared library dynamically linked
across all platforms is shipped with the package. The command model is a **front door / engine
separation**: the common command `yx` (a small front door with built-in version management—the
counterpart of rustup/Go GOTOOLCHAIN), and the engine `yaoxiang-rs` (renamed from the current
`yaoxiang` monolith), avoiding the ecosystem fragmentation that Python/Node later patched with
nvm/pdm. Installation is two-tiered: the standard channel aligns with Go/Zig—**the distribution
package is the product**, extract + PATH; the foolproof channel offers a one-liner install (Linux
`apt` / `curl | sh`, Windows `irm | iex` / Inno exe wizard). This solves problems like `libz3.dll`
missing, the standard library being invisible to users, and duplicated CI script maintenance.

## Motivation

### Why is this feature needed?

Users who download YaoXiang should be able to **use it out of the box** without any extra steps; the
standard library should be **directly readable** by users, not hidden as a black box inside a
binary.

### Current problems

#### Problem 1: Windows users can't run it after downloading

The current Release only uploads `yaoxiang.exe`, but `libz3.dll` is not packaged. Double-clicking on
Windows yields the error:

```
The code execution cannot proceed because libz3.dll was not found.
```

This is a **blocking bug**—users can't even get past the first step.

#### Problem 2: Release artifacts are just a single exe; the standard library is invisible to users

The current state is a triple break:

- Release artifacts are bare binaries, with no standard library shipped
- The LSP's interface file lookup chain is nearly disconnected: when calling
  `find_std_interface_file`, the project directory is not passed (only the global `~/.yaoxiang/std/`
  is checked, and no process populates it); `package init` writes to `.yaoxiang/std`, which is not
  on the lookup chain
- The standard library source code (`.yx` layer) and interface view (native layer) are completely
  black boxes to users

The industrial approach: users open the standard library directory directly to read the source code,
just like reading Python's `Lib/`—**the distribution package physically carrying the std directory
is a hard requirement of this plan** (already decided).

#### Problem 3: Hand-written CI scripts require repeated maintenance

Multiple build pipelines are currently maintained:

| File                      | Responsibility       | Lines          |
| ------------------------- | -------------------- | -------------- |
| `_build-platforms.yml`    | Cross-platform build | ~255 lines     |
| `release.yml`             | Version release      | ~189 lines     |
| `nightly.yml`             | Daily build          | ~173 lines     |
| `scripts/build/setup.iss` | Inno Setup installer | ~250 lines     |
| **Total**                 |                      | **~870 lines** |

Most of it is duplicated (install Rust → cache → build → rename → upload), and each platform needs
to be written once.

#### Problem 4: Inno Setup version number is hardcoded

In `setup.iss`, `MyAppVersion` is hardcoded to `0.7.0`, and relies on `sed` substitution at build
time. This will fail eventually.

#### Problem 5: Ambiguous boundary with RFC-014b

RFC-014b defines the "YaoXiang package build and distribution mechanism" (i.e., the `[build]` and
`[binaries]` configurations in `yaoxiang.toml`), but **does not cover "how the YaoXiang compiler
itself is released"**. This RFC fills that gap.

## Proposal

### Core design

cargo-dist only handles the **build orchestration layer**; package structure and installers are
entirely proprietary. Division of responsibilities:

```
cargo-dist responsibilities (build orchestration layer):
  ├── Cross-platform compilation (5 targets)
  └── Generate archives and checksums
  (Native installers and npm wrapper abandoned—their flat binary assumption conflicts with the bin/+lib/ structure)

build.rs continues to handle:
  └── Z3 download/linking (all-platform dynamic + rpath)

YaoXiang proprietary scripts:
  ├── package-dist.sh — Reorganize package structure (bin/ + lib/), include shared libraries,
  │   populate the std directory (repo pre-generated interface view + .yx layer source), recompute checksums
  └── Inno Setup — Windows installation wizard (existing asset; lays out the complete directory structure)

Command model (front door / engine separation):
  ├── yx — Front door (new small crate): version resolution + dispatch; only retains toolchain/self verbs, others pass through
  └── yaoxiang-rs — Engine (renamed from current yaoxiang monolith): build/run/package management/fmt/lsp subcommands

Installation methods (two-tier):
  ├── Standard channel (Go/Zig model): distribution package is the product, extract + PATH
  └── Foolproof channel (Rust model): one-liner install + version management (built into the yx front door)
      ├── Linux: apt (self-hosted deb repo, system-level flat install) / curl … | sh (one-line full install)
      ├── Windows: irm … | iex (one-line full install) / Inno Setup wizard (existing asset, system-level flat install)
      └── macOS: curl … | sh (homebrew pending homebrew-core community)
```

### Release directory structure (already decided: physically carries the standard library source)

Users must be able to read the standard library directly, just like reading Python's `Lib/`—the
distribution package shipping with a std directory is a hard requirement, not a packaging detail.
The same applies to Z3: as an external system, directory-based shared library distribution is its
natural form—stuffing `.so` into an exe vs. placing it outside for dynamic linking are equivalent in
terms of "both must ship with the package," but the latter preserves replaceability.

Each platform's distribution package is reorganized by `package-dist.sh` after cargo-dist builds:

```
yaoxiang-{version}-{target}.tar.gz / .zip     (portable, ready-to-use: extract and run directly from bin/)
├── bin/
│   ├── yx                            # Front door (or yx.exe)
│   ├── yaoxiang-rs                   # Engine (or yaoxiang-rs.exe)
│   └── libz3.so / libz3.dylib / libz3.dll
├── lib/
│   └── yaoxiang/
│       └── std/                      # Directly readable by users (Python Lib/ model)
│           ├── io.yx                 # native module: repo pre-generated interface view
│           ├── math.yx
│           ├── test.yx               # .yx layer: real source from the repo copied as-is
│           └── ...
├── README.md
└── LICENSE
```

Installation = extract to any directory + add `bin/` to PATH (the Go `/usr/local/go/bin` model;
extracting to `~/.yaoxiang/` is a common choice). On Windows, the Inno Setup wizard does the same
thing (defaults to Program Files). In portable extraction, the `yx` front door has no `~/.yaoxiang`
state and falls back to the adjacent `yaoxiang-rs`—consistent with managed install behavior; the
engine's rpath and exe-relative std lookup don't change because of the front door.

### Platform support

| Platform       | Target triple               | Notes                |
| -------------- | --------------------------- | -------------------- |
| Linux x86_64   | `x86_64-unknown-linux-gnu`  | Primary platform     |
| Linux ARM64    | `aarch64-unknown-linux-gnu` | Cross-compiled on CI |
| macOS x86_64   | `x86_64-apple-darwin`       | Intel Mac            |
| macOS ARM64    | `aarch64-apple-darwin`      | Apple Silicon        |
| Windows x86_64 | `x86_64-pc-windows-msvc`    | Primary platform     |

5 targets in total. Windows ARM64 is not supported for now (Z3 has no official prebuilt ARM64
packages).

### Z3 distribution strategy

**All-platform dynamic linking** (reviewed and maintained):

| Platform | Change                  | Artifact      |
| -------- | ----------------------- | ------------- |
| Linux    | **Static→Dynamic**      | `libz3.so`    |
| macOS    | **Static→Dynamic**      | `libz3.dylib` |
| Windows  | Unchanged               | `libz3.dll`   |
| wasm32   | Unchanged (static link) | Embedded `.a` |

Rationale:

- **Consistency** — Unified behavior across three platforms, no special cases
- **It's an external library, it should be distributed as a shared library**. Python
  (`python3.dll`+`DLLs/lib*.dll`), Node (`node`+`lib/`) do the same
- **Users can upgrade Z3 without waiting for a compiler version** — Just swap a
  `.so`/`.dylib`/`.dll`
- **Smaller binary size** — Z3 is not small, static linking bloats the exe by several MB

Dynamic linking has one **necessary companion**: the Linux/macOS dynamic linker doesn't search the
binary's directory by default, so rpath must be injected, otherwise "extract and use" doesn't work
(Windows searches the exe directory by default, no handling needed). Corresponding `build.rs`
modification:

```rust
// Unified dynamic linking + rpath
fn link_z3(z3_dir: &Path) {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    // Z3 distribution package layout isn't uniform, probe both lib/ and bin/
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    // RFC-037: All-platform dynamic linking. Shared libraries are shipped with the package in bin/;
    // users can replace the whole thing to upgrade Z3
    if target_os == "windows" {
        // MSVC import lib is named libz3.lib
        println!("cargo:rustc-link-lib=libz3");
    } else {
        println!("cargo:rustc-link-lib=z3");
        // The dynamic linker doesn't search the binary's directory by default; rpath must be injected
        // for "extract and use" to work
        // (The exe and libz3 are in the same bin/ in the distribution package; Windows searches the exe directory by default, no handling needed)
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

**"All-platform static linking Z3" is not a goal.** This is not eliminating special cases—this is
eliminating a reasonable case in the wrong way. Shared libraries are the normal distribution form
for external libraries.

### Installer support

Comparing mainstream language toolchain distribution methods (2026-09 survey):

| Language | Official artifact            | Official install method                       | Installer maintainer           |
| -------- | ---------------------------- | --------------------------------------------- | ------------------------------ |
| Go       | `go/{bin,src,pkg}` tarball   | Official docs are "download → extract → PATH" | None (brew/apt by community)   |
| Zig      | `zig/{bin,lib/std}` tarball  | Same as above, no official install script     | None (homebrew-core community) |
| Node     | `{bin,lib,include}` tarball  | tar + official pkg/msi                        | Team-written                   |
| Rust     | Multi-component tarball      | rustup                                        | Team-written                   |
| Crystal  | `{bin,src,embedded}` tarball | deb/rpm/tar                                   | Team + brew community          |
| Deno/Bun | Single-binary zip            | Official curl script                          | Team-written (script is tiny)  |
| Gleam    | cargo-dist single-binary     | cargo-dist generated scripts                  | cargo-dist                     |

Three rules:

- **No multi-file toolchain uses a third-party generator for installers**—cargo-dist's installer
  only fits the single-binary scenario (Gleam can use it precisely because it's a single Rust binary
  with no external dependencies)
- The simplest model is **Go/Zig's "distribution package is the product"**: the official install
  guide is just extract + PATH, with zero installer code; shipping the std source as readable is the
  norm (Go's `src/`, Zig's `lib/std/`, Crystal's `src/`)
- Those wanting curl one-line install (Deno/Bun/rustup) all use **self-written scripts** that barely
  evolve; brew formulas are all community-maintained in homebrew-core, language teams don't build
  their own tap (Crystal team explicitly says the formula is the community's)

YaoXiang adopts the two-tier model:

| Channel                                                  | Tier      | Status | Description                                                                                                                                       |
| -------------------------------------------------------- | --------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| zip / tar.gz                                             | Standard  | ✅     | Extract and use (rpath + same-directory shared library), extract + PATH is the official guide                                                     |
| `yx` (front door, built-in version management)           | Foolproof | ✅     | rustup/Go GOTOOLCHAIN equivalent: multi-version install/switch/update + project pin                                                               |
| `curl ... \| sh` (install.sh)                            | Foolproof | ✅     | Linux / macOS: download the reorganized package and install it into `versions/`, place `bin/yx` in the install root and write the default version |
| `irm ... \| iex` (install.ps1)                           | Foolproof | ✅     | Windows: same logic                                                                                                                               |
| `apt install yaoxiang`                                   | Foolproof | ✅     | `.deb` (amd64/arm64) + GitHub Pages static apt repo; system-level flat install, `apt upgrade` follows versions                                    |
| Inno Setup exe                                           | Foolproof | ✅     | Windows wizard (existing asset), system-level flat install, lays out the complete bin/+lib/ structure                                             |
| winget / `.rpm` / homebrew-core / npm                    | —         | ⏸      | Optional follow-up: winget and brew-core are community-maintained, rpm and deb are isomorphic                                                     |
| MSI / cargo-dist native installer / self-hosted brew tap | —         | ❌     | See "Alternatives"                                                                                                                                |

**The foolproof channel references Rust, with version management built into the front door.** Rust's
decomposition is "bootstrap script (sh.rustup.rs) → rustup → toolchain package"; rustup handles
multi-version, switching, and updating from the start. Python/Node's official installers didn't do
this layer, and the ecosystem grew pyenv/nvm/pdm to fill the gap, each in its own way. YaoXiang
adopts **front door / engine separation** (Go's `go` front door + GOTOOLCHAIN, rustup's proxy
dispatch—the isomorphic form of both): the common command is `yx`, the engine is `yaoxiang-rs`—

```
~/.yaoxiang/
├── bin/yx                  # Front door: small binary, version resolution + dispatch (project pin > default > adjacent engine)
├── settings.toml           # Default version, mirror sources
└── versions/               # Version is a first-class concept; <ver>/ is the extraction root of that version's distribution package
    └── 0.7.14/
        ├── bin/
        │   ├── yaoxiang-rs      # Engine: build/run/package management/fmt/lsp subcommands
        │   └── libz3.so
        └── lib/yaoxiang/std/
```

- The install root `~/.yaoxiang/` aligns with industry convention (pyenv `~/.pyenv`, nvm `~/.nvm`,
  deno `~/.deno`, bun `~/.bun`, volta `~/.volta` all single-root; rustup's `~/.cargo`+`~/.rustup`
  double root is a historical artifact of cargo pre-dating rustup, not emulated), and `~/.yaoxiang`
  is already the codebase's existing namespace (the std global fallback slot); supports
  `YAOXIANG_HOME` environment variable override (precedent: RUSTUP_HOME / DENO_INSTALL, serving CI
  and container scenarios); Windows is `%USERPROFILE%\.yaoxiang`
- **Version directory = distribution package extraction root**: `versions/<ver>/` is fully
  isomorphic with portable extract and deb install tree—installing a version is just extracting a
  distribution package—zero structural fork across three channels
- Command surface (rustup equivalent): `yx toolchain install / default / update / list / uninstall`,
  including `yx self update`; other verbs are passed through to the engine as-is
- **Version locking is a structural guarantee**: tools like fmt evolve in lockstep with the syntax
  (old fmt doesn't recognize new syntax), version resolution is done once at the front door and the
  whole set switches—no "new engine with old fmt" combination space; if fmt/LSP are split into
  separate binaries in the future, they also live in the same version's `bin/`
- Project-level pin: `yx-toolchain.toml` (precedent: rust-toolchain.toml, follows the command name;
  not in `yaoxiang.toml`—a package manifest shouldn't impose the toolchain version on library
  consumers)
- Bootstrap entry (`curl | sh` / `irm | iex`) installs the latest stable full package in one go:
  extract into `versions/`, `bin/yx` placed in the install root, write `settings.toml` default
  version (equivalent convergence of rustup's "bootstrap installs the manager" decomposition—front
  door and engine in the same package, no two-step)
- Configurable mirror sources (settings.toml), continuing domestic user considerations (same network
  issues as Z3 download)
- **Version management doesn't break the self-contained invariant**: each version is a complete
  distribution tree, rpath and exe-relative std lookup are self-consistent within the tree, the
  front door only does dispatch and doesn't change the structure

`.deb` and Inno are **system-level flat install** channels (root / Program Files single version,
updated by `apt upgrade` / Control Panel), serving servers, CI, and pure beginner scenarios;
coexistence with the manager relies on PATH order (precedent: apt's rustc and rustup coexist). All
channels share the same artifact tree.

`.deb` layout reuses the same directory tree: `/usr/lib/yaoxiang/` (the entire distribution tree:
`bin/{yx,yaoxiang-rs,libz3.so}` + `lib/yaoxiang/std/`) + `/usr/bin/yx` symlink pointing to
`/usr/lib/yaoxiang/bin/yx`—`$ORIGIN` is computed by the **resolved real path** after the symlink,
still hitting the same `libz3.so` directory, isomorphic with the extracted package structure. The
full brand name stays in the package name and product name (`apt install yaoxiang`, Inno product
name YaoXiang), the command surface is unified as `yx`—same as Go: package name `golang-go`, command
`go`. The apt repo is statically hosted on GitHub Pages (Packages/Release/InRelease metadata
GPG-signed, published by release CI); in the long term, can apply for Debian/Ubuntu official
inclusion (long cycle, lagging versions, not the main path).

### Standard library directory

The contents of `lib/yaoxiang/std/` all come from repo static files; packaging is **pure copy**, no
runtime generation entry:

| Layer                             | Source                                                       | Nature                                              |
| --------------------------------- | ------------------------------------------------------------ | --------------------------------------------------- |
| native modules (io/math/…)        | Repo `src/std/interfaces/*.yx` pre-generated interface views | Interface signature view (implementation in binary) |
| .yx layer modules (test/…growing) | Repo `src/std/*.yx` copied as-is                             | Real source code                                    |

Pre-generated views are derived from `StdModule::exports()` (`generate_all_interfaces()` in
`src/std/gen_interfaces.rs`), **no runtime generation subcommand** (2026-09-10 decision: once
packaging is finalized, a subcommand would be a redundant interface surface). Synchronization uses
the "generated artifacts committed + test gate" model (same as RFC-013 codetable):
`test_committed_interface_files_match_generation` byte-by-byte compares pre-generated files with the
current generation, drift fails; recovery goes through the bless entry
`cargo test update_committed_interface_files -- --ignored`. Generation logic depends on the crate's
internal `StdModule` implementation, can't be pushed down to build.rs, so the gate is at test time,
not build time.

**Runtime lookup chain**—`find_std_interface_file` adds one exe-relative lookup:

1. Project `.yaoxiang/vendor/std/<name>.yx` (project override, current behavior)
2. **Exe directory `../lib/yaoxiang/std/<name>.yx` (new)**: portable extract, managed install
   (`versions/<ver>/`), deb flat install all hit this
3. `~/.yaoxiang/std/<name>.yx` (global fallback, kept as a manual override slot)

The current chain is nearly disconnected (LSP calls don't pass the project directory, the
`.yaoxiang/std` written by `package init` is not on the chain), this fix picks that up, and unifies
`package init` output to `.yaoxiang/vendor/std` (consistent with the package manager's vendor
directory).

**Compilation authority does not change**: the `.yx` layer still uses `include_str!` embedding
(RFC-036's "std version is strictly bound to the binary" invariant preserved). The distribution
directory positioning is a **readable view + LSP resolution source**, not a compilation input—user
hand-edits to `.yx` in the distribution directory will not be picked up by the compiler (whether to
open up Python-style "edit `Lib/` and it takes effect" semantics, see open question).

### Wasm build

**Stays independent, not migrated to cargo-dist.**

cargo-dist handles "sending the compiler to users", wasm is "embedding online playground in the docs
site"—two completely different deliverables.

| Aspect         | Approach                                           |
| -------------- | -------------------------------------------------- |
| Build tool     | Keep `wasm-pack build`                             |
| CI workflow    | Keep `_build-wasm.yml` as an independent job       |
| Trigger timing | Same tag push as release, parallel independent job |
| Publish target | `docs/public/wasm/` → GitHub Pages                 |

### npm publishing

| Package                | Content                                         | Status                                                                                                                                                                                                                                                         |
| ---------------------- | ----------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `@yaoxiang/cli`        | Wrapper that downloads the distribution package | Deferred: cargo-dist's npm wrapper is also based on the flat artifact assumption, deprecated along with the installer; if an npm channel is needed, write a wrapper yourself (download and extract the reorganized package, same logic as the extract install) |
| `@yaoxiang/playground` | wasm library (JS + .wasm)                       | Optional, currently only published to docs                                                                                                                                                                                                                     |

The two don't conflict, and the names don't conflict.

### Integration with existing release flow

Current `release.yml`: push main → check-version (only passes if `v{version}` tag doesn't exist) →
build / build-wasm / security / test four branches → release job (tag push +
`generate-commit-list.ts` generates body with @mentions + upload artifacts).

The pipeline generated by cargo-dist is tag-driven, with built-in announce/publish, and doesn't
include fmt/clippy/test/audit gates; the release notes format also can't carry merge commit
changelogs. **Direct wholesale replacement would break the existing release ceremony** (PR → CI all
green → bump → merge commit as changelog).

Integration principle: **triggers and gates stay the same, building goes to `cargo dist build`,
publishing stays the same.**

1. check-version / security / test and tag pushing move into `dist-release.yml`'s `gate` /
   `security` / `test` / `tag` jobs, trigger stays as push main (**tag-driven isn't feasible**: tags
   pushed by `GITHUB_TOKEN` in the workflow don't trigger other workflows; the old `release.yml`
   could only release flat binaries itself, deleted along with `_build-platforms.yml`, single
   release point is this file)
2. Only build after tag is produced: `plan` job uses dist to compute runner/system dependency matrix
   → `cargo dist build` (5 targets) → `package-dist.sh` reorganizes per target → Inno Setup job
   (takes Windows reorganized package to build the wizard, `/DMyAppVersion=` injects version, no
   second compile) → `_build-wasm.yml` (parallel job)
3. publish job: `generate-commit-list.ts` generates body (current script reused) → upload
   reorganized package + `.sha256` + `.deb` + wasm + Setup exe, `action-gh-release` creates the
   Release; independent `publish-apt` job publishes GitHub Pages apt repo metadata (auto-skip when
   `secrets.APT_GPG_KEY` is not configured, doesn't affect other channels)
4. Re-release/rebuild: `workflow_dispatch` specifies an existing tag (`gate` skips the tag step
   accordingly)

### Nightly release

cargo-dist has no native nightly support
([axodotdev#1143](https://github.com/axodotdev/cargo-dist/issues/1143), still an open feature
request).

Keep the current cron + tag override approach, swap the build part from `_build-platforms.yml` to
`cargo dist build`—it's essentially a cargo command, can be called directly in nightly.yml. No
workflow reuse (the imagined `uses: ./release.yml` isn't feasible: the reused party needs a
`workflow_call` trigger, and cargo-dist workflows are tag-driven with build and publish coupled):

```yaml
# nightly.yml (after migration)
on:
  schedule:
    - cron: '17 22 * * *'
jobs:
  build: # cargo dist build + package-dist.sh (same as the official release)
  publish: # Kept as-is: move/create nightly tag → overwrite GitHub Pre-release
```

### cargo-dist configuration (landed as dist-workspace.toml)

```toml
[workspace]
members = ["cargo:.", "cargo:tools/yx"]

# Config for 'dist'
[dist]
# Lock dist version (Cargo.toml SemVer syntax)
cargo-dist-version = "0.32.0"
ci = "github"
# Installers are all proprietary, cargo-dist only does build + archives + checksums
installers = []
targets = ["aarch64-apple-darwin", "aarch64-unknown-linux-gnu", "x86_64-apple-darwin", "x86_64-unknown-linux-gnu", "x86_64-pc-windows-msvc"]
# The generated workflow has been intentionally modified (package-dist.sh reorganization + proprietary publish segment, RFC-037),
# not refusing to build due to drift from the generate template
allow-dirty = ["ci"]
```

The above is the actual config vendored in the repo (after `cargo dist init` generates, modify as
needed); `cargo-dist-version` is locked to 0.32.0, the generated workflow is vendored into the repo
for review, not pulled at runtime. The build profile is injected by init into the root Cargo.toml's
`[profile.dist]` (inherits release, lto=thin), the two binaries are written to
`target/<triple>/dist/` for the reorganization script.

### package-dist.sh (landed)

Refer to the repo's `scripts/release/package-dist.sh`, key points:

- The two binaries are taken directly from `cargo dist build`'s build output directory
  `target/<triple>/dist/` (profile=dist)—the flat single-binary archives that cargo-dist itself
  produces aren't deliverables, the same-named reorganized package in `target/distrib/` directly
  overwrites
- Z3 shared libraries are also taken from the build output directory—`build.rs` already selected the
  corresponding platform shared library and copied it during linking (`copy_shared_lib`), **single
  source**: the packaging script doesn't know and doesn't need to know the Z3 version and platform
  directory naming; the Z3 license text is written with the library and carried by the package (MIT
  distribution obligation), the macOS side unifies the dylib's install_name to `@rpath` during
  packaging and re-signs the binary ad-hoc
- std directory is pure copy: `src/std/interfaces/*.yx` (pre-generated interface views) +
  `src/std/*.yx` (.yx layer real source)
- Include README/LICENSE; repackage (Windows zip / others tar.gz; three-tier fallback when Git Bash
  has no zip: zip → System32 bsdtar → PowerShell) and recompute `.sha256`
- When Linux and `dpkg-deb` is available, additionally call `build-deb.sh` to produce `.deb`
  (`/usr/lib/yaoxiang` flat install tree + `/usr/bin/yx` symlink)

### Deprecated hand-written CI

Files adjusted after migration is complete:

| File                                     | Lines          | Disposition                                                                       |
| ---------------------------------------- | -------------- | --------------------------------------------------------------------------------- |
| `.github/workflows/_build-platforms.yml` | 254            | Delete (cargo-dist build matrix replaces)                                         |
| `.github/workflows/release.yml`          | 189            | Delete (gates and tag pushing merged into dist-release.yml, single release point) |
| `.github/workflows/nightly.yml`          | 173            | Build section swaps to `cargo dist build`, publish logic kept                     |
| `scripts/build/setup.iss`                | ~250           | **Kept and promoted** (Windows wizard)                                            |
| **Total deleted**                        | **~600 lines** |                                                                                   |

Kept:

- `ci.yml` (daily fmt + clippy + test + MSRV, not part of the release flow)
- `_build-wasm.yml` (independent build flow, hooked into dist-release.yml as parallel job)
- `_build-z3-wasm.yml` (wasm-specific Z3)
- `docs-deploy.yml` (docs deployment)

### Acceptance criteria

"Out of the box" is testable; the judgment that migration is complete is not "new and old artifacts
are consistent," but that all of the following pass:

- On a clean machine (no Rust / no Z3 / no `~/.yaoxiang`), extract any platform's archive, run
  `bin/yaoxiang-rs --version` directly—don't set `LD_LIBRARY_PATH` (rpath takes effect)
- All `lib/yaoxiang/std/*.yx` in the extract directory are readable: native modules are signature
  interface views, `.yx` layer is real source
- Start LSP on a sample project in the extract directory, std member completion / go-to-definition
  works (exe-relative lookup takes effect)
- After following the official guide to extract to `/usr/local` (or `~/.yaoxiang`) and add to PATH,
  `yx --version` succeeds in any directory
- After `apt install yaoxiang` (self-hosted repo), it works, and `apt upgrade` follows versions;
  under the `/usr/bin/yx` symlink, the engine's `$ORIGIN` (resolved by real path) hits
  `bin/libz3.so`
- After `curl ... | sh` and `irm ... | iex` execute in a clean environment, `yx` works, PATH is in
  place
- `yx toolchain install <ver>` / `default` / `update` takes effect: multi-version coexistence,
  `yx-toolchain.toml` project pin takes precedence over default version, the front door dispatches
  to the correct version (rpath and std lookup within the version tree are self-consistent, no "new
  engine with old fmt" combination)
- After portable extract, `yx` falls back to the adjacent `yaoxiang-rs`, behavior consistent with
  managed install
- After Inno Setup install, the directory structure is complete, PATH takes effect, uninstallable
- Release assets are complete: 5 platform reorganized packages + `.sha256` consistent with the
  actual content
- Release body is the output of `generate-commit-list.ts` (merge commit changelog complete)
- Nightly artifact is Pre-release, doesn't affect the latest official tag

## Trade-offs

### Pros

- **Out of the box** — Portable extract and use (rpath + same-directory shared library), installer
  lays out the complete directory
- **Standard library is readable** — Users read std directly like Python's `Lib/` (hard requirement
  met)
- **Reduced maintenance cost** — ~600 lines of hand-written build YAML replaced with cargo-dist +
  ~80 lines of proprietary scripts
- **Cross-platform consistency** — All-platform dynamic linking + same-directory shared library, no
  special cases
- **Two-tier install** — Standard channel has zero new code (Go/Zig model); foolproof channel
  references Rust, apt / curl / iex / exe four entry points share the same artifact structure
- **Version management built-in** — Front door `yx` is the rustup/Go GOTOOLCHAIN equivalent, avoids
  the ecosystem fragmentation that Python/Node later patched with pyenv/nvm/pdm; tool version
  locking is a structural guarantee, not a convention

### Cons and risks

- **Foolproof channel maintenance surface** — install.sh / install.ps1 (one-line scripts, barely
  evolve) + `.deb` and apt repo metadata publishing (release CI automated) + `yx` front door crate
- **Engine rename impact** — `yaoxiang` → `yaoxiang-rs` requires one-time migration of CI artifact
  names, Inno, tests, and documentation (completed in phase 5)
- **Learning cost** — Team needs to learn cargo-dist configuration
- **cargo-dist upstream risk** — Briefly stalled with Axo in mid-2025, the original author revived
  it in September of the same year and continues releasing (0.29 → 0.32+); mitigated by locking
  `dist-version` + vendoring generated artifacts into the repo for review
- **cargo-dist has no native nightly** — Nightly release still requires hand-writing part of it

### Relationship with RFC-014b

|                        | RFC-014b                                   | RFC-037                                          |
| ---------------------- | ------------------------------------------ | ------------------------------------------------ |
| **Scope**              | Third-party package build and distribution | The compiler itself's packaging and distribution |
| **Tool**               | `yaoxiang build` / `yaoxiang publish`      | `cargo-dist` + proprietary scripts               |
| **Artifact**           | Third-party package FFI library            | Compiler + standard library + toolchain          |
| **Mutually exclusive** | No, complementary                          | No, complementary                                |

## Alternatives

| Plan                                        | Why not chosen                                                                                                                                                                                                                                         |
| ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Keep hand-written CI**                    | Already hand-wrote ~870 lines, repeated work, easy to miss DLL                                                                                                                                                                                         |
| **Write a packaging tool ourselves**        | Don't reinvent the wheel, cargo-dist is mature                                                                                                                                                                                                         |
| **Only tar.gz, no installer**               | The only official channel is extract + PATH; Inno only serves Windows wizard habits (already decided to keep for domestic users)                                                                                                                       |
| **Docker distribution**                     | Compilers and language toolchains need native binaries, not container scenarios                                                                                                                                                                        |
| **Self-hosted Homebrew tap**                | Taps are all community-maintained (homebrew-core), self-hosting is a premature need; macOS entry for foolproof channel is the curl script                                                                                                              |
| **Independent `yaoxiangup` manager binary** | rustup's original precedent, feasible; but creates a second user verb, and raises the symmetry question of "should package manager/fmt also be independent"—the front door / engine separation resolves this together (2026-09-09 discussion rejected) |
| **Only self-update, no multi-version**      | Single-version self-update can't address multi-project pin different versions; Python/Node lack official version management, ecosystem forced to grow pyenv/nvm/pdm—built-in decided (2026-09-09)                                                      |
| **All-static linking Z3**                   | Rejected—directory-based shared library distribution is the natural form for external systems; stuffing into exe vs. placing outside are equivalent in "both must ship with the package", but the latter loses replaceability                          |
| **Deprecate Inno Setup**                    | Rejected—kept as Windows wizard (additional channel)                                                                                                                                                                                                   |
| **cargo-dist native installer**             | Flat binary assumption conflicts with bin/+lib/ structure, installs are missing libraries                                                                                                                                                              |
| **Self-maintained WiX/MSI**                 | After losing the cargo-dist generator, the cost is higher than the value already covered by Inno                                                                                                                                                       |

## Implementation strategy

### Phase 1: Language-side changes (P0)

1. `build.rs`: all-platform unified dynamic linking + rpath link-arg; `copy_dll()` extended to
   `copy_shared_lib()` (so/dylib/dll)
2. Repo pre-generates native interface views (`src/std/interfaces/`), test gate forces
   synchronization with `StdModule::exports()` (gen-std subcommand already decided to cancel)
3. `find_std_interface_file` adds exe-relative lookup branch; `package init` output path unified to
   `.yaoxiang/vendor/std`

### Phase 2: cargo-dist integration (P0)

1. Run `cargo dist init` to generate initial config (`installers = []`, lock dist-version)
2. Write `package-dist.sh` (reorganization + .yx source copy + checksum recomputation)
3. `dist-release.yml` carries everything: push main trigger → gate version gate → gates (audit / fmt
   / clippy / test) → tag → dist build → reorganization → wasm parallel → proprietary publish + apt;
   `release.yml` and `_build-platforms.yml` deleted
4. Run new and old pipelines in parallel, verify item by item against acceptance criteria

### Phase 3: Old CI decommission (P1)

1. `_build-platforms.yml` and `release.yml` deleted together (single release point:
   `dist-release.yml`)
2. `nightly.yml` build section swaps to `cargo dist build`
3. `setup.iss` hooks into the new artifact structure (Inno promoted; version number injected from
   Cargo.toml, eliminating sed substitution)

### Phase 4: Foolproof channel (P2)

1. `install.sh` / `install.ps1` (detect platform → download latest reorganized package → extract
   into `versions/` → place `bin/yx` in install root → write `settings.toml` default version → PATH
   prompt/write; lands in the same round as phase 5, one step to the final state)
2. `.deb` packaging (reuse the same directory tree from `package-dist.sh` + `/usr/bin` symlink) +
   GitHub Pages static apt repo (metadata GPG-signed, published by release CI)

### Phase 5: Front door yx and engine rename (P2)

1. The current monolith renames to `yaoxiang-rs` (CI artifact names, Inno, tests and documentation
   all go through, one-time migration)
2. New front door small crate `yx` (workspace member): version resolution, download release
   artifacts, tar/zip extraction, settings.toml, dispatch (only retains toolchain/self verbs, others
   pass through, hand-written dispatch without clap to ensure arguments are forwarded verbatim);
   version index comes from GitHub Releases latest API (mirror source is ghproxy-style prefix
   concatenation); the `yx` command name has had a name-collision check (mainstream
   distributions/Homebrew have no common command with the same name, only a niche tool uses yx as an
   alias)
3. `yx toolchain install/default/update/list/uninstall` + `yx-toolchain.toml` project pin + mirror
   source + `yx self update`
4. Bootstrap script: `curl | sh` / `irm | iex` → download reorganized package, install front door
   and default stable in one go (lands in the same round as phase 4 as the final state)

### Phase 6: Optional follow-up (none blocking)

1. winget submission (points to Inno exe, community-maintained, mirrors the homebrew-core model)
2. `.rpm` (dnf users, isomorphic with `.deb`)
3. Homebrew: submit by community after reaching the homebrew-core entry threshold
4. npm `@yaoxiang/cli` self-written wrapper (name currently unregistered)

## Open questions

### Unresolved

- **Does the .yx layer provide Python-style "edit distribution directory and get compiled in"
  semantics?** Default no—compilation authority stays with RFC-036 embedding (std version strictly
  bound to binary), distribution directory is positioned as a readable view + LSP resolution source.
  If opened up in the future, the version-binding invariant needs to be re-examined.

### Closed

The following questions were resolved during design discussion:

- ~~Windows Z3 static linking feasibility?~~ → **No static linking, all-platform dynamic**
  (2026-09-09 review maintained)
- ~~gen-std-interfaces subcommand naming?~~ → **No subcommand** (2026-09-10 decision: once packaging
  is finalized, a subcommand is redundant; native interface views changed to repo pre-generated
  `src/std/interfaces/` + test gate sync, packaging is pure copy)
- ~~Keep Inno Setup?~~ → **Kept as Windows wizard (additional channel)**
- ~~Does the distribution package structure physically carry standard library source?~~ →
  **Required** (user readability aligned with Python's `Lib/`, 2026-09-09 decision)
- ~~cargo-dist native installer (shell/powershell/homebrew/msi/npm)?~~ → **All deprecated**,
  installers are proprietary (flat assumption conflicts with bin/+lib/)
- ~~Installer strategy?~~ → **Two-tier** (2026-09-09 final decision): standard channel =
  distribution package is the product (Go/Zig model, extract + PATH); foolproof channel references
  Rust one-liner install—Linux `apt` (self-hosted deb repo) / `curl | sh`, Windows `irm | iex` /
  Inno exe. Not doing: MSI, cargo-dist native installer, self-hosted brew tap
- ~~Is a version manager included?~~ → **Required, part of the installer system** (2026-09-09
  decision, overturning the same day's earlier "long-term independent RFC" boundary): motivation is
  the ecosystem fragmentation caused by Python/Node lacking official version management, which led
  to pyenv/nvm/pdm patchwork
- ~~Version manager form: independent binary or subcommand?~~ → **Front door / engine separation**
  (2026-09-09 final decision, three rounds of convergence A→C→naming inversion): common command `yx`
  = front door (small binary, built-in toolchain/self verbs), engine `yaoxiang-rs` = current
  `yaoxiang` monolith renamed; directory structure `versions/<ver>/`—version is a first-class
  concept, version directory is the distribution package extraction root, tools are locked together
  with the version (old fmt doesn't recognize new syntax; the once-envisioned inner `toolchains/`
  was cancelled for redundancy). Precedents: Go `go` front door + GOTOOLCHAIN, rustup's proxy
  dispatch
- ~~cargo-dist extra-artifacts conditional execution?~~ → **Handled by `package-dist.sh` script,
  using shell case branches**
- ~~Standard library interface version compatibility?~~ → **Released with the compiler version, in
  the same archive**

## References

- [cargo-dist official documentation](https://axodotdev.github.io/cargo-dist/)
- [cargo-dist GitHub](https://github.com/axodotdev/cargo-dist)
- [RFC-014b: Build System and Binary Distribution](014b-build-system.md)
- [cargo-dist nightly feature request](https://github.com/axodotdev/cargo-dist/issues/1143)
- [Z3 build configuration — CMakeLists.txt](https://github.com/Z3Prover/z3/blob/master/src/CMakeLists.txt)

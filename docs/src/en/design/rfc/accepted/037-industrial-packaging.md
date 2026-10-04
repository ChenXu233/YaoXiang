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

> This RFC complements
> [RFC-014b: Build System and Binary Distribution](../accepted/014b-build-system.md). RFC-014b
> defines how the **YaoXiang package manager** builds and distributes third-party packages; this RFC
> defines how the **YaoXiang compiler/toolchain itself** is packaged and distributed.

## Summary

Use `cargo-dist` (the Rust ecosystem's binary distribution tool) for cross-platform build
orchestration, with our own scripts handling the distribution package structure. Two core
commitments: the distribution package **physically carries the standard library source directory**
(users read it directly like Python's `Lib/`), and the Z3 shared library dynamically linked across
all platforms is shipped with the package. The command model is **front-door/engine separation**:
the common command `yx` (a small front door with built-in version management—mirroring rustup / Go's
GOTOOLCHAIN), and the engine `yaoxiang-rs` (the current monolithic `yaoxiang` renamed), avoiding the
ecosystem fragmentation that Python/Node suffered from later patching up with nvm/pdm. Two-tier
installation: the standard channel aligns with Go/Zig—**the distribution package IS the product**,
extract + PATH; the friendly channel is a one-liner install (Linux `apt` / `curl | sh`, Windows
`irm | iex` / Inno exe wizard). This solves problems like missing `libz3.dll`, the standard library
being invisible to users, and duplicated CI script maintenance.

## Motivation

### Why is this feature needed?

Users who download YaoXiang should be able to **use it out of the box** without any extra steps; the
standard library should be **directly readable by users**, not hidden as a black box inside a
binary.

### Current Problems

#### Problem 1: Windows users can't run it after downloading

The current Release only uploads `yaoxiang.exe`, but `libz3.dll` is not bundled. Double-clicking on
Windows produces:

```
The code execution cannot proceed because libz3.dll was not found.
```

This is a **blocking bug**—users can't even get past the first step.

#### Problem 2: Release artifacts are only single-file exe; standard library is invisible to users

The current state has three layers of disconnection:

- Release artifacts are bare binaries; the standard library is not shipped with the distribution
- The LSP's interface file lookup chain is nearly broken: when calling `find_std_interface_file`,
  the project directory is not passed (it only checks the global `~/.yaoxiang/std/`, and no process
  populates it); `package init` writes to `.yaoxiang/std`, which is not on the lookup chain
- The standard library source (`.yx` layer) and the interface view (native layer) are completely
  black boxes to users

The industrial approach: users directly open the standard library directory and read source code,
just like Python's `Lib/`—**the distribution package physically carrying the std directory is a hard
requirement of this plan** (already decided).

#### Problem 3: Hand-written CI scripts are duplicated maintenance

Multiple build pipelines are currently maintained:

| File                      | Responsibility       | Lines          |
| ------------------------- | -------------------- | -------------- |
| `_build-platforms.yml`    | Cross-platform build | ~255 lines     |
| `release.yml`             | Version release      | ~189 lines     |
| `nightly.yml`             | Daily build          | ~173 lines     |
| `scripts/build/setup.iss` | Inno Setup installer | ~250 lines     |
| **Total**                 |                      | **~870 lines** |

Most of it is repetitive (install Rust → cache → build → rename → upload), written once per
platform.

#### Problem 4: Inno Setup version number is hardcoded

`setup.iss` has `MyAppVersion` hardcoded to `0.7.0`, with the build relying on `sed` replacement.
This is bound to fail eventually.

#### Problem 5: Blurred boundary with RFC-014b

RFC-014b defines the "YaoXiang package build and distribution mechanism" (i.e., the `[build]` and
`[binaries]` configuration in `yaoxiang.toml`), but **does not cover "how the YaoXiang compiler
itself is released"**. This RFC fills that gap.

## Proposal

### Core Design

cargo-dist only handles the **build orchestration layer**; the package structure and installers are
all our own. Division of responsibilities:

```
cargo-dist responsibilities (build orchestration layer):
  ├── Cross-platform compilation (5 targets)
  └── Generate archives and checksums
  (Native installers and npm wrappers are deprecated—their flat binary assumption
   conflicts with bin/+lib/ structure)

build.rs continues to handle:
  └── Z3 download/linking (dynamic on all platforms + rpath)

YaoXiang's own scripts:
  ├── package-dist.sh — Restructure package (bin/ + lib/), include shared libraries,
  │   populate std directory (repository pre-generated interface view + .yx layer source),
  │   recompute checksum
  └── Inno Setup — Windows installation wizard (existing asset; lays out complete directory structure)

Command model (front-door/engine separation):
  ├── yx — front door (new small crate): version resolution + dispatch; keeps only toolchain/self verbs, others pass through
  └── yaoxiang-rs — engine (current yaoxiang monolith renamed): compile/run/package-management/fmt/lsp subcommands

Installation methods (two-tier):
  ├── Standard channel (Go/Zig model): distribution package is the product, extract + PATH
  └── Friendly channel (Rust model): one-liner install + version management (built into the yx front door)
      ├── Linux: apt (self-hosted deb repo, system-level flat install) / curl … | sh (one-line full install)
      ├── Windows: irm … | iex (one-line full install) / Inno Setup wizard (existing asset, system-level flat install)
      └── macOS: curl … | sh (brew pending homebrew-core community)
```

### Release Directory Structure (Decided: Physically Carry Standard Library Source)

Users must be able to directly read the standard library like Python's `Lib/`—the distribution
package carrying its own std directory is a hard requirement, not a packaging detail. Same for Z3:
as an external system, directory-style shared library distribution is its natural form—stuffing
`.so` into an exe versus placing it outside for dynamic linking are equivalent in "both must ship
with the distribution," but the latter retains replaceability.

Each platform's distribution package, restructured by `package-dist.sh` after cargo-dist builds:

```
yaoxiang-{version}-{target}.tar.gz / .zip     (portable ready-to-use: after extraction, bin/ runs directly)
├── bin/
│   ├── yx                            # front door (or yx.exe)
│   ├── yaoxiang-rs                   # engine (or yaoxiang-rs.exe)
│   └── libz3.so / libz3.dylib / libz3.dll
├── lib/
│   └── yaoxiang/
│       └── std/                      # directly readable by users (Python Lib/ model)
│           ├── io.yx                 # native module: repository pre-generated interface view
│           ├── math.yx
│           ├── test.yx               # .yx layer: real source from the repository copied as-is
│           └── ...
├── README.md
└── LICENSE
```

Installation = extract to any directory + add `bin/` to PATH (Go's `/usr/local/go/bin` model;
extracting to `~/.yaoxiang/` is a common choice). On Windows, the Inno Setup wizard does the same
(default Program Files). For portable extraction, the `yx` front door has no `~/.yaoxiang` state and
falls back to the adjacent `yaoxiang-rs`—identical behavior to managed installation; the engine's
rpath and exe-relative std lookup don't change because the front door exists.

### Platform Support

| Platform       | target triple               | Notes                |
| -------------- | --------------------------- | -------------------- |
| Linux x86_64   | `x86_64-unknown-linux-gnu`  | Primary              |
| Linux ARM64    | `aarch64-unknown-linux-gnu` | Cross-compiled on CI |
| macOS x86_64   | `x86_64-apple-darwin`       | Intel Mac            |
| macOS ARM64    | `aarch64-apple-darwin`      | Apple Silicon        |
| Windows x86_64 | `x86_64-pc-windows-msvc`    | Primary              |

Total 5 targets. Windows ARM64 is not supported for now (Z3 has no official pre-built ARM64
package).

### Z3 Distribution Strategy

**Dynamic linking on all platforms** (decision maintained after re-review):

| Platform | Change               | Artifact      |
| -------- | -------------------- | ------------- |
| Linux    | **Static → Dynamic** | `libz3.so`    |
| macOS    | **Static → Dynamic** | `libz3.dylib` |
| Windows  | No change            | `libz3.dll`   |
| wasm32   | No change (static)   | Embedded `.a` |

Reasons:

- **Consistency** — uniform behavior across three platforms, no platform-specific quirks
- **It's an external library, so it should be distributed as a shared library**. Python
  (`python3.dll`+`DLLs/lib*.dll`) and Node (`node`+`lib/`) do this
- **Users can upgrade Z3 without waiting for a compiler release** — just swap a
  `.so`/`.dylib`/`.dll`
- **Smaller binary size** — Z3 is not small; static linking bloats the exe by several MB

Dynamic linking has one **necessary companion**: Linux/macOS dynamic linkers don't search the
binary's directory by default, so rpath must be injected, otherwise "extract and use" doesn't hold
(Windows searches the exe's directory by default, no action needed). Corresponding `build.rs`
change:

```rust
// Unified dynamic linking + rpath
fn link_z3(z3_dir: &Path) {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    // Z3 release package layout is inconsistent; try both lib/bin directories
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    // RFC-037: dynamic linking on all platforms. Shared library distributed in bin/;
    // users can replace it entirely to upgrade Z3
    if target_os == "windows" {
        // MSVC import lib is named libz3.lib
        println!("cargo:rustc-link-lib=libz3");
    } else {
        println!("cargo:rustc-link-lib=z3");
        // Dynamic linkers don't search the binary's directory by default; rpath must be
        // injected for "extract and use" to work
        // (in the distribution package, exe and libz3 are both in bin/;
        //  Windows searches the exe directory by default, no action needed)
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

**"Static linking on all platforms" is not a goal.** This isn't eliminating edge cases; it's
eliminating a reasonable case in the wrong way. Shared library is the normal distribution form for
external libraries.

### Installer Support

Comparing distribution methods of mainstream language toolchains (Sept 2026 research):

| Language | Official Release             | Official Install Method                       | Installer Maintainer                       |
| -------- | ---------------------------- | --------------------------------------------- | ------------------------------------------ |
| Go       | `go/{bin,src,pkg}` tarball   | Official docs are "download → extract → PATH" | None (brew/apt is community)               |
| Zig      | `zig/{bin,lib/std}` tarball  | Same; no official install script              | None (homebrew-core community)             |
| Node     | `{bin,lib,include}` tarball  | tar + official pkg/msi                        | Self-written by team                       |
| Rust     | Multi-component tarball      | rustup                                        | Self-written by team                       |
| Crystal  | `{bin,src,embedded}` tarball | deb/rpm/tar                                   | Team + brew community                      |
| Deno/Bun | Single-binary zip            | Official curl scripts                         | Self-written by team (scripts are minimal) |
| Gleam    | cargo-dist single-binary     | cargo-dist generated scripts                  | cargo-dist                                 |

Three patterns:

- **No multi-file toolchain uses a third-party generator for installers**—cargo-dist's installer
  only fits the single-binary scenario (Gleam works precisely because it's a single Rust binary with
  no external dependencies)
- The simplest model is **Go/Zig's "distribution package is the product"**: the official install
  guide is extract + PATH, zero installer code; it's common for the distribution to carry readable
  std source (Go's `src/`, Zig's `lib/std/`, Crystal's `src/`)
- Those wanting curl one-liner installs (Deno/Bun/rustup) all **self-write scripts** that barely
  evolve; brew formulas are all community-maintained in homebrew-core; language teams don't
  self-host taps (the Crystal team explicitly says the formula is community's)

YaoXiang adopts a two-tier model:

| Channel                                                  | Tier     | Status | Description                                                                                                                |
| -------------------------------------------------------- | -------- | ------ | -------------------------------------------------------------------------------------------------------------------------- |
| zip / tar.gz                                             | Standard | ✅     | Extract-and-use (rpath + same-directory shared library), extract + PATH is the official guide                              |
| `yx` (front door, built-in version mgmt)                 | Friendly | ✅     | rustup/Go GOTOOLCHAIN equivalent: multi-version install/switch/update + project pin                                        |
| `curl ... \| sh` (install.sh)                            | Friendly | ✅     | Linux / macOS: download restructured package into `versions/`, `bin/yx` placed at install root and default version written |
| `irm ... \| iex` (install.ps1)                           | Friendly | ✅     | Windows: same logic                                                                                                        |
| `apt install yaoxiang`                                   | Friendly | ✅     | `.deb` (amd64/arm64) + GitHub Pages static apt repo; system-level flat install, `apt upgrade` tracks version               |
| Inno Setup exe                                           | Friendly | ✅     | Windows wizard (existing asset), system-level flat install, lays out full bin/+lib/ structure                              |
| winget / `.rpm` / homebrew-core / npm                    | —        | ⏸      | Optional follow-up: winget and brew-core are community-maintained, rpm mirrors deb structure                               |
| MSI / cargo-dist native installer / self-hosted brew tap | —        | ❌     | See "Alternatives"                                                                                                         |

**The friendly channel follows Rust, with version management built into the front door.** Rust's
decomposition is "bootstrap script (sh.rustup.rs) → rustup → toolchain packages"; rustup handles
multi-version, switching, and updates from the start. Python/Node's official installers didn't have
this layer, and the ecosystem grew pyenv/nvm/pdm afterward, each going their own way. YaoXiang
adopts **front-door/engine separation** (Go's `go` front door + GOTOOLCHAIN, rustup's proxy
dispatch—two instances of the same pattern): the common command is `yx`, the engine is
`yaoxiang-rs`—

```
~/.yaoxiang/
├── bin/yx                  # front door: small binary, version resolution + dispatch
│                           # (project pin > default > adjacent engine)
├── settings.toml           # default version, mirror source
└── versions/               # version is a first-class concept; <ver>/ is that version's distribution extraction root
    └── 0.7.14/
        ├── bin/
        │   ├── yaoxiang-rs      # engine: compile/run/package-management/fmt/lsp subcommands
        │   └── libz3.so
        └── lib/yaoxiang/std/
```

- Install root `~/.yaoxiang/` aligns with industry convention (pyenv `~/.pyenv`, nvm `~/.nvm`, deno
  `~/.deno`, bun `~/.bun`, volta `~/.volta` all use a single root; rustup's `~/.cargo`+`~/.rustup`
  dual root is historical baggage from cargo predating rustup—not emulated), and `~/.yaoxiang` is
  already a namespace in the codebase (std global fallback slot); supports `YAOXIANG_HOME`
  environment variable override (precedent: RUSTUP_HOME / DENO_INSTALL, serves CI and container
  scenarios); Windows is `%USERPROFILE%\.yaoxiang`
- **Version directory = distribution package extraction root**: `versions/<ver>/` is fully
  isomorphic with portable extraction and deb install trees; installing a version is extracting a
  distribution package—three channels with zero structural divergence
- Command surface (rustup equivalent): `yx toolchain install / default / update / list / uninstall`,
  including `yx self update`; other verbs pass through to the engine verbatim
- **Version lock is a structural guarantee**: tools like fmt evolve with syntax in the same version
  (old fmt doesn't understand new syntax); version resolution happens once at the front door, the
  whole set switches together—no combinatorial space of "new engine with old fmt"; if fmt/LSP are
  later split into independent binaries, they will land in the same version's `bin/`
- Project-level pin: `yx-toolchain.toml` (precedent: rust-toolchain.toml, follows command name; not
  in `yaoxiang.toml`—a package manifest shouldn't impose toolchain version on library users)
- Bootstrap entry (`curl | sh` / `irm | iex`) installs the latest stable full package in one shot:
  extract into `versions/`, place `bin/yx` at the install root, write `settings.toml` default
  version (equivalent convergence to rustup's "bootstrap installs manager" decomposition—front door
  and engine in the same package, no two-step)
- Mirror source is configurable (settings.toml), continuing the consideration for users in China
  (same network issue as Z3 download)
- **Version management doesn't break the self-contained invariant**: each version is a complete
  distribution tree, rpath and exe-relative std lookup are self-consistent within the tree, the
  front door only dispatches, doesn't modify structure

`.deb` and Inno are **system-level flat install** channels (root / Program Files, single version,
tracked by `apt upgrade` / Control Panel), serving server, CI, and pure-beginner scenarios;
coexistence with the manager relies on PATH order (precedent: apt's rustc and rustup coexist). All
channels share the same product tree.

`.deb` layout reuses the same directory tree: `/usr/lib/yaoxiang/` (the whole distribution tree:
`bin/{yx,yaoxiang-rs,libz3.so}` + `lib/yaoxiang/std/`) + `/usr/bin/yx` symlink pointing to
`/usr/lib/yaoxiang/bin/yx`—`$ORIGIN` is computed against the **resolved real path**, so after
symlink resolution it still hits `libz3.so` in the same directory, isomorphic with the extracted
package structure. The full brand name stays on the package name and product name
(`apt install yaoxiang`, Inno product name YaoXiang), command surface unified as `yx`—same as Go:
package name `golang-go`, command `go`. The apt repo is statically hosted on GitHub Pages
(Packages/Release/InRelease metadata with GPG signature, published by release CI); long-term may
apply for inclusion in Debian/Ubuntu official archives (long cycle, version lag, not the main path).

### Standard Library Directory

`lib/yaoxiang/std/` contents all come from repository static files; packaging is **pure copy**, no
runtime generation entry point:

| Layer                              | Source                                                      | Nature                                              |
| ---------------------------------- | ----------------------------------------------------------- | --------------------------------------------------- |
| Native modules (io/math/…)         | Repo `src/std/interfaces/*.yx` pre-generated interface view | Interface signature view (implementation in binary) |
| .yx layer modules (test/… growing) | Repo `src/std/*.yx` copied as-is                            | Real source code                                    |

The pre-generated view is derived from `StdModule::exports()` (`generate_all_interfaces()` in
`src/std/gen_interfaces.rs`), **no runtime generation subcommand** (2026-09-10 decision: once
packaging is finalized, subcommands are redundant interface surface). Synchronization uses a
"generated artifacts checked in + test gate" pattern (same as RFC-013 codetable):
`test_committed_interface_files_match_generation` byte-compares the pre-generated files with current
generation, red on drift; healing via bless entry
`cargo test update_committed_interface_files -- --ignored`. Generation logic depends on the
crate-internal `StdModule` implementation and cannot be pushed down to build.rs, so the gate is at
test time rather than build time.

**Runtime lookup chain**—`find_std_interface_file` gains a new exe-relative lookup level:

1. Project `.yaoxiang/vendor/std/<name>.yx` (project override, current)
2. **Exe's directory `../lib/yaoxiang/std/<name>.yx` (new)**: portable extraction, managed install
   (`versions/<ver>/`), deb flat install all hit this uniformly
3. `~/.yaoxiang/std/<name>.yx` (global fallback, kept as a manual override slot)

The current chain is nearly broken (LSP call doesn't pass project directory, `package init` writes
to `.yaoxiang/std` which isn't on the chain); this work also fixes that and unifies `package init`
output to `.yaoxiang/vendor/std` (consistent with the package manager's vendor directory).

**Compilation authority does not change**: the `.yx` layer still uses `include_str!` embedding
(RFC-036's "std version strictly bound to binary" invariant is maintained). The distribution
directory position is a **readable view + LSP resolution source**, not a compilation input—user
modifications to `.yx` in the distribution directory won't be picked up by the compiler (whether to
open Python-style "edit `Lib/` takes effect" semantics, see Open Questions).

### Wasm Build

**Kept independent, not migrated into cargo-dist.**

cargo-dist manages "ship the compiler to users"; wasm is "embed online playground in docs site"—two
completely different deliverables.

| Aspect         | Approach                                           |
| -------------- | -------------------------------------------------- |
| Build tool     | Keep `wasm-pack build`                             |
| CI workflow    | Keep `_build-wasm.yml` as independent job          |
| Trigger timing | Same tag push as release, parallel independent job |
| Publish target | `docs/public/wasm/` → GitHub Pages                 |

### npm Publishing

| Package                | Contents                                     | Status                                                                                                                                                                                                                                           |
| ---------------------- | -------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `@yaoxiang/cli`        | Wrapper for downloading distribution package | Deferred: cargo-dist's npm wrapper is also based on the flat artifact assumption, deprecated along with the installer; if npm channel is needed, self-written wrapper (download restructured package and extract, same logic as extract-install) |
| `@yaoxiang/playground` | wasm library (JS + .wasm)                    | Optional, currently only published to docs                                                                                                                                                                                                       |

The two don't conflict, nor do the names.

### Integration with Existing Release Process

Current `release.yml`: push main → check-version (`v{version}` tag doesn't exist → pass) → four
lanes: build / build-wasm / security / test → release job (push tag + `generate-commit-list.ts`
generates body with @mentions + upload artifacts).

cargo-dist's generated pipeline is tag-driven, has its own announce/publish, doesn't include
fmt/clippy/test/audit gates, and its release notes format can't carry the merge commit changelog.
**Direct wholesale replacement would break the existing release ceremony** (PR → CI all green → bump
→ merge commit as changelog).

Integration principle: **trigger and gates stay as-is, build goes to `cargo dist build`, publish
stays as-is.**

1. check-version / security / test and tag pushing all move into `dist-release.yml`'s `gate` /
   `security` / `test` / `tag` jobs, trigger stays as push main (**tag-driven is not feasible**: a
   tag pushed by `GITHUB_TOKEN` in a workflow doesn't trigger other workflows; the old `release.yml`
   therefore had to ship flat binaries itself, and was deleted along with `_build-platforms.yml`;
   the single release entry point is this file)
2. Build only after tag is produced: `plan` job uses dist to compute runner/system dependency matrix
   → `cargo dist build` (5 targets) → `package-dist.sh` restructures per target → Inno Setup job
   (consumes Windows restructured package to build wizard, `/DMyAppVersion=` injects version, no
   second compilation) → `_build-wasm.yml` (parallel job)
3. publish job: `generate-commit-list.ts` generates body (reuse existing script) → upload
   restructured packages + `.sha256` + `.deb` + wasm + Setup exe, `action-gh-release` self-built
   Release; independent `publish-apt` job publishes GitHub Pages apt repo metadata (auto-skips when
   `secrets.APT_GPG_KEY` isn't configured, doesn't affect other channels)
4. Re-release/re-build: `workflow_dispatch` specifies an existing tag (`gate` skips tag step
   accordingly)

### Nightly Release

cargo-dist has no native nightly support
([axodotdev#1143](https://github.com/axodotdev/cargo-dist/issues/1143), still an open feature
request).

Keep the existing cron + tag override approach; replace the build section from
`_build-platforms.yml` with `cargo dist build`—it's essentially a cargo command, callable directly
in nightly.yml. No workflow reuse (the imagined `uses: ./release.yml` is not feasible: the reused
party needs a `workflow_call` trigger, and cargo-dist's workflow is tag-driven with build and
publish coupled):

```yaml
# nightly.yml (after migration)
on:
  schedule:
    - cron: '17 22 * * *'
jobs:
  build: # cargo dist build + package-dist.sh (same as release)
  publish: # kept as-is: create/move nightly tag → overwrite GitHub Pre-release
```

### cargo-dist Configuration (Already in dist-workspace.toml)

```toml
[workspace]
members = ["cargo:.", "cargo:tools/yx"]

# Config for 'dist'
[dist]
# Lock dist version (Cargo.toml SemVer syntax)
cargo-dist-version = "0.32.0"
ci = "github"
# Installers all self-managed, cargo-dist only does build + archive + checksum
installers = []
targets = ["aarch64-apple-darwin", "aarch64-unknown-linux-gnu", "x86_64-apple-darwin", "x86_64-unknown-linux-gnu", "x86_64-pc-windows-msvc"]
# The generated workflow has been intentionally modified (package-dist.sh restructuring + self-written publish section, RFC-037),
# don't refuse to build due to drift from the generate template
allow-dirty = ["ci"]
```

The above is the actual configuration vendored in the repository (generated by `cargo dist init`
then modified as needed); `cargo-dist-version` locked to 0.32.0, the generated workflow is vendored
into the repository for review, not fetched at runtime. The build profile is injected by init into
the root Cargo.toml's `[profile.dist]` (inherits release, lto=thin), dual binary lands in
`target/<triple>/dist/` for the restructure script to use.

### package-dist.sh (Already in Place)

The repository's `scripts/release/package-dist.sh` is authoritative; key points:

- Dual binary is taken directly from `cargo dist build`'s build output directory
  `target/<triple>/dist/` (profile=dist)—cargo-dist's self-produced flat single-binary archives are
  not deliverables; the same-named restructured package directly overwrites in `target/distrib/`
- Z3 shared library is also taken from the build output directory—build.rs has already selected and
  copied the corresponding platform shared library during linking (`copy_shared_lib`), **single
  source**: the packaging script doesn't know and doesn't need to know Z3 version and platform
  directory naming; the Z3 license text is dropped with the library and carried by packaging (MIT
  distribution obligation), and on the macOS side the dylib's install_name is normalized to `@rpath`
  at packaging time and the binary is ad-hoc re-signed
- std directory is pure copy: `src/std/interfaces/*.yx` (pre-generated interface view) +
  `src/std/*.yx` (.yx layer real source)
- Attach README/LICENSE; re-pack (Windows zip / others tar.gz; if Git Bash has no zip, three-tier
  fallback: zip → System32 bsdtar → PowerShell) and recompute `.sha256`
- On Linux when `dpkg-deb` is available, also call `build-deb.sh` to produce `.deb`
  (`/usr/lib/yaoxiang` flat-install tree + `/usr/bin/yx` symlink)

### Deprecated Hand-written CI

Files adjusted after migration:

| File                                     | Lines          | Disposition                                                                       |
| ---------------------------------------- | -------------- | --------------------------------------------------------------------------------- |
| `.github/workflows/_build-platforms.yml` | 254            | Delete (cargo-dist build matrix replaces it)                                      |
| `.github/workflows/release.yml`          | 189            | Delete (gates and tag pushing merged into dist-release.yml, single release entry) |
| `.github/workflows/nightly.yml`          | 173            | Build section replaced with `cargo dist build`, publish logic kept                |
| `scripts/build/setup.iss`                | ~250           | **Kept and promoted to official** (Windows wizard)                                |
| **Total reduction**                      | **~600 lines** |                                                                                   |

Kept:

- `ci.yml` (daily fmt + clippy + test + MSRV, not part of release process)
- `_build-wasm.yml` (independent build flow, hooked as a parallel job in dist-release.yml)
- `_build-z3-wasm.yml` (wasm-specific Z3)
- `docs-deploy.yml` (docs deployment)

### Acceptance Criteria

"Out of the box" is testable; the migration completion criterion is not "old and new products match"
but all of the following passing:

- On a clean machine (no Rust / no Z3 / no `~/.yaoxiang`), extract any platform archive, directly
  execute `bin/yaoxiang-rs --version` successfully—no `LD_LIBRARY_PATH` needed (rpath works)
- All `lib/yaoxiang/std/*.yx` in the extracted directory are readable: native modules are signature
  interface views, `.yx` layer is real source
- Start LSP on an example project in the extracted directory, std member completion /
  go-to-definition works (exe-relative lookup works)
- Follow the official guide to extract to `/usr/local` (or `~/.yaoxiang`) and add to PATH, then
  `yx --version` succeeds from any directory
- After `apt install yaoxiang` (self-hosted repo) it runs, `apt upgrade` tracks version; the
  engine's `$ORIGIN` under the `/usr/bin/yx` symlink (resolved by real path) hits `bin/libz3.so`
- `curl ... | sh` and `irm ... | iex` on a clean environment, after execution `yx` runs, PATH is in
  place
- `yx toolchain install <ver>` / `default` / `update` work: multiple versions coexist,
  `yx-toolchain.toml` project pin takes priority over default, the front door dispatches to the
  correct version (rpath and std lookup self-consistent within the version tree, no "new engine with
  old fmt" combination)
- After portable extraction, `yx` falls back to adjacent `yaoxiang-rs`, behavior consistent with
  managed install
- After Inno Setup install, directory structure is complete, PATH works, uninstallable
- Release assets complete: 5 platform restructured packages + `.sha256` consistent with actual
  content
- Release body is `generate-commit-list.ts` output (merge commit changelog complete)
- Nightly product is Pre-release, doesn't affect the latest official tag

## Trade-offs

### Advantages

- **Out of the box** — portable extract-and-use (rpath + same-directory shared library), installer
  lays out complete directory
- **Standard library readable** — users directly read std like Python's `Lib/` (hard requirement
  met)
- **Reduced maintenance cost** — ~600 lines of hand-written build YAML replaced with cargo-dist +
  ~80 lines of own scripts
- **Cross-platform consistency** — dynamic linking + same-directory shared library on all platforms,
  no quirks
- **Two-tier installation** — standard channel has zero new code (Go/Zig model); friendly channel
  follows Rust, apt / curl / iex / exe four entry points share the same product structure
- **Built-in version management** — front door `yx` mirrors rustup/Go GOTOOLCHAIN, avoiding
  Python/Node's ecosystem fragmentation from post-hoc pyenv/nvm/pdm; tool version lock is a
  structural guarantee, not a convention

### Disadvantages and Risks

- **Friendly channel maintenance surface** — install.sh / install.ps1 (one-line scripts, barely
  evolve) + `.deb` and apt repo metadata publishing (automated by release CI) + `yx` front door
  crate
- **Engine rename impact** — `yaoxiang` → `yaoxiang-rs` requires a one-time migration of CI artifact
  names, Inno, tests, and documentation (completed within Phase 5)
- **Learning cost** — team needs to learn cargo-dist configuration
- **cargo-dist upstream risk** — stalled mid-2025 with Axo, but the original author revived it in
  September of the same year and has continued releasing (0.29 → 0.32+); mitigated by `dist-version`
  lock + generated artifacts vendored into the repo for review
- **cargo-dist has no native nightly** — nightly release part still needs to be hand-written

### Relationship with RFC-014b

|                      | RFC-014b                                   | RFC-037                                   |
| -------------------- | ------------------------------------------ | ----------------------------------------- |
| **Scope**            | Third-party package build and distribution | Compiler's own packaging and distribution |
| **Tools**            | `yaoxiang build` / `yaoxiang publish`      | `cargo-dist` + own scripts                |
| **Output**           | Third-party package FFI libraries          | Compiler + standard library + toolchain   |
| **Mutual exclusion** | No, complementary                          | No, complementary                         |

## Alternatives

| Plan                                       | Why Not                                                                                                                                                                                                                                  |
| ------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Continue hand-written CI**               | Already hand-written ~870 lines, repetitive work, easy to miss DLLs                                                                                                                                                                      |
| **Write own packaging tool**               | Don't reinvent the wheel; cargo-dist is already mature                                                                                                                                                                                   |
| **Only use tar.gz, no installer**          | The only official channel is extract + PATH; Inno only serves Windows wizard habit (decided to retain for users in China)                                                                                                                |
| **Docker distribution**                    | Compilers and language toolchains need native binaries, not container scenarios                                                                                                                                                          |
| **Self-hosted Homebrew tap**               | Taps are all community-maintained (homebrew-core), self-hosting is ahead of need; the macOS entry of the friendly channel is the curl script                                                                                             |
| **Standalone `yaoxiangup` manager binary** | Rustup's exact precedent, feasible; but creates a second user verb and invites the symmetric question of "should package manager/fmt also be independent"—front-door/engine separation resolves both (rejected in 2026-09-09 discussion) |
| **Self-update only, no multi-version**     | Single-version self-update can't handle the need for different project pins on different versions; Python/Node lack official version management, the ecosystem is forced to grow pyenv/nvm/pdm—decided to build in (2026-09-09)          |
| **Fully static Z3 linking**                | Decision rejected—external system directory-style shared library distribution is its natural form; stuffing into exe versus placing outside is equivalent in "both must ship with the package," but the latter loses replaceability      |
| **Deprecate Inno Setup**                   | Decision rejected—kept as Windows wizard (additional channel)                                                                                                                                                                            |
| **cargo-dist native installer**            | Flat binary assumption conflicts with bin/+lib/ structure, install result lacks libraries                                                                                                                                                |
| **Self-maintained WiX/MSI**                | Cost exceeds the value already covered by Inno once the cargo-dist generator is dropped                                                                                                                                                  |

## Implementation Strategy

### Phase 1: Language-side Changes (P0)

1. `build.rs`: unify dynamic linking + rpath link-arg on all platforms; extend `copy_dll()` to
   `copy_shared_lib()` (so/dylib/dll)
2. Repository pre-generates native interface views (`src/std/interfaces/`), test gate forces sync
   with `StdModule::exports()` (gen-std subcommand decided cancelled)
3. `find_std_interface_file` gains exe-relative lookup branch; `package init` output path unified to
   `.yaoxiang/vendor/std`

### Phase 2: cargo-dist Integration (P0)

1. Run `cargo dist init` to generate initial configuration (`installers = []`, lock dist-version)
2. Write `package-dist.sh` (restructure + .yx source copy + checksum recompute)
3. `dist-release.yml` carries everything: push main trigger → gate version check → gates (audit /
   fmt / clippy / test) → push tag → dist build → restructure → wasm parallel → own publish + apt;
   `release.yml` and `_build-platforms.yml` deleted
4. Run new and old pipelines in parallel, verify each acceptance criterion

### Phase 3: Old CI Decommission (P1)

1. `_build-platforms.yml` and `release.yml` deleted together (single release entry:
   `dist-release.yml`)
2. `nightly.yml` build section replaced with `cargo dist build`
3. `setup.iss` hooked to new product structure (Inno promoted to official; version number injected
   from Cargo.toml, eliminating sed replacement)

### Phase 4: Friendly Channel (P2)

1. `install.sh` / `install.ps1` (detect platform → download latest restructured package → extract
   into `versions/` → place `bin/yx` at install root → write `settings.toml` default version → PATH
   prompt/write; lands in the same round as Phase 5, one-step final state)
2. `.deb` packaging (reuse `package-dist.sh`'s same directory tree + `/usr/bin` symlink) + GitHub
   Pages static apt repo (metadata GPG-signed, published by release CI)

### Phase 5: Front Door yx and Engine Rename (P2)

1. Current monolith renamed to `yaoxiang-rs` (CI artifact names, Inno, tests, and documentation all
   swept through, one-time migration)
2. Add new front door small crate `yx` (workspace member): version resolution, download release
   artifacts, tar/zip extraction, settings.toml, dispatch (keep only toolchain/self verbs, others
   pass through; manually written dispatch without clap to guarantee verbatim argument forwarding);
   version index from GitHub Releases latest API (mirror source as ghproxy-style prefix
   concatenation); `yx` command name has been checked for collisions (no same-name common command in
   mainstream distros/Homebrew, only a small niche tool uses yx as an alias)
3. `yx toolchain install/default/update/list/uninstall` + `yx-toolchain.toml` project pin + mirror
   source + `yx self update`
4. Bootstrap scripts: `curl | sh` / `irm | iex` → download restructured package to install front
   door and default stable in one shot (lands in the same round as Phase 4 as final state)

### Phase 6: Optional Follow-up (None Blocking)

1. winget submission (pointing to Inno exe, community-maintained, symmetric to homebrew-core model)
2. `.rpm` (dnf users, isomorphic to `.deb`)
3. Homebrew: submitted by community after reaching homebrew-core inclusion threshold
4. npm `@yaoxiang/cli` self-written wrapper (name currently unregistered)

## Open Questions

### Open

- **Should the .yx layer provide Python-style semantics of "manually editing the distribution
  directory is adopted by the compiler"?** Default no—compilation authority maintains RFC-036
  embedding (std version strictly bound to binary), the distribution directory position is a
  readable view + LSP resolution source. If opened in the future, the version-binding invariant
  needs to be re-examined.

### Closed

The following questions were resolved during design discussion:

- ~~Feasibility of static linking Z3 on Windows?~~ → **No static linking, dynamic on all platforms**
  (2026-09-09 decision maintained)
- ~~gen-std-interfaces subcommand naming?~~ → **No subcommand** (2026-09-10 decision: once packaging
  is finalized, subcommand surface is redundant; native interface views changed to repo
  pre-generated `src/std/interfaces/` + test gate sync, packaging is pure copy)
- ~~Keep Inno Setup?~~ → **Kept as Windows wizard (additional channel)**
- ~~Does the distribution package structure physically carry standard library source?~~ →
  **Required** (user readability aligned with Python `Lib/`, 2026-09-09 decision)
- ~~cargo-dist native installers (shell/powershell/homebrew/msi/npm)?~~ → **All deprecated**,
  installers self-managed (flat assumption conflicts with bin/+lib/)
- ~~Installer strategy?~~ → **Two-tier** (2026-09-09 final): standard channel = distribution package
  is the product (Go/Zig model, extract + PATH); friendly channel follows Rust one-line
  install—Linux `apt` (self-hosted deb repo) / `curl | sh`, Windows `irm | iex` / Inno exe. Not
  doing: MSI, cargo-dist native installer, self-hosted brew tap
- ~~Include version manager?~~ → **Required, part of installer system** (2026-09-09 decision,
  overturning the same morning's "future independent RFC" boundary): motivation is Python/Node's
  lack of official version management led to pyenv/nvm/pdm post-hoc ecosystem fragmentation
- ~~Version manager form: standalone binary or subcommand?~~ → **Front-door/engine separation**
  (2026-09-09 final, three rounds of convergence A→C→name reversal): common command `yx` = front
  door (small binary, built-in toolchain/self verbs), engine `yaoxiang-rs` = current `yaoxiang`
  monolith renamed; directory structure `versions/<ver>/`—version is a first-class concept, version
  directory IS the distribution package extraction root, tools lock with version as a whole (old fmt
  doesn't understand new syntax; the once-imagined inner `toolchains/` was cancelled as redundant).
  Precedents: Go's `go` front door + GOTOOLCHAIN, rustup's proxy dispatch
- ~~cargo-dist extra-artifacts conditional execution?~~ → **Handle with `package-dist.sh` script,
  use shell case branches**
- ~~Standard library interface version compatibility?~~ → **Released together with compiler version,
  in the same archive**

## References

- [cargo-dist Official Documentation](https://axodotdev.github.io/cargo-dist/)
- [cargo-dist GitHub](https://github.com/axodotdev/cargo-dist)
- [RFC-014b: Build System and Binary Distribution](../accepted/014b-build-system.md)
- [cargo-dist nightly feature request](https://github.com/axodotdev/cargo-dist/issues/1143)
- [Z3 Build Configuration — CMakeLists.txt](https://github.com/Z3Prover/z3/blob/master/src/CMakeLists.txt)

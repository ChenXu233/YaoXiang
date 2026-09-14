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
orchestration, with in-house scripts handling distribution package structure. The two core
commitments are: the distribution package **physically carries the standard library source
directory** (users can read it directly, like reading Python's `Lib/`), and dynamically linked Z3
shared libraries for all platforms ship with the package. The command model follows a **front door /
engine separation**: the common command is `yx` (a small front door, with built-in version
management — equivalent to rustup / Go GOTOOLCHAIN), while the engine is `yaoxiang-rs` (renamed from
the current monolithic `yaoxiang`), avoiding the ecosystem fragmentation that Python/Node suffered
from relying on nvm/pdm after the fact. Installation comes in two layers: the standard channel
aligns with Go/Zig — **the distribution package is the product**, extract + PATH; the easy channel
is a one-line command install (Linux `apt` / `curl | sh`, Windows `irm | iex` / Inno exe wizard).
This solves issues like missing `libz3.dll`, an invisible standard library, and repetitive CI script
maintenance.

## Motivation

### Why is this feature needed?

Users who download YaoXiang should be able to **use it out of the box**, with no extra steps; the
standard library should be **directly readable** by users, not hidden as a black box inside the
binary.

### Current Problems

#### Problem 1: Windows users can't run it after downloading

The current Release only uploads `yaoxiang.exe`, but `libz3.dll` is not bundled. Double-clicking on
Windows produces this error:

```
The code execution cannot proceed because libz3.dll was not found.
```

This is a **blocking bug** — users can't even take the first step.

#### Problem 2: Release artifacts are just a single-file exe, the standard library is invisible to users

The current state is a threefold break:

- Release artifacts are bare binaries; the standard library doesn't ship with the distribution
- The LSP's interface file lookup chain is nearly orphaned: `find_std_interface_file` is called
  without a project directory (it only checks the global `~/.yaoxiang/std/`, and nothing populates
  it); `package init` writes to `.yaoxiang/std`, which is not in the lookup chain
- Standard library source (`.yx` layer) and interface view (native layer) are completely black-boxed
  from users

The industrial approach: users can open the standard library directory directly and read the source
— like reading Python's `Lib/`. **Having the distribution package physically carry the std directory
is a hard requirement of this plan** (already decided).

#### Problem 3: Repeated maintenance of hand-written CI scripts

Multiple build pipelines are currently being maintained:

| File                      | Responsibility       | Lines          |
| ------------------------- | -------------------- | -------------- |
| `_build-platforms.yml`    | Cross-platform build | ~255 lines     |
| `release.yml`             | Version release      | ~189 lines     |
| `nightly.yml`             | Daily build          | ~173 lines     |
| `scripts/build/setup.iss` | Inno Setup installer | ~250 lines     |
| **Total**                 |                      | **~870 lines** |

Most of it is repetitive (install Rust → cache → build → rename → upload), and each platform
requires its own script.

#### Problem 4: Hard-coded version number in Inno Setup

`MyAppVersion` is hard-coded to `0.7.0` in `setup.iss`, relying on `sed` substitution at build time.
It will eventually fail.

#### Problem 5: Blurred boundary with RFC-014b

RFC-014b defines "YaoXiang package build and distribution mechanisms" (i.e. the `[build]` and
`[binaries]` configs in `yaoxiang.toml`), but it **doesn't cover "how the YaoXiang compiler itself
is released"**. This RFC fills that gap.

## Proposal

### Core Design

cargo-dist only handles the **build orchestration layer**; package structure and installers are all
in-house. Responsibility division:

```
cargo-dist responsibilities (build orchestration layer):
  ├── Cross-platform compilation (5 targets)
  └── Generate archives and checksums
  (Native installers and npm wrappers are abandoned — their flat-binary assumption conflicts with the bin/+lib/ structure)

build.rs continues to handle:
  └── Z3 download/linking (all-platform dynamic + rpath)

YaoXiang in-house scripts:
  ├── package-dist.sh — Restructure package (bin/ + lib/), attach shared library,
  │   populate std directory (repo pre-generated interface views + .yx layer sources), recompute checksum
  └── Inno Setup — Windows install wizard (existing asset; lays out full directory structure)

Command model (front door / engine separation):
  ├── yx — Front door (new small crate): version resolution + dispatch; keeps only toolchain/self as verbs, transparently passes through the rest
  └── yaoxiang-rs — Engine (current yaoxiang monolith renamed): compile/run/package management/fmt/lsp subcommands

Installation (two layers):
  ├── Standard channel (Go/Zig model): distribution package is the product, extract + PATH
  └── Easy channel (Rust model): one-line install + version management (built into the yx front door)
      ├── Linux: apt (self-hosted deb repo, system-level flat install) / curl … | sh (one-line install of full package)
      ├── Windows: irm … | iex (one-line install of full package) / Inno Setup wizard (existing asset, system-level flat install)
      └── macOS: curl … | sh (brew pending homebrew-core community)
```

### Release Directory Structure (Decided: Physically Carry Standard Library Source)

Users must be able to directly read the standard library, like reading Python's `Lib/` — having the
distribution package ship with the std directory is a hard requirement, not a packaging detail. The
same applies to Z3: as an external system, directory-based shared library distribution is its
natural form — stuffing `.so` into an exe is equivalent to putting it outside and dynamically
linking in terms of "both must ship with the distribution package", but the latter preserves
replaceability.

Each platform's distribution package is restructured by `package-dist.sh` after cargo-dist builds:

```
yaoxiang-{version}-{target}.tar.gz / .zip     (portable, ready-to-use: extract and run directly from bin/)
├── bin/
│   ├── yx                            # Front door (or yx.exe)
│   ├── yaoxiang-rs                   # Engine (or yaoxiang-rs.exe)
│   └── libz3.so / libz3.dylib / libz3.dll
├── lib/
│   └── yaoxiang/
│       └── std/                      # Directly readable by users (Python Lib/ model)
│           ├── io.yx                 # native modules: pre-generated interface views from repo
│           ├── math.yx
│           ├── test.yx               # .yx layer: real source copied verbatim from the repo
│           └── ...
├── README.md
└── LICENSE
```

Install = extract to any directory + add `bin/` to PATH (Go's `/usr/local/go/bin` model; extracting
to `~/.yaoxiang/` is a common choice). On Windows, the Inno Setup wizard does the same thing
(defaulting to Program Files). For portable extraction, when `yx` front door has no `~/.yaoxiang`
state, it falls back to the adjacent `yaoxiang-rs` — same behavior as managed install; the engine's
rpath and exe-relative std lookup don't change because the front door exists.

### Platform Support

| Platform       | target triple               | Notes               |
| -------------- | --------------------------- | ------------------- |
| Linux x86_64   | `x86_64-unknown-linux-gnu`  | Main platform       |
| Linux ARM64    | `aarch64-unknown-linux-gnu` | Cross-compile on CI |
| macOS x86_64   | `x86_64-apple-darwin`       | Intel Mac           |
| macOS ARM64    | `aarch64-apple-darwin`      | Apple Silicon       |
| Windows x86_64 | `x86_64-pc-windows-msvc`    | Main platform       |

Five targets in total. Windows ARM64 is not yet supported (Z3 has no official pre-built ARM64
package).

### Z3 Distribution Strategy

**All-platform dynamic linking** (review maintained):

| Platform | Change             | Output        |
| -------- | ------------------ | ------------- |
| Linux    | **Static→Dynamic** | `libz3.so`    |
| macOS    | **Static→Dynamic** | `libz3.dylib` |
| Windows  | Unchanged          | `libz3.dll`   |
| wasm32   | Unchanged (static) | Embedded `.a` |

Reasons:

- **Consistency** — uniform behavior across three platforms, no more special cases
- **This is an external library, it should be distributed as a shared library**. Python
  (`python3.dll` + `DLLs/lib*.dll`), Node (`node` + `lib/`) both do this
- **Users can upgrade Z3 without waiting for a compiler version** — just swap a
  `.so`/`.dylib`/`.dll`
- **Smaller binary size** — Z3 is not small; static linking would bloat the exe by several MB

Dynamic linking has one **necessary companion**: the dynamic linker on Linux/macOS doesn't search
the binary's directory by default, so rpath must be injected, otherwise "extract and use" doesn't
work (Windows searches the exe's directory by default, no action needed). The corresponding
`build.rs` change:

```rust
// Unified dynamic linking + rpath
fn link_z3(z3_dir: &Path) {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    // Z3 distribution package layout is inconsistent, probe both lib/ and bin/
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    // RFC-037: all-platform dynamic linking. Shared library ships with bin/ in distribution, users can replace/upgrade Z3 as a whole
    if target_os == "windows" {
        // MSVC import lib is named libz3.lib
        println!("cargo:rustc-link-lib=libz3");
    } else {
        println!("cargo:rustc-link-lib=z3");
        // Dynamic linker doesn't search binary's directory by default, must inject rpath for "extract and use" to work
        // (exe and libz3 are in the same bin/ in distribution; Windows searches exe's directory by default, no action needed)
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

**"All-platform static linking Z3" is not a goal.** This isn't about eliminating special cases —
it's eliminating a reasonable case in the wrong way. Shared library is the natural distribution
method for an external library.

### Installer Support

Comparing distribution methods of mainstream language toolchains (2026-09 survey):

| Language | Official distribution        | Official install method                    | Installer maintainer             |
| -------- | ---------------------------- | ------------------------------------------ | -------------------------------- |
| Go       | `go/{bin,src,pkg}` tarball   | Official docs: "download → extract → PATH" | None (brew/apt are community)    |
| Zig      | `zig/{bin,lib/std}` tarball  | Same as above, no official install script  | None (homebrew-core community)   |
| Node     | `{bin,lib,include}` tarball  | tar + official pkg/msi                     | Team self-written                |
| Rust     | Multi-component tarball      | rustup                                     | Team self-written                |
| Crystal  | `{bin,src,embedded}` tarball | deb/rpm/tar                                | Team + brew community            |
| Deno/Bun | Single-binary zip            | Official curl script                       | Team self-written (scripts tiny) |
| Gleam    | cargo-dist single binary     | cargo-dist generated scripts               | cargo-dist                       |

Three patterns:

- **No multi-file toolchain uses a third-party generator for installers** — cargo-dist's installer
  only fits the single-binary case (Gleam can use it precisely because it's a single Rust binary
  with no external dependencies)
- The simplest model is **Go/Zig's "distribution package is the product"**: official install
  instructions are just extract + PATH, with zero installer code; distribution packages with
  readable std source included (Go's `src/`, Zig's `lib/std/`, Crystal's `src/`) is the norm
- Want curl one-line install? (Deno/Bun/rustup) all use **self-written scripts** that rarely evolve;
  brew formulas are always community-maintained in homebrew-core, language teams don't self-host
  taps (Crystal team explicitly says formulas are community)

YaoXiang adopts a two-layer model:

| Channel                                                  | Layer    | Status | Description                                                                                                             |
| -------------------------------------------------------- | -------- | ------ | ----------------------------------------------------------------------------------------------------------------------- |
| zip / tar.gz                                             | Standard | ✅     | Extract and use (rpath + same-directory shared library); extract + PATH is the official guide                           |
| `yx` (front door, built-in version management)           | Easy     | ✅     | rustup/Go GOTOOLCHAIN equivalent: multi-version install/switch/update + project pin                                     |
| `curl ... \| sh` (install.sh)                            | Easy     | ✅     | Linux / macOS: download restructured package into `versions/`, place `bin/yx` at install root and write default version |
| `irm ... \| iex` (install.ps1)                           | Easy     | ✅     | Windows: same logic                                                                                                     |
| `apt install yaoxiang`                                   | Easy     | ✅     | `.deb` (amd64/arm64) + GitHub Pages static apt repo; system-level flat install, `apt upgrade` follows versions          |
| Inno Setup exe                                           | Easy     | ✅     | Windows wizard (existing asset), system-level flat install, lays out full bin/+lib/ structure                           |
| winget / `.rpm` / homebrew-core / npm                    | —        | ⏸      | Optional follow-up: winget and brew-core are community-maintained, rpm is isomorphic to deb                             |
| MSI / cargo-dist native installer / self-hosted brew tap | —        | ❌     | See "Alternative Plans"                                                                                                 |

**The easy channel is modeled after Rust, with version management built into the front door.**
Rust's decomposition is "bootstrap script (sh.rustup.rs) → rustup → toolchain package", and rustup
manages multi-version, switching, and updating from the start; Python/Node's official installers
didn't do this, so the ecosystem had to grow pyenv/nvm/pdm, each going its own way. YaoXiang adopts
**front door / engine separation** (Go's `go` front door + GOTOOLCHAIN, rustup's proxy dispatch —
both are isomorphic): the common command is `yx`, the engine is `yaoxiang-rs` —

```
~/.yaoxiang/
├── bin/yx                  # Front door: small binary, version resolution + dispatch (project pin > default > adjacent engine)
├── settings.toml           # Default version, mirror sources
└── versions/               # Version is a first-class concept; <ver>/ is the extract root of that version's distribution package
    └── 0.7.14/
        ├── bin/
        │   ├── yaoxiang-rs      # Engine: compile/run/package management/fmt/lsp subcommands
        │   └── libz3.so
        └── lib/yaoxiang/std/
```

- The install root `~/.yaoxiang/` aligns with industry conventions (pyenv `~/.pyenv`, nvm `~/.nvm`,
  deno `~/.deno`, bun `~/.bun`, volta `~/.volta` all use single root; rustup's
  `~/.cargo`+`~/.rustup` dual root is a historical artifact of cargo predating rustup, not
  imitated), and `~/.yaoxiang` is already an existing namespace in the codebase (std global fallback
  slot); supports `YAOXIANG_HOME` env var override (precedent: RUSTUP_HOME / DENO_INSTALL, serves CI
  and container scenarios); on Windows it's `%USERPROFILE%\.yaoxiang`
- **Version directory = distribution package extract root**: `versions/<ver>/` is fully isomorphic
  to portable extract and deb install trees — installing a version is just extracting a distribution
  package — zero structural divergence across three channels
- Command surface (rustup equivalent): `yx toolchain install / default / update / list / uninstall`,
  including `yx self update`; other verbs pass through verbatim to the engine
- **Version locking is a structural guarantee**: tools like fmt evolve with the syntax of the same
  version (old fmt doesn't recognize new syntax), version resolution happens once at the front door,
  switching the whole set — there's no "new engine + old fmt" combination space; in the future, if
  fmt/LSP are split into separate binaries, they also live in the same version's `bin/`
- Project-level pin: `yx-toolchain.toml` (precedent: rust-toolchain.toml, follows command name; not
  in `yaoxiang.toml` — package manifests shouldn't impose toolchain version on library users)
- Bootstrap entry (`curl | sh` / `irm | iex`) installs the latest stable package in one go: extract
  into `versions/`, place `bin/yx` at install root, write `settings.toml` default version
  (equivalent to rustup's "bootstrap installs manager" decomposition — front door and engine are in
  the same package, no two-step)
- Mirror sources configurable (settings.toml), continuing domestic user considerations (same network
  issues as Z3 download)
- **Version management doesn't break the self-contained invariant**: each version is a complete
  distribution tree, rpath and exe-relative std lookup are self-consistent within the tree, the
  front door only dispatches and doesn't modify the structure

The `.deb` and Inno channels are **system-level flat install** channels (root / Program Files single
version, updated by `apt upgrade` / Control Panel), serving servers, CI, and pure beginner
scenarios; coexistence with the manager relies on PATH ordering (precedent: apt's rustc coexisting
with rustup). All channels share the same artifact tree.

The `.deb` layout reuses the same directory tree: `/usr/lib/yaoxiang/` (the entire distribution
tree: `bin/{yx,yaoxiang-rs,libz3.so}` + `lib/yaoxiang/std/`) + `/usr/bin/yx` symlink pointing to
`/usr/lib/yaoxiang/bin/yx` — `$ORIGIN` is computed based on the resolved **real path** after
symlink, still hitting the same `libz3.so` directory, isomorphic to the extract package structure.
Brand full name stays in the package name and product name (`apt install yaoxiang`, Inno product
name YaoXiang), command surface unified as `yx` — same as Go: package `golang-go`, command `go`. The
apt repo is statically hosted on GitHub Pages (Packages/Release/InRelease metadata GPG-signed,
published by release CI); long-term can apply for Debian/Ubuntu official inclusion (long cycle,
version lag, not a main path).

### Standard Library Directory

The contents of `lib/yaoxiang/std/` all come from repo static files; packaging is **pure copy** with
no runtime generation entry:

| Layer                             | Source                                                       | Nature                                               |
| --------------------------------- | ------------------------------------------------------------ | ---------------------------------------------------- |
| native modules (io/math/…)        | Repo `src/std/interfaces/*.yx` pre-generated interface views | Interface signature views (implementation in binary) |
| .yx layer modules (test/…growing) | Repo `src/std/*.yx` copied verbatim                          | Real source                                          |

The pre-generated views are derived from `StdModule::exports()` (via `generate_all_interfaces()` in
`src/std/gen_interfaces.rs`), **no runtime generation subcommand is set** (2026-09-10 decision: once
packaging is finalized, subcommands are redundant interface surface). Sync adopts the
"generate-and-commit + test gate" model (same as RFC-013 codetable):
`test_committed_interface_files_match_generation` does byte-by-byte comparison of pre-generated
files against current generation, red on drift; fix via bless entry
`cargo test update_committed_interface_files -- --ignored`. Generation logic depends on
crate-internal `StdModule` implementation and can't be moved to build.rs, so the gate is at test
time rather than build time.

**Runtime lookup chain** — `find_std_interface_file` gains an exe-relative lookup level:

1. Project `.yaoxiang/vendor/std/<name>.yx` (project override, current)
2. **Exe's directory `../lib/yaoxiang/std/<name>.yx` (new)**: portable extract, managed install
   (`versions/<ver>/`), deb flat install all hit uniformly
3. `~/.yaoxiang/std/<name>.yx` (global fallback, kept as manual override slot)

The current chain is nearly orphaned (LSP calls don't pass project directory, `package init` writes
to `.yaoxiang/std` which is not in the chain); this fix patches it up, and unifies `package init`
output to `.yaoxiang/vendor/std` (consistent with package manager vendor directory).

**Compilation authority is unchanged**: the `.yx` layer still uses `include_str!` embedding
(RFC-036's "std version strictly bound to binary" invariant is preserved). The distribution
directory positioning is **readable view + LSP parse source**, not compilation input — user edits to
`.yx` in the distribution directory will not be picked up by the compiler (whether to open
Python-style "edit `Lib/` and it takes effect" semantics, see Open Questions).

### Wasm Build

**Kept independent, not migrated into cargo-dist.**

cargo-dist handles "send the compiler to users", wasm is "embed online playground in docs site" —
two completely different deliverables.

| Aspect         | Approach                                           |
| -------------- | -------------------------------------------------- |
| Build tool     | Keep `wasm-pack build`                             |
| CI workflow    | Keep `_build-wasm.yml` as independent job          |
| Trigger        | Same tag push as release, parallel independent job |
| Publish target | `docs/public/wasm/` → GitHub Pages                 |

### npm Publish

| Package                | Content                                     | Status                                                                                                                                                                                                                                   |
| ---------------------- | ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `@yaoxiang/cli`        | Wrapper that downloads distribution package | Deferred: cargo-dist's npm wrapper is also based on flat-artifact assumption, abandoned together with installer; if npm channel is needed, self-write wrapper (download restructured package and extract, same logic as extract install) |
| `@yaoxiang/playground` | wasm library (JS + .wasm)                   | Optional, currently only published in docs                                                                                                                                                                                               |

The two don't conflict, and the names don't conflict either.

### Integration with Existing Release Flow

Current `release.yml`: push main → check-version (`v{version}` tag doesn't exist to allow) → four
paths: build / build-wasm / security / test → release job (tag push + `generate-commit-list.mjs`
generates body with @mentions + upload artifacts).

cargo-dist generated pipeline is tag-driven, with built-in announce/publish, and doesn't include
fmt/clippy/test/audit gates, and release notes format can't carry merge commit changelog. **Direct
wholesale replacement would break the existing release ceremony** (PR → CI all green → bump → merge
commit is changelog).

Integration principle: **trigger and gates stay as is, builds go to `cargo dist build`, publish
stays as is.**

1. check-version / security / test three jobs stay as is (push main triggered, pre-tag gate)
2. After all pass, release job creates and pushes `v{version}` tag (current state unchanged)
3. tag push triggers new `dist-release.yml`: plan job uses dist to compute runner/system dependency
   matrix → `cargo dist build` (5 targets) → `package-dist.sh` restructures per target → Inno Setup
   job (takes Windows restructured package to build wizard, version injected via `/DMyAppVersion=`,
   no recompile) → `_build-wasm.yml` (parallel job)
4. publish job: `generate-commit-list.mjs` generates body (current script reused) → additionally
   upload restructured packages + `.sha256` + `.deb` + wasm + Setup exe; independent `publish-apt`
   job publishes GitHub Pages apt repo metadata (auto-skips when `secrets.APT_GPG_KEY` is not
   configured, doesn't affect other channels)

### Nightly Release

cargo-dist has no native nightly support
([axodotdev#1143](https://github.com/axodotdev/cargo-dist/issues/1143), still open feature request).

Keep current cron + tag override approach, swap build section from `_build-platforms.yml` to
`cargo dist build` — it's essentially a cargo command, directly callable in nightly.yml. No workflow
reuse (envisioned `uses: ./release.yml` is infeasible: the used party needs `workflow_call` trigger,
and cargo-dist workflows are tag-driven with build and publish coupled):

```yaml
# nightly.yml (after migration)
on:
  schedule:
    - cron: '17 22 * * *'
jobs:
  build: # cargo dist build + package-dist.sh (same as release)
  publish: # Current state preserved: move/remove nightly tag → overwrite GitHub Pre-release
```

### cargo-dist Configuration (Landed as dist-workspace.toml)

```toml
[workspace]
members = ["cargo:.", "cargo:tools/yx"]

# Config for 'dist'
[dist]
# Lock dist version (Cargo.toml SemVer syntax)
cargo-dist-version = "0.32.0"
ci = "github"
# Installers all in-house, cargo-dist only does build + archive + checksum
installers = []
targets = ["aarch64-apple-darwin", "aarch64-unknown-linux-gnu", "x86_64-apple-darwin", "x86_64-unknown-linux-gnu", "x86_64-pc-windows-msvc"]
# Generated workflow has been intentionally modified (package-dist.sh restructure + in-house publish segment, RFC-037),
# don't refuse to build due to drift from generate template
allow-dirty = ["ci"]
```

The above is the actual vendor config in the repo (after `cargo dist init` generates and modifies as
needed); `cargo-dist-version` locks 0.32.0, generated workflow vendored into the repo for review,
not fetched at runtime. Build profile is injected by init into root Cargo.toml's `[profile.dist]`
(inherits release, lto=thin), dual binaries land at `target/<triple>/dist/` for the restructure
script to pick up.

### package-dist.sh (Landed)

Based on repo `scripts/release/package-dist.sh`, key points:

- Dual binaries are taken directly from `cargo dist build`'s build output directory
  `target/<triple>/dist/` (profile=dist) — cargo-dist's self-produced flat single-binary archive is
  not the deliverable, same-name restructured package in `target/distrib/` overwrites directly
- Z3 shared library is also taken from the build output directory — `build.rs` already selected the
  corresponding platform's shared library and copied it during linking (`copy_shared_lib`), **single
  source**: the packaging script doesn't know and doesn't need to know Z3 version and platform
  directory naming; Z3 license text ships with the library and is included by packaging (MIT
  distribution obligation), on macOS the dylib's install_name is normalized to `@rpath` and the
  binary is ad-hoc re-signed during packaging
- std directory pure copy: `src/std/interfaces/*.yx` (pre-generated interface views) +
  `src/std/*.yx` (.yx layer real source)
- Attach README/LICENSE; re-archive (Windows zip / others tar.gz; three-level fallback when Git Bash
  has no zip: zip → System32 bsdtar → PowerShell) and recompute `.sha256`
- On Linux with `dpkg-deb` available, additionally call `build-deb.sh` to produce `.deb`
  (`/usr/lib/yaoxiang` flat install tree + `/usr/bin/yx` symlink)

### Deprecated Hand-written CI

Files adjusted after migration:

| File                                     | Lines          | Disposition                                                          |
| ---------------------------------------- | -------------- | -------------------------------------------------------------------- |
| `.github/workflows/_build-platforms.yml` | 254            | Delete (replaced by cargo-dist build matrix)                         |
| `.github/workflows/release.yml`          | 189            | Shrinks to gate + tag push (build/publish moves to dist-release.yml) |
| `.github/workflows/nightly.yml`          | 173            | Build section switched to `cargo dist build`, publish logic kept     |
| `scripts/build/setup.iss`                | ~250           | **Keep and formalize** (Windows wizard)                              |
| **Total reduction**                      | **~600 lines** |                                                                      |

Kept:

- `ci.yml` (daily fmt + clippy + test + MSRV, not part of release flow)
- `_build-wasm.yml` (independent build flow, hooks into dist-release.yml as parallel job)
- `_build-z3-wasm.yml` (wasm-specific Z3)
- `docs-deploy.yml` (docs deployment)

### Acceptance Criteria

"Out of the box" is testable, and the determination of migration completion is not "old and new
artifacts match", but all of the following pass:

- Clean machine (no Rust / no Z3 / no `~/.yaoxiang`) extracts any platform archive, directly runs
  `bin/yaoxiang-rs --version` successfully — no `LD_LIBRARY_PATH` set (rpath works)
- All `lib/yaoxiang/std/*.yx` in the extract directory are readable: native modules as signature
  interface views, `.yx` layer as real source
- Starting LSP on a sample project in the extract directory, std member completion /
  go-to-definition works (exe-relative lookup works)
- After following official guide to extract to `/usr/local` (or `~/.yaoxiang`) and adding to PATH,
  `yx --version` succeeds in any directory
- `apt install yaoxiang` (self-hosted repo) works after install, `apt upgrade` follows versions;
  `/usr/bin/yx` symlink's `$ORIGIN` (resolved by real path) hits `bin/libz3.so`
- `curl ... | sh` and `irm ... | iex` execute in clean environment, `yx` runs, PATH is in place
- `yx toolchain install <ver>` / `default` / `update` works: multi-version coexistence,
  `yx-toolchain.toml` project pin takes precedence over default version, front door dispatches to
  correct version (rpath and std lookup self-consistent within version tree, no "new engine + old
  fmt" combination)
- After portable extract, `yx` falls back to adjacent `yaoxiang-rs`, behavior consistent with
  managed install
- After Inno Setup install, directory structure is complete, PATH takes effect, uninstallable
- Release assets are complete: 5-platform restructured packages + `.sha256` matches actual content
- Release body is `generate-commit-list.mjs` output (merge commit changelog complete)
- nightly artifact is Pre-release, doesn't affect latest formal tag

## Trade-offs

### Advantages

- **Out of the box** — portable extract and use (rpath + same-directory shared library), installer
  lays out complete directory
- **Standard library is readable** — users can directly read std like Python `Lib/` (hard
  requirement achieved)
- **Reduced maintenance cost** — ~600 lines of hand-written build YAML swapped for cargo-dist + ~80
  lines of in-house scripts
- **Cross-platform consistency** — all-platform dynamic linking + same-directory shared library, no
  special cases
- **Two-layer installation** — standard channel has zero new code (Go/Zig model); easy channel
  modeled after Rust, apt / curl / iex / exe four entry points share the same artifact structure
- **Built-in version management** — front door `yx` equivalent to rustup/Go GOTOOLCHAIN, avoiding
  Python/Node's ecosystem fragmentation from pyenv/nvm/pdm after the fact; tool version locking is a
  structural guarantee, not a convention

### Disadvantages and Risks

- **Easy channel maintenance surface** — install.sh / install.ps1 (one-line scripts, rarely
  evolve) + `.deb` and apt repo metadata publishing (release CI automated) + `yx` front door crate
- **Engine rename blast radius** — `yaoxiang` → `yaoxiang-rs` requires one-time migration of CI
  artifact names, Inno, tests, and docs (done within phase five)
- **Learning curve** — team needs to learn cargo-dist configuration
- **cargo-dist upstream risk** — was stalled with Axo in mid-2025, original author revived in
  September same year and continues releasing (0.29 → 0.32+); mitigated by `dist-version` lock +
  vendored generated artifacts for repo review
- **cargo-dist has no native nightly** — nightly release still needs hand-writing

### Relationship with RFC-014b

|                      | RFC-014b                                       | RFC-037                                           |
| -------------------- | ---------------------------------------------- | ------------------------------------------------- |
| **Scope**            | Build and distribution of third-party packages | Packaging and distribution of the compiler itself |
| **Tool**             | `yaoxiang build` / `yaoxiang publish`          | `cargo-dist` + in-house scripts                   |
| **Artifact**         | Third-party package FFI libraries              | Compiler + standard library + toolchain           |
| **Mutual exclusion** | No, complementary                              | No, complementary                                 |

## Alternative Plans

| Plan                                        | Why not chosen                                                                                                                                                                                                                              |
| ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Keep hand-writing CI**                    | Already hand-written ~870 lines, repetitive work, easy to miss DLL                                                                                                                                                                          |
| **Write own packaging tool**                | Don't reinvent the wheel, cargo-dist is mature                                                                                                                                                                                              |
| **Only tar.gz, no installer**               | The only official channel is extract + PATH; Inno only serves Windows wizard habits (decided to keep for domestic users)                                                                                                                    |
| **Docker distribution**                     | Compilers and language toolchains need native binaries, not container scenarios                                                                                                                                                             |
| **Self-hosted Homebrew tap**                | Taps are always community-maintained (homebrew-core), self-hosting is over-engineering; macOS entry for easy channel is curl script                                                                                                         |
| **Independent `yaoxiangup` manager binary** | Direct precedent of rustup, viable; but creates a second user verb, and raises "should package manager/fmt also be independent" symmetry question — front door / engine separation resolves it all at once (rejected 2026-09-09 discussion) |
| **Only self-update, no multi-version**      | Single-version self-update doesn't solve the need for different versions pinned by different projects; Python/Node lack official version management, ecosystem forced to grow pyenv/nvm/pdm — decided to build in (2026-09-09)              |
| **All-static Z3 linking**                   | Decided to reject — directory-based shared library distribution is the natural form for external systems; stuffing into exe is equivalent to putting outside in terms of "must ship with package", but loses replaceability                 |
| **Deprecate Inno Setup**                    | Decided to reject — keep as Windows wizard (additional channel)                                                                                                                                                                             |
| **cargo-dist native installer**             | Flat-binary assumption conflicts with bin/+lib/ structure, install would be missing libraries                                                                                                                                               |
| **Self-maintained WiX/MSI**                 | Cost after losing cargo-dist generator is higher than value already covered by Inno                                                                                                                                                         |

## Implementation Strategy

### Phase 1: Language-side Changes (P0)

1. `build.rs`: all-platform unified dynamic linking + rpath link-arg; `copy_dll()` extended to
   `copy_shared_lib()` (so/dylib/dll)
2. Repo pre-generates native interface views (`src/std/interfaces/`), test gate enforces sync with
   `StdModule::exports()` (gen-std subcommand decided cancelled)
3. `find_std_interface_file` adds exe-relative lookup branch; `package init` output path unified to
   `.yaoxiang/vendor/std`

### Phase 2: cargo-dist Integration (P0)

1. Run `cargo dist init` to generate initial config (`installers = []`, lock dist-version)
2. Write `package-dist.sh` (restructure + .yx source copy + checksum recompute)
3. Create new `dist-release.yml` (tag-driven: dist build → restructure → wasm parallel → in-house
   publish); `release.yml` shrinks to gate + tag push
4. Dual-run old and new pipelines, verify item by item against acceptance criteria

### Phase 3: Old CI Decommission (P1)

1. After verification, delete `_build-platforms.yml`
2. `nightly.yml` build section switched to `cargo dist build`
3. `setup.iss` connects to new artifact structure (Inno formalized; version number injected from
   Cargo.toml, eliminate sed replacement)

### Phase 4: Easy Channel (P2)

1. `install.sh` / `install.ps1` (detect platform → download latest restructured package → extract
   into `versions/` → place `bin/yx` at install root → write `settings.toml` default version → PATH
   prompt/write; lands together with phase five as final state in one go)
2. `.deb` packaging (reuses `package-dist.sh`'s same directory tree + `/usr/bin` symlink) + GitHub
   Pages static apt repo (metadata GPG-signed, release CI publishes)

### Phase 5: Front Door yx and Engine Rename (P2)

1. Current monolith renamed `yaoxiang-rs` (full sweep of CI artifact names, Inno, tests, and docs,
   one-time migration)
2. Add new front door small crate `yx` (workspace member): version resolution, download release
   artifact, tar/zip extraction, settings.toml, dispatch (keep only toolchain/self as verbs,
   transparently pass through the rest, hand-written dispatch without clap to guarantee verbatim
   argument forwarding); version index comes from GitHub Releases latest API (mirror source is
   ghproxy-style prefix concatenation); `yx` command name has been checked for collisions (no common
   command with the same name in mainstream distros/Homebrew, only a niche tool uses yx as alias)
3. `yx toolchain install/default/update/list/uninstall` + `yx-toolchain.toml` project pin + mirror
   sources + `yx self update`
4. Bootstrap script: `curl | sh` / `irm | iex` → download restructured package and install front
   door + default stable in one go (lands together with phase four as final state)

### Phase 6: Optional Follow-up (Non-blocking)

1. winget submission (point to Inno exe, community-maintained, symmetric to homebrew-core model)
2. `.rpm` (for dnf users, isomorphic to `.deb`)
3. Homebrew: community submits after reaching homebrew-core entry threshold
4. npm `@yaoxiang/cli` self-written wrapper (name currently unregistered)

## Open Questions

### Unresolved

- **Should the `.yx` layer provide Python-style semantics where "hand-edits to the distribution
  directory are picked up by the compiler"?** Default no — compilation authority keeps RFC-036
  embedding (std version strictly bound to binary), the distribution directory positioning is
  readable view + LSP parse source. If opened in the future, the version binding invariant needs
  re-examination.

### Closed

The following questions were resolved during design discussion:

- ~~Feasibility of static linking Z3 on Windows?~~ → **No static linking, all-platform dynamic**
  (2026-09-09 review maintained)
- ~~gen-std-interfaces subcommand naming?~~ → **No subcommand** (2026-09-10 decision: once packaging
  is finalized, subcommand surface is redundant; native interface views changed to repo
  pre-generated `src/std/interfaces/` + test gate sync, packaging is pure copy)
- ~~Keep Inno Setup?~~ → **Keep as Windows wizard (additional channel)**
- ~~Should distribution package physically carry standard library source?~~ → **Required** (user
  readability aligned with Python `Lib/`, 2026-09-09 decision)
- ~~cargo-dist native installers (shell/powershell/homebrew/msi/npm)?~~ → **All abandoned**,
  installers in-house (flat assumption conflicts with bin/+lib/)
- ~~Installer strategy?~~ → **Two-layer** (2026-09-09 final decision): standard channel =
  distribution package is the product (Go/Zig model, extract + PATH); easy channel modeled after
  Rust one-line install — Linux `apt` (self-hosted deb repo) / `curl | sh`, Windows `irm | iex` /
  Inno exe. Not doing: MSI, cargo-dist native installer, self-hosted brew tap
- ~~Should version manager be included?~~ → **Required, belongs to the installer system**
  (2026-09-09 decision, overturning same-day earlier "future independent RFC" boundary): motivation
  is Python/Node lacking official version management leading to pyenv/nvm/pdm ecosystem
  fragmentation
- ~~Version manager form: independent binary or subcommand?~~ → **Front door / engine separation**
  (2026-09-09 final decision, three rounds of convergence A→C→name inversion): common command `yx` =
  front door (small binary, built-in toolchain/self verbs), engine `yaoxiang-rs` = current
  `yaoxiang` monolith renamed; directory structure `versions/<ver>/` — version is a first-class
  concept, version directory is distribution package extract root, tools locked together by version
  (old fmt doesn't recognize new syntax; previously envisioned inner `toolchains/` cancelled due to
  redundancy). Precedents: Go `go` front door + GOTOOLCHAIN, rustup proxy dispatch
- ~~cargo-dist extra-artifacts conditional execution?~~ → **Handle with `package-dist.sh` script,
  using shell case branches**
- ~~Standard library interface version compatibility?~~ → **Released with compiler version, in the
  same archive**

## References

- [cargo-dist Official Documentation](https://axodotdev.github.io/cargo-dist/)
- [cargo-dist GitHub](https://github.com/axodotdev/cargo-dist)
- [RFC-014b: Build System and Binary Distribution](../review/014b-build-system.md)
- [cargo-dist nightly feature request](https://github.com/axodotdev/cargo-dist/issues/1143)
- [Z3 Build Configuration — CMakeLists.txt](https://github.com/Z3Prover/z3/blob/master/src/CMakeLists.txt)

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

Use `cargo-dist` (the Rust ecosystem's binary distribution tool) to handle cross-platform build
orchestration, while in-house scripts handle distribution package structure. The core commitments
are two: distribution packages **physically carry the standard library source directory** (users can
read it directly like Python's `Lib/`), and the cross-platform dynamically-linked Z3 shared library
is distributed with the package. The command model is **front door / engine separation**: the common
command `yx` (small front door, with built-in version management — rustup/Go GOTOOLCHAIN
equivalent), the engine `yaoxiang-rs` (renamed from the current monolithic `yaoxiang`), avoiding the
ecosystem fragmentation that Python/Node later had to patch with nvm/pdm. Two-tier installation: the
standard channel aligns with Go/Zig — **the distribution package IS the product**, extract + PATH;
the easy channel is one-line install (Linux `apt` / `curl | sh`, Windows `irm | iex` / Inno exe
wizard). This solves the `libz3.dll` missing problem, makes the standard library invisible to users,
and the repetition of maintaining CI scripts.

## Motivation

### Why is this feature needed?

Users who download YaoXiang should be able to use it **out of the box** without any extra steps; the
standard library should be **directly readable by users**, not hidden as a black box inside the
binary.

### Current Problems

#### Problem 1: Windows users can't run it after downloading

The current Release only uploads `yaoxiang.exe`, but `libz3.dll` is not packaged in. When users
double-click to run on Windows, they get an error:

```
The code execution cannot proceed because libz3.dll was not found.
```

This is a **disruptive bug** — users can't even get past the first step.

#### Problem 2: Release artifacts are only a single-file exe, the standard library is invisible to users

The current state is a triple break:

- Release artifacts are bare binaries, the standard library is not distributed with the release
- The LSP's interface file lookup chain is nearly disconnected: when calling
  `find_std_interface_file`, the project directory is not passed (it only checks the global
  `~/.yaoxiang/std/`, with no flow to populate it); what `package init` writes is `.yaoxiang/std`,
  which is not on the lookup chain
- The standard library source (`.yx` layer) and interface view (native layer) are complete black
  boxes to users

The industrial approach: users open the standard library directory directly to read the source like
reading Python's `Lib/` — **the distribution package physically carrying the std directory is a hard
requirement of this plan** (already decided).

#### Problem 3: Hand-written CI scripts repeatedly maintained

Multiple build pipelines are currently maintained:

| File                      | Responsibility       | Line Count     |
| ------------------------- | -------------------- | -------------- |
| `_build-platforms.yml`    | Cross-platform build | ~255 lines     |
| `release.yml`             | Version release      | ~189 lines     |
| `nightly.yml`             | Daily build          | ~173 lines     |
| `scripts/build/setup.iss` | Inno Setup installer | ~250 lines     |
| **Total**                 |                      | **~870 lines** |

Most is repetitive (install Rust → cache → build → rename → upload), written once for each platform.

#### Problem 4: Inno Setup version number is hardcoded

`MyAppVersion` in `setup.iss` is hardcoded to `0.7.0`, relying on `sed` substitution at build time.
This will eventually fail.

#### Problem 5: Boundary ambiguity with RFC-014b

RFC-014b defines the "YaoXiang package build and distribution mechanism" (i.e., the `[build]` and
`[binaries]` configurations in `yaoxiang.toml`), but **does not cover "how the YaoXiang compiler
itself is released"**. This RFC fills that gap.

## Proposal

### Core Design

cargo-dist only handles the **build orchestration layer**; package structure and installers are all
in-house. Division of responsibilities:

```
cargo-dist responsibilities (build orchestration layer):
  ├── Cross-platform compilation (5 targets)
  └── Generate compressed packages and checksums
  (Native installer and npm wrapper deprecated — their flat-binary assumption conflicts with the bin/+lib/ structure)

build.rs continues to handle:
  └── Z3 download/link (cross-platform dynamic + rpath)

YaoXiang in-house scripts:
  ├── package-dist.sh — Reorganize package structure (bin/ + lib/), include shared libraries,
  │   populate the std directory (repository pre-generated interface views + .yx layer source), recompute checksums
  └── Inno Setup — Windows installation wizard (existing asset; lays out the complete directory structure)

Command model (front door / engine separation):
  ├── yx — front door (new small crate): version resolution + dispatch; only retains toolchain/self verbs, the rest are passed through transparently
  └── yaoxiang-rs — engine (renamed from current monolithic yaoxiang): build/run/package management/fmt/lsp subcommands

Installation (two-tier):
  ├── Standard channel (Go/Zig model): the distribution package IS the product, extract + PATH
  └── Easy channel (Rust model): one-line install + version management (built into the yx front door)
      ├── Linux: apt (self-hosted deb repository, system-level flat install) / curl … | sh (one-click full package install)
      ├── Windows: irm … | iex (one-click full package install) / Inno Setup wizard (existing asset, system-level flat install)
      └── macOS: curl … | sh (homebrew pending homebrew-core community)
```

### Release Directory Structure (decided: physically carry standard library source)

Users must be able to read the standard library directly like reading Python's `Lib/` — the
distribution package having its own std directory is a hard requirement, not a packaging detail. Z3
is the same: as an external system, directory-style shared library distribution is its natural form
— stuffing `.so` into an exe vs. putting it outside for dynamic linking is equivalent in "both must
travel with the distribution package", while the latter preserves replaceability.

The distribution package for each platform is reorganized by `package-dist.sh` after cargo-dist
builds:

```
yaoxiang-{version}-{target}.tar.gz / .zip     (portable and ready to use: run directly from bin/ after extraction)
├── bin/
│   ├── yx                            # front door (or yx.exe)
│   ├── yaoxiang-rs                   # engine (or yaoxiang-rs.exe)
│   └── libz3.so / libz3.dylib / libz3.dll
├── lib/
│   └── yaoxiang/
│       └── std/                      # users can read directly (Python Lib/ model)
│           ├── io.yx                 # native module: repository pre-generated interface view
│           ├── math.yx
│           ├── test.yx               # .yx layer: real source from the repository, copied as-is
│           └── ...
├── README.md
└── LICENSE
```

Installation = extract to any directory + add `bin/` to PATH (Go's `/usr/local/go/bin` model;
extracting to `~/.yaoxiang/` is a common choice). On Windows, the Inno Setup wizard does the same
(default Program Files). For portable extraction, the `yx` front door has no `~/.yaoxiang` state and
falls back to the adjacent `yaoxiang-rs` — same behavior as managed installation; the engine's rpath
and the exe's relative std lookup don't change because of the front door's presence.

### Platform Support

| Platform       | target triple               | Notes               |
| -------------- | --------------------------- | ------------------- |
| Linux x86_64   | `x86_64-unknown-linux-gnu`  | Main platform       |
| Linux ARM64    | `aarch64-unknown-linux-gnu` | Cross-compile on CI |
| macOS x86_64   | `x86_64-apple-darwin`       | Intel Mac           |
| macOS ARM64    | `aarch64-apple-darwin`      | Apple Silicon       |
| Windows x86_64 | `x86_64-pc-windows-msvc`    | Main platform       |

5 targets total. Windows ARM64 is not supported for now (Z3 has no official pre-built ARM64
package).

### Z3 Distribution Strategy

**Cross-platform dynamic linking** (re-confirmed, maintained):

| Platform | Change                     | Artifact      |
| -------- | -------------------------- | ------------- |
| Linux    | **Static → Dynamic**       | `libz3.so`    |
| macOS    | **Static → Dynamic**       | `libz3.dylib` |
| Windows  | Unchanged                  | `libz3.dll`   |
| wasm32   | Unchanged (static linking) | Embedded `.a` |

Reasons:

- **Consistency** — Uniform behavior across three platforms, no more platform-specific exceptions
- **This is an external library, it should be distributed as a shared library**. Python
  (`python3.dll` + `DLLs/lib*.dll`), Node (`node` + `lib/`) all do this
- **Users upgrading Z3 don't need to wait for a compiler version** — just swap a
  `.so`/`.dylib`/`.dll`
- **Smaller binary size** — Z3 isn't small, static linking would bloat the exe by several MB

Dynamic linking has one **necessary companion**: Linux/macOS dynamic linkers don't search the
binary's directory by default, so rpath must be injected, otherwise "extract and use" doesn't work
(Windows searches the exe directory by default, no handling needed). The corresponding `build.rs`
modification:

```rust
// Unified dynamic linking + rpath
fn link_z3(z3_dir: &Path) {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    // Z3 distribution package layout is not uniform, probe both lib/ and bin/ directories
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    // RFC-037: Cross-platform dynamic linking. Shared library is distributed with the package's bin/, users can replace/upgrade Z3 as a whole
    if target_os == "windows" {
        // MSVC import lib is named libz3.lib
        println!("cargo:rustc-link-lib=libz3");
    } else {
        println!("cargo:rustc-link-lib=z3");
        // The dynamic linker doesn't search the binary's directory by default, rpath must be injected for "extract and use" to work
        // (In the distribution package, exe and libz3 are both in bin/; Windows searches the exe directory by default, no handling needed)
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

**"Cross-platform static linking" is not a goal.** This isn't about eliminating special cases — it's
the wrong way to eliminate a reasonable one. Shared libraries are the normal distribution method for
external libraries.

### Installer Support

Comparing the distribution methods of mainstream language toolchains (survey as of 2026-09):

| Language | Official Release             | Official Installation                         | Installer Maintainer            |
| -------- | ---------------------------- | --------------------------------------------- | ------------------------------- |
| Go       | `go/{bin,src,pkg}` tarball   | Official docs ARE "download → extract → PATH" | None (brew/apt is community)    |
| Zig      | `zig/{bin,lib/std}` tarball  | Same as above, no official install script     | None (homebrew-core community)  |
| Node     | `{bin,lib,include}` tarball  | tar + official pkg/msi                        | Team-written                    |
| Rust     | Multi-component tarball      | rustup                                        | Team-written                    |
| Crystal  | `{bin,src,embedded}` tarball | deb/rpm/tar                                   | Team + brew community           |
| Deno/Bun | Single-binary zip            | Official curl scripts                         | Team-written (scripts are tiny) |
| Gleam    | cargo-dist single-binary     | cargo-dist generated scripts                  | cargo-dist                      |

Three patterns:

- **No multi-file toolchain uses a third-party generator for its installer** — cargo-dist's
  installer only fits the single-binary scenario (Gleam can use it precisely because it's a single
  Rust binary with no external dependencies)
- The simplest model is **Go/Zig's "distribution package IS the product"**: the official
  installation guide IS extract + PATH, zero installer code; distribution packages carrying readable
  std source (Go's `src/`, Zig's `lib/std/`, Crystal's `src/`) is the norm
- Those that want curl one-click install (Deno/Bun/rustup) all **write their own scripts** and they
  barely evolve; brew formulas are all community-maintained in homebrew-core, language teams don't
  self-host taps (the Crystal team explicitly says formulas are community)

YaoXiang adopts a two-tier model:

| Channel                                                  | Tier     | Status | Description                                                                                                                             |
| -------------------------------------------------------- | -------- | ------ | --------------------------------------------------------------------------------------------------------------------------------------- |
| zip / tar.gz                                             | Standard | ✅     | Extract and use (rpath + same-directory shared library), official guide is extract + PATH                                               |
| `yx` (front door, built-in version management)           | Easy     | ✅     | rustup/Go GOTOOLCHAIN equivalent: multi-version install/switch/update + project pin                                                     |
| `curl ... \| sh` (install.sh)                            | Easy     | ✅     | Linux / macOS: download reorganized package, install into `versions/`, place `bin/yx` at the install root and write the default version |
| `irm ... \| iex` (install.ps1)                           | Easy     | ✅     | Windows: same logic                                                                                                                     |
| `apt install yaoxiang`                                   | Easy     | ✅     | `.deb` (amd64/arm64) + GitHub Pages static apt repository; system-level flat install, version tracked by `apt upgrade`                  |
| Inno Setup exe                                           | Easy     | ✅     | Windows wizard (existing asset), system-level flat install, lays out the complete bin/+lib/ structure                                   |
| winget / `.rpm` / homebrew-core / npm                    | —        | ⏸      | Optional follow-up: winget and brew-core are community-maintained, rpm is isomorphic to deb                                             |
| MSI / cargo-dist native installer / self-hosted brew tap | —        | ❌     | See "Alternatives"                                                                                                                      |

**The easy channel references Rust, with version management built into the front door.** Rust's
decomposition is "bootstrap script (sh.rustup.rs) → rustup → toolchain packages" — rustup takes care
of multi-version, switching, and updating from the start; Python/Node's official installers didn't
do this layer, and the ecosystem later grew pyenv/nvm/pdm each doing their own thing. YaoXiang
adopts **front door / engine separation** (Go's `go` front door + GOTOOLCHAIN, rustup's proxy
dispatch, both are isomorphic forms): the common command is `yx`, the engine is `yaoxiang-rs` —

```
~/.yaoxiang/
├── bin/yx                  # front door: small binary, version resolution + dispatch (project pin > default > adjacent engine)
├── settings.toml           # default version, mirror source
└── versions/               # version is a first-class concept; <ver>/ IS that version's distribution package extract root
    └── 0.7.14/
        ├── bin/
        │   ├── yaoxiang-rs      # engine: build/run/package management/fmt/lsp subcommands
        │   └── libz3.so
        └── lib/yaoxiang/std/
```

- Install root `~/.yaoxiang/` aligns with industry conventions (pyenv `~/.pyenv`, nvm `~/.nvm`, deno
  `~/.deno`, bun `~/.bun`, volta `~/.volta` all use a single root; rustup's `~/.cargo`+`~/.rustup`
  double root is historical baggage from cargo predating rustup, not emulated), and `~/.yaoxiang` is
  already the codebase's existing namespace (the std global fallback slot); supports the
  `YAOXIANG_HOME` environment variable override (precedent: RUSTUP_HOME / DENO_INSTALL, serves CI
  and container scenarios); on Windows it's `%USERPROFILE%\.yaoxiang`
- **Version directory = distribution package extract root**: `versions/<ver>/` is completely
  isomorphic with portable extraction and deb install trees — installing a version is extracting a
  distribution package — zero structural forking across the three channels
- Command surface (rustup equivalent): `yx toolchain install / default / update / list / uninstall`,
  including `yx self update`; other verbs are transparently passed through to the engine
- **Version locking is a structural guarantee**: tools like fmt evolve with the syntax in the same
  version (old fmt doesn't recognize new syntax), version resolution is done once at the front door,
  the whole set switches together — no "new engine paired with old fmt" combination space; if
  fmt/LSP are split into independent binaries in the future, they also go in the same version's
  `bin/`
- Project-level pin: `yx-toolchain.toml` (precedent: rust-toolchain.toml, follows the command name;
  not in `yaoxiang.toml` — a package manifest shouldn't force the toolchain version on library
  users)
- Bootstrap entry (`curl | sh` / `irm | iex`) installs the latest stable full package in one go:
  extract into `versions/`, place `bin/yx` at the install root, write `settings.toml` default
  version (convergent equivalent of rustup's "bootstrap installs the manager" decomposition — front
  door and engine in the same package, no two-step needed)
- Configurable mirror sources (settings.toml), continuing consideration for users in China (same
  network issue as Z3 downloads)
- **Version management doesn't break the self-contained invariant**: each version is a complete
  distribution tree, rpath and exe's relative std lookup are self-consistent within the tree, the
  front door only dispatches and doesn't modify the structure

The `.deb` and Inno are **system-level flat install** channels (root / Program Files, single
version, tracked by `apt upgrade` / Control Panel), serving servers, CI, and pure-novice scenarios;
coexistence with the manager relies on PATH order (precedent: apt's rustc coexists with rustup). All
channels share the same product tree.

The `.deb` layout reuses the same directory tree: `/usr/lib/yaoxiang/` (the entire distribution
tree: `bin/{yx,yaoxiang-rs,libz3.so}` + `lib/yaoxiang/std/`) + `/usr/bin/yx` symlink pointing to
`/usr/lib/yaoxiang/bin/yx` — `$ORIGIN` is computed from the **resolved real path** after the
symlink, which still hits `libz3.so` in the same directory, isomorphic with the extracted package
structure. The full brand name stays on the package name and product name (`apt install yaoxiang`,
Inno product name YaoXiang), the command surface is unified as `yx` — same as Go: package name
`golang-go`, command `go`. The apt repository is statically hosted on GitHub Pages
(Packages/Release/InRelease metadata GPG signed, published by the release CI); long-term it can
apply for inclusion in Debian/Ubuntu official repos (long cycle, version lag, not the main path).

### Standard Library Directory

The contents of `lib/yaoxiang/std/` all come from the repository's static files, packaging is **pure
copy**, no runtime generation entry point:

| Layer                              | Source                                                            | Nature                                                         |
| ---------------------------------- | ----------------------------------------------------------------- | -------------------------------------------------------------- |
| Native modules (io/math/…)         | Repository `src/std/interfaces/*.yx` pre-generated interface view | Interface signature view (implementation is inside the binary) |
| .yx layer modules (test/… growing) | Repository `src/std/*.yx` copied as-is                            | Real source code                                               |

The pre-generated view is derived from `StdModule::exports()` (`generate_all_interfaces()` in
`src/std/gen_interfaces.rs`), **no runtime generation subcommand is set up** (decided 2026-09-10:
after packaging is finalized, the subcommand is a redundant interface surface). Synchronization
adopts the "generated artifacts committed + test gate" model (same as RFC-013 codetable):
`test_committed_interface_files_match_generation` byte-compares pre-generated files with current
generation, drift means red; healing goes through the bless entry
`cargo test update_committed_interface_files -- --ignored`. The generation logic depends on the
crate-internal `StdModule` implementation and cannot be moved to build.rs, so the gate is at test
time rather than build time.

**Runtime lookup chain** — `find_std_interface_file` adds an exe-relative lookup level:

1. Project `.yaoxiang/vendor/std/<name>.yx` (project override, current)
2. **Exe's directory `../lib/yaoxiang/std/<name>.yx` (new)**: portable extraction, managed
   installation (`versions/<ver>/`), deb flat install all hit this uniformly
3. `~/.yaoxiang/std/<name>.yx` (global fallback, retained as a manual override slot)

The current state of this chain is nearly disconnected (LSP calls don't pass the project directory,
what `package init` writes to `.yaoxiang/std` is not on the chain), this time it's picked up, and
`package init` output is unified to `.yaoxiang/vendor/std` (consistent with the package manager's
vendor directory).

**Compile authority does not change**: the `.yx` layer still goes through `include_str!` embedding
(RFC-036's "std version strictly bound to binary" invariant is maintained). The distribution
directory positioning is **readable view + LSP resolution source**, not compile input — if users
hand-edit `.yx` in the distribution directory, it won't be picked up by the compiler (whether to
open the Python-style "edit `Lib/` and it takes effect" semantics, see open questions).

### Wasm Build

**Remain independent, not migrated into cargo-dist.**

cargo-dist handles "delivering the compiler to users", wasm is "online playground embedded in the
documentation site" — two completely different deliverables.

| Aspect         | Approach                                           |
| -------------- | -------------------------------------------------- |
| Build tool     | Keep `wasm-pack build`                             |
| CI workflow    | Keep `_build-wasm.yml` as independent job          |
| Trigger timing | Same tag push as release, parallel independent job |
| Publish target | `docs/public/wasm/` → GitHub Pages                 |

### npm Publication

| Package                | Contents                                        | Status                                                                                                                                                                                                                                                 |
| ---------------------- | ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `@yaoxiang/cli`        | Wrapper that downloads the distribution package | Deferred: cargo-dist's npm wrapper has the same flat-artifact assumption, deprecated along with the installer; if the npm channel is needed, write your own wrapper (download and extract the reorganized package, same logic as extract installation) |
| `@yaoxiang/playground` | wasm library (JS + .wasm)                       | Optional, currently only published to docs                                                                                                                                                                                                             |

They don't conflict, and the names don't conflict.

### Integration with Existing Release Process

Current `release.yml`: push main → check-version (`v{version}` tag doesn't exist, then pass) → build
/ build-wasm / security / test four paths → release job (push tag + `generate-commit-list.ts`
generates body with @mentions + upload artifacts).

cargo-dist's generated pipeline is tag-driven, includes announce/publish, and doesn't include
fmt/clippy/test/audit gates, and the release notes format can't carry merge commit changelogs.
**Direct overall replacement would break the existing release ceremony** (PR → CI all green → bump →
merge commit IS the changelog).

Integration principle: **triggering and gates stay as-is, build handed to `cargo dist build`,
publish stays as-is.**

1. check-version / security / test together with tag pushing are moved into the `gate` / `security`
   / `test` / `tag` jobs of `dist-release.yml`, triggering stays as push main (**tag-driven is not
   feasible**: tags pushed by `GITHUB_TOKEN` don't trigger other workflows; the old `release.yml`
   could therefore only publish flat binaries itself, already deleted along with
   `_build-platforms.yml`, this file is the single point of release)
2. Build only after the tag is produced: `plan` job computes runner/system dependency matrix via
   dist → `cargo dist build` (5 targets) → `package-dist.sh` reorganizes per target → Inno Setup job
   (takes the Windows reorganized package to build the wizard, `/DMyAppVersion=` injects version, no
   second compilation) → `_build-wasm.yml` (parallel job)
3. publish job: `generate-commit-list.ts` generates body (existing script reused) → upload
   reorganized packages + `.sha256` + `.deb` + wasm + Setup exe, `action-gh-release` self-built
   Release; independent `publish-apt` job publishes GitHub Pages apt repository metadata (auto-skip
   when `secrets.APT_GPG_KEY` is not configured, doesn't affect other channels)
4. Re-release / re-build: `workflow_dispatch` specifies an existing tag (`gate` skips the
   tag-pushing step accordingly)

### Nightly Release

cargo-dist has no native nightly support
([axodotdev#1143](https://github.com/axodotdev/cargo-dist/issues/1143), still an open feature
request).

Keep the existing cron + tag override solution, swap the build section from `_build-platforms.yml`
to `cargo dist build` — it's essentially a cargo command, callable directly in nightly.yml. Don't go
workflow reuse (the imagined `uses: ./release.yml` is not feasible: the reused party needs a
`workflow_call` trigger, and the cargo-dist workflow is tag-driven, build and publish are coupled):

```yaml
# nightly.yml (after migration)
on:
  schedule:
    - cron: '17 22 * * *'
jobs:
  build: # cargo dist build + package-dist.sh (same as the formal version)
  publish: # Kept as-is: push/move nightly tag → override GitHub Pre-release
```

### cargo-dist Configuration (landed as dist-workspace.toml)

```toml
[workspace]
members = ["cargo:.", "cargo:tools/yx"]

# Config for 'dist'
[dist]
# Lock dist version (Cargo.toml SemVer syntax)
cargo-dist-version = "0.32.0"
ci = "github"
# All installers are in-house, cargo-dist only does build + compressed package + checksum
installers = []
targets = ["aarch64-apple-darwin", "aarch64-unknown-linux-gnu", "x86_64-apple-darwin", "x86_64-unknown-linux-gnu", "x86_64-pc-windows-msvc"]
# The generated workflow has been intentionally modified (package-dist.sh reorganization + in-house publish section, RFC-037),
# Not refusing to build due to drift from the generate template
allow-dirty = ["ci"]
```

The above is the actual configuration vendored in the repository (generated by `cargo dist init`
then modified as needed); `cargo-dist-version` is locked at 0.32.0, the generated workflow is
vendored into the repository for review, not fetched at runtime. The build profile is injected by
init into the root Cargo.toml's `[profile.dist]` (inherits release, lto=thin), the dual binaries
land in `target/<triple>/dist/` for the reorganization script to pick up.

### package-dist.sh (Landed)

The authoritative source is the repository's `scripts/release/package-dist.sh`, key points:

- Dual binaries are taken directly from `cargo dist build`'s build output directory
  `target/<triple>/dist/` (profile=dist) — the flat single-binary archive produced by cargo-dist
  itself is not a deliverable, the same-named reorganized package directly overwrites in
  `target/distrib/`
- Z3 shared library is also taken from the build output directory — build.rs has selected the
  appropriate platform shared library and copied it to disk during linking (`copy_shared_lib`),
  **single source**: the packaging script doesn't know and doesn't need to know the Z3 version and
  platform directory naming; the Z3 license text is dropped alongside the library and brought along
  by the packaging (MIT distribution obligation), on the macOS side the packaging normalizes the
  dylib's install_name to `@rpath` and ad-hoc re-signs the binary
- std directory is pure copy: `src/std/interfaces/*.yx` (pre-generated interface view) +
  `src/std/*.yx` (.yx layer real source)
- Attach README/LICENSE; repackage (Windows zip / rest tar.gz; Git Bash without zip, three-level
  fallback: zip → System32 bsdtar → PowerShell) and recompute `.sha256`
- On Linux with `dpkg-deb` available, also call `build-deb.sh` to produce `.deb`
  (`/usr/lib/yaoxiang` flat install tree + `/usr/bin/yx` symlink)

### Deprecated Hand-written CI

Files adjusted after migration:

| File                                     | Line Count     | Disposition                                                                          |
| ---------------------------------------- | -------------- | ------------------------------------------------------------------------------------ |
| `.github/workflows/_build-platforms.yml` | 254            | Delete (cargo-dist build matrix replaces)                                            |
| `.github/workflows/release.yml`          | 189            | Delete (gates and tag pushing merged into dist-release.yml, single point of release) |
| `.github/workflows/nightly.yml`          | 173            | Build section swapped to `cargo dist build`, publish logic kept                      |
| `scripts/build/setup.iss`                | ~250           | **Keep and formalize** (Windows wizard)                                              |
| **Total removed**                        | **~600 lines** |                                                                                      |

Kept:

- `ci.yml` (daily fmt + clippy + test + MSRV, not part of the release process)
- `_build-wasm.yml` (independent build flow, hooked into dist-release.yml parallel job)
- `_build-z3-wasm.yml` (wasm-specific Z3)
- `docs-deploy.yml` (docs deployment)

### Acceptance Criteria

"Out of the box" is testable. The judgment of completed migration is not "new and old products are
consistent", but all of the following pass:

- Clean machine (no Rust / no Z3 / no `~/.yaoxiang`) extracts any platform compressed package,
  directly executing `bin/yaoxiang-rs --version` succeeds — no `LD_LIBRARY_PATH` set (rpath takes
  effect)
- All `lib/yaoxiang/std/*.yx` in the extracted directory are readable: native modules are signature
  interface views, `.yx` layer is real source
- Starting LSP on the example project under the extracted directory, std member completion /
  jump-to-definition works (exe-relative lookup works)
- After extracting to `/usr/local` (or `~/.yaoxiang`) and adding to PATH as per official guide,
  `yx --version` succeeds in any directory
- After `apt install yaoxiang` (self-hosted repo) it can run, `apt upgrade` tracks versions; under
  the `/usr/bin/yx` symlink, the engine's `$ORIGIN` (resolved by real path) hits `bin/libz3.so`
- After executing `curl ... | sh` and `irm ... | iex` in a clean environment, `yx` is runnable, PATH
  is in place
- `yx toolchain install <ver>` / `default` / `update` take effect: multiple versions coexist,
  `yx-toolchain.toml` project pin takes precedence over the default version, the front door
  dispatches to the correct version (rpath and std lookup within the version tree are
  self-consistent, no "new engine paired with old fmt" combination)
- After portable extraction, `yx` falls back to the adjacent `yaoxiang-rs`, behavior consistent with
  managed installation
- After Inno Setup installation, the directory structure is complete, PATH takes effect, can be
  uninstalled
- Release assets are complete: 5 platforms reorganized packages + `.sha256` consistent with actual
  content
- Release body is the output of `generate-commit-list.ts` (merge commit changelog complete)
- nightly artifact is Pre-release, doesn't affect the latest formal tag

## Trade-offs

### Advantages

- **Out of the box** — Portable extract and use (rpath + same-directory shared library), installer
  lays out the complete directory
- **Standard library readable** — Users can read std directly like Python's `Lib/` (hard requirement
  met)
- **Reduced maintenance cost** — ~600 lines of hand-written build YAML replaced with cargo-dist +
  ~80 lines of in-house scripts
- **Cross-platform consistency** — Cross-platform dynamic linking + same-directory shared library,
  no exceptions
- **Two-tier installation** — Standard channel zero new code (Go/Zig model); easy channel references
  Rust, apt / curl / iex / exe four entry points share the same product structure
- **Version management built-in** — Front door `yx` equivalent to rustup/Go GOTOOLCHAIN, avoiding
  the ecosystem fragmentation that Python/Node later had to patch with pyenv/nvm/pdm; tool version
  locking is a structural guarantee, not a convention

### Disadvantages and Risks

- **Easy channel maintenance surface** — install.sh / install.ps1 (one-click scripts, barely
  evolve) + `.deb` and apt repository metadata publishing (release CI automated) + `yx` front door
  crate
- **Engine rename impact** — `yaoxiang` → `yaoxiang-rs` requires one-time migration of CI artifact
  names, Inno, tests, and documentation (completed within phase five)
- **Learning cost** — Team needs to learn cargo-dist configuration
- **cargo-dist upstream risk** — Halted in mid-2025 with Axo, the original author revived it in
  September of the same year and continued releasing versions (0.29 → 0.32+); mitigated by
  `dist-version` lock + generated artifacts vendored into the repository for review
- **cargo-dist has no native nightly** — Nightly release still needs to be hand-written

### Relationship with RFC-014b

|                        | RFC-014b                                   | RFC-037                                           |
| ---------------------- | ------------------------------------------ | ------------------------------------------------- |
| **Scope**              | Third-party package build and distribution | Packaging and distribution of the compiler itself |
| **Tool**               | `yaoxiang build` / `yaoxiang publish`      | `cargo-dist` + in-house scripts                   |
| **Artifact**           | Third-party package FFI libraries          | Compiler + standard library + toolchain           |
| **Mutually exclusive** | No, complementary                          | No, complementary                                 |

## Alternatives

| Plan                                        | Why not chosen                                                                                                                                                                                                                                       |
| ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Continue hand-writing CI**                | Already hand-written ~870 lines, repetitive work, easy to miss DLL                                                                                                                                                                                   |
| **Write our own packaging tool**            | Don't reinvent the wheel, cargo-dist is already mature                                                                                                                                                                                               |
| **Only tar.gz, no installer**               | The only official channel is extract + PATH; Inno only serves Windows wizard habits (already decided to keep for Chinese users)                                                                                                                      |
| **Docker distribution**                     | Compilers and language toolchains need native binaries, not container scenarios                                                                                                                                                                      |
| **Self-hosted Homebrew tap**                | Taps are all community-maintained (homebrew-core), self-hosting is premature; macOS entry for easy channel is curl script                                                                                                                            |
| **Independent `yaoxiangup` manager binary** | Direct rustup precedent, viable; but creates a second user verb, and raises the symmetry question of "should package manager/fmt also be independent" — front door / engine separation resolves this all at once (rejected in 2026-09-09 discussion) |
| **Only do self-update, no multi-version**   | Single-version self-update can't handle multiple projects pinning different versions; Python/Node lacking official version management forced pyenv/nvm/pdm into the ecosystem — decided to be built-in (2026-09-09)                                  |
| **Statically link Z3 everywhere**           | Already rejected — external systems' directory-style shared library distribution is their natural form; stuffing into exe vs. putting outside is equivalent in "both must travel with the package", and the latter loses replaceability              |
| **Deprecate Inno Setup**                    | Already rejected — kept as Windows wizard (additional channel)                                                                                                                                                                                       |
| **cargo-dist native installer**             | Flat-binary assumption conflicts with bin/+lib/ structure, install is missing libraries                                                                                                                                                              |
| **Self-maintain WiX/MSI**                   | After losing the cargo-dist generator, the cost is higher than the value already covered by Inno                                                                                                                                                     |

## Implementation Strategy

### Phase One: Language-side Changes (P0)

1. `build.rs`: Cross-platform unified dynamic linking + rpath link-arg; expand `copy_dll()` to
   `copy_shared_lib()` (so/dylib/dll)
2. Repository pre-generates native interface views (`src/std/interfaces/`), test gate enforces
   synchronization with `StdModule::exports()` (gen-std subcommand already decided to be cancelled)
3. `find_std_interface_file` adds exe-relative lookup branch; `package init` output path unified to
   `.yaoxiang/vendor/std`

### Phase Two: cargo-dist Integration (P0)

1. Run `cargo dist init` to generate initial configuration (`installers = []`, lock dist-version)
2. Write `package-dist.sh` (reorganization + .yx source copy + checksum recompute)
3. `dist-release.yml` carries everything: push main trigger → gate version gate → gates (audit / fmt
   / clippy / test) → tag pushing → dist build → reorganization → wasm parallel → in-house publish +
   apt; `release.yml` and `_build-platforms.yml` deleted
4. Run new and old pipelines in parallel, verify item by item against acceptance criteria

### Phase Three: Old CI Decommission (P1)

1. `_build-platforms.yml` and `release.yml` deleted together (single point of release:
   `dist-release.yml`)
2. `nightly.yml` build section swapped to `cargo dist build`
3. `setup.iss` hooked into new product structure (Inno formalized; version number injected from
   Cargo.toml, sed replacement eliminated)

### Phase Four: Easy Channel (P2)

1. `install.sh` / `install.ps1` (detect platform → download latest reorganized package → extract
   into `versions/` → `bin/yx` at install root → write `settings.toml` default version → PATH
   prompt/write; landed in the same round as phase five, all the way to the final state)
2. `.deb` packaging (reuses the same directory tree from `package-dist.sh` + `/usr/bin` symlink) +
   GitHub Pages static apt repository (metadata GPG signed, published by release CI)

### Phase Five: Front Door yx and Engine Rename (P2)

1. Current monolith renamed `yaoxiang-rs` (CI artifact names, Inno, tests, and documentation all go
   through one pass, one-time migration)
2. New front door small crate `yx` (workspace member): version resolution, download release
   artifacts, tar/zip extraction, settings.toml, dispatch (only retains toolchain/self verbs, the
   rest are transparently passed through, hand-written dispatch without clap to ensure parameters
   are forwarded verbatim); version index from GitHub Releases latest API (mirror source is
   ghproxy-style prefix concatenation); `yx` command name passed name collision check (no commonly
   used command with the same name in mainstream distros/Homebrew, only a niche tool uses yx as an
   alias)
3. `yx toolchain install/default/update/list/uninstall` + `yx-toolchain.toml` project pin + mirror
   source + `yx self update`
4. Bootstrap script: `curl | sh` / `irm | iex` → download reorganized package, install front door
   and default stable in one go (landed in the same round as phase four as the final state)

### Phase Six: Optional Follow-up (None Blocking)

1. winget submission (pointing to Inno exe, community-maintained, symmetrical with homebrew-core
   model)
2. `.rpm` (dnf users, isomorphic to `.deb`)
3. Homebrew: after popularity reaches homebrew-core entry threshold, community submits
4. npm `@yaoxiang/cli` self-written wrapper (name currently not registered)

## Open Questions

### Unresolved

- **Does the `.yx` layer provide Python-style semantics where "hand-editing the distribution
  directory IS picked up by the compiler"?** Default no — compile authority keeps RFC-036 embedding
  (std version strictly bound to binary), distribution directory is positioned as readable view +
  LSP resolution source. If opened in the future, the version binding invariant needs to be
  re-examined.

### Closed

The following questions were resolved during design discussion:

- ~~Feasibility of static linking Z3 on Windows?~~ → **Don't statically link, dynamic everywhere**
  (re-confirmed 2026-09-09)
- ~~gen-std-interfaces subcommand naming?~~ → **No subcommand** (decided 2026-09-10: after normal
  packaging is finalized, subcommand surface is redundant; native interface view is changed to
  repository pre-generated `src/std/interfaces/` + test gate synchronization, packaging is pure
  copy)
- ~~Keep Inno Setup?~~ → **Keep as Windows wizard (additional channel)**
- ~~Does the distribution package structure physically carry the standard library source?~~ →
  **Must** (user readability aligned with Python `Lib/`, decided 2026-09-09)
- ~~cargo-dist native installer (shell/powershell/homebrew/msi/npm)?~~ → **All deprecated**,
  installer is in-house (flat assumption conflicts with bin/+lib/)
- ~~Installer strategy?~~ → **Two-tier** (final decision 2026-09-09): standard channel =
  distribution package IS product (Go/Zig model, extract + PATH); easy channel references Rust
  one-line install — Linux `apt` (self-hosted deb repository) / `curl | sh`, Windows `irm | iex` /
  Inno exe. Not done: MSI, cargo-dist native installer, self-hosted brew tap
- ~~Is the version manager included?~~ → **Must, part of the installer system** (decided 2026-09-09,
  overturning the "long-term independent RFC" boundary of earlier the same day): motivation is
  Python/Node lacking official version management forcing pyenv/nvm/pdm to patch the ecosystem
  fragmentation
- ~~Version manager form: independent binary or subcommand?~~ → **Front door / engine separation**
  (final decision 2026-09-09, three rounds of convergence A→C→naming inversion): the common command
  `yx` = front door (small binary, built-in toolchain/self verbs), the engine `yaoxiang-rs` =
  current `yaoxiang` monolith renamed; directory structure `versions/<ver>/` — version is a
  first-class concept, version directory IS the distribution package extract root, tools lock as a
  whole with the version (old fmt doesn't recognize new syntax; the once-imagined inner
  `toolchains/` was cancelled due to redundancy). Precedent: Go's `go` front door + GOTOOLCHAIN,
  rustup's proxy dispatch
- ~~cargo-dist extra-artifacts conditional execution?~~ → **Handle via `package-dist.sh` script, use
  shell case branches**
- ~~Standard library interface version compatibility?~~ → **Released with the compiler version, in
  the same compressed package**

## References

- [cargo-dist Official Documentation](https://axodotdev.github.io/cargo-dist/)
- [cargo-dist GitHub](https://github.com/axodotdev/cargo-dist)
- [RFC-014b: Build System and Binary Distribution](../review/014b-build-system.md)
- [cargo-dist nightly feature request](https://github.com/axodotdev/cargo-dist/issues/1143)
- [Z3 Build Configuration — CMakeLists.txt](https://github.com/Z3Prover/z3/blob/master/src/CMakeLists.txt)

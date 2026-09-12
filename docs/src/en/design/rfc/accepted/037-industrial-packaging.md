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
orchestration, with our own scripts responsible for the release package structure. Two core
commitments: the release package **physically carries the standard library source directory** (users
can read it directly, just like reading Python's `Lib/`), and the Z3 shared library is dynamically
linked across all platforms and distributed with the package. The command model is **front door /
engine separation**: the common command is `yx` (small front door, with built-in version management
— the rustup/Go GOTOOLCHAIN equivalent), and the engine is `yaoxiang-rs` (the renamed `yaoxiang`
monolith). This avoids the ecosystem fragmentation that Python/Node later had to patch with nvm/pdm.
Two-tier installation: the standard channel aligns with Go/Zig — **the release package is the
product**, extract + PATH; the easy channel is one-line install (Linux `apt` / `curl | sh`, Windows
`irm | iex` / Inno exe wizard). This solves the missing `libz3.dll`, the standard library being
invisible to users, and the repetitive maintenance of CI scripts.

## Motivation

### Why is this feature needed?

Users who download YaoXiang should be able to **use it out of the box** without any additional
steps; the standard library should be **directly readable** by users, not a black box hidden inside
binaries.

### Current Problems

#### Problem 1: Windows users cannot run after downloading

The current Release only uploads `yaoxiang.exe`, but `libz3.dll` is not packaged in. When users
double-click to run on Windows, they get an error:

```
The code execution cannot proceed because libz3.dll was not found.
```

This is a **disruptive bug** — users can't even get past the first step.

#### Problem 2: Release artifacts are only single-file exe, standard library is invisible to users

The current state has three layers of breakage:

- Release artifacts are bare binaries, the standard library is not distributed
- The LSP's interface file lookup chain is nearly disconnected: when calling
  `find_std_interface_file`, the project directory is not passed (only the global `~/.yaoxiang/std/`
  is checked, with no process to populate it); `package init` writes to `.yaoxiang/std`, which is
  not in the lookup chain
- Standard library source (the `.yx` layer) and interface views (the native layer) are completely
  black-boxed from the user

The industrial approach: users can directly open the standard library directory to read the source
code, just like reading Python's `Lib/` — **the release package physically carrying the std
directory is a hard requirement of this plan** (already decided).

#### Problem 3: Manually-written CI scripts require repetitive maintenance

Currently maintaining multiple build pipelines:

| File                      | Responsibility        | Lines          |
| ------------------------- | --------------------- | -------------- |
| `_build-platforms.yml`    | Cross-platform builds | ~255 lines     |
| `release.yml`             | Version release       | ~189 lines     |
| `nightly.yml`             | Daily builds          | ~173 lines     |
| `scripts/build/setup.iss` | Inno Setup installer  | ~250 lines     |
| **Total**                 |                       | **~870 lines** |

Most of it is repetitive (install Rust → cache → build → rename → upload), written once per
platform.

#### Problem 4: Inno Setup version number is hardcoded

`MyAppVersion` is hardcoded to `0.7.0` in `setup.iss`, relying on `sed` to replace at build time.
This will fail eventually.

#### Problem 5: Ambiguous boundary with RFC-014b

RFC-014b defines "YaoXiang package build and distribution mechanism" (i.e., the `[build]` and
`[binaries]` config in `yaoxiang.toml`), but **does not cover "how the YaoXiang compiler itself is
released"**. This RFC fills that gap.

## Proposal

### Core Design

cargo-dist only takes on the **build orchestration layer**; package structure and installers are
entirely our own. Division of responsibilities:

```
cargo-dist responsibilities (build orchestration layer):
  ├── Cross-platform compilation (5 targets)
  └── Generate compressed packages and checksums
  (Native installers and npm wrapper are deprecated — their flat-binary assumption conflicts with the bin/+lib/ structure)

build.rs continues to be responsible for:
  └── Z3 download/linking (dynamic across all platforms + rpath)

YaoXiang's own scripts:
  ├── package-dist.sh — Reorganize package structure (bin/ + lib/), attach shared libraries,
  │   populate std directory (pre-generated interface views from repo + .yx layer source), recompute checksum
  └── Inno Setup — Windows installation wizard (existing asset; lays out complete directory structure)

Command model (front door / engine separation):
  ├── yx — Front door (new small crate): version resolution + dispatch; only retains toolchain/self verbs, the rest pass through
  └── yaoxiang-rs — Engine (renamed from the yaoxiang monolith): compile/run/package management/fmt/lsp subcommands

Installation methods (two-tier):
  ├── Standard channel (Go/Zig model): release package is the product, extract + PATH
  └── Easy channel (Rust model): one-line install + version management (built into the yx front door)
      ├── Linux: apt (self-hosted deb repo, system-level install) / curl … | sh (one-line full package install)
      ├── Windows: irm … | iex (one-line full package install) / Inno Setup wizard (existing asset, system-level install)
      └── macOS: curl … | sh (homebrew left to homebrew-core community)
```

### Release Directory Structure (decided: physically carry standard library source)

Users must be able to read the standard library directly, just like reading Python's `Lib/` — the
release package carrying its own std directory is a hard requirement, not a packaging detail. Z3 is
the same: as an external system, directory-based shared library distribution is its natural form —
stuffing `.so` into the exe versus placing it outside and dynamically linking are equivalent in
terms of "both have to ship with the release package", but the latter preserves replaceability.

For each platform's release package, `package-dist.sh` reorganizes after cargo-dist builds:

```
yaoxiang-{version}-{target}.tar.gz / .zip     (portable and ready: extract and run directly in bin/)
├── bin/
│   ├── yx                            # Front door (or yx.exe)
│   ├── yaoxiang-rs                   # Engine (or yaoxiang-rs.exe)
│   └── libz3.so / libz3.dylib / libz3.dll
├── lib/
│   └── yaoxiang/
│       └── std/                      # Directly readable by users (Python Lib/ model)
│           ├── io.yx                 # native modules: pre-generated interface views from repo
│           ├── math.yx
│           ├── test.yx               # .yx layer: real source copied as-is from repo
│           └── ...
├── README.md
└── LICENSE
```

Install = extract to any directory + add `bin/` to PATH (Go's `/usr/local/go/bin` model; extracting
to `~/.yaoxiang/` is a common choice). On Windows, the Inno Setup wizard does the same thing
(default Program Files). When extracting portably, the `yx` front door has no `~/.yaoxiang` state
and falls back to the adjacent `yaoxiang-rs` — consistent with managed install behavior; the
engine's rpath and exe-relative std lookup are not changed by the front door's existence.

### Platform Support

| Platform       | target triple               | Notes               |
| -------------- | --------------------------- | ------------------- |
| Linux x86_64   | `x86_64-unknown-linux-gnu`  | Primary platform    |
| Linux ARM64    | `aarch64-unknown-linux-gnu` | Cross-compile on CI |
| macOS x86_64   | `x86_64-apple-darwin`       | Intel Mac           |
| macOS ARM64    | `aarch64-apple-darwin`      | Apple Silicon       |
| Windows x86_64 | `x86_64-pc-windows-msvc`    | Primary platform    |

Total 5 targets. Windows ARM64 is not supported for now (Z3 has no official pre-compiled ARM64
package).

### Z3 Distribution Strategy

**Dynamic linking across all platforms** (decision maintained after review):

| Platform | Change                        | Artifact      |
| -------- | ----------------------------- | ------------- |
| Linux    | **Static → Dynamic**          | `libz3.so`    |
| macOS    | **Static → Dynamic**          | `libz3.dylib` |
| Windows  | No change                     | `libz3.dll`   |
| wasm32   | No change (statically linked) | Embedded `.a` |

Rationale:

- **Consistency** — Uniform behavior across three platforms, no more special cases
- **This is an external library, it should be distributed as a shared library**. Python
  (`python3.dll`+`DLLs/lib*.dll`), Node (`node`+`lib/`) all do this
- **Users can upgrade Z3 without waiting for a compiler version** — just swap out a
  `.so`/`.dylib`/`.dll`
- **Smaller binary size** — Z3 is not small; static linking will bloat the exe by several MB

Dynamic linking requires a **necessary companion**: Linux/macOS dynamic linkers don't search the
binary's directory by default, rpath must be injected, otherwise "extract and use" doesn't hold
(Windows searches the exe directory by default, no handling needed). The corresponding `build.rs`
modification:

```rust
// Unified dynamic linking + rpath
fn link_z3(z3_dir: &Path) {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    // Z3 release package layout is not uniform, lib/bin dual directory probe
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    // RFC-037: Dynamic linking across all platforms. Shared library distributed with bin/ in release package, users can replace and upgrade Z3 as a whole
    if target_os == "windows" {
        // MSVC import lib named libz3.lib
        println!("cargo:rustc-link-lib=libz3");
    } else {
        println!("cargo:rustc-link-lib=z3");
        // Dynamic linker doesn't search the binary's directory by default, rpath must be injected for "extract and use" to work
        // (Exe and libz3 are in the same bin/ in the release package; Windows searches exe directory by default, no handling needed)
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

**"Static linking across all platforms" is not a goal.** This isn't about eliminating special cases,
it's about eliminating a reasonable case in the wrong way. Shared libraries are the normal
distribution form for external libraries.

### Installer Support

Comparing mainstream language toolchain distribution methods (research as of 2026-09):

| Language | Official Distribution        | Official Installation Method                  | Installer Maintainer                   |
| -------- | ---------------------------- | --------------------------------------------- | -------------------------------------- |
| Go       | `go/{bin,src,pkg}` tarball   | Official docs are "download → extract → PATH" | None (brew/apt is community)           |
| Zig      | `zig/{bin,lib/std}` tarball  | Same as above, no official install script     | None (homebrew-core community)         |
| Node     | `{bin,lib,include}` tarball  | tar + official pkg/msi                        | Team self-written                      |
| Rust     | Multi-component tarball      | rustup                                        | Team self-written                      |
| Crystal  | `{bin,src,embedded}` tarball | deb/rpm/tar                                   | Team + brew community                  |
| Deno/Bun | Single binary zip            | Official curl script                          | Team self-written (very small scripts) |
| Gleam    | cargo-dist single binary     | cargo-dist generated scripts                  | cargo-dist                             |

Three rules:

- **No multi-file toolchain uses third-party generators for installers** — cargo-dist's installer
  only fits the single-binary scenario (Gleam can use it precisely because it's a single Rust binary
  with no external dependencies)
- The simplest model is **Go/Zig's "release package is the product"**: official install instructions
  are just extract + PATH, zero installer code; release packages carrying readable std source (Go's
  `src/`, Zig's `lib/std/`, Crystal's `src/`) is the norm
- Those wanting curl one-line install (Deno/Bun/rustup) are all **self-written scripts** that barely
  evolve; brew formulas are uniformly community-maintained in homebrew-core, language teams don't
  self-build taps (Crystal team explicitly said formulas are community)

YaoXiang adopts a two-tier model:

| Channel                                                 | Tier     | Status | Description                                                                                                             |
| ------------------------------------------------------- | -------- | ------ | ----------------------------------------------------------------------------------------------------------------------- |
| zip / tar.gz                                            | Standard | ✅     | Extract and use (rpath + same-directory shared lib), extract + PATH is the official guide                               |
| `yx` (front door, built-in version management)          | Easy     | ✅     | rustup/Go GOTOOLCHAIN equivalent: multi-version install/switch/update + project pin                                     |
| `curl ... \| sh` (install.sh)                           | Easy     | ✅     | Linux / macOS: download reorganized package into `versions/`, `bin/yx` placed in install root and write default version |
| `irm ... \| iex` (install.ps1)                          | Easy     | ✅     | Windows: same logic                                                                                                     |
| `apt install yaoxiang`                                  | Easy     | ✅     | `.deb` (amd64/arm64) + GitHub Pages static apt repo; system-level install, follows versions with `apt upgrade`          |
| Inno Setup exe                                          | Easy     | ✅     | Windows wizard (existing asset), system-level install, lays out complete bin/+lib/ structure                            |
| winget / `.rpm` / homebrew-core / npm                   | —        | ⏸      | Optional follow-up: winget and brew-core are community-maintained, rpm and deb are isomorphic                           |
| MSI / cargo-dist native installer / self-built brew tap | —        | ❌     | See "Alternatives"                                                                                                      |

**The easy channel references Rust, with version management built into the front door.** Rust's
decomposition is "bootstrap script (sh.rustup.rs) → rustup → toolchain package" — rustup manages
multi-version, switching, and updating from the start; Python/Node official installers didn't do
this layer, so the ecosystem later grew pyenv/nvm/pdm each going their own way. YaoXiang adopts
**front door / engine separation** (Go's `go` front door + GOTOOLCHAIN, rustup's proxy dispatch —
isomorphic forms of both): the common command is `yx`, the engine is `yaoxiang-rs` —

```
~/.yaoxiang/
├── bin/yx                  # Front door: small binary, version resolution + dispatch (project pin > default > adjacent engine)
├── settings.toml           # Default version, mirror source
└── versions/               # Version is a first-class concept; <ver>/ is that version's release package extraction root
    └── 0.7.14/
        ├── bin/
        │   ├── yaoxiang-rs      # Engine: compile/run/package management/fmt/lsp subcommands
        │   └── libz3.so
        └── lib/yaoxiang/std/
```

- Install root `~/.yaoxiang/` aligns with industry convention (pyenv `~/.pyenv`, nvm `~/.nvm`, deno
  `~/.deno`, bun `~/.bun`, volta `~/.volta` are all single-root; rustup's `~/.cargo`+`~/.rustup`
  dual-root is a historical artifact from cargo predating rustup, not followed), and `~/.yaoxiang`
  is already an existing namespace in the codebase (std global fallback slot); supports
  `YAOXIANG_HOME` environment variable override (precedent: RUSTUP_HOME / DENO_INSTALL, serving CI
  and container scenarios); Windows is `%USERPROFILE%\.yaoxiang`
- **Version directory = release package extraction root**: `versions/<ver>/` is completely
  isomorphic with portable extraction and deb install tree — installing a version is extracting a
  release package — three channels with zero structural fork
- Command surface (rustup equivalent): `yx toolchain install / default / update / list / uninstall`,
  including `yx self update`; other verbs pass through to the engine as-is
- **Version locking is a structural guarantee**: tools like fmt evolve with the same version as the
  syntax (old fmt doesn't recognize new syntax), version resolution is done once at the front door,
  whole set switches — no "new engine paired with old fmt" combination space; if fmt/LSP are split
  into independent binaries in the future, they also live in the same version's `bin/`
- Project-level pin: `yx-toolchain.toml` (precedent: rust-toolchain.toml, following command name;
  not in `yaoxiang.toml` — package manifest shouldn't impose toolchain version on library consumers)
- Bootstrap entry (`curl | sh` / `irm | iex`) installs the latest stable full package in one go:
  extract into `versions/`, `bin/yx` placed in install root, write `settings.toml` default version
  (equivalent convergence of rustup "bootstrap installs manager" decomposition — front door and
  engine in the same package, no two-step required)
- Mirror source configurable (settings.toml), continuing the consideration for domestic users (same
  network issues as Z3 download)
- **Version management doesn't break the self-contained invariant**: each version is a complete
  release tree, rpath and exe-relative std lookup are self-consistent within the tree, the front
  door only does dispatch and doesn't change structure

`.deb` and Inno are **system-level install** channels (root / Program Files single version, followed
up by `apt upgrade` / control panel), serving server, CI, and pure beginner scenarios; coexistence
with manager relies on PATH order (precedent: apt's rustc and rustup coexist). All channels share
the same product tree.

`.deb` layout reuses the same directory tree: `/usr/lib/yaoxiang/` (complete release tree:
`bin/{yx,yaoxiang-rs,libz3.so}` + `lib/yaoxiang/std/`) + `/usr/bin/yx` symlink pointing to
`/usr/lib/yaoxiang/bin/yx` — `$ORIGIN` is computed by the resolved **real path**, and the symlink
still hits the same directory's `libz3.so`, isomorphic with the extracted package structure. The
full brand name stays in the package name and product name (`apt install yaoxiang`, Inno product
name YaoXiang), command surface unified to `yx` — same as Go: package name `golang-go`, command
`go`. apt repo is statically hosted on GitHub Pages (Packages/Release/InRelease metadata GPG-signed,
published by release CI); in the long term can apply for Debian/Ubuntu official inclusion (long
cycle, version lag, not the main path).

### Standard Library Directory

The content of `lib/yaoxiang/std/` all comes from repo static files, packaging is **pure copy**, no
runtime generation entry:

| Layer                              | Source                                                     | Nature                                                  |
| ---------------------------------- | ---------------------------------------------------------- | ------------------------------------------------------- |
| native modules (io/math/…)         | Pre-generated interface views in `src/std/interfaces/*.yx` | Interface signature view (implementation inside binary) |
| .yx layer modules (test/… growing) | Real source in `src/std/*.yx` copied as-is                 | Real source code                                        |

Pre-generated views are derived from `StdModule::exports()` (`generate_all_interfaces()` in
`src/std/gen_interfaces.rs`), **no runtime generation subcommand is set** (2026-09-10 decision:
after packaging matures, the subcommand is redundant interface surface). Synchronization uses the
"generated artifacts committed + test gate" pattern (same as RFC-013 codetable):
`test_committed_interface_files_match_generation` byte-by-byte compares pre-generated files with
current generation, any drift is red; healing goes through the bless entry
`cargo test update_committed_interface_files -- --ignored`. Generation logic depends on the crate's
internal `StdModule` implementation and cannot be pushed down to build.rs, so the gate point is at
test time, not build time.

**Runtime lookup chain** — `find_std_interface_file` adds an exe-relative lookup:

1. Project `.yaoxiang/vendor/std/<name>.yx` (project override, current)
2. **Exe directory `../lib/yaoxiang/std/<name>.yx` (new)**: portable extraction, managed install
   (`versions/<ver>/`), deb flat install all uniformly hit
3. `~/.yaoxiang/std/<name>.yx` (global fallback, kept as manual override slot)

Currently this chain is nearly disconnected (LSP calls don't pass project directory, `package init`
writes `.yaoxiang/std` which is not in the chain), this work incidentally picks this up, and unifies
`package init` output to `.yaoxiang/vendor/std` (consistent with package manager's vendor
directory).

**Compile authority doesn't change**: the `.yx` layer still goes through `include_str!` embedding
(RFC-036's "std version strictly bound to binary" invariant preserved). Release directory
positioning is **readable view + LSP resolution source**, not compile input — user hand-edits to
`.yx` in the release directory won't be adopted by the compiler (whether to open up Python-style
"edit `Lib/` and it takes effect" semantics, see open questions).

### Wasm Build

**Kept independent, not migrated into cargo-dist.**

cargo-dist manages "sending the compiler to users", wasm is "embedding online playground into
documentation website" — two completely different deliverables.

| Aspect         | Approach                                        |
| -------------- | ----------------------------------------------- |
| Build tool     | Keep `wasm-pack build`                          |
| CI workflow    | Keep `_build-wasm.yml` as independent job       |
| Trigger        | Same release tag push, parallel independent job |
| Publish target | `docs/public/wasm/` → GitHub Pages              |

### npm Publishing

| Package                | Content                                | Status                                                                                                                                                                                                                                     |
| ---------------------- | -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `@yaoxiang/cli`        | Wrapper that downloads release package | Postponed: cargo-dist's npm wrapper is also based on the flat artifact assumption, deprecated along with installer; if npm channel is needed, self-write wrapper (download reorganized package and extract, same logic as extract install) |
| `@yaoxiang/playground` | wasm library (JS + .wasm)              | Optional, currently only published to docs                                                                                                                                                                                                 |

The two don't conflict, and neither do the names.

### Integration with Existing Release Process

Current `release.yml`: push main → check-version (`v{version}` tag doesn't exist to allow pass) →
build / build-wasm / security / test four routes → release job (tag push +
`generate-commit-list.mjs` generates body with @mentions + upload artifacts).

The pipeline generated by cargo-dist is tag-driven, comes with announce/publish, and doesn't include
fmt/clippy/test/audit gates, release notes format also can't carry merge commit changelog. **Direct
overall replacement would shatter the existing release ceremony** (PR → CI all green → bump → merge
commit as changelog).

Integration principle: **Trigger and gates keep current state, builds are handed to
`cargo dist build`, publishing keeps current state.**

1. check-version / security / test three jobs keep current state (push main trigger, pre-tag gates)
2. After all pass, the release job creates and pushes the `v{version}` tag (current state unchanged)
3. tag push triggers new `dist-release.yml`: plan job computes runner/system dependency matrix from
   dist → `cargo dist build` (5 targets) → `package-dist.sh` reorganizes per target → Inno Setup job
   (consume Windows reorganized package to build wizard, inject version via `/DMyAppVersion=`, no
   second compilation) → `_build-wasm.yml` (parallel job)
4. publish job: `generate-commit-list.mjs` generates body (current script reused) → additionally
   upload reorganized package + `.sha256` + `.deb` + wasm + Setup exe; independent `publish-apt` job
   publishes GitHub Pages apt repo metadata (automatically skipped when `secrets.APT_GPG_KEY` is not
   configured, doesn't affect other channels)

### Nightly Release

cargo-dist has no native nightly support
([axodotdev#1143](https://github.com/axodotdev/cargo-dist/issues/1143), still an open feature
request).

Keep the current cron + tag override solution, swap the build part from `_build-platforms.yml` to
`cargo dist build` — it's essentially a cargo command, can be called directly in nightly.yml. Don't
go through workflow reuse (the assumed `uses: ./release.yml` is not feasible: the reused party needs
`workflow_call` trigger, and cargo-dist workflow is tag-driven, with build and publish coupled):

```yaml
# nightly.yml (after migration)
on:
  schedule:
    - cron: '17 22 * * *'
jobs:
  build: # cargo dist build + package-dist.sh (same as release)
  publish: # Current state preserved: create/move nightly tag → overwrite GitHub Pre-release
```

### cargo-dist Configuration (committed as dist-workspace.toml)

```toml
[workspace]
members = ["cargo:.", "cargo:tools/yx"]

# Config for 'dist'
[dist]
# Lock dist version (Cargo.toml SemVer syntax)
cargo-dist-version = "0.32.0"
ci = "github"
# All installers self-built, cargo-dist only does build + compressed package + checksum
installers = []
targets = ["aarch64-apple-darwin", "aarch64-unknown-linux-gnu", "x86_64-apple-darwin", "x86_64-unknown-linux-gnu", "x86_64-pc-windows-msvc"]
# Generated workflow has been intentionally modified (package-dist.sh reorganize + self-built publish section, RFC-037),
# not refusing to build due to drift from generate template
allow-dirty = ["ci"]
```

The above is the actual config vendored in the repo (after `cargo dist init` generates and is
modified as needed); `cargo-dist-version` locked at 0.32.0, generated workflow vendored into repo
for review, not fetched at runtime. Build profile is injected by init into root Cargo.toml's
`[profile.dist]` (inherits release, lto=thin), dual-binary lands in `target/<triple>/dist/` for the
reorganize script to pick up.

### package-dist.sh (committed)

Subject to the repo's `scripts/release/package-dist.sh`, key points:

- Dual binary taken directly from `cargo dist build`'s build output directory
  `target/<triple>/dist/` (profile=dist) — cargo-dist's self-produced flat single-binary archive is
  not the deliverable, same-name reorganized package directly overwrites in `target/distrib/`
- Z3 shared library copied from `.z3/z3-<ver>-<tag>/` (lib → bin dual directory probe) into `bin/`;
  `<tag>` mapping aligned with `build.rs::detect_target()`'s release package naming (maintained in
  two places, see risks)
- std directory pure copy: `src/std/interfaces/*.yx` (pre-generated interface views) +
  `src/std/*.yx` (.yx layer real source)
- Attached README/LICENSE; re-package (Windows zip / others tar.gz; three-level fallback when Git
  Bash has no zip: zip → System32 bsdtar → PowerShell) and recompute `.sha256`
- Linux with `dpkg-deb` available also calls `build-deb.sh` to produce `.deb` (`/usr/lib/yaoxiang`
  flat install tree + `/usr/bin/yx` symlink)

### Deprecated Manually-Written CI

Files adjusted after migration completes:

| File                                     | Lines          | Disposition                                                            |
| ---------------------------------------- | -------------- | ---------------------------------------------------------------------- |
| `.github/workflows/_build-platforms.yml` | 254            | Delete (cargo-dist build matrix replaces)                              |
| `.github/workflows/release.yml`          | 189            | Shrinks to gates + tag push (build/publish moved to dist-release.yml)  |
| `.github/workflows/nightly.yml`          | 173            | Build section replaced with `cargo dist build`, publish logic retained |
| `scripts/build/setup.iss`                | ~250           | **Retained and formalized** (Windows wizard)                           |
| **Total reduction**                      | **~600 lines** |                                                                        |

Retained:

- `ci.yml` (daily fmt + clippy + test + MSRV, not part of release process)
- `_build-wasm.yml` (independent build flow, hooked into dist-release.yml parallel job)
- `_build-z3-wasm.yml` (wasm-specific Z3)
- `docs-deploy.yml` (documentation deploy)

### Acceptance Criteria

"Out of the box" is testable, the migration completion criterion is not "old and new products
match", but that all of the following pass:

- Clean machine (no Rust / no Z3 / no `~/.yaoxiang`) extracts any platform's compressed package,
  directly executes `bin/yaoxiang-rs --version` successfully — no `LD_LIBRARY_PATH` needed (rpath
  works)
- All `lib/yaoxiang/std/*.yx` in the extracted directory are readable: native modules are signature
  interface views, .yx layer is real source
- Start LSP on a sample project in the extracted directory, std member completion / go-to-definition
  works (exe-relative lookup works)
- After extracting to `/usr/local` (or `~/.yaoxiang`) per official guide and adding to PATH,
  `yx --version` succeeds in any directory
- `apt install yaoxiang` (self-built repo) runs afterward, `apt upgrade` follows versions; under
  `/usr/bin/yx` symlink the engine's `$ORIGIN` (resolved by real path) hits `bin/libz3.so`
- `curl ... | sh` and `irm ... | iex` execute in clean environment, `yx` runs, PATH is in place
- `yx toolchain install <ver>` / `default` / `update` work: multi-version coexistence,
  `yx-toolchain.toml` project pin takes precedence over default version, front door dispatches to
  correct version (rpath and std lookup self-consistent within version tree, no "new engine paired
  with old fmt" combinations)
- After portable extraction, `yx` falls back to adjacent `yaoxiang-rs`, behavior consistent with
  managed install
- After Inno Setup install, directory structure is complete, PATH works, uninstallable
- Release assets complete: 5 platform reorganized packages + `.sha256` consistent with actual
  content
- Release body is `generate-commit-list.mjs` output (merge commit changelog complete)
- nightly product is Pre-release, doesn't affect latest official tag

## Trade-offs

### Advantages

- **Out of the box** — Portable extract and use (rpath + same-directory shared lib), installer lays
  out complete directory
- **Readable standard library** — Users can read std directly like Python's `Lib/` (hard requirement
  met)
- **Reduced maintenance cost** — ~600 lines of manually-written build YAML replaced with
  cargo-dist + ~80 lines of own scripts
- **Cross-platform consistency** — Dynamic linking across all platforms + same-directory shared lib,
  no special cases
- **Two-tier installation** — Standard channel with zero new code (Go/Zig model); easy channel
  references Rust, apt / curl / iex / exe four entry points share the same product structure
- **Built-in version management** — Front door `yx` equivalent to rustup/Go GOTOOLCHAIN, avoids
  Python/Node's post-hoc pyenv/nvm/pdm ecosystem fragmentation; tool version locking is structural
  guarantee, not convention

### Disadvantages and Risks

- **Easy channel maintenance surface** — install.sh / install.ps1 (one-line scripts, barely
  evolve) + `.deb` and apt repo metadata publishing (release CI automated) + `yx` front door crate
- **Engine rename impact surface** — `yaoxiang` → `yaoxiang-rs` requires one-time migration of CI
  artifact names, Inno, tests, and documentation (completed within phase five)
- **Learning curve** — Team needs to learn cargo-dist configuration
- **cargo-dist upstream risk** — Paused with Axo in mid-2025, original author revived in September
  same year and continues releasing versions (0.29 → 0.32+); mitigated by `dist-version` lock +
  generated artifacts vendored in repo for review
- **cargo-dist has no native nightly** — Nightly release part still needs to be manually written
- **Z3 directory naming maintained in two places** — `package-dist.sh` and
  `build.rs::detect_target()` need to stay in sync (should converge to single source)

### Relationship with RFC-014b

|                        | RFC-014b                                   | RFC-037                                    |
| ---------------------- | ------------------------------------------ | ------------------------------------------ |
| **Scope**              | Third-party package build and distribution | Compiler itself packaging and distribution |
| **Tools**              | `yaoxiang build` / `yaoxiang publish`      | `cargo-dist` + own scripts                 |
| **Artifacts**          | Third-party package FFI libraries          | Compiler + standard library + toolchain    |
| **Mutually exclusive** | No, complementary                          | No, complementary                          |

## Alternatives

| Plan                                        | Why not chosen                                                                                                                                                                                                                               |
| ------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Continue manually-written CI**            | Already manually written ~870 lines, repetitive work, easy to miss DLL                                                                                                                                                                       |
| **Write our own packaging tool**            | Don't reinvent the wheel, cargo-dist is mature                                                                                                                                                                                               |
| **Only use tar.gz without installer**       | Only official channel is extract + PATH; Inno only serves Windows wizard habit (already decided to retain for domestic users)                                                                                                                |
| **Docker distribution**                     | Compilers and language toolchains need native binaries, not container scenarios                                                                                                                                                              |
| **Self-built Homebrew tap**                 | Taps are uniformly community-maintained (homebrew-core), self-building is ahead of need; macOS easy channel entry is curl script                                                                                                             |
| **Independent `yaoxiangup` manager binary** | rustup original precedent, feasible; but creates a second user verb, and raises the symmetry question of "should package manager/fmt also be independent" — front door / engine separation resolves both (rejected in 2026-09-09 discussion) |
| **Only self-update, no multi-version**      | Single-version self-update doesn't solve multi-project pinning of different versions; Python/Node lack official version management, ecosystem forced to grow pyenv/nvm/pdm — built-in decided (2026-09-09)                                   |
| **All-static Z3 linking**                   | Rejected in decision — external system directory-based shared library distribution is its natural form; stuffing into exe vs placing outside is equivalent in "both have to ship with package", but loses replaceability                     |
| **Deprecate Inno Setup**                    | Rejected in decision — retained as Windows wizard (additional channel)                                                                                                                                                                       |
| **cargo-dist native installer**             | Flat binary assumption conflicts with bin/+lib/ structure, install result lacks libraries                                                                                                                                                    |
| **Self-maintained WiX/MSI**                 | Without cargo-dist generator, cost exceeds value already covered by Inno                                                                                                                                                                     |

## Implementation Strategy

### Phase One: Language-side Changes (P0)

1. `build.rs`: Unified dynamic linking across all platforms + rpath link-arg; `copy_dll()` extended
   to `copy_shared_lib()` (so/dylib/dll)
2. Repo pre-generates native interface views (`src/std/interfaces/`), test gate enforces sync with
   `StdModule::exports()` (gen-std subcommand already decided cancelled)
3. `find_std_interface_file` adds exe-relative lookup branch; `package init` output path unified to
   `.yaoxiang/vendor/std`

### Phase Two: cargo-dist Integration (P0)

1. Run `cargo dist init` to generate initial config (`installers = []`, lock dist-version)
2. Write `package-dist.sh` (reorganize + .yx source copy + checksum recompute)
3. New `dist-release.yml` (tag-driven: dist build → reorganize → wasm parallel → own publish);
   `release.yml` shrinks to gates + tag push
4. Dual-run old and new pipelines, verify each item against acceptance criteria

### Phase Three: Old CI Decommission (P1)

1. Delete `_build-platforms.yml` after verification
2. `nightly.yml` build section replaced with `cargo dist build`
3. `setup.iss` adopts new product structure (Inno formalized; version number injected from
   Cargo.toml, eliminate sed replacement)

### Phase Four: Easy Channel (P2)

1. `install.sh` / `install.ps1` (detect platform → download latest reorganized package → extract
   into `versions/` → `bin/yx` placed in install root → write `settings.toml` default version → PATH
   prompt/write; land in same round as phase five, one-step to final state)
2. `.deb` packaging (reuse same directory tree as `package-dist.sh` + `/usr/bin` symlink) + GitHub
   Pages static apt repo (metadata GPG-signed, release CI publishes)

### Phase Five: Front Door yx and Engine Rename (P2)

1. Rename current monolith to `yaoxiang-rs` (CI artifact name, Inno, tests and documentation all
   migrated in one pass)
2. New front door small crate `yx` (workspace member): version resolution, download release
   artifacts, tar/zip extraction, settings.toml, dispatch (only retain toolchain/self verbs, rest
   pass through, hand-written dispatch to guarantee verbatim argument forwarding without clap);
   version index from GitHub Releases latest API (mirror source as ghproxy-style prefix concat);
   `yx` command name collision-checked (no common command with this name in mainstream
   distros/Homebrew, only a niche tool uses yx as alias)
3. `yx toolchain install/default/update/list/uninstall` + `yx-toolchain.toml` project pin + mirror
   source + `yx self update`
4. Bootstrap script: `curl | sh` / `irm | iex` → download reorganized package to install front door
   and default stable in one go (lands in same round as phase four as final state)

### Phase Six: Optional Follow-up (non-blocking)

1. winget submission (point to Inno exe, community-maintained, symmetric to homebrew-core model)
2. `.rpm` (for dnf users, isomorphic with `.deb`)
3. Homebrew: submit by community after reaching homebrew-core inclusion threshold
4. npm `@yaoxiang/cli` self-written wrapper (name not currently registered)

## Open Questions

### Unresolved

- **Should the .yx layer provide Python-style semantics where hand-edits to the release directory
  are adopted at compile time?** Default no — compile authority maintains RFC-036 embedding (std
  version strictly bound to binary), release directory positioned as readable view + LSP resolution
  source. If opened up in the future, the version-binding invariant needs to be revisited.

### Closed

The following questions were resolved during design discussion:

- ~~Z3 static linking feasibility on Windows?~~ → **No static linking, all-platform dynamic**
  (2026-09-09 review maintained)
- ~~gen-std-interfaces subcommand naming?~~ → **No subcommand** (2026-09-10 decision: after normal
  packaging matures, subcommand surface is redundant; native interface view changed to repo
  pre-generated `src/std/interfaces/` + test gate sync, packaging is pure copy)
- ~~Retain Inno Setup?~~ → **Retained as Windows wizard (additional channel)**
- ~~Does the release package structure physically carry standard library source?~~ → **Required**
  (user readability aligned with Python `Lib/`, 2026-09-09 decision)
- ~~cargo-dist native installers (shell/powershell/homebrew/msi/npm)?~~ → **All deprecated**,
  installers self-built (flat assumption conflicts with bin/+lib/)
- ~~Installer strategy?~~ → **Two-tier** (2026-09-09 final decision): standard channel = release
  package is the product (Go/Zig model, extract + PATH); easy channel references Rust one-line
  install — Linux `apt` (self-built deb repo) / `curl | sh`, Windows `irm | iex` / Inno exe. Not
  doing: MSI, cargo-dist native installer, self-built brew tap
- ~~Is version manager included?~~ → **Required, part of installer system** (2026-09-09 decision,
  overturning same-day earlier "future independent RFC" boundary): motivation is Python/Node lack
  official version management leading to pyenv/nvm/pdm post-hoc ecosystem fragmentation
- ~~Version manager form: independent binary or subcommand?~~ → **Front door / engine separation**
  (2026-09-09 final decision, three rounds of convergence A→C→naming inversion): common command `yx`
  = front door (small binary, built-in toolchain/self verbs), engine `yaoxiang-rs` = rename of
  current `yaoxiang` monolith; directory structure `versions/<ver>/` — version is a first-class
  concept, version directory is release package extraction root, tools locked with version as a
  whole (old fmt doesn't recognize new syntax; previously envisioned inner `toolchains/` cancelled
  for redundancy). Precedents: Go `go` front door + GOTOOLCHAIN, rustup proxy dispatch
- ~~cargo-dist extra-artifacts conditional execution?~~ → **Handle with `package-dist.sh` script,
  shell case branches**
- ~~Standard library interface version compatibility?~~ → **Released together with compiler version,
  in the same compressed package**

## References

- [cargo-dist Official Documentation](https://axodotdev.github.io/cargo-dist/)
- [cargo-dist GitHub](https://github.com/axodotdev/cargo-dist)
- [RFC-014b: Build System and Binary Distribution](../review/014b-build-system.md)
- [cargo-dist nightly feature request](https://github.com/axodotdev/cargo-dist/issues/1143)
- [Z3 Build Configuration — CMakeLists.txt](https://github.com/Z3Prover/z3/blob/master/src/CMakeLists.txt)

---
title: 'Install YaoXiang'
description:
  'Two-tier installation channel — Standard channel: extract and use (Go/Zig mode); Easy channel:
  one-line command + version management (Rust/rustup mode)'
---

# Install YaoXiang

YaoXiang's command surface is **front-end / engine separated** (RFC-037):

- **`yx`** — the front-end, the daily entry point. It has built-in version management
  (`yx toolchain` / `yx self update`); all other commands are passed through to the engine.
- **`yaoxiang-rs`** — the engine, responsible for compile/run/package management/format/LSP; not
  normally invoked directly.

Installation is split into two tiers; all channels share the same artifact structure (`bin/` +
`lib/yaoxiang/std/`).

## Standard channel: extract and use (Go/Zig mode)

Download the platform-specific archive from
[GitHub Releases](https://github.com/ChenXu233/YaoXiang/releases), extract it to any directory, and
add `bin/` to your PATH:

```sh
tar xzf yaoxiang-<version>-<platform>.tar.gz
export PATH="$PWD/yaoxiang-<version>-<platform>/bin:$PATH"
```

After extraction you can use it directly: `yx --version`. The archive contains all dependencies
(including the Z3 shared library) and the readable standard library source under `lib/yaoxiang/std/`
— no extra steps required.

## Easy channel: one-line command (Rust/rustup mode)

### Linux / macOS

```sh
curl -fsSL https://raw.githubusercontent.com/ChenXu233/YaoXiang/main/scripts/install/install.sh | sh
```

### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/ChenXu233/YaoXiang/main/scripts/install/install.ps1 | iex
```

The script installs the toolchain into `~/.yaoxiang/` (or `%USERPROFILE%\.yaoxiang` on Windows) and
adds `yx` to your PATH.

### Linux: apt repository

```sh
curl -fsSL <apt-repo-url>/KEY.gpg | sudo gpg --dearmor -o /usr/share/keyrings/yaoxiang.gpg
echo "deb [signed-by=/usr/share/keyrings/yaoxiang.gpg] <apt-repo-url> stable main" | sudo tee /etc/apt/sources.list.d/yaoxiang.list
sudo apt update && sudo apt install yaoxiang
```

apt install is a system-level flat install (`/usr/lib/yaoxiang/`, command entry `/usr/bin/yx`);
`apt upgrade` follows the version. Repository metadata is automatically signed and published by the
release CI.

### Windows: Inno Setup wizard

Download `YaoXiang-Setup-<version>.exe` from
[Releases](https://github.com/ChenXu233/YaoXiang/releases) and follow the wizard to install
(optionally auto-added to PATH). Same as apt, this is a system-level flat install.

## Version management (built into the yx front-end)

```sh
yx toolchain install stable    # install the latest stable version
yx toolchain install 0.8.2    # install the specified version
yx toolchain default 0.8.2    # set the default version
yx toolchain list              # list installed versions
yx toolchain update            # upgrade to the latest stable version
yx self update                 # update the yx binary itself
```

Multiple versions coexist under `~/.yaoxiang/versions/<version>/`; each version is a complete,
self-contained toolchain tree (engine, Z3, and standard library are version-locked together).

### Project-level version pinning

Place a `yx-toolchain.toml` at the project root (precedent: `rust-toolchain.toml`):

```toml
toolchain = "0.8.2"
```

Running any `yx` command in that directory will use the version specified by the pin — different
projects can work with different versions.

## Environment variables

| Variable           | Description                                                                   |
| ------------------ | ----------------------------------------------------------------------------- |
| `YAOXIANG_HOME`    | Override the install root; default `~/.yaoxiang` (CI and container scenarios) |
| `YAOXIANG_VERSION` | The install script installs the specified version instead of the latest       |

## Network and mirrors (restricted networks)

When the current network cannot reach GitHub directly, `yx` supports a download mirror: configure a
ghproxy-style URL prefix in `<install-root>/settings.toml` (default `~/.yaoxiang/settings.toml`):

```toml
mirror = "https://ghproxy.example.com"
```

Effective scope (`yx` will concatenate the full GitHub URL after this prefix):

- `yx toolchain install` / `yx toolchain update` — release packages, `.sha256` sidecar checksums,
  and the version query API
- `yx self update` — same as above

Notes:

- The one-line install scripts (`install.sh` / `install.ps1`) do not read the mirror config and
  connect to GitHub directly.
- On download failure, `yx` will suggest configuring a mirror in the error message; if the mirror
  itself is unreachable, it is reported as a network error.

## Verify installation

```sh
yx --version        # front-end version
yx run main.yx      # any YaoXiang program
```

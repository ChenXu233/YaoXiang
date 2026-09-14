---
title: 'Installing YaoXiang'
description:
  'Two-tier installation channels — the standard channel is extract-and-use (Go/Zig style), the
  friendly channel is a one-line command plus version management (Rust/rustup style)'
---

# Installing YaoXiang

YaoXiang's command surface follows a **front door / engine separation** (RFC-037):

- **`yx`** — the front door, the daily entry point. It has built-in version management
  (`yx toolchain` / `yx self update`); all other commands are transparently forwarded to the engine
- **`yaoxiang-rs`** — the engine, responsible for compilation / execution / package management /
  formatting / LSP; generally not invoked directly

Installation is split into two channels, and all channels share the same product structure (`bin/` +
`lib/yaoxiang/std/`).

## Standard Channel: Extract-and-Use (Go/Zig style)

Download the platform-appropriate archive from
[GitHub Releases](https://github.com/ChenXu233/YaoXiang/releases), extract it to any directory, and
add `bin/` to your PATH:

```sh
tar xzf yaoxiang-<version>-<platform>.tar.gz
export PATH="$PWD/yaoxiang-<version>-<platform>/bin:$PATH"
```

After extraction, you can use it directly: `yx --version`. The archive ships with all dependencies
(including the Z3 shared library) and the readable standard library source `lib/yaoxiang/std/`; no
additional steps are required.

## Friendly Channel: One-Line Command (Rust/rustup style)

### Linux / macOS

```sh
curl -fsSL https://raw.githubusercontent.com/ChenXu233/YaoXiang/main/scripts/install/install.sh | sh
```

### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/ChenXu233/YaoXiang/main/scripts/install/install.ps1 | iex
```

The script installs the toolchain into `~/.yaoxiang/` (on Windows: `%USERPROFILE%\.yaoxiang`) and
puts `yx` onto your PATH.

### Linux: apt repository

```sh
curl -fsSL <apt-repo-url>/KEY.gpg | sudo gpg --dearmor -o /usr/share/keyrings/yaoxiang.gpg
echo "deb [signed-by=/usr/share/keyrings/yaoxiang.gpg] <apt-repo-url> stable main" | sudo tee /etc/apt/sources.list.d/yaoxiang.list
sudo apt update && sudo apt install yaoxiang
```

apt installs a system-wide flat layout (`/usr/lib/yaoxiang/`, command entry at `/usr/bin/yx`), and
`apt upgrade` follows the versions. Repository metadata is automatically signed and published by the
release CI.

### Windows: Inno Setup wizard

Download `YaoXiang-Setup-<version>.exe` from
[Releases](https://github.com/ChenXu233/YaoXiang/releases) and follow the wizard (optionally adds
itself to PATH). Like apt, this is a system-wide flat installation.

## Version Management (built into the yx front door)

```sh
yx toolchain install stable    # Install the latest stable version
yx toolchain install 0.7.14    # Install a specific version
yx toolchain default 0.7.14    # Set the default version
yx toolchain list              # List installed versions
yx toolchain update            # Upgrade to the latest stable version
yx self update                 # Update the yx binary itself
```

Multiple versions coexist under `~/.yaoxiang/versions/<version>/`, where each version is a fully
self-contained toolchain tree (engine, Z3, and standard library are version-locked together).

### Project-level Version Pinning

Place a `yx-toolchain.toml` at the project root (precedent: `rust-toolchain.toml`):

```toml
toolchain = "0.7.14"
```

Any `yx` command run within that directory will use the pinned version — different projects can work
with different versions.

## Environment Variables

| Variable           | Description                                                                            |
| ------------------ | -------------------------------------------------------------------------------------- |
| `YAOXIANG_HOME`    | Overrides the install root; defaults to `~/.yaoxiang` (for CI and container scenarios) |
| `YAOXIANG_VERSION` | The install script installs the specified version rather than the latest               |

## Network and Mirrors (restricted networks)

When the current network cannot reach GitHub directly, `yx` supports download mirrors. Configure a
ghproxy-style URL prefix in `<install-root>/settings.toml` (default `~/.yaoxiang/settings.toml`):

```toml
mirror = "https://ghproxy.example.com"
```

Scope of effect (`yx` appends the full GitHub URL to the end of this prefix):

- `yx toolchain install` / `yx toolchain update` — release archives, `.sha256` sidecar files, and
  the version query API
- `yx self update` — same as above

Notes:

- The one-line install scripts (`install.sh` / `install.ps1`) do not read the mirror configuration
  and connect directly to GitHub
- When a download fails, `yx` will suggest configuring a mirror in the error message; if the mirror
  itself is unreachable, it is reported as a network error

## Verifying the Installation

```sh
yx --version        # Front door version
yx run main.yx      # Any YaoXiang program
```

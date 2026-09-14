---
title: 'Installing YaoXiang'
description:
  'Dual-layer installation channels—standard channel is extract-and-use (Go/Zig mode), easy channel
  is one-line command + version management (Rust/rustup mode)'
---

# Installing YaoXiang

YaoXiang's command surface is **front-door / engine separation** (RFC-037):

- **`yx`** — The front door, the daily entry point. Built-in version management (`yx toolchain` /
  `yx self update`); other commands are passed through to the engine.
- **`yaoxiang-rs`** — The engine, responsible for compilation, running, package management,
  formatting, and LSP. Not generally called directly.

Installation is split into two channels; all channels share the same artifact structure (`bin/` +
`lib/yaoxiang/std/`).

## Standard Channel: Extract and Use (Go/Zig Mode)

Download the archive for your platform from
[GitHub Releases](https://github.com/ChenXu233/YaoXiang/releases), extract it to any directory, and
add `bin/` to your PATH:

```sh
tar xzf yaoxiang-<version>-<platform>.tar.gz
export PATH="$PWD/yaoxiang-<version>-<platform>/bin:$PATH"
```

After extraction, you can use it directly: `yx --version`. The archive includes all dependencies
(including the Z3 shared library) and the readable standard library source `lib/yaoxiang/std/`,
requiring no additional steps.

## Easy Channel: One-Line Command (Rust/rustup Mode)

### Linux / macOS

```sh
curl -fsSL https://raw.githubusercontent.com/ChenXu233/YaoXiang/main/scripts/install/install.sh | sh
```

### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/ChenXu233/YaoXiang/main/scripts/install/install.ps1 | iex
```

The script installs the toolchain into `~/.yaoxiang/` (on Windows: `%USERPROFILE%\.yaoxiang`) and
puts `yx` into your PATH.

### Linux: apt Repository

```sh
curl -fsSL <apt repo address>/KEY.gpg | sudo gpg --dearmor -o /usr/share/keyrings/yaoxiang.gpg
echo "deb [signed-by=/usr/share/keyrings/yaoxiang.gpg] <apt repo address> stable main" | sudo tee /etc/apt/sources.list.d/yaoxiang.list
sudo apt update && sudo apt install yaoxiang
```

The apt install is a system-level flat install (`/usr/lib/yaoxiang/`, command entry `/usr/bin/yx`);
versions follow `apt upgrade`. Repository metadata is automatically signed and published by the
release CI.

### Windows: Inno Setup Wizard

Download `YaoXiang-Setup-<version>.exe` from
[Releases](https://github.com/ChenXu233/YaoXiang/releases) and follow the wizard to install
(optionally added to PATH automatically). Like apt, this is a system-level flat install.

## Version Management (Built into the yx Front Door)

```sh
yx toolchain install stable    # Install the latest stable version
yx toolchain install 0.7.14    # Install a specific version
yx toolchain default 0.7.14    # Set the default version
yx toolchain list              # List installed versions
yx toolchain update            # Upgrade to the latest stable version
yx self update                 # Update the yx binary itself
```

Multiple versions coexist under `~/.yaoxiang/versions/<version>/`. Each version is a complete,
self-contained toolchain tree (engine, Z3, and standard library all version-locked together).

### Project-Level Version Pinning

Place a `yx-toolchain.toml` in the project root (modeled after `rust-toolchain.toml`):

```toml
toolchain = "0.7.14"
```

Running any `yx` command in that directory will use the version specified by the pin—different
projects can work with different versions.

## Environment Variables

| Variable           | Description                                                                            |
| ------------------ | -------------------------------------------------------------------------------------- |
| `YAOXIANG_HOME`    | Override the installation root, default `~/.yaoxiang` (for CI and container scenarios) |
| `YAOXIANG_VERSION` | Have the install script install the specified version instead of the latest            |

Mirror downloads can be configured in `~/.yaoxiang/settings.toml` (ghproxy-style prefix):

```toml
mirror = "https://ghproxy.example.com"
```

## Verifying Installation

```sh
yx --version        # Front door version
yx run main.yx      # Any YaoXiang program
```

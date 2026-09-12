---
title: Installing YaoXiang
description:
  Dual installation channels — Standard channel (Go/Zig mode) where you unzip and use, Easy channel
  with one-line command + version management (Rust/rustup mode)
---

# Installing YaoXiang

YaoXiang's command line uses a **front-door / engine separation** (RFC-037):

- **`yx`** — the front door, the everyday entry point. It has built-in version management
  (`yx toolchain` / `yx self update`), and forwards all other commands to the engine
- **`yaoxiang-rs`** — the engine, responsible for compilation / execution / package management /
  formatting / LSP, and is not normally invoked directly

Installation is split into two channels, all sharing the same artifact layout (`bin/` +
`lib/yaoxiang/std/`).

## Standard Channel: Unzip and Use (Go/Zig Mode)

Download the archive for your platform from
[GitHub Releases](https://github.com/ChenXu233/YaoXiang/releases), extract it to any directory, and
add `bin/` to your PATH:

```sh
tar xzf yaoxiang-<version>-<platform>.tar.gz
export PATH="$PWD/yaoxiang-<version>-<platform>/bin:$PATH"
```

After extraction you can use it directly: `yx --version`. The archive ships with all dependencies
(including the Z3 shared library) and browsable standard library sources under `lib/yaoxiang/std/`,
requiring no extra steps.

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
puts `yx` on your PATH.

### Linux: apt Repository

```sh
curl -fsSL <apt-repo-url>/KEY.gpg | sudo gpg --dearmor -o /usr/share/keyrings/yaoxiang.gpg
echo "deb [signed-by=/usr/share/keyrings/yaoxiang.gpg] <apt-repo-url> stable main" | sudo tee /etc/apt/sources.list.d/yaoxiang.list
sudo apt update && sudo apt install yaoxiang
```

apt installs a system-wide version (`/usr/lib/yaoxiang/`, command entry `/usr/bin/yx`), and
`apt upgrade` follows the version. Repository metadata is automatically signed and published by the
release CI.

### Windows: Inno Setup Wizard

Download `YaoXiang-Setup-<version>.exe` from
[Releases](https://github.com/ChenXu233/YaoXiang/releases) and follow the wizard (optionally adding
to PATH automatically). Like apt, this is a system-wide install.

## Version Management (Built into the `yx` Front Door)

```sh
yx toolchain install stable    # Install the latest stable version
yx toolchain install 0.7.14    # Install a specific version
yx toolchain default 0.7.14    # Set the default version
yx toolchain list              # List installed versions
yx toolchain update            # Upgrade to the latest stable version
yx self update                 # Update the yx binary itself
```

Multiple versions coexist under `~/.yaoxiang/versions/<version>/`, where each version is a complete,
self-contained toolchain tree (engine, Z3, and standard library locked to the same version).

### Project-Level Version Pinning

Place a `yx-toolchain.toml` at the project root (precedent: `rust-toolchain.toml`):

```toml
toolchain = "0.7.14"
```

Any `yx` command run in that directory will use the pinned version — different projects can work
with different versions.

## Environment Variables

| Variable           | Description                                                                            |
| ------------------ | -------------------------------------------------------------------------------------- |
| `YAOXIANG_HOME`    | Override the installation root, default `~/.yaoxiang` (for CI and container scenarios) |
| `YAOXIANG_VERSION` | The install script installs this specific version instead of the latest                |

Mirror downloads can be configured in `~/.yaoxiang/settings.toml` (ghproxy-style prefix):

```toml
mirror = "https://ghproxy.example.com"
```

## Verifying the Installation

```sh
yx --version        # Front-door version
yx run main.yx      # Any YaoXiang program
```

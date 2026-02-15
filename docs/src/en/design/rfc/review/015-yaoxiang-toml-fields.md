---
title: RFC-015: YaoXiang Configuration System Design
---

# RFC-015: YaoXiang Configuration System Design

> **Status**: Review
> **Author**: Chen Xu
> **Created Date**: 2026-02-12
> **Last Updated**: 2026-02-15

> **Prerequisite RFC**: [RFC-014: Package Manager Design](014-package-manager.md)

## Summary

Design a unified configuration system for YaoXiang language, supporting user-level and project-level tiers, providing shared configuration infrastructure for package manager, compiler, REPL, LSP, and other components.

## Motivation

### Why this feature/change is needed?

YaoXiang toolchain includes multiple components:
- Package manager (reads dependency configuration)
- Compiler front-end (reads i18n configuration)
- REPL (reads interactive configuration)
- LSP (reads fmt/lint/test configuration)
- Build system (reads build configuration)

All components require unified configuration infrastructure.

### Current problems

- Configuration scattered across components, no unified specification
- Users cannot manage preferences consistently
- No clear hierarchy between project and user configurations

## Proposal

### Core design

**Layered architecture**:
```
Configuration priority (high → low):
┌─────────────────────────────────────────────┐
│ 1. Project-level yaoxiang.toml              │ ← Controlled by project team
├─────────────────────────────────────────────┤
│ 2. User-level ~/.config/yaoxiang/config.toml │ ← User preferences
├─────────────────────────────────────────────┤
│ 3. Compiler defaults                        │ ← Reasonable initial values
└─────────────────────────────────────────────┘
```

**Rule**: Upper layer overrides lower layer; unconfigured options fall back to lower layer.

### Configuration tier restrictions

| Section | User-level | Project-level | Consumer |
|---------|------------|---------------|----------|
| `[package].*` | ❌ | ✅ | Package manager |
| `[yaoxiang]` | ❌ | ✅ | Compiler |
| `[dependencies]` | ❌ | ✅ | Package manager |
| `[dev-dependencies]` | ❌ | ✅ | Package manager |
| `[bin]` | ❌ | ✅ | Package manager |
| `[lib]` | ❌ | ✅ | Package manager |
| `[build]` | ✅ | ✅ | Build system |
| `[profile.*]` | ✅ | ✅ | Build system |
| `[install]` | ✅ | ❌ | Package manager |
| `[i18n]` | ✅ | ✅ | Compiler |
| `[repl]` | ✅ | ✅ | REPL |
| `[fmt]` | ✅ | ✅ | LSP |
| `[lint]` | ✅ | ✅ | LSP |
| `[test]` | ✅ | ✅ | LSP |
| `[tasks]` | ✅ | ✅ | CLI |

### Example

**Project-level configuration**:
```toml
# yaoxiang.toml
[package]
name = "my-package"
version = "0.1.0"

[yaoxiang]
version = ">=0.1.0, <1.0.0"

[dependencies]
foo = "^1.0.0"

[build]
output = "dist/"

[tasks]
build = "yaoxiang build"
test = "yaoxiang test"
```

**User-level configuration**:
```toml
# ~/.config/yaoxiang/config.toml
[install]
dir = "~/.local/share/yaoxiang"

[i18n]
lang = "zh"
fallback = "en"

[repl]
history-size = 1000
prompt = "yx> "
colors = true

[fmt]
line-width = 120
indent-width = 4

[lint]
rules = ["recommended"]
```

## Detailed design

### Project-level only configuration

```toml
[package]
name = "my-package"
version = "0.1.0"
description = "A short description"
authors = ["Alice <alice@example.com>"]
license = "MIT"
repository = "https://github.com/alice/my-project"

[yaoxiang]
version = ">=0.1.0, <1.0.0"

[dependencies]
foo = "^1.0.0"

[dev-dependencies]
test-utils = "0.1.0"

[lib]
path = "src/lib.yx"

[[bin]]
name = "my-cli"
path = "src/cli.yx"

[exports]
"." = "src/lib.yx"
"./foo" = "src/foo.yx"

[build]
script = "build.yx"
output = "dist/"

[profile.release]
optimize = true
lto = true

[run]
main = "src/main.yx"
args = ["--quiet"]

[tasks]
build = "yaoxiang build"
test = "yaoxiang test"
lint = "yaoxiang fmt && yaoxiang check"
```

### User-level only configuration

```toml
[install]
dir = "~/.local/share/yaoxiang"
```

### Shared configuration

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `[i18n].lang` | String | "en" | Language |
| `[i18n].fallback` | String | "en" | Fallback language |
| `[repl].history-size` | Number | 1000 | History entries |
| `[repl].history-file` | Path | ~ | History file |
| `[repl].prompt` | String | "yx> " | Prompt |
| `[repl].colors` | Boolean | true | Syntax highlighting |
| `[repl].auto-imports` | [String] | [] | Auto-imports |
| `[fmt].line-width` | Number | 120 | Line width |
| `[fmt].indent-width` | Number | 4 | Indent width |
| `[fmt].use-tabs` | Boolean | false | Tab indentation |
| `[fmt].single-quote` | Boolean | false | Single quotes |
| `[lint].rules` | [String] | ["recommended"] | Rule set |
| `[lint].strict` | Boolean | false | Strict mode |
| `[test].report` | String | "console" | Test report |
| `[build].output` | String | "dist/" | Output directory |

### CLI and environment variable overrides

```bash
# CLI overrides
yaoxiang run main.yx --lang zh
yaoxiang fmt --config-indent-width=2

# Environment variables
export YAOXIANG_LANG=zh
export YAOXIANG_FMT_INDENT_WIDTH=2
```

**Priority**: `CLI > Environment Variables > Configuration File`

### yaoxiang config command

Provides CLI commands to manage configuration:

```bash
# Initialize user-level configuration (generate with defaults)
yaoxiang config init

# Edit user-level configuration (opens editor)
yaoxiang config edit

# Show current configuration (merged effective configuration)
yaoxiang config show

# Show configuration sources
yaoxiang config show --source

# Reset to default configuration
yaoxiang config reset
```

**First run**: When users first run any `yaoxiang` command, it automatically checks if user-level configuration exists. If not, it auto-generates with default options.

**Configuration file locations**:
- Project-level: `./yaoxiang.toml` (project root)
- User-level: `~/.config/yaoxiang/config.toml`

### Configuration merge semantics

Configuration from different tiers merges according to the following rules:

| Type | Strategy | Description |
|------|----------|-------------|
| Scalar (String/Number/Boolean) | Replace | Project-level overrides user-level |
| Array (Array) | Replace | Project-level completely replaces user-level |
| Object (Object) | Deep merge | Field-by-field merge, undefined fields inherit from lower tier |

**Example - Object deep merge**:
```toml
# User-level
[lint]
rules = ["recommended"]
severity = "warn"

# Project-level
[lint]
strict = true

# Merged result
[lint]
rules = ["recommended"]    # from user-level
severity = "warn"          # from user-level
strict = true             # from project-level
```

### Backward compatibility

- ✅ Existing no-config mode continues to be supported (all components use built-in defaults)
- ✅ New configuration options all have reasonable defaults
- ✅ Auto-generate configuration with default options on first run
- ✅ Show friendly error messages on config parse failure, with specific line numbers and error details

## Trade-offs

### Advantages

- Unified configuration infrastructure, reduces duplicate code
- User preferences consistent across projects
- LSP/REPL/compiler share the same configuration
- Progressive configuration, declare as needed

### Disadvantages

- Many configuration options, slightly increased learning curve
- Requires a unified configuration parser

## Alternative approaches

| Approach | Why not chosen |
|----------|----------------|
| Separate configuration per component | Duplicate code, fragmented user experience |
| CLI arguments only | Cannot persist user preferences |
| Environment variables only | Project configuration hard to version control |

## Implementation strategy

### Phase breakdown

| Phase | Content |
|-------|---------|
| **Phase 1** | Basic configuration parser, toml support, project-level configuration, `yaoxiang config init` |
| **Phase 2** | User-level configuration, configuration merging logic, `yaoxiang config edit/show` |
| **Phase 3** | CLI/environment variable overrides, `platform` platform constraints, `[tool.*]` extension |

### Dependencies

- Depends on RFC-014 Package Manager

### Risks

| Risk | Mitigation |
|------|------------|
| Too many configuration options | Provide reasonable defaults, invisible to users |
| Complex parser | Use existing toml library |

## Open questions

- [x] `features` conditional compilation syntax? → **Moved to separate RFC**, depends on RFC-011 Generic Type System
- [x] `workspace` workspace design? → **Moved to separate RFC**, high complexity, requires independent design

### Accepted features (Phase 3)

#### `platform` Platform Constraints

> **Note**: The following syntax is used in the `yaoxiang.toml` **configuration file**, **not** in YaoXiang source code (`.yx` files). Users should **not** write `cfg(...)` in their source code.

Support platform-specific configuration based on target OS/architecture:

```toml
# yaoxiang.toml (configuration file)

[target.'cfg(windows)'.build]
output = "dist/win32"

[target.'cfg(unix)'.build]
output = "dist/unix"

[target.'cfg(target_arch = "x86_64")'.build]
rustflags = ["-C target-cpu=native"]
```

**Syntax**: `[target.'<condition>'.<section>]`

**Explanation**:
- This syntax only appears in `yaoxiang.toml` configuration file
- During build, configurations are selected based on the `--target` parameter
- Users should **not** write `cfg(...)` syntax in `.yx` source code

**Supported conditions**:
- `cfg(os = "windows")` - Windows
- `cfg(os = "linux")` - Linux
- `cfg(os = "macos")` - macOS
- `cfg(target_arch = "x86_64")` - 64-bit x86
- `cfg(target_arch = "aarch64")` - ARM 64-bit

#### `[tool.*]` Third-party Tool Configuration Extension

Allow third-party tools to store their configuration under `[tool.<name>]`:

```toml
[tool.eslint]
extension = ["yx", "yxp"]
ignore = ["node_modules/", "dist/"]

[tool.prettier]
semi = false
singleQuote = true
```

**Behavior**:
- YaoXiang ignores unknown `[tool.*]` sections but preserves them in the config file
- Third-party tools can be integrated via `yaoxiang tool run <name>` or direct access
- No validation performed on tool-specific configuration

---

## References

- [Cargo Manifest](https://doc.rust-lang.org/cargo/reference/manifest.html)
- [deno.json](https://deno.com/manual@v1.28.3/getting-started/configuration_file)
- [npm package.json](https://docs.npmjs.com/cli/v9/configuring-npm/package-json)

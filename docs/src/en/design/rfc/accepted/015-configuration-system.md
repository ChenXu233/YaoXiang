---
title: "RFC-015: YaoXiang Configuration System Design"
status: "Accepted"
author: "Chenxu"
created: "2026-02-12"
updated: "2026-02-15"
issue: "#133"
---

# RFC-015: YaoXiang Configuration System Design

> **Acceptance Date**: 2026-02-15

> **Prerequisite RFC**: [RFC-014: Package Management System Design](014-package-manager.md)

## Summary

Design a unified configuration system for the YaoXiang language, supporting both user-level and project-level scopes, providing shared configuration infrastructure for components such as the package manager, compiler, REPL, and LSP.

## Motivation

### Why is this feature/change needed?

The YaoXiang toolchain consists of multiple components:
- Package manager (reads dependency configuration)
- Compiler frontend (reads i18n configuration)
- REPL (reads interactive configuration)
- LSP (reads fmt/lint/test configuration)
- Build system (reads build configuration)

All components require a unified configuration infrastructure.

### Current Problems

- Component configurations are scattered with no unified specification
- Users cannot manage preferences in a unified way
- No clear hierarchy between project and user configurations

## Proposal

### Core Design

**Layered Architecture**:
```
Configuration Priority (high → low):
┌─────────────────────────────────────────────┐
│ 1. Project-level yaoxiang.toml               │ ← Controlled by project team
├─────────────────────────────────────────────┤
│ 2. User-level ~/.config/yaoxiang/config.toml│ ← User preferences
├─────────────────────────────────────────────┤
│ 3. Compiler defaults                        │ ← Sensible initial values
└─────────────────────────────────────────────┘
```

**Rule**: Higher layers override lower layers; unset options fall back to lower layers.

### Configuration Layer Restrictions

| Configuration Section | User-level | Project-level | Consumer |
|--------|--------|--------|--------|
| `[package].*` | ❌ | ✅ | Package Manager |
| `[yaoxiang]` | ❌ | ✅ | Compiler |
| `[dependencies]` | ❌ | ✅ | Package Manager |
| `[dev-dependencies]` | ❌ | ✅ | Package Manager |
| `[bin]` | ❌ | ✅ | Package Manager |
| `[lib]` | ❌ | ✅ | Package Manager |
| `[build]` | ✅ | ✅ | Build System |
| `[profile.*]` | ✅ | ✅ | Build System |
| `[install]` | ✅ | ❌ | Package Manager |
| `[i18n]` | ✅ | ✅ | Compiler |
| `[repl]` | ✅ | ✅ | REPL |
| `[fmt]` | ✅ | ✅ | LSP |
| `[lint]` | ✅ | ✅ | LSP |
| `[test]` | ✅ | ✅ | LSP |
| `[tasks]` | ✅ | ✅ | CLI |

### Examples

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

## Detailed Design

### Project-level Exclusive Configuration

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

### User-level Exclusive Configuration

```toml
[install]
dir = "~/.local/share/yaoxiang"
```

### Configuration Available in Both

| Field | Type | Default | Description |
|------|------|--------|------|
| `[i18n].lang` | String | "en" | Language |
| `[i18n].fallback` | String | "en" | Fallback language |
| `[repl].history-size` | Number | 1000 | History entry count |
| `[repl].history-file` | Path | ~ | History file |
| `[repl].prompt` | String | "yx> " | Prompt |
| `[repl].colors` | Boolean | true | Syntax highlighting |
| `[repl].auto-imports` | [String] | [] | Auto-imports |
| `[fmt].line-width` | Number | 120 | Line width |
| `[fmt].indent-width` | Number | 4 | Indent |
| `[fmt].use-tabs` | Boolean | false | Tab indentation |
| `[fmt].single-quote` | Boolean | false | Single quotes |
| `[lint].rules` | [String] | ["recommended"] | Rule sets |
| `[lint].strict` | Boolean | false | Strict mode |
| `[test].report` | String | "console" | Test report |
| `[build].output` | String | "dist/" | Output directory |

### Command-line and Environment Variable Overrides

```bash
# Command-line override
yaoxiang run main.yx --lang zh
yaoxiang fmt --config-indent-width=2

# Environment variables
export YAOXIANG_LANG=zh
export YAOXIANG_FMT_INDENT_WIDTH=2
```

**Priority**: `Command-line > Environment variable > Configuration file`

### yaoxiang config Command

Provides a CLI command to manage configuration:

```bash
# Initialize user-level configuration (generated with default options)
yaoxiang config init

# Edit user-level configuration (opens editor)
yaoxiang config edit

# View current configuration (merged effective configuration)
yaoxiang config show

# View configuration source
yaoxiang config show --source

# Reset to default configuration
yaoxiang config reset
```

**First Run**: When a user runs any `yaoxiang` command for the first time, the system automatically detects whether the user-level configuration exists. If not, it is automatically generated with default options.

**Configuration File Locations**:
- Project-level: `./yaoxiang.toml` (project root)
- User-level: `~/.config/yaoxiang/config.toml`

### Configuration Merge Semantics

Configurations at different layers are merged according to the following rules:

| Type | Strategy | Description |
|------|------|------|
| Scalar (String/Number/Boolean) | Replace | Project-level overrides user-level |
| Array | Replace | Project-level completely replaces user-level |
| Object | Deep merge | Merge field by field; undefined fields inherit from lower layer |

**Example - Object Deep Merge**:
```toml
# User-level
[lint]
rules = ["recommended"]
severity = "warn"

# Project-level
[lint]
strict = true

# Merge result
[lint]
rules = ["recommended"]    # From user-level
severity = "warn"          # From user-level
strict = true             # From project-level
```

### Backward Compatibility

- ✅ Existing no-config-file mode continues to be supported (all components use built-in defaults)
- ✅ Newly added configuration items have sensible default values
- ✅ Configuration is automatically generated with default options on the user's first command run
- ✅ When configuration parsing fails, friendly errors are shown with specific line numbers and error reasons

## Trade-offs

### Advantages

- Unified configuration infrastructure, reducing duplicate code
- User preferences consistent across projects
- LSP/REPL/compiler share the same configuration
- Progressive configuration, declared on demand

### Disadvantages

- Many configuration items, slightly increased learning cost
- Requires a unified configuration parser

## Alternatives

| Approach | Why Not Chosen |
|------|-----------|
| Independent configuration for each component | Duplicate code, fragmented user experience |
| Command-line arguments only | Cannot persist user preferences |
| Environment variables only | Project configuration difficult to version control |

## Implementation Strategy

### Phases

| Phase | Content |
|------|------|
| **Phase 1** | Basic configuration parser, TOML support, project-level configuration, `yaoxiang config init` |
| **Phase 2** | User-level configuration, configuration merge logic, `yaoxiang config edit/show` |
| **Phase 3** | Command-line/environment variable overrides, `platform` constraints, `[tool.*]` extensions |

### Dependencies

- Depends on RFC-014 Package Management System

### Risks

| Risk | Mitigation |
|------|----------|
| Too many configuration items | Provide sensible defaults, transparent to users |
| Parser complexity | Use existing toml library |

## Open Questions

- [x] `features` conditional compilation syntax? → **Moved to a separate RFC**, depends on RFC-011 Generics system
- [x] `workspace` design? → **Moved to a separate RFC**, high complexity, requires independent design

### Accepted Features (Phase 3)

#### `platform` Platform Constraints

> **Note**: The following syntax is used in the `yaoxiang.toml` **configuration file**, **not** in YaoXiang source code (`.yx` files). Users do not need to write `cfg(...)` in code.

Supports platform-specific configuration based on target OS/architecture:

```toml
# yaoxiang.toml (configuration file)

[target.'cfg(windows)'.build]
output = "dist/win32"

[target.'cfg(unix)'.build]
output = "dist/unix"

[target.'cfg(target_arch = "x86_64")'.build]
rustflags = ["-C target-cpu=native"]
```

**Syntax**: `[target.'<condition>'.<configuration section>]`

**Notes**:
- This syntax only appears in the `yaoxiang.toml` configuration file
- The corresponding configuration is selected at build time based on the `--target` parameter
- Users do **not** need to, and **should not**, write `cfg(...)` syntax in `.yx` source code

**Supported Conditions**:
- `cfg(os = "windows")` - Windows system
- `cfg(os = "linux")` - Linux system
- `cfg(os = "macos")` - macOS system
- `cfg(target_arch = "x86_64")` - 64-bit x86 architecture
- `cfg(target_arch = "aarch64")` - ARM 64-bit architecture

#### `[tool.*]` Third-party Tool Configuration Extension

Allows third-party tools to store configuration under `[tool.<name>]`:

```toml
[tool.eslint]
extension = ["yx", "yxp"]
ignore = ["node_modules/", "dist/"]

[tool.prettier]
semi = false
singleQuote = true
```

**Behavior**:
- YaoXiang ignores unknown `[tool.*]` sections, but preserves them in the configuration file
- Third-party tools can be integrated via `yaoxiang tool run <name>` or accessed directly
- Tool-specific configuration is not validated

---

## References

- [Cargo Manifest](https://doc.rust-lang.org/cargo/reference/manifest.html)
- [deno.json](https://deno.com/manual@v1.28.3/getting-started/configuration_file)
- [npm package.json](https://docs.npmjs.com/cli/v9/configuring-npm/package-json)
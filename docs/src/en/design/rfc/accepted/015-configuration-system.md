---
title: "RFC-015: YaoXiang Configuration System Design"
status: "Accepted"
author: "晨煦 (Chenxu)"
created: "2026-02-12"
updated: "2026-02-15"
issue: "#133"

# RFC-015: YaoXiang Configuration System Design

> **Acceptance Date**: 2026-02-15

> **Predecessor RFC**: [RFC-014: Package Management System Design](014-package-manager.md)

## Abstract

Design a unified configuration system for the YaoXiang language, supporting both user-level and project-level layers, providing shared configuration infrastructure for components such as the package manager, compiler, REPL, and LSP.

## Motivation

### Why is this feature/change needed?

The YaoXiang toolchain contains multiple components:
- Package manager (reads dependency configuration)
- Compiler frontend (reads i18n configuration)
- REPL (reads interaction configuration)
- LSP (reads fmt/lint/test configuration)
- Build system (reads build configuration)

Each component requires a unified configuration infrastructure.

### Current Problems

- Component configurations are scattered with no unified specification
- Users cannot centrally manage preference settings
- No clear hierarchy between project and user configurations

## Proposal

### Core Design

**Layered Architecture**:
```
Configuration Priority (high → low):
┌─────────────────────────────────────────────┐
│ 1. Project-level yaoxiang.toml                │ ← Project team controlled
├─────────────────────────────────────────────┤
│ 2. User-level ~/.config/yaoxiang/config.toml  │ ← User preferences
├─────────────────────────────────────────────┤
│ 3. Compiler defaults                          │ ← Sensible initial values
└─────────────────────────────────────────────┘
```

**Rule**: Higher layers override lower layers; unconfigured options fall back to the lower layer.

### Configuration Layer Restrictions

| Configuration Section | User-level | Project-level | Consumer |
|--------|--------|--------|--------|
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

### Examples

**Project-level Configuration**:
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

**User-level Configuration**:
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

### Configuration Available at Both Levels

| Field | Type | Default | Description |
|------|------|--------|------|
| `[i18n].lang` | String | "en" | Language |
| `[i18n].fallback` | String | "en" | Fallback language |
| `[repl].history-size` | Number | 1000 | History entries |
| `[repl].history-file` | Path | ~ | History file |
| `[repl].prompt` | String | "yx> " | Prompt |
| `[repl].colors` | Boolean | true | Syntax highlighting |
| `[repl].auto-imports` | [String] | [] | Auto imports |
| `[fmt].line-width` | Number | 120 | Line width |
| `[fmt].indent-width` | Number | 4 | Indent width |
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

Provide CLI commands to manage configuration:

```bash
# Initialize user-level configuration (generate with default options)
yaoxiang config init

# Edit user-level configuration (open editor)
yaoxiang config edit

# View current configuration (merged effective configuration)
yaoxiang config show

# View configuration source
yaoxiang config show --source

# Reset to default configuration
yaoxiang config reset
```

**First Run**: When the user runs any `yaoxiang` command for the first time, automatically detect whether the user-level configuration exists. If it does not, automatically generate one with default options.

**Configuration File Locations**:
- Project-level: `./yaoxiang.toml` (project root directory)
- User-level: `~/.config/yaoxiang/config.toml`

### Configuration Merging Semantics

Configurations at different layers are merged according to the following rules:

| Type | Strategy | Description |
|------|------|------|
| Scalar (String/Number/Boolean) | Replace | Project-level overrides user-level |
| Array | Replace | Project-level completely replaces user-level |
| Object | Deep merge | Merge field by field, undefined fields inherit from lower layer |

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

- ✅ Existing no-configuration-file mode continues to be supported (all components use built-in default values)
- ✅ Newly added configuration items all have sensible default values
- ✅ The user-level configuration is automatically generated with default options on first command run
- ✅ When configuration parsing fails, a friendly error is displayed, indicating the specific line number and error reason

## Trade-offs

### Advantages

- Unified configuration infrastructure reduces duplicate code
- User preferences are consistent across projects
- LSP/REPL/compiler share the same configuration
- Progressive configuration, declared on demand

### Disadvantages

- Many configuration items, slightly higher learning cost
- Requires a unified configuration parser

## Alternatives

| Approach | Why Not Chosen |
|------|-----------|
| Independent configuration per component | Duplicate code, fragmented user experience |
| Command-line parameters only | Cannot persist user preferences |
| Environment variables only | Project configuration is hard to version control |

## Implementation Strategy

### Phase Division

| Phase | Content |
|------|------|
| **Phase 1** | Basic configuration parser, toml support, project-level configuration, `yaoxiang config init` |
| **Phase 2** | User-level configuration, configuration merging logic, `yaoxiang config edit/show` |
| **Phase 3** | Command-line/environment variable overrides, `platform` constraints, `[tool.*]` extensions |

### Dependencies

- Depends on RFC-014 Package Management System

### Risks

| Risk | Mitigation |
|------|----------|
| Too many configuration items | Provide sensible defaults, user has no overhead |
| Parser complexity | Use an existing toml library |

## Open Questions

- [x] `features` conditional compilation syntax? → **Moved to a separate RFC**, depends on RFC-011 generics system
- [x] `workspace` workspace design? → **Moved to a separate RFC**, high complexity, requires independent design

### Accepted Features (Phase Three)

#### `platform` Constraints

> **Note**: The following syntax is used in the `yaoxiang.toml` **configuration file**, **not** in YaoXiang source code (`.yx` files). Users do not need to write `cfg(...)` in code.

Supports platform-specific configuration based on target operating system/architecture:

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

**Description**:
- This syntax only appears in the `yaoxiang.toml` configuration file
- At build time, the corresponding configuration is selected based on the `--target` parameter
- Users do **not** need to and **should not** write the `cfg(...)` syntax in `.yx` source code

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
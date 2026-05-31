---
title: "RFC-015: YaoXiang Configuration System Design"
---

# RFC-015: YaoXiang Configuration System Design

> **Status**: Accepted
> **Author**: Chen Xu
> **Created**: 2026-02-12
> **Accepted**: 2026-02-15
> **Last Updated**: 2026-02-15

> **Prerequisite RFC**: [RFC-014: Package Manager System Design](014-package-manager.md)

## Summary

Design a unified configuration system for the YaoXiang language, supporting two levels: user-level and project-level, providing shared configuration infrastructure for components such as package manager, compiler, REPL, and LSP.

## Motivation

### Why is this feature/change needed?

The YaoXiang toolchain includes multiple components:
- Package manager (reads dependency configuration)
- Compiler frontend (reads i18n configuration)
- REPL (reads interactive configuration)
- LSP (reads fmt/lint/test configuration)
- Build system (reads build configuration)

Each component needs unified configuration infrastructure.

### Current Problems

- Configuration for each component is scattered with no unified standard
- Users cannot manage preference settings in a unified way
- No clear hierarchy between project configuration and user configuration

## Proposal

### Core Design

**Layered Architecture**:
```
Configuration Priority (High → Low):
┌─────────────────────────────────────────────┐
│ 1. Project-level yaoxiang.toml              │ ← Controlled by project team
├─────────────────────────────────────────────┤
│ 2. User-level ~/.config/yaoxiang/config.toml │ ← User preferences
├─────────────────────────────────────────────┤
│ 3. Compiler default values                  │ ← Reasonable initial values
└─────────────────────────────────────────────┘
```

**Rule**: Upper layers override lower layers; unset options fall back to lower layers.

### Configuration Layer Restrictions

| Config Section | User-level | Project-level | Consumer |
|----------------|------------|---------------|----------|
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

### Project-level-only Configuration

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

### User-level-only Configuration

```toml
[install]
dir = "~/.local/share/yaoxiang"
```

### Shared Configuration (Both Levels)

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `[i18n].lang` | String | "en" | Language |
| `[i18n].fallback` | String | "en" | Fallback language |
| `[repl].history-size` | Number | 1000 | History entries |
| `[repl].history-file` | Path | ~ | History file |
| `[repl].prompt` | String | "yx> " | Prompt |
| `[repl].colors` | Boolean | true | Syntax highlighting |
| `[repl].auto-imports` | [String] | [] | Auto imports |
| `[fmt].line-width` | Number | 120 | Line width |
| `[fmt].indent-width` | Number | 4 | Indentation |
| `[fmt].use-tabs` | Boolean | false | Tab indentation |
| `[fmt].single-quote` | Boolean | false | Single quotes |
| `[lint].rules` | [String] | ["recommended"] | Rule set |
| `[lint].strict` | Boolean | false | Strict mode |
| `[test].report` | String | "console" | Test report |
| `[build].output` | String | "dist/" | Output directory |

### Command Line and Environment Variable Overrides

```bash
# Command line overrides
yaoxiang run main.yx --lang zh
yaoxiang fmt --config-indent-width=2

# Environment variables
export YAOXIANG_LANG=zh
export YAOXIANG_FMT_INDENT_WIDTH=2
```

**Priority**: `Command line > Environment variable > Config file`

### yaoxiang config Command

Provide CLI commands for managing configuration:

```bash
# Initialize user-level configuration (generate with default options)
yaoxiang config init

# Edit user-level configuration (open editor)
yaoxiang config edit

# Show current configuration (merged effective configuration)
yaoxiang config show

# Show configuration source
yaoxiang config show --source

# Reset to default configuration
yaoxiang config reset
```

**First Run**: When the user first runs any `yaoxiang` command, automatically check if user-level configuration exists. If not, auto-generate with default options.

**Configuration File Locations**:
- Project-level: `./yaoxiang.toml` (project root)
- User-level: `~/.config/yaoxiang/config.toml`

### Configuration Merge Semantics

Configurations from different layers are merged according to the following rules:

| Type | Strategy | Description |
|------|----------|-------------|
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

# Merged result
[lint]
rules = ["recommended"]    # from user-level
severity = "warn"          # from user-level
strict = true             # from project-level
```

### Backward Compatibility

- ✅ Existing mode without config files continues to be supported (all components use built-in defaults)
- ✅ New configuration items all have reasonable defaults
- ✅ Auto-generate configuration with default options when user first runs commands
- ✅ Display friendly errors on config parsing failure, showing specific line numbers and error reasons

## Trade-offs

### Advantages

- Unified configuration infrastructure, reducing duplicate code
- User preferences consistent across projects
- LSP/REPL/Compiler share the same configuration system
- Incremental configuration, declare as needed

### Disadvantages

- More configuration items, slightly increased learning curve
- Requires a unified configuration parser

## Alternative Approaches

| Approach | Why Not Chosen |
|----------|---------------|
| Each component has independent configuration | Duplicate code, fragmented user experience |
| Command line arguments only | Cannot persist user preferences |
| Environment variables only | Project configuration difficult to version control |

## Implementation Strategy

### Phase Division

| Phase | Content |
|-------|---------|
| **Phase 1** | Basic config parser, toml support, project-level configuration, `yaoxiang config init` |
| **Phase 2** | User-level configuration, config merge logic, `yaoxiang config edit/show` |
| **Phase 3** | Command line/environment variable overrides, `platform` target constraints, `[tool.*]` extensions |

### Dependencies

- Depends on RFC-014 Package Manager System

### Risks

| Risk | Mitigation |
|------|------------|
| Too many configuration items | Provide reasonable defaults, invisible to users |
| Complex parser | Use existing toml library |

## Open Questions

- [x] `features` conditional compilation syntax? → **Moved to separate RFC**, depends on RFC-011 generics system
- [x] `workspace` workspace design? → **Moved to separate RFC**, high complexity, requires independent design

### Accepted Features (Phase 3)

#### `platform` Target Constraints

> **Note**: The following syntax is used in `yaoxiang.toml` **config file**, **not** in YaoXiang source code (`.yx` files). Users do not need to write `cfg(...)` in code.

Support for platform-specific configuration based on target OS/architecture:

```toml
# yaoxiang.toml (config file)

[target.'cfg(windows)'.build]
output = "dist/win32"

[target.'cfg(unix)'.build]
output = "dist/unix"

[target.'cfg(target_arch = "x86_64")'.build]
rustflags = ["-C target-cpu=native"]
```

**Syntax**: `[target.'<condition>'.<config-section>]`

**Explanation**:
- This syntax only appears in `yaoxiang.toml` config files
- During build, the corresponding configuration is selected based on `--target` parameter
- Users **should not** write `cfg(...)` syntax in `.yx` source code

**Supported Conditions**:
- `cfg(os = "windows")` - Windows
- `cfg(os = "linux")` - Linux
- `cfg(os = "macos")` - macOS
- `cfg(target_arch = "x86_64")` - 64-bit x86 architecture
- `cfg(target_arch = "aarch64")` - ARM 64-bit architecture

#### `[tool.*]` Third-party Tool Configuration Extension

Allow third-party tools to store configuration under `[tool.<name>]`:

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
- Third-party tools can be integrated via `yaoxiang tool run <name>` or accessed directly
- Tool-specific configuration is not validated

---

## References

- [Cargo Manifest](https://doc.rust-lang.org/cargo/reference/manifest.html)
- [deno.json](https://deno.com/manual@v1.28.3/getting-started/configuration_file)
- [npm package.json](https://docs.npmjs.com/cli/v9/configuring-npm/package-json)
---
title: 'Command Line Interface'
description: 'Detailed description of all package manager commands'
---

# Command Line Interface

This page lists the exact arguments for each command. For workflows and scenario selection, see the
[Package Management Guide](../../guide/packaging).

## yx init

Initialize a new YaoXiang project.

### Usage

```bash
yx init [NAME] [--lib]
```

### Arguments

| Argument | Type   | Description                                                    |
| -------- | ------ | -------------------------------------------------------------- |
| `NAME`   | string | Project name; **uses the current directory name when omitted** |

### Options

| Option  | Description                               |
| ------- | ----------------------------------------- |
| `--lib` | Generate a library project (`src/lib.yx`) |

### Artifacts

`yaoxiang.toml`, `yaoxiang.lock`, `.gitignore`, `tests/`, `src/main.yx`, `.yaoxiang/vendor/std/`.

If a `[workspace]` exists in the parent directory, `init` will **automatically register the new
package as a workspace member** and output:

```bash
✓ Registered to workspace (key 'mylib')
```

::: warning `--lib` artifacts are an empty shell `src/lib.yx` contains only comments and no code;
you need to fill in the implementation yourself. :::

---

## yx add

Add dependencies to the project.

### Usage

```bash
yx add <DEP> [--git <GIT> | --path <PATH>] [-v <VERSION>] [-D]
```

### Arguments

| Argument | Type   | Description     |
| -------- | ------ | --------------- |
| `DEP`    | string | Dependency name |

### Options

| Option          | Description                                                              |
| --------------- | ------------------------------------------------------------------------ |
| `--git <URL>`   | Git source; **must be specified explicitly for cross-repo dependencies** |
| `--path <PATH>` | Local path (see [Known Issues](../../guide/packaging#capability-status)) |
| `-v, --version` | Version specifier, see table below                                       |
| `-D, --dev`     | Write to `dev-dependencies`                                              |
| `--trust`       | Trust this package's `build.yx`                                          |

### Version Syntax

Version is an **option**, not a positional argument. `yx add foo 1.0.0` will report
`unexpected argument`.

| Syntax              | Meaning                     |
| ------------------- | --------------------------- |
| `1.0.0`             | Exact match (**not** caret) |
| `*`                 | Any version                 |
| `^1.0.0`            | caret                       |
| `~1.0.0`            | Allow patch upgrades        |
| `>=1.0.0`, `<2.0.0` | Lower / upper bound         |
| `1.*`               | Segment-level wildcard      |
| `>=1.2.3, <2.0.0`   | Comma combination           |

`1.2`, `1` are auto-completed to `1.2.0`, `1.0.0`.

### Examples

```bash
# Git source (recommended)
yx add some-lib --git https://github.com/example/some-lib
yx add some-lib --git https://github.com/example/some-lib --version "^0.1.0"

# Dev dependency
yx add test-utils --git https://github.com/example/test-utils --dev
```

::: danger Don't omit the source option Without `--git` / `--path`, the dependency falls to the
official registry source, which is not yet implemented; `yx install` will fail. :::

---

## yx rm

Remove dependencies from the project.

### Usage

```bash
yx rm <DEP> [-D]
```

### Options

| Option      | Description                    |
| ----------- | ------------------------------ |
| `-D, --dev` | Remove from `dev-dependencies` |

---

## yx install

Install project dependencies.

### Usage

```bash
yx install [--trust]
```

### Behavior

1. Parse dependencies in `yaoxiang.toml`
2. Download to `.yaoxiang/vendor/`
3. Generate or update `yaoxiang.lock`
4. Verify vendor matches the lock file

### Examples

```bash
$ yx install
✓ Resolved 1 dependencies:
  some-lib (0.1.0) [Installed]

Updated yaoxiang.lock
```

When `install` is run at the workspace root, dependencies of all members are merged and uniformly
placed in the root lock file and root vendor.

---

## yx update

Update dependencies.

### Usage

```bash
yx update [PKG] [--trust]
```

| Argument | Description                                 |
| -------- | ------------------------------------------- |
| `PKG`    | Specify package name; update all if omitted |

---

## yx list

List project dependencies and their sources.

### Usage

```bash
yx list
```

### Examples

```bash
$ yx list
app v0.1.0

[dependencies]
  mylib = "*"
```

::: warning Not available at workspace root The workspace root manifest has no `[package]` section;
running it in the root directory will report `missing field package`. Please run it in a member
directory. :::

---

## yx outdated

Check whether dependencies have new versions.

### Usage

```bash
yx outdated
```

### Description

- The default branch is checked for Git sources
- path / workspace sources are skipped
- Dependencies pinned to a tag / branch / rev are reported as pinned

```bash
$ yx outdated
All dependencies are up to date.
```

---

## yx clean

Clean build artifacts and packages in vendor that are not referenced by the lock file.

### Usage

```bash
yx clean
```

---

## yx cache clean

Clear the global package cache.

### Usage

```bash
yx cache clean
```

```bash
$ yx cache clean
Cache cleaned (freed 29.70 KB)
```

Currently `cache` has only one subcommand: `clean`.

---

## yx workspace

Manage workspace members.

### Usage

```bash
yx workspace list
yx workspace add <PATH> [--as <KEY>]
yx workspace remove <KEY>
```

### Description

- `add` defaults to the member's `[package].name` for the key; `--as` can specify it explicitly
- `remove` only unregisters; the directory is preserved
- Members reference siblings with `{ workspace = "<key>" }`

### `[workspace]` Section

```toml
[workspace.members]
app = "app/yaoxiang.toml"
mylib = "mylib/yaoxiang.toml"

[workspace.dependencies]
```

::: warning members values are manifest file paths Writing a directory (e.g. `"app"`) will produce
the misleading `nested workspace is not supported` error. :::

---

## yx publish

Package and publish. The currently available channel is GitHub Release; the official registry is not
yet implemented.

### Usage

```bash
yx publish --dry-run          # Local validation + packaging, no publish
yx publish --github           # Publish as GitHub Release
yx publish --github --no-test # Skip pre-publish tests
```

### Options

| Option      | Description                                                       |
| ----------- | ----------------------------------------------------------------- |
| `--dry-run` | Validate locally and generate `.yxpkg`, **zero network requests** |
| `--github`  | Publish as GitHub Release                                         |
| `--no-test` | Skip pre-publish tests                                            |

### Prerequisites

`[package].description` is required; otherwise it reports
`[package] must provide description for publishing`.

### --dry-run

Process: run `[tool.test]` → workspace references materialized as `^version` → package → compute
SHA-256.

```bash
$ yx publish --dry-run
Running pre-publish tests…
No tests found.
✓ Packed mylib/target/yxpkg/mylib-0.1.0.yxpkg (SHA-256 47a2c83…)
```

### --github

Requires the environment variable `YX_GITHUB_TOKEN`. The target repository is determined by
`[package].repository` first, falling back to `git remote origin`.

**publish does not create tags for you**; you must push them yourself first:

```bash
git tag v0.1.0
git push origin v0.1.0
export YX_GITHUB_TOKEN=<token>
yx publish --github
```

If a Release with the same name already exists, it will be rejected.

### Not Yet Available

`yx publish` (without any options), `yx login` / `logout`, `yank`, `--registry <url>` all belong to
the official registry that has not yet been implemented.

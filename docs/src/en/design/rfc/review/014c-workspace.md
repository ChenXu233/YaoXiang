---
title: 'RFC-014c: Workspace Support'
status: 'Under Review'
author: 'Chen Xu'
created: '2026-06-11'
updated: '2026-07-05'
group: 'rfc-014'
issue: '#113'
---

# RFC-014c: Workspace Support

> This RFC is a sub-RFC of [RFC-014: Package Manager Design](../accepted/014-package-manager.md).

## Summary

Defines YaoXiang's workspace mechanism: dependency sharing, path references, unified lockfile, and Cargo workspace integration for developing multiple related packages together.

## Motivation

As projects grow, code needs to be split into multiple packages. These packages need:

- Mutual references (path dependencies)
- Shared external dependency versions (avoid version drift)
- Unified lockfile (ensure build consistency)
- Cargo workspace collaboration (FFI parts)

### Current Problems

- Each project independently manages dependencies, cannot share
- No automatic replacement mechanism for path dependencies at publish time
- No integration with Cargo workspace

## Proposal

### Core Design: Coordination Layer + Self-Contained Members

The root workspace only coordinates; each member is fully self-contained.

### Root yaoxiang.toml

```toml
# Root yaoxiang.toml
[workspace.members]
core = "packages/core/yaoxiang.toml"
utils = "packages/utils/yaoxiang.toml"
app = "packages/app/yaoxiang.toml"
```

**The root toml only does three things:**

1. Declares member list (in dictionary form, key is member name, value is toml path)
2. Provides shared lockfile (`yaoxiang.lock`)
3. Provides shared vendor directory (`.yaoxiang/vendor/`)

**The root toml does not define dependencies.** Each member writes its own dependencies in its own `yaoxiang.toml`.

### Member yaoxiang.toml

```toml
# packages/core/yaoxiang.toml
[package]
name = "core"
version = "0.1.0"

[dependencies]
json = "^2.0.0"
utils = { workspace = "utils" }    # Reference workspace member
regex = "^1.0.0"
```

```toml
# packages/utils/yaoxiang.toml
[package]
name = "utils"
version = "0.2.0"

[dependencies]
regex = "^1.0.0"
```

### Workspace Structure

```
my-workspace/
├── yaoxiang.toml              # Workspace root config
├── yaoxiang.lock              # Shared lockfile
├── .yaoxiang/
│   └── vendor/                # Shared vendor directory
├── packages/
│   ├── core/
│   │   ├── yaoxiang.toml      # Member package config
│   │   └── src/lib.yx
│   ├── utils/
│   │   ├── yaoxiang.toml
│   │   └── src/lib.yx
│   └── app/
│       ├── yaoxiang.toml
│       └── src/main.yx
└── Cargo.toml                 # Optional: Shared Cargo workspace (FFI)
```

### Dependency Resolution

- Each member reads its own `[dependencies]`
- At resolution, merge all members' dependencies to generate a shared lockfile
- Version conflicts are reported at lockfile generation time
- The same package across different members must resolve to the same version

### Workspace Dependency Reference

`{ workspace = "member-name" }` references the **key** of `[workspace.members]` (not the member's `[package].name`).

```toml
# Root yaoxiang.toml
[workspace.members]
utils = "packages/utils/yaoxiang.toml"    # key = "utils"
```

```toml
# packages/app/yaoxiang.toml
[package]
name = "app"

[dependencies]
utils = { workspace = "utils" }   # ✅ Reference key "utils"
# Even if packages/utils/yaoxiang.toml has name = "my-utils"
```

**Why use key instead of name:**

- Key is controlled by workspace, stable and unique
- `[package].name` is a public name, may change at publish time
- Key is the BTreeMap key, naturally unique
- At publish time, workspace references are replaced with version dependencies, key does not leak to public API

### Path Dependencies and Publishing

During development, use workspace references:

```toml
[dependencies]
utils = { workspace = "utils" }
```

At publish time, automatically replace with version dependencies:

```toml
[dependencies]
utils = "^0.2.0"
```

**Version source:** Read the `[package].version` of the depended member, add `^` prefix. Does not check Registry — the authoritative source of version is the member's `yaoxiang.toml`, Registry is just the distribution channel.

The package manager automatically performs this replacement during `yaoxiang publish`.

### Cargo Workspace Integration

If the workspace has FFI packages, you can also define a Cargo workspace:

```toml
# Root Cargo.toml
[workspace]
members = ["packages/core/native", "packages/utils/native"]
```

```
my-workspace/
├── yaoxiang.toml          # YaoXiang workspace
├── Cargo.toml             # Cargo workspace (FFI part)
├── packages/
│   ├── core/
│   │   ├── src/lib.yx     # YaoXiang code
│   │   └── native/
│   │       ├── Cargo.toml # Rust FFI code
│   │       └── src/lib.rs
│   └── utils/
│       ├── src/lib.yx
│       └── native/
│           ├── Cargo.toml
│           └── src/lib.rs
```

`yaoxiang build` automatically detects and calls `cargo build` to compile native parts.

### CLI Commands

| Command                            | Function                              |
| ---------------------------------- | ------------------------------------- |
| `yaoxiang workspace list`          | List workspace members                |
| `yaoxiang workspace add <path>`    | Add a member                          |
| `yaoxiang workspace remove <name>` | Remove a member                       |
| `yaoxiang build`                   | Build all members (by dependency order) |
| `yaoxiang build core`              | Build specific member                 |
| `yaoxiang test`                    | Run tests for all members             |

**`yaoxiang build` behavior:** Build all members, sorted by dependency topology. If core → utils → app, build order is core → utils → app.

## Detailed Design

### WorkspaceManifest Structure

The root toml uses a separate `WorkspaceManifest` type, not reusing `PackageManifest`:

```rust
struct WorkspaceManifest {
    workspace: WorkspaceConfig,
}

struct WorkspaceConfig {
    members: BTreeMap<String, String>,  // key -> toml path
}

struct Workspace {
    root: PathBuf,
    manifest: WorkspaceManifest,
    members: Vec<WorkspaceMember>,
    lock: LockFile,
}

struct WorkspaceMember {
    name: String,           // Key of [workspace.members]
    root: PathBuf,
    manifest: PackageManifest,
}
```

**Detection logic:** When loading toml, if there is a `[workspace]` section, parse as `WorkspaceManifest`, otherwise parse as `PackageManifest`.

### Workspace Dependency Reference

`{ workspace = "member-name" }` semantics:

- References another workspace member in `dependencies`
- Resolves to local path during development
- Replaced with Registry version at publish time
- Member name must exist in `[workspace.members]`

### Lockfile Sharing

- Workspace has only one `yaoxiang.lock` (in root directory)
- All members' dependencies are merged into the same lockfile
- Version conflicts are reported at lockfile generation with conflict source information

## Trade-offs

### Advantages

- Unified management for multi-package projects
- Shared lockfile ensures consistency
- Good developer experience with path dependencies
- Seamless integration with Cargo workspace

### Disadvantages

- All members must use the same external dependency versions (may be too strict)
- Root toml cannot have its own dependencies (design constraint)
- Cargo workspace integration adds complexity

## Alternative Solutions

| Solution                     | Why Not Chosen                        |
| ---------------------------- | ------------------------------------- |
| Independent projects + path dependencies | Lockfile not unified, version drift risk |
| Like npm workspaces          | npm's workspace has many problems, not worth copying |
| Reuse Cargo workspace directly | YaoXiang and Cargo are different package ecosystems |

## Implementation Strategy

### Phase Breakdown

| Phase     | Content                                           |
| --------- | ------------------------------------------------- |
| Phase 6a  | `[workspace.members]` parsing + WorkspaceManifest |
| Phase 6b  | Shared lockfile + dependency merge resolution    |
| Phase 6c  | `{ workspace = "name" }` path dependency reference |
| Phase 6d  | Automatic path dependency replacement at publish  |
| Phase 6e  | Cargo workspace integration                       |

### Dependencies

- Depends on RFC-014 Phase 3 (global cache)
- Optional dependency RFC-014b (build system, for native members)

## Open Questions

- [ ] Should circular dependencies between members be allowed?
- [ ] Should workspace-level `[build]` configuration be supported?
- [ ] Can members have their own lockfile (override root lockfile)?
- [ ] Should nested workspaces be supported?

---

## References

- [Cargo Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [npm Workspaces](https://docs.npmjs.com/cli/using-npm/workspaces)
- [pnpm Workspaces](https://pnpm.io/workspaces)

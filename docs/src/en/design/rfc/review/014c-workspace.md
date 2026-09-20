---
title: 'RFC-014c: Workspace Support'
status: 'Under Review'
author: 'Chenxu'
created: '2026-06-11'
updated: '2026-09-15'
group: 'rfc-014'
issue: '#113'
---

# RFC-014c: Workspace Support

> This RFC is a sub-RFC of
> [RFC-014: Package Management System Design](../accepted/014-package-manager.md).

## 2026-09-15 Review Decision

The following decisions were made by the owner on 2026-09-15:

1. **Implementation Order Moved Up**: The workspace does not depend on the network and build system
   (`{ workspace = "key" }` path resolution + shared lockfile is purely local), so the schedule is
   moved up to **before** RFC-014b (the overall execution order is adjusted to
   `3 → 3.5 → 6 → 4 → 5`). The compiler repository itself (compiler + vscode extension + wasm +
   docs) will be the first user.
2. **Workspace-level `[build]`**: Not supported. We adhere to the principle of "root only
   coordinates, members are self-contained"; build declarations are only written in member toml
   files.
3. **Member-owned lockfile**: Not allowed; the root `yaoxiang.lock` is the only one.
4. **Nested workspace**: Not supported initially.

## Summary

Define the workspace mechanism for YaoXiang: dependency sharing, path references, unified lockfile,
and integration with Cargo workspaces when multiple related packages are developed together.

## Motivation

As the project grows in scale, the code needs to be split into multiple packages. These packages
need:

- Reference each other (path dependencies)
- Share external dependency versions (to avoid version drift)
- Unified lockfile (to ensure build consistency)
- Collaborate with Cargo workspaces (for the FFI portion)

### Current Issues

- Each project manages dependencies independently and cannot share
- No automatic replacement mechanism for path dependencies at publish time
- No integration with Cargo workspaces

## Proposal

### Core Design: Coordination Layer + Self-Contained Members

The root workspace only coordinates; each member is completely self-contained.

### Root yaoxiang.toml

```toml
# 根 yaoxiang.toml
[workspace.members]
core = "packages/core/yaoxiang.toml"
utils = "packages/utils/yaoxiang.toml"
app = "packages/app/yaoxiang.toml"
```

**The root toml does only three things:**

1. Declares the member list (in dictionary form, where key is the member name and value is the toml
   path)
2. Provides a shared lockfile (`yaoxiang.lock`)
3. Provides a shared vendor directory (`.yaoxiang/vendor/`)

**The root toml does not define dependencies.** Each member's dependencies are written in its own
`yaoxiang.toml`.

### Member yaoxiang.toml

```toml
# packages/core/yaoxiang.toml
[package]
name = "core"
version = "0.1.0"

[dependencies]
json = "^2.0.0"
utils = { workspace = "utils" }    # 引用工作空间成员
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
├── yaoxiang.toml              # 工作空间根配置
├── yaoxiang.lock              # 共享 lockfile
├── .yaoxiang/
│   └── vendor/                # 共享 vendor 目录
├── packages/
│   ├── core/
│   │   ├── yaoxiang.toml      # 成员包配置
│   │   └── src/lib.yx
│   ├── utils/
│   │   ├── yaoxiang.toml
│   │   └── src/lib.yx
│   └── app/
│       ├── yaoxiang.toml
│       └── src/main.yx
└── Cargo.toml                 # 可选：共享 Cargo workspace（FFI）
```

### Dependency Resolution

- Each member reads its own `[dependencies]`
- During resolution, merge all members' dependencies to generate a shared lockfile
- Version conflicts are reported as errors during lockfile generation
- The same package must be resolved to the same version across different members

### Workspace Dependency Reference

`{ workspace = "member-name" }` references the **key** of `[workspace.members]` (not the member's
`[package].name`).

```toml
# 根 yaoxiang.toml
[workspace.members]
utils = "packages/utils/yaoxiang.toml"    # key = "utils"
```

```toml
# packages/app/yaoxiang.toml
[package]
name = "app"

[dependencies]
utils = { workspace = "utils" }   # ✅ 引用 key "utils"
# 即使 packages/utils/yaoxiang.toml 里写的是 name = "my-utils"
```

**Why use key instead of name:**

- The key is controlled by the workspace, stable and unique
- `[package].name` is a public name, which may change at publish time
- The key is a BTreeMap key, naturally unique
- At publish time, workspace references are replaced with version dependencies, so the key does not
  leak into the public API

### Path Dependencies and Publishing

Use workspace references during development:

```toml
[dependencies]
utils = { workspace = "utils" }
```

Automatically replaced with version dependencies at publish time:

```toml
[dependencies]
utils = "^0.2.0"
```

**Version source:** Read the `[package].version` of the depended-on member, and add the `^` prefix.
The Registry is not checked—the authoritative source of the version is the member's `yaoxiang.toml`;
the Registry is only a distribution channel.

The package manager automatically performs this replacement during `yaoxiang publish`.

### Integration with Cargo Workspace

If there are FFI packages in the workspace, a Cargo workspace can also be defined:

```toml
# 根 Cargo.toml
[workspace]
members = ["packages/core/native", "packages/utils/native"]
```

```
my-workspace/
├── yaoxiang.toml          # YaoXiang workspace
├── Cargo.toml             # Cargo workspace（FFI 部分）
├── packages/
│   ├── core/
│   │   ├── src/lib.yx     # YaoXiang 代码
│   │   └── native/
│   │       ├── Cargo.toml # Rust FFI 代码
│   │       └── src/lib.rs
│   └── utils/
│       ├── src/lib.yx
│       └── native/
│           ├── Cargo.toml
│           └── src/lib.rs
```

`yaoxiang build` automatically detects and invokes `cargo build` to compile the native part.

### CLI Commands

| Command                            | Function                                          |
| ---------------------------------- | ------------------------------------------------- |
| `yaoxiang workspace list`          | List workspace members                            |
| `yaoxiang workspace add <path>`    | Add a member                                      |
| `yaoxiang workspace remove <name>` | Remove a member                                   |
| `yaoxiang build`                   | Build all members (sorted by dependency topology) |
| `yaoxiang build core`              | Build the specified member                        |
| `yaoxiang test`                    | Run all members' tests                            |

**Behavior of `yaoxiang build`:** Build all members, sorted by dependency topology. If core → utils
→ app, the build order is core → utils → app.

## Detailed Design

### WorkspaceManifest Structure

The root toml uses an independent `WorkspaceManifest` type and does not reuse `PackageManifest`:

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
    name: String,           // [workspace.members] 的 key
    root: PathBuf,
    manifest: PackageManifest,
}
```

**Detection logic:** When loading the toml, if a `[workspace]` section is present, parse it as
`WorkspaceManifest`; otherwise, parse it as `PackageManifest`.

### Workspace Dependency Reference

Semantics of `{ workspace = "member-name" }`:

- References another workspace member in `dependencies`
- Resolved to a local path during development
- Replaced with a Registry version at publish time
- The member name must exist in `[workspace.members]`

### Lockfile Sharing

- The workspace has only one `yaoxiang.lock` (in the root directory)
- All members' dependency resolution is merged into the same lockfile
- Version conflicts are reported as errors during lockfile generation, with information about the
  source of the conflict

## Trade-offs

### Advantages

- Unified management for multi-package projects
- Shared lockfile ensures consistency
- Path dependencies provide a good development experience
- Seamless integration with Cargo workspaces

### Disadvantages

- All members must use the same external dependency versions (may be too strict)
- The root toml cannot have its own dependencies (design constraint)
- Cargo workspace integration adds complexity

## Alternatives

| Approach                                | Why Not Chosen                                       |
| --------------------------------------- | ---------------------------------------------------- |
| Standalone projects + path dependencies | Lockfile is not unified, risk of version drift       |
| Similar to npm workspaces               | npm's workspace has many issues, not worth imitating |
| Directly reusing Cargo workspace        | YaoXiang and Cargo are different package ecosystems  |

## Implementation Strategy

### Phase Division

| Phase    | Content                                               |
| -------- | ----------------------------------------------------- |
| Phase 6a | `[workspace.members]` parsing + WorkspaceManifest     |
| Phase 6b | Shared lockfile + dependency merge resolution         |
| Phase 6c | `{ workspace = "name" }` path dependency reference    |
| Phase 6d | Automatic path dependency replacement at publish time |
| Phase 6e | Cargo workspace integration                           |

### Dependencies

- Depends on RFC-014 Phase 3 (global cache)
- Optionally depends on RFC-014b (build system, for native members)

## Open Questions

- [x] Are circular dependencies between members allowed? → **Not allowed.** Members are independent
      packages; cycles between packages are compilation errors. (RFC-029 decision, 2026-07-30)
- [x] Is workspace-level `[build]` configuration supported? → Not supported; members are
      self-contained (2026-09-15 decision 2)
- [x] Can a member have its own lockfile (overriding the root lockfile)? → Not allowed; the root
      lockfile is the only one (2026-09-15 decision 3)
- [x] Is nested workspace supported? → Not supported initially (2026-09-15 decision 4)

---

## References

- [Cargo Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [npm Workspaces](https://docs.npmjs.com/cli/using-npm/workspaces)
- [pnpm Workspaces](https://pnpm.io/workspaces)

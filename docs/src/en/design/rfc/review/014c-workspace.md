---
title: 'RFC-014c: Workspace Support'
status: 'Under Review'
author: 'Chenxu'
created: '2026-06-11'
updated: '2026-07-05'
group: 'rfc-014'
issue: '#113'
---

# RFC-014c: Workspace Support

> This RFC is a sub-RFC of [RFC-014: Package Manager Design](../accepted/014-package-manager.md).

## Summary

Define YaoXiang's workspace mechanism: dependency sharing, path references, unified lockfile, and
integration with Cargo workspace when multiple related packages are developed together.

## Motivation

As projects grow, code needs to be split into multiple packages. These packages need:

- To reference each other (path dependency)
- To share external dependency versions (avoid version drift)
- A unified lockfile (ensure build consistency)
- To collaborate with Cargo workspace (for FFI parts)

### Current Problems

- Each project independently manages dependencies, unable to share
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

1. Declares the member list (in dictionary form, where the key is the member name and the value is
   the toml path)
2. Provides the shared lockfile (`yaoxiang.lock`)
3. Provides the shared vendor directory (`.yaoxiang/vendor/`)

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
utils = { workspace = "utils" }    # Reference a workspace member
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
├── yaoxiang.toml              # Workspace root configuration
├── yaoxiang.lock              # Shared lockfile
├── .yaoxiang/
│   └── vendor/                # Shared vendor directory
├── packages/
│   ├── core/
│   │   ├── yaoxiang.toml      # Member package configuration
│   │   └── src/lib.yx
│   ├── utils/
│   │   ├── yaoxiang.toml
│   │   └── src/lib.yx
│   └── app/
│       ├── yaoxiang.toml
│       └── src/main.yx
└── Cargo.toml                 # Optional: shared Cargo workspace (FFI)
```

### Dependency Resolution

- Each member reads its own `[dependencies]`
- During resolution, dependencies from all members are merged to generate a single shared lockfile
- Version conflicts are reported as errors when the lockfile is generated
- The same package must resolve to the same version across different members

### workspace Dependency Reference

`{ workspace = "member-name" }` references the **key** of `[workspace.members]` (not the member's
`[package].name`).

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
utils = { workspace = "utils" }   # ✅ References the key "utils"
# Even if packages/utils/yaoxiang.toml says name = "my-utils"
```

**Why use key instead of name:**

- The key is controlled by the workspace and is stable and unique
- `[package].name` is a public name and may change when published
- The key is the key of a BTreeMap, naturally unique
- At publish time, workspace references are replaced with version dependencies, so the key does not
  leak into the public API

### Path Dependency and Publish

Use workspace reference during development:

```toml
[dependencies]
utils = { workspace = "utils" }
```

Automatically replaced with a version dependency at publish time:

```toml
[dependencies]
utils = "^0.2.0"
```

**Version source:** Read the `[package].version` of the depended member, add the `^` prefix. The
Registry is not consulted — the authoritative source of the version is the member's `yaoxiang.toml`;
the Registry is only the distribution channel.

The package manager automatically performs this replacement during `yaoxiang publish`.

### Integration with Cargo Workspace

If the workspace contains FFI packages, a Cargo workspace can be defined at the same time:

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

`yaoxiang build` automatically detects and invokes `cargo build` to compile the native parts.

### CLI Commands

| Command                            | Function                                          |
| ---------------------------------- | ------------------------------------------------- |
| `yaoxiang workspace list`          | List workspace members                            |
| `yaoxiang workspace add <path>`    | Add a member                                      |
| `yaoxiang workspace remove <name>` | Remove a member                                   |
| `yaoxiang build`                   | Build all members (sorted by dependency topology) |
| `yaoxiang build core`              | Build a specified member                          |
| `yaoxiang test`                    | Run tests for all members                         |

**`yaoxiang build` behavior:** Builds all members, sorted by dependency topology. If core → utils →
app, the build order is core → utils → app.

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
    name: String,           // The key of [workspace.members]
    root: PathBuf,
    manifest: PackageManifest,
}
```

**Detection logic:** When loading a toml, if there is a `[workspace]` section, it is parsed as
`WorkspaceManifest`; otherwise it is parsed as `PackageManifest`.

### workspace Dependency Reference

The semantics of `{ workspace = "member-name" }`:

- References another workspace member in `dependencies`
- Resolves to a local path during development
- Replaced with a Registry version at publish time
- The member name must exist in `[workspace.members]`

### Shared lockfile

- The workspace has only one `yaoxiang.lock` (in the root directory)
- Dependency resolution for all members is merged into the same lockfile
- Version conflicts are reported as errors when the lockfile is generated, with information about
  the source of the conflict

## Trade-offs

### Advantages

- Unified management of multi-package projects
- Shared lockfile ensures consistency
- Good developer experience for path dependencies
- Seamless integration with Cargo workspace

### Disadvantages

- All members must use the same external dependency versions (may be too strict)
- The root toml cannot have its own dependencies (design constraint)
- Cargo workspace integration adds complexity

## Alternatives

| Alternative                              | Why Not Chosen                                       |
| ---------------------------------------- | ---------------------------------------------------- |
| Independent projects + path dependencies | lockfiles are not unified, version drift risk        |
| npm workspaces style                     | npm's workspace issues are many, not worth imitating |
| Reuse Cargo workspace directly           | YaoXiang and Cargo are different package ecosystems  |

## Implementation Strategy

### Phase Division

| Phase    | Content                                               |
| -------- | ----------------------------------------------------- |
| Phase 6a | `[workspace.members]` parsing + WorkspaceManifest     |
| Phase 6b | Shared lockfile + merged dependency resolution        |
| Phase 6c | `{ workspace = "name" }` path dependency reference    |
| Phase 6d | Automatic path dependency replacement at publish time |
| Phase 6e | Cargo workspace integration                           |

### Dependencies

- Depends on RFC-014 Phase 3 (global cache)
- Optional dependency on RFC-014b (build system, for native members)

## Open Questions

- [x] Are circular dependencies between members allowed? → **No.** Members are independent packages;
      circular dependencies between packages are compilation errors. (RFC-029 decision, 2026-07-30)
- [ ] Is workspace-level `[build]` configuration supported?
- [ ] Can a member have its own lockfile (overriding the root lockfile)?
- [ ] Is nested workspace supported?

---

## References

- [Cargo Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [npm Workspaces](https://docs.npmjs.com/cli/using-npm/workspaces)
- [pnpm Workspaces](https://pnpm.io/workspaces)

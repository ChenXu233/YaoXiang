---
title: "RFC-014c: Workspace Support"
status: "Draft"
author: "晨煦"
created: "2026-06-11"
updated: "2026-06-11"
group: "rfc-014"
---

# RFC-014c: Workspace Support

> This RFC is a sub-RFC of [RFC-014: Package Manager Design](../accepted/014-package-manager.md).

## Summary

Define YaoXiang's workspace mechanism: dependency sharing, path references, unified lockfile, and Cargo workspace integration when multiple related packages are developed together.

## Motivation

As projects grow in scale, code needs to be split into multiple packages. These packages need:
- Mutual references (path dependencies)
- Shared external dependency versions (to avoid version drift)
- A unified lockfile (to ensure build consistency)
- Collaboration with Cargo workspace (for the FFI portion)

### Current Problems

- Each project manages dependencies independently, no sharing possible
- No mechanism to auto-replace path dependencies at publish time
- No integration with Cargo workspace

## Proposal

### Core Design: Coordination Layer + Self-Contained Members

The root workspace only acts as coordinator; each member is fully self-contained.

### Root yaoxiang.toml

```toml
# Root yaoxiang.toml
[workspace.members]
core = "packages/core/yaoxiang.toml"
utils = "packages/utils/yaoxiang.toml"
app = "packages/app/yaoxiang.toml"
```

**The root toml only does three things:**
1. Declare the member list (dictionary form, where key is the member name and value is the toml path)
2. Provide a shared lockfile (`yaoxiang.lock`)
3. Provide a shared vendor directory (`.yaoxiang/vendor/`)

**The root toml does not define dependencies.** Each member's dependencies are written in its own `yaoxiang.toml`.

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
- During resolution, all members' dependencies are merged to generate a shared lockfile
- Version conflicts are reported as errors when the lockfile is generated
- The same package must resolve to the same version across different members

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

The package manager automatically performs this replacement during `yaoxiang publish`.

### Cargo Workspace Integration

If the workspace contains FFI packages, a Cargo workspace can be defined alongside:

```toml
# Root Cargo.toml
[workspace]
members = ["packages/core/native", "packages/utils/native"]
```

```
my-workspace/
├── yaoxiang.toml          # YaoXiang workspace
├── Cargo.toml             # Cargo workspace (FFI portion)
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

`yaoxiang build` automatically detects and invokes `cargo build` to compile the native portion.

### CLI Commands

| Command | Function |
|------|------|
| `yaoxiang workspace list` | List workspace members |
| `yaoxiang workspace add <path>` | Add a member |
| `yaoxiang workspace remove <name>` | Remove a member |
| `yaoxiang build` | Build all members |
| `yaoxiang build core` | Build a specified member |
| `yaoxiang test` | Run tests for all members |

## Detailed Design

### WorkspaceManifest Structure

```rust
struct WorkspaceManifest {
    members: BTreeMap<String, String>,  // name -> toml path
}

struct Workspace {
    root: PathBuf,
    manifest: WorkspaceManifest,
    members: Vec<WorkspaceMember>,
    lock: LockFile,
}

struct WorkspaceMember {
    name: String,
    root: PathBuf,
    manifest: PackageManifest,
}
```

### workspace Dependency Reference

Semantics of `{ workspace = "member-name" }`:
- References another workspace member in `dependencies`
- Resolves to a local path during development
- Replaced with a Registry version at publish time
- The member name must exist in `[workspace.members]`

### Shared lockfile

- The workspace has only one `yaoxiang.lock` (in the root directory)
- All members' dependency resolution is merged into the same lockfile
- Version conflicts are reported as errors when the lockfile is generated, with information about the source of the conflict

## Trade-offs

### Advantages

- Unified management for multi-package projects
- Shared lockfile ensures consistency
- Good development experience for path dependencies
- Seamless integration with Cargo workspace

### Disadvantages

- All members must use the same external dependency versions (may be too strict)
- The root toml cannot have its own dependencies (design constraint)
- Cargo workspace integration adds complexity

## Alternatives

| Alternative | Why Not Chosen |
|------|-----------|
| Independent projects + path dependencies | Lockfiles are not unified, version drift risk |
| Similar to npm workspaces | npm's workspace has many issues, not worth imitating |
| Direct reuse of Cargo workspace | YaoXiang and Cargo are different package ecosystems |

## Implementation Strategy

### Phase Breakdown

| Phase | Content |
|------|------|
| Phase 6a | `[workspace.members]` parsing + WorkspaceManifest |
| Phase 6b | Shared lockfile + merged dependency resolution |
| Phase 6c | `{ workspace = "name" }` path dependency reference |
| Phase 6d | Auto-replacement of path dependencies at publish time |
| Phase 6e | Cargo workspace integration |

### Dependencies

- Depends on RFC-014 Phase 3 (global cache)
- Optionally depends on RFC-014b (build system, for native members)

## Open Questions

- [ ] Are circular dependencies between members allowed?
- [ ] Is workspace-level `[build]` configuration supported?
- [ ] Can members have their own lockfile (overriding the root lockfile)?
- [ ] Is nested workspace supported?

---

## References

- [Cargo Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [npm Workspaces](https://docs.npmjs.com/cli/using-npm/workspaces)
- [pnpm Workspaces](https://pnpm.io/workspaces)
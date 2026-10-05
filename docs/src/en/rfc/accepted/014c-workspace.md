---
title: 'RFC-014c: Workspace Support'
status: 'Accepted'
author: 'Chenxu'
created: '2026-06-11'
updated: '2026-09-29'
group: 'rfc-014'
issue: '#113'
---

# RFC-014c: Workspace Support

> This RFC is a sub-RFC of
> [RFC-014: Package Management System Design](014-package-manager.md).

## 2026-09-15 Review Resolution

The following resolution was finalized by the owner on 2026-09-15:

1. **Implementation order moved up:** The workspace does not depend on networking or the build
   system (`{ workspace = "key" }` path resolution + shared lockfile is purely local), so it is
   scheduled **before** RFC-014b (the overall execution order is now adjusted to
   `3 → 3.5 → 6 → 4 → 5`). The compiler repository itself (compiler + vscode extension + wasm +
   docs) is the first user.
2. **workspace-level `[build]`:** Not supported. The "root only coordinates, members are
   self-contained" principle is upheld; build declarations are only written in member tomls.
3. **Member-owned lockfile:** Not allowed; the root `yaoxiang.lock` is the only one.
4. **Nested workspace:** Not supported initially.

## Summary

Define YaoXiang's workspace mechanism: dependency sharing, path references, unified lockfile, and
Cargo workspace integration when multiple related packages are developed together.

## Motivation

As projects grow in size, code needs to be split into multiple packages. These packages need to:

- Reference each other (path dependencies)
- Share external dependency versions (avoid version drift)
- Have a unified lockfile (ensure build consistency)
- Coordinate with Cargo workspaces (FFI portion)

### Current Issues

- Each project manages dependencies independently, unable to share
- No automatic replacement mechanism for path dependencies at publish time
- No integration with Cargo workspaces

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

[workspace.dependencies]        # 2026-09-29 revision: authoritative versions for shared dependencies
regex = "^1.0"
json = "^2.0"
```

**The root toml does only four things:**

1. Declare the member list (as a dictionary, key is the member name, value is the toml path)
2. Provide a shared lockfile (`yaoxiang.lock`)
3. Provide a shared vendor directory (`.yaoxiang/vendor/`)
4. (2026-09-29 revision) Declare shared dependency versions (`[workspace.dependencies]`)

> **2026-09-29 revision: New `[workspace.dependencies]` inheritance (the original "root toml does
> not define dependencies" restriction is relaxed).** Under strict unified-version rules, the
> friction of multiple members each declaring a same-named package is foreseeable; following the
> precedent of Cargo/uv, the root declares the authoritative version of shared dependencies, and
> members inherit them item by item with `{ workspace = true }` (upgrading a shared version requires
> changing only the root). Member references are still `{ workspace = "<key>" }` (string) — same
> key, different type, isomorphic to Cargo. Merged resolution: all members' deps + dev-deps are
> merged into the shared lockfile; same-named packages must have a non-empty intersection, otherwise
> report a conflict and list the source members; inconsistent base URLs for git dependencies are
> treated as conflicts.

### Member yaoxiang.toml

```toml
# packages/core/yaoxiang.toml
[package]
name = "core"
version = "0.1.0"

[dependencies]
json = "^2.0.0"
utils = { workspace = "utils" }    # reference workspace member
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
├── yaoxiang.toml              # workspace root configuration
├── yaoxiang.lock              # shared lockfile
├── .yaoxiang/
│   └── vendor/                # shared vendor directory
├── packages/
│   ├── core/
│   │   ├── yaoxiang.toml      # member package configuration
│   │   └── src/lib.yx
│   ├── utils/
│   │   ├── yaoxiang.toml
│   │   └── src/lib.yx
│   └── app/
│       ├── yaoxiang.toml
│       └── src/main.yx
└── Cargo.toml                 # optional: shared Cargo workspace (FFI)
```

### Dependency Resolution

- Each member reads its own `[dependencies]`
- All members' dependencies are merged at resolution time, producing a single shared lockfile
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
utils = { workspace = "utils" }   # ✅ reference key "utils"
# Even if packages/utils/yaoxiang.toml says name = "my-utils"
```

**Why use the key instead of the name:**

- The key is controlled by the workspace, stable and unique
- `[package].name` is the public name, which may change at publish time
- The key is the key of a BTreeMap, naturally unique
- At publish time, workspace references are replaced with version dependencies; the key does not
  leak into the public API

### Path Dependency and Publishing

Use workspace reference during development:

```toml
[dependencies]
utils = { workspace = "utils" }
```

Automatically replaced with version dependency at publish time:

```toml
[dependencies]
utils = "^0.2.0"
```

**Version source:** Read the referenced member's `[package].version` and prepend `^`. The Registry
is not consulted — the authoritative source of the version is the member's `yaoxiang.toml`; the
Registry is only a distribution channel.

The package manager performs this replacement automatically on `yaoxiang publish`.

### Cargo Workspace Integration

If the workspace contains FFI packages, a Cargo workspace can be defined at the same time:

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

| Command                            | Function                                                                                                 |
| ---------------------------------- | -------------------------------------------------------------------------------------------------------- |
| `yaoxiang workspace list`          | List workspace members                                                                                   |
| `yaoxiang workspace add <path>`    | Add a member (key is taken from `[package].name`, overridable with `--as`; ✅ implemented)               |
| `yaoxiang workspace remove <name>` | Remove a member (only unregisters, does not delete directory; warns if still referenced; ✅ implemented) |
| `yaoxiang build`                   | Build all members (sorted by dependency topology)                                                        |
| `yaoxiang build core`              | Build a specified member                                                                                 |
| `yaoxiang test`                    | Run all members' tests                                                                                   |

**`yaoxiang build` behavior:** Builds all members, sorted by dependency topology. If core → utils →
app, the build order is core → utils → app.

## Detailed Design

### WorkspaceManifest Structure

The root toml uses a dedicated `WorkspaceManifest` type, not reusing `PackageManifest`:

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
    name: String,           // key in [workspace.members]
    root: PathBuf,
    manifest: PackageManifest,
}
```

**Detection logic:** When loading a toml, if it contains a `[workspace]` section, it is parsed as
`WorkspaceManifest`; otherwise, it is parsed as `PackageManifest`.

### workspace Dependency Reference

The semantics of `{ workspace = "member-name" }`:

- References another workspace member in `dependencies`
- Resolves to a local path during development
- Replaced with a Registry version at publish time
- The member name must exist in `[workspace.members]`

### Member Visibility (2026-09-29 finalization: pnpm-style strict)

A member's `use` is resolved **only from the dependencies it itself declares** (version-type + path
dependency + workspace reference) — packages declared by other members in the shared vendor are
**invisible** to this member. Undeclared means not installed; misuse directly reports "module not
found" with a hint to add the declaration: phantom dependencies (the famous pit of npm hoisting) are
exposed at compile time, not as a bomb for consumers after package publication.

Member references and path dependencies resolve via the path to the target package's root `src/`
layout (same structure as a vendor entry, no version suffix), and apply the target's import surface
(RFC-029f). Consistency checks are likewise anchored at the workspace root: merged dependencies vs.
root lockfile vs. root vendor.

### Shared lockfile

- The workspace has only one `yaoxiang.lock` (in the root directory)
- All members' dependency resolution is merged into the same lockfile
- Version conflicts are reported as errors when the lockfile is generated, with source-of-conflict
  information attached

## Trade-offs

### Advantages

- Unified management for multi-package projects
- Shared lockfile ensures consistency
- Path dependencies offer a good development experience
- Seamless integration with Cargo workspace

### Disadvantages

- All members must use the same version of external dependencies (may be too strict)
- The root toml cannot have its own dependencies (design constraint)
- Cargo workspace integration adds complexity

## Alternatives

| Approach                                 | Why Not Chosen                                       |
| ---------------------------------------- | ---------------------------------------------------- |
| Independent projects + path dependencies | Ununified lockfile, risk of version drift            |
| npm workspaces style                     | npm's workspace has many issues, not worth imitating |
| Reuse Cargo workspace directly           | YaoXiang and Cargo are different package ecosystems  |

## Implementation Strategy

### Phase Breakdown

| Phase    | Content                                                                                                      | Status                                                                                                                                                                                                                                    |
| -------- | ------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Phase 6a | `[workspace.members]` parsing + WorkspaceManifest                                                            | ✅ Done                                                                                                                                                                                                                                   |
| Phase 6b | Shared lockfile + merged dependency resolution (`[workspace.dependencies]` inheritance, 2026-09-29 revision) | ✅ Done                                                                                                                                                                                                                                   |
| Phase 6c | `{ workspace = "name" }` path dependency reference (pnpm-style strict visibility, 2026-09-29 revision)       | ✅ Done                                                                                                                                                                                                                                   |
| Phase 6d | Automatic path-dependency replacement at publish time                                                        | ✅ Done (with publish: replacement is materialized into the in-archive manifest at **packaging time**, while the on-disk manifest keeps the workspace form; `workspace = true` inheritance is likewise materialized as root declarations) |
| Phase 6e | Cargo workspace integration                                                                                  | Postponed                                                                                                                                                                                                                                 |

### Dependencies

- Depends on RFC-014 Phase 3 (global cache)
- Optionally depends on RFC-014b (build system, for native members)

## Open Questions

- [x] Are circular dependencies between members allowed? → **Not allowed.** Members are independent
      packages; inter-package circularity is a compile error. (RFC-029 decision, 2026-07-30)
- [x] Is workspace-level `[build]` configuration supported? → Not supported; members are
      self-contained (2026-09-15 resolution 2)
- [x] Can members have their own lockfile (overriding the root lockfile)? → Not allowed; the root
      lockfile is the only one (2026-09-15 resolution 3)
- [x] Are nested workspaces supported? → Not supported initially (2026-09-15 resolution 4)

---

## References

- [Cargo Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [npm Workspaces](https://docs.npmjs.com/cli/using-npm/workspaces)
- [pnpm Workspaces](https://pnpm.io/workspaces)

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

> This RFC is a sub-RFC of [RFC-014: Package Management System Design](014-package-manager.md).

## 2026-09-15 Review Decision

The following decisions were finalized by the owner on 2026-09-15:

1. **Implementation order moved up**: Workspace does not depend on the network or build system
   (`{ workspace = "key" }` path resolution + shared lockfile is purely local), so the schedule is
   moved up to **before** RFC-014b (overall execution order adjusted to `3 → 3.5 → 6 → 4 → 5`). The
   compiler repository itself (compiler + vscode extension + wasm + docs) is the first user.
2. **Workspace-level `[build]`**: Not supported. Adhere to the principle of "root only coordinates,
   members are self-contained"; build declarations are only written in member toml.
3. **Member-owned lockfile**: Not allowed; the root `yaoxiang.lock` is the only one.
4. **Nested workspace**: Not supported initially.

## Summary

Defines the workspace mechanism in YaoXiang: dependency sharing, path references, lockfile
unification, and Cargo workspace integration when multiple related packages are developed together.

## Motivation

As project scale grows, the code needs to be split into multiple packages. These packages need:

- Reference each other (path dependencies)
- Share external dependency versions (avoid version drift)
- Unified lockfile (guarantee build consistency)
- Cooperate with Cargo workspace (FFI parts)

### Current Issues

- Each project independently manages dependencies, cannot share
- No auto-replacement mechanism for path dependencies at publish time
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

[workspace.dependencies]        # 2026-09-29 revision: authoritative versions for shared dependencies
regex = "^1.0"
json = "^2.0"
```

**The root toml only does four things:**

1. Declare the member list (dictionary form, key is member name, value is toml path)
2. Provide a shared lockfile (`yaoxiang.lock`)
3. Provide a shared vendor directory (`.yaoxiang/vendor/`)
4. (2026-09-29 revision) Declare shared dependency versions (`[workspace.dependencies]`)

> **2026-09-29 revision: Add `[workspace.dependencies]` inheritance (the original "root toml does
> not define dependencies" restriction is relaxed).** Under strict unified version rules, friction
> from multiple members independently declaring same-named packages is foreseeable; following the
> Cargo/uv precedent, the root declares authoritative versions for shared dependencies, and members
> inherit one by one with `{ workspace = true }` (upgrading a shared version only requires changing
> the root once). Member references are still `{ workspace = "<key>" }` (string) — same key,
> different type, isomorphic with Cargo. Merged resolution: all members' deps + dev-deps are merged
> into the shared lockfile; same-named packages require computing the intersection; an empty
> intersection reports a conflict and lists the source members; inconsistent base URLs for git
> dependencies are treated as a conflict.

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
- When resolving, merge all members' dependencies and generate a shared lockfile
- Version conflicts are reported as errors when the lockfile is generated
- The same package must be resolved to the same version across different members

### Workspace Dependency References

`{ workspace = "member-name" }` references the **key** in `[workspace.members]` (not the member's
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
utils = { workspace = "utils" }   # ✅ Reference key "utils"
# Even if packages/utils/yaoxiang.toml has name = "my-utils"
```

**Why use key instead of name:**

- The key is controlled by the workspace, stable and unique
- `[package].name` is a public name, which may change at publish time
- The key is the key of a BTreeMap, inherently unique
- At publish time, workspace references are replaced with version dependencies; the key is not
  leaked into the public API

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

**Version source:** Reads the `[package].version` of the depended-on member, with a `^` prefix. Does
not check the Registry — the authoritative source of the version is the member's `yaoxiang.toml`;
the Registry is just a distribution channel.

The package manager automatically performs this replacement during `yaoxiang publish`.

### Integration with Cargo Workspace

If there are FFI packages in the workspace, you can define a Cargo workspace at the same time:

```toml
# Root Cargo.toml
[workspace]
members = ["packages/core/native", "packages/utils/native"]
```

```
my-workspace/
├── yaoxiang.toml          # YaoXiang workspace
├── Cargo.toml             # Cargo workspace (FFI parts)
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

| Command                            | Function                                                                                                 |
| ---------------------------------- | -------------------------------------------------------------------------------------------------------- |
| `yaoxiang workspace list`          | List workspace members                                                                                   |
| `yaoxiang workspace add <path>`    | Add a member (key taken from [package].name, `--as` can override; ✅ Implemented)                        |
| `yaoxiang workspace remove <name>` | Remove a member (only unregister, do not delete the directory; warn if still referenced; ✅ Implemented) |
| `yaoxiang build`                   | Build all members (sorted by dependency topology)                                                        |
| `yaoxiang build core`              | Build the specified member                                                                               |
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
    name: String,           // key from [workspace.members]
    root: PathBuf,
    manifest: PackageManifest,
}
```

**Detection logic:** When loading the toml, if there is a `[workspace]` section, it is parsed as
`WorkspaceManifest`; otherwise, parsed as `PackageManifest`.

### Workspace Dependency References

The semantics of `{ workspace = "member-name" }`:

- References another workspace member in `dependencies`
- Resolved to a local path during development
- Replaced with the Registry version at publish time
- The member name must exist in `[workspace.members]`

### Member Visibility (2026-09-29 finalized: pnpm-style strict)

A member's code's `use` only resolves from **its own declared dependencies** (versioned + path
dependencies + workspace references) — packages declared by other members in the shared vendor are
**not visible** to this member. Undeclared means uninstalled; misuse directly reports "module not
found" prompting a supplement of the declaration: ghost dependencies (the famous pitfall of npm
hoisting) are exposed at compile time, rather than becoming a bomb for consumers after the package
is published.

Member references and path dependencies resolve by path to the target package root's `src/` layout
(same vendor entry structure, no version suffix), and apply the target's import surface (RFC-029f).
Consistency checks also use the workspace root as the anchor: merged dependencies vs root lockfile
vs root vendor.

### Shared Lockfile

- The workspace has only one `yaoxiang.lock` (in the root directory)
- All members' dependency resolutions are merged into the same lockfile
- Version conflicts are reported as errors when the lockfile is generated, with information about
  the conflict source

## Trade-offs

### Advantages

- Unified management of multi-package projects
- Shared lockfile guarantees consistency
- Good development experience with path dependencies
- Seamless integration with Cargo workspace

### Disadvantages

- All members must use the same external dependency versions (may be too strict)
- The root toml cannot have its own dependencies (design constraint)
- Cargo workspace integration adds complexity

## Alternatives

| Alternative                      | Why not chosen                                       |
| -------------------------------- | ---------------------------------------------------- |
| Independent projects + path deps | Lockfile not unified, version drift risk             |
| npm workspaces-like              | npm's workspace has many issues, not worth imitating |
| Cargo workspace direct reuse     | YaoXiang and Cargo are different package ecosystems  |

## Implementation Strategy

### Phase Division

| Phase    | Content                                                                                                      | Status                                                                                                                                                                                                                             |
| -------- | ------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Phase 6a | `[workspace.members]` parsing + WorkspaceManifest                                                            | ✅ Done                                                                                                                                                                                                                            |
| Phase 6b | Shared lockfile + merged dependency resolution (`[workspace.dependencies]` inheritance, 2026-09-29 revision) | ✅ Done                                                                                                                                                                                                                            |
| Phase 6c | `{ workspace = "name" }` path dependency reference (pnpm-style strict visibility, 2026-09-29 revision)       | ✅ Done                                                                                                                                                                                                                            |
| Phase 6d | Auto-replacement of path dependencies at publish time                                                        | ✅ Done (with publish: replacement is materialized into the in-archive manifest at **pack time**; the on-disk manifest keeps the workspace form; `workspace = true` inheritance is similarly materialized to the root declaration) |
| Phase 6e | Cargo workspace integration                                                                                  | Deferred                                                                                                                                                                                                                           |

### Dependencies

- Depends on RFC-014 Phase 3 (global cache)
- Optional dependence on RFC-014b (build system, for native members)

## Open Questions

- [x] Are cyclic dependencies between members allowed? → **Not allowed.** Members are independent
      packages; cyclic dependencies between packages are compilation errors. (RFC-029 decision,
      2026-07-30)
- [x] Is workspace-level `[build]` configuration supported? → Not supported, members are
      self-contained (2026-09-15 Decision 2)
- [x] Can members have their own lockfile (overriding the root lockfile)? → Not allowed; the root
      lockfile is the only one (2026-09-15 Decision 3)
- [x] Is nested workspace supported? → Not supported initially (2026-09-15 Decision 4)

---

## References

- [Cargo Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [npm Workspaces](https://docs.npmjs.com/cli/using-npm/workspaces)
- [pnpm Workspaces](https://pnpm.io/workspaces)

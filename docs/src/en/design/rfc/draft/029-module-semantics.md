---
title: "RFC-029: Module Semantic System"
status: "Draft"
author: "晨煦"
created: "2026-06-13"
updated: "2026-07-14 (Rewrite: removed compatibility section, open issues split into sub-RFCs)"
---

# RFC-029: Module Semantic System

## Summary

Wire the module system into the compilation pipeline, enabling multi-file compilation and package-level visibility control.

**Core principle**: The type checker only queries a pre-built module registry; it never touches the disk. The module graph is fully constructed before type checking begins.

**Out of scope**: Caching, file watching, hot reload, and incremental recompilation. These are compilation-lifecycle optimizations and belong in follow-up RFCs.

## Motivation

### Current Problems

1. **The compiler only supports single files**: `Compiler::compile(name, source)` cannot handle cross-file dependencies.
2. **Export rules conflict with each other**: types auto-exported, constants auto-exported, methods auto-exported, functions check `pub`—four sets of exceptions.
3. **Two module resolvers exist**: `frontend/module/resolver.rs` and `package/source/module_resolver.rs` have different search orders.
4. **The type checker is coupled to file loading**: A previous draft required `use` to trigger `ModuleLoader::load()` during type checking.

### Design Goals

- A project can compile multiple `.yx` files.
- `use` statements have clear, unambiguous semantics.
- Visibility rules are unified into a single rule.
- Single-file mode continues to work without requiring `yaoxiang.toml`.
- The type checker is pure logic and performs no file I/O.

## Proposal

### 1. Module Identity and Path Resolution

#### Module Definition

A **module** is a `.yx` file. A module path is a dot-separated named path that corresponds to a file system location.

```
math.geometry → src/math/geometry.yx
             → src/math/geometry/mod.yx
             → src/math/geometry/index.yx
```

A **package** is a project with a `yaoxiang.toml` that contains multiple modules. The package is the sole encapsulation boundary.

#### Path Resolution Rules

Lookup order (the single rule, replacing the existing two resolvers):

1. **Standard library**: `std` or `std.*` → builtin modules, queried from `ModuleRegistry`.
2. **Vendor directory**: `.yaoxiang/vendor/<pkg>-*/src/` → dependency packages.
3. **Current file relative path**: relative to the directory of the current `.yx` file.
4. **Project src directory**: `<project_root>/src/`.

File location attempt order:

```
base/name.yx
base/name/mod.yx
base/name/index.yx
```

Stop at the first existing file. If both `name.yx` and `name/mod.yx` exist, report an error:

```
Module path ambiguity: `math.geometry` matches both:
  src/math/geometry.yx
  src/math/geometry/mod.yx
Please delete one of them.
```

#### Unified Resolver

Eliminate the existing two `ModuleResolver` implementations. Keep `frontend/module/resolver.rs` as the sole implementation and delete `package/source/module_resolver.rs`. The `YXPATH` environment variable support is merged into the single resolver.

### 2. Import Semantics

#### Syntax Forms

```yaoxiang
use math.geometry                          # module namespace
use math.geometry as geo                   # module namespace alias
use math.geometry.{Point}                  # selective import
use math.geometry.{Point, distance}        # multi-item selective import
use math.geometry.{Point as P}             # selective import with alias
use math.geometry.{Point as P, distance as dist}  # multi-item with alias
```

#### Semantics

All import forms are **compile-time name resolution rules**, not runtime reference copies. Imported names point to declaration identities in the module's export table.

| Syntax | Bound into current scope | Usage |
|------|-----------------|----------|
| `use path` | last segment of path as module namespace | `geometry.Point` |
| `use path as alias` | alias as module namespace | `alias.Point` |
| `use path.{item}` | item itself | `item` |
| `use path.{item as alias}` | alias itself | `alias` |

#### Removed Syntax

- ~~`from path use item`~~: The Python from-import form is not adopted.
- ~~`use path.*`~~: Wildcard imports carry collision risks; module-namespace imports are sufficient.
- ~~`use path.{a, b} as c, d`~~: Parallel lists paired by position are fragile data structures; aliases must follow each declaration: `use path.{a as c, b as d}`.

#### Path Semantics

`path` in `use path` is always a **module path**, not a declaration. If the module is not found, report an error directly:

```
Module `math.geometry.Point` not found.
If `Point` is a declaration in module `math.geometry`, use:
use math.geometry.{Point}
```

No "try the full module first, then treat the last segment as a declaration" fallback.

#### Import Conflicts

Same-name imports report an error directly; no silent shadowing:

```
Name `Point` import conflict:
  math.geometry.Point
  graphics.geometry.Point
Please use selective imports or a module namespace alias.
```

### 3. Visibility

#### Rules

The package is the sole encapsulation boundary. Modules do not serve as permission boundaries.

| Notation | Within current package | Other packages |
|------|:--------:|:------:|
| Default (no `pub`) | ✅ | ❌ |
| `pub` | ✅ | ✅ |

**One rule applies to all top-level declarations**: types, functions, constants, methods.

Eliminate the four existing exceptions in the code:

- ~~Type definitions are always exported~~ → same rule.
- ~~Constants are auto-exported~~ → same rule.
- ~~Methods are auto-exported~~ → same rule.
- ~~Only functions check `pub`~~ → same rule.

#### Data Structure

Replace `is_pub: bool` in the AST with:

```rust
pub enum Visibility {
    Package,  // Default: visible within the current package
    Public,   // pub: visible to all packages
}
```

#### Export Table

Each module maintains two tables:

- **PackageSymbols**: The complete symbol table within the package, containing all top-level declarations.
- **PublicExports**: The subset of `pub` declarations exposed to other packages.

A same-package `use` queries `PackageSymbols`; a cross-package `use` can only query `PublicExports`.

Cross-package references to non-`pub` declarations report an error directly:

```
`internalHelper` in module `math.geometry` is not visible.
It is not a pub declaration and can only be used within the `math` package.
```

### 4. Project Compilation Flow

#### Compilation Pipeline

```
Project entry
  → Read yaoxiang.toml to obtain the entry file
  → Recursively parse use statements from the entry to discover all dependent modules
  → Build the module dependency graph (ModuleDependencyGraph)
  → Detect circular dependencies
  → Topological sort
  → For each module in order: lex → parse → extract exports
  → Build the ModuleRegistry (containing every module's export table)
  → Type check each module in topological order (querying the ModuleRegistry)
  → Generate multiple ModuleIRs
  → Aggregate diagnostics
```

The type checker **only queries** the pre-built `ModuleRegistry`; it performs no file loading and touches no disk.

#### Entry File Selection

Priority:

1. `[run].main` (if present)
2. `path` of the first `[[bin]]` entry
3. `[lib].path`
4. `src/main.yx` (conventional default)

Single-file mode does not need `yaoxiang.toml`; it compiles the given file directly.

#### Circular Dependencies

```
Circular dependency detected:
  math.geometry → math.transform → math.geometry
```

Circular dependencies are compilation errors and are not handled specially.

#### Error Aggregation

Errors from multi-file compilation are aggregated in module topological order. Each error is annotated with its source module and file location:

```
Error: in module `math.geometry`:
  src/math/geometry.yx:12:5
  type `Circle` is not defined

Error: in module `app.main`:
  src/main.yx:3:1
  module `math.geometry` is not visible
```

### 5. Compiler Changes

| Component | Change |
|------|------|
| `compiler.rs` | Add `compile_project(project_root)` method |
| `pipeline.rs` | Keep the responsibility of single-module compilation; do not turn into a god object |
| `typecheck/checker.rs` | `use` statements query `ModuleRegistry`; do not trigger file loading |
| `typecheck/inference/statements.rs` | Same as above; `process_use_stmt` only queries, never loads |
| `frontend/module/resolver.rs` | Merge YXPATH support from `package/source/module_resolver.rs`; become the sole resolver |
| `frontend/module/loader.rs` | Extend: support recursive discovery and full module graph construction |
| `frontend/module/dep_graph.rs` | Already implemented; reuse topological sort and cycle detection |
| `frontend/module/registry.rs` | Already implemented; reuse export table queries |
| `frontend/module/cache.rs` | Already implemented; not wired into the compilation pipeline in this RFC |
| `frontend/module/hot_reload.rs` | Already implemented; not wired into the compilation pipeline in this RFC |
| AST `is_pub: bool` | Replace with `Visibility` enum |
| `package/source/module_resolver.rs` | Delete; responsibilities merged into `frontend/module/resolver.rs` |

## Implementation Strategy

### Phases

**Phase 1: Unify Module Resolution**
1. Merge the two `ModuleResolver` implementations; delete `package/source/module_resolver.rs`
2. Support the `YXPATH` environment variable
3. Module path ambiguity detection

**Phase 2: Visibility Data Structure**
4. AST `is_pub: bool` → `Visibility` enum
5. Parser maps the `pub` keyword to `Visibility::Public`
6. `ModuleLoader::extract_exports` uniformly uses `Visibility` to determine exports

**Phase 3: Project Compilation Entry**
7. `compiler.rs` adds `compile_project(project_root)` method
8. Recursively discover modules from the entry; build the `ModuleDependencyGraph`
9. Topologically sort, load modules in order, and extract exports
10. Build the full `ModuleRegistry`
11. Type check each module in topological order
12. Generate multiple `ModuleIR`s and aggregate diagnostics

**Phase 4: Import Syntax**
13. Implement the `use path.{item as alias}` syntax
14. Eliminate the trailing-path fallback guess

### Dependencies

- RFC-014 (Package Manager) — package names come from `yaoxiang.toml`; vendor directory layout.
- RFC-011 (Generic Type System) — traits are structural types and are unrelated to module ownership.
- RFC-009 (Ownership Model) — module imports are compile-time name resolution and involve no runtime reference copies.

## Sub-RFC Planning

The following sub-RFCs are in the **anticipated plan**; drafting has not yet begun:

| Sub-RFC | Capability (anticipated) | Prerequisite (anticipated) |
|--------|-------------|-----------------|
| 029a | Module cache and incremental recompilation | Module graph and export table stable |
| 029b | File watching and hot reload | Cache invalidation mechanism from 029a |
| 029c | Re-export (`pub use`) | Export table and visibility rules landed |
| 029d | CLI flag `--entry` to override entry selection | Project compilation entry available |
| 029e | Multi-file diagnostic `--json` output format | Diagnostic aggregation mechanism available |
| — | `pub(package)` module-private visibility | No real-world demand yet; not included for now |
| — | Workspace multi-package compilation | Carried by RFC-014c |

## References

- [RFC-009: Ownership Model](accepted/009-ownership-model.md) — Move semantics; imports are compile-time name resolution.
- [RFC-011: Generic Type System](accepted/011-generic-type-system.md) — Structural type definitions.
- [RFC-014: Package Manager System Design](accepted/014-package-manager.md) — Package name sources; vendor directory.
- [RFC-015: Configuration System](accepted/015-configuration-system.md) — `yaoxiang.toml` field definitions.
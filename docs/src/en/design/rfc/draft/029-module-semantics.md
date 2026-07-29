---
title: 'RFC-029: Module Semantics System'
status: 'Draft'
author: 'Chen Xu'
created: '2026-06-13'
updated: '2026-07-14 (Rewritten: removed compatibility section, open issues split into sub-RFCs)'
---

# RFC-029: Module Semantics System

## Summary

Integrate the module system into the compilation pipeline to enable multi-file compilation and
package-level visibility control.

**Core Principle**: The type checker only queries the pre-built module registry and does not touch
the disk. The module graph is fully constructed before type checking.

**Not Included**: Caching, file watching, hot reload, incremental recompilation. These are
compilation lifecycle optimizations and belong in subsequent independent RFCs.

## Motivation

### Current Problems

1. **Compiler only supports single file**: `Compiler::compile(name, source)` cannot handle
   cross-file dependencies
2. **Export rules conflict with each other**: types auto-export, constants auto-export, methods
   auto-export, functions check `pub` — four sets of exceptions
3. **Two module resolvers**: `frontend/module/resolver.rs` and `package/source/module_resolver.rs`
   have different search orders
4. **Type checker coupled with file loading**: Draft previously required `use` to trigger
   `ModuleLoader::load()` during type checking

### Design Goals

- A project can compile multiple `.yx` files
- The semantics of `use` statements are clear and unambiguous
- Visibility rules unified into a single rule
- Single-file mode continues to work without requiring `yaoxiang.toml`
- Type checker is pure logic and does not perform file I/O

## Proposal

### 1. Module Identity and Path Resolution

#### Module Definition

A **module** is a `.yx` file. A module path is a dot-separated naming path that corresponds to a
filesystem location.

```
math.geometry → src/math/geometry.yx
             → src/math/geometry/mod.yx
             → src/math/geometry/index.yx
```

A **package** is a project with `yaoxiang.toml`, containing multiple modules. The package is the
only encapsulation boundary.

#### Path Resolution Rules

Search order (single rule, replacing the existing two resolvers):

1. **Standard library**: `std` or `std.*` → built-in modules, queried from `ModuleRegistry`
2. **vendor directory**: `.yaoxiang/vendor/<pkg>-*/src/` → dependency packages
3. **Current file relative path**: relative to the current `.yx` file's directory
4. **Project src directory**: `<project_root>/src/`

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

Eliminate the existing two `ModuleResolver` implementations. Keep `frontend/module/resolver.rs` as
the sole implementation and delete `package/source/module_resolver.rs`. The `YXPATH` environment
variable support is merged into the sole resolver.

### 2. Import Semantics

#### Syntax Forms

```yaoxiang
use math.geometry                          # module namespace
use math.geometry as geo                   # module namespace with alias
use math.geometry.{Point}                  # selective import
use math.geometry.{Point, distance}        # multiple selective import
use math.geometry.{Point as P}             # selective import with alias
use math.geometry.{Point as P, distance as dist}  # multiple with aliases
```

#### Semantics

All import forms are **compile-time name resolution rules**, not runtime reference copies. Imported
names point to declared identities in the module's export table.

| Syntax                     | Bound to current scope            | Usage            |
| -------------------------- | --------------------------------- | ---------------- |
| `use path`                 | last segment of path as namespace | `geometry.Point` |
| `use path as alias`        | alias as module namespace         | `alias.Point`    |
| `use path.{item}`          | item itself                       | `item`           |
| `use path.{item as alias}` | alias itself                      | `alias`          |

#### Deleted Syntax

- ~~`from path use item`~~: Python from-import style not adopted
- ~~`use path.*`~~: wildcard imports introduce conflict risks; module namespace imports are
  sufficient
- ~~`use path.{a, b} as c, d`~~: positional pairing of parallel lists is fragile; aliases must
  follow each declaration: `use path.{a as c, b as d}`

#### Path Semantics

The `path` in `use path` is always a **module path**, not a declaration. If the module cannot be
found, report an error directly:

```
Module `math.geometry.Point` not found.
If `Point` is a declaration in module `math.geometry`, use:
use math.geometry.{Point}
```

No "try to find the complete module first, if that fails treat the last segment as a declaration"
fallback.

#### Import Conflicts

Same-name imports report an error directly without silent override:

```
Name `Point` import conflict:
  math.geometry.Point
  graphics.geometry.Point
Please use selective imports or module namespace aliases.
```

### 3. Visibility

#### Rules

The package is the only encapsulation boundary. Modules do not bear permission boundaries.

| Syntax             | Current package | Other packages |
| ------------------ | :-------------: | :------------: |
| Default (no `pub`) |       ✅        |       ❌       |
| `pub`              |       ✅        |       ✅       |

**Single rule applies to all top-level declarations**: types, functions, constants, methods.

Eliminate the four sets of exceptions in the existing code:

- ~~Types always exported~~ → same rule
- ~~Constants auto-exported~~ → same rule
- ~~Methods auto-exported~~ → same rule
- ~~Only functions check `pub`~~ → same rule

#### Data Structures

Replace `is_pub: bool` in AST with:

```rust
pub enum Visibility {
    Package,  // Default: visible within current package
    Public,   // pub: visible to all packages
}
```

#### Export Tables

Each module maintains two tables:

- **PackageSymbols**: complete symbol table for the package, containing all top-level declarations
- **PublicExports**: subset of `pub` declarations provided to other packages

Same-package `use` queries `PackageSymbols`; cross-package `use` can only query `PublicExports`.

Cross-package references to non-`pub` declarations report an error directly:

```
`internalHelper` in module `math.geometry` is not visible.
It is not a pub declaration and can only be used within the `math` package.
```

### 4. Project Compilation Flow

#### Compilation Pipeline

```
Project entry
  → Read yaoxiang.toml to get entry file
  → Recursively parse use statements from entry to discover all dependent modules
  → Build module dependency graph (ModuleDependencyGraph)
  → Detect circular dependencies
  → Topological sort
  → For each module in order: lexical analysis → syntax analysis → extract exports
  → Build ModuleRegistry (contains export tables for all modules)
  → Type-check each module in topological order (query ModuleRegistry)
  → Generate multiple ModuleIRs
  → Aggregate diagnostics
```

The type checker **only queries** the pre-built `ModuleRegistry`, does not perform file loading, and
does not touch the disk.

#### Entry File Selection

Priority order:

1. `[run].main` (if it exists)
2. First entry in `[[bin]]` `path`
3. `[lib].path`
4. `src/main.yx` (convention default)

Single-file mode does not require `yaoxiang.toml` and compiles the given file directly.

#### Circular Dependencies

```
Circular dependency detected:
  math.geometry → math.transform → math.geometry
```

Circular dependencies are compilation errors and receive no special treatment.

#### Error Aggregation

Multi-file compilation errors are aggregated in module topological order. Each error is annotated
with the source module and file location:

```
Error in module `math.geometry`:
  src/math/geometry.yx:12:5
  Type `Circle` is not defined

Error in module `app.main`:
  src/main.yx:3:1
  Module `math.geometry` is not visible
```

### 5. Compiler Changes

| Component                           | Changes                                                                             |
| ----------------------------------- | ----------------------------------------------------------------------------------- |
| `compiler.rs`                       | Add `compile_project(project_root)` method                                          |
| `pipeline.rs`                       | Maintain single-module compilation responsibility, avoid becoming god object        |
| `typecheck/checker.rs`              | `use` statements query `ModuleRegistry`, do not trigger file loading                |
| `typecheck/inference/statements.rs` | Same as above, `process_use_stmt` only queries, does not load                       |
| `frontend/module/resolver.rs`       | Merge YXPATH support from `package/source/module_resolver.rs`, become sole resolver |
| `frontend/module/loader.rs`         | Extend: support recursive discovery, build complete module graph                    |
| `frontend/module/dep_graph.rs`      | Already implemented, reuse topological sort and cycle detection                     |
| `frontend/module/registry.rs`       | Already implemented, reuse export table queries                                     |
| `frontend/module/cache.rs`          | Already implemented, not connected to compilation pipeline in this RFC              |
| `frontend/module/hot_reload.rs`     | Already implemented, not connected to compilation pipeline in this RFC              |
| AST `is_pub: bool`                  | Replace with `Visibility` enum                                                      |
| `package/source/module_resolver.rs` | Delete, responsibilities merged into `frontend/module/resolver.rs`                  |

## Implementation Strategy

### Phase Division

**Phase 1: Unify Module Resolution**

1. Merge the two `ModuleResolver` implementations, delete `package/source/module_resolver.rs`
2. Support `YXPATH` environment variable
3. Module path ambiguity detection

**Phase 2: Visibility Data Structures** 4. AST `is_pub: bool` → `Visibility` enum 5. Parser supports
`pub` keyword mapping to `Visibility::Public` 6. `ModuleLoader::extract_exports` uniformly uses
`Visibility` to determine exports

**Phase 3: Project Compilation Entry** 7. `compiler.rs` adds `compile_project(project_root)`
method 8. Recursively discover modules from entry, build `ModuleDependencyGraph` 9. Topological
sort, load modules in order and extract exports 10. Build complete `ModuleRegistry` 11. Type-check
each module in topological order 12. Generate multiple `ModuleIRs`, aggregate diagnostics

**Phase 4: Import Syntax** 13. Implement `use path.{item as alias}` syntax 14. Eliminate path ending
fallback guessing

### Dependencies

- RFC-014 (Package Manager) — package names from `yaoxiang.toml`, vendor directory structure
- RFC-011 (Generics System) — traits are structured types, not involving module ownership
- RFC-009 (Ownership Model) — module imports are compile-time name resolution, not involving runtime
  reference copies

## Sub-RFC Planning

The following sub-RFCs are **planned** and not yet started:

| Sub-RFC | Planned Capability                               | Planned Prerequisites                        |
| ------- | ------------------------------------------------ | -------------------------------------------- |
| 029a    | Module caching and incremental recompilation     | Stable module graph and export tables        |
| 029b    | File watching and hot reload                     | Cache invalidation mechanism from 029a       |
| 029c    | Re-exports (`pub use`)                           | Export tables and visibility rules in place  |
| 029d    | CLI argument `--entry` overrides entry selection | Project compilation entry available          |
| 029e    | Multi-file diagnostics `--json` output format    | Diagnostic aggregation mechanism available   |
| —       | `pub(package)` module-private visibility         | No current real demand, not included for now |
| —       | Workspace multi-package compilation              | Carried by RFC-014c                          |

## References

- [RFC-009: Ownership Model](accepted/009-ownership-model.md) — Move semantics, imports are
  compile-time name resolution
- [RFC-011: Generic Type System](accepted/011-generic-type-system.md) — Structured type definitions
- [RFC-014: Package Management System Design](accepted/014-package-manager.md) — Package name
  sources, vendor directory
- [RFC-015: Configuration System](accepted/015-configuration-system.md) — `yaoxiang.toml` field
  definitions

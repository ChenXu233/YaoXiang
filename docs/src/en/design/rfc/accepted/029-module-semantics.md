---
title: 'RFC-029: Module Semantics System'
status: 'Accepted'
author: 'Chenxu'
created: '2026-06-13'
updated:
  '2026-07-30 (Rewritten: Based on review discussion, established module=record semantics, removed
  visibility mechanism)'
issue: '#232'
---

# RFC-029: Module Semantics System

## Summary

Integrate the module system into the compilation pipeline to enable multi-file compilation.

**Core definition**: Module = all top-level bindings of a `.yx` file. Module's type = the types of
those bindings (inferred). `use` = record destructuring. No `pub`, no `private`, no `export`, no
visibility mechanism.

**Core principles**:

- The type checker only queries the pre-built ModuleRegistry; it does not touch the disk
- Files within a package are merged into a single compilation unit (AST splicing); cyclic references
  within a package are naturally allowed
- Registry loads on demand: only modules reachable from the entry point along `use`

**Not included**: caching, file watching, hot reloading, incremental recompilation, cross-package
cyclic dependency handling.

## Motivation

### Current Problems

1. **Compiler only supports single file**: `Pipeline::run(name, source)` takes a string and cannot
   handle cross-file dependencies
2. **`use` can only resolve std modules**: `use` between local files all report "Unknown variable"
   (#232)
3. **Module resolver is in the wrong place**: The only path resolution logic is in
   `package/source/module_resolver.rs`; `frontend/module/resolver.rs` is actually compile-time
   predicate strictification (RFC-027)

### Design Goals

- A project can compile multiple `.yx` files
- `use` statement semantics are clear: record destructuring, not a special mechanism
- Single file continues to work, no `yaoxiang.toml` required
- Pipeline (`Pipeline`) unchanged; multi-file support is the orchestrator's job
- No new keywords, no new AST nodes, no new concepts

## Proposal

### 1. Module = record of bindings

A **module** is all top-level bindings of a `.yx` file.

```yaoxiang
// math/geometry.yx
Point: Type = { x: Float, y: Float }
distance: (a: Point, b: Point) -> Float = { ... }
```

The content of this module is `{ Point: Type, distance: (Point, Point) -> Float }`.

A module is not a special entity. It is an instance of the `name: type = value` model—a record that
happens to be defined at the file boundary. The module's type is inferred from its bindings;
explicit annotation is never required.

Bindings introduced by `use` are **also** part of the module's content:

```yaoxiang
// math/mod.yx
use geometry.{Point, distance}
```

The content of the `math` module = `{ Point: Type, distance: (Point, Point) -> Float }`. External
`use math.{Point}` can obtain it. `use math.geometry.{Point}` can also obtain it. Both paths point
to the same binding.

### 2. use = record destructuring

All `use` forms are record field access + binding:

```yaoxiang
use math.geometry.{Point, distance}
```

Is equivalent to:

```yaoxiang
Point = math.geometry.Point
distance = math.geometry.distance
```

| Syntax              | Semantics                                                                  |
| ------------------- | -------------------------------------------------------------------------- |
| `use path.{item}`   | Take the `item` field of the `path` record, bind it into the current scope |
| `use path.{a, b}`   | Take multiple fields                                                       |
| `use path`          | Take the `path` record itself, bind it to the last segment's name          |
| `use path as alias` | Take the `path` record itself, bind it to `alias`                          |

#### Non-existent Syntax

- ~~`use path.*`~~: Wildcard import. Not needed; list bindings explicitly.
- ~~`from path use item`~~: Python-style. Not adopted.
- ~~`use path.{item as alias}`~~: Alias inside braces. Optional in Phase 4, does not block #232.

#### Import Conflicts

Same-named bindings directly report an error:

```
Name `Point` conflicts:
  math.geometry.Point
  graphics.shapes.Point
Please use a different name or a module alias.
```

### 3. Visibility: Does Not Exist

**This RFC does not introduce any visibility mechanism.** All top-level bindings are visible to any
code that can write a path.

This is an intentional design decision, not an oversight.

#### Design Rationale

| Want to express             | How to do it                      | Mechanism    |
| --------------------------- | --------------------------------- | ------------ |
| "This is API"               | Put it in the published package   | Distribution |
| "This is internal"          | Put it in a non-published package | Distribution |
| "This is inside a function" | Write it inside the function body | Scope        |

All three layers use existing mechanisms: package, file, scope. Nothing new is needed.

#### Why No `pub`

- Things you don't want others to use shouldn't be at the top level (put them in local scope)
- Helper functions shared across multiple files go into a separate, non-published package
- "Whether it can be defended against" and "Whether there should be a signal" are two different
  things. At the current stage, with no third-party ecosystem, signals are meaningless
- The door is not locked. Using the door is polite; climbing over the wall is free. The language
  does not concern itself with politeness

#### Future

When the ecosystem matures and enforced boundaries are needed, they can be introduced via a separate
RFC. Adding restrictions is backward-compatible (default public → explicitly mark as internal). But
this RFC neither presupposes that direction nor promises it will come.

### 4. Path Resolution

#### Module Path → File

```
use math.geometry.{Point}
```

Lookup order:

1. **Registry-registered module**: `math.geometry` is already in the Registry → use it directly
2. **Standard library**: `std` or `std.*` → builtin modules
3. **Importer's directory**: `<importer_dir>/math/geometry.yx` (local module takes priority)
4. **Project root** (nearest `yaoxiang.toml` ancestor): `<project_root>/math/geometry.yx`
5. **vendor directory**: `.yaoxiang/vendor/<pkg>-*/src/` (future)

File location attempt order:

```
base/name.yx
base/name/mod.yx
```

Stop at the first match. If both exist → report error:

```
Module path ambiguity: `math.geometry` matches both:
  src/math/geometry.yx
  src/math/geometry/mod.yx
Please remove one of them.
```

If the same module key hits a file in **two roots** and both are referenced (e.g., `tests/lib.yx`
and `<root>/lib.yx` are referenced by the tests/ entry and the root entry respectively) → same
ambiguity error, not silent shadowing.

> Revised 2026-08-03 (#247 / RFC-036 driven): Discovery and resolution land by implementation.
> Discovery follows `use` tracing (the protocol established in this RFC §5), replacing the directory
> recursion of the initial implementation—compilation errors in unrelated files no longer block
> execution, which is what makes test file isolation in `yaoxiang test` work. "Importer directory
> takes priority" in the dual-root rule preserves same-directory project behavior; "project root as
> fallback" enables subdirectory entries (e.g., `tests/foo_test.yx`) to import project root modules.
> The `src/` layout (RFC-014 packages) is handled in the vendor layer, not affecting the local dual
> root.

#### mod.yx = Directory Entry (Convention)

`mod.yx` is the directory's entry file. `use math` loads `src/math/mod.yx`.

This is a **convention, not a mandate**. Users can directly `use math.geometry` to penetrate into
sub-files. `mod.yx` is the "recommended entry" (door plate), not the "sole entry" (lock).

#### Unified Resolver

Currently the only path resolution logic is in `package/source/module_resolver.rs`. Move it to
`frontend/module/resolver.rs` (replacing the current misnamed predicate strictification file, which
moves to `frontend/core/types/eval/`).

### 5. Project Compilation Flow

#### Path A: Merge AST

All files within a package are merged into a **single compilation unit**. Pipeline unchanged.

```
Orchestrator (above Pipeline):
  1. Determine entry file
  2. Parse use statements in the entry file (read use lines only, not function bodies)
  3. Discover files along the use path, add to queue
  4. Parse use statements of files in the queue
  5. Repeat 3-4 until queue is empty (on-demand discovery)
  6. Fully parse all discovered files individually → multiple ASTs
  7. Merge into one Module (all top-level items spliced, Span retains source file)
  8. Feed to Pipeline::run() (pipeline doesn't know there are multiple files)
```

#### In-Package Cyclic References: Allowed

Because all files are merged into one AST, in-package `use` between files is equivalent to mutual
references within the same file:

```yaoxiang
// tree.yx
use node.{Node}
Tree: Type = { root: Node }

// node.yx
use tree.{Tree}
Node: Type = { value: Int, parent: Tree }
```

After merging, it becomes two mutually-referencing type definitions within a single AST. The
compiler already supports this.

#### Cross-Package Cycles: Later

Packages are distribution units and require topological order between them. With no third-party
package ecosystem at present, this is not handled. Encountering such a case directly reports an
error.

#### Entry File Selection

Priority:

1. `[run].main` (yaoxiang.toml)
2. `path` of the first `[[bin]]` item
3. `src/main.yx` (conventional default)

When no `yaoxiang.toml`: directly compile the given file. Registry contains only std. This is not
"single-file mode"—it is the "natural result of an empty discovery".

#### Registry Loads On Demand

The Registry's content = all modules reachable from the entry along `use`. Unreachable modules are
not parsed, not registered, do not exist. **This is not an optimization; it is the definition.**

### 6. std Modules and User Modules Are Homogeneous

From the typechecker's perspective, `use std.io.{println}` and `use math.geometry.{Point}` operate
identically:

1. Find the module record in the Registry
2. Take the field
3. Bind to the current scope

Source (Std / User / Vendor) is metadata and does not affect resolution logic. Special handling of
native functions is deferred to the IR gen / codegen layer.

## Compiler Changes

| Component                                    | Change                                                                                                                                                                                               |
| -------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `frontend/module/resolver.rs`                | **Rewrite**: Currently predicate strictification (RFC-027), move to `frontend/core/types/eval/`. This file becomes the real module path resolver (migrated from `package/source/module_resolver.rs`) |
| `frontend/module/mod.rs`                     | Extend: supplement source file tracking needed for merging ASTs (Span carries filename)                                                                                                              |
| `frontend/module/registry.rs`                | Extend: support registering user modules (currently only registers std)                                                                                                                              |
| `frontend/module/orchestrator.rs`            | **New**: multi-file orchestrator (discover → parse → merge → call Pipeline)                                                                                                                          |
| `frontend/pipeline.rs`                       | **Unchanged**                                                                                                                                                                                        |
| `frontend/core/parser/statements/imports.rs` | Unchanged (`use` parsing already implemented)                                                                                                                                                        |
| `package/source/module_resolver.rs`          | **Delete**, logic migrated to `frontend/module/resolver.rs`                                                                                                                                          |
| `frontend/core/typecheck/`                   | `use` handling changed to query Registry (currently only queries std)                                                                                                                                |
| AST `is_pub: bool`                           | **Untouched**. This RFC does not concern visibility                                                                                                                                                  |

### Files That Do Not Exist (RFC old version claimed "implemented" but actually do not exist)

- ~~`frontend/module/loader.rs`~~ — does not exist; responsibility belongs to the orchestrator
- ~~`frontend/module/dep_graph.rs`~~ — does not exist; packages do not need topological sort (merge
  AST)
- ~~`frontend/module/cache.rs`~~ — does not exist; belongs to sub-RFC 029a
- ~~`frontend/module/hot_reload.rs`~~ — does not exist; belongs to sub-RFC 029b

## Implementation Strategy

### Phase 1: Unified Path Resolution

1. Move predicate strictification from `frontend/module/resolver.rs` to `frontend/core/types/eval/`
2. Migrate path resolution logic from `package/source/module_resolver.rs` to
   `frontend/module/resolver.rs`
3. Module path ambiguity detection (`name.yx` and `name/mod.yx` both exist → error)

### Phase 2: Multi-File Orchestrator

4. Create `frontend/module/orchestrator.rs`
5. Implement on-demand discovery (recursion from entry along use)
6. Implement AST merging (multi-file item splicing, Span carries source file)
7. Add `compile_project(project_root)` in `compiler.rs` that calls the orchestrator

### Phase 3: use Name Resolution

8. Change typecheck's `process_use_stmt` to query Registry (no longer only std)
9. Import conflict detection (same name → error)
10. E2E test: multi-file project `use` local modules

### Phase 4 (Optional, Does Not Block #232)

11. `use path.{item as alias}` alias inside braces
12. vendor directory resolution (in conjunction with RFC-014)

### Dependencies

- RFC-014 (package manager) — `yaoxiang.toml` fields, vendor directory structure (needed in Phase 4)
- No other prerequisites

## Sub-RFC Planning

| Sub-RFC | Capability                                   | Prerequisite           |
| ------- | -------------------------------------------- | ---------------------- |
| 029a    | Module caching and incremental recompilation | Orchestrator stable    |
| 029b    | File watching and hot reloading              | 029a                   |
| 029d    | CLI `--entry` override entry                 | Orchestrator available |
| 029e    | Multi-file diagnostic `--json` output        | Diagnostic aggregation |

Deleted: ~~029c (re-export)~~ — not needed. `use` itself is re-export; there is no "pub use"
concept.

## Design Decision Records

| Decision               | Conclusion                                | Date       | Basis                                                                                            |
| ---------------------- | ----------------------------------------- | ---------- | ------------------------------------------------------------------------------------------------ |
| What a module is       | A record of file top-level bindings       | 2026-07-30 | RFC-010 `name: type = value` unified model                                                       |
| `use` semantics        | Record destructuring                      | 2026-07-30 | No new mechanism, reuse existing record semantics                                                |
| Visibility             | Does not exist                            | 2026-07-30 | Scope + distribution boundaries cover all scenarios, no new keyword needed                       |
| `pub` keyword          | Not wanted                                | 2026-07-30 | "If you don't want others to use it, don't put it at the top level / don't publish that package" |
| mod.yx semantics       | Directory entry (convention, not mandate) | 2026-07-30 | Python `__init__.py` model: the door is not locked                                               |
| Module type annotation | Not needed                                | 2026-07-30 | Internal bindings already carry types, annotation is redundant                                   |
| In-package cycles      | Allowed (merge AST)                       | 2026-07-30 | Path A: pipeline unchanged, in-Rust-crate model                                                  |
| Cross-package cycles   | Not handled for now                       | 2026-07-30 | No third-party ecosystem; report error on encounter                                              |
| Registry loading       | On demand (only reachable modules)        | 2026-07-30 | Not an optimization, it is the definition                                                        |
| Single file vs project | Same mechanism                            | 2026-07-30 | Registry content differs, lookup logic is the same                                               |

## References

- [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md) — `name: type = value`
  model
- [RFC-009: Ownership Model](../accepted/009-ownership-model.md) — Imports are compile-time name
  resolution
- [RFC-011: Generic Type System](../accepted/011-generic-type-system.md) — Structural types
- [RFC-014: Package Management System Design](../accepted/014-package-manager.md) — Package names,
  vendor directory
- [RFC-026: FFI Core Mechanism](../accepted/026-ffi-core-mechanism.md) — StdModule registration
- [RFC-030: assert Mechanism](../accepted/030-assert-mechanism.md) — StdModule unified registration
  precedent

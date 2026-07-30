---
title: 'RFC-029: Module Semantic System'
status: 'Accepted'
author: 'Chenxu'
created: '2026-06-13'
updated:
  '2026-07-30 (Rewritten: based on review discussions, established module = record semantics,
  removed visibility mechanism)'
issue: '#232'
---

# RFC-029: Module Semantic System

## Summary

Integrate the module system into the compilation pipeline to enable multi-file compilation.

**Core definition**: A module = all top-level bindings of a `.yx` file. The type of a module = the
types of those bindings (inferred). `use` = record destructuring. No `pub`, no `private`, no
`export`, no visibility mechanism.

**Core principles**:

- The type checker only queries the pre-built ModuleRegistry, never touches the disk
- Files within a package are merged into a single compilation unit (AST splicing), and circular
  references within a package are naturally allowed
- Registry loads on demand: only modules reachable from the entry point along `use` chains are
  loaded

**Not included**: caching, file watching, hot reloading, incremental recompilation, cross-package
circular dependency handling.

## Motivation

### Current Problems

1. **The compiler only supports single files**: `Pipeline::run(name, source)` takes a string and
   cannot handle cross-file dependencies
2. **`use` can only resolve std modules**: All local `use` between files report "Unknown variable"
   (#232)
3. **The module resolver is in the wrong place**: The only path resolution logic lives in
   `package/source/module_resolver.rs`, while `frontend/module/resolver.rs` is actually compile-time
   predicate strictification (RFC-027)

### Design Goals

- A project can compile multiple `.yx` files
- The semantics of `use` statements are clear: record destructuring, not a special mechanism
- Single files continue to work without requiring `yaoxiang.toml`
- Zero changes to the pipeline (`Pipeline`); multi-file support is the orchestrator's job
- No new keywords, no new AST nodes, no new concepts

## Proposal

### 1. Module = record of bindings

A **module** is all the top-level bindings of a `.yx` file.

```yaoxiang
// math/geometry.yx
Point: Type = { x: Float, y: Float }
distance: (a: Point, b: Point) -> Float = { ... }
```

The contents of this module are `{ Point: Type, distance: (Point, Point) -> Float }`.

A module is not a special entity. It is an instance of the `name: type = value` model — a record
that happens to be defined at a file boundary. The type of the module is inferred from its bindings;
explicit annotation is never required.

Bindings introduced by `use` are **also** part of the module's contents:

```yaoxiang
// math/mod.yx
use geometry.{Point, distance}
```

The contents of the `math` module = `{ Point: Type, distance: (Point, Point) -> Float }`. An
external `use math.{Point}` can fetch it. `use math.geometry.{Point}` can also fetch it. Both paths
point to the same binding.

### 2. use = record destructuring

All `use` forms are record field access + binding:

```yaoxiang
use math.geometry.{Point, distance}
```

is equivalent to:

```yaoxiang
Point = math.geometry.Point
distance = math.geometry.distance
```

| Syntax              | Semantics                                                             |
| ------------------- | --------------------------------------------------------------------- |
| `use path.{item}`   | Take the `item` field of the `path` record, bind to the current scope |
| `use path.{a, b}`   | Take multiple fields                                                  |
| `use path`          | Take the `path` record itself, bind to its last-segment name          |
| `use path as alias` | Take the `path` record itself, bind to `alias`                        |

#### Non-existent syntaxes

- ~~`use path.*`~~: wildcard import. Not needed; list bindings explicitly.
- ~~`from path use item`~~: Python-style. Not adopted.
- ~~`use path.{item as alias}`~~: in-brace alias. Optional for Phase 4; does not block #232.

#### Import conflicts

Same-name bindings report an error directly:

```
Name `Point` conflicts:
  math.geometry.Point
  graphics.shapes.Point
Please use different names or module aliases.
```

### 3. Visibility: does not exist

**This RFC introduces no visibility mechanism.** All top-level bindings are visible to any code that
can write a path to them.

This is a deliberate design decision, not an oversight.

#### Design rationale

| What you want to express | How to do it                     | Mechanism    |
| ------------------------ | -------------------------------- | ------------ |
| "This is the API"        | Put it in the published package  | Distribution |
| "This is internal"       | Put it in an unpublished package | Distribution |
| "This is function-local" | Put it in the function body      | Scope        |

All three layers are existing mechanisms: package, file, scope. Nothing new is needed.

#### Why not `pub`

- Things you don't want others to use shouldn't be at the top level (put them in local scope)
- Helpers shared across multiple files go in a separate unpublished package
- "Whether it can be blocked" and "Whether there should be a signal" are two different questions. At
  the current stage there is no third-party ecosystem, so a signal is meaningless
- Doors aren't locked. Walking through the door is polite; climbing over the wall is your freedom.
  The language does not police politeness

#### Future

When the ecosystem matures and enforced boundaries are needed, they can be introduced via a separate
RFC. Adding restrictions is backward-compatible (default-public → explicitly marked as internal).
But this RFC neither presupposes that direction nor promises it will come.

### 4. Path Resolution

#### Module path → file

```
use math.geometry.{Point}
```

Lookup order (the only rule):

1. **Registry already has this module**: `math.geometry` is already in the Registry → use it
   directly
2. **Standard library**: `std` or `std.*` → builtin module
3. **Project src directory**: `<project_root>/src/math/geometry.yx`
4. **vendor directory**: `.yaoxiang/vendor/<pkg>-*/src/` (future)

File location attempt order:

```
base/name.yx
base/name/mod.yx
```

Stop at the first match. If both exist → error:

```
Module path ambiguous: `math.geometry` matches both:
  src/math/geometry.yx
  src/math/geometry/mod.yx
Please remove one of them.
```

#### mod.yx = directory entry (convention)

`mod.yx` is a directory's entry file. When you `use math`, it loads `src/math/mod.yx`.

This is a **convention, not a requirement**. Users can directly `use math.geometry` to drill through
to subfiles. `mod.yx` is the "recommended entry" (a house number), not the "only entry" (a lock).

#### Unified resolver

The only existing path resolution logic is in `package/source/module_resolver.rs`. Move it to
`frontend/module/resolver.rs` (replacing the current misnamed predicate strictification file;
predicate strictification moves to `frontend/core/types/eval/`).

### 5. Project Compilation Flow

#### Path A: Merge ASTs

All files within a package are merged into a **single compilation unit**. Zero changes to the
pipeline.

```
Orchestrator (above Pipeline):
  1. Determine the entry file
  2. Parse the entry file's use statements (read use lines only, do not parse function bodies)
  3. Discover files along use paths, add them to the queue
  4. For files in the queue, parse their use statements
  5. Repeat 3-4 until the queue is empty (on-demand discovery)
  6. Fully parse all discovered files individually → multiple ASTs
  7. Merge into one Module (concatenate all top-level items, Span preserves the source file)
  8. Feed to Pipeline::run() (the pipeline does not know there are multiple files)
```

#### Circular references within a package: allowed

Because all files are merged into one AST, mutual `use` between files within a package is equivalent
to mutual references within the same file:

```yaoxiang
// tree.yx
use node.{Node}
Tree: Type = { root: Node }

// node.yx
use tree.{Tree}
Node: Type = { value: Int, parent: Tree }
```

After merging, this is just two mutually referencing type definitions in one AST. The compiler
already supports this.

#### Cross-package circularity: TBD

Packages are the unit of distribution, so cross-package dependencies need a topological order. There
is currently no third-party package ecosystem, so this is not handled for now. If encountered,
simply report an error.

#### Entry file selection

Priority:

1. `[run].main` (yaoxiang.toml)
2. `path` of the first `[[bin]]` entry
3. `src/main.yx` (default convention)

When there is no `yaoxiang.toml`: compile the given file directly. The Registry only contains std.
This is not "single-file mode" — it is the "natural result of an empty discovery".

#### Registry loads on demand

The contents of the Registry = all modules reachable from the entry point along `use` chains.
Unreachable modules are not parsed, not registered, do not exist. **This is not an optimization; it
is the definition.**

### 6. std modules and user modules are homogeneous

For typecheck, `use std.io.{println}` and `use math.geometry.{Point}` operate identically:

1. Find the module record in the Registry
2. Take the field
3. Bind to the current scope

The source (Std / User / Vendor) is metadata that does not affect resolution logic. Special handling
of native functions is deferred to the IR gen / codegen layer.

## Compiler Changes

| Component                                    | Change                                                                                                                                                                                               |
| -------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `frontend/module/resolver.rs`                | **Rewrite**: currently predicate strictification (RFC-027), move to `frontend/core/types/eval/`. This file becomes the real module path resolver (migrated from `package/source/module_resolver.rs`) |
| `frontend/module/mod.rs`                     | Extend: add source-file tracking needed for AST merging (Span carries filename)                                                                                                                      |
| `frontend/module/registry.rs`                | Extend: support registering user modules (currently only std is registered)                                                                                                                          |
| `frontend/module/orchestrator.rs`            | **New**: multi-file orchestrator (discover → parse → merge → call Pipeline)                                                                                                                          |
| `frontend/pipeline.rs`                       | **No change**                                                                                                                                                                                        |
| `frontend/core/parser/statements/imports.rs` | No change (`use` parsing is already implemented)                                                                                                                                                     |
| `package/source/module_resolver.rs`          | **Delete**, logic migrated to `frontend/module/resolver.rs`                                                                                                                                          |
| `frontend/core/typecheck/`                   | `use` handling switches to querying the Registry (currently only queries std)                                                                                                                        |
| AST `is_pub: bool`                           | **No change**. This RFC does not address visibility                                                                                                                                                  |

### Non-existent files (the old RFC claimed "implemented" but they do not exist)

- ~~`frontend/module/loader.rs`~~ — does not exist, responsibilities belong to the orchestrator
- ~~`frontend/module/dep_graph.rs`~~ — does not exist, intra-package topological sorting is not
  needed (AST merging)
- ~~`frontend/module/cache.rs`~~ — does not exist, belongs to sub-RFC 029a
- ~~`frontend/module/hot_reload.rs`~~ — does not exist, belongs to sub-RFC 029b

## Implementation Strategy

### Phase 1: Unified Path Resolution

1. Move predicate strictification from `frontend/module/resolver.rs` to `frontend/core/types/eval/`
2. Migrate the path resolution logic from `package/source/module_resolver.rs` into
   `frontend/module/resolver.rs`
3. Module path ambiguity detection (both `name.yx` and `name/mod.yx` exist → error)

### Phase 2: Multi-File Orchestrator

4. Create `frontend/module/orchestrator.rs`
5. Implement on-demand discovery (recursion from the entry point along use)
6. Implement AST merging (concatenate items from multiple files, Span carries source file)
7. `compiler.rs` adds `compile_project(project_root)` that calls the orchestrator

### Phase 3: use Name Resolution

8. typecheck's `process_use_stmt` switches to querying the Registry (no longer only std)
9. Import conflict detection (same name → error)
10. E2E test: multi-file project with `use` referencing local modules

### Phase 4 (optional, does not block #232)

11. `use path.{item as alias}` in-brace alias
12. vendor directory resolution (in coordination with RFC-014)

### Dependencies

- RFC-014 (package manager) — `yaoxiang.toml` fields, vendor directory structure (only needed for
  Phase 4)
- No other prerequisites

## Sub-RFC Planning

| Sub-RFC | Capability                                 | Prerequisites          |
| ------- | ------------------------------------------ | ---------------------- |
| 029a    | Module cache and incremental recompilation | Orchestrator is stable |
| 029b    | File watching and hot reloading            | 029a                   |
| 029d    | CLI `--entry` to override the entry        | Orchestrator is usable |
| 029e    | Multi-file diagnostic `--json` output      | Diagnostic aggregation |

Deleted: ~~029c (re-export)~~ — not needed. `use` is re-export; there is no "pub use" concept.

## Design Decision Log

| Decision                          | Conclusion                                    | Date       | Rationale                                                                                        |
| --------------------------------- | --------------------------------------------- | ---------- | ------------------------------------------------------------------------------------------------ |
| What is a module                  | A record of a file's top-level bindings       | 2026-07-30 | RFC-010 `name: type = value` unified model                                                       |
| `use` semantics                   | Record destructuring                          | 2026-07-30 | No new mechanism; reuse existing record semantics                                                |
| Visibility                        | Does not exist                                | 2026-07-30 | Scope + distribution boundaries cover all scenarios; no new keyword needed                       |
| `pub` keyword                     | No                                            | 2026-07-30 | "If you don't want others to use it, don't put it at the top level / don't publish that package" |
| mod.yx semantics                  | Directory entry (convention, not requirement) | 2026-07-30 | Python `__init__.py` model: doors are not locked                                                 |
| Module type annotation            | Not needed                                    | 2026-07-30 | Internal bindings already carry their own types; annotation is redundant                         |
| Intra-package circular references | Allowed (AST merging)                         | 2026-07-30 | Path A: zero pipeline changes, matches Rust crate internals                                      |
| Cross-package circular references | Not handled for now                           | 2026-07-30 | No third-party ecosystem; reporting an error is sufficient                                       |
| Registry loading                  | On demand (only reachable modules are loaded) | 2026-07-30 | Not an optimization; it is the definition                                                        |
| Single file vs project            | Same mechanism                                | 2026-07-30 | Registry contents differ; lookup logic is the same                                               |

## References

- [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md) — `name: type = value`
  model
- [RFC-009: Ownership Model](../accepted/009-ownership-model.md) — imports are compile-time name
  resolution
- [RFC-011: Generics Type System](../accepted/011-generic-type-system.md) — structural typing
- [RFC-014: Package Management System Design](../accepted/014-package-manager.md) — package names,
  vendor directory
- [RFC-026: FFI Core Mechanism](../accepted/026-ffi-core-mechanism.md) — StdModule registration
- [RFC-030: assert Mechanism](../accepted/030-assert-mechanism.md) — precedent for unified StdModule
  registration

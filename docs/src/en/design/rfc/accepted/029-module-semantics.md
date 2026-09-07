---
title: 'RFC-029: Module Semantics System'
status: 'Accepted'
author: 'Chenxu'
created: '2026-06-13'
updated:
  '2026-07-30 (Rewritten: based on review discussion, established module=record semantics, removed
  visibility mechanism)'
issue: '#232'
---

# RFC-029: Module Semantics System

## Summary

Wire the module system into the compilation pipeline to enable multi-file compilation.

**Core Definition**: Module = all top-level bindings of a `.yx` file. Module type = types of those
bindings (inferred). `use` = record destructuring. No `pub`, no `private`, no `export`, no
visibility mechanism.

**Core Principles**:

- The type checker only queries a pre-built ModuleRegistry; it never touches the disk
- Files within a package are merged into a single compilation unit (AST splicing); intra-package
  circular references are naturally allowed
- The Registry is loaded on demand: only modules reachable from the entry point along `use` are
  loaded

**Excludes**: Caching, file watching, hot reload, incremental recompilation, inter-package circular
dependency handling.

## Motivation

### Current Problems

1. **The compiler only supports single files**: `Pipeline::run(name, source)` takes a single string
   and cannot handle cross-file dependencies
2. **`use` can only resolve std modules**: All local file `use` statements report "Unknown variable"
3. **The module resolver is in the wrong place**: The only path resolution logic lives in
   `package/source/module_resolver.rs`; `frontend/module/resolver.rs` is actually compile-time
   predicate normalization (RFC-027)

### Design Goals

- A project can compile multiple `.yx` files
- `use` statement semantics clear: record destructuring, not a special mechanism
- Single-file continues to work, no `yaoxiang.toml` required
- Pipeline (`Pipeline`) zero changes; multi-file support is an orchestration concern
- No new keywords, new AST nodes, or new concepts introduced

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
that happens to be defined at a file boundary. The module type is inferred from the bindings;
explicit annotation is never required.

Bindings introduced by `use` are also **part of** the module's contents:

```yaoxiang
// math/mod.yx
use geometry.{Point, distance}
```

The contents of `math` = `{ Point: Type, distance: (Point, Point) -> Float }`. An external
`use math.{Point}` can retrieve it. `use math.geometry.{Point}` can also retrieve it. Both paths
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

| Syntax              | Semantics                                                                  |
| ------------------- | -------------------------------------------------------------------------- |
| `use path.{item}`   | Take the `item` field of the `path` record, bind it into the current scope |
| `use path.{a, b}`   | Take multiple fields                                                       |
| `use path`          | Take the `path` record itself, bind it to the last segment name            |
| `use path as alias` | Take the `path` record itself, bind it to `alias`                          |

#### Non-existent Syntax

- ~~`use path.*`~~: Wildcard import. Not needed; list bindings explicitly.
- ~~`from path use item`~~: Python-style. Not adopted.
- ~~`use path.{item as alias}`~~: Aliases within braces. Phase 4 optional; does not block the main
  line.

#### Import Conflicts

Same-named bindings are reported as an error directly:

```
Name `Point` conflicts:
  math.geometry.Point
  graphics.shapes.Point
Please use different names or module aliases.
```

### 3. Visibility: Does Not Exist

**This RFC introduces no visibility mechanism.** All top-level bindings are visible to any code that
can write a path to them.

This is a deliberate design decision, not an oversight.

#### Design Rationale

| What you want to express    | How to do it                     | Mechanism    |
| --------------------------- | -------------------------------- | ------------ |
| "This is API"               | Put it in the published package  | Distribution |
| "This is internal"          | Put it in an unpublished package | Distribution |
| "This is function-internal" | Write it inside a function body  | Scope        |

All three layers are existing mechanisms: package, file, scope. No new things needed.

#### Why No `pub`

- If you don't want others to use it, don't put it at the top level (put it in local scope)
- Helper functions shared by multiple files go in a separate, unpublished package
- "Can it be prevented" and "Should there be a signal" are two different questions. At this stage
  there is no third-party ecosystem; signals are meaningless
- The door is not locked. Going through the door is polite; climbing over the wall is free. The
  language does not concern itself with politeness

#### Future

When the ecosystem matures and forced boundaries are needed, a separate RFC may introduce them.
Adding restrictions is backward-compatible (default public → explicitly marked internal). But this
RFC does not presuppose that direction, nor does it promise it will come.

### 4. Path Resolution

#### Module Path → File

```
use math.geometry.{Point}
```

Lookup order:

1. **Modules already registered in the Registry**: `math.geometry` is in the Registry → use directly
2. **Standard library**: `std` or `std.*` → built-in module
3. **Importer's directory**: `<importer_dir>/math/geometry.yx` (local module takes priority)
4. **Project root** (nearest `yaoxiang.toml` ancestor): `<project_root>/math/geometry.yx`
5. **vendor directory**: `.yaoxiang/vendor/<pkg>-*/src/` (future)

File location attempt order:

```
base/name.yx
base/name/mod.yx
```

Stop at the first match. If both exist → error:

```
Module path ambiguity: `math.geometry` matches both:
  src/math/geometry.yx
  src/math/geometry/mod.yx
Please delete one of them.
```

The same module key hitting one file in **two roots** and both being referenced (e.g.,
`tests/lib.yx` and `<root>/lib.yx` being referenced by the tests/ entry and the root entry
respectively) → same ambiguity error, not silent shadowing.

> 2026-08-03 revision (RFC-036 driven): Discovery and resolution land per implementation. Discovery
> follows `use` tracing (the protocol established in this RFC's §5), replacing the directory
> recursion of the original implementation — compilation errors from unrelated files no longer block
> running, which is what makes `yaoxiang test` file isolation work. The "importer directory takes
> priority" rule in the dual-root scheme preserves same-directory project behavior; the "project
> root fallback" lets subdirectory entries (e.g., `tests/foo_test.yx`) import project root modules.
> The `src/` layout (RFC-014 package) lands alongside the vendor layer without affecting local dual
> roots.

#### mod.yx = Directory Entry (Convention)

`mod.yx` is the entry file of a directory. When `use math` is encountered, `src/math/mod.yx` is
loaded.

This is a **convention, not a mandate**. Users can directly `use math.geometry` to drill through to
a sub-file. `mod.yx` is a "recommended entry" (doorplate), not the "only entry" (lock).

#### Unified Resolver

The only path resolution logic currently lives in `package/source/module_resolver.rs`. Move it to
`frontend/module/resolver.rs` (replacing the current misnamed predicate normalization file;
predicate normalization moves to `frontend/core/types/eval/`).

### 5. Project Compilation Flow

#### Path A: Merged AST

All files within a package are merged into a **single compilation unit**. Pipeline changes: zero.

```
Orchestrator (above Pipeline):
  1. Determine the entry file
  2. Parse the entry file's use statements (read use lines only, do not parse function bodies)
  3. Discover files along use paths, add to queue
  4. For files in the queue, parse their use statements
  5. Repeat 3-4 until queue is empty (on-demand discovery)
  6. Fully parse all discovered files one by one → multiple ASTs
  7. Merge into one Module (concatenate all top-level items, Span retains source file)
  8. Feed to Pipeline::run() (the pipeline doesn't know there are multiple files)
```

#### Intra-package Circular References: Allowed

Because all files are merged into one AST, files within a package using each other is equivalent to
mutual references within the same file:

```yaoxiang
// tree.yx
use node.{Node}
Tree: Type = { root: Node }

// node.yx
use tree.{Tree}
Node: Type = { value: Int, parent: Tree }
```

After merging, it is two mutually-referencing type definitions within a single AST. The compiler
already supports this.

#### Inter-package Cycles: Later

Packages are distribution units; inter-package requires topological order. There is no third-party
package ecosystem yet, so this is not handled. Just report an error when encountered.

#### Entry File Selection

Priority:

1. `[run].main` (yaoxiang.toml)
2. First item of `[[bin]]` `path`
3. `src/main.yx` (convention default)

When there is no `yaoxiang.toml`: compile the given file directly. The Registry contains only std.
This is not "single-file mode" — it is "the natural result of empty discovery".

#### On-demand Registry Loading

The Registry's contents = all modules reachable from the entry point along `use`. Unreachable
modules are not parsed, not registered, do not exist. **This is not an optimization; it is the
definition.**

### 6. std Modules and User Modules Are Homogeneous

From typecheck's perspective, `use std.io.{println}` and `use math.geometry.{Point}` operate
identically:

1. Find the module record in the Registry
2. Take the field
3. Bind it into the current scope

The source (Std / User / Vendor) is metadata and does not affect resolution logic. Special handling
of native functions is deferred to the IR gen / codegen layer.

## Compiler Changes

| Component                                    | Change                                                                                                                                                                                             |
| -------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `frontend/module/resolver.rs`                | **Rewrite**: currently predicate normalization (RFC-027), move to `frontend/core/types/eval/`. This file becomes the real module path resolver (migrated from `package/source/module_resolver.rs`) |
| `frontend/module/mod.rs`                     | Extend: add source file tracking needed for AST merging (Span carries file name)                                                                                                                   |
| `frontend/module/registry.rs`                | Extend: support registering user modules (currently registers only std)                                                                                                                            |
| `frontend/module/orchestrator.rs`            | **New**: multi-file orchestrator (discover → parse → merge → invoke Pipeline)                                                                                                                      |
| `frontend/pipeline.rs`                       | **Unchanged**                                                                                                                                                                                      |
| `frontend/core/parser/statements/imports.rs` | Unchanged (`use` parsing already implemented)                                                                                                                                                      |
| `package/source/module_resolver.rs`          | **Delete**, logic migrated to `frontend/module/resolver.rs`                                                                                                                                        |
| `frontend/core/typecheck/`                   | `use` handling changed to query Registry (currently only queries std)                                                                                                                              |
| AST `is_pub: bool`                           | **Untouched**. This RFC does not concern visibility                                                                                                                                                |

### Files That Do Not Exist (Old RFC versions claimed "implemented" but do not actually exist)

- ~~`frontend/module/loader.rs`~~ — does not exist; responsibility taken on by the orchestrator
- ~~`frontend/module/dep_graph.rs`~~ — does not exist; intra-package needs no topological sort
  (merged AST)
- ~~`frontend/module/cache.rs`~~ — does not exist; belongs to sub-RFC 029a
- ~~`frontend/module/hot_reload.rs`~~ — does not exist; belongs to sub-RFC 029b

## Implementation Strategy

### Phase 1: Unified Path Resolution

1. Move predicate normalization from `frontend/module/resolver.rs` to `frontend/core/types/eval/`
2. Migrate the path resolution logic from `package/source/module_resolver.rs` to
   `frontend/module/resolver.rs`
3. Module path ambiguity detection (`name.yx` and `name/mod.yx` both existing → error)

### Phase 2: Multi-file Orchestrator

4. Create `frontend/module/orchestrator.rs`
5. Implement on-demand discovery (recursion from entry along use)
6. Implement AST merging (multi-file item concatenation, Span carries source file)
7. `compiler.rs` adds `compile_project(project_root)` to invoke the orchestrator

### Phase 3: use Name Resolution

8. `process_use_stmt` in typecheck changes to query Registry (no longer only std)
9. Import conflict detection (same-name error)
10. E2E test: multi-file project `use` local module

### Phase 4 (Optional, Does Not Block Main Line)

11. `use path.{item as alias}` aliases within braces
12. vendor directory resolution (paired with RFC-014)

### Dependencies

- RFC-014 (Package Manager) — `yaoxiang.toml` fields, vendor directory structure (only needed for
  Phase 4)
- No other prerequisites

## Sub-RFC Planning

| Sub-RFC | Capability                                   | Prerequisite           |
| ------- | -------------------------------------------- | ---------------------- |
| 029a    | Module caching and incremental recompilation | Orchestrator stable    |
| 029b    | File watching and hot reload                 | 029a                   |
| 029d    | CLI `--entry` override entry                 | Orchestrator available |
| 029e    | Multi-file diagnostics `--json` output       | Diagnostic aggregation |

Deleted: ~~029c (re-export)~~ — Not needed. `use` is re-export; there is no "pub use" concept.

## Design Decision Records

| Decision               | Conclusion                                | Date       | Basis                                                                                        |
| ---------------------- | ----------------------------------------- | ---------- | -------------------------------------------------------------------------------------------- |
| What is a module       | Record of file's top-level bindings       | 2026-07-30 | RFC-010 `name: type = value` unified model                                                   |
| `use` semantics        | Record destructuring                      | 2026-07-30 | No new mechanism introduced; reuses existing record semantics                                |
| Visibility             | Does not exist                            | 2026-07-30 | Scope + distribution boundaries cover all scenarios; no new keyword needed                   |
| `pub` keyword          | No                                        | 2026-07-30 | "If you don't want others to use it, don't put it at top level / don't publish that package" |
| mod.yx semantics       | Directory entry (convention, not mandate) | 2026-07-30 | Python `__init__.py` model: door is not locked                                               |
| Module type annotation | Not needed                                | 2026-07-30 | Internal bindings already carry their types; annotation is redundant                         |
| Intra-package cycles   | Allowed (merged AST)                      | 2026-07-30 | Path A: zero pipeline changes; Rust crate internal model                                     |
| Inter-package cycles   | Not handled for now                       | 2026-07-30 | No third-party ecosystem; error on encounter                                                 |
| Registry loading       | On demand (only reachable modules loaded) | 2026-07-30 | Not an optimization, it is the definition                                                    |
| Single-file vs project | Same mechanism                            | 2026-07-30 | Registry contents differ, lookup logic is the same                                           |

## References

- [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md) — `name: type = value`
  model
- [RFC-009: Ownership Model](../accepted/009-ownership-model.md) — Import is compile-time name
  resolution
- [RFC-011: Generics Type System](../accepted/011-generic-type-system.md) — Structural types
- [RFC-014: Package Management System Design](../accepted/014-package-manager.md) — Package names,
  vendor directory
- [RFC-026: FFI Core Mechanism](../accepted/026-ffi-core-mechanism.md) — StdModule registration
- [RFC-030: assert Mechanism](../accepted/030-assert-mechanism.md) — StdModule unified registration
  precedent

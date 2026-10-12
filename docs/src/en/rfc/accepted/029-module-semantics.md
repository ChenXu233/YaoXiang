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

Plug the module system into the compilation pipeline, enabling multi-file compilation.

**Core definition**: A module = all top-level bindings of one `.yx` file. The type of a module = the
types of those bindings (inferred). `use` = record destructuring. No `pub`, no `private`, no
`export`, no visibility mechanism.

**Core principles**:

- The type checker only queries the pre-built ModuleRegistry, never touches the disk
- Files within a package merge into a single compilation unit (AST splicing); intra-package circular
  references are naturally allowed
- Registry loads on demand: only modules reachable from the entry point along `use` are loaded

**Not included**: caching, file watching, hot reload, incremental recompilation, cross-package
circular dependency handling.

## Motivation

### Current Problems

1. **Compiler only supports single file**: `Pipeline::run(name, source)` takes a string, can't
   handle cross-file dependencies
2. **`use` can only resolve std modules**: `use` between local files all report "Unknown variable"
3. **Module resolver is in the wrong place**: The only path resolution logic lives in
   `package/source/module_resolver.rs`; `frontend/module/resolver.rs` is actually compile-time
   predicate strict normalization (RFC-027)

### Design Goals

- One project can compile multiple `.yx` files
- `use` statement semantics are clear: record destructuring, not a special mechanism
- Single-file continues to work, no `yaoxiang.toml` required
- Pipeline (`Pipeline`) is unchanged; multi-file support is the orchestrator layer's job
- No new keywords, no new AST nodes, no new concepts

## Proposal

### 1. Module = record of bindings

A **module** is all top-level bindings of one `.yx` file.

```yaoxiang
// math/geometry.yx
Point: Type = { x: Float, y: Float }
distance: (a: Point, b: Point) -> Float = { ... }
```

The contents of this module is `{ Point: Type, distance: (Point, Point) -> Float }`.

A module is not a special entity. It is an instance of the `name: type = value` model—a record that
happens to be defined at the file boundary. The module's type is inferred from the bindings;
explicit annotation is never needed.

Bindings brought in by `use` are **also** part of the module's contents:

```yaoxiang
// math/mod.yx
use geometry.{Point, distance}
```

The contents of the `math` module = `{ Point: Type, distance: (Point, Point) -> Float }`. External
`use math.{Point}` can get it. `use math.geometry.{Point}` can also get it. Both paths point to the
same binding.

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

#### Nonexistent Syntax

- ~~`use path.*`~~: wildcard import. Not needed; list bindings explicitly.
- ~~`from path use item`~~: Python-style. Not adopted.
- ~~`use path.{item as alias}`~~: alias inside braces. Optional for Phase 4, doesn't block the main
  path.

#### Import Conflicts

Same-name bindings directly error:

```
Name `Point` conflicts:
  math.geometry.Point
  graphics.shapes.Point
Please use different names or module aliases.
```

### 3. Visibility: does not exist

**This RFC introduces no visibility mechanism.** All top-level bindings are visible to any code that
can write a path to them.

This is an intentional design decision, not an oversight.

#### Design Rationale

| What you want to express    | How to do it                     | Mechanism    |
| --------------------------- | -------------------------------- | ------------ |
| "This is an API"            | Put it in a published package    | Distribution |
| "This is internal"          | Put it in an unpublished package | Distribution |
| "This is inside a function" | Write it inside a function body  | Scope        |

All three layers are existing mechanisms: package, file, scope. Nothing new is needed.

#### Why no `pub`

- Things you don't want others to use shouldn't sit at the top level (put them in local scope)
- Helper functions shared across multiple files go in a separate, unpublished package
- "Whether it can be prevented" and "whether there should be a signal" are two different things. At
  the current stage, with no third-party ecosystem, the signal is meaningless
- The door isn't locked. Going through the door is polite, climbing the wall is freedom. The
  language doesn't govern politeness

#### Future

When the ecosystem matures and enforced boundaries are needed, they can be introduced via a separate
RFC. Adding restrictions is backward-compatible (default public → explicitly mark as internal). But
this RFC does not presuppose that direction, nor does it promise it will come.

### 4. Path Resolution

#### Module path → file

```
use math.geometry.{Point}
```

Lookup order:

1. **Registry-registered module**: `math.geometry` is already in the Registry → use it directly
2. **Standard library**: `std` or `std.*` → built-in modules
3. **Importer's directory**: `<importer_dir>/math/geometry.yx` (local modules take priority)
4. **Project root** (nearest `yaoxiang.toml` ancestor): `<project_root>/math/geometry.yx`
5. **vendor directory**: `.yaoxiang/vendor/<pkg>-*/src/` (future)

File location attempt order:

```
base/name.yx
base/name/mod.yx
```

Stop at the first one found. If both exist → error:

```
Module path ambiguous: `math.geometry` matches both:
  src/math/geometry.yx
  src/math/geometry/mod.yx
Please delete one.
```

The same module key hits a file in **two roots** and both are referenced (e.g. `tests/lib.yx` and
`<root>/lib.yx` referenced by the tests/ entry and the root entry respectively) → also report an
ambiguity error, not silent shadowing.

> Revised 2026-08-03 (driven by RFC-036): discovery follows implementation. Discovery follows `use`
> (the protocol established in §5 of this RFC), replacing the directory recursion of the original
> implementation—compilation errors in unrelated files no longer block execution, and the test file
> isolation of `yaoxiang test` only works this way. The "importer directory priority" rule in the
> two-root rule guarantees unchanged same-directory project behavior; "project root fallback" lets
> subdirectory entries (e.g. `tests/foo_test.yx`) import project-root modules. The `src/` layout
> (RFC-014 package) is handled together with the vendor layer and doesn't affect local dual roots.

#### mod.yx = directory entry (convention)

`mod.yx` is the entry file of a directory. When `use math` is invoked, `src/math/mod.yx` is loaded.

This is a **convention, not an enforcement**. Users can directly `use math.geometry` to pierce
through to subfiles. `mod.yx` is the "recommended entry" (door number), not the "only entry" (lock).

#### Unified Resolver

The only path resolution logic currently lives in `package/source/module_resolver.rs`. Move it to
`frontend/module/resolver.rs` (replacing the current misnamed predicate strict normalization file;
predicate strict normalization moves to `frontend/core/types/eval/`).

### 5. Project Compilation Flow

#### Path A: Merge AST

All files in a package merge into a **single compilation unit**. Zero changes to the pipeline.

```
Orchestrator (on top of Pipeline):
  1. Determine the entry file
  2. Parse the entry file's use statements (only read use lines, don't parse function bodies)
  3. Discover files along the use path, add to queue
  4. For files in the queue, parse their use statements
  5. Repeat 3-4 until the queue is empty (on-demand discovery)
  6. Fully parse all discovered files → multiple ASTs
  7. Merge into one Module (all top-level items spliced, Span preserves source file)
  8. Feed to Pipeline::run() (pipeline doesn't know there are multiple files)
```

#### Intra-package circular references: allowed

Because all files merge into one AST, mutual `use` between files within a package is equivalent to
mutual references within the same file:

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

#### Inter-package circular: TBD

Packages are distribution units; inter-package needs a topological order. Currently no third-party
package ecosystem; not handled for now. When encountered, just error out directly.

#### Entry File Selection

Priority:

1. `[run].main` (yaoxiang.toml)
2. `[[bin]]` first entry's `path`
3. `src/main.yx` (default convention)

When there's no `yaoxiang.toml`: compile the given file directly. The Registry only has std. This is
not "single-file mode"—it is "the natural result of an empty discovery".

#### Registry Loads on Demand

The Registry's contents = all modules reachable from the entry along `use`. Unreachable modules are
not parsed, not registered, don't exist. **This is not an optimization; it is the definition.**

### 6. std modules and user modules are homogeneous

For typecheck, `use std.io.{println}` and `use math.geometry.{Point}` operate identically:

1. Find the module record in the Registry
2. Take the field
3. Bind to the current scope

The source (Std / User / Vendor) is metadata and doesn't affect resolution logic. Special handling
of native functions is deferred to the IR gen / codegen layer.

## Compiler Changes

| Component                                    | Change                                                                                                                                                                                                     |
| -------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `frontend/module/resolver.rs`                | **Rewrite**: currently predicate strict normalization (RFC-027), move to `frontend/core/types/eval/`. This file changes to real module path resolution (migrated from `package/source/module_resolver.rs`) |
| `frontend/module/mod.rs`                     | Extend: supplement source file tracking needed for AST merging (Span carries filename)                                                                                                                     |
| `frontend/module/registry.rs`                | Extend: support registering user modules (currently only std is registered)                                                                                                                                |
| `frontend/module/orchestrator.rs`            | **New**: multi-file orchestrator (discover → parse → merge → call Pipeline)                                                                                                                                |
| `frontend/pipeline.rs`                       | **Unchanged**                                                                                                                                                                                              |
| `frontend/core/parser/statements/imports.rs` | Unchanged (`use` parsing is already implemented)                                                                                                                                                           |
| `package/source/module_resolver.rs`          | **Delete**, logic migrates to `frontend/module/resolver.rs`                                                                                                                                                |
| `frontend/core/typecheck/`                   | `use` handling changes to query Registry (currently only queries std)                                                                                                                                      |
| AST `is_pub: bool`                           | ~~Unchanged~~ (029g has ruled to overturn: removed along with the deletion of the `pub` keyword; **landed 2026-10-08**, see [029g](029g-remove-pub-and-auto-bind.md) and WBS §P3.5)                        |

### Files That Don't Exist (older RFC versions claimed "already implemented" but they don't)

- ~~`frontend/module/loader.rs`~~ — doesn't exist, responsibilities handled by the orchestrator
- ~~`frontend/module/dep_graph.rs`~~ — doesn't exist, packages don't need topological sort (merge
  AST)
- ~~`frontend/module/cache.rs`~~ — doesn't exist, belongs to sub-RFC 029a
- ~~`frontend/module/hot_reload.rs`~~ — doesn't exist, belongs to sub-RFC 029b

## Implementation Strategy

### Phase 1: Unify path resolution

1. Move predicate strict normalization from `frontend/module/resolver.rs` to
   `frontend/core/types/eval/`
2. Migrate the path resolution logic of `package/source/module_resolver.rs` to
   `frontend/module/resolver.rs`
3. Module path ambiguity detection (both `name.yx` and `name/mod.yx` exist → error)

### Phase 2: Multi-file orchestrator

4. Create `frontend/module/orchestrator.rs`
5. Implement on-demand discovery (recursive along use from the entry)
6. Implement AST merging (multi-file item splicing, Span carries source file)
7. Add `compile_project(project_root)` to `compiler.rs` to call the orchestrator

### Phase 3: use name resolution

8. `process_use_stmt` in typecheck changes to query Registry (no longer only queries std)
9. Import conflict detection (same name errors)
10. E2E tests: multi-file project `use`s local modules

### Phase 4 (optional, doesn't block the main line)

11. `use path.{item as alias}` alias inside braces
12. vendor directory resolution (paired with RFC-014)

### Dependencies

- RFC-014 (package manager) — `yaoxiang.toml` fields, vendor directory structure (needed only for
  Phase 4)
- No other prerequisite dependencies

## Sub-RFC Planning

| Sub-RFC | Capability                                                                                  | Prerequisite           |
| ------- | ------------------------------------------------------------------------------------------- | ---------------------- |
| 029a    | Module caching and incremental recompilation                                                | Orchestrator stable    |
| 029b    | File watching and hot reload                                                                | 029a                   |
| 029d    | CLI `--entry` overrides entry                                                               | Orchestrator available |
| 029e    | Multi-file diagnostic `--json` output                                                       | Diagnostic aggregation |
| 029f    | Compile target role and import surface semantics (accepted 2026-09-13, #334)                | Orchestrator stable    |
| 029g    | Remove `pub` keyword and auto-bind (visibility mechanism final ruling; accepted 2026-10-02) | None                   |

Deleted: ~~029c (re-export)~~ — not needed. `use` is re-export; there is no "pub use" concept.

## Design Decision Log

| Decision               | Conclusion                                    | Date       | Rationale                                                                                 |
| ---------------------- | --------------------------------------------- | ---------- | ----------------------------------------------------------------------------------------- |
| What is a module       | record of file top-level bindings             | 2026-07-30 | RFC-010 `name: type = value` unified model                                                |
| `use` semantics        | record destructuring                          | 2026-07-30 | No new mechanism; reuse existing record semantics                                         |
| Visibility             | Does not exist                                | 2026-07-30 | Scope + distribution boundary cover all scenarios; no new keyword needed                  |
| `pub` keyword          | No                                            | 2026-07-30 | "Don't want others to use it? Don't put it at the top level / don't publish that package" |
| mod.yx semantics       | Directory entry (convention, not enforcement) | 2026-07-30 | Python `__init__.py` model: the door isn't locked                                         |
| Module type annotation | Not needed                                    | 2026-07-30 | Internal bindings already carry their types; annotation is redundant                      |
| Intra-package circular | Allowed (merge AST)                           | 2026-07-30 | Path A: zero pipeline changes; Rust crate internal model                                  |
| Inter-package circular | Not handled for now                           | 2026-07-30 | No third-party ecosystem; just error when encountered                                     |
| Registry loading       | On demand (only load reachable modules)       | 2026-07-30 | Not an optimization; it is the definition                                                 |
| Single file vs project | Same mechanism                                | 2026-07-30 | Registry contents differ; lookup logic is the same                                        |

## References

- [RFC-010: Unified Type Syntax](010-unified-type-syntax.md) — `name: type = value` model
- [RFC-009: Ownership Model](009-ownership-model.md) — Import is compile-time name resolution
- [RFC-011: Generic Type System](011-generic-type-system.md) — Structural types
- [RFC-014: Package Management System Design](014-package-manager.md) — Package name, vendor
  directory
- [RFC-026: FFI Core Mechanism](026-ffi-core-mechanism.md) — StdModule registration
- [RFC-030: assert Mechanism](030-assert-mechanism.md) — StdModule unified registration precedent

---
title: 'RFC-029: Module Semantics System'
status: 'Accepted'
author: 'Chenxu'
created: '2026-06-13'
updated:
  '2026-07-30 (Rewrite: based on review discussion, established module=record semantics, removed
  visibility mechanism)'
issue: '#232'
---

# RFC-029: Module Semantics System

## Summary

Integrate the module system into the compilation pipeline to enable multi-file compilation.

**Core definition**: A module = all top-level bindings of a `.yx` file. A module's type = the types
of those bindings (inferred). `use` = record destructuring. No `pub`, no `private`, no `export`, no
visibility mechanism.

**Core principles**:

- The type checker only queries the pre-built ModuleRegistry; it never touches the disk
- Files within a package are merged into a single compilation unit (AST splicing), and circular
  references within a package are naturally allowed
- The Registry loads on demand: only modules reachable from the entry point along `use` are
  installed

**Out of scope**: caching, file watching, hot reloading, incremental recompilation, cross-package
circular dependency handling.

## Motivation

### Current Problems

1. **The compiler only supports a single file**: `Pipeline::run(name, source)` takes a single string
   and cannot handle cross-file dependencies
2. **`use` can only resolve std modules**: every `use` between local files reports "Unknown
   variable"
3. **The module resolver is in the wrong place**: the only path-resolution logic lives in
   `package/source/module_resolver.rs`; `frontend/module/resolver.rs` is actually compile-time
   predicate normalization (RFC-027)

### Design Goals

- A project can compile multiple `.yx` files
- The semantics of `use` statements are clear: record destructuring, not a special mechanism
- Single-file mode keeps working; `yaoxiang.toml` is not required
- Zero changes to the pipeline (`Pipeline`); multi-file support is the orchestrator's job
- No new keywords, no new AST nodes, no new concepts

## Proposal

### 1. Module = record of bindings

A **module** is all top-level bindings of a `.yx` file.

```yaoxiang
// math/geometry.yx
Point: Type = { x: Float, y: Float }
distance: (a: Point, b: Point) -> Float = { ... }
```

The contents of this module are `{ Point: Type, distance: (Point, Point) -> Float }`.

A module is not a special entity. It is an instance of the `name: type = value` model—a record that
happens to be defined at the file boundary. A module's type is inferred from its bindings; explicit
annotation is never required.

Bindings introduced by `use` are **also** part of the module's contents:

```yaoxiang
// math/mod.yx
use geometry.{Point, distance}
```

The contents of the `math` module = `{ Point: Type, distance: (Point, Point) -> Float }`. Outside,
`use math.{Point}` can fetch it. `use math.geometry.{Point}` can also fetch it. Both paths point to
the same binding.

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
| `use path`          | Take the `path` record itself, bind it to the last segment's name          |
| `use path as alias` | Take the `path` record itself, bind it to `alias`                          |

#### Non-existent syntax

- ~~`use path.*`~~: wildcard import. Not needed; list bindings explicitly.
- ~~`from path use item`~~: Python-style. Not adopted.
- ~~`use path.{item as alias}`~~: aliases inside braces. Optional for Phase 4, does not block the
  main line.

#### Import conflicts

Same-name bindings fail directly:

```
Name `Point` conflicts:
  math.geometry.Point
  graphics.shapes.Point
Please use a different name or a module alias.
```

### 3. Visibility: Does not exist

**This RFC introduces no visibility mechanism whatsoever.** All top-level bindings are visible to
any code that can write a path to them.

This is an intentional design decision, not an oversight.

#### Design rationale

| What you want to express    | How to do it                     | Mechanism    |
| --------------------------- | -------------------------------- | ------------ |
| "This is the API"           | Put it in a published package    | Distribution |
| "This is internal"          | Put it in an unpublished package | Distribution |
| "This is inside a function" | Write it inside a function body  | Scope        |

All three layers are existing mechanisms: package, file, scope. Nothing new is needed.

#### Why no `pub`

- Things you don't want others to use shouldn't live at the top level (put them in a local scope)
- Helper functions shared by multiple files belong in a separate, unpublished package
- "Does it block misuse" and "Should there be a signal" are two different questions. There is no
  third-party ecosystem at this stage, so the signal is meaningless
- Doors don't have locks. Walking through the door is polite; climbing over the wall is free. The
  language does not police politeness

#### Future

When the ecosystem matures and forced boundaries are needed, they can be introduced via a separate
RFC. Adding restrictions is backward-compatible (default public → explicitly marked internal). But
this RFC does not pre-assume that direction, nor does it promise it will come.

### 4. Path resolution

#### Module path → file

```
use math.geometry.{Point}
```

Lookup order:

1. **Registry-registered modules**: `math.geometry` is already in the Registry → use it directly
2. **Standard library**: `std` or `std.*` → builtin module
3. **Importer's directory**: `<importer_dir>/math/geometry.yx` (local modules take priority)
4. **Project root** (nearest `yaoxiang.toml` ancestor): `<project_root>/math/geometry.yx`
5. **vendor directory**: `.yaoxiang/vendor/<pkg>-*/src/` (future)

File-locator attempt order:

```
base/name.yx
base/name/mod.yx
```

Stop at the first match. If both exist simultaneously → error:

```
Module path ambiguous: `math.geometry` matches both:
  src/math/geometry.yx
  src/math/geometry/mod.yx
Please remove one of them.
```

The same module key hits a file under **two roots** and both are referenced (e.g. `tests/lib.yx` and
`<root>/lib.yx` are respectively referenced by a tests/ entry and a root entry) → likewise report an
ambiguity error rather than silent shadowing.

> 2026-08-03 revision (driven by RFC-036): discovery and resolution are implemented as designed.
> Discovery follows `use` tracing (the protocol established in §5 of this RFC), replacing the
> original directory-recursion implementation—compile errors in unrelated files no longer block
> execution, which is what makes the test-file isolation of `yaoxiang test` work. The "importer's
> directory first" rule in the dual-root scheme preserves same-directory project behavior unchanged;
> the "project root fallback" lets subdirectory entries (e.g. `tests/foo_test.yx`) import
> project-root modules. The `src/` layout (RFC-014 packages) is handled together with the vendor
> layer when it lands, and does not affect local dual-root behavior.

#### mod.yx = directory entry (convention)

`mod.yx` is the entry file of a directory. When you write `use math`, the loader picks up
`src/math/mod.yx`.

This is a **convention, not a requirement**. Users can write `use math.geometry` directly to reach
into a subfile. `mod.yx` is the "recommended entry" (a name plate), not the "only entry" (a lock).

#### Unified resolver

The only path-resolution logic today lives in `package/source/module_resolver.rs`. Move it to
`frontend/module/resolver.rs` (replacing the current misnamed predicate-normalization file;
predicate normalization moves to `frontend/core/types/eval/`).

### 5. Project compilation flow

#### Path A: Merge ASTs

All files within a package are merged into a **single compilation unit**. Zero changes to the
pipeline.

```
Orchestrator (above Pipeline):
  1. Determine the entry file
  2. Parse the entry file's use statements (read use lines only; do not parse function bodies)
  3. Discover files along use paths, add them to the queue
  4. For files in the queue, parse their use statements
  5. Repeat 3-4 until the queue is empty (on-demand discovery)
  6. Fully parse every discovered file → multiple ASTs
  7. Merge into a single Module (all top-level items spliced; Span retains its source file)
  8. Feed it to Pipeline::run() (the pipeline does not know there are multiple files)
```

#### Circular references within a package: allowed

Because all files merge into one AST, mutual `use` between files in a package is equivalent to two
mutually referencing definitions in a single file:

```yaoxiang
// tree.yx
use node.{Node}
Tree: Type = { root: Node }

// node.yx
use tree.{Tree}
Node: Type = { value: Int, parent: Tree }
```

After merging, this is just two mutually referencing type definitions inside one AST. The compiler
already supports that.

#### Circular references between packages: TBD

Packages are the unit of distribution and need a topological order between them. There is no
third-party package ecosystem yet, so this is not handled for now. If encountered, just report an
error.

#### Entry file selection

Priority:

1. `[run].main` (yaoxiang.toml)
2. `path` of the first `[[bin]]` entry
3. `src/main.yx` (convention default)

With no `yaoxiang.toml`: compile the given file directly. The Registry contains only std. This is
not "single-file mode"—it is the natural result of an empty discovery.

#### Registry loads on demand

The contents of the Registry = all modules reachable from the entry point along `use`. Unreachable
modules are not parsed, not registered, and do not exist. **This is not an optimization; it is the
definition.**

### 6. std modules and user modules are homogeneous

To the typechecker, `use std.io.{println}` and `use math.geometry.{Point}` are exactly the same
operation:

1. Find the module record in the Registry
2. Take the field
3. Bind it into the current scope

The source (Std / User / Vendor) is metadata and does not affect resolution logic. The special
handling of native functions is deferred to the IR gen / codegen layer.

## Compiler changes

| Component                                    | Change                                                                                                                                                                                             |
| -------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `frontend/module/resolver.rs`                | **Rewrite**: currently predicate normalization (RFC-027), move to `frontend/core/types/eval/`. This file becomes the real module-path resolver (migrated from `package/source/module_resolver.rs`) |
| `frontend/module/mod.rs`                     | Extend: add source-file tracking required for AST merging (Span carries filename)                                                                                                                  |
| `frontend/module/registry.rs`                | Extend: support registering user modules (currently registers std only)                                                                                                                            |
| `frontend/module/orchestrator.rs`            | **New**: multi-file orchestrator (discover → parse → merge → call Pipeline)                                                                                                                        |
| `frontend/pipeline.rs`                       | **Unchanged**                                                                                                                                                                                      |
| `frontend/core/parser/statements/imports.rs` | Unchanged (`use` parsing is already implemented)                                                                                                                                                   |
| `package/source/module_resolver.rs`          | **Delete**; logic migrates to `frontend/module/resolver.rs`                                                                                                                                        |
| `frontend/core/typecheck/`                   | `use` handling switches to Registry lookup (currently looks up std only)                                                                                                                           |
| AST `is_pub: bool`                           | **Untouched**. This RFC does not address visibility                                                                                                                                                |

### Files that do not exist (older versions of the RFC claimed "implemented" but they don't)

- ~~`frontend/module/loader.rs`~~ — does not exist; responsibility belongs to the orchestrator
- ~~`frontend/module/dep_graph.rs`~~ — does not exist; no topological sort needed within a package
  (AST merging)
- ~~`frontend/module/cache.rs`~~ — does not exist; belongs to sub-RFC 029a
- ~~`frontend/module/hot_reload.rs`~~ — does not exist; belongs to sub-RFC 029b

## Implementation strategy

### Phase 1: Unified path resolution

1. Move predicate normalization from `frontend/module/resolver.rs` to `frontend/core/types/eval/`
2. Migrate the path-resolution logic from `package/source/module_resolver.rs` to
   `frontend/module/resolver.rs`
3. Module path ambiguity detection (both `name.yx` and `name/mod.yx` exist → error)

### Phase 2: Multi-file orchestrator

4. Create `frontend/module/orchestrator.rs`
5. Implement on-demand discovery (recursion from the entry point along `use`)
6. Implement AST merging (splice top-level items from multiple files; Span carries source file)
7. `compiler.rs` adds `compile_project(project_root)` that calls the orchestrator

### Phase 3: use name resolution

8. `process_use_stmt` in the typechecker switches to Registry lookup (no longer std-only)
9. Import conflict detection (same name → error)
10. End-to-end test: a multi-file project where `use` imports a local module

### Phase 4 (optional, does not block the main line)

11. `use path.{item as alias}` — aliases inside braces
12. vendor directory resolution (in coordination with RFC-014)

### Dependencies

- RFC-014 (package manager) — `yaoxiang.toml` fields, vendor directory structure (only needed in
  Phase 4)
- No other prerequisites

## Sub-RFC planning

| Sub-RFC | Capability                                                                    | Prerequisite           |
| ------- | ----------------------------------------------------------------------------- | ---------------------- |
| 029a    | Module caching and incremental recompilation                                  | Orchestrator stable    |
| 029b    | File watching and hot reloading                                               | 029a                   |
| 029d    | CLI `--entry` to override the entry point                                     | Orchestrator available |
| 029e    | Multi-file diagnostic `--json` output                                         | Diagnostic aggregation |
| 029f    | Compile target roles and import-surface semantics (accepted 2026-09-13, #334) | Orchestrator stable    |

Deleted: ~~029c (re-exports)~~ — not needed. `use` is already a re-export; there is no "pub use"
concept.

## Design decision records

| Decision                          | Conclusion                                    | Date       | Rationale                                                                                        |
| --------------------------------- | --------------------------------------------- | ---------- | ------------------------------------------------------------------------------------------------ |
| What is a module                  | A record of a file's top-level bindings       | 2026-07-30 | RFC-010's `name: type = value` unified model                                                     |
| `use` semantics                   | Record destructuring                          | 2026-07-30 | No new mechanism; reuses existing record semantics                                               |
| Visibility                        | Does not exist                                | 2026-07-30 | Scope + distribution boundaries cover every case; no new keyword needed                          |
| `pub` keyword                     | No                                            | 2026-07-30 | "If you don't want others to use it, don't put it at the top level / don't publish that package" |
| mod.yx semantics                  | Directory entry (convention, not requirement) | 2026-07-30 | Python `__init__.py` model: doors don't have locks                                               |
| Module type annotation            | Not required                                  | 2026-07-30 | Internal bindings already carry types; annotation is redundant                                   |
| Intra-package circular references | Allowed (AST merge)                           | 2026-07-30 | Path A: zero changes to the pipeline, matches the in-Rust-crate model                            |
| Inter-package circular references | Not handled for now                           | 2026-07-30 | No third-party ecosystem; just report an error if encountered                                    |
| Registry loading                  | On demand (only reachable modules installed)  | 2026-07-30 | Not an optimization; it is the definition                                                        |
| Single file vs project            | Same mechanism                                | 2026-07-30 | Different Registry contents, same lookup logic                                                   |

## References

- [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md) — the `name: type = value`
  model
- [RFC-009: Ownership Model](../accepted/009-ownership-model.md) — imports are compile-time name
  resolution
- [RFC-011: Generic Type System](../accepted/011-generic-type-system.md) — structural typing
- [RFC-014: Package Management System Design](../accepted/014-package-manager.md) — package names,
  vendor directory
- [RFC-026: FFI Core Mechanism](../accepted/026-ffi-core-mechanism.md) — StdModule registration
- [RFC-030: assert Mechanism](../accepted/030-assert-mechanism.md) — prior art for unified StdModule
  registration

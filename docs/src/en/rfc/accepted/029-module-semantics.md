---
title: 'RFC-029: Module Semantics System'
status: 'Accepted'
author: 'Chenxu'
created: '2026-06-13'
updated:
  '2026-07-30 (Rewrite: based on review discussions, established module=record semantics, removed
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

- The type checker only queries the pre-built ModuleRegistry, never touches disk
- Files within a package are merged into a single compilation unit (AST splicing); intra-package
  circular references are naturally allowed
- Registry is loaded on demand: only modules reachable from the entry point via `use` are loaded

**Not included**: caching, file watching, hot reload, incremental recompilation, cross-package
circular dependency handling.

## Motivation

### Current Problems

1. **The compiler only supports single files**: `Pipeline::run(name, source)` accepts a single
   string and cannot handle cross-file dependencies
2. **`use` can only resolve std modules**: `use` between local files all report "Unknown variable"
3. **The module resolver is in the wrong place**: the only path resolution logic is in
   `package/source/module_resolver.rs`; `frontend/module/resolver.rs` is actually compile-time
   predicate strictification (RFC-027)

### Design Goals

- A project can compile multiple `.yx` files
- The semantics of `use` statements are clear: record destructuring, not a special mechanism
- Single-file mode continues to work without requiring `yaoxiang.toml`
- Zero changes to the pipeline (`Pipeline`); multi-file support is the orchestrator's job
- No new keywords, new AST nodes, or new concepts

## Proposal

### 1. Module = record of bindings

A **module** is all top-level bindings of a `.yx` file.

```yaoxiang
// math/geometry.yx
Point: Type = { x: Float, y: Float }
distance: (a: Point, b: Point) -> Float = { ... }
```

The contents of this module is `{ Point: Type, distance: (Point, Point) -> Float }`.

A module is not a special entity. It is an instance of the `name: type = value` model—a record
defined at the file boundary. A module's type is inferred from its bindings; explicit annotation is
never needed.

Bindings introduced by `use` are **also** part of a module's contents:

```yaoxiang
// math/mod.yx
use geometry.{Point, distance}
```

The contents of the `math` module = `{ Point: Type, distance: (Point, Point) -> Float }`. An
external `use math.{Point}` can get it. `use math.geometry.{Point}` can also get it. Both paths
point to the same binding.

### 2. use = record destructuring

All `use` forms are record field access + binding:

```yaoxiang
use math.geometry.{Point, distance}
```

equivalent to:

```yaoxiang
Point = math.geometry.Point
distance = math.geometry.distance
```

| Syntax              | Semantics                                                                |
| ------------------- | ------------------------------------------------------------------------ |
| `use path.{item}`   | Take the `item` field of the `path` record, bind it to the current scope |
| `use path.{a, b}`   | Take multiple fields                                                     |
| `use path`          | Take the `path` record itself, bind it to the last segment name          |
| `use path as alias` | Take the `path` record itself, bind it to `alias`                        |

#### Nonexistent syntax

- ~~`use path.*`~~: wildcard import. Not needed; list bindings explicitly.
- ~~`from path use item`~~: Python-style. Not adopted.
- ~~`use path.{item as alias}`~~: alias inside braces. Optional in Phase 4, does not block the main
  path.

#### Import conflicts

Same-name bindings are reported as errors directly:

```
Name `Point` conflicts:
  math.geometry.Point
  graphics.shapes.Point
Please use different names or module aliases.
```

### 3. Visibility: does not exist

**This RFC introduces no visibility mechanism.** All top-level bindings are visible to all code that
can write a path to them.

This is an intentional design decision, not an oversight.

#### Design rationale

| What you want to express    | How to do it                  | Mechanism    |
| --------------------------- | ----------------------------- | ------------ |
| "This is API"               | Put in a published package    | Distribution |
| "This is internal"          | Put in an unpublished package | Distribution |
| "This is function-internal" | Write inside a function body  | Scope        |

All three layers are existing mechanisms: package, file, scope. Nothing new needed.

#### Why no `pub`

- Things you don't want others to use should not be placed at the top level (put them in local
  scope)
- Helper functions shared by multiple files go in a separate unpublished package
- "Whether it can be prevented" and "whether there should be a signal" are two different things. At
  the current stage there is no third-party ecosystem; signals are meaningless
- The door is not locked. Using the door is courtesy, climbing over the wall is freedom. The
  language does not care about courtesy

#### Future

When the ecosystem matures and forced boundaries are needed, they can be introduced via a separate
RFC. Adding restrictions is backward-compatible (public by default → explicitly marked as internal).
However, this RFC does not pre-assume that direction, nor does it promise it will come.

### 4. Path resolution

#### Module path → file

```
use math.geometry.{Point}
```

Search order:

1. **Modules registered in the Registry**: `math.geometry` is already in the Registry → use directly
2. **Standard library**: `std` or `std.*` → built-in module
3. **Importer's directory**: `<importer_dir>/math/geometry.yx` (local modules take priority)
4. **Project root** (closest `yaoxiang.toml` ancestor): `<project_root>/math/geometry.yx`
5. **Vendor directory**: `.yaoxiang/vendor/<pkg>-*/src/` (future)

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
Please delete one of them.
```

When the same module key hits one file in **both roots** and both are referenced (e.g.,
`tests/lib.yx` and `<root>/lib.yx` are referenced by the tests/ entry and the root entry
respectively) → likewise report an ambiguity error, rather than silently shadowing.

> 2026-08-03 revision (driven by RFC-036): discovery and resolution are implemented as landed.
> Discovery follows `use` tracing (the protocol established in this RFC §5), replacing the original
> implementation's directory recursion—compile errors of unrelated files no longer block execution,
> and `yaoxiang test`'s test file isolation only then holds. The "importer directory first" rule in
> the dual-root scheme preserves the behavior of same-directory projects; the "project root
> fallback" lets subdirectory entries (e.g., `tests/foo_test.yx`) import project root modules. The
> `src/` layout (RFC-014 package) is handled together at the vendor layer when landed, and does not
> affect the local dual root.

#### mod.yx = directory entry (convention)

`mod.yx` is a directory's entry file. When using `use math`, `src/math/mod.yx` is loaded.

This is a **convention, not enforced**. Users can directly use `use math.geometry` to penetrate to
subfiles. `mod.yx` is a "recommended entry" (doorplate), not a "sole entry" (lock).

#### Unified resolver

The only path resolution logic currently lives in `package/source/module_resolver.rs`. Move it to
`frontend/module/resolver.rs` (replacing the currently misnamed predicate strictification file;
predicate strictification moves to `frontend/core/types/eval/`).

### 5. Project compilation flow

#### Path A: merge AST

All files within a package are merged into a **single compilation unit**. Zero changes to the
pipeline.

```
Orchestrator (above the pipeline):
  1. Determine the entry file
  2. Parse the entry file's use statements (only read the use lines, do not parse function bodies)
  3. Discover files along use paths, add to the queue
  4. For files in the queue, parse their use statements
  5. Repeat 3-4 until the queue is empty (on-demand discovery)
  6. Fully parse all discovered files one by one → multiple ASTs
  7. Merge into a single Module (concatenate all top-level items, Span retains source file)
  8. Feed to Pipeline::run() (the pipeline does not know there are multiple files)
```

#### Intra-package circular references: allowed

Because all files are merged into a single AST, intra-package files using each other via `use` is
equivalent to cross-references within a single file:

```yaoxiang
// tree.yx
use node.{Node}
Tree: Type = { root: Node }

// node.yx
use tree.{Tree}
Node: Type = { value: Int, parent: Tree }
```

After merging, it is just two mutually referencing type definitions within a single AST. The
compiler already supports this.

#### Inter-package cycles: tbd

Packages are distribution units; inter-package requires topological order. Currently there is no
third-party package ecosystem, so this is not handled for now. Just report an error when
encountered.

#### Entry file selection

Priority:

1. `[run].main` (yaoxiang.toml)
2. The first `[[bin]]` entry's `path`
3. `src/main.yx` (default convention)

Without `yaoxiang.toml`: directly compile the given file. The Registry only has std. This is not a
"single-file mode"—it is the "natural result when discovery yields nothing".

#### Registry on-demand loading

The Registry's contents = all modules reachable from the entry point via `use`. Unreachable modules
are not parsed, not registered, do not exist. **This is not an optimization, it is the definition.**

### 6. std modules and user modules are homogeneous

For typecheck, `use std.io.{println}` and `use math.geometry.{Point}` operate identically:

1. Find the module record in the Registry
2. Take the field
3. Bind it to the current scope

Source (Std / User / Vendor) is metadata and does not affect resolution logic. Special handling of
native functions is deferred to the IR gen / codegen layer.

## Compiler changes

| Component                                    | Change                                                                                                                                                                                                |
| -------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `frontend/module/resolver.rs`                | **Rewrite**: currently predicate strictification (RFC-027), moves to `frontend/core/types/eval/`. This file becomes the real module path resolver (migrated from `package/source/module_resolver.rs`) |
| `frontend/module/mod.rs`                     | Extension: add source file tracking required for AST merging (Span with file name)                                                                                                                    |
| `frontend/module/registry.rs`                | Extension: support registering user modules (currently only registers std)                                                                                                                            |
| `frontend/module/orchestrator.rs`            | **New**: multi-file orchestrator (discover → parse → merge → call Pipeline)                                                                                                                           |
| `frontend/pipeline.rs`                       | **No change**                                                                                                                                                                                         |
| `frontend/core/parser/statements/imports.rs` | No change (`use` parsing is already implemented)                                                                                                                                                      |
| `package/source/module_resolver.rs`          | **Delete**, logic migrates to `frontend/module/resolver.rs`                                                                                                                                           |
| `frontend/core/typecheck/`                   | `use` handling changed to query the Registry (currently only queries std)                                                                                                                             |
| AST `is_pub: bool`                           | ~~No change~~ (029g has reversed: to be removed together with the `pub` keyword, 2026-10-02, pending implementation)                                                                                  |

### Nonexistent files (older RFC versions claimed "already implemented" but do not actually exist)

- ~~`frontend/module/loader.rs`~~ — does not exist, responsibilities handled by the orchestrator
- ~~`frontend/module/dep_graph.rs`~~ — does not exist, intra-package does not need topological
  sorting (merged AST)
- ~~`frontend/module/cache.rs`~~ — does not exist, belongs to sub-RFC 029a
- ~~`frontend/module/hot_reload.rs`~~ — does not exist, belongs to sub-RFC 029b

## Implementation strategy

### Phase 1: Unify path resolution

1. Move predicate strictification from `frontend/module/resolver.rs` to `frontend/core/types/eval/`
2. Migrate the path resolution logic from `package/source/module_resolver.rs` to
   `frontend/module/resolver.rs`
3. Module path ambiguity detection (both `name.yx` and `name/mod.yx` exist → report error)

### Phase 2: Multi-file orchestrator

4. Create `frontend/module/orchestrator.rs`
5. Implement on-demand discovery (recursive from the entry along use)
6. Implement AST merging (concatenate items from multiple files, Span carries source file)
7. Add `compile_project(project_root)` to `compiler.rs` to call the orchestrator

### Phase 3: use name resolution

8. typecheck's `process_use_stmt` changed to query the Registry (no longer only querying std)
9. Import conflict detection (same name → report error)
10. E2E tests: multi-file project `use` local modules

### Phase 4 (optional, does not block the main line)

11. `use path.{item as alias}` alias inside braces
12. Vendor directory resolution (in coordination with RFC-014)

### Dependencies

- RFC-014 (package manager) — `yaoxiang.toml` fields, vendor directory structure (only needed for
  Phase 4)
- No other prerequisites

## Sub-RFC planning

| Sub-RFC | Capability                                                                                        | Prerequisite           |
| ------- | ------------------------------------------------------------------------------------------------- | ---------------------- |
| 029a    | Module caching and incremental recompilation                                                      | Orchestrator stable    |
| 029b    | File watching and hot reload                                                                      | 029a                   |
| 029d    | CLI `--entry` override entry                                                                      | Orchestrator available |
| 029e    | Multi-file diagnostic `--json` output                                                             | Diagnostic aggregation |
| 029f    | Compilation target role and import surface semantics (accepted 2026-09-13, #334)                  | Orchestrator stable    |
| 029g    | Remove `pub` keyword and auto-binding (final ruling on visibility mechanism; accepted 2026-10-02) | None                   |

Removed: ~~029c (re-export)~~ — Not needed. `use` is re-export, no "pub use" concept.

## Design decision record

| Decision               | Conclusion                                 | Date       | Rationale                                                                                    |
| ---------------------- | ------------------------------------------ | ---------- | -------------------------------------------------------------------------------------------- |
| What is a module       | A record of file top-level bindings        | 2026-07-30 | RFC-010 `name: type = value` unified model                                                   |
| `use` semantics        | Record destructuring                       | 2026-07-30 | No new mechanism; reuses existing record semantics                                           |
| Visibility             | Does not exist                             | 2026-07-30 | Scope + distribution boundary covers all cases; no new keyword needed                        |
| `pub` keyword          | Not wanted                                 | 2026-07-30 | "If you don't want others to use it, don't put it at top level / don't publish that package" |
| mod.yx semantics       | Directory entry (convention, not enforced) | 2026-07-30 | Python `__init__.py` model: the door is not locked                                           |
| Module type annotation | Not needed                                 | 2026-07-30 | Internal bindings already carry types; annotation is redundant                               |
| Intra-package cycles   | Allowed (merged AST)                       | 2026-07-30 | Path A: zero changes to the pipeline; Rust crate internal model                              |
| Inter-package cycles   | Not handled for now                        | 2026-07-30 | No third-party ecosystem; just report an error when encountered                              |
| Registry loading       | On demand (only reachable modules loaded)  | 2026-07-30 | Not an optimization, it is the definition                                                    |
| Single file vs project | Same mechanism                             | 2026-07-30 | Registry contents differ, lookup logic is the same                                           |

## References

- [RFC-010: Unified Type Syntax](010-unified-type-syntax.md) — `name: type = value` model
- [RFC-009: Ownership Model](009-ownership-model.md) — Imports are compile-time name resolution
- [RFC-011: Generic Type System](011-generic-type-system.md) — Structured types
- [RFC-014: Package Manager System Design](014-package-manager.md) — Package names, vendor directory
- [RFC-026: FFI Core Mechanism](026-ffi-core-mechanism.md) — StdModule registration
- [RFC-030: assert Mechanism](030-assert-mechanism.md) — StdModule unified registration precedent

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

**Core Definition**: A module = all top-level bindings of a `.yx` file. The type of a module = the
types of those bindings (inferred). `use` = record destructuring. No `pub`, no `private`, no
`export`, no visibility mechanism.

**Core Principles**:

- The type checker only queries the pre-built ModuleRegistry, never touches the disk
- Files within a package are merged into a single compilation unit (AST splicing), naturally
  allowing intra-package circular references
- The Registry loads on demand: only modules reachable from the entry point along `use`

**Not Included**: caching, file watching, hot reload, incremental recompilation, cross-package
circular dependency handling.

## Motivation

### Current Problems

1. **The compiler only supports single files**: `Pipeline::run(name, source)` takes a string and
   cannot handle cross-file dependencies
2. **`use` can only resolve std modules**: `use` between local files all report "Unknown variable"
3. **The module resolver is in the wrong place**: The sole path resolution logic is in
   `package/source/module_resolver.rs`; `frontend/module/resolver.rs` is actually compile-time
   predicate normalization (RFC-027)

### Design Goals

- A project can compile multiple `.yx` files
- Clear semantics for `use` statements: record destructuring, not a special mechanism
- Single-file mode continues to work, no `yaoxiang.toml` required
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

The content of this module is `{ Point: Type, distance: (Point, Point) -> Float }`.

A module is not a special entity. It is an instance of the `name: type = value` model—a record that
happens to be defined at a file boundary. The module's type is inferred from the bindings, never
requiring an explicit annotation.

Bindings introduced by `use` **are also** part of the module's content:

```yaoxiang
// math/mod.yx
use geometry.{Point, distance}
```

The content of the `math` module = `{ Point: Type, distance: (Point, Point) -> Float }`. An external
`use math.{Point}` can get `Point`. `use math.geometry.{Point}` can also get it. Both paths point to
the same binding.

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

| Syntax              | Semantics                                                                |
| ------------------- | ------------------------------------------------------------------------ |
| `use path.{item}`   | Take the `item` field of the `path` record, bind it to the current scope |
| `use path.{a, b}`   | Take multiple fields                                                     |
| `use path`          | Take the `path` record itself, bind it to the last segment name          |
| `use path as alias` | Take the `path` record itself, bind it to `alias`                        |

#### Non-existent Syntax

- ~~`use path.*`~~: Wildcard import. Not needed; list bindings explicitly.
- ~~`from path use item`~~: Python-style. Not adopted.
- ~~`use path.{item as alias}`~~: Alias inside braces. Phase 4 optional, does not block the main
  path.

#### Import Conflicts

Same-name bindings report an error directly:

```
Name `Point` conflict:
  math.geometry.Point
  graphics.shapes.Point
Please use different names or module aliases.
```

### 3. Visibility: Does Not Exist

**This RFC does not introduce any visibility mechanism.** All top-level bindings are visible to any
code that can write a path.

This is an intentional design decision, not an oversight.

#### Design Rationale

| What you want to express    | How to do it                      | Mechanism    |
| --------------------------- | --------------------------------- | ------------ |
| "This is an API"            | Put it in the published package   | Distribution |
| "This is internal"          | Put it in an unpublished package  | Distribution |
| "This is inside a function" | Write it inside the function body | Scope        |

All three layers are existing mechanisms: package, file, scope. Nothing new needed.

#### Why Not `pub`

- Things you don't want others to use shouldn't be at the top level (put them in a local scope)
- Helper functions shared across multiple files go in an independent unpublished package
- "Whether you can prevent it" and "whether there should be a signal" are two different things. At
  the current stage there's no third-party ecosystem, so signals are meaningless
- Doors don't lock. Going through the door is polite, climbing over the wall is free. The language
  doesn't enforce politeness

#### Future

When the ecosystem matures and forced boundaries are needed, a separate RFC can introduce them.
Adding restrictions is backward-compatible (default public → explicit marking as internal). But this
RFC does not presuppose this direction, nor promise it will happen.

### 4. Path Resolution

#### Module Path → File

```
use math.geometry.{Point}
```

Lookup order:

1. **Already registered in the Registry**: `math.geometry` is already in the Registry → use it
   directly
2. **Standard library**: `std` or `std.*` → built-in module
3. **Importer's directory**: `<importer_dir>/math/geometry.yx` (local modules take priority)
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
Please delete one.
```

If the same module key hits a file in **each of two roots** and both are referenced (e.g.,
`tests/lib.yx` and `<root>/lib.yx` referenced by the tests/ entry and the root entry respectively) →
likewise report an ambiguity error, not silent shadowing.

> 2026-08-03 revision (RFC-036 driven): discovery and resolution land per implementation. Discovery
> follows `use` tracking (the protocol established in §5 of this RFC), replacing the
> directory-recursion implementation from the initial version—compilation errors in unrelated files
> no longer block execution, which is what makes `yaoxiang test`'s test file isolation viable.
> Within the dual-root scheme, "importer's directory first" preserves the same-directory project's
> behavior; "project root fallback" lets subdirectory entries (e.g., `tests/foo_test.yx`) import
> project-root modules. The `src/` layout (RFC-014 package) is handled together at the vendor layer
> when it lands, and does not affect local dual-root behavior.

#### mod.yx = Directory Entry (Convention)

`mod.yx` is the entry file of a directory. `use math` loads `src/math/mod.yx`.

This is a **convention, not an enforcement**. Users can directly `use math.geometry` to penetrate
into a subfile. `mod.yx` is the "recommended entry" (door plate), not the "sole entry" (lock).

#### Unified Resolver

The current sole path resolution logic is in `package/source/module_resolver.rs`. Move it to
`frontend/module/resolver.rs` (replacing the current misnamed predicate-normalization file;
predicate normalization moves to `frontend/core/types/eval/`).

### 5. Project Compilation Flow

#### Path A: Merge AST

All files in the package are merged into a **single compilation unit**. Zero changes to the
pipeline.

```
Orchestrator (above Pipeline):
  1. Determine the entry file
  2. Parse the entry file's use statements (only read the use lines, don't parse function bodies)
  3. Discover files along the use path, add to the queue
  4. For files in the queue, parse their use statements
  5. Repeat 3-4 until the queue is empty (on-demand discovery)
  6. Fully parse all discovered files one by one → multiple ASTs
  7. Merge into one Module (all top-level items concatenated, Span retains the source file)
  8. Feed to Pipeline::run() (the pipeline doesn't know there are multiple files)
```

#### Intra-package Circular References: Allowed

Since all files are merged into one AST, files in the same package using each other with `use` is
equivalent to mutual references within the same file:

```yaoxiang
// tree.yx
use node.{Node}
Tree: Type = { root: Node }

// node.yx
use tree.{Tree}
Node: Type = { value: Int, parent: Tree }
```

After merging, it becomes two type definitions in one AST that reference each other. The compiler
already supports this.

#### Inter-package Circular: TBD

Packages are distribution units, so inter-package requires a topological order. Currently there's no
third-party package ecosystem, so handle later. Just report an error when encountered.

#### Entry File Selection

Priority:

1. `[run].main` (yaoxiang.toml)
2. The `path` of the first item in `[[bin]]`
3. `src/main.yx` (convention default)

Without `yaoxiang.toml`: compile the given file directly. The Registry only has std. This is not
"single-file mode"—it is "the natural result of an empty discovery".

#### Registry Loads on Demand

Registry content = all modules reachable from the entry along `use`. Unreachable modules are not
parsed, not registered, don't exist. **This is not an optimization; it is the definition.**

### 6. std Modules and User Modules Are Homogeneous

From the typechecker's perspective, `use std.io.{println}` and `use math.geometry.{Point}` operate
exactly the same way:

1. Find the module record in the Registry
2. Get the field
3. Bind to the current scope

Source (Std / User / Vendor) is metadata, and does not affect resolution logic. Special handling of
native functions is deferred to the IR gen / codegen layer.

## Compiler Changes

| Component                                    | Change                                                                                                                                                                                             |
| -------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `frontend/module/resolver.rs`                | **Rewrite**: currently predicate normalization (RFC-027), move to `frontend/core/types/eval/`. This file becomes the real module path resolver (migrated from `package/source/module_resolver.rs`) |
| `frontend/module/mod.rs`                     | Extend: add source file tracking needed for AST merging (Span carries filename)                                                                                                                    |
| `frontend/module/registry.rs`                | Extend: support registering user modules (currently only registers std)                                                                                                                            |
| `frontend/module/orchestrator.rs`            | **New**: multi-file orchestrator (discover → parse → merge → call Pipeline)                                                                                                                        |
| `frontend/pipeline.rs`                       | **Unchanged**                                                                                                                                                                                      |
| `frontend/core/parser/statements/imports.rs` | Unchanged (`use` parsing is already implemented)                                                                                                                                                   |
| `package/source/module_resolver.rs`          | **Delete**; logic migrated to `frontend/module/resolver.rs`                                                                                                                                        |
| `frontend/core/typecheck/`                   | `use` handling changes to query the Registry (currently only queries std)                                                                                                                          |
| AST `is_pub: bool`                           | ~~Unchanged~~ (029g ruling overturns: removed together with the `pub` keyword deletion, 2026-10-02, pending implementation)                                                                        |

### Non-existent Files (RFC old version claimed "implemented" but actually doesn't exist)

- ~~`frontend/module/loader.rs`~~ — doesn't exist; responsibilities taken over by the orchestrator
- ~~`frontend/module/dep_graph.rs`~~ — doesn't exist; no topological sort needed within a package
  (AST merge)
- ~~`frontend/module/cache.rs`~~ — doesn't exist; belongs to sub-RFC 029a
- ~~`frontend/module/hot_reload.rs`~~ — doesn't exist; belongs to sub-RFC 029b

## Implementation Strategy

### Phase 1: Unified Path Resolution

1. Move predicate normalization from `frontend/module/resolver.rs` to `frontend/core/types/eval/`
2. Migrate the path resolution logic from `package/source/module_resolver.rs` to
   `frontend/module/resolver.rs`
3. Module path ambiguity detection (`name.yx` and `name/mod.yx` both exist → error)

### Phase 2: Multi-file Orchestrator

4. Create `frontend/module/orchestrator.rs`
5. Implement on-demand discovery (recursive from the entry along use)
6. Implement AST merging (multi-file item concatenation, Span carries source file)
7. `compiler.rs` adds `compile_project(project_root)` to call the orchestrator

### Phase 3: use Name Resolution

8. typecheck's `process_use_stmt` changes to query the Registry (no longer only std)
9. Import conflict detection (same-name error)
10. E2E test: multi-file project `use` local modules

### Phase 4 (Optional, does not block the main line)

11. `use path.{item as alias}` alias inside braces
12. vendor directory resolution (with RFC-014)

### Dependencies

- RFC-014 (package manager) — `yaoxiang.toml` fields, vendor directory structure (only needed for
  Phase 4)
- No other prerequisites

## Sub-RFC Planning

| Sub-RFC | Capability                                                                                        | Prerequisite           |
| ------- | ------------------------------------------------------------------------------------------------- | ---------------------- |
| 029a    | Module caching and incremental recompilation                                                      | Orchestrator stable    |
| 029b    | File watching and hot reload                                                                      | 029a                   |
| 029d    | CLI `--entry` to override entry                                                                   | Orchestrator available |
| 029e    | Multi-file diagnostic `--json` output                                                             | Diagnostic aggregation |
| 029f    | Compilation target roles and import surface semantics (accepted 2026-09-13, #334)                 | Orchestrator stable    |
| 029g    | Remove `pub` keyword and auto-binding (final ruling on visibility mechanism; accepted 2026-10-02) | None                   |

Deleted: ~~029c (re-export)~~ — not needed. `use` is re-export; there is no "pub use" concept.

## Design Decision Record

| Decision               | Conclusion                                 | Date       | Basis                                                                                            |
| ---------------------- | ------------------------------------------ | ---------- | ------------------------------------------------------------------------------------------------ |
| What a module is       | The record of a file's top-level bindings  | 2026-07-30 | RFC-010 `name: type = value` unified model                                                       |
| `use` semantics        | Record destructuring                       | 2026-07-30 | Don't introduce new mechanisms; reuse existing record semantics                                  |
| Visibility             | Does not exist                             | 2026-07-30 | Scope + distribution boundary cover all scenarios; no new keyword needed                         |
| `pub` keyword          | No                                         | 2026-07-30 | "If you don't want others to use it, don't put it at the top level / don't publish that package" |
| mod.yx semantics       | Directory entry (convention, not enforced) | 2026-07-30 | Python `__init__.py` model: door doesn't lock                                                    |
| Module type annotation | Not needed                                 | 2026-07-30 | Internal bindings already carry types; annotation is redundant                                   |
| Intra-package circular | Allowed (AST merge)                        | 2026-07-30 | Path A: zero pipeline changes; Rust crate internal model                                         |
| Inter-package circular | Handle later                               | 2026-07-30 | No third-party ecosystem; just error when encountered                                            |
| Registry loading       | On demand (only load reachable modules)    | 2026-07-30 | Not an optimization; it is the definition                                                        |
| Single file vs project | Same mechanism                             | 2026-07-30 | Different Registry content, same lookup logic                                                    |

## References

- [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md) — `name: type = value`
  model
- [RFC-009: Ownership Model](../accepted/009-ownership-model.md) — import is compile-time name
  resolution
- [RFC-011: Generic Type System](../accepted/011-generic-type-system.md) — structural types
- [RFC-014: Package Management System Design](../accepted/014-package-manager.md) — package names,
  vendor directory
- [RFC-026: FFI Core Mechanism](../accepted/026-ffi-core-mechanism.md) — StdModule registration
- [RFC-030: assert Mechanism](../accepted/030-assert-mechanism.md) — StdModule unified registration
  precedent

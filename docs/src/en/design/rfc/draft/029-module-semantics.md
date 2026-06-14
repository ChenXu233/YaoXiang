---
title: "RFC-029: Module Semantics System"
status: "Draft"
author: "Chenxu"
created: "2026-06-13"
updated: "2026-06-13 (Removed orphan rules / coherence checks, focused on module integration into the compilation pipeline)"
---

# RFC-029: Module Semantics System

## Summary

Integrate the module system into the compilation pipeline, enabling multi-file compilation, module-level visibility control, and hot reload. **No orphan rules or coherence checks are introduced**—YaoXiang's traits are structural types (RFC-011 §2.1), so Rust-style nominal impl attribution tracking is unnecessary.

## Motivation

### Current Issues

The **physical layer** of the module system (loading, parsing, caching, dependency graph, hot reload) has been fully implemented (`frontend/module/`), but is **not yet integrated into the compilation pipeline**:

- `pipeline.rs` only accepts a single source string and does not support multi-file projects
- `use` statements cannot actually load modules during type checking
- `ModuleCache`, `HotReloader`, `VendorLoader` are implemented but have no callers
- Standard library native functions are hard-coded via `ModuleRegistry::with_std()`, not going through the generic module loading path

### Why Orphan Rules Are Not Needed

RFC-011 defines traits as structural types:

```yaoxiang
Clone: Type = { clone: (Self) -> Self }
```

- **No `impl Trait for Type`** — methods are defined directly on the type
- **No orphan rules** — any module can add methods to its own types
- **No coherence checks** — methods are part of the type's structure, not matched nominally

Therefore `TraitImplementation` does not need `defined_in` or `module` fields. Related issues #46 and #73 have been closed.

## Proposal

### Core Design

**Two-layer integration**:

```text
┌─────────────────────────────────────────────────┐
│  Pipeline Integration                            │  ← New: multi-file compilation, module loading
├─────────────────────────────────────────────────┤
│  Visibility                                      │  ← New: pub / default (visible within module)
└─────────────────────────────────────────────────┘
```

### 1. Multi-file Compilation

The compiler entry point is extended from a single file to a project directory:

```rust
/// Compile a project (rather than a single file)
pub fn compile_project(&mut self, project_root: &Path) -> Result<Vec<ModuleIR>, CompileError> {
    // 1. Read yaoxiang.toml to get the entry file
    // 2. Recursively load dependent modules from the entry file
    // 3. Topologically sort the dependency graph
    // 4. Compile each module in order
    // 5. Cross-module type checking (use statement resolution)
}
```

Integration point: `compiler.rs` adds a `compile_project` method, internally using `ModuleLoader` to load modules.

### 2. Module Resolution for use Statements

Currently `statements.rs` has `ModuleRegistry` but it only performs registration queries. It needs to be extended to actually perform loading:

```yaoxiang
# Current: use statements cannot find modules during type checking
use math.geometry.Point  # ❌ ModuleRegistry has no math.geometry

# Goal: use statements trigger module loading
use math.geometry.Point  # ✅ ModuleLoader loads math/geometry.yx, extracts Point export
```

Implementation path:
1. `use` statements trigger `ModuleLoader::load()`
2. Load results are registered in `ModuleRegistry`
3. The type checker queries the `ModuleRegistry` for exported types

### 3. Visibility System

```yaoxiang
# math/geometry.yx
pub type Point = { x: Int, y: Int }       # pub = accessible from other modules
type InternalState = { cache: Int }        # default = visible only within geometry module

pub Point.distance: (self: Point, other: Point) -> Float = {
    # ...
}
```

```rust
/// Visibility level
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Visibility {
    /// Public — accessible by all modules
    Public,
    /// Default — visible only within the defining module
    Module,
}
```

The type checker checks visibility on cross-module references.

### 4. Module Cache

`ModuleCache` has already implemented LRU/TTL caching strategies. After integrating into the compilation pipeline:
- First compilation: load + compile + cache
- Subsequent compilations: skip if cache hit
- File changes: `HotReloader` automatically invalidates dirty caches

### 5. Hot Reload Integration

`HotReloader` has been fully implemented (`frontend/module/hot_reload.rs`) and needs to be integrated into the compilation pipeline:

```rust
// When starting the compilation pipeline
let mut reloader = HotReloader::new(project_root, config, cache.clone());
let mut event_rx = reloader.start()?;

// In the async main loop
tokio::spawn(async move {
    while let Some(event) = event_rx.recv().await {
        for module in &event.affected_modules {
            pipeline.recompile_module(module).await;
        }
    }
});
```

## Compiler Changes

| Component | Change |
|------|------|
| `compiler.rs` | Add `compile_project` method |
| `pipeline.rs` | Support multi-module compilation, module cache queries |
| `typecheck/inference/statements.rs` | `use` statements trigger module loading |
| `typecheck/mod.rs` | Register native functions from generic module paths (replacing hard-coding) |
| `frontend/module/loader.rs` | Already implemented, no changes needed |
| `frontend/module/cache.rs` | Already implemented, no changes needed |
| `frontend/module/hot_reload.rs` | Already implemented, no changes needed |
| AST layer | `pub` keyword for type visibility annotation (if not yet supported) |

## Implementation Strategy

### Phased Plan

**Phase 1: Multi-file Compilation Entry Point**
1. Add `compile_project(project_root)` method to `compiler.rs`
2. Use `ModuleLoader` to recursively load dependencies from the entry file
3. Use `ModuleDependencyGraph` for topological sort
4. Invoke the existing single-file compilation flow in order

**Phase 2: use Statement Module Resolution**
5. `use` statements in `statements.rs` trigger `ModuleLoader::load()`
6. Load results are registered in `ModuleRegistry`
7. Exported types are available during type checking

**Phase 3: Visibility**
8. Parse `pub` annotations on types at the AST layer
9. Type checker checks visibility on cross-module references

**Phase 4: Cache and Hot Reload**
10. `pipeline.rs` integrates `ModuleCache`
11. `pipeline.rs` integrates `HotReloader`
12. Incremental recompilation only handles affected modules

### Dependencies

- RFC-014 (Package Manager) — package names come from `yaoxiang.toml`
- RFC-011 (Generics System) — traits are structural types, no module attribution involved

## Open Questions

- [ ] Should default visibility be "module-internal" or "package-internal"? (Rust defaults to module-internal, Go defaults to package-internal)
- [ ] Is a `pub(crate)` level needed?
- [ ] Should hot reload support recompilation across module dependency chains?
- [ ] How should multi-file compilation error reports be aggregated?

---

## References

- [RFC-011: Generic Type System](accepted/011-generic-type-system.md) — Structural type definition
- [RFC-014: Package Management System Design](accepted/014-package-manager.md) — Package name source
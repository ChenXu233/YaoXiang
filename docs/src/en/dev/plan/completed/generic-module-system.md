# YaoXiang General Module System Refactoring Plan

## 1. Overview

The current YaoXiang module system is a special implementation targeting `std` builtin modules. It contains multiple hardcoded sections and does not support user-defined modules. This document describes how to implement a general-purpose module system.

## 2. Acceptance Criteria

### 2.1 Required Features

| Feature | Syntax Example | Description |
|---------|---------------|-------------|
| Module import | `use my_module` | Load module from filesystem |
| Selective import | `use my_module.{func1, func2}` | Import only specified functions |
| Module alias | `use my_module as m` | Access via alias |
| Module call | `my_module.func()` | Call function through module |
| Submodule | `my_module.sub.func()` | Access submodule |
| Module attribute | `my_module.CONST` | Access module constant |

### 2.2 std Module Compatibility

Must maintain the same behavior after refactoring:

| Syntax | Example | Status |
|--------|---------|--------|
| `std.io.print()` | `use std.io` + `std.io.println("x")` | ✅ Must maintain |
| `print()` (selective import) | `use std.io.{print}` + `print("x")` | ✅ Must maintain |
| `io.print()` (module import) | `use std.{io}` + `io.println("x")` | ✅ Must maintain |
| `std.math.PI` | Constant access | ✅ Must maintain |

### 2.3 Module Loading Rules

```
// Module search paths (relative to current file)
1. ./my_module.yx           // Current directory
2. ./my_module/mod.yx       // Subdirectory
3. ./my_module/index.yx     // index file

// std module special handling
// std module is located at src/std/, always available
```

## 3. Testing Requirements

### 3.1 Functional Tests

```yaoxiang
// test_user_module.yx
// Assumes my_module.yx file exists

// 1. Basic module import
use my_module
main = {
    my_module.greet()  // Call module function
}

// 2. Selective import
use my_module.{add, sub}
main = {
    add(1, 2)
}

// 3. Module alias
use my_module as m
main = {
    m.greet()
}

// 4. Submodule
use my_module.utils
main = {
    my_module.utils.help()
}

// 5. Nested import
use my_module.{sub1, sub2.sub3}
```

### 3.2 std Module Compatibility Tests

All existing test cases must continue to pass:

```yaoxiang
// Existing syntax tests
use std.io
main = { std.io.println("test") }

use std.io.{print}
main = { print("test") }

use std.{io}
main = { io.println("test") }

use std.math
main = { std.math.PI }
```

### 3.3 Boundary Case Tests

- Empty module import
- Module circular dependency (should report error)
- Non-existent module (should report error)
- Duplicate import
- Import conflict

## 4. Existing Code Analysis

### 4.1 Code Smell Locations

| File | Line Number | Issue |
|------|-------------|-------|
| `src/std/mod.rs` | 29-100 | Hardcoded `get_module_exports`, only handles std modules |
| `src/frontend/typecheck/mod.rs` | 593-622 | Hardcoded std module handling logic |
| `src/frontend/typecheck/inference/expressions.rs` | 515-598 | Hardcoded `io\|math\|net\|concurrent` list |
| `src/middle/core/ir_gen.rs` | 65-131 | Hardcoded namespace handling |

### 4.2 Key Structure Analysis

#### 4.2.1 ModuleExport (src/std/mod.rs)

```rust
pub struct ModuleExport {
    pub short_name: &'static str,      // Short name
    pub qualified_name: &'static str, // Full path
    pub signature: &'static str,      // Function signature
}
```

**Issue**: Only used for std modules, needs to be generalized for common module exports

#### 4.2.2 NativeDeclaration (src/std/io.rs etc.)

```rust
pub struct NativeDeclaration {
    pub name: &'static str,
    pub native_name: &'static str,
    pub signature: &'static str,
    pub doc: &'static str,
    pub implemented: bool,
}
```

**Explanation**: `NativeDeclaration` is designed for FFI (Foreign Function Interface), specifically for YaoXiang to call Rust functions. Users achieve interoperability with Rust through the std.ffi `native` function.

**No changes needed**: User modules are written in YaoXiang itself, and do not require NativeDeclaration. The module system only needs to load YaoXiang source files.

**Design separation**:
| Module Type | Implementation | Loading |
|-------------|---------------|---------|
| FFI module | Rust + NativeDeclaration | Builtin registration |
| User module | YaoXiang source files | Filesystem loading |

#### 4.2.3 StmtKind::Use (AST)

```rust
StmtKind::Use {
    path: String,           // Module path
    items: Option<Vec<String>>, // Imported items
    alias: Option<String>,     // Alias
}
```

**Current state**: Parser already supports complete syntax, type checking not fully implemented

## 5. Implementation Plan

### 5.1 Phase 1: Design General Module Interface

**Goal**: Define common module registration and query interfaces

**Files that may need changes**:
- New `src/frontend/module.rs` - Module system core interface

**Design content**:
```rust
// Module export item
pub struct ModuleExport {
    pub name: String,           // Export name
    pub full_path: String,     // Full path
    pub kind: ExportKind,      // Function/Constant/Submodule
    pub type_info: TypeInfo,   // Type information
}

// Module registry
pub trait ModuleRegistry {
    fn get_module(&self, path: &str) -> Option<Module>;
    fn register_module(&mut self, path: String, module: Module);
}

// Module loader
pub trait ModuleLoader {
    fn load_module(&self, path: &str) -> Result<Module, ModuleError>;
}
```

### 5.2 Phase 2: Implement Module Loader

**Goal**: Load user modules from filesystem

**Files that may need changes**:
- New `src/frontend/module/loader.rs` - Filesystem module loading
- Modify `src/frontend/compiler.rs` - Integrate module loading

**Implementation content**:
- Implement module search paths
- Implement module parsing (AST generation)
- Implement module caching

### 5.3 Phase 3: Refactor Type Checking

**Goal**: Replace hardcoded logic with general module system

**Files that may need changes**:
- `src/frontend/typecheck/mod.rs` - Use common module interface
- `src/frontend/typecheck/inference/expressions.rs` - Remove hardcoded logic
- `src/std/mod.rs` - Implement ModuleRegistry trait

### 5.4 Phase 4: Refactor IR Generation

**Goal**: Remove hardcoded namespace handling

**Files that may need changes**:
- `src/middle/core/ir_gen.rs` - Use common module interface

### 5.5 Phase 5: std Module Adaptation

**Goal**: Ensure std module compatibility

**Files that may need changes**:
- `src/std/mod.rs` - Adapt to new module interface

### 5.6 Phase 6: Testing and Validation

**Goal**: Ensure all features work correctly

**Files that may be new**:
- `tests/modules/` - Module system tests

## 6. Architecture Design (Draft)

```
┌─────────────────────────────────────────────────────────────────┐
│                        Compiler Frontend                         │
├─────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐   │
│  │   Parser     │───▶│ TypeChecker  │───▶│  IR Gen      │   │
│  └──────────────┘    └──────────────┘    └──────────────┘   │
│         │                   │                   │              │
│         ▼                   ▼                   ▼              │
│  ┌──────────────────────────────────────────────────────┐    │
│  │                   Module System                       │    │
│  ├──────────────────────────────────────────────────────┤    │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐ │    │
│  │  │  Registry   │  │  Loader     │  │  Resolver   │ │    │
│  │  │  (模块注册)  │  │  (加载器)   │  │  (名称解析)  │ │    │
│  │  └─────────────┘  └─────────────┘  └─────────────┘ │    │
│  └──────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
```

## 7. Circular Dependency Handling

### 7.1 Design Decision: Fail Fast

The module system adopts a **fail fast** strategy, terminating compilation immediately when a circular dependency is detected and displaying a clear error message.

**Rationale**:
- Clear: Users immediately know where the problem is
- Helpful: Error message can display the circular path
- Intuitive: Similar to how languages like Rust/Go handle this

### 7.2 Detection Algorithm

```rust
// 1. Build dependency graph
//    Each module -> list of modules it depends on

// 2. Topological sort (Kahn's algorithm)
//    - Nodes with in-degree 0 have priority
//    - Cannot sort = cycle exists

// 3. If sorting fails, report circular dependency
```

### 7.3 Cycle Types

#### Direct Cycle
```yaoxiang
// a.yx
use b

// b.yx
use a  // Error: Circular dependency a <-> b
```

#### Indirect Cycle
```yaoxiang
// a.yx
use b

// b.yx
use c

// c.yx
use a  // Error: Indirect cycle a -> b -> c -> a
```

#### Self Reference
```yaoxiang
// a.yx
use a  // Error: Self reference
```

### 7.4 Error Message Example

```yaoxiang
error[E1001]: cyclic dependency detected
  --> a.yx:1:1
   |
1  | use b
   | ^^^^^^
   |
note: dependency path: a -> b -> c -> a
help: break the cycle by removing one of these imports
```

### 7.5 Implementation Location

- **Module loader** (`src/frontend/module/loader.rs`)
- Detected immediately after module graph construction
- Detected early in compilation to avoid wasted subsequent work

## 8. Other Constraints

### 8.1 Module Versioning

Module version management is not considered at this time.

### 8.2 Conditional Compilation

Conditional compilation (feature flags) is not considered at this time.

## 9. Module Caching Strategy

### 9.1 Design Goals

Reference RFC-014 project structure, supporting hot reload during development:

```
my-project/
├── yaoxiang.toml
├── yaoxiang.lock
├── src/
│   └── main.yx
└── .yaoxiang/
    └── vendor/
        ├── foo-1.2.3/
        │   └── src/
        │       └── foo.yx
        └── bar-0.5.0/
```

### 9.2 Cache Types

| Cache Type | Timing | Strategy |
|-----------|--------|----------|
| **Compile-time cache** | During compilation | Memory cache, reused within same compilation unit |
| **Development cache** | During `yaoxiang run` runtime | Filesystem cache, auto-reload on file changes |
| **Release cache** | For release builds | Locked versions, no file change monitoring |

### 9.3 Hot Reload Mechanism

```rust
// Development mode: file watching
struct HotReloader {
    watcher: notify::Watcher,  // Filesystem watcher
}

// Trigger conditions:
// 1. Dependent .yx file changes
// 2. yaoxiang.toml or yaoxiang.lock changes

// Reload strategy:
// - Incrementally recompile changed modules
// - Rebuild dependency graph
// - Check for circular dependencies
```

### 9.4 Implementation Location

- **Module cache**: `src/frontend/module/cache.rs`
- **Hot reload**: `src/frontend/module/hot_reload.rs`
- **Integration points**: `src/frontend/compiler.rs`

---

## 10. Compile Error Message Optimization

### 10.1 Design Goals

Reference Rust's error message detail level, providing clear, helpful error hints.

### 10.2 Error Types and Examples

#### 10.2.1 Module Not Found

```yaoxiang
error[E1001]: module not found: 'my_module'
  --> main.yx:1:1
   |
1  | use my_module
   | ^^^^^^^^^^^^^^
   |
help: check if the file exists at one of these locations:
  - ./my_module.yx
  - ./my_module/index.yx
  - ./.yaoxiang/vendor/my_module-*/src/my_module.yx
note: you may need to add 'my_module' to your dependencies in yaoxiang.toml
```

#### 10.2.2 Function Not Exported

```yaoxiang
error[E1002]: export not found: 'undefined_func'
  --> main.yx:3:5
   |
3  | my_module.undefined_func()
   |     ^^^^^^^^^^^^^^^^^^^^
   |
help: 'my_module' exports these functions:
  - greet(name: String) -> String
  - add(a: i64, b: i64) -> i64
  - sub(a: i64, b: i64) -> i64
```

#### 10.2.3 Circular Dependency

```yaoxiang
error[E1003]: cyclic dependency detected
  --> a.yx:1:1
   |
1  | use b
   | ^^^^^
   |
note: dependency path: a -> b -> c -> a
help: break the cycle by removing one of these imports
```

#### 10.2.4 Path Resolution Error

```yaoxiang
error[E1004]: invalid module path
  --> main.yx:1:1
   |
1  | use ./relative/path
   | ^^^^^^^^^^^^^^^^^^
   |
help: use absolute module names (e.g., use my_module) or configure in yaoxiang.toml
```

### 10.3 Implementation Location

- **Error definitions**: `src/util/diagnostic/error_codes.rs`
- **Error generation**: Various module system components

---

## 11. Module Path Resolution Rules

### 11.1 Design Basis

Reference RFC-014 lines 125-135 for module resolution order design.

### 11.2 Path Syntax

| Syntax | Example | Description |
|--------|---------|-------------|
| Standard path | `use foo.bar` | Search vendor/ and src/ |
| Submodule | `use foo.bar.baz` | Nested module |
| Current module | `use .` | Current module (self) |
| Parent module | `use ..` | Parent module (parent) |

**Not supported**:
- Relative paths (`use ./utils`)
- Absolute paths (`use /usr/local/lib`)
- String package names (`use "@org/package"`)

### 11.3 Search Order

```
use foo.bar.baz;

Search order:
1. ./.yaoxiang/vendor/*/src/foo/bar/baz.yx  (vendor/ directory)
2. ./src/foo/bar/baz.yx                      (local modules)
3. $YXPATH/foo/bar/baz.yx                    (global path, reserved)
4. $YXLIB/std/foo/bar/baz.yx                 (standard library)
```

### 11.4 std Module Special Handling

```
use std.io          ->  mapped to $YXLIB/std/io/
use std.math        ->  mapped to $YXLIB/std/math/
```

### 11.5 Implementation Location

- **Module resolver**: `src/frontend/module/resolver.rs`
- **Path search**: `src/frontend/module/loader.rs`

---

## 12. Unified Interface Design

### 12.1 Design Goals

std modules and user modules use a unified module interface for easier extension.

### 12.2 Core Interfaces

```rust
// Module
pub trait Module {
    fn path(&self) -> &str;
    fn exports(&self) -> &HashMap<String, Export>;
}

// Module registry
pub trait ModuleRegistry {
    fn get(&self, path: &str) -> Option<Box<dyn Module>>;
    fn register(&mut self, path: String, module: Box<dyn Module>);
}

// Module loader
pub trait ModuleLoader {
    fn load(&self, path: &str) -> Result<Box<dyn Module>, ModuleError>;
}
```

### 12.3 Specific Implementations

| Implementation | Purpose |
|----------------|---------|
| `StdModule` | std standard library (builtin) |
| `UserModule` | User-defined modules (file loading) |
| `VendorModule` | .yaoxiang/vendor/ dependencies |
| `CompositeRegistry` | Combine multiple registries, query by search order |

### 12.4 Composite Registry

```rust
struct CompositeRegistry {
    // Search order: first one has priority
    std: StdModule,
    vendor: VendorRegistry,
    user: FileModuleRegistry,
}

impl ModuleRegistry for CompositeRegistry {
    fn get(&self, path: &str) -> Option<Box<dyn Module>> {
        // Try each registry in order
        self.std.get(path)
            .or_else(|| self.vendor.get(path))
            .or_else(|| self.user.get(path))
    }
}
```

### 12.5 Implementation Location

- **Interface definitions**: `src/frontend/module/mod.rs`
- **std implementation**: `src/frontend/module/std.rs`
- **File loading**: `src/frontend/module/file.rs`
- **Compositor**: `src/frontend/module/registry.rs`

---

## 13. Implementation Checklist

- [x] Module caching strategy (`src/frontend/module/cache.rs`)
- [x] Hot reload mechanism (`src/frontend/module/hot_reload.rs`)
- [x] Compile error messages (E5001-E5007 defined)
- [x] Module path resolution (`src/frontend/module/resolver.rs`)
- [x] Unified module interface (`src/frontend/module/mod.rs`)
- [x] std module adaptation (`src/frontend/module/registry.rs` + `src/std/mod.rs`)
- [x] Circular dependency detection (`src/frontend/module/loader.rs` Kahn's algorithm)
- [x] Type checking refactoring - Remove hardcoded logic (`src/frontend/typecheck/mod.rs`)
- [x] IR generation refactoring - Remove hardcoded logic (`src/middle/core/ir_gen.rs`)
- [x] Expression inference refactoring - Remove hardcoded logic (`src/frontend/typecheck/inference/expressions.rs`)
- [x] User module file loading (`src/frontend/module/loader.rs`, parse .yx files to extract exports)
- [x] RFC-014 integration (`src/frontend/module/vendor.rs`, vendor/, yaoxiang.toml)

---

## 14. Completed Implementation Records

### 14.1 Phase 1: General Module Interface (Completed ✅)

**New files**:
- `src/frontend/module/mod.rs` - Module system core type definitions (`Export`, `ModuleInfo`, `ModuleSource`, `ModuleError`, etc.)
- `src/frontend/module/registry.rs` - Unified module registry (`ModuleRegistry`), auto-discovers and registers std modules
- `src/frontend/module/resolver.rs` - Module path resolver (`ModuleResolver`), supports std/vendor/user module search
- `src/frontend/module/loader.rs` - Module loader (`ModuleLoader`), supports circular dependency detection

**Design decisions**:
- Use data-driven approach (`ModuleInfo` + `HashMap`) instead of trait objects to simplify interface
- `ModuleRegistry::with_std()` auto-discovers exports from each std module's `native_declarations()`
- `ModuleSource` enum distinguishes `Std`/`User`/`Vendor` module sources

### 14.2 Phase 2: Hardcoded Logic Elimination (Completed ✅)

**Modified files and eliminated hardcoded logic**:

| File | Eliminated Hardcoding | Replacement |
|------|----------------------|-------------|
| `src/std/mod.rs` | `match module_path { "std" => ..., "std.io" => ... }` | Delegated to `ModuleRegistry::with_std()` |
| `src/frontend/typecheck/mod.rs` | `let std_modules = ["std.io", "std.math", ...]` | Dynamically obtained via `registry.std_submodule_names()` |
| `src/frontend/typecheck/mod.rs` | `crate::std::get_module_exports(path)` in Use handling | Query via `self.env.module_registry.get(path)` |
| `src/frontend/typecheck/inference/expressions.rs` | `matches!(name.as_str(), "io" \| "math" \| "net" \| "concurrent")` | Dynamic check via `std_submodules` list |
| `src/middle/core/ir_gen.rs` | `use crate::std::{concurrent, io, math, net}` + manual map construction | Auto-generated `NATIVE_NAMES`/`SHORT_TO_QUALIFIED`/`STD_SUBMODULES` via `ModuleRegistry::with_std()` |

### 14.3 Phase 3: Error Code Definitions (Completed ✅)

**New error codes**:
- `E5005` - Invalid module path
- `E5006` - Duplicate import
- `E5007` - Module export hints

### 14.4 Phase 4: User Module File Loading (Completed ✅)

**Modified files**:
- `src/frontend/module/loader.rs` - Implemented `load_from_file()`, extracts exports via tokenize → parse → `extract_exports()`

**Export extraction rules**:
| Statement Type | Export Condition | ExportKind |
|---------------|-----------------|------------|
| `pub fn_name: ...` | `is_pub = true` | Function |
| `Name: Type = ...` | Always exported | Type |
| `name = expr` (immutable) | `is_mut = false` | Constant |
| `mut name = expr` | Not exported | — |

**New helper functions**:
- `format_type()` - Formats AST type nodes into signature strings

### 14.5 Phase 5: RFC-014 Integration (Completed ✅)

**New files**:
- `src/frontend/module/vendor.rs` - VendorBridge, bridges `PackageManifest` + `VendorManager` with module system

**Workflow**:
1. Read `yaoxiang.toml` to get declared dependencies
2. Scan `.yaoxiang/vendor/` directory to find installed dependencies
3. Parse entry file for each dependency and extract exports
4. Register with `ModuleRegistry`

### 14.6 Phase 6: Module Caching Strategy (Completed ✅)

**New files**:
- `src/frontend/module/cache.rs` - Thread-safe module cache (`parking_lot::RwLock`)

**Cache modes**:
| Mode | Change Detection | Use Case |
|------|-----------------|----------|
| `Compile` | None | Compile-time memory cache |
| `Development` | FNV-1a file hash | Auto-invalidate during development |
| `Release` | None | Production builds |

### 14.7 Phase 7: Hot Reload Mechanism (Completed ✅)

**New files**:
- `src/frontend/module/hot_reload.rs` - File watching + debouncing + cache invalidation

**New dependencies**:
- `notify = "7.0.0"` (filesystem watching)

**Architecture**:
```
FileWatcher (notify) → Debounce (300ms) → classify_events → invalidate_cache → ReloadEvent channel
```

**Event classification**:
| File Change | FileChange | Cache Handling |
|-------------|-----------|---------------|
| `.yx` modified | SourceModified | invalidate_by_file |
| `.yx` created | SourceCreated | No action needed |
| `.yx` deleted | SourceDeleted | invalidate_by_file |
| `yaoxiang.toml` | ManifestChanged | clear() |
| `yaoxiang.lock` | LockfileChanged | clear() |

### 14.8 Validation Results

- ✅ All 1494 tests passed (1458 unit + 6 doc + 30 integration, 0 failures)
- ✅ `cargo check` has no compile errors
- ✅ All checklist items completed
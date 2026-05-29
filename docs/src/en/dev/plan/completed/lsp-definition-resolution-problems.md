# LSP Definition Resolution Issues and Solutions

> **Task**: Fix LSP Go to Definition functionality
> **Date**: 2026-02-28
> **Status**: ✅ Completed
> **Found**: 2026-02-28
> **Completed**: 2026-03-01

---

## Overview

This document records issues and solutions related to the "Go to Definition" feature in the LSP server.

---

## Issue 1: Cannot Locate Symbols Imported via Use Statements

### Problem Description

Module symbols imported via `use std.list` statements (such as `push`, `pop`, etc.) cannot be found through the Go to Definition feature.

### Root Cause Analysis

**Location**: `src/lsp/world.rs:98-169`

```rust
// update_index_from_ast only processes these statement types
StmtKind::Var { ... }      // ✅ Processed
StmtKind::Fn { ... }       // ✅ Processed
StmtKind::TypeDef { ... }  // ✅ Processed
StmtKind::MethodBind { ... } // ✅ Processed
StmtKind::Use { ... }      // ❌ Not processed!
```

**Flow Issue**:
1. User code uses `use std.list` to import a module
2. Type checking phase registers imported symbols in `TypeEnvironment`
3. However, `update_index_from_ast` only parses top-level AST statements, **does not process Use statements**
4. Therefore, symbols imported via Use do not enter `SymbolIndex`
5. LSP Go to Definition cannot find these symbols

### Impact

- After users use `use std.list`, they cannot jump to definitions of functions like `std.list.push`
- Completion feature also cannot provide definitions for standard library functions

---

## Issue 2: Same-Name Functions Jump to Wrong Location

### Problem Description

When the same function name appears in multiple files, Go to Definition returns all definitions with the same name, making it impossible to precisely jump to the correct location.

### Root Cause Analysis

**Location**: `src/lsp/handlers/definition.rs:51-66`

```rust
// Current implementation: only matches by name, returns all same-name definitions
let symbols = world.symbol_index().find_by_name(&ident.name);

let locations: Vec<Location> = symbols
    .iter()
    .filter_map(|sym| { ... })  // Returns as long as name matches, without considering context
    .collect();
```

**Problem**:
1. `SymbolIndex` only contains top-level symbols (Var, Fn, TypeDef, MethodBind)
2. Lookup only matches by name, without considering:
   - Symbol type (variable vs function)
   - Symbol scope (local vs global)
   - Type context at call site
3. Returns all same-name definitions, client selects (usually first)

### Impact

- When same-name functions exist in multiple files, jumps may go to wrong locations
- Users need to manually select the correct definition

---

## Issue 3: Local Variables and Function Parameters Cannot Be Located

### Problem Description

Local variables and function parameters defined inside functions cannot be found through the Go to Definition feature.

### Root Cause Analysis

**Location**: `src/lsp/world.rs:107-168`

```rust
// update_index_from_ast only processes top-level module statements
for stmt in &module.items {
    match &stmt.kind {
        StmtKind::Var { name, .. } => {
            // Only processes top-level variables
        }
        StmtKind::Fn { name, params, .. } => {
            // Only processes function definitions, does not process parameters and local variables in function body
        }
        // ...
    }
}
```

**Problem**:
1. `SymbolIndex` only extracts symbols from **top-level module statements**
2. Symbols in nested scopes like function parameters and local variables are not indexed
3. LSP Go to Definition cannot find these symbols

---

## Issue 4: Standard Library Has No YaoXiang Source Files

### Problem Description

Standard library (std.list, std.io, etc.) is implemented in Rust and has no corresponding .yx source files.

### Root Cause Analysis

**Current Architecture**:

```
User Code (.yx)              Standard Library (Rust)
      │                              │
      ▼                              ▼
  Parse → AST              StdModule → NativeExport
      │                              │
      ▼                              ▼
 SymbolIndex ◄──────── ModuleRegistry
                          (Invisible to LSP)
```

**Problem**:
1. Standard library is registered to `ModuleRegistry` through `StdModule` trait
2. Each module has a `NativeExport` list containing:
   - `name`: short name (e.g., "push")
   - `native_name`: fully qualified name (e.g., "std.list.push")
   - `signature`: function signature (e.g., "(list: List, item: Any) -> List")
   - **Note**: No `Span` (Rust functions have no YaoXiang source location)
3. LSP server is unaware of `ModuleRegistry`'s existence
4. Therefore cannot locate definitions of standard library functions

---

## Solutions

### Solution A: Standard Library YaoXiang Interface Files

**Core Idea**: Create YaoXiang interface files for the standard library, binding to Rust functions using ExternalBindingStmt.

**File Structure**:
```
~/.yaoxiang/std/                    # Installation directory (global standard library)
├── list.yx                          # list module interface
│   push: (list: List, item: Any) -> List = ...
│   pop: (list: List) -> Any = ...
├── io.yx                            # io module interface
│   print: (...args) -> () = ...
│   println: (...args) -> () = ...
│   read_line: () -> String = ...
│   read_file: (path: String) -> String = ...
│   write_file: (path: String, content: String) -> Bool = ...
│   append_file: (path: String, content: String) -> Bool = ...
│   format_fallback: (value: Any, type_name: String) -> String = ...
├── dict.yx
├── string.yx
└── ...

Project Directory/
├── main.yx
└── .yaoxiang/
    └── vendor/
        └── std/                     # Project-local standard library (overrides global)
            └── list.yx              # Optional: overrides global list interface
```

**Interface File Format**:
```yaoxiang
// io.yx - Standard Library IO Module Interface
// For LSP navigation and type viewing only, not involved in actual execution


print: (...args) -> () = {
    // Output to standard output
    ... // Implementation provided by Rust
}


println: (...args) -> () = {
    // Output to standard output and newline
    ...
}


read_line: () -> String = {
    // Read a line from standard input
    ...
}

read_file: (path: String) -> String = {
    /* Read file content
    @param path file path */
    ...
}

write_file: (path: String, content: String) -> Bool = {
    /* Write file content, overwrite existing content
    @param path file path
    @param content file content
    @return whether successful */
    ...
}


append_file: (path: String, content: String) -> Bool = {
    /* Append file content
    @param path file path
    @param content file content
    @return whether successful */
    ...
}

format_fallback: (value: Any, type_name: String) -> String = {
    /* Format any type to string
    @param value any value
    @param type_name type name of the value
    @return formatted string */
    ...
}
```

**Syntax Notes**:
- Use `...` on the right side of the equals sign to skip actual implementation
- If function documentation needs to be added, use block syntax:
  ```yaoxiang
  print: (...args) -> () = {
      // Comment documentation
      ...
  }
  ```

**Module Lookup Order** (similar to Python):
```
use std.list lookup order:
1. project_dir/.yaoxiang/vendor/std/list.yx  ← Priority (used if exists)
2. ~/.yaoxiang/std/list.yx                   ← Fallback (default)
```

**Advantages**:
- User code and standard library use the same syntax
- LSP can directly parse these files to provide navigation and completion
- Standard library interface is self-documenting
- Easy to maintain
- Supports project-local overriding of global standard library

**Implementation Steps**:

1. **Automated Generation Tool**: Write a code generation tool to automatically generate `.yx` interface files from `NativeExport` in Rust code
   - Input: `NativeExport` definitions in `src/std/io.rs`
   - Output: `.yaoxiang/std/io.yx` interface files
   - Generation rules:
     - `name` → function name
     - `signature` → type annotation
     - `native_name` → not written (only used for Rust binding)

2. **Integrate into Build Process**: Automatically run generation tool during Cargo build
   ```rust
   // build.rs or standalone generation script
   fn main() {
       // Read src/std/*.rs
       // Parse NativeExport definitions
       // Generate .yaoxiang/std/*.yx files
   }
   ```

3. **Modify Module Resolution Logic**: Support dual-path lookup (project priority → global fallback)

4. **Modify LSP Server**: Load interface files into symbol index, provide navigation and completion

---

### Solution B: Fix Same-Name Function Precise Matching

**Core Idea**: Use type information and scope information from SemanticDB for precise matching.

**Resources Already Available**:
- `SemanticDB`: Contains more precise symbol information (type, scope)
- `symbol_defs`: symbol name → definition location list
- `symbol_refs`: symbol name → reference location list

**Implementation Steps**:
1. Modify `handle_definition` function to prioritize using `SemanticDB` for lookup
2. Use cursor position context (expression type, scope) for precise matching
3. Fall back to `SymbolIndex` if not found in `SemanticDB`

---

### Solution C: Handle Local Variables and Function Parameters

**Core Idea**: Add symbols inside functions to the index as well.

**Implementation Steps**:
1. Modify `update_index_from_ast` to traverse function bodies
2. Extract function parameters and local variables
3. Record scope level for each symbol
4. Use scope information for filtering during lookup

---

## Recommended Priority

| Priority | Issue | Solution |
|----------|-------|----------|
| P1 | Same-name function navigation error | Solution B: Use SemanticDB for precise matching |
| P2 | Local variables cannot be located | Solution C: Expand symbol index scope |
| P3 | Use imported symbols cannot be located | Solution A: Create standard library interface files |
| P4 | Standard library cannot be located | Solution A: Create standard library interface files |

---

## Related Code Locations

| File | Description |
|------|-------------|
| `src/lsp/handlers/definition.rs` | Go to Definition handler |
| `src/lsp/world.rs` | Symbol index update logic |
| `src/frontend/core/lexer/symbols.rs` | SymbolIndex definition |
| `src/frontend/typecheck/semantic_db.rs` | SemanticDB definition |
| `src/frontend/typecheck/mod.rs` | Type checker (processes Use statements) |
| `src/std/mod.rs` | Standard library module definition |
| `src/frontend/module/registry.rs` | Module registry |

---

## Implementation Process

### Issue Dependency Graph

```
┌─────────────────────────────────────────────────────────────┐
│                      Implementation Dependency Graph        │
└─────────────────────────────────────────────────────────────┘

Issue 4: Standard Library Interface Files
    │
    │  ┌─────────────────────────────────────────────────────┐
    │  │ 1. Automated generation tool (NativeExport → .yx)  │
    │  │ 2. Module dual-path lookup (project → global)      │
    │  │ 3. LSP loads interface files into symbol index     │
    │  └─────────────────────────────────────────────────────┘
    ▼
Issue 1: Use Statement Symbol Location
    │
    │  ┌─────────────────────────────────────────────────────┐
    │  │ 4. update_index_from_ast processes Use statements   │
    │  │ 5. Add use-imported symbols to index                │
    │  └─────────────────────────────────────────────────────┘
    ▼
Issue 2: Same-Name Function Navigation Error  ←─┐
    │                                            │
    │  ┌──────────────────┐                     │
    │  │ 6. Use SemanticDB │                     │
    │  │    for precise    │                     │
    │  │    matching       │                     │
    │  └──────────────────┘                     │
    │                                            │
Issue 3: Local Variables Cannot Be Located  ─────┘
    │
    │  ┌─────────────────────────────────────────────────────┐
    │  │ 7. Expand symbol index scope (traverse function body)│
    │  │ 8. Record scope levels                              │
    │  └─────────────────────────────────────────────────────┘
    ▼
   All Complete
```

---

## Implementation Priority

| Order | Issue | Complexity | Reason | Status |
|-------|-------|------------|--------|--------|
| 1 | Issue 4: Standard Library Interface | Medium | Infrastructure, other issues may depend on it | ✅ Completed |
| 2 | Issue 1: Use Symbol Location | Low | After fix, standard library navigation is available | ✅ Completed |
| 3 | Issue 2: Same-Name Function Navigation | Medium | Improve lookup algorithm | ✅ Completed |
| 4 | Issue 3: Local Variables | High | Need to traverse function body | ✅ Completed |

---

## Implementation Log

### Completion Date: 2026-03-01

### Issue 2 Implementation: SemanticDB Precise Matching

**Modified Files**: `src/lsp/handlers/definition.rs`

**Implementation Content**:
- Refactored `handle_definition` function with two-phase lookup strategy:
  1. **Prioritize SemanticDB**: Use `try_semantic_db_lookup` to find precise definition locations via `symbol_defs`
  2. **Fallback to SymbolIndex**: Use global symbol index when SemanticDB has no results
- Added `try_semantic_db_lookup` function:
  - Uses `SemanticDB.find_innermost_scope()` to find innermost scope at cursor
  - Uses `symbols` information in scope for context filtering
  - Prioritizes same-file definitions to reduce incorrect jumps
- When multiple same-name definitions exist, sort by current file priority

**New Tests**:
- `test_definition_via_semantic_db`: Verify SemanticDB precise matching
- `test_definition_semantic_db_disambiguates`: Verify multi-definition disambiguation

---

### Issue 3 Implementation: Local Variable and Function Parameter Indexing

**Modified Files**: `src/lsp/world.rs`

**Implementation Content**:
- Extended `update_index_from_ast` method to recursively traverse function bodies:
  - Extract function parameters (`Param`) to symbol index
  - Traverse statements and expressions in function body / method body
- Added recursive helper methods:
  - `index_stmt_symbols`: Process nested statements (Var, For, If, nested Fn, etc.)
  - `index_expr_symbols`: Process symbol definitions in expressions (Lambda, FnDef, For, ListComp, etc.)
  - `index_block_symbols`: Traverse block-level code

**New Tests**:
- `test_update_index_fn_params`: Verify function parameters are correctly indexed

---

### Issue 1 Implementation: Use Statement Symbol Location

**Modified Files**: `src/lsp/world.rs`

**Implementation Content**:
- Added `StmtKind::Use` branch handling in `update_index_from_ast`
- Added `index_use_symbols` method:
  - Uses `ModuleRegistry::with_std()` to find module exports
  - Supports `use std.io` (import all exports)
  - Supports `use std.io.{println}` (import specific items)
  - Adds imported symbols to current file's symbol index

**New Tests**:
- `test_update_index_use_stmt`: Verify `use std.io` imports all exports
- `test_update_index_use_stmt_with_items`: Verify specified item imports

---

### Issue 4 Implementation: Standard Library Interface File Generation

**New Files**: `src/std/gen_interfaces.rs`

**Implementation Content**:
- `generate_interface_content`: Automatically generate `.yx` interface file content from `StdModule::exports()`
- `generate_all_interfaces`: Batch generate interfaces for all 10 standard library modules
- `write_interfaces_to_dir`: Write interface files to specified directory
- `default_std_interface_dir`: Get global standard library interface directory (`~/.yaoxiang/std/`)
- `find_std_interface_file`: Dual-path lookup (project-local → global fallback)

**LSP Integration** (`src/lsp/world.rs`):
- Added `load_std_library_symbols` method:
  - Gets standard library exports from `all_module_infos()`
  - If `.yx` interface file exists, parse to get precise Span information
  - Otherwise use virtual path (`std://std.io`, etc.) and `Span::dummy()`
- Added `parse_interface_file_spans` function: Simple text parsing to get function name line number mapping
- Automatically call `load_std_library_symbols` during LSP server startup (`src/lsp/server.rs`)

**Interface File Format**:
```yaoxiang
// io.yx - Standard Library std.io Module Interface
// For LSP navigation and type viewing only, not involved in actual execution

print: (...args) -> () = {
    ...
}

println: (...args) -> () = {
    ...
}
```

**Module Lookup Order**:
```
1. project_dir/.yaoxiang/vendor/std/<name>.yx  ← Project-local (priority)
2. ~/.yaoxiang/std/<name>.yx                   ← Global fallback
```

**New Tests**:
- `test_generate_all_interfaces`: Verify 10 module interfaces generation
- `test_io_interface_content`: Verify io interface content
- `test_math_interface_has_constants`: Verify constant exports
- `test_list_interface_content`: Verify list interface content
- `test_write_interfaces_to_temp_dir`: Verify file writing
- `test_find_std_interface_file`: Verify path lookup
- `test_load_std_library_symbols`: Verify standard library symbol loading
- `test_parse_interface_file_spans`: Verify interface file Span parsing

---

## References

- RFC-004: Multi-location Union Binding Design for Curried Methods (ExternalBindingStmt)
- [Language Server Protocol Specification](https://microsoft.github.io/language-server-protocol/)
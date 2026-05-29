# Block-level Function Definitions and Closure-related Issues

> **Status**: ✅ Completed
>
> **Created**: 2026-02-19
> **Completed**: 2026-02-19

## 1. Problem Summary

### 1.1 Issue 1: Block-level Function Definitions Cannot Find Variables

**Symptoms**:
```yaoxiang
main = {
  add = (a, b) => a + b;      // Function definition without type annotation
  result = add(1, 2);         // ❌ Unknown variable: 'add'
}
```

**Root Cause**: Lines 548-563 in `src/frontend/typecheck/checking/mod.rs`

```rust
// Only add function to scope when there's a type annotation!
if let Some(crate::frontend::core::parser::ast::Type::Fn { ... }) = type_annotation {
    // ... construct function type
    self.add_var(name.to_string(), PolyType::mono(fn_type));  // ❌ Not executed
}
```

When using the minimal form `add = (a, b) => ...`, `type_annotation = None`, and the function name is never added to the scope.

### 1.2 Issue 2: Module-level Functions Work Fine

Module-level functions (like `main = { ... }`) work because they follow a different code path:

```
check_module
  → collect_function_signature (lines 379-382)
    → Adds type variables for functions without type annotations
```

This indicates the type inference infrastructure exists; it's just that `check_fn_stmt` doesn't use it correctly.

### 1.3 Issue 3: use std.{io} Field Access Error

**Symptoms**:
```yaoxiang
use std.{io}
add: (a: Int, b: Int) -> Int = (a, b) => a + b;
main = {
  result = add(1, 2);
  io.println(result);  // ❌ Cannot access field on non-struct type 'fn(t113) -> void'
}
```

**Related but Different Issue**: `io` is recognized as a function type instead of a module.

### 1.4 Four Forms of Function Definitions - Test Results

| Form | Code | Module-level | Inside Block |
|------|------|--------------|--------------|
| Full form | `add: (a: Int, b: Int) -> Int = (a, b) => a + b` | ✅ | ✅ |
| Abbreviated (omit Lambda header) | `add: (a: Int, b: Int) -> Int = { return a + b }` | ✅ | ✅ |
| Abbreviated (omit parameter types) | `add: (a, b) -> Int = (a, b) => { return a + b }` | ✅ | ❌ |
| Minimal form | `add = (a, b) => { return a + b }` | ✅ | ❌ |

---

## 2. Fixes

### 2.1 Issue 1 Fix: Block-level Function Definitions

**Status**: ✅ Fixed

The fix has two parts:

#### 2.1.1 Type Checking Fix

Modified the `check_fn_stmt` function in `src/frontend/typecheck/checking/mod.rs` (lines 546-583):
- Add function to scope regardless of whether there's a type annotation
- If there's a type annotation, use the annotated type
- Otherwise, create type variables from parameters

#### 2.1.2 IR Generation Fix

Modified `src/middle/core/ir_gen.rs`:
1. Added `nested_functions` field to store nested functions (line 152)
2. Modified `generate_local_stmt_ir` to generate IR for nested functions (lines 1013-1032)
3. Modified `generate_module_ir` to add nested functions to module function list (lines 416-417)

**Verification Results**:
- ✅ Compilation phase passes (no more `Unknown variable` error)
- ✅ Runtime executes normally

### 2.3 Issue 3 Fix: use std.{io}

This requires separate investigation; it may be:
1. The `use` statement isn't setting the module type correctly after parsing
2. Field access checking doesn't handle modules properly

---

## 3. Acceptance Criteria

### 3.1 Compilation Acceptance

- [x] `cargo check` passes
- [x] Block-level full-form function calls work (compilation phase)
- [x] Block-level minimal-form function calls work (compilation phase)

### 3.2 Functional Acceptance

- [x] `main = { add = (a,b) => a + b; add(1,2) }` executes normally
- [x] `main = { add: (a:Int,b:Int)->Int = (a,b)=>a+b; add(1,2) }` executes normally

### 3.3 Current Status

| Stage | Module-level Functions | Block-level Functions |
|-------|------------------------|----------------------|
| Lexical/Syntax Analysis | ✅ | ✅ |
| Type Checking | ✅ | ✅ Fixed |
| Code Generation | ✅ | ✅ Fixed |

---

## 4. Pending Issues

### 4.1 use std.{io} Field Access Error

**Status**: ✅ Fixed

**Fix Content**:
Modified the `collect_use_statement` function in `src/frontend/typecheck/mod.rs` (lines 645-678):
- Create a StructType containing exported functions for submodules, instead of the incorrect Fn type
- Get submodule export information from the module registry

**Verification Results**:
```yaoxiang
use std.{io}
main = {
  add = (a, b) => a + b;
  result = add(100, 200);
  io.println(result)  // ✅ Works correctly
}
```

---

## 4. Test Cases

### 4.1 Block-level Function Definition Tests

```yaoxiang
// test_block_fn.yx
main = {
  // Full form
  add1: (a: Int, b: Int) -> Int = (a, b) => a + b;

  // Minimal form (no type annotation)
  add2 = (a, b) => a + b;

  result1 = add1(1, 2);
  result2 = add2(3, 4);
  result1 + result2  // Returns 10
}
```

---

## 5. Related Files

| File | Lines | Description |
|------|-------|-------------|
| `src/frontend/typecheck/checking/mod.rs` | 548-584 | `check_fn_stmt` function |
| `src/frontend/typecheck/mod.rs` | 379-382 | `collect_function_signature` |
| `src/frontend/typecheck/mod.rs` | 546-590 | Module-level function signature collection logic |
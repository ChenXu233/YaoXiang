# YaoXiang Native Function Signature Parsing and Closure Invocation Fix Plan

> **Status**: ✅ Completed
> **Date**: 2026-02-19
> **Completion Date**: 2026-02-19

---

## Overview

### Background

Currently, YaoXiang language has two related issues when using higher-order functions (such as `list.map`, `list.filter`, `list.reduce`):

1. **Signature parsing error**: The signature strings for `map`/`filter`/`reduce` functions in `src/std/list.rs` use an invalid type `Fn`
2. **Misleading error messages**: When signature parsing fails, the error message shows "Invalid signature 'Float': missing '->'", instead of a more reasonable error prompt

### Problematic Code

`src/std/list.rs` lines 72-87:

```rust
NativeExport::new(
    "map",
    "std.list.map",
    "(list: List, fn: Fn) -> List",  // ❌ Fn is an invalid type (should be (T) -> T)
    native_map as NativeHandler,
),
NativeExport::new(
    "filter",
    "std.list.filter",
    "(list: List, fn: Fn) -> List",   // ❌ Fn is an invalid type
    native_filter as NativeHandler,
),
NativeExport::new(
    "reduce",
    "std.list.reduce",
    "(list: List, fn: Fn, init: Any) -> Any",  // ❌ Fn is an invalid type
    native_reduce as NativeHandler,
),
```

**Current signature format** (correct):
```
(list: List, fn: Fn) -> List
  ↑       ↑     ↑
  │       │     └── parameter type (invalid)
  │       └── parameter name
  └── parameter type
```

**Issue**: `fn` is the parameter name, `Fn` is the type name. `Fn` is not a valid primitive type name.

### Runtime Error

When executing test code:

```yaoxiang
main = {
    doubled = list.map([1, 2, 3], x => x * 2);
    io.println(doubled);
}
```

Output (current behavior - error):

```
[Warning] Invalid signature 'Float': missing '->'
[Warning] Invalid signature 'Float': missing '->'
[Warning] Invalid signature 'Float': missing '->'
Error: Runtime error: Type error: Expected function value
```

Expected output after fix:

```
[Error] Invalid signature: unknown type 'Fn'
Error: (compilation failed)
```

---

## Implementation Goals

### Goal 1: Fix Signature Definition

According to RFC-010 unified type syntax, the correct format for generic functions is:

```
function_name: [generic_parameter_list](parameter_list) -> return_type
```

Where the generic parameter `[T]` is declared at the function level and applies to the entire function signature.

Modify the signatures of `map`/`filter`/`reduce` to (generic parameter `[T]` before function name):

```rust
// map: generic [T] scope is the entire function
"[T](list: List<T>, fn: (item: T) -> T) -> List<T>"

// filter: generic [T] scope is the entire function
"[T](list: List<T>, fn: (item: T) -> Bool) -> List<T>"

// reduce: generic [T] scope is the entire function
"[T](list: List<T>, fn: (acc: Any, item: T) -> Any, init: Any) -> Any"
```

**Signature structure explanation**:

```
[T](list: List<T>, fn: (item: T) -> T) -> List<T>
│  │         │      │    │        │
│  │         │      │    │        └── return type (uses T)
│  │         │      │    └── parameter type (uses T)
│  │         │      └── parameter name
│  │         └── parameter type (function type)
│  └── parameter type (List generic, uses T)
└── generic parameter declaration (scope is entire function)
```

### Goal 2: Generic Parameter Scope Rules

**Generic parameter declaration prohibits shadowing** (No Shadowing):

There are multiple scope levels in the signature:

```
[T](list: List[T], fn: [T](item: T) -> T) -> List[T]
│                      │
│                      └── inner function type scope (fn's type parameter)
└── outer function scope (function's generic parameter)
```

**Rules**:

1. **Same-level prohibition of shadowing**: Generic parameters within the same scope cannot have the same name
2. **Inner prohibits shadowing outer**: Generic parameters in inner scope cannot have the same name as outer scope
3. **Function parameters prohibited from shadowing**: Function parameter names cannot have the same name as any generic parameter

**Valid examples**:

```yaoxiang
// ✅ Valid: generic parameter T scope is the entire function
map: [T](list: List[T], fn: (item: T) -> T) -> List[T]

// ✅ Valid: inner function type has no generic parameters
map: [T](list: List[T], fn: (item: T) -> T) -> List[T]

// ✅ Valid: multiple generic parameters
zip: [T, U](a: List[T], b: List[U]) -> List<(T, U)>

// ✅ Valid: function parameter names differ from generic parameters
foo: [T](x: Int, y: T) -> T
```

**Invalid examples**:

```yaoxiang
// ❌ Invalid: inner function generic shadows outer generic (shadowing prohibited)
"[T](list: List[T], fn: [T](item: T) -> T) -> List[T]"
# Error: Generic parameter 'T' in function type shadows outer generic parameter 'T'

// ❌ Invalid: generic parameters with same name (same-level shadowing prohibited)
"[T, T](x: T, y: T) -> T"
# Error: Duplicate generic parameter 'T'

// ❌ Invalid: function parameter name same as generic parameter (shadowing prohibited)
"[T](T: Int) -> T"
# Error: Parameter 'T' shadows generic parameter 'T'
```

### Goal 3: Signature Parameter Name Validation

When parsing signatures, parameter name legitimacy should be validated:
1. **Parameter names cannot be duplicated** (E2093)
2. **Generic parameters prohibited from shadowing** (E2095, E2096)

> **Note**: Cases where parameter names are keywords will automatically report syntax errors when the parser parses the signature, so no separate validation is needed.

Example:
```
// Valid signature (conforms to RFC-010)
"[T](list: List<T>, fn: (item: T) -> T) -> List<T>"

// Invalid signature - function parameter name same as generic parameter (shadowing prohibited)
"[T](list: List<T>, fn: (T: T) -> T) -> List<T>"
# Should report error: Parameter 'T' shadows generic parameter 'T'

// Invalid signature - duplicate parameter name
"[T](x: Int, x: Int) -> Int"
# Should report error: Invalid signature: duplicate parameter name 'x'

// Note: Cases where parameter names are keywords will automatically report syntax errors when the parser parses the signature
```

### Goal 4: Fix Error Messages

When signature parsing encounters errors, the error code system should be used to report errors (E2xxx - semantic analysis phase):

**New error codes to add**:

| Error Code | Message Template | Description |
|------------|------------------|-------------|
| E2090 | Invalid signature: {reason} | Signature parsing failed (general) |
| E2091 | Invalid signature: unknown type '{type_name}' | Unknown type |
| E2092 | Invalid signature: missing '->' | Missing arrow |
| E2093 | Invalid signature: duplicate parameter '{name}' | Duplicate parameter name |
| E2094 | Invalid signature: generic '{name}' shadows outer generic | Generic parameter shadowing |
| E2095 | Invalid signature: parameter '{name}' shadows generic | Parameter name shadows generic |

> **Note**: Cases where parameter names are keywords do not need separate validation, because when the parser parses the signature and encounters a keyword, it will automatically report a syntax error.

**Error message format** (conforms to RFC-013):

```
[Error] E2091: Invalid signature: unknown type 'Fn'
 --> std/list.yx:1:1
  |
1 | "(list: List, fn: Fn) -> List"
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
help: Use a valid type like '(T) -> T' for function parameters
```

---

## Acceptance Criteria

### Acceptance Criterion 1: Compilation Passes

After modifying the signature, the test code should compile successfully, no more "Invalid signature" warnings:

```bash
$ cargo run -- run tests/closure_test2.yx
# Should output:
# [Test map:]
# [2, 4, 6]
# [Test filter:]
# [3, 4, 5]
# [Test reduce:]
# 10
# [All tests passed!]
```

### Acceptance Criterion 2: Correct Error Messages

When using invalid signatures, it should display **Error** (instead of Warning), using the error code system (E2xxx):

```bash
# Test invalid signature
# Expected output:
[Error] E2091: Invalid signature: unknown type 'Fn'
[Error] E2092: Invalid signature: missing '->'
[Error] E2093: Invalid signature: duplicate parameter 'x'
```

### Acceptance Criterion 3: Lambda Parameter Name Matching

The lambda parameter names passed in must match the function parameter names defined in the signature:

```yaoxiang
// Signature definition: fn: (item: T) -> T
// Passed lambda: x => x * 2

// ❌ Wrong - parameter name mismatch
list.map([1, 2, 3], x => x * 2)
# Expected error: Parameter name mismatch: expected 'item', got 'x'

// ✅ Correct - parameter name matches
list.map([1, 2, 3], item => item * 2)

// For reduce, signature is fn: (acc: Any, item: T) -> Any
list.reduce([1, 2, 3], (accumulator, item) => accumulator + item, 0)
# ✅ Correct - parameter name matches
```

### Acceptance Criterion 4: Parameter Name Validation (Signature Parsing)

When parsing signatures, invalid parameter names should be detected:
- Duplicate parameter names should report errors (E2093)
- Generic parameter shadowing should report errors (E2095, E2096)

> **Note**: Cases where parameter names are keywords will automatically report syntax errors when the parser parses the signature, so no separate validation is needed.

```bash
# Expected errors:
[Error] E2093: Invalid signature: duplicate parameter 'x'
[Error] E2095: Invalid signature: generic 'T' shadows outer generic
```

### Acceptance Criterion 5: Higher-Order Functions Work Normally

- `list.map` applies a function to each element of the list, returns a new list
- `list.filter` keeps elements that satisfy the condition, returns a new list
- `list.reduce` performs cumulative calculation on elements

---

## Test Plan

### Test Case 1: Basic map Function

```yaoxiang
main = {
    // Signature definition: fn: (item: T) -> T, parameter name is item
    doubled = list.map([1, 2, 3], item => item * 2);
    io.println(doubled);  // Expected: [2, 4, 6]
}
```

### Test Case 2: Basic filter Function

```yaoxiang
main = {
    // Signature definition: fn: (item: T) -> Bool, parameter name is item
    filtered = list.filter([1, 2, 3, 4, 5], item => item > 2);
    io.println(filtered);  // Expected: [3, 4, 5]
}
```

### Test Case 3: Basic reduce Function

```yaoxiang
main = {
    // Signature definition: fn: (acc: Any, item: T) -> Any, parameter names are acc, item
    sum = list.reduce([1, 2, 3, 4], (acc, item) => acc + item, 0);
    io.println(sum);  // Expected: 10
}
```

### Test Case 4: Lambda Parameter Name Mismatch

```yaoxiang
main = {
    // ❌ Wrong - parameter name mismatch
    // Signature: fn: (item: T) -> T, but passed parameter name is x
    doubled = list.map([1, 2, 3], x => x * 2);
}
# Expected compilation error: Parameter name mismatch: expected 'item', got 'x'
```

### Test Case 5: Complex Lambda

```yaoxiang
main = {
    // Multi-parameter lambda
    result = list.reduce([1, 2, 3], (acc, x) => acc * x, 1);
    io.println(result);  // Expected: 6

    // Nested calls
    data = list.map([1, 2, 3], x => {
        y = x + 1;
        y * 2
    });
    io.println(data);  // Expected: [4, 6, 8]
}
```

### Test Case 6: Invalid Signature Error Message

Validate that error messages are displayed correctly (should be Error and use error codes):

```bash
# Expected output:
[Error] E2091: Invalid signature: unknown type 'Fn'
# instead of:
# [Warning] Invalid signature 'Float': missing '->'
```

### Test Case 7: Duplicate Parameter Name

```rust
// Suppose there is such an invalid signature
// "(x: Int, x: Int) -> Int"
// Expected compilation error:
// [Error] Invalid signature: duplicate parameter name 'x'
```

---

## Technical Details

### Related Code Files

| File | Role | |
|------|------|------|
| `src/std/list.rs` | Native function export definition (✅ Signature modified) | |
| `src/frontend/typecheck/mod.rs` | Signature parsing logic (✅ parse_signature rewritten) | |
| `src/backends/interpreter/executor.rs` | Runtime closure invocation (✅ MakeClosure lookup fixed) | |
| `src/middle/core/bytecode.rs` | Bytecode decoding (✅ MakeClosure decoder added) | |
| `src/std/io.rs` | IO module (✅ List display format fixed) | |
| `src/util/diagnostic/codes/e2xxx.rs` | Error code definition (✅ E2090-E2095 added) | |
| `src/util/diagnostic/codes/i18n/zh.json` | Chinese i18n (✅ added) | |
| `src/util/diagnostic/codes/i18n/en.json` | English i18n (✅ added) | |

### Signature Parsing Flow

1. `TypeCheckResult::new()` calls `register_std_native_signatures()`
2. `register_std_native_signatures()` iterates over std module exports
3. For each `Export`, calls `parse_signature(&export.signature, env)`
4. `parse_signature` parses signature string into `MonoType::Fn`
5. On parsing failure, uses error code system to report errors (E2090-E2095)

### Actual Code Changes

1. **`src/std/list.rs:71-88`**: Modified signature strings for three functions (RFC-010 generic function syntax)
   ```rust
   "[T](list: List<T>, fn: (item: T) -> T) -> List<T>"
   "[T](list: List<T>, fn: (item: T) -> Bool) -> List<T>"
   "[T](list: List<T>, fn: (acc: Any, item: T) -> Any, init: Any) -> Any"
   ```

2. **`src/frontend/typecheck/mod.rs`** (parse_signature rewrite):
   - Support `[T]` generic parameter prefix parsing
   - Support function type parameters `(item: T) -> T`
   - Correct bracket matching handling (find_matching_close)
   - Parameter name duplicate checking (E2093)
   - Generic parameter shadowing checking (E2094, E2095)
   - Constant type signature handling (such as `"Float"`)
   - Error messages upgraded from Warning to Error + error codes

3. **`src/middle/core/bytecode.rs`** (bytecode decoder):
   - Added `Opcode::MakeClosure` decoder (previously swallowed by catch-all branch becoming Nop)

4. **`src/backends/interpreter/executor.rs`** (closure execution):
   - Fixed `MakeClosure` handler: `FunctionRef::Index` uses index directly instead of constructing name

5. **`src/std/io.rs`** (IO module):
   - print/println support readable formatted output for lists/dicts (via heap parsing)

6. **Error code definitions**:
   - `src/util/diagnostic/codes/e2xxx.rs`: Added E2090-E2095 definitions and shortcut methods
   - `src/util/diagnostic/codes/i18n/zh.json`: Added Chinese error messages
   - `src/util/diagnostic/codes/i18n/en.json`: Added English error messages

---

## Dependencies

This task does not depend on other tasks and has been completed independently.

---

## Risks and Notes

1. **Generics support**: ✅ The type system supports parsing of generic `List<T>`, generic parameters are parsed as TypeRef
2. **Closure environment capture**: Current implementation does not handle closure capture of external variables; map/filter/reduce use cases do not require this feature
3. **Additional issues discovered and fixed**:
   - MakeClosure bytecode decoder missing (caused closure to become Nop)
   - MakeClosure executor's incorrect handling of FunctionRef::Index (constructed "fn_N" name instead of directly using index)
   - io.println unable to format and display list contents (only showed handle address)
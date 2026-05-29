# Implementation Plan for Function Parameter `mut` Syntax Support

> **Status**: Implemented
> **Date**: 2026-02-19

---

## Overview

### Problem Background

Currently, YaoXiang language function parameters do not support the `mut` keyword, and parameters are immutable by default. This causes the following issues:

1. **Closure parameters are immutable**: For example, in `list.map([1,2,3], x => x * 2)`, the closure parameter `x` is immutable, and cannot be modified within the closure body.
2. **Function parameters are immutable**: Regular function parameters cannot be modified, making it impossible to implement the "in-place modification" pattern.

### Goals

Implement `mut` syntax support for function parameters, allowing function parameters to be declared as mutable.

### Syntax Design

```yaoxiang
// Regular function parameters
fn foo(mut x: Int) -> Int {
    x = x + 1  // Valid, can be modified
    x
}

// Lambda parameters
f = (mut x) => x + 1

// Higher-order function calls
list.map([1, 2, 3], (mut x) => x * 2)  // Valid
```

---

## Error Code Design

### E2010 vs E2011 Distinction

| Error Code | Description | Scenario |
|------------|-------------|----------|
| E2010 | cannot assign to immutable variable | `x = 1; x = 2` (user explicitly modifies variable) |
| **E2011** | **closure parameter requires mut** | `list.map([..., x => ...])` (closure parameter needs mut) |

### E2011 Design

**Error Message**:
```
error[E2011]: closure parameter '{param_name}' requires 'mut' to be modified
 --> example.yx:1:20
  |
1 | list.map([1,2,3], x => x * 2);
  |                    ^ consider adding 'mut' to parameter: (mut x) => ...
```

**Trigger Conditions**:
1. Closure is passed as a parameter to a higher-order function
2. Higher-order function attempts to modify closure parameter internally
3. Closure parameter is not declared as `mut`

**Fix Suggestion**:
- Suggest changing `x => ...` to `(mut x) => ...`

---

## Acceptance Criteria

### Parser Layer

- [x] Parser supports parsing parameters in the form `(mut x: Type)`
- [x] Parser supports parsing parameters in the form `(mut x)` (type omitted)
- [x] Parser supports Lambda syntax `(mut x) => body`

### AST Layer

- [x] `Param` struct adds `is_mut: bool` field
- [x] Type checking correctly identifies mutable parameters

### Semantic Analysis Layer

- [x] Type checker correctly handles mutable parameters
- [x] Mutable parameters can be modified within function body

### IR Generation Layer

- [x] IR generator correctly handles mutable parameters (registered as mutable local variables)
- [x] Closure mutable parameters are correctly passed to closure functions

### Testing Requirements

- [x] Test case: regular functions with `mut` parameters
- [x] Test case: Lambda with `mut` parameters
- [x] Test case: `mut` parameter closures in higher-order functions
- [x] Test case: error scenario - immutable parameters being modified

---

## Implementation Steps

### Phase 1: AST Modification

#### 1.1 Modify Param Struct

**File**: `src/frontend/core/parser/ast.rs`

```rust
pub struct Param {
    pub name: String,
    pub ty: Option<Type>,
    pub is_mut: bool,  // New
    pub span: Span,
}
```

**Verification**:
- [x] Param in AST contains is_mut field

---

### Phase 2: Parser Modification

#### 2.1 Modify Parameter Parsing Logic

**File**: `src/frontend/core/parser/statements/declarations.rs`

**Function**: `parse_fn_params`

Detect `mut` keyword before parsing parameter name:

```rust
// Detect mut keyword
let is_mut = state.skip(&TokenKind::KwMut);

let name = match state.current().map(|t| &t.kind) {
    Some(TokenKind::Identifier(n)) => n.clone(),
    _ => break,
};
state.bump();

// Parse type annotation
let ty = if state.skip(&TokenKind::Colon) {
    parse_type_annotation(state)
} else {
    None
};

params.push(Param {
    name,
    ty,
    is_mut,  // New
    span: param_span,
});
```

**Verification**:
- [x] `(mut x: Int)` parses correctly
- [x] `(mut x)` parses correctly (no type annotation)
- [x] `(x: Int)` parses as immutable (is_mut = false)

---

### Phase 3: Type Checking

#### 3.1 Type Checker Passes is_mut Information

**File**: `src/frontend/typecheck/`

Need to pass parameter mutability information during the type checking phase.

**Verification**:
- [x] Type checking passes function definitions with mutable parameters
- [x] Type checking rejects code that modifies immutable parameters

---

### Phase 4: IR Generation

#### 4.1 Modify generate_function_ir

**File**: `src/middle/core/ir_gen.rs`

Modify parameter registration logic based on `is_mut` to determine whether to register as mutable:

```rust
for (i, param) in params.iter().enumerate() {
    // Register parameter
    self.register_local(&param.name, i);
    // Only mut parameters are registered as mutable
    if param.is_mut {
        self.current_mut_locals.insert(i);
    }
}
```

#### 4.2 Modify generate_lambda_body_ir

**File**: `src/middle/core/ir_gen.rs`

Similarly modify closure parameter handling:

```rust
for (i, param) in params.iter().enumerate() {
    self.register_local(&param.name, i);
    // Only mut parameters are registered as mutable
    if param.is_mut {
        self.current_mut_locals.insert(i);
    }
}
```

**Verification**:
- [x] Mutable parameters are correctly marked as mutable in IR
- [x] Closure mutable parameters are correctly passed

---

## Files Involved

| Module | File | Modifications |
|--------|------|---------------|
| AST | `src/frontend/core/parser/ast.rs` | Param struct adds is_mut field |
| Parser | `src/frontend/core/parser/statements/declarations.rs` | Parameter parsing supports mut keyword, function type annotations recognize mut parameters |
| Parser | `src/frontend/core/parser/statements/bindings.rs` | Binding parameter parsing supports mut keyword |
| Parser | `src/frontend/core/parser/pratt/nud.rs` | Typed parameter list supports mut prefix |
| Parser | `src/frontend/core/parser/pratt/led.rs` | Lambda parameter conversion supports Expr::Lambda |
| Parser | `src/frontend/core/parser/pratt/mod.rs` | Lambda parameter adds is_mut: false default value |
| IR Generation | `src/middle/core/ir_gen.rs` | Register mutable local variables based on is_mut, fix lambda state isolation |
| Tests | `src/frontend/typecheck/tests/*.rs` | Update Param construction to add is_mut field |
| Tests | `tests/mut_param_test.yx` | mut parameter passing scenario test |
| Tests | `tests/mut_param_error_test.yx` | Immutable parameter modification error test |

---

## Test Cases

### Passing Scenarios

```yaoxiang
// 1. Mutable parameters in regular functions
fn increment(mut x: Int) -> Int {
    x = x + 1
}
main = {
    result = increment(5);
}

// 2. Mutable parameters in Lambda
main = {
    f = (mut x) => {
        x = x + 1
        x
    };
    result = f(5);
}

// 3. Mutable parameter closures in higher-order functions
use std.{io, list}
main = {
    result = list.map([1, 2, 3], (mut x) => {
        x = x * 2
        x
    });
}
```

### Error Scenarios

#### E2010 - Regular Immutable Variable Modification

```yaoxiang
// Immutable parameter modified - E2010
fn foo(x: Int) -> Int {
    x = x + 1  // E2010: cannot assign to immutable variable
}
```

#### E2011 - Closure Parameter Requires mut (New)

```yaoxiang
// Closure parameter not declared as mut - E2011
list.map([1, 2, 3], x => x * 2)
// E2011: closure parameter 'x' requires 'mut' to be modified
// help: consider adding 'mut' to parameter: (mut x) => ...

list.filter([1, 2, 3], x => x > 2)
// E2011: closure parameter 'x' requires 'mut' to be modified

list.reduce([1, 2, 3], (acc, x) => acc + x, 0)
// E2011: closure parameter 'acc' requires 'mut' to be modified
// E2011: closure parameter 'x' requires 'mut' to be modified
```

---

## Risks and Notes

1. **Backward Compatibility**: Existing code without `mut` parameters maintains immutable behavior.
2. **Type Inference**: When type annotation is omitted, `(mut x)` should be able to infer the type automatically.
3. **Closure Scenarios**: Ensure mutable parameters are correctly passed to closure function bodies.

---

## Additional Issues Found and Fixed During Implementation

### 1. Lambda IR Generation State Isolation

**Problem**: `generate_lambda_body_ir` clears `current_mut_locals` and `current_local_names` when generating closure function bodies, causing the parent function's mutability information to be lost.

**Fix**: Save parent function state (`current_mut_locals`, `current_local_names`, `next_temp`) before entering lambda body IR generation, and restore after exiting.

### 2. Lambda Return Value Register Conflict

**Problem**: `generate_lambda_body_ir` uses fixed register 0 as the return value register, which conflicts with parameter register 0, causing MutChecker to report false positives.

**Fix**: Changed to use `self.next_temp_reg()` to allocate an independent return value register.

### 3. List Literal StoreIndex Mutability

**Problem**: List literals `[1, 2, 3]` write elements through multiple `StoreIndex` operations, and the second and subsequent writes are treated by MutChecker as modifications to immutable variables.

**Fix**: After `AllocArray`, register the list's temporary register as mutable (`current_mut_locals.insert(result_reg)`).

### 4. Function Type Annotation `mut` Parameter Recognition

**Problem**: When parsing RFC-010 function type annotations `(mut x: Int) -> Int`, the `looks_like_named_params` check cannot recognize parameter lists starting with `KwMut`, causing them to be misidentified as old syntax.

**Fix**: Add `state.at(&TokenKind::KwMut)` check in both `looks_like_named_params` locations.
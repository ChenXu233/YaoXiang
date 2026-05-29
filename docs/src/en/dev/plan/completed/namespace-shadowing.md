# Variable Naming Space and Shadowing Mechanism Code Review Report

## Review Date
2026-02-18

## Fix Date
2026-02-18

## Review Scope
- Variable naming space management (scope)
- Variable shadowing detection
- For loop variable semantics
- `mut` declaration handling

---

## Requirements Review

According to the design, the language's variable shadowing rules are as follows:

1. **Prohibition of Shadowing** - Any attempt to redeclare a variable name that already exists in an outer scope will result in an error
2. **No let Keyword** - Variables are created via `mut` or implicit declaration
3. **Shadowing is an Error** - Whether it's a regular declaration or a `mut` declaration, shadowing results in an error
4. **Local Variables Destroyed on Scope Exit** - Variables are destroyed when the local scope ends
5. **For Loop Semantics** - The `i` in `for i in iter` is a rebinding, creating a new local binding each iteration

---

## Score: 🟢 Fixed

## Scope Implementation Overview

### Type Inference Phase (ExprInferrer)

| Scope Type | Implementation Status | Code Location |
|-----------|----------------------|---------------|
| For Loop | ✅ Implemented | expressions.rs (enter_scope/exit_scope + try_add_var) |
| Function Definition | ✅ Implemented | expressions.rs (enter_scope/exit_scope) |
| Lambda Expression | ✅ Implemented | expressions.rs (enter_scope/exit_scope) |
| List Comprehension | ✅ Implemented | expressions.rs (enter_scope/exit_scope) |
| If Statement Branch | ✅ **Fixed** | expressions.rs (each branch enter_scope/exit_scope) |
| While Loop Body | ✅ **Fixed** | expressions.rs (enter_scope/exit_scope) |

### Type Checking Phase (BodyChecker)

| Scope Type | Implementation Status | Description |
|-----------|----------------------|-------------|
| Scope Stack | ✅ **Fixed** | `scopes: Vec<HashMap<String, PolyType>>` |
| For Loop Scope | ✅ **Fixed** | `check_for_stmt` uses enter/exit_scope |
| Function Parameter Scope | ✅ **Fixed** | `check_fn_def` uses enter/exit_scope |
| If Statement Scope | ✅ **Fixed** | `check_block` automatically creates scope |
| Regular Block Scope | ✅ **Fixed** | `check_block` automatically creates scope |
| mut Shadowing Check | ✅ **Fixed** | `check_var_stmt` checks var_exists_in_any_scope |
| For Loop Shadowing Check | ✅ Implemented | `check_for_stmt` checks var_exists_in_any_scope |
| Assignment Expression Shadowing Check | ✅ **Fixed** | `check_expr_stmt` distinguishes current/outer scope |

---

## Fixed Issues

### Fix 1: BodyChecker Scope Stack (Original Issue 2)

**Modified File**: `src/frontend/typecheck/checking/mod.rs`

**Modification**: Replaced `vars: HashMap<String, PolyType>` with `scopes: Vec<HashMap<String, PolyType>>`

New methods:
- `enter_scope()` - Enter a new scope
- `exit_scope()` - Exit the current scope
- `var_exists_in_any_scope(name)` - Check if a variable exists in any scope
- `var_exists_in_current_scope(name)` - Check if a variable exists in the current scope
- `update_var(name, poly)` - Update a variable in existing scopes

`add_var` now adds to the current scope (`scopes.last_mut()`), and `get_var` now searches from inner to outer scopes.

---

### Fix 2: mut Declaration Shadowing Check (Original Issue 1)

**Modified File**: `src/frontend/typecheck/checking/mod.rs` - `check_var_stmt`

**Before**:
```rust
// If variable already exists, unify types  ← Wrong! Allows shadowing
if let Some(existing_poly) = self.vars.get(name) {
    let _ = self.solver.unify(&existing_poly.body, &ty);
}
self.vars.insert(name.to_string(), PolyType::mono(ty));
```

**After**:
```rust
// Shadowing check: if variable already exists in any scope, error
if self.var_exists_in_any_scope(name) {
    return Err(Box::new(
        ErrorCodeDefinition::variable_shadowing(name).build(),
    ));
}
self.add_var(name.to_string(), PolyType::mono(ty));
```

---

### Fix 3: For Loop Scope (Original Issue 3)

**Modified File**: `src/frontend/typecheck/checking/mod.rs` - `check_for_stmt`

**Modification**: Added `enter_scope()` / `exit_scope()` before and after loop variable checking and loop body checking to ensure the loop variable is destroyed after the loop ends.

---

### Fix 4: Assignment Expression Shadowing Check (New)

**Modified File**: `src/frontend/typecheck/checking/mod.rs` - `check_expr_stmt`

**Modification**: The handling of `BinOp::Assign` now distinguishes three cases:
1. **Exists in current scope** → Assignment operation (unify types)
2. **Exists only in outer scope** → Shadowing error
3. **Does not exist** → Create new variable

---

### Fix 5: Function Definition Scope (New)

**Modified File**: `src/frontend/typecheck/checking/mod.rs` - `check_fn_def`

**Modification**: Function definition checking now creates an independent function scope; function parameters and local variables are destroyed after the function ends.

Additionally, the duplicate parameter addition logic was removed from `check_fn_stmt`, with parameters now managed uniformly by `check_fn_def` within the function scope.

---

### Fix 6: If/Code Block Scope (New)

**Modified File**: `src/frontend/typecheck/checking/mod.rs` - `check_block`

**Modification**: `check_block` now automatically creates and exits scopes, used for If statement's then/elif/else branches.

---

### Fix 7: ExprInferrer If/While Scope (New)

**Modified File**: `src/frontend/typecheck/inference/expressions.rs`

**Modification**:
- If expression: Each branch (then/elif/else) uses an independent scope
- While expression: Loop body uses an independent scope
- For loop: Fixed a bug where `try_add_var` failed to exit the scope on failure

---

## Test Coverage

New test file: `src/frontend/typecheck/tests/shadowing.rs` (14 tests)

| Test Name | Test Content |
|-----------|-------------|
| `test_body_checker_scope_basic` | Basic scope enter/exit |
| `test_body_checker_nested_scopes` | Nested scope variable visibility and destruction |
| `test_body_checker_get_var_finds_innermost` | get_var prioritizes returning inner variables |
| `test_body_checker_vars_returns_all` | vars() returns all scope variables |
| `test_mut_shadowing_error` | mut duplicate declaration reports shadowing error |
| `test_for_loop_shadowing_error` | for loop using existing variable name reports error |
| `test_for_loop_variable_scoped` | for loop variable destroyed after loop ends |
| `test_for_loop_no_conflict_with_unique_var` | Shadowing-free for loop works normally |
| `test_if_block_creates_scope` | Variables inside if block don't leak outside |
| `test_assignment_shadowing_in_block` | Assigning to outer variable inside if block reports shadowing error |
| `test_assignment_in_same_scope_ok` | Repeated assignment in same scope is normal |
| `test_inferrer_try_add_var_shadowing` | ExprInferrer shadowing detection |
| `test_inferrer_scope_destroyed_on_exit` | ExprInferrer variable destruction after scope exit |
| `test_fn_def_creates_scope` | Function parameters destroyed after function ends |

---

## Summary

### Inference Phase (ExprInferrer)

| Feature | Status |
|---------|--------|
| For Loop Scope | ✅ Implemented |
| Function Definition Scope | ✅ Implemented |
| Lambda Scope | ✅ Implemented |
| List Comprehension Scope | ✅ Implemented |
| If Statement Scope | ✅ **Fixed** |
| While Loop Body Scope | ✅ **Fixed** |

### Checking Phase (BodyChecker)

| Feature | Status |
|---------|--------|
| Scope Stack | ✅ **Fixed** |
| For Loop Scope | ✅ **Fixed** |
| Function Parameter Scope | ✅ **Fixed** |
| If Statement Scope | ✅ **Fixed** |
| Regular Block Scope | ✅ **Fixed** |
| mut Shadowing Check | ✅ **Fixed** |
| For Loop Shadowing Check | ✅ Implemented |
| Assignment Expression Shadowing Check | ✅ **Fixed** |

---

## Legacy Notes

### For Loop "Rebinding" Semantics

The semantics of `i` in For loops (creating new bindings each iteration) are handled at the IR generation and interpreter level. The type checking phase only needs to ensure variables are within scope. The current implementation is acceptable.
# For Loop and Shadowing Check Implementation Document

## Overview

This document describes the design and implementation of for loop variable mutability and shadowing checks in YaoXiang language.

**Background Problem**:
- The current `for i in 1..5` syntax reports an error at the IR stage because the loop variable is not marked as mutable
- The language needs to implement rules that forbid shadowing
- Loop variables need explicit mutability declaration inside the loop, consistent with the `let mut` syntax rules, but since the language has no `let` syntax, shadowing must be prohibited

## Design Principles

1. **No Shadowing**: Variables newly declared in any namespace (for, if, {} code blocks, etc.) cannot shadow variables that already exist in outer scopes
2. **Explicit Mutability**: Mutability must be explicitly declared via the `mut` keyword; loop variables are immutable by default
3. **New Binding Per Iteration**: The for loop variable creates a new binding on each iteration, rather than modifying the same variable

## Implementation Content

### 1. Shadowing Prohibition Check

#### 1.1 Type Checking Phase

Detect shadowing at all locations where new variables are created:

- **for loops**: Check if the loop variable name is already declared in the current or outer scope
- **let declarations**: Check if the variable name is already declared in the current scope
- **if/while and other block statements**: Check if internal declarations shadow external ones

#### 1.2 Implementation Location

Modify the following file:
- `src/frontend/typecheck/checking/mod.rs` - Add shadowing detection logic

#### 1.3 Error Code

New error code `E2xxx` (to be determined):
```
[E2xxx] Variable Shadowing Error
error: cannot shadow existing variable 'x'
 --> example.yx:3:5
  |
3 |     for x in 1..5 {
  |     ^ variable 'x' is already declared in outer scope
```

### 2. For Loop Variable Mutability

#### 2.1 Syntax Design

```yaoxiang
for i in 1..5 {      // i is immutable (default)
    print(i)         // OK
    i = i + 1        // Error: cannot assign to immutable variable
}

for mut i in 1..5 {  // i is mutable
    i = i + 1        // OK
}
```

#### 2.2 Syntax Analysis

Modify `src/frontend/core/parser/statements/control_flow.rs`:
- When parsing for statements, check if `mut` follows the `for` keyword
- If `mut` is present, record it in the AST node

AST structure change:
```rust
StmtKind::For {
    var,           // Variable name
    var_mut: bool, // New: whether the variable is mutable
    iterable,
    body,
    label,
}
```

#### 2.3 IR Generation

Modify `generate_for_loop_ir` in `src/middle/core/ir_gen.rs`:
- If `var_mut` is true, add the loop variable to `current_mut_locals`
- This allows the MutChecker to permit repeated assignments to that variable

```rust
// In generate_for_loop_ir
if var_mut {
    self.current_mut_locals.insert(var_reg);
}
```

## Implementation Effects (Examples)

### Example 1: Basic For Loop

```yaoxiang
// Input
for i in 1..5 {
    print(i)
}

// Output
1
2
3
4
```

### Example 2: For Loop Variable Modification

```yaoxiang
// Input
for mut i in 1..3 {
    i = i + 10
    print(i)
}

// Output
11
12
13
```

### Example 3: Shadowing Prohibition - For Loop

```yaoxiang
// Input
i = 10
for i in 1..5 {
    print(i)
}

// Error Output
error [E2xxx] cannot shadow existing variable 'i'
 --> example.yx:2:5
  |
2 |     for i in 1..5 {
  |         ^ variable 'i' is already declared in outer scope
help: consider renaming the inner variable or outer variable to avoid shadowing
```

### Example 4: {} Code Block Shadowing Prohibition

```yaoxiang
// Input
x = 1
{
    x = 2  // Error!
    print(x)
}

// Error Output
error [E2xxx] cannot shadow existing variable 'x'
 --> example.yx:3:1
  |
3 |     x = 2
  |     ^ variable 'x' is already declared in outer scope
help: consider renaming the inner variable or outer variable to avoid shadowing
```

### Example 5: Shadowing in If Block

```yaoxiang
// Input
x = 1
if true {
    x = 2  // Error!
    print(x)
}

// Error Output
error [E2xxx] cannot shadow existing variable 'x'
 --> example.yx:4:5
  |
4 |         x = 2
  |         ^ variable 'x' is already declared in outer scope
help: consider renaming the inner variable or outer variable to avoid shadowing
```

### Example 6: Modifying Immutable Variable in Loop Body

```yaoxiang
// Input
for i in 1..5 {
    i = i + 1
}

// Error Output
error [E2010] Cannot assign to immutable variable 'i'
 --> example.yx:2:5
  |
2 |     i = i + 1
  |     ^ cannot assign to immutable variable 'i'
help: Use 'mut' to declare a mutable variable
```

## 2. For Loop Variable Creates New Binding Per Iteration

```yaoxiang
// Input
for i in 1..3 {
    print(i)
}

// Here, i inside the loop is a new binding on each iteration because i declared inside the loop body is destroyed after each iteration ends, and a new i binding is created on the next iteration rather than modifying the same variable.
```

## Acceptance Criteria

### Feature Acceptance

1. **For Loop Basic Functionality**
   - [ ] `for i in 1..5 { print(i) }` correctly outputs 1-4
   - [ ] `for mut i in 1..5 { i = i + 1; print(i) }` correctly outputs 2-5

2. **Shadowing Prohibition**
   - [ ] Using a same-named variable in a for loop when it exists externally reports an error
   - [ ] Declaring a same-named variable when it exists externally reports an error
   - [ ] Declaring variables inside if/while blocks that shadow external variables reports an error
   - [ ] Cross-function scope shadowing detection

3. **Mutability Check**
   - [ ] For loop variables are immutable by default, modification reports an error
   - [ ] for mut variables can be modified, work normally

### Error Message Acceptance

- [ ] Shadowing error messages are clear, pointing out the shadowed variable and its location
- [ ] Mutability error messages are consistent with the existing E2010 style

## Test Plan

### Unit Tests

Add tests in `src/frontend/typecheck/tests/`:

```rust
#[test]
fn test_for_loop_basic() {
    // Test basic for loop
}

#[test]
fn test_for_loop_mut() {
    // Test for mut
}

#[test]
fn test_shadowing_for_loop() {
    // Test for loop shadowing detection
}

#[test]
fn test_shadowing_block() {
    // Test shadowing detection
}

#[test]
fn test_shadowing_if_block() {
    // Test if block shadowing detection
}
```

### Integration Tests

Add test cases in `docs/src/tutorial/examples/`:

1. `test_for_loop.yx` - for loop basic test
2. `test_shadowing.yx` - shadowing detection test

### Manual Testing

```bash
# Test basic functionality
cargo run -- run docs/src/tutorial/examples/std_io_examples.yx

# Test shadowing error
echo 'i = 10; for i in 1..5 { print(i) }' | cargo run -- run -
```

### Regression Testing

Ensure existing tests don't fail due to this change:
```bash
cargo test
```
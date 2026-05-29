# Constant Call Issue Fix

## Overview

Fix the issue where constants like `std.math.PI` display as `unit` when used.

## Current Status

- **Problem**: Using `PI` returns `unit` instead of the expected floating-point value
- **Cause**: Constants are treated as zero-argument function calls, but the FFI handler is not being executed correctly

## Problem Analysis

In the current code:

```rust
// FFI Registration
registry.register("std.math.PI", |_args| {
    Ok(RuntimeValue::Float(std::f64::consts::PI))
});
```

However, constant calls (like `PI`) may be compiled to different instructions rather than function calls.

## Modules to Modify

### 1. Compiler - Code Generation

File: `src/middle/passes/codegen/`

Need to correctly identify constant references (like `PI`) as native function calls and generate the corresponding bytecode instructions.

### 2. Interpreter/Executor

File: `src/backends/interpreter/executor.rs`

Ensure constant references can correctly route to the FFI handler.

## Implementation Plan

### Plan A: Register Constant Names in the Translator

```rust
// src/middle/passes/codegen/translator.rs
// Add constants to native_functions
native_functions.insert("std.math.PI".to_string());
native_functions.insert("std.math.E".to_string());
native_functions.insert("std.math.TAU".to_string());
```

### Plan B: Use Special Prefix in FFI

Use a convention like `__const__std.math.PI` to distinguish constants from functions.

## Test Cases

```yaoxiang
use std.math.*

// Expected output 3.14159...
println(PI)

// Expected output 2.71828...
println(E)
```

## Related Files

- `src/middle/passes/codegen/translator.rs` - Code generation
- `src/backends/interpreter/executor.rs` - Interpreter
- `src/backends/interpreter/ffi.rs` - FFI registration
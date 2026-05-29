# Namespace Call Support

## Overview

Implement `std.module.function` style namespace call syntax, enabling code to call module functions like `std.io.print` or `std.math.abs`.

## Current Status

- **Issue**: `use std.io.*` can import short names, but calls in the form `std.io.print` result in "Unknown variable: 'std'" error
- **Expected Behavior**: Users should be able to call functions using the `std.<module>.<function>` form

## Modules to Modify

### 1. Compiler Frontend - Parser

File: `src/frontend/parser/`

Need to support recognizing name expressions in the form `a.b.c` and correctly parsing namespace paths.

### 2. Compiler Frontend - Type Checker

File: `src/frontend/typecheck/`

When encountering namespace paths, need to:

1. Recognize `std` as a builtin namespace
2. Resolve subsequent module names (such as `io`, `math`, `net`)
3. Look up functions within modules and verify types

### 3. IR Generation

File: `src/middle/passes/codegen/`

When generating IR, need to convert namespace paths to target function references.

### 4. Interpreter/Runtime

File: `src/backends/interpreter/executor.rs`

Ensure that namespace paths are correctly resolved to FFI handlers during execution.

## Implementation Steps

1. **Parser Modifications**: Recognize member access expressions in the form `a.b`
2. **Semantic Analysis**: Implement namespace resolution logic
3. **Code Generation**: Generate correct function call instructions
4. **Testing**: Add test cases to verify calls like `std.io.print`

## Test Cases

```yaoxiang
use std.io

// Should work
std.io.println("Hello")

// Short names should also work
use std.io.*
println("World")
```

## Related Files

- `src/frontend/parser/` - Parser
- `src/frontend/typecheck/` - Type Checker
- `src/middle/passes/codegen/` - Code Generation
- `src/backends/interpreter/` - Interpreter
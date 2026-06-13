# Error Code Reference

The YaoXiang compiler uses error codes to identify different types of diagnostic information. Error codes are grouped by number range, with each error code corresponding to a specific error scenario.

---

## E0xxx -- Lexical and Syntax Analysis

Errors produced during the lexical analyzer (Lexer) and parser (Parser) stages.

| Error Code | Template | Description |
|------------|----------|-------------|
| E0001 | `Invalid character: '{char}'` | Invalid character |
| E0002 | `Invalid number literal: '{literal}'` | Invalid numeric literal |
| E0003 | `Unterminated string starting at line {line}` | Unterminated string |
| E0004 | `Invalid character literal: '{literal}'` | Invalid character literal |
| E0010 | `Expected {expected}, found {found}` | Expected token |
| E0011 | `Unexpected token: '{token}'` | Unexpected token |
| E0012 | `Invalid syntax: {reason}` | Invalid syntax |
| E0013 | `Mismatched {bracket_type}: opened at line {open_line}, column {open_col}, not closed` | Mismatched brackets |
| E0014 | `Missing semicolon after {statement}` | Missing semicolon |

## E1xxx -- Type Checking

Errors produced during type checking, covering variable types, function calls, pattern matching, generic instantiation, concurrency semantics, and error propagation.

| Error Code | Template | Description |
|------------|----------|-------------|
| E1001 | `Unknown variable: '{name}'` | Unknown variable |
| E1002 | `Expected type '{expected}', found type '{found}'` | Type mismatch |
| E1003 | `Unknown type: '{type}'` | Unknown type |
| E1010 | `Function '{func}' expects {expected} arguments, found {found}` | Argument count mismatch |
| E1011 | `Parameter type mismatch: expected '{expected}', found '{found}'` | Parameter type mismatch |
| E1012 | `Return type mismatch: expected '{expected}', found '{found}'` | Return type mismatch |
| E1013 | `Function not found: '{func}'` | Function not found |
| E1020 | `Cannot infer type for '{expr}'` | Cannot infer type |
| E1021 | `Type inference conflict: {reason}` | Type inference conflict |
| E1030 | `Pattern non-exhaustive: missing patterns {patterns}` | Incomplete pattern |
| E1031 | `Unreachable pattern: '{pattern}'` | Unreachable pattern |
| E1040 | `Operation '{op}' is not supported for type '{type}'` | Unsupported operation |
| E1041 | `Index out of bounds: valid range is 0..{max}, found {index}` | Index out of bounds |
| E1042 | `Field '{field}' not found in struct '{struct}'` | Field not found |
| E1050 | `Logical operation requires boolean operands, found '{left}' and '{right}'` | Boolean operands required |
| E1051 | `Logical NOT requires boolean operand, found '{type}'` | Logical NOT requires boolean operand |
| E1052 | `Cannot dereference type '{type}', expected pointer type` | Invalid dereference |
| E1053 | `Cannot access field on non-struct type '{type}'` | Non-struct field access |
| E1054 | `Condition must be boolean, found '{type}'` | Condition type mismatch |
| E1055 | `Constraint type '{type}' can only be used in generic context` | Constraint in non-generic context |
| E1060 | `Expected {expected} type argument(s), found {found}` | Type argument count mismatch |
| E1061 | `Cannot instantiate generic type with given arguments` | Cannot instantiate generics |
| E1070 | `Unknown label: '{label}'` | Unknown label |
| E1081 | `` `?` is only allowed inside functions returning Result `` | `?` only allowed in functions returning Result |
| E1082 | `` `?` requires a Result expression, found '{type}' `` | `?` can only be used with Result expressions |
| E1083 | `` Result error type mismatch for `?`: expected '{expected}', found '{found}' `` | `?` error type mismatch |
| E1090 | `Type: Type = Type` | Unspeakable (Easter egg) |
| E1091 | `Generic meta-type self-reference is not allowed: '{decl}'` | Invalid generic meta-type |

## E2xxx -- Semantic Analysis

Errors produced during semantic analysis, covering scope, variable lifetime, ownership, and function signature resolution.

| Error Code | Template | Description |
|------------|----------|-------------|
| E2001 | `Variable '{name}' is not in scope` | Scope error |
| E2002 | `Duplicate definition: '{name}' is already defined in this scope` | Duplicate definition |
| E2003 | `Ownership constraint violated: {reason}` | Ownership error |
| E2010 | `Cannot assign to immutable variable '{name}'` | Immutable assignment |
| E2011 | `Use of uninitialized variable '{name}'` | Use of uninitialized variable |
| E2012 | `Mutability conflict: cannot use mutable reference in immutable context` | Mutability conflict |
| E2013 | `Cannot shadow existing variable '{name}'` | Variable shadowing |
| E2014 | `'{name}' has been moved and cannot be used` | Use of moved variable |
| E2090 | `Invalid signature: {reason}` | Invalid signature |
| E2091 | `Invalid signature: unknown type '{type_name}'` | Unknown type in signature |
| E2092 | `Invalid signature: missing '->'` | Missing arrow in signature |
| E2093 | `Invalid signature: duplicate parameter '{name}'` | Duplicate parameter name |
| E2094 | `Invalid signature: generic '{name}' shadows outer generic` | Generic parameter shadowing |
| E2095 | `Invalid signature: parameter '{name}' shadows generic` | Parameter name shadows generic |

## E4xxx -- Generics and Traits

Errors related to generic constraints and trait system.

| Error Code | Template | Description |
|------------|----------|-------------|
| E4001 | `Type '{type}' does not satisfy the trait bound '{trait}'` | Generic constraint violation |
| E4002 | `Trait '{trait}' not found` | Trait not found |
| E4003 | `Missing implementation for trait '{trait}' for type '{type}'` | Missing trait implementation |
| E4004 | `Conflicting trait implementations for '{trait}'` | Trait implementation conflict |
| E4005 | `Associated type '{assoc_type}' not found in '{container}'` | Associated type not found |

## E5xxx -- Modules and Imports

Errors related to module system and imports.

| Error Code | Template | Description |
|------------|----------|-------------|
| E5001 | `Module '{module}' not found` | Module not found |
| E5002 | `Failed to import module '{module}': {reason}` | Import error |
| E5003 | `Export '{export}' not found in module '{module}'` | Export not found |
| E5004 | `Circular dependency detected: {path}` | Circular dependency |
| E5005 | `Invalid module path: '{path}'` | Invalid module path |
| E5006 | `Duplicate import: '{name}' is already imported` | Duplicate import |
| E5007 | `Module '{module}' exports: {available}` | Module export hint |

## E6xxx -- Runtime

Errors produced during runtime.

| Error Code | Template | Description |
|------------|----------|-------------|
| E6001 | `Division by zero in expression: {expr}` | Division by zero |
| E6002 | `Null pointer dereference at {location}` | Null pointer dereference |
| E6003 | `Array index out of bounds: valid range is 0..{max}, found {index}` | Array index out of bounds |
| E6004 | `Stack overflow: recursion depth exceeded limit {limit}` | Stack overflow |
| E6005 | `Assertion failed: {condition}` | Assertion failed |
| E6006 | `Function not found: '{func}'` | Function not found (runtime) |
| E6007 | `Runtime error: {message}` | Runtime error |

## E7xxx -- I/O and System

I/O operations and system-level errors.

| Error Code | Template | Description |
|------------|----------|-------------|
| E7001 | `File not found: '{path}'` | File not found |
| E7002 | `Permission denied: '{path}'` | Permission denied |
| E7003 | `I/O error: {reason}` | I/O error |
| E7004 | `Network error: {reason}` | Network error |

## E8xxx -- Internal Compiler Errors

Internal compiler errors, usually indicating a bug in the compiler itself. Please report such errors at [GitHub Issues](https://github.com/yaoxiang/yaoxiang/issues).

| Error Code | Template | Description |
|------------|----------|-------------|
| E8001 | `Internal compiler error: {message}` | Internal compiler error |
| E8002 | `Unexpected compiler panic: {reason}` | Unexpected panic |
| E8003 | `Compiler phase error: {phase} - {message}` | Compiler phase error |

## W1xxx -- Warnings

Warnings related to dead code detection. Warnings do not prevent compilation but indicate possible issues in the code.

| Error Code | Template | Description |
|------------|----------|-------------|
| W1001 | `Unused exported function: '{name}'` | Unused exported function |
| W1002 | `Unused exported type: '{name}'` | Unused exported type |
| W1003 | `Unused import: '{name}'` | Unused import |
| W1004 | `Unused exported variable: '{name}'` | Unused exported variable |
| W1005 | `Unused exported method: '{name}'` | Unused exported method |

---

Total: **83** diagnostic codes (78 error codes + 5 warning codes).
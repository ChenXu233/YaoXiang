# Error Code Reference

> Complete reference for YaoXiang compiler error codes, organized by category.

## Quick Navigation

| Category | Range | Description |
|----------|-------|-------------|
| [Lexer & Parser](./E0xxx.md) | E0xxx | Lexical and parsing errors |
| [Type Checking](./E1xxx.md) | E1xxx | Type system errors |
| [Semantic Analysis](./E2xxx.md) | E2xxx | Scope, lifetime, etc. |
| [Generics & Traits](./E4xxx.md) | E4xxx | Generic bounds, trait impl |
| [Modules & Imports](./E5xxx.md) | E5xxx | Module resolution, imports |
| [Runtime Errors](./E6xxx.md) | E6xxx | Execution-time errors |
| [I/O & System Errors](./E7xxx.md) | E7xxx | File, network, system errors |
| [Internal Compiler Errors](./E8xxx.md) | E8xxx | Compiler internal errors |

## Error Message Format

```
error[E0001]: Type mismatch: expected `Int`, found `String`
  --> file.yx:10:5
   |
10 | x: Int = "hello";
   |            ^^^^^^^^ expected `Int`, found `String`
   |
   = help: Consider using `.to_int()` method
```

## Using `yaoxiang explain`

```bash
# View error details
yaoxiang explain E0001

# Specify language
yaoxiang explain E0001 --lang en

# JSON format (for IDE/LSP)
yaoxiang explain E0001 --json
```

## Related Resources

- [Error System Design](../../old/guides/error-system-design.md)
- [RFC-013: Error Code Specification](../../old/design/accepted/013-error-code-specification.md)

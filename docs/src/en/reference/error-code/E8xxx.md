# E8xxx: Internal Compiler Errors

> Compiler internal errors, usually bugs.

## E8001: Internal compiler error

Compiler internal error.

```
error[E8001]: Internal compiler error
  --> example.yx:1:1
   |
 1 | [compiler internal error]
   |
   = note: This is a bug in the YaoXiang compiler
   = please file an issue at https://github.com/yaoxiang-lang/yaoxiang/issues
```

## E8002: Codegen error

IR/bytecode generation failed.

```
error[E8002]: Codegen error
  --> example.yx:5:1
   |
 5 | [code generation failed]
   |
   = note: Failed to generate bytecode for function
```

## E8003: Unimplemented feature

Using an unimplemented feature.

```yaoxiang
main: () -> Void = {
    # Some not yet implemented feature
}
```

```
error[E8003]: Unimplemented feature
  --> example.yx:2:5
   |
 2 |     [feature not yet implemented]
   |
   = note: This feature is not yet implemented
```

## E8004: Optimization error

Compiler optimization error.

```
error[E8004]: Optimization error
  --> example.yx:1:1
   |
 1 | [compiler optimization failed]
   |
   = note: This is a bug in the compiler's optimizer
```

## Reporting Internal Errors

When encountering E8xxx errors:

1. Verify the error is reproducible
2. Collect a minimal reproduction case
3. Report at [GitHub Issues](https://github.com/yaoxiang-lang/yaoxiang/issues)

## Related

- [E6xxx: Runtime Errors](./E6xxx.md)
- [E7xxx: I/O & System Errors](./E7xxx.md)
- [Error Code Index](./index.md)

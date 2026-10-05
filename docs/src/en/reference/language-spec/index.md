# Language Specification

The syntax and semantic specification document for the yaoxiang language.

## Table of Contents

- [Syntax Specification](syntax.md) - Lexical structure, grammar rules, operator precedence
- [Type System](type-system.md) - Primitive types, composite types, generics, trait
- [Module System](modules.md) - Module definition, import/export, scope
- [FFI Specification](ffi.md) - `native` extensions, C ABI type mapping and opaque types
- [Concurrency Model](concurrency.md) - Block semantics, spawn concurrency primitives, error
  handling, resource types
- [Standard Library](stdlib.md) - Module overview, core library, IO library, math library

## Overview

This specification defines the core syntax and semantics of the YaoXiang programming language. It is
the authoritative reference for the language, intended for compiler and tool implementers.

### Specification Structure

1. **Syntax Specification**: Defines the language's lexical structure, grammar rules, and operator
   precedence
2. **Type System**: Defines the language's type system, including primitive types, composite types,
   generics, and trait
3. **Module System**: Defines the language's module system, including module definition,
   import/export, and scope
4. **FFI Specification**: Defines `native("symbol")` declarations, `unsafe {}` opaque types, and
   `[0]` method bindings
5. **Concurrency Model**: Defines the language's concurrency model, including spawn primitives and
   the memory model
6. **Standard Library**: Defines the language's standard library, including the core library, IO
   library, and math library

### Version Information

- Compiler version: Synchronized with the `version` field in `Cargo.toml`, current
  <!-- yx-version -->
- Status: Specification
- Author: Chenxu

> This page no longer self-reports a "specification version" — the specification is released
> alongside the compiler at the same version, and the version number is governed by the
> `CHANGELOG.md` and `Cargo.toml` at the repository root, to prevent the two numbers from drifting
> apart. The **module-level API signatures** in this directory are derived from
> `StdModule::exports()` and guarded by a gate (see
> [Standard Library Reference](../stdlib/index.md#文档维护)).

### Related Resources

- [Tutorial](../../tutorial/index.md) - Getting started tutorial and example code
- [Design Documents](../../explanation/) - Language design documents
- [RFC Documents](../../rfc/) - Language change proposals

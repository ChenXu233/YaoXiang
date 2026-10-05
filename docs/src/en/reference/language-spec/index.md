# Language Specification

The syntax and semantic specification document for the yaoxiang language.

## Table of Contents

- [Syntax Specification](./syntax.md) - Lexical structure, syntax rules, operator precedence
- [Type System](./type-system.md) - Primitive types, composite types, generics, trait
- [Module System](./modules.md) - Module definitions, imports/exports, scopes
- [FFI Specification](./ffi.md) - `native` extensions, C ABI type mapping, and opaque types
- [Concurrency Model](./concurrency.md) - Block semantics, spawn concurrency primitive, error
  handling, resource types
- [Standard Library](./stdlib.md) - Module overview, core library, IO library, math library

## Overview

This specification defines the core syntax and semantics of the YaoXiang programming language. It is
the authoritative reference for the language, aimed at compiler and tool implementers.

### Specification Structure

1. **Syntax Specification**: Defines the lexical structure, syntax rules, and operator precedence of
   the language
2. **Type System**: Defines the type system of the language, including primitive types, composite
   types, generics, and trait
3. **Module System**: Defines the module system of the language, including module definitions,
   imports/exports, and scopes
4. **FFI Specification**: Defines `native("symbol")` declarations, `unsafe {}` opaque types, and
   `[0]` method bindings
5. **Concurrency Model**: Defines the concurrency model of the language, including spawn primitives
   and memory model
6. **Standard Library**: Defines the standard library of the language, including the core library,
   IO library, and math library

### Version Information

- Compiler version: Synchronized with the `version` in `Cargo.toml`, currently **0.8.3**
- Status: Specification
- Author: Chenxu

> This page no longer self-reports a "specification version" — the specification is released in
> lockstep with the compiler, and the version number is governed by `CHANGELOG.md` and `Cargo.toml`
> at the repository root to avoid drift between the two. The **module-level API signatures** in this
> directory are derived from `StdModule::exports()` and gated by a CI check (see
> [Standard Library Reference](../stdlib/index.md#documentation-maintenance)).

### Related Resources

- [Tutorial](../../tutorial/index.md) - Getting started tutorial and example code
- [Design Documents](../../design/) - Language design documents
- [RFC Documents](../../design/rfc/) - Language change proposals

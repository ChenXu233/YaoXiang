# Language Specification

The syntax and semantics specification document for the yaoxiang language.

## Contents

- [Syntax Specification](./syntax.md) - Lexical structure, syntax rules, operator precedence
- [Type System](./type-system.md) - Primitive types, composite types, generics, traits
- [Module System](./modules.md) - Module definition, imports/exports, scope
- [FFI Specification](./ffi.md) - `native` extensions, C ABI type mapping, and opaque types
- [Concurrency Model](./concurrency.md) - Block semantics, spawn concurrency primitives, error
  handling, resource types
- [Standard Library](./stdlib.md) - Module overview, core library, IO library, math library

## Overview

This specification defines the core syntax and semantics of the YaoXiang programming language. It is
the authoritative reference for the language, aimed at compiler and tool implementers.

### Specification Structure

1. **Syntax Specification**: Defines the lexical structure, syntax rules, and operator precedence of
   the language
2. **Type System**: Defines the language's type system, including primitive types, composite types,
   generics, and traits
3. **Module System**: Defines the language's module system, including module definition,
   imports/exports, and scope
4. **FFI Specification**: Defines `native("symbol")` declarations, `unsafe {}` opaque types, and
   `[0]` method bindings
5. **Concurrency Model**: Defines the language's concurrency model, including spawn primitives and
   the memory model
6. **Standard Library**: Defines the language's standard library, including the core library, IO
   library, and math library

### Version Information

- Compiler version: Synced with the `version` in `Cargo.toml`, currently **0.8.2**
- Status: Specification
- Author: 晨煦

> This page no longer self-reports a "specification version" — the specification is released in
> lockstep with the compiler, and the version number is determined by `CHANGELOG.md` and
> `Cargo.toml` at the repository root, to prevent the two numbers from drifting. The **module-level
> API signatures** in this directory are derived from `StdModule::exports()` and are guarded by a
> gate (see [Standard Library Reference](../stdlib/index.md#documentation-maintenance)).

### Related Resources

- [Tutorial](../../tutorial/index.md) - Getting started tutorial and example code
- [Design Documents](../../design/) - Language design documents
- [RFC Documents](../../design/rfc/) - Language change proposals

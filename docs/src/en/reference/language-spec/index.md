# Language Specification

Syntax and semantic specification document for YaoXiang language.

## Table of Contents

- [Syntax Specification](./syntax.md) - Lexical structure, grammar rules, operator precedence
- [Type System](./type-system.md) - Primitive types, composite types, generics, trait
- [Module System](./modules.md) - Module definition, import/export, scope
- [Concurrency Model](./concurrency.md) - {} block semantics, spawn concurrency primitive, error handling, resource types
- [Standard Library](./stdlib.md) - Core library, IO library, math library

## Overview

This specification defines the core syntax and semantics of the YaoXiang programming language. It is the authoritative reference for the language, targeting compiler and tool implementers.

### Specification Structure

1. **Syntax Specification**: Defines the lexical structure, grammar rules, and operator precedence of the language
2. **Type System**: Defines the type system of the language, including primitive types, composite types, generics, and trait
3. **Module System**: Defines the module system of the language, including module definition, import/export, and scope
4. **Concurrency Model**: Defines the concurrency model of the language, including async programming, concurrency primitives, and memory model
5. **Standard Library**: Defines the standard library of the language, including core library, IO library, and math library

### Version Information

- Current Version: v1.8.0
- Status: Specification
- Author: Chen Xu
- Last Updated: 2026-02-22

### Related Resources

- [Tutorial](../../tutorial/index.md) - Getting started tutorial with example code
- [Design Documents](../../design/) - Language design documents
- [RFC Documents](../../design/rfc/) - Language change proposals
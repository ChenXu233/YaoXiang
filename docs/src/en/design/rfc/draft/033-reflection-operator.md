---
title: 'RFC-033: `^^` Reflection Operator'
status: 'Under Review'
author: 'Chen Xu'
created: '2026-06-16'
updated: '2026-07-05'
issue: '#136'
---

# RFC-033: `^^` Reflection Operator

> **References**:
>
> - [RFC-010: Unified Type Syntax - name: type = value Model](../accepted/010-unified-type-syntax.md)
> - [RFC-011: Generic Type System Design](../accepted/011-generic-type-system.md)
> - [RFC-027: Compile-Time Predicates and Unified Static Verification](../accepted/027-compile-time-evaluation-types.md)
> - [RFC-011a: Interface Implementation and Dynamic Dispatch](../draft/011a-interface-implementation.md)

## Summary

This document proposes introducing the `^^` operator as a reflection entry point for accessing type
and value metadata. `^^T` returns a static metadata object for type `T`, and `^^obj` returns dynamic
type metadata for value `obj`. Metadata objects are ordinary record types containing information
such as name, parameters, and fields, usable at both compile-time and runtime.

## Motivation

### Why is this feature needed?

1. **Serialization/deserialization**: Need to access field information of types to automatically
   generate serialization code
2. **Compile-time metaprogramming**: Need to access type structure at compile-time to generate code
   or verify constraints
3. **Runtime debugging/tools**: Need to print type information at runtime for debugging assistance
4. **Runtime type checking**: Need to determine type relationships at runtime, such as "what type is
   obj?"

### Current problems

Currently, YaoXiang has no reflection mechanism to access type metadata at compile-time or runtime.
If `.name`, `.fields` were used directly to access type metadata, they would conflict with
user-defined fields:

```yaoxiang
Person: Type = { name: String, age: Int }

# If Person.name is the type metadata name, or the field name?
# This causes parsing difficulties and semantic confusion
```

A syntax that does **not intrude on the ordinary field namespace** is needed to access type
metadata.

## Proposal

### Core design

Introduce the `^^` operator as a reflection entry point, clearly distinguishing between ordinary
code and metadata queries.

**Two usage patterns**:

1. **Static reflection (acting on types)**: `^^T` returns a static metadata object for type `T`
2. **Dynamic reflection (acting on values)**: `^^obj` returns dynamic type metadata for value `obj`

**Metadata structure**:

```yaoxiang
TypeMeta: Type = {
    name: String,
    params: Array(ParamMeta),
    fields: Array(FieldMeta),
    return_type: Type,
    refinement: Option(Expr)  # Some(Expr) at compile-time, None at runtime
}

ParamMeta: Type = {
    name: String,
    type: Type
}

FieldMeta: Type = {
    name: String,
    type: Type
}
```

**Universe level**: If `T: Type_n`, then `^^T: Type_{n+1}`, conforming to standard universe lifting
rules in type theory.

**Precedence**: `^^` is a unary prefix operator with the highest precedence. `^^T.name` is
equivalent to `^^T.name`.

### Examples

#### Basic usage

```yaoxiang
Point: Type = { x: Float, y: Float }

# Static reflection
meta = ^^Point
print(meta.name)           # "Point"
print(meta.fields.len)     # 2
print(meta.fields[0].name) # "x"
print(meta.fields[0].type) # Float

# Dynamic reflection (requires enabling runtime reflection)
obj = Point(1.0, 2.0)
meta = ^^obj
print(meta.name)           # "Point"
```

#### Generic types

```yaoxiang
List: (T: Type) -> Type = { data: Array(T), length: Int }

# Reflect on the generic type itself
meta = ^^List
print(meta.name)           # "List"
print(meta.params)         # [{ name: "T", type: Type }]

# Reflect on a concrete instantiation type
meta = ^^List(Int)
print(meta.name)           # "List(Int)"
print(meta.params)         # []
```

#### Functions

```yaoxiang
add: (a: Int, b: Int) -> Int = a + b

meta = ^^add
print(meta.name)           # "add"
print(meta.params)         # [{ name: "a", type: Int }, { name: "b", type: Int }]
print(meta.return_type)    # Int
```

#### Refinement types

```yaoxiang
Positive: (x: Int) -> Type = { x > 0 }

# At compile-time: refinement is Some(Expr)
meta = ^^Positive
print(meta.name)           # "Positive"
print(meta.refinement)     # Some(AST(x > 0))

# At runtime: refinement is None (erased)
```

#### Usage in compile-time predicates

```yaoxiang
# Check if a type has fields
HasFields: (T: Type) -> Type = { ^^T.fields.len > 0 }

# Check field type
HasFloatField: (T: Type) -> Type = {
    exists field in ^^T.fields: field.type == Float
}

# Usage
obj: HasFields(Point) = Point(1.0, 2.0)  # ✅ Validation passes
# obj: HasFields(Int) = 42  # ❌ Validation fails
```

#### Serialization example

```yaoxiang
# Compile-time pure function: generates JSON string
to_json: (T: Type) -> ((obj: T) -> String) = {
    meta = ^^T
    parts: Array(String) = []
    for field in meta.fields {
        # Compile-time code generation for field access
        parts.push("\"${field.name}\": ${obj.${field.name}}")
    }
    return "{" + parts.join(", ") + "}"
}

# Usage
point_to_json = to_json(Point)
print(point_to_json(Point(1.0, 2.0)))  # '{"x": 1.0, "y": 2.0}'
```

### Syntax changes

| Before        | After                                        |
| ------------- | -------------------------------------------- |
| No reflection | `^^T` gets type metadata                     |
| No reflection | `^^obj` gets dynamic type metadata for value |

## Detailed design

### Type system impact

- **New types**: `TypeMeta`, `ParamMeta`, `FieldMeta`
- **Universe level**: The type returned by `^^T` is one level higher than `T`
- **Generic interaction**: Both `^^List` and `^^List(Int)` are supported
- **Function interaction**: `^^add` returns function metadata (including parameters and return type)
- **Refinement type interaction**: `^^Positive` returns refinement type metadata (including
  refinement expression)

### Runtime behavior

**Compile-time reflection**:

- `^^T` is fully evaluated at compile-time, with results inlined as constants
- Refinement expressions are available at compile-time

**Runtime reflection**:

- Disabled by default, zero overhead
- Enabled via `--enable-runtime-reflection` compilation option
- When enabled, `^^obj` returns dynamic type metadata
- Refinement expressions are erased to `None` at runtime

**On-demand generation + treeshake**:

- Only types that actually use `^^` generate metadata
- Unreferenced types do not generate metadata (treeshake)

### Compiler changes

1. **Lexer**: Recognize `^^` as a single token
2. **Parser**: Add `^^` prefix expression rule
3. **Type system**: Add `TypeMeta`, `ParamMeta`, `FieldMeta` type definitions
4. **Type checker**: Generate metadata instances for each type
5. **Compile-time evaluator**: Support compile-time evaluation of `^^T`
6. **Runtime (optional)**: Generate RTTI for reflected types

### Backward compatibility

- ✅ No impact on existing syntax: `^^` is a new operator, does not conflict with existing syntax
- ✅ No impact on existing types: All types automatically support `^^`
- ✅ No impact on existing functions: Functions may use `^^` but are not forced to
- ✅ No impact on compile-time predicates: `^^T` behaves consistently in predicates
- ✅ No runtime impact: Runtime reflection is disabled by default, zero overhead

## Trade-offs

### Advantages

- **Uniformity**: Functions, generics, and refinement types are handled uniformly
- **Zero overhead**: Compile-time reflection is fully erased; runtime reflection is optional
- **Integration with existing systems**: Seamlessly integrates with compile-time predicates
  (RFC-027)
- **Conciseness**: `^^` is pure symbols, does not conflict with user-defined identifiers
- **On-demand generation**: Treeshake optimization, unused types have zero overhead

### Disadvantages

- **Learning curve**: Need to understand the semantics of `^^` and metadata structure
- **Runtime overhead**: Enabling runtime reflection increases memory overhead (one pointer per
  instance)
- **Implementation complexity**: Requires modifying multiple compiler components

## Alternative approaches

| Approach                | Why not chosen                                                                             |
| ----------------------- | ------------------------------------------------------------------------------------------ |
| `reflect(T)` function   | Introduces additional identifiers into scope, may be shadowed by users                     |
| `type_info(T)` function | Same as above                                                                              |
| Single `^` operator     | May conflict with bitwise operations; C++26 chose `^^` precisely because of such conflicts |
| `@@`, `##` or similar   | No precedent exists; less intuitive than `^^`                                              |

## Implementation phases

| Phase   | Content                               | Dependency |
| ------- | ------------------------------------- | ---------- |
| Phase 1 | Compile-time `^^` operator parsing    | None       |
| Phase 2 | `TypeMeta` data structure definition  | Phase 1    |
| Phase 3 | Compile-time metadata generation      | Phase 2    |
| Phase 4 | Runtime reflection support (optional) | Phase 3    |
| Phase 5 | Compile-time predicate integration    | Phase 3    |

### Dependency graph

```
Phase 1 (Parsing)
    ↓
Phase 2 (Data structures)
    ↓
Phase 3 (Compile-time metadata)
    ↓
    ├────────────┐
    ↓            ↓
Phase 4        Phase 5
(Runtime      (Compile-time
reflection)    predicates)
```

### Risks

- **Parsing conflict**: `^^` may conflict with existing syntax (analysis shows no conflict)
- **Performance impact**: Compile-time metadata generation may increase compilation time (can be
  optimized with treeshake)
- **Runtime overhead**: Enabling runtime reflection increases memory overhead (mitigated by
  on-demand generation)

## Open questions

- [x] Scope of `^^`: Acts only on types and values, not on expressions
- [x] Chained access: Supported; metadata objects returned by `^^T` can normally access properties
- [x] Pattern matching: Supported; `TypeMeta` is an ordinary record type, can normally be
      pattern-matched
- [x] Comparison: Supported; metadata objects of the same type are equal
- [x] Memory overhead: On-demand generation + treeshake optimization

---

## Appendix

### Appendix A: Design decision records

| Decision                      | Decision                                             | Date       | Recorder |
| ----------------------------- | ---------------------------------------------------- | ---------- | -------- |
| Scope of `^^`                 | Acts only on types and values, not on expressions    | 2026-06-16 | Chen Xu  |
| Chained access                | Supported                                            | 2026-06-16 | Chen Xu  |
| Pattern matching              | Supported                                            | 2026-06-16 | Chen Xu  |
| Comparison                    | Supported; metadata of the same type is equal        | 2026-06-16 | Chen Xu  |
| Memory overhead               | On-demand generation + treeshake                     | 2026-06-16 | Chen Xu  |
| Generic interaction           | Both `^^List` and `^^List(Int)` are supported        | 2026-06-16 | Chen Xu  |
| Refinement expression storage | Available at compile-time, erased to None at runtime | 2026-06-16 | Chen Xu  |

### Appendix B: Glossary

| Term            | Definition                                                                        |
| --------------- | --------------------------------------------------------------------------------- |
| Reflection      | The ability to access type metadata at runtime or compile-time                    |
| Metadata        | Information describing type structure (name, fields, parameters, etc.)            |
| RTTI            | Run-Time Type Information, runtime type information                               |
| Treeshake       | Compiler optimization that removes unused code                                    |
| Refinement type | A type with constraint conditions, e.g., `Positive: (x: Int) -> Type = { x > 0 }` |

---

## References

- [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md)
- [RFC-011: Generic Type System Design](../accepted/011-generic-type-system.md)
- [RFC-027: Compile-Time Predicates and Unified Static Verification](../accepted/027-compile-time-evaluation-types.md)
- [RFC-011a: Interface Implementation and Dynamic Dispatch](../draft/011a-interface-implementation.md)
- [C++26 Reflection Proposal](https://wg21.link/P2996)

---

## Lifecycle and disposition

```
┌─────────────┐
│   Draft     │  ← Current status
└──────┬──────┘
       │
       ▼
┌─────────────┐
│Under Review │  ← Open for community discussion and feedback
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  Accepted   │    │  Rejected   │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   accepted/ │    │    rfc/     │
│ (Final spec)│    │ (Kept in    │
│             │    │  original)  │
└─────────────┘    └─────────────┘
```

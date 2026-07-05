---
title: "RFC-033: `^^` Reflection Operator"
status: "Under Review"
author: "Chenxu"
created: "2026-06-16"
updated: "2026-07-05"
issue: "#136"
---

# RFC-033: `^^` Reflection Operator

> **References**:
>
> - [RFC-010: Unified Type Syntax - name: type = value Model](../accepted/010-unified-type-syntax.md)
> - [RFC-011: Generics Type System Design](../accepted/011-generic-type-system.md)
> - [RFC-027: Compile-time Predicates and Unified Static Verification](../accepted/027-compile-time-evaluation-types.md)
> - [RFC-011a: Interface Implementation and Dynamic Dispatch](../draft/011a-interface-implementation.md)

## Summary

This RFC proposes introducing the `^^` operator as a reflection entry point, used to obtain metadata of types and values. `^^T` returns the static metadata object of type `T`, and `^^obj` returns the dynamic type metadata of value `obj`. The metadata object is a regular record type containing information such as the name, parameters, and fields, and can be used at compile-time and runtime.

## Motivation

### Why is this feature needed?

1. **Serialization/Deserialization**: Need to access field information of types to automatically generate serialization code
2. **Compile-time Metaprogramming**: Need to access type structure at compile-time to generate code or verify constraints
3. **Runtime Debugging/Tools**: Need to print type information at runtime to assist debugging
4. **Runtime Type Checking**: Need to determine type relationships at runtime, such as "what type is obj?"

### Current Problems

Currently, YaoXiang has no reflection mechanism, and there is no way to access type metadata at compile-time or runtime. If we directly use `.name`, `.fields` to access type metadata, it will conflict with user-defined fields:

```yaoxiang
Person: Type = { name: String, age: Int }

# Is Person.name the name of the type metadata, or the field name?
# This would cause parsing difficulties and semantic confusion
```

A syntax that **does not intrude into the regular field namespace** is needed to access type metadata.

## Proposal

### Core Design

Introduce the `^^` operator as a reflection entry point, clearly distinguishing regular code from metadata queries.

**Two usages**:

1. **Static reflection (acts on types)**: `^^T` returns the static metadata object of type `T`
2. **Dynamic reflection (acts on values)**: `^^obj` returns the dynamic type metadata of value `obj`

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

**Universe level**: If `T: Type_n`, then `^^T: Type_{n+1}`, conforming to the standard universe lifting rule in type theory.

**Precedence**: `^^` is a unary prefix operator with the highest precedence. `^^T.name` is equivalent to `(^^T).name`.

### Examples

#### Basic Usage

```yaoxiang
Point: Type = { x: Float, y: Float }

# Static reflection
meta = ^^Point
print(meta.name)           # "Point"
print(meta.fields.len)     # 2
print(meta.fields[0].name) # "x"
print(fields[0].type)      # Float

# Dynamic reflection (requires enabling runtime reflection)
obj = Point(1.0, 2.0)
meta = ^^obj
print(meta.name)           # "Point"
```

#### Generic Types

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

#### Refinement Types

```yaoxiang
Positive: (x: Int) -> Type = { x > 0 }

# Compile-time: refinement is Some(Expr)
meta = ^^Positive
print(meta.name)           # "Positive"
print(meta.refinement)     # Some(AST(x > 0))

# Runtime: refinement is None (erased)
```

#### Usage in Compile-time Predicates

```yaoxiang
# Check if a type has fields
HasFields: (T: Type) -> Type = { ^^T.fields.len > 0 }

# Check field types
HasFloatField: (T: Type) -> Type = {
    exists field in ^^T.fields: field.type == Float
}

# Usage
obj: HasFields(Point) = Point(1.0, 2.0)  # ✅ Verification passes
# obj: HasFields(Int) = 42  # ❌ Verification fails
```

#### Serialization Example

```yaoxiang
# Compile-time pure function: generates a JSON string
to_json: (T: Type) -> ((obj: T) -> String) = {
    meta = ^^T
    parts: Array(String) = []
    for field in meta.fields {
        # Generate field access code at compile-time
        parts.push("\"${field.name}\": ${obj.${field.name}}")
    }
    return "{" + parts.join(", ") + "}"
}

# Usage
point_to_json = to_json(Point)
print(point_to_json(Point(1.0, 2.0)))  # '{"x": 1.0, "y": 2.0}'
```

### Syntax Changes

| Before | After |
|------|------|
| No reflection mechanism | `^^T` obtains type metadata |
| No reflection mechanism | `^^obj` obtains the dynamic type metadata of a value |

## Detailed Design

### Type System Impact

- **New types**: `TypeMeta`, `ParamMeta`, `FieldMeta`
- **Universe level**: The type returned by `^^T` is one level higher than `T`
- **Generics interaction**: Both `^^List` and `^^List(Int)` are supported
- **Function interaction**: `^^add` returns the metadata of a function (including parameters and return type)
- **Refinement type interaction**: `^^Positive` returns the metadata of a refinement type (including the refinement expression)

### Runtime Behavior

**Compile-time reflection**:
- `^^T` is fully evaluated at compile-time, and the result is inlined as a constant
- The refinement expression is available at compile-time

**Runtime reflection**:
- Disabled by default, zero overhead
- Enabled via the `--enable-runtime-reflection` compilation option
- Once enabled, `^^obj` returns dynamic type metadata
- The refinement expression is erased to `None` at runtime

**On-demand generation + treeshake**:
- Metadata is generated only for types that actually use `^^`
- Types that are not referenced do not generate metadata (treeshake)

### Compiler Changes

1. **Lexer**: Recognize `^^` as a single token
2. **Parser**: Add `^^` prefix expression rules
3. **Type system**: Add definitions for `TypeMeta`, `ParamMeta`, `FieldMeta` types
4. **Type checker**: Generate metadata instances for each type
5. **Compile-time evaluator**: Support compile-time evaluation of `^^T`
6. **Runtime (optional)**: Generate RTTI for types that are reflected upon

### Backward Compatibility

- ✅ No impact on existing syntax: `^^` is a new operator and does not conflict with existing syntax
- ✅ No impact on existing types: All types automatically support `^^`
- ✅ No impact on existing functions: Functions can use `^^` but are not required to
- ✅ No impact on compile-time predicates: `^^T` in predicates is consistent with regular content
- ✅ No impact on runtime: Runtime reflection is disabled by default, zero overhead

## Trade-offs

### Advantages

- **Uniformity**: Functions, generics, and refinement types are handled uniformly
- **Zero overhead**: Compile-time reflection is fully erased; runtime reflection is optional
- **Integration with existing systems**: Seamless integration with compile-time predicates (RFC-027)
- **Concise**: `^^` is a pure symbol and does not conflict with user-defined identifiers
- **On-demand generation**: Treeshake optimization ensures zero overhead for unused types

### Disadvantages

- **Learning curve**: Need to understand the semantics of `^^` and the metadata structure
- **Runtime overhead**: Enabling runtime reflection increases memory overhead (one pointer per instance)
- **Implementation complexity**: Multiple compiler components need to be modified

## Alternatives

| Alternative | Why Not Chosen |
|------|--------------|
| `reflect(T)` function | Would introduce an extra identifier into the scope, which may be shadowed by the user |
| `type_info(T)` function | Same as above |
| Single `^` operator | May conflict with bitwise operations, and C++26 chose `^^` precisely because of this conflict |
| `@@`, `##` and other symbols | No precedent, not as easy to explain as `^^` |

## Implementation Phases

| Phase | Content | Dependencies |
|------|------|------|
| Phase 1 | Compile-time `^^` operator parsing | None |
| Phase 2 | `TypeMeta` data structure definition | Phase 1 |
| Phase 3 | Compile-time metadata generation | Phase 2 |
| Phase 4 | Runtime reflection support (optional) | Phase 3 |
| Phase 5 | Compile-time predicate integration | Phase 3 |

### Dependency Graph

```
Phase 1 (Parsing)
    ↓
Phase 2 (Data Structure)
    ↓
Phase 3 (Compile-time Metadata)
    ↓
    ├────────────┐
    ↓            ↓
Phase 4        Phase 5
(Runtime Reflection)  (Compile-time Predicates)
```

### Risks

- **Parsing conflicts**: `^^` may conflict with existing syntax (analysis shows no conflicts)
- **Performance impact**: Compile-time metadata generation may increase compilation time (can be optimized via treeshake)
- **Runtime overhead**: Enabling runtime reflection increases memory overhead (mitigated by on-demand generation)

## Open Questions

- [x] Scope of `^^`: Acts only on types and values, not on expressions
- [x] Chained access: Supported, the metadata object returned by `^^T` can have its properties accessed normally
- [x] Pattern matching: Supported, `TypeMeta` is a regular record type and can be pattern-matched normally
- [x] Comparison: Supported, metadata objects of the same type are equal
- [x] Memory overhead: On-demand generation + treeshake optimization

---

## Appendix

### Appendix A: Design Decision Records

| Decision | Resolution | Date | Recorder |
|------|------|------|--------|
| Scope of `^^` | Acts only on types and values, not on expressions | 2026-06-16 | Chenxu |
| Chained access | Supported | 2026-06-16 | Chenxu |
| Pattern matching | Supported | 2026-06-16 | Chenxu |
| Comparison | Supported, metadata of the same type is equal | 2026-06-16 | Chenxu |
| Memory overhead | On-demand generation + treeshake | 2026-06-16 | Chenxu |
| Generics interaction | Both `^^List` and `^^List(Int)` are supported | 2026-06-16 | Chenxu |
| Refinement expression storage | Available at compile-time, erased to None at runtime | 2026-06-16 | Chenxu |

### Appendix B: Glossary

| Term | Definition |
|------|------|
| Reflection | The ability to access type metadata at runtime or compile-time |
| Metadata | Information describing type structure (name, fields, parameters, etc.) |
| RTTI | Run-Time Type Information |
| Treeshake | Compiler optimization that removes unused code |
| Refinement type | A type with constraint conditions, e.g. `Positive: (x: Int) -> Type = { x > 0 }` |

---

## References

- [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md)
- [RFC-011: Generics Type System Design](../accepted/011-generic-type-system.md)
- [RFC-027: Compile-time Predicates and Unified Static Verification](../accepted/027-compile-time-evaluation-types.md)
- [RFC-011a: Interface Implementation and Dynamic Dispatch](../draft/011a-interface-implementation.md)
- [C++26 Reflection Proposal](https://wg21.link/P2996)

---

## Lifecycle and Destination

```
┌─────────────┐
│   Draft     │  ← Current status
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Under Review│  ← Open community discussion and feedback
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
│ (Official Design) │ (Kept in place) │
└─────────────┘    └─────────────┘
```
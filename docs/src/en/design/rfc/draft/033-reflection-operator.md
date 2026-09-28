---
title: 'RFC-033: `^^` Reflection Operator'
status: 'Under Review'
author: 'Chenxu'
created: '2026-06-16'
updated: '2026-07-05'
issue: '#136'
---

# RFC-033: `^^` Reflection Operator

> **References**:
>
> - [RFC-010: Unified Type Syntax - name: type = value model](../accepted/010-unified-type-syntax.md)
> - [RFC-011: Generic Type System Design](../accepted/011-generic-type-system.md)
> - [RFC-027: Compile-Time Predicates and Unified Static Verification](../accepted/027-compile-time-evaluation-types.md)
> - [RFC-011a: Interface Implementation and Dynamic Dispatch](../accepted/011a-interface-implementation.md)

## Summary

This document proposes introducing the `^^` operator as a reflection entry point for obtaining type
and value metadata. `^^T` returns the static metadata object of type `T`, and `^^obj` returns the
dynamic type metadata of value `obj`. The metadata object is a regular record type containing
information such as name, parameters, and fields, usable at both compile-time and runtime.

## Motivation

### Why is this feature needed?

1. **Serialization/deserialization**: Need to access field information of types to automatically
   generate serialization code
2. **Compile-time metaprogramming**: Need to access type structure at compile-time to generate code
   or verify constraints
3. **Runtime debugging/tools**: Need to print type information at runtime to assist debugging
4. **Runtime type checking**: Need to determine type relationships at runtime, such as "what type is
   obj?"

### Current Problems

Currently, YaoXiang has no reflection mechanism and cannot access type metadata at compile-time or
runtime. If `.name` or `.fields` were used directly to access type metadata, they would conflict
with user-defined fields:

```yaoxiang
Person: Type = { name: String, age: Int }

# If Person.name is the type metadata name, or the field name?
# This leads to parsing difficulties and semantic confusion
```

A syntax that does **not intrude into the normal field namespace** is needed to access type
metadata.

## Proposal

### Core Design

Introduce the `^^` operator as a reflection entry point, clearly distinguishing between normal code
and metadata queries.

**Two usages**:

1. **Static reflection (applies to types)**: `^^T` returns the static metadata object of type `T`
2. **Dynamic reflection (applies to values)**: `^^obj` returns the dynamic type metadata of value
   `obj`

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

**Universe level**: If `T: Type_n`, then `^^T: Type_{n+1}`, conforming to the standard universe
lifting rules of type theory.

**Precedence**: `^^` is a unary prefix operator with the highest precedence. `^^T.name` is
equivalent to `^^T).name`.

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

# Dynamic reflection (requires runtime reflection to be enabled)
obj = Point(1.0, 2.0)
meta = ^^obj
print(meta.name)           # "Point"
```

#### Generic Types

```yaoxiang
List: (T: Type) -> Type = { data: Array(T), length: Int }

# Reflect the generic type itself
meta = ^^List
print(meta.name)           # "List"
print(meta.params)         # [{ name: "T", type: Type }]

# Reflect a concrete instantiated type
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
obj: HasFields(Point) = Point(1.0, 2.0)  # ✅ Validation passed
# obj: HasFields(Int) = 42  # ❌ Validation failed
```

#### Serialization Example

```yaoxiang
# Compile-time pure function: generate JSON string
to_json: (T: Type) -> ((obj: T) -> String) = {
    meta = ^^T
    parts: Array(String) = []
    for field in meta.fields {
        # Compile-time generated field access code
        parts.push("\"${field.name}\": ${obj.${field.name}}")
    }
    return "{" + parts.join(", ") + "}"
}

# Usage
point_to_json = to_json(Point)
print(point_to_json(Point(1.0, 2.0)))  # '{"x": 1.0, "y": 2.0}'
```

### Syntax Changes

| Before        | After                                         |
| ------------- | --------------------------------------------- |
| No reflection | `^^T` obtains type metadata                   |
| No reflection | `^^obj` obtains value's dynamic type metadata |

## Detailed Design

### Type System Impact

- **New types**: `TypeMeta`, `ParamMeta`, `FieldMeta`
- **Universe levels**: The type returned by `^^T` is one level higher than `T`
- **Generic interaction**: Both `^^List` and `^^List(Int)` are supported
- **Function interaction**: `^^add` returns function metadata (including parameters and return type)
- **Refinement type interaction**: `^^Positive` returns refinement type metadata (including
  refinement expression)

### Runtime Behavior

**Compile-time reflection**:

- `^^T` is fully evaluated at compile-time, and the result is inlined as a constant
- Refinement expressions are available at compile-time

**Runtime reflection**:

- Disabled by default, zero overhead
- Enabled via the `--enable-runtime-reflection` compilation option
- When enabled, `^^obj` returns dynamic type metadata
- Refinement expressions are erased to `None` at runtime

**On-demand generation + treeshake**:

- Metadata is generated only for types that actually use `^^`
- Types not referenced do not generate metadata (treeshake)

### Compiler Changes

1. **Lexer**: Recognize `^^` as a single token
2. **Parser**: Add `^^` prefix expression rule
3. **Type system**: Add `TypeMeta`, `ParamMeta`, `FieldMeta` type definitions
4. **Type checker**: Generate a metadata instance for each type
5. **Compile-time evaluator**: Support compile-time evaluation of `^^T`
6. **Runtime (optional)**: Generate RTTI for reflected types

### Backward Compatibility

- ✅ No impact on existing syntax: `^^` is a new operator and does not conflict with existing syntax
- ✅ No impact on existing types: All types automatically support `^^`
- ✅ No impact on existing functions: Functions can use `^^` but are not forced to
- ✅ No impact on compile-time predicates: `^^T` behaves consistently in predicates and normal code
- ✅ No runtime impact: Runtime reflection is disabled by default, zero overhead

## Trade-offs

### Advantages

- **Uniformity**: Functions, generics, and refinement types are handled uniformly
- **Zero overhead**: Compile-time reflection is fully erased, runtime reflection is optional
- **Integration with existing systems**: Seamlessly integrates with compile-time predicates
  (RFC-027)
- **Simplicity**: `^^` is pure symbol and does not conflict with user-defined identifiers
- **On-demand generation**: Treeshake optimization, unused types have zero overhead

### Disadvantages

- **Learning curve**: Need to understand the semantics of `^^` and metadata structure
- **Runtime overhead**: Enabling runtime reflection increases memory overhead (one pointer per
  instance)
- **Implementation complexity**: Requires modifying multiple compiler components

## Alternatives

| Approach                     | Why not chosen                                                                             |
| ---------------------------- | ------------------------------------------------------------------------------------------ |
| `reflect(T)` function        | Introduces extra identifier to scope, could be shadowed by users                           |
| `type_info(T)` function      | Same as above                                                                              |
| Single `^` operator          | May conflict with bitwise operations; C++26 chose `^^` precisely because of such conflicts |
| `@@`, `##` and other symbols | No precedent; `^^` is easier to interpret                                                  |

## Implementation Phases

| Phase   | Content                               | Dependency |
| ------- | ------------------------------------- | ---------- |
| Phase 1 | Compile-time `^^` operator parsing    | None       |
| Phase 2 | `TypeMeta` data structure definition  | Phase 1    |
| Phase 3 | Compile-time metadata generation      | Phase 2    |
| Phase 4 | Runtime reflection support (optional) | Phase 3    |
| Phase 5 | Compile-time predicate integration    | Phase 3    |

### Dependencies

```
Phase 1 (Parsing)
    ↓
Phase 2 (Data structure)
    ↓
Phase 3 (Compile-time Metadata)
    ↓
    ├────────────┐
    ↓            ↓
Phase 4        Phase 5
(Runtime      (Compile-time
reflection)   predicates)
```

### Risks

- **Parsing conflict**: `^^` may conflict with existing syntax (analyzed and found no conflict)
- **Performance impact**: Compile-time metadata generation may increase compilation time (treeshake
  optimization can mitigate)
- **Runtime overhead**: Enabling runtime reflection increases memory overhead (on-demand generation
  alleviates this)

## Open Questions

- [x] Scope of `^^`: Only applies to types and values, not expressions
- [x] Chained access: Supported, the metadata object returned by `^^T` can access properties
      normally
- [x] Pattern matching: Supported, `TypeMeta` is a regular record type and can be pattern matched
      normally
- [x] Comparison: Supported, metadata objects of the same type are equal
- [x] Memory overhead: On-demand generation + treeshake optimization

---

## Appendices

### Appendix A: Design Decision Record

| Decision                      | Decision                                             | Date       | Recorder |
| ----------------------------- | ---------------------------------------------------- | ---------- | -------- |
| Scope of `^^`                 | Only applies to types and values, not expressions    | 2026-06-16 | Chen Xu  |
| Chained access                | Supported                                            | 2026-06-16 | Chen Xu  |
| Pattern matching              | Supported                                            | 2026-06-16 | Chen Xu  |
| Comparison                    | Supported, same-type metadata is equal               | 2026-06-16 | Chen Xu  |
| Memory overhead               | On-demand generation + treeshake                     | 2026-06-16 | Chen Xu  |
| Generic interaction           | Both `^^List` and `^^List(Int)` supported            | 2026-06-16 | Chen Xu  |
| Refinement expression storage | Available at compile-time, erased to None at runtime | 2026-06-16 | Chen Xu  |

### Appendix B: Glossary

| Term            | Definition                                                                      |
| --------------- | ------------------------------------------------------------------------------- |
| Reflection      | Ability to access type metadata at runtime or compile-time                      |
| Metadata        | Information describing type structure (name, fields, parameters, etc.)          |
| RTTI            | Run-Time Type Information                                                       |
| Treeshake       | Compiler optimization that removes unused code                                  |
| Refinement type | Type with constraint conditions, e.g., `Positive: (x: Int) -> Type = { x > 0 }` |

---

## References

- [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md)
- [RFC-011: Generic Type System Design](../accepted/011-generic-type-system.md)
- [RFC-027: Compile-Time Predicates and Unified Static Verification](../accepted/027-compile-time-evaluation-types.md)
- [RFC-011a: Interface Implementation and Dynamic Dispatch](../accepted/011a-interface-implementation.md)
- [C++26 Reflection Proposal](https://wg21.link/P2996)

---

## Lifecycle and fate

```
┌─────────────┐
│   Draft     │  ← Current state
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Under Review │  ← Open for community discussion and feedback
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   Accepted  │    │  Rejected   │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   accepted/ │    │    rfc/     │
│ (official   │    │ (preserved  │
│  design)    │    │  in place)  │
└─────────────┘    └─────────────┘
```

---
title: "RFC-033: `^^` Reflection Operator"
status: "Draft"
author: "Chenxu"
created: "2026-06-16"
updated: "2026-06-16"
---

# RFC-033: `^^` Reflection Operator

> **References**:
>
> - [RFC-010: Unified Type Syntax - name: type = value Model](../accepted/010-unified-type-syntax.md)
> - [RFC-011: Generic System Design](../accepted/011-generic-type-system.md)
> - [RFC-027: Compile-Time Predicates and Unified Static Verification](../accepted/027-compile-time-evaluation-types.md)
> - [RFC-011a: Interface Implementation and Dynamic Dispatch](../draft/011a-interface-implementation.md)

## Summary

This RFC proposes the introduction of the `^^` operator as a reflection entry point, used to obtain metadata of types and values. `^^T` returns the static metadata object of type `T`, and `^^obj` returns the dynamic type metadata of value `obj`. Metadata objects are ordinary record types, containing information such as name, parameters, fields, and so on, and can be used at compile-time and runtime.

## Motivation

### Why is this feature needed?

1. **Serialization/Deserialization**: Requires access to a type's field information to automatically generate serialization code.
2. **Compile-time metaprogramming**: Requires access to type structures at compile-time to generate code or verify constraints.
3. **Runtime debugging/tools**: Requires printing type information at runtime to assist debugging.
4. **Runtime type checking**: Requires determining type relationships at runtime, e.g., "What type is obj?"

### Current Problems

Currently, YaoXiang has no reflection mechanism and cannot access a type's metadata at compile-time or runtime. If we use `.name`, `.fields` directly to access type metadata, it will conflict with user-defined fields:

```yaoxiang
Person: Type = { name: String, age: Int }

# If Person.name is the type metadata's name, or is it the field name?
# This leads to parsing difficulties and semantic confusion
```

A syntax that **does not intrude into the regular field namespace** is needed to access type metadata.

## Proposal

### Core Design

Introduce the `^^` operator as the reflection entry point, clearly distinguishing regular code from metadata queries.

**Two usages**:

1. **Static reflection (acts on types)**: `^^T` returns the static metadata object of type `T`.
2. **Dynamic reflection (acts on values)**: `^^obj` returns the dynamic type metadata of value `obj`.

**Metadata structure**:

```yaoxiang
TypeMeta: Type = {
    name: String,
    params: Array(ParamMeta),
    fields: Array(FieldMeta),
    return_type: Type,
    refinement: Option(Expr)  # Compile-time: Some(Expr), Runtime: None
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

# Dynamic reflection (requires runtime reflection to be enabled)
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

# Reflect on a concrete instantiated type
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

#### Used in Compile-Time Predicates

```yaoxiang
# Check whether a type has fields
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
# Compile-time pure function: generate a JSON string
to_json: (T: Type) -> ((obj: T) -> String) = {
    meta = ^^T
    parts: Array(String) = []
    for field in meta.fields {
        # Compile-time generation of field access code
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
| No reflection mechanism | `^^T` retrieves type metadata |
| No reflection mechanism | `^^obj` retrieves the dynamic type metadata of a value |

## Detailed Design

### Impact on the Type System

- **New types**: `TypeMeta`, `ParamMeta`, `FieldMeta`
- **Universe level**: The type returned by `^^T` is one level higher than `T`
- **Interaction with generics**: Both `^^List` and `^^List(Int)` are supported
- **Interaction with functions**: `^^add` returns the function's metadata (including parameters and return type)
- **Interaction with refinement types**: `^^Positive` returns the refinement type's metadata (including the refinement expression)

### Runtime Behavior

**Compile-time reflection**:
- `^^T` is fully evaluated at compile-time, and the result is inlined as a constant.
- The refinement expression is available at compile-time.

**Runtime reflection**:
- Disabled by default, zero overhead.
- Enabled via the `--enable-runtime-reflection` compilation option.
- When enabled, `^^obj` returns the dynamic type metadata.
- The refinement expression is erased to `None` at runtime.

**On-demand generation + treeshake**:
- Metadata is generated only for types that actually use `^^`.
- Types that are not referenced do not generate metadata (treeshake).

### Compiler Changes

1. **Lexer**: Recognize `^^` as a single token.
2. **Parser**: Add the `^^` prefix expression rule.
3. **Type system**: Add type definitions for `TypeMeta`, `ParamMeta`, `FieldMeta`.
4. **Type checker**: Generate metadata instances for each type.
5. **Compile-time evaluator**: Support the compile-time evaluation of `^^T`.
6. **Runtime (optional)**: Generate RTTI for types that are reflected.

### Backward Compatibility

- ✅ No impact on existing syntax: `^^` is a new operator that does not conflict with existing syntax.
- ✅ No impact on existing types: All types automatically support `^^`.
- ✅ No impact on existing functions: Functions can use `^^` but are not required to.
- ✅ No impact on compile-time predicates: `^^T` behaves the same as ordinary content in predicates.
- ✅ No runtime impact: Runtime reflection is disabled by default, zero overhead.

## Trade-offs

### Advantages

- **Uniformity**: Functions, generics, and refinement types are handled uniformly.
- **Zero overhead**: Compile-time reflection is fully erased; runtime reflection is optional.
- **Integration with existing systems**: Seamless integration with compile-time predicates (RFC-027).
- **Conciseness**: `^^` is a pure symbol that does not conflict with user-defined identifiers.
- **On-demand generation**: With treeshake optimization, unused types incur zero overhead.

### Disadvantages

- **Learning curve**: Requires understanding the semantics of `^^` and the metadata structure.
- **Runtime overhead**: Enabling runtime reflection increases memory overhead (one pointer per instance).
- **Implementation complexity**: Requires modifications to multiple compiler components.

## Alternatives

| Alternative | Why not chosen |
|------|--------------|
| `reflect(T)` function | Introduces an extra identifier into the scope, which may be shadowed by users. |
| `type_info(T)` function | Same as above. |
| Single `^` operator | May conflict with bitwise operations; C++26 chose `^^` precisely because of this conflict. |
| `@@`, `##`, etc. | No precedent; less self-explanatory than `^^`. |

## Implementation Phases

| Phase | Content | Dependencies |
|------|------|------|
| Phase 1 | Parse the compile-time `^^` operator | None |
| Phase 2 | Define the `TypeMeta` data structure | Phase 1 |
| Phase 3 | Compile-time metadata generation | Phase 2 |
| Phase 4 | Runtime reflection support (optional) | Phase 3 |
| Phase 5 | Compile-time predicate integration | Phase 3 |

### Dependency Graph

```
Phase 1 (Parsing)
    ↓
Phase 2 (Data structure)
    ↓
Phase 3 (Compile-time metadata)
    ↓
    ├────────────┐
    ↓            ↓
Phase 4        Phase 5
(Runtime reflection)  (Compile-time predicates)
```

### Risks

- **Parsing conflicts**: `^^` may conflict with existing syntax (analysis shows no conflict).
- **Performance impact**: Compile-time metadata generation may increase compilation time (mitigated by treeshake).
- **Runtime overhead**: Enabling runtime reflection increases memory overhead (mitigated by on-demand generation).

## Open Questions

- [x] Scope of `^^`: Acts only on types and values, not on expressions.
- [x] Chained access: Supported; the metadata object returned by `^^T` can have its attributes accessed normally.
- [x] Pattern matching: Supported; `TypeMeta` is a regular record type and can be pattern-matched normally.
- [x] Comparison: Supported; metadata objects of the same type are equal.
- [x] Memory overhead: Mitigated by on-demand generation and treeshake optimization.

---

## Appendix

### Appendix A: Design Decision Records

| Decision | Decision | Date | Recorder |
|------|------|------|--------|
| Scope of `^^` | Acts only on types and values, not on expressions | 2026-06-16 | Chenxu |
| Chained access | Supported | 2026-06-16 | Chenxu |
| Pattern matching | Supported | 2026-06-16 | Chenxu |
| Comparison | Supported; metadata of the same type is equal | 2026-06-16 | Chenxu |
| Memory overhead | On-demand generation + treeshake | 2026-06-16 | Chenxu |
| Generic interaction | Both `^^List` and `^^List(Int)` are supported | 2026-06-16 | Chenxu |
| Refinement expression storage | Available at compile-time, erased to None at runtime | 2026-06-16 | Chenxu |

### Appendix B: Glossary

| Term | Definition |
|------|------|
| Reflection | The ability to access type metadata at runtime or compile-time. |
| Metadata | Information describing a type's structure (name, fields, parameters, etc.). |
| RTTI | Run-Time Type Information. |
| treeshake | A compiler optimization that removes unused code. |
| Refinement type | A type with constraints, e.g., `Positive: (x: Int) -> Type = { x > 0 }`. |

---

## References

- [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md)
- [RFC-011: Generic System Design](../accepted/011-generic-type-system.md)
- [RFC-027: Compile-Time Predicates and Unified Static Verification](../accepted/027-compile-time-evaluation-types.md)
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
│  Under      │  ← Open community discussion and feedback
│  Review     │
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
│ (Official)  │    │ (Retained)  │
└─────────────┘    └─────────────┘
```
---
title: 'RFC-033: `^^` Reflection Operator'
status: 'Draft'
author: 'Chenxu'
created: '2026-06-16'
updated: '2026-10-02'
issue: '#136'
---

# RFC-033: `^^` Reflection Operator

> **References**:
>
> - [RFC-010: Unified Type Syntax - name: type = value Model](../accepted/010-unified-type-syntax.md)
> - [RFC-011: Generic Type System Design](../accepted/011-generic-type-system.md)
> - [RFC-027: Compile-Time Predicates and Unified Static Verification](../accepted/027-compile-time-evaluation-types.md)
> - [RFC-011a: Interface Implementation and Dynamic Dispatch](../accepted/011a-interface-implementation.md)

## Summary

This RFC proposes introducing the `^^` operator as a reflection entry point, used to obtain metadata
of types and values. `^^T` returns a static metadata object of type `T`, and `^^obj` returns the
dynamic type metadata of value `obj`. The metadata object is a regular record type, containing
information such as name, parameters, fields, etc., and can be used at both compile time and
runtime.

## Motivation

### Why is this feature needed?

1. **Serialization/Deserialization**: need to access type field information to automatically
   generate serialization code
2. **Compile-time metaprogramming**: need to access type structures at compile time to generate code
   or verify constraints
3. **Runtime debugging/tools**: need to print type information at runtime to aid debugging
4. **Runtime type checking**: need to determine type relationships at runtime, such as "what type is
   `obj`?"

### Current Problem

Currently, YaoXiang has no reflection mechanism, and cannot access type metadata at compile time or
runtime. If `.name` and `.fields` are directly used to access type metadata, it will conflict with
user-defined fields:

```yaoxiang
Person: Type = { name: String, age: Int }

# Is Person.name the name of the type metadata, or the field name?
# This will cause parsing difficulties and semantic confusion
```

A syntax that **does not intrude into the regular field namespace** is needed to access type
metadata.

## Proposal

### Core Design

Introduce the `^^` operator as a reflection entry point, clearly distinguishing regular code from
metadata queries.

**Two usages**:

1. **Static reflection (on types)**: `^^T` returns the static metadata object of type `T`
2. **Dynamic reflection (on values)**: `^^obj` returns the dynamic type metadata of value `obj`

**Metadata structure**:

```yaoxiang
TypeMeta: Type = {
    name: String,
    params: Array(ParamMeta),
    fields: Array(FieldMeta),
    return_type: Type,
    refinement: Option(Expr)  # Compile-time: Some(Expr); runtime: None
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

**Universe levels**: if `T: Type_n`, then `^^T: Type_{n+1}`, conforming to the standard universe
lifting rule in type theory.

**Precedence**: `^^` is a unary prefix operator with the highest precedence. `^^T.name` is
equivalent to `(^^T).name`.

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

# Reflect the generic type itself
meta = ^^List
print(meta.name)           # "List"
print(meta.params)         # [{ name: "T", type: Type }]

# Reflect a specific instantiated type
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

#### Use in Compile-Time Predicates

```yaoxiang
# Check if type has fields
HasFields: (T: Type) -> Type = { ^^T.fields.len > 0 }

# Check field type
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
        # Compile-time generate field access code
        parts.push("\"${field.name}\": ${obj.${field.name}}")
    }
    return "{" + parts.join(", ") + "}"
}

# Usage
point_to_json = to_json(Point)
print(point_to_json(Point(1.0, 2.0)))  # '{"x": 1.0, "y": 2.0}'
```

### Syntax Changes

| Before                  | After                                           |
| ----------------------- | ----------------------------------------------- |
| No reflection mechanism | `^^T` to obtain type metadata                   |
| No reflection mechanism | `^^obj` to obtain value's dynamic type metadata |

## Detailed Design

### Type System Impact

- **New types**: `TypeMeta`, `ParamMeta`, `FieldMeta`
- **Universe levels**: the type returned by `^^T` is one level higher than `T`
- **Generic interaction**: both `^^List` and `^^List(Int)` are supported
- **Function interaction**: `^^add` returns the function's metadata (including parameters and return
  type)
- **Refinement type interaction**: `^^Positive` returns the refinement type's metadata (including
  the refinement expression)

### Runtime Behavior

**Compile-time reflection**:

- `^^T` is fully evaluated at compile time, the result is inlined as a constant
- Refinement expressions are available at compile time

**Runtime reflection**:

- Disabled by default, zero overhead
- Enabled via the `--enable-runtime-reflection` compiler option
- When enabled, `^^obj` returns dynamic type metadata
- Refinement expressions are erased to `None` at runtime

**On-demand generation + treeshake**:

- Only types that actually use `^^` have metadata generated
- Unreferenced types do not have metadata generated (treeshake)

### Compiler Changes

1. **Lexer**: recognize `^^` as a single token
2. **Parser**: add `^^` prefix expression rules
3. **Type system**: add `TypeMeta`, `ParamMeta`, `FieldMeta` type definitions
4. **Type checker**: generate metadata instances for each type
5. **Compile-time evaluator**: support compile-time evaluation of `^^T`
6. **Runtime (optional)**: generate RTTI for reflected types

### Backward Compatibility

- ✅ No impact on existing syntax: `^^` is a new operator and does not conflict with existing syntax
- ✅ No impact on existing types: all types automatically support `^^`
- ✅ No impact on existing functions: functions can use `^^` but are not required to
- ✅ No impact on compile-time predicates: `^^T` in predicates works the same as regular content
- ✅ No impact on runtime: runtime reflection is disabled by default, zero overhead

## Trade-offs

### Advantages

- **Unification**: functions, generics, and refinement types are handled uniformly
- **Zero overhead**: compile-time reflection is fully erased, runtime reflection is optional
- **Integration with existing systems**: seamless integration with compile-time predicates (RFC-027)
- **Conciseness**: `^^` is a pure symbol and does not conflict with user-defined identifiers
- **On-demand generation**: treeshake optimization, zero overhead for unused types

### Disadvantages

- **Learning curve**: need to understand the semantics of `^^` and the metadata structure
- **Runtime overhead**: enabling runtime reflection increases memory overhead (one pointer per
  instance)
- **Implementation complexity**: requires modifying multiple compiler components

## Alternatives

| Approach                | Why not chosen                                                                       |
| ----------------------- | ------------------------------------------------------------------------------------ |
| `reflect(T)` function   | Would introduce additional identifiers into the scope, may be shadowed by users      |
| `type_info(T)` function | Same as above                                                                        |
| Single `^` operator     | May conflict with bitwise operations, and C++26 chose `^^` precisely for this reason |
| `@@`, `##` etc. symbols | No precedent, not as easy to explain as `^^`                                         |

## Implementation Phases

| Phase   | Content                               | Dependencies |
| ------- | ------------------------------------- | ------------ |
| Phase 1 | Compile-time `^^` operator parsing    | None         |
| Phase 2 | `TypeMeta` data structure definition  | Phase 1      |
| Phase 3 | Compile-time metadata generation      | Phase 2      |
| Phase 4 | Runtime reflection support (optional) | Phase 3      |
| Phase 5 | Compile-time predicate integration    | Phase 3      |

### Dependencies

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
(Runtime reflection)  (Compile-time predicates)
```

### Risks

- **Parsing conflicts**: `^^` may conflict with existing syntax (analysis shows no conflicts)
- **Performance impact**: compile-time metadata generation may increase compilation time (can be
  optimized via treeshake)
- **Runtime overhead**: enabling runtime reflection increases memory overhead (mitigated by
  on-demand generation)

## Open Questions

- [x] `^^` scope: only applies to types and values, not expressions
- [x] Chained access: supported, the metadata object returned by `^^T` can have its properties
      accessed normally
- [x] Pattern matching: supported, `TypeMeta` is a regular record type that can be pattern matched
      normally
- [x] Comparison: supported, metadata objects of the same type are equal
- [x] Memory overhead: on-demand generation + treeshake optimization

---

## Appendix

### Appendix A: Design Decision Record

| Decision                      | Resolution                                           | Date       | Recorder |
| ----------------------------- | ---------------------------------------------------- | ---------- | -------- |
| `^^` scope                    | Only applies to types and values, not expressions    | 2026-06-16 | Chenxu   |
| Chained access                | Supported                                            | 2026-06-16 | Chenxu   |
| Pattern matching              | Supported                                            | 2026-06-16 | Chenxu   |
| Comparison                    | Supported, same-type metadata are equal              | 2026-06-16 | Chenxu   |
| Memory overhead               | On-demand generation + treeshake                     | 2026-06-16 | Chenxu   |
| Generic interaction           | Both `^^List` and `^^List(Int)` are supported        | 2026-06-16 | Chenxu   |
| Refinement expression storage | Available at compile time, erased to None at runtime | 2026-06-16 | Chenxu   |

### Appendix B: Glossary

| Term            | Definition                                                                       |
| --------------- | -------------------------------------------------------------------------------- |
| Reflection      | The ability to access type metadata at runtime or compile time                   |
| Metadata        | Information describing type structure (name, fields, parameters, etc.)           |
| RTTI            | Run-Time Type Information                                                        |
| treeshake       | Compiler optimization that removes unused code                                   |
| Refinement type | A type with constraint conditions, e.g. `Positive: (x: Int) -> Type = { x > 0 }` |

---

## References

- [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md)
- [RFC-011: Generic Type System Design](../accepted/011-generic-type-system.md)
- [RFC-027: Compile-Time Predicates and Unified Static Verification](../accepted/027-compile-time-evaluation-types.md)
- [RFC-011a: Interface Implementation and Dynamic Dispatch](../accepted/011a-interface-implementation.md)
- [C++26 Reflection Proposal](https://wg21.link/P2996)

---

## Lifecycle and Disposition

```
┌─────────────┐
│   Draft     │  ← Current state
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Under review│  ← Open community discussion and feedback
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
│ (Official   │    │ (Keep in    │
│  design)    │    │  place)     │
└─────────────┘    └─────────────┘
```

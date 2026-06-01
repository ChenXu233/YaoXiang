# Concurrency Model Specification

> **Status: Possibly deprecated, unimplemented.** This document does not represent the current design. For the actual ownership model, see [Type System Specification](./type-system.md) Chapter 11 (Type Properties) and Chapter 12 (Borrow Tokens). The `@block`/`@eager` annotations, `Mutex[T]`/`Atomic[T]`/`RwLock[T]`, etc. are early drafts and do not reflect the current design direction.

This file is on hold, to be rewritten or deleted once the concurrency model design is finalized.

**Function Annotations**:

| Annotation | Position | Behavior |
This file is on hold, to be rewritten or deleted once the concurrency model design is finalized.

---

## Ownership Model (Summary)

For the complete model, see [Type System Specification](./type-system.md) Chapters 11 and 12. Only the safety model summary is retained here:

```yaoxiang
// Move (default, zero-copy)
p2 = p

// ref (compiler automatically chooses Rc or Arc)
shared = ref p

// clone (explicit deep copy)
p2 = p.clone()

// unsafe (raw pointer)
unsafe {
    ptr: *Point = &p
    (*ptr).x = 0.0
}
```

| Semantic | Description | Overhead |
|------|------|------|
| Move | Default, ownership transfer | Zero |
| `&T` / `&mut T` | Borrow token, zero-size compile-time permission proof | Zero |
| `ref` | Compiler automatically chooses Rc (single-task) / Arc (cross-task) | On-demand |

## spawn Syntax (Excerpt)

For the complete syntax, see [Syntax Specification](./syntax.md) Section 3.10.

```yaoxiang
// spawn block
(result_a, result_b) = spawn {
    parse(fetch("url1")),
    parse(fetch("url2"))
}

// spawn loop
results = spawn for item in items {
    process(item)
}
```
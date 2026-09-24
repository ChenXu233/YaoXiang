---
title: 'RFC-023: Closure Capture Model'
status: 'Deprecated'
author: 'Chenxu'
created: '2026-05-29'
updated: '2026-06-16'
---

> **Deprecation Reason**: 2026-06-16 language design decision — Lambdas/function values do not
> implicitly capture outer variables; instead, explicit parameter passing is used. `spawn { }`
> executes in the same frame and does not involve closure capture. The capture analysis system of
> this RFC has been completely removed (~850 lines of code). The context-dependent solution =
> closures only accept parameters + currying is fixed at the creation point (SPEC §12.3 / RFC-009
> §2.3). See [RFC-009 Design Decision](../accepted/009-ownership-model.md#design-decision-records)
> for details.

# RFC-023: Closure Capture Model

> **References**:
>
> - [RFC-007: Function Syntax Unification](../accepted/007-function-syntax-unification.md)
> - [RFC-009: Ownership Model v9](../accepted/009-ownership-model.md)
> - [RFC-011: Generic Type System Design](../accepted/011-generic-type-system.md) — Section 2.4:
>   Dup/Clone Builtin Marker Trait

## Summary

This RFC defines the **Closure Capture Model** of the YaoXiang language. The compiler automatically
analyzes the external variables referenced by the closure body, and automatically selects the
capture method based on the variable type (Dup/non-Dup) and whether the closure escapes — Dup types
are copied directly, non-Dup non-escaping closures are borrowed, and non-Dup escaping closures are
moved. Users need zero annotations, sharing the same set of rules as the automatic borrow selection
for function calls.

## Motivation

### Why is it needed?

Currently, closure capture is an **empty implementation** — the `env` field of the `MakeClosure`
instruction is always empty, and lambdas cannot reference any external variables. The borrow token
system requires closures to be able to capture `&T` tokens (zero-cost copy), which is a core use
case.

### Current Problem

```yaoxiang
# 这种代码目前无法编译——lambda 不能引用 threshold
filter_by: (items: List(Point), threshold: &Float) -> List(Point) = {
    items.filter(|p| p.x > threshold)  # ❌ threshold 无法捕获
}
```

## Proposal

### Core Design

Closure capture is automatically determined by the compiler. The rules are **exactly the same** as
the automatic borrow selection for function calls:

```
Variable Type    Closure Escapes?    Capture Method
─────────────────────────────────────────
Dup              Any                 Copy (bitwise copy or zero-cost)
Non-Dup          No                  Auto-borrow (&T or &mut T)
Non-Dup          Yes                 Move (ownership transfer)
```

**Escape Determination**:

```
spawn { || ... }           → Escapes
return || ...              → Escapes
let x = || ... ;  x stored in field → Escapes
items.filter(|p| ...)      → Does not escape (sync higher-order function call)
||.method()                → Does not escape (called immediately)
```

Conservative principle: When uncertain, treat as escaping.

### Examples

```yaoxiang
# 1. Dup 令牌——直接复制（零成本）
filter_by: (items: List(Point), threshold: &Float) -> List(Point) = {
    # threshold: &Float → Dup → 编译器复制令牌进闭包
    # 零大小令牌，零运行时开销
    items.filter(|p| p.x > threshold)
}

# 2. 非 Dup + 不逃逸——自动借用
process: (buf: Buffer) -> Void = {
    # buf 不 Dup，filter 不逃逸 → 自动创建 &Buffer 令牌
    transform(|b| b.read())
    # 闭包返回后令牌释放，buf 恢复可用
}

# 3. 闭包逃逸——Move
spawn_worker: (data: Data) -> Void = {
    # data 不 Dup，spawn → 逃逸 → Move
    spawn { use(data) }
}

# 4. 混合捕获
complex: (items: List(Point), config: &Config, buf: Buffer) -> List(Point) = {
    # config: &Config → Dup → 复制令牌
    # buf: Buffer → 不 Dup，不逃逸 → &mut Buffer 借用
    items.filter(|p| {
        let threshold = config.get_threshold()
        buf.update(p)
        p.x > threshold
    })
}

# 5. 借用冲突检测
bad: (buf: Buffer) -> Void = {
    closure = |b| b.write()
    buf.read()  # ❌ buf 已被闭包借用，此处冲突
}
```

### Syntax Changes

**Zero syntax changes**. The capture method is automatically determined by the compiler, and users
do not need to annotate.

## Detailed Design

### Type System Impact

Lambda type signatures remain unchanged: `(params) -> Return`. Captured variables are not reflected
in the type signature and are handled by the compiler during the IR generation phase.

### Compiler Changes

| Component          | Change                                              | Description |
| ------------------ | --------------------------------------------------- | ----------- |
| `capture.rs` (new) | Capture analysis + escape analysis + mode selection | ~150 lines  |
| `expressions.rs`   | Lambda type inference calls capture analysis        | ~10 lines   |
| `ir_gen.rs`        | MakeClosure env filling; ZST skipping               | ~80 lines   |
| `ir.rs`            | MakeClosure env type may need adjustment            | ~5 lines    |

**Capture Analysis Flow**:

```
1. Traverse lambda body AST
2. Collect all Expr::Var(name) references
3. Filter: keep only variables from outside the closure scope
4. Classify: Read (read-only) / Write (read-write) / Move (transferred)
5. Check type property: is Dup
6. Determine escape: how the closure is used
7. Select capture mode:
   Dup → Copy
   Non-Dup + not escaping + Read → Borrow (&T)
   Non-Dup + not escaping + Write → BorrowMut (&mut T)
   Non-Dup + escaping → Move
```

**IR Generation**:

```rust
// 当前（空）
Instruction::MakeClosure { dst, func, env: Vec::new() }

// 改为
Instruction::MakeClosure { dst, func, env: captured_env }

// captured_env 的生成逻辑：
for captured in captures {
    match captured.mode {
        Copy if is_zst(captured.ty) => {
            // 零大小类型——不生成任何指令
            // 闭包体直接引用外层（编译期消除）
        }
        Copy => {
            // 生成 Move dst, src（Dup 类型的浅复制）
        }
        Borrow => {
            // 生成 Borrow dst, src（创建 ReadToken）
        }
        BorrowMut => {
            // 生成 Borrow dst, src（创建 WriteToken）
        }
        Move => {
            // 生成 Move dst, src（所有权转移）
        }
    }
}
```

### Runtime Behavior

The capture method does not affect runtime performance:

- **Dup + ZST** (e.g., `&T` token) → Zero instructions, the closure body directly references the
  outer variable
- **Dup + non-ZST** (e.g., Int) → One register copy
- **Borrow/BorrowMut** → Create token (compile-time concept, zero overhead)
- **Move** → Same cost as a normal Move

### Backward Compatibility

Fully compatible. Currently, all lambdas cannot capture external variables. This RFC only adds
expressive capability and does not break any existing code.

## Trade-offs

### Advantages

1. **Zero annotations**: Users do not need to write any capture annotations
2. **Unified with function calls**: Capture rules = function call auto-borrow rules
3. **Zero cost**: Capture of Dup tokens is completely eliminated at compile-time
4. **Safe**: Escape analysis prevents use-after-free

### Disadvantages

1. **Conservative escape analysis**: When uncertain, treated as escaping, may unnecessarily Move
2. **Implicit**: The capture method is not reflected in the code; debugging requires examining
   compiler output

## Alternative Approaches

| Approach                               | Why not chosen                                            |
| -------------------------------------- | --------------------------------------------------------- |
| Rust-style explicit `move` keyword     | Introduces new syntax, increases user cognitive burden    |
| All Move                               | Cannot express zero-cost token borrowing                  |
| All borrow                             | Closure escaping leads to dangling references             |
| User manually annotates capture method | Violates the "fully automatic compiler" design philosophy |

## Implementation Strategy

### Phases

1. **Phase 1**: Capture analysis (only identify external variable references, not distinguishing
   capture methods)
2. **Phase 2**: Escape analysis + mode selection
3. **Phase 3**: IR generation + ZST optimization
4. **Phase 4**: Borrow conflict detection integration

### Dependencies

- Depends on RFC-011 (Generic type system, Section 2.4 Dup/Clone trait) — needs Dup trait to
  determine if a variable is copyable
- Depends on RFC-009 v9 (Borrow tokens) — Borrow/BorrowMut capture modes need token types
- After RFC-023 and this RFC are implemented, the borrow token system (RFC-009 v9 implementation)
  can begin

### Risks

- Escape analysis may be overly conservative, leading to unnecessary Moves; can be optimized later
- Capture analysis of generic closures may require additional handling

## Design Decision Records

| Decision                                | Determination                  | Reason                                          | Date       |
| --------------------------------------- | ------------------------------ | ----------------------------------------------- | ---------- |
| Capture method selection                | Fully automatic                | Unified with function call rules                | 2026-05-29 |
| Escape analysis                         | Conservative principle         | When uncertain, treat as escaping, safety first | 2026-05-29 |
| ZST optimization                        | Skip during IR generation      | Simpler than subsequent optimization passes     | 2026-05-29 |
| Capture not reflected in type signature | Handled internally by compiler | Keep lambda types concise                       | 2026-05-29 |

## References

### YaoXiang Official Documentation

- [RFC-007: Function Syntax Unification](../accepted/007-function-syntax-unification.md)
- [RFC-009: Ownership Model v9](../accepted/009-ownership-model.md)
- [RFC-011: Generic Type System Design](../accepted/011-generic-type-system.md) — Section 2.4:
  Dup/Clone Builtin Marker trait

### External References

- [Rust Closure Capture Rules](https://doc.rust-lang.org/reference/types/closure.html#capture-modes)
- [Swift Closure Capture Semantics](https://docs.swift.org/swift-book/documentation/the-swift-programming-language/closures/#Capturing-Values)

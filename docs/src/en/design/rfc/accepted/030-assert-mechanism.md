---
title: 'RFC-030: assert Mechanism'
status: 'Accepted'
author: 'Chenxu'
created: '2026-06-15'
updated: '2026-07-14'
decision:
  'assert and Assert are two sides of the same coin; dispatch routes automatically. All 6 Phases
  implemented (#157–#162 closed). The std.assert module is registered uniformly (#169 closed); the
  assert native function and the Assert/IsTrue type family share the same path.'
issue: '#97'
issues_impl:
  - '#155'
  - '#157'
  - '#158'
  - '#159'
  - '#160'
  - '#161'
  - '#162'
  - '#169'
---

# RFC-030: assert Mechanism

## Summary

This RFC introduces the `assert` mechanism for YaoXiang, used for testing, precondition checks, and
runtime panics. `assert` and the compile-time refinement type `Assert(C)` (see RFC-011 §4.3) are
**two sides of the same refinement primitive**—dispatch automatically routes to compile-time proof
or runtime check based on whether the predicate's free variables are reachable at compile time.
`assert(false, "msg")` is equivalent to `raise`; no separate `throw`/`raise` keyword is required.

## Motivation

### Why is this feature needed?

Currently, YaoXiang's E2E tests can only simulate assertions through `if` + `io.println` + `return`:

```yaoxiang
val = some_func()
if val != 42 {
    io.println("FAIL: expected 42")
    return
}
```

This style has three problems:

1. **Excessive boilerplate**: every assertion requires 4 lines, bloating test files
2. **Weak error messages**: manual string concatenation, lacking source location
3. **Not composable**: cannot batch-register assertions, cannot be passed as arguments to a test
   framework

### Current Problems

- No unified assertion mechanism
- Test code is cluttered with the `if` + print + `return` pattern
- The bytecode layer already has a `Throw` instruction, but it is not exposed at the language level
- RFC-011 defines the compile-time `Assert(C)` conditional type, but runtime `assert()` has not yet
  been implemented

### Design Principles

`assert` is the only user-space panic mechanism in YaoXiang. `assert(false, "msg")` is equivalent to
`raise`; no separate `throw`/`raise` keyword is required. The `assert` function itself is the best
encapsulation of `if raise`.

**No new keywords, no new syntax. Everything is a function call.**

## Option A: Native Function

Implement `assert` as a native function, with no new keywords introduced.

```yaoxiang
use std.assert.assert

main: () -> Void = {
    assert(1 + 1 == 2, "math is broken")
    assert(get_name() == "YaoXiang", "name mismatch")
}
```

### Overload Signatures

`assert` has two overloads:

```
// 核心签名：assert 是 Assert 的值宇宙引入子
assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))
//                                       ^^^^^^^^^^^^^^^^^^^^^^^^
//                                       返回精化类型，不是 ()
//
// IsTrue: Bool -> Type 是真值到类型的桥：
//   IsTrue(true)  = Void   (⊤，程序继续)
//   IsTrue(false) = Never  (⊥，发散/编译错误)
```

The actual behavior of `assert` is determined by dispatch routing:

- All free variables compile-time known → **CompileTime**: the compiler evaluates `cond`; `true` →
  erased to `Void`, `false` → compile error (Never cannot be inhabited)
- Runtime free variables present → **Runtime**: insert a check, inject refinement facts into the
  flow-sensitive assumption set Γ

The optional message `?msg` and Result overloads (see below) are preserved as runtime raise
payloads.

#### Overload 1: Conditional Assertion `(Bool, ?String | Error)`

`Bool` + optional message. The message can be a `String` or an `Error` value:

```yaoxiang
assert(1 + 1 == 2)                    // 无消息，默认 panic 信息
assert(1 + 1 == 2, "math is broken")   // 字符串消息
assert(x > 0, my_error)                // 直接抛 Error 值
```

`assert(false, "msg")` is YaoXiang's `raise`/`throw` equivalent—no separate keyword is needed.

#### Overload 2: Result Assertion `(Result)`

A single `Result` parameter, automatically checking if it is `Err`:

### Advantages

- **Zero syntax changes**: pure function, no new keywords needed
- **Zero new concepts**: reuses the existing native function registration mechanism
- **High extensibility**: function overloading naturally supports multiple signatures
- **Self-documenting**: the `std.assert` namespace itself is documentation

### Disadvantages

- None. When assert's type signature is correct, the compiler can infer dead code via function
  reachability analysis. No additional pass is needed.

### Runtime Behavior

1. Evaluate the first argument `condition: Bool`
2. If `true`, return `Unit`
3. If `false`, trigger a runtime panic:
   - Output the `message` content (if any)
   - Output the call stack (in debug mode)
   - Terminate the current execution

#### Failure Behavior of Each Overload

| Signature                  | Failure Behavior                 |
| -------------------------- | -------------------------------- |
| `assert(false)`            | Default panic message            |
| `assert(false, "msg")`     | Output string message then panic |
| `assert(false, error_val)` | Throw Error value                |
| `assert(Err(x))`           | Extract Err content and panic    |

### Relationship with Compile-Time Assert

`assert` and `Assert` are **two sides of the same refinement primitive**—automatically selected by
the dispatch routing pipeline based on whether the predicate's free variables are reachable at
compile time:

| Condition                             | Routing                      | Behavior                                                                      |
| ------------------------------------- | ---------------------------- | ----------------------------------------------------------------------------- |
| All free variables compile-time known | CompileTime → proof pipeline | Proved → erased; Disproved → compile error; Unknown → requires proof          |
| Runtime free variables exist          | Runtime → insert check       | Bool check + inject refinement facts into the flow-sensitive assumption set Γ |

```yaoxiang
use std.assert

# 编译期已知（泛型参数）—— 走 CompileTime，零运行时开销
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    length: assert.Assert(N > 0),   # N 是泛型参数，编译期求值
}

# 运行时值 —— 走 Runtime，插入 Bool 检查
x = read_int()
assert.assert(x > 0, "expected positive")  # 运行时 check
```

> **2026-07-12 Unified Approach**: The earlier "fully independent" conclusion is now superseded.
> `assert()` is the value-level introducer of `Assert`; dispatch routes automatically.

### Compiler Changes

**No changes needed to parser, AST, typecheck, or IR gen.**

Only native function registration needs to be added under `src/std/`:

1. Add `src/std/assert.rs`
2. Register `std.assert.assert` and `std.assert.Assert` (the latter is a compile-time conditional
   type)
3. Internally call the existing `BytecodeInstr::Throw` instruction

### Advantages

- **Zero syntax changes**: pure function, no new keywords needed
- **Zero new concepts**: reuses the existing native function registration mechanism
- **High extensibility**: function signatures can be extended to variants like `assert_eq` (future)
- **Self-documenting**: the `std.assert` namespace itself is documentation

### Disadvantages

- ~~Compile-time opaque: unlike Option B (keyword), cannot perform compile-time dead code
  elimination~~ → **No longer holds under the unified approach**. CompileTime-mode `assert` goes
  through the proof pipeline; compile-time-known `cond` → erased or compile error (`assert(false)` →
  Never → dead code).
- The call stack is only available in debug mode

## Option B: Built-in Keyword (Superseded by Unified Approach)

> Deprecated. The opposition between Options A and B is dissolved by the dispatch routing
> pipeline—`assert` is the value-level introducer of `Assert`; compile-time known goes through the
> proof pipeline (zero runtime overhead), runtime goes through check. No need to choose between
> "function" and "keyword". The following is for historical reference.

```yaoxiang
assert(1 + 1 == 2, "math is broken")
```

### Type Signature

No independent type signature—the keyword is handled by the parser.

### Runtime Behavior

Same as Option A.

### Compiler Changes

Need to modify parser, AST, typecheck, and IR gen:

1. parser: add `Expr::Assert` variant
2. AST: add `Expr::Assert` node
3. typecheck: validate argument types
4. IR gen: emit `BytecodeInstr::Throw`

### Advantages

- Source location is known at compile time (does not depend on debug info)
- Compile-time constant folding is possible: `assert(true)` → no-op, `assert(false)` → compile error

### Disadvantages

| Disadvantage                            | Impact                                                   |
| --------------------------------------- | -------------------------------------------------------- |
| Requires parser changes                 | Introduces new syntax nodes, increasing maintenance cost |
| Keyword is not extensible               | Variants like `assert_eq` still require functions        |
| Compile-time advantages are theoretical | See analysis below                                       |

### Comparison

| Dimension           | Option A (Function)  | Option B (Keyword)                |
| ------------------- | -------------------- | --------------------------------- |
| Implementation cost | ~20 lines            | parser + AST + typecheck + IR gen |
| Syntax change       | None                 | New keyword                       |
| Extensibility       | Function overloading | Requires companion macros         |
| Source location     | debug info           | Compile-time known                |
| Constant folding    | Requires a pass      | Compile-time known                |
| Runtime overhead    | Function call        | Minimal                           |

### Practical Constraints of Compile-Time Analysis

Option B's core advantage—compile-time analysis—requires a **constant folding pass** to take effect.
That is, the compiler must evaluate `false` in `assert(false)` at compile time in order to recognize
it as dead code.

YaoXiang currently has no constant folding pass. Even with Option B, common patterns like
`assert(x > 0)` still cannot be analyzed at compile time. Only literals like `assert(true)` /
`assert(false)` can be analyzed.

Therefore, Option B's compile-time advantages **are theoretical at the current stage, not
practical**.

---

## Open Questions

- [x] ~~Choose Option A or Option B?~~ → **Unified approach: `assert` is the value-level introducer
      of `Assert`**. The opposition between Options A and B is dissolved by the dispatch routing
      pipeline—compile-time known goes through the proof pipeline, runtime goes through check. No
      "either/or" is needed.
- [x] ~~Does `assert` need to support the simplified form `assert(cond)` without `message`?~~ →
      **Supported. `assert(cond, ?msg)`, message is optional.**
- [x] ~~Are variants like `assert_eq`, `assert_ne` needed?~~ → **Not needed. YAGNI. Wait until the
      test framework matures.**
- [x] ~~Does panic output include source location?~~ → Option A depends on debug info (call stack).
- [x] ~~assert / Assert unification~~ → **Determined**. Unified approach:
      `assert: (Bool) -> Assert(IsTrue(cond))`, two sides of the same coin, dispatch routes
      automatically. The `Never` type (⊥) is built in as the return type of `assert(false)`.

### 2026-07-05: Option A Selected (Superseded by Unified Approach)

Option A's 20-line implementation wins on the value-vs-cost tradeoff. After the unified approach was
determined on 2026-07-12, the opposition between Options A and B is dissolved by the dispatch
routing pipeline—`assert` is the value-level introducer of `Assert`; no longer an "either/or"
between "function" and "keyword".

### 2026-07-12: Unified Approach Determined (Supersedes the 2026-07-11 "Fully Independent" Conclusion)

**Conclusion**: `assert` and `Assert` are not two independent mechanisms.
`assert: (Bool) -> Assert(IsTrue(cond))`—dispatch routes automatically:

- Compile-time known → enters the proof pipeline (Proved erased / Disproved error / Unknown requires
  proof)
- Runtime input → insert check + inject Γ assumption

**Module structure**: `std.assert` uniformly hosts runtime assertions (`assert`) and compile-time
refinement types (`Assert`, `IsTrue`). No longer "implemented separately", but two sides of the same
primitive.

### 2026-07-11: assert Overload Design

**Question**: Why does `assert` need two overloads instead of a unified `(Bool, ?String)`?

**Answer**:

Runtime `assert()` is the only user-space panic mechanism in YaoXiang. `assert(false, "msg")` is
equivalent to `raise`/`throw` in other languages. Therefore, it needs to cover three scenarios:

1. Condition + simple message: `assert(cond, "msg")`
2. Condition + custom Error: `assert(cond, my_error)`
3. Result check: `assert(result)` — the most concise `if is_err { panic }`

The rationale for the Result overload is: this is the shortest path for error propagation—"Result
should be Ok, otherwise die". There is no need to first call `.is_ok()` and then handle the error
separately.

## Appendix B: Design Decision Record

| Decision                                             | Decision                                                                                                                     | Date       | Recorder |
| ---------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- | ---------- | -------- |
| Choose Option A or Option B                          | **Unified approach**: dispatch routing pipeline dissolves A/B opposition; `assert` is the value-level introducer of `Assert` | 2026-07-12 | Chenxu   |
| Whether message is optional                          | **Yes**: `assert(cond, ?msg)`, String or Error                                                                               | 2026-07-11 | Chenxu   |
| Whether variants like `assert_eq` are needed         | **Not needed**. YAGNI; wait until the test framework matures                                                                 | 2026-07-11 | Chenxu   |
| Whether a separate `raise`/`throw` keyword is needed | **Not needed**. `assert(false, msg)` is equivalent to `raise`                                                                | 2026-07-11 | Chenxu   |
| Relationship between `assert` and `Assert`           | **Two sides of the same coin**. `assert: (Bool) -> Assert(IsTrue(cond))`, dispatch routes automatically                      | 2026-07-12 | Chenxu   |

## References

- [RFC-007: Unified Function Definition Syntax](007-function-syntax-unification.md) — the
  `name: type = value` model
- [RFC-010: Unified Type Syntax](010-unified-type-syntax.md) — foundations of the type system
- [RFC-011: Generic Type System Design §4.3](../accepted/011-generic-type-system.md) — compile-time
  verification and the `Assert(C)` conditional type
- [RFC-026: FFI Core Mechanism](026-ffi-core-mechanism.md) — native function registration mechanism
- [RFC-027: Compile-Time Predicates and Unified Static Verification](../accepted/027-compile-time-evaluation-types.md)
  — the compile-time evaluation system

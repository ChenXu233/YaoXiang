---
title: 'std.assert'
description: 'Assertion'
---

# std.assert

Assertion module. The most commonly used tool in tests and examples.

```yaoxiang
use std.assert
```

## Function Overview

<!-- stdlib:table:assert start -->

| Function | Signature                             |
| -------- | ------------------------------------- |
| `assert` | `(cond: Bool, ?msg: String) -> Never` |

<!-- stdlib:table:assert end -->

## Functions

### assert

<!-- stdlib:sig:assert.assert start -->

```yaoxiang
assert: (cond: Bool, ?msg: String) -> Never
```

<!-- stdlib:sig:assert.assert end -->

Assert that `cond` is true.

- `cond` —— the boolean expression to be evaluated
- `msg` —— optional message, `?` indicates it can be omitted; printed with the diagnostic when the
  condition is not met

Return: returns `Void` when the condition holds, without interrupting execution (the implementation
at `src/std/assert.rs:82-83` returns `RuntimeValue::Void` in the passing branch). **The declared
return type, however, is `Never`** — the signature `(cond: Bool, ?msg: String) -> Never` comes from
`src/std/assert.rs:24-29`; the two do not contradict: use sites of `Void` (such as
`x = assert(true)`) already accept `Never <: T` (principle of explosion, see
[Type System §2.2](../language-spec/type-system.md)).

Error: throws `E6005` (assertion failed) when the condition is false; the program exits with a
non-zero code.

```yaoxiang
use std.assert

main: () -> Void = {
    assert(1 > 0)
    assert(1 > 0, "这个字面断言必然成立")
}
```

Assertions are the **primary means of evaluation for test corpora** — `src/std/tests/*.yx` and
`tests/yaoxiang/**` both work by having `assert` failures cause the process to error out:

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    list.len([1, 2, 3]) == 3
    assert(list.len([1, 2, 3]) == 3, "len == 3")
}
```

## Type Families (Undocumented)

In addition to one native export, `std.assert` also registers two **type families**
(`TypeFamilyExport`), which belong to the type universe rather than the value universe, and are
therefore **not counted in the "Function Overview" table above**:

| Name     | Definition                                             | Source                    |
| -------- | ------------------------------------------------------ | ------------------------- |
| `IsTrue` | `(b: Bool) -> Type`, `true => Void` / `false => Never` | `src/std/assert.rs:34-44` |
| `Assert` | `(cond: Bool) -> Type`, i.e. `IsTrue(cond)`            | `src/std/assert.rs:45-49` |

There is also `EffectSpec::new("assert", [GammaAssume { predicate_arg: 0 }], true)`
(`src/std/assert.rs:53-59`), used for injecting into the flow-sensitive assumption set `Γ`.

> **Currently you cannot write these two names in type position**: `Assert(true)` reports
> `E0010 Expected a type, found BoolLiteral(true)`, and `assert.Assert(true)` reports
> `E0012 Invalid syntax: declaration requires an initializer`. In other words, **only the runtime
> `assert` is available; compile-time refinement primitives are not yet wired up**. For the
> spec-level notation, see [Standard Library Specification §1.5](../language-spec/stdlib.md).

## Related

- [Test Specification](../../dev/test-specification.md) — corpus organization and evaluation
  conventions
- [Error Code Reference](../error-code/) — `E6005` assertion failed

---
title: 'std.assert'
description: 'Assertion'
---

# std.assert

The assertion module. The most commonly used tool in tests and examples.

```yaoxiang
use std.assert
```

## Function Overview

<!-- stdlib:table:assert start -->

| Function | Signature                            |
| -------- | ------------------------------------ |
| `assert` | `(cond: Bool, ?msg: String) -> Void` |

<!-- stdlib:table:assert end -->

## Functions

### assert

<!-- stdlib:sig:assert.assert start -->

```yaoxiang
assert: (cond: Bool, ?msg: String) -> Void
```

<!-- stdlib:sig:assert.assert end -->

Asserts that `cond` is true.

- `cond` — The boolean expression to check
- `msg` — Optional message; `?` means it can be omitted; emitted alongside the diagnostic when the
  condition is not satisfied

Returns: Returns `Void` when the condition holds, without interrupting execution. Errors: Throws
`E6005` (assertion failed) when the condition is false; the program exits with a non-zero code.

```yaoxiang
use std.assert

main: () -> Void = {
    assert(1 > 0)
    assert(1 > 0, "This literal assertion must hold")
}
```

Assertions are **the primary means of judgment in test corpora** — both `src/std/tests/*.yx` and
`tests/yaoxiang/**` work in a way that an `assert` failure causes a process error:

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    list.len([1, 2, 3]) == 3
    assert(list.len([1, 2, 3]) == 3, "len == 3")
}
```

## Related

- [Test Specification](../dev/test-specification.md) — Corpus organization and judgment conventions
- [Error Code Reference](../error-code/) — `E6005` Assertion Failed

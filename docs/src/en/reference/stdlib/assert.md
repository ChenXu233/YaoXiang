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

- `cond` — The boolean expression to evaluate
- `msg` — Optional message, `?` indicates it can be omitted; output along with diagnostics when the
  condition is not met

Returns: returns `Void` when the condition holds, without interrupting execution. Error: throws
`E6005` (Assertion Failed) when the condition is false, and the program exits with a non-zero code.

```yaoxiang
use std.assert

main: () -> Void = {
    assert(1 > 0)
    assert(1 > 0, "this literal assertion must hold")
}
```

Assertions are the **primary means of evaluation in test corpora** — `src/std/tests/*.yx` and
`tests/yaoxiang/**` all work in a way where an `assert` failure causes the process to error:

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    list.len([1, 2, 3]) == 3
    assert(list.len([1, 2, 3]) == 3, "len == 3")
}
```

## Related

- [Test Specification](../dev/test-specification.md) — Corpus organization and evaluation
  conventions
- [Error Code Reference](../error-code/) — `E6005` Assertion Failed

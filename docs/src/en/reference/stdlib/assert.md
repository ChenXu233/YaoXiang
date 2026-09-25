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

Asserts that `cond` is true.

- `cond` — the boolean expression to evaluate
- `msg` — optional message, `?` means omittable; output together with diagnostics when the condition
  does not hold

Returns: returns `Void` when the condition holds, without interrupting execution. Error: throws
`E6005` (assertion failed) when the condition is false, and the program exits with a non-zero code.

```yaoxiang
use std.assert

main: () -> Void = {
    assert(1 > 0)
    assert(1 > 0, "this literal assertion must hold")
}
```

Assertions are **the primary means of judgment for test corpora** — both `src/std/tests/*.yx` and
`tests/yaoxiang/**` work in such a way that an `assert` failure causes the process to report an
error:

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    list.len([1, 2, 3]) == 3
    assert(list.len([1, 2, 3]) == 3, "len == 3")
}
```

## Related

- [Test Specification](../../dev/test-specification.md) — corpus organization and judgment
  conventions
- [Error Code Reference](../error-code/) — `E6005` assertion failed

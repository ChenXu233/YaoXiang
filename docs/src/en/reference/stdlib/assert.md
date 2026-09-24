---
title: 'std.assert'
description: 'Assertions'
---

# std.assert

Assertion module. The most commonly used tool in tests and examples.

```yaoxiang
use std.assert
```

## Function Overview

<!-- stdlib:table:assert start -->

| Function | Signature                           |
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
- `msg` — Optional message, `?` indicates it can be omitted; output together with diagnostics when the condition doesn't hold

Returns: Returns `Void` when the condition holds, without interrupting execution. Error: Throws `E6005` (assertion failed) when the condition is false, and the program exits with a non-zero code.

```yaoxiang
use std.assert

main: () -> Void = {
    assert(1 > 0)
    assert(1 > 0, "This literal assertion is always true")
}
```

Assertions are the **primary judgment mechanism in test suites** — both `src/std/tests/*.yx` and `tests/yaoxiang/**` operate on the principle that an `assert` failure causes the process to report an error:

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    list.len([1, 2, 3]) == 3
    assert(list.len([1, 2, 3]) == 3, "len == 3")
}
```

## See Also

- [Test Specification](../../dev/test-specification.md) — Test organization and judgment conventions
- [Error Code Reference](../error-code/) — `E6005` assertion failed
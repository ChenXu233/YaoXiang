---
title: 'std.test'
description: 'Test assertion library (value semantics)'
---

# std.test

YaoXiang's test library is written in YaoXiang itself—the first dogfooding module (RFC-036 §3).

**Value-semantics contract**: An assertion failure is represented as `Err(diagnostic message)` and
**does not abort the process**. Every assertion function's return type is `Result(Void, String)`, so
multiple assertions can run in one program, with the caller deciding whether they pass or fail.
Process-level abort semantics remain in [`std.assert`](assert) (the runtime guard) and do not enter
the test assertion path.

```yaoxiang
use std.test
```

> **This page is hand-written**. This module is implemented entirely in YaoXiang source code
> (`src/std/test.yx`) and is outside the generation scope of `StdModule::exports()`, so it has no
> `<!-- stdlib:... -->` generated-section marker. The signatures in the table below are taken
> verbatim from the file's top-level binding declarations.

## Exports

| Export             | Signature                                                              | Description                               |
| ------------------ | ---------------------------------------------------------------------- | ----------------------------------------- |
| `assert_eq`        | `(a: Any, b: Any) -> Result(Void, String)`                             | Equality assertion                        |
| `assert_ne`        | `(a: Any, b: Any) -> Result(Void, String)`                             | Inequality assertion                      |
| `assert_true`      | `(cond: Bool) -> Result(Void, String)`                                 | Asserts that a condition is true          |
| `assert_false`     | `(cond: Bool) -> Result(Void, String)`                                 | Asserts that a condition is false         |
| `assert_not`       | `(cond: Bool) -> Result(Void, String)`                                 | Same body as `assert_false`               |
| `assert_err`       | `(T: Type, E: Type)(r: Result(T, E)) -> Result(Void, String)`          | The `Result` is `Err`                     |
| `assert_err_code`  | `(T: Type)(r: Result(T, Error), want: String) -> Result(Void, String)` | Error-code equality assertion             |
| `assert_approx_eq` | `(a: Float, b: Float, eps: Float) -> Result(Void, String)`             | Floating-point approximate equality       |
| `suite`            | `(tests: Vec((String, () -> Result(Void, String)))) -> Void`           | Runs each test and aggregates the results |

## Assertion Functions

The first five have the same shape: `Ok(void)` indicates a pass, `Err(message)` indicates a failure,
and execution is **not interrupted**.

| Function       | Failure message                      |
| -------------- | ------------------------------------ |
| `assert_eq`    | `Expected {b}, got {a}`              |
| `assert_ne`    | `Expected not equal to {b}, got {a}` |
| `assert_true`  | `Expected true, got {cond}`          |
| `assert_false` | `Expected false, got {cond}`         |
| `assert_not`   | Same as `assert_false`               |

`assert_not` and `assert_false` currently use the same implementation—the unary form `!assert` will
be replaced only once `not` syntax becomes available (RFC-036 §8.1).

```yaoxiang
use std.assert
use std.result
use std.test

main: () -> Void = {
    assert(result.is_ok(test.assert_eq(1, 1)))
    assert(result.is_err(test.assert_eq(1, 2)))
    assert(result.unwrap_err(test.assert_eq(1, 2)) == "Expected 2, got 1")

    assert(result.is_ok(test.assert_ne(1, 2)))
    assert(result.is_err(test.assert_ne(1, 1)))

    assert(result.is_ok(test.assert_true(true)))
    assert(result.is_err(test.assert_true(false)))
    assert(result.is_ok(test.assert_false(false)))
    assert(result.is_ok(test.assert_not(false)))

    // A failure does not interrupt execution: the following lines still run
    _ = test.assert_eq(1, 2)
    println("still running")
}
```

The argument types of `assert_eq` / `assert_ne` are `Any`—comparison uses value-level `==`, and the
element type must support equality comparison (primitive types support it natively).

## assert_err

Checks whether a `Result` is the `Err` variant. `T` / `E` are inferred from the arguments at the
call site and do not need to be supplied explicitly.

```yaoxiang
use std.assert
use std.result
use std.string
use std.test

main: () -> Void = {
    // Parsing failure → Err
    assert(result.is_ok(test.assert_err(string.parse_int("abc"))))

    // Parsing succeeds → Ok; the assertion fails
    assert(result.is_err(test.assert_err(string.parse_int("12"))))
}
```

## assert_err_code

Further asserts that the **error code** carried by `Err` equals the expected value. `E` is fixed to
`Error`—an error code exists only on the standard library's `Error` carrier (the `code` / `message`
fields; see [`std.result`](result)).

The failure message is `Expected code {want}, got {c}`.

```yaoxiang
use std.assert
use std.result
use std.string
use std.test

main: () -> Void = {
    // The failure code for string.parse_int is E6010
    assert(result.is_ok(test.assert_err_code(string.parse_int("abc"), "E6010")))

    r = test.assert_err_code(string.parse_int("abc"), "E9999")
    assert(result.is_err(r))
    assert(result.unwrap_err(r) == "Expected code E9999, got E6010")
}
```

## assert_approx_eq

Uses `|a - b| <= eps` to determine floating-point approximate equality.

- `eps` — **provided explicitly by the caller**; the tolerance is part of the test contract, with no
  hidden default value
- `eps < 0` — always `Err` (a contract that can never pass should not remain undetected until the
  assertion fails)
- `NaN` — always `Err` (subtracting any value from `NaN` yields `NaN`, so the comparison is always
  false)

The failure message is `Expected {b} (±{eps}), got {a} (diff {d})`.

```yaoxiang
use std.assert
use std.result
use std.test

main: () -> Void = {
    assert(result.is_ok(test.assert_approx_eq(1.0, 1.05, 0.1)))
    assert(result.is_err(test.assert_approx_eq(1.0, 1.5, 0.1)))
    assert(result.is_err(test.assert_approx_eq(1.0, 1.0, -1.0)))
}
```

## suite

Calls each test function in turn and collects the per-test verdicts (RFC-036 §7 value-semantics
model).

```
suite: (tests: Vec((String, () -> Result(Void, String)))) -> Void
```

- `tests` — a list of `(name, test function)` pairs; test functions take **no arguments** and return
  `Result(Void, String)`

Behavior:

- **All tests return `Ok`, with no output**, and the file exit code is `0`
- If any test returns `Err`, aggregate the failure details (name + diagnostic) and then abort, with
  a nonzero file exit code—the abort here is the **runtime guard of the test binary** (via
  `assert.assert(failed == 0, …)`), not the assertion path
- After one test fails, the remaining tests **continue to run as usual**

> Top-level test function names cannot yet be referenced as values (an IR-level limitation), so use
> a closure when adding an entry to the suite: `("name", () => test_fn())`.

```yaoxiang
use std.test

t_add: () -> Result(Void, String) = () => test.assert_eq(1 + 1, 2)

main: () -> Void = {
    test.suite([
        ("add", () => t_add()),
        ("truth", () => test.assert_true(true)),
    ])
    println("all passed")
}
```

When an item in the suite fails, `E6005` is reported, with a message of the following form:

```
1 of 1 test(s) failed
  [FAIL] bad: Expected 2, got 1
```

—**Failure example, illustrating behavior only** (deliberately not in a `yaoxiang` fence because the
example gate only runs executable blocks):

```
test.suite([("bad", () => test.assert_eq(1, 2))])
// → E6005: assertion failed: 1 of 1 test(s) failed
//              [FAIL] bad: Expected 2, got 1
```

## Related

- [`std.assert`](assert) — a process-level runtime guard (`E6005`), complementary to this module's
  value semantics
- [`std.result`](result) — a family of `Result` unwrapping utilities
- [Error code reference](../error-code/) — error codes carried by `Err`, such as `E6010` / `E6011`

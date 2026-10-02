---
title: 'std.test'
description: 'Test assertion library (value semantics)'
---

# std.test

Test assertion library. YaoXiang's testing library is written in YaoXiang itself — this is the first
dogfooding module (RFC-036 §3).

**Value semantics contract**: assertion failures are expressed as `Err(diagnostic message)`,
**without aborting the process**. The return type of every assertion function is
`Result(Void, String)`, so multiple assertions can run consecutively in one program, with the caller
deciding pass or fail. Process-level abort semantics are retained in [`std.assert`](./assert)
(runtime guard), and do not enter the test assertion path.

```yaoxiang
use std.test
```

> **This page is hand-written.** This module is implemented purely in YaoXiang source
> (`src/std/test.yx`), outside the generation range of `StdModule::exports()`, so it has no
> `<!-- stdlib:... -->` generated-region marker. The signatures in the table below are taken
> verbatim from the top-level binding declarations of that file.

## Exports

| Export             | Signature                                                              | Description                          |
| ------------------ | ---------------------------------------------------------------------- | ------------------------------------ |
| `assert_eq`        | `(a: Any, b: Any) -> Result(Void, String)`                             | Equality assertion                   |
| `assert_ne`        | `(a: Any, b: Any) -> Result(Void, String)`                             | Inequality assertion                 |
| `assert_true`      | `(cond: Bool) -> Result(Void, String)`                                 | Truth assertion                      |
| `assert_false`     | `(cond: Bool) -> Result(Void, String)`                                 | Falsity assertion                    |
| `assert_not`       | `(cond: Bool) -> Result(Void, String)`                                 | Same as `assert_false`               |
| `assert_err`       | `(T: Type, E: Type)(r: Result(T, E)) -> Result(Void, String)`          | `Result` is `Err`                    |
| `assert_err_code`  | `(T: Type)(r: Result(T, Error), want: String) -> Result(Void, String)` | Error code equality assertion        |
| `assert_approx_eq` | `(a: Float, b: Float, eps: Float) -> Result(Void, String)`             | Float approximate equality assertion |
| `suite`            | `(tests: Vec((String, () -> Result(Void, String)))) -> Void`           | Run each and summarize               |

## Assertion Functions

The first five share a uniform shape: `Ok(void)` indicates pass, `Err(message)` indicates failure,
**without interrupting execution**.

| Function       | Failure message                      |
| -------------- | ------------------------------------ |
| `assert_eq`    | `Expected {b}, got {a}`              |
| `assert_ne`    | `Expected not equal to {b}, got {a}` |
| `assert_true`  | `Expected true, got {cond}`          |
| `assert_false` | `Expected false, got {cond}`         |
| `assert_not`   | Same as `assert_false`               |

`assert_not` and `assert_false` are currently the same implementation — the unary form `!assert`
will be replaced once the `not` syntax lands (RFC-036 §8.1).

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

    // Failure does not interrupt: the following lines still run
    _ = test.assert_eq(1, 2)
    println("still running")
}
```

The argument types of `assert_eq` / `assert_ne` are `Any` — comparison uses value-level `==`, and
the element types must support equality comparison (built-in types support this natively).

## assert_err

Asserts that a `Result` is the `Err` variant. `T` / `E` are inferred from the call-site arguments;
they need not be given explicitly.

```yaoxiang
use std.assert
use std.result
use std.string
use std.test

main: () -> Void = {
    // Parse failed → Err
    assert(result.is_ok(test.assert_err(string.parse_int("abc"))))

    // Parse succeeded → Ok, assertion fails
    assert(result.is_err(test.assert_err(string.parse_int("12"))))
}
```

## assert_err_code

Further asserts that the **error code** in the `Err` carrier equals the expected value. `E` is
pinned to `Error` — codes exist only on the standard library's `Error` carrier (`code` / `message`
fields, see [`std.result`](./result)).

The failure message is `Expected code {want}, got {c}`.

```yaoxiang
use std.assert
use std.result
use std.string
use std.test

main: () -> Void = {
    // string.parse_int failure code is E6010
    assert(result.is_ok(test.assert_err_code(string.parse_int("abc"), "E6010")))

    r = test.assert_err_code(string.parse_int("abc"), "E9999")
    assert(result.is_err(r))
    assert(result.unwrap_err(r) == "Expected code E9999, got E6010")
}
```

## assert_approx_eq

Determines float approximate equality by `|a - b| <= eps`.

- `eps` — **explicitly given by the caller**; tolerance is part of the test contract, no hidden
  default value
- `eps < 0` — always `Err` (a contract that can never pass should not be discovered only at
  assertion failure)
- `NaN` — always `Err` (`NaN` minus any value is `NaN`, comparison is always false)

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

Calls test functions one by one and collects per-test verdicts (RFC-036 §7 value model).

```
suite: (tests: Vec((String, () -> Result(Void, String)))) -> Void
```

- `tests` — list of `(name, test function)` pairs; the test function is **zero-arg** and returns
  `Result(Void, String)`

Behavior:

- **All Ok silently**, file exit code `0`
- Any `Err`: aggregate failure details (name + diagnosis) then abort, file exit code is non-`0` —
  the abort here is the **test binary's runtime guard** (via `assert.assert(failed == 0, …)`), not
  the assertion path
- After a test fails, the remaining tests **continue to run**

> Top-level test function names cannot currently be referenced as values (an IR-level limitation);
> use closures when enqueueing: `("name", () => test_fn())`.

```yaoxiang
use std.assert
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

When there are failed items in the suite, `E6005` is reported, with a message like:

```
1 of 1 test(s) failed
  [FAIL] bad: Expected 2, got 1
```

—**failure example, only illustrates behavior** (intentionally not using `yaoxiang` fences; the
example gatekeeper only runs runnable blocks):

```
test.suite([("bad", () => test.assert_eq(1, 2))])
// → E6005: assertion failed: 1 of 1 test(s) failed
//              [FAIL] bad: Expected 2, got 1
```

## Related

- [`std.assert`](./assert) — process-level runtime guard (`E6005`), complements the value semantics
  of this module
- [`std.result`](./result) — `Result` unpacking tool family
- [Error code reference](../error-code/) — error codes on `Err` carriers, such as `E6010` / `E6011`

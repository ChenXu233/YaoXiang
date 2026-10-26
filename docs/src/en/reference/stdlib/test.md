---
title: 'std.test'
description: 'Test assertion library (value semantics)'
---

# std.test

Test assertion library. YaoXiang's test library is written in YaoXiang itself — this is the first
dogfooding module (RFC-036 §3).

**Value-semantics contract**: assertion failures are expressed as `Err(diagnostic info)`, **without
aborting the process**. Each assertion function returns `Result(Void, String)`, so multiple
assertions can be chained in one program and the caller decides the outcome. Process-level abort
semantics are reserved for [`std.assert`](assert) (runtime guard) and do not enter the test
assertion path.

```yaoxiang
use std.test
```

> **This page is hand-written.** This module is implemented in pure YaoXiang source
> (`src/std/test.yx`) and is not within the generation scope of `StdModule::exports()`, so it has no
> `<!-- stdlib:... -->` generated-region marker. The signatures in the table below are taken
> verbatim from the top-level binding declarations of that file.

## Exports

| Export             | Signature                                                              | Description                   |
| ------------------ | ---------------------------------------------------------------------- | ----------------------------- |
| `assert_eq`        | `(a: Any, b: Any) -> Result(Void, String)`                             | Equality assertion            |
| `assert_ne`        | `(a: Any, b: Any) -> Result(Void, String)`                             | Inequality assertion          |
| `assert_true`      | `(cond: Bool) -> Result(Void, String)`                                 | True assertion                |
| `assert_false`     | `(cond: Bool) -> Result(Void, String)`                                 | False assertion               |
| `assert_not`       | `(cond: Bool) -> Result(Void, String)`                                 | Same body as `assert_false`   |
| `assert_err`       | `(T: Type, E: Type)(r: Result(T, E)) -> Result(Void, String)`          | `Result` is `Err`             |
| `assert_err_code`  | `(T: Type)(r: Result(T, Error), want: String) -> Result(Void, String)` | Error-code equality assertion |
| `assert_approx_eq` | `(a: Float, b: Float, eps: Float) -> Result(Void, String)`             | Float approximate equality    |
| `suite`            | `(tests: Vec((String, () -> Result(Void, String)))) -> Void`           | Run one by one and summarize  |

## Assertion Functions

The first five share a uniform shape: `Ok(void)` means pass, `Err(message)` means failure —
**execution is not interrupted**.

| Function       | Failure Message                      |
| -------------- | ------------------------------------ |
| `assert_eq`    | `Expected {b}, got {a}`              |
| `assert_ne`    | `Expected not equal to {b}, got {a}` |
| `assert_true`  | `Expected true, got {cond}`          |
| `assert_false` | `Expected false, got {cond}`         |
| `assert_not`   | Same as `assert_false`               |

`assert_not` and `assert_false` currently share the same implementation — the unary form `!assert`
will only be swapped in once the `not` syntax lands (RFC-036 §8.1).

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

    // Failure does not interrupt: the following lines still run as usual
    _ = test.assert_eq(1, 2)
    println("still running")
}
```

The argument types of `assert_eq` / `assert_ne` are `Any` — comparison goes through value-level
`==`, and the element types must support equality comparison (primitive types are supported
natively).

## assert_err

Asserts that a `Result` is the `Err` variant. `T` / `E` are inferred from the call-site arguments;
no explicit annotation is required.

```yaoxiang
use std.assert
use std.result
use std.string
use std.test

main: () -> Void = {
    // Parse failure → Err
    assert(result.is_ok(test.assert_err(string.parse_int("abc"))))

    // Parse success → Ok, assertion fails
    assert(result.is_err(test.assert_err(string.parse_int("12"))))
}
```

## assert_err_code

Further asserts that the **error code** carried by `Err` equals the expected value. `E` is pinned to
`Error` — codes only exist on the standard library's `Error` carrier (`code` / `message` fields, see
[`std.result`](result)).

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

Asserts float approximate equality via `|a - b| <= eps`.

- `eps` — **explicitly given by the caller**; tolerance is part of the test contract, no hidden
  default
- `eps < 0` — always `Err` (a contract that can never pass shouldn't be discovered only at assertion
  time)
- `NaN` — always `Err` (`NaN` minus anything is `NaN`, comparison is always false)

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

Calls test functions one by one and collects per-test verdicts (RFC-036 §7 value-based model).

```
suite: (tests: Vec((String, () -> Result(Void, String)))) -> Void
```

- `tests` — a list of `(name, test function)` pairs; the test function takes **zero parameters** and
  returns `Result(Void, String)`

Behavior:

- **All Ok silent**, file exit code `0`.
- Any `Err`: summarize the failure details (name + diagnostic) then abort, file exit code non-`0` —
  the abort here is a **runtime guard of the test binary** (via `assert.assert(failed == 0, …)`),
  not an assertion path.
- After one test fails, the remaining tests **still run as usual**.

> Top-level test function names cannot currently be referenced as values (IR-level limitation), so
> use a closure when entering the list: `("name", () => test_fn())`.

```yaoxiang
use std.test

t_add: () -> Result(Void, String) = () => test.assert_eq(1 + 1, 2)

main: () -> Void = {
    // Value-based verdict: calling it directly executes the assertion (Ok = pass)
    t_add()

    test.suite([
        ("add", () => t_add()),
        ("truth", () => test.assert_true(true)),
    ])
    println("all passed")
}
```

When the suite contains a failed item, it reports `E6005` with a message of the form:

```
1 of 1 test(s) failed
  [FAIL] bad: Expected 2, got 1
```

—— **failure example, illustrates behavior only** (deliberately not wrapped in a `yaoxiang` fence;
the example gate only runs executable blocks):

```
test.suite([("bad", () => test.assert_eq(1, 2))])
// → E6005: assertion failed: 1 of 1 test(s) failed
//              [FAIL] bad: Expected 2, got 1
```

## Related

- [`std.assert`](assert) — process-level runtime guard (`E6005`); complements this module's value
  semantics
- [`std.result`](result) — `Result` unwrap tool family
- [错误码参考](../error-code/) — error codes on the `Err` carrier such as `E6010` / `E6011`

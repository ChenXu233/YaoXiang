---
title: 'std.test'
description: 'Test assertion library (value semantics)'
---

# std.test

Test assertion library. The YaoXiang test library is written in YaoXiang itself — this is the first
dogfooding module (RFC-036 §3).

**Value-semantics contract**: assertion failures are expressed as `Err(diagnostic)`, **without
aborting the process**. Every assertion function returns `Result(Void, String)`, so you can run
multiple assertions in one program and let the caller decide pass/fail. Process-level abort
semantics remain in [`std.assert`](assert) (runtime guard) and are not part of the test assertion
path.

```yaoxiang
use std.test
```

> **This page is hand-written**. This module is implemented in pure YaoXiang source
> (`src/std/test.yx`), outside the generation scope of `StdModule::exports()`, so there is no
> `<!-- stdlib:... -->` generation region marker. The signatures in the table below are taken
> verbatim from the top-level binding declarations in that file.

## Exports

| Export             | Signature                                                              | Description                          |
| ------------------ | ---------------------------------------------------------------------- | ------------------------------------ |
| `assert_eq`        | `(a: Any, b: Any) -> Result(Void, String)`                             | Equality assertion                   |
| `assert_ne`        | `(a: Any, b: Any) -> Result(Void, String)`                             | Inequality assertion                 |
| `assert_true`      | `(cond: Bool) -> Result(Void, String)`                                 | Truthy assertion                     |
| `assert_false`     | `(cond: Bool) -> Result(Void, String)`                                 | Falsy assertion                      |
| `assert_not`       | `(cond: Bool) -> Result(Void, String)`                                 | Same body as `assert_false`          |
| `assert_err`       | `(T: Type, E: Type)(r: Result(T, E)) -> Result(Void, String)`          | `Result` is `Err`                    |
| `assert_err_code`  | `(T: Type)(r: Result(T, Error), want: String) -> Result(Void, String)` | Error code equality assertion        |
| `assert_approx_eq` | `(a: Float, b: Float, eps: Float) -> Result(Void, String)`             | Float approximate equality assertion |
| `suite`            | `(tests: Vec((String, () -> Result(Void, String)))) -> Void`           | Run each one and summarize           |

## Assertion functions

The first five share a consistent shape: `Ok(void)` indicates pass, `Err(message)` indicates
failure, **without interrupting execution**.

| Function       | Failure message                      |
| -------------- | ------------------------------------ |
| `assert_eq`    | `Expected {b}, got {a}`              |
| `assert_ne`    | `Expected not equal to {b}, got {a}` |
| `assert_true`  | `Expected true, got {cond}`          |
| `assert_false` | `Expected false, got {cond}`         |
| `assert_not`   | Same as `assert_false`               |

`assert_not` and `assert_false` currently share the same implementation — the unary form `!assert`
will be reworked once the `not` syntax lands (RFC-036 §8.1).

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

    // Failure does not interrupt: the following lines still execute
    _ = test.assert_eq(1, 2)
    println("still running")
}
```

The argument types of `assert_eq` / `assert_ne` are `Any` — comparison goes through value-level
`==`, and element types must support equality comparison (built-in types do so natively).

## assert_err

Check that a `Result` is the `Err` variant. `T` / `E` are inferred from the call site arguments, no
need to provide them explicitly.

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

Further assert that the **error code** of the `Err` carrier equals the expected value. `E` is fixed
to `Error` — the code only exists on the standard library's `Error` carrier (the `code` / `message`
fields, see [`std.result`](result)).

Failure message is `Expected code {want}, got {c}`.

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

Check float approximate equality as `|a - b| <= eps`.

- `eps` — **explicitly given by the caller**, the tolerance is part of the test contract, no hidden
  default value
- `eps < 0` — always `Err` (a contract that cannot pass should not be discovered only at assertion
  failure)
- `NaN` — always `Err` (`NaN` minus any value is `NaN`, comparison is always false)

Failure message is `Expected {b} (±{eps}), got {a} (diff {d})`.

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

Call test functions one by one and collect per-test judgments (RFC-036 §7 value-oriented model).

```
suite: (tests: Vec((String, () -> Result(Void, String)))) -> Void
```

- `tests` — a list of `(name, test function)` pairs; the test function takes **zero arguments** and
  returns `Result(Void, String)`

Behavior:

- **All Ok silent**, file exit code `0`
- Any `Err`: aggregate failure details (name + diagnosis) then abort, file exit code non-`0` — this
  abort is the **test binary's runtime guard** (goes through `assert.assert(failed == 0, …)`), not
  the assertion path
- After a test fails, the remaining tests **continue to run**

> Top-level test function names cannot yet be referenced as values (IR-level limitation), use
> closures when entering the list: `("name", () => test_fn())`.

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

When the suite has failed items, it reports `E6005`, with a message like:

```
1 of 1 test(s) failed
  [FAIL] bad: Expected 2, got 1
```

——**Failure example, illustrates behavior only** (deliberately not using the `yaoxiang` fence; the
example gate only runs executable blocks):

```
test.suite([("bad", () => test.assert_eq(1, 2))])
// → E6005: assertion failed: 1 of 1 test(s) failed
//              [FAIL] bad: Expected 2, got 1
```

## Related

- [`std.assert`](assert) — process-level runtime guard (`E6005`), complementary to this module's
  value semantics
- [`std.result`](result) — unpacking tool family for `Result`
- [Error code reference](../error-code/) — codes on `Err` carriers like `E6010` / `E6011`

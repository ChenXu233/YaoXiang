---
title: 'std.time'
description: 'Timestamps, formatting, and DateTime field access'
---

# std.time

Time module.

```yaoxiang
use std.time
```

> **Implementation gaps fixed (#338 / #340, 2026-09-19)**.
>
> `DateTime` is now an **alias for the timestamp** (i.e., `Int`) — at runtime it was always
> `RuntimeValue::Int`; previously it was just a name with no substance, which prevented the return
> value of `now()` from being passed into `format_time` and the accessors. The accessor export names
> also changed from `DateTime::year` to **`datetime_year`** (`::` is a lexically reserved token and
> cannot appear in field access position).
>
> Now the return values of `now()` / `parse_time()` can be used directly in arithmetic, formatting,
> and all accessors.

## Function Overview

<!-- stdlib:table:time start -->

| Function             | Signature                              |
| -------------------- | -------------------------------------- |
| `now`                | `() -> DateTime`                       |
| `timestamp`          | `() -> Int`                            |
| `timestamp_ms`       | `() -> Int`                            |
| `sleep`              | `(seconds: Float) -> Void`             |
| `format_time`        | `(dt: Int, fmt: String) -> String`     |
| `parse_time`         | `(fmt: String, s: String) -> DateTime` |
| `datetime_year`      | `(dt: Int) -> Int`                     |
| `datetime_month`     | `(dt: Int) -> Int`                     |
| `datetime_day`       | `(dt: Int) -> Int`                     |
| `datetime_hour`      | `(dt: Int) -> Int`                     |
| `datetime_minute`    | `(dt: Int) -> Int`                     |
| `datetime_second`    | `(dt: Int) -> Int`                     |
| `datetime_weekday`   | `(dt: Int) -> Int`                     |
| `datetime_to_string` | `(dt: Int) -> String`                  |

<!-- stdlib:table:time end -->

## Time Acquisition

### now

<!-- stdlib:sig:time.now start -->

```yaoxiang
now: () -> DateTime
```

<!-- stdlib:sig:time.now end -->

Returns the current time.

Returns: a `DateTime` value, printed in the form `DateTime(1789471990)`. **It is not `Int`**, so it
cannot directly participate in arithmetic or comparison, nor can it be passed as an `Int` parameter
to other functions (see [`format_time`](#format_time)).

```yaoxiang
use std.time

main: () -> Void = {
    t = time.now()
    println(t)          // DateTime(1789471990)
}
```

> When you need to participate in computations, use [`timestamp`](#timestamp) or
> [`timestamp_ms`](#timestamp_ms), which return `Int` directly.

### timestamp

<!-- stdlib:sig:time.timestamp start -->

```yaoxiang
timestamp: () -> Int
```

<!-- stdlib:sig:time.timestamp end -->

Returns the current Unix timestamp (**seconds**), which can directly participate in arithmetic and
comparison.

```yaoxiang
use std.assert
use std.time

main: () -> Void = {
    assert(time.timestamp() > 0)
}
```

### timestamp_ms

<!-- stdlib:sig:time.timestamp_ms start -->

```yaoxiang
timestamp_ms: () -> Int
```

<!-- stdlib:sig:time.timestamp_ms end -->

Returns the current Unix timestamp (**milliseconds**).

```yaoxiang
use std.assert
use std.time

main: () -> Void = {
    // millisecond precision is at least as fine as second precision
    assert(time.timestamp_ms() >= time.timestamp())
}
```

### sleep

<!-- stdlib:sig:time.sleep start -->

```yaoxiang
sleep: (seconds: Float) -> Void
```

<!-- stdlib:sig:time.sleep end -->

Sleeps for the specified number of **seconds** (may have a fractional part). The similarly-named
[`std.concurrent.sleep`](./concurrent#sleep) uses **milliseconds** — note the distinction.

- `seconds` — number of seconds to sleep; accepts `Int` (interpreted as seconds) or `Float`

Error: throws `E6007` if the argument is neither `Int` nor `Float`.

```yaoxiang
use std.time

main: () -> Void = {
    time.sleep(0.0)
}
```

> Not exported on the `wasm32` target.

## Formatting and Parsing

### format_time

<!-- stdlib:sig:time.format_time start -->

```yaoxiang
format_time: (dt: Int, fmt: String) -> String
```

<!-- stdlib:sig:time.format_time end -->

Formats a timestamp according to `fmt`. Supports `strftime`-style placeholders.

- `dt` — Unix timestamp (**seconds**), must be `Int`
- `fmt` — format string

> **Type note**: `dt` must be `Int`. Passing the return value of [`now`](#now) or
> [`parse_time`](#parse_time) will produce `E1002` (`expected type 'int64', found type 'DateTime'`),
> because they both return `DateTime`. Currently there is no way to convert `DateTime` to `Int`, so
> **in practice you can only pass an `Int` literal or the result of [`timestamp`](#timestamp)**.

Supported placeholders:

| Placeholder | Meaning                        | Example      |
| ----------- | ------------------------------ | ------------ |
| `%Y`        | Four-digit year                | `2024`       |
| `%m`        | Two-digit month                | `01`         |
| `%d`        | Two-digit day                  | `15`         |
| `%H`        | Two-digit hour (24-hour clock) | `10`         |
| `%M`        | Two-digit minute               | `30`         |
| `%S`        | Two-digit second               | `00`         |
| `%w`        | Weekday (0 = Sunday)           | `1`          |
| `%F`        | Equivalent to `%Y-%m-%d`       | `2024-01-15` |
| `%T`        | Equivalent to `%H:%M:%S`       | `10:30:00`   |

Broken down in **local time**. Unrecognized placeholders are preserved verbatim.

Error: throws `E6007` if `dt` is not `Int`, `fmt` is not `String`, or arguments are missing.

```yaoxiang
use std.assert
use std.string
use std.time

main: () -> Void = {
    // 0 = Unix epoch
    assert(time.format_time(0, "%Y") == "1970")
    assert(time.format_time(0, "%m") == "01")

    // the current timestamp (Int) can also be used directly
    s = time.format_time(time.timestamp(), "%Y")
    assert(string.len(s) == 4)
}
```

### parse_time

<!-- stdlib:sig:time.parse_time start -->

```yaoxiang
parse_time: (fmt: String, s: String) -> DateTime
```

<!-- stdlib:sig:time.parse_time end -->

Parses a time string into a time.

- `fmt` — **currently ignored** (see below)
- `s` — the string to parse

Returns: a `DateTime` value.

> **Two implementation limitations (#340)**:
>
> 1. The `fmt` parameter is **not used during parsing**. The function only recognizes ISO 8601 forms
>    `YYYY-MM-DDTHH:MM:SS` or `YYYY-MM-DD HH:MM:SS` (date and time separated by `T` or a space). Any
>    other form will fail regardless of what `fmt` says.
> 2. The returned `DateTime` **currently cannot be used further** — it is neither `Int` (so it
>    cannot be passed to `format_time` / `DateTime::*`), nor does it have any callable accessors
>    (see the next section).

Error: throws `E6007` if the format does not match.

```yaoxiang
use std.time

main: () -> Void = {
    // the parsing itself can succeed
    ts = time.parse_time("", "2024-01-15 10:30:00")
    println(ts)
}
```

## DateTime Field Access

`std.time` exports 8 date-component accessors (fixed in `#338`, with flat export names):

| Export name          | Signature             | Description            |
| -------------------- | --------------------- | ---------------------- |
| `datetime_year`      | `(dt: Int) -> Int`    | Four-digit year        |
| `datetime_month`     | `(dt: Int) -> Int`    | Month (1–12)           |
| `datetime_day`       | `(dt: Int) -> Int`    | Day (1–31)             |
| `datetime_hour`      | `(dt: Int) -> Int`    | Hour (0–23)            |
| `datetime_minute`    | `(dt: Int) -> Int`    | Minute (0–59)          |
| `datetime_second`    | `(dt: Int) -> Int`    | Second (0–59)          |
| `datetime_weekday`   | `(dt: Int) -> Int`    | Weekday (0 = Sunday)   |
| `datetime_to_string` | `(dt: Int) -> String` | ISO 8601 format string |

`DateTime` is an **alias for the timestamp** (i.e., `Int`) — the return values of `now()` /
`parse_time()` can be passed directly to these accessors, and can also be used directly in
[`format_time`](#format_time):

```yaoxiang
use std.assert
use std.time

main: () -> Void = {
    ts = time.parse_time("%Y-%m-%d", "2024-01-15")
    assert(time.datetime_year(ts) == 2024)
    assert(time.datetime_month(ts) == 1)
    assert(time.datetime_day(ts) == 15)

    // equivalent to format_time
    assert(time.format_time(ts, "%Y-%m-%d") == "2024-01-15")
}
```

## Related

- [`std.concurrent`](./concurrent) — millisecond-level sleep
- [Error code reference](../error-code/) — `E6007` generic runtime error

---
title: 'std.time'
description: 'Timestamps, formatting, and date component access'
---

# std.time

The time module.

```yaoxiang
use std.time
```

> **Implementation gaps have been fixed (#338 / #340, 2026-09-19)**.
>
> The return type of `now` / `parse_time` is **`Int`** (a Unix timestamp, in seconds). `DateTime` is
> not a built-in type name—older versions used a temporary compatibility alias with the same meaning
> (at runtime it was still `Int`, also fixed by #338), and **P10 S11 (issue #513) has migrated it
> out**: a bare `DateTime` annotation now raises an unknown-type error, and timestamps must always
> be written as `Int`. The accessor names are exported in a flat form, such as **`datetime_year`**
> (`::` is a lexically reserved token and cannot appear in a field-access position).
>
> The return values of `now()` / `parse_time()` can now be used directly for arithmetic, formatting,
> and all the accessors.
>
> **Time zones**: Both `format_time` and the eight `datetime_*` accessors use
> `timestamp_to_datetime` (`src/std/time.rs:123-178`), which performs **pure UTC arithmetic**—it
> does not read the local time-zone offset, and there is no distinction between `Local` and `Utc`.
> Therefore, the ISO 8601 string output by `datetime_to_string` is also in UTC.

## Function Overview

<!-- stdlib:table:time start -->

| Function             | Signature                          |
| -------------------- | ---------------------------------- |
| `now`                | `() -> Int`                        |
| `timestamp`          | `() -> Int`                        |
| `timestamp_ms`       | `() -> Int`                        |
| `sleep`              | `(seconds: Float) -> Void`         |
| `format_time`        | `(dt: Int, fmt: String) -> String` |
| `parse_time`         | `(fmt: String, s: String) -> Int`  |
| `datetime_year`      | `(dt: Int) -> Int`                 |
| `datetime_month`     | `(dt: Int) -> Int`                 |
| `datetime_day`       | `(dt: Int) -> Int`                 |
| `datetime_hour`      | `(dt: Int) -> Int`                 |
| `datetime_minute`    | `(dt: Int) -> Int`                 |
| `datetime_second`    | `(dt: Int) -> Int`                 |
| `datetime_weekday`   | `(dt: Int) -> Int`                 |
| `datetime_to_string` | `(dt: Int) -> String`              |

<!-- stdlib:table:time end -->## Time Retrieval

### now

<!-- stdlib:sig:time.now start -->

```yaoxiang
now: () -> Int
```

<!-- stdlib:sig:time.now end -->

Returns the current time.

Returns: an `Int` timestamp (**seconds**), that is, the number of seconds since the Unix epoch (the
implementation at `src/std/time.rs:238-244` returns `RuntimeValue::Int` directly)—the return value
**is** an `Int`: it can be used directly in arithmetic and comparisons, or passed directly to
[`format_time`](#format_time) and all the `datetime_*` accessors.

Use [`timestamp_ms`](#timestamp_ms) when millisecond precision is required.

```yaoxiang
use std.assert
use std.time

main: () -> Void = {
    t = time.now()
    assert(time.datetime_year(t) > 2020)     // Int, can be passed directly to the accessors
    assert(t > 0)                             // Int, can be used directly in comparisons
    println(time.format_time(t, "%Y-%m-%dT%H:%M:%SZ"))
}
```

> [`now`](#now) and [`timestamp`](#timestamp) return the same value (both are Unix timestamps in
> seconds, and both have type `Int`); the two names are retained only to improve readability at call
> sites.

### timestamp

<!-- stdlib:sig:time.timestamp start -->

```yaoxiang
timestamp: () -> Int
```

<!-- stdlib:sig:time.timestamp end -->

Returns the current Unix timestamp (**seconds**), which can be used directly in arithmetic and
comparisons.

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
    // Millisecond precision is finer than second precision
    assert(time.timestamp_ms() >= time.timestamp())
}
```

### sleep

<!-- stdlib:sig:time.sleep start -->

```yaoxiang
sleep: (seconds: Float) -> Void
```

<!-- stdlib:sig:time.sleep end -->

Sleeps for the specified **number of seconds** (fractional values are allowed). The similarly named
[`std.concurrent.sleep`](concurrent#sleep) uses **milliseconds**, so take care to distinguish them.

- `seconds` — Number of seconds to sleep; accepts an `Int` (interpreted as seconds) or a `Float`

Error: Throws `E6007` if the argument is neither an `Int` nor a `Float`.

```yaoxiang
use std.time

main: () -> Void = {
    time.sleep(0.0)
}
```

> Not exported for `wasm32` targets.

## Formatting and Parsing

### format_time

<!-- stdlib:sig:time.format_time start -->

```yaoxiang
format_time: (dt: Int, fmt: String) -> String
```

<!-- stdlib:sig:time.format_time end -->

Formats a timestamp according to `fmt` and supports `strftime`-style placeholders.

- `dt` — Unix timestamp (**seconds**), must be an `Int`
- `fmt` — Format string

> The return values of [`now`](#now) and [`parse_time`](#parse_time) are already `Int`s, so passing
> them directly **does not** raise `E1002`.

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

Timestamp components are broken down in **UTC** (`src/std/time.rs:123-178` performs pure arithmetic
without a local time-zone offset). Unrecognized placeholders are left unchanged.

Error: Throws `E6007` when there are too few arguments.

```yaoxiang
use std.assert
use std.string
use std.time

main: () -> Void = {
    // 0 = Unix epoch (UTC)
    assert(time.format_time(0, "%Y") == "1970")
    assert(time.format_time(0, "%m") == "01")
    assert(time.format_time(0, "%F %T") == "1970-01-01 00:00:00")

    // The current timestamp (Int) can also be used directly
    s = time.format_time(time.timestamp(), "%Y")
    assert(string.len(s) == 4)
}
```

### parse_time

<!-- stdlib:sig:time.parse_time start -->

```yaoxiang
parse_time: (fmt: String, s: String) -> Int
```

<!-- stdlib:sig:time.parse_time end -->

Parses a time string into a Unix timestamp (**seconds**) according to `fmt`.

- `fmt` — Format string, **used for parsing** (`src/std/time.rs:382` calls
  `parse_by_format(&fmt, &s)`)
- `s` — String to parse

Returns: an `Int` timestamp (**seconds**; see [`now`](#now)). It can be used directly in arithmetic
and passed directly to [`format_time`](#format_time) and all the `datetime_*` accessors.

The directives recognized by `fmt` correspond one-to-one with those recognized by
[`format_time`](#format_time): `%Y` `%m` `%d` `%H` `%M` `%S`, as well as the composite forms `%F`
(`%Y-%m-%d`) and `%T` (`%H:%M:%S`). **Any fields that are omitted default to January 1 at
00:00:00**—`parse_time("%Y", "2024")` yields `2024-01-01T00:00:00Z`.

Error: Throws `E6007` if the string does not match `fmt`, with a message such as
`Invalid time format: '…' does not match '…'`. This means that `fmt` is a **contract**, not merely a
hint: `parse_time("%d/%m/%Y", "15/01/2024")` succeeds, whereas
`parse_time("totally-bogus", "2024-01-15T10:30:00")` raises an error.

```yaoxiang
use std.assert
use std.time

main: () -> Void = {
    // fmt determines the parsing format
    ts = time.parse_time("%d/%m/%Y", "15/01/2024")
    assert(time.datetime_year(ts) == 2024)
    assert(time.datetime_month(ts) == 1)
    assert(time.datetime_day(ts) == 15)

    // Composite directives are also recognized
    full = time.parse_time("%Y-%m-%d %H:%M:%S", "2024-01-15 10:30:00")
    assert(time.datetime_hour(full) == 10)
    assert(time.datetime_minute(full) == 30)
}
```

## Date Component Access

`std.time` exports eight date-component accessors (fixed in `#338`, with export names in a flat
form):

| Export name          | Signature             | Description            |
| -------------------- | --------------------- | ---------------------- |
| `datetime_year`      | `(dt: Int) -> Int`    | Four-digit year        |
| `datetime_month`     | `(dt: Int) -> Int`    | Month (1–12)           |
| `datetime_day`       | `(dt: Int) -> Int`    | Day (1–31)             |
| `datetime_hour`      | `(dt: Int) -> Int`    | Hour (0–23)            |
| `datetime_minute`    | `(dt: Int) -> Int`    | Minute (0–59)          |
| `datetime_second`    | `(dt: Int) -> Int`    | Second (0–59)          |
| `datetime_weekday`   | `(dt: Int) -> Int`    | Weekday (0 = Sunday)   |
| `datetime_to_string` | `(dt: Int) -> String` | ISO 8601-format string |

The return values of `now()` / `parse_time()` are `Int` timestamps and can be passed directly to
these accessors or used directly with [`format_time`](#format_time). They are broken down in **UTC**
(`src/std/time.rs:123-178` performs pure arithmetic without a local time-zone offset):

```yaoxiang
use std.assert
use std.time

main: () -> Void = {
    ts = time.parse_time("%Y-%m-%d", "2024-01-15")
    assert(time.datetime_year(ts) == 2024)
    assert(time.datetime_month(ts) == 1)
    assert(time.datetime_day(ts) == 15)

    // Equivalent to format_time
    assert(time.format_time(ts, "%Y-%m-%d") == "2024-01-15")

    // 0 = Unix epoch (Thursday in UTC)
    assert(time.datetime_weekday(0) == 4)
    assert(time.datetime_to_string(0) == "1970-01-01T00:00:00")
}
```

## Related

- [`std.concurrent`](concurrent) — Millisecond-precision sleep
- [Error code reference](../error-code/) — `E6007` generic runtime error

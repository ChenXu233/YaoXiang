---
title: 'std.time'
description: 'Timestamps, formatting, and DateTime field access'
---

# std.time

Time module.

```yaoxiang
use std.time
```

> **Implementation gaps have been fixed (#338 / #340, 2026-09-19)**.
>
> `DateTime` is an **alias for the timestamp** (i.e. `Int`, see
> `src/frontend/core/types/mono.rs:621-627`) — at runtime it is already `RuntimeValue::Int`
> (`src/std/time.rs:243`); previously it was just a name without a backing type, so the return value
> of `now()` couldn't be passed into `format_time` and the accessors. The accessor export name has
> also been changed from `DateTime::year` to **`datetime_year`** (`::` is a lexically reserved token
> and cannot appear in field-access position).
>
> Now the return values of `now()` / `parse_time()` can be used directly in arithmetic, formatting,
> and all accessors.
>
> **Time zone**: `format_time` and the 8 `datetime_*` accessors all go through
> `timestamp_to_datetime` (`src/std/time.rs:123-178`), which is **pure UTC arithmetic** — it does
> not read any local time-zone offset, and there is no `Local` / `Utc` distinction. The ISO 8601
> string output by `datetime_to_string` is therefore also UTC.

## Function list

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

<!-- stdlib:table:time end -->## Time acquisition

### now

<!-- stdlib:sig:time.now start -->

```yaoxiang
now: () -> DateTime
```

<!-- stdlib:sig:time.now end -->

Returns the current time.

Returns: an `Int` timestamp (**seconds**), i.e. the number of seconds since the Unix epoch (returns
`RuntimeValue::Int` directly in `src/std/time.rs:238-244`). The signature writes it as `DateTime`,
but `DateTime` is an alias for `Int` (`src/frontend/core/types/mono.rs:627`), so the return value
**is** an `Int` — it can participate in arithmetic and comparisons directly, and can also be passed
directly to [`format_time`](#format_time) and all `datetime_*` accessors.

For millisecond precision, use [`timestamp_ms`](#timestamp_ms).

```yaoxiang
use std.assert
use std.time

main: () -> Void = {
    t = time.now()
    assert(time.datetime_year(t) > 2020)     // Int, can be passed directly to accessors
    assert(t > 0)                             // Int, can participate in comparisons directly
    println(time.format_time(t, "%Y-%m-%dT%H:%M:%SZ"))
}
```

> [`now`](#now) and [`timestamp`](#timestamp) return the same thing (both are Unix timestamps in
> seconds); the only difference is readability: the former's return type is annotated as `DateTime`,
> the latter as `Int`.

### timestamp

<!-- stdlib:sig:time.timestamp start -->

```yaoxiang
timestamp: () -> Int
```

<!-- stdlib:sig:time.timestamp end -->

Returns the current Unix timestamp (**seconds**), which can participate in arithmetic and
comparisons directly.

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
    // millisecond precision is no lower than second precision
    assert(time.timestamp_ms() >= time.timestamp())
}
```

### sleep

<!-- stdlib:sig:time.sleep start -->

```yaoxiang
sleep: (seconds: Float) -> Void
```

<!-- stdlib:sig:time.sleep end -->

Sleeps for the specified number of **seconds** (decimals allowed). The same-named
[`std.concurrent.sleep`](concurrent#sleep) is in **milliseconds**, so be careful to distinguish
them.

- `seconds` — the number of seconds to sleep; accepts `Int` (interpreted as seconds) or `Float`

Error: throws `E6007` when the argument is neither `Int` nor `Float`.

```yaoxiang
use std.time

main: () -> Void = {
    time.sleep(0.0)
}
```

> Not exported on the `wasm32` target.

## Formatting and parsing

### format_time

<!-- stdlib:sig:time.format_time start -->

```yaoxiang
format_time: (dt: Int, fmt: String) -> String
```

<!-- stdlib:sig:time.format_time end -->

Formats a timestamp according to `fmt`, supporting `strftime`-style placeholders.

- `dt` — Unix timestamp (**seconds**), must be `Int`
- `fmt` — format string

> `DateTime` is an alias for `Int`, so passing the return value of [`now`](#now) or
> [`parse_time`](#parse_time) will **not** produce an `E1002` — both sides are `Int` and can be
> passed in directly.

Supported placeholders:

| Placeholder | Meaning                  | Example      |
| ----------- | ------------------------ | ------------ |
| `%Y`        | Four-digit year          | `2024`       |
| `%m`        | Two-digit month          | `01`         |
| `%d`        | Two-digit day            | `15`         |
| `%H`        | Two-digit hour (24-hour) | `10`         |
| `%M`        | Two-digit minute         | `30`         |
| `%S`        | Two-digit second         | `00`         |
| `%w`        | Weekday (0 = Sunday)     | `1`          |
| `%F`        | Equivalent to `%Y-%m-%d` | `2024-01-15` |
| `%T`        | Equivalent to `%H:%M:%S` | `10:30:00`   |

Decomposition is done in **UTC** (`src/std/time.rs:123-178` is pure arithmetic, without any local
time-zone offset). Unrecognized placeholders are preserved as-is.

Error: throws `E6007` when there are not enough arguments.

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
parse_time: (fmt: String, s: String) -> DateTime
```

<!-- stdlib:sig:time.parse_time end -->

Parses a time string according to `fmt` into a Unix timestamp (**seconds**).

- `fmt` — format string, **participates in parsing** (calls `parse_by_format(&fmt, &s)` at
  `src/std/time.rs:382`)
- `s` — string to parse

Returns: an `Int` timestamp (the signature writes it as `DateTime`, which is actually an `Int`
alias; see [`now`](#now)), which can be used directly in arithmetic, with
[`format_time`](#format_time), and with all `datetime_*` accessors.

The directives recognized by `fmt` correspond one-to-one with those of
[`format_time`](#format_time): `%Y` `%m` `%d` `%H` `%M` `%S`, as well as the combined forms `%F`
(`%Y-%m-%d`) and `%T` (`%H:%M:%S`). **Fields that are not present default to January 1, 00:00:00** —
`parse_time("%Y", "2024")` yields `2024-01-01T00:00:00Z`.

Error: throws `E6007` when the string doesn't match `fmt`, with a message like
`Invalid time format: '…' does not match '…'`. This means that `fmt` is a **contract**, not a hint:
`parse_time("%d/%m/%Y", "15/01/2024")` passes, while
`parse_time("totally-bogus", "2024-01-15T10:30:00")` reports an error.

```yaoxiang
use std.assert
use std.time

main: () -> Void = {
    // fmt determines the parsing shape
    ts = time.parse_time("%d/%m/%Y", "15/01/2024")
    assert(time.datetime_year(ts) == 2024)
    assert(time.datetime_month(ts) == 1)
    assert(time.datetime_day(ts) == 15)

    // Combined directives are also recognized
    full = time.parse_time("%Y-%m-%d %H:%M:%S", "2024-01-15 10:30:00")
    assert(time.datetime_hour(full) == 10)
    assert(time.datetime_minute(full) == 30)
}
```

## DateTime field access

`std.time` exports 8 date-component accessors (fixed in `#338`; export names are in flat form):

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

`DateTime` is an **alias for the timestamp** (i.e. `Int`) — the return values of `now()` /
`parse_time()` can be passed directly into these accessors, and can also be used directly in
[`format_time`](#format_time). Decomposition is done in **UTC** (`src/std/time.rs:123-178` is pure
arithmetic, without any local time-zone offset):

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

    // 0 = Unix epoch (UTC Thursday)
    assert(time.datetime_weekday(0) == 4)
    assert(time.datetime_to_string(0) == "1970-01-01T00:00:00")
}
```

## Related

- [`std.concurrent`](concurrent) — millisecond-precision sleep
- [Error code reference](../error-code/) — `E6007` generic runtime error

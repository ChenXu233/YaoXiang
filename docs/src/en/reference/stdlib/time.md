---
title: 'std.time'
description: 'Timestamps, formatting, and DateTime field access'
---

# std.time

Time module.

```yaoxiang
use std.time
```

> **This module has several implementation gaps (#338 / #340)**, which have been verified through
> actual testing while writing this documentation. Available and unavailable surfaces are noted
> separately below to avoid producing examples that won't compile based on the signatures.
>
> Available for normal use: [`now`](#now) / [`timestamp`](#timestamp) /
> [`timestamp_ms`](#timestamp_ms) / [`sleep`](#sleep) / [`format_time`](#format_time) (accepts `Int`
> timestamp literals).
>
> Currently unavailable: the return value of [`parse_time`](#parse_time), and all
> [`DateTime::*`](#datetime-field-access-unavailable) accessors.

## Function List

<!-- stdlib:table:time start -->

| Function              | Signature                              |
| --------------------- | -------------------------------------- |
| `now`                 | `() -> DateTime`                       |
| `timestamp`           | `() -> Int`                            |
| `timestamp_ms`        | `() -> Int`                            |
| `sleep`               | `(seconds: Float) -> Void`             |
| `format_time`         | `(dt: Int, fmt: String) -> String`     |
| `parse_time`          | `(fmt: String, s: String) -> DateTime` |
| `DateTime::year`      | `(dt: Int) -> Int`                     |
| `DateTime::month`     | `(dt: Int) -> Int`                     |
| `DateTime::day`       | `(dt: Int) -> Int`                     |
| `DateTime::hour`      | `(dt: Int) -> Int`                     |
| `DateTime::minute`    | `(dt: Int) -> Int`                     |
| `DateTime::second`    | `(dt: Int) -> Int`                     |
| `DateTime::weekday`   | `(dt: Int) -> Int`                     |
| `DateTime::to_string` | `(dt: Int) -> String`                  |

<!-- stdlib:table:time end -->## Getting Time

### now

<!-- stdlib:sig:time.now start -->

```yaoxiang
now: () -> DateTime
```

<!-- stdlib:sig:time.now end -->

Returns the current time.

Returns: a `DateTime` value that prints in the form `DateTime(1789471990)`. **It is not an `Int`**,
so it cannot directly participate in arithmetic or comparisons, nor can it be passed as an `Int`
parameter to other functions (see [`format_time`](#format_time)).

```yaoxiang
use std.time

main: () -> Void = {
    t = time.now()
    println(t)          // DateTime(1789471990)
}
```

> When you need to perform calculations, use [`timestamp`](#timestamp) or
> [`timestamp_ms`](#timestamp_ms), which return `Int` directly.

### timestamp

<!-- stdlib:sig:time.timestamp start -->

```yaoxiang
timestamp: () -> Int
```

<!-- stdlib:sig:time.timestamp end -->

Returns the current Unix timestamp (in **seconds**), which can directly participate in arithmetic
and comparisons.

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

Returns the current Unix timestamp (in **milliseconds**).

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
[`std.concurrent.sleep`](./concurrent#sleep) takes **milliseconds**, so be careful to distinguish.

- `seconds` —— number of seconds to sleep; accepts `Int` (interpreted as seconds) or `Float`

Errors: throws `E6007` when the argument is neither `Int` nor `Float`.

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

Formats a timestamp according to `fmt`, supporting `strftime`-style placeholders.

- `dt` —— Unix timestamp (in **seconds**), must be `Int`
- `fmt` —— format string

> **Type note**: `dt` must be `Int`. Passing the return value of [`now`](#now) or
> [`parse_time`](#parse_time) will produce error `E1002`
> (`expected type 'int64', found type 'DateTime'`), because they are both `DateTime`. Currently
> there is no `DateTime` → `Int` conversion, so **in practice you can only pass an `Int` literal or
> the result of [`timestamp`](#timestamp)**.

Supported placeholders:

| Placeholder | Meaning                  | Example      |
| ----------- | ------------------------ | ------------ |
| `%Y`        | Four-digit year          | `2024`       |
| `%m`        | Two-digit month          | `01`         |
| `%d`        | Two-digit day            | `15`         |
| `%H`        | Two-digit hour (24-hour) | `10`         |
| `%M`        | Two-digit minute         | `30`         |
| `%S`        | Two-digit second         | `00`         |
| `%w`        | Day of week (0 = Sunday) | `1`          |
| `%F`        | Equivalent to `%Y-%m-%d` | `2024-01-15` |
| `%T`        | Equivalent to `%H:%M:%S` | `10:30:00`   |

Decomposed in **local time**. Unrecognized placeholders are preserved as-is.

Errors: throws `E6007` when `dt` is not `Int`, `fmt` is not `String`, or arguments are insufficient.

```yaoxiang
use std.assert
use std.string
use std.time

main: () -> Void = {
    // 0 = Unix epoch
    assert(time.format_time(0, "%Y") == "1970")
    assert(time.format_time(0, "%m") == "01")

    // Current timestamp (Int) can also be used directly
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

- `fmt` —— **currently ignored** (see below)
- `s` —— string to parse

Returns: a `DateTime` value.

> **Two implementation limitations (#340)**:
>
> 1. The `fmt` parameter **does not participate in parsing**. The function only recognizes the ISO
>    8601 form `YYYY-MM-DDTHH:MM:SS` or `YYYY-MM-DD HH:MM:SS` (date and time separated by `T` or a
>    space); any other form fails regardless of what `fmt` is.
> 2. The returned `DateTime` **cannot currently be used further**—it is neither an `Int` (so it
>    can't be passed to `format_time` / `DateTime::*`), nor does it have any callable accessors (see
>    the next section).

Errors: throws `E6007` when the format does not match.

```yaoxiang
use std.time

main: () -> Void = {
    // Parsing itself can succeed
    ts = time.parse_time("", "2024-01-15 10:30:00")
    println(ts)
}
```

## DateTime Field Access (Unavailable)

> Tracking: #338

`std.time` exports 8 accessors named with the `DateTime::` prefix:

| Export name           | Signature             | Description              |
| --------------------- | --------------------- | ------------------------ |
| `DateTime::year`      | `(dt: Int) -> Int`    | Four-digit year          |
| `DateTime::month`     | `(dt: Int) -> Int`    | Month (1–12)             |
| `DateTime::day`       | `(dt: Int) -> Int`    | Day (1–31)               |
| `DateTime::hour`      | `(dt: Int) -> Int`    | Hour (0–23)              |
| `DateTime::minute`    | `(dt: Int) -> Int`    | Minute (0–59)            |
| `DateTime::second`    | `(dt: Int) -> Int`    | Second (0–59)            |
| `DateTime::weekday`   | `(dt: Int) -> Int`    | Day of week (0 = Sunday) |
| `DateTime::to_string` | `(dt: Int) -> String` | ISO 8601 format string   |

**However, these names currently cannot be called from YaoXiang source code (#338).** The export
name contains `::`, and `::` is a reserved token in the syntax and cannot appear in a field access
position. The following attempts have been verified through testing to all fail:

| Attempted syntax           | Result                                              |
| -------------------------- | --------------------------------------------------- |
| `time.DateTime::year(0)`   | `E0010 expected RParen, found ColonColon`           |
| `time.DateTime.year(0)`    | `E1042 Field 'DateTime' not found in struct 'time'` |
| `time.year(0)`             | `E1042 Field 'year' not found in struct 'time'`     |
| `time.DateTime_year(0)`    | `E1042 Field 'DateTime_year' not found`             |
| `time."DateTime::year"(0)` | `E0011 Unexpected token: StringLiteral`             |

**Alternative**: When you need date components, use [`format_time`](#format_time) to format as
needed—the internal timestamp → year/month/day decomposition is already done there:

```yaoxiang
use std.assert
use std.time

main: () -> Void = {
    // Use format_time to get each component
    assert(time.format_time(0, "%Y") == "1970")
    assert(time.format_time(0, "%m") == "01")
    assert(time.format_time(0, "%d") == "01")
}
```

## Related

- [`std.concurrent`](./concurrent) —— millisecond-level sleep
- [Error code reference](../error-code/) —— `E6007` generic runtime error

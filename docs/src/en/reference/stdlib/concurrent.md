---
title: 'std.concurrent'
description: 'Sleep, yield scheduling, and thread identifier'
---

# std.concurrent

Concurrency helper module.

```yaoxiang
use std.concurrent
```

> This module depends on operating system threads and is **not exported** on the `wasm32` target.

## Function List

<!-- stdlib:table:concurrent start -->

| Function    | Signature               |
| ----------- | ----------------------- |
| `sleep`     | `(millis: Int) -> Void` |
| `thread_id` | `() -> String`          |
| `yield_now` | `() -> Void`            |

<!-- stdlib:table:concurrent end -->

## Functions

### sleep

<!-- stdlib:sig:concurrent.sleep start -->

```yaoxiang
sleep: (millis: Int) -> Void
```

<!-- stdlib:sig:concurrent.sleep end -->

Blocks the current thread for the specified number of **milliseconds**.

- `millis` —— number of milliseconds to sleep; if not an `Int` or missing, treated as `0` (no error)

> **Unit note**: [`std.time.sleep`](./time#sleep) takes **seconds** and accepts decimals, while this
> function takes **milliseconds**. `concurrent.sleep(1)` sleeps for 1 millisecond, and
> `time.sleep(1)` sleeps for 1 second.

```yaoxiang
use std.concurrent

main: () -> Void = {
    concurrent.sleep(0)
    concurrent.sleep(1)
}
```

### thread_id

<!-- stdlib:sig:concurrent.thread_id start -->

```yaoxiang
thread_id: () -> String
```

<!-- stdlib:sig:concurrent.thread_id end -->

Returns the identifier string of the current thread.

Returns: a string like `ThreadId(1)`. The specific value varies by platform and scheduling; you
should **only check for its existence**, not rely on its specific content or format.

```yaoxiang
use std.assert
use std.concurrent
use std.string

main: () -> Void = {
    tid = concurrent.thread_id()
    assert(string.len(tid) > 0)
}
```

### yield_now

<!-- stdlib:sig:concurrent.yield_now start -->

```yaoxiang
yield_now: () -> Void
```

<!-- stdlib:sig:concurrent.yield_now end -->

Actively yields the current thread's scheduling time slice, giving other threads a chance to run.

```yaoxiang
use std.concurrent

main: () -> Void = {
    concurrent.yield_now()
}
```

## Related

- [`std.time.sleep`](./time#sleep) —— second-level sleep
- [Language Spec: Concurrency Model](../language-spec/concurrency.md) —— `spawn` and spawn semantics

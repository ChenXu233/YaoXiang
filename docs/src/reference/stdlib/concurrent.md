---
title: 'std.concurrent'
description: '休眠、让出调度与线程标识'
---

# std.concurrent

并发辅助模块。

```yaoxiang
use std.concurrent
```

> 本模块依赖操作系统线程，在 `wasm32` 目标上**不导出**。

## 函数一览

<!-- stdlib:table:concurrent start -->

| 函数        | 签名                    |
| ----------- | ----------------------- |
| `sleep`     | `(millis: Int) -> Void` |
| `thread_id` | `() -> String`          |
| `yield_now` | `() -> Void`            |

<!-- stdlib:table:concurrent end -->

## 函数

### sleep

<!-- stdlib:sig:concurrent.sleep start -->

```yaoxiang
sleep: (millis: Int) -> Void
```

<!-- stdlib:sig:concurrent.sleep end -->

阻塞当前线程指定**毫秒数**。

- `millis` —— 休眠毫秒数；非 `Int` 或缺失时按 `0` 处理（不报错）

> **单位注意**：[`std.time.sleep`](./time#sleep)
> 以**秒**为单位且接受小数，本函数以**毫秒**为单位。`concurrent.sleep(1)` 睡 1 毫秒，
> `time.sleep(1)` 睡 1 秒。

```yaoxiang
use std.concurrent

main = {
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

返回当前线程的标识字符串。

返回：形如 `ThreadId(1)`
的字符串。具体值随平台与调度变化，**只应做存在性判断**，不要依赖其具体内容或格式。

```yaoxiang
use std.assert
use std.concurrent
use std.string

main = {
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

主动让出当前线程的调度时间片，给其它线程运行机会。

```yaoxiang
use std.concurrent

main = {
    concurrent.yield_now()
}
```

## 相关

- [`std.time.sleep`](./time#sleep) —— 秒级休眠
- [语言规范：并发模型](../language-spec/concurrency.md) —— `spawn` 与并作语义

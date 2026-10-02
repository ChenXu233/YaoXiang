---
title: 'std.time'
description: '时间戳、格式化与 DateTime 字段访问'
---

# std.time

时间模块。

```yaoxiang
use std.time
```

> **实现缺口已修复（#338 / #340，2026-09-19）**。
>
> `DateTime` 是**时间戳的别名**（即 `Int`，见
> `src/frontend/core/types/mono.rs:621-627`）——运行时本就是
> `RuntimeValue::Int`（`src/std/time.rs:243`），此前只是一个没实体的名字，使 `now()` 的
> 返回值传不进 `format_time` 与访问器。访问器导出名也从 `DateTime::year` 改为
> **`datetime_year`**（`::` 是词法保留记号，不能出现在字段访问位置）。
>
> 现在 `now()` / `parse_time()` 的返回值可直接用于算术、格式化与全部访问器。
>
> **时区**：`format_time` 与 8 个 `datetime_*` 访问器都走 `timestamp_to_datetime`
> （`src/std/time.rs:123-178`），那是**纯 UTC 算术**——不读本地时区偏移，也没有
> `Local` / `Utc` 之分。`datetime_to_string` 输出的 ISO 8601 串因此也是 UTC。

## 函数一览

<!-- stdlib:table:time start -->

| 函数 | 签名 |
| ---- | ---- |
| `now` | `() -> DateTime` |
| `timestamp` | `() -> Int` |
| `timestamp_ms` | `() -> Int` |
| `sleep` | `(seconds: Float) -> Void` |
| `format_time` | `(dt: Int, fmt: String) -> String` |
| `parse_time` | `(fmt: String, s: String) -> DateTime` |
| `datetime_year` | `(dt: Int) -> Int` |
| `datetime_month` | `(dt: Int) -> Int` |
| `datetime_day` | `(dt: Int) -> Int` |
| `datetime_hour` | `(dt: Int) -> Int` |
| `datetime_minute` | `(dt: Int) -> Int` |
| `datetime_second` | `(dt: Int) -> Int` |
| `datetime_weekday` | `(dt: Int) -> Int` |
| `datetime_to_string` | `(dt: Int) -> String` |

<!-- stdlib:table:time end -->## 时间获取

### now

<!-- stdlib:sig:time.now start -->

```yaoxiang
now: () -> DateTime
```

<!-- stdlib:sig:time.now end -->

返回当前时间。

返回：`Int` 时间戳（**秒**），即 Unix epoch 起的秒数（`src/std/time.rs:238-244` 直接
`RuntimeValue::Int`）。签名里写作 `DateTime`，但 `DateTime` 是 `Int` 的别名
（`src/frontend/core/types/mono.rs:627`），因此返回值**就是** `Int`——可直接参与算术与
比较，也可直接传给 [`format_time`](#format_time) 与全部 `datetime_*` 访问器。

需要毫秒精度用 [`timestamp_ms`](#timestamp_ms)。

```yaoxiang
use std.assert
use std.time

main: () -> Void = {
    t = time.now()
    assert(time.datetime_year(t) > 2020)     // Int，可直接传入访问器
    assert(t > 0)                             // Int，可直接参与比较
    println(time.format_time(t, "%Y-%m-%dT%H:%M:%SZ"))
}
```

> [`now`](#now) 与 [`timestamp`](#timestamp) 返回同一个东西（都是 Unix 秒时间戳），
> 区别只在可读性：前者的返回类型标注为 `DateTime`，后者标注为 `Int`。

### timestamp

<!-- stdlib:sig:time.timestamp start -->

```yaoxiang
timestamp: () -> Int
```

<!-- stdlib:sig:time.timestamp end -->

返回当前 Unix 时间戳（**秒**），可直接参与算术与比较。

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

返回当前 Unix 时间戳（**毫秒**）。

```yaoxiang
use std.assert
use std.time

main: () -> Void = {
    // 毫秒精度不低于秒精度
    assert(time.timestamp_ms() >= time.timestamp())
}
```

### sleep

<!-- stdlib:sig:time.sleep start -->

```yaoxiang
sleep: (seconds: Float) -> Void
```

<!-- stdlib:sig:time.sleep end -->

休眠指定**秒数**（可带小数）。同名的 [`std.concurrent.sleep`](./concurrent#sleep)
以**毫秒**为单位，注意区分。

- `seconds` —— 休眠秒数；接受 `Int`（按秒解释）或 `Float`

错误：参数既非 `Int` 也非 `Float` 时抛出 `E6007`。

```yaoxiang
use std.time

main: () -> Void = {
    time.sleep(0.0)
}
```

> 在 `wasm32` 目标上不导出。

## 格式化与解析

### format_time

<!-- stdlib:sig:time.format_time start -->

```yaoxiang
format_time: (dt: Int, fmt: String) -> String
```

<!-- stdlib:sig:time.format_time end -->

把时间戳按 `fmt` 格式化，支持 `strftime` 风格占位符。

- `dt` —— Unix 时间戳（**秒**），必须是 `Int`
- `fmt` —— 格式串

> `DateTime` 是 `Int` 的别名，所以传 [`now`](#now) 或 [`parse_time`](#parse_time) 的
> 返回值**不会**报 `E1002`——双方都是 `Int`，可直接传入。

支持的占位符：

| 占位符 | 含义                  | 示例         |
| ------ | --------------------- | ------------ |
| `%Y`   | 四位年份              | `2024`       |
| `%m`   | 两位月份              | `01`         |
| `%d`   | 两位日期              | `15`         |
| `%H`   | 两位小时（24 小时制） | `10`         |
| `%M`   | 两位分钟              | `30`         |
| `%S`   | 两位秒                | `00`         |
| `%w`   | 星期（0 = 周日）      | `1`          |
| `%F`   | 等价于 `%Y-%m-%d`     | `2024-01-15` |
| `%T`   | 等价于 `%H:%M:%S`     | `10:30:00`   |

按 **UTC** 拆解（`src/std/time.rs:123-178` 是纯算术，不含本地时区偏移）。
不认识的占位符原样保留。

错误：参数不足时抛出 `E6007`。

```yaoxiang
use std.assert
use std.string
use std.time

main: () -> Void = {
    // 0 = Unix epoch（UTC）
    assert(time.format_time(0, "%Y") == "1970")
    assert(time.format_time(0, "%m") == "01")
    assert(time.format_time(0, "%F %T") == "1970-01-01 00:00:00")

    // 当前时间戳（Int）也可直接使用
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

按 `fmt` 把时间字符串解析为 Unix 时间戳（**秒**）。

- `fmt` —— 格式串，**参与解析**（`src/std/time.rs:382` 调 `parse_by_format(&fmt, &s)`）
- `s` —— 待解析字符串

返回：`Int` 时间戳（签名写作 `DateTime`，实为 `Int` 别名，见 [`now`](#now)），
可直接用于算术、[`format_time`](#format_time) 与全部 `datetime_*` 访问器。

`fmt` 认得的指示符与 [`format_time`](#format_time) 一一对应：`%Y` `%m` `%d` `%H` `%M`
`%S`，以及组合形 `%F`（`%Y-%m-%d`）与 `%T`（`%H:%M:%S`）。**未出现的字段默认
1 月 1 日 0 时 0 分 0 秒**——`parse_time("%Y", "2024")` 得到的是 `2024-01-01T00:00:00Z`。

错误：字符串与 `fmt` 不匹配时抛出 `E6007`，消息形如
`Invalid time format: '…' does not match '…'`。这意味着 `fmt` 是**契约**而非提示：
`parse_time("%d/%m/%Y", "15/01/2024")` 能过，而
`parse_time("totally-bogus", "2024-01-15T10:30:00")` 报错。

```yaoxiang
use std.assert
use std.time

main: () -> Void = {
    // fmt 决定解析形态
    ts = time.parse_time("%d/%m/%Y", "15/01/2024")
    assert(time.datetime_year(ts) == 2024)
    assert(time.datetime_month(ts) == 1)
    assert(time.datetime_day(ts) == 15)

    // 组合指示符也认
    full = time.parse_time("%Y-%m-%d %H:%M:%S", "2024-01-15 10:30:00")
    assert(time.datetime_hour(full) == 10)
    assert(time.datetime_minute(full) == 30)
}
```

## DateTime 字段访问

`std.time` 导出 8 个日期分量访问器（`#338` 已修复，导出名为扁平形式）：

| 导出名                  | 签名                  | 说明                |
| ----------------------- | --------------------- | ------------------- |
| `datetime_year`         | `(dt: Int) -> Int`    | 四位年份            |
| `datetime_month`        | `(dt: Int) -> Int`    | 月份（1–12）        |
| `datetime_day`          | `(dt: Int) -> Int`    | 日期（1–31）        |
| `datetime_hour`         | `(dt: Int) -> Int`    | 小时（0–23）        |
| `datetime_minute`       | `(dt: Int) -> Int`    | 分钟（0–59）        |
| `datetime_second`       | `(dt: Int) -> Int`    | 秒（0–59）          |
| `datetime_weekday`      | `(dt: Int) -> Int`    | 星期（0 = 周日）    |
| `datetime_to_string`    | `(dt: Int) -> String` | ISO 8601 形态字符串 |

`DateTime` 是**时间戳的别名**（即 `Int`）——`now()` / `parse_time()` 的返回值
可直接传入这些访问器，也可直接用于 [`format_time`](#format_time)。拆解按 **UTC**
（`src/std/time.rs:123-178` 纯算术，不含本地时区偏移）：

```yaoxiang
use std.assert
use std.time

main: () -> Void = {
    ts = time.parse_time("%Y-%m-%d", "2024-01-15")
    assert(time.datetime_year(ts) == 2024)
    assert(time.datetime_month(ts) == 1)
    assert(time.datetime_day(ts) == 15)

    // 与 format_time 等价
    assert(time.format_time(ts, "%Y-%m-%d") == "2024-01-15")

    // 0 = Unix epoch（UTC 星期四）
    assert(time.datetime_weekday(0) == 4)
    assert(time.datetime_to_string(0) == "1970-01-01T00:00:00")
}
```

## 相关

- [`std.concurrent`](./concurrent) —— 毫秒级休眠
- [错误码参考](../error-code/) —— `E6007` 通用运行时错误

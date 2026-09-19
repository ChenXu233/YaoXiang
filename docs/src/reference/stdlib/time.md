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
> `DateTime` 现为**时间戳的别名**（即 `Int`）——运行时本就是
> `RuntimeValue::Int`，此前只是一个没实体的名字，使 `now()` 的返回值传不进
> `format_time` 与访问器。访问器导出名也从 `DateTime::year` 改为
> **`datetime_year`**（`::` 是词法保留记号，不能出现在字段访问位置）。
>
> 现在 `now()` / `parse_time()` 的返回值可直接用于算术、格式化与全部访问器。

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

返回：`DateTime` 值，打印形如 `DateTime(1789471990)`。**它不是
`Int`**，因此不能直接参与算术或比较，也无法作为 `Int` 形参传给其它函数（见
[`format_time`](#format_time)）。

```yaoxiang
use std.time

main: () -> Void = {
    t = time.now()
    println(t)          // DateTime(1789471990)
}
```

> 需要参与计算时用 [`timestamp`](#timestamp) 或 [`timestamp_ms`](#timestamp_ms)，它们直接返回
> `Int`。

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

> **类型注意**：`dt` 必须是 `Int`。传入 [`now`](#now) 或 [`parse_time`](#parse_time) 的返回值会报
> `E1002` （`expected type 'int64', found type 'DateTime'`），因为它们都是 `DateTime`。目前没有
> `DateTime` → `Int` 的转换手段，所以**实际只能传 `Int` 字面量或 [`timestamp`](#timestamp)
> 的结果**。

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

按**本地时间**拆解。不认识的占位符原样保留。

错误：`dt` 非 `Int`、`fmt` 非 `String`，或参数不足时抛出 `E6007`。

```yaoxiang
use std.assert
use std.string
use std.time

main: () -> Void = {
    // 0 = Unix epoch
    assert(time.format_time(0, "%Y") == "1970")
    assert(time.format_time(0, "%m") == "01")

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

把时间字符串解析为时间。

- `fmt` —— **目前被忽略**（见下）
- `s` —— 待解析字符串

返回：`DateTime` 值。

> **两个实现限制（#340）**：
>
> 1. `fmt` 参数**不参与解析**。函数只识别 ISO 8601 形态的 `YYYY-MM-DDTHH:MM:SS` 或
>    `YYYY-MM-DD HH:MM:SS`（日期与时间之间用 `T` 或空格分隔），传入其它形态一律失败，无论 `fmt`
>    写什么。
> 2. 返回的 `DateTime` **目前无法继续使用**——它既不是 `Int`（不能传给 `format_time` /
>    `DateTime::*`），也没有可调用的访问器（见下节）。

错误：格式不匹配时抛出 `E6007`。

```yaoxiang
use std.time

main: () -> Void = {
    // 解析本身可以成功
    ts = time.parse_time("", "2024-01-15 10:30:00")
    println(ts)
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
可直接传入这些访问器，也可直接用于 [`format_time`](#format_time)：

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
}
```

## 相关

- [`std.concurrent`](./concurrent) —— 毫秒级休眠
- [错误码参考](../error-code/) —— `E6007` 通用运行时错误

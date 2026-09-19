---
title: 'std.time'
description: '时间戳、格式化与 DateTime 字段访问'
---

# std.time

时间模块。

```yaoxiang
use std.time
```

> **本模块存在若干实现缺口（#338 /
> #340）**，写作文档时已实测确认。可用面与不可用面在下文分别标注，避免照签名写出无法编译的示例。
>
> 可正常使用：[`now`](#now) / [`timestamp`](#timestamp) / [`timestamp_ms`](#timestamp_ms) /
> [`sleep`](#sleep) / [`format_time`](#format_time)（接受 `Int` 时间戳字面量）。
>
> 目前不可用：[`parse_time`](#parse_time) 的返回值，以及全部
> [`DateTime::*`](#datetime-字段访问不可用) 访问器。

## 函数一览

<!-- stdlib:table:time start -->

| 函数                  | 签名                                   |
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

## DateTime 字段访问（不可用）

> 追踪：#338

`std.time` 导出了 8 个以 `DateTime::` 为名的访问器：

| 导出名                | 签名                  | 说明                |
| --------------------- | --------------------- | ------------------- |
| `DateTime::year`      | `(dt: Int) -> Int`    | 四位年份            |
| `DateTime::month`     | `(dt: Int) -> Int`    | 月份（1–12）        |
| `DateTime::day`       | `(dt: Int) -> Int`    | 日期（1–31）        |
| `DateTime::hour`      | `(dt: Int) -> Int`    | 小时（0–23）        |
| `DateTime::minute`    | `(dt: Int) -> Int`    | 分钟（0–59）        |
| `DateTime::second`    | `(dt: Int) -> Int`    | 秒（0–59）          |
| `DateTime::weekday`   | `(dt: Int) -> Int`    | 星期（0 = 周日）    |
| `DateTime::to_string` | `(dt: Int) -> String` | ISO 8601 形态字符串 |

**但这些名字目前无法从 YaoXiang 源码调用（#338）。** 导出名含 `::`，而 `::`
是语法中的保留记号，不能出现在字段访问位置。已实测确认以下写法全部失败：

| 尝试的写法                 | 结果                                                |
| -------------------------- | --------------------------------------------------- |
| `time.DateTime::year(0)`   | `E0010 expected RParen, found ColonColon`           |
| `time.DateTime.year(0)`    | `E1042 Field 'DateTime' not found in struct 'time'` |
| `time.year(0)`             | `E1042 Field 'year' not found in struct 'time'`     |
| `time.DateTime_year(0)`    | `E1042 Field 'DateTime_year' not found`             |
| `time."DateTime::year"(0)` | `E0011 Unexpected token: StringLiteral`             |

**替代方案**：需要日期分量时，用 [`format_time`](#format_time)
按需格式化，它内部已完成时间戳 → 年月日的拆解：

```yaoxiang
use std.assert
use std.time

main: () -> Void = {
    // 用 format_time 取各分量
    assert(time.format_time(0, "%Y") == "1970")
    assert(time.format_time(0, "%m") == "01")
    assert(time.format_time(0, "%d") == "01")
}
```

## 相关

- [`std.concurrent`](./concurrent) —— 毫秒级休眠
- [错误码参考](../error-code/) —— `E6007` 通用运行时错误

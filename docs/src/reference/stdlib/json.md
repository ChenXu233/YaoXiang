---
title: 'std.json'
description: 'JSON 解析与序列化'
---

# std.json

JSON 解析与序列化模块（RFC 8259，issue #55）。`Json` 是本模块导出的**记录式和类型**，
与用户自定义和类型同一套机制：变体构造 `Json.num(1.5)`、match 变体解构、穷尽性检查。

```yaoxiang
use std.json
```

> **本页是手写页**。本模块由纯 YaoXiang 源码实现（`src/std/json.yx`），不在
> `StdModule::exports()` 的生成范围内，因此没有 `<!-- stdlib:... -->` 生成区标记——
> 下表的签名逐字取自 `src/std/json.yx` 的导出声明。

## 导出

| 导出       | 签名                                | 说明                             |
| ---------- | ----------------------------------- | -------------------------------- |
| `Json`     | 和类型（六变体）                    | JSON 值                          |
| `parse`    | `(s: &String) -> Result(Json, Error)` | 解析 JSON 文本，失败为 `Err`  |
| `stringify`| `(v: &Json) -> String`              | 紧凑序列化                       |
| `pretty`   | `(v: &Json) -> String`              | 2 空格缩进序列化                 |

## Json 类型

六变体对齐 RFC 8259 §3：

```
Json: Type = {
    null: () -> Json,
    bool: (Bool) -> Json,
    num: (Float) -> Json,
    str: (String) -> Json,
    arr: (Vec(Json)) -> Json,
    obj: (Dict(String, Json)) -> Json,
}
```

变体构造在表达式位置**必须类型限定**（RFC-010），即 `Json.num(1.5)` 而非裸 `num(1.5)`；
解构用 `match` 变体模式（需 `use std.json` 导入变体集，见[语法规范 §2.8](../language-spec/syntax.md)）。

数字统一经 `Float` 承载：JSON 规范本身不区分整型与浮点型（`1` 与 `1.0` 同值），解析后
一律是 `Json.num`，序列化时按最短 `Float` 表示输出。因此 `Json.num(1.0)` 序列化为 `1.0`。

## 函数

### parse

```yaoxiang
parse: (s: &String) -> Result(Json, Error)
```

把 JSON 文本解析为 `Json`。

- `s` —— 待解析文本（只读借用）

返回：`Result.ok(Json)`；失败为 `Result.err(Error)`，**不抛错**。错误码恒为 `E6013`
（`src/std/result.rs:31`），消息形如 `<原因> at line <行>, column <列>`，行列从 1 起算。

语法按 RFC 8259 严格实现：拒绝尾随逗号、尾随内容、单引号与注释；对象键必须是字符串。

```yaoxiang
use std.assert
use std.json
use std.result
use std.string

main: () -> Void = {
    v = result.unwrap(json.parse("{\"n\": 2.0}"))
    assert(json.stringify(v) == "{\"n\":2.0}")

    // 数字统一按 Float 承载
    assert(json.stringify(Json.num(1.0)) == "1.0")

    // 顶层标量也能解析
    assert(json.stringify(result.unwrap(json.parse("null"))) == "null")
    assert(json.stringify(result.unwrap(json.parse("[true]"))) == "[true]")

    // 非法输入走 Err 分支，不中断
    bad = json.parse("{oops}")
    assert(result.is_err(bad))
    assert(result.code(result.unwrap_err(bad)) == "E6013")
    assert(string.contains(result.message(result.unwrap_err(bad)), "line 1"))
}
```

### stringify

```yaoxiang
stringify: (v: &Json) -> String
```

紧凑序列化（不含任何空白）。

- `v` —— JSON 值（只读借用，`Json` 是记录类型，按值复制代价高，故取借用）

返回：JSON 文本。字符串转义对齐 RFC 8259 §7：`"`、`\` 与 U+0000..U+001F
（`\b` `\t` `\n` `\f` `\r` 用短形态，其余写成 `\u00xx`）；非 ASCII 字符原样输出。

> **键序不保证**：对象的键来自 `std.dict`，输出顺序跟随字典的遍历顺序，不等于源文本的
> 书写顺序。需要稳定输出时请自行排序，或改用 [`pretty`](#pretty) 供人阅读。

### pretty

```yaoxiang
pretty: (v: &Json) -> String
```

格式化序列化：2 空格缩进，数组与对象逐元素换行展开；**空容器仍取紧凑形态**
（`[]` / `{}`），不换行。与 [`stringify`](#stringify) 共用同一套字符串转义规则。

```yaoxiang
use std.json
use std.result

main: () -> Void = {
    v = result.unwrap(json.parse("{\"a\":[1.0,null]}"))

    // 紧凑：无空白
    s = json.stringify(v)
    println(s)     // {"a":[1.0,null]}

    // 缩进：2 空格，空容器仍紧凑
    println(json.pretty(Json.arr([])))   // []
    println(json.pretty(v))
}
```

## 构造 Json

对象变体 `obj` 收的是 `Dict(String, Json)`，用 [`std.dict`](./dict) 逐键构造：

```yaoxiang
use std.dict
use std.json
use std.string

main: () -> Void = {
    d = dict.set(dict.new(), "x", Json.num(3.0))
    o = Json.obj(d)
    println(json.stringify(o))     // {"x":3.0}
    println(string.contains(json.pretty(o), "\n"))   // 缩进形态含换行
}
```

## 错误模型

| 场景                       | 表现                                |
| -------------------------- | ----------------------------------- |
| 语法错误（缺键、尾随内容） | `Result.err`，`code` 为 `E6013`     |
| 行列定位                   | 消息形如 `… at line 1, column 2`    |
| 类型不符（传非 `&String`） | 编译期类型错误                      |

解析失败**不中断执行**，用 `?` 传播或 [`result.unwrap`](./result#unwrap) 显式分流。
注意 `?` 传播要求外层函数返回 `Result`（见 [Option 页](./option#已知缺口) 中关于
`Try` 实例化的说明）。

## 已知缺口

- 对象的键序不保证（见 [`stringify`](#stringify) 提示）。
- `\uXXXX` 转义支持代理对组合（高代理必须紧跟低代理），码点合法性由
  `string.from_char_code` 校验，非法码点返回 `E6012`。

## 相关

- [`std.dict`](./dict) —— 构造与读取 `Json.obj` 用的字典
- [`std.string`](./string) —— `char_code` / `from_char_code` 是本模块的转义基元
- [`std.result`](./result) —— 拆包 `parse` 的返回值
- [错误码参考](../error-code/) —— `E6013` JSON 解析失败

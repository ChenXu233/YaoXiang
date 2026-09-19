---
title: 'std.string'
description: '字符串查找、切分、格式化与解析'
---

# std.string

字符串操作模块。除 `format`
外，所有函数对入参做只读借用（`&String`），源字符串在调用后仍可继续使用。

所有函数在参数类型不符时退化为**空字符串语义**（而非报错）：`split`/`trim`/ `upper` 等把非 `String`
入参当作
`""`。这意味着错误的参数类型不会被打断，但也不会得到预期结果——建议依赖类型检查器在编译期拦截。

```yaoxiang
use std.string
```

## 函数一览

<!-- stdlib:table:string start -->

| 函数          | 签名                                                 |
| ------------- | ---------------------------------------------------- |
| `split`       | `(s: &String, sep: &String) -> List(String)`         |
| `trim`        | `(s: &String) -> String`                             |
| `upper`       | `(s: &String) -> String`                             |
| `lower`       | `(s: &String) -> String`                             |
| `replace`     | `(s: &String, old: &String, new: &String) -> String` |
| `contains`    | `(s: &String, sub: &String) -> Bool`                 |
| `starts_with` | `(s: &String, prefix: &String) -> Bool`              |
| `ends_with`   | `(s: &String, suffix: &String) -> Bool`              |
| `index_of`    | `(s: &String, sub: &String) -> Int`                  |
| `substring`   | `(s: &String, start: Int, end: Int) -> String`       |
| `is_empty`    | `(s: &String) -> Bool`                               |
| `len`         | `(s: &String) -> Int`                                |
| `chars`       | `(s: &String) -> List(String)`                       |
| `concat`      | `(s1: &String, s2: &String) -> String`               |
| `repeat`      | `(s: &String, n: Int) -> String`                     |
| `reverse`     | `(s: &String) -> String`                             |
| `format`      | `(format: &String, ...args) -> String`               |
| `parse_int`   | `(s: &String) -> Result(Int, Error)`                 |
| `parse_float` | `(s: &String) -> Result(Float, Error)`               |

<!-- stdlib:table:string end -->## 函数

### split

<!-- stdlib:sig:string.split start -->

```yaoxiang
split: (s: &String, sep: &String) -> List(String)
```

<!-- stdlib:sig:string.split end -->

按 `sep` 切分 `s`，返回子串列表。

- `s` —— 待切分字符串
- `sep` —— 分隔符；**为空串时按字符逐个切分**

返回：`List(String)`。未找到分隔符时返回单元素列表。

```yaoxiang
use std.assert
use std.list
use std.string

main: () -> Void = {
    assert(list.len(string.split("a,b,c", ",")) == 3)
    assert(list.get(string.split("a,b,c", ","), 0) == "a")

    // 空分隔符 → 逐字符
    cs = string.split("abc", "")
    assert(list.len(cs) == 3)
}
```

### trim

<!-- stdlib:sig:string.trim start -->

```yaoxiang
trim: (s: &String) -> String
```

<!-- stdlib:sig:string.trim end -->

去除首尾 Unicode 空白字符。

返回：去除首尾空白后的新字符串（不修改 `s`）。

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.trim("  hi  ") == "hi")
}
```

### upper

<!-- stdlib:sig:string.upper start -->

```yaoxiang
upper: (s: &String) -> String
```

<!-- stdlib:sig:string.upper end -->

转换为大写（Unicode 感知）。

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.upper("abc") == "ABC")
}
```

### lower

<!-- stdlib:sig:string.lower start -->

```yaoxiang
lower: (s: &String) -> String
```

<!-- stdlib:sig:string.lower end -->

转换为小写（Unicode 感知）。

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.lower("ABC") == "abc")
}
```

### replace

<!-- stdlib:sig:string.replace start -->

```yaoxiang
replace: (s: &String, old: &String, new: &String) -> String
```

<!-- stdlib:sig:string.replace end -->

把 `s` 中**所有** `old` 替换为 `new`。

- `old` —— 为空串时**原样返回** `s`（不插入）

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.replace("a-b-c", "-", "+") == "a+b+c")
    assert(string.replace("abc", "", "x") == "abc")
}
```

### contains

<!-- stdlib:sig:string.contains start -->

```yaoxiang
contains: (s: &String, sub: &String) -> Bool
```

<!-- stdlib:sig:string.contains end -->

`sub` 是否出现在 `s` 中。空串恒为 `true`。

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.contains("hello", "ell"))
    assert(!string.contains("hello", "xyz"))
}
```

### starts_with

<!-- stdlib:sig:string.starts_with start -->

```yaoxiang
starts_with: (s: &String, prefix: &String) -> Bool
```

<!-- stdlib:sig:string.starts_with end -->

`s` 是否以 `prefix` 开头。空 `prefix` 恒为 `true`。

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.starts_with("hello", "he"))
}
```

### ends_with

<!-- stdlib:sig:string.ends_with start -->

```yaoxiang
ends_with: (s: &String, suffix: &String) -> Bool
```

<!-- stdlib:sig:string.ends_with end -->

`s` 是否以 `suffix` 结尾。空 `suffix` 恒为 `true`。

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.ends_with("hello", "lo"))
}
```

### index_of

<!-- stdlib:sig:string.index_of start -->

```yaoxiang
index_of: (s: &String, sub: &String) -> Int
```

<!-- stdlib:sig:string.index_of end -->

`sub` 首次出现的**字节**下标。

返回：找到时返回下标；未找到返回 `-1`。

> 返回的是字节偏移。含多字节字符时，可用 `chars` 转换后再定位字符下标。

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.index_of("hello", "ll") == 2)
    assert(string.index_of("hello", "xyz") == -1)
}
```

### substring

<!-- stdlib:sig:string.substring start -->

```yaoxiang
substring: (s: &String, start: Int, end: Int) -> String
```

<!-- stdlib:sig:string.substring end -->

按**字符**下标取 `[start, end)` 区间。

- `start` —— 起始字符下标，缺省 `0`
- `end` —— 结束字符下标（不含），缺省为字符串末尾

返回：截取结果。越界边界被**钳制**到合法范围，不报错；`start > end` 时钳制到空串。

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.substring("hello", 1, 4) == "ell")
    assert(string.substring("hello", 1, 99) == "ello")   // 上界钳制
}
```

### is_empty

<!-- stdlib:sig:string.is_empty start -->

```yaoxiang
is_empty: (s: &String) -> Bool
```

<!-- stdlib:sig:string.is_empty end -->

`s` 是否为空串。

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.is_empty(""))
    assert(!string.is_empty("x"))
}
```

### len

<!-- stdlib:sig:string.len start -->

```yaoxiang
len: (s: &String) -> Int
```

<!-- stdlib:sig:string.len end -->

返回 **UTF-8 字节长度**，不是字符个数。

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.len("hello") == 5)
    assert(string.len("中") == 3)   // 字节长度
}
```

### chars

<!-- stdlib:sig:string.chars start -->

```yaoxiang
chars: (s: &String) -> List(String)
```

<!-- stdlib:sig:string.chars end -->

拆成单字符字符串列表（按 Unicode 标量值）。

```yaoxiang
use std.assert
use std.list
use std.string

main: () -> Void = {
    cs = string.chars("ab")
    assert(list.len(cs) == 2)
    assert(list.get(cs, 0) == "a")
}
```

### concat

<!-- stdlib:sig:string.concat start -->

```yaoxiang
concat: (s1: &String, s2: &String) -> String
```

<!-- stdlib:sig:string.concat end -->

拼接两个字符串。也可直接用 `+` 运算符。

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.concat("a", "b") == "ab")
}
```

### repeat

<!-- stdlib:sig:string.repeat start -->

```yaoxiang
repeat: (s: &String, n: Int) -> String
```

<!-- stdlib:sig:string.repeat end -->

把 `s` 重复 `n` 次。

- `n` —— 重复次数；`n <= 0` 返回空串

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.repeat("ab", 3) == "ababab")
    assert(string.repeat("ab", 0) == "")
}
```

### reverse

<!-- stdlib:sig:string.reverse start -->

```yaoxiang
reverse: (s: &String) -> String
```

<!-- stdlib:sig:string.reverse end -->

按字符反转。

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.reverse("abc") == "cba")
}
```

### format

<!-- stdlib:sig:string.format start -->

```yaoxiang
format: (format: &String, ...args) -> String
```

<!-- stdlib:sig:string.format end -->

按 `{index}` 占位符格式化，可选宽度/对齐说明符。

占位符语法：

| 形态     | 含义                                        |
| -------- | ------------------------------------------- |
| `{0}`    | 第 0 个参数（`format` 之后的参数从 0 编号） |
| `{0:03}` | 宽度 3                                      |
| `{0:>3}` | 宽度 3，右对齐（默认）                      |
| `{0:<3}` | 宽度 3，左对齐                              |
| `{0:^3}` | 宽度 3，居中                                |

字面量花括号用双写表示：两个左花括号得一个字面左花括号，两个右花括号同理。

返回值：格式化后的字符串。参数会先被转成字符串（同 `convert.to_string`）；下标越界取空串，非法宽度按
`0` 处理。

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.format("{0}-{1}", "a", "b") == "a-b")
    assert(string.format("[{0:>5}]", "ab") == "[   ab]")
    assert(string.format("[{0:<5}]", "ab") == "[ab   ]")
}
```

### parse_int

<!-- stdlib:sig:string.parse_int start -->

```yaoxiang
parse_int: (s: &String) -> Result(Int, Error)
```

<!-- stdlib:sig:string.parse_int end -->

解析十进制整数（自动去除首尾空白）。

返回：成功为 `Result.ok(Int)`；失败为 `Result.err(Error)`，其 `code` 为 `E6010`。**不抛错**。

```yaoxiang
use std.assert
use std.result
use std.string

main: () -> Void = {
    assert(result.is_ok(string.parse_int("42")))
    assert(result.is_err(string.parse_int("abc")))
}
```

### parse_float

<!-- stdlib:sig:string.parse_float start -->

```yaoxiang
parse_float: (s: &String) -> Result(Float, Error)
```

<!-- stdlib:sig:string.parse_float end -->

解析浮点数（自动去除首尾空白）。

返回：成功为 `Result.ok(Float)`；失败为 `Result.err(Error)`，其 `code` 为 `E6011`。

```yaoxiang
use std.assert
use std.result
use std.string

main: () -> Void = {
    assert(result.is_ok(string.parse_float("3.14")))
    assert(result.is_err(string.parse_float("xxx")))
}
```

## 相关

- [`std.convert`](./convert) —— 数值转字符串
- [`std.result`](./result) —— 解包 `parse_*` 的结果

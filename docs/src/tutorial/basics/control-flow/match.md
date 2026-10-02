---
title: 'match 基础'
---

# match 基础

`match`
是 YaoXiang 最强大的控制流结构。它让你根据一个**值**的形态，选择不同的处理路径。如果你用过其他语言的
`switch`，你会发现 `match` 是一个全面升级的版本。

## 基本语法

语法规范中 `match` 表达式的定义：

```
match Expr { MatchArm+ }
MatchArm : Pattern ('|' Pattern)* ('if' Expr)? '=>' Expr ','
```

分解来看：

- `match` 后面跟一个要匹配的值
- `{}` 中包含一个或多个 **匹配臂**（MatchArm）
- 每个匹配臂：**模式** 后面跟 `=>`，然后是**结果表达式**
- 每个臂用逗号结尾

一个最简单的例子：

```yaoxiang
number = 2

text = match number {
    0 => "零",
    1 => "一",
    2 => "二",
}
print(text)  // "二"
```

## match 是表达式

和 `if` 一样，`match` 也是**表达式**——它计算出一个值。所有匹配臂的返回值类型必须一致：

```yaoxiang
score = 85

// 注意：0.8.2 的模式解析器不支持范围模式——`80..89 => "B"` 报
// 「Expected FatArrow, found DotDot」(E0010)。用卫表达式（`n if 条件`）写等价逻辑。
grade = match score {
    n if n >= 90 => "A",
    n if n >= 80 => "B",
    n if n >= 70 => "C",
    n if n >= 60 => "D",
    _ => "F",          // 通配符：匹配所有剩余情况
}
print(grade)  // "B"
```

> **注意**：范围模式 `80..89` 在 0.8.2 尚不可用，请用上面的卫表达式写法。
> 卫表达式（`pattern if 条件`）会在 [模式匹配进阶](../pattern-matching/index.md)
> 中深入讲解。本章先聚焦于基础模式。

## 基础模式

### 字面量模式

用具体的值来匹配：

```yaoxiang
response = 404

message = match response {
    200 => "OK",
    301 => "Moved",
    404 => "Not Found",
    500 => "Server Error",
    _ => "Unknown",
}
print(message)  // "Not Found"
```

### 标识符模式

用变量名来捕获匹配到的值：

```yaoxiang
// Result 变体构造器必须「类型限定」：裸写 ok(42) 报 E1001
use std.result

r: Result(Int, String) = Result(Int, String).ok(42)

description = match r {
    ok(value) => "成功，值是: {value}",
    err(e) => "失败，原因: " + e,
}
print(description)  // "成功，值是: 42"
```

`ok(value)` 中的 `value` 是一个标识符模式——它捕获了 `ok` 包裹的实际值，你可以在 `=>`
后面的表达式里使用它。

### 通配符模式

`_` 是通配符，匹配**任何值**。通常放在最后，作为兜底：

```yaoxiang
command = "exit"

action = match command {
    "start" => "启动服务",
    "stop" => "停止服务",
    "restart" => "重启服务",
    _ => "未知指令: " + command,
}
print(action)  // "未知指令: exit"
```

## 匹配必须穷尽

YaoXiang 的 `match` 要求覆盖所有可能的情况——如果编译器发现你漏掉了某些可能的值，会直接报错。这是
`match` 安全性的体现。

```yaoxiang
// 这段代码会编译失败
// value = true
// result = match value {
//     true => "是",
//     // 缺少 false 分支——编译错误！
// }

// 正确——使用 _ 兜底
value = true
result = match value {
    true => "是",
    _ => "否",      // _ 确保 false 也有处理
}
```

当你明确知道只有有限的几种情况时（比如匹配枚举），编译器会帮你检查是否每个变体都覆盖到了。这是防止漏写分支bug的利器。

## 多模式组合

一个匹配臂可以匹配多个模式，用 `|` 分隔：

```yaoxiang
day = "sunday"

type = match day {
    "monday" | "tuesday" | "wednesday" | "thursday" | "friday" => "工作日",
    "saturday" | "sunday" => "休息日",
    _ => "无效",
}
print(type)  // "休息日"
```

## 匹配臂按顺序执行

`match` 从第一个臂开始尝试匹配，**第一个匹配成功的分支会生效**，后面的不会被执行：

<!-- docs-example: skip -->
```yaoxiang
number = 5

result = match number {
    _ => "其他",     // 通配符匹配一切，这里会匹配
    5 => "五",       // 永远不会被执行——上面已经匹配了
}
print(result)  // "其他"
```

> 上面这段**故意不通过检查**——编译器会报 `E1031 Unreachable pattern`。它用真实诊断演示
> 「第一个匹配的臂生效」这条规则。

这个特性意味着**把通配符 `_` 放在最后**是一个好习惯。

## 小结

| 要点       | 说明                                   |
| ---------- | -------------------------------------- |
| 语法       | `match 值 { 模式 => 表达式, ... }`     |
| 表达式     | `match` 计算出一个值，所有分支类型一致 |
| 字面量模式 | 精确匹配具体值：`200 => "OK"`          |
| 标识符模式 | 捕获值到变量：`ok(value) => ...`       |
| 通配符 `_` | 匹配任何值，做兜底                     |
| 穷尽性     | 必须覆盖所有可能，编译器会检查         |
| 多模式     | `模式1 \| 模式2 => 表达式`             |
| 顺序执行   | 从上到下，第一个匹配的分支生效         |

> **下一步**：本文覆盖了 `match` 的基础用法。更高级的模式（嵌套模式、卫表达式、结构体解构等）请参阅
> [模式匹配进阶](../pattern-matching/index.md)。

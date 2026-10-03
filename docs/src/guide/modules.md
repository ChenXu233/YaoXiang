---
title: '模块系统'
description: 'use 导入、模块定义与目录组织'
---

# 模块系统

YaoXiang 没有 `import` 关键字，也没有「先声明再导出」的模块声明。**一个 `.yx` 文件就是一个模块**，`use` 把另一个模块的绑定拿进来用。

本页讲怎么用。设计裁定见[模块系统规范](../reference/language-spec/modules)。

## 60 秒上手

```yaoxiang
use std.io
use std.math.{sqrt}

// 整模块导入 → 用命名空间访问
io.println("hello")

// 指定条目导入 → 直接裸用
result = sqrt(16.0)
```

## 导入的四种写法

### 1. 导入整个模块

```yaoxiang
use std.io

main = () => {
    io.println("用 io. 前缀访问")
    io.print("print 和 println 都在 std.io 里")
}
```

`use std.io` 引入的是**命名空间**，成员要用 `io.` 前缀访问。

### 2. 只导入指定条目

花括号里列出的名字会被直接绑到当前作用域，**不带前缀**：

```yaoxiang
use std.math.{sqrt, sin, cos}

main = () => {
    print(sqrt(16.0))
    print(sin(0.0))
}
```

这种写法适合只用到模块里少数几个函数时——既省去反复写前缀，也避免把整个命名空间拖进来。

### 3. 条目内联别名

重命名写在花括号**里面**：

```yaoxiang
use std.io.{print as say}

main = () => {
    say("用 say 代替 print")
}
```

::: warning 别名要写在花括号内
这是支持的写法：

```yaoxiang
use std.io.{print as say}   // ✅
```

下面这个会被拒绝，编译器会提示你改用内联形式：

<!-- docs-example: skip -->

```yaoxiang
use std.io.{print} as say   // ❌ 位置式别名，不受支持
```

:::

### 4. 模块别名

<!-- docs-example: skip -->

```yaoxiang
use helper as h

main = () => {
    h.shout("通过 h 访问 helper")
}
```

::: danger 已知问题：对标准库模块使用别名会崩溃
`use <项目内模块> as <别名>` 可用，但 `use <标准库模块> as <别名>` 目前会触发编译器内部错误（`E8001` 闭包函数未在函数名映射表中注册）：

```yaoxiang
use std.math as m   // ❌ 调用 m.sqrt(...) 时报 E8001
```

**替代写法**：改用条目内联别名，它对标准库模块正常工作：

```yaoxiang
use std.math.{sqrt, sin, cos}   // ✅
```

该问题已单独记录，不影响上面三种写法的使用。
:::

## 模块的目录组织

### mod.yx 是目录入口

一个目录要成为模块，入口文件必须叫 `mod.yx`：

```
src/
├── main.yx
├── math/
│   ├── mod.yx       ← 目录入口，模块 math
│   └── vector.yx    ← 模块 math.vector
└── utils/
    ├── mod.yx       ← 目录入口，模块 utils
    └── string.yx    ← 模块 utils.string
```

<!-- docs-example: skip -->

```yaoxiang
// src/helper/mod.yx
use std.io

shout: (msg: string) -> Void = {
    io.print(msg)
}

label: () -> string = "src/helper/mod.yx"
```

<!-- docs-example: skip -->

```yaoxiang
// src/main.yx
use std.io
use helper

main = () => {
    io.print(helper.label())
    helper.shout("跨模块调用成功")
}
```

### index.yx 不被识别

这是从 Rust 过来最容易踩的坑。写成 `index.yx` 时解析不到：

<!-- docs-example: skip -->

```yaoxiang
// src/helper/index.yx
label: () => string = "这样写会失败"
```

<!-- docs-example: skip -->

```yaoxiang
use helper   // ❌ E5001: module 'helper' not found
```

**原因**：路径映射只认 `name.yx` 与 `name/mod.yx` 两种形态，`index.yx` 不在其中。`mod.yx` 是约定而非强制——`helper.yx` 同样能承载模块 `helper`。

## 导出：不需要 pub

**本语言没有可见性机制。** 没有 `pub`，没有 `private`，没有 `export`。

模块的每个顶层绑定，对所有能写出该模块路径的代码可见：

<!-- docs-example: skip -->

```yaoxiang
// src/helper/mod.yx
// plain 前面没有任何修饰
plain: () => string = "没有 pub 也能被导入"
```

<!-- docs-example: skip -->

```yaoxiang
use helper
// helper.plain() 可直接调用，无需 pub
```

想「限制」某个绑定不被外部使用，唯一的办法是不把它放在模块顶层。

> **`pub` 的历史**：早期规范教过「用 `pub` 声明导出项、默认私有」。该设计已被 RFC-029 推翻、RFC-029g 裁定删除 `pub` 关键字。`pub` 从不参与导出判定——写不写它，不影响能否被导入。详见[规范 §3](../reference/language-spec/modules#第三章导出面)。

## 方法要显式绑定

方法不是靠 `pub` 自动生成的，而是显式写在类型上：

<!-- docs-example: skip -->

```yaoxiang
Point: Type = { x: Float, y: Float }

// 显式方法：第一个参数是 self
Point.distance: (self: &Point, other: &Point) -> Float = {
    dx = self.x - other.x
    dy = self.y - other.y
    (dx * dx + dy * dy).sqrt()
}
```

## 排错

### E5001:module not found

按可能性从高到低排查：

1. **目录入口写成了 `index.yx`** — 改成 `mod.yx`，这是最常见的原因
2. **路径拼写或层级不对** — `use math.vector` 对应 `src/math/vector.yx`
3. **依赖没装** — 错误信息会提示运行 `yx install`
4. **文件在项目外** — 查找起点只有「导入者所在目录」和「项目根」两处

### E5003:export not found

模块找到了，但里面没有你要的名字。常见是拼写差异，例如 `std.io` 里是 `read_line` 而不是 `read`：

<!-- docs-example: skip -->

```yaoxiang
use std.io.{read_line}   // ✅
use std.io.{read}        // ❌ E5003
```

## 速查

| 写法                             | 效果                        |
| -------------------------------- | --------------------------- |
| `use std.io`                     | 命名空间 `io`，用 `io.x` 访问 |
| `use std.math.{sqrt}`            | 裸用 `sqrt`                 |
| `use std.io.{print as say}`      | 裸用 `say`                  |
| `use helper as h`                | 命名空间 `h`，用 `h.x` 访问  |
| `use std.io.{print} as say`      | 语法错误，改用内联形式      |
| `use .relative`                  | 语法错误，路径必须从模块根写全 |
| `pub fn foo`                     | 语义无效，可见性不存在      |

---
title: "特殊语法规则"
description: F-String、导入语句、错误处理、Unsafe 块的格式化规则
---

# 特殊语法规则

---

## §13 F-String

**§13.1 F-String 格式。** F-String 使用 `f"..."` 格式，插值使用 `{expr}`。

```
// ✅ 正确
let msg = f"Hello, {name}!";
let msg = f"Result: {x + y}";
```

**§13.2 格式化规格。** F-String 支持格式化规格 `{expr:spec}`。

```
// ✅ 正确
let msg = f"{value:.2f}";
```

---

## §14 导入语句

**§14.1 导入排序。** 当 `sort_imports = true` 时，导入语句按以下顺序排序：

1. 标准库（`std`, `core`, `alloc`）
2. 外部 crate
3. 相对路径（`.` 或 `..` 开头）

**§14.2 组内排序。** 同一组内的导入按字母顺序排序。

```
// 排序前
use z_crate;
use std::collections;
use a_crate;
use ./local;

// 排序后
use std::collections;
use a_crate;
use z_crate;
use ./local;
```

---

## §17 错误处理

**§17.1 Try 操作符。** 使用 `expr?` 格式。

```
// ✅ 正确
let x = foo()?;

// ❌ 错误
let x = foo() ?;
```

---

## §18 Unsafe 块

**§18.1 Unsafe 格式。** 使用 `unsafe { ... }` 格式。

```
// ✅ 正确
let x = unsafe { dangerous_function() };

// ❌ 错误
let x = unsafe{ dangerous_function() };
```

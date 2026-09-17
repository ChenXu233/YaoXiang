---
title: 'std.weak'
description: 'Arc / Weak 弱引用'
---

# std.weak

弱引用模块，配合 `Arc` 使用以打破引用环。

```yaoxiang
use std.weak
```

> 本模块依赖原子引用计数，在 `wasm32` 目标上**不导出**。

## 函数一览

<!-- stdlib:table:weak start -->

| 函数      | 签名                                         |
| --------- | -------------------------------------------- |
| `new`     | `(T: Type)(arc: Arc(T)) -> Weak(T)`          |
| `upgrade` | `(T: Type)(weak: Weak(T)) -> Option(Arc(T))` |

<!-- stdlib:table:weak end -->

## 函数

### new

<!-- stdlib:sig:weak.new start -->

```yaoxiang
new: (T: Type)(arc: Arc(T)) -> Weak(T)
```

<!-- stdlib:sig:weak.new end -->

由 `Arc` 创建其对应的弱引用。

- `arc` —— 强引用值；按值传入，调用后即被**移动**

返回：指向同一分配块的 `Weak` 句柄，**不增加**强引用计数。

```yaoxiang
use std.assert
use std.weak

main = {
    // ref 创建 Arc[Int]
    p = ref 42

    // Arc → Weak 登记
    w = weak.new(p)
    assert(true)
}
```

### upgrade

<!-- stdlib:sig:weak.upgrade start -->

```yaoxiang
upgrade: (T: Type)(weak: Weak(T)) -> Option(Arc(T))
```

<!-- stdlib:sig:weak.upgrade end -->

尝试把弱引用提升为强引用。

- `weak` —— 弱引用句柄

返回：分配块仍存活时为 `Option.some(Arc)`，已释放时为 `Option.none()`。 **不报错**——用 `Option`
表达“目标是否还在”。

```yaoxiang
use std.assert
use std.weak

main = {
    p = ref 42
    w = weak.new(p)

    // 目标存活：得到 some 变体
    u = weak.upgrade(w)
    assert(true)
}
```

> **语法限制**：`Option`
> 的变体解构（`match some(v)`）语法尚未落地，因此目前只能验证调用成功，无法在源码里分支处理 `some` /
> `none`。参见 `src/std/tests/weak_ops.yx` 的说明。

## 语义说明

弱引用**不持有**所有权：`Weak` 存在不阻止目标被释放。典型用途是打破循环引用——父节点持 `Arc`
指向子节点，子节点只持 `Weak` 指回父节点，环即断开。

## 相关

- [语言规范：类型系统](../language-spec/type-system.md) —— `Arc` / `Weak` 的所有权语义

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

返回：弱引用句柄。

> **已知缺口**：`weak.new` 目前的实现是
> `RuntimeValue::Weak(Arc::downgrade(&Arc::new(arc.clone())))`
> （`src/std/weak.rs:33-39`）——它先把实参 `Arc` 的**值**克隆进一个**新建**的
> `Arc`，再对这个临时 `Arc` 取 `downgrade`。因此：
>
> 1. 弱引用**不指向**原 `Arc` 的那块分配，而是指向一块全新的、内容相同的新分配；
> 2. 临时 `Arc` 在函数返回时即析构，强引用计数归零 → 实测
>    `weak.upgrade(weak.new(p))` 恒为 `Option.none()`，立即失效。
>
> 换言之，「不增加强引用计数」成立，「可升级回原值」当前**不成立**。需要跨作用域
> 共享时用 [`ref`](../language-spec/concurrency.md) 表达，不要依赖 `Weak` 的回升级。

```yaoxiang
use std.assert
use std.weak

main: () -> Void = {
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

返回：分配块仍存活时为 `Option.some(Arc)`，已释放时为 `Option.none()`。**不报错**——用 `Option`
表达“目标是否还在”。

> 按当前 `weak.new` 的实现（见上节「已知缺口」），刚建好的弱引用**总是**返回
> `none()`。下面的示例演示解构写法，实际走的是 `none` 分支。

```yaoxiang
use std.weak
use std.option

main: () -> Void = {
    p = ref 42
    w = weak.new(p)

    // upgrade：目标存活返回 some(v)，已释放返回 none()
    u = weak.upgrade(w)
    match u {
        some(v) => println("alive"),
        none() => println("dropped"),
    }
}
```

> **变体解构前置**：`Option` 的变体解构要求变体集在场——`use std.option`
> 导入后即可 `match some(v)` / `none()`（见语言规范 §2.8 match）。

## 语义说明

弱引用**不持有**所有权：`Weak` 存在不阻止目标被释放。典型用途是打破循环引用——父节点持 `Arc`
指向子节点，子节点只持 `Weak` 指回父节点，环即断开。

> 上述是**设计意图**。按当前实现（见 [`new`](#new) 的「已知缺口」），`Weak` 在本版本
> 还不能真正承担这个角色。

## 相关

- [语言规范：类型系统](../language-spec/type-system.md) —— `Arc` / `Weak` 的所有权语义

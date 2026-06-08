---
title: "类型系统格式化规则"
description: 类型注解、引用和借用、类型转换的格式化规则
---

# 类型系统格式化规则

---

## §9 类型注解

**§9.1 变量类型注解。** 类型注解使用 `: Type` 格式，冒号后有一个空格。

```
// ✅ 正确
let x: Int = 1;

// ❌ 错误
let x:Int = 1;
let x : Int = 1;
```

**§9.2 函数参数类型。** 参数名与类型之间使用 `: ` 连接。

```
// ✅ 正确
fn foo(x: Int, y: String) { ... }

// ❌ 错误
fn foo(x:Int, y:String) { ... }
```

**§9.3 泛型参数。** 泛型参数使用 `(T: Constraint)` 格式。

```
// ✅ 正确
fn foo<T: Clone>(x: T) { ... }

// ❌ 错误
fn foo <T:Clone> (x: T) { ... }
```

---

## §15 引用和借用

**§15.1 不可变引用。** 使用 `&expr` 格式。

```
// ✅ 正确
let x = &value;

// ❌ 错误
let x = & value;
```

**§15.2 可变引用。** 使用 `&mut expr` 格式。

```
// ✅ 正确
let x = &mut value;

// ❌ 错误
let x = &mut  value;
let x = & mut value;
```

**§15.3 类型中的引用。** 类型中的引用使用 `&Type` 或 `&mut Type` 格式。

```
// ✅ 正确
fn foo(x: &Int) { ... }
fn bar(x: &mut Int) { ... }
```

---

## §16 类型转换

**§16.1 as 转换。** 使用 `expr as Type` 格式。

```
// ✅ 正确
let x = value as Int;

// ❌ 错误
let x = value as Int;
let x = value  as  Int;
```

---

## §17 Ref 关键字

**§17.1 Ref 格式。** `ref` 关键字与表达式之间用空格分隔。

```
// ✅ 正确
let x = ref value;
let y = ref obj;

// ❌ 错误
let x = refvalue;  // 缺少空格
let y = ref  value;  // 多余空格
```

**§17.2 Ref 语义。** `ref` 创建 Arc（原子引用计数）副本。

```
// 创建共享引用
let shared = ref original;
```

---
title: "函数相关格式化规则"
description: 函数定义、函数调用、Lambda 表达式的格式化规则
---

# 函数相关格式化规则

---

## §4 函数定义

**§4.1 函数签名。** 函数名与参数列表之间不加空格。

```
// ✅ 正确
foo: (a: Int, b: Int) -> Int = a + b

// ❌ 错误
foo : (a: Int, b: Int) -> Int = a + b
```

**§4.2 参数列表换行。** 当参数列表超过行宽时，每个参数占一行，尾随逗号。

```
// 超过行宽时
very_long_function_name: (first_param: Int, second_param: Int, third_param: Int) -> Int = first_param + second_param + third_param

// 格式化后
very_long_function_name:
    first_param: Int,
    second_param: Int,
    third_param: Int,
) -> Int = first_param + second_param + third_param
```

**§4.3 返回类型。** 返回类型与参数列表之间用 ` -> ` 连接，` ->` 前后各有一个空格。

```
// ✅ 正确
foo: () -> Int = 1

// ❌ 错误
foo: () ->Int = 1
foo: ()-> Int = 1
foo:()-> Int = 1
```

**§4.4 函数体。** 函数体与返回类型之间用一个空格分隔。

```
// ✅ 正确
foo: () -> Int = 1

// ❌ 错误（两个空格）
foo: () -> Int  = 1
```

---

## §7 函数调用

**§7.1 参数列表。** 参数之间用逗号分隔，逗号后有一个空格。

```
// ✅ 正确
foo(1, 2, 3)

// ❌ 错误
foo(1,2,3)
foo(1 , 2 , 3)
```

**§7.2 命名参数。** 命名参数使用 `name = value` 格式。

```
// ✅ 正确
foo(x = 1, y = 2)

// ❌ 错误
foo(x=1, y=2)
```

**§7.3 参数换行。** 当参数列表超过行宽时，每个参数占一行，尾随逗号。

```
// 超过行宽时
very_long_function_name(first_argument, second_argument, third_argument)

// 格式化后
very_long_function_name(
    first_argument,
    second_argument,
    third_argument,
)
```

---

## §12 Lambda 表达式

**§12.1 Lambda 格式。** Lambda 使用 `(params) => body` 格式。

```
// ✅ 正确
f = (x) => x + 1

// 单表达式 body
f = (x) => x * 2

// 多语句 body
f = (x) => {
    y = x + 1
    y * 2
}
```

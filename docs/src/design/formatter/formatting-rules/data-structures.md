---
title: "数据结构格式化规则"
description: 字面量、列表和字典、Match 表达式的格式化规则
---

# 数据结构格式化规则

---

## §8 字面量

**§8.1 整数字面量。** 整数字面量直接输出。

```
// ✅ 正确
let x = 42;
```

**§8.2 浮点字面量。** 浮点字面量必须包含小数点。

```
// ✅ 正确
let x = 3.14;
let y = 42.0;  // 必须有小数点

// ❌ 错误
let y = 42;    // 整数，不是浮点数
```

**§8.3 字符串字面量。** 默认使用双引号。当 `single_quote = true` 时，使用单引号。

```
// 默认（双引号）
let s = "hello";

// single_quote = true
let s = 'hello';
```

**§8.4 布尔字面量。** 布尔字面量使用小写。

```
// ✅ 正确
let x = true;
let y = false;

// ❌ 错误
let x = True;
let y = FALSE;
```

---

## §10 列表和字典

**§10.1 列表格式。** 列表使用 `[]` 包围，元素之间用逗号分隔。

```
// ✅ 正确
let x = [1, 2, 3];

// ❌ 错误
let x = [1,2,3];
```

**§10.2 字典格式。** 字典使用 `{}` 包围，键值对使用 `key: value` 格式。

```
// ✅ 正确
let x = {"a": 1, "b": 2};

// ❌ 错误
let x = {"a":1, "b":2};
```

**§10.3 列表推导式。** 列表推导式使用 `[expr for var in iterable]` 格式。

```
// ✅ 正确
let x = [i * 2 for i in range(10)];

// 带条件
let x = [i for i in range(10) if i > 5];
```

---

## §11 Match 表达式

**§11.1 Match 格式。** `match` 关键字与表达式之间用空格分隔。

```
// ✅ 正确
match x { ... }

// ❌ 错误
match(x) { ... }
```

**§11.2 Pattern 对齐。** 多个 pattern 应该对齐，使用空格填充。

```
// ✅ 对齐
match x {
    1    => "one",
    2    => "two",
    100  => "hundred",
    _    => "other",
}
```

**§11.3 Pattern 过长换行。** 当 pattern 过长时，pattern 换行，`=>` 与 body 对齐。

```
// ✅ 换行
match x {
    VeryLongPatternName { field1, field2 }
        => handle_case(field1, field2),
    _ => default_case(),
}
```

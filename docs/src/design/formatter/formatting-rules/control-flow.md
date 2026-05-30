---
title: "控制流格式化规则"
description: if/elif/else、for 循环、while 循环、循环标签的格式化规则
---

# 控制流格式化规则

---

## §5 控制流

**§5.1 if 表达式。** `if` 关键字与条件之间用空格分隔，条件与代码块之间用空格分隔。

```
// ✅ 正确
if condition { ... }

// ❌ 错误
if(condition) { ... }
if condition{ ... }
```

**§5.2 elif/else。** `elif` 和 `else` 与前一个代码块之间用空格分隔。

```
// ✅ 正确
if a > 0 { ... } elif a < 0 { ... } else { ... }

// ❌ 错误
if a > 0 { ... }elif a < 0 { ... }else { ... }
```

**§5.3 for 循环。** `for` 关键字、变量、`in` 关键字、迭代器之间用空格分隔。

```
// ✅ 正确
for item in collection { ... }

// ❌ 错误
for item in(collection) { ... }
for(item) in collection { ... }
```

**§5.4 while 循环。** `while` 关键字与条件之间用空格分隔。

```
// ✅ 正确
while condition { ... }

// ❌ 错误
while(condition) { ... }
```

**§5.5 循环标签。** 标签与循环关键字之间用 `: ` 连接。

```
// ✅ 正确
'outer: for i in range(10) { ... }

// ❌ 错误
'outer:for i in range(10) { ... }
'outer : for i in range(10) { ... }
```

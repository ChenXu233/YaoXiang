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

---

## §5.6 Return 语句

**§5.6.1 Return 格式。** `return` 关键字与表达式之间用空格分隔。

```
// ✅ 正确
return 42;
return x + y;

// ❌ 错误
return(42);  // 缺少空格
return  42;  // 多余空格
```

**§5.6.2 空 Return。** 空 return 直接使用 `return` 关键字。

```
// ✅ 正确
return;

// ❌ 错误
return ;  // 多余空格
return void;  // 不需要 void
```

---

## §5.7 Break 语句

**§5.7.1 Break 格式。** `break` 关键字与标签之间用空格分隔。

```
// ✅ 正确
break;
break 'outer;

// ❌ 错误
break(outer);  // 错误语法
break  'outer;  // 多余空格
```

---

## §5.8 Continue 语句

**§5.8.1 Continue 格式。** `continue` 关键字与标签之间用空格分隔。

```
// ✅ 正确
continue;
continue 'outer;

// ❌ 错误
continue(outer);  // 错误语法
continue  'outer;  // 多余空格
```

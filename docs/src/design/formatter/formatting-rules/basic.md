---
title: "基础格式化规则"
description: 缩进、行宽、运算符、代码块的格式化规则
---

# 基础格式化规则

---

## §1 缩进

**§1.1 缩进宽度。** 默认使用 4 个空格缩进。可通过 `indent_width` 配置项修改。

```
// 默认缩进（4 空格）
fn foo() {
    let x = 1;
    if x > 0 {
        print(x);
    }
}

// 2 空格缩进（indent_width = 2）
fn foo() {
  let x = 1;
  if x > 0 {
    print(x);
  }
}
```

**§1.2 Tab 缩进。** 当 `use_tabs = true` 时，使用 tab 字符缩进。默认为 `false`。

**§1.3 缩进一致性。** 同一文件内不得混用 tab 和空格。

---

## §2 行宽

**§2.1 最大行宽。** 默认最大行宽为 120 个字符。可通过 `line_width` 配置项修改。

**§2.2 换行策略。** 当一行超过最大行宽时，必须在适当位置换行。换行位置的优先级：

1. 低优先级运算符后（`+`, `-`, `||`, `&&`, `=`）
2. 函数参数列表
3. 列表/字典元素
4. 高优先级运算符后（`*`, `/`, `%`, `==`, `!=`）

**§2.3 换行缩进。** 换行后的内容必须增加一级缩进。

```
// 超过行宽时换行
let result = very_long_variable_name + another_long_name + yet_another_long_name;

// 格式化后
let result = very_long_variable_name
    + another_long_name
    + yet_another_long_name;
```

---

## §3 运算符

**§3.1 运算符空格。** 二元运算符两侧必须有空格。

```
// ✅ 正确
let x = 1 + 2;
let y = a == b;

// ❌ 错误
let x = 1+2;
let y = a==b;
```

**§3.2 一元运算符。** 一元运算符与操作数之间不加空格。

```
// ✅ 正确
let x = -1;
let y = !flag;
let z = *ptr;

// ❌ 错误
let x = - 1;
let y = ! flag;
```

**§3.3 低优先级运算符换行。** 当表达式超过行宽时，低优先级运算符放在新行行首。

```
// 超过行宽时
let result = first_value + second_value + third_value + fourth_value;

// 格式化后
let result = first_value
    + second_value
    + third_value
    + fourth_value;
```

**§3.4 高优先级运算符换行。** 高优先级运算符放在新行行首。

```
// 超过行宽时
let result = first_value * second_value / third_value % fourth_value;

// 格式化后
let result = first_value
    * second_value
    / third_value
    % fourth_value;
```

---

## §6 代码块

**§6.1 代码块格式。** 代码块使用花括号 `{}` 包围，开括号前有一个空格。

```
// ✅ 正确
fn foo() {
    let x = 1;
}

// ❌ 错误
fn foo(){
    let x = 1;
}
fn foo()
{
    let x = 1;
}
```

**§6.2 单行代码块。** 当代码块只有一行且总长度不超过行宽时，可以使用单行格式。

```
// ✅ 单行格式
fn foo() { 1 }

// ✅ 多行格式
fn foo() {
    let x = 1;
    let y = 2;
    x + y
}
```

**§6.3 空代码块。** 空代码块使用 `{}` 表示。

```
// ✅ 正确
fn foo() {}

// ❌ 错误
fn foo() {
}
```

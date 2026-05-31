---
title: "Basic Formatting Rules"
description: "Formatting rules for indentation, line width, operators, and code blocks"
---

# Basic Formatting Rules

---

## §1 Indentation

**§1.1 Indentation width.** Default uses 4 spaces for indentation. Can be modified via the `indent_width` configuration option.

```
// Default indentation (4 spaces)
fn foo() {
    let x = 1;
    if x > 0 {
        print(x);
    }
}

// 2-space indentation (indent_width = 2)
fn foo() {
  let x = 1;
  if x > 0 {
    print(x);
  }
}
```

**§1.2 Tab indentation.** When `use_tabs = true`, use tab characters for indentation. Defaults to `false`.

**§1.3 Indentation consistency.** Tabs and spaces must not be mixed within the same file.

---

## §2 Line Width

**§2.1 Maximum line width.** Default maximum line width is 120 characters. Can be modified via the `line_width` configuration option.

**§2.2 Line break strategy.** When a line exceeds the maximum line width, it must be broken at an appropriate position. Priority for line break locations:

1. After low-precedence operators (`+`, `-`, `||`, `&&`, `=`)
2. Function parameter lists
3. List/dictionary elements
4. After high-precedence operators (`*`, `/`, `%`, `==`, `!=`)

**§2.3 Line break indentation.** Content after a line break must be indented one additional level.

```
// Line break when exceeding line width
let result = very_long_variable_name + another_long_name + yet_another_long_name;

// After formatting
let result = very_long_variable_name
    + another_long_name
    + yet_another_long_name;
```

---

## §3 Operators

**§3.1 Operator spacing.** Binary operators must have spaces on both sides.

```
// ✅ Correct
let x = 1 + 2;
let y = a == b;

// ❌ Incorrect
let x = 1+2;
let y = a==b;
```

**§3.2 Unary operators.** No space between unary operators and operands.

```
// ✅ Correct
let x = -1;
let y = !flag;
let z = *ptr;

// ❌ Incorrect
let x = - 1;
let y = ! flag;
```

**§3.3 Low-precedence operator line breaks.** When an expression exceeds the line width, place low-precedence operators at the beginning of the new line.

```
// When exceeding line width
let result = first_value + second_value + third_value + fourth_value;

// After formatting
let result = first_value
    + second_value
    + third_value
    + fourth_value;
```

**§3.4 High-precedence operator line breaks.** Place high-precedence operators at the beginning of the new line.

```
// When exceeding line width
let result = first_value * second_value / third_value % fourth_value;

// After formatting
let result = first_value
    * second_value
    / third_value
    % fourth_value;
```

---

## §6 Code Blocks

**§6.1 Code block format.** Code blocks are enclosed with curly braces `{}`, with a space before the opening brace.

```
// ✅ Correct
fn foo() {
    let x = 1;
}

// ❌ Incorrect
fn foo(){
    let x = 1;
}
fn foo()
{
    let x = 1;
}
```

**§6.2 Single-line code blocks.** When a code block contains only one statement and the total length does not exceed the line width, the single-line format may be used.

```
// ✅ Single-line format
fn foo() { 1 }

// ✅ Multi-line format
fn foo() {
    let x = 1;
    let y = 2;
    x + y
}
```

**§6.3 Empty code blocks.** Empty code blocks are represented with `{}`.

```
// ✅ Correct
fn foo() {}

// ❌ Incorrect
fn foo() {
}
```
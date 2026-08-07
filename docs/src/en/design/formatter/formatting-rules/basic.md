---
title: 'Basic Formatting Rules'
description: Formatting rules for indentation, line width, operators, and code blocks
---

# Basic Formatting Rules

---

## §1 Indentation

**§1.1 Indent width.** The default indentation is 4 spaces. This can be modified via the
`indent_width` configuration option.

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

**§1.2 Tab indentation.** When `use_tabs = true`, tab characters are used for indentation. The
default is `false`.

**§1.3 Indentation consistency.** Tabs and spaces must not be mixed within the same file.

---

## §2 Line Width

**§2.1 Maximum line width.** The default maximum line width is 120 characters. This can be modified
via the `line_width` configuration option.

**§2.2 Line-breaking strategy.** When a line exceeds the maximum line width, it must be broken at an
appropriate position. The priority of line-break positions is:

1. After low-priority operators (`+`, `-`, `or`, `and`, `=`)
2. Function parameter lists
3. List/dictionary elements
4. After high-priority operators (`*`, `/`, `%`, `==`, `!=`)

**§2.3 Line-break indentation.** Content after a line break must increase indentation by one level.

```
// Exceeds line width
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

**§3.2 Unary operators.** No space is added between a unary operator and its operand.

```
// ✅ Correct (! is a tightly-bound unary operator, no space)
let x = -1;
let y = !flag;
let z = *ptr;

// ❌ Incorrect
let x = - 1;
let y = ! flag;
```

**§3.3 Line-breaking at low-priority operators.** When an expression exceeds the line width, place
low-priority operators at the beginning of the new line.

```
// Exceeds line width
let result = first_value + second_value + third_value + fourth_value;

// After formatting
let result = first_value
    + second_value
    + third_value
    + fourth_value;
```

**§3.4 Line-breaking at high-priority operators.** Place high-priority operators at the beginning of
the new line.

```
// Exceeds line width
let result = first_value * second_value / third_value % fourth_value;

// After formatting
let result = first_value
    * second_value
    / third_value
    % fourth_value;
```

---

## §3.5 Variable References

**§3.5.1 Variable names.** Variable references output the variable name directly, without adding
extra spaces.

```
// ✅ Correct
let x = my_variable;
let y = camelCaseName;

// ❌ Incorrect
let x = my_variable ;  // Extra space
let y = "camelCaseName";  // Should not be quoted
```

---

## §6 Code Blocks

**§6.1 Code block format.** Code blocks are enclosed in curly braces `{}`, with a single space
before the opening brace.

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

**§6.2 Single-line code blocks.** When a code block contains only one line and the total length does
not exceed the line width, the single-line format may be used.

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

**§6.3 Empty code blocks.** Empty code blocks are represented using `{}`.

```
// ✅ Correct
fn foo() {}

// ❌ Incorrect
fn foo() {
}
```

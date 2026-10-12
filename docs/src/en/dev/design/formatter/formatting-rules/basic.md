---
title: 'Basic Formatting Rules'
description: 'Formatting rules for indentation, line width, operators, and code blocks'
---

# Basic Formatting Rules

---

## §1 Indentation

**§1.1 Indent Width.** Use 4 spaces for indentation by default. This can be modified via the
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

**§1.2 Tab Indentation.** When `use_tabs = true`, use tab characters for indentation. Default is
`false`.

**§1.3 Indentation Consistency.** Do not mix tabs and spaces within the same file.

---

## §2 Line Width

**§2.1 Maximum Line Width.** The default maximum line width is 120 characters. This can be modified
via the `line_width` configuration option.

**§2.2 Line Break Strategy.** When a line exceeds the maximum line width, it must be broken at an
appropriate position. The priority of line break positions is:

1. After low-priority operators (`+`, `-`, `or`, `and`, `=`)
2. Function parameter list
3. List/dictionary elements
4. After high-priority operators (`*`, `/`, `%`, `==`, `!=`)

**§2.3 Line Break Indentation.** Content after a line break must be indented one additional level.

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

**§3.1 Operator Spacing.** There must be spaces on both sides of a binary operator.

```
// ✅ Correct
let x = 1 + 2;
let y = a == b;

// ❌ Incorrect
let x = 1+2;
let y = a==b;
```

**§3.2 Unary Operators.** No space is added between a unary operator and its operand.

```
// ✅ Correct (! is a tightly-bound unary operator with no space)
let x = -1;
let y = !flag;
let z = *ptr;

// ❌ Incorrect
let x = - 1;
let y = ! flag;
```

**§3.3 Low-Priority Operator Line Breaks.** When an expression exceeds the line width, place the
low-priority operator at the beginning of the new line.

```
// When exceeding line width
let result = first_value + second_value + third_value + fourth_value;

// After formatting
let result = first_value
    + second_value
    + third_value
    + fourth_value;
```

**§3.4 High-Priority Operator Line Breaks.** Place the high-priority operator at the beginning of
the new line.

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

## §3.5 Variable Reference

**§3.5.1 Variable Names.** Variable references output the variable name directly, without adding
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

**§6.1 Code Block Format.** Code blocks are enclosed with curly braces `{}`, with one space before
the opening brace.

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

**§6.2 Single-Line Code Block.** When a code block contains only one line and the total length does
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

**§6.3 Empty Code Block.** Use `{}` to represent an empty code block.

```
// ✅ Correct
fn foo() {}

// ❌ Incorrect
fn foo() {
}
```

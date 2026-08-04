---
title: 'Basic Formatting Rules'
description: Formatting rules for indentation, line width, operators, and code blocks
---

# Basic Formatting Rules

---

## §1 Indentation

**§1.1 Indentation Width.** The default indentation is 4 spaces. This can be modified via the
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

**§1.2 Tab Indentation.** When `use_tabs = true`, tab characters are used for indentation. The
default is `false`.

**§1.3 Indentation Consistency.** Tab and space characters must not be mixed within the same file.

---

## §2 Line Width

**§2.1 Maximum Line Width.** The default maximum line width is 120 characters. This can be modified
via the `line_width` configuration option.

**§2.2 Line Break Strategy.** When a line exceeds the maximum line width, it must be wrapped at an
appropriate position. Priority of break positions:

1. After low-precedence operators (`+`, `-`, `or`, `and`, `=`)
2. Function parameter lists
3. List/dictionary elements
4. After high-precedence operators (`*`, `/`, `%`, `==`, `!=`)

**§2.3 Indentation After Line Break.** Content after a line break must increase the indentation by
one level.

```
// Line exceeds line width
let result = very_long_variable_name + another_long_name + yet_another_long_name;

// After formatting
let result = very_long_variable_name
    + another_long_name
    + yet_another_long_name;
```

---

## §3 Operators

**§3.1 Operator Spacing.** Binary operators must have spaces on both sides.

```
// ✅ Correct
let x = 1 + 2;
let y = a == b;

// ❌ Incorrect
let x = 1+2;
let y = a==b;
```

**§3.2 Unary Operators.** No space is added between unary operators and their operands.

```
// ✅ Correct (not is a keyword operator and requires a space)
let x = -1;
let y = not flag;
let z = *ptr;

// ❌ Incorrect
let x = - 1;
let y = not(flag);
```

**§3.3 Line Breaks for Low-Precedence Operators.** When an expression exceeds the line width,
low-precedence operators are placed at the beginning of the new line.

```
// Exceeds line width
let result = first_value + second_value + third_value + fourth_value;

// After formatting
let result = first_value
    + second_value
    + third_value
    + fourth_value;
```

**§3.4 Line Breaks for High-Precedence Operators.** High-precedence operators are placed at the
beginning of the new line.

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

**§3.5.1 Variable Names.** Variable references output the variable name directly, without adding
extra spaces.

```
// ✅ Correct
let x = my_variable;
let y = camelCaseName;

// ❌ Incorrect
let x = my_variable ;  // Extra spaces
let y = "camelCaseName";  // Should not be quoted
```

---

## §6 Code Blocks

**§6.1 Code Block Format.** Code blocks are enclosed in curly braces `{}`, with one space before the
opening brace.

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

**§6.2 Single-Line Code Blocks.** When a code block has only one line and the total length does not
exceed the line width, the single-line format may be used.

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

**§6.3 Empty Code Blocks.** Empty code blocks are represented as `{}`.

```
// ✅ Correct
fn foo() {}

// ❌ Incorrect
fn foo() {
}
```

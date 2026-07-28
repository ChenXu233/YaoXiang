---
title: 'Basic Formatting Rules'
description: 'Formatting rules for indentation, line width, operators, and code blocks'
---

# Basic Formatting Rules

---

## §1 Indentation

**§1.1 Indentation Width.** Default indentation uses 4 spaces. Can be modified via the
`indent_width` configuration option.

```
// Default indentation (4 spaces)
fn foo() {
    let x = 1;
    if x > 0 {
        print(x);
    }
}

// 2 space indentation (indent_width = 2)
fn foo() {
  let x = 1;
  if x > 0 {
    print(x);
  }
}
```

**§1.2 Tab Indentation.** When `use_tabs = true`, use tab characters for indentation. Defaults to
`false`.

**§1.3 Indentation Consistency.** Tabs and spaces must not be mixed within the same file.

---

## §2 Line Width

**§2.1 Maximum Line Width.** Default maximum line width is 120 characters. Can be modified via the
`line_width` configuration option.

**§2.2 Line Break Strategy.** When a line exceeds the maximum line width, it must be broken at an
appropriate position. Priority order for line breaks:

1. After low-priority operators (`+`, `-`, `||`, `&&`, `=`)
2. Function parameter lists
3. List/dictionary elements
4. After high-priority operators (`*`, `/`, `%`, `==`, `!=`)

**§2.3 Line Break Indentation.** Content after a line break must be indented one additional level.

```
// When exceeding line width
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

// ❌ Wrong
let x = 1+2;
let y = a==b;
```

**§3.2 Unary Operators.** No space between unary operators and their operands.

```
// ✅ Correct
let x = -1;
let y = !flag;
let z = *ptr;

// ❌ Wrong
let x = - 1;
let y = ! flag;
```

**§3.3 Low-Priority Operator Line Breaks.** When an expression exceeds line width, low-priority
operators go at the start of the new line.

```
// When exceeding line width
let result = first_value + second_value + third_value + fourth_value;

// After formatting
let result = first_value
    + second_value
    + third_value
    + fourth_value;
```

**§3.4 High-Priority Operator Line Breaks.** High-priority operators go at the start of the new
line.

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

## §3.5 Variable References

**§3.5.1 Variable Names.** Variable references output the variable name directly without additional
spacing.

```
// ✅ Correct
let x = my_variable;
let y = camelCaseName;

// ❌ Wrong
let x = my_variable ;  // Extra space
let y = "camelCaseName";  // Should not have quotes
```

---

## §6 Code Blocks

**§6.1 Code Block Format.** Code blocks are enclosed in curly braces `{}`, with a space before the
opening brace.

```
// ✅ Correct
fn foo() {
    let x = 1;
}

// ❌ Wrong
fn foo(){
    let x = 1;
}
fn foo()
{
    let x = 1;
}
```

**§6.2 Single-Line Code Blocks.** When a code block contains only one statement and the total length
does not exceed the line width, a single-line format may be used.

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

**§6.3 Empty Code Blocks.** Empty code blocks use `{}`.

```
// ✅ Correct
fn foo() {}

// ❌ Wrong
fn foo() {
}
```

---
title: "Function-Related Formatting Rules"
description: "Formatting rules for function definitions, function calls, and Lambda expressions"
---

# Function-Related Formatting Rules

---

## §4 Function Definition

**§4.1 Function Signature.** No space between the function name and the parameter list.

```
// ✅ Correct
fn foo(a: Int, b: Int) -> Int { ... }

// ❌ Incorrect
fn foo (a: Int, b: Int) -> Int { ... }
```

**§4.2 Parameter List Line Breaking.** When the parameter list exceeds the line width, each parameter takes one line with a trailing comma.

```
// When exceeding line width
fn very_long_function_name(first_param: Int, second_param: Int, third_param: Int) -> Int { ... }

// After formatting
fn very_long_function_name(
    first_param: Int,
    second_param: Int,
    third_param: Int,
) -> Int { ... }
```

**§4.3 Return Type.** Connect the return type to the parameter list with ` -> `, with one space before and after `->`.

```
// ✅ Correct
fn foo() -> Int { ... }

// ❌ Incorrect
fn foo()->Int { ... }
fn foo() ->Int { ... }
fn foo()-> Int { ... }
```

**§4.4 Function Body.** The function body is separated from the return type by a single space.

```
// ✅ Correct
fn foo() -> Int { 1 }

// ❌ Incorrect (two spaces)
fn foo() -> Int  { 1 }
```

---

## §7 Function Call

**§7.1 Parameter List.** Parameters are separated by commas, with one space after each comma.

```
// ✅ Correct
foo(1, 2, 3)

// ❌ Incorrect
foo(1,2,3)
foo(1 , 2 , 3)
```

**§7.2 Named Arguments.** Named arguments use the `name = value` format.

```
// ✅ Correct
foo(x = 1, y = 2)

// ❌ Incorrect
foo(x=1, y=2)
```

**§7.3 Parameter Line Breaking.** When the parameter list exceeds the line width, each parameter takes one line with a trailing comma.

```
// When exceeding line width
very_long_function_name(first_argument, second_argument, third_argument)

// After formatting
very_long_function_name(
    first_argument,
    second_argument,
    third_argument,
)
```

---

## §12 Lambda Expression

**§12.1 Lambda Format.** Lambda uses the `(params) => body` format.

```
// ✅ Correct
let f = (x) => x + 1;

// Single expression body
let f = (x) => x * 2;

// Multi-statement body
let f = (x) => {
    let y = x + 1;
    y * 2
};
```
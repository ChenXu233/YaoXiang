---
title: "Function-Related Formatting Rules"
description: Formatting rules for function definitions, function calls, and Lambda expressions
---

# Function-Related Formatting Rules

---

## §4 Function Definition

**§4.1 Function Signature.** No space between the function name and the parameter list.

```
// ✅ Correct
foo: (a: Int, b: Int) -> Int = a + b

// ❌ Incorrect
foo : (a: Int, b: Int) -> Int = a + b
```

**§4.2 Parameter List Line Break.** When the parameter list exceeds the line width, each parameter occupies one line, with a trailing comma.

```
// When exceeding the line width
very_long_function_name: (first_param: Int, second_param: Int, third_param: Int) -> Int = first_param + second_param + third_param

// After formatting
very_long_function_name:
    first_param: Int,
    second_param: Int,
    third_param: Int,
) -> Int = first_param + second_param + third_param
```

**§4.3 Return Type.** The return type is connected to the parameter list using ` -> `, with a space on each side of `->`.

```
// ✅ Correct
foo: () -> Int = 1

// ❌ Incorrect
foo: () ->Int = 1
foo: ()-> Int = 1
foo:()-> Int = 1
```

**§4.4 Function Body.** The function body is separated from the return type by a single space.

```
// ✅ Correct
foo: () -> Int = 1

// ❌ Incorrect (two spaces)
foo: () -> Int  = 1
```

---

## §7 Function Call

**§7.1 Parameter List.** Parameters are separated by commas, with a space after each comma.

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

**§7.3 Argument Line Break.** When the argument list exceeds the line width, each argument occupies one line, with a trailing comma.

```
// When exceeding the line width
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
f = (x) => x + 1

// Single-expression body
f = (x) => x * 2

// Multi-statement body
f = (x) => {
    y = x + 1
    y * 2
}
```
---
title: REPL Interactive Interpreter
description: YaoXiang REPL User Guide - Interactive Code Execution Environment
---

# REPL Interactive Interpreter

The YaoXiang REPL (Read-Eval-Print Loop) is an interactive code execution environment that allows
you to input and execute YaoXiang code line by line, making it ideal for learning, testing, and
debugging.

## Quick Start

### Starting the REPL

Run the following command in the terminal to start the REPL:

```bash
yaoxiang repl
```

Or run `yaoxiang` directly (without any subcommand):

```bash
yaoxiang
```

After starting, you will see the prompt:

```
YaoXiang REPL - Type :help for assistance
Press Ctrl+D or :quit to exit

>>
```

### Basic Usage

Enter YaoXiang code after the prompt `>>` and press Enter to execute:

```yaoxiang
>> 1 + 2
3

>> "Hello, World!"
"Hello, World!"

>> x = 10
>> x * 2
20
```

### Exiting the REPL

There are three ways to exit the REPL:

1. **Shortcut**: Press `Ctrl+D`
2. **Command**: Enter `:quit` or `:q`
3. **Interrupt**: Press `Ctrl+C` to interrupt current input

## Command System

The REPL provides a set of special commands that start with a colon `:`.

### Help Command

```yaoxiang
>> :help
```

Displays help information for all available commands.

### Quit Command

```yaoxiang
>> :quit
```

Exits the REPL. You can also use the shorthand `:q`.

### Clear Command

```yaoxiang
>> :clear
```

Clears all defined variables and functions, resetting the REPL state. You can also use the shorthand
`:c`.

### Type Lookup Command

```yaoxiang
>> :type x
```

Displays type information for symbol `x`. You can also use the shorthand `:t`.

**Example**:

```yaoxiang
>> name = "YaoXiang"
>> :type name
name: String

>> add: (a: Int, b: Int) -> Int = a + b
>> :type add
add: fn(Int, Int) -> Int
```

### Symbol List Command

```yaoxiang
>> :symbols
```

Lists all defined symbols (variables and functions) in the current REPL. You can also use the
shorthand `:i` or `:info`.

**Example**:

```yaoxiang
>> x = 10
>> y = 20
>> greet: (name: String) -> String = "Hello, " + name
>> :symbols
x: Int
y: Int
greet: fn(String) -> String
```

### History Command

```yaoxiang
>> :history
```

Displays command history. You can also use the shorthand `:hist`.

### Stats Command

```yaoxiang
>> :stats
```

Displays execution statistics, including evaluation count and total execution time.

**Example**:

```yaoxiang
>> :stats
Eval count: 5
Total time: 12.34ms
```

## Code Execution

### Expression Evaluation

The REPL can execute any valid YaoXiang expression:

```yaoxiang
>> 1 + 2
3

>> 10 * 5 + 3
53

>> "Hello" + " " + "World"
"Hello World"

>> true && false
false
```

### Variable Definition

Define variables directly using the variable name:

```yaoxiang
>> name = "YaoXiang"
>> age = 25
>> pi = 3.14159
```

You can also annotate types explicitly:

```yaoxiang
>> name: String = "YaoXiang"
>> age: Int = 25
```

After definition, variables can be used in subsequent code:

```yaoxiang
>> name
"YaoXiang"

>> age + 5
30
```

### Function Definition

YaoXiang has no `fn` keyword; functions are simply values with signatures:

```yaoxiang
>> add: (a: Int, b: Int) -> Int = a + b
>> greet: (name: String) -> String = "Hello, " + name
```

Calling functions:

```yaoxiang
>> add(3, 4)
7

>> greet("World")
"Hello World"
```

### Multi-line Code

The REPL supports multi-line code input. When incomplete code is detected (such as unclosed
brackets), it automatically enters continuation mode:

```yaoxiang
>> factorial: (n: Int) -> Int = {
..     if n <= 1 { return 1 }
..     return n * factorial(n - 1)
.. }
```

The continuation prompt is `..`, indicating that you are currently in multi-line input mode.

### Type Definition

```yaoxiang
>> Point: Type = { x: Float, y: Float }
```

### Variant Type Definition (enum)

```yaoxiang
>> Color: Type = { red: () -> Color, green: () -> Color, blue: () -> Color }
```

## Auto-completion

The REPL provides intelligent auto-completion to help you input code quickly.

### Triggering

Press the `Tab` key to trigger auto-completion.

### What Gets Completed

1. **Keyword completion**: YaoXiang language keywords (press Tab to expand)
2. **Symbol completion**: Defined variable and function names
3. **Builtin function completion**: `print`, `len`, `range`, `typeof`, `assert`, and other builtin
   functions

### Completion Examples

```yaoxiang
>> my_variable = 42
>> my_<Tab>
my_variable: Int

>> calculate_sum: (a: Int, b: Int) -> Int = a + b
>> calc<Tab>
calculate_sum: fn(Int, Int) -> Int
```

## Advanced Features

### Error Handling

When code contains errors, the REPL displays detailed error information:

```yaoxiang
>> x = 10 / 0
Error: Runtime error: DivisionByZero

>> undefined_variable
Error: Unknown symbol: undefined_variable
```

Errors do not terminate the REPL session; you can continue entering new code.

### History

The REPL automatically saves command history, supporting:

- **Up/Down arrows**: Browse through command history
- **Search**: After entering partial content, use up/down arrows to search
- **History file**: History is saved to a file and automatically loaded on next startup

### Execution Statistics

Use the `:stats` command to view execution statistics:

```yaoxiang
>> :stats
Eval count: 15
Total time: 45.67ms
```

This helps in monitoring code performance.

## Best Practices

### 1. Use Meaningful Variable Names

```yaoxiang
// Good
user_name = "YaoXiang"
max_retries = 3

// Bad
x = "YaoXiang"
n = 3
```

### 2. Define Functions to Reuse Code

```yaoxiang
>> is_even: (n: Int) -> Bool = n % 2 == 0
>> is_even(4)
true
>> is_even(7)
false
```

### 3. Use `:clear` to Reset State

When the REPL state becomes confusing, use `:clear` to reset:

```yaoxiang
>> :clear
Context cleared
```

### 4. Leverage Auto-completion for Efficiency

After typing the first few characters, press Tab to quickly complete variable and function names.

### 5. Use Multi-line Input for Complex Code

```yaoxiang
>> fibonacci: (n: Int) -> Int = {
..     if n <= 1 { return n }
..     return fibonacci(n - 1) + fibonacci(n - 2)
.. }
```

## Frequently Asked Questions

### Q: How do I view a function's definition?

A: Use the `:type` command to view the function signature:

```yaoxiang
>> :type my_function
my_function: fn(Int, String) -> Bool
```

### Q: How do I clear all definitions?

A: Use the `:clear` command:

```yaoxiang
>> :clear
```

### Q: Why isn't my multi-line code executing?

A: Check for unclosed brackets, quotes, or braces. The REPL waits for complete code input.

### Q: How do I interrupt long-running code?

A: Press `Ctrl+C` to interrupt the current execution.

### Q: What data types does the REPL support?

A: The REPL supports all YaoXiang data types:

- `Int`: Integer
- `Float`: Floating-point number
- `String`: String
- `Bool`: Boolean
- `Void`: void type
- Custom record types and variant types

## Example Session

Here is a complete REPL session example:

```yaoxiang
YaoXiang REPL - Type :help for assistance
Press Ctrl+D or :quit to exit

>> greeting = "Hello"
>> name = "YaoXiang"
>> greeting + ", " + name + "!"
"Hello, YaoXiang!"

>> factorial: (n: Int) -> Int = {
..     if n <= 1 { return 1 }
..     return n * factorial(n - 1)
.. }
..
>> factorial(5)
120

>> :symbols
greeting: String
name: String
factorial: fn(Int) -> Int

>> :stats
Eval count: 4
Total time: 2.34ms

>> :quit
```

## Related Commands

| Command    | Shortcut | Function             |
| ---------- | -------- | -------------------- |
| `:help`    | `:h`     | Display help info    |
| `:quit`    | `:q`     | Exit REPL            |
| `:clear`   | `:c`     | Clear all state      |
| `:type`    | `:t`     | View symbol type     |
| `:symbols` | `:i`     | List all symbols     |
| `:history` | `:hist`  | Show command history |
| `:stats`   | -        | Show execution stats |

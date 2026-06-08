---
title: REPL Interactive Interpreter
description: YaoXiang REPL User Guide - Interactive Code Execution Environment
---

# REPL Interactive Interpreter

YaoXiang REPL (Read-Eval-Print Loop) is an interactive code execution environment that allows you to input and execute YaoXiang code line by line, making it ideal for learning, testing, and debugging.

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

After startup, you will see the prompt:

```
YaoXiang REPL - Type :help for assistance
Press Ctrl+D or :quit to exit

>>
```

### Basic Usage

Enter YaoXiang code after the prompt `>>` and press Enter to execute:

```rust
>> 1 + 2
3

>> "Hello, World!"
"Hello, World!"

>> let x = 10
>> x * 2
20
```

### Exiting the REPL

There are three ways to exit the REPL:

1. **Shortcut**: Press `Ctrl+D`
2. **Command**: Enter `:quit` or `:q`
3. **Interrupt**: Press `Ctrl+C` to interrupt current input

## Command System

The REPL provides a series of special commands prefixed with a colon `:`.

### Help Command

```rust
>> :help
```

Displays help information for all available commands.

### Quit Command

```rust
>> :quit
```

Exits the REPL. Can also use the shorthand `:q`.

### Clear Command

```rust
>> :clear
```

Clears all defined variables and functions, resetting the REPL state. Can also use the shorthand `:c`.

### Type Command

```rust
>> :type x
```

View type information for symbol `x`. Can also use the shorthand `:t`.

**Example**:

```rust
>> let name = "YaoXiang"
>> :type name
name: String

>> fn add(a: Int, b: Int) -> Int = a + b
>> :type add
add: fn(Int, Int) -> Int
```

### Symbols Command

```rust
>> :symbols
```

Lists all defined symbols (variables and functions) in the current REPL. Can also use the shorthand `:i` or `:info`.

**Example**:

```rust
>> let x = 10
>> let y = 20
>> fn greet(name: String) -> String = "Hello, " + name
>> :symbols
x: Int
y: Int
greet: fn(String) -> String
```

### History Command

```rust
>> :history
```

Displays command history. Can also use the shorthand `:hist`.

### Stats Command

```rust
>> :stats
```

Displays execution statistics, including evaluation count and total execution time.

**Example**:

```rust
>> :stats
Eval count: 5
Total time: 12.34ms
```

## Code Execution

### Expression Evaluation

The REPL can execute any valid YaoXiang expression:

```rust
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

Use the `let` keyword to define variables:

```rust
>> let name = "YaoXiang"
>> let age = 25
>> let pi = 3.14159
```

After definition, variables can be used in subsequent code:

```rust
>> name
"YaoXiang"

>> age + 5
30
```

### Function Definition

Use the `fn` keyword to define functions:

```rust
>> fn add(a: Int, b: Int) -> Int = a + b
>> fn greet(name: String) -> String = "Hello, " + name
```

Calling functions:

```rust
>> add(3, 4)
7

>> greet("World")
"Hello World"
```

### Multi-line Code

The REPL supports multi-line code input. When incomplete code is detected (such as unclosed brackets), it automatically enters continuation mode:

```rust
>> fn factorial(n: Int) -> Int =
..   if n <= 1 then 1
..   else n * factorial(n - 1)
```

The continuation prompt is `..`, indicating the current multi-line input mode.

### Struct Definition

```rust
>> struct Point {
..   x: Float,
..   y: Float
.. }
```

### Enum Definition

```rust
>> enum Color {
..   Red,
..   Green,
..   Blue
.. }
```

## Auto-completion

The REPL provides intelligent auto-completion to help you quickly input code.

### Trigger Method

Press the `Tab` key to trigger auto-completion.

### Completion Content

1. **Keyword completion**: YaoXiang language keywords
   - `let`, `fn`, `if`, `else`, `match`, `for`, `while`, `return`, etc.

2. **Variable completion**: Defined variables
   - Type the first few characters of a variable name and press Tab to complete.

3. **Function completion**: Defined functions
   - Type the first few characters of a function name and press Tab to complete.

4. **Built-in function completion**: Built-in functions
   - `print`, `len`, `range`, `typeof`, `assert`, etc.

### Completion Example

```rust
>> let my_variable = 42
>> my_<Tab>
my_variable: Int

>> fn calculate_sum(a: Int, b: Int) -> Int = a + b
>> calc<Tab>
calculate_sum: fn(Int, Int) -> Int
```

## Advanced Features

### Error Handling

When code contains errors, the REPL displays detailed error information:

```rust
>> let x = 10 / 0
Error: Runtime error: DivisionByZero

>> undefined_variable
Error: Unknown symbol: undefined_variable
```

Errors do not terminate the REPL session; you can continue entering new code.

### History

The REPL automatically saves command history, supporting:

- **Up/Down arrows**: Browse history commands
- **Search**: Use up/down arrows to search after entering partial content
- **History file**: History is saved to a file and automatically loaded on next startup

### Execution Stats

Use the `:stats` command to view execution statistics:

```rust
>> :stats
Eval count: 15
Total time: 45.67ms
```

This helps monitor code performance.

## Best Practices

### 1. Use Meaningful Variable Names

```rust
// Good
let user_name = "YaoXiang"
let max_retries = 3

// Bad
let x = "YaoXiang"
let n = 3
```

### 2. Define Functions to Reuse Code

```rust
>> fn is_even(n: Int) -> Bool = n % 2 == 0
>> is_even(4)
true
>> is_even(7)
false
```

### 3. Use `:clear` to Reset State

When the REPL state becomes messy, use `:clear` to reset:

```rust
>> :clear
Context cleared
```

### 4. Leverage Auto-completion for Efficiency

Type the first few characters and press Tab to quickly complete variable and function names.

### 5. Use Multi-line Input for Complex Code

```rust
>> fn fibonacci(n: Int) -> Int =
..   if n <= 1 then n
..   else fibonacci(n - 1) + fibonacci(n - 2)
```

## FAQ

### Q: How do I view a function's definition?

A: Use the `:type` command to view the function signature:

```rust
>> :type my_function
my_function: fn(Int, String) -> Bool
```

### Q: How do I clear all definitions?

A: Use the `:clear` command:

```rust
>> :clear
```

### Q: Why didn't my multi-line code execute?

A: Check for unclosed brackets, quotes, or braces. The REPL waits for complete code input.

### Q: How do I interrupt long-running code?

A: Press `Ctrl+C` to interrupt the current execution.

### Q: What data types does the REPL support?

A: The REPL supports all YaoXiang data types:
- `Int`: Integer
- `Float`: Floating-point number
- `String`: String
- `Bool`: Boolean
- `Unit`: Unit type
- Custom structs and enums

## Example Session

Here is a complete REPL session example:

```rust
YaoXiang REPL - Type :help for assistance
Press Ctrl+D or :quit to exit

>> let greeting = "Hello"
>> let name = "YaoXiang"
>> greeting + ", " + name + "!"
"Hello, YaoXiang!"

>> fn factorial(n: Int) -> Int =
..   if n <= 1 then 1
..   else n * factorial(n - 1)
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

| Command | Shortcut | Function |
|---------|----------|----------|
| `:help` | `:h` | Show help information |
| `:quit` | `:q` | Exit REPL |
| `:clear` | `:c` | Clear all state |
| `:type` | `:t` | View symbol type |
| `:symbols` | `:i` | List all symbols |
| `:history` | `:hist` | Show command history |
| `:stats` | - | Show execution statistics |
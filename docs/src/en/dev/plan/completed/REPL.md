# REPL Implementation Documentation

## Overview

The REPL (Read-Eval-Print Loop) is YaoXiang's interactive interpreter, providing developers with a programming environment that offers immediate feedback. Users can directly input code snippets and immediately view execution results, greatly improving prototyping efficiency and language learning.

The REPL module is located at `src/backends/dev/repl.rs` and is closely integrated with the development Shell (`src/backends/dev/shell.rs`), together forming YaoXiang's interactive development environment. The REPL focuses on code evaluation, while the Shell provides a richer set of development commands including file management, debugging, and other features.

The current REPL implementation supports multi-line input, expression completeness detection, history management, special command handling, and other core features. Code evaluation is achieved through integration with the frontend compiler, supporting real-time syntax checking and error reporting.

## Architecture Design

### Module Location and Dependencies

```
src/backends/dev/
├── repl.rs         # REPL main module
├── shell.rs        # Development Shell (wraps REPL)
└── debugger.rs     # Debugger integration

src/backends/common/
├── value.rs        # RuntimeValue runtime value type
├── heap.rs         # Heap memory management
└── mod.rs          # Common module exports

src/backends/interpreter/
├── executor.rs     # Instruction executor
├── frames.rs       # Call frame management
└── registers.rs    # Register management
```

The REPL module depends on the following core types: `RuntimeValue` provides unified runtime value representation, `Interpreter` is responsible for code execution, and `Compiler` handles source code compilation. Modules communicate through clearly defined interfaces, ensuring each component can be independently tested and evolved.

### Core Data Structures

#### REPLConfig Configuration Structure

```rust
#[derive(Debug, Clone)]
pub struct REPLConfig {
    /// Standard prompt, displayed at the beginning of each input line
    pub prompt: String,
    /// Multi-line input prompt, used for block structure continuation
    pub multi_line_prompt: String,
    /// Syntax highlight toggle (interface reserved for future terminal highlighting library integration)
    pub syntax_highlight: bool,
    /// Auto-indent toggle (interface reserved, requires input library integration)
    pub auto_indent: bool,
    /// Maximum number of history entries to prevent unbounded memory growth
    pub history_size: usize,
}
```

The configuration structure adopts an extensible design. The `syntax_highlight` and `auto_indent` fields are reserved interfaces for future enhancements. The current standard prompt is `">> "`, the multi-line prompt is `".. "`, and history defaults to storing 1000 entries.

#### REPLResult Evaluation Result Enum

```rust
#[derive(Debug)]
pub enum REPLResult {
    /// Evaluation produced an actual value that needs to be printed
    Value(RuntimeValue),
    /// Evaluation has no return value (unit type)
    Ok,
    /// An error occurred during evaluation
    Error(String),
    /// User voluntarily exited (:quit or Ctrl-D)
    Exit,
}
```

The result enum clearly distinguishes four evaluation outcomes, facilitating separate handling in the main loop. `Value` wraps the actual runtime value, `Error` carries human-readable error information, and `Exit` signals the main loop to terminate.

#### REPL Main Structure

```rust
#[derive(Debug)]
pub struct REPL {
    /// REPL configuration
    config: REPLConfig,
    /// Code interpreter instance
    interpreter: Interpreter,
    /// Input history (supports up/down arrow traversal)
    history: Vec<String>,
    /// Current input buffer (used for multi-line continuation)
    buffer: String,
    /// Current line count (0 indicates a new expression start)
    line_count: usize,
}
```

The REPL structure separates configuration, internal state, and execution engine. The `buffer` stores the multi-line expression the user is currently typing, and `line_count` tracks continuation state. Together, they enable multi-line input support for block structures (such as function definitions, if expressions).

## Workflow

### Main Loop Flow

```
┌─────────────────────────────────────────────────────┐
│                    REPL.run()                        │
├─────────────────────────────────────────────────────┤
│  1. Print welcome message                            │
│  2. Enter main loop                                   │
│     ┌─────────────────────────────────────────────┐ │
│     │  read_line()                                │ │
│     │  ├─ Display prompt                          │ │
│     │  ├─ Read one line of input                  │ │
│     │  ├─ Detect special commands (:quit, :help)  │ │
│     │  └─ Add to history                          │ │
│     ├─ Determine expression completeness           │ │
│     │  └─ Incomplete → Continue read_line()        │ │
│     └─ Complete → evaluate()                       │ │
│         ├─ Wrap code as complete function          │ │
│         ├─ Call Compiler to compile                │ │
│         └─ Return result                           │ │
│  3. Process result and loop or exit                  │
└─────────────────────────────────────────────────────┘
```

The main loop follows the classic REPL pattern: read input, evaluate code, print result. The key design decision is to separate expression completeness judgment from input reading, allowing users to gradually construct complex expressions during multi-line input.

### Expression Completeness Detection

The `is_complete()` method determines whether an expression is complete by tracking the pairing state of parentheses, braces, and brackets. The algorithm considers string escaping to avoid misinterpreting brackets inside strings as expression delimiters.

```rust
fn is_complete(&self, code: &str) -> bool {
    let mut braces = 0;   // { }
    let mut brackets = 0; // [ ]
    let mut parens = 0;   // ( )
    let mut in_string = false;
    let mut escaped = false;

    for c in code.chars() {
        // Handle escape characters
        if escaped { escaped = false; continue; }
        if c == '\\' { escaped = true; continue; }

        // Handle strings
        if c == '"' { in_string = !in_string; continue; }

        // Count in non-string regions
        if !in_string {
            match c {
                '{' => braces += 1,
                '}' => { if braces == 0 { return true; } braces -= 1; }
                '[' => brackets += 1,
                ']' => { if brackets == 0 { return true; } brackets -= 1; }
                '(' => parens += 1,
                ')' => { if parens == 0 { return true; } parens -= 1; }
                _ => {}
            }
        }
    }

    braces == 0 && brackets == 0 && parens == 0 && !in_string && !escaped
}
```

The boundary case handling in completeness detection is worth noting: when encountering an unmatched closing bracket (e.g., `}` but braces is already 0), the method immediately returns `true`, indicating the preceding expression is complete. This allows users to input incomplete continuation lines and continue typing.

### Code Evaluation Flow

```rust
fn evaluate(&mut self, code: &str) -> Result<REPLResult, io::Error> {
    // 1. Wrap code as a complete function
    let wrapped = format!(
        "main() -> () = () => {{\n{}\n}}",
        code
    );

    // 2. Call frontend compiler
    let mut compiler = crate::frontend::Compiler::new();
    match compiler.compile_with_source("<repl>", &wrapped) {
        Ok(_module) => Ok(REPLResult::Ok),
        Err(e) => {
            // 3. Error handling (streamlined output)
            let lines: Vec<&str> = error_msg.lines().collect();
            if lines.len() > 2 {
                Ok(REPLResult::Error(
                    lines[lines.len() - 2..].join("\n")
                ))
            } else {
                Ok(REPLResult::Error(error_msg))
            }
        }
    }
}
```

The code wrapping strategy embeds user input into a `main() -> () = () => { ... }` function. This design ensures that both expressions and statements can be correctly processed by the compiler. The wrapped code goes through the standard compilation flow and can ultimately be executed by the interpreter.

Error output undergoes streamlined processing, removing file context lines and keeping only core error information for a friendlier interactive experience.

## Configuration and Customization

### Default Configuration

```rust
impl Default for REPLConfig {
    fn default() -> Self {
        Self {
            prompt: ">> ".to_string(),
            multi_line_prompt: ".. ".to_string(),
            syntax_highlight: true,
            auto_indent: true,
            history_size: 1000,
        }
    }
}
```

### Custom Configuration Example

```rust
let config = REPLConfig {
    prompt: "yx> ".to_string(),
    multi_line_prompt: "... ".to_string(),
    syntax_highlight: false,  // Disable syntax highlighting
    auto_indent: false,       // Disable auto-indent
    history_size: 500,        // Reduce history size
};

let mut repl = REPL::with_config(config);
```

### Creation Methods

```rust
// Use default configuration
let repl = REPL::new();

// Use custom configuration
let repl = REPL::with_config(custom_config);
```

## Special Commands Reference

### Available Command List

| Command | Alias | Description |
|---------|-------|-------------|
| `:quit` | `:q` | Exit REPL and return to parent Shell |
| `:help` | `:h` | Display available command list |
| `:clear` | `:c` | Clear current buffer (abandon unfinished input) |
| `:history` | `:hist` | Display history (with line numbers) |

### Command Processing Flow

```rust
fn handle_command(&mut self, command: &str) -> Result<REPLResult, io::Error> {
    match command {
        ":quit" | ":q" => Ok(REPLResult::Exit),
        ":help" | ":h" => {
            // Display help information
            Ok(REPLResult::Ok)
        }
        ":clear" | ":c" => {
            // Clear buffer
            self.buffer.clear();
            self.line_count = 0;
            Ok(REPLResult::Ok)
        }
        ":history" | ":hist" => {
            // Traverse and display history
            for (i, line) in self.history.iter().enumerate() {
                tlog!(info, /* ... */);
            }
            Ok(REPLResult::Ok)
        }
        _ => {
            // Unknown command prompt
            Ok(REPLResult::Ok)
        }
    }
}
```

## Integration with DevShell

### Shell Invokes REPL

```rust
":repl" | "repl" => {
    if let Err(e) = self.repl.run() {
        ShellResult::Error(format!("REPL error: {}", e))
    } else {
        ShellResult::Success
    }
}
```

### State Transitions

```
Shell ──:repl──> REPL.run()
                    │
                    ├─ :quit ──> Return to Shell
                    └─ Ctrl-D ──> Return to Shell
```

### Shell Auxiliary Features

Shell additionally provides the following features that REPL cannot directly access:

| Shell Command | Description |
|---------------|-------------|
| `:cd <path>` | Change working directory |
| `:pwd` | Display current directory |
| `:ls [path]` | List directory contents |
| `:run <file>` | Run file and time execution |
| `:load <file>` | Load file into environment |
| `:debug <file>` | Start debug mode |
| `:break <fn> <offset>` | Set breakpoint |

## Technical Implementation Details

### Input Line Reading

```rust
fn read_line(&mut self) -> Result<REPLResult, io::Error> {
    // 1. Determine prompt (single-line or multi-line)
    let prompt = if self.line_count == 0 {
        &self.config.prompt
    } else {
        &self.config.multi_line_prompt
    };

    // 2. Print prompt and flush output buffer
    tlog!(debug, MSG::ReplPrompt, &prompt.to_string());
    io::stdout().flush()?;

    // 3. Read from standard input
    let mut line = String::new();
    let stdin = io::stdin();

    if stdin.read_line(&mut line)? == 0 {
        return Ok(REPLResult::Exit);  // Ctrl-D detection
    }

    // 4. Process command or add to buffer
    // ...
}
```

### History Management

```rust
// Only non-empty lines are added to history
if !line.is_empty() {
    self.history.push(line.clone());
}

// History is used to provide up/down arrow traversal
// (Note: Current implementation stores in Vec, terminal interaction requires readline library)
```

History only stores non-empty lines to avoid blank lines consuming space. In the future, mature libraries like `rustyline` or `liner` can be integrated to provide true interactive history browsing.

### File Loading

```rust
pub fn load_file(&mut self, path: &Path) -> Result<REPLResult, io::Error> {
    let source = std::fs::read_to_string(path)?;

    let mut compiler = crate::frontend::Compiler::new();
    match compiler.compile_with_source(&path.display().to_string(), &source) {
        Ok(_module) => Ok(REPLResult::Ok),
        Err(e) => Ok(REPLResult::Error(format!("{}", e))),
    }
}
```

`load_file()` reuses the compiler interface to support loading and executing `.yx` source files. File paths use the `Path` type to ensure cross-platform compatibility.

### i18n Message Mapping

The REPL uses a unified message system (`src/util/i18n/mod.rs`), with all user-visible text accessed through the `MSG` enum:

```rust
MSG::ReplWelcome     // Welcome message
MSG::ReplHelp        // Help information
MSG::ReplError       // Error prefix
MSG::ReplValue       // Value output format
MSG::ReplPrompt      // Prompt format
// ...
```

This design supports multi-language extension by simply providing different `MSG` implementations for different languages.

## Test Coverage

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_repl_new() {
        let repl = REPL::new();
        assert!(repl.history.is_empty());
    }

    #[test]
    fn test_repl_is_complete() {
        let repl = REPL::new();

        // Complete expressions
        assert!(repl.is_complete("1 + 2"));
        assert!(repl.is_complete("let x = 42"));
        assert!(repl.is_complete("fn foo() { 1 }"));

        // Incomplete expressions
        assert!(!repl.is_complete("fn foo() {"));
        assert!(!repl.is_complete("if true {"));
        assert!(!repl.is_complete("{"));
    }
}
```

## Future Evolution Directions

### Short-term Enhancements

1. **Integrate readline library**: Provide true interactive editing (Emacs/Vi mode, incremental search)
2. **Syntax highlighting**: Integrate ANSI escape sequences for highlighting keywords, numbers, strings
3. **Tab auto-completion**: Symbol completion, function parameter hints
4. **Multi-line editing**: Support smart continuation triggered by `(` or `{`

### Medium-term Goals

1. **Incremental type checking**: Re-check only changed parts to avoid full compilation latency
2. **Inline result printing**: Provide summaries for complex values, `:expand` for detailed view
3. **Persistent history**: Retain history across sessions
4. **Module imports**: Direct import of compiled modules within REPL

### Long-term Vision

1. **Jupyter Kernel**: Provide IPython/Jupyter integration
2. **Graphical REPL**: Visualize data structures, call stack, timeline
3. **Remote REPL**: Debug remote processes via network REPL
4. **Performance profiling**: `:profile` command to output execution time, memory allocation statistics

## Related File Index

| File | Responsibility |
|------|----------------|
| `src/backends/dev/repl.rs` | REPL main module |
| `src/backends/dev/shell.rs` | Development Shell |
| `src/backends/dev/debugger.rs` | Debugger |
| `src/backends/common/value.rs` | Runtime value type |
| `src/backends/common/heap.rs` | Heap memory management |
| `src/backends/interpreter/mod.rs` | Interpreter module |
| `src/util/i18n/mod.rs` | Internationalization messages |
| `src/frontend/Compiler.rs` | Compiler frontend |
---
title: "REPL State"
---

# REPL

> **Module Status**: Incomplete (3 items to improve)
> **Location**: `src/backends/dev/repl/`
> **Last Updated**: 2026-06-01

---

## Module Overview

The REPL (Read-Eval-Print Loop) module provides an interactive programming environment. It adopts a trait abstraction architecture, supporting different backend implementations.

**Code Size**: 1,037 lines (8 files)

---

## Feature List

### REPLBackend trait (backend_trait.rs)

- ✅ `eval()` evaluate
- ✅ `complete()` completion candidates
- ✅ `get_symbols()` symbol list
- ✅ `get_type()` type query
- ✅ `clear()` clear state
- ✅ `stats()` execution statistics

### Evaluation Engine (engine/evaluator.rs - 299 lines)

- ✅ Code compilation and execution
- ✅ Bracket/quote integrity detection
- ✅ Automatic expression/statement wrapping
- ✅ Extract definitions from bytecode

### Execution Context (engine/context.rs - 168 lines)

- ✅ Variable definition/query
- ✅ Function definition/query
- ✅ Symbol type query
- ✅ Execution statistics

### Command System (commands/mod.rs - 95 lines)

- ✅ `:quit/:q` exit
- ✅ `:help/:h` help
- ✅ `:clear/:c` clear
- ✅ `:type/:t` type view
- ✅ `:symbols/:info` symbol list
- ✅ `:stats` statistics
- ⚠️ `:history` command — **Not implemented** (only prints a hint)

### Session REPL (session/mod.rs - 247 lines)

- ✅ rustyline integration
- ✅ Multi-line input support
- ✅ History save/load
- ✅ VI/Emacs editing mode
- ✅ File load and execute
- ✅ Custom configuration

### Auto-completion (session/completer.rs - 126 lines)

- ✅ keyword completion
- ✅ variable/function completion
- ✅ builtin function completion

---

## Test Coverage

**0 unit tests**

The REPL module has no test code. The entire `src/backends/dev/repl/` directory has no `#[test]` or `#[cfg(test)]` annotations.

---

## Code Quality Assessment

| Dimension | Score | Notes |
|-----------|-------|-------|
| Incomplete Items | 3 | Add tests, :history command, add tests |
| Test Coverage | 0% | No tests whatsoever |
| Documentation Quality | Good | Complete user guide (`docs/src/guide/repl.md`, 436 lines) and code comments |
| Architecture Design | Excellent | Clear trait abstraction, layered design, extensible |

---

## Integration Status

REPL is integrated into the following components:

1. **DevShell** (`src/backends/dev/shell.rs`): Switches to REPL mode via the `:repl` command
2. **Module Exports** (`src/backends/dev/mod.rs`): Exports `SessionREPL`, `Evaluator`, `REPLBackend`
3. **CLI Entry**: Starts via `yaoxiang repl` or `yaoxiang`

---

## Items to Improve

1. **Add unit tests** (zero test coverage is the biggest issue)
2. **Implement `:history` command**
3. **Add boundary condition tests**
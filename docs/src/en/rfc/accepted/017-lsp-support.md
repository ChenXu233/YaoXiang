---
title: 'RFC-017: Language Server Protocol (LSP) Support Design'
status: 'Accepted'
author: '晨煦'
created: '2026-02-15'
updated: '2026-07-05'
impl_status: 'complete'

issue: '#11'
---

# RFC-017: Language Server Protocol (LSP) Support Design

>

>

>

> **Reference**: See the [full example](../EXAMPLE_full_feature_proposal.md) to learn how to write
> an RFC.

## ⚠️ Implementation Prerequisites (Important)

Before implementing LSP, the following two core issues must be resolved:

### Issue 1: Diagnostic Error Collection

**Current state**: The current type checker returns immediately upon encountering the first error
(using the `?` operator), and cannot collect all errors.

**LSP requirement**: The IDE needs to display **all** errors, not just the first one.

**Solution**:

#### 1.1 Error Collection Mode

- Modify the `src/frontend/typecheck/inference/` module to return `Result<Type, Vec<Error>>`
- Do not return immediately upon encountering an error; instead, continue checking
- Return all errors together after checking completes

#### 1.2 Error Severity Levels

Distinguish errors by severity:

```rust
enum ErrorKind {
    Error,      // Serious error, may cause cascading errors
    Warning,    // Warning, continue checking but do not block
    Note,       // Additional information
}
```

- If there is an `Error`: `publishDiagnostics` displays the error
- If only `Warning`: continue compiling and display warnings

#### 1.3 Parser Error Recovery

- When parsing fails, insert **placeholder nodes** (such as `MissingExpression`) instead of giving
  up
- Prevent type checker panic caused by incomplete AST
- Example: `let x = ;` → `let x = MissingExpression`

#### 1.4 Delayed Emission

- Some errors may be "cascading" (caused by earlier errors)
- They can be collected first and obvious cascading errors filtered out after AST parsing completes
- Or simply: report all of them and let the user fix them one by one

### Issue 2: File-Level Parse Cache

**Current state**: Each LSP request re-parses the entire file, with no caching mechanism.

**LSP requirement**: Each edit should respond quickly without re-parsing unchanged files.

**Solution**:

#### 2.1 File Cache Structure

```rust
struct DocumentCache {
    version: u32,           // LSP document version number
    content: String,        // Current content
    content_hash: u64,      // Content hash (for fast comparison)
    ast: Option<Ast>,       // Cached AST (optional)
}
```

#### 2.2 Detecting Changes

- Receive new content with each `textDocument/didChange`
- Compute the hash of the new content and compare with cached `content_hash`
- **If changed: re-parse the entire file**
- **If unchanged: return the cached result directly**

#### 2.3 Re-parse Strategy

- **File-level**: re-parse only the current file, not the entire project
- This is a simplified design; no function-level incremental parsing
- Modern computers can parse a single file of several thousand lines in just a few milliseconds

#### 2.4 Difference from cargo check

|           | cargo check        | YaoXiang LSP              |
| --------- | ------------------ | ------------------------- |
| Scope     | Entire project     | Single file               |
| Frequency | Manually triggered | Every edit                |
| Goal      | Full compile check | Fast incremental response |

### Integration with Existing Modules

| Existing module                  | LSP integration approach                                                    |
| -------------------------------- | --------------------------------------------------------------------------- |
| `util/span.rs`                   | ✅ Already has `Position`/`Span`, directly map to LSP `Position`            |
| `util/diagnostic/collect.rs`     | ⚠️ Needs to be changed to "collection mode", continuously accumulate errors |
| `frontend/core/lexer/symbols.rs` | ⚠️ Needs to be extended to add `uri` + `span` location information          |
| `frontend/typecheck/mod.rs`      | ⚠️ Needs to modify `TypeResult` to return all errors                        |
| `frontend/core/parser/ast.rs`    | ✅ Each node already has `Span`, no changes needed                          |

---

## Summary

Add Language Server Protocol (LSP) support to YaoXiang, implementing a full language server so that
mainstream IDEs (VS Code, Neovim, Emacs, etc.) can provide development tooling features such as code
completion, go-to-definition, diagnostics, and reference search.

## Motivation

### Why is this feature needed?

Currently, the YaoXiang language lacks official IDE integration support. Developers can only use
basic text editors to write code, lacking:

1. **Code completion** - cannot intelligently complete identifiers, keywords, types based on context
2. **Go to definition** - cannot quickly jump to the definition of a function, type, or variable
3. **Real-time diagnostics** - cannot display syntax errors and type errors in real time while
   editing
4. **Reference search** - cannot find all reference locations of a symbol
5. **Hover hints** - cannot display type information and documentation comments on mouse hover

LSP is a standard feature of modern programming languages. Mainstream languages (Rust, Python,
TypeScript, Go, etc.) all provide mature LSP implementations. Implementing LSP support will
significantly improve the YaoXiang development experience.

### Current Problems

1. **Low development efficiency** - lack of code completion and smart hints
2. **Difficult debugging** - cannot quickly locate symbol definitions
3. **Steep learning curve** - lack of IDE assistance features
4. **Incomplete ecosystem** - cannot attract developers accustomed to modern IDEs

## Proposal

### Core Design

Implement an independent LSP server process that communicates with the IDE via JSON-RPC:

```mermaid
flowchart TD
    subgraph IDE_Environment [IDE Environment]
        IDE["IDE (VS Code)"]
    end

    subgraph LSP_Server [LSP Server]
        LSP["YaoXiang LSP Server"]
    end

    subgraph World_Compile [Compilation World]
        direction TB
        W_Symbol["Symbol Index"]
        W_Type["Type Env"]
        W_Diag["Diagnostics"]
    end

    subgraph Cache [Document Cache]
        direction TB
        C_Version["Version Management"]
        C_Content["Content Cache"]
        C_AST["AST Cache"]
        C_Delta["Incremental Change Regions"]
    end

    subgraph Frontend [Compiler Frontend]
        direction TB
        F_Lexer["Lexer (util/span.rs Position)"]
        F_Parser["Parser (ast.rs already has Span)"]
        F_TypeCheck["Type Check (changed to collection mode)"]
        F_ErrorCollector["ErrorCollector (util/diagnostic/)"]
    end

    IDE <-->|JSON-RPC| LSP

    LSP --- World_Compile
    LSP --- Cache

    Cache -- "Incremental Update" --> World_Compile

    World_Compile --- Frontend
    Cache --- Frontend
```

### LSP Server Architecture

```
src/lsp/
├── main.rs              # LSP server entry point
├── server.rs           # Server core logic
├── session.rs          # Session management
├── capabilities.rs     # Server capability declaration
├── handlers/
│   ├── mod.rs
│   ├── initialize.rs   # Initialization handling
│   ├── text_document.rs # Document operation handling
│   ├── completion.rs   # Completion handling
│   ├── definition.rs   # Go-to-definition handling
│   ├── references.rs   # Reference search handling
│   ├── hover.rs        # Hover hint handling
│   └── diagnostics.rs  # Diagnostic handling
├── world.rs            # Compilation world (symbol table, AST cache)
├── scroller.rs         # Symbol index construction
├── protocol.rs         # LSP protocol type definitions
└── cache/              # Incremental cache module (new)
    ├── mod.rs
    ├── document.rs     # Document cache (version, AST, symbol table)
    └── incremental.rs  # Incremental parse strategy
```

### Compilation World Design

Manages global compilation state:

- Document cache (version, AST, symbol table)
- Global symbol index
- Error collector
- Type environment cache

Core methods:

- `on_document_change`: handles incremental changes
- `incremental_reparse`: incremental re-parse
- `collect_diagnostics`: collect all errors (non-blocking)

### Core LSP Method Support

| Category          | Method                                             | Description             |
| ----------------- | -------------------------------------------------- | ----------------------- |
| **Lifecycle**     | `initialize` / `initialized` / `shutdown` / `exit` | Server lifecycle        |
| **Document Sync** | `didOpen` / `didChange` / `didClose`               | Document management     |
| **Diagnostics**   | `publishDiagnostics`                               | Publish diagnostics     |
| **Completion**    | `completion`                                       | Code completion         |
| **Go to**         | `definition`                                       | Go to definition        |
| **References**    | `references`                                       | Find references         |
| **Hover**         | `hover`                                            | Hover hints             |
| **Symbols**       | `workspace/symbol`                                 | Workspace symbol search |

### Text Document Sync Mechanism

Uses an incremental sync strategy:

- Keep document version number
- Apply incremental changes (range + text)
- Fall back to full replacement on large changes

### Symbol Index Construction

Leverages the existing symbol table system to build a reverse index:

- Need to extend `SymbolEntry` to add a `location` field
- Index: name → list of locations, file → list of symbols

### Code Completion Implementation

Sources of completion: keywords, variables, functions, types, struct fields, modules

### Go-to-Definition Implementation

AST-based symbol resolution: find the definition location corresponding to an identifier/function
call

## Detailed Design

### Type System Impact

1. **Symbol information extension** - add location information (file, line number, column number) to
   the symbol table
2. **Type information exposure** - provide a type query interface for LSP
3. **Documentation comment integration** - support generating doc strings from comments

### Runtime Behavior

- The LSP server runs as an independent process
- Use stdin/stdout for JSON-RPC communication
- Support multi-session concurrent handling

### Compiler Changes

| Component                     | Change                                           |
| ----------------------------- | ------------------------------------------------ |
| `frontend/events`             | Extend event system to support LSP notifications |
| `frontend/core/lexer/symbols` | Enhance symbol table to add location information |
| New `src/lsp/`                | LSP server implementation                        |

### Backward Compatibility

- ✅ Fully backward compatible
- The LSP server is an independent component that does not affect the existing compilation flow
- Existing CLI tools are not affected

### Integration with Existing Systems

1. **Event system** - leverage the event subscription mechanism from `frontend/events/`
2. **Diagnostic system** - reuse diagnostic output from `util/diagnostic/`
   - Reuse `ErrorCollector<E>` to collect all errors
   - Convert `Diagnostic` to LSP `Diagnostic` format
3. **Symbol table** - extend the symbol location capabilities in `symbols.rs`
   - Extend `SymbolEntry` to add a `location: Location` field
   - Build a `SymbolIndex` reverse index (name → list of locations)
4. **Compiler frontend** - directly call Lexer, Parser, type checker
   - **Key change**: the type checker must be changed to "collection mode" and not block execution

#### Diagnostic Format Conversion

```rust
/// Convert YaoXiang Diagnostic to LSP Diagnostic
fn to_lsp_diagnostic(diag: &Diagnostic) -> lsp_types::Diagnostic {
    let severity = match diag.severity() {
        Severity::Error => lsp_types::DiagnosticSeverity::ERROR,
        Severity::Warning => lsp_types::DiagnosticSeverity::WARNING,
        Severity::Info => lsp_types::DiagnosticSeverity::INFORMATION,
    };

    lsp_types::Diagnostic {
        range: to_lsp_range(diag.span()),
        severity: Some(severity),
        message: diag.message().to_string(),
        code: diag.code().map(|c| lsp_types::NumberOrString::String(c.as_string())),
        ..Default::default()
    }
}

/// Convert YaoXiang Span to LSP Range
fn to_lsp_range(span: &Span) -> lsp_types::Range {
    lsp_types::Range {
        start: lsp_types::Position {
            line: span.start.line.saturating_sub(1), // LSP uses 0-indexed
            character: span.start.column.saturating_sub(1),
        },
        end: lsp_types::Position {
            line: span.end.line.saturating_sub(1),
            character: span.end.column.saturating_sub(1),
        },
    }
}
```

## YaoXiang-Specific Advanced Features

Leverage YaoXiang's powerful compile-time evaluation and ownership system to provide a unique
development experience that other languages cannot offer:

### 1. Inlay Hints

- **Constant value hints**: display compile-time computed values (e.g., `const MAX = 100 + 200`
  shows `300` next to it)
- **Mutability hints**: show whether a variable is mutable (e.g., `mut x`, `x` with a visible
  underline)
- **Ownership consumption hints**: show whether function parameters are consumed (e.g., `consumed` /
  `borrowed`)
- **Empty-ownership semantics hints**: hint that the variable can be reassigned after being moved by
  dimming its color
- **Type inference hints**: display the inferred concrete type (e.g., `x = vec![]` shows `Vec<i32>`
  next to it)

### 2. Ownership Semantics Visualization

- Display the move path of a variable (from definition location to all usage locations)
- Borrow lifetime visualization

### 3. Compile-Time Evaluation Preview

- Hover to display the compile-time computation result of constant expressions

### Implementation Priority

| Feature                     | Priority |
| --------------------------- | -------- |
| Constant value inlay hints  | P0       |
| Mutability hints            | P0       |
| Ownership consumption hints | P1       |
| Ownership visualization     | P2       |

---

## Communication and Remote Support

### Communication Modes

Supports three modes:

| Mode               | Use                                  |
| ------------------ | ------------------------------------ |
| stdio              | Local development (default)          |
| TCP Socket         | Remote development/debugging         |
| Unix Domain Socket | High-performance local communication |

### Remote Debugging

Implemented based on DAP (Debug Adapter Protocol):

- Support line breakpoints, function breakpoints, conditional breakpoints
- YaoXiang-specific breakpoint: triggered when a variable is moved

### Startup Parameters

```bash
# Local mode
yaoxiang-lsp

# TCP server
yaoxiang-lsp --tcp --port 8765

# Enable debugging as well
yaoxiang-lsp --tcp --port 8765 --enable-debug
```

---

## Concurrency Model

**Design decision: single-threaded + async event loop**

Reasons:

- The compiler is not thread-safe; refactoring cost is high
- LSP requests are naturally serial; concurrency is not required
- Single-threaded is simpler and easier to debug
- async I/O on a single thread is fast enough

Background tasks use `spawn_blocking` to take advantage of multiple cores.

---

## Built-in LSP Test Tool (Optional)

> This feature is not required for MVP and can be added in a later version.

Provides a JSON test case format:

```bash
# Run tests
yaoxiang-lsp --test
```

---

## Trade-offs

### Pros

1. **Improved development experience** - IDE support approaching that of mainstream languages
2. **Improved ecosystem** - attracts more developers to use YaoXiang
3. **Improved code quality** - real-time diagnostics reduce runtime errors
4. **Community contribution** - developers can participate in LSP toolchain development

### Cons

1. **High implementation complexity** - need to handle many LSP edge cases
2. **Maintenance cost** - need to follow LSP protocol version updates
3. **Performance considerations** - indexing and query performance for large projects
4. **Testing difficulty** - need to simulate IDE behavior for testing

## Alternatives

| Alternative                      | Why not chosen                                |
| -------------------------------- | --------------------------------------------- |
| Only provide syntax highlighting | Cannot meet modern development needs          |
| Use Tree-sitter                  | Extra learning cost and limited functionality |

## Implementation Strategy

### Phase Breakdown

1. **Phase 0 (prerequisite)**: Compiler adaptation ⚠️ **Critical**
   - Modify the type checker to "collection mode", returning `Result<Type, Vec<Error>>`
   - Implement error severity levels (Error / Warning / Note)
   - Parser error recovery: insert placeholder nodes
   - Extend symbol table `SymbolEntry` to add `location` field
   - Implement the DocumentCache system (version + content + hash)
   - **This phase is a prerequisite for LSP implementation and must be completed first**

2. **Phase 1 (v0.7)**: Basic framework
   - LSP server skeleton
   - Lifecycle methods (initialize/shutdown/exit)
   - Basic logging and error handling

3. **Phase 2 (v0.7)**: Diagnostic support
   - Text document sync
   - Compile diagnostic integration
   - `textDocument/publishDiagnostics`

4. **Phase 3 (v0.8)**: Completion support
   - Symbol index construction
   - Keyword completion
   - Identifier completion

5. **Phase 4 (v0.8)**: Go-to support
   - Go to definition
   - Find references
   - Hover hints

6. **Phase 5 (v0.9)**: Advanced features
   - Workspace symbol search
   - Code formatting
   - Refactoring support (optional)

### Dependencies

- No external LSP library dependencies (uses the `lsp-types` crate)
- Depends on existing compiler frontend modules
- Depends on `serde_json` for JSON-RPC serialization

### Risks

1. **Performance issues** - large file parsing may cause lag
   - Solution: incremental parsing, background thread processing
2. **Memory usage** - symbol index occupies memory
   - Solution: lazy loading, LRU cache
3. **Protocol compatibility** - LSP version differences
   - Solution: declare supported protocol versions

## Open Questions

- [x] Error collection mechanism (see "Implementation Prerequisites" section)
- [x] Incremental cache system (see "Implementation Prerequisites" section)
- [x] LSP protocol version: use 3.18 (supports new features such as Inlay Hints and Inline Values)
- [x] Remote communication support (via TCP, accommodating both LSP and debugging)
- [x] Remote debugging support (based on DAP protocol)
- [x] Concurrency model: single-threaded + async event loop
- [x] Built-in LSP test tool (optional): use JSON test cases

---

## Appendix (Optional)

### Appendix A: Design Discussion Record

> Used to record detailed discussions during the design decision process.

### Appendix B: Design Decision Record

| Decision                | Decision                                                                     | Date       | Recorder |
| ----------------------- | ---------------------------------------------------------------------------- | ---------- | -------- |
| LSP server architecture | Independent process, communicating via stdio                                 | 2026-02-15 | 晨煦     |
| Protocol version        | Support LSP 3.18 (needs new features such as Inlay Hints)                    | 2026-02-22 | 晨煦     |
| Error collection mode   | Return `Result<Type, Vec<Error>>`, support error severity and error recovery | 2026-02-22 | 晨煦     |
| Cache strategy          | File-level cache: version + content + hash, re-parse the entire file         | 2026-02-22 | 晨煦     |
| Communication modes     | Support stdio + TCP + UnixSocket                                             | 2026-02-22 | 晨煦     |
| Remote debugging        | Based on DAP protocol, sharing transport layer with LSP                      | 2026-02-22 | 晨煦     |
| Concurrency model       | Single-threaded + async event loop                                           | 2026-02-22 | 晨煦     |
| Test tool (optional)    | JSON test cases + built-in test runner                                       | 2026-02-22 | 晨煦     |

### Appendix C: Glossary

| Term              | Definition                                       |
| ----------------- | ------------------------------------------------ |
| LSP               | Language Server Protocol                         |
| JSON-RCP          | JSON-Remote Procedure Call                       |
| DAP               | Debug Adapter Protocol                           |
| Symbol index      | Symbol-to-location map built at compile time     |
| Compilation world | Context containing all compilation information   |
| Inlay hints       | In-line hint information displayed within a line |
| Ownership trace   | Visualization of variable ownership flow         |

---

## References

- [Language Server Protocol Specification](https://microsoft.github.io/language-server-protocol/)
- [LSP Specification 3.18](https://github.com/microsoft/language-server-protocol/blob/main/specifications/specification-3-18.md)
- [Debug Adapter Protocol Specification](https://microsoft.github.io/debug-adapter-protocol/)
- [Rust Analyzer](https://rust-analyzer.github.io/) - reference implementation
- [lsp-types crate](https://crates.io/crates/lsp-types) - LSP type definitions
- [JSON-RPC 2.0 Specification](https://www.jsonrpc.org/specification)

---

## Lifecycle and Destination

RFCs have the following status transitions:

```
┌─────────────┐
│   Draft     │  ← Author creates
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Under      │  ← Community discussion
│  Review     │
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  Accepted   │    │  Rejected   │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   accepted/ │    │  rejected/  │
│ (official   │    │ (rejected)  │
│  design)    │    │             │
└─────────────┘    └─────────────┘
```

### Status Description

| Status           | Location                  | Description                                                   |
| ---------------- | ------------------------- | ------------------------------------------------------------- |
| **Draft**        | `docs/design/rfc/draft/`  | Author's draft, awaiting submission for review                |
| **Under Review** | `docs/design/rfc/review/` | Open community discussion and feedback                        |
| **Accepted**     | `docs/design/accepted/`   | Becomes official design document, enters implementation phase |
| **Rejected**     | `docs/design/rfc/`        | Retained in RFC directory, status updated                     |

### Actions After Acceptance

1. Move the RFC to the `docs/design/accepted/` directory
2. Update the file name to a descriptive name (e.g., `lsp-support.md`)
3. Update the status to "Official"
4. Update the status to "Accepted" and add the acceptance date

### Actions After Rejection

1. Retain in the `docs/design/rfc/draft/` directory
2. Add rejection reason and date at the top of the file
3. Update the status to "Rejected"

### Actions After Discussion Consensus

When consensus is reached on an open question:

1. **Update Appendix A**: write the "Resolution" under the discussion topic
2. **Update the main text**: sync the decision into the document body
3. **Record the decision**: add to "Appendix B: Design Decision Record"
4. **Mark the question**: check `[x]` in the "Open Questions" list

---

> **Note**: RFC numbers are only used during the discussion phase. After acceptance, remove the
> number and use a descriptive file name.

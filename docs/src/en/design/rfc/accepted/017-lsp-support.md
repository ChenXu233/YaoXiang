---
title: 'RFC-017: Language Server Protocol (LSP) Support Design'
status: 'Implemented'
author: '晨煦'
created: '2026-02-15'
updated: '2026-07-05'

issue: '#11'
---

# RFC-017: Language Server Protocol (LSP) Support Design

>

>

>

> **Reference**: See the [full example](../EXAMPLE_full_feature_proposal.md) to learn how to write
> an RFC.

## ⚠️ Implementation Prerequisites (Important)

Before implementing LSP, the following two core issues need to be resolved first:

### Issue 1: Diagnostic Error Collection

**Current state**: The current type checker returns immediately upon encountering the first error
(using the `?` operator), unable to collect all errors.

**LSP requirements**: The IDE needs to display **all** errors, not just the first one.

**Solution**:

#### 1.1 Error Collection Mode

- Modify the `src/frontend/typecheck/inference/` module to return `Result<Type, Vec<Error>>`
- Do not return immediately upon encountering an error; continue checking instead
- Return all errors collectively after checking is complete

#### 1.2 Error Levels

Differentiate errors by severity:

```rust
enum ErrorKind {
    Error,      // Severe errors that may cause cascading errors
    Warning,    // Warnings, continue checking but do not block
    Note,       // Additional information
}
```

- If there is an `Error`: `publishDiagnostics` displays the error
- If only `Warning`: continue compiling and display warnings

#### 1.3 Parser Error Recovery

- When parsing fails, insert **placeholder nodes** (e.g. `MissingExpression`) instead of giving up
- Avoid panics in type checking due to incomplete AST
- Example: `let x = ;` → `let x = MissingExpression`

#### 1.4 Delayed Emission

- Some errors may be "cascading" (caused by previous errors)
- Can be collected first and filtered after the AST is fully parsed
- Or handle it simply: report all, let users fix them one by one

### Issue 2: File-Level Parsing Cache

**Current state**: Every LSP request re-parses the entire file; no caching mechanism exists.

**LSP requirements**: Every edit should be responded to quickly without re-parsing unchanged files.

**Solution**:

#### 2.1 File Cache Structure

```rust
struct DocumentCache {
    version: u32,           // LSP document version number
    content: String,        // Current content
    content_hash: u64,      // Content hash (for quick comparison)
    ast: Option<Ast>,       // Cached AST (optional)
}
```

#### 2.2 Detecting Changes

- Receive new content on every `textDocument/didChange`
- Compute the hash of the new content and compare it with the cached `content_hash`
- **If changed: re-parse the entire file**
- **If unchanged: return the cached result directly**

#### 2.3 Re-parsing Strategy

- **File-level**: Only re-parse the current file, not the entire project
- This is a simplified design; no function-level incremental parsing
- Modern computers only need a few milliseconds to parse a single file of several thousand lines

#### 2.4 Difference from cargo check

|           | cargo check            | YaoXiang LSP              |
| --------- | ---------------------- | ------------------------- |
| Scope     | Entire project         | Single file               |
| Frequency | Manually triggered     | Every edit                |
| Goal      | Full compilation check | Fast incremental response |

### Integration with Existing Modules

| Existing module                  | LSP integration method                                                        |
| -------------------------------- | ----------------------------------------------------------------------------- |
| `util/span.rs`                   | ✅ Already has `Position`/`Span`, maps directly to LSP `Position`             |
| `util/diagnostic/collect.rs`     | ⚠️ Needs to be changed to "collection mode" to continuously accumulate errors |
| `frontend/core/lexer/symbols.rs` | ⚠️ Needs to be extended to add `uri` + `span` location info                   |
| `frontend/typecheck/mod.rs`      | ⚠️ Needs to modify `TypeResult` to return all errors                          |
| `frontend/core/parser/ast.rs`    | ✅ Every node already has `Span`, no changes needed                           |

---

## Summary

Add Language Server Protocol (LSP) support to YaoXiang, implementing a complete language server that
enables mainstream IDEs (VS Code, Neovim, Emacs, etc.) to provide development tooling features such
as code completion, go-to-definition, diagnostics, and reference search.

## Motivation

### Why is this feature needed?

The current YaoXiang language lacks official IDE integration support. Developers can only use basic
text editors to write code, missing out on:

1. **Code Completion** - Unable to intelligently complete identifiers, keywords, and types based on
   context
2. **Go-to-Definition** - Unable to quickly jump to the definition of a function, type, or variable
3. **Real-time Diagnostics** - Unable to display syntax and type errors instantly while editing
4. **Reference Search** - Unable to find all reference locations of a symbol
5. **Hover Information** - Unable to display type information and documentation comments on mouse
   hover

LSP is a standard feature of modern programming languages. Mainstream languages (Rust, Python,
TypeScript, Go, etc.) all provide mature LSP implementations. Implementing LSP support will
significantly improve the YaoXiang development experience.

### Current Problems

1. **Low Development Efficiency** - Lack of code completion and smart hints
2. **Difficult Debugging** - Unable to quickly locate symbol definitions
3. **Steep Learning Curve** - Lack of IDE assistance features
4. **Incomplete Ecosystem** - Unable to attract developers accustomed to modern IDEs

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

    subgraph World_Compile [Compile World]
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

    Cache -- "Incremental update" --> World_Compile

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
│   ├── initialize.rs   # Initialization handler
│   ├── text_document.rs # Document operation handler
│   ├── completion.rs   # Completion handler
│   ├── definition.rs   # Go-to-definition handler
│   ├── references.rs   # Reference search handler
│   ├── hover.rs        # Hover information handler
│   └── diagnostics.rs  # Diagnostics handler
├── world.rs            # Compile world (symbol table, AST cache)
├── scroller.rs         # Symbol index builder
├── protocol.rs         # LSP protocol type definitions
└── cache/              # Incremental cache module (new)
    ├── mod.rs
    ├── document.rs     # Document cache (version, AST, symbol table)
    └── incremental.rs  # Incremental parsing strategy
```

### Compile World Design

Manages global compilation state:

- Document cache (version, AST, symbol table)
- Global symbol index
- Error collector
- Type environment cache

Core methods:

- `on_document_change`: handle incremental changes
- `incremental_reparse`: incremental re-parse
- `collect_diagnostics`: collect all errors (non-blocking)

### Core LSP Method Support

| Category          | Methods                                            | Description             |
| ----------------- | -------------------------------------------------- | ----------------------- |
| **Lifecycle**     | `initialize` / `initialized` / `shutdown` / `exit` | Server lifecycle        |
| **Document Sync** | `didOpen` / `didChange` / `didClose`               | Document management     |
| **Diagnostics**   | `publishDiagnostics`                               | Publish diagnostics     |
| **Completion**    | `completion`                                       | Code completion         |
| **Navigation**    | `definition`                                       | Go to definition        |
| **References**    | `references`                                       | Find references         |
| **Hover**         | `hover`                                            | Hover information       |
| **Symbols**       | `workspace/symbol`                                 | Workspace symbol search |

### Text Document Sync Mechanism

Use an incremental sync strategy:

- Maintain document version numbers
- Apply incremental changes (range + text)
- Fall back to full replacement for large changes

### Symbol Index Construction

Leverage the existing symbol table system to build a reverse index:

- Extend `SymbolEntry` to add a `location` field
- Index: name → list of locations, file → list of symbols

### Code Completion Implementation

Completion sources: keywords, variables, functions, types, struct fields, modules

### Go-to-Definition Implementation

AST-based symbol resolution: find the definition location corresponding to an identifier/function
call

## Detailed Design

### Impact on the Type System

1. **Symbol Information Extension** - Add location information (file, line, column) to the symbol
   table
2. **Type Information Exposure** - Provide a type query interface for LSP
3. **Documentation Comment Integration** - Support generating documentation strings from comments

### Runtime Behavior

- The LSP server runs as an independent process
- Uses stdin/stdout for JSON-RPC communication
- Supports concurrent handling of multiple sessions

### Compiler Changes

| Component                     | Changes                                        |
| ----------------------------- | ---------------------------------------------- |
| `frontend/events`             | Extend event system, support LSP notifications |
| `frontend/core/lexer/symbols` | Enhance symbol table, add location information |
| New `src/lsp/`                | LSP server implementation                      |

### Backward Compatibility

- ✅ Fully backward compatible
- The LSP server is an independent component that does not affect the existing compilation flow
- Existing CLI tools are not affected

### Integration with Existing Systems

1. **Event System** - Leverage the event subscription mechanism in `frontend/events/`
2. **Diagnostic System** - Reuse the diagnostic output in `util/diagnostic/`
   - Reuse `ErrorCollector<E>` to collect all errors
   - Convert `Diagnostic` to LSP's `Diagnostic` format
3. **Symbol Table** - Extend the symbol location capabilities in `symbols.rs`
   - Extend `SymbolEntry`, add `location: Location` field
   - Build `SymbolIndex` reverse index (name -> list of locations)
4. **Compiler Frontend** - Directly call Lexer, Parser, and type checking
   - **Key change**: The type checker must be changed to "collection mode" without blocking
     execution

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
development experience that other languages cannot:

### 1. Inlay Hints

- **Constant value hints**: Display compile-time computed constants (e.g. show `300` next to
  `const MAX = 100 + 200`)
- **Mutability hints**: Display whether a variable is mutable (e.g. `mut x`, `x` has an obvious
  underline)
- **Ownership consumption hints**: Display whether function parameters are consumed (e.g. `consumed`
  / `borrowed`)
- **Empty ownership semantic hints**: Show that a variable can be reassigned after being moved by
  fading its color
- **Type inference hints**: Display the inferred concrete type (e.g. show `Vec<i32>` next to
  `x = vec![]`)

### 2. Ownership Semantics Visualization

- Display the move path of a variable (from definition to all usage locations)
- Borrow lifetime visualization

### 3. Compile-Time Evaluation Preview

- Hovering displays the compile-time computation result of constant expressions

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

| Mode               | Use case                             |
| ------------------ | ------------------------------------ |
| stdio              | Local development (default)          |
| TCP Socket         | Remote development/debugging         |
| Unix Domain Socket | High-performance local communication |

### Remote Debugging

Implemented based on DAP (Debug Adapter Protocol):

- Supports line breakpoints, function breakpoints, conditional breakpoints
- YaoXiang-specific breakpoints: triggered when a variable is moved

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

**Design decision: Single-threaded + async event loop**

Reasons:

- The compiler is not thread-safe; the refactoring cost is high
- LSP requests are naturally serial; no concurrency needed
- Single-threaded is simpler and easier to debug
- Single-threaded async I/O provides sufficient performance

Background tasks use `spawn_blocking` to leverage multiple cores.

---

## LSP Built-in Test Tool (Optional)

> This feature is not required for MVP and can be added in a later version.

Provides a JSON test case format:

```bash
# Run tests
yaoxiang-lsp --test
```

---

## Trade-offs

### Pros

1. **Improved Development Experience** - IDE support close to mainstream languages
2. **Ecosystem Improvement** - Attract more developers to use YaoXiang
3. **Code Quality Improvement** - Real-time diagnostics reduce runtime errors
4. **Community Contributions** - Developers can participate in LSP toolchain development

### Cons

1. **High Implementation Complexity** - Need to handle many LSP edge cases
2. **Maintenance Cost** - Need to keep up with LSP protocol version updates
3. **Performance Considerations** - Indexing and query performance for large projects
4. **Testing Difficulty** - Need to simulate IDE behavior for testing

## Alternatives

| Alternative                      | Why not chosen                                     |
| -------------------------------- | -------------------------------------------------- |
| Provide syntax highlighting only | Cannot meet modern development needs               |
| Use Tree-sitter                  | Additional learning cost and limited functionality |

## Implementation Strategy

### Phases

1. **Phase 0 (Prerequisite)**: Compiler Adaptation ⚠️ **Critical**
   - Modify the type checker to "collection mode", return `Result<Type, Vec<Error>>`
   - Implement error levels (Error / Warning / Note)
   - Parser error recovery: insert placeholder nodes
   - Extend symbol table `SymbolEntry`, add `location` field
   - Implement DocumentCache caching system (version + content + hash)
   - **This phase is a prerequisite for LSP implementation and must be completed first**

2. **Phase 1 (v0.7)**: Basic Framework
   - LSP server skeleton
   - Lifecycle methods (initialize/shutdown/exit)
   - Basic logging and error handling

3. **Phase 2 (v0.7)**: Diagnostics Support
   - Text document synchronization
   - Compilation diagnostics integration
   - `textDocument/publishDiagnostics`

4. **Phase 3 (v0.8)**: Completion Support
   - Symbol index construction
   - Keyword completion
   - Identifier completion

5. **Phase 4 (v0.8)**: Navigation Support
   - Go to definition
   - Find references
   - Hover information

6. **Phase 5 (v0.9)**: Advanced Features
   - Workspace symbol search
   - Code formatting
   - Refactoring support (optional)

### Dependencies

- No external LSP library dependencies (use the `lsp-types` crate)
- Depends on existing compiler frontend modules
- Depends on `serde_json` for JSON-RPC serialization

### Risks

1. **Performance Issues** - Parsing large files may cause lag
   - Solution: Incremental parsing, background thread processing
2. **Memory Usage** - Symbol index occupies memory
   - Solution: Lazy loading, LRU cache
3. **Protocol Compatibility** - LSP version differences
   - Solution: Declare supported protocol versions

## Open Questions

- [x] Error collection mechanism (see "Implementation Prerequisites" section)
- [x] Incremental cache system (see "Implementation Prerequisites" section)
- [x] LSP protocol version: use 3.18 (supports new features like Inlay Hints, Inline Values)
- [x] Remote communication support (via TCP, balancing LSP + debugging)
- [x] Remote debugging support (based on DAP protocol)
- [x] Concurrency model: single-threaded + async event loop
- [x] LSP built-in test tool (optional): use JSON test cases

---

## Appendix (Optional)

### Appendix A: Design Discussion Records

> Used to record detailed discussions during the design decision process.

### Appendix B: Design Decision Records

| Decision                | Decision                                                                   | Date       | Recorder |
| ----------------------- | -------------------------------------------------------------------------- | ---------- | -------- |
| LSP server architecture | Independent process, communicating via stdio                               | 2026-02-15 | 晨煦     |
| Protocol version        | Support LSP 3.18 (requires new features like Inlay Hints)                  | 2026-02-22 | 晨煦     |
| Error collection mode   | Return `Result<Type, Vec<Error>>`, support error levels and error recovery | 2026-02-22 | 晨煦     |
| Cache strategy          | File-level cache: version + content + hash, re-parse entire file           | 2026-02-22 | 晨煦     |
| Communication mode      | Support stdio + TCP + UnixSocket                                           | 2026-02-22 | 晨煦     |
| Remote debugging        | Based on DAP protocol, share transport layer with LSP                      | 2026-02-22 | 晨煦     |
| Concurrency model       | Single-threaded + async event loop                                         | 2026-02-22 | 晨煦     |
| Test tool (optional)    | JSON test cases + built-in test runner                                     | 2026-02-22 | 晨煦     |

### Appendix C: Glossary

| Term            | Definition                                                |
| --------------- | --------------------------------------------------------- |
| LSP             | Language Server Protocol                                  |
| JSON-RCP        | JSON-Remote Procedure Call                                |
| DAP             | Debug Adapter Protocol                                    |
| Symbol Index    | A mapping table of symbol locations built at compile time |
| Compile World   | The context containing all compilation information        |
| Inlay Hints     | Inline hint information displayed within the line         |
| Ownership Trace | Visualization of the flow of variable ownership           |

---

## References

- [Language Server Protocol Specification](https://microsoft.github.io/language-server-protocol/)
- [LSP Specification 3.18](https://github.com/microsoft/language-server-protocol/blob/main/specifications/specification-3-18.md)
- [Debug Adapter Protocol Specification](https://microsoft.github.io/debug-adapter-protocol/)
- [Rust Analyzer](https://rust-analyzer.github.io/) - Reference implementation
- [lsp-types crate](https://crates.io/crates/lsp-types) - LSP type definitions
- [JSON-RPC 2.0 Specification](https://www.jsonrpc.org/specification)

---

## Lifecycle and Destination

RFCs have the following state transitions:

```
┌─────────────┐
│   Draft     │  ← Author creates
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Reviewing  │  ← Community discussion
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
│  accepted/  │    │  rejected/  │
│ (Official design) │ (Rejected)│
└─────────────┘    └─────────────┘
```

### State Descriptions

| State         | Location                  | Description                                                |
| ------------- | ------------------------- | ---------------------------------------------------------- |
| **Draft**     | `docs/design/rfc/draft/`  | Author draft, awaiting submission for review               |
| **Reviewing** | `docs/design/rfc/review/` | Open community discussion and feedback                     |
| **Accepted**  | `docs/design/accepted/`   | Becomes an official design document, enters implementation |
| **Rejected**  | `docs/design/rfc/`        | Kept in the RFC directory, status updated                  |

### Actions After Acceptance

1. Move the RFC to the `docs/design/accepted/` directory
2. Update the filename to a descriptive name (e.g. `lsp-support.md`)
3. Update the status to "Official"
4. Update the status to "Accepted", add the acceptance date

### Actions After Rejection

1. Keep it in the `docs/design/rfc/draft/` directory
2. Add the rejection reason and date at the top of the file
3. Update the status to "Rejected"

### Actions After Discussion Consensus

When consensus is reached on an open question:

1. **Update Appendix A**: Fill in the "Resolution" under the discussion topic
2. **Update the main text**: Sync the decision into the document body
3. **Record the decision**: Add to "Appendix B: Design Decision Records"
4. **Mark the question**: Check `[x]` in the "Open Questions" list

---

> **Note**: The RFC number is only used during the discussion phase. After acceptance, remove the
> number and use a descriptive filename.

# LSP Support Implementation Plan

> **Task**: Implement YaoXiang Language Server Protocol (LSP) support
> **Based on RFC**: RFC-017 Language Server Protocol (LSP) Support Design
> **Date**: 2026-02-23
> **Status**: In Progress
> **Target Version**: v0.7 - v0.9

---

## Overview

This plan, based on the RFC-017 document, decomposes LSP implementation into 6 phases with a total of 20 sub-steps. Each step includes detailed implementation objectives, acceptance criteria, and test items.

### Dependency Overview

```
Phase 0 (Prerequisites) ──────┐
    │               │
    ▼               │
Phase 1 ──────────────┼──►  Phase 2 ──►  Phase 3 ──►  Phase 4 ──►  Phase 5
                       │         │         │         │
                       └─────────┴─────────┴─────────┘
                                 (Can be developed in parallel)
```

---

## Phase 0: Compiler Prerequisites ✅ Completed

> **Importance**: This phase is a prerequisite for LSP implementation and must be completed first
> **Target Version**: v0.6 (in parallel with LSP server development)
> **Completion Date**: 2025-07

### 0.1 Error Collection Pattern

**Implementation Objectives**:
- Modify the `src/frontend/typecheck/inference/` module to return `Result<Type, Vec<Error>>` instead of returning immediately on error
- Implement the `ErrorKind` enum, containing `Error` (critical error), `Warning` (warning), and `Note` (additional information)
- Error collector continuously accumulates errors and returns all errors uniformly after the check is complete

**Acceptance Criteria**:
- [x] Type checker returns all errors for a single file (non-short-circuit return)
- [x] Errors contain Severity level information
- [x] When Error exists, publishDiagnostics displays errors
- [x] When only Warning exists, compilation continues and warnings are displayed

**Implementation Notes**:
- `StatementChecker` adds `collect_all_errors` mode; errors no longer short-circuit return but accumulate to `collected_errors: Vec<Diagnostic>`
- `TypeChecker::check_module_collect_all()` provides a full error collection entry point for LSP
- Reuses existing `Severity` enum (Error/Warning/Info/Hint)
- Modified files: `src/frontend/typecheck/inference/statements.rs`, `src/frontend/typecheck/mod.rs`

**Test Items**:
- [x] Single file multi-error collection test (at least 3 type errors)
- [x] Error/Warning/Note level differentiation test
- [x] Error accumulation and unified return test
- [x] Regression test: existing correct code behavior unchanged

---

### 0.2 Parser Error Recovery

**Implementation Objectives**:
- When parsing encounters errors, insert `MissingExpression`, `MissingStatement` and other placeholder nodes
- Avoid type check panics caused by incomplete AST
- Example: `x = ;` → `x = MissingExpression`

**Acceptance Criteria**:
- [x] Parser generates placeholder nodes instead of panicking when encountering syntax errors
- [x] Placeholder nodes have reasonable Span information
- [x] Type checker can handle placeholder nodes (report errors but no panic)

**Implementation Notes**:
- AST adds `Expr::Error(Span)` and `StmtKind::Error(Span)` placeholder variants
- `parse_with_recovery()` always returns `ParseResult` (containing Module + error list), never fails
- Both `ExpressionInferrer` and `StatementChecker` can handle Error variants (report `invalid_syntax` error but no panic)
- Modified files: `src/frontend/core/parser/ast.rs`, `src/frontend/core/parser/mod.rs`, `src/frontend/core/parser/parser_state.rs`, `src/frontend/typecheck/inference/expressions.rs`, `src/middle/core/ir_gen.rs`

**Test Items**:
- [x] Syntax error recovery test (missing expression, semicolon, brackets, etc.)
- [x] Continuous error recovery test
- [x] Placeholder node Span correctness test
- [x] Error cascade scenario test

---

### 0.3 Symbol Table Location Extension

**Implementation Objectives**:
- Extend `SymbolEntry` structure by adding `location: Location` field (file path, line number, column number)
- Build `SymbolIndex` reverse index (name → list of locations)
- Support fast lookup of symbol definition locations

**Acceptance Criteria**:
- [x] SymbolEntry contains complete location information
- [x] Can quickly query all definition locations by name
- [x] Can query all symbols in a file by file

**Implementation Notes**:
- `SymbolEntry` adds `location: Option<SymbolLocation>` field; `SymbolLocation` contains `file_path` and `Span`
- `SymbolTable` adds `insert_with_location()` and `insert_full()` methods
- New `SymbolIndex` reverse index structure, supporting `by_name` and `by_file` bidirectional queries
- Methods include: `find_by_name()`, `find_by_file()`, `from_table()`, `remove_file()`, etc.
- Modified file: `src/frontend/core/lexer/symbols.rs`

**Test Items**:
- [x] Symbol location information correctness test
- [x] Name-to-location mapping test
- [x] Multi-file symbol index test
- [x] Symbol overload/duplicate name handling test

---

### 0.4 Document Cache System (DocumentCache)

**Implementation Objectives**:
- Implement `DocumentCache` structure, containing:
  - `version: u32` - LSP document version number
  - `content: String` - Current content
  - `content_hash: u64` - Content hash (fast comparison)
  - `ast: Option<Ast>` - Cached AST
- Implement incremental change detection (compare content_hash)
- File-level cache: re-parse entire file when changes are detected

**Acceptance Criteria**:
- [x] DocumentCache correctly manages version numbers
- [x] Hash detection quickly identifies unchanged documents
- [x] Correctly re-parses when changes are detected
- [x] Reasonable memory usage (has cleanup mechanism)

**Implementation Notes**:
- `DocumentCache` structure: version, content, content_hash, ast (`Option<Module>`), file_path, dirty
- `DocumentStore` manages all open documents, `HashMap<String, DocumentCache>`, supports capacity limits and automatic cleanup
- Content hash uses `DefaultHasher`; `update()` only updates content and invalidates AST cache when hash changes
- Cleanup strategy: when exceeding `max_documents` (default 128), remove documents with the lowest version number
- Includes complete test suite (7 unit tests)
- Modified file: `src/util/cache.rs`

**Test Items**:
- [x] Version number increment test
- [x] Hash detection accuracy test
- [x] Incremental change application test
- [x] Cache cleanup/expiration test
- [ ] Large file cache performance test (supplemented in later phases)

---

## Phase 1: LSP Basic Framework (v0.7) ✅ Completed

### 1.1 Project Structure Creation

**Implementation Objectives**:
- Create `src/lsp/` directory structure
- Introduce `lsp-types` crate dependency
- Configure Cargo.toml

**Directory Structure**:
```
src/lsp/
├── main.rs              # LSP server entry point
├── server.rs           # Server core logic
├── session.rs          # Session management
├── capabilities.rs     # Server capability declaration
├── handlers/
│   ├── mod.rs
│   ├── initialize.rs   # Initialize handler
│   ├── text_document.rs # Document operation handler
│   ├── completion.rs   # Completion handler
│   ├── definition.rs   # Go-to definition handler
│   ├── references.rs   # References search handler
│   ├── hover.rs        # Hover tooltip handler
│   └── diagnostics.rs  # Diagnostics handler
├── world.rs            # Compilation world
├── scroller.rs         # Symbol index construction
├── protocol.rs         # LSP protocol type definitions
└── cache/              # Incremental cache module
    ├── mod.rs
    ├── document.rs     # Document cache
    └── incremental.rs  # Incremental parsing strategy
```

**Acceptance Criteria**:
- [x] Directory structure created
- [x] Dependencies correctly introduced (lsp-types 0.97, lsp-server 0.7, serde_json, tokio, etc.)
- [x] Basic modules compile successfully

**Implementation Notes**:
- Create `src/lsp/` directory, containing `mod.rs`, `server.rs`, `session.rs`, `capabilities.rs`, `protocol.rs`, `world.rs`, `handlers/`
- Add `lsp-types = "0.97"` and `lsp-server = "0.7"` dependencies to Cargo.toml
- `lib.rs` registers `pub mod lsp`
- `main.rs` adds `yaoxiang lsp` subcommand entry
- Handler submodules: initialize, text_document, diagnostics (implemented); completion, definition, references, hover (placeholder)

**Test Items**:
- [x] Module compilation test
- [x] Dependency version compatibility test

---

### 1.2 Lifecycle Methods Implementation

**Implementation Objectives**:
- Implement `initialize` request handling (return serverCapabilities)
- Implement `initialized` notification handling
- Implement `shutdown` / `exit` request handling
- Declare supported LSP protocol version (3.18)

**Acceptance Criteria**:
- [x] initialize returns correct serverCapabilities
- [x] All supported standard methods respond correctly
- [x] Correctly handle client closing connection

**Implementation Notes**:
- `handle_initialize()`: returns ServerCapabilities (currently supports TextDocumentSync Full mode) + ServerInfo
- `handle_initialized()`: session enters Running state
- `handle_shutdown()`: clears document cache, session enters ShuttingDown state
- `exit` notification ends main loop
- Session state machine: Uninitialized → Initializing → Running → ShuttingDown
- Unknown methods return MethodNotFound error
- Modified files: `src/lsp/handlers/initialize.rs`, `src/lsp/server.rs`, `src/lsp/session.rs`

**Test Items**:
- [x] initialize request/response test
- [x] shutdown/exit flow test
- [x] capabilities declaration completeness test

---

### 1.3 Basic Logging and Error Handling

**Implementation Objectives**:
- Configure logging system (env_logger or tracing)
- Implement JSON-RPC error response
- Format error messages into readable logs

**Acceptance Criteria**:
- [x] Output configuration information at startup
- [x] Return correct error response for erroneous requests
- [x] Logs include request/response key information

**Implementation Notes**:
- Reuse project's existing `tracing` logging system; log info level for each request/notification
- `protocol.rs` implements JSON-RPC response building functions: `ok_response()`, `error_response()`, `method_not_found()`, `internal_error()`, `notification()`
- Supports ErrorCode: MethodNotFound, InternalError, InvalidRequest, etc.
- Modified file: `src/lsp/protocol.rs`

**Test Items**:
- [x] Log output test
- [x] Error response format test
- [x] Abnormal request handling test

---

## Phase 2: Diagnostics Support (v0.7) ✅ Completed

### 2.1 Text Document Synchronization

**Implementation Objectives**:
- Implement `textDocument/didOpen` notification handling
- Implement `textDocument/didChange` notification handling
- Implement `textDocument/didClose` notification handling
- Integrate DocumentCache for document state management

**Acceptance Criteria**:
- [x] didOpen correctly parses and caches documents
- [x] didChange correctly updates document content
- [x] didClose correctly cleans up document cache
- [x] Document version numbers correctly managed

**Test Items**:
- [x] didOpen/didChange/didClose complete flow test
- [x] Incremental change test
- [x] Multi-document management test
- [x] Concurrent change test

---

### 2.2 Diagnostics Integration

**Implementation Objectives**:
- Reuse `util/diagnostic/` diagnostics system
- Convert YaoXiang Diagnostic to LSP Diagnostic
- Implement diagnostics format conversion functions

**Conversion Rules**:
```
YaoXiang Severity::Error   → LSP DiagnosticSeverity::ERROR
YaoXiang Severity::Warning → LSP DiagnosticSeverity::WARNING
YaoXiang Severity::Info    → LSP DiagnosticSeverity::INFORMATION
```

**Acceptance Criteria**:
- [x] Type errors convert to correct severity
- [x] Syntax errors correctly reported
- [x] Location information accurate (line numbers 0-indexed)

**Test Items**:
- [x] Error type conversion test
- [x] Location offset correctness test
- [x] Multi-error diagnostics test

---

### 2.3 publishDiagnostics Publishing

**Implementation Objectives**:
- Implement `textDocument/publishDiagnostics` notification
- Automatically trigger diagnostics after document changes
- Support incremental diagnostics updates

**Acceptance Criteria**:
- [x] Correctly send publishDiagnostics notification
- [x] Diagnostics contain file uri and version number
- [x] Send empty diagnostics when errors are cleared

**Test Items**:
- [x] Diagnostics publishing test
- [x] Error clearing test
- [x] Version number matching test

---

## Phase 3: Completion Support (v0.8) ✅ Completed

### 3.1 Symbol Index Construction

**Implementation Objectives**:
- Implement symbol index of World structure
- Build: name → list of locations reverse index
- Implement file → list of symbols index

**Acceptance Criteria**:
- [x] Can obtain context symbols based on cursor position
- [x] Completion response time < 100ms
- [x] Index supports incremental updates

**Test Items**:
- [x] Symbol index construction test
- [x] Index query performance test
- [x] Incremental update test

---

### 3.2 Keyword Completion

**Implementation Objectives**:
- Implement YaoXiang keyword completion
- Support keyword suggestion sorting

**Keyword List** (based on language-spec.md Section 2.3, total 17):
```
pub         # Public declaration
use         # Module import
spawn       # spawn function marker
ref         # Arc reference count sharing
mut         # Mutable binding
if          # Conditional branch
elif        # Else if
else        # Else branch
match       # Pattern matching
while       # Conditional loop
for         # Iterative loop
return      # Function return
break       # Loop break
continue    # Loop continue
as          # Type casting
in          # for loop iteration
unsafe      # Unsafe code block
```

**Reserved Words** (based on language-spec.md Section 2.4, total 7):
```
Type        # Meta type (used in type definitions)
true        # Bool true value
false       # Bool false value
void        # Void null value
some(T)     # Option value variant constructor
ok(T)       # Result success variant constructor
err(E)      # Result error variant constructor
```

**Function Annotations** (based on language-spec.md Section 6.9.1):
```
@block      # Disable concurrent optimization
@eager      # Force eager evaluation
```

**Acceptance Criteria**:
- [x] All 17 keywords appear in completion list
- [x] All 7 reserved words appear in completion list
- [x] Both function annotations (@block, @eager) appear in completion list
- [x] Keywords correctly categorized (keywords/reserved words/annotations)

**Test Items**:
- [x] Keyword completion test (pub, use, spawn, ref, mut, if, elif, else, match, while, for, return, break, continue, as, in, unsafe)
- [x] Reserved word completion test (Type, true, false, void, some, ok, err)
- [x] Function annotation completion test (@block, @eager)
- [x] Context-dependent keyword test (e.g., if/elif/else appear as a group)

---

### 3.3 Identifier Completion

**Implementation Objectives**:
- Completion based on symbols in current scope
- Completion based on symbols from imported modules
- Support type prefix filtering (e.g., `Vec::`)

**Acceptance Criteria**:
- [x] Current file symbols are completable
- [x] Imported module symbols are completable
- [x] Completion items contain kind information (keyword, function, variable, type)

**Test Items**:
- [x] Variable name completion test
- [x] Function name completion test
- [x] Type name completion test
- [x] Module member completion test
- [x] Completion trigger test (after typing characters)

---

## Phase 4: Navigation Support (v0.8) ✅ Completed

### 4.1 Go to Definition

**Implementation Objectives**:
- Implement `textDocument/definition` handling
- Find identifier definition location based on AST
- Support navigation for functions, structs, variables, and type definitions

**Acceptance Criteria**:
- [x] Function calls navigate to function definition
- [x] Variable references navigate to variable definition
- [x] Type usage navigates to type definition
- [x] Support cross-file navigation

**Test Items**:
- [x] Function definition navigation test
- [x] Variable definition navigation test
- [x] Type definition navigation test
- [x] Cross-file navigation test
- [x] Multiple definitions (same name) handling test

---

### 4.2 Find References

**Implementation Objectives**:
- Implement `textDocument/references` handling
- Find all reference locations of a symbol
- Exclude the definition itself

**Acceptance Criteria**:
- [x] Return all reference locations
- [x] Does not include definition location
- [x] References include definition location information

**Test Items**:
- [x] Variable reference search test
- [x] Function reference search test
- [x] Cross-file reference search test

---

### 4.3 Hover Tooltip

**Implementation Objectives**:
- Implement `textDocument/hover` handling
- Display symbol type information
- Display function signature and documentation comments

**Acceptance Criteria**:
- [x] Variables display inferred type
- [x] Functions display function signature
- [x] Constants display computed value

**Test Items**:
- [x] Variable hover test
- [x] Function hover test
- [x] Constant hover test
- [x] Cross-file hover test

---

## Phase 5: Advanced Features (v0.9) ✅ Completed

### 5.1 Workspace Symbol Search

**Implementation Objectives**:
- Implement `workspace/symbol` handling
- Support fuzzy search
- Support symbol type filtering

**Acceptance Criteria**:
- [x] Fuzzy matching search results correct
- [x] Search response time < 500ms
- [x] Support file filtering

**Test Items**:
- [x] Fuzzy search test
- [x] Symbol type filtering test
- [x] Performance test (large workspace)

---

### 5.2 Formatting Support (Optional)

**Implementation Objectives**:
- Implement `textDocument/formatting` handling
- Implement `textDocument/rangeFormatting` handling
- Define YaoXiang code style

**Acceptance Criteria**:
- [x] Basic formatting correct (indentation, spaces)
- [x] Range formatting correct

**Test Items**:
- [x] Full file formatting test
- [x] Range formatting test
- [x] Formatting performance test

---

### 5.3 Refactoring Support (Optional)

**Implementation Objectives**:
- Implement symbol rename (textDocument/rename)
- Implement code actions (textDocument/codeAction)

**Acceptance Criteria**:
- [x] Rename updates all references
- [x] Preview changes

**Test Items**:
- [x] Symbol rename test
- [x] Reference update test

---

## Advanced Features (Future Versions)

### Inlay Hints ✅ Completed

**Priority**: P0

| Feature | Implementation Objectives |
|---------|---------------------------|
| Constant value hints | Display pre-computed compile-time constants (e.g., display `300` beside `const MAX = 100 + 200`) |
| Mutability hints | Display whether variables are mutable (e.g., `mut x` has visible marker) |
| Ownership consumption hints | Display whether function parameters are consumed |
| Type inference hints | Display inferred concrete types (e.g., display `Vec<i32>` beside `x = vec()`) |

**Acceptance Criteria**:
- [x] All Inlay Hints display correctly
- [x] Performance impact < 50ms

---

### Ownership Semantics Visualization

**Priority**: P2

**Implementation Objectives**:
- Display variable move paths (from definition location to all usage locations)
- Borrow lifetime visualization

---

## Testing Strategy

### Unit Tests
- Independent unit tests for each module
- Use mocks to isolate dependencies

### Integration Tests
- LSP protocol compatibility tests
- Integration tests with real IDEs (VS Code, Neovim)

### Performance Tests
- Large file parsing performance
- Completion response time
- Navigation response time

---

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Performance issues | Incremental parsing, background thread processing |
| Memory usage | LRU cache, lazy loading |
| Protocol compatibility | Declare supported protocol version |

---

## References

- [Language Server Protocol Specification](https://microsoft.github.io/language-server-protocol/)
- [LSP Specification 3.18](https://github.com/microsoft/language-server-protocol/blob/main/specifications/specification-3-18.md)
- [lsp-types crate](https://crates.io/crates/lsp-types)
- [Rust Analyzer](https://rust-analyzer.github.io/) - Reference implementation
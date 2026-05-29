# LSP Support Implementation Plan

> **Task**: Implement YaoXiang Language Server Protocol (LSP) support
> **Based on RFC**: RFC-017 Language Server Protocol (LSP) Support Design
> **Date**: 2026-02-23
> **Status**: In Progress
> **Target Version**: v0.7 - v0.9

---

## Overview

This plan, based on RFC-017 document, decomposes LSP implementation into 6 phases with a total of 20 sub-steps. Each step includes detailed implementation goals, acceptance criteria, and test items.

### Dependency Overview

```
Phase 0 (Prerequisites) ──────┐
    │                         │
    ▼                         │
Phase 1 ─────────────────────┼──► Phase 2 ──► Phase 3 ──► Phase 4 ──► Phase 5
                              │         │         │         │
                              └─────────┴─────────┴─────────┘
                                        (Parallelizable Development)
```

---

## Phase 0: Compiler Prerequisites ✅ Completed

> **Importance**: This phase is a prerequisite for LSP implementation and must be completed first
> **Target Version**: v0.6 (parallel with LSP server development)
> **Completion Date**: 2025-07

### 0.1 Error Collection Pattern

**Implementation Goal**:
- Modify the `src/frontend/typecheck/inference/` module to return `Result<Type, Vec<Error>>` instead of returning immediately on error
- Implement `ErrorKind` enum with `Error` (severe error), `Warning` (warning), and `Note` (additional information)
- Error collector continuously accumulates errors, returning all errors after the check is complete

**Acceptance Criteria**:
- [x] Type checker returns all errors for a single file (non-short-circuit return)
- [x] Errors include Severity level information
- [x] When Error exists, publishDiagnostics displays errors
- [x] When only Warning exists, continue compilation and display warnings

**Implementation Notes**:
- `StatementChecker` adds `collect_all_errors` mode; errors no longer short-circuit but accumulate into `collected_errors: Vec<Diagnostic>`
- `TypeChecker::check_module_collect_all()` provides full error collection entry point for LSP
- Reuses existing `Severity` enum (Error/Warning/Info/Hint)
- Modified files: `src/frontend/typecheck/inference/statements.rs`, `src/frontend/typecheck/mod.rs`

**Test Items**:
- [x] Single file multi-error collection test (at least 3 type errors)
- [x] Error/Warning/Note level distinction test
- [x] Error accumulation and unified return test
- [x] Regression test: existing correct code behavior unchanged

---

### 0.2 Parser Error Recovery

**Implementation Goal**:
- When parsing encounters errors, insert placeholder nodes like `MissingExpression`, `MissingStatement`, etc.
- Prevent type check panics due to incomplete AST
- Example: `x = ;` → `x = MissingExpression`

**Acceptance Criteria**:
- [x] Parser generates placeholder nodes instead of panicking when encountering syntax errors
- [x] Placeholder nodes have reasonable Span information
- [x] Type checker handles placeholder nodes (reports error but does not panic)

**Implementation Notes**:
- AST adds `Expr::Error(Span)` and `StmtKind::Error(Span)` placeholder variants
- `parse_with_recovery()` function always returns `ParseResult` (containing Module + error list), never fails
- Both `ExpressionInferrer` and `StatementChecker` can handle Error variants (reports `invalid_syntax` error but does not panic)
- Modified files: `src/frontend/core/parser/ast.rs`, `src/frontend/core/parser/mod.rs`, `src/frontend/core/parser/parser_state.rs`, `src/frontend/typecheck/inference/expressions.rs`, `src/middle/core/ir_gen.rs`

**Test Items**:
- [x] Syntax error recovery test (missing expressions, semicolons, parentheses, etc.)
- [x] Continuous error recovery test
- [x] Placeholder node Span correctness test
- [x] Error cascade scenario test

---

### 0.3 Symbol Table Location Extension

**Implementation Goal**:
- Extend `SymbolEntry` structure to add `location: Location` field (file path, line number, column number)
- Build `SymbolIndex` reverse index (name → list of locations)
- Support fast lookup of symbol definition locations

**Acceptance Criteria**:
- [x] SymbolEntry contains complete location information
- [x] Can quickly query all definition locations by name
- [x] Can query all symbols in a file by file

**Implementation Notes**:
- `SymbolEntry` adds `location: Option<SymbolLocation>>` field; `SymbolLocation` contains `file_path` and `Span`
- `SymbolTable` adds `insert_with_location()` and `insert_full()` methods
- New `SymbolIndex` reverse index structure supports `by_name` and `by_file` bidirectional queries
- Methods include: `find_by_name()`, `find_by_file()`, `from_table()`, `remove_file()`, etc.
- Modified file: `src/frontend/core/lexer/symbols.rs`

**Test Items**:
- [x] Symbol location information correctness test
- [x] Name to location mapping test
- [x] Multi-file symbol index test
- [x] Symbol overload/duplicate name handling test

---

### 0.4 Document Cache System (DocumentCache)

**Implementation Goal**:
- Implement `DocumentCache` structure containing:
  - `version: u32` - LSP document version number
  - `content: String` - Current content
  - `content_hash: u64` - Content hash (fast comparison)
  - `ast: Option<Ast>>` - Cached AST
- Implement incremental change detection (compare content_hash)
- File-level cache: re-parse entire file when changed

**Acceptance Criteria**:
- [x] DocumentCache correctly manages version numbers
- [x] Hash detection quickly identifies unchanged documents
- [x] Correctly re-parses when changed
- [x] Reasonable memory footprint (has cleanup mechanism)

**Implementation Notes**:
- `DocumentCache` structure: version, content, content_hash, ast (`Option<Module>>`), file_path, dirty
- `DocumentStore` manages all open documents, `HashMap<String, DocumentCache>`, supports capacity limits and auto-cleanup
- Content hash uses `DefaultHasher`; `update()` only updates content and invalidates AST cache when hash changes
- Cleanup strategy: When exceeding `max_documents` (default 128), removes documents with lowest version number
- Contains complete test suite (7 unit tests)
- Modified file: `src/util/cache.rs`

**Test Items**:
- [x] Version number increment test
- [x] Hash detection accuracy test
- [x] Incremental change application test
- [x] Cache cleanup/expiration test
- [ ] Large file cache performance test (to be added in subsequent phases)

---

## Phase 1: LSP Basic Framework (v0.7) ✅ Completed

### 1.1 Project Structure Creation

**Implementation Goal**:
- Create `src/lsp/` directory structure
- Introduce `lsp-types` crate dependency
- Configure Cargo.toml

**Directory Structure**:
```
src/lsp/
├── main.rs              # LSP server entry
├── server.rs           # Server core logic
├── session.rs          # Session management
├── capabilities.rs     # Server capability declaration
├── handlers/
│   ├── mod.rs
│   ├── initialize.rs   # Initialization handler
│   ├── text_document.rs # Document operation handler
│   ├── completion.rs   # Completion handler
│   ├── definition.rs   # Go-to definition handler
│   ├── references.rs   # Reference search handler
│   ├── hover.rs        # Hover tooltip handler
│   └── diagnostics.rs  # Diagnostics handler
├── world.rs            # Compilation world
├── scroller.rs         # Symbol index builder
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
- Create `src/lsp/` directory containing `mod.rs`, `server.rs`, `session.rs`, `capabilities.rs`, `protocol.rs`, `world.rs`, `handlers/`
- Cargo.toml adds `lsp-types = "0.97"` and `lsp-server = "0.7"` dependencies
- `lib.rs` registers `pub mod lsp`
- `main.rs` adds `yaoxiang lsp` subcommand entry
- Handler submodules: initialize, text_document, diagnostics (implemented); completion, definition, references, hover (placeholders)

**Test Items**:
- [x] Module compilation test
- [x] Dependency version compatibility test

---

### 1.2 Lifecycle Method Implementation

**Implementation Goal**:
- Implement `initialize` request handler (return serverCapabilities)
- Implement `initialized` notification handler
- Implement `shutdown` / `exit` request handlers
- Declare supported LSP protocol version (3.18)

**Acceptance Criteria**:
- [x] initialize returns correct serverCapabilities
- [x] All supported standard methods respond correctly
- [x] Correctly handle client closing connection

**Implementation Notes**:
- `handle_initialize()`: returns ServerCapabilities (currently supports TextDocumentSync Full mode) + ServerInfo
- `handle_initialized()`: session enters Running state
- `handle_shutdown()`: cleans up document cache, session enters ShuttingDown state
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

**Implementation Goal**:
- Configure logging system (env_logger or tracing)
- Implement JSON-RPC error responses
- Format error messages into readable logs

**Acceptance Criteria**:
- [x] Output configuration information at startup
- [x] Error requests return correct error response
- [x] Logs contain key request/response information

**Implementation Notes**:
- Reuse project's existing `tracing` logging system; every request/notification logs info-level
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

**Implementation Goal**:
- Implement `textDocument/didOpen` notification handler
- Implement `textDocument/didChange` notification handler
- Implement `textDocument/didClose` notification handler
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

**Implementation Goal**:
- Reuse `util/diagnostic/` diagnostics system
- Convert YaoXiang Diagnostic to LSP Diagnostic
- Implement diagnostics format conversion function

**Conversion Rules**:
```
YaoXiang Severity::Error   → LSP DiagnosticSeverity::ERROR
YaoXiang Severity::Warning → LSP DiagnosticSeverity::WARNING
YaoXiang Severity::Info    → LSP DiagnosticSeverity::INFORMATION
```

**Acceptance Criteria**:
- [x] Type errors converted to correct severity
- [x] Syntax errors correctly reported
- [x] Location information accurate (line number 0-indexed)

**Test Items**:
- [x] Error type conversion test
- [x] Location offset correctness test
- [x] Multi-error diagnostics test

---

### 2.3 publishDiagnostics Publishing

**Implementation Goal**:
- Implement `textDocument/publishDiagnostics` notification
- Automatically trigger diagnostics after document changes
- Support incremental diagnostics updates

**Acceptance Criteria**:
- [x] Correctly send publishDiagnostics notification
- [x] Diagnostics include file uri, version number
- [x] Send empty diagnostics when errors are cleared

**Test Items**:
- [x] Diagnostics publishing test
- [x] Error clearing test
- [x] Version number matching test

---

## Phase 3: Completion Support (v0.8) ✅ Completed

### 3.1 Symbol Index Construction

**Implementation Goal**:
- Implement symbol index for World structure
- Build: name → list of locations reverse index
- Implement file → symbol list index

**Acceptance Criteria**:
- [x] Can get context symbols at cursor position
- [x] Completion response time < 100ms
- [x] Index supports incremental updates

**Test Items**:
- [x] Symbol index construction test
- [x] Index query performance test
- [x] Incremental update test

---

### 3.2 Keyword Completion

**Implementation Goal**:
- Implement YaoXiang keyword completion
- Support keyword suggestion sorting

**Keyword List** (based on language-spec.md section 2.3, 17 total):
```
pub         # Public declaration
use         # Module import
spawn       # spawn function marker
ref         # Arc reference counting shared
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
as          # Type cast
in          # for loop iteration
unsafe      # Unsafe code block
```

**Reserved Words** (based on language-spec.md section 2.4, 7 total):
```
Type        # meta type (used in type definitions)
true        # Bool true value
false       # Bool false value
void        # Void null value
some(T)     # Option value variant constructor
ok(T)       # Result success variant constructor
err(E)      # Result error variant constructor
```

**Function Annotations** (based on language-spec.md section 6.9.1):
```
@block      # Disable concurrent optimization
@eager      # Force eager evaluation
```

**Acceptance Criteria**:
- [x] All 17 keywords appear in completion list
- [x] All 7 reserved words appear in completion list
- [x] Both 2 function annotations (@block, @eager) appear in completion list
- [x] Keywords correctly categorized (keywords/reserved words/annotations)

**Test Items**:
- [x] Keyword completion test (pub, use, spawn, ref, mut, if, elif, else, match, while, for, return, break, continue, as, in, unsafe)
- [x] Reserved word completion test (Type, true, false, void, some, ok, err)
- [x] Function annotation completion test (@block, @eager)
- [x] Context-dependent keyword test (e.g., if/elif/else appear together)

---

### 3.3 Identifier Completion

**Implementation Goal**:
- Completion based on symbols in current scope
- Completion based on imported module symbols
- Support type prefix filtering (e.g., `Vec::`)

**Acceptance Criteria**:
- [x] Current file symbols can be completed
- [x] Imported module symbols can be completed
- [x] Completion items include kind information (keyword, function, variable, type)

**Test Items**:
- [x] Variable name completion test
- [x] Function name completion test
- [x] Type name completion test
- [x] Module member completion test
- [x] Completion trigger test (after typing characters)

---

## Phase 4: Navigation Support (v0.8) ✅ Completed

### 4.1 Go to Definition

**Implementation Goal**:
- Implement `textDocument/definition` handler
- Find identifier definition location based on AST
- Support jumps for functions, structs, variables, type definitions

**Acceptance Criteria**:
- [x] Function calls jump to function definitions
- [x] Variable references jump to variable definitions
- [x] Type usage jumps to type definitions
- [x] Support cross-file jumps

**Test Items**:
- [x] Function definition jump test
- [x] Variable definition jump test
- [x] Type definition jump test
- [x] Cross-file jump test
- [x] Multiple definitions (same name) handling test

---

### 4.2 Find References

**Implementation Goal**:
- Implement `textDocument/references` handler
- Find all reference locations of a symbol
- Exclude the definition itself

**Acceptance Criteria**:
- [x] Returns all reference locations
- [x] Does not include definition location
- [x] References include definition location information

**Test Items**:
- [x] Variable reference find test
- [x] Function reference find test
- [x] Cross-file reference find test

---

### 4.3 Hover Tooltips

**Implementation Goal**:
- Implement `textDocument/hover` handler
- Display symbol type information
- Display function signatures and documentation comments

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

**Implementation Goal**:
- Implement `workspace/symbol` handler
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

**Implementation Goal**:
- Implement `textDocument/formatting` handler
- Implement `textDocument/rangeFormatting` handler
- Define YaoXiang code style

**Acceptance Criteria**:
- [x] Basic formatting correct (indentation, spacing)
- [x] Range formatting correct

**Test Items**:
- [x] Whole file formatting test
- [x] Range formatting test
- [x] Formatting performance test

---

### 5.3 Refactoring Support (Optional)

**Implementation Goal**:
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

| Feature | Implementation Goal |
|---------|---------------------|
| Constant Value Hints | Display compile-time computed constants (e.g., show `300` beside `const MAX = 100 + 200`) |
| Mutability Hints | Show whether a variable is mutable (e.g., `mut x` has obvious marker) |
| Ownership Consumption Hints | Show whether function parameters are consumed |
| Type Inference Hints | Show inferred specific types (e.g., show `Vec<i32>` beside `x = vec()`) |

**Acceptance Criteria**:
- [x] All Inlay Hint types display correctly
- [x] Performance impact < 50ms

---

### Ownership Semantics Visualization

**Priority**: P2

**Implementation Goal**:
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
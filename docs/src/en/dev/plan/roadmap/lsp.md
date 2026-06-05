---
title: "Language Server Status"
---

# Language Server (LSP)

> **Module Status**: Stable (5 items pending improvement)
> **Location**: `src/lsp/`
> **Last Updated**: 2026-06-01

---

## Module Overview

The LSP module implements the Language Server Protocol, providing code intelligence features for editors/IDEs. The implementation scope covers RFC-017 phases 0-5, and has ahead-of-schedule implemented advanced features such as semantic tokens, rename, and code actions.

**Code Size**: 7,542 lines (21 Rust files)

---

## Feature List

### Implemented LSP Features (13 items)

| Feature | File | Status | Description |
|---------|------|--------|-------------|
| **Lifecycle Management** | `handlers/initialize.rs` | ✅ | initialize/shutdown/exit/initialized, with session state machine |
| **Document Synchronization** | `handlers/text_document.rs` | ✅ | didOpen/didChange/didClose, full sync mode |
| **Diagnostic Publishing** | `handlers/diagnostics.rs` | ✅ | tokenize + parse_with_recovery + check_module_collect_all pipeline |
| **Code Completion** | `handlers/completion.rs` | ✅ | 17 keywords + 7 reserved words + 2 annotations + identifier completion |
| **Go to Definition** | `handlers/definition.rs` | ✅ | SemanticDB exact match + global symbol index fallback, supports cross-file |
| **Find References** | `handlers/references.rs` | ✅ | Variable/function reference lookup, supports cross-file |
| **Hover Information** | `handlers/hover.rs` | ✅ | Variable type, function signature (parameter count/generics), type definition info |
| **Rename** | `handlers/rename.rs` | ✅ | Symbol rename, generates WorkspaceEdit |
| **Code Actions** | `handlers/code_action.rs` | ✅ | Diagnostic-based quick fixes + refactoring suggestions |
| **Semantic Tokens** | `handlers/semantic_tokens.rs` | ✅ | full + full/delta modes, with version caching |
| **Document Formatting** | `handlers/formatting.rs` | ✅ | Full document formatting + range formatting |
| **Inlay Hints** | `handlers/inlay_hint.rs` | ✅ | Type inference hints, constant value hints, mutability hints |
| **Workspace Symbol Search** | `handlers/workspace_symbol.rs` | ✅ | Fuzzy matching, filter by symbol type |

### Core Supporting Modules

| Module | File | Function |
|--------|------|----------|
| **Compilation World** | `world.rs` (1,019 lines) | Symbol index, Semantic Database, Standard library symbol loading, AST symbol extraction |
| **Cursor Positioning** | `locate.rs` | Identifier lookup, Span↔Range conversion, find all occurrences |
| **Session Management** | `session.rs` | Lifecycle state machine (Uninitialized→Initializing→Running→ShuttingDown) |
| **Protocol Utilities** | `protocol.rs` | JSON-RPC message construction, error code definitions |
| **Capability Declaration** | `capabilities.rs` | Server capability declaration, includes semantic tokens legend |

---

## Test Coverage

**145 unit tests**, distributed as follows:

| File | Test Count |
|------|------------|
| workspace_symbol.rs | 18 |
| completion.rs | 16 |
| server.rs | 15 |
| semantic_tokens.rs | 15 |
| world.rs | 10 |
| locate.rs | 10 |
| diagnostics.rs | 10 |
| hover.rs | 8 |
| definition.rs | 8 |
| capabilities.rs | 6 |
| protocol.rs | 5 |
| references.rs | 5 |
| Others | 19 |

---

## RFC Comparison (RFC-017)

| Phase | RFC Design Content | Implementation Status | Difference Notes |
|-------|-------------------|----------------------|------------------|
| **Phase 0 (Prerequisite)** | Error collection mode, Parser error recovery, DocumentCache, Extended symbol table | ✅ Completed | Uses `check_module_collect_all` for collection mode |
| **Phase 1 (v0.7)** | LSP server skeleton, lifecycle methods | ✅ Completed | Fully implemented |
| **Phase 2 (v0.7)** | Text document synchronization, diagnostic support | ✅ Completed | Fully implemented |
| **Phase 3 (v0.8)** | Symbol index construction, code completion | ✅ Completed | Fully implemented, supports keyword/reserved word/annotation/identifier completion |
| **Phase 4 (v0.8)** | Go to definition, find references, hover information | ✅ Completed | Fully implemented, SemanticDB exact match + global index fallback |
| **Phase 5 (v0.9)** | Workspace symbol search, code formatting | ✅ Completed | Fully implemented, includes fuzzy matching |

### Additional Implementations Beyond RFC Design

| Feature | RFC Planned Version | Description |
|---------|---------------------|-------------|
| **Semantic Tokens** | v0.10 | Implemented ahead of schedule, supports full + delta modes |
| **Rename** | v0.9 (mentioned in RFC) | Implemented |
| **Code Actions** | v0.9 (mentioned in RFC) | Implemented quick fixes |
| **Inlay Hints** | RFC Advanced Feature | Implemented type inference/constant value/mutability hints |

### Unimplemented RFC Designs

| Feature | RFC Status | Description |
|---------|------------|-------------|
| **Incremental Sync** | RFC design | Currently uses full sync (TextDocumentSyncKind::FULL) |
| **TCP/Unix Socket Communication** | RFC design | Currently supports stdio only |
| **Remote Debugging (DAP)** | RFC design | Not implemented |
| **Ownership Visualization** | RFC Advanced Feature | Not implemented |
| **Compile-time Evaluation Preview** | RFC Advanced Feature | Not implemented |

---

## Code Quality Assessment

| Dimension | Score | Description |
|-----------|-------|-------------|
| Unfinished Items | 5 | Incremental sync, TCP/Unix Socket, DAP, visualization, compile-time evaluation preview |
| Test Coverage | Excellent | 145 unit tests |
| Documentation Quality | Excellent | Complete module/function-level documentation, includes ASCII architecture diagrams |
| Code Architecture | Excellent | Clear layering: handlers/world/locate/session/protocol |
| RFC Compliance | Exceeds Expectations | Implementation scope exceeds RFC design |

---

## Pending Improvements

1. **Implement Incremental Sync** (TextDocumentSyncKind::INCREMENTAL)
2. **Implement TCP/Unix Socket Remote Communication**
3. **Implement DAP Debugger Adapter**
4. **Implement Ownership Semantics Visualization**
5. **Implement Compile-time Evaluation Preview**
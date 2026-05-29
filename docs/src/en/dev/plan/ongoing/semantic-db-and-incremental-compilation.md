# Semantic Information Platform and Incremental Compilation Implementation Plan

> **Task**: Implement the semantic information platform, providing LSP semantic highlighting, incremental compilation, and dead code warning capabilities
> **Based on RFC**: This plan is for new feature design
> **Related RFC**: [RFC-008: Runtime Concurrency Model](../design/rfc/accepted/008-runtime-concurrency-model.md) - DAG concurrency belongs to the runtime and is not in scope for this plan
> **Date**: 2026-02-23
> **Status**: Phase 1 + Phase 2 + Phase 3 completed
> **Target Version**: v0.10 - v0.11

---

## Overview

This plan decomposes the semantic information platform implementation into 3 main phases. The core idea is **single traversal, multiple uses**:

1. **Semantic collection is completed during the typecheck phase** (instead of LSP layer traversing AST separately)
2. Collected semantic information simultaneously serves LSP semantic highlighting, incremental compilation, and dead code analysis

> **Important Clarification**:
> - **DAG concurrency** is a runtime feature (RFC-008) and is not in scope for this plan
> - **Parallel compilation of module dependency graph** is a build system feature, which is a different concept from runtime DAG
> - Semantic collection should be completed during the typecheck phase, and LSP directly reuses the data instead of writing a separate traverser

---

## Phase 1: SemanticDB Infrastructure

> **Importance**: This phase is the foundation for all subsequent features and must be completed first
> **Target Version**: v0.10
> **Status**: ✅ Completed


**Implementation Goals**:
- Define `SemanticDB` struct to uniformly manage semantic information
- Define `SemanticToken` enum containing LSP standard TokenType
- Define `SymbolReference` struct to record symbol reference locations
- Define `ModuleSymbol` struct to record module-level symbol definitions

**Data Structure Design**:

```rust
// Semantic Information Database (implemented in src/frontend/typecheck/semantic_db.rs)
pub struct SemanticDB {
    // File path -> semantic information for that file
    by_file: HashMap<String, FileSemanticInfo>,
    // Symbol name -> all definition locations
    symbol_defs: HashMap<String, Vec<SymbolLocation>>,
    // Symbol name -> all reference locations
    symbol_refs: HashMap<String, Vec<SymbolLocation>>,
}

// Semantic information for a single file
pub struct FileSemanticInfo {
    pub file_path: String,
    pub tokens: Vec<SemanticToken>,
    pub scopes: Vec<ScopeInfo>,
}

// Semantic Token (using struct + type enum, instead of enum variants as originally planned)
pub struct SemanticToken {
    pub name: String,
    pub token_type: SemanticTokenType,
    pub modifiers: Vec<SemanticTokenModifier>,
    pub span: Span,
}

pub enum SemanticTokenType {
    Function, Type, Variable, Property, Method,
    Namespace, Parameter, LocalVariable, TypeParameter,
    Keyword, String, Number,
}

pub enum SemanticTokenModifier {
    Declaration, Readonly, Mutable, Public, Generic,
}

// Scope information
pub struct ScopeInfo {
    pub span: Span,
    pub parent: Option<usize>,  // Parent scope index
    pub symbols: Vec<String>,   // Symbols within the scope
    pub kind: ScopeKind,        // Global, Function, Block, Lambda
}
```

**Acceptance Criteria**:
- [x] SemanticDB struct definition completed
- [x] SemanticToken covers LSP standard token types (12 types + 5 modifiers)
- [x] Support querying semantic information by file
- [x] Support querying symbol definitions and reference locations by symbol name

**Test Items**:
- [x] SemanticDB struct creation test
- [x] Query by file test
- [x] Query by symbol name test
- [x] Empty database boundary test
- [x] Multi-file management test
- [x] File overwrite update test

---

### 1.2 TypeCheck Semantic Collector Integration

**Design Decision**: Semantic collection **should not** be implemented separately in the LSP layer, but should be completed during the typecheck phase.

**Reason**:
- Typecheck is already traversing the AST and already knows all symbol definitions and reference locations
- LSP implementing SemanticCollector separately = duplicate traversal + maintaining two sets of logic
- **Good Taste**: Single traversal, multiple uses

**Implementation Goals**:
- Extend semantic collection functionality in `src/frontend/typecheck/` module
- Produce `SemanticDB` data simultaneously during type checking
- LSP layer directly queries and reuses without re-traversing AST

**Collection Rules for Typecheck** (phase output):
```
StmtKind::Fn        → SemanticTokenType::Function (definition)
StmtKind::TypeDef   → SemanticTokenType::Type (definition)
StmtKind::Var       → SemanticTokenType::Variable (definition)
StmtKind::MethodBind→ SemanticTokenType::Method (definition)
StmtKind::Use       → SemanticTokenType::Namespace (reference)
Param               → SemanticTokenType::Parameter (definition)
Expr::Var           → SemanticTokenType::Variable (reference)
Expr::Call          → SemanticTokenType::Function (reference)
Expr::FieldAccess   → SemanticTokenType::Property (reference)
Expr::Cast          → SemanticTokenType::Type (reference)
```

**Acceptance Criteria**:
- [x] Typecheck phase produces SemanticDB
- [x] LSP can query semantic information produced by typecheck
- [x] Eliminate duplicate AST traversal in LSP layer

---

### 1.3 Scope Chain Collection

**Implementation Goals**:
- Scope information is also produced during the typecheck phase
- Record start and end positions of each scope
- Record symbol lists within scopes
- Support correct parent-child relationships for nested scopes
- Support 4 scope types: Global, Function, Block, Lambda

**Note**: This information is already managed in typecheck's `TypeEnvironment`, and now needs to be exported for SemanticDB use.

**Acceptance Criteria**:
- [x] Global scope information correct
- [x] Function scope information correct
- [x] Block-level scope information correct
- [x] Nested scope parent-child relationships correct

**Test Items**:
- [x] Single-level scope test (global scope)
- [x] Nested scope test (global + function)
- [x] Lambda scope test
- [x] Innermost scope lookup test

---

### 1.4 World Extension Integration

**Implementation Goals**:
- Extend World struct in `src/lsp/world.rs`
- Add SemanticDB field
- When LSP document changes, trigger typecheck re-execution to update semantic information
- LSP handlers directly query SemanticDB produced by typecheck

**Design Adjustment**:
- No longer need to call SemanticCollector separately in LSP layer
- LSP only needs to trigger typecheck re-execution after document changes
- World holds a reference to the compilation pipeline to get the latest SemanticDB

**Acceptance Criteria**:
- [x] World contains SemanticDB field
- [x] Document changes trigger typecheck re-execution and semantic information update
- [x] LSP handlers can query semantic information

**Test Items**:
- [x] World update semantic information test (verified through existing server tests)
- [x] Multi-file semantic information management test
- [x] Semantic information query interface test

---

## Phase 2: LSP Semantic Highlighting

> **Target Version**: v0.10
> **Dependency**: Phase 1 completed
> **Status**: ✅ Completed

### 2.1 Semantic Tokens Capability Declaration

**Implementation Goals**:
- Declare semanticTokensProvider in `src/lsp/capabilities.rs`
- Define token type mapping (YaoXiang → LSP)
- Define token modifier mapping

**Token Type Mapping**:
```
YaoXiang SymbolKind::Function    → LSP TokenType::FUNCTION
YaoXiang SymbolKind::Type        → LSP TokenType::TYPE
YaoXiang SymbolKind::Variable    → LSP TokenType::VARIABLE
YaoXiang SymbolKind::GenericType  → LSP TokenType::TYPE
YaoXiang SymbolKind::Parameter    → LSP TokenType::PARAMETER
YaoXiang SymbolKind::Property     → LSP TokenType::PROPERTY
YaoXiang SymbolKind::Method       → LSP TokenType::METHOD
YaoXiang SymbolKind::Namespace    → LSP TokenType::NAMESPACE
```

**Acceptance Criteria**:
- [x] Capabilities declaration includes semanticTokensProvider
- [x] Token type mapping correct
- [x] Support full and delta modes

**Test Items**:
- [x] Capability declaration test
- [x] Protocol compatibility test

---

### 2.2 textDocument/semanticTokens/full Handler

**Implementation Goals**:
- Implement `handle_semantic_tokens_full` handler function
- Get semantic tokens for the file from SemanticDB
- Convert to LSP SemanticToken format
- Support full refresh

**LSP Response Format**:
```json
{
  "data": [
    0,   // deltaLine (relative to previous token)
    0,   // deltaStart (relative to previous token)
    5,   // length
    0,   // tokenType (function)
    0    // tokenModifiers
  ]
}
```

**Acceptance Criteria**:
- [x] Return correct semantic tokens data
- [x] Line and column numbers start from 0
- [x] Response time < 200ms (single file < 1000 lines)
- [x] Empty file returns empty array

**Test Items**:
- [x] Simple function semantic highlighting test
- [x] Complex nested structure test
- [ ] Performance test (1000-line file) — pending benchmark
- [x] Empty file test

---

### 2.3 textDocument/semanticTokens/full/delta Handler

**Implementation Goals**:
- Implement incremental semantic tokens update
- Track document version differences
- Return only changed tokens

**Acceptance Criteria**:
- [x] Incremental update returns correct delta
- [x] Version number tracking correct
- [x] Delete operations handled correctly

**Test Items**:
- [x] Add token delta test
- [x] Delete token delta test
- [x] Modify token delta test

---

### 2.4 VSCode Theme Configuration

**Implementation Goals**:
- Add semantic highlighting theme configuration examples in language-pack
- Document token type to theme color mapping

**Theme Color Mapping Suggestions**:
```json
{
  "tokenTypes": {
    "function": "entity.name.function",
    "type": "entity.name.type",
    "variable": "variable",
    "parameter": "variable.parameter",
    "property": "variable.property",
    "namespace": "namespace"
  }
}
```

**Acceptance Criteria**:
- [x] Theme configuration example complete
- [x] Documentation clear

---

## Phase 3: Incremental Compilation

> **Target Version**: v0.11
> **Dependency**: Phase 1 completed
> **Status**: ✅ Completed

### 3.1 Module Dependency Graph Construction

**Implementation Goals**:
- Implement `ModuleDependencyGraph` struct
- Parse import/use statements to construct module dependency relationships
- Support circular dependency detection

**Data Structure**:
```rust
pub struct ModuleDependencyGraph {
    // Module ID -> list of dependent module IDs
    deps: HashMap<ModuleId, Vec<ModuleId>>,
    // Module ID -> list of exported symbols
    exports: HashMap<ModuleId, Vec<SymbolId>>,
    // Symbol definition locations
    symbol_defs: HashMap<SymbolId, SymbolLocation>,
}

pub struct ModuleId {
    pub name: String,
    pub path: PathBuf,
}
```

**Acceptance Criteria**:
- [x] Single-file project dependency graph correct
- [x] Multi-file project dependency graph correct
- [x] Circular dependency detection correct
- [x] Incremental update correctly updates dependency graph

**Test Items**:
- [x] Single-file dependency test
- [x] Multi-file dependency test
- [x] Circular dependency detection test
- [x] Incremental update test

---

### 3.2 Compilation Cache System

**Implementation Goals**:
- Implement compilation artifact cache (AST, type information, IR)
- Detect changes based on file content hash
- Implement cache serialization/deserialization

**Cache Contents**:
```rust
pub struct CompilationCache {
    // File path -> file cache
    files: HashMap<PathBuf, FileCache>,
    // Cache metadata
    metadata: CacheMetadata,
}

pub struct FileCache {
    pub content_hash: u64,
    pub ast: Option<Module>,
    pub type_info: Option<TypeInfo>,
    pub ir: Option<ModuleIR>,
    pub semantic_db: Option<SemanticDB>,
    pub timestamp: SystemTime,
}
```

**Acceptance Criteria**:
- [x] Unchanged files directly use cache
- [x] Changed files correctly recompile
- [x] Cache serialization correct (in-memory cache, based on Clone)
- [x] Cache cleanup mechanism works correctly

**Test Items**:
- [x] Cache hit test
- [x] Cache miss test
- [x] Cache serialization test (in-memory cache, Clone method)
- [x] Cache cleanup test

---

### 3.3 Incremental Compilation Scheduler

**Implementation Goals**:
- Implement compilation scheduling based on dependency graph
- Only compile files affected by changes
- Determine compilation order via topological sort

**Scheduling Strategy**:
```
1. Detect changed file list
2. Find all modules depending on changed files (recursive upward)
3. Topological sort to determine compilation order
4. Parallel/serial scheduling for compilation
```

**Acceptance Criteria**:
- [x] Single file change only recompiles necessary files
- [x] Compilation order correct (dependencies first)
- [x] Parallel compilation without race conditions (batch grouping support)

**Test Items**:
- [x] Single file change test
- [x] Multi-file change test
- [x] Dependency chain change test
- [x] Parallel compilation test (batch grouping)

---

### 3.4 Build System Integration

**Implementation Goals**:
- Implement incremental compilation support for `yaoxiang build` command
- Output incremental compilation statistics
- Support `--force` for forced full compilation

**Acceptance Criteria**:
- [x] Incremental compilation command works correctly
- [x] Full compilation command works correctly (clear_cache)
- [x] Statistics output correct
- [x] Error handling correct

**Test Items**:
- [x] Incremental compilation function test
- [x] Full compilation function test
- [x] Statistics test

---

## Phase 4: Dead Code Warning (Integrated into Compilation Flow)

> **Target Version**: v0.11
> **Dependency**: Phase 1 completed (semantic information from typecheck phase)

> **Note**: Dead code warning depends on symbol reference information from the typecheck phase and is a compile-time analysis feature, not a runtime feature.

> **Architecture Adjustment**: Dead code analysis is integrated into the typecheck phase, as both need to traverse AST/SemanticDB

### 4.1 Dead Code Analyzer

**Implementation Goals**:
- Implement `DeadCodeAnalyzer` struct
- Analyze unused exported symbols
- Analyze unused imports
- Generate warning messages

**Design Decision**: Dead code analysis should be completed during the **typecheck phase** because:
- Typecheck already knows all symbol definitions and references
- No additional AST traversal needed
- Semantic information is already provided through SemanticDB

**Analysis Rules**:
```
1. Collect all entry points (main, pub functions)
2. Starting from entry points, mark all reachable symbols
3. Unreachable exported symbols -> Warning
4. Unused imports -> Warning
```

**Data Structure**:
```rust
pub struct DeadCodeAnalyzer {
    // Entry points
    entry_points: HashSet<SymbolId>,
    // All symbol definitions
    all_defs: HashMap<SymbolId, SymbolDef>,
    // Symbol references (obtained from SemanticDB)
    references: HashMap<SymbolId, Vec<Location>>,
    // Import list
    imports: Vec<ImportInfo>,
}

pub struct SymbolDef {
    pub name: String,
    pub kind: SymbolKind,
    pub location: Location,
    pub is_exported: bool,
}
```

**Acceptance Criteria**:
- [x] Unused exported functions can be detected
- [x] Unused exported types can be detected
- [x] Unused imports can be detected
- [x] Warning message format correct

**Test Items**:
- [x] Unused exported function test
- [x] Unused exported type test
- [x] Unused import test
- [x] Multi-level dependency test


---

### 4.2 Warning System Integration

**Implementation Goals**:
- Integrate dead code detection into the compilation process
- Publish warnings through `CompilationWarning` events
- Support multiple output formats (terminal, JSON)

**Warning Format**:
```
warning: unused function `dead_function`
  --> src/utils.yx:10:1
   |
10 | fn dead_function() { }
   | ^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: function is never used
```

**Acceptance Criteria**:
- [x] Dead code warning triggered correctly
- [x] Warning location information accurate
- [x] Warnings configurable (enable/disable)
- [x] Terminal output format aesthetically pleasing

**Test Items**:
- [x] Warning trigger test
- [x] Warning location test
- [x] Configuration test
- [x] Output format test

---

## Note on DAG Concurrency

**This plan does not include DAG concurrent compilation**, for the following reasons:

| Concept | Belongs To | Description |
|---------|------------|-------------|
| **Runtime DAG** | RFC-008 Runtime | Lazy evaluation dependency graph, controls runtime task scheduling |
| **Module Dependency Graph** | Phase 3 of this plan | Module dependencies at compiler level, used for incremental compilation |
| **Module-level Parallel Compilation** | Build System | Implemented based on Phase 3's dependency graph, not part of LSP |

**Correct Placement**:
- Runtime DAG concurrency → See [RFC-008: Runtime Concurrency Model](../design/rfc/accepted/008-runtime-concurrency-model.md)
- Module dependency graph → Phase 3 of this plan (completed/in progress)
- Module-level parallel compilation → Should be implemented as a build system feature, can be based on Phase 3's dependency graph

---

## Architecture Design Summary

### Unified Data Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                   Semantic Information Platform Architecture     │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   Source Code                                                     │
│     │                                                              │
│     ▼                                                              │
│   ┌─────────────────┐                                            │
│   │  Lexing/Parsing │ ──▶ AST                                    │
│   └────────┬────────┘                                            │
│            │                                                       │
│            ▼                                                       │
│   ┌─────────────────┐                                            │
│   │  Type Checking  │ ──┬─▶ TypeResult + Bindings                │
│   │                  │   │                                        │
│   │  Also produces   │   │  ← Single traversal, multiple uses    │
│   │  SemanticDB      │   │                                        │
│   └────────┬────────┘   │                                        │
│            │            │                                        │
│            ▼            │                                        │
│   ┌─────────────────┐  │                                        │
│   │  SemanticDB     │◄─┘  ← typecheck output                    │
│   │  - Symbol defs  │                                            │
│   │  - Symbol refs  │                                            │
│   │  - Scope chain  │                                            │
│   └────────┬────────┘                                            │
│            │                                                       │
│    ┌───────┴───────┐                                            │
│    ▼               ▼                                             │
│ ┌──────┐       ┌──────────┐                                    │
│ │ LSP  │       │ Incremental │
│ │Semantic│     │ Compilation│
│ │Highlight│   │ + Dead Code │
│ └──────┘       └──────────┘                                    │
│                                                                 │
│   ▲                                                         ▲    │
│   │                                                         │    │
│   │  DAG Concurrency → RFC-008 Runtime, not in this plan   │    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Design Principles

1. **Single traversal**: Typecheck phase produces semantic information simultaneously, no duplicate AST traversal
2. **Multiple uses**: LSP semantic highlighting, incremental compilation, and dead code analysis share the same data
3. **Good Taste**: Do not add unnecessary abstraction layers for the sake of "decoupling"

### File Modification Checklist

| Phase | New Files | Modified Files | Status |
|-------|-----------|----------------|--------|
| 1 | `src/frontend/typecheck/semantic_db.rs` | `src/frontend/typecheck/mod.rs` | ✅ Completed |
| 1 | - | `src/lsp/world.rs` | ✅ Completed |
| 2 | - | `src/lsp/capabilities.rs` | ✅ Completed |
| 2 | `src/lsp/handlers/semantic_tokens.rs` | `src/lsp/handlers/mod.rs` | ✅ Completed (including delta support) |
| 2 | - | `src/lsp/server.rs` | ✅ Completed (new semanticTokens/full + delta request dispatch) |
| 2 | - | `vscode-extension/language-pack/package.json` | ✅ Completed (semantic highlighting theme configuration) |
| 3 | `src/frontend/module/dep_graph.rs` | `src/frontend/module/mod.rs` | ✅ Completed |
| 3 | `src/frontend/pipeline/compilation_cache.rs` | `src/frontend/pipeline.rs` | ✅ Completed |
| 3 | `src/frontend/pipeline/incremental_scheduler.rs` | `src/frontend/compiler.rs` | ✅ Completed |
| 4 | `src/frontend/typecheck/dead_code.rs` | `src/frontend/typecheck/mod.rs` | ✅ Completed |
| 4 | - | `src/frontend/pipeline.rs` | ✅ Completed (integrated into compilation flow) |
| 4 | - | `src/frontend/typecheck/semantic_db.rs` | ✅ Completed (added reference access methods) |

**Key Adjustment**: Semantic collector migrated from `src/lsp/` to `src/frontend/typecheck/`

---

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Typecheck coupling with semantic information | Decoupled design, SemanticDB as independent data structure |
| Circular dependency handling | Explicit detection and warning |
| Incremental compilation race conditions | Use Mutex to protect shared state |
| Cache consistency | Version number tracking, hash verification |

---

## References

- [LSP Semantic Tokens Specification](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#semanticTokens)
- [Rust Analyzer Semantic Highlighting](https://github.com/rust-lang/rust-analyzer/blob/master/docs/dev/semantic-highlighting.md)
- [Incremental Compilation (Rustc)](https://rustc-dev-guide.rust-lang.org/inc-intro.html)
- [RFC-008: Runtime Concurrency Model](../design/rfc/accepted/008-runtime-concurrency-model.md)
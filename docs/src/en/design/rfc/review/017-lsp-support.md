---
title: 'RFC-017: Language Server Protocol (LSP) Support Design'
---

# RFC-017: Language Server Protocol (LSP) Support Design

> **Status**: Draft
>
> **Author**: Chenxu
>
> **Created**: 2026-02-15
>
> **Last Updated**: 2026-02-15

> **Reference**: See [Full Example](EXAMPLE_full_feature_proposal.md) for how to write an RFC.

## Summary

Add Language Server Protocol (LSP) support to YaoXiang by implementing a full-featured language server, enabling major IDEs (VS Code, Neovim, Emacs, etc.) to provide development tooling features such as code completion, go-to-definition, diagnostics, and reference search.

## Motivation

### Why is this feature needed?

Currently, the YaoXiang language lacks official IDE integration support. Developers can only write code using basic text editors, lacking:

1. **Code Completion** - Unable to intelligently complete identifiers, keywords, and types based on context
2. **Go to Definition** - Unable to quickly navigate to definition locations of functions, types, and variables
3. **Real-time Diagnostics** - Unable to immediately display syntax errors and type errors during editing
4. **Reference Search** - Unable to find all reference locations of symbols
5. **Hover Information** - Unable to display type information and documentation comments on mouse hover

LSP is a standard feature for modern programming languages. Major languages (Rust, Python, TypeScript, Go, etc.) all provide mature LSP implementations. Implementing LSP support will significantly improve the YaoXiang development experience.

### Current Problems

1. **Low Development Efficiency** - Lack of code completion and intelligent hints
2. **Difficult Debugging** - Unable to quickly locate symbol definitions
3. **Steep Learning Curve** - Lack of IDE assistance features
4. **Incomplete Ecosystem** - Unable to attract developers accustomed to modern IDEs

## Proposal

### Core Design

Implement a standalone LSP server process that communicates with IDEs via JSON-RPC:

```
┌─────────────┐     JSON-RPC      ┌─────────────┐
│   IDE       │◄────────────────► │  YaoXiang   │
│ (VS Code)   │                   │   LSP       │
└─────────────┘                   │   Server    │
                                  └──────┬──────┘
                                         │
                                         ▼
                                  ┌─────────────┐
                                  │ Compiler    │
                                  │ Frontend    │
                                  └─────────────┘
```

### LSP Server Architecture

```
src/lsp/
├── main.rs              # LSP server entry point
├── server.rs           # Server core logic
├── session.rs          # Session management
├── capabilities.rs     # Server capability declarations
├── handlers/
│   ├── mod.rs
│   ├── initialize.rs   # Initialize handler
│   ├── text_document.rs # Document operation handler
│   ├── completion.rs  # Completion handler
│   ├── definition.rs  # Go-to-definition handler
│   ├── references.rs  # References handler
│   ├── hover.rs       # Hover handler
│   └── diagnostics.rs # Diagnostics handler
├── world.rs            # Compilation world (symbol table, AST cache)
├── scroller.rs         # Symbol index construction
└── protocol.rs         # LSP protocol type definitions
```

### Core LSP Method Support

| Category | Method | Description |
|----------|--------|-------------|
| **Lifecycle** | `initialize` | Client initialization |
| | `initialized` | Server initialization complete notification |
| | `shutdown` | Shutdown request |
| | `exit` | Exit notification |
| **Document Sync** | `textDocument/didOpen` | Document open notification |
| | `textDocument/didChange` | Document change notification |
| | `textDocument/didClose` | Document close notification |
| **Diagnostics** | `textDocument/publishDiagnostics` | Publish diagnostic information |
| **Completion** | `textDocument/completion` | Code completion |
| **Go to** | `textDocument/definition` | Go to definition |
| **References** | `textDocument/references` | Find references |
| **Hover** | `textDocument/hover` | Hover information |
| **Symbols** | `workspace/symbol` | Workspace symbol search |

### Text Document Sync Mechanism

Use incremental sync strategy:

```rust
/// Text document content cache
struct Document {
    uri: DocumentUri,
    version: i32,
    content: String,
    changes: Vec<TextDocumentContentChangeEvent>,
}

impl Document {
    /// Apply incremental changes
    fn apply_changes(&mut self, changes: Vec<TextDocumentContentChangeEvent>) {
        for change in changes {
            if let Some(range) = change.range {
                // Replace specified range
                self.content.replace_range(range, &change.text);
            } else {
                // Full replacement
                self.content = change.text;
            }
        }
        self.version += 1;
    }
}
```

### Symbol Index Construction

Build a reverse index using the existing symbol table system:

```rust
/// Symbol location information
struct SymbolLocation {
    uri: DocumentUri,
    span: Span,
    name: String,
    kind: SymbolKind,
}

/// Symbol index
struct SymbolIndex {
    /// Name -> Location list
    by_name: HashMap<String, Vec<SymbolLocation>>,
    /// File -> Symbol list
    by_file: HashMap<DocumentUri, Vec<SymbolLocation>>,
}
```

### Code Completion Implementation

```rust
/// Completion item
struct CompletionItem {
    label: String,
    kind: CompletionItemKind,
    detail: Option<String>,
    documentation: Option<String>,
    insert_text: Option<String>,
}

/// Completion source
enum CompletionSource {
    Keywords,      // Keywords
    Variables,    // Variables
    Functions,    // Functions
    Types,        // Types
    Fields,       // Struct fields
    Modules,      // Modules
}
```

### Go to Definition Implementation

Symbol resolution based on AST:

```rust
/// Find symbol definition location
fn find_definition(ast: &Ast, position: Position) -> Option<Location> {
    let node = ast.find_node_at(position)?;
    match node.kind() {
        NodeKind::Identifier(name) => {
            // Look up symbol table
            world.lookup_symbol(&name)
        }
        NodeKind::FunctionCall(name) => {
            world.lookup_symbol(&name)
        }
        _ => None
    }
}
```

## Detailed Design

### Type System Impact

1. **Symbol Information Extension** - Add location information (file, line, column) to symbol table
2. **Type Information Exposure** - Provide type query interface for LSP
3. **Documentation Comment Integration** - Support generating documentation from comments

### Runtime Behavior

- LSP server runs as a separate process
- Uses stdin/stdout for JSON-RPC communication
- Supports concurrent multi-session handling

### Compiler Changes

| Component | Change |
|-----------|--------|
| `frontend/events` | Extend event system to support LSP notifications |
| `frontend/core/lexer/symbols` | Enhance symbol table with location information |
| New `src/lsp/` | LSP server implementation |

### Backward Compatibility

- ✅ Fully backward compatible
- LSP server is a standalone component, does not affect existing compilation flow
- Existing CLI tools are unaffected

### Integration with Existing Systems

1. **Event System** - Use event subscription mechanism from `frontend/events/`
2. **Diagnostic System** - Reuse diagnostic output from `util/diagnostic/`
3. **Symbol Table** - Extend symbol location capability in `symbols.rs`
4. **Compiler Frontend** - Directly invoke Lexer, Parser, type checking

## Trade-offs

### Advantages

1. **Improved Development Experience** - Near parity with mainstream language IDE support
2. **Ecosystem Improvement** - Attract more developers to use YaoXiang
3. **Code Quality Improvement** - Real-time diagnostics reduce runtime errors
4. **Community Contributions** - Developers can participate in LSP tooling development

### Disadvantages

1. **High Implementation Complexity** - Need to handle many LSP edge cases
2. **Maintenance Cost** - Need to follow LSP protocol version updates
3. **Performance Considerations** - Indexing and query performance for large projects
4. **Testing Difficulty** - Need to simulate IDE behavior for testing

## Alternative Solutions

| Solution | Why Not Chosen |
|----------|---------------|
| Syntax highlighting only | Cannot meet modern development needs |
| Use Tree-sitter | Additional learning cost, limited functionality |

## Implementation Strategy

### Phases

1. **Phase 1 (v0.7)**: Basic Framework
   - LSP server skeleton
   - Lifecycle methods (initialize/shutdown/exit)
   - Basic logging and error handling

2. **Phase 2 (v0.7)**: Diagnostics Support
   - Text document synchronization
   - Compiler diagnostics integration
   - `textDocument/publishDiagnostics`

3. **Phase 3 (v0.8)**: Completion Support
   - Symbol index construction
   - Keyword completion
   - Identifier completion

4. **Phase 4 (v0.8)**: Navigation Support
   - Go to definition
   - Find references
   - Hover information

5. **Phase 5 (v0.9)**: Advanced Features
   - Workspace symbol search
   - Code formatting
   - Refactoring support (optional)

### Dependencies

- No external LSP library dependencies (use `lsp-types` crate)
- Depends on existing compiler frontend modules
- Depends on `serde_json` for JSON-RPC serialization

### Risks

1. **Performance Issues** - Large file parsing may cause stuttering
   - Solution: Incremental parsing, background thread processing
2. **Memory Usage** - Symbol index consumes memory
   - Solution: Lazy loading, LRU cache
3. **Protocol Compatibility** - LSP version differences
   - Solution: Declare supported protocol version

## Open Questions

- [ ] LSP protocol version selection (3.16 vs 3.18)
- [ ] Whether to support remote LSP (via TCP)
- [ ] Concurrency model design (single-threaded vs multi-threaded)
- [ ] Whether to provide built-in LSP testing tools

---

## Appendix (Optional)

### Appendix A: Design Discussion Record

> Used to record detailed discussions during the design decision process.

### Appendix B: Design Decision Record

| Decision | Resolution | Date | Recorder |
|----------|------------|------|----------|
| LSP Server Architecture | Standalone process, communicate via stdio | 2026-02-15 | Chenxu |
| Protocol Version | Support LSP 3.16+ | 2026-02-15 | Chenxu |

### Appendix C: Glossary

| Term | Definition |
|------|------------|
| LSP | Language Server Protocol |
| JSON-RPC | JSON-Remote Procedure Call |
| Symbol Index | Symbol location map built at compile time |
| Compilation World | Context containing all compilation information |

---

## References

- [Language Server Protocol Specification](https://microsoft.github.io/language-server-protocol/)
- [LSP Specification 3.18](https://github.com/microsoft/language-server-protocol/blob/main/specifications/specification-3-18.md)
- [Rust Analyzer](https://rust-analyzer.github.io/) - Reference implementation
- [lsp-types crate](https://crates.io/crates/lsp-types) - LSP type definitions
- [JSON-RPC 2.0 Specification](https://www.jsonrpc.org/specification)

---

## Lifecycle and Destination

RFC has the following status transitions:

```
┌─────────────┐
│   Draft     │  ← Author creates
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  In Review  │  ← Community discussion
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  Accepted   │    │   Rejected  │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  accepted/  │    │  rejected/  │
│ (final spec)│    │  (rejected) │
└─────────────┘    └─────────────┘
```

### Status Description

| Status | Location | Description |
|--------|----------|-------------|
| **Draft** | `docs/design/rfc/draft/` | Author's draft, waiting for review submission |
| **In Review** | `docs/design/rfc/review/` | Open community discussion and feedback |
| **Accepted** | `docs/design/accepted/` | Becomes official design document, enters implementation phase |
| **Rejected** | `docs/design/rfc/` | Stays in RFC directory, status updated |

### Actions After Acceptance

1. Move RFC to `docs/design/accepted/` directory
2. Update filename to descriptive name (e.g., `lsp-support.md`)
3. Update status to "Final"
4. Update status to "Accepted", add acceptance date

### Actions After Rejection

1. Keep in `docs/design/rfc/draft/` directory
2. Add rejection reason and date at top of file
3. Update status to "Rejected"

### Actions After Discussion Resolution

When an open question reaches consensus:

1. **Update Appendix A**: Fill in "Resolution" under the discussion topic
2. **Update Main Text**: Sync decision to document body
3. **Record Decision**: Add to "Appendix B: Design Decision Record"
4. **Mark Question**: Check `[x]` in "Open Questions" list

---

> **Note**: RFC numbers are only used during the discussion phase. After acceptance, remove the number and use descriptive filename.

# Task: Implement LSP (Language Server Protocol) Support

## Overview

To improve the developer experience of YaoXiang, it is necessary to implement a language server that conforms to the Language Server Protocol. This server will be embedded in the compiler frontend, providing real-time code analysis, completion, go-to-definition, diagnostics, and other features, which can be integrated with mainstream editors (VS Code, Vim, Emacs, etc.).

## Goals

- Implement YaoXiang language server binary `yaoxiang-lsp`.
- Support the following LSP requests:
  - `textDocument/didOpen` / `didChange` / `didSave` / `didClose`: Synchronize file content.
  - `textDocument/documentSymbol`: List document symbols (functions, types, variables, etc.).
  - `textDocument/completion`: Provide code completion suggestions (based on symbols within scope).
  - `textDocument/definition`: Go to definition location.
  - `textDocument/hover`: Display type information and doc comments on hover.
  - `textDocument/publishDiagnostics`: Push syntax errors, type errors and other diagnostic information in real time.
- Implement incremental compilation/checking capabilities with the compiler to avoid full re-parsing each time.

## Specific Steps

1. **Design Server Architecture** (1 week)
   - Build based on existing compiler frontend libraries (lexical analysis, parsing, type checking).
   - Use asynchronous I/O (such as Tokio) to process LSP requests.
   - Design data structures for document caching and incremental updates.

2. **Implement Basic LSP Protocol Communication** (1 week)
   - Implement sending and receiving of JSON-RPC messages.
   - Handle lifecycle management such as initialization requests and shutdown requests.

3. **Integrate Compiler Frontend** (2 weeks)
   - Encapsulate lexical analysis, parsing, and type checking as reentrant library functions.
   - Maintain a "compilation session" for each document, supporting incremental updates (only re-parse modified parts).
   - Implement mapping from AST nodes to source code locations, used for go-to-definition and hover.

4. **Implement Diagnostics Feature** (1 week)
   - Capture errors and warnings during type checking and convert them to LSP diagnostic messages.
   - Automatically push diagnostics after file modifications.

5. **Implement Completion and Symbol Navigation** (2 weeks)
   - Build scope symbol table to provide candidate list for completion (including keywords, builtin types, variables in current scope, etc.).
   - Implement go-to-definition: Find the file and position of a symbol definition based on cursor position.
   - Implement hover information: Display type signatures and associated doc comments (if supported).

6. **Testing and Integration** (1 week)
   - Write VS Code client plugin (simple wrapper) for testing.
   - Ensure all features work in common scenarios without significant performance issues.

## Acceptance Criteria

- Server starts correctly and responds to initialization requests.
- After opening a YaoXiang file, diagnostic information (syntax errors, type errors) is displayed in the editor in real time.
- Hovering over a variable or function name displays its type information.
- Using go-to-definition on an identifier opens the corresponding file and navigates to the definition line.
- After typing a dot, record type field completion is available (requires type information).
- Server response time on large files (thousands of lines) is within an acceptable range (<500ms).

## Dependencies

- Compiler frontend library must be stable and provide APIs for obtaining AST, symbol tables, and type information.
- Incremental compilation capability needs to be implemented (at least document-level caching).
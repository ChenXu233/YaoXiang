# Task: Implement LSP (Language Server Protocol) Support

## Overview

To improve the developer experience of YaoXiang, it is necessary to implement a language server that conforms to the Language Server Protocol. This server will embed the compiler frontend, providing real-time code analysis, completion, go-to definition, diagnostics, and other features, and can be integrated with mainstream editors (VS Code, Vim, Emacs, etc.).

## Goals

- Implement YaoXiang language server binary `yaoxiang-lsp`.
- Support the following LSP requests:
  - `textDocument/didOpen` / `didChange` / `didSave` / `didClose`: Synchronize file contents.
  - `textDocument/documentSymbol`: List document symbols (functions, types, variables, etc.).
  - `textDocument/completion`: Provide code completion suggestions (based on in-scope symbols).
  - `textDocument/definition`: Jump to definition location.
  - `textDocument/hover`: Display type information and documentation comments on hover.
  - `textDocument/publishDiagnostics`: Push syntax errors, type errors, and other diagnostic information in real-time.
- Implement incremental compilation/checking capability with the compiler to avoid full re-parsing on each request.

## Specific Steps

1. **Design Server Architecture** (1 week)
   - Build upon the existing compiler frontend libraries (lexical analysis, parsing, type checking).
   - Use asynchronous I/O (e.g., Tokio) to handle LSP requests.
   - Design data structures for document caching and incremental updates.

2. **Implement Basic LSP Protocol Communication** (1 week)
   - Implement sending and receiving of JSON-RPC messages.
   - Handle lifecycle management such as initialization requests and shutdown requests.

3. **Integrate Compiler Frontend** (2 weeks)
   - Encapsulate lexical analysis, parsing, and type checking as reusable library functions.
   - Maintain a "compilation session" for each document, supporting incremental updates (only re-parse modified parts).
   - Implement mapping from AST nodes to source code locations for jump-to-definition and hover features.

4. **Implement Diagnostics Feature** (1 week)
   - Capture errors and warnings during type checking and convert them to LSP diagnostic messages.
   - Automatically push diagnostics after file modifications.

5. **Implement Completion and Symbol Navigation** (2 weeks)
   - Build a scope symbol table to provide candidate lists for completion (including keywords, builtin types, variables in current scope, etc.).
   - Implement go-to-definition: Find the file and location of a symbol's definition based on cursor position.
   - Implement hover information: Display type signatures and associated documentation comments (if supported).

6. **Testing and Integration** (1 week)
   - Write a VS Code client extension (simple wrapper) for testing.
   - Ensure all features work in common scenarios without obvious performance issues.

## Acceptance Criteria

- The server starts correctly and responds to initialization requests.
- After opening a YaoXiang file, diagnostic information (syntax errors, type errors) is displayed in real-time in the editor.
- Hovering over a variable or function name displays its type information.
- Jumping to definition on an identifier opens the corresponding file and positions the cursor at the definition line.
- After typing a dot, fields of the record type can be completed (requires type information).
- The server responds within an acceptable time range (<500ms) on large files (thousands of lines).

## Dependencies

- The compiler frontend library must be stable and provide APIs to obtain AST, symbol tables, and type information.
- Incremental compilation capability needs to be implemented (at least document-level caching).
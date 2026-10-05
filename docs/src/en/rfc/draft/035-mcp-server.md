---
title: 'RFC-035: MCP Server Support (AI Agent Integration)'
status: 'Draft'
author: 'Chen Xu'
created: '2026-07-11'
updated: '2026-07-11'
issue: '#154'
---

# RFC-035: MCP Server Support (AI Agent Integration)

## Summary

Add an MCP (Model Context Protocol) server to YaoXiang, enabling AI agents (Claude Code, Continue,
Cody, Zed, etc.) to directly query YaoXiang source code for **AST, parsing errors, types, symbols,
references, and formatting results**. Reuse the `World` backend already implemented in RFC-017, add
the `yaoxiang mcp` subcommand, single binary with dual mode, multi-process independent World.

## Motivation

### Why is this feature needed?

RFC-017 enabled YaoXiang to be understood by editors (hover / goto-def / completion). But LSP is a
**position-driven** protocol:

- Each request strongly depends on `textDocument` URI + `Position`
- Editor must first open file, save, and maintain long connection with LSP server
- AI agent workflow is **code snippet-based**: "paste a piece of code" in conversation to ask
  questions, **without** saving to disk first

AI agents actually usable LSP clients (vscode-langservers-extracted, `mcp-lsp-bridge` type projects)
**only translate L1**: goto-def, hover. What AI wants to do:

- 「Does this code **parse correctly**」—— requires parse + full diagnostic stream
- 「How is this symbol **used in the file**」—— requires lookup_symbol by name
- 「What does this code **look like after formatting**」—— requires format_source
- 「Where are **all** type errors」—— requires typecheck to run full workspace

These L1 LSP translation capabilities **cannot do it**, because LSP is not designed to support this.

### Current Problems

1. Poor AI agent LSP experience: requires mock documents, huge JSON, strong URI dependency
2. YaoXiang project lacks "AI-First" interface layer: humans use LSP in IDE, AI agents cannot use
   LSP
3. Mainstream AI agents like Claude Code / Continue have default MCP support, which is a blank
   ecosystem for YaoXiang

### What is MCP?

MCP (Model Context Protocol) is an AI agent tool-calling protocol released and open-sourced by
Anthropic in 2024-2025, and has become a de facto standard (adopted by OpenAI, Google, Microsoft,
Zed, Continue, Cody, etc.). Features:

- Based on JSON-RPC 2.0 (same origin as LSP)
- Three primitives: **Tools** (actions), Resources (data), Prompts (templates)
- Transport: `stdio` (subprocess) / streamable `HTTP` / SSE
- Tool input/output has **JSON Schema** strong typing (LLM-friendly)
- 2025-06+ streamable HTTP specification released, this RFC is compatible with both old SSE

**This RFC uses only Tools primitive**——aligned with LSP's "provide service" concept, without
introducing the file model complexity of Resources.

## Proposal

### Core Design

Single binary with dual mode:

```text
┌─────────────────────────────────────────────────────────┐
│                    yaoxiang (v0.7.7+)                   │
│  ┌─────────────────┐      ┌──────────────────────────┐  │
│  │ yaoxiang lsp    │      │   yaoxiang mcp           │  │
│  │ (stdio JSON-RPC)│      │   (stdio default         │  │
│  │ RFC-017 impl    │      │    + HTTP optional)      │  │
│  └────────┬────────┘      └──────────┬───────────────┘  │
│           │                         │                   │
│           ▼                         ▼                   │
│  ┌──────────────────────────────────────────────────┐  │
│  │  Shared lib crate (`yaoxiang`)                    │  │
│  │  src/lsp/{server,session,world}.rs                │  │
│  │  src/frontend/{lexer,parser,core}/...             │  │
│  │  src/middle/...                                   │  │
│  └──────────────────────────────────────────────────┘  │
│                                                          │
│  ┌──────────────────────────────────────────────────┐  │
│  │            src/mcp/  ← New                       │  │
│  │  ├── mod.rs          (module entry + startup)    │  │
│  │  ├── transport/      (stdio + HTTP/SSE)          │  │
│  │  ├── server.rs       (JSON-RPC message loop)      │  │
│  │  ├── tools/          (6 tool handlers)           │  │
│  │  ├── schema.rs       (input/output JSON Schema)   │  │
│  │  └── project.rs      (project root detection +   │  │
│  │                        path resolution)          │  │
│  └──────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

**Key Decisions**:

- **Same binary**: `yaoxiang` switches via subcommand; LSP process and MCP process **do not
  coexist** in same runtime
- **Multi-process independent World**: Each `yaoxiang mcp` process holds one `World`; independent
  from LSP process and other MCP processes (no lock contention, independent crash isolation)
- **stdio default**: Avoid port conflicts, zero network config; HTTP as optional fallback
- **Reuse not duplicate**: Directly call `yaoxiang::frontend` / `yaoxiang::middle` /
  `yaoxiang::lsp::handlers` lib APIs, **do not** go through LSP-client relay

### Tool Set (8 tools, delivered in 3 phases)

Designed with "eliminate edge cases + phased delivery" principle: pure source tools stateless first,
workspace tools share LSP World, AST rewrite tools added independently.

| Tool Name            | Input                                                                                           | Output                                                       | Reuse                                                                 | Phase       |
| -------------------- | ----------------------------------------------------------------------------------------------- | ------------------------------------------------------------ | --------------------------------------------------------------------- | ----------- |
| `parse_source`       | `source: String`, `tab_size?: u32`                                                              | `{ast: Node, diagnostics: Diagnostic[]}`                     | Direct call to `frontend::parse`                                      | v0.8.x      |
| `format_source`      | `source: String`, `tab_size?: u32`                                                              | `{formatted: String, diff: Hunk[]}`                          | Direct call to `formatter::format`                                    | v0.8.x      |
| `lookup_symbol`      | `query: String`, `workspace_root?: String`, `kind?: SymbolKind[]`                               | `{symbols: Symbol[]}`                                        | Reuse `lsp::handlers::workspace_symbol` (fuzzy match by `query`)      | v0.8.x      |
| `find_references`    | `query: String`, `workspace_root?: String`                                                      | `{locations: Location[]}`                                    | Reuse `lsp::handlers::references` (by `query` not position)           | v0.8.x      |
| `typecheck`          | `file_paths: String[]`, `project_root: String`                                                  | `{diagnostics: Diagnostic[], summary: Counts}`               | Reuse `lsp::world::typecheck_full`                                    | v0.8.x      |
| `explain_diagnostic` | `code: String` (e.g. `E0001`), `lang?: String`                                                  | `{code, category, title, description, example, help}`        | **Direct call** to `util::diagnostic::command::render_explain_output` | **v0.9.x**  |
| `list_imports`       | `file_path: String`, `project_root?: String`                                                    | `{imports: [{module, items, source_file}]}`                  | Reuse `middle::passes::module::ModuleGraph::validate_imports`         | **v0.9.x**  |
| `rename_symbol`      | `source: String`, `old_name: String`, `new_name: String`, `scope?: "module" \| "function:name"` | `{source: String, edits: Edit[], diagnostics: Diagnostic[]}` | **New** `src/middle/rename.rs` (AST rewrite)                          | **v0.10.x** |

**Boundaries of the 8 tools**:

- `parse_source` / `format_source` —— **Pure source stateless**, no World involvement
- `lookup_symbol` / `find_references` —— Accept `workspace_root` (if not passed, use
  `--project-root` from startup)
- `typecheck` —— **Required** `file_paths`, ensures workspace completeness
- `explain_diagnostic` —— **Zero file dependency**, pure string query to error code registry
- `list_imports` —— `file_path` is physical file, output import parsing result for that file
- `rename_symbol` —— **Pure source AST rewrite**, no LSP-style position query (semantics differ from
  existing `lsp::handlers::rename`)
- ~~`hover` / `completion` / `signature_help`~~ —— **All cut**: AI agent doesn't do
  "position-sensitive" semantics, use `lookup_symbol` by name instead

**World Loading Timing**: At server startup, scan `yaoxiang.toml` and `src/**/*.yx` by
`--project-root`, reuse LSP-017's already implemented `World::load_*` API to load `World.documents`
at once. **No new** lib API added.

### Tool Contract

**Input**: Described with JSON Schema, each field has `description` + `examples` (LLM automatically
understands).

**Output**: Structured JSON, uniformly with `schemaVersion: "1.0"` field:

```jsonc
// Success response
{
  "schemaVersion": "1.0",
  "isError": false,
  "content": [
    { "type": "json", "json": { /* tool-specific data */ } }
  ]
}

// Diagnostics structured return (not considered tool error)
{
  "schemaVersion": "1.0",
  "isError": false,
  "content": [{ "type": "json", "json": {
    "ast": {...},
    "diagnostics": [
      { "code": "E0001", "severity": "error", "message": "...", "span": [12, 4, 12, 18] }
    ]
  }}]
}

// Tool-level error (e.g. parse_source receives invalid UTF-8)
{
  "schemaVersion": "1.0",
  "isError": true,
  "content": [{ "type": "text", "text": "MCP-INVALID-INPUT: source is not valid UTF-8" }],
  "errorCode": "MCP-INVALID-INPUT"
}
```

**Error System**:

- **Diagnostics**: Parse/type errors, follow RFC-013 (`E0001` etc.) —— **Not considered tool
  errors**
- **Tool-level errors**: Use `MCP-` prefix (`MCP-INVALID-INPUT`, `MCP-PROJECT-NOT-FOUND`,
  `MCP-INTERNAL`) —— Treated as `isError: true`
- **panic/crash**: JSON-RPC `-32603 Internal error`, server does not exit

**Path Resolution Rules** (applicable to `workspace_root` for `lookup_symbol` / `find_references`,
`file_paths` for `typecheck`):

1. Command line `--project-root <dir>` highest priority (overrides default)
2. Otherwise: Find `yaoxiang.toml` upward from cwd until filesystem root (follow RFC-015)
3. Otherwise: cwd itself
4. `file_paths` must be within project root (prevent traversal); out of bounds →
   `MCP-PATH-OUTSIDE-PROJECT`

### Transport Layer

**stdio (default)**:

```bash
yaoxiang mcp
# After startup, read JSON-RPC from stdin, write to stdout, stderr for logs
```

AI agent configuration (Claude Code `.mcp.json` / Continue `config.json`):

```jsonc
{
  "mcpServers": {
    "yaoxiang": {
      "command": "yaoxiang",
      "args": ["mcp", "--project-root", "${workspaceFolder}"],
    },
  },
}
```

**streamable HTTP (optional)**:

```bash
yaoxiang mcp --http --addr 127.0.0.1:7325  # Single HTTP port, new MCP spec
yaoxiang mcp --http --sse --addr 127.0.0.1:7325  # Compatible with old SSE (v0.10)
```

**Security Constraints**:

- **Listen on loopback only** (127.0.0.1 / ::1); public network binding explicitly refused with
  error and exit
- HTTP **no authentication** (loopback trusted by default); future add `--require-token <hex>` field
- stdio subprocess mode naturally isolated (parent process controls permissions)

### Multi-process and Concurrency

Each `yaoxiang mcp` process holds one `World`, not shared:

```text
┌─────────────┐   ┌─────────────┐   ┌─────────────┐
│ yaoxiang    │   │ yaoxiang    │   │ yaoxiang    │
│   lsp       │   │   mcp       │   │   mcp       │
│ (Editor 1)  │   │ (Claude 1)  │   │ (Claude 2)  │
└──────┬──────┘   └──────┬──────┘   └──────┬──────┘
       │ stdio/stdout    │ stdio          │ stdio
   ┌───┴────┐        ┌───┴────┐        ┌───┴────┐
   │ Editor │        │ Claude │        │ Claude │
   └────────┘        └────────┘        └────────┘
```

**Port Conflicts**: AI agent config "start subprocess"——naturally zero port conflicts. HTTP mode
requires users to manage port allocation. **World Isolation**: Each process has independent LSP sync
state——one MCP process crash **does not affect** LSP/other MCP processes. **Future Sessions**: v2
considers multi-workspace dispatch (multiple `Session` in same process), **this RFC does not**.

## Detailed Design

### Data Structures

New `src/mcp/project.rs`:

```rust
pub struct ProjectRoot {
    /// Absolute path
    pub root: PathBuf,
    /// Strategy source for project root detection at load time
    pub source: ProjectRootSource,
}

pub enum ProjectRootSource {
    CliFlag,           // yaoxiang mcp --project-root
    AutoDetected,      // Search upward for yaoxiang.toml
    FallbackCwd,       // Fallback to cwd
}

pub struct ResolvedPath {
    /// Path relative to project root (recommended for AI to read)
    pub relative: String,
    /// Resolved absolute path (used for World operations)
    pub absolute: PathBuf,
}

impl ProjectRoot {
    /// Resolve "file_path" to safe path——prevent traversal
    pub fn resolve(&self, file_path: &str) -> Result<ResolvedPath, McpError>;
}
```

`ProjectRoot` singleton + automatic tool schema generation in `src/mcp/schema.rs`:

```rust
pub struct ProjectRoot {
    /// Absolute path (must contain `yaoxiang.toml` or fallback)
    pub root: PathBuf,
    pub source: ProjectRootSource,
}

impl ProjectRoot {
    /// Detect once at CLI startup, result cached in `McpServer` context——all tools reuse
    pub fn detect(cli_override: Option<PathBuf>) -> Result<Self, McpError>;
}
```

Tool schema auto-generated from input structs using `schemars` crate, avoiding manual JSON Schema
drift:

```rust
#[derive(Deserialize, schemars::JsonSchema)]
pub struct ParseSourceInput {
    /// Complete YaoXiang source snippet——**not** saved to disk, pure transient
    pub source: String,
    pub tab_size: Option<u32>,
}
```

**`parse_source` / `format_source` tool schema has no `file_path` field** ——these two tools only
accept string source, do not participate in project semantics. `lookup_symbol` / `find_references` /
`typecheck` accept `workspace_root` or `file_paths` (required or not per tool table).

### Compiler Changes

| Module                                 | Change                                                                                         |
| -------------------------------------- | ---------------------------------------------------------------------------------------------- |
| `src/lsp/world.rs`                     | **Zero change**——MCP startup calls already existing `World::load_*` API to load workspace once |
| `src/lsp/handlers/workspace_symbol.rs` | **Zero change**——`mcp/tools/lookup.rs` wraps to convert `query` to LSP params                  |
| `src/lsp/handlers/references.rs`       | **Zero change**——same as above                                                                 |
| `src/lsp/handlers/formatter.rs`        | **Zero change**——format_source calls directly                                                  |
| `src/main.rs`                          | Add `Mcp` subcommand branch                                                                    |
| `Cargo.toml`                           | Add `mcp-server` feature (or main binary always includes it)                                   |
| `src/util/diagnostic/`                 | **Zero change** (RFC-017 already implemented)                                                  |

**Key Constraint**: `src/mcp/` **must not** reverse-depend on `src/lsp/` private symbols——can only
call handlers via `crate::lsp::` public API.

### Backward Compatibility

- ✅ **Fully backward compatible**: New subcommand `yaoxiang mcp`, does not change any existing
  behavior of `yaoxiang` / `yaoxiang lsp`
- ✅ **LSP server unchanged**: All capabilities, APIs, internal state implemented by RFC-017
  unchanged
- ✅ **lib crate public API unchanged**: All `pub` paths unchanged; MCP only consumes existing
  APIs—— **zero** new `pub` methods

### Integration with Existing Systems

| Existing Module                           | MCP Integration Method                                                                                  |
| ----------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| `src/frontend/lexer`                      | parse_source directly calls lexer                                                                       |
| `src/frontend/core/parser`                | parse_source directly calls parser; failure produces `Missing*` nodes (RFC-017)                         |
| `src/frontend/core/typecheck/inference/*` | typecheck reuses `collect_diagnostics` pattern (RFC-017 §Problem 1)                                     |
| `src/middle/`                             | typecheck runs all middle passes (dependency analysis, etc.)                                            |
| `src/lsp/world.rs`                        | Call `World::load_*` API at startup (already exists); World **does not** accept any "virtual documents" |
| `src/lsp/handlers/workspace_symbol.rs`    | `mcp/tools/lookup.rs` wraps, converting `query: String` to LSP params (by-name query)                   |
| `src/lsp/handlers/references.rs`          | `mcp/tools/find_refs.rs` wraps, converting `query: String` to LSP params                                |
| `src/lsp/handlers/formatter.rs`           | `mcp/tools/format.rs` calls directly (if not implemented, add `formatter::format_with_diff`)            |
| `src/util/i18n/`                          | Error messages go through multilingual resource files (zh-CN/en)                                        |

### Error Handling

| Source                                   | Handling                                                                                      |
| ---------------------------------------- | --------------------------------------------------------------------------------------------- |
| Parse error                              | `Diagnostic{code:"E0xxx", severity, message, span}` (**not tool error**, returned in content) |
| Type error                               | Same as above                                                                                 |
| `file_paths` out of bounds (`typecheck`) | Tool-level error `MCP-PATH-OUTSIDE-PROJECT`                                                   |
| `source` invalid UTF-8                   | Tool-level error `MCP-INVALID-INPUT`                                                          |
| Tool panic                               | JSON-RPC `-32603 Internal error`; server **does not exit**                                    |
| Client sends non-JSON-RPC                | Stream ends directly (stdio EOF), restart = new session                                       |

Diagnostic severity levels follow RFC-017 (already implemented)
`enum ErrorKind { Error, Warning, Note }`.

### Testing Strategy

| Layer           | Testing                                                                                              |
| --------------- | ---------------------------------------------------------------------------------------------------- |
| **Unit**        | `src/mcp/project.rs::resolve` path traversal, `src/mcp/schema.rs` schema validation                  |
| **Integration** | Mock stdio: start a server, pump JSON-RPC to stdin, read response from stdout, compare fixtures      |
| **E2E**         | Run real `yaoxiang mcp` process, Claude Code style tool call chain: parse → fix → format → typecheck |
| **Fuzz**        | `cargo-fuzz` for MCP JSON-RPC parsing (libFuzzer harness)                                            |

Each tool must have at least 1 happy path + 1 diagnostic scenario + 1 tool-error scenario
integration test.

## Tradeoffs

### Pros

- **Extremely low reuse cost**: `World` / `Session` / `handlers` / diagnostic collection all already
  implemented (RFC-017), this RFC is "add an MCP shell"
- **AI-First interface**: Tool contract is 3-5x more intuitive than LSP; LLM directly reads schema
- **Multi-process isolation**: Decoupled from LSP editor session and other MCP processes, **zero
  lock contention**
- **stdio friendly**: All mainstream AI agents default subprocess mode, zero-config integration
- **YAGNI passed**: This RFC cuts Resources, Sessions, cross-process state, remote MCP——v2 reopens
  these

### Cons

- **Protocol fragmentation**: Future LSP / MCP / DAP three protocols evolve independently,
  consistency maintenance cost
- **HTTP mode is second-class citizen**: Loopback restriction positions as local tool, remote
  scenarios need v2 redesign
- **Repeated parse overhead**: AI repeatedly fine-tuning source and calling `parse_source` re-runs
  lexer+parser. **Mitigation**: RFC-017's `DocumentCache` still accelerates second parse of **disk**
  same source; pure transient source unavoidable
- **Testing coverage cost**: 5 tools × 3 scenarios = 15 integration tests minimum

## Alternative Approaches

| Approach                                                   | Why Not Chosen                                                                                                 |
| ---------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| **In-process dual protocol** (LSP+MCP listener coexisting) | stdin/stdout can only have one consumer; HTTP also needs coexistence——complexity > benefit                     |
| **MCP as LSP-client bridge**                               | Extra layer of IPC; LSP design doesn't support symbol lookup by name——capabilities MCP wants LSP can't provide |
| **Use gRPC / custom protocol**                             | Deviate from de facto standard; community already has MCP SDK (TypeScript, Python, Rust), with ecosystem       |
| **Reuse all LSP handler capabilities** (L3 tool set)       | Lots of position↔intent adapter work; diminishing marginal returns                                             |
| **First version HTTP only** (no stdio)                     | Claude Code / Continue default stdio, too high barrier                                                         |

## Implementation Strategy

### Dependencies

- **Strong dependency**: RFC-017 LSP implementation (already implemented)
- **Strong dependency**: RFC-013 error code system (already implemented)
- **Strong dependency**: RFC-014 / RFC-015 project root detection (partially implemented)
- **New dependencies** (Rust crate):
  - `mcp-rust-sdk` (pending evaluation, reference
    [modelcontextprotocol/rust-sdk](https://github.com/modelcontextprotocol/rust-sdk))
  - `tokio` (**already have**, optional feature)
  - `axum` (HTTP mode) or `hyper` directly——pending evaluation
- **Zero language spec changes**: Pure toolchain increment

### Implementation Phases

| Phase                              | Content                                                                                                                                                                                                                                | Time Estimate |
| ---------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------- |
| **v0.8.x (MVP)**                   | `src/mcp/{mod.rs, server.rs, transport/stdio.rs, project.rs, schema.rs}` + `parse_source` + `format_source` + `lookup_symbol` + `find_references` + `typecheck` (**5 tools**) + `yaoxiang mcp` subcommand + `World::load_*` at startup | **3-4 weeks** |
| **v0.9.x (YaoXiang Intelligence)** | `+ explain_diagnostic` (**direct call** to `render_explain_output`) + `+ list_imports` (wrap `ModuleGraph::validate_imports`) + unit/integration tests                                                                                 | **1-2 weeks** |
| **v0.10.x (AST + HTTP)**           | `+ rename_symbol` (**new** `src/middle/rename.rs`, AST rewrite) + streamable HTTP transport + performance tuning (parse_source P99 < 100ms)                                                                                            | **2-3 weeks** |

**Why 3 phases**: MVP first validates stdio + 5 tools to verify interface design; v0.9.x adds
low-risk zero-adaptation "YaoXiang-specific" tools to verify integration correctness; v0.10.x opens
high-risk "AST rewrite" new module (independent PR review more focused).

### Risks

1. **`mcp-rust-sdk` maintenance activity**: Released in 2025, API may change drastically.
   **Mitigation**: If unstable, write lightweight JSON-RPC 2.0 + tool dispatcher ourselves (< 500
   lines)
2. **Repeated parse overhead**: AI repeatedly fine-tuning source and calling `parse_source` re-runs
   lexer+parser. **Mitigation**: RFC-017's `DocumentCache` still accelerates second parse of
   **disk** same source; pure transient source unavoidable
3. **AI agent schema compatibility**: Different agents have different MCP schema strictness.
   **Mitigation**: Use `schemars` crate to auto-generate schema from Rust input structures, zero
   manual drift
4. **Cross-platform path resolution**: Windows paths case-insensitive, UNC paths, `\\` boundaries.
   **Mitigation**: Use `camino::Utf8Path` for path resolution instead of `std::path`
5. **MCP tool schema not 1:1 with LSP params**: LSP `workspace_symbol` takes `(query)`; when passing
   to LSP internals, need to wrap into position+URI for existing handler reuse. **Mitigation**: Do
   adaptation layer in `mcp/tools/lookup.rs`, encapsulation details on MCP side
6. **`rename_symbol` AST rewrite semantics differ from LSP `rename`**: LSP `textDocument/rename` is
   URI + position + new_name → WorkspaceEdit; MCP `rename_symbol` is source + old_name + new_name →
   new source. **Cannot reuse directly**. **Mitigation**: Implement `src/middle/rename.rs`
   separately, scope-aware rewrite references, does not interfere with LSP handler implementation

## Open Questions

- [ ] `mcp-rust-sdk` selection / self-implementation? (@Chen Xu: evaluate rust-sdk June version
      first, then decide)
- [ ] HTTP authentication path? (RFC for v0.10)
- [ ] Does MCP need to output `tools/list` at startup for AI proactive discovery? (MCP standard
      requires, **implement by default**)
- [ ] Does `typecheck` support `mode: "fast|full"` (fast = current file subset only, full = full
      workspace)?
- [ ] Is parse_source P99 < 100ms performance budget realistic? (Need benchmark RFC-017's
      implemented `DocumentCache` actual overhead in source-string mode)

## References

- [RFC-017: Language Server Protocol (LSP) Support Design](../accepted/017-lsp-support.md)
- [RFC-013: Error Code Specification Design](../accepted/013-error-code-specification.md)
- [RFC-014: Package Manager System Design](../accepted/014-package-manager.md)
- [RFC-015: YaoXiang Configuration System Design](../accepted/015-configuration-system.md)
- [MCP Specification](https://modelcontextprotocol.io/)
- [MCP Rust SDK](https://github.com/modelcontextprotocol/rust-sdk)
- [JSON-RPC 2.0](https://www.jsonrpc.org/specification)
- [LSP Specification 3.18](https://github.com/microsoft/language-server-protocol/blob/main/specifications/specification-3-18.md)
- [Rust Analyzer](https://rust-analyzer.github.io/) —— M2 / MCP integration reference
- [zed-industries/zed MCP implementation](https://github.com/zed-industries/zed/tree/main/crates/mcp)

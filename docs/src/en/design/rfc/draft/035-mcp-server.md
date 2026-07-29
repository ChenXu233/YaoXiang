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
Cody, Zed, etc.) to directly query YaoXiang source code's **AST, parse errors, types, symbols,
references, and formatting results**. Reusing the already-implemented `World` backend from RFC-017,
add a `yaoxiang mcp` subcommand, single binary with dual mode, multi-process independent World.

## Motivation

### Why is this feature needed?

RFC-017 allows YaoXiang to be **understood** by editors (hover / goto-def / completion). But LSP is
a **position-driven** protocol:

- Every request heavily depends on `textDocument` URI + `Position`
- Editors must first open files, save them, and maintain long connections with the LSP server
- AI agent workflows work with **code snippets**: "paste a piece of code" in conversation to ask
  questions, **without** saving to disk first

LSP clients actually available to AI agents (vscode-langservers-extracted, `mcp-lsp-bridge`-type
projects) only **translate L1**: goto-def, hover. What AI wants to do:

- "Is this code **parsed correctly**" — needs parse + complete diagnostic stream
- "How is this symbol **used in the file**" — needs lookup_symbol by name
- "What does this code **look like after formatting**" — needs format_source
- "Where are **all** the type errors" — needs typecheck to run the entire workspace

These L1 LSP translation capabilities **cannot do it**, because LSP is by design not designed to
support them.

### Current Problems

1. AI agents calling LSP have poor experience: need to mock documents, huge JSON, strong URI
   dependency
2. YaoXiang project lacks an "AI-First" interface layer: humans use IDE with LSP, AI agents can't
   use LSP
3. Mainstream AI agents like Claude Code / Continue already support MCP by default, leaving a blank
   ecosystem for YaoXiang

### What is MCP?

MCP (Model Context Protocol) is an AI agent tool-calling protocol led by Anthropic and open-sourced
in 2024-2025, becoming a de facto standard (adopted by OpenAI, Google, Microsoft, Zed, Continue,
Cody, etc.). Features:

- Based on JSON-RPC 2.0 (same origin as LSP)
- Three primitives: **Tools** (actions), Resources (data), Prompts (templates)
- Transport: `stdio` (subprocess) / streamable `HTTP` / SSE
- Tool inputs/outputs have **JSON Schema** strong typing (LLM-friendly)
- 2025-06+ has released the streamable HTTP specification; this RFC is also compatible with legacy
  SSE

**This RFC only uses the Tools primitive** — aligned with LSP's "provide services" approach, without
introducing the file model complexity of Resources.

## Proposal

### Core Design

Single binary, dual mode:

```text
┌─────────────────────────────────────────────────────────┐
│                    yaoxiang（v0.7.7+）                  │
│  ┌─────────────────┐      ┌──────────────────────────┐  │
│  │ yaoxiang lsp    │      │   yaoxiang mcp           │  │
│  │ (stdio JSON-RPC)│      │   (stdio default         │  │
│  │ RFC-017 已实现  │      │    + HTTP 可选)          │  │
│  └────────┬────────┘      └──────────┬───────────────┘  │
│           │                         │                   │
│           ▼                         ▼                   │
│  ┌──────────────────────────────────────────────────┐  │
│  │  共享 lib crate（`yaoxiang`）                      │  │
│  │  src/lsp/{server,session,world}.rs                │  │
│  │  src/frontend/{lexer,parser,core}/...             │  │
│  │  src/middle/...                                   │  │
│  └──────────────────────────────────────────────────┘  │
│                                                          │
│  ┌──────────────────────────────────────────────────┐  │
│  │            src/mcp/  ← 新增                       │  │
│  │  ├── mod.rs          （模块入口 + 启动函数）       │  │
│  │  ├── transport/      （stdio + HTTP/SSE）         │  │
│  │  ├── server.rs       （JSON-RPC 消息循环）         │  │
│  │  ├── tools/          （6 个 tool handler）        │  │
│  │  ├── schema.rs       （输入输出 JSON Schema）     │  │
│  │  └── project.rs      （项目根识别 + 路径解析）    │  │
│  └──────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

**Key Decisions**:

- **Same binary**: `yaoxiang` switches via subcommand; LSP processes and MCP processes **do not
  coexist** in the same runtime
- **Multi-process independent World**: each `yaoxiang mcp` process holds a `World`; does not affect
  LSP processes or other MCP processes (no lock contention, independent crash isolation)
- **stdio by default**: avoids port conflicts, zero network configuration; HTTP as an optional
  fallback
- **Reuse rather than duplicate**: directly call the lib API of `yaoxiang::frontend` /
  `yaoxiang::middle` / `yaoxiang::lsp::handlers`, **not** going through LSP-client relay

### Tool Set (8 tools, delivered in 3 phases)

Designed with the "eliminate special cases + phased" principle: pure source tools (stateless) first,
workspace tools share LSP World, AST rewriting tools added independently.

| Tool Name            | Input                                                                                           | Output                                                       | Reuse                                                                | Phase       |
| -------------------- | ----------------------------------------------------------------------------------------------- | ------------------------------------------------------------ | -------------------------------------------------------------------- | ----------- |
| `parse_source`       | `source: String`, `tab_size?: u32`                                                              | `{ast: Node, diagnostics: Diagnostic[]}`                     | Directly call `frontend::parse`                                      | v0.8.x      |
| `format_source`      | `source: String`, `tab_size?: u32`                                                              | `{formatted: String, diff: Hunk[]}`                          | Directly call `formatter::format`                                    | v0.8.x      |
| `lookup_symbol`      | `query: String`, `workspace_root?: String`, `kind?: SymbolKind[]`                               | `{symbols: Symbol[]}`                                        | Reuse `lsp::handlers::workspace_symbol` (fuzzy match by `query`)     | v0.8.x      |
| `find_references`    | `query: String`, `workspace_root?: String`                                                      | `{locations: Location[]}`                                    | Reuse `lsp::handlers::references` (by `query` instead of position)   | v0.8.x      |
| `typecheck`          | `file_paths: String[]`, `project_root: String`                                                  | `{diagnostics: Diagnostic[], summary: Counts}`               | Reuse `lsp::world::typecheck_full`                                   | v0.8.x      |
| `explain_diagnostic` | `code: String` (e.g. `E0001`), `lang?: String`                                                  | `{code, category, title, description, example, help}`        | **Directly call** `util::diagnostic::command::render_explain_output` | **v0.9.x**  |
| `list_imports`       | `file_path: String`, `project_root?: String`                                                    | `{imports: [{module, items, source_file}]}`                  | Reuse `middle::passes::module::ModuleGraph::validate_imports`        | **v0.9.x**  |
| `rename_symbol`      | `source: String`, `old_name: String`, `new_name: String`, `scope?: "module" \| "function:name"` | `{source: String, edits: Edit[], diagnostics: Diagnostic[]}` | **Newly added** `src/middle/rename.rs` (AST rewriting)               | **v0.10.x** |

**Boundaries of the 8 tools**:

- `parse_source` / `format_source` — **pure source stateless**, do not enter World
- `lookup_symbol` / `find_references` — accept `workspace_root` (if not passed, use the
  `--project-root` at startup)
- `typecheck` — **required** `file_paths`, ensuring workspace completeness
- `explain_diagnostic` — **zero file dependency**, pure string query against error code registry
- `list_imports` — `file_path` is a physical file, outputs the import resolution result of that file
- `rename_symbol` — **pure source AST rewriting**, no LSP-style position query (semantics differ
  from existing `lsp::handlers::rename`)
- ~~`hover` / `completion` / `signature_help`~~ — **all cut**: AI agents do not do
  "position-sensitive" semantics, replaced by `lookup_symbol` by name

**World load timing**: at server startup, scan `yaoxiang.toml` and `src/**/*.yx` according to
`--project-root`, reuse the already-implemented `World::load_*` API from LSP-017 to load into
`World.documents` in one shot. **No** new lib APIs added.

### Tool Contract

**Input**: described with JSON Schema, each field has `description` + `examples` (LLM can
automatically understand).

**Output**: structured JSON, uniformly carrying a `schemaVersion: "1.0"` field:

```jsonc
// Success response
{
  "schemaVersion": "1.0",
  "isError": false,
  "content": [
    { "type": "json", "json": { /* tool-specific data */ } }
  ]
}

// Diagnostics returned structurally (not treated as tool error)
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

- **Diagnostics**: parse/type errors, follow RFC-013 (`E0001` etc.) — **not counted as tool errors**
- **Tool-level errors**: use `MCP-` prefix (`MCP-INVALID-INPUT`, `MCP-PROJECT-NOT-FOUND`,
  `MCP-INTERNAL`) — treated as `isError: true`
- **panic/crash**: JSON-RPC `-32603 Internal error`, server does not exit

**Path Resolution Rules** (applies to `workspace_root` of `lookup_symbol` / `find_references`, and
`file_paths` of `typecheck`):

1. Command-line `--project-root <dir>` has highest priority (overrides default)
2. Otherwise: from cwd, walk upward looking for `yaoxiang.toml` until filesystem root (follows
   RFC-015)
3. Otherwise: cwd itself
4. `file_paths` must fall within the project root (prevent traversal); out of bounds →
   `MCP-PATH-OUTSIDE-PROJECT`

### Transport Layer

**stdio (default)**:

```bash
yaoxiang mcp
# After startup, read JSON-RPC from stdin, write to stdout, stderr for logging
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
yaoxiang mcp --http --addr 127.0.0.1:7325  # single HTTP port, new MCP specification
yaoxiang mcp --http --sse --addr 127.0.0.1:7325  # compatible with legacy SSE (v0.10)
```

**Security Constraints**:

- **Only listen on loopback** (127.0.0.1 / ::1); public binding is explicitly rejected with an error
  and exit
- HTTP **no authentication** (loopback trusted by default); future add `--require-token <hex>` field
- stdio subprocess mode is naturally isolated (parent process controls permissions)

### Multi-process and Concurrency

Each `yaoxiang mcp` process holds a `World`, mutually not shared:

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

**Port conflict**: AI agent configures "spawn subprocess" — naturally zero port conflict. HTTP mode
requires user to manage port allocation themselves. **World isolation**: each process has
independent LSP sync state — one MCP process crashing **does not affect** LSP / other MCP processes.
**future Sessions**: multi-workspace dispatch (multiple `Session` within same process) is only
considered in v2, **not done in this RFC**.

## Detailed Design

### Data Structures

Add `src/mcp/project.rs`:

```rust
pub struct ProjectRoot {
    /// Absolute path
    pub root: PathBuf,
    /// Strategy source for identifying project root at load time
    pub source: ProjectRootSource,
}

pub enum ProjectRootSource {
    CliFlag,           // yaoxiang mcp --project-root
    AutoDetected,      // walk upward to find yaoxiang.toml
    FallbackCwd,       // fallback to cwd
}

pub struct ResolvedPath {
    /// Relative path relative to project root (recommended for AI to read)
    pub relative: String,
    /// Resolved absolute path (used for World operations)
    pub absolute: PathBuf,
}

impl ProjectRoot {
    /// Resolve "file_path" as a safe path — prevent traversal
    pub fn resolve(&self, file_path: &str) -> Result<ResolvedPath, McpError>;
}
```

`ProjectRoot` singleton + `src/mcp/schema.rs` tool schema auto-generation:

```rust
pub struct ProjectRoot {
    /// Absolute path (must contain `yaoxiang.toml` or backward-compatible fallback)
    pub root: PathBuf,
    pub source: ProjectRootSource,
}

impl ProjectRoot {
    /// Identified once at CLI startup, result cached in `McpServer` context — all tools reuse
    pub fn detect(cli_override: Option<PathBuf>) -> Result<Self, McpError>;
}
```

Tool schemas use the `schemars` crate to auto-generate from input struct, avoiding hand-written JSON
Schema drift:

```rust
#[derive(Deserialize, schemars::JsonSchema)]
pub struct ParseSourceInput {
    /// Complete YaoXiang source code snippet — **not** saved to disk, purely transient
    pub source: String,
    pub tab_size: Option<u32>,
}
```

**`parse_source` / `format_source` tool schemas do not have a `file_path` field** — these two tools
only accept string sources, not participating in project semantics. `lookup_symbol` /
`find_references` / `typecheck` accept `workspace_root` or `file_paths` (required or not, see tool
table).

### Compiler Changes

| Module                                 | Change                                                                                          |
| -------------------------------------- | ----------------------------------------------------------------------------------------------- |
| `src/lsp/world.rs`                     | **Zero changes** — MCP startup calls existing `World::load_*` API to load workspace in one shot |
| `src/lsp/handlers/workspace_symbol.rs` | **Zero changes** — `mcp/tools/lookup.rs` wraps one layer converting `query` to LSP input        |
| `src/lsp/handlers/references.rs`       | **Zero changes** — same as above                                                                |
| `src/lsp/handlers/formatter.rs`        | **Zero changes** — format_source directly calls                                                 |
| `src/main.rs`                          | Add `Mcp` subcommand branch                                                                     |
| `Cargo.toml`                           | Add `mcp-server` feature (or always included in main binary)                                    |
| `src/util/diagnostic/`                 | **Zero changes** (RFC-017 already implemented)                                                  |

**Key constraint**: `src/mcp/` **must not** reverse-depend on private symbols of `src/lsp/` — only
call handlers through the public API of `crate::lsp::`.

### Backward Compatibility

- ✅ **Fully backward compatible**: new subcommand `yaoxiang mcp`, does not change any existing
  behavior of `yaoxiang` / `yaoxiang lsp`
- ✅ **LSP server unchanged**: all capabilities, APIs, and internal state implemented in RFC-017
  remain unchanged
- ✅ **lib crate public API unchanged**: all `pub` paths unchanged; MCP only consumes existing APIs
  — **zero** new `pub` methods

### Integration with Existing Systems

| Existing Module                           | MCP Integration Method                                                                             |
| ----------------------------------------- | -------------------------------------------------------------------------------------------------- |
| `src/frontend/lexer`                      | parse_source directly calls lexer                                                                  |
| `src/frontend/core/parser`                | parse_source directly calls parser; failures produce `Missing*` nodes (RFC-017)                    |
| `src/frontend/core/typecheck/inference/*` | typecheck reuses `collect_diagnostics` pattern (RFC-017 §Problem 1)                                |
| `src/middle/`                             | typecheck runs all middle passes (dependency analysis etc.)                                        |
| `src/lsp/world.rs`                        | At startup, call `World::load_*` API (existing); World **does not** accept any "virtual documents" |
| `src/lsp/handlers/workspace_symbol.rs`    | `mcp/tools/lookup.rs` wraps one layer, converting `query: String` to LSP input (lookup by name)    |
| `src/lsp/handlers/references.rs`          | `mcp/tools/find_refs.rs` wraps one layer, converting `query: String` to LSP input                  |
| `src/lsp/handlers/formatter.rs`           | `mcp/tools/format.rs` directly calls (if not implemented, add `formatter::format_with_diff`)       |
| `src/util/i18n/`                          | Error messages go through multilingual resource files (zh-CN/en)                                   |

### Error Handling

| Source                                        | Handling                                                                                        |
| --------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| Parse error                                   | `Diagnostic{code:"E0xxx", severity, message, span}` (**not a tool error**, returned in content) |
| Type error                                    | Same as above                                                                                   |
| `file_paths` out of bounds (`typecheck` tool) | Tool-level error `MCP-PATH-OUTSIDE-PROJECT`                                                     |
| `source` invalid UTF-8                        | Tool-level error `MCP-INVALID-INPUT`                                                            |
| Tool panic                                    | JSON-RPC `-32603 Internal error`; server **does not exit**                                      |
| Client sends non-JSON-RPC                     | Direct stream termination (stdio EOF), restart means new session                                |

Diagnostic severity levels follow RFC-017 (already implemented)
`enum ErrorKind { Error, Warning, Note }`.

### Testing Strategy

| Layer           | Testing                                                                                                |
| --------------- | ------------------------------------------------------------------------------------------------------ |
| **Unit**        | `src/mcp/project.rs::resolve` path traversal, `src/mcp/schema.rs` schema validation                    |
| **Integration** | mock stdio: start a server, pour JSON-RPC into stdin, read response from stdout, compare to fixture    |
| **E2E**         | Run actual `yaoxiang mcp` process, Claude Code-style tool call chain: parse → fix → format → typecheck |
| **Fuzz**        | `cargo-fuzz` for MCP JSON-RPC parsing (libFuzzer harness)                                              |

Each tool must have at least 1 happy path + 1 diagnostic scenario + 1 tool-error scenario
integration test.

## Trade-offs

### Advantages

- **Very low reuse cost**: `World` / `Session` / `handlers` / diagnostic collection all already
  implemented (RFC-017), this RFC is "adding an MCP shell layer"
- **AI-First interface**: tool contract is 3-5x more intuitive than LSP; LLM directly reads schema
- **Multi-process isolation**: decoupled from LSP editor session and other MCP processes, **zero
  lock contention**
- **stdio-friendly**: all mainstream AI agents default to subprocess mode, zero-configuration
  integration
- **YAGNI passed**: this RFC cuts Resources, Sessions, cross-process state, remote MCP — reopen in
  v2

### Disadvantages

- **Protocol split**: LSP / MCP / DAP three protocols evolve independently in the future,
  consistency maintenance cost
- **HTTP mode second-class citizen**: loopback restriction positioned as local tool, remote
  scenarios need v2 redesign
- **Duplicate parse overhead**: AI repeatedly tweaking source code and repeatedly calling
  `parse_source` will re-run lexer+parser. **Mitigation**: rely on RFC-017's `DocumentCache` to
  still accelerate **disk** secondary parsing of same source; pure transient source parsing once is
  unavoidable
- **Test coverage cost**: 5 tools × 3 scenarios = 15 integration tests minimum

## Alternative Approaches

| Approach                                                           | Why Not Chosen                                                                                                     |
| ------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------ |
| **In-process embedded dual protocol** (LSP+MCPlistener coexisting) | stdin/stdout can only have one consumer; HTTP also has to coexist — complexity > benefit                           |
| **MCP as LSP-client bridge**                                       | One extra IPC layer; LSP by design does not support lookup by name — capabilities MCP wants, LSP cannot provide    |
| **Use gRPC / custom protocol**                                     | Deviates from de facto standard; community already has MCP SDK (TypeScript, Python, Rust), with built-in ecosystem |
| **Reuse all LSP handler capabilities** (L3 toolset)                | Lots of position↔intent adaptation work; diminishing marginal returns                                              |
| **First version only HTTP** (no stdio)                             | Claude Code / Continue etc. default to stdio, threshold too high                                                   |

## Implementation Strategy

### Dependencies

- **Strong dependency**: RFC-017 LSP implementation (already implemented)
- **Strong dependency**: RFC-013 error code system (already implemented)
- **Strong dependency**: RFC-014 / RFC-015 project root identification (partially implemented)
- **New dependencies** (Rust crate):
  - `mcp-rust-sdk` (to be evaluated, refer to
    [modelcontextprotocol/rust-sdk](https://github.com/modelcontextprotocol/rust-sdk))
  - `tokio` (**already existing**, optional feature)
  - `axum` (HTTP mode) or `hyper` directly — to be evaluated
- **Zero language specification changes**: pure toolchain increment

### Phases (synchronized with #154)

| Phase                              | Content                                                                                                                                                                                                                                | Duration Estimate |
| ---------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------- |
| **v0.8.x (MVP)**                   | `src/mcp/{mod.rs, server.rs, transport/stdio.rs, project.rs, schema.rs}` + `parse_source` + `format_source` + `lookup_symbol` + `find_references` + `typecheck` (**5 tools**) + `yaoxiang mcp` subcommand + `World::load_*` at startup | **3-4 weeks**     |
| **v0.9.x (YaoXiang Intelligence)** | `+ explain_diagnostic` (**directly call** `render_explain_output`) + `+ list_imports` (wrap `ModuleGraph::validate_imports`) + unit/integration tests                                                                                  | **1-2 weeks**     |
| **v0.10.x (AST + HTTP)**           | `+ rename_symbol` (**newly added** `src/middle/rename.rs`, AST rewriting) + streamable HTTP transport + performance tuning (parse_source P99 < 100ms)                                                                                  | **2-3 weeks**     |

**Why 3 phases**: MVP first runs through stdio + 5 tools to verify interface design is reasonable;
v0.9.x adds low-risk zero-adaptation "YaoXiang-specific" tools to verify integration correctness;
v0.10.x then opens the high-risk "AST rewriting" new module (independent PR review more focused).

### Risks

1. **`mcp-rust-sdk` maintenance activity**: released in 2025, API may change drastically.
   **Mitigation**: if evaluation shows instability, write own lightweight JSON-RPC 2.0 + tool
   dispatcher (< 500 lines)
2. **Duplicate parse overhead**: AI repeatedly tweaking source code and repeatedly calling
   `parse_source` will re-run lexer+parser. **Mitigation**: rely on RFC-017's `DocumentCache` to
   still accelerate **disk** secondary parsing of same source; pure transient source parsing once is
   unavoidable
3. **AI agent schema compatibility**: different agents' MCP schema strictness differs.
   **Mitigation**: use `schemars` crate to auto-generate schema from Rust input structures, zero
   hand-written drift
4. **Path resolution multi-platform**: Windows path case-insensitivity, UNC paths, `\\` boundaries.
   **Mitigation**: path resolution uses `camino::Utf8Path` instead of `std::path`
5. **MCP tool schema and LSP input not 1:1**: LSP `workspace_symbol` accepts `(query)`; passing to
   LSP internals requires wrapping as position+URI to allow existing handler reuse. **Mitigation**:
   do the adaptation layer in `mcp/tools/lookup.rs`, encapsulating details on the MCP side
6. **`rename_symbol` AST rewriting and LSP `rename` semantics differ**: LSP `textDocument/rename` is
   URI + position + new_name → WorkspaceEdit; MCP `rename_symbol` is source + old_name + new_name →
   new source. **Cannot directly reuse**. **Mitigation**: independently implement
   `src/middle/rename.rs`, scope-aware rewriting of references, not interfering with LSP handler
   implementation

## Open Questions

- [ ] `mcp-rust-sdk` selection / self-implementation? (@Chen Xu: first evaluate rust-sdk June
      version, then decide)
- [ ] HTTP authentication path? (reopen RFC in v0.10)
- [ ] Does `MCP` need to output `tools/list` at startup for AI active discovery? (MCP standard
      requires, **implemented by default**)
- [ ] Does `typecheck` support `mode: "fast|full"` (fast = current file subset only, full = entire
      workspace)?
- [ ] Is performance budget parse_source P99 < 100ms realistic? (need to benchmark RFC-017's
      already-implemented `DocumentCache` actual overhead in source-string mode)

## References

- [RFC-017: Language Server Protocol (LSP) Support Design](./accepted/017-lsp-support.md)
- [RFC-013: Error Code Specification Design](./accepted/013-error-code-specification.md)
- [RFC-014: Package Management System Design](./accepted/014-package-manager.md)
- [RFC-015: YaoXiang Configuration System Design](./accepted/015-configuration-system.md)
- [MCP Specification](https://modelcontextprotocol.io/)
- [MCP Rust SDK](https://github.com/modelcontextprotocol/rust-sdk)
- [JSON-RPC 2.0](https://www.jsonrpc.org/specification)
- [LSP Specification 3.18](https://github.com/microsoft/language-server-protocol/blob/main/specifications/specification-3-18.md)
- [Rust Analyzer](https://rust-analyzer.github.io/) — M2 / MCP integration reference
- [zed-industries/zed MCP implementation](https://github.com/zed-industries/zed/tree/main/crates/mcp)

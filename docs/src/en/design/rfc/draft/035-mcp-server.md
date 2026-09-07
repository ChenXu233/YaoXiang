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

Add a MCP (Model Context Protocol) server to YaoXiang, allowing AI agents (Claude Code, Continue,
Cody, Zed, etc.) to directly query YaoXiang source code's **AST, parse errors, types, symbols,
references, and formatting results**. Reuse the `World` backend already implemented in RFC-017, add
a new `yaoxiang mcp` subcommand, single binary dual mode, multi-process independent World.

## Motivation

### Why is this feature needed?

RFC-017 makes YaoXiang **understandable** by editors (hover / goto-def / completion). But LSP is a
**position-driven** protocol:

- Every request strongly depends on `textDocument` URI + `Position`
- The editor must first open the file, save it, and maintain a long connection with the LSP server
- AI agent workflow is **code snippets**: "paste a piece of code" in a conversation to ask
  questions, **not** saving to disk first

The LSP clients actually available to AI agents (vscode-langservers-extracted, projects like
`mcp-lsp-bridge`) **only translate L1**: goto-def, hover. What AI wants to do:

- "Is this piece of code **parsed correctly**" — needs parse + complete diagnostic stream
- "How is this symbol **used in the file**" — needs lookup_symbol to query by name
- "What does this code **look like after formatting**" — needs format_source
- "Where are **all** the type errors" — needs typecheck to run the complete workspace

These L1 LSP translation capabilities **cannot do this**, because LSP is not designed to support
them by design.

### Current problems

1. AI agents have a poor experience calling LSP: need to mock documents, JSON is huge, strong URI
   dependency
2. YaoXiang project lacks an "AI-First" interface layer: humans open IDE and use LSP, AI agents
   cannot use LSP
3. Mainstream AI agents like Claude Code / Continue already support MCP by default, but it's a blank
   ecosystem for YaoXiang

### What is MCP?

MCP (Model Context Protocol) is an AI agent tool-calling protocol led and open-sourced by Anthropic
in 2024-2025, which has become a de facto standard (OpenAI, Google, Microsoft, Zed, Continue, Cody,
etc. have integrated). Features:

- Based on JSON-RPC 2.0 (shares lineage with LSP)
- Three primitives: **Tools** (actions), Resources (data), Prompts (templates)
- Transport: `stdio` (child process) / streamable `HTTP` / SSE
- Tool input/output has **JSON Schema** strong typing (LLM-friendly)
- 2025-06+ has released the streamable HTTP specification; this RFC is also compatible with the old
  SSE

**This RFC only uses the Tools primitive** — aligned with LSP's "provide services", without
introducing the complexity of Resources' file model.

## Proposal

### Core design

Single binary dual mode:

```text
┌─────────────────────────────────────────────────────────┐
│                    yaoxiang (v0.7.7+)                  │
│  ┌─────────────────┐      ┌──────────────────────────┐  │
│  │ yaoxiang lsp    │      │   yaoxiang mcp           │  │
│  │ (stdio JSON-RPC)│      │   (stdio default         │  │
│  │ RFC-017 done    │      │    + HTTP optional)      │  │
│  └────────┬────────┘      └──────────┬───────────────┘  │
│           │                         │                   │
│           ▼                         ▼                   │
│  ┌──────────────────────────────────────────────────┐  │
│  │  Shared lib crate (`yaoxiang`)                   │  │
│  │  src/lsp/{server,session,world}.rs               │  │
│  │  src/frontend/{lexer,parser,core}/...            │  │
│  │  src/middle/...                                  │  │
│  └──────────────────────────────────────────────────┘  │
│                                                          │
│  ┌──────────────────────────────────────────────────┐  │
│  │            src/mcp/  ← new                        │  │
│  │  ├── mod.rs          (module entry + startup)     │  │
│  │  ├── transport/      (stdio + HTTP/SSE)          │  │
│  │  ├── server.rs       (JSON-RPC message loop)     │  │
│  │  ├── tools/          (6 tool handlers)           │  │
│  │  ├── schema.rs       (input/output JSON Schema)  │  │
│  │  └── project.rs      (project root + path resolution) │
│  └──────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

**Key decisions**:

- **Same binary**: `yaoxiang` switches via subcommand; LSP process and MCP process **do not
  coexist** in the same runtime
- **Multi-process independent World**: each `yaoxiang mcp` process holds one `World`; does not
  affect LSP process or other MCP processes (no lock contention, independent crash isolation)
- **stdio default**: avoid port conflicts, zero network configuration; HTTP as a fallback option
- **Reuse rather than duplicate**: directly call the lib APIs of `yaoxiang::frontend` /
  `yaoxiang::middle` / `yaoxiang::lsp::handlers`, **not** via LSP-client transit

### Tool set (8 tools, delivered in 3 phases)

Designed following the "eliminate special cases + phased" principle: pure source stateless tools
first, workspace tools sharing LSP World, AST rewriting tools added independently.

| Tool name            | Input                                                                                           | Output                                                       | Reuse                                                                | Phase       |
| -------------------- | ----------------------------------------------------------------------------------------------- | ------------------------------------------------------------ | -------------------------------------------------------------------- | ----------- |
| `parse_source`       | `source: String`, `tab_size?: u32`                                                              | `{ast: Node, diagnostics: Diagnostic[]}`                     | Directly call `frontend::parse`                                      | v0.8.x      |
| `format_source`      | `source: String`, `tab_size?: u32`                                                              | `{formatted: String, diff: Hunk[]}`                          | Directly call `formatter::format`                                    | v0.8.x      |
| `lookup_symbol`      | `query: String`, `workspace_root?: String`, `kind?: SymbolKind[]`                               | `{symbols: Symbol[]}`                                        | Reuse `lsp::handlers::workspace_symbol` (fuzzy match by `query`)     | v0.8.x      |
| `find_references`    | `query: String`, `workspace_root?: String`                                                      | `{locations: Location[]}`                                    | Reuse `lsp::handlers::references` (by `query` not position)          | v0.8.x      |
| `typecheck`          | `file_paths: String[]`, `project_root: String`                                                  | `{diagnostics: Diagnostic[], summary: Counts}`               | Reuse `lsp::world::typecheck_full`                                   | v0.8.x      |
| `explain_diagnostic` | `code: String` (e.g. `E0001`), `lang?: String`                                                  | `{code, category, title, description, example, help}`        | **Directly call** `util::diagnostic::command::render_explain_output` | **v0.9.x**  |
| `list_imports`       | `file_path: String`, `project_root?: String`                                                    | `{imports: [{module, items, source_file}]}`                  | Reuse `middle::passes::module::ModuleGraph::validate_imports`        | **v0.9.x**  |
| `rename_symbol`      | `source: String`, `old_name: String`, `new_name: String`, `scope?: "module" \| "function:name"` | `{source: String, edits: Edit[], diagnostics: Diagnostic[]}` | **New** `src/middle/rename.rs` (AST rewrite)                         | **v0.10.x** |

**Boundary of 8 tools**:

- `parse_source` / `format_source` — **pure source stateless**, not entering World
- `lookup_symbol` / `find_references` — take `workspace_root` (if not passed, use `--project-root`
  at startup)
- `typecheck` — **required** `file_paths`, ensuring complete workspace
- `explain_diagnostic` — **zero file dependency**, pure string query error code registry
- `list_imports` — `file_path` physical file, output that file's import resolution result
- `rename_symbol` — **pure source AST rewrite**, no LSP-style position query (semantics different
  from existing `lsp::handlers::rename`)
- ~~`hover` / `completion` / `signature_help`~~ — **all cut**: AI agents don't do
  "position-sensitive" semantics, rely on `lookup_symbol` by name query instead

**World load timing**: at server startup, scan `yaoxiang.toml` and `src/**/*.yx` by
`--project-root`, reuse the `World::load_*` API already implemented in LSP-017 to load into
`World.documents` in one shot. **No** new lib API.

### Tool contract

**Input**: described in JSON Schema, each field has `description` + `examples` (LLM automatically
understands).

**Output**: structured JSON, uniformly with `schemaVersion: "1.0"` field:

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

**Error system**:

- **Diagnostics**: parse/type errors, following RFC-013 (`E0001` etc.) — **not counted as tool
  error**
- **Tool-level errors**: prefixed with `MCP-` (`MCP-INVALID-INPUT`, `MCP-PROJECT-NOT-FOUND`,
  `MCP-INTERNAL`) — treated as `isError: true`
- **panic/crash**: JSON-RPC `-32603 Internal error`, server does not exit

**Path resolution rules** (apply to `workspace_root` of `lookup_symbol` / `find_references`,
`file_paths` of `typecheck`):

1. Command line `--project-root <dir>` highest priority (override default)
2. Otherwise: cwd walks up to find `yaoxiang.toml` until filesystem root (following RFC-015)
3. Otherwise: cwd itself
4. `file_paths` must be inside project root (anti-traversal); out-of-bounds →
   `MCP-PATH-OUTSIDE-PROJECT`

### Transport layer

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
yaoxiang mcp --http --sse --addr 127.0.0.1:7325  # compatible with old SSE (v0.10)
```

**Security constraints**:

- **Only listen on loopback** (127.0.0.1 / ::1); public binding explicitly rejected with error and
  exit
- HTTP **no authentication** (loopback defaults to trusted); future add `--require-token <hex>`
  field
- stdio child process mode is naturally isolated (parent process controls permissions)

### Multi-process and concurrency

Each `yaoxiang mcp` process holds one `World`, not shared with each other:

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

**Port conflict**: AI agent configures "launch child process" — naturally zero port conflict. HTTP
mode requires user-managed port allocation. **World isolation**: each process has independent LSP
sync state — one MCP process crash **does not affect** LSP/other MCP processes. **future Sessions**:
v2 will consider multi-workspace dispatch (multiple `Session` in same process), **not in this RFC**.

## Detailed design

### Data structures

Add `src/mcp/project.rs`:

```rust
pub struct ProjectRoot {
    /// absolute path
    pub root: PathBuf,
    /// strategy source when loading project root
    pub source: ProjectRootSource,
}

pub enum ProjectRootSource {
    CliFlag,           // yaoxiang mcp --project-root
    AutoDetected,      // walk up to find yaoxiang.toml
    FallbackCwd,       // fallback to cwd
}

pub struct ResolvedPath {
    /// relative path to project root (recommended for AI to read)
    pub relative: String,
    /// resolved absolute path (for World operations)
    pub absolute: PathBuf,
}

impl ProjectRoot {
    /// resolve "file_path" as safe path — anti-traversal
    pub fn resolve(&self, file_path: &str) -> Result<ResolvedPath, McpError>;
}
```

`ProjectRoot` singleton + `src/mcp/schema.rs` tool schema auto-generation:

```rust
pub struct ProjectRoot {
    /// absolute path (must contain `yaoxiang.toml` or backward-compatible fallback)
    pub root: PathBuf,
    pub source: ProjectRootSource,
}

impl ProjectRoot {
    /// Identify once at CLI startup, result cached in `McpServer` context — all tools reuse
    pub fn detect(cli_override: Option<PathBuf>) -> Result<Self, McpError>;
}
```

Tool schema uses `schemars` crate to auto-generate from input struct, avoiding hand-written JSON
Schema drift:

```rust
#[derive(Deserialize, schemars::JsonSchema)]
pub struct ParseSourceInput {
    /// complete YaoXiang source snippet — **not** saved to disk, pure transient
    pub source: String,
    pub tab_size: Option<u32>,
}
```

**`parse_source` / `format_source` tool schema has no `file_path` field** — these two tools only
accept string sources, not participating in project semantics. `lookup_symbol` / `find_references` /
`typecheck` accept `workspace_root` or `file_paths` (required or not see tool table).

### Compiler changes

| Module                                 | Change                                                                                              |
| -------------------------------------- | --------------------------------------------------------------------------------------------------- |
| `src/lsp/world.rs`                     | **Zero changes** — MCP startup calls existing LSP `World::load_*` API to load workspace in one shot |
| `src/lsp/handlers/workspace_symbol.rs` | **Zero changes** — `mcp/tools/lookup.rs` wraps a layer to convert `query` to LSP input              |
| `src/lsp/handlers/references.rs`       | **Zero changes** — same as above                                                                    |
| `src/lsp/handlers/formatter.rs`        | **Zero changes** — format_source directly calls                                                     |
| `src/main.rs`                          | Add `Mcp` subcommand branch                                                                         |
| `Cargo.toml`                           | Add `mcp-server` feature (or main binary always carries)                                            |
| `src/util/diagnostic/`                 | **Zero changes** (RFC-017 already implemented)                                                      |

**Key constraint**: `src/mcp/` **not allowed** to reverse-depend on private symbols of `src/lsp/` —
can only call handlers through `crate::lsp::` public API.

### Backward compatibility

- ✅ **Fully backward compatible**: new subcommand `yaoxiang mcp`, does not change any existing
  behavior of `yaoxiang` / `yaoxiang lsp`
- ✅ **LSP server unchanged**: all capabilities, APIs, internal state implemented in RFC-017
  unchanged
- ✅ **lib crate public API unchanged**: all `pub` paths unchanged; MCP only consumes existing API —
  **zero** new `pub` methods

### Integration with existing systems

| Existing module                           | MCP integration way                                                                              |
| ----------------------------------------- | ------------------------------------------------------------------------------------------------ |
| `src/frontend/lexer`                      | parse_source directly calls lexer                                                                |
| `src/frontend/core/parser`                | parse_source directly calls parser; failure produces `Missing*` node (RFC-017)                   |
| `src/frontend/core/typecheck/inference/*` | typecheck reuses `collect_diagnostics` pattern (RFC-017 §Problem 1)                              |
| `src/middle/`                             | typecheck runs all middle passes (dependency analysis, etc.)                                     |
| `src/lsp/world.rs`                        | Startup calls `World::load_*` API (existing); World **does not** accept any "virtual document"   |
| `src/lsp/handlers/workspace_symbol.rs`    | `mcp/tools/lookup.rs` wraps a layer, converts `query: String` to LSP input (by name)             |
| `src/lsp/handlers/references.rs`          | `mcp/tools/find_refs.rs` wraps a layer, converts `query: String` to LSP input                    |
| `src/lsp/handlers/formatter.rs`           | `mcp/tools/format.rs` directly calls (if not implemented, add new `formatter::format_with_diff`) |
| `src/util/i18n/`                          | Error messages go through multilingual resource files (zh-CN/en)                                 |

### Error handling

| Source                                        | Handling                                                                                      |
| --------------------------------------------- | --------------------------------------------------------------------------------------------- |
| Parse error                                   | `Diagnostic{code:"E0xxx", severity, message, span}` (**not tool error**, returned in content) |
| Type error                                    | Same as above                                                                                 |
| `file_paths` out-of-bounds (`typecheck` tool) | Tool-level error `MCP-PATH-OUTSIDE-PROJECT`                                                   |
| `source` invalid UTF-8                        | Tool-level error `MCP-INVALID-INPUT`                                                          |
| Tool panic                                    | JSON-RPC `-32603 Internal error`; server **does not exit**                                    |
| Client sends non-JSON-RPC                     | Directly close stream (stdio EOF), restart means new session                                  |

Diagnostic severity levels follow RFC-017 (already implemented)
`enum ErrorKind { Error, Warning, Note }`.

### Test strategy

| Layer           | Test                                                                                                 |
| --------------- | ---------------------------------------------------------------------------------------------------- |
| **Unit**        | `src/mcp/project.rs::resolve` path traversal, `src/mcp/schema.rs` schema validation                  |
| **Integration** | mock stdio: start a server, pour JSON-RPC into stdin, read response from stdout, compare fixture     |
| **E2E**         | run real `yaoxiang mcp` process, Claude Code style tool call chain: parse → fix → format → typecheck |
| **Fuzz**        | `cargo-fuzz` for MCP JSON-RPC parsing (libFuzzer harness)                                            |

Each tool must have at least 1 happy path + 1 diagnostic scenario + 1 tool-error scenario
integration test.

## Trade-offs

### Advantages

- **Extremely low reuse cost**: `World` / `Session` / `handlers` / diagnostic collection all already
  implemented (RFC-017), this RFC is "add a layer of MCP shell"
- **AI-First interface**: tool contract 3-5 times more intuitive than LSP; LLM directly reads schema
- **Multi-process isolation**: decoupled from LSP editor session and other MCP processes, **zero
  lock contention**
- **stdio friendly**: all mainstream AI agents default to child process mode, zero-config
  integration
- **YAGNI passed**: this RFC cuts Resources, Sessions, cross-process state, remote MCP — v2 will
  open them again

### Disadvantages

- **Protocol fragmentation**: LSP / MCP / DAP will evolve separately in the future, consistency
  maintenance cost
- **HTTP mode second-class citizen**: loopback restriction positioned as local tool, remote
  scenarios need v2 redesign
- **Repeated parse overhead**: AI repeatedly fine-tunes source and repeatedly calls `parse_source`
  will re-lexer+parse. **Mitigation**: rely on RFC-017's `DocumentCache` to still accelerate
  **disk** same source's second parse; pure transient source unavoidable one parse
- **Test coverage cost**: 5 tools × 3 scenarios = 15 integration tests to start

## Alternatives

| Plan                                                             | Why not choose                                                                                                     |
| ---------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| **In-process embedded dual protocol** (LSP+MCP listener coexist) | stdin/stdout can only have one consumer; HTTP also needs to coexist — complexity > benefit                         |
| **MCP as LSP-client bridge**                                     | One more IPC layer; LSP design doesn't support by-name symbol query — MCP's desired capabilities LSP can't provide |
| **Use gRPC / custom protocol**                                   | Deviate from de facto standard; community already has MCP SDK (TypeScript, Python, Rust), with ecosystem           |
| **Reuse all LSP handler capabilities** (L3 tool set)             | Lots of position↔intent adaptation work; diminishing marginal returns                                              |
| **First version only HTTP** (no stdio)                           | Claude Code / Continue etc. default stdio, barrier too high                                                        |

## Implementation strategy

### Dependencies

- **Strong dependency**: RFC-017 LSP implementation (already landed)
- **Strong dependency**: RFC-013 error code system (already landed)
- **Strong dependency**: RFC-014 / RFC-015 project root identification (partially landed)
- **New dependencies** (Rust crate):
  - `mcp-rust-sdk` (to be evaluated, reference
    [modelcontextprotocol/rust-sdk](https://github.com/modelcontextprotocol/rust-sdk))
  - `tokio` (**existing**, optional feature)
  - `axum` (HTTP mode) or `hyper` directly — to be evaluated
- **Zero language specification changes**: pure toolchain increment

### Implementation phases

| Phase                              | Content                                                                                                                                                                                                                             | Duration estimate |
| ---------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------- |
| **v0.8.x (MVP)**                   | `src/mcp/{mod.rs, server.rs, transport/stdio.rs, project.rs, schema.rs}` + `parse_source` + `format_source` + `lookup_symbol` + `find_references` + `typecheck` (**5 tools**) + `yaoxiang mcp` subcommand + startup `World::load_*` | **3-4 weeks**     |
| **v0.9.x (YaoXiang intelligence)** | `+ explain_diagnostic` (**directly call** `render_explain_output`) + `+ list_imports` (wrap `ModuleGraph::validate_imports`) + unit/integration tests                                                                               | **1-2 weeks**     |
| **v0.10.x (AST + HTTP)**           | `+ rename_symbol` (**new** `src/middle/rename.rs`, AST rewrite) + streamable HTTP transport + performance tuning (parse_source P99 < 100ms)                                                                                         | **2-3 weeks**     |

**Why split into 3 phases**: MVP first gets stdio + 5 tools working to verify interface design is
reasonable; v0.9.x adds low-risk zero-adaptation "YaoXiang-specific" tools to verify integration is
correct; v0.10.x then opens the high-risk "AST rewrite" new module (independent PR review more
focused).

### Risks

1. **`mcp-rust-sdk` maintenance activity**: released in 2025, API may change drastically.
   **Mitigation**: if evaluation shows unstable, write lightweight JSON-RPC 2.0 + tool dispatcher
   yourself (< 500 lines)
2. **Repeated parse overhead**: AI repeatedly fine-tunes source and repeatedly calls `parse_source`
   will re-lexer+parse. **Mitigation**: rely on RFC-017's `DocumentCache` to still accelerate
   **disk** same source's second parse; pure transient source unavoidable one parse
3. **AI agent schema compatibility**: different agents' MCP schema strictness differs.
   **Mitigation**: use `schemars` crate to auto-generate schema from Rust input structure, zero
   hand-written drift
4. **Path resolution multi-platform**: Windows path case-insensitive, UNC path, `\\` boundary.
   **Mitigation**: path resolution uses `camino::Utf8Path` to replace `std::path`
5. **MCP tool schema and LSP input not 1:1**: LSP `workspace_symbol` takes `(query)`; when passing
   into LSP internals need to wrap as position+URI to let existing handler reuse. **Mitigation**: in
   `mcp/tools/lookup.rs` do adaptation layer, encapsulate details on MCP side
6. **`rename_symbol` AST rewrite and LSP `rename` semantics differ**: LSP `textDocument/rename` is
   URI + position + new_name → WorkspaceEdit; MCP `rename_symbol` is source + old_name + new_name →
   new source. **Cannot directly reuse**. **Mitigation**: implement separately
   `src/middle/rename.rs`, scope-aware rewrite references, does not interfere with LSP handler
   implementation

## Open questions

- [ ] `mcp-rust-sdk` selection / self-implementation? (@Chen Xu: first evaluate rust-sdk June
      version, then decide)
- [ ] HTTP authentication path? (v0.10 RFC opens again)
- [ ] Does MCP need to output `tools/list` to AI for active discovery at startup? (MCP standard
      requires, **default implementation**)
- [ ] Does `typecheck` support `mode: "fast|full"` (fast = only current file subset, full = entire
      workspace)?
- [ ] Is performance budget parse_source P99 < 100ms realistic? (need benchmark RFC-017 already
      implemented `DocumentCache` actual overhead in source-string mode)

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
- [zed-industries/zed's MCP implementation](https://github.com/zed-industries/zed/tree/main/crates/mcp)

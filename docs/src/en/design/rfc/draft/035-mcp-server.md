---
title: "RFC-035: MCP Server Support (AI Agent Integration)"
status: "Draft"
author: "Chen Xu"
created: "2026-07-11"
updated: "2026-07-11"
issue: "#154"
---

# RFC-035: MCP Server Support (AI Agent Integration)

## Summary

Add an MCP (Model Context Protocol) server to YaoXiang, enabling AI agents (Claude Code, Continue, Cody, Zed, etc.) to directly query the **AST, parse errors, types, symbols, references, and formatting results** of YaoXiang source code. It reuses the `World` backend already implemented in RFC-017, adds a new `yaoxiang mcp` subcommand, provides a single binary with dual mode, and uses independent `World` instances per process.

## Motivation

### Why is this feature needed?

RFC-017 makes YaoXiang **understandable** to editors (hover / goto-def / completion). But LSP is a **position-driven** protocol:
- Every request strongly depends on a `textDocument` URI + `Position`
- The editor must first open a file, save it, and maintain a long connection with the LSP server
- AI agent workflows work with **code snippets**: pasting "a piece of code" in a conversation to ask questions, **without** first saving to disk

The LSP clients actually available to AI agents (vscode-langservers-extracted, `mcp-lsp-bridge`-like projects) only translate L1: goto-def, hover. What AI wants to do:
- "Is this code **parsed correctly**" — requires parse + full diagnostic stream
- "How is this symbol **used in the file**" — requires `lookup_symbol` lookup by name
- "What does this code **look like after formatting**" — requires `format_source`
- "Where are **all** type errors" — requires `typecheck` run on the entire workspace

These L1 LSP translation capabilities **cannot do it**, because LSP was not designed to support such use cases.

### Current Problems

1. **Poor experience when AI agents call LSP**: need to mock documents, JSON is huge, strong URI dependency
2. **YaoXiang project lacks an "AI-First" interface layer**: humans use IDE with LSP, AI agents cannot use LSP
3. **Mainstream AI agents like Claude Code / Continue already support MCP by default, but YaoXiang has a blank ecosystem**

### What is MCP?

MCP (Model Context Protocol) is an AI agent tool-calling protocol released and open-sourced by Anthropic in 2024-2025, which has become a de facto standard (OpenAI, Google, Microsoft, Zed, Continue, Cody, etc. have adopted it). Features:
- Based on JSON-RPC 2.0 (same origin as LSP)
- Three primitives: **Tools** (actions), Resources (data), Prompts (templates)
- Transport: `stdio` (subprocess) / streamable `HTTP` / SSE
- Tool input/output has **JSON Schema** strong typing (LLM-friendly)
- The streamable HTTP spec has been released in 2025-06+; this RFC is also compatible with the legacy SSE

**This RFC only uses the Tools primitive** — aligned with LSP's "provide services" model, without introducing the file model complexity of Resources.

## Proposal

### Core Design

Single binary, dual mode:

```text
┌─────────────────────────────────────────────────────────┐
│                    yaoxiang (v0.7.7+)                   │
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
│  │            src/mcp/  ← new                       │  │
│  │  ├── mod.rs          (module entry + startup)    │  │
│  │  ├── transport/      (stdio + HTTP/SSE)          │  │
│  │  ├── server.rs       (JSON-RPC message loop)     │  │
│  │  ├── tools/          (6 tool handlers)           │  │
│  │  ├── schema.rs       (input/output JSON Schema)  │  │
│  │  └── project.rs      (project root detection)    │  │
│  └──────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

**Key decisions**:
- **Same binary**: `yaoxiang` switches via subcommand; LSP process and MCP process **do not coexist** in the same runtime
- **Multi-process independent World**: each `yaoxiang mcp` process holds a `World`; does not affect LSP process or other MCP processes (no lock contention, independent crash isolation)
- **stdio default**: avoids port conflicts, zero network configuration; HTTP as optional fallback
- **Reuse, not duplicate**: directly call the lib API of `yaoxiang::frontend` / `yaoxiang::middle` / `yaoxiang::lsp::handlers`, **not** going through LSP-client transit

### Tool Set (8 tools, delivered in 3 phases)

Designed by the "eliminate special cases + phased" principle: pure-source stateless tools first, workspace tools sharing the LSP World, AST rewrite tools added independently.

| Tool Name | Input | Output | Reuse | Phase |
|---|---|---|---|---|
| `parse_source` | `source: String`, `tab_size?: u32` | `{ast: Node, diagnostics: Diagnostic[]}` | Directly call `frontend::parse` | v0.8.x |
| `format_source` | `source: String`, `tab_size?: u32` | `{formatted: String, diff: Hunk[]}` | Directly call `formatter::format` | v0.8.x |
| `lookup_symbol` | `query: String`, `workspace_root?: String`, `kind?: SymbolKind[]` | `{symbols: Symbol[]}` | Reuse `lsp::handlers::workspace_symbol` (fuzzy match by `query`) | v0.8.x |
| `find_references` | `query: String`, `workspace_root?: String` | `{locations: Location[]}` | Reuse `lsp::handlers::references` (by `query` instead of position) | v0.8.x |
| `typecheck` | `file_paths: String[]`, `project_root: String` | `{diagnostics: Diagnostic[], summary: Counts}` | Reuse `lsp::world::typecheck_full` | v0.8.x |
| `explain_diagnostic` | `code: String` (e.g. `E0001`), `lang?: String` | `{code, category, title, description, example, help}` | **Directly call** `util::diagnostic::command::render_explain_output` | **v0.9.x** |
| `list_imports` | `file_path: String`, `project_root?: String` | `{imports: [{module, items, is_public}]}` | Reuse `middle::passes::module::ModuleGraph::validate_imports` | **v0.9.x** |
| `rename_symbol` | `source: String`, `old_name: String`, `new_name: String`, `scope?: "module" \| "function:name"` | `{source: String, edits: Edit[], diagnostics: Diagnostic[]}` | **Newly add** `src/middle/rename.rs` (AST rewrite) | **v0.10.x** |

**Boundaries of the 8 tools**:
- `parse_source` / `format_source` — **pure source stateless**, do not enter the World
- `lookup_symbol` / `find_references` — accept `workspace_root` (if not passed, use the `--project-root` at startup)
- `typecheck` — `file_paths` **required**, ensures workspace completeness
- `explain_diagnostic` — **zero file dependency**, pure string query against the error code registry
- `list_imports` — `file_path` is a physical file, outputs the import resolution result of that file
- `rename_symbol` — **pure source AST rewrite**, does not do LSP-style position queries (semantics differ from the existing `lsp::handlers::rename`)
- ~~`hover` / `completion` / `signature_help`~~ — **all cut**: AI agents do not do "position-sensitive" semantics, replaced by `lookup_symbol` lookup by name

**World loading timing**: at server startup, scan `yaoxiang.toml` and `src/**/*.yx` according to `--project-root`, reuse the `World::load_*` API already implemented in RFC-017 to load into `World.documents` in one shot. **No new lib API added**.

### Tool Contract

**Input**: described using JSON Schema, with `description` + `examples` for each field (LLM reads it automatically).

**Output**: structured JSON, uniformly carrying the `schemaVersion: "1.0"` field:

```jsonc
// Success response
{
  "schemaVersion": "1.0",
  "isError": false,
  "content": [
    { "type": "json", "json": { /* tool-specific data */ } }
  ]
}

// Diagnostics returned structurally (not treated as a tool error)
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
- **Diagnostics**: parse/type errors, following RFC-013 (`E0001` etc.) — **not a tool error**
- **Tool-level errors**: use the `MCP-` prefix (`MCP-INVALID-INPUT`, `MCP-PROJECT-NOT-FOUND`, `MCP-INTERNAL`) — treated as `isError: true`
- **panic/crash**: JSON-RPC `-32603 Internal error`, server does not exit

**Path resolution rules** (apply to `workspace_root` of `lookup_symbol` / `find_references`, and `file_paths` of `typecheck`):
1. Command line `--project-root <dir>` has highest priority (overrides default)
2. Otherwise: walk up from cwd looking for `yaoxiang.toml` until the filesystem root (follows RFC-015)
3. Otherwise: cwd itself
4. `file_paths` must fall within the project root (prevents traversal); out of bounds → `MCP-PATH-OUTSIDE-PROJECT`

### Transport Layer

**stdio (default)**:

```bash
yaoxiang mcp
# After startup, reads JSON-RPC from stdin, writes to stdout, stderr is used for logs
```

AI agent configuration (Claude Code `.mcp.json` / Continue `config.json`):
```jsonc
{
  "mcpServers": {
    "yaoxiang": {
      "command": "yaoxiang",
      "args": ["mcp", "--project-root", "${workspaceFolder}"]
    }
  }
}
```

**streamable HTTP (optional)**:

```bash
yaoxiang mcp --http --addr 127.0.0.1:7325  # Single HTTP port, new MCP spec
yaoxiang mcp --http --sse --addr 127.0.0.1:7325  # Compatible with legacy SSE (v0.10)
```

**Security constraints**:
- **Only listens on loopback** (127.0.0.1 / ::1); public binding is explicitly rejected and exits with an error
- HTTP **has no auth** (loopback trusted by default); future addition of `--require-token <hex>` field
- stdio subprocess mode is naturally isolated (parent process controls permissions)

### Multi-process and Concurrency

Each `yaoxiang mcp` process holds a `World`, not shared with each other:

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

**Port conflicts**: AI agent configuration "launch subprocess" — naturally zero port conflicts. HTTP mode requires user-managed port allocation.
**World isolation**: each process has independent LSP sync state — one MCP process crash **does not affect** the LSP/other MCP processes.
**future Sessions**: v2 will consider multi-workspace dispatch (multiple `Session` in the same process), **not done in this RFC**.

## Detailed Design

### Data Structures

Add `src/mcp/project.rs`:

```rust
pub struct ProjectRoot {
    /// Absolute path
    pub root: PathBuf,
    /// Source of the project root identification strategy at load time
    pub source: ProjectRootSource,
}

pub enum ProjectRootSource {
    CliFlag,           // yaoxiang mcp --project-root
    AutoDetected,      // Walk up to find yaoxiang.toml
    FallbackCwd,       // Fallback to cwd
}

pub struct ResolvedPath {
    /// Relative path to the project root (recommended for AI to read)
    pub relative: String,
    /// Resolved absolute path (used for World operations)
    pub absolute: PathBuf,
}

impl ProjectRoot {
    /// Resolve "file_path" to a safe path — prevents traversal
    pub fn resolve(&self, file_path: &str) -> Result<ResolvedPath, McpError>;
}
```

`ProjectRoot` singleton + `src/mcp/schema.rs` tool schema auto-generation:

```rust
pub struct ProjectRoot {
    /// Absolute path (must contain `yaoxiang.toml` or fall back for compatibility)
    pub root: PathBuf,
    pub source: ProjectRootSource,
}

impl ProjectRoot {
    /// Detected once at CLI startup, result cached in `McpServer` context — all tools reuse it
    pub fn detect(cli_override: Option<PathBuf>) -> Result<Self, McpError>;
}
```

Tool schemas are auto-generated from input struct using the `schemars` crate, avoiding hand-written JSON Schema drift:

```rust
#[derive(Deserialize, schemars::JsonSchema)]
pub struct ParseSourceInput {
    /// Complete YaoXiang source code snippet — **not** saved to disk, purely transient
    pub source: String,
    pub tab_size: Option<u32>,
}
```

**The `parse_source` / `format_source` tool schemas do not have a `file_path` field** — these two tools only accept a string source and do not participate in project semantics. `lookup_symbol` / `find_references` / `typecheck` accept `workspace_root` or `file_paths` (required or not as per the tool table).


### Compiler Changes

| Module | Change |
|---|---|
| `src/lsp/world.rs` | **Zero changes** — MCP calls existing `World::load_*` API of LSP at startup to load the workspace in one shot |
| `src/lsp/handlers/workspace_symbol.rs` | **Zero changes** — `mcp/tools/lookup.rs` wraps a layer to convert `query` to LSP input |
| `src/lsp/handlers/references.rs` | **Zero changes** — same as above |
| `src/lsp/handlers/formatter.rs` | **Zero changes** — format_source calls it directly |
| `src/main.rs` | Add `Mcp` subcommand branch |
| `Cargo.toml` | Add `mcp-server` feature (or always carried in the main binary) |
| `src/util/diagnostic/` | **Zero changes** (RFC-017 has landed) |

**Key constraint**: `src/mcp/` **is not allowed** to depend inversely on private symbols in `src/lsp/` — can only call handlers through the public API of `crate::lsp::`.

### Backward Compatibility

- ✅ **Fully backward compatible**: new subcommand `yaoxiang mcp`, does not change any existing behavior of `yaoxiang` / `yaoxiang lsp`
- ✅ **LSP server unchanged**: all capabilities, API, and internal state implemented in RFC-017 remain unchanged
- ✅ **lib crate public API unchanged**: all `pub` paths unchanged; MCP only consumes existing API — **zero** new `pub` methods

### Integration with Existing Systems

| Existing Module | MCP Integration |
|---|---|
| `src/frontend/lexer` | `parse_source` calls lexer directly |
| `src/frontend/core/parser` | `parse_source` calls parser directly; failures produce `Missing*` nodes (RFC-017) |
| `src/frontend/core/typecheck/inference/*` | `typecheck` reuses the `collect_diagnostics` pattern (RFC-017 §Problem 1) |
| `src/middle/` | `typecheck` runs all middle passes (dependency analysis, etc.) |
| `src/lsp/world.rs` | Calls `World::load_*` API at startup (already exists); World **does not** accept any "virtual document" |
| `src/lsp/handlers/workspace_symbol.rs` | `mcp/tools/lookup.rs` wraps a layer, converting `query: String` to LSP input (lookup by name) |
| `src/lsp/handlers/references.rs` | `mcp/tools/find_refs.rs` wraps a layer, converting `query: String` to LSP input |
| `src/lsp/handlers/formatter.rs` | `mcp/tools/format.rs` calls it directly (if not implemented, newly add `formatter::format_with_diff`) |
| `src/util/i18n/` | Error messages go through multilingual resource files (zh-CN/en) |

### Error Handling

| Source | Handling |
|---|---|
| Parse error | `Diagnostic{code:"E0xxx", severity, message, span}` (**not a tool error**, returned in content) |
| Type error | Same as above |
| `file_paths` out of bounds (`typecheck` tool) | Tool-level error `MCP-PATH-OUTSIDE-PROJECT` |
| `source` invalid UTF-8 | Tool-level error `MCP-INVALID-INPUT` |
| Tool panic | JSON-RPC `-32603 Internal error`; server **does not exit** |
| Client sends non-JSON-RPC | Directly close the stream (stdio EOF), restart as a new session |

Diagnostic severity follows RFC-017 (already landed) `enum ErrorKind { Error, Warning, Note }`.

### Test Strategy

| Layer | Test |
|---|---|
| **Unit** | `src/mcp/project.rs::resolve` path traversal, `src/mcp/schema.rs` schema validation |
| **Integration** | mock stdio: start a server, feed JSON-RPC via stdin, read response from stdout, compare against fixtures |
| **E2E** | Run real `yaoxiang mcp` process, Claude Code-style tool call chain: parse → fix → format → typecheck |
| **Fuzz** | `cargo-fuzz` for MCP JSON-RPC parsing (libFuzzer harness) |

Each tool must have at least 1 happy path + 1 diagnostic scenario + 1 tool-error scenario integration test.

## Trade-offs

### Advantages

- **Extremely low reuse cost**: `World` / `Session` / `handlers` / diagnostic collection are all already landed (RFC-017); this RFC is "adding an MCP shell"
- **AI-First interface**: tool contract is 3-5 times more intuitive than LSP; LLM directly reads the schema
- **Multi-process isolation**: decoupled from LSP editor sessions and other MCP processes, **zero lock contention**
- **stdio friendly**: all mainstream AI agents default to subprocess mode, zero-config integration
- **YAGNI passed**: this RFC cuts Resources, Sessions, cross-process state, remote MCP — open in v2

### Disadvantages

- **Protocol split**: LSP / MCP / DAP will evolve independently in the future, consistency maintenance cost
- **HTTP mode is second-class citizen**: loopback restriction positioned as a local tool, remote scenarios need v2 redesign
- **Repeated parse overhead**: AI repeatedly tweaking source and repeatedly calling `parse_source` will re-lexer+parser. **Mitigation**: rely on RFC-017's `DocumentCache` to still accelerate secondary parsing of the same source **on disk**; pure transient source parsing once is unavoidable
- **Test coverage cost**: 5 tools × 3 scenarios = 15 integration tests as a starting point

## Alternatives

| Option | Why not chosen |
|---|---|
| **Embed dual protocol in-process** (LSP+MCPlistener coexist) | stdin/stdout can only have one consumer; HTTP would also have to coexist — complexity > benefit |
| **MCP as LSP-client bridge** | Adds an extra layer of IPC; LSP is not designed to support symbol lookup by name — MCP's desired capabilities are beyond LSP |
| **Use gRPC / custom protocol** | Deviates from the de facto standard; the community already has MCP SDKs (TypeScript, Python, Rust) with their own ecosystem |
| **Reuse all LSP handler capabilities** (L3 tool set) | A lot of position↔intent adaptation work; diminishing marginal returns |
| **First version only HTTP** (no stdio) | Claude Code / Continue etc. default to stdio, barrier too high |

## Implementation Strategy

### Dependencies

- **Strong dependency**: RFC-017 LSP implementation (already landed)
- **Strong dependency**: RFC-013 error code system (already landed)
- **Strong dependency**: RFC-014 / RFC-015 project root identification (partially landed)
- **New dependencies** (Rust crates):
  - `mcp-rust-sdk` (to be evaluated, see [modelcontextprotocol/rust-sdk](https://github.com/modelcontextprotocol/rust-sdk))
  - `tokio` (**already have**, optional feature)
  - `axum` (HTTP mode) or `hyper` directly — to be evaluated
- **Zero language spec changes**: pure toolchain increment

### Phases (synced with #154)

| Phase | Content | Duration Estimate |
|---|---|---|
| **v0.8.x (MVP)** | `src/mcp/{mod.rs, server.rs, transport/stdio.rs, project.rs, schema.rs}` + `parse_source` + `format_source` + `lookup_symbol` + `find_references` + `typecheck` (**5 tools**) + `yaoxiang mcp` subcommand + `World::load_*` at startup | **3-4 weeks** |
| **v0.9.x (YaoXiang Intelligence)** | `+ explain_diagnostic` (**directly calls** `render_explain_output`) + `+ list_imports` (wraps `ModuleGraph::validate_imports`) + unit/integration tests | **1-2 weeks** |
| **v0.10.x (AST + HTTP)** | `+ rename_symbol` (**newly add** `src/middle/rename.rs`, AST rewrite) + streamable HTTP transport + performance tuning (parse_source P99 < 100ms) | **2-3 weeks** |


**Why 3 phases**: MVP first gets stdio + 5 tools running to verify the interface design is reasonable; v0.9.x adds low-risk, zero-adaptation "YaoXiang-specific" tools to verify integration correctness; v0.10.x opens the high-risk "AST rewrite" new module (independent PR review is more focused).

### Risks

1. **`mcp-rust-sdk` maintenance activity**: only released in 2025, API may change dramatically. **Mitigation**: if the evaluation finds it unstable, write a lightweight JSON-RPC 2.0 + tool dispatcher ourselves (< 500 lines)
2. **Repeated parse overhead**: AI repeatedly tweaking source and repeatedly calling `parse_source` will re-lexer+parser. **Mitigation**: rely on RFC-017's `DocumentCache` to still accelerate secondary parsing of the same source **on disk**; pure transient source parsing once is unavoidable
3. **AI agent schema compatibility**: different agents have different MCP schema strictness. **Mitigation**: use the `schemars` crate to auto-generate schema from Rust input structures, zero hand-written drift
4. **Path resolution across platforms**: Windows path case insensitivity, UNC paths, `\\` boundaries. **Mitigation**: use `camino::Utf8Path` instead of `std::path` for path resolution
5. **MCP tool schema not 1:1 with LSP input**: LSP `workspace_symbol` accepts `(query)`; when passing into LSP internals it needs to be wrapped into position+URI to allow existing handlers to be reused. **Mitigation**: do an adaptation layer in `mcp/tools/lookup.rs`, encapsulate the details on the MCP side
6. **`rename_symbol` AST rewrite semantics differ from LSP `rename`**: LSP `textDocument/rename` is URI + position + new_name → WorkspaceEdit; MCP `rename_symbol` is source + old_name + new_name → new source. **Cannot directly reuse**. **Mitigation**: implement `src/middle/rename.rs` separately, scope-aware rewrite of references, does not interfere with the LSP handler implementation

## Open Questions

- [ ] `mcp-rust-sdk` selection / self-implementation? (@Chen Xu: first evaluate the June version of rust-sdk, then decide)
- [ ] HTTP auth path? (v0.10 RFC to open)
- [ ] Does `MCP` need to output `tools/list` at startup for AI active discovery? (MCP standard requires it, **default implementation**)
- [ ] Does `typecheck` support `mode: "fast|full"` (fast = current file subset only, full = entire workspace)?
- [ ] Is the performance budget parse_source P99 < 100ms realistic? (needs benchmark on the actual overhead of RFC-017's already-landed `DocumentCache` in source-string mode)

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
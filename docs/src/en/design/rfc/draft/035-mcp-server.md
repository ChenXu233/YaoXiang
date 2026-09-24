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
Cody, Zed, etc.) to directly query the **AST, parse errors, types, symbols, references, and
formatted output** of YaoXiang source code. Reuse the `World` backend already implemented in
RFC-017, add a new `yaoxiang mcp` subcommand, support dual-mode in a single binary, and run multiple
processes with independent World instances.

## Motivation

### Why is this feature needed?

RFC-017 made it possible for editors to understand YaoXiang (hover / goto-def / completion). But LSP
is a **position-driven** protocol:

- Every request heavily depends on `textDocument` URI + `Position`
- The editor must first open the file, save it, and maintain a long-lived connection with the LSP
  server
- AI agent workflows work with **code snippets**: "paste a piece of code" in a conversation to ask
  questions, **without** saving it to disk first

The LSP clients actually usable by AI agents (vscode-langservers-extracted, projects like
`mcp-lsp-bridge`) only translate **L1**: goto-def, hover. What AI wants to do:

- "Is this piece of code **parsed correctly**" — needs parse + complete diagnostic stream
- "How is this symbol **used in the file**" — needs lookup_symbol by name
- "What does this code **look like after formatting**" — needs format_source
- "Where are **all** the type errors" — needs typecheck run on the entire workspace

These L1 LSP translation capabilities **cannot do it**, because LSP is not designed to support it.

### Current problems

1. AI agents have a poor experience calling LSP: need to mock documents, JSON is huge, strong URI
   dependency
2. The YaoXiang project lacks an "AI-First" interface layer: humans open IDEs and use LSP, but AI
   agents cannot use LSP
3. Mainstream AI agents like Claude Code / Continue already support MCP by default, leaving a gap in
   the YaoXiang ecosystem

### What is MCP?

MCP (Model Context Protocol) is an AI agent tool-calling protocol led and open-sourced by Anthropic
in 2024-2025. It has become the de facto standard (adopted by OpenAI, Google, Microsoft, Zed,
Continue, Cody, etc.). Features:

- Based on JSON-RPC 2.0 (same origin as LSP)
- Three primitives: **Tools** (actions), Resources (data), Prompts (templates)
- Transports: `stdio` (subprocess) / streamable `HTTP` / SSE
- Tool input/output has strongly-typed **JSON Schema** (LLM-friendly)
- The streamable HTTP spec was released in 2025-06+; this RFC is also compatible with the old SSE

**This RFC only uses the Tools primitive** — aligning with LSP's "service provision" model, without
introducing the file model complexity of Resources.

## Proposal

### Core design

Single binary, dual mode:

```text
┌─────────────────────────────────────────────────────────┐
│                    yaoxiang (v0.7.7+)                    │
│  ┌─────────────────┐      ┌──────────────────────────┐  │
│  │ yaoxiang lsp    │      │   yaoxiang mcp           │  │
│  │ (stdio JSON-RPC)│      │   (stdio default         │  │
│  │ RFC-017 done    │      │    + HTTP optional)      │  │
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
│  │            src/mcp/  ← New                        │  │
│  │  ├── mod.rs          (module entry + boot fn)     │  │
│  │  ├── transport/      (stdio + HTTP/SSE)           │  │
│  │  ├── server.rs       (JSON-RPC message loop)      │  │
│  │  ├── tools/          (8 tool handlers)            │  │
│  │  ├── schema.rs       (input/output JSON Schema)   │  │
│  │  └── project.rs      (project root detection +    │  │
│  │                       path resolution)            │  │
│  └──────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

**Key decisions**:

- **Same binary**: `yaoxiang` switches via subcommand; the LSP process and MCP process do **not
  coexist** in the same runtime
- **Multi-process independent World**: every `yaoxiang mcp` process holds one `World`; it does not
  affect the LSP process or other MCP processes (no lock contention, independent crash isolation)
- **stdio by default**: avoids port conflicts, zero network configuration; HTTP is an optional
  fallback
- **Reuse rather than duplicate**: directly call the lib API of `yaoxiang::frontend` /
  `yaoxiang::middle` / `yaoxiang::lsp::handlers`, **without** going through an LSP-client
  intermediary

### Toolset (8 tools, delivered in 3 phases)

Designed by the principle of "eliminating special cases + phased delivery": stateless pure-source
tools first, workspace tools share the LSP World, AST rewriting tools added independently.

| Tool name            | Input                                                                                           | Output                                                       | Reuse                                                                | Phase       |
| -------------------- | ----------------------------------------------------------------------------------------------- | ------------------------------------------------------------ | -------------------------------------------------------------------- | ----------- |
| `parse_source`       | `source: String`, `tab_size?: u32`                                                              | `{ast: Node, diagnostics: Diagnostic[]}`                     | directly call `frontend::parse`                                      | v0.8.x      |
| `format_source`      | `source: String`, `tab_size?: u32`                                                              | `{formatted: String, diff: Hunk[]}`                          | directly call `formatter::format`                                    | v0.8.x      |
| `lookup_symbol`      | `query: String`, `workspace_root?: String`, `kind?: SymbolKind[]`                               | `{symbols: Symbol[]}`                                        | reuse `lsp::handlers::workspace_symbol` (fuzzy match on `query`)     | v0.8.x      |
| `find_references`    | `query: String`, `workspace_root?: String`                                                      | `{locations: Location[]}`                                    | reuse `lsp::handlers::references` (by `query` rather than position)  | v0.8.x      |
| `typecheck`          | `file_paths: String[]`, `project_root: String`                                                  | `{diagnostics: Diagnostic[], summary: Counts}`               | reuse `lsp::world::typecheck_full`                                   | v0.8.x      |
| `explain_diagnostic` | `code: String` (e.g. `E0001`), `lang?: String`                                                  | `{code, category, title, description, example, help}`        | **directly call** `util::diagnostic::command::render_explain_output` | **v0.9.x**  |
| `list_imports`       | `file_path: String`, `project_root?: String`                                                    | `{imports: [{module, items, source_file}]}`                  | reuse `middle::passes::module::ModuleGraph::validate_imports`        | **v0.9.x**  |
| `rename_symbol`      | `source: String`, `old_name: String`, `new_name: String`, `scope?: "module" \| "function:name"` | `{source: String, edits: Edit[], diagnostics: Diagnostic[]}` | **new** `src/middle/rename.rs` (AST rewriting)                       | **v0.10.x** |

**Boundaries of the 8 tools**:

- `parse_source` / `format_source` — **pure source, stateless**, do not enter World
- `lookup_symbol` / `find_references` — accept `workspace_root` (if not given, use the
  `--project-root` at startup)
- `typecheck` — `file_paths` is **required**, to ensure workspace completeness
- `explain_diagnostic` — **zero file dependency**, pure string query of the error code registry
- `list_imports` — `file_path` is a physical file, outputs the import resolution result of that file
- `rename_symbol` — **pure source AST rewriting**, does not do LSP-style position queries (different
  semantics from the existing `lsp::handlers::rename`)
- ~~`hover` / `completion` / `signature_help`~~ — **all cut**: AI agents do not have
  "position-sensitive" semantics, replaced by `lookup_symbol` queries by name

**World load timing**: at server startup, scan `yaoxiang.toml` and `src/**/*.yx` according to
`--project-root`, and reuse the `World::load_*` API already implemented in RFC-017 to populate
`World.documents` in one shot. **No** new lib API is added.

### Tool contract

**Input**: described by JSON Schema, each field has `description` + `examples` (LLM reads it
automatically).

**Output**: structured JSON, uniformly with the `schemaVersion: "1.0"` field:

```jsonc
// Success response
{
  "schemaVersion": "1.0",
  "isError": false,
  "content": [
    { "type": "json", "json": { /* tool-specific data */ } }
  ]
}

// Diagnostics returned structurally (not treated as tool errors)
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

- **Diagnostics**: parse/type errors, follow RFC-013 (such as `E0001`) — **not** counted as tool
  errors
- **Tool-level errors**: use `MCP-` prefix (`MCP-INVALID-INPUT`, `MCP-PROJECT-NOT-FOUND`,
  `MCP-INTERNAL`) — treated as `isError: true`
- **panic/crash**: JSON-RPC `-32603 Internal error`, server does not exit

**Path resolution rules** (apply to `workspace_root` of `lookup_symbol` / `find_references`, and
`file_paths` of `typecheck`):

1. CLI `--project-root <dir>` has highest priority (overrides default)
2. Otherwise: walk upward from cwd to find `yaoxiang.toml` until the file system root (follows
   RFC-015)
3. Otherwise: cwd itself
4. `file_paths` must be within the project root (prevent traversal); out-of-bounds →
   `MCP-PATH-OUTSIDE-PROJECT`

### Transport layer

**stdio (default)**:

```bash
yaoxiang mcp
# After startup, read JSON-RPC from stdin, write to stdout, stderr is used for logging
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
yaoxiang mcp --http --addr 127.0.0.1:7325  # single HTTP port, new MCP spec
yaoxiang mcp --http --sse --addr 127.0.0.1:7325  # compatible with old SSE (v0.10)
```

**Security constraints**:

- **Listen on loopback only** (127.0.0.1 / ::1); binding to public networks is explicitly rejected
  and exits with an error
- HTTP **has no authentication** (loopback is trusted by default); in the future, add the
  `--require-token <hex>` field
- stdio subprocess mode is naturally isolated (parent process controls permissions)

### Multi-process and concurrency

Each `yaoxiang mcp` process holds one `World`, no sharing:

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

**Port conflicts**: AI agents are configured to "spawn subprocesses" — naturally zero port
conflicts. In HTTP mode, the user must manage port allocation themselves. **World isolation**: each
process has its own independent LSP sync state — a crash in one MCP process does **not** affect
LSP/other MCP processes. **future Sessions**: multi-workspace dispatch (multiple `Session`s in the
same process) is only considered in v2, **not in this RFC**.

## Detailed Design

### Data structures

Add `src/mcp/project.rs`:

```rust
pub struct ProjectRoot {
    /// Absolute path
    pub root: PathBuf,
    /// Source of the strategy that identified the project root at load time
    pub source: ProjectRootSource,
}

pub enum ProjectRootSource {
    CliFlag,           // yaoxiang mcp --project-root
    AutoDetected,      // walk upward to find yaoxiang.toml
    FallbackCwd,       // fallback to cwd
}

pub struct ResolvedPath {
    /// Relative path relative to the project root (recommended for AI to read)
    pub relative: String,
    /// Resolved absolute path (for World operations)
    pub absolute: PathBuf,
}

impl ProjectRoot {
    /// Resolve a "file_path" to a safe path — prevents traversal
    pub fn resolve(&self, file_path: &str) -> Result<ResolvedPath, McpError>;
}
```

`ProjectRoot` singleton + automatic tool schema generation in `src/mcp/schema.rs`:

```rust
pub struct ProjectRoot {
    /// Absolute path (must contain `yaoxiang.toml` or fall back to a compatible downgrade)
    pub root: PathBuf,
    pub source: ProjectRootSource,
}

impl ProjectRoot {
    /// Identified once at CLI startup, the result is cached in the `McpServer` context — reused by all tools
    pub fn detect(cli_override: Option<PathBuf>) -> Result<Self, McpError>;
}
```

Tool schema is auto-generated from input structs using the `schemars` crate, avoiding hand-written
JSON Schema drift:

```rust
#[derive(Deserialize, schemars::JsonSchema)]
pub struct ParseSourceInput {
    /// Complete YaoXiang source code snippet — **not** saved to disk, purely transient
    pub source: String,
    pub tab_size: Option<u32>,
}
```

**The `parse_source` / `format_source` tool schemas do not have a `file_path` field** — these two
tools only accept a string source and do not participate in project semantics. `lookup_symbol` /
`find_references` / `typecheck` accept `workspace_root` or `file_paths` (see the tool table for
whether they are required).

### Compiler changes

| Module                                 | Change                                                                                                    |
| -------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| `src/lsp/world.rs`                     | **Zero changes** — MCP startup calls LSP's existing `World::load_*` API to load the workspace in one shot |
| `src/lsp/handlers/workspace_symbol.rs` | **Zero changes** — `mcp/tools/lookup.rs` wraps a layer to convert `query` to LSP input                    |
| `src/lsp/handlers/references.rs`       | **Zero changes** — same as above                                                                          |
| `src/lsp/handlers/formatter.rs`        | **Zero changes** — format_source calls it directly                                                        |
| `src/main.rs`                          | Add `Mcp` subcommand branch                                                                               |
| `Cargo.toml`                           | Add `mcp-server` feature (or main binary always includes it)                                              |
| `src/util/diagnostic/`                 | **Zero changes** (RFC-017 already implemented)                                                            |

**Key constraint**: `src/mcp/` is **not** allowed to depend on the private symbols of `src/lsp/` —
it can only call handlers through the public API of `crate::lsp::`.

### Backward compatibility

- ✅ **Fully backward compatible**: new subcommand `yaoxiang mcp`, does not change any existing
  behavior of `yaoxiang` / `yaoxiang lsp`
- ✅ **LSP server untouched**: all capabilities, API, and internal state implemented in RFC-017
  remain unchanged
- ✅ **lib crate public API unchanged**: all `pub` paths unchanged; MCP only consumes existing APIs
  — **zero** new `pub` methods

### Integration with existing systems

| Existing module                           | MCP integration approach                                                                           |
| ----------------------------------------- | -------------------------------------------------------------------------------------------------- |
| `src/frontend/lexer`                      | parse_source calls lexer directly                                                                  |
| `src/frontend/core/parser`                | parse_source calls parser directly; on failure, produce `Missing*` nodes (RFC-017)                 |
| `src/frontend/core/typecheck/inference/*` | typecheck reuses the `collect_diagnostics` pattern (RFC-017 §Problem 1)                            |
| `src/middle/`                             | typecheck runs all middle passes (dependency analysis, etc.)                                       |
| `src/lsp/world.rs`                        | At startup, call `World::load_*` API (existing); World **does not** accept any "virtual documents" |
| `src/lsp/handlers/workspace_symbol.rs`    | `mcp/tools/lookup.rs` wraps a layer, converts `query: String` to LSP input (by-name query)         |
| `src/lsp/handlers/references.rs`          | `mcp/tools/find_refs.rs` wraps a layer, converts `query: String` to LSP input                      |
| `src/lsp/handlers/formatter.rs`           | `mcp/tools/format.rs` calls it directly (if not implemented, add `formatter::format_with_diff`)    |
| `src/util/i18n/`                          | Error messages go through the multilingual resource files (zh-CN/en)                               |

### Error handling

| Source                                        | Handling                                                                                        |
| --------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| Parse error                                   | `Diagnostic{code:"E0xxx", severity, message, span}` (**not a tool error**, returned in content) |
| Type error                                    | Same as above                                                                                   |
| `file_paths` out of bounds (`typecheck` tool) | tool-level error `MCP-PATH-OUTSIDE-PROJECT`                                                     |
| `source` is invalid UTF-8                     | tool-level error `MCP-INVALID-INPUT`                                                            |
| Tool panic                                    | JSON-RPC `-32603 Internal error`; server **does not exit**                                      |
| Client sends non-JSON-RPC                     | Disconnect directly (stdio EOF), restart for a new session                                      |

Diagnostic severity follows RFC-017's already-implemented `enum ErrorKind { Error, Warning, Note }`.

### Testing strategy

| Layer           | Test                                                                                                     |
| --------------- | -------------------------------------------------------------------------------------------------------- |
| **Unit**        | `src/mcp/project.rs::resolve` path traversal, `src/mcp/schema.rs` schema validation                      |
| **Integration** | mock stdio: start a server, pour JSON-RPC into stdin, read response from stdout, compare against fixture |
| **E2E**         | run real `yaoxiang mcp` process, Claude Code-style tool call chain: parse → fix → format → typecheck     |
| **Fuzz**        | `cargo-fuzz` for MCP JSON-RPC parsing (libFuzzer harness)                                                |

Each tool must have at least 1 happy path + 1 diagnostic scenario + 1 tool-error scenario
integration test.

## Trade-offs

### Pros

- **Very low reuse cost**: `World` / `Session` / `handlers` / diagnostic collection are all already
  implemented (RFC-017); this RFC is "adding an MCP shell"
- **AI-First interface**: tool contract is 3-5 times more intuitive than LSP; LLM reads the schema
  directly
- **Multi-process isolation**: decoupled from the LSP editor session and other MCP processes, **zero
  lock contention**
- **stdio-friendly**: all mainstream AI agents default to subprocess mode, zero-config integration
- **YAGNI passes**: this RFC cuts Resources, Sessions, cross-process state, remote MCP — reopen in
  v2

### Cons

- **Protocol divergence**: in the future, LSP / MCP / DAP will each evolve independently, with
  maintenance cost for consistency
- **HTTP mode as a second-class citizen**: loopback restriction positions it as a local tool; remote
  scenarios need v2 redesign
- **Duplicate parse overhead**: AI repeatedly fine-tunes source and repeatedly calls `parse_source`,
  which re-lexes and re-parses. **Mitigation**: depending on RFC-017's `DocumentCache` can still
  accelerate re-parsing of the same source **on disk**; pure transient source inevitably parses once
- **Test coverage cost**: 5 tools × 3 scenarios = at least 15 integration tests

## Alternatives

| Plan                                                             | Why not choose                                                                                                              |
| ---------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| **In-process dual protocol embedding** (LSP+MCPlistener coexist) | stdin/stdout has only one consumer; HTTP must also coexist — complexity > benefit                                           |
| **MCP as LSP-client bridge**                                     | One extra IPC layer; LSP was not designed to query symbols by name — the capabilities MCP wants LSP cannot provide          |
| **gRPC / custom protocol**                                       | Deviates from the de facto standard; the community already has MCP SDKs (TypeScript, Python, Rust), with built-in ecosystem |
| **Reuse all LSP handler capabilities** (L3 toolset)              | Lots of position↔intent adaptation work; diminishing marginal returns                                                       |
| **First version only HTTP** (no stdio)                           | Claude Code / Continue default to stdio, entry barrier too high                                                             |

## Implementation Strategy

### Dependencies

- **Strong dependency**: RFC-017 LSP implementation (already implemented)
- **Strong dependency**: RFC-013 error code system (already implemented)
- **Strong dependency**: RFC-014 / RFC-015 project root identification (partially implemented)
- **New dependencies** (Rust crate):
  - `mcp-rust-sdk` (to be evaluated, refer to
    [modelcontextprotocol/rust-sdk](https://github.com/modelcontextprotocol/rust-sdk))
  - `tokio` (**existing**, optional feature)
  - `axum` (HTTP mode) or `hyper` directly — to be evaluated
- **Zero language spec changes**: pure toolchain increment

### Implementation phases

| Phase                       | Content                                                                                                                                                                                                                                | Duration estimate |
| --------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------- |
| **v0.8.x (MVP)**            | `src/mcp/{mod.rs, server.rs, transport/stdio.rs, project.rs, schema.rs}` + `parse_source` + `format_source` + `lookup_symbol` + `find_references` + `typecheck` (**5 tools**) + `yaoxiang mcp` subcommand + `World::load_*` at startup | **3-4 weeks**     |
| **v0.9.x (YaoXiang Smart)** | `+ explain_diagnostic` (**directly call** `render_explain_output`) + `+ list_imports` (wraps `ModuleGraph::validate_imports`) + unit/integration tests                                                                                 | **1-2 weeks**     |
| **v0.10.x (AST + HTTP)**    | `+ rename_symbol` (**new** `src/middle/rename.rs`, AST rewriting) + streamable HTTP transport + performance tuning (parse_source P99 < 100ms)                                                                                          | **2-3 weeks**     |

**Why 3 phases**: MVP first runs stdio + 5 tools to verify the interface design is reasonable;
v0.9.x adds low-risk, zero-adaptation "YaoXiang-specific" tools to verify integration correctness;
v0.10.x then opens the high-risk "AST rewriting" new module (independent PR review is more focused).

### Risks

1. **`mcp-rust-sdk` maintenance activity**: released in 2025, API may change drastically.
   **Mitigation**: if evaluation shows it is not stable, write a lightweight JSON-RPC 2.0 + tool
   dispatcher yourself (< 500 lines)
2. **Duplicate parse overhead**: AI repeatedly fine-tunes source and repeatedly calls
   `parse_source`, which re-lexes and re-parses. **Mitigation**: depending on RFC-017's
   `DocumentCache` can still accelerate re-parsing of the same source **on disk**; pure transient
   source inevitably parses once
3. **AI agent schema compatibility**: different agents have different strictness of MCP schema.
   **Mitigation**: use the `schemars` crate to auto-generate schemas from Rust input structures,
   with zero hand-written drift
4. **Path resolution multi-platform**: Windows path case-insensitivity, UNC paths, `\\` boundaries.
   **Mitigation**: path resolution uses `camino::Utf8Path` instead of `std::path`
5. **MCP tool schema and LSP input are not 1:1**: LSP `workspace_symbol` takes `(query)`; when
   passed to the LSP internals, it needs to be wrapped into position+URI to allow existing handlers
   to be reused. **Mitigation**: do the adaptation layer in `mcp/tools/lookup.rs`, encapsulating the
   details on the MCP side
6. **`rename_symbol` AST rewriting has different semantics from LSP `rename`**: LSP
   `textDocument/rename` is URI + position + new_name → WorkspaceEdit; MCP `rename_symbol` is
   source + old_name + new_name → new source. **Cannot be directly reused**. **Mitigation**:
   implement `src/middle/rename.rs` separately, scope-aware rewriting of references, does not
   interfere with the LSP handler implementation

## Open questions

- [ ] `mcp-rust-sdk` selection / self-implementation? (@Chen Xu: first evaluate the June version of
      rust-sdk, then decide)
- [ ] HTTP authentication path? (reopen in v0.10 RFC)
- [ ] Does MCP need to output `tools/list` at startup for AI to actively discover? (MCP standard
      requirement, **default implementation**)
- [ ] Does `typecheck` support `mode: "fast|full"` (fast = only the current file subset, full =
      entire workspace)?
- [ ] Is the performance budget of parse_source P99 < 100ms realistic? (need to benchmark the actual
      overhead of RFC-017's already-implemented `DocumentCache` in source-string mode)

## References

- [RFC-017: Language Server Protocol (LSP) Support Design](../accepted/017-lsp-support.md)
- [RFC-013: Error Code Specification Design](../accepted/013-error-code-specification.md)
- [RFC-014: Package Management System Design](../accepted/014-package-manager.md)
- [RFC-015: YaoXiang Configuration System Design](../accepted/015-configuration-system.md)
- [MCP Specification](https://modelcontextprotocol.io/)
- [MCP Rust SDK](https://github.com/modelcontextprotocol/rust-sdk)
- [JSON-RPC 2.0](https://www.jsonrpc.org/specification)
- [LSP Specification 3.18](https://github.com/microsoft/language-server-protocol/blob/main/specifications/specification-3-18.md)
- [Rust Analyzer](https://rust-analyzer.github.io/) — M2 / MCP integration reference
- [zed-industries/zed's MCP implementation](https://github.com/zed-industries/zed/tree/main/crates/mcp)

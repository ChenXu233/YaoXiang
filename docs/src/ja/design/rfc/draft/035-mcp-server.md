---
title: 'RFC-035: MCP Server サポート（AI Agent 統合）'
status: 'ドラフト'
author: '晨煦'
created: '2026-07-11'
updated: '2026-07-11'
issue: '#154'
---

# RFC-035: MCP Server サポート（AI Agent 統合）

## 要約

YaoXiang に MCP（Model Context Protocol）サーバーを追加し、AI agent（Claude
Code、Continue、Cody、Zed など）が YaoXiang ソースコードの
**AST、解析エラー、型、シンボル、参照、フォーマット結果**を直接クエリできるようにする。RFC-017 で既に実装されている
`World` バックエンドを再利用し、`yaoxiang mcp`
サブコマンドを追加する。単一バイナリでデュアルモード、プロセスごとに独立した World。

## 動機

### なぜこの機能が必要なのか？

RFC-017 により、YaoXiang はエディタに**理解される**ようになった（hover / goto-def /
completion）。しかし LSP は**位置駆動**のプロトコルである：

- 各リクエストは `textDocument` URI + `Position` に強く依存する
- エディタはまずファイルを開き、保存し、LSP server との長接続を維持する必要がある
- AI
  agent のワークフローは**コードスニペット**：会話の中で「コードを貼って」質問する。**ファイルを保存しない**

AI agent が実際に利用可能な LSP クライアント（vscode-langservers-extracted、`mcp-lsp-bridge`
系プロジェクト）は**L1 のみを翻訳**する：goto-def、hover。AI が行いたいこと：

- 「このコードは**正しく解析されるか**」 — parse + 完全な diagnostic ストリームが必要
- 「このシンボルは**ファイル内でどう使われているか**」 — `lookup_symbol` で名前で検索が必要
- 「このコードを**フォーマットするとどうなるか**」 — `format_source` が必要
- 「**すべての**型エラーの場所」 — `typecheck` でワークスペース全体を走査する必要

これらの L1 LSP 翻訳機能では**不可能**。LSP の設計上サポートされていないからだ。

### 現在の問題点

1. AI agent による LSP 利用体験が悪い：ドキュメントのモックが必要、JSON が巨大、URI への強い依存
2. YaoXiang プロジェクトに「AI-First」インターフェース層がない：人間は IDE で LSP を使うが、AI
   agent は LSP を使えない
3. Claude Code / Continue などの主要な AI
   agent はデフォルトで MCP をサポートしているが、YaoXiang にはこのエコシステムがない

### MCP とは何か？

MCP（Model Context Protocol）は 2024-2025 年に Anthropic 主導で公開・オープンソース化された AI
agent ツール呼び出しプロトコルで、事実上の標準となっている（OpenAI、Google、Microsoft、Zed、Continue、Cody などが採用）。特徴：

- JSON-RPC 2.0 ベース（LSP と同源）
- 3 つのプリミティブ：**Tools**（アクション）、Resources（データ）、Prompts（テンプレート）
- トランスポート：`stdio`（子プロセス）/ streamable `HTTP` / SSE
- ツールの入出力は **JSON Schema** で強く型付けされている（LLM にとって扱いやすい）
- 2025-06+ で streamable HTTP 仕様が公開されており、本 RFC は旧 SSE とも互換

**本 RFC では Tools プリミティブのみを使用**する。LSP の「サービスを提供する」設計と整合し、Resources のファイルモデルによる複雑さを持ち込まない。

## 提案

### 中核となる設計

単一バイナリでデュアルモード：

```text
┌─────────────────────────────────────────────────────────┐
│                    yaoxiang（v0.7.7+）                  │
│  ┌─────────────────┐      ┌──────────────────────────┐  │
│  │ yaoxiang lsp    │      │   yaoxiang mcp           │  │
│  │ (stdio JSON-RPC)│      │   (stdio default         │  │
│  │ RFC-017 実装済  │      │    + HTTP optional)       │  │
│  └────────┬────────┘      └──────────┬───────────────┘  │
│           │                         │                   │
│           ▼                         ▼                   │
│  ┌──────────────────────────────────────────────────┐  │
│  │  共有 lib crate（`yaoxiang`）                      │  │
│  │  src/lsp/{server,session,world}.rs                │  │
│  │  src/frontend/{lexer,parser,core}/...             │  │
│  │  src/middle/...                                   │  │
│  └──────────────────────────────────────────────────┘  │
│                                                          │
│  ┌──────────────────────────────────────────────────┐  │
│  │            src/mcp/  ← 新規                        │  │
│  │  ├── mod.rs          （モジュール入口 + 起動関数）  │  │
│  │  ├── transport/      （stdio + HTTP/SSE）         │  │
│  │  ├── server.rs       （JSON-RPC メッセージループ）  │  │
│  │  ├── tools/          （6 つの tool handler）       │  │
│  │  ├── schema.rs       （入出力 JSON Schema）        │  │
│  │  └── project.rs      （プロジェクトルート検出 + パス解決）  │
│  └──────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

**重要な意思決定**：

- **同一バイナリ**：`yaoxiang`
  はサブコマンドで切り替える；LSP プロセスと MCP プロセスは**同一ランタイムに共存しない**
- **プロセスごとに独立した World**：各 `yaoxiang mcp` プロセスは 1 つの `World`
  を保持する。LSP プロセスや他の MCP プロセスと互いに影響しない（ロック競合なし、独立したクラッシュ分離）
- **stdio がデフォルト**：ポート競合の回避、ネットワーク設定不要。HTTP はオプションの裏口
- **再利用、複製ではなく**：`yaoxiang::frontend` / `yaoxiang::middle` / `yaoxiang::lsp::handlers`
  の lib API を直接呼び出す。**LSP-client 経由はしない**

### ツールセット（8 ツール、3 段階に分けて提供）

「特殊ケースの排除 + 段階分け」原則に従って設計：純粋なソースツール（stateless）を先行し、ワークスペースツールは LSP
World を共有、AST 書き換えツールは独立して追加。

| ツール名             | 入力                                                                                            | 出力                                                         | 再利用                                                                | 段階        |
| -------------------- | ----------------------------------------------------------------------------------------------- | ------------------------------------------------------------ | --------------------------------------------------------------------- | ----------- |
| `parse_source`       | `source: String`, `tab_size?: u32`                                                              | `{ast: Node, diagnostics: Diagnostic[]}`                     | `frontend::parse` を直接呼び出し                                      | v0.8.x      |
| `format_source`      | `source: String`, `tab_size?: u32`                                                              | `{formatted: String, diff: Hunk[]}`                          | `formatter::format` を直接呼び出し                                    | v0.8.x      |
| `lookup_symbol`      | `query: String`, `workspace_root?: String`, `kind?: SymbolKind[]`                               | `{symbols: Symbol[]}`                                        | `lsp::handlers::workspace_symbol` を再利用（`query` で fuzzy 検索）   | v0.8.x      |
| `find_references`    | `query: String`, `workspace_root?: String`                                                      | `{locations: Location[]}`                                    | `lsp::handlers::references` を再利用（位置ではなく `query` で）       | v0.8.x      |
| `typecheck`          | `file_paths: String[]`, `project_root: String`                                                  | `{diagnostics: Diagnostic[], summary: Counts}`               | `lsp::world::typecheck_full` を再利用                                 | v0.8.x      |
| `explain_diagnostic` | `code: String`（例：`E0001`）、`lang?: String`                                                  | `{code, category, title, description, example, help}`        | `util::diagnostic::command::render_explain_output` を**直接呼び出し** | **v0.9.x**  |
| `list_imports`       | `file_path: String`, `project_root?: String`                                                    | `{imports: [{module, items, source_file}]}`                  | `middle::passes::module::ModuleGraph::validate_imports` を再利用      | **v0.9.x**  |
| `rename_symbol`      | `source: String`, `old_name: String`, `new_name: String`, `scope?: "module" \| "function:name"` | `{source: String, edits: Edit[], diagnostics: Diagnostic[]}` | `src/middle/rename.rs` を**新規追加**（AST 書き換え）                 | **v0.10.x** |

**8 ツールの境界**：

- `parse_source` / `format_source` — **純粋なソース stateless**、World に入らない
- `lookup_symbol` / `find_references` — `workspace_root` を取る（指定しない場合は起動時の
  `--project-root` を使用）
- `typecheck` — `file_paths` は**必須**、ワークスペースの完全性を保証
- `explain_diagnostic` — **ファイル依存ゼロ**、純粋にエラーコードレジストリを文字列検索
- `list_imports` — `file_path` は物理ファイル、そのファイルの import 解析結果を出力
- `rename_symbol` — **純粋なソース AST 書き換え**、LSP スタイルの位置クエリは行わない（既存の
  `lsp::handlers::rename` とはセマンティクスが異なる）
- ~~`hover` / `completion` / `signature_help`~~ — **すべて削除**：AI
  agent は「位置に敏感な」セマンティクスを扱わない、`lookup_symbol` による名前検索で代替

**World のロードタイミング**：server 起動時に `--project-root` に従い `yaoxiang.toml` と
`src/**/*.yx` を走査し、LSP-017 で既に実装されている `World::load_*` API を使って `World.documents`
に一括ロードする。**lib API の新規追加は一切行わない**。

### ツール契約

**入力**：JSON Schema で記述し、各フィールドに `description` + `examples`
を付ける（LLM が自動的に理解する）。

**出力**：構造化 JSON、`schemaVersion: "1.0"` フィールドを統一付与：

```jsonc
// 成功レスポンス
{
  "schemaVersion": "1.0",
  "isError": false,
  "content": [
    { "type": "json", "json": { /* ツール固有データ */ } }
  ]
}

// 診断は構造化されて返される（tool エラーとは見なさない）
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

// ツールレベルのエラー（例：parse_source が不正な UTF-8 を受信）
{
  "schemaVersion": "1.0",
  "isError": true,
  "content": [{ "type": "text", "text": "MCP-INVALID-INPUT: source が有効な UTF-8 ではありません" }],
  "errorCode": "MCP-INVALID-INPUT"
}
```

**エラー体系**：

- **診断（diagnostic）**：解析/型エラー、RFC-013（`E0001` など）に従う —
  **tool エラーとは見なさない**
- **ツールレベルエラー**：`MCP-`
  接頭辞を使用（`MCP-INVALID-INPUT`、`MCP-PROJECT-NOT-FOUND`、`MCP-INTERNAL`）— `isError: true`
  として扱う
- **panic/crash**：JSON-RPC `-32603 Internal error`、server は終了しない

**パス解決ルール**（`lookup_symbol` / `find_references` の `workspace_root`、`typecheck` の
`file_paths` に適用）：

1. コマンドライン `--project-root <dir>` が最優先（デフォルトを上書き）
2. それ以外：cwd から `yaoxiang.toml` を上方向に探索、ファイルシステムのルートまで（RFC-015 に従う）
3. それ以外：cwd 自身
4. `file_paths`
   はプロジェクトルート内に存在しなければならない（ディレクトリトラバーサル防止）；逸脱した場合 →
   `MCP-PATH-OUTSIDE-PROJECT`

### トランスポート層

**stdio（デフォルト）**：

```bash
yaoxiang mcp
# 起動後 stdin から JSON-RPC を読み、stdout に書き、stderr をログに使用
```

AI agent 設定（Claude Code `.mcp.json` / Continue `config.json`）：

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

**streamable HTTP（オプション）**：

```bash
yaoxiang mcp --http --addr 127.0.0.1:7325  # 単一 HTTP ポート、新しい MCP 仕様
yaoxiang mcp --http --sse --addr 127.0.0.1:7325  # 旧 SSE 互換（v0.10）
```

**セキュリティ制約**：

- **loopback のみをリッスン**（127.0.0.1 /
  ::1）；パブリックネットワークへのバインドは明示的に拒否しエラーで終了
- HTTP **認証なし**（loopback はデフォルトで信頼）；将来 `--require-token <hex>`
  フィールドを追加予定
- stdio 子プロセスモードは本質的に隔離される（parent プロセスが権限を制御）

### マルチプロセスと並行性

各 `yaoxiang mcp` プロセスは 1 つの `World` を保持し、共有しない：

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

**ポート競合**：AI
agent は「子プロセスを起動」設定 — 本質的にポート競合ゼロ。HTTP モードではポート割り当てをユーザーが管理する必要がある。
**World 分離**：各プロセスは独立した LSP 同期状態を持つ —
1 つの MCP プロセスのクラッシュは LSP や他の MCP プロセスに**影響しない**。
**将来の Sessions**：v2 で複数ワークスペースのディスパッチ（同一プロセス内の複数
`Session`）を検討する。**本 RFC では扱わない**。

## 詳細設計

### データ構造

`src/mcp/project.rs` を新規追加：

```rust
pub struct ProjectRoot {
    /// 絶対パス
    pub root: PathBuf,
    /// ロード時にプロジェクトルートを識別した戦略のソース
    pub source: ProjectRootSource,
}

pub enum ProjectRootSource {
    CliFlag,           // yaoxiang mcp --project-root
    AutoDetected,      // 上方向に yaoxiang.toml を探索
    FallbackCwd,       // cwd にフォールバック
}

pub struct ResolvedPath {
    /// プロジェクトルートからの相対パス（AI 読み取り推奨）
    pub relative: String,
    /// 解決後の絶対パス（World 操作用）
    pub absolute: PathBuf,
}

impl ProjectRoot {
    /// "file_path" を安全なパスに解決する — ディレクトリトラバーサル防止
    pub fn resolve(&self, file_path: &str) -> Result<ResolvedPath, McpError>;
}
```

`ProjectRoot` シングルトン + `src/mcp/schema.rs` のツール schema を自動生成：

```rust
pub struct ProjectRoot {
    /// 絶対パス（`yaoxiang.toml` を含むか、互換のためのフォールバック）
    pub root: PathBuf,
    pub source: ProjectRootSource,
}

impl ProjectRoot {
    /// CLI 起動時に一度だけ識別し、結果を `McpServer` コンテキストにキャッシュ — すべてのツールで再利用
    pub fn detect(cli_override: Option<PathBuf>) -> Result<Self, McpError>;
}
```

ツール schema は `schemars` crate を使って input struct から自動生成し、手書きの JSON
Schema ずれを防ぐ：

```rust
#[derive(Deserialize, schemars::JsonSchema)]
pub struct ParseSourceInput {
    /// 完全な YaoXiang ソースコードスニペット — **ディスクには保存しない**、純粋に transient
    pub source: String,
    pub tab_size: Option<u32>,
}
```

**`parse_source` / `format_source` ツールの schema には `file_path` フィールドがない**
— これらのツールは文字列ソースのみを受け取り、プロジェクトセマンティクスには参加しない。`lookup_symbol`
/ `find_references` / `typecheck` は `workspace_root` または `file_paths`
を受け取る（必須かどうかは上の表を参照）。

### コンパイラの変更

| モジュール                             | 変更                                                                                         |
| -------------------------------------- | -------------------------------------------------------------------------------------------- |
| `src/lsp/world.rs`                     | **変更なし** — MCP 起動時に既存の `World::load_*` API を呼び出してワークスペースを一括ロード |
| `src/lsp/handlers/workspace_symbol.rs` | **変更なし** — `mcp/tools/lookup.rs` がラッパーで `query` を LSP の入力に変換                |
| `src/lsp/handlers/references.rs`       | **変更なし** — 同上                                                                          |
| `src/lsp/handlers/formatter.rs`        | **変更なし** — format_source を直接呼び出し                                                  |
| `src/main.rs`                          | `Mcp` サブコマンド分岐を追加                                                                 |
| `Cargo.toml`                           | `mcp-server` feature を追加（またはメインビナリに常に含める）                                |
| `src/util/diagnostic/`                 | **変更なし**（RFC-017 で実装済み）                                                           |

**重要な制約**：`src/mcp/` から `src/lsp/`
のプライベートシンボルへの**逆方向依存を禁止**。`crate::lsp::`
の公開 API 経由でのみ handlers を呼び出せる。

### 後方互換性

- ✅ **完全に後方互換**：新サブコマンド `yaoxiang mcp`、既存の `yaoxiang` / `yaoxiang lsp`
  の挙動は一切変更しない
- ✅ **LSP server はそのまま**：RFC-017 で実装されたすべての機能、API、内部状態は不変
- ✅ **lib crate の公開 API はそのまま**：すべての `pub` パスは不変；MCP は既存 API を消費するのみ —
  `pub` メソッドの新規追加は**ゼロ**

### 既存システムとの統合

| 既存モジュール                            | MCP 統合方法                                                                                            |
| ----------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| `src/frontend/lexer`                      | parse_source が lexer を直接呼び出し                                                                    |
| `src/frontend/core/parser`                | parse_source が parser を直接呼び出し；失敗時は `Missing*` ノードを生成（RFC-017）                      |
| `src/frontend/core/typecheck/inference/*` | typecheck が `collect_diagnostics` パターンを再利用（RFC-017 §問題1）                                   |
| `src/middle/`                             | typecheck がすべての middle pass（依存解析など）を実行                                                  |
| `src/lsp/world.rs`                        | 起動時に `World::load_*` API を呼び出し（既存）；World はいかなる「仮想ドキュメント」も**受け付けない** |
| `src/lsp/handlers/workspace_symbol.rs`    | `mcp/tools/lookup.rs` がラッパーとなり、`query: String` を LSP の入力に変換（名前検索）                 |
| `src/lsp/handlers/references.rs`          | `mcp/tools/find_refs.rs` がラッパーとなり、`query: String` を LSP の入力に変換                          |
| `src/lsp/handlers/formatter.rs`           | `mcp/tools/format.rs` を直接呼び出し（未実装の場合、新規 `formatter::format_with_diff` を追加）         |
| `src/util/i18n/`                          | エラーメッセージは多言語リソースファイル（zh-CN/en）経由                                                |

### エラーハンドリング

| ソース                                  | 処理                                                                                             |
| --------------------------------------- | ------------------------------------------------------------------------------------------------ |
| 解析エラー                              | `Diagnostic{code:"E0xxx", severity, message, span}`（**tool エラーではない**、content 内で返す） |
| 型エラー                                | 同上                                                                                             |
| `file_paths` 逸脱（`typecheck` ツール） | ツールレベルエラー `MCP-PATH-OUTSIDE-PROJECT`                                                    |
| `source` が不正な UTF-8                 | ツールレベルエラー `MCP-INVALID-INPUT`                                                           |
| ツール panic                            | JSON-RPC `-32603 Internal error`；server は**終了しない**                                        |
| クライアントから非 JSON-RPC             | 即座にストリーム切断（stdio EOF）、再起動で新セッション                                          |

診断の重大度レベルは RFC-017（実装済み）の `enum ErrorKind { Error, Warning, Note }` に従う。

### テスト戦略

| 層              | テスト                                                                                                             |
| --------------- | ------------------------------------------------------------------------------------------------------------------ |
| **Unit**        | `src/mcp/project.rs::resolve` のディレクトリトラバーサル、`src/mcp/schema.rs` の schema 検証                       |
| **Integration** | stdio のモック：server を起動し、stdin に JSON-RPC を流し込み、stdout からレスポンスを読み、fixture と比較         |
| **E2E**         | `yaoxiang mcp` の実プロセスを実行、Claude Code スタイルのツール呼び出しチェーン：parse → 修正 → format → typecheck |
| **Fuzz**        | MCP JSON-RPC 解析の `cargo-fuzz`（libFuzzer harness）                                                              |

各ツールは少なくとも 1 つの happy path + 1 つの diagnostic シナリオ +
1 つの tool-error シナリオの integration テストを持たなければならない。

## トレードオフ

### 利点

- **再利用コストが極めて低い**：`World` / `Session` / `handlers`
  / 診断収集はすべて実装済み（RFC-017）、本 RFC は「MCP の殻を被せるだけ」
- **AI-First インターフェース**：tool 契約は LSP より 3-5 倍直感的。LLM が schema を直接読める
- **マルチプロセス分離**：LSP エディタセッションや他の MCP プロセスと分離、**ロック競合ゼロ**
- **stdio フレンドリ**：すべての主要 AI agent はデフォルトで子プロセスモード、設定不要で統合可能
- **YAGNI の遵守**：本 RFC は Resources、Sessions、クロスプロセス状態、リモート MCP をすべてカット。v2 で改めて検討

### 欠点

- **プロトコル分裂**：将来 LSP / MCP /
  DAP の 3 つのプロトコルがそれぞれ進化し、一貫性維持のコストが発生
- **HTTP モードは二次市民**：loopback 制限によりローカルツールとして位置付けられ、リモートシナリオは v2 で再設計が必要
- **parse の重複コスト**：AI がソースを微調整し `parse_source`
  を繰り返し呼ぶと、毎回 lexer+parser を実行する。**緩和策**：RFC-017 の `DocumentCache`
  による**ディスク上**の同ソースの 2 回目以降の解析は高速化可能。純粋な transient ソースの 1 回限りの解析は不可避
- **テストカバレッジのコスト**：5 ツール × 3 シナリオ = 最低 15 の integration テストが必要

## 代替案

| 案                                                               | 採用しない理由                                                                                                              |
| ---------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| **プロセス内で 2 つのプロトコルを共存**（LSP+MCP Listener 同居） | stdin/stdout は 1 つのコンシューマしか持てない；HTTP も同居させる必要があり、複雑性 > 利益                                  |
| **MCP を LSP-client ブリッジとする**                             | IPC が 1 階層増える；LSP の設計上、名前でのシンボル検索はサポートされていない — MCP が必要とする能力は LSP では提供できない |
| **gRPC / カスタムプロトコルを採用**                              | 事実上の標準から離れる。MCP SDK（TypeScript、Python、Rust）がコミュニティに存在し、エコシステムが整っている                 |
| **LSP handler のすべての能力を再利用**（L3 ツールセット）        | 大量の position↔intent 適応作業が必要；限界効用が逓減する                                                                   |
| **最初のバージョンで HTTP のみ**（stdio なし）                   | Claude Code / Continue などはデフォルトで stdio を使うため、参入障壁が高すぎる                                              |

## 実装戦略

### 依存関係

- **強い依存**：RFC-017 LSP 実装（実装済み）
- **強い依存**：RFC-013 エラーコード体系（実装済み）
- **強い依存**：RFC-014 / RFC-015 プロジェクトルート識別（一部実装済み）
- **新規追加依存**（Rust crate）：
  - `mcp-rust-sdk`（要評価。参考：[modelcontextprotocol/rust-sdk](https://github.com/modelcontextprotocol/rust-sdk)）
  - `tokio`（**既存**、optional feature）
  - `axum`（HTTP モード）または `hyper` 直接利用 — 要評価
- **言語仕様の変更はゼロ**：純粋にツールチェーンの増分

### 実装フェーズ

| フェーズ                               | 内容                                                                                                                                                                                                                                  | 所要期間     |
| -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------ |
| **v0.8.x (MVP)**                       | `src/mcp/{mod.rs, server.rs, transport/stdio.rs, project.rs, schema.rs}` + `parse_source` + `format_source` + `lookup_symbol` + `find_references` + `typecheck`（**5 ツール**）+ `yaoxiang mcp` サブコマンド + 起動時 `World::load_*` | **3-4 週間** |
| **v0.9.x (YaoXiang インテリジェンス)** | `+ explain_diagnostic`（`render_explain_output` を**直接呼び出し**）+ `+ list_imports`（`ModuleGraph::validate_imports` をラップ）+ ユニット/統合テスト                                                                               | **1-2 週間** |
| **v0.10.x (AST + HTTP)**               | `+ rename_symbol`（`src/middle/rename.rs` を**新規追加**、AST 書き換え）+ streamable HTTP トランスポート + パフォーマンスチューニング（parse_source P99 < 100ms）                                                                     | **2-3 週間** |

**なぜ 3 段階に分けるか**：MVP ではまず stdio +
5 ツールでインターフェース設計の妥当性を検証する。v0.9.x ではリスクの低い「YaoXiang 特有」ツールを追加し、統合の正しさを検証する。v0.10.x でリスクの高い「AST 書き換え」新モジュールを追加する（独立した PR レビューのほうが焦点が絞られるため）。

### リスク

1. **`mcp-rust-sdk`
   のメンテナンス活発度**：2025 年に公開されたばかりで、API が大きく変わる可能性がある。**緩和策**：安定性を評価した上で不安なら、軽量な JSON-RPC
   2.0 + tool dispatcher を自前で書く（< 500 行）
2. **parse の重複コスト**：AI がソースを微調整し `parse_source`
   を繰り返し呼ぶと、毎回 lexer+parser を実行する。**緩和策**：RFC-017 の `DocumentCache`
   による**ディスク上**の同ソースの 2 回目以降の解析は高速化可能。純粋な transient ソースの 1 回限りの解析は不可避
3. **AI agent の schema 互換性**：agent によって MCP schema の厳格さが異なる。**緩和策**：`schemars`
   crate を使って Rust の input 構造から schema を自動生成し、手書きによるずれを排除
4. **マルチプラットフォームでのパス解決**：Windows パスは大文字小文字を区別しない、UNC パス、`\\`
   区切り。**緩和策**：パス解決には `std::path` の代わりに `camino::Utf8Path` を使用
5. **MCP ツールの schema と LSP の入力が 1:1 ではない**：LSP `workspace_symbol` は `(query)`
   を受ける；LSP 内部に転送する際、既存 handler を再利用するため位置+URI にラップする必要がある。**緩和策**：`mcp/tools/lookup.rs`
   に適応層を設け、詳細を MCP 側に閉じ込める
6. **`rename_symbol` の AST 書き換えは LSP `rename` とセマンティクスが異なる**：LSP
   `textDocument/rename` は URI + 位置 + new_name → WorkspaceEdit；MCP `rename_symbol` は source +
   old_name + new_name → 新しい source。**直接再利用はできない**。**緩和策**：`src/middle/rename.rs`
   を別途実装し、scope-aware に参照を書き換える。LSP handler の実装とは互いに干渉しない

## 未解決問題

- [ ] `mcp-rust-sdk` を選定するか、自前実装か？（@Chen
      Xu：まず rust-sdk の 6 月版を評価してから決定）
- [ ] HTTP 認証のパス？（v0.10 RFC で改めて検討）
- [ ] 起動時に `tools/list`
      を出力して AI が能動的に発見できるようにすべきか？（MCP 標準で要求、**デフォルトで実装**）
- [ ] `typecheck` は `mode: "fast|full"` をサポートすべきか？（fast
      = 現在のファイルサブセットのみ、full = ワークスペース全体）
- [ ] パフォーマンス予算 parse_source P99 < 100ms は現実的か？（RFC-017 で実装された `DocumentCache`
      の source-string モードでの実コストをベンチマークで確認する必要あり）

## 参考文献

- [RFC-017: 言語サーバープロトコル（LSP）サポート設計](../accepted/017-lsp-support.md)
- [RFC-013: エラーコード仕様設計](../accepted/013-error-code-specification.md)
- [RFC-014: パッケージ管理システム設計](../accepted/014-package-manager.md)
- [RFC-015: YaoXiang 設定システム設計](../accepted/015-configuration-system.md)
- [MCP 仕様](https://modelcontextprotocol.io/)
- [MCP Rust SDK](https://github.com/modelcontextprotocol/rust-sdk)
- [JSON-RPC 2.0](https://www.jsonrpc.org/specification)
- [LSP 仕様 3.18](https://github.com/microsoft/language-server-protocol/blob/main/specifications/specification-3-18.md)
- [Rust Analyzer](https://rust-analyzer.github.io/) — M2 / MCP 統合の参考
- [zed-industries/zed の MCP 実装](https://github.com/zed-industries/zed/tree/main/crates/mcp)

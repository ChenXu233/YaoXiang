---
title: 'RFC-035: MCP Server サポート（AI Agent 統合）'
status: 'ドラフト'
author: '晨煦'
created: '2026-07-11'
updated: '2026-07-11'
issue: '#154'
---

# RFC-035: MCP Server サポート（AI Agent 統合）

## 概要

YaoXiang に MCP（Model Context Protocol）サーバーを追加し、AI agent（Claude
Code、Continue、Cody、Zed など）が YaoXiang ソースコードの
**AST、解析エラー、型、シンボル、参照、フォーマット結果**
を直接クエリできるようにする。RFC-017 で実装済みの `World` バックエンドを再利用し、`yaoxiang mcp`
サブコマンドを追加する。単一バイナリでデュアルモード、プロセスが独立した World を保持する。

## 動機

### なぜこの機能が必要なのか？

RFC-017 によって YaoXiang はエディタから**理解される**ようになった（hover / goto-def /
completion）。しかし LSP は**位置駆動**のプロトコルである：

- 各リクエストは `textDocument` URI + `Position` に強く依存する
- エディタは先にファイルを開き、保存し、LSP サーバーとの長寿命接続を維持する必要がある
- AI
  agent のワークフローは**コードスニペット**：対話の中で「コードを貼って」質問する。**保存しない**

AI agent が実際に利用可能な LSP クライアント（vscode-langservers-extracted、`mcp-lsp-bridge`
系プロジェクト）はすべて **L1 のみを変換**する：goto-def、hover。AI が行いたいこと：

- 「このコードは**正しく解析されるか**」— parse + 完全な diagnostic ストリームが必要
- 「このシンボルは**ファイル内でどう使われているか**」— `lookup_symbol` で名前による検索が必要
- 「このコードを**フォーマットするとどうなるか**」— `format_source` が必要
- 「**すべての**型エラーの場所」— `typecheck` でワークスペース全体を走査する必要がある

これらの L1 LSP 変換機能では**実現できない**。LSP の設計上サポートされていないからだ。

### 現在の問題

1. AI agent の LSP 呼び出し体験が悪い：ドキュメントのモックが必要、JSON が巨大、URI への強い依存
2. YaoXiang プロジェクトに「AI-First」インターフェース層がない：人間は IDE で LSP を使うが、AI
   agent は LSP を使えない
3. Claude Code / Continue などの主要な AI
   agent は MCP をデフォルトサポートしているが、YaoXiang にとっては未開拓のエコシステム

### MCP とは？

MCP（Model Context
Protocol）は 2024-2025 年に Anthropic が主導してリリースし、オープンソース化した AI
agent のツール呼び出しプロトコルで、事実上の標準となっている（OpenAI、Google、Microsoft、Zed、Continue、Cody などが対応）。特徴：

- JSON-RPC 2.0 ベース（LSP と同源）
- 3 大プリミティブ：**Tools**（アクション）、Resources（データ）、Prompts（テンプレート）
- トランスポート：`stdio`（サブプロセス）/ streamable `HTTP` / SSE
- ツールの入出力に **JSON Schema** による強い型付け（LLM フレンドリー）
- 2025-06+ で streamable HTTP 仕様がリリース済み、本 RFC は旧 SSE とも互換

**本 RFC では Tools プリミティブのみを使用する** —
LSP の「サービスを提供する」設計と整合し、Resources のファイルモデルの複雑さを導入しない。

## 提案

### 中核設計

単一バイナリでデュアルモード：

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
│  │  共有 lib crate（`yaoxiang`）                      │  │
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

**重要な意思決定**：

- **同一バイナリ**：`yaoxiang`
  はサブコマンドで切り替える；LSP プロセスと MCP プロセスは**同一ランタイム内で共存しない**
- **マルチプロセス独立 World**：各 `yaoxiang mcp` プロセスは 1 つの `World`
  を保持する；LSP プロセス、他の MCP プロセスと相互に影響しない（ロック競合なし、独立したクラッシュ分離）
- **stdio をデフォルトに**：ポート競合を回避、ゼロネットワーク設定；HTTP は代替手段としてオプション
- **再利用であって重複ではない**：`yaoxiang::frontend` / `yaoxiang::middle` /
  `yaoxiang::lsp::handlers` の lib API を直接呼び出し、LSP-client 経由は**行わない**

### ツールセット（8 ツール、3 段階で提供）

「特殊ケースの排除 + 段階的」という原則で設計：純粋なソースツール（stateless）を先行、ワークスペースツールは LSP
World を共有、AST 書き換えツールは独立して追加。

| ツール名             | 入力                                                                                            | 出力                                                         | 再利用                                                                 | 段階        |
| -------------------- | ----------------------------------------------------------------------------------------------- | ------------------------------------------------------------ | ---------------------------------------------------------------------- | ----------- |
| `parse_source`       | `source: String`, `tab_size?: u32`                                                              | `{ast: Node, diagnostics: Diagnostic[]}`                     | `frontend::parse` を直接呼び出し                                       | v0.8.x      |
| `format_source`      | `source: String`, `tab_size?: u32`                                                              | `{formatted: String, diff: Hunk[]}`                          | `formatter::format` を直接呼び出し                                     | v0.8.x      |
| `lookup_symbol`      | `query: String`, `workspace_root?: String`, `kind?: SymbolKind[]`                               | `{symbols: Symbol[]}`                                        | `lsp::handlers::workspace_symbol` を再利用（`query` でファジーマッチ） | v0.8.x      |
| `find_references`    | `query: String`, `workspace_root?: String`                                                      | `{locations: Location[]}`                                    | `lsp::handlers::references` を再利用（位置ではなく `query` で）        | v0.8.x      |
| `typecheck`          | `file_paths: String[]`, `project_root: String`                                                  | `{diagnostics: Diagnostic[], summary: Counts}`               | `lsp::world::typecheck_full` を再利用                                  | v0.8.x      |
| `explain_diagnostic` | `code: String`（例：`E0001`）, `lang?: String`                                                  | `{code, category, title, description, example, help}`        | **`util::diagnostic::command::render_explain_output` を直接呼び出し**  | **v0.9.x**  |
| `list_imports`       | `file_path: String`, `project_root?: String`                                                    | `{imports: [{module, items, source_file}]}`                  | `middle::passes::module::ModuleGraph::validate_imports` を再利用       | **v0.9.x**  |
| `rename_symbol`      | `source: String`, `old_name: String`, `new_name: String`, `scope?: "module" \| "function:name"` | `{source: String, edits: Edit[], diagnostics: Diagnostic[]}` | **新規追加** `src/middle/rename.rs`（AST 書き換え）                    | **v0.10.x** |

**8 ツールの境界**：

- `parse_source` / `format_source` — **純粋なソース stateless**、World に入らない
- `lookup_symbol` / `find_references` — `workspace_root` を受け取る（未指定なら起動時の
  `--project-root` を使用）
- `typecheck` — `file_paths` **必須**、ワークスペースの完全性を保証
- `explain_diagnostic` — **ファイル依存ゼロ**、エラーコードレジストリへの純粋な文字列クエリ
- `list_imports` — `file_path` は物理ファイル、そのファイルの import 解析結果を出力
- `rename_symbol` — **純粋なソース AST 書き換え**、LSP スタイルの位置クエリは行わない（既存の
  `lsp::handlers::rename` とはセマンティクスが異なる）
- ~~`hover` / `completion` / `signature_help`~~ — **すべて削除**：AI
  agent は「位置に敏感な」セマンティクスを扱わない、`lookup_symbol` による名前検索で代替

**World のロードタイミング**：サーバー起動時に `--project-root` に基づいて `yaoxiang.toml` と
`src/**/*.yx` をスキャンし、LSP-017 で実装済みの `World::load_*` API を再利用して `World.documents`
に一括ロードする。**新規の lib API は一切追加しない**。

### ツール契約

**入力**：JSON Schema で記述、各フィールドに `description` + `examples`
を持たせる（LLM が自動理解できる）。

**出力**：構造化 JSON、統一して `schemaVersion: "1.0"` フィールドを含む：

```jsonc
// 成功レスポンス
{
  "schemaVersion": "1.0",
  "isError": false,
  "content": [
    { "type": "json", "json": { /* ツール固有データ */ } }
  ]
}

// 診断を構造化して返す（tool エラーとは見なさない）
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
  "content": [{ "type": "text", "text": "MCP-INVALID-INPUT: source は正当な UTF-8 ではない" }],
  "errorCode": "MCP-INVALID-INPUT"
}
```

**エラー体系**：

- **診断（diagnostic）**：解析 / 型エラー、RFC-013（`E0001` など）に従う —
  **tool エラーとは見なさない**
- **ツールレベルエラー**：`MCP-`
  プレフィックス（`MCP-INVALID-INPUT`、`MCP-PROJECT-NOT-FOUND`、`MCP-INTERNAL`）— `isError: true`
  として扱う
- **panic / クラッシュ**：JSON-RPC `-32603 Internal error`、サーバーは終了しない

**パス解決ルール**（`lookup_symbol` / `find_references` の `workspace_root`、`typecheck` の
`file_paths` に適用）：

1. コマンドライン `--project-root <dir>` が最優先（デフォルトを上書き）
2. それ以外：cwd から上方向に `yaoxiang.toml` を探索、ファイルシステムのルートまで（RFC-015 に従う）
3. それ以外：cwd 自体
4. `file_paths` はプロジェクトルート内に収まる必要がある（パストラバーサル防止）；越境 →
   `MCP-PATH-OUTSIDE-PROJECT`

### トランスポート層

**stdio（デフォルト）**：

```bash
yaoxiang mcp
# 起動後 stdin から JSON-RPC を読み、stdout に書き、stderr はログ用
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
yaoxiang mcp --http --sse --addr 127.0.0.1:7325  # 旧 SSE と互換（v0.10）
```

**セキュリティ制約**：

- **loopback のみで待ち受け**（127.0.0.1 / ::1）；パブリックバインドは明示的に拒否しエラーで終了
- HTTP **認証なし**（loopback はデフォルトで信頼）；将来 `--require-token <hex>`
  フィールドを追加予定
- stdio サブプロセスモードは本質的に分離される（parent プロセスが権限を制御）

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
agent は「サブプロセスを起動」する設定 — 本質的にゼロポート競合。HTTP モードはユーザーがポート割り当てを管理する必要がある。
**World 分離**：各プロセスが独立した LSP 同期状態を保持 — 1 つの MCP プロセスのクラッシュは **LSP
/ 他の MCP プロセスに影響しない**。**将来の Sessions**：v2 で複数ワークスペースのディスパッチ（同一プロセス内の複数
`Session`）を検討する、**本 RFC では扱わない**。

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
    /// プロジェクトルートからの相対パス（AI に読ませる推奨形式）
    pub relative: String,
    /// 解決後の絶対パス（World 操作用）
    pub absolute: PathBuf,
}

impl ProjectRoot {
    /// 「file_path」を安全なパスに解決する — パストラバーサル防止
    pub fn resolve(&self, file_path: &str) -> Result<ResolvedPath, McpError>;
}
```

`ProjectRoot` シングルトン + `src/mcp/schema.rs` でツールスキーマ自動生成：

```rust
pub struct ProjectRoot {
    /// 絶対パス（`yaoxiang.toml` を含むか、互換フォールバック）
    pub root: PathBuf,
    pub source: ProjectRootSource,
}

impl ProjectRoot {
    /// CLI 起動時に 1 回識別し、結果を `McpServer` コンテキストにキャッシュ — 全ツールで再利用
    pub fn detect(cli_override: Option<PathBuf>) -> Result<Self, McpError>;
}
```

ツールスキーマは `schemars` crate で input struct から自動生成し、手書き JSON
Schema のドリフトを回避する：

```rust
#[derive(Deserialize, schemars::JsonSchema)]
pub struct ParseSourceInput {
    /// 完全な YaoXiang ソースコードスニペット — ディスクには**保存しない**、純粋に transient
    pub source: String,
    pub tab_size: Option<u32>,
}
```

**`parse_source` / `format_source` のツールスキーマには `file_path` フィールドがない**
— これら 2 つのツールはソース文字列のみを受け取り、プロジェクトセマンティクスには参加しない。`lookup_symbol`
/ `find_references` / `typecheck` は `workspace_root` または `file_paths`
を受け取る（必須かどうかはツール表参照）。

### コンパイラの変更

| モジュール                             | 変更                                                                                          |
| -------------------------------------- | --------------------------------------------------------------------------------------------- |
| `src/lsp/world.rs`                     | **変更なし** — MCP 起動時に LSP 既存の `World::load_*` API を呼び出してワークスペースをロード |
| `src/lsp/handlers/workspace_symbol.rs` | **変更なし** — `mcp/tools/lookup.rs` でラップして `query` を LSP 入力パラメータに変換         |
| `src/lsp/handlers/references.rs`       | **変更なし** — 同上                                                                           |
| `src/lsp/handlers/formatter.rs`        | **変更なし** — `format_source` から直接呼び出し                                               |
| `src/main.rs`                          | `Mcp` サブコマンド分岐を追加                                                                  |
| `Cargo.toml`                           | `mcp-server` feature を追加（またはメインビナリに常時含める）                                 |
| `src/util/diagnostic/`                 | **変更なし**（RFC-017 で実装済み）                                                            |

**重要な制約**：`src/mcp/` から `src/lsp/` のプライベートシンボルへの**逆方向依存は禁止** —
`crate::lsp::` の公開 API 経由でのみ handlers を呼び出せる。

### 後方互換性

- ✅ **完全に後方互換**：新規サブコマンド `yaoxiang mcp`、既存の `yaoxiang` / `yaoxiang lsp`
  の動作は一切変更しない
- ✅ **LSP サーバーは不変**：RFC-017 で実装されたすべての機能、API、内部状態を変更しない
- ✅ **lib crate の公開 API は不変**：すべての `pub`
  パスは変わらない；MCP は既存 API のみを消費する — 新規 `pub` メソッドは**ゼロ**

### 既存システムとの統合

| 既存モジュール                            | MCP 統合方法                                                                                            |
| ----------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| `src/frontend/lexer`                      | `parse_source` から lexer を直接呼び出し                                                                |
| `src/frontend/core/parser`                | `parse_source` から parser を直接呼び出し；失敗時は `Missing*` ノードを生成（RFC-017）                  |
| `src/frontend/core/typecheck/inference/*` | `typecheck` で `collect_diagnostics` パターンを再利用（RFC-017 §問題 1）                                |
| `src/middle/`                             | `typecheck` で全 middle pass（依存解析など）を実行                                                      |
| `src/lsp/world.rs`                        | 起動時に `World::load_*` API を呼び出し（既存）；World はいかなる「仮想ドキュメント」も**受け付けない** |
| `src/lsp/handlers/workspace_symbol.rs`    | `mcp/tools/lookup.rs` でラップし、`query: String` を LSP 入力パラメータに変換（名前検索）               |
| `src/lsp/handlers/references.rs`          | `mcp/tools/find_refs.rs` でラップし、`query: String` を LSP 入力パラメータに変換                        |
| `src/lsp/handlers/formatter.rs`           | `mcp/tools/format.rs` から直接呼び出し（未実装なら `formatter::format_with_diff` を新規追加）           |
| `src/util/i18n/`                          | エラーメッセージは多言語リソースファイル経由（zh-CN / en）                                              |

### エラーハンドリング

| ソース                                    | 処理                                                                                             |
| ----------------------------------------- | ------------------------------------------------------------------------------------------------ |
| 解析エラー                                | `Diagnostic{code:"E0xxx", severity, message, span}`（**tool エラーではない**、content 内で返す） |
| 型エラー                                  | 同上                                                                                             |
| `file_paths` の越境（`typecheck` ツール） | tool レベルエラー `MCP-PATH-OUTSIDE-PROJECT`                                                     |
| `source` が不正な UTF-8                   | tool レベルエラー `MCP-INVALID-INPUT`                                                            |
| ツールの panic                            | JSON-RPC `-32603 Internal error`；サーバーは**終了しない**                                       |
| クライアントが非 JSON-RPC を送信          | 即座に切断（stdio EOF）、再起動で新規セッション                                                  |

診断の重大度レベルは RFC-017（実装済み）の `enum ErrorKind { Error, Warning, Note }` に従う。

### テスト戦略

| 層              | テスト                                                                                                            |
| --------------- | ----------------------------------------------------------------------------------------------------------------- |
| **Unit**        | `src/mcp/project.rs::resolve` のパストラバーサル、`src/mcp/schema.rs` のスキーマ検証                              |
| **Integration** | mock stdio：サーバーを起動、stdin に JSON-RPC を流し込み、stdout から応答を読み、fixture と比較                   |
| **E2E**         | 実プロセス `yaoxiang mcp` を実行、Claude Code スタイルのツール呼び出しチェーン：parse → 修正 → format → typecheck |
| **Fuzz**        | MCP JSON-RPC パーサの `cargo-fuzz`（libFuzzer ハーネス）                                                          |

各ツールは少なくとも 1 つの happy path + 1 つの diagnostic シナリオ +
1 つの tool-error シナリオの integration テストを持つこと。

## トレードオフ

### 利点

- **再利用コストが極めて低い**：`World` / `Session` / `handlers`
  / 診断収集はすべて実装済み（RFC-017）、本 RFC は「MCP シェルを 1 層追加するだけ」
- **AI-First インターフェース**：ツール契約は LSP より 3-5 倍直感的；LLM がスキーマを直接読める
- **マルチプロセス分離**：LSP エディタセッションや他の MCP プロセスと分離、**ロック競合ゼロ**
- **stdio フレンドリー**：すべての主要 AI agent がデフォルトでサブプロセスモード、ゼロ設定で接続可能
- **YAGNI 準拠**：本 RFC は Resources、Sessions、クロスプロセス状態、リモート MCP を削減 —
  v2 で再検討

### 欠点

- **プロトコル分裂**：将来 LSP / MCP / DAP の 3 つのプロトコルがそれぞれ進化し、一貫性維持のコスト
- **HTTP モードは第二級市民**：loopback 制限によりローカルツールと位置付け、リモートシナリオは v2 で再設計
- **重複 parse のオーバーヘッド**：AI がソースコードを繰り返し微調整し `parse_source`
  を呼び出すたびに lexer + parser を再実行。**緩和策**：RFC-017 の `DocumentCache`
  が**ディスク上**の同一ソースの 2 回目以降の解析を高速化できる；純粋に transient なソースの 1 回の解析は不可避
- **テストカバレッジコスト**：5 ツール × 3 シナリオ = 最低 15 の integration テスト

## 代替案

| 案                                                                  | 不採用の理由                                                                                                                       |
| ------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| **プロセス内デュアルプロトコル埋め込み**（LSP + MCP listener 共存） | stdin/stdout は 1 つの消費者しか持てない；HTTP も同居させる必要があり、複雑度 > 利益                                               |
| **MCP を LSP-client ブリッジとして実装**                            | IPC レイヤーが増える；LSP はそもそも名前によるシンボル検索を設計上サポートしていない — MCP が必要とする能力は LSP では提供できない |
| **gRPC / カスタムプロトコルを採用**                                 | 事実上の標準から離れる；コミュニティにはすでに MCP SDK（TypeScript、Python、Rust）があり、エコシステムを自前で持つ                 |
| **LSP handler の全能力を再利用**（L3 ツールセット）                 | 大量の位置↔意図の適応作業が必要；限界効用が逓減                                                                                    |
| **最初のバージョンで HTTP のみ**（stdio なし）                      | Claude Code / Continue などはデフォルトで stdio、敷居が高すぎる                                                                    |

## 実装戦略

### 依存関係

- **強い依存**：RFC-017 LSP 実装（実装済み）
- **強い依存**：RFC-013 エラーコード体系（実装済み）
- **強い依存**：RFC-014 / RFC-015 プロジェクトルート識別（一部実装済み）
- **新規依存**（Rust crate）：
  - `mcp-rust-sdk`（要評価、[modelcontextprotocol/rust-sdk](https://github.com/modelcontextprotocol/rust-sdk)
    を参照）
  - `tokio`（**既存**、optional feature）
  - `axum`（HTTP モード）または `hyper` を直接 — 要評価
- **言語仕様の変更ゼロ**：純粋にツールチェーンの増分

### 実装フェーズ

| フェーズ                               | 内容                                                                                                                                                                                                                                  | 期間見積もり |
| -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------ |
| **v0.8.x (MVP)**                       | `src/mcp/{mod.rs, server.rs, transport/stdio.rs, project.rs, schema.rs}` + `parse_source` + `format_source` + `lookup_symbol` + `find_references` + `typecheck`（**5 ツール**）+ `yaoxiang mcp` サブコマンド + 起動時 `World::load_*` | **3-4 週間** |
| **v0.9.x (YaoXiang インテリジェンス)** | `+ explain_diagnostic`（`render_explain_output` を**直接呼び出し**）+ `+ list_imports`（`ModuleGraph::validate_imports` をラップ）+ ユニット / 統合テスト                                                                             | **1-2 週間** |
| **v0.10.x (AST + HTTP)**               | `+ rename_symbol`（**新規追加** `src/middle/rename.rs`、AST 書き換え）+ streamable HTTP トランスポート + パフォーマンスチューニング（`parse_source` P99 < 100ms）                                                                     | **2-3 週間** |

**なぜ 3 段階に分けるか**：MVP でまず stdio +
5 ツールを動作させ、インターフェース設計の妥当性を検証する；v0.9.x で低リスク・適応不要の「YaoXiang 固有」ツールを追加し、統合の正確性を検証する；v0.10.x でハイリスクな「AST 書き換え」新モジュールを開く（独立した PR レビューで集中できる）。

### リスク

1. **`mcp-rust-sdk`
   のメンテナンス活発度**：2025 年にリリースされたばかりで、API が激変する可能性がある。**緩和策**：安定性を評価した上で問題あれば、軽量な JSON-RPC
   2.0 + tool ディスパッチャを自前実装（< 500 行）
2. **重複 parse のオーバーヘッド**：AI がソースコードを繰り返し微調整し `parse_source`
   を呼び出すたびに lexer + parser を再実行。**緩和策**：RFC-017 の `DocumentCache`
   が**ディスク上**の同一ソースの 2 回目以降の解析を高速化できる；純粋に transient なソースの 1 回の解析は不可避
3. **AI
   agent のスキーマ互換性**：agent によって MCP スキーマの厳密度が異なる。**緩和策**：`schemars`
   crate で Rust input 構造体から自動生成し、手書きによるドリフトを排除
4. **パス解決のマルチプラットフォーム対応**：Windows のパス大文字小文字無視、UNC パス、`\\`
   の境界。**緩和策**：パス解決に `camino::Utf8Path` を `std::path` の代わりに使用
5. **MCP ツールスキーマと LSP 入力パラメータが 1:1 でない**：LSP `workspace_symbol` は `(query)`
   を受ける；既存 handler を再利用するには位置 +
   URI にラップする必要がある。**緩和策**：`mcp/tools/lookup.rs`
   にアダプタ層を置き、詳細を MCP 側に閉じ込める
6. **`rename_symbol` AST 書き換えと LSP `rename` のセマンティクスが異なる**：LSP
   `textDocument/rename` は URI + 位置 + new_name → WorkspaceEdit；MCP `rename_symbol` は source +
   old_name + new_name → 新しい source。**直接再利用はできない**。**緩和策**：`src/middle/rename.rs`
   を別途実装、scope 対応の参照書き換え、LSP handler 実装と相互干渉しない

## 未解決の問題

- [ ] `mcp-rust-sdk` を採用するか / 自前実装か？（@Chen
      Xu：まず 6 月バージョンの rust-sdk を評価してから決定）
- [ ] HTTP 認証パス？（v0.10 の RFC で再検討）
- [ ] 起動時に `tools/list`
      を出力して AI に能動的に発見させる必要があるか？（MCP 標準で要求、**デフォルトで実装**）
- [ ] `typecheck` に `mode: "fast|full"`（fast = 現在のファイル部分集合のみ、full
      = ワークスペース全体）をサポートするか？
- [ ] パフォーマンス予算 `parse_source` P99 < 100ms は現実的か？（RFC-017 で実装済みの
      `DocumentCache` を source-string モードで実測し benchmark が必要）

## 参考文献

- [RFC-017: 言語サーバープロトコル（LSP）サポート設計](./accepted/017-lsp-support.md)
- [RFC-013: エラーコード仕様設計](./accepted/013-error-code-specification.md)
- [RFC-014: パッケージ管理システム設計](./accepted/014-package-manager.md)
- [RFC-015: YaoXiang 設定システム設計](./accepted/015-configuration-system.md)
- [MCP 仕様](https://modelcontextprotocol.io/)
- [MCP Rust SDK](https://github.com/modelcontextprotocol/rust-sdk)
- [JSON-RPC 2.0](https://www.jsonrpc.org/specification)
- [LSP 仕様 3.18](https://github.com/microsoft/language-server-protocol/blob/main/specifications/specification-3-18.md)
- [Rust Analyzer](https://rust-analyzer.github.io/) — M2 / MCP 統合の参考
- [zed-industries/zed の MCP 実装](https://github.com/zed-industries/zed/tree/main/crates/mcp)

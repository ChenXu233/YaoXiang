---
title: "RFC-035: MCP Server サポート（AI Agent 統合）"
status: "ドラフト"
author: "晨煦"
created: "2026-07-11"
updated: "2026-07-11"
issue: "#154"
---

# RFC-035: MCP Server サポート（AI Agent 統合）

## 要約

YaoXiang に MCP（Model Context Protocol）サーバーを追加し、AI agent（Claude Code、Continue、Cody、Zed など）が YaoXiang ソースコードの **AST、解析エラー、型、シンボル、参照、フォーマット結果** を直接クエリできるようにする。RFC-017 で既に実装済みの `World` バックエンドを再利用し、`yaoxiang mcp` サブコマンドを追加する。単一バイナリで二モード動作し、マルチプロセスで独立した World を持つ。

## 動機

### なぜこの機能が必要なのか？

RFC-017 により、YaoXiang はエディタから**理解される**ようになった（hover / goto-def / completion）。しかし LSP は**位置駆動**のプロトコルである：
- 各リクエストは `textDocument` URI + `Position` に強く依存する
- エディタはまずファイルを開き、保存し、LSP server との長接続を維持する必要がある
- AI agent のワークフローは**コードスニペット**である：対話の中で「コードを貼って」質問し、**保存しない**

AI agent が実際に使える LSP クライアント（vscode-langservers-extracted、`mcp-lsp-bridge` 系のプロジェクト）はすべて **L1 のみを翻訳する**：goto-def、hover。AI が行いたいことは：
- 「このコードは**正しく解析されるか**」——parse + 完全な diagnostic ストリームが必要
- 「このシンボルは**ファイル内でどう使われているか**」——`lookup_symbol` で名前による検索が必要
- 「このコードを**フォーマットするとどうなるか**」——`format_source` が必要
- 「**すべての**型エラーの場所」——`typecheck` でワークスペース全体を実行する必要がある

これらの L1 LSP 翻訳能力では**不可能**である。なぜなら LSP は設計上これらをサポートしていないからである。

### 現在の問題

1. AI agent が LSP を呼び出す体験が悪い：モックドキュメントが必要、JSON が巨大、URI への強い依存
2. YaoXiang プロジェクトに「AI-First」インターフェース層が欠けている：人間は IDE を開いて LSP を使うが、AI agent は LSP を使えない
3. Claude Code / Continue などの主要な AI agent はすでに MCP をデフォルトサポートしているが、YaoXiang にとってこのエコシステムは空白である

### MCP とは何か？

MCP（Model Context Protocol）は 2024-2025 年に Anthropic が主導してリリースし、オープンソース化した AI agent ツール呼び出しプロトコルであり、デファクト標準となっている（OpenAI、Google、Microsoft、Zed、Continue、Cody などが採用）。特徴：
- JSON-RPC 2.0 ベース（LSP と同源）
- 三大プリミティブ：**Tools**（アクション）、Resources（データ）、Prompts（テンプレート）
- トランスポート：`stdio`（子プロセス）/ streamable `HTTP` / SSE
- ツールの入出力は **JSON Schema** で強く型付けされる（LLM にとって親和性が高い）
- 2025-06+ で streamable HTTP 仕様がリリースされており、本 RFC は旧 SSE とも互換性を持つ

**本 RFC では Tools プリミティブのみを使用する**——LSP の「サービスを提供する」アラインメントに沿い、Resources のファイルモデルの複雑さを導入しない。

## 提案

### コア設計

単一バイナリで二モード動作：

```text
┌─────────────────────────────────────────────────────────┐
│                    yaoxiang（v0.7.7+）                  │
│  ┌─────────────────┐      ┌──────────────────────────┐  │
│  │ yaoxiang lsp    │      │   yaoxiang mcp           │  │
│  │ (stdio JSON-RPC)│      │   (stdio default         │  │
│  │ RFC-017 実装済み │      │    + HTTP オプション)    │  │
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
│  │            src/mcp/  ← 新規                       │  │
│  │  ├── mod.rs          （モジュール入口 + 起動関数）│  │
│  │  ├── transport/      （stdio + HTTP/SSE）         │  │
│  │  ├── server.rs       （JSON-RPC メッセージループ）│  │
│  │  ├── tools/          （6 個の tool handler）      │  │
│  │  ├── schema.rs       （入出力 JSON Schema）       │  │
│  │  └── project.rs      （プロジェクトルート識別 + パス解決）│  │
│  └──────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

**重要な決定**：
- **同一バイナリ**：`yaoxiang` はサブコマンドで切り替える；LSP プロセスと MCP プロセスは**同一ランタイムには共存しない**
- **マルチプロセス独立 World**：各 `yaoxiang mcp` プロセスは 1 つの `World` を保持する；LSP プロセスや他の MCP プロセスと相互に影響しない（ロック競合なし、独立したクラッシュ分離）
- **stdio がデフォルト**：ポート競合を回避し、ネットワーク設定が不要；HTTP はオプションの代替手段
- **再利用、複製なし**：`yaoxiang::frontend` / `yaoxiang::middle` / `yaoxiang::lsp::handlers` の lib API を直接呼び出し、**LSP-client 中継は経由しない**

### ツールセット（8 個のツール、3 段階で提供）

「特殊ケースの排除 + 段階的提供」の原則で設計：純粋なソースツールの stateless 化を先行し、ワークスペースツールは LSP World を共有し、AST 書き換えツールは独立して追加する。

| ツール名 | 入力 | 出力 | 再利用 | 段階 |
|---|---|---|---|---|
| `parse_source` | `source: String`, `tab_size?: u32` | `{ast: Node, diagnostics: Diagnostic[]}` | `frontend::parse` を直接呼び出し | v0.8.x |
| `format_source` | `source: String`, `tab_size?: u32` | `{formatted: String, diff: Hunk[]}` | `formatter::format` を直接呼び出し | v0.8.x |
| `lookup_symbol` | `query: String`, `workspace_root?: String`, `kind?: SymbolKind[]` | `{symbols: Symbol[]}` | `lsp::handlers::workspace_symbol` を再利用（`query` で曖昧一致） | v0.8.x |
| `find_references` | `query: String`, `workspace_root?: String` | `{locations: Location[]}` | `lsp::handlers::references` を再利用（位置ではなく `query` による） | v0.8.x |
| `typecheck` | `file_paths: String[]`, `project_root: String` | `{diagnostics: Diagnostic[], summary: Counts}` | `lsp::world::typecheck_full` を再利用 | v0.8.x |
| `explain_diagnostic` | `code: String`（例：`E0001`）、`lang?: String` | `{code, category, title, description, example, help}` | `util::diagnostic::command::render_explain_output` を**直接呼び出し** | **v0.9.x** |
| `list_imports` | `file_path: String`, `project_root?: String` | `{imports: [{module, items, is_public}]}` | `middle::passes::module::ModuleGraph::validate_imports` を再利用 | **v0.9.x** |
| `rename_symbol` | `source: String`, `old_name: String`, `new_name: String`, `scope?: "module" \| "function:name"` | `{source: String, edits: Edit[], diagnostics: Diagnostic[]}` | `src/middle/rename.rs` を**新規追加**（AST 書き換え） | **v0.10.x** |

**8 個のツールの境界**：
- `parse_source` / `format_source` —— **純粋なソース stateless**、World に参加しない
- `lookup_symbol` / `find_references` —— `workspace_root` を取る（指定されない場合は起動時の `--project-root` を使用）
- `typecheck` —— `file_paths` が**必須**、ワークスペースの完全性を保証
- `explain_diagnostic` —— **ファイル依存ゼロ**、エラーコード登録表への純粋な文字列クエリ
- `list_imports` —— `file_path` は物理ファイル、そのファイルの import 解析結果を出力
- `rename_symbol` —— **純粋なソース AST 書き換え**、LSP 形式の位置クエリは行わない（既存の `lsp::handlers::rename` とはセマンティクスが異なる）
- ~~`hover` / `completion` / `signature_help`~~ —— **すべて削除**：AI agent は「位置に敏感な」セマンティクスを行わない、代わりに `lookup_symbol` で名前検索を使用

**World ロードタイミング**：server 起動時に `--project-root` で `yaoxiang.toml` と `src/**/*.yx` をスキャンし、LSP-017 で既に実装済みの `World::load_*` API を再利用し、`World.documents` に一度にロードする。**lib API は一切新規追加しない**。

### ツール契約

**入力**：JSON Schema で記述し、各フィールドに `description` + `examples` を付ける（LLM が自動的に理解する）。

**出力**：構造化された JSON、すべてに `schemaVersion: "1.0"` フィールドを含む：

```jsonc
// 成功レスポンス
{
  "schemaVersion": "1.0",
  "isError": false,
  "content": [
    { "type": "json", "json": { /* ツール固有データ */ } }
  ]
}

// 診断は構造化されて返される（tool エラーとは見なされない）
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
  "content": [{ "type": "text", "text": "MCP-INVALID-INPUT: source は有効な UTF-8 ではありません" }],
  "errorCode": "MCP-INVALID-INPUT"
}
```

**エラー体系**：
- **診断（diagnostic）**：解析/型エラー、RFC-013（`E0001` など）に従う——**tool エラーとは見なさない**
- **ツールレベルエラー**：`MCP-` 接頭辞を使用（`MCP-INVALID-INPUT`、`MCP-PROJECT-NOT-FOUND`、`MCP-INTERNAL`）——`isError: true` として扱う
- **パニック/クラッシュ**：JSON-RPC `-32603 Internal error`、server は終了しない

**パス解決ルール**（`lookup_symbol` / `find_references` の `workspace_root`、`typecheck` の `file_paths` に適用）：
1. コマンドライン `--project-root <dir>` が最優先（デフォルトを上書き）
2. それ以外：cwd から `yaoxiang.toml` を上に探す（ファイルシステムのルートまで、RFC-015 に従う）
3. それ以外：cwd 自体
4. `file_paths` はプロジェクトルート内に存在しなければならない（パストラバーサル防止）；範囲外 → `MCP-PATH-OUTSIDE-PROJECT`

### トランスポート層

**stdio（デフォルト）**：

```bash
yaoxiang mcp
# 起動後、stdin から JSON-RPC を読み、stdout に書き、stderr はログ用
```

AI agent 設定（Claude Code `.mcp.json` / Continue `config.json`）：
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

**streamable HTTP（オプション）**：

```bash
yaoxiang mcp --http --addr 127.0.0.1:7325  # 単一 HTTP ポート、新しい MCP 仕様
yaoxiang mcp --http --sse --addr 127.0.0.1:7325  # 旧 SSE と互換（v0.10）
```

**セキュリティ制約**：
- **loopback のみリッスン**（127.0.0.1 / ::1）；パブリックバインドは明示的に拒否しエラー終了
- HTTP **認証なし**（loopback はデフォルトで信頼）；将来 `--require-token <hex>` フィールドを追加予定
- stdio 子プロセスモードは本質的に分離される（parent プロセスが権限を制御）

### マルチプロセスと並行処理

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

**ポート競合**：AI agent は「子プロセスを起動」設定——本質的にポート競合ゼロ。HTTP モードではユーザーが自分でポートを管理する必要がある。
**World 分離**：各プロセスは独立した LSP 同期状態を持つ——1 つの MCP プロセスのクラッシュは **LSP や他の MCP プロセスに影響しない**。
**将来の Sessions**：v2 でようやく複数ワークスペースのディスパッチ（同一プロセス内の複数 `Session`）を検討する、**本 RFC では行わない**。

## 詳細設計

### データ構造

新規 `src/mcp/project.rs`：

```rust
pub struct ProjectRoot {
    /// 絶対パス
    pub root: PathBuf,
    /// ロード時にプロジェクトルートを識別した戦略のソース
    pub source: ProjectRootSource,
}

pub enum ProjectRootSource {
    CliFlag,           // yaoxiang mcp --project-root
    AutoDetected,      // yaoxiang.toml を上方向に検索
    FallbackCwd,       // cwd にフォールバック
}

pub struct ResolvedPath {
    /// プロジェクトルートからの相対パス（AI 読み取り用に推奨）
    pub relative: String,
    /// 解決後の絶対パス（World 操作用）
    pub absolute: PathBuf,
}

impl ProjectRoot {
    /// "file_path" を安全なパスに解決する——パストラバーサル防止
    pub fn resolve(&self, file_path: &str) -> Result<ResolvedPath, McpError>;
}
```

`ProjectRoot` シングルトン + `src/mcp/schema.rs` のツール schema 自動生成：

```rust
pub struct ProjectRoot {
    /// 絶対パス（`yaoxiang.toml` を含むか、後方互換のフォールバックを持つ）
    pub root: PathBuf,
    pub source: ProjectRootSource,
}

impl ProjectRoot {
    /// CLI 起動時に一度だけ識別し、結果を `McpServer` コンテキストにキャッシュする——すべてのツールで再利用
    pub fn detect(cli_override: Option<PathBuf>) -> Result<Self, McpError>;
}
```

ツール schema は `schemars` crate を使って input struct から自動生成し、手書きの JSON Schema のずれを防ぐ：

```rust
#[derive(Deserialize, schemars::JsonSchema)]
pub struct ParseSourceInput {
    /// 完全な YaoXiang ソーススニペット——**ディスクには保存されない**、純粋な transient
    pub source: String,
    pub tab_size: Option<u32>,
}
```

**`parse_source` / `format_source` ツールの schema には `file_path` フィールドがない**——この 2 つのツールは文字列ソースのみを受け取り、プロジェクトセマンティクスに参加しない。`lookup_symbol` / `find_references` / `typecheck` は `workspace_root` または `file_paths` を受け取る（必須か否かはツール表参照）。


### コンパイラ変更

| モジュール | 変更 |
|---|---|
| `src/lsp/world.rs` | **変更なし**——MCP 起動時に LSP に既存の `World::load_*` API を呼び出してワークスペースを一度ロード |
| `src/lsp/handlers/workspace_symbol.rs` | **変更なし**——`mcp/tools/lookup.rs` で 1 段ラップして `query` を LSP の引数に変換 |
| `src/lsp/handlers/references.rs` | **変更なし**——同上 |
| `src/lsp/handlers/formatter.rs` | **変更なし**——`format_source` から直接呼び出し |
| `src/main.rs` | `Mcp` サブコマンド分岐を追加 |
| `Cargo.toml` | `mcp-server` feature を追加（またはメインバイナリに常に含める） |
| `src/util/diagnostic/` | **変更なし**（RFC-017 で実装済み） |

**重要な制約**：`src/mcp/` は `src/lsp/` のプライベートシンボルへの**逆依存を許可しない**——`crate::lsp::` の公開 API 経由でのみ handler を呼び出せる。

### 後方互換性

- ✅ **完全に後方互換**：新サブコマンド `yaoxiang mcp` を追加し、`yaoxiang` / `yaoxiang lsp` の既存動作は**一切変更しない**
- ✅ **LSP server は不変**：RFC-017 で実装されたすべての能力、API、内部状態は不変
- ✅ **lib crate の公開 API は不変**：すべての `pub` パスは不変；MCP は既存 API を消費するのみ——新規 `pub` メソッドの追加は**ゼロ**

### 既存システムとの統合

| 既存モジュール | MCP 統合方法 |
|---|---|
| `src/frontend/lexer` | `parse_source` から lexer を直接呼び出し |
| `src/frontend/core/parser` | `parse_source` から parser を直接呼び出し；失敗時に `Missing*` ノードを生成（RFC-017） |
| `src/frontend/core/typecheck/inference/*` | `typecheck` は `collect_diagnostics` パターンを再利用（RFC-017 §問題1） |
| `src/middle/` | `typecheck` は全 middle pass を実行（依存解析など） |
| `src/lsp/world.rs` | 起動時に `World::load_*` API を呼び出し（既存）；World は「仮想ドキュメント」を一切受け付けない |
| `src/lsp/handlers/workspace_symbol.rs` | `mcp/tools/lookup.rs` で 1 段ラップし、`query: String` を LSP の引数に変換（名前検索） |
| `src/lsp/handlers/references.rs` | `mcp/tools/find_refs.rs` で 1 段ラップし、`query: String` を LSP の引数に変換 |
| `src/lsp/handlers/formatter.rs` | `mcp/tools/format.rs` から直接呼び出し（未実装なら `formatter::format_with_diff` を新規追加） |
| `src/util/i18n/` | エラーメッセージは多言語リソースファイル（zh-CN/en）経由 |

### エラー処理

| ソース | 処理 |
|---|---|
| 解析エラー | `Diagnostic{code:"E0xxx", severity, message, span}`（**tool エラーではない**、content 内で返す） |
| 型エラー | 同上 |
| `file_paths` の範囲外（`typecheck` ツール） | ツールレベルエラー `MCP-PATH-OUTSIDE-PROJECT` |
| `source` が不正な UTF-8 | ツールレベルエラー `MCP-INVALID-INPUT` |
| ツールパニック | JSON-RPC `-32603 Internal error`；server は**終了しない** |
| クライアントが非 JSON-RPC を送信 | 直接ストリーム切断（stdio EOF）、再起動で新セッション |

診断の重大度レベルは RFC-017（実装済み）の `enum ErrorKind { Error, Warning, Note }` に従う。

### テスト戦略

| 層 | テスト |
|---|---|
| **Unit** | `src/mcp/project.rs::resolve` のパストラバーサル、`src/mcp/schema.rs` の schema 検証 |
| **Integration** | stdio をモック：server を起動し、stdin に JSON-RPC を流し、stdout からレスポンスを読み、fixture と比較 |
| **E2E** | 実際の `yaoxiang mcp` プロセスを実行、Claude Code スタイルのツール呼び出しチェーン：parse → 修正 → format → typecheck |
| **Fuzz** | MCP JSON-RPC 解析の `cargo-fuzz`（libFuzzer ハーネス） |

各ツールは少なくとも happy path 1 個 + diagnostic シナリオ 1 個 + tool-error シナリオ 1 個の integration テストを持たなければならない。

## トレードオフ

### 利点

- **再利用コストが極めて低い**：`World` / `Session` / `handlers` / 診断収集はすべて実装済み（RFC-017）であり、本 RFC は「MCP シェルを 1 段追加する」だけ
- **AI-First インターフェース**：ツール契約は LSP より 3-5 倍直感的；LLM は schema を直接読める
- **マルチプロセス分離**：LSP エディタセッションや他の MCP プロセスから分離され、**ロック競合ゼロ**
- **stdio フレンドリ**：すべての主要な AI agent がデフォルトの子プロセスモード、ゼロ設定で統合
- **YAGNI を満たす**：本 RFC は Resources、Sessions、クロスプロセス状態、リモート MCP を削除する——v2 で再開

### 欠点

- **プロトコル分裂**：将来 LSP / MCP / DAP の 3 つのプロトコルが個別に進化し、一貫性維持のコストがかかる
- **HTTP モードは二の次**：loopback 制限によりローカルツールとして位置付けられ、リモートシナリオは v2 で再設計が必要
- **parse 処理の重複**：AI がソースコードを繰り返し微調整して `parse_source` を繰り返し呼び出すと、毎回 lexer+parser が再実行される。**緩和策**：RFC-017 の `DocumentCache` により**ディスク上**の同一ソースの 2 回目の解析は高速化可能；純粋な transient ソースは 1 回の解析が避けられない
- **テストカバレッジのコスト**：5 ツール × 3 シナリオ = 最低 15 個の integration テスト

## 代替案

| 案 | 選ばない理由 |
|---|---|
| **同一プロセス内に二プロトコルを埋め込む**（LSP+MCPlistener 共存） | stdin/stdout は 1 つのコンシューマしか持てない；HTTP も共存必要——複雑度 > 利益 |
| **LSP-client ブリッジとしての MCP** | IPC が 1 段増える；LSP は設計上、名前によるシンボル検索をサポートしない——MCP が欲しい能力は LSP では提供できない |
| **gRPC / カスタムプロトコルを使用** | デファクト標準から離れる；コミュニティにすでに MCP SDK（TypeScript、Python、Rust）があり、エコシステムが整っている |
| **LSP handler の全能力（L3 ツールセット）を再利用** | 大量の位置↔インテント適応作業が必要；限界収益逓減 |
| **最初のバージョンは HTTP のみ**（stdio なし） | Claude Code / Continue などは stdio がデフォルトであり、障壁が高すぎる |

## 実装戦略

### 依存関係

- **強い依存**：RFC-017 LSP 実装（実装済み）
- **強い依存**：RFC-013 エラーコード体系（実装済み）
- **強い依存**：RFC-014 / RFC-015 プロジェクトルート識別（一部実装済み）
- **新規依存**（Rust crate）：
  - `mcp-rust-sdk`（評価待ち、[modelcontextprotocol/rust-sdk](https://github.com/modelcontextprotocol/rust-sdk) 参照）
  - `tokio`（**既存**、optional feature）
  - `axum`（HTTP モード）または `hyper` 直接——評価待ち
- **言語仕様の変更ゼロ**：純粋なツールチェーン増分

### 段階（#154 と同期）

| 段階 | 内容 | 期間見積もり |
|---|---|---|
| **v0.8.x (MVP)** | `src/mcp/{mod.rs, server.rs, transport/stdio.rs, project.rs, schema.rs}` + `parse_source` + `format_source` + `lookup_symbol` + `find_references` + `typecheck`（**5 ツール**）+ `yaoxiang mcp` サブコマンド + 起動時 `World::load_*` | **3-4 週間** |
| **v0.9.x (YaoXiang スマート)** | `+ explain_diagnostic`（`render_explain_output` を**直接呼び出し**）+ `+ list_imports`（`ModuleGraph::validate_imports` をラップ）+ ユニット/統合テスト | **1-2 週間** |
| **v0.10.x (AST + HTTP)** | `+ rename_symbol`（`src/middle/rename.rs` を**新規追加**、AST 書き換え）+ streamable HTTP transport + パフォーマンスチューニング（`parse_source` P99 < 100ms） | **2-3 週間** |


**なぜ 3 段階に分けるか**：MVP で stdio + 5 ツールをまず動かし、インターフェース設計の妥当性を検証する；v0.9.x で低リスク・ゼロ適応の「YaoXiang 特有」ツールを追加し、統合の正確性を検証する；v0.10.x でようやく高リスクな「AST 書き換え」新モジュールを開く（独立した PR レビューがより焦点化される）。

### リスク

1. **`mcp-rust-sdk` のメンテナンス活発度**：2025 年にリリースされたばかりで、API が激しく変化する可能性がある。**緩和策**：安定性を評価し、不安定なら軽量な JSON-RPC 2.0 + tool dispatcher を自作（< 500 行）
2. **parse 処理の重複**：AI がソースコードを繰り返し微調整して `parse_source` を繰り返し呼び出すと、毎回 lexer+parser が再実行される。**緩和策**：RFC-017 の `DocumentCache` により**ディスク上**の同一ソースの 2 回目の解析は高速化可能；純粋な transient ソースは 1 回の解析が避けられない
3. **AI agent の schema 互換性**：agent によって MCP schema の厳密度が異なる。**緩和策**：`schemars` crate を使い、Rust の input 構造から schema を自動生成し、手書きのずれをゼロに
4. **パス解決のマルチプラットフォーム対応**：Windows パスは大文字小文字を区別しない、UNC パス、`\\` 境界。**緩和策**：パス解決に `camino::Utf8Path` を `std::path` の代わりに使用
5. **MCP ツール schema と LSP の引数の 1:1 不一致**：LSP `workspace_symbol` は `(query)` を受け取る；LSP 内部に渡す際は、既存 handler を再利用するために位置+URI にラップする必要がある。**緩和策**：`mcp/tools/lookup.rs` に適応層を設け、詳細を MCP 側にカプセル化
6. **`rename_symbol` の AST 書き換えと LSP `rename` のセマンティクスの違い**：LSP `textDocument/rename` は URI + 位置 + new_name → WorkspaceEdit；MCP `rename_symbol` は source + old_name + new_name → 新 source。**直接の再利用は不可**。**緩和策**：`src/middle/rename.rs` を別途実装し、scope-aware に参照を書き換え、LSP handler の実装と相互干渉させない

## 未解決の問題

- [ ] `mcp-rust-sdk` 採用 / 自作？（@Chen Xu：まず rust-sdk の 6 月バージョンを評価してから決定）
- [ ] HTTP 認証パス？（v0.10 RFC で再開）
- [ ] `MCP` 起動時に `tools/list` を出力して AI の能動的発見を可能にするか？（MCP 標準要件、**デフォルト実装**）
- [ ] `typecheck` は `mode: "fast|full"` をサポートするか？（fast = 現在のファイルサブセットのみ、full = ワークスペース全体）？
- [ ] パフォーマンス予算 `parse_source` P99 < 100ms は現実的か？（RFC-017 で実装済みの `DocumentCache` の source-string モードでの実オーバーヘッドをベンチマークする必要あり）

## 参考文献

- [RFC-017: 言語サーバープロトコル（LSP）サポート設計](./accepted/017-lsp-support.md)
- [RFC-013: エラーコード仕様設計](./accepted/013-error-code-specification.md)
- [RFC-014: パッケージ管理システム設計](./accepted/014-package-manager.md)
- [RFC-015: YaoXiang 設定システム設計](./accepted/015-configuration-system.md)
- [MCP 仕様](https://modelcontextprotocol.io/)
- [MCP Rust SDK](https://github.com/modelcontextprotocol/rust-sdk)
- [JSON-RPC 2.0](https://www.jsonrpc.org/specification)
- [LSP 仕様 3.18](https://github.com/microsoft/language-server-protocol/blob/main/specifications/specification-3-18.md)
- [Rust Analyzer](https://rust-analyzer.github.io/) —— M2 / MCP 統合の参考
- [zed-industries/zed の MCP 実装](https://github.com/zed-industries/zed/tree/main/crates/mcp)
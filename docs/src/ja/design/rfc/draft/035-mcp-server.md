---
title: 'RFC-035: MCP サーバー対応（AI Agent 統合）'
status: '草案'
author: '晨煦'
created: '2026-07-11'
updated: '2026-07-11'
issue: '#154'
---

# RFC-035: MCP サーバー対応（AI Agent 統合）

## 摘要

YaoXiang に MCP（Model Context Protocol）サーバーを追加し、AI agent（Claude
Code、Continue、Cody、Zed 等）が YaoXiang ソースコードの
**AST、解析エラー、型、シンボル、参照、フォーマット結果**を直接クエリできるようにする。RFC-017 で既に実装済みの `World` バックエンドを再利用し、新規に
`yaoxiang mcp` サブコマンドを追加、単一バイナリで二モード、マルチプロセスで独立した World。

## 動機

### なぜこの機能が必要か？

RFC-017 で YaoXiang をエディターから理解できるようにした（hover / goto-def /
completion）。しかし LSP は**位置駆動**プロトコルである：

- 各リクエストは `textDocument` URI + `Position` に強く依存する
- エディターは先にファイルを開いて保存し、LSP サーバーとの長寿命接続を維持する必要がある
- AI agent のワークフローは**コードスニペット**中心：会話の中で「コード片を貼り付けて」質問する、**先に保存しない**

AI agent が実際に使える LSP クライアント（vscode-langservers-extracted、`mcp-lsp-bridge`
などのプロジェクト）は L1 のみ翻訳可能：goto-def、hover。AI がやりたいこと：

- 「このコードが**正しく解析できているか**」—— parse + 完全な diagnostic フローが必要
- 「このシンボルが**ファイル内でどのように使われているか**」—— lookup_symbol を名前でクエリ必要
- 「このコードが**フォーマット後どうなるか**」—— format_source 必要
- 「**全部の**型エラーはどこか」—— typecheck で完全なワークスペース実行必要

これらの L1 LSP 翻訳能力は**做不到**。LSP の設計上サポートしていないため。

### 現在の問題

1. AI agent の LSP 呼び出し体験が悪い：モックドキュメントが必要、JSON が巨大、强 URI 依存
2. YaoXiang プロジェクトに「AI-First」インターフェース層がない：人間は IDE で LSP を使うが、AI agent は LSP を使えない
3. Claude Code / Continue などの主要な AI agent は既にデフォルトで MCP をサポートしており、YaoXiang にとっては空白のエコシステム

### MCP とは？

MCP（Model Context Protocol）は 2024-2025 年に Anthropic が主導して公开发表・开源した AI
agent ツール呼び出しプロトコルで、事実上の標準となっている（OpenAI、Google、Microsoft、Zed、Continue、Cody などが採用）。特徴：

- JSON-RPC 2.0 ベース（LSP と同源）
- 3 大プリミティブ：**Tools**（アクション）、Resources（データ）、Prompts（テンプレート）
- トランスポート：`stdio`（サブプロセス）/ streamable `HTTP` / SSE
- ツールの入出力には **JSON Schema** による强型付け（LLM に優しい）
- 2025-06 以降に streamable HTTP 仕様が公开发表され、本 RFC は旧 SSE との互換性も持つ

**本 RFC は Tools プリミティブのみを使用**——LSP の「サービスを提供」と对齐し、Resources のファイルモデル複雑さを引入しない。

## 提案

### コアデザイン

単一バイナリで二モード：

```text
┌─────────────────────────────────────────────────────────┐
│                    yaoxiang（v0.7.7+）                  │
│  ┌─────────────────┐      ┌──────────────────────────┐  │
│  │ yaoxiang lsp    │      │   yaoxiang mcp           │  │
│  │ (stdio JSON-RPC)│      │   (stdio default         │  │
│  │ RFC-017 実装済み│      │    + HTTP 任意)          │  │
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
│  │            src/mcp/  ← 新規追加                   │  │
│  │  ├── mod.rs          （モジュール入口 + 起動関数） │  │
│  │  ├── transport/      （stdio + HTTP/SSE）         │  │
│  │  ├── server.rs       （JSON-RPC メッセージループ） │  │
│  │  ├── tools/          （6 つの tool handler）        │  │
│  │  ├── schema.rs       （入出力 JSON Schema）        │  │
│  │  └── project.rs      （プロジェクトルート認識 + パス解決）│  │
│  └──────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

**主要な意思決定**：

- **同一バイナリ**：`yaoxiang` はサブコマンドで切り替え；LSP プロセスと MCP プロセスは**同じランタイムに共存しない**
- **マルチプロセス独立 World**：各 `yaoxiang mcp` プロセスは 1 つの
  `World` を保持；LSP プロセスや他の MCP プロセスとは互いに不影响（ロック競合なし、独立したクラッシュ隔离）
- **stdio デフォルト**：ポート冲突を回避、ゼロネットワーク設定；HTTP は任意の後継手段
- **再利用而非重复**：直接 `yaoxiang::frontend` / `yaoxiang::middle` / `yaoxiang::lsp::handlers`
  の lib API を呼び出す、LSP-client 中転を**しない**

### ツール群（8 つのツール、3 段階でリリース）

「特殊ケースの排除 + 段階的リリース」の原則でデザイン：純粋なソース stateless ツールが先導、ワークスペースツールは LSP World を共有、AST 書き換えツールは新規追加。

| Tool 名                | 入力                                                                                             | 出力                                                          | 再利用                                                         | 段階        |
| ---------------------- | ------------------------------------------------------------------------------------------------ | ------------------------------------------------------------- | -------------------------------------------------------------- | ----------- |
| `parse_source`         | `source: String`, `tab_size?: u32`                                                               | `{ast: Node, diagnostics: Diagnostic[]}`                      | 直接 `frontend::parse` を呼び出す                              | v0.8.x      |
| `format_source`        | `source: String`, `tab_size?: u32`                                                               | `{formatted: String, diff: Hunk[]}`                          | 直接 `formatter::format` を呼び出す                            | v0.8.x      |
| `lookup_symbol`        | `query: String`, `workspace_root?: String`, `kind?: SymbolKind[]`                               | `{symbols: Symbol[]}`                                         | `lsp::handlers::workspace_symbol` を再利用（`query` 模糊マッチ）| v0.8.x      |
| `find_references`      | `query: String`, `workspace_root?: String`                                                        | `{locations: Location[]}`                                     | `lsp::handlers::references` を再利用（`query` で而非位置）      | v0.8.x      |
| `typecheck`            | `file_paths: String[]`, `project_root: String`                                                   | `{diagnostics: Diagnostic[], summary: Counts}`               | `lsp::world::typecheck_full` を再利用                          | v0.8.x      |
| `explain_diagnostic`   | `code: String`（例：`E0001`），`lang?: String`                                                    | `{code, category, title, description, example, help}`        | **直接** `util::diagnostic::command::render_explain_output` を呼び出す | **v0.9.x**  |
| `list_imports`          | `file_path: String`, `project_root?: String`                                                     | `{imports: [{module, items, is_public}]}`                     | `middle::passes::module::ModuleGraph::validate_imports` を再利用| **v0.9.x**  |
| `rename_symbol`        | `source: String`, `old_name: String`, `new_name: String`, `scope?: "module" \| "function:name"` | `{source: String, edits: Edit[], diagnostics: Diagnostic[]}`| **新規追加** `src/middle/rename.rs`（AST 書き換え）             | **v0.10.x** |

**8 ツールの境界**：

- `parse_source` / `format_source` —— **純粋ソース stateless**、World に人不る
- `lookup_symbol` / `find_references` —— `workspace_root` を受け取る（渡さない場合は起動時の `--project-root` を使用）
- `typecheck` —— `file_paths` は**必須**、ワークスペースの完全性を保证
- `explain_diagnostic` —— **ゼロファイル依存**、純粋な文字列でエラーコードレジストリをクエリ
- `list_imports` —— `file_path` は物理ファイル、そのファイルの import 解析結果を返す
- `rename_symbol` —— **純粋ソース AST 書き換え**、LSP スタイルの位置クエリはしない（既存の `lsp::handlers::rename` とは意味が異なる）
- ~~`hover` / `completion` / `signature_help`~~ —— **全て廃止**：AI agent は「位置敏感的」セマンティクスを行わない、`lookup_symbol` で名前クエリすれば十分

**World 読み込みタイミング**：サーバー起動時に `--project-root` で `yaoxiang.toml` と
`src/**/*.yx` をスキャン、RFC-017 で既に実装済みの `World::load_*` API を再利用し、一度に
`World.documents` に読み込む。新規 lib API は**追加しない**。

### ツールコントラクト

**入力**：JSON Schema で記述、各フィールドに `description` + `examples` を付与（LLM が自动理解）。

**出力**：構造化 JSON、统一して `schemaVersion: "1.0"` フィールドを含む：

```jsonc
// 成功応答
{
  "schemaVersion": "1.0",
  "isError": false,
  "content": [
    { "type": "json", "json": { /* ツール固有データ */ } }
  ]
}

// 診断は構造化で返される（tool エラーとは見なさない）
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

// ツールレベルのエラー（例：parse_source が不正な UTF-8 を受け取る）
{
  "schemaVersion": "1.0",
  "isError": true,
  "content": [{ "type": "text", "text": "MCP-INVALID-INPUT: source は有効な UTF-8 ではありません" }],
  "errorCode": "MCP-INVALID-INPUT"
}
```

**エラー体系**：

- **診断（diagnostic）**：解析/型エラー、RFC-013（`E0001` 等）を沿用—— **tool エラーとは見なさない**
- **ツールレベルエラー**：`MCP-`
  プレフィックス（`MCP-INVALID-INPUT`、`MCP-PROJECT-NOT-FOUND`、`MCP-INTERNAL`）—— `isError: true` として扱う
- **panic/crash**：JSON-RPC `-32603 Internal error`、サーバーは終了しない

**パス解決ルール**（`lookup_symbol` / `find_references` の `workspace_root`、`typecheck` の
`file_paths` に適用）：

1. コマンドライン `--project-root <dir>` が最高優先度（デフォルトをオーバーライド）
2. それ以外：cwd を上に辿ってファイルシステムルートまで `yaoxiang.toml` を探す（RFC-015 を沿用）
3. それ以外：cwd 自身
4. `file_paths` はプロジェクトルート内に存在する必要がある（トラバーサル防止）；範囲外 → `MCP-PATH-OUTSIDE-PROJECT`

### トランスポート層

**stdio（デフォルト）**：

```bash
yaoxiang mcp
# 起動後は stdin から JSON-RPC を読み込み、stdout に書き込み、stderr はログ用
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

**streamable HTTP（任意）**：

```bash
yaoxiang mcp --http --addr 127.0.0.1:7325  # 単一 HTTP ポート、新 MCP 仕様
yaoxiang mcp --http --sse --addr 127.0.0.1:7325  # 旧 SSE との互換性（v0.10）
```

**セキュリティ制約**：

- **loopback のみlisten**（127.0.0.1 / ::1）；パブリックバインディングは明示的に拒否してエラー終了
- HTTP は**認証なし**（loopback はデフォルトで信頼）；将来自動で `--require-token <hex>` フィールド追加
- stdio サブプロセスモードは自然に隔离（親プロセスが権限を制御）

### マルチプロセスと並行処理

各 `yaoxiang mcp` プロセスは 1 つの `World` を保持し、互いに共有しない：

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

**ポート冲突**：AI agent が「サブプロセスを起動」するように設定——自然にゼロポート冲突。HTTP モードではユーザーが自行でポート割り当てを管理する必要がある。
**World 隔离**：各プロセスが独立した LSP 同期状態を保持——1 つの MCP プロセスがクラッシュしても**影響しない**LSP/他の MCP プロセス。 **future Sessions**：v2 で初めてマルチワークスペース配布を検討（同一プロセス内で複数の `Session`）、**本 RFC では行わない**。

## 詳細な設計

### データ構造

新規 `src/mcp/project.rs`：

```rust
pub struct ProjectRoot {
    /// 絶対パス
    pub root: PathBuf,
    /// 読み込み時のプロジェクトルート認識戦略のソース
    pub source: ProjectRootSource,
}

pub enum ProjectRootSource {
    CliFlag,           // yaoxiang mcp --project-root
    AutoDetected,      // 上に yaoxiang.toml を探す
    FallbackCwd,       // cwd にフォールバック
}

pub struct ResolvedPath {
    /// プロジェクトルートからの相対パス（AI が読む，推荐）
    pub relative: String,
    /// 解決後の絶対パス（World 操作に使用）
    pub absolute: PathBuf,
}

impl ProjectRoot {
    /// 「file_path」を安全パスに解決——トラバーサル防止
    pub fn resolve(&self, file_path: &str) -> Result<ResolvedPath, McpError>;
}
```

`ProjectRoot` シングルトン + `src/mcp/schema.rs` ツール schema 自動生成：

```rust
pub struct ProjectRoot {
    /// 絶対パス（`yaoxiang.toml` を含む、または下位互換性フォールバック）
    pub root: PathBuf,
    pub source: ProjectRootSource,
}

impl ProjectRoot {
    /// CLI 起動時に一度識別し、結果を `McpServer` コンテキストにキャッシュ——全ツールで再利用
    pub fn detect(cli_override: Option<PathBuf>) -> Result<Self, McpError>;
}
```

ツール schema は `schemars` crate を使用して input struct から自動生成。手書き JSON Schema のドリフトを避ける：

```rust
#[derive(Deserialize, schemars::JsonSchema)]
pub struct ParseSourceInput {
    /// 完全な YaoXiang ソースコードスニペット——**ディスクに保存しない**、純粋 transient
    pub source: String,
    pub tab_size: Option<u32>,
}
```

**`parse_source` / `format_source` ツール schema には `file_path` フィールドがない**——この 2 ツールは文字列ソースのみを受け取り、プロジェクトセマンティクスに関与しない。`lookup_symbol` / `find_references` / `typecheck` は `workspace_root` または `file_paths` を受け取る（必須かどうかはツール表を参照）。

### コンパイラの改动

| モジュール                                 | 改动                                                                        |
| ------------------------------------------ | --------------------------------------------------------------------------- |
| `src/lsp/world.rs`                         | **ゼロ改动**——MCP 起動時に LSP の既存の `World::load_*` API でワークスペースを一括読み込み |
| `src/lsp/handlers/workspace_symbol.rs`     | **ゼロ改动**——`mcp/tools/lookup.rs` がラッパーとして `query` を LSP 入力に変換 |
| `src/lsp/handlers/references.rs`           | **ゼロ改动**——同上                                                          |
| `src/lsp/handlers/formatter.rs`            | **ゼロ改动**——format_source が直接呼び出す                                  |
| `src/main.rs`                              | `Mcp` サブコマンドブランチを追加                                             |
| `Cargo.toml`                               | `mcp-server` feature を追加（またはメインビナリは常に含む）                 |
| `src/util/diagnostic/`                     | **ゼロ改动**（RFC-017 で実装済み）                                           |

**主要な制約**：`src/mcp/` は `src/lsp/` のプライベートシンボルへの逆依存を**許可しない**——`crate::lsp::`
の公開 API を通じてのみ handlers を呼び出す。

### 後方互換性

- ✅ **完全な後方互換性**：新規サブコマンド `yaoxiang mcp`、`yaoxiang` / `yaoxiang lsp` の既存動作は変更なし
- ✅ **LSP サーバーは不动**：RFC-017 実装の全機能、API、内部状態は一切不变
- ✅ **lib crate 公開 API は不动**：全て `pub` パスは変更なし；MCP は既存の API のみを利用——**ゼロ**新規 `pub` メソッド

### 既存システムとの統合

| 既存モジュール                              | MCP 統合方式                                                               |
| ------------------------------------------- | ------------------------------------------------------------------------- |
| `src/frontend/lexer`                        | parse_source が直接 lexer を呼び出す                                       |
| `src/frontend/core/parser`                  | parse_source が直接 parser を呼び出す；失敗時は `Missing*` ノードを生成（RFC-017）|
| `src/frontend/core/typecheck/inference/*`   | typecheck が `collect_diagnostics` パターンを再利用（RFC-017 §問題1）        |
| `src/middle/`                               | typecheck が全 middle pass を実行（依存関係分析等）                         |
| `src/lsp/world.rs`                          | 起動時に `World::load_*` API を呼び出す（既存）；World は「仮想ドキュメント」を受け入れない |
| `src/lsp/handlers/workspace_symbol.rs`       | `mcp/tools/lookup.rs` がラッパー、`query: String` を LSP 入力に変換（名前クエリ）|
| `src/lsp/handlers/references.rs`            | `mcp/tools/find_refs.rs` がラッパー、`query: String` を LSP 入力に変換      |
| `src/lsp/handlers/formatter.rs`             | `mcp/tools/format.rs` が直接呼び出す（未実装の場合、`formatter::format_with_diff` を新規追加）|
| `src/util/i18n/`                            | エラーメッセージは多言語リソースファイルを参照（zh-CN/en）                   |

### エラー処理

| ソース                                    | 処理                                                                                         |
| ----------------------------------------- | -------------------------------------------------------------------------------------------- |
| 解析エラー                                 | `Diagnostic{code:"E0xxx", severity, message, span}`（**tool エラーではない**、content 内で返される）|
| 型エラー                                   | 同上                                                                                         |
| `file_paths` 範囲外（`typecheck` ツール）   | tool レベルエラー `MCP-PATH-OUTSIDE-PROJECT`                                                   |
| `source` 不正な UTF-8                      | tool レベルエラー `MCP-INVALID-INPUT`                                                          |
| ツール panic                               | JSON-RPC `-32603 Internal error`；サーバーは**終了しない**                                    |
| クライアントが非 JSON-RPC を送信            | ストリームを直に切断（stdio EOF）、再起動で新セッション                                        |

診断深刻度レベルは RFC-017（既に実装済み）を沿用 `enum ErrorKind { Error, Warning, Note }`。

### テスト戦略

| 層                  | テスト                                                                                     |
| ------------------- | ------------------------------------------------------------------------------------------|
| **ユニット**        | `src/mcp/project.rs::resolve` パストラバーサル、`src/mcp/schema.rs` schema バリデーション  |
| **統合**            | stdio モック：サーバーを起動、stdin に JSON-RPC を注入、stdout から応答を読み、fixture と比較 |
| **E2E**            | `yaoxiang mcp` 本当のプロセスを実行、Claude Code スタイルのツール呼び出しチェーン：parse → 修正 → format → typecheck |
| **ファズ**          | MCP JSON-RPC 解析の `cargo-fuzz`（libFuzzer ハーネス）                                      |

各 tool には最低 1 つの happy path + 1 つの diagnostic シナリオ +
1 つの tool-error シナリオの統合テストがなければならない。

## 权衡

### メリット

- **再利用コストが非常に低い**：`World` / `Session` / `handlers`
  / 診断収集は全て既に実装済み（RFC-017）、本 RFC は「MCP シェルを追加する」だけ
- **AI-First インターフェース**：tool コントラクトは LSP より 3-5 倍直感的；LLM が schema を直に読む
- **マルチプロセス隔离**：LSP エディターセッション、他の MCP プロセスと切り離され、**ゼロロック競合**
- **stdio フレンドリー**：全ての主要 AI agent はデフォルトでサブプロセスモード、ゼロ設定で統合可能
- **YAGNI 通過**：本 RFC は Resources、Sessions、クロスプロセス状態、リモート MCP を全て排除——v2 で再開

### デメリット

- **プロトコル分裂**：將來 LSP / MCP / DAP の 3 つのプロトコルがそれぞれ進化、一貫性維持コスト
- **HTTP モードは二等市民**：loopback 制限でローカルツールとして位置付け、リモートシナリオは v2 で再設計必要
- **重复 parse オーバーヘッド**：AI がソースコードを微調整して `parse_source` を繰り返し呼び出すと、lexer+parser を再実行する。**緩和**：RFC-017 の `DocumentCache` に依存して**ディスク上**の同一ソースの二次解析を高速化；純粋 transient source の場合は一回解析は不可避
- **テストカバーコスト**：5 ツール × 3 シナリオ = 15 の統合テスト부터

## 代替案

| 方案                                          | 为什么不選                                                         |
| --------------------------------------------- | ------------------------------------------------------------------ |
| **プロセス内埋め込み二プロトコル**（LSP+MCPlistener 共存） | stdin/stdout は 1 つのコンシューマーのみ可能；HTTP も共存必要——複雑さ > 收益 |
| **MCP を LSP-client ブリッジとして使う**        | IPC がもう一层入る；LSP 設計は名前クエリをサポートしていない——MCP が欲しい能力は LSP からは得られない |
| **gRPC / カスタムプロトコルを使う**              | 事実標準から逸脱する；コミュニティには既に MCP SDK がある（TypeScript、Python、Rust）、エコシステムが揃っている |
| **LSP handler の全能力を再利用**（L3 ツール群）  | 大量の位置↔意図のアダプテーション作業；限界収益递減                  |
| **最初のバージョンは HTTP のみ**（stdio なし）   | Claude Code / Continue などはデフォルトで stdio、敷居が高すぎる       |

## 実装戦略

### 依存関係

- **強い依存**：RFC-017 LSP 実装（既に実装済み）
- **強い依存**：RFC-013 エラーコード体系（既に実装済み）
- **強い依存**：RFC-014 / RFC-015 プロジェクトルート認識（部分是既に実装済み）
- **新規依存**（Rust crate）：
  - `mcp-rust-sdk`（評価待ち、
    [modelcontextprotocol/rust-sdk](https://github.com/modelcontextprotocol/rust-sdk) を参照）
  - `tokio`（**既にあり**、optional feature）
  - `axum`（HTTP モード）または `hyper` 直接——評価待ち
- **ゼロ言語仕様変更**：純粋なツールチェーン增量

### 段階（#154 と同期）

| 段階                        | 内容                                                                                                                                                                                                                       | 工数見積もり |
| --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- |
| **v0.8.x (MVP)**            | `src/mcp/{mod.rs, server.rs, transport/stdio.rs, project.rs, schema.rs}` + `parse_source` + `format_source` + `lookup_symbol` + `find_references` + `typecheck`（**5 ツール**）+ `yaoxiang mcp` サブコマンド + 起動時の `World::load_*` | **3-4 週** |
| **v0.9.x (YaoXiang 知能)**  | `+ explain_diagnostic`（**直接** `render_explain_output` を呼び出す）+ `+ list_imports`（`ModuleGraph::validate_imports` をラップ）+ ユニット/統合テスト                                                                      | **1-2 週** |
| **v0.10.x (AST + HTTP)**    | `+ rename_symbol`（**新規** `src/middle/rename.rs`、AST 書き換え）+ streamable HTTP transport + パフォーマンス最適化（parse_source P99 < 100ms）                                                                               | **2-3 週** |

**なぜ 3 段階なのか**：MVP はまず stdio + 5 ツールでインターフェース設計の妥当性を検証；v0.9.x はリスクの低い「YaoXiang 固有」ツールで統合の正しさを検証；v0.10.x で高リスクな「新モジュール AST 書き換え」を開く（独立 PR レビューがより集中）。

### リスク

1. **`mcp-rust-sdk` のメンテナンス状況**：2025 年にようやく公开发表、API が激しく変わる可能性がある。**緩和**：評価して不安定であれば、自前で軽量な JSON-RPC 2.0 + tool dispatcher を実装（< 500 行）
2. **重复 parse オーバーヘッド**：AI がソースコードを微調整して `parse_source` を繰り返し呼び出すと、lexer+parser を再実行する。**緩和**：RFC-017 の `DocumentCache` に依存して**ディスク上**の同一ソースの二次解析を高速化；純粋 transient source の場合は一回解析は不可避
3. **AI agent schema 互換性**：異なる agent の MCP schema 厳格さが異なる。**緩和**：`schemars` crate を使用して Rust input 構造から schema を自動生成、手書きドリフトを排除
4. **パス解決のマルチプラットフォーム**：Windows パスは大小文字を区別しない、UNC パス、`\\` 境界。**緩和**：パス解決に `camino::Utf8Path` を使用して `std::path` を置き換える
5. **MCP ツール schema と LSP 入力は 1:1 ではない**：LSP `workspace_symbol` は
   `(query)` を受け取る；LSP 内部に渡すには位置+URI でラップして既存の handler を再利用させる必要がある。**緩和**：`mcp/tools/lookup.rs` にアダプターレイヤーを置き、MCP 側で詳細をカプセル化
6. **`rename_symbol` AST 書き換えと LSP `rename` の意味が異なる**：LSP `textDocument/rename` は URI + 位置 +
   new_name → WorkspaceEdit；MCP `rename_symbol` は source + old_name + new_name → 新 source。**直接再利用不可**。**緩和**：`src/middle/rename.rs` を单独実装、scope-aware な参照書き換え、LSP handler 実装とは互いに干渉しない

## 開放問題

- [ ] `mcp-rust-sdk` の選定 / 自前実装？（@Chen Xu：まず rust-sdk の 6 月バージョンを評価してから決定）
- [ ] HTTP 認証パス？（v0.10 RFC で再開）
- [ ] `MCP` 起動時に `tools/list` を出力して AI が能動的に発見できるようにする必要があるか？（MCP 標準で要求、**デフォルト実装**）
- [ ] `typecheck` は `mode: "fast|full"` をサポートするか？（fast = 現在のファイルサブセットのみ、full = ワークスペース全体）？
- [ ] パフォーマンス予算 parse_source P99 < 100ms は現実的か？（RFC-017 で既に実装済みの `DocumentCache` が source-string モードでどの程度のオーバーヘッドがあるかのベンチマークが必要）

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

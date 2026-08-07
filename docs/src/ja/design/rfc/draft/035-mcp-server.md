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
**AST、解析エラー、型、シンボル、参照、フォーマット結果**を直接問い合わせできるようにする。RFC-017 で実装済みの
`World` バックエンドを再利用し、`yaoxiang mcp`
サブコマンドを追加。単一バイナリでデュアルモード、プロセスごとに独立した World を保持する。

## 動機

### なぜこの機能が必要か？

RFC-017 によって YaoXiang は**エディタから理解される**ようになった（hover / goto-def /
completion）。しかし LSP は**位置駆動**のプロトコルである：

- 各リクエストは `textDocument` URI + `Position` に強く依存する
- エディタはまずファイルを開き、保存し、LSP サーバーと長寿命接続を維持する必要がある
- AI agent のワークフローは**コード片**：対話の中で「コードを貼って」質問する。**保存しない**

AI agent が実際に使える LSP クライアント（vscode-langservers-extracted、`mcp-lsp-bridge`
系プロジェクト）はいずれも**L1 のみを変換**する：goto-def、hover。AI が行いたいこと：

- 「このコードは**正しく解析されるか**」——parse + 完全な diagnostic ストリームが必要
- 「このシンボルは**ファイル内でどう使われているか**」——`lookup_symbol` で名前検索が必要
- 「このコードを**フォーマットするとどうなるか**」——`format_source` が必要
- 「**すべての**型エラーの場所」——`typecheck` でワークスペース全体を走らせる必要がある

これらの L1 LSP 変換能力では**不十分**。なぜなら LSP は設計上サポートしていないからだ。

### 現状の問題

1. AI agent からの LSP 呼び出し体験が悪い：ドキュメントのモックが必要、JSON が巨大、URI への強い依存
2. YaoXiang プロジェクトに「AI-First」インターフェース層がない：人間は IDE を使い LSP でやり、AI
   agent は LSP を使えない
3. Claude Code / Continue などの主要な AI
   agent は MCP をデフォルトサポートしているが、YaoXiang には空白のエコシステムがある

### MCP とは何か？

MCP（Model Context Protocol）は 2024-2025 年に Anthropic が主導してリリースしオープンソース化した AI
agent のツール呼び出しプロトコルで、事実上の標準となっている（OpenAI、Google、Microsoft、Zed、Continue、Cody などが対応）。特徴：

- JSON-RPC 2.0 ベース（LSP と同源）
- 3 つのプリミティブ：**Tools**（アクション）、Resources（データ）、Prompts（テンプレート）
- トランスポート：`stdio`（サブプロセス）/ streamable `HTTP` / SSE
- ツールの入出力に **JSON Schema** による強い型付け（LLM フレンドリー）
- 2025-06 以降に streamable HTTP 仕様がリリースされており、本 RFC は旧 SSE とも互換

**本 RFC では Tools プリミティブのみを使用**——LSP の「サービス提供」アラインメントに従い、Resources のファイルモデルの複雑さは導入しない。

## 提案

### コアデザイン

単一バイナリでデュアルモード：

```text
┌─────────────────────────────────────────────────────────┐
│                    yaoxiang（v0.7.7+）                  │
│  ┌─────────────────┐      ┌──────────────────────────┐  │
│  │ yaoxiang lsp    │      │   yaoxiang mcp           │  │
│  │ (stdio JSON-RPC)│      │   (stdio default         │  │
│  │ RFC-017 実装済  │      │    + HTTP オプション)    │  │
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
│  │  ├── server.rs       （JSON-RPC メッセージループ）│  │
│  │  ├── tools/          （8 つの tool handler）       │  │
│  │  ├── schema.rs       （入出力 JSON Schema）       │  │
│  │  └── project.rs      （プロジェクトルート検出 + パス解決）│
│  └──────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

**重要な意思決定**：

- **同一バイナリ**：`yaoxiang`
  はサブコマンドで切り替える；LSP プロセスと MCP プロセスは**同一ランタイムに共存しない**
- **プロセスごとに独立した World**：各 `yaoxiang mcp` プロセスが `World`
  を 1 つ保持；LSP プロセスや他の MCP プロセスとは互いに影響しない（ロック競合なし、独立したクラッシュ隔離）
- **stdio デフォルト**：ポート競合回避、ネットワーク設定不要；HTTP は任意の代替手段
- **再利用であって重複ではない**：`yaoxiang::frontend` / `yaoxiang::middle` /
  `yaoxiang::lsp::handlers` の lib API を直接呼び出し、LSP-client 経由とは**しない**

### ツールセット（8 ツール、3 フェーズで提供）

「特殊ケースの排除 + 段階化」の原則で設計：純粋なソースツールを stateless で先行提供し、ワークスペースツールは LSP
World を共有、AST 書き換えツールは独立して追加。

| ツール名             | 入力                                                                                            | 出力                                                         | 再利用                                                                | フェーズ    |
| -------------------- | ----------------------------------------------------------------------------------------------- | ------------------------------------------------------------ | --------------------------------------------------------------------- | ----------- |
| `parse_source`       | `source: String`, `tab_size?: u32`                                                              | `{ast: Node, diagnostics: Diagnostic[]}`                     | `frontend::parse` を直接呼び出し                                      | v0.8.x      |
| `format_source`      | `source: String`, `tab_size?: u32`                                                              | `{formatted: String, diff: Hunk[]}`                          | `formatter::format` を直接呼び出し                                    | v0.8.x      |
| `lookup_symbol`      | `query: String`, `workspace_root?: String`, `kind?: SymbolKind[]`                               | `{symbols: Symbol[]}`                                        | `lsp::handlers::workspace_symbol` を再利用（`query` で曖昧マッチ）    | v0.8.x      |
| `find_references`    | `query: String`, `workspace_root?: String`                                                      | `{locations: Location[]}`                                    | `lsp::handlers::references` を再利用（位置ではなく `query` で）       | v0.8.x      |
| `typecheck`          | `file_paths: String[]`, `project_root: String`                                                  | `{diagnostics: Diagnostic[], summary: Counts}`               | `lsp::world::typecheck_full` を再利用                                 | v0.8.x      |
| `explain_diagnostic` | `code: String`（例：`E0001`）, `lang?: String`                                                  | `{code, category, title, description, example, help}`        | `util::diagnostic::command::render_explain_output` を**直接呼び出し** | **v0.9.x**  |
| `list_imports`       | `file_path: String`, `project_root?: String`                                                    | `{imports: [{module, items, source_file}]}`                  | `middle::passes::module::ModuleGraph::validate_imports` を再利用      | **v0.9.x**  |
| `rename_symbol`      | `source: String`, `old_name: String`, `new_name: String`, `scope?: "module" \| "function:name"` | `{source: String, edits: Edit[], diagnostics: Diagnostic[]}` | `src/middle/rename.rs` を**新規追加**（AST 書き換え）                 | **v0.10.x** |

**8 ツールの境界**：

- `parse_source` / `format_source` —— **純粋なソース stateless**、World に入らない
- `lookup_symbol` / `find_references` —— `workspace_root` を受け取る（指定なしなら起動時の
  `--project-root` を使用）
- `typecheck` —— `file_paths` **必須**、ワークスペースの完全性を保証
- `explain_diagnostic` —— **ファイル依存ゼロ**、純粋にエラーコード登録表を文字列検索
- `list_imports` —— `file_path` は物理ファイル、そのファイルの import 解析結果を出力
- `rename_symbol` —— **純粋なソース AST 書き換え**、LSP 形式の位置クエリはしない（既存の
  `lsp::handlers::rename` とはセマンティクスが異なる）
- ~~`hover` / `completion` / `signature_help`~~ —— **すべて廃止**：AI
  agent は「位置依存」のセマンティクスを行わず、`lookup_symbol` で名前検索すれば十分

**World のロードタイミング**：サーバー起動時に `--project-root` に基づき `yaoxiang.toml` と
`src/**/*.yx` をスキャンし、LSP-017 で実装済みの `World::load_*` API を使って `World.documents`
に一度に流し込む。**lib API は何も追加しない**。

### ツール契約

**入力**：JSON Schema で記述し、各フィールドに `description` + `examples` を付与（LLM が自動理解）。

**出力**：構造化 JSON、すべてに `schemaVersion: "1.0"` フィールドを含む：

```jsonc
// 成功レスポンス
{
  "schemaVersion": "1.0",
  "isError": false,
  "content": [
    { "type": "json", "json": { /* ツール固有データ */ } }
  ]
}

// 診断は構造化されて返される（tool エラー扱いしない）
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

// ツールレベルエラー（例：parse_source が不正な UTF-8 を受信）
{
  "schemaVersion": "1.0",
  "isError": true,
  "content": [{ "type": "text", "text": "MCP-INVALID-INPUT: source は有効な UTF-8 ではありません" }],
  "errorCode": "MCP-INVALID-INPUT"
}
```

**エラー体系**：

- **診断（diagnostic）**：解析 / 型エラー、RFC-013（`E0001` など）に従う——**tool エラー扱いしない**
- **ツールレベルエラー**：`MCP-`
  接頭辞を使用（`MCP-INVALID-INPUT`、`MCP-PROJECT-NOT-FOUND`、`MCP-INTERNAL`）——`isError: true` 扱い
- **panic / crash**：JSON-RPC `-32603 Internal error`、サーバーは終了しない

**パス解決ルール**（`lookup_symbol` / `find_references` の `workspace_root`、`typecheck` の
`file_paths` に適用）：

1. コマンドライン `--project-root <dir>` が最優先（デフォルトを上書き）
2. 次に：cwd から上方へ `yaoxiang.toml` をファイルシステムルートまで探索（RFC-015 に従う）
3. それもなし：cwd 自身
4. `file_paths` はプロジェクトルート内に収まらなければならない（パストラバーサル防止）；越境 →
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

- **loopback のみリッスン**（127.0.0.1 / ::1）；パブリックバインドは明示的に拒否してエラー終了
- HTTP **認証なし**（loopback はデフォルトで信頼）；将来 `--require-token <hex>`
  フィールドを追加予定
- stdio サブプロセスモードは自然に隔離（parent プロセスが権限を制御）

### マルチプロセスと並行性

各 `yaoxiang mcp` プロセスは `World` を 1 つ保持し、共有しない：

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
agent は「サブプロセス起動」を設定する——自然にポート競合ゼロ。HTTP モードではユーザーがポート割り当てを管理する必要がある。
**World 隔離**：各プロセスが独立して LSP 同期状態を保持——ある MCP プロセスがクラッシュしても LSP
/ 他の MCP プロセスには**影響しない**。**将来の Sessions**：v2 で同じプロセス内の複数ワークスペースのディスパッチ（同一プロセス内の複数
`Session`）を検討——**本 RFC では行わない**。

## 詳細設計

### データ構造

`src/mcp/project.rs` を新規追加：

```rust
pub struct ProjectRoot {
    /// 絶対パス
    pub root: PathBuf,
    /// プロジェクトルート識別の戦略ソース
    pub source: ProjectRootSource,
}

pub enum ProjectRootSource {
    CliFlag,           // yaoxiang mcp --project-root
    AutoDetected,      // 上方へ yaoxiang.toml を探索
    FallbackCwd,       // cwd にフォールバック
}

pub struct ResolvedPath {
    /// プロジェクトルートからの相対パス（AI 読み取り推奨）
    pub relative: String,
    /// 解決後の絶対パス（World 操作用）
    pub absolute: PathBuf,
}

impl ProjectRoot {
    /// "file_path" を安全なパスに解決する——パストラバーサル防止
    pub fn resolve(&self, file_path: &str) -> Result<ResolvedPath, McpError>;
}
```

`ProjectRoot` シングルトン + `src/mcp/schema.rs` でツールスキーマを自動生成：

```rust
pub struct ProjectRoot {
    /// 絶対パス（`yaoxiang.toml` を含むか、後方互換のためフォールバック）
    pub root: PathBuf,
    pub source: ProjectRootSource,
}

impl ProjectRoot {
    /// CLI 起動時に一度だけ識別し、結果を `McpServer` コンテキストにキャッシュ——全ツールで再利用
    pub fn detect(cli_override: Option<PathBuf>) -> Result<Self, McpError>;
}
```

ツールスキーマは `schemars` crate を使って input struct から自動生成し、手書き JSON
Schema のずれを防ぐ：

```rust
#[derive(Deserialize, schemars::JsonSchema)]
pub struct ParseSourceInput {
    /// 完全な YaoXiang ソースコード片——**ディスクに保存しない**、純粋に transient
    pub source: String,
    pub tab_size: Option<u32>,
}
```

**`parse_source` / `format_source` のツールスキーマには `file_path`
フィールドがない**——この 2 ツールは文字列ソースのみを受け取り、プロジェクトセマンティクスに参加しない。`lookup_symbol`
/ `find_references` / `typecheck` は `workspace_root` または `file_paths`
を受け取る（必須かどうかはツール表参照）。

### コンパイラ変更

| モジュール                             | 変更                                                                                            |
| -------------------------------------- | ----------------------------------------------------------------------------------------------- |
| `src/lsp/world.rs`                     | **変更なし**——MCP 起動時に既存の LSP `World::load_*` API を呼び出してワークスペースを一度ロード |
| `src/lsp/handlers/workspace_symbol.rs` | **変更なし**——`mcp/tools/lookup.rs` がラッパーで `query` を LSP 入力に変換                      |
| `src/lsp/handlers/references.rs`       | **変更なし**——同上                                                                              |
| `src/lsp/handlers/formatter.rs`        | **変更なし**——`format_source` が直接呼び出し                                                    |
| `src/main.rs`                          | `Mcp` サブコマンド分岐を追加                                                                    |
| `Cargo.toml`                           | `mcp-server` feature を追加（またはメインバイナリに常に含める）                                 |
| `src/util/diagnostic/`                 | **変更なし**（RFC-017 で実装済み）                                                              |

**重要な制約**：`src/mcp/` は `src/lsp/`
のプライベートシンボルへの**逆依存を許可しない**——`crate::lsp::`
の公開 API 経由でのみ handler を呼び出せる。

### 後方互換性

- ✅ **完全に後方互換**：新サブコマンド `yaoxiang mcp`、`yaoxiang` / `yaoxiang lsp`
  の既存挙動は変えない
- ✅ **LSP サーバーは不変**：RFC-017 で実装されたすべての能力、API、内部状態は変えない
- ✅ **lib crate 公開 API は不変**：すべての `pub`
  パスは変えない；MCP は既存 API を消費するのみ——新規 `pub` メソッドは**ゼロ**

### 既存システムとの統合

| 既存モジュール                            | MCP 統合方法                                                                                            |
| ----------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| `src/frontend/lexer`                      | `parse_source` が lexer を直接呼び出し                                                                  |
| `src/frontend/core/parser`                | `parse_source` が parser を直接呼び出し；失敗時は `Missing*` ノードを生成（RFC-017）                    |
| `src/frontend/core/typecheck/inference/*` | `typecheck` が `collect_diagnostics` パターンを再利用（RFC-017 §問題 1）                                |
| `src/middle/`                             | `typecheck` がすべての middle pass を実行（依存解析など）                                               |
| `src/lsp/world.rs`                        | 起動時に `World::load_*` API を呼び出し（既存）；World はいかなる「仮想ドキュメント」も**受け付けない** |
| `src/lsp/handlers/workspace_symbol.rs`    | `mcp/tools/lookup.rs` がラッパーで `query: String` を LSP 入力に変換（名前検索）                        |
| `src/lsp/handlers/references.rs`          | `mcp/tools/find_refs.rs` がラッパーで `query: String` を LSP 入力に変換                                 |
| `src/lsp/handlers/formatter.rs`           | `mcp/tools/format.rs` が直接呼び出し（未実装なら新規 `formatter::format_with_diff` を追加）             |
| `src/util/i18n/`                          | エラーメッセージは多言語リソースファイル（zh-CN / en）を使用                                            |

### エラーハンドリング

| ソース                                  | 処理                                                                                             |
| --------------------------------------- | ------------------------------------------------------------------------------------------------ |
| 解析エラー                              | `Diagnostic{code:"E0xxx", severity, message, span}`（**tool エラーではない**、content 内で返す） |
| 型エラー                                | 同上                                                                                             |
| `file_paths` 越界（`typecheck` ツール） | ツールレベルエラー `MCP-PATH-OUTSIDE-PROJECT`                                                    |
| `source` 不正 UTF-8                     | ツールレベルエラー `MCP-INVALID-INPUT`                                                           |
| ツール panic                            | JSON-RPC `-32603 Internal error`；サーバーは**終了しない**                                       |
| クライアントが非 JSON-RPC を送信        | 直接ストリーム切断（stdio EOF）、再起動で新セッション                                            |

診断の重大度レベルは RFC-017（実装済み）の `enum ErrorKind { Error, Warning, Note }` に従う。

### テスト戦略

| 層              | テスト                                                                                                            |
| --------------- | ----------------------------------------------------------------------------------------------------------------- |
| **Unit**        | `src/mcp/project.rs::resolve` のパストラバーサル、`src/mcp/schema.rs` のスキーマ検証                              |
| **Integration** | stdio モック：サーバーを起動し、stdin に JSON-RPC を流し、stdout から応答を読み、fixture と比較                   |
| **E2E**         | 実プロセス `yaoxiang mcp` を実行、Claude Code スタイルのツール呼び出しチェーン：parse → 修正 → format → typecheck |
| **Fuzz**        | MCP JSON-RPC 解析の `cargo-fuzz`（libFuzzer ハーネス）                                                            |

各ツールは少なくとも 1 つの happy path + 1 つの diagnostic シナリオ +
1 つの tool-error シナリオの integration テストを持つこと。

## トレードオフ

### 利点

- **再利用コストが極めて低い**：`World` / `Session` / `handlers`
  / 診断収集はすべて実装済み（RFC-017）、本 RFC は「MCP シェルを 1 層被せる」だけ
- **AI-First インターフェース**：ツール契約は LSP より 3-5 倍直感的；LLM がスキーマを直接読む
- **マルチプロセス隔離**：LSP エディタセッションや他の MCP プロセスから分離、**ロック競合ゼロ**
- **stdio フレンドリー**：すべての主要 AI agent がデフォルトでサブプロセスモード、設定不要で統合可能
- **YAGNI 達成**：本 RFC では Resources、Sessions、クロスプロセス状態、リモート MCP を廃止——v2 で再開

### 欠点

- **プロトコル分裂**：将来 LSP / MCP / DAP の 3 プロトコルが個別に進化し、一貫性維持コストが発生
- **HTTP モードは二の次**：loopback 制限でローカルツールとして位置付け、リモートシナリオは v2 で再設計が必要
- **parse の重複コスト**：AI がソースを繰り返し微調整して `parse_source` を呼ぶと lexer +
  parser が再実行される。**緩和策**：RFC-017 の `DocumentCache`
  が**ディスク上**の同ソースの 2 回目解析を高速化できる；純粋な transient ソースの 1 回限りの解析は不可避
- **テストカバレッジコスト**：5 ツール × 3 シナリオ = 最低 15 の integration テスト

## 代替案

| 案                                                                | 採用しない理由                                                                                     |
| ----------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| **プロセス内に 2 プロトコルを埋め込む**（LSP + MCP リスナー共存） | stdin / stdout は 1 消費者しか持てない；HTTP も共存が必要——複雑度 > 利益                           |
| **MCP を LSP-client ブリッジにする**                              | IPC が 1 層増える；LSP 設計上、名前でシンボル検索をサポートしない——MCP の欲しい能力は LSP 提供不可 |
| **gRPC / カスタムプロトコル**                                     | 事実標準から逸脱；MCP SDK（TypeScript、Python、Rust）がコミュニティに存在し、エコシステムあり      |
| **LSP handler の全能力を再利用**（L3 ツールセット）               | 多大な position ↔ intent 適応作業が必要；限界効用が逓減                                            |
| **最初のバージョンは HTTP のみ**（stdio なし）                    | Claude Code / Continue は stdio がデフォルト、障壁が高すぎる                                       |

## 実装戦略

### 依存関係

- **強い依存**：RFC-017 LSP 実装（実装済み）
- **強い依存**：RFC-013 エラーコード体系（実装済み）
- **強い依存**：RFC-014 / RFC-015 プロジェクトルート識別（部分実装済み）
- **新規依存**（Rust crate）：
  - `mcp-rust-sdk`（要評価、[modelcontextprotocol/rust-sdk](https://github.com/modelcontextprotocol/rust-sdk)
    参照）
  - `tokio`（**既存**、optional feature）
  - `axum`（HTTP モード）または `hyper` 直接——要評価
- **言語仕様の変更はゼロ**：純粋にツールチェーンの追加

### フェーズ（#154 と同期）

| フェーズ                               | 内容                                                                                                                                                                                                                                  | 期間見積   |
| -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- |
| **v0.8.x (MVP)**                       | `src/mcp/{mod.rs, server.rs, transport/stdio.rs, project.rs, schema.rs}` + `parse_source` + `format_source` + `lookup_symbol` + `find_references` + `typecheck`（**5 ツール**）+ `yaoxiang mcp` サブコマンド + 起動時 `World::load_*` | **3-4 週** |
| **v0.9.x (YaoXiang インテリジェンス)** | `+ explain_diagnostic`（`render_explain_output` を**直接呼び出し**）+ `+ list_imports`（`ModuleGraph::validate_imports` をラップ）+ ユニット / 統合テスト                                                                             | **1-2 週** |
| **v0.10.x (AST + HTTP)**               | `+ rename_symbol`（`src/middle/rename.rs` を**新規追加**、AST 書き換え）+ streamable HTTP トランスポート + パフォーマンス調整（`parse_source` P99 < 100ms）                                                                           | **2-3 週** |

**なぜ 3 フェーズに分かれるか**：MVP でまず stdio +
5 ツールでインターフェース設計を検証；v0.9.x で低リスクでゼロ適応の「YaoXiang 固有」ツールで統合を検証；v0.10.x でようやく高リスクの「AST 書き換え」新モジュールを開く（独立 PR レビューで集中）。

### リスク

1. **`mcp-rust-sdk`
   のメンテナンス活発度**：2025 年リリース、API が激変する可能性。**緩和策**：安定性を評価し、不安定なら軽量 JSON-RPC
   2.0 + tool dispatcher を自前実装（< 500 行）
2. **parse の重複コスト**：AI がソースを繰り返し微調整して `parse_source` を呼ぶと lexer +
   parser が再実行される。**緩和策**：RFC-017 の `DocumentCache`
   が**ディスク上**の同ソースの 2 回目解析を高速化できる；純粋な transient ソースの 1 回限りの解析は不可避
3. **AI
   agent のスキーマ互換性**：agent によって MCP スキーマの厳格度が異なる。**緩和策**：`schemars`
   crate を使って Rust input 構造からスキーマを自動生成、手書きによるずれをゼロに
4. **マルチプラットフォームのパス解決**：Windows のパス大文字小文字無視、UNC パス、`\\`
   境界。**緩和策**：パス解決に `std::path` の代わりに `camino::Utf8Path` を使用
5. **MCP ツールスキーマと LSP 入力が 1:1 でない**：LSP `workspace_symbol` は `(query)`
   を受ける；既存 handler を再利用するには内部 LSP に位置 +
   URI でラップする必要がある。**緩和策**：`mcp/tools/lookup.rs`
   にアダプタ層を設け、詳細を MCP 側に閉じ込める
6. **`rename_symbol` AST 書き換えと LSP `rename` のセマンティクス差異**：LSP `textDocument/rename`
   は URI + 位置 + new_name → WorkspaceEdit；MCP `rename_symbol` は source + old_name + new_name
   → 新 source。**直接再利用不可**。**緩和策**：`src/middle/rename.rs`
   を別途実装、scope-aware に参照を書き換え、LSP handler 実装と干渉させない

## オープンな課題

- [ ] `mcp-rust-sdk` を採用するか自前実装か？（@Chen Xu：まず rust-sdk の 6 月版を評価してから決定）
- [ ] HTTP 認証パス？（v0.10 RFC で別途提案）
- [ ] 起動時に MCP が `tools/list`
      を出力して AI に能動的に発見させる必要があるか？（MCP 標準で要求、**デフォルト実装**）
- [ ] `typecheck` は `mode: "fast|full"`（fast = 現在のファイルサブセットのみ、full
      = ワークスペース全体）をサポートすべきか？
- [ ] パフォーマンス予算 `parse_source` P99 < 100ms は現実的か？（RFC-017 で実装済みの
      `DocumentCache` を source-string モードでベンチマーク要）

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

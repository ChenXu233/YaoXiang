---
title: 'RFC-035: MCP サーバーサポート（AI Agent 統合）'
status: '草案'
author: '晨煦'
created: '2026-07-11'
updated: '2026-07-11'
issue: '#154'
---

# RFC-035: MCP サーバーサポート（AI Agent 統合）

## 概要

YaoXiang に MCP（Model Context Protocol）サーバーを追加し、AI agent（Claude
Code、Continue、Cody、Zed など）が YaoXiang ソースコードの
**AST、解析エラー、型、 Symbol、参照、フォーマット結果**を直接クエリできるようにする。RFC-017 で既に実装済みの `World` バックエンドを再利用し、`yaoxiang mcp` サブコマンドを追加、単一バイナリでデュアルモード、 멀티프로세스로 독립적인 World。

## 動機

### なぜこの機能が必要なのか？

RFC-017 は YaoXiang をエディタから理解できるようにした（hover / goto-def /
completion）。しかし LSP は**位置駆動**プロトコルである：

- 各リクエストは `textDocument` URI + `Position` に強く依存する
- エディタはまずファイルを開き、保存し、LSP サーバーとの長接続を維持する必要がある
- AI agent のワークフローは**コードスニペット**：会話内で「コードを貼り付けて」質問する、**先に保存しない**

AI agent が実際に使用できる LSP クライアント（vscode-langservers-extracted、`mcp-lsp-bridge`
的なプロジェクト）はどちらも **L1 のみ翻訳**：goto-def、hover。AI がやりたいことは：

- 「このコードは**解析正しいか**」——parse + 完全な diagnostic フローが必要
- 「この Symbol は**ファイル内でどう使われているか**」——lookup_symbol を名前でクエリ必要
- 「このコードは**フォーマット後どう見えるか**」——format_source 必要
- 「**全部の**型エラーはどこか」——typecheck で全ワークスペース実行必要

これらの L1 LSP 翻訳能力では**做不到**、LSP の設計上サポートしていないため。

### 現在の問題

1. AI agent が LSP を使用するのは体験が悪い：モックドキュメントが必要、JSON が巨大、強URI依存
2. YaoXiang プロジェクトに「AI-First」インターフェース層がない：人間は IDE で LSP を使うが、AI agent は LSP を使えない
3. Claude Code / Continue など主要な AI agent はデフォルトで MCP をサポートしており、YaoXiang にとっては未開拓のエコシステム

### MCP とは？

MCP（Model Context Protocol）は 2024-2025 年に Anthropic が主導して公開・オープンソース化した AI
agent ツール呼び出しプロトコルで、既に事実上の標準になっている（OpenAI、Google、Microsoft、Zed、Continue、Cody などが対応）。特徴：

- JSON-RPC 2.0 ベース（LSP と同源）
- 3つのプリミティブ：**Tools**（アクション）、Resources（データ）、Prompts（テンプレート）
- トランスポート：`stdio`（子プロセス）/ streamable `HTTP` / SSE
- ツールの入出力には **JSON Schema** の厳格な型付け（LLM に優しい）
- 2025-06 以降 streamable HTTP 仕様がリリースされており、本 RFC は旧 SSE との互換性も持つ

**本 RFC は Tools プリミティブのみ使用**——LSP の「サービスを提供」と整合し、Resources のファイルモデル複雑性を導入しない。

## 提案

### コアデザイン

単一バイナリでデュアルモード：

```text
┌─────────────────────────────────────────────────────────┐
│                    yaoxiang（v0.7.7+）                  │
│  ┌─────────────────┐      ┌──────────────────────────┐  │
│  │ yaoxiang lsp    │      │   yaoxiang mcp           │  │
│  │ (stdio JSON-RPC)│      │   (stdio default         │  │
│  │ RFC-017 実装済  │      │    + HTTP optional)      │  │
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
│  │  ├── tools/          （6つの tool handler）       │  │
│  │  ├── schema.rs       （入出力 JSON Schema）       │  │
│  │  └── project.rs      （プロジェクトルート認識 + パス解決）│  │
│  └──────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

**主要な意思決定**：

- **同一バイナリ**：`yaoxiang` はサブコマンドで切り替え；LSP プロセスと MCP プロセスは同一ランタイムで**共存しない**
- **マルチプロセス独立 World**：各 `yaoxiang mcp` プロセスは独立した
  `World` を保持；LSP プロセス、他の MCP プロセスとは互いに影響しない（ロック競合なし、独立したクラッシュ隔離）
- **stdio デフォルト**：ポート衝突を避け、ネットワーク設定不要；HTTP はオプションの代替手段
- **再利用而非重复**：直接 `yaoxiang::frontend` / `yaoxiang::middle` / `yaoxiang::lsp::handlers`
  の lib API を呼び出す、**LSP-client 中継を経由しない**

### ツールセット（8つのツール、3フェーズで交付）

「特殊ケースの排除 + 段階的リリース」原則で設計：純粋ソース stateless ツールが先で、ワークスペースツールは LSP
World を共有、AST 書き換えツールは独立して新規追加。

| Tool 名称            | 入力                                                                                            | 出力                                                         | 再利用                                                         | フェーズ      |
| -------------------- | ----------------------------------------------------------------------------------------------- | ------------------------------------------------------------ | ------------------------------------------------------------- | ----------- |
| `parse_source`       | `source: String`, `tab_size?: u32`                                                              | `{ast: Node, diagnostics: Diagnostic[]}`                     | 直接 `frontend::parse` を呼び出す                              | v0.8.x      |
| `format_source`      | `source: String`, `tab_size?: u32`                                                              | `{formatted: String, diff: Hunk[]}`                          | 直接 `formatter::format` を呼び出す                            | v0.8.x      |
| `lookup_symbol`      | `query: String`, `workspace_root?: String`, `kind?: SymbolKind[]`                               | `{symbols: Symbol[]}`                                        | `lsp::handlers::workspace_symbol` を再利用（`query` 模糊一致） | v0.8.x      |
| `find_references`    | `query: String`, `workspace_root?: String`                                                      | `{locations: Location[]}`                                    | `lsp::handlers::references` を再利用（`query` 而非位置）       | v0.8.x      |
| `typecheck`          | `file_paths: String[]`, `project_root: String`                                                  | `{diagnostics: Diagnostic[], summary: Counts}`               | `lsp::world::typecheck_full` を再利用                          | v0.8.x      |
| `explain_diagnostic` | `code: String`（例 `E0001`），`lang?: String`                                                   | `{code, category, title, description, example, help}`        | **`直接`** `util::diagnostic::command::render_explain_output` を呼び出す | **v0.9.x**  |
| `list_imports`       | `file_path: String`, `project_root?: String`                                                    | `{imports: [{module, items, source_file}]}`                    | `middle::passes::module::ModuleGraph::validate_imports` を再利用  | **v0.9.x**  |
| `rename_symbol`      | `source: String`, `old_name: String`, `new_name: String`, `scope?: "module" \| "function:name"` | `{source: String, edits: Edit[], diagnostics: Diagnostic[]}` | **`新規追加**` `src/middle/rename.rs`（AST 書き換え）           | **v0.10.x** |

**8つのツールの境界**：

- `parse_source` / `format_source` —— **純粋ソース stateless**、World に入らない
- `lookup_symbol` / `find_references` —— `workspace_root` を受け取る（渡さない場合は起動時の `--project-root` を使用）
- `typecheck` —— `file_paths` **必須**、ワークスペースの完全性を保証
- `explain_diagnostic` —— **ファイル依存ゼロ**、純粋文字列でエラーコードレジストリをクエリ
- `list_imports` —— `file_path` は物理ファイルでそのファイルの import 解析結果を出力
- `rename_symbol` —— **純粋ソース AST 書き換え**、LSP スタイルの位置クエリを行わない（既存の `lsp::handlers::rename` とはセマンティクスが異なる）
- ~~`hover` / `completion` / `signature_help`~~ —— **すべて切り捨て**：AI agent は「位置Sensitive」セマンティクスを行わない、`lookup_symbol` で名前クエリ代替

**World ロードタイミング**：サーバー起動時に `--project-root` で `yaoxiang.toml` と
`src/**/*.yx` をスキャンし、RFC-017 で既に実装済みの `World::load_*` API を使用して
`World.documents` に一括ロード。**新しい lib API は追加しない**。

### ツールコントラクト

**入力**：JSON Schema で記述、各フィールドには `description` + `examples` 付き（LLM が自動的に理解）。

**出力**：構造化 JSON、`schemaVersion: "1.0"` フィールドを統一で付与：

```jsonc
// 成功応答
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

// ツールレベルのエラー（例：parse_source が不正な UTF-8 を受け取る）
{
  "schemaVersion": "1.0",
  "isError": true,
  "content": [{ "type": "text", "text": "MCP-INVALID-INPUT: source は有効な UTF-8 ではありません" }],
  "errorCode": "MCP-INVALID-INPUT"
}
```

**エラー体系**：

- **診断（diagnostic）**：解析/型エラー、RFC-013（`E0001` など）を流用——**tool エラーとは数えない**
- **ツールレベルエラー**：`MCP-`
  プレフィックス（`MCP-INVALID-INPUT`、`MCP-PROJECT-NOT-FOUND`、`MCP-INTERNAL`）——`isError: true` として扱う
- **panic/crash**：JSON-RPC `-32603 Internal error`、サーバーは終了しない

**パス解決ルール**（`lookup_symbol` / `find_references` の `workspace_root`、`typecheck` の
`file_paths` に適用）：

1. コマンドライン `--project-root <dir>` が最優先（デフォルトをオーバーライド）
2. それ以外：cwd から上方に向かってファイルシステムルートまで `yaoxiang.toml` を探す（RFC-015 を流用）
3. それ以外：cwd 自身
4. `file_paths` はプロジェクトルート内に存在する必要がある（穿越防止）；範囲外 → `MCP-PATH-OUTSIDE-PROJECT`

### トランスポート層

**stdio（デフォルト）**：

```bash
yaoxiang mcp
# 起動後 stdin から JSON-RPC を読み取り、stdout に書き込み、stderr はログ用
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
yaoxiang mcp --http --sse --addr 127.0.0.1:7325  # 旧 SSE との互換性（v0.10）
```

**セキュリティ制約**：

- **loopback のみリスン**（127.0.0.1 / ::1）；パブリックネットバインディングは明示的に拒否してエラー終了
- HTTP **認証なし**（loopback はデフォルトで信頼）；将来の `--require-token <hex>` フィールド追加予定
- stdio 子プロセスモードは自然に隔離（親プロセスが権限を制御）

### マルチプロセスと並行処理

各 `yaoxiang mcp` プロセスは独立した `World` を保持し、共有しない：

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

**ポート衝突**：AI agent が「子プロセスを起動」するよう設定——ポート衝突が本質的にゼロ。HTTP モードではユーザーがポート割り当てを管理する必要がある。
**World 隔離**：各プロセスは独立した LSP 同期状態を保持——1つの MCP プロセスがクラッシュしても **LSP/他の MCP プロセスを影響しない**。 **future Sessions**：v2 で初めてマルチワークスペースディスパッチ（同プロセス内の複数の `Session`）を検討、**本 RFC では行わない**。

## 詳細設計

### データ構造

新規 `src/mcp/project.rs`：

```rust
pub struct ProjectRoot {
    /// 絶対パス
    pub root: PathBuf,
    /// ロード時にプロジェクトルートを識別した戦略ソース
    pub source: ProjectRootSource,
}

pub enum ProjectRootSource {
    CliFlag,           // yaoxiang mcp --project-root
    AutoDetected,      // 上方へ yaoxiang.toml を探す
    FallbackCwd,       // cwd にフォールバック
}

pub struct ResolvedPath {
    /// プロジェクトルートからの相対パス（AI に読ませる推奨）
    pub relative: String,
    /// 解決後の絶対パス（World 操作に使用）
    pub absolute: PathBuf,
}

impl ProjectRoot {
    /// 「file_path」を安全パスに解決——穿越防止
    pub fn resolve(&self, file_path: &str) -> Result<ResolvedPath, McpError>;
}
```

`ProjectRoot` シングルトン + `src/mcp/schema.rs` ツール schema 自動生成：

```rust
pub struct ProjectRoot {
    /// 絶対パス（`yaoxiang.toml` を含むか、下位互換フォールバック必須）
    pub root: PathBuf,
    pub source: ProjectRootSource,
}

impl ProjectRoot {
    /// CLI 起動時に1回識別、結果を `McpServer` コンテキストにキャッシュ——全ツールが再利用
    pub fn detect(cli_override: Option<PathBuf>) -> Result<Self, McpError>;
}
```

ツール schema は `schemars` crate を使用して input struct から自動生成。手書き JSON Schema ドリフトを避ける：

```rust
#[derive(Deserialize, schemars::JsonSchema)]
pub struct ParseSourceInput {
    /// 完全な YaoXiang ソーススニペット——**ディスクに保存しない**、純粋 transient
    pub source: String,
    pub tab_size: Option<u32>,
}
```

**`parse_source` / `format_source` ツール schema には `file_path` フィールドがない**——この2つのツールは文字列ソースのみ受け入れ、プロジェクトセマンティクスに関与しない。`lookup_symbol` / `find_references` / `typecheck`
は `workspace_root` または `file_paths` を受け入れる（必須かどうかはツール表参照）。

### コンパイラの更改

| モジュール                                   | 更改内容                                                                    |
| -------------------------------------------- | --------------------------------------------------------------------------- |
| `src/lsp/world.rs`                           | **変更ゼロ**——MCP 起動時に LSP 既存の `World::load_*` API を使用してワークスペースを一括ロード |
| `src/lsp/handlers/workspace_symbol.rs`       | **変更ゼロ**——`mcp/tools/lookup.rs` で包んで `query` を LSP 入参に変換      |
| `src/lsp/handlers/references.rs`             | **変更ゼロ**——同上                                                         |
| `src/lsp/handlers/formatter.rs`              | **変更ゼロ**——format_source を直接呼び出す                                 |
| `src/main.rs`                                | `Mcp` サブコマンドブランチを追加                                            |
| `Cargo.toml`                                 | `mcp-server` feature を追加（またはメインビナリは常に含む）                 |
| `src/util/diagnostic/`                       | **変更ゼロ**（RFC-017 で既に実装済み）                                      |

**主要な制約**：`src/mcp/` は `src/lsp/` の private  Symbol を**逆方向に依存することを許可しない**——`crate::lsp::`
の public API を通じてのみ handlers を呼び出す。

### 後方互換性

- ✅ **完全後方互換**：新しいサブコマンド `yaoxiang mcp`、`yaoxiang` / `yaoxiang lsp` の既存の動作は一切変更しない
- ✅ **LSP サーバーは変更なし**：RFC-017 実装の全 capability、API、内部状態は変更なし
- ✅ **lib crate の public API は変更なし**：すべての `pub` パスは変更なし；MCP は既存の API のみ消費——**ゼロ**新增 `pub` メソッド

### 既存システムとの統合

| 既存モジュール                                | MCP 統合方式                                                                |
| --------------------------------------------- | ---------------------------------------------------------------------------|
| `src/frontend/lexer`                          | parse_source が直接 lexer を呼び出す                                        |
| `src/frontend/core/parser`                    | parse_source が直接 parser を呼び出す；失敗時は `Missing*` ノードを生成（RFC-017）|
| `src/frontend/core/typecheck/inference/*`     | typecheck が `collect_diagnostics` パターンを再利用（RFC-017 §問題1）        |
| `src/middle/`                                 | typecheck が全 middle pass を実行（依存性分析など）                          |
| `src/lsp/world.rs`                            | 起動時に `World::load_*` API を呼び出す（既存）；World は「仮想ドキュメント」を受け入れない |
| `src/lsp/handlers/workspace_symbol.rs`         | `mcp/tools/lookup.rs` が包んで、`query: String` を LSP 入参に変換（名前クエリ）|
| `src/lsp/handlers/references.rs`               | `mcp/tools/find_refs.rs` が包んで、`query: String` を LSP 入参に変換        |
| `src/lsp/handlers/formatter.rs`                | `mcp/tools/format.rs` が直接呼び出す（未実装の場合は新規 `formatter::format_with_diff` を追加）|
| `src/util/i18n/`                              | エラーメッセージは多言語リソースファイルに従う（zh-CN/en）                    |

### エラー処理

| ソース                                  | 処理                                                                                       |
| --------------------------------------- | ------------------------------------------------------------------------------------------ |
| 解析エラー                              | `Diagnostic{code:"E0xxx", severity, message, span}`（**tool エラーではない**、content 内で返す）|
| 型エラー                                | 同上                                                                                       |
| `file_paths` 範囲外（`typecheck` ツール）| tool レベルエラー `MCP-PATH-OUTSIDE-PROJECT`                                                |
| `source` が不正な UTF-8                  | tool レベルエラー `MCP-INVALID-INPUT`                                                      |
| ツール panic                            | JSON-RPC `-32603 Internal error`；サーバーは**終了しない**                                  |
| クライアントが非 JSON-RPC を送信         | ストリームを切断（stdio EOF）、再起動で新規セッション                                        |

診断重大度レベルは RFC-017（既に実装済み）の `enum ErrorKind { Error, Warning, Note }` を流用。

### テスト戦略

| レイヤー            | テスト                                                                                    |
| --------------- | --------------------------------------------------------------------------------------- |
| **Unit**        | `src/mcp/project.rs::resolve` パス穿越、`src/mcp/schema.rs` schema 検証                 |
| **Integration** | mock stdio：サーバーを起動し、stdin に JSON-RPC を投入、stdout から応答を読み取り、fixture と比較 |
| **E2E**         | `yaoxiang mcp` 真プロセスを実行、Claude Code スタイルのツール呼び出しチェーン：parse → 修正 → format → typecheck |
| **Fuzz**        | MCP JSON-RPC 解析の `cargo-fuzz`（libFuzzer harness）                                   |

各 tool には少なくとも 1 つの happy path + 1 つの diagnostic シナリオ +
1 つの tool-error シナリオの integration テストが必要。

## トレードオフ

### 利点

- **再利用コスト極めて低い**：`World` / `Session` / `handlers`
  / 診断収集は全て既に実装済み（RFC-017）、本 RFC は「MCP シェルを追加する」だけ
- **AI-First インターフェース**：tool コントラクトは LSP より 3-5 倍直感的；LLM が schema を直接読める
- **マルチプロセス隔離**：LSP エディタセッションおよび他の MCP プロセスと分離、**ロック競合ゼロ**
- **stdio フレンドリー**：すべての主要 AI agent がデフォルトでサブプロセスモード、ゼロ設定で統合可能
- **YAGNI 通過**：本 RFC は Resources、Sessions、クロスプロセス状態、リモート MCP を切り捨て——v2 で再開

### 欠点

- **プロトコル分裂**：将来的に LSP / MCP / DAP の3套プロトコルがそれぞれ進化、一貫性維持コスト
- **HTTP モードは二級市民**：loopback 制限はローカルツールとして位置づけ、リモートシナリオは v2 で再設計が必要
- **重复 parse オーバーヘッド**：AI がソースを繰り返し微調整して `parse_source` を呼び出すと、lexer+parser が再実行される。**緩和**：RFC-017 の `DocumentCache`
  に依存すれば、ディスク上の同一ソースの2回目解析を高速化できる；純粋 transient source の1回解析は不可避
- **テストカバレッジコスト**：5つの tool × 3つのシナリオ = 15の integration テストから開始

## 代替案

| 案                                         | 選択しない理由                                                           |
| -------------------------------------------- | ----------------------------------------------------------------------- |
| **プロセス内へのデュアルプロトコル埋め込み**（LSP+MCP listener 共存） | stdin/stdout は1つのコンシューマしか持てない；HTTP も共存させる必要——複雑さ > 利益 |
| **MCP を LSP-client ブリッジとして活用**     | IPC がもう1層増える；LSP は名前でクエリする Symbol をサポートしていない——MCP が欲しい capability を LSP は提供できない |
| **gRPC / カスタムプロトコルを使用**          | 事実上の標準から逸脱；コミュニティには MCP SDK（TypeScript、Python、Rust）が既にあり、エコシステムが付属 |
| **LSP handler の全 capability を再利用**（L3 ツールセット）| 大量の位置↔意図のアダプテーション作業；限界効用逓減                      |
| **最初のバージョンは HTTP のみ**（stdio なし）| Claude Code / Continue などはデフォルトで stdio、敷居が高すぎる          |

## 実装戦略

### 依存関係

- **強依存**：RFC-017 LSP 実装（既に実装済み）
- **強依存**：RFC-013 エラーコード体系（既に実装済み）
- **強依存**：RFC-014 / RFC-015 プロジェクトルート識別（部分是既に実装済み）
- **新規依存**（Rust crate）：
  - `mcp-rust-sdk`（評価待ち、
    [modelcontextprotocol/rust-sdk](https://github.com/modelcontextprotocol/rust-sdk) を参照）
  - `tokio`（**既にあり**、optional feature）
  - `axum`（HTTP モード）または `hyper` 直接——評価待ち
- **言語仕様変更ゼロ**：純粋なツールチェーン増分

### 実施フェーズ

| フェーズ                       | 内容                                                                                                                                                                                                                          | 期間見積もり   |
| -------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- |
| **v0.8.x (MVP)**           | `src/mcp/{mod.rs, server.rs, transport/stdio.rs, project.rs, schema.rs}` + `parse_source` + `format_source` + `lookup_symbol` + `find_references` + `typecheck`（**5ツール**）+ `yaoxiang mcp` サブコマンド + 起動時の `World::load_*` | **3-4週間** |
| **v0.9.x (YaoXiang インテリジェンス)** | `+ explain_diagnostic`（**直接** `render_explain_output` を呼び出す）+ `+ list_imports`（`ModuleGraph::validate_imports` を包装）+ unit/integration テスト                                                                                          | **1-2週間** |
| **v0.10.x (AST + HTTP)**   | `+ rename_symbol`（**新規追加** `src/middle/rename.rs`、AST 書き換え）+ streamable HTTP transport + パフォーマンスチューン（parse_source P99 < 100ms）                                                                                              | **2-3週間** |

**なぜ3フェーズに分けるのか**：MVP でまず stdio +
5ツールを通じてインターフェース設計の妥当性を検証；v0.9.x でリスクが低い"YaoXiang 固有"ツールを追加して統合の正しさを検証；v0.10.x でリスクの高い「AST 書き換え」の新規モジュールを開く（独立 PR レビューがより焦点を当てやすい）。

### リスク

1. **`mcp-rust-sdk`
   メンテナンス状況**：2025年にやっとリリース、API が激しく変わる可能性がある。**緩和**：評価して不安定であれば軽量な JSON-RPC 2.0 +
   tool dispatcher を自作（< 500行）
2. **重复 parse オーバーヘッド**：AI がソースを繰り返し微調整して `parse_source` を呼び出すと、lexer+parser が再実行される。**緩和**：RFC-017 の `DocumentCache`
   に依存すれば、ディスク上の同一ソースの2回目解析を高速化できる；純粋 transient source の1回解析は不可避
3. **AI agent schema 互換性**：異なる agent の MCP schema 厳格度は異なる。**緩和**：`schemars`
   crate を使用して Rust input 構造から schema を自動生成、手書きドリフトゼロ
4. **パス解決のマルチプラットフォーム**：Windows パスは大小文字を区別しない、UNC パス、`\\` 境界。**緩和**：パス解決には
   `camino::Utf8Path` を使用して `std::path` を置き換え
5. **MCP ツール schema と LSP 入参が 1:1 でない**：LSP `workspace_symbol` は
   `(query)` を受け取る；LSP 内部に渡すには、既存の handler を再利用するために位置+URI に包装する必要がある。**緩和**：
   `mcp/tools/lookup.rs` にアダプターレイヤーを置き、詳細は MCP 側に封装
6. **`rename_symbol` AST 書き換えと LSP `rename` のセマンティクスが異なる**：LSP `textDocument/rename` は URI + 位置 +
   new_name → WorkspaceEdit；MCP `rename_symbol` は source + old_name + new_name
   → 新 source。**直接再利用不可**。**緩和**：
   `src/middle/rename.rs` を別途実装、scope-aware 書き換えで参照、LSP handler 実装と互いに干渉しない

## 開放問題

- [ ] `mcp-rust-sdk` 選択 / 自作？（@Chen Xu：まず rust-sdk の6月バージョンを評価、それから決定）
- [ ] HTTP 認証パス？（v0.10 RFC で再開）
- [ ] `MCP` 起動時に AI が能動的に発見するための `tools/list` 出力が必要か？（MCP 標準では要求、**デフォルト実装**）
- [ ] `typecheck` は `mode: "fast|full"` をサポートするか？（fast = 現在のファイルサブセットのみ、full = 全ワークスペース）？
- [ ] パフォーマンス予算 parse_source P99 < 100ms は現実的か？（RFC-017 で既に実装済みの `DocumentCache`
      が source-string モードで実際にどれだけのオーバーヘッドがあるか benchmark が必要）

## 参考文献

- [RFC-017: 言語サーバープロトコル（LSP）サポート設計](../accepted/017-lsp-support.md)
- [RFC-013: エラーコード仕様設計](../accepted/013-error-code-specification.md)
- [RFC-014: パッケージ管理システム設計](../accepted/014-package-manager.md)
- [RFC-015: YaoXiang 設定システム設計](../accepted/015-configuration-system.md)
- [MCP 仕様](https://modelcontextprotocol.io/)
- [MCP Rust SDK](https://github.com/modelcontextprotocol/rust-sdk)
- [JSON-RPC 2.0](https://www.jsonrpc.org/specification)
- [LSP 仕様 3.18](https://github.com/microsoft/language-server-protocol/blob/main/specifications/specification-3-18.md)
- [Rust Analyzer](https://rust-analyzer.github.io/) —— M2 / MCP 統合の参照
- [zed-industries/zed の MCP 実装](https://github.com/zed-industries/zed/tree/main/crates/mcp)
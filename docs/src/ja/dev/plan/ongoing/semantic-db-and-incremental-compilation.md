# セマンティック情報プラットフォームと增量コンパイル実装計画

> **タスク**：セマンティック情報プラットフォームを実現し、LSP セマンティックハイライト、增量コンパイル、デッドコード警告機能を提供する
> **RFC ベース**：本計画は新機能設計である
> **関連 RFC**：[RFC-008: 実行時並行モデル](../design/rfc/accepted/008-runtime-concurrency-model.md) - DAG 並行は実行時の機能であり、本計画の範囲外
> **日付**：2026-02-23
> **状態**：フェーズ 1 + フェーズ 2 + フェーズ 3 完了
> **ターゲットバージョン**：v0.10 - v0.11

---

## 概要

本計画では、セマンティック情報プラットフォームの実装を3つの主要フェーズに分解する。核となる考え方は**1度の走査で多处中使用**：

1. **セマンティック収集は typecheck フェーズで完了**（LSP 層で別個に AST を走査するのではなく）
2. 収集したセマンティック情報は LSP セマンティックハイライト、增量コンパイル、デッドコード分析に共用

> **重要な澄清**：
> - **DAG 並行**は実行時機能（RFC-008）であり、本計画の範囲外
> - **モジュール依存グラフの並行コンパイル**はビルドシステム機能で、実行時 DAG とは別の概念
> - セマンティック収集は typecheck フェーズで完了すべきであり、LSP はそのまま再利用するべきであり、独立した走査器を再実装する必要はない

---

## フェーズ 1：SemanticDB インフラストラクチャ

> **重要性**：本フェーズは今後の全機能の基礎であり、先に完了させる必要がある
> **ターゲットバージョン**：v0.10
> **状態**：✅ 完了


**実装目標**：

- `SemanticDB` 構造体を定義し、セマンティック情報を統一管理する
- `SemanticToken` 列挙型を定義し、LSP 標準 TokenType を含む
- `SymbolReference` 構造体を定義し、シンボル参照位置を記録する
- `ModuleSymbol` 構造体を定義し、モジュールレベルシンボル定義を記録する

**データ構造設計**：

```rust
// セマンティック情報データベース（src/frontend/typecheck/semantic_db.rs に実装）
pub struct SemanticDB {
    // ファイルパス -> 当該ファイル内のセマンティック情報
    by_file: HashMap<String, FileSemanticInfo>,
    // シンボル名 -> 全定義位置
    symbol_defs: HashMap<String, Vec<SymbolLocation>>,
    // シンボル名 -> 全参照位置
    symbol_refs: HashMap<String, Vec<SymbolLocation>>,
}

// 単一ファイルのセマンティック情報
pub struct FileSemanticInfo {
    pub file_path: String,
    pub tokens: Vec<SemanticToken>,
    pub scopes: Vec<ScopeInfo>,
}

// セマンティック Token（列挙型 variant ではなく構造体 + 型列挙型を使用）
pub struct SemanticToken {
    pub name: String,
    pub token_type: SemanticTokenType,
    pub modifiers: Vec<SemanticTokenModifier>,
    pub span: Span,
}

pub enum SemanticTokenType {
    Function, Type, Variable, Property, Method,
    Namespace, Parameter, LocalVariable, TypeParameter,
    Keyword, String, Number,
}

pub enum SemanticTokenModifier {
    Declaration, Readonly, Mutable, Public, Generic,
}

// スコープ情報
pub struct ScopeInfo {
    pub span: Span,
    pub parent: Option<usize>,  // 親スコープインデックス
    pub symbols: Vec<String>,   // スコープ内のシンボル
    pub kind: ScopeKind,        // Global, Function, Block, Lambda
}
```

**受け入れ基準**：

- [x] SemanticDB 構造体定義完了
- [x] SemanticToken が LSP 標準 token 型をカバー（12 種類の型 + 5 種類の修飾子）
- [x] ファイル別セマンティック情報クエリ対応
- [x] シンボル名別定義・参照位置クエリ対応

**テスト項目**：

- [x] SemanticDB 構造体作成テスト
- [x] ファイル別クエリテスト
- [x] シンボル名別クエリテスト
- [x] 空データベース境界テスト
- [x] 複数ファイル管理テスト
- [x] ファイル上書き更新テスト

---

### 1.2 TypeCheck セマンティックコレクタ統合

**設計判断**：セマンティック収集は**LSP 層で別個に実装すべきではなく**、typecheck フェーズで完了させるべきである。

**理由**：

- typecheck は既に AST を走査しており、すべてのシンボルの定義・参照位置を把握している
- LSP で SemanticCollector を別途実装 = 重複走査 + 2 套のロジック保守
- **良い設計**：1度の走査で多处中使用

**実装目標**：

- `src/frontend/typecheck/` モジュールでセマンティック収集機能を拡張
- 型検査時に同時に `SemanticDB` データを生成
- LSP 層は直接クエリして再利用し、AST を重複走査しない

**typecheck ルールによる収集**（フェーズ成果物）：

```
StmtKind::Fn        → SemanticTokenType::Function (定義)
StmtKind::TypeDef   → SemanticTokenType::Type (定義)
StmtKind::Var       → SemanticTokenType::Variable (定義)
StmtKind::MethodBind→ SemanticTokenType::Method (定義)
StmtKind::Use       → SemanticTokenType::Namespace (参照)
Param               → SemanticTokenType::Parameter (定義)
Expr::Var           → SemanticTokenType::Variable (参照)
Expr::Call          → SemanticTokenType::Function (参照)
Expr::FieldAccess   → SemanticTokenType::Property (参照)
Expr::Cast          → SemanticTokenType::Type (参照)
```

**受け入れ基準**：

- [x] typecheck フェーズで SemanticDB を生成
- [x] LSP が typecheck が生成したセマンティック情報をクエリ可能
- [x] LSP 層の重複 AST 走査を排除

---

### 1.3 スコープチェーン収集

**実装目標**：

- スコープ情報もまた typecheck フェーズで生成
- 各スコープの開始・終了位置を記録
- スコープ内のシンボルリストを記録
- ネストスコープの正しい親子関係をサポート
- 4 種類のスコープタイプをサポート：Global, Function, Block, Lambda

**注意**：これらの情報は既に typecheck の `TypeEnvironment` で管理されており、今は SemanticDB で利用するためにエクスポートする必要がある。

**受け入れ基準**：

- [x] グローバルスコープ情報正確
- [x] 関数スコープ情報正確
- [x] ブロックレベルスコープ情報正確
- [x] ネストスコープの親子関係正確

**テスト項目**：

- [x] 単層スコープテスト（グローバルスコープ）
- [x] ネストスコープテスト（グローバル + 関数）
- [x] Lambda スコープテスト
- [x] スコープ最内層検索テスト

---

### 1.4 World 拡張統合

**実装目標**：

- `src/lsp/world.rs` の World 構造体を拡張
- SemanticDB フィールドを追加
- LSP ドキュメント変更時に typecheck 再実行をトリガーしてセマンティック情報を更新
- LSP handlers が typecheck が生成した SemanticDB を直接クエリ

**設計調整**：

- LSP 層で別途 SemanticCollector を呼び出す必要はない
- LSP はドキュメント変更後に typecheck 再実行をトリガーするだけでよい
- World はコンパイルパイプラインへの参照を保持し、最新の SemanticDB を取得

**受け入れ基準**：

- [x] World に SemanticDB フィールドが含まれる
- [x] ドキュメント変更時に typecheck 再実行をトリガーしてセマンティック情報を更新
- [x] LSP handlers がセマンティック情報をクエリ可能

**テスト項目**：

- [x] World セマンティック情報更新テスト（既存のサーバテストで検証）
- [x] 複数ファイルセマンティック情報管理テスト
- [x] セマンティック情報クエリインターフェーステスト

---

## フェーズ 2：LSP セマンティックハイライト

> **ターゲットバージョン**：v0.10
> **依存**：フェーズ 1 完了
> **状態**：✅ 完了

### 2.1 Semantic Tokens Capability 宣言

**実装目標**：

- `src/lsp/capabilities.rs` で semanticTokensProvider を宣言
- token 型マッピングを定義（YaoXiang → LSP）
- token 修飾子マッピングを定義

**Token 型マッピング**：

```
YaoXiang SymbolKind::Function    → LSP TokenType::FUNCTION
YaoXiang SymbolKind::Type        → LSP TokenType::TYPE
YaoXiang SymbolKind::Variable    → LSP TokenType::VARIABLE
YaoXiang SymbolKind::GenericType  → LSP TokenType::TYPE
YaoXiang SymbolKind::Parameter    → LSP TokenType::PARAMETER
YaoXiang SymbolKind::Property     → LSP TokenType::PROPERTY
YaoXiang SymbolKind::Method       → LSP TokenType::METHOD
YaoXiang SymbolKind::Namespace    → LSP TokenType::NAMESPACE
```

**受け入れ基準**：

- [x] capabilities 宣言に semanticTokensProvider が含まれる
- [x] token 型マッピング正確
- [x] full と delta モードをサポート

**テスト項目**：

- [x] capability 宣言テスト
- [x] プロトコル互換性テスト

---

### 2.2 textDocument/semanticTokens/full Handler

**実装目標**：

- `handle_semantic_tokens_full` 処理関数を実装
- SemanticDB からファイルのセマンティック tokens を取得
- LSP SemanticToken 形式に変換
- 全量リフレッシュをサポート

**LSP レスポンス形式**：

```json
{
  "data": [
    0,   // deltaLine (前の token との相対値)
    0,   // deltaStart (前の token との相対値)
    5,   // length
    0,   // tokenType (function)
    0    // tokenModifiers
  ]
}
```

**受け入れ基準**：

- [x] 正しいセマンティック tokens データを返す
- [x] 行番号・列番号は 0 から開始
- [x] レスポンス時間 < 200ms（単一ファイル < 1000 行）
- [x] 空ファイルは空配列を返す

**テスト項目**：

- [x] 単純関数セマンティックハイライトテスト
- [x] 複雑なネスト構造テスト
- [ ] パフォーマンステスト（1000 行ファイル）——ベンチマーク待ち
- [x] 空ファイルテスト

---

### 2.3 textDocument/semanticTokens/full/delta Handler

**実装目標**：

- 增量セマンティック tokens 更新を実装
- ドキュメントバージョン差分を追跡
- 変更された tokens のみを返す

**受け入れ基準**：

- [x] 增量更新が正しい delta を返す
- [x] バージョン番号が正しく追跡される
- [x] 削除操作が正しく処理される

**テスト項目**：

- [x] token 追加增量テスト
- [x] token 削除增量テスト
- [x] token 変更增量テスト

---

### 2.4 VSCode テーマ設定

**実装目標**：

- language-pack にセマンティックハイライトテーマ設定例を追加
- token 型とテーマ色のマッピングを文書化

**テーマ色マッピング推奨**：

```json
{
  "tokenTypes": {
    "function": "entity.name.function",
    "type": "entity.name.type",
    "variable": "variable",
    "parameter": "variable.parameter",
    "property": "variable.property",
    "namespace": "namespace"
  }
}
```

**受け入れ基準**：

- [x] テーマ設定例が完全
- [x] ドキュメント説明が明確

---

## フェーズ 3：增量コンパイル

> **ターゲットバージョン**：v0.11
> **依存**：フェーズ 1 完了
> **状態**：✅ 完了

### 3.1 モジュールド依存グラフ構築

**実装目標**：

- `ModuleDependencyGraph` 構造体を実装
- import/use 文を解析してモジュールの依存関係を構築
- 循環依存検出をサポート

**データ構造**：

```rust
pub struct ModuleDependencyGraph {
    // モジュール ID -> 依存するモジュール ID リスト
    deps: HashMap<ModuleId, Vec<ModuleId>>,
    // モジュール ID -> エクスポートするシンボルリスト
    exports: HashMap<ModuleId, Vec<SymbolId>>,
    // シンボル定義位置
    symbol_defs: HashMap<SymbolId, SymbolLocation>,
}

pub struct ModuleId {
    pub name: String,
    pub path: PathBuf,
}
```

**受け入れ基準**：

- [x] 単一ファイルプロジェクトの依存グラフが正しい
- [x] 複数ファイルプロジェクトの依存グラフが正しい
- [x] 循環依存検出が正しい
- [x] 增量更新時に依存グラフが正しく更新される

**テスト項目**：

- [x] 単一ファイル依存テスト
- [x] 複数ファイル依存テスト
- [x] 循環依存検出テスト
- [x] 增量更新テスト

---

### 3.2 コンパイルキャッシュシステム

**実装目標**：

- コンパイル成果物キャッシュを実装（AST、型情報、IR）
- ファイル内容ハッシュに基づいて変更を検出
- キャッシュのシリアライズ/デシリアライズを実装

**キャッシュ内容**：

```rust
pub struct CompilationCache {
    // ファイルパス -> ファイルキャッシュ
    files: HashMap<PathBuf, FileCache>,
    // キャッシュメタデータ
    metadata: CacheMetadata,
}

pub struct FileCache {
    pub content_hash: u64,
    pub ast: Option<Module>,
    pub type_info: Option<TypeInfo>,
    pub ir: Option<ModuleIR>,
    pub semantic_db: Option<SemanticDB>,
    pub timestamp: SystemTime,
}
```

**受け入れ基準**：

- [x] 未変更ファイルはキャッシュを直接使用
- [x] 変更ファイルは正しく再コンパイル
- [x] キャッシュシリアライズが正しい（メモリキャッシュ、Clone ベース）
- [x] キャッシュクリーン機構が正常に動作

**テスト項目**：

- [x] キャッシュヒットテスト
- [x] キャッシュミステスト
- [x] キャッシュシリアライズテスト（メモリキャッシュ、Clone 方式）
- [x] キャッシュクリーンテスト

---

### 3.3 增量コンパイルスケジューラ

**実装目標**：

- 依存グラフベースのコンパイルスケジューリングを実装
- 変更影響を受けるファイルのみをコンパイル
- トポロジカルソートでコンパイル順序を決定

**スケジューリング戦略**：

```
1. 変更ファイルリストを検出
2. 変更ファイルに依存する全モジュールを算出（再帰的上方）
3. トポロジカルソートでコンパイル順序を決定
4. 並行/串行コンパイルスケジューリング
```

**受け入れ基準**：

- [x] 単一ファイル変更は必要なファイルのみ再コンパイル
- [x] コンパイル順序が正しい（依存が先）
- [x] 並行コンパイルに競合条件なし（バッチグループ対応）

**テスト項目**：

- [x] 単一ファイル変更テスト
- [x] 複数ファイル変更テスト
- [x] 依存チェーン変更テスト
- [x] 並行コンパイルテスト（バッチグループ）

---

### 3.4 ビルドシステム統合

**実装目標**：

- `yaoxiang build` コマンドに增量コンパイルサポートを実装
- 增量コンパイル統計情報を出力
- `--force` で強制全量コンパイルをサポート

**受け入れ基準**：

- [x] 增量コンパイルコマンドが正常に動作
- [x] 全量コンパイルコマンドが正常に動作（clear_cache）
- [x] 統計情報が正しく出力される
- [x] エラーハンドリングが正しい

**テスト項目**：

- [x] 增量コンパイル機能テスト
- [x] 全量コンパイル機能テスト
- [x] 統計情報テスト

---

## フェーズ 4：デッドコード警告（コンパイルフローに統合）

> **ターゲットバージョン**：v0.11
> **依存**：フェーズ 1 完了（typecheck フェーズのセマンティック情報）

> **説明**：デッドコード警告は typecheck フェーズのシンボル参照情報に依存し、コンパイル時分析機能であり、実行時機能ではない。

> **アーキテクチャ調整**：デッドコード分析は typecheck フェーズに統合する。両者とも AST/SemanticDB の走査が必要だからである。

### 4.1 デッドコードアナライザ

**実装目標**：

- `DeadCodeAnalyzer` 構造体を実装
- 未使用のエクスポートシンボルを分析
- 未使用のインポートを分析
- 警告情報を生成

**設計判断**：デッドコード分析は **typecheck フェーズ** で完了すべきである。理由は：

- typecheck は既にすべてのシンボルの定義・参照を把握している
- 追加の AST 走査が不要
- セマンティック情報は既に SemanticDB を通じて提供されている

**分析ルール**：

```
1. 全エントリポイント（main, pub 関数）を収集
2. エントリポイントから到達可能な全シンボルをマーク
3. 到達不可能なエクスポートシンボル -> 警告
4. 未使用のインポート -> 警告
```

**データ構造**：

```rust
pub struct DeadCodeAnalyzer {
    // エントリポイント
    entry_points: HashSet<SymbolId>,
    // 全シンボル定義
    all_defs: HashMap<SymbolId, SymbolDef>,
    // シンボル参照（SemanticDB から取得）
    references: HashMap<SymbolId, Vec<Location>>,
    // インポートリスト
    imports: Vec<ImportInfo>,
}

pub struct SymbolDef {
    pub name: String,
    pub kind: SymbolKind,
    pub location: Location,
    pub is_exported: bool,
}
```

**受け入れ基準**：

- [x] 未使用のエクスポート関数を検出可能
- [x] 未使用のエクスポート型を検出可能
- [x] 未使用のインポートを検出可能
- [x] 警告情報形式が正しい

**テスト項目**：

- [x] 未使用エクスポート関数テスト
- [x] 未使用エクスポート型テスト
- [x] 未使用インポートテスト
- [x] 複数レベル依存テスト


---

### 4.2 警告システム統合

**実装目標**：

- コンパイルプロセスにデッドコード検出を統合
- `CompilationWarning` イベントを通じて警告をパブリッシュ
- 複数出力形式をサポート（ターミナル、JSON）

**警告形式**：

```
warning: unused function `dead_function`
  --> src/utils.yx:10:1
   |
10 | fn dead_function() { }
   | ^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: function is never used
```

**受け入れ基準**：

- [x] デッドコード警告が正しくトリガーされる
- [x] 警告位置情報が正確
- [x] 警告が設定可能（有効/無効）
- [x] ターミナル出力形式が整形されている

**テスト項目**：

- [x] 警告トリガーテスト
- [x] 警告位置テスト
- [x] 設定テスト
- [x] 出力形式テスト

---

## DAG 並行についての説明

**本計画には DAG 並行コンパイルは含まない**。理由は以下の通り：

| 概念 | 帰属 | 説明 |
|------|------|------|
| **実行時 DAG** | RFC-008 Runtime | 遅延評価依存グラフ、実行時のタスクスケジューリングを制御 |
| **モジュールの依存グラフ** | 本計画フェーズ3 | コンパイラレベルのモジュール依存、增量コンパイル用 |
| **モジュールレベル並行コンパイル** | ビルドシステム | フェーズ3の依存グラフに基づいて実装、LSP には属さない |

**正しい位置**：

- 実行時 DAG 並行 → [RFC-008: 実行時並行モデル](../design/rfc/accepted/008-runtime-concurrency-model.md) を参照
- モジュールの依存グラフ → 本計画フェーズ3（完了/進行中）
- モジュールレベル並行コンパイル → ビルドシステム機能として実装すべき。フェーズ3の依存グラフに基于可能

---

## アーキテクチャ設計まとめ

### 統一データフロー

```
┌─────────────────────────────────────────────────────────────────┐
│                      セマンティック情報プラットフォームアーキテクチャ   │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   ソースコード                                                        │
│     │                                                              │
│     ▼                                                              │
│   ┌─────────────────┐                                            │
│   │  字句解析/構文解析  │ ──▶ AST                                    │
│   └────────┬────────┘                                            │
│            │                                                       │
│            ▼                                                       │
│   ┌─────────────────┐                                            │
│   │  型検査          │ ──┬─▶ TypeResult + Bindings                │
│   │                  │   │                                        │
│   │  同時生成         │   │  ← 1度の走査で多处中使用                │
│   │  SemanticDB      │   │                                        │
│   └────────┬────────┘   │                                        │
│            │            │                                        │
│            ▼            │                                        │
│   ┌─────────────────┐  │                                        │
│   │  SemanticDB     │◄─┘  ← typecheck が生成                      │
│   │  - シンボル定義  │                                            │
│   │  - シンボル参照  │                                            │
│   │  - スコープチェーン│                                            │
│   └────────┬────────┘                                            │
│            │                                                       │
│    ┌───────┴───────┐                                            │
│    ▼               ▼                                             │
│ ┌──────┐       ┌──────────┐                                    │
│ │ LSP  │       │ 增量コンパイル │                                    │
│ │セマンティック│       │ + デッドコード │                                    │
│ │ハイライト│       │              │                                    │
│ └──────┘       └──────────┘                                    │
│                                                                 │
│   ▲                                                         ▲    │
│   │                                                         │    │
│   │  DAG 並行 → RFC-008 実行時、本計画の範囲外               │    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 設計原則

1. **1度の走査**：typecheck フェーズでセマンティック情報を同時に生成。AST の重複走査なし
2. **多处中使用**：LSP セマンティックハイライト、增量コンパイル、デッドコード分析が同一データを共用
3. **良い設計**：「疎結合」ために不必要な抽象化層を追加しない

### ファイル変更リスト

| フェーズ | 新規ファイル | 変更ファイル | 状態 |
|------|----------|----------|------|
| 1 | `src/frontend/typecheck/semantic_db.rs` | `src/frontend/typecheck/mod.rs` | ✅ 完了 |
| 1 | - | `src/lsp/world.rs` | ✅ 完了 |
| 2 | - | `src/lsp/capabilities.rs` | ✅ 完了 |
| 2 | `src/lsp/handlers/semantic_tokens.rs` | `src/lsp/handlers/mod.rs` | ✅ 完了（delta サポート含む） |
| 2 | - | `src/lsp/server.rs` | ✅ 完了（semanticTokens/full + delta リクエストディスパッチ追加） |
| 2 | - | `vscode-extension/language-pack/package.json` | ✅ 完了（セマンティックハイライトテーマ設定） |
| 3 | `src/frontend/module/dep_graph.rs` | `src/frontend/module/mod.rs` | ✅ 完了 |
| 3 | `src/frontend/pipeline/compilation_cache.rs` | `src/frontend/pipeline.rs` | ✅ 完了 |
| 3 | `src/frontend/pipeline/incremental_scheduler.rs` | `src/frontend/compiler.rs` | ✅ 完了 |
| 4 | `src/frontend/typecheck/dead_code.rs` | `src/frontend/typecheck/mod.rs` | ✅ 完了 |
| 4 | - | `src/frontend/pipeline.rs` | ✅ 完了（コンパイルフローに統合） |
| 4 | - | `src/frontend/typecheck/semantic_db.rs` | ✅ 完了（参照アクセスメソッド追加） |

**重要な調整**：セマンティックコレクタを `src/lsp/` から `src/frontend/typecheck/` へ移行

---

## リスクと緩和策

| リスク | 緩和策 |
|------|----------|
| typecheck がセマンティック情報に密結合 | 疎結合設計。SemanticDB を独立したデータ構造とする |
| 循環依存の処理 | 明示的に検出して警告 |
| 增量コンパイルの競合 | Mutex で共有状態を保護 |
| キャッシュの一貫性 | バージョン番号追跡、ハッシュ検証 |

---

## 参考資料

- [LSP Semantic Tokens Specification](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#semanticTokens)
- [Rust Analyzer Semantic Highlighting](https://github.com/rust-lang/rust-analyzer/blob/master/docs/dev/semantic-highlighting.md)
- [Incremental Compilation (Rustc)](https://rustc-dev-guide.rust-lang.org/inc-intro.html)
- [RFC-008: 実行時並行モデル](../design/rfc/accepted/008-runtime-concurrency-model.md)
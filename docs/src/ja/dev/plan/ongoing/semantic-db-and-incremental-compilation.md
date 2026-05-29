# 意味情報プラットフォームと增量コンパイル実装計画

> **任務**：意味情報プラットフォームを実装し、LSP 意味強調表示、增量コンパイル、デッドコード警告機能を提供する
> **RFC ベース**：本計画は新規機能設計
> **関連 RFC**：[RFC-008: ランタイム並行モデル](../design/rfc/accepted/008-runtime-concurrency-model.md) - DAG 並行はランタイムに属し、本計画範囲外
> **日付**：2026-02-23
> **状態**：フェーズ 1 + フェーズ 2 + フェーズ 3 完了
> **目標バージョン**：v0.10 - v0.11

---

## 概要

本計画では、意味情報プラットフォームの実装を 3 つの主要フェーズに分解する。核心理念は**1回の走査で複数箇所で使用**：

1. **意味収集は typecheck フェーズで完了**（LSP 層で個別に AST を走査するのではなく）
2. 収集された意味情報は LSP 意味強調表示、增量コンパイル、死コード分析の全てに供される

> **重要な澄清**：
> - **DAG 並行**はランタイム機能（RFC-008）であり、本計画範囲外
> - **モジュール依存グラフの並行コンパイル**はビルドシステム機能であり、ランタイム DAG とは異なる概念
> - 意味収集は typecheck フェーズで完了すべきであり、LSP はそのまま再利用し、独立した走査器を記述しない

---

## フェーズ 1：SemanticDB インフラストラクチャ

> **重要性**：本フェーズは後続全機能の基礎であり、必ず最初に完了する必要がある
> **目標バージョン**：v0.10
> **状態**：✅ 完了


**実装目標**：
- `SemanticDB` 構造体を定義し、意味情報を統一管理する
- `SemanticToken` 列挙型を定義し、LSP 標準 TokenType を含む
- `SymbolReference` 構造体を定義し、シンボル参照位置を記録する
- `ModuleSymbol` 構造体を定義し、モジュールレベルシンボル定義を記録する

**データ構造設計**：

```rust
// 意味情報データベース（src/frontend/typecheck/semantic_db.rs に実装）
pub struct SemanticDB {
    // ファイルパス -> 当該ファイルの意味情報
    by_file: HashMap<String, FileSemanticInfo>,
    // シンボル名 -> 全ての定義位置
    symbol_defs: HashMap<String, Vec<SymbolLocation>>,
    // シンボル名 -> 全ての参照位置
    symbol_refs: HashMap<String, Vec<SymbolLocation>>,
}

// 単一ファイルの意味情報
pub struct FileSemanticInfo {
    pub file_path: String,
    pub tokens: Vec<SemanticToken>,
    pub scopes: Vec<ScopeInfo>,
}

// 意味トークン（列挙型変体ではなく構造体 + 型列挙型を使用）
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
- [x] SemanticToken が LSP 標準トークンタイプをカバー（12 種タイプ + 5 種修飾子）
- [x] ファイル単位の意味情報クエリをサポート
- [x] シンボル名による定義と参照位置のクエリをサポート

**テスト項目**：
- [x] SemanticDB 構造体作成テスト
- [x] ファイル単位クエリテスト
- [x] シンボル名クエリテスト
- [x] 空データベース境界テスト
- [x] 複数ファイル管理テスト
- [x] ファイル上書き更新テスト

---

### 1.2 TypeCheck 意味収集器の統合

**設計決定**：意味収集は **LSP 層で独立して実装するのではなく**、typecheck フェーズで完了する必要がある。

**理由**：
- typecheck は既に AST を走査しており、すべてのシンボルの定義と参照位置を把握している
- LSP で独立した SemanticCollector を実装すると = 走査の重複 + 2 套のロジック保守
- **良い設計**：1 回の走査で複数箇所使用

**実装目標**：
- `src/frontend/typecheck/` モジュールで意味収集機能を拡張
- 型検査時に `SemanticDB` データも生成
- LSP 層は直接クエリして再利用し、AST を繰り返し走査しない

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
- [x] LSP が typecheck が生成した意味情報をクエリ可能
- [x] LSP 層の重複 AST 走査を消除

---

### 1.3 スコープチェーン収集

**実装目標**：
- スコープ情報は typecheck フェーズで生成する
- 各スコープの開始位置と終了位置を記録
- スコープ内のシンボルリストを記録
- ネストスコープの正しい親子関係をサポート
- 4 種類のスコープタイプをサポート：Global, Function, Block, Lambda

**注意**：これらの情報は既に typecheck の `TypeEnvironment` で管理されており、今は SemanticDB で使用するためにエクスポートする必要がある。

**受け入れ基準**：
- [x] グローバルスコープ情報が正確
- [x] 関数スコープ情報が正確
- [x] ブロックレベルスコープ情報が正確
- [x] ネストスコープの親子関係が正確

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
- LSP ドキュメント変更時に、typecheck 再実行をトリガーして意味情報を更新
- LSP handlers は typecheck が生成した SemanticDB を直接クエリ

**設計調整**：
- LSP 層で独立して SemanticCollector を呼び出す必要はなくなった
- LSP はドキュメント変更後に typecheck 再実行をトリガーするだけでよい
- World はコンパイルパイプラインへの参照を保持し、最新の SemanticDB を取得

**受け入れ基準**：
- [x] World に SemanticDB フィールドが含まれる
- [x] ドキュメント変更時に typecheck 再実行をトリガーし意味情報を更新
- [x] LSP handlers が意味情報をクエリ可能

**テスト項目**：
- [x] World 意味情報更新テスト（既存のサーバーテストで検証）
- [x] 複数ファイル意味情報管理テスト
- [x] 意味情報クエリインターフェーステスト

---

## フェーズ 2：LSP 意味強調表示

> **目標バージョン**：v0.10
> **依存**：フェーズ 1 完了
> **状態**：✅ 完了

### 2.1 Semantic Tokens Capability 宣言

**実装目標**：
- `src/lsp/capabilities.rs` で semanticTokensProvider を宣言
- トークンタイプマッピングを定義（YaoXiang → LSP）
- トークン修飾子マッピングを定義

**トークンタイプマッピング**：
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
- [x] トークンタイプマッピングが正確
- [x] full と delta モードをサポート

**テスト項目**：
- [x] capability 宣言テスト
- [x] プロトコル互換性テスト

---

### 2.2 textDocument/semanticTokens/full Handler

**実装目標**：
- `handle_semantic_tokens_full` 処理関数を実装
- SemanticDB からファイルのセマンティックトークンを取得
- LSP SemanticToken 形式に変換
- 全量リフレッシュをサポート

**LSP 応答形式**：
```json
{
  "data": [
    0,   // deltaLine (前のトークンからの差分)
    0,   // deltaStart (前のトークンからの差分)
    5,   // length
    0,   // tokenType (function)
    0    // tokenModifiers
  ]
}
```

**受け入れ基準**：
- [x] 正しいセマンティックトークンデータを返す
- [x] 行番号・列番号は 0 から開始
- [x] 応答時間 < 200ms（単一ファイル < 1000 行）
- [x] 空ファイルは空配列を返す

**テスト項目**：
- [x] 単純関数意味強調テスト
- [x] 複雑なネスト構造テスト
- [ ] パフォーマンステスト（1000 行ファイル）——ベンチマーク待ち
- [x] 空ファイルテスト

---

### 2.3 textDocument/semanticTokens/full/delta Handler

**実装目標**：
- 增量セマンティックトークン更新を実装
- ドキュメントバージョン差分を追跡
- 変化したトークンのみを返す

**受け入れ基準**：
- [x] 增量更新が正しい delta を返す
- [x] バージョン番号が正しく追跡される
- [x] 削除操作が正しく処理される

**テスト項目**：
- [x] トークン追加增量テスト
- [x] トークン削除增量テスト
- [x] トークン変更增量テスト

---

### 2.4 VSCode テーマ設定

**実装目標**：
- language-pack に意味強調テーマ設定サンプルを追加
- トークンタイプとテーマ色のマッピングを文書化

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
- [x] テーマ設定サンプルが完整
- [x] ドキュメント説明が明確

---

## フェーズ 3：增量コンパイル

> **目標バージョン**：v0.11
> **依存**：フェーズ 1 完了
> **状態**：✅ 完了

### 3.1 モジュュー依存グラフ構築

**実装目標**：
- `ModuleDependencyGraph` 構造体を実装
- import/use 文を解析してモジュール依存関係を構築
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
- [x] 単一ファイルプロジェクトの依存グラフが正確
- [x] 複数ファイルプロジェクトの依存グラフが正確
- [x] 循環依存検出が正確
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
- キャッシュの直列化/逆直列化を実装

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
- [x] キャッシュ直列化が正確（メモリキャッシュ、Clone ベース）
- [x] キャッシュクリアメカニズムが正常

**テスト項目**：
- [x] キャッシュヒットテスト
- [x] キャッシュ未ahitテスト
- [x] キャッシュ直列化テスト（メモリキャッシュ、Clone 方式）
- [x] キャッシュクリアテスト

---

### 3.3 增量コンパイルスケジューラ

**実装目標**：
- 依存グラフベースのコンパイルスケジューリングを実装
- 変更影響ファイルのみをコンパイル
- トポロジカルソートでコンパイル順序を決定

**スケジューリング戦略**：
```
1. 変更ファイルリストを検出
2. 変更ファイルに依存する全モジュールを找出（再帰的に上位へ）
3. トポロジカルソートでコンパイル順序を決定
4. 並行/串列コンパイルをスケジューリング
```

**受け入れ基準**：
- [x] 単一ファイル変更時は必要なファイルのみを再コンパイル
- [x] コンパイル順序が正確（依存が先）
- [x] 並行コンパイルに競合条件なし（バッチグループサポート）

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
- `--force` で全量コンパイルを強制をサポート

**受け入れ基準**：
- [x] 增量コンパイルコマンドが正常に動作
- [x] 全量コンパイルコマンドが正常に動作（clear_cache）
- [x] 統計情報出力が正確
- [x] エラー処理が正確

**テスト項目**：
- [x] 增量コンパイル機能テスト
- [x] 全量コンパイル機能テスト
- [x] 統計情報テスト

---

## フェーズ 4：死コード警告（コンパイルフローに統合）

> **目標バージョン**：v0.11
> **依存**：フェーズ 1 完了（typecheck フェーズの意味情報）

> **説明**：死コード警告は typecheck フェーズのシンボル参照情報に依存し、编译時分析機能であり、运行时特性ではない。

> **アーキテクチャ調整**：死コード分析は typecheck フェーズに統合，两者都需要遍历 AST/SemanticDB

### 4.1 死コード分析器

**実装目標**：
- `DeadCodeAnalyzer` 構造体を実装
- 未使用のエクスポートシンボルを分析
- 未使用のインポートを分析
- 警告情報を生成

**設計決定**：死コード分析は **typecheck フェーズで完了すべき**、理由は：
- typecheck は既にすべてのシンボルの定義と参照を把握している
- AST を追加で走査する必要がない
- 意味情報は既に SemanticDB を通じて提供されている

**分析ルール**：
```
1. 全ての入口点を収集（main, pub 関数）
2. 入口点から出発し，到達可能なシンボルを全てマーク
3. 到達不能なエクスポートシンボル → 警告
4. 未使用のインポート → 警告
```

**データ構造**：
```rust
pub struct DeadCodeAnalyzer {
    // 入口点
    entry_points: HashSet<SymbolId>,
    // 全てのシンボル定義
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
- [x] 警告情報形式が正確

**テスト項目**：
- [x] 未使用エクスポート関数テスト
- [x] 未使用エクスポート型テスト
- [x] 未使用インポートテスト
- [x] 複数レベル依存テスト


---

### 4.2 警告システム統合

**実装目標**：
- コンパイルプロセスに死コード検出を統合
- `CompilationWarning` イベントを通じて警告を publish
- 複数出力形式をサポート（端末、JSON）

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
- [x] 死コード警告が正しくトリガー
- [x] 警告位置情報が正確
- [x] 警告が設定可能（有効/無効）
- [x] 端末出力形式が美观

**テスト項目**：
- [x] 警告トリガーテスト
- [x] 警告位置テスト
- [x] 設定テスト
- [x] 出力形式テスト

---

## DAG 並行に関する説明

**本計画には DAG 並行コンパイルは含まない**、理由は：

| 概念 | 帰属 | 説明 |
|------|------|------|
| **ランタイム DAG** | RFC-008 Runtime | 遅延評価依存グラフ、ランタイムタスクスケジューリング制御 |
| **モジュール依存グラフ** | 本計画フェーズ3 | コンパイラレベルのモジュール依存、增量コンパイル用 |
| **モジュールレベル並行コンパイル** | ビルドシステム | フェーズ3の依存グラフベース実装、LSP 範囲外 |

**正しい位置**：
- ランタイム DAG 並行 → [RFC-008: ランタイム並行モデル](../design/rfc/accepted/008-runtime-concurrency-model.md) を参照
- モジューユ依存グラフ → 本計画フェーズ3（完了/進行中）
- モジューユレベル並行コンパイル → ビルドシステム機能として実装すべき、フェーズ3の依存グラフに基于

---

## アーキテクチャ設計まとめ

### 統一データフロー

```
┌─────────────────────────────────────────────────────────────────┐
│                      意味情報プラットフォームアーキテクチャ          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   ソースコード                                                     │
│     │                                                              │
│     ▼                                                              │
│   ┌─────────────────┐                                            │
│   │  字句解析/構文解析 │ ──▶ AST                                    │
│   └────────┬────────┘                                            │
│            │                                                       │
│            ▼                                                       │
│   ┌─────────────────┐                                            │
│   │  型検査          │ ──┬─▶ TypeResult + Bindings                │
│   │                  │   │                                        │
│   │  同時出力         │   │  ← 1回の走査で複数箇所使用              │
│   │  SemanticDB      │   │                                        │
│   └────────┬────────┘   │                                        │
│            │            │                                        │
│            ▼            │                                        │
│   ┌─────────────────┐  │                                        │
│   │  SemanticDB     │◄─┘  ← typecheck 出力                      │
│   │  - シンボル定義 │                                            │
│   │  - シンボル参照 │                                            │
│   │  - スコープチェーン │                                            │
│   └────────┬────────┘                                            │
│            │                                                       │
│    ┌───────┴───────┐                                            │
│    ▼               ▼                                             │
│ ┌──────┐       ┌──────────┐                                    │
│ │ LSP  │       │ 增量コンパイル │                                    │
│ │意味強調│       │ + 死コード   │                                    │
│ └──────┘       └──────────┘                                    │
│                                                                 │
│   ▲                                                         ▲    │
│   │                                                         │    │
│   │  DAG 並行 → RFC-008 ランタイム、本計画範囲外              │    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 設計原則

1. **1 回の走査**：typecheck フェーズで同時に意味情報を出力し、AST を繰り返し走査しない
2. **複数箇所使用**：LSP 意味強調表示、增量コンパイル、死コード分析が同じデータを共有
3. **良い設計**：「疎結合」を理由に不必要な抽象化層を追加しない

### ファイル変更一覧

| フェーズ | 新規ファイル | 変更ファイル | 状態 |
|------|----------|----------|------|
| 1 | `src/frontend/typecheck/semantic_db.rs` | `src/frontend/typecheck/mod.rs` | ✅ 完了 |
| 1 | - | `src/lsp/world.rs` | ✅ 完了 |
| 2 | - | `src/lsp/capabilities.rs` | ✅ 完了 |
| 2 | `src/lsp/handlers/semantic_tokens.rs` | `src/lsp/handlers/mod.rs` | ✅ 完了（delta サポート含） |
| 2 | - | `src/lsp/server.rs` | ✅ 完了（semanticTokens/full + delta リクエスト分配追加） |
| 2 | - | `vscode-extension/language-pack/package.json` | ✅ 完了（意味強調テーマ設定） |
| 3 | `src/frontend/module/dep_graph.rs` | `src/frontend/module/mod.rs` | ✅ 完了 |
| 3 | `src/frontend/pipeline/compilation_cache.rs` | `src/frontend/pipeline.rs` | ✅ 完了 |
| 3 | `src/frontend/pipeline/incremental_scheduler.rs` | `src/frontend/compiler.rs` | ✅ 完了 |
| 4 | `src/frontend/typecheck/dead_code.rs` | `src/frontend/typecheck/mod.rs` | ✅ 完了 |
| 4 | - | `src/frontend/pipeline.rs` | ✅ 完了（コンパイルフローに統合） |
| 4 | - | `src/frontend/typecheck/semantic_db.rs` | ✅ 完了（参照アクセスメソッド追加） |

**重要な調整**：意味収集器を `src/lsp/` から `src/frontend/typecheck/` へ移行

---

## リスクと緩和策

| リスク | 緩和策 |
|------|----------|
| typecheck と意味情報の結合 | 疎結合設計、SemanticDB を独立データ構造として分離 |
| 循環依存処理 | 明確検出と警告 |
| 增量コンパイル競合 | Mutex による共有状態保護 |
| キャッシュ整合性 | バージョン番号追跡、ハッシュ検証 |

---

## 参考資料

- [LSP Semantic Tokens Specification](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#semanticTokens)
- [Rust Analyzer Semantic Highlighting](https://github.com/rust-lang/rust-analyzer/blob/master/docs/dev/semantic-highlighting.md)
- [Incremental Compilation (Rustc)](https://rustc-dev-guide.rust-lang.org/inc-intro.html)
- [RFC-008: ランタイム並行モデル](../design/rfc/accepted/008-runtime-concurrency-model.md)
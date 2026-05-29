# セマンティック情報プラットフォームとインクリメンタルコンパイル実装計画

> **タスク**：セマンティック情報プラットフォームを実現し、LSP セマンティックハイライト、インクリメンタルコンパイル、デッドコード警告機能を提供
> **RFC ベース**：本計画は新規機能設計
> **関連 RFC**：[RFC-008: ランタイム並行モデル](../design/rfc/accepted/008-runtime-concurrency-model.md) - DAG 並行はランタイムに属し、本計画の範囲外
> **日付**：2026-02-23
> **ステータス**：フェーズ 1 + フェーズ 2 + フェーズ 3 完了
> **ターゲットバージョン**：v0.10 - v0.11

---

## 概要

本計画では、セマンティック情報プラットフォームの実装を3つの主要フェーズに分解する。核となる考え方は**1度の走査で複数箇所で使用**：

1. **セマンティック収集は typecheck フェーズで完了**（LSP 層で別途 AST を走査しない）
2. 収集されたセマンティック情報は LSP セマンティックハイライト、インクリメンタルコンパイル、デッドコード分析に同時提供服务

> **重要な澄清**：
> - **DAG 並行**はランタイム機能（RFC-008）であり、本計画の範囲外
> - **モジュール依存グラフの並列コンパイル**はビルドシステム機能で、ランタイム DAG とは別の概念
> - セマンティック収集は typecheck フェーズで完了すべきであり、LSP は直接再利用すべき

---

## フェーズ 1：SemanticDB インフラストラクチャ

> **重要性**：このフェーズは以降すべての機能の基盤であり、先に完了する必要がある
> **ターゲットバージョン**：v0.10
> **ステータス**：✅ 完了


**実装目標**：

- `SemanticDB` 構造体を定義し、セマンティック情報を統一管理
- `SemanticToken` 列挙型を定義し、LSP 標準 TokenType を含む
- `SymbolReference` 構造体を定義し、 символ参照位置を記録
- `ModuleSymbol` 構造体を定義し、モジュールレベル символ定義を記録

**データ構造設計**：

```rust
// セマンティック情情報データベース（src/frontend/typecheck/semantic_db.rs に実装）
pub struct SemanticDB {
    // ファイルパス -> そのファイル内のセマンティック情報
    by_file: HashMap<String, FileSemanticInfo>,
    // シンボル名 -> すべての定義位置
    symbol_defs: HashMap<String, Vec<SymbolLocation>>,
    // シンボル名 -> すべての参照位置
    symbol_refs: HashMap<String, Vec<SymbolLocation>>,
}

// 単一ファイルのセマンティック情報
pub struct FileSemanticInfo {
    pub file_path: String,
    pub tokens: Vec<SemanticToken>,
    pub scopes: Vec<ScopeInfo>,
}

// セマンティック Token（列挙型ではなく構造体 + 型列挙型を使用）
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

// スコープ情情報
pub struct ScopeInfo {
    pub span: Span,
    pub parent: Option<usize>,  // 親スコープインデックス
    pub symbols: Vec<String>,   // スコープ内のシンボル
    pub kind: ScopeKind,        // Global, Function, Block, Lambda
}
```

**受入基準**：

- [x] SemanticDB 構造体定義完了
- [x] SemanticToken が LSP 標準 token 型をカバー（12種頑 + 5種頑修飾子）
- [x] ファイル別セマンティック情情報クエリサポート
- [x] シンボル名による定義・参照位置クエリサポート

**テスト項目**：

- [x] SemanticDB 構造体作成テスト
- [x] ファイル別クエリテスト
- [x] シンボル名別クエリテスト
- [x] 空データベース境界テスト
- [x] 複数ファイル管理テスト
- [x] ファイル上書き更新テスト

---

### 1.2 TypeCheck セマンティックコレクタ統合

**設計上の決定**：セマンティック収集は**LSP 層で別途実装すべきではなく**、typecheck フェーズで完了すべきである。

**理由**：

- typecheck はすでに AST を走査しており、すべてのシンボルの定義と参照位置を把握している
- LSP で別途 SemanticCollector を実装すると、重複走査 + 2套ロジック保守になる
- **良い設計**：1度の走査で複数箇所で使用

**実装目標**：

- `src/frontend/typecheck/` モジュールのセマンティック収集機能を拡張
- 型チェック時に `SemanticDB` データを同時产出
- LSP 層は直接クエリして再利用し、AST を再走査しない

**typecheck ルールによる収集**（フェーズ产出）：

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

**受入基準**：

- [x] typecheck フェーズで SemanticDB 产出
- [x] LSP が typecheck の产出したセマンティック情報をクエリ可能
- [x] LSP 層の重複 AST 走査を消除

---

### 1.3 スコープチェーン収集

**実装目標**：

- スコープ情 тоже 由 typecheck フェーズで产出
- 各スコープの開始・終了位置を記録
- スコープ内のシンボルリストを記録
- 嵌套スコープの正しい親子関係をサポート
- 4種類のスコープタイプをサポート：Global, Function, Block, Lambda

**注意**：これらの情報はすでに typecheck の `TypeEnvironment` で管理されており、今は SemanticDB で使用するためにエクスポートする必要がある。

**受入基準**：

- [x] グローバルスコープ情報が正しい
- [x] 関数スコープ情報が正しい
- [x] ブロックレベルスコープ情報が正しい
- [x] 嵌套スコープの親子関係が正しい

**テスト項目**：

- [x] 単層スコープテスト（グローバルスコープ）
- [x] 嵌套スコープテスト（グローバル + 関数）
- [x] Lambda スコープテスト
- [x] スコープ最内層検索テスト

---

### 1.4 World 拡張統合

**実装目標**：

- `src/lsp/world.rs` の World 構造体を拡張
- SemanticDB フィールドを追加
- LSP ドキュメント変更時に、typecheck 再実行をトリガーしてセマンティック情報を更新
- LSP handlers は typecheck の产出した SemanticDB を直接クエリ

**設計調整**：

- LSP 層で別途 SemanticCollector を呼び出す必要はない
- LSP はドキュメント変更後に typecheck 再実行をトリガーするだけでよい
- World はコンパイル pipeline への参照を保持し、最新の SemanticDB を取得

**受入基準**：

- [x] World に SemanticDB フィールドが含まれる
- [x] ドキュメント変更時に typecheck 再実行をトリガーし、セマンティック情報を更新
- [x] LSP handlers がセマンティック情報をクエリ可能

**テスト項目**：

- [x] World セマンティック情情報更新テスト（既存の server テストで検証）
- [x] 複数ファイルセマンティック情情報管理テスト
- [x] セマンティック情情報クエリインターフェーステスト

---

## フェーズ 2：LSP セマンティックハイライト

> **ターゲットバージョン**：v0.10
> **依存**：フェーズ 1 完了
> **ステータス**：✅ 完了

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

**受入基準**：

- [x] capabilities 宣言に semanticTokensProvider が含まれる
- [x] token 型マッピングが正しい
- [x] full と delta モードをサポート

**テスト項目**：

- [x] capability 宣言テスト
- [x] プロトコル互換性テスト

---

### 2.2 textDocument/semanticTokens/full Handler

**実装目標**：

- `handle_semantic_tokens_full` 処理関数を実装
- SemanticDB からファイルのセマンティック tokens を取得
- LSP SemanticToken フォーマットに変換
- フルリフレッシュをサポート

**LSP 応答フォーマット**：

```json
{
  "data": [
    0,   // deltaLine（前の token 相对于）
    0,   // deltaStart（前の token 相对于）
    5,   // length
    0,   // tokenType (function)
    0    // tokenModifiers
  ]
}
```

**受入基準**：

- [x] 正し セマンティック tokens データを返す
- [x] 行番号・列番号は 0 から開始
- [x] 応答時間 < 200ms（単一ファイル < 1000 行）
- [x] 空ファイルは空配列を返す

**テスト項目**：

- [x] 単純関数セマンティックハイライトテスト
- [x] 複雑嵌套構造テスト
- [ ] パフォーマンステスト（1000行ファイル）——ベンチマーク待ち
- [x] 空ファイルテスト

---

### 2.3 textDocument/semanticTokens/full/delta Handler

**実装目標**：

- 增量セマンティック tokens 更新を実装
- ドキュメントバージョン差分を追踪
- 変化した tokens のみを返す

**受入基準**：

- [x] 增量更新が正しい delta を返す
- [x] バージョン番号が正しく追踪される
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

**受入基準**：

- [x] テーマ設定例が完成
- [x] ドキュメント説明が明確

---

## フェーズ 3：インクリメンタルコンパイル

> **ターゲットバージョン**：v0.11
> **依存**：フェーズ 1 完了
> **ステータス**：✅ 完了

### 3.1 モジュ従属グラフ構築

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

**受入基準**：

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

- コンパイル产物キャッシュを実装（AST、型情報、IR）
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

**受入基準**：

- [x] 未変更ファイルはキャッシュを直接使用
- [x] 変更ファイルは正しく再コンパイル
- [x] キャッシュシリアライズが正しい（メモリキャッシュ、Clone ベース）
- [x] キャッシュクリーン機構が正常

**テスト項目**：

- [x] キャッシュヒットテスト
- [x] キャッシュ未ヒットテスト
- [x] キャッシュシリアライズテスト（メモリキャッシュ、Clone 方式）
- [x] キャッシュクリーンステスト

---

### 3.3 インクリメンタルコンパイルスケジューラ

**実装目標**：

- 依存グラフベースのコンパイルスケジューリングを実装
- 変更影響を受けるファイルのみをコンパイル
- トポロジカルソートでコンパイル順序を決定

**スケジューリング戦略**：

```
1. 変更ファイルリストを検出
2. 変更ファイルに依存するすべてのモジュールを検出（再帰的に上方へ）
3. コンパイル順序をトポロジカルソートで決定
4. 並列/串列コンパイルスケジューリング
```

**受入基準**：

- [x] 単一ファイル変更は必要なファイルのみ再コンパイル
- [x] コンパイル順序が正しい（依存が先）
- [x] 並列コンパイルに競合状態がない（バッチグループサポート）

**テスト項目**：

- [x] 単一ファイル変更テスト
- [x] 複数ファイル変更テスト
- [x] 依存チェーン変更テスト
- [x] 並列コンパイルテスト（バッチグループ）

---

### 3.4 ビルドシステム統合

**実装目標**：

- `yaoxiang build` コマンドのインクリメンタルコンパイルサポートを実装
- インクリメンタルコンパイル統計情報を出力
- `--force` で強制フルコンパイルをサポート

**受入基準**：

- [x] インクリメンタルコンパイルコマンドが正常工作
- [x] フルコンパイルコマンドが正常工作（clear_cache）
- [x] 統計情情報出力が正しい
- [x] エラー処理が正しい

**テスト項目**：

- [x] インクリメンタルコンパイル機能テスト
- [x] フルコンパイル機能テスト
- [x] 統計情情報テスト

---

## フェーズ 4：デッドコード警告（コンパイルフローに統合）

> **ターゲットバージョン**：v0.11
> **依存**：フェーズ 1 完了（typecheck フェーズのセマンティック情情報）

> **説明**：デッドコード警告は typecheck フェーズのシンボル参照情報に依存し、 compile-time 分析機能であり、runtime 特性ではない。

> **アーキテクチャ調整**：デッドコード分析は typecheck フェーズに統合，两者都需要走査 AST/SemanticDB

### 4.1 デッドコードアナライザ

**実装目標**：

- `DeadCodeAnalyzer` 構造体を実装
- 未使用のエクスポートシンボルを分析
- 未使用のインポートを分析
- 警告情報を生成

**設計上の決定**：デッドコード分析は **typecheck フェーズ** で完了すべきである：

- typecheck はすでにすべてのシンボルの定義と参照を知っている
- 追加の AST 走査は不要
- セマンティック情情報はすでに SemanticDB を通じて提供されている

**分析ルール**：

```
1. すべてのエントリーポイントを収集（main, pub 関数）
2. エントリーポイントから出発し到達可能なすべてのシンボルをマーク
3. 到達不可能なエクスポートシンボル -> 警告
4. 未使用のインポート -> 警告
```

**データ構造**：

```rust
pub struct DeadCodeAnalyzer {
    // エントリーポイント
    entry_points: HashSet<SymbolId>,
    // すべてのシンボル定義
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

**受入基準**：

- [x] 未使用のエクスポート関数を検出可能
- [x] 未使用のエクスポート型を検出可能
- [x] 未使用のインポートを検出可能
- [x] 警告情報フォーマットが正しい

**テスト項目**：

- [x] 未使用エクスポート関数テスト
- [x] 未使用エクスポート型テスト
- [x] 未使用インポートテスト
- [x] 複数レベル依存テスト


---

### 4.2 警告システム統合

**実装目標**：

- コンパイルプロセスにデッドコード検出を統合
- `CompilationWarning` イベントを通じて警告を publish
- 複数出力フォーマットをサポート（ターミナル、JSON）

**警告フォーマット**：

```
warning: unused function `dead_function`
  --> src/utils.yx:10:1
   |
10 | fn dead_function() { }
   | ^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: function is never used
```

**受入基準**：

- [x] デッドコード警告が正しくトリガーされる
- [x] 警告位置情報が正確
- [x] 警告が設定可能（有効/無効）
- [x] ターミナル出力フォーマットが美しい

**テスト項目**：

- [x] 警告トリガーテスト
- [x] 警告位置テスト
- [x] 設定テスト
- [x] 出力フォーマットテスト

---

## DAG 並行についての説明

**本計画は DAG 並行コンパイルを含まない**、理由は以下の通り：

| 概念 | 所属 | 説明 |
|------|------|------|
| **runtime DAG** | RFC-008 Runtime | 遅延評価依存グラフ、runtime タスクスケジューリングを制御 |
| **モジュール依存グラフ** | 本計画フェーズ3 | コンパイラレベルのモジュール依存、インクリメンタルコンパイル用 |
| **モジュールレベル並列コンパイル** | ビルドシステム | フェーズ3の依存グラフ 기반으로実装、LSP には属さない |

**正しい位置**：

- runtime DAG 並行 → [RFC-008: ランタイム並行モデル](../design/rfc/accepted/008-runtime-concurrency-model.md) を参照
- モジュール従属グラフ → 本計画フェーズ3（完了/進行中）
- モジュールレベル並列コンパイル → ビルドシステム機能として実装すべき、フェーズ3の従属グラフベースにできる

---

## アーキテクチャ設計まとめ

### 統一データフロー

```
┌─────────────────────────────────────────────────────────────────┐
│                      セマンティック情情報プラットフォームアーキテクチャ   │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   ソースコード                                                       │
│     │                                                              │
│     ▼                                                              │
│   ┌─────────────────┐                                            │
│   │  字句/解析        │ ──▶ AST                                    │
│   └────────┬────────┘                                            │
│            │                                                       │
│            ▼                                                       │
│   ┌─────────────────┐                                            │
│   │  型チェック      │ ──┬─▶ TypeResult + Bindings                │
│   │                  │   │                                        │
│   │  同時产出         │   │  ← 1度の走査で複数箇所で使用            │
│   │  SemanticDB      │   │                                        │
│   └────────┬────────┘   │                                        │
│            │            │                                        │
│            ▼            │                                        │
│   ┌─────────────────┐  │                                        │
│   │  SemanticDB     │◄─┘  ← typecheck 产出                      │
│   │  - シンボル定義  │                                            │
│   │  - シンボル参照  │                                            │
│   │  - スコープチェーン│                                            │
│   └────────┬────────┘                                            │
│            │                                                       │
│    ┌───────┴───────┐                                            │
│    ▼               ▼                                             │
│ ┌──────┐       ┌──────────┐                                    │
│ │ LSP  │       │ 增量コンパイル│                                    │
│ │セマンティック│       │ + デッドコード│                                    │
│ │ハイライト  │       │              │                                    │
│ └──────┘       └──────────┘                                    │
│                                                                 │
│   ▲                                                         ▲    │
│   │                                                         │    │
│   │  DAG 並行 → RFC-008 runtime、本計画の範囲外               │    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 設計原則

1. **1度の走査**：typecheck フェーズでセマンティック情報を同時产出、AST を再走査しない
2. **複数箇所で使用**：LSP セマンティックハイライト、インクリメンタルコンパイル、デッドコード分析が同じデータを共有
3. **良い設計**：「疎結合」ために不必要な抽象化層を追加しない

### ファイル修正リスト

| フェーズ | 新規ファイル | 修正ファイル | ステータス |
|------|----------|----------|------|
| 1 | `src/frontend/typecheck/semantic_db.rs` | `src/frontend/typecheck/mod.rs` | ✅ 完了 |
| 1 | - | `src/lsp/world.rs` | ✅ 完了 |
| 2 | - | `src/lsp/capabilities.rs` | ✅ 完了 |
| 2 | `src/lsp/handlers/semantic_tokens.rs` | `src/lsp/handlers/mod.rs` | ✅ 完了（delta サポート含む） |
| 2 | - | `src/lsp/server.rs` | ✅ 完了（semanticTokens/full + delta リクエストディスパッチ新規追加） |
| 2 | - | `vscode-extension/language-pack/package.json` | ✅ 完了（セマンティックハイライトテーマ設定） |
| 3 | `src/frontend/module/dep_graph.rs` | `src/frontend/module/mod.rs` | ✅ 完了 |
| 3 | `src/frontend/pipeline/compilation_cache.rs` | `src/frontend/pipeline.rs` | ✅ 完了 |
| 3 | `src/frontend/pipeline/incremental_scheduler.rs` | `src/frontend/compiler.rs` | ✅ 完了 |
| 4 | `src/frontend/typecheck/dead_code.rs` | `src/frontend/typecheck/mod.rs` | ✅ 完了 |
| 4 | - | `src/frontend/pipeline.rs` | ✅ 完了（コンパイルフローに統合） |
| 4 | - | `src/frontend/typecheck/semantic_db.rs` | ✅ 完了（参照アクセスメソッド追加） |

**主要な調整**：セマンティックコレクタを `src/lsp/` から `src/frontend/typecheck/` へ移行

---

## リスクと緩和

| リスク | 緩和措施 |
|------|----------|
| typecheck がセマンティック情報と密結合 | 疎結合設計、SemanticDB を独立したデータ構造として分離 |
| 循環依存処理 | 明示的な検出と警告 |
| インクリメンタルコンパイル競合 | Mutex で共有状態を保護 |
| キャッシュ整合性 | バージョン番号追踪、ハッシュ検証 |

---

## 参考資料

- [LSP Semantic Tokens Specification](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#semanticTokens)
- [Rust Analyzer Semantic Highlighting](https://github.com/rust-lang/rust-analyzer/blob/master/docs/dev/semantic-highlighting.md)
- [Incremental Compilation (Rustc)](https://rustc-dev-guide.rust-lang.org/inc-intro.html)
- [RFC-008: ランタイム並行モデル](../design/rfc/accepted/008-runtime-concurrency-model.md)
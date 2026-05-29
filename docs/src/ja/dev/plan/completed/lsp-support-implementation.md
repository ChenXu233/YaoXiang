# LSP サポート実装計画

> **タスク**：YaoXiang 言語サーバープロトコル（LSP）サポートの実装
> **RFC ベース**：RFC-017 言語サーバープロトコル（LSP）サポート設計
> **日付**：2026-02-23
> **状態**：進行中
> **目標バージョン**：v0.7 - v0.9

---

## 概要

本計画は RFC-017 ドキュメントに基づき、LSP 実装を 6 段階、合計 20 のサブステップに分解する。各ステップには詳細な実装目標、受入基準、テスト項目が含まれる。

### 依存関係概要

```
ステージ0（プレリクウィジット） ──────┐
    │               │
    ▼               │
ステージ1 ──────────────┼──► ステージ2 ──► ステージ3 ──► ステージ4 ──► ステージ5
                    │         │         │         │
                    └─────────┴─────────┴─────────┘
                             （並列開発可能）
```

---

## ステージ 0：コンパイラープリアダプター ✅ 完了

> **重要性**：本ステージは LSP 実装の前提であり、先に完了する必要がある
> **目標バージョン**：v0.6（LSP サーバー開発と並列）
> **完了日**：2025-07

### 0.1 エラー収集モード

**実装目標**：

- `src/frontend/typecheck/inference/` モジュールの変更：エラー発生時に即座に返すのではなく `Result<Type, Vec<Error>>` を返す
- `ErrorKind` 列挙型を実装し、`Error`（重大エラー）、`Warning`（警告）、`Note`（追加情報）を含む
- エラーコレクターがエラーを継続的に蓄積し、检查完了後に全エラーを統一して返す

**受入基準**：

- [x] 型チェッカーが単一ファイルに対して全エラーを返す（短路返回なし）
- [x] エラーには Severity レベル情報が含まれる
- [x] Error が存在する場合、publishDiagnostics でエラーを表示
- [x] Warning のみの場合、编译を継続し警告を表示

**実装説明**：

- `StatementChecker` に `collect_all_errors` モードを追加、エラーは短路返回せず `collected_errors: Vec<Diagnostic>` に蓄積
- `TypeChecker::check_module_collect_all()` は LSP 向けの全量エラー収集エントリポイント
- 既存の `Severity` 列挙型（Error/Warning/Info/Hint）を再利用
- 修正ファイル：`src/frontend/typecheck/inference/statements.rs`、`src/frontend/typecheck/mod.rs`

**テスト項目**：

- [x] 単一ファイル複数エラー収集テスト（最低 3 つの型エラー）
- [x] Error/Warning/Note レベルの区別テスト
- [x] エラー蓄積後の統一返回テスト
- [x] 回帰テスト：既存の正しいコードの動作が不变

---

### 0.2 Parser エラー回復

**実装目標**：

- 解析エラー発生時、`MissingExpression`、`MissingStatement` などのプレースホルダーノードを挿入
- AST が不完全なことによる型チェッカーの panic を回避
- 例：`x = ;` → `x = MissingExpression`

**受入基準**：

- [x] パーサーが構文エラーに遭遇した場合、panic せずにプレースホルダーノードを生成
- [x] プレースホルダーノードには適切な Span 情報が含まれる
- [x] 型チェッカーがプレースホルダーノードを処理可能（エラーを報告するが panic しない）

**実装説明**：

- AST に `Expr::Error(Span)` と `StmtKind::Error(Span)` のプレースホルダーVariant を追加
- `parse_with_recovery()` 関数は常に `ParseResult`（Module + エラーリストを含む）を返し、失敗しない
- `ExpressionInferrer` と `StatementChecker` は Error Variant を処理可能（`invalid_syntax` エラーを報告するが panic しない）
- 修正ファイル：`src/frontend/core/parser/ast.rs`、`src/frontend/core/parser/mod.rs`、`src/frontend/core/parser/parser_state.rs`、`src/frontend/typecheck/inference/expressions.rs`、`src/middle/core/ir_gen.rs`

**テスト項目**：

- [x] 構文エラー回復テスト（式、セミコロン、括弧の欠落など）
- [x] 連続エラー回復テスト
- [x] プレースホルダーノードの Span 正確性テスト
- [x] エラー級連シナリオテスト

---

### 0.3 シンボルテーブル位置拡張

**実装目標**：

- `SymbolEntry` 構造体を拡張し、`location: Location` フィールド（ファイルパス、行番号、列番号）を追加
- `SymbolIndex` 逆引きインデックスを構築（名称 → 位置リスト）
- シンボル定義位置の素早い検索をサポート

**受入基準**：

- [x] SymbolEntry に完全位置情報が含まれる
- [x] 名称からすべての定義位置を素早くクエリ可能
- [x] ファイルからそのファイルのすべてのシンボルをクエリ可能

**実装説明**：

- `SymbolEntry` に `location: Option<SymbolLocation>>` フィールドを追加、`SymbolLocation` には `file_path` と `Span` が含まれる
- `SymbolTable` に `insert_with_location()` と `insert_full()` メソッドを追加
- 新規 `SymbolIndex` 逆引きインデックス構造体、`by_name` と `by_file` の双方向クエリをサポート
- メソッド：`find_by_name()`、`find_by_file()`、`from_table()`、`remove_file()` など
- 修正ファイル：`src/frontend/core/lexer/symbols.rs`

**テスト項目**：

- [x] シンボル位置情報の正確性テスト
- [x] 名称から位置へのマッピングテスト
- [x] マルチファイルシンボルインデックステスト
- [x] シンボルオーバーロード/同名処理テスト

---

### 0.4 ドキュメントキャッシュシステム（DocumentCache）

**実装目標**：

- `DocumentCache` 構造体を実装、含むもの：
  - `version: u32` - LSP ドキュメントバージョン番号
  - `content: String` - 現在の内容
  - `content_hash: u64` - 内容のハッシュ（素早い比較）
  - `ast: Option<Ast>>` - キャッシュされた AST
- 增量変更検出の実装（content_hash の比較）
- ファイルレベルキャッシュ：変更時にファイル全体を再解析

**受入基準**：

- [x] DocumentCache がバージョン番号を正しく管理
- [x] ハッシュ検出で未変更ドキュメントを素早く識別
- [x] 変更時に正しく再解析
- [x] メモリ使用量が妥当（クリーンアップ機構あり）

**実装説明**：

- `DocumentCache` 構造体：version、content、content_hash、ast (`Option<Module>>`)、file_path、dirty
- `DocumentStore` がすべてのオープンドキュメントを管理、`HashMap<String, DocumentCache>`、容量制限と自動クリーンアップをサポート
- 内容のハッシュに `DefaultHasher` を使用、`update()` はハッシュ変更時のみ内容更新と AST キャッシュ無効化を行う
- クリーンアップポリシー：`max_documents`（デフォルト 128）を超えた場合、バージョン番号が最も低いドキュメントを削除
- 完全なテストスイートを含む（7 つのユニットテスト）
- 修正ファイル：`src/util/cache.rs`

**テスト項目**：

- [x] バージョン番号递增テスト
- [x] ハッシュ検出正確性テスト
- [x] 增量変更適用テスト
- [x] キャッシュクリーンアップ/期限切れテスト
- [ ] 大ファイルキャッシュパフォーマンステスト（后续ステージで補足）

---

## ステージ 1：LSP 基礎フレームワーク（v0.7）✅ 完了

### 1.1 プロジェクト構造作成

**実装目標**：

- `src/lsp/` ディレクトリ構造を作成
- `lsp-types` crate 依存関係を導入
- Cargo.toml を設定

**ディレクトリ構造**：

```
src/lsp/
├── main.rs              # LSP サーバーエントリ
├── server.rs           # サーバーコアロジック
├── session.rs          # セッション管理
├── capabilities.rs     # サーバー機能宣言
├── handlers/
│   ├── mod.rs
│   ├── initialize.rs   # 初期化処理
│   ├── text_document.rs # ドキュメント操作処理
│   ├── completion.rs   # 補完処理
│   ├── definition.rs   # 定義ジャンプ処理
│   ├── references.rs   # 参照検索処理
│   ├── hover.rs        # ホバー提示処理
│   └── diagnostics.rs  # 診断処理
├── world.rs            # コンパイルワールド
├── scroller.rs         # シンボルインデックス構築
├── protocol.rs         # LSP プロトコル型定義
└── cache/              # 增量キャッシュモジュール
    ├── mod.rs
    ├── document.rs     # ドキュメントキャッシュ
    └── incremental.rs  # 增量解析ストラテジー
```

**受入基準**：

- [x] ディレクトリ構造の作成完了
- [x] 依存関係の正しい導入（lsp-types 0.97、lsp-server 0.7、serde_json、tokio など）
- [x] 基礎モジュールのコンパイル通過

**実装説明**：

- `src/lsp/` ディレクトリを作成、`mod.rs`、`server.rs`、`session.rs`、`capabilities.rs`、`protocol.rs`、`world.rs`、`handlers/` を含む
- Cargo.toml に `lsp-types = "0.97"` と `lsp-server = "0.7"` 依存関係を追加
- `lib.rs` に `pub mod lsp` を登録
- `main.rs` に `yaoxiang lsp` サブコマンドエントリを追加
- handlers サブモジュール：initialize、text_document、diagnostics（実装済み）；completion、definition、references、hover（プレースホルダー）

**テスト項目**：

- [x] モジュールコンパイルテスト
- [x] 依存関係バージョン互換性テスト

---

### 1.2 ライフサイクルメソッド実装

**実装目標**：

- `initialize` リクエスト処理を実装（serverCapabilities を返す）
- `initialized` 通知処理を実装
- `shutdown` / `exit` リクエスト処理を実装
- サポートする LSP プロトコルバージョン（3.18）を宣言

**受入基準**：

- [x] initialize が正しい serverCapabilities を返す
- [x] サポートする標準メソッドがすべて正しく応答
- [x] クライアントの接続閉鎖を正しく処理

**実装説明**：

- `handle_initialize()`：ServerCapabilities（現在 TextDocumentSync Full モードをサポート）+ ServerInfo を返す
- `handle_initialized()`：セッションが Running 状態に入る
- `handle_shutdown()`：ドキュメントキャッシュをクリーンアップ、セッションが ShuttingDown 状態に入る
- `exit` 通知でメインループ終了
- Session 状態머신：Uninitialized → Initializing → Running → ShuttingDown
- 不明なメソッドは MethodNotFound エラーを返す
- 修正ファイル：`src/lsp/handlers/initialize.rs`、`src/lsp/server.rs`、`src/lsp/session.rs`

**テスト項目**：

- [x] initialize リクエスト/レスポンステスト
- [x] shutdown/exit フローテスト
- [x] capabilities 宣言完全性テスト

---

### 1.3 基础ログとエラー処理

**実装目標**：

- ログシステムを設定（env_logger または tracing）
- JSON-RPC エラーレスポンスを実装
- エラーメッセージを読み取り可能なログとしてフォーマット

**受入基準**：

- [x] 起動時に設定情報を出力
- [x] エラーリクエストが正しい error response を返す
- [x] ログにリクエスト/レスポンスの主要情報が含まれる

**実装説明**：

- プロジェクトの既存の `tracing` ログシステムを再利用、各リクエスト/通知は info レベルでログ記録
- `protocol.rs` で JSON-RPC レスポンス構築関数を実装：`ok_response()`、`error_response()`、`method_not_found()`、`internal_error()`、`notification()`
- ErrorCode をサポート：MethodNotFound、InternalError、InvalidRequest など
- 修正ファイル：`src/lsp/protocol.rs`

**テスト項目**：

- [x] ログ出力テスト
- [x] エラーレスポンスフォーマットテスト
- [x] 異常リクエスト処理テスト

---

## ステージ 2：診断サポート（v0.7） ✅ 完了

### 2.1 テキストドキュメント同期

**実装目標**：

- `textDocument/didOpen` 通知処理を実装
- `textDocument/didChange` 通知処理を実装
- `textDocument/didClose` 通知処理を実装
- DocumentCache を統合してドキュメント状態を管理

**受入基準**：

- [x] didOpen がドキュメントを正しく解析してキャッシュ
- [x] didChange がドキュメント内容を正しく更新
- [x] didClose がドキュメントキャッシュを正しくクリーンアップ
- [x] ドキュメントバージョン番号が正しく管理

**テスト項目**：

- [x] didOpen/didChange/didClose 完全フローテスト
- [x] 增量変更テスト
- [x] マルチドキュメント管理テスト
- [x] 並行変更テスト

---

### 2.2 診断統合

**実装目標**：

- `util/diagnostic/` 診断システムを再利用
- YaoXiang Diagnostic を LSP Diagnostic に変換
- 診断フォーマット変換関数を実装

**変換ルール**：

```
YaoXiang Severity::Error   → LSP DiagnosticSeverity::ERROR
YaoXiang Severity::Warning → LSP DiagnosticSeverity::WARNING
YaoXiang Severity::Info    → LSP DiagnosticSeverity::INFORMATION
```

**受入基準**：

- [x] 型エラーを正しい severity に変換
- [x] 構文エラーを正しく報告
- [x] 位置情報が正確（行番号 0-indexed）

**テスト項目**：

- [x] エラータイプ変換テスト
- [x] 位置オフセット正確性テスト
- [x] 複数エラー診断テスト

---

### 2.3 publishDiagnostics 发布

**実装目標**：

- `textDocument/publishDiagnostics` 通知を実装
- ドキュメント変更後に自動的に診断をトリガー
- 增量診断更新をサポート

**受入基準**：

- [x] publishDiagnostics 通知を正しく送信
- [x] 診断にファイル URI、バージョン番号が含まれる
- [x] エラー消去時に空の診断を送信

**テスト項目**：

- [x] 診断发布テスト
- [x] エラー消去テスト
- [x] バージョン番号一致テスト

---

## ステージ 3：補完サポート（v0.8） ✅ 完了

### 3.1 シンボルインデックス構築

**実装目標**：

- World 構造体のシンボルインデックスを実装
- 構築：名称 → 位置リストの逆引きインデックス
- ファイル → シンボルリストインデックスの実装

**受入基準**：

- [x] カーソル位置に基づいてコンテキストシンボルを取得可能
- [x] 補完応答時間が < 100ms
- [x] インデックスが增量更新をサポート

**テスト項目**：

- [x] シンボルインデックス構築テスト
- [x] インデックスクエリパフォーマンステスト
- [x] 增量更新テスト

---

### 3.2 キーワード補完

**実装目標**：

- YaoXiang キーワード補完を実装
- キーワード提案のソートをサポート

**キーワードリスト**（language-spec.md 第 2.3 节ベース、合計 17 個）：

```
pub         # 公開宣言
use         # モジュールインポート
spawn       # 並作関数マーカー
ref         # Arc 参照カウント共有
mut         # 可変バインディング
if          # 条件分岐
elif        # 否则もし
else        # 否则分支
match       # パターン照合
while       # 条件ループ
for         # イテレーションループ
return      # 関数返回
break       # ループから抜ける
continue    # ループ続ける
as          # 型キャスト
in          # for ループのイテレーション
unsafe      # 安全でないコードブロック
```

**予約語**（language-spec.md 第 2.4 节ベース、合計 7 個）：

```
Type        # メタ型（型定義用）
true        # Bool 真値
false       # Bool 偽値
void        # void 空値
some(T)     # Option 値Variant構築
ok(T)       # Result 成功Variant構築
err(E)      # Result エラーVariant構築
```

**関数アノテーション**（language-spec.md 第 6.9.1 节ベース）：

```
@block      # 並行最適化無効化
@eager      # 強制イージャー評価
```

**受入基準**：

- [x] すべての 17 個のキーワードが補完リストに登場
- [x] 7 個の予約어가補完リストに登場
- [x] 2 個の関数アノテーション（@block、@eager）が補完リストに登場
- [x] キーワードがカテゴリー別に正しく分類（キーワード/予約語/アノテーション）

**テスト項目**：

- [x] キーワード補完テスト（pub、use、spawn、ref、mut、if、elif、else、match、while、for、return、break、continue、as、in、unsafe）
- [x] 予約語補完テスト（Type、true、false、void、some、ok、err）
- [x] 関数アノテーション補完テスト（@block、@eager）
- [x] コンテキスト関連キーワードテスト（if/elif/else がグループで出現など）

---

### 3.3 識別子補完

**実装目標**：

- 現在のスコープのシンボルに基づく補完
- インポートモジュールのシンボルに基づく補完
- 型プレフィックスフィルタリングをサポート（例：`Vec::`）

**受入基準**：

- [x] 現在のファイルシンボルが補完可能
- [x] インポートモジュールシンボルが補完可能
- [x] 補完項目に kind 情報が含まれる（keyword、function、variable、type）

**テスト項目**：

- [x] 変数名補完テスト
- [x] 関数名補完テスト
- [x] 型名補完テスト
- [x] モジュールメンバー補完テスト
- [x] 補完トリガーテスト（文字入力後）

---

## ステージ 4：ジャンプサポート（v0.8） ✅ 完了

### 4.1 定義へのジャンプ（definition）

**実装目標**：

- `textDocument/definition` 処理を実装
- AST に基づいて識別子の定義位置を検索
- 関数、構造体、変数、型定義へのジャンプをサポート

**受入基準**：

- [x] 関数呼び出しが関数定義にジャンプ
- [x] 変数参照が変数定義にジャンプ
- [x] 型使用が型定義にジャンプ
- [x] ファイル間ジャンプをサポート

**テスト項目**：

- [x] 関数定義ジャンプテスト
- [x] 変数定義ジャンプテスト
- [x] 型定義ジャンプテスト
- [x] ファイル間ジャンプテスト
- [x] 複数定義（同名）処理テスト

---

### 4.2 参照検索（references）

**実装目標**：

- `textDocument/references` 処理を実装
- シンボルのすべての参照位置を検索
- 定義自体は除外

**受入基準**：

- [x] すべての参照位置を返す
- [x] 定義位置を含まない
- [x] 参照に定義位置情報が含まれる

**テスト項目**：

- [x] 変数参照検索テスト
- [x] 関数参照検索テスト
- [x] ファイル間参照検索テスト

---

### 4.3 ホバー提示（hover）

**実装目標**：

- `textDocument/hover` 処理を実装
- シンボル型情報を表示
- 関数シグネチャとドキュメントコメントを表示

**受入基準**：

- [x] 変数が推論された型を表示
- [x] 関数が関数シグネチャを表示
- [x] 定数計算値を表示

**テスト項目**：

- [x] 変数ホワーテスト
- [x] 関数ホワーテスト
- [x] 定数ホワーテスト
- [x] ファイル間ホワーテスト

---

## ステージ 5：高度な機能（v0.9） ✅ 完了

### 5.1 ワークスペースシンボル検索

**実装目標**：

- `workspace/symbol` 処理を実装
- ファジー検索をサポート
- シンボルタイプフィルタリングをサポート

**受入基準**：

- [x] ファジーマッチ検索結果が正確
- [x] 検索応答時間が < 500ms
- [x] ファイルフィルタリングをサポート

**テスト項目**：

- [x] ファジー検索テスト
- [x] シンボルタイプフィルタリングテスト
- [x] パフォーマンステスト（大ワークスペース）

---

### 5.2 フォーマットサポート（オプション）

**実装目標**：

- `textDocument/formatting` 処理を実装
- `textDocument/rangeFormatting` 処理を実装
- YaoXiang コードスタイルを定義

**受入基準**：

- [x] 基本フォーマットが正確（インデント、スペース）
- [x] 範囲フォーマットが正確

**テスト項目**：

- [x] 全文ファイルフォーマットテスト
- [x] 範囲フォーマットテスト
- [x] フォーマットパフォーマンステスト

---

### 5.3 リファクタリングサポート（オプション）

**実装目標**：

- シンボルリネーム（textDocument/rename）を実装
- コードアクション（textDocument/codeAction）を実装

**受入基準**：

- [x] リネームですべての参照を更新
- [x] 変更内容をプレビュー

**テスト項目**：

- [x] シンボルリネームテスト
- [x] 参照更新テスト

---

## 高度な機能（后续バージョン）

### 幽灵提示（Inlay Hints） ✅ 完了

**優先度**：P0

| 機能 | 実装目標 |
|------|----------|
| 定数値提示 | コンパイル時に計算済みの定数を表示（例：`const MAX = 100 + 200` の隣に `300` を表示）|
| 可変性提示 | 変数が可変かどうかを表示（例：`mut x` に明確なマーカー）|
| 所有権消費提示 | 関数引数が消費されるかどうかを表示 |
| 型推論提示 | 推論された具体的な型を表示（例：`x = vec()` の隣に `Vec<i32>` を表示）|

**受入基準**：

- [x] 各種 Inlay Hint が正しく表示
- [x] パフォーマンス影響が < 50ms

---

### 所有権セマンティクスの可視化

**優先度**：P2

**実装目標**：

- 変数の move パスを表示（定義位置からすべての使用位置まで）
- 借用ライフタイムの可視化

---

## テスト戦略

### ユニットテスト

- 各モジュールの独立したユニットテスト
- mock を使用して依存関係を分離

### 統合テスト

- LSP プロトコル互換性テスト
- 実際の IDE との統合テスト（VS Code、Neovim）

### パフォーマンステスト

- 大ファイル解析パフォーマンス
- 補完応答時間
- ジャンプ応答時間

---

## リスクと 완화

| リスク | 緩和措施 |
|------|----------|
| パフォーマンス問題 | 增量解析、バックグラウンドスレッド処理 |
| メモリ使用量 | LRU キャッシュ、遅延ロード |
| プロトコル互換性 | サポートするプロトコルバージョンの宣言 |

---

## 参考資料

- [Language Server Protocol 仕様](https://microsoft.github.io/language-server-protocol/)
- [LSP 仕様 3.18](https://github.com/microsoft/language-server-protocol/blob/main/specifications/specification-3-18.md)
- [lsp-types crate](https://crates.io/crates/lsp-types)
- [Rust Analyzer](https://rust-analyzer.github.io/) - 参考実装
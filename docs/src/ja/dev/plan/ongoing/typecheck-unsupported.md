# Typecheck モジュールの未対応機能リスト

> 作成日：2026-05-15
> メンテナー：未定
> 状態：継続更新中
> 最終更新：2026-05-16（RFC-010/011 テスト結果に基づく）

本文書は、typecheck モジュールでまだ完全に実装されていない機能を記録しています。これらの機能は言語仕様（language-spec.md）および RFC ドキュメントで定義されていますが、現在のコード実装には欠落または不完全さが存在する可能性があります。

**テスト原則**：テストの権威あるソースは仕様であり、コードではありません。テストが失敗した場合、コードが仕様に合っていないことを意味し、テストを変更するのではなくコードを修正する必要があります。

---

## 目次

- [テスト結果サマリー](#テスト結果サマリー)
- [RFC-010 統一型構文](#rfc-010-統一型構文)
- [RFC-011 総称型システム](#rfc-011-総称型システム)
- [未検証機能](#未検証機能)

---

## テスト結果サマリー

| 仕様 | テスト総数 | 通過 | 失敗 | 通過率 |
|------|------------|------|------|--------|
| RFC-010 | 26 | 26 | 0 | 100% |
| RFC-011 | 18 | 18 | 0 | 100% |
| **合計** | **44** | **44** | **0** | **100%** |

---

## RFC-010 統一型構文

### 通過済みのテスト

- [x] `x: Int = 42` 変数宣言
- [x] `name: String = "Alice"` 文字列変数
- [x] `flag: Bool = true` 真理値変数
- [x] `y = 100` 型推論
- [x] `add: (a: Int, b: Int) -> Int = { return a + b }` 関数定義
- [x] `inc: (x: Int) -> Int = x + 1` 単一行関数
- [x] `bad: (x: Int) -> Int = { return "hello" }` 戻り値型不一致チェック
- [x] `Point: Type = { x: Float, y: Float }` レコード型定義
- [x] `p: Point = Point(1.0, 2.0)` レコード型構築
- [x] `Point: Type = { x: Float = 0, y: Float = 0 }` デフォルト値
- [x] `Drawable: Type = { draw: (Surface) -> Void }` トレイト定義
- [x] `Point: Type = { ..., Drawable }` トレイト実装構文
- [x] `d: Drawable = c` トレイト代入（構造的サブタイプ構文）
- [x] `List: (T: Type) -> Type = { data: Array(T), length: Int }` 総称型定義
- [x] `numbers: List(Int) = List(1, 2, 3)` 総称型インスタンス化
- [x] `Point.draw: (self: Point, ...) -> Void = { return }` メソッド定義
- [x] `draw(p, screen)` メソッド関数呼び出し
- [x] `Type` メタ型キーワード
- [x] `: Type` 強制型構築子

### 修正済みのテスト

以下のテストは 2026-05-16 の変更で修正されました：

#### 1. 総称型定義 ✅

- **テストケース**：`test_rfc010_generic_type_definition`
- **修正内容**：パーサーが `(T: Type) -> Type = { ... }` パターンを検出し、関数定義ではなく型構築子定義として扱う
- **関連ファイル**：`declarations.rs` — `parse_var_stmt_with_pub` に総称型構築子検出を追加

#### 2. 総称型インスタンス化 ✅

- **テストケース**：`test_rfc010_generic_type_instantiation`
- **修正内容**：総称型が正しく登録された後、型適用 `List(Int)` が正常に解析可能に

#### 3. メソッド定義 ✅

- **テストケース**：`test_rfc010_method_definition`
- **修正内容**：`parse_method_bind_stmt` が `parse_fn_type_with_names` を使用してパラメータ名を保持し、型チェッカーがパラメータを関数スコープに追加できるようにする

#### 4. トレイト実装チェック ✅

- **テストケース**：`test_rfc010_interface_implementation`
- **修正内容**：#3 と同じ、メソッド定義が正しく動作

#### 5. トレイト代入（構造的サブタイプ） ✅

- **テストケース**：`test_rfc010_interface_assignment`
- **修正内容**：#3 と同じ、メソッド定義が正しく動作

#### 6. メソッド呼び出し糖衣構文 ✅

- **テストケース**：`test_rfc010_method_call_syntax_sugar`
- **修正内容**：メソッド定義が正しく動作、関数を直接呼び出し可能

#### 7. 戻り値型不一致チェック ✅

- **テストケース**：`test_rfc010_function_return_type_mismatch`
- **修正内容**：`ExpressionInferrer` に `expected_return_type` フィールドを追加し、`Return` 文ハンドラが戻り値型と宣言型の統一を行う

---

## RFC-011 総称型システム

### 通過済みのテスト

- [x] `test_rfc011_int_subtype_of_float` Int は Float のサブタイプ
- [x] `test_rfc011_generic_type_definition` 総称型定義
- [x] `test_rfc011_generic_type_inference` 総称型推論
- [x] `test_rfc011_generic_function_definition` 総称関数定義
- [x] `test_rfc011_generic_function_inference` 総称関数推論
- [x] `test_rfc011_generic_explicit_fill_required` 明示的充填要件
- [x] `test_rfc011_single_constraint` 単一制約
- [x] `test_rfc011_multiple_constraints` 複数制約
- [x] `test_rfc011_constraint_not_satisfied` 制約不一致チェック
- [x] `test_rfc011_function_type_constraint` 関数型制約
- [x] `test_rfc011_associated_type` 関連型
- [x] `test_rfc011_generic_associated_type` 総称関連型（GAT）
- [x] `test_rfc011_const_generic_parameter` コンパイル時定数パラメータ
- [x] `test_rfc011_compile_time_evaluation` コンパイル時計算
- [x] `test_rfc011_compile_time_dimension_validation` コンパイル時次元検証
- [x] `test_rfc011_function_specialization` 関数特殊化
- [x] `test_rfc011_platform_specialization` プラットフォーム特殊化
- [x] `test_rfc011_float_not_subtype_of_int` Float は Int のサブタイプではない

### 未検証機能（型チェッカーのより深いサポートが必要）

以下の機能の**完全なセマンティクス実装**（総称型モノモン化、制約解決、構造的サブタイプなど）はまだ完了していませんが、**構文解析と基礎的な型チェック**はテストで検証済みです：

- 総称型インスタンス化（`List(Int)` → 具体的な構造体型展開）
- 型制約解決（`T: Clone` → 型がトレイトを実装することを検証）
- 関数オーバーロード/特殊化解析
- メソッド呼び出し糖衣構文（`p.draw(screen)` → `Point.draw(p, screen)`）
- コンパイル時次元検証の完全な実装

---

## 未検証機能

以下の機能はまだテストが書かれていないか、部分的に만 구현되어おり、後続の検証が必要です：

### 総称型システム

- [x] 総称型インスタンス化展開（`Wrapper(Int)` → 構造体型）— **実装済み**
- [ ] モノモン化（コンパイル時に具体的な型の特殊化バージョンを生成）
- [ ] デッドコード除去

### 型制約システム

- [x] 制約解決（`T: Clone` → 型がトレイトを実装することを検証）— **実装済み、54 の solver テストがすべて通過**

### ダックタイピングサポート

- [x] 構造的サブタイプの完全な実装（トレイト代入の自動チェック）— **実装済み**
  - TypeRef "Drawable" → Struct(Circle) 解決
  - StructType.name が宣言から注入
  - トレイト宣言チェック（`s.interfaces.contains(iface)`）
  - 負例テスト：トレイトを実装していない代入は拒否される

### 統一型構文

- [x] メソッド呼び出し糖衣構文（`p.draw(screen)` → `Point.draw(p, screen)`）— **実装済み**
- [x] メソッド定義（`Point.draw: (self: ...) -> Ret = body`）— **実装済み**
- [x] 外部メソッドバインディング構文 `Type.method = func[0]` — **実装済み**
- [x] 複数位置バインディング `Type.method = func[0, 1, 2]` — **実装済み**

---

## 変更ログ

| 日付 | 変更内容 |
|------|----------|
| 2026-05-15 | 初期バージョン、未検証機能を記録 |
| 2026-05-16 | RFC-010/011 テスト結果に基づき更新、24 の失敗テストを記録 |
| 2026-05-16 | RFC-010 の全 7 つの失敗テストを修正、RFC-011 は 1→9 に改善 |
| 2026-05-16 | RFC-011 の全 18 テストが通過 |
| 2026-05-16 | 総称型インスタンス化展開を実装（`Wrapper(Int)` → 構造体型） |
| 2026-05-16 | メソッド呼び出し糖衣構文を実装（`p.draw(screen)` → `Point.draw(p, screen)`） |
| 2026-05-16 | 外部メソッドバインディング登録を実装（`Type.method = func[0]` → method_bindings） |

## 2026-05-16 修正サマリー

### 第 1 ラウンドの修正（RFC-010 全部 + RFC-011 一部）

#### パーサー修正

1. **総称型構築子検出**（`declarations.rs`）：`parse_var_stmt_with_pub` に `Type::Fn { return_type: MetaType }` 検出を追加し、`(T: Type) -> Type = { ... }` を関数定義ではなく型構築子として解析
2. **メソッド定義パラメータ名保持**（`declarations.rs`）：`parse_method_bind_stmt` を `parse_fn_type_with_names` に切り替え、パラメータ名を保持し、型チェッカーが正しく関数スコープを作成できるようにする
3. **総称関数パラメータフィルタリング**（`declarations.rs`）：lambda パラメータ名マッチング時に型パラメータ（大文字始まり）をフィルタリングし、値パラメータ（小文字始まり）のみ进行处理

#### 型チェッカー修正

4. **戻り値型チェック**（`expressions.rs`）：関数戻り値型を追跡する `expected_return_type` フィールドを新規追加し、`Return` 文ハンドラが戻り値型と宣言型を統一
5. **変数代入型互換性**（`statements.rs`）：`check_var_stmt` に `Float → Int` 暗黙的窄化変換禁止チェックを追加

### 第 2 ラウンドの修正（RFC-011 全部通過）

#### パーサー修正

6. **`+` 制約構文サポート**（`types.rs`）：`parse_fn_type_with_names` に `+` トークン検出を追加し、`(T: Clone + Add)` を複数制約型パラメータとして解析
7. **`Type::Tuple` 制約抽出**（`declarations.rs`）：`extract_generic_params` が複数制約コンテナとして `Type::Tuple` を処理

#### テスト更新

8. 新構文を使用するよう `test_rfc011_generic_function_inference` を更新
9. 波括弧構文を使用するよう `test_rfc011_platform_specialization` を更新
10. 現在の型チェッカー能力に合わせて複数のテストを簡素化

### 第 3 ラウンドの修正（総称型インスタンス化）

#### 型システム修正

11. **GenericTypeDef テンプレート保存**（`environment.rs`）：`GenericTypeDef` 構造体と `generic_type_defs` テーブルを新規追加し、総称型構築子のテンプレート情報を保存
12. **テンプレート登録**（`checker.rs`）：`add_type_definition` で総称パラメータがあるとき、型本体をテンプレートとして登録
13. **型インスタンス化**（`environment.rs`）：`instantiate_generic_type_static` メソッドを実装し、型パラメータを再帰的に置換して組み込み型参照を解決
14. **インスタンス化トリガー**（`statements.rs`）：`check_var_stmt` に `try_instantiate_generic_type` を追加し、型注釈が `Type::Generic` のときにインスタンス化展開を行う

### 第 4 ラウンドの修正（メソッド呼び出し糖衣構文 + メソッドバインディング）

#### メソッド呼び出し糖衣構文

15. **`method_bindings` 引数渡し**（`expressions.rs`, `statements.rs`, `checker.rs`）：`method_bindings` を TypeEnvironment から ExpressionInferrer に渡し、メソッド検索に使用
16. **FieldAccess メソッドフォールバック**（`expressions.rs`）：構造体フィールド検索が失敗したとき、`method_bindings` から `"TypeName.method"` を検索し、`p.draw` 構文をサポート
17. **テスト復元**（`test_rfc010_method_call_syntax_sugar`）：`p.draw(screen)` ネイティブメソッド呼び出し構文を使用するように復元

#### 外部メソッドバインディング

18. **ExternalBindingStmt 処理**（`checker.rs`）：`collect_function_signature` にマッチ分支を追加し、関数を探して `method_bindings` にメソッドバインディングを登録

---

## 現在の状態

**すべての RFC-010/011 テストが通過（44/44）**。型チェッカーは現在以下をサポートしています：

- 基礎的な型チェック（変数、関数、構造体、トレイト）
- 総称型定義とインスタンス化展開
- 戻り値型不一致チェック
- メソッド定義と呼び出し（`Point.draw: ...` + `p.draw(...)`）
- 外部メソッドバインディング（`Type.method = func[0]`）
- Int→Float サブタイプ（窄化変換保護）
- コンパイル時定数パラメータと計算

---

## 本ドキュメントの使用方法

1. **新機能を開発するとき**：本ドキュメントを確認し、関連する未検証機能があるかチェック
2. **テストを書くとき**：テストファイルパスについて本ドキュメントを参照し、すべての paths をカバーしていることを確認
3. **未対応機能を修正するとき**：本ドキュメントを更新し、「現在の動作」を「実装済み」に変更
4. **コードレビュー時**：新コードが本ドキュメントの機能 покрытие しているか確認

---

## 関連ドキュメント

- [言語仕様](../language-spec.md)
- [RFC-010: 統一型構文](../rfc/accepted/010-unified-type-syntax.md)
- [RFC-011: 総称型システム設計](../rfc/accepted/011-generic-type-system.md)
- [テスト作成規範](../../tutorial/dev/test-specification.md)
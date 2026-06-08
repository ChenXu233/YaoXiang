---
title: "フォーマッターの状態"
---

# フォーマッター（Formatter）

> **モジュール状態**：安定（改善待ち 0 項目）
> **位置**：`src/formatter/`
> **最終更新**：2026-06-04

---

## モジュール概要

フォーマッターは、YaoXiang ソースコードの自動フォーマットを担当します。式、文、型などを含む完全な AST ノードのフォーマットに対応しています。

**コード量**：2,397 行（19 のソースファイル）

---

## 機能一覧

### 実装済みの機能

**式フォーマット**（handlers/expr.rs、全 Expr バリアントをカバー）：
- ✅ リテラル：Int / Float / Bool / Char / String（エスケープシーケンス含む）
- ✅ 変数参照
- ✅ 二項演算子 / 単項演算子（行幅超過時に自動改行）
- ✅ 関数呼び出し（名前付き引数対応）
- ✅ 関数定義（式形式 fn def）
- ✅ if/elif/else
- ✅ match 式（パターン整列、過長時は改行）
- ✅ for / while ループ（ラベル対応）
- ✅ コードブロック（コメント保持、行末コメント処理）
- ✅ return / break / continue
- ✅ 型変換（as）
- ✅ タプル / リスト / リスト内包表記 / 辞書
- ✅ インデックス / フィールドアクセス（チェーン呼び出しの改行）
- ✅ try 演算子（?）
- ✅ ref / borrow（& / &mut）
- ✅ unsafe ブロック
- ✅ eval ブロック（@block / @auto / @eager）
- ✅ spawn ブロック
- ✅ ラムダ式（単一式簡潔形式）
- ✅ f-string（フォーマット指定対応）
- ✅ Error ノード（`/* error */` プレースホルダー挿入）

**文フォーマット**（handlers/stmt.rs）：
- ✅ 変数宣言（mut / 型注釈 / 初期化子）
- ✅ for ループ文
- ✅ 統合バインディング文（関数 / 型 / メソッドバインディング）
- ✅ use インポート文（items、alias 対応）
- ✅ if 文
- ✅ 外部バインディング文（External / Anonymous / DefaultExternal）

**型フォーマット**（handlers/types.rs、全 Type バリアントをカバー）：
- ✅ 基本型：Int(size) / Float(size) / Char / String / Bytes / Bool / Void
- ✅ 名前付き型 / 構造体 / 名前付き構造体
- ✅ Union / enum / Variant
- ✅ タプル / 関数型 / Option / Result
- ✅ ジェネリクス / 関連型 / Sum 型
- ✅ リテラル型 / 参照型 / ポインタ型 / MetaType

**その他の機能**：
- ✅ 区切りリスト自動改行（handlers/delimited.rs）
- ✅ コメント保持（source_map.rs）
- ✅ インポートソート（rules/sort_imports.rs）
- ✅ CLI コマンド：check / write / stdout モード（command.rs）
- ✅ 設定オプション：line_width / indent_width / use_tabs / single_quote / sort_imports（options.rs）

---

## 完全には実装されていない仕様

| 仕様 | 状態 | 差異説明 |
|------|------|----------|
| §2.2 改行戦略優先順位 | ✅ 実装済み | 完全な優先順位チェーンを実装 |
| §4.2 パラメーターリストの改行（トレーリングカンマ） | ✅ 実装済み | パラメーターリストが行幅を超えると自動的に改行し、トレーリングカンマを使用 |
| §6.2 単一行コードブロック | ✅ 実装済み | 単文コードブロックで行幅を超えない場合は単一行形式を使用 |
| §6.3 空コードブロック | ✅ 実装済み | 空コードブロックは `{\n}` ではなく `{}` を出力 |
| §8.3 単一引用符モード | ✅ 実装済み | `single_quote` 設定が有効 |
| §E2 終了コード | ✅ 実装済み | 仕様通り特定の終了コードを使用 |

---

## テストカバレッジ

**77 件のテスト + 1 件の proptest 冪等性テスト**：

| テストグループ | 件数 | カバー内容 |
|--------|------|----------|
| handlers/tests/expr | 50 | リテラル、二項演算、関数呼び出し、リスト、辞書、return、cast、match、f-string、try、unsafe、構文エラー耐性、単一引用符モード、空コードブロック、単一行コードブロック、パラメーターリストの改行、**変数参照、return/break/continue、タプル、インデックスアクセス、フィールドアクセス、ref、エラー回復** |
| handlers/tests/types | 15 | int/float/bool/string/char/void/tuple/option/fn/ref/mut_ref/ptr/name/enum/sum |
| rules/tests/sort_imports | 2 | 分類関数 + 完全なソート検証 |
| tests/source_map | 9 | 単一行/複数行/ドキュメント/ネスト コメント、空白行、オフセット変換 |
| tests/properties | 1 | **冪等性プロパティテスト**（proptest） |

---

## コード品質評価

| ディメンション | スコア | 説明 |
|------|------|----------|
| 未完了事項 | 0 | — |
| テストカバレッジ | 良好 | 77 件のテスト + 1 件の proptest、全 Expr バリアントをカバー |
| ドキュメント完全度 | 高 | ソースコードコメントは完整、デザインドキュメントは詳細（18 のルール + 4 の原則） |
| コード品質 | 良好 | モジュール分割は明確、handler/rules/tests の階層化は合理的 |

---

## 改善待ち項目（優先度順）

全仕様は実装済み、改善待ち項目はありません。
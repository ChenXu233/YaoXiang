---
title: "解析器状態"
---

# パーサー（Parser）

> **モジュール状態**：安定（5項目待改进）
> **位置**：`src/frontend/core/parser/`
> **最終更新**：2026-06-01

---

## モジュール概述

パーサーはトークン流れをAST（抽象構文木）に変換する責務を持つ。古典的なPratt Parsing（トップダウン演算子優先順位）アルゴリズムを採用しており、完全なYaoXiang言語文法仕様をサポートしている。

**コード量**：約5000行（31のソースファイル、うち14個はテストファイル）

---

## 機能一覧

### 式解析（Pratt Parser）

**前置詞（nud）**：
- ✅ 全てのリテラル：Int、Float、String、Char、Bool、FString
- ✅ 識別子/変数参照
- ✅ 単項演算子：`-`、`+`、`not`、`*`（逆参照）
- ✅ 借用式：`&expr`、`&mut expr`
- ✅ グループ化/タプル：`(expr)`、`(a, b, c)`
- ✅ リストリテラルとリスト内包表記：`[1,2,3]`、`[x*x for x in items]`
- ✅ ブロック式：`{ stmts; expr }`
- ✅ 制御フロー：`if/elif/else`、`match`、`while`、`for`
- ✅ `ref` キーワード（Arc生成）
- ✅ `unsafe` ブロック
- ✅ `@block/@auto/@eager` 評価戦略アノテーション
- ✅ `spawn` 並行ブロック
- ✅ `return`、`break`、`continue`（ラベル付き対応）

**中置詞（led）**：
- ✅ 全二元演算子：`+`、`-`、`*`、`/`、`%`、`==`、`!=`、`<`、`<=`、`>`、`>=`、`and`、`or`、`..`
- ✅ 代入：`=`
- ✅ 関数呼び出し：`f(a, b)`、名前付き引数対応 `f(x=1, y=2)`
- ✅ フィールドアクセス：`obj.field`（チェーン対応：`a.b.c`）
- ✅ インデックスアクセス：`arr[0]`（チェーン対応：`m[i][j]`）
- ✅ 型変換：`expr as Type`
- ✅ エラー伝播：`expr?`
- ✅ ラムダ：`x => expr`、`(a, b) => expr`、`(x: Int) => x + 1`

**優先順位階層（10レベル）**：Lowest(0) < Assign/Range(1) < Or(2) < And(3) < Eq(4) < Cmp(5) < Add(6) < Mul(7) < Unary/Cast(8) < Call(9) < Highest(10)

### 文解析

- ✅ 変数宣言：`x = 42`、`x: Int = 42`、`mut x: Int = 0`、`pub x: Int = 42`
- ✅ 関数定義（RFC-010）：`add: (a: Int, b: Int) -> Int = a + b`
- ✅ 型定義（RFC-010）：`Name: Type = { ... }`
- ✅ メソッド定義（RFC-010）：`Point.draw: (self: Point, s: Surface) -> Void = ...`
- ✅ 外部バインディング（RFC-004）：`Point.distance = distance[0]`
- ✅ 制御フロー：`if/elif/else`、`while`、`for [mut] item in iter`、`return`、`break [label]`、`continue [label]`
- ✅ インポート：`use path`、`use path.{a, b}`、`use path as alias`
- ✅ 評価戦略アノテーション（RFC-001/008）：`@block`、`@auto`、`@eager`
- ✅ `pub` 公開修飾子

### 型システム解析

- ✅ 名前付き型：`Int`、`String`、`Bool`、`Float`
- ✅ メタ型（MetaType）：`Type`（RFC-010 コア）
- ✅ 関数型：`(Int, Float) -> Bool`
- ✅ タプル型：`(Int, String, Bool)`
- ✅ 構造体型：`{ x: Float, y: Float }`
- ✅ 列挙型/値バリアント型：`{ red | green | blue }`、`{ ok(Int) | err(String) }`
- ✅ ジェネリック型：`List(Int)`、`Map(String, Int)`
- ✅ ベアポインタ：`*Int`
- ✅ 参照型：`&T`、`&mut T`
- ✅ 関連型：`T::Item`
- ✅ リテラル型（定数ジェネリクス）：`n: n`

### エラー回復

- ✅ `parse()`：最初のエラーで `Err` を返す
- ✅ `parse_with_recovery()`：常に `ParseResult` を返し、エラー位置に `StmtKind::Error` / `Expr::Error` プレースホルダーノードを挿入
- ✅ `synchronize()` メソッド：次の文境界までジャンプして回復

---

## テストカバレッジ

**285個のテスト全て合格**、14個のテストファイルに分布在：

| テストファイル | テスト数 | カバー範囲 |
|----------|--------|----------|
| `tests/ast.rs` | ~55 | 全ASTノードバリアントの構築と照合 |
| `tests/expressions.rs` | ~28 | リテラル、単項/二元演算子、関数呼び出し、ラムダ、制御フローなど |
| `tests/integration.rs` | 5 | 完全プログラム解析（混合文） |
| `tests/parser_state.rs` | 15 | 状態機械操作（bump、skip、save/restore、エラー追跡） |
| `tests/error_recovery.rs` | 6 | エラー回復（空入力、単一/複数エラー、回復後の継続解析） |
| `pratt/tests/nud.rs` | ~30 | 前置詞解析ルーティングと機能 |
| `pratt/tests/led.rs` | ~30 | 中置詞解析ルーティングと機能 |
| `pratt/tests/precedence.rs` | 1 | 優先順位順序検証 |
| `statements/tests/declarations.rs` | ~16 | 変数、関数、型定義、メソッド定義 |
| `statements/tests/control_flow.rs` | ~10 | if/while/for/return/break/continue |
| `statements/tests/functions.rs` | 5 | 関数定義の各形式 |
| `statements/tests/imports.rs` | 4 | use文の各形式 |
| `statements/tests/types.rs` | ~20 | 型注釈解析 |
| `statements/tests/bindings.rs` | ~18 | バインディング構文（RFC-004/010） |

---

## RFC 比較

| RFC | 実装状態 | 説明 |
|-----|----------|------|
| RFC-001 並行モデル | ✅ 実装済み | `EvalMode` (Block/Auto/Eager) アノテーション |
| RFC-004 Curry多位置バインディング | ✅ 実装済み | `Type.method = func[0,1]` 外部バインディング構文 |
| RFC-007 関数構文統一 | ✅ 実装済み | ラムダ `(a, b) => body`、HM推論 |
| RFC-008 実行時並行モデル | ✅ 実装済み | `spawn { ... }` ブロック |
| RFC-010 統一型構文 | ✅ 実装済み | `name: type = value` 統一モデル、`Type` メタ型 |
| RFC-011 ジェネリック型システム | ✅ 実装済み | `(T: Type, N: Int) -> Type` ジェネリック構文 |
| RFC-012 F-string テンプレート文字列 | ✅ 実装済み | `f"Hello {name}"` を FString ノードとして解析 |
| RFC-017 LSP サポート | ✅ 実装済み | `parse_with_recovery()` + Error プレースホルダーノード |

---

## コード品質評価

| 次元 | 評価 | 説明 |
|------|------|------|
| 未完了事項 | 5 | テスト補完、プレースホルダーバインディング、Platform解析 |
| テストカバレッジ | 優秀 | 285個のテスト全て合格 |
| ドキュメント品質 | 良好 | ファイルレベルと関数レベルのコメント十分、RFC関連明確 |
| コードアーキテクチャ | 優秀 | Pratt Parser標準実装、モジュール化清晰 |
| RFC コンプライアンス | 高度コンプライアンス | RFC-001/004/007/008/010/011/012/017 全て実装済み |

---

## 待改进事項

1. **Dictリテラル解析テストの補完**
2. **FString解析エンドツーエンドテストの補完**
3. **`@block/@auto/@eager` と `spawn` の解析エンドツーエンドテストの補完**
4. **プレースホルダー `_` 位置バインディングの実装**（RFC-004）
5. **Platform パラメータ解析の実装**（RFC-011）
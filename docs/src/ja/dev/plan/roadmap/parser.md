---
title: "パーサステータス"
---

# パーサ（Parser）

> **モジュールステータス**：安定（5件の改善待ち項目あり）
> **位置**：`src/frontend/core/parser/`
> **最終更新**：2026-06-01

---

## モジュール概要

パーサは Token ストリームを AST（抽象構文木）に変換する役割を担う。古典的な Pratt Parsing（トップダウン演算子優先順位）アルゴリズムを採用しており、YaoXiang 言語の完全な文法仕様をサポートしている。

**コード量**：約 5000 行（31 個のソースファイル、うち 14 個はテストファイル）

---

## 機能一覧

### 式の解析（Pratt Parser）

**前置（nud）**：
- ✅ すべてのリテラル：Int、Float、String、Char、Bool、FString
- ✅ 識別子/変数参照
- ✅ 単項演算子：`-`、`+`、`not`、`*`（dereference）
- ✅ 借用式：`&expr`、`&mut expr`
- ✅ グループ化/タプル：`(expr)`、`(a, b, c)`
- ✅ リストリテラルとリスト内包表記：`[1,2,3]`、`[x*x for x in items]`
- ✅ ブロック式：`{ stmts; expr }`
- ✅ 制御フロー：`if/elif/else`、`match`、`while`、`for`
- ✅ `ref` キーワード（Arc を作成）
- ✅ `unsafe` ブロック
- ✅ `spawn` 並行ブロック（RFC-024）
- ✅ `return`、`break`、`continue`（オプションのラベル付き）

**中置（led）**：
- ✅ すべての二項演算子：`+`、`-`、`*`、`/`、`%`、`==`、`!=`、`<`、`<=`、`>`、`>=`、`and`、`or`、`..`
- ✅ 代入：`-`
- ✅ 関数呼び出し：`f(a, b)`、名前付き引数 `f(x=1, y=2)` を含む
- ✅ フィールドアクセス：`obj.field`（連鎖：`a.b.c`）
- ✅ インデックスアクセス：`arr[0]`（連鎖：`m[i][j]`）
- ✅ 型変換：`expr as Type`
- ✅ エラー伝播：`expr?`
- ✅ ラムダ：`x => expr`、`(a, b) => expr`、`(x: Int) => x + 1`

**優先順位階層（10 段）**：Lowest(0) < Assign/Range(1) < Or(2) < And(3) < Eq(4) < Cmp(5) < Add(6) < Mul(7) < Unary/Cast(8) < Call(9) < Highest(10)

### 文の解析

- ✅ 変数宣言：`x = 42`、`x: Int = 42`、`mut x: Int = 0`、`pub x: Int = 42`
- ✅ 関数定義（RFC-010）：`add: (a: Int, b: Int) -> Int = a + b`
- ✅ 型定義（RFC-010）：`Name: Type = { ... }`
- ✅ メソッド定義（RFC-010）：`Point.draw: (self: Point, s: Surface) -> Void = ...`
- ✅ 外部バインディング（RFC-004）：`Point.distance = distance[0]`
- ✅ 制御フロー：`if/elif/else`、`while`、`for [mut] item in iter`、`return`、`break [label]`、`continue [label]`
- ✅ インポート：`use path`、`use path.{a, b}`、`use path as alias`
- ✅ `pub` 可視性修飾子

### 型システムの解析

- ✅ 名前付き型：`Int`、`String`、`Bool`、`Float`
- ✅ メタ型（MetaType）：`Type`（RFC-010 の中核）
- ✅ 関数型：`(Int, Float) -> Bool`
- ✅ タプル型：`(Int, String, Bool)`
- ✅ 構造体型：`{ x: Float, y: Float }`
- ✅ enum/変種型：`{ red | green | blue }`、`{ ok(Int) | err(String) }`
- ✅ ジェネリック型：`List(Int)`、`Map(String, Int)`
- ✅ 素ポインタ：`*Int`
- ✅ 参照型：`&T`、`&mut T`
- ✅ 関連型：`T::Item`
- ✅ リテラル型（const generics）：`n: n`

### エラー回復

- ✅ `parse()`：最初のエラーで `Err` を返す
- ✅ `parse_with_recovery()`：常に `ParseResult` を返し、エラー位置に `StmtKind::Error` / `Expr::Error` プレースホルダノードを挿入
- ✅ `synchronize()` メソッド：次の文の境界にジャンプして回復

---

## テストカバレッジ

**285 件のテストすべてが合格**、14 個のテストファイルに分散：

| テストファイル | テスト数 | カバレッジ範囲 |
|----------|--------|----------|
| `tests/ast.rs` | ~55 | すべての AST ノード変種の構築とマッチング |
| `tests/expressions.rs` | ~28 | リテラル、単項/二項演算子、関数呼び出し、ラムダ、制御フローなど |
| `tests/integration.rs` | 5 | 完全なプログラム解析（混合文） |
| `tests/parser_state.rs` | 15 | 状態機械操作（bump、skip、save/restore、error tracking） |
| `tests/error_recovery.rs` | 6 | エラー回復（空入力、単一/複数のエラー、回復後の解析続行） |
| `pratt/tests/nud.rs` | ~30 | 前置パーサのルーティングと機能 |
| `pratt/tests/led.rs` | ~30 | 中置パーサのルーティングと機能 |
| `pratt/tests/precedence.rs` | 1 | 優先順位の順序検証 |
| `statements/tests/declarations.rs` | ~16 | 変数、関数、型定義、メソッド定義 |
| `statements/tests/control_flow.rs` | ~10 | if/while/for/return/break/continue |
| `statements/tests/functions.rs` | 5 | 関数定義の各形式 |
| `statements/tests/imports.rs` | 4 | use 文の各形式 |
| `statements/tests/types.rs` | ~20 | 型注釈の解析 |
| `statements/tests/bindings.rs` | ~18 | バインディング構文（RFC-004/010） |

---

## RFC 比較

| RFC | 実装状態 | 説明 |
|-----|----------|------|
| RFC-001 並行モデル | ✅ 実装済み | `EvalMode` (Block/Auto/Eager) アノテーション |
| RFC-004 Curry 複数位置バインディング | ✅ 実装済み | `Type.method = func[0,1]` 外部バインディング構文 |
| RFC-007 関数構文の統一 | ✅ 実装済み | ラムダ `(a, b) => body`、HM 推論 |
| RFC-008 ランタイム並行モデル | ✅ 実装済み | `spawn { ... }` ブロック |
| RFC-010 統一型構文 | ✅ 実装済み | `name: type = value` 統一モデル、`Type` メタ型 |
| RFC-011 ジェネリック型システム | ✅ 実装済み | `(T: Type, N: Int) -> Type` ジェネリック構文 |
| RFC-012 F-string テンプレート文字列 | ✅ 実装済み | `f"Hello {name}"` を FString ノードとして解析 |
| RFC-017 LSP サポート | ✅ 実装済み | `parse_with_recovery()` + Error プレースホルダノード |

---

## コード品質評価

| ディメンション | スコア | 説明 |
|------|------|------|
| 未完了事項 | 5 | テストの追加、プレースホルダバインディング、Platform 解析 |
| テストカバレッジ | 優秀 | 285 件のテストすべてが合格 |
| ドキュメント品質 | 良好 | ファイルレベルと関数レベルのコメントが十分、RFC との関連が明確 |
| コードアーキテクチャ | 優秀 | Pratt Parser の標準実装、モジュール化が明快 |
| RFC 準拠 | 高い準拠度 | RFC-001/004/007/008/010/011/012/017 すべて実装済み |

---

## 改善待ち項目

1. **Dict リテラル解析テストの追加**
2. **FString 解析のエンドツーエンドテストの追加**
3. **`spawn` の解析エンドツーエンドテストの追加**
4. **プレースホルダ `_` 位置バインディングの実装**（RFC-004）
5. **Platform 引数解析の実装**（RFC-011）
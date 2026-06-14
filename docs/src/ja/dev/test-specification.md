---
title: "テスト作成規範"
description: YaoXiang プロジェクトのテスト作成に関する厳格な規範。ユニットテスト、統合テスト、ベンチマークテスト、ドキュメントテスト、プロパティテストの作成基準を定義する
---

# テスト作成規範

本文書は YaoXiang プロジェクトのテスト作成に関する厳格な規範を定義する。すべての貢献者は以下のルールを遵守しなければならず、違反したものはコードレビューで修正を求められる。

---

## 目次

- [総則](#総則)
- [ユニットテスト規範](#ユニットテスト規範)
- [統合テスト規範](#統合テスト規範)
- [ベンチマークテスト規範](#ベンチマークテスト規範)
- [ドキュメントテスト規範](#ドキュメントテスト規範)
- [プロパティテスト規範](#プロパティテスト規範)
- [カバレッジ要件](#カバレッジ要件)
- [付録](#付録)

---

## 総則

### 適用範囲

本規範は YaoXiang プロジェクト内のすべての Rust テストコードに適用される。

| テスト種別 | 配置場所 | フレームワーク |
|------------|----------|----------------|
| ユニットテスト | `src/<module>/tests/` | `#[test]` + `#[cfg(test)]` |
| 統合テスト | `tests/` | `#[test]` |
| ベンチマークテスト | `benches/` | Criterion.rs |
| ドキュメントテスト | API ドキュメントコメント | `cargo test --doc` |
| プロパティテスト | 任意のテスト配置場所 | proptest / quickcheck |

### 核心原則

**原則 0：テストの権威ある情報源は規範であり、コードではない。** これは本文書で最も重要な原則である。テストが検証するのは「コードが規範に準拠しているか」であり、「コードが現在の手法で動作するか」ではない。テストがコードの振る舞いと規範の不一致を検出した場合、**テストを修正するのではなく、コードを修正する**。

規範ファイルは以下の場所にある：
- `docs/src/design/language-spec.md` —— 言語コア規範
- `docs/src/design/rfc/accepted/` —— 承認済み RFC 設計文書

各テストファイルの冒頭には対応する規範の章を宣言しなければならない（ルール 2.1 を参照）。いかなる開発者も規範文書とテストを照らし合わせて、実装の正当性を検証できるべきである。逆の見方をすれば——あるコードに対応す规范記述がない場合、そのコードは存在すべきではなく、ましてテストされるべきでもない。

```rust
// 🟢 良い例——テストが規範を直接参照し、コードが規範に従っているかを検証する
//! リテラルテスト — 言語規範 §2.6 に基づく
//!
//! §2.6.1: 整数 Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: 浮動小数点数（小数点と指数付き）
//! §2.6.3: 文字列（エスケープシーケンス \\nrt'"\\, \\x, \\u{}）
//! RFC-012: F-String 補間

#[test]
fn test_decimal_literal_parsing() {
    // 規範 §2.6.1: Decimal ::= [0-9][0-9_]*
    let result = parse_literal("42").unwrap();
    assert_eq!(result, Literal::Int(42));
}

// 🔴 悪い例——テストが現在のコードの実装動作に迎合しており、規範を検証していない
#[test]
fn test_literal_1() {
    // このコードが規範のどの節に対応するか不明
    // parse_literal が誤った値を返しても、このテストは「緑で通過」してしまう
    // 関数が panic しないことしか検証していないため
    let result = parse_literal("42");
    assert!(result.is_ok());
}
```

**シナリオ**：テストを書いたところ、コードの振る舞いが規範と一致していないことが判明した。2 つの選択肢がある：
| 誤った対応 | 正しい対応 |
|------------|------------|
| テストを修正して「通過」させる | コードを修正して、振る舞いを規範に合わせる |
| テストに `#[ignore]` を追加する | 直ちにコード実装を修正する |
| コードに迎合する特殊条件分岐をテストに追加する | 分岐を削除し、テストに直接問題を露呈させる |

覚えておいてほしい：**赤信号 = コードが間違っている、テストが間違っているのではない。**（テスト自体にバグがある場合は別の話だが）。

**原則 1：テストは文書である。** いかなる開発者も、テストを読むことで被テストコードの振る舞いを理解でき、余分なコメントや外部文書は必要としない。

```rust
// 🟢 良い例——テスト名が何をテストし、何を期待しているかを明示している
#[test]
fn test_tokenize_empty_input_returns_eof() {
    let tokens = tokenize("").unwrap();
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].kind, TokenKind::Eof));
}

// 🔴 悪い例——何をテストしているのか誰もわからない
#[test]
fn test_tokenize_1() {
    let tokens = tokenize("").unwrap();
    assert!(tokens.len() > 0);
}
```

**原則 2：ランダムな失敗はゼロトレランス。** テストはあらゆる環境で再現可能に実行されなければならない。乱数、システム時刻、スレッドスケジューリング順序に依存するテストは、シード固定化またはモックで代替しなければならない。

**原則 3：1 つのテストは 1 つのことだけをテストする。** テスト名に「と」で接続される複数の振る舞いが必要な場合は、複数のテストに分割する。

```rust
// 🟢 良い例——各テストが 1 つのシナリオのみを検証する
#[test]
fn test_parse_int_positive() { /* ... */ }
#[test]
fn test_parse_int_zero() { /* ... */ }

// 🔴 悪い例——1 つのテストに無関係な内容が多すぎる
#[test]
fn test_parser() {
    // tokenize, parse, typecheck, codegen をすべてテスト...
}
```

**原則 4：振る舞いをテストし、実装をテストしない。** 内部実装をリファクタリングしてもテストが失敗してはならない。実装コードを 1 行変更して 10 個のテストが失敗するなら、テストの書き方が間違っている。

ただし重要な区別がある：**「振る舞い」の定義は規範から来ており、現在のコードの動作からではない。** コードが振る舞いを変更した場合（すなわち規範に合致しない新しい振る舞いの場合）、テストは必ず失敗しなければならない。これが達成できないなら、テストは「コードに迎合するテスト」であり、バグの侵入を許してしまう。

```
規範（language-spec.md / RFC）  ──定義──►  期待される振る舞い  ──駆動──►  テスト
                                           │
現在のコード  ──実装──►  実際の振る舞い  ──比較──►  テスト結果

実際の振る舞い ≠ 期待される振る舞いの場合：
  テストは必ず失敗（赤信号）  ──►  コードを修正  ──►  テスト通過（緑信号）
  
実際の振る舞い = 期待される振る舞い（ただし実装が酷い）：
  テスト通過  ──►  実装をリファクタリング  ──►  テストは依然通過  ← これが原則 4 の意味
```

**原則 5：後退／互換／特定パターン有効化のためのテストコードは書かない。** テスト環境は完全に制御可能な環境である。`#[cfg(not(ci))]` を使ってあるテストをスキップする必要があるなら、そのテスト設計には根本的な問題がある。

### 用語定義

| 用語 | 定義 |
|------|------|
| ユニットテスト | 単一の関数またはモジュールの振る舞いをテストし、外部システムに依存しない |
| 統合テスト | 複数のモジュールの連携を、公開 API またはコマンドラインエントリを通じてテストする |
| ベンチマークテスト | コードパフォーマンスを測定し、パフォーマンス回帰を検出する |
| ドキュメントテスト | ドキュメントコメントに埋め込まれた実行可能なコード例 |
| プロパティテスト | ランダム入力に基づいて不変量（property）を検証するテスト |

### コミット規範との関連

すべてのテスト関連のコミットは `:white_check_mark: test:` タイプを使用しなければならず、[コミット規範](./commit-convention.md)を参照する。

```
:white_check_mark: test(parser): Pratt パーサの中置演算子テストを追加
:white_check_mark: test(codegen): switch 文の IR 生成テストを補完
```

---

## ユニットテスト規範

### ファイル構成

**ルール 1.1**：ユニットテストの `tests/` ディレクトリは被テストモジュールの `mod.rs` と**同レベルでなければならない**。`tests/` は上位集約せず、複数階層を跨いでまとめない。

```
src/frontend/core/parser/
├── mod.rs              # #[cfg(test)] mod tests; ——同レベルの tests/ を宣言
├── ast.rs
├── pratt/
│   ├── mod.rs          # #[cfg(test)] mod tests; ——pratt 自身のテスト
│   └── tests/
│       ├── mod.rs
│       ├── led.rs
│       ├── nud.rs
│       └── precedence.rs
└── tests/              # parser モジュールレベルのテスト（pratt サブモジュールの内容は含まない）
    ├── mod.rs
    ├── ast.rs
    ├── expressions.rs
    ├── error_recovery.rs
    └── parser_state.rs
```

重要な判断基準：**`tests/` をどのディレクトリに置くか、そのディレクトリの `mod.rs` は `#[cfg(test)] mod tests;` でそれを宣言しなければならない。**

**ルール 1.1 補足：上位集約は禁止。** サブディレクトリモジュールのテストは、そのサブディレクトリ自身の `tests/` に置かなければならず、親階層の `tests/` に集約してはならない。

| モジュール種別 | テスト配置場所 | 例 |
|----------------|----------------|-----|
| ディレクトリモジュール（`mod.rs` あり） | そのディレクトリ下の `tests/` | `emitter/tests/`、`codes/tests/` |
| 単一ファイルモジュール（`.rs` のみ） | 親階層の `tests/` | `session.rs` → `diagnostic/tests/session.rs` |

```text
# ✅ 正しい：各ディレクトリモジュールのテストがそれぞれ独立
src/util/diagnostic/
├── codes/
│   ├── mod.rs              # #[cfg(test)] mod tests;
│   └── tests/              # ✅ codes 自身のテスト
│       ├── mod.rs
│       └── codes.rs
├── emitter/
│   ├── mod.rs              # #[cfg(test)] mod tests;
│   └── tests/              # ✅ emitter 自身のテスト
│       ├── mod.rs
│       ├── text.rs
│       └── ansi.rs
└── tests/                  # ✅ diagnostic レベル（単一ファイルモジュール）
    ├── mod.rs
    ├── session.rs
    ├── suggest.rs
    └── collect.rs

# ❌ 誤り：emitter と codes のテストを diagnostic/tests/ に集約
src/util/diagnostic/
└── tests/
    ├── mod.rs              # ❌ 強制的に mod emitter; mod codes; を宣言することになる
    ├── emitter/            # ❌ emitter/tests/ に置くべき
    └── codes/              # ❌ codes/tests/ に置くべき
```

#### 単一ファイルモジュール vs ディレクトリモジュールのテスト配置ルール

**核心的な違い**：モジュールの組織形式がテストの配置場所を決定する。

| モジュール種別 | 判定基準 | テスト配置場所 | 例 |
|----------------|----------|----------------|-----|
| **ディレクトリモジュール** | 独立したディレクトリと `mod.rs` を持つ | そのディレクトリ下の `tests/` | `inference/tests/` |
| **単一ファイルモジュール** | `.rs` ファイルのみで、独立したディレクトリなし | 親モジュールの `tests/` | `overload.rs` → `typecheck/tests/overload.rs` |

**詳細説明**：

```
src/frontend/core/typecheck/
├── mod.rs                          # typecheck モジュールの mod.rs
├── checker.rs                      # 単一ファイルモジュール
├── environment.rs                  # 単一ファイルモジュール
├── overload.rs                     # 単一ファイルモジュール
├── type_eval.rs                    # 単一ファイルモジュール
├── dead_code.rs                    # 単一ファイルモジュール
├── spawn_placement.rs              # 単一ファイルモジュール
├── signature.rs                    # 単一ファイルモジュール
├── types.rs                        # 単一ファイルモジュール
│
├── tests/                          # ✅ typecheck のテストディレクトリ
│   ├── mod.rs                      # 単一ファイルモジュールのテストを宣言
│   ├── checker.rs                  # checker.rs のテスト
│   ├── environment.rs              # environment.rs のテスト
│   ├── overload.rs                 # overload.rs のテスト（単一ファイルモジュールのテストはここに配置）
│   ├── type_eval.rs                # type_eval.rs のテスト
│   ├── dead_code.rs                # dead_code.rs のテスト
│   ├── spawn_placement.rs          # spawn_placement.rs のテスト
│   ├── signature.rs                # signature.rs のテスト
│   └── types.rs                    # types.rs のテスト
│
├── inference/                      # ディレクトリモジュール（mod.rs あり）
│   ├── mod.rs                      # #[cfg(test)] mod tests; ——同レベルの tests/ を宣言
│   ├── expressions.rs
│   ├── statements.rs
│   ├── patterns.rs
│   ├── bounds.rs
│   ├── subtyping.rs
│   ├── generics.rs
│   ├── compatibility.rs
│   ├── scope.rs
│   ├── assignment.rs
│   └── tests/                      # ✅ inference のテストディレクトリ
│       ├── mod.rs
│       ├── expressions.rs          # expressions.rs のテスト
│       ├── statements.rs           # statements.rs のテスト
│       └── ...
│
└── traits/                         # 削除済み（ロジックは types/trait_data.rs に統合）
```

**なぜ単一ファイルモジュールのテストを親階層の `tests/` に置くのか？**

単一ファイルモジュール（例：`overload.rs`）には独自の `mod.rs` がないため、`#[cfg(test)] mod tests;` を宣言できない。Rust のモジュールシステムによれば、テストファイルは何らかの `mod.rs` で宣言されなければコンパイルされない。したがって、単一ファイルモジュールのテストは親階層の `mod.rs` によって宣言され、親階層の `tests/` ディレクトリに置かれるしかない。

**判断フロー**：

```
あるモジュールに遭遇した。テストをどこに置くか？
│
├── そのモジュールはディレクトリ（mod.rs あり）？
│   └── はい → そのディレクトリ下に tests/ を作成し、そのディレクトリの mod.rs が宣言する
│
├── そのモジュールは単一ファイル（.rs のみ）？
│   └── はい → 親階層の tests/ ディレクトリにテストを置き、親階層の mod.rs が宣言する
│
└── 不明？
    └── 独立したディレクトリと mod.rs があるかを確認する
```

**よくある誤り**：

```
# ❌ 誤り 1：単一ファイルモジュール用に独立した tests/ ディレクトリを作成する
src/frontend/core/typecheck/
├── overload.rs
└── overload/                       # ❌ 単一ファイルモジュール用にディレクトリを作成すべきでない
    └── tests/
        └── overload.rs

# ❌ 誤り 2：単一ファイルモジュール内で #[cfg(test)] mod tests; を宣言する
# overload.rs
#[cfg(test)]                        # ❌ 単一ファイルモジュールはこう宣言できない
mod tests;                          # overload/tests/ ディレクトリがないため

# ✅ 正しい方法：テストを親階層の tests/ に配置
src/frontend/core/typecheck/
├── overload.rs                     # ソースファイル
└── tests/
    └── overload.rs                 # テストファイル、typecheck/mod.rs が宣言
```

⚠️ **アンチパターン——こう書いてはいけない：**

```
# ❌ 誤り：サブモジュールのテストを親階層に集中させる
src/frontend/core/types/
├── mod.rs              # base と computation のみを宣言すべき
├── base/
│   ├── mod.rs
│   └── var.rs
└── tests/              # ❌ 親階層の tests/ にサブモジュールのテストが含まれる
    ├── mod.rs          # ❌ 強制的に mod base; mod computation; を宣言することになる
    ├── base/           # ❌ この部分は base/tests/ に置くべき
    │   └── var.rs
    └── computation/    # ❌ この部分は computation/tests/ に置くべき
        └── ...
```

```
# ✅ 正しい方法：各モジュールのテストがそれぞれ独立
src/frontend/core/types/
├── mod.rs              # pub mod base; pub mod computation; のみを宣言
├── base/
│   ├── mod.rs          # #[cfg(test)] mod tests; ——同レベルの tests/ を宣言
│   ├── var.rs
│   └── tests/
│       ├── mod.rs
│       └── var.rs
└── computation/
    ├── mod.rs          # #[cfg(test)] mod tests; ——同レベルの tests/ を宣言
    ├── operations.rs
    └── tests/
        ├── mod.rs
        └── operations.rs
```

**なぜ上位集約できないのか？** Rust のモジュールシステムは、宣言位置で `#[cfg(test)] mod tests;` がテストファイルのコンパイルを決定するためである。`types/mod.rs` が `mod tests;` を宣言している場合、`types/tests/` の内容は `types` モジュールのプライベートな内容であり、`base` や `computation` の領域に踏み込むべきではない。各モジュールのテストはそのモジュールの内部実装詳細であるべきで、親モジュールのものであるべきではない。このルールはモジュールのリファクタリングにも同様に適用される：`types` を `base` と `computation` に分割する際、テストも分割後のモジュールに従って分割されるべきであり、元の場所に残すべきではない。**テストディレクトリはソース構造をミラーリングするのではなく、モジュール境界に従う。**

**ルール 1.2**：`tests/mod.rs` はモジュールの宣言と re-export のみを担当し、テスト関数を置かない。

```rust
//! Parser core tests — mirrors src/frontend/core/parser/
//!
//! Tests for ast.rs, parser_state.rs, and expression/integration parsing.

mod ast;
mod error_recovery;
mod expressions;
mod integration;
mod parser_state;
```

**ルール 1.3**：各テストファイルはソースファイル 1 つだけに対応する。複数のソースモジュールのテストを 1 つのファイルに混在させることは許可されない。

**ルール 1.4**：テスト宣言はファイル形式 `mod tests;`（セミコロン付き）を使用しなければならず、同レベルの `tests/` ディレクトリを指す。**インライン形式 `mod tests { ... }` を使ってテストコードをソースファイル内に直接書くことは禁止。**

```rust
// ✅ 正しい——ファイル形式での宣言、テストコードは独立したファイルに
// src/frontend/core/parser/mod.rs
#[cfg(test)]
mod tests;

// 🔴 禁止——インライン形式、テストコードがソースファイル内に寄生
// src/frontend/core/parser/mod.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        // テストコードはソースファイル内に現れてはならない
    }
}
```

**なぜインラインを禁止するのか？**
1. ソースファイルの責務を単一にする：ソースファイルには実装のみ、テストファイルにはテストのみ。混在すると、テスト変更時にファイル末尾までスクロールし、実装変更時にテストをスキップする必要がある。
2. モジュール境界の明確化：`tests/` ディレクトリは物理的な境界であり、どのモジュールにテストがあり、ないかが一目でわかる。
3. リファクタリングの安全性：モジュール分割時、`tests/` ディレクトリも一緒に移動する。インラインテストはソースファイルから手動で切り離す必要がある。
4. コードレビュー：PR diff 内でソース変更とテスト変更が別ファイルになり、混ざらない。

### モジュール宣言規範

**ルール 2.1**：すべてのテストファイルの先頭にはモジュールレベルのドキュメントコメント `//!` を置き、テストがカバーする規範の情報源（言語規範の章番号 + RFC 番号）を説明しなければならない。あるテストがどの規範の章も参照していない場合、そのコードには規範的根拠がないことを意味する——そのコードは存在すべきではない。

```rust
//! リテラルテスト — 言語規範 §2.6 に基づく
//!
//! §2.6.1: 整数 Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: 浮動小数点数（小数点と指数付き）
//! §2.6.3: 文字列（エスケープシーケンス \\nrt'"\\, \\x, \\u{}）
//! RFC-012: F-String 補間
```

**なぜ規範を参照しなければならないのか？** テストの期待値は規範から来るべきであり、「現在のコードの出力」から来るべきではない。もしある日コードが出力を変更し、テストがそれに合わせて更新されたら、テストは何も保護していない。規範に紐づけられたテストのみが、「意図的な breaking change」と「意図しない回帰」を区別できる。

**ルール 2.2**：テストモジュールの `use` インポートは具体的な型/関数まで正確に行わなければならず、glob インポート `use super::*` は禁止。

```rust
// 🟢 良い例——正確なインポート
use crate::frontend::core::lexer::{tokenize, TokenKind};
use crate::frontend::core::parser::{ParserState, ParseError};

// 🔴 悪い例——何をテストしているのか他人にわからない
use super::*;
```

### 命名規範

**ルール 3.1**：テスト関数の命名フォーマットは `test_<what>_<scenario>` で、すべて小文字、アンダースコア区切り。

```rust
#[test]
fn test_tokenize_empty_string() { /* ... */ }
#[test]
fn test_parse_int_overflow() { /* ... */ }
#[test]
fn test_typecheck_fn_return_mismatch() { /* ... */ }
```

**ルール 3.2**：テスト関数名は自己説明的である必要がある。関数名を読んだだけで、何をテストし、何を期待しているかがわかること。数字のシーケンス番号での命名は禁止。

```rust
// 🟢 良い例
fn test_skip_semicolon_success() { /* ... */ }
fn test_skip_semicolon_failure_when_identifier() { /* ... */ }

// 🔴 悪い例——何をテストしているのか完全にわからない
fn test_skip_1() { /* ... */ }
fn test_skip_2() { /* ... */ }
```

**ルール 3.3**：ヘルパー関数には `test_` 接頭辞は不要で、動詞または名詞でその用途を記述する。

```rust
fn parse_expr(source: &str) -> Expr { /* ... */ }
fn tokenize_single(source: &str) -> Token { /* ... */ }
fn setup_parser_with_tokens(tokens: &[Token]) -> ParserState { /* ... */ }
```

### テスト構造規範 (Arrange-Act-Assert)

**ルール 4.1**：各テスト関数は 3 段構成（準備（Arrange）→ 実行（Act）→ 検証（Assert））に従わなければならず、段の間は空行で区切る。

```rust
#[test]
fn test_parse_binary_addition() {
    // Arrange
    let source = "1 + 2";

    // Act
    let expr = parse_expr(source);

    // Assert
    assert!(matches!(expr, Expr::Binary { op: BinOp::Add, .. }));
}
```

**ルール 4.2**：シンプルなテスト（単一の呼び出し + 単一の検証）は段のコメントを省略できるが、5 行のロジックコードを超えないこと。5 行を超えるテストは明示的に 3 段をマークしなければならない。

### ヘルパー関数規範

**ルール 5.1**：3 回以上繰り返し現れる setup ロジックはヘルパー関数に抽出しなければならない。

```rust
// 🟢 良い例——共通の setup を抽出
fn with_state<F>(source: &str, mut f: F)
where
    F: FnMut(&mut ParserState<'_>),
{
    let tokens = tokenize(source).unwrap();
    let mut state = ParserState::new(&tokens);
    f(&mut state);
}

#[test]
fn test_current_returns_first_token() {
    with_state("42", |state| {
        let tok = state.current();
        assert_eq!(&tok.unwrap().kind, &TokenKind::IntLiteral(42));
    });
}
```

**ルール 5.2**：ヘルパー関数内の `unwrap()` / `expect()` は、panic 時に十分なコンテキストを出力しなければならない。テスト関数本体内（`#[test] fn ...`）では直接 `unwrap()` できる——失敗時に Rust が自動的に行番号を出力するため。一方、ヘルパー関数内で失敗した場合、行番号はヘルパー関数の定義位置を指し、呼び出し時点のコンテキストがわからない。

```rust
// 🟢 良い例——ヘルパー関数の失敗時にソース内容を表示
fn run_ok(source: &str) {
    run(source).unwrap_or_else(|e| panic!("Execution failed:\nSource:\n{}\nError:\n{:?}", source, e));
}

// 🔴 悪い例——失敗時にどのソースファイルが問題だったか見えない
fn run_ok(source: &str) {
    run(source).unwrap();
}
```

**ルール 5.3**：ヘルパー関数はテストファイルの先頭、`use` インポートの直後に配置する。複数のテストモジュールで共有される場合、`tests/mod.rs` に置いて `pub(crate)` でエクスポートする。

### アサーションスタイル

**ルール 6.1**：enum バリアントのマッチングには `assert!(matches!(...))` を優先的に使用し、`if let` + `panic!` を使用してはならない。

```rust
// 🟢 良い例
assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(42)));

// 🔴 悪い例
if let TokenKind::IntLiteral(v) = tokens[0].kind {
    assert_eq!(v, 42);
} else {
    panic!("Expected IntLiteral");
}
```

**ルール 6.2**：正確な値の比較には `assert_eq!` を使用し、ブールアサーションには `assert!` を使用する。`assert!(a == b)` を `assert_eq!(a, b)` の代わりに使用することは禁止。

**ルール 6.3**：すべてのアサーションにはカスタムエラーメッセージを付けること。ただし、アサーション自体が失敗理由を完全に記述している場合は除く。

```rust
// 🟢 良い例——アサーション失敗時に迅速に問題を特定できる
assert!(
    state.infix_info().is_some(),
    "infix_info should handle '{op}'"
);

// 🟢 良い例——assert_eq! の失敗時に値の差分が自動表示され、追加メッセージは不要
assert_eq!(error_count, 0);

// 🔴 悪い例——失敗時に "assertion failed" としか表示されない
assert!(state.infix_info().is_some());
```

**ルール 6.4**：アサーションの順序は `assert_eq!(actual, expected)` とし、実際の値が先、期待値が後。

### アンチパターン一覧

禁止されている書き方と代替案：

| アンチパターン | 問題 | 代替案 |
|---------------|------|--------|
| `#[cfg(test)] mod tests { ... }` インラインテスト | ソースファイル肥大化、モジュール境界の曖昧さ、リファクタリング困難 | テストコードを独立した `tests/` ディレクトリに置き、`mod tests;` で宣言（ルール 1.4 参照） |
| テストがコードの誤った振る舞いに迎合する | 規範偏差を隠蔽し、バグを合法化 | 規範に照らしてコードを修正し、テストは不変に保つ |
| コード出力からテスト期待値を逆算する | テストが「現在実装の録音機」になる | 規範から期待値を導出する |
| `#[ignore]` の永続的な付与 | 腐ったテストを隠蔽 | 修正または削除 |
| `println!` デバッグ出力 | テスト出力を汚染 | `assert!` で明確にアサート |
| `thread::sleep` | ランダム失敗 + 遅い | 同期機構またはモックを使用 |
| テストで実際のファイルシステムを操作 | 遅く再現不可能 | `tempfile` を使用 |
| テスト実行順序への依存 | ランダム失敗 | 各テストが独立した setup を持つ |
| 1 つのテスト関数が 30 行のロジックを超える | 誰も理解できない | テストを分割するかヘルパー関数を使用 |
| ヘルパー関数内の `unwrap()` がコンテキストを報告しない | 問題の特定が困難 | `expect("why")` またはカスタム panic を使用（ルール 5.2 参照） |
| 3 回以上同じ setup のコピー＆ペースト | 変更コストが高い | ヘルパー関数を抽出 |

---

## 統合テスト規範

### テスト構成

**ルール 7.1**：統合テストはプロジェクトルートの `tests/` ディレクトリに配置する。エントリファイル `tests/integration.rs` は `#[path]` 属性を使ってサブモジュールをインクルードする。

```rust
// tests/integration.rs
#[path = "integration/backends.rs"]
mod backends;
#[path = "integration/codegen.rs"]
mod codegen;
#[path = "integration/execution.rs"]
mod execution;
```

**ルール 7.2**：各 `tests/integration/*.rs` ファイルは 1 つのテストテーマ（コンパイラバックエンド、コード生成、エグゼキュータなど）に対応し、混在させてはならない。

**ルール 7.3**：統合テストはプロジェクトの公開 API を通じてテストしなければならない。統合テスト内で `crate::` 内部モジュールを直接参照することは禁止。`yaoxiang::` 公開パスを使用する。

```rust
// 🟢 良い例——公開 API 経由
use yaoxiang::run;

// 🔴 悪い例——公開 API 境界を迂回
use yaoxiang::middle::codegen::bytecode::BytecodeFile;
```

### テストデータ管理

**ルール 8.1**：統合テストはインラインソース文字列を優先的に使用する。ソースが 30 行を超える場合のみ、外部フィクスチャファイル（`tests/fixtures/` に配置）を使用する。

```rust
#[test]
fn test_fibonacci() {
    run_ok(
        r#"
        main = {
            mut a = 0
            mut b = 1
            while a < 100 {
                mut next = a + b
                a = b
                b = next
            }
        }
        "#,
    );
}
```

**ルール 8.2**：フィクスチャファイルは `.yx` 拡張子で終わり、ファイル名はテストの意図を記述する。

### E2E カバレッジ原則

**ルール 9.1**：各言語機能の統合テストは 3 つのパスをカバーしなければならない：

| パス | 説明 |
|------|------|
| Happy path | 合法的な入力が予期される出力を生成する |
| Error path | 不正な入力が明確なエラー情報を生成する（panic ではなく） |
| Boundary | 境界値（空入力、最大値、ネスト深度の上限） |

**ルール 9.2**：統合テストはネットワーク、システム環境変数、外部サービスに依存してはならない。

---

## ベンチマークテスト規範

### Criterion.rs 使用規範

**ルール 10.1**：ベンチマークテストは `benches/` ディレクトリに統一して配置し、エントリファイルは `benches/lib.rs` とする。テストテーマごとにファイル分割する。

```
benches/
├── lib.rs              # エントリ、criterion_group/criterion_main を定義
├── lang_compare/
│   └── fibonacci.rs    # 言語横断比較ベンチマーク
├── parser.rs           # パーサベンチマーク
└── codegen.rs          # コード生成ベンチマーク
```

**ルール 10.2**：各ベンチ関数にはモジュールドキュメントコメント `//!` を付け、テスト目的と測定指標を説明しなければならない。

```rust
//! YaoXiang インタプリタパフォーマンステスト
//!
//! 測定指標：単一イテレーション時間（wall time）
//! ベースライン：Rust ネイティブ実装
```

### コンパイラ最適化の防止

**ルール 11.1**：すべてのベンチマークテストの被テスト出力は `criterion::black_box` を経由してコンパイラの最適化による削除を防がなければならない。

```rust
use criterion::{black_box, Criterion};

fn bench_parse(c: &mut Criterion) {
    c.bench_function("parse_fib", |b| {
        b.iter(|| {
            let result = parse(black_box(FIB_SOURCE));
            black_box(result)
        })
    });
}
```

**ルール 11.2**：ベンチマークテストの入力データは `const` または `lazy_static` なければならず、`iter` クロージャ内で動的に生成してはならない——さもなければ測定されるのは被テストロジックだけでなくデータ生成も含む合計時間となる。

### ベンチマークグループ化と命名

**ルール 12.1**：ベンチマークテストの命名フォーマットは `<被テストモジュール>_<シナリオ>` で、すべて小文字、アンダースコア区切り。ユニットテストの命名規則と一致する。

**ルール 12.2**：関連するベンチマークは `criterion_group!` を使って論理的にグループ化しなければならない。すべてのベンチマークを 1 つのグループに詰め込むことは禁止。

```rust
criterion_group!(parser, bench_parse_expr, bench_parse_stmt);
criterion_group!(codegen, bench_codegen_module, bench_codegen_switch);
criterion_main!(parser, codegen);
```

---

## ドキュメントテスト規範

### 使用シナリオ

**ルール 13.1**：すべての `pub` 関数、型、メソッドはドキュメントコメント内に少なくとも 1 つの実行可能なコード例を含めなければならない。この例は `cargo test --doc` で実行される。

```rust
/// ソース文字列をトークン列に分割する。
///
/// ```
/// use yaoxiang::frontend::core::lexer::tokenize;
///
/// let tokens = tokenize("42").unwrap();
/// assert_eq!(tokens.len(), 2); // IntLiteral + Eof
/// ```
pub fn tokenize(source: &str) -> Result<Vec<Token>, LexError> {
    // ...
}
```

**ルール 13.2**：ドキュメントテストのコード例はコンパイルが通り、検証に成功しなければならない。コンパイル時エラーを示す例でない限り、`ignore` マーク付きの例を含んではならない。

```rust
/// ```ignore
/// // コンパイル時エラーを示す——ignore 可
/// let x: int = "string";
/// ```
```

### カバレッジ要件

**ルール 14.1**：ドキュメントテストは API の happy path をカバーすればよい。境界ケースやエラーパスはユニットテストでカバーする。

**ルール 14.2**：ドキュメントテスト内のサンプルコードは簡潔でなければならない——10 行以内。サンプルにより長いコンテキストが必要な場合は、API 設計に問題がある。

---

## プロパティテスト規範

### 使用シナリオ

**ルール 15.1**：以下のシナリオでは、手書きの複数の境界値テストケースではなく、プロパティテスト（proptest または quickcheck）を使用しなければならない：

| シナリオ | 例 |
|----------|-----|
| パーサのラウンドトリップ | `parse(pretty_print(ast)) == ast` |
| シリアライズ/デシリアライズ | `deserialize(serialize(data)) == data` |
| 数学演算の恒等式 | `a + b == b + a` |
| コンパイラ最適化が意味を変えない | `eval(code) == eval(optimize(code))` |

**ルール 15.2**：プロパティテストは主要なプロパティテストフレームワークとして `proptest` を使用する（`Cargo.toml` の `dev-dependencies` で宣言済み）。

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_roundtrip_serialize_deserialize(value: i64) {
        let serialized = serialize(&value);
        let deserialized: i64 = deserialize(&serialized).unwrap();
        prop_assert_eq!(deserialized, value);
    }
}
```

### プロパティ定義の原則

**ルール 16.1**：各プロパティテストには明確なプロパティ宣言が必要——コメントに検証する不変量を記述する。

```rust
// プロパティ：任意のリテラル整数を tokenize → tokens_to_string すると同じ値が生成される
proptest! {
    #[test]
    fn test_int_literal_roundtrip(n in any::<i64>()) {
        let source = n.to_string();
        let tokens = tokenize(&source).unwrap();
        // ...
    }
}
```

**ルール 16.2**：プロパティテストが失敗を発見した場合、`proptest` の回帰メカニズムを使用しなければならない——失敗した入力を `proptest-regressions/` ディレクトリに追加し、手書きの通常テストで代替してはならない。

---

## カバレッジ要件

### 新規コードのカバレッジ目標

**ルール 17.1**：新規コードのテストカバレッジ要件：

| コード種別 | 行カバレッジ | 分岐カバレッジ |
|------------|--------------|----------------|
| コアコンパイラモジュール（frontend/middle/backends） | ≥ 85% | ≥ 80% |
| ユーティリティ/ヘルパーモジュール（util） | ≥ 75% | ≥ 70% |
| ランタイムモジュール（vm/runtime） | ≥ 80% | ≥ 75% |
| 標準ライブラリ（std） | ≥ 75% | ≥ 70% |
| エラーハンドリングと診断 | ≥ 90% | ≥ 85% |

**ルール 17.2**：エラーハンドリングパス（すべての `Err` 分岐）は 100% カバーしなければならない。ユーザーが目にする可能性のあるエラーメッセージはテストで検証済みでなければならない。

### PR レビューチェックリスト

**ルール 18.1**：PR を提出する前に、著者は以下の項目を自己点検しなければならない：

- [ ] `cargo test` がすべて通過
- [ ] `cargo test --doc` がすべて通過
- [ ] `cargo bench` でパフォーマンス回帰がない（ホットパスの変更が関わる場合）
- [ ] 新規コードがカバレッジ目標に準拠
- [ ] テスト命名が命名規範に準拠
- [ ] 各テストファイルが対応する規範の章を宣言している（ルール 2.1）
- [ ] テスト期待値が規範定義から来ており、「現在のコードの出力」ではない
- [ ] `#[ignore]` マーク付きのテストがない（明確な issue 番号コメントがある場合を除く）
- [ ] 不要な `unwrap()` がない（`expect` またはカスタム panic メッセージを使用すべき）
- [ ] コミットメッセージが `:white_check_mark: test:` タイプを使用
- [ ] **「コードの振る舞いが規範と一致しない」ことを理由にテスト期待値を変更していない——変更するのはコードであり、テストではない**
- [ ] **インラインテストがない**（`#[cfg(test)] mod tests { ... }` は `mod tests;` + 独立ファイルに変更すること、ルール 1.4 参照）

**ルール 18.2**：Reviewer は以下の問題を含む PR を拒否しなければならない：

- happy path テストのみで、エラーパスがない
- テスト内に `thread::sleep` があるか、実行順序に依存している
- コピー＆ペーストされたテストコードが 3 回以上でヘルパー関数が抽出されていない
- テスト名が命名規範に準拠していない
- 永続的に `#[ignore]` されたテストが存在する
- **テストがコードの誤った振る舞いに迎合している**（コードと規範が一致しない時にコードではなくテストを変更する）
- **テストが対応する規範の章を宣言していない**（ルール 2.1 参照）
- **テスト期待値が規範定義ではなくコード出力から来ている**（逆算されたテストはテストしないのと同じ）
- **インラインテストが存在する**（`#[cfg(test)] mod tests { ... }` ではなく `mod tests;` + 独立ファイル、ルール 1.4 参照）
- テストが「panic しない」ことのみを検証し、具体的な振る舞いを検証していない
- コードのバグを露呈する失敗テストを削除した（コードを修正してから緑になるのを待つのではなく）

---

## 付録

### A. テストコマンド早見表

```bash
# すべてのテストを実行
cargo test

# ユニットテストのみ実行
cargo test --lib

# 統合テストのみ実行
cargo test --test integration

# ドキュメントテストのみ実行
cargo test --doc

# 特定のテストを実行（名前でフィルタ）
cargo test test_parse_expr

# ベンチマークテストを実行
cargo bench

# テスト出力を表示（デフォルトでは stdout が非表示）
cargo test -- --nocapture

# シングルスレッドで実行（並行問題切り分け用）
cargo test -- --test-threads=1

# カバレッジレポートを生成（cargo-llvm-cov が必要）
cargo llvm-cov --html
```

### B. コミットメッセージテンプレート

テスト関連のコミットは以下のテンプレートに従わなければならない：

```
:white_check_mark: test(<scope>): <簡潔な説明>

<オプション：カバーするシナリオのリスト>
```

例：

```
:white_check_mark: test(parser): Pratt パーサの中置演算子テストを追加

カバーするシナリオ：
- 算術演算子の優先順位（+, -, *, /, %）
- 比較演算子の連鎖（1 < x < 10）
- 論理演算子の短絡
- 代入演算子の右結合
```

### C. 新規テストファイルリスト

新しいテストモジュールを作成する際、以下のファイルが含まれることを確認する：

```
# src/<module>/ ディレクトリ下に新規テストを追加
src/<module>/tests/
├── mod.rs          # モジュール宣言 + 共通ヘルパー関数
└── <subject>.rs    # テストファイル、被テストソースファイルに命名を合わせる

# tests/ ディレクトリ下に新規統合テストを追加
tests/
├── integration.rs   # 更新：#[path] 宣言を追加
└── integration/
    └── <topic>.rs   # 新規テストファイル
```

### D. 参考資料

- [YaoXiang 言語規範](../../design/language-spec.md) —— **テストの権威ある情報源**
- [承認済み RFC](../../design/rfc/accepted/) —— **設計決定の権威ある情報源**
- [Rust テストドキュメント](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Criterion.rs ユーザーガイド](https://bheisler.github.io/criterion.rs/book/)
- [proptest ドキュメント](https://docs.rs/proptest/latest/proptest/)
- [プロジェクトコミット規範](./commit-convention.md)
- [プロジェクト貢献ガイド](./contributing.md)

---

> 💡 **覚えておいてほしい**：テストはコードが「動く」ことを検証するのではない——コードが規範に準拠していることを検証する。規範が変われば、テストは規範に従って変わる。コードが間違っているなら、テストではなくコードを修正。**コードは規範に従い、テストは規範を守る。テストがコードに迎合した瞬間、すべての保護を失う。**
```markdown
---
title: "テスト記述規範"
description: YaoXiang プロジェクトのテスト記述に関する厳格な規範であり、ユニットテスト、統合テスト、パフォーマンステスト、ドキュメントテスト、プロパティテストの記述基準を定義する
---

# テスト記述規範

本書では YaoXiang プロジェクトのテスト記述に関する厳格な規範を定義する。すべての貢献者は以下の規則を遵守しなければならず、違反者はコードレビューで修正を要求される。

---

## 目次

- [総則](#総則)
- [ユニットテスト規範](#ユニットテスト規範)
- [統合テスト規範](#統合テスト規範)
- [パフォーマンステスト規範](#パフォーマンステスト規範)
- [ドキュメントテスト規範](#ドキュメントテスト規範)
- [プロパティテスト規範](#プロパティテスト規範)
- [カバレッジ要件](#カバレッジ要件)
- [付録](#付録)

---

## 総則

### 適用範囲

本規範は YaoXiang プロジェクトにおけるすべての Rust テストコードに適用される：

| テスト種別 | 位置 | フレームワーク |
|------------|------|----------------|
| ユニットテスト | `src/<module>/tests/` | `#[test]` + `#[cfg(test)]` |
| 統合テスト | `tests/` | `#[test]` |
| パフォーマンステスト | `benches/` | Criterion.rs |
| ドキュメントテスト | API ドキュメントコメント | `cargo test --doc` |
| プロパティテスト | 任意のテスト位置 | proptest / quickcheck |

### 基本原則

**原則 0：テストの権威ある水源は規範であり、コードではない。** これは本文書において最も重要な原則である。テストはコードが規範に従っているかを検証ものであり、現在の実装が「動作している」かを検証ものではない。テストがコードの振る舞いと規範の間に不一致を発見した場合、**コードを修正し、テストを決して修正しない**。

規範ファイルの位置：
- `docs/src/design/language-spec.md` —— 言語コア規範
- `docs/src/design/rfc/accepted/` —— 受理された RFC 設計文書

各テストファイルの先頭には対応する規範セクションを宣言しなければならない（規則 2.1 参照）。開発者は規範ドキュメントを手にテストと照らし合わせて実装の正しさを検証できるべきである。逆もまた然り——もしあるコードに対応する規範記述が存在しないなら、それは存在するべきではなく、テストされるべきでもない。

```rust
// 🟢 良い——テストは規範を直接参照し、コードが規範に従っているかを検証
//! リテラルテスト — 言語規範 §2.6 に基づく
//!
//! §2.6.1: 整数 Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: 浮動小数点数（小数点と指数を含む）
//! §2.6.3: 文字列（エスケープシーケンス \\nrt'"\\, \\x, \\u{}）
//! RFC-012: F-String 補間

#[test]
fn test_decimal_literal_parsing() {
    // 規範 §2.6.1: Decimal ::= [0-9][0-9_]*
    let result = parse_literal("42").unwrap();
    assert_eq!(result, Literal::Int(42));
}

// 🔴 悪い——テストが現在のコードの実装に従って調整されており、規範を検証していない
#[test]
fn test_literal_1() {
    // このコードが規範のどの節に該当するのか不明
    // parse_literal が誤った値を返した場合、このテストは「合格」
    // 関数が panic しないことだけを検証しているため
    let result = parse_literal("42");
    assert!(result.is_ok());
}
```

**シナリオ**：あなたはテストを書き、コードの振る舞いが規範に反していることに気づいた。二つの選択肢がある：
| 誤った做法 | 正しい做法 |
|------------|------------|
| テストを「通過するように」修正する | コードを修正し、規範に従わせる |
| テストに `#[ignore]` を追加する | 直ちにコード実装を修正する |
| コードに合わせてテストに特別な条件分岐を追加する | 分岐を削除し、テストで問題を露呈させる |

覚えておくこと：**赤い結果 = コードが間違っているであり、テストが間違っているのではない。**（ただし、テスト自体にバグがある場合は別。それはまた別の話である。）

**原則 1：テストはドキュメントである。** 開発者はテストを読むだけで被テストコードの振る舞いを理解できるべきであり、追加のコメントや外部ドキュメントを必要としない。

```rust
// 🟢 良い——テスト名で何をテストし、何を期待するかを説明している
#[test]
fn test_tokenize_empty_input_returns_eof() {
    let tokens = tokenize("").unwrap();
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].kind, TokenKind::Eof));
}

// 🔴 悪い——何をテストしているのか誰にもわからない
#[test]
fn test_tokenize_1() {
    let tokens = tokenize("").unwrap();
    assert!(tokens.len() > 0);
}
```

**原則 2：ランダムな失敗は許容しない。** テストはあらゆる環境で繰り返し実行可能でなければならない。乱数、システム時刻、スレッドスケジューリング順序に依存するテストは、シード固定またはモックを使用する必要がある。

**原則 3：一つのテストは一件事だけをテストする。** テスト名で「と」で複数の振る舞いを接続する必要がある場合は、複数のテストに分割する。

```rust
// 🟢 良い——各テストは一つのシナリオのみを検証
#[test]
fn test_parse_int_positive() { /* ... */ }
#[test]
fn test_parse_int_zero() { /* ... */ }

// 🔴 悪い——一つのテストに無関係な内容を詰め込みすぎている
#[test]
fn test_parser() {
    // tokenize をテスト、parse をテスト、typecheck をテスト、codegen をテスト...
}
```

**原則 4：振る舞いをテストし、実装をテストしない。** 内部実装のリファクタリングはテストの失敗を引き起こすべきではない。一行の実装を変更して 10 個のテストが失敗するなら、あなたのテストの書き方が間違っている。

しかしここに重要な区別がある：**「振る舞い」の定義は規範から来たものであり、現在のコードの動きからは来ない。** コードが振る舞いを変えた（規範に反する新しい振る舞い）場合、テストは失敗しなければならない。これ做不到場合、あなたのテストは「コードに合わせ込んだテスト」——バグが入り込むことを許している——である。

```
規範（language-spec.md / RFC）  ──定義──►  期待する振る舞い  ──駆動──►  テスト
                                           │
現在のコード  ──実装──►  実際の振る舞い  ──比較──►  テスト結果

実際の振る舞い ≠ 期待する振る舞いの場合：
  テストは失敗しなければならない（赤信号）  ──►  コードを修正  ──►  テストが通過（緑信号）
  
実際の振る舞い = 期待する振る舞いの場合（ただし実装が悪い）：
  テストが通過  ──►  実装をリファクタリング  ──►  テストは今も通過  ← これこそが原則 4 の意味
```

**原則 5：フォールバック/互換性/特定パターンが動作するテストコードを書かない。** テスト環境はあなたが完全に制御できる環境である。`#[cfg(not(ci))]` で特定のテストをスキップする必要があるなら、そのテストの設計に根本的な問題がある。

### 用語定義

| 用語 | 定義 |
|------|------|
| ユニットテスト | 単一の関数やモジュールの振る舞いをテストし、外部システムに依存しない |
| 統合テスト | 複数のモジュールが協調して動作することをテストし、パブリック API またはコマンドライン入口を通じて行う |
| パフォーマンステスト | コードのパフォーマンスを測定し、パフォーマンスリグレッションを検出する |
| ドキュメントテスト | ドキュメントコメントに埋め込まれた実行可能なコード例 |
| プロパティテスト | ランダム入力に基づいて不変量（property）を検証するテスト |

### コミット規範との関連

すべてのテスト関連コミットは `:white_check_mark: test:` タイプを使用しなければならず、[コミット規範](./commit-convention.md)を参照のこと。

```
:white_check_mark: test(parser): Pratt パーサーの中置式テストを追加
:white_check_mark: test(codegen): switch 文の IR 生成テストを補完
```

---

## ユニットテスト規範

### ファイル構成

**規則 1.1**：ユニットテストの `tests/` ディレクトリは、被テストモジュールの `mod.rs` **と同レベル**に配置しなければならない。`tests/` は上位に集約されず、跨いで集計されない。

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
└── tests/              # parser モジュールのテスト（pratt サブモジュールの内容を含まない）
    ├── mod.rs
    ├── ast.rs
    ├── expressions.rs
    ├── error_recovery.rs
    └── parser_state.rs
```

主要な判断基準：**`tests/` をどのディレクトリに置くかによって、どのディレクトリの `mod.rs` が `#[cfg(test)] mod tests;` でそれを宣言しなければならないかが決まる。**

#### 単一ファイルモジュール vs ディレクトリモジュールのテスト配置規則

**主な区別**：モジュールの組織形式がテストの配置位置を決定する。

| モジュール種別 | 判断基準 | テスト位置 | 例 |
|----------------|----------|------------|-----|
| **ディレクトリモジュール** | 独立ディレクトリと `mod.rs` を持つ | そのディレクトリ下の `tests/` | `inference/tests/` |
| **単一ファイルモジュール** | `.rs` ファイルのみ、独立ディレクトリなし | 親モジュールの `tests/` | `overload.rs` → `typecheck/tests/overload.rs` |

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
│   ├── overload.rs                 # overload.rs のテスト（単一ファイルモジュールのテストはこちら）
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
└── traits/                         # ディレクトリモジュール（mod.rs あり）
    ├── mod.rs                      # #[cfg(test)] mod tests; ——同レベルの tests/ を宣言
    ├── solver.rs
    ├── impl_check.rs
    ├── inheritance.rs
    ├── coherence.rs
    ├── auto_derive.rs
    ├── object_safety.rs
    ├── resolution.rs
    ├── std_traits.rs
    ├── gat/
    ├── specialization/
    └── tests/                      # ✅ traits のテストディレクトリ
        ├── mod.rs
        ├── solver.rs               # solver.rs のテスト
        ├── impl_check.rs           # impl_check.rs のテスト
        └── ...
```

**なぜ単一ファイルモジュールのテストを親レベルの `tests/` に置くのか？**

単一ファイルモジュール（例：`overload.rs`）は独自の `mod.rs` を持たないため、`#[cfg(test)] mod tests;` を宣言できない。Rust のモジュールシステムにより、テストファイルは何かの `mod.rs` で宣言されなければコンパイルされない。 따라서、単一ファイルモジュールのテストは親モジュールの `mod.rs` で宣言され、親レベルの `tests/` ディレクトリに配置するしかない。

**判断フロー**：

```
モジュールに遭遇したら、どこにテストを置くか判断する
│
├── そのモジュールはディレクトリか（mod.rs あり）？
│   └── はい → そのディレクトリ下に tests/ を作成し、そのディレクトリ mod.rs が宣言
│
├── そのモジュールは単一ファイルか（.rs ファイルのみ）？
│   └── はい → テストは親の tests/ ディレクトリに置き、親の mod.rs が宣言
│
└── 不確実？
    └── 独立ディレクトリと mod.rs があるか確認
```

**よくある誤り**：

```
# ❌ 誤り 1：単一ファイルモジュール用に独立の tests/ ディレクトリを作成
src/frontend/core/typecheck/
├── overload.rs
└── overload/                       # ❌ 単一ファイルモジュール用にディレクトリを作成すべきでない
    └── tests/
        └── overload.rs

# ❌ 誤り 2：単一ファイルモジュール内で #[cfg(test)] mod tests; を宣言
# overload.rs
#[cfg(test)]                        # ❌ 単一ファイルモジュールではこのように宣言できない
mod tests;                          # overload/tests/ ディレクトリが存在しないため

# ✅ 正しい做法：テストは親レベルの tests/ に配置
src/frontend/core/typecheck/
├── overload.rs                     # ソースファイル
└── tests/
    └── overload.rs                 # テストファイル、typecheck/mod.rs が宣言
```

⚠️ **アンチパターン——次のように書かない**：

```
# ❌ 誤り：サブモジュールのテストを親レベルに集中させる
src/frontend/core/types/
├── mod.rs              # 本来は base と computation のみを宣言すべき
├── base/
│   ├── mod.rs
│   └── var.rs
└── tests/              # ❌ 親の tests/ にサブモジュールのテストが含まれている
    ├── mod.rs          # ❌ mod base; mod computation; を宣言せざるを得ない
    ├── base/           # ❌ この部分は base/tests/ に配置すべき
    │   └── var.rs
    └── computation/    # ❌ この部分は computation/tests/ に配置すべき
        └── ...
```

```
# ✅ 正しい做法：各モジュールのテストは独立
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

**なぜ上位への集約は許されないのか？** Rust のモジュールシステムでは、`#[cfg(test)] mod tests;` は宣言時点でテストファイルのコンパイルを決定する。`types/mod.rs` が `mod tests;` を宣言するなら、`types/tests/` の内容は `types` モジュールのプライベート内容であり、`base` や `computation` の領域に跨ぐべきではない。各モジュールのテストはそのモジュールの内部実装の詳細であり、親モジュールのそれではない。この規則はモジュールリファクタリングにも適用される：`types` を `base` と `computation` に分割する際、テストも分割後のモジュールに従って分割すべきであり、元の場所に残すべきではない。**テストディレクトリはソース構造をミラーするのでなく、モジュール境界に従う。**

**規則 1.2**：`tests/mod.rs` はモジュールの宣言と re-export のみを担当し、テスト関数は配置しない。

```rust
//! Parser コアテスト — src/frontend/core/parser/ を反映
//!
//! ast.rs, parser_state.rs と式/統合パーステストのテスト。

mod ast;
mod error_recovery;
mod expressions;
mod integration;
mod parser_state;
```

**規則 1.3**：各テストファイルは一つのソースファイルのみに対応する。複数のソースモジュールのテストを同じファイルに混在させてはならない。

**規則 1.4**：`#[cfg(test)]` は次の二つの位置にしか出現してはならない——`lib.rs` 内の `mod tests` 宣言、または被テストソースファイル内のインライン宣言 `#[cfg(test)] mod tests;`。他の場所での使用は禁止。

```rust
// src/frontend/core/parser/mod.rs または lib.rs
#[cfg(test)]
mod tests;
```

### モジュール宣言規範

**規則 2.1**：すべてのテストファイルの先頭にはモジュールレベルのドキュメントコメント `//!` が必要であり、テストがカバーする規範源（言語規範セクション番号 + RFC 番号）を説明しなければならない。特定のテストが規範セクションを参照していないなら、そのコードは規範的依据がない——存在すべきではなく、テストされるべきでもない。

```rust
//! リテラルテスト — 言語規範 §2.6 に基づく
//!
//! §2.6.1: 整数 Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: 浮動小数点数（小数点と指数を含む）
//! §2.6.3: 文字列（エスケープシーケンス \\nrt'"\\, \\x, \\u{}）
//! RFC-012: F-String 補間
```

**なぜ規範を参照しなければならないのか？** テストの期待値は規範から来るものであり、「現在のコードの出力」から来るべきではない。いつかコードが出力を変えた場合、テスト随之更新するなら、そのテストは何も保護していない。規範に結びついたテストだけが「意図的な breaking change」と「意図しないリグレッション」を区別できる。

**規則 2.2**：テストモジュールの `use` インポートは具体的な型/関数に精密に 指定しなければならず、glob インポート `use super::*;` は禁止。

```rust
// 🟢 良い——精密なインポート
use crate::frontend::core::lexer::{tokenize, TokenKind};
use crate::frontend::core::parser::{ParserState, ParseError};

// 🔴 悪い——何をテストしているのか他者にはわからない
use super::*;
```

### 命名規範

**規則 3.1**：テスト関数の命名形式は `test_<what>_<scenario>` とし、すべて小文字でアンダースコア区切りとする。

```rust
#[test]
fn test_tokenize_empty_string() { /* ... */ }
#[test]
fn test_parse_int_overflow() { /* ... */ }
#[test]
fn test_typecheck_fn_return_mismatch() { /* ... */ }
```

**規則 3.2**：テスト関数名は自己説明的 でなければならない。関数名を読めば何をテストし何を期待するかがわかるべきである。数字の連番による命名は禁止。

```rust
// 🟢 良い
fn test_skip_semicolon_success() { /* ... */ }
fn test_skip_semicolon_failure_when_identifier() { /* ... */ }

// 🔴 悪い——何をテストしているのか全くわからない
fn test_skip_1() { /* ... */ }
fn test_skip_2() { /* ... */ }
```

**規則 3.3**：ヘルパー関数には `test_` 接頭辞は不要であり、動詞または名詞で用途を記述すべきである。

```rust
fn parse_expr(source: &str) -> Expr { /* ... */ }
fn tokenize_single(source: &str) -> Token { /* ... */ }
fn setup_parser_with_tokens(tokens: &[Token]) -> ParserState { /* ... */ }
```

### テスト構造規範 (Arrange-Act-Assert)

**規則 4.1**：各テスト関数は三段階構造に従わなければならない：準備（Arrange）→ 実行（Act）→ 断言（Assert）、三段階の間は空行で 区切る。

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

**規則 4.2**：単純なテスト（単一の呼び出し + 単一の断言）は段階コメントを省略できるが、5 行以上のロジックコードを含んでならない。5 行を超えるテストは三段階を明示的に表示しなければならない。

### ヘルパー関数規範

**規則 5.1**：3 回以上繰り返される setup ロジックはヘルパー関数として抽出しなければならない。

```rust
// 🟢 良い——公共 setup を抽出
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

**規則 5.2**：ヘルパー関数内の `unwrap()` / `expect()` は panic 時に十分なコンテキストを出力しなければならない。テスト関数本体（`#[test] fn ...`）では直接 `unwrap()` してよい——失敗時、Rust は自動的に行番号を出力するが、ヘルパー関数内で失敗した場合、行番号はヘルパー関数定義處を指し、呼び出し時のコンテキストが見えない。

```rust
// 🟢 良い——ヘルパー関数の失敗時にソースコードの内容を出力
fn run_ok(source: &str) {
    run(source).unwrap_or_else(|e| panic!("Execution failed:\nSource:\n{}\nError:\n{:?}", source, e));
}

// 🔴 悪い——失敗時にどのソースファイルが問題を起こしたのかわからない
fn run_ok(source: &str) {
    run(source).unwrap();
}
```

**規則 5.3**：ヘルパー関数はテストファイルの先頭、`use` インポートの直後に配置すべきである。複数のテストモジュールで共有される場合、`tests/mod.rs` に配置し `pub(crate)` でエクスポートする。

### 断言スタイル

**規則 6.1**：列挙型変体のマッチングには `assert!(matches!(...))` を使用し、`if let` + `panic!` は使用禁止。

```rust
// 🟢 良い
assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(42)));

// 🔴 悪い
if let TokenKind::IntLiteral(v) = tokens[0].kind {
    assert_eq!(v, 42);
} else {
    panic!("Expected IntLiteral");
}
```

**規則 6.2**：精密な値比較には `assert_eq!` を使用し、ブール断言には `assert!` を使用する。`assert!(a == b)` を `assert_eq!(a, b)` の代わりに使用することは禁止。

**規則 6.3**：すべての断言にはカスタムエラーメッセージを含まなければならない。ただし、断言自体が既に失敗理由を完全に記述している場合はこの限りでない。

```rust
// 🟢 良い——断言失敗時に素早く特定できる
assert!(
    state.infix_info().is_some(),
    "infix_info should handle '{op}'"
);

// 🟢 良い——assert_eq! の失敗時は自動的に値の差分を出力するため、追加メッセージ不要
assert_eq!(error_count, 0);

// 🔴 悪い——失敗しても「assertion failed」であることしかわからない
assert!(state.infix_info().is_some());
```

**規則 6.4**：断言の順序は `assert_eq!(actual, expected)` とし、実際の値を先に、期待する値を後に 配置する。

### アンチパターンの一覧

以下は禁止された記述法とその代替案である：

| アンチパターン | 問題点 | 代替案 |
|----------------|--------|--------|
| テストがコードの誤った振る舞いに合わせている | 規範逸脱を隠蔽し、バグを合法化させる | 規範に従ってコードを修正し、テストはそのまま維持 |
| コードの出力を元にテストの期待値を逆算 | テストが「現在の実装の録音機」になる | 規範から期待値を導出 |
| `#[ignore]` の永続的なマーク | 腐ったテストを隠蔽 | 修正または削除 |
| `println!` デバッグ出力 | テスト出力を汚染 | `assert!` を使用して明確に断言 |
| `thread::sleep` | ランダム失敗 + 遅延 | 同期メカニズムまたはモックを使用 |
| テストで実際のファイルシステムを操作 | 遅延があり再現性がない | `tempfile` を使用 |
| テスト実行順序に依存 | ランダム失敗 | 各テストが独立した setup を持つ |
| 一つのテスト関数が 30 行を超えるロジック | 誰も理解できない | テストを分割またはヘルパー関数を使用 |
| ヘルパー関数内の `unwrap()` がコンテキストを報告しない | 特定が困難 | `expect("理由")` またはカスタム panic を使用（規則 5.2 参照） |
| 同一 setup を 3 回以上 copy-paste | 修正コストが高い | ヘルパー関数を抽出 |

---

## 統合テスト規範

### テスト構成

**規則 7.1**：統合テストはプロジェクトルートの `tests/` ディレクトリに配置する。入口ファイル `tests/integration.rs` は `#[path]` 属性を使用してサブモジュールを導入する。

```rust
// tests/integration.rs
#[path = "integration/backends.rs"]
mod backends;
#[path = "integration/codegen.rs"]
mod codegen;
#[path = "integration/execution.rs"]
mod execution;
```

**規則 7.2**：各 `tests/integration/*.rs` ファイルは一つのテストテーマ（コンパイラバックエンド、コード生成、エグゼキュータなど）に対応し、混合配置は禁止。

**規則 7.3**：統合テストはプロジェクトのパブリック API を通じてテストしなければならない。統合テストで `crate::` 内部モジュールを直接参照することは禁止。パブリックパス `yaoxiang::` を使用する。

```rust
// 🟢 良い——パブリック API 経由
use yaoxiang::run;

// 🔴 悪い——パブリック API 境界をバイパス
use yaoxiang::middle::codegen::bytecode::BytecodeFile;
```

### テストデータ管理

**規則 8.1**：統合テストはインラインソースコード文字列を使用を優先する。ソースコードが 30 行を超える場合にのみ、外部 fixture ファイル（`tests/fixtures/` に配置）を使用する。

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

**規則 8.2**：fixture ファイルは `.yx` 拡張子で終わり、ファイル名はテストの意図を記述する。

### E2E カバレッジ原則

**規則 9.1**：各言語特性の統合テストは三つのパスをカバーしなければならない：

| パス | 説明 |
|------|------|
| Happy path | 合法入力が予想出力を生成 |
| Error path | 違法入力が明確なエラーメッセージを生成（非 panic） |
| Boundary | 境界値（空入力、最大値、ネスト深度の上限） |

**規則 9.2**：統合テストはネットワーク、システム環境変数、外部サービスに依存してはならない。

---

## パフォーマンステスト規範

### Criterion.rs 使用規範

**規則 10.1**：パフォーマンステストは `benches/` ディレクトリに統一して配置し、入口ファイルは `benches/lib.rs` とする。テストテーマごとにファイルを分割する。

```
benches/
├── lib.rs              # 入口、criterion_group/criterion_main を定義
├── lang_compare/
│   └── fibonacci.rs    # 言語間比較ベンチマーク
├── parser.rs           # パーサー benchmark
└── codegen.rs          # コード生成 benchmark
```

**規則 10.2**：各ベンチマーク関数には `//!` モジュールドキュメントコメントを含まなければならず、テスト目的と測定指標を説明する。

```rust
//! YaoXiang インタープリタ性能ベンチマークテスト
//!
//! 測定指標：単一反復時間（wall time）
//! ベースライン：Rust ネイティブ実装
```

### コンパイラの最適化防止

**規則 11.1**：すべてのパフォーマンステストの被テスト出力は `criterion::black_box` を使用してコンパイラの最適化消除を防ぐ。

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

**規則 11.2**：パフォーマンステストの入力データは `const` または `lazy_static` でなければならず、`iter` クロージャ内で動的に生成してはならない——さもなくば測定するのはデータ生成 + 被テストロジックの合計時間となる。

### ベンチマークのグループ化と命名

**規則 12.1**：パフォーマンステストの命名形式は `<被テストモジュール>_<シナリオ>` とし、すべて小文字でアンダースコア区切りとする。ユニットテストの命名規則と一貫性を保つ。

**規則 12.2**：`criterion_group!` を使用して関連するベンチマークを論理的にグループ化しなければならない。すべてのベンチマークを一つのグループに押し込めることは禁止。

```rust
criterion_group!(parser, bench_parse_expr, bench_parse_stmt);
criterion_group!(codegen, bench_codegen_module, bench_codegen_switch);
criterion_main!(parser, codegen);
```

---

## ドキュメントテスト規範

### 使用シナリオ

**規則 13.1**：すべての `pub` 関数、型、メソッドにはドキュメントコメントに少なくとも一つの実行可能なコード例を含まなければならない。その例は `cargo test --doc` で実行される。

```rust
/// ソースコード文字列を Token のシーケンスに字句解析する。
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

**規則 13.2**：ドキュメントテストのコード例はコンパイルが通り、断言が成功しなければならない。コンパイル時エラーを示す例でない限り、`ignore` マーク付きの例を含んでならない。

```rust
/// ```ignore
/// // コンパイル時エラーを示す——ignore 可
/// let x: int = "string";
/// ```
```

### カバレッジ要件

**規則 14.1**：ドキュメントテストは API の happy path をカバーすればよい。境界情况和錯誤パスはユニットテストでカバーする。

**規則 14.2**：ドキュメントテストのコード例は簡潔でなければならない——10 行を超えない。例により長いコンテキストが必要な場合は、API 設計に問題がある。

---

## プロパティテスト規範

### 使用シナリオ

**規則 15.1**：以下のシナリオでは手動で複数の境界値テストケースを書くのではなく、プロパティテスト（proptest または quickcheck）を使用しなければならない：

| シナリオ | 例 |
|----------|-----|
| パーサー round-trip | `parse(pretty_print(ast)) == ast` |
| シリアライズ/デシリアライズ | `deserialize(serialize(data)) == data` |
| 数学演算の恒等式 | `a + b == b + a` |
| コンパイラ最適化が意味を変えない | `eval(code) == eval(optimize(code))` |

**規則 15.2**：プロパティテストは主要なプロパティテストフレームワークとして `proptest` を使用する（`Cargo.toml` の `dev-dependencies` に宣言済み）。

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

### プロパティ定義原則

**規則 16.1**：各プロパティテストには明確なプロパティ宣言が必要である——コメントに検証する不変量を記述する。

```rust
// プロパティ：任意のリテラル整数は tokenize → tokens_to_string 後に同じ値を生成
proptest! {
    #[test]
    fn test_int_literal_roundtrip(n in any::<i64>()) {
        let source = n.to_string();
        let tokens = tokenize(&source).unwrap();
        // ...
    }
}
```

**規則 16.2**：プロパティテストが失敗を発見した場合、proptest のリグレッション機構を使用する——失敗した入力を `proptest-regressions/` ディレクトリに追加し、手動で通常のテストを書いて代替してはならない。

---

## カバレッジ要件

### 新規コードのカバレッジ目標

**規則 17.1**：新規コードのテストカバレッジ要件：

| コード種別 | 行カバレッジ | 分岐カバレッジ |
|------------|--------------|----------------|
| コアコンパイラモジュール（frontend/middle/backends） | ≥ 85% | ≥ 80% |
| ユーティリティ/補助モジュール（util） | ≥ 75% | ≥ 70% |
| 実行時モジュール（vm/runtime） | ≥ 80% | ≥ 75% |
| 標準ライブラリ（std） | ≥ 75% | ≥ 70% |
| エラー処理と診断 | ≥ 90% | ≥ 85% |

**規則 17.2**：エラー処理パス（すべての `Err` 分岐）は 100% カバレッジが必要である。ユーザーが目にするエラーメッセージはテストで検証されていなければならない。

### PR レビュー確認リスト

**規則 18.1**：PR を提出する前に、作成者は以下の項目を自己確認しなければならない：

- [ ] `cargo test` がすべて通過
- [ ] `cargo test --doc` がすべて通過
- [ ] `cargo bench` にパフォーマンスリグレッションがない（ヒートパスの変更に関与する場合）
- [ ] 新規コードがカバレッジ目標を満たしている
- [ ] テスト名が命名規範に適合している
- [ ] 各テストファイルが対応する規範セクションを宣言している（規則 2.1）
- [ ] テストの期待値が規範定義から来ており、「現在のコードの出力」からは来ていない
- [ ] `#[ignore]` マーク付きテストがない（明確な issue 番号コメントがある場合を除く）
- [ ] 不要な `unwrap()` がない（`expect` またはカスタム panic メッセージを使用すべき）
- [ ] コミットメッセージが `:white_check_mark: test:` タイプを使用している
- [ ] **「コードの振る舞いが規範に反している」ことを理由にテストの期待値を修正していない——修正するのはコードであり、テストではない**

**規則 18.2**：レビュアーは以下の問題を含む PR を拒否しなければならない：

- happy path テストのみで、錯誤パスがない
- テストに `thread::sleep` 或は実行順序への依存がある
- 3 回以上のコピペテストコードがありながらヘルパー関数を抽出していない
- テスト名が命名規範に適合していない
- 永続的な `#[ignore]` のテストが存在する
- **テストがコードの誤った振る舞いに合わせている**（コードが規範に反している場合、テストではなくコードを修正する）
- **テストが対応する規範セクションを宣言していない**（規則 2.1 参照）
- **テストの期待値がコード出力から逆算されており、規範から来ていない**（逆算されたテストはテストとして機能しない）
- テストが「panic しないこと」のみを検証し、具体的な振る舞いを断言していない
- コードのバグを露呈した失敗したテストを削除した（コードを修正してから緑信号を見るのではなく）

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

# ベンチマークを実行
cargo bench

# テスト出力を表示（デフォルトでは非表示）
cargo test -- --nocapture

# 単一スレッドで実行（並行問題排查）
cargo test -- --test-threads=1

# カバレッジレポートを生成（cargo-llvm-cov が必要）
cargo llvm-cov --html
```

### B. コミットメッセージテンプレート

テスト関連コミットは次のテンプレートに従わなければならない：

```
:white_check_mark: test(<scope>): <短い説明>

<任意：カバーするシナリオリスト>
```

例：

```
:white_check_mark: test(parser): Pratt パーサーの中置演算子テストを追加

カバーするシナリオ：
- 算術演算子の優先順位（+, -, *, /, %）
- 比較演算子の連結（1 < x < 10）
- 論理演算子の短絡評価
- 代入演算子の右結合
```

### C. 新規テストファイルチェックリスト

新しいテストモジュールを作成する場合、以下のファイルを含めることを確認する：

```
# src/<module>/ ディレクトリ以下にテストを追加
src/<module>/tests/
├── mod.rs          # モジュール宣言 + 公共ヘルパー関数
└── <subject>.rs    # テストファイル、被テストソースファイルの命名に対応

# tests/ ディレクトリ以下に統合テストを追加
tests/
├── integration.rs   # 更新：#[path] 宣言を追加
└── integration/
    └── <topic>.rs   # 新規テストファイル
```

### D. 参考資料

- [YaoXiang 言語規範](../../design/language-spec.md) —— **テストの権威ある水源**
- [受理された RFC](../../design/rfc/accepted/) —— **設計決定の権威ある水源**
- [Rust テストドキュメント](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Criterion.rs ユーザーガイド](https://bheisler.github.io/criterion.rs/book/)
- [proptest ドキュメント](https://docs.rs/proptest/latest/proptest/)
- [プロジェクトコミット規範](./commit-convention.md)
- [プロジェクト貢献ガイド](./contributing.md)

---

> 💡 **覚えておくこと**：テストはコードが「動作する」かを検証するのではなく、コードが規範に従っているかを検証する。規範が変化すれば、テストも規範に従って変化する。コードが間違っていれば、コード を修正し、テストを修正しない。**コードは規範に奉仕し、テストは規範を守る。テストがコードに合わせ込んだ瞬間、すべての保護を失う。**
```
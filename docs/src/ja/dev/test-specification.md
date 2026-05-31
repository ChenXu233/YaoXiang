---
title: "テスト記述規範"
description: YaoXiang プロジェクトのテスト記述に関する厳格な規範であり、ユニットテスト、統合テスト、ベンチマークテスト、ドキュメンテーションテスト、プロパティテストの記述標準を定義する
---

# テスト記述規範

本文書は YaoXiang プロジェクトのテスト記述に関する厳格な規範を定義する。すべての貢献者は以下のルール遵守しなければならない。違反者は Code Review において修正を要求される。

---

## 目次

- [総則](#総則)
- [ユニットテスト規範](#ユニットテスト規範)
- [統合テスト規範](#統合テスト規範)
- [ベンチマークテスト規範](#ベンチマークテスト規範)
- [ドキュメンテーションテスト規範](#ドキュメンテーションテスト規範)
- [プロパティテスト規範](#プロパティテスト規範)
- [カバレッジ要件](#カバレッジ要件)
- [付録](#付録)

---

## 総則

### 適用範囲

本規範は YaoXiang プロジェクトにおけるすべての Rust テストコードに適用される：

| テストタイプ | 位置 | フレームワーク |
|----------|------|------|
| ユニットテスト | `src/<module>/tests/` | `#[test]` + `#[cfg(test)]` |
| 統合テスト | `tests/` | `#[test]` |
| ベンチマークテスト | `benches/` | Criterion.rs |
| ドキュメンテーションテスト | API ドキュメントコメント | `cargo test --doc` |
| プロパティテスト | 任意のテスト位置 | proptest / quickcheck |

### 基本原則

**原則 0：テストの権威あるソースは規範であり、コードではない。** これは本文書において最も重要な原則である。テストはコードが規範に適合するかを検証するものであり、現在の実装で「たまたま動いた」かどうかを検証するものではない。テストがコードの動作が規範と一致しないことを発見した場合、**コードを修正し、、決してテストを修正しない**。

規範ファイルは以下の場所にある：
- `docs/src/design/language-spec.md` —— 言語コア規範
- `docs/src/design/rfc/accepted/` —— 受理済みの RFC 設計文書

各テストファイルの先頭には対応する規範セクションを宣言しなければならない（ルール 2.1 参照）。開発者は規範文書を持ってテストと照らし合わせ、実装の正確性を検証できるべきである。逆もまた然り——もしあるコードに対応する規範記述が存在しないなら、そのコードは存在すべきではなく、テストされるべきでもない。

```rust
// 🟢 良い——テストは規範を直接参照し、コードが規範に従うかを検証する
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

// 🔴 悪い——テストが現在のコードの実装動作に迎合しており、規範を検証していない
#[test]
fn test_literal_1() {
    // このコードが規範のどの節に該当するのかわからない
    // parse_literal が誤った値を返しても、このテストは「グリーン通過」する
    // 関数が panic しないことだけを検証しているため
    let result = parse_literal("42");
    assert!(result.is_ok());
}
```

**シナリオ**：テストを書いて、コードの動作が規範に適合しないことを発見した。選択は2つ：
| 誤った做法 | 正しい做法 |
|----------|----------|
| テストを「通過する」ように修正する | コードの動作を規範に適合させる |
| テストに `#[ignore]` を追加する | 直ちにコード実装を修正する |
| コード迎合のためにテストに特殊条件分岐を追加する | 分岐を削除し、テストが直接問題を露呈させる |

覚えておくべきこと：**红灯 = コードが間違っており、テストが間違っているのではない。**（ただし、テスト自体にバグがある場合は別の話。）

**原則 1：テストはドキュメントである。** すべての開発者がテストを読むことで被テストコードの動作を理解できるべきであり、追加のコメントや外部ドキュメントを必要としない。

```rust
// 🟢 良い——テスト名が何をテストし、何を期待するかを説明している
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

**原則 2：ランダムな失敗はゼロ容忍。** テストはあらゆる環境で再現可能に実行されなければならない。乱数、システム時刻、スレッドスケジューリング順序に依存するテストは、シード固定または mock を使用しなければならない。

**原則 3：一つのテストは一件事だけをテストする。** テスト名が「と」で複数の動作を接続する必要がある場合は、複数のテストに分割する。

```rust
// 🟢 良い——各テストは1つのシナリオのみを検証する
#[test]
fn test_parse_int_positive() { /* ... */ }
#[test]
fn test_parse_int_zero() { /* ... */ }

// 🔴 悪い——1つのテストに必要以上の無関係な内容を詰め込んでいる
#[test]
fn test_parser() {
    // tokenize をテストし、parse をテストし、typecheck をテストし、codegen をテストし...
}
```

**原則 4：動作をテストし、実装をテストしない。** 内部実装のリファクタリングはテストの失敗を招くべきではない。1行の実装コードを変更して10個のテストが落ちるなら、テストの書き方が間違っている。

しかし、ここに重要な区別がある：**「動作」の定義は規範から来たものであり、現在のコードの動作からは来ない。** コードが動作を変えた（即ち規範に適合しない新しい動作を追加した）場合、テストは失敗しなければならない。これ做不到であれば、テストは「コード迎合型のテスト」——バグが入り込むのを許している——である。

```
規範（language-spec.md / RFC）  ──定義──►  期待動作  ──駆動──►  テスト
                                           │
現在のコード  ──実装──►  実際の動作  ──対比──►  テスト結果

実際の動作 ≠ 期待動作 の場合：
  テストは失敗しなければならない（红灯）  ──►  コードを修正  ──►  テスト通過（绿灯）
  
実際の動作 = 期待動作 の場合（ただし実装が酷い）：
  テスト通過  ──►  実装をリファクタリング  ──►  テストは仍然通過  ← これこそが原則 4 の意味
```

**原則 5：フォールバック/互換/特定パターンが有効になるテストコードは書かない。** テスト環境は完全に制御できる環境である。`#[cfg(not(ci))]` でテストをスキップする必要があるなら、そのテスト設計には根本的な問題がある。

### 用語定義

| 用語 | 定義 |
|------|------|
| ユニットテスト | 単一の関数やモジュールの動作をテストし、外部システムに依存しない |
| 統合テスト | 複数のモジュールが協調して動作することをテストし、パブリック API またはコマンドラインエントリポイント経由 |
| ベンチマークテスト | コードの性能を測定し、パフォーマンスリグレッションを検出する |
| ドキュメンテーションテスト | ドキュメントコメントに埋め込まれた実行可能なコード例 |
| プロパティテスト | ランダム入力に基づいて不変量（property）を検証するテスト |

### コミット規範との関連

すべてのテスト関連コミットは `:white_check_mark: test:` タイプを使用しなければならず、[コミット規範](./commit-convention.md)を参照する。

```
:white_check_mark: test(parser): Pratt パーサーの中置式テストを追加
:white_check_mark: test(codegen): switch 文の IR 生成テストを補完
```

---

## ユニットテスト規範

### ファイル構成

**ルール 1.1**：ユニットテストの `tests/` ディレクトリは被テストモジュールの `mod.rs` **と同レベル**に配置しなければならない。`tests/` は上に集約せず、跨って纏めない。

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
└── tests/              # parser モジュールのテスト（pratt サブモジュールを含まない）
    ├── mod.rs
    ├── ast.rs
    ├── expressions.rs
    ├── error_recovery.rs
    └── parser_state.rs
```

 判断の基準：**`tests/` を置くディレクトリが、そのディレクトリの `mod.rs` で `#[cfg(test)] mod tests;` を宣言しなければならない。**

**ルール 1.1 補足：上方への集約禁止。** サブディレクトリ モジュールのテストは自身のサブディレクトリの `tests/` に配置しなければならず、親レベルの `tests/` に集約してはならない。

| モジュールの種類 | テストの位置 | 例 |
|----------|----------|------|
| ディレクトリモジュール（`mod.rs` あり） | そのディレクトリ下の `tests/` | `emitter/tests/`、`codes/tests/` |
| 単一ファイルモジュール（`.rs` のみ） | 親モジュールの `tests/` | `session.rs` → `diagnostic/tests/session.rs` |

```text
# ✅ 正しい：各ディレクトリモジュールのテストが各自独立
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
    ├── mod.rs              # ❌ mod emitter; mod codes; の宣言を強いられる
    ├── emitter/            # ❌ emitter/tests/ にあるべき
    └── codes/              # ❌ codes/tests/ にあるべき
```

#### 単一ファイルモジュール vs ディレクトリモジュールのテスト配置ルール

**根本的な違い**：モジュールの組織形式がテストの配置位置を決める。

| モジュールの種類 | 判断基準 | テストの位置 | 例 |
|----------|----------|----------|------|
| **ディレクトリモジュール** | 独立ディレクトリと `mod.rs` あり | そのディレクトリ下の `tests/` | `inference/tests/` |
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
│   ├── overload.rs                 # overload.rs のテスト（単一ファイルモジュールテストはここに）
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

**なぜ単一ファイルモジュールのテストは親レベルの `tests/` に置くのか？**

単一ファイルモジュール（例：`overload.rs`）には自身の `mod.rs` がなく、`#[cfg(test)] mod tests;` を宣言できない。Rust のモジュールシステムにより、テストファイルは何かの `mod.rs` で宣言されなければコンパイルされない。だから、単一ファイルモジュールのテストは親モジュールの `mod.rs` で宣言され、親レベルの `tests/` ディレクトリに配置するしかない。

**判断の流れ**：

```
モジュールに遭遇し、テストをどこに置くか判断する？
│
├── そのモジュールはディレクトリか？（mod.rs あり）？
│   └── はい → そのディレクトリ下に tests/ を作成し、そのディレクトリで宣言
│
├── そのモジュールは単一ファイルか？（.rs のみ）？
│   └── はい → テストは親レベルの tests/ ディレクトリに置き、親の mod.rs で宣言
│
└── 不確定？
    └── 独立ディレクトリと mod.rs があるか確認
```

**よくある誤り**：

```
# ❌ 誤り 1：単一ファイルモジュール用に独立の tests/ ディレクトリを作成
src/frontend/core/typecheck/
├── overload.rs
└── overload/                       # ❌ 単一ファイルモジュールにディレクトリを作成すべきでない
    └── tests/
        └── overload.rs

# ❌ 誤り 2：単一ファイルモジュール内で #[cfg(test)] mod tests; を宣言
# overload.rs
#[cfg(test)]                        # ❌ 単一ファイルモジュールでは 이렇게 宣言できない
mod tests;                          # overload/tests/ ディレクトリがないため

# ✅ 正しい做法：テストは親レベルの tests/ に配置
src/frontend/core/typecheck/
├── overload.rs                     # ソースファイル
└── tests/
    └── overload.rs                 # テストファイル、typecheck/mod.rs で宣言
```

⚠️ **反パターン——这样做しない：** 

```
# ❌ 誤り：サブモジュールのテストを親レベルに集中させる
src/frontend/core/types/
├── mod.rs              # base と computation のみを宣言すべき
├── base/
│   ├── mod.rs
│   └── var.rs
└── tests/              # ❌ 親レベルの tests/ にサブモジュールのテストが含まれている
    ├── mod.rs          # ❌ mod base; mod computation; の宣言を強いられる
    ├── base/           # ❌ この部分は base/tests/ にあるべき
    │   └── var.rs
    └── computation/    # ❌ この部分は computation/tests/ にあるべき
        └── ...
```

```
# ✅ 正しい做法：各モジュールのテストが各自独立
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

**なぜ上方への集約はできないのか？** Rust のモジュールシステムは `#[cfg(test)] mod tests;` が宣言された時点でテストファイルのコンパイルが決まるからである。`types/mod.rs` が `mod tests;` を宣言すると、`types/tests/` の内容は `types` モジュールのプライベートな内容になる——それは `base` や `computation` の領域に跨いではならない。各モジュールのテストはそのモジュールの内部実装の詳細であり、親モジュールのそれではない。このルールはモジュールのリファクタリングにも適用される：`types` を `base` と `computation` に分割する時、テストも分割後のモジュールに従って分割されるべきであり、現状に留まるべきではない。**テストディレクトリはソース構造をミラーイングするのではなく、モジュール境界に従う。**

**ルール 1.2**：`tests/mod.rs` はモジュールの宣言と re-export のみ担当し、テスト関数は配置しない。

```rust
//! Parser core tests — src/frontend/core/parser/ をミラー
//!
//! ast.rs、parser_state.rs、式/統合パース 테스트。

mod ast;
mod error_recovery;
mod expressions;
mod integration;
mod parser_state;
```

**ルール 1.3**：各テストファイルは1つのソースファイルのみに対応する。複数のソースモジュールのテストを1つのファイルに混在させてはならない。

**ルール 1.4**：`#[cfg(test)]` は2つの位置にしか出現しない——`lib.rs` での `mod tests` 宣言、または被テストソースファイル内のインライン宣言 `#[cfg(test)] mod tests;`。他の場所での使用は禁止。

```rust
// src/frontend/core/parser/mod.rs または lib.rs
#[cfg(test)]
mod tests;
```

### モジュール宣言規範

**ルール 2.1**：すべてのテストファイルの先頭にモジュールレベルのドキュメントコメント `//!` を配置し、テストがカバーする規範ソース（言語規範のセクション番号 + RFC 番号）を説明しなければならない。特定のテストが規範セクションを参照していないなら、そのコードは規範的依据がない——存在すべきではなく、テストされるべきでもない。

```rust
//! リテラルテスト — 言語規範 §2.6 に基づく
//!
//! §2.6.1: 整数 Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: 浮動小数点数（小数点と指数を含む）
//! §2.6.3: 文字列（エスケープシーケンス \\nrt'"\\, \\x, \\u{}）
//! RFC-012: F-String 補間
```

**なぜ規範を参照しなければならないのか？** テストの期待値は規範から来ており、「現在のコードの出力」から来ていてはならないからである。いつかコードが出力を変更し、テストがその 따라更新されるなら、テストは何も保護していない。規範にアンカーされたテストだけが「意図的な breaking change」と「意図しないリグレッション」を区別できる。

**ルール 2.2**：テストモジュールの `use` import は具体的な型/関数に精密でなければならず、glob import `use super::*` は禁止。

```rust
// 🟢 良い——精密な import
use crate::frontend::core::lexer::{tokenize, TokenKind};
use crate::frontend::core::parser::{ParserState, ParseError};

// 🔴 悪い——何をテストしているのか他人にはわからない
use super::*;
```

### 命名規範

**ルール 3.1**：テスト関数の命名形式は `test_<what>_<scenario>` とし、すべて小文字でアンダースコア区切りとする。

```rust
#[test]
fn test_tokenize_empty_string() { /* ... */ }
#[test]
fn test_parse_int_overflow() { /* ... */ }
#[test]
fn test_typecheck_fn_return_mismatch() { /* ... */ }
```

**ルール 3.2**：テスト関数名は自己説明的でなければならない。関数名を読めば何をテストし、何を期待するかがわかるべきである。数字シーケンスでの命名は禁止。

```rust
// 🟢 良い
fn test_skip_semicolon_success() { /* ... */ }
fn test_skip_semicolon_failure_when_identifier() { /* ... */ }

// 🔴 悪い——何をテストしているのか全くわからない
fn test_skip_1() { /* ... */ }
fn test_skip_2() { /* ... */ }
```

**ルール 3.3**：ヘルパー関数には `test_` プレフィックスは不要で、その用途を説明する動詞または名詞を使用するべきである。

```rust
fn parse_expr(source: &str) -> Expr { /* ... */ }
fn tokenize_single(source: &str) -> Token { /* ... */ }
fn setup_parser_with_tokens(tokens: &[Token]) -> ParserState { /* ... */ }
```

### テスト構造規範 (Arrange-Act-Assert)

**ルール 4.1**：各テスト関数は3段階構造に従わなければならない：準備（Arrange）→ 実行（Act）→ 断言（Assert）、3段階の間は空行で区切る。

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

**ルール 4.2**：単純なテスト（単一の呼び出し + 単一の断言）は分段コメントを省略してもよいが、5行以上の論理コードがあってはならない。5行を超えるテストは3段階を明示的に標示しなければならない。

### ヘルパー関数規範

**ルール 5.1**：3回以上繰り返し登場する setup 論理はヘルパー関数に抽出しなければならない。

```rust
// 🟢 良い——共通の setup を抽出
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

**ルール 5.2**：ヘルパー関数内の `unwrap()` / `expect()` は panic 時に十分なコンテキストを出力しなければならない。テスト関数本体（`#[test] fn ...`）では直接 `unwrap()` してよい——失敗時は Rust が自動的に行番号を出力する。しかしヘルパー関数内で失敗した時は、行番号はヘルパー関数の定義箇所を指し、呼び出し時のコンテキストが見えない。

```rust
// 🟢 良い——ヘルパー関数の失敗時にソースコード内容を出力
fn run_ok(source: &str) {
    run(source).unwrap_or_else(|e| panic!("Execution failed:\nSource:\n{}\nError:\n{:?}", source, e));
}

// 🔴 悪い——失敗時にどのソースファイルが問題を引き起こしたのかわからない
fn run_ok(source: &str) {
    run(source).unwrap();
}
```

**ルール 5.3**：ヘルパー関数はテストファイルの先頭、`use` import の直後に配置する。複数のテストモジュールで共有する場合は `tests/mod.rs` に配置し `pub(crate)` でエクスポートする。

### 断言スタイル

**ルール 6.1**：枚举型 variant の一致には `assert!(matches!(...))` を使用者优先し、`if let` + `panic!` の使用は禁止。

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

**ルール 6.2**：精密な値比較には `assert_eq!` を使用し、ブール断言には `assert!` を使用する。`assert!(a == b)` を `assert_eq!(a, b)` の代わりに使用することは禁止。

**ルール 6.3**：すべての断言にはカスタムエラーメッセージを含まなければならない。ただし、断言自体が失敗理由を完全に説明している場合は例外。

```rust
// 🟢 良い——断言失敗時に迅速に特定可能
assert!(
    state.infix_info().is_some(),
    "infix_info should handle '{op}'"
);

// 🟢 良い——assert_eq! 失敗時は自動的に値の差分を出力、追加メッセージ不要
assert_eq!(error_count, 0);

// 🔴 悪い——失敗すると「assertion failed」であることしかわからない
assert!(state.infix_info().is_some());
```

**ルール 6.4**：断言の順序は `assert_eq!(actual, expected)` とし、実際の値を前に、期待値を後に配置する。

### 反パターンの一覧

以下は禁止の記述法とその代替案である：

| 反パターン | 問題点 | 代替方案 |
|--------|------|----------|
| テストがコードのエラー動作に迎合 | 規範からの逸脱を隠蔽し、バグを合法化 | 規範に照らしてコードを修正し、テストはそのまま |
| コードの出力を逆算してテスト期待値を設定 | テストが「現在の実装の録音機」になる | 規範から期待値を導出 |
| `#[ignore]` 永続マーク | 腐敗したテストを隠蔽 | 修正または削除 |
| `println!` デバッグ出力 | テスト出力を汚染 | 明確なの断言で `assert!` を使用 |
| `thread::sleep` | ランダム失敗 + 遅延 | 同期メカニズムまたは mock を使用 |
| テストで実際のファイルシステムを操作 | 遅く再現不可 | `tempfile` を使用 |
| テスト実行順序に依存 | ランダム失敗 | 各テストが独立した setup を持つ |
| 1つのテスト関数が30行以上の論理コード | 誰にも理解できない | テストを分割またはヘルパー関数を使用 |
| ヘルパー関数内の `unwrap()` がコンテキストを報告しない | 特定が困難 | `expect("why")` またはカスタム panic を使用（ルール 5.2 参照） |
| 同一 setup を3回以上 copy-paste | 修正コストが高い | ヘルパー関数を抽出 |

---

## 統合テスト規範

### テスト組織

**ルール 7.1**：統合テストはプロジェクトルートの `tests/` ディレクトリに配置する。エントリファイル `tests/integration.rs` は `#[path]` 属性を使用してサブモジュールを引入する。

```rust
// tests/integration.rs
#[path = "integration/backends.rs"]
mod backends;
#[path = "integration/codegen.rs"]
mod codegen;
#[path = "integration/execution.rs"]
mod execution;
```

**ルール 7.2**：各 `tests/integration/*.rs` ファイルは1つのテストテーマ（コンパイラバックエンド、コード生成、エグゼキュータなど）に対応し、混在させてはならない。

**ルール 7.3**：統合テストはプロジェクトのパブリック API 経由でテストしなければならない。統合テストで `crate::` 内部モジュールを直接参照することは禁止。`yaoxiang::` パブリックパスを使用する。

```rust
// 🟢 良い——パブリック API 経由
use yaoxiang::run;

// 🔴 悪い——パブリック API 境界をバイパス
use yaoxiang::middle::codegen::bytecode::BytecodeFile;
```

### テストデータ管理

**ルール 8.1**：統合テストはインラインソースコード文字列を優先して使用する。ソースコードが30行を超える場合にのみ外部 fixture ファイルを使用する（`tests/fixtures/` に配置）。

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

**ルール 8.2**：fixture ファイルは `.yx` 拡張子で終わらせ、ファイル名はテストの意図を説明するものとする。

### E2E カバー原則

**ルール 9.1**：各言語特性の統合テストは3つのパスをカバーしなければならない：

| パス | 説明 |
|------|------|
| Happy path | 合法的な入力が予期される出力を生成 |
| Error path | 非法な入力が明確なエラーメッセージを生成（非 panic） |
| Boundary | 境界値（空入力、最大値、ネスト深度上限） |

**ルール 9.2**：統合テストはネットワーク、システム環境変数、外部サービスに依存してはならない。

---

## ベンチマークテスト規範

### Criterion.rs 使用規範

**ルール 10.1**：ベンチマークテストは `benches/` ディレクトリに統一して配置し、エントリファイルは `benches/lib.rs` とする。テストテーマ별로ファイルを分割する。

```
benches/
├── lib.rs              # エントリ、criterion_group/criterion_main を定義
├── lang_compare/
│   └── fibonacci.rs    # 言語間比較ベンチマーク
├── parser.rs           # パーサー ベンチマーク
└── codegen.rs          # コード生成 ベンチマーク
```

**ルール 10.2**：各ベンチマーク関数にはモジュールレベルのドキュメントコメント `//!` を含め、テスト目的と測定指標を説明しなければならない。

```rust
//! YaoXiang インタープリタ性能ベンチマーク
//!
//! 測定指標：単一イテレーション所要時間（wall time）
//! ベンチマークライン：Rust ネイティブ実装
```

### コンパイラの最適化防止

**ルール 11.1**：すべてのベンチマークテストの被テスト出力は `criterion::black_box` を使用してコンパイラの最適化による削除を防止する。

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

**ルール 11.2**：ベンチマークテストの入力データは `const` または `lazy_static` としなければならず、`iter` クロージャ内で動的に生成してはならない——さもなければ測定するのはデータ生成 + 被テスト論理の合計時間となる。

### ベンチマークグループ化と命名

**ルール 12.1**：ベンチマークテストの命名形式は `<被テストモジュール>_<シナリオ}` とし、すべて小文字でアンダースコア区切りとする。ユニットテストの命名規則と一致。

**ルール 12.2**：関連するベンチマークは `criterion_group!` で論理的にグループ化しなければならない。すべてのベンチマークを1つのグループに押し込めることは禁止。

```rust
criterion_group!(parser, bench_parse_expr, bench_parse_stmt);
criterion_group!(codegen, bench_codegen_module, bench_codegen_switch);
criterion_main!(parser, codegen);
```

---

## ドキュメンテーションテスト規範

### 使用シナリオ

**ルール 13.1**：すべての `pub` 関数、型、メソッドにはドキュメントコメントに少なくとも1つの実行可能なコード例を含まなければならない。この例は `cargo test --doc` で実行される。

```rust
/// ソースコード文字列を Token シーケンスにトークン化する。
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

**ルール 13.2**：ドキュメンテーションテストのコード例はコンパイルが通るかつ断言が成功しなければならない。`ignore` マークのある例は、その例がコンパイル時エラーを展示する場合を除いて禁止。

```rust
/// ```ignore
/// // コンパイル時エラーを展示——ignore 化してよい
/// let x: int = "string";
/// ```
```

### カバー要件

**ルール 14.1**：ドキュメンテーションテストは API の happy path をカバーすればよい。境界情况和とエラーパスはユニットテストがカバーする。

**ルール 14.2**：ドキュメンテーションテスト内のコード例は簡潔でなければならず——10行を超えてはならない。例により長いコンテキストが必要な場合は、API 設計に問題があることを意味する。

---

## プロパティテスト規範

### 使用シナリオ

**ルール 15.1**：以下のシナリオでは、手書きの複数の境界値ケースではなく、プロパティテスト（proptest または quickcheck）を使用しなければならない：

| シナリオ | 例 |
|------|------|
| パーサー round-trip | `parse(pretty_print(ast)) == ast` |
| シリアライズ/デシリアライズ | `deserialize(serialize(data)) == data` |
| 数学演算の恒等式 | `a + b == b + a` |
| コンパイラの最適化が意味論を変えない | `eval(code) == eval(optimize(code))` |

**ルール 15.2**：プロパティテストは `proptest` を主要なプロパティテストフレームワークとして使用する（`Cargo.toml` の `dev-dependencies` に宣言済み）。

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

**ルール 16.1**：各プロパティテストには明確なプロパティ宣言がなければならず——コメントに検証する不変量を記述する。

```rust
// プロパティ：任意の整数のリテラルが tokenize → tokens_to_string の後に同じ値を生成する
proptest! {
    #[test]
    fn test_int_literal_roundtrip(n in any::<i64>()) {
        let source = n.to_string();
        let tokens = tokenize(&source).unwrap();
        // ...
    }
}
```

**ルール 16.2**：プロパティテストが失敗を発見した場合は、`proptest` のリグレッション・メカニズムを使用しなければならない——失敗した入力を `proptest-regressions/` ディレクトリに追加し、手書きで通常のテストに置き換えてはならない。

---

## カバレッジ要件

### 新規コードのカバレッジ目標

**ルール 17.1**：新規コードのテストカバレッジ要件：

| コードの種類 | 行カバレッジ | 分岐カバレッジ |
|----------|----------|------------|
| コアコンパイラモジュール（frontend/middle/backends） | ≥ 85% | ≥ 80% |
| ツール/補助モジュール（util） | ≥ 75% | ≥ 70% |
| ランタイムモジュール（vm/runtime） | ≥ 80% | ≥ 75% |
| 標準ライブラリ（std） | ≥ 75% | ≥ 70% |
| エラー処理と診断 | ≥ 90% | ≥ 85% |

**ルール 17.2**：エラー処理パス（すべての `Err` 分岐）は100%カバレッジしなければならない。ユーザーが見るエラーメッセージはテストで検証されていなければならない。

### PR レビュー・チェックリスト

**ルール 18.1**：PR 提交前に、著者は以下の項目を自查しなければならない：

- [ ] `cargo test` がすべて通過
- [ ] `cargo test --doc` がすべて通過
- [ ] `cargo bench` にパフォーマンスリグレッションなし（ホットパス変更を伴う場合）
- [ ] 新規コードがカバレッジ目標に適合
- [ ] テスト命名が命名規範に適合
- [ ] 各テストファイルが対応する規範セクションを宣言（ルール 2.1）
- [ ] テスト期待値が規範定義から来ており、「現在のコードの出力」からは来ていない
- [ ] `#[ignore]` マークのあるテストなし（明確な issue 番号コメントがある場合を除く）
- [ ] 不必要な `unwrap()` なし（`expect` またはカスタム panic メッセージを使用すべき）
- [ ] コミットメッセージが `:white_check_mark: test:` タイプを使用
- [ ] **「コード動作が規範に適合しない」ことを理由にテスト期待値を修正していない——修正したのはコードであり、テストではない**

**ルール 18.2**：Reviewer は以下の問題を含む PR を拒否しなければならない：

- happy path テストのみで、エラーパスが不足
- テスト中に `thread::sleep` または実行順序に依存
- 3回以上コピー＆ペーストしたテストコードでヘルパー関数を抽出していない
- テスト名が命名規範に適合しない
- 永続的に `#[ignore]` されたテストが存在
- **テストがコードのエラー動作に迎合**（コードと規範が一致しない時にテストを修正而非コードを修正）
- **テストが対応する規範セクションを宣言していない**（ルール 2.1 参照）
- **テスト期待値がコード出力而非規範から導出されている**（逆算されたテストはテストとしての意味がない）
- テストが「panic しない」ことを検証するだけで具体的な動作を断言していない
- コードバグを露呈する失敗テストを削除した（コードを修正してから Green になるのを見ているのではなく）

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

# ドキュメンテーションテストのみ実行
cargo test --doc

# 特定のテストを実行（名前でフィルタ）
cargo test test_parse_expr

# ベンチマークテストを実行
cargo bench

# テスト出力を表示（デフォルトで stdout は非表示）
cargo test -- --nocapture

# 単一スレッドで実行（並行問題排查）
cargo test -- --test-threads=1

# カバレッジレポートを生成（cargo-llvm-cov が必要）
cargo llvm-cov --html
```

### B. コミットメッセージ・テンプレート

テスト関連コミットは以下のテンプレートに従わなければならない：

```
:white_check_mark: test(<scope>): <简短説明>

<オプション：カバーするシナリオリスト>
```

例：

```
:white_check_mark: test(parser): Pratt パーサーの中置演算子テストを追加

カバーするシナリオ：
- 算術演算子の優先順位（+, -, *, /, %）
- 比較演算子のチェーン（1 < x < 10）
- 論理演算子の短絡
- 代入演算子の右結合
```

### C. 新規テストファイル・チェックリスト

新しいテストモジュールを作成する際は、以下のファイルが含まれることを確認：

```
# src/<module>/ ディレクトリにテストを追加
src/<module>/tests/
├── mod.rs          # モジュールの宣言 + 公共ヘルパー関数
└── <subject>.rs    # テストファイル、被テストソースファイルの命名に従う

# tests/ ディレクトリに統合テストを追加
tests/
├── integration.rs   # 更新：#[path] 宣言を追加
└── integration/
    └── <topic>.rs   # 新規テストファイル
```

### D. 参考資料

- [YaoXiang 言語規範](../../design/language-spec.md) —— **テストの権威あるソース**
- [受理済み RFC](../../design/rfc/accepted/) —— **設計決定の権威あるソース**
- [Rust テストドキュメント](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Criterion.rs ユーザーガイド](https://bheisler.github.io/criterion.rs/book/)
- [proptest ドキュメント](https://docs.rs/proptest/latest/proptest/)
- [プロジェクト・コミット規範](./commit-convention.md)
- [プロジェクト・コントリビューション・ガイド](./contributing.md)

---

> 💡 **覚えておくこと**：テストはコードが「動く」かどうかを検証するのではなく、コードが規範に適合するかを検証する。規範が変われば、テストも規範に従って変わる。コードが間違っていれば、コードを変更し、テストを変更しない。**コードは規範に奉仕し、テストは規範を守る。テストがコード迎合的那一刻、すべて保護を失う。**
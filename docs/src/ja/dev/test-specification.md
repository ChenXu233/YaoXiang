```markdown
---
title: "テスト作成規範"
description: YaoXiangプロジェクトのテスト作成に関する厳格な規範。ユニットテスト、結合テスト、ベンチマークテスト、ドキュメントテスト、プロパティテストの作成標準を定義する
---

# テスト作成規範

本文書は YaoXiang プロジェクトのテスト作成に関する厳格な規範を定義する。すべての貢献者は以下の規則を遵守しなければならず、違反者は Code Review において修正を求められる。

---

## 目次

- [総則](#総則)
- [ユニットテスト規範](#ユニットテスト規範)
- [結合テスト規範](#結合テスト規範)
- [ベンチマークテスト規範](#ベンチマークテスト規範)
- [ドキュメントテスト規範](#ドキュメントテスト規範)
- [プロパティテスト規範](#プロパティテスト規範)
- [カバレッジ要件](#カバレッジ要件)
- [付録](#付録)

---

## 総則

### 適用範囲

本規範は YaoXiang プロジェクトにおけるすべての Rust テストコードに適用される。

| テスト種別 | 配置場所 | フレームワーク |
|----------|------|------|
| ユニットテスト | `src/<module>/tests/` | `#[test]` + `#[cfg(test)]` |
| 結合テスト | `tests/` | `#[test]` |
| ベンチマークテスト | `benches/` | Criterion.rs |
| ドキュメントテスト | API ドキュメントコメント | `cargo test --doc` |
| プロパティテスト | 任意のテスト配置場所 | proptest / quickcheck |

### 中核原則

**原則 0：テストの権威的な出所は規範であり、コードではない。** これは本文書で最も重要な原則である。テストが検証するのは「コードが規範に適合しているか」であり、「コードが現在の実装で動くか」ではない。テストによってコードの挙動が規範と一致していないことが判明したなら、**テストを修正するのではなく、コードを修正すること**。

規範ファイルは以下に配置される：
- `docs/src/design/language-spec.md` —— 言語コア規範
- `docs/src/design/rfc/accepted/` —— 受理済みの RFC 設計文書

各テストファイルの先頭には対応する規範の章を宣言しなければならない（規則 2.1 を参照）。あらゆる開発者が規範文書とテストを照合し、実装の正当性を検証できるはずである。逆に——コードに対応する規範記述がないなら、そのコードは存在すべきではなく、ましてテストされるべきでもない。

```rust
// 🟢 良い例——テストが規範を直接参照し、コードが規範に従っているかを検証する
//! literal テスト — 言語規範 §2.6 に基づく
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

// 🔴 駄目な例——テストが現在のコードの実装挙動に迎合しており、規範を検証していない
#[test]
fn test_literal_1() {
    // このコードが規範のどの節に対応するか不明
    // parse_literal が誤った値を返しても、このテストは「緑」で通過する
    // 関数が panic しないことしか検証していないため
    let result = parse_literal("42");
    assert!(result.is_ok());
}
```

**シナリオ**：テストを書いていて、コードの挙動が規範と一致していないことに気づいた。あなたには 2 つの選択肢がある：
| 誤った対応 | 正しい対応 |
|----------|----------|
| テストを修正して「通過」させる | コードを修正して挙動を規範に合わせる |
| テストに `#[ignore]` を追加する | 直ちにコード実装を修正する |
| テストに特別な条件分岐を追加してコードに迎合する | 分岐を削除し、テストに直接問題を露出させる |

覚えておくこと：**赤信号 = コードが間違っている、テストが間違っているのではない。**（テスト自体にバグがある場合は別問題である。）

**原則 1：テストはドキュメントである。** あらゆる開発者が、追加のコメントや外部ドキュメントなしに、テストを読むだけで被験コードの挙動を理解できるべきである。

```rust
// 🟢 良い例——テスト名が何をテストし、何を期待しているかを明示している
#[test]
fn test_tokenize_empty_input_returns_eof() {
    let tokens = tokenize("").unwrap();
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].kind, TokenKind::Eof));
}

// 🔴 駄目な例——何をテストしているのか誰にも分からない
#[test]
fn test_tokenize_1() {
    let tokens = tokenize("").unwrap();
    assert!(tokens.len() > 0);
}
```

**原則 2：不規則な失敗を一切許容しない。** テストはあらゆる環境で再現可能に実行できなければならない。乱数、システム時刻、スレッドスケジューリング順序に依存するテストは、シード固定または mock による置換を使用しなければならない。

**原則 3：1 つのテストは 1 つの事柄のみをテストする。** テスト名に「と」で複数の挙動を連結する必要があれば、複数のテストに分割する。

```rust
// 🟢 良い例——各テストが 1 つのシナリオのみを検証する
#[test]
fn test_parse_int_positive() { /* ... */ }
#[test]
fn test_parse_int_zero() { /* ... */ }

// 🔴 駄目な例——無関係な内容を 1 つのテストに詰め込んでいる
#[test]
fn test_parser() {
    // tokenize、parse、typecheck、codegen を全部テスト...
}
```

**原則 4：実装ではなく挙動をテストする。** 内部実装のリファクタリングでテストが失敗してはならない。実装コードを 1 行変更して 10 個のテストが失敗するなら、テストの書き方が間違っている。

ただしここに重要な区別がある：**「挙動」の定義は規範から来るもので、現在のコードの現れ方から来るものではない。** コードが挙動を変更した（すなわち規範に合わない新しい挙動）場合、テストは必ず失敗しなければならない。これができないなら、あなたのテストは「コードに迎合するテスト」であり、bug の侵入を許してしまう。

```
規範（language-spec.md / RFC）  ──定義──►  期待挙動  ──駆動──►  テスト
                                           │
現在のコード  ──実装──►  実際の挙動  ──比較──►  テスト結果

実際の挙動 ≠ 期待挙動 の場合：
  テストは必ず失敗（赤信号）  ──►  コードを修正  ──►  テスト通過（緑信号）

実際の挙動 = 期待挙動（だが実装が酷い）の場合：
  テスト通過  ──►  実装をリファクタ  ──►  テストは依然通過  ← これが原則 4 の真の意味
```

**原則 5：フォールバック・互換・特定パターン有効化のためのテストコードを書かない。** テスト環境はあなたが完全に制御できる環境である。テストをスキップするために `#[cfg(not(ci))]` が必要なら、そのテスト設計には根本的な問題がある。

### 用語定義

| 用語 | 定義 |
|------|------|
| ユニットテスト | 単一の関数またはモジュールの挙動をテストし、外部システムに依存しない |
| 結合テスト | 複数のモジュールの連携を、公開 API またはコマンドラインエントリ経由でテストする |
| ベンチマークテスト | コード性能を測定し、性能回帰を検出する |
| ドキュメントテスト | ドキュメントコメントに埋め込まれた実行可能なコード例 |
| プロパティテスト | ランダム入力に基づいて不変条件（property）を検証するテスト |

### コミット規範との関連

すべてのテスト関連コミットは `:white_check_mark: test:` タイプを使用しなければならず、[コミット規範](./commit-convention.md)を参照する。

```
:white_check_mark: test(parser): Pratt パーサの中置式テストを追加
:white_check_mark: test(codegen): switch 文の IR 生成テストを補完
```

---

## ユニットテスト規範

### ファイル構成

**規則 1.1**：ユニットテストの `tests/` ディレクトリは、被験モジュールの `mod.rs` と**同階層**でなければならない。`tests/` は上位に集約せず、階層をまたいでまとめない。

```
src/frontend/core/parser/
├── mod.rs              # #[cfg(test)] mod tests; ——同階層の tests/ を宣言
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

重要な判断基準：**`tests/` を配置したディレクトリの `mod.rs` は、必ず `#[cfg(test)] mod tests;` でそれを宣言しなければならない。**

**規則 1.1 補足：上位への集約を禁止する。** サブディレクトリモジュールのテストは、当該サブディレクトリ自身の `tests/` に配置しなければならず、上の階層の `tests/` に集約してはならない。

| モジュール種別 | テスト配置場所 | 例 |
|----------|----------|------|
| ディレクトリモジュール（`mod.rs` を持つ） | 当該ディレクトリ下の `tests/` | `emitter/tests/`、`codes/tests/` |
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

#### 単一ファイルモジュール vs ディレクトリモジュールのテスト配置規則

**核心的な違い**：モジュールの組織形式がテストの配置場所を決定する。

| モジュール種別 | 判断基準 | テスト配置場所 | 例 |
|----------|----------|----------|------|
| **ディレクトリモジュール** | 独立したディレクトリと `mod.rs` を持つ | 当該ディレクトリ下の `tests/` | `inference/tests/` |
| **単一ファイルモジュール** | `.rs` ファイルのみ、独立ディレクトリなし | 親階層の `tests/` | `overload.rs` → `typecheck/tests/overload.rs` |

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
│   ├── overload.rs                 # overload.rs のテスト（単一ファイルモジュールのテストをここに配置）
│   ├── type_eval.rs                # type_eval.rs のテスト
│   ├── dead_code.rs                # dead_code.rs のテスト
│   ├── spawn_placement.rs          # spawn_placement.rs のテスト
│   ├── signature.rs                # signature.rs のテスト
│   └── types.rs                    # types.rs のテスト
│
├── inference/                      # ディレクトリモジュール（mod.rs を持つ）
│   ├── mod.rs                      # #[cfg(test)] mod tests; ——同階層の tests/ を宣言
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
└── traits/                         # ディレクトリモジュール（mod.rs を持つ）
    ├── mod.rs                      # #[cfg(test)] mod tests; ——同階層の tests/ を宣言
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

**なぜ単一ファイルモジュールのテストを親階層の `tests/` に配置するのか？**

単一ファイルモジュール（例：`overload.rs`）は自身の `mod.rs` を持たないため、`#[cfg(test)] mod tests;` を宣言できない。Rust のモジュールシステムによると、テストファイルは何らかの `mod.rs` で宣言されなければコンパイルされない。したがって、単一ファイルモジュールのテストは親階層の `mod.rs` で宣言され、親階層の `tests/` ディレクトリに配置されるしかない。

**判断フロー**：

```
あるモジュールに遭遇し、テストの配置場所を判断する
│
├── 当該モジュールはディレクトリ（mod.rs を持ち）か？
│   └── はい → 当該ディレクトリ下に tests/ を作成し、当該ディレクトリの mod.rs で宣言
│
├── 当該モジュールは単一ファイル（.rs のみ）か？
│   └── はい → テストを親階層の tests/ ディレクトリに配置し、親階層の mod.rs で宣言
│
└── 不確か？
    └── 独立したディレクトリと mod.rs を持つか確認
```

**よくある誤り**：

```
# ❌ 誤り 1：単一ファイルモジュールのために独立した tests/ ディレクトリを作成
src/frontend/core/typecheck/
├── overload.rs
└── overload/                       # ❌ 単一ファイルモジュールのためにディレクトリを作成すべきでない
    └── tests/
        └── overload.rs

# ❌ 誤り 2：単一ファイルモジュール内で #[cfg(test)] mod tests; を宣言
# overload.rs
#[cfg(test)]                        # ❌ 単一ファイルモジュールではこのように宣言できない
mod tests;                          # overload/tests/ ディレクトリが存在しないため

# ✅ 正しい対応：テストを親階層の tests/ に配置
src/frontend/core/typecheck/
├── overload.rs                     # ソースファイル
└── tests/
    └── overload.rs                 # テストファイル、typecheck/mod.rs で宣言される
```

⚠️ **アンチパターン——以下のように書いてはならない：**

```
# ❌ 誤り：サブモジュールのテストを親階層に集中
src/frontend/core/types/
├── mod.rs              # base と computation のみを宣言すべき
├── base/
│   ├── mod.rs
│   └── var.rs
└── tests/              # ❌ 親階層の tests/ にサブモジュールのテストが含まれている
    ├── mod.rs          # ❌ 強制的に mod base; mod computation; を宣言することになる
    ├── base/           # ❌ この部分は base/tests/ に置くべき
    │   └── var.rs
    └── computation/    # ❌ この部分は computation/tests/ に置くべき
        └── ...
```

```
# ✅ 正しい対応：各モジュールのテストがそれぞれ独立
src/frontend/core/types/
├── mod.rs              # pub mod base; pub mod computation; のみを宣言
├── base/
│   ├── mod.rs          # #[cfg(test)] mod tests; ——同階層の tests/ を宣言
│   ├── var.rs
│   └── tests/
│       ├── mod.rs
│       └── var.rs
└── computation/
    ├── mod.rs          # #[cfg(test)] mod tests; ——同階層の tests/ を宣言
    ├── operations.rs
    └── tests/
        ├── mod.rs
        └── operations.rs
```

**なぜ上位に集約できないのか？** Rust のモジュールシステムは、`#[cfg(test)] mod tests;` が宣言された場所でテストファイルのコンパイルを決定するためである。`types/mod.rs` で `mod tests;` を宣言した場合、`types/tests/` の内容は `types` モジュールのプライベートな内容となる——それは `base` や `computation` の領域に踏み込むべきではない。各モジュールのテストはそのモジュールの内部実装詳細であるべきで、親モジュールのものではない。この規則はモジュールのリファクタリングにも同様に適用される：`types` を `base` と `computation` に分割するとき、テストも分割後のモジュールに従って分割すべきであり、元の場所に残してはならない。**テストディレクトリはソース構造をミラーリングするのではなく、モジュール境界に従う。**

**規則 1.2**：`tests/mod.rs` はモジュールの宣言と re-export のみを行い、テスト関数を置かない。

```rust
//! Parser core tests — src/frontend/core/parser/ をミラーリング
//!
//! ast.rs、parser_state.rs、および式/統合パース用のテスト。

mod ast;
mod error_recovery;
mod expressions;
mod integration;
mod parser_state;
```

**規則 1.3**：各テストファイルは 1 つのソースファイルにのみ対応する。複数のソースモジュールのテストを 1 つのファイルに混在させることは許可されない。

**規則 1.4**：テスト宣言はファイル形式 `mod tests;`（セミコロン付き）を使用し、同階層の `tests/` ディレクトリを指すものとする。**インライン形式 `mod tests { ... }` を使用してテストコードをソースファイル内に直接書くことを禁止する。**

```rust
// ✅ 正しい——ファイル形式での宣言、テストコードは独立したファイルに
// src/frontend/core/parser/mod.rs
#[cfg(test)]
mod tests;

// 🔴 禁止——インライン形式、テストコードがソースファイルに寄生
// src/frontend/core/parser/mod.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        // テストコードはソースファイルに書くべきではない
    }
}
```

**なぜインラインを禁止するのか？**
1. ソースファイルの責務を単一化する：ソースファイルは実装のみ、テストファイルはテストのみ。混在すると、テスト修正時はファイル末尾までスクロールし、実装修正時はテストをスキップする必要がある。
2. モジュール境界を明確にする：`tests/` ディレクトリは物理的な境界であり、どのモジュールにテストがあり、ないか一目瞭然である。
3. リファクタリングの安全性：モジュール分割時、`tests/` ディレクトリも一緒に移動する。インラインテストはソースファイルから手動で剥離する必要がある。
4. コードレビュー：PR diff において、ソース変更とテスト変更が別ファイルとなり、混在しない。

### モジュール宣言規範

**規則 2.1**：すべてのテストファイルの先頭には、モジュールレベルのドキュメントコメント `//!` がなければならず、テストカバーの規範の出所（言語規範の章番号 + RFC 番号）を記述する。テストがいかなる規範の章も参照していないなら、そのコードは規範の根拠を持たない——それは存在すべきではない。

```rust
//! literal テスト — 言語規範 §2.6 に基づく
//!
//! §2.6.1: 整数 Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: 浮動小数点数（小数点と指数を含む）
//! §2.6.3: 文字列（エスケープシーケンス \\nrt'"\\, \\x, \\u{}）
//! RFC-012: F-String 補間
```

**なぜ規範を参照しなければならないのか？** テストの期待値は規範から来るべきであり、「現在のコードの出力」から来るべきではない。ある日コードの出力が変更され、テストがそれに合わせて更新されるなら、テストは何も保護していない。規範に固定されたテストのみが「意図的な breaking change」と「意図しない回帰」を区別できる。

**規則 2.2**：テストモジュールの `use` インポートは具体的な型/関数まで正確に行い、glob インポート `use super::*` を禁止する。

```rust
// 🟢 良い例——正確なインポート
use crate::frontend::core::lexer::{tokenize, TokenKind};
use crate::frontend::core::parser::{ParserState, ParseError};

// 🔴 駄目な例——何をテストしているのか他人に分からない
use super::*;
```

### 命名規範

**規則 3.1**：テスト関数の命名形式は `test_<what>_<scenario>` とし、すべて小文字のアンダースコア区切りとする。

```rust
#[test]
fn test_tokenize_empty_string() { /* ... */ }
#[test]
fn test_parse_int_overflow() { /* ... */ }
#[test]
fn test_typecheck_fn_return_mismatch() { /* ... */ }
```

**規則 3.2**：テスト関数名は自己説明的であるなければならない。関数名を読めば、何をテストし、何を期待しているかが分かること。数字の連番による命名を禁止する。

```rust
// 🟢 良い例
fn test_skip_semicolon_success() { /* ... */ }
fn test_skip_semicolon_failure_when_identifier() { /* ... */ }

// 🔴 駄目な例——何をテストしているのか完全に分からない
fn test_skip_1() { /* ... */ }
fn test_skip_2() { /* ... */ }
```

**規則 3.3**：ヘルパー関数は `test_` 接頭辞を必要とせず、動詞または名詞でその用途を記述する。

```rust
fn parse_expr(source: &str) -> Expr { /* ... */ }
fn tokenize_single(source: &str) -> Token { /* ... */ }
fn setup_parser_with_tokens(tokens: &[Token]) -> ParserState { /* ... */ }
```

### テスト構造規範（Arrange-Act-Assert）

**規則 4.1**：各テスト関数は 3 段構成、すなわち準備（Arrange）→ 実行（Act）→ 検証（Assert）に従わなければならず、段の間は空行で区切る。

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

**規則 4.2**：単純なテスト（単一の呼び出し + 単一の検証）は段のコメントを省略してよいが、論理コード 5 行を超過してはならない。5 行を超えるテストは 3 段を明示的に示さなければならない。

### ヘルパー関数規範

**規則 5.1**：3 回以上繰り返される setup ロジックは、ヘルパー関数として抽出しなければならない。

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

**規則 5.2**：ヘルパー関数内の `unwrap()` / `expect()` は、panic 時に十分なコンテキストを出力しなければならない。テスト関数本体内（`#[test] fn ...`）では直接 `unwrap()` してよい——失敗時に Rust が自動的に行番号を出力するため。しかしヘルパー関数内で失敗した場合、行番号はヘルパー関数の定義箇所を指し、呼び出し時のコンテキストが分からない。

```rust
// 🟢 良い例——ヘルパー関数失敗時にソースコード内容を出力
fn run_ok(source: &str) {
    run(source).unwrap_or_else(|e| panic!("Execution failed:\nSource:\n{}\nError:\n{:?}", source, e));
}

// 🔴 駄目な例——失敗時にどのソースファイルが問題の原因か分からない
fn run_ok(source: &str) {
    run(source).unwrap();
}
```

**規則 5.3**：ヘルパー関数はテストファイルの先頭、`use` インポートの直後に配置する。複数のテストモジュールで共有される場合、`tests/mod.rs` に配置して `pub(crate)` でエクスポートする。

### 検証スタイル

**規則 6.1**：enum 変体のマッチングには `assert!(matches!(...))` を優先的に使用し、`if let` + `panic!` を使用してはならない。

```rust
// 🟢 良い例
assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(42)));

// 🔴 駄目な例
if let TokenKind::IntLiteral(v) = tokens[0].kind {
    assert_eq!(v, 42);
} else {
    panic!("Expected IntLiteral");
}
```

**規則 6.2**：精密な値の比較には `assert_eq!` を使用し、ブール検証には `assert!` を使用する。`assert!(a == b)` を `assert_eq!(a, b)` の代わりに使用することを禁止する。

**規則 6.3**：すべての検証にはカスタムエラーメッセージを付けること。ただし、検証自体が失敗原因を完全に記述している場合は除く。

```rust
// 🟢 良い例——検証失敗時にすばやく問題箇所を特定できる
assert!(
    state.infix_info().is_some(),
    "infix_info should handle '{op}'"
);

// 🟢 良い例——assert_eq! 失敗時は値の差分を自動出力するため、追加メッセージ不要
assert_eq!(error_count, 0);

// 🔴 駄目な例——失敗時に「assertion failed」としか分からない
assert!(state.infix_info().is_some());
```

**規則 6.4**：検証の順序は `assert_eq!(actual, expected)` とし、実際値を前、期待値を後ろにする。

### アンチパターン一覧

以下は禁止される書き方と代替案：

| アンチパターン | 問題点 | 代替案 |
|--------|------|----------|
| `#[cfg(test)] mod tests { ... }` インラインテスト | ソースファイル肥大化、モジュール境界の曖昧化、リファクタリング困難 | テストコードを独立した `tests/` ディレクトリに配置し、`mod tests;` で宣言（規則 1.4 を参照） |
| コードの誤った挙動にテストが迎合する | 規範偏差を隠し、bug を合法化する | 規範に照らしてコードを修正し、テストは変更しない |
| コード出力からテスト期待値を逆算する | テストが「現在の実装の録音機」になる | 規範から期待値を導出する |
| 永続的な `#[ignore]` マーカー | 腐ったテストを隠蔽する | 修正または削除 |
| `println!` デバッグ出力 | テスト出力を汚染する | `assert!` で明確に検証する |
| `thread::sleep` | 不規則な失敗 + 遅延 | 同期機構または mock を使用 |
| テストで実ファイルシステムを操作 | 遅く、再現不可能 | `tempfile` を使用 |
| テスト実行順序への依存 | 不規則な失敗 | 各テストを独立して setup |
| 1 つのテスト関数が論理 30 行を超える | 誰も理解できない | テストを分割するか、ヘルパー関数を使用 |
| ヘルパー関数内で `unwrap()` がコンテキストを出力しない | 問題箇所の特定が困難 | `expect("why")` またはカスタム panic を使用（規則 5.2 を参照） |
| 同じ setup を 3 回以上コピー&ペースト | 修正コストが高い | ヘルパー関数を抽出 |

---

## 結合テスト規範

### テスト構成

**規則 7.1**：結合テストはプロジェクトルートの `tests/` ディレクトリに配置する。エントリファイル `tests/integration.rs` は `#[path]` 属性を使用してサブモジュールを導入する。

```rust
// tests/integration.rs
#[path = "integration/backends.rs"]
mod backends;
#[path = "integration/codegen.rs"]
mod codegen;
#[path = "integration/execution.rs"]
mod execution;
```

**規則 7.2**：各 `tests/integration/*.rs` ファイルは 1 つのテスト主題（コンパイラバックエンド、コード生成、エグゼキュータなど）に対応し、混在させてはならない。

**規則 7.3**：結合テストはプロジェクトの公開 API 経由でテストしなければならない。結合テスト内で `crate::` 内部モジュールを直接参照することを禁止する。`yaoxiang::` 公開パスを使用する。

```rust
// 🟢 良い例——公開 API 経由
use yaoxiang::run;

// 🔴 駄目な例——公開 API 境界を迂回している
use yaoxiang::middle::codegen::bytecode::BytecodeFile;
```

### テストデータ管理

**規則 8.1**：結合テストはインラインソース文字列を優先する。ソースが 30 行を超える場合に限り、外部 fixture ファイル（`tests/fixtures/` に配置）を使用する。

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

**規則 8.2**：fixture ファイルは `.yx` 拡張子で終わり、ファイル名はテストの意図を記述するものとする。

### E2E カバレッジ原則

**規則 9.1**：各言語機能の結合テストは 3 つのパスをカバーしなければならない：

| パス | 説明 |
|------|------|
| Happy path | 合法的な入力が予期された出力を生成する |
| Error path | 非法的な入力が明確なエラーメッセージを生成する（panic ではない） |
| Boundary | 境界値（空入力、最大値、ネスト深度の上限） |

**規則 9.2**：結合テストはネットワーク、システム環境変数、外部サービスに依存してはならない。

---

## ベンチマークテスト規範

### Criterion.rs 使用規範

**規則 10.1**：ベンチマークテストは `benches/` ディレクトリに統一して配置し、エントリファイルは `benches/lib.rs` とする。テスト主題ごとにファイル分割する。

```
benches/
├── lib.rs              # エントリ、criterion_group/criterion_main を定義
├── lang_compare/
│   └── fibonacci.rs    # 言語間比較ベンチマーク
├── parser.rs           # パーサベンチマーク
└── codegen.rs          # コード生成ベンチマーク
```

**規則 10.2**：各ベンチ関数にはモジュールドキュメントコメント `//!` がなければならず、テスト目的と測定指標を記述する。

```rust
//! YaoXiang インタプリタ性能ベンチマークテスト
//!
//! 測定指標：単一反復の所要時間（wall time）
//! ベースライン：Rust ネイティブ実装
```

### コンパイラ最適化の防止

**規則 11.1**：すべてのベンチマークテストの被験出力は、`criterion::black_box` を通じてコンパイラによる最適化の除去を防止しなければならない。

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

**規則 11.2**：ベンチマークテストの入力データは `const` または `lazy_static` でなければならず、`iter` クロージャ内で動的に生成してはならない——そうでないと、データ生成 + 被験ロジックの合計時間を測定することになる。

### ベンチマークグループ化と命名

**規則 12.1**：ベンチマークテストの命名形式は `<被験モジュール>_<シナリオ>` とし、すべて小文字のアンダースコア区切りとする。ユニットテストの命名規則と一致する。

**規則 12.2**：関連するベンチマークを論理的にグループ化するために `criterion_group!` を使用しなければならない。すべてのベンチマークを 1 つのグループに詰め込むことを禁止する。

```rust
criterion_group!(parser, bench_parse_expr, bench_parse_stmt);
criterion_group!(codegen, bench_codegen_module, bench_codegen_switch);
criterion_main!(parser, codegen);
```

---

## ドキュメントテスト規範

### 適用シナリオ

**規則 13.1**：すべての `pub` 関数、型、メソッドは、ドキュメントコメント内に少なくとも 1 つの実行可能なコード例を含まなければならない。このコード例は `cargo test --doc` で実行される。

```rust
/// ソースコード文字列をトークン列に分割する。
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

**規則 13.2**：ドキュメントテストのコード例はコンパイルを通過し、検証に成功しなければならない。コンパイル時エラーを展示する場合を除き、`ignore` マーカーの例を含んではならない。

```rust
/// ```ignore
/// // コンパイル時エラーの展示——ignore 可
/// let x: int = "string";
/// ```
```

### カバレッジ要件

**規則 14.1**：ドキュメントテストは API の happy path をカバーすればよい。境界ケースやエラーパスはユニットテストでカバーする。

**規則 14.2**：ドキュメントテスト内のサンプルコードは簡潔でなければならない——10 行以内。サンプルにより長いコンテキストが必要なら、API 設計に問題があることを示している。

---

## プロパティテスト規範

### 適用シナリオ

**規則 15.1**：以下のシナリオでは、手書きで複数の境界値ケースを書くのではなく、プロパティテスト（proptest または quickcheck）を使用しなければならない：

| シナリオ | 例 |
|------|------|
| パーサの round-trip | `parse(pretty_print(ast)) == ast` |
| シリアライズ/デシリアライズ | `deserialize(serialize(data)) == data` |
| 数学的演算の恒等式 | `a + b == b + a` |
| コンパイラ最適化が意味論を変えない | `eval(code) == eval(optimize(code))` |

**規則 15.2**：プロパティテストは、主要なプロパティテストフレームワークとして `proptest` を使用する（既に `Cargo.toml` の `dev-dependencies` で宣言済み）。

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

**規則 16.1**：各プロパティテストには明確なプロパティ宣言がなければならない——コメントに検証する不変条件を記述する。

```rust
// プロパティ：任意の整数 literal は tokenize → tokens_to_string 後に同じ値を生成する
proptest! {
    #[test]
    fn test_int_literal_roundtrip(n in any::<i64>()) {
        let source = n.to_string();
        let tokens = tokenize(&source).unwrap();
        // ...
    }
}
```

**規則 16.2**：プロパティテストで失敗が発見された場合、`proptest` のリグレッション機構を使用しなければならない——失敗した入力を `proptest-regressions/` ディレクトリに追加し、代わりに通常のテストを手書きしない。

---

## カバレッジ要件

### 新規コードのカバレッジ目標

**規則 17.1**：新規コードのテストカバレッジ要件：

| コード種別 | 行カバレッジ | 分岐カバレッジ |
|----------|----------|----------|
| 中核コンパイラモジュール（frontend/middle/backends） | ≥ 85% | ≥ 80% |
| ユーティリティ/補助モジュール（util） | ≥ 75% | ≥ 70% |
| ランタイムモジュール（vm/runtime） | ≥ 80% | ≥ 75% |
| 標準ライブラリ（std） | ≥ 75% | ≥ 70% |
| エラー処理と診断 | ≥ 90% | ≥ 85% |

**規則 17.2**：エラーハンドリングパス（すべての `Err` 分岐）は 100% カバーしなければならない。ユーザーが見ることになるエラーメッセージは必ずテストで検証済みでなければならない。

### PR レビューチェックリスト

**規則 18.1**：PR を提出する前に、著者は以下の項目を自己点検しなければならない：

- [ ] `cargo test` がすべて通過する
- [ ] `cargo test --doc` がすべて通過する
- [ ] `cargo bench` で性能回帰がない（ホットパスの変更が関わる場合）
- [ ] 新規コードがカバレッジ目標に適合する
- [ ] テスト命名が命名規範に適合する
- [ ] 各テストファイルが対応する規範の章を宣言している（規則 2.1）
- [ ] テスト期待値が「現在のコードの出力」ではなく規範定義から来ている
- [ ] `#[ignore]` マーカー付きのテストがない（明確な issue 番号のコメントがある場合を除く）
- [ ] 不必要な `unwrap()` がない（`expect` またはカスタム panic メッセージを使用すべき）
- [ ] コミットメッセージが `:white_check_mark: test:` タイプを使用している
- [ ] **「コードの挙動が規範と一致しない」ことを理由にテスト期待値を変更していない——変更するのはコードであり、テストではない**
- [ ] **インラインテストがない**（`#[cfg(test)] mod tests { ... }` は `mod tests;` + 独立ファイルに変更すること、規則 1.4 を参照）

**規則 18.2**：Reviewer は以下の問題を含む PR を必ず拒否しなければならない：

- happy path テストのみで、エラーパスのテストが不足している
- テスト内に `thread::sleep` がある、または実行順序に依存している
- コピー&ペーストされたテストコードが 3 回を超えており、ヘルパー関数が抽出されていない
- テスト名が命名規範に適合しない
- 永続的な `#[ignore]` のテストが存在する
- **テストがコードの誤った挙動に迎合している**（コードと規範が一致しないときにコードを修正せずテストを修正する）
- **テストが対応する規範の章を宣言していない**（規則 2.1 を参照）
- **テスト期待値がコード出力から来ており、規範定義から来ていない**（逆算されたテストはテストしないのと同じ）
- **インラインテストが存在する**（`#[cfg(test)] mod tests { ... }` であり、`mod tests;` + 独立ファイルではない、規則 1.4 を参照）
- テストが「panic しない」ことのみを検証しており、具体的な挙動を検証していない
- コードの bug を露出する失敗テストを削除した（コードを修正してから緑になるのを見るのではなく）

---

## 付録

### A. テストコマンド早見表

```bash
# すべてのテストを実行
cargo test

# ユニットテストのみを実行
cargo test --lib

# 結合テストのみを実行
cargo test --test integration

# ドキュメントテストのみを実行
cargo test --doc

# 特定のテストを実行（名前でフィルタ）
cargo test test_parse_expr

# ベンチマークテストを実行
cargo bench

# テスト出力を表示（デフォルトでは stdout は非表示）
cargo test -- --nocapture

# シングルスレッドで実行（並行問題の原因調査）
cargo test -- --test-threads=1

# カバレッジレポートを生成（cargo-llvm-cov が必要）
cargo llvm-cov --html
```

### B. コミットメッセージテンプレート

テスト関連コミットは以下のテンプレートに従わなければならない：

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
- 論理演算子の短絡評価
- 代入演算子の右結合
```

### C. 新規テストファイル一覧

新しいテストモジュールを作成する際、以下のファイルが含まれることを確認する：

```
# src/<module>/ ディレクトリ下に新規テストを追加
src/<module>/tests/
├── mod.rs          # モジュール宣言 + 共通ヘルパー関数
└── <subject>.rs    # テストファイル、被験ソースファイルの命名に対応する

# tests/ ディレクトリ下に結合テストを追加
tests/
├── integration.rs   # 更新：#[path] 宣言を追加
└── integration/
    └── <topic>.rs   # 新規テストファイル
```

### D. 参考資料

- [YaoXiang 言語規範](../../design/language-spec.md) —— **テストの権威的な出所**
- [受理済みの RFC](../../design/rfc/accepted/) —— **設計決定の権威的な出所**
- [Rust テストドキュメント](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Criterion.rs ユーザーガイド](https://bheisler.github.io/criterion.rs/book/)
- [proptest ドキュメント](https://docs.rs/proptest/latest/proptest/)
- [プロジェクトコミット規範](./commit-convention.md)
- [プロジェクト貢献ガイド](./contributing.md)

---

> 💡 **覚えておくこと**：テストは「あなたのコードが動くか」を検証するのではない——「あなたのコードが規範に適合しているか」を検証する。規範が変われば、テストは規範に従って変わる。コードが間違っていれば、コードを修正し、テストを修正しない。**コードは規範に仕え、テストは規範を守る。テストがコードに迎合した瞬間、あなたはすべての保護を失う。**
```
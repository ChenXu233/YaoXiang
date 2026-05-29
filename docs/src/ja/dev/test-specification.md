---
title: "テスト記述規範"
description: YaoXiangプロジェクトにおけるテスト記述の厳格な規範。ユニットテスト、統合テスト、ベンチマークテスト、ドキュメンテーションテスト、プロパティテストの記述基準を定義する
---

# テスト記述規範

本文書はYaoXiangプロジェクトのテスト記述に関する厳格な規範を定義する。全貢献者は以下の規則に従う必要があり、違反者はコードレビューで修正を求められる。

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

本規範はYaoXiangプロジェクトにおけるすべてのRustテストコードに適用される：

| テストタイプ | 位置 | フレームワーク |
|----------|------|------|
| ユニットテスト | `src/<module>/tests/` | `#[test]` + `#[cfg(test)]` |
| 統合テスト | `tests/` | `#[test]` |
| ベンチマークテスト | `benches/` | Criterion.rs |
| ドキュメンテーションテスト | APIドキュメントコメント | `cargo test --doc` |
| プロパティテスト | 任意のテスト位置 | proptest / quickcheck |

### 基本原則

**原則0：テストの権威ある水源は規範であり、コードではない。** これは本文書で最も重要な原則である。テストはコードが規範に従っているかを検証するものであり、コードが「現在の実装で動作している」かを検証するものではない。テストがコードの動作と規範の間に不一致を発見した場合、**コードを修正し、テストを修正しない。**

規範ファイルの位置：
- `docs/src/design/language-spec.md` —— 言語コア規範
- `docs/src/design/rfc/accepted/` —— 採用されたRFC設計文書

各テストファイルの先頭には対応する規範セクションを宣言する必要がある（規則2.1参照）。すべての開発者は規範文書を持ってテストと照合し、実装の正しさを検証できるはずである。逆に言えば——もしあるコードに対応する規範記述が存在しないなら、そのコードは存在すべきではなく、テストされるべきでもない。

```rust
// 🟢 良い——テストは規範を直接参照し、コードが規範に従っているかを検証する
//! 字句テスト — 言語規範 §2.6に基づく
//!
//! §2.6.1: 整数 Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: 浮動小数点数（小数点と指数を含む）
//! §2.6.3: 文字列（エスケープシーケンス \\nrt'"\\, \\x, \\u{}）
//! RFC-012: F-String補間

#[test]
fn test_decimal_literal_parsing() {
    // 規範 §2.6.1: Decimal ::= [0-9][0-9_]*
    let result = parse_literal("42").unwrap();
    assert_eq!(result, Literal::Int(42));
}

// 🔴 悪い——テストが現在のコードの実装動作に合わせるものであり、規範を検証していない
#[test]
fn test_literal_1() {
    // このコードが規範のどの節に対応するか分からない
    // parse_literalが誤った値を返した場合、このテストは「緑信号通過」になる
    // 関数panicしないことだけを検証しているため
    let result = parse_literal("42");
    assert!(result.is_ok());
}
```

**シナリオ**：あなたはテストを書き、コードの動作が規範不符であることを発見した。2つの選択肢がある：
| 誤った做法 | 正しい做法 |
|----------|----------|
| テストを「通過するように」修正する | コードを修正し、動作を規範に従わせる |
| テストに `#[ignore]` を追加する | 直ちにコード実装を修正する |
| テストにコードに合わせた特殊条件分岐を追加する | 分岐を削除し、テストに直接問題を暴露させる |

覚えておくべきこと：**赤信号 = コードが間違っている，而非テストが間違っている。**（ただし、テスト自体にバグがある場合を除く。それは別の話である。）

**原則1：テストはドキュメントである。** すべての開発者はテストを読むことで被テストコードの動作を理解できるはずであり、追加のコメントや外部ドキュメントを必要としない。

```rust
// 🟢 良い——テスト名が何 tested what、期待値を説明している
#[test]
fn test_tokenize_empty_input_returns_eof() {
    let tokens = tokenize("").unwrap();
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].kind, TokenKind::Eof));
}

// 🔴 悪い——何 tested whatが誰にも分からない
#[test]
fn test_tokenize_1() {
    let tokens = tokenize("").unwrap();
    assert!(tokens.len() > 0);
}
```

**原則2：ランダム失敗はゼロ容忍。** テストは任意の環境で再現可能な実行が可能である必要がある。乱数、システム時刻、スレッドスケジューリング順序に依存するテストは、シード固定またはmockを使用する必要がある。

**原則3：一つのテストは一つのことだけをtestする。** テスト名が「と」で複数の動作を接続する必要がある場合、複数のテストに分割する。

```rust
// 🟢 良い——各テストは1つのシナリオのみ検証する
#[test]
fn test_parse_int_positive() { /* ... */ }
#[test]
fn test_parse_int_zero() { /* ... */ }

// 🔴 悪い——一つのテストに無関係な内容が詰め込まれている
#[test]
fn test_parser() {
    // tokenizeをtestし、parseをtestし、typecheckをtestし、codegenをtestし...
}
```

**原則4：動作をtestし、実装をtestしない。** 内部実装のリファクタリングはテスト失敗を引き起こすべきではない。一行の実装コードを変えて10個のテストが落ちたなら、テストの記述が間違っている。

しかしここに重要な区別がある：**「動作」の定義は規範から来ており、現在のコードの動作から来ているのではない。** コードが動作を変えた（即ち規範不符の新しい動作が生じた場合）、テストは失敗しなければならない。これ做不到の場合、あなたのテストは「コードに合わせているテスト」である——それはバグを侵入させる。

```
規範（language-spec.md / RFC）  ──定義──►  期待動作  ──駆動──►  テスト
                                           │
現在のコード  ──実装──►  実際動作  ──対比──►  テスト結果

実際動作 ≠ 期待動作 の場合：
  テストは失敗しなければならない（赤信号）  ──►  コードを修正  ──►  テスト通過（緑信号）
  
実際動作 = 期待動作 の場合（しかし実装がひどい）：
  テスト通過  ──►  実装をリファクタリング  ──►  テストもなお通過  ← これが原則4の意味
```

**原則5：フォールバック/互換性/特定パターンが有効なテストコードを書かない。** テスト環境はあなたが完全に制御できる環境である。テストをスキップするために `#[cfg(not(ci))]` が必要な場合、そのテスト設計には根本的な問題がある。

### 用語定義

| 用語 | 定義 |
|------|------|
| ユニットテスト | 単一の関数やモジュールの動作をtestし、外部システムに依存しない |
| 統合テスト | 複数のモジュール協調をtestし、公共APIまたはコマンドラインエントリポイント経由 |
| ベンチマークテスト | コードパフォーマンスを測定し、パフォーマンス回帰を検出する |
| ドキュメンテーションテスト | ドキュメントコメントに埋め込まれた実行可能なコード例 |
| プロパティテスト | ランダム入力に基づいて不変量（property）を検証するテスト |

### コミット規範との関連

すべてのテスト関連コミットは `:white_check_mark: test:` タイプを使用する必要があり、[コミット規範](./commit-convention.md)を参照。

```
:white_check_mark: test(parser): Prattパーサーの前置式テストを追加
:white_check_mark: test(codegen): switch文のIR生成テストを補完
```

---

## ユニットテスト規範

### ファイル組織

**規則1.1**：ユニットテストの `tests/` ディレクトリは被テストモジュールの `mod.rs` と**同レベル**にある必要がある。`tests/` は上位に集約されず、跨がって集計されない。

```
src/frontend/core/parser/
├── mod.rs              # #[cfg(test)] mod tests; ——同レベルのtests/を宣言
├── ast.rs
├── pratt/
│   ├── mod.rs          # #[cfg(test)] mod tests; ——pratt独自のテスト
│   └── tests/
│       ├── mod.rs
│       ├── led.rs
│       ├── nud.rs
│       └── precedence.rs
└── tests/              # parserモジュールレベルのテスト（prattサブモジュールの内容は含まない）
    ├── mod.rs
    ├── ast.rs
    ├── expressions.rs
    ├── error_recovery.rs
    └── parser_state.rs
```

重要な判断基準：**`tests/` を配置するディレクトリに応じて、そのディレクトリの `mod.rs` は `#[cfg(test)] mod tests;` で宣言する必要がある。**

#### 単一ファイルモジュール vs ディレクトリモジュールのテスト配置規則

**根本的な違い**：モジュールの組織形式がテストの配置位置を決定する。

| モジュールタイプ | 判断基準 | テスト位置 | 例 |
|----------|----------|----------|------|
| **ディレクトリモジュール** | 独立ディレクトリと `mod.rs` がある | そのディレクトリ下の `tests/` | `inference/tests/` |
| **単一ファイルモジュール** | `.rs` ファイルのみ、独立ディレクトリなし | 親モジュールの `tests/` | `overload.rs` → `typecheck/tests/overload.rs` |

**詳細説明**：

```
src/frontend/core/typecheck/
├── mod.rs                          # typecheckモジュールのmod.rs
├── checker.rs                      # 単一ファイルモジュール
├── environment.rs                  # 単一ファイルモジュール
├── overload.rs                     # 単一ファイルモジュール
├── type_eval.rs                    # 単一ファイルモジュール
├── dead_code.rs                    # 単一ファイルモジュール
├── spawn_placement.rs              # 単一ファイルモジュール
├── signature.rs                    # 単一ファイルモジュール
├── types.rs                        # 単一ファイルモジュール
│
├── tests/                          # ✅ typecheckのテストディレクトリ
│   ├── mod.rs                      # 単一ファイルモジュールのテストを宣言
│   ├── checker.rs                  # checker.rsのテスト
│   ├── environment.rs              # environment.rsのテスト
│   ├── overload.rs                 # overload.rsのテスト（単一ファイルモジュールのテストはこちら）
│   ├── type_eval.rs                # type_eval.rsのテスト
│   ├── dead_code.rs                # dead_code.rsのテスト
│   ├── spawn_placement.rs          # spawn_placement.rsのテスト
│   ├── signature.rs                # signature.rsのテスト
│   └── types.rs                    # types.rsのテスト
│
├── inference/                      # ディレクトリモジュール（mod.rsあり）
│   ├── mod.rs                      # #[cfg(test)] mod tests; ——同レベルのtests/を宣言
│   ├── expressions.rs
│   ├── statements.rs
│   ├── patterns.rs
│   ├── bounds.rs
│   ├── subtyping.rs
│   ├── generics.rs
│   ├── compatibility.rs
│   ├── scope.rs
│   ├── assignment.rs
│   └── tests/                      # ✅ inferenceのテストディレクトリ
│       ├── mod.rs
│       ├── expressions.rs          # expressions.rsのテスト
│       ├── statements.rs           # statements.rsのテスト
│       └── ...
│
└── traits/                         # ディレクトリモジュール（mod.rsあり）
    ├── mod.rs                      # #[cfg(test)] mod tests; ——同レベルのtests/を宣言
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
    └── tests/                      # ✅ traitsのテストディレクトリ
        ├── mod.rs
        ├── solver.rs               # solver.rsのテスト
        ├── impl_check.rs           # impl_check.rsのテスト
        └── ...
```

**なぜ単一ファイルモジュールのテストを親級 `tests/` に配置するのか？**

単一ファイルモジュール（例：`overload.rs`）には自身の `mod.rs` がなく、`#[cfg(test)] mod tests;` を宣言できない。Rustモジュールシステムにより、テストファイルはある `mod.rs` によって宣言される必要がある。したがって、単一ファイルモジュールのテストは親モジュール `mod.rs` によって宣言され、親の `tests/` ディレクトリに配置される。

**判断フロー**：

```
モジュールに遭遇した時、テストはどこに配置するか？
│
├── そのモジュールはディレクトリか（mod.rsあり）？
│   └── はい → そのディレクトリ下にtests/を作成し、そのディレクトリmod.rsで宣言
│
├── そのモジュールは単一ファイルか（.rsファイルのみ）？
│   └── はい → テストは親レベルのtests/ディレクトリに配置し、親のmod.rsで宣言
│
└── 不確実？
    └── 独立ディレクトリとmod.rsがあるかを確認
```

**よくある間違い**：

```
# ❌ 誤り1：単一ファイルモジュール用に独立tests/ディレクトリを作成
src/frontend/core/typecheck/
├── overload.rs
└── overload/                       # ❌ 単一ファイルモジュール用にディレクトリを作成しない
    └── tests/
        └── overload.rs

# ❌ 誤り2：単一ファイルモジュール内で #[cfg(test)] mod tests; を宣言
# overload.rs
#[cfg(test)]                        # ❌ 単一ファイルモジュールでは 이렇게宣言できない
mod tests;                          # overload/tests/ディレクトリがないため

# ✅ 正しい做法：テストは親レベルtests/に配置
src/frontend/core/typecheck/
├── overload.rs                     # ソースファイル
└── tests/
    └── overload.rs                 # テストファイル、typecheck/mod.rsで宣言
```

⚠️ **アンチパターン—— 이렇게書かない：** 

```
# ❌ 誤り：サブモジュールのテストを親レベルに集中させる
src/frontend/core/types/
├── mod.rs              # 本来はbaseとcomputationのみ宣言すべき
├── base/
│   ├── mod.rs
│   └── var.rs
└── tests/              # ❌ 親レベルtests/にサブモジュールのテストが含まれている
    ├── mod.rs          # ❌ mod base; mod computation; の宣言を強いられる
    ├── base/           # ❌ この部分はbase/tests/に配置すべき
    │   └── var.rs
    └── computation/    # ❌ この部分はcomputation/tests/に配置すべき
        └── ...
```

```
# ✅ 正しい做法：各モジュールのテストは各自独立
src/frontend/core/types/
├── mod.rs              # pub mod base; pub mod computation; のみ宣言
├── base/
│   ├── mod.rs          # #[cfg(test)] mod tests; ——同レベルのtests/を宣言
│   ├── var.rs
│   └── tests/
│       ├── mod.rs
│       └── var.rs
└── computation/
    ├── mod.rs          # #[cfg(test)] mod tests; ——同レベルのtests/を宣言
    ├── operations.rs
    └── tests/
        ├── mod.rs
        └── operations.rs
```

**なぜ上位集約できないのか？** Rustのモジュールシステムでは `#[cfg(test)] mod tests;` は宣言位置でテストファイルのコンパイルを決定する。`types/mod.rs` が `mod tests;` を宣言すると、`types/tests/` の内容は `types` モジュールのプライベート内容になる——それは `base` や `computation` の領域に跨がるべきではない。各モジュールのテストはそのモジュールの内部実装の詳細であり、親モジュールのではない。この規則はモジュールのリファクタリングにも適用される：`types` を `base` と `computation` に分割する時、テストも分割後のモジュールに従うべきであり、その場に留まるべきではない。**テストディレクトリはソースコード構造を反映せず、モジュール境界に従う。**

**規則1.2**：`tests/mod.rs` はモジュールの宣言とre-exportを担当し、テスト関数は配置しない。

```rust
//! Parser core tests — src/frontend/core/parser/を反映
//!
//! ast.rs、parser_state.rs、式/統合パース用のテスト。

mod ast;
mod error_recovery;
mod expressions;
mod integration;
mod parser_state;
```

**規則1.3**：各テストファイルは1つのソースファイルのみに対応する。複数のソースモジュールテストを1つのファイルに混在させない。

**規則1.4**：`#[cfg(test)]` は2箇所のみ出現可能——`lib.rs` で `mod tests` を宣言するか、被テストソースファイル内でインライン宣言 `#[cfg(test)] mod tests;`。他での使用は禁止。

```rust
// src/frontend/core/parser/mod.rs または lib.rs
#[cfg(test)]
mod tests;
```

### モジュール宣言規範

**規則2.1**：すべてのテストファイルの先頭にモジュールレベルのドキュメントコメント `//!` が必要であり、テストがカバーする規範水源（言語規範セクション番号 + RFC番号）を説明する。某个テストがどの規範セクションも参照していない場合、そのコードには規範根拠がない——存在すべきではなく、テストされるべきでもない。

```rust
//! 字句テスト — 言語規範 §2.6に基づく
//!
//! §2.6.1: 整数 Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: 浮動小数点数（小数点と指数を含む）
//! §2.6.3: 文字列（エスケープシーケンス \\nrt'"\\, \\x, \\u{}）
//! RFC-012: F-String補間
```

**なぜ規範を参照する必要があるのか？** テストの期待値は規範から来ており、「現在のコードの出力」から来るべきではない。いつかコードが 出力を変え、テストも随之更新するなら、テストは何も保護していない。規範に固定されたテストだけが「意図的なbreaking change」と「意図しない回帰」を区別できる。

**規則2.2**：テストモジュールの `use` インポートは具体的な型/関数に精密である必要があり、globインポート `use super::*;` は禁止。

```rust
// 🟢 良い——精密インポート
use crate::frontend::core::lexer::{tokenize, TokenKind};
use crate::frontend::core::parser::{ParserState, ParseError};

// 🔴 悪い——何をテストしているのか分からない
use super::*;
```

### 命名規範

**規則3.1**：テスト関数名のフォーマットは `test_<what>_<scenario>`、全小文字アンダースコア区切り。

```rust
#[test]
fn test_tokenize_empty_string() { /* ... */ }
#[test]
fn test_parse_int_overflow() { /* ... */ }
#[test]
fn test_typecheck_fn_return_mismatch() { /* ... */ }
```

**規則3.2**：テスト関数名は自己説明的である必要がある。関数名读完就知道何をテストし、何を期待するか。数字シーケンス命名は禁止。

```rust
// 🟢 良い
fn test_skip_semicolon_success() { /* ... */ }
fn test_skip_semicolon_failure_when_identifier() { /* ... */ }

// 🔴 悪い——何をテストしているのか完全に分からない
fn test_skip_1() { /* ... */ }
fn test_skip_2() { /* ... */ }
```

**規則3.3**：ヘルパー関数には `test_` 接頭辞は不要であり、動詞または名詞で用途を説明するべき。

```rust
fn parse_expr(source: &str) -> Expr { /* ... */ }
fn tokenize_single(source: &str) -> Token { /* ... */ }
fn setup_parser_with_tokens(tokens: &[Token]) -> ParserState { /* ... */ }
```

### テスト構造規範 (Arrange-Act-Assert)

**規則4.1**：各テスト関数は3段階構造に従う必要がある：準備（Arrange）→実行（Act）→アサート（Assert）、3段階の間は空行 区切り。

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

**規則4.2**：単純なテスト（単一呼び出し + 単一アサート）は分段コメントを書かなくても良いが、5行以上のロジックコードは超えてはならない。5行を超えるテストは3段階を明示的に標示する必要がある。

### ヘルパー関数規範

**規則5.1**：3回以上重複出現するsetupロジックはヘルパー関数として抽出する必要がある。

```rust
// 🟢 良い——公共setupを抽出
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

**規則5.2**：ヘルパー関数の `unwrap()` / `expect()` はpanic時に十分なコンテキストを出力する必要がある。テスト関数本体（`#[test] fn ...`）では直接 `unwrap()` して良い——失敗時、Rustは自動的に行番号を出力する；しかしヘルパー関数内で失敗した時、行番号はヘルパー関数定義位置を指し、呼び出し時のコンテキストが見えない。

```rust
// 🟢 良い——ヘルパー関数失敗時にソースコード内容を表示
fn run_ok(source: &str) {
    run(source).unwrap_or_else(|e| panic!("Execution failed:\nSource:\n{}\nError:\n{:?}", source, e));
}

// 🔴 悪い——失敗時にどのソースファイルの問題か分からない
fn run_ok(source: &str) {
    run(source).unwrap();
}
```

**規則5.3**：ヘルパー関数はテストファイル先頭、`use` インポートの直後に配置する必要がある。複数のテストモジュールで共有される場合、`tests/mod.rs` に配置し `pub(crate)` エクスポートする。

### アサートスタイル

**規則6.1**：列挙型variantマッチングは `assert!(matches!(...))` の使用を優先し、`if let` + `panic!` は使用禁止。

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

**規則6.2**：精密値比較は `assert_eq!` を使用し、ブール値アサートは `assert!` を使用する。`assert!(a == b)` を `assert_eq!(a, b)` の代わりに使用することは禁止。

**規則6.3**：すべてのアサートにはカスタムエラーメッセージが必要である。ただし、アサート自体が失敗理由を完全に記述している場合は除く。

```rust
// 🟢 良い——アサート失敗時にすぐに位置特定可能
assert!(
    state.infix_info().is_some(),
    "infix_info should handle '{op}'"
);

// 🟢 良い——assert_eq!失敗時に自動的に値差分を出力、追加メッセージ不要
assert_eq!(error_count, 0);

// 🔴 悪い——失敗時に"assertion failed"としか分からない
assert!(state.infix_info().is_some());
```

**規則6.4**：アサート順序は `assert_eq!(actual, expected)` であり、実際の値在前、期待値在后。

### アンチパターンチェックリスト

以下は禁止の記述法とその代替方案：

| アンチパターン | 問題 | 代替方案 |
|--------|------|------|
| テストがコードの誤動作に合わせている | 規範偏差を隠蔽し、bugを合法化 | 規範に照らしてコードを修正、テストは不变 |
| コード出力からテスト期待値を逆算 | テストが「現在実装の録音機」になる | 規範から期待値を導出 |
| `#[ignore]` 永久マーク | 腐ったテストを隠蔽 | 修復または削除 |
| `println!` デバッグ出力 | テスト出力を汚染 | `assert!` で明確アサート |
| `thread::sleep` | ランダム失敗 + 遅延 | 同期機構またはmockを使用 |
| テストで реальファイルシステムを操作 | 遅く再現不可 | `tempfile` を使用 |
| テスト実行順序に依存 | ランダム失敗 | 各テスト独立setup |
| 1つのテスト関数が30行以上のロジック | 理解不能 | テスト分割またはヘルパー関数使用 |
| ヘルパー関数の `unwrap()` がコンテキストを報告しない | 位置特定困難 | `expect("why")` またはカスタムpanicを使用（規則5.2参照） |
| copy-pasteで3回以上同じsetup | 修正コスト高い | ヘルパー関数を抽出 |

---

## 統合テスト規範

### テスト組織

**規則7.1**：統合テストはプロジェクトルートディレクトリの `tests/` ディレクトリに配置する。エントリファイル `tests/integration.rs` は `#[path]` 属性を使用してサブモジュールを導入する。

```rust
// tests/integration.rs
#[path = "integration/backends.rs"]
mod backends;
#[path = "integration/codegen.rs"]
mod codegen;
#[path = "integration/execution.rs"]
mod execution;
```

**規則7.2**：各 `tests/integration/*.rs` ファイルは1つのテストテーマ（コンパイラバックエンド、コード生成、エグゼキュータなど）に対応し、混在禁止。

**規則7.3**：統合テストはプロジェクトの公共API経由でテストする必要がある。統合テストで `crate::` 内部モジュールを直接参照することは禁止。`yaoxiang::` 公共パスを使用。

```rust
// 🟢 良い——公共API経由
use yaoxiang::run;

// 🔴 悪い——公共API境界をバイパス
use yaoxiang::middle::codegen::bytecode::BytecodeFile;
```

### テストデータ管理

**規則8.1**：統合テストはソースコード文字列をインライン使用を優先する。ソースコードが30行を超える場合のみ、外部fixtureファイル（`tests/fixtures/` 配下）を使用。

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

**規則8.2**：fixtureファイルは `.yx` 拡張子で終わる必要があり、ファイル名でテスト意図を説明する。

### E2E カバレッジ原則

**規則9.1**：各言語特性の統合テストは3つのパスをカバーする必要がある：

| パス | 説明 |
|------|------|
| Happy path | 正しい入力が予想出力を生成 |
| Error path | 正しくない入力が明確なエラーメッセージを生成（非panic） |
| Boundary | 境界値（空入力、最大値、ネスト深度上限） |

**規則9.2**：統合テストはネットワーク、システム環境変数、外部サービスに依存禁止。

---

## ベンチマークテスト規範

### Criterion.rs 使用規範

**規則10.1**：ベンチマークテストは `benches/` ディレクトリに統一配置し、エントリファイルは `benches/lib.rs`。テストテーマ別にファイルを分割。

```
benches/
├── lib.rs              # エントリ、criterion_group/criterion_mainを定義
├── lang_compare/
│   └── fibonacci.rs    # 跨言語比較ベンチマーク
├── parser.rs           # パーサーバンクマーク
└── codegen.rs          # コード生成ベンチマーク
```

**規則10.2**：各ベンチマーク関数はテスト目的と測定指標を説明するモジュールドキュメントコメント `//!` を含める必要がある。

```rust
//! YaoXiangインタープリタパフォーマンステスト
//!
//! 測定指標：単一イテレーション所要時間（wall time）
//! ベンチマークライン：Rustネイティブ実装
```

### コンパイラ最適化防止

**規則11.1**：すべてのベンチマークテストの被テスト出力は `criterion::black_box` を使用してコンパイラ最適化除去を防止する必要がある。

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

**規則11.2**：ベンチマークテストの入力データは `const` または `lazy_static` である必要があり、`iter` クロージャ内で動的に生成禁止——さもなくば測定するのはデータ生成 + 被テストロジックの合計時間になる。

### ベンチマークグループ化と命名

**規則12.1**：ベンチマークテスト名のフォーマットは `<被テストモジュール>_<シナリオ>`、全小文字アンダースコア区切り。ユニットテスト命名規則と一致。

**規則12.2**：関連ベンチマークは `criterion_group!` で論理グループ化する必要がある。すべてのベンチマークを1つのグループに押し込むことは禁止。

```rust
criterion_group!(parser, bench_parse_expr, bench_parse_stmt);
criterion_group!(codegen, bench_codegen_module, bench_codegen_switch);
criterion_main!(parser, codegen);
```

---

## ドキュメンテーションテスト規範

### 使用シナリオ

**規則13.1**：すべての `pub` 関数、型、メソッドはドキュメントコメントに少なくとも1つの実行可能コード例を含める必要がある。この例は `cargo test --doc` で実行される。

```rust
/// ソースコード文字列をToken列に字句解析する。
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

**規則13.2**：ドキュメンテーションテストのコード例はコンパイル成功かつアサート成功が必要である。`ignore` マークのある例を含むことは禁止、ただしその例がコンパイル時エラーを展示する場合は除く。

```rust
/// ```ignore
/// // コンパイル時エラーを展示——ignore可
/// let x: int = "string";
/// ```
```

### カバレッジ要件

**規則14.1**：ドキュメンテーションテストはAPIのhappy pathのみカバレッジすれば良い。境界ケースとエラーパスはユニットテストでカバレッジ。

**規則14.2**：ドキュメンテーションテスト内のコード例は簡潔である必要がある——10行以内。例により長いコンテキストが必要な場合、API設計に問題があることを意味する。

---

## プロパティテスト規範

### 使用シナリオ

**規則15.1**：以下のシナリオでは、手動で複数の境界値ケースを書く代わりに、プロパティテスト（proptestまたはquickcheck）を使用する必要がある：

| シナリオ | 例 |
|------|------|
| パーサー round-trip | `parse(pretty_print(ast)) == ast` |
| シリアライズ/デシリアライズ | `deserialize(serialize(data)) == data` |
| 数学演算恒等式 | `a + b == b + a` |
| コンパイラ最適化が意味論を変えない | `eval(code) == eval(optimize(code))` |

**規則15.2**：プロパティテストは主要なプロパティテストフレームワークとして `proptest` を使用する（`Cargo.toml` の `dev-dependencies` で宣言済み）。

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

**規則16.1**：各プロパティテストには明確なプロパティ宣言が必要である——コメントで検証する不変量を記述。

```rust
// プロパティ：任意の整数literalはtokenize → tokens_to_string後に同じ値を生成
proptest! {
    #[test]
    fn test_int_literal_roundtrip(n in any::<i64>()) {
        let source = n.to_string();
        let tokens = tokenize(&source).unwrap();
        // ...
    }
}
```

**規則16.2**：プロパティテストが失敗を発見した場合、`proptest` の回帰メカニズムを使用する必要がある——失敗した入力を `proptest-regressions/` ディレクトリに追加し、手動で通常のテストを書くことで代替禁止。

---

## カバレッジ要件

### 新規コードカバレッジ目標

**規則17.1**：新規コードのテストカバレッジ要件：

| コードタイプ | 行カバレッジ | 分岐カバレッジ |
|----------|----------|------------|
| コアコンパイラモジュール（frontend/middle/backends） | ≥ 85% | ≥ 80% |
| ツール/補助モジュール（util） | ≥ 75% | ≥ 70% |
| 実行時モジュール（vm/runtime） | ≥ 80% | ≥ 75% |
| 標準ライブラリ（std） | ≥ 75% | ≥ 70% |
| エラー処理と診断 | ≥ 90% | ≥ 85% |

**規則17.2**：エラー処理パス（すべての `Err` 分岐）は100%カバレッジが必要である。ユーザーが見るエラーメッセージはテストで検証済みである必要がある。

### PRレビュー確認リスト

**規則18.1**：PR 提出前、作成者は以下の項目を自查する必要がある：

- [ ] `cargo test` すべて通過
- [ ] `cargo test --doc` すべて通過
- [ ] `cargo bench` パフォーマンステスト回帰なし（ホットパス変更涉及時）
- [ ] 新規コードはカバレッジ目標に準拠
- [ ] テスト命名は命名規範に準拠
- [ ] 各テストファイルは対応する規範セクションを宣言（規則2.1参照）
- [ ] テスト期待値は規範定義から来ており、「現在のコードの出力」ではない
- [ ] `#[ignore]` マークのテストなし（明確なissue番号コメントがある場合を除く）
- [ ] 不必要な `unwrap()` なし（`expect` またはカスタムpanicメッセージを使用すべき）
- [ ] コミットメッセージは `:white_check_mark: test:` タイプを使用
- [ ] **「コード動作と規範不符」を理由にテスト期待値を修正していない——修正したのはコードであり、テストではない**

**規則18.2**：レビュワーは以下の問題を含むPRを拒否する必要がある：

- happy pathテストのみ、エラーパス欠落
- テストに `thread::sleep` または実行順序依存がある
- 3回以上のcopy-pasteテストコードがありヘルパー関数を抽出していない
- テスト名が命名規範に準拠していない
- 永久 `#[ignore]` テストが存在する
- **テストがコードの誤動作に合わせている**（コードと規範不符時にテストではなくコードを修正）
- **テストが対応する規範セクションを宣言していない**（規則2.1参照）
- **テスト期待値がコード出力而不是規範定義から来ている**（逆算されたテストはtestしていないのと同じ）
- テストが「panicしない」だけを検証し、具体的な動作をアサートしていない
- コードバグを暴露した失敗テストを削除した（コードを修復後に緑信号看到看到）

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

# テスト出力を表示（デフォルトはstdoutを隠す）
cargo test -- --nocapture

# 単一スレッド実行（同時実行問題排查）
cargo test -- --test-threads=1

# カバレッジレポート生成（cargo-llvm-covが必要）
cargo llvm-cov --html
```

### B. コミットメッセージテンプレート

テスト関連コミットは以下のテンプレートに従う必要がある：

```
:white_check_mark: test(<scope>): <簡単な説明>

<任意：カバーするシナリオリスト>
```

例：

```
:white_check_mark: test(parser): Prattパーサー前置演算子テストを追加

カバーするシナリオ：
- 算術演算子優先順位（+, -, *, /, %）
- 比較演算子連結（1 < x < 10）
- 論理演算子短絡
- 代入演算子右結合
```

### C. 新規テストファイルチェックリスト

新しいテストモジュールを作成する時、以下のファイルを含める必要がある：

```
# src/<module>/ ディレクトリに新規テストを追加
src/<module>/tests/
├── mod.rs          # モジュール宣言 + 公共ヘルパー関数
└── <subject>.rs    # テストファイル、被テストソースファイル命名に対応

# tests/ ディレクトリに新規統合テストを追加
tests/
├── integration.rs   # 更新：#[path]宣言を追加
└── integration/
    └── <topic>.rs   # 新規テストファイル
```

### D. 参考資料

- [YaoXiang言語規範](../../design/language-spec.md) —— **テストの権威ある水源**
- [採用されたRFC](../../design/rfc/accepted/) —— **設計決定の権威ある水源**
- [Rustテストドキュメント](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Criterion.rsユーザーガイド](https://bheisler.github.io/criterion.rs/book/)
- [proptestドキュメント](https://docs.rs/proptest/latest/proptest/)
- [プロジェクトコミット規範](./commit-convention.md)
- [プロジェクト貢献ガイド](./contributing.md)

---

> 💡 **覚えておくこと**：テストはコードが「動作する」かを検証的不是——コードが規範に準拠しているかを検証する。規範が変われば、テストも規範に従って変わる。コードが間違っているなら、コードの方を修正し、テストを修正しない。**コードは規範に奉仕し、テストは規範を守る。テストがコードに合わせ的那一刻，你就失去了所有保护。**
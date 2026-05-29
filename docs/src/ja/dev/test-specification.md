```markdown
---
title: テスト記述規範
description: YaoXiang プロジェクトのテスト記述に関する厳格な規範。ユニットテスト、統合テスト、ベンチマークテスト、ドキュメントテスト、プロパティテストの記述標準を定義
---

# テスト記述規範

本文書は YaoXiang プロジェクトのテスト記述に関する厳格な規範を定義します。すべての貢献者は以下の規則に従う必要があり、違反者はコードレビューで修正を要求されます。

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

本規範は YaoXiang プロジェクトにおけるすべての Rust テストコードに適用されます：

| テストタイプ | 場所 | フレームワーク |
|----------|------|------|
| ユニットテスト | `src/<module>/tests/` | `#[test]` + `#[cfg(test)]` |
| 統合テスト | `tests/` | `#[test]` |
| ベンチマークテスト | `benches/` | Criterion.rs |
| ドキュメントテスト | API ドキュメントコメント | `cargo test --doc` |
| プロパティテスト | 任意のテスト位置 | proptest / quickcheck |

### 基本原則

**原則 0：テストの権威あるソースは規範であり、コードではない。** これは本文書で最も重要な原則です。テストはコードが規範に適合するかを検証ものであり、現在の実装が「 通るか」を検証するものではありません。テストがコードの動作と規範の間に不一致を発見した場合、**コードを修正し、テストを修正しない。**

規範ファイルは以下の場所にあります：
- `docs/src/design/language-spec.md` —— 言語コア規範
- `docs/src/design/rfc/accepted/` —— 採用された RFC 設計文書

各テストファイルの先頭には対応する規範のセクションを宣言する必要があります（規則 2.1 参照）。開発者は規範文書を持ち出してテストと照合し、実装の正しさを検証できるべきです。逆に言えば、コードに対応する規範記述がない場合、そのコードは存在すべきではなく、テストされるべきでもありません。

```rust
// 🟢 良い——テストが規範を直接参照し、コードが規範に従うかを検証
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

// 🔴 悪い——テストが現在のコードの実装動作に合わせており、規範を検証していない
#[test]
fn test_literal_1() {
    // このコードが規範のどの節に対応するかわからない
    // parse_literal が誤った値を返しても、このテストは「グリーン通過」する
    // 関数が panic しないことだけを検証しているため
    let result = parse_literal("42");
    assert!(result.is_ok());
}
```

**シナリオ**：テストを作成し、コードの動作が規範に適合しないことを発見しました。2つの選択肢があります：
| 誤った做法 | 正しい做法 |
|----------|----------|
| テストを「通過」するように修正する | コードを修正し、動作を規範に適合させる |
| テストに `#[ignore]` を追加する | コードの実装を直ちに修正する |
| テストにコードに合わせた特別な条件分岐を追加する | 分岐を削除し、テストで問題を直接露呈させる |

覚えておいてください：**红灯 = コードが間違っている，而不是テストが間違っている。**（ただし、テスト自体にバグがある場合は別、それはまた別の話ですが。）

**原則 1：テストはドキュメントである。** すべての開発者はテストを読むことで追加のコメントや外部ドキュメントなしで被テストコードの動作を理解できるべきです。

```rust
// 🟢 良い——テスト名で何をテストし、何を期待するかを説明
#[test]
fn test_tokenize_empty_input_returns_eof() {
    let tokens = tokenize("").unwrap();
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].kind, TokenKind::Eof));
}

// 🔴 悪い——何を得ているかわからない
#[test]
fn test_tokenize_1() {
    let tokens = tokenize("").unwrap();
    assert!(tokens.len() > 0);
}
```

**原則 2：ランダムな失敗は許容しない。** テストはどのような環境でも繰り返し実行可能である必要があります。乱数、システム時刻、スレッドスケジューリング順序に依存するテストは、シード固定またはモック使用が必須です。

**原則 3：一つのテストは一つのことだけをテストする。** テスト名で複数の動作を「と」で 연결する必要がある場合、複数のテストに分割します。

```rust
// 🟢 良い——各テストは1つのシナリオのみを検証
#[test]
fn test_parse_int_positive() { /* ... */ }
#[test]
fn test_parse_int_zero() { /* ... */ }

// 🔴 悪い——1つのテストに無関係な内容が多すぎる
#[test]
fn test_parser() {
    // tokenize をテスト、parse をテスト、typecheck をテスト、codegen をテスト...
}
```

**原則 4：動作をテストし、実装をテストしない。** 内部実装のリファクタリングはテストの失敗を引き起こすべきではありません。実装コードを1行変えただけで10個のテストが落ちるなら、テストの書き方が間違っています。

しかし、ここでは重要な区別があります：**「動作」の定義は規範から来ており、現在のコードの動作からは来ません。** コードが動作を変えた場合（規範に適合しない新しい動作）、テストは失敗する必要があります。これができないなら、あなたのテストは「コード迎合型のテスト」です——バグを侵入させてしまいます。

```
規範（language-spec.md / RFC）  ──定義──►  期待動作  ──駆動──►  テスト
                                           │
現在のコード  ──実装──►  実際の動作  ──比較──►  テスト結果

実際の動作 ≠ 期待動作 の場合：
  テストは失敗する必要がある（红灯）  ──►  コードを修正  ──►  テスト通過（绿灯）
  
実際の動作 = 期待動作 の場合（だが実装が最悪）：
  テスト通過  ──►  実装をリファクタリング  ──►  テスト仍然通過  ← これが原則 4 の意味
```

**原則 5：フォールバック/互換性/特定パターンのテストコードを書かない。** テスト環境は完全に制御できる環境です。テストをスキップするために `#[cfg(not(ci))]` が必要な場合、そのテストの設計的根本的な問題があります。

### 用語定義

| 用語 | 定義 |
|------|------|
| ユニットテスト | 単一の関数やモジュールの動作をテストし、外部システムに依存しない |
| 統合テスト | 複数のモジュールが協調して動作するかをテストし、パブリック API またはコマンドライン入口を使用 |
| ベンチマークテスト | コードのパフォーマンスを測定し、パフォーマンスリグレッションを検出 |
| ドキュメントテスト | ドキュメントコメントに埋め込まれた実行可能なコード例 |
| プロパティテスト | ランダム入力に基づいて不変量（プロパティ）を検証するテスト |

### コミット規範との関連

すべてのテスト関連コミットは `:white_check_mark: test:` タイプを使用する必要があります。[コミット規範](./commit-convention.md)を参照してください。

```
:white_check_mark: test(parser): Pratt パーサーの前置式テストを追加
:white_check_mark: test(codegen): switch 文の IR 生成テストを補完
```

---

## ユニットテスト規範

### ファイル構成

**規則 1.1**：ユニットテストの `tests/` ディレクトリは被テストモジュールの `mod.rs` と**同レベル**にある必要があります。`tests/` は上位に集約されず、跨いでまとめられません。

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
└── tests/              # parser モジューレベルのテスト（pratt サブモジュールは含まない）
    ├── mod.rs
    ├── ast.rs
    ├── expressions.rs
    ├── error_recovery.rs
    └── parser_state.rs
```

主要な判断基準：**`tests/` をどのディレクトリに置くかによって、どのディレクトリの `mod.rs` が `#[cfg(test)] mod tests;` を宣言するかが決まります。**

#### 単ファイルモジュール vs ディレクトリモジュールのテスト配置規則

**主要な区別**：モジュールの組織形式がテストの配置位置を決定します。

| モジュールタイプ | 判断基準 | テスト位置 | 例 |
|----------|----------|----------|------|
| **ディレクトリモジュール** | 独立ディレクトリと `mod.rs` がある | そのディレクトリ内の `tests/` | `inference/tests/` |
| **単ファイルモジュール** | `.rs` ファイルのみ、独立ディレクトリなし | 親モジュールの `tests/` | `overload.rs` → `typecheck/tests/overload.rs` |

**詳細説明**：

```
src/frontend/core/typecheck/
├── mod.rs                          # typecheck モジュールの mod.rs
├── checker.rs                      # 単ファイルモジュール
├── environment.rs                  # 単ファイルモジュール
├── overload.rs                     # 単ファイルモジュール
├── type_eval.rs                    # 単ファイルモジュール
├── dead_code.rs                    # 単ファイルモジュール
├── spawn_placement.rs              # 単ファイルモジュール
├── signature.rs                    # 単ファイルモジュール
├── types.rs                        # 単ファイルモジュール
│
├── tests/                          # ✅ typecheck のテストディレクトリ
│   ├── mod.rs                      # 単ファイルモジュールのテストを宣言
│   ├── checker.rs                  # checker.rs のテスト
│   ├── environment.rs              # environment.rs のテスト
│   ├── overload.rs                 # overload.rs のテスト（単ファイルモジュールテストはここに配置）
│   ├── type_eval.rs                # type_eval.rs のテスト
│   ├── dead_code.rs                # dead_code.rs のテスト
│   ├── spawn_placement.rs          # spawn_placement.rs のテスト
│   ├── signature.rs                # signature.rs のテスト
│   └── types.rs                    # types.rs のテスト
│
├── inference/                      # ディレクトリモジュール（mod.rs がある）
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
└── traits/                         # ディレクトリモジュール（mod.rs がある）
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

**なぜ単ファイルモジュールのテストは親レベルの `tests/` に置くのですか？**

単ファイルモジュール（`overload.rs` など）には独自の `mod.rs` がないため、`#[cfg(test)] mod tests;` を宣言できません。Rust のモジュールシステムでは、テストファイルはどれかの `mod.rs` によって宣言されないとコンパイルされません。したがって、単ファイルモジュールのテストは親モジュールの `mod.rs` によって宣言され、親レベルの `tests/` ディレクトリに配置する必要があります。

**判断フロー**：

```
モジュールに遭遇、テストをどこにするか判断？
│
├── そのモジュールはディレクトリか（mod.rs があるか）？
│   └── はい → そのディレクトリ下に tests/ を作成し、そのディレクトリの mod.rs が宣言
│
├── そのモジュールは単ファイルか（.rs ファイルのみ）？
│   └── はい → テストは親の tests/ ディレクトリに配置し、親の mod.rs が宣言
│
└── 不確定？
    └── 独立ディレクトリと mod.rs があるかを確認
```

**一般的なエラー**：

```
# ❌ エラー 1：単ファイルモジュール用に独立の tests/ ディレクトリを作成
src/frontend/core/typecheck/
├── overload.rs
└── overload/                       # ❌ 単ファイルモジュール用にディレクトリを作成すべきでない
    └── tests/
        └── overload.rs

# ❌ エラー 2：単ファイルモジュール内で #[cfg(test)] mod tests; を宣言
# overload.rs
#[cfg(test)]                        # ❌ 単ファイルモジュールでは 이렇게 선언할 수 없음
mod tests;                          # overload/tests/ ディレクトリがないため

# ✅ 正しい做法：テストは親の tests/ に配置
src/frontend/core/typecheck/
├── overload.rs                     # ソースファイル
└── tests/
    └── overload.rs                 # テストファイル、typecheck/mod.rs が宣言
```

⚠️ **アンチパターン—— 이렇게 하지 마세요：**

```
# ❌ エラー：サブモジュールのテストを親レベルに集中させる
src/frontend/core/types/
├── mod.rs              # base と computation のみを宣言すべき
├── base/
│   ├── mod.rs
│   └── var.rs
└── tests/              # ❌ 親の tests/ にサブモジュールのテストが含まれている
    ├── mod.rs          # ❌ mod base; mod computation; を宣言被迫
    ├── base/           # ❌ これは base/tests/ に配置すべき
    │   └── var.rs
    └── computation/    # ❌ これは computation/tests/ に配置すべき
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

**なぜ上位への集約ができないのですか？** Rust のモジュールシステムは `#[cfg(test)] mod tests;` が宣言箇所でテストファイルのコンパイルを決定するからです。`types/mod.rs` が `mod tests;` を宣言した場合、`types/tests/` の内容は `types` モジュールのプライベート内容になります——それは `base` や `computation` の領域に跨いではなりません。各モジュールのテストは、そのモジュールの内部実装の詳細であるべきであり、親モジュールのそれではありません。この規則はモジュールのリファクタリングにも適用されます：`types` を `base` と `computation` に分割する場合、テストも分割後のモジュールに沿って分割されるべきであり、元の場所に留まるべきではありません。**テストディレクトリはソース構造をミラーするのではなく、モジュール境界に従います。**

**規則 1.2**：`tests/mod.rs` はモジュールの宣言と re-export のみを担当し、テスト関数は配置しません。

```rust
//! Parser core tests — mirrors src/frontend/core/parser/
//!
//! ast.rs、parser_state.rs、式/統合解析のテスト。

mod ast;
mod error_recovery;
mod expressions;
mod integration;
mod parser_state;
```

**規則 1.3**：各テストファイルは1つのソースファイルのみに対応します。複数のソースモジュールのテストを1つのファイルに混在させることは許可されません。

**規則 1.4**：`#[cfg(test)]` は次の2箇所のみに出現できます——`lib.rs` で `mod tests` を宣言するか、被テストソースファイル内でインラインで `#[cfg(test)] mod tests;` を宣言します。他の場所での使用は禁止です。

```rust
// src/frontend/core/parser/mod.rs または lib.rs
#[cfg(test)]
mod tests;
```

### モジュール宣言規範

**規則 2.1**：すべてのテストファイルの先頭にモジュールレベルのドキュメントコメント `//!` が必要であり、テストがカバーする規範ソース（言語規範の章番号 + RFC 番号）を説明します。特定のテストが規範の章を参照していない場合、そのコードには規範的な根拠がないことを意味します——存在すべきではなく、テストされるべきでもありません。

```rust
//! リテラルテスト — 言語規範 §2.6 に基づく
//!
//! §2.6.1: 整数 Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: 浮動小数点数（小数点と指数を含む）
//! §2.6.3: 文字列（エスケープシーケンス \\nrt'"\\, \\x, \\u{}）
//! RFC-012: F-String 補間
```

**なぜ規範を参照する必要があるのですか？** テストの期待値は規範から来ており、「現在のコードの出力」から来ていてはならないからです。いつかコードが出力を変え、テスト随之更新された場合、そのテストは何も保護していません。規範にアンカーされたテストだけが「意図的な breaking change」と「意図しないリグレッション」を区別できます。

**規則 2.2**：テストモジュールの `use` インポートは具体的な型/関数に正確にする必要があり、glob インポート `use super::*;` は禁止です。

```rust
// 🟢 良い——正確なインポート
use crate::frontend::core::lexer::{tokenize, TokenKind};
use crate::frontend::core::parser::{ParserState, ParseError};

// 🔴 悪い——何をしているのかわからない
use super::*;
```

### 命名規範

**規則 3.1**：テスト関数名の形式は `test_<what>_<scenario>` とし、すべて小文字でアンダースコア区切りです。

```rust
#[test]
fn test_tokenize_empty_string() { /* ... */ }
#[test]
fn test_parse_int_overflow() { /* ... */ }
#[test]
fn test_typecheck_fn_return_mismatch() { /* ... */ }
```

**規則 3.2**：テスト関数名は自己説明的である必要があります。関数名を読んだだけで何をテストし何を期待するかがわかるべきです。数字の連番による命名は禁止です。

```rust
// 🟢 良い
fn test_skip_semicolon_success() { /* ... */ }
fn test_skip_semicolon_failure_when_identifier() { /* ... */ }

// 🔴 悪い——何をテストするのか全くわからない
fn test_skip_1() { /* ... */ }
fn test_skip_2() { /* ... */ }
```

**規則 3.3**：ヘルパー関数には `test_` プレフィックスは不要であり、動詞または名詞で用途を描述する必要があります。

```rust
fn parse_expr(source: &str) -> Expr { /* ... */ }
fn tokenize_single(source: &str) -> Token { /* ... */ }
fn setup_parser_with_tokens(tokens: &[Token]) -> ParserState { /* ... */ }
```

### テスト構造規範 (Arrange-Act-Assert)

**規則 4.1**：各テスト関数は3段階構造に従う必要があります：準備（Arrange）→ 実行（Act）→ 表明（Assert）、各段階の間は空行で区切ります。

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

**規則 4.2**：単純なテスト（単一呼び出し + 単一表明）は段階コメントを省略可能ですが、5行以上のロジックコードを含むテストは3段階を明示的に示す必要があります。

### ヘルパー関数規範

**規則 5.1**：3回以上繰り返される setup ロジックはヘルパー関数として抽出する必要があります。

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

**規則 5.2**：ヘルパー関数内の `unwrap()` / `expect()` は、パニック時に十分なコンテキストを出力する必要があります。テスト関数本体（`#[test] fn ...`）では直接 `unwrap()` を使用できます——失敗時、Rust は自動的に行番号を出力します。しかしヘルパー関数内で失敗した場合、行番号はヘルパー関数の定義箇所を指し、呼び出し時のコンテキストが見えません。

```rust
// 🟢 良い——ヘルパー関数が失敗した時、ソースコードの内容を出力
fn run_ok(source: &str) {
    run(source).unwrap_or_else(|e| panic!("Execution failed:\nSource:\n{}\nError:\n{:?}", source, e));
}

// 🔴 悪い——失敗した時、どのソースファイルが問題を起こしたかわからない
fn run_ok(source: &str) {
    run(source).unwrap();
}
```

**規則 5.3**：ヘルパー関数はテストファイルの先頭、 `use` インポートの直後に配置する必要があります。複数のテストモジュールで共有される場合、`tests/mod.rs` に配置し `pub(crate)` でエクスポートします。

### 表明スタイル

**規則 6.1**：列挙型変体の照合には `assert!(matches!(...))` を使用することが優先され、`if let` + `panic!` は使用禁止です。

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

**規則 6.2**：精密な値の比較には `assert_eq!` を使用し、ブール表明には `assert!` を使用します。`assert!(a == b)` を `assert_eq!(a, b)` の代わりに使用することは禁止です。

**規則 6.3**：すべての表明にはカスタムエラーメッセージを含める必要があります，表明自体が既に失敗理由を完全に説明している場合は例外です。

```rust
// 🟢 良い——表明失敗時に素早く位置を特定できる
assert!(
    state.infix_info().is_some(),
    "infix_info should handle '{op}'"
);

// 🟢 良い——assert_eq! 失敗時に値が自動的に出力されるため、追加メッセージ不要
assert_eq!(error_count, 0);

// 🔴 悪い——失敗すると「表明失敗」のみがわかる
assert!(state.infix_info().is_some());
```

**規則 6.4**：表明の順序は `assert_eq!(actual, expected)` とし、実際の値を前、期待値を後にします。

### アンチパターンのリスト

以下是禁止の写法及其替代方案：

| アンチパターン | 問題 | 代替方案 |
|--------|------|----------|
| テストがコードのエラー動作迎合 | 規範偏差を隠蔽し、bug を合法化 | 規範に従ってコードを修正し、テストは不变 |
| コード出力を逆算してテスト期待値を設定 | テストが「現在の実装の録音機」になる | 規範から期待値を導出 |
| `#[ignore]` の永続マーク | 腐ったテストを隠蔽 | 修正または削除 |
| `println!` デバッグ出力 | テスト出力を汚染 | `assert!` を使用して明確に表明 |
| `thread::sleep` | ランダム失敗 + 遅い | 同期メカニズムまたはモックを使用 |
| テストで реальний ファイルシステムを操作 | 遅く且つ再現できない | `tempfile` を使用 |
| テスト実行順序に依存 | ランダム失敗 | 各テストが独立した setup |
| 1つのテスト関数が30行以上のロジック | 誰にも理解できない | テストを分割またはヘルパー関数を使用 |
| ヘルパー関数の `unwrap()` がコンテキストを報告しない | 位置特定が困難 | `expect("理由")` またはカスタム panic を使用（規則 5.2 参照）|
| 3回以上同じ setup を copy-paste | 変更コストが高い | ヘルパー関数を抽出 |

---

## 統合テスト規範

### テスト構成

**規則 7.1**：統合テストはプロジェクトルートの `tests/` ディレクトリに配置します。入口ファイル `tests/integration.rs` は `#[path]` 属性を使用してサブモジュールを導入します。

```rust
// tests/integration.rs
#[path = "integration/backends.rs"]
mod backends;
#[path = "integration/codegen.rs"]
mod codegen;
#[path = "integration/execution.rs"]
mod execution;
```

**規則 7.2**：各 `tests/integration/*.rs` ファイルは1つのテストテーマ（コンパイラバックエンド、コード生成、エグゼキュータなど）に対応し、混合配置は禁止です。

**規則 7.3**：統合テストはプロジェクトのパブリック API を通じてテストする必要があります。統合テストで `crate::` 内部モジュールを直接参照することは禁止です。`yaoxiang::` パブリックパスを使用します。

```rust
// 🟢 良い——パブリック API を使用
use yaoxiang::run;

// 🔴 悪い——パブリック API 境界をバイパス
use yaoxiang::middle::codegen::bytecode::BytecodeFile;
```

### テストデータ管理

**規則 8.1**：統合テストではインラインソースコード文字列を優先的に使用します。ソースコードが30行を超える場合にのみ、外部 fixture ファイル（`tests/fixtures/` に配置）を使用します。

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

**規則 8.2**：fixture ファイルは `.yx` 拡張子で終わらせる必要があり、ファイル名でテストの意図を描述します。

### E2E カバレッジ原則

**規則 9.1**：各言語機能の統合テストは3つのパスをカバーする必要があります：

| パス | 説明 |
|------|------|
| Happy path | 合法入力が期待される出力を生成 |
| Error path | 非法入力が明確なエラーメッセージを生成（非 panic） |
| Boundary | 境界値（空入力、最大値、ネスト深度上限） |

**規則 9.2**：統合テストはネットワーク、システム環境変数、外部サービスに依存してはなりません。

---

## ベンチマークテスト規範

### Criterion.rs 使用規範

**規則 10.1**：ベンチマークテストは `benches/` ディレクトリに統一して配置し、入口ファイルは `benches/lib.rs` です。テストテーマ別にファイルを分けます。

```
benches/
├── lib.rs              # 入口、criterion_group/criterion_main を定義
├── lang_compare/
│   └── fibonacci.rs    # 跨言語比較ベンチマーク
├── parser.rs           # パーサーバンチマーク
└── codegen.rs          # コード生成ベンチマーク
```

**規則 10.2**：各ベンチマーク関数にはテスト目的と測定指標を説明するモジュールドキュメントコメント `//!` を含める必要があります。

```rust
//! YaoXiang インタープリタ性能ベンチマークテスト
//!
//! 測定指標：単一反復時間（wall time）
//! 基準線：Rust ネイティブ実装
```

### コンパイラ最適化防止

**規則 11.1**：すべてのベンチマークテストの被テスト出力は `criterion::black_box` を使用してコンパイラの最適化除去を防ぎます。

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

**規則 11.2**：ベンチマークテストの入力データは `const` または `lazy_static` である必要があり、`iter` クロージャ内で動的に生成してはなりません——さもなくば測定するのはデータ生成 + 被テストロジックの合計時間になります。

### ベンチマークグループ化と命名

**規則 12.1**：ベンチマークテストの名前形式は `<被テストモジュール>_<シナリオ>` とし、すべて小文字でアンダースコア区切りです。ユニットテストの命名規則と一致します。

**規則 12.2**：`criterion_group!` を使用して関連するベンチマークを論理的にグループ化する必要があります。すべてのベンチマークを1つのグループに押し込むことは禁止です。

```rust
criterion_group!(parser, bench_parse_expr, bench_parse_stmt);
criterion_group!(codegen, bench_codegen_module, bench_codegen_switch);
criterion_main!(parser, codegen);
```

---

## ドキュメントテスト規範

### 使用シナリオ

**規則 13.1**：すべての `pub` 関数、型、メソッドには少なくとも1つの実行可能なコード例を含める必要があります。この例は `cargo test --doc` で実行されます。

```rust
/// ソースコード文字列を Token シーケンスにトークナイズします。
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

**規則 13.2**：ドキュメントテストのコード例はコンパイルが通るかつ表明が成功する必要があります。コンパイル時エラーを示す例でない限り、`ignore` マークを含む例は禁止です。

```rust
/// ```ignore
/// // コンパイル時エラーを示す——ignore 可能
/// let x: int = "string";
/// ```
```

### カバレッジ要件

**規則 14.1**：ドキュメントテストは API の happy path のカバレッジのみで十分です。境界情况和錯誤パスはユニットテストでカバーします。

**規則 14.2**：ドキュメントテストのコード例は簡潔である必要があります——10行以内とします。例に追加のコンテキストが必要な場合、API 設計に問題があることを示唆しています。

---

## プロパティテスト規範

### 使用シナリオ

**規則 15.1**：以下のシナリオでは、手書きの複数の境界値テストケースではなく、プロパティテスト（proptest または quickcheck）を使用する必要があります：

| シナリオ | 例 |
|------|------|
| パーサー round-trip | `parse(pretty_print(ast)) == ast` |
| シリアライズ/デシリアライズ | `deserialize(serialize(data)) == data` |
| 数学演算の恒等式 | `a + b == b + a` |
| コンパイラ最適化が意味論を変えない | `eval(code) == eval(optimize(code))` |

**規則 15.2**：プロパティテストでは `proptest` を主要なプロパティテストフレームワークとして使用します（`Cargo.toml` の `dev-dependencies` に宣言済み）。

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

**規則 16.1**：各プロパティテストには明確なプロパティ宣言が必要です——コメントで検証される不変量を記述します。

```rust
// プロパティ：任意の整数字リテラルが tokenize → tokens_to_string 後に同じ値を生成
proptest! {
    #[test]
    fn test_int_literal_roundtrip(n in any::<i64>()) {
        let source = n.to_string();
        let tokens = tokenize(&source).unwrap();
        // ...
    }
}
```

**規則 16.2**：プロパティテストが失敗を発見した場合、`proptest` の回帰メカニズムを使用する必要があります——失敗した入力を `proptest-regressions/` ディレクトリに追加し、手書きの通常のテストで代替してはなりません。

---

## カバレッジ要件

### 新規コードのカバレッジ目標

**規則 17.1**：新規コードのテストカバレッジ要件：

| コードタイプ | 行カバレッジ | 分岐カバレッジ |
|----------|----------|------------|
| コアコンパイラモジュール（frontend/middle/backends） | ≥ 85% | ≥ 80% |
| ユーティリティ/ヘルパーモジュール（util） | ≥ 75% | ≥ 70% |
| ランタイムモジュール（vm/runtime） | ≥ 80% | ≥ 75% |
| 標準ライブラリ（std） | ≥ 75% | ≥ 70% |
| エラー処理と診断 | ≥ 90% | ≥ 85% |

**規則 17.2**：エラー処理パス（すべての `Err` 分岐）は100%カバレッジが必要です。ユーザーが見ることのできるエラーメッセージはテストで検証済みである必要があります。

### PR レビューチェックリスト

**規則 18.1**：PR を提出する前に、著者は以下の項目を自己確認する必要があります：

- [ ] `cargo test` がすべて通過
- [ ] `cargo test --doc` がすべて通過
- [ ] `cargo bench` にパフォーマンスリグレッションがない（ホットパスの変更涉及する場合）
- [ ] 新規コードがカバレッジ目標に適合
- [ ] テスト名が命名規範に適合
- [ ] 各テストファイルが対応する規範セクションを宣言（規則 2.1）
- [ ] テスト期待値が規範定義から来ており、「現在のコードの出力」から来ていない
- [ ] `#[ignore]` マークのテストがない（明確な issue 番号コメントがある場合を除く）
- [ ] 不必要な `unwrap()` がない（`expect` またはカスタム panic メッセージを使用すべき）
- [ ] コミットメッセージが `:white_check_mark: test:` タイプを使用
- [ ] **「コードの動作が規範に適合しない」ためにテスト期待値を修正していない——修正したのはコードであり、テストではない**

**規則 18.2**：レビュアーは以下の問題を含む PR を拒否する必要があります：

- happy path テストのみで、錯誤パスがない
- テストに `thread::sleep` または実行順序に依存がある
- 3回以上コピー＆ペーストされたテストコードがヘルパー関数に抽出されていない
- テスト名が命名規範に適合しない
- 永続的な `#[ignore]` のテストが存在する
- **テストがコードのエラー動作迎合**（コードと規範が不一致な時、テストではなくコードを修正）
- **テストが対応する規範セクションを宣言していない**（規則 2.1 参照）
- **テスト期待値がコード出力から来ており、規範から来ていない**（逆算されたテストはテストにならない）
- テストが「panic しない」だけを検証し、具体的な動作を表明していない
- コードの bug を露呈する失敗テストを削除した（コード修正後に緑になるのを見ているべき）

---

## 付録

### A. テストコマンド早見表

```bash
# すべてのテストを実行
cargo test

# ユニットテストのみを実行
cargo test --lib

# 統合テストのみを実行
cargo test --test integration

# ドキュメントテストのみを実行
cargo test --doc

# 特定のテストを実行（名前でフィルタ）
cargo test test_parse_expr

# ベンチマークテストを実行
cargo bench

# テスト出力を表示（デフォルトでは非表示）
cargo test -- --nocapture

# 単一スレッドで実行（並行問題排查）
cargo test -- --test-threads=1

# カバレッジレポートを生成（cargo-llvm-cov が必要）
cargo llvm-cov --html
```

### B. コミットメッセージテンプレート

テスト関連コミットは以下のテンプレートに従う必要があります：

```
:white_check_mark: test(<scope>): <简短説明>

<オプション：カバーするシナリオリスト>
```

例：

```
:white_check_mark: test(parser): Pratt パーサーの前置式テストを追加

カバーするシナリオ：
- 算術演算子の優先順位（+, -, *, /, %）
- 比較演算子リンク（1 < x < 10）
- 論理演算子の短絡評価
- 代入演算子の右結合
```

### C. 新規テストファイルチェックリスト

新しいテストモジュールを作成する際は、以下のファイルが含まれていることを確認します：

```
# src/<module>/ ディレクトリ下に新規テストを追加
src/<module>/tests/
├── mod.rs          # モジュール宣言 + 公共ヘルパー関数
└── <subject>.rs    # テストファイル、被テストソースファイルに対応

# tests/ ディレクトリ下に新規統合テストを追加
tests/
├── integration.rs   # 更新：#[path] 宣言を追加
└── integration/
    └── <topic>.rs   # 新規テストファイル
```

### D. 参考資料

- [YaoXiang 言語規範](../../design/language-spec.md) —— **テストの権威あるソース**
- [採用された RFC](../../design/rfc/accepted/) —— **設計決定の権威あるソース**
- [Rust テストドキュメント](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Criterion.rs ユーザーガイド](https://bheisler.github.io/criterion.rs/book/)
- [proptest ドキュメント](https://docs.rs/proptest/latest/proptest/)
- [プロジェクトコミット規範](./commit-convention.md)
- [プロジェクト寄稿ガイド](./contributing.md)

---

> 💡 **覚えておいてください**：テストはコードが「 通るか」を検証するのではなく、コードが規範に適合するかを検証します。規範が変われば、テストも規範に沿って変わります。コードが間違っていれば、コードを変え、テストを変えないでください。**コードは規範に奉仕し、テストは規範を守ります。テストがコード迎合的那一刻，你就失去了所有保护。**
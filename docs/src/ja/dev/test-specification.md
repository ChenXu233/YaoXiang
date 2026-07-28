---
title: 'テスト記述規範'
description:
  YaoXiang プロジェクトのテスト記述に関する硬規範。ユニットテスト、統合テスト、ベンチマークテスト、ドキュメントテスト、プロパティテストの記述標準を定義
---

# テスト記述規範

本文書は YaoXiang プロジェクトのテスト記述に関する硬規範を定義しています。すべてのコントリビューターは以下のルールに従う必要があり、違反した場合は Code Review で修正を求められます。

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

| テスト種別 | 位置                  | フレームワーク                     |
| ---------- | --------------------- | ---------------------------------- |
| ユニットテスト | `src/<module>/tests/` | `#[test]` + `#[cfg(test)]` |
| 統合テスト | `tests/`              | `#[test]`                          |
| ベンチマークテスト | `benches/`            | Criterion.rs                       |
| ドキュメントテスト | API ドキュメントコメント | `cargo test --doc`                |
| プロパティテスト | 任意のテスト位置       | proptest / quickcheck              |

### 基本原則

**原則 0：テストの権威あるソースは規範であり、コードではない。**
これは本文書において最も重要な原則です。テストはコードが規範是否符合かを検証ものであり、コードが「現在の実装で動作したか」を検証するものではありません。テストがコードの動作と規範が一致しないことを発見した場合、**コードを修正し、テストを修正してはならない**。

規範ファイルは以下の場所にあります：

- `docs/src/design/language-spec.md` —— 言語コア規範
- `docs/src/design/rfc/accepted/` —— 承認済み RFC 設計文書

各テストファイルの先頭には対応する規範セクションを宣言する必要があります（ルール 2.1 参照）。すべての開発者は規範文書，拿着テストを照らし合わせて実装の正しさを検証できるべきです。逆に言えば、あるコードに対応する規範記述が存在しない場合、そのコードは存在すべきではなく、さらにテストされるべきでもないのです。

```rust
// 🟢 良い——テストが規範を直接参照し、コードが規範に従っているかを検証
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

// 🔴 悪い——テストが現在のコードの実装動作に忖度しており、規範を検証していない
#[test]
fn test_literal_1() {
    // このコードが規範のどの節に対応するかわからない
    // parse_literal が誤った値を返した場合、このテストは「合格」する
    // 関数が panic しないことだけを検証しているため
    let result = parse_literal("42");
    assert!(result.is_ok());
}
```

**シナリオ**：あなたはテストを書き、コードの動作が規範に反することを発見しました。あなたは二つの選択肢があります：

| 誤った做法                         | 正しい做法                           |
| ---------------------------------- | ------------------------------------ |
| テストを「合格」するよう修正する   | コードを変更し、動作を規範に合わせる   |
| テストに `#[ignore]` を追加する    | コード実装を即座に修正する            |
| テストに特殊条件分岐を追加して忖度 | 分岐を削除し、テストで問題を露わにする |

覚えておいてください：**赤信号 = コードが間違っている，而非テストが間違っている。**（ただし、あなたのテスト自体にバグがある場合は別話です。）

**原則 1：テストはドキュメントである。** すべての開発者はテストを読むことで被テストコードの動作を理解できるべきであり、追加のコメントや外部ドキュメントを必要としないはずです。

```rust
// 🟢 良い——テスト名が何をテストし、何を期待しているかを説明している
#[test]
fn test_tokenize_empty_input_returns_eof() {
    let tokens = tokenize("").unwrap();
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].kind, TokenKind::Eof));
}

// 🔴 悪い——何のテストかわからない
#[test]
fn test_tokenize_1() {
    let tokens = tokenize("").unwrap();
    assert!(tokens.len() > 0);
}
```

**原則 2：ランダムな失敗は絶対に許さない。**
テストはどのような環境でも再現可能に実行できなければなりません。乱数、システム時刻、スレッドスケジューリング順序に依存するテストは、シード固定または mock を使用する必要があります。

**原則 3：一つのテストは一つのことだけをテストする。** テスト名で「と」で複数の動作を接続する必要がある場合、複数のテストに分割してください。

```rust
// 🟢 良い——各テストは一つのシナリオのみを検証
#[test]
fn test_parse_int_positive() { /* ... */ }
#[test]
fn test_parse_int_zero() { /* ... */ }

// 🔴 悪い——一つのテストに無関係な内容が太多
#[test]
fn test_parser() {
    // tokenize をテスト、parse をテスト、typecheck をテスト、codegen をテスト...
}
```

**原則 4：動作をテストし、実装をテストしない。**
内部実装のリファクタリングはテストの失敗を引き起こすべきではありません。一行の実装コードを変えて10個のテストが落ちたら、あなたのテストの書き方が間違っています。

しかし、ここで重要な区別があります：**「動作」の定義は規範から来ており、現在のコードの動作からは来ません。**
コードが動作を変えた場合（つまり規範不符の新しい動作）、テストは失敗する必要があります。これが做不到であれば、あなたのテストは「コードに忖度したテスト」です——バグが入り込む余地を与えてしまいます。

```
規範（language-spec.md / RFC）  ──定義──►  期待動作  ──駆動──►  テスト
                                           │
現在のコード  ──実装──►  実際の動作  ──対比──►  テスト結果

実際の動作 ≠ 期待動作の場合：
  テストは失敗する必要がある（赤信号）  ──►  コードを修正  ──►  テスト合格（緑信号）

実際の動作 = 期待動作の場合（ただし実装が酷い）：
  テスト合格  ──►  実装をリファクタリング  ──►  テストも変わらず合格  ← これが原則 4 の意味
```

**原則 5：フォールバック/互換/特定パターンが有効になるテストコードを書かない。** テスト環境は完全に制御できる環境です。テストをスキップするために `#[cfg(not(ci))]` が必要な場合、そのテスト設計には根本的な問題があります。

### 用語定義

| 用語           | 定義                                               |
| -------------- | -------------------------------------------------- |
| ユニットテスト | 単一の関数またはモジュールの動作をテストし、外部システムに依存しない |
| 統合テスト     | 複数のモジュールが協調して動作することをテストし、パブリック API またはコマンドライン入口を使用 |
| ベンチマークテスト | コードのパフォーマンスを測定し、パフォーマンスリグレッションを検出 |
| ドキュメントテスト | ドキュメントコメントに埋め込まれた実行可能なコード例 |
| プロパティテスト | ランダム入力に基づいて不変量（property）を検証するテスト |

### コミット規範との関連

すべてのテスト関連コミットは `:white_check_mark: test:` タイプを使用し、[コミット規範](./commit-convention.md)を参照してください。

```
:white_check_mark: test(parser): Pratt パーサーの前置式テストを追加
:white_check_mark: test(codegen): switch 文の IR 生成テストを補完
```

---

## ユニットテスト規範

### ファイル構成

**ルール 1.1**：ユニットテストの `tests/` ディレクトリは被テストモジュールの `mod.rs` **と同レベル**にある必要があります。`tests/` は上位に集約せず、跨いで汇总しません。

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
└── tests/              # parser モジューレベルのテスト（pratt サブモジュールの内容は含まない）
    ├── mod.rs
    ├── ast.rs
    ├── expressions.rs
    ├── error_recovery.rs
    └── parser_state.rs
```

重要な判断基準：**`tests/` を配置するディレクトリが、そのディレクトリの `mod.rs` で `#[cfg(test)] mod tests;` を宣言する必要があります。**

**ルール 1.1 補足：上位への集約を禁止。** サブディレクトリモジュールのテストは、そのサブディレクトリ自身の `tests/` に配置しなければならず、親レベルの `tests/` に集約してはなりません。

| モジュールタイプ             | テスト位置              | 例                                           |
| -------------------------- | --------------------- | -------------------------------------------- |
| ディレクトリモジュール（`mod.rs` あり）  | そのディレクトリ下の `tests/` | `emitter/tests/`、`codes/tests/`             |
| 単一ファイルモジュール（`.rs` のみ）  | 親の `tests/`     | `session.rs` → `diagnostic/tests/session.rs` |

```text
# ✅ 正しい：各ディレクトリモジュールのテストは独立
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
    ├── mod.rs              # ❌ mod emitter; mod codes; を宣言被迫
    ├── emitter/            # ❌ emitter/tests/ にあるべき
    └── codes/              # ❌ codes/tests/ にあるべき
```

#### 単一ファイルモジュール vs ディレクトリモジュールのテスト配置ルール

**核心的な違い**：モジュールの組織形式がテストの配置位置を決定します。

| モジュールタイプ     | 判断基準                    | テスト位置              | 例                                           |
| -------------- | --------------------------- | ------------------- | --------------------------------------------- |
| **ディレクトリモジュール**   | 独立ディレクトリがあり `mod.rs` がある       | そのディレクトリ下の `tests/` | `inference/tests/`                            |
| **単一ファイルモジュール** | `.rs` ファイルのみ、独立ディレクトリなし | 親モジュールの `tests/` | `overload.rs` → `typecheck/tests/overload.rs` |

**詳細な説明**：

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
│   ├── overload.rs                 # overload.rs のテスト（単一ファイルモジュールテストはこちら）
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

**なぜ単一ファイルモジュールのテストは親レベルの `tests/` にするのか？**

単一ファイルモジュール（例：`overload.rs`）には独自の `mod.rs` がなく、`#[cfg(test)] mod tests;` を宣言できません。Rust のモジュールシステムにより、テストファイルは某かの `mod.rs` によって宣言されなければコンパイルできません。したがって、単一ファイルモジュールのテストは親モジュールの `mod.rs` によって宣言され、親レベルの `tests/` ディレクトリに配置する必要があります。

**判断フロー**：

```
モジュールに遭遇し、テストをどこに配置するか判断する？
│
├── そのモジュールはディレクトリか？（mod.rs あり）？
│   └── はい → そのディレクトリ下に tests/ を作成し、そのディレクトリの mod.rs が宣言
│
├── そのモジュールは単一ファイルか？（.rs のみ）？
│   └── はい → テストは親の tests/ ディレクトリに配置し、親の mod.rs が宣言
│
└── 不確定？
    └── 独立ディレクトリと言 mod.rs があるかをチェック
```

**よくある誤り**：

```
# ❌ 誤り 1：単一ファイルモジュール用に独立の tests/ ディレクトリを作成
src/frontend/core/typecheck/
├── overload.rs
└── overload/                       # ❌ 単一ファイルモジュール用にディレクトリを作成しない
    └── tests/
        └── overload.rs

# ❌ 誤り 2：単一ファイルモジュール内で #[cfg(test)] mod tests; を宣言
# overload.rs
#[cfg(test)]                        # ❌ 単一ファイルモジュールではこのように宣言できない
mod tests;                          # overload/tests/ ディレクトリがないため

# ✅ 正しい做法：テストは親の tests/ に配置
src/frontend/core/typecheck/
├── overload.rs                     # ソースファイル
└── tests/
    └── overload.rs                 # テストファイル、typecheck/mod.rs が宣言
```

⚠️ **アンチパターン——以下のように書かない：**

```
# ❌ 誤り：サブモジュールのテストを親レベルに集中
src/frontend/core/types/
├── mod.rs              # 本来は base と computation のみを宣言すべき
├── base/
│   ├── mod.rs
│   └── var.rs
└── tests/              # ❌ 親の tests/ にサブモジュールのテストが含まれている
    ├── mod.rs          # ❌ mod base; mod computation; を宣言被迫
    ├── base/           # ❌ この部分は base/tests/ にあるべき
    │   └── var.rs
    └── computation/    # ❌ この部分は computation/tests/ にあるべき
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

**なぜ上位への集約はできないのか？** Rust のモジュールシステムは `#[cfg(test)] mod tests;` が宣言時点でテストファイルのコンパイルを決定します。`types/mod.rs` が `mod tests;` を宣言すると、`types/tests/` の内容は `types` モジュールのプライベートな内容になります——それは `base` や `computation` の領域に跨いではなりません。各モジュールのテストはそのモジュールの内部実装の詳細であり、親モジュールの者にはありません。このルールはモジュールのリファクタリングにも適用されます：`types` を `base` と `computation` に分割する際、テストも分割後のモジュールに従って分割されるべきであり、元の場所に残すべきではありません。**テストディレクトリはソースコード構造をミラーするのではなく、モジュール境界に従います。**

**ルール 1.2**：`tests/mod.rs` はモジュールの宣言と re-export のみを担当し、テスト関数は配置しません。

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

**ルール 1.3**：各テストファイルは一つのソースファイルのみに対応します。複数のソースモジュールのテストを一つファイルに混在させることは許可されません。

**ルール 1.4**：テストの宣言はファイル形式 `mod tests;`（セミコロン付き）を使用し、同レベルの `tests/` ディレクトリを指す必要があります。**インライン形式 `mod tests { ... }` でテストコードをソースファイル内に直接書くことは禁止です。**

```rust
// ✅ 正しい——ファイル形式で宣言、テストコードは独立ファイルに配置
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
        // テストコードはソースファイルに現れてはならない
    }
}
```

**なぜインラインが禁止なのか？**

1. ソースファイルの責務を明確に：ソースファイルは実装のみを配置し、テストファイルはテストのみを配置。混在させると、テストを修正するためにファイルの末尾までスクロールし、実装を修正するためにテストをスキップする必要がある。
2. モジュール境界を明確に：`tests/` ディレクトリは物理的な境界であり、どのモジュールにテストがありどのモジュールにないかが一目でわかる。
3. リファクタリングを安全に：モジュールを分割する際、`tests/` ディレクトリは一緒についていく；インラインテストはソースファイルから手動で切り離す必要がある。
4. コードレビュー：PR diff でソースコードの変更とテストの変更は別ファイルであり、混在しない。

### モジュール宣言規範

**ルール 2.1**：すべてのテストファイルの先頭にモジュールレベルのドキュメントコメント `//!` を記述し、テストがカバーする規範の出典（言語規範セクション番号 + RFC 番号）を説明する必要があります。あるテストが規範セクションを参照していない場合、そのコードには規範上の根拠がないことを意味します——存在すべきではなく、テストされるべきでもないのです。

```rust
//! リテラルテスト — 言語規範 §2.6 に基づく
//!
//! §2.6.1: 整数 Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: 浮動小数点数（小数点と指数を含む）
//! §2.6.3: 文字列（エスケープシーケンス \\nrt'"\\, \\x, \\u{}）
//! RFC-012: F-String 補間
```

**なぜ規範を参照する必要があるのか？**
テストの期待値は規範から来ており、「現在のコードの出力」から来ていてはならないからです。もしコードがいつかは出力を変え、テスト随之更新された場合、そのテストは何も保護していません。規範に準拠したテストだけが「意図的な breaking change」と「意図しないリグレッション」を区別できます。

**ルール 2.2**：テストモジュールの `use` インポートは具体的な型/関数に精密に行い、glob インポート `use super::*` は禁止です。

```rust
// 🟢 良い——精密なインポート
use crate::frontend::core::lexer::{tokenize, TokenKind};
use crate::frontend::core::parser::{ParserState, ParseError};

// 🔴 悪い——何をテストしているのかわからない
use super::*;
```

### 命名規範

**ルール 3.1**：テスト関数名は `test_<what>_<scenario>` 形式で、全小文字アンダースコア区切りです。

```rust
#[test]
fn test_tokenize_empty_string() { /* ... */ }
#[test]
fn test_parse_int_overflow() { /* ... */ }
#[test]
fn test_typecheck_fn_return_mismatch() { /* ... */ }
```

**ルール 3.2**：テスト関数名は自己説明的である必要があります。関数名を読めば何をテストし何を期待しているかがわかるべきです。数字の連番による命名は禁止です。

```rust
// 🟢 良い
fn test_skip_semicolon_success() { /* ... */ }
fn test_skip_semicolon_failure_when_identifier() { /* ... */ }

// 🔴 悪い——何のテストかわからない
fn test_skip_1() { /* ... */ }
fn test_skip_2() { /* ... */ }
```

**ルール 3.3**：ヘルパー関数には `test_` プレフィックスは不要で、その用途を説明する動詞または名詞を使用すべきです。

```rust
fn parse_expr(source: &str) -> Expr { /* ... */ }
fn tokenize_single(source: &str) -> Token { /* ... */ }
fn setup_parser_with_tokens(tokens: &[Token]) -> ParserState { /* ... */ }
```

### テスト構造規範 (Arrange-Act-Assert)

**ルール 4.1**：各テスト関数は三段構造に従う必要があります：準備（Arrange）→ 実行（Act）→ アサーション（Assert）、三段の間は空行で区切ります。

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

**ルール 4.2**：単純なテスト（単一呼び出し + 単一アサーション）は分段コメントを省略できますが、5 行以上のロジックコードを含めることはできません。5 行を超えるテストは三段を明示的に示す必要があります。

### ヘルパー関数規範

**ルール 5.1**：3 回以上繰り返し出現する setup ロジックはヘルパー関数に抽出する必要があります。

```rust
// 🟢 良い——公共の setup を抽出
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

**ルール 5.2**：ヘルパー関数内の `unwrap()` / `expect()` は panic 時に十分なコンテキストを出力する必要があります。テスト関数本体（`#[test] fn ...`）では直接 `unwrap()` を使用できます——失敗時に Rust が自動的に行番号を出力するため；但しヘルパー関数内で失敗した場合には行番号はヘルパー関数の定義位置を指し、呼び出し時のコンテキストが見えません。

```rust
// 🟢 良い——ヘルパー関数失敗時にソースコードの内容を出力
fn run_ok(source: &str) {
    run(source).unwrap_or_else(|e| panic!("実行に失敗:\nソース:\n{}\nエラー:\n{:?}", source, e));
}

// 🔴 悪い——失敗時にどのソースファイルが問題を起こしたかわからない
fn run_ok(source: &str) {
    run(source).unwrap();
}
```

**ルール 5.3**：ヘルパー関数はテストファイルの先頭、`use` インポートの直後に配置する必要があります。複数のテストモジュールで共有される場合、`tests/mod.rs` に配置し `pub(crate)` でエクスポートします。

### アサーションスタイル

**ルール 6.1**：enum 変体のマッチングは `assert!(matches!(...))` を使用することを優先し、`if let` + `panic!` は使用禁止です。

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

**ルール 6.2**：精密な値の比較は `assert_eq!` を使用し、ブール値のアサーションは `assert!` を使用します。`assert!(a == b)` を `assert_eq!(a, b)` の代わりに使用することは禁止です。

**ルール 6.3**：すべてのアサーションにはカスタムエラーメッセージを含める必要があります，除非アサーション自体が失敗理由を完全に説明している場合。

```rust
// 🟢 良い——アサーション失敗時に素早く特定できる
assert!(
    state.infix_info().is_some(),
    "infix_info should handle '{op}'"
);

// 🟢 良い——assert_eq! 失敗時に自動的に値の差分を出力、追加メッセージ不要
assert_eq!(error_count, 0);

// 🔴 悪い——失敗時に "assertion failed" しかわからない
assert!(state.infix_info().is_some());
```

**ルール 6.4**：アサーションの順序は `assert_eq!(actual, expected)` とし、実際の値を前、期待値を後にする必要があります。

### アンチパターンのリスト

以下は禁止の写法とその代替案です：

| アンチパターン                                       | 問題                               | 代替案                                                             |
| -------------------------------------------------- | ---------------------------------- | ---------------------------------------------------------------- |
| `#[cfg(test)] mod tests { ... }` インラインテスト | ソースファイル肥大化、モジュール境界曖昧化、リファクタ困難 | テストコードを独立の `tests/` ディレクトリに置き、`mod tests;` で宣言（ルール 1.4 参照） |
| テストがコードのエラー動作に忖度                       | 規範偏差を隠蔽し、bug を合法化        | 規範に照らしてコードを修正し、テストはそのまま保持                         |
| コード出力を基にテストの期待値を逆算                   | テストが「現在実装の録音機」になる      | 規範から期待値を導出                                                  |
| `#[ignore]` 永続マーク                             | 腐敗したテストを隠蔽                 | 修復または削除                                                      |
| `println!` デバッグ出力                             | テスト出力を汚染                    | `assert!` で明確にアサーション                                       |
| `thread::sleep`                                    | ランダム失敗 + 低速                  | 同期メカニズムまたは mock を使用                                      |
| テストで реальный ファイルシステムを操作               | 低速で再現不可                      | `tempfile` を使用                                                   |
| テスト実行順序に依存                                 | ランダム失敗                       | 各テストは独立した setup                                             |
| テスト関数하나が 30 行以上のロジック                   | 理解不能                           | テストを分割またはヘルパー関数を使用                                    |
| ヘルパー関数内の `unwrap()` がコンテキストなし         | 特定困難                           | `expect("理由")` またはカスタム panic を使用（ルール 5.2 参照）          |
| 同一 setup を copy-paste で 3 回以上                 | 修改コスト高昂                      | ヘルパー関数を抽出                                                    |

---

## 統合テスト規範

### テスト組織

**ルール 7.1**：統合テストはプロジェクトルートの `tests/` ディレクトリに配置します。入口ファイル `tests/integration.rs` は `#[path]` 属性を使用してサブモジュールを導入します。

```rust
// tests/integration.rs
#[path = "integration/backends.rs"]
mod backends;
#[path = "integration/codegen.rs"]
mod codegen;
#[path = "integration/execution.rs"]
mod execution;
```

**ルール 7.2**：各 `tests/integration/*.rs` ファイルは1つのテストテーマ（コンパイラバックエンド、コード生成、エグゼキュータなど）に対応し、混在させてはいけません。

**ルール 7.3**：統合テストはプロジェクトのパブリック API を通じてテストする必要があります。統合テストで `crate::` 内部モジュールを直接参照することは禁止です。`yaoxiang::` パブリックパスを使用してください。

```rust
// 🟢 良い——パブリック API 経由
use yaoxiang::run;

// 🔴 悪い——パブリック API 境界を迂回
use yaoxiang::middle::codegen::bytecode::BytecodeFile;
```

### テストデータ管理

**ルール 8.1**：統合テストはまずインラインソースコード文字列を使用します。ソースコードが 30 行を超える場合にのみ、外部 fixture ファイル（`tests/fixtures/` に配置）を使用します。

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

**ルール 8.2**：fixture ファイルは `.yx` 拡張子で終わらせ、ファイル名はテストの意図を説明します。

### E2E 覆蓋原則

**ルール 9.1**：各言語機能の統合テストは以下の三つのパスをカバーする必要があります：

| パス         | 説明                                       |
| ------------ | ------------------------------------------ |
| Happy path   | 合法な入力に対して期待される出力を生成         |
| Error path   | 非法な入力に対して明確なエラーメッセージを生成（非 panic） |
| Boundary     | 境界値（空入力、最大値、ネスト深度上限）        |

**ルール 9.2**：統合テストはネットワーク、システム環境変数、外部サービスに依存してはなりません。

---

## ベンチマークテスト規範

### Criterion.rs 使用規範

**ルール 10.1**：ベンチマークテストは `benches/` ディレクトリに統一して配置し、入口ファイルは `benches/lib.rs` です。テストテーマごとにファイルを分割します。

```
benches/
├── lib.rs              # 入口、criterion_group/criterion_main を定義
├── lang_compare/
│   └── fibonacci.rs    # 言語間比較ベンチマーク
├── parser.rs           # パーサーバンチマーク
└── codegen.rs          # コード生成ベンチマーク
```

**ルール 10.2**：各ベンチマーク関数にはテスト目的と測定指標を説明するモジュールドキュメントコメント `//!` を含める必要があります。

```rust
//! YaoXiang インタープリタ性能ベンチマークテスト
//!
//! 測定指標：単一イテレーション所要時間（wall time）
//! ベースライン：Rust ネイティブ実装
```

### コンパイラ最適化防止

**ルール 11.1**：すべてのベンチマークテストの対象出力は `criterion::black_box` を使用してコンパイラの最適化による消去を防ぎます。

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

**ルール 11.2**：ベンチマークテストの入力データは `const` または `lazy_static` である必要があり、`iter` クロージャ内で動的に生成してはなりません——さもなくば測定されるのはデータ生成 + 被テストロジックの合計時間になります。

### ベンチマークグループ分けと命名

**ルール 12.1**：ベンチマークテストの命名形式は `<被テストモジュール>_<シナリオ>` で、全小文字アンダースコア区切りです。ユニットテストの命名規則と一貫性を持たせます。

**ルール 12.2**：`criterion_group!` を使用して関連するベンチマークをロジックグループにまとめる必要があります。すべてのベンチマークを一つのグループに押し込めることは禁止です。

```rust
criterion_group!(parser, bench_parse_expr, bench_parse_stmt);
criterion_group!(codegen, bench_codegen_module, bench_codegen_switch);
criterion_main!(parser, codegen);
```

---

## ドキュメントテスト規範

### 使用シナリオ

**ルール 13.1**：すべての `pub` 関数、型、メソッドのドキュメントコメントには少なくとも1つの実行可能なコード例を含める必要があります。この例は `cargo test --doc` で実行されます。

````rust
/// ソースコード文字列を Token シーケンスに字句解析します。
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
````

**ルール 13.2**：ドキュメントテストのコード例はコンパイル成功とアサーション成功が必要です。コンパイル時エラーを示す例でない限り、`ignore` マークのある例を含めないでください。

````rust
/// ```ignore
/// // コンパイル時エラーを示す——ignore 可能
/// let x: int = "string";
/// ```
````

### 覆蓋要件

**ルール 14.1**：ドキュメントテストは API の happy path のみをカバーすれば十分です。境界情况和エラー経路はユニットテストがカバーします。

**ルール 14.2**：ドキュメントテストのコード例は簡潔である必要があります——10 行を超えないこと。例により長いコンテキストが必要な場合は、API 設計に問題があることを示しています。

---

## プロパティテスト規範

### 使用シナリオ

**ルール 15.1**：以下のシナリオでは、手書きの複数の境界値ケースではなく、プロパティテスト（proptest または quickcheck）を使用する必要があります：

| シナリオ               | 例                                        |
| -------------------- | ---------------------------------------- |
| パーサー round-trip   | `parse(pretty_print(ast)) == ast`        |
| シリアライズ/デシリアライズ | `deserialize(serialize(data)) == data` |
| 数学演算の恒等式       | `a + b == b + a`                         |
| コンパイラ最適化が意味を変えない | `eval(code) == eval(optimize(code))` |

**ルール 15.2**：プロパティテストでは `proptest` を主要なプロパティテストフレームワークとして使用します（`Cargo.toml` の `dev-dependencies` で宣言済み）。

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

**ルール 16.1**：各プロパティテストには明確なプロパティ宣言が必要です——コメントに検証する不変量を明記します。

```rust
// プロパティ：任意の整数のリテラルが tokenize → tokens_to_string の後に同じ値を生成
proptest! {
    #[test]
    fn test_int_literal_roundtrip(n in any::<i64>()) {
        let source = n.to_string();
        let tokens = tokenize(&source).unwrap();
        // ...
    }
}
```

**ルール 16.2**：プロパティテストが失敗を発見した場合、`proptest` のリグレッション機構を使用する必要があります——失敗した入力を `proptest-regressions/` ディレクトリに追加し、普通の手書きテストで代用しないでください。

---

## カバレッジ要件

### 新規コードのカバレッジ目標

**ルール 17.1**：新規コードのテストカバレッジ要件：

| コードタイプ                               | 行カバレッジ | 分岐カバレッジ |
| ------------------------------------------ | ----------- | ------------- |
| コアコンパイラモジュール（frontend/middle/backends） | ≥ 85%       | ≥ 80%         |
| ユーティリティ/補助モジュール（util）              | ≥ 75%       | ≥ 70%         |
| ランタイムモジュール（vm/runtime）               | ≥ 80%       | ≥ 75%         |
| 標準ライブラリ（std）                          | ≥ 75%       | ≥ 70%         |
| エラー処理と診断                              | ≥ 90%       | ≥ 85%         |

**ルール 17.2**：エラー処理経路（すべての `Err` 分岐）は 100% カバレッジが必要です。ユーザーが見るエラーメッセージはテストで検証されていなければなりません。

### PR レビューチェックリスト

**ルール 18.1**：PR を送信する前に、作者は以下の項目を自查する必要があります：

- [ ] `cargo test` がすべて合格
- [ ] `cargo test --doc` がすべて合格
- [ ] `cargo bench` にパフォーマンスリグレッションなし（ホットパス変更を伴う場合）
- [ ] 新規コードがカバレッジ目標に合致
- [ ] テスト名が命名規範に合致
- [ ] 各テストファイルが対応する規範セクションを宣言（ルール 2.1）
- [ ] テストの期待値が「現在のコードの出力」ではなく規範定義から来ている
- [ ] `#[ignore]` マークのテストがない（明確な issue 番号コメントがある場合を除く）
- [ ] 不必要な `unwrap()` がない（`expect` またはカスタム panic メッセージを使用すべき）
- [ ] コミットメッセージが `:white_check_mark: test:` タイプを使用
- [ ] **「コードの動作が規範不符」を理由にテスト期待値を修正していない——修正するのはコードであり、テストではない**
- [ ] **インラインテ스트がない**（`#[cfg(test)] mod tests { ... }` を `mod tests;` + 独立ファイルに変更、ルール 1.4 参照）

**ルール 18.2**：Reviewer は以下の問題を含む PR を拒否する必要があります：

- happy path テストのみ、エラー経路がない
- テストに `thread::sleep` または実行順序への依存がある
- 3 回以上 copy-paste されたテストコードがあり、ヘルパー関数を抽出していない
- テスト名が命名規範不合致
- 永続的な `#[ignore]` テストが存在する
- **テストがコードのエラー動作に忖度している**（コードと規範不符時にテスト而非コードを修正）
- **テストが対応する規範セクションを宣言していない**（ルール 2.1 参照）
- **テスト期待値がコード出力而非規範定義から来ている**（逆算されたテストはテストとして機能しない）
- **インラインテストが存在する**（`#[cfg(test)] mod tests { ... }` 而非 `mod tests;` + 独立ファイル、ルール 1.4 参照）
- テストが「panic しない」だけを検証し、具体的な動作をアサートしていない
- コードバグを露呈した失敗テストを削除した（而非コードを修復してから緑信号を確認）

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

# テスト出力を表示（デフォルトは非表示）
cargo test -- --nocapture

# 単一スレッドで実行（並行問題排查）
cargo test -- --test-threads=1

# カバレッジレポートを生成（cargo-llvm-cov が必要）
cargo llvm-cov --html
```

### B. コミットメッセージテンプレート

テスト関連コミットは以下のテンプレートに従う必要があります：

```
:white_check_mark: test(<scope>): <簡単な説明>

<オプション：カバーするシナリオリスト>
```

例：

```
:white_check_mark: test(parser): Pratt パーサーの中置演算子テストを追加

カバーするシナリオ：
- 算術演算子の優先順位（+, -, *, /, %）
- 比較演算子のチェーン（1 < x < 10）
- 論理演算子のショートサーキット
- 代入演算子の右結合
```

### C. 新規テストファイルチェックリスト

新しいテストモジュールを作成する際、以下のファイルを含める必要があります：

```
# src/<module>/ ディレクトリ下に新規テスト
src/<module>/tests/
├── mod.rs          # モジュール宣言 + 公共ヘルパー関数
└── <subject>.rs    # テストファイル、被テストソースファイルの命名に対応

# tests/ ディレクトリ下に新規統合テスト
tests/
├── integration.rs   # 更新：#[path] 宣言を追加
└── integration/
    └── <topic>.rs   # 新規テストファイル
```

### D. 参考資料

- [YaoXiang 言語規範](../../design/language-spec.md) —— **テストの権威あるソース**
- [承認済み RFC](../../design/rfc/accepted/) —— **設計決定の権威あるソース**
- [Rust テストドキュメント](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Criterion.rs ユーザーガイド](https://bheisler.github.io/criterion.rs/book/)
- [proptest ドキュメント](https://docs.rs/proptest/latest/proptest/)
- [プロジェクトコミット規範](./commit-convention.md)
- [プロジェクトコントリビューションガイド](./contributing.md)

---

> 💡
> **覚えておいてください**：テストはコードが「動作するかどうか」を検証するのではなく、コードが規範是否符合かを検証します。規範が変わりえれば、テストも規範に従って変わります。コードが間違っていれば、コードを修正し、テストを修正してはなりません。**コードは規範に奉仕し、テストは規範を守ります。テストがコードに忖度的那一刻、あなたはすべての保護を失います。**

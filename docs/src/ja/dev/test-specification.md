---
title: 'テスト作成規範'
description:
  YaoXiang
  プロジェクトのテスト作成ハード規範。ユニットテスト、結合テスト、ベンチマークテスト、ドキュメントテスト、プロパティテストの作成基準を定義する
---

# テスト作成規範

本文書は YaoXiang プロジェクトのテスト作成ハード規範を定義する。すべての貢献者は以下のルールを遵守しなければならず、違反者はコードレビューで修正を要求される。

---

## 目次

- [総則](#総則)
- [yx コーパスとライブラリテスト階層](#yx-コーパスとライブラリテスト階層)
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

本規範は YaoXiang プロジェクト内のすべての Rust テストコードに適用される。

| テスト種別         | 位置                     | フレームワーク             |
| ------------------ | ------------------------ | -------------------------- |
| ユニットテスト     | `src/<module>/tests/`    | `#[test]` + `#[cfg(test)]` |
| 結合テスト         | `tests/`                 | `#[test]`                  |
| ベンチマークテスト | `benches/`               | Criterion.rs               |
| ドキュメントテスト | API ドキュメントコメント | `cargo test --doc`         |
| プロパティテスト   | 任意のテスト位置         | proptest / quickcheck      |

### 核心原則

**原則 0：テストの権威ある情報源は規範であり、コードではない。**
これは本文書で最も重要な原則である。テストが検証するのは、コードが規範に合致しているかどうかであり、コードが「現在の実装で動作している」かどうかではない。テストによってコードの挙動が規範と一致していないことが発見された場合、**テストを修正するのではなく、コードを修正する**。

規範ファイルの位置：

- `docs/src/design/language-spec.md` —— 言語コア規範
- `docs/src/design/rfc/accepted/` —— 承認済み RFC 設計文書

各テストファイルの冒頭には、対応する規範の章を宣言しなければならない（ルール 2.1 を参照）。いかなる開発者も規範文書とテストを照合し、実装の正当性を検証できるべきである。逆に言えば、規範による記述を持たないコードは存在すべきではなく、ましてやテストされるべきでもない。

```rust
// 🟢 良い例——テストは規範を直接参照し、コードが規範に従っているかを検証する
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

// 🔴 悪い例——テストは現在のコードの実装動作に迎合しており、規範を検証していない
#[test]
fn test_literal_1() {
    // このコードが規範のどの節に対応するか不明
    // もし parse_literal が誤った値を返しても、このテストは「緑で通過」する
    // 関数がパニックしないことしか検証していないため
    let result = parse_literal("42");
    assert!(result.is_ok());
}
```

**シナリオ**：テストを書いて、コードの挙動が規範と合わないことに気づいた。2 つの選択肢がある：

| 誤った方法                                     | 正しい方法                                 |
| ---------------------------------------------- | ------------------------------------------ |
| テストを修正して「通過」させる                 | コードを修正して挙動を規範に合わせる       |
| テストに `#[ignore]` を追加する                | 直ちにコード実装を修正する                 |
| テストに特殊条件分岐を追加してコードに迎合する | 分岐を削除し、テストに直接問題を露出させる |

**赤信号 = コードが間違っている、テストが間違っているのではない。**（テスト自体にバグがある場合は別だが、それは別の話だ。）

**原則 1：テストはドキュメントである。**
いかなる開発者も、テストを読むだけで被テストコードの挙動を理解できるべきであり、追加のコメントや外部ドキュメントを必要としてはならない。

```rust
// 🟢 良い例——テスト名は何をテストし何を期待しているかを語っている
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

**原則 2：不規則な失敗のゼロトレランス。**
テストはどんな環境でも再現可能に実行できなければならない。乱数、システム時刻、スレッドスケジューリング順序に依存するテストは、シード固定を使用するかモックで代替しなければならない。

**原則 3：1 つのテストは 1 つの事柄だけをテストする。**
テスト名に「と」で複数の挙動を接続する必要があるなら、複数のテストに分割する。

```rust
// 🟢 良い例——各テストは 1 つのシナリオのみを検証する
#[test]
fn test_parse_int_positive() { /* ... */ }
#[test]
fn test_parse_int_zero() { /* ... */ }

// 🔴 悪い例——1 つのテストに無関係な内容が詰め込まれている
#[test]
fn test_parser() {
    // tokenize、parse、typecheck、codegen をテスト...
}
```

**原則 4：挙動をテストし、実装をテストしない。**
内部実装をリファクタリングしてもテストが失敗してはならない。実装コードを 1 行変更して 10 個のテストが失敗するなら、テストの書き方が間違っている。

しかし、ここに重要な区別がある：**「挙動」の定義は規範から来るものであり、現在のコードの表現から来るものではない。**
コードが挙動を変更した場合（すなわち規範に合わない新しい挙動）、テストは必ず失敗しなければならない。これが達成できないなら、そのテストは「コードに迎合するテスト」である——バグの侵入を許してしまう。

```
規範（language-spec.md / RFC）  ──定義──►  期待挙動  ──駆動──►  テスト
                                          │
現在のコード  ──実装──►  実際の挙動  ──対比──►  テスト結果

実際の挙動 ≠ 期待挙動の場合：
  テストは必ず失敗（赤信号）──► コードを修正 ──► テスト通過（緑信号）

実際の挙動 = 期待挙動（ただし実装が酷い）：
  テスト通過 ──► 実装をリファクタ ──► テストは依然通過  ← これが原則 4 の意味するところ
```

**原則 5：後退・互換・特定パターン有効化のテストコードは書かない。**
テスト環境は完全に制御できる環境である。もし `#[cfg(not(ci))]`
を使ってあるテストをスキップする必要があるなら、そのテスト設計には根本的な問題がある。

### 用語定義

| 用語               | 定義                                                                          |
| ------------------ | ----------------------------------------------------------------------------- |
| ユニットテスト     | 単一の関数またはモジュールの挙動をテストし、外部システムに依存しない          |
| 結合テスト         | 複数のモジュールの協調をテストし、公開 API またはコマンドラインエントリを介す |
| ベンチマークテスト | コード性能を測定し、性能退化を検出する                                        |
| ドキュメントテスト | ドキュメントコメントに埋め込まれた実行可能なコード例                          |
| プロパティテスト   | ランダム入力に基づいて不変量（property）を検証するテスト                      |

### コミット規範との関連

すべてのテスト関連のコミットは `:white_check_mark: test:`
タイプを使用しなければならず、[コミット規範](./commit-convention.md)を参照する。

```
:white_check_mark: test(parser): Pratt パーサの中置式テストを追加
:white_check_mark: test(codegen): switch 文の IR 生成テストを補完
```

---

## yx コーパスとライブラリテスト階層

本規範は **Rust 側テストコード** を制約する。YaoXiang 言語自身のテスト（`.yx`
コーパスとライブラリテスト）は被テスト対象によって 2 階層に分けられ、体系設計と判定契約は RFC-036（§7 スイート収集 /
§8 負の 3 階層 / §9 テスト体系階層）に属し、コーパス作成細則は `tests/yaoxiang/TEST_STANDARDS.md`
に属する：

- **言語可用性コーパス**（`tests/yaoxiang/`）——被テスト対象は言語自体；std はアサーションの道具としてのみ使われる。コーパス内では失敗発生層によって 3 種類の判定に分ける：挙動テスト / コンパイル期拒否テスト / 実行期失敗テスト
- **ライブラリテスト**（ライブラリに随伴）——被テスト対象はライブラリの公開 API 契約；std の yx レベルテストは
  `src/std/tests/` に位置し、将来のユーザーパッケージのテストはパッケージ内の `[tool.test]`
  で発見される

`.yx` テストのファイルヘッダ形式、ヘッダ指令（`// expect:` / `// skip:` / `// mode:`、RFC-036
§8.2）とアサーション規約は TEST_STANDARDS.md に準拠する；判定解析は双 runner 共有の
`src/util/test_markers.rs`（Rust 側、本規範の制約を受ける）で実装される。

---

## ユニットテスト規範

### ファイル構成

**ルール 1.1**：ユニットテストの `tests/` ディレクトリは被テストモジュールの `mod.rs`
と**同じ階層**になければならない。`tests/` は上位に集約せず、階層をまたいでまとめない。

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

重要な判断基準：**`tests/` をどのディレクトリに置くか、そのディレクトリの `mod.rs` は必ず
`#[cfg(test)] mod tests;` で宣言しなければならない。**

**ルール 1.1 補足：上位への集約を禁止する。**
サブディレクトリモジュールのテストはそのサブディレクトリ自身の `tests/`
に置かなければならず、上の階層の `tests/` に集約してはならない。

| モジュール種別                          | テスト位置                    | 例                                           |
| --------------------------------------- | ----------------------------- | -------------------------------------------- |
| ディレクトリモジュール（`mod.rs` あり） | 当該ディレクトリ下の `tests/` | `emitter/tests/`、`codes/tests/`             |
| 単一ファイルモジュール（`.rs` のみ）    | 親階層の `tests/`             | `session.rs` → `diagnostic/tests/session.rs` |

```text
# ✅ 正しい：各ディレクトリモジュールのテストはそれぞれ独立している
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

**核心的な違い**：モジュールの組織形式がテストの配置位置を決定する。

| モジュール種別             | 判断基準                                 | テスト位置                    | 例                                            |
| -------------------------- | ---------------------------------------- | ----------------------------- | --------------------------------------------- |
| **ディレクトリモジュール** | 独立したディレクトリと `mod.rs` を持つ   | 当該ディレクトリ下の `tests/` | `inference/tests/`                            |
| **単一ファイルモジュール** | `.rs` ファイルのみ、独立ディレクトリなし | 親階層の `tests/`             | `overload.rs` → `typecheck/tests/overload.rs` |

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
│   ├── overload.rs                 # overload.rs のテスト（単一ファイルモジュールのテストはここに置く）
│   ├── type_eval.rs                # type_eval.rs のテスト
│   ├── dead_code.rs                # dead_code.rs のテスト
│   ├── spawn_placement.rs          # spawn_placement.rs のテスト
│   ├── signature.rs                # signature.rs のテスト
│   └── types.rs                    # types.rs のテスト
│
├── inference/                      # ディレクトリモジュール（mod.rs あり）
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
└── traits/                         # 削除済み（ロジックは types/trait_data.rs にマージ）
```

**なぜ単一ファイルモジュールのテストを親階層の `tests/` に置くのか？**

単一ファイルモジュール（例：`overload.rs`）は独自の `mod.rs`
を持たないため、`#[cfg(test)] mod tests;`
を宣言できない。Rust のモジュールシステムによると、テストファイルは何らかの `mod.rs`
によって宣言されなければコンパイルされない。したがって、単一ファイルモジュールのテストは親階層の
`mod.rs` によって宣言され、親階層の `tests/` ディレクトリに置かれるしかない。

**判断フロー**：

```
あるモジュールに遭遇し、テストをどこに置くか判断する
│
├── そのモジュールはディレクトリ（mod.rs あり）か？
│   └── はい → そのディレクトリ下に tests/ を作成し、当該ディレクトリの mod.rs で宣言
│
├── そのモジュールは単一ファイル（.rs のみ）か？
│   └── はい → 親階層の tests/ ディレクトリにテストを置き、親階層の mod.rs で宣言
│
└── 不確か？
    └── 独立したディレクトリと mod.rs があるか確認
```

**よくある誤り**：

```
# ❌ 誤り 1：単一ファイルモジュールに対して独立した tests/ ディレクトリを作成
src/frontend/core/typecheck/
├── overload.rs
└── overload/                       # ❌ 単一ファイルモジュールに対してディレクトリを作成すべきでない
    └── tests/
        └── overload.rs

# ❌ 誤り 2：単一ファイルモジュール内で #[cfg(test)] mod tests; を宣言
# overload.rs
#[cfg(test)]                        # ❌ 単一ファイルモジュールではこのように宣言できない
mod tests;                          # overload/tests/ ディレクトリがないため

# ✅ 正しい方法：テストを親階層の tests/ に置く
src/frontend/core/typecheck/
├── overload.rs                     # ソースファイル
└── tests/
    └── overload.rs                 # テストファイル、typecheck/mod.rs によって宣言される
```

⚠️ **アンチパターン——こう書いてはいけない：**

```
# ❌ 誤り：サブモジュールのテストを親階層に集中させる
src/frontend/core/types/
├── mod.rs              # 本来 base と computation のみを宣言すべき
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
# ✅ 正しい方法：各モジュールのテストはそれぞれ独立
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

**なぜ上位に集約できないのか？** Rust のモジュールシステムは `#[cfg(test)] mod tests;`
宣言位置でテストファイルのコンパイルを決定するため。もし `types/mod.rs` が `mod tests;`
を宣言したなら、`types/tests/` の内容は `types` モジュールのプライベート内容となる——それは `base` や
`computation`
の領分に踏み込んではならない。各モジュールのテストはそのモジュールの内部実装詳細であるべきであり、親モジュールのそれではない。このルールはモジュールのリファクタリングにも同様に適用される：`types`
を `base` と `computation`
に分割するとき、テストも分割後のモジュールに従って分割されるべきであり、元の場所に残されるべきではない。**テストディレクトリはソースコードの構造をミラーするのではなく、モジュール境界に従う。**

**ルール 1.2**：`tests/mod.rs` はモジュール宣言と re-export のみを担当し、テスト関数を置かない。

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

**ルール 1.3**：各テストファイルは 1 つのソースファイルにのみ対応する。複数のソースモジュールのテストを 1 つのファイルに混在させてはならない。

**ルール 1.4**：テスト宣言はファイル形式
`mod tests;`（セミコロン付き）を使用しなければならず、同階層の `tests/`
ディレクトリを指す。**インライン形式 `mod tests { ... }`
を使ってテストコードをソースファイル内に直接書くことを禁止する。**

```rust
// ✅ 正しい——ファイル形式宣言、テストコードは独立したファイルに
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
        // テストコードはソースファイル内に存在すべきでない
    }
}
```

**なぜインラインを禁止するのか？**

1. ソースファイルの責務が単一：ソースファイルは実装のみ、テストファイルはテストのみ。一緒に混ぜると、テスト変更時にファイル末尾までスクロールし、実装変更時にテストをスキップすることになる。
2. モジュール境界が明確：`tests/`
   ディレクトリは物理的な境界であり、どのモジュールにテストがあり、ないかが一目瞭然。
3. リファクタリングが安全：モジュール分割時、`tests/`
   ディレクトリも一緒に移動する；インラインテストはソースファイルから手動で剥離する必要がある。
4. コードレビュー：PR diff でソースコード変更とテスト変更が別ファイルに分離され、混ざらない。

### モジュール宣言規範

**ルール 2.1**：すべてのテストファイルの冒頭にはモジュールレベルドキュメントコメント `//!`
がなければならず、テストがカバーする規範の出所（言語規範章番号 +
RFC 番号）を説明する。あるテストがいかなる規範の章も参照していないなら、そのコードには規範上の根拠がない——それは存在すべきではない。

```rust
//! リテラルテスト — 言語規範 §2.6 に基づく
//!
//! §2.6.1: 整数 Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: 浮動小数点数（小数点と指数を含む）
//! §2.6.3: 文字列（エスケープシーケンス \\nrt'"\\, \\x, \\u{}）
//! RFC-012: F-String 補間
```

**なぜ規範を参照しなければならないのか？**
テストの期待値は規範から来るものであり、「現在のコードの出力」から来るものではない。いつかコードの出力が変わってテストがそれに追随するなら、そのテストは何も保護していない。規範に紐づいたテストだけが「意図的な breaking
change」と「意図しない退化」を区別できる。

**ルール 2.2**：テストモジュールの `use`
インポートは具体的な型/関数まで正確でなければならず、glob インポート `use super::*` を禁止する。

```rust
// 🟢 良い例——正確なインポート
use crate::frontend::core::lexer::{tokenize, TokenKind};
use crate::frontend::core::parser::{ParserState, ParseError};

// 🔴 悪い例——何をテストしているかが他人にはわからない
use super::*;
```

### 命名規範

**ルール 3.1**：テスト関数命名形式は `test_<what>_<scenario>`、すべて小文字でアンダースコア区切り。

```rust
#[test]
fn test_tokenize_empty_string() { /* ... */ }
#[test]
fn test_parse_int_overflow() { /* ... */ }
#[test]
fn test_typecheck_fn_return_mismatch() { /* ... */ }
```

**ルール 3.2**：テスト関数名は自己説明的でなければならない。関数名を読んだだけで何をテストし何を期待しているかがわかること。数字による連番命名を禁止する。

```rust
// 🟢 良い例
fn test_skip_semicolon_success() { /* ... */ }
fn test_skip_semicolon_failure_when_identifier() { /* ... */ }

// 🔴 悪い例——何をテストしているかが完全にわからない
fn test_skip_1() { /* ... */ }
fn test_skip_2() { /* ... */ }
```

**ルール 3.3**：ヘルパー関数は `test_` 接頭辞を必要とせず、動詞または名詞で用途を記述すべきである。

```rust
fn parse_expr(source: &str) -> Expr { /* ... */ }
fn tokenize_single(source: &str) -> Token { /* ... */ }
fn setup_parser_with_tokens(tokens: &[Token]) -> ParserState { /* ... */ }
```

### テスト構造規範 (Arrange-Act-Assert)

**ルール 4.1**：各テスト関数は三段式構造に従わなければならない：準備（Arrange）→ 実行（Act）→ 検証（Assert）、三段の間は空行で区切る。

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

**ルール 4.2**：簡単なテスト（単一呼び出し + 単一アサーション）は段分けコメントを書かなくてよいが、論理コード 5 行を超過してはならない。5 行を超えるテストは明示的に三段を標示しなければならない。

### ヘルパー関数規範

**ルール 5.1**：3 回以上繰り返される setup ロジックはヘルパー関数として抽出しなければならない。

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

**ルール 5.2**：ヘルパー関数内の `unwrap()` / `expect()`
はパニック時に十分なコンテキストを出力しなければならない。テスト関数本体内（`#[test] fn ...`）では直接
`unwrap()`
できる——失敗時に Rust が自動的に行番号を出力するため。しかしヘルパー関数内で失敗した場合、行番号はヘルパー関数の定義位置を指し、呼び出し時のコンテキストがわからない。

```rust
// 🟢 良い例——ヘルパー関数失敗時にソース内容を出力
fn run_ok(source: &str) {
    run(source).unwrap_or_else(|e| panic!("Execution failed:\nSource:\n{}\nError:\n{:?}", source, e));
}

// 🔴 悪い例——失敗時にどのソースファイルが原因かが見えない
fn run_ok(source: &str) {
    run(source).unwrap();
}
```

**ルール 5.3**：ヘルパー関数はテストファイルの冒頭、`use`
インポートの直後に置くべきである。複数のテストモジュールで共有される場合、`tests/mod.rs` に置いて
`pub(crate)` でエクスポートする。

### アサーションスタイル

**ルール 6.1**：列挙バリアントの一致には `assert!(matches!(...))` の使用を優先し、`if let` +
`panic!` の使用を禁止する。

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

**ルール 6.2**：正確な値の比較には `assert_eq!` を使用し、論理アサーションには `assert!`
を使用する。`assert_eq!(a, b)` の代わりに `assert!(a == b)` を使うことを禁止する。

**ルール 6.3**：すべてのアサーションにはカスタムエラーメッセージを付けること。ただし、アサーション自体が失敗原因を完全に記述している場合は例外。

```rust
// 🟢 良い例——アサーション失敗時に迅速に原因を特定できる
assert!(
    state.infix_info().is_some(),
    "infix_info should handle '{op}'"
);

// 🟢 良い例——assert_eq! 失敗時に値の差分が自動出力されるため追加メッセージ不要
assert_eq!(error_count, 0);

// 🔴 悪い例——失敗時に「assertion failed」としかわからない
assert!(state.infix_info().is_some());
```

**ルール 6.4**：アサーションの順序は `assert_eq!(actual, expected)`
でなければならず、実際の値が前、期待値が後。

### アンチパターン一覧

以下は禁止されている書き方と、その代替案である：

| アンチパターン                                         | 問題                                                             | 代替案                                                                                     |
| ------------------------------------------------------ | ---------------------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| `#[cfg(test)] mod tests { ... }` インラインテスト      | ソースファイル膨張、モジュール境界の曖昧化、リファクタリング困難 | テストコードを独立した `tests/` ディレクトリに置き、`mod tests;` で宣言（ルール 1.4 参照） |
| コードの誤った挙動にテストが迎合する                   | 規範偏差を覆い隠し、バグを合法化                                 | 規範と照らし合わせてコードを修正し、テストは変更しない                                     |
| コード出力から逆算してテスト期待値を決める             | テストが「現在の実装の録音機」になる                             | 規範から期待値を導く                                                                       |
| 永久 `#[ignore]` マーキング                            | 腐ったテストを隠蔽                                               | 修正または削除                                                                             |
| `println!` デバッグ出力                                | テスト出力を汚染                                                 | `assert!` で明示的にアサート                                                               |
| `thread::sleep`                                        | 不規則な失敗 + 低速                                              | 同期機構またはモックを使用                                                                 |
| テストで実際のファイルシステムを操作                   | 遅く再現不可能                                                   | `tempfile` を使用                                                                          |
| テスト実行順序に依存                                   | 不規則な失敗                                                     | 各テスト独立に setup                                                                       |
| 1 つのテスト関数が 30 行超の論理                       | 誰も理解できない                                                 | テストを分割するかヘルパー関数を使用                                                       |
| ヘルパー関数内の `unwrap()` がコンテキストを出力しない | 原因特定の困難                                                   | `expect("why")` またはカスタム panic を使用（ルール 5.2 参照）                             |
| 同じ setup を 3 回以上コピペ                           | 修正コストが高い                                                 | ヘルパー関数を抽出                                                                         |

---

## 結合テスト規範

### テスト構成

**ルール 7.1**：結合テストはプロジェクトルートディレクトリの `tests/`
ディレクトリに置く。エントリファイル `tests/integration.rs` は `#[path]`
属性を使ってサブモジュールをインクルードする。

```rust
// tests/integration.rs
#[path = "integration/backends.rs"]
mod backends;
#[path = "integration/codegen.rs"]
mod codegen;
#[path = "integration/execution.rs"]
mod execution;
```

**ルール 7.2**：各 `tests/integration/*.rs`
ファイルは 1 つのテストテーマ（コンパイラバックエンド、コード生成、エグゼキュータなど）に対応し、混在させてはならない。

**ルール 7.3**：結合テストはプロジェクトの公開 API を通じてテストしなければならない。結合テスト内で
`crate::` 内部モジュールを直接参照してはならない。`yaoxiang::` 公開パスを使用する。

```rust
// 🟢 良い例——公開 API を通す
use yaoxiang::run;

// 🔴 悪い例——公開 API 境界を迂回
use yaoxiang::middle::codegen::bytecode::BytecodeFile;
```

### テストデータ管理

**ルール 8.1**：結合テストはインラインソース文字列を優先する。ソースが 30 行を超える場合のみ外部 fixture ファイル（`tests/fixtures/`
に配置）を使用する。

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

**ルール 8.2**：fixture ファイルは `.yx` 拡張子で終わり、ファイル名はテスト意図を記述する。

### E2E カバレッジ原則

**ルール 9.1**：各言語機能の結合テストは 3 つのパスをカバーしなければならない：

| パス       | 説明                                                     |
| ---------- | -------------------------------------------------------- |
| Happy path | 合法的な入力が予期される出力を生成                       |
| Error path | 違法な入力が明確なエラーメッセージを生成（パニック以外） |
| Boundary   | 境界値（空入力、最大値、ネスト深度上限）                 |

**ルール 9.2**：結合テストはネットワーク、システム環境変数、外部サービスに依存してはならない。

---

## ベンチマークテスト規範

### Criterion.rs 使用規範

**ルール 10.1**：ベンチマークテストは `benches/` ディレクトリに統一して置き、エントリファイルは
`benches/lib.rs`。テストテーマごとにファイルを分割する。

```
benches/
├── lib.rs              # エントリ、criterion_group/criterion_main を定義
├── lang_compare/
│   └── fibonacci.rs    # 言語横断比較ベンチマーク
├── parser.rs           # パーサベンチマーク
└── codegen.rs          # コード生成ベンチマーク
```

**ルール 10.2**：各ベンチ関数にはモジュールドキュメントコメント `//!`
を含め、テスト目的と測定指標を説明しなければならない。

```rust
//! YaoXiang インタプリタ性能ベンチマークテスト
//!
//! 測定指標：単一反復時間（wall time）
//! 基準線：Rust ネイティブ実装
```

### コンパイラ最適化の防止

**ルール 11.1**：すべてのベンチマークテストの被テスト出力は `criterion::black_box`
を通過させ、コンパイラによる最適化除去を防がなければならない。

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

**ルール 11.2**：ベンチマークテストの入力データは `const` または `lazy_static`
でなければならず、`iter`
クロージャ内で動的に生成してはならない——さもないとデータ生成 + 被テスト論理の合計時間を測定することになる。

### ベンチグループと命名

**ルール 12.1**：ベンチマークテストの命名形式は
`<被テストモジュール>_<シナリオ>`、すべて小文字でアンダースコア区切り。ユニットテスト命名規則と一致する。

**ルール 12.2**：`criterion_group!`
を使用して関連ベンチを論理的にグループ化しなければならない。すべてのベンチを 1 つのグループに詰め込むことを禁止する。

```rust
criterion_group!(parser, bench_parse_expr, bench_parse_stmt);
criterion_group!(codegen, bench_codegen_module, bench_codegen_switch);
criterion_main!(parser, codegen);
```

---

## ドキュメントテスト規範

### 使用シーン

**ルール 13.1**：すべての `pub`
関数、型、メソッドはドキュメントコメントに少なくとも 1 つの実行可能なコード例を含まなければならない。この例は
`cargo test --doc` によって実行される。

````rust
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
````

**ルール 13.2**：ドキュメントテストのコード例はコンパイル成功かつアサーション成功でなければならない。`ignore`
マーク付きの例を含めてはならない。ただし、その例がコンパイル期エラーを示す場合は例外。

````rust
/// ```ignore
/// // コンパイル期エラーのデモ——ignore 可
/// let x: int = "string";
/// ```
````

### カバレッジ要件

**ルール 14.1**：ドキュメントテストは API の happy
path をカバーすればよい。境界ケースやエラーパスはユニットテストでカバーする。

**ルール 14.2**：ドキュメントテストのサンプルコードは簡潔でなければならない——10 行を超えない。もし例により多くのコンテキストが必要なら、API 設計に問題がある。

---

## プロパティテスト規範

### 使用シーン

**ルール 15.1**：以下のシーンではプロパティテスト（proptest または quickcheck）を使用しなければならず、手作業で複数の境界値ケースを書くのではない：

| シーン                             | 例                                     |
| ---------------------------------- | -------------------------------------- |
| パーサ round-trip                  | `parse(pretty_print(ast)) == ast`      |
| シリアライズ/デシリアライズ        | `deserialize(serialize(data)) == data` |
| 数学演算恒等式                     | `a + b == b + a`                       |
| コンパイラ最適化が意味論を変えない | `eval(code) == eval(optimize(code))`   |

**ルール 15.2**：プロパティテストは `proptest`
を主要なプロパティテストフレームワークとして使用する（既に `Cargo.toml` の `dev-dependencies`
で宣言済み）。

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

**ルール 16.1**：各プロパティテストには明確なプロパティ宣言が必要である——コメントに検証する不変量を記述する。

```rust
// プロパティ：任意の整数リテラルは tokenize → tokens_to_string 後に同じ値を生成する
proptest! {
    #[test]
    fn test_int_literal_roundtrip(n in any::<i64>()) {
        let source = n.to_string();
        let tokens = tokenize(&source).unwrap();
        // ...
    }
}
```

**ルール 16.2**：プロパティテストが失敗を発見した場合、`proptest`
のリグレッション機構を使用しなければならない——失敗した入力を `proptest-regressions/`
ディレクトリに追加し、通常のテストを手作業で書いて代替してはならない。

---

## カバレッジ要件

### 新規コードカバレッジ目標

**ルール 17.1**：新規コードのテストカバレッジ要件：

| コード種別                                           | 行カバレッジ | 分岐カバレッジ |
| ---------------------------------------------------- | ------------ | -------------- |
| コアコンパイラモジュール（frontend/middle/backends） | ≥ 85%        | ≥ 80%          |
| ツール/ヘルパーモジュール（util）                    | ≥ 75%        | ≥ 70%          |
| ランタイムモジュール（vm/runtime）                   | ≥ 80%        | ≥ 75%          |
| 標準ライブラリ（std）                                | ≥ 75%        | ≥ 70%          |
| エラー処理と診断                                     | ≥ 90%        | ≥ 85%          |

**ルール 17.2**：エラーハンドリングパス（すべての `Err`
分岐）は 100% カバーしなければならない。ユーザに見えるエラーメッセージはテストで検証済みでなければならない。

### PR レビューチェックリスト

**ルール 18.1**：PR 送信前、作者は以下の項目を自己点検しなければならない：

- [ ] `cargo test` すべて通過
- [ ] `cargo test --doc` すべて通過
- [ ] `cargo bench` で性能退化なし（ホットパス変更に関連する場合）
- [ ] 新規コードがカバレッジ目標に適合
- [ ] テスト命名が命名規範に適合
- [ ] 各テストファイルが対応する規範の章を宣言している（ルール 2.1）
- [ ] テスト期待値が規範定義から来ており、「現在のコードの出力」からではない
- [ ] `#[ignore]` マーキングされたテストがない（明確な issue 番号のコメントがある場合を除く）
- [ ] 不必要な `unwrap()` がない（`expect` またはカスタム panic メッセージを使用すべき）
- [ ] コミットメッセージが `:white_check_mark: test:` タイプを使用
- [ ] **「コードの挙動が規範と合わない」ことを理由にテスト期待値を修正していない——修正するのはコードであり、テストではない**
- [ ] **インラインテストがない**（`#[cfg(test)] mod tests { ... }` は
      `mod tests;` + 独立ファイルに変更、ルール 1.4 参照）

**ルール 18.2**：Reviewer は以下の問題を含む PR を拒否しなければならない：

- happy path テストのみで、エラーパスが欠如
- テスト内に `thread::sleep` または実行順序への依存がある
- コピペされたテストコードが 3 回を超えておりヘルパー関数が抽出されていない
- テスト名が命名規範に適合しない
- 永久に `#[ignore]` されたテストが存在する
- **テストがコードの誤った挙動に迎合している**（コードと規範が合わないときにコードではなくテストを修正している）
- **テストが対応する規範の章を宣言していない**（ルール 2.1 参照）
- **テスト期待値がコード出力から来ており、規範定義からではない**（逆算したテストはテストしていないのと同じ）
- **インラインテストが存在する**（`#[cfg(test)] mod tests { ... }`
  であり、`mod tests;` + 独立ファイルではない、ルール 1.4 参照）
- テストが「パニックしない」ことしか検証せず、具体的な挙動をアサートしない
- コードのバグを露出する失敗テストを削除した（コードを修正して緑になるのではなく）

---

## 付録

### A. テストコマンド早見表

```bash
# すべてのテストを実行
cargo test

# ユニットテストのみ実行
cargo test --lib

# 結合テストのみ実行
cargo test --test integration

# ドキュメントテストのみ実行
cargo test --doc

# 特定テストを実行（名前でフィルタ）
cargo test test_parse_expr

# ベンチマークテストを実行
cargo bench

# テスト出力を表示（デフォルトでは stdout を隠す）
cargo test -- --nocapture

# シングルスレッドで実行（並行問題調査）
cargo test -- --test-threads=1

# カバレッジレポートを生成（cargo-llvm-cov が必要）
cargo llvm-cov --html
```

### B. コミットメッセージテンプレート

テスト関連のコミットは以下のテンプレートに従わなければならない：

```
:white_check_mark: test(<scope>): <短い説明>

<オプション：カバーするシナリオリスト>
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

### C. 新規テストファイル一覧

新しいテストモジュールを作成するとき、以下のファイルが含まれていることを確認する：

```
# src/<module>/ ディレクトリに新規テストを追加
src/<module>/tests/
├── mod.rs          # モジュール宣言 + 共通ヘルパー関数
└── <subject>.rs    # テストファイル、被テストソースファイル名に対応

# tests/ ディレクトリに新規結合テストを追加
tests/
├── integration.rs   # 更新：#[path] 宣言を追加
└── integration/
    └── <topic>.rs   # 新規テストファイル
```

### D. 参考資料

- [YaoXiang 言語規範](../../design/language-spec.md) —— **テストの権威ある情報源**
- [承認済み RFC](../../design/rfc/accepted/) —— **設計決定の権威ある情報源**
- [Rust テストドキュメント](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Criterion.rs ユーザガイド](https://bheisler.github.io/criterion.rs/book/)
- [proptest ドキュメント](https://docs.rs/proptest/latest/proptest/)
- [プロジェクトコミット規範](./commit-convention.md)
- [プロジェクト貢献ガイド](./contributing.md)

---

> 💡
> **覚えておくこと**：テストはコードが「動く」ことを検証するのではない——コードが規範に合致していることを検証する。規範が変われば、テストは規範に従って変わる。コードが間違ったら、テストではなくコードを修正する。**コードは規範に仕え、テストは規範を守る。テストがコードに迎合した瞬間、すべての保護を失う。**

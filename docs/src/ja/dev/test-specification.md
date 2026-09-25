---
title: 'テスト作成規範'
description:
  YaoXiang
  プロジェクトのテスト作成ハード規範。ユニットテスト、結合テスト、ベンチマークテスト、ドキュメントテスト、プロパティテストの作成標準を定義する
---

# テスト作成規範

本文書は YaoXiang プロジェクトのテスト作成ハード規範を定義する。すべての貢献者は以下の規則を遵守しなければならず、違反者は Code
Review で修正を要求される。

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

| テスト種別         | 場所                  | フレームワーク             |
| ------------------ | --------------------- | -------------------------- |
| ユニットテスト     | `src/<module>/tests/` | `#[test]` + `#[cfg(test)]` |
| 結合テスト         | `tests/`              | `#[test]`                  |
| ベンチマークテスト | `benches/`            | Criterion.rs               |
| ドキュメントテスト | API ドキュメント注釈  | `cargo test --doc`         |
| プロパティテスト   | 任意のテスト場所      | proptest / quickcheck      |

### 核心原則

**原則 0：テストの権威の源は規範であり、コードではない。**
これは本文書で最も重要な原則である。テストが検証するのはコードが規範に適合しているかどうかであり、コードが「現在の実装で動く」かどうかではない。テストが規範と一致しないコードの挙動を発見した時、**テストを修正するのではなく、コードを修正する**。

規範ファイルの所在：

- `docs/src/design/language-spec.md` —— 言語コア規範
- `docs/src/design/rfc/accepted/` —— 受理済みの RFC 設計文書

各テストファイルの冒頭には対応する規範セクションを宣言しなければならない（規則 2.1 参照）。任意の開発者は規範文書とテストを照合して、実装の正当性を検証できるはずである。逆に——仕様に対応する記述のないコードは存在するべきではなく、ましてテストされるべきでもない。

```rust
// 🟢 良い——テストは規範を直接参照し、コードが規範に従っているかを検証する
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

// 🔴 ゴミ——テストは現在のコードの実装挙動に迎合しており、規範を検証していない
#[test]
fn test_literal_1() {
    // このコードが規範のどのセクションに対応するか不明
    // 仮に parse_literal が誤った値を返したとしても、このテストは「緑で通る」
    // 関数がパニックしないことしか検証していないため
    let result = parse_literal("42");
    assert!(result.is_ok());
}
```

**シナリオ**：テストを書いて、コードの挙動が規範と一致しないことに気づいた。あなたには二つの選択肢がある：

| 誤ったやり方                                     | 正しいやり方                               |
| ------------------------------------------------ | ------------------------------------------ |
| テストを変更して「通す」                         | コードを変更して、挙動を規範に合わせる     |
| テストに `#[ignore]` を付ける                    | 直ちにコード実装を修正する                 |
| テストに特別な条件分岐を追加してコードに迎合する | 分岐を削除し、テストに直接問題を露出させる |

記憶せよ：**赤信号 = コードが間違っている、テストが間違っているのではない。**（テスト自体にバグがある場合は別の話だが。）

**原則 1：テスト即ドキュメント。**
任意の開発者はテストを読むだけで被験コードの挙動を理解できなければならず、追加の注釈や外部ドキュメントを必要としてはならない。

```rust
// 🟢 良い——テスト名が何を測り、何を期待しているかを述べている
#[test]
fn test_tokenize_empty_input_returns_eof() {
    let tokens = tokenize("").unwrap();
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].kind, TokenKind::Eof));
}

// 🔴 ゴミ——何を測っているのか誰にもわからない
#[test]
fn test_tokenize_1() {
    let tokens = tokenize("").unwrap();
    assert!(tokens.len() > 0);
}
```

**原則 2：ランダムな失敗はゼロトレランス。**
テストはあらゆる環境で再現可能に実行されなければならない。乱数、システム時刻、スレッドスケジューリング順序に依存するテストは、シード固定を用いるかモックで代替しなければならない。

**原則 3：一つのテストは一つの事柄のみを測る。**
テスト名を「and」で繋いで複数の挙動を記述する必要があれば、複数のテストに分割する。

```rust
// 🟢 良い——各テストは一つのシナリオのみを検証する
#[test]
fn test_parse_int_positive() { /* ... */ }
#[test]
fn test_parse_int_zero() { /* ... */ }

// 🔴 ゴミ——一つのテストにあまりにも多くの無関係な内容が入っている
#[test]
fn test_parser() {
    // tokenize, parse, typecheck, codegen を全部測っている...
}
```

**原則 4：実装ではなく挙動をテストする。**
内部実装をリファクタリングしてもテストは失敗しないべきである。実装コードを一行変更して 10 個のテストが落ちたなら、テストの書き方が間違っている。

ただしここに重要な区別がある：**「挙動」の定義は現在のコードの現れからではなく、規範から来る。**
コードが挙動（すなわち規範に合致しない新しい挙動）を変更した場合、テストは必ず失敗しなければならない。これができないなら、あなたのテストは「コードに迎合するテスト」であり、バグの侵入を許してしまう。

```
規範（language-spec.md / RFC）  ──定義──►  期待挙動  ──駆動──►  テスト
                                           │
現在のコード  ──実装──►  実際の挙動  ──比較──►  テスト結果

実際の挙動 ≠ 期待挙動 の場合：
  テストは必ず失敗（赤信号）  ──►  コードを修正  ──►  テスト合格（緑信号）

実際の挙動 = 期待挙動（ただし実装は酷い）の場合：
  テスト合格  ──►  実装をリファクタ  ──►  テストは引き続き合格  ← これが原則 4 の真意
```

**原則 5：フォールバック/互換/特定パターン有効化のためのテストコードは書かない。**
テスト環境はあなたが完全に制御できる環境である。もし `#[cfg(not(ci))]`
を必要として特定のテストをスキップするならば、そのテスト設計には根本的な問題がある。

### 用語定義

| 用語               | 定義                                                                                      |
| ------------------ | ----------------------------------------------------------------------------------------- |
| ユニットテスト     | 単一の関数またはモジュールの挙動をテストし、外部システムに依存しない                      |
| 結合テスト         | 複数のモジュールの協調をテストし、公共 API またはコマンドラインエントリポイントを経由する |
| ベンチマークテスト | コード性能を計測し、性能回帰を検出する                                                    |
| ドキュメントテスト | ドキュメント注釈に埋め込まれた実行可能なコード例                                          |
| プロパティテスト   | ランダム入力に基づいて不変条件（property）を検証するテスト                                |

### コミット規範との関連

すべてのテスト関連のコミットは `:white_check_mark: test:`
タイプを使用しなければならず、[コミット規範](./commit-convention.md)を参照。

```
:white_check_mark: test(parser): Pratt パーサ中置式テストを追加
:white_check_mark: test(codegen): switch 文 IR 生成テストを補完
```

---

## yx コーパスとライブラリテスト階層

本規範は **Rust 側のテストコード** を制約する。YaoXiang 言語自身のテスト（`.yx`
コーパスとライブラリテスト）は被験対象によって二層に分けられ、体系設計と判定契約は RFC-036（§7 スイート収集 /
§8 負方向三層 / §9 テスト体系分层）に属し、コーパス作成細則は `tests/yaoxiang/TEST_STANDARDS.md`
に属する：

- **言語可用性コーパス**（`tests/yaoxiang/`）——被験対象は言語自身；std はアサーションの道具としてのみ使用。コーパス内では失敗の発生層によって三種類の判定に分ける：挙動テスト / コンパイル期拒否テスト / ランタイム期失敗テスト
- **ライブラリテスト**（ライブラリに随伴）——被験対象はライブラリの公共 API 契約；std の yx レベルテストは
  `src/std/tests/` に位置し、将来のユーザーパッケージのテストはパッケージ内の `[tool.test]`
  検出に従って随伴する

`.yx` テストのファイルヘッダ形式、ヘッダ指令（`// expect:` / `// skip:` / `// mode:`、RFC-036
§8.2）とアサーション規約は TEST_STANDARDS.md を基準とする；判定解析は二つのランナーが共有する
`src/util/test_markers.rs` で実装される（Rust 側、本規範の制約を受ける）。

---

## ユニットテスト規範

### ファイル構成

**規則 1.1**：ユニットテストの `tests/` ディレクトリは被験モジュールの `mod.rs` と **同レベル**
でなければならない。`tests/` は上向き集約せず、階層を跨いでまとめない。

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

重要な判断基準：**`tests/` をどのディレクトリに置くか、そのディレクトリの `mod.rs` は必ず
`#[cfg(test)] mod tests;` でそれを宣言しなければならない。**

**規則 1.1 補足：上向き集約禁止。** サブディレクトリモジュールのテストはそのサブディレクトリ自身の
`tests/` に置かなければならず、親レベル `tests/` に集約してはならない。

| モジュール種別                            | テスト場所                    | 例                                           |
| ----------------------------------------- | ----------------------------- | -------------------------------------------- |
| ディレクトリモジュール（`mod.rs` を持つ） | 当該ディレクトリ下の `tests/` | `emitter/tests/`、`codes/tests/`             |
| 単一ファイルモジュール（`.rs` のみ）      | 親レベルの `tests/`           | `session.rs` → `diagnostic/tests/session.rs` |

```text
# ✅ 正しい：各ディレクトリモジュールのテストはそれぞれ独立
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
    ├── mod.rs              # ❌ 無理に mod emitter; mod codes; を宣言することになる
    ├── emitter/            # ❌ emitter/tests/ に置くべき
    └── codes/              # ❌ codes/tests/ に置くべき
```

#### 単一ファイルモジュール vs ディレクトリモジュールのテスト配置規則

**核心的差異**：モジュールの組織形式がテストの配置場所を決定する。

| モジュール種別             | 判定根拠                                 | テスト場所                    | 例                                            |
| -------------------------- | ---------------------------------------- | ----------------------------- | --------------------------------------------- |
| **ディレクトリモジュール** | 独立したディレクトリと `mod.rs` を持つ   | 当該ディレクトリ下の `tests/` | `inference/tests/`                            |
| **単一ファイルモジュール** | `.rs` ファイルのみ、独立ディレクトリなし | 親モジュールの `tests/`       | `overload.rs` → `typecheck/tests/overload.rs` |

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
│   ├── overload.rs                 # overload.rs のテスト（単一ファイルモジュールのテストはここ）
│   ├── type_eval.rs                # type_eval.rs のテスト
│   ├── dead_code.rs                # dead_code.rs のテスト
│   ├── spawn_placement.rs          # spawn_placement.rs のテスト
│   ├── signature.rs                # signature.rs のテスト
│   └── types.rs                    # types.rs のテスト
│
├── inference/                      # ディレクトリモジュール（mod.rs を持つ）
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
└── traits/                         # 削除済み（ロジックは types/trait_data.rs にマージ）
```

**なぜ単一ファイルモジュールのテストを親レベル `tests/` に置くのか？**

単一ファイルモジュール（例：`overload.rs`）は自身の `mod.rs` を持たず、 `#[cfg(test)] mod tests;`
を宣言できない。Rust のモジュールシステムによれば、テストファイルはある `mod.rs`
によって宣言されなければコンパイルされない。したがって、単一ファイルモジュールのテストは親モジュールの
`mod.rs` によって宣言され、親レベルの `tests/` ディレクトリに置かれなければならない。

**判断フロー**：

```
あるモジュールに遭遇し、テストをどこに置くか判断する
│
├── 当該モジュールはディレクトリ（mod.rs を持つ）か？
│   └── はい → 当該ディレクトリ下に tests/ を作成し、当該ディレクトリの mod.rs が宣言する
│
├── 当該モジュールは単一ファイル（.rs のみ）か？
│   └── はい → テストを親レベルの tests/ ディレクトリに置き、親レベルの mod.rs が宣言する
│
└── 不確か？
    └── 独立したディレクトリと mod.rs を持つか確認する
```

**よくある誤り**：

```
# ❌ 誤り 1：単一ファイルモジュールのために独立した tests/ ディレクトリを作成する
src/frontend/core/typecheck/
├── overload.rs
└── overload/                       # ❌ 単一ファイルモジュールのためにディレクトリを作ってはならない
    └── tests/
        └── overload.rs

# ❌ 誤り 2：単一ファイルモジュール内で #[cfg(test)] mod tests; を宣言する
# overload.rs
#[cfg(test)]                        # ❌ 単一ファイルモジュールではこのように宣言できない
mod tests;                          # overload/tests/ ディレクトリが存在しないため

# ✅ 正しいやり方：テストを親レベル tests/ に置く
src/frontend/core/typecheck/
├── overload.rs                     # ソースファイル
└── tests/
    └── overload.rs                 # テストファイル、typecheck/mod.rs が宣言する
```

⚠️ **アンチパターン——こう書いてはいけない：**

```
# ❌ 誤り：サブモジュールのテストを親レベルに集中させる
src/frontend/core/types/
├── mod.rs              # base と computation のみを宣言すべき
├── base/
│   ├── mod.rs
│   └── var.rs
└── tests/              # ❌ 親レベル tests/ がサブモジュールのテストを含む
    ├── mod.rs          # ❌ 無理に mod base; mod computation; を宣言することになる
    ├── base/           # ❌ この部分は base/tests/ に置くべき
    │   └── var.rs
    └── computation/    # ❌ この部分は computation/tests/ に置くべき
        └── ...
```

```
# ✅ 正しいやり方：各モジュールのテストはそれぞれ独立
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

**なぜ上向き集約できないのか？** Rust のモジュールシステムは `#[cfg(test)] mod tests;`
が宣言箇所でテストファイルのコンパイルを決定することを要求する。`types/mod.rs` が `mod tests;`
を宣言したなら、`types/tests/` の内容は `types` モジュールのプライベートな内容であり、`base` や
`computation`
の領域に踏み込むべきではない。各モジュールのテストはそのモジュールの内部実装詳細であり、親モジュールのものではない。この規則はモジュールのリファクタリングにも同様に適用される：`types`
を `base` と `computation`
に分割した時、テストも分割後のモジュールに従って移動すべきであり、元の場所に残すべきではない。**テストディレクトリはソース構造を鏡写しにするのではなく、モジュール境界に追随する。**

**規則 1.2**：`tests/mod.rs` はモジュール宣言と re-export のみを担当し、テスト関数を置かない。

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

**規則 1.3**：各テストファイルは一つのソースファイルのみに対応する。複数のソースモジュールのテストを一つのファイルに混在させてはならない。

**規則 1.4**：テスト宣言はファイル形式 `mod tests;`（セミコロン付き）を使用し、同レベルの `tests/`
ディレクトリを指すものとする。**インライン形式 `mod tests { ... }`
を使ってテストコードをソースファイル内に直接書くことを禁止する。**

```rust
// ✅ 正しい——ファイル形式宣言、テストコードは独立したファイルに
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

1. ソースファイルの責務単一：ソースファイルには実装のみ、テストファイルにはテストのみ。混在すると、テスト変更にはファイル末尾までスクロールし、実装変更にはテストをスキップする必要がある。
2. モジュール境界の明確さ：`tests/`
   ディレクトリは物理的境界であり、どのモジュールにテストがあり、ないか一目瞭然。
3. リファクタリングの安全性：モジュール分割時、`tests/`
   ディレクトリも追随する；インラインテストはソースファイルから手動で剥離する必要がある。
4. コードレビュー：PR diff でソースコード変更とテスト変更は別々のファイルとなり、混在しない。

### モジュール宣言規範

**規則 2.1**：すべてのテストファイルの冒頭にはモジュールレベルドキュメントコメント `//!`
を置き、テストがカバーする規範の出所（言語規範セクション番号 +
RFC 番号）を記述しなければならない。あるテストがどの規範セクションも参照していないなら、そのコードには規範的根拠がない——それは存在すべきではない。

```rust
//! literal テスト — 言語規範 §2.6 に基づく
//!
//! §2.6.1: 整数 Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: 浮動小数点数（小数点と指数を含む）
//! §2.6.3: 文字列（エスケープシーケンス \\nrt'"\\, \\x, \\u{}）
//! RFC-012: F-String 補間
```

**なぜ規範を参照しなければならないのか？**
テストの期待値は規範から来るべきであり、「現在のコードの出力」から来るべきではない。もしある日コードの出力が変更され、テストがそれに合わせて更新されたなら、そのテストは何も保護していない。規範に固定されたテストのみが「意図的な breaking
change」と「意図しない回帰」を区別できる。

**規則 2.2**：テストモジュールの `use` インポートは具体的な型/関数に正確に絞り、glob インポート
`use super::*` を禁止する。

```rust
// 🟢 良い——精確なインポート
use crate::frontend::core::lexer::{tokenize, TokenKind};
use crate::frontend::core::parser::{ParserState, ParseError};

// 🔴 ゴミ——何をテストしているか他人にわからない
use super::*;
```

### 命名規範

**規則 3.1**：テスト関数命名形式は `test_<what>_<scenario>`、すべて小文字アンダースコア区切り。

```rust
#[test]
fn test_tokenize_empty_string() { /* ... */ }
#[test]
fn test_parse_int_overflow() { /* ... */ }
#[test]
fn test_typecheck_fn_return_mismatch() { /* ... */ }
```

**規則 3.2**：テスト関数名は自己説明的でなければならない。関数名を読んだだけで何を測り、何を期待しているかわかること。数字による連番命名を禁止する。

```rust
// 🟢 良い
fn test_skip_semicolon_success() { /* ... */ }
fn test_skip_semicolon_failure_when_identifier() { /* ... */ }

// 🔴 ゴミ——何を測っているか全くわからない
fn test_skip_1() { /* ... */ }
fn test_skip_2() { /* ... */ }
```

**規則 3.3**：補助関数は `test_` 接頭辞を必要とせず、動詞または名詞でその用途を記述する。

```rust
fn parse_expr(source: &str) -> Expr { /* ... */ }
fn tokenize_single(source: &str) -> Token { /* ... */ }
fn setup_parser_with_tokens(tokens: &[Token]) -> ParserState { /* ... */ }
```

### テスト構造規範 (Arrange-Act-Assert)

**規則 4.1**：各テスト関数は三段構造（準備 Arrange → 実行 Act
→ 検証 Assert）に従わなければならず、三段の間は空行で区切る。

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

**規則 4.2**：簡単なテスト（単一呼び出し + 単一アサーション）はセグメントコメントを書かなくてよいが、5 行の論理コードを超えない。5 行を超えるテストは三段を明示しなければならない。

### 補助関数規範

**規則 5.1**：3 回以上繰り返される setup ロジックは補助関数として抽出しなければならない。

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

**規則 5.2**：補助関数内の `unwrap()` / `expect()`
はパニック時に十分なコンテキストを出力しなければならない。テスト関数本体 (`#[test] fn ...`) では直接
`unwrap()`
してよい——失敗時に Rust が自動的に行番号を出力するため；但し補助関数内で失敗した時、行番号は補助関数の定義位置を指し、呼び出し時のコンテキストが見えない。

```rust
// 🟢 良い——補助関数失敗時にソース内容を表示
fn run_ok(source: &str) {
    run(source).unwrap_or_else(|e| panic!("Execution failed:\nSource:\n{}\nError:\n{:?}", source, e));
}

// 🔴 ゴミ——失敗時、どのソースファイルが原因か見えない
fn run_ok(source: &str) {
    run(source).unwrap();
}
```

**規則 5.3**：補助関数はテストファイルの冒頭に置き、`use`
インポートの直後に置く。複数のテストモジュールで共有する場合は `tests/mod.rs` に置き、`pub(crate)`
でエクスポートする。

### アサーションスタイル

**規則 6.1**：enum 値変体マッチは `assert!(matches!(...))` を優先的に使用し、`if let` + `panic!`
を使用してはならない。

```rust
// 🟢 良い
assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(42)));

// 🔴 ゴミ
if let TokenKind::IntLiteral(v) = tokens[0].kind {
    assert_eq!(v, 42);
} else {
    panic!("Expected IntLiteral");
}
```

**規則 6.2**：精確な値比較は `assert_eq!` を使用し、布尔アサーションは `assert!`
を使用する。`assert!(a == b)` を `assert_eq!(a, b)` の代わりに使用することを禁止する。

**規則 6.3**：すべてのアサーションはカスタムエラーメッセージを伴わなければならない。ただしアサーション自体が失敗原因を完全に記述している場合は除く。

```rust
// 🟢 良い——アサーション失敗時に迅速に特定できる
assert!(
    state.infix_info().is_some(),
    "infix_info should handle '{op}'"
);

// 🟢 良い——assert_eq! 失敗時に値の差異を自動表示、追加入力不要
assert_eq!(error_count, 0);

// 🔴 ゴミ——失敗時に「assertion failed」としかわからない
assert!(state.infix_info().is_some());
```

**規則 6.4**：アサーションの順序は `assert_eq!(actual, expected)`、実際の値が先、期待値が後。

### アンチパターン一覧

以下は禁止されている書き方と代替案：

| アンチパターン                                     | 問題                                                         | 代替案                                                                                   |
| -------------------------------------------------- | ------------------------------------------------------------ | ---------------------------------------------------------------------------------------- |
| `#[cfg(test)] mod tests { ... }` インラインテスト  | ソースファイル肥大、モジュール境界曖昧、リファクタリング困難 | テストコードを独立した `tests/` ディレクトリに置き、`mod tests;` で宣言（規則 1.4 参照） |
| コードの誤った挙動にテストが迎合する               | 規範偏差を覆い隠し、バグを合法化する                         | 規範照らし合わせでコードを修正、テストは不変に保つ                                       |
| コード出力から逆算してテスト期待値を決める         | テストが「現在実装の録音機」になる                           | 規範から期待値を導出する                                                                 |
| `#[ignore]` 永久マーク                             | 腐ったテストを隠蔽                                           | 修正または削除                                                                           |
| `println!` デバッグ出力                            | テスト出力を汚染                                             | `assert!` で明確にアサート                                                               |
| `thread::sleep`                                    | ランダム失敗 + 遅い                                          | 同期機構またはモックを使用                                                               |
| テストで実ファイルシステムを操作                   | 遅く再現不可                                                 | `tempfile` を使用                                                                        |
| テスト実行順序に依存                               | ランダム失敗                                                 | 各テスト独立 setup                                                                       |
| 一つのテスト関数が 30 行の論理を超える             | 誰も読めない                                                 | テスト分割または補助関数使用                                                             |
| 補助関数内の `unwrap()` がコンテキストを出力しない | 位置特定困難                                                 | `expect("why")` またはカスタム panic を使用（規則 5.2 参照）                             |
| 3 回以上同じ setup を copy-paste                   | 変更コスト高い                                               | 補助関数を抽出                                                                           |

---

## 結合テスト規範

### テスト構成

**規則 7.1**：結合テストはプロジェクトルートディレクトリの `tests/`
ディレクトリに置く。エントリファイル `tests/integration.rs` は `#[path]`
属性を使用して子モジュールをインポートする。

```rust
// tests/integration.rs
#[path = "integration/backends.rs"]
mod backends;
#[path = "integration/codegen.rs"]
mod codegen;
#[path = "integration/execution.rs"]
mod execution;
```

**規則 7.2**：各 `tests/integration/*.rs`
ファイルは一つのテストテーマ（コンパイラバックエンド、コード生成、エグゼキュータ等）に対応し、混在させてはならない。

**規則 7.3**：結合テストはプロジェクトの公共 API 経由でテストしなければならない。結合テスト内で
`crate::` 内部モジュールを直接参照することを禁止。`yaoxiang::` 公共パスを使用する。

```rust
// 🟢 良い——公共 API 経由
use yaoxiang::run;

// 🔴 ゴミ——公共 API 境界を迂回
use yaoxiang::middle::codegen::bytecode::BytecodeFile;
```

### テストデータ管理

**規則 8.1**：結合テストはインラインソース文字列を優先する。ソースが 30 行を超える場合に限り、外部 fixture ファイル（`tests/fixtures/`
に配置）を使用する。

```rust
#[test]
fn test_fibonacci() {
    run_ok(
        r#"
        main: () -> Void = {
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

**規則 8.2**：fixture ファイルは `.yx` 拡張子で終わり、ファイル名はテスト意図を記述する。

### E2E カバレッジ原則

**規則 9.1**：各言語機能の結合テストは三つのパスをカバーしなければならない：

| パス       | 説明                                                           |
| ---------- | -------------------------------------------------------------- |
| Happy path | 合法入力が予期された出力を生成する                             |
| Error path | 非法入力が明確なエラーメッセージを生成する（パニックではなく） |
| Boundary   | 境界値（空入力、最大値、ネスト深度上限）                       |

**規則 9.2**：結合テストはネットワーク、システム環境変数または外部サービスに依存してはならない。

---

## ベンチマークテスト規範

### Criterion.rs 使用規範

**規則 10.1**：ベンチマークテストは `benches/` ディレクトリに統一配置し、エントリファイルは
`benches/lib.rs`。テストテーマごとにファイル分割する。

```
benches/
├── lib.rs              # エントリ、criterion_group/criterion_main を定義
├── lang_compare/
│   └── fibonacci.rs    # 言語間比較ベンチマーク
├── parser.rs           # パーサベンチマーク
└── codegen.rs          # コード生成ベンチマーク
```

**規則 10.2**：各ベンチマーク関数はモジュールドキュメントコメント `//!`
を含め、テスト目的と測定指標を記述する。

```rust
//! YaoXiang インタプリタ性能ベンチマークテスト
//!
//! 測定指標：一回反復の所要時間（wall time）
//! ベースライン：Rust ネイティブ実装
```

### コンパイラ最適化の防止

**規則 11.1**：すべてのベンチマークテストの被験出力は `criterion::black_box`
を通じてコンパイラ最適化による除去を防止しなければならない。

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

**規則 11.2**：ベンチマークテストの入力データは `const` または `lazy_static`
でなければならず、`iter`
クロージャ内で動的に生成してはならない——さもないとデータ生成 + 被験ロジックの合計時間を測定することになる。

### ベンチマークグループ化と命名

**規則 12.1**：ベンチマークテストの命名形式は
`<被験モジュール>_<シナリオ>`、すべて小文字アンダースコア区切り。ユニットテスト命名規則と一致する。

**規則 12.2**：`criterion_group!`
を使用して関連ベンチマークを論理的にグループ化しなければならない。すべてのベンチマークを一つのグループに押し込むことを禁止する。

```rust
criterion_group!(parser, bench_parse_expr, bench_parse_stmt);
criterion_group!(codegen, bench_codegen_module, bench_codegen_switch);
criterion_main!(parser, codegen);
```

---

## ドキュメントテスト規範

### 使用シナリオ

**規則 13.1**：すべての `pub`
関数、型、メソッドはドキュメント注釈に少なくとも一つの実行可能なコード例を含まなければならない。当該例は
`cargo test --doc` により実行される。

````rust
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
````

**規則 13.2**：ドキュメントテストのコード例はコンパイル成功かつアサーション成功でなければならない。当該例がコンパイルエラーを示すのでない限り、`ignore`
マーク付きの例を含んではならない。

````rust
/// ```ignore
/// // コンパイルエラーを示す——ignore 可
/// let x: int = "string";
/// ```
````

### カバレッジ要件

**規則 14.1**：ドキュメントテストは API の happy
path をカバーすればよい。境界状況とエラーパスはユニットテストがカバーする。

**規則 14.2**：ドキュメントテストのサンプルコードは簡潔でなければならない——10 行を超えない。もしサンプルがより長いコンテキストを要するならば、API 設計に問題がある。

---

## プロパティテスト規範

### 使用シナリオ

**規則 15.1**：以下のシナリオでは、手書きで複数の境界値用例を書くのではなく、プロパティテスト（proptest または quickcheck）を使用しなければならない：

| シナリオ                         | 例                                     |
| -------------------------------- | -------------------------------------- |
| パーサ round-trip                | `parse(pretty_print(ast)) == ast`      |
| シリアライズ/デシリアライズ      | `deserialize(serialize(data)) == data` |
| 数学演算恒等式                   | `a + b == b + a`                       |
| コンパイラ最適化が意味を変えない | `eval(code) == eval(optimize(code))`   |

**規則 15.2**：プロパティテストは `proptest`
を主要なプロパティテストフレームワークとして使用する（既に `Cargo.toml` の `dev-dependencies`
に宣言済み）。

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

**規則 16.1**：各プロパティテストは明確なプロパティ宣言を持たなければならない——コメントに検証する不変条件を明記する。

```rust
// プロパティ：任意の整数の literal は tokenize → tokens_to_string 後に同じ値を生成する
proptest! {
    #[test]
    fn test_int_literal_roundtrip(n in any::<i64>()) {
        let source = n.to_string();
        let tokens = tokenize(&source).unwrap();
        // ...
    }
}
```

**規則 16.2**：プロパティテストが失敗を発見した場合、`proptest`
の回帰機構を使用しなければならない——失敗した入力を `proptest-regressions/`
ディレクトリに追加し、代わりに普通のテストを手書きしてはならない。

---

## カバレッジ要件

### 新規コードカバレッジ目標

**規則 17.1**：新規コードのテストカバレッジ要件：

| コード種別                                           | 行カバレッジ | 分岐カバレッジ |
| ---------------------------------------------------- | ------------ | -------------- |
| コアコンパイラモジュール（frontend/middle/backends） | ≥ 85%        | ≥ 80%          |
| ツール/補助モジュール（util）                        | ≥ 75%        | ≥ 70%          |
| ランタイムモジュール（vm/runtime）                   | ≥ 80%        | ≥ 75%          |
| 標準ライブラリ（std）                                | ≥ 75%        | ≥ 70%          |
| エラー処理と診断                                     | ≥ 90%        | ≥ 85%          |

**規則 17.2**：エラー処理パス（すべての `Err`
分岐）は 100% カバレッジでなければならない。ユーザに見えるエラーメッセージはテストで検証済みでなければならない。

### PR レビュー検査チェックリスト

**規則 18.1**：PR 提出前に、作者は以下の項目を自己点検しなければならない：

- [ ] `cargo test` すべて合格
- [ ] `cargo test --doc` すべて合格
- [ ] `cargo bench` 性能回帰なし（ホットパス変更が関わる場合）
- [ ] 新規コードがカバレッジ目標に適合
- [ ] テスト命名が命名規範に適合
- [ ] 各テストファイルが対応する規範セクションを宣言している（規則 2.1）
- [ ] テスト期待値が「現在のコードの出力」ではなく規範定義から来ている
- [ ] `#[ignore]` マーク付きテストがない（明確な issue 番号注釈がある場合を除く）
- [ ] 不要な `unwrap()` がない（`expect` またはカスタム panic メッセージを使用すべき）
- [ ] コミットメッセージが `:white_check_mark: test:` タイプを使用
- [ ] **「コード挙動と規範の不一致」によりテスト期待値を変更していない——変更するのはコードであり、テストではない**
- [ ] **インラインテストがない**（`#[cfg(test)] mod tests { ... }` は
      `mod tests;` + 独立ファイルに変更しなければならない、規則 1.4 参照）

**規則 18.2**：Reviewer は以下の問題を含む PR を必ず拒否しなければならない：

- happy path テストのみで、エラーパスが欠如
- テスト内に `thread::sleep` または実行順序への依存がある
- 3 回を超える copy-paste テストコードが補助関数として抽出されていない
- テスト名が命名規範に適合しない
- 永久 `#[ignore]` のテストが存在する
- **テストがコードの誤った挙動に迎合している**（規範とコードが不一致の際、コードではなくテストを変更）
- **テストが対応する規範セクションを宣言していない**（規則 2.1 参照）
- **テスト期待値が規範定義ではなくコード出力から来ている**（逆算したテストは測っていないのと同じ）
- **インラインテストが存在する**（`#[cfg(test)] mod tests { ... }` ではなく
  `mod tests;` + 独立ファイル、規則 1.4 参照）
- テストが「不パニック」のみを検証し、具体的な挙動をアサートしない
- コードのバグを露出する失敗テストを削除した（コード修正後に緑に変わるのではなく）

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

# シングルスレッド実行（並行問題の切り分け）
cargo test -- --test-threads=1

# カバレッジレポートを生成（cargo-llvm-cov が必要）
cargo llvm-cov --html
```

### B. コミットメッセージテンプレート

テスト関連のコミットは以下のテンプレートに従わなければならない：

```
:white_check_mark: test(<scope>): <簡潔な説明>

<任意：カバーするシナリオ一覧>
```

例：

```
:white_check_mark: test(parser): Pratt パーサ中置演算子テストを追加

カバーするシナリオ：
- 算術演算子の優先順位（+, -, *, /, %）
- 比較演算子の連鎖（1 < x < 10）
- 論理演算子の短絡
- 代入演算子の右結合
```

### C. 新規テストファイル一覧

新しいテストモジュールを作成する際、以下のファイルを含むことを確認する：

```
# src/<module>/ ディレクトリ下にテストを新規追加
src/<module>/tests/
├── mod.rs          # モジュール宣言 + 公共補助関数
└── <subject>.rs    # テストファイル、被験ソースファイルの命名に対応する

# tests/ ディレクトリ下に結合テストを新規追加
tests/
├── integration.rs   # 更新：#[path] 宣言を追加
└── integration/
    └── <topic>.rs   # 新規テストファイル
```

### D. 参考資料

- [YaoXiang 言語規範](../reference/language-spec/index.md) —— **テストの権威の源**
- [受理済み RFC](../design/rfc/index.md) —— **設計判断の権威の源**
- [Rust テストドキュメント](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Criterion.rs ユーザーガイド](https://bheisler.github.io/criterion.rs/book/)
- [proptest ドキュメント](https://docs.rs/proptest/latest/proptest/)
- [プロジェクトコミット規範](./commit-convention.md)
- [プロジェクト貢献ガイド](./contributing.md)

---

> 💡
> **記憶せよ**：テストはあなたのコードが「動く」ことを検証するのではない——あなたのコードが規範に適合しているかを検証する。規範が変わり、テストは規範に従って変わる。コードを書き間違えたら、テストではなくコードを修正せよ。**コードは規範に仕え、テストは規範を守る。テストがコードに迎合した瞬間、あなたはすべての保護を失う。**

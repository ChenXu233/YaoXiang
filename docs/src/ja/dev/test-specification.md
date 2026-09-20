---
title: 'テスト作成規範'
description:
  YaoXiang
  プロジェクトのテスト作成に関する厳格な規範。ユニットテスト、統合テスト、ベンチマークテスト、ドキュメントテスト、プロパティテストの作成基準を定義する
---

# テスト作成規範

本文書は YaoXiang プロジェクトのテスト作成に関する厳格な規範を定義する。すべての貢献者は以下の規則を遵守しなければならず、違反者は Code
Review で修正を求められる。

---

## 目次

- [総則](#総則)
- [yx コーパスとライブラリテスト階層](#yx-コーパスとライブラリテスト階層)
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

本規範は YaoXiang プロジェクトにおけるすべての Rust テストコードに適用される：

| テスト種別         | 位置                     | フレームワーク             |
| ------------------ | ------------------------ | -------------------------- |
| ユニットテスト     | `src/<module>/tests/`    | `#[test]` + `#[cfg(test)]` |
| 統合テスト         | `tests/`                 | `#[test]`                  |
| ベンチマークテスト | `benches/`               | Criterion.rs               |
| ドキュメントテスト | API ドキュメントコメント | `cargo test --doc`         |
| プロパティテスト   | 任意のテスト位置         | proptest / quickcheck      |

### 核心原則

**原則 0：テストの権威ある出典は規範であり、コードではない。**
これは本文書で最も重要な原則である。テストが検証するのは「コードが規範に合致しているか」であり、「コードが現在の実装で動作するか」ではない。テストがコードの挙動と規範の不一致を発見した時、**テストを修正するのではなく、コードを修正する**。

規範ファイルの位置：

- `docs/src/design/language-spec.md` —— 言語コア規範
- `docs/src/design/rfc/accepted/` —— 承認済み RFC 設計文書

各テストファイルの先頭には、対応する規範セクションを必ず宣言しなければならない（規則 2.1 参照）。すべての開発者は規範文書とテストを照合し、実装の正当性を検証できるべきである。逆に——コードに対応する規範記述がなければ、そのコードは存在すべきではなく、ましてテストされるべきではない。

```rust
// 🟢 良い——テストが規範を直接参照し、コードが規範に従っているかを検証する
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

// 🔴 ゴミ——テストが現在のコードの実装挙動に迎合しており、規範を検証していない
#[test]
fn test_literal_1() {
    // このコードが規範のどのセクションに対応するか不明
    // parse_literal が誤った値を返しても、このテストは「緑」で通過する
    // 関数が panic しないことのみ検証しているため
    let result = parse_literal("42");
    assert!(result.is_ok());
}
```

**シナリオ**：テストを書いて、コードの挙動が規範と一致しないことに気づいた。2 つの選択肢がある：

| 誤った方法                                     | 正しい方法                                 |
| ---------------------------------------------- | ------------------------------------------ |
| テストを変更して「通過」させる                 | コードを修正し、挙動を規範に合わせる       |
| テストに `#[ignore]` を追加する                | 直ちにコード実装を修正する                 |
| テストに特殊条件分岐を追加し、コードに迎合する | 分岐を削除し、テストに直接問題を露出させる |

覚えておくこと：**赤信号 = コードが間違っている、テストが間違っているのではない。**（テスト自体にバグがある場合は別問題である。）

**原則 1：テスト即是ドキュメント。**
すべての開発者はテストを読むことで、テスト対象コードの挙動を理解でき、追加のコメントや外部ドキュメントを必要としない。

```rust
// 🟢 良い——テスト名に何をテストし、何を期待するかが明記されている
#[test]
fn test_tokenize_empty_input_returns_eof() {
    let tokens = tokenize("").unwrap();
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].kind, TokenKind::Eof));
}

// 🔴 ゴミ——何をテストしているのか誰もわからない
#[test]
fn test_tokenize_1() {
    let tokens = tokenize("").unwrap();
    assert!(tokens.len() > 0);
}
```

**原則 2：ランダムな失敗のゼロトレランス。**
テストは任意の環境で繰り返し実行可能でなければならない。乱数、システム時刻、スレッドスケジューリング順序に依存するテストは、シード固定またはモックによる置換を使用しなければならない。

**原則 3：1 つのテストは 1 つの事柄のみをテストする。**
テスト名に「と」で複数の挙動を接続する必要があるなら、複数のテストに分割する。

```rust
// 🟢 良い——各テストが 1 つのシナリオのみを検証
#[test]
fn test_parse_int_positive() { /* ... */ }
#[test]
fn test_parse_int_zero() { /* ... */ }

// 🔴 ゴミ——1 つのテストに多くの無関係な内容が詰め込まれている
#[test]
fn test_parser() {
    // tokenize、parse、typecheck、codegen をすべてテスト...
}
```

**原則 4：挙動をテストし、実装をテストしない。**
内部実装のリファクタリングがテスト失敗を引き起こしてはならない。実装コードを 1 行変更して 10 個のテストが失敗するなら、テストの書き方が間違っている。

ただし、ここに重要な区別がある：**「挙動」の定義は規範から来るものであり、現在のコードの表れから来るものではない。**
コードが挙動を変更した場合（すなわち、規範に合致しない新しい挙動）、テストは失敗しなければならない。これが達成できないなら、テストは「コードに迎合するテスト」である——バグの侵入を許してしまう。

```
規範（language-spec.md / RFC）  ──定義──►  期待挙動  ──駆動──►  テスト
                                            │
現在のコード  ──実装──►  実際の挙動  ──対比──►  テスト結果

実際の挙動 ≠ 期待挙動の場合：
  テストは必ず失敗（赤）  ──►  コードを修正  ──►  テスト通過（緑）

実際の挙動 = 期待挙動（ただし実装が酷い）：
  テスト通過  ──►  実装をリファクタ  ──►  テストは依然として通過  ← これが原則 4 の真の意味
```

**原則 5：後退/互換/特定パターン有効化のテストコードを書かない。**
テスト環境は完全に制御可能な環境である。テストをスキップするために `#[cfg(not(ci))]`
が必要なら、そのテスト設計には根本的な問題がある。

### 用語定義

| 用語               | 定義                                                                                |
| ------------------ | ----------------------------------------------------------------------------------- |
| ユニットテスト     | 単一の関数またはモジュールの挙動をテストし、外部システムに依存しない                |
| 統合テスト         | 複数のモジュールの連携をテストし、公開 API またはコマンドラインエントリポイント経由 |
| ベンチマークテスト | コード性能を測定し、性能回帰を検出                                                  |
| ドキュメントテスト | ドキュメントコメントに埋め込まれた実行可能なコードサンプル                          |
| プロパティテスト   | ランダム入力に基づいて不変量（プロパティ）を検証するテスト                          |

### コミット規範との関連

すべてのテスト関連のコミットは `:white_check_mark: test:`
タイプを使用しなければならず、[コミット規範](./commit-convention.md)を参照。

```
:white_check_mark: test(parser): Pratt パーサの中置式テストを追加
:white_check_mark: test(codegen): switch 文の IR 生成テストを補完
```

---

## yx コーパスとライブラリテスト階層

本規範は **Rust 側のテストコード** を制約する。YaoXiang 言語自体のテスト（`.yx`
コーパスとライブラリテスト）はテスト対象によって 2 つの階層に分けられ、体系設計と判定契約は RFC-036（§7 スイート収集 /
§8 負の 3 層 / §9 テスト体系階層）に従い、コーパス作成細則は `tests/yaoxiang/TEST_STANDARDS.md`
に従う：

- **言語可用性コーパス**（`tests/yaoxiang/`）——テスト対象は言語自体；std はアサーションのツールとしてのみ使用。コーパス内では失敗発生層によって 3 種類の判定に分類される：挙動テスト / コンパイル時拒否テスト / 実行時失敗テスト
- **ライブラリテスト**（ライブラリに付随）——テスト対象はライブラリの公開 API 契約；std の yx レベルテストは
  `src/std/tests/` に配置され、将来のユーザーパッケージのテストはパッケージ内の `[tool.test]`
  検出による

`.yx` テストのファイルヘッダ形式、ヘッダ指令（`// expect:` / `// skip:` / `// mode:`、RFC-036
§8.2）とアサーション規約は TEST_STANDARDS.md に従う；判定解析は両ランナー共通の
`src/util/test_markers.rs`（Rust 側、本規範の制約を受ける）で実装される。

---

## ユニットテスト規範

### ファイル構成

**規則 1.1**：ユニットテストの `tests/` ディレクトリは、テスト対象モジュールの `mod.rs`
と同階層でなければならない。`tests/` は上位に集約せず、階層をまたいでまとめない。

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

重要な判断基準：**`tests/` を配置したディレクトリの `mod.rs` は、必ず `#[cfg(test)] mod tests;`
でそれを宣言しなければならない。**

**規則 1.1 補足：上位への集約を禁止する。**
サブディレクトリモジュールのテストは、そのサブディレクトリ自身の `tests/`
に配置しなければならず、親階層の `tests/` に集約してはならない。

| モジュール種別                          | テスト位置                    | 例                                           |
| --------------------------------------- | ----------------------------- | -------------------------------------------- |
| ディレクトリモジュール（`mod.rs` あり） | 当該ディレクトリ下の `tests/` | `emitter/tests/`、`codes/tests/`             |
| 単一ファイルモジュール（`.rs` のみ）    | 親階層の `tests/`             | `session.rs` → `diagnostic/tests/session.rs` |

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

**核心的な違い**：モジュールの組織形式がテストの配置位置を決定する。

| モジュール種別             | 判断基準                                 | テスト位置                    | 例                                            |
| -------------------------- | ---------------------------------------- | ----------------------------- | --------------------------------------------- |
| **ディレクトリモジュール** | 独立したディレクトリと `mod.rs` あり     | 当該ディレクトリ下の `tests/` | `inference/tests/`                            |
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
└── traits/                         # 削除済み（ロジックは types/trait_data.rs に統合）
```

**なぜ単一ファイルモジュールのテストを親階層の `tests/` に配置するのか？**

単一ファイルモジュール（例：`overload.rs`）は自身の `mod.rs`
を持たないため、`#[cfg(test)] mod tests;`
を宣言できない。Rust のモジュールシステムによると、テストファイルは何らかの `mod.rs`
によって宣言されなければコンパイルできない。したがって、単一ファイルモジュールのテストは親階層モジュールの
`mod.rs` で宣言され、親階層の `tests/` ディレクトリに配置されるしかない。

**判断フロー**：

```
モジュールに遭遇し、テストの配置先を判断する
│
├── 当該モジュールはディレクトリ（mod.rs あり）？
│   └── はい → 当該ディレクトリ下に tests/ を作成し、当該ディレクトリの mod.rs で宣言
│
├── 当該モジュールは単一ファイル（.rs のみ）？
│   └── はい → 親階層の tests/ ディレクトリにテストを配置し、親階層の mod.rs で宣言
│
└── 不確定？
    └── 独立したディレクトリと mod.rs があるかを確認
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
#[cfg(test)]                        # ❌ 単一ファイルモジュールはこのように宣言できない
mod tests;                          # overload/tests/ ディレクトリがないため

# ✅ 正しい方法：テストを親階層の tests/ に配置
src/frontend/core/typecheck/
├── overload.rs                     # ソースファイル
└── tests/
    └── overload.rs                 # テストファイル、typecheck/mod.rs で宣言される
```

⚠️ **アンチパターン——以下のように書かないこと：**

```
# ❌ 誤り：サブモジュールのテストを親階層に集中
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
宣言箇所でテストファイルのコンパイルを決定するため。`types/mod.rs` が `mod tests;`
を宣言する場合、`types/tests/` の内容は `types` モジュールのプライベート内容となる——`base` や
`computation`
の領域に踏み込むべきではない。各モジュールのテストはそのモジュールの内部実装の詳細であるべきであり、親モジュールのものではない。この規則はモジュールのリファクタリングにも同様に適用される：`types`
を `base` と `computation`
に分割する際、テストも分割後のモジュールに従うべきであり、元の場所に残るべきではない。**テストディレクトリはソース構造を鏡写しにするのではなく、モジュール境界に従う。**

**規則 1.2**：`tests/mod.rs` はモジュールの宣言と re-export のみを担当し、テスト関数を配置しない。

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

**規則 1.3**：各テストファイルは 1 つのソースファイルにのみ対応する。複数のソースモジュールのテストを 1 つのファイルに混在させてはならない。

**規則 1.4**：テスト宣言はファイル形式
`mod tests;`（セミコロン付き）を使用しなければならず、同階層の `tests/`
ディレクトリを指す。**インライン形式 `mod tests { ... }`
を使用してテストコードをソースファイル内に直接記述することは禁止する。**

```rust
// ✅ 正しい——ファイル形式の宣言、テストコードは独立したファイルに
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
        // テストコードはソースファイル内に記述すべきでない
    }
}
```

**なぜインラインを禁止するのか？**

1. ソースファイルの責務を単一化：ソースファイルには実装のみ、テストファイルにはテストのみ。混在すると、テスト変更時にファイル末尾までスクロールし、実装変更時にテストをスキップすることになる。
2. モジュール境界の明確化：`tests/`
   ディレクトリは物理的な境界であり、どのモジュールにテストがあり、ないかが一目でわかる。
3. リファクタリングの安全性：モジュール分割時、`tests/`
   ディレクトリも一緒に移動する；インラインテストはソースファイルから手動で剥離する必要がある。
4. コードレビュー：PR の diff において、ソースコード変更とテスト変更が別ファイルに分離され、混在しない。

### モジュール宣言規範

**規則 2.1**：すべてのテストファイルの先頭には、モジュールレベルのドキュメントコメント `//!`
を記述し、テストがカバーする規範の出所（言語規範のセクション番号 +
RFC 番号）を説明しなければならない。あるテストがどの規範セクションも参照しない場合、そのコードには規範上の根拠がないことになる——それは存在すべきではない。

```rust
//! literal テスト — 言語規範 §2.6 に基づく
//!
//! §2.6.1: 整数 Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: 浮動小数点数（小数点と指数を含む）
//! §2.6.3: 文字列（エスケープシーケンス \\nrt'"\\, \\x, \\u{}）
//! RFC-012: F-String 補間
```

**なぜ規範を参照しなければならないのか？**
テストの期待値は規範から来るべきであり、「現在のコードの出力」から来るべきではない。いつかコードの出力に合わせてテストが更新されたなら、そのテストは何も保護していない。規範に紐付けられたテストのみが「意図的な breaking
change」と「意図しないリグレッション」を区別できる。

**規則 2.2**：テストモジュールの `use`
インポートは具体的な型/関数まで正確に行わなければならず、glob インポート `use super::*` を禁止する。

```rust
// 🟢 良い——精密なインポート
use crate::frontend::core::lexer::{tokenize, TokenKind};
use crate::frontend::core::parser::{ParserState, ParseError};

// 🔴 ゴミ——何をテストしているかが他人にはわからない
use super::*;
```

### 命名規範

**規則 3.1**：テスト関数の命名形式は `test_<what>_<scenario>`
とし、すべて小文字のアンダースコア区切りとする。

```rust
#[test]
fn test_tokenize_empty_string() { /* ... */ }
#[test]
fn test_parse_int_overflow() { /* ... */ }
#[test]
fn test_typecheck_fn_return_mismatch() { /* ... */ }
```

**規則 3.2**：テスト関数名は自己説明的でなければならない。関数名を読んだだけで何をテストし、何を期待するかがわかること。数字のシーケンス番号による命名を禁止する。

```rust
// 🟢 良い
fn test_skip_semicolon_success() { /* ... */ }
fn test_skip_semicolon_failure_when_identifier() { /* ... */ }

// 🔴 ゴミ——何をテストしているかが全くわからない
fn test_skip_1() { /* ... */ }
fn test_skip_2() { /* ... */ }
```

**規則 3.3**：ヘルパー関数は `test_` 接頭辞を必要とせず、動詞または名詞でその用途を記述する。

```rust
fn parse_expr(source: &str) -> Expr { /* ... */ }
fn tokenize_single(source: &str) -> Token { /* ... */ }
fn setup_parser_with_tokens(tokens: &[Token]) -> ParserState { /* ... */ }
```

### テスト構造規範 (Arrange-Act-Assert)

**規則 4.1**：各テスト関数は三段式構造、すなわち準備（Arrange）→ 実行（Act）→ 検証（Assert）に従わなければならず、三段の間は空行で区切る。

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

**規則 4.2**：単純なテスト（単一の呼び出し + 単一のアサーション）はセグメントコメントを省略してもよいが、論理コードが 5 行を超えないこと。5 行を超えるテストは明示的に三段を記述しなければならない。

### ヘルパー関数規範

**規則 5.1**：3 回以上繰り返される setup ロジックはヘルパー関数として抽出しなければならない。

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

**規則 5.2**：ヘルパー関数内の `unwrap()` / `expect()`
は、panic 時に十分なコンテキストを出力しなければならない。テスト関数本体内（`#[test] fn ...`）では直接
`unwrap()`
を使用できる——失敗時に Rust は自動的に行番号を出力する；しかしヘルパー関数内で失敗した場合、行番号はヘルパー関数の定義箇所を指し、呼び出し時のコンテキストが見えない。

```rust
// 🟢 良い——ヘルパー関数の失敗時にソースコードの内容を出力
fn run_ok(source: &str) {
    run(source).unwrap_or_else(|e| panic!("Execution failed:\nSource:\n{}\nError:\n{:?}", source, e));
}

// 🔴 ゴミ——失敗時にどのソースファイルが原因かが見えない
fn run_ok(source: &str) {
    run(source).unwrap();
}
```

**規則 5.3**：ヘルパー関数はテストファイルの先頭、`use`
インポートの直後に配置する。複数のテストモジュールで共有する場合は、`tests/mod.rs` に配置して
`pub(crate)` でエクスポートする。

### アサーションスタイル

**規則 6.1**：enum バリアントのマッチングは `assert!(matches!(...))` を優先し、`if let` + `panic!`
の使用を禁止する。

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

**規則 6.2**：精密な値の比較には `assert_eq!` を、ブールアサーションには `assert!`
を使用する。`assert!(a == b)` を `assert_eq!(a, b)` の代わりに使用することを禁止する。

**規則 6.3**：アサーション自体が失敗理由を完全に記述している場合を除き、すべてアサーションにはカスタムエラーメッセージを付与しなければならない。

```rust
// 🟢 良い——アサーション失敗時に迅速に問題を特定できる
assert!(
    state.infix_info().is_some(),
    "infix_info should handle '{op}'"
);

// 🟢 良い——assert_eq! 失敗時は値の差異を自動出力するため、追加メッセージ不要
assert_eq!(error_count, 0);

// 🔴 ゴミ——失敗時に「assertion failed」としか表示されない
assert!(state.infix_info().is_some());
```

**規則 6.4**：アサーションの順序は `assert_eq!(actual, expected)`
とし、実際の値が先、期待値が後とする。

### アンチパターン一覧

以下は禁止されている書き方と代替案である：

| アンチパターン                                         | 問題                                                                   | 代替案                                                                                     |
| ------------------------------------------------------ | ---------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| `#[cfg(test)] mod tests { ... }` インラインテスト      | ソースファイルの肥大化、モジュール境界の曖昧化、リファクタリングの困難 | テストコードを独立した `tests/` ディレクトリに配置し、`mod tests;` で宣言（規則 1.4 参照） |
| コードのエラー挙動にテストが迎合する                   | 規範の偏差を隠蔽し、バグを正当化する                                   | 規範に照らしてコードを修正し、テストを変更しない                                           |
| コードの出力から逆算してテストの期待値を決める         | テストが「現在の実装の録音機」になる                                   | 規範から期待値を導出する                                                                   |
| `#[ignore]` の恒久的な付与                             | 腐敗したテストを隠蔽する                                               | 修正または削除                                                                             |
| `println!` デバッグ出力                                | テスト出力を汚染する                                                   | `assert!` で明示的にアサート                                                               |
| `thread::sleep`                                        | ランダムな失敗 + 遅延                                                  | 同期機構または mock を使用                                                                 |
| テストで実際のファイルシステムを操作する               | 遅く、再現不可能                                                       | `tempfile` を使用                                                                          |
| テストの実行順序に依存する                             | ランダムな失敗                                                         | 各テストが独立して setup を行う                                                            |
| 1 つのテスト関数が 30 行を超える                       | 誰も理解できない                                                       | テストを分割するか、ヘルパー関数を使用                                                     |
| ヘルパー関数内の `unwrap()` がコンテキストを出力しない | 問題の特定が困難                                                       | `expect("why")` またはカスタム panic を使用（規則 5.2 参照）                               |
| 3 回以上の同一 setup のコピペ                          | 修正コストが高い                                                       | ヘルパー関数を抽出                                                                         |

---

## 統合テスト規範

### テスト構成

**規則 7.1**：統合テストはプロジェクトルートの `tests/` ディレクトリに配置する。エントリファイル
`tests/integration.rs` は `#[path]` 属性でサブモジュールをインクルードする。

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
ファイルは 1 つのテストテーマ（コンパイラバックエンド、コード生成、エクゼキュータなど）に対応し、混在させてはならない。

**規則 7.3**：統合テストはプロジェクトの公開 API 経由で行わなければならない。統合テストで `crate::`
内部モジュールを直接参照することは禁止。`yaoxiang::` 公開パスを使用する。

```rust
// 🟢 良い——公開 API 経由
use yaoxiang::run;

// 🔴 ゴミ——公開 API 境界を迂回している
use yaoxiang::middle::codegen::bytecode::BytecodeFile;
```

### テストデータ管理

**規則 8.1**：統合テストはインラインソース文字列を優先する。ソースが 30 行を超える場合に限り、外部 fixture ファイル（`tests/fixtures/`
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

**規則 8.2**：fixture ファイルは `.yx` 拡張子で終わり、ファイル名はテストの意図を記述する。

### E2E カバレッジ原則

**規則 9.1**：各言語機能の統合テストは 3 つのパスをカバーしなければならない：

| パス       | 説明                                                           |
| ---------- | -------------------------------------------------------------- |
| Happy path | 合法的な入力が予期される出力を生成する                         |
| Error path | 違法な入力が明確なエラーメッセージを生成する（panic ではない） |
| Boundary   | 境界値（空入力、最大値、ネスト深度の上限）                     |

**規則 9.2**：統合テストはネットワーク、システム環境変数、外部サービスに依存してはならない。

---

## ベンチマークテスト規範

### Criterion.rs 使用規範

**規則 10.1**：ベンチマークテストは `benches/` ディレクトリに統一配置し、エントリファイルは
`benches/lib.rs` とする。テストテーマごとにファイル分けする。

```
benches/
├── lib.rs              # エントリ、criterion_group/criterion_main を定義
├── lang_compare/
│   └── fibonacci.rs    # 言語横断比較ベンチマーク
├── parser.rs           # パーサベンチマーク
└── codegen.rs          # コード生成ベンチマーク
```

**規則 10.2**：各ベンチマーク関数にはモジュールドキュメントコメント `//!`
を記述し、テストの目的と測定指標を説明しなければならない。

```rust
//! YaoXiang インタプリタ性能ベンチマークテスト
//!
//! 測定指標：単一イテレーションの所要時間（wall time）
//! ベースライン：Rust ネイティブ実装
```

### コンパイラ最適化の防止

**規則 11.1**：すべてのベンチマークテストの被測定出力は `criterion::black_box`
を通じてコンパイラの最適化による削除を防止しなければならない。

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
クロージャ内で動的に生成してはならない——さもないとデータ生成 + 被測定ロジックの合計時間を測定することになる。

### ベンチマークグループ化と命名

**規則 12.1**：ベンチマークテストの命名形式は `<被測定モジュール>_<シナリオ>`
とし、すべて小文字のアンダースコア区切りとする。ユニットテストの命名規則と一致する。

**規則 12.2**：`criterion_group!`
を使用して関連ベンチマークを論理的にグループ化しなければならない。すべてのベンチマークを 1 つのグループに詰め込むことを禁止する。

```rust
criterion_group!(parser, bench_parse_expr, bench_parse_stmt);
criterion_group!(codegen, bench_codegen_module, bench_codegen_switch);
criterion_main!(parser, codegen);
```

---

## ドキュメントテスト規範

### 使用シナリオ

**規則 13.1**：すべての `pub`
関数、型、メソッドはドキュメントコメントに少なくとも 1 つの実行可能なコードサンプルを含めなければならない。このサンプルは
`cargo test --doc` で実行される。

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

**規則 13.2**：ドキュメントテストのコードサンプルはコンパイルを通過し、アサーションに成功しなければならない。当該サンプルがコンパイル時エラーを示す場合を除き、`ignore`
マークのサンプルを含めてはならない。

````rust
/// ```ignore
/// // コンパイル時エラーを示す——ignore 可
/// let x: int = "string";
/// ```
````

### カバレッジ要件

**規則 14.1**：ドキュメントテストは API の happy
path をカバーすればよい。境界ケースとエラーパスはユニットテストが担当する。

**規則 14.2**：ドキュメントテスト内のサンプルコードは簡潔でなければならない——10 行を超えない。サンプルにより長いコンテキストが必要な場合、API 設計に問題がある。

---

## プロパティテスト規範

### 使用シナリオ

**規則 15.1**：以下のシナリオでは、手書きで複数の境界値ケースを書くのではなく、プロパティテスト（proptest または quickcheck）を使用しなければならない：

| シナリオ                           | 例                                     |
| ---------------------------------- | -------------------------------------- |
| パーサの round-trip                | `parse(pretty_print(ast)) == ast`      |
| シリアライズ/デシリアライズ        | `deserialize(serialize(data)) == data` |
| 数学演算の恒等式                   | `a + b == b + a`                       |
| コンパイラ最適化が意味論を変えない | `eval(code) == eval(optimize(code))`   |

**規則 15.2**：プロパティテストは主要なプロパティテストフレームワークとして `proptest`
を使用する（`Cargo.toml` の `dev-dependencies` に宣言済み）。

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

**規則 16.2**：プロパティテストが失敗を発見した場合、`proptest`
のリグレッション機構を使用しなければならない——失敗した入力を `proptest-regressions/`
ディレクトリに追加し、通常のテストを手書きで代用しない。

---

## カバレッジ要件

### 新規コードカバレッジ目標

**規則 17.1**：新規コードのテストカバレッジ要件：

| コード種別                                           | 行カバレッジ | 分岐カバレッジ |
| ---------------------------------------------------- | ------------ | -------------- |
| コアコンパイラモジュール（frontend/middle/backends） | ≥ 85%        | ≥ 80%          |
| ユーティリティ/ヘルパーモジュール（util）            | ≥ 75%        | ≥ 70%          |
| ランタイムモジュール（vm/runtime）                   | ≥ 80%        | ≥ 75%          |
| 標準ライブラリ（std）                                | ≥ 75%        | ≥ 70%          |
| エラー処理と診断                                     | ≥ 90%        | ≥ 85%          |

**規則 17.2**：エラー処理パス（すべての `Err`
分岐）は 100% カバーされなければならない。ユーザに見えるエラーメッセージはテストで検証済みでなければならない。

### PR レビューチェックリスト

**規則 18.1**：PR を提出する前に、著者は以下の項目を自己点検しなければならない：

- [ ] `cargo test` がすべて通過する
- [ ] `cargo test --doc` がすべて通過する
- [ ] `cargo bench` に性能回帰がない（ホットパスの変更を伴う場合）
- [ ] 新規コードがカバレッジ目標を満たしている
- [ ] テスト命名が命名規範に準拠している
- [ ] 各テストファイルが対応する規範セクションを宣言している（規則 2.1）
- [ ] テストの期待値が「現在のコードの出力」ではなく規範定義から来ている
- [ ] `#[ignore]` マークのテストがない（明確な issue 番号コメントがある場合を除く）
- [ ] 不必要な `unwrap()` がない（`expect` またはカスタム panic メッセージを使用すべき）
- [ ] コミットメッセージが `:white_check_mark: test:` タイプを使用している
- [ ] **「コードの挙動が規範と一致しない」ためにテストの期待値を変更していない——変更するのはコードであり、テストではない**
- [ ] **インラインテストがない**（`#[cfg(test)] mod tests { ... }` は
      `mod tests;` + 独立ファイルに変更すること、規則 1.4 参照）

**規則 18.2**：レビュアーは以下の問題を含む PR を拒否しなければならない：

- happy path テストのみで、エラーパスが欠けている
- テストに `thread::sleep` が含まれる、または実行順序に依存する
- 3 回を超えるコピペテストコードがヘルパー関数化されていない
- テスト名が命名規範に準拠していない
- 恒久的な `#[ignore]` のテストが存在する
- **テストがコードのエラー挙動に迎合している**（コードと規範が一致しない時にコードではなくテストを変更する）
- **テストが対応する規範セクションを宣言していない**（規則 2.1 参照）
- **テストの期待値が規範定義ではなくコード出力から来ている**（逆算したテストはテストしないのと同義）
- **インラインテストが存在する**（`mod tests;` + 独立ファイルではなく
  `#[cfg(test)] mod tests { ... }`、規則 1.4 参照）
- テストが「panic しない」ことのみを検証し、具体的な挙動をアサートしていない
- コードのバグを露出する失敗テストを削除した（コードを修正して緑になるのではなく）

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

# テスト出力を表示（デフォルトでは stdout は非表示）
cargo test -- --nocapture

# シングルスレッドで実行（並行問題のトラブルシューティング）
cargo test -- --test-threads=1

# カバレッジレポートを生成（cargo-llvm-cov が必要）
cargo llvm-cov --html
```

### B. コミットメッセージテンプレート

テスト関連のコミットは以下のテンプレートに従わなければならない：

```
:white_check_mark: test(<scope>): <短い説明>

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

新しいテストモジュールを作成する際、以下のファイルを含めることを確認する：

```
# src/<module>/ ディレクトリ下に新規テストを追加
src/<module>/tests/
├── mod.rs          # モジュール宣言 + 共通ヘルパー関数
└── <subject>.rs    # テストファイル、被テストソースファイル名に対応

# tests/ ディレクトリ下に新規統合テストを追加
tests/
├── integration.rs   # 更新：#[path] 宣言を追加
└── integration/
    └── <topic>.rs   # 新規テストファイル
```

### D. 参考資料

- [YaoXiang 言語規範](../../design/language-spec.md) —— **テストの権威ある出典**
- [承認済み RFC](../../design/rfc/accepted/) —— **設計決定の権威ある出典**
- [Rust テストドキュメント](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Criterion.rs ユーザーガイド](https://bheisler.github.io/criterion.rs/book/)
- [proptest ドキュメント](https://docs.rs/proptest/latest/proptest/)
- [プロジェクトコミット規範](./commit-convention.md)
- [プロジェクト貢献ガイド](./contributing.md)

---

> 💡
> **覚えておくこと**：テストは「コードが動作するか」を検証するものではない——「コードが規範に合致しているか」を検証する。規範が変われば、テストは規範に従って変わる。コードが間違っていれば、コードを修正する。テストは変更しない。**コードは規範に仕え、テストは規範を守る。テストがコードに迎合した瞬間、すべての保護を失う。**

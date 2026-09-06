---
title: 'テスト作成規範'
description:
  YaoXiang
  プロジェクトのテスト作成に関する厳格な規範。単体テスト、統合テスト、ベンチマークテスト、ドキュメントテスト、プロパティテストの作成基準を定義する
---

# テスト作成規範

本ドキュメントは YaoXiang プロジェクトのテスト作成に関する厳格な規範を定義する。すべての貢献者は以下のルールを遵守しなければならず、違反者はコードレビューで修正を要求される。

---

## 目次

- [総則](#総則)
- [yx コーパスとライブラリテスト階層](#yx-コーパスとライブラリテスト階層)
- [単体テスト規範](#単体テスト規範)
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

| テスト種別         | 場所                     | フレームワーク             |
| ------------------ | ------------------------ | -------------------------- |
| 単体テスト         | `src/<module>/tests/`    | `#[test]` + `#[cfg(test)]` |
| 統合テスト         | `tests/`                 | `#[test]`                  |
| ベンチマークテスト | `benches/`               | Criterion.rs               |
| ドキュメントテスト | API ドキュメントコメント | `cargo test --doc`         |
| プロパティテスト   | 任意のテスト位置         | proptest / quickcheck      |

### 核心原則

**原則 0：テストの権威ある情報源は仕様であり、コードではない。**
これは本ドキュメントの最も重要な原則である。テストが検証するのはコードが仕様に従っているかどうかであり、コードが「現在の実装で動作する」かどうかではない。テストがコードの動作と仕様の不一致を発見した場合、**コードを修正し、絶対にテストを修正しない**。

仕様文書は以下に位置する：

- `docs/src/design/language-spec.md` —— 言語コア仕様
- `docs/src/design/rfc/accepted/` —— 承認済み RFC 設計文書

各テストファイルの先頭には対応する仕様章を宣言しなければならない（ルール 2.1 を参照）。すべての開発者は仕様書とテストを照らし合わせ、実装の正確性を検証できるべきである。逆に言えば、コードに対応する仕様の記述がない場合、それは存在すべきではなく、ましてやテストされるべきではない。

```rust
// 🟢 好——测试直接引用规范，验证代码是否遵循规范
//! 字面量测试 — 基于语言规范 §2.6
//!
//! §2.6.1: 整数 Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: 浮点数（带小数点和指数）
//! §2.6.3: 字符串（转义序列 \\nrt'"\\, \\x, \\u{}）
//! RFC-012: F-String 插值

#[test]
fn test_decimal_literal_parsing() {
    // 规范 §2.6.1: Decimal ::= [0-9][0-9_]*
    let result = parse_literal("42").unwrap();
    assert_eq!(result, Literal::Int(42));
}

// 🔴 垃圾——测试迁就了当前代码的实现行为，而非验证规范
#[test]
fn test_literal_1() {
    // 不知道这段代码对应规范的哪一节
    // 如果 parse_literal 返回了错误的值，这个测试会"绿灯通过"
    // 因为它只验证了函数不 panic
    let result = parse_literal("42");
    assert!(result.is_ok());
}
```

**シナリオ**：テストを作成していて、コードの動作が仕様と一致しないことに気づいた。2 つの選択肢がある：

| 誤った方法                                       | 正しい方法                                 |
| ------------------------------------------------ | ------------------------------------------ |
| テストを修正して「通過」させる                   | コードを修正して、動作を仕様に合わせる     |
| テストに `#[ignore]` を追加する                  | コードの実装を即座に修正する               |
| テストに特殊な条件分岐を追加してコードに迎合する | 分岐を削除し、テストに直接問題を露出させる |

覚えておいてほしい：**赤信号 = コードが間違っており、テストではない。**（テスト自体にバグがある場合は別の話。）

**原則 1：テストはドキュメントである。**
すべての開発者は、追加のコメントや外部ドキュメントなしに、テストを読むことで被テストコードの動作を理解できるべきである。

```rust
// 🟢 好——测试名说了被测什么、期望什么
#[test]
fn test_tokenize_empty_input_returns_eof() {
    let tokens = tokenize("").unwrap();
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].kind, TokenKind::Eof));
}

// 🔴 垃圾——谁也不知道这测的什么
#[test]
fn test_tokenize_1() {
    let tokens = tokenize("").unwrap();
    assert!(tokens.len() > 0);
}
```

**原則 2：ランダムな失敗はゼロトレランスである。**
テストはあらゆる環境で再現可能に実行できなければならない。乱数、システム時間、スレッドスケジューリング順序に依存するテストは、シードの固定またはモックへの置換を使用しなければならない。

**原則 3：1 つのテストは 1 つの事柄のみをテストする。**
テスト名を「と」で接続して複数の動作を記述する必要がある場合は、複数のテストに分割する。

```rust
// 🟢 好——每个测试只验证一个场景
#[test]
fn test_parse_int_positive() { /* ... */ }
#[test]
fn test_parse_int_zero() { /* ... */ }

// 🔴 垃圾——一个测试塞了太多无关内容
#[test]
fn test_parser() {
    // 测 tokenize，测 parse，测 typecheck，测 codegen...
}
```

**原則 4：実装ではなく動作をテストする。**
内部実装のリファクタリングがテストの失敗を引き起こすべきではない。実装コードを 1 行変更して 10 個のテストが失敗するなら、テストの書き方が間違っている。

ただし、ここに重要な区別がある：**「動作」の定義は仕様から来ており、現在のコードの動作から来るのではない。**
コードが動作を変更した場合（すなわち仕様と一致しない新しい動作）、テストは失敗しなければならない。これができない場合、テストは「コードに迎合するテスト」であり、バグを容易に通過させる。

```
仕様（language-spec.md / RFC）  ──定義──►  期待される動作  ──駆動──►  テスト
                                           │
現在のコード  ──実装──►  実際の動作  ──対比──►  テスト結果

実際の動作 ≠ 期待される動作の場合：
  テストは必ず失敗する（赤信号）  ──►  コードを修正  ──►  テストが通過する（緑信号）

実際の動作 = 期待される動作（ただし実装がひどい）：
  テストが通過  ──►  実装をリファクタリング  ──►  テストは依然として通過  ← これが原則 4 の意味するところ
```

**原則 5：フォールバック/互換/特定パターン対応のテストコードは書かない。**
テスト環境は完全に制御可能な環境である。テストをスキップするために `#[cfg(not(ci))]`
が必要な場合、そのテスト設計には根本的な問題があることを示している。

### 用語定義

| 用語               | 定義                                                                                      |
| ------------------ | ----------------------------------------------------------------------------------------- |
| 単体テスト         | 単一の関数またはモジュールの動作をテストし、外部システムに依存しない                      |
| 統合テスト         | 複数のモジュールの連携を、公共 API またはコマンドラインエントリポイントを介してテストする |
| ベンチマークテスト | コードの性能を計測し、性能リグレッションを検出する                                        |
| ドキュメントテスト | ドキュメントコメント内に埋め込まれた実行可能なコード例                                    |
| プロパティテスト   | ランダムな入力に基づいて不変条件（property）を検証するテスト                              |

### コミット規約との関連

すべてのテスト関連のコミットは `:white_check_mark: test:`
タイプを使用しなければならず、[コミット規約](./commit-convention.md)を参照する。

```
:white_check_mark: test(parser): 添加 Pratt 解析器中缀表达式测试
:white_check_mark: test(codegen): 补全 switch 语句 IR 生成测试
```

---

## yx コーパスとライブラリテスト階層

本規範は **Rust 側のテストコード** を規定する。YaoXiang 言語自体のテスト（`.yx`
コーパスとライブラリテスト）は被テスト対象によって 2 階層に分類され、システム設計と判定契約は RFC-036（§7 スイート収集 /
§8 ネガティブ 3 階層 / §9 テストシステム階層）に属し、コーパス作成の細則は
`tests/yaoxiang/TEST_STANDARDS.md` に属する：

- **言語利用可能性コーパス**（`tests/yaoxiang/`）——被テスト対象は言語自体である；std はアサーションツールとしてのみ使用される。コーパス内では失敗発生層によって 3 種類の判定に分類される：動作テスト / コンパイル時拒否テスト / 実行時失敗テスト
- **ライブラリテスト**（ライブラリに付属）——被テスト対象はライブラリの公開 API 契約である；std の yx レベルテストは
  `src/std/tests/` にあり、将来のユーザーパッケージのテストはパッケージ内で `[tool.test]`
  により検出される

`.yx` テストのファイルヘッダ形式、マーカー（`[test:error]` / `[test:ignore]` / `[test:runtime]` /
`预期: EXXXX`）とアサーション規約は TEST_STANDARDS.md に従う；判定解析はデュアル runner 共有の
`src/util/test_markers.rs` によって実装される（Rust 側、本規範の制約を受ける）。

---

## 単体テスト規範

### ファイル構成

**ルール 1.1**：単体テストの `tests/` ディレクトリは被テストモジュールの `mod.rs`
と**同じ階層**でなければならない。`tests/` は上位に集約せず、階層をまたいでまとめない。

```
src/frontend/core/parser/
├── mod.rs              # #[cfg(test)] mod tests; ——声明同级 tests/
├── ast.rs
├── pratt/
│   ├── mod.rs          # #[cfg(test)] mod tests; ——pratt 自己的测试
│   └── tests/
│       ├── mod.rs
│       ├── led.rs
│       ├── nud.rs
│       └── precedence.rs
└── tests/              # parser 模块级别的测试（不包含 pratt 子模块的内容）
    ├── mod.rs
    ├── ast.rs
    ├── expressions.rs
    ├── error_recovery.rs
    └── parser_state.rs
```

**关键判断标准：** **`tests/` 放在哪个目录，哪个目录的 `mod.rs` 就必须用 `#[cfg(test)] mod tests;`
声明它。**

**ルール 1.1 補足：上位への集約を禁止する。**
サブディレクトリモジュールのテストは、そのサブディレクトリ自身の `tests/`
に配置しなければならず、親階層の `tests/` に集約してはならない。

| モジュール種別                            | テスト場所                      | 例                                           |
| ----------------------------------------- | ------------------------------- | -------------------------------------------- |
| ディレクトリモジュール（`mod.rs` を持つ） | 当該ディレクトリ配下の `tests/` | `emitter/tests/`、`codes/tests/`             |
| 単一ファイルモジュール（`.rs` のみ）      | 親階層の `tests/`               | `session.rs` → `diagnostic/tests/session.rs` |

```text
# ✅ 正确：每个目录模块的测试各自独立
src/util/diagnostic/
├── codes/
│   ├── mod.rs              # #[cfg(test)] mod tests;
│   └── tests/              # ✅ codes 自己的测试
│       ├── mod.rs
│       └── codes.rs
├── emitter/
│   ├── mod.rs              # #[cfg(test)] mod tests;
│   └── tests/              # ✅ emitter 自己的测试
│       ├── mod.rs
│       ├── text.rs
│       └── ansi.rs
└── tests/                  # ✅ diagnostic 级别（单文件模块）
    ├── mod.rs
    ├── session.rs
    ├── suggest.rs
    └── collect.rs

# ❌ 错误：将 emitter 和 codes 的测试聚合到 diagnostic/tests/
src/util/diagnostic/
└── tests/
    ├── mod.rs              # ❌ 被迫声明 mod emitter; mod codes;
    ├── emitter/            # ❌ 应该在 emitter/tests/
    └── codes/              # ❌ 应该在 codes/tests/
```

#### 単一ファイルモジュールとディレクトリモジュールのテスト配置ルール

**核心的な違い**：モジュールの組織形式がテストの配置場所を決定する。

| モジュール種別             | 判定基準                                             | テスト場所                      | 例                                            |
| -------------------------- | ---------------------------------------------------- | ------------------------------- | --------------------------------------------- |
| **ディレクトリモジュール** | 独立したディレクトリと `mod.rs` を持つ               | 当該ディレクトリ配下の `tests/` | `inference/tests/`                            |
| **単一ファイルモジュール** | `.rs` ファイルのみで、独立したディレクトリを持たない | 親階層の `tests/`               | `overload.rs` → `typecheck/tests/overload.rs` |

**詳細説明**：

```
src/frontend/core/typecheck/
├── mod.rs                          # typecheck 模块的 mod.rs
├── checker.rs                      # 单文件模块
├── environment.rs                  # 单文件模块
├── overload.rs                     # 单文件模块
├── type_eval.rs                    # 单文件模块
├── dead_code.rs                    # 单文件模块
├── spawn_placement.rs              # 单文件模块
├── signature.rs                    # 单文件模块
├── types.rs                        # 单文件模块
│
├── tests/                          # ✅ typecheck 的测试目录
│   ├── mod.rs                      # 声明单文件模块的测试
│   ├── checker.rs                  # checker.rs 的测试
│   ├── environment.rs              # environment.rs 的测试
│   ├── overload.rs                 # overload.rs 的测试（单文件模块测试放这里）
│   ├── type_eval.rs                # type_eval.rs 的测试
│   ├── dead_code.rs                # dead_code.rs 的测试
│   ├── spawn_placement.rs          # spawn_placement.rs 的测试
│   ├── signature.rs                # signature.rs 的测试
│   └── types.rs                    # types.rs 的测试
│
├── inference/                      # 目录模块（有 mod.rs）
│   ├── mod.rs                      # #[cfg(test)] mod tests; ——声明同级 tests/
│   ├── expressions.rs
│   ├── statements.rs
│   ├── patterns.rs
│   ├── bounds.rs
│   ├── subtyping.rs
│   ├── generics.rs
│   ├── compatibility.rs
│   ├── scope.rs
│   ├── assignment.rs
│   └── tests/                      # ✅ inference 的测试目录
│       ├── mod.rs
│       ├── expressions.rs          # expressions.rs 的测试
│       ├── statements.rs           # statements.rs 的测试
│       └── ...
│
└── traits/                         # 已删除（逻辑合并进 types/trait_data.rs）
```

**なぜ単一ファイルモジュールのテストを親階層の `tests/` に配置するのか？**

単一ファイルモジュール（例：`overload.rs`）には独自の `mod.rs` がないため、`#[cfg(test)] mod tests;`
を宣言できない。Rust のモジュールシステムによれば、テストファイルは何らかの `mod.rs`
によって宣言されなければコンパイルできない。したがって、単一ファイルモジュールのテストは親階層の
`mod.rs` によって宣言され、親階層の `tests/` ディレクトリに配置される。

**判定フロー**：

```
モジュールに遭遇し、テストの配置場所を判断する
│
├── そのモジュールはディレクトリ（mod.rs を有する）か？
│   └── はい → 当該ディレクトリ配下に tests/ を作成し、当該ディレクトリの mod.rs が宣言する
│
├── そのモジュールは単一ファイル（.rs のみ）か？
│   └── はい → テストを親階層の tests/ ディレクトリに配置し、親階層の mod.rs が宣言する
│
└── 不確かな場合
    └── 独立したディレクトリと mod.rs があるかを確認する
```

**よくあるエラー**：

```
# ❌ 错误 1：为单文件模块创建独立的 tests/ 目录
src/frontend/core/typecheck/
├── overload.rs
└── overload/                       # ❌ 不应该为单文件模块创建目录
    └── tests/
        └── overload.rs

# ❌ 错误 2：在单文件模块内声明 #[cfg(test)] mod tests;
# overload.rs
#[cfg(test)]                        # ❌ 单文件模块不能这样声明
mod tests;                          # 因为没有 overload/tests/ 目录

# ✅ 正确做法：测试放在父级 tests/
src/frontend/core/typecheck/
├── overload.rs                     # 源文件
└── tests/
    └── overload.rs                 # 测试文件，由 typecheck/mod.rs 声明
```

⚠️ **アンチパターン——以下のように書かないこと：**

```
# ❌ 错误：子模块的测试集中到父级
src/frontend/core/types/
├── mod.rs              # 本应只声明 base 和 computation
├── base/
│   ├── mod.rs
│   └── var.rs
└── tests/              # ❌ 父级 tests/ 包含子模块的测试
    ├── mod.rs          # ❌ 被迫声明 mod base; mod computation;
    ├── base/           # ❌ 这部分应放在 base/tests/
    │   └── var.rs
    └── computation/    # ❌ 这部分应放在 computation/tests/
        └── ...
```

```
# ✅ 正确的做法：每个模块的测试各自独立
src/frontend/core/types/
├── mod.rs              # 只声明 pub mod base; pub mod computation;
├── base/
│   ├── mod.rs          # #[cfg(test)] mod tests; ——声明同级的 tests/
│   ├── var.rs
│   └── tests/
│       ├── mod.rs
│       └── var.rs
└── computation/
    ├── mod.rs          # #[cfg(test)] mod tests; ——声明同级的 tests/
    ├── operations.rs
    └── tests/
        ├── mod.rs
        └── operations.rs
```

**なぜ上位に集約できないのか？** Rust のモジュールシステムは、`#[cfg(test)] mod tests;`
が宣言箇所でテストファイルのコンパイルを決定することを要求している。`types/mod.rs` が `mod tests;`
を宣言する場合、`types/tests/` の内容は `types` モジュールのプライベートな内容となる——`base` や
`computation`
の領域に踏み込んではならない。各モジュールのテストはそのモジュールの内部実装の詳細であり、親モジュールのものではない。このルールはモジュールのリファクタリングにも同様に適用される：`types`
を `base` と `computation`
に分割する場合、テストも分割後のモジュールに従って移動すべきであり、元の場所に残すべきではない。**テストディレクトリはソースコードの構造をミラーリングするのではなく、モジュール境界に従う。**

**ルール 1.2**：`tests/mod.rs` はモジュールの宣言と re-export のみを担当し、テスト関数は配置しない。

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

**ルール 1.3**：各テストファイルは 1 つのソースファイルにのみ対応する。複数のソースモジュールのテストを 1 つのファイルに混在させることは許可されない。

**ルール 1.4**：テスト宣言はファイル形式 `mod tests;`（セミコロン付き）を使用し、同じ階層の `tests/`
ディレクトリを指すものとする。**インライン形式 `mod tests { ... }`
を使用してテストコードをソースファイル内に直接記述することは禁止する。**

```rust
// ✅ 正确——文件形式声明，测试代码在独立文件中
// src/frontend/core/parser/mod.rs
#[cfg(test)]
mod tests;

// 🔴 禁止——inline 形式，测试代码寄生在源文件内
// src/frontend/core/parser/mod.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        // 测试代码不应该出现在源文件中
    }
}
```

**なぜインラインを禁止するのか？**

1. **ソースファイルの責務を単一にする**：ソースファイルには実装のみを配置し、テストファイルにはテストのみを配置する。混在させると、テストを変更するにはファイル末尾までスクロールし、実装を変更するにはテストをスキップする必要がある。
2. **モジュール境界を明確にする**：`tests/`
   ディレクトリは物理的な境界であり、どのモジュールにテストがあり、どのモジュールにないかが一目でわかる。
3. **リファクタリングの安全性**：モジュールを分割する際、`tests/`
   ディレクトリはそれに従って移動する；インラインテストはソースファイルから手動で切り出す必要がある。
4. **コードレビュー**：PR diff 内でソースコードの変更とテストの変更は別ファイルとなり、混在しない。

### モジュール宣言規範

**ルール 2.1**：すべてのテストファイルの先頭にはモジュールレベルのドキュメントコメント `//!`
を記述し、テストがカバーする仕様の出典（言語仕様の章番号 +
RFC 番号）を説明しなければならない。テストがいかなる仕様章も参照していない場合、そのコードには仕様上の根拠がないことを意味する——それは存在すべきではない。

```rust
//! 字面量测试 — 基于语言规范 §2.6
//!
//! §2.6.1: 整数 Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: 浮点数（带小数点和指数）
//! §2.6.3: 字符串（转义序列 \\nrt'"\\, \\x, \\u{}）
//! RFC-012: F-String 插值
```

**なぜ仕様を参照しなければならないのか？**
テストの期待値は仕様から来るべきであり、「現在のコードの出力」から来るべきではない。ある日コードの出力が変更され、テストがそれに従って更新されるとしたら、テストは何も保護していない。仕様に基づくテストのみが「意図的な breaking
change」と「意図しないリグレッション」を区別できる。

**ルール 2.2**：テストモジュールの `use`
インポートは具体的な型/関数まで正確に行わなければならず、glob インポート `use super::*` は禁止する。

```rust
// 🟢 好——精确导入
use crate::frontend::core::lexer::{tokenize, TokenKind};
use crate::frontend::core::parser::{ParserState, ParseError};

// 🔴 垃圾——别人不知道你在测什么
use super::*;
```

### 命名規範

**ルール 3.1**：テスト関数の命名形式は `test_<what>_<scenario>`
とし、すべて小文字とアンダースコアで区切る。

```rust
#[test]
fn test_tokenize_empty_string() { /* ... */ }
#[test]
fn test_parse_int_overflow() { /* ... */ }
#[test]
fn test_typecheck_fn_return_mismatch() { /* ... */ }
```

**ルール 3.2**：テスト関数名は自己説明的である必要がある。関数名を読んだだけで、何をテストし、何を期待しているかがわかるようにする。数字による連番命名は禁止する。

```rust
// 🟢 好
fn test_skip_semicolon_success() { /* ... */ }
fn test_skip_semicolon_failure_when_identifier() { /* ... */ }

// 🔴 垃圾——完全不知道测什么
fn test_skip_1() { /* ... */ }
fn test_skip_2() { /* ... */ }
```

**ルール 3.3**：ヘルパー関数には `test_` 接頭辞は不要であり、動詞または名詞でその用途を記述する。

```rust
fn parse_expr(source: &str) -> Expr { /* ... */ }
fn tokenize_single(source: &str) -> Token { /* ... */ }
fn setup_parser_with_tokens(tokens: &[Token]) -> ParserState { /* ... */ }
```

### テスト構造規範 (Arrange-Act-Assert)

**ルール 4.1**：各テスト関数は 3 段構成に従わなければならない：準備（Arrange）→ 実行（Act）→ アサート（Assert）、各段の間は空行で区切る。

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

**ルール 4.2**：簡単なテスト（単一の呼び出し + 単一のアサーション）はセグメントコメントを省略できるが、5 行を超えるロジックコードを含めてはならない。5 行を超えるテストは 3 段構成を明示的に示さなければならない。

### ヘルパー関数規範

**ルール 5.1**：3 回以上繰り返される setup ロジックはヘルパー関数として抽出しなければならない。

```rust
// 🟢 好——提取公共 setup
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
は、panic 時に十分なコンテキストを出力しなければならない。テスト関数本体（`#[test] fn ...`）内では直接
`unwrap()`
を使用できる——失敗時に Rust が自動的に行番号を出力する；しかし、ヘルパー関数内で失敗した場合、行番号はヘルパー関数の定義箇所を指し、呼び出し時のコンテキストが表示されない。

```rust
// 🟢 好——辅助函数失败时打印源码内容
fn run_ok(source: &str) {
    run(source).unwrap_or_else(|e| panic!("Execution failed:\nSource:\n{}\nError:\n{:?}", source, e));
}

// 🔴 垃圾——失败时你看不到是哪个源文件导致的问题
fn run_ok(source: &str) {
    run(source).unwrap();
}
```

**ルール 5.3**：ヘルパー関数はテストファイルの先頭、`use`
インポートの直後に配置する。複数のテストモジュールで共有される場合は、`tests/mod.rs` に配置し
`pub(crate)` でエクスポートする。

### アサーションスタイル

**ルール 6.1**：列挙バリアントのマッチングには `assert!(matches!(...))` を優先的に使用し、`if let` +
`panic!` の使用は禁止する。

```rust
// 🟢 好
assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(42)));

// 🔴 垃圾
if let TokenKind::IntLiteral(v) = tokens[0].kind {
    assert_eq!(v, 42);
} else {
    panic!("Expected IntLiteral");
}
```

**ルール 6.2**：厳密な値の比較には `assert_eq!` を使用し、ブール値のアサーションには `assert!`
を使用する。`assert_eq!(a, b)` の代わりに `assert!(a == b)` を使用することは禁止する。

**ルール 6.3**：アサーション自体が失敗原因を完全に記述している場合を除き、すべてのアサーションにはカスタムエラーメッセージを付ける。

```rust
// 🟢 好——断言失败时能快速定位
assert!(
    state.infix_info().is_some(),
    "infix_info should handle '{op}'"
);

// 🟢 好——assert_eq! 失败时自动打印值差异，不需要额外消息
assert_eq!(error_count, 0);

// 🔴 垃圾——失败了只知道 "assertion failed"
assert!(state.infix_info().is_some());
```

**ルール 6.4**：アサーションの順序は `assert_eq!(actual, expected)`
でなければならず、実際の値が先、期待値が後とする。

### アンチパターン一覧

以下は禁止されている書き方と代替案である：

| アンチパターン                                         | 問題                                                                               | 代替案                                                                                             |
| ------------------------------------------------------ | ---------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| `#[cfg(test)] mod tests { ... }` インラインテスト      | ソースファイルが肥大化し、モジュール境界が曖昧になり、リファクタリングが困難になる | テストコードを独立した `tests/` ディレクトリに配置し、`mod tests;` で宣言する（ルール 1.4 を参照） |
| テストがコードのエラーある動作に迎合する               | 仕様の逸脱を隠し、バグを合法化する                                                 | 仕様に照らしてコードを修正し、テストは変更しない                                                   |
| コード出力からテスト期待値を逆算する                   | テストが「現在実装の録音機」になる                                                 | 仕様から期待値を導出する                                                                           |
| `#[ignore]` の恒久的なマーク                           | 腐敗したテストを隠す                                                               | 修正または削除する                                                                                 |
| `println!` によるデバッグ出力                          | テスト出力を汚染する                                                               | `assert!` を使用して明示的にアサートする                                                           |
| `thread::sleep`                                        | ランダムな失敗と低速化                                                             | 同期機構またはモックを使用する                                                                     |
| テスト内で実際のファイルシステムを操作する             | 低速で再現不可能                                                                   | `tempfile` を使用する                                                                              |
| テストの実行順序に依存する                             | ランダムな失敗                                                                     | 各テストは独立して setup する                                                                      |
| 1 つのテスト関数が 30 行を超えるロジックを含む         | 誰も理解できない                                                                   | テストを分割するかヘルパー関数を使用する                                                           |
| ヘルパー関数内の `unwrap()` がコンテキストを出力しない | 問題箇所の特定が困難                                                               | `expect("why")` またはカスタム panic を使用する（ルール 5.2 を参照）                               |
| 同じ setup を 3 回以上コピー＆ペーストする             | 変更コストが高い                                                                   | ヘルパー関数を抽出する                                                                             |

---

## 統合テスト規範

### テスト構成

**ルール 7.1**：統合テストはプロジェクトルートディレクトリの `tests/`
ディレクトリに配置する。エントリファイル `tests/integration.rs` は `#[path]`
属性を使用してサブモジュールをインクルードする。

```rust
// tests/integration.rs
#[path = "integration/backends.rs"]
mod backends;
#[path = "integration/codegen.rs"]
mod codegen;
#[path = "integration/execution.rs"]
mod execution;
```

**ルール 7.2**：`tests/integration/*.rs`
の各ファイルは 1 つのテストトピック（コンパイラバックエンド、コードジェネレータ、エグゼキュータなど）に対応し、混在させてはならない。

**ルール 7.3**：統合テストはプロジェクトの公共 API を通じてテストしなければならない。統合テスト内で
`crate::` 内部モジュールを直接参照することは禁止する。`yaoxiang::` 公共パスを使用する。

```rust
// 🟢 好——通过公共 API
use yaoxiang::run;

// 🔴 垃圾——绕过了公共 API 边界
use yaoxiang::middle::codegen::bytecode::BytecodeFile;
```

### テストデータ管理

**ルール 8.1**：統合テストではインラインのソース文字列を優先的に使用する。ソースが 30 行を超える場合にのみ、外部フィクスチャファイル（`tests/fixtures/`
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

**ルール 8.2**：フィクスチャファイルは `.yx` 拡張子で終わり、ファイル名はテストの意図を記述する。

### E2E カバレッジ原則

**ルール 9.1**：各言語機能の統合テストは 3 つのパスをカバーしなければならない：

| パス       | 説明                                                     |
| ---------- | -------------------------------------------------------- |
| Happy path | 合法な入力が予期される出力を生成する                     |
| Error path | 違法な入力が明確なエラー情報を生成する（panic ではない） |
| Boundary   | 境界値（空入力、最大値、ネストの深さ上限）               |

**ルール 9.2**：統合テストはネットワーク、システム環境変数、外部サービスに依存してはならない。

---

## ベンチマークテスト規範

### Criterion.rs 使用規範

**ルール 10.1**：ベンチマークテストは `benches/` ディレクトリに統一して配置し、エントリファイルは
`benches/lib.rs` とする。テストトピックごとにファイル分割する。

```
benches/
├── lib.rs              # 入口，定义 criterion_group/criterion_main
├── lang_compare/
│   └── fibonacci.rs    # 跨语言对比基准
├── parser.rs           # 解析器基准
└── codegen.rs          # 代码生成基准
```

**ルール 10.2**：各ベンチマーク関数にはモジュールドキュメントコメント `//!`
を含め、テストの目的と測定指標を説明しなければならない。

```rust
//! YaoXiang 解释器性能基准测试
//!
//! 测量指标：单次迭代耗时（wall time）
//! 基准线：Rust 原生实现
```

### コンパイラ最適化の防止

**ルール 11.1**：すべてのベンチマークテストの被テスト出力は、`criterion::black_box`
を介してコンパイラによる最適化削除を防止しなければならない。

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
である必要があり、`iter`
クロージャ内で動的に生成してはならない——そうでないと、データ生成と被テストロジックの合計時間が測定される。

### ベンチマークのグループ化と命名

**ルール 12.1**：ベンチマークテストの命名形式は `<被测试モジュール>_<シナリオ>`
とし、すべて小文字とアンダースコアで区切る。単体テストの命名ルールと一致する。

**ルール 12.2**：`criterion_group!`
を使用して関連するベンチマークを論理的にグループ分けしなければならない。すべてのベンチマークを 1 つのグループに詰め込むことは禁止する。

```rust
criterion_group!(parser, bench_parse_expr, bench_parse_stmt);
criterion_group!(codegen, bench_codegen_module, bench_codegen_switch);
criterion_main!(parser, codegen);
```

---

## ドキュメントテスト規範

### 使用シナリオ

**ルール 13.1**：すべての `pub`
関数、型、メソッドはドキュメントコメント内に少なくとも 1 つの実行可能なコード例を含めなければならない。この例は
`cargo test --doc` で実行される。

````rust
/// 将源码字符串分词为 Token 序列。
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

**ルール 13.2**：ドキュメントテストのコード例はコンパイルが通り、アサーションが成功しなければならない。コンパイル時エラーを示す例を除き、`ignore`
マーク付きの例を含めてはならない。

````rust
/// ```ignore
/// // 展示编译期错误——可以 ignore
/// let x: int = "string";
/// ```
````

### カバレッジ要件

**ルール 14.1**：ドキュメントテストは API の happy
path をカバーすればよい。境界ケースとエラーパスは単体テストがカバーする。

**ルール 14.2**：ドキュメントテスト内のサンプルコードは簡潔である必要がある——10 行を超えないこと。例により長いコンテキストが必要な場合、API 設計に問題があることを示している。

---

## プロパティテスト規範

### 使用シナリオ

**ルール 15.1**：以下のシナリオでは、複数の境界値ケースを手書きするのではなく、プロパティテスト（proptest または quickcheck）を使用しなければならない：

| シナリオ                         | 例                                     |
| -------------------------------- | -------------------------------------- |
| パーサの round-trip              | `parse(pretty_print(ast)) == ast`      |
| シリアライズ/デシリアライズ      | `deserialize(serialize(data)) == data` |
| 数学演算の恒等式                 | `a + b == b + a`                       |
| コンパイラ最適化が意味を変えない | `eval(code) == eval(optimize(code))`   |

**ルール 15.2**：プロパティテストでは、主要なプロパティテストフレームワークとして `proptest`
を使用する（既に `Cargo.toml` の `dev-dependencies` で宣言済み）。

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

**ルール 16.1**：各プロパティテストには明確なプロパティ宣言が必要である——コメント内に検証する不変条件を記述する。

```rust
// 属性：任意整数字面量在 tokenize → tokens_to_string 后产生相同值
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
ディレクトリに追加し、手書きの通常のテストで代用しない。

---

## カバレッジ要件

### 新規コードのカバレッジ目標

**ルール 17.1**：新規コードのテストカバレッジ要件：

| コード種別                                           | 行カバレッジ | 分岐カバレッジ |
| ---------------------------------------------------- | ------------ | -------------- |
| コアコンパイラモジュール（frontend/middle/backends） | ≥ 85%        | ≥ 80%          |
| ユーティリティ/ヘルパーモジュール（util）            | ≥ 75%        | ≥ 70%          |
| ランタイムモジュール（vm/runtime）                   | ≥ 80%        | ≥ 75%          |
| 標準ライブラリ（std）                                | ≥ 75%        | ≥ 70%          |
| エラーハンドリングと診断                             | ≥ 90%        | ≥ 85%          |

**ルール 17.2**：エラーハンドリングパス（すべての `Err`
分岐）は 100% カバーしなければならない。ユーザーが見ることができるエラーメッセージはテストによって検証されていなければならない。

### PR レビューチェックリスト

**ルール 18.1**：PR を提出する前に、著者は以下の項目を自己チェックしなければならない：

- [ ] `cargo test` がすべて成功する
- [ ] `cargo test --doc` がすべて成功する
- [ ] `cargo bench` に性能リグレッションがない（ホットパスの変更に関わる場合）
- [ ] 新規コードがカバレッジ目標に適合している
- [ ] テスト命名が命名規約に適合している
- [ ] 各テストファイルが対応する仕様章を宣言している（ルール 2.1）
- [ ] テスト期待値が「現在のコードの出力」ではなく、仕様の定義から来ている
- [ ] `#[ignore]` マーク付きのテストがない（明確な issue 番号の注釈がある場合を除く）
- [ ] 不要な `unwrap()` がない（`expect` またはカスタム panic メッセージを使用すべき）
- [ ] コミットメッセージが `:white_check_mark: test:` タイプを使用している
- [ ] **「コードの動作と仕様が一致しない」ためにテスト期待値を変更していない——変更するのはコードであり、テストではない**
- [ ] **インラインテストがない**（`#[cfg(test)] mod tests { ... }` は
      `mod tests;` + 独立ファイルに変更しなければならない、ルール 1.4 を参照）

**ルール 18.2**：Reviewer は以下の問題を含む PR を拒否しなければならない：

- happy path テストのみで、エラーパスが欠けている
- テストに `thread::sleep` が含まれている、または実行順序に依存している
- テストコードのコピー＆ペーストが 3 回を超え、ヘルパー関数が抽出されていない
- テスト名が命名規約に適合していない
- 恒久的な `#[ignore]` 付きのテストが存在する
- **テストがコードのエラーある動作に迎合している**（コードと仕様が一致しない時にテストを修正し、コードを修正しない）
- **テストが対応する仕様章を宣言していない**（ルール 2.1 を参照）
- **テスト期待値がコード出力から来ており、仕様定義から来ていない**（逆算されたテストはテストしていないのと同じ）
- **インラインテストが存在する**（`mod tests;` + 独立ファイルではなく、`#[cfg(test)] mod tests { ... }`、ルール 1.4 を参照）
- テストが「panic しない」のみを検証しており、具体的な動作をアサートしていない
- コードのバグを露出させる失敗テストを削除した（コードを修正してから緑になるのではなく）

---

## 付録

### A. テストコマンド早見表

```bash
# 运行所有测试
cargo test

# 只运行单元测试
cargo test --lib

# 只运行集成测试
cargo test --test integration

# 只运行文档测试
cargo test --doc

# 运行特定测试（按名称过滤）
cargo test test_parse_expr

# 运行基准测试
cargo bench

# 显示测试输出（默认隐藏 stdout）
cargo test -- --nocapture

# 单线程运行（排查并发问题）
cargo test -- --test-threads=1

# 生成覆盖率报告（需要 cargo-llvm-cov）
cargo llvm-cov --html
```

### B. コミットメッセージテンプレート

テスト関連のコミットは以下のテンプレートに従わなければならない：

```
:white_check_mark: test(<scope>): <简短描述>

<可选：覆盖的场景列表>
```

例：

```
:white_check_mark: test(parser): 添加 Pratt 解析器中缀运算符测试

覆盖场景：
- 算术运算符优先级（+, -, *, /, %）
- 比较运算符链接（1 < x < 10）
- 逻辑运算符短路
- 赋值运算符右结合
```

### C. 新規テストファイル一覧

新しいテストモジュールを作成する際、以下のファイルが含まれることを確認する：

```
# 在 src/<module>/ 目录下新增测试
src/<module>/tests/
├── mod.rs          # 模块声明 + 公共辅助函数
└── <subject>.rs    # 测试文件，对应被测源文件命名

# 在 tests/ 目录下新增集成测试
tests/
├── integration.rs   # 更新：添加 #[path] 声明
└── integration/
    └── <topic>.rs   # 新测试文件
```

### D. 参考資料

- [YaoXiang 言語仕様](../../design/language-spec.md) —— **テストの権威ある情報源**
- [承認済み RFC](../../design/rfc/accepted/) —— **設計判断の権威ある情報源**
- [Rust テストドキュメント](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Criterion.rs ユーザーガイド](https://bheisler.github.io/criterion.rs/book/)
- [proptest ドキュメント](https://docs.rs/proptest/latest/proptest/)
- [プロジェクトコミット規約](./commit-convention.md)
- [プロジェクト貢献ガイド](./contributing.md)

---

> 💡
> **覚えておくこと**：テストはあなたのコードが「動く」かどうかを検証するのではない——あなたのコードが仕様に従っているかどうかを検証する。仕様が変わり、テストは仕様に従って変わる。コードが間違っていたら、テストではなくコードを修正する。**コードは仕様に仕え、テストは仕様を守る。テストがコードに迎合した瞬間、すべての保護を失う。**

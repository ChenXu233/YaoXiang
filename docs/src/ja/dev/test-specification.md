---
title: 'テスト記述規範'
description:
  YaoXiangプロジェクトのテスト記述に関する厳格な規範。ユニットテスト、統合テストベンチマークテスト、ドキュメンテーションテスト、プロパティテストの記述標準を定義
---

# テスト記述規範

本文書は YaoXiang プロジェクトのテスト記述に関する厳格な規範を定義します。全コントリビューターは以下 のルールに従う必要があります。違反者は Code Review で修正を 要求されます。

---

## 目次

- [総則](#総則)
- [yx コーパスとライブラリのテストレイヤー](#yx-コーパスとライブラリのテストレイヤー)
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

本規範は YaoXiang プロジェクト内の全 Rust テストコードに適用されます：

| テスト種別 | 位置                  | フレームワーク                     |
| ---------- | --------------------- | ---------------------------------- |
| ユニットテスト | `src/<module>/tests/` | `#[test]` + `#[cfg(test)]`        |
| 統合テスト | `tests/`              | `#[test]`                          |
| ベンチマークテスト | `benches/`        | Criterion.rs                       |
| ドキュメンテーションテスト | API ドキュメントコメント | `cargo test --doc`  |
| プロパティテスト | 任意のテスト位置    | proptest / quickcheck              |

### コア原則

**原則 0：テストの権威あるソースは規範であり、コードではない。**
これは本文書で最も重要な原則です。テストはコードが規範に従っているかを検証ものであり、"現在の実装で 通るかどうか"を検証するものではありません。テストがコードの動作が規範と不一致を発見した場合、**コードを修正し、テストを修正しない。**

規範ファイルは以下の場所にあります：

- `docs/src/design/language-spec.md` —— 言語コア規範
- `docs/src/design/rfc/accepted/` —— 採用された RFC 設計文書

各テストファイルの先頭には対応する規範セクションを宣言する必要があります（規則 2.1 参照）。すべての開発者は規範 文書を持ってテストと照合し、実装の正しさを検証できるべきです。逆来说——もしあるコードに対応する規範記述がない場合、それは存在するべきではなく、ましてやテストされるべきではない。

```rust
// 🟢 良い——テストは直接規範を参照し、コードが規範に従うかを検証
//! リテラルテスト — 言語規範 §2.6 に基づく
//!
//! §2.6.1: 整数 Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: 浮動小数点数（小数点と指数付き）
//! §2.6.3: 文字列（エスケープシーケンス \\nrt'"\\, \\x, \\u{}）
//! RFC-012: F-String 補間

#[test]
fn test_decimal_literal_parsing() {
    // 規範 §2.6.1: Decimal ::= [0-9][0-9_]*
    let result = parse_literal("42").unwrap();
    assert_eq!(result, Literal::Int(42));
}

// 🔴 悪い——テストが現在のコードの実装動作に我慢している，而非検証規範
#[test]
fn test_literal_1() {
    // このコードが規範のどの節に対応するかわからない
    // parse_literal が誤った値を返しても、このテストは"緑で通過"する
    // 関数内で panic しないことだけを検証しているため
    let result = parse_literal("42");
    assert!(result.is_ok());
}
```

**シナリオ**：テストを作成し、コード動作が規範不符であることを発見しました。2つの選択肢があります：

| 誤った做法                             | 正しい做法                             |
| -------------------------------------- | -------------------------------------- |
| テストを"通過するように"修正する       | コードを修正し、動作を規範に合わせる   |
| テストに `#[ignore]` を追加する        | 直ちにコード実装を修正する             |
| テストにコードを我慢する特殊条件分岐を追加 | 分岐を削除し、テストに直接問題を露呈させる |

覚えておいてください：**赤 сигнал = コードが間違っている，而不是 тест。**（ただしテスト自体にバグがある場合は別話。）

**原則 1：テストは文書である。** すべての開発者はテストを読むことで 被テストコードの動作を理解できるべきです。追加のコメントや外部 文書なしで理解できるべきです。

```rust
// 🟢 良い——テスト名が何をテストし、何を期待するかを説明している
#[test]
fn test_tokenize_empty_input_returns_eof() {
    let tokens = tokenize("").unwrap();
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].kind, TokenKind::Eof));
}

// 🔴 悪い——何テストしているのか誰もわからない
#[test]
fn test_tokenize_1() {
    let tokens = tokenize("").unwrap();
    assert!(tokens.len() > 0);
}
```

**原則 2：ランダムな失敗はゼロ容忍。**
テストは任意の環境下で繰り返し実行可能でなければならなりません。乱数、システム時刻、スレッドスケジューリング順序に依存するテストは、シード固定または mock を使用する必要があります。

**原則 3：1つのテストは1つのことだけをテストする。** テスト名が"と"で複数の動作を 连接する必要がある場合、複数のテストに拆分してください。

```rust
// 🟢 良い——各テストは1つのシナリオのみを検証
#[test]
fn test_parse_int_positive() { /* ... */ }
#[test]
fn test_parse_int_zero() { /* ... */ }

// 🔴 悪い——1つのテストに無関係な内容が多すぎる
#[test]
fn test_parser() {
    // tokenize をテストし、parse をテストし、typecheck をテストし、codegen をテスト...
}
```

**原則 4：動作をテストし、実装をテストしない。**
内部実装のリファクタリングはテスト失敗を引き起こすべきではない。実装コードの1行を変えて10個のテストがコケたら、テストの書き方が間違っている。

しかしここに重要な区別があります：**"動作"の定義は規範から来ものであり、現在のコードの動作からは来ない。**
コードが動作を変えた場合（規範不符の新しい動作）、テストは失敗しなければならない。これ做不到的话、你的测试就是"遷就コードのテスト"——それはバグを侵入させます。

```
規範（language-spec.md / RFC）  ──定義──►  期待動作  ──駆動──►  テスト
                                           │
現在のコード  ──実装──►  実際動作  ──対比──►  テスト結果

もし実際動作 ≠ 期待動作：
  テストは失敗しなければならない（赤 сигнал）  ──►  コードを修正  ──►  テスト通過（緑 сигнал）

もし実際動作 = 期待動作（だが実装がだめな場合）：
  テスト通過  ──►  実装をリファクタリング  ──►  テストは依然通過  ← これこそ原則 4 の意味
```

**原則 5：フォールバック/互換性/特定パターンが有効になるテストコードを書かない。** テスト環境は完全に制御できる環境如果你需要
`#[cfg(not(ci))]` を使用して某个テストをスキップする場合、そのテスト設計に根本的な問題があることを意味します。

### 用語定義

| 用語       | 定義                                        |
| ---------- | ------------------------------------------- |
| ユニットテスト | 単一関数またはモジュール動作をテストし、外部システムに依存しない |
| 統合テスト | 複数モジュール協業をテストし、パブリック API またはコマンドライン入口を使用 |
| ベンチマークテスト | コードパフォーマンスを測定し、パフォーマンスリグレッションを検出 |
| ドキュメンテーションテスト | ドキュメントコメントに埋め込まれた実行可能なコード例 |
| プロパティテスト | ランダム入力に基づいて不変量（property）を検証するテスト |

### 提交規範との関連

すべてのテスト関連提交は `:white_check_mark: test:` タイプを使用する必要があります。[提交規範](./commit-convention.md)を参照。

```
:white_check_mark: test(parser): Pratt パーサーの中置式テストを追加
:white_check_mark: test(codegen): switch 文の IR 生成テストを補完
```

---

## yx コーパスとライブラリのテストレイヤー

本規範は **Rust 側テストコード**を制約します。YaoXiang 言語自体のテスト（`.yx` コーパスとライブラリテスト）は 被テスト対象に分け2層、システム設計と判定契約は RFC-036（§7 スイート収集 / §8 負向三層 / §9 テスト体系レイヤー）に、コーパス記述細則は `tests/yaoxiang/TEST_STANDARDS.md` に所属します：

- **言語可用性コーパス**（`tests/yaoxiang/`）—— 被テスト対象は言語自体；std はアサーション道具のみとして使用。 コーパス内失敗発生層で3類判定：動作テスト / コンパイル期拒否テスト / 実行期失敗テスト
- **ライブラリセット**（ライブラリに従う）—— 被テスト対象はライブラリの公開 API 契約；std の yx 級テストは `src/std/tests/` に、未来のユーザーパッケージテストはパッケージ内で `[tool.test]` 発見に従う

`.yx` テストのファイルヘッダー形式、ヘッダー命令（`// expect:` / `// skip:` / `// mode:`、 RFC-036 §8.2）とアサーション約束は TEST_STANDARDS.md を標準とする；判定解析は2つの runner が共用する `src/util/test_markers.rs` で実装（Rust 側、本規範の制約を受ける）。

---

## ユニットテスト規範

### ファイル構成

**規則 1.1**：ユニットテストの `tests/` ディレクトリは被テストモジュールの `mod.rs` **同レベル**に 配置する必要があります。`tests/` は 上に集約せず、跨レベルでもない。

```
src/frontend/core/parser/
├── mod.rs              # #[cfg(test)] mod tests; ——同レベル tests/ を宣言
├── ast.rs
├── pratt/
│   ├── mod.rs          # #[cfg(test)] mod tests; ——pratt 自身のテスト
│   └── tests/
│       ├── mod.rs
│       ├── led.rs
│       ├── nud.rs
│       └── precedence.rs
└── tests/              # parser モジュールのテスト（pratt サブモジュール内容は含まない）
    ├── mod.rs
    ├── ast.rs
    ├── expressions.rs
    ├── error_recovery.rs
    └── parser_state.rs
```

重要な判断基準：**`tests/` を配置するディレクトリには、そのディレクトリの `mod.rs` で `#[cfg(test)] mod tests;` を宣言する必要があります。**

**規則 1.1 補足：上への集約を禁止。** サブディレクトリモジュールのテストはそのサブディレクトリ自身の `tests/` に 配置し、親レベルの `tests/` に集約してはならない。

| モジュール種別             | テスト位置            | 例                                         |
| ------------------------ | ------------------- | ------------------------------------------ |
| ディレクトリモジュール（`mod.rs` あり） | そのディレクトリ下の `tests/` | `emitter/tests/`、`codes/tests/`         |
| 単一ファイルモジュール（`.rs` のみ） | 親の `tests/`     | `session.rs` → `diagnostic/tests/session.rs` |

```text
# ✅ 正しい：各ディレクトリモジュールのテストは互いに独立
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
    ├── mod.rs              # ❌ mod emitter; mod codes; を宣言強いられる
    ├── emitter/            # ❌ emitter/tests/ にあるべき
    └── codes/              # ❌ codes/tests/ にあるべき
```

#### 単一ファイルモジュール vs ディレクトリモジュールのテスト配置規則

**コアな違い**：モジュールの構成形式がテストの配置位置を決定します。

| モジュール種別 | 判断根拠                | テスト位置            | 例                                          |
| -------------- | ----------------------- | ------------------- | --------------------------------------------- |
| **ディレクトリモジュール** | 独立ディレクトリ と `mod.rs` あり | そのディレクトリ下の `tests/` | `inference/tests/`                            |
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
│   ├── overload.rs                 # overload.rs のテスト（単一ファイルモジュールテストはこちら）
│   ├── type_eval.rs                # type_eval.rs のテスト
│   ├── dead_code.rs                # dead_code.rs のテスト
│   ├── spawn_placement.rs          # spawn_placement.rs のテスト
│   ├── signature.rs                # signature.rs のテスト
│   └── types.rs                    # types.rs のテスト
│
├── inference/                      # ディレクトリモジュール（mod.rs あり）
│   ├── mod.rs                      # #[cfg(test)] mod tests; ——同レベル tests/ を宣言
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

**なぜ単一ファイルモジュールのテストは親 `tests/` に 配置するのか？**

単一ファイルモジュール（例：`overload.rs`）には独自の `mod.rs` がなく、`#[cfg(test)] mod tests;` を宣言できません。 Rust モジュールシステムにより、テストファイルはある `mod.rs` で宣言されなければコンパイルできません。そのため、単一ファイルモジュールのテストは親モジュールの `mod.rs` で宣言され、親の `tests/` ディレクトリに 配置する必要があります。

**判断フロー**：

```
モジュールに出会ったとき、テストをどこに配置するか？
│
├── そのモジュールはディレクトリか（mod.rs あり）？
│   └── はい → そのディレクトリ下に tests/ を作成し、そのディレクトリ mod.rs で宣言
│
├── そのモジュールは単一ファイルか（.rs のみ）？
│   └── はい → テストは親の tests/ ディレクトリに 配置し、親の mod.rs で宣言
│
└── 不確定？
    └── 独立ディレクトリ と mod.rs があるかを確認
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
#[cfg(test)]                        # ❌ 単一ファイルモジュールはこうして宣言できない
mod tests;                          # overload/tests/ ディレクトリがないため

# ✅ 正しい做法：テストは親の tests/ に 配置
src/frontend/core/typecheck/
├── overload.rs                     # ソースファイル
└── tests/
    └── overload.rs                 # テストファイル、typecheck/mod.rs で宣言
```

⚠️ ** антиパターン—— 이렇게 하지 마세요:**

```
# ❌ 誤り：サブモジュールのテストを親レベルに集中
src/frontend/core/types/
├── mod.rs              # 本来は base と computation のみを宣言するはず
├── base/
│   ├── mod.rs
│   └── var.rs
└── tests/              # ❌ 親 tests/ にサブモジュールのテストが含まれている
    ├── mod.rs          # ❌ mod base; mod computation; を宣言強いられる
    ├── base/           # ❌ これは base/tests/ にあるべき
    │   └── var.rs
    └── computation/    # ❌ これは computation/tests/ にあるべき
        └── ...
```

```
# ✅ 正しい做法：各モジュールのテストは互いに独立
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

**なぜ上へ集約できないのか？** Rust のモジュールシステムは `#[cfg(test)] mod tests;` が宣言場所でテストファイルのコンパイルを決定ためです。`types/mod.rs` が `mod tests;` を宣言すると、`types/tests/` の内容は `types` モジュールのプライベート内容になります——それは `base` や `computation` の領域に跨いで入るべきではないです。各モジュールのテストはそのモジュールの内部実装詳細であり、親モジュールのものではありません。この規則はモジュールリファクタリングにも適用されます：`types` を `base` と `computation` に分割するとき、テストも分割後のモジュールに従うべきです，而不是残留原地。**テストディレクトリはソースコード構造をミラーするのではなく、モジュール境界に従います。**

**規則 1.2**：`tests/mod.rs` はモジュールの宣言と re-export のみ担当し、テスト関数は 配置しません。

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

**規則 1.3**：各テストファイルは1つのソースファイルのみに対応します。複数のソースモジュールのテストを1つのファイルに混在させることは許可されません。

**規則 1.4**：テストの宣言はファイル形式 `mod tests;`（セミコロン付き）を使用し、同レベルの `tests/` ディレクトリを指す必要があります。**inline 形式 `mod tests { ... }` を使用してテストコードをソースファイル内に直接書くことは禁止です。**

```rust
// ✅ 正しい——ファイル形式で宣言、テストコードは独立ファイル
// src/frontend/core/parser/mod.rs
#[cfg(test)]
mod tests;

// 🔴 禁止——inline 形式、テストコードがソースファイルに寄生
// src/frontend/core/parser/mod.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        // テストコードはソースファイルに現れるべきではない
    }
}
```

**なぜ inline を禁止するのか？**

1. ソースファイルの責任は単一：ソースファイルは実装のみ配置し、テストファイルはテストのみ配置。混在させると、テスト修正はファイル末尾にスクロールし、実装修正はテストをスキップする必要がある。
2. モジュール境界が明確：`tests/` ディレクトリは物理的境界であり、どのモジュールにテストがあり、どれがないかが一目瞭然。
3. リファクタリング安全：モジュール分割時、`tests/` ディレクトリは従う；inline テストはソースファイルから手動で切り離す必要がある。
4. コードレビュー：PR diff でソースコード変更とテスト変更は別ファイルであり、混在しない。

### モジュール宣言規範

**規則 2.1**：すべてのテストファイルの先頭にモジュールレベルドキュメントコメント `//!` が必要で、テストがカバーする規範ソース（言語規範セクション番号 + RFC 番号）を説明します。特定のテストがどの規範セクションも参照していない場合、そのコードには規範根拠がないことを意味します——存在すべきではない。

```rust
//! リテラルテスト — 言語規範 §2.6 に基づく
//!
//! §2.6.1: 整数 Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: 浮動小数点数（小数点と指数付き）
//! §2.6.3: 文字列（エスケープシーケンス \\nrt'"\\, \\x, \\u{}）
//! RFC-012: F-String 補間
```

**なぜ規範を参照する必要があるのか？**
テストの期待値は規範から来るものであり、"現在のコードの出力"からは来るべきではないからです。もし有一天コードが出力を変えたのにテストも随之更新された場合、そのテストは何も保護していません。規範にアンカーされたテストだけが"意図的な breaking change"と"意図しないリグレッション"を区別できます。

**規則 2.2**：テストモジュールの `use` インポートは具体的な型/関数に正確でなければならず、glob インポート `use super::*` は禁止です。

```rust
// 🟢 良い——正確なインポート
use crate::frontend::core::lexer::{tokenize, TokenKind};
use crate::frontend::core::parser::{ParserState, ParseError};

// 🔴 悪い——何テストしているのか他人にはわからない
use super::*;
```

### 命名規範

**規則 3.1**：テスト関数名の形式は `test_<what>_<scenario>`、全小文字アンダースコア 区切り。

```rust
#[test]
fn test_tokenize_empty_string() { /* ... */ }
#[test]
fn test_parse_int_overflow() { /* ... */ }
#[test]
fn test_typecheck_fn_return_mismatch() { /* ... */ }
```

**規則 3.2**：テスト関数名は自己説明的である必要があります。関数名读完就能知道何をテストし、何を期待するか。数字シーケンスによる命名は禁止。

```rust
// 🟢 良い
fn test_skip_semicolon_success() { /* ... */ }
fn test_skip_semicolon_failure_when_identifier() { /* ... */ }

// 🔴 悪い——何をテストするのか全くわからない
fn test_skip_1() { /* ... */ }
fn test_skip_2() { /* ... */ }
```

**規則 3.3**：ヘルパー関数には `test_` プレフィックスは不要で、その用途を説明する動詞または名詞を使用する必要があります。

```rust
fn parse_expr(source: &str) -> Expr { /* ... */ }
fn tokenize_single(source: &str) -> Token { /* ... */ }
fn setup_parser_with_tokens(tokens: &[Token]) -> ParserState { /* ... */ }
```

### テスト構造規範 (Arrange-Act-Assert)

**規則 4.1**：各テスト関数は三段構成に従う必要があります：準備（Arrange）→ 実行（Act）→ アサーション（Assert）、三段の間は空行 区切り。

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

**規則 4.2**：単純なテスト（単一呼び出し + 単一アサーション）はセクションコメントを省略できますが、5行以上のロジックコードは超えてはいけません。5行を超えるテストは三段を明示的に標示する必要があります。

### ヘルパー関数規範

**規則 5.1**：3回以上繰り返し登場する setup ロジックはヘルパー関数に抽出する必要があります。

```rust
// 🟢 良い——共通 setup を抽出
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

**規則 5.2**：ヘルパー関数の `unwrap()` / `expect()` は panic 時により多くのコンテキストを出力する必要があります。テスト関数本体 (`#[test] fn ...`) は直接 `unwrap()` できます——失敗時 Rust は自動的に行番号を出力；但しヘルパー関数内失敗時、行番号はヘルパー関数定義箇所を指し、呼び出し時のコンテキストが見えない。

```rust
// 🟢 良い——ヘルパー関数失敗時ソースコード内容を出力
fn run_ok(source: &str) {
    run(source).unwrap_or_else(|e| panic!("Execution failed:\nSource:\n{}\nError:\n{:?}", source, e));
}

// 🔴 悪い——失敗時どのソースファイルが問題を起こしたかわからない
fn run_ok(source: &str) {
    run(source).unwrap();
}
```

**規則 5.3**：ヘルパー関数はテストファイル先頭、`use` インポートの直後に 配置する必要があります。複数のテストモジュールで共有される場合、`tests/mod.rs` に 配置し `pub(crate)` でエクスポートします。

### アサーションスタイル

**規則 6.1**：列挙型 variant マッチングは `assert!(matches!(...))` を優先使用し、`if let` + `panic!` は使用禁止。

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

**規則 6.2**：精密値比較は `assert_eq!` を使用し、ブールアサーションは `assert!` を使用します。`assert!(a == b)` を `assert_eq!(a, b)` の代わりに使用することは禁止です。

**規則 6.3**：すべてのアサーションにはカスタムエラーメッセージが必要です。アサーション自体が既に失敗理由を完全に説明している場合は例外。

```rust
// 🟢 良い——アサーション失敗時素早く特定できる
assert!(
    state.infix_info().is_some(),
    "infix_info should handle '{op}'"
);

// 🟢 良い——assert_eq! 失敗時自動的に値差を出力、追加メッセージ不要
assert_eq!(error_count, 0);

// 🔴 悪い——失敗時 "assertion failed" しかわからない
assert!(state.infix_info().is_some());
```

**規則 6.4**：アサーション順序は `assert_eq!(actual, expected)`、実際値が前で期待値が後。

### アンチパターンの一覧

以下是禁止の写法及其替代方案：

| アンチパターン                               | 問題                               | 代替方案                                                             |
| -------------------------------------------- | ---------------------------------- | -------------------------------------------------------------------- |
| `#[cfg(test)] mod tests { ... }` inline テスト | ソースファイル肥大化、モジュール境界曖昧化、リファクタリング困難 | テストコードを独立の `tests/` ディレクトリに 配置し、`mod tests;` で宣言（規則 1.4 参照） |
| テストがコードの誤動作に我慢する                       | 規範偏差を覆い隠し、バグを合法化        | 規範に照らしてコードを修正し、テストはそのまま保持                                       |
| コード出力を逆手に取ってテスト期待値を決定                   | テストが"現在の実装の録音機"になる         | 規範から期待値を導く                                                   |
| `#[ignore]` 永続マーク                         | 腐ったテストを隠す                     | 修復または削除                                                           |
| `println!` デバッグ出力                          | テスト出力を汚染                       | `assert!` を使用して明確なアサーション                                              |
| `thread::sleep`                              | ランダム失敗 + 遅い                      | 同期メカニズムまたは mock を使用                                               |
| テストで реальный ファイルシステムを操作                     | 遅く且つ繰り返せない                       | `tempfile` を使用                                                       |
| テスト実行順序に依存                             | ランダム失敗                           | 各テストは独立 setup                                                     |
| 1つのテスト関数が30行以上のロジック                   | 誰も理解できない                         | テストを分割またはヘルパー関数を使用                                             |
| ヘルパー関数の `unwrap()` がコンテキストを報告しない           | 特定困難                               | `expect("why")` またはカスタム panic を使用（規則 5.2 参照）                  |
| copy-paste で3回以上同じ setup                | 修正コストが高い                         | ヘルパー関数を抽出                                                         |

---

## 統合テスト規範

### テスト組織

**規則 7.1**：統合テストはプロジェクトルートディレクトリの `tests/` ディレクトリに 配置します。入口ファイル `tests/integration.rs` は `#[path]` 属性を使用してサブモジュールを導入します。

```rust
// tests/integration.rs
#[path = "integration/backends.rs"]
mod backends;
#[path = "integration/codegen.rs"]
mod codegen;
#[path = "integration/execution.rs"]
mod execution;
```

**規則 7.2**：各 `tests/integration/*.rs` ファイルは1つのテストテーマ（コンパイラバックエンド、コード生成、エグゼキュータなど）に対応し、混在 配置は禁止です。

**規則 7.3**：統合テストはプロジェクトのパブリック API を通じてテストする必要があります。統合テストで `crate::` 内部モジュールを 直接参照することは禁止です。`yaoxiang::` パブリックパスを使用します。

```rust
// 🟢 良い——パブリック API を通じた
use yaoxiang::run;

// 🔴 悪い——パブリック API 境界をバイパス
use yaoxiang::middle::codegen::bytecode::BytecodeFile;
```

### テストデータ管理

**規則 8.1**：統合テストはインラインソースコード文字列を優先使用します。ソースコードが30行を超える場合のみ、外部 fixture ファイル（`tests/fixtures/` に 配置）を使用します。

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

**規則 8.2**：fixture ファイルは `.yx` 拡張子で終わり、ファイル名はテスト意図を記述します。

### E2E カバー原則

**規則 9.1**：各言語機能の統合テストは3つのパスをカバーする必要があります：

| パス         | 説明                                   |
| ------------ | -------------------------------------- |
| Happy path   | 正しい入力が予想出力を生成                   |
| Error path   | 無効な入力が明確なエラーメッセージを生成（非 panic） |
| Boundary     | 境界値（空入力、最大値、ネスト深さ上限）        |

**規則 9.2**：統合テストはネットワーク、システム環境変数、外部サービスに依存してはなりません。

---

## ベンチマークテスト規範

### Criterion.rs 使用規範

**規則 10.1**：ベンチマークテストは統一して `benches/` ディレクトリに 配置し、入口ファイルは `benches/lib.rs` です。テストテーマ別にファイルを分割します。

```
benches/
├── lib.rs              # 入口、criterion_group/criterion_main を定義
├── lang_compare/
│   └── fibonacci.rs    # 跨言語比較ベンチマーク
├── parser.rs           # パーサーベンチマーク
└── codegen.rs          # コード生成ベンチマーク
```

**規則 10.2**：各ベンチマーク関数にはテスト目的と測定指標を説明するモジュールドキュメントコメント `//!` が 必须。

```rust
//! YaoXiang インタープリタパフォーマンスベンチマークテスト
//!
//! 測定指標：単一イテレーション所要時間（wall time）
//! ベンチマーク線：Rust ネイティブ実装
```

### コンパイラ最適化防止

**規則 11.1**：すべてのベンチマークテスト的被テスト出力は `criterion::black_box` を使用してコンパイラ最適化消除を防ぎます。

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

**規則 11.2**：ベンチマークテストの入力データは `const` または `lazy_static` でなければならず、`iter` クロージャ内で動的に生成してはなりません——さもないと測定するのはデータ生成 + 被テストロジックの合計時間になります。

### ベンチマークグループ分けと命名

**規則 12.1**：ベンチマークテスト名の形式は `<被テストモジュール>_<シナリオ>`、全小文字アンダースコア 区切り。ユニットテスト命名規則と一致。

**規則 12.2**：関連するベンチマークは `criterion_group!` を使用してロジックグループ化が必要です。すべてのベンチマークを1つのグループに集中させることは禁止です。

```rust
criterion_group!(parser, bench_parse_expr, bench_parse_stmt);
criterion_group!(codegen, bench_codegen_module, bench_codegen_switch);
criterion_main!(parser, codegen);
```

---

## ドキュメンテーションテスト規範

### 使用シナリオ

**規則 13.1**：すべての `pub` 関数、型、メソッドには少なくとも1つの実行可能なコード例を含むドキュメントコメントが必要です。その例は `cargo test --doc` で実行されます。

````rust
/// ソースコード文字列を Token シーケンスにトークナイズする。
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

**規則 13.2**：ドキュメンテーションテストのコード例はコンパイル成功とアサーション成功が必要です。`ignore` マークを含む例は禁止です。ただしその例がコンパイル時エラーを展示する場合は例外。

````rust
/// ```ignore
/// // コンパイル時エラーを展示——ignore  допустимо
/// let x: int = "string";
/// ```
````

### カバー要件

**規則 14.1**：ドキュメンテーションテストは API の happy path をカバーすれば十分です。境界情况和錯誤パスはユニットテストでカバーします。

**規則 14.2**：ドキュメンテーションテスト内のサンプルコードは簡潔でなければならず——10行を超えてはいけません。サンプルにより長いコンテキストが必要な場合、API 設計に問題があることを意味します。

---

## プロパティテスト規範

### 使用シナリオ

**規則 15.1**：以下のシナリオでは、手書きの複数の境界値ケースではなく、プロパティテスト（proptest または quickcheck）を使用する必要があります：

| シナリオ                 | 例                                   |
| -------------------- | -------------------------------------- |
| パーサー round-trip    | `parse(pretty_print(ast)) == ast`      |
| シリアライズ/デシリアライズ      | `deserialize(serialize(data)) == data` |
| 数学演算恒等式       | `a + b == b + a`                       |
| コンパイラ最適化は意味を変えない | `eval(code) == eval(optimize(code))`  |

**規則 15.2**：プロパティテストは `proptest` を主要なプロパティテストフレームワークとして使用（`Cargo.toml` の `dev-dependencies` で宣言済み）。

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

**規則 16.1**：各プロパティテストには明確なプロパティ宣言が必要です——コメントに検証する不変量を明記します。

```rust
// プロパティ：任意の整数を tokenize → tokens_to_string した後、同じ値が生成される
proptest! {
    #[test]
    fn test_int_literal_roundtrip(n in any::<i64>()) {
        let source = n.to_string();
        let tokens = tokenize(&source).unwrap();
        // ...
    }
}
```

**規則 16.2**：プロパティテストが失敗を発見した場合、`proptest` のリグレッションメカニ즘を使用する必要があります——失敗した入力を `proptest-regressions/` ディレクトリに追加し、手動で通常のテストを作成して代之ことは禁止です。

---

## カバレッジ要件

### 新規コードカバレッジ目標

**規則 17.1**：新規コードのテストカバレッジ要件：

| コード種別                                   | 行カバレッジ | 分岐カバレッジ |
| ------------------------------------------ | -------- | ---------- |
| コアコンパイラモジュール（frontend/middle/backends） | ≥ 85%    | ≥ 80%      |
| ツール/ヘルパーモジュール（util）                      | ≥ 75%    | ≥ 70%      |
| ランタイムモジュール（vm/runtime）                   | ≥ 80%    | ≥ 75%      |
| 標準ライブラリ（std）                              | ≥ 75%    | ≥ 70%      |
| 錯誤処理と診断                             | ≥ 90%    | ≥ 85%      |

**規則 17.2**：錯誤処理パス（すべての `Err` 分岐）は100%カバーが必要です。ユーザーが見るエラーメッセージはテストで検証済みでなければなりません。

### PR レビューチェックリスト

**規則 18.1**：PR 提出前、著者は以下の項目を自查する必要があります：

- [ ] `cargo test` 全部通過
- [ ] `cargo test --doc` 全部通過
- [ ] `cargo bench` パフォーマンスリグレッションなし（ホットパス変更涉及の場合）
- [ ] 新規コードはカバレッジ目標に適合
- [ ] テスト命名は命名規範に適合
- [ ] 各テストファイルは対応する規範セクションを宣言（規則 2.1）
- [ ] テスト期待値は規範定義から来ており、"現在のコードの出力"からではない
- [ ] `#[ignore]` マークのテストなし（明確な issue 番号コメントがある場合を除く）
- [ ] 不必要な `unwrap()` なし（`expect` またはカスタム panic メッセージを使用すべき）
- [ ] 提交メッセージは `:white_check_mark: test:` タイプを使用
- [ ] **"コード動作が規範不符"という理由でテスト期待値を修正していない——修正したのはコードであり、テストではない**
- [ ] **inline テストなし**（`#[cfg(test)] mod tests { ... }` は `mod tests;` + 独立ファイルに変更、規則 1.4 参照）

**規則 18.2**：レビュアーは以下の問題を含む PR を拒否する必要があります：

- happy path テストのみで、錯誤パス欠如
- テストに `thread::sleep` または実行順序依存
- コピー＆ペーストのテストコードが3回以上繰り返され、ヘルパー関数を抽出していない
- テスト名が命名規範不符
- 永続 `#[ignore]` のテストが存在
- **テストがコードの誤動作に我慢している**（コードと規範不符時、テストではなくコードを修正）
- **テストが対応する規範セクションを宣言していない**（規則 2.1 参照）
- **テスト期待値がコード出力から来ており、規範定義から来ていない**（逆手に取ったテストはテストにならない）
- **inline テストが存在**（`#[cfg(test)] mod tests { ... }` 而不是 `mod tests;` + 独立ファイル、規則 1.4 参照）
- テストが"panic しない"みを検証し、具体的な動作をアサートしていない
- コードバグを露呈した失敗テストを削除（コード修正後に緑看到它变绿ではなく）

---

## 付録

### A. テストコマンド早見表

```bash
# 全テスト実行
cargo test

# ユニットテストのみ実行
cargo test --lib

# 統合テストのみ実行
cargo test --test integration

# ドキュメンテーションテストのみ実行
cargo test --doc

# 特定テスト実行（名前でフィルタ）
cargo test test_parse_expr

# ベンチマークテスト実行
cargo bench

# テスト出力表示（デフォルトで stdout は非表示）
cargo test -- --nocapture

# 単一スレッド実行（同時実行問題排查）
cargo test -- --test-threads=1

# カバレッジレポート生成（cargo-llvm-cov が必要）
cargo llvm-cov --html
```

### B. 提交メッセージテンプレート

テスト関連提交は以下のテンプレートに従う必要があります：

```
:white_check_mark: test(<scope>): <簡単な説明>

<オプション：カバーするシナリオリスト>
```

例：

```
:white_check_mark: test(parser): Pratt パーサーの中置演算子テストを追加

カバーするシナリオ：
- 算術演算子優先度（+, -, *, /, %）
- 比較演算子リンク（1 < x < 10）
- 論理演算子ショートサーキット
- 代入演算子右結合
```

### C. 新規テストファイルチェックリスト

新しいテストモジュールを作成する際、以下のファイルを含める必要があります：

```
# src/<module>/ ディレクトリに新規テスト追加
src/<module>/tests/
├── mod.rs          # モジュール宣言 + 公共ヘルパー関数
└── <subject>.rs    # テストファイル、被テストソースファイル名に対応

# tests/ ディレクトリに新規統合テスト追加
tests/
├── integration.rs   # 更新：#[path] 宣言を追加
└── integration/
    └── <topic>.rs   # 新規テストファイル
```

### D. 参考資料

- [YaoXiang 言語規範](../reference/language-spec/index.md) —— **テストの権威あるソース**
- [採用された RFC](../design/rfc/index.md) —— **設計決定の権威あるソース**
- [Rust テスト文書](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Criterion.rs ユーザーガイド](https://bheisler.github.io/criterion.rs/book/)
- [proptest 文書](https://docs.rs/proptest/latest/proptest/)
- [プロジェクト提交規範](./commit-convention.md)
- [プロジェクト貢献ガイド](./contributing.md)

---

> 💡
> **覚えておいてください**：テストはコードが"動くか"を検証するのではなく、コードが規範に従っているかを検証します。規範が変われば、テストも規範に従って変わります。コードが間違っていれば、コードを修正し、テストを修正しないでください。**コードは規範に奉仕し、テストは規範を守ります。テストがコードに我慢那一刻，你就失去了所有保护。**
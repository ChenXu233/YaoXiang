---
title: 'RFC 013: エラーコード仕様'
status: '承認済み'
author: '晨煦'
created: '2026-02-02'
updated: '2026-09-03'
issue: '#125'
issues_impl:
  - '#125'
pr_impl:
  - '#7'
  - '#9'
  - '#29'
  - '#66'
---

# RFC 013: エラーコード仕様

## 概要

本 RFC は YaoXiang コンパイラのエラーコード分類仕様を提案する。Rust ライクな単層番号システムを採用し、JSON リソースファイルで多言語サポートを実現し、`yaoxiang explain`
コマンドでエラー解説機能を提供する。

## 動機

### なぜ標準化されたエラーコードが必要なのか？

1. **ユーザエクスペリエンス**：エラーコードを見ることで、ユーザーはエラーの種類や重大度をすばやく判断できる
2. **ドキュメント構成**：カテゴリ別でグループ化することで、エラーリファレンスドキュメントの作成と保守が容易になる
3. **ツール連携**：IDE/LSP がエラーコードに基づいてクイックフィックス提案やドキュメントリンクを提供できる
4. **国際化対応**：エラーメッセージとコードを分離することで、多言語翻訳が容易になる

### 設計目標

- **簡潔**：単層番号方式で、ユーザーは複雑な分類ルールを覚える必要がない
- **親しみやすい**：Rust ライクなエラーメッセージ形式、ヘルプ情報とサンプル付き
- **拡張可能**：リソースファイル駆動で、新しいエラーや新しい言語の追加が容易
- **ツールフレンドリー**：explain コマンド + JSON 出力で IDE/LSP 連携をサポート

---

## 提案

### 中核設計：単層番号システム

4 桁の数字番号を採用し、コンパイル段階でグループ化する：

```
Exxxx
││││
│││└── 連番 (000-999)
││└─── コンパイル段階 (0-9)
└───── 固定プレフィックス 'E'
```

### 段階の分類

| 段階  | 範囲  | 説明                   |
| ----- | ----- | ---------------------- |
| **0** | E0xxx | 語彙・構文解析         |
| **1** | E1xxx | 型検査                 |
| **2** | E2xxx | 意味解析               |
| **3** | E3xxx | コード生成             |
| **4** | E4xxx | ジェネリクスとトレイト |
| **5** | E5xxx | モジュールとインポート |
| **6** | E6xxx | ランタイムエラー       |
| **7** | E7xxx | I/O とシステムエラー   |
| **8** | E8xxx | 内部コンパイラエラー   |
| **9** | E9xxx | 予約/実験的            |

### エラーカテゴリ列挙

```rust
/// エラーカテゴリ
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    Lexer,      // E0xxx: 語彙と構文解析
    Parser,     // E0xxx: パーサエラー
    TypeCheck,  // E1xxx: 型検査
    Semantic,   // E2xxx: 意味解析
    Generic,    // E4xxx: ジェネリクスとトレイト
    Module,     // E5xxx: モジュールとインポート
    Runtime,    // E6xxx: ランタイムエラー
    Io,         // E7xxx: I/O とシステムエラー
    Internal,   // E8xxx: 内部コンパイラエラー
}
```

### エラーコード定義と汎用 Builder

**中核原則**：エラーコード定義と表示テキストを分離する

- `ErrorCodeDefinition`：エラーコードのメタデータ（code、category、template）、表示テキストは含まない
- `locales/*.json`：各言語の表示テキスト（title、message、help、エラーコードはネストオブジェクト）
- `DiagnosticBuilder`：汎用ビルダー、trait-per-error 設計に代わる

#### エラーコード定義

```rust
// diagnostic/codes/mod.rs

use crate::util::span::Span;
use crate::util::diagnostic::{Diagnostic, Severity};

/// エラーコード定義（メタデータのみ、表示テキストは i18n ファイルに）
#[derive(Debug, Clone, Copy)]
pub struct ErrorCodeDefinition {
    pub code: &'static str,
    pub category: ErrorCategory,
    pub message_template: &'static str,  // メッセージテンプレート、{param} プレースホルダ対応
}

/// 汎用診断ビルダー
pub struct DiagnosticBuilder {
    code: &'static str,
    message_template: &'static str,
    params: Vec<(&'static str, String)>,
    span: Option<Span>,
}

impl DiagnosticBuilder {
    pub fn new(code: &'static str, template: &'static str) -> Self {
        Self {
            code,
            message_template: template,
            params: Vec::new(),
            span: None,
        }
    }

    /// テンプレート引数を追加
    pub fn param(mut self, key: &'static str, value: impl Into<String>) -> Self {
        self.params.push((key, value.into()));
        self
    }

    /// 位置を設定
    pub fn at(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    /// Diagnostic を構築（テンプレートレンダリングはコンパイル時に完了）
    pub fn build(&self, i18n: &I18nRegistry) -> Diagnostic {
        // テンプレート内のすべての {key} に対応する引数があるか確認
        self.validate_params();

        let message = i18n.render(self.message_template, &self.params);
        let help = self.help(i18n);

        Diagnostic {
            severity: Severity::Error,
            code: self.code.to_string(),
            message,
            help,
            span: self.span,
            related: Vec::new(),
        }
    }
}
```

#### 各エラーコードのショートカットメソッド

```rust
// diagnostic/codes/e1xxx.rs

impl ErrorCodeDefinition {
    /// E1001 未知の変数
    pub fn unknown_variable(name: &str) -> DiagnosticBuilder {
        let def = Self::find("E1001").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template)
            .param("name", name)
    }

    /// E1002 型の不一致
    pub fn type_mismatch(expected: &str, found: &str) -> DiagnosticBuilder {
        let def = Self::find("E1002").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template)
            .param("expected", expected)
            .param("found", found)
    }
}
```

#### 使用例

```rust
// checking/mod.rs

use crate::util::diagnostic::codes::{ErrorCodeDefinition, E1001};

// 簡略方式
return Err(E1001::unknown_variable(&var_name)
    .at(span)
    .build(&i18n_registry));

// 手動方式
return Err(ErrorCodeDefinition::find("E1001")
    .builder()
    .param("name", var_name)
    .at(span)
    .build(&i18n_registry));
```

#### エラーコード定義例

```rust
// diagnostic/codes/e1xxx.rs

pub static E1XXX: &[ErrorCodeDefinition] = &[
    ErrorCodeDefinition {
        code: "E1001",
        category: ErrorCategory::TypeCheck,
        message_template: "Unknown variable: '{name}'",
    },
    ErrorCodeDefinition {
        code: "E1002",
        category: ErrorCategory::TypeCheck,
        message_template: "Expected type '{expected}', found type '{found}'",
    },
    // ... その他のエラーコード
];
```

#### 設計上の利点

| 特性                             | 説明                                                     |
| -------------------------------- | -------------------------------------------------------- |
| **単一 Builder**                 | 1 つの `DiagnosticBuilder` ですべてのエラーコードに対応  |
| **型安全**                       | ショートカットメソッドが引数の正確性を保証               |
| **自己文書化**                   | `E1001::unknown_variable(name)` が一目でわかる           |
| **テンプレート分離**             | メッセージテンプレートとコードが分離され、i18n が容易    |
| **ランタイムオーバーヘッドゼロ** | コンパイル時レンダリング、AOT バイナリにテーブル参照なし |

---

### エラーマクロの簡素化

#### error! マクロ（コンテキスト自動注入）

```rust
/// コンパイル時に span と i18n 設定を自動取得するマクロ
macro_rules! error {
    ($code:ident, $($key:ident = $value:expr),* $(,)?) => {
        $code()
            $(.$key($value))*
            .at(crate::util::span::Span::current())
            .build(crate::util::diagnostic::I18nRegistry::current())
    };
}

/// 使用法：引数のみを渡せば span と i18n は自動注入される
return Err(error!(E1001, name = var_name));
return Err(error!(E1002, expected = "bool", found = cond_ty));
```

#### Builder の手動使用

```rust
// 手動制御が必要な場合
E1001::unknown_variable(&var_name)
    .at(my_span)           // カスタム span
    .build(&custom_i18n)   // カスタム i18n
```

---

## 詳細設計

### エラーコード一覧

#### E0xxx：語彙・構文解析

| コード | エラー種別                | 説明                               |
| ------ | ------------------------- | ---------------------------------- |
| E0001  | Invalid character         | ソースコードに不正な文字が含まれる |
| E0002  | Invalid number literal    | 数値リテラルの形式が正しくない     |
| E0003  | Unterminated string       | 複数行文字列の終了引用符が欠落     |
| E0004  | Invalid character literal | 文字リテラルが正しくない           |
| E0010  | Expected token            | 構文解析時に特定のトークンが必要   |
| E0011  | Unexpected token          | 予期しないトークンに遭遇           |
| E0012  | Invalid syntax            | 式/文の構文エラー                  |
| E0013  | Mismatched brackets       | 丸括弧、角括弧、波括弧の不一致     |
| E0014  | Missing semicolon         | 文末にセミコロンが欠落             |
| E0016  | Expected expression       | 式が必要                           |
| E0018  | Keyword as name           | キーワードを名前として使用できない |

#### E1xxx：型検査

| コード | エラー種別                                             | 説明                                              |
| ------ | ------------------------------------------------------ | ------------------------------------------------- |
| E1001  | Unknown variable                                       | 参照された変数が未定義                            |
| E1002  | Type mismatch                                          | 期待型と実際の型が一致しない                      |
| E1003  | Unknown type                                           | 参照された型が存在しない                          |
| E1010  | Parameter count mismatch                               | 関数呼び出しの引数数が定義と一致しない            |
| E1011  | Parameter type mismatch                                | 引数の型検査に失敗                                |
| E1012  | Return type mismatch                                   | 関数の戻り値の型が誤り                            |
| E1013  | Function not found                                     | 未定義の関数を呼び出し                            |
| E1020  | Cannot infer type                                      | コンテキストから型を推論できない                  |
| E1021  | Type inference conflict                                | 複数の制約により型が矛盾                          |
| E1030  | Pattern non-exhaustive                                 | match 式がすべてのケースを網羅していない          |
| E1031  | Unreachable pattern                                    | 決してマッチしないパターン                        |
| E1040  | Operation not supported                                | 型がその操作をサポートしていない                  |
| E1041  | Index out of bounds                                    | 配列/リストのインデックスが範囲外                 |
| E1042  | Field not found                                        | 存在しない構造体フィールドにアクセス              |
| E1050  | Boolean operand required                               | ブールオペランドが必要                            |
| E1051  | Logical NOT requires boolean operand                   | 論理 NOT にはブールオペランドが必要               |
| E1052  | Invalid dereference                                    | 不正な逆参照                                      |
| E1053  | Non-struct field access                                | 非構造体のフィールドアクセス                      |
| E1054  | Conditional type mismatch                              | 条件型の不一致                                    |
| E1055  | Constraint in non-generic context                      | 制約が非ジェネリックコンテキストに出現            |
| E1060  | Type parameter count mismatch                          | 型パラメータ数が一致しない                        |
| E1061  | Cannot instantiate generic                             | ジェネリクスをインスタンス化できない              |
| E1062  | Const generic constraint failed                        | const ジェネリック制約の失敗                      |
| E1064  | Invalid binding position                               | バインディング位置のインデックスが無効（RFC-004） |
| E1071  | Type definitions are only allowed at module level      | 型定義はモジュールレベルでのみ許可                |
| E1081  | `?` can only be used within functions returning Result | `?` は Result を返す関数内でのみ使用可能          |
| E1082  | `?` can only be used with Result expressions           | `?` は Result 式にのみ使用可能                    |
| E1083  | Error type mismatch for `?`                            | `?` のエラー型が一致しない                        |
| E1090  | Type universe easter egg                               | Type: Type = Type イースターエッグ（Note レベル） |
| E1091  | Invalid generic meta type                              | 不正なジェネリックメタ型                          |
| E1092  | Invalid refinement type argument form                  | 精化型の実引数形式が不正                          |
| E1093  | Refinement argument count mismatch                     | 精化実引数の個数が一致しない                      |
| E1094  | Unused compile-time value parameter                    | 未使用のコンパイル時値パラメータ                  |
| E1095  | Unknown interface                                      | 未知のインターフェース                            |
| E1096  | Interface arity mismatch                               | インターフェースのパラメータ数が不一致            |
| E1097  | Interface member name conflict                         | インターフェースメンバーの名前衝突                |
| E1098  | Interface method not implemented                       | インターフェースメソッドが未実装                  |
| E1099  | Interface method signature mismatch                    | インターフェースメソッドのシグネチャ不一致        |
| E1100  | Duplicate interface method implementation              | インターフェースメソッドの重複実装                |
| E1101  | Type does not implement interface                      | 型がインターフェースを実装していない              |
| E1102  | Loop control statement outside of a loop               | ループ制御文がループ外に出現                      |

#### E2xxx：意味解析

| コード | エラー種別                        | 説明                                   |
| ------ | --------------------------------- | -------------------------------------- |
| E2001  | Scope error                       | 変数が現在のスコープにない             |
| E2002  | Duplicate definition              | 同一スコープ内での重複定義             |
| E2003  | Lifetime error                    | ライフタイム制約が満たされない         |
| E2010  | Immutable assignment              | 不変変数の変更を試みた                 |
| E2011  | Uninitialized use                 | 未初期化変数の使用                     |
| E2012  | Mutability conflict               | 不変コンテキストでの可変参照の使用     |
| E2013  | Variable shadowing                | 変数のシャドーイング                   |
| E2014  | Use of moved value                | ムーブ済み値の使用                     |
| E2016  | Immutable assignment              | 不変代入                               |
| E2018  | Mutable/immutable borrow conflict | 可変/不変の借用競合                    |
| E2019  | Double free                       | 二重解放                               |
| E2020  | Use after free                    | 解放後の使用                           |
| E2027  | Unsafe dereference                | unsafe な逆参照                        |
| E2090  | Invalid signature                 | 関数シグネチャの解析エラー             |
| E2091  | Unknown type in signature         | シグネチャに未知の型が出現             |
| E2092  | Missing arrow in signature        | シグネチャに戻り矢印がない             |
| E2093  | Duplicate parameter name          | 重複するパラメータ名                   |
| E2094  | Generic parameter shadowing       | ジェネリックパラメータのシャドーイング |
| E2095  | Parameter name shadows generic    | パラメータ名がジェネリクスをシャドー   |

#### E3xxx：コード生成

| コード | エラー種別                             | 説明                                                       |
| ------ | -------------------------------------- | ---------------------------------------------------------- |
| E3004  | Unsupported iterator                   | サポートされていないイテレータ                             |
| E3005  | IR generation error                    | IR 生成の内部エラー                                        |
| E3006  | Unresolved variable                    | IR 生成段階で変数が未解決                                  |
| E3007  | Top-level initializer must be constant | トップレベルバインディングの初期化は定数でなければならない |
| E3014  | Register overflow                      | レジスタオーバーフロー                                     |
| E3017  | Invalid operand (code generation)      | 不正なオペランド（コード生成）                             |

#### E4xxx：ジェネリクスとトレイト

| コード | エラー種別                              | 説明                                      |
| ------ | --------------------------------------- | ----------------------------------------- |
| E4001  | Generic parameter mismatch              | ジェネリックパラメータの数/型が一致しない |
| E4002  | Trait bound violated                    | トレイト制約が満たされない                |
| E4003  | Associated type error                   | 関連型の定義/使用エラー                   |
| E4004  | Duplicate trait implementation          | 同一トレイトの重複実装                    |
| E4005  | Trait not found                         | 要求されたトレイトが見つからない          |
| E4006  | Sized bound violated                    | Sized 制約が満たされない（予約、未実装）  |
| E4010  | Division by zero in constant expression | 定数式でのゼロ除算                        |
| E4011  | Constant overflow                       | 定数オーバーフロー                        |
| E4012  | Constant recursion too deep             | 定数再帰が深すぎる                        |
| E4014  | Constant evaluation failed              | 定数評価失敗                              |
| E4018  | Refinement predicate violation          | 精化述語違反                              |
| E4019  | Type equality does not hold             | 型の等式が成立しない                      |

#### E5xxx：モジュールとインポート

| コード | エラー種別            | 説明                                         |
| ------ | --------------------- | -------------------------------------------- |
| E5001  | Module not found      | インポートされたモジュールが存在しない       |
| E5002  | Cyclic import         | モジュール間の循環依存                       |
| E5003  | Symbol not exported   | 未エクスポートのシンボルにアクセス           |
| E5004  | Invalid module path   | モジュールパスの形式エラー                   |
| E5005  | Private access        | プライベートシンボルへのアクセス             |
| E5006  | Duplicate import      | 重複インポート                               |
| E5007  | Module export listing | モジュールエクスポート一覧（付随ヒント情報） |

#### E6xxx：ランタイムエラー

| コード | エラー種別                  | 説明                                        |
| ------ | --------------------------- | ------------------------------------------- |
| E6001  | Division by zero            | 整数のゼロ除算                              |
| E6002  | ~~Assertion failed~~        | ~~予約（言語概念なし、削除済み）~~          |
| E6003  | Runtime index out of bounds | ランタイムのインデックス範囲外（#280 接続） |
| E6004  | Stack overflow              | スタック空間枯渇                            |
| E6005  | Assertion failed            | assert 失敗（#280 接続）                    |
| E6006  | Function not found          | ランタイムで関数が見つからない              |
| E6007  | Runtime error (generic)     | 汎用ランタイムエラー                        |
| E6008  | Key not found               | Dict のキー欠如（#299 §4）                  |

> **#280 改訂（2026-08-09）**：コード表は元々 Rust セマンティクス草案（Assertion failed/Arithmetic
> overflow/Heap allocation failed/Type cast
> failed）に基づいて定義されており、実装の実際のニーズと一致していなかった。YaoXiang には null ポインタ/ヒープ割り当て失敗/型キャストの概念がなく（値セマンティクス +
> Rust メモリ安全性）、ランタイムオーバーフロー経路の検出は実装されていない。校正後：
>
> - E6002 削除（元の Assertion
>   failed は E6005 に移動、元の null ポインタセマンティクスは言語概念なし）
> - E6003 を Arithmetic overflow から Runtime index out of
>   bounds に変更（実際のトリガー面、#279/#271）
> - E6005 を Heap allocation failed から Assertion failed に変更（std.assert の実際のパス）
> - E6006 を Runtime index out of bounds から Function not found に変更（実装は既にこの通り、#255）
> - E6007 を Type cast failed から汎用 Runtime
>   error に変更（ExecutorError の未マッピングバリアントの統一フォールバック）

#### E7xxx：I/O とシステムエラー

| コード | エラー種別        | 説明                               |
| ------ | ----------------- | ---------------------------------- |
| E7001  | File not found    | 存在しないファイルの読み込みを試行 |
| E7002  | Permission denied | ファイル権限不足                   |
| E7003  | I/O error         | 汎用 I/O エラー                    |
| E7004  | Network error     | ネットワーク操作失敗               |

#### E8xxx：内部コンパイラエラー

| コード | エラー種別              | 説明                                   |
| ------ | ----------------------- | -------------------------------------- |
| E8001  | Internal compiler error | コンパイラの内部エラー                 |
| E8002  | Codegen error           | IR/バイトコード生成失敗                |
| E8003  | Unimplemented feature   | 未実装機能の使用                       |
| E8004  | Optimization error      | コンパイラ最適化エラー（予約、未実装） |

#### W1xxx：警告コード

| コード | 警告種別                                     | 説明                                 |
| ------ | -------------------------------------------- | ------------------------------------ |
| W1001  | Unused exported function                     | 未使用のエクスポート関数             |
| W1002  | Unused exported type                         | 未使用のエクスポート型               |
| W1003  | Unused import                                | 未使用のインポート                   |
| W1004  | Unused exported variable                     | 未使用のエクスポート変数             |
| W1005  | Unused exported method                       | 未使用のエクスポートメソッド         |
| W1063  | Const generic constraint cannot be evaluated | const ジェネリック制約を評価できない |

> W コード位置ルール：E コードと同じく段階でグループ化（W+段階千位セグメント）、W1xxx
> = 型検査段階の警告。
>
> **発射チャネル（#321 M2）**：W コード診断は builder が W 接頭辞でデフォルト `Severity::Warning`
> をマークする（明示的指定優先）、収集と表示はエラーと同じ経路（`warning[W####]`
> 接頭辞レンダリング）だが、コンパイルを遮断せず、成功終了コードにも影響しない。`yaoxiang check --deny-warnings`
> は警告を失敗に昇格させる（警告存在時に非ゼロコードで終了）、CI 厳格モード用。per-code 抑制（allow 属性など）は将来の拡張項目。

---

### ランタイムエラー値とコードの連携

> 本節は #323（M4 ランタイム Error 値にコードを付与、2026-09-03）によって導入された。E6xxx/E7xxx セマンティクス空間は 2 つのチャネルを同時に担い、コード空間は同一、提示チャネルは異なる。

#### 2 つのチャネル

| チャネル                     | 媒体                                                  | 提示方式                                                          |
| ---------------------------- | ----------------------------------------------------- | ----------------------------------------------------------------- |
| コンパイラ/CLI 診断チャネル  | `ExecutorError` などのホスト層ハードエラー            | stderr `error[E####]:`（#280/#281 で E6003/E6005/E6007 接続済み） |
| プログラム内エラー値チャネル | std ライブラリ `Result(T, Error)` の Err 媒体 `Error` | 言語値、プログラムが match/比較で消費                             |

#### Error 構造（v0.8 以降、破壊的変更）

```
Error { code: String, message: String }
```

- `code` は本仕様の E6xxx/E7xxx 番号を再利用、文字列形式（例：`"E6008"`）。
- **安定契約**：割り当てられたコードはバージョン間で意味が変わらない；同じ意味に削除済みコードを再利用しない（E6002 が先例）。
- **消費面**：プログラム内の `e.code == "E6xxx"`
  比較が唯一のプログラマブル判定契約；`yaoxiang explain E6xxx`
  ドキュメントで連携；ツールチェーン（LSP /
  DAP、RFC-034 参照）はコードを exceptionId として使用する。
- **アクセサ**：`std.result.code(e)` / `std.result.message(e)`。
- **ユーザー定義エラー**：`Result(T, E)`
  の E はジェネリック引数、真面目にモデリングする場合はユーザー定義型で；std `Error`
  は便利なフォールバック媒体に過ぎず、そのコード体系はユーザー E 型を制約しない。

#### コード割り当てルール

1. ランタイムエラー値コードとコンパイル時診断コードは E6xxx/E7xxx 空間を共有し、新しいコードは**実際のトリガー面**に基づいて割り当てられ、想像上のシナリオのために予約しない。
2. 登録してから使用：新しいコードは権威あるレジストリに登録され、三者間一貫性検証（codes/*.rs ↔
   locales ↔ 本ドキュメントコード表）を経た後にのみ発射できる。
3. E7xxx は std.io / std.net エラー値のために予約されたセグメント（現在空予約、io/net
   Result 化時に有効化）。

#### 進化経路（線 C、未実施）

パターンマッチの完备化（RFC-039）が実装された後、`Error` は `{ kind: ErrorKind, message: String }`
にアップグレードでき、`code`
は kind から派生する属性になる（バリアント定義箇所がコードレジストリ）。進化期間中、本節のコード安定契約は変わらず；このアップグレードは独立した決定であり、本節のコミットメントを構成しない。

---

### 多言語リソースファイル

#### リソースファイル形式

```json
// locales/en.json
{
  "E1001": {
    "title": "Unknown variable",
    "message": "Referenced variable is not defined",
    "template": "Unknown variable: '{name}'",
    "help": "Check if the variable name is spelled correctly, or define it first",
    "example": "x = 100;",
    "error_output": "error[E1001]: Unknown variable: 'x'\n  --> example.yx:1:1\n   |\n 1 | print(x)\n   | ^ unknown variable 'x'"
  },
  "E1002": {
    "title": "Type mismatch",
    "message": "Expected type does not match actual type",
    "template": "Expected type '{expected}', found type '{found}'",
    "help": "Use the correct type or add a type conversion",
    "example": "x: Int = \"hello\";",
    "error_output": "error[E1002]: Type mismatch\n  --> example.yx:1:12\n   |\n 1 | x: Int = \"hello\";\n   |            ^ expected 'Int', found 'String'"
  }
}
```

```json
// locales/zh.json
{
  "E1001": {
    "title": "未知变量",
    "message": "引用的变量未定义",
    "template": "未知变量：'{name}'",
    "help": "检查变量名是否拼写正确，或先定义它",
    "example": "x = 100;",
    "error_output": "error[E1001]: 未知变量：'x'\n  --> example.yx:1:1\n   |\n 1 | print(x)\n   | ^ 未知变量 'x'"
  },
  "E1002": {
    "title": "类型不匹配",
    "message": "期望类型与实际类型不匹配",
    "template": "期望类型 '{expected}'，实际类型 '{found}'",
    "help": "使用正确的类型或添加类型转换",
    "example": "x: Int = \"hello\";",
    "error_output": "error[E1002]: 类型不匹配\n  --> example.yx:1:12\n   |\n 1 | x: Int = \"hello\";\n   |            ^ 期望 'Int'，找到 'String'"
  }
}
```

```json
// locales/ja.json
{
  "E1001": {
    "title": "未知の変数",
    "message": "参照された変数が未定義です",
    "template": "未知の変数：'{name}'",
    "help": "変数名のスペルが正しいか確認するか、先に定義してください",
    "example": "x = 100;",
    "error_output": "error[E1001]: 未知の変数：'x'\n  --> example.yx:1:1\n   |\n 1 | print(x)\n   | ^ 未知の変数 'x'"
  },
  "E1002": {
    "title": "型の不一致",
    "message": "期待された型と実際の型が一致しません",
    "template": "期待された型 '{expected}'、実際の型 '{found}'",
    "help": "正しい型を使用するか、型変換を追加してください",
    "example": "x: Int = \"hello\";",
    "error_output": "error[E1002]: 型の不一致\n  --> example.yx:1:12\n   |\n 1 | x: Int = \"hello\";\n   |            ^ 期待 'Int'、見つかった 'String'"
  }
}
```

#### I18nRegistry 実装

```rust
// locales/*.json（エラーコードオブジェクト）

/// i18n 表示テキストレジストリ（コンパイル時に JSON から読み込み、ランタイムはテーブル参照なし）
pub struct I18nRegistry {
    /// タイトル
    titles: HashMap<&'static str, &'static str>,
    /// 説明
    messages: HashMap<&'static str, &'static str>,
    /// ヘルプ情報
    helps: HashMap<&'static str, &'static str>,
    /// サンプルコード
    examples: HashMap<&'static str, &'static str>,
    /// エラー出力サンプル
    error_outputs: HashMap<&'static str, &'static str>,
}

/// 単一エラーコード情報
#[derive(Clone, Copy)]
pub struct ErrorInfo<'a> {
    pub title: &'a str,
    pub message: &'a str,
    pub help: &'a str,
    pub example: Option<&'a str>,
    pub error_output: Option<&'a str>,
}

impl I18nRegistry {
    /// 言語コードに応じてレジストリを取得
    pub fn new(lang: &str) -> Self {
        match lang {
            "zh" => Self::zh(),
            "ja" => Self::ja(),
            _ => Self::en(),
        }
    }

    /// エラー情報を取得
    pub fn get_info(&self, code: &str) -> Option<ErrorInfo<'_>> {
        Some(ErrorInfo {
            title: self.titles.get(code)?,
            message: self.messages.get(code)?,
            help: self.helps.get(code)?,
            example: self.examples.get(code).copied(),
            error_output: self.error_outputs.get(code).copied(),
        })
    }

    /// テンプレートをレンダリング（コンパイル時に完了、ランタイムオーバーヘッドゼロ）
    pub fn render(&self, template: &'static str, params: &[(&str, String)]) -> String {
        let mut result = String::with_capacity(template.len() + 64);
        let mut chars = template.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '{' {
                let mut key = String::new();
                while let Some(&c) = chars.peek() {
                    if c == '}' {
                        chars.next();
                        if let Some((_, value)) = params.iter().find(|(k, _)| k == &key) {
                            result.push_str(value);
                        } else {
                            result.push_str(&format!("{{{}}}", key));
                        }
                        break;
                    }
                    key.push(c);
                    chars.next();
                }
            } else {
                result.push(c);
            }
        }
        result
    }
}
```

#### テンプレートプレースホルダ

##### 定義済みプレースホルダ（よく使用されるもの）

| プレースホルダ | 用途                               | 例                                  |
| -------------- | ---------------------------------- | ----------------------------------- |
| `{name}`       | 変数名/型名/トレイト名などの識別子 | `Unknown variable: '{name}'`        |
| `{expected}`   | 期待された型                       | `Expected type '{expected}'`        |
| `{found}`      | 実際の/見つかった型                | `, found type '{found}'`            |
| `{method}`     | メソッド名                         | `Method {method} is not a function` |
| `{trait}`      | トレイト名                         | `Cannot find trait: {trait}`        |
| `{path}`       | モジュールパス                     | `Invalid path: {path}`              |
| `{ty}`         | 型式                               | `Invalid type: {ty}`                |
| `{message}`    | 内部エラーメッセージ               | `Internal error: {message}`         |

##### 任意のキーサポート

**params は任意のキーをサポートし、定義済みのみに限定されない**。呼び出し側は任意の `key` を渡せる：

```rust
// 任意のキーを使用
E1001::unknown_variable(&var_name)
    .param("location", "global scope")
    .param("hint", "try declaring it first")
    .at(span)
    .build(&i18n);

// テンプレート定義
"Unknown variable: '{name}' at {location}. {hint}"
```

> **注意**：すべてのエラーコードがプレースホルダを使用するわけではない。一部のエラーコード（例：E0001）は静的メッセージで、引数は不要。

#### 言語優先度

```
1. yaoxiang.toml [language.default]
2. ~/.yaoxiang/yaoxiang.toml [language.default]
3. デフォルト値: en
```

### yaoxiang.toml 設定

#### プロジェクトレベル設定

```toml
# yaoxiang.toml
[project]
name = "my-project"
version = "0.1.0"

[language]
# エラーメッセージの言語、選択肢：en, zh, ja, ...
default = "ja"
```

#### ユーザレベル設定

```toml
# ~/.yaoxiang/yaoxiang.toml
[language]
default = "ja"
```

#### コンパイル時言語選択

```
1. プロジェクトレベル yaoxiang.toml の language.default を読み込み
2. 未設定の場合、ユーザレベル ~/.yaoxiang/yaoxiang.toml を読み込み
3. どちらも未設定の場合、デフォルトで "en" を使用
4. コンパイラは選択された言語に基づいて I18nRegistry を作成（一度だけ）
5. すべてのエラーはこの I18nRegistry を使用してメッセージをレンダリング
```

#### テーブル参照オーバーヘッドゼロの鍵

**レンダリングはユーザプロジェクトのコンパイル時に発生し、ランタイムではない。**

```
┌─────────────────────────────────────────────────────────────────────────┐
│  段階 1: Rust が YaoXiang コンパイラをコンパイル                          │
│                                                                           │
│  JSON がコンパイラバイナリにパッケージ化される                             │
│  目的：explain コマンドが i18n データを直接読み取れるように                 │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  段階 2: YaoXiang がユーザプロジェクトをコンパイル（ここでレンダリング）    │
│                                                                           │
│  error! マクロ呼び出し時：                                                  │
│  1. yaoxiang.toml を読み込んで言語設定を取得                              │
│  2. コンパイラバイナリから対応言語の i18n JSON を読み込み                  │
│  3. テンプレート + 引数 → render() → "Unknown variable: 'x'"             │
│  4. Diagnostic.message = レンダリング済み文字列                            │
│                                                                           │
│  AOT バイナリは最終文字列を直接格納、テンプレートなし、テーブル参照なし    │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  段階 3: ユーザプログラムのランタイム                                      │
│                                                                           │
│  println!("{}", diagnostic.message)                                       │
│  // 最終文字列を直接出力、テーブル参照なし                                 │
└─────────────────────────────────────────────────────────────────────────┘
```

| コンポーネント               | 責務                             | レンダリングタイミング         |
| ---------------------------- | -------------------------------- | ------------------------------ |
| `I18nRegistry`               | テンプレートと表示テキストを提供 | ユーザプロジェクトコンパイル時 |
| `DiagnosticBuilder.render()` | テンプレート + 引数 → 最終文字列 | ユーザプロジェクトコンパイル時 |
| `Diagnostic.message`         | レンダリング済み文字列           | 最終結果を格納                 |
| AOT バイナリ                 | 最終文字列を含む                 | ランタイムで直接使用           |

---

### エラーメッセージ形式

エラーメッセージは以下の形式を採用する：

```
error[E####]: <簡潔な説明>
  --> <ファイル>:<行>:<列>
   <行> | <コード片>
          ^^^<ハイライト>
```

#### 完全な例

```
error[E1001]: Unknown variable: x
  --> src/main.yx:5:12
   5 |   print(x)
          ^
          help: Did you mean to define it?
```

---

### 重大度レベル

エラーの重大度は `DiagnosticLevel` 列挙で管理され、エラーコード番号から分離される：

```rust
pub enum DiagnosticLevel {
    Error,    // コンパイル失敗を引き起こす
    Warning,  // コンパイルには影響しないが、修正を推奨
    Note,     // 補足情報
    Help,     // 修正提案
}
```

| レベル  | 接頭辞            | 説明                       |
| ------- | ----------------- | -------------------------- |
| Error   | `error[E####]:`   | コンパイル失敗を引き起こす |
| Warning | `warning[E####]:` | コンパイルには影響しない   |
| Note    | `note[E####]:`    | 補足情報                   |
| Help    | `help[E####]:`    | 修正提案                   |

---

### `yaoxiang explain` コマンド

#### コマンド構文

```bash
yaoxiang explain <ERROR_CODE> [OPTIONS]
```

#### オプション

| オプション      | 説明                                             |
| --------------- | ------------------------------------------------ |
| `--lang <code>` | 言語指定 (en-US, zh-CN, ja-JP、デフォルト en-US) |
| `--json`        | JSON 形式出力（IDE/LSP 用）                      |
| `--json-pretty` | 整形済み JSON 出力                               |
| `--examples`    | サンプルコードのみ表示                           |
| `--help`        | ヘルプ情報を表示                                 |

#### 使用例

```bash
# デフォルト英語
$ yaoxiang explain E1001
error[E1001]: Unknown variable: {name}
  --> <file>:<line>:<col>

Help: Did you mean to define it?

Example:
  let {name} = value;

# 中国語出力
$ yaoxiang explain E1001 --lang zh
error[E1001]: 未知变量: {name}
  --> <file>:<line>:<col>

帮助: 你是否想要定义它？

示例:
  let {name} = value;

# 日本語出力
$ yaoxiang explain E1001 --lang ja
error[E1001]: 未知の変数: {name}
  --> <file>:<line>:<col>

ヘルプ: 定義することでしたか？

例:
  let {name} = value;

# JSON 出力（LSP 連携）
$ yaoxiang explain E1001 --json
{
  "code": "E1001",
  "message": "Unknown variable: {name}",
  "help": "Did you mean to define it?",
  "examples": ["let {name} = value;"],
  "language": "en-US"
}
```

#### JSON 出力形式

```json
{
  "code": "E1001",
  "message": "Unknown variable: {name}",
  "help": "Did you mean to define it?",
  "examples": ["let {name} = value;"],
  "language": "en-US"
}
```

---

### 後方互換性

本 RFC はエラーコードシステムをゼロから設計するため、後方互換性の問題はない。

**将来の移行戦略**（後続バージョン参考用）：

1. 旧エラーコードから新エラーコードへのマッピングを維持
2. 移行期間中は新旧コードを同時表示
3. 廃止スケジュールの提供

---

## 実装戦略

### フェーズ 1：エラーコード基礎構造

1. `src/diagnostics/` ディレクトリ構造の作成
2. `ErrorCode` 列挙の実装
3. `Diagnostic` と `DiagnosticLevel` の実装
4. リソースファイルディレクトリとサンプル JSON の作成

### フェーズ 2：explain コマンド

1. `yaoxiang explain` CLI コマンドの実装
2. `--lang` と `--json` オプションのサポート
3. リソースファイル読み込みの統合
4. パラメータテンプレートのレンダリング実装

### フェーズ 3：コンパイル時統合

1. すべてのエラー報告ポイントを新システムに更新
2. メッセージテンプレート引数注入の実装
3. 言語優先度ロジックの追加
4. ユニットテストカバレッジ

### フェーズ 4：IDE/LSP 連携

1. LSP サーバーが explain JSON 出力を統合
2. IDE でのエラーコードリンク表示
3. ホバーでエラー解説表示
4. クイックフィックス提案

---

## 付録

### 完全エラーコード早見表

| 範囲  | カテゴリ               |
| ----- | ---------------------- |
| E0xxx | 語彙・構文解析         |
| E1xxx | 型検査                 |
| E2xxx | 意味解析               |
| E3xxx | コード生成             |
| E4xxx | ジェネリクスとトレイト |
| E5xxx | モジュールとインポート |
| E6xxx | ランタイムエラー       |
| E7xxx | I/O とシステムエラー   |
| E8xxx | 内部コンパイラエラー   |
| E9xxx | 予約                   |

### サポート言語

| コード | 言語         | ステータス |
| ------ | ------------ | ---------- |
| en-US  | English (US) | デフォルト |
| zh-CN  | 简体中文     | 計画中     |
| ja-JP  | 日本語       | 計画中     |

### エラーメッセージ例比較

```
# 英語 (en-US)
error[E1001]: Unknown variable: x
  --> src/main.yx:5:12
   5 |   print(x)
          ^
          help: Did you mean to define it?

# 中国語 (zh-CN)
error[E1001]: 未知变量: x
  --> src/main.yx:5:12
   5 |   print(x)
          ^
          帮助: 你是否想要定义它？

# 日本語 (ja-JP)
error[E1001]: 未知の変数: x
  --> src/main.yx:5:12
   5 |   print(x)
          ^
          ヘルプ: 定義することでしたか？
```

## 参考文献

- [Rust コンパイラエラー索引](https://doc.rust-lang.org/error_codes/error-index.html)
- [GCC エラーメッセージ形式](https://gcc.gnu.org/onlinedocs/gcc-13.1.0/gcc/Warning-Options.html)
- [Clang 診断形式](https://clang.llvm.org/diagnostics.html)
- [Language Server Protocol](https://microsoft.github.io/language-server-protocol/)

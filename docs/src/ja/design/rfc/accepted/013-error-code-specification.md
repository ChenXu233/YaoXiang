---
title: 'RFC 013: 錯誤コード規範'
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

# RFC 013: 錯誤コード規範

## 概要

本 RFC は YaoXiang コンパイラの錯誤コード分類規範を提案する。Rust 類似の単層番号システムを採用し、JSON リソースファイルによる多言語サポートを実現し、`yaoxiang explain`
コマンドを通じて錯誤解説機能を提供する。

## 動機

### なぜ標準化された錯誤コードが必要なのか？

1. **ユーザー体験**：ユーザーが錯誤コードを見ることで、錯誤の種類と深刻度を迅速に判断できる
2. **ドキュメント構成**：カテゴリ別にグループ化することで、錯誤リファレンスドキュメントの作成と保守が容易になる
3. **ツール統合**：IDE/LSP が錯誤コードに基づいてクイック修正提案やドキュメントリンクを提供できる
4. **国際化サポート**：エラーメッセージとコードを分離することで、多言語翻訳が容易になる

### 設計目標

- **簡潔**：単層番号方式により、ユーザーは複雑な分類ルールを覚える必要がない
- **親しみやすい**：Rust 類似のエラーメッセージ形式、ヘルプ情報とサンプル付き
- **拡張可能**：リソースファイル駆動で、新しい錯誤や新しい言語の追加が容易
- **ツールフレンドリー**：explain コマンド + JSON 出力で、IDE/LSP 統合をサポート

---

## 提案

### コア設計：単層番号システム

4 桁の数字番号を採用、コンパイル段階でグループ化：

```
Exxxx
││││
│││└── 番号 (000-999)
││└─── コンパイル段階 (0-9)
└───── 固定プレフィックス 'E'
```

### 段階区分

| 段階  | 範囲  | 説明                   |
| ----- | ----- | ---------------------- |
| **0** | E0xxx | 字句解析と構文解析     |
| **1** | E1xxx | 型チェック             |
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
    Lexer,      // E0xxx: 字句解析と構文解析
    Parser,     // E0xxx: Parser errors
    TypeCheck,  // E1xxx: 型チェック
    Semantic,   // E2xxx: 意味解析
    Generic,    // E4xxx: ジェネリクスとトレイト
    Module,     // E5xxx: モジュールとインポート
    Runtime,    // E6xxx: ランタイムエラー
    Io,         // E7xxx: I/Oとシステムエラー
    Internal,   // E8xxx: 内部コンパイラエラー
}
```

### エラーコード定義と汎用 Builder

**基本原則**：エラーコード定義と表示テキストを分離

- `ErrorCodeDefinition`：エラーコードのメタデータ（code、category、template）、表示テキストは含まない
- `locales/*.json`：各言語の表示テキスト（title、message、help、エラーコードはネストされたオブジェクト）
- `DiagnosticBuilder`：汎用ビルダー、trait-per-error 設計に代わるもの

#### エラーコード定義

```rust
// diagnostic/codes/mod.rs

use crate::util::span::Span;
use crate::util::diagnostic::{Diagnostic, Severity};

/// エラーコード定義（メタデータのみ、表示テキストは i18n ファイル）
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

    /// テンプレートパラメータを追加
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
        // テンプレート内のすべての {key} に対応するパラメータがあるか確認
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
| **型安全性**                     | ショートカットメソッドがパラメータの正確性を保証         |
| **自己文書化**                   | `E1001::unknown_variable(name)` で一目瞭然               |
| **テンプレート分離**             | メッセージテンプレートとコードが分離され、i18n が容易    |
| **ランタイムオーバーヘッドゼロ** | コンパイル時レンダリング、AOT バイナリにテーブル参照なし |

---

### エラーマクロによる簡略化

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

/// 使用方法：パラメータを渡すだけで、span と i18n は自動注入
return Err(error!(E1001, name = var_name));
return Err(error!(E1002, expected = "bool", found = cond_ty));
```

#### 手動で Builder を使用

```rust
// 手動制御が必要な場合
E1001::unknown_variable(&var_name)
    .at(my_span)           // カスタム span
    .build(&custom_i18n)   // カスタム i18n
```

---

## 詳細設計

### エラーコード一覧

#### E0xxx：字句解析と構文解析

| コード | エラー種別                | 説明                                   |
| ------ | ------------------------- | -------------------------------------- |
| E0001  | Invalid character         | ソースコードに不正な文字が含まれている |
| E0002  | Invalid number literal    | 数値リテラルの形式が正しくない         |
| E0003  | Unterminated string       | 複数行文字列の終了引用符が欠落         |
| E0004  | Invalid character literal | 文字リテラルが正しくない               |
| E0010  | Expected token            | 構文解析時に特定の token が必要        |
| E0011  | Unexpected token          | 予期しない token が出現                |
| E0012  | Invalid syntax            | 式/文の構文エラー                      |
| E0013  | Mismatched brackets       | 丸括弧、角括弧、波括弧が一致しない     |
| E0014  | Missing semicolon         | 文末にセミコロンが欠落                 |

#### E1xxx：型チェック

| コード | エラー種別               | 説明                                   |
| ------ | ------------------------ | -------------------------------------- |
| E1001  | Unknown variable         | 参照された変数が未定義                 |
| E1002  | Type mismatch            | 期待する型と実際の型が一致しない       |
| E1003  | Unknown type             | 参照された型が存在しない               |
| E1010  | Parameter count mismatch | 関数呼び出しの引数数が定義と一致しない |
| E1011  | Parameter type mismatch  | 引数の型チェックが失敗                 |
| E1012  | Return type mismatch     | 関数の戻り値の型が誤り                 |
| E1013  | Function not found       | 未定義の関数を呼び出し                 |
| E1020  | Cannot infer type        | コンテキストから型を推論できない       |
| E1021  | Type inference conflict  | 複数の制約により型の矛盾が発生         |
| E1030  | Pattern non-exhaustive   | match 式がすべての場合を網羅していない |
| E1031  | Unreachable pattern      | 決してマッチしないパターン             |
| E1040  | Operation not supported  | 型がその操作をサポートしていない       |
| E1041  | Index out of bounds      | 配列/リストのインデックスが範囲外      |
| E1042  | Field not found          | 存在しない構造体フィールドにアクセス   |

#### E2xxx：意味解析

| コード | エラー種別           | 説明                               |
| ------ | -------------------- | ---------------------------------- |
| E2001  | Scope error          | 変数が現在のスコープに存在しない   |
| E2002  | Duplicate definition | 同一スコープ内で重複定義           |
| E2003  | Lifetime error       | ライフタイム制約が満たされない     |
| E2010  | Immutable assignment | 不変変数への代入を試みた           |
| E2011  | Uninitialized use    | 未初期化変数の使用                 |
| E2012  | Mutability conflict  | 不変コンテキストでの可変参照の使用 |

#### E4xxx：ジェネリクスとトレイト

| コード | エラー種別                     | 説明                                      |
| ------ | ------------------------------ | ----------------------------------------- |
| E4001  | Generic parameter mismatch     | ジェネリクスパラメータの数/型が一致しない |
| E4002  | Trait bound violated           | trait 制約が満たされない                  |
| E4003  | Associated type error          | 関連型の定義/使用エラー                   |
| E4004  | Duplicate trait implementation | 同一 trait の重複実装                     |
| E4005  | Trait not found                | 要求される trait が見つからない           |
| E4006  | Sized bound violated           | Sized 制約が満たされない                  |

#### E5xxx：モジュールとインポート

| コード | エラー種別          | 説明                                     |
| ------ | ------------------- | ---------------------------------------- |
| E5001  | Module not found    | インポートされたモジュールが存在しない   |
| E5002  | Cyclic import       | モジュール間の循環依存                   |
| E5003  | Symbol not exported | 未エクスポートのシンボルへのアクセス試行 |
| E5004  | Invalid module path | モジュールパスの形式が誤り               |
| E5005  | Private access      | プライベートシンボルへのアクセス         |

#### E6xxx：ランタイムエラー

| コード | エラー種別                  | 説明                                      |
| ------ | --------------------------- | ----------------------------------------- |
| E6001  | Division by zero            | 整数のゼロ除算                            |
| E6002  | ~~Assertion failed~~        | ~~予約枠（言語概念なし、削除済み）~~      |
| E6003  | Runtime index out of bounds | ランタイムインデックス範囲外（#280 連携） |
| E6004  | Stack overflow              | スタック領域の枯渇                        |
| E6005  | Assertion failed            | assert 失敗（#280 連携）                  |
| E6006  | Function not found          | ランタイムで関数が見つからない            |
| E6007  | Runtime error (generic)     | 汎用ランタイムエラー                      |

> **#280 改訂（2026-08-09）**：コード表は元々 Rust セマンティクス草案（Assertion failed/Arithmetic
> overflow/Heap allocation failed/Type cast
> failed）に基づいて定義されていたが、実装の実際の要件と一致しなかった。YaoXiang には null ポインタ/ヒープ割り当て失敗/型変換の概念がなく（値セマンティクス +
> Rust メモリ安全性）、ランタイムオーバーフロー経路は検出未実装。校正後：
>
> - E6002 削除（旧 Assertion failed は E6005 に移動、旧 null ポインタセマンティクスは言語概念なし）
> - E6003 は Arithmetic overflow から Runtime index out of
>   bounds に変更（実際のトリガー場面、#279/#271）
> - E6005 は Heap allocation failed から Assertion failed に変更（std.assert 実際のパス）
> - E6006 は Runtime index out of bounds から Function not found に変更（実装は既にこの通り、#255）
> - E6007 は Type cast failed から汎用 Runtime
>   error に変更（ExecutorError の未マップ変体の統一ランディングポイント）

#### E7xxx：I/O とシステムエラー

| コード | エラー種別        | 説明                             |
| ------ | ----------------- | -------------------------------- |
| E7001  | File not found    | 存在しないファイルの読み込み試行 |
| E7002  | Permission denied | ファイル権限不足                 |
| E7003  | I/O error         | 汎用 I/O エラー                  |
| E7004  | Network error     | ネットワーク操作の失敗           |

#### E8xxx：内部コンパイラエラー

| コード | エラー種別              | 説明                    |
| ------ | ----------------------- | ----------------------- |
| E8001  | Internal compiler error | コンパイラの内部エラー  |
| E8002  | Codegen error           | IR/バイトコード生成失敗 |
| E8003  | Unimplemented feature   | 未実装機能の使用        |
| E8004  | Optimization error      | コンパイラ最適化エラー  |

---

### ランタイムエラー値とコードの連結

> 本節は #323（M4 ランタイム Error 値にコードを付与、2026-09-03）により導入。E6xxx/E7xxx セマンティクス空間は 2 つのチャネルを同時に担い、コード空間は同一、提示チャネルは異なる。

#### 2 つのチャネル

| チャネル                     | 载体                                                  | 提示方式                                                          |
| ---------------------------- | ----------------------------------------------------- | ----------------------------------------------------------------- |
| コンパイラ/CLI 診断チャネル  | `ExecutorError` などのホスト層ハードエラー            | stderr `error[E####]:`（#280/#281 で E6003/E6005/E6007 連携済み） |
| プログラム内エラー値チャネル | std ライブラリ `Result(T, Error)` の Err 载体 `Error` | 言語値、プログラムが match/比較で消費                             |

#### Error 構造（v0.8 より、破壊的変更）

```
Error { code: String, message: String }
```

- `code` は本規範の E6xxx/E7xxx 番号を再利用、文字列形式（例：`"E6008"`）。
- **安定契約**：割り当て済みのコードはバージョン間で意味が変わらない；同一意味に削除済みコードを再利用しない（E6002 が前例）。
- **消費面**：プログラム内の `e.code == "E6xxx"`
  比較が唯一のプログラマブル判定契約；`yaoxiang explain E6xxx`
  ドキュメントと連結；ツールチェーン（LSP / DAP、RFC-034 参照）はコードを exceptionId として扱う。
- **アクセサ**：`std.result.code(e)` / `std.result.message(e)`。
- **ユーザー定義エラー**：`Result(T, E)`
  の E はジェネリクスパラメータ、真剣にモデリングする場合はユーザー定義型を使用；std `Error`
  は便利なフォールバック载体に過ぎず、そのコード体系はユーザー E 型を制約しない。

#### コード割り当てルール

1. ランタイムエラー値コードとコンパイル時診断コードは E6xxx/E7xxx 空間を共有し、新しいコードは**実際のトリガー場面**に基づいて割り当てる。想像上のシナリオに対して予約しない。
2. 登録してから使用：新しいコードは権威あるレジストリに登録され、3 者間の一貫性検証（codes/*.rs ↔
   locales ↔ 本ドキュメントのコード表）を経た後にのみ発行可能。
3. E7xxx は std.io /
   std.net エラー値のために予約されたセグメント（現在は空状態、io/net の Result 化時に有効化）。

#### 進化パス（線 C、未実施）

パターンマッチの完備化（RFC-039）着地後、`Error` は `{ kind: ErrorKind, message: String }`
にアップグレード可能で、`code`
は kind から派生する属性となる（バリアント定義箇所がコードレジストリ）。進化期間中、本節のコード安定契約は変更なし；このアップグレードは独立した決定であり、本節のコミットメントを構成しない。

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

#### I18nRegistry 実装

```rust
// locales/*.json（エラーコードオブジェクト）

/// i18n 表示テキストレジストリ（コンパイル時に JSON からロード、ランタイムはテーブル参照なし）
pub struct I18nRegistry {
    /// タイトル
    titles: HashMap<&'static str, &'static str>,
    /// 説明
    messages: HashMap<&'static str, &'static str>,
    /// ヘルプ情報
    helps: HashMap<&'static str, &'static str>,
    /// サンプルコード
    examples: HashMap<&'static str, &'static str>,
    /// エラー出力例
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
    /// 言語コードに基づいてレジストリを取得
    pub fn new(lang: &str) -> Self {
        match lang {
            "zh" => Self::zh(),
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

##### 定義済みプレースホルダ（よく使用）

| プレースホルダ | 用途                             | 例                                  |
| -------------- | -------------------------------- | ----------------------------------- |
| `{name}`       | 変数名/型名/トレイト名等の識別子 | `Unknown variable: '{name}'`        |
| `{expected}`   | 期待される型                     | `Expected type '{expected}'`        |
| `{found}`      | 実際/見つかった型                | `, found type '{found}'`            |
| `{method}`     | メソッド名                       | `Method {method} is not a function` |
| `{trait}`      | トレイト名                       | `Cannot find trait: {trait}`        |
| `{path}`       | モジュールパス                   | `Invalid path: {path}`              |
| `{ty}`         | 型式                             | `Invalid type: {ty}`                |
| `{message}`    | 内部エラーメッセージ             | `Internal error: {message}`         |

##### 任意の key をサポート

**params は任意の key をサポートし、定義済みに限定されない**。呼び出し側は任意の `key` を渡せる：

```rust
// 任意の key を使用
E1001::unknown_variable(&var_name)
    .param("location", "global scope")
    .param("hint", "try declaring it first")
    .at(span)
    .build(&i18n);

// テンプレート定義
"Unknown variable: '{name}' at {location}. {hint}"
```

> **注意**：すべてのエラーコードがプレースホルダを使用するわけではない。一部のエラーコード（E0001 など）は静的メッセージで、パラメータは不要。

#### 言語優先順位

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
# エラーメッセージ言語、選択肢：en, zh, ja, ...
default = "zh"
```

#### ユーザーレベル設定

```toml
# ~/.yaoxiang/yaoxiang.toml
[language]
default = "zh"
```

#### コンパイル時言語選択

```
1. プロジェクトレベル yaoxiang.toml の language.default を読む
2. 未設定の場合、ユーザーレベル ~/.yaoxiang/yaoxiang.toml を読む
3. どちらも未設定の場合、デフォルトで "en" を使用
4. コンパイラは選択された言語に基づいて I18nRegistry を作成（一度だけ）
5. すべてのエラーはこの I18nRegistry を使用してメッセージをレンダリング
```

#### テーブル参照オーバーヘッドゼロの鍵

**レンダリングはユーザープロジェクトのコンパイル時に行われ、ランタイムではない。**

```
┌─────────────────────────────────────────────────────────────────────────┐
│  段階 1: Rust コンパイラが YaoXiang コンパイラをコンパイル               │
│                                                                           │
│  JSON をコンパイラバイナリにパッケージ化                                │
│  目的：explain コマンドが i18n データを直接読み取れるように               │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  段階 2: YaoXiang がユーザープロジェクトをコンパイル（レンダリング発生）│
│                                                                           │
│  error! マクロ呼び出し時：                                              │
│  1. yaoxiang.toml を読んで言語設定を取得                                │
│  2. コンパイラバイナリから対応言語の i18n JSON をロード                  │
│  3. テンプレート + パラメータ → render() → "Unknown variable: 'x'"     │
│  4. Diagnostic.message = レンダリング済み文字列                        │
│                                                                           │
│  AOT バイナリは最終文字列を直接格納、テンプレートなし、テーブル参照なし│
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  段階 3: ユーザープログラムランタイム                                    │
│                                                                           │
│  println!("{}", diagnostic.message)                                      │
│  // 最終文字列を直接出力、テーブル参照なし                              │
└─────────────────────────────────────────────────────────────────────────┘
```

| コンポーネント               | 責務                                   | レンダリングタイミング           |
| ---------------------------- | -------------------------------------- | -------------------------------- |
| `I18nRegistry`               | テンプレートと表示テキスト提供         | ユーザープロジェクトコンパイル時 |
| `DiagnosticBuilder.render()` | テンプレート + パラメータ → 最終文字列 | ユーザープロジェクトコンパイル時 |
| `Diagnostic.message`         | レンダリング済み文字列                 | 最終結果格納                     |
| AOT バイナリ                 | 最終文字列を含む                       | ランタイム直接使用               |

---

### エラーメッセージ形式

エラーメッセージは以下の形式を採用：

```
error[E####]: <簡潔な説明>
  --> <ファイル>:<行>:<列>
   <行> | <コードスニペット>
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

エラーの重大度は `DiagnosticLevel` 列挙で管理され、エラーコード番号と分離されている：

```rust
pub enum DiagnosticLevel {
    Error,    // コンパイル失敗を引き起こす
    Warning,  // コンパイルに影響しないが、修正を推奨
    Note,     // 補足情報
    Help,     // 修正提案
}
```

| レベル  | プレフィックス    | 説明                       |
| ------- | ----------------- | -------------------------- |
| Error   | `error[E####]:`   | コンパイル失敗を引き起こす |
| Warning | `warning[E####]:` | コンパイルに影響しない     |
| Note    | `note[E####]:`    | 補足情報                   |
| Help    | `help[E####]:`    | 修正提案                   |

---

### `yaoxiang explain` コマンド

#### コマンド構文

```bash
yaoxiang explain <ERROR_CODE> [OPTIONS]
```

#### オプション

| オプション      | 説明                                        |
| --------------- | ------------------------------------------- |
| `--lang <code>` | 言語を指定 (en-US, zh-CN、デフォルト en-US) |
| `--json`        | JSON 形式で出力（IDE/LSP 用）               |
| `--json-pretty` | フォーマット済み JSON 出力                  |
| `--examples`    | サンプルコードのみ表示                      |
| `--help`        | ヘルプ情報を表示                            |

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

# JSON 出力（LSP 統合）
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

**将来のマイグレーション戦略**（後続バージョン参考用）：

1. 旧エラーコードと新エラーコードのマッピングを保持
2. マイグレーション期間中は新旧コードを同時表示
3. 非推奨タイムテーブルを提供

---

## 実装戦略

### 段階 1：エラーコード基礎アーキテクチャ

1. `src/diagnostics/` ディレクトリ構造を作成
2. `ErrorCode` 列挙を実装
3. `Diagnostic` と `DiagnosticLevel` を実装
4. リソースファイルディレクトリとサンプル JSON を作成

### 段階 2：explain コマンド

1. `yaoxiang explain` CLI コマンドを実装
2. `--lang` と `--json` オプションをサポート
3. リソースファイル読み込みを統合
4. パラメータテンプレートのレンダリングを実装

### 段階 3：コンパイル時統合

1. すべてのエラー報告ポイントを更新して新システムを使用
2. メッセージテンプレートパラメータ注入を実装
3. 言語優先順位ロジックを追加
4. ユニットテストでカバレッジを確保

### 段階 4：IDE/LSP 統合

1. LSP サーバーに explain JSON 出力を統合
2. IDE にエラーコードリンクを表示
3. ホバーでエラー解説を表示
4. クイック修正提案

---

## 付録

### 完全エラーコード早見表

| 範囲  | カテゴリ               |
| ----- | ---------------------- |
| E0xxx | 字句解析と構文解析     |
| E1xxx | 型チェック             |
| E2xxx | 意味解析               |
| E3xxx | コード生成             |
| E4xxx | ジェネリクスとトレイト |
| E5xxx | モジュールとインポート |
| E6xxx | ランタイムエラー       |
| E7xxx | I/O とシステムエラー   |
| E8xxx | 内部コンパイラエラー   |
| E9xxx | 予約                   |

### サポート言語

| コード | 言語         | 状態       |
| ------ | ------------ | ---------- |
| en-US  | English (US) | デフォルト |
| zh-CN  | 簡体字中国語 | 計画中     |

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
```

## 参考文献

- [Rust コンパイラエラー索引](https://doc.rust-lang.org/error_codes/error-index.html)
- [GCC エラーメッセージ形式](https://gcc.gnu.org/onlinedocs/gcc-13.1.0/gcc/Warning-Options.html)
- [Clang 診断形式](https://clang.llvm.org/diagnostics.html)
- [Language Server Protocol](https://microsoft.github.io/language-server-protocol/)

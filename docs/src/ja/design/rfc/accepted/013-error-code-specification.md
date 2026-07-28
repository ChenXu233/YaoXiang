---
title: 'RFC 013: エラーコード仕様'
status: '承認済み'
author: '晨煦'
created: '2026-02-02'
updated: '2026-02-12'
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

本 RFC は YaoXiang コンパイラのエラーコード分類仕様を提案する。Rust のような単層番号システムを採用し、JSON リソースファイルで多言語サポートを実現し、`yaoxiang explain`
コマンドでエラー説明機能を提供する。

## 動機

### なぜエラーコードの標準化が必要なのか？

1. **ユーザー体験**：ユーザーがエラーコードを見ることで、エラーの種類と重大度をすぐに判断できる
2. **ドキュメント整理**：カテゴリ別分類により、エラー参照ドキュメントの作成と保守が容易になる
3. **ツール統合**：IDE/LSP はエラーコードに基づいてクイックフィックス提案やドキュメントリンクを提供できる
4. **国際化サポート**：エラーメッセージとコードが分離されているため、多言語への翻訳が容易

### 設計目標

- **簡潔**：単層番号で、複雑な分類ルールを記憶する必要がない
- **親切**：Rust のようなエラーメッセージ形式でヘルプ信息和示例を含む
- **拡張可能**：リソースファイル驱动で、新しいエラーや新しい言語を追加しやすい
- **ツールフレンドリー**：explain コマンド + JSON 出力で IDE/LSP 統合をサポート

---

## 提案

### コア設計：単層番号システム

4桁の数字编号を採用し、コンパイル段階でグループ分けする：

```
Exxxx
││││
│││└── 番号 (000-999)
││└─── コンパイル段階 (0-9)
└───── 固定接頭辞 'E'
```

### 段階分け

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

### エラーカテゴリ列挙型

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

**コア原則**：エラーコード定義と表示メッセージを分離

- `ErrorCodeDefinition`：エラーのメタデータ（code、category、template）で、表示メッセージを含まない
- `i18n/*.json`：各言語の表示メッセージ（title、message、help）
- `DiagnosticBuilder`：汎用ビルダーで、trait-per-error 設計の代わり

#### エラーコード定義

```rust
// diagnostic/codes/mod.rs

use crate::util::span::Span;
use crate::util::diagnostic::{Diagnostic, Severity};

/// エラーコード定義（メタデータのみ、表示メッセージは i18n ファイル）
#[derive(Debug, Clone, Copy)]
pub struct ErrorCodeDefinition {
    pub code: &'static str,
    pub category: ErrorCategory,
    pub message_template: &'static str,  // メッセージテンプレート、{param} プレースホルダー対応
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

    /// Diagnostic をビルド（テンプレートレンダリングはコンパイル時に完了）
    pub fn build(&self, i18n: &I18nRegistry) -> Diagnostic {
        // テンプレート内のすべての {key} に対応するパラメータがあることを確認
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

#### 各エラーコードのクイックメソッド

```rust
// diagnostic/codes/e1xxx.rs

impl ErrorCodeDefinition {
    /// E1001 未知の変数
    pub fn unknown_variable(name: &str) -> DiagnosticBuilder {
        let def = Self::find("E1001").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template)
            .param("name", name)
    }

    /// E1002 型が一致しない
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

// 簡略化 방법
return Err(E1001::unknown_variable(&var_name)
    .at(span)
    .build(&i18n_registry));

// 手動 방법
return Err(ErrorCodeDefinition::find("E1001")
    .builder()
    .param("name", var_name)
    .at(span)
    .build(&i18n_registry));
```

#### エラーコード定義の例

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
    // ... 他のエラーコード
];
```

#### 設計の優位性

| 特性                   | 説明                                                       |
| ---------------------- | ---------------------------------------------------------- |
| **单一 Builder**       | 1つの `DiagnosticBuilder` がすべてのエラーコードに使用可能 |
| **型安全**             | クイックメソッドがパラメータの正確性を保証                 |
| **自己文書化**         | `E1001::unknown_variable(name)` が一目でわかる             |
| **テンプレート分離**   | メッセージテンプレートとコードが分離され、i18n が容易      |
| **ゼロオーバーヘッド** | コンパイル時レンダリング、AOT バイナリにはルックアップ不要 |

---

### エラーメacroの簡略化

#### error! macro（自動コンテキスト注入）

```rust
/// コンパイル時に span と i18n 設定を自動取得する macro
macro_rules! error {
    ($code:ident, $($key:ident = $value:expr),* $(,)?) => {
        $code()
            $(.$key($value))*
            .at(crate::util::span::Span::current())
            .build(crate::util::diagnostic::I18nRegistry::current())
    };
}

/// 使用：パラメータ만 전달하면 되고、span と i18n は自動注入
return Err(error!(E1001, name = var_name));
return Err(error!(E1002, expected = "bool", found = cond_ty));
```

#### 手動で Builder を使用

```rust
// 手動で制御する必要がある場合
E1001::unknown_variable(&var_name)
    .at(my_span)           // カスタム span
    .build(&custom_i18n)   // カスタム i18n
```

---

## 詳細設計

### エラーコードリスト

#### E0xxx：字句解析と構文解析

| コード | エラー種類                | 説明                                    |
| ------ | ------------------------- | --------------------------------------- |
| E0001  | Invalid character         | ソースコードに不正な文字が含まれている  |
| E0002  | Invalid number literal    | 数字リテラルの形式が不正                |
| E0003  | Unterminated string       | 複数行文字列に終了引用符がない          |
| E0004  | Invalid character literal | 文字リテラルが不正                      |
| E0010  | Expected token            | 構文解析時に特定の token を期待していた |
| E0011  | Unexpected token          | 予期しない token に遭遇                 |
| E0012  | Invalid syntax            | 式/文の構文エラー                       |
| E0013  | Mismatched brackets       | 丸括弧、角括弧、波括弧が不一致          |
| E0014  | Missing semicolon         | 文の末尾にセミコロンがない              |

#### E1xxx：型チェック

| コード | エラー種類               | 説明                                         |
| ------ | ------------------------ | -------------------------------------------- |
| E1001  | Unknown variable         | 参照された変数が未定義                       |
| E1002  | Type mismatch            | 期待する型と実際の型が一致しない             |
| E1003  | Unknown type             | 参照された型が存在しない                     |
| E1010  | Parameter count mismatch | 関数呼び出しのパラメータ数が定義と一致しない |
| E1011  | Parameter type mismatch  | パラメータの型チェックに失敗                 |
| E1012  | Return type mismatch     | 関数の戻り値の型エラー                       |
| E1013  | Function not found       | 未定義の関数を呼び出そうとしている           |
| E1020  | Cannot infer type        | 文脈から型を推論できない                     |
| E1021  | Type inference conflict  | 複数の制約により型の矛盾が発生               |
| E1030  | Pattern non-exhaustive   | match 式がすべてのケースをカバーしていない   |
| E1031  | Unreachable pattern      | 決してマッチしないパターン                   |
| E1040  | Operation not supported  | その型は 해당 操作をサポートしていない       |
| E1041  | Index out of bounds      | 配列/リストのインデックスが範囲外            |
| E1042  | Field not found          | 存在しない構造体フィールドにアクセス         |

#### E2xxx：意味解析

| コード | エラー種類           | 説明                               |
| ------ | -------------------- | ---------------------------------- |
| E2001  | Scope error          | 変数が現在のスコープにない         |
| E2002  | Duplicate definition | 同一スコープ内での重複定義         |
| E2003  | Lifetime error       | ライフタイム制約が満たされていない |
| E2010  | Immutable assignment | 不変変数を変更しようとしている     |
| E2011  | Uninitialized use    | 未初期化の変数を使用している       |
| E2012  | Mutability conflict  | 不変コンテキストで可変参照を使用   |

#### E4xxx：ジェネリクスとトレイト

| コード | エラー種類                     | 説明                                      |
| ------ | ------------------------------ | ----------------------------------------- |
| E4001  | Generic parameter mismatch     | ジェネリックパラメータの数/型が一致しない |
| E4002  | Trait bound violated           | トレイト制約が満たされていない            |
| E4003  | Associated type error          | 関連型の定義/使用エラー                   |
| E4004  | Duplicate trait implementation | 同一トレイトの重複実装                    |
| E4005  | Trait not found                | 要求されたトレイトが見つからない          |
| E4006  | Sized bound violated           | Sized 制約が満たされていない              |

#### E5xxx：モジュールとインポート

| コード | エラー種類          | 説明                                                       |
| ------ | ------------------- | ---------------------------------------------------------- |
| E5001  | Module not found    | インポートされたモジュールが存在しない                     |
| E5002  | Cyclic import       | モジュール間の循環依存                                     |
| E5003  | Symbol not exported | エクスポートされていないシンボルにアクセスしようとしている |
| E5004  | Invalid module path | モジュールパス形式エラー                                   |
| E5005  | Private access      | プライベートシンボルへのアクセス                           |

#### E6xxx：ランタイムエラー

| コード | エラー種類                  | 説明                                         |
| ------ | --------------------------- | -------------------------------------------- |
| E6001  | Division by zero            | 整数除算でゼロ除算                           |
| E6002  | Assertion failed            | assert! macro が失敗                         |
| E6003  | Arithmetic overflow         | 算術演算のオーバーフロー                     |
| E6004  | Stack overflow              | スタック領域の枯渇                           |
| E6005  | Heap allocation failed      | メモリ割り当て失敗                           |
| E6006  | Runtime index out of bounds | ランタイム時のインデックス範囲外             |
| E6007  | Type cast failed            | 型を互換性のない型にキャストしようとしている |

#### E7xxx：I/O とシステムエラー

| コード | エラー種類        | 説明                                     |
| ------ | ----------------- | ---------------------------------------- |
| E7001  | File not found    | 存在しないファイルを読み込もうとしている |
| E7002  | Permission denied | ファイル権限が不足                       |
| E7003  | I/O error         | 汎用 I/O エラー                          |
| E7004  | Network error     | ネットワーク操作の失敗                   |

#### E8xxx：内部コンパイラエラー

| コード | エラー種類              | 説明                     |
| ------ | ----------------------- | ------------------------ |
| E8001  | Internal compiler error | コンパイラの内部エラー   |
| E8002  | Codegen error           | IR/バイトコード生成失敗  |
| E8003  | Unimplemented feature   | 未実装の機能を使用       |
| E8004  | Optimization error      | コンパイラの最適化エラー |

---

### 多言語リソースファイル

#### リソースファイル形式

```json
// diagnostic/codes/i18n/en.json
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
// diagnostic/codes/i18n/zh.json
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
// diagnostic/codes/i18n/mod.rs

/// i18n 表示メッセージレジストリ（コンパイル時に JSON からロード、ランタイム時のルックアップ不要）
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

    /// テンプレートをレンダリング（コンパイル時に完了、ランタイム時のオーバーヘッドなし）
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

#### テンプレートプレースホルダー

##### 定義済みプレースホルダー（よく使用するもの）

| プレースホルダー | 用途                               | 例                                  |
| ---------------- | ---------------------------------- | ----------------------------------- |
| `{name}`         | 変数名/型名/トレイト名などの識別子 | `Unknown variable: '{name}'`        |
| `{expected}`     | 期待する型                         | `Expected type '{expected}'`        |
| `{found}`        | 実際の/見つかった型                | `, found type '{found}'`            |
| `{method}`       | メソッド名                         | `Method {method} is not a function` |
| `{trait}`        | トレイト名                         | `Cannot find trait: {trait}`        |
| `{path}`         | モジュールパス                     | `Invalid path: {path}'`             |
| `{ty}`           | 型式                               | `Invalid type: {ty}`                |
| `{message}`      | 内部エラーメッセージ               | `Internal error: {message}`         |

##### 任意の key サポート

**params は定義済みだけでなく任意の key をサポート**。呼び出し元は任意の `key` を渡せる：

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

> **注意**：必ずしもすべてのエラーコードがプレースホルダーを使用するわけではない。一部のエラーコード（E0001 など）は静的メッセージであり、パラメータを必要としない。

#### 言語優先順位

```
1. yaoxiang.toml [language.default]
2. ~/.yaoxiang/yaoxiang.toml [language.default]
3. デフォルト値: en
```

### yaoxiang.toml 設定

#### プロジェクトレベルの設定

```toml
# yaoxiang.toml
[project]
name = "my-project"
version = "0.1.0"

[language]
# エラーメッセージ言語、省略可能：en, zh, ja, ...
default = "zh"
```

#### ユーザーレベルの設定

```toml
# ~/.yaoxiang/yaoxiang.toml
[language]
default = "zh"
```

#### コンパイル時の言語選択

```
1. プロジェクトレベルの yaoxiang.toml の language.default を読み込む
2. 設定されていない場合、ユーザーレベルの ~/.yaoxiang/yaoxiang.toml を読み込む
3. どちらも設定されていない場合、デフォルトで "en" を使用
4. コンパイラは選択した言語に基づいて I18nRegistry を生成する（1回）
5. すべてのエラーはその I18nRegistry を使用してメッセージをレンダリング
```

#### ゼロ・ルックアップ・オーバーヘッドの鍵

**レンダリングはユーザーのプロジェクトのコンパイル時に発生し、ランタイムではない。**

```
┌─────────────────────────────────────────────────────────────────────────┐
│  段階 1: Rust で YaoXiang コンパイラをコンパイル                        │
│                                                                           │
│  JSON がコンパイラのバイナリにパックされる                               │
│  目的：explain コマンドが i18n データを直接読み取れる                    │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  段階 2: YaoXiang でユーザーのプロジェクトをコンパイル（レンダリング発生） │
│                                                                           │
│  error! macro 呼び出し時：                                               │
│  1. yaoxiang.toml から言語設定を読み込む                                 │
│  2. コンパイラのバイナリから対応する言語の i18n JSON をロード            │
│  3. テンプレート + パラメータ → render() → "Unknown variable: 'x'"      │
│  4. Diagnostic.message = レンダリング済みの文字列                        │
│                                                                           │
│  AOT バイナリは最終文字列を直接保存、テンプレートなし、ルックアップなし  │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  段階 3: ユーザープログラムのランタイム                                   │
│                                                                           │
│  println!("{}", diagnostic.message)                                      │
│  // 最終文字列を直接出力、ルックアップなし                                │
└─────────────────────────────────────────────────────────────────────────┘
```

| コンポーネント               | 責務                                   | レンダリングタイミング             |
| ---------------------------- | -------------------------------------- | ---------------------------------- |
| `I18nRegistry`               | テンプレートと表示メッセージを提供     | ユーザーのプロジェクトコンパイル時 |
| `DiagnosticBuilder.render()` | テンプレート + パラメータ → 最終文字列 | ユーザーのプロジェクトコンパイル時 |
| `Diagnostic.message`         | レンダリング済みの文字列               | 最終結果を保存                     |
| AOT バイナリ                 | 最終文字列を含む                       | ランタイムで直接使用               |

---

### エラーメッセージ形式

エラーメッセージは以下の形式を採用：

```
error[E####]: <短い説明>
  --> <ファイル>:<行>:<列>
   <行> | <コード断片>
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

エラーの重大度は `DiagnosticLevel` 列挙型で管理され、エラーコード番号とは分離されている：

```rust
pub enum DiagnosticLevel {
    Error,    // コンパイル失敗を引き起こす
    Warning,  // コンパイルには影響しないが、修正を推奨
    Note,     |/ 補足情報
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

| オプション      | 説明                                        |
| --------------- | ------------------------------------------- |
| `--lang <code>` | 言語を指定 (en-US, zh-CN, デフォルト en-US) |
| `--json`        | JSON 形式出力（IDE/LSP 向け）               |
| `--json-pretty` | フォーマットされた JSON 出力                |
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
error[E1001]: 未知変数: {name}
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

### 下位互換性

本 RFC はゼロからエラーコードシステムを設計するため、下位互換性の問題はない。

**将来の移行戦略**（後続バージョンの参照用）：

1. 旧エラーコードから新エラーコードへのマッピングを維持
2. 移行期間中は新旧両方のコードを表示
3. 廃止スケジュールを提供

---

## 実施戦略

### 段階一：错误コード基盤インフラ

1. `src/diagnostics/` ディレクトリ構造を作成
2. `ErrorCode` 列挙型を実装
3. `Diagnostic` と `DiagnosticLevel` を実装
4. リソースファイルディレクトリとサンプル JSON を作成

### 段階二：explain コマンド

1. `yaoxiang explain` CLI コマンドを実装
2. `--lang` と `--json` オプションをサポート
3. リソースファイルのロードを統合
4. パラメータテンプレートのレンダリングを実装

### 段階三：コンパイル時統合

1. すべてのエラー報告箇所を更新して新システムを使用
2. メッセージテンプレートパラメータ注入を実装
3. 言語優先順位ロジックを追加
4. ユニットテストカバレッジ

### 段階四：IDE/LSP 統合

1. LSP サーバーが explain JSON 出力を統合
2. IDE にエラーコードリンクを表示
3. ホバーでエラー説明を表示
4. クイックフィックス提案

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

### サポートされている言語

| コード | 言語         | ステータス |
| ------ | ------------ | ---------- |
| en-US  | English (US) | デフォルト |
| zh-CN  | 简体中文     | 計画中     |

### エラーメッセージ例比較

```
# 英語 (en-US)
error[E1001]: Unknown variable: x
  --> src/main.yx:5:12
   5 |   print(x)
          ^
          help: Did you mean to define it?

# 中国語 (zh-CN)
error[E1001]: 未知変数: x
  --> src/main.yx:5:12
   5 |   print(x)
          ^
          帮助: 你是否想要定义它？
```

## 参考文献

- [Rust コンパイラエラーインデックス](https://doc.rust-lang.org/error_codes/error-index.html)
- [GCC エラーメッセージ形式](https://gcc.gnu.org/onlinedocs/gcc-13.1.0/gcc/Warning-Options.html)
- [Clang 診断形式](https://clang.llvm.org/diagnostics.html)
- [Language Server Protocol](https://microsoft.github.io/language-server-protocol/)

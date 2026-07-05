---
title: "RFC 013: エラーコード仕様"
status: "受領"
author: "晨煦"
created: "2026-02-02"
updated: "2026-02-12"
---

# RFC 013: エラーコード仕様

## 概要

本 RFC は YaoXiang コンパイラのエラーコード分類仕様を提案する。Rust 类似的单层番号システムを採用し、JSON リソースファイルと連携した多言語サポート、`yaoxiang explain` コマンドによるエラー説明機能を提供する。

## 動機

### なぜエラーコードの標準化が必要なのか？

1. **ユーザー体験**：ユーザーがエラーコードを見ることで、エラーの種類と重大度を素早く判断できる
2. **ドキュメンテーションの整理**：カテゴリ別にグループ化することで、エラーリファレンスドキュメントの作成と保守が容易になる
3. **ツール統合**：IDE/LSP はエラーコードに基づいてクイックフィックス提案とドキュメントリンクを提供できる
4. **国際化サポート**：エラーメッセージとコードが分離されているため、多言語翻訳が容易

### 設計目標

- **簡潔**：单层番号で、複雑な分類ルールを覚える必要がない
- **優しい**：Rust 类似のエラーメッセージ形式でヘルプ信息和示例を含む
- **拡張可能**：リソースファイル驱动で、新しいエラーや新しい言語の追加が容易
- **ツールフレンドリー**：explain コマンド + JSON 出力で IDE/LSP 統合に対応

---

## 提案

### コア設計：单层番号システム

四桁の数字番号を採用し、コンパイル段階でグループ化する：

```
Exxxx
││││
│││└── 序号 (000-999)
││└─── コンパイル段階 (0-9)
└───── 固定プレフィックス 'E'
```

### 段階划分

| 段階 | 範囲 | 説明 |
|------|------|------|
| **0** | E0xxx | 字句解析と構文解析 |
| **1** | E1xxx | 型チェック |
| **2** | E2xxx | 意味解析 |
| **3** | E3xxx | コード生成 |
| **4** | E4xxx | ジェネリクスとトレイト |
| **5** | E5xxx | モジュールとインポート |
| **6** | E6xxx | 実行時エラー |
| **7** | E7xxx | I/O とシステムエラー |
| **8** | E8xxx | 内部コンパイラエラー |
| **9** | E9xxx | 予約/実験的 |

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
    Runtime,    // E6xxx: 実行時エラー
    Io,         // E7xxx: I/Oとシステムエラー
    Internal,   // E8xxx: 内部コンパイラエラー
}
```

### エラーコード定義と汎用ビルダー

**コア原則**：エラーコード定義と表示文案の分離

- `ErrorCodeDefinition`：エラーコードのメタデータ（code、category、template）で、表示文案を含まない
- `i18n/*.json`：各言語のの表示文案（title、message、help）
- `DiagnosticBuilder`：汎用ビルダーで、trait-per-error 設計の代替

#### エラーコード定義

```rust
// diagnostic/codes/mod.rs

use crate::util::span::Span;
use crate::util::diagnostic::{Diagnostic, Severity};

/// エラーコード定義（メタデータのみ、表示文案は i18n ファイル）
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

    /// Diagnostic をビルド（テンプレートのレンダリングはコンパイル時に完了）
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

#### 各エラーコードのショート-cutメソッド

```rust
// diagnostic/codes/e1xxx.rs

impl ErrorCodeDefinition {
    /// E1001 不明な変数
    pub fn unknown_variable(name: &str) -> DiagnosticBuilder {
        let def = Self::find("E1001").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template)
            .param("name", name)
    }

    /// E1002 型不一致
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

// 簡略化された方法
return Err(E1001::unknown_variable(&var_name)
    .at(span)
    .build(&i18n_registry));

// 手動方法
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
    // ... その他のエラーコード
];
```

#### 設計の優位性

| 特性 | 説明 |
|------|------|
| **単一ビルダー** | 1つの `DiagnosticBuilder` がすべてのエラーコードに対応 |
| **型安全** | ショート-cutメソッドでパラメータの正しさを保証 |
| **自己文書化** | `E1001::unknown_variable(name)` が一目で分かる |
| **テンプレート分離** | メッセージテンプレートとコードの分離で i18n が容易 |
| **実行時オーバーヘッドゼロ** | コンパイル時のレンダリング、AOT バイナリに查表なし |

---

### エラーメクロの簡略化

#### error! マクロ（コンテキストの自動注入）

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

/// 使用方法：パラメータのみを渡せばよく、span と i18n は自動注入
return Err(error!(E1001, name = var_name));
return Err(error!(E1002, expected = "bool", found = cond_ty));
```

#### ビルダーの手動使用

```rust
// 手動制御が必要な場合
E1001::unknown_variable(&var_name)
    .at(my_span)           // カスタム span
    .build(&custom_i18n)   // カスタム i18n
```

---

## 詳細設計

### エラーコードリスト

#### E0xxx：字句解析と構文解析

| コード | エラーの種類 | 説明 |
|------|----------|------|
| E0001 | Invalid character | ソースコードに不正な文字が含まれている |
| E0002 | Invalid number literal | 数字の字句量が正しくない形式 |
| E0003 | Unterminated string | 複数行文字列に終了引用符がない |
| E0004 | Invalid character literal | 文字の字句量が正しくない |
| E0010 | Expected token | 構文解析時に特定のトークンが期望された |
| E0011 | Unexpected token | 予期しないトークンに遭遇した |
| E0012 | Invalid syntax | 式/ステートメントの構文エラー |
| E0013 | Mismatched brackets | 丸括弧、角括弧、波括弧が一致しない |
| E0014 | Missing semicolon | ステートメント末尾にセミコロンがない |

#### E1xxx：型チェック

| コード | エラーの種類 | 説明 |
|------|----------|------|
| E1001 | Unknown variable | 参照された変数が定義されていない |
| E1002 | Type mismatch | 期望された型と実際の型が一致しない |
| E1003 | Unknown type | 参照された型が存在しない |
| E1010 | Parameter count mismatch | 関数呼び出しのパラメータ数が定義と一致しない |
| E1011 | Parameter type mismatch | パラメータの型チェック失敗 |
| E1012 | Return type mismatch | 関数の戻り値の型エラー |
| E1013 | Function not found | 未定義の関数を呼び出した |
| E1020 | Cannot infer type | コンテキストから型を推断できない |
| E1021 | Type inference conflict | 複数の制約が型 противоречие を起こす |
| E1030 | Pattern non-exhaustive | match 式がすべてのケースをカバーしていない |
| E1031 | Unreachable pattern | 決してマッチしないパターン |
| E1040 | Operation not supported | 型がその操作をサポートしていない |
| E1041 | Index out of bounds | 配列/リストのインデックスが範囲外 |
| E1042 | Field not found | 存在しない構造体フィールドにアクセスした |

#### E2xxx：意味解析

| コード | エラーの種類 | 説明 |
|------|----------|------|
| E2001 | Scope error | 変数が現在のスコープにない |
| E2002 | Duplicate definition | 同じスコープ内で重複定義した |
| E2003 | Lifetime error | ライフタイム制約が満たされていない |
| E2010 | Immutable assignment | 不変変数を変更しようとした |
| E2011 | Uninitialized use | 未初期化の変数を使用した |
| E2012 | Mutability conflict | 不変コンテキストで可変参照を使用した |

#### E4xxx：ジェネリクスとトレイト

| コード | エラーの種類 | 説明 |
|------|----------|------|
| E4001 | Generic parameter mismatch | ジェネリックパラメータの数/型が一致しない |
| E4002 | Trait bound violated | trait 制約を満たしていない |
| E4003 | Associated type error | 関連型の定義/使用エラー |
| E4004 | Duplicate trait implementation | 同じ trait を重複実装した |
| E4005 | Trait not found | 要求された trait が見つからない |
| E4006 | Sized bound violated | Sized 制約を満たしていない |

#### E5xxx：モジュールとインポート

| コード | エラーの種類 | 説明 |
|------|----------|------|
| E5001 | Module not found | インポートされたモジュールが存在しない |
| E5002 | Cyclic import | モジュール間の循環依存 |
| E5003 | Symbol not exported | エクスポートされていないシンボルにアクセスした |
| E5004 | Invalid module path | モジュールパス形式エラー |
| E5005 | Private access | プライベートシンボルにアクセスした |

#### E6xxx：実行時エラー

| コード | エラーの種類 | 説明 |
|------|----------|------|
| E6001 | Division by zero | 整数を零で割った |
| E6002 | Assertion failed | assert! マクロが失敗した |
| E6003 | Arithmetic overflow | 算術演算がオーバーフローした |
| E6004 | Stack overflow | スタック領域が使い果たされた |
| E6005 | Heap allocation failed | メモリアロケーション失敗 |
| E6006 | Runtime index out of bounds | 実行時にインデックスが範囲外 |
| E6007 | Type cast failed | 型を互換性のない型にキャストしようとした |

#### E7xxx：I/O とシステムエラー

| コード | エラーの種類 | 説明 |
|------|----------|------|
| E7001 | File not found | 存在しないファイルを読み込もうとした |
| E7002 | Permission denied | ファイル権限が不十分 |
| E7003 | I/O error | 汎用 I/O エラー |
| E7004 | Network error | ネットワーク操作失敗 |

#### E8xxx：内部コンパイラエラー

| コード | エラーの種類 | 説明 |
|------|----------|------|
| E8001 | Internal compiler error | コンパイラの内部エラー |
| E8002 | Codegen error | IR/バイトコード生成失敗 |
| E8003 | Unimplemented feature | 未実装の機能を使用した |
| E8004 | Optimization error | コンパイラの最適化エラー |

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
// diagnostic/codes/i18n/ja.json
{
  "E1001": {
    "title": "不明な変数",
    "message": "参照された変数が定義されていません",
    "template": "不明な変数: '{name}'",
    "help": "変数名が正しく入力されているか確認するか、先に定義してください",
    "example": "x = 100;",
    "error_output": "error[E1001]: 不不明な変数: 'x'\n  --> example.yx:1:1\n   |\n 1 | print(x)\n   | ^ 不不明な変数 'x'"
  },
  "E1002": {
    "title": "型不一致",
    "message": "期待された型と実際の型が一致しません",
    "template": "期待された型 '{expected}'、実際の型 '{found}'",
    "help": "正しい型を使用するか、型変換を追加してください",
    "example": "x: Int = \"hello\";",
    "error_output": "error[E1002]: 型不一致\n  --> example.yx:1:12\n   |\n 1 | x: Int = \"hello\";\n   |            ^ 期待された型 'Int'、実際の型 'String'"
  }
}
```

#### I18nRegistry 実装

```rust
// diagnostic/codes/i18n/mod.rs

/// i18n 表示文案レジストリ（コンパイル時に JSON からロード、実行時に查表なし）
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
            "ja" => Self::ja(),
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

    /// テンプレートをレンダリング（コンパイル時に完了、実行時オーバーヘッドなし）
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

##### 定義済みプレースホルダー（よく使用されるもの）

| プレースホルダー | 用途 | 例 |
|--------|------|------|
| `{name}` | 変数名/型名/トレイト名などの識別子 | `Unknown variable: '{name}'` |
| `{expected}` | 期待される型 | `Expected type '{expected}'` |
| `{found}` | 実際/見つかった型 | `, found type '{found}'` |
| `{method}` | メソッド名 | `Method {method} is not a function` |
| `{trait}` | トレイト名 | `Cannot find trait: {trait}` |
| `{path}` | モジュールパス | `Invalid path: {path}` |
| `{ty}` | 型式 | `Invalid type: {ty}` |
| `{message}` | 内部エラーメッセージ | `Internal error: {message}` |

##### 任意の key サポート

**params は定義済みのみならず任意の key をサポート**。呼び出し元は任意の `key` を渡せる：

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

> **注意**：すべてのエラーコードがプレースホルダーを使用するわけではない。一部のエラーコード（例：E0001）は静的メッセージで、パラメータが不要。

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
# エラーメッセージ言語（オプション：en, zh, ja, ...）
default = "ja"
```

#### ユーザーレベル設定

```toml
# ~/.yaoxiang/yaoxiang.toml
[language]
default = "ja"
```

#### コンパイル時の言語選択

```
1. プロジェクトレベルの yaoxiang.toml の language.default を読み取る
2. 設定されていない場合は ~/.yaoxiang/yaoxiang.toml を読み取る
3. どちらも設定されていない場合はデフォルトで "en" を使用
4. コンパイラは選択された言語に基づいて I18nRegistry を作成（1 回）
5. すべてのエラーはその I18nRegistry を使用してメッセージをレンダリング
```

#### 零查表オーバーヘッドの鍵

**レンダリングはユーザーのプロジェクトのコンパイル時に発生し、実行時ではない。**

```
┌─────────────────────────────────────────────────────────────────────────┐
│  段階 1: Rust で YaoXiang コンパイラをコンパイル                          │
│                                                                           │
│  JSON をコンパイラのバイナリにパック                                       │
│  目的：explain コマンドが直接 i18n データを読み取れるようにするため       │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  段階 2: YaoXiang でユーザーのプロジェクトをコンパイル（レンダリング発生）  │
│                                                                           │
│  error! マクロ呼び出し時：                                                │
│  1. yaoxiang.toml を読み取って言語設定を入手                              │
│  2. コンパイラのバイナリから対応する言語の i18n JSON をロード              │
│  3. テンプレート + パラメータ → render() → "Unknown variable: 'x'"      │
│  4. Diagnostic.message = レンダリング済み文字列                           │
│                                                                           │
│  AOT バイナリは最終文字列を直接存储、テンプレートなし、查表なし            │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  段階 3: ユーザーのプログラム実行時                                        │
│                                                                           │
│  println!("{}", diagnostic.message)                                      │
│  // 最終文字列を直接出力、任何の查表なし                                    │
└─────────────────────────────────────────────────────────────────────────┘
```

| コンポーネント | 責任 | レンダリングタイミング |
|------|------|----------|
| `I18nRegistry` | テンプレートと表示文案を提供 | ユーザーのプロジェクトをコンパイル時 |
| `DiagnosticBuilder.render()` | テンプレート + パラメータ → 最終文字列 | ユーザーのプロジェクトをコンパイル時 |
| `Diagnostic.message` | レンダリング済み文字列 | 結果を存储 |
| AOT バイナリ | 最終文字列を含む | 実行時に直接使用 |

---

### エラーメッセージ形式

エラーメッセージは以下の形式を使用：

```
error[E####]: <短い説明>
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

エラーの重大度は `DiagnosticLevel` 列挙型で管理され、エラーコード番号とは切り離されている：

```rust
pub enum DiagnosticLevel {
    Error,    // コンパイル失敗を引き起こす
    Warning,  // コンパイルには影響しないが、修正を推奨
    Note,     // 補足情報
    Help,     // 修正提案
}
```

| レベル | プレフィックス | 説明 |
|------|------|------|
| Error | `error[E####]:` | コンパイル失敗を引き起こす |
| Warning | `warning[E####]:` | コンパイルに影響しない |
| Note | `note[E####]:` | 補足情報 |
| Help | `help[E####]:` | 修正提案 |

---

### `yaoxiang explain` コマンド

#### コマンド構文

```bash
yaoxiang explain <ERROR_CODE> [OPTIONS]
```

#### オプション

| オプション | 説明 |
|------|------|
| `--lang <code>` | 言語を指定 (en-US, ja-JP, zh-CN、デフォルト en-US) |
| `--json` | JSON 形式で出力（IDE/LSP 用） |
| `--json-pretty` | フォーマットされた JSON 出力 |
| `--examples` | サンプルコードのみを表示 |
| `--help` | ヘルプ情報を表示 |

#### 使用例

```bash
# デフォルト英語
$ yaoxiang explain E1001
error[E1001]: Unknown variable: {name}
  --> <file>:<line>:<col>

Help: Did you mean to define it?

Example:
  let {name} = value;

# 日本語出力
$ yaoxiang explain E1001 --lang ja
error[E1001]: 不不明な変数: {name}
  --> <file>:<line>:<col>

ヘルプ: 定義しようとしていますか？

例:
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
  "examples": [
    "let {name} = value;"
  ],
  "language": "en-US"
}
```

---

### 後方互換性

本 RFC はゼロからエラーコードシステムを設計するため、後方互換性の問題は存在しない。

**将来の移行戦略**（今後のバージョンの参考）：

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
3. 言語優先度ロジックを追加
4. ユニットテストカバレッジ

### 段階四：IDE/LSP 統合

1. LSP サーバーに explain JSON 出力を統合
2. IDE でエラーコードリンクを表示
3. ホバーでエラー説明を表示
4. クイックフィックス提案

---

## 付録

### 完全エラーコード早見表

| 範囲 | カテゴリ |
|------|------|
| E0xxx | 字句解析と構文解析 |
| E1xxx | 型チェック |
| E2xxx | 意味解析 |
| E3xxx | コード生成 |
| E4xxx | ジェネリクスとトレイト |
| E5xxx | モジュールとインポート |
| E6xxx | 実行時エラー |
| E7xxx | I/O とシステムエラー |
| E8xxx | 内部コンパイラエラー |
| E9xxx | 予約 |

### サポートされている言語

| コード | 言語 | ステータス |
|------|------|------|
| en-US | English (US) | デフォルト |
| ja-JP | 日本語 | 計画中 |
| zh-CN | 簡体字中国語 | 計画中 |

### エラーメッセージ例 сравнение

```
# 英語 (en-US)
error[E1001]: Unknown variable: x
  --> src/main.yx:5:12
   5 |   print(x)
          ^
          help: Did you mean to define it?

# 日本語 (ja-JP)
error[E1001]: 不不明な変数: x
  --> src/main.yx:5:12
   5 |   print(x)
          ^
          ヘルプ: 定義しようとしていますか？
```

## 参考文献

- [Rust コンパイラエラーインデックス](https://doc.rust-lang.org/error_codes/error-index.html)
- [GCC エラーメッセージ形式](https://gcc.gnu.org/onlinedocs/gcc-13.1.0/gcc/Warning-Options.html)
- [Clang 診断形式](https://clang.llvm.org/diagnostics.html)
- [Language Server Protocol](https://microsoft.github.io/language-server-protocol/)
```
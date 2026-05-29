```yaml
---
title: RFC-013：错误コード仕様
---

# RFC 013: エラーコード仕様

> **ステータス**: 承認済み
> **作者**: 晨煦
> **作成日**: 2026-02-02
> **最終更新**: 2026-02-12

## 概要

本 RFC は YaoXiang コンパイラのエラーコード分類仕様を提案するものである。Rust と同様の単層番号システムを採用し、JSON リソースファイルによる多言語サポート、`yaoxiang explain` コマンドによるエラー解説機能を提供する。

## 動機

### 標準化されたエラーコードが必要な理由

1. **ユーザー体験**: ユーザーがエラーコードを見るだけで、エラーの種類と重大度を素早く判断できる
2. **ドキュメンテーションの整理**: カテゴリごとの分類により、エラーレファレンルドキュメントの作成と保守が容易になる
3. **ツール統合**: IDE/LSP がエラーコードに基づいてクイックフィックス提案やドキュメントリンクを提供できる
4. **国際化サポート**: エラーメッセージとコードを分離することで、多言語翻訳が容易になる

### 設計目標

- **シンプル**: 単層番号方式で、複雑な分類ルールを記憶する必要がない
- **ユーザーフレンドリー**: Rust のエラーメッセージ形式と同様にヘルプ信息和示例を含む
- **拡張可能**: リソースファイル駆動型で、新しいエラーや新しい言語の追加が容易
- **ツールフレンドリー**: explain コマンド + JSON 出力で、IDE/LSP 統合をサポート

---

## 提案

### コア設計：単層番号システム

4桁の数字番号を採用し、コンパイル段階でグループ化する：

```
Exxxx
││││
│││└── シーケンス番号 (000-999)
││└─── コンパイル段階 (0-9)
└───── 固定接頭辞 'E'
```

### 段階の区分

| 段階 | 範囲 | 説明 |
|------|------|------|
| **0** | E0xxx | 字句解析と構文解析 |
| **1** | E1xxx | 型チェック |
| **2** | E2xxx | 意味解析 |
| **3** | E3xxx | コード生成 |
| **4** | E4xxx | ジェネクスとトレイト |
| **5** | E5xxx | モジュールとインポート |
| **6** | E6xxx | 运行时エラー |
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
    Generic,    // E4xxx: ジェネクスとトレイト
    Module,     // E5xxx: モジュールとインポート
    Runtime,    // E6xxx: 运行时エラー
    Io,         // E7xxx: I/Oとシステムエラー
    Internal,   // E8xxx: 内部コンパイラエラー
}
```

### エラーコード定義と汎用 Builder

**コア原則**: エラーコード定義と表示文案の分離

- `ErrorCodeDefinition`: エラーコードのメタデータ（code、category、template）で、表示文案は含まない
- `i18n/*.json`: 各言語の表示文案（title、message、help）
- `DiagnosticBuilder`: 汎用ビルダーで、trait-per-error 設計に代わるもの

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

#### 各エラーコードのショート-cut メソッド

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

// 簡略化された方法
return Err(E1001::unknown_variable(&var_name)
    .at(span)
    .build(&i18n_registry));

// 手動の方法
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
| **单一 Builder** | 1つの `DiagnosticBuilder` がすべてのエラーコードに対応 |
| **型安全** | ショート-cut メソッドがパラメータの正確性を確保 |
| **自己文書化** | `E1001::unknown_variable(name)` が一目でわかる |
| **テンプレート分離** | メッセージテンプレートとコードを分離し、i18n が容易 |
| **ゼロ実行時オーバーヘッド** | コンパイル時のレンダリング、AOT バイナリにテーブル参照なし |

---

### エラーメacro の簡略化

#### error! macro（コンテキストの自動注入）

```rust
/// コンパイル時に span と i18n 設定を自動取得する macro
macro_rules! error {
    ($code:ident, $($key:ident = $value:expr),* $(,)?) => {
        $code()
            $(.$key($value))*            .at(crate::util::span::Span::current())
            .build(crate::util::diagnostic::I18nRegistry::current())
    };
}

/// 使用方法：パラメータを渡すだけで、span と i18n は自動注入
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

## 詳細な設計

### エラーコード一覧

#### E0xxx：字句解析と構文解析

| コード | エラータイプ | 説明 |
|------|----------|------|
| E0001 | Invalid character | ソースコードに不正な文字が含まれている |
| E0002 | Invalid number literal | 数字リテラルのフォーマットが不正 |
| E0003 | Unterminated string | 複数行文字列の終了引用符がない |
| E0004 | Invalid character literal | 文字リテラルが不正 |
| E0010 | Expected token | 構文解析時に特定の token を期待していた |
| E0011 | Unexpected token | 予期しない token に遭遇した |
| E0012 | Invalid syntax | 式/文の構文エラー |
| E0013 | Mismatched brackets | 丸括弧、角括弧、波括弧が一致しない |
| E0014 | Missing semicolon | 文の末尾にセミコロンがない |

#### E1xxx：型チェック

| コード | エラータイプ | 説明 |
|------|----------|------|
| E1001 | Unknown variable | 参照された変数が未定義 |
| E1002 | Type mismatch | 期待する型と実際の型が一致しない |
| E1003 | Unknown type | 参照された型が存在しない |
| E1010 | Parameter count mismatch | 関数呼び出しのパラメータ数が定義と一致しない |
| E1011 | Parameter type mismatch | パラメータの型チェックに失敗 |
| E1012 | Return type mismatch | 関数の戻り値型が不正 |
| E1013 | Function not found | 未定義の関数を呼び出そうとした |
| E1020 | Cannot infer type | コンテキストから型を推断できない |
| E1021 | Type inference conflict | 複数の制約が型 противоречие を起こす |
| E1030 | Pattern non-exhaustive | match 式がすべてのケースをカバーしていない |
| E1031 | Unreachable pattern | 決してマッチことのないパターン |
| E1040 | Operation not supported | 型がその操作をサポートしていない |
| E1041 | Index out of bounds | 配列/リストのインデックスが範囲外 |
| E1042 | Field not found | 存在しない構造体フィールドにアクセス |

#### E2xxx：意味解析

| コード | エラータイプ | 説明 |
|------|----------|------|
| E2001 | Scope error | 変数が現在のスコープにない |
| E2002 | Duplicate definition | 同一スコープ内での重複定義 |
| E2003 | Lifetime error | ライフタイム制約が満たされていない |
| E2010 | Immutable assignment | 不変変数の変更を試みた |
| E2011 | Uninitialized use | 未初期化変数の使用 |
| E2012 | Mutability conflict | 不変コンテキストで可変参照を使用 |

#### E4xxx：ジェネクスとトレイト

| コード | エラータイプ | 説明 |
|------|----------|------|
| E4001 | Generic parameter mismatch | ジェネリックパラメータの数/型が一致しない |
| E4002 | Trait bound violated | trait 制約を満たしていない |
| E4003 | Associated type error | 関連型の定義/使用エラー |
| E4004 | Duplicate trait implementation | 同一 trait の重複実装 |
| E4005 | Trait not found | 要求された trait が見つからない |
| E4006 | Sized bound violated | Sized 制約を満たしていない |

#### E5xxx：モジュールとインポート

| コード | エラータイプ | 説明 |
|------|----------|------|
| E5001 | Module not found | インポートされたモジュールが存在しない |
| E5002 | Cyclic import | モジュール間の循環依存 |
| E5003 | Symbol not exported | エクスポートされていないシンボルにアクセスしようとした |
| E5004 | Invalid module path | モジュールパスフォーマットエラー |
| E5005 | Private access | プライベートシンボルへのアクセス |

#### E6xxx：実行時エラー

| コード | エラータイプ | 説明 |
|------|----------|------|
| E6001 | Division by zero | 整数をゼロで除算 |
| E6002 | Assertion failed | assert! macro が失敗 |
| E6003 | Arithmetic overflow | 算術演算のオーバーフロー |
| E6004 | Stack overflow | スタック領域の枯渇 |
| E6005 | Heap allocation failed | メモリ割り当ての失敗 |
| E6006 | Runtime index out of bounds | 実行時のインデックス越界 |
| E6007 | Type cast failed | 型を互換性のない型にキャストしようとした |

#### E7xxx：I/O とシステムエラー

| コード | エラータイプ | 説明 |
|------|----------|------|
| E7001 | File not found | 存在しないファイルの読み込みを試みた |
| E7002 | Permission denied | ファイル権限が不十分 |
| E7003 | I/O error | 汎用 I/O エラー |
| E7004 | Network error | ネットワーク操作の失敗 |

#### E8xxx：内部コンパイラエラー

| コード | エラータイプ | 説明 |
|------|----------|------|
| E8001 | Internal compiler error | コンパイラの内部エラー |
| E8002 | Codegen error | IR/バイトコード生成の失敗 |
| E8003 | Unimplemented feature | 未実装の 功能の使用 |
| E8004 | Optimization error | コンパイラの最適化エラー |

---

### 多言語リソースファイル

#### リソースファイルのフォーマット

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
    "title": "未知の変数",
    "message": "参照された変数が未定義です",
    "template": "未知の変数：'{name}'",
    "help": "変数名が正しく入力されているか確認するか、まず定義してください",
    "example": "x = 100;",
    "error_output": "error[E1001]: 未知の変数：'x'\n  --> example.yx:1:1\n   |\n 1 | print(x)\n   | ^ 未知の変数 'x'"
  },
  "E1002": {
    "title": "型が一致しない",
    "message": "期待する型と実際の型が一致しません",
    "template": "期待する型 '{expected}'、実際の型 '{found}'",
    "help": "正しい型を使用するか、型変換を追加してください",
    "example": "x: Int = \"hello\";",
    "error_output": "error[E1002]: 型が一致しない\n  --> example.yx:1:12\n   |\n 1 | x: Int = \"hello\";\n   |            ^ 期待 'Int'、実際 'String'"
  }
}
```

#### I18nRegistry 実装

```rust
// diagnostic/codes/i18n/mod.rs

/// i18n 表示文案レジストリ（コンパイル時に JSON からロード、実行時にテーブル参照なし）
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

    /// テンプレートをレンダリング（コンパイル時に完了、実行時にゼロオーバーヘッド）
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

| プレースホルダー | 用途 | 例 |
|--------|------|------|
| `{name}` | 変数名/型名/trait 名などの識別子 | `Unknown variable: '{name}'` |
| `{expected}` | 期待する型 | `Expected type '{expected}'` |
| `{found}` | 実際/見つかった型 | `, found type '{found}'` |
| `{method}` | メソッド名 | `Method {method} is not a function` |
| `{trait}` | trait 名 | `Cannot find trait: {trait}` |
| `{path}` | モジュールパス | `Invalid path: {path}` |
| `{ty}` | 型式 | `Invalid type: {ty}` |
| `{message}` | 内部エラーメッセージ | `Internal error: {message}` |

##### 任意の key サポート

**params は定義済み以外にも任意の `key` をサポート**。呼び出し元は任意の `key` を渡せる：

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

> **注意**: すべてのエラーコードがプレースホルダーを使用するわけではありません。一部のエラーコード（E0001 など）は静的メッセージで、パラメータは不要です。

#### 言語の優先順位

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
# エラーメッセージの言語オプション：en, ja, zh, ...
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
1. プロジェクトレベルの yaoxiang.toml から language.default を読み取る
2. 設定されていなければ、ユーザーレベルの ~/.yaoxiang/yaoxiang.toml を読み取る
3. どちらも設定されていなければ、デフォルトの "en" を使用
4. コンパイラは選択された言語に基づいて I18nRegistry を生成（1回のみ）
5. すべてのエラーは해당 I18nRegistry を使用してメッセージをレンダリング
```

#### テーブル参照ゼロオーバーヘッドの鍵

**レンダリングはユーザーのプロジェクトをコンパイルする時に発生し、実行時ではない。**

```
┌─────────────────────────────────────────────────────────────────────────┐
│  段階 1: Rust が YaoXiang コンパイラをコンパイル                                      │
│                                                                           │
│  JSON をコンパイラのバイナリにパック                                               │
│  目的：explain コマンドが直接 i18n データを読み取れる                                  │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  段階 2: YaoXiang がユーザーのプロジェクトをコンパイル（レンダリングはここで発生）              │
│                                                                           │
│  error! macro 呼び出し時：                                                       │
│  1. yaoxiang.toml から言語設定を 읽음                                        │
│  2. コンパイラのバイナリから해당 言語の i18n JSON をロード                                 │
│  3. テンプレート + パラメータ → render() → "Unknown variable: 'x'"                │
│  4. Diagnostic.message =既にレンダリングされた文字列                              │
│                                                                           │
│  AOT バイナリは最終文字列を直接 저장、テンプレートなし、テーブル参照なし                │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  段階 3: ユーザープログラムの実行時                                              │
│                                                                           │
│  println!("{}", diagnostic.message)                                      │
│  // 最終文字列を直接出力、テーブル参照は一切なし                                        │
└─────────────────────────────────────────────────────────────────────────┘
```

| コンポーネント | 責任 | レンダリングタイミング |
|------|------|----------|
| `I18nRegistry` | テンプレートと表示文案を提供 | ユーザーのプロジェクトをコンパイルする時 |
| `DiagnosticBuilder.render()` | テンプレート + パラメータ → 最終文字列 | ユーザーのプロジェクトをコンパイルする時 |
| `Diagnostic.message` | 既にレンダリングされた文字列 | 最終結果を保存 |
| AOT バイナリ | 最終文字列を含む | 実行時に直接使用 |

---

### エラーメッセージフォーマット

エラーメッセージは次のフォーマットを使用する：

```
error[E####]: < короткое описание >
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

エラーの重大度は `DiagnosticLevel` 列挙型で管理され、エラーコード番号とは分離されている：

```rust
pub enum DiagnosticLevel {
    Error,    // コンパイル失敗の原因
    Warning,  // コンパイルには影響しないが、修正を推奨
    Note,     // 補足情報
    Help,     // 修正提案
}
```

| レベル | 接頭辞 | 説明 |
|------|------|------|
| Error | `error[E####]:` | コンパイル失敗の原因 |
| Warning | `warning[E####]:` | コンパイルには影響しない |
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
| `--lang <code>` | 言語を指定 (en-US, ja-JP, デフォルト en-US) |
| `--json` | JSON フォーマット出力（LSP 統合用） |
| `--json-pretty` | 整形された JSON 出力 |
| `--examples` | サンプルコードのみ表示 |
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
error[E1001]: 未知の変数: {name}
  --> <file>:<line>:<col>

ヘルプ: これを定義しようとしていましたか？

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

#### JSON 出力フォーマット

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

本 RFC はゼロからエラーコードシステムを設計するため、後方互換性の問題はない。

**将来の移行戦略**（今後のバージョンの参考）：

1. 古いエラーコードから新しいエラーコードへのマッピングを維持
2. 移行期間中は新旧両方のコードを表示
3. 廃止タイムラインを提供

---

## 実施戦略

### 段階一：错误コードインフラストラクチャ

1. `src/diagnostics/` ディレクトリ構造を作成
2. `ErrorCode` 列挙型を実装
3. `Diagnostic` と `DiagnosticLevel` を実装
4. リソースファイルディレクトリ、サンプル JSON を作成

### 段階二：explain コマンド

1. `yaoxiang explain` CLI コマンドを実装
2. `--lang` と `--json` オプションをサポート
3. リソースファイルのロードを統合
4. パラメータテンプレートのレンダリングを実装

### 段階三：コンパイル時統合

1. すべてのエラー報告ポイントを更新して新システムを使用
2. メッセージテンプレートパラメータ注入を実装
3. 言語優先順位ロジックを追加
4. ユニットテストカバレッジ

### 段階四：IDE/LSP 統合

1. LSP サーバーが explain JSON 出力を統合
2. IDE にエラーコードリンクを表示
3. ホバーでエラー解説を表示
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
| E4xxx | ジェネクスとトレイト |
| E5xxx | モジュールとインポート |
| E6xxx | 実行時エラー |
| E7xxx | I/O とシステムエラー |
| E8xxx | 内部コンパイラエラー |
| E9xxx | 予約 |

### サポート言語

| コード | 言語 | ステータス |
|------|------|------|
| en-US | English (US) | デフォルト |
| ja-JP | 日本語 | 予定 |
| zh-CN | 简体中文 | 予定 |

### エラーメッセージ例比較

```
# 英語 (en-US)
error[E1001]: Unknown variable: x
  --> src/main.yx:5:12
   5 |   print(x)
          ^
          help: Did you mean to define it?

# 日本語 (ja-JP)
error[E1001]: 未知の変数: x
  --> src/main.yx:5:12
   5 |   print(x)
          ^
          ヘルプ: これを定義しようとしていましたか？
```

## 参考文献

- [Rust コンパイラエラーインデックス](https://doc.rust-lang.org/error_codes/error-index.html)
- [GCC エラーメッセージフォーマット](https://gcc.gnu.org/onlinedocs/gcc-13.1.0/gcc/Warning-Options.html)
- [Clang 診断フォーマット](https://clang.llvm.org/diagnostics.html)
- [Language Server Protocol](https://microsoft.github.io/language-server-protocol/)
```
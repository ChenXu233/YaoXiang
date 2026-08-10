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

本RFCはYaoXiangコンパイラのエラーコード分類仕様を提案する。Rustに類似した単層番号システムを採用し、JSONリソースファイルと組み合わせることで多言語サポートを実現し、`yaoxiang explain`コマンドによりエラー説明機能を提供する。

## 動機

### なぜ標準化されたエラーコードが必要なのか？

1. **ユーザーエクスペリエンス**：ユーザーはエラーコードを見ることでエラーの種類と重大度を迅速に判断できる
2. **ドキュメント構成**：カテゴリ別に分類することでエラー参考ドキュメントの作成と保守が容易になる
3. **ツール統合**：IDE/LSPはエラーコードに基づいて迅速な修正提案とドキュメントリンクを提供できる
4. **国際化サポート**：エラーメッセージとコードを分離することで多言語への翻訳が容易になる

### 設計目標

- **簡潔**：単層番号方式により、ユーザーは複雑な分類ルールを覚える必要がない
- **親しみやすい**：Rustに類似したエラーメッセージ形式、ヘルプ情報と例を含む
- **拡張可能**：リソースファイル駆動で、新しいエラーと新しい言語の追加が容易
- **ツールフレンドリー**：explainコマンド + JSON出力で、IDE/LSP統合をサポート

---

## 提案

### 中核設計：単層番号システム

4桁の数字番号を採用し、コンパイル段階でグループ化する：

```
Exxxx
││││
│││└── 序号 (000-999)
││└─── 编译阶段 (0-9)
└───── 固定前缀 'E'
```

### 段階区分

| 段階  | 範囲  | 説明                   |
| ----- | ----- | ---------------------- |
| **0** | E0xxx | 字句解析と構文解析     |
| **1** | E1xxx | 型検査                 |
| **2** | E2xxx | 意味解析               |
| **3** | E3xxx | コード生成             |
| **4** | E4xxx | ジェネリクスとトレイト |
| **5** | E5xxx | モジュールとインポート |
| **6** | E6xxx | 実行時エラー           |
| **7** | E7xxx | I/Oとシステムエラー    |
| **8** | E8xxx | 内部コンパイラエラー   |
| **9** | E9xxx | 予約/実験的            |

### エラーカテゴリ列挙

```rust
/// 错误类别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    Lexer,      // E0xxx: 词法和语法分析
    Parser,     // E0xxx: Parser errors
    TypeCheck,  // E1xxx: 类型检查
    Semantic,   // E2xxx: 语义分析
    Generic,    // E4xxx: 泛型与特质
    Module,     // E5xxx: 模块与导入
    Runtime,    // E6xxx: 运行时错误
    Io,         // E7xxx: I/O与系统错误
    Internal,   // E8xxx: 内部编译器错误
}
```

### エラーコード定義と汎用Builder

**中核原則**：エラーコード定義と表示テキストを分離する

- `ErrorCodeDefinition`：エラーコードのメタデータ（code、category、template）、表示テキストを含まない
- `locales/*.json`：各言語の表示テキスト（title、message、help、エラーコードはネストされたオブジェクト）
- `DiagnosticBuilder`：汎用ビルダー、trait-per-error設計を置き換える

#### エラーコード定義

```rust
// diagnostic/codes/mod.rs

use crate::util::span::Span;
use crate::util::diagnostic::{Diagnostic, Severity};

/// 错误码定义（仅元数据，展示文案在 i18n 文件）
#[derive(Debug, Clone, Copy)]
pub struct ErrorCodeDefinition {
    pub code: &'static str,
    pub category: ErrorCategory,
    pub message_template: &'static str,  // 消息模板，支持 {param} 占位符
}

/// 通用诊断构建器
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

    /// 添加模板参数
    pub fn param(mut self, key: &'static str, value: impl Into<String>) -> Self {
        self.params.push((key, value.into()));
        self
    }

    /// 设置位置
    pub fn at(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    /// 构建 Diagnostic（模板渲染在编译期完成）
    pub fn build(&self, i18n: &I18nRegistry) -> Diagnostic {
        // 检查模板中所有 {key} 都有对应参数
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
    /// E1001 未知变量
    pub fn unknown_variable(name: &str) -> DiagnosticBuilder {
        let def = Self::find("E1001").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template)
            .param("name", name)
    }

    /// E1002 类型不匹配
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

// 简化方式
return Err(E1001::unknown_variable(&var_name)
    .at(span)
    .build(&i18n_registry));

// 手动方式
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
    // ... 其他错误码
];
```

#### 設計上の利点

| 特性                         | 説明                                                       |
| ---------------------------- | ---------------------------------------------------------- |
| **単一Builder**              | 1つの`DiagnosticBuilder`ですべてのエラーコードに対応       |
| **型安全**                   | ショートカットメソッドがパラメータの正確性を保証           |
| **自己文書化**               | `E1001::unknown_variable(name)`で一目瞭然                  |
| **テンプレート分離**         | メッセージテンプレートとコードが分離されており、i18nが容易 |
| **実行時オーバーヘッドゼロ** | コンパイル時にレンダリング、AOTバイナリにテーブル参照なし  |

---

### エラーマクロの簡素化

#### error!マクロ（コンテキスト自動注入）

```rust
/// 编译期自动获取 span 和 i18n 配置的宏
macro_rules! error {
    ($code:ident, $($key:ident = $value:expr),* $(,)?) => {
        $code()
            $(.$key($value))*
            .at(crate::util::span::Span::current())
            .build(crate::util::diagnostic::I18nRegistry::current())
    };
}

/// 使用：只需传参数，span 和 i18n 自动注入
return Err(error!(E1001, name = var_name));
return Err(error!(E1002, expected = "bool", found = cond_ty));
```

#### 手動でBuilderを使用

```rust
// 需要手动控制时
E1001::unknown_variable(&var_name)
    .at(my_span)           // 自定义 span
    .build(&custom_i18n)   // 自定义 i18n
```

---

## 詳細設計

### エラーコード一覧

#### E0xxx：字句解析と構文解析

| コード | エラータイプ              | 説明                                   |
| ------ | ------------------------- | -------------------------------------- |
| E0001  | Invalid character         | ソースコードに不正な文字が含まれている |
| E0002  | Invalid number literal    | 数値リテラルの形式が正しくない         |
| E0003  | Unterminated string       | 複数行文字列の終了引用符が欠落         |
| E0004  | Invalid character literal | 文字リテラルが正しくない               |
| E0010  | Expected token            | 構文解析中に特定のトークンが必要       |
| E0011  | Unexpected token          | 予期しないトークンに遭遇した           |
| E0012  | Invalid syntax            | 式/文の構文エラー                      |
| E0013  | Mismatched brackets       | 丸括弧、角括弧、波括弧が一致しない     |
| E0014  | Missing semicolon         | 文末にセミコロンが欠落                 |

#### E1xxx：型検査

| コード | エラータイプ             | 説明                                     |
| ------ | ------------------------ | ---------------------------------------- |
| E1001  | Unknown variable         | 参照された変数が未定義                   |
| E1002  | Type mismatch            | 期待される型と実際の型が一致しない       |
| E1003  | Unknown type             | 参照された型が存在しない                 |
| E1010  | Parameter count mismatch | 関数呼び出しの引数の数と定義が一致しない |
| E1011  | Parameter type mismatch  | 引数の型検査に失敗                       |
| E1012  | Return type mismatch     | 関数の戻り値の型エラー                   |
| E1013  | Function not found       | 未定義の関数を呼び出した                 |
| E1020  | Cannot infer type        | コンテキストから型を推論できない         |
| E1021  | Type inference conflict  | 複数の制約により型が矛盾する             |
| E1030  | Pattern non-exhaustive   | match式がすべての場合を網羅していない    |
| E1031  | Unreachable pattern      | 決してマッチしないパターン               |
| E1040  | Operation not supported  | 型がその操作をサポートしていない         |
| E1041  | Index out of bounds      | 配列/リストのインデックスが範囲外        |
| E1042  | Field not found          | 存在しない構造体フィールドにアクセス     |

#### E2xxx：意味解析

| コード | エラータイプ         | 説明                             |
| ------ | -------------------- | -------------------------------- |
| E2001  | Scope error          | 変数が現在のスコープに存在しない |
| E2002  | Duplicate definition | 同一スコープ内での重複定義       |
| E2003  | Lifetime error       | ライフタイム制約が満たされない   |
| E2010  | Immutable assignment | 不変変数の変更を試みた           |
| E2011  | Uninitialized use    | 未初期化の変数を使用             |
| E2012  | Mutability conflict  | 不変コンテキストで可変参照を使用 |

#### E4xxx：ジェネリクスとトレイト

| コード | エラータイプ                   | 説明                                      |
| ------ | ------------------------------ | ----------------------------------------- |
| E4001  | Generic parameter mismatch     | ジェネリクスパラメータの数/型が一致しない |
| E4002  | Trait bound violated           | トレイト制約が満たされない                |
| E4003  | Associated type error          | 関連型の定義/使用エラー                   |
| E4004  | Duplicate trait implementation | 同一トレイトの重複実装                    |
| E4005  | Trait not found                | 要求されたトレイトが見つからない          |
| E4006  | Sized bound violated           | Sized制約が満たされない                   |

#### E5xxx：モジュールとインポート

| コード | エラータイプ        | 説明                                           |
| ------ | ------------------- | ---------------------------------------------- |
| E5001  | Module not found    | インポートされたモジュールが存在しない         |
| E5002  | Cyclic import       | モジュール間の循環依存                         |
| E5003  | Symbol not exported | 未エクスポートのシンボルにアクセスしようとした |
| E5004  | Invalid module path | モジュールパスの形式エラー                     |
| E5005  | Private access      | プライベートシンボルへのアクセス               |

#### E6xxx：実行時エラー

| コード | エラータイプ                | 説明                                 |
| ------ | --------------------------- | ------------------------------------ |
| E6001  | Division by zero            | 整数のゼロ除算                       |
| E6002  | ~~Assertion failed~~        | ~~予約（概念削除、#280）~~           |
| E6003  | Runtime index out of bounds | 実行時のインデックス範囲外           |
| E6004  | Stack overflow              | スタック領域の枯渇                   |
| E6005  | Assertion failed            | アサーション失敗                     |
| E6006  | Function not found          | 実行時の関数未検出                   |
| E6007  | Runtime error (generic)     | 汎用実行時エラー                     |

#### E7xxx：I/Oとシステムエラー

| コード | エラータイプ      | 説明                                 |
| ------ | ----------------- | ------------------------------------ |
| E7001  | File not found    | 存在しないファイルの読み取りを試みた |
| E7002  | Permission denied | ファイル権限不足                     |
| E7003  | I/O error         | 汎用I/Oエラー                        |
| E7004  | Network error     | ネットワーク操作の失敗               |

#### E8xxx：内部コンパイラエラー

| コード | エラータイプ            | 説明                      |
| ------ | ----------------------- | ------------------------- |
| E8001  | Internal compiler error | コンパイラの内部エラー    |
| E8002  | Codegen error           | IR/バイトコード生成の失敗 |
| E8003  | Unimplemented feature   | 未実装の機能を使用        |
| E8004  | Optimization error      | コンパイラの最適化エラー  |

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

#### I18nRegistry実装

```rust
// locales/*.json（错误码对象）

/// i18n 展示文案注册表（编译期从 JSON 加载，运行时零查表）
pub struct I18nRegistry {
    /// 标题
    titles: HashMap<&'static str, &'static str>,
    /// 描述
    messages: HashMap<&'static str, &'static str>,
    /// 帮助信息
    helps: HashMap<&'static str, &'static str>,
    /// 示例代码
    examples: HashMap<&'static str, &'static str>,
    /// 错误输出示例
    error_outputs: HashMap<&'static str, &'static str>,
}

/// 单个错误码信息
#[derive(Clone, Copy)]
pub struct ErrorInfo<'a> {
    pub title: &'a str,
    pub message: &'a str,
    pub help: &'a str,
    pub example: Option<&'a str>,
    pub error_output: Option<&'a str>,
}

impl I18nRegistry {
    /// 根据语言代码获取注册表
    pub fn new(lang: &str) -> Self {
        match lang {
            "zh" => Self::zh(),
            _ => Self::en(),
        }
    }

    /// 获取错误信息
    pub fn get_info(&self, code: &str) -> Option<ErrorInfo<'_>> {
        Some(ErrorInfo {
            title: self.titles.get(code)?,
            message: self.messages.get(code)?,
            help: self.helps.get(code)?,
            example: self.examples.get(code).copied(),
            error_output: self.error_outputs.get(code).copied(),
        })
    }

    /// 渲染模板（编译期完成，运行时零开销）
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

##### 定義済みプレースホルダ（一般的）

| プレースホルダ | 用途                               | 例                                  |
| -------------- | ---------------------------------- | ----------------------------------- |
| `{name}`       | 変数名/型名/トレイト名などの識別子 | `Unknown variable: '{name}'`        |
| `{expected}`   | 期待される型                       | `Expected type '{expected}'`        |
| `{found}`      | 実際/見つかった型                  | `, found type '{found}'`            |
| `{method}`     | メソッド名                         | `Method {method} is not a function` |
| `{trait}`      | トレイト名                         | `Cannot find trait: {trait}`        |
| `{path}`       | モジュールパス                     | `Invalid path: {path}`              |
| `{ty}`         | 型式                               | `Invalid type: {ty}`                |
| `{message}`    | 内部エラーメッセージ               | `Internal error: {message}`         |

##### 任意のkeyのサポート

**paramsは事前定義に限定されず、任意のkeyをサポートします。**
呼び出し側は任意の`key`を渡すことができます：

```rust
// 使用任意 key
E1001::unknown_variable(&var_name)
    .param("location", "global scope")
    .param("hint", "try declaring it first")
    .at(span)
    .build(&i18n);

// 模板定义
"Unknown variable: '{name}' at {location}. {hint}"
```

> **注意**：すべてのエラーコードがプレースホルダを使用するわけではありません。一部のエラーコード（例：E0001）は静的メッセージで、パラメータは不要です。

#### 言語の優先順位

```
1. yaoxiang.toml [language.default]
2. ~/.yaoxiang/yaoxiang.toml [language.default]
3. 默认值: en
```

### yaoxiang.toml設定

#### プロジェクトレベル設定

```toml
# yaoxiang.toml
[project]
name = "my-project"
version = "0.1.0"

[language]
# 错误消息语言，可选：en, zh, ja, ...
default = "zh"
```

#### ユーザーレベル設定

```toml
# ~/.yaoxiang/yaoxiang.toml
[language]
default = "zh"
```

#### コンパイル時の言語選択

```
1. 读取项目级 yaoxiang.toml 的 language.default
2. 若未配置，读取用户级 ~/.yaoxiang/yaoxiang.toml
3. 若都未配置，默认使用 "en"
4. 编译器根据选择的语言创建 I18nRegistry（一次）
5. 所有错误使用该 I18nRegistry 渲染消息
```

#### ゼロテーブル参照オーバーヘッドの鍵

**レンダリングはユーザープロジェクトのコンパイル時に発生し、実行時ではありません。**

```
┌─────────────────────────────────────────────────────────────────────────┐
│  阶段 1: Rust 编译 YaoXiang 编译器                                      │
│                                                                           │
│  JSON 打包进编译器二进制                                                 │
│  目的：explain 指令能直接读取 i18n 数据                                  │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  阶段 2: YaoXiang 编译用户项目（渲染发生在这里）                          │
│                                                                           │
│  error! 宏调用时：                                                       │
│  1. 读取 yaoxiang.toml 获取语言偏好                                      │
│  2. 从编译器二进制加载对应语言的 i18n JSON                                │
│  3. 模板 + 参数 → render() → "Unknown variable: 'x'"                    │
│  4. Diagnostic.message = 已渲染的字符串                                   │
│                                                                           │
│  AOT 二进制直接存储最终字符串，无模板，无查表                            │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  阶段 3: 用户程序运行时                                                  │
│                                                                           │
│  println!("{}", diagnostic.message)                                      │
│  // 直接输出最终字符串，无任何查表                                        │
└─────────────────────────────────────────────────────────────────────────┘
```

| コンポーネント               | 責務                                   | レンダリングタイミング             |
| ---------------------------- | -------------------------------------- | ---------------------------------- |
| `I18nRegistry`               | テンプレートと表示テキストを提供       | ユーザープロジェクトのコンパイル時 |
| `DiagnosticBuilder.render()` | テンプレート + パラメータ → 最終文字列 | ユーザープロジェクトのコンパイル時 |
| `Diagnostic.message`         | レンダリング済み文字列                 | 最終結果を保存                     |
| AOTバイナリ                  | 最終文字列を含む                       | 実行時に直接使用                   |

---

### エラーメッセージ形式

エラーメッセージは以下の形式を採用する：

```
error[E####]: <简短描述>
  --> <文件>:<行>:<列>
   <行> | <代码片段>
          ^^^<高亮>
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

エラーの重大度は`DiagnosticLevel`列挙で管理され、エラーコード番号とは分離されている：

```rust
pub enum DiagnosticLevel {
    Error,    // 导致编译失败
    Warning,  // 不影响编译，但建议修复
    Note,     // 补充信息
    Help,     // 修复建议
}
```

| レベル  | 接頭辞            | 説明                       |
| ------- | ----------------- | -------------------------- |
| Error   | `error[E####]:`   | コンパイル失敗を引き起こす |
| Warning | `warning[E####]:` | コンパイルには影響しない   |
| Note    | `note[E####]:`    | 補足情報                   |
| Help    | `help[E####]:`    | 修正提案                   |

---

### `yaoxiang explain`コマンド

#### コマンド構文

```bash
yaoxiang explain <ERROR_CODE> [OPTIONS]
```

#### オプション

| オプション      | 説明                                        |
| --------------- | ------------------------------------------- |
| `--lang <code>` | 言語を指定 (en-US, zh-CN、デフォルト en-US) |
| `--json`        | JSON形式出力（IDE/LSP用）                   |
| `--json-pretty` | フォーマット済みJSON出力                    |
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

# 中文出力
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

#### JSON出力形式

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

本RFCはエラーコードシステムをゼロから設計するため、後方互換性の問題はありません。

**将来の移行戦略**（後続バージョン参考用）：

1. 旧エラーコードから新エラーコードへのマッピングを維持する
2. 移行期間中は新旧コードを同時に表示する
3. 廃止スケジュールを提供する

---

## 実装戦略

### フェーズ1：エラーコードインフラストラクチャ

1. `src/diagnostics/`ディレクトリ構造を作成する
2. `ErrorCode`列挙を実装する
3. `Diagnostic`と`DiagnosticLevel`を実装する
4. リソースファイルディレクトリとサンプルJSONを作成する

### フェーズ2：explainコマンド

1. `yaoxiang explain` CLIコマンドを実装する
2. `--lang`と`--json`オプションをサポートする
3. リソースファイルの読み込みを統合する
4. パラメータテンプレートのレンダリングを実装する

### フェーズ3：コンパイル時統合

1. すべてのエラー報告ポイントを新システムを使用するように更新する
2. メッセージテンプレートパラメータ注入を実装する
3. 言語優先順位ロジックを追加する
4. ユニットテストカバレッジ

### フェーズ4：IDE/LSP統合

1. LSPサーバーがexplain JSON出力を統合する
2. IDEにエラーコードリンクを表示する
3. ホバー時にエラー説明を表示する
4. 迅速な修正提案

---

## 付録

### 完全エラーコード早見表

| 範囲  | カテゴリ               |
| ----- | ---------------------- |
| E0xxx | 字句解析と構文解析     |
| E1xxx | 型検査                 |
| E2xxx | 意味解析               |
| E3xxx | コード生成             |
| E4xxx | ジェネリクスとトレイト |
| E5xxx | モジュールとインポート |
| E6xxx | 実行時エラー           |
| E7xxx | I/Oとシステムエラー    |
| E8xxx | 内部コンパイラエラー   |
| E9xxx | 予約                   |

### サポートされる言語

| コード | 言語         | ステータス |
| ------ | ------------ | ---------- |
| en-US  | English (US) | デフォルト |
| zh-CN  | 简体中文     | 計画中     |

### エラーメッセージ例の比較

```
# 英文 (en-US)
error[E1001]: Unknown variable: x
  --> src/main.yx:5:12
   5 |   print(x)
          ^
          help: Did you mean to define it?

# 中文 (zh-CN)
error[E1001]: 未知变量: x
  --> src/main.yx:5:12
   5 |   print(x)
          ^
          帮助: 你是否想要定义它？
```

## 参考文献

- [Rustコンパイラエラー索引](https://doc.rust-lang.org/error_codes/error-index.html)
- [GCCエラーメッセージ形式](https://gcc.gnu.org/onlinedocs/gcc-13.1.0/gcc/Warning-Options.html)
- [Clang診断形式](https://clang.llvm.org/diagnostics.html)
- [Language Server Protocol](https://microsoft.github.io/language-server-protocol/)

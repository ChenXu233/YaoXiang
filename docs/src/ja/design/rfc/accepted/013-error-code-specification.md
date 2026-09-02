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

本 RFC は YaoXiang コンパイラのエラーコード分類仕様を提案するものである。Rust ライクな単層番号システムを採用し、JSON リソースファイルによる多言語サポートを実現し、`yaoxiang explain`
コマンドでエラー説明機能を提供する。

## 動機

### なぜ標準化されたエラーコードが必要か？

1. **ユーザー体験**：ユーザーはエラーコードを見ることで、エラーの種類と重大度を迅速に判断できる
2. **ドキュメント構成**：カテゴリ別に分類することで、エラー参考ドキュメントの作成と保守が容易になる
3. **ツール統合**：IDE/LSP はエラーコードに基づいてクイックフィックス提案やドキュメントリンクを提供できる
4. **国際化サポート**：エラーメッセージとコードを分離することで、多言語翻訳が容易になる

### 設計目標

- **簡潔さ**：単層番号により、ユーザーは複雑な分類ルールを記憶する必要がない
- **親しみやすさ**：Rust ライクなエラーメッセージ形式とヘルプ情報・サンプルを提供
- **拡張性**：リソースファイル駆動により、新しいエラーや新しい言語の追加が容易
- **ツール親和性**：explain コマンド + JSON 出力により、IDE/LSP 統合をサポート

---

## 提案

### 中核設計：単層番号システム

四位数字番号を採用し、コンパイルフェーズごとにグループ化する：

```
Exxxx
││││
│││└── 番号 (000-999)
││└─── コンパイルフェーズ (0-9)
└───── 固定プレフィックス 'E'
```

### フェーズ区分

| フェーズ | 範囲  | 説明                   |
| -------- | ----- | ---------------------- |
| **0**    | E0xxx | 語彙解析と構文解析     |
| **1**    | E1xxx | 型検査                 |
| **2**    | E2xxx | 意味解析               |
| **3**    | E3xxx | コード生成             |
| **4**    | E4xxx | ジェネリクスとtrait    |
| **5**    | E5xxx | モジュールとインポート |
| **6**    | E6xxx | ランタイムエラー       |
| **7**    | E7xxx | I/O とシステムエラー   |
| **8**    | E8xxx | 内部コンパイラエラー   |
| **9**    | E9xxx | 予約/実験的            |

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

### エラーコード定義と汎用 Builder

**中核原則**：エラーコード定義と表示テキストの分離

- `ErrorCodeDefinition`：エラーコードのメタデータ（code、category、template）。表示テキストは含まない
- `locales/*.json`：各言語の表示テキスト（title、message、help、エラーコードはネストオブジェクト）
- `DiagnosticBuilder`：汎用ビルダー。trait-per-error 設計を置き換える

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

| 特性                             | 説明                                                                             |
| -------------------------------- | -------------------------------------------------------------------------------- |
| **単一 Builder**                 | 一つの `DiagnosticBuilder` ですべてのエラーコードに対応する                      |
| **型安全**                       | ショートカットメソッドがパラメータの正確性を保証する                             |
| **セルフドキュメンテーション**   | `E1001::unknown_variable(name)` を見れば自明                                     |
| **テンプレート分離**             | メッセージテンプレートとコードが分離されており、i18n が容易                      |
| **ゼロランタイムオーバーヘッド** | コンパイル時にレンダリングされ、AOT バイナリにはテーブルルックアップが存在しない |

---

### エラーマクロの簡素化

#### error! マクロ（コンテキスト自動注入）

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

#### Builder の手動使用

```rust
// 需要手动控制时
E1001::unknown_variable(&var_name)
    .at(my_span)           // 自定义 span
    .build(&custom_i18n)   // 自定义 i18n
```

---

## 詳細設計

### エラーコード一覧

#### E0xxx：語彙解析と構文解析

| コード | エラータイプ              | 説明                                   |
| ------ | ------------------------- | -------------------------------------- |
| E0001  | Invalid character         | ソースコードに不正な文字が含まれている |
| E0002  | Invalid number literal    | 数字リテラルの形式が正しくない         |
| E0003  | Unterminated string       | 複数行文字列の終了引用符が欠落している |
| E0004  | Invalid character literal | 文字リテラルが正しくない               |
| E0010  | Expected token            | 構文解析時に特定のトークンが必要       |
| E0011  | Unexpected token          | 予期しないトークンに遭遇した           |
| E0012  | Invalid syntax            | 式/文の構文が誤っている                |
| E0013  | Mismatched brackets       | 丸括弧、角括弧、波括弧が一致しない     |
| E0014  | Missing semicolon         | 文末にセミコロンが欠落している         |
| E0016  | Expected expression       | 式が必要                               |
| E0018  | Keyword as name           | キーワードを名前として使用できない     |

#### E1xxx：型検査

| コード | エラータイプ                                           | 説明                                              |
| ------ | ------------------------------------------------------ | ------------------------------------------------- |
| E1001  | Unknown variable                                       | 参照された変数が未定義                            |
| E1002  | Type mismatch                                          | 期待される型と実際の型が一致しない                |
| E1003  | Unknown type                                           | 参照された型が存在しない                          |
| E1010  | Parameter count mismatch                               | 関数呼び出しの引数の数が定義と一致しない          |
| E1011  | Parameter type mismatch                                | 引数の型チェックに失敗                            |
| E1012  | Return type mismatch                                   | 関数の戻り値の型が誤り                            |
| E1013  | Function not found                                     | 未定義の関数を呼び出した                          |
| E1020  | Cannot infer type                                      | コンテキストから型を推論できない                  |
| E1021  | Type inference conflict                                | 複数の制約により型が矛盾する                      |
| E1030  | Pattern non-exhaustive                                 | match 式がすべてのパターンを網羅していない        |
| E1031  | Unreachable pattern                                    | 決してマッチしないパターン                        |
| E1040  | Operation not supported                                | 型がその操作をサポートしていない                  |
| E1041  | Index out of bounds                                    | 配列/リストのインデックスが範囲外                 |
| E1042  | Field not found                                        | 存在しない構造体のフィールドにアクセスした        |
| E1050  | Boolean operand required                               | ブールオペランドが必要                            |
| E1051  | Logical NOT requires boolean operand                   | 論理 NOT はブールオペランドを要求する             |
| E1052  | Invalid dereference                                    | 無効なデリファレンス                              |
| E1053  | Non-struct field access                                | 非構造体のフィールドにアクセス                    |
| E1054  | Conditional type mismatch                              | 条件型が一致しない                                |
| E1055  | Constraint in non-generic context                      | 非ジェネリックコンテキストに制約が出現            |
| E1060  | Type parameter count mismatch                          | 型引数の数が一致しない                            |
| E1061  | Cannot instantiate generic                             | ジェネリックをインスタンス化できない              |
| E1062  | Const generic constraint failed                        | const ジェネリック制約の失敗                      |
| E1064  | Invalid binding position                               | バインディング位置のインデックスが無効（RFC-004） |
| E1071  | Type definitions are only allowed at module level      | 型定義はモジュールレベルでのみ許可される          |
| E1081  | `?` can only be used within functions returning Result | `?` は Result を返す関数内でのみ使用可能          |
| E1082  | `?` can only be used with Result expressions           | `?` は Result 式にのみ使用可能                    |
| E1083  | Error type mismatch for `?`                            | `?` のエラー型が一致しない                        |
| E1090  | Type universe easter egg                               | Type: Type = Type イースターエッグ（注記レベル）  |
| E1091  | Invalid generic meta type                              | 無効なジェネリックメタ型                          |
| E1092  | Invalid refinement type argument form                  | 精化型の実引数の形式が不正                        |
| E1093  | Refinement argument count mismatch                     | 精化実引数の数が一致しない                        |
| E1094  | Unused compile-time value parameter                    | 未使用のコンパイル時値パラメータ                  |
| E1095  | Unknown interface                                      | 未知のインターフェース                            |
| E1096  | Interface arity mismatch                               | インターフェースのパラメータ数が一致しない        |
| E1097  | Interface member name conflict                         | インターフェースメンバーの名前が衝突              |
| E1098  | Interface method not implemented                       | インターフェースメソッドが未実装                  |
| E1099  | Interface method signature mismatch                    | インターフェースメソッドのシグネチャが一致しない  |
| E1100  | Duplicate interface method implementation              | インターフェースメソッドの実装が重複              |
| E1101  | Type does not implement interface                      | 型がインターフェースを実装していない              |
| E1102  | Loop control statement outside of a loop               | ループ制御文がループ外にある                      |

#### E2xxx：意味解析

| コード | エラータイプ                      | 説明                                               |
| ------ | --------------------------------- | -------------------------------------------------- |
| E2001  | Scope error                       | 変数が現在のスコープにない                         |
| E2002  | Duplicate definition              | 同一スコープ内での重複定義                         |
| E2003  | Lifetime error                    | ライフタイム制約が満たされない                     |
| E2010  | Immutable assignment              | イミュータブル変数への代入を試みた                 |
| E2011  | Uninitialized use                 | 未初期化の変数を使用                               |
| E2012  | Mutability conflict               | イミュータブルコンテキストでミュータブル参照を使用 |
| E2013  | Variable shadowing                | 変数のシャドウイング                               |
| E2014  | Use of moved value                | ムーブ済みの値を使用                               |
| E2016  | Immutable assignment              | イミュータブル代入                                 |
| E2018  | Mutable/immutable borrow conflict | ミュータブル/イミュータブル借用の衝突              |
| E2019  | Double free                       | 二重解放                                           |
| E2020  | Use after free                    | 解放後使用                                         |
| E2027  | Unsafe dereference                | unsafe デリファレンス                              |
| E2090  | Invalid signature                 | 関数シグネチャの解析エラー                         |
| E2091  | Unknown type in signature         | シグネチャに未知の型が出現                         |
| E2092  | Missing arrow in signature        | シグネチャに戻り値の矢印が欠落                     |
| E2093  | Duplicate parameter name          | 引数名の重複                                       |
| E2094  | Generic parameter shadowing       | ジェネリックパラメータのシャドウイング             |
| E2095  | Parameter name shadows generic    | 引数名がジェネリックをシャドウイング               |

#### E3xxx：コード生成

| コード | エラータイプ                           | 説明                                                         |
| ------ | -------------------------------------- | ------------------------------------------------------------ |
| E3004  | Unsupported iterator                   | サポートされていないイテレータ                               |
| E3005  | IR generation error                    | IR 生成の内部エラー                                          |
| E3006  | Unresolved variable                    | IR 生成フェーズで変数が未解決                                |
| E3007  | Top-level initializer must be constant | トップレベルバインディングの初期化子は定数でなければならない |
| E3014  | Register overflow                      | レジスタオーバーフロー                                       |
| E3017  | Invalid operand (code generation)      | 無効なオペランド（コード生成）                               |

#### E4xxx：ジェネリクスとtrait

| コード | エラータイプ                            | 説明                                      |
| ------ | --------------------------------------- | ----------------------------------------- |
| E4001  | Generic parameter mismatch              | ジェネリックパラメータの数/型が一致しない |
| E4002  | Trait bound violated                    | trait 制約が満たされない                  |
| E4003  | Associated type error                   | 関連型の定義/使用エラー                   |
| E4004  | Duplicate trait implementation          | 同じ trait の重複実装                     |
| E4005  | Trait not found                         | 要求された trait が見つからない           |
| E4006  | Sized bound violated                    | Sized 制約が満たされない（予約、未実装）  |
| E4010  | Division by zero in constant expression | 定数式でのゼロ除算                        |
| E4011  | Constant overflow                       | 定数オーバーフロー                        |
| E4012  | Constant recursion too deep             | 定数再帰が深すぎる                        |
| E4014  | Constant evaluation failed              | 定数評価失敗                              |
| E4018  | Refinement predicate violation          | 精化述語の違反                            |
| E4019  | Type equality does not hold             | 型の等価性が成立しない                    |
| E4020  | Proof function required                 | 制約検証のための証明関数が必要            |

#### E5xxx：モジュールとインポート

| コード | エラータイプ          | 説明                                         |
| ------ | --------------------- | -------------------------------------------- |
| E5001  | Module not found      | インポートされたモジュールが存在しない       |
| E5002  | Cyclic import         | モジュール間の循環依存                       |
| E5003  | Symbol not exported   | 未エクスポートのシンボルにアクセス           |
| E5004  | Invalid module path   | モジュールパスの形式が誤り                   |
| E5005  | Private access        | プライベートシンボルにアクセス               |
| E5006  | Duplicate import      | 重複インポート                               |
| E5007  | Module export listing | モジュールエクスポート一覧（付随ヒント情報） |

#### E6xxx：ランタイムエラー

| コード | エラータイプ                | 説明                                      |
| ------ | --------------------------- | ----------------------------------------- |
| E6001  | Division by zero            | 整数除算のゼロ除算                        |
| E6002  | ~~Assertion failed~~        | ~~予約ビット（言語概念なし、削除済み）~~  |
| E6003  | Runtime index out of bounds | ランタイムインデックス範囲外（#280 接続） |
| E6004  | Stack overflow              | スタック領域の枯渇                        |
| E6005  | Assertion failed            | assert 失敗（#280 接続）                  |
| E6006  | Function not found          | ランタイムで関数未発見                    |
| E6007  | Runtime error (generic)     | 汎用ランタイムエラー                      |
| E6008  | Key not found               | Dict のキー欠如（#299 §4）                |

> **#280 改訂（2026-08-09）**：コード表は元々 Rust セマンティクス草案（Assertion failed/Arithmetic
> overflow/Heap allocation failed/Type cast
> failed）に基づいて定義されており、実装の実際の要件と一致していなかった。YaoXiang にはヌルポインタ/ヒープ割り当て失敗/型変換の概念がなく（値セマンティクス +
> Rust メモリ安全性）、ランタイムオーバーフローのパスは検出を実装していない。校正後：
>
> - E6002 を削除（旧 Assertion
>   failed は E6005 に移動；旧ヌルポインタのセマンティクスは言語概念なし）
> - E6003 を Arithmetic overflow から Runtime index out of bounds に変更（実際の発生面、#279/#271）
> - E6005 を Heap allocation failed から Assertion failed に変更（std.assert の実際のパス）
> - E6006 を Runtime index out of bounds から Function not
>   found に変更（実装は以前よりこうなっている、#255）
> - E6007 を Type cast failed から汎用 Runtime
>   error に変更（ExecutorError のマッピングされていないバリアントの統一ポイント）

#### E7xxx：I/O とシステムエラー

| コード | エラータイプ      | 説明                                 |
| ------ | ----------------- | ------------------------------------ |
| E7001  | File not found    | 存在しないファイルの読み取りを試みた |
| E7002  | Permission denied | ファイル権限が不足                   |
| E7003  | I/O error         | 汎用 I/O エラー                      |
| E7004  | Network error     | ネットワーク操作の失敗               |

#### E8xxx：内部コンパイラエラー

| コード | エラータイプ            | 説明                                   |
| ------ | ----------------------- | -------------------------------------- |
| E8001  | Internal compiler error | コンパイラの内部エラー                 |
| E8002  | Codegen error           | IR/バイトコード生成の失敗              |
| E8003  | Unimplemented feature   | 未実装機能の使用                       |
| E8004  | Optimization error      | コンパイラ最適化エラー（予約、未実装） |

#### W1xxx：警告コード

| コード | 警告タイプ                                   | 説明                                                                 |
| ------ | -------------------------------------------- | -------------------------------------------------------------------- |
| W1001  | Unused exported function                     | 未使用のエクスポート関数                                             |
| W1002  | Unused exported type                         | 未使用のエクスポート型                                               |
| W1003  | Unused import                                | 未使用のインポート                                                   |
| W1004  | Unused exported variable                     | 未使用のエクスポート変数                                             |
| W1005  | Unused exported method                       | 未使用のエクスポートメソッド                                         |
| W1063  | Const generic constraint cannot be evaluated | const ジェネリック制約が評価不能                                     |
| W1080  | Constraint demoted to runtime check          | コンパイル時に制約を証明できないため、ランタイムチェックに降格された |

> W コード位置ルール：E コードと同じくフェーズごとにグループ化（W + フェーズ千位セグメント）、W1xxx
> = 型検査フェーズの警告。
>
> **出力チャネル（#321 M2）**：W コード診断は builder が W プレフィックスに基づきデフォルトで
> `Severity::Warning` を付与する（明示的指定が優先）。収集と表示はエラーと同じ経路（`warning[W####]`
> プレフィックスでレンダリング）だが、コンパイルをブロックせず、正常終了コードにも影響しない。
> `yaoxiang check --deny-warnings`
> は警告を失敗にエスカレーションする（警告が存在する場合、非ゼロコードで終了）。CI の厳格モード用。per-code 抑制（allow 属性など）は将来の拡張項目。

### メッセージ品質仕様

> 本節は #322（M3 メッセージ単一経路と品質、2026-09-03）により導入。`scripts/audit_diagnostics.py`
> により CI で強制実行される。

1. **メッセージ単一経路**：すべてのユーザー可視診断メッセージは、権威あるレジストリのショートカットメソッド +
   locales テンプレートレンダリングを経由しなければならず、コードは構造化パラメータのみを渡す。レジストリをバイパスして
   `Diagnostic::error(...)`
   などの生の値を直接構築することは禁止される——このパスはコード検証と i18n をバイパスする。
2. **コード合法性**：未登録コードと疑似コード（例：`E_INTERNAL`）の使用は禁止。使用箇所のコードリテラルはレジストリに定義済みでなければならない。内部エラーはすべて E8001（`internal_error`）にフォールバックする。
3. **型表示**：型の Display はインスタンス化前後の形式を区別しなければならない（#286：`Expected 'Container', found 'Container'`
   のような生名では区別できない）。
4. **ソルバー内部状態の隔離**：ソルバーの中間状態 TypeVar（Display 形式
   `t<N>`）はユーザー可視メッセージに含めてはならない（#287）。テスト固定：`test_type_error_message_no_solver_typevar_leak`。
5. **E8xxx 境界**：E8xxx はコンパイラの内部一貫性問題（ICE）にのみ使用する。ユーザーが修正可能なエラーに E8001 をフォールバックとして使用することは禁止。ICE メッセージには最小限の再現ガイダンスを添付しなければならない。

---

### ランタイムエラー値とコードの統合

> 本節は #323（M4 ランタイム Error 値にコードを付与、2026-09-03）により導入。E6xxx/E7xxx の意味空間は 2 つのチャネルを同時に担い、コード空間は同一だが、提示チャネルが異なる。

#### 2 つのチャネル

| チャネル                     | キャリア                                                  | 提示方式                                                          |
| ---------------------------- | --------------------------------------------------------- | ----------------------------------------------------------------- |
| コンパイラ/CLI 診断チャネル  | `ExecutorError` などのホスト層のハードエラー              | stderr `error[E####]:`（#280/#281 で E6003/E6005/E6007 接続済み） |
| プログラム内エラー値チャネル | std ライブラリ `Result(T, Error)` の Err キャリア `Error` | 言語値。プログラムが match/比較で消費                             |

#### Error 構造（v0.8 より、破壊的変更）

```
Error { code: String, message: String }
```

- `code` は本仕様の E6xxx/E7xxx 番号を再利用し、文字列形式（例：`"E6008"`）。
- **安定契約**：割り当て済みのコードはバージョン横断で意味が不変。同じ意味のコードに削除済みコード（E6002 の先例）を再利用しない。
- **消費面**：プログラム内で `e.code == "E6xxx"`
  を比較することが唯一のプログラマブル判定契約。`yaoxiang explain E6xxx`
  でドキュメントが統合。ツールチェーン（LSP /
  DAP、RFC-034 参照）はコードを exceptionId として使用する。
- **アクセッサ**：`std.result.code(e)` / `std.result.message(e)`。
- **ユーザー定義エラー**：`Result(T, E)`
  の E はジェネリックパラメータ。真剣にモデリングする場合はユーザー定義型を採用。std の `Error`
  は単なる便利なフォールバックキャリアであり、そのコード体系はユーザー E 型を制約しない。

#### コード割り当てルール

1. ランタイムエラー値コードとコンパイル時診断コードは E6xxx/E7xxx 空間を共有する。新規コードは**実際の発生面**に基づいて割り当て、想像上のシナリオのために予約しない。
2. 登録してから使用：新規コードは権威あるレジストリに登録し、三者間一貫性検証（codes/*.rs ↔ locales
   ↔ 本ドキュメントのコード表）を経た後にのみ発行可能。
3. E7xxx は std.io /
   std.net エラー値用の予約セグメント（現在未使用、io/net の Result 化時に有効化）。

#### 進化パス（ライン C、未実施）

パターンマッチングの完全化（RFC-039）が実装された後、`Error` は
`{ kind: ErrorKind, message: String }` にアップグレード可能で、`code`
は kind から派生する属性となる（バリアント定義箇所がそのままコードレジストリ）。進化期間中、本節の安定契約は維持される。このアップグレードは独立した決定であり、本節の約束を構成しない。

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

##### 定義済みプレースホルダ（よく使用されるもの）

| プレースホルダ | 用途                            | 例                                  |
| -------------- | ------------------------------- | ----------------------------------- |
| `{name}`       | 変数名/型名/trait名などの識別子 | `Unknown variable: '{name}'`        |
| `{expected}`   | 期待される型                    | `Expected type '{expected}'`        |
| `{found}`      | 実際/見つかった型               | `, found type '{found}'`            |
| `{method}`     | メソッド名                      | `Method {method} is not a function` |
| `{trait}`      | trait 名                        | `Cannot find trait: {trait}`        |
| `{path}`       | モジュールパス                  | `Invalid path: {path}`              |
| `{ty}`         | 型式                            | `Invalid type: {ty}`                |
| `{message}`    | 内部エラーメッセージ            | `Internal error: {message}`         |

##### 任意のキー対応

**params は任意のキーに対応し、定義済みに限定されない**。呼び出し側は任意の `key` を渡せる：

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

> **注意**：すべてのエラーコードがプレースホルダを使用するわけではない。一部のエラーコード（例：E0001）は静的メッセージであり、パラメータは不要。

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
1. プロジェクトレベル yaoxiang.toml の language.default を読む
2. 未設定の場合、ユーザーレベル ~/.yaoxiang/yaoxiang.toml を読む
3. どちらも未設定の場合、デフォルトで "en" を使用
4. コンパイラは選択された言語に基づいて I18nRegistry を作成（一度）
5. すべてのエラーはこの I18nRegistry を使用してメッセージをレンダリング
```

#### ゼロテーブルルックアップコストの鍵

**レンダリングはユーザーのプロジェクトをコンパイルする際に発生し、ランタイムではない。**

```
┌─────────────────────────────────────────────────────────────────────────┐
│  フェーズ 1: Rust が YaoXiang コンパイラをコンパイル                          │
│                                                                           │
│  JSON がコンパイラバイナリにパッケージングされる                                    │
│  目的：explain コマンドが直接 i18n データを読み取れるようにする                       │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  フェーズ 2: YaoXiang がユーザープロジェクトをコンパイル（ここでレンダリングが発生）          │
│                                                                           │
│  error! マクロ呼び出し時：                                                    │
│  1. yaoxiang.toml から言語設定を読み取る                                      │
│  2. コンパイラバイナリから対応する言語の i18n JSON をロード                           │
│  3. テンプレート + パラメータ → render() → "Unknown variable: 'x'"           │
│  4. Diagnostic.message = レンダリング済み文字列                                  │
│                                                                           │
│  AOT バイナリには最終文字列が直接格納され、テンプレートもテーブルルックアップも存在しない             │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  フェーズ 3: ユーザープログラム実行時                                            │
│                                                                           │
│  println!("{}", diagnostic.message)                                      │
│  // 最終文字列を直接出力。テーブルルックアップは一切行われない                              │
└─────────────────────────────────────────────────────────────────────────┘
```

| コンポーネント               | 役割                                   | レンダリングタイミング             |
| ---------------------------- | -------------------------------------- | ---------------------------------- |
| `I18nRegistry`               | テンプレートと表示テキストを提供       | ユーザープロジェクトのコンパイル時 |
| `DiagnosticBuilder.render()` | テンプレート + パラメータ → 最終文字列 | ユーザープロジェクトのコンパイル時 |
| `Diagnostic.message`         | レンダリング済み文字列                 | 最終結果を格納                     |
| AOT バイナリ                 | 最終文字列を含む                       | ランタイムに直接使用               |

---

### エラーメッセージ形式

エラーメッセージは以下の形式を採用する：

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

エラーの重大度は `DiagnosticLevel` 列挙で管理され、エラーコード番号から疎結合されている：

```rust
pub enum DiagnosticLevel {
    Error,    // 导致编译失败
    Warning,  // 不影响编译，但建议修复
    Note,     // 补充信息
    Help,     // 修复建议
}
```

| レベル  | プレフィックス    | 説明                       |
| ------- | ----------------- | -------------------------- |
| Error   | `error[E####]:`   | コンパイル失敗の原因となる |
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
| `--lang <code>` | 言語を指定 (en-US, zh-CN、デフォルト en-US) |
| `--json`        | JSON 形式で出力（IDE/LSP での使用用）       |
| `--json-pretty` | 整形された JSON 出力                        |
| `--examples`    | サンプルコードのみを表示                    |
| `--help`        | ヘルプ情報を表示                            |

#### 使用例

```bash
# 默认英文
$ yaoxiang explain E1001
error[E1001]: Unknown variable: {name}
  --> <file>:<line>:<col>

Help: Did you mean to define it?

Example:
  let {name} = value;

# 中文输出
$ yaoxiang explain E1001 --lang zh
error[E1001]: 未知变量: {name}
  --> <file>:<line>:<col>

帮助: 你是否想要定义它？

示例:
  let {name} = value;

# JSON 输出（LSP 集成）
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

本 RFC はエラーコードシステムをゼロから設計するため、後方互換性の問題は存在しない。

**将来の移行戦略**（後続バージョン参考用）：

1. 旧エラーコードから新エラーコードへのマッピングを維持
2. 移行期間中は新旧コードを同時表示
3. 廃止スケジュールの提供

---

## 実施戦略

### フェーズ 1：エラーコードインフラ

1. `src/diagnostics/` ディレクトリ構造を作成
2. `ErrorCode` 列挙を実装
3. `Diagnostic` と `DiagnosticLevel` を実装
4. リソースファイルディレクトリとサンプル JSON を作成

### フェーズ 2：explain コマンド

1. `yaoxiang explain` CLI コマンドを実装
2. `--lang` と `--json` オプションをサポート
3. リソースファイルのロードを統合
4. パラメータテンプレートのレンダリングを実装

### フェーズ 3：コンパイル時統合

1. すべてのエラー報告ポイントを更新して新システムを使用
2. メッセージテンプレートのパラメータ注入を実装
3. 言語優先ロジックを追加
4. 単体テストカバレッジ

### フェーズ 4：IDE/LSP 統合

1. LSP サーバーが explain JSON 出力を統合
2. IDE にエラーコードリンクを表示
3. ホバーでエラー説明を表示
4. クイックフィックス提案

---

## 付録

### 完全エラーコード早見表

| 範囲  | カテゴリ               |
| ----- | ---------------------- |
| E0xxx | 語彙解析と構文解析     |
| E1xxx | 型検査                 |
| E2xxx | 意味解析               |
| E3xxx | コード生成             |
| E4xxx | ジェネリクスとtrait    |
| E5xxx | モジュールとインポート |
| E6xxx | ランタイムエラー       |
| E7xxx | I/O とシステムエラー   |
| E8xxx | 内部コンパイラエラー   |
| E9xxx | 予約                   |

### サポート言語

| コード | 言語         | ステータス |
| ------ | ------------ | ---------- |
| en-US  | English (US) | デフォルト |
| zh-CN  | 簡体字中国語 | 計画中     |

### エラーメッセージ例の比較

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

- [Rust コンパイラエラーインデックス](https://doc.rust-lang.org/error_codes/error-index.html)
- [GCC エラーメッセージ形式](https://gcc.gnu.org/onlinedocs/gcc-13.1.0/gcc/Warning-Options.html)
- [Clang 診断形式](https://clang.llvm.org/diagnostics.html)
- [Language Server Protocol](https://microsoft.github.io/language-server-protocol/)

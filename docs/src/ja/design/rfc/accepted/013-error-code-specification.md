---
title: 'RFC 013: エラーコード規範'
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

# RFC 013: エラーコード規範

## 概要

本 RFC は YaoXiang コンパイラのエラーコード分類規範を提案する。Rust 類似の単層番号システムを採用し、JSON リソースファイルと組み合わせて多言語サポートを実現し、`yaoxiang explain`
コマンドを通じてエラー説明機能を提供する。

## 動機

### なぜ標準化されたエラーコードが必要か？

1. **ユーザーエクスペリエンス**：エラーコードを見ることで、ユーザーはエラーの種類と重大度を迅速に判断できる
2. **ドキュメント構成**：カテゴリ別にグループ化することで、エラーリファレンスドキュメントの作成と保守が容易になる
3. **ツール統合**：IDE/LSP はエラーコードに基づいて迅速な修正提案とドキュメントリンクを提供できる
4. **国際化サポート**：エラーメッセージとコードを分離することで、多言語翻訳が容易になる

### 設計目標

- **簡潔**：単層番号方式で、ユーザーは複雑な分類ルールを記憶する必要がない
- **親しみやすい**：Rust 類似のエラーメッセージ形式で、ヘルプ情報と例を含む
- **拡張可能**：リソースファイル駆動で、新しいエラーや新しい言語の追加が容易
- **ツールフレンドリー**：explain コマンド + JSON 出力で、IDE/LSP 統合をサポート

---

## 提案

### コア設計：単層番号システム

4 桁の数字番号を採用し、コンパイル段階でグループ化する：

```
Exxxx
││││
│││└── 連番 (000-999)
││└─── コンパイル段階 (0-9)
└───── 固定プレフィックス 'E'
```

### 段階区分

| 段階  | 範囲  | 説明                   |
| ----- | ----- | ---------------------- |
| **0** | E0xxx | 字句・構文解析         |
| **1** | E1xxx | 型検査                 |
| **2** | E2xxx | 意味解析               |
| **3** | E3xxx | コード生成             |
| **4** | E4xxx | ジェネリクスと trait   |
| **5** | E5xxx | モジュールとインポート |
| **6** | E6xxx | ランタイムエラー       |
| **7** | E7xxx | I/O とシステムエラー   |
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

### エラーコード定義と汎用 Builder

**基本原則**：エラーコード定義と表示テキストの分離

- `ErrorCodeDefinition`：エラーコードのメタデータ（code、category、template）で、表示テキストは含まない
- `locales/*.json`：各言語の表示テキスト（title、message、help、エラーコードはネストされたオブジェクト）
- `DiagnosticBuilder`：trait-per-error 設計に代わる汎用ビルダー

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

| 特性                             | 説明                                                     |
| -------------------------------- | -------------------------------------------------------- |
| **単一 Builder**                 | 1 つの `DiagnosticBuilder` ですべてのエラーコードに汎用  |
| **型安全**                       | ショートカットメソッドがパラメータの正確性を保証         |
| **自己文書化**                   | `E1001::unknown_variable(name)` が一目でわかる           |
| **テンプレート分離**             | メッセージテンプレートとコードの分離で、i18n が容易      |
| **ランタイムオーバーヘッドゼロ** | コンパイル時レンダリング、AOT バイナリにテーブル参照なし |

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

#### 手動で Builder を使用

```rust
// 需要手动控制时
E1001::unknown_variable(&var_name)
    .at(my_span)           // 自定义 span
    .build(&custom_i18n)   // 自定义 i18n
```

---

## 詳細設計

### エラーコード一覧

#### E0xxx：字句・構文解析

| コード | エラー種別                | 説明                                   |
| ------ | ------------------------- | -------------------------------------- |
| E0001  | Invalid character         | ソースコードに不正な文字が含まれている |
| E0002  | Invalid number literal    | 数値リテラルのフォーマットが正しくない |
| E0003  | Unterminated string       | 複数行文字列に終了引用符がない         |
| E0004  | Invalid character literal | 文字リテラルが正しくない               |
| E0010  | Expected token            | 構文解析時に特定の token が必要        |
| E0011  | Unexpected token          | 予期しない token に遭遇した            |
| E0012  | Invalid syntax            | 式/文の構文エラー                      |
| E0013  | Mismatched brackets       | 丸括弧、角括弧、波括弧の不一致         |
| E0014  | Missing semicolon         | 文末にセミコロンがない                 |
| E0016  | Expected expression       | 式が必要                               |
| E0018  | Keyword as name           | キーワードは名前として使用できない     |

#### E1xxx：型検査

| コード | エラー種別                                             | 説明                                              |
| ------ | ------------------------------------------------------ | ------------------------------------------------- |
| E1001  | Unknown variable                                       | 参照された変数が未定義                            |
| E1002  | Type mismatch                                          | 期待型と実際の型が一致しない                      |
| E1003  | Unknown type                                           | 参照された型が存在しない                          |
| E1010  | Parameter count mismatch                               | 関数呼び出しの引数の数が定義と一致しない          |
| E1011  | Parameter type mismatch                                | 引数の型チェックに失敗                            |
| E1012  | Return type mismatch                                   | 関数の戻り値の型が誤り                            |
| E1013  | Function not found                                     | 未定義の関数を呼び出し                            |
| E1020  | Cannot infer type                                      | コンテキストから型を推論できない                  |
| E1021  | Type inference conflict                                | 複数の制約により型が矛盾                          |
| E1030  | Pattern non-exhaustive                                 | match 式がすべてのパターンを網羅していない        |
| E1031  | Unreachable pattern                                    | 決してマッチしないパターン                        |
| E1040  | Operation not supported                                | 型がその操作をサポートしていない                  |
| E1041  | Index out of bounds                                    | 配列/リストのインデックスが範囲外                 |
| E1042  | Field not found                                        | 存在しない構造体のフィールドにアクセス            |
| E1050  | Boolean operand required                               | ブールオペランドが必要                            |
| E1051  | Logical NOT requires boolean operand                   | 論理 NOT にはブールオペランドが必要               |
| E1052  | Invalid dereference                                    | 無効な参照外し                                    |
| E1053  | Non-struct field access                                | 非構造体のフィールドアクセス                      |
| E1054  | Conditional type mismatch                              | 条件型の不一致                                    |
| E1055  | Constraint in non-generic context                      | 非ジェネリックコンテキストに制約が出現            |
| E1060  | Type parameter count mismatch                          | 型パラメータの数が一致しない                      |
| E1061  | Cannot instantiate generic                             | ジェネリックをインスタンス化できない              |
| E1062  | Const generic constraint failed                        | const ジェネリック制約の失敗                      |
| E1064  | Invalid binding position                               | バインディング位置のインデックスが無効（RFC-004） |
| E1071  | Type definitions are only allowed at module level      | 型定義はモジュールレベルでのみ許可                |
| E1081  | `?` can only be used within functions returning Result | `?` は Result を返す関数内でのみ使用可能          |
| E1082  | `?` can only be used with Result expressions           | `?` は Result 式にのみ使用可能                    |
| E1083  | Error type mismatch for `?`                            | `?` のエラー型が一致しない                        |
| E1090  | Type universe easter egg                               | Type: Type = Type イースターエッグ（Note レベル） |
| E1091  | Invalid generic meta type                              | 無効なジェネリックメタ型                          |
| E1092  | Invalid refinement type argument form                  | 精化型実引数の形式が不正                          |
| E1093  | Refinement argument count mismatch                     | 精化実引数の数が一致しない                        |
| E1094  | Unused compile-time value parameter                    | 未使用のコンパイル時値パラメータ                  |
| E1095  | Unknown interface                                      | 未知のインターフェース                            |
| E1096  | Interface arity mismatch                               | インターフェースのパラメータ数が一致しない        |
| E1097  | Interface member name conflict                         | インターフェースメンバーの名前衝突                |
| E1098  | Interface method not implemented                       | インターフェースメソッドが未実装                  |
| E1099  | Interface method signature mismatch                    | インターフェースメソッドのシグネチャが一致しない  |
| E1100  | Duplicate interface method implementation              | インターフェースメソッドの実装が重複              |
| E1101  | Type does not implement interface                      | 型がインターフェースを実装していない              |
| E1102  | Loop control statement outside of a loop               | ループ外でのループ制御文                          |

#### E2xxx：意味解析

| コード | エラー種別                        | 説明                                       |
| ------ | --------------------------------- | ------------------------------------------ |
| E2001  | Scope error                       | 変数が現在のスコープにない                 |
| E2002  | Duplicate definition              | 同一スコープ内で重複定義                   |
| E2003  | Lifetime error                    | ライフタイム制約が満たされない             |
| E2010  | Immutable assignment              | 不変変数の変更を試みた                     |
| E2011  | Uninitialized use                 | 未初期化変数の使用                         |
| E2012  | Mutability conflict               | 不変コンテキストでの可変参照の使用         |
| E2013  | Variable shadowing                | 変数のシャドーイング                       |
| E2014  | Use of moved value                | ムーブ済み値の使用                         |
| E2016  | Immutable assignment              | 不変代入                                   |
| E2018  | Mutable/immutable borrow conflict | 可変/不変の借用衝突                        |
| E2019  | Double free                       | 二重解放                                   |
| E2020  | Use after free                    | 解放後使用                                 |
| E2027  | Unsafe dereference                | unsafe な参照外し                          |
| E2090  | Invalid signature                 | 関数シグネチャの解析エラー                 |
| E2091  | Unknown type in signature         | シグネチャに未知の型が出現                 |
| E2092  | Missing arrow in signature        | シグネチャに戻り値の矢印がない             |
| E2093  | Duplicate parameter name          | パラメータ名の重複                         |
| E2094  | Generic parameter shadowing       | ジェネリックパラメータのシャドーイング     |
| E2095  | Parameter name shadows generic    | パラメータ名がジェネリックをシャドーイング |

#### E3xxx：コード生成

| コード | エラー種別                             | 説明                                                       |
| ------ | -------------------------------------- | ---------------------------------------------------------- |
| E3004  | Unsupported iterator                   | サポートされていないイテレータ                             |
| E3005  | IR generation error                    | IR 生成の内部エラー                                        |
| E3006  | Unresolved variable                    | IR 生成段階で変数が未解決                                  |
| E3007  | Top-level initializer must be constant | トップレベルバインディングの初期化は定数でなければならない |
| E3014  | Register overflow                      | レジスタオーバーフロー                                     |
| E3017  | Invalid operand (code generation)      | 無効なオペランド（コード生成）                             |

#### E4xxx：ジェネリクスと trait

| コード | エラー種別                              | 説明                                     |
| ------ | --------------------------------------- | ---------------------------------------- |
| E4001  | Generic parameter mismatch              | ジェネリックパラメータの不一致           |
| E4002  | Trait bound violated                    | trait 制約が満たされない                 |
| E4003  | Associated type error                   | 関連型の定義/使用エラー                  |
| E4004  | Duplicate trait implementation          | 同一 trait の重複実装                    |
| E4005  | Trait not found                         | 要求される trait が見つからない          |
| E4006  | Sized bound violated                    | Sized 制約が満たされない（予約、未実装） |
| E4010  | Division by zero in constant expression | 定数式でのゼロ除算                       |
| E4011  | Constant overflow                       | 定数オーバーフロー                       |
| E4012  | Constant recursion too deep             | 定数再帰が深すぎる                       |
| E4014  | Constant evaluation failed              | 定数評価失敗                             |
| E4018  | Refinement predicate violation          | 精化述語違反                             |
| E4019  | Type equality does not hold             | 型の等式が成立しない                     |
| E4020  | Proof function required                 | 制約検証のための証明関数が必要           |

#### E5xxx：モジュールとインポート

| コード | エラー種別            | 説明                                           |
| ------ | --------------------- | ---------------------------------------------- |
| E5001  | Module not found      | インポートされたモジュールが存在しない         |
| E5002  | Cyclic import         | モジュール間の循環依存                         |
| E5003  | Symbol not exported   | 未エクスポートのシンボルにアクセスしようとした |
| E5004  | Invalid module path   | モジュールパスのフォーマットエラー             |
| E5005  | Private access        | プライベートシンボルにアクセス                 |
| E5006  | Duplicate import      | 重複インポート                                 |
| E5007  | Module export listing | モジュールエクスポート一覧（付随ヒント情報）   |

#### E6xxx：ランタイムエラー

| コード | エラー種別                  | 説明                                                     |
| ------ | --------------------------- | -------------------------------------------------------- |
| E6001  | Division by zero            | 整数のゼロ除算                                           |
| E6002  | ~~Assertion failed~~        | ~~予約位置（言語概念なし、削除済み）~~                   |
| E6003  | Runtime index out of bounds | ランタイムインデックス範囲外（#280 配線）                |
| E6004  | Stack overflow              | スタック領域の枯渇                                       |
| E6005  | Assertion failed            | assert 失敗（#280 配線）                                 |
| E6006  | Function not found          | ランタイムで関数が見つからない                           |
| E6007  | Runtime error (generic)     | 汎用ランタイムエラー                                     |
| E6008  | Key not found               | Dict キー欠如（#299 §4）                                 |
| E6009  | Invalid range step          | Range ステップ値不正（step=0、std.range Result 化 #316） |
| E6010  | Integer parse failed        | 整数パース失敗（std.string.parse_int）                   |
| E6011  | Float parse failed          | 浮動小数点パース失敗（std.string.parse_float）           |

> **#280 改訂（2026-08-09）**：コード表は元々 Rust セマンティクス草案（Assertion failed/Arithmetic
> overflow/Heap allocation failed/Type cast
> failed）に基づいて定義されていたが、実装の実際の要件と一致しない。YaoXiang には null ポインタ/ヒープ割り当て失敗/型変換の概念がなく（値セマンティクス +
> Rust メモリ安全性）、ランタイムオーバーフロー経路は検出が実装されていない。校正後：
>
> - E6002 削除（旧 Assertion
>   failed を E6005 に移動；旧 null ポインタのセマンティクスは言語概念なし）
> - E6003 を Arithmetic overflow から Runtime index out of
>   bounds に変更（実際のトリガー面、#279/#271）
> - E6005 を Heap allocation failed から Assertion failed に変更（std.assert 実際のパス）
> - E6006 を Runtime index out of bounds から Function not found に変更（実装は元からこう、#255）
> - E6007 を Type cast failed から汎用 Runtime error に変更（ExecutorError 未マップ変種の統一落点）

#### E7xxx：I/O とシステムエラー

| コード | エラー種別        | 説明                                 |
| ------ | ----------------- | ------------------------------------ |
| E7001  | File not found    | 存在しないファイルの読み込みを試みた |
| E7002  | Permission denied | ファイル権限不足                     |
| E7003  | I/O error         | 汎用 I/O エラー                      |
| E7004  | Network error     | ネットワーク操作失敗                 |

#### E8xxx：内部コンパイラエラー

| コード | エラー種別              | 説明                                   |
| ------ | ----------------------- | -------------------------------------- |
| E8001  | Internal compiler error | コンパイラの内部エラー                 |
| E8002  | Codegen error           | IR/バイトコード生成失敗                |
| E8003  | Unimplemented feature   | 未実装機能の使用                       |
| E8004  | Optimization error      | コンパイラ最適化エラー（予約、未実装） |

#### W1xxx：警告コード

| コード | 警告種別                                     | 説明                                                           |
| ------ | -------------------------------------------- | -------------------------------------------------------------- |
| W1001  | Unused exported function                     | 未使用のエクスポート関数                                       |
| W1002  | Unused exported type                         | 未使用のエクスポート型                                         |
| W1003  | Unused import                                | 未使用のインポート                                             |
| W1004  | Unused exported variable                     | 未使用のエクスポート変数                                       |
| W1005  | Unused exported method                       | 未使用のエクスポートメソッド                                   |
| W1063  | Const generic constraint cannot be evaluated | const ジェネリック制約が評価不能                               |
| W1080  | Constraint demoted to runtime check          | コンパイル時に制約を証明できないため、ランタイムチェックに降格 |

> W コード位規則：E コードと等価に段階でグループ化（W+段階千位セグメント）、W1xxx
> = 型検査段階の警告。
>
> **発射パス（#321 M2）**：W コード診断は builder により W 接頭辞でデフォルト `Severity::Warning`
> と注釈付けされる（明示的な指定が優先）、収集と表示はエラーと同軌道で（`warning[W####]`
> 接頭辞レンダリング）、コンパイルをブロックせず、成功終了コードにも影響しない。
> `yaoxiang check --deny-warnings`
> は警告を失敗に格上げする（警告が存在する場合に非ゼロコードで終了）、CI 厳格モード向け。per-code 抑制（allow 属性など）は今後の拡張項目。

### メッセージ品質規範

> 本節は #322（M3 メッセージ単一経路と品質、2026-09-03）により導入。`scripts/audit_diagnostics.py`
> により CI で強制実行。

1. **メッセージ単一経路**：すべてのユーザー可視診断メッセージは権威ある登録表のショートカットメソッド +
   locales テンプレートレンダリングを経由する必要がある。コードは構造化パラメータのみを渡す。登録表をバイパスして
   `Diagnostic::error(...)`
   などのネイティブ値を直接構築することは禁止——このパスはコード検証と i18n をバイパスする。
2. **コード合法性**：未登録コードと偽コード（例：`E_INTERNAL`）の使用は禁止。使用箇所のコードリテラルは登録表で定義済みである必要がある。内部エラーは一律 E8001（`internal_error`）にフォールバック。
3. **型表示**：型の Display はインスタンス化前後の形態を区別する必要がある（#286：`Expected 'Container', found 'Container'`
   のような素の名前では区別できない）。
4. **ソルバー内部状態の隔離**：ソルバー中間状態の TypeVar（Display 形態
   `t<N>`）はユーザー可視メッセージに入ってはいけない（#287）。テストアンカー：`test_type_error_message_no_solver_typevar_leak`。
5. **E8xxx の境界**：E8xxx はコンパイラの内部整合性の問題（ICE）にのみ使用する。ユーザーが修正可能なエラーは E8001 をフォールバックとして使用することは禁止；ICE メッセージには最小限の再現手順を添付する必要がある。

---

### ランタイムエラー値とコードの連携

> 本節は #323（M4 ランタイム Error 値にコードを付与、2026-09-03）により導入。E6xxx/E7xxx セマンティクス空間は 2 つのチャネルを同時に担い、コード空間は同一、表示チャネルが異なる。

#### 2 つのチャネル

| チャネル                     | 载体                                                  | 表示方式                                                          |
| ---------------------------- | ----------------------------------------------------- | ----------------------------------------------------------------- |
| コンパイラ/CLI 診断チャネル  | `ExecutorError` などホスト層のハードエラー            | stderr `error[E####]:`（#280/#281 で E6003/E6005/E6007 配線済み） |
| プログラム内エラー値チャネル | std ライブラリ `Result(T, Error)` の Err 载体 `Error` | 言語値、プログラムが match/比較で消費                             |

#### Error 構造（v0.8 以降、破壊的変更）

```
Error { code: String, message: String }
```

- `code` は本規範の E6xxx/E7xxx 番号を再利用、文字列形態（例：`"E6008"`）。
- **安定契約**：割り当て済みコードはバージョンを超えてセマンティクスが不变；同一セマンティクスに削除済みコードを再利用しない（E6002 が先例）。
- **消費面**：プログラム内 `e.code == "E6xxx"`
  の比較が唯一のプログラマブル判定契約；`yaoxiang explain E6xxx`
  ドキュメントが貫通；ツールチェーン（LSP / DAP、RFC-034 参照）はコードを exceptionId とする。
- **アクセサ**：`std.result.code(e)` / `std.result.message(e)`。
- **ユーザー定義エラー**：`Result(T, E)`
  の E はジェネリックパラメータ、真剣にモデル化する場合はユーザー定義型を経由；std `Error`
  は単なる便利なフォールバック载体であり、そのコード体系はユーザー E 型を制約しない。

#### コード割当規則

1. ランタイムエラー値コードとコンパイラ診断コードは E6xxx/E7xxx 空間を共有し、新コードは**実際のトリガー面**に従って割当。想像上のシナリオに対する予約は行わない。
2. 先に登録、後で使用：新コードは権威ある登録表に登録され、3 者の整合性検証（codes/*.rs ↔ locales
   ↔ 本ドキュメントのコード表）を経た後にのみ発射可能。ランタイムエラー値コードの登録ソースは
   `src/std/result.rs` の `RUNTIME_ERROR_CODES` テーブル（診断コードと同じく
   `scripts/check_error_codes.py` で検証）。
3. E7xxx は std.io / std.net エラー値用にセグメントを予約（現在空、io/net Result 化時に有効化）。
4. 発射ポイント（#323 M4）：std 各モジュールは `error_new(code, message)`
   で Error 値を構築；消費側は `std.result.unwrap_err`
   で Err 载体を取り出し、`std.result.code/message` でフィールドを読み取る。

#### 進化パス（線 C、未実施）

パターンマッチ完全化（RFC-039）の着地後、`Error` を `{ kind: ErrorKind, message: String }`
にアップグレード可能、`code`
は kind から派生する属性に変換（変種定義箇所がそのままコード登録表）。進化期間中の本節のコード安定契約は不变を維持；このアップグレードは独立した決定であり、本節のコミットメントを構成しない。

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

#### テンプレートプレースホルダー

##### 事前定義プレースホルダー（一般的）

| プレースホルダー | 用途                             | 例                                  |
| ---------------- | -------------------------------- | ----------------------------------- |
| `{name}`         | 変数名/型名/trait 名などの識別子 | `Unknown variable: '{name}'`        |
| `{expected}`     | 期待される型                     | `Expected type '{expected}'`        |
| `{found}`        | 実際/見つかった型                | `, found type '{found}'`            |
| `{method}`       | メソッド名                       | `Method {method} is not a function` |
| `{trait}`        | trait 名                         | `Cannot find trait: {trait}`        |
| `{path}`         | モジュールパス                   | `Invalid path: {path}`              |
| `{ty}`           | 型式                             | `Invalid type: {ty}`                |
| `{message}`      | 内部エラーメッセージ             | `Internal error: {message}`         |

##### 任意の key をサポート

**params は任意の key をサポートし、事前定義されたものに限らない**。呼び出し側は任意の `key`
を渡すことができる：

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

> **注意**：すべてのエラーコードがプレースホルダーを使用するわけではない。E0001 のような一部のエラーコードは静的メッセージであり、パラメータは不要。

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
# 错误消息语言，可选：en, zh, ja, ...
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
1. 读取项目级 yaoxiang.toml 的 language.default
2. 若未配置，读取用户级 ~/.yaoxiang/yaoxiang.toml
3. 若都未配置，默认使用 "en"
4. 编译器根据选择的语言创建 I18nRegistry（一次）
5. 所有错误使用该 I18nRegistry 渲染消息
```

#### ゼロテーブルルックアップオーバーヘッドの鍵

**レンダリングはユーザープロジェクトをコンパイルする時に発生し、ランタイムではない。**

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
| AOT バイナリ                 | 最終文字列を含む                       | ランタイムで直接使用               |

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

エラー重大度は `DiagnosticLevel` 列挙で管理され、エラーコード番号から分離される：

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

### `yaoxiang explain` コマンド

#### コマンド構文

```bash
yaoxiang explain <ERROR_CODE> [OPTIONS]
```

#### オプション

| オプション      | 説明                                        |
| --------------- | ------------------------------------------- |
| `--lang <code>` | 言語を指定 (en-US、zh-CN、デフォルト en-US) |
| `--json`        | JSON 形式出力（IDE/LSP で使用）             |
| `--json-pretty` | フォーマット済み JSON 出力                  |
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

## 後方互換性

本 RFC はエラーコードシステムをゼロから設計するため、後方互換性の問題はない。

**将来の移行戦略**（後続バージョンの参考）：

1. 旧エラーコードと新エラーコードのマッピングを保持
2. 移行期間中は新旧コードの両方を表示
3. 廃止スケジュールの提供

---

## 実装戦略

### 段階 1：エラーコード基盤アーキテクチャ

1. `src/diagnostics/` ディレクトリ構造の作成
2. `ErrorCode` 列挙の実装
3. `Diagnostic` と `DiagnosticLevel` の実装
4. リソースファイルディレクトリとサンプル JSON の作成

### 段階 2：explain コマンド

1. `yaoxiang explain` CLI コマンドの実装
2. `--lang` と `--json` オプションのサポート
3. リソースファイル読み込みの統合
4. パラメータテンプレートレンダリングの実装

### 段階 3：コンパイル時統合

1. すべてのエラー報告箇所を新システムを使用するように更新
2. メッセージテンプレートパラメータ注入の実装
3. 言語優先度ロジックの追加
4. ユニットテストカバレッジ

### 段階 4：IDE/LSP 統合

1. LSP サーバーへの explain JSON 出力の統合
2. IDE でのエラーコードリンクの表示
3. ホバーでのエラー説明の表示
4. クイックフィックス提案

---

## 付録

### 完全エラーコード早見表

| 範囲  | カテゴリ               |
| ----- | ---------------------- |
| E0xxx | 字句・構文解析         |
| E1xxx | 型検査                 |
| E2xxx | 意味解析               |
| E3xxx | コード生成             |
| E4xxx | ジェネリクスと trait   |
| E5xxx | モジュールとインポート |
| E6xxx | ランタイムエラー       |
| E7xxx | I/O とシステムエラー   |
| E8xxx | 内部コンパイラエラー   |
| E9xxx | 予約                   |

### サポートされている言語

| コード | 言語         | 状態       |
| ------ | ------------ | ---------- |
| en-US  | English (US) | デフォルト |
| zh-CN  | 简体中文     | 計画中     |

### エラーメッセージの例比較

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

- [Rust コンパイラエラー索引](https://doc.rust-lang.org/error_codes/error-index.html)
- [GCC エラーメッセージ形式](https://gcc.gnu.org/onlinedocs/gcc-13.1.0/gcc/Warning-Options.html)
- [Clang 診断形式](https://clang.llvm.org/diagnostics.html)
- [Language Server Protocol](https://microsoft.github.io/language-server-protocol/)

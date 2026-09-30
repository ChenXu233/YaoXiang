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

本 RFC は YaoXiang コンパイラのエラーコード分類仕様を提案する。Rust 風の単層番号システムを採用し、JSON リソースファイルと組み合わせて多言語サポートを実現し、`yaoxiang explain`
コマンドによりエラー説明機能を提供する。

## 動機

### なぜ標準化されたエラーコードが必要なのか？

1. **ユーザー体験**：エラーコードを見ることで、ユーザーはエラーの種類と重大度をすばやく判断できる
2. **ドキュメント構成**：カテゴリ別にグループ化することで、エラー参考ドキュメントの作成と保守が容易になる
3. **ツール統合**：IDE/LSP はエラーコードに基づいて迅速な修正提案やドキュメントリンクを提供できる
4. **国際化対応**：エラーメッセージとコードを分離することで、多言語翻訳が容易になる

### 設計目標

- **簡潔**：単層番号方式により、ユーザーは複雑な分類ルールを覚える必要がない
- **親しみやすい**：Rust 風のエラーメッセージ形式を採用。ヘルプ情報とサンプル付き
- **拡張性**：リソースファイル駆動により、新しいエラーや新しい言語の追加が容易
- **ツールフレンドリ**：explain コマンド + JSON 出力で IDE/LSP 統合をサポート

---

## 提案

### 中核設計：単層番号システム

4 桁の数字番号を採用し、コンパイル段階でグループ化する：

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
- `DiagnosticBuilder`：汎用ビルダー。trait-per-error 設計に代わるもの

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
    // ... 其他错误码
];
```

#### 設計上の利点

| 特性                     | 説明                                                               |
| ------------------------ | ------------------------------------------------------------------ |
| **単一 Builder**         | 一つの `DiagnosticBuilder` ですべてのエラーコードに対応            |
| **型安全**               | ショートカットメソッドにより引数の正しさを保証                     |
| **自己文書化**           | `E1001::unknown_variable(name)` で一目瞭然                         |
| **テンプレート分離**     | メッセージテンプレートとコードが分離されており、i18n が容易        |
| **ランタイムコストゼロ** | コンパイル時レンダリングにより、AOT バイナリでのテーブル参照は不要 |

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

#### ビルダーの手動使用

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

<!-- code-table:E0xxx start -->

| コード | 説明                   |
| ------ | ---------------------- |
| E0001  | 無効な文字             |
| E0002  | 無効な数値リテラル     |
| E0003  | 終端されていない文字列 |
| E0004  | 無効な文字リテラル     |
| E0010  | 期待されたトークン     |
| E0011  | 予期しないトークン     |
| E0012  | 無効な構文             |
| E0013  | 一致しない括弧         |
| E0014  | セミコロンの欠落       |
| E0016  | 期待された式           |
| E0018  | キーワードを名前に使用 |

<!-- code-table:E0xxx end -->

#### E1xxx：型検査

<!-- code-table:E1xxx start -->

| コード | 説明                                               |
| ------ | -------------------------------------------------- |
| E1001  | 未知の変数                                         |
| E1002  | 型の不一致                                         |
| E1003  | 未知の型                                           |
| E1010  | 引数の数が一致しない                               |
| E1011  | 引数の型が一致しない                               |
| E1012  | 戻り値の型が一致しない                             |
| E1013  | 関数が見つからない                                 |
| E1014  | 名前付き引数の名前が未知                           |
| E1015  | 引数の重複指定                                     |
| E1020  | 型を推論できない                                   |
| E1021  | 型推論の衝突                                       |
| E1030  | パターンが不完全                                   |
| E1031  | 到達不能なパターン                                 |
| E1032  | パターンの重複バインド                             |
| E1033  | OR パターンのバインド不一致                        |
| E1034  | 構造体パターンにフィールド欠落                     |
| E1040  | 操作がサポートされていない                         |
| E1041  | インデックス範囲外                                 |
| E1042  | フィールドが見つからない                           |
| E1050  | ブール被演算子が必要                               |
| E1051  | 論理 NOT はブール被演算子が必要                    |
| E1052  | 無効なデリファレンス                               |
| E1053  | 非構造体へのフィールドアクセス                     |
| E1054  | 条件型が一致しない                                 |
| E1055  | 非ジェネリクス文脈での制約                         |
| E1060  | 型引数の数が一致しない                             |
| E1061  | ジェネリクスをインスタンス化できない               |
| E1062  | const ジェネリクス制約違反                         |
| E1064  | バインド位置のインデックスが無効                   |
| E1065  | 非関数値の呼び出し                                 |
| E1071  | 型定義はモジュールレベルでのみ可能                 |
| E1081  | `?` は伝播可能型を返す関数内でのみ使用可能         |
| E1082  | `?` は Try を実装する型にのみ使用可能              |
| E1083  | `?` のエラー型が一致しない                         |
| E1090  | ✨ 名状しがたいもの ✨                             |
| E1091  | 無効なジェネリクスメタ型                           |
| E1092  | 精化型引数の形式が不正                             |
| E1093  | 精化引数の数が一致しない                           |
| E1094  | 未使用のコンパイル時値引数                         |
| E1095  | 未知のインタフェース                               |
| E1096  | インタフェースの引数の数が一致しない               |
| E1097  | インタフェースメンバーの名前衝突                   |
| E1098  | インタフェースメソッドが未実装                     |
| E1099  | インタフェースメソッドのシグネチャが一致しない     |
| E1100  | インタフェースメソッドの重複実装                   |
| E1101  | 型がインタフェースを実装していない                 |
| E1102  | ループ外でのループ制御文                           |
| E1103  | 型位置に角括弧は使用不可                           |
| E1104  | インタフェース実装が型定義モジュールにない         |
| E1105  | 値変体コンストラクタはフィールドとしてアクセス不可 |
| E1106  | 制約が満たされていない                             |
| E1107  | メソッドオーバーロードの曖昧性                     |

<!-- code-table:E1xxx end -->

> **RFC-011b 関連（2026-09-22 注）**：[RFC-011b: 演算子オーバーロード](./011b-operator-overloading.md)
> 実装時に本セクションの 3 箇所に影響する——① `E1081` / `E1082`
> の文言から「Result」という語を削除（`?` は `Try`
> インタフェースによる判定に変更し、特定の型名に紐づけない）、段階 2 で本文書の「三方整合性」フローに従って同期（codes/*.rs
> ↔ locales ↔ コード表）；② `Equal` の前置制約（線形トークン）が満たされない場合の拒否診断は
> `E1101`（型がインタフェースを実装していない）族を再利用；③ 段階 1 配線後、`Struct == Struct` は
> `E6007` ランタイムエラーからコンパイル時判定に変わり、`E6007`
> の発火面が縮小する。コード表内の登録文言は実装が定着するまで現状維持。

#### E2xxx：意味解析

<!-- code-table:E2xxx start -->

| コード | 説明                                     |
| ------ | ---------------------------------------- |
| E2001  | スコープエラー                           |
| E2002  | 重複定義                                 |
| E2003  | 所有権エラー                             |
| E2010  | 不変変数への代入                         |
| E2011  | 未初期化変数の使用                       |
| E2012  | 可変性の衝突                             |
| E2013  | 変数のシャドーイング                     |
| E2014  | 移動済み値の使用                         |
| E2016  | 不変変数への代入                         |
| E2018  | 可変/不変の借用衝突                      |
| E2019  | 二重解放                                 |
| E2020  | 解放後の使用                             |
| E2027  | unsafe デリファレンス                    |
| E2029  | spawn 内 ref ループ                      |
| E2030  | 精化型制約違反                           |
| E2031  | 精化制約を証明できない                   |
| E2090  | 無効なシグネチャ                         |
| E2091  | シグネチャに未知の型                     |
| E2092  | シグネチャに矢印がない                   |
| E2093  | 引数名の重複                             |
| E2094  | ジェネリクス引数のシャドーイング         |
| E2095  | 引数名によるジェネリクスのシャドーイング |

<!-- code-table:E2xxx end -->

> 予約コード説明（2026-09-14 棚卸し、#251 リリース方針）：E2019（二重解放）、E2020（解放後使用）、E2027（unsafe デリファレンス）、E2029（spawn 内 ref ループ）は登録とユニットテストの固定が完了しているが、まだ到達可能な yx ソースコード表層がない（明示的 drop 文、Ptr デリファレンス文法、spawn
> ref サイクル構築経路）——意味的に完全正確という主張はこれら 4 コードをカバーせず、実装補完まで「予約」として扱う。

#### E3xxx：コード生成

<!-- code-table:E3xxx start -->

| コード | 説明                                                 |
| ------ | ---------------------------------------------------- |
| E3004  | サポートされていないイテレータ                       |
| E3005  | IR 生成エラー                                        |
| E3006  | 未解決の変数                                         |
| E3007  | トップレベルバインドの初期化は定数でなければならない |
| E3008  | サポートされていない match パターン                  |
| E3014  | レジスタオーバーフロー                               |
| E3017  | 無効な被演算子（コード生成）                         |
| E3018  | 単相化インスタンス化の失敗                           |
| E3019  | トップレベルバインドの循環依存                       |
| E3020  | プログラムエントリポイントの欠落                     |
| E3021  | エントリポイントが関数でない                         |
| E3022  | エントリポイント main のシグネチャが一致しない       |
| E3023  | トップレベルでは実行文は許可されない                 |

<!-- code-table:E3xxx end -->

#### E4xxx：ジェネリクスとトレイト

<!-- code-table:E4xxx start -->

| コード | 説明                           |
| ------ | ------------------------------ |
| E4001  | ジェネリクス制約違反           |
| E4002  | トレイトが見つからない         |
| E4003  | トレイト実装の欠落             |
| E4004  | トレイト実装の衝突             |
| E4005  | 関連型が見つからない           |
| E4010  | 定数のゼロ除算                 |
| E4011  | 定数のオーバーフロー           |
| E4012  | 定数再帰が深すぎる             |
| E4014  | 定数評価失敗                   |
| E4018  | 精化述語違反                   |
| E4019  | 型の等式が成立しない           |
| E4020  | 証明関数が必要                 |
| E4021  | ループ停止性を自動証明できない |
| E4022  | 測度が成立しない               |

<!-- code-table:E4xxx end -->

> E4006/E8004 は現在発火点なし（予約コード）：Sized 制約と最適化エラーパスは実装待ち、実装時は実際のトリガー面に従って配線する。

#### E5xxx：モジュールとインポート

<!-- code-table:E5xxx start -->

| コード | 説明                       |
| ------ | -------------------------- |
| E5001  | モジュールが見つからない   |
| E5002  | インポートエラー           |
| E5003  | エクスポートが見つからない |
| E5004  | 循環依存                   |
| E5005  | 無効なモジュールパス       |
| E5006  | 重複インポート             |
| E5007  | モジュールエクスポート     |

<!-- code-table:E5xxx end -->

#### E6xxx：ランタイムエラー

<!-- code-table:E6xxx start -->

| コード | 説明                             |
| ------ | -------------------------------- |
| E6001  | ゼロ除算エラー                   |
| E6003  | 配列インデックス範囲外           |
| E6004  | スタックオーバーフロー           |
| E6005  | アサーション失敗                 |
| E6006  | 関数が見つからない（ランタイム） |
| E6007  | ランタイムエラー                 |
| E6008  | キーが存在しない                 |
| E6009  | Range のステップ幅が不正         |
| E6010  | 整数解析失敗                     |
| E6011  | 浮動小数点解析失敗               |
| E6012  | 不正なコードポイント             |
| E6013  | JSON 解析失敗                    |

<!-- code-table:E6xxx end -->

> **コード表改訂（2026-08-09）**：コード表は元々 Rust 意味論ドラフト（Assertion failed/Arithmetic
> overflow/Heap allocation failed/Type cast
> failed）に基づいて定義されており、実装の実際の要件と一致しない。YaoXiang にはヌルポインタ/ヒープ割り当て失敗/型キャストという概念がない（値意味論 +
> Rust メモリ安全性）、ランタイムオーバーフロー経路は検出未実装。校正後：
>
> - E6002 削除（旧 Assertion failed は E6005 へ移動；旧ヌルポインタ意味論は言語に概念なし）
> - E6003 は Arithmetic overflow から Runtime index out of bounds へ変更（実際のトリガー面）
> - E6005 は Heap allocation failed から Assertion failed へ変更（std.assert の実際のパス）
> - E6006 は Runtime index out of bounds から Function not found へ変更（実装は既にこうなっている）
> - E6007 は Type cast failed から汎用 Runtime
>   error へ変更（ExecutorError の未マップ変体の統一着地点）

#### E7xxx：I/O とシステムエラー

<!-- code-table:E7xxx start -->

| コード | 説明                   |
| ------ | ---------------------- |
| E7001  | ファイルが見つからない |
| E7002  | アクセス拒否           |
| E7003  | I/O エラー             |
| E7004  | ネットワークエラー     |

<!-- code-table:E7xxx end -->

#### E8xxx：内部コンパイラエラー

<!-- code-table:E8xxx start -->

| コード | 説明                 |
| ------ | -------------------- |
| E8001  | 内部コンパイラエラー |
| E8002  | 予期しない Panic     |
| E8003  | コンパイラ段階エラー |

<!-- code-table:E8xxx end -->

#### W1xxx：警告コード

<!-- code-table:W1xxx start -->

| コード | 説明                                                   |
| ------ | ------------------------------------------------------ |
| W1001  | 未使用のプライベート関数                               |
| W1002  | 未使用のプライベート型                                 |
| W1003  | 未使用のインポート                                     |
| W1004  | 未使用のプライベート変数                               |
| W1005  | 未使用のプライベートメソッド                           |
| W1006  | ローカルモジュールによる依存パッケージのシャドーイング |
| W1063  | const ジェネリクス制約を評価できない                   |
| W1080  | コンパイル時証明の降格                                 |

<!-- code-table:W1xxx end -->

> W コード位置ルール：E コードと同形で段階別グループ化（W + 段階千位セグメント）、W1xxx
> = 型検査段階の警告。
>
> **デッドコード意味論（#321 決定 B）**：`pub`
> 定義は外部インタフェースであり、永久に報告しない——外部消費者が使用するかは単一ファイル解析の範囲を超えるため、誤検知より沈黙を選ぶ。W1001/W1002/W1004/W1005 は
> **プライベート（非 pub）定義** のみを対象とする：`main` と `pub`
> 定義から到達する参照解析で一度も参照されていなければ報告。メソッド（W1005）は呼び出し点の短縮名で照合する。bin/lib
> target セマンティクス（bin 内の未使用 pub 報告）は将来拡張（#289 プラン A）のため、プロジェクトモデルのサポートが必要。
>
> **未使用インポート（W1003）**：typecheck の use
> elaboration で検出（pass2 でインポートローカル名を登録し、式解析と型注釈位置でヒットしたものは使用済みと見なす）、全体インポート（`use std.io`
> → モジュール別名）と名前付きインポート（`use std.io.{print}`）の両方をカバー。
>
> **発火チャネル**：W コード診断はビルダーにより W プレフィックスでデフォルト `Severity::Warning`
> としてマークされる（明示指定優先）、収集と表示はエラーと同じ経路（`warning[W####]`
> プレフィックスレンダリング）、ただしコンパイルは中断せず、成功終了コードにも影響しない。`yaoxiang check --deny-warnings`
> は警告を失敗に昇格させる（警告が存在する場合、非ゼロコードで終了）、CI 厳格モード用。per-code 抑制（allow 属性など）は将来の拡張項目。

### メッセージ品質規範

> 本セクションはメッセージ単線化と品質改訂（2026-09-03）により導入された。`scripts/audit_diagnostics.py`
> により CI で強制執行される。

1. **メッセージ単線化**：すべてのユーザー可視診断メッセージは権威登録テーブルのショートカットメソッド +
   locales テンプレートレンダリングを経由する必要があり、コードは構造化パラメータのみを渡す。登録テーブルを迂回して
   `Diagnostic::error(...)`
   などの生の値を直接構築することは禁止——この経路はコード検証と i18n を迂回する。
2. **コード合法性**：未登録コードと擬似コード（例：`E_INTERNAL`）の使用は禁止；使用箇所のコードリテラルは登録テーブルで定義済みである必要がある。内部エラーは一律 E8001（`internal_error`）に着地。
3. **型表示**：型の Display はインスタンス化前後の形式を区別しなければならない（`Expected 'Container', found 'Container'`
   のベア名は区別不能）。
4. **ソルバー内部状態隔離**：ソルバー中間状態の TypeVar（Display 形式
   `t<N>`）はユーザー可視メッセージに入ってはならない。テスト固定：`test_type_error_message_no_solver_typevar_leak`。
5. **E8xxx 境界**：E8xxx はコンパイラ内部の一貫性问题（ICE）にのみ使用。ユーザーが修正可能なエラーは E8001 でのフォールバック禁止；ICE メッセージには最小再現手順を添付しなければならない。

---

### ランタイムエラー値とコードの貫通

> 本セクションはランタイム Error 値へのコード付与改訂（2026-09-03）により導入された。E6xxx/E7xxx 意味論空間は二つのチャネルを同時に担い、コード空間は同一、提示チャネルは異なる。

#### 二つのチャネル

| チャネル                     | キャリア                                                  | 提示方式                                             |
| ---------------------------- | --------------------------------------------------------- | ---------------------------------------------------- |
| コンパイラ/CLI 診断チャネル  | `ExecutorError` などのホスト層ハードエラー                | stderr `error[E####]:`（配線済み E6003/E6005/E6007） |
| プログラム内エラー値チャネル | std ライブラリ `Result(T, Error)` の Err キャリア `Error` | 言語値、プログラムが match/比較で消費                |

#### Error 構造（v0.8 から、破壊的変更）

```
Error { code: String, message: String }
```

- `code` は本仕様の E6xxx/E7xxx 番号を再利用、文字列形式（例：`"E6008"`）。
- **安定契約**：割り当て済みコードはバージョンを超えて意味論が変化しない；同じ意味論に対して削除済みコードを再利用しない（E6002 の前例）。
- **消費面**：プログラム内の `e.code == "E6xxx"`
  比較が唯一のプログラマブル判定契約；`yaoxiang explain E6xxx` ドキュメント貫通；ツールチェーン（LSP
  / DAP、RFC-034 参照）はコードを exceptionId として扱う。
- **アクセサ**：`std.result.code(e)` / `std.result.message(e)`。
- **ユーザー定義エラー**：`Result(T, E)`
  の E はジェネリクス引数であり、真剣にモデリングする場合はユーザー定義型で進める；std `Error`
  は単なる便利なフォールバックキャリアであり、そのコード体系はユーザーの E 型を制約しない。

#### コード割り当てルール

1. ランタイムエラー値コードとコンパイラ診断コードは E6xxx/E7xxx 空間を共有し、新しいコードは**実際のトリガー面**に基づいて割り当てる。想像上のシナリオに対する予約は行わない。
2. 先に登録してから使用：新しいコードは権威登録テーブルに入り、三方整合性検証（codes/*.rs ↔ locales
   ↔ 本ドキュメントコード表）を経た後に発火可能。ランタイムエラー値コードの登録ソースは
   `src/std/result.rs` の `RUNTIME_ERROR_CODES` テーブル（診断コードと同様に `build.rs` 構築期閾値 +
   `tools/code-tables` 検証を受ける）。
3. E7xxx は std.io / std.net エラー値用の予約セグメント（現在空状態、io/net Result 化時に有効化）。
4. 発火点：std 各モジュールは `error_new(code, message)` で Error 値を構築；消費側は
   `std.result.unwrap_err` で Err キャリアを取り出し、`std.result.code/message`
   でフィールドを読み取る。

#### 進化パス（ライン C、未実施）

パターンマッチの完備化（RFC-010b）実装後、`Error` は `{ kind: ErrorKind, message: String }`
にアップグレード可能で、`code`
は kind から派生する属性に変換される（変体定義箇所がコード登録テーブルになる）。進化期間中、本セクションの code 安定契約は不変；このアップグレードは独立した決定であり、本セクションの約束を構成しない。

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

##### 事前定義プレースホルダ（よく使用される）

| プレースホルダ | 用途                               | 例                                  |
| -------------- | ---------------------------------- | ----------------------------------- |
| `{name}`       | 変数名/型名/トレイト名などの識別子 | `Unknown variable: '{name}'`        |
| `{expected}`   | 期待型                             | `Expected type '{expected}'`        |
| `{found}`      | 実際/見つかった型                  | `, found type '{found}'`            |
| `{method}`     | メソッド名                         | `Method {method} is not a function` |
| `{trait}`      | トレイト名                         | `Cannot find trait: {trait}`        |
| `{path}`       | モジュールパス                     | `Invalid path: {path}`              |
| `{ty}`         | 型式                               | `Invalid type: {ty}`                |
| `{message}`    | 内部エラーメッセージ               | `Internal error: {message}`         |

##### 任意の key サポート

**params は任意の key をサポートし、事前定義に限定されない**。呼び出し側は任意の `key` を渡せる：

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

> **注意**：すべてのエラーコードがプレースホルダを使用するわけではない。コードの一部（例：E0001）は静的メッセージで、引数を必要としない。

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

#### ゼロテーブルルックアップコストの鍵

**レンダリングはコンパイル時に発生し、ランタイムではない。**

```
┌─────────────────────────────────────────────────────────────────────────┐
│  段階 1: Rust コンパイラが YaoXiang コンパイラをコンパイル                  │
│                                                                           │
│  JSON をコンパイラバイナリにバンドル                                       │
│  目的：explain コマンドが直接 i18n データを読み取れるように                  │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  段階 2: YaoXiang がユーザープロジェクトをコンパイル（ここでレンダリング）     │
│                                                                           │
│  error! マクロ呼び出し時：                                                │
│  1. yaoxiang.toml から言語設定を読み取り                                  │
│  2. コンパイラバイナリから対応言語の i18n JSON をロード                     │
│  3. テンプレート + 引数 → render() → "Unknown variable: 'x'"              │
│  4. Diagnostic.message = レンダリング済み文字列                            │
│                                                                           │
│  AOT バイナリには最終文字列が直接格納され、テンプレートもテーブル参照も不要  │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  段階 3: ユーザープログラム実行時                                          │
│                                                                           │
│  println!("{}", diagnostic.message)                                      │
│  // 最終文字列を直接出力、テーブル参照なし                                  │
└─────────────────────────────────────────────────────────────────────────┘
```

| コンポーネント               | 責務                             | レンダリング時期                   |
| ---------------------------- | -------------------------------- | ---------------------------------- |
| `I18nRegistry`               | テンプレートと表示テキストを提供 | ユーザープロジェクトのコンパイル時 |
| `DiagnosticBuilder.render()` | テンプレート + 引数 → 最終文字列 | ユーザープロジェクトのコンパイル時 |
| `Diagnostic.message`         | レンダリング済み文字列           | 最終結果を格納                     |
| AOT バイナリ                 | 最終文字列を含む                 | ランタイムで直接使用               |

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

エラーの重大度は `DiagnosticLevel` 列挙で管理され、エラーコード番号とは分離されている：

```rust
pub enum DiagnosticLevel {
    Error,    // 导致编译失败
    Warning,  // 不影响编译，但建议修复
    Note,     // 补充信息
    Help,     // 修复建议
}
```

| レベル  | 接頭辞            | 説明                     |
| ------- | ----------------- | ------------------------ |
| Error   | `error[E####]:`   | コンパイル失敗           |
| Warning | `warning[E####]:` | コンパイルには影響しない |
| Note    | `note[E####]:`    | 補足情報                 |
| Help    | `help[E####]:`    | 修正提案                 |

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
| `--json`        | JSON 形式出力（IDE/LSP 用）                 |
| `--json-pretty` | 整形済み JSON 出力                          |
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
3. 廃止タイムテーブルの提供

---

## 実施戦略

### 段階 1：エラーコード基盤アーキテクチャ

1. `src/diagnostics/` ディレクトリ構造の作成
2. `ErrorCode` 列挙の実装
3. `Diagnostic` と `DiagnosticLevel` の実装
4. リソースファイルディレクトリとサンプル JSON の作成

### 段階 2：explain コマンド

1. `yaoxiang explain` CLI コマンドの実装
2. `--lang` と `--json` オプションのサポート
3. リソースファイル読み込みの統合
4. 引数テンプレートレンダリングの実装

### 段階 3：コンパイル時統合

1. すべてのエラー報告箇所を新システムに更新
2. メッセージテンプレート引数注入の実装
3. 言語優先度ロジックの追加
4. ユニットテストカバレッジ

### 段階 4：IDE/LSP 統合

1. LSP サーバーへの explain JSON 出力統合
2. IDE でのエラーコードリンク表示
3. ホバー時のエラー説明表示
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
| E6xxx | ランタイムエラー       |
| E7xxx | I/O とシステムエラー   |
| E8xxx | 内部コンパイラエラー   |
| E9xxx | 予約                   |

### サポートされている言語

| コード | 言語         | 状態       |
| ------ | ------------ | ---------- |
| en-US  | English (US) | デフォルト |
| zh-CN  | 简体中文     | 計画中     |

### エラーメッセージサンプル比較

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

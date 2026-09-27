---
title: 'RFC 013: エラーコード仕様'
status: '受領'
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

本RFCは、YaoXiangコンパイラのエラーコード分類仕様を提案する。Rustに類似した単層番号システムを採用し、JSONリソースファイルによる多言語サポートを実現、`yaoxiang explain`コマンドでエラー説明機能を提供する。

## 動機

### なぜ標準化されたエラーコードが必要なのか？

1. **ユーザー体験**：エラーコードを見ることで、ユーザーはエラーの種類や重大度を迅速に判断できる
2. **ドキュメント構成**：カテゴリ別にグループ化することで、エラーリファレンスドキュメントの作成と保守が容易になる
3. **ツール統合**：IDE/LSPがエラーコードに基づいてクイックフィックス提案やドキュメントリンクを提供できる
4. **国際化対応**：エラーメッセージとコードを分離することで、多言語翻訳が容易になる

### 設計目標

- **簡潔**：単層番号方式で、複雑な分類ルールを覚える必要がない
- **親しみやすい**：Rustに類似したエラーメッセージ形式、ヘルプ情報と例を添付
- **拡張可能**：リソースファイル駆動で、新しいエラーや新しい言語の追加が容易
- **ツールフレンドリー**：explainコマンド + JSON出力で、IDE/LSP統合をサポート

---

## 提案

### 中核設計：単層番号システム

4桁の数字番号を採用し、コンパイル段階でグループ化する：

```
Exxxx
││││
│││└── シーケンス番号 (000-999)
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
| **4** | E4xxx | ジェネリクスとtrait    |
| **5** | E5xxx | モジュールとインポート |
| **6** | E6xxx | ランタイムエラー       |
| **7** | E7xxx | I/Oとシステムエラー    |
| **8** | E8xxx | 内部コンパイラエラー   |
| **9** | E9xxx | 予約済み/実験的        |

### エラーカテゴリ enum

```rust
/// エラーカテゴリ
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

**基本原則**：エラーコード定義と表示テキストを分離する

- `ErrorCodeDefinition`：エラーコードのメタデータ（code、category、template）、表示テキストを含まない
- `locales/*.json`：各言語の表示テキスト（title、message、help、エラーコードはネストされたオブジェクト）
- `DiagnosticBuilder`：汎用ビルダー、trait-per-error設計の代替

#### エラーコード定義

```rust
// diagnostic/codes/mod.rs

use crate::util::span::Span;
use crate::util::diagnostic::{Diagnostic, Severity};

/// エラーコード定義（メタデータのみ、表示テキストはi18nファイルに）
#[derive(Debug, Clone, Copy)]
pub struct ErrorCodeDefinition {
    pub code: &'static str,
    pub category: ErrorCategory,
    pub message_template: &'static str,  // メッセージテンプレート、{param}プレースホルダをサポート
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

    /// Diagnosticを構築（テンプレートレンダリングはコンパイル時に完了）
    pub fn build(&self, i18n: &I18nRegistry) -> Diagnostic {
        // テンプレート内のすべての{key}に対応するパラメータがあることを確認
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

#### 設計上の優位性

| 特性                             | 説明                                                        |
| -------------------------------- | ----------------------------------------------------------- |
| **単一Builder**                  | 1つの`DiagnosticBuilder`ですべてのエラーコードに汎用対応    |
| **型安全**                       | ショートカットメソッドがパラメータの正確性を保証            |
| **自己文書化**                   | `E1001::unknown_variable(name)`で一目瞭然                   |
| **テンプレート分離**             | メッセージテンプレートとコードが分離されており、i18nが容易  |
| **ランタイムオーバーヘッドなし** | コンパイル時レンダリング、AOTバイナリにはテーブル検索が不要 |

---

### エラーマクロの簡素化

#### error!マクロ（コンテキストの自動注入）

```rust
/// コンパイル時にspanとi18n設定を自動取得するマクロ
macro_rules! error {
    ($code:ident, $($key:ident = $value:expr),* $(,)?) => {
        $code()
            $(.$key($value))*
            .at(crate::util::span::Span::current())
            .build(crate::util::diagnostic::I18nRegistry::current())
    };
}

/// 使用法：パラメータのみ渡す、spanとi18nは自動注入
return Err(error!(E1001, name = var_name));
return Err(error!(E1002, expected = "bool", found = cond_ty));
```

#### 手動でBuilderを使用

```rust
// 手動制御が必要な場合
E1001::unknown_variable(&var_name)
    .at(my_span)           // カスタムspan
    .build(&custom_i18n)   // カスタムi18n
```

---

## 詳細設計

### エラーコード一覧

#### E0xxx：字句解析と構文解析

<!-- code-table:E0xxx start -->

| コード | 説明                       |
| ------ | -------------------------- |
| E0001  | 無効な文字                 |
| E0002  | 無効な数字リテラル         |
| E0003  | 終了していない文字列       |
| E0004  | 無効な文字リテラル         |
| E0010  | 期待されるトークン         |
| E0011  | 予期しないトークン         |
| E0012  | 無効な構文                 |
| E0013  | 一致しない括弧             |
| E0014  | セミコロンの欠落           |
| E0016  | 期待される式               |
| E0018  | キーワードが名前として使用 |

<!-- code-table:E0xxx end -->

#### E1xxx：型検査

<!-- code-table:E1xxx start -->

| コード | 説明                                                       |
| ------ | ---------------------------------------------------------- |
| E1001  | 未知の変数                                                 |
| E1002  | 型の不一致                                                 |
| E1003  | 未知の型                                                   |
| E1010  | 引数の数が一致しない                                       |
| E1011  | 引数の型が一致しない                                       |
| E1012  | 戻り型が一致しない                                         |
| E1013  | 関数が見つからない                                         |
| E1014  | 名前付き引数名が未知                                       |
| E1015  | 引数の重複指定                                             |
| E1020  | 型を推論できない                                           |
| E1021  | 型推論の競合                                               |
| E1030  | パターンが不完全                                           |
| E1031  | 到達不能なパターン                                         |
| E1032  | パターンの重複バインド                                     |
| E1033  | ORパターン束縛の不一致                                     |
| E1034  | 構造体パターンにフィールド不足                             |
| E1040  | 操作がサポートされていない                                 |
| E1041  | インデックスが範囲外                                       |
| E1042  | フィールドが見つからない                                   |
| E1050  | ブールオペランドが必要                                     |
| E1051  | 論理NOTにはブールオペランドが必要                          |
| E1052  | 無効なデリファレンス                                       |
| E1053  | 非構造体フィールドアクセス                                 |
| E1054  | 条件型が一致しない                                         |
| E1055  | 非ジェネリックコンテキストでの制約                         |
| E1060  | 型パラメータの数が一致しない                               |
| E1061  | ジェネリクスをインスタンス化できない                       |
| E1062  | constジェネリクス制約の失敗                                |
| E1064  | バインド位置のインデックスが無効                           |
| E1065  | 非関数値の呼び出し                                         |
| E1071  | 型定義はモジュールレベルでのみ可能                         |
| E1081  | `?`は伝播可能型を返す関数内でのみ使用可能                  |
| E1082  | `?`はTryを実装する型にのみ使用可能                         |
| E1083  | `?`のエラー型が一致しない                                  |
| E1090  | ✨ 語りに難く ✨                                           |
| E1091  | 無効なジェネリックメタ型                                   |
| E1092  | 精化型引数の形式が不正                                     |
| E1093  | 精化引数の数が一致しない                                   |
| E1094  | 未使用のコンパイル時値パラメータ                           |
| E1095  | 未知のインターフェース                                     |
| E1096  | インターフェースパラメータの数が一致しない                 |
| E1097  | インターフェースメンバーの名前競合                         |
| E1098  | インターフェースメソッドが未実装                           |
| E1099  | インターフェースメソッドのシグネチャが一致しない           |
| E1100  | インターフェースメソッドの重複実装                         |
| E1101  | 型がインターフェースを実装していない                       |
| E1102  | ループ外でのループ制御文                                   |
| E1103  | 型位置に角括弧は使用不可                                   |
| E1104  | インターフェース実装が型の定義モジュールにない             |
| E1105  | バリアントコンストラクタはフィールドアクセスとして使用不可 |
| E1106  | 制約が満たされていない                                     |
| E1107  | メソッドオーバーロードの曖昧性                             |

<!-- code-table:E1xxx end -->

> **RFC-011b関連（2026-09-22注）**：[RFC-011b: 演算子オーバーロード](./011b-operator-overloading.md)
> の実装時には、本セクションの3か所が影響を受ける——① `E1081` /
> `E1082`のテキストから "Result" の字句を削除（`?`は`Try`インタフェースによる判定に変更し、特定の型名に束縛しない）、フェーズ2にて本ドキュメントの「三方一貫性」フローに従って同期（codes/*.rs
> ↔ locales ↔ コード表）；②
> `Equal`前置制約（線形トークン）が満たされない場合の拒否診断には`E1101`（型がインターフェースを実装していない）ファミリを再利用；③ フェーズ1の結線後、`Struct == Struct`は`E6007`ランタイムエラーからコンパイル時判定に移行し、`E6007`のトリガー範囲が縮小する。表内の登録テキストは実装が着地するまで現状を維持する。

#### E2xxx：意味解析

<!-- code-table:E2xxx start -->

| コード | 説明                                     |
| ------ | ---------------------------------------- |
| E2001  | スコープエラー                           |
| E2002  | 重複定義                                 |
| E2003  | 所有権エラー                             |
| E2010  | 不変変数への代入                         |
| E2011  | 未初期化変数の使用                       |
| E2012  | 可変性の競合                             |
| E2013  | 変数のシャドーイング                     |
| E2014  | ムーブ後の値の使用                       |
| E2016  | 不変変数への代入                         |
| E2018  | 可変/不変借用の競合                      |
| E2019  | 二重解放                                 |
| E2020  | 解放後の使用                             |
| E2027  | unsafeデリファレンス                     |
| E2029  | spawn内の参照ループ                      |
| E2030  | 精化型制約違反                           |
| E2090  | 無効なシグネチャ                         |
| E2091  | シグネチャに未知の型                     |
| E2092  | シグネチャに矢印がない                   |
| E2093  | 重複する引数名                           |
| E2094  | ジェネリクスパラメータのシャドーイング   |
| E2095  | 引数名によるジェネリクスのシャドーイング |

<!-- code-table:E2xxx end -->

> 予約コードの説明（2026-09-14棚卸し、#251リリース版準拠）：E2019（二重解放）、E2020（解放後の使用）、E2027（unsafeデリファレンス）、E2029（spawn内ref循環）は登録完了かつユニットテストでアンカーされているが、現在のところ到達可能なyxソースの表層パス（明示的drop文、Ptrデリファレンス文法、spawn
> refループ構築パス）がない——意味的に完全に正しいという主張はこれら4コードには及ばず、実装が補完されるまでは「予約済み」として扱う。

#### E3xxx：コード生成

<!-- code-table:E3xxx start -->

| コード | 説明                                                 |
| ------ | ---------------------------------------------------- |
| E3004  | サポートされていないイテレータ                       |
| E3005  | IR生成エラー                                         |
| E3006  | 未解決変数                                           |
| E3007  | トップレベルバインドの初期化は定数でなければならない |
| E3008  | サポートされていないmatchパターン                    |
| E3014  | レジスタオーバーフロー                               |
| E3017  | 無効なオペランド（コード生成）                       |
| E3018  | 単相化インスタンス化失敗                             |
| E3019  | トップレベルバインドの循環依存                       |
| E3020  | プログラムエントリがない                             |
| E3021  | エントリが関数ではない                               |
| E3022  | エントリmainのシグネチャが一致しない                 |
| E3023  | トップレベルで実行文は許可されない                   |

<!-- code-table:E3xxx end -->

#### E4xxx：ジェネリクスとtrait

<!-- code-table:E4xxx start -->

| コード | 説明                             |
| ------ | -------------------------------- |
| E4001  | ジェネリクス制約違反             |
| E4002  | traitが見つからない              |
| E4003  | trait実装の欠落                  |
| E4004  | trait実装の競合                  |
| E4005  | 関連型が見つからない             |
| E4010  | 定数のゼロ除算                   |
| E4011  | 定数オーバーフロー               |
| E4012  | 定数の再帰が深すぎる             |
| E4014  | 定数評価失敗                     |
| E4018  | 精化述語違反                     |
| E4019  | 型等式が成立しない               |
| E4020  | 証明関数が必要                   |
| E4021  | ループの停止性を自動証明できない |

<!-- code-table:E4xxx end -->

> E4006/E8004は現在発射点なし（予約コード）：Sized制約と最適化エラーパスの実装が未着手の状態、実装時に実際のトリガー面に従って結線する。

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
| E6009  | Rangeステップが無効              |
| E6010  | 整数解析失敗                     |
| E6011  | 浮動小数点解析失敗               |

<!-- code-table:E6xxx end -->

> **コード表改訂（2026-08-09）**：コード表は元来Rustセマンティクスのドラフト（Assertion
> failed/Arithmetic overflow/Heap allocation failed/Type cast
> failed）に基づいて定義されており、実装の実際のニーズと一致していなかった。YaoXiangにはヌルポインタ/ヒープ割り当て失敗/型キャストという概念がなく（値セマンティクス +
> Rustメモリ安全性）、ランタイムオーバーフローパスの検出も実装されていない。校正後：
>
> - E6002 削除（旧Assertion failedはE6005へ移動；旧ヌルポインタのセマンティクスは言語に概念なし）
> - E6003 をArithmetic overflowからランタイムインデックス範囲外に変更（実際のトリガー面）
> - E6005 をHeap allocation failedからAssertion failedに変更（std.assertの実際のパス）
> - E6006 をランタイムインデックス範囲外から関数が見つからないに変更（実装は以前からこの通り）
> - E6007 をType cast
>   failedから汎用ランタイムエラーに変更（ExecutorErrorのマッピングされていないバリアントの統一落点）

#### E7xxx：I/Oとシステムエラー

<!-- code-table:E7xxx start -->

| コード | 説明                   |
| ------ | ---------------------- |
| E7001  | ファイルが見つからない |
| E7002  | 権限が拒否されました   |
| E7003  | I/Oエラー              |
| E7004  | ネットワークエラー     |

<!-- code-table:E7xxx end -->

#### E8xxx：内部コンパイラエラー

<!-- code-table:E8xxx start -->

| コード | 説明                     |
| ------ | ------------------------ |
| E8001  | 内部コンパイラエラー     |
| E8002  | 予期しないpanic          |
| E8003  | コンパイラフェーズエラー |

<!-- code-table:E8xxx end -->

#### W1xxx：警告コード

<!-- code-table:W1xxx start -->

| コード | 説明                                |
| ------ | ----------------------------------- |
| W1001  | 未使用のプライベート関数            |
| W1002  | 未使用のプライベート型              |
| W1003  | 未使用のインポート                  |
| W1004  | 未使用のプライベート変数            |
| W1005  | 未使用のプライベートメソッド        |
| W1063  | constジェネリクス制約を評価できない |
| W1080  | コンパイル時証明の降格              |

<!-- code-table:W1xxx end -->

> Wコード位置ルール：Eコードと同形で段階別にグループ化（W+段階千位セグメント）、W1xxx
> = 型検査段階の警告。
>
> **デッドコードコード定義（#321決定案B）**：`pub`定義は外部インターフェースであり、永久に報告しない——外部消費者が使用するかどうかは単一ファイル解析の範囲を超えるため、誤報しないよう沈黙させる方が望ましい。W1001/W1002/W1004/W1005は
> **プライベート（pubでない）定義のみ**を対象とする：`main`と`pub`定義から到達可能性解析を開始し、参照されない場合に報告する。メソッド（W1005）は呼び出し点の短い名前で照合する。bin/libターゲットのセマンティクス（bin内で未使用のpubに対する警告）は将来の拡張（#289案A）であり、プロジェクトモデルのサポートが必要。
>
> **未使用インポート（W1003）**：typecheckのuse
> elaborationで検出（パス2でインポートのローカル名を登録し、式解析と型注釈位置でヒットすれば使用済みと見なす）、全体インポート（`use std.io`
> → モジュール別名）と名前付きインポート（`use std.io.{print}`）の両方をカバーする。
>
> **発射チャネル**：Wコード診断はbuilderがWプレフィックスでデフォルト`Severity::Warning`を付与し（明示指定が優先）、収集と表示はエラーと同一の経路で行われ（`warning[W####]`プレフィックスでレンダリング）、コンパイルをブロックせず成功終了コードにも影響しない。`yaoxiang check --deny-warnings`は警告を失敗に昇格させ（警告が存在する場合に非ゼロコードで終了）、CI厳格モード用。コード別の抑制（allow属性など）は後続の拡張項目。

### メッセージ品質仕様

> 本セクションはメッセージ単一化と品質改訂（2026-09-03）によって導入された。`scripts/audit_diagnostics.py`がCIで強制実行する。

1. **メッセージ単一化**：すべてのユーザー可視診断メッセージは、権威あるレジストリのショートカットメソッド +
   localesテンプレートのレンダリングを経由しなければならず、コードは構造化パラメータのみを渡す。レジストリを迂回して`Diagnostic::error(...)`などの生の値を直接構築することは禁止——このパスはコード検証とi18nをバイパスする。
2. **コード合法性**：未登録コードとフェイクコード（例：`E_INTERNAL`）の使用は禁止；使用箇所のコードリテラルはレジストリで定義済みである必要がある。内部エラーはすべてE8001（`internal_error`）にフォールバックする。
3. **型表示**：型のDisplayはインスタンス化前後の形式を区別しなければならない（`Expected 'Container', found 'Container'`のようなベア名では区別できない）。
4. **ソルバー内部状態の隔離**：ソルバーの中間状態TypeVar（Display形式`t<N>`）はユーザー可視メッセージに漏らしてはならない。テストアンカー：`test_type_error_message_no_solver_typevar_leak`。
5. **E8xxx境界**：E8xxxはコンパイラの内部一貫性の問題（ICE）にのみ使用する。ユーザーが修正可能なエラーにE8001をフォールバックとして使用することは禁止；ICEメッセージには最小限の再現手順を添付しなければならない。

---

### ランタイムエラー値とコード貫通

> 本セクションはランタイムError値コード化改訂（2026-09-03）によって導入された。E6xxx/E7xxxのセマンティクス空間は2つのチャネルを担い、コード空間は同一、提示チャネルが異なる。

#### 2つのチャネル

| チャネル                     | キャリア                                            | 提示方式                                            |
| ---------------------------- | --------------------------------------------------- | --------------------------------------------------- |
| コンパイラ/CLI診断チャネル   | `ExecutorError`などのホスト層のハードエラー         | stderr `error[E####]:`（E6003/E6005/E6007結線済み） |
| プログラム内エラー値チャネル | stdライブラリ`Result(T, Error)`のErrキャリア`Error` | 言語値、プログラムがmatch/比較で消費                |

#### Error構造（v0.8以降、破壊的変更）

```
Error { code: String, message: String }
```

- `code`は本仕様のE6xxx/E7xxx番号を再利用、文字列形式（例：`"E6008"`）。
- **安定契約**：割り当て済みのコードはバージョンを超えてセマンティクスが不変；同一セマンティクスに削除されたコード（E6002の先例）を再利用しない。
- **消費面**：プログラム内の`e.code == "E6xxx"`比較が唯一のプログラマブル判定契約；`yaoxiang explain E6xxx`でドキュメント貫通；ツールチェーン（LSP
  / DAP、RFC-034参照）はコードをexceptionIdとする。
- **アクセサ**：`std.result.code(e)` / `std.result.message(e)`。
- **ユーザー定義エラー**：`Result(T, E)`のEはジェネリックパラメータ、真剣にモデリングする場合はユーザー定義型；std
  `Error`は便利なフォールバックキャリアに過ぎず、そのコード体系はユーザーE型を制約しない。

#### コード割り当てルール

1. ランタイムエラー値コードとコンパイラ診断コードはE6xxx/E7xxx空間を共有し、新しいコードは**実際のトリガー面**に従って割り当て、想像上のシナリオのために予約しない。
2. 登録してから使用：新しいコードは権威あるレジストリに登録され、三方一貫性検証（codes/*.rs ↔
   locales
   ↔ 本ドキュメントのコード表）を経た後にのみ発射可能。ランタイムエラー値コードの登録ソースは`src/std/result.rs`の`RUNTIME_ERROR_CODES`テーブル（診断コードと同様に`build.rs`構築時の閾値 +
   `tools/code-tables`検証の対象）。
3. E7xxxはstd.io /
   std.netのエラー値のためにセグメントを予約（現在は空き、io/netのResult化時に有効化）。
4. 発射点：stdの各モジュールは`error_new(code, message)`でError値を構築；消費側は`std.result.unwrap_err`でErrキャリアを取り出し、`std.result.code/message`でフィールドを読み取る。

#### 進化パス（ラインC、未実施）

パターンマッチング完備化（RFC-010b）の実装後、`Error`は`{ kind: ErrorKind, message: String }`にアップグレード可能、`code`はkindから派生したプロパティに変換される（バリアント定義箇所がコードレジストリ）。進化期間中、本セクションのコード安定契約は不変を維持；このアップグレードは独立した決定であり、本セクションの約束を構成しない。

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
// locales/*.json（エラーコードオブジェクト）

/// i18n表示テキストレジストリ（コンパイル時にJSONからロード、ランタイムはゼロテーブル検索）
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

    /// テンプレートをレンダリング（コンパイル時に完了、ランタイムオーバーヘッドなし）
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

##### 事前定義プレースホルダ（よく使用されるもの）

| プレースホルダ | 用途                            | 例                                  |
| -------------- | ------------------------------- | ----------------------------------- |
| `{name}`       | 変数名/型名/trait名などの識別子 | `Unknown variable: '{name}'`        |
| `{expected}`   | 期待される型                    | `Expected type '{expected}'`        |
| `{found}`      | 実際/見つかった型               | `, found type '{found}'`            |
| `{method}`     | メソッド名                      | `Method {method} is not a function` |
| `{trait}`      | trait名                         | `Cannot find trait: {trait}`        |
| `{path}`       | モジュールパス                  | `Invalid path: {path}`              |
| `{ty}`         | 型式                            | `Invalid type: {ty}`                |
| `{message}`    | 内部エラーメッセージ            | `Internal error: {message}`         |

##### 任意のキーサポート

**paramsは任意のキーをサポートし、事前定義に限定されない**。呼び出し側は任意の`key`を渡すことができる：

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

> **注意**：すべてのエラーコードがプレースホルダを使用するわけではない。一部のエラーコード（例：E0001）は静的メッセージで、パラメータは不要。

#### 言語優先度

```
1. yaoxiang.toml [language.default]
2. ~/.yaoxiang/yaoxiang.toml [language.default]
3. デフォルト値: en
```

### yaoxiang.toml設定

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

#### コンパイル時の言語選択

```
1. プロジェクトレベルyaoxiang.tomlのlanguage.defaultを読み取る
2. 未設定の場合、ユーザーレベル~/.yaoxiang/yaoxiang.tomlを読み取る
3. どちらも未設定の場合、デフォルトで "en" を使用
4. コンパイラは選択された言語に基づいてI18nRegistryを作成（1回）
5. すべてのエラーはそのI18nRegistryを使用してメッセージをレンダリング
```

#### ゼロテーブル検索オーバーヘッドの鍵

**レンダリングはユーザープロジェクトのコンパイル時に発生し、ランタイムではない。**

```
┌─────────────────────────────────────────────────────────────────────────┐
│  フェーズ 1: RustコンパイラがYaoXiangコンパイラをコンパイル                                       │
│                                                                           │
│  JSONはコンパイラバイナリにパッケージ化される                                             │
│  目的：explainコマンドがi18nデータを直接読み取れるようにする                                       │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  フェーズ 2: YaoXiangがユーザープロジェクトをコンパイル（ここでレンダリングが発生）                        │
│                                                                           │
│  error!マクロ呼び出し時：                                                            │
│  1. yaoxiang.tomlを読み取り言語設定を取得                                                │
│  2. コンパイラバイナリから対応する言語のi18n JSONをロード                                          │
│  3. テンプレート + パラメータ → render() → "Unknown variable: 'x'"                          │
│  4. Diagnostic.message = レンダリング済み文字列                                             │
│                                                                           │
│  AOTバイナリは最終文字列を直接格納、テンプレートやテーブル検索は不要                              │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  フェーズ 3: ユーザープログラム実行時                                                      │
│                                                                           │
│  println!("{}", diagnostic.message)                                              │
│  // 最終文字列を直接出力、テーブル検索はなし                                                  │
└─────────────────────────────────────────────────────────────────────────┘
```

| コンポーネント               | 役割                                   | レンダリングタイミング             |
| ---------------------------- | -------------------------------------- | ---------------------------------- |
| `I18nRegistry`               | テンプレートと表示テキストを提供       | ユーザープロジェクトのコンパイル時 |
| `DiagnosticBuilder.render()` | テンプレート + パラメータ → 最終文字列 | ユーザープロジェクトのコンパイル時 |
| `Diagnostic.message`         | レンダリング済み文字列                 | 最終結果を格納                     |
| AOTバイナリ                  | 最終文字列を含む                       | ランタイムに直接使用               |

---

### エラーメッセージ形式

エラーメッセージは以下の形式を採用する：

```
error[E####]: <簡単な説明>
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

エラーの重大度は`DiagnosticLevel` enumで管理され、エラーコード番号とは分離されている：

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

### `yaoxiang explain`コマンド

#### コマンド構文

```bash
yaoxiang explain <ERROR_CODE> [OPTIONS]
```

#### オプション

| オプション      | 説明                                        |
| --------------- | ------------------------------------------- |
| `--lang <code>` | 言語を指定 (en-US, zh-CN、デフォルト en-US) |
| `--json`        | JSON形式の出力（IDE/LSPが利用）             |
| `--json-pretty` | フォーマットされたJSON出力                  |
| `--examples`    | サンプルコードのみを表示                    |
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

# JSON出力（LSP統合）
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

本RFCはエラーコードシステムをゼロから設計するため、後方互換性の問題はない。

**将来のマイグレーション戦略**（後続バージョン参考用）：

1. 旧エラーコードと新エラーコードのマッピングを保持
2. マイグレーション期間中は新旧両方のコードを表示
3. 廃止タイムテーブルを提供

---

## 実装戦略

### フェーズ1：エラーコードインフラストラクチャ

1. `src/diagnostics/`ディレクトリ構造の作成
2. `ErrorCode` enumの実装
3. `Diagnostic`と`DiagnosticLevel`の実装
4. リソースファイルディレクトリとサンプルJSONの作成

### フェーズ2：explainコマンド

1. `yaoxiang explain` CLIコマンドの実装
2. `--lang`と`--json`オプションのサポート
3. リソースファイル読み込みの統合
4. パラメータテンプレートのレンダリング実装

### フェーズ3：コンパイル時統合

1. すべてのエラー報告ポイントを新システムを使用するように更新
2. メッセージテンプレートパラメータ注入の実装
3. 言語優先度ロジックの追加
4. ユニットテストカバレッジ

### フェーズ4：IDE/LSP統合

1. LSPサーバーがexplain JSON出力を統合
2. IDEにエラーコードリンクを表示
3. ホバーでエラー説明を表示
4. クイックフィックス提案

---

## 付録

### 完全エラーコード早見表

| 範囲  | カテゴリ               |
| ----- | ---------------------- |
| E0xxx | 字句解析と構文解析     |
| E1xxx | 型検査                 |
| E2xxx | 意味解析               |
| E3xxx | コード生成             |
| E4xxx | ジェネリクスとtrait    |
| E5xxx | モジュールとインポート |
| E6xxx | ランタイムエラー       |
| E7xxx | I/Oとシステムエラー    |
| E8xxx | 内部コンパイラエラー   |
| E9xxx | 予約済み               |

### サポートされる言語

| コード | 言語         | 状態       |
| ------ | ------------ | ---------- |
| en-US  | English (US) | デフォルト |
| zh-CN  | 简体中文     | 計画中     |

### エラーメッセージ例の比較

```
# 英語 (en-US)
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

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

本 RFC は YaoXiang コンパイラのエラーコード分類仕様を提案する。Rust 風の単層番号システムを採用しつつ、JSON リソースファイルによって多言語対応を実現し、`yaoxiang explain`
コマンドによりエラー解説機能を提供する。

## 動機

### なぜ標準化されたエラーコードが必要なのか？

1. **ユーザー体験**：エラーコードを見ることで、ユーザーはエラーの種類や重大度を迅速に判断できる
2. **ドキュメント構成**：カテゴリ別にグループ化することで、エラー参考ドキュメントの記述と保守が容易になる
3. **ツール統合**：IDE/LSP がエラーコードに基づいてクイックフィックス提案やドキュメントリンクを提供できる
4. **国際化対応**：エラーメッセージとコードを分離することで、多言語翻訳が容易になる

### 設計目標

- **簡潔**：単層番号方式で、複雑な分類ルールを覚える必要がない
- **親しみやすい**：Rust 風のエラーメッセージ形式、ヘルプ情報とサンプル付き
- **拡張可能**：リソースファイル駆動で、新たなエラーや言語の追加が容易
- **ツールフレンドリー**：explain コマンド + JSON 出力で、IDE/LSP 統合をサポート

---

## 提案

### コア設計：単層番号システム

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
| **0** | E0xxx | 字句・構文解析         |
| **1** | E1xxx | 型チェック             |
| **2** | E2xxx | 意味解析               |
| **3** | E3xxx | コード生成             |
| **4** | E4xxx | 泛型と特質             |
| **5** | E5xxx | モジュールとインポート |
| **6** | E6xxx | ランタイムエラー       |
| **7** | E7xxx | I/O とシステムエラー   |
| **8** | E8xxx | 内部コンパイラエラー   |
| **9** | E9xxx | 予約／実験的           |

### エラーカテゴリ enum

```rust
/// エラーカテゴリ
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    Lexer,      // E0xxx: 字句と構文解析
    Parser,     // E0xxx: Parser errors
    TypeCheck,  // E1xxx: 型チェック
    Semantic,   // E2xxx: 意味解析
    Generic,    // E4xxx: 泛型と特質
    Module,     // E5xxx: モジュールとインポート
    Runtime,    // E6xxx: ランタイムエラー
    Io,         // E7xxx: I/Oとシステムエラー
    Internal,   // E8xxx: 内部コンパイラエラー
}
```

### エラーコード定義と汎用 Builder

**基本原則**：エラーコード定義と表示テキストの分離

- `ErrorCodeDefinition`：エラーコードのメタデータ（code、category、template）を含み、表示テキストは含まない
- `locales/*.json`：各言語の表示テキスト（title、message、help、エラーコードはネストされたオブジェクト）
- `DiagnosticBuilder`：汎用ビルダー、trait-per-error 設計を代替する

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
    pub message_template: &'static str,  // メッセージテンプレート、{param} プレースホルダをサポート
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
        // テンプレート内のすべての {key} に対応する引数があるかチェック
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

| 特性                     | 説明                                                     |
| ------------------------ | -------------------------------------------------------- |
| **単一 Builder**         | 一つの `DiagnosticBuilder` ですべてのエラーコードに対応  |
| **型安全**               | ショートカットメソッドが引数の正確性を保証               |
| **自己文書化**           | `E1001::unknown_variable(name)` で一目瞭然               |
| **テンプレート分離**     | メッセージテンプレートとコードが分離され、i18n が容易    |
| **ゼロランタイムコスト** | コンパイル時レンダリング、AOT バイナリにテーブル参照なし |

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
};

/// 使用：引数のみ渡せば、span と i18n が自動注入される
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

#### E0xxx：字句と構文解析

<!-- code-table:E0xxx start -->

| コード | 説明                       |
| ------ | -------------------------- |
| E0001  | 無効な文字                 |
| E0002  | 無効な数値リテラル         |
| E0003  | 未終端の文字列             |
| E0004  | 無効な文字リテラル         |
| E0010  | 期待されたトークン         |
| E0011  | 予期しないトークン         |
| E0012  | 無効な構文                 |
| E0013  | 一致しない括弧             |
| E0014  | セミコロン欠落             |
| E0016  | 式が期待される             |
| E0018  | キーワードが名前として使用 |

<!-- code-table:E0xxx end -->

#### E1xxx：型チェック

<!-- code-table:E1xxx start -->

| コード | 説明                                         |
| ------ | -------------------------------------------- |
| E1001  | 未知の変数                                   |
| E1002  | 型の不一致                                   |
| E1003  | 未知の型                                     |
| E1010  | 引数の数が一致しない                         |
| E1011  | 引数の型が一致しない                         |
| E1012  | 戻り値の型が一致しない                       |
| E1013  | 関数が見つからない                           |
| E1014  | 未知の命名引数名                             |
| E1015  | 引数の重複指定                               |
| E1020  | 型を推論できない                             |
| E1021  | 型推論の競合                                 |
| E1030  | パターンが不完全                             |
| E1031  | 到達不可能なパターン                         |
| E1032  | パターンの重複バインド                       |
| E1033  | OR パターンのバインド不一致                  |
| E1034  | 構造体パターンにフィールド不足               |
| E1040  | 操作がサポートされていない                   |
| E1041  | インデックス範囲外                           |
| E1042  | フィールドが見つからない                     |
| E1050  | ブール被演算子が必要                         |
| E1051  | 論理 NOT にはブール被演算子が必要            |
| E1052  | 無効な deref                                 |
| E1053  | 非構造体フィールドアクセス                   |
| E1054  | 条件型の不一致                               |
| E1055  | 非泛型コンテキストでの制約                   |
| E1060  | 型引数の数が一致しない                       |
| E1061  | 泛型をインスタンス化できない                 |
| E1062  | const 泛型制約の失敗                         |
| E1064  | バインド位置のインデックスが無効             |
| E1065  | 非関数値の呼び出し                           |
| E1071  | 型定義はモジュールレベルでのみ可能           |
| E1081  | `?` は伝播可能型を返す関数内でのみ使用可     |
| E1082  | `?` は Try を実装した型にのみ使用可          |
| E1083  | `?` のエラー型が一致しない                   |
| E1090  | ✨ 名状しがたい ✨                           |
| E1091  | 無効な泛型のメタ型                           |
| E1092  | 精化型実引数の形式が不正                     |
| E1093  | 精化実引数の数が一致しない                   |
| E1094  | 未使用のコンパイル時値引数                   |
| E1095  | 未知のインタフェース                         |
| E1096  | インタフェース引数の数が一致しない           |
| E1097  | インタフェースメンバーの名前競合             |
| E1098  | インタフェースメソッド未実装                 |
| E1099  | インタフェースメソッドのシグネチャ不一致     |
| E1100  | インタフェースメソッド重複実装               |
| E1101  | 型がインタフェースを実装していない           |
| E1102  | ループ制御文がループ外に出現                 |
| E1103  | 型位置に角括弧は使用不可                     |
| E1104  | インタフェース実装が型の定義モジュールにない |
| E1105  | 変体コンストラクタはフィールドアクセス不可   |
| E1106  | 制約が満たされていない                       |
| E1107  | メソッドオーバーロードの曖昧さ               |

<!-- code-table:E1xxx end -->

> **RFC-011b 関連（2026-09-22 注）**：[RFC-011b: 演算子オーバーロード](./011b-operator-overloading.md)
> 実装時に本節の 3 箇所に関わる——① `E1081` / `E1082` 文言から「Result」表記を削除（`?` を `Try`
> インタフェース判定に変更し、具体的な型名に拘束しない）、フェーズ 2 にて本ドキュメントの「三方一貫性」フローに従い同期（codes/*.rs
> ↔ locales ↔ コード表）；② `Equal` 前置制約（線形トークン）不充足の拒絶診断は
> `E1101`（型がインタフェースを実装していない）族を再利用；③ フェーズ 1 結線後、`Struct == Struct`
> は `E6007` ランタイムエラーからコンパイル時判定へ移行し、`E6007`
> のトリガー面は縮小する。表内の登録文言は実装着地まで現状を維持。

#### E2xxx：意味解析

<!-- code-table:E2xxx start -->

| コード | 説明                         |
| ------ | ---------------------------- |
| E2001  | スコープエラー               |
| E2002  | 重複定義                     |
| E2003  | 所有権エラー                 |
| E2010  | 不変への代入                 |
| E2011  | 未初期化変数の使用           |
| E2012  | 可変性の競合                 |
| E2013  | 変数のシャドウィング         |
| E2014  | ムーブ済み値の使用           |
| E2016  | 不変への代入                 |
| E2018  | 可変/不変借用競合            |
| E2019  | 双重解放                     |
| E2020  | 解放後使用                   |
| E2027  | unsafe deref                 |
| E2029  | spawn 内 ref ループ          |
| E2030  | 精化型制約違反               |
| E2031  | 精化制約を証明できない       |
| E2090  | 無効な署名                   |
| E2091  | 署名に未知の型               |
| E2092  | 署名に矢印不足               |
| E2093  | 引数名の重複                 |
| E2094  | 泛型引数のシャドウィング     |
| E2095  | 引数名が泛型をシャドウィング |

<!-- code-table:E2xxx end -->

> 予約コード説明（2026-09-14 棚卸、#251 リリース口径）：E2019（双重解放）、E2020（解放後使用）、E2027（unsafe
> deref）、E2029（spawn 内 ref ループ）は登録済みでユニットテストのアンカーもあるが、到達可能な yx ソースの表層経路（明示的 drop 文、Ptr
> deref 文法、spawn
> ref ループ構築経路）がない——「意味的に完全に正しい」という主張はこれら 4 コードをカバーせず、実装補完までは「予約」として扱う。

#### E3xxx：コード生成

<!-- code-table:E3xxx start -->

| コード | 説明                                                 |
| ------ | ---------------------------------------------------- |
| E3004  | サポートされていないイテレータ                       |
| E3005  | IR 生成エラー                                        |
| E3006  | 未解決変数                                           |
| E3007  | トップレベルバインドの初期化は定数でなければならない |
| E3008  | サポートされていない match パターン                  |
| E3014  | レジスタオーバーフロー                               |
| E3017  | 無効な被演算子（コード生成）                         |
| E3018  | 単態化インスタンス化失敗                             |
| E3019  | トップレベルバインドの循環依存                       |
| E3020  | プログラム入口欠如                                   |
| E3021  | 入口が関数でない                                     |
| E3022  | 入口 main のシグネチャ不一致                         |
| E3023  | トップレベルでの実行文は許可されない                 |

<!-- code-table:E3xxx end -->

#### E4xxx：泛型と特質

<!-- code-table:E4xxx start -->

| コード | 説明                           |
| ------ | ------------------------------ |
| E4001  | 泛型制約違反                   |
| E4002  | 特質が見つからない             |
| E4003  | 特質実装欠落                   |
| E4004  | 特質実装競合                   |
| E4005  | 関連型が見つからない           |
| E4010  | 定数のゼロ除算                 |
| E4011  | 定数オーバーフロー             |
| E4012  | 定数递归過深                   |
| E4014  | 定数評価失敗                   |
| E4018  | 精化述語違反                   |
| E4019  | 型等式不成立                   |
| E4020  | 証明関数が必要                 |
| E4021  | ループ停止性を自動証明できない |

<!-- code-table:E4xxx end -->

> E4006/E8004 は現在発射点なし（予約コード）：Sized 制約と最適化エラーパスは実装待ち、実装時に実際のトリガー面に結線する。

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
| E6009  | Range ステップ値不正             |
| E6010  | 整数解析失敗                     |
| E6011  | 浮動小数点解析失敗               |

<!-- code-table:E6xxx end -->

> **コード表改訂（2026-08-09）**：コード表は元来 Rust 意味論ドラフト（Assertion failed / Arithmetic
> overflow / Heap allocation failed / Type cast
> failed）に基づき定義されており、実装の実際の要求と一致していなかった。YaoXiang には null ポインタ／ヒープ割り当て失敗／型キャストの概念がなく（値意味論 +
> Rust メモリ安全性）、ランタイムオーバーフローパスも検出未実装。校正後：
>
> - E6002 削除（元 Assertion failed は E6005 へ移動；元 null ポインタ意味論は言語概念なし）
> - E6003 を Arithmetic overflow から Runtime index out of bounds に変更（実際のトリガー面）
> - E6005 を Heap allocation failed から Assertion failed に変更（std.assert の実際のパス）
> - E6006 を Runtime index out of bounds から Function not found に変更（実装は既にそうなっていた）
> - E6007 を Type cast failed から汎用 Runtime
>   error に変更（ExecutorError のマッピングされていない変体の統一落点）

#### E7xxx：I/O とシステムエラー

<!-- code-table:E7xxx start -->

| コード | 説明                   |
| ------ | ---------------------- |
| E7001  | ファイルが見つからない |
| E7002  | 権限拒否               |
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

| コード | 説明                         |
| ------ | ---------------------------- |
| W1001  | 未使用のプライベート関数     |
| W1002  | 未使用のプライベート型       |
| W1003  | 未使用のインポート           |
| W1004  | 未使用のプライベート変数     |
| W1005  | 未使用のプライベートメソッド |
| W1063  | const 泛型制約を評価不能     |
| W1080  | コンパイル時証明の降格       |

<!-- code-table:W1xxx end -->

> W コード位置規則：E コードと同形で段階別にグループ化（W+段階千位）、W1xxx = 型チェック段階の警告。
>
> **デッドコードのコード意味論（#321 決定 B）**：`pub`
> 定義は対外インタフェースであり、永続的に報告しない——対外消費者が使用するかどうかは単一ファイル解析の範囲を超え、誤報しない方がよい。W1001/W1002/W1004/W1005 は
> **プライベート（非 pub）定義** のみ対象：`main` と `pub`
> 定義を起点とする参照到達性解析で一度も参照されない場合に報告。メソッド（W1005）は呼び出し点のショート名でマッチ。bin/lib
> target 意味論（bin 内で未使用の pub を警告）は将来拡張（#289 案 A）で、プロジェクトモデルのサポートが必要。
>
> **未使用インポート（W1003）**：typecheck の use
> elaboration が検出（pass2 でインポートのローカル名を登録、式解析と型注釈位置でヒットしたものは使用済みと見なす）、全体インポート（`use std.io`
> → モジュールエイリアス）と具名インポート（`use std.io.{print}`）の両方をカバー。
>
> **発射チャネル**：W コード診断は builder が W プレフィックスに基づきデフォルトで
> `Severity::Warning` を付与（明示的指定優先）、収集と呈示はエラーと同軌道（`warning[W####]`
> プレフィックスでレンダリング）、ただしコンパイルはブロックせず、成功終了コードにも影響しない。
> `yaoxiang check --deny-warnings`
> は警告を失敗に昇格させ（警告存在時に非ゼロコードで終了）、CI 厳格モード用。per-code 抑制（allow 属性など）は今後の拡張項目。

### メッセージ品質規範

> 本節はメッセージ単軌道と品質改訂（2026-09-03）により導入。`scripts/audit_diagnostics.py`
> により CI で強制執行される。

1. **メッセージ単軌道**：すべてのユーザー可視診断メッセージは権威登録表のショートカットメソッド +
   locales テンプレートレンダリングを経由しなければならず、コードは構造化引数のみを渡す。登録表を迂回して直接
   `Diagnostic::error(...)`
   などのネイティブ値を構築することは禁止——このパスはコード検証と i18n を迂回する。
2. **コード合法性**：未登録コードと疑似コード（例：`E_INTERNAL`）の使用は禁止；使用箇所のリテラルコードは登録表に定義済みでなければならない。内部エラーはすべて E8001（`internal_error`）にフォールバック。
3. **型表示**：型の Display はインスタンス化前後の形態を区別しなければならない（`Expected 'Container', found 'Container'`
   のようなベア名では区別不可）。
4. **ソルバー内部状態隔離**：ソルバー中間状態の TypeVar（Display 形態
   `t<N>`）はユーザー可視メッセージに入ってはならない。テストアンカー：`test_type_error_message_no_solver_typevar_leak`。
5. **E8xxx 境界**：E8xxx はコンパイラ内部一貫性問題（ICE）のみに使用する。ユーザーが修正可能なエラーに E8001 をフォールバックとして使用することは禁止；ICE メッセージには最小再現ガイダンスを添付しなければならない。

---

### ランタイムエラー値とコードの貫通

> 本節はランタイム Error 値とコードの連結改訂（2026-09-03）により導入。E6xxx/E7xxx 意味空間は 2 つのチャネルを同時に担い、コード空間は同一、呈示チャネルは異なる。

#### 2 つのチャネル

| チャネル                     | 载体                                                  | 呈示方式                                           |
| ---------------------------- | ----------------------------------------------------- | -------------------------------------------------- |
| コンパイラ/CLI 診断チャネル  | `ExecutorError` などのホスト層ハードエラー            | stderr `error[E####]:`（既結線 E6003/E6005/E6007） |
| プログラム内エラー値チャネル | std ライブラリ `Result(T, Error)` の Err 载体 `Error` | 言語値、プログラムの match/比較で消費              |

#### Error 構造（v0.8 より、破壊的変更）

```
Error { code: String, message: String }
```

- `code` は本仕様の E6xxx/E7xxx 番号を再利用、文字列形態（例：`"E6008"`）。
- **安定契約**：割り当て済みコードはバージョン間で意味不変；同一意味に削除済みコードを再利用しない（E6002 の前例あり）。
- **消費面**：プログラム内で `e.code == "E6xxx"`
  の比較が唯一のプログラマブル判定契約；`yaoxiang explain E6xxx`
  でドキュメント貫通；ツールチェーン（LSP / DAP、RFC-034 参照）はコードを exceptionId として扱う。
- **アクセサ**：`std.result.code(e)` / `std.result.message(e)`。
- **ユーザー定義エラー**：`Result(T, E)`
  の E は泛型引数、真剣にモデル化するならユーザー定義型で進める；std `Error`
  は単なる便利なフォールバック载体であり、そのコード体系はユーザー E 型を拘束しない。

#### コード割り当て規則

1. ランタイムエラー値コードとコンパイラ診断コードは E6xxx/E7xxx 空間を共有し、新コードは
   **実際のトリガー面** に基づいて割り当て、想象中的シナリオの予約は行わない。
2. 先登録後使用：新コードは権威登録表に登録し、三方一貫性検証（codes/*.rs ↔ locales
   ↔ 本ドキュメントコード表）を経た後に発射可能。ランタイムエラー値コードの登録ソースは
   `src/std/result.rs` の `RUNTIME_ERROR_CODES` 表（診断コードと同様に `build.rs` 構築時閾値 +
   `tools/code-tables` 検証を受ける）。
3. E7xxx は std.io / std.net エラー値用に予約されたセグメント（現在空き吊り、io/net
   Result 化時に有効化）。
4. 発射点：std 各モジュールは `error_new(code, message)` で Error 値を構築；消費側は
   `std.result.unwrap_err` で Err 载体を取り出し、`std.result.code/message` でフィールドを読み取る。

#### 進化経路（線 C、未実施）

パターンマッチ完備化（RFC-010b）着地後、`Error` は `{ kind: ErrorKind, message: String }`
にアップグレード可能、`code`
は kind から派生する属性に変換（変体定義箇所がそのままコード登録表）。進化期間中、本節の code 安定契約は不変を維持；このアップグレードは独立した決定であり、本節のコミットメントを構成しない。

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

/// i18n 表示テキスト登録表（コンパイル時に JSON から読み込み、ランタイムはテーブル参照ゼロ）
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
    /// 言語コードに基づいて登録表を取得
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

    /// テンプレートをレンダリング（コンパイル時に完了、ランタイムコストゼロ）
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

##### 事前定義プレースホルダ（一般的）

| プレースホルダ | 用途                             | 例                                  |
| -------------- | -------------------------------- | ----------------------------------- |
| `{name}`       | 変数名／型名／特質名などの識別子 | `Unknown variable: '{name}'`        |
| `{expected}`   | 期待型                           | `Expected type '{expected}'`        |
| `{found}`      | 実際／見つかった型               | `, found type '{found}'`            |
| `{method}`     | メソッド名                       | `Method {method} is not a function` |
| `{trait}`      | 特質名                           | `Cannot find trait: {trait}`        |
| `{path}`       | モジュールパス                   | `Invalid path: {path}`              |
| `{ty}`         | 型式                             | `Invalid type: {ty}`                |
| `{message}`    | 内部エラーメッセージ             | `Internal error: {message}`         |

##### 任意 key サポート

**params は任意の key をサポートし、事前定義に限定されない**。呼び出し側は任意の `key` を渡せる：

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

> **注意**：すべてのエラーコードがプレースホルダを使用するわけではない。一部のエラーコード（例：E0001）は静的メッセージで引数を必要としない。

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
# エラーメッセージ言語、選択可能：en, zh, ja, ...
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
1. プロジェクトレベル yaoxiang.toml の language.default を読み取る
2. 未設定の場合、ユーザーレベル ~/.yaoxiang/yaoxiang.toml を読み取る
3. どちらも未設定の場合、デフォルトで "en" を使用
4. コンパイラは選択された言語に基づいて I18nRegistry を作成（一度）
5. すべてのエラーは当該 I18nRegistry を使用してメッセージをレンダリング
```

#### ゼロテーブル参照コストの鍵

**レンダリングはユーザープロジェクトのコンパイル時に発生し、ランタイムではない。**

```
┌─────────────────────────────────────────────────────────────────────────┐
│  フェーズ 1: Rust が YaoXiang コンパイラをコンパイル                              │
│                                                                           │
│  JSON がコンパイラバイナリにパッケージ化される                                       │
│  目的：explain 命令が直接 i18n データを読み取れるように                          │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  フェーズ 2: YaoXiang がユーザープロジェクトをコンパイル（レンダリングはここで発生）        │
│                                                                           │
│  error! マクロ呼び出し時：                                                  │
│  1. yaoxiang.toml を読み取り言語設定を取得                                       │
│  2. コンパイラバイナリから対応言語の i18n JSON を読み込み                              │
│  3. テンプレート + 引数 → render() → "Unknown variable: 'x'"               │
│  4. Diagnostic.message = レンダリング済み文字列                                │
│                                                                           │
│  AOT バイナリは最終文字列を直接格納、テンプレートなし、テーブル参照なし                    │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  フェーズ 3: ユーザープログラムランタイム                                            │
│                                                                           │
│  println!("{}", diagnostic.message)                                      │
│  // 最終文字列を直接出力、テーブル参照一切なし                                        │
└─────────────────────────────────────────────────────────────────────────┘
```

| コンポーネント               | 責務                             | レンダリングタイミング           |
| ---------------------------- | -------------------------------- | -------------------------------- |
| `I18nRegistry`               | テンプレートと表示テキストを提供 | ユーザープロジェクトコンパイル時 |
| `DiagnosticBuilder.render()` | テンプレート + 引数 → 最終文字列 | ユーザープロジェクトコンパイル時 |
| `Diagnostic.message`         | レンダリング済み文字列           | 最終結果を格納                   |
| AOT バイナリ                 | 最終文字列を含む                 | ランタイムで直接使用             |

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

エラーの重大度は `DiagnosticLevel` enum によって管理され、エラーコード番号とは分離されている：

```rust
pub enum DiagnosticLevel {
    Error,    // コンパイル失敗を引き起こす
    Warning,  // コンパイルには影響しないが、修正を推奨
    Note,     // 補足情報
    Help,     // 修正提案
}
```

| レベル  | プレフィックス    | 説明                       |
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
3. 廃止タイムテーブルを提供

---

## 実施戦略

### フェーズ 1：エラーコード基礎構造

1. `src/diagnostics/` ディレクトリ構造を作成
2. `ErrorCode` enum を実装
3. `Diagnostic` と `DiagnosticLevel` を実装
4. リソースファイルディレクトリとサンプル JSON を作成

### フェーズ 2：explain コマンド

1. `yaoxiang explain` CLI コマンドを実装
2. `--lang` と `--json` オプションをサポート
3. リソースファイル読み込みを統合
4. 引数テンプレートレンダリングを実装

### フェーズ 3：コンパイル時統合

1. すべてのエラー報告ポイントを更新し新システムを使用
2. メッセージテンプレート引数注入を実装
3. 言語優先度ロジックを追加
4. ユニットテストカバレッジ

### フェーズ 4：IDE/LSP 統合

1. LSP サーバーが explain JSON 出力を統合
2. IDE 内にエラーコードリンクを表示
3. ホバーでエラー解説を表示
4. クイックフィックス提案

---

## 付録

### 完全エラーコード早見表

| 範囲  | カテゴリ               |
| ----- | ---------------------- |
| E0xxx | 字句と構文解析         |
| E1xxx | 型チェック             |
| E2xxx | 意味解析               |
| E3xxx | コード生成             |
| E4xxx | 泛型と特質             |
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

### エラーメッセージ例比較

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

- [Rust コンパイラエラー索引](https://doc.rust-lang.org/error_codes/error-index.html)
- [GCC エラーメッセージ形式](https://gcc.gnu.org/onlinedocs/gcc-13.1.0/gcc/Warning-Options.html)
- [Clang 診断形式](https://clang.llvm.org/diagnostics.html)
- [Language Server Protocol](https://microsoft.github.io/language-server-protocol/)

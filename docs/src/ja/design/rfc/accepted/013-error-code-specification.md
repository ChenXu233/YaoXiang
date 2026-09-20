---
title: 'RFC 013: エラーコード規約'
status: '採用'
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

# RFC 013: エラーコード規約

## 概要

本 RFC は YaoXiang コンパイラのエラーコード分類規約を提案する。Rust ライクな単層番号システムを採用しつつ、JSON リソースファイルによる多言語サポートを実現し、`yaoxiang explain`
コマンドでエラー解説機能を提供する。

## 動機

### なぜ標準化されたエラーコードが必要なのか？

1. **ユーザー体験**：ユーザーがエラーコードを見ることでエラーの種類と重大度を迅速に判断できる
2. **ドキュメント整備**：カテゴリ別に分類することでエラー参考ドキュメントの作成・保守が容易になる
3. **ツール連携**：IDE/LSP がエラーコードに基づいてクイックフィックス提案やドキュメントリンクを提供できる
4. **国際化対応**：エラーメッセージとコードを分離することで多言語翻訳が容易になる

### 設計目標

- **簡潔**：単層番号方式により、ユーザーが複雑な分類ルールを覚える必要がない
- **親しみやすい**：Rust ライクなエラーメッセージ形式、ヘルプ情報とサンプル付き
- **拡張性**：リソースファイル駆動で新しいエラーや新しい言語の追加が容易
- **ツールフレンドリー**：explain コマンド + JSON 出力、IDE/LSP 連携をサポート

---

## 提案

### 中核設計：単層番号システム

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
    Lexer,      // E0xxx: 字句・構文解析
    Parser,     // E0xxx: Parser errors
    TypeCheck,  // E1xxx: 型検査
    Semantic,   // E2xxx: 意味解析
    Generic,    // E4xxx: ジェネリクスとトレイト
    Module,     // E5xxx: モジュールとインポート
    Runtime,    // E6xxx: ランタイムエラー
    Io,         // E7xxx: I/O とシステムエラー
    Internal,   // E8xxx: 内部コンパイラエラー
}
```

### エラーコード定義と汎用 Builder

**中核原則**：エラーコード定義と表示テキストの分離

- `ErrorCodeDefinition`：エラーコードのメタデータ（code、category、template）、表示テキストを含まない
- `locales/*.json`：各言語の表示テキスト（title、message、help、エラーコードはネストされたオブジェクト）
- `DiagnosticBuilder`：汎用ビルダー、trait-per-error 設計の代替

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
        // テンプレート内のすべての {key} に対応するパラメータがあるかチェック
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

| 特性                   | 説明                                                         |
| ---------------------- | ------------------------------------------------------------ |
| **単一 Builder**       | 1 つの `DiagnosticBuilder` ですべてのエラーコードに共通対応  |
| **型安全**             | ショートカットメソッドによりパラメータの正しさが保証される   |
| **自己文書化**         | `E1001::unknown_variable(name)` を見れば意味が自明           |
| **テンプレート分離**   | メッセージテンプレートとコードが分離されており、i18n が容易  |
| **ランタイムコスト 0** | コンパイル時レンダリング、AOT バイナリにはテーブル参照が不要 |

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

/// 使用法：パラメータだけを渡せばよく、span と i18n は自動注入
return Err(error!(E1001, name = var_name));
return Err(error!(E1002, expected = "bool", found = cond_ty));
```

#### Builder の手動使用

```rust
// 手動で制御したい場合
E1001::unknown_variable(&var_name)
    .at(my_span)           // カスタム span
    .build(&custom_i18n)   // カスタム i18n
```

---

## 詳細設計

### エラーコード一覧

#### E0xxx：字句・構文解析

<!-- code-table:E0xxx start -->

| コード | 説明                       |
| ------ | -------------------------- |
| E0001  | 無効な文字                 |
| E0002  | 無効な数字リテラル         |
| E0003  | 終端されていない文字列     |
| E0004  | 無効な文字リテラル         |
| E0010  | 予期されたトークン         |
| E0011  | 予期しないトークン         |
| E0012  | 無効な構文                 |
| E0013  | 一致しない括弧             |
| E0014  | セミコロン不足             |
| E0016  | 式が期待される             |
| E0018  | キーワードが名前として使用 |

<!-- code-table:E0xxx end -->

#### E1xxx：型検査

<!-- code-table:E1xxx start -->

| コード | 説明                                             |
| ------ | ------------------------------------------------ |
| E1001  | 未知の変数                                       |
| E1002  | 型の不一致                                       |
| E1003  | 未知の型                                         |
| E1010  | 引数の数が一致しない                             |
| E1011  | 引数の型が一致しない                             |
| E1012  | 戻り値の型が一致しない                           |
| E1013  | 関数が見つからない                               |
| E1014  | 名前付き引数名が未知                             |
| E1015  | 引数の重複指定                                   |
| E1020  | 型を推論できない                                 |
| E1021  | 型推論の競合                                     |
| E1030  | パターンが不完全                                 |
| E1031  | 到達不能パターン                                 |
| E1040  | 操作がサポートされていない                       |
| E1041  | インデックス範囲外                               |
| E1042  | フィールドが見つからない                         |
| E1050  | ブールオペランドが必要                           |
| E1051  | 論理 NOT はブールオペランドが必要                |
| E1052  | 無効な逆参照                                     |
| E1053  | 非構造体フィールドアクセス                       |
| E1054  | 条件型が一致しない                               |
| E1055  | 非ジェネリックコンテキストでの制約               |
| E1060  | 型パラメータの数が一致しない                     |
| E1061  | ジェネリクスをインスタンス化できない             |
| E1062  | const ジェネリック制約の失敗                     |
| E1064  | バインディング位置のインデックスが無効           |
| E1065  | 非関数値の呼び出し                               |
| E1071  | 型定義はモジュールレベルでのみ可能               |
| E1081  | `?` は Result を返す関数内でのみ使用可能         |
| E1082  | `?` は Result 式にのみ使用可能                   |
| E1083  | `?` のエラー型が一致しない                       |
| E1090  | ✨ 不可言 ✨                                     |
| E1091  | 無効なジェネリックメタ型                         |
| E1092  | 精化型引数の形式が不正                           |
| E1093  | 精化引数の数が一致しない                         |
| E1094  | 未使用のコンパイル時値パラメータ                 |
| E1095  | 未知のインターフェース                           |
| E1096  | インターフェース引数の数が一致しない             |
| E1097  | インターフェースメンバー名の競合                 |
| E1098  | インターフェースメソッドが未実装                 |
| E1099  | インターフェースメソッドのシグネチャが一致しない |
| E1100  | インターフェースメソッドの重複実装               |
| E1101  | 型がインターフェースを実装していない             |
| E1102  | ループ制御文がループ外に出現                     |

<!-- code-table:E1xxx end -->

#### E2xxx：意味解析

<!-- code-table:E2xxx start -->

| コード | 説明                                   |
| ------ | -------------------------------------- |
| E2001  | スコープエラー                         |
| E2002  | 重複定義                               |
| E2003  | 所有権エラー                           |
| E2010  | 不変代入                               |
| E2011  | 未初期化変数の使用                     |
| E2012  | 可変性の競合                           |
| E2013  | 変数のシャドウィング                   |
| E2014  | 移動済み値の使用                       |
| E2016  | 不変代入                               |
| E2018  | 可変/不変借用競合                      |
| E2019  | 二重解放                               |
| E2020  | 解放後使用                             |
| E2027  | unsafe 逆参照                          |
| E2029  | spawn 内参照ループ                     |
| E2030  | 精化型制約違反                         |
| E2090  | 無効なシグネチャ                       |
| E2091  | シグネチャに未知の型                   |
| E2092  | シグネチャに矢印がない                 |
| E2093  | 重複引数名                             |
| E2094  | ジェネリックパラメータのシャドウィング |
| E2095  | 引数名がジェネリックを覆い隠す         |

<!-- code-table:E2xxx end -->

> 予約コード説明（2026-09-14 棚卸、#251 リリース基準）：E2019（二重解放）、E2020（解放後使用）、E2027（unsafe 逆参照）、E2029（spawn 内 ref ループ）は登録が完了しユニットテストが固定されているが、到達可能な yx ソースコード表層がまだない（明示的 drop 文、Ptr 逆参照文法、spawn
> ref ループ構築経路）— 意味が完全に正しいという主張はこれら 4 コードをカバーしない、実装補完までは「予約」扱い。

#### E3xxx：コード生成

<!-- code-table:E3xxx start -->

| コード | 説明                                                       |
| ------ | ---------------------------------------------------------- |
| E3004  | サポートされていないイテレータ                             |
| E3005  | IR 生成エラー                                              |
| E3006  | 未解決変数                                                 |
| E3007  | トップレベルバインディングの初期化は定数でなければならない |
| E3008  | サポートされていない match パターン                        |
| E3014  | レジスタオーバーフロー                                     |
| E3017  | 無効なオペランド（コード生成）                             |
| E3018  | 単相化インスタンス化の失敗                                 |
| E3019  | トップレベルバインディングの循環依存                       |
| E3020  | プログラムエントリがない                                   |
| E3021  | エントリが関数ではない                                     |
| E3022  | エントリ main のシグネチャが一致しない                     |

<!-- code-table:E3xxx end -->

#### E4xxx：ジェネリクスとトレイト

<!-- code-table:E4xxx start -->

| コード | 説明                   |
| ------ | ---------------------- |
| E4001  | ジェネリック制約違反   |
| E4002  | トレイトが見つからない |
| E4003  | トレイト実装の欠如     |
| E4004  | トレイト実装の競合     |
| E4005  | 関連型が見つからない   |
| E4010  | 定数のゼロ除算         |
| E4011  | 定数オーバーフロー     |
| E4012  | 定数の再帰が深すぎる   |
| E4014  | 定数評価の失敗         |
| E4018  | 精化述語違反           |
| E4019  | 型等式が成立しない     |
| E4020  | 証明関数が必要         |

<!-- code-table:E4xxx end -->

> E4006/E8004 は現在発射ポイントなし（予約コード）：Sized 制約と最適化エラーパスは実装待ち、実装時は実際のトリガー面に従って配線。

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
| E6009  | Range ステップが無効             |
| E6010  | 整数解析失敗                     |
| E6011  | 浮動小数点解析失敗               |

<!-- code-table:E6xxx end -->

> **コード表改訂（2026-08-09）**：コード表は元々 Rust セマンティクス草案（Assertion
> failed/Arithmetic overflow/Heap allocation failed/Type cast
> failed）に従って定義されていたが、実装の実態のニーズと一致しなかった。YaoXiang にはヌルポインタ/ヒープ割り当て失敗/型変換の概念がなく（値セマンティクス +
> Rust メモリ安全性）、ランタイムオーバーフロー経路は検出が未実装。キャリブレーション後：
>
> - E6002 削除（元 Assertion
>   failed は E6005 に移動；元ヌルポインタセマンティクスは言語の概念にない）
> - E6003 を Arithmetic overflow から Runtime index out of bounds に変更（実際のトリガー面）
> - E6005 を Heap allocation failed から Assertion failed に変更（std.assert の実際のパス）
> - E6006 を Runtime index out of bounds から Function not found に変更（実装は既にそうなっていた）
> - E6007 を Type cast failed から汎用 Runtime
>   error に変更（ExecutorError 未マッピングバリアントの統一フォールバック）

#### E7xxx：I/O とシステムエラー

<!-- code-table:E7xxx start -->

| コード | 説明                   |
| ------ | ---------------------- |
| E7001  | ファイルが見つからない |
| E7002  | 権限が拒否されました   |
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

| コード | 説明                             |
| ------ | -------------------------------- |
| W1001  | 未使用のプライベート関数         |
| W1002  | 未使用のプライベート型           |
| W1003  | 未使用のインポート               |
| W1004  | 未使用のプライベート変数         |
| W1005  | 未使用のプライベートメソッド     |
| W1063  | const ジェネリック制約が評価不能 |
| W1080  | コンパイル時証明のダウングレード |

<!-- code-table:W1xxx end -->

> W コード位ルール：E コードと同じ構造で段階別にグループ化（W+段階千位セグメント）、W1xxx
> = 型検査段階の警告。
>
> **デッドコード規約（#321 プラン B 確定）**：`pub`
> 定義は外部向けインターフェースであり、決して報告しない—外部消費者が使用するかどうかは単一ファイル分析の範囲を超えるため、誤報を避けるために沈黙する方が良い。W1001/W1002/W1004/W1005 はプライベート（`pub`
> でない）定義のみを対象とする：`main` と `pub`
> 定義から始まる参照到達性分析で一度も参照されない場合に報告。メソッド（W1005）は呼び出し点の短い名前で照合する。bin/lib
> target セマンティクス（bin 内で未使用の pub の警告）は将来の拡張（#289 プラン A）で、プロジェクトモデルのサポートが必要。
>
> **未使用インポート（W1003）**：typecheck の use 展開（elaboration）によって検出される（pass2 でインポートローカル名を登録し、式解析と型注釈位置でヒットすれば使用済みと見なす）、全体インポート（`use std.io`
> → モジュールエイリアス）と名前付きインポート（`use std.io.{print}`）の両方をカバーする。
>
> **発射チャネル**：W コード診断は builder により W プレフィックスでデフォルト `Severity::Warning`
> とマークされ（明示的な指定が優先）、収集と表示はエラーと同じ経路（`warning[W####]`
> プレフィックスでレンダリング）、ただしコンパイルをブロックせず、成功終了コードにも影響しない。`yaoxiang check --deny-warnings`
> は警告を失敗に昇格させ（警告が存在する場合は非ゼロコードで終了）、CI 厳格モードに使用される。per-code 抑制（allow 属性など）は将来の拡張項目。

### メッセージ品質規約

> 本節はメッセージ単一経路と品質改訂（2026-09-03）によって導入された。`scripts/audit_diagnostics.py`
> により CI で強制実行される。

1. **メッセージ単一経路**：すべてのユーザー可視診断メッセージは権威登録テーブルのショートカットメソッド +
   locales テンプレートレンダリングを経由しなければならず、コードは構造化パラメータのみを渡す。登録テーブルをバイパスして
   `Diagnostic::error(...)`
   などのネイティブ値を直接構築することは禁止—この経路はコード検証と i18n をバイパスする。
2. **コード合法性**：未登録コードと擬似コード（例：`E_INTERNAL`）の使用を禁止；使用地点のコードリテラルは登録テーブルに既に定義されている必要がある。内部エラーは一律 E8001（`internal_error`）にフォールバック。
3. **型表示**：型 Display はインスタンス化前後の形式を区別しなければならない（`Expected 'Container', found 'Container'`
   裸の名前では区別不可）。
4. **ソルバー内部状態の隔離**：ソルバー中間状態の TypeVar（Display 形式
   `t<N>`）はユーザー可視メッセージに入れてはならない。テスト固定：`test_type_error_message_no_solver_typevar_leak`。
5. **E8xxx 境界**：E8xxx はコンパイラ内部の整合性の問題（ICE）にのみ使用する。ユーザーが修正可能なエラーは E8001 フォールバックの使用を禁止；ICE メッセージには最小限の再現手順を添付しなければならない。

---

### ランタイムエラー値とコードの連結

> 本節はランタイム Error 値にコードを付与する改訂（2026-09-03）によって導入された。E6xxx/E7xxx セマンティクス空間は 2 つのチャネルを同時に担い、コード空間は同一、表示チャネルが異なる。

#### 2 つのチャネル

| チャネル                     | 载体                                                  | 表示方式                                             |
| ---------------------------- | ----------------------------------------------------- | ---------------------------------------------------- |
| コンパイラ/CLI 診断チャネル  | `ExecutorError` などのホスト層のハードエラー          | stderr `error[E####]:`（E6003/E6005/E6007 配線済み） |
| プログラム内エラー値チャネル | std ライブラリ `Result(T, Error)` の Err 载体 `Error` | 言語値、プログラムの match/比較で消費                |

#### Error 構造（v0.8 以降、破壊的変更）

```
Error { code: String, message: String }
```

- `code` は本規約の E6xxx/E7xxx 番号を再利用し、文字列形式（例：`"E6008"`）。
- **安定契約**：割り当てられたコードはバージョンを超えてセマンティクスが変わらない；同じセマンティクスで削除済みコードは再利用しない（E6002 の先例）。
- **消費面**：プログラム内の `e.code == "E6xxx"`
  比較が唯一のプログラマブル判定契約；`yaoxiang explain E6xxx`
  ドキュメントで貫通；ツールチェーン（LSP / DAP、RFC-034 参照）はコードを exceptionId とする。
- **アクセサ**：`std.result.code(e)` / `std.result.message(e)`。
- **ユーザー定義エラー**：`Result(T, E)`
  の E はジェネリックパラメータであり、真面目にモデリングする場合はユーザー定義型を使用する；std
  `Error` は単なる便利なフォールバックキャリアであり、そのコード体系はユーザーの E 型を制約しない。

#### コード割り当てルール

1. ランタイムエラー値コードとコンパイラ診断コードは E6xxx/E7xxx スペースを共有し、新コードは実際のトリガー面に従って割り当て、想像上のシナリオのために予約しない。
2. 先に登録してから使用：新コードは権威登録テーブルに登録し、三者間一貫性検証（codes/*.rs ↔ locales
   ↔ 本ドキュメントのコード表）を経た後にのみ発射可能。ランタイムエラー値コードの登録ソースは
   `src/std/result.rs` の `RUNTIME_ERROR_CODES` テーブル（診断コードと同じく
   `build.rs コンパイル時閾値 + `tools/code-tables`` 検証を受ける）。
3. E7xxx は std.io / std.net エラー値用に予約されたセグメント（現在空、io/net
   Result 化時に有効化）。
4. 発射ポイント：std の各モジュールは `error_new(code, message)` 経由で Error 値を構築；消費側は
   `std.result.unwrap_err` で Err キャリアを取り出し、`std.result.code/message`
   でフィールドを読み取る。

#### 進化パス（ライン C、未実施）

パターンマッチの完備化（RFC-039）が実装された後、`Error` は `{ kind: ErrorKind, message: String }`
にアップグレード可能となり、`code`
は kind から派生する属性に変換される（バリアント定義箇所がコード登録テーブル）。進化期間中、本節のコード安定契約は変わらない；このアップグレードは独立した決定であり、本節の約束を構成しない。

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

/// i18n 表示テキスト登録テーブル（コンパイル時に JSON から読み込み、ランタイムはテーブル参照なし）
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
    /// 言語コードから登録テーブルを取得
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

    /// テンプレートをレンダリング（コンパイル時に完了、ランタイムコスト 0）
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

##### 定義済みプレースホルダ（よく使うもの）

| プレースホルダ | 用途                               | 例示                                |
| -------------- | ---------------------------------- | ----------------------------------- |
| `{name}`       | 変数名/型名/トレイト名などの識別子 | `Unknown variable: '{name}'`        |
| `{expected}`   | 期待される型                       | `Expected type '{expected}'`        |
| `{found}`      | 実際/見つかった型                  | `, found type '{found}'`            |
| `{method}`     | メソッド名                         | `Method {method} is not a function` |
| `{trait}`      | トレイト名                         | `Cannot find trait: {trait}`        |
| `{path}`       | モジュールパス                     | `Invalid path: {path}`              |
| `{ty}`         | 型式                               | `Invalid type: {ty}`                |
| `{message}`    | 内部エラーメッセージ               | `Internal error: {message}`         |

##### 任意の key サポート

**params は任意の key をサポートし、定義済みプレースホルダに限定されない**。呼び出し側は任意の `key`
を渡せる：

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

> **注意**：すべてのエラーコードがプレースホルダを使用するわけではない。E0001 のような一部のエラーコードは静的メッセージであり、パラメータは不要。

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
1. プロジェクトレベル yaoxiang.toml の language.default を読み込む
2. 未設定の場合、ユーザーレベル ~/.yaoxiang/yaoxiang.toml を読み込む
3. どちらも未設定の場合、デフォルトで "en" を使用
4. コンパイラは選択された言語に応じて I18nRegistry を作成（一度だけ）
5. すべてのエラーはその I18nRegistry を使ってメッセージをレンダリング
```

#### テーブル参照コスト 0 の鍵

**レンダリングはユーザープロジェクトをコンパイルする時に発生し、ランタイムではない。**

```
┌─────────────────────────────────────────────────────────────────────────┐
│  フェーズ 1: Rust が YaoXiang コンパイラをコンパイル                          │
│                                                                           │
│  JSON はコンパイラバイナリに埋め込まれる                                       │
│  目的：explain コマンドが i18n データを直接読み取れるようにする                      │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  フェーズ 2: YaoXiang がユーザープロジェクトをコンパイル（レンダリングはここで発生）   │
│                                                                           │
│  error! マクロ呼び出し時：                                                     │
│  1. yaoxiang.toml から言語設定を取得                                        │
│  2. コンパイラバイナリから対応する言語の i18n JSON を読み込み                       │
│  3. テンプレート + パラメータ → render() → "Unknown variable: 'x'"           │
│  4. Diagnostic.message = レンダリング済み文字列                                 │
│                                                                           │
│  AOT バイナリは最終的な文字列を直接保持、テンプレートもテーブル参照も不要               │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  フェーズ 3: ユーザープログラムのランタイム                                          │
│                                                                           │
│  println!("{}", diagnostic.message)                                      │
│  // 最終的な文字列を直接出力、テーブル参照は一切なし                                  │
└─────────────────────────────────────────────────────────────────────────┘
```

| コンポーネント               | 役割                                   | レンダリングタイミング           |
| ---------------------------- | -------------------------------------- | -------------------------------- |
| `I18nRegistry`               | テンプレートと表示テキストを提供       | ユーザープロジェクトコンパイル時 |
| `DiagnosticBuilder.render()` | テンプレート + パラメータ → 最終文字列 | ユーザープロジェクトコンパイル時 |
| `Diagnostic.message`         | レンダリング済み文字列                 | 最終結果を保持                   |
| AOT バイナリ                 | 最終文字列を埋め込み                   | ランタイムに直接使用             |

---

### エラーメッセージ形式

エラーメッセージは以下の形式を使用する：

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

エラーの重大度は `DiagnosticLevel` 列挙型で管理し、エラーコード番号とは疎結合とする：

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

### `yaoxiang explain` コマンド

#### コマンド構文

```bash
yaoxiang explain <ERROR_CODE> [OPTIONS]
```

#### オプション

| オプション      | 説明                                        |
| --------------- | ------------------------------------------- |
| `--lang <code>` | 言語を指定 (en-US, zh-CN、デフォルト en-US) |
| `--json`        | JSON 形式で出力（IDE/LSP で使用）           |
| `--json-pretty` | 整形された JSON 出力                        |
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

# JSON 出力（LSP 連携）
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

1. 旧エラーコードから新エラーコードへのマッピングを維持する
2. 移行期間中は新旧コードを同時に表示する
3. 廃止スケジュールを提供する

---

## 実装戦略

### フェーズ 1：エラーコードインフラ

1. `src/diagnostics/` ディレクトリ構造を作成する
2. `ErrorCode` 列挙型を実装する
3. `Diagnostic` と `DiagnosticLevel` を実装する
4. リソースファイルディレクトリとサンプル JSON を作成する

### フェーズ 2：explain コマンド

1. `yaoxiang explain` CLI コマンドを実装する
2. `--lang` と `--json` オプションをサポートする
3. リソースファイルの読み込みを統合する
4. パラメータテンプレートレンダリングを実装する

### フェーズ 3：コンパイル時統合

1. すべてのエラーレポート地点を新システムを使用するように更新する
2. メッセージテンプレートパラメータ注入を実装する
3. 言語優先順位ロジックを追加する
4. ユニットテストカバレッジを確保する

### フェーズ 4：IDE/LSP 統合

1. LSP サーバーが explain の JSON 出力を統合する
2. IDE にエラーコードリンクを表示する
3. ホバーでエラー説明を表示する
4. クイック修正提案を提供する

---

## 付録

### 完全エラーコード早見表

| 範囲  | カテゴリ               |
| ----- | ---------------------- |
| E0xxx | 字句・構文解析         |
| E1xxx | 型検査                 |
| E2xxx | 意味解析               |
| E3xxx | コード生成             |
| E4xxx | ジェネリクスとトレイト |
| E5xxx | モジュールとインポート |
| E6xxx | ランタイムエラー       |
| E7xxx | I/O とシステムエラー   |
| E8xxx | 内部コンパイラエラー   |
| E9xxx | 予約                   |

### サポートされる言語

| コード | 言語         | ステータス |
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

- [Rust コンパイラエラーインデックス](https://doc.rust-lang.org/error_codes/error-index.html)
- [GCC エラーメッセージ形式](https://gcc.gnu.org/onlinedocs/gcc-13.1.0/gcc/Warning-Options.html)
- [Clang 診断形式](https://clang.llvm.org/diagnostics.html)
- [Language Server Protocol](https://microsoft.github.io/language-server-protocol/)

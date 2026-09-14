---
title: 'RFC 013: エラーコード規約'
status: '受入済'
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

本 RFC は YaoXiang コンパイラのエラーコード分類規約を提案する。Rust に類似した単層番号システムを採用し、JSON リソースファイルで多言語対応を実現し、`yaoxiang explain`
コマンドによりエラー解説機能を提供する。

## 動機

### なぜ標準化されたエラーコードが必要なのか？

1. **ユーザ体験**：エラーコードを見ることで、ユーザはエラーの種類と深刻度を迅速に判断できる
2. **ドキュメント構成**：カテゴリごとに分類することで、エラー参考ドキュメントの記述と保守が容易になる
3. **ツール統合**：IDE/LSP がエラーコードに基づいてクイック修正提案とドキュメントリンクを提供できる
4. **国際化対応**：エラーメッセージとコードを分離することで、多言語翻訳が容易になる

### 設計目標

- **簡潔**：単層番号方式により、ユーザは複雑な分類ルールを記憶する必要がない
- **親しみやすい**：Rust 風のエラーメッセージ形式、ヘルプ情報とサンプル付き
- **拡張可能**：リソースファイル駆動により、新しいエラーと言語の追加が容易
- **ツール親和性**：explain コマンド + JSON 出力により、IDE/LSP 統合をサポート

---

## 提案

### 中核設計：単層番号システム

四位数字番号を採用し、コンパイル段階でグループ化する：

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
| **1** | E1xxx | 型チェック             |
| **2** | E2xxx | 意味解析               |
| **3** | E3xxx | コード生成             |
| **4** | E4xxx | ジェネリクスとtrait    |
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
    TypeCheck,  // E1xxx: 型チェック
    Semantic,   // E2xxx: 意味解析
    Generic,    // E4xxx: ジェネリクスとtrait
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
- `DiagnosticBuilder`：汎用ビルダー、trait-per-error 設計を置き換え

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
    pub message_template: &'static str,  // メッセージテンプレート、{param} プレースホルダ対応
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

    /// Diagnostic を構築（テンプレートのレンダリングはコンパイル期に完了）
    pub fn build(&self, i18n: &I18nRegistry) -> Diagnostic {
        // テンプレート内のすべての {key} に対応する引数があることを検証
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

| 特性                             | 説明                                                           |
| -------------------------------- | -------------------------------------------------------------- |
| **単一 Builder**                 | 一つの `DiagnosticBuilder` ですべてのエラーコードに対応        |
| **型安全**                       | ショートカットメソッドが引数の正確性を保証                     |
| **自己文書化**                   | `E1001::unknown_variable(name)` で一目瞭然                     |
| **テンプレートの分離**           | メッセージテンプレートとコードが分離され、i18n が容易          |
| **ランタイムオーバーヘッドゼロ** | コンパイル期レンダリングにより、AOT バイナリにテーブル参照なし |

---

### エラーマクロの簡略化

#### error! マクロ（コンテキスト自動注入）

```rust
/// コンパイル期に span と i18n 設定を自動取得するマクロ
macro_rules! error {
    ($code:ident, $($key:ident = $value:expr),* $(,)?) => {
        $code()
            $(.$key($value))*
            .at(crate::util::span::Span::current())
            .build(crate::util::diagnostic::I18nRegistry::current())
    };
}

/// 使用法：引数だけを渡せば、span と i18n は自動注入
return Err(error!(E1001, name = var_name));
return Err(error!(E1002, expected = "bool", found = cond_ty));
```

#### 手動での Builder 使用

```rust
// 手動制御が必要な場合
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
| E0002  | 無効な数値リテラル         |
| E0003  | 未終端の文字列             |
| E0004  | 無効な文字リテラル         |
| E0010  | 期待されたトークン         |
| E0011  | 予期しないトークン         |
| E0012  | 無効な構文                 |
| E0013  | 一致しない括弧             |
| E0014  | セミコロン不足             |
| E0016  | 式が期待される             |
| E0018  | キーワードを名前として使用 |

<!-- code-table:E0xxx end -->

#### E1xxx：型チェック

<!-- code-table:E1xxx start -->

| コード | 説明                                     |
| ------ | ---------------------------------------- |
| E1001  | 未知の変数                               |
| E1002  | 型の不一致                               |
| E1003  | 未知の型                                 |
| E1010  | 引数の数が一致しない                     |
| E1011  | 引数の型が一致しない                     |
| E1012  | 戻り値の型が一致しない                   |
| E1013  | 関数が見つからない                       |
| E1020  | 型を推論できない                         |
| E1021  | 型推論の衝突                             |
| E1030  | パターンが不完全                         |
| E1031  | 到達不能パターン                         |
| E1040  | 未サポートの操作                         |
| E1041  | インデックス範囲外                       |
| E1042  | フィールドが見つからない                 |
| E1050  | ブールオペランドが必要                   |
| E1051  | 論理 NOT にブールオペランドが必要        |
| E1052  | 無効な deref                             |
| E1053  | 非構造体フィールドアクセス               |
| E1054  | 条件型が一致しない                       |
| E1055  | 非ジェネリクス文脈での型制約             |
| E1060  | 型引数の数が一致しない                   |
| E1061  | ジェネリクスをインスタンス化できない     |
| E1062  | const ジェネリクス制約違反               |
| E1064  | バインディング位置インデックスが無効     |
| E1071  | 型定義はモジュールレベルでのみ可能       |
| E1081  | `?` は Result を返す関数内でのみ使用可能 |
| E1082  | `?` は Result 式にのみ使用可能           |
| E1083  | `?` のエラー型が一致しない               |
| E1090  | ✨ 言語化不能 ✨                         |
| E1091  | 無効なジェネリクスのメタ型               |
| E1092  | 精化型引数の形式が不正                   |
| E1093  | 精化引数の数が一致しない                 |
| E1094  | 未使用のコンパイル期値引数               |
| E1095  | 未知の interface                         |
| E1096  | interface の引数の数が一致しない         |
| E1097  | interface メンバー名の衝突               |
| E1098  | interface メソッド未実装                 |
| E1099  | interface メソッドシグネチャが一致しない |
| E1100  | interface メソッド重複実装               |
| E1101  | 型が interface を実装していない          |
| E1102  | ループ外でのループ制御文                 |

<!-- code-table:E1xxx end -->

#### E2xxx：意味解析

<!-- code-table:E2xxx start -->

| コード | 説明                             |
| ------ | -------------------------------- |
| E2001  | スコープエラー                   |
| E2002  | 重複定義                         |
| E2003  | 所有権エラー                     |
| E2010  | 不可変変数への代入               |
| E2011  | 未初期化変数の使用               |
| E2012  | 可変性の衝突                     |
| E2013  | 変数のシャドウィング             |
| E2014  | 移動済み値の使用                 |
| E2016  | 不可変変数への代入               |
| E2018  | 可変/不可変借用の衝突            |
| E2019  | ダブルフリー                     |
| E2020  | 解放後使用                       |
| E2027  | unsafe deref                     |
| E2029  | spawn 内 ref 循環                |
| E2030  | 精化型制約違反                   |
| E2090  | 無効なシグネチャ                 |
| E2091  | シグネチャの未知の型             |
| E2092  | シグネチャに矢印がない           |
| E2093  | 引数名の重複                     |
| E2094  | ジェネリクス引数のシャドウィング |
| E2095  | ジェネリクスを覆う引数名         |

<!-- code-table:E2xxx end -->

> 予約コードの説明（2026-09-14 棚卸し、#251 リリース基準）：E2019（ダブルフリー）、E2020（解放後使用）、E2027（unsafe
> deref）、E2029（spawn 内 ref 循環）は登録済みでユニットテストにより固定されているが、現時点で到達可能な yx ソースの表層（明示的な drop 文、Ptr
> deref 文法、spawn
> ref 環構築経路）がない——意味的に完全に正しいという主張はこれら 4 コードには及ばず、実装補完までは「予約」として扱う。

#### E3xxx：コード生成

<!-- code-table:E3xxx start -->

| コード | 説明                                                       |
| ------ | ---------------------------------------------------------- |
| E3004  | 未サポートのイテレータ                                     |
| E3005  | IR 生成エラー                                              |
| E3006  | 未解決の変数                                               |
| E3007  | トップレベルバインディングの初期化は定数でなければならない |
| E3008  | 未サポートの match パターン                                |
| E3014  | レジスタオーバーフロー                                     |
| E3017  | 無効なオペランド（コード生成）                             |
| E3018  | モノモーフィゼーション失敗                                 |

<!-- code-table:E3xxx end -->

#### E4xxx：ジェネリクスとtrait

<!-- code-table:E4xxx start -->

| コード | 説明                 |
| ------ | -------------------- |
| E4001  | ジェネリクス制約違反 |
| E4002  | trait が見つからない |
| E4003  | trait 実装欠如       |
| E4004  | trait 実装衝突       |
| E4005  | 関連型が見つからない |
| E4010  | 定数のゼロ除算       |
| E4011  | 定数オーバーフロー   |
| E4012  | 定数再帰が深すぎる   |
| E4014  | 定数評価失敗         |
| E4018  | 精化述語違反         |
| E4019  | 型等式が成立しない   |
| E4020  | 証明関数が必要       |

<!-- code-table:E4xxx end -->

> E4006/E8004 は現在射出点なし（予約コード）：Sized 制約と最適化エラーパスは未実装、実装時に実際のトリガー面で配線する。

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
| E6009  | Range のステップ値が無効         |
| E6010  | 整数パース失敗                   |
| E6011  | 浮動小数点パース失敗             |

<!-- code-table:E6xxx end -->

> **コード表改訂（2026-08-09）**：コード表は元々 Rust 意味論の草案（Assertion failed/Arithmetic
> overflow/Heap allocation failed/Type cast
> failed）に基づいて定義されており、実装の実際の要件と一致していなかった。YaoXiang には null ポインタ/ヒープ割り当て失敗/型変換の概念がなく（値意味論 +
> Rust メモリ安全性）、ランタイムオーバーフロー経路は未実装である。校正後：
>
> - E6002 削除（旧 Assertion failed は E6005 へ移動；旧 null ポインタ意味論は言語に概念なし）
> - E6003 は Arithmetic overflow から Runtime index out of bounds（実際のトリガー面）へ変更
> - E6005 は Heap allocation failed から Assertion failed（std.assert 実際経路）へ変更
> - E6006 は Runtime index out of bounds から Function not found（実装はすでにこう）へ変更
> - E6007 は Type cast failed から汎用 Runtime error（ExecutorError の未マップ変体の統一落点）へ変更

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

| コード | 説明                             |
| ------ | -------------------------------- |
| W1001  | 未使用のプライベート関数         |
| W1002  | 未使用のプライベート型           |
| W1003  | 未使用のインポート               |
| W1004  | 未使用のプライベート変数         |
| W1005  | 未使用のプライベートメソッド     |
| W1063  | const ジェネリクス制約が評価不能 |
| W1080  | コンパイル期証明の降格           |

<!-- code-table:W1xxx end -->

> W コード位置ルール：E コードと同型で段階別グループ化（W+段階千位セグメント）、W1xxx
> = 型チェック段階の警告。
>
> **デッドコードコード意味（#321 決定 B）**：`pub`
> 定義は外部インターフェースであり、永久に報告しない——外部コンシューマーが使用するかどうかは単一ファイル解析の境界を超えるため、誤報よりも沈黙を優先する。W1001/W1002/W1004/W1005 は
> **プライベート（非 pub）定義**のみ対象：`main` と `pub`
> 定義から開始する参照到達性解析で一度も参照されない場合に報告。メソッド（W1005）は呼び出し点の短い名前で照合。bin/lib ターゲット意味論（bin 内未使用 pub 警告）は将来の拡張（#289 案 A）、プロジェクトモデルが必要。
>
> **未使用インポート（W1003）**：typecheck の use
> elaboration で検出（pass2 でインポートローカル名を登録、式解析と型注釈位置でヒットすれば使用済みとみなす）、全体インポート（`use std.io`
> → モジュールエイリアス）と名前付きインポート（`use std.io.{print}`）の両方をカバー。
>
> **射出チャネル**：W コード診断は builder により W 接頭辞でデフォルト `Severity::Warning`
> を付与（明示指定優先）、収集と表示はエラーと同じ経路（`warning[W####]`
> 接頭辞レンダリング）だが、コンパイルを中断せず、成功終了コードにも影響しない。
> `yaoxiang check --deny-warnings`
> は警告を失敗に昇格させる（警告存在時に非ゼロコードで終了）、CI 厳格モード用。コード毎の抑制（allow 属性など）は今後の拡張項目。

### メッセージ品質規約

> 本節はメッセージ単一経路と品質改訂（2026-09-03）により導入。`scripts/audit_diagnostics.py`
> により CI で強制実行される。

1. **メッセージ単一経路**：すべてのユーザ可視診断メッセージは権威あるレジストリのショートカットメソッド +
   locales テンプレートのレンダリングを経由しなければならず、コードは構造化された引数のみを渡す。レジストリを迂回して
   `Diagnostic::error(...)`
   などのネイティブ値を直接構築することは禁止——この経路はコード検証と i18n を迂回する。
2. **コード合法性**：未登録コードと仮コード（例：`E_INTERNAL`）の使用は禁止；使用箇所のコードリテラルはレジストリに定義済みでなければならない。内部エラーはすべて E8001（`internal_error`）にフォールバック。
3. **型表示**：型 Display はインスタンス化前後の形態を区別しなければならない（`Expected 'Container', found 'Container'`
   のようなベア名は区別不能）。
4. **ソルバー内部状態の隔離**：ソルバー中間状態 TypeVar（Display 形態
   `t<N>`）はユーザ可視メッセージに入ってはならない。テストアンカー：`test_type_error_message_no_solver_typevar_leak`。
5. **E8xxx 境界**：E8xxx はコンパイラの内部一貫性问题（ICE）のみ。ユーザが修正可能なエラーに E8001 フォールバックを使用することは禁止；ICE メッセージには最小限の再現ガイダンスを添付しなければならない。

---

### ランタイムエラー値とコードの貫通

> 本節はランタイム Error 値にコードを付与する改訂（2026-09-03）により導入。E6xxx/E7xxx 意味空間は二つのチャネルを担い、コード空間は共通、提示チャネルは異なる。

#### 二つのチャネル

| チャネル                     | 媒体                                                  | 提示方式                                                   |
| ---------------------------- | ----------------------------------------------------- | ---------------------------------------------------------- |
| コンパイラ/CLI 診断チャネル  | `ExecutorError` などのホスト層ハードエラー            | stderr `error[E####]:`（すでに配線済み E6003/E6005/E6007） |
| プログラム内エラー値チャネル | std ライブラリ `Result(T, Error)` の Err 媒体 `Error` | 言語値、プログラムが match/比較で消費                      |

#### Error 構造（v0.8 以降、破壊的変更）

```
Error { code: String, message: String }
```

- `code` は本規約の E6xxx/E7xxx 番号を再利用、文字列形態（例：`"E6008"`）。
- **安定契約**：割り当て済みコードはバージョン間で意味不変；同じ意味に削除済みコードを再利用しない（E6002 が先例）。
- **消費面**：プログラム内 `e.code == "E6xxx"`
  の比較が唯一のプログラム可能な判定契約；`yaoxiang explain E6xxx`
  でドキュメント貫通；ツールチェーン（LSP / DAP、RFC-034 参照）はコードを exceptionId とする。
- **アクセサ**：`std.result.code(e)` / `std.result.message(e)`。
- **ユーザカスタムエラー**：`Result(T, E)`
  の E はジェネリクス引数、真剣にモデル化するならユーザカスタム型で進める；std `Error`
  は単なる便利なフォールバック媒体であり、そのコード体系はユーザ E 型を制約しない。

#### コード割り当てルール

1. ランタイムエラー値コードとコンパイラ診断コードは E6xxx/E7xxx 空間を共有し、新コードは**実際のトリガー面**で割り当て、想像上のシナリオに対しては予約しない。
2. 登録してから使用：新コードは権威あるレジストリに登録し、三方一貫性検証（codes/*.rs ↔ locales
   ↔ 本ドキュメントのコード表）を経た後に射出可能。ランタイムエラー値コードの登録ソースは
   `src/std/result.rs` の `RUNTIME_ERROR_CODES` 表（診断コードと同様に
   `build.rs 構築期スレッショルド + `tools/code-tables`` 検証を受ける）。
3. E7xxx は std.io / std.net エラー値用の予約セグメント（現在空、io/net Result 化時に有効化）。
4. 射出点：std 各モジュールは `error_new(code, message)` 経由で Error 値を構築；消費側は
   `std.result.unwrap_err` で Err 媒体を取り出し、`std.result.code/message` でフィールドを読み取る。

#### 進化経路（ライン C、未実施）

パターンマッチ完全化（RFC-039）着地後、`Error` を `{ kind: ErrorKind, message: String }`
にアップグレード可能、`code`
は kind から派生する属性に変換（バリアント定義箇所がコードレジストリになる）。進化期間中、本節の code 安定契約は不変；このアップグレードは独立した決定であり、本節の約束を構成しない。

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

/// i18n 表示テキストレジストリ（コンパイル期に JSON からロード、ランタイムはテーブル参照なし）
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

    /// テンプレートをレンダリング（コンパイル期に完了、ランタイムオーバーヘッドゼロ）
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

##### 事前定義プレースホルダ（よく使われるもの）

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

##### 任意の key 対応

**params は任意の key をサポート、事前定義に限定されない**。呼び出し側は任意の `key` を渡せる：

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

> **注意**：すべてのエラーコードがプレースホルダを使用するわけではない。一部のエラーコード（例：E0001）は静的メッセージで、引数は不要。

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
# エラーメッセージ言語、選択可能：en, zh, ja, ...
default = "zh"
```

#### ユーザレベル設定

```toml
# ~/.yaoxiang/yaoxiang.toml
[language]
default = "zh"
```

#### コンパイル期言語選択

```
1. プロジェクトレベル yaoxiang.toml の language.default を読み取る
2. 未設定の場合、ユーザレベル ~/.yaoxiang/yaoxiang.toml を読み取る
3. どちらも未設定の場合、デフォルトで "en" を使用
4. コンパイラは選択された言語に基づいて I18nRegistry を作成（一度だけ）
5. すべてのエラーはこの I18nRegistry を使ってメッセージをレンダリング
```

#### テーブル参照オーバーヘッドゼロの鍵

**レンダリングはユーザプロジェクトコンパイル時に発生、ランタイムではない。**

```
┌─────────────────────────────────────────────────────────────────────────┐
│  段階 1: Rust が YaoXiang コンパイラをコンパイル                                      │
│                                                                           │
│  JSON をコンパイラバイナリにパッケージ化                                                 │
│  目的：explain コマンドが i18n データを直接読み取れるように                                  │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  段階 2: YaoXiang がユーザプロジェクトをコンパイル（レンダリングはここで発生）                          │
│                                                                           │
│  error! マクロ呼び出し時：                                                       │
│  1. yaoxiang.toml を読み取り、言語設定を取得                                      │
│  2. コンパイラバイナリから対応言語の i18n JSON をロード                                │
│  3. テンプレート + 引数 → render() → "Unknown variable: 'x'"                    │
│  4. Diagnostic.message = レンダリング済み文字列                                   │
│                                                                           │
│  AOT バイナリは最終文字列を直接格納、テンプレートなし、テーブル参照なし                            │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  段階 3: ユーザプログラムランタイム                                                  │
│                                                                           │
│  println!("{}", diagnostic.message)                                      │
│  // 最終文字列を直接出力、テーブル参照一切なし                                        │
└─────────────────────────────────────────────────────────────────────────┘
```

| コンポーネント               | 責務                             | レンダリング時期               |
| ---------------------------- | -------------------------------- | ------------------------------ |
| `I18nRegistry`               | テンプレートと表示テキストを提供 | ユーザプロジェクトコンパイル時 |
| `DiagnosticBuilder.render()` | テンプレート + 引数 → 最終文字列 | ユーザプロジェクトコンパイル時 |
| `Diagnostic.message`         | レンダリング済み文字列           | 最終結果を格納                 |
| AOT バイナリ                 | 最終文字列を含む                 | ランタイム直接使用             |

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

エラーの重大度は `DiagnosticLevel` 列挙で管理し、エラーコード番号から分離する：

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

| オプション      | 説明                                      |
| --------------- | ----------------------------------------- |
| `--lang <code>` | 言語指定 (en-US, zh-CN、デフォルト en-US) |
| `--json`        | JSON 形式出力（IDE/LSP で使用）           |
| `--json-pretty` | 整形された JSON 出力                      |
| `--examples`    | サンプルコードのみ表示                    |
| `--help`        | ヘルプ情報を表示                          |

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

### 段階一：エラーコード基盤アーキテクチャ

1. `src/diagnostics/` ディレクトリ構造の作成
2. `ErrorCode` 列挙の実装
3. `Diagnostic` と `DiagnosticLevel` の実装
4. リソースファイルディレクトリとサンプル JSON の作成

### 段階二：explain コマンド

1. `yaoxiang explain` CLI コマンドの実装
2. `--lang` と `--json` オプションのサポート
3. リソースファイル読み込みの統合
4. パラメータテンプレートレンダリングの実装

### 段階三：コンパイル期統合

1. すべてのエラー報告箇所を新システムを使用するように更新
2. メッセージテンプレート引数注入の実装
3. 言語優先順位ロジックの追加
4. ユニットテストカバレッジ

### 段階四：IDE/LSP 統合

1. LSP サーバの explain JSON 出力統合
2. IDE でのエラーコードリンク表示
3. ホバー時のエラー説明表示
4. クイック修正提案

---

## 付録

### 完全エラーコード早見表

| 範囲  | カテゴリ               |
| ----- | ---------------------- |
| E0xxx | 字句・構文解析         |
| E1xxx | 型チェック             |
| E2xxx | 意味解析               |
| E3xxx | コード生成             |
| E4xxx | ジェネリクスとtrait    |
| E5xxx | モジュールとインポート |
| E6xxx | ランタイムエラー       |
| E7xxx | I/O とシステムエラー   |
| E8xxx | 内部コンパイラエラー   |
| E9xxx | 予約                   |

### サポート言語

| コード | 言語         | 状態       |
| ------ | ------------ | ---------- |
| en-US  | English (US) | デフォルト |
| zh-CN  | 簡体字中国語 | 計画中     |

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

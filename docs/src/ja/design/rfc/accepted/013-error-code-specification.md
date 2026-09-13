---
title: 'RFC 013: エラーコード規約'
status: '採用済み'
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

本 RFC は YaoXiang コンパイラのエラーコード分類規約を提案する。Rust ライクな単層番号システムを採用し、JSON リソースファイルで多言語サポートを実現し、`yaoxiang explain`
コマンドでエラー解説機能を提供する。

## 動機

### なぜ標準化されたエラーコードが必要なのか？

1. **ユーザー体験**：エラーコードを見ることでエラータイプや重大度を素早く判断できる
2. **ドキュメント構成**：カテゴリ別にグループ化することでエラー参照ドキュメントの記述と保守が容易になる
3. **ツール統合**：IDE/LSP がエラーコードに基づいてクイック修正の提案やドキュメントリンクを提供できる
4. **国際化対応**：エラーメッセージとコードを分離することで、多言語翻訳が容易になる

### 設計目標

- **簡潔**：単層番号方式、複雑な分類ルールを覚える必要なし
- **親しみやすい**：Rust ライクなエラーメッセージ形式、ヘルプ情報とサンプル付き
- **拡張性**：リソースファイル駆動、新しいエラーや言語の追加が容易
- **ツール親和性**：explain コマンド + JSON 出力で IDE/LSP 統合をサポート

---

## 提案

### 中核設計：単層番号システム

4 桁の数字番号を採用し、コンパイル段階でグループ化する：

```
Exxxx
││││
│││└── 番号 (000-999)
││└─── コンパイル段階 (0-9)
└───── 固定接頭辞 'E'
```

### 段階区分

| 段階  | 範囲  | 説明                   |
| ----- | ----- | ---------------------- |
| **0** | E0xxx | 字句解析と構文解析     |
| **1** | E1xxx | 型チェック             |
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
/// エラーカテゴリ
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    Lexer,      // E0xxx: 字句解析と構文解析
    Parser,     // E0xxx: Parser errors
    TypeCheck,  // E1xxx: 型チェック
    Semantic,   // E2xxx: 意味解析
    Generic,    // E4xxx: ジェネリクスと trait
    Module,     // E5xxx: モジュールとインポート
    Runtime,    // E6xxx: ランタイムエラー
    Io,         // E7xxx: I/O とシステムエラー
    Internal,   // E8xxx: 内部コンパイラエラー
}
```

### エラーコード定義と汎用 Builder

**中核原則**：エラーコード定義と表示テキストの分離

- `ErrorCodeDefinition`：エラーコードのメタデータ（code、category、template）、表示テキストは含まない
- `locales/*.json`：各言語の表示テキスト（title、message、help、エラーコードはネストオブジェクト）
- `DiagnosticBuilder`：汎用ビルダー、trait-per-error 設計を代替

#### エラーコード定義

```rust
// diagnostic/codes/mod.rs

use crate::util::span::Span;
use crate::util::diagnostic::{Diagnostic, Severity};

/// エラーコード定義（メタデータのみ、表示テキストは i18n ファイル内）
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
        // テンプレート内のすべての {key} に対応するパラメータがあるか確認
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

| 特性                             | 説明                                                     |
| -------------------------------- | -------------------------------------------------------- |
| **単一 Builder**                 | 一つの `DiagnosticBuilder` ですべてのエラーコードに対応  |
| **型安全**                       | ショートカットメソッドがパラメータの正しさを保証         |
| **自己文書化**                   | `E1001::unknown_variable(name)` で一目瞭然               |
| **テンプレート分離**             | メッセージテンプレートとコードが分離され、i18n が容易    |
| **ランタイムオーバーヘッドゼロ** | コンパイル時レンダリング、AOT バイナリにテーブル参照なし |

---

### エラーマクロの簡略化

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

/// 使用：パラメータのみを渡す、span と i18n は自動注入
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

## 詳細設計

### エラーコード一覧

#### E0xxx：字句解析と構文解析

<!-- code-table:E0xxx start -->

| コード | 説明                       |
| ------ | -------------------------- |
| E0001  | 無効な文字                 |
| E0002  | 無効な数値リテラル         |
| E0003  | 未終端文字列               |
| E0004  | 無効な文字リテラル         |
| E0010  | 予期されたトークン         |
| E0011  | 予期しないトークン         |
| E0012  | 無効な構文                 |
| E0013  | 一致しない括弧             |
| E0014  | セミコロン不足             |
| E0016  | 式を期待                   |
| E0018  | キーワードを名前として使用 |

<!-- code-table:E0xxx end -->

#### E1xxx：型チェック

<!-- code-table:E1xxx start -->

| コード | 説明                                             |
| ------ | ------------------------------------------------ |
| E1001  | 未知の変数                                       |
| E1002  | 型不一致                                         |
| E1003  | 未知の型                                         |
| E1010  | 引数の数が一致しない                             |
| E1011  | 引数の型が一致しない                             |
| E1012  | 戻り値の型が一致しない                           |
| E1013  | 関数が見つからない                               |
| E1020  | 型を推論できない                                 |
| E1021  | 型推論の競合                                     |
| E1030  | パターンが不完全                                 |
| E1031  | 到達不能パターン                                 |
| E1040  | 操作がサポートされない                           |
| E1041  | インデックス範囲外                               |
| E1042  | フィールドが見つからない                         |
| E1050  | ブールオペランドが必要                           |
| E1051  | 論理 NOT はブールオペランドを必要とする          |
| E1052  | 無効な逆参照                                     |
| E1053  | 非構造体のフィールドアクセス                     |
| E1054  | 条件型が一致しない                               |
| E1055  | 非ジェネリクスコンテキストでの制約               |
| E1060  | 型引数の数が一致しない                           |
| E1061  | ジェネリクスをインスタンス化できない             |
| E1062  | const ジェネリクス制約の失敗                     |
| E1064  | 位置バインディングのインデックスが無効           |
| E1071  | 型定義はモジュールレベルでのみ可能               |
| E1081  | `?` は Result を返す関数内でのみ使用可能         |
| E1082  | `?` は Result 式にのみ使用可能                   |
| E1083  | `?` のエラー型が一致しない                       |
| E1090  | ✨ 語るに値しない ✨                             |
| E1091  | 無効なジェネリクスのメタ型                       |
| E1092  | 精化型引数の形式が不正                           |
| E1093  | 精化引数の数が一致しない                         |
| E1094  | 未使用のコンパイル時値パラメータ                 |
| E1095  | 未知のインターフェース                           |
| E1096  | インターフェース引数の数が一致しない             |
| E1097  | インターフェースメンバーの名前衝突               |
| E1098  | インターフェースメソッドが未実装                 |
| E1099  | インターフェースメソッドのシグネチャが一致しない |
| E1100  | インターフェースメソッドの重複実装               |
| E1101  | 型がインターフェースを実装していない             |
| E1102  | ループ外のループ制御文                           |

<!-- code-table:E1xxx end -->

#### E2xxx：意味解析

<!-- code-table:E2xxx start -->

| コード | 説明                                       |
| ------ | ------------------------------------------ |
| E2001  | スコープエラー                             |
| E2002  | 重複定義                                   |
| E2003  | 所有権エラー                               |
| E2010  | 不変変数への代入                           |
| E2011  | 未初期化変数の使用                         |
| E2012  | 可変性競合                                 |
| E2013  | 変数のシャドウィング                       |
| E2014  | 移動済み値の使用                           |
| E2016  | 不変変数への代入                           |
| E2018  | 可変/不変借用競合                          |
| E2019  | 二重解放                                   |
| E2020  | 解放後の使用                               |
| E2027  | unsafe な逆参照                            |
| E2029  | spawn 内の参照ループ                       |
| E2030  | 精化型制約違反                             |
| E2090  | 無効なシグネチャ                           |
| E2091  | シグネチャに未知の型                       |
| E2092  | シグネチャに矢印がない                     |
| E2093  | 重複するパラメータ名                       |
| E2094  | ジェネリクスパラメータのシャドウィング     |
| E2095  | パラメータ名がジェネリクスをシャドウィング |

<!-- code-table:E2xxx end -->

#### E3xxx：コード生成

<!-- code-table:E3xxx start -->

| コード | 説明                                                       |
| ------ | ---------------------------------------------------------- |
| E3004  | サポートされないイテレータ                                 |
| E3005  | IR 生成エラー                                              |
| E3006  | 未解決変数                                                 |
| E3007  | トップレベルバインディングの初期化は定数でなければならない |
| E3008  | サポートされない match パターン                            |
| E3014  | レジスタオーバーフロー                                     |
| E3017  | 無効なオペランド（コード生成）                             |

<!-- code-table:E3xxx end -->

#### E4xxx：ジェネリクスと trait

<!-- code-table:E4xxx start -->

| コード | 説明                 |
| ------ | -------------------- |
| E4001  | ジェネリクス制約違反 |
| E4002  | trait が見つからない |
| E4003  | trait 実装の欠落     |
| E4004  | trait 実装の競合     |
| E4005  | 関連型が見つからない |
| E4010  | 定数のゼロ除算       |
| E4011  | 定数オーバーフロー   |
| E4012  | 定数再帰が深すぎる   |
| E4014  | 定数評価失敗         |
| E4018  | 精化述語違反         |
| E4019  | 型等式が成立しない   |
| E4020  | 証明関数が必要       |

<!-- code-table:E4xxx end -->

> E4006/E8004 は現在発射ポイントなし（予約コード）：Sized 制約と最適化エラーパスは未実装、実装時に実際のトリガー面に従って配線する。

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
| E6009  | Range のステップが不正           |
| E6010  | 整数解析失敗                     |
| E6011  | 浮動小数点解析失敗               |

<!-- code-table:E6xxx end -->

> **コード表改訂（2026-08-09）**：コード表は元々は Rust セマンティクス草案（Assertion
> failed/Arithmetic overflow/Heap allocation failed/Type cast
> failed）に基づいて定義されており、実装の実際の要件と一致していなかった。YaoXiang には null ポインタ/ヒープ割り当て失敗/型変換の概念がなく（値セマンティクス +
> Rust メモリ安全性）、ランタイムオーバーフローパスには検出が実装されていない。校正後：
>
> - E6002 削除（元 Assertion
>   failed は E6005 に移動；元 null ポインタセマンティクスは言語概念に存在しない）
> - E6003 を Arithmetic overflow から Runtime index out of bounds に変更（実際のトリガー面）
> - E6005 を Heap allocation failed から Assertion failed に変更（std.assert の実際のパス）
> - E6006 を Runtime index out of bounds から Function not
>   found に変更（実装は以前よりそうなっていた）
> - E6007 を Type cast failed から汎用 Runtime
>   error に変更（ExecutorError のマッピングされていないバリアントの統一落点）

#### E7xxx：I/O とシステムエラー

<!-- code-table:E7xxx start -->

| コード | 説明                   |
| ------ | ---------------------- |
| E7001  | ファイルが見つからない |
| E7002  | 権限が拒否された       |
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

| コード | 説明                                 |
| ------ | ------------------------------------ |
| W1001  | 未使用のプライベート関数             |
| W1002  | 未使用のプライベート型               |
| W1003  | 未使用のインポート                   |
| W1004  | 未使用のプライベート変数             |
| W1005  | 未使用のプライベートメソッド         |
| W1063  | const ジェネリクス制約を評価できない |
| W1080  | コンパイル時証明の降格               |

<!-- code-table:W1xxx end -->

> W コード位置ルール：E コードとアイソモルフィックに段階でグループ化（W+段階千位セグメント）、W1xxx
> = 型チェック段階の警告。
>
> **デッドコードのコード意味（#321 決定 B）**：`pub`
> 定義は外部インターフェースであり、絶対に報告しない ― 外部コンシューマーが使用するかどうかは単一ファイル分析の境界を超えるため、誤検知しない方がよい。W1001/W1002/W1004/W1005 は
> **プライベート（pub でない）定義のみ**を対象とする：`main` と `pub`
> 定義から到達可能性分析を開始し、一度も参照されない場合に報告。メソッド（W1005）は呼び出しポイントの短い名前で照合する。bin/lib
> target セマンティクス（bin 内で未使用の pub を警告）は将来の拡張（#289 方案 A）、プロジェクトモデルのサポートが必要。
>
> **未使用インポート（W1003）**：typecheck の use
> elaboration で検出（pass2 でインポートのローカル名を登録、式解決と型注釈位置でヒットした時点で使用済みと見なす）、全体インポート（`use std.io`
> → モジュールエイリアス）と名前付きインポート（`use std.io.{print}`）の両方をカバー。
>
> **発射チャネル**：W コード診断は builder が W 接頭辞でデフォルト `Severity::Warning`
> とラベル付け（明示的指定が優先）、収集と表示はエラーと同じトラック（`warning[W####]`
> 接頭辞でレンダリング）だが、コンパイルをブロックせず、成功終了コードにも影響しない。
> `yaoxiang check --deny-warnings`
> は警告を失敗に昇格させる（警告が存在する場合に非ゼロコードで終了）、CI 厳格モード用。per-code 抑制（allow 属性など）は今後の拡張項目。

### メッセージ品質規約

> 本節はメッセージ単一トラックと品質改訂（2026-09-03）によって導入された。`scripts/audit_diagnostics.py`
> によって CI で強制実行される。

1. **メッセージ単一トラック**：すべてのユーザー可視診断メッセージは権威あるレジストリのショートカットメソッド +
   locales テンプレートレンダリングを経由しなければならず、コードは構造化パラメータのみを渡す。レジストリをバイパスして
   `Diagnostic::error(...)`
   などのネイティブ値を直接構築することは禁止 ― このパスはコード検証と i18n をバイパスする。
2. **コード合法性**：未登録コードと疑似コード（例：`E_INTERNAL`）の使用は禁止。使用ポイントのコードリテラルはレジストリで定義済みでなければならない。内部エラーはすべて E8001（`internal_error`）にフォールバック。
3. **型表示**：型の Display はインスタンス化前後の形式を区別しなければならない（`Expected 'Container', found 'Container'`
   のベア名は区別できない）。
4. **ソルバー内部状態の隔離**：ソルバー中間状態の TypeVar（Display 形式
   `t<N>`）はユーザー可視メッセージに入ってはならない。テストアンカー：`test_type_error_message_no_solver_typevar_leak`。
5. **E8xxx 境界**：E8xxx はコンパイラの内部一貫性问题（ICE）のみに使用する。ユーザーが修正可能なエラーに E8001 をフォールバックとして使用することは禁止。ICE メッセージには最小限の再現ガイダンスを添付しなければならない。

---

### ランタイムエラー値とコードの貫通

> 本節はランタイム Error 値とコードの改訂（2026-09-03）によって導入された。E6xxx/E7xxx セマンティクス空間は二つのチャネルを同時に担い、コード空間は同一、表示チャネルは異なる。

#### 二つのチャネル

| チャネル                     | キャリア                                                  | 表示方式                                                 |
| ---------------------------- | --------------------------------------------------------- | -------------------------------------------------------- |
| コンパイラ/CLI 診断チャネル  | `ExecutorError` などのホスト層ハードエラー                | stderr `error[E####]:`（既に配線済み E6003/E6005/E6007） |
| プログラム内エラー値チャネル | std ライブラリ `Result(T, Error)` の Err キャリア `Error` | 言語値、プログラムが match/比較で消費する                |

#### Error 構造（v0.8 以降、破壊的変更）

```
Error { code: String, message: String }
```

- `code` は本規約の E6xxx/E7xxx 番号を再利用、文字列形式（例：`"E6008"`）。
- **安定契約**：割り当てられたコードはバージョン間でセマンティクスが変更されない；同じセマンティクスに削除済みコード（E6002 の前例）を再利用しない。
- **消費面**：プログラム内の `e.code == "E6xxx"`
  比較が唯一のプログラマブル判定契約；`yaoxiang explain E6xxx` ドキュメント貫通；ツールチェーン（LSP
  / DAP、RFC-034 参照）はコードを exceptionId として使用する。
- **アクセサ**：`std.result.code(e)` / `std.result.message(e)`。
- **ユーザー定義エラー**：`Result(T, E)`
  の E はジェネリクスパラメータ、真剣にモデリングする場合はユーザー定義型を取る；std `Error`
  は単なる便利なフォールバックキャリアであり、そのコード体系はユーザー E 型を制約しない。

#### コード割り当てルール

1. ランタイムエラー値コードとコンパイラ診断コードは E6xxx/E7xxx 空間を共有し、新しいコードは**実際のトリガー面**に従って割り当て、想像上のシナリオのために予約しない。
2. まず登録してから使用：新しいコードは権威あるレジストリに登録し、三方一貫性検証（codes/*.rs ↔
   locales
   ↔ 本ドキュメントのコード表）を経た後にのみ発射可能。ランタイムエラー値コードの登録ソースは
   `src/std/result.rs` の `RUNTIME_ERROR_CODES` テーブル（診断コードと同様に
   `build.rs ビルド時閾値 + `tools/code-tables`` 検証を受ける）。
3. E7xxx は std.io / std.net エラー値用に予約されたセグメント（現在空、io/net
   Result 化時に有効化）。
4. 発射ポイント：std の各モジュールは `error_new(code, message)` を介して Error 値を構築；消費側は
   `std.result.unwrap_err` で Err キャリアを取得し、`std.result.code/message`
   でフィールドを読み取る。

#### 進化パス（ライン C、未実施）

パターンマッチング完備化（RFC-039）が実装された後、`Error` を `{ kind: ErrorKind, message: String }`
にアップグレードでき、`code`
は kind から派生した属性に変換される（バリアント定義箇所がコードレジストリになる）。進化期間中、本節のコード安定契約は変更されない；このアップグレードは独立した決定であり、本節の約束を構成しない。

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

/// i18n 表示テキストレジストリ（コンパイル時に JSON から読み込み、ランタイムはテーブル参照なし）
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

    /// テンプレートをレンダリング（コンパイル時に完了、ランタイムオーバーヘッドゼロ）
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

##### 定義済みプレースホルダ（よく使用される）

| プレースホルダ | 用途                             | 例                                  |
| -------------- | -------------------------------- | ----------------------------------- |
| `{name}`       | 変数名/型名/trait 名などの識別子 | `Unknown variable: '{name}'`        |
| `{expected}`   | 期待される型                     | `Expected type '{expected}'`        |
| `{found}`      | 実際の/見つかった型              | `, found type '{found}'`            |
| `{method}`     | メソッド名                       | `Method {method} is not a function` |
| `{trait}`      | trait 名                         | `Cannot find trait: {trait}`        |
| `{path}`       | モジュールパス                   | `Invalid path: {path}`              |
| `{ty}`         | 型式                             | `Invalid type: {ty}`                |
| `{message}`    | 内部エラーメッセージ             | `Internal error: {message}`         |

##### 任意のキーサポート

**params は任意のキーをサポート、定義済みに限定されない**。呼び出し側は任意の `key`
を渡すことができる：

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

### yaoxiang.toml 設定

#### プロジェクトレベル設定

```toml
# yaoxiang.toml
[project]
name = "my-project"
version = "0.1.0"

[language]
# エラーメッセージの言語、選択可能：en, zh, ja, ...
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
1. プロジェクトレベルの yaoxiang.toml から language.default を読み込む
2. 未設定の場合、ユーザーレベルの ~/.yaoxiang/yaoxiang.toml を読み込む
3. どちらも未設定の場合、デフォルトで "en" を使用
4. コンパイラは選択された言語に基づいて I18nRegistry を作成（一度だけ）
5. すべてのエラーはその I18nRegistry を使用してメッセージをレンダリング
```

#### テーブル参照オーバーヘッドゼロの鍵

**レンダリングはユーザープロジェクトのコンパイル時に発生し、ランタイムではない。**

```
┌─────────────────────────────────────────────────────────────────────────┐
│  段階 1: Rust が YaoXiang コンパイラをコンパイル                                       │
│                                                                           │
│  JSON をコンパイラバイナリにパッケージ化                                                 │
│  目的：explain コマンドが i18n データを直接読み取れるようにする                                  │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  段階 2: YaoXiang がユーザープロジェクトをコンパイル（レンダリングはここで発生）                    │
│                                                                           │
│  error! マクロ呼び出し時：                                                       │
│  1. yaoxiang.toml を読み込んで言語設定を取得                                      │
│  2. コンパイラバイナリから対応する言語の i18n JSON を読み込む                                │
│  3. テンプレート + パラメータ → render() → "Unknown variable: 'x'"                    │
│  4. Diagnostic.message = レンダリング済み文字列                                   │
│                                                                           │
│  AOT バイナリは最終文字列を直接格納、テンプレートなし、テーブル参照なし                            │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  段階 3: ユーザープログラム実行時                                                  │
│                                                                           │
│  println!("{}", diagnostic.message)                                      │
│  // 最終文字列を直接出力、テーブル参照なし                                        │
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

エラーの重大度は `DiagnosticLevel` 列挙で管理され、エラーコード番号とは分離されている：

```rust
pub enum DiagnosticLevel {
    Error,    // コンパイル失敗につながる
    Warning,  // コンパイルには影響しないが、修正を推奨
    Note,     // 補足情報
    Help,     // 修正提案
}
```

| レベル  | 接頭辞            | 説明                     |
| ------- | ----------------- | ------------------------ |
| Error   | `error[E####]:`   | コンパイル失敗につながる |
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
2. 移行期間中は新旧コードを同時に表示
3. 廃止タイムテーブルを提供

---

## 実装戦略

### 段階一：エラーコードインフラストラクチャ

1. `src/diagnostics/` ディレクトリ構造を作成
2. `ErrorCode` 列挙を実装
3. `Diagnostic` と `DiagnosticLevel` を実装
4. リソースファイルディレクトリとサンプル JSON を作成

### 段階二：explain コマンド

1. `yaoxiang explain` CLI コマンドを実装
2. `--lang` と `--json` オプションをサポート
3. リソースファイル読み込みを統合
4. パラメータテンプレートレンダリングを実装

### 段階三：コンパイル時統合

1. すべてのエラー報告ポイントを新システムを使用するように更新
2. メッセージテンプレートパラメータ注入を実装
3. 言語優先度ロジックを追加
4. 単体テストカバレッジ

### 段階四：IDE/LSP 統合

1. LSP サーバーが explain JSON 出力を統合
2. IDE でエラーコードリンクを表示
3. ホバーでエラー解説を表示
4. クイック修正提案

---

## 付録

### 完全エラーコード早見表

| 範囲  | カテゴリ               |
| ----- | ---------------------- |
| E0xxx | 字句解析と構文解析     |
| E1xxx | 型チェック             |
| E2xxx | 意味解析               |
| E3xxx | コード生成             |
| E4xxx | ジェネリクスと trait   |
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

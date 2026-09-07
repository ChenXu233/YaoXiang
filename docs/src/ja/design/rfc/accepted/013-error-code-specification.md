---
title: 'RFC 013: エラーコード規約'
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

# RFC 013: エラーコード規約

## 概要

本 RFC は YaoXiang コンパイラのエラーコード分類規約を提案する。Rust ライクな単層番号システムを採用し、JSON リソースファイルと組み合わせることで多言語サポートを実現し、`yaoxiang explain`
コマンドによってエラー説明機能を提供する。

## 動機

### なぜ標準化されたエラーコードが必要か？

1. **ユーザー体験**：エラーコードを見ることで、エラーの種類や重大度をすばやく判断できる
2. **ドキュメントの整理**：カテゴリ別にグループ化することで、エラー参考ドキュメントの執筆と保守が容易になる
3. **ツール統合**：IDE/LSP がエラーコードに基づいてクイックフィックスやドキュメントリンクを提供できる
4. **国際化サポート**：エラーメッセージとコードを分離することで、多言語への翻訳が容易になる

### 設計目標

- **簡潔**：単層番号方式により、ユーザーが複雑な分類ルールを覚える必要がない
- **親しみやすい**：Rust ライクなエラーメッセージ形式を採用し、ヘルプ情報とサンプルを併記
- **拡張性**：リソースファイル駆動により、新しいエラーや新しい言語の追加が容易
- **ツールフレンドリ**：explain コマンド + JSON 出力により、IDE/LSP との統合をサポート

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
| **0** | E0xxx | 字句解析と構文解析     |
| **1** | E1xxx | 型検査                 |
| **2** | E2xxx | 意味解析               |
| **3** | E3xxx | コード生成             |
| **4** | E4xxx | ジェネリクスとトレイト |
| **5** | E5xxx | モジュールとインポート |
| **6** | E6xxx | 実行時エラー           |
| **7** | E7xxx | I/O とシステムエラー   |
| **8** | E8xxx | 内部コンパイラエラー   |
| **9** | E9xxx | 予約/実験的            |

### エラーカテゴリ列挙

```rust
/// エラーカテゴリ
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    Lexer,      // E0xxx: 字句および構文解析
    Parser,     // E0xxx: Parser errors
    TypeCheck,  // E1xxx: 型検査
    Semantic,   // E2xxx: 意味解析
    Generic,    // E4xxx: ジェネリクスとトレイト
    Module,     // E5xxx: モジュールとインポート
    Runtime,    // E6xxx: 実行時エラー
    Io,         // E7xxx: I/O とシステムエラー
    Internal,   // E8xxx: 内部コンパイラエラー
}
```

### エラーコード定義と汎用 Builder

**基本原則**：エラーコード定義と表示文言を分離する

- `ErrorCodeDefinition`：エラーコードのメタデータ（code、category、template）。表示文言は含まない
- `locales/*.json`：各言語の表示文言（title、message、help。エラーコードはネストオブジェクト）
- `DiagnosticBuilder`：汎用ビルダー。trait-per-error 設計に代わるもの

#### エラーコード定義

```rust
// diagnostic/codes/mod.rs

use crate::util::span::Span;
use crate::util::diagnostic::{Diagnostic, Severity};

/// エラーコード定義（メタデータのみ。表示文言は i18n ファイル）
#[derive(Debug, Clone, Copy)]
pub struct ErrorCodeDefinition {
    pub code: &'static str,
    pub category: ErrorCategory,
    pub message_template: &'static str,  // メッセージテンプレート。{param} プレースホルダをサポート
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

| 特性                         | 説明                                                                   |
| ---------------------------- | ---------------------------------------------------------------------- |
| **単一 Builder**             | 1 つの `DiagnosticBuilder` ですべてのエラーコードに対応                |
| **型安全**                   | ショートカットメソッドにより引数の正確性を保証                         |
| **自己文書化**               | `E1001::unknown_variable(name)` で一目瞭然                             |
| **テンプレート分離**         | メッセージテンプレートとコードを分離し、i18n が容易                    |
| **ゼロ実行時オーバーヘッド** | コンパイル時にレンダリングされ、AOT バイナリにテーブルルックアップなし |

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
}

/// 使用法：引数だけを渡せば、span と i18n は自動注入される
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

| コード | 説明                                 |
| ------ | ------------------------------------ |
| E0001  | 無効な文字                           |
| E0002  | 無効な数値リテラル                   |
| E0003  | 終端されていない文字列               |
| E0004  | 無効な文字リテラル                   |
| E0010  | 予期されるトークン                   |
| E0011  | 予期しないトークン                   |
| E0012  | 無効な構文                           |
| E0013  | 対応しない括弧                       |
| E0014  | セミコロン不足                       |
| E0016  | 予期される式                         |
| E0018  | キーワードが名前として使用されている |

<!-- code-table:E0xxx end -->

#### E1xxx：型検査

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
| E1031  | 到達不可能パターン                               |
| E1040  | 操作がサポートされていない                       |
| E1041  | インデックスが範囲外                             |
| E1042  | フィールドが見つからない                         |
| E1050  | ブール被演算子が必要                             |
| E1051  | 論理 NOT にはブール被演算子が必要                |
| E1052  | 無効なデリファレンス                             |
| E1053  | 非構造体フィールドアクセス                       |
| E1054  | 条件型が一致しない                               |
| E1055  | 非ジェネリクスコンテキストでの制約               |
| E1060  | 型引数の数が一致しない                           |
| E1061  | ジェネリクスをインスタンス化できない             |
| E1062  | const ジェネリクス制約失敗                       |
| E1064  | バインディング位置のインデックスが無効           |
| E1071  | 型定義はモジュールレベルでのみ可能               |
| E1081  | `?` は Result を返す関数内でのみ使用可能         |
| E1082  | `?` は Result 式にのみ使用可能                   |
| E1083  | `?` のエラー型が一致しない                       |
| E1090  | ✨ 名状しがたい ✨                               |
| E1091  | 無効なジェネリクスメタ型                         |
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
| E1102  | ループ外でのループ制御文                         |

<!-- code-table:E1xxx end -->

#### E2xxx：意味解析

<!-- code-table:E2xxx start -->

| コード | 説明                                     |
| ------ | ---------------------------------------- |
| E2001  | スコープエラー                           |
| E2002  | 重複定義                                 |
| E2003  | 所有権エラー                             |
| E2010  | 不変への代入                             |
| E2011  | 未初期化変数の使用                       |
| E2012  | 可変性競合                               |
| E2013  | 変数のシャドウイング                     |
| E2014  | 移動済み値の使用                         |
| E2016  | 不変への代入                             |
| E2018  | 可変/不変借用競合                        |
| E2019  | 二重解放                                 |
| E2020  | 解放後使用                               |
| E2027  | unsafe デリファレンス                    |
| E2029  | spawn 内の参照ループ                     |
| E2090  | 無効なシグネチャ                         |
| E2091  | シグネチャの未知の型                     |
| E2092  | シグネチャに矢印がない                   |
| E2093  | 引数名の重複                             |
| E2094  | ジェネリクスパラメータのシャドウイング   |
| E2095  | 引数名によるジェネリクスのシャドウイング |

<!-- code-table:E2xxx end -->

#### E3xxx：コード生成

<!-- code-table:E3xxx start -->

| コード | 説明                                                       |
| ------ | ---------------------------------------------------------- |
| E3004  | サポートされていないイテレータ                             |
| E3005  | IR 生成エラー                                              |
| E3006  | 未解決変数                                                 |
| E3007  | トップレベルバインディングの初期化は定数でなければならない |
| E3014  | レジスタオーバーフロー                                     |
| E3017  | 無効なオペランド（コード生成）                             |

<!-- code-table:E3xxx end -->

#### E4xxx：ジェネリクスとトレイト

<!-- code-table:E4xxx start -->

| コード | 説明                   |
| ------ | ---------------------- |
| E4001  | ジェネリクス制約違反   |
| E4002  | トレイトが見つからない |
| E4003  | トレイト実装の欠落     |
| E4004  | トレイト実装の競合     |
| E4005  | 関連型が見つからない   |
| E4010  | 定数のゼロ除算         |
| E4011  | 定数オーバーフロー     |
| E4012  | 定数再帰が深すぎます   |
| E4014  | 定数評価失敗           |
| E4018  | 精化述語違反           |
| E4019  | 型等式が成立しません   |
| E4020  | 証明関数が必要         |

<!-- code-table:E4xxx end -->

> E4006/E8004 は現在発射ポイントなし（予約コード）：Sized 制約と最適化エラーパスは未実装。実装時には実際のトリガー面に従って配線する。

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

#### E6xxx：実行時エラー

<!-- code-table:E6xxx start -->

| コード | 説明                         |
| ------ | ---------------------------- |
| E6001  | ゼロ除算エラー               |
| E6003  | 実行時インデックス範囲外     |
| E6004  | スタックオーバーフロー       |
| E6005  | アサーション失敗             |
| E6006  | 関数が見つからない（実行時） |
| E6007  | 実行時エラー                 |
| E6008  | キーが存在しない             |
| E6009  | Range のステップ値が不正     |
| E6010  | 整数解析失敗                 |
| E6011  | 浮動小数点解析失敗           |

<!-- code-table:E6xxx end -->

> **コード表改訂（2026-08-09）**：コード表は元来 Rust セマンティクス草案（Assertion
> failed/Arithmetic overflow/Heap allocation failed/Type cast
> failed）に基づいて定義されていたが、実装の実際の要件と一致していなかった。YaoXiang には null ポインタ・ヒープ割り当て失敗・型変換の概念がなく（値セマンティクス +
> Rust メモリ安全性）、実行時オーバーフロー経路は検出が実装されていない。キャリブレーション後：
>
> - E6002 削除（元 Assertion
>   failed は E6005 に移動。元の null ポインタセマンティクスは言語に概念なし）
> - E6003 を Arithmetic overflow から Runtime index out of bounds に変更（実際のトリガー面）
> - E6005 を Heap allocation failed から Assertion failed に変更（std.assert の実際のパス）
> - E6006 を Runtime index out of bounds から Function not
>   found に変更（実装はすでにそうなっていた）
> - E6007 を Type cast failed から汎用 Runtime
>   error に変更（ExecutorError のマッピングされていないバリアントの統一フォールバック）

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
| E8002  | 予期しないパニック   |
| E8003  | コンパイラ段階エラー |

<!-- code-table:E8xxx end -->

#### W1xxx：警告コード

<!-- code-table:W1xxx start -->

| コード | 説明                                   |
| ------ | -------------------------------------- |
| W1001  | 未使用のエクスポート関数               |
| W1002  | 未使用のエクスポート型                 |
| W1003  | 未使用のインポート                     |
| W1004  | 未使用のエクスポート変数               |
| W1005  | 未使用のエクスポートメソッド           |
| W1063  | const ジェネリクス制約を評価できません |
| W1080  | コンパイル時証明の降格                 |

<!-- code-table:W1xxx end -->

> W コード位置ルール：E コードと同型で段階別にグループ化（W+段階千位セグメント）。W1xxx
> = 型検査段階の警告。
>
> **発射チャネル**：W コード診断は builder が W プレフィックスによりデフォルトで `Severity::Warning`
> を付与（明示的指定が優先）。収集と表示はエラーと同一トラック（`warning[W####]`
> プレフィックスでレンダリング）だが、コンパイルは中断せず、成功終了コードにも影響しない。
> `yaoxiang check --deny-warnings`
> は警告を失敗に昇格させる（警告が存在する場合、非ゼロコードで終了）。CI 厳格モード用。per-code 抑制（allow 属性など）は今後の拡張項目。

### メッセージ品質規約

> 本節はメッセージ単一トラックと品質改訂（2026-09-03）により導入された。`scripts/audit_diagnostics.py`
> によって CI で強制執行される。

1. **メッセージ単一トラック**：すべてのユーザー可視診断メッセージは、権威あるレジストリのショートカットメソッド +
   locales テンプレートレンダリングを経由しなければならない。コードは構造化パラメータのみを渡し、レジストリを迂回して
   `Diagnostic::error(...)`
   などのネイティブ値を直接構築することは禁止されている。この経路はコード検証と i18n を迂回する。
2. **コード合法性**：未登録コードや仮コード（例：`E_INTERNAL`）の使用は禁止されている。使用するコードリテラルはレジストリで定義済みでなければならない。内部エラーはすべて E8001（`internal_error`）にフォールバックする。
3. **型表示**：型の Display はインスタンス化前後の形態を区別しなければならない（`Expected 'Container', found 'Container'`
   のように裸名では区別できない）。
4. **ソルバー内部状態の隔離**：ソルバー中間状態の TypeVar（Display 形態
   `t<N>`）はユーザー可視メッセージに入ってはならない。テストアンカー：`test_type_error_message_no_solver_typevar_leak`。
5. **E8xxx 境界**：E8xxx はコンパイラの内部整合性问题（ICE）にのみ使用する。ユーザーが修正可能なエラーに E8001 をフォールバックとして使用することは禁止されている。ICE メッセージには最小再現手順を添付しなければならない。

---

### 実行時エラー値とコードの貫通

> 本節は実行時 Error 値とコードの改訂（2026-09-03）により導入された。E6xxx/E7xxx セマンティクス空間は 2 つのチャネルを同時に担い、コード空間は同一、表示チャネルは異なる。

#### 2 つのチャネル

| チャネル                     | 载体                                                  | 表示方式                                             |
| ---------------------------- | ----------------------------------------------------- | ---------------------------------------------------- |
| コンパイラ/CLI 診断チャネル  | `ExecutorError` などのホスト層ハードエラー            | stderr `error[E####]:`（E6003/E6005/E6007 配線済み） |
| プログラム内エラー値チャネル | std ライブラリ `Result(T, Error)` の Err 载体 `Error` | 言語値。プログラムが match/比較で消費                |

#### Error 構造（v0.8 から、破壊的変更）

```
Error { code: String, message: String }
```

- `code` は本規約の E6xxx/E7xxx 番号を再利用し、文字列形態（例：`"E6008"`）。
- **安定性契約**：割り当て済みのコードはバージョン間でセマンティクスが変わらない。同じセマンティクスに削除済みコード（E6002 の前例）を再利用しない。
- **消費面**：プログラム内の `e.code == "E6xxx"`
  比較が唯一のプログラマブル判定契約。`yaoxiang explain E6xxx`
  ドキュメントも貫通。ツールチェーン（LSP /
  DAP、RFC-034 参照）はコードを exceptionId として使用する。
- **アクセサ**：`std.result.code(e)` / `std.result.message(e)`。
- **ユーザー定義エラー**：`Result(T, E)`
  の E はジェネリクスパラメータ。真面目にモデル化する場合はユーザー定義型を使用。std `Error`
  は単なる便利なフォールバック载体であり、そのコード体系はユーザー E 型を制約しない。

#### コード割り当てルール

1. 実行時エラー値コードとコンパイラ診断コードは E6xxx/E7xxx 空間を共有する。新規コードは**実際のトリガー面**に基づいて割り当て、想像上のシナリオのために予約しない。
2. 先に登録してから使用：新規コードは権威あるレジストリに登録し、三者間一貫性検証（codes/*.rs ↔
   locales ↔ 本ドキュメントのコード表）を経た後にのみ発射可能。実行時エラー値コードの登録ソースは
   `src/std/result.rs` の `RUNTIME_ERROR_CODES` テーブル（診断コードと同様に
   `build.rs 構築時閾値 + `tools/code-tables`` 検証を受ける）。
3. E7xxx は std.io /
   std.net エラー値用に予約されたセグメント（現在空枠。io/net の Result 化時に有効化）。
4. 発射ポイント：std の各モジュールは `error_new(code, message)` で Error 値を構築。消費側は
   `std.result.unwrap_err` で Err 载体を取り出し、`std.result.code/message` でフィールドを読み取る。

#### 進化パス（線 C、未実施）

パターンマッチング完備化（RFC-039）が実装された後、`Error` は `{ kind: ErrorKind, message: String }`
にアップグレード可能。`code`
は kind から派生する属性に変換される（バリアント定義箇所がコードレジストリになる）。進化期間中、本節のコード安定性契約は変わらない。このアップグレードは独立した決定であり、本節の約束を構成しない。

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

/// i18n 表示文言レジストリ（コンパイル時に JSON から読み込み、実行時はゼロルックアップ）
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

    /// テンプレートをレンダリング（コンパイル時に完了。実行時はゼロオーバーヘッド）
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

##### 定義済みプレースホルダ（一般）

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

##### 任意の key サポート

**params は任意の key をサポートし、定義済みプレースホルダに限定されない**。呼び出し側は任意の `key`
を渡すことができる：

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

> **注意**：すべてのエラーコードがプレースホルダを使用するわけではない。一部のエラーコード（例：E0001）は静的メッセージで、引数を必要としない。

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
# エラーメッセージの言語。オプション：en, zh, ja, ...
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
4. コンパイラは選択された言語に基づいて I18nRegistry を作成（1 回のみ）
5. すべてのエラーはその I18nRegistry を使用してメッセージをレンダリング
```

#### ゼロテーブルルックアップオーバーヘッドの鍵

**レンダリングはユーザープロジェクトのコンパイル時に行われ、実行時ではない。**

```
┌─────────────────────────────────────────────────────────────────────────┐
│  段階 1: Rust が YaoXiang コンパイラをコンパイル                          │
│                                                                           │
│  JSON がコンパイラバイナリにバンドルされる                                  │
│  目的：explain コマンドが i18n データを直接読み取れるように                   │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  段階 2: YaoXiang がユーザープロジェクトをコンパイル（レンダリングはここで発生） │
│                                                                           │
│  error! マクロ呼び出し時：                                                  │
│  1. yaoxiang.toml を読み込んで言語設定を取得                                │
│  2. コンパイラバイナリから対応する言語の i18n JSON をロード                    │
│  3. テンプレート + 引数 → render() → "Unknown variable: 'x'"              │
│  4. Diagnostic.message = レンダリング済み文字列                            │
│                                                                           │
│  AOT バイナリには最終文字列が直接格納され、テンプレートもテーブルルックアップもない │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  段階 3: ユーザープログラムの実行時                                         │
│                                                                           │
│  println!("{}", diagnostic.message)                                      │
│  // 最終文字列を直接出力。テーブルルックアップは一切なし                      │
└─────────────────────────────────────────────────────────────────────────┘
```

| コンポーネント               | 責務                             | レンダリングタイミング             |
| ---------------------------- | -------------------------------- | ---------------------------------- |
| `I18nRegistry`               | テンプレートと表示文言を提供     | ユーザープロジェクトのコンパイル時 |
| `DiagnosticBuilder.render()` | テンプレート + 引数 → 最終文字列 | ユーザープロジェクトのコンパイル時 |
| `Diagnostic.message`         | レンダリング済み文字列           | 最終結果を格納                     |
| AOT バイナリ                 | 最終文字列を含む                 | 実行時に直接使用                   |

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
| `--lang <code>` | 言語を指定 (en-US, zh-CN。デフォルト en-US) |
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

本 RFC はエラーコードシステムをゼロから設計するため、後方互換性の問題は存在しない。

**将来の移行戦略**（後続バージョン参考用）：

1. 旧エラーコードから新エラーコードへのマッピングを保持
2. 移行期間中は新旧コードを同時表示
3. 廃止スケジュールの提供

---

## 実施戦略

### フェーズ 1：エラーコードインフラ

1. `src/diagnostics/` ディレクトリ構造の作成
2. `ErrorCode` 列挙の実装
3. `Diagnostic` と `DiagnosticLevel` の実装
4. リソースファイルディレクトリとサンプル JSON の作成

### フェーズ 2：explain コマンド

1. `yaoxiang explain` CLI コマンドの実装
2. `--lang` と `--json` オプションのサポート
3. リソースファイル読み込みの統合
4. 引数テンプレートレンダリングの実装

### フェーズ 3：コンパイル時統合

1. すべてのエラー報告ポイントを新システムに更新
2. メッセージテンプレート引数注入の実装
3. 言語優先順位ロジックの追加
4. ユニットテストのカバレッジ

### フェーズ 4：IDE/LSP 統合

1. LSP サーバーへの explain JSON 出力統合
2. IDE でのエラーコードリンク表示
3. ホバー時のエラー説明表示
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
| E4xxx | ジェネリクスとトレイト |
| E5xxx | モジュールとインポート |
| E6xxx | 実行時エラー           |
| E7xxx | I/O とシステムエラー   |
| E8xxx | 内部コンパイラエラー   |
| E9xxx | 予約                   |

### サポートされている言語

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

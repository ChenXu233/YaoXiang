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

本 RFC は YaoXiang コンパイラのエラーコード分類仕様を提案する。Rust ライクな単層番号システムを採用し、JSON リソースファイルと組み合わせて多言語サポートを実現し、`yaoxiang explain`
コマンドによりエラー解説機能を提供する。

## 動機

### なぜ標準化されたエラーコードが必要なのか？

1. **ユーザ体験**：ユーザはエラーコードを見ることで、エラーの種類と重大度を迅速に判断できる
2. **ドキュメント構成**：カテゴリ別にグループ化されているため、エラー参考ドキュメントの作成と保守が容易
3. **ツール統合**：IDE/LSP はエラーコードに基づいてクイック修正提案やドキュメントリンクを提供できる
4. **国際化対応**：エラーメッセージとコードが分離されており、多言語翻訳が容易

### 設計目標

- **簡潔**：単層番号方式で、ユーザは複雑な分類ルールを覚える必要がない
- **親しみやすい**：Rust ライクなエラーメッセージ形式、ヘルプ情報とサンプル付き
- **拡張性**：リソースファイル駆動で、新しいエラーと言語の追加が容易
- **ツールフレンドリ**：explain コマンド + JSON 出力で、IDE/LSP 統合をサポート

---

## 提案

### コア設計：単層番号システム

四位数字の番号方式を採用し、コンパイル段階でグループ化する：

```
Exxxx
││││
│││└── 番号 (000-999)
││└─── コンパイル段階 (0-9)
└───── 固定プレフィックス 'E'
```

### 段階の分類

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

### エラーコード定義と汎用 Builder

**中核原則**：エラーコード定義と表示テキストを分離する

- `ErrorCodeDefinition`：エラーコードのメタデータ（code、category、template）を含み、表示テキストは含まない
- `locales/*.json`：各言語の表示テキスト（title、message、help、エラーコードはネストされたオブジェクト）
- `DiagnosticBuilder`：汎用ビルダー、trait-per-error 設計を置き換える

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

| 特性                         | 説明                                                        |
| ---------------------------- | ----------------------------------------------------------- |
| **単一 Builder**             | 1つの `DiagnosticBuilder` ですべてのエラーコードに対応      |
| **型安全**                   | ショートカットメソッドがパラメータの正確性を保証            |
| **自己文書化**               | `E1001::unknown_variable(name)` で一目瞭然                  |
| **テンプレート分離**         | メッセージテンプレートとコードが分離されており、i18n が容易 |
| **実行時オーバーヘッドゼロ** | コンパイル時レンダリング、AOT バイナリにテーブル参照なし    |

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

#### E0xxx：字句解析と構文解析

<!-- code-table:E0xxx start -->

| コード | 説明                       |
| ------ | -------------------------- |
| E0001  | 無効な文字                 |
| E0002  | 無効な数値リテラル         |
| E0003  | 終端されていない文字列     |
| E0004  | 無効な文字リテラル         |
| E0010  | 期待されたトークン         |
| E0011  | 予期しないトークン         |
| E0012  | 無効な構文                 |
| E0013  | 一致しない括弧             |
| E0014  | セミコロン欠落             |
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
| E1020  | 型を推論できない                                 |
| E1021  | 型推論の衝突                                     |
| E1030  | パターンが不完全                                 |
| E1031  | 到達不能パターン                                 |
| E1040  | 操作はサポートされていません                     |
| E1041  | インデックス範囲外                               |
| E1042  | フィールドが見つからない                         |
| E1050  | ブール値が必要                                   |
| E1051  | 論理 NOT にはブール値が必要                      |
| E1052  | 無効な逆参照                                     |
| E1053  | 非構造体のフィールドアクセス                     |
| E1054  | 条件型の不一致                                   |
| E1055  | 非ジェネリクス文脈での制約                       |
| E1060  | 型引数の数が一致しない                           |
| E1061  | ジェネリクスをインスタンス化できない             |
| E1062  | const ジェネリクス制約違反                       |
| E1064  | バインディング位置のインデックスが無効           |
| E1071  | 型定義はモジュールレベルでのみ可能               |
| E1081  | `?` は Result を返す関数内でのみ使用可能         |
| E1082  | `?` は Result 式にのみ使用可能                   |
| E1083  | `?` のエラー型が一致しない                       |
| E1090  | ✨ 不可言説 ✨                                   |
| E1091  | 無効なジェネリクスメタ型                         |
| E1092  | 精化型の引数の形式が不正                         |
| E1093  | 精化引数の数が一致しない                         |
| E1094  | 未使用のコンパイル時値引数                       |
| E1095  | 未知のインターフェース                           |
| E1096  | インターフェース引数の数が一致しない             |
| E1097  | インターフェースメンバーの名前衝突               |
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
| E2010  | 不変変数への代入                         |
| E2011  | 未初期化変数の使用                       |
| E2012  | 可変性の衝突                             |
| E2013  | 変数のシャドーイング                     |
| E2014  | 移動済み値の使用                         |
| E2016  | 不変変数への代入                         |
| E2018  | 可変/不変の借用衝突                      |
| E2019  | 二重解放                                 |
| E2020  | 解放後の使用                             |
| E2027  | unsafe な逆参照                          |
| E2029  | spawn 内の参照サイクル                   |
| E2090  | 無効なシグネチャ                         |
| E2091  | シグネチャ内の未知の型                   |
| E2092  | シグネチャに矢印がない                   |
| E2093  | 引数名の重複                             |
| E2094  | ジェネリクス引数のシャドーイング         |
| E2095  | 引数名によるジェネリクスのシャドーイング |

<!-- code-table:E2xxx end -->

#### E3xxx：コード生成

<!-- code-table:E3xxx start -->

| コード | 説明                                                       |
| ------ | ---------------------------------------------------------- |
| E3004  | サポートされていないイテレータ                             |
| E3005  | IR 生成エラー                                              |
| E3006  | 未解決の変数                                               |
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
| E4004  | トレイト実装の衝突     |
| E4005  | 関連型が見つからない   |
| E4010  | 定数のゼロ除算         |
| E4011  | 定数オーバーフロー     |
| E4012  | 定数の再帰が深すぎる   |
| E4014  | 定数評価失敗           |
| E4018  | 精化述語違反           |
| E4019  | 型の等式が成立しない   |
| E4020  | 証明関数が必要         |

<!-- code-table:E4xxx end -->

> E4006/E8004 は現在発火点がありません（予約コード）：Sized 制約と最適化エラーパスは実装待ちで、実装時に実際の発火面に従って配線します。

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
| E6003  | 配列インデックス範囲外       |
| E6004  | スタックオーバーフロー       |
| E6005  | アサーション失敗             |
| E6006  | 関数が見つからない（実行時） |
| E6007  | 実行時エラー                 |
| E6008  | キーが存在しない             |
| E6009  | Range のステップが不正       |
| E6010  | 整数解析失敗                 |
| E6011  | 浮動小数点解析失敗           |

<!-- code-table:E6xxx end -->

> **#280 改訂（2026-08-09）**：コード表は元々 Rust 意味論ドラフト（Assertion failed/Arithmetic
> overflow/Heap allocation failed/Type cast
> failed）に基づいて定義されていましたが、実装の実際の要件と一致しませんでした。YaoXiang には null ポインタ/ヒープ割り当て失敗/型キャストの概念がなく（値セマンティクス +
> Rust メモリ安全性）、実行時オーバーフロー経路は検出を実装していません。校正後：
>
> - E6002 削除（旧 Assertion failed は E6005 に移動、旧 null ポインタ意味論は言語概念なし）
> - E6003 を Arithmetic overflow から Runtime index out of bounds に変更（実際の発火面、#279/#271）
> - E6005 を Heap allocation failed から Assertion failed に変更（std.assert の実際のパス）
> - E6006 を Runtime index out of bounds から Function not
>   found に変更（実装はすでにこうなっている、#255）
> - E6007 を Type cast failed から汎用 Runtime
>   error に変更（ExecutorError の未マップバリアントの統一落し先）

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

| コード | 説明                     |
| ------ | ------------------------ |
| E8001  | 内部コンパイラエラー     |
| E8002  | 予期しないパニック       |
| E8003  | コンパイラフェーズエラー |

<!-- code-table:E8xxx end -->

#### W1xxx：警告コード

<!-- code-table:W1xxx start -->

| コード | 説明                                 |
| ------ | ------------------------------------ |
| W1001  | 未使用のエクスポート関数             |
| W1002  | 未使用のエクスポート型               |
| W1003  | 未使用のインポート                   |
| W1004  | 未使用のエクスポート変数             |
| W1005  | 未使用のエクスポートメソッド         |
| W1063  | const ジェネリクス制約を評価できない |
| W1080  | コンパイル時証明の降格               |

<!-- code-table:W1xxx end -->

> W コード位置ルール：E コードと同型で段階別にグループ化（W+段階千位セグメント）、W1xxx
> = 型検査段階の警告。
>
> **発火チャネル（#321 M2）**：W コード診断はビルダーにより W プレフィックスに基づきデフォルトで
> `Severity::Warning`
> とラベル付けされ（明示的指定が優先）、収集と表示はエラーと同軌道（`warning[W####]`
> プレフィックスレンダリング）ですが、コンパイルをブロックせず、成功終了コードにも影響しません。`yaoxiang check --deny-warnings`
> は警告を失敗に昇格させ（警告が存在する場合に非ゼロコードで終了）、CI 厳格モード用です。コード単位の抑制（allow 属性など）は今後の拡張項目です。

### メッセージ品質規範

> 本節は #322（M3 メッセージ単線化と品質、2026-09-03）により導入。`scripts/audit_diagnostics.py`
> により CI で強制実行。

1. **メッセージ単線化**：すべてのユーザ可視診断メッセージは正規登録表のショートカットメソッド +
   locales テンプレートレンダリングを経由しなければならず、コードは構造化パラメータのみを渡す。登録表をバイパスして
   `Diagnostic::error(...)`
   などの生値を直接構築する経路は禁止——この経路はコード検証と i18n をバイパスする。
2. **コード合法性**：未登録コードと疑似コード（例：`E_INTERNAL`）の使用は禁止。使用時点のリテラルコードは登録表で既に定義されている必要がある。内部エラーはすべて E8001（`internal_error`）に落す。
3. **型表示**：型 Display はインスタンス化前後の形式を区別しなければならない（#286：`Expected 'Container', found 'Container'`
   のような裸名では区別できない）。
4. **ソルバー内部状態の隔離**：ソルバー中間状態の TypeVar（Display 形式
   `t<N>`）はユーザ可視メッセージに含まれてはならない（#287）。テストアンカー：`test_type_error_message_no_solver_typevar_leak`。
5. **E8xxx の境界**：E8xxx はコンパイラの内部整合性问题（ICE）にのみ使用する。ユーザが修正可能なエラーに E8001 を兜底として使うことは禁止。ICE メッセージには最小限の再現ガイダンスを添付しなければならない。

---

### 実行時エラー値とコードの貫通

> 本節は #323（M4 実行時 Error 値にコードを付与、2026-09-03）により導入。E6xxx/E7xxx 意味空間は 2 つのチャネルを同時に担い、コード空間は同一で、提示チャネルが異なる。

#### 2 つのチャネル

| チャネル                     | 担体                                                  | 提示方式                                                          |
| ---------------------------- | ----------------------------------------------------- | ----------------------------------------------------------------- |
| コンパイラ/CLI 診断チャネル  | `ExecutorError` などのホスト層ハードエラー            | stderr `error[E####]:`（#280/#281 で E6003/E6005/E6007 配線済み） |
| プログラム内エラー値チャネル | std ライブラリ `Result(T, Error)` の Err 担体 `Error` | 言語値、プログラムが match/比較して消費                           |

#### Error 構造（v0.8 以降、破壊的変更）

```
Error { code: String, message: String }
```

- `code` は本仕様の E6xxx/E7xxx 番号を再利用し、文字列形式（例：`"E6008"`）。
- **安定契約**：割り当て済みのコードはバージョン間で意味が変更されない；同じ意味で削除済みコードを再利用しない（E6002 が先例）。
- **消費側**：プログラム内の `e.code == "E6xxx"`
  比較が唯一のプログラミング可能な判定契約；`yaoxiang explain E6xxx`
  のドキュメント貫通；ツールチェーン（LSP / DAP、RFC-034 参照）はコードを exceptionId として扱う。
- **アクセサ**：`std.result.code(e)` / `std.result.message(e)`。
- **ユーザカスタムエラー**：`Result(T, E)`
  の E はジェネリクス引数であり、真面目にモデル化する場合はユーザカスタム型で定義する；std `Error`
  は単なる便利な兜底担体であり、そのコード体系はユーザ E 型を制約しない。

#### コード割り当てルール

1. 実行時エラー値コードとコンパイラ診断コードは E6xxx/E7xxx 空間を共有し、新コードは**実際の発火面**に基づいて割り当て、想像上のシナリオのために予約しない。
2. 先に登録してから使用：新コードは正規登録表に登録し、三方一貫性検証（codes/*.rs ↔ locales
   ↔ 本ドキュメントのコード表）を経た後にのみ発火可能。実行時エラー値コードの登録ソースは
   `src/std/result.rs` の `RUNTIME_ERROR_CODES` 表（診断コードと同様に `build.rs` 構築時の閾値 +
   `tools/code-tables` 検証の対象）。
3. E7xxx は std.io /
   std.net エラー値用の予約セグメント（現在は空、未割り当て、io/net の Result 化時に有効化）。
4. 発火点（#323 M4）：std の各モジュールは `error_new(code, message)` で Error 値を構築；消費側は
   `std.result.unwrap_err` で Err 担体を取り出し、`std.result.code/message` でフィールドを読み取る。

#### 進化パス（線 C、未実施）

パターンマッチング完備化（RFC-039）が実現した後、`Error` は `{ kind: ErrorKind, message: String }`
にアップグレード可能で、`code`
は kind から派生する属性になる（バリアント定義箇所がコード登録表）。進化期間中、本節のコード安定契約は変更されない；このアップグレードは独立した決定であり、本節のコミットメントを構成しない。

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

##### 任意 key サポート

**params は任意の key をサポートし、定義済みプレースホルダに限定されません**。呼び出し側は任意の
`key` を渡せます：

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

#### ユーザレベル設定

```toml
# ~/.yaoxiang/yaoxiang.toml
[language]
default = "zh"
```

#### コンパイル時言語選択

```
1. プロジェクトレベル yaoxiang.toml の language.default を読む
2. 未設定の場合、ユーザレベル ~/.yaoxiang/yaoxiang.toml を読む
3. どちらも未設定の場合、デフォルトで "en" を使用
4. コンパイラは選択された言語に基づいて I18nRegistry を作成（1 回）
5. すべてのエラーはこの I18nRegistry を使用してメッセージをレンダリング
```

#### テーブル参照ゼロオーバーヘッドの鍵

**レンダリングはユーザプロジェクトのコンパイル時に発生し、実行時ではありません。**

```
┌─────────────────────────────────────────────────────────────────────────┐
│  フェーズ 1: Rust が YaoXiang コンパイラをコンパイル                      │
│                                                                           │
│  JSON がコンパイラバイナリにパックされる                                   │
│  目的：explain コマンドが i18n データを直接読めるようにする                │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  フェーズ 2: YaoXiang がユーザプロジェクトをコンパイル（レンダリングはここ）│
│                                                                           │
│  error! マクロ呼び出し時：                                                 │
│  1. yaoxiang.toml を読んで言語設定を取得                                 │
│  2. コンパイラバイナリから対応言語の i18n JSON をロード                    │
│  3. テンプレート + パラメータ → render() → "Unknown variable: 'x'"       │
│  4. Diagnostic.message = レンダリング済み文字列                          │
│                                                                           │
│  AOT バイナリは最終文字列を直接格納、テンプレートなし、テーブル参照なし   │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  フェーズ 3: ユーザプログラムの実行時                                      │
│                                                                           │
│  println!("{}", diagnostic.message)                                      │
│  // 最終文字列を直接出力、テーブル参照なし                                │
└─────────────────────────────────────────────────────────────────────────┘
```

| コンポーネント               | 責務                                   | レンダリングタイミング           |
| ---------------------------- | -------------------------------------- | -------------------------------- |
| `I18nRegistry`               | テンプレートと表示テキストを提供       | ユーザプロジェクトのコンパイル時 |
| `DiagnosticBuilder.render()` | テンプレート + パラメータ → 最終文字列 | ユーザプロジェクトのコンパイル時 |
| `Diagnostic.message`         | レンダリング済み文字列                 | 最終結果を格納                   |
| AOT バイナリ                 | 最終文字列を含む                       | 実行時に直接使用                 |

---

### エラーメッセージ形式

エラーメッセージは以下の形式を採用する：

```
error[E####]: <短い説明>
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

エラーの重大度は `DiagnosticLevel` enum で管理され、エラーコード番号と疎結合：

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
| `--json`        | JSON 形式で出力（IDE/LSP 用）               |
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

本 RFC はゼロからエラーコードシステムを設計するため、後方互換性の問題はありません。

**将来の移行戦略**（後続バージョンの参考用）：

1. 旧エラーコードから新エラーコードへのマッピングを維持
2. 移行期間中は新旧コードを同時に表示
3. 廃止タイムテーブルを提供

---

## 実施戦略

### フェーズ 1：エラーコード基盤

1. `src/diagnostics/` ディレクトリ構造の作成
2. `ErrorCode` enum の実装
3. `Diagnostic` と `DiagnosticLevel` の実装
4. リソースファイルディレクトリとサンプル JSON の作成

### フェーズ 2：explain コマンド

1. `yaoxiang explain` CLI コマンドの実装
2. `--lang` と `--json` オプションのサポート
3. リソースファイル読み込みの統合
4. パラメータテンプレートレンダリングの実装

### フェーズ 3：コンパイル時統合

1. すべてのエラー報告ポイントを新システムに更新
2. メッセージテンプレートパラメータ注入の実装
3. 言語優先度ロジックの追加
4. ユニットテストカバレッジ

### フェーズ 4：IDE/LSP 統合

1. LSP サーバーが explain JSON 出力を統合
2. IDE でエラーコードリンクを表示
3. ホバーでエラー解説を表示
4. クイック修正提案

---

## 付録

### 完全なエラーコード早見表

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

### サポートされる言語

| コード | 言語         | 状態       |
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

# 日本語 (ja-JP)
error[E1001]: 未知の変数: x
  --> src/main.yx:5:12
   5 |   print(x)
          ^
          ヘルプ: 定義したいということでしょうか？
```

## 参考文献

- [Rust コンパイラエラー索引](https://doc.rust-lang.org/error_codes/error-index.html)
- [GCC エラーメッセージ形式](https://gcc.gnu.org/onlinedocs/gcc-13.1.0/gcc/Warning-Options.html)
- [Clang 診断形式](https://clang.llvm.org/diagnostics.html)
- [Language Server Protocol](https://microsoft.github.io/language-server-protocol/)

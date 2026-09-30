---
title: 'RFC 013: エラーコード仕様'
status: '受け入れ済み'
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

本 RFC は YaoXiang コンパイラのエラーコード分類仕様を提案する。Rust ライクな単層番号システムを採用し、JSON リソースファイルと組み合わせることで多言語サポートを実現し、
`yaoxiang explain` コマンドによりエラー説明機能を提供する。

## 動機

### なぜ標準化されたエラーコードが必要か？

1. **ユーザー体験**：エラーコードを見ることで、ユーザーはエラーの種類と重大度を迅速に判断できる
2. **ドキュメント構成**：カテゴリごとに分類することでエラー参考ドキュメントの作成と保守が容易になる
3. **ツール統合**：IDE/LSP はエラーコードに基づいてクイック修正提案とドキュメントリンクを提供できる
4. **国際化サポート**：エラーメッセージとコードを分離することで多言語翻訳が容易になる

### 設計目標

- **簡潔**：単層番号により、ユーザーが複雑な分類ルールを記憶する必要がない
- **親しみやすい**：Rust ライクなエラーメッセージ形式、ヘルプ情報と例付き
- **拡張可能**：リソースファイル駆動で新しいエラーや新しい言語の追加が容易
- **ツールフレンドリー**：explain コマンド + JSON 出力で IDE/LSP 統合をサポート

---

## 提案

### 中核設計：単層番号システム

4 桁の数字番号を採用し、コンパイル段階でグループ化する：

```
Exxxx
││││
│││└── 序号 (000-999)
││└─── 编译阶段 (0-9)
└───── 固定前缀 'E'
```

### 段階区分

| 段階  | 範囲  | 説明                   |
| ----- | ----- | ---------------------- |
| **0** | E0xxx | 字句・構文解析         |
| **1** | E1xxx | 型検査                 |
| **2** | E2xxx | セマンティック解析     |
| **3** | E3xxx | コード生成             |
| **4** | E4xxx | generics と trait      |
| **5** | E5xxx | モジュールとインポート |
| **6** | E6xxx | runtime エラー         |
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

- `ErrorCodeDefinition`：エラーコードメタデータ（code、category、template）、表示テキストを含まない
- `locales/*.json`：各言語の表示テキスト（title、message、help、エラーコードはネストオブジェクト）
- `DiagnosticBuilder`：汎用ビルダー、trait-per-error 設計の代替

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

#### 設計上の優位性

| 特性                             | 説明                                                      |
| -------------------------------- | --------------------------------------------------------- |
| **単一 Builder**                 | 1 つの `DiagnosticBuilder` ですべてのエラーコードに汎用   |
| **型安全**                       | ショートカットメソッドがパラメータの正確性を保証          |
| **自己文書化**                   | `E1001::unknown_variable(name)` で一目瞭然                |
| **テンプレート分離**             | メッセージテンプレートとコードの分離、容易な i18n         |
| **ランタイムオーバーヘッドゼロ** | compile-time レンダリング、AOT バイナリにテーブル参照なし |

---

### エラーメクロ簡素化

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

#### E0xxx：字句・構文解析

<!-- code-table:E0xxx start -->

| コード | 説明                                 |
| ------ | ------------------------------------ |
| E0001  | 無効な文字                           |
| E0002  | 無効な数値リテラル                   |
| E0003  | 終端されていない文字列               |
| E0004  | 無効な文字リテラル                   |
| E0010  | 期待されたトークン                   |
| E0011  | 予期しないトークン                   |
| E0012  | 無効な構文                           |
| E0013  | 一致しない括弧                       |
| E0014  | セミコロンの欠落                     |
| E0016  | 期待された式                         |
| E0018  | キーワードが名前として使用されている |

<!-- code-table:E0xxx end -->

#### E1xxx：型検査

<!-- code-table:E1xxx start -->

| コード | 説明                                                        |
| ------ | ----------------------------------------------------------- |
| E1001  | 未知の変数                                                  |
| E1002  | 型の不一致                                                  |
| E1003  | 未知の型                                                    |
| E1010  | 引数の数が一致しない                                        |
| E1011  | 引数の型が一致しない                                        |
| E1012  | 戻り値の型が一致しない                                      |
| E1013  | 関数が見つからない                                          |
| E1014  | 名前付き引数名が未知                                        |
| E1015  | 引数の重複指定                                              |
| E1020  | 型を推論できない                                            |
| E1021  | type inference の衝突                                       |
| E1030  | パターンが不完全                                            |
| E1031  | 到達不能パターン                                            |
| E1032  | パターンの重複バインド                                      |
| E1033  | OR パターンのバインド不一致                                 |
| E1034  | 構造体パターンにフィールド欠落                              |
| E1040  | 操作はサポートされていない                                  |
| E1041  | インデックス範囲外                                          |
| E1042  | フィールドが見つからない                                    |
| E1050  | ブールオペランドが必要                                      |
| E1051  | 論理 NOT にブールオペランドが必要                           |
| E1052  | 無効なデリファレンス                                        |
| E1053  | 非構造体フィールドアクセス                                  |
| E1054  | conditional type が一致しない                               |
| E1055  | 制約が非ジェネリックコンテキストにある                      |
| E1060  | 型パラメータの数が一致しない                                |
| E1061  | generics をインスタンス化できない                           |
| E1062  | const generics 制約の失敗                                   |
| E1064  | バインド位置インデックスが無効                              |
| E1065  | 非関数値の呼び出し                                          |
| E1071  | 型定義はモジュールレベルでのみ可能                          |
| E1081  | `?` は error propagation 可能な型を返す関数内でのみ使用可能 |
| E1082  | `?` は Try を実装した型にのみ使用可能                       |
| E1083  | `?` のエラー型が一致しない                                  |
| E1090  | ✨ 語りに難しい ✨                                          |
| E1091  | 無効な generics メタ型                                      |
| E1092  | 精化型引数の形式が不正                                      |
| E1093  | 精化引数の数が一致しない                                    |
| E1094  | 未使用の compile-time 値パラメータ                          |
| E1095  | 未知のインタフェース                                        |
| E1096  | インタフェース引数の数が一致しない                          |
| E1097  | インタフェースメンバーの名前衝突                            |
| E1098  | インタフェースメソッドが未実装                              |
| E1099  | インタフェースメソッドのシグネチャが一致しない              |
| E1100  | インタフェースメソッドの重複実装                            |
| E1101  | 型がインタフェースを実装していない                          |
| E1102  | ループ制御文がループ外に出現                                |
| E1103  | 型位置に角括弧は使用不可                                    |
| E1104  | インタフェース実装が型の定義モジュールにない                |
| E1105  | バリアントコンストラクタはフィールドとしてアクセス不可      |
| E1106  | 制約が満たされていない                                      |
| E1107  | メソッドオーバーロードの曖昧性                              |

<!-- code-table:E1xxx end -->

> **RFC-011b 関連（2026-09-22 注）**：[RFC-011b: 演算子オーバーロード](./011b-operator-overloading.md)
> 落地時に本段落の 3 箇所に影響する——① `E1081` / `E1082` の文言から "Result" の文字を削除（`?` を
> `Try`
> インタフェース判定に変更し、特定の型名に紐づけない）、フェーズ 2 で本ドキュメントの「三方一貫性」フローに従って同期（codes/*.rs
> ↔ locales ↔ コード表）；② `Equal` 前置制約（線形トークン）不満足の拒否診断は
> `E1101`（型がインタフェースを実装していない）族を再利用；③ フェーズ 1 配線後、`Struct == Struct`
> は `E6007` runtime エラーから compile-time 判定に変わり、`E6007`
> の発火面が縮小する。表内の登録文言は実装落地前は現状維持。

#### E2xxx：セマンティック解析

<!-- code-table:E2xxx start -->

| コード | 説明                                |
| ------ | ----------------------------------- |
| E2001  | スコープエラー                      |
| E2002  | 重複定義                            |
| E2003  | ownership エラー                    |
| E2010  | 不変代入                            |
| E2011  | 未初期化変数の使用                  |
| E2012  | 可変性の衝突                        |
| E2013  | 変数のシャドウイング                |
| E2014  | 移動済み値の使用                    |
| E2016  | 不変代入                            |
| E2018  | 可変/不変借用の衝突                 |
| E2019  | 二重解放                            |
| E2020  | 解放後使用                          |
| E2027  | unsafe デリファレンス               |
| E2029  | spawn 内の ref 循環                 |
| E2030  | 精化型制約違反                      |
| E2031  | 精化制約を証明できない              |
| E2090  | 無効なシグネチャ                    |
| E2091  | シグネチャに未知の型                |
| E2092  | シグネチャに矢印がない              |
| E2093  | 引数名の重複                        |
| E2094  | generics パラメータのシャドウイング |
| E2095  | 引数名が generics をシャドウイング  |

<!-- code-table:E2xxx end -->

> 予約コード説明（2026-09-14 棚卸し、#251 リリース方針）：E2019（二重解放）、E2020（解放後使用）、E2027（unsafe デリファレンス）、E2029（spawn 内 ref 循環）は登録が完了しユニットテストでアンカーされているが、まだ到達可能な yx ソースコードの表層がない（明示的 drop 文、Ptr デリファレンス文法、spawn
> ref ループ構築経路）——意味的に完全に正しいという主張はこれら 4 コードをカバーしない、実装補完前は「予約」として扱う。

#### E3xxx：コード生成

<!-- code-table:E3xxx start -->

| コード | 説明                                                 |
| ------ | ---------------------------------------------------- |
| E3004  | サポートされていないイテレータ                       |
| E3005  | IR 生成エラー                                        |
| E3006  | 解決されていない変数                                 |
| E3007  | トップレベルバインドの初期化は定数でなければならない |
| E3008  | サポートされていない match パターン                  |
| E3014  | レジスタオーバーフロー                               |
| E3017  | 無効なオペランド（コード生成）                       |
| E3018  | 単相化インスタンス化の失敗                           |
| E3019  | トップレベルバインドの循環依存                       |
| E3020  | プログラムエントリがない                             |
| E3021  | エントリが関数ではない                               |
| E3022  | エントリ main のシグネチャが一致しない               |
| E3023  | トップレベルで実行文は許可されない                   |

<!-- code-table:E3xxx end -->

#### E4xxx：generics と trait

<!-- code-table:E4xxx start -->

| コード | 説明                           |
| ------ | ------------------------------ |
| E4001  | generics 制約違反              |
| E4002  | trait が見つからない           |
| E4003  | trait 実装欠落                 |
| E4004  | trait 実装の衝突               |
| E4005  | associated type が見つからない |
| E4010  | 定数のゼロ除算                 |
| E4011  | 定数オーバーフロー             |
| E4012  | 定数再帰が深すぎる             |
| E4014  | 定数評価失敗                   |
| E4018  | 精化述語違反                   |
| E4019  | 型の等式が成立しない           |
| E4020  | 証明関数が必要                 |
| E4021  | ループ停止性を自動証明できない |
| E4022  | 測度が成立しない               |

<!-- code-table:E4xxx end -->

> E4006/E8004 は現在発火ポイントなし（予約コード）：Sized 制約と最適化エラーパスは実装待ち、実装時に実際の発火面で配線する。

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

#### E6xxx：runtime エラー

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
| E6009  | Range ステップが不正             |
| E6010  | 整数解析失敗                     |
| E6011  | 浮動小数点解析失敗               |
| E6012  | コードポイントが不正             |
| E6013  | JSON 解析失敗                    |

<!-- code-table:E6xxx end -->

> **コード表改訂（2026-08-09）**：コード表は元々 Rust セマンティクス草案（Assertion
> failed/Arithmetic overflow/Heap allocation failed/Type cast
> failed）に従って定義されており、実装の実際の要件と一致していなかった。YaoXiang には null ポインタ/ヒープ割り当て失敗/型変換の概念がなく（値セマンティクス +
> Rust メモリ安全性）、runtime オーバーフロー経路には検出が実装されていない。較正後：
>
> - E6002 削除（元 Assertion
>   failed は E6005 に移動；元 null ポインタセマンティクスは言語に概念なし）
> - E6003 を Arithmetic overflow から Runtime index out of bounds に変更（実際の発火面）
> - E6005 を Heap allocation failed から Assertion failed に変更（std.assert の実際のパス）
> - E6006 を Runtime index out of bounds から Function not found に変更（実装はすでにこの通り）
> - E6007 を Type cast failed から汎用 Runtime
>   error に変更（ExecutorError の未マップ変体の統一着地点）

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

| コード | 説明                              |
| ------ | --------------------------------- |
| W1001  | 未使用のプライベート関数          |
| W1002  | 未使用のプライベート型            |
| W1003  | 未使用のインポート                |
| W1004  | 未使用のプライベート変数          |
| W1005  | 未使用のプライベートメソッド      |
| W1063  | const generics 制約を評価できない |
| W1080  | compile-time 証明の降格           |

<!-- code-table:W1xxx end -->

> W コード位置ルール：E コードと同じ構造で段階ごとにグループ化（W + 段階の千位）、W1xxx
> = 型検査段階の警告。
>
> **デッドコードの意味（#321 決定案 B）**：`pub`
> 定義は外部インタフェースであり、永久に報告しない——外部消費者が使用するかどうかは単一ファイル解析の境界を超えるため、誤報よりも沈黙を優先する。W1001/W1002/W1004/W1005 は
> **プライベート（pub でない）定義** のみを対象：`main` と `pub`
> 定義から始まる参照到達性分析で一度も参照されていない場合に報告。メソッド（W1005）は呼び出し点の短い名前で一致させる。bin/lib
> target セマンティクス（bin 内で未使用の pub を警告）は将来の拡張（#289 案 A）、プロジェクトモデルのサポートが必要。
>
> **未使用インポート（W1003）**：typecheck の use
> elaboration によって検出される（pass2 でインポートローカル名を登録し、式解析と型注釈位置でヒットしたものは使用済みと見なす）、全体インポート（`use std.io`
> → モジュールエイリアス）と名前付きインポート（`use std.io.{print}`）の両方をカバー。
>
> **発火チャネル**：W コード診断は builder が W 接頭辞でデフォルト `Severity::Warning`
> を付与（明示的指定優先）、収集と表示はエラーと同じトラック（`warning[W####]`
> 接頭辞レンダリング）だが、コンパイルをブロックせず、成功終了コードにも影響しない。`yaoxiang check --deny-warnings`
> は警告を失敗に昇格させ（警告が存在する場合に非ゼロコードで終了）、CI 厳格モードに使用。per-code 抑制（allow 属性など）は後続の拡張項目。

### メッセージ品質仕様

> 本節はメッセージ単一経路と品質改訂（2026-09-03）によって導入された。`scripts/audit_diagnostics.py`
> によって CI で強制実行される。

1. **メッセージ単一経路**：すべてのユーザー可視診断メッセージは権威ある登録表のショートカットメソッド +
   locales テンプレートレンダリングを経由しなければならず、コードは構造化パラメータのみを渡す。登録表をバイパスして
   `Diagnostic::error(...)`
   などのネイティブ値を直接構築することは禁止——この経路はコード検証と i18n をバイパスする。
2. **コード合法性**：未登録コードとフェイクコード（例：`E_INTERNAL`）の使用は禁止；使用箇所のコードリテラルは登録表で定義済みでなければならない。内部エラーは一律 E8001（`internal_error`）にフォールバック。
3. **型表示**：型の Display はインスタンス化の前後形態を区別しなければならない（`Expected 'Container', found 'Container'`
   裸名では区別不可）。
4. **ソルバー内部状態隔離**：ソルバー中間状態の TypeVar（Display 形態
   `t<N>`）はユーザー可視メッセージに入ってはいけない。テストアンカー：`test_type_error_message_no_solver_typevar_leak`。
5. **E8xxx 境界**：E8xxx はコンパイラ内部の一貫性問題（ICE）にのみ使用。ユーザーが修正可能なエラーは E8001 をフォールバックとして使用することを禁止；ICE メッセージには最小限の再現ガイダンスを添付しなければならない。

---

### runtime エラー値とコードの貫通

> 本節は runtime
> Error 値コード付き改訂（2026-09-03）によって導入された。E6xxx/E7xxx セマンティクス空間は 2 つのチャネルを同時に担い、コード空間は同一、提示チャネルは異なる。

#### 2 つのチャネル

| チャネル                     | キャリア                                                  | 提示方式                                             |
| ---------------------------- | --------------------------------------------------------- | ---------------------------------------------------- |
| コンパイラ/CLI 診断チャネル  | `ExecutorError` などのホスト層ハードエラー                | stderr `error[E####]:`（E6003/E6005/E6007 配線済み） |
| プログラム内エラー値チャネル | std ライブラリ `Result(T, Error)` の Err キャリア `Error` | 言語値、プログラムの match/比較で消費                |

#### Error 構造（v0.8 破壊的変更）

```
Error { code: String, message: String }
```

- `code` は本仕様の E6xxx/E7xxx 番号を再利用、文字列形態（例：`"E6008"`）。
- **安定契約**：割り当て済みコードはバージョン間でセマンティクスが不変；同一セマンティクスで削除済みコードは再利用しない（E6002 が前例）。
- **消費面**：プログラム内 `e.code == "E6xxx"`
  比較が唯一のプログラマブル判定契約；`yaoxiang explain E6xxx` ドキュメント貫通；ツールチェーン（LSP
  / DAP、RFC-034 参照）はコードを exceptionId とする。
- **アクセサ**：`std.result.code(e)` / `std.result.message(e)`。
- **ユーザー定義エラー**：`Result(T, E)`
  の E は generics パラメータ、真剣にモデリングする場合はユーザー定義型を使用；std `Error`
  は単なる便利なフォールバックキャリアであり、そのコード体系はユーザー E 型を制約しない。

#### コード割り当てルール

1. runtime エラー値コードとコンパイラ診断コードは E6xxx/E7xxx 空間を共有し、新コードは
   **実際の発火面** に従って割り当て、想像上のシナリオのために予約しない。
2. 先に登録してから使用：新コードは権威ある登録表に入り、三方一貫性検証（codes/*.rs ↔ locales
   ↔ 本ドキュメントコード表）を経た後に発射可能。runtime エラー値コードの登録ソースは
   `src/std/result.rs` の `RUNTIME_ERROR_CODES` 表（診断コードと同じく `build.rs` 構築期閾値 +
   `tools/code-tables` 検証を受ける）。
3. E7xxx は std.io / std.net エラー値のためにセグメントを予約（現在は空、io/net
   Result 化時に有効化）。
4. 発火点：std 各モジュールは `error_new(code, message)` を介して Error 値を構築；消費側は
   `std.result.unwrap_err` で Err キャリアを取り出し、`std.result.code/message`
   でフィールドを読み取る。

#### 進化パス（ライン C、未実施）

パターンマッチ完全化（RFC-010b）落地後、`Error` は `{ kind: ErrorKind, message: String }`
にアップグレード可能、`code`
は kind から派生する属性に変換（バリアント定義箇所がコード登録表）。進化期間中、本節の code 安定契約は不変を保つ；このアップグレードは独立した決定であり、本節のコミットメントを構成しない。

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

##### 定義済みプレースホルダ（よく使用）

| プレースホルダ | 用途                             | 例                                  |
| -------------- | -------------------------------- | ----------------------------------- |
| `{name}`       | 変数名/型名/trait 名などの識別子 | `Unknown variable: '{name}'`        |
| `{expected}`   | 期待される型                     | `Expected type '{expected}'`        |
| `{found}`      | 実際/見つかった型                | `, found type '{found}'`            |
| `{method}`     | メソッド名                       | `Method {method} is not a function` |
| `{trait}`      | trait 名                         | `Cannot find trait: {trait}`        |
| `{path}`       | モジュールパス                   | `Invalid path: {path}`              |
| `{ty}`         | 型式                             | `Invalid type: {ty}`                |
| `{message}`    | 内部エラーメッセージ             | `Internal error: {message}`         |

##### 任意の key サポート

**params は任意の key をサポート、定義済みに限定されない**。呼び出し側は任意の `key`
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

> **注意**：すべてのエラーコードがプレースホルダを使用するわけではない。一部のエラーコード（例：E0001）は静的メッセージで、パラメータ不要。

#### 言語優先度

```
1. yaoxiang.toml [language.default]
2. ~/.yaoxiang/yaoxiang.toml [language.default]
3. 默认值: en
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

#### compile-time 言語選択

```
1. 读取项目级 yaoxiang.toml 的 language.default
2. 若未配置，读取用户级 ~/.yaoxiang/yaoxiang.toml
3. 若都未配置，默认使用 "en"
4. 编译器根据选择的语言创建 I18nRegistry（一次）
5. 所有错误使用该 I18nRegistry 渲染消息
```

#### テーブル参照ゼロオーバーヘッドの鍵

**レンダリングはユーザープロジェクトのコンパイル時に発生し、runtime ではない。**

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

| コンポーネント               | 責務                                   | レンダリングタイミング               |
| ---------------------------- | -------------------------------------- | ------------------------------------ |
| `I18nRegistry`               | テンプレートと表示テキストを提供       | ユーザープロジェクト compile-time 時 |
| `DiagnosticBuilder.render()` | テンプレート + パラメータ → 最終文字列 | ユーザープロジェクト compile-time 時 |
| `Diagnostic.message`         | レンダリング済み文字列                 | 最終結果を保存                       |
| AOT バイナリ                 | 最終文字列を含む                       | runtime 直接使用                     |

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

エラーの重大度は `DiagnosticLevel` 列挙で管理され、エラーコード番号と疎結合：

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
| Warning | `warning[E####]:` | コンパイルに影響しない     |
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
| `--lang <code>` | 言語指定 (en-US、zh-CN、デフォルト en-US) |
| `--json`        | JSON 形式出力（IDE/LSP 用）               |
| `--json-pretty` | フォーマット済み JSON 出力                |
| `--examples`    | サンプルコードのみ表示                    |
| `--help`        | ヘルプ情報を表示                          |

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

本 RFC はエラーコードシステムをゼロから設計するため、後方互換性の問題はない。

**将来の移行戦略**（後続バージョン参考用）：

1. 旧エラーコードから新エラーコードへのマッピングを保持
2. 移行期間中は新旧コードを同時表示
3. 廃止スケジュールの提供

---

## 実装戦略

### フェーズ 1：エラーコード基盤アーキテクチャ

1. `src/diagnostics/` ディレクトリ構造の作成
2. `ErrorCode` 列挙の実装
3. `Diagnostic` と `DiagnosticLevel` の実装
4. リソースファイルディレクトリとサンプル JSON の作成

### フェーズ 2：explain コマンド

1. `yaoxiang explain` CLI コマンドの実装
2. `--lang` と `--json` オプションのサポート
3. リソースファイル読み込みの統合
4. パラメータテンプレートレンダリングの実装

### フェーズ 3：compile-time 統合

1. すべてのエラー報告点を新システム使用に更新
2. メッセージテンプレートパラメータ注入の実装
3. 言語優先度ロジックの追加
4. ユニットテストカバレッジ

### フェーズ 4：IDE/LSP 統合

1. LSP サーバーが explain JSON 出力を統合
2. IDE にエラーコードリンクを表示
3. ホバーでエラー説明を表示
4. クイック修正提案

---

## 付録

### 完全エラーコード早見表

| 範囲  | カテゴリ               |
| ----- | ---------------------- |
| E0xxx | 字句・構文解析         |
| E1xxx | 型検査                 |
| E2xxx | セマンティック解析     |
| E3xxx | コード生成             |
| E4xxx | generics と trait      |
| E5xxx | モジュールとインポート |
| E6xxx | runtime エラー         |
| E7xxx | I/O とシステムエラー   |
| E8xxx | 内部コンパイラエラー   |
| E9xxx | 予約                   |

### サポート言語

| コード | 言語         | ステータス |
| ------ | ------------ | ---------- |
| en-US  | English (US) | デフォルト |
| zh-CN  | 簡体字中国語 | 計画中     |

### エラーメッセージ例比較

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

- [Rust コンパイラエラーインデックス](https://doc.rust-lang.org/error_codes/error-index.html)
- [GCC エラーメッセージ形式](https://gcc.gnu.org/onlinedocs/gcc-13.1.0/gcc/Warning-Options.html)
- [Clang 診断形式](https://clang.llvm.org/diagnostics.html)
- [Language Server Protocol](https://microsoft.github.io/language-server-protocol/)

---
title: 診断システム
description: YaoXiang 診断システムの設計
---

# 診断システム

## エラーコード体系

エラーコードはカテゴリ別にグループ化されています：

| 範囲  | カテゴリ     | 説明                         |
| ----- | ----------- | ---------------------------- |
| E0xxx | 字句/構文解析 | 字句解析および構文解析エラー     |
| E1xxx | 型検査       | 型の不一致、未定義変数など       |
| E2xxx | 意味解析     | 意味解析エラー                   |
| E4xxx | ジェネリクス/トレイト | ジェネリクスとトレイトシステムエラー |
| E5xxx | モジュール/インポート | モジュールシステムエラー         |
| E6xxx | ランタイム    | ランタイムエラー                 |
| E7xxx | I/O          | I/O とシステムエラー            |
| E8xxx | 内部         | 内部コンパイルエラー             |
| W1xxx | 警告         | デッドコード、未使用変数など      |

## Diagnostic データ構造

```rust
pub struct Diagnostic {
    pub code: String,           // エラーコード 例: "E1001"
    pub severity: Severity,     // Error / Warning / Info / Hint
    pub message: String,        // レンダリング後のメッセージ
    pub span: Option<Span>,     // ソースコード位置
    pub help: Option<String>,   // 修正提案
    pub related: Vec<Box<Diagnostic>>,  // 関連診断
}
```

## DiagnosticBuilder パターン

`ErrorCodeDefinition` からビルダーを取得し、チェーンメソッドでパラメータを設定します：

```rust
let diagnostic = ErrorCodeDefinition::unknown_variable("x")
    .at(span)
    .help("did you mean 'y'?")
    .build();
```

## i18n サポート

すべてのエラーコードのタイトルとヘル_plate は `I18nRegistry` で管理され、中英語切り替えに対応しています。メッセージテンプレートは `{param}` プレースホルダをサポートします。

## Emitter 出力

- `TextEmitter`：テキスト形式出力、色と Unicode 記号をサポート
- `JsonEmitter`：JSON 形式出力、CI と LSP 用

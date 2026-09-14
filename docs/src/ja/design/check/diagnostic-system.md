---
title: '診断システム'
description: 'YaoXiang 診断システムのアーキテクチャ設計'
---

# 診断システム

## エラーコード体系

エラーコードはカテゴリ別にグループ化されています：

| 範囲  | カテゴリ              | 説明                                   |
| ----- | --------------------- | -------------------------------------- |
| E0xxx | 字句/構文             | 字句解析および構文解析エラー           |
| E1xxx | 型検査                | 型の不一致、未定義の変数など           |
| E2xxx | セマンティック解析    | セマンティックエラー                   |
| E4xxx | ジェネリクス/トレイト | ジェネリクスとトレイトシステムのエラー |
| E5xxx | モジュール/インポート | モジュールシステムエラー               |
| E6xxx | ランタイム            | ランタイムエラー                       |
| E7xxx | I/O                   | I/O およびシステムエラー               |
| E8xxx | 内部                  | 内部コンパイラエラー                   |
| W1xxx | 警告                  | デッドコード、未使用の変数など         |

## Diagnostic データ構造

```rust
pub struct Diagnostic {
    pub code: String,           // エラーコード（例: "E1001"）
    pub severity: Severity,     // Error / Warning / Info / Hint
    pub message: String,        // レンダリングされたメッセージ
    pub span: Option<Span>,     // ソースコードの位置
    pub help: Option<String>,   // 修正の提案
    pub related: Vec<Box<Diagnostic>>,  // 関連する診断
}
```

## DiagnosticBuilder パターン

`ErrorCodeDefinition` から builder を取得し、メソッドチェーンでパラメータを設定します：

```rust
let diagnostic = ErrorCodeDefinition::unknown_variable("x")
    .at(span)
    .help("did you mean 'y'?")
    .build();
```

## i18n サポート

すべてのエラーコードのタイトルとヘルプテキストは `I18nRegistry`
によって管理され、中国語と英語の切り替えに対応しています。メッセージテンプレートは `{param}`
プレースホルダをサポートします。

## Emitter 出力

- `TextEmitter`：テキスト形式での出力。色や Unicode 記号をサポート
- `JsonEmitter`：JSON 形式での出力。CI や LSP で使用

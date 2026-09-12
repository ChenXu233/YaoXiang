# エラーコードリファレンス

> `src/util/diagnostic/codes/` から自動生成

YaoXiang コンパイラは統一されたエラーコードシステムを使用しています。各エラーコードには以下が含まれます：

- **コード**: エラー識別子（例：`E1001`）
- **カテゴリ**: エラーが属するフェーズ
- **タイトル**: エラーの簡潔な説明
- **メッセージ**: 詳細なエラーメッセージ
- **ヘルプ**: 考えられる解決策

## エラーコード一覧

| 接頭辞 | カテゴリ     | 説明                         |
| ------ | ------------ | ---------------------------- |
| E0xxx  | Lexer/Parser | 字句解析および構文解析エラー |
| E1xxx  | TypeCheck    | 型チェックエラー             |
| E2xxx  | Semantic     | 意味解析エラー               |
| E4xxx  | Generic      | ジェネリクスとトレイトエラー |
| E5xxx  | Module       | モジュールとインポートエラー |
| E6xxx  | Runtime      | ランタイムエラー             |
| E7xxx  | I/O          | I/O およびシステムエラー     |
| E8xxx  | Internal     | 内部コンパイラエラー         |

## 使用方法

### CLI コマンド

`yx explain` コマンドを使用してエラーの詳細を確認します：

```bash
# エラー詳細の確認
yx explain E1001

# JSON 形式での出力
yx explain E1001 --json
```

### コード内で

```rust
use yaoxiang::util::diagnostic::{ErrorCodeDefinition, I18nRegistry};

// エラーコードを検索し、I18nRegistry を通じてタイトルとヘルプ情報を取得
let i18n = I18nRegistry::default();

if let Some(code) = ErrorCodeDefinition::find("E1001") {
    let title = i18n.get_title(&code);
    println!("Title: {}", title);

    if let Some(help) = i18n.get_help(&code) {
        println!("Help: {}", help);
    }
}
```

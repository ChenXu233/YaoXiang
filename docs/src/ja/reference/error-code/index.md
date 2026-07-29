# エラーコードリファレンス

> `src/util/diagnostic/codes/` から自動生成

YaoXiang コンパイラは統一されたエラーコードシステムを採用しており、各エラーコードには以下の要素が含まれます：

- **コード**: エラー識別子（例：`E1001`）
- **カテゴリ**: エラーの属するフェーズ
- **タイトル**: エラーの簡単な説明
- **メッセージ**: 詳細なエラーメッセージ
- **ヘルプ**: 可能な解決策

## エラーコード一覧

| プレフィックス | カテゴリ     | 説明                         |
| -------------- | ------------ | ---------------------------- |
| E0xxx          | Lexer/Parser | レキサーとパーサーのエラー   |
| E1xxx          | TypeCheck    | 型チェックエラー             |
| E2xxx          | Semantic     | 意味解析エラー               |
| E4xxx          | Generic      | ジェネリクスとtraitエラー    |
| E5xxx          | Module       | モジュールとインポートエラー |
| E6xxx          | Runtime      | ランタイムエラー             |
| E7xxx          | I/O          | I/Oとシステムエラー          |
| E8xxx          | Internal     | コンパイラの内部エラー       |

## 使い方

### CLI コマンド

`yaoxiang explain` コマンドを使用してエラー詳細を確認します：

```bash
# エラー詳細の確認
yaoxiang explain E1001

# JSON 形式での出力
yaoxiang explain E1001 --json
```

### コード内

```rust
use yaoxiang::util::diagnostic::{ErrorCodeDefinition, I18nRegistry};

// エラーコードを検索し、I18nRegistry からタイトルとヘルプ情報を取得
let i18n = I18nRegistry::default();

if let Some(code) = ErrorCodeDefinition::find("E1001") {
    let title = i18n.get_title(&code);
    println!("Title: {}", title);

    if let Some(help) = i18n.get_help(&code) {
        println!("Help: {}", help);
    }
}
```

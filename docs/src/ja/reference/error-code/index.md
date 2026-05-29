# エラーコードリファレンス

> `src/util/diagnostic/codes/` から自動生成

YaoXiang コンパイラは統一されたエラーコードシステムを使用しており、各エラーコードには以下が含まれます：

- **コード**: エラー識別子（例：`E1001`）
- **カテゴリ**: エラーが発生したフェーズ
- **タイトル**: エラーの簡単な説明
- **メッセージ**: 詳細なエラーメッセージ
- **ヘルプ**: 考えられる解決策

## エラーコード一覧

| プレフィックス | カテゴリ | 説明 |
|------|------|------|
| E0xxx | Lexer/Parser | 字句解析および構文解析エラー |
| E1xxx | TypeCheck | 型チェックエラー |
| E2xxx | Semantic | 静的意味解析エラー |
| E4xxx | Generic | ジェネリクスとトレイトエラー |
| E5xxx | Module | モジュールとインポートエラー |
| E6xxx | Runtime | ランタイムエラー |
| E7xxx | I/O | I/Oとシステムエラー |
| E8xxx | Internal | コンパイラの内部エラー |

## 使用方法

### CLI コマンド

`yaoxiang explain` コマンドでエラーの詳細を確認できます：

```bash
# エラーの詳細を確認
yaoxiang explain E1001

# JSON フォーマットで出力
yaoxiang explain E1001 --json
```

### コード内

```rust
use yaoxiang::util::diagnostic::{ErrorCodeDefinition, I18nRegistry};

# エラーコードを查找し、I18nRegistry からタイトルとヘルプ情報を取得
let i18n = I18nRegistry::default();

if let Some(code) = ErrorCodeDefinition::find("E1001") {
    let title = i18n.get_title(&code);
    println!("Title: {}", title);

    if let Some(help) = i18n.get_help(&code) {
        println!("Help: {}", help);
    }
}
```
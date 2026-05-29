# エラーコードリファレンス

> `src/util/diagnostic/codes/` から自動生成

YaoXiang コンパイラは統一されたエラーコードシステムを使用しており、各エラーコードには以下が含まれます：
- **コード**: エラー識別子（例：`E1001`）
- **カテゴリ**: エラーの所属フェーズ
- **タイトル**: エラーの短い説明
- **メッセージ**: 詳細なエラーメッセージ
- **ヘルプ**: 考えられる解決策

## エラーコード一覧

| プレフィックス | カテゴリ | 説明 |
|------|------|------|
| E0xxx | Lexer/Parser | 字句解析と構文解析エラー |
| E1xxx | TypeCheck | 型検査エラー |
| E2xxx | Semantic | 意味解析エラー |
| E4xxx | Generic | 泛型とtraitエラー |
| E5xxx | Module | モジュールとインポートエラー |
| E6xxx | Runtime | 运行时エラー |
| E7xxx | I/O | I/Oとシステムエラー |
| E8xxx | Internal | コンパイラの内部エラー |

## 使用方法

### CLI コマンド

`yaoxiang explain` コマンドを使用してエラー詳細を確認します：

```bash
# エラー詳細の確認
yaoxiang explain E1001

# JSON フォーマット出力
yaoxiang explain E1001 --json
```

### コード内

```rust
use yaoxiang::util::diagnostic::{ErrorCodeDefinition, I18nRegistry};

// エラーコードを検索し、I18nRegistry を使用してタイトルとヘルプ情報を取得
let i18n = I18nRegistry::default();

if let Some(code) = ErrorCodeDefinition::find("E1001") {
    let title = i18n.get_title(&code);
    println!("Title: {}", title);

    if let Some(help) = i18n.get_help(&code) {
        println!("Help: {}", help);
    }
}
```
# YaoXiang 設計ドキュメント

> 道生一，一生二，二生三，三生万物。

本ディレクトリには YaoXiang プログラミング言語の設計上の意思決定、提案、議論が含まれています。

## コア設計理念

| 理念 | 説明 |
|------|------|
| **一切皆タイプ** | 値、関数、モジュールはすべてタイプであり、型は第一級市民である |
| **自然な構文** | Python のような可読性、自然言語に近い |
| **所有権モデル** | ゼロコスト抽象化、GCなし、高パフォーマンス |
| **並作モデル** | 同期構文、非同期本質、自动並列化 |
| **AI フレンドリー** | 厳密な構造化、明確な AST |

## 設計ドキュメントの構造

```
design/
├── index.md              # 本インデックス
├── accepted/             # 採用済み設計提案
│   └── *.md
├── rfc/                  # RFC 提案（审议中）
│   ├── *.md
│   └── RFC_TEMPLATE.md
└── discussion/           # 設計議論エリア（オープン議論）
    └── *.md
```

## 採用済み設計提案

| ドキュメント | 状態 | 説明 |
|------|------|------|
| [008-並行モデル](./accepted/008-runtime-concurrency-model.md) | ✅ 正式 | 並作モデルとタスクスケジューラー設計 |

> 完全なリストは [`accepted/`](./accepted/) ディレクトリを参照してください。

## RFC 提案

> RFC（Request for Comments）は、新機能と大きな変更のための提案プロセスです。

### アクティブな提案

| 番号 | タイトル | 状態 |
|------|------|------|
| RFC-003 | バージョニングポリシー | 待审议 |
| RFC-005 | 自動化 CVE スキャン | 待审议 |
| RFC-006 | ドキュメントサイトの最適化 | 待审议 |
| RFC-012 | f-string テンプレート文字列 | 待审议 |

### RFC テンプレート

新しい提案を送信する前に、以下を参照してください：
- [RFC_TEMPLATE.md](./rfc/RFC_TEMPLATE.md)
- [完全な例](./rfc/EXAMPLE_full_feature_proposal.md)

## 設計議論への参加

### 提案プロセス

```
1. 提案の下書き作成（RFC テンプレートを使用）
   → rfc/ ディレクトリに配置

2. コミュニティ議論
   → rfc/REPO の対応する issue で議論

3. コアチームレビュー
   → 採用 → accepted/ に移動
   → 拒否 → archived/ に移動または削除
```

### 設計原則

- **明確な境界**：各設計意思決定には明確な適用範囲が必要です
- **実用優先**：現実の問題解決、架空の脅威ではなく
- **漸進的透明性**：並行モデルのレイヤー設計（L1-L3）
- **ユーザー可視動作の不変性**：Never break userspace

## コード例

```yaoxiang
// 型定義
Point: Type = { x: Float, y: Float }
Result: Type[T, E] = { ok(T) | err(E) }

// 関数定義
add: (a: Int, b: Int) -> Int = a + b

// 並作関数（自動並行化）
fetch_data: (url: String) -> JSON spawn = {
    HTTP.get(url).json()
}

// main 関数
main: () -> Void = {
    print("Hello, YaoXiang!")
}
```

## 重要な設計意思決定

### 1. 型システム

- **統合型構文**：`enum`、`struct`、`union` を廃止し、`Name: Type = {...}` に統一
- **コンストラクター即タイプ**：「型」と「値」の溝を消除
- **ジェネリクス対応**：コンパイル時の単態化、ゼロランタイムオーバーヘッド

### 2. 並作モデル

```yaoxiang
// 三層並行抽象

// L1: @blocking 同期（並行化無効）
fetch: (String) -> JSON @blocking = (url) => { ... }

// L2: spawn 明示的並行化
process: () -> Void spawn = () => {
    users = fetch_users()
    posts = fetch_posts()  // 自動並行化
}

// L3: 完全透過（デフォルト）
compute: (Int) -> Int = (n) => {
    a = heavy_calc(1)  // システムが自動的に依存関係を分析
    b = heavy_calc(2)
    c = heavy_calc(3)
    a + b + c
}
```

### 3. エラー処理

```yaoxiang
Result: Type[T, E] = { ok(T) | err(E) }

process: () -> Result[Data, Error] = {
    data = fetch_data()?      // ? 演算子が透過的に伝播
    transformed = transform(data)?
    save(transformed)?
}
```

## 関連リソース

- [チュートリアル](../tutorial/) - YaoXiang の使い方を学ぶ
- [リファレンスドキュメント](../reference/) - API と標準ライブラリ
- [言語仕様](../reference/language-spec/index.md) - 完全な言語仕様
- [GitHub Discussions](https://github.com/ChenXu233/YaoXiang/discussions)
- [コントリビューションガイド](../tutorial/contributing.md)

## 歴史アーカイブ

設計プロセスの歴史ドキュメントは [`docs/old/`](../../old/) ディレクトリに移動されました：
- 初期アーキテクチャ設計
- 廃棄された提案
- 時代遅れの実装計画
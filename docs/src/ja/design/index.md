# YaoXiang 設計ドキュメント

> 道生一，一生二，二生三，三生万物。

本ディレクトリには YaoXiang プログラミング言語の設計上の決定事項、提案、議論が含まれています。

## コア設計理念

| 理念 | 説明 |
|------|------|
| **すべてが型** | 値、関数、モジュールはすべて型であり、型は第一級市民である |
| **自然な構文** | Python のような可読性、自然言語に近い |
| **所有権モデル** | ゼロコスト抽象化、GC なし、高性能 |
| **spawn モデル** | 同期的な構文、非同期の本質、自動並列化 |
| **AI フレンドリー** | 厳格な構造化、明確な AST |

## 設計ドキュメントの構造

```
design/
├── index.md              # 本インデックス
├── accepted/             # 承認済み設計提案
│   └── *.md
├── rfc/                  # RFC 提案（審議中）
│   ├── *.md
│   └── RFC_TEMPLATE.md
└── discussion/           # 設計議論エリア（オープン議論）
    └── *.md
```

## 承認済み設計提案

| ドキュメント | 状態 | 説明 |
|------|------|------|
| [008-並行処理モデル](./accepted/008-runtime-concurrency-model.md) | ✅ 正式 | spawn モデルとタスクスケジューラの設計 |

> 完全なリストは [`accepted/`](./accepted/) ディレクトリを参照してください。

## RFC 提案

> RFC（Request for Comments）は、新機能と重大な変更のための提案プロセスです。

### アクティブな提案

| 番号 | タイトル | 状態 |
|------|------|------|
| RFC-003 | バージョン計画 | 審議待ち |
| RFC-005 | 自動 CVE スキャン | 審議待ち |
| RFC-006 | ドキュメントサイト最適化 | 審議待ち |
| RFC-012 | f-string テンプレート文字列 | 審議待ち |

### RFC テンプレート

新規提案の提出前に、以下を参照してください：
- [RFC_TEMPLATE.md](./rfc/RFC_TEMPLATE.md)
- [完全な例](./rfc/EXAMPLE_full_feature_proposal.md)

## 設計議論への参加

### 提案プロセス

```
1. 提案の起草（RFC テンプレートを使用）
   → rfc/ ディレクトリに配置

2. コミュニティ議論
   → rfc/REPO の対応する issue で議論

3. コアチームレビュー
   → 承認 → accepted/ に移動
   → 拒否 → archived/ に移動または削除
```

### 設計原則

- **明確な境界**：各設計決定には明確な適用範囲が必要
- **実用優先**：实际问题解决优先，而非假想威胁
- **漸進的透明性**：並行処理モデルの階層的設計（L1-L3）
- **ユーザー可见動作不变**：Never break userspace

## コード例

```yaoxiang
// 型定義
Point: Type = { x: Float, y: Float }
Result: Type[T, E] = { ok(T) | err(E) }

// 関数定義
add: (a: Int, b: Int) -> Int = a + b

// spawn 関数（自動並行処理）
fetch_data: (url: String) -> JSON spawn = {
    HTTP.get(url).json()
}

// main 関数
main: () -> Void = {
    print("Hello, YaoXiang!")
}
```

## 重要な設計上の決定

### 1. 型システム

- **統一型構文**：`enum`、`struct`、`union` を廃止し、統一して `Name: Type = {...}` を使用
- **コンストラクタは型である**：型と値の間の溝を消除
- **generics サポート**：compile-time 単態化、ゼロ実行時オーバーヘッド

### 2. spawn モデル

```yaoxiang
// 三層並行処理抽象化

// L1: @blocking 同期（並行処理無効）
fetch: (String) -> JSON @blocking = (url) => { ... }

// L2: spawn 明示的並行処理
process: () -> Void spawn = () => {
    users = fetch_users()
    posts = fetch_posts()  // 自動並行処理
}

// L3: 完全透過的（デフォルト）
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
    data = fetch_data()?      // ? 演算子は透過的に伝播
    transformed = transform(data)?
    save(transformed)?
}
```

## 関連リソース

- [チュートリアル](../tutorial/) - YaoXiang の使い方
- [リファレンスドキュメント](../reference/) - API と標準ライブラリ
- [言語仕様](../reference/language-spec/index.md) - 完全な言語仕様
- [GitHub Discussions](https://github.com/ChenXu233/YaoXiang/discussions)
- [貢献ガイド](../tutorial/contributing.md)

## 歴史アーカイブ

設計プロセスの歴史ドキュメントは [`docs/old/`](../../old/) ディレクトリに移動されました：
- 初期アーキテクチャ設計
- 廃棄された提案
- 時代遅れの実装計画
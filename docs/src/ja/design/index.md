# YaoXiang 設計ドキュメント

> 道生一，一生二，二生三，三生万物。

本ディレクトリには YaoXiang プログラミング言語の設計決定、提案、議論が含まれています。

## コア設計理念

| 理念 | 説明 |
|------|------|
| **すべてが型** | 値、関数、モジュールはすべて型であり、型は第一級市民である |
| **自然な構文** | Python のような可読性、自然言語に近い |
| **所有権モデル** | ゼロコスト抽象化、GC なし、高パフォーマンス |
| **スポーン モデル** | 同期構文、非同期本質、自动並行処理 |
| **AI フレンドリー** | 厳格な構造化、明確な AST |

## 設計ドキュメント構造

```
design/
├── index.md              # 本インデックス
├── deprecated/           # 非推奨（新設計に置き換え）
│   └── *.md
├── rejected/             # 拒否済み
│   └── *.md
├── rfc/
│   ├── draft/            # 草案（作業中）
│   ├── review/           # レビュー中（議論開放）
│   ├── accepted/         # 承認済み（設計通過）
│   ├── deprecated/       # 非推奨（置き換え）
│   └── rejected/         # 拒否済み（不採用）
└── discussion/           # 設計議論エリア（議論開放）
    └── *.md
```

## 承認済みの設計提案

| ドキュメント | 状態 | 説明 |
|------|------|------|
| [RFC-001 並行モデルエラー処理](./rfc/accepted/001-concurrent-model-error-handling.md) | ✅ 承認済み | 並行モデルにおけるエラー処理設計 |
| [RFC-008 ランタイム並行モデル](./rfc/accepted/008-runtime-concurrency-model.md) | ✅ 承認済み | スポーン モデルとタスクスケジューラ設計 |
| [RFC-009 所有権モデル](./rfc/accepted/009-ownership-model.md) | ✅ 承認済み | 所有権と借用システム設計 |
| [RFC-010 統合型構文](./rfc/accepted/010-unified-type-syntax.md) | ✅ 承認済み | 統合型定義構文 |
| [RFC-011 ジェネリクス型システム](./rfc/accepted/011-generic-type-system.md) | ✅ 承認済み | ジェネリクス型システム設計 |

> 完全なリストは [`rfc/accepted/`](./rfc/accepted/) ディレクトリを参照。

## RFC 提案

> RFC（Request for Comments）は、新機能および重大な変更の提案プロセスです。

### アクティブな提案

| 番号 | タイトル | 状態 |
|------|------|------|
| RFC-003 | バージョン計画 | レビュー中 |
| RFC-016 | 量子ネイティブサポート | 草案 |
| RFC-018 | LLVM AOT コンパイラ | レビュー中 |
| RFC-019 | 型付き同像性 | 草案 |
| RFC-020 | 動的モジュール FFI | 草案 |
| RFC-021 | ライブラリ駆動 FFI 拡張 | レビュー中 |
| RFC-022 | Hoare 論理静的検証 | レビュー中 |

### 承認済み提案

| 番号 | タイトル | 状態 |
|------|------|------|
| RFC-001 | 並行モデルエラー処理 | 承認済み |
| RFC-004 | 柯里化多位置バインディング | 承認済み |
| RFC-006 | ドキュメントサイト最適化 | 承認済み |
| RFC-007 | 関数構文統合 | 承認済み |
| RFC-008 | ランタイム並行モデル | 承認済み |
| RFC-009 | 所有権モデル | 承認済み |
| RFC-010 | 統合型構文 | 承認済み |
| RFC-011 | ジェネリクス型システム | 承認済み |
| RFC-012 | f-string テンプレート文字列 | 承認済み |
| RFC-013 | エラーコード規範 | 承認済み |
| RFC-014 | パッケージマネージャ | 承認済み |
| RFC-015 | 設定システム | 承認済み |
| RFC-017 | LSP サポート | 承認済み |
| RFC-023 | クロージャ捕獲モデル | 承認済み |

### 拒否済み提案

| 番号 | タイトル | 状態 |
|------|------|------|
| RFC-002 | クロスプラットフォーム IO (libuv) | 拒否済み |
| RFC-005 | 自動 CVE スキャン | 拒否済み |

### RFC テンプレート

新規提案の提出前に、以下を参照してください：
- [RFC_TEMPLATE.md](./rfc/RFC_TEMPLATE.md)
- [完全示例](./rfc/EXAMPLE_full_feature_proposal.md)

## 設計議論への参加

### RFC ライフサイクル

RFC 提案には 5 つの状態があります：

| 状態 | 意味 |
|------|------|
| 草案 | 作業進行中 |
| レビュー中 | 議論開放 |
| 承認済み | 設計通過 |
| 非推奨 | かつて承認され、新設計に置き換え |
| 拒否済み | 不採用 |

完全なライフサイクル：
```
草案 → レビュー中 → 承認済み → 非推奨（置き換え）
                    ↓
                 拒否済み（不採用）
```

### 提案プロセス

```
1. 提案の起草（RFC テンプレートを使用）
   → rfc/draft/ に配置

2. レビュー提出
   → rfc/review/ に移動、社区議論開放

3. コアチームレビュー
   → 承認 → rfc/accepted/ に移動
   → 拒否 → rfc/rejected/ に移動

4. 継続的なメンテナンス
   → 置き換え → rfc/deprecated/ に移動
```

### 設計原則

- **明確な境界**: 各設計決定には明確な適用範囲が必要
- **実用優先**: 实际问题を解決し、假设の脅威に対応しない
- **ユーザー可視動作の不変性**: Never break userspace

## コード示例

```yaoxiang
// 型定義
Point: Type = { x: Float, y: Float }
Result: Type(T, E) = { ok(T) | err(E) }

// 関数定義
add: (a: Int, b: Int) -> Int = a + b

// main 関数
main: () -> Void = {
    print("Hello, YaoXiang!")
}
```

## 重要な設計決定

### 1. 型システム

- **統合型構文**: `enum`、`struct`、`union` を廃止し、統一して `Name: Type = {...}` を使用
- **コンストラクタ即型**: 「型」と「値」の溝を消除
- **ジェネリクス対応**: コンパイル時モノモルフィゼーション、ゼロランタイムオーバーヘッド

### 2. スポーン モデル

```yaoxiang
// スポーン モデル：デフォルトは順序実行、spawn でデータフロー並行処理を導入

// デフォルトは順序実行
compute: (Int) -> Int = (n) => {
    a = heavy_calc(1)
    b = heavy_calc(2)  // 順序実行、a の完了を待機
    c = heavy_calc(3)  // 順序実行、b の完了を待機
    a + b + c
}

// spawn ブロックでデータフロー並行処理を導入
process: () -> Void = () => {
    spawn {
        users = fetch_users()   // 並行
        posts = fetch_posts()   // 並行
    }
    // 呼び出し元が結果を同期的にブロックして待機
    render(users, posts)
}
```

### 3. エラー処理

```yaoxiang
Result: Type(T, E) = { ok(T) | err(E) }

process: () -> Result(Data, Error) = {
    data = fetch_data()?      // ? 演算子は透過的に伝播
    transformed = transform(data)?
    save(transformed)?
}
```

## 関連リソース

- [チュートリアル](../tutorial/) - YaoXiang の使い方学习
- [リファレンスドキュメント](../reference/) - API と標準ライブラリ
- [言語仕様](../reference/language-spec/index.md) - 完全な言語仕様
- [GitHub Discussions](https://github.com/ChenXu233/YaoXiang/discussions)
- [貢献ガイド](../tutorial/contributing.md)

## 歴史アーカイブ

設計プロセスの歴史ドキュメントは [`docs/old/`](../../old/) ディレクトリに移動されました、そこには：
- 初期アーキテクチャ設計
- 廃棄された提案
- 時代遅れの実装計画
# YaoXiang 設計ドキュメント

> 道は一を生じ、一は二を生じ、二は三を生じ、三は万物を生ず。

本ディレクトリには YaoXiang プログラミング言語の設計判断、提案、議論が含まれています。

## 核心設計理念

| 理念 | 説明 |
|------|------|
| **すべては型である** | 値、関数、モジュールはすべて型であり、型は第一級市民である |
| **自然な構文** | Python のような可読性、自然言語に近い |
| **所有権モデル** | ゼロコスト抽象、GC なし、高性能 |
| **spawn モデル** | 同期構文、非同期の本質、自動並列 |
| **AI フレンドリー** | 厳密な構造化、明確な AST |

## 設計ドキュメントの構成

```
design/
├── index.md              # 本索引
├── deprecated/           # 廃止済み（新設計に置換）
│   └── *.md
├── rejected/             # 拒否済み
│   └── *.md
├── rfc/
│   ├── draft/            # 草案（作業中）
│   ├── review/           # レビュー中（オープン議論）
│   ├── accepted/         # 承認済み（設計通過）
│   ├── deprecated/       # 廃止済み（置換）
│   └── rejected/         # 拒否済み（不通過）
└── discussion/           # 設計議論エリア（オープン議論）
    └── *.md
```

## 承認済みの設計提案

| ドキュメント | 状態 | 説明 |
|------|------|------|
| [RFC-010 統一型構文](./rfc/accepted/010-unified-type-syntax.md) | ✅ 承認済み | 型定義構文の統一 |
| [RFC-011 ジェネリクス型システム](./rfc/accepted/011-generic-type-system.md) | ✅ 承認済み | ジェネリクス型システムの設計 |
| [RFC-009 所有権モデル](./rfc/accepted/009-ownership-model.md) | ✅ 承認済み | 所有権と借用システムの設計 |
| [RFC-024 並行モデル](./rfc/accepted/024-concurrency-model.md) | ✅ 承認済み | spawn 並行プリミティブの意味論 |
| [RFC-027 コンパイル時アサーション](./rfc/accepted/027-compile-time-evaluation-types.md) | ✅ 承認済み | コンパイル時述語と静的検証 |
|
| > 完全なリスト（計 16 件）は [`rfc/accepted/`](./rfc/accepted/) ディレクトリを、最新状態は [`rfc/index.md`](./rfc/index.md) をご覧ください。

## RFC 提案

> RFC（Request for Comments）は新機能や重要な変更の提案プロセスです。


### アクティブな提案
| 番号 | タイトル | 状態 |
|------|------|------|
| RFC-019 | 型付き同像性 | 草案 |
| RFC-028 | JIT コンパイラ | 草案 |
| RFC-029 | モジュール意味論システム | 草案 |
| RFC-031 | 最適化レベル | 草案 |
| RFC-033 | ^^ 反射演算子 | 草案 |
| RFC-034 | デバッグツールチェーン | 草案 |
| RFC-035 | MCP サーバ | 草案 |
| RFC-002 | クロスプラットフォーム IO（libuv） | 草案 |
| RFC-026b | yx-bindgen | 草案 |
| RFC-011a | インターフェース実装と動的ディスパッチ | レビュー中 |
| RFC-014a | Registry プロトコル | レビュー中 |
| RFC-014b | ビルドシステム | レビュー中 |
| RFC-014c | ワークスペース | レビュー中 |
| RFC-026a | 拡張可能 FFI | レビュー中 |
| RFC-032 | spawn 統一式 | レビュー中 |

### 承認済みの提案
| 番号 | タイトル | 状態 |
|------|------|------|
| RFC-004 | カリー化複数位置バインディング | 承認済み |
| RFC-006 | ドキュメントサイト最適化 | 承認済み |
| RFC-007 | 関数構文の統一 | 承認済み |
| RFC-008 | ランタイム並行モデル | 承認済み |
| RFC-009 | 所有権モデル | 承認済み |
| RFC-009a | トークンライフタイム解析 | 承認済み |
| RFC-010 | 統一型構文 | 承認済み |
| RFC-011 | ジェネリクスシステム | 承認済み |
| RFC-012 | f-string | 承認済み |
| RFC-013 | エラーコード規範 | 承認済み |
| RFC-014 | パッケージマネージャ | 承認済み |
| RFC-015 | 設定システム | 承認済み |
| RFC-017 | LSP サポート | 承認済み |
| RFC-018 | LLVM AOT コンパイラ | 承認済み |
| RFC-024 | 並行モデル | 承認済み |
| RFC-026 | FFI コア機構 | 承認済み |
| RFC-027 | コンパイル時アサーション | 承認済み |
| RFC-030 | assert 機構 | 承認済み |

### 拒否済みの提案
| 番号 | タイトル | 状態 |
|------|------|------|
| RFC-003 | バージョン計画 | 拒否 |
| RFC-005 | CVE スキャン | 拒否 |
| RFC-016 | 量子ネイティブサポート | 拒否 |
| RFC-025 | プリミティブ型拡張 | 拒否 |
### RFC テンプレート

新しい提案を提出する前に、以下を参照してください：
- [RFC_TEMPLATE.md](./rfc/RFC_TEMPLATE.md)
- [完全な例](./rfc/EXAMPLE_full_feature_proposal.md)

## 設計議論への参加

### RFC ライフサイクル

RFC 提案には 5 つの状態があります：

| 状態 | 意味 |
|------|------|
| 草案 | 作業中 |
| レビュー中 | オープン議論 |
| 承認済み | 設計通過 |
| 廃止済み | かつて承認されたが、新設計に置換 |
| 拒否 | 不通過 |

完全なライフサイクル：
```
草案 → レビュー中 → 承認済み → 廃止済み（置換）
                  ↓
               拒否（不通過）
```

### 提案プロセス

```
1. 提案を起草（RFC テンプレートを使用）
   → rfc/draft/ に配置

2. レビューに提出
   → rfc/review/ に移動し、コミュニティ議論をオープン

3. コアチームによる評価
   → 承認 → rfc/accepted/ に移動
   → 拒否 → rfc/rejected/ に移動

4. その後のメンテナンス
   → 置換される → rfc/deprecated/ に移動
```

### 設計原則

- **明確な境界**：各設計判断には明確な適用範囲が必要
- **実用優先**：実際の問題を解決する、想定上の脅威ではなく
- **ユーザーに見える振る舞いは不変**：Never break userspace

## コード例

```yaoxiang
// 型定義
Point: Type = { x: Float, y: Float }
Result: Type(T, E) = { ok(T) | err(E) }

// 関数定義
add: (a: Int, b: Int) -> Int = a + b

// メイン関数
main: () -> Void = {
    print("Hello, YaoXiang!")
}
```

## 重要な設計判断

### 1. 型システム

- **統一型構文**：`enum`、`struct`、`union` を廃止し、統一して `Name: Type = {...}` を使用
- **コンストラクタは型である**：「型」と「値」の溝を排除
- **ジェネリクスサポート**：コンパイル時モノモーフィゼーション、ランタイムオーバーヘッドゼロ

### 2. spawn モデル

```yaoxiang
// spawn モデル：デフォルトでは順次実行、spawn がデータフロー並列を導入

// デフォルトでは順次実行
compute: (Int) -> Int = (n) => {
    a = heavy_calc(1)
    b = heavy_calc(2)  // 順次実行、a の完了を待つ
    c = heavy_calc(3)  // 順次実行、b の完了を待つ
    a + b + c
}

// spawn ブロックがデータフロー並列を導入
process: () -> Void = () => {
    spawn {
        users = fetch_users()   // 並列
        posts = fetch_posts()   // 並列
    }
    // 呼び出し側は同期的に結果を待機
    render(users, posts)
}
```

### 3. エラー処理

```yaoxiang
Result: Type(T, E) = { ok(T) | err(E) }

process: () -> Result(Data, Error) = {
    data = fetch_data()?      // ? 演算子が透過的に伝播
    transformed = transform(data)?
    save(transformed)?
}
```

## 関連リソース

- [チュートリアル](../tutorial/) - YaoXiang の使い方を学ぶ
- [リファレンス](../reference/) - API と標準ライブラリ
- [言語仕様](../reference/language-spec/index.md) - 完全な言語仕様
- [GitHub Discussions](https://github.com/ChenXu233/YaoXiang/discussions)
- [コントリビューションガイド](../tutorial/contributing.md)

## 歴史的アーカイブ

設計過程の歴史的ドキュメントは [`docs/old/`](../../old/) ディレクトリに移動されました。内容：
- 初期のアーキテクチャ設計
- 廃止済みの提案
- 時代遅れの実装計画
# YaoXiang 設計ドキュメント

> 道は一を生じ、一は二を生じ、二は三を生じ、三は万物を生ず。

本ディレクトリには YaoXiang プログラミング言語の設計上の決定、提案、議論を含みます。

## 核心設計理念

| 理念 | 説明 |
|------|------|
| **全てが型** | 値、関数、モジュールは全て型；型は第一級市民 |
| **自然な構文** | Pythonのような可読性、自然言語に近い |
| **所有権モデル** | ゼロコスト抽象、GCなし、高性能 |
| **spawnモデル** | 同期構文、非同期本質、自動で並列 |
| **AI フレンドリー** | 厳格な構造化、明確な AST |

## 設計ドキュメントの構成

```
design/
├── index.md              # 本インデックス
├── deprecated/           # 廃止済み（新しい設計に置換）
│   └── *.md
├── rejected/             # 拒否済み
│   └── *.md
├── rfc/
│   ├── draft/            # ドラフト（作業進行中）
│   ├── review/           # レビュー中（オープンな議論）
│   ├── accepted/         # 採用済み（設計承認）
│   ├── deprecated/       # 廃止済み（置換済み）
│   └── rejected/         # 拒否済み（不承認）
└── discussion/           # 設計議論エリア（オープンな議論）
    └── *.md
```

## 採用済みの設計提案

| ドキュメント | 状態 | 説明 |
|------|------|------|
| [RFC-010 統一型構文](./rfc/accepted/010-unified-type-syntax.md) | ✅ 採用済み | 型定義構文の統一 |
| [RFC-011 ジェネリック型システム](./rfc/accepted/011-generic-type-system.md) | ✅ 採用済み | ジェネリック型システムの設計 |
| [RFC-009 所有権モデル](./rfc/accepted/009-ownership-model.md) | ✅ 採用済み | 所有権と借用システムの設計 |
| [RFC-024 並行モデル](./rfc/accepted/024-concurrency-model.md) | ✅ 採用済み | spawn並行プリミティブのセマンティクス |
| [RFC-027 コンパイル時アサーション](./rfc/accepted/027-compile-time-evaluation-types.md) | ✅ 採用済み | コンパイル時述語と静的検証 |
|
| > 完全なリスト（16件）は [`rfc/accepted/`](./rfc/accepted/) ディレクトリを、最新ステータスは [`rfc/index.md`](./rfc/index.md) を参照してください。

## RFC 提案

> RFC（Request for Comments）は新機能と重大な変更の提案プロセスです。


### アクティブな提案
| 番号 | タイトル | 状態 |
|------|------|------|
| RFC-019 | 型付き同像性 | ドラフト |
| RFC-028 | JITコンパイラ | ドラフト |
| RFC-029 | モジュール意味論システム | ドラフト |
| RFC-031 | 最適化レベル | ドラフト |
| RFC-033 | ^^リフレクション演算子 | ドラフト |
| RFC-034 | デバッグツールチェーン | ドラフト |
| RFC-035 | MCP Server | ドラフト |
| RFC-002 | クロスプラットフォームIO（libuv） | ドラフト |
| RFC-026b | yx-bindgen | ドラフト |
| RFC-011a | インターフェース実装と動的ディスパッチ | レビュー中 |
| RFC-014a | Registryプロトコル | レビュー中 |
| RFC-014b | ビルドシステム | レビュー中 |
| RFC-014c | ワークスペース | レビュー中 |
| RFC-026a | 拡張可能FFI | レビュー中 |
| RFC-032 | spawn統一式 | レビュー中 |

### 採用済み提案
| 番号 | タイトル | 状態 |
|------|------|------|
| RFC-004 | カリー化複数位置束縛 | 採用済み |
| RFC-006 | ドキュメントサイト最適化 | 採用済み |
| RFC-007 | 関数構文の統一 | 採用済み |
| RFC-008 | ランタイム並行モデル | 採用済み |
| RFC-009 | 所有権モデル | 採用済み |
| RFC-009a | トークン生存期間解析 | 採用済み |
| RFC-010 | 統一型構文 | 採用済み |
| RFC-011 | ジェネリックシステム | 採用済み |
| RFC-012 | f-string | 採用済み |
| RFC-013 | エラーコード規約 | 採用済み |
| RFC-014 | パッケージマネージャ | 採用済み |
| RFC-015 | 設定システム | 採用済み |
| RFC-017 | LSPサポート | 採用済み |
| RFC-018 | LLVM AOTコンパイラ | 採用済み |
| RFC-024 | 並行モデル | 採用済み |
| RFC-026 | FFIコア機構 | 採用済み |
| RFC-027 | コンパイル時アサーション | 採用済み |
| RFC-030 | assertアサーション機構 | 採用済み |

### 拒否済み提案
| 番号 | タイトル | 状態 |
|------|------|------|
| RFC-003 | バージョン計画 | 拒否済み |
| RFC-005 | CVEスキャン | 拒否済み |
| RFC-016 | 量子ネイティブサポート | 拒否済み |
| RFC-025 | プリミティブ型拡張 | 拒否済み |
### RFC テンプレート

新しい提案を提出する前に、以下を参照してください：
- [RFC_TEMPLATE.md](./rfc/RFC_TEMPLATE.md)
- [完全な例](./rfc/EXAMPLE_full_feature_proposal.md)

## 設計議論への参加

### RFC ライフサイクル

RFC 提案には 5 つの状態があります：

| 状態 | 意味 |
|------|------|
| ドラフト | 作業進行中 |
| レビュー中 | オープンな議論 |
| 採用済み | 設計承認 |
| 廃止済み | 採用されたが、新しい設計に置換された |
| 拒否済み | 不承認 |

完全なライフサイクル：
```
ドラフト → レビュー中 → 採用済み → 廃止済み（置換）
                       ↓
                    拒否済み（不承認）
```

### 提案プロセス

```
1. 提案を起草（RFC テンプレートを使用）
   → rfc/draft/ に配置

2. レビューに提出
   → rfc/review/ に移動、コミュニティの議論を開始

3. コアチームによる評価
   → 承認 → rfc/accepted/ に移動
   → 拒否 → rfc/rejected/ に移動

4. その後のメンテナンス
   → 置換 → rfc/deprecated/ に移動
```

### 設計原則

- **明確な境界**：各設計決定には明確な適用範囲が必要
- **実用優先**：実際の問題を解決する、想定上の脅威ではない
- **ユーザーに見える振る舞いを変えない**：Never break userspace

## コード例

```yaoxiang
// 型定義
Point: Type = { x: Float, y: Float }
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// 関数定義
add: (a: Int, b: Int) -> Int = a + b

// メイン関数
main: () -> Void = {
    print("Hello, YaoXiang!")
}
```

## 重要な設計決定

### 1. 型システム

- **統一型構文**：`enum`、`struct`、`union` を廃止し、`Name: Type = {...}` に統一
- **コンストラクタが型**：「型」と「値」の溝を解消
- **ジェネリックサポート**：コンパイル時モノモーフィゼーション、ランタイムオーバーヘッドゼロ

### 2. spawnモデル

```yaoxiang
// spawnモデル：デフォルトで順次実行、spawn がデータフローの並列性を導入

// デフォルトで順次実行
compute: (Int) -> Int = (n) => {
    a = heavy_calc(1)
    b = heavy_calc(2)  // 順次実行、a の完了を待つ
    c = heavy_calc(3)  // 順次実行、b の完了を待つ
    a + b + c
}

// spawn ブロックでデータフローの並列性を導入
process: () -> Void = () => {
    spawn {
        users = fetch_users()   // 並列
        posts = fetch_posts()   // 並列
    }
    // 呼び出し側は同期的にブロックして結果を待つ
    render(users, posts)
}
```

### 3. エラー処理

```yaoxiang
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

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
- [貢献ガイド](../tutorial/contributing.md)

## 歴史的アーカイブ

設計過程の歴史的ドキュメントは [`docs/old/`](../../old/) ディレクトリに移動済みで、以下を含みます：
- 初期のアーキテクチャ設計
- 廃止済みの提案
- 時代遅れの実装計画
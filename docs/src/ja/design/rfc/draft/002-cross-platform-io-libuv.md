---
title: 'RFC-002：libuvに基づくリソース型IO実装層'
status: '草案'
author: '晨煦'
created: '2026-01-05'
updated: '2026-07-05'
issue: '#102'
---

# RFC-002：libuvに基づくリソース型IO実装層

> **参考**:
>
> - [RFC-024：spawnブロックに基づく并发モデル](../accepted/024-concurrency-model.md)
> - [RFC-008：Runtime并发モデルとスケジューラの脱耦設計](../accepted/008-runtime-concurrency-model.md)
> - [RFC-009：所有権モデル設計](../accepted/009-ownership-model.md)
> - [并发モデル仕様](../../../../reference/language-spec/concurrency.md)

## 摘要

本ドキュメントはYaoXiangのIO実装層を定義する：libuvに基づきクロスプラットフォームIO能力を提供し、RFC-024リソース型システムの低レベル実装とする。

**コアポジショニング**：

```
RFC-024：リソース型定義（FilePath, HttpUrl, DBUrl, Console）
    ↓ 使用
RFC-002：リソース型IO実装（libuvに基づく）
    ↓ 低レベル
libuv：クロスプラットフォームIOエンジン（イベントループ + スレッドプール）
```

** 무엇이지 않음**：

- ❌ 「透明非同期」ではない——ユーザーはspawnブロックにより明示的に并发を制御する
- ❌ 「自動非同期化」ではない——IO操作はspawnブロック内で明示的に呼び出す必要がある
- ❌ 「開発者が低レベルの詳細を気にする必要がない」ではない——リソース型システムが并发安全を保証する

** 무엇인가**：

- ✅ リソース型（FilePath, HttpUrl, DBUrl, Console）のIO実装層
- ✅ クロスプラットフォームIOの統一（libuvがWindows/Linux/macOSの差異を処理）
- ✅ 共有イベントループアーキテクチャ（1つのlibuvイベントループがすべてのIOを処理）
- ✅ RFC-024リソース型システムとの統合

## 動機

### libuvが必要な理由

RFC-024はリソース型システムを定義している：

- `FilePath` - ファイルシステムパス
- `HttpUrl` - HTTPエンドポイント
- `DBUrl` - データベース接続
- `Console` - 標準出力

これらのリソース型は低レベルのIO実装を必要とする。libuvが提供するもの：

| ニーズ      | libuvが提供するもの                             |
| ---------- | ---------------------------------------------- |
| クロスプラットフォームIO | Windows/Linux/macOS APIの統一             |
| 非同期能力  | 共有イベントループ、すべてのworkerのIOを集中処理 |
| スレッドプール    | 阻塞操作専用のスレッドプール               |
| 并发安全    | 単一スレッドイベントループ、天然の競合なし       |

### RFC-024との関係

```
┌─────────────────────────────────────────────────────────┐
│  RFC-024：并发モデル                                       │
│  - spawn {} ブロック（明示的并发）                               │
│  - リソース型定義（FilePath, HttpUrl, DBUrl, Console）     │
│  - リソース競合検出（同一パスは自動串行化）                         │
└─────────────────────────────────────────────────────────┘
                          ↓ 利用
┌─────────────────────────────────────────────────────────┐
│  RFC-002：リソース型IO実装                               │
│  - FilePath → libuvファイルIO                              │
│  - HttpUrl → libuvネットワークIO                               │
│  - DBUrl → データベース接続プール                                  │
│  - Console → 標準出力串行化                              │
└─────────────────────────────────────────────────────────┘
                          ↓ 低レベル
┌─────────────────────────────────────────────────────────┐
│  libuv：クロスプラットフォームIOエンジン                                   │
│  - イベントループ                                              │
│  - スレッドプール                                                │
│  - クロスプラットフォーム統一 API                                        │
└─────────────────────────────────────────────────────────┘
```

---

## 提案

### 1. libuvアーキテクチャ

#### 1.1 共有イベントループアーキテクチャ

```
┌─────────────────────────────────────────────────────────┐
│                    Runtime                               │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐    │
│  │  Worker 0   │  │  Worker 1   │  │  Worker N   │    │
│  │  計算タスク    │  │  計算タスク    │  │  計算タスク    │    │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘    │
│         │                │                │            │
│         └────────────────┼────────────────┘            │
│                          ↓                              │
│  ┌─────────────────────────────────────────────────┐  │
│  │          libuv イベントループ（専用スレッド）               │  │
│  │          すべてのIO操作を処理                         │  │
│  └─────────────────────────────────────────────────┘  │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

**重要特性**：

- 1つの共有libuvイベントループ（専用スレッドで実行）
- すべてのworkerのIO操作がこの共有イベントループに提交される
- 単一スレッドイベントループは天然に競合を避ける
- リソース効率が高く、各workerごとにイベントループを作成する必要がない

#### 1.2 并发安全メカニズム

| libuv特性      | YaoXiang対応        | 并发安全   |
| ------------- | ------------------ | ---------- |
| 単一スレッドイベントループ | spawnブロック内の順序実行   | 天然の競合なし |
| スレッドプール隔離     | 阻塞操作がメンスレッドを阻塞しない | 共有状態なし |
| 非同期コールバック       | DAGスケジューラが依存関係を管理   | 確定性実行 |

### 2. リソース型IOマッピング

#### 2.1 FilePath → libuvファイルIO

```rust
// std.ioモジュール（libuvに基づく）
pub struct IoModule;

impl StdModule for IoModule {
    fn exports(&self) -> Vec<NativeExport> {
        vec![
            // ファイル操作 → libuv fs_* API
            NativeExport::new("read_file", "std.io.read_file",
                "(path: FilePath) -> String", native_read_file),
            NativeExport::new("write_file", "std.io.write_file",
                "(path: FilePath, content: String) -> Bool", native_write_file),
            NativeExport::new("append_file", "std.io.append_file",
                "(path: FilePath, content: String) -> Bool", native_append_file),
            // Console操作 → libuv tty API
            NativeExport::new("print", "std.io.print",
                "(...args) -> ()", native_print),
            NativeExport::new("println", "std.io.println",
                "(...args) -> ()", native_println),
        ]
    }
}

// libuvファイルIO実装
fn native_read_file(args: &[RuntimeValue], ctx: &mut NativeContext) -> Result<RuntimeValue, ExecutorError> {
    let path = extract_file_path(args)?;

    // libuvイベントループに提交
    // libuv非同期ファイル読み取り
    // 結果を返す
    ctx.uv_loop.fs_read(path)
}
```

#### 2.2 HttpUrl → libuvネットワークIO

```rust
// std.netモジュール（libuvに基づく）
pub struct NetModule;

impl StdModule for NetModule {
    fn exports(&self) -> Vec<NativeExport> {
        vec![
            // HTTP操作 → libuv http API
            NativeExport::new("http_get", "std.net.http_get",
                "(url: HttpUrl) -> Response", native_http_get),
            NativeExport::new("http_post", "std.net.http_post",
                "(url: HttpUrl, body: String) -> Response", native_http_post),
        ]
    }
}

// libuvネットワークIO実装
fn native_http_get(args: &[RuntimeValue], ctx: &mut NativeContext) -> Result<RuntimeValue, ExecutorError> {
    let url = extract_http_url(args)?;

    // libuvイベントループに提交
    // libuv非同期HTTPリクエスト
    // 結果を返す
    ctx.uv_loop.http_get(url)
}
```

#### 2.3 DBUrl → データベース接続プール

```rust
// std.dbモジュール（libuvに基づく）
pub struct DbModule;

impl StdModule for DbModule {
    fn exports(&self) -> Vec<NativeExport> {
        vec![
            // データベース操作 → libuvスレッドプール
            NativeExport::new("query", "std.db.query",
                "(url: DBUrl, sql: String) -> Rows", native_query),
        ]
    }
}

// libuvデータベースIO実装
fn native_query(args: &[RuntimeValue], ctx: &mut NativeContext) -> Result<RuntimeValue, ExecutorError> {
    let url = extract_db_url(args)?;
    let sql = extract_sql(args)?;

    // libuvスレッドプールに提交
    // データベースクエリはスレッドプールで実行
    // 完了後コールバックでメンスレッドに通知
    ctx.uv_loop.db_query(url, sql)
}
```

#### 2.4 Console → 標準出力串行化

```rust
// Console操作は自動串行化（RFC-024リソース型ルール）
// すべてのConsole操作は同じスレッド内で順序実行
fn native_print(args: &[RuntimeValue], ctx: &mut NativeContext) -> Result<RuntimeValue, ExecutorError> {
    let output = format_args(args);

    // Console操作串行化
    // libuv tty書き込み
    ctx.uv_loop.tty_write(output)
}
```

### 3. spawnブロックとの統合

#### 3.1 ユーザー視点

```yaoxiang
# リソース型定義（RFC-024）
FilePath: Resource
HttpUrl: Resource

# IO操作（RFC-002実装）
File.read: (FilePath) -> String
HTTP.get: (HttpUrl) -> Response

# ユーザーの明示的并发（RFC-024）
(a, b) = spawn {
    read_file("data.txt"),      # リソース型FilePath、libuv低レベル
    fetch("http://example.com") # リソース型HttpUrl、libuv低レベル
}
# コンパイラ：FilePathとHttpUrlに競合はなく、パラレル実行可能
```

#### 3.2 コンパイル時分析

```
コンパイラがspawnブロックを分析：
1. リソース型操作を識別
2. リソース競合を検出（同パス/同URLは自動串行化）
3. DAG実行計画を生成
4. IOノードをマーク（libuvに提交）
```

#### 3.3 ランタイム実行

```
ランタイムがspawnブロックを実行：
1. Worker 0がIOタスクを提交 → 共有イベントループ
2. Worker 1がIOタスクを提交 → 共有イベントループ
3. イベントループがすべてのIO操作を統一処理
4. IO完了後、対応するWorkerに通知
5. Workerが後続タスクを継続実行
```

### 4. Runtime三層アーキテクチャとlibuv

| レイヤ             | libuv使用   | 非同期能力       | 適用シナリオ             |
| ---------------- | ------------ | -------------- | -------------------- |
| Embedded Runtime | libuvなし    | 非同期なし         | WASM、ゲームスクリプト       |
| Standard Runtime | 共有イベントループ | IO非同期        | Webサービス、データパイプライン   |
| Full Runtime     | 共有イベントループ | IO非同期 + 并行 | 科学計算、大規模并行 |

**Embedded Runtime**：libuvなし、即時実行、非同期能力なし。

**Standard Runtime**：共有libuvイベントループ、すべてのIO操作が非同期処理される。

**Full Runtime**：共有libuvイベントループ、マルチスレッド并行 + IO非同期。

---

## 詳細設計

### 1. Rustバインディング構造

```rust
// libuvバインディングモジュール
pub mod uv {
    // イベントループ
    pub struct UvLoop {
        loop_handle: *mut uv_loop_t,
    }

    // ファイル操作
    pub trait FileOps {
        fn fs_read(&self, path: &str) -> Result<String, UvError>;
        fn fs_write(&self, path: &str, content: &str) -> Result<(), UvError>;
        fn fs_append(&self, path: &str, content: &str) -> Result<(), UvError>;
    }

    // ネットワーク操作
    pub trait NetOps {
        fn http_get(&self, url: &str) -> Result<Response, UvError>;
        fn http_post(&self, url: &str, body: &str) -> Result<Response, UvError>;
    }

    // データベース操作
    pub trait DbOps {
        fn db_query(&self, url: &str, sql: &str) -> Result<Rows, UvError>;
    }

    // Console操作
    pub trait ConsoleOps {
        fn tty_write(&self, data: &str) -> Result<(), UvError>;
    }
}
```

### 2. 標準ライブラリモジュール構造

```
src/std/
├── io.rs          # FilePath IO（libuvに基づく）
├── net.rs         # HttpUrl IO（libuvに基づく）
├── db.rs          # DBUrl IO（libuvに基づく）
├── console.rs     # Console IO（libuvに基づく）
└── mod.rs         # モジュール登録
```

### 3. DAGスケジューラとの統合

```rust
// IOノードインターフェース（RFC-008定義）
trait IoScheduler {
    // IOタスクを提交し、ハンドルを返す
    fn submit_io(&self, task: IoTask) -> IoHandle;

    // IO完了時にlibuvが呼び出し、DAGノードを起こす
    fn on_io_complete(&self, handle: IoHandle);
}

// libuv実装
impl IoScheduler for UvLoop {
    fn submit_io(&self, task: IoTask) -> IoHandle {
        match task.resource_type {
            ResourceType::FilePath => self.fs_read(task.path),
            ResourceType::HttpUrl => self.http_get(task.url),
            ResourceType::DBUrl => self.db_query(task.url, task.sql),
            ResourceType::Console => self.tty_write(task.data),
        }
    }

    fn on_io_complete(&self, handle: IoHandle) {
        // DAGスケジューラに下游ノードを起こすよう通知
        self.dag_scheduler.wake_dependents(handle.node_id);
    }
}
```

---

## トレードオフ

### 优点

1. **クロスプラットフォーム統一**：libuvがWindows/Linux/macOSの差異を処理
2. **IO非同期能力**：共有イベントループがすべてのIOを処理、async/await不要
3. **并发安全**：単一スレッドイベントループは天然に競合なし
4. **リソース効率**：1つのイベントループ、メモリオーバーヘッド小
5. **RFC-024との整合**：リソース型システムが并发安全を保証
6. **成熟と安定**：libuvはNode.jsの大規模検証を経ている

### 缺点

1. **Cライブラリ依存**：libuv Cライブラリへのバインディングが必要
2. **自己bootstrap制限**：自己bootstrap後はYaoXiangネイティブ実装に置き換える可能性
3. **WASMサポート**：追加の適応作業が必要

---

## 代替方案

| 方案         | なぜ選択しないか                                         |
| ----------- | -------------------------------------------------------- |
| Rust std::io | 同期阻塞的で、spawnブロックと組み合わせて非同期を実現できない                    |
| tokio        | Rust async/await向けに設計されており、YaoXiangの明示的并发モデルと整合しない |
| mio          | 生の非同期プリミティブのみ提供し、高级なIO機能を欠く                     |
| ゼロから実装     | 複雑でエラーしやすく、libuvの成熟度に匹敵できない                     |

---

## 実装策略

### フェーズ分け

1. **フェーズ1（v0.3）**：libuvバインディング、基本ファイルIO
2. **フェーズ2（v0.5）**：ネットワークIO、HTTPサポート
3. **フェーズ3（v0.7）**：データベースIO、接続プール
4. **フェーズ4（v1.0）**：WASM適応、パフォーマンス最適化

### 依存関係

- RFC-024（并发モデル）→ 完了済み
- RFC-008（Runtimeアーキテクチャ）→ 完了済み
- RFC-009（所有権モデル）→ 完了済み
- RFC-011（泛型システム）→ 完了済み

---

## 設計意思決定記録

| 意思決定         | 決定                      | 理由                        | 日付       |
| -------------- | ------------------------- | --------------------------- | ---------- |
| IO実装層    | libuv                     | クロスプラットフォーム、非同期能力、并发安全  | 2025-01-05 |
| ポジショニング         | リソース型IO実装層        | RFC-024リソース型システムとの統合 | 2026-06-16 |
| イベントループアーキテクチャ | 共有イベントループ              | リソース効率が高く、繰り返し作成を避ける    | 2026-06-16 |
| 并发安全     | 単一スレッドイベントループ            | 天然の競合なし、RFC-024と整合 | 2026-06-16 |
| 標準ライブラリ書き直し   | std.io/std.netはlibuvに基づく | クロスプラットフォーム統一、非同期能力        | 2026-06-16 |

---

## 開放問題

- [ ] WASM環境でのlibuv適応方案
- [ ] データベース接続プールの設計
- [ ] HTTPクライアントの完全実装
- [ ] ファイルシステムイベントのクロスプラットフォーム一貫性
- [ ] ネットワークIOのタイムアウトメカニズム設計
- [ ] 自己bootstrap後のlibuv置換戦略

---

## 参考文献

### YaoXiang公式ドキュメント

- [RFC-024并发モデル](../accepted/024-concurrency-model.md)
- [RFC-008 Runtimeアーキテクチャ](../accepted/008-runtime-concurrency-model.md)
- [RFC-009 所有権モデル](../accepted/009-ownership-model.md)
- [并发モデル仕様](../../../../reference/language-spec/concurrency.md)

### 外部参照

- [libuv公式ドキュメント](https://docs.libuv.org/)
- [Node.jsイベントループ](https://nodejs.org/en/docs/guides/event-loop-timers-and-nexttick/)
- [Rust libuvバインディング](https://github.com/libuv/libuv)

---

## ライフサイクルと归宿

| 状態     | 位置                     | 説明       |
| -------- | ------------------------ | ---------- |
| **草案** | `docs/design/rfc/draft/` | 再審査中 |
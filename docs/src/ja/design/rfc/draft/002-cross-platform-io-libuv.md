---
title: 'RFC-002：libuv ベースの Resource 型 IO 実装層'
status: '草案'
author: '晨煦'
created: '2025-01-05'
updated: '2026-07-05'
issue: '#102'
---

# RFC-002：libuv ベースの Resource 型 IO 実装層

> **参考**:
>
> - [RFC-024: spawn ブロックベースの並行モデル](./024-concurrency-model.md)
> - [RFC-008: Runtime 並行モデルとスケジューラの分離設計](./008-runtime-concurrency-model.md)
> - [RFC-009: 所有権モデル設計](./009-ownership-model.md)
> - [並行モデル仕様](/reference/language-spec/concurrency.md)

## 概要

このドキュメントは YaoXiang の IO 実装層を定義する：libuv に基づいてクロスプラットフォーム IO 機能を提供し、RFC-024 の Resource 型システムの基盤実装となる。

**コアポジショニング**：

```
RFC-024：Resource 型定義（FilePath, HttpUrl, DBUrl, Console）
    ↓ 使用
RFC-002：Resource 型 IO 実装（libuv ベース）
    ↓ 基盤
libuv：クロスプラットフォーム IO エンジン（イベントループ + スレッドプール）
```

**何でないか**：

- ❌ 「透過的非同期」ではない——ユーザーは spawn ブロックを通じて明示的に並行性を制御する
- ❌ 「自動非同期化」ではない——IO 操作は spawn ブロック内で明示的に呼び出す必要がある
- ❌ 「開発者が基盤の詳細を気にする必要がない」ではない——Resource 型システムが並行安全性を確保する

**何であるか**：

- ✅ Resource 型（FilePath, HttpUrl, DBUrl, Console）の IO 実装層
- ✅ クロスプラットフォーム IO の統一（libuv が Windows/Linux/macOS の差異を処理）
- ✅ 共有イベントループアーキテクチャ（1 つの libuv イベントループがすべての IO を処理）
- ✅ RFC-024 Resource 型システムとの統合

## 動機

### なぜ libuv が必要か？

RFC-024 は Resource 型システムを定義している：

- `FilePath` - ファイルシステムパス
- `HttpUrl` - HTTP エンドポイント
- `DBUrl` - データベース接続
- `Console` - 標準出力

これらの Resource 型は基盤 IO 実装を必要とする。libuv は以下を提供する：

| ニーズ                    | libuv の提供内容                                     |
| ------------------------- | ---------------------------------------------------- |
| クロスプラットフォーム IO | 統一された Windows/Linux/macOS API                   |
| 非同期機能                | 共有イベントループ、すべての worker の IO を集中処理 |
| スレッドプール            | ブロッキング操作専用のスレッドプール                 |
| 並行安全性                | シングルスレッドイベントループ、競合知らず           |

### RFC-024 との関係

```
┌─────────────────────────────────────────────────────────┐
│  RFC-024：並行モデル                                      │
│  - spawn {} ブロック（明示的並行性）                       │
│  - Resource 型定義（FilePath, HttpUrl, DBUrl, Console）   │
│  - Resource 競合検出（同一パスは自動直列化）               │
└─────────────────────────────────────────────────────────┘
                          ↓ 使用
┌─────────────────────────────────────────────────────────┐
│  RFC-002：Resource 型 IO 実装                             │
│  - FilePath → libuv ファイル IO                           │
│  - HttpUrl → libuv ネットワーク IO                         │
│  - DBUrl → データベース接続プール                          │
│  - Console → 標準出力の直列化                              │
└─────────────────────────────────────────────────────────┘
                          ↓ 基盤
┌─────────────────────────────────────────────────────────┐
│  libuv：クロスプラットフォーム IO エンジン                  │
│  - イベントループ                                          │
│  - スレッドプール                                          │
│  - クロスプラットフォーム統一 API                          │
└─────────────────────────────────────────────────────────┘
```

---

## 提案

### 1. libuv アーキテクチャ

#### 1.1 共有イベントループアーキテクチャ

```
┌─────────────────────────────────────────────────────────┐
│                    Runtime                               │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐    │
│  │  Worker 0   │  │  Worker 1   │  │  Worker N   │    │
│  │  計算タスク   │  │  計算タスク   │  │  計算タスク   │    │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘    │
│         │                │                │            │
│         └────────────────┼────────────────┘            │
│                          ↓                              │
│  ┌─────────────────────────────────────────────────┐  │
│  │          libuv イベントループ（専用スレッド）       │  │
│  │          すべての IO 操作を処理                    │  │
│  └─────────────────────────────────────────────────┘  │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

**重要な特性**：

- 1 つの共有 libuv イベントループ（専用スレッドで実行）
- すべての worker の IO 操作がこの共有イベントループに提交される
- シングルスレッドイベントループは競合を自然に避ける
- リソース効率が高い、各 worker ごとにイベントループを作成する必要がない

#### 1.2 並行安全性メカニズム

| libuv の特性                   | YaoXiang への対応                              | 並行安全性     |
| ------------------------------ | ---------------------------------------------- | -------------- |
| シングルスレッドイベントループ | spawn ブロック内の順次実行                     | 自然に競合なし |
| スレッドプール分離             | ブロッキング操作がメンスレッドをブロックしない | 共有状態なし   |
| 非同期コールバック             | DAG スケジューラが依存関係を管理               | 決定論的実行   |

### 2. Resource 型 IO マッピング

#### 2.1 FilePath → libuv ファイル IO

```rust
// std.io モジュール（libuv ベース）
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
            // Console 操作 → libuv tty API
            NativeExport::new("print", "std.io.print",
                "(...args) -> ()", native_print),
            NativeExport::new("println", "std.io.println",
                "(...args) -> ()", native_println),
        ]
    }
}

// libuv ファイル IO 実装
fn native_read_file(args: &[RuntimeValue], ctx: &mut NativeContext) -> Result<RuntimeValue, ExecutorError> {
    let path = extract_file_path(args)?;

    // libuv イベントループに提交
    // libuv 非同期ファイル読み取り
    // 結果を返す
    ctx.uv_loop.fs_read(path)
}
```

#### 2.2 HttpUrl → libuv ネットワーク IO

```rust
// std.net モジュール（libuv ベース）
pub struct NetModule;

impl StdModule for NetModule {
    fn exports(&self) -> Vec<NativeExport> {
        vec![
            // HTTP 操作 → libuv http API
            NativeExport::new("http_get", "std.net.http_get",
                "(url: HttpUrl) -> Response", native_http_get),
            NativeExport::new("http_post", "std.net.http_post",
                "(url: HttpUrl, body: String) -> Response", native_http_post),
        ]
    }
}

// libuv ネットワーク IO 実装
fn native_http_get(args: &[RuntimeValue], ctx: &mut NativeContext) -> Result<RuntimeValue, ExecutorError> {
    let url = extract_http_url(args)?;

    // libuv イベントループに提交
    // libuv 非同期 HTTP リクエスト
    // 結果を返す
    ctx.uv_loop.http_get(url)
}
```

#### 2.3 DBUrl → データベース接続プール

```rust
// std.db モジュール（libuv ベース）
pub struct DbModule;

impl StdModule for DbModule {
    fn exports(&self) -> Vec<NativeExport> {
        vec![
            // データベース操作 → libuv スレッドプール
            NativeExport::new("query", "std.db.query",
                "(url: DBUrl, sql: String) -> Rows", native_query),
        ]
    }
}

// libuv データベース IO 実装
fn native_query(args: &[RuntimeValue], ctx: &mut NativeContext) -> Result<RuntimeValue, ExecutorError> {
    let url = extract_db_url(args)?;
    let sql = extract_sql(args)?;

    // libuv スレッドプールに提交
    // データベースクエリはスレッドプールで実行
    // 完了後コールバックでメンスレッドに通知
    ctx.uv_loop.db_query(url, sql)
}
```

#### 2.4 Console → 標準出力の直列化

```rust
// Console 操作は自動直列化（RFC-024 Resource 型ルール）
// すべての Console 操作は同じスレッド内で順次実行
fn native_print(args: &[RuntimeValue], ctx: &mut NativeContext) -> Result<RuntimeValue, ExecutorError> {
    let output = format_args(args);

    // Console 操作の直列化
    // libuv tty 書き込み
    ctx.uv_loop.tty_write(output)
}
```

### 3. spawn ブロックとの統合

#### 3.1 ユーザー視点

```yaoxiang
# Resource 型定義（RFC-024）
FilePath: Resource
HttpUrl: Resource

# IO 操作（RFC-002 実装）
File.read: (FilePath) -> String
HTTP.get: (HttpUrl) -> Response

# ユーザーの明示的並行性（RFC-024）
(a, b) = spawn {
    read_file("data.txt"),      # Resource 型 FilePath、基盤は libuv
    fetch("http://example.com") # Resource 型 HttpUrl、基盤は libuv
}
# コンパイラ：FilePath と HttpUrl に競合なし、並列実行可能
```

#### 3.2 コンパイル時解析

```
コンパイラが spawn ブロックを解析：
1. Resource 型操作を識別
2. Resource 競合を検出（同一パス/同一 URL は自動直列化）
3. DAG 実行計画を生成
4. IO ノードをマーク（libuv に提交）
```

#### 3.3 ランタイム実行

```
ランタイムが spawn ブロックを実行：
1. Worker 0 が IO タスクを提交 → 共有イベントループ
2. Worker 1 が IO タスクを提交 → 共有イベントループ
3. イベントループがすべての IO 操作を一括処理
4. IO 完了後、対応する Worker に通知
5. Worker が後続タスクを引き続き実行
```

### 4. Runtime 三層アーキテクチャと libuv

| 层级             | libuv 使用         | 非同期機能       | 適用シナリオ                     |
| ---------------- | ------------------ | ---------------- | -------------------------------- |
| Embedded Runtime | libuv なし         | 非同期なし       | WASM、ゲームスクリプト           |
| Standard Runtime | 共有イベントループ | IO 非同期        | Web サービス、データパイプライン |
| Full Runtime     | 共有イベントループ | IO 非同期 + 並列 | 科学計算、大規模並列             |

**Embedded Runtime**：libuv なし、同期実行、非同期機能なし。

**Standard Runtime**：共有 libuv イベントループ、すべての IO 操作が非同期で処理される。

**Full Runtime**：共有 libuv イベントループ、マルチスレッド並列 + IO 非同期。

---

## 詳細設計

### 1. Rust バインディング構造

```rust
// libuv バインディングモジュール
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

    // Console 操作
    pub trait ConsoleOps {
        fn tty_write(&self, data: &str) -> Result<(), UvError>;
    }
}
```

### 2. 標準ライブラリモジュール構造

```
src/std/
├── io.rs          # FilePath IO（libuv ベース）
├── net.rs         # HttpUrl IO（libuv ベース）
├── db.rs          # DBUrl IO（libuv ベース）
├── console.rs     # Console IO（libuv ベース）
└── mod.rs         # モジュール登録
```

### 3. DAG スケジューラとの統合

```rust
// IO ノードインターフェース（RFC-008 で定義）
trait IoScheduler {
    // IO タスクを提交、ハンドルを返す
    fn submit_io(&self, task: IoTask) -> IoHandle;

    // IO 完了時に libuv が呼び出し、DAG ノードを起こす
    fn on_io_complete(&self, handle: IoHandle);
}

// libuv 実装
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
        // DAG スケジューラに通知、下流ノードを起こす
        self.dag_scheduler.wake_dependents(handle.node_id);
    }
}
```

---

## トレードオフ

### 利点

1. **クロスプラットフォームの統一**：libuv が Windows/Linux/macOS の差異を処理
2. **IO 非同期機能**：共有イベントループがすべての IO を処理、async/await 不要
3. **並行安全性**：シングルスレッドイベントループは自然に競合なし
4. **リソース効率**：1 つのイベントループ、小さなメモリオーバーヘッド
5. **RFC-024 との整合性**：Resource 型システムが並行安全性を確保
6. **成熟度と安定性**：libuv は Node.js での大規模検証済み

### 欠点

1. **C ライブラリ依存**：libuv C ライブラリのバインディングが必要
2. **ブートストラッピングの制約**：ブートストラッピング後は YaoXiang ネイティブ実装に置き換える必要性
3. **WASM サポート**：追加の適応作業が必要

---

## 代替案

| 案           | 選択しない理由                                                     |
| ------------ | ------------------------------------------------------------------ |
| Rust std::io | 同期ブロッキング、spawn ブロックとの非同期協調不可                 |
| tokio        | Rust async/await 向けに設計、YaoXiang の明示的並行モデルに合わない |
| mio          | 低水準な非同期プリミティブのみ、高水準 IO 機能欠如                 |
| 一から実装   | 複雑でバグりやすい、libuv の成熟度にかなわない                     |

---

## 実装戦略

### フェーズ分け

1. **フェーズ 1（v0.3）**：libuv バインディング、基盤ファイル IO
2. **フェーズ 2（v0.5）**：ネットワーク IO、HTTP サポート
3. **フェーズ 3（v0.7）**：データベース IO、接続プール
4. **フェーズ 4（v1.0）**：WASM 適応、パフォーマンス最適化

### 依存関係

- RFC-024（並行モデル）→ 完了済み
- RFC-008（Runtime アーキテクチャ）→ 完了済み
- RFC-009（所有権モデル）→ 完了済み
- RFC-011（ジェネリクスシステム）→ 完了済み

---

## 設計決定記録

| 決定                         | 決定内容                       | 理由                                           | 日付       |
| ---------------------------- | ------------------------------ | ---------------------------------------------- | ---------- |
| IO 実装層                    | libuv                          | クロスプラットフォーム、非同期機能、並行安全性 | 2025-01-05 |
| ポジショニング               | Resource 型 IO 実装層          | RFC-024 Resource 型システムとの統合            | 2026-06-16 |
| イベントループアーキテクチャ | 共有イベントループ             | リソース効率が高く、繰り返し作成を避ける       | 2026-06-16 |
| 並行安全性                   | シングルスレッドイベントループ | 自然に競合なし、RFC-024 との整合性             | 2026-06-16 |
| 標準ライブラリ書き直し       | std.io/std.net は libuv ベース | クロスプラットフォーム統一、非同期機能         | 2026-06-16 |

---

## 未解決の問題

- [ ] WASM 環境での libuv 適応方案
- [ ] データベース接続プールの設計
- [ ] HTTP クライアントの完全な実装
- [ ] ファイルシステムイベントのクロスプラットフォーム一貫性
- [ ] ネットワーク IO のタイムアウト機構の設計
- [ ] ブートストラッピング後の libuv 置換戦略

---

## 参考文献

### YaoXiang 公式ドキュメント

- [RFC-024 並行モデル](./024-concurrency-model.md)
- [RFC-008 Runtime アーキテクチャ](./008-runtime-concurrency-model.md)
- [RFC-009 所有権モデル](./009-ownership-model.md)
- [並行モデル仕様](/reference/language-spec/concurrency.md)

### 外部参照

- [libuv 公式ドキュメント](https://docs.libuv.org/)
- [Node.js イベントループ](https://nodejs.org/en/docs/guides/event-loop-timers-and-nexttick/)
- [Rust libuv バインディング](https://github.com/libuv/libuv)

---

## ライフサイクルと归宿

| 状態     | 場所                     | 説明     |
| -------- | ------------------------ | -------- |
| **草案** | `docs/design/rfc/draft/` | 再審査中 |

---
title: 'RFC-002: libuv をベースとしたリソース型 IO 実装層'
status: 'ドラフト'
author: '晨煦'
created: '2026-01-05'
updated: '2026-07-05'
issue: '#102'
---

# RFC-002: libuv をベースとしたリソース型 IO 実装層

> **参考**:
>
> - [RFC-024: spawn ブロックをベースとした並行モデル](../accepted/024-concurrency-model.md)
> - [RFC-008: Runtime 並行モデルとスケジューラの疎結合設計](../accepted/008-runtime-concurrency-model.md)
> - [RFC-009: 所有権モデル設計](../accepted/009-ownership-model.md)
> - [並行モデル仕様](../../../reference/language-spec/concurrency.md)

## 要約

本文書は YaoXiang の IO 実装層を定義する。libuv をベースとしてクロスプラットフォーム IO 機能を提供し、RFC-024 のリソース型システムの基盤実装となる。

**中核的な位置付け**:

```
RFC-024: リソース型定義 (FilePath, HttpUrl, DBUrl, Console)
    ↓ 使用
RFC-002: リソース型 IO 実装 (libuv ベース)
    ↓ 基盤
libuv: クロスプラットフォーム IO エンジン (イベントループ + スレッドプール)
```

**何ではないか**:

- ❌ 「透過的な非同期」ではない — ユーザーは spawn ブロックで明示的に並行性を制御する
- ❌ 「自動的な非同期化」ではない — IO 操作は spawn ブロック内で明示的に呼び出す必要がある
- ❌ 「開発者が低レベルの詳細を気にする必要がない」わけではない — リソース型システムが並行安全性を保証する

**何であるか**:

- ✅ リソース型 (FilePath, HttpUrl, DBUrl, Console) の IO 実装層
- ✅ クロスプラットフォーム IO の統合 (libuv が Windows/Linux/macOS の差異を吸収)
- ✅ 共有イベントループアーキテクチャ (単一の libuv イベントループがすべての IO を処理)
- ✅ RFC-024 リソース型システムとの統合

## 動機

### なぜ libuv が必要か

RFC-024 はリソース型システムを定義する:

- `FilePath` - ファイルシステムパス
- `HttpUrl` - HTTP エンドポイント
- `DBUrl` - データベース接続
- `Console` - 標準出力

これらのリソース型には低レベル IO 実装が必要である。libuv は次を提供する:

| 必要性                    | libuv の提供機能                                     |
| ------------------------- | ---------------------------------------------------- |
| クロスプラットフォーム IO | Windows/Linux/macOS の統一 API                       |
| 非同期能力                | 共有イベントループ、すべての worker の IO を集中処理 |
| スレッドプール            | ブロッキング操作専用のスレッドプール                 |
| 並行安全性                | シングルスレッドイベントループ、競合を自然に回避     |

### RFC-024 との関係

```
┌─────────────────────────────────────────────────────────┐
│  RFC-024: 並行モデル                                     │
│  - spawn {} ブロック (明示的な並行)                      │
│  - リソース型定義 (FilePath, HttpUrl, DBUrl, Console)   │
│  - リソース競合検出 (同一パスを自動直列化)               │
└─────────────────────────────────────────────────────────┘
                          ↓ 使用
┌─────────────────────────────────────────────────────────┐
│  RFC-002: リソース型 IO 実装                             │
│  - FilePath → libuv ファイル IO                         │
│  - HttpUrl → libuv ネットワーク IO                      │
│  - DBUrl → データベースコネクションプール                │
│  - Console → 標準出力の直列化                           │
└─────────────────────────────────────────────────────────┘
                          ↓ 基盤
┌─────────────────────────────────────────────────────────┐
│  libuv: クロスプラットフォーム IO エンジン               │
│  - イベントループ                                        │
│  - スレッドプール                                        │
│  - クロスプラットフォーム統一 API                        │
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
│  │  計算タスク │  │  計算タスク │  │  計算タスク │    │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘    │
│         │                │                │            │
│         └────────────────┼────────────────┘            │
│                          ↓                              │
│  ┌─────────────────────────────────────────────────┐  │
│  │          libuv イベントループ (専用スレッド)      │  │
│  │          すべての IO 操作を処理                   │  │
│  └─────────────────────────────────────────────────┘  │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

**主要な特徴**:

- 単一の共有 libuv イベントループ (専用スレッドで実行)
- すべての worker の IO 操作がこの共有イベントループに投入される
- シングルスレッドイベントループは競合を自然に回避する
- リソース効率が高く、worker ごとにイベントループを作成する必要がない

#### 1.2 並行性の安全機構

| libuv の特性                   | YaoXiang での対応                            | 並行安全性       |
| ------------------------------ | -------------------------------------------- | ---------------- |
| シングルスレッドイベントループ | spawn ブロック内の順次実行                   | 本質的に競合なし |
| スレッドプール隔離             | ブロッキング操作が主スレッドをブロックしない | 共有状態なし     |
| 非同期コールバック             | DAG スケジューラが依存関係を管理             | 決定論的実行     |

### 2. リソース型 IO マッピング

#### 2.1 FilePath → libuv ファイル IO

```rust
// std.io モジュール (libuv ベース)
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

    // libuv イベントループに投入
    // libuv による非同期ファイル読み込み
    // 結果を返す
    ctx.uv_loop.fs_read(path)
}
```

#### 2.2 HttpUrl → libuv ネットワーク IO

```rust
// std.net モジュール (libuv ベース)
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

    // libuv イベントループに投入
    // libuv による非同期 HTTP リクエスト
    // 結果を返す
    ctx.uv_loop.http_get(url)
}
```

#### 2.3 DBUrl → データベースコネクションプール

```rust
// std.db モジュール (libuv ベース)
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

    // libuv スレッドプールに投入
    // データベースクエリはスレッドプールで実行
    // 完了時にメインスレッドへコールバック通知
    ctx.uv_loop.db_query(url, sql)
}
```

#### 2.4 Console → 標準出力の直列化

```rust
// Console 操作は自動直列化される (RFC-024 リソース型ルール)
// すべての Console 操作は同じスレッド内で順次実行される
fn native_print(args: &[RuntimeValue], ctx: &mut NativeContext) -> Result<RuntimeValue, ExecutorError> {
    let output = format_args(args);

    // Console 操作の直列化
    // libuv tty への書き込み
    ctx.uv_loop.tty_write(output)
}
```

### 3. spawn ブロックとの統合

#### 3.1 ユーザーの視点

```yaoxiang
# リソース型定義 (RFC-024)
FilePath: Resource
HttpUrl: Resource

# IO 操作 (RFC-002 実装)
File.read: (FilePath) -> String
HTTP.get: (HttpUrl) -> Response

# ユーザーによる明示的な並行 (RFC-024)
(a, b) = spawn {
    read_file("data.txt"),      # リソース型 FilePath、libuv が基盤
    fetch("http://example.com") # リソース型 HttpUrl、libuv が基盤
}
# コンパイラ: FilePath と HttpUrl には競合がないので並列化可能
```

#### 3.2 コンパイル時の解析

```
コンパイラが spawn ブロックを解析:
1. リソース型操作を識別
2. リソース競合を検出 (同一パス/同一 URL を自動直列化)
3. DAG 実行計画を生成
4. IO ノードにマーク (libuv に投入)
```

#### 3.3 ランタイム実行

```
ランタイムが spawn ブロックを実行:
1. Worker 0 が IO タスクを投入 → 共有イベントループ
2. Worker 1 が IO タスクを投入 → 共有イベントループ
3. イベントループがすべての IO 操作を統一処理
4. IO 完了時に該当 Worker へ通知
5. Worker が後続タスクの実行を続行
```

### 4. Runtime 三層アーキテクチャと libuv

| レイヤー         | libuv の使用       | 非同期能力       | 適用シナリオ                     |
| ---------------- | ------------------ | ---------------- | -------------------------------- |
| Embedded Runtime | libuv なし         | 非同期なし       | WASM、ゲームスクリプト           |
| Standard Runtime | 共有イベントループ | IO 非同期        | Web サービス、データパイプライン |
| Full Runtime     | 共有イベントループ | IO 非同期 + 並列 | 科学計算、大規模並列             |

**Embedded Runtime**: libuv なし、即時実行、非同期能力なし。

**Standard Runtime**: 共有 libuv イベントループ、すべての IO 操作を非同期処理。

**Full Runtime**: 共有 libuv イベントループ、マルチスレッド並列 + IO 非同期。

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
├── io.rs          # FilePath IO (libuv ベース)
├── net.rs         # HttpUrl IO (libuv ベース)
├── db.rs          # DBUrl IO (libuv ベース)
├── console.rs     # Console IO (libuv ベース)
└── mod.rs         # モジュール登録
```

### 3. DAG スケジューラとの統合

```rust
// IO ノードインターフェース (RFC-008 で定義)
trait IoScheduler {
    // IO タスクを投入し、ハンドルを返す
    fn submit_io(&self, task: IoTask) -> IoHandle;

    // libuv が IO 完了時に呼び出し、DAG ノードを起床する
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
        // DAG スケジューラへ通知し、下流ノードを起床する
        self.dag_scheduler.wake_dependents(handle.node_id);
    }
}
```

---

## トレードオフ

### 利点

1. **クロスプラットフォーム統合**: libuv が Windows/Linux/macOS の差異を吸収
2. **IO 非同期能力**: 共有イベントループがすべての IO を処理、async/await 不要
3. **並行安全性**: シングルスレッドイベントループにより本質的に競合なし
4. **リソース効率**: 単一イベントループによるメモリオーバーヘッドの削減
5. **RFC-024 との適合**: リソース型システムが並行安全性を保証
6. **成熟した安定性**: libuv は Node.js で大規模に検証済み

### 欠点

1. **C ライブラリ依存**: libuv C ライブラリのバインディングが必要
2. **セルフホスティングの制限**: セルフホスティング後は YaoXiang ネイティブ実装への置換が必要
3. **WASM サポート**: 追加の適合作業が必要

---

## 代替案

| 案             | 採用しない理由                                                           |
| -------------- | ------------------------------------------------------------------------ |
| Rust std::io   | 同期ブロッキングであり、spawn ブロックと非同期で連携不可                 |
| tokio          | Rust async/await 向けに設計され、YaoXiang の明示的並行モデルと適合しない |
| mio            | 生の非同期プリミティブのみ提供し、高レベル IO 機能が不足                 |
| ゼロからの実装 | 複雑でエラーが発生しやすく、libuv の成熟度に及ばない                     |

---

## 実装戦略

### フェーズ分割

1. **フェーズ 1 (v0.3)**: libuv バインディング、基本ファイル IO
2. **フェーズ 2 (v0.5)**: ネットワーク IO、HTTP サポート
3. **フェーズ 3 (v0.7)**: データベース IO、コネクションプール
4. **フェーズ 4 (v1.0)**: WASM 適合、性能最適化

### 依存関係

- RFC-024 (並行モデル) → 完了済み
- RFC-008 (Runtime アーキテクチャ) → 完了済み
- RFC-009 (所有権モデル) → 完了済み
- RFC-011 (generics システム) → 完了済み

---

## 設計上の決定記録

| 決定                         | 採用                           | 理由                                           | 日付       |
| ---------------------------- | ------------------------------ | ---------------------------------------------- | ---------- |
| IO 実装層                    | libuv                          | クロスプラットフォーム、非同期能力、並行安全性 | 2025-01-05 |
| 位置付け                     | リソース型 IO 実装層           | RFC-024 リソース型システムとの統合             | 2026-06-16 |
| イベントループアーキテクチャ | 共有イベントループ             | リソース効率、繰り返し作成の回避               | 2026-06-16 |
| 並行安全性                   | シングルスレッドイベントループ | 本質的に競合なし、RFC-024 と適合               | 2026-06-16 |
| 標準ライブラリ再構築         | std.io/std.net は libuv ベース | クロスプラットフォーム統合、非同期能力         | 2026-06-16 |

---

## 未解決の問題

- [ ] WASM 環境における libuv 適合方案
- [ ] データベースコネクションプールの設計
- [ ] HTTP クライアントの完全な実装
- [ ] ファイルシステムイベントのクロスプラットフォーム一貫性
- [ ] ネットワーク IO のタイムアウト機構設計
- [ ] セルフホスティング後の libuv 置換戦略

---

## 参考文献

### YaoXiang 公式ドキュメント

- [RFC-024 並行モデル](../accepted/024-concurrency-model.md)
- [RFC-008 Runtime アーキテクチャ](../accepted/008-runtime-concurrency-model.md)
- [RFC-009 所有権モデル](../accepted/009-ownership-model.md)
- [並行モデル仕様](../../../reference/language-spec/concurrency.md)

### 外部参考資料

- [libuv 公式ドキュメント](https://docs.libuv.org/)
- [Node.js イベントループ](https://nodejs.org/en/docs/guides/event-loop-timers-and-nexttick/)
- [Rust libuv バインディング](https://github.com/libuv/libuv)

---

## ライフサイクルと帰趣

| 状態         | 場所                     | 説明     |
| ------------ | ------------------------ | -------- |
| **ドラフト** | `docs/design/rfc/draft/` | 再審査中 |

---
title: 'RFC-002：libuv ベースのリソースタイプ IO 実装層'
status: 'ドラフト'
author: '晨煦'
created: '2026-01-05'
updated: '2026-07-05'
issue: '#102'
---

# RFC-002：libuv ベースのリソースタイプ IO 実装層

> **参考**:
>
> - [RFC-024: spawn ブロックベースの並行性モデル](./024-concurrency-model.md)
> - [RFC-008: Runtime 並行性モデルとスケジューラの疎結合設計](./008-runtime-concurrency-model.md)
> - [RFC-009: 所有権モデル設計](./009-ownership-model.md)
> - [並行性モデル仕様](/reference/language-spec/concurrency.md)

## 概要

本文書では YaoXiang の IO 実装層を定義する：libuv に基づいてクロスプラットフォームの IO 機能を提供し、RFC-024 のリソースタイプシステムの基盤実装として機能する。

**中核となる位置付け**：

```
RFC-024：リソースタイプ定義（FilePath、HttpUrl、DBUrl、Console）
    ↓ 使用
RFC-002：リソースタイプ IO 実装（libuv ベース）
    ↓ 基盤
libuv：クロスプラットフォーム IO エンジン（イベントループ + スレッドプール）
```

**何ではないか**：

- ❌ 「透過的な非同期」ではない — ユーザーは spawn ブロックで明示的に並行性を制御する
- ❌ 「自動的な非同期化」ではない — IO 操作は spawn ブロック内で明示的に呼び出す必要がある
- ❌ 「開発者が基盤の詳細を気にする必要がない」わけではない — リソースタイプシステムが並行性の安全性を保証する

**何か**：

- ✅ リソースタイプ（FilePath、HttpUrl、DBUrl、Console）の IO 実装層
- ✅ クロスプラットフォーム IO の統一（libuv が Windows/Linux/macOS の差異を処理）
- ✅ 共有イベントループアーキテクチャ（1つの libuv イベントループがすべての IO を処理）
- ✅ RFC-024 のリソースタイプシステムとの統合

## 動機

### なぜ libuv が必要なのか？

RFC-024 はリソースタイプシステムを定義している：

- `FilePath` - ファイルシステムパス
- `HttpUrl` - HTTP エンドポイント
- `DBUrl` - データベース接続
- `Console` - 標準出力

これらのリソースタイプには基盤となる IO 実装が必要である。libuv は以下を提供する：

| 必要性                    | libuv の提供内容                                     |
| ------------------------- | ---------------------------------------------------- |
| クロスプラットフォーム IO | Windows/Linux/macOS を統一する API                   |
| 非同期機能                | 共有イベントループ、すべての worker の IO を集中処理 |
| スレッドプール            | ブロッキング操作専用のスレッドプール                 |
| 並行性の安全性            | シングルスレッドイベントループ、自然に競合なし       |

### RFC-024 との関係

```
┌─────────────────────────────────────────────────────────┐
│  RFC-024：並行性モデル                                   │
│  - spawn {} ブロック（明示的な並行性）                   │
│  - リソースタイプ定義（FilePath、HttpUrl、DBUrl、Console）│
│  - リソース競合検出（同一パスを自動直列化）               │
└─────────────────────────────────────────────────────────┘
                          ↓ 使用
┌─────────────────────────────────────────────────────────┐
│  RFC-002：リソースタイプ IO 実装                         │
│  - FilePath → libuv ファイル IO                         │
│  - HttpUrl → libuv ネットワーク IO                      │
│  - DBUrl → データベース接続プール                       │
│  - Console → 標準出力の直列化                           │
└─────────────────────────────────────────────────────────┘
                          ↓ 基盤
┌─────────────────────────────────────────────────────────┐
│  libuv：クロスプラットフォーム IO エンジン                │
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
│  │  計算タスク  │  │  計算タスク  │  │  計算タスク  │    │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘    │
│         │                │                │            │
│         └────────────────┼────────────────┘            │
│                          ↓                              │
│  ┌─────────────────────────────────────────────────┐  │
│  │       libuv イベントループ（専用スレッド）         │  │
│  │       すべての IO 操作を処理                       │  │
│  └─────────────────────────────────────────────────┘  │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

**主要な特徴**：

- 1つの共有 libuv イベントループ（専用スレッドで動作）
- すべての worker の IO 操作はこの共有イベントループに投入される
- シングルスレッドイベントループは自然に競合を回避
- リソース効率が高く、各 worker ごとにイベントループを作成する必要がない

#### 1.2 並行性安全性のメカニズム

| libuv の機能                   | YaoXiang での対応                                | 並行性の安全性 |
| ------------------------------ | ------------------------------------------------ | -------------- |
| シングルスレッドイベントループ | spawn ブロック内の順次実行                       | 自然に競合なし |
| スレッドプール分離             | ブロッキング操作はメインスレッドをブロックしない | 共有状態なし   |
| 非同期コールバック             | DAG スケジューラが依存関係を管理                 | 決定論的実行   |

### 2. リソースタイプ IO マッピング

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

    // libuv イベントループに投入
    // libuv がファイルを非同期で読み取る
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

    // libuv イベントループに投入
    // libuv が非同期で HTTP リクエストを実行
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

    // libuv スレッドプールに投入
    // データベースクエリはスレッドプールで実行される
    // 完了後、コールバックでメインスレッドに通知
    ctx.uv_loop.db_query(url, sql)
}
```

#### 2.4 Console → 標準出力の直列化

```rust
// Console 操作は自動的に直列化される（RFC-024 リソースタイプルール）
// すべての Console 操作は同じスレッド内で順次実行される
fn native_print(args: &[RuntimeValue], ctx: &mut NativeContext) -> Result<RuntimeValue, ExecutorError> {
    let output = format_args(args);

    // Console 操作の直列化
    // libuv tty 書き込み
    ctx.uv_loop.tty_write(output)
}
```

### 3. spawn ブロックとの統合

#### 3.1 ユーザーの視点

```yaoxiang
# リソースタイプ定義（RFC-024）
FilePath: Resource
HttpUrl: Resource

# IO 操作（RFC-002 実装）
File.read: (FilePath) -> String
HTTP.get: (HttpUrl) -> Response

# ユーザーの明示的な並行性（RFC-024）
(a, b) = spawn {
    read_file("data.txt"),      # リソースタイプ FilePath、基盤は libuv
    fetch("http://example.com") # リソースタイプ HttpUrl、基盤は libuv
}
# コンパイラ：FilePath と HttpUrl は競合しないため、並列実行可能
```

#### 3.2 コンパイル時解析

```
コンパイラが spawn ブロックを解析：
1. リソースタイプ操作を識別
2. リソース競合を検出（同一パス/同一 URL は自動直列化）
3. DAG 実行計画を生成
4. IO ノードにマークを付ける（libuv に投入）
```

#### 3.3 ランタイム実行

```
ランタイムが spawn ブロックを実行：
1. Worker 0 が IO タスクを投入 → 共有イベントループ
2. Worker 1 が IO タスクを投入 → 共有イベントループ
3. イベントループがすべての IO 操作を一元処理
4. IO 完了後、対応する Worker に通知
5. Worker が後続タスクの実行を継続
```

### 4. Runtime 三層アーキテクチャと libuv

| 階層             | libuv の使用       | 非同期機能       | 適用シナリオ                     |
| ---------------- | ------------------ | ---------------- | -------------------------------- |
| Embedded Runtime | libuv なし         | 非同期なし       | WASM、ゲームスクリプト           |
| Standard Runtime | 共有イベントループ | IO 非同期        | Web サービス、データパイプライン |
| Full Runtime     | 共有イベントループ | IO 非同期 + 並列 | 科学計算、大規模並列             |

**Embedded Runtime**：libuv なし、即時実行、非同期機能なし。

**Standard Runtime**：共有 libuv イベントループ、すべての IO 操作を非同期処理。

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
    // IO タスクを投入し、ハンドルを返す
    fn submit_io(&self, task: IoTask) -> IoHandle;

    // IO 完了時に libuv によって呼び出され、DAG ノードを起こす
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
        // DAG スケジューラに通知して下流ノードを起こす
        self.dag_scheduler.wake_dependents(handle.node_id);
    }
}
```

---

## トレードオフ

### 利点

1. **クロスプラットフォームの統一**：libuv が Windows/Linux/macOS の差異を処理
2. **IO 非同期機能**：共有イベントループがすべての IO を処理、async/await 不要
3. **並行性の安全性**：シングルスレッドイベントループで自然に競合なし
4. **リソース効率**：1つのイベントループでメモリオーバーヘッドが小さい
5. **RFC-024 との整合性**：リソースタイプシステムが並行性の安全性を保証
6. **成熟性と安定性**：libuv は Node.js で大規模に検証済み

### 欠点

1. **C ライブラリへの依存**：libuv C ライブラリのバインディングが必要
2. **セルフホスティングの制限**：セルフホスティング後に YaoXiang ネイティブ実装への置き換えが必要になる可能性
3. **WASM サポート**：追加の適応作業が必要

---

## 代替案

| 代替案       | 採用しない理由                                                                      |
| ------------ | ----------------------------------------------------------------------------------- |
| Rust std::io | 同期ブロッキングで、spawn ブロックと組み合わせて非同期を実現できない                |
| tokio        | Rust の async/await 用に設計されており、YaoXiang の明示的な並行性モデルと整合しない |
| mio          | 生の非同期プリミティブのみ提供、高度な IO 機能がない                                |
| ゼロから実装 | 複雑でエラーが発生しやすく、libuv の成熟度と比肩できない                            |

---

## 実装戦略

### 段階分け

1. **段階 1（v0.3）**：libuv バインディング、基本的なファイル IO
2. **段階 2（v0.5）**：ネットワーク IO、HTTP サポート
3. **段階 3（v0.7）**：データベース IO、接続プール
4. **段階 4（v1.0）**：WASM 適応、パフォーマンス最適化

### 依存関係

- RFC-024（並行性モデル）→ 完了
- RFC-008（Runtime アーキテクチャ）→ 完了
- RFC-009（所有権モデル）→ 完了
- RFC-011（generics システム）→ 完了

---

## 設計上の決定記録

| 決定                         | 選択                             | 理由                                               | 日付       |
| ---------------------------- | -------------------------------- | -------------------------------------------------- | ---------- |
| IO 実装層                    | libuv                            | クロスプラットフォーム、非同期機能、並行性の安全性 | 2025-01-05 |
| 位置付け                     | リソースタイプ IO 実装層         | RFC-024 のリソースタイプシステムと統合             | 2026-06-16 |
| イベントループアーキテクチャ | 共有イベントループ               | リソース効率が高く、重複作成を回避                 | 2026-06-16 |
| 並行性の安全性               | シングルスレッドイベントループ   | 自然に競合なし、RFC-024 と整合                     | 2026-06-16 |
| 標準ライブラリ書き換え       | std.io/std.net を libuv ベースに | クロスプラットフォームの統一、非同期機能           | 2026-06-16 |

---

## オープンな問題

- [ ] WASM 環境での libuv 適応方案
- [ ] データベース接続プールの設計
- [ ] HTTP クライアントの完全な実装
- [ ] ファイルシステムイベントのクロスプラットフォームの一貫性
- [ ] ネットワーク IO のタイムアウトメカニズム設計
- [ ] セルフホスティング後の libuv 置換戦略

---

## 参考文献

### YaoXiang 公式ドキュメント

- [RFC-024 並行性モデル](./024-concurrency-model.md)
- [RFC-008 Runtime アーキテクチャ](./008-runtime-concurrency-model.md)
- [RFC-009 所有権モデル](./009-ownership-model.md)
- [並行性モデル仕様](/reference/language-spec/concurrency.md)

### 外部参考

- [libuv 公式ドキュメント](https://docs.libuv.org/)
- [Node.js イベントループ](https://nodejs.org/en/docs/guides/event-loop-timers-and-nexttick/)
- [Rust libuv バインディング](https://github.com/libuv/libuv)

---

## ライフサイクルと帰属

| 状態         | 位置                     | 説明     |
| ------------ | ------------------------ | -------- |
| **ドラフト** | `docs/design/rfc/draft/` | 再審査中 |

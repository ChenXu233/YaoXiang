---
title: "RFC-002: libuv ベースのリソース型 IO 実装層"
status: "ドラフト"
author: "晨煦"
created: "2025-01-05"
updated: "2026-06-16（改訂: リソース型 IO 実装層として位置付け、透明な非同期を削除、RFC-024 との整合性を確保; 共有イベントループアーキテクチャ）"
---

# RFC-002: libuv ベースのリソース型 IO 実装層

> **参考**:
> - [RFC-024: spawn ブロックベースの並行モデル](./024-concurrency-model.md)
> - [RFC-008: Runtime 並行モデルとスケジューラの疎結合設計](./008-runtime-concurrency-model.md)
> - [RFC-009: 所有権モデル設計](./009-ownership-model.md)
> - [並行モデル仕様](/reference/language-spec/concurrency.md)

## 概要

本文書は YaoXiang の IO 実装層を定義する: libuv ベースでクロスプラットフォーム IO 機能を提供し、RFC-024 リソース型システムの下層実装として機能する。

**中核となる位置付け**:

```
RFC-024: リソース型の定義（FilePath, HttpUrl, DBUrl, Console）
    ↓ 使用
RFC-002: リソース型 IO 実装（libuv ベース）
    ↓ 基盤
libuv: クロスプラットフォーム IO エンジン（イベントループ + スレッドプール）
```

**何ではない**:
- ❌ 「透明な非同期」ではない——ユーザーは spawn ブロックで明示的に並行性を制御する
- ❌ 「自動的な非同期化」ではない——IO 操作は spawn ブロック内で明示的に呼び出す必要がある
- ❌ 「開発者が基盤詳細を気にする必要がない」ではない——リソース型システムが並行性安全を保証する

**何であるか**:
- ✅ リソース型（FilePath, HttpUrl, DBUrl, Console）の IO 実装層
- ✅ クロスプラットフォーム IO の統一（libuv が Windows/Linux/macOS の差異を処理）
- ✅ 共有イベントループアーキテクチャ（一つの libuv イベントループがすべての IO を処理）
- ✅ RFC-024 リソース型システムとの統合

## 動機

### なぜ libuv が必要か？

RFC-024 はリソース型システムを定義している:
- `FilePath` - ファイルシステムパス
- `HttpUrl` - HTTP エンドポイント
- `DBUrl` - データベース接続
- `Console` - 標準出力

これらのリソース型には下層 IO 実装が必要である。libuv は次を提供:

| 必要要件 | libuv の提供 |
|------|-----------|
| クロスプラットフォーム IO | Windows/Linux/macOS の統一 API |
| 非同期能力 | 共有イベントループ、すべての worker の IO を集中処理 |
| スレッドプール | ブロッキング操作専用のスレッドプール |
| 並行性安全 | シングルスレッドイベントループ、本質的に競合なし |

### RFC-024 との関係

```
┌─────────────────────────────────────────────────────────┐
│  RFC-024: 並行モデル                                     │
│  - spawn {} ブロック（明示的並行性）                       │
│  - リソース型の定義（FilePath, HttpUrl, DBUrl, Console）   │
│  - リソース競合検出（同一パスの自動直列化）                 │
└─────────────────────────────────────────────────────────┘
                          ↓ 使用
┌─────────────────────────────────────────────────────────┐
│  RFC-002: リソース型 IO 実装                             │
│  - FilePath → libuv ファイル IO                          │
│  - HttpUrl → libuv ネットワーク IO                        │
│  - DBUrl → データベースコネクションプール                  │
│  - Console → 標準出力の直列化                            │
└─────────────────────────────────────────────────────────┘
                          ↓ 基盤
┌─────────────────────────────────────────────────────────┐
│  libuv: クロスプラットフォーム IO エンジン                │
│  - イベントループ                                        │
│  - スレッドプール                                        │
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
│  │  計算タスク  │  │  計算タスク  │  │  計算タスク  │    │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘    │
│         │                │                │            │
│         └────────────────┼────────────────┘            │
│                          ↓                              │
│  ┌─────────────────────────────────────────────────┐  │
│  │          libuv イベントループ（専用スレッド）     │  │
│  │          すべての IO 操作を処理                   │  │
│  └─────────────────────────────────────────────────┘  │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

**主要な特徴**:
- 一つの共有 libuv イベントループ（専用スレッドで実行）
- すべての worker の IO 操作はこの共有イベントループに送信される
- シングルスレッドイベントループは本質的に競合を回避
- リソース効率が高く、各 worker のためにイベントループを作成する必要がない

#### 1.2 並行性安全機構

| libuv の特徴 | YaoXiang 対応 | 並行性安全 |
|------------|---------------|----------|
| シングルスレッドイベントループ | spawn ブロック内の順次実行 | 本質的に競合なし |
| スレッドプール隔離 | ブロッキング操作はメインスレッドをブロックしない | 共有状態なし |
| 非同期コールバック | DAG スケジューラが依存関係を管理 | 決定論的実行 |

### 2. リソース型 IO マッピング

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
    
    // libuv イベントループに送信
    // libuv が非同期にファイルを読み取る
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
    
    // libuv イベントループに送信
    // libuv が非同期に HTTP リクエストを実行
    // 結果を返す
    ctx.uv_loop.http_get(url)
}
```

#### 2.3 DBUrl → データベースコネクションプール

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
    
    // libuv スレッドプールに送信
    // データベースクエリはスレッドプールで実行される
    // 完了後、メインスレッドにコールバックで通知
    ctx.uv_loop.db_query(url, sql)
}
```

#### 2.4 Console → 標準出力の直列化

```rust
// Console 操作は自動直列化（RFC-024 リソース型ルール）
// すべての Console 操作は同じスレッド内で順次実行される
fn native_print(args: &[RuntimeValue], ctx: &mut NativeContext) -> Result<RuntimeValue, ExecutorError> {
    let output = format_args(args);
    
    // Console 操作を直列化
    // libuv tty 書き込み
    ctx.uv_loop.tty_write(output)
}
```

### 3. spawn ブロックとの統合

#### 3.1 ユーザーの視点

```yaoxiang
# リソース型定義（RFC-024）
FilePath: Resource
HttpUrl: Resource

# IO 操作（RFC-002 実装）
File.read: (FilePath) -> String
HTTP.get: (HttpUrl) -> Response

# ユーザーによる明示的並行性（RFC-024）
(a, b) = spawn {
    read_file("data.txt"),      # リソース型 FilePath、libuv ベース
    fetch("http://example.com") # リソース型 HttpUrl、libuv ベース
}
# コンパイラ: FilePath と HttpUrl は競合しないため、並列実行可能
```

#### 3.2 コンパイル時分析

```
コンパイラが spawn ブロックを分析:
1. リソース型操作を識別
2. リソース競合を検出（同一パス/同一 URL は自動直列化）
3. DAG 実行計画を生成
4. IO ノードにマーク（libuv に送信）
```

#### 3.3 ランタイム実行

```
ランタイムが spawn ブロックを実行:
1. Worker 0 が IO タスクを送信 → 共有イベントループ
2. Worker 1 が IO タスクを送信 → 共有イベントループ
3. イベントループがすべての IO 操作を統一処理
4. IO 完了時に対応する Worker に通知
5. Worker が後続タスクの実行を継続
```

### 4. Runtime 三層アーキテクチャと libuv

| 階層 | libuv 使用 | 非同期能力 | 適用シーン |
|------|-----------|----------|----------|
| Embedded Runtime | libuv なし | 非同期なし | WASM、ゲームスクリプト |
| Standard Runtime | 共有イベントループ | IO 非同期 | Web サービス、データパイプライン |
| Full Runtime | 共有イベントループ | IO 非同期 + 並列 | 科学計算、大規模並列 |

**Embedded Runtime**: libuv なし、即時実行、非同期能力なし。

**Standard Runtime**: 共有 libuv イベントループ、すべての IO 操作が非同期処理。

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
├── io.rs          # FilePath IO（libuv ベース）
├── net.rs         # HttpUrl IO（libuv ベース）
├── db.rs          # DBUrl IO（libuv ベース）
├── console.rs     # Console IO（libuv ベース）
└── mod.rs         # モジュール登録
```

### 3. DAG スケジューラとの統合

```rust
// IO ノードインターフェース（RFC-008 定義）
trait IoScheduler {
    // IO タスクを送信し、ハンドルを返す
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
        // DAG スケジューラに通知して下流ノードを起こす
        self.dag_scheduler.wake_dependents(handle.node_id);
    }
}
```

---

## トレードオフ

### 利点

1. **クロスプラットフォーム統一**: libuv が Windows/Linux/macOS の差異を処理
2. **IO 非同期能力**: 共有イベントループがすべての IO を処理、async/await 不要
3. **並行性安全**: シングルスレッドイベントループは本質的に競合なし
4. **リソース効率**: 一つのイベントループ、メモリオーバーヘッド小
5. **RFC-024 との整合性**: リソース型システムが並行性安全を保証
6. **成熟した安定性**: libuv は Node.js による大規模検証済み

### 欠点

1. **C ライブラリ依存**: libuv C ライブラリのバインディングが必要
2. **セルフホスティング制限**: セルフホスティング後、YaoXiang ネイティブ実装への置き換えが必要な可能性
3. **WASM サポート**: 追加のアダプタ作業が必要

---

## 代替案

| 案 | 選択しない理由 |
|------|--------------|
| Rust std::io | 同期ブロッキング、spawn ブロックと協調して非同期を実現できない |
| tokio | Rust async/await 用に設計、YaoXiang の明示的並行モデルと整合しない |
| mio | 原始的な非同期プリミティブのみ提供、高度な IO 機能不足 |
| ゼロから実装 | 複雑でエラーが発生しやすい、libuv の成熟度と比較できない |

---

## 実装戦略

### フェーズ分割

1. **フェーズ 1（v0.3）**: libuv バインディング、基本ファイル IO
2. **フェーズ 2（v0.5）**: ネットワーク IO、HTTP サポート
3. **フェーズ 3（v0.7）**: データベース IO、コネクションプール
4. **フェーズ 4（v1.0）**: WASM アダプタ、性能最適化

### 依存関係

- RFC-024（並行モデル）→ 完了
- RFC-008（Runtime アーキテクチャ）→ 完了
- RFC-009（所有権モデル）→ 完了
- RFC-011（ジェネリクスシステム）→ 完了

---

## 設計決定記録

| 決定 | 決定内容 | 理由 | 日付 |
|------|------|------|------|
| IO 実装層 | libuv | クロスプラットフォーム、非同期能力、並行性安全 | 2025-01-05 |
| 位置付け | リソース型 IO 実装層 | RFC-024 リソース型システムとの統合 | 2026-06-16 |
| イベントループアーキテクチャ | 共有イベントループ | リソース効率高、重複作成回避 | 2026-06-16 |
| 並行性安全 | シングルスレッドイベントループ | 本質的に競合なし、RFC-024 と整合 | 2026-06-16 |
| 標準ライブラリ書き換え | std.io/std.net は libuv ベース | クロスプラットフォーム統一、非同期能力 | 2026-06-16 |

---

## オープン問題

- [ ] WASM 環境での libuv アダプタ方案
- [ ] データベースコネクションプールの設計
- [ ] HTTP クライアントの完全実装
- [ ] ファイルシステムイベントのクロスプラットフォーム一貫性
- [ ] ネットワーク IO のタイムアウト機構設計
- [ ] セルフホスティング後の libuv 置換戦略

---

## 参考文献

### YaoXiang 公式ドキュメント

- [RFC-024 並行モデル](./024-concurrency-model.md)
- [RFC-008 Runtime アーキテクチャ](./008-runtime-concurrency-model.md)
- [RFC-009 所有権モデル](./009-ownership-model.md)
- [並行モデル仕様](/reference/language-spec/concurrency.md)

### 外部参考

- [libuv 公式ドキュメント](https://docs.libuv.org/)
- [Node.js イベントループ](https://nodejs.org/en/docs/guides/event-loop-timers-and-nexttick/)
- [Rust libuv バインディング](https://github.com/libuv/libuv)

---

## ライフサイクルと帰趣

| 状態 | 位置 | 説明 |
|------|------|------|
| **ドラフト** | `docs/design/rfc/draft/` | 再審査中 |
```
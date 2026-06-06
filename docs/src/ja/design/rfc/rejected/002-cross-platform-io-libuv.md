```markdown
---
title: "RFC-002：クロスプラットフォームI/Oとlibuv統合"
status: "却下"
author: "晨煦"
created: "2025-01-05"
updated: "2026-02-15"
---

# RFC-002：クロスプラットフォームI/Oとlibuv統合

## 却下理由

本RFCは以下に示す理由により却下されました：

### 1. libuvはCライブラリであり、ブートストラップ後のYaoXiangに使用できない

YaoXiangは最終的にはブートストラップ（YaoXiang自体でインタープリタータを実装する）を行う必要があり、この際にCライブラリに依存することはできません。
libuvはCライブラリであり、FFI呼び出しが必要なため、ブートストラップの足を引っ張ることになります。

### 2. tokioの方が適切な選択である

Rustエコシステムにおいてtokioは主流の非同期ランタイム（市場シェア90%以上）であり、純粋なRust実装であるため、ブートストラップ後もバインディングを継続して使用でき、libuvよりも長期的なアーキテクチャに適しています。

### 3. 実用的な考慮事項

現在の段階では、YaoXiang言語を動作させることが最優先であり、I/Oの実装にはRust標準ライブラリを使用してすぐに実装でき、実際の非同期ランタイムはブートストラップ後にtokioバインディングまたは自主開発によって実現可能です。

---

## 要約

YaoXiangのクロスプラットフォーム非同期I/O方案を提案し、libuvを統合して統一的な非同期抽象化を実現します。コアとなる設計目標は、ブロックI/O操作を自動的に透過的に非同期化させ、開発者が下位層の詳細を気にする必要がないようにすることです。

## 動機

### なぜlibuvが必要なのか？

YaoXiangの並作モデルは効率的な非同期I/Oサポートを必要とします：

| 必要性 | 従来の方案の問題点 |
|------|------------------------------------------------ |
| クロスプラットフォームI/O | 各プラットフォームのAPIが統一されていない（Windows IOCP、Linux epoll、macOS kqueue） |
| 非同期イベントループ | ゼロからの実装は複雑で間違いやすい |
| スレッドプール管理 | ブロック操作には専用のスレッドプールが必要 |
| 性能要件 | ゼロオーバーヘッドの非同期抽象化が必要 |

### libuvの利点

```
libuv ✓ 成熟・安定 - Node.jsの下位ランタイムで、大規模な検証済み
libuv ✓ クロスプラットフォーム - Windows、Linux、macOSのI/O APIを統一
libuv ✓ 高性能 - イベント駆動型、ブロッキングなしI/O
libuv ✓ スレッドプール - 内蔵のブロック操作スレッドプール管理
```

## 提案

### 1. 技術選択の決定

| コンポーネント | 選択 | 理由 |
|------|------|------|
| I/Oランタイム | libuv | クロスプラットフォームで成熟、Node.jsで検証済み |
| イベントループ | libuv loop | 軽量で効率的 |
| スレッドプール | libuv + カスタム | ブロック操作用に専用 |
| スケジューリングアルゴリズム | ワークスティーリング + DAG最適化 | 高性能、負荷分散 |
| メモリ管理 | 所有権 + スタック割り当て | GCなし、ゼロコスト抽象化 |

### 2. アーキテクチャ設計

#### 2.1 ランタイムの全体構造

```
┌─────────────────────────────────────────────────────────────┐
│                    YaoXiang Runtime                         │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              WorkStealingScheduler                  │   │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐              │   │
│  │  │Worker 0 │ │Worker 1 │ │Worker 2 │ ...         │   │
│  │  │   DAG   │ │   DAG   │ │   DAG   │              │   │
│  │  │ Executor│ │ Executor│ │ Executor│              │   │
│  │  └────┬────┘ └────┬────┘ └────┬────┘              │   │
│  └───────┼───────────┼───────────┼────────────────────┘   │
│          │           │           │                        │
│          └───────────┴───────────┘                        │
│                      │                                    │
│          ┌───────────▼───────────┐                        │
│          │   libuv Event Loop    │                        │
│          │   （クロスプラットフォームI/O抽象）    │                        │
│          └───────────┬───────────┘                        │
│                      │                                    │
│          ┌───────────▼───────────┐                        │
│          │   Thread Pool         │  ← ブロック操作用       │
│          │   (libuv threads)     │                        │
│          └───────────────────────┘                        │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

#### 2.2 ランタイム構造の定義

```rust
struct YaoXiangRuntime {
    // libuvイベントループ（クロスプラットフォームI/Oコア）
    uv_loop: *mut uv_loop_t,

    // ワークスティーリングスケジューラ
    scheduler: WorkStealingScheduler,

    // ブロック操作用スレッドプール
    io_thread_pool: ThreadPool,

    // タスクキュー
    task_queues: Vec<Deque<Task>>,

    // 統計情報
    stats: RuntimeStats,
}

struct WorkStealingScheduler {
    workers: Vec<WorkerThread>,
    global_queue: ConcurrentDeque<Task>,
    victim_steal_attempts: AtomicUsize,
}

struct ThreadPool {
    size: usize,
    sender: Sender<Task>,
    receiver: Receiver<Task>,
}
```

### 3. 統一的な非同期抽象化

#### 3.1 ブロックから透過への変換

```
┌─────────────────────────────────────────────────────────────┐
│  ブロックC関数  →  自動ラップ  →  透過的なAsync[T]          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  // 元のブロックAPI                                          │
│  data = File.read("file.txt")  // ブロック呼び出し           │
│                                                             │
│  // YaoXiang自動変換                                        │
│  // 1. ブロック呼び出しを検出                               │
│  // 2. スレッドプールに自動サブミット                       │
│  // 3. Async[T] プロキシを返す                               │
│  // 4. 使用時に結果を自動待機                               │
│                                                             │
│  // 開発者の視点                                             │
│  content = File.read("config.yaml")  // Async[String]       │
│  data = parse(content)               // 自動待機            │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

#### 3.2 I/O操作の例

```yaoxiang
# 非同期ファイル読み込み（開発者の視点：同期構文、自动非同期化）
read_config: (String) -> Config spawn = (path) => {
    content = File.read(path)  # 自動非同期化
    config = parse_yaml(content)
    config
}

# 非同期ネットワークリクエスト
fetch_user: (Int) -> User spawn = (user_id) => {
    response = HTTP.get("/users/" + user_id.to_string())
    parse_user(response.body())
}

# 並列ファイル処理
process_files: ([String]) -> [Result[FileData, Error]] = (paths) => {
    # すべてのファイルを自動的に並列読み込み
    data = paths.map(path => {
        File.read(path)  # spawnが自動挿入
    })
    data.map(d => process_content(d))
}

# ストリーム処理（段階的読み込み）
stream_large_file: (String) -> Void = (path) => {
    stream = File.open_stream(path)
    for chunk in stream.chunks(8192) {  # 自動非同期反復
        process(chunk)
    }
}
```

#### 3.3 ネットワークI/O

```yaoxiang
# HTTPサーバ
router: (HTTPRequest) -> HTTPResponse = (req) => {
    match req.path {
        "/" => home_page()
        "/api/users" => list_users()
        "/api/posts" => list_posts()
        _ => not_found()
    }
}

start_server: (Int) -> Void spawn = (port) => {
    server = HTTP.Server.new(port)
    server.serve(router)  # 自動的に同時リクエストを処理
}

# WebSocket
chat_server: (String) -> Void spawn = (port) => {
    ws = WebSocket.new("ws://localhost:" + port.to_string())
    for message in ws.incoming() {  # 自動ストリーム処理
        broadcast(message)
    }
}
```

### 4. クロスプラットフォーム保証

#### 4.1 プラットフォームサポートマトリクス

| プラットフォーム | ステータス | イベント機構 | 備考 |
| ----------- | ----- | ------ | --------- |
| **Linux**   | ✅ サポート  | epoll  | メイン開発プラットフォーム    |
| **macOS**   | ✅ サポート  | kqueue | libuvネイティブサポート |
| **Windows** | ✅ サポート  | IOCP   | libuvネイティブサポート |
| **WASM**    | ⚠️ 未定 | ブラウザAPI | 追加適応が必要    |
| **WASI**    | ⚠️ 未定 | WASI呼び出し | 長期目標      |

#### 4.2 クロスプラットフォームAPIの統一

```yaoxiang
# ファイルI/O - 統一API
file_api: () -> Void = () => {
    # すべてのプラットフォームで同じAPI
    content = File.read("data.txt")      # 読み込み
    File.write("output.txt", content)    # 書き込み
    exists = File.exists("data.txt")     # 確認
    File.delete("temp.txt")              # 削除
}

# ネットワークI/O - 統一API
network_api: () -> Void = () => {
    socket = Net.Socket.new(Net.IP.v4(127, 0, 0, 1), 8080)
    socket.connect()
    socket.send("Hello")
    response = socket.recv()
    socket.close()
}

# プロセスI/O - 統一API
process_api: () -> Void = () => {
    output = Process.run("ls", ["-la"])  # クロスプラットフォーム実行
    print(output.stdout)
}
```

#### 4.3 プラットフォーム固有の最適化

```yaoxiang
# Windows固有の最適化
when os() == "windows" {
    use_windows_registry()
}

# Linux固有の最適化
when os() == "linux" {
    use_inotify()
}

# macOS固有の最適化
when os() == "macos" {
    use_fsevents()
}
```

### 5. 性能に関する考慮事項

#### 5.1 スレッドプール設定

```yaoxiang
# スクリプトヘッダでスレッドプールサイズを設定
# @thread_pool: 4

# または実行時設定
configure_runtime: () -> Void = () => {
    Runtime.set_thread_pool_size(8)
    Runtime.set_max_concurrent_tasks(100)
}
```

#### 5.2 I/Oバッチ最適化

```yaoxiang
# バッチファイル操作（システムコールの削減）
batch_read: ([String]) -> [String] = (paths) => {
    # libuvバッチサブミットでコンテキストスイッチを削減
    File.batch_read(paths)
}

# ゼロコピー最適化
zero_copy_transfer: (String, String) -> Void = (src, dst) => {
    # サポートされているプラットフォームでsendfile/spliceを使用
    File.transfer(src, dst)
}
```

## 詳細な設計

### 1. Rustバインディング構造

```rust
// libuvバインディングモジュール
pub mod uv {
    use std::ffi::c_void;
    use std::ptr::null_mut;

    // 基本型
    pub struct UvLoop(uv_loop_t);

    // ファイル操作
    pub trait FileOps {
        fn fs_open(path: &str, flags: i32, mode: i32) -> Result<RawFd, Errno>;
        fn fs_read(fd: RawFd, buf: &mut [u8], offset: i64) -> Result<usize, Errno>;
        fn fs_write(fd: RawFd, buf: &[u8], offset: i64) -> Result<usize, Errno>;
        fn fs_close(fd: RawFd) -> Result<(), Errno>;
    }

    // ネットワーク操作
    pub trait NetOps {
        fn tcp_new() -> Result<RawTcpSocket, Errno>;
        fn tcp_connect(socket: RawTcpSocket, addr: &SocketAddr) -> Result<(), Errno>;
        fn tcp_read(socket: RawTcpSocket, buf: &mut [u8]) -> Result<usize, Errno>;
        fn tcp_write(socket: RawTcpSocket, buf: &[u8]) -> Result<usize, Errno>;
    }

    // スレッドプール
    pub struct ThreadPool {
        size: usize,
        queue: Channel<Task>,
    }
}
```

### 2. スケジューラ設計

```rust
// ワークスティーリングスケジューラ
pub struct WorkStealingScheduler {
    workers: Vec<Worker>,
    global_queue: ConcurrentDeque<Task>,
    victim_queue: AtomicUsize,
}

impl WorkStealingScheduler {
    pub fn schedule(&self, task: Task) {
        // ローカルキューを優先的に使用
        if let Ok(worker) = self.current_worker() {
            worker.local_queue.push_back(task);
        } else {
            // ワーカーがない場合はグローバルキューに追加
            self.global_queue.push_back(task);
        }
    }

    pub fn steal(&self, victim: &Worker) -> Option<Task> {
        // 他のワーカーのキューからタスクをスティール
        victim.local_queue.pop_back()
    }
}
```

### 3. 非同期タスクのライフサイクル

```
┌─────────────────────────────────────────────────────────────┐
│  Task Lifecycle                                             │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────┐   ┌─────────────┐   ┌─────────┐              │
│  │ Created │ → │ Scheduled   │ → │ Running │              │
│  └─────────┘   └─────────────┘   └────┬────┘              │
│                                       │                    │
│                      ┌────────────────┴────────────────┐   │
│                      ▼                                 ▼   │
│               ┌───────────┐                    ┌───────────┐│
│               │ Completed │                    │  Failed   ││
│               └───────────┘                    └───────────┘│
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 4. エラー処理の統合

```rust
// I/Oエラーの伝播
#[derive(Debug)]
pub enum IoError {
    FileNotFound(String),
    PermissionDenied(String),
    IoErrno(i32, String),
    Cancelled,
}

impl From<uv::UvError> for IoError {
    fn from(err: uv::UvError) -> Self {
        match err.code() {
            uv::ENOENT => IoError::FileNotFound(err.path()),
            uv::EACCES => IoError::PermissionDenied(err.path()),
            _ => IoError::IoErrno(err.code(), err.message()),
        }
    }
}
```

## トレードオフ

### 利点

1. **クロスプラットフォームの一貫性**：同一のAPIで主要なプラットフォームをすべてカバー
2. **高性能**：イベント駆動+ワークスティーリングで、手書きの非同期性能に近い
3. **透過的な非同期**：開発者が非同期の詳細を手動で処理する必要がない
4. **ブロック安全**：ブロック操作は自動的にスレッドプールに入り、イベントループをブロックしない
5. **成熟・安定**：libuvはNode.jsで大規模に検証済み

### 欠点

1. **依存関係の導入**：libuv Cライブラリのバインディングが必要
2. **Windows互換性**：一部のAPIはWindowsでの動作がわずかに異なる
3. **WASMサポート**：追加の適応作業が必要
4. **デバッグの困難さ**：非同期スタックトレースが不完全な場合がある

## 代替方案

| 方案 | なぜ選択しないのか |
|------|--------------|
| ゼロからのイベントループ実装 | 複雑で間違いやすい、libuvの成熟度には敵わない |
| mioの使用 | 生の非同期プリミティブのみを提供し、スレッドプールがない |
| async-std/tokioの使用 | Rustエコシステムだが、YaoXiangは独自のランタイムが必要 |
| libc epollの直接使用 | クロスプラットフォーム不可 |

## 実装策略

### 段階的区分

1. **段階1 (v0.1)**: 基本的なlibuvバインディング、シンプルなファイルI/O
2. **段階2 (v0.3)**: ネットワークI/O、スレッドプール統合
3. **段階3 (v0.5)**: 上級機能、ストリームAPI
4. **段階4 (v1.0)**: WASM適応、性能最適化

### 依存関係

- 外部RFCへの依存なし
- **RFC-001並作モデル**：DAGスケジューラを定義し、RFC-002はI/O抽象化を提供

## RFC-001並作モデルとの統合

RFC-001は**DAGスケジューラ**（スケジューリング層）を定義し、RFC-002は**libuv + スレッドプール**（I/O層）を定義しています。両者が連携して「同期構文、自动並作」を実現します。

### 分層アーキテクチャ

```
┌─────────────────────────────────────────────────────────────┐
│                    YaoXiang Runtime                         │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────────┐    ┌─────────────────────┐        │
│  │   RFC-001: DAG      │    │  RFC-002: libuv     │        │
│  │   スケジューリング層 │    │  I/O 層             │        │
│  │                     │    │                     │        │
│  │  • トポロジカルソート│    │  • クロスプラットフォーム I/O       │        │
│  │    スケジューリング │    │  • イベントループ    │        │
│  │  • ワークスティーリング│   │  • スレッドプール   │        │
│  │  • 依存性分析        │    │                     │        │
│  └──────────┬──────────┘    └──────────┬──────────┘        │
│             │                         │                    │
│             │     ┌───────────────────┘                    │
│             │     │                                         │
│             ▼     ▼                                         │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Runtime インターフェース層              │   │
│  │  • spawn/suspend/resume プロトコル                   │   │
│  │  • IO 完了コールバック                               │   │
│  │  • タスクサブミットと起床                            │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### 協調フロー

```markdown
1. **コンパイル時**：リソース型の操作がI/Oノードとして識別される
   - File.read、HTTP.getなどが「非同期実行が必要」とマークされる
   - DAGノードが作成され、I/O型としてマークされる

2. **実行時**：DAGスケジューラがI/Oノードに遭遇
   - 非計算ノードとして識別し、libuvにサブミット
   - スケジューラは他の実行可能ノード，继续執行

3. **I/O完了**：libuvコールバックがトリガー
   - libuvスレッドプールがブロック操作を完了
   - 完了コールバックがDAGスケジューラに通知
   - 下流ノードが実行可能になる
```

### インターフェースプロトコル

```rust
// RFC-001で定義されたI/Oノードインターフェース
trait IoScheduler {
    // I/Oタスクをサブミットし、future/handleを返す
    fn submit_io(&self, task: IoTask) -> IoHandle;

    // I/O完了時にlibuvによって呼び出され、DAGノード起床
    fn on_io_complete(&self, handle: IoHandle);
}

// RFC-002で実装されたlibuv統合
impl IoScheduler for LibUvRuntime {
    fn submit_io(&self, task: IoTask) -> IoHandle {
        // 1. タスクをlibuvスレッドプールにサブミット
        let handle = self.thread_pool.submit(|| {
            // 実際のI/Oをブロック実行
            let result = perform_blocking_io(&task);
            // 2. I/O完了、コールバック呼び出し
            self.on_io_complete(handle);
        });
        handle
    }

    fn on_io_complete(&self, handle: IoHandle) {
        // DAGスケジューラに下流ノード起床を通知
        self.dag_scheduler.wake_dependents(handle.node_id);
    }
}
```

### 透過的非同期メカニズム

#### コンパイル時処理

```yaoxiang
# ユーザーコード（同期構文）
read_config: String -> Config = (path) => {
    content = File.read(path)  # リソース操作
    parse_yaml(content)
}

# コンパイル時自動変換
# 1. File.readをリソース型操作として識別
# 2. DAGノードを作成し、I/O型としてマーク
# 3. 暗黙的なawaitポイントを追加
```

#### 実行時処理

```markdown
| ステップ | 操作 | 説明 |
|------|------|------|
| 1 | DAG解析 | I/Oノードを発見 |
| 2 | I/Oサブミット | タスクをlibuvスレッドプールに追加 |
| 3 | スケジューリング継続 | 他の実行可能ノードを実行 |
| 4 | I/O完了 | libuvコールバックがトリガー |
| 5 | 下流起床 | DAGスケジューラが待機中のノードを再開 |
```

### リソース型とI/O操作のマッピング

```yaoxiang
# RFC-001定義：リソース型
FilePath: Resource
HttpUrl: Resource

# RFC-002実装：リソース操作のI/Oセマンティクス
File.read: (FilePath) -> String = path => {
    # I/O操作としてマーク、libuvスレッドプールに自動投入
}

HTTP.get: (HttpUrl) -> Response = url => {
    # I/O操作としてマーク、libuv非同期ネットワークAPIを使用
}
```

**処理ルール**：
- リソース型引数の操作 → I/Oノードとしてマーク
- I/Oノードはlibuvスレッドプールで実行
- 完了コールバックがDAG下流ノードを起床

### リスク

1. **libuvバインディングの完全性**：完全なバインディングには大量の変更が必要
2. **Windows互換性**：一部のAPIは特別な処理が必要
3. **性能オーバーヘッド**：FFI呼び出しにはある程度のオーバーヘッドがある
4. **統合の複雑さ**：libuvスレッドプールとDAGスケジューラの調整には慎重な設計が必要

## 開放問題

- [ ] WASM環境下でのイベントループ適応方案
- [ ] ファイルシステムイベントのクロスプラットフォーム一貫性
- [ ] ネットワークI/Oのタイムアウトメカニズム設計
- [ ] ゼロコピー最適化の境界
- [ ] キャンセル操作のセマンティクス設計
- [ ] libuvスレッドプールサイズの動的調整戦略
- [ ] I/Oノード優先度と計算ノード優先度の調整

## 参考文献

- [libuv公式ドキュメント](https://docs.libuv.org/)
- [Node.jsイベントループ](https://nodejs.org/en/docs/guides/event-loop-timers-and-nexttick/)
- [ワークスティーリング論文](https://ieftimov.com/posts/understanding-stealing-queues/)
- [Rust非同期ランタイム設計](https://smallcultfollowing.com/babysteps/blog/2019/08/22/async-await-simplified/)
```
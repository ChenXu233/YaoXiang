# RFC-002：クロスプラットフォーム I/O と libuv 統合

> **ステータス**: 却下
> **著者**: 晨煦
> **作成日**: 2025-01-05
> **最終更新**: 2026-02-15

## 却下理由

本 RFC は以下の理由により却下されました：

### 1. libuv は C ライブラリであり、自己ブートstrap 後の YaoXiang では使用できない

YaoXiang は最終的に自己ブートstrap（YaoXiang 自前の実装でインタープリターを実現）を行う必要があり、その際に C ライブラリに依存することはできません。
libuv は C ライブラリであり FFI 呼び出しが必要となるため、自己ブートstrap のプロセスを阻害します。

### 2. tokio の方が適切な選択である

Rust エコシステムにおいて tokio は主要な非同期ランタイムであり（市場シェア >90%）、Pure Rust 実装であるため、自己ブートstrap 後もバインディング経由で引き続き使用できます。libuv よりも長期的なアーキテクチャに適しています。

### 3. 実用的考量

現在の段階では YaoXiang 言語をまずは動作させることが最優先であり、I/O 実装は Rust std で迅速に実現可能です。真の非同期ランタイムは自己ブートstrap 後に tokio バインディングを使用するか、独自の開発を行うべきです。

---

## 要約

YaoXiang のクロスプラットフォーム非同期 I/O 方案を提案し、libuv を統合して統合的な非同期抽象化を実現します。コアとなる設計目標は、ブロッキング I/O 操作を自動的かつ透過的に非同期化することで、開発者が下位詳細を気にする必要がないことです。

## 動機

### なぜ libuv が必要か？

YaoXiang の並作モデルには効率的な非同期 I/O サポートが必要です：

| 必要性     | 従来方案の問題                                          |
| -------- | -------------------------------------------------- |
| クロスプラットフォーム I/O | 各プラットフォームの API が統一されていない（Windows IOCP、Linux epoll、macOS kqueue） |
| 非同期イベントループ | ゼロからの実装は複雑で間違いやすい                           |
| スレッドプール管理  | ブロッキング操作には専用のスレッドプールが必要                    |
| パフォーマンス要件   | ゼロオーバーヘッドの非同期抽象化が必要                          |

### libuv の優位性

```
libuv ✓ 成熟・安定 - Node.js の下位ランタイム、大規模検証済み
libuv ✓ クロスプラットフォーム - Windows、Linux、macOS の I/O API を統一
libuv ✓ 高パフォーマンス - イベント駆動、ブロッキングなし I/O
libuv ✓ スレッドプール - 内蔵のブロッキング操作スレッドプール管理
```

## 提案

### 1. 技術選定の決定

| コンポーネント | 選定 | 理由 |
|------|------|------|
| I/O ランタイム | libuv | クロスプラットフォーム成熟、Node.js 検証済み |
| イベントループ | libuv loop |軽量級、高効率 |
| スレッドプール | libuv + カスタム | ブロッキング操作専用 |
| スケジューリングアルゴリズム | ワークスティーリング + DAG 最適化 | 高パフォーマンス、負荷分散 |
| メモリ管理 | 所有権 + スタック割り当て | GC なし、ゼロコスト抽象化 |

### 2. アーキテクチャ設計

#### 2.1 ランタイム全体構造

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
│          │   (クロスプラットフォーム I/O 抽象)  │                        │
│          └───────────┬───────────┘                        │
│                      │                                    │
│          ┌───────────▼───────────┐                        │
│          │   Thread Pool         │  ← ブロッキング操作専用        │
│          │   (libuv threads)     │                        │
│          └───────────────────────┘                        │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

#### 2.2 ランタイム構造定義

```rust
struct YaoXiangRuntime {
    // libuv イベントループ（クロスプラットフォーム I/O コア）
    uv_loop: *mut uv_loop_t,

    // ワークスティーリングスケジューラー
    scheduler: WorkStealingScheduler,

    // ブロッキング操作スレッドプール
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

### 3. 統合非同期抽象化

#### 3.1 ブロッキングから透明への変換

```
┌─────────────────────────────────────────────────────────────┐
│  ブロッキング C 関数  →  自動ラップ  →  透過的 Async[T]                    │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  // 生のブロッキング API                                             │
│  data = File.read("file.txt")  // ブロッキング呼び出し                  │
│                                                             │
│  // YaoXiang 自動変換                                        │
│  // 1. ブロッキング呼び出しを検出                                       │
│  // 2. スレッドプールに自動サブミット                                     │
│  // 3. Async[T] プロキシを返す                                   │
│  // 4. 使用時に結果を自動待機                                       │
│                                                             │
│  // 開発者視点                                              │
│  content = File.read("config.yaml")  // Async[String]       │
│  data = parse(content)               // 自動待機            │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

#### 3.2 I/O 操作示例

```yaoxiang
# 非同期ファイル読み込み（開発者視点：同期構文、自動非同期）
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

# 並行ファイル処理
process_files: ([String]) -> [Result[FileData, Error]] = (paths) => {
    # すべてのファイルを自動並列読み込み
    data = paths.map(path => {
        File.read(path)  # spawn が自動挿入
    })
    data.map(d => process_content(d))
}

# ストリーミング処理（段階的読み込み）
stream_large_file: (String) -> Void = (path) => {
    stream = File.open_stream(path)
    for chunk in stream.chunks(8192) {  # 自動非同期反復
        process(chunk)
    }
}
```

#### 3.3 ネットワーク I/O

```yaoxiang
# HTTP サーバー
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
    server.serve(router)  # 自動並行リクエスト処理
}

# WebSocket
chat_server: (String) -> Void spawn = (port) => {
    ws = WebSocket.new("ws://localhost:" + port.to_string())
    for message in ws.incoming() {  # 自動ストリーミング処理
        broadcast(message)
    }
}
```

### 4. クロスプラットフォーム保証

#### 4.1 プラットフォームサポートマトリックス

| プラットフォーム      | ステータス    | イベント機構   | 備考        |
| ------------- | ----- | ------ | --------- |
| **Linux**   | ✅ サポート  | epoll  | メイン開発プラットフォーム    |
| **macOS**   | ✅ サポート  | kqueue | libuv ネイティブサポート |
| **Windows** | ✅ サポート  | IOCP   | libuv ネイティブサポート |
| **WASM**    | ⚠️ 要確認 | ブラウザ API | 追加適応が必要    |
| **WASI**    | ⚠️ 要確認 | WASI 呼び出し | 長期目標      |

#### 4.2 クロスプラットフォーム API 統一

```yaoxiang
# ファイル I/O - 統一 API
file_api: () -> Void = () => {
    # 全プラットフォームで同一 API
    content = File.read("data.txt")      # 読み込み
    File.write("output.txt", content)    # 書き込み
    exists = File.exists("data.txt")     # 存在確認
    File.delete("temp.txt")              # 削除
}

# ネットワーク I/O - 統一 API
network_api: () -> Void = () => {
    socket = Net.Socket.new(Net.IP.v4(127, 0, 0, 1), 8080)
    socket.connect()
    socket.send("Hello")
    response = socket.recv()
    socket.close()
}

# プロセス I/O - 統一 API
process_api: () -> Void = () => {
    output = Process.run("ls", ["-la"])  # クロスプラットフォーム実行
    print(output.stdout)
}
```

#### 4.3 プラットフォーム固有最適化

```yaoxiang
# Windows 固有最適化
when os() == "windows" {
    use_windows_registry()
}

# Linux 固有最適化
when os() == "linux" {
    use_inotify()
}

# macOS 固有最適化
when os() == "macos" {
    use_fsevents()
}
```

### 5. パフォーマンス考量

#### 5.1 スレッドプール設定

```yaoxiang
# スクリプトヘッダー設定スレッドプールサイズ
# @thread_pool: 4

# またはランタイム設定
configure_runtime: () -> Void = () => {
    Runtime.set_thread_pool_size(8)
    Runtime.set_max_concurrent_tasks(100)
}
```

#### 5.2 I/O バッチ最適化

```yaoxiang
# バッチファイル操作（システムコール削減）
batch_read: ([String]) -> [String] = (paths) => {
    # libuv バッチサブミット、コンテキストスイッチ削減
    File.batch_read(paths)
}

# ゼロコピー最適化
zero_copy_transfer: (String, String) -> Void = (src, dst) => {
    # サポートプラットフォームで sendfile/splice を使用
    File.transfer(src, dst)
}
```

## 詳細設計

### 1. Rust バインディング構造

```rust
// libuv バインディングモジュール
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

### 2. スケジューラー設計

```rust
// ワークスティーリングスケジューラー
pub struct WorkStealingScheduler {
    workers: Vec<Worker>,
    global_queue: ConcurrentDeque<Task>,
    victim_queue: AtomicUsize,
}

impl WorkStealingScheduler {
    pub fn schedule(&self, task: Task) {
        // ローカルキューへの追加を優先
        if let Ok(worker) = self.current_worker() {
            worker.local_queue.push_back(task);
        } else {
            // ワーカーがない場合はグローバルキューに追加
            self.global_queue.push_back(task);
        }
    }

    pub fn steal(&self, victim: &Worker) -> Option<Task> {
        // 他のワーカのキューからタスクをスティール
        victim.local_queue.pop_back()
    }
}
```

### 3. 非同期タスクライフサイクル

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

### 4. エラー処理統合

```rust
// I/O エラーの伝播
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

### メリット

1. **クロスプラットフォームの一貫性**：同一 API で全メジャープラットフォームをカバー
2. **高パフォーマンス**：イベント駆動＋ワークスティーリング、手書き非同期に近い性能
3. **透過的な非同期化**：開発者が非同期詳細を手動処理する必要がない
4. **ブロッキング安全**：ブロッキング操作は自動的でスレッドプールに入り、イベントループをブロックしない
5. **成熟・安定**：libuv は Node.js で大規模検証済み

### デメリット

1. **依存関係の導入**：libuv C ライブラリのバインディングが必要
2. **Windows 互換性**：特定の API は Windows での動作がやや異なる
3. **WASM サポート**：追加適応作業が必要
4. **デバッグ困難**：非同期スタックトレースが不完全な可能性がある

## 代替案

| 方案 | 選択しない理由 |
|------|--------------|
| ゼロからのイベントループ実装 | 複雑で間違いやすく、libuv の成熟度にはかなわない |
| mio の使用 | 生の非同期プリミティブのみ提供、スレッドプールがない |
| async-std/tokio の使用 | Rust エコシステムだが、YaoXiang は独自のランタイムが必要 |
| 直接 libc epoll の使用 | クロスプラットフォーム不可 |

## 実装戦略

### フェーズ分け

1. **フェーズ 1 (v0.1)**: 基本的 libuv バインディング、単純なファイル I/O
2. **フェーズ 2 (v0.3)**: ネットワーク I/O、スレッドプール統合
3. **フェーズ 3 (v0.5)**: 上級機能、ストリーミング API
4. **フェーズ 4 (v1.0)**: WASM 適応、パフォーマンス最適化

### 依存関係

- 外部 RFC への依存なし
- **RFC-001 並行モデル**：DAG スケジューラーを定義、RFC-002 は I/O 抽象化を提供

## RFC-001 並行モデルとの統合

RFC-001 は **DAG スケジューラー**（スケジューリング層）を定義し、RFC-002 は **libuv + スレッドプール**（I/O 層）を定義します。両者が協力し、「同期構文、自动並行」を実現します。

### レイヤー化アーキテクチャ

```
┌─────────────────────────────────────────────────────────────┐
│                    YaoXiang Runtime                         │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────────┐    ┌─────────────────────┐        │
│  │   RFC-001: DAG      │    │  RFC-002: libuv     │        │
│  │   スケジューリング層 │    │  I/O 層              │        │
│  │                     │    │                     │        │
│  │  • トポロジカルソートスケジューリング     │    │  • クロスプラットフォーム I/O       │        │
│  │  • ワークスティーリング        │    │  • イベントループ         │        │
│  │  • 依存性分析           │    │  • スレッドプール           │        │
│  └──────────┬──────────┘    └──────────┬──────────┘        │
│             │                         │                    │
│             │     ┌───────────────────┘                    │
│             │     │                                         │
│             ▼     ▼                                         │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Runtime インターフェース層                          │   │
│  │  • spawn/suspend/resume プロトコル                         │   │
│  │  • IO Completion コールバック                                │   │
│  │  • タスクサブミットと起床                                    │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### 協調フロー

```markdown
1. **コンパイル時**：リソース型操作が I/O ノードとして識別される
   - File.read, HTTP.get などが「非同期実行が必要」とマークされる
   - DAG ノードが作成され、I/O 型としてマークされる

2. **ランタイム時**：DAG スケジューラーが I/O ノードに遭遇
   - 非計算ノードとして識別し、libuv にサブミット
   - スケジューラーは他の実行可能ノードを継続実行

3. **I/O 完了時**：libuv コールバックがトリガー
   - libuv スレッドプールがブロッキング操作を完了
   - completion コールバックが DAG スケジューラーに通知
   - 下流ノードが実行可能になる
```

### インターフェースプロトコル

```rust
// RFC-001 が定義する I/O ノードインターフェース
trait IoScheduler {
    // I/O タスクをサブミットし、future/handle を返す
    fn submit_io(&self, task: IoTask) -> IoHandle;

    // I/O 完了時に libuv が呼び出し、DAG ノード起床
    fn on_io_complete(&self, handle: IoHandle);
}

// RFC-002 が実装する libuv 統合
impl IoScheduler for LibUvRuntime {
    fn submit_io(&self, task: IoTask) -> IoHandle {
        // 1. タスクを libuv スレッドプールにサブミット
        let handle = self.thread_pool.submit(|| {
            // ブロッキング実行 실제 I/O
            let result = perform_blocking_io(&task);
            // 2. I/O 完了、コールバック呼び出し
            self.on_io_complete(handle);
        });
        handle
    }

    fn on_io_complete(&self, handle: IoHandle) {
        // DAG スケジューラーに下流ノード起床を通知
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
# 1. File.read をリソース型操作として識別
# 2. DAG ノードを作成し、I/O 型としてマーク
# 3. 暗黙の await 点を追加
```

#### ランタイム時処理

```markdown
| ステップ | 操作 | 説明 |
|------|------|------|
| 1 | DAG 解析 | I/O ノードを発見 |
| 2 | I/O サブミット | タスクを libuv スレッドプールに追加 |
| 3 | スケジューリング継続 | 他の実行可能ノードを実行 |
| 4 | I/O 完了 | libuv コールバックがトリガー |
| 5 | 下流起床 | DAG スケジューラーが待機中のノードを resume |
```

### リソース型と I/O 操作のマッピング

```yaoxiang
# RFC-001 定義：リソース型
FilePath: Resource
HttpUrl: Resource

# RFC-002 実装：リソース操作の I/O セマンティクス
File.read: (FilePath) -> String = path => {
    # I/O 操作としてマーク、libuv スレッドプールに自動参加
}

HTTP.get: (HttpUrl) -> Response = url => {
    # I/O 操作としてマーク、libuv 非同期ネットワーク API を使用
}
```

**処理ルール**：

- リソース型パラメータを持つ操作 → I/O ノードとしてマーク
- I/O ノードは libuv スレッドプールで実行
- completion コールバックが DAG 下流ノード起床

### リスク

1. **libuv バインディングの完全性**：完全なバインディングには大量の準備が必要
2. **Windows 互換性**：特定の API は特殊処理が必要
3. **パフォーマンスオーバーヘッド**：FFI 呼び出しには一定のオーバーヘッドがある
4. **統合複雑度**：libuv スレッドプールと DAG スケジューラーの調整には慎重な設計が必要

## 開放的な問題

- [ ] WASM 環境でのイベントループ適応方案
- [ ] ファイルシステムイベントのクロスプラットフォーム一貫性
- [ ] ネットワーク I/O のタイムアウトメカニズム設計
- [ ] ゼロコピー最適化の境界
- [ ] キャンセル操作のセマンティクス設計
- [ ] libuv スレッドプールサイズの動的調整戦略
- [ ] I/O ノード優先度と計算ノード優先度の調整

## 参考文献

- [libuv 公式ドキュメント](https://docs.libuv.org/)
- [Node.js イベントループ](https://nodejs.org/en/docs/guides/event-loop-timers-and-nexttick/)
- [ワークスティーリング論文](https://ieftimov.com/posts/understanding-stealing-queues/)
- [Rust 非同期ランタイム設計](https://smallcultfollowing.com/babysteps/blog/2019/08/22/async-await-simplified/)
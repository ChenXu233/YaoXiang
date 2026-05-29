```markdown
---
title: RFC-002：クロスプラットフォームI/Oとlibuv統合（却下）
---

# RFC-002：クロスプラットフォームI/Oとlibuv統合

> **ステータス**: 却下
> **著者**: 晨煦
> **作成日**: 2025-01-05
> **最終更新**: 2026-02-15

## 却下理由

本RFCは、以下の理由により却下されました：

### 1. libuvはCライブラリであり、ブートストラップ後のYaoXiangには使用できない

YaoXiangは最終的にブートstrap（YaoXiang自身でインタープリタを実装）する必要があるため、Cライブラリに依存することはできません。
libuvはCライブラリであり、FFI呼び出しが必要となるため、ブートstrapプロセスを阻害します。

### 2. tokioの方が適切な選択である

Rustエコシステムにおいて、tokioは支配的な非同期ランタイム（市場占有率>90%）であり、純粋なRust実装であるため、ブートstrap後もバインディングを継続使用でき、長期的なアーキテクチャにより適しています。

### 3. 実用的な考量

現在の段階では、YaoXiang言語をまずは動作させることが最優先であり、I/O実装はRust stdで迅速に実現でき、本当の非同期ランタイムはブートstrap後にtokioバインディングや独自開発可以实现可能です。

---

## 概要

YaoXiangのクロスプラットフォーム非同期I/O方案を提案し、libuvを統合して統一的な非同期抽象化を実現します。コア設計目標は、ブロックI/O操作を自動的に透過的に非同期化し、開発者が下位詳細を気にする必要がないようにすることです。

## 動機

### なぜlibuvが必要なのか？

YaoXiangのspawnモデルには、高效な非同期I/Oサポートが必要です：

| ニーズ | 従来方案の問題点 |
|------|------------------------------------------------ |
| クロスプラットフォームI/O | 各プラットフォームのAPIが統一されていない（Windows IOCP、Linux epoll、macOS kqueue） |
| 非同期イベントループ | ゼロからの実装は複雑でバグしやすい |
| スレッドプール管理 | ブロック操作には专用スレッドプールが必要 |
| 性能要件 | ゼロオーバーヘッドの非同期抽象化が必要 |

### libuvの優位性

```
libuv ✓ 成熟安定 - Node.jsの下位ランタイムであり、大規模検証済み
libuv ✓ クロスプラットフォーム - Windows、Linux、macOSのI/O APIを統一
libuv ✓ 高性能 - イベント駆動、ブロッキングなしI/O
libuv ✓ スレッドプール - 内蔵のブロック操作スレッドプール管理
```

## 提案

### 1. 技術選択の意思決定

| コンポーネント | 選択 | 理由 |
|------|------|------|
| I/Oランタイム | libuv | クロスプラットフォーム成熟、Node.js検証済み |
| イベントループ | libuv loop | 軽量级、高效 |
| スレッドプール | libuv + カスタム | ブロック操作专用 |
| スケジューリングアルゴリズム | ワークスティaling + DAG最適化 | 高性能、ロードバランシング |
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
│          │   (クロスプラットフォームI/O抽象)      │                        │
│          └───────────┬───────────┘                        │
│                      │                                    │
│          ┌───────────▼───────────┐                        │
│          │   Thread Pool         │  ← ブロック操作専用        │
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

    // ワークスティalingスケジューラ
    scheduler: WorkStealingScheduler,

    // ブロック操作スレッドプール
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

### 3. 統一非同期抽象化

#### 3.1 ブロッキングから透過への変換

```
┌─────────────────────────────────────────────────────────────┐
│  ブロッキングC関数  →  自動ラップ  →  透過的なAsync[T]                    │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  // 元のブロッキングAPI                                             │
│  data = File.read("file.txt")  // ブロッキング呼び出し                  │
│                                                             │
│  // YaoXiang自動変換                                        │
│  // 1. ブロッキング呼び出しを検出                                       │
│  // 2. スレッドプールに自動提交                                     │
│  // 3. Async[T] プロキシを返す                                   │
│  // 4. 使用時に自動待機結果                                       │
│                                                             │
│  // 開発者の視点                                              │
│  content = File.read("config.yaml")  // Async[String]       │
│  data = parse(content)               // 自動待機            │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

#### 3.2 I/O操作の例

```yaoxiang
# 非同期ファイル読み込み（開発者の視点：同期構文、自動非同期）
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
    # すべてのファイルを自動並列読み込み
    data = paths.map(path => {
        File.read(path)  # spawnが自動挿入
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

| プラットフォーム      | ステータス  | イベント機構   | 備考        |
| ----------- | ----- | ------ | --------- |
| **Linux**   | ✅ サポート  | epoll  | 主要開発プラットフォーム    |
| **macOS**   | ✅ サポート  | kqueue | libuvネイティブサポート |
| **Windows** | ✅ サポート  | IOCP   | libuvネイティブサポート |
| **WASM**    | ⚠️ 未定 | ブラウザAPI | 追加適応が必要    |
| **WASI**    | ⚠️ 未定 | WASI呼び出し | 長期目標      |

#### 4.2 クロスプラットフォームAPI統一

```yaoxiang
# ファイルI/O - 統一API
file_api: () -> Void = () => {
    # すべてのプラットフォームで同じAPI
    content = File.read("data.txt")      # 読み込み
    File.write("output.txt", content)    # 書き込み
    exists = File.exists("data.txt")     # チェック
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

#### 4.3 プラットフォーム固有最適化

```yaoxiang
# Windows固有最適化
when os() == "windows" {
    use_windows_registry()
}

# Linux固有最適化
when os() == "linux" {
    use_inotify()
}

# macOS固有最適化
when os() == "macos" {
    use_fsevents()
}
```

### 5. 性能考量

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
    # libuvバッチ提交、コテキストスイッチ削減
    File.batch_read(paths)
}

# ゼロコピ最適化
zero_copy_transfer: (String, String) -> Void = (src, dst) => {
    # サポートされるプラットフォームでsendfile/spliceを使用
    File.transfer(src, dst)
}
```

## 詳細設計

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
// ワークスティalingスケジューラ
pub struct WorkStealingScheduler {
    workers: Vec<Worker>,
    global_queue: ConcurrentDeque<Task>,
    victim_queue: AtomicUsize,
}

impl WorkStealingScheduler {
    pub fn schedule(&self, task: Task) {
        // ローカルキューに優先的に追加
        if let Ok(worker) = self.current_worker() {
            worker.local_queue.push_back(task);
        } else {
            // ワーカーがない場合はグローバルキューに追加
            self.global_queue.push_back(task);
        }
    }

    pub fn steal(&self, victim: &Worker) -> Option<Task> {
        // 他のワーカーのキューからタスクをスティaling
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
// I/Oエラー伝播
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

1. **クロスプラットフォーム一貫性**：同一のAPIですべての主要プラットフォームをカバー
2. **高性能**：イベント駆動+ワークスティaling、手書き非同期に近い性能
3. **透過的非同期**：開発者が手動で非同期詳細を処理する必要がない
4. **ブロッキング安全**：ブロック操作は自動的でスレッドプールに入り、イベントループをブロックしない
5. **成熟安定**：libuvはNode.jsの大規模検証済み

### デメリット

1. **依存関係の導入**：libuv Cライブラリのバインディングが必要
2. **Windows互換性**：一部のAPIはWindowsでの動作がわずかに異なる
3. **WASMサポート**：追加適応作業が必要
4. **デバッグ困難**：非同期スタックトレースが不完全な場合がある

## 代替方案

| 方案 | なぜ選択しないか |
|------|--------------|
| ゼロからイベントループを実装 | libuvの成熟度と比較して複雑でバグしやすい |
| mioを使用 | 生の非同期プリミティブのみを提供し、スレッドプールがない |
| async-std/tokioを使用 | Rustエコシステムだが、YaoXiangは独自のランタイムが必要 |
| 直接libc epollを使用 | クロスプラットフォームできない |

## 実装策略

### フェーズ分け

1. **フェーズ1 (v0.1)**: 基本libuvバインディング、シンプルファイルI/O
2. **フェーズ2 (v0.3)**: ネットワークI/O、スレッドプール統合
3. **フェーズ3 (v0.5)**: 上級機能、ストリーミングAPI
4. **フェーズ4 (v1.0)**: WASM適応、性能最適化

### 依存関係

- 外部RFC依存なし
- **RFC-001 並列モデル**：DAGスケジューラを定義、RFC-002はI/O抽象化を提供

## RFC-001並列モデルとの統合

RFC-001は**DAGスケジューラ**（スケジューリング層）を定義し、RFC-002は**libuv + スレッドプール**（I/O層）を定義します。两者は協力して「同期構文、自動并行」を实现します。

### レイヤーアーキテクチャ

```
┌─────────────────────────────────────────────────────────────┐
│                    YaoXiang Runtime                         │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────────┐    ┌─────────────────────┐        │
│  │   RFC-001: DAG      │    │  RFC-002: libuv     │        │
│  │   スケジューリング層            │    │  I/O 層              │        │
│  │                     │    │                     │        │
│  │  • トポロジカルソートスケジューリング     │    │  • クロスプラットフォーム I/O       │        │
│  │  • ワークスティaling         │    │  • イベントループ         │        │
│  │  • 依存性分析         │    │  • スレッドプール           │        │
│  └──────────┬──────────┘    └──────────┬──────────┘        │
│             │                         │                    │
│             │     ┌───────────────────┘                    │
│             │     │                                         │
│             ▼     ▼                                         │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Runtime インターフェース層                          │   │
│  │  • spawn/suspend/resume プロトコル                         │   │
│  │  • IO Completion コールバック                                │   │
│  │  • タスク提交と起床                                    │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### 協業フロー

```markdown
1. **コンパイル時**：リソース型の操作がI/Oノードとして識別される
   - File.read、HTTP.getなどが「非同期実行が必要」とマークされる
   - DAGノードが作成され、I/O型としてマークされる

2. **実行時**：DAGスケジューラがI/Oノードに遭遇
   - 非計算ノードとして識別され、libuvに提交される
   - スケジューラは他の実行可能なノード，继续執行を続ける

3. **I/O完了**：libuvコールバックがトリガーされる
   - libuvスレッドプールがブロック操作を完了する
   - completionコールバックがDAGスケジューラに通知する
   - 下流ノードが実行可能になる
```

### インターフェースプロトコル

```rust
// RFC-001で定義されたI/Oノードインターフェース
trait IoScheduler {
    // I/Oタスクを提交し、future/handleを返す
    fn submit_io(&self, task: IoTask) -> IoHandle;

    // I/O完了時にlibuvが呼び出し、DAGノードを起床させる
    fn on_io_complete(&self, handle: IoHandle);
}

// RFC-002で実装されたlibuv統合
impl IoScheduler for LibUvRuntime {
    fn submit_io(&self, task: IoTask) -> IoHandle {
        // 1. タスクをlibuvスレッドプールに提交する
        let handle = self.thread_pool.submit(|| {
            // 実際のI/Oをブロッキング実行する
            let result = perform_blocking_io(&task);
            // 2. I/O完了、コールバックを呼び出す
            self.on_io_complete(handle);
        });
        handle
    }

    fn on_io_complete(&self, handle: IoHandle) {
        // DAGスケジューラに下流ノードを起床させるよう通知する
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
# 1. File.readをリソース型操作として識別する
# 2. DAGノードを作成し、I/O型としてマークする
# 3. 暗黙的なawait点を追加する
```

#### 実行時処理

```markdown
| ステップ | 操作 | 説明 |
|------|------|------|
| 1 | DAG解析 | I/Oノードを検出 |
| 2 | I/O提交 | タスクをlibuvスレッドプールに追加 |
| 3 | スケジューリング継続 | 他の実行可能なノードを実行 |
| 4 | I/O完了 | libuvコールバックがトリガー |
| 5 | 下流起床 | DAGスケジューラが待機中のノードをresume |
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
- リソース型パラメータを持つ操作 → I/Oノードとしてマーク
- I/Oノードはlibuvスレッドプールで実行される
- completionコールバックがDAG下流ノードを起床させる

### リスク

1. **libuvバインディング完全性**：完全なバインディングには大量の準備が必要
2. **Windows互換性**：一部のAPIには的特殊処理が必要
3. **性能オーバーヘッド**：FFI呼び出しにはある程度のオーバーヘッドがある
4. **統合複雑度**：libuvスレッドプールとDAGスケジューラの協調には慎重な設計が必要

## 未解決の問題

- [ ] WASM環境でのイベントループ適応方案
- [ ] ファイルシステムイベントのクロスプラットフォーム一貫性
- [ ] ネットワークI/Oのタイムアウト機構設計
- [ ] ゼロコピ最適化の境界
- [ ] キャンセル操作のセマンティクス設計
- [ ] libuvスレッドプールサイズの動的調整戦略
- [ ] I/Oノード優先度と計算ノード優先度の協調

## 参考文献

- [libuv公式ドキュメント](https://docs.libuv.org/)
- [Node.jsイベントループ](https://nodejs.org/en/docs/guides/event-loop-timers-and-nexttick/)
- [ワークスティaling論文](https://ieftimov.com/posts/understanding-stealing-queues/)
- [Rust非同期ランタイム設計](https://smallcultfollowing.com/babysteps/blog/2019/08/22/async-await-simplified/)
```
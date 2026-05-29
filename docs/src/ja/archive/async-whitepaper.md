# **《並作：遅延評価に基づく透過的非同期並行モデル》技術白書**

## 🏛️ 一、コア定義：並作モデル

**並作モデル**は、『易・復卦』の「万物並作、吾れ以って復を觀る」に由来し、プログラミング言語の並行パラダイムである。開発者が同期・順序的な思考でロジックを記述でき、言語ランタイムがその計算ユニットを万物並作のごとく自動かつ効率的に並行実行させ、最終的に統一的に協調させる。

### コア設計理念：デフォルト遅延 + spawn 型マーキング

| 設計原則 | 説明 |
|----------|------|
| **デフォルト遅延評価** | すべての関数はデフォルトで遅延（Haskell類似）、Lazy[T] を返す |
| **コア数設定** | スクリプトヘッダー宣言 `// @cores: N` で自動並列化を有効化 |
| **spawn 型マーキング** | `-> T spawn` で関数を厳密に非同期・並行可能としてマーク，其余默认可并发 |
| **混合評価モード** | `@eager`（デコレータ、強制即時）、`@auto`（デコレータ、並列維持） |
| **Void 自動即時** | Void を返す関数は自動即時評価（副作用は必ず実行） |

### コア三原則

| コア原則 | 解説 |
|----------|------|
| **同期構文** | 見た目がそのままの順序コード、書いた通りの実行フロー |
| **並行本質** | ランタイムが自動的で並列性を抽出し、データ依存関係から並行機会を掘り起こす |
| **統一協調** | 結果が必要時に自動集約され、論理的正しさを保証 |

**これは以下の二つの根本的な転換によって達成される：**

1. **「制御流」を「データ流」に転換**：プログラムは純粋関数的な遅延評価データフローグラフとしてみなされる
2. **「非同期伝染」を「依存解決」に転換**：非同期性は関数のシグネチャのエフェクトではなくなり、ランタイムがデータ依存点で自動実行する待機操作となる

---

## 📚 二、用語体系：統一された概念マップ

「並作」を中心に、明確で自己完結した用語体系を構築し、すべての設計を串联する：

| 公式用語 | 対応構文/概念 | 解説 |
|----------|---------------|------|
| **並作関数** | `-> T spawn` | 戻り型マーキング并发执行の计算ユニット |
| **並作ブロック** | `spawn { a(), b() }` | 開発者が明示的に宣言した並行領域、ブロック内のタスクが「並作」実行される |
| **並作ループ** | `spawn for x in xs { ... }` | データ並列パラダイム、ループ体がすべてのデータ要素で「並作」実行される |
| **並作値** | `Async[T]` プロキシ型 | 編集中の「未来値」、使用時に自動待機して「作」完了を待つ |
| **並作グラフ** | 遅延計算グラフ（DAG） | 「並作」が発生する舞台、すべての計算ユニット間の依存関係と並列関係を記述する |
| **並作スケジューラ** | ランタイムタスクスケジューラ | 「万物」を調整し、正しいタイミングで「並作」させる知的中枢 |
| **エラープログラム** | Error Graph | 並行環境下的错误传播路径可视化，类似调用栈但展示DAG中的错误流向 |
| **リソース競合** | Resource Conflict | 複数のタスクが同時に同一の書き込み可能リソースにアクセスする際の競合、コンパイル時検出と自動直列化 |

> **技術交流示例**："这里我们用个并作块来并发调用两个并作函数，就能自动获得它们的并作值。"

---

## 三、三層並行アーキテクチャ：漸進的透過性

### 3.1 アーキテクチャ概要

並作モデルは**三層漸進的並行抽象**を提供し、異なるスキルレベルの開発者が適切な使用パターンを見つけられる：

| レベル | モード | 構文マーキング | 実行方式 | 制御性 | 適用シナリオ |
|------|------|----------|----------|--------|----------|
| **L1** | `@blocking` 同期 | `@blocking` | 完全順序実行 | 最高 | デバッグ、初心者学習、重要なコード段 |
| **L2** | 明示的spawn | `spawn` | 開発者制御可能な並行 | 中 | 中級ユーザー、精细控制并发が必要な場合 |
| **L3** | 完全透過 | なし（デフォルト） | 自動最適並列 | 最低 | 上級者、自動並列最適化 |

### 3.2 L1: `@blocking` 同期モード

**コア特性**：すべての並列最適化を無効化、完全順序実行、デバッグと理解が容易。

```yaoxiang
# L1: @blocking 同期モード（戻り型の後に注釈を配置）
fetch_sync: (String) -> JSON @blocking = (url) => {
    HTTP.get(url).json()
}

main: () -> Void @blocking = () => {
    # 厳密に順序実行、並行なし
    data1 = fetch_sync("https://api.example.com/data1")
    data2 = fetch_sync("https://api.example.com/data2")
    process(data1, data2)
}
```

### 3.3 L2: 明示的 spawn 並列

**コア特性**：開発者が並列可能なユニットを明示的にマーク、制御可能性を保ちながら並列化の恩恵を受ける。

```yaoxiang
# L2: 明示的 spawn 並列
fetch_data: (String) -> JSON spawn = (url) => {
    HTTP.get(url).json()
}

process_users_and_posts: () -> Void spawn = () => {
    users = fetch_data("https://api.example.com/users")
    posts = fetch_data("https://api.example.com/posts")
    # users と posts が自動並列実行
    print(users.length.to_string())
    print(posts.length.to_string())
}

# 明示的並列ブロック
compute_all: () -> (Int, Int, Int) spawn = () => {
    (a, b, c) = spawn {
        heavy_calc(1),
        heavy_calc(2),
        heavy_calc(3)
    }
    (a, b, c)
}
```

### 3.4 L3: 完全透過（デフォルト）

**コア特性**：マーキング不要、コンパイラが自動分析して最適な並列実行計画を生成。

```yaoxiang
# L3: 完全透過（デフォルトモード）
heavy_calc: (Int) -> Int = (n) => {
    fibonacci(n)
}

auto_parallel: (Int) -> Int = (n) => {
    # システムが自動分析：a, b, c に依存なし→完全並列可能
    a = heavy_calc(1)
    b = heavy_calc(2)
    c = heavy_calc(3)
    a + b + c
}
```

### 3.5 手動制御注釈

| 注釈 | 動作 | 使用シナリオ |
|------|------|----------|
| `@eager` | 強制即時評価 | 結果を即座に取得する必要がある計算 |

---

## 二、コアコンセプト

### 2.1 並作グラフ：万物並作の舞台

すべてのプログラムはコンパイル時に**有向無閉路計算グラフ（DAG）**に変換され、これは**並作グラフ**と呼ばれる。

| 要素 | 説明 |
|------|------|
| **ノード** | 式計算ユニットを表現 |
| **辺** | データ依存関係を示す（A → B は B が A の結果に依存することを示す） |
| **遅延** | ノードはその出力が**実際に必要**になった时才被求值 |

### 2.2 デフォルト遅延評価

すべての関数はデフォルトで**遅延評価**戦略を採用：

```yaoxiang
# スクリプトヘッダーで並列コア数を設定
# @cores: 4

# すべての関数はデフォルトで遅延評価（默认可并发）
heavy_computation: (Int) -> Int = (x) => {
    # この関数は即座には実行されない
    # 結果が使用された时才执行
    fibonacci(x)
}

main: () -> Void = () => {
    # heavy_computation は Int を返し、型は Lazy[Int]
    result = heavy_computation(100)

    # ここで、result が加算に使用され、評価がトリガーされる
    # システムは自動的に最適なタイミングで並列実行
    total = result + heavy_computation(200)
}
```

### 2.3 混合評価注釈（デコレータスタイル）

YaoXiang の注釈は Python のデコレータ类似しており、関数や式の動作を変更するために使用：

| 注釈（デコレータ） | 動作 |
|----------------|------|
| `@eager` | **デコレータ**：强制即時評価，立即実行 |
| `@auto` | **デコレータ**：保持並列（デフォルト，省略可能） |

**Void 自動即時ルール：** Void を返す関数は（任何注釈都不要で）自动即時評価、因为副作用必须执行。

```yaoxiang
# @eager デコレータ：强制即時評価
heavy_computation: (Int) -> Int = (x) => {
    fibonacci(x)
}

# Void を返す関数は自動即時評価（副作用関数）
log: (String) -> Void = (message) => {
    print(message)
}

main: () -> Void = () => {
    # log は自動即時実行（Void を返すため）
    log("Processing started")

    # @eager を使用して强制即時
    @eager heavy_computation(100)
}
```

### 2.4 並作値：Async[T] 遅延プロキシ型

戻り型マーキングが `-> T spawn` の関数は、即座に `Async[T]` 型の値を返す。これは**並作値**と呼ばれる。

```yaoxiang
# 並作関数：戻り型マーキングが -> JSON spawn
# これは厳密に並作実行可能な計算ユニットであることを示す
fetch: (String) -> JSON spawn = (url) => {
    HTTP.get(url).json()
}

main: () -> Void = () => {
    # fetch が返すのは並作値 Async[JSON]
    # しかし、使用時に追加の構文は不要
    data = fetch("https://api.example.com")  # Async[JSON]

    # ここで、data は自動待機して JSON にアンパックされる
    print(data.name)  # 同期コード一样自然
}
```

#### 並作値のコア特性

| 特性 | 説明 |
|------|------|
| **構文透過** | `Async[T]` は型システムで `T` のサブタイプであり、`T` が期待されるあらゆるコンテキストで使用可能 |
| **需要時待機** | `T` 型の具体的な値を使用必须时（例：フィールドアクセス、算術演算）、ランタイムは自動停止して待機 |
| **エラー伝播** | 内部的には `Result<T, E>` であり、エラーはデータフローに沿って自然に伝播 |

### 2.7 並作構成：「修飾子」から「型マーキング」への転換

`spawn` キーワードは同期思考と非同期実装を接続する唯一の架け橋であり、三重のセマンティクスを持つ：

| 構文形式 | 公式用語 | セマンティクス | ランタイム動作 |
|:---------|:---------|:-----|:----------|
| **`-> T spawn`** | 並作関数 | 戻り型マーキング并发可能な計算ユニット임을示す | その呼び出しは `Async[T]` を返し、並作グラフノードの作成を意味する |
| **`spawn { ... }`** | 並作ブロック | 明示的な並行領域の宣言 | ランタイムはブロック内の各式を**積極的に**独立したタスクとして並行実行し、ブロック終了時に暗黙的にすべての結果を待機 |
| **`spawn for`** | 並作ループ | データ並列ループ | ループ体を複数の並列タスクに変換し、自动的なデータシャーディング、スケジューリング、結果収集を行う |

---

## 三、動作原理：コードから実行へ

### 3.1 コンパイル時：並作グラフの構築

```yaoxiang
# 並作関数定義：戻り型マーキングが spawn
fetch: (String) -> String spawn = (url) => { ... }
parse: (String) -> Model spawn = (data) => { ... }

process: () -> Report = () => {
    # コンパイラはここで並作グラフノードを作成
    data_a = fetch("url1")  # ノード A: Async[String]
    data_b = fetch("url2")  # ノード B: Async[String]

    # 並作ブロック：明示的な並行領域
    (model_a, model_b) = spawn {
        parse(data_a),  # ノード C: A に依存
        parse(data_b)   # ノード D: B に依存
    }

    # 最終集約ノード
    generate_report(model_a, model_b)  # ノード E
}
```

**コンパイラ動作：**
1. ソースコードを解析し、グローバルの並作グラフを構築
2. 各式に対して計算ノードを作成
3. データ依存関係を分析し、辺関係を確立
4. `spawn { }` と `spawn for` ブロック内のサブグラフには **「並列評価」** マーキングが付与

### 4.2 ランタイム：並作スケジューラ

 intelligenteで work-stealing をサポートする**並作スケジューラ**が並作グラフの実行を担当：

```rust
// 並作スケジューラコアロジック
impl FlowScheduler {
    fn execute_node(&self, node_id: NodeId) {
        let node = self.get_node(node_id);
        
        match &node.kind {
            NodeKind::AsyncCompute => {
                // 並作関数：コルーチンプールにサブミット
                self.submit_async(node_id);
            }
            NodeKind::ParallelBlock => {
                // 並作ブロック：積極的にすべての直接サブノードを並列実行
                self.submit_parallel(node_id);
            }
            NodeKind::DataParallel { iterator, body } => {
                // 並作ループ：自動シャーディング
                self.submit_data_parallel(node_id, iterator, body);
            }
            _ => { /* 同期実行 */ }
        }
    }
}
```

#### 実行フロー

```
1. [E] の評価のため、[C] と [D] が必要
2. [C] は [A] に依存、[D] は [B] に依存
3. 並作スケジューラが [A] と [B] に依存関係がないを発見 → 直ちに並列実行
4. [A]、[B] 完了後、並作ブロックマーキングにより → 直ちに [C] と [D] を並列実行
5. [C]、[D] 完了後、[E] を実行
```

**重要なメカニズム：**

| メカニズム | 説明 |
|------|------|
| **遅延トリガー** | 実行は最終結果の要求から始まり、依存関係を逆方向に追跡 |
| **自動待機** | `Async[T]` に遭遇した時自動停止、他の準備完了タスクを実行 |
| **ワークスティーリング** | スレッドが他のスレッドキューからタスクを奪い取り、CPU 利用率を向上 |

---

## 四、重要メカニズムの詳細

### 4.1 副作用と評価保証

純粋な遅延評価は副作用（例：ログ、書き込み）が永不執行につながる可能性がある。並作モデルは**戻り型に基づく自動推論**を採用：

| ルール | 条件 | 動作 |
|------|------|------|
| **ルール1** | Void を返す関数 | **自動即時評価**（副作用は必殺実行） |
| **ルール2** | `@eager` デコレータを使用した式 | 戻り型に関係なく**強制即時評価** |
| **ルール3** | Void 以外を返す型 | **遅延評価**（デフォルト） |

```yaoxiang
# Void を返す関数は自動即時実行（副作用）
log: (String) -> Void = (message) => {
    print(message)
}

# @eager デコレータ：强制即時評価
cache_compute: (Int) -> Int = (x) => {
    # Int を返しても、強制即時実行
    expensive_calculation(x)
}

main: () -> Void = () => {
    # log は自動即時実行（Void を返すため）
    log("Processing started")

    # @eager で强制即時実行
    @eager
    cache_compute(100)

    # 通常の関数は遅延実行（Int を返すため）
    result = heavy_computation(200)  # この時点では実行されない
    print(result)  # ここで初めて実行
}
```

### 4.2 エラー処理

#### Result 型定義

```yaoxiang
# 標準Result型（統一コンストラクタ構文）
type Result[T, E] = ok(T) | err(E)

# カスタムエラー型
type ParseError = invalid_format | unexpected_eof | position(Int)

parse_config: (String) -> Result[Config, ParseError] = (content) => {
    if content.is_empty() {
        err(invalid_format)
    } else {
        ok(parse(content))
    }
}
```

#### エラー伝播構文

Rust 式の `?` 演算子を採用し、透過的なエラー伝播を実現：

```yaoxiang
# Rust式 ? 演算子
process() -> Result[Data, Error] = {
    data = fetch_data()?      # 自動待機してエラー検査
    processed = transform(data)?
    save(processed)?          # エラーは自動的に上位に伝播
}

# パターン照合でエラーを処理
handle_result: (Result[Int, Error]) -> String = (result) => {
    match result {
        ok(value) => "Success: " + value.to_string()
        err(e) => match e {
            network_error => "Network failed"
            parse_error => "Parse failed"
            _ => "Unknown error"
        }
    }
}
```

#### エラープログラム可視化

エラープログラムは呼び出しスタック类似だが、DAG でのエラー伝播パスを表示：

```
┌─────────────────────────────────────────────────────────────┐
│ Error: Division by zero                                     │
├─────────────────────────────────────────────────────────────┤
│ Error Graph:                                                │
│                                                             │
│   main()                                                   │
│     │                                                       │
│     ├──► calculate()                                        │
│     │         │                                             │
│     │         └──► divide(100, 0)  ✗ [Division by zero]     │
│     │                                                       │
│     └──► fallback()  ✓                                      │
│                                                             │
│ 因果鎖: main → calculate → divide                           │
│ 捕獲位置: calculate (第42行)                                │
└─────────────────────────────────────────────────────────────┘
```

#### エラー処理ベストプラクティス

```yaoxiang
# 複数のエラー発生可能な操作を組合せる
batch_process: ([String]) -> Result[[String], Error] = (items) => {
    results = items.map(item => {
        process_item(item)?
    })
    ok(results)
}

# with? 糖衣構文（将来機能）
validate_user: (User) -> Result[ValidatedUser, ValidationError] = (user) => {
    name = user.name.with?(validate_name)?
    email = user.email.with?(validate_email)?
    ok(ValidatedUser(name, email))
}
```

### 4.3 純粋関数と `@blocking` 同期保証

**コア洞察：純粋関数は阻塞しない！**

理由は：
- 純粋関数には I/O がなく、CPU 計算のみ
- 計算が長くてもスケジューラを阻塞せず、CPU 時間のみ占有

**実行戦略：**

| 関数型 | 実行戦略 | 阻塞？ |
|----------|----------|--------|
| 純粋関数（I/O なし） | 同期実行 | いいえ（CPU 占有のみ） |
| 非同期関数（`Async[T]` を返す） | 非同期実行 | いいえ |
| `@blocking` 注釈関数 | 同期実行、内部スケジューリング | いいえ |

**`@blocking` 注釈：同期実行保証**

`@blocking` 注釈は関数が同期姿勢で実行することを保証：
- 関数が戻る時、結果は既に準備完了
- 内部に非同期呼び出しがある場合、内部で完了までスケジューリング
- 同期セマンティクスが必要だが内部に非同期操作が含まれるシナリオに適合

```yaoxiang
# @blocking：同期実行、内部非同期スケジューリング完了後に戻る
heavy_compute: (List[Int]) -> Int = (data) => {
    # 内部に非同期操作があるかもしれないが、戻る前に完了
    processed = data.map(x => async_transform(x))
    processed.sum()
}

# 通常の非同期関数：Async[T] を返す
fetch_user: (Int) -> Async[User] = (id) => {
    database.query("SELECT * FROM users WHERE id = ?", id)
}

# 純粋関数：自動同期（I/O なし）
factorial: (Int) -> Int = (n) => {
    if n <= 1 then 1 else n * factorial(n - 1)
}

main: () -> Void = () => {
    # @blocking 関数：同期実行
    result = heavy_compute([1, 2, 3, 4, 5])  # 即座に結果を返す
    print(result)  # 15

    # 非同期関数：Async[User] を返す
    user = fetch_user(123)  # Async[User]
    print(user.name)  # 自動待機してアンパック
}
```

**ランタイム戦略：**

```rust
fn execute_function(node: &DAGNode) {
    match node.execution_mode {
        ExecutionMode::Pure => {
            // 純粋関数：同期実行
            node.execute();
        }
        ExecutionMode::Async => {
            // 非同期関数：async スケジューラにサブミット
            async_runtime.submit(node);
        }
        ExecutionMode::Blocking => {
            // @blocking 関数：同期実行、内部で非同期操作をスケジューリング
            execute_blocking(node);
        }
    }
}

fn execute_blocking(node: &DAGNode) {
    // 関数本体を実行
    let result = node.execute_body();
    
    // 内部のすべての非同期操作を収集
    let internal_async_ops = collect_async_ops(node);
    
    // すべての内部非同期操作を待機
    if !internal_async_ops.is_empty() {
        async_runtime.wait_all(internal_async_ops);
    }
    
    // 結果を返す
    result
}
```

**設計上の優位性：**
- **簡潔**：複雑な effect システム不要
- **柔軟**：`@blocking` はオプションで、同期セマンティクスが必要な時のみ使用
- **効率的**：純粋関数は自動同期実行
- **安全**：メインスケジューラは永不阻塞

### 4.4 リソース競合検出

コンパイル時にリソースアクセスパターンを分析し、競合する操作を自動直列化：

```
リソース競合ルールマトリックス：
╔═══════════╦══════════╦══════════╗
║   アクセス    ║   読     ║    書     ║
╠═══════════╬══════════╬══════════╣
║   読      ║  並列可能  ║  直列化  ║
║   書      ║  直列化  ║  直列化  ║
╚═══════════╩══════════╩══════════╝
```

**コンパイル時分析示例：**

```rust
// コンパイル時のリソースアクセス分析
struct ResourceAccess {
    reads: Set<ResourceId>,   // 読み取るリソース
    writes: Set<ResourceId>,  // 書き込むリソース
}

// 示例
file1 = open("a.txt")  // リソース1：読
file2 = open("b.txt")  // リソース2：読
// file1 読 と file2 読 → 並列可能

file3 = open("c.txt")  // リソース3：書
// file1 読 と file3 書 → 直列化
// file2 読 と file3 書 → 直列化
```

**コード示例：**

```yaoxiang
# コンパイラが自動検出と競合操作の直列化
process_files: () -> Void = () => {
    file_a = open("a.txt")  # リソース1：読
    file_b = open("b.txt")  # リソース2：読
    # file_a と file_b が両方とも読のみ → 並列可能

    file_c = open("c.txt")  # リソース3：書
    # file_a 読 と file_c 書 → 直列化
    # file_b 読 と file_c 書 → 直列化
}

# 複数の書操作は自動直列化
write_logs: () -> Void = () => {
    log1 = open_log("log1.txt")  # リソース1：書
    log2 = open_log("log2.txt")  # リソース2：書
    # log1 と log2 が異なるリソース → 並列可能
}
```

### 4.5 並列競合制御：型システムによるアトミック性保証

**コア思想：型システムで並行アクセスするデータをマーキングし、コンパイラが同期正確性を検査。**

**型マーキング体系：**

| 型 | セマンティクス | 並行安全 | 説明 |
|------|------|----------|------|
| `T` | 不変データ | ✅ 安全 | デフォルト型、マルチタスク読み取りで競合なし |
| `Ref[T]` | 変更可能参照 | ⚠️ 同期必要 | 並行変更可能としてマーキング、ロック使用をコンパイラ検査 |
| `Atomic[T]` | アトミック型 | ✅ 安全 | 下位レベルのアトミック操作、ロック不要の並行 |
| `Mutex[T]` | ミューテックス包装 | ✅ 安全 | 自動ロック/ロック解除、コンパイラが保証 |
| `RwLock[T]` | 読み書きロック包装 | ✅ 安全 | 読み取り多用書き込み少量シナリオの最適化 |

**型安全性保証：**

```yaoxiang
# デフォルト不変 - 自然に競合なし
data: List[Int] = [1, 2, 3, 4, 5]
spawn for x in data { process(x) }  # ✅ 安全、読み取りのみで競合なし

# 変更可能参照 - 同期が必要
counter: Ref[Int] = Ref.new(0)

# 錯誤示例：ロックなしでの Ref アクセス（コンパイルエラー）
spawn for i in 1..10 {
    # ❌ コンパイルエラー：Ref は同期プリミティブ経由でアクセス必須
    counter.value = counter.value + i
}

# 正しい示例：with 糖衣構文で自動ロック
spawn for i in 1..10 {
    # ✅ with ブロックが自動的にロック取得と解放
    with counter.lock() {
        counter.value = counter.value + i
    }
}

# アトミック型 - ロック不要の並行
atomic_counter: Atomic[Int] = Atomic.new(0)
spawn for i in 1..10 {
    # ✅ アトミック操作、ロック不要で安全
    atomic_counter.fetch_add(i)
}
```

**Mutex[T] 型 - コンパイル時ロック保証：**

```yaoxiang
# ミューテックスで包装されたデータを作成
shared_state: Mutex[Map[String, Int]] = Mutex.new(Map.empty())

# with 糖衣構文を使用（Go の defer 类似）
main: () -> Void = () => {
    spawn for i in 1..100 {
        # with が自動的にロック取得、ブロック終了時に自動解放
        with shared_state.lock() {
            # クリティカルセクション：Mutex で保護
            current = shared_state.get("count").or(0)
            shared_state.set("count", current + 1)
        }
    }

    # すべてのタスク完了を待機
    print(shared_state.get("count"))  # 100
}
```

**型推論とロック検査：**

```rust
// コンパイラがコンパイル時にロックを検査
fn compile_check_locks(func: &Function) {
    for node in func.nodes {
        match node {
            NodeKind::ReadRef(ref_var) => {
                // ロック保護範囲内にいるか検査
                if !is_inside_lock_guard(ref_var) {
                    compile_error!("Ref アクセスは lock() 保護範囲内でなければならない");
                }
            }
            NodeKind::WriteRef(ref_var, _) => {
                // 二重検査：ロック + 唯一の書き込み者
                if !is_inside_lock_guard(ref_var) {
                    compile_error!("Ref 変更は lock() 保護範囲内でなければならない");
                }
                if has_multiple_writers(func, ref_var) {
                    compile_error!("Mutex[T] は書き込み者が1つのみのため、RwLock[T] が必要");
                }
            }
            _ => {}
        }
    }
}
```

**設計上の優位性：**

| 優位性 | 説明 |
|------|------|
| **コンパイル時検査** | ロック洩れはコンパイル時に捕獲、実行時デッドロックではない |
| **ゼロランタイムオーバーヘッド** | 競合 없을 시 Mutex 包装にオーバーヘッドなし |
| **構文簡潔** | `with lock() { ... }` 糖衣構文、ライフサイクルを自動管理 |
| **型安全** | Ref を誤って Atomic の代わりに使用すると型レベルでエラー |

---

## 五、優位性のまとめ

| 優位性 | 説明 |
|------|------|
| **ゼロ伝染性** | 非同期コードと同期コードが構文と型シグネチャで違いなし、\"async/await\" 伝染を完全に根絶 |
| **高性能並列** | 遅延並作グラフと明示的な `spawn` マーキングの組み合わせにより、ランタイムが自動的で並列性を発掘できると同時に、程序员が极限性能最適化のための明確なツールを持つ |
| **メンタルモデル簡潔** | 開発者はデータフローとビジネスロジックのみに注意を払えばよく、複雑な並行プリミティブとコールバックを理解する必要はない |
| **リファクタリング容易** | 順序ロジックと並行ロジック間の切り替えコストが極めて低く、`spawn {}` 包装を増減するのみでよい |
| **用語直感的** | 「並作関数」「並作ブロック」「並作値」により、技術DISCUSSION が極めて直感的になる |

---

## 六、実装上の考慮事項

### 6.1 コンパイラ

- [ ] データフロー分析を実装し、並作グラフを構築
- [ ] `spawn` 戻り型マーキングの解析と型推論を実装
- [ ] `spawn {}` と `spawn for` をランタイム並列プリミティブにデシュガー
- [ ] 注釈（`@eager`、`@blocking`）をサポート
- [ ] Void 戻り型自動即時評価ロジックを実装
- [ ] リソース競合検出を実装
- [ ] Send/Sync 型制約検査を実装

### 6.2 ランタイム

- [ ] ワークスティーリングをサポートする並作スケジューラを実装
- [ ] 計算グラフ依存関係を認識するタスクスケジューリングを実装
- [ ] `Async[T]` 型の自動アンパックメカニズムを実装
- [ ] Void 関数の自動即時実行を実装
- [ ] エラープログラムの生成と伝播を実装
- [ ] リソースアクセス直列化を実装

### 6.3 デバッグツール ⚠️ 必须

**計算グラフ可視化デバッガ**は複雑なプログラム動作を理解するための鍵：

| 機能 | 説明 |
|------|------|
| **ノード状態可視化** | 各計算ノードの Pending/Running/Completed 状態を観察 |
| **依存関係表示** | ノード間のデータ依存辺を表示 |
| **タスクフロー追跡** | タスクの各スレッド間での流れを観察 |
| **パフォーマンスボトルネック特定** | 長いチェーンとホットスポットノードを識別 |
| **エラープログラム可視化** | 並行環境下でのエラー伝播パス表示 |

---

## 七、コード示例

### 7.1 基本的な並作関数

```yaoxiang
use std.net

# 並作関数定義：戻り型マーキングが spawn
fetch_user: (Int) -> User spawn = (id) => {
    response = net.HTTP.get("/users/" + id.to_string())
    response.json()
}

fetch_posts: (Int) -> List[Post] spawn = (user_id) => {
    response = net.HTTP.get("/users/" + user_id.to_string() + "/posts")
    response.json()
}

main: () -> Void = () => {
    # 自動並列実行（依存関係なし）
    user = fetch_user(123)      # Async[User]
    posts = fetch_posts(123)    # Async[List[Post]]

    # ここで自動待機してアンパック
    print(user.name)            # 同期コード一样自然
    print(posts.length)
}
```

### 7.2 並作ブロック

```yaoxiang
fetch: (String) -> JSON spawn = (url) => { ... }
parse: (JSON) -> Model spawn = (json) => { ... }

parallel_fetch: () -> (Model, Model) = () => {
    # 並作ブロック：明示的な並行領域
    (model_a, model_b) = spawn {
        parse(fetch("https://api1.com/data")),
        parse(fetch("https://api2.com/data"))
    }
    # モデル a と b はここですぐ利用可能
    (model_a, model_b)
}
```

### 7.3 並作ループ

```yaoxiang
process_item: (Item) -> Result[Processed, Error] spawn = (item) => { ... }

batch_process: (List[Item]) -> List[Result[Processed, Error]] = (items) => {
    # 並作ループ：データ並列
    results = [spawn for item in items {
        process_item(item)
    }]
    # results はここですべての処理結果を含む List
    results
}
```

---

> *"万物並作、吾れ以って復を觀る。"*
> —— 『易・復卦』
>
> 並作モデルは遅延評価の宣言的優雅さと高性能並行の要求を組み合わせ、システムプログラミングに安全でありながら極めて表現力豊かな全新パラダイムを提供することを目指す。
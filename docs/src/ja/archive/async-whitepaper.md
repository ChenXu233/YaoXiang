> **⚠️ 注意：本文档已过时，仅供参考。**
>
> 本ドキュメントはすでに時代遅れであり、参考目的のみの使用を想定しています。

# **《併作：惰性求値に基づく無痛非同期的並行処理モデル》技術白書**

## 🏛️ 一、核心定義：併作モデル

**併作モデル**は、《易・復卦》の「万物并作，吾以观復」に由来します。これはプログラミング言語の並行処理パラダイムであり、開発者が同期・順序的な思考でロジックを記述でき、言語ランタイムがその計算ユニットを万物並作のごとく自動かつ効率的に並行実行させ、最終的に統一的に協調させるものです。

### コア設計思想：デフォルト惰性 + spawn 型マーキング

| 設計原則 | 説明 |
|----------|------|
| **デフォルト惰性求値** | 全関数は（Haskell同理的に）デフォルトで惰性であり、Lazy[T] を返す |
| **コア数設定** | スクリプトヘッダの `// @cores: N` 宣言で並列化を自動有効化 |
| **spawn 型マーキング** | `-> T spawn` で関数を厳密に非同期的かつ並行実行可能としてマーク |
| **混合求値モード** | `@eager`（デコレータ、強制即時）、`@auto`（デコレータ、並列維持） |
| **Void 自動即時** | Void を返す関数は自動的に即時求値（副作用は実行されなければならない） |

### コア三原則

| コア原則 | 解説 |
|----------|------|
| **同期構文** | 目に見える通りの順序コード、記述した通りの実行フロー |
| **並行本質** | ランタイムが自動的に並列性を抽出し、データ依存関係から並行の機会を挖掘 |
| **統一的協調** | 結果が必要なタイミングで自動的に収束し、論理的正確性を保証 |

**これは以下の二つの根本的転換によって達成されます：**

1. **「制御フロー」を「データフロー」に変換**：プログラムは純粋関数的な惰性求値データフローグラフとして解釈される
2. **「非同期伝染」を「依存関係解決」に変換**：非同期的であることは関数のシグネチャの作用ではなくなり、ランタイムがデータ依存点で自動的に実行を待つ操作となる

---

## 📚 二、用語体系：統一された概念マップ

「併作」を中心に、明確で自己完結した用語体系を構築し、すべての設計を繋ぎ合わせます：

| 公式用語 | 対応構文/概念 | 解説 |
|----------|---------------|------|
| **併作関数** | `-> T spawn` | 戻り値の型マーキングであり、「併作」並行実行に参加可能な計算ユニットを示す |
| **併作ブロック** | `spawn { a(), b() }` | 開発者が明示的に宣言した並行領域であり、ブロック内のタスクは「併作」実行される |
| **併作ループ** | `spawn for x in xs { ... }` | データ並列パラダイムであり、ループ体がすべてのデータ要素で「併作」実行される |
| **併作値** | `Async[T]` プロキシ型 | 併作進行中の「未来値」であり、使用時に自動的に「作」完了を待つ |
| **併作グラフ** | 惰性計算グラフ（DAG） | 「併作」が発生する舞台であり、すべての計算ユニット間の依存関係と並列関係を記述 |
| **併作スケジューラ** | ランタイムタスクスケジューラ | 「万物」を調整し、正しいタイミングで「併作」させる知的中枢 |
| **エラーフグラフ** | Error Graph | 並行環境下でのエラー伝播パス可視化であり、コールスタックに類似するがDAG中のエラー流向を示す |
| **リソース競合** | Resource Conflict | 複数のタスクが同時に同一書き込みリソースにアクセスする際の競合であり、コンパイル時に検出され自動直列化される |

> **技術交流示例**：「ここでは併作ブロックを使って二つの併作関数を並行呼び出しし、自動的にその併作値を取得しています。」

---

## 三、三層並行アーキテクチャ：漸進的透明性

### 3.1 アーキテクチャ概要

併作モデルは**三層漸進的並行抽象化**を提供し、不同なスキルレベルの開発者が各自に相応しい使用パターンを見つけられるようにします：

| レベル | モード | 構文マーキング | 実行方式 | 制御性 | 適用シーン |
|------|------|----------|----------|--------|----------|
| **L1** | `@blocking` 同期 | `@blocking` | 完全順序実行 | 最高 | デバッグ、初心者の学習、重要なコード段 |
| **L2** | 明示的 spawn | `spawn` | 開発者制御可能並行 | 中 | 中級ユーザー、精密な並行制御が必要な場合 |
| **L3** | 完全透明 | なし（デフォルト） | 自動最適並列 | 最低 | 上級者、自動並列最適化 |

### 3.2 L1: `@blocking` 同期モード

**コア特性**：全並行最適化を無効化し、完全順序実行を実現し、デバッグと理解を容易にする。

```yaoxiang
# L1: @blocking 同期モード（注解は戻り値型の後に配置）
fetch_sync: (String) -> JSON @blocking = (url) => {
    HTTP.get(url).json()
}

main: () -> Void @blocking = () => {
    # 厳密な順序実行であり、並行は一切ない
    data1 = fetch_sync("https://api.example.com/data1")
    data2 = fetch_sync("https://api.example.com/data2")
    process(data1, data2)
}
```

### 3.3 L2: 明示的 spawn 並行

**コア特性**：開発者が並行可能なユニットを明示的にマークし、制御可能性を維持しながら並行好处を獲得。

```yaoxiang
# L2: 明示的 spawn 並行
fetch_data: (String) -> JSON spawn = (url) => {
    HTTP.get(url).json()
}

process_users_and_posts: () -> Void spawn = () => {
    users = fetch_data("https://api.example.com/users")
    posts = fetch_data("https://api.example.com/posts")
    # users と posts は自動的に並列実行される
    print(users.length.to_string())
    print(posts.length.to_string())
}

# 明示的並行ブロック
compute_all: () -> (Int, Int, Int) spawn = () => {
    (a, b, c) = spawn {
        heavy_calc(1),
        heavy_calc(2),
        heavy_calc(3)
    }
    (a, b, c)
}
```

### 3.4 L3: 完全透明（デフォルト）

**コア特性**：マーキングは一切不要であり、コンパイラが自動的に依存関係を分析して最適並列実行計画を生成。

```yaoxiang
# L3: 完全透明（デフォルトモード）
heavy_calc: (Int) -> Int = (n) => {
    fibonacci(n)
}

auto_parallel: (Int) -> Int = (n) => {
    # システムは自動的に分析：a, b, c は依存関係なし、完全並列可能
    a = heavy_calc(1)
    b = heavy_calc(2)
    c = heavy_calc(3)
    a + b + c
}
```

### 3.5 手動制御アノテーション

| アノテーション | 動作 | 使用シーン |
|------|------|----------|
| `@eager` | 強制即時求値 | 即座に結果を取得する必要がある計算 |

---

## 二、核心概念

### 2.1 並作グラフ：万物並作の舞台

すべてのプログラムはコンパイル時に**有向非巡回計算グラフ（DAG）**に変換され、これは**並作グラフ**と呼ばれます。

| 要素 | 説明 |
|------|------|
| **ノード** | 式計算ユニットを表現 |
| **辺** | データ依存関係を表現（A → B は B が A の結果を依存することを示す） |
| **惰性** | ノードはその出力が**実際に必要**になった時のみ求値される |

### 2.2 デフォルト惰性求値

全関数はデフォルトで**惰性求値**戦略を採用します：

```yaoxiang
# スクリプトヘッダで並列コア数を設定
# @cores: 4

# 全関数はデフォルトで惰性求値（デフォルトで並行可能）
heavy_computation: (Int) -> Int = (x) => {
    # この関数は即座には実行されない
    # 結果が使われた时才执行
    fibonacci(x)
}

main: () -> Void = () => {
    # heavy_computation は Int を返し、型は Lazy[Int]
    result = heavy_computation(100)

    # ここで、result が加算に使われ、求値がトリガーされる
    # システムは自動的に最適なタイミングで並列実行を見つける
    total = result + heavy_computation(200)
}
```

### 2.3 混合求値アノテーション（デコレータスタイル）

YaoXiang のアノテーションは Python のデコレータに類似し、関数や式の動作を変更するために使用されます：

| アノテーション（デコレータ） | 動作 |
|----------------|------|
| `@eager` | **デコレータ**：強制即時求値、即座に実行 |
| `@auto` | **デコレータ**：並列維持（デフォルト、省略可能） |

**Void 自動即時ルール：** Void を返す関数は自動的に即時求値されます（いかなるアノテーション也不要）。なぜならば副作用は実行されなければならないからです。

```yaoxiang
# @eager デコレータ：強制即時求値
heavy_computation: (Int) -> Int = (x) => {
    fibonacci(x)
}

# Void を返す関数は自動的に即時求値（副作用関数）
log: (String) -> Void = (message) => {
    print(message)
}

main: () -> Void = () => {
    # log は自動的に即時実行（Void を返すため）
    log("Processing started")

    # @eager を使用して強制即時
    @eager heavy_computation(100)
}
```

### 2.4 並作値：Async[T] 惰性プロキシ型

戻り値の型が `-> T spawn` とマーキングされた任意の関数は、直ちに `Async[T]` 型の値を返します。これを**並作値**と呼びます。

```yaoxiang
# 併作関数：戻り値型が -> JSON spawn とマーキング
# これは厳密に並行実行可能な計算ユニットであることを示す
fetch: (String) -> JSON spawn = (url) => {
    HTTP.get(url).json()
}

main: () -> Void = () => {
    # fetch が返すのは並作値 Async[JSON]
    # しかし、使用時には追加の構文は不要
    data = fetch("https://api.example.com")  # Async[JSON]

    # ここで、data は自動的に待機され JSON にアンパックされる
    print(data.name)  # 同期コードと同様に自然
}
```

#### 並作値のコア特性

| 特性 | 説明 |
|------|------|
| **構文透明性** | `Async[T]` は型システムにおいて `T` のサブタイプであり、`T` が期待される任意のコンテキストで使用可能 |
| **オンデマンド待機** | `T` 型の具体値を使用する必要がある時（例：フィールドアクセス、算術演算）、ランタイムは自動的にサスペンドして待機 |
| **エラー伝播** | 内部的には実際には `Result<T, E>` であり、エラーはデータフローに沿って自然に伝播 |

### 2.7 併作構文要素：「修飾子」から「型マーキング」への進化

`spawn` キーワードは同期思考と同期実装を繋ぐ唯一の架け橋であり、三重の意味を持ちます：

| 構文形式 | 公式用語 | 意味 | ランタイム動作 |
|:---------|:---------|:-----|:----------|
| **`-> T spawn`** | 併作関数 | 戻り値型マーキングであり、これは厳密に並行参加可能な計算ユニットであることを示す | その呼び出しは `Async[T]` を返し、並作グラフノードの作成をマークする |
| **`spawn { ... }`** | 併作ブロック | 明示的に宣言された並行領域 | ランタイムはブロック内の各式を**積極的に**独立タスクとして並行実行し、ブロック終了時に暗黙的に全結果を待機 |
| **`spawn for`** | 併作ループ | データ並列ループ | ループ体を複数の並列タスクに変換し、自動的にデータシャーディング、スケジューリング、结果収集を行う |

---

## 三、作動原理：コードから実行へ

### 3.1 コンパイル時：並作グラフの構築

```yaoxiang
# 併作関数定義：戻り値型が spawn とマーキング
fetch: (String) -> String spawn = (url) => { ... }
parse: (String) -> Model spawn = (data) => { ... }

process: () -> Report = () => {
    # コンパイラはここで並作グラフノードを作成
    data_a = fetch("url1")  # ノード A: Async[String]
    data_b = fetch("url2")  # ノード B: Async[String]

    # 併作ブロック：明示的並行領域
    (model_a, model_b) = spawn {
        parse(data_a),  # ノード C: A に依存
        parse(data_b)   # ノード D: B に依存
    }

    # 最終集約ノード
    generate_report(model_a, model_b)  # ノード E
}
```

**コンパイラ操作：**
1. ソースコードを解析し、グローバル並作グラフを構築
2. 各式に対して計算ノードを作成
3. データ依存関係を分析し、辺関係を確立
4. `spawn { }` と `spawn for` ブロック内のサブグラフには **「並列求値」** マークが付与

### 4.2 ランタイム：並作スケジューラ

wargs窃取をサポートするインテリジェントな**並作スケジューラ**が並作グラフの実行を担当します：

```rust
// 併作スケジューラのコアロジック
impl FlowScheduler {
    fn execute_node(&self, node_id: NodeId) {
        let node = self.get_node(node_id);
        
        match &node.kind {
            NodeKind::AsyncCompute => {
                // 併作関数：コルーチンプールにサブミット
                self.submit_async(node_id);
            }
            NodeKind::ParallelBlock => {
                // 併作ブロック：積極的に全直接サブノードを並列実行
                self.submit_parallel(node_id);
            }
            NodeKind::DataParallel { iterator, body } => {
                // 併作ループ：自動シャーディング
                self.submit_data_parallel(node_id, iterator, body);
            }
            _ => { /* 同期実行 */ }
        }
    }
}
```

#### 実行フロー

```
1. [E] の求値的需要から、[C] と [D] が必要
2. [C] は [A] に依存、[D] は [B] に依存
3. 並作スケジューラは [A] と [B] に依存関係がないことを発見 → 直ちに並列実行
4. [A]、[B] 完了後、併作ブロックマークにより → 直ちに [C] と [D] を並列実行
5. [C]、[D] 完了後、[E] を実行
```

**重要機構：**

| 機構 | 説明 |
|------|------|
| **惰性トリガー** | 最終結果の要求から実行が開始され、依存関係を逆方向に追跡 |
| **自動待機** | `Async[T]` に遭遇すると自動的にサスペンドし、他の準備完了タスクを実行 |
| **ワークスティール** | スレッドが他のスレッドのキューからタスクをスティールし、CPU 利用率を向上 |

---

## 四、重要機構詳解

### 4.1 副作用と求値保証

純粋な惰性求値は副作用（ログ、書き込みなど）が永不執行につながる可能性があります。並作モデルは**戻り値型に基づく自動推論**を採用します：

| ルール | 条件 | 動作 |
|------|------|------|
| **ルール一** | Void を返す関数 | **自動即時求値**（副作用は実行されなければならない） |
| **ルール二** | `@eager` デコレータを使用する式 | 戻り値型にかかわらず、**強制即時求値** |
| **ルール三** | Void 以外の型を返す | **惰性求値**（デフォルト） |

```yaoxiang
# Void を返す関数は自動的に即時実行（副作用）
log: (String) -> Void = (message) => {
    print(message)
}

# @eager デコレータ：強制即時求値
cache_compute: (Int) -> Int = (x) => {
    # Int を返即便も、強制即時実行
    expensive_calculation(x)
}

main: () -> Void = () => {
    # log は自動的に即時実行（Void を返すため）
    log("Processing started")

    # @eager で強制即時実行
    @eager
    cache_compute(100)

    # 通常の関数は惰性実行（Int を返す）
    result = heavy_computation(200)  # この時点では実行されない
    print(result)  # ここで初めて実行
}
```

### 4.2 エラー処理

#### Result 型定義

```yaoxiang
# 標準 Result 型（統一構築子構文）
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

Rust 式の `?` 演算子を採用し、透明なエラー伝播を実現：

```yaoxiang
# Rust式 ? 演算子
process() -> Result[Data, Error] = {
    data = fetch_data()?      # 自動的に待機しエラーをチェック
    processed = transform(data)?
    save(processed)?          # エラーは自動的に上に伝播
}

# パターンマッチでエラーを処理
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

#### エラーフグラフ可視化

エラーフグラフはコールスタックに類似しますが、DAG 内のエラー伝播パスを示します：

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
# 複数のエラー発生可能な操作を組み合わせる
batch_process: ([String]) -> Result[[String], Error] = (items) => {
    results = items.map(item => {
        process_item(item)?
    })
    ok(results)
}

# with? 構文糖（将来機能）
validate_user: (User) -> Result[ValidatedUser, ValidationError] = (user) => {
    name = user.name.with?(validate_name)?
    email = user.email.with?(validate_email)?
    ok(ValidatedUser(name, email))
}
```

### 4.3 純粋関数と `@blocking` 同期保証

**核心的洞察：純粋関数はブロックしない！**

なぜならば：
- 純粋関数は I/O を持たず、CPU 計算のみ
- 計算が多长时间かかってもスケジューラをブロックせず、CPU 時間のみ占有

**実行策略：**

| 関数型 | 実行策略 | ブロック？ |
|----------|----------|----------|
| 純粋関数（I/O なし） | 同期実行 | いいえ（CPU 占有のみ） |
| 非同期関数（`Async[T]` を返す） | 非同期実行 | いいえ |
| `@blocking` アノテーション関数 | 同期実行、内部スケジューリング | いいえ |

**`@blocking` アノテーション：同期実行保証**

`@blocking` アノテーションは関数が同期姿勢で実行されることを保証します：
- 関数が戻った時には結果が準備完了
- 内部に非同期呼び出しがあっても、内部でスケジューリングを完了
- 同期 семантикаが必要だが内部に非同期操作が含まれる可能性があるシナリオに適合

```yaoxiang
# @blocking：同期実行、内部非同期スケジューリング完了後に返す
heavy_compute: (List[Int]) -> Int = (data) => {
    # 内部に非同期操作がある可能性があるが、返回前に完了
    processed = data.map(x => async_transform(x))
    processed.sum()
}

# 通常の非同期関数：Async[T] を返す
fetch_user: (Int) -> Async[User] = (id) => {
    database.query("SELECT * FROM users WHERE id = ?", id)
}

# 純粋関数：自動的に同期（I/O なし）
factorial: (Int) -> Int = (n) => {
    if n <= 1 then 1 else n * factorial(n - 1)
}

main: () -> Void = () => {
    # @blocking 関数：同期実行
    result = heavy_compute([1, 2, 3, 4, 5])  # 直ちに結果を返す
    print(result)  # 15

    # 非同期関数：Async[User] を返す
    user = fetch_user(123)  # Async[User]
    print(user.name)  # 自動的に待機しアンパック
}
```

**ランタイム策略：**

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
    
    // 内部の全非同期操作を収集
    let internal_async_ops = collect_async_ops(node);
    
    // 内部の全非同期操作の完了を待機
    if !internal_async_ops.is_empty() {
        async_runtime.wait_all(internal_async_ops);
    }
    
    // 結果を返す
    result
}
```

**設計优势：**
- **簡潔**：複雑な effect システムが不要
- **柔軟**：`@blocking` はオプションであり、同期 семантикаが必要な時に使用
- **効率**：純粋関数は自動的に同期実行
- **安全**：メインストリームのスケジューラは永不ブロック

### 4.4 リソース競合検出

コンパイル時にリソースアクセスパターンを分析し、競合操作を自動的に直列化：

```
リソース競合ルールマトリックス：
╔═══════════╦══════════╦══════════╗
║   アクセス   ║   読取     ║    書込    ║
╠═══════════╬══════════╬══════════╣
║   読取      ║  並列可能  ║  直列化   ║
║   書込      ║  直列化   ║  直列化   ║
╚═══════════╩══════════╩══════════╝
```

**コンパイル時分析示例**：

```rust
// コンパイル時のリソースアクセス分析
struct ResourceAccess {
    reads: Set<ResourceId>,   // 読み取るリソース
    writes: Set<ResourceId>,  // 書き込むリソース
}

// 示例
file1 = open("a.txt")  // リソース1：読取
file2 = open("b.txt")  // リソース2：読取
// file1 読取 と file2 読取 →  並列可能

file3 = open("c.txt")  // リソース3：書込
// file1 読取 と file3 書込 → 直列化
// file2 読取 と file3 書込 → 直列化
```

**コード示例**：

```yaoxiang
# コンパイラは自動的に競合を検出し直列化
process_files: () -> Void = () => {
    file_a = open("a.txt")  # リソース1：読取
    file_b = open("b.txt")  # リソース2：読取
    # file_a と file_b は両方とも読取のみ →  並列可能

    file_c = open("c.txt")  # リソース3：書込
    # file_a 読取 と file_c 書込 → 直列化
    # file_b 読取 と file_c 書込 → 直列化
}

# 複数の書込操作は自動的に直列化
write_logs: () -> Void = () => {
    log1 = open_log("log1.txt")  # リソース1：書込
    log2 = open_log("log2.txt")  # リソース2：書込
    # log1 と log2 は異なるリソース →  並列可能
}
```

### 4.5 並列競合制御：型システムによる原子性保証

**核心思想：型システムで並行アクセスデータをマークし、コンパイラが同期正確性をチェックする。**

**型マーキング体系：**

| 型 |  семантика | 並行安全 | 説明 |
|------|------|----------|------|
| `T` | 変更不可データ | ✅ 安全 | デフォルト型、複数タスクの読取は競合なし |
| `Ref[T]` | 変更可能参照 | ⚠️ 同期が必要 | 並列変更可能としてマーク、ロック使用をコンパイル時にチェック |
| `Atomic[T]` | 原子型 | ✅ 安全 | 下位層原子操作、ロック不要の並行 |
| `Mutex[T]` | 相互排除ロック包装 | ✅ 安全 | 自動ロック・ロック解除、コンパイル時保証 |
| `RwLock[T]` | 読取書込ロック包装 | ✅ 安全 | 読取多書込少シナリオを最適化 |

**型安全性保証：**

```yaoxiang
# デフォルト不変 - 自然無競合
data: List[Int] = [1, 2, 3, 4, 5]
spawn for x in data { process(x) }  # ✅ 安全、読取のみで競合なし

# 変更可能参照 - 同期が必要
counter: Ref[Int] = Ref.new(0)

# 錯誤示例：ロックなしの Ref アクセス（コンパイルエラー）
spawn for i in 1..10 {
    # ❌ コンパイルエラー：Ref は同期プリミティブを通じてアクセスする必要がある
    counter.value = counter.value + i
}

# 正しい示例：with 構文糖で自動ロック
spawn for i in 1..10 {
    # ✅ with ブロックは自動的にロック取得・解放
    with counter.lock() {
        counter.value = counter.value + i
    }
}

# 原子型 - ロック不要の並行
atomic_counter: Atomic[Int] = Atomic.new(0)
spawn for i in 1..10 {
    # ✅ 原子操作、ロック不要で安全
    atomic_counter.fetch_add(i)
}
```

**Mutex[T] 型 - コンパイル時ロック保証：**

```yaoxiang
# 相互排除ロックで包装されたデータを作成
shared_state: Mutex[Map[String, Int]] = Mutex.new(Map.empty())

# with 構文糖を使用（Go の defer に類似）
main: () -> Void = () => {
    spawn for i in 1..100 {
        # with は自動的にロックを取得し、ブロック終了時に自動解放
        with shared_state.lock() {
            # 臨界區間：Mutex により保護
            current = shared_state.get("count").or(0)
            shared_state.set("count", current + 1)
        }
    }

    # すべてのタスク完了を待機
    print(shared_state.get("count"))  # 100
}
```

**型推論とロックチェック：**

```rust
// コンパイラはコンパイル時にチェック
fn compile_check_locks(func: &Function) {
    for node in func.nodes {
        match node {
            NodeKind::ReadRef(ref_var) => {
                // ロック保護範囲内かどうかチェック
                if !is_inside_lock_guard(ref_var) {
                    compile_error!("Ref アクセスは lock() 保護範囲内でなければならない");
                }
            }
            NodeKind::WriteRef(ref_var, _) => {
                // 二重チェック：ロック + 唯一の書込者
                if !is_inside_lock_guard(ref_var) {
                    compile_error!("Ref 変更は lock() 保護範囲内でなければならない");
                }
                if has_multiple_writers(func, ref_var) {
                    compile_error!("Mutex[T] は唯一の書込者のみ持てる。RwLock[T] を使用する必要がある");
                }
            }
            _ => {}
        }
    }
}
```

**設計优势：**

| 优势 | 説明 |
|------|------|
| **コンパイル時チェック** | ロック漏olo はコンパイル時に捕捉され、ランタイムデッドロックではない |
| **ゼロランタイムオーバーヘッド** | 競合がない時には Mutex 包装にオーバーヘッドがない |
| **構文簡潔** | `with lock() { ... }` 構文糖でライフサイクルを自動管理 |
| **型安全** | Ref ではなく Atomic を誤用すると型レベルでエラーとなる |

---

## 五、优势まとめ

| 优势 | 説明 |
|------|------|
| **ゼロ伝染性** | 非同期コードと同期コードは構文と型シグネチャにおいて区別なく、「async/await」伝染を完全に根絶 |
| **高性能並列** | 惰性並作グラフと明示的 `spawn` マーキングの組み合わせにより、ランタイムが自動的に並列性を挖掘することを許可すると同時に、プログラマが极限性能最適化のための明確なツールを得られる |
| **メンタモデル簡潔** | 開発者はデータフローとビジネスロジックのみ的关注すればよく、複雑な並行プリミティブとコールバックを理解する必要がない |
| **リファクタリング容易** | 順序ロジックと並行ロジック間の切り替えコストが極めて低く、`spawn {}` 包装を増減するのみでよい |
| **用語直観的** | 「併作関数」「併作ブロック」「併作値」により、技術議論が非常に直観的になる |

---

## 六、実装考量

### 6.1 コンパイラ

- [ ] データフロー分析を実装し、並作グラフを構築
- [ ] `spawn` 戻り値型マーキングの解析と型推論を実装
- [ ] `spawn {}` と `spawn for` をランタイム並列プリミティブにデシュガー
- [ ] アノテーション（`@eager`、`@blocking`）をサポート
- [ ] Void 戻り値型自動即時求値ロジックを実装
- [ ] リソース競合検出を実装
- [ ] Send/Sync 型制約チェックを実装

### 6.2 ランタイム

- [ ] ワークスティールをサポートする並作スケジューラを実装
- [ ] 計算グラフ依存関係認識タスクスケジューリングを実装
- [ ] `Async[T]` 型の自動アンパック機構を実装
- [ ] Void 関数の自動即時実行を実装
- [ ] エラーフグラフ生成と伝播を実装
- [ ] リソースアクセス直列化を実装

### 6.3 デバッグツール ⚠️ 必须

**計算グラフ可視化デバッガ**は複雑なプログラム動作を理解するための鍵です：

| 機能 | 説明 |
|------|------|
| **ノード状態可視化** | 各計算ノードの Pending/Running/Completed 状態を観察 |
| **依存関係表示** | ノード間のデータ依存辺を表示 |
| **タスクフロー追跡** | タスクのスレッド間流转を観察 |
| **パフォーマンスボトルネック特定** | 長いチェーンとホットスポットノードを識別 |
| **エラーフグラフ可視化** | 並行環境下でのエラー伝播パス表示 |

---

## 七、コード示例

### 7.1 基本併作関数

```yaoxiang
use std.net

# 併作関数定義：戻り値型が spawn とマーキング
fetch_user: (Int) -> User spawn = (id) => {
    response = net.HTTP.get("/users/" + id.to_string())
    response.json()
}

fetch_posts: (Int) -> List[Post] spawn = (user_id) => {
    response = net.HTTP.get("/users/" + user_id.to_string() + "/posts")
    response.json()
}

main: () -> Void = () => {
    # 自動的に並列実行（依存関係なし）
    user = fetch_user(123)      # Async[User]
    posts = fetch_posts(123)    # Async[List[Post]]

    # ここで自動的に待機しアンパック
    print(user.name)            # 同期コードと同様に自然
    print(posts.length)
}
```

### 7.2 併作ブロック

```yaoxiang
fetch: (String) -> JSON spawn = (url) => { ... }
parse: (JSON) -> Model spawn = (json) => { ... }

parallel_fetch: () -> (Model, Model) = () => {
    # 併作ブロック：明示的並行領域
    (model_a, model_b) = spawn {
        parse(fetch("https://api1.com/data")),
        parse(fetch("https://api2.com/data"))
    }
    # モデル a と b はここですべて準備完了
    (model_a, model_b)
}
```

### 7.3 併作ループ

```yaoxiang
process_item: (Item) -> Result[Processed, Error] spawn = (item) => { ... }

batch_process: (List[Item]) -> List[Result[Processed, Error]] = (items) => {
    # 併作ループ：データ並列
    results = [spawn for item in items {
        process_item(item)
    }]
    # results はここですべての結果を含む List
    results
}
```

---

> *"万物并作，吾以观復。"*
> —— 《易・復卦》
>
> 併作モデルは惰性求値の宣言的洗練さと高性能並行処理の要求を組み合わせ、システムプログラミングに安全でありながら極めて表現力のある全新パラダイムを提供することを目指します。
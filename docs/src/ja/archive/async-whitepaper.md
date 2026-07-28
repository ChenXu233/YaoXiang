> **⚠️ 注意：このドキュメントは古い版本であり、参考用です。**
>
> このドキュメントの内容はすでに適用外となっています。最新のドキュメントを参照してください。

# **《Spawn：遅延評価に基づく透過的非同期並行モデル》技術白書**

## 🏛️ 一、核心定義：Spawnモデル

**Spawnモデル**は、《易·復卦》「万物并作，吾以観復」に由来します。プログラミング言語の並行パラダイムであり、開発者が同期的で逐次的な思考でロジックを記述でき、言語ランタイムがその計算ユニットを万物并作のように自動かつ効率的に並行実行させ、最終的に統一的に協調させます。

### コア設計理念：デフォルト遅延評価 + spawn型マーキング

| 設計原則               | 説明                                                              |
| ---------------------- | ----------------------------------------------------------------- |
| **デフォルト遅延評価** | すべての関数はデフォルトで遅延（Hasell類似）、Lazy[T]を返す       |
| **コア数設定**         | スクリプトヘッダー `// @cores: N` で自動並列化を有効化            |
| **spawn型マーキング**  | `-> T spawn` で関数を厳密に非同期・並行可能とマーク               |
| **混合評価モード**     | `@eager`（デコレータ、強制即時）、`@auto`（デコレータ、並列維持） |
| **Void自動即時**       | Voidを返す関数は自動即時評価（副作用は実行する必要があるため）    |

### コア三原則

| コア原則     | 解説                                                             |
| ------------ | ---------------------------------------------------------------- |
| **同期構文** | 見たままの逐次コード、書いたままの実行フロー                     |
| **並行本質** | ランタイムが自動的に並列性を抽出し、データ依存から並行機会を発掘 |
| **統一協調** | 結果が必要時に自動的に集約され、論理的正確性を保証               |

**これは2つの根本的な転換によって達成されます：**

1. **「制御フロー」を「データフロー」に転換**：プログラムは純粋関数的な遅延評価データフローグラフとして扱われます
2. **「非同期伝染」を「依存解析」に転換**：非同期性は関数のシグネチャ эффектではなくなり、ランタイムがデータ依存点で自動的に実行を待つ操作となります

---

## 📚 二、用語体系：統一された概念マップ

"Spawn"を中心に、明確で一貫性のある用語体系を構築し、すべての設計を 연결합니다：

| 公式用語              | 対応構文/概念                | 解説                                                                                                                     |
| --------------------- | ---------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| **spawn関数**         | `-> T spawn`                 | 戻り値の型マーキングであり、"spawn"並行実行に参加できる厳密に計算ユニットであることを示す                                |
| **spawnブロック**     | `spawn { a(), b() }`         | 開発者が明示的に宣言する並行領域であり、ブロック内のタスクは"spawn"実行される                                            |
| **spawnループ**       | `spawn for x in xs { ... }`  | データ並列パラダイムであり、ループ体がすべてのデータ要素で"spawn"実行される                                              |
| **spawn値**           | `Async[T]` プロキシ型        | 実行中の"未来値"であり、使用時に自動的に"作"終わるまで待つ                                                               |
| **spawnグラフ**       | 遅延計算グラフ（DAG）        | "spawn"が発生する舞台であり、すべての計算ユニット間の依存関係と並列関係を記述                                            |
| **spawnスケジューラ** | ランタイムタスクスケジューラ | "万物"を調整し、正しいタイミングで"spawn"させる Intelligent Center                                                       |
| **エラーグラフ**      | Error Graph                  | 並行環境下でのエラー伝播パス可視化であり、呼び出しスタックに似ているがDAG内のエラー流向を示す                            |
| **リソース競合**      | Resource Conflict            | 複数のタスクが同時に同じ書き込み可能リソースにアクセスする際の競合であり、コンパイル時に検出され自動的にシリアル化される |

> **技術交流例**："ここではspawnブロックを使って2つのspawn関数を並行呼び出し，它们的spawn値を自動的に取得できます。"

---

## 三、三層並行アーキテクチャ：段階的透過性

### 3.1 アーキテクチャ概要

Spawnモデルは**3段階の漸進的並行抽象**を提供し、異なるスキルレベルの開発者が適切な使用パターンを見つけられるようにします：

| レベル | モード          | 構文マーキング     | 実行方式           | 制御性 | 適用シナリオ                             |
| ------ | --------------- | ------------------ | ------------------ | ------ | ---------------------------------------- |
| **L1** | `@blocking`同期 | `@blocking`        | 完全逐次実行       | 最高   | デバッグ新手学習、重要なコードセクション |
| **L2** | 明示的spawn     | `spawn`            | 開発者制御可能并行 | 中     | 中級ユーザー、微細な並行制御が必要       |
| **L3** | 完全透過        | なし（デフォルト） | 自動最適並列       | 最低   | 上級者、自動並列最適化                   |

### 3.2 L1: `@blocking` 同期モード

**コア特性**：すべての並列最適化を無効化し、完全逐次実行，便于调试和理解。

```yaoxiang
# L1: @blocking 同期モード（註釈は戻り値の型の後に配置）
fetch_sync: (String) -> JSON @blocking = (url) => {
    HTTP.get(url).json()
}

main: () -> Void @blocking = () => {
    # 厳密に逐次実行、任何並行なし
    data1 = fetch_sync("https://api.example.com/data1")
    data2 = fetch_sync("https://api.example.com/data2")
    process(data1, data2)
}
```

### 3.3 L2: 明示的 spawn 並行

**コア特性**：開発者が明示的に並行可能なユニットをマークし、制御可能な的同时に並行enefitsを得る。

```yaoxiang
# L2: 明示的 spawn 並行
fetch_data: (String) -> JSON spawn = (url) => {
    HTTP.get(url).json()
}

process_users_and_posts: () -> Void spawn = () => {
    users = fetch_data("https://api.example.com/users")
    posts = fetch_data("https://api.example.com/posts")
    # users と posts は自動的に並列実行
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

### 3.4 L3: 完全透過（デフォルト）

**コア特性**：マーキング不要、コンパイラが自動的に依存関係を分析して最適な並列実行プランを生成。

```yaoxiang
# L3: 完全透過（デフォルトモード）
heavy_calc: (Int) -> Int = (n) => {
    fibonacci(n)
}

auto_parallel: (Int) -> Int = (n) => {
    # システム自動分析：a, b, c は依存関係なく、完全並列可能
    a = heavy_calc(1)
    b = heavy_calc(2)
    c = heavy_calc(3)
    a + b + c
}
```

### 3.5 手動制御アノテーション

| アノテーション | 動作         | 使用シナリオ                       |
| -------------- | ------------ | ---------------------------------- |
| `@eager`       | 強制即時評価 | 即座に結果を取得する必要がある計算 |

---

## 二、核心概念

### 2.1 spawnグラフ：万物并作のステージ

すべてのプログラムはコンパイル時に**有向非巡回計算グラフ（DAG）**に変換され、これは**spawnグラフ**と呼ばれます。

| 要素       | 説明                                                                   |
| ---------- | ---------------------------------------------------------------------- |
| **ノード** | 表达式計算ユニットを表現                                               |
| **エッジ** | データ依存関係を表現（A → B は B が A の結果に依存することを意味する） |
| **遅延**   | ノードは出力が**実際に必要**とされる拖のみ評価される                   |

### 2.2 デフォルト遅延評価

すべての関数はデフォルトで**遅延評価**戦略を採用します：

```yaoxiang
# スクリプトヘッダーで並列コア数を設定
# @cores: 4

# すべての関数はデフォルトで遅延評価（デフォルトで並行可能）
heavy_computation: (Int) -> Int = (x) => {
    # この関数は즉시実行されない
    # 結果を使用する拖のみ実行される
    fibonacci(x)
}

main: () -> Void = () => {
    # heavy_computation は Int を返し、型は Lazy[Int]
    result = heavy_computation(100)

    # ここで、result が加算に使用され、評価がトリガーされる
    # システムは自動的に最適なタイミングで並列実行を見つける
    total = result + heavy_computation(200)
}
```

### 2.3 混合評価アノテーション（デコレータスタイル）

YaoXiangのアノテーションはPythonのデコレータ类似しており、関数や式の動作を変更するために使用されます：

| アノテーション（デコレータ） | 動作                                             |
| ---------------------------- | ------------------------------------------------ |
| `@eager`                     | **デコレータ**：強制即時評価、즉시実行           |
| `@auto`                      | **デコレータ**：並列維持（デフォルト、省略可能） |

**Void自動即時ルール：**
Voidを返す関数は自動的に即時評価されます（任何アノテーション不要）。理由は副作用が実行される必要があるためです。

```yaoxiang
# @eager デコレータ：強制即時評価
heavy_computation: (Int) -> Int = (x) => {
    fibonacci(x)
}

# Void を返す関数は自動的に即時評価（副作用関数）
log: (String) -> Void = (message) => {
    print(message)
}

main: () -> Void = () => {
    # log は自動的に即時実行、Void を返すため
    log("Processing started")

    # @eager を使用して強制即時実行
    @eager heavy_computation(100)
}
```

### 2.4 spawn値：Async[T] 遅延プロキシ型

戻り値の型が `-> T spawn` とマークされた関数は、即座に `Async[T]`
型の値を返します。これを**spawn値**と呼びます。

```yaoxiang
# spawn関数：戻り値の型が -> JSON spawn とマーク
# これは厳密にspawn実行可能な計算ユニットであることを示す
fetch: (String) -> JSON spawn = (url) => {
    HTTP.get(url).json()
}

main: () -> Void = () => {
    # fetch は spawn値 Async[JSON] を返す
    # しかし、使用時に追加の構文は不要
    data = fetch("https://api.example.com")  # Async[JSON]

    # ここで、data は自動的に待って JSON にアンパックされる
    print(data.name)  # 同期コードのように自然
}
```

#### spawn値のコア特性

| 特性                 | 説明                                                                                                           |
| -------------------- | -------------------------------------------------------------------------------------------------------------- |
| **構文透過**         | `Async[T]` は型システムで T のサブタイプであり、T が期望されるどんなコンテキストでも使用可能                   |
| **オンデマンド待機** | T 型の具体的な値を使用する必要がある場合（例：フィールドアクセス、算術演算）、ランタイムは自動的に停止して待機 |
| **エラー伝播**       | 内部的には実際には `Result<T, E>` であり、エラーはデータフローに沿って自然に伝播                               |

### 2.7 spawn構成：「修飾子」から「型マーキング」への転換

`spawn`キーワードは同期思考と非同期実装を接続する唯一の架け橋であり、3つの意味を持っています：

| 構文形式            | 公式用語      | セマンティクス                                                                  | ランタイム動作                                                                                                       |
| :------------------ | :------------ | :------------------------------------------------------------------------------ | :------------------------------------------------------------------------------------------------------------------- |
| **`-> T spawn`**    | spawn関数     | 戻り値の型マーキングであり、厳密にspawnに参加できる計算ユニットであることを示す | その呼び出しは `Async[T]` を返し、spawnグラフノードの作成を意味する                                                  |
| **`spawn { ... }`** | spawnブロック | 明示的に宣言された並行領域                                                      | ランタイムはブロック内の各式を**積極的に**独立したタスクとして並行実行し、ブロック終了時に暗黙的にすべての結果を待つ |
| **`spawn for`**     | spawnループ   | データ並列ループ                                                                | ループ体を複数の並列タスクに変換し、自動的にデータシャーディング、scheduling、结果収集を行う                         |

---

## 三、動作原理：コードから実行へ

### 3.1 コンパイル時：spawnグラフの構築

```yaoxiang
# spawn関数定義：戻り値の型が spawn とマーク
fetch: (String) -> String spawn = (url) => { ... }
parse: (String) -> Model spawn = (data) => { ... }

process: () -> Report = () => {
    # コンパイラはここでspawnグラフノードを作成
    data_a = fetch("url1")  # ノード A: Async[String]
    data_b = fetch("url2")  # ノード B: Async[String]

    # spawnブロック：明示的な並行領域
    (model_a, model_b) = spawn {
        parse(data_a),  # ノード C: A に依存
        parse(data_b)   # ノード D: B に依存
    }

    # 最終集約ノード
    generate_report(model_a, model_b)  # ノード E
}
```

**コンパイラ操作：**

1. ソースコードを解析し、グローバルspawnグラフを構築
2. 各式に対して計算ノードを作成
3. データ依存関係を分析し、エッジ関係を確立
4. `spawn { }` と `spawn for` ブロック内のサブグラフには **「並列評価」** マークが付けられる

### 4.2 ランタイム：spawnスケジューラ

intelligenteな、工作輪取り。支持する**spawnスケジューラ**がspawnグラフの実行を担当します：

```rust
// spawnスケジューラのコアロジック
impl FlowScheduler {
    fn execute_node(&self, node_id: NodeId) {
        let node = self.get_node(node_id);

        match &node.kind {
            NodeKind::AsyncCompute => {
                // spawn関数：コrinaプールに提交
                self.submit_async(node_id);
            }
            NodeKind::ParallelBlock => {
                // spawnブロック：積極的にすべての直接サブノードを並行実行
                self.submit_parallel(node_id);
            }
            NodeKind::DataParallel { iterator, body } => {
                // spawnループ：自動シャーディング
                self.submit_data_parallel(node_id, iterator, body);
            }
            _ => { /* 同期実行 */ }
        }
    }
}
```

#### 実行フロー

```
1. [E] を評価するために [C] と [D] が必要
2. [C] は [A] に依存、[D] は [B] に依存
3. spawnスケジューラは [A] と [B] に依存関係がないことを発見 → 即座に並列実行
4. [A]、[B] 完了後、spawnブロックマークのため → 即座に [C] と [D] を並列実行
5. [C]、[D] 完了後、[E] を実行
```

**主要機構：**

| 機構             | 説明                                                              |
| ---------------- | ----------------------------------------------------------------- |
| **遅延トリガー** | 最終結果の要求から実行を開始し、逆方向に依存関係を追踪            |
| **自動待機**     | `Async[T]` に遭遇すると自動的に停止し、他の準備完了タスクを実行   |
| **工作輪取り**   | スレッドが他のスレッドキューからタスクを奪い取り、CPU利用率を向上 |

---

## 四、主要機構の詳細

### 4.1 副作用と評価保証

純粋な遅延評価は副作用（例：ログ、書き込み）が永不実行になる可能导致します。Spawnモデルは**戻り値に基づく自動推論**を採用しています：

| ルール       | 条件                            | 動作                                           |
| ------------ | ------------------------------- | ---------------------------------------------- |
| **ルール一** | Void を返す関数                 | **自動即時評価**（副作用は実行する必要がある） |
| **ルール二** | `@eager` デコレータを使用した式 | 戻り値の型に関係なく、**強制即時評価**         |
| **ルール三** | Void 以外の型を返す             | **遅延評価**（デフォルト）                     |

```yaoxiang
# Void を返す関数は自動的に即時実行（副作用）
log: (String) -> Void = (message) => {
    print(message)
}

# @eager デコレータ：強制即時評価
cache_compute: (Int) -> Int = (x) => {
    # Int を返としても、強制的に즉시実行
    expensive_calculation(x)
}

main: () -> Void = () => {
    # log は自動的に即時実行（Void を返すため）
    log("Processing started")

    # @eager で強制即時実行
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
# 標準Result型（統一構築子構文）
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

Rustスタイルの`?`演算子を採用し透過的なエラー伝播を実現：

```yaoxiang
# Rustスタイル ? 演算子
process() -> Result[Data, Error] = {
    data = fetch_data()?      # 自動待機およびエラー検査
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

#### エラーグラフ可視化

エラーグラフは呼び出しスタックに似ていますが、DAG内のエラー伝播パスを表示します：

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
│ 捕获位置: calculate (第42行)                                │
└─────────────────────────────────────────────────────────────┘
```

#### エラー処理のベストプラクティス

```yaoxiang
# 複数のエラーが発生する可能性のある操作を組み合わせる
batch_process: ([String]) -> Result[[String], Error] = (items) => {
    results = items.map(item => {
        process_item(item)?
    })
    ok(results)
}

# with? 構文糖衣（将来の機能）
validate_user: (User) -> Result[ValidatedUser, ValidationError] = (user) => {
    name = user.name.with?(validate_name)?
    email = user.email.with?(validate_email)?
    ok(ValidatedUser(name, email))
}
```

### 4.3 純粋関数と `@blocking` 同期保証

**核心洞察：純粋関数はブロックしない！**

理由は：

- 純粋関数にはI/Oがなく、CPU計算のみ
- 計算が長くてもスケジューラをブロックせず、CPU時間のみ占有

**実行戦略：**

| 関数型                          | 実行戦略                       | ブロック？            |
| ------------------------------- | ------------------------------ | --------------------- |
| 純粋関数（I/Oなし）             | 同期実行                       | いいえ（CPU占有のみ） |
| 非同期関数（`Async[T]` を返す） | 非同期実行                     | いいえ                |
| `@blocking` アノテーション関数  | 同期実行、内部スケジューリング | いいえ                |

**`@blocking` アノテーション：同期実行保証**

`@blocking` アノテーションは関数が同期姿勢で実行されることを保証します：

- 関数が戻る時に結果はすでに準備できている
- 内部に非同期呼び出しがある場合は、内部で完了させてから戻る
- 同期セマンティクスが必要だが内部に非同期操作が含まれるシナリオに適している

```yaoxiang
# @blocking：同期実行，内部非同期スケジューリング完了後に戻る
heavy_compute: (List[Int]) -> Int = (data) => {
    # 内部に非同期操作があるかもしれないが、戻る前に完了
    processed = data.map(x => async_transform(x))
    processed.sum()
}

# 通常の非同期関数：Async[T] を返す
fetch_user: (Int) -> Async[User] = (id) => {
    database.query("SELECT * FROM users WHERE id = ?", id)
}

# 純粋関数：自動同期（I/Oなし）
factorial: (Int) -> Int = (n) => {
    if n <= 1 then 1 else n * factorial(n - 1)
}

main: () -> Void = () => {
    # @blocking 関数：同期実行
    result = heavy_compute([1, 2, 3, 4, 5])  # 即座に結果を返す
    print(result)  # 15

    # 非同期関数：Async[User] を返す
    user = fetch_user(123)  # Async[User]
    print(user.name)  # 自動待機およびアンパック
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
            // 非同期関数：asyncスケジューラに提交
            async_runtime.submit(node);
        }
        ExecutionMode::Blocking => {
            // @blocking 関数：同期実行，内部で非同期操作をスケジューリング
            execute_blocking(node);
        }
    }
}

fn execute_blocking(node: &DAGNode) {
    // 関数本体を実行
    let result = node.execute_body();

    // 内部のすべての非同期操作を収集
    let internal_async_ops = collect_async_ops(node);

    // 内部のすべての非同期操作の完了を待つ
    if !internal_async_ops.is_empty() {
        async_runtime.wait_all(internal_async_ops);
    }

    // 結果を返す
    result
}
```

**設計優位性：**

- **簡潔**：複雑なeffectシステム不要
- **柔軟**：`@blocking` は任意であり、同期セマンティクスが必要な時に使用
- **効率的**：純粋関数は自動的に同期実行
- **安全**：メインスケジューラは永不ブロック

### 4.4 リソース競合検出

コンパイル時にリソースアクセスパターンを分析し、競合操作を自動的にシリアル化：

```
リソース競合ルールマトリックス：
╔═══════════╦══════════╦══════════╗
║   アクセス    ║   読取     ║    書込    ║
╠═══════════╬══════════╬══════════╣
║   読取      ║  並行可能  ║  シリアル化  ║
║   書込      ║  シリアル化  ║  シリアル化  ║
╚═══════════╩══════════╩══════════╝
```

**コンパイル時分析例**：

```rust
// コンパイル時のリソースアクセス分析
struct ResourceAccess {
    reads: Set<ResourceId>,   // 読み取るリソース
    writes: Set<ResourceId>,  // 書き込むリソース
}

// 例
file1 = open("a.txt")  // リソース1：読取
file2 = open("b.txt")  // リソース2：読取
// file1 読取 と file2 読取 → 並行可能

file3 = open("c.txt")  // リソース3：書込
// file1 読取 と file3 書込 → シリアル化
// file2 読取 と file3 書込 → シリアル化
```

**コード例**：

```yaoxiang
# コンパイラが自動的に競合を検出してシリアル化
process_files: () -> Void = () => {
    file_a = open("a.txt")  # リソース1：読取
    file_b = open("b.txt")  # リソース2：読取
    # file_a と file_b は両方とも読取のみ → 並行可能

    file_c = open("c.txt")  # リソース3：書込
    # file_a 読取 と file_c 書込 → シリアル化
    # file_b 読取 と file_c 書込 → シリアル化
}

# 複数の書込操作は自動的にシリアル化
write_logs: () -> Void = () => {
    log1 = open_log("log1.txt")  # リソース1：書込
    log2 = open_log("log2.txt")  # リソース2：書込
    # log1 と log2 は異なるリソース → 並行可能
}
```

### 4.5 並行競合制御：型システムによるAtomic性保証

**核心思想：型システムで並行アクセスデータをマークし、コンパイラが同期正確性を検査する。**

**型マーキング体系：**

| 型          | セマンティクス     | 並行安全    | 説明                                                 |
| ----------- | ------------------ | ----------- | ---------------------------------------------------- |
| `T`         | 不変データ         | ✅ 安全     | デフォルト型、複数のタスクが読み取り可能で競合なし   |
| `Ref[T]`    | 可変参照           | ⚠️ 同期必要 | 並行変更可能とマーク、ロック使用をコンパイル時に検査 |
| `Atomic[T]` | 原子型             | ✅ 安全     | низкоуровневые atomic操作、ロック不要並行            |
| `Mutex[T]`  | 相互排除ロック包装 | ✅ 安全     | 自動ロック/ロック解除、コンパイル保証                |
| `RwLock[T]` | 読み書きロック包装 | ✅ 安全     | 読み取り多用書き込み少量シナリオの最適化             |

**型安全性保証：**

```yaoxiang
# デフォルト不変 - 自然に競合なし
data: List[Int] = [1, 2, 3, 4, 5]
spawn for x in data { process(x) }  # ✅ 安全、読取のみで競合なし

# 可変参照 - 同期が必要
counter: Ref[Int] = Ref.new(0)

# 錯誤例：ロックなしで Ref にアクセス（コンパイルエラー）
spawn for i in 1..10 {
    # ❌ コンパイルエラー：Ref は同期プリミティブ経由でアクセスする必要がある
    counter.value = counter.value + i
}

# 正しい例：with 構文糖衣を使用して自動ロック
spawn for i in 1..10 {
    # ✅ with ブロックは自動的にロックを取得および解放
    with counter.lock() {
        counter.value = counter.value + i
    }
}

# 原子型 - ロック不要並行
atomic_counter: Atomic[Int] = Atomic.new(0)
spawn for i in 1..10 {
    # ✅ atomic操作、ロック不要で安全
    atomic_counter.fetch_add(i)
}
```

**Mutex[T] 型 - コンパイル時ロック保証：**

```yaoxiang
# 相互排除ロックで包装されたデータを作成
shared_state: Mutex[Map[String, Int]] = Mutex.new(Map.empty())

# with 構文糖衣を使用（Go の defer 类似）
main: () -> Void = () => {
    spawn for i in 1..100 {
        # with は自動的にロックを取得し、ブロック終了時に自動解放
        with shared_state.lock() {
            # クリティカルセクション：Mutex で保護
            current = shared_state.get("count").or(0)
            shared_state.set("count", current + 1)
        }
    }

    # すべてのタスク完了を待つ
    print(shared_state.get("count"))  # 100
}
```

**型推論とロック検査：**

```rust
// コンパイラはコンパイル時にロックを検査
fn compile_check_locks(func: &Function) {
    for node in func.nodes {
        match node {
            NodeKind::ReadRef(ref_var) => {
                // ロック保護範囲内かどうかを確認
                if !is_inside_lock_guard(ref_var) {
                    compile_error!("Ref アクセスは lock() 保護範囲内である必要がある");
                }
            }
            NodeKind::WriteRef(ref_var, _) => {
                // 二重検査：ロック + 一意の書き込み者
                if !is_inside_lock_guard(ref_var) {
                    compile_error!("Ref 変更は lock() 保護範囲内である必要がある");
                }
                if has_multiple_writers(func, ref_var) {
                    compile_error!("Mutex[T] は書き込み者が1つのみ必要で、RwLock[T] を使用する必要がある");
                }
            }
            _ => {}
        }
    }
}
```

**設計優位性：**

| 優位性                       | 説明                                                                 |
| ---------------------------- | -------------------------------------------------------------------- |
| **コンパイル時検査**         | ロック漏れはコンパイル時にキャプチャされ、実行時デッドロックではない |
| **ゼロ実行時オーバーヘッド** | ロック包装は無競合時にオーバーヘッドなし                             |
| **構文簡潔**                 | `with lock() { ... }` 構文糖衣でライフサイクルを自動管理             |
| **型安全**                   | Ref ではなく Atomic を誤使用すると型レベルでエラーが発生             |

---

## 五、優位性のまとめ

| 優位性                   | 説明                                                                                                                                                             |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **ゼロ伝染性**           | 非同期コードと同期コードは構文と型シグネチャに差异なく、"async/await"伝染を完全に根絶                                                                            |
| **高性能並列**           | 遅延spawnグラフと明示的な `spawn` マーキングを組み合わせ、ランタイムが自動的に並列性を発掘できるように的同时に、程序员が极限性能最適化のための明確なツールを持つ |
| **メンタモデル簡潔**     | 開発者はデータフローとビジネスロジックにのみ焦点を当てる必要があり、複雑な並行プリミティブとコールバックを理解する必要はない                                     |
| **易于リファクタリング** | 逐次ロジックと並行ロジックの間の切り替えコストは非常に低く、`spawn {}` 包装を増減するだけです                                                                    |
| **用語直感的**           | "spawn関数"、"spawnブロック"、"spawn値"により、技術議論が非常に直感的になる                                                                                      |

---

## 六、実装上の考慮事項

### 6.1 コンパイラ

- [ ] データフロー分析を実装し、spawnグラフを構築
- [ ] `spawn` 戻り値型マーキングの解析と型推論を実装
- [ ] `spawn {}` と `spawn for` をランタイム並列プリミティブにdesugar
- [ ] アノテーション（`@eager`、`@blocking`）をサポート
- [ ] Void 戻り値型自動即時評価ロジックを実装
- [ ] リソース競合検出を実装
- [ ] Send/Sync 型制約検査を実装

### 6.2 ランタイム

- [ ] 工作輪取り 支持するspawnスケジューラを実装
- [ ] 計算グラフ依存認識タスクスケジューリングを実装
- [ ] `Async[T]` 型の自動アンパック機構を実装
- [ ] Void 関数の自動即時実行を実装
- [ ] エラーグラフ生成と伝播を実装
- [ ] リソースアクセスシリアル化を実装

### 6.3 デバッグツール ⚠️ 必須

**計算グラフ可視化デバッガ**は複雑なプログラム動作を理解するための鍵です：

| 機能                               | 説明                                                |
| ---------------------------------- | --------------------------------------------------- |
| **ノード状態可視化**               | 各計算ノードの Pending/Running/Completed 状態を観察 |
| **依存関係表示**                   | ノード間のデータ依存エッジを表示                    |
| **タスクフロー追跡**               | タスクが各スレッド間を流れる様子を観察              |
| **パフォーマンスボトルネック特定** | 長いチェーンとホットスポットノードを識別            |
| **エラーグラフ可視化**             | 並行環境下でのエラー伝播パス表示                    |

---

## 七、コード例

### 7.1 基本的なspawn関数

```yaoxiang
use std.net

# spawn関数定義：戻り値の型が spawn とマーク
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

    # ここで自動的に待機してアンパック
    print(user.name)            # 同期コードのように自然
    print(posts.length)
}
```

### 7.2 spawnブロック

```yaoxiang
fetch: (String) -> JSON spawn = (url) => { ... }
parse: (JSON) -> Model spawn = (json) => { ... }

parallel_fetch: () -> (Model, Model) = () => {
    # spawnブロック：明示的な並行領域
    (model_a, model_b) = spawn {
        parse(fetch("https://api1.com/data")),
        parse(fetch("https://api2.com/data"))
    }
    # モデル a と b はここで両方とも準備完了
    (model_a, model_b)
}
```

### 7.3 spawnループ

```yaoxiang
process_item: (Item) -> Result[Processed, Error] spawn = (item) => { ... }

batch_process: (List[Item]) -> List[Result[Processed, Error]] = (items) => {
    # spawnループ：データ並列
    results = [spawn for item in items {
        process_item(item)
    }]
    # results はここで List であり、すべての処理結果を含む
    results
}
```

---

> _"万物并作，吾以観復。"_ —— 《易·復卦》
>
> Spawnモデルは遅延評価の宣言的な優雅さと高性能並行の必要性を組み合わせ、系统编程に安全かつ非常に表現力のある新たなパラダイムを提供することを目指します。

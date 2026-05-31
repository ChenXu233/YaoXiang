> **⚠️ 注意：このドキュメントはすでに古いものであり、参考用です。**
>
> このドキュメントの内容はすでに適用外となっています。最新のドキュメントを参照してください。

# **《Spawn：惰性評価ベースの非同期コンカレンシモデル》技術ホワイトペーパー**

## 🏛️ 一、コア定義：Spawnモデル

**Spawnモデル**は、《易・復卦》の「万物spawn action、吾れ以って復を観察す」に由来します。これはプログラミング言語のコンカレンシーパラダイムであり、開発者が同期・順序的な思考でロジックを記述できるようにし、言語ランタイムがその中の計算ユニットを万物spawn般に自動かつ効率的にコンカレントに実行させ、最終的に統一的に協調させます。

### コア設計理念：デフォルト遅延評価 + spawn 型マーキング

| 設計原則 | 説明 |
|----------|------|
| **デフォルト遅延評価** | すべての関数はデフォルトで遅延（ Haskell のようなもの）であり、Lazy[T] を返す |
| **コア数設定** | スクリプトヘッダで `// @cores: N` を宣言すると自動並列化が有効になる |
| **spawn 型マーキング** | `-> T spawn` で関数をstrictに非同期・コンカレント可能とマーク，其余はデフォルトでコンカレント可能 |
| **混合評価モード** | `@eager`（デコレータ、強制即時評価）、`@auto`（デコレータ、並列維持） |
| **Void 自動即時評価** | Void を返す関数は自動的に即時評価（副作用は実行されなければならない） |

### コア3原則

| コア原則 | 説明 |
|----------|------|
| **同期構文** | 記述した通りの順序コード、記述した通りの実行フロー |
| **コンカレンシー本質** | ランタイムが自動的に並列性を抽出し、データ依存関係からコンカレンシーの機会を発見する |
| **統一協調** | 必要に応じて結果が自動的に集約され、論理的正確性が保証される |

**これを実現するために、2つの根本的な変革を採用しています：**

1. **「制御フロー」を「データフロー」に変換**：プログラムは純粋関数的な遅延評価データフローグラフとして扱われる
2. **「非同期伝染」を「依存関係解析」に変換**：非同期性は関数シグネチャのエフェクト不再り、ランタイムがデータ依存点で自動的に実行を待機する操作となる

---

## 📚 二、用語体系：統一された概念マップ

「spawn」を中心に、明確で自己整合性のある用語体系を構築し、すべての設計をつなげます：

| 公式用語 | 対応構文/概念 | 説明 |
|----------|---------------|------|
| **spawn関数** | `-> T spawn` | 戻り値の型マーキングであり、「spawn」コンカレント実行に参加できるstrictな計算ユニットを示す |
| **spawnブロック** | `spawn { a(), b() }` | 開発者が明示的に宣言したコンカレンシードメインであり、ブロック内のタスクは「spawn」実行される |
| **spawnループ** | `spawn for x in xs { ... }` | データ並列パラダイムであり、ループ体がすべてのデータ要素で「spawn」実行される |
| **spawn値** | `Async[T]` 代理型 | 実行中の「future値」であり、使用時に自動的にその「実行」が完了するのを待つ |
| **spawnグラフ** | 遅延計算グラフ（DAG） | 「spawn」が発生する舞台であり、すべての計算ユニット間の依存関係と並列関係を記述する |
| **spawnスケジューラ** | ランタイムタスクスケジューラ | 「万物」を調整し、正しいタイミングで「spawn」させるIntelligent中枢 |
| **エラーチャート** | Error Graph | コンカレント環境でのエラー伝播パス可視化であり、コールスタックに似ているがDAG内のエラー流向を示す |
| **リソース衝突** | Resource Conflict | 複数のタスクが同時に同じ書き込み可能なリソースにアクセスする際の衝突であり、コンパイル時に検出され自動的に直列化される |

> **技術交流の例**：「ここでspawnブロックを使って2つのspawn関数をコンカレントに呼び出すと、自動的にそのspawn値が得られます。」

---

## 三、三層コンカレンシーアーキテクチャ：漸進的透明性

### 3.1 アーキテクチャ概要

Spawnモデルは**3層の段階的コンカレンシー抽象化**を提供し、異なるスキルレベルの開発者が適切な使用パターンを見つけられるようにします：

| レベル | モード | 構文マーキング | 実行方式 | 制御性 | 適用シナリオ |
|------|------|----------|--------|------|------------|
| **L1** | `@blocking`同期 | `@blocking` | 完全順序実行 | 最高 | デバッグ新手学習、关键コード段 |
| **L2** | 明示的spawn | `spawn` | 開発者制御コンカレンシー | 中 | 中級ユーザー、精细なコンカレンシー制御が必要 |
| **L3** | 完全透明 | なし（デフォルト） | 自動最適並列 | 最低 | 上級者、自動並列最適化 |

### 3.2 L1: `@blocking` 同期モード

**コア特性**：すべてのコンカレンシー最適化を無効化し、完全順序実行し、デバッグと理解を容易にする。

```yaoxiang
# L1: @blocking 同期モード（注解は戻り値の型の後に配置）
fetch_sync: (String) -> JSON @blocking = (url) => {
    HTTP.get(url).json()
}

main: () -> Void @blocking = () => {
    # 厳密な順序実行、コンカレンシーなし
    data1 = fetch_sync("https://api.example.com/data1")
    data2 = fetch_sync("https://api.example.com/data2")
    process(data1, data2)
}
```

### 3.3 L2: 明示的 spawn コンカレンシー

**コア特性**：開発者がコンカレント可能なユニットを明示的にマークし、制御可能性を維持しながらコンカレンシー効果を得る。

```yaoxiang
# L2: 明示的 spawn コンカレンシー
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

# 明示的コンカレンシーブロック
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

**コア特性**：マーキングは一切不要で、コンパイラが自動的に依存関係を解析し最適な並列実行計画を生成する。

```yaoxiang
# L3: 完全透明（デフォルトモード）
heavy_calc: (Int) -> Int = (n) => {
    fibonacci(n)
}

auto_parallel: (Int) -> Int = (n) => {
    # システムが自動解析：a, b, c は依存関係なし、完全並列可能
    a = heavy_calc(1)
    b = heavy_calc(2)
    c = heavy_calc(3)
    a + b + c
}
```

### 3.5 手動制御デコレータ

| デコレータ | 動作 | 使用シナリオ |
|------|------|------------|
| `@eager` | 強制即時評価 | 即座に結果を取得する必要がある計算 |

---

## 二、コア概念

### 2.1 Spawnグラフ：万物spawnの舞台

すべてのプログラムはコンパイル時に**有向非環計算グラフ（DAG）**に変換され、これは**Spawnグラフ**と呼ばれます。

| 要素 | 説明 |
|------|------|
| **ノード** | 式計算ユニットを表現 |
| **エッジ** | データ依存関係を示す（A → B は B が A の結果を依存することを示す） |
| **遅延** | ノードはその出力が**本当に必要**になったときにのみ評価される |

### 2.2 デフォルト遅延評価

すべての関数はデフォルトで**遅延評価**戦略を採用します：

```yaoxiang
# スクリプトヘッダで並列コア数を設定
# @cores: 4

# すべての関数はデフォルトで遅延評価（デフォルトでコンカレント可能）
heavy_computation: (Int) -> Int = (x) => {
    # この関数は即座には実行されない
    # 結果が使用されたときにのみ実行される
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

### 2.3 混合評価デコレータ（デコレータスタイル）

YaoXiang のデコレータは Python のデコレータに類似しており、関数や式の動作を変更するために使用されます：

| デコレータ | 動作 |
|----------------|------|
| `@eager` | **デコレータ**：強制即時評価、即座に実行 |
| `@auto` | **デコレータ**：並列維持（デフォルト、省略可能） |

**Void 自動即時評価ルール：** Void を返す関数は自動的に即時評価されます（デコレータ不要）。なぜならば副作用は実行されなければならないからです。

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
    # log は自動的に即時実行される（Void を返すため）
    log("Processing started")

    # @eager を使用して強制即時評価
    @eager heavy_computation(100)
}
```

### 2.4 Spawn値：Async[T] 遅延代理型

戻り値の型が `-> T spawn` とマーキングされた関数は、即座に `Async[T]` 型の値を返します。これを**spawn値**と呼びます。

```yaoxiang
# spawn関数：戻り値の型が -> JSON spawn とマーキングされている
# strictにコンカレント実行可能な計算ユニットであることを示す
fetch: (String) -> JSON spawn = (url) => {
    HTTP.get(url).json()
}

main: () -> Void = () => {
    # fetch が返すのはspawn値 Async[JSON]
    # しかし、使用時に追加の構文は不要
    data = fetch("https://api.example.com")  # Async[JSON]

    # ここで、data は自動的に待機され JSON にアンラップされる
    print(data.name)  # 同期コードのように自然に
}
```

#### Spawn値のコア特性

| 特性 | 説明 |
|------|------|
| **構文透明** | `Async[T]` は型システムでは `T` のサブタイプであり、`T` が期待される任意のコンテキストで使用可能 |
| **オンデマンド待機** | `T` 型の具体的な値を使用する必要があるとき（フィールドアクセス、算術演算など）、ランタイムは自動的にサスペンドして待機 |
| **エラー伝播** | 内部的には実際に `Result<T, E>` であり、エラーはデータフローに沿って自然に伝播する |

### 2.7 Spawn構成：「修飾子」から「型マーキング」への移行

`spawn` キーワードは同期思考と非同期実装を接続する唯一の橋渡しであり、3重のセマンティクスを持っています：

| 構文形式 | 公式用語 | セマンティクス | ランタイム動作 |
|:---------|:---------|:-----|:----------|
| **`-> T spawn`** | spawn関数 | 戻り値の型マーキングであり、strictにspawnの計算に参加できるユニットを示す | その呼び出しは `Async[T]` を返し、spawnグラフノードの作成をマークする |
| **`spawn { ... }`** | spawnブロック | 明示的に宣言されたコンカレンシードメイン | ランタイムがブロック内の各式を独立したタスクとして**積極的に**コンカレント実行し、ブロック结束时に暗黙的にすべての結果を待機する |
| **`spawn for`** | spawnループ | データ並列ループ | ループ体を複数の並列タスクに変換し、自動的にデータシャーディング、スケジューリング、结果収集を行う |

---

## 三、動作原理：コードから実行へ

### 3.1 コンパイル時：Spawnグラフの構築

```yaoxiang
# spawn関数定義：戻り値の型が spawn とマーキングされている
fetch: (String) -> String spawn = (url) => { ... }
parse: (String) -> Model spawn = (data) => { ... }

process: () -> Report = () => {
    # コンパイラがここでspawnグラフノードを作成する
    data_a = fetch("url1")  # ノード A: Async[String]
    data_b = fetch("url2")  # ノード B: Async[String]

    # spawnブロック：明示的なコンカレンシードメイン
    (model_a, model_b) = spawn {
        parse(data_a),  # ノード C: A に依存
        parse(data_b)   # ノード D: B に依存
    }

    # 最終集約ノード
    generate_report(model_a, model_b)  # ノード E
}
```

**コンパイラ動作：**
1. ソースコードを解析し、グローバルspawnグラフを構築する
2. 各式に対して計算ノードを作成する
3. データ依存関係を解析し、エッジ関係を確立する
4. `spawn { }` と `spawn for` ブロック内のサブグラフには **「並列評価」** マークが付けられる

### 4.2 ランタイム：Spawnスケジューラ

Intelligentでワークスティーリング 지원하는**Spawnスケジューラ**がspawnグラフの実行を担当します：

```rust
// Spawnスケジューラコアロジック
impl FlowScheduler {
    fn execute_node(&self, node_id: NodeId) {
        let node = self.get_node(node_id);
        
        match &node.kind {
            NodeKind::AsyncCompute => {
                // spawn関数：コルーチンプールにサブミット
                self.submit_async(node_id);
            }
            NodeKind::ParallelBlock => {
                // spawnブロック：積極的にすべての直接サブノードを並列実行
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
1. [E] の評価のために [C] と [D] が必要
2. [C] は [A] に依存、[D] は [B] に依存
3. Spawnスケジューラが [A] と [B] に依存関係がないことを発見 → 即座に並列実行
4. [A]、[B] 完了後、spawnブロックマークにより → 即座に [C] と [D] を並列実行
5. [C]、[D] 完了後、[E] を実行
```

**主要なメカニズム：**

| メカニズム | 説明 |
|------|------|
| **遅延トリガー** | 最終結果をリクエストすることから実行を開始し、逆方向に依存関係を追跡する |
| **自動待機** | `Async[T]` に遭遇したときに自動的にサスペンドし、他の準備完了タスクを実行する |
| **ワークスティーリング** | スレッドが他のスレッドキューからタスクをスティールし、CPU 使用率を向上させる |

---

## 四、主要メカニズムの詳細

### 4.1 副作用と評価保証

純粋な遅延評価は副作用（ログ、書き込みなど）が永不に実行されない可能性がある。Spawnモデルは**戻り値に基づく自動推論**を採用しています：

| ルール | 条件 | 動作 |
|------|------|------|
| **ルール1** | Void を返す関数 | **自動即時評価**（副作用は実行されなければならない） |
| **ルール2** | `@eager` デコレータを使用した式 | 戻り値の型に関係なく**強制即時評価** |
| **ルール3** | 非Void 型を返す | **遅延評価**（デフォルト） |

```yaoxiang
# Void を返す関数は自動的に即時実行される（副作用）
log: (String) -> Void = (message) => {
    print(message)
}

# @eager デコレータ：強制即時評価
cache_compute: (Int) -> Int = (x) => {
    # Int を返しても強制的に即座に実行する
    expensive_calculation(x)
}

main: () -> Void = () => {
    # log は自動的に即時実行される（Void を返すため）
    log("Processing started")

    # @eager は強制即時実行
    @eager
    cache_compute(100)

    # 通常の関数は遅延実行（Int を返す）
    result = heavy_computation(200)  # この時点では実行されない
    print(result)  # ここで初めて実行される
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

Rust形式の`?`演算子を採用し、透明なエラー伝播を実現します：

```yaoxiang
# Rust形式の ? 演算子
process() -> Result[Data, Error] = {
    data = fetch_data()?      # 自動的に待機しエラーをチェック
    processed = transform(data)?
    save(processed)?          # エラーは自動的に上に伝播
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

#### エラーチャート可視化

エラーチャートはコールスタックに似ているが、DAG 内のエラー伝播パスを示す：

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
│ 因果チェーン: main → calculate → divide                     │
│ キャプチャ位置: calculate (42行目)                          │
└─────────────────────────────────────────────────────────────┘
```

#### エラー処理ベストプラクティス

```yaoxiang
# 複数のエラーが発生する可能性のある操作を組み合わせて処理
batch_process: ([String]) -> Result[[String], Error] = (items) => {
    results = items.map(item => {
        process_item(item)?
    })
    ok(results)
}

# with? 糖衣構文（未来機能）
validate_user: (User) -> Result[ValidatedUser, ValidationError] = (user) => {
    name = user.name.with?(validate_name)?
    email = user.email.with?(validate_email)?
    ok(ValidatedUser(name, email))
}
```

### 4.3 純粋関数と `@blocking` 同期保証

**コア洞察：純粋関数はブロックしない！**

理由は：
- 純粋関数には I/O がなく、CPU 計算のみ
- 計算が долго かかってもスケジューラをブロックせず、CPU 時間のみを占有する

**実行戦略：**

| 関数型 | 実行戦略 | ブロック？ |
|----------|----------|--------|
| 純粋関数（I/O なし） | 同期実行 | いいえ（CPU 占有のみ） |
| 非同期関数（`Async[T]` を返す） | 非同期実行 | いいえ |
| `@blocking` アノテーション関数 | 同期実行、内部スケジューリング | いいえ |

**`@blocking` アノテーション：同期実行保証**

`@blocking` アノテーションは関数が同期姿勢で実行されることを保証する：
- 関数が戻るとき、結果はすでに準備完了
- 内部に非同期呼び出しがある場合、内部で完了をスケジューリングする
- 同期セマンティクスが必要だが内部に非同期操作が含まれるシナリオに適している

```yaoxiang
# @blocking：同期実行、内部非同期スケジューリング完了後に返る
heavy_compute: (List[Int]) -> Int = (data) => {
    # 内部に非同期操作があるかもしれないが、返る前に完了する
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
    result = heavy_compute([1, 2, 3, 4, 5])  # 即座に結果を返す
    print(result)  # 15

    # 非同期関数：Async[User] を返す
    user = fetch_user(123)  # Async[User]
    print(user.name)  # 自動的に待機しアンラップ
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
    
    // すべての内部非同期操作の完了を待機
    if !internal_async_ops.is_empty() {
        async_runtime.wait_all(internal_async_ops);
    }
    
    // 結果を返す
    result
}
```

**設計上の優位性：**
- **簡潔**：複雑な effect システムが不要
- **柔軟**：`@blocking` はオプションで、同期セマンティクスが必要なときだけ使用
- **効率的**：純粋関数は自動的に同期実行
- **安全**：メイン�スケジューラは永不にブロックしない

### 4.4 リソース衝突検出

コンパイル時にリソースアクセスパターンを解析し、衝突する操作を自動的に直列化する：

```
リソース衝突ルールマトリックス：
╔═══════════╦══════════╦══════════╗
║   アクセス ║   読取    ║   書込    ║
╠═══════════╬══════════╬══════════╣
║   読取    ║  並列可能  ║  直列化  ║
║   書込    ║  直列化   ║  直列化  ║
╚═══════════╩══════════╩══════════╝
```

**コンパイル時解析の例**：

```rust
// コンパイル時のリソースアクセス解析
struct ResourceAccess {
    reads: Set<ResourceId>,   // 読取リソース
    writes: Set<ResourceId>,  // 書込リソース
}

// 例
file1 = open("a.txt")  // リソース1：読取
file2 = open("b.txt")  // リソース2：読取
// file1 読取 と file2 読取 → 並列可能

file3 = open("c.txt")  // リソース3：書込
// file1 読取 と file3 書込 → 直列化
// file2 読取 と file3 書込 → 直列化
```

**コード例**：

```yaoxiang
# コンパイラが自動的に衝突を検出し直列化
process_files: () -> Void = () => {
    file_a = open("a.txt")  # リソース1：読取
    file_b = open("b.txt")  # リソース2：読取
    # file_a と file_b は両方とも読取のみ → 並列可能

    file_c = open("c.txt")  # リソース3：書込
    # file_a 読取 と file_c 書込 → 直列化
    # file_b 読取 と file_c 書込 → 直列化
}

# 複数の書込操作は自動的に直列化
write_logs: () -> Void = () => {
    log1 = open_log("log1.txt")  # リソース1：書込
    log2 = open_log("log2.txt")  # リソース2：書込
    # log1 と log2 は異なるリソース → 並列可能
}
```

### 4.5 並列競合制御：型システムによるAtomic性保証

**コア思想：型システムでコンカレントアクセスデータをマークし、コンパイラが同期の正確性をチェックする。**

**型マーキング体系：**

| 型 | セマンティクス | コンカレンシー安全 | 説明 |
|------|------|----------|------|
| `T` | 不変データ | ✅ 安全 | デフォルト型、マルチタスク読取で競合なし |
| `Ref[T]` | 変更可能参照 | ⚠️ 同期が必要 | コンカレント変更可能とマークされ、コンパイル時にロック使用をチェック |
| `Atomic[T]` | 原子型 | ✅ 安全 | 低レベル原子操作、ロック不要のコンカレンシー |
| `Mutex[T]` | 相互排除ロック包装 | ✅ 安全 | 自動ロック・ロック解除、コンパイル保証 |
| `RwLock[T]` | 読取書込ロック包装 | ✅ 安全 | 読取多書込少シナリオの最適化 |

**型の安全性保証：**

```yaoxiang
# デフォルトで不変 - 自然に競合なし
data: List[Int] = [1, 2, 3, 4, 5]
spawn for x in data { process(x) }  # ✅ 安全、読取のみで競合なし

# 変更可能参照 - 同期が必要
counter: Ref[Int] = Ref.new(0)

# 誤った例：ロックなしで Ref にアクセス（コンパイルエラー）
spawn for i in 1..10 {
    # ❌ コンパイルエラー：Ref は同期プリミティブを通じてアクセスする必要がある
    counter.value = counter.value + i
}

# 正しい例：with 糖衣構文で自動ロック
spawn for i in 1..10 {
    # ✅ with ブロックが自動的にロックを取得・解放
    with counter.lock() {
        counter.value = counter.value + i
    }
}

# 原子型 - ロック不要のコンカレンシー
atomic_counter: Atomic[Int] = Atomic.new(0)
spawn for i in 1..10 {
    # ✅ 原子操作、ロック不要で安全
    atomic_counter.fetch_add(i)
}
```

**Mutex[T] 型 - コンパイル時ロック保証：**

```yaoxiang
# 相互排除ロック包装のデータを作成
shared_state: Mutex[Map[String, Int]] = Mutex.new(Map.empty())

# with 糖衣構文を使用（Go の defer に類似）
main: () -> Void = () => {
    spawn for i in 1..100 {
        # with が自動的にロックを取得し、ブロック終了時に自動解放
        with shared_state.lock() {
            # クリティカルセクション：Mutex によって保護
            current = shared_state.get("count").or(0)
            shared_state.set("count", current + 1)
        }
    }

    # すべてのタスクの完了を待機
    print(shared_state.get("count"))  # 100
}
```

**型推論とロックチェック：**

```rust
// コンパイラがコンパイル時にチェック
fn compile_check_locks(func: &Function) {
    for node in func.nodes {
        match node {
            NodeKind::ReadRef(ref_var) => {
                // ロック保護範囲内かどうかをチェック
                if !is_inside_lock_guard(ref_var) {
                    compile_error!("Ref アクセスは lock() 保護範囲内で行う必要がある");
                }
            }
            NodeKind::WriteRef(ref_var, _) => {
                // 双重チェック：ロック + 一意の書き込み者
                if !is_inside_lock_guard(ref_var) {
                    compile_error!("Ref 変更は lock() 保護範囲内で行う必要がある");
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

**設計上の優位性：**

| 優位性 | 説明 |
|------|------|
| **コンパイル時チェック** | ロックの见落としはコンパイル時にキャプチャされ、実行時のデッドロックではない |
| **ゼロ実行時オーバーヘッド** | 競合がないとき、Mutex 包装にはオーバーヘッドがない |
| **簡潔な構文** | `with lock() { ... }` 糖衣構文で自動的にライフサイクルを管理 |
| **型安全** | Ref の代わりに Atomic を誤用すると型レベルでエラーが発生する |

---

## 五、利点のまとめ

| 利点 | 説明 |
|------|------|
| **ゼロ伝染性** | 非同期コードと同期コードは構文と型シグネチャに違いがなく、「async/await」伝染を完全に根絶する |
| **高性能並列処理** | 遅延spawnグラフと明示的な `spawn` マーキングを組み合わせることで、ランタイムが自動的に並列性を発見することも、プログラマが极限性能最適化のための明確なツールを持つことも可能 |
| **シンプルなメンタルモデル** | 開発者は複雑なコンカレンシーprimitives やコールバックを理解する必要はなく、データ流向とビジネスロジック에만 집중すればよい |
| **リファクタリングが容易** | 順序ロジックとコンカレンシーロジック間の切り替えコストは非常に低く、`spawn {}` 包装を増減するだけでよい |
| **直感的な用語** | 「spawn関数」「spawnブロック」「spawn値」により、技術Discussionが非常に直感的になる |

---

## 六、実装上の考慮事項

### 6.1 コンパイラ

- [ ] データフロー解析を実装し、spawnグラフを構築する
- [ ] `spawn` 戻り値型マーキングの解析と型推論を実装する
- [ ] `spawn {}` と `spawn for` を実行時並列primitives に脱糖する
- [ ] デコレータ（`@eager`、`@blocking`）をサポート
- [ ] Void 戻り値型自動即時評価ロジックを実装する
- [ ] リソース衝突検出を実装する
- [ ] Send/Sync 型制約チェックを実装する

### 6.2 ランタイム

- [ ] ワークスティーリング 지원하는spawnスケジューラを実装する
- [ ] 計算グラフ依存関係を感知したタスクスケジューリングを実装する
- [ ] `Async[T]` 型の自動アンラップメカニズムを実装する
- [ ] Void 関数の自動即時実行を実装する
- [ ] エラーチャート生成と伝播を実装する
- [ ] リソースアクセス直列化を実装する

### 6.3 デバッグツール ⚠️ 必须

**計算グラフ可視化デバッガ**は複雑なプログラム動作を理解するための鍵です：

| 機能 | 説明 |
|------|------|
| **ノード状態可視化** | 各計算ノードの Pending/Running/Completed 状態を観察する |
| **依存関係表示** | ノード間のデータ依存エッジを表示する |
| **タスクフロー追跡** | タスクのスレッド間での流转を観察する |
| **パフォーマンスボトルネック位置特定** | 長いチェーンとホットスポットノードを識別する |
| **エラーチャート可視化** | コンカレント環境でのエラー伝播パス表示 |

---

## 七、コード例

### 7.1 基本的なspawn関数

```yaoxiang
use std.net

# spawn関数定義：戻り値の型が spawn とマーキングされている
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

    # ここで自動的に待機しアンラップ
    print(user.name)            # 同期コードのように自然に
    print(posts.length)
}
```

### 7.2 Spawnブロック

```yaoxiang
fetch: (String) -> JSON spawn = (url) => { ... }
parse: (JSON) -> Model spawn = (json) => { ... }

parallel_fetch: () -> (Model, Model) = () => {
    # spawnブロック：明示的なコンカレンシードメイン
    (model_a, model_b) = spawn {
        parse(fetch("https://api1.com/data")),
        parse(fetch("https://api2.com/data"))
    }
    # モデル a と b はここで両方とも準備完了
    (model_a, model_b)
}
```

### 7.3 Spawnループ

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

> *"万物spawn action、吾れ以って復を観察す。"*
> —— 《易・復卦》
>
> Spawnモデルは遅延評価の宣言的な優雅さと高性能コンカレンシーの要求を組み合わせることで、システムプログラミングに安全性和极具表現力の全新パラダイムを提供することを目指しています。
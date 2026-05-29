```yaml
---
title: "並作 (spawn)"
description: 新しいランタイムで非同期計算を開始します
order: 4
state: Draft
---
```

# 並作 (spawn)

Tokio において、`spawn` 関数は、指定された非同期計算を新しい**タスク**としてランタイム内で起動します。この関数は主に `tokio::spawn` として使用され、コード并发性を実現するための基本的な手法となります。

## 基本的な使用法

```rust
#[tokio::main]
async fn main() {
    // タスクを生成
    let handle = tokio::spawn(async {
        // ここに非同期処理を書く
        println!("このコードは新しいタスクで実行されます");
    });

    // タスクの完了を待つ
    handle.await.unwrap();
}
```

## `spawn` 関数の特徴

`spawn` には、以下のような特徴があります：

- **即座に返る**: `spawn` はタスクが作成されるとすぐに `JoinHandle` を返すため、呼び出し元をブロックしません
- **独立したタスク**: 各 spawn されたタスクは独立して実行され、タスク間の進行は干渉しません
- **エラーの伝播**: タスクの生成に失敗した場合、`JoinError` を返します
- **型推論**: Rust の型推論により、`spawn` の返り値の型は自動的に決定されます

```rust
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    // 複数のタスクを同時に生成
    let handle1 = tokio::spawn(async {
        sleep(Duration::from_secs(2)).await;
        println!("タスク 1 が完了しました");
        42
    });

    let handle2 = tokio::spawn(async {
        sleep(Duration::from_secs(1)).await;
        println!("タスク 2 が完了しました");
        "完了"
    });

    // 両方のタスクの結果を待つ
    let result1 = handle1.await.unwrap();
    let result2 = handle2.await.unwrap();

    println!("結果: {}, {}", result1, result2);
}
```

## タスク間の待ち合わせ

複数のタスクを同時に待つには、`join!` マクロを使用します：

```rust
use tokio::time::{sleep, Duration};
use tokio::join;

#[tokio::main]
async fn main() {
    let handle1 = tokio::spawn(async {
        sleep(Duration::from_secs(2)).await;
        "最初の結果"
    });

    let handle2 = tokio::spawn(async {
        sleep(Duration::from_secs(1)).await;
        "2番目の結果"
    });

    // 両方のタスクの結果を同時に待つ
    let (result1, result2) = join!(handle1, handle2);

    println!("{}, {}", result1.unwrap(), result2.unwrap());
}
```

## `spawn` と `block_on` の違い

| 関数 | 動作 | 主な用途 |
|------|------|----------|
| `block_on` | 単一の future をポーリングして完了までブロック | ランタイム外で future を実行 |
| `spawn` | 新しいタスクを生成して非同期に実行 | 並发タスクの作成 |

```rust
use tokio::time::{sleep, Duration};
use tokio::runtime::Runtime;

fn main() {
    // ランタイムを作成
    let runtime = Runtime::new().unwrap();

    // タスク A を生成
    let handle_a = runtime.spawn(async {
        println!("タスク A: 開始");
        sleep(Duration::from_millis(50)).await;
        println!("タスク A: ブロックを開始");
        // このブロックはタスク A 内でのみブロックし、
        // 他のタスクの進行を妨げない
        let result = tokio::task::spawn_blocking(|| {
            // CPU 集中的な作業
            println!("タスク A: ブロック中...");
            sleep(Duration::from_secs(1)).await;
            "タスク A の結果"
        }).await.unwrap();
        println!("タスク A: 完了");
        result
    });

    // タスク B を生成
    let handle_b = runtime.spawn(async {
        println!("タスク B: 実行中");
        sleep(Duration::from_millis(100)).await;
        println!("タスク B: 完了");
        "タスク B の結果"
    });

    // 両方のタスクの結果を取得
    let result_a = runtime.block_on(handle_a).unwrap();
    let result_b = runtime.block_on(handle_b).unwrap();

    println!("最終結果: {}, {}", result_a, result_b);
}
```

この例では：

- **タスク A** と **タスク B** は両方ともランタイムによって同時にスケジュールされます
- タスク A が `spawn_blocking` を使用しても、タスク B は妨げられることなく実行を続けられます
- `join!` を使用すると、複数のタスクの結果をより簡潔に待つことができます

## `spawn` 使用時の注意点

### 所有権の移動

タスクに渡すデータは、所有権を.move` する必要があります：

```rust
#[tokio::main]
async fn main() {
    let data = vec![1, 2, 3];

    // data をタスクに移動
    let handle = tokio::spawn(async move {
        // data はこのタスクに所有権が移動する
        println!("所有権が移動したデータ: {:?}", data);
    });

    handle.await.unwrap();

    // data はもうここでは使用できない
    // println!("{:?}", data); // コンパイルエラー
}
```

### タスク間のデータ共有

タスク間でデータを共有するには、`Arc` を使用します：

```rust
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() {
    // Arc でデータを共有
    let counter = Arc::new(Mutex::new(0));

    let mut handles = vec![];

    for _ in 0..10 {
        let counter = Arc::clone(&counter);
        let handle = tokio::spawn(async move {
            let mut num = counter.lock().await;
            *num += 1;
        });
        handles.push(handle);
    }

    // すべてのタスクが完了するのを待つ
    for handle in handles {
        handle.await.unwrap();
    }

    println!("最終的なカウント: {}", *counter.lock().await);
}
```

### タスクのスコープ

`tokio::spawn` で生成されたタスクは、親タスクが完了しても実行を継続します。タスクの寿命を制御するには、スコープ付き spawn を使用します：

```rust
use tokio::task::{spawn, ScopedJoinHandle};

#[tokio::main]
async fn main() {
    // スコープ付きタスクで変数を借用
    let data = vec![1, 2, 3];

    let result = tokio::task::scope(|scope| {
        spawn(scope, async {
            // data を借用
            println!("スコープ内: {:?}", data);
            42
        })
    }).unwrap();

    // data はここで引き続き使用可能
    println!("結果: {}", result);
    println!("データはまだ使用可能: {:?}", data);
}
```

## まとめ

`spawn` はTokioにおける并发性の基本です。主な点は：

1. **`spawn`**: 新しいタスクを生成し、即座に `JoinHandle` を返します
2. **`block_on`**: 単一の future を実行し、完了までブロックします
3. **`join!`**: 複数のタスクの結果を同時に待ちます
4. **所有権**: タスクに移動するデータは `.move` する必要があります
5. **データ共有**: `Arc` と適切な同期プリミティブを使用します
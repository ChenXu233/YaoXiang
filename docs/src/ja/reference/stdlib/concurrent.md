---
title: 'std.concurrent'
description: 'スリープ、スケジューリングの譲歩、スレッド識別子'
---

# std.concurrent

並行処理支援モジュール。

```yaoxiang
use std.concurrent
```

> 本モジュールはオペレーティングシステムのスレッドに依存しており、`wasm32`
> ターゲットでは**エクスポートされません**。

## 関数一覧

<!-- stdlib:table:concurrent start -->

| 関数        | シグネチャ              |
| ----------- | ----------------------- |
| `sleep`     | `(millis: Int) -> Void` |
| `thread_id` | `() -> String`          |
| `yield_now` | `() -> Void`            |

<!-- stdlib:table:concurrent end -->

## 関数

### sleep

<!-- stdlib:sig:concurrent.sleep start -->

```yaoxiang
sleep: (millis: Int) -> Void
```

<!-- stdlib:sig:concurrent.sleep end -->

現在のスレッドを指定された**ミリ秒数**ブロックします。

- `millis` —— スリープするミリ秒数。非 `Int` または欠落時は `0`
  として扱われます（エラーは発生しません）

> **単位に関する注意**：[`std.time.sleep`](./time#sleep)
> は**秒**単位で小数を許容しますが、本関数は**ミリ秒**単位です。`concurrent.sleep(1)`
> は 1 ミリ秒スリープし、 `time.sleep(1)` は 1 秒スリープします。

```yaoxiang
use std.concurrent

main: () -> Void = {
    concurrent.sleep(0)
    concurrent.sleep(1)
}
```

### thread_id

<!-- stdlib:sig:concurrent.thread_id start -->

```yaoxiang
thread_id: () -> String
```

<!-- stdlib:sig:concurrent.thread_id end -->

現在のスレッドの識別子文字列を返します。

戻り値：`ThreadId(1)`
形式の文字列。具体的な値はプラットフォームやスケジューリングによって変化するため、**存在チェックのみに使用**し、その具体的な内容や形式には依存しないでください。

```yaoxiang
use std.assert
use std.concurrent
use std.string

main: () -> Void = {
    tid = concurrent.thread_id()
    assert(string.len(tid) > 0)
}
```

### yield_now

<!-- stdlib:sig:concurrent.yield_now start -->

```yaoxiang
yield_now: () -> Void
```

<!-- stdlib:sig:concurrent.yield_now end -->

現在のスレッドのスケジューリング・タイムスライスを自発的に譲り、他のスレッドに実行の機会を与えます。

```yaoxiang
use std.concurrent

main: () -> Void = {
    concurrent.yield_now()
}
```

## 関連

- [`std.time.sleep`](./time#sleep) —— 秒単位のスリープ
- [言語仕様：並行性モデル](../language-spec/concurrency.md) —— `spawn` と並作のセマンティクス

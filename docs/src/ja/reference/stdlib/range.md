---
title: 'std.range'
description: '区間イテレーション、述語、遅延アダプタ'
---

# std.range

区間（`Range`）のイテレーションとアダプタ。

```yaoxiang
use std.range
```

## 区間リテラル

| 書き方    | 意味                            |
| --------- | ------------------------------- |
| `a..b`    | `a` から `b` まで、ステップ `1` |
| `a..b..s` | `a` から `b` まで、ステップ `s` |

区間は**終端値を含まない**（半開区間）。ステップは負の値でもよく、降順を意味する。

## イテレータプロトコル

[`iter`](#iter) は `Result` を返す——ステップが `0` の場合はエラー経路（`E6009`）となるため、`unwrap`
するか明示的に処理する必要がある：

```yaoxiang
use std.assert
use std.list
use std.range
use std.result

main: () -> Void = {
    it = result.unwrap(range.iter(1..4))
    assert(range.has_next(it))
}
```

> **ムーブセマンティクス**：`has_next` と `next` のシグネチャには `&`
> が付かないため、イテレータを**ムーブ**する。したがって、参照するたびに毎回イテレータを作り直すか、`for ... in`
> で直接走査する必要がある。これは [`std.list`](./list) のイテレータと同じ仕様である。

```yaoxiang
use std.assert
use std.range
use std.result

main: () -> Void = {
    // イテレータを毎回新しく作成
    a = result.unwrap(range.iter(1..3))
    assert(range.has_next(a))

    b = result.unwrap(range.iter(1..3))
    assert(range.next(b) == 1)
}
```

日常的な走査は直接 `for ... in` を使う：

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    nums = [1, 2, 3]
    mut sum = 0
    for x in nums {
        sum = sum + x
    }
    assert(sum == 6)
}
```

## 関数一覧

<!-- stdlib:table:range start -->

| 関数                 | シグネチャ                                                    |
| -------------------- | ------------------------------------------------------------- |
| `iter`               | `(r: Range(Int)) -> Result(Iterator(Any), Error)`             |
| `has_next`           | `(it: Iterator(Any)) -> Bool`                                 |
| `next`               | `(it: &Iterator(Any)) -> Any`                                 |
| `contains`           | `(r: Range(Int), x: Int) -> Result(Bool, Error)`              |
| `abort_invalid_step` | `(r: Range(Int)) -> Any`                                      |
| `map`                | `(it: Iterator(Any), f: (Any) -> Any) -> Iterator(Any)`       |
| `filter`             | `(it: Iterator(Any), p: (Any) -> Bool) -> Iterator(Any)`      |
| `collect`            | `(it: Iterator(Any)) -> List(Any)`                            |
| `reduce`             | `(it: Iterator(Any), init: Any, f: (Any, Any) -> Any) -> Any` |
| `for_each`           | `(it: Iterator(Any), f: (Any) -> Void) -> Void`               |

<!-- stdlib:table:range end -->## イテレータプロトコル

### iter

<!-- stdlib:sig:range.iter start -->

```yaoxiang
iter: (r: Range(Int)) -> Result(Iterator(Any), Error)
```

<!-- stdlib:sig:range.iter end -->

区間からイテレータを作成する。

- `r` —— 区間。例：`1..6` や `3..0..-1`

戻り値：成功時は `Result.ok(イテレータ)`。ステップが `0` の場合は `Result.err` となり、`code` は
`E6009`。

```yaoxiang
use std.assert
use std.range
use std.result

main: () -> Void = {
    it = result.unwrap(range.iter(1..4))
    assert(range.has_next(it))
}
```

### has_next

<!-- stdlib:sig:range.has_next start -->

```yaoxiang
has_next: (it: Iterator(Any)) -> Bool
```

<!-- stdlib:sig:range.has_next end -->

未消費の要素があるかどうか。

> イテレータを**ムーブ**する。

```yaoxiang
use std.assert
use std.range
use std.result

main: () -> Void = {
    it = result.unwrap(range.iter(1..4))
    assert(range.has_next(it))
}
```

### next

<!-- stdlib:sig:range.next start -->

```yaoxiang
next: (it: &Iterator(Any)) -> Any
```

<!-- stdlib:sig:range.next end -->

現在の要素を取り出し、内部カーソルを 1 つ進める。

戻り値：現在の要素。反復終了時は `Void` を返す。

> イテレータを**ムーブ**する。

```yaoxiang
use std.assert
use std.range
use std.result

main: () -> Void = {
    it = result.unwrap(range.iter(1..4))
    assert(range.next(it) == 1)
}
```

降順区間も同様にサポート：

```yaoxiang
use std.assert
use std.range
use std.result

main: () -> Void = {
    desc = result.unwrap(range.iter(3..0..-1))
    assert(range.next(desc) == 3)
}
```

### contains

<!-- stdlib:sig:range.contains start -->

```yaoxiang
contains: (r: Range(Int), x: Int) -> Result(Bool, Error)
```

<!-- stdlib:sig:range.contains end -->

`x` が区間内に含まれるかどうかを判定する。

- `r` —— 区間
- `x` —— 判定対象の値

戻り値：`Result.ok(Bool)`。終端値は**開区間**（含まない）；ステップが指定された場合は、ステップに揃った要素のみマッチする。

```yaoxiang
use std.assert
use std.range
use std.result

main: () -> Void = {
    assert(result.unwrap(range.contains(1..10, 5)))
    assert(!result.unwrap(range.contains(1..10, 10)))     // 終端値は含まない

    assert(result.unwrap(range.contains(0..10..2, 4)))    // ステップに揃う
    assert(!result.unwrap(range.contains(0..10..2, 3)))   // 揃わない
}
```

### abort_invalid_step

<!-- stdlib:sig:range.abort_invalid_step start -->

```yaoxiang
abort_invalid_step: (r: Range(Int)) -> Any
```

<!-- stdlib:sig:range.abort_invalid_step end -->

ステップが不正な場合の中止フック。`for ... in` がステップ `0` の区間を消費する際に呼ばれる。

**常に** `E6007` をスローし、メッセージは `Range step must be non-zero (for/in consumption)`
である。通常のコードから直接呼び出す必要はない。

```yaoxiang
use std.range

main: () -> Void = {
    // iter を直接使うと Err が返ってくるため、このフックを通す必要はない
    r = range.iter(1..3)
}
```

## アダプタ

`map` と `filter` は**遅延**アダプタを返す——即座には計算せず、[`collect`](#collect) /
[`reduce`](#reduce) / [`for_each`](#for_each) / `for ... in`
で消費されたときに初めて結果を生成する。

### map

<!-- stdlib:sig:range.map start -->

```yaoxiang
map: (it: Iterator(Any), f: (Any) -> Any) -> Iterator(Any)
```

<!-- stdlib:sig:range.map end -->

`f` を各要素にマッピングし、新しい遅延イテレータを返す。

```yaoxiang
use std.assert
use std.list
use std.range
use std.result

main: () -> Void = {
    doubled = range.collect(range.map(result.unwrap(range.iter(1..4)), x => x * 2))
    assert(list.len(doubled) == 3)
    assert(list.get(doubled, 0) == 2)
}
```

### filter

<!-- stdlib:sig:range.filter start -->

```yaoxiang
filter: (it: Iterator(Any), p: (Any) -> Bool) -> Iterator(Any)
```

<!-- stdlib:sig:range.filter end -->

`p` が真となる要素のみを保持し、新しい遅延イテレータを返す。

```yaoxiang
use std.assert
use std.list
use std.range
use std.result

main: () -> Void = {
    big = range.collect(range.filter(result.unwrap(range.iter(1..6)), x => x > 3))
    assert(list.len(big) == 2)
    assert(list.get(big, 0) == 4)
}
```

アダプタはチェーンして組み合わせ可能：

```yaoxiang
use std.assert
use std.list
use std.range
use std.result

main: () -> Void = {
    r = 1..6
    chained = range.collect(range.map(range.filter(result.unwrap(range.iter(r)), x => x % 2 == 0), x => x * 10))
    assert(list.get(chained, 0) == 20)
    assert(list.get(chained, 1) == 40)
}
```

### collect

<!-- stdlib:sig:range.collect start -->

```yaoxiang
collect: (it: Iterator(Any)) -> List(Any)
```

<!-- stdlib:sig:range.collect end -->

イテレータを消費し、全要素を `List` に集める。

```yaoxiang
use std.assert
use std.list
use std.range
use std.result

main: () -> Void = {
    xs = range.collect(result.unwrap(range.iter(1..4)))
    assert(list.len(xs) == 3)
}
```

### reduce

<!-- stdlib:sig:range.reduce start -->

```yaoxiang
reduce: (it: Iterator(Any), init: Any, f: (Any, Any) -> Any) -> Any
```

<!-- stdlib:sig:range.reduce end -->

イテレータを消費して畳み込む。

- `it` —— イテレータ
- `init` —— 初期累積値
- `f` —— リダクション関数 `(累積値, 要素) -> 新しい累積値`

> 引数の順序は [`std.list.reduce`](./list#reduce) と異なる点に注意：本モジュールは
> `(イテレータ, 初期値, 関数)` であるのに対し、`std.list` は `(リスト, 関数, 初期値)` である。

```yaoxiang
use std.assert
use std.range
use std.result

main: () -> Void = {
    total = range.reduce(result.unwrap(range.iter(1..6)), 0, (acc, x) => acc + x)
    assert(total == 15)
}
```

### for_each

<!-- stdlib:sig:range.for_each start -->

```yaoxiang
for_each: (it: Iterator(Any), f: (Any) -> Void) -> Void
```

<!-- stdlib:sig:range.for_each end -->

各要素に対して `f` を実行する。副作用を伴う処理用。

```yaoxiang
use std.assert
use std.range
use std.result

main: () -> Void = {
    // 1、2、3 を出力
    range.for_each(result.unwrap(range.iter(1..4)), x => println(x))
    assert(true)
}
```

> クロージャは現状**外側の `mut` 変数を捕捉して書き換えることができない**ため、`for_each`
> で累積を行うことはできない（`E1001` が発生する）——累積には [`reduce`](#reduce) を使用すること。

## 関連

- [`std.list`](./list) —— リストとそのイテレータ
- [`std.result`](./result) —— `iter` / `contains` の戻り値をアンラップする
- [エラーコードリファレンス](../error-code/) —— `E6009` ステップ不正

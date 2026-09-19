---
title: 'std.range'
description: '範囲の反復、述語と遅延アダプタ'
---

# std.range

範囲（`Range`）の反復とアダプタ。

```yaoxiang
use std.range
```

## 範囲リテラル

| 書き方    | 意味                            |
| --------- | ------------------------------- |
| `a..b`    | `a` から `b` まで、ステップ `1` |
| `a..b..s` | `a` から `b` まで、ステップ `s` |

範囲は**終了値を含まない**（左閉右開）。ステップは負の値も可能で、減少を表す。

## イテレータプロトコル

[`iter`](#iter) は `Result` を返す——ステップが `0` の場合はエラーパス（`E6009`）となるため、`unwrap`
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
> が付かないため、イテレータを**ムーブ**してしまう。したがって、毎回イテレータを作り直すか、`for ... in`
> で直接走査する必要がある。これは [`std.list`](./list) のイテレータと同じ動作である。

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
| `collect`            | `(it: Iterator(Any)) -> Vec(Any)`                             |
| `reduce`             | `(it: Iterator(Any), init: Any, f: (Any, Any) -> Any) -> Any` |
| `for_each`           | `(it: Iterator(Any), f: (Any) -> Void) -> Void`               |

<!-- stdlib:table:range end -->## イテレータプロトコル

### iter

<!-- stdlib:sig:range.iter start -->

```yaoxiang
iter: (r: Range(Int)) -> Result(Iterator(Any), Error)
```

<!-- stdlib:sig:range.iter end -->

範囲からイテレータを作成する。

- `r` —— 範囲（例：`1..6` や `3..0..-1`）

戻り値：成功時は `Result.ok(イテレータ)`、ステップが `0` の場合は `Result.err` となり、`code` は
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

まだ消費されていない要素があるかどうか。

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

現在の要素を取り出し、内部カーソルを1つ進める。

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

減少範囲も同様にサポート：

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

`x` が範囲内に含まれるかどうかを判定する。

- `r` —— 範囲
- `x` —— 判定対象の値

戻り値：`Result.ok(Bool)`。終了値は**開区間**（含まない）。ステップが指定されている場合は、ステップに整列している要素のみが一致する。

```yaoxiang
use std.assert
use std.range
use std.result

main: () -> Void = {
    assert(result.unwrap(range.contains(1..10, 5)))
    assert(!result.unwrap(range.contains(1..10, 10)))     // 終了値は含まない

    assert(result.unwrap(range.contains(0..10..2, 4)))    // ステップに整列
    assert(!result.unwrap(range.contains(0..10..2, 3)))   // 整列していない
}
```

### abort_invalid_step

<!-- stdlib:sig:range.abort_invalid_step start -->

```yaoxiang
abort_invalid_step: (r: Range(Int)) -> Any
```

<!-- stdlib:sig:range.abort_invalid_step end -->

ステップが不正な場合の中断フック。`for ... in` がステップ `0` の範囲を消費する際に呼び出される。

**常に** `E6007` をスローし、メッセージは
`Range step must be non-zero (for/in consumption)`。通常のコードでは直接呼び出す必要はない。

```yaoxiang
use std.range

main: () -> Void = {
    // 直接 iter を使うと Err になるため、このフックを通す必要はない
    r = range.iter(1..3)
}
```

## アダプタ

`map` と `filter` は**遅延**アダプタを返す——これらは即座には計算されず、[`collect`](#collect) /
[`reduce`](#reduce) / [`for_each`](#for_each) / `for ... in` で消費された後に結果が生成される。

### map

<!-- stdlib:sig:range.map start -->

```yaoxiang
map: (it: Iterator(Any), f: (Any) -> Any) -> Iterator(Any)
```

<!-- stdlib:sig:range.map end -->

`f` を各要素にマップし、新しい遅延イテレータを返す。

```yaoxiang
use std.assert
use std.list
use std.range
use std.result

main: () -> Void = {
    doubled = range.collect(range.map(result.unwrap(range.iter(1..4)), x => x * 2))
    assert(doubled.length == 3)
    assert(doubled[0] == 2)
}
```

### filter

<!-- stdlib:sig:range.filter start -->

```yaoxiang
filter: (it: Iterator(Any), p: (Any) -> Bool) -> Iterator(Any)
```

<!-- stdlib:sig:range.filter end -->

`p` が真となる要素を保持し、新しい遅延イテレータを返す。

```yaoxiang
use std.assert
use std.list
use std.range
use std.result

main: () -> Void = {
    big = range.collect(range.filter(result.unwrap(range.iter(1..6)), x => x > 3))
    assert(big.length == 2)
    assert(big[0] == 4)
}
```

アダプタはチェーン状に組み合わせ可能：

```yaoxiang
use std.assert
use std.list
use std.range
use std.result

main: () -> Void = {
    r = 1..6
    chained = range.collect(range.map(range.filter(result.unwrap(range.iter(r)), x => x % 2 == 0), x => x * 10))
    // 値セマンティクス：`chained` はインデックス読み取りで消費され、1つずつローカル変数にバインドされる
    first = chained[0]
    second = chained[1]
    assert(first == 20)
    assert(second == 40)
}
```

### collect

<!-- stdlib:sig:range.collect start -->

```yaoxiang
collect: (it: Iterator(Any)) -> Vec(Any)
```

<!-- stdlib:sig:range.collect end -->

イテレータを消費し、すべての要素を `List` に収集する。

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
- `f` —— 畳み込み関数 `(累積値, 要素) -> 新しい累積値`

> 引数の順序が [`std.list.reduce`](./list#reduce) と異なることに注意：本モジュールは
> `(イテレータ, 初期値, 関数)` であり、`std.list` は `(リスト, 関数, 初期値)` である。

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

各要素に対して `f` を実行し、副作用のために使用する。

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

> クロージャは現時点で外側の `mut` 変数を**キャプチャして書き換える**ことはできないため、`for_each`
> での累積は不可能（`E1001` が発生する）——累積には [`reduce`](#reduce) を使用してください。

## 関連

- [`std.list`](./list) —— リストとそのイテレータ
- [`std.result`](./result) —— `iter` / `contains` の戻り値をアンラップする
- [エラーコードリファレンス](../error-code/) —— `E6009` ステップが不正

---
title: 'std.list'
description: 'リストの追加・削除、スライス、高階関数とイテレータプロトコル'
---

# std.list

リスト操作モジュール。**ムーブセマンティクスには特に注意が必要**：2 種類の関数がある ― ソースリストを読み取り専用で借用するものと、ソースリストを消費（ムーブ）するもの。
`&` 仮引数の自動借用ルールについては RFC-009
§2.8 を参照：実引数が呼び出し後にも使用される場合、コンパイラが自動的に読み取り専用トークンを作成する。

```yaoxiang
use std.list
```

## 意味の分類

シグネチャに `&` が付くパラメータは読み取り専用借用で、呼び出し後もソース値は使用可能；`&`
の付かないパラメータは値渡しで渡され、呼び出し後はソース値が**ムーブ済み**となり、再度使用すると
`E2014` を送出する。

| カテゴリ               | 関数                                                                                                             | 動作                                     |
| ---------------------- | ---------------------------------------------------------------------------------------------------------------- | ---------------------------------------- |
| **ソースリストを消費** | `push` `append` `prepend` `set` `pop` `remove_at`                                                                | ソースリストがムーブされ、以後は使用不可 |
| 読み取り専用借用       | `len` `is_empty` `get` `first` `last` `slice` `reverse` `concat` `contains` `find_index` `map` `filter` `reduce` | ソースリストは繰り返し使用可能           |
| イテレータプロトコル   | `iter`（ソースリストを消費し、イテレータを返す）`has_next` `next`（イテレータの借用 / 可変借用）                 | 下記参照                                 |

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    nums = [1, 2, 3]

    // 読み取り専用借用：nums は繰り返し使用可能
    assert(list.len(nums) == 3)
    assert(list.len(nums) == 3)
    assert(list.contains(nums, 2))

    // 消費：base はここでは以後使用不可
    base = [1, 2]
    extended = list.push(base, 3)
    assert(list.len(extended) == 3)
}
```

## 関数一覧

<!-- stdlib:table:list start -->

| 関数         | シグネチャ                                                                                 |
| ------------ | ------------------------------------------------------------------------------------------ |
| `push`       | `(A: Type) -> (list: Vec(A), item: A) -> Vec(A)`                                           |
| `pop`        | `(A: Type) -> (list: Vec(A)) -> Vec(A)`                                                    |
| `append`     | `(A: Type) -> (list: Vec(A), item: A) -> Vec(A)`                                           |
| `prepend`    | `(A: Type) -> (list: Vec(A), item: A) -> Vec(A)`                                           |
| `remove_at`  | `(A: Type) -> (list: Vec(A), index: Int) -> Vec(A)`                                        |
| `reverse`    | `(A: Type) -> (list: &Vec(A)) -> Vec(A)`                                                   |
| `concat`     | `(A: Type) -> (a: &Vec(A), b: &Vec(A)) -> Vec(A)`                                          |
| `map`        | `(T: Type, R: Type) -> (list: &Vec(T), f: (item: T) -> R) -> Vec(R)`                       |
| `filter`     | `(T: Type) -> (list: &Vec(T), keep: (item: T) -> Bool) -> Vec(T)`                          |
| `reduce`     | `(T: Type, Acc: Type) -> (list: &Vec(T), f: (acc: Acc, item: T) -> Acc, init: Acc) -> Acc` |
| `len`        | `(A: Type) -> (list: &Vec(A)) -> Int`                                                      |
| `is_empty`   | `(A: Type) -> (list: &Vec(A)) -> Bool`                                                     |
| `get`        | `(A: Type) -> (list: &Vec(A), index: Int) -> A`                                            |
| `set`        | `(A: Type) -> (list: Vec(A), index: Int, value: A) -> Vec(A)`                              |
| `first`      | `(A: Type) -> (list: &Vec(A)) -> A`                                                        |
| `last`       | `(A: Type) -> (list: &Vec(A)) -> A`                                                        |
| `slice`      | `(A: Type) -> (list: &Vec(A), start: Int, end: Int) -> Vec(A)`                             |
| `contains`   | `(A: Type) -> (list: &Vec(A), item: A) -> Bool`                                            |
| `find_index` | `(A: Type) -> (list: &Vec(A), item: A) -> Int`                                             |
| `iter`       | `(T: Type) -> (list: Vec(T)) -> Iter(T)`                                                   |
| `next`       | `(T: Type) -> (it: &mut Iter(T)) -> T`                                                     |
| `has_next`   | `(T: Type) -> (it: &Iter(T)) -> Bool`                                                      |
| `empty`      | `(T: Type) -> Vec(T)`                                                                      |
| `of`         | `(T: Type) -> (data: Vec(T)) -> Vec(T)`                                                    |

<!-- stdlib:table:list end -->## 関数

### push

<!-- stdlib:sig:list.push start -->

```yaoxiang
push: (A: Type) -> (list: Vec(A), item: A) -> Vec(A)
```

<!-- stdlib:sig:list.push end -->

`list` の末尾に `item` を追加した**新しいリスト**を返す。`list`
は値渡しで渡され、呼び出し後は**ムーブ**されて使用できなくなる。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    base = [1, 2]
    extended = list.push(base, 3)
    assert(list.len(extended) == 3)
}
```

### append

<!-- stdlib:sig:list.append start -->

```yaoxiang
append: (A: Type) -> (list: Vec(A), item: A) -> Vec(A)
```

<!-- stdlib:sig:list.append end -->

`push` の別名。動作は完全に同一。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    extended = list.append([1, 2], 3)
    assert(list.len(extended) == 3)
}
```

### prepend

<!-- stdlib:sig:list.prepend start -->

```yaoxiang
prepend: (A: Type) -> (list: Vec(A), item: A) -> Vec(A)
```

<!-- stdlib:sig:list.prepend end -->

`list` の先頭に `item` を挿入した新しいリストを返す。`list`
は値渡しで渡され、呼び出し後は**ムーブ**される。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    l = list.prepend([2, 3], 1)
    assert(list.first(l) == 1)
}
```

### pop

<!-- stdlib:sig:list.pop start -->

```yaoxiang
pop: (A: Type) -> (list: Vec(A)) -> Vec(A)
```

<!-- stdlib:sig:list.pop end -->

末尾の要素を削除し、**短縮されたリスト**を返す（値セマンティクス）。ソースリストは消費され、ネイティブ版に見られる「シグネチャに
`&` が付いていながらソース値をインプレースに変更する」という例外的な形式は採らない。

戻り値：末尾の要素を除いた新しいリスト。リストが空の場合はそのまま返す。削除された要素を読み取るには、呼び出し前に
`last` で値を取得する。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    l = [1, 2, 3]
    rest = list.pop(l)               // l は消費され、rest は短縮された新しいリスト
    assert(list.len(rest) == 2)
    assert(list.last(rest) == 2)     // 末尾の 3 は削除済み

    // 削除された要素を読み取るには、先に last で値を取得してから pop する
    l2 = [1, 2, 3]
    removed = list.last(l2)
    assert(removed == 3)

    empty = list.empty(Int)
    assert(list.is_empty(list.pop(empty)))
}
```

### remove_at

<!-- stdlib:sig:list.remove_at start -->

```yaoxiang
remove_at: (A: Type) -> (list: Vec(A), index: Int) -> Vec(A)
```

<!-- stdlib:sig:list.remove_at end -->

インデックス `index`
の位置の要素を削除し、**短縮された新しいリスト**を返す（値セマンティクス）。ソースリストは消費される。

- `index` ―― 要素のインデックス

戻り値：その要素を除いた新しいリスト。エラー：インデックスが負または長さ以上のとき、`E6003`（インデックス範囲外）を送出する。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    l = [10, 20, 30]
    got = list.remove_at(l, 1)
    assert(list.len(got) == 2)
    assert(list.get(got, 0) == 10)
    assert(list.get(got, 1) == 30)
}
```

### set

<!-- stdlib:sig:list.set start -->

```yaoxiang
set: (A: Type) -> (list: Vec(A), index: Int, value: A) -> Vec(A)
```

<!-- stdlib:sig:list.set end -->

インデックス `index` を `value` に書き換えた新しいリストを返す。`list`
は値渡しで渡され、呼び出し後は**ムーブ**される。

- `index` ―― インデックス；デフォルト `0`
- `value` ―― 新しい値；デフォルト `Void`

エラー：インデックスが負または長さ以上のとき、`E6003`
を送出する（範囲外への書き込みはもはや静かに破棄されない）。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    l = list.set([1, 2, 3], 1, 99)
    assert(list.get(l, 1) == 99)
}
```

### get

<!-- stdlib:sig:list.get start -->

```yaoxiang
get: (A: Type) -> (list: &Vec(A), index: Int) -> A
```

<!-- stdlib:sig:list.get end -->

インデックス `index` の位置の要素を読み取る（読み取り専用借用で、`list` は再利用可能）。

- `index` ―― インデックス；デフォルト `0`

戻り値：要素の値。**範囲外の場合は `Void`
を返す**（エラーは送出しない）。エラー：インデックスが負のとき、`E6007` を送出する。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    nums = [1, 2, 3, 4]
    assert(list.get(nums, 1) == 2)
}
```

### first

<!-- stdlib:sig:list.first start -->

```yaoxiang
first: (A: Type) -> (list: &Vec(A)) -> A
```

<!-- stdlib:sig:list.first end -->

先頭の要素を返す。空のリストの場合は `Void` を返す。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    assert(list.first([1, 2, 3]) == 1)
}
```

### last

<!-- stdlib:sig:list.last start -->

```yaoxiang
last: (A: Type) -> (list: &Vec(A)) -> A
```

<!-- stdlib:sig:list.last end -->

末尾の要素を返す。空のリストの場合は `Void` を返す。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    assert(list.last([1, 2, 3]) == 3)
}
```

### slice

<!-- stdlib:sig:list.slice start -->

```yaoxiang
slice: (A: Type) -> (list: &Vec(A), start: Int, end: Int) -> Vec(A)
```

<!-- stdlib:sig:list.slice end -->

`[start, end)` 区間の部分リストを取得する。

- `start` ―― 開始インデックス；デフォルト `0`
- `end` ―― 終了インデックス（含まない）；デフォルトはリストの末尾

戻り値：新しいリスト。境界は**クランプ**されて有効範囲となるため、エラーは送出しない。エラー：`start`
または `end` が負のとき、`E6007` を送出する。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    sub = list.slice([1, 2, 3, 4], 1, 3)
    assert(list.len(sub) == 2)
    assert(list.first(sub) == 2)
}
```

### reverse

<!-- stdlib:sig:list.reverse start -->

```yaoxiang
reverse: (A: Type) -> (list: &Vec(A)) -> Vec(A)
```

<!-- stdlib:sig:list.reverse end -->

要素の順序を反転した新しいリストを返す。ソースリストは変更されない。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    rev = list.reverse([1, 2, 3])
    assert(list.first(rev) == 3)
}
```

### concat

<!-- stdlib:sig:list.concat start -->

```yaoxiang
concat: (A: Type) -> (a: &Vec(A), b: &Vec(A)) -> Vec(A)
```

<!-- stdlib:sig:list.concat end -->

2 つのリストを連結し、新しいリストを返す。両方のソースリストは変更されない。

エラー：第 2 引数がリストでない場合、`E6007` を送出する。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    joined = list.concat([1, 2], [3, 4])
    assert(list.len(joined) == 4)
}
```

### len

<!-- stdlib:sig:list.len start -->

```yaoxiang
len: (A: Type) -> (list: &Vec(A)) -> Int
```

<!-- stdlib:sig:list.len end -->

要素数。読み取り専用借用で、`list` は繰り返し使用可能。

エラー：引数がリストでない場合、`E6007` を送出する。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    nums = [1, 2, 3]
    assert(list.len(nums) == 3)
    assert(list.len(nums) == 3)      // 再利用可能
}
```

### is_empty

<!-- stdlib:sig:list.is_empty start -->

```yaoxiang
is_empty: (A: Type) -> (list: &Vec(A)) -> Bool
```

<!-- stdlib:sig:list.is_empty end -->

リストが空かどうかを返す。

エラー：引数がリストでない場合、`E6007` を送出する。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    assert(list.is_empty([]))
    assert(!list.is_empty([1]))
}
```

### contains

<!-- stdlib:sig:list.contains start -->

```yaoxiang
contains: (A: Type) -> (list: &Vec(A), item: A) -> Bool
```

<!-- stdlib:sig:list.contains end -->

`item` がリスト内に存在するかを判定する（値による等価比較）。

戻り値：存在する場合は `true`。引数がリストでない場合は `false` を返す。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    nums = [1, 2, 3, 4]
    assert(list.contains(nums, 3))
    assert(!list.contains(nums, 99))
}
```

### find_index

<!-- stdlib:sig:list.find_index start -->

```yaoxiang
find_index: (A: Type) -> (list: &Vec(A), item: A) -> Int
```

<!-- stdlib:sig:list.find_index end -->

`item` が初出するインデックスを返す。

戻り値：見つかった場合はインデックスを返し、見つからない場合は `-1` を返す。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    assert(list.find_index([1, 2, 3, 4], 3) == 2)
    assert(list.find_index([1, 2], 99) == -1)
}
```

### map

<!-- stdlib:sig:list.map start -->

```yaoxiang
map: (T: Type, R: Type) -> (list: &Vec(T), f: (item: T) -> R) -> Vec(R)
```

<!-- stdlib:sig:list.map end -->

各要素に対して `fn`
を呼び出し、結果を組み合わせた新しいリストを返す。関数値を渡すのは**カリー化**形式：`list.map(nums, x => x * 2)`。ソースリストは変更されない。

エラー：第 2 引数が関数でない場合、`E6007` を送出する。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    doubled = list.map([1, 2, 3], x => x * 2)
    assert(list.get(doubled, 0) == 2)
}
```

### filter

<!-- stdlib:sig:list.filter start -->

```yaoxiang
filter: (T: Type) -> (list: &Vec(T), keep: (item: T) -> Bool) -> Vec(T)
```

<!-- stdlib:sig:list.filter end -->

`fn` が真となる要素を保持する。ソースリストは変更されない。

エラー：第 2 引数が関数でない場合、`E6007` を送出する。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    evens = list.filter([1, 2, 3, 4], x => x % 2 == 0)
    assert(list.len(evens) == 2)
}
```

### reduce

<!-- stdlib:sig:list.reduce start -->

```yaoxiang
reduce: (T: Type, Acc: Type) -> (list: &Vec(T), f: (acc: Acc, item: T) -> Acc, init: Acc) -> Acc
```

<!-- stdlib:sig:list.reduce end -->

左から右へ畳み込みを行う。`init` を初期値として、`fn(acc, item)` を順に呼び出す。

- `fn` ―― 畳み込み関数 `(累積値, 要素) -> 新しい累積値`
- `init` ―― 初期累積値

戻り値：最終累積値。リストが空の場合は `init` を返す。

エラー：第 2 引数が関数でない場合、`E6007` を送出する。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    total = list.reduce([1, 2, 3, 4], (acc, x) => acc + x, 0)
    assert(total == 10)
}
```

### iter

<!-- stdlib:sig:list.iter start -->

```yaoxiang
iter: (T: Type) -> (list: Vec(T)) -> Iter(T)
```

<!-- stdlib:sig:list.iter end -->

イテレータを作成する。イテレータは `(リスト, インデックス)` のタプル状態キャリアで、作成後 `next`
内で**順番に消費**される。ソースリストは読み取り専用で借用され、反復中も使用可能。

戻り値：イテレータタプル。`next` / `has_next` に渡して使用する。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    it = list.iter([1, 2, 3])
    assert(list.has_next(it))
}
```

### next

<!-- stdlib:sig:list.next start -->

```yaoxiang
next: (T: Type) -> (it: &mut Iter(T)) -> T
```

<!-- stdlib:sig:list.next end -->

現在の要素を取り出し、内部インデックスを 1 つ進める。

戻り値：現在の要素。反復終了時は `Void` を返す。

> `next` と `has_next` はどちらもイテレータを**ムーブ**する（シグネチャに `&`
> が無い）ため、取得するたびにイテレータを再作成する必要がある。あるいは `for ... in`
> で直接反復する。これは [`std.range.next`](./range#next) の借用形式とは異なる。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    it = list.iter([7, 8])
    assert(list.next(it) == 7)
}
```

### has_next

<!-- stdlib:sig:list.has_next start -->

```yaoxiang
has_next: (T: Type) -> (it: &Iter(T)) -> Bool
```

<!-- stdlib:sig:list.has_next end -->

未消費の要素があるかどうかを返す。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    it = list.iter([1])
    assert(list.has_next(it))
}
```

### for ... in による反復

リストは `for ... in` で直接反復でき、`next` を手動で呼び出す必要はない：

```yaoxiang
use std.assert

main: () -> Void = {
    mut sum = 0
    for x in [1, 2, 3] {
        sum = sum + x
    }
    assert(sum == 6)
}
```

## 関連

- [`std.range`](./range) ―― 区間反復と遅延アダプタ
- [`std.assert`](./assert) ―― 例で使用されているアサートツール

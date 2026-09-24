---
title: 'std.list'
description: 'リストの追加・削除、スライス、高階関数とイテレータプロトコル'
---

# std.list

リスト操作モジュール。**ムーブセマンティクスには特に注意が必要**です。関数には2種類あり、ソースリストを**読み取り専用借用**するものと、ソースリストを**消費（ムーブ）**するものがあります。
`&` 仮引数の自動借用ルールについては RFC-009
§2.8 を参照してください。実引数が呼び出し後にまだ使用される場合、コンパイラが自動的に読み取り専用トークンを作成します。

```yaoxiang
use std.list
```

## セマンティクス分類

シグネチャに `&`
を含む仮引数は読み取り専用借用であり、呼び出し後もソース値はそのまま使用可能です。`&`
の付かない仮引数は値渡しであり、呼び出し後にソース値は**ムーブ済み**となり、再利用しようとすると
`E2014` が発生します。

| カテゴリ               | 関数                                                                                                             | 動作                                       |
| ---------------------- | ---------------------------------------------------------------------------------------------------------------- | ------------------------------------------ |
| **ソースリストを消費** | `push` `append` `prepend` `set` `pop` `remove_at`                                                                | ソースリストはムーブされ、以後は再利用不可 |
| 読み取り専用借用       | `len` `is_empty` `get` `first` `last` `slice` `reverse` `concat` `contains` `find_index` `map` `filter` `reduce` | ソースリストは繰り返し使用可能             |
| イテレータプロトコル   | `iter`（ソースリストを消費し、イテレータを返す）`has_next` `next`（イテレータの借用 / 可変借用）                 | 下記参照                                   |

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    nums = [1, 2, 3]

    // 読み取り専用借用：nums は繰り返し使用可能
    assert(list.len(nums) == 3)
    assert(list.len(nums) == 3)
    assert(list.contains(nums, 2))

    // 消費：base はこの後は再利用不可
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

`list` の末尾に `item` を追加した**新しいリスト**を返します。`list`
は値渡しで渡されるため、呼び出し後は**ムーブ済み**となり再利用できません。

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

`push` の別名であり、動作は完全に同一です。

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

`list` の先頭に `item` を挿入した新しいリストを返します。`list`
は値渡しで渡されるため、呼び出し後は**ムーブ済み**となります。

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

末尾の要素を取り除き、**短縮されたリスト**を返します（値セマンティクス）。ソースリストは消費され、もはやネイティブ版で「シグネチャに
`&` が付いているのにソース値をその場で変更する」例外形態ではありません。

戻り値：末尾の要素を取り除いた新しいリスト。リストが空の場合はそのまま返されます。取り除かれた要素を読み取るには、呼び出し前に
`last` を使って値を取得してください。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    l = [1, 2, 3]
    rest = list.pop(l)               // l は消費され、rest は短縮された新しいリスト
    assert(list.len(rest) == 2)
    assert(list.last(rest) == 2)     // 末尾要素 3 は取り除かれた

    // 取り除かれた要素を読み取るには、先に last で取得してから pop
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

下標 `index`
の要素を取り除き、**短縮された新しいリスト**を返します（値セマンティクス）。ソースリストは消費されます。

- `index` —— 要素の下標

戻り値：その要素を取り除いた新しいリスト。エラー：下標が負または ≥ 長さの場合、`E6003`（インデックス範囲外）が送出されます。

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

下標 `index` を `value` に書き換えた新しいリストを返します。`list`
は値渡しで渡されるため、呼び出し後は**ムーブ済み**となります。

- `index` —— 下標；デフォルト `0`
- `value` —— 新しい値；デフォルト `Void`

エラー：下標が負または ≥ 長さの場合、`E6003`
が送出されます（範囲外への書き込みはもはや静かに破棄されません）。

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

下標 `index` の要素を読み取ります（読み取り専用借用、`list` は再利用可能）。

- `index` —— 下標；デフォルト `0`

戻り値：要素の値；**範囲外の場合は `Void`
を返します**（エラーは送出されません）。エラー：下標が負の場合、`E6007` が送出されます。

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

先頭要素を返します。空のリストの場合は `Void` を返します。

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

末尾要素を返します。空のリストの場合は `Void` を返します。

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

`[start, end)` 区間のサブリストを取得します。

- `start` —— 開始下標；デフォルト `0`
- `end` —— 終了下標（含まない）；デフォルトはリストの末尾

戻り値：新しいリスト。境界は有効な範囲に**クランプ**され、エラーは発生しません。エラー：`start`
または `end` が負の場合、`E6007` が送出されます。

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

要素の順序を反転した新しいリストを返します。ソースリストは変更されません。

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

2つのリストを連結し、新しいリストを返します。両方のソースリストは変更されません。

エラー：第2引数がリストでない場合、`E6007` が送出されます。

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

要素の個数。読み取り専用借用なので、`list` は繰り返し使用可能です。

エラー：引数がリストでない場合、`E6007` が送出されます。

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

リストが空かどうかを判定します。

エラー：引数がリストでない場合、`E6007` が送出されます。

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

`item` がリスト内に存在するかどうかを判定します（値の等価比較。要素の型は `==`
をサポートしている必要があります。基礎型はネイティブでサポートし、記録型は RFC-011b の `Equal`
によって自動導出または明示的なインスタンス化で提供されます）。

戻り値：存在する場合は `true`；引数がリストでない場合は `false`。

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

`item` が最初に出現する下標を返します。

戻り値：見つかった場合は下標；見つからなかった場合は `-1`。

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
を呼び出し、結果を要素とする新しいリストを返します。関数値を渡すのは**カリー化**形式です：
`list.map(nums, x => x * 2)`。ソースリストは変更されません。

エラー：第2引数が関数でない場合、`E6007` が送出されます。

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

`fn` が真を返す要素のみを保持します。ソースリストは変更されません。

エラー：第2引数が関数でない場合、`E6007` が送出されます。

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

左から右へ畳み込みます：`init` を初期値として、`fn(acc, item)` を順次呼び出します。

- `fn` —— 集約関数 `(累積値, 要素) -> 新しい累積値`
- `init` —— 初期累積値

戻り値：最終的な累積値。リストが空の場合は `init` を返します。

エラー：第2引数が関数でない場合、`E6007` が送出されます。

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

イテレータを作成します。イテレータは `(リスト, 下標)` のタプル状態を持ち、作成後に `next`
で**順次消費**されます。ソースリストは読み取り専用借用されるため、イテレーション中も使用可能です。

戻り値：`next` / `has_next` に渡すイテレータタプル。

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

現在の要素を取り出し、内部の下標を1つ進めます。

戻り値：現在の要素；イテレーション終了時は `Void`。

> `next` と `has_next` はどちらもイテレータを**ムーブ**します（シグネチャに `&`
> が付かない）。したがって、取得するたびに毎回イテレータを作り直すか、 `for ... in`
> で直接走査してください。これは [`std.range.next`](./range#next) の借用形式とは異なります。

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

未消費の要素がまだあるかどうかを判定します。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    it = list.iter([1])
    assert(list.has_next(it))
}
```

### for ... in による走査

リストは `for ... in` で直接走査でき、手動で `next` を呼び出す必要はありません：

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

## 関連項目

- [`std.range`](./range) —— 区間イテレーションと遅延アダプタ
- [`std.assert`](./assert) —— 例で使用するアサーション

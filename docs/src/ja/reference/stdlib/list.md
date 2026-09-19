---
title: 'std.list'
description: 'リストの追加・削除、スライス、高階関数、イテレータプロトコル'
---

# std.list

リスト操作モジュール。**ムーブセマンティクスには特に注意が必要**です。一部の関数はソースリストを読み取り専用で借用し、別の一部はソースリストを消費（ムーブ）し、さらに借用と明記されているにもかかわらずインプレースでリストを変更する関数も 2 つあります。

```yaoxiang
use std.list
```

## セマンティクスの分類

シグネチャに `&`
が含まれるパラメータは読み取り専用の借用で、呼び出し後もソース値は引き続き使用できます。`&`
のないパラメータは値渡しで、呼び出し後はソース値が**ムーブ済み**となり、再使用すると `E2014`
が発生します。

| カテゴリ                     | 関数                                                                                                                    | 振る舞い                                 |
| ---------------------------- | ----------------------------------------------------------------------------------------------------------------------- | ---------------------------------------- |
| **ソースリストを消費**       | `push` `append` `prepend` `set`                                                                                         | ソースリストがムーブされ、以降は使用不可 |
| 読み取り専用借用             | `len` `is_empty` `get` `first` `last` `slice` `reverse` `concat` `contains` `find_index` `map` `filter` `reduce` `iter` | ソースリストは繰り返し使用可能           |
| 借用だが**インプレース変更** | `pop` `remove_at`                                                                                                       | ソースリストの内容が変更される           |
| イテレータを消費             | `next` `has_next`                                                                                                       | イテレータのタプルがムーブされる         |

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    nums = [1, 2, 3]

    // 読み取り専用借用：nums は繰り返し使用可能
    assert(list.len(nums) == 3)
    assert(list.len(nums) == 3)
    assert(list.contains(nums, 2))

    // 消費：base はこれ以降使用不可
    base = [1, 2]
    extended = list.push(base, 3)
    assert(list.len(extended) == 3)
}
```

## 関数一覧

<!-- stdlib:table:list start -->

| 関数         | シグネチャ                                                                    |
| ------------ | ----------------------------------------------------------------------------- |
| `push`       | `(A: Type)(list: List(A), item: A) -> List(A)`                                |
| `pop`        | `(A: Type)(list: &List(A)) -> Any`                                            |
| `append`     | `(A: Type)(list: List(A), item: A) -> List(A)`                                |
| `prepend`    | `(A: Type)(list: List(A), item: A) -> List(A)`                                |
| `remove_at`  | `(A: Type)(list: &List(A), index: Int) -> Any`                                |
| `reverse`    | `(A: Type)(list: &List(A)) -> List(A)`                                        |
| `concat`     | `(A: Type)(a: &List(A), b: &List(A)) -> List(A)`                              |
| `map`        | `(T: Type)(list: &List(T), fn: (item: T) -> T) -> List(T)`                    |
| `filter`     | `(T: Type)(list: &List(T), fn: (item: T) -> Bool) -> List(T)`                 |
| `reduce`     | `(T: Type)(list: &List(T), fn: (acc: Any, item: T) -> Any, init: Any) -> Any` |
| `len`        | `(A: Type)(list: &List(A)) -> Int`                                            |
| `is_empty`   | `(A: Type)(list: &List(A)) -> Bool`                                           |
| `get`        | `(A: Type)(list: &List(A), index: Int) -> Any`                                |
| `set`        | `(A: Type)(list: List(A), index: Int, value: A) -> List(A)`                   |
| `first`      | `(A: Type)(list: &List(A)) -> Any`                                            |
| `last`       | `(A: Type)(list: &List(A)) -> Any`                                            |
| `slice`      | `(A: Type)(list: &List(A), start: Int, end: Int) -> List(A)`                  |
| `contains`   | `(A: Type)(list: &List(A), item: Any) -> Bool`                                |
| `find_index` | `(A: Type)(list: &List(A), item: Any) -> Int`                                 |
| `iter`       | `(A: Type)(list: &List(A)) -> Tuple`                                          |
| `next`       | `(iterator: Tuple) -> Any`                                                    |
| `has_next`   | `(iterator: Tuple) -> Bool`                                                   |

<!-- stdlib:table:list end -->## 関数

### push

<!-- stdlib:sig:list.push start -->

```yaoxiang
push: (A: Type)(list: List(A), item: A) -> List(A)
```

<!-- stdlib:sig:list.push end -->

`list` の末尾に `item` を追加した**新しいリスト**を返します。`list`
は値渡しで、呼び出し後は**ムーブ**され、再使用できません。

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
append: (A: Type)(list: List(A), item: A) -> List(A)
```

<!-- stdlib:sig:list.append end -->

`push` の別名で、振る舞いは完全に同一です。

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
prepend: (A: Type)(list: List(A), item: A) -> List(A)
```

<!-- stdlib:sig:list.prepend end -->

`list` の先頭に `item` を挿入した新しいリストを返します。`list`
は値渡しで、呼び出し後は**ムーブ**されます。

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
pop: (A: Type)(list: &List(A)) -> Any
```

<!-- stdlib:sig:list.pop end -->

末尾の要素を削除して返します。`list` を**インプレース変更**します——シグネチャに `&`
が付くにもかかわらず、ソース値を変更する例外です。

戻り値: 削除された要素。リストが空の場合は `Void` を返し、リストは空のままとなります。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    mut l = [1, 2, 3]
    gone = list.pop(l)
    assert(list.len(l) == 2)         // インプレースで短縮
    assert(gone == 3)

    mut empty = []
    v = list.pop(empty)
    assert(list.is_empty(empty))
}
```

### remove_at

<!-- stdlib:sig:list.remove_at start -->

```yaoxiang
remove_at: (A: Type)(list: &List(A), index: Int) -> Any
```

<!-- stdlib:sig:list.remove_at end -->

インデックス `index` の位置にある要素を削除して返します。`list` を**インプレース変更**します。

- `index` —— 文字/要素インデックス。デフォルトは `0`

戻り値: 削除された要素。エラー: インデックスが負、または長さ以上の場合は
`E6003`（インデックス範囲外）を投げ、リストは変更されません。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    mut l = [10, 20, 30]
    x = list.remove_at(l, 1)
    assert(x == 20)
    assert(list.len(l) == 2)
}
```

### set

<!-- stdlib:sig:list.set start -->

```yaoxiang
set: (A: Type)(list: List(A), index: Int, value: A) -> List(A)
```

<!-- stdlib:sig:list.set end -->

インデックス `index` を `value` に書き換えた新しいリストを返します。`list`
は値渡しで、呼び出し後は**ムーブ**されます。

- `index` —— インデックス。デフォルトは `0`
- `value` —— 新しい値。デフォルトは `Void`

エラー: インデックスが負、または長さ以上の場合は
`E6003`（範囲外書き込みはもはや静かに破棄されません）を投げます。

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
get: (A: Type)(list: &List(A), index: Int) -> Any
```

<!-- stdlib:sig:list.get end -->

インデックス `index` の位置にある要素を読み取ります（読み取り専用借用、`list` は再利用可能）。

- `index` —— インデックス。デフォルトは `0`

戻り値: 要素の値。**範囲外の場合は `Void`
を返します**（エラーを投げません）。エラー: インデックスが負の場合は `E6007` を投げます。

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
first: (A: Type)(list: &List(A)) -> Any
```

<!-- stdlib:sig:list.first end -->

先頭要素を返します。空のリストでは `Void` を返します。

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
last: (A: Type)(list: &List(A)) -> Any
```

<!-- stdlib:sig:list.last end -->

末尾要素を返します。空のリストでは `Void` を返します。

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
slice: (A: Type)(list: &List(A), start: Int, end: Int) -> List(A)
```

<!-- stdlib:sig:list.slice end -->

`[start, end)` の範囲の部分リストを取得します。

- `start` —— 開始インデックス。デフォルトは `0`
- `end` —— 終了インデックス（含まない）。デフォルトはリストの末尾

戻り値: 新しいリスト。境界は有効な範囲に**クランプ**され、エラーは発生しません。エラー: `start`
または `end` が負の場合は `E6007` を投げます。

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
reverse: (A: Type)(list: &List(A)) -> List(A)
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
concat: (A: Type)(a: &List(A), b: &List(A)) -> List(A)
```

<!-- stdlib:sig:list.concat end -->

2 つのリストを連結し、新しいリストを返します。両方のソースリストは変更されません。

エラー: 第 2 引数がリストでない場合は `E6007` を投げます。

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
len: (A: Type)(list: &List(A)) -> Int
```

<!-- stdlib:sig:list.len end -->

要素数。読み取り専用借用で、`list` は繰り返し使用可能です。

エラー: 引数がリストでない場合は `E6007` を投げます。

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
is_empty: (A: Type)(list: &List(A)) -> Bool
```

<!-- stdlib:sig:list.is_empty end -->

リストが空かどうかを判定します。

エラー: 引数がリストでない場合は `E6007` を投げます。

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
contains: (A: Type)(list: &List(A), item: Any) -> Bool
```

<!-- stdlib:sig:list.contains end -->

`item` がリスト内に存在するかどうか（値の等価比較）。

戻り値: 存在すれば `true`。引数がリストでない場合は `false` を返します。

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
find_index: (A: Type)(list: &List(A), item: Any) -> Int
```

<!-- stdlib:sig:list.find_index end -->

`item` が最初に現れるインデックス。

戻り値: 見つかった場合はインデックス、見つからない場合は `-1`。

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
map: (T: Type)(list: &List(T), fn: (item: T) -> T) -> List(T)
```

<!-- stdlib:sig:list.map end -->

各要素に対して `fn`
を呼び出し、結果を組み合わせた新しいリストを返します。関数値を渡すのは**カリー化**の形式です:
`list.map(nums, x => x * 2)`。ソースリストは変更されません。

エラー: 第 2 引数が関数でない場合は `E6007` を投げます。

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
filter: (T: Type)(list: &List(T), fn: (item: T) -> Bool) -> List(T)
```

<!-- stdlib:sig:list.filter end -->

`fn` が真となる要素を保持します。ソースリストは変更されません。

エラー: 第 2 引数が関数でない場合は `E6007` を投げます。

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
reduce: (T: Type)(list: &List(T), fn: (acc: Any, item: T) -> Any, init: Any) -> Any
```

<!-- stdlib:sig:list.reduce end -->

左から右へ畳み込みます。`init` を初期値として、`fn(acc, item)` を順に呼び出します。

- `fn` —— リダクション関数 `(累積値, 要素) -> 新しい累積値`
- `init` —— 初期累積値

戻り値: 最終的な累積値。リストが空の場合は `init` を返します。

エラー: 第 2 引数が関数でない場合は `E6007` を投げます。

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
iter: (A: Type)(list: &List(A)) -> Tuple
```

<!-- stdlib:sig:list.iter end -->

イテレータを作成します。イテレータは `(リスト, インデックス)` のタプルによる状態キャリアで、作成後
`next` 内で
**順番に消費**されます。ソースリストは読み取り専用の借用で、イテレート中も引き続き使用できます。

戻り値: イテレータタプル。`next` / `has_next` に渡して使用します。

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
next: (iterator: Tuple) -> Any
```

<!-- stdlib:sig:list.next end -->

現在の要素を取り出し、内部インデックスを 1 つ進めます。

戻り値: 現在の要素。イテレーション終了時は `Void` を返します。

> `next` と `has_next` はどちらもイテレータを**ムーブ**します（シグネチャに `&`
> がありません）。したがって、取得のたびにイテレータを再作成するか、 `for ... in`
> で直接反復処理する必要があります。これは [`std.range.next`](./range#next)
> の借用形式とは異なります。

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
has_next: (iterator: Tuple) -> Bool
```

<!-- stdlib:sig:list.has_next end -->

まだ消費されていない要素があるかどうかを判定します。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    it = list.iter([1])
    assert(list.has_next(it))
}
```

### for ... in による反復

リストは `for ... in` で直接反復処理でき、手動で `next` を呼び出す必要はありません:

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
- [`std.assert`](./assert) —— 例で使用するアサーション utility

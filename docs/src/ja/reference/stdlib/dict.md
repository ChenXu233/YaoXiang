---
title: 'std.dict'
description: '辞書の読み書き、キーと値のビュー、マージ'
---

# std.dict

辞書（`Dict(K, V)`）操作モジュール。

```yaoxiang
use std.dict
```

## 意味分類

| カテゴリ             | 関数                                                           | 動作                                   |
| -------------------- | -------------------------------------------------------------- | -------------------------------------- |
| 読み取り専用借用     | `get` `has` `values` `keys` `entries` `len` `is_empty` `merge` | ソース辞書は再利用可能                 |
| **ソース辞書を消費** | `set` `delete`                                                 | ソース辞書がムーブされ、使用不可となる |

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    d = dict.set(dict.new(), "a", 1)

    // 読み取り専用借用：d は再利用可能
    assert(dict.get(d, "a") == 1)
    assert(dict.len(d) == 1)
    assert(dict.has(d, "a"))
}
```

## 関数一覧

<!-- stdlib:table:dict start -->

| 関数       | シグネチャ                                                                 |
| ---------- | -------------------------------------------------------------------------- |
| `get`      | `(K: Type, V: Type)(dict: &Dict(K, V), key: Any) -> Any`                   |
| `set`      | `(K: Type, V: Type)(dict: Dict(K, V), key: Any, value: Any) -> Dict(K, V)` |
| `has`      | `(K: Type, V: Type)(dict: &Dict(K, V), key: Any) -> Bool`                  |
| `values`   | `(A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> Vec(C)`                 |
| `keys`     | `(A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> Vec(C)`                 |
| `entries`  | `(A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> Vec(C)`                 |
| `delete`   | `(K: Type, V: Type)(dict: Dict(K, V), key: Any) -> Dict(K, V)`             |
| `len`      | `(K: Type, V: Type)(dict: &Dict(K, V)) -> Int`                             |
| `is_empty` | `(K: Type, V: Type)(dict: &Dict(K, V)) -> Bool`                            |
| `merge`    | `(A: Type, B: Type)(a: &Dict(A, B), b: &Dict(A, B)) -> Dict(A, B)`         |
| `new`      | `(K: Type, V: Type)() -> Dict(K, V)`                                       |

<!-- stdlib:table:dict end -->## 関数

### set

<!-- stdlib:sig:dict.set start -->

```yaoxiang
set: (K: Type, V: Type)(dict: Dict(K, V), key: Any, value: Any) -> Dict(K, V)
```

<!-- stdlib:sig:dict.set end -->

`key` → `value` を書き込んだ**新しい辞書**を返す。`dict`
は値渡しされ、呼び出し後に**ムーブ**される。

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    d1 = dict.set(dict.new(), "a", 1)
    d2 = dict.set(d1, "b", 2)
    assert(dict.len(d2) == 2)
}
```

### get

<!-- stdlib:sig:dict.get start -->

```yaoxiang
get: (K: Type, V: Type)(dict: &Dict(K, V), key: Any) -> Any
```

<!-- stdlib:sig:dict.get end -->

キーで値を取得（読み取り専用借用、`dict` は再利用可能）。

戻り値：キーに対応する値。エラー：**キーが存在しない場合、`E6008`をスロー**（キー欠落）。値を取得する前に
[`has`](#has) で先に確認できる。

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    d = dict.set(dict.new(), "a", 1)
    assert(dict.get(d, "a") == 1)
}
```

存在を確認してから取得する場合：

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    d = dict.set(dict.new(), "a", 1)
    assert(!dict.has(d, "nope"))
    if dict.has(d, "a") {
        assert(dict.get(d, "a") == 1)
    }
}
```

### has

<!-- stdlib:sig:dict.has start -->

```yaoxiang
has: (K: Type, V: Type)(dict: &Dict(K, V), key: Any) -> Bool
```

<!-- stdlib:sig:dict.has end -->

辞書内に `key` が存在するかどうか。

エラー：第1引数が辞書でない場合、`E6007`をスロー。

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    d = dict.set(dict.new(), "a", 1)
    assert(dict.has(d, "a"))
    assert(!dict.has(d, "zzz"))
}
```

### delete

<!-- stdlib:sig:dict.delete start -->

```yaoxiang
delete: (K: Type, V: Type)(dict: Dict(K, V), key: Any) -> Dict(K, V)
```

<!-- stdlib:sig:dict.delete end -->

`key` を削除した**新しい辞書**を返す。`dict` は値渡しされ、呼び出し後に**ムーブ**される。

戻り値：新しい辞書。存在しないキーを削除してもエラーは発生せず、辞書は変更されない。

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    d = dict.set(dict.new(), "a", 1)
    deleted = dict.delete(d, "a")
    assert(!dict.has(deleted, "a"))
}
```

### keys

<!-- stdlib:sig:dict.keys start -->

```yaoxiang
keys: (A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> Vec(C)
```

<!-- stdlib:sig:dict.keys end -->

すべてのキーで構成されるリストを返す（読み取り専用借用）。

> 戻り値の順序はハッシュ実装に依存し、**安定性は保証されない**。順序が必要な場合は各自でソートすること。

```yaoxiang
use std.assert
use std.dict
use std.list

main: () -> Void = {
    d = dict.set(dict.new(), "a", 1)
    ks = dict.keys(d)
    assert(list.len(ks) == 1)
}
```

### values

<!-- stdlib:sig:dict.values start -->

```yaoxiang
values: (A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> Vec(C)
```

<!-- stdlib:sig:dict.values end -->

すべての値で構成されるリストを返す（読み取り専用借用）。順序の安定性は保証されない。

```yaoxiang
use std.assert
use std.dict
use std.list

main: () -> Void = {
    d = dict.set(dict.new(), "a", 1)
    vs = dict.values(d)
    assert(list.len(vs) == 1)
}
```

### entries

<!-- stdlib:sig:dict.entries start -->

```yaoxiang
entries: (A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> Vec(C)
```

<!-- stdlib:sig:dict.entries end -->

キーと値のペアのリストを返し、各要素は `(key, value)` のタプル。順序の安定性は保証されない。

```yaoxiang
use std.assert
use std.dict
use std.list

main: () -> Void = {
    d = dict.set(dict.new(), "a", 1)
    es = dict.entries(d)
    assert(list.len(es) == 1)
}
```

### len

<!-- stdlib:sig:dict.len start -->

```yaoxiang
len: (K: Type, V: Type)(dict: &Dict(K, V)) -> Int
```

<!-- stdlib:sig:dict.len end -->

エントリ数。読み取り専用借用で、`dict` は再利用可能。

エラー：引数が辞書でない場合、`E6007`をスロー。

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    d = dict.set(dict.new(), "a", 1)
    assert(dict.len(d) == 1)
    assert(dict.len(d) == 1)      // 再利用可能
}
```

### is_empty

<!-- stdlib:sig:dict.is_empty start -->

```yaoxiang
is_empty: (K: Type, V: Type)(dict: &Dict(K, V)) -> Bool
```

<!-- stdlib:sig:dict.is_empty end -->

辞書が空かどうか。

エラー：引数が辞書でない場合、`E6007`をスロー。

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    assert(dict.is_empty(dict.new()))
    d = dict.set(dict.new(), "a", 1)
    assert(!dict.is_empty(d))
}
```

### merge

<!-- stdlib:sig:dict.merge start -->

```yaoxiang
merge: (A: Type, B: Type)(a: &Dict(A, B), b: &Dict(A, B)) -> Dict(A, B)
```

<!-- stdlib:sig:dict.merge end -->

2つの辞書をマージし、新しい辞書を返す。両方のソース辞書は読み取り専用借用で、変更されない。

キーが衝突した場合、**`b` の値で `a` を上書き**する。

エラー：いずれかの引数が辞書でない場合、`E6007`をスロー。

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    m = dict.merge(dict.set(dict.new(), "x", 10), dict.set(dict.new(), "y", 20))
    assert(dict.get(m, "x") == 10)
    assert(dict.get(m, "y") == 20)
}
```

## 関連

- [`std.list`](./list) —— `keys` / `values` / `entries` の戻り値を処理
- [エラーコードリファレンス](../error-code/) —— `E6008` キー欠落

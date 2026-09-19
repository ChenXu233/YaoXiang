---
title: 'std.dict'
description: '辞書の読み書き、キー・値ビュー、マージ'
---

# std.dict

辞書（`Dict(K, V)`）操作モジュール。

```yaoxiang
use std.dict
```

## 意味分類

| カテゴリ           | 関数                                                           | 動作                             |
| ------------------ | -------------------------------------------------------------- | -------------------------------- |
| 読み取り借用       | `get` `has` `values` `keys` `entries` `len` `is_empty` `merge` | 元の辞書は再利用可能             |
| **元の辞書を消費** | `set` `delete`                                                 | 元の辞書はムーブされ、再利用不可 |

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    d = dict.set({}, "a", 1)

    // 只读借用：d 可反复使用
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
| `values`   | `(A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> List(C)`                |
| `keys`     | `(A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> List(C)`                |
| `entries`  | `(A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> List(C)`                |
| `delete`   | `(K: Type, V: Type)(dict: Dict(K, V), key: Any) -> Dict(K, V)`             |
| `len`      | `(K: Type, V: Type)(dict: &Dict(K, V)) -> Int`                             |
| `is_empty` | `(K: Type, V: Type)(dict: &Dict(K, V)) -> Bool`                            |
| `merge`    | `(A: Type, B: Type)(a: &Dict(A, B), b: &Dict(A, B)) -> Dict(A, B)`         |

<!-- stdlib:table:dict end -->

## 関数

### set

<!-- stdlib:sig:dict.set start -->

```yaoxiang
set: (K: Type, V: Type)(dict: Dict(K, V), key: Any, value: Any) -> Dict(K, V)
```

<!-- stdlib:sig:dict.set end -->

`key` → `value` を書き込んだ**新しい辞書**を返します。`dict`
は値で渡され、呼び出し後に**ムーブ**されます。

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    d1 = dict.set({}, "a", 1)
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

キーで値を取得（読み取り借用、`dict` は再利用可能）。

戻り値：キーに対応する値。エラー：**キーが存在しない場合 `E6008`
をスロー**（キー欠落）。値を取得する前に [`has`](#has) で確認できます。

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    d = dict.set({}, "a", 1)
    assert(dict.get(d, "a") == 1)
}
```

存在確認後に取得：

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    d = dict.set({}, "a", 1)
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

辞書に `key` が存在するかどうか。

エラー：第 1 引数が辞書でない場合 `E6007` をスロー。

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    d = dict.set({}, "a", 1)
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

`key` を削除した**新しい辞書**を返します。`dict` は値で渡され、呼び出し後に**ムーブ**されます。

戻り値：新しい辞書。存在しないキーを削除してもエラーにならず、辞書は変更されません。

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    d = dict.set({}, "a", 1)
    deleted = dict.delete(d, "a")
    assert(!dict.has(deleted, "a"))
}
```

### keys

<!-- stdlib:sig:dict.keys start -->

```yaoxiang
keys: (A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> List(C)
```

<!-- stdlib:sig:dict.keys end -->

すべてのキーを含むリストを返します（読み取り借用）。

> 戻り値の順序はハッシュ実装に依存し、**安定性は保証されません**。順序付き出力が必要な場合はご自身でソートしてください。

```yaoxiang
use std.assert
use std.dict
use std.list

main: () -> Void = {
    d = dict.set({}, "a", 1)
    ks = dict.keys(d)
    assert(list.len(ks) == 1)
}
```

### values

<!-- stdlib:sig:dict.values start -->

```yaoxiang
values: (A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> List(C)
```

<!-- stdlib:sig:dict.values end -->

すべての値を含むリストを返します（読み取り借用）。順序の安定性は保証されません。

```yaoxiang
use std.assert
use std.dict
use std.list

main: () -> Void = {
    d = dict.set({}, "a", 1)
    vs = dict.values(d)
    assert(list.len(vs) == 1)
}
```

### entries

<!-- stdlib:sig:dict.entries start -->

```yaoxiang
entries: (A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> List(C)
```

<!-- stdlib:sig:dict.entries end -->

キーと値のペアのリストを返し、各要素は `(key, value)` のタプルです。順序の安定性は保証されません。

```yaoxiang
use std.assert
use std.dict
use std.list

main: () -> Void = {
    d = dict.set({}, "a", 1)
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

エントリ数。読み取り借用で、`dict` は再利用可能。

エラー：引数が辞書でない場合 `E6007` をスロー。

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    d = dict.set({}, "a", 1)
    assert(dict.len(d) == 1)
    assert(dict.len(d) == 1)      // 可复用
}
```

### is_empty

<!-- stdlib:sig:dict.is_empty start -->

```yaoxiang
is_empty: (K: Type, V: Type)(dict: &Dict(K, V)) -> Bool
```

<!-- stdlib:sig:dict.is_empty end -->

辞書が空かどうか。

エラー：引数が辞書でない場合 `E6007` をスロー。

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    assert(dict.is_empty({}))
    d = dict.set({}, "a", 1)
    assert(!dict.is_empty(d))
}
```

### merge

<!-- stdlib:sig:dict.merge start -->

```yaoxiang
merge: (A: Type, B: Type)(a: &Dict(A, B), b: &Dict(A, B)) -> Dict(A, B)
```

<!-- stdlib:sig:dict.merge end -->

2 つの辞書をマージして新しい辞書を返します。両方の元の辞書は読み取り借用で、変更されません。

キーが衝突した場合、`b` の値で `a` を**上書き**します。

エラー：いずれかの引数が辞書でない場合 `E6007` をスロー。

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    m = dict.merge(dict.set({}, "x", 10), dict.set({}, "y", 20))
    assert(dict.get(m, "x") == 10)
    assert(dict.get(m, "y") == 20)
}
```

## 関連

- [`std.list`](./list) —— `keys` / `values` / `entries` の戻り値を処理
- [エラーコードリファレンス](../error-code/) —— `E6008` キー欠落

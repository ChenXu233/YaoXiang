---
title: 'std.assert'
description: 'アサーション'
---

# std.assert

アサーションモジュール。テストとサンプルで最も使用されるツール。

```yaoxiang
use std.assert
```

## 関数一覧

<!-- stdlib:table:assert start -->

| 関数     | シグネチャ                           |
| -------- | ------------------------------------ |
| `assert` | `(cond: Bool, ?msg: String) -> Void` |

<!-- stdlib:table:assert end -->

## 関数

### assert

<!-- stdlib:sig:assert.assert start -->

```yaoxiang
assert: (cond: Bool, ?msg: String) -> Void
```

<!-- stdlib:sig:assert.assert end -->

`cond`が真であることをアサートする。

- `cond` —— 判定するブール式
- `msg` —— 任意メッセージ、`?`は省略可能；条件不成立時に診断情報と一緒に出力される

戻る：条件成立時に`Void`を返し、中断しない。エラー：条件が偽の場合`E6005`（アサーション失敗）をスローし、プログラムは非ゼロで終了する。

```yaoxiang
use std.assert

main: () -> Void = {
    assert(1 > 0)
    assert(1 > 0, "このリテラルアサーションは常に成立する")
}
```

アサーションは**テストスイートの主判定手段**——`src/std/tests/*.yx` と `tests/yaoxiang/**` はどちらも `assert` の失敗時にプロセスエラーで終了する：

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    list.len([1, 2, 3]) == 3
    assert(list.len([1, 2, 3]) == 3, "len == 3")
}
```

## 関連

- [テスト仕様](../../dev/test-specification.md) —— テストスイートの構成と判定規則
- [エラーコード参照](../error-code/) —— `E6005` アサーション失敗
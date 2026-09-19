---
title: 'std.assert'
description: 'アサート'
---

# std.assert

アサートモジュール。テストと例で最もよく使われるツール。

```yaoxiang
use std.assert
```

## 関数一覧

<!-- stdlib:table:assert start -->

| 関数     | 署名                                 |
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

`cond` が真であることをアサートする。

- `cond` —— 判定対象の真偽式
- `msg` —— オプションのメッセージ。`?` は省略可を示す。条件が成立しない場合、診断とともに表示される

戻り値：条件が成立する場合は `Void` を返し、実行を中断しない。エラー：条件が偽の場合に
`E6005`（アサート失敗）をスローし、プログラムは非ゼロコードで終了する。

```yaoxiang
use std.assert

main: () -> Void = {
    assert(1 > 0)
    assert(1 > 0, "このリテラルアサーションは必ず成立する")
}
```

アサートは **テストケースの主要な判定手段** である —— `src/std/tests/*.yx` と `tests/yaoxiang/**`
はすべて、`assert` の失敗によってプロセスがエラーを報告する方式で動作する。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    list.len([1, 2, 3]) == 3
    assert(list.len([1, 2, 3]) == 3, "len == 3")
}
```

## 関連

- [テスト仕様](../dev/test-specification.md) —— テストケースの構成と判定の規約
- [エラーコードリファレンス](../error-code/) —— `E6005` アサート失敗

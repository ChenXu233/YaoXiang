---
title: 'std.assert'
description: 'アサーション'
---

# std.assert

アサーションモジュール。テストとサンプルで最もよく使われるツールです。

```yaoxiang
use std.assert
```

## 関数一覧

<!-- stdlib:table:assert start -->

| 関数     | シグネチャ                            |
| -------- | ------------------------------------- |
| `assert` | `(cond: Bool, ?msg: String) -> Never` |

<!-- stdlib:table:assert end -->

## 関数

### assert

<!-- stdlib:sig:assert.assert start -->

```yaoxiang
assert: (cond: Bool, ?msg: String) -> Never
```

<!-- stdlib:sig:assert.assert end -->

`cond` が真であることをアサートします。

- `cond` —— 判定対象のブール式
- `msg` —— オプションメッセージ。`?`
  は省略可能であることを示します。条件が成立しない場合、診断と一緒に出力されます

戻り値：条件が成立する場合は `Void`
を返し、実行を中断しません。エラー：条件が偽の場合、`E6005`（アサーション失敗）をスローし、プログラムは非ゼロコードで終了します。

```yaoxiang
use std.assert

main: () -> Void = {
    assert(1 > 0)
    assert(1 > 0, "この自明なアサーションは必ず成立する")
}
```

アサーションは**テストコーパスの主要な判定手段**です——`src/std/tests/*.yx` と `tests/yaoxiang/**`
はいずれも、`assert` の失敗が即座にプロセスエラーとなる方式で動作します：

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    list.len([1, 2, 3]) == 3
    assert(list.len([1, 2, 3]) == 3, "len == 3")
}
```

## 関連

- [テスト仕様](../../dev/test-specification.md) —— コーパスの構成と判定規約
- [エラーコードリファレンス](../error-code/) —— `E6005` アサーション失敗

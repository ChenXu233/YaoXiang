---
title: 'std.assert'
description: 'アサーション'
---

# std.assert

アサーションモジュール。テストと例で最もよく使われるツールです。

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

`cond` が真であることをアサートします。

- `cond` —— 判定対象のブール式
- `msg` —— オプションのメッセージ。`?`
  は省略可能を意味します。条件が成立しない場合、診断情報と一緒に出力されます

戻り値: 条件が成立した場合は `Void`
を返し、実行は中断されません。エラー: 条件が偽の場合、`E6005`（アサーション失敗）をスローし、プログラムは非ゼロの終了コードで終了します。

```yaoxiang
use std.assert

main = {
    assert(1 > 0)
    assert(1 > 0, "このリテラルアサーションは必ず成立する")
}
```

アサーションは**テストコーパスの主要な判定手段**です。`src/std/tests/*.yx` と `tests/yaoxiang/**`
はどちらも `assert` の失敗によってプロセスがエラーを報告するという仕組みで動作します：

```yaoxiang
use std.assert
use std.list

main = {
    list.len([1, 2, 3]) == 3
    assert(list.len([1, 2, 3]) == 3, "len == 3")
}
```

## 関連項目

- [テスト仕様](../dev/test-specification.md) —— コーパスの構成と判定の規約
- [エラーコードリファレンス](../error-code/) —— `E6005` アサーション失敗

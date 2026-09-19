---
title: 'std.assert'
description: '断言'
---

# std.assert

断言模块。测试与示例里最常用的工具。

```yaoxiang
use std.assert
```

## 函数一览

<!-- stdlib:table:assert start -->

| 函数     | 签名                                 |
| -------- | ------------------------------------ |
| `assert` | `(cond: Bool, ?msg: String) -> Void` |

<!-- stdlib:table:assert end -->

## 函数

### assert

<!-- stdlib:sig:assert.assert start -->

```yaoxiang
assert: (cond: Bool, ?msg: String) -> Void
```

<!-- stdlib:sig:assert.assert end -->

断言 `cond` 为真。

- `cond` —— 待判定的布尔表达式
- `msg` —— 可选消息，`?` 表示可省略；条件不成立时随诊断一起输出

返回：条件成立时返回 `Void`，不中断执行。错误：条件为假时抛出
`E6005`（断言失败），程序以非零码退出。

```yaoxiang
use std.assert

main: () -> Void = {
    assert(1 > 0)
    assert(1 > 0, "这个字面断言必然成立")
}
```

断言是**测试语料的主要判定手段**——`src/std/tests/*.yx` 与 `tests/yaoxiang/**` 都以 `assert`
失败即进程报错的方式工作：

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    list.len([1, 2, 3]) == 3
    assert(list.len([1, 2, 3]) == 3, "len == 3")
}
```

## 相关

- [测试规范](../dev/test-specification.md) —— 语料组织与判定约定
- [错误码参考](../error-code/) —— `E6005` 断言失败

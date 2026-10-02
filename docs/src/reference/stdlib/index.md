---
title: '标准库总览'
description: 'YaoXiang 标准库模块总览与使用约定'
---

# 标准库参考

YaoXiang 标准库（`std`）以模块为组织单位，每个模块通过 `use` 导入后以 `模块名.函数名(...)`
调用。本目录是按模块拆分的 API 参考文档。

## 模块索引

<!-- stdlib:index:modules start -->

| 模块 | 导出数 | 说明 |
| ---- | ------ | ---- |
| [`std.convert`](./convert) | 11 | 任意值到 String 的转换 |
| [`std.dict`](./dict) | 11 | 字典读写、键值视图与合并 |
| [`std.fs`](./fs) | 22 | 文件、目录与路径操作 |
| [`std.io`](./io) | 4 | 标准输出、标准输入与格式化 |
| [`std.math`](./math) | 18 | 整数、浮点与三角函数，含 PI/E/TAU 常量 |
| [`std.string`](./string) | 21 | 字符串查找、切分、格式化与解析 |
| [`std.time`](./time) | 14 | 时间戳、格式化与 DateTime 字段访问 |
| [`std.result`](./result) | 8 | Result 与 Error 的构造和拆包 |
| [`std.range`](./range) | 10 | 区间迭代、谓词与惰性适配器 |
| [`std.assert`](./assert) | 1 | 断言 |
| [`std.net`](./net) | 4 | HTTP 请求与 URL 百分号编解码 |
| [`std.concurrent`](./concurrent) | 3 | 休眠、让出调度与线程标识 |
| [`std.os`](./os) | 12 | 文件句柄、环境变量与工作目录 |
| [`std.weak`](./weak) | 2 | Arc / Weak 弱引用 |

<!-- stdlib:index:modules end -->

### 纯 YaoXiang 实现的模块

下列模块由 `.yx` 源码实现并随二进制嵌入（`src/std/yx_sources.rs`），**不是** native
`StdModule`，因此不在上表的生成范围内（`src/std/gen_docs.rs:43` 的
`modules_for_docs()` 显式不含它们）。它们的页面是手写页，签名逐字取自 `.yx` 源文件。

| 模块               | 源码                 | 说明                                                 |
| ------------------ | -------------------- | ---------------------------------------------------- |
| [`std.list`](./list)   | `src/std/list.yx`   | 列表增删、切片、高阶函数与迭代器协议                 |
| [`std.json`](./json)   | `src/std/json.yx`   | JSON 解析与序列化（RFC 8259）                        |
| [`std.option`](./option) | `src/std/option.yx` | `Option(T)` 可选值与和类型                           |
| [`std.test`](./test)   | `src/std/test.yx`   | 测试断言库（值语义，RFC-036 §3）                     |

> `std.result` 是**双实现**模块：native 工具族（`src/std/result.rs`）与纯 yx 的
> `Result` 类型（`src/std/result.yx`）合并为同一导出面——`Result` 变体构造走 yx，
> `is_ok` / `unwrap` / `code` 等工具走 native。详见 [`std.result`](./result)。

## 导入约定

模块整体导入：

```yaoxiang
use std.list
use std.string

main: () -> Void = {
    parts = string.split("a,b,c", ",")
    println(list.len(parts))
}
```

也可以从模块中按名导入，包括常量：

```yaoxiang
use std.assert
use std.math.{E, PI, TAU}

main: () -> Void = {
    assert(PI > 3.14)
}
```

## 参数借用约定

签名中的 `&` 表示**只读自动借用**（RFC-009
§2.8）：调用方传入的变量不会被移动，调用后仍可继续使用。这是标准库大量只读函数的默认形态。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    nums = [1, 2, 3]

    // 三处调用都只读借用 nums，之后仍可用
    assert(list.len(nums) == 3)
    assert(list.len(nums) == 3)
    assert(list.contains(nums, 2))
}
```

不带 `&` 的参数表示**按值传入**。多数“修改”函数因此是**消耗源值、返回新值**
的函数式形态，而非原地改写：

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    base = [1, 2]
    extended = list.push(base, 3)   // 返回新列表；base 已被移动
    assert(list.len(extended) == 3)
}
```

各模块页的「语义分类」一节列出该模块哪些函数借用、哪些消耗、哪些原地改动。要特别注意的是
**取值与边界**：`list.get` / `list.first` / `list.last` / `list.slice` 越界或空列表都走
`[]` 的边界检查报 `E6003`，**不返回哨兵值**。

## 错误模型

标准库有两类失败形态，各函数条目中分别以「错误」和「返回 …」标注：

| 形态           | 表现                              | 典型场景                                     |
| -------------- | --------------------------------- | -------------------------------------------- |
| 抛出运行时错误 | 以 `E6xxx` 码终止当前执行         | 字典缺键 `E6008`、索引越界 `E6003`、断言失败 `E6005` |
| 返回哨兵值     | 不中断，返回 `Void` / `-1` / `""` | `list.find_index` 未命中、`string.index_of` 未命中、缺环境变量 |

`list` 是**纯 YaoXiang 实现**（`src/std/list.yx`）：取值类函数直接用 `[]` 下标，不做
边界钳制，越界即 `E6003`（`src/std/list.yx:77-79`），空列表取首末元素同理
（`src/std/list.yx:82-99`）。

常见的运行时错误码：

| 错误码  | 含义                        | 触发示例                         |
| ------- | --------------------------- | -------------------------------- |
| `E6003` | 索引越界                    | `list.set(l, 99, v)`             |
| `E6005` | 断言失败                    | `assert(false)`                  |
| `E6007` | 通用运行时错误              | 文件不存在、`result.unwrap` 失败 |
| `E6008` | 键缺失                      | `dict.get(d, "nope")`            |
| `E6010` | 整数解析失败（作为 Err 值） | `string.parse_int("abc")`        |
| `E6011` | 浮点解析失败（作为 Err 值） | `string.parse_float("abc")`      |

错误码总表见[错误码参考](../error-code/)。

`string.parse_int` / `string.parse_float` 属于第三种形态：**不抛错**，把失败包装成 `Result` 的 `Err`
值返回，可以用 [`std.result`](./result) 拆包或 `?` 传播。

```yaoxiang
use std.assert
use std.result
use std.string

main: () -> Void = {
    assert(result.is_ok(string.parse_int("42")))
    assert(result.is_err(string.parse_int("abc")))
}
```

## 迭代协议

`std.list` 与 `std.range` 都提供 `iter` / `has_next` / `next`，但**两侧的形参形态相反**，
照搬任一侧的写法到另一侧都会不匹配：

| 函数       | `std.list`（`src/std/list.yx`） | `std.range`（`src/std/range.rs`） |
| ---------- | ------------------------------ | ------------------------------- |
| `has_next` | `(it: &Iter(T)) -> Bool`       | `(it: Iterator(Any)) -> Bool`   |
| `next`     | `(it: &mut Iter(T)) -> T`      | `(it: &Iterator(Any)) -> Any`   |

也就是说：`list` 侧**借用**迭代器（`next` 取 `&mut`，`has_next` 取 `&`），同一个迭代器
可以连续取元素；`range` 侧的 `has_next` **按值消耗**迭代器（签名无 `&`），每次判定都要
重新创建，`next` 才借用。日常遍历直接用 `for ... in`，两种形态都不用管。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    it = list.iter([1, 2, 3])
    assert(list.has_next(it))

    // list 侧是借用：同一个迭代器可以连续取
    assert(list.next(it) == 1)
    assert(list.next(it) == 2)
}
```

日常遍历直接用 `for ... in`：

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

[`range.map`](./range#map) / [`range.filter`](./range#filter) 返回**惰性** 适配器，需由 `collect` /
`reduce` / `for_each` / `for ... in` 消费后才产生结果：

```yaoxiang
use std.assert
use std.list
use std.range
use std.result

main: () -> Void = {
    doubled = range.collect(range.map(result.unwrap(range.iter(1..4)), x => x * 2))
    assert(list.get(doubled, 0) == 2)
    assert(list.len(doubled) == 3)
}
```

## 平台可用性

以下内容依赖操作系统能力，在 `wasm32` 目标上**不导出**：

| 范围                                                            | 需要      |
| --------------------------------------------------------------- | --------- |
| `std.os` 全部、`std.net` 全部、`std.weak` 全部                  | 文件/网络 |
| `std.concurrent` 全部                                           | 线程      |
| `std.io.read_line`                                              | 标准 I/O  |
| `std.fs` 全部                                                   | 文件系统  |
| `std.time.sleep`                                                | 线程休眠  |

`std.string` / `std.dict` / `std.math` / `std.convert` / `std.result` / `std.range` /
`std.assert`，纯 yx 的 `std.list` / `std.json` / `std.option` / `std.test`，以及
`std.io.print` / `println` / `format_fallback` 在所有目标上可用。

## 已知缺口

以下条目在撰写文档时逐个真跑示例实测确认：

| 位置 | 原问题 | 现状 |
| ---- | ------ | ---- |
| [`os.open`](./os#open) | 句柄一次性，`open`→`write`→`close` 无法编译 | 句柄改按引用传递 ✅ |
| [`time.datetime_*`](./time#datetime-字段访问) | 8 个访问器无从源码调用（导出名含 `::`）| 改扁平名 `datetime_year` 等 ✅ |
| [`time.parse_time`](./time#parse_time) | `fmt` 参数被忽略；返回值无法继续使用 | 按 fmt 步进解析，返回 `Int` ✅ |
| [`math.clamp`](./math#clamp) | `min > max` 会 panic 解释器而非返回错误 | 返回 `E6007` ✅ |
| [`net.http_get`](./net#http_get) / [`http_post`](./net#http_post) | 曾是占位实现，不发请求 | 已改为真实实现（`ureq` 同步阻塞 + rustls TLS，issue #56）✅ |

仍开放的问题：

| 位置 | 问题 | 追踪 |
| ---- | ---- | ---- |
| [`std.option`](./option#已知缺口) | `Try(Option(T), T, Void)` 实例化未被类型检查器承认，`?` 报 `E1081`；`from_error` 运行时报 `E6006` | 见该页「已知缺口」 |
| [`list.slice`](./list#slice) | 越界下标不做钳制，直接 `E6003` | 见该页条目 |

## 文档维护

本目录是**生成 + 手写**混合结构：

- **生成区**（`<!-- stdlib:KEY start/end -->` 标记之间）：函数一览表与签名块，由
  `StdModule::exports()` 派生。签名逐字节来自 `NativeExport::signature`，不可能与实现漂移。
- **手写区**（标记之外）：模块概述、借用/移动语义、错误模型、已知缺口与示例。

门禁（CI 里随 `cargo test --lib` 运行）：

| 门禁       | 测试                                          | 作用                            |
| ---------- | --------------------------------------------- | ------------------------------- |
| 漂移检测   | `test_stdlib_docs_match_generation`           | 生成区必须与 `exports()` 一致   |
| 孤儿检测   | `test_stdlib_docs_has_no_orphan_module_pages` | 模块页不得多于生成器产出        |
| 覆盖面     | `test_stdlib_docs_covers_interface_modules`   | 文档模块集须覆盖接口视图        |
| 示例可运行 | `test_stdlib_docs_examples_run`               | 每个 ```yaoxiang 示例必须真能跑 |

> 生成器只覆盖 native `StdModule`；纯 yx 模块（`list` / `json` / `option` / `test`）的页
> 不含生成区，其合法性由示例可运行门禁与孤儿检测（页名在
> `src/std/yx_sources.rs` 的白名单内）共同保证。

`exports()` 变更后，用治愈工具重写生成区：

```bash
cargo run --example gen-stdlib-docs
```

与 `gen-std-interfaces`（RFC-037 接口视图）、`tools/code-tables --fix` （RFC-013 码表）同构。

## 相关文档

- [标准库规范](../language-spec/stdlib.md) —— 语言层面的标准库设计约定
- [FFI 规范](../language-spec/ffi.md) —— 用户侧 `native` 扩展与 C ABI 绑定
- [错误码参考](../error-code/) —— `E6xxx` 运行时错误码全表

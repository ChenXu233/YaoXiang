---
title: "标准库状态"
---

# 标准库（Std）

> **模块状态**：有缺口（4 项待改进）
> **位置**：`src/std/`
> **最后更新**：2026-06-01

---

## 模块概述

标准库提供 YaoXiang 语言的核心功能模块。包含 IO、数学、字符串、列表、字典、文件系统、网络、并发等模块。

**代码量**：5,071 行（14 个子模块）

---

## 功能清单

### std.io（379 行）- ✅ 已完成

| 函数 | 签名 | 状态 |
|------|------|------|
| `print` | `(...args) -> ()` | ✅ |
| `println` | `(...args) -> ()` | ✅ |
| `read_line` | `() -> String` | ✅ |
| `read_file` | `(path: String) -> String` | ✅ |
| `write_file` | `(path: String, content: String) -> Bool` | ✅ |
| `append_file` | `(path: String, content: String) -> Bool` | ✅ |
| `format_fallback` | `(value, type_name: String) -> String` | ✅ |

### std.math（301 行）- ✅ 已完成

| 函数 | 签名 | 状态 |
|------|------|------|
| `abs` | `(n: Int) -> Int` | ✅ |
| `max/min` | `(a: Int, b: Int) -> Int` | ✅ |
| `clamp` | `(value: Int, min: Int, max: Int) -> Int` | ✅ |
| `fabs/fmax/fmin` | Float 版本 | ✅ |
| `pow` | `(base: Float, exp: Float) -> Float` | ✅ |
| `sqrt` | `(n: Float) -> Float` | ✅ |
| `floor/ceil/round` | `(n: Float) -> Float` | ✅ |
| `sin/cos/tan` | `(n: Float) -> Float` | ✅ |
| `PI/E/TAU` | 常量 | ✅ |

### std.string（523 行）- ✅ 已完成

| 函数 | 签名 | 状态 |
|------|------|------|
| `split` | `(s: String, sep: String) -> List` | ✅ |
| `trim` | `(s: String) -> String` | ✅ |
| `upper/lower` | `(s: String) -> String` | ✅ |
| `replace` | `(s: String, old: String, new: String) -> String` | ✅ |
| `contains/starts_with/ends_with` | `(s: String, sub: String) -> Bool` | ✅ |
| `index_of` | `(s: String, sub: String) -> Int` | ✅ |
| `substring` | `(s: String, start: Int, end: Int) -> String` | ✅ |
| `is_empty/len` | `(s: String) -> Bool/Int` | ✅ |
| `chars` | `(s: String) -> List` | ✅ |
| `concat/repeat/reverse` | 字符串操作 | ✅ |
| `format` | `(format: String, ...args) -> String` | ✅ |

### std.list（784 行）- ✅ 已完成

| 函数 | 签名 | 状态 |
|------|------|------|
| `push/pop/append/prepend` | 列表修改 | ✅ |
| `remove_at` | `(list: List, index: Int) -> Any` | ✅ |
| `reverse/concat` | 列表操作 | ✅ |
| `map/filter/reduce` | 高阶函数 | ✅ |
| `len/is_empty` | 列表信息 | ✅ |
| `get/set` | 索引访问 | ✅ |
| `first/last` | 边界元素 | ✅ |
| `slice` | `(list: List, start: Int, end: Int) -> List` | ✅ |
| `contains/find_index` | 查找 | ✅ |
| `iter/next/has_next` | 迭代器协议 | ✅ |

### std.dict（335 行）- ✅ 已完成

| 函数 | 签名 | 状态 |
|------|------|------|
| `get/set` | 字典访问 | ✅ |
| `has` | `(dict: Dict, key: Any) -> Bool` | ✅ |
| `keys/values/entries` | 获取集合 | ✅ |
| `delete` | `(dict: Dict, key: Any) -> Dict` | ✅ |
| `len/is_empty` | 字典信息 | ✅ |
| `merge` | `(a: Dict, b: Dict) -> Dict` | ✅ |

### std.convert（149 行）- ✅ 已完成

- ✅ `to_string` — 通用类型转换为字符串
- ✅ 各类型 `to_string` 方法：int, float, bool, char, string, list, dict, tuple, set, range

### std.os（1,023 行）- ✅ 已完成

- ✅ 文件操作：open, close, read, write, seek, tell, flush
- ✅ 目录操作：mkdir, rmdir, read_dir
- ✅ 路径检查：remove, exists, is_file, is_dir
- ✅ 文件操作：copy, rename
- ✅ 环境变量：get_env, set_env
- ✅ 进程信息：args, chdir, getcwd

### std.time（507 行）- ✅ 已完成

- ✅ 时间获取：now, timestamp, timestamp_ms
- ✅ `sleep` — `(seconds: Float) -> Void`
- ✅ 格式化：format_time, parse_time（strftime 风格）
- ✅ DateTime 方法：year, month, day, hour, minute, second, weekday, to_string

### std.net（177 行）- ⚠️ 桩实现

| 函数 | 签名 | 状态 |
|------|------|------|
| `http_get` | `(url: String) -> String` | ⚠️ 桩 - 返回 `"GET: {url}"` |
| `http_post` | `(url: String, body: String) -> String` | ⚠️ 桩 - 返回 `"POST {url}: {body}"` |
| `url_encode` | `(s: String) -> String` | ✅ |
| `url_decode` | `(s: String) -> String` | ✅ |

### std.concurrent（85 行）- ✅ 基本完成

- ✅ `sleep` — `(millis: Int) -> Void`
- ✅ `thread_id` — `() -> String`
- ✅ `yield_now` — `() -> Void`

### std.ffi（265 行）- ✅ 已完成

- ✅ `native` — `(symbol: String) -> Never`（编译时拦截）

### std.weak（45 行）- ⚠️ 基础实现

- ✅ `weak_new` — `(arc) -> Weak`
- ✅ `weak_upgrade` — `(weak) -> Option`
- ⚠️ 缺少 `StdModule` trait 实现，无法通过 `use std.weak` 导入

### gen_interfaces（208 行）- ✅ 已完成

- ✅ 自动生成 `.yx` 接口文件
- ✅ 支持写入目录、查找接口文件

---

## 测试覆盖

**仅 8 个单元测试**，严重不足：

| 模块 | 单元测试数 | 状态 |
|------|-----------|------|
| io | 0 | ❌ 缺失 |
| math | 0 | ❌ 缺失 |
| string | 0 | ❌ 缺失 |
| list | 0 | ❌ 缺失 |
| dict | 0 | ❌ 缺失 |
| convert | 0 | ❌ 缺失 |
| os | 0 | ❌ 缺失 |
| time | 0 | ❌ 缺失 |
| net | 0 | ❌ 缺失 |
| concurrent | 0 | ❌ 缺失 |
| ffi | 2 | ✅ 基础覆盖 |
| gen_interfaces | 6 | ✅ 较好 |

**间接测试覆盖**：
- `tests/yx_runner.rs` 通过 E2E 测试覆盖部分功能
- `tests/integration/execution.rs` 有基础集成测试

---

## 发现的问题

1. **net 模块为桩实现**：`http_get` 和 `http_post` 返回模拟字符串
2. **weak 模块不完整**：缺少 `StdModule` trait 实现，无法通过 `use std.weak` 导入
3. **os.chdir 未实际切换目录**：只检查目录是否存在，未调用 `std::env::set_current_dir()`
4. **string.len 返回字节数**：`native_len` 使用 `s.len()` 返回字节数而非字符数

---

## 代码质量评估

| 维度 | 评分 | 说明 |
|------|------|------|
| 未完成事项 | 4 | 补充测试、修复 bug、weak 模块、HTTP 桩 |
| 测试覆盖 | 严重不足 | 仅 8 个单元测试 |
| 文档质量 | 良好 | 每个模块有模块级 `//!` 文档注释 |
| 代码架构 | 良好 | 模块划分清晰 |

---

## 待改进项

1. **为各模块添加单元测试**（最高优先级）
2. **修复 `os.chdir` 和 `string.len` 的问题**
3. **完善 `weak` 模块的 `StdModule` 实现**
4. **实现真实的 HTTP 功能或明确标记为桩**

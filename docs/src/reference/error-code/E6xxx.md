---
title: 'E6xxx：运行时错误'
description: '程序运行期（VM / std 原生函数）产生的错误。'
---

# E6xxx：运行时错误

 本页由 `scripts/docs/gen-error-code-docs.py` 从 `src/util/diagnostic/codes/` 的 `define_codes!` 注册表与 `locales/zh.json` 生成，请勿手工编辑。改文案请改 `locales/*.json`，改码表请改 `define_codes!`，然后重跑 `python scripts/docs/gen-error-code-docs.py`。

程序运行期（VM / std 原生函数）产生的错误。

本族共 **7** 个码，全部在 `define_codes!` 注册表中，类别为 `Runtime`。完整索引见[错误码首页](index.md)。

## 码一览

| 码号 | 中文名 | 类别 | span 豁免 | 模板 | 发射点 |
| --- | --- | --- | --- | --- | --- |
| `E6001` | 除零错误 | `Runtime` | 是 | `表达式 {expr} 除以零` | ✅ 8 处 |
| `E6003` | 数组索引越界 | `Runtime` | 是 | `数组索引越界：有效范围是 0..{max}，找到 {index}` | ✅ 3 处 |
| `E6004` | 栈溢出 | `Runtime` | 是 | `栈溢出：递归深度超出限制 {limit}` | ✅ 3 处 |
| `E6005` | 断言失败 | `Runtime` | 是 | `断言失败：{condition}` | ✅ 3 处 |
| `E6006` | 函数未找到（运行时） | `Runtime` | 是 | `函数未找到：'{func}'` | ✅ 1 处 |
| `E6007` | 运行时错误 | `Runtime` | 是 | `运行时错误：{message}` | ✅ 5 处 |
| `E6008` | 键不存在 | `Runtime` | 是 | `键不存在：{key}` | ✅ 2 处 |

## 逐码说明

### E6001：除零错误

- **类别**：`Runtime`
- **span 豁免**：是
- **构造函数**：`ErrorCodeDefinition::division_by_zero(expr)`
- **模板**：`表达式 {expr} 除以零`
- **消息**：试图除以零
- **帮助**：添加检查以防止除以零
- **发射点**：`src/backends/interpreter/executor/executor.rs:825`、`src/backends/interpreter/executor/executor.rs:834`、`src/backends/mod.rs:163` 等 8 处

### E6003：数组索引越界

- **类别**：`Runtime`
- **span 豁免**：是
- **构造函数**：`ErrorCodeDefinition::runtime_index_out_of_bounds(max, index)`
- **模板**：`数组索引越界：有效范围是 0..{max}，找到 {index}`
- **消息**：数组索引在运行时越界
- **帮助**：确保索引在数组范围内
- **发射点**：`src/util/diagnostic/mod.rs:212`、`src/util/test_markers.rs:251`、`src/util/test_markers.rs:341`
- **源码注释名**：数组索引越界（运行时）

### E6004：栈溢出

- **类别**：`Runtime`
- **span 豁免**：是
- **构造函数**：`ErrorCodeDefinition::stack_overflow(limit)`
- **模板**：`栈溢出：递归深度超出限制 {limit}`
- **消息**：递归深度超出栈限制
- **帮助**：减少递归深度或使用迭代
- **发射点**：`src/backends/mod.rs:158`、`src/backends/interpreter/executor/executor.rs:320`、`src/util/diagnostic/mod.rs:204`

### E6005：断言失败

- **类别**：`Runtime`
- **span 豁免**：是
- **构造函数**：`ErrorCodeDefinition::assertion_failed(condition)`
- **模板**：`断言失败：{condition}`
- **消息**：运行时断言失败
- **帮助**：修复断言条件或提供有效输入
- **发射点**：`src/backends/mod.rs:185`、`src/std/assert.rs:94`、`src/util/diagnostic/mod.rs:230`

### E6006：函数未找到（运行时）

- **类别**：`Runtime`
- **span 豁免**：是
- **构造函数**：`ErrorCodeDefinition::runtime_function_not_found(func)`
- **模板**：`函数未找到：'{func}'`
- **帮助**：确保函数已定义且拼写正确
- **发射点**：`src/util/diagnostic/mod.rs:191`

### E6007：运行时错误

- **类别**：`Runtime`
- **span 豁免**：是
- **构造函数**：`ErrorCodeDefinition::runtime_error(message)`
- **模板**：`运行时错误：{message}`
- **帮助**：查看错误消息了解详情
- **发射点**：`src/util/diagnostic/mod.rs:91`、`src/util/diagnostic/mod.rs:202`、`src/util/diagnostic/mod.rs:203` 等 5 处
- **源码注释名**：运行时错误（通用）

### E6008：键不存在

- **类别**：`Runtime`
- **span 豁免**：是
- **构造函数**：`ErrorCodeDefinition::key_not_found(key)`
- **模板**：`键不存在：{key}`
- **消息**：字典在运行时找不到该键
- **帮助**：索引前先用 dict.has 检查键是否存在
- **发射点**：`src/backends/mod.rs:141`、`src/util/diagnostic/mod.rs:228`
- **源码注释名**：键缺失（#299 §4）

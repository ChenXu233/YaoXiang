---
title: 'E8xxx：内部编译器错误'
description: '编译器内部错误，通常表示编译器自身的缺陷。遇到此类错误请提交 issue。'
---

# E8xxx：内部编译器错误

 本页由 `scripts/docs/gen-error-code-docs.py` 从 `src/util/diagnostic/codes/` 的 `define_codes!` 注册表与 `locales/zh.json` 生成，请勿手工编辑。改文案请改 `locales/*.json`，改码表请改 `define_codes!`，然后重跑 `python scripts/docs/gen-error-code-docs.py`。

编译器内部错误，通常表示编译器自身的缺陷。遇到此类错误请提交 issue。

本族共 **3** 个码，全部在 `define_codes!` 注册表中，类别为 `Internal`。完整索引见[错误码首页](index.md)。

## 码一览

| 码号 | 中文名 | 类别 | span 豁免 | 模板 | 发射点 |
| --- | --- | --- | --- | --- | --- |
| `E8001` | 内部编译器错误 | `Internal` | 是 | `内部编译器错误：{message}` | ✅ 27 处 |
| `E8002` | 意外 Panic | `Internal` | 否 | `意外编译器 panic：{reason}` | ⚠ 暂未发射 |
| `E8003` | 编译器阶段错误 | `Internal` | 否 | `编译器阶段错误：{phase} - {message}` | ⚠ 暂未发射 |

## 逐码说明

### E8001：内部编译器错误

- **类别**：`Internal`
- **span 豁免**：是
- **构造函数**：`ErrorCodeDefinition::internal_error(message)`
- **模板**：`内部编译器错误：{message}`
- **消息**：编译器发生内部错误
- **帮助**：请在 https://github.com/ChenXu233/YaoXiang/issues/new 报告此错误
- **发射点**：`src/frontend/pipeline.rs:427`、`src/frontend/core/typecheck/proof/verdict.rs:289` 等 26 处
- **历史**：原列出的 `src/frontend/core/typecheck/layers/dispatch.rs:151` 随该模块在 #377-2 删除而消失（原计数 27 处，现为 26 处）

### E8002：意外 Panic

- **类别**：`Internal`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::unexpected_panic(reason)`
- **模板**：`意外编译器 panic：{reason}`
- **消息**：编译器遇到意外 panic
- **帮助**：请使用 panic 详情报告此错误
- **发射点**：⚠ 暂未发射（注册表中保留该码，但非测试代码里没有调用处，用户无法触发）
- **源码注释名**：意外 panic

### E8003：编译器阶段错误

- **类别**：`Internal`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::compiler_phase_error(phase, message)`
- **模板**：`编译器阶段错误：{phase} - {message}`
- **消息**：编译器阶段发生错误
- **帮助**：这可能是编译器错误，请报告
- **发射点**：⚠ 暂未发射（注册表中保留该码，但非测试代码里没有调用处，用户无法触发）

---
title: 'E7xxx：I/O 与系统错误'
description: 'I/O 操作与系统调用失败产生的错误。'
---

# E7xxx：I/O 与系统错误

 本页由 `scripts/docs/gen-error-code-docs.py` 从 `src/util/diagnostic/codes/` 的 `define_codes!` 注册表与 `locales/zh.json` 生成，请勿手工编辑。改文案请改 `locales/*.json`，改码表请改 `define_codes!`，然后重跑 `python scripts/docs/gen-error-code-docs.py`。

I/O 操作与系统调用失败产生的错误。

本族共 **4** 个码，全部在 `define_codes!` 注册表中，类别为 `Io`。完整索引见[错误码首页](index.md)。

## 码一览

| 码号 | 中文名 | 类别 | span 豁免 | 模板 | 发射点 |
| --- | --- | --- | --- | --- | --- |
| `E7001` | 文件未找到 | `Io` | 否 | `文件未找到：'{path}'` | ⚠ 暂未发射 |
| `E7002` | 权限被拒绝 | `Io` | 否 | `权限被拒绝：'{path}'` | ⚠ 暂未发射 |
| `E7003` | I/O 错误 | `Io` | 否 | `I/O 错误：{reason}` | ⚠ 暂未发射 |
| `E7004` | 网络错误 | `Io` | 否 | `网络错误：{reason}` | ⚠ 暂未发射 |

## 逐码说明

### E7001：文件未找到

- **类别**：`Io`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::file_not_found(path)`
- **模板**：`文件未找到：'{path}'`
- **消息**：指定文件不存在
- **帮助**：检查文件路径，确保文件存在
- **发射点**：⚠ 暂未发射（注册表中保留该码，但非测试代码里没有调用处，用户无法触发）

### E7002：权限被拒绝

- **类别**：`Io`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::permission_denied(path)`
- **模板**：`权限被拒绝：'{path}'`
- **消息**：访问文件的权限不足
- **帮助**：检查文件权限或使用适当权限运行
- **发射点**：⚠ 暂未发射（注册表中保留该码，但非测试代码里没有调用处，用户无法触发）

### E7003：I/O 错误

- **类别**：`Io`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::io_error(reason)`
- **模板**：`I/O 错误：{reason}`
- **消息**：发生未指定的 I/O 错误
- **帮助**：检查错误详情获取更多信息
- **发射点**：⚠ 暂未发射（注册表中保留该码，但非测试代码里没有调用处，用户无法触发）

### E7004：网络错误

- **类别**：`Io`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::network_error(reason)`
- **模板**：`网络错误：{reason}`
- **消息**：发生网络错误
- **帮助**：检查网络连接并重试
- **发射点**：⚠ 暂未发射（注册表中保留该码，但非测试代码里没有调用处，用户无法触发）

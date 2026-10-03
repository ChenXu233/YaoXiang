---
title: 'E5xxx：模块与导入错误'
description: '模块系统与导入解析相关的错误。'
---

# E5xxx：模块与导入错误

 本页由 `scripts/docs/gen-error-code-docs.py` 从 `src/util/diagnostic/codes/` 的 `define_codes!` 注册表与 `locales/zh.json` 生成，请勿手工编辑。改文案请改 `locales/*.json`，改码表请改 `define_codes!`，然后重跑 `python scripts/docs/gen-error-code-docs.py`。

模块系统与导入解析相关的错误。

本族共 **7** 个码，全部在 `define_codes!` 注册表中，类别为 `Module`。完整索引见[错误码首页](./index.md)。

## 码一览

| 码号 | 中文名 | 类别 | span 豁免 | 模板 | 发射点 |
| --- | --- | --- | --- | --- | --- |
| `E5001` | 模块未找到 | `Module` | 否 | `模块 '{module}' 未找到` | ✅ 1 处 |
| `E5002` | 导入错误 | `Module` | 否 | `导入模块 '{module}' 失败：{reason}` | ⚠ 暂未发射 |
| `E5003` | 导出未找到 | `Module` | 否 | `在模块 '{module}' 中找不到导出 '{export}'` | ✅ 1 处 |
| `E5004` | 循环依赖 | `Module` | 否 | `检测到循环依赖：{path}` | ⚠ 暂未发射 |
| `E5005` | 无效的模块路径 | `Module` | 否 | `无效的模块路径：'{path}'` | ⚠ 暂未发射 |
| `E5006` | 重复导入 | `Module` | 否 | `重复导入：'{name}' 已被导入` | `statements.rs` `ensure_import_name_free` |
| `E5007` | 模块导出 | `Module` | 否 | `模块 '{module}' 的导出：{available}` | ⚠ 暂未发射 |
| `E5008` | 导入名冲突 | `Module` | 否 | `导入名 '{name}' 与现有绑定冲突（导入自 '{module}'）` | `statements.rs` `ensure_import_name_free` |

## 逐码说明

### E5001：模块未找到

- **类别**：`Module`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::module_not_found(module)`
- **模板**：`模块 '{module}' 未找到`
- **消息**：引用的模块不存在
- **帮助**：检查模块路径，确保它存在
- **发射点**：`src/frontend/core/typecheck/inference/statements.rs:554`

### E5002：导入错误

- **类别**：`Module`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::import_error(module, reason)`
- **模板**：`导入模块 '{module}' 失败：{reason}`
- **消息**：导入模块失败
- **帮助**：检查导入路径和模块解析
- **发射点**：⚠ 暂未发射（注册表中保留该码，但非测试代码里没有调用处，用户无法触发）

### E5003：导出未找到

- **类别**：`Module`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::export_not_found(export, module)`
- **模板**：`在模块 '{module}' 中找不到导出 '{export}'`
- **消息**：模块中不存在的导出
- **帮助**：检查模块的导出
- **发射点**：`src/frontend/core/typecheck/inference/statements.rs:613`

### E5004：循环依赖

- **类别**：`Module`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::circular_dependency(path)`
- **模板**：`检测到循环依赖：{path}`
- **消息**：模块存在循环依赖
- **帮助**：重构以移除循环依赖
- **发射点**：⚠ 暂未发射（注册表中保留该码，但非测试代码里没有调用处，用户无法触发）

### E5005：无效的模块路径

- **类别**：`Module`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::invalid_module_path(path)`
- **模板**：`无效的模块路径：'{path}'`
- **帮助**：检查模块路径格式
- **发射点**：⚠ 暂未发射（注册表中保留该码，但非测试代码里没有调用处，用户无法触发）

### E5006：重复导入

- **类别**：`Module`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::duplicate_import(name)`
- **模板**：`重复导入：'{name}' 已被导入`
- **帮助**：请移除重复的导入，或使用不同名字/模块别名
- **发射点**：`src/frontend/core/typecheck/inference/statements.rs:284`（`ensure_import_name_free`：同名本地名导两次——跨模块同名别名、同语句重复别名 `as m, m`、同名字项两次导入）

### E5007：模块导出

- **类别**：`Module`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::module_exports_hint(module, available)`
- **模板**：`模块 '{module}' 的导出：{available}`
- **帮助**：可用的导出列在错误消息中
- **发射点**：⚠ 暂未发射（注册表中保留该码，但非测试代码里没有调用处，用户无法触发）
- **源码注释名**：模块导出提示（用于辅助错误消息）

### E5008：导入名冲突

- **类别**：`Module`
- **span 豁免**：否
- **构造函数**：`ErrorCodeDefinition::import_name_conflict(name, module)`
- **模板**：`导入名 '{name}' 与现有绑定冲突（导入自 '{module}'）`
- **帮助**：请使用不同名字或模块别名
- **发射点**：`src/frontend/core/typecheck/inference/statements.rs:293`（`ensure_import_name_free`：导入名撞本文件已有的顶层绑定。RFC-029 §导入冲突「同名绑定直接报错」；导入在前、定义在后的同型冲突由 E2002 重复定义承接）

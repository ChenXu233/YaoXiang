---
title: 跨文件分析
description: YaoXiang check 跨文件类型检查的设计
---

# 跨文件分析

## 问题描述

早期实现中，`check_files_with_diagnostics` 为每个文件创建独立的 `Compiler`，无法检测跨文件引用。fileA 定义的 `pub` 函数在 fileB 中无法被识别。

## 解决方案

使用共享 `TypeEnvironment`，按依赖顺序检查所有模块。

## 实现流程

```text
1. 并行解析所有 .yx 文件 → Vec<(PathBuf, ModuleId, AST)>
2. 用 ModuleDependencyGraph::build_from_ast 构建依赖图
3. detect_cycles() 检查循环依赖 → 报错
4. topological_sort() 得到编译顺序
5. 按序类型检查：
   a. 创建共享 TypeEnvironment（含 std 模块）
   b. 对每个模块：注册其导出到共享环境 → 类型检查
   c. 收集诊断信息
6. 返回 CheckResult
```

## 命名空间隔离

使用 `module_name.symbol_name` 格式存储导出符号，避免不同模块的同名符号冲突。

## 已知限制

- `traits/` 占位实现（coherence/impl_check/object_safety/resolution）未完成
- `check_single_module` 仍为每个模块创建独立 Compiler（共享 env 的类型信息传递尚未完全实现）

## 未来工作

- T8：跨文件类型检查端到端测试
- A4：共享 trait_table 和 native_signatures

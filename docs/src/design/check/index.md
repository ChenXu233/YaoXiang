---
title: check 命令设计文档
description: yaoxiang check 静态检查工具的设计规范
---

# check 命令设计文档

`yaoxiang check` 是 YaoXiang 编译器的静态检查工具，提供类型检查、跨文件分析和增量检查功能。

## 设计原则

1. **零误报**：报告的每个错误都必须是真实的错误
2. **跨文件感知**：正确检测跨模块的类型错误和未定义引用
3. **增量优先**：watch 模式只重检查受影响的文件
4. **自文档化**：错误码、消息模板、帮助文本均通过 i18n 管理

## 文档导航

- [诊断系统](./diagnostic-system.md) — 错误码体系、Diagnostic 数据结构、Emitter 输出
- [跨文件分析](./cross-file-analysis.md) — 共享类型环境、依赖图、拓扑排序
- [增量检查](./incremental-checking.md) — CheckSession、affected_modules、watch 模式

## 与其他系统的边界

| 系统 | 职责 | 与 check 的关系 |
|------|------|-----------------|
| 编译器 (`yaoxiang build`) | 完整编译（解析 → 类型检查 → 代码生成） | check 只做前两步 |
| LSP | 编辑器集成（补全、跳转、诊断） | check 的诊断可复用 |
| 格式化 (`yaoxiang fmt`) | 代码风格 | 独立，CI 中并行使用 |

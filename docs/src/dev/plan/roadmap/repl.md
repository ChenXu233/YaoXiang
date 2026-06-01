---
title: "REPL 状态"
---

# REPL

> **模块状态**：已完成（v0.7.2 重写）
> **位置**：`src/backends/dev/repl/`
> **最后更新**：2026-06-01

---

## 模块概述

REPL（Read-Eval-Print Loop）模块提供交互式编程环境。采用 trait 抽象架构，支持不同后端实现。

**代码量**：1,037 行（8 个文件）

---

## 功能清单

### REPLBackend trait（backend_trait.rs）

- ✅ `eval()` 求值
- ✅ `complete()` 补全候选
- ✅ `get_symbols()` 符号列表
- ✅ `get_type()` 类型查询
- ✅ `clear()` 清除状态
- ✅ `stats()` 执行统计

### 求值引擎（engine/evaluator.rs - 299 行）

- ✅ 代码编译执行
- ✅ 括号/引号完整性检测
- ✅ 表达式/语句自动包装
- ✅ 从字节码提取定义

### 执行上下文（engine/context.rs - 168 行）

- ✅ 变量定义/查询
- ✅ 函数定义/查询
- ✅ 符号类型查询
- ✅ 执行统计

### 命令系统（commands/mod.rs - 95 行）

- ✅ `:quit/:q` 退出
- ✅ `:help/:h` 帮助
- ✅ `:clear/:c` 清除
- ✅ `:type/:t` 类型查看
- ✅ `:symbols/:info` 符号列表
- ✅ `:stats` 统计
- ⚠️ `:history` 命令 — **未实现**（仅打印提示）

### 会话 REPL（session/mod.rs - 247 行）

- ✅ rustyline 集成
- ✅ 多行输入支持
- ✅ 历史记录保存/加载
- ✅ VI/Emacs 编辑模式
- ✅ 文件加载执行
- ✅ 自定义配置

### 自动补全（session/completer.rs - 126 行）

- ✅ 关键字补全
- ✅ 变量/函数补全
- ✅ 内置函数补全

---

## 测试覆盖

**0 个单元测试**

REPL 模块没有任何测试代码。整个 `src/backends/dev/repl/` 目录下没有任何 `#[test]` 或 `#[cfg(test)]` 标注。

---

## 代码质量评估

| 维度 | 评分 | 说明 |
|------|------|------|
| 功能完成度 | 90% | 核心功能完整，仅 :history 未实现 |
| 测试覆盖 | 0% | 无任何测试 |
| 文档质量 | 良好 | 有完整用户指南（`docs/src/guide/repl.md`，436 行）和代码注释 |
| 架构设计 | 优秀 | trait 抽象、分层清晰、可扩展 |

---

## 集成状态

REPL 已集成到以下组件：

1. **DevShell**（`src/backends/dev/shell.rs`）：通过 `:repl` 命令切换到 REPL 模式
2. **模块导出**（`src/backends/dev/mod.rs`）：导出 `SessionREPL`, `Evaluator`, `REPLBackend`
3. **CLI 入口**：通过 `yaoxiang repl` 或 `yaoxiang` 启动

---

## 待改进项

1. **补充单元测试**（零测试覆盖是最大问题）
2. **实现 `:history` 命令**
3. **补充边界条件测试**

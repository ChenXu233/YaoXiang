---
title: "发版模板"
---

# 发版模板

> `/release` 命令按此模板生成 changelog。  
> changelog 是**给人读的变更说明**，不是 commit 列表。

## 格式规范

```
:bookmark: V<版本号>: <标题>
```

## 📦 版本信息

| 项目     | 值                      |
| -------- | ----------------------- |
| 发布日期 | YYYY-MM-DD              |
| 版本变更 | `<旧版本>` → `<新版本>` |
| 提交数   | N 个 commit             |

## 📋 本次更新概要

用 2-3 句话总结本次发版的核心内容和意义。

## ✨ 新功能

### <功能领域标题>

<一段话说明这个功能做了什么、解决了什么问题、对用户意味着什么>

- 具体改动 1
- 具体改动 2
- 具体改动 3

### <另一个功能领域>

<说明>

- 具体改动

## 🐛 Bug 修复

### <修复领域>

<说明修复了什么问题、影响范围>

- 具体修复 1
- 具体修复 2

## ♻️ 重构优化

### <重构方向>

<说明为什么重构、重构后有什么好处>

- 具体改动

## 📝 提交记录

|   Hash    | 描述              |
| :-------: | ----------------- |
| `abc1234` | feat(scope): 描述 |
| `def5678` | fix(scope): 描述  |

## 标题规则

一句话概括核心变更，不超过 50 字符：

```
:bookmark: V0.7.2: REPL 重写与类型系统改进
:bookmark: V0.7.3: 类型系统修复与所有权模型改进
:bookmark: V0.8.0: 并发模型与泛型系统
```

## 分类规则

按 `type` 前缀归类，空分类省略：
| type | 归类 | 前缀 |
|:---:|:---:|:---:|
| `feat` | ✨ 新功能 | `:sparkles:` |
| `fix` | 🐛 Bug 修复 | `:bug:` |
| `refactor` | ♻️ 重构优化 | `:recycle:` |
| `perf` | ⚡ 性能优化 | `:zap:` |
| `docs` | 📝 文档 | `:memo:` |
| `style` | 🎨 格式 | `:art:` |
| `test` | ✅ 测试 | `:white_check_mark:` |
| `chore` | 🔧 构建/工具 | `:wrench:` |
| `ci` | 💚 CI/CD | `:green_heart:` |

## 获取提交记录

```bash
git log <最新tag>..HEAD --oneline --no-merges
```

## 完整示例

`:bookmark: V0.7.3: 类型系统修复与所有权模型改进`

```markdown
## 📦 版本信息

| 项目     | 值                |
| -------- | ----------------- |
| 发布日期 | 2026-06-07        |
| 版本变更 | `0.7.2` → `0.7.3` |
| 提交数   | 22 个 commit      |

## 📋 本次更新概要

本次发版聚焦类型系统和所有权模型的稳定性修复。泛型类型实例化（如
`List(Int) = List(1, 2, 3)`）现在可以正确工作，`&T` 字段访问和元组
解构赋值的多个边界情况已修复。同时完成了所有权检查的统一重构，为后
续 move 语义的完善打下基础。

## ✨ 新功能

### 泛型类型实例化

支持 `List(Int) = List(1, 2, 3)` 语法，类型构造器正确注册为 Struct。
泛型函数多次调用时类型推断不再冲突，每次调用获得独立的类型变量实例。

- 类型构造器注册为 Struct，修复实例化根因
- 泛型函数多次调用独立推断
- 接口方法绑定修复

### Move 后重新绑定

变量被 move 后可以重新绑定新值，赋值优先查找现在正确处理 moved 状态，
避免误报"变量已移动"错误。

- VarInfo 增加 moved 状态标记
- 赋值优先查找增加 moved 分支

### Resource marker trait

新增 `Resource` marker trait，标记实现了 IO 副作用的类型。为后续
副作用追踪和并发安全分析提供基础。

### PLDI SRC demo

完成 PLDI SRC demo MVP 全部任务，包含端到端的类型检查和代码生成演示。

## 🐛 Bug 修复

### &T 字段访问

修复不可变引用类型的字段访问在类型检查中的多个问题，包括字段赋值和
通过引用调用构造函数的推断。

- `&T` 字段访问类型检查修复
- 字段赋值目标类型推断
- 构造函数通过引用调用的推断

### LSP 语义高亮

修复 `spawn {}` 块内变量和元组解构赋值 `(a, b) = ...` 的语义高亮
缺失。AST 中 `DestructureAssign.names` 现在携带每个变量名的位置信息。

- spawn 块内变量正确染色
- 元组解构赋值的变量名正确染色
- 为后续 move 语义的 LSP 支持打下基础

### freeze 移除

移除已废弃的 `freeze` 函数，清理相关测试用例。

## ♻️ 重构优化

### 统一诊断系统

将剩余 5 个错误枚举迁移至统一的 `ErrorCodeDefinition` 诊断系统，
错误码覆盖率从 60% 提升至 80%。所有编译错误现在遵循统一的格式和
错误码规范。

### 所有权检查统一

将分散在多处的所有权检查逻辑统一为 `OwnershipPass`，单一入口
处理 borrow、move、生命周期检查。降低维护成本，提高错误一致性。

### 并发模型简化

移除 Send/Sync 约束系统，`spawn {}` 重构为分组执行模型。移除
`@block`/`@eager`/`@auto` 装饰器相关代码（RFC-024 phase 1）。
简化并发模型，为后续可扩展并发原语设计铺路。

## 📝 提交记录

|   Hash    | 描述                                                        |
| :-------: | ----------------------------------------------------------- |
| `7297c65` | feat: 泛型函数多次调用 + yx_runner 错误测试支持             |
| `57a9893` | feat: 泛型类型实例化 — List(Int) = List(1, 2, 3)            |
| `ab8a133` | feat: 根因修复 — 类型构造器注册为 Struct                    |
| `196daec` | feat: 修复泛型实例化 + 接口方法绑定                         |
| `148e7a2` | feat: 完成 PLDI SRC demo MVP 全部任务                       |
| `9a5a1b3` | fix(lsp): 修复 spawn 块和元组解构的语义高亮                 |
| `75489c4` | feat: 修复 &T 字段访问 + 构造函数推断 + 移除 freeze         |
| `a5b6135` | feat: 修复 &T 字段访问 + 字段赋值 + 元组解构                |
| `0e24fcd` | refactor(diagnostic): 迁移剩余 5 个错误枚举至统一诊断系统   |
| `c262ddc` | refactor(diagnostic): 迁移错误码至统一诊断系统              |
| `e8869c2` | feat(middle): 添加 Resource marker trait 支持 IO 副作用感知 |
| `a76cdd3` | refactor(lifetime): 将所有权检查统一为 OwnershipPass        |
| `c7af770` | feat(typecheck): 赋值优先查找增加 moved 分支                |
| `bb83e12` | feat(typecheck): VarInfo 增加 moved 状态标记                |
| `e4a44c4` | refactor(middle): 移除 Send/Sync 约束系统                   |
| `1d3fe2d` | refactor(frontend): 移除 @block/@eager/@auto 相关代码       |
| `4fd4e0a` | feat(formatter): 实现缺失的格式化规则                       |
| `13fbc21` | fix(backends): 修复 execute.rs 中的语法错误                 |
```

## 流程概览

```
收集提交 → 生成 changelog → 创建 PR → 等待 CI 全绿 → bump 版本 → 合并
```

详见 `.claude/commands/release.md`

---
title: '开发文档'
description: '贡献者与维护者入口：改代码、改文档、架构文档、工具设计规范、流程规范'
---

# 开发文档

面向贡献者与维护者。所有改动从同一本手册进入，按改动类型各守一本规则。

## 动手之前（必读）

| 你要做什么 | 入口（统一自检）           | 规则本体                             |
| ---------- | -------------------------- | ------------------------------------ |
| 改代码     | [HOWTO.md](./HOWTO.md) §一 | [coding-rules.md](./coding-rules.md) |
| 改文档     | [HOWTO.md](./HOWTO.md) §二 | [docs-rules.md](./docs-rules.md)     |

拿不准自己算哪种？先读 HOWTO 第 0 步，它会替你判定。

## 编译器架构

[architecture/](./architecture/)
收录 RFC-039 重构的附属设计文档：01 功能路由、02 阶段契约、03 类型表示、04
SSA、05 前端范式、06 清理清单、07 等价性判据、08 维护机制、09 施工任务表（WBS）。

## 工具设计规范

- [design/check/](./design/check/)：yx check 静态检查的设计规范（零误报原则、跨文件分析、增量检查）
- [design/formatter/](./design/formatter/)：yx format 格式化工具的行为规范

> 用户侧的命令用法在参考目录：[check](../reference/check-command.md)、[format](../reference/format-command.md)、[test](../reference/test-command.md)。

## 流程与规范

- [贡献指南](./contributing.md)：如何参与开发
- [提交规范](./commit-convention.md)：Git 提交信息格式
- [分支维护指南](./branch-maintenance-guide.md)：分支管理策略
- [发布流程](./release.md)：版本发布与产物分发
- [测试规范](./test-specification.md)：测试分层、语料组织与门禁

## 这里不收什么

- 语言特性的提案与定案 → [rfc/](../rfc/)
- 语言理念与宣言 → [explanation/](../explanation/)
- 命令的用户文档 → [reference/](../reference/)

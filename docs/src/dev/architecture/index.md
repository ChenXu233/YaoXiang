# 编译器架构重构 · 附属设计文档

> 本目录是 [RFC-039 编译器架构重构](../../rfc/draft/039-compiler-architecture.md) 的附属设计文档集合，与 `../check/`、`../formatter/` 同级。
>
> **审阅入口是 RFC-039 本身。** 本目录是立项后的施工图——每篇文档对应 RFC 正文中的一个层次或一个横切关注点。

## 阅读顺序

| 顺序 | 文档 | 读它来解决什么问题 |
| --- | --- | --- |
| 1 | [RFC-039 正文](../../rfc/draft/039-compiler-architecture.md) | 为什么重构、四层模型、判据分级、P0–P10 执行顺序 |
| 2 | [HOWTO.md](../HOWTO.md) | **实现者手册**：动工前自检表 + "补丁式修复"判定（D3）。改代码前第一个打开的文档 |
| 3 | [coding-rules.md](../coding-rules.md) + [08](08-maintenance-mechanism.md) | 规矩本体（coding-rules，长期有效）；08 为 2026-10 诊断记录 |
| 4 | [09-execution-wbs.md](09-execution-wbs.md) | **施工清单**：11 个一级 / 45 个二级 / 122 个三级任务、依赖、冲突登记 |
| 5 | [01-routing.md](01-routing.md) | **加一个特性该改哪里**、施工完成后的目标目录结构（长期参考） |
| 6 | [07-equivalence-oracle.md](07-equivalence-oracle.md) | 每一步靠什么证明没改坏 |
| 7 | [02 ~ 05](02-stage-contract.md) | 各层次的具体设计 |

## 文档清单

| 文档 | 层次 | 主题 | 规模 |
| --- | --- | --- | --- |
| [01-routing.md](01-routing.md) | 跨层 | 功能路由表 A/B/C、依赖方向规范、**施工完成后的目标目录结构**、现状→目标映射、命名约定、未来扩展指引 | 315 行 |
| [02-stage-contract.md](02-stage-contract.md) | L1 编排 | 编译阶段契约与义务账本：11 处阶段不一致、8 步漏洞证据链、16 个义务字段、`Stage` 枚举、`Obligations` 账本、统一 `Driver`、`ProgramKind`（含 `WasmPlayground`） | 746 行 |
| [03-type-unification.md](03-type-unification.md) | L2/L3 根因 | 类型表示单一化：26 个 `ast::Type` 变体逐个处置、13 个前向零构造变体、`NameKind` 替代手写同义词表、类型表生成期门禁 T1-T4 | 785 行 |
| [04-ssa.md](04-ssa.md) | L3 IR | 中间表示 SSA 化：四类缺陷、SSA 形态定义、分配器评估、38 项改动清单 | 751 行 |
| [05-frontend-paradigm.md](05-frontend-paradigm.md) | L2 前端 | 词法收敛为唯一实现 / 语法 LALRPOP 文法驱动、改运算符改动面收敛、死阶梯处理、测试重建 | 792 行 |
| [06-cleanup-inventory.md](06-cleanup-inventory.md) | 全域 | 死代码与空头设计清理：可达性分析方法、23 文件 1081 行孤儿、1500+ 行零调用死代码、空头设计逐项处置 | 260 行 |
| [07-equivalence-oracle.md](07-equivalence-oracle.md) | 跨层 | 等价性判据：C1-C6 分级、三层判据（IR 校验器 / 规范化快照 / 语料差分）、性能基线、回归门禁 | 222 行 |
| [08-maintenance-mechanism.md](08-maintenance-mechanism.md) | 跨层 | **仓库维护机制**：三条禁令、D0–D4 决策程序、可机检规则清单、代码审查清单、外部惯例参照（含不可照搬清单） | 241 行 |
| [09-execution-wbs.md](09-execution-wbs.md) | 跨分册 | **多级施工任务表**：三级 WBS、强制串行主链、7 个可并行分组、冲突登记（已收敛至 RFC-039 决议） | 332 行 |

## 交叉引用约定

- 引用本文档集内部：`[05-frontend-paradigm.md](05-frontend-paradigm.md)`
- 引用 RFC 正文：`[RFC-039](../../rfc/draft/039-compiler-architecture.md)`
- 引用其他 RFC：`[RFC-013](../../rfc/accepted/013-error-code-specification.md)`

## 关于编号

这些文档**不是 RFC**，没有 RFC 编号，也不参与 `scripts/rfc/check_tracking.py` 的状态追踪——它们是 RFC-039 的组成部分，与 RFC 同生共死。RFC-039 状态变为 `rejected` 或 `deprecated` 时，本目录应一并归档。

`01-routing.md` 是唯一例外：它记录的是**当前代码的事实**（哪些反向依赖存在、哪些门禁缺失），而非 RFC-039 的提案内容。即使 RFC-039 被否决，这张路由表仍然成立——它回答的是"现在加一个特性要改哪些地方"。

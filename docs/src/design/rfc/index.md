---
title: "RFC 索引"
---

# YaoXiang RFC（请求评议）索引

> RFC（Request for Comments）是YaoXiang语言特性设计提案的正式提交格式。

## 目录

- [模板](#模板)
- [草案RFC](#草案rfc)
- [审核中RFC](#审核中rfc)
- [已接受RFC](#已接受rfc)
- [已废弃RFC](#已废弃rfc)
- [已拒绝RFC](#已拒绝rfc)

---

## 模板

| 文件 | 说明 |
|------|------|
| [RFC_TEMPLATE.md](RFC_TEMPLATE.md) | RFC标准模板 |
| [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) | 完整示例（模式匹配增强） |

---

## 草案RFC

| 编号 | 标题 | 作者 | 创建日期 | 状态 |
|------|------|------|----------|------|
| RFC-019 | [RFC-019: 类型级同像性 (Typed Homoiconicity) - 语法即类型](./draft/019-typed-homoiconicity.md) | 晨煦 | 2026-02-20 | 草案 |
| RFC-028 | [RFC-028：JIT 编译器 — VM 内多级执行引擎](./draft/028-jit-compiler.md) | 晨煦 | 2026-06-11 | 草案 |
| RFC-029 | [RFC-029: 模块语义系统](./draft/029-module-semantics.md) | 晨煦 | 2026-06-13 | 草案 |
| RFC-030 | [RFC-030: assert 断言机制](./draft/030-assert-mechanism.md) | 晨煦 | 2026-06-15 | 草案 |
| RFC-031 | [RFC-031：优化级别与 Pass 管理器](./draft/031-optimization-levels.md) | 晨煦 | 2026-06-16 | 草案 |
| RFC-032 | [RFC-032: spawn 统一表达式修饰 — 消除 spawn for 特殊情况](./draft/032-spawn-unified-expression.md) | 晨煦 | 2026-06-16 | 草案 |
| RFC-002 | [RFC-002：基于 libuv 的资源类型 IO 实现层](./draft/002-cross-platform-io-libuv.md) | 晨煦 | 2025-01-05 | 草案（重新审核） |

---

## 审核中RFC

| 编号 | 标题 | 作者 | 创建日期 | 状态 |
|------|------|------|----------|------|
| RFC-025 | [RFC-025: 可扩展原语类型机制](./review/025-primitive-extension.md) | 晨煦 | 2026-06-05 | 审核中 |
| RFC-026 | [RFC-026：FFI 核心机制](./review/026-ffi-core-mechanism.md) | 晨煦 | 2026-06-05 | 审核中 |

---

## 已接受RFC

| 编号 | 标题 | 作者 | 创建日期 | 状态 |
|------|------|------|----------|------|
| RFC-004 | [RFC-004: 柯里化方法的多位置联合绑定设计](./accepted/004-curry-multi-position-binding.md) | 晨煦 | 2025-01-05 | 已接受 |
| RFC-006 | [RFC-006: 文档站点建设](./accepted/006-documentation-site-optimization.md) | 晨煦 | 2025-01-05 | 已接受 |
| RFC-007 | [RFC-007: 函数定义语法统一方案](./accepted/007-function-syntax-unification.md) | 沫郁酱 | 2025-01-05 | 已接受 |
| RFC-008 | [RFC-008：Runtime 并发模型与调度器脱耦设计](./accepted/008-runtime-concurrency-model.md) | 晨煦 | 2025-01-05 | 已接受 |
| RFC-009 | [RFC-009: 所有权模型设计](./accepted/009-ownership-model.md) | 晨煦 | 2025-01-08 | 已接受 |
| ↳ RFC-009a | [RFC-009a: 令牌生命期分析——基于霍尔证明管道](./accepted/009a-borrow-proof-pipeline.md) | 晨煦 | 2026-06-13 | 已接受 |
| RFC-010 | [RFC-010: 统一类型语法 - name: type = value 模型](./accepted/010-unified-type-syntax.md) | 晨煦 | 2025-01-20 | 已接受 |
| RFC-011 | [RFC-011: 泛型系统设计 - 零成本抽象与宏替代](./accepted/011-generic-type-system.md) | 晨煦 | 2025-01-25 | 已接受 |
| ↳ RFC-011a | [RFC-011a: 接口实现与动态分发](./draft/011a-interface-implementation.md) | 晨煦 | 2026-06-14 | 草案RFC |
| RFC-012 | [RFC 012: F-String 模板字符串](./accepted/012-f-string-template-strings.md) | Chen Xu | 2025-01-27 | 已接受 |
| RFC-013 | [RFC 013: 错误代码规范](./accepted/013-error-code-specification.md) | 晨煦 | 2026-02-02 | 已接受 |
| RFC-014 | [RFC-014: 包管理系统设计](./accepted/014-package-manager.md) | 晨煦 | 2026-02-12 | 已接受 |
| ↳ RFC-014a | [RFC-014a: Registry 协议规范](./review/014a-registry-protocol.md) | 晨煦 | 2026-06-11 | 审核中RFC |
| ↳ RFC-014b | [RFC-014b: 构建系统与二进制分发](./review/014b-build-system.md) | 晨煦 | 2026-06-11 | 审核中RFC |
| ↳ RFC-014c | [RFC-014c: 工作空间支持](./review/014c-workspace.md) | 晨煦 | 2026-06-11 | 审核中RFC |
| RFC-015 | [RFC-015: YaoXiang 配置系统设计](./accepted/015-configuration-system.md) | 晨煦 | 2026-02-12 | 已接受 |
| RFC-017 | [RFC-017: 语言服务器协议（LSP）支持设计](./accepted/017-lsp-support.md) | 晨煦 | 2026-02-15 | 审核中 |
| RFC-018 | [RFC-018：LLVM AOT 编译器设计](./accepted/018-llvm-aot-compiler.md) | 晨煦 | 2026-02-15 | 已接受 |
| RFC-024 | [RFC-024：基于 spawn 块的并发模型](./accepted/024-concurrency-model.md) | 晨煦 | 2026-06-05 | 已接受 |
| RFC-027 | [RFC-027：编译期谓词与统一静态验证](./accepted/027-compile-time-evaluation-types.md) | 晨煦 | 2026-06-07 | 已接受 |

---

## 已废弃RFC

| 编号 | 标题 | 作者 | 创建日期 | 状态 |
|------|------|------|----------|------|
| RFC-001 | [RFC-001：并作模型与错误处理系统](./deprecated/001-concurrent-model-error-handling.md) | 晨煦 | 2025-01-05 | 已废弃（被 RFC-024 取代） |
| RFC-020 | [RFC-020：动态模块与 FFI 集成](./deprecated/020-dynamic-modules-ffi.md) | 晨煦 | 2026-03-14 | 已废弃 |
| RFC-021 | [RFC-021: 库驱动 FFI 扩展与跨语言调用支持](./deprecated/021-library-driven-ffi-extension.md) | 晨煦 | 2026-03-14 | 已废弃 |
| RFC-022 | [RFC 022: 霍尔逻辑静态验证支持（规约注释与规约类型）](./deprecated/022-hoare-logic-static-verification.md) | 晨煦 | 2026-03-16 | 已废弃（被 RFC-027 取代） |
| RFC-023 | [RFC-023: 闭包捕获模型](./deprecated/023-closure-capture-model.md) | 晨煦 | 2026-05-29 | 已废弃 |

---

## 已拒绝RFC

| 编号 | 标题 | 作者 | 创建日期 | 状态 |
|------|------|------|----------|------|
| RFC-003 | [RFC-003：版本规划](./rejected/003-version-planning.md) | 晨煦 | 2025-01-05 | 已拒绝 |
| RFC-005 | [RFC-005: 自动化CVE安全检查系统](./rejected/005-automated-cve-scanning.md) | 晨煦 | 2025-01-05 | 已拒绝 |
| RFC-016 | [RFC 016: 量子原生支持与多重后端集成](./rejected/016-quantum-native-support.md) | 晨煦 | 2026-02-13 | 已拒绝 |

---

## RFC生命周期

```
草案 → 审核中 → 已接受 → 已废弃（被取代）
                  ↓
               已拒绝（不通过）
```

### 状态说明

| 状态 | 位置 | 说明 |
|------|------|------|
| **草案** | `rfc/draft/` | 作者草稿，等待提交审核 |
| **审核中** | `rfc/review/` | 开放社区讨论和反馈 |
| **已接受** | `rfc/accepted/` | 成为正式设计文档，进入实现阶段 |
| **已废弃** | `rfc/deprecated/` | 曾被接受，被新设计取代 |
| **已拒绝** | `rfc/rejected/` | 被拒绝的RFC文档 |

---

## 提交RFC

1. 阅读 [RFC_TEMPLATE.md](RFC_TEMPLATE.md) 了解格式要求
2. 参考 [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) 学习写法
3. 创建新文件，命名为 `序号-描述性标题.md`
4. 将文件放入 `docs/reference/rfc/draft/` 目录
5. 更新本索引文件，添加新RFC条目
6. 提交PR进入审核流程

---

## 贡献指南

请参阅 [CONTRIBUTING.md](../../../../CONTRIBUTING.md) 了解贡献指南。
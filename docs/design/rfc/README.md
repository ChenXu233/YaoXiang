# YaoXiang RFC（请求评议）索引

> RFC（Request for Comments）是YaoXiang语言特性设计提案的正式提交格式。

## 目录

- [模板](#模板)
- [草案RFC](#草案rfc)
- [审核中RFC](#审核中rfc)
- [已接受RFC](#已接受rfc)
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
| RFC-002 | [跨平台I/O与libuv集成](002-cross-platform-io-libuv.md) | 晨煦 | 2025-01-05 | 草案 |
| RFC-005 | [自动化CVE安全检查系统](005-automated-cve-scanning.md) | 晨煦 | 2025-01-05 | 草案 |
| RFC-006 | [文档站点建设与优化方案](006-documentation-site-optimization.md) | 晨煦 | 2025-01-05 | 草案 |
---

## 审核中RFC

| 编号 | 标题 | 作者 | 创建日期 | 状态 |
|------|------|------|----------|------|
| RFC-003 | [版本规划与实现建议](003-version-planning.md) | 晨煦 | 2025-01-05 | 审核中 |
| RFC-004 | [柯里化方法的多位置联合绑定设计](004-curry-multi-position-binding.md) | 晨煦 | 2025-01-05 | 审核中 |
| RFC-010 | [统一类型语法 - 方法即数据](010-unified-type-syntax.md) | 晨煦 | 2025-01-20 | 审核中 |


---

## 已接受RFC

| 编号 | 标题 | 作者 | 创建日期 | 状态 |
|------|------|------|----------|------|
| RFC-001 | [并作模型与错误处理系统](001-concurrent-model-error-handling.md) | 晨煦 | 2025-01-05 | 审核中 |
| RFC-008 | [Runtime 并发模型与调度器脱耦设计](008-runtime-concurrency-model.md) | 晨煦 | 2025-01-05 | 已接受 |
| RFC-009 | [所有权模型 v7](009-ownership-model.md) | 晨煦 | 2025-01-05 | 审核中 |
| RFC-007 | [函数定义语法统一方案](007-function-syntax-unification.md) | 晨煦 | 2025-01-05 | 已接受 |

---

## 已拒绝RFC

暂无

---

## RFC生命周期

```
┌─────────────┐
│   草案      │  ← 作者创建
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  审核中     │  ← 开放社区讨论和反馈
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  已接受     │    │  已拒绝     │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   accepted/ │    │    rfc/     │
│ (正式设计)  │    │ (保留原位)  │
└─────────────┘    └─────────────┘
```

### 状态说明

| 状态 | 位置 | 说明 |
|------|------|------|
| **草案** | `docs/design/rfc/` | 作者草稿，等待提交审核 |
| **审核中** | `docs/design/rfc/` | 开放社区讨论和反馈 |
| **已接受** | `docs/design/accepted/` | 成为正式设计文档，进入实现阶段 |
| **已拒绝** | `docs/design/rfc/` | 保留在RFC目录，更新状态 |

---

## 提交RFC

1. 阅读 [RFC_TEMPLATE.md](RFC_TEMPLATE.md) 了解格式要求
2. 参考 [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) 学习写法
3. 创建新文件，命名为 `序号-描述性标题.md`
4. 将文件放入 `docs/design/rfc/` 目录
5. 更新本索引文件，添加新RFC条目
6. 提交PR进入审核流程

---

## 贡献指南

请参阅 [CONTRIBUTING.md](../../../../CONTRIBUTING.md) 了解贡献指南。

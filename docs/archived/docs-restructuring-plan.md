# YaoXiang 文档目录重构计划

## 概述

**目标**：重构 `docs/` 目录，创建独立的**设计讨论区**和**实现计划追踪**目录

---

## 目标目录结构

```
docs/
├── design/                    # ⭐ 设计讨论区（新增核心目录）
│   ├── README.md              # 设计文档索引
│   ├── manifesto.md           # 设计宣言
│   ├── language-spec.md       # 语言规范
│   ├── async-whitepaper.md    # 异步白皮书
│   ├── 00-wtf.md              # 设计权衡（FAQ）
│   └── 01-philosophy.md       # 设计哲学（原"一个2006年出生者的语言设计观.md"）
│
├── design/rfc/                # RFC 风格设计提案（可选）
│   └── (未来提案)
│
├── design/discussion/         # 开放讨论区（草稿）
│   └── (待讨论的设计文档)
│
├── plans/                     # ⭐ 实施计划（从 works/plans 提升）
│   ├── README.md
│   ├── book-improvement.md
│   ├── stdlib-implementation.md
│   ├── test-organization.md
│   └── async/
│       ├── implementation-plan.md
│       └── threading-safety.md
│
├── implementation/            # ⭐ 实现追踪（新增）
│   ├── README.md
│   ├── phase1/
│   │   └── type-check-inference.md
│   └── phase5/
│       ├── bytecode-generation.md
│       └── gap-analysis.md
│
├── architecture/              # 架构设计（保留）
├── guides/                    # 使用指南（保留）
├── examples/                  # 示例代码（保留）
└── reference/                 # 参考文档（保留）
```

---

## 目录职责说明

| 目录 | 职责 | 内容类型 |
|------|------|----------|
| `design/` | 已完成的设计决策讨论 | 宣言、规范、白皮书、设计权衡 |
| `design/rfc/` | 提案中的设计（可选） | RFC 文档、草稿 |
| `design/discussion/` | 待讨论的设计 | 开放问题、讨论中的草稿 |
| `plans/` | 打算进行的实现计划 | 实施路线图、任务分解 |
| `implementation/` | 已完成/进行中的实现详情 | 技术细节、阶段报告 |

---

## 迁移清单

### 1. 移动到 `design/`

| 原位置 | 新位置 |
|--------|--------|
| `docs/YaoXiang-design-manifesto.md` | `docs/design/manifesto.md` |
| `docs/YaoXiang-language-specification.md` | `docs/design/language-spec.md` |
| `docs/YaoXiang-async-whitepaper.md` | `docs/design/async-whitepaper.md` |
| `docs/YaoXiang-WTF.md` | `docs/design/00-wtf.md` |
| `docs/一个2006年出生者的语言设计观.md` | `docs/design/01-philosophy.md` |

### 2. 提升 `works/plans/` 到根级

| 原位置 | 新位置 |
|--------|--------|
| `docs/works/plans/` | `docs/plans/` |

### 3. 移动到 `implementation/`

| 原位置 | 新位置 |
|--------|--------|
| `docs/works/phase/phase1/type-check-inference-rules.md` | `docs/implementation/phase1/type-check-inference.md` |
| `docs/works/phase/phase5/phase5-bytecode-generation.md` | `docs/implementation/phase5/bytecode-generation.md` |
| `docs/works/phase/phase5/phase5-implementation-gap-analysis.md` | `docs/implementation/phase5/gap-analysis.md` |

### 4. 保留原状

| 目录 | 说明 |
|------|------|
| `docs/architecture/` | 架构设计已独立，保持不变 |
| `docs/guides/` | 用户指南已独立，保持不变 |
| `docs/examples/` | 示例代码，保持不变 |
| `docs/works/old/` | 历史归档，保留或删除 |
| `docs/works/plans/async/` | 已提升到 `plans/async/` |

### 5. 可选：更新 `docs/README.md`

需要更新文档索引以反映新的目录结构。

---

## 执行步骤

### 步骤 1：创建目录结构

```bash
mkdir -p docs/design/discussion
mkdir -p docs/design/rfc
mkdir -p docs/plans/async
mkdir -p docs/implementation/phase1
mkdir -p docs/implementation/phase5
```

### 步骤 2：移动设计文档

```bash
# 移动到 design/
mv docs/YaoXiang-design-manifesto.md docs/design/manifesto.md
mv docs/YaoXiang-language-specification.md docs/design/language-spec.md
mv docs/YaoXiang-async-whitepaper.md docs/design/async-whitepaper.md
mv docs/YaoXiang-WTF.md docs/design/00-wtf.md
mv "docs/一个2006年出生者的语言设计观.md" docs/design/01-philosophy.md

# 移动到 design/discussion/ (可选：存放待讨论草稿)
```

### 步骤 3：提升 plans 目录

```bash
# 移动 works/plans 到根级
mv docs/works/plans/* docs/plans/
rmdir docs/works/plans
```

### 步骤 4：移动实现文档

```bash
# 移动到 implementation/
mv docs/works/phase/phase1/type-check-inference-rules.md docs/implementation/phase1/type-check-inference.md
mv docs/works/phase/phase5/phase5-bytecode-generation.md docs/implementation/phase5/bytecode-generation.md
mv docs/works/phase/phase5/phase5-implementation-gap-analysis.md docs/implementation/phase5/gap-analysis.md
```

### 步骤 5：更新 docs/README.md

更新文档索引，添加新目录说明。

### 步骤 6：清理空目录

```bash
rmdir docs/works/phase/phase5
rmdir docs/works/phase/phase1
rmdir docs/works/phase
rmdir docs/works/old/archived
rmdir docs/works/old
```

---

## 向后兼容性

⚠️ **重要**：此重构会破坏现有引用，建议：

1. **不删除原文件**，先创建软链接或移动后验证
2. **更新所有内部链接**：检查 `docs/**/*.md` 中的相对路径引用
3. **更新 IDE 配置**：如果存在 `.vscode` 或其他配置

---

## 预期收益

1. **职责清晰**：设计 vs 计划 vs 实现，边界明确
2. **访问便捷**：`design/` 和 `plans/` 在根级，无需深入 `works/`
3. **扩展性好**：新增 `design/rfc/` 和 `design/discussion/` 支持 RFC 流程
4. **文档类型分明**：已完成设计、待讨论设计、实施计划、实现追踪各归其位

---

## 注意事项

- 确认是否需要保留 `works/` 目录的归档内容
- 检查是否有其他文档引用这些文件路径
- 考虑是否需要为 `design/rfc/` 建立 RFC 模板

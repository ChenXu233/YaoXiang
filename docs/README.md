# YaoXiang Documentation

YaoXiang 编程语言的官方文档。

---

<!-- language-nav-start -->
🌐 **Language / 语言** | [English](#english) | [中文](#中文)
<!-- language-nav-end -->

---

<!-- bilingual-section-start -->
## <a name="english"></a>📚 Documentation Structure

```
docs/
├── design/                    # Design discussion area
│   ├── manifesto.md           # Design manifesto (core philosophy & roadmap)
│   ├── language-spec.md       # Language specification (authoritative reference)
│   ├── async-whitepaper.md    # Async concurrency whitepaper
│   ├── manifesto-wtf.md       # Satirical version of the manifesto
│   ├── philosophy.md          # Design philosophy
│   ├── discussion/            # Open discussion area (drafts)
│   └── rfc/                   # RFC-style design proposals
│
├── plans/                     # Implementation plans
│   ├── YaoXiang-implementation-plan.md  # Overall implementation plan
│   ├── book-improvement.md              # Language guide improvement plan
│   ├── stdlib-implementation.md         # Standard library implementation plan
│   ├── test-organization.md             # Test organization improvement plan
│   └── async/
│       ├── implementation-plan.md       # Async implementation plan
│       └── threading-safety.md          # Thread safety design
│
├── implementation/            # Implementation tracking
│   ├── phase1/
│   │   └── type-check-inference.md      # Type checking and inference
│   └── phase5/
│       ├── bytecode-generation.md       # Bytecode generation
│       └── gap-analysis.md              # Implementation gap analysis
│
├── architecture/              # Architecture documentation
│   ├── README.md              # Architecture index
│   ├── project-structure.md   # Project structure
│   ├── compiler-design.md     # Compiler design
│   └── runtime-design.md      # Runtime design
│
├── guides/                    # User guides
│   ├── getting-started.md     # Quick start (5 minutes)
│   ├── getting-started.en.md  # Quick Start (English)
│   ├── error-system-design.md # Error system design
│   ├── YaoXiang-book.md       # Language guide (tutorial)
│   ├── YaoXiang-book.en.md    # Language Guide (English)
│   └── dev/                   # Developer guides
│       ├── commit-convention.md   # Commit convention
│       └── release-guide.md       # Release guide
│
├── tutorial/                  # Tutorials (detailed examples)
│   ├── zh/                    # Chinese tutorials
│   │   ├── README.md          # Tutorial index
│   │   ├── basics.md          # Quick start
│   │   ├── types.md           # Type system
│   │   ├── functions.md       # Functions and closures
│   │   ├── control-flow.md    # Control flow
│   │   └── modules.md         # Module system
│   │
│   └── en/                    # English tutorials
│       ├── README.md          # Tutorial index
│       ├── basics.md          # Quick start
│       ├── types.md           # Type system
│       ├── functions.md       # Functions and closures
│       ├── control-flow.md    # Control flow
│       └── modules.md         # Module system
│
├── maintenance/               # Maintenance specifications
│   └── MAINTENANCE.md         # Documentation maintenance rules
│
├── works/                     # Working documents
│   └── old/                   # Historical archives
│       └── archived/          # Archived documents
│
├── examples/                  # Example code
└── reference/                 # Reference documentation
```

### Directory Responsibilities

| Directory | Responsibility | Content Type |
|-----------|----------------|--------------|
| `design/` | Completed design decisions | Manifestos, specs, whitepapers, design trade-offs |
| `design/discussion/` | Designs under discussion | Open issues, drafts in discussion |
| `design/rfc/` | RFC-style design proposals | Proposed designs |
| `guides/` | Usage guides and tutorials | Quick start, language guide, developer guide |
| `tutorial/` | Detailed tutorials | Step-by-step examples, best practices |
| `plans/` | Implementation plans | Implementation roadmap, task breakdown |
| `implementation/` | Implementation details | Technical details, phase reports |
| `maintenance/` | Documentation rules | Version management, review process |

---

## Reading Order

### Getting Started

1. [Quick Start](guides/getting-started.md) - Get up and running in 5 minutes
2. [YaoXiang Guide](guides/YaoXiang-book.md) - Learn core concepts systematically
3. [Tutorial: Basics](tutorial/en/basics.md) - Variables, types, operators

### Advanced Learning

4. [Tutorial: Type System](tutorial/en/types.md) - Deep understanding of types
5. [Tutorial: Functions](tutorial/en/functions.md) - Function definitions and higher-order functions
6. [Language Specification](design/language-spec.md) - Complete syntax and semantics reference

### Advanced Content

7. [Project Structure](architecture/project-structure.md) - Codebase overview
8. [Compiler Design](architecture/compiler-design.md) - Compilation principles and implementation
9. [Runtime Design](architecture/runtime-design.md) - Virtual machine and concurrency model

### Reference Materials

- [Design Manifesto](design/manifesto.md) - Core philosophy and roadmap
- [Async Whitepaper](design/async-whitepaper.md) - Detailed concurrency model
- [Satirical Manifesto](design/manifesto-wtf.md) - Satirical version of the manifesto
- [Documentation Maintenance](maintenance/MAINTENANCE.md) - Version management, archiving rules

---

## Contributing

Contributions are welcome! Please submit a Pull Request or Issue.

## License

MIT License

---

<!-- separator-start -->
***
---

## <a name="中文"></a>📚 文档结构

```
docs/
├── design/                    # 设计讨论区
│   ├── manifesto.md           # 设计宣言（核心理念与路线图）
│   ├── language-spec.md       # 语言规范（权威参考）
│   ├── async-whitepaper.md    # 异步并发白皮书
│   ├── manifesto-wtf.md       # 宣言的讽刺版
│   ├── philosophy.md          # 设计哲学
│   ├── discussion/            # 开放讨论区（草稿）
│   └── rfc/                   # RFC 风格设计提案
│
├── plans/                     # 实施计划
│   ├── YaoXiang-implementation-plan.md  # 整体实现规划
│   ├── book-improvement.md              # 语言指南改进计划
│   ├── stdlib-implementation.md         # 标准库实现计划
│   ├── test-organization.md             # 测试组织改进计划
│   └── async/
│       ├── implementation-plan.md       # 异步实现计划
│       └── threading-safety.md          # 线程安全设计
│
├── implementation/            # 实现追踪
│   ├── phase1/
│   │   └── type-check-inference.md      # 类型检查与推断
│   └── phase5/
│       ├── bytecode-generation.md       # 字节码生成
│       └── gap-analysis.md              # 实现差距分析
│
├── architecture/              # 架构文档
│   ├── README.md              # 架构索引
│   ├── project-structure.md   # 项目结构
│   ├── compiler-design.md     # 编译器设计
│   └── runtime-design.md      # 运行时设计
│
├── guides/                    # 用户指南
│   ├── getting-started.md     # 快速入门（5 分钟上手）
│   ├── getting-started.en.md  # Quick Start (English)
│   ├── error-system-design.md # 错误系统设计
│   ├── YaoXiang-book.md       # 语言指南（入门教程）
│   ├── YaoXiang-book.en.md    # Language Guide (English)
│   └── dev/                   # 开发者指南
│       ├── commit-convention.md   # 提交规范
│       └── release-guide.md       # 发布指南
│
├── tutorial/                  # 教程（详细示例）
│   ├── zh/                    # 中文教程
│   │   ├── README.md          # 教程索引
│   │   ├── basics.md          # 快速入门
│   │   ├── types.md           # 类型系统
│   │   ├── functions.md       # 函数与闭包
│   │   ├── control-flow.md    # 控制流
│   │   └── modules.md         # 模块系统
│   │
│   └── en/                    # English tutorials
│       ├── README.md          # Tutorial index
│       ├── basics.md          # Quick start
│       ├── types.md           # Type system
│       ├── functions.md       # Functions and closures
│       ├── control-flow.md    # Control flow
│       └── modules.md         # Module system
│
├── maintenance/               # 维护规范
│   └── MAINTENANCE.md         # 文档维护规范
│
├── works/                     # 工作文档
│   └── old/                   # 历史归档
│       └── archived/          # 已归档文档
│
├── examples/                  # 示例代码
└── reference/                 # 参考文档
```

### 目录职责

| 目录 | 职责 | 内容类型 |
|------|------|----------|
| `design/` | 已完成的设计决策讨论 | 宣言、规范、白皮书、设计权衡 |
| `design/discussion/` | 待讨论的设计 | 开放问题、讨论中的草稿 |
| `design/rfc/` | RFC 风格设计提案 | 提案中的设计 |
| `guides/` | 使用指南和教程 | 快速入门、语言指南、开发者指南 |
| `tutorial/` | 详细教程 | 逐步示例、最佳实践 |
| `plans/` | 实施计划 | 实施路线图、任务分解 |
| `implementation/` | 实现详情 | 技术细节、阶段报告 |
| `maintenance/` | 文档规范 | 版本管理、审查流程 |

---

## 阅读顺序

### 新手入门

1. [快速入门](guides/getting-started.md) - 5 分钟快速上手
2. [YaoXiang 指南](guides/YaoXiang-book.md) - 系统学习核心概念
3. [教程：基础](tutorial/zh/basics.md) - 变量、类型、运算符

### 进阶学习

4. [教程：类型系统](tutorial/zh/types.md) - 深入理解类型
5. [教程：函数与闭包](tutorial/zh/functions.md) - 函数定义和高阶函数
6. [语言规范](design/language-spec.md) - 完整的语法和语义定义（参考）

### 高级内容

7. [项目结构](architecture/project-structure.md) - 代码库结构概览
8. [编译器设计](architecture/compiler-design.md) - 编译原理与实现
9. [运行时设计](architecture/runtime-design.md) - 虚拟机与并发模型

### 参考资料

- [设计宣言](design/manifesto.md) - 核心理念与路线图
- [异步白皮书](design/async-whitepaper.md) - 并作模型详解
- [宣言的讽刺版](design/manifesto-wtf.md) - 设计与使用 FAQ
- [文档维护规范](maintenance/MAINTENANCE.md) - 版本管理、归档规则

---

## 贡献

欢迎贡献文档！请提交 Pull Request 或 Issue。

## 许可

MIT License
<!-- separator-end -->
<!-- bilingual-section-end -->

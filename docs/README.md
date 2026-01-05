# YaoXiang 文档

YaoXiang 编程语言的官方文档。

## 文档结构

```
docs/
├── design/                    # 设计讨论区
│   ├── manifesto.md           # 设计宣言（核心理念与路线图）
│   ├── language-spec.md       # 语言规范（权威参考）
│   ├── async-whitepaper.md    # 异步并发白皮书
│   ├── 00-wtf.md              # 常见问题与解答
│   ├── 01-philosophy.md       # 设计哲学
│   ├── discussion/            # 开放讨论区（草稿）
│   └── rfc/                   # RFC 风格设计提案
│
├── plans/                     # 实施计划
│   ├── YaoXiang-implementation-plan.md  # 整体实现规划
│   ├── book-improvement.md    # 语言指南改进计划
│   ├── stdlib-implementation.md   # 标准库实现计划
│   ├── test-organization.md   # 测试组织改进计划
│   └── async/
│       ├── implementation-plan.md  # 异步实现计划
│       └── threading-safety.md     # 线程安全设计
│
├── implementation/            # 实现追踪
│   ├── phase1/
│   │   └── type-check-inference.md # 类型检查与推断
│   └── phase5/
│       ├── bytecode-generation.md  # 字节码生成
│       └── gap-analysis.md         # 实现差距分析
│
├── architecture/              # 架构文档
│   ├── README.md              # 架构索引
│   ├── project-structure.md   # 项目结构
│   ├── compiler-design.md     # 编译器设计
│   └── runtime-design.md      # 运行时设计
│
├── guides/                    # 用户指南
│   ├── getting-started.md     # 快速入门（5 分钟上手）
│   ├── error-system-design.md # 错误系统设计
│   ├── YaoXiang-book.md       # 语言指南（入门教程）
│   └── dev/                   # 开发者指南
│       ├── commit-convention.md   # 提交规范
│       └── release-guide.md       # 发布指南
│
├── tutorial/                  # 教程（详细示例）
│   ├── README.md              # 教程索引
│   ├── basics.md              # 快速入门
│   ├── types.md               # 类型系统
│   ├── functions.md           # 函数与闭包
│   ├── control-flow.md        # 控制流
│   ├── modules.md             # 模块系统
│   └── ...
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

## 目录职责

| 目录 | 职责 | 内容类型 |
|------|------|----------|
| `design/` | 已完成的设计决策讨论 | 宣言、规范、白皮书、设计权衡 |
| `design/discussion/` | 待讨论的设计 | 开放问题、讨论中的草稿 |
| `design/rfc/` | RFC 风格设计提案 | 提案中的设计 |
| `guides/` | 使用指南和教程 | 快速入门、语言指南、开发者指南 |
| `tutorial/` | 详细教程 | 逐步示例、最佳实践 |
| `plans/` | 打算进行的实现计划 | 实施路线图、任务分解 |
| `implementation/` | 已完成/进行中的实现详情 | 技术细节、阶段报告 |
| `maintenance/` | 文档维护规范 | 版本管理、审查流程、归档规则 |

## 阅读顺序

### 新手入门

1. [快速入门](guides/getting-started.md) - 5 分钟快速上手
2. [YaoXiang 指南](guides/YaoXiang-book.md) - 系统学习核心概念
3. [教程：基础](tutorial/basics.md) - 变量、类型、运算符

### 进阶学习

4. [教程：类型系统](tutorial/types.md) - 深入理解类型
5. [教程：函数与闭包](tutorial/functions.md) - 函数定义和高阶函数
6. [语言规范](design/language-spec.md) - 完整的语法和语义定义（参考）

### 高级内容

7. [项目结构](architecture/project-structure.md) - 代码库结构概览
8. [编译器设计](architecture/compiler-design.md) - 编译原理与实现
9. [运行时设计](architecture/runtime-design.md) - 虚拟机与并发模型

### 参考资料

- [设计宣言](design/manifesto.md) - 核心理念与路线图
- [异步白皮书](design/async-whitepaper.md) - 并作模型详解
- [常见问题](design/00-wtf.md) - 设计与使用 FAQ
- [文档维护规范](maintenance/MAINTENANCE.md) - 版本管理、归档规则

## 贡献

欢迎贡献文档！请提交 Pull Request 或 Issue。

## 许可

MIT License

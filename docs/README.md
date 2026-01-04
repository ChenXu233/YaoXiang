# YaoXiang 文档

YaoXiang 编程语言的官方文档。

## 文档结构

```
docs/
├── YaoXiang-book.md                    # 语言指南（入门教程）
├── YaoXiang-language-specification.md  # 语言规范（权威参考）
├── YaoXiang-design-manifesto.md        # 设计宣言（核心理念与路线图）
├── YaoXiang-implementation-plan.md     # 实现计划（技术细节）
├── YaoXiang-WTF.md                     # 常见问题与解答
├── YaoXiang-async-whitepaper.md        # 异步并发白皮书
├── README.md                           # 文档索引（本文档）
│
├── guides/                             # 用户指南
│   ├── getting-started.md              # 快速入门（5 分钟上手）
│   ├── error-system-design.md          # 错误系统设计
│   └── dev/                            # 开发者指南
│       ├── commit-convention.md        # 提交规范
│       └── release-guide.md            # 发布指南
│
├── architecture/                       # 架构文档（v2.0.0）
│   ├── README.md                       # 架构索引
│   ├── project-structure.md            # 项目结构
│   ├── compiler-design.md              # 编译器设计
│   └── runtime-design.md               # 运行时设计
│
└── works/                              # 工作文档
    ├── old/                            # 历史文档（已归档）
    │   └── archived/                   # 归档的旧文档
    ├── phase/                          # 阶段性文档
    └── plans/                          # 规划文档
```

## 阅读顺序

### 新手入门

1. [快速入门](guides/getting-started.md) - 5 分钟快速上手
2. [YaoXiang 指南](YaoXiang-book.md) - 系统学习核心概念

### 深入学习

3. [语言规范](YaoXiang-language-specification.md) - 完整的语法和语义定义

### 高级内容

4. [项目结构](architecture/project-structure.md) - 代码库结构概览
5. [编译器设计](architecture/compiler-design.md) - 编译原理与实现
6. [运行时设计](architecture/runtime-design.md) - 虚拟机与并发模型

### 参考资料

- [设计宣言](YaoXiang-design-manifesto.md) - 核心理念与路线图
- [异步白皮书](YaoXiang-async-whitepaper.md) - 并作模型详解
- [常见问题](YaoXiang-WTF.md) - 设计与使用 FAQ

## 贡献

欢迎贡献文档！请提交 Pull Request 或 Issue。

## 许可

MIT License

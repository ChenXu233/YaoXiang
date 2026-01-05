# YaoXiang Documentation

Official documentation for the YaoXiang programming language.

[English](README.en.md) | [中文](README.zh.md)

## Documentation Structure

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
│       ├── implementation-plan.md            # Async implementation plan
│       └── threading-safety.md               # Thread safety design
│
├── implementation/            # Implementation tracking
│   ├── phase1/
│   │   └── type-check-inference.md # Type checking and inference
│   └── phase5/
│       ├── bytecode-generation.md  # Bytecode generation
│       └── gap-analysis.md         # Implementation gap analysis
│
├── architecture/              # Architecture documentation
│   ├── README.md              # Architecture index
│   ├── project-structure.md   # Project structure
│   ├── compiler-design.md     # Compiler design
│   └── runtime-design.md      # Runtime design
│
├── guides/                    # User guides
│   ├── getting-started.md     # Quick start (5 minutes)
│   ├── error-system-design.md # Error system design
│   ├── YaoXiang-book.md       # Language guide (tutorial)
│   └── dev/                   # Developer guides
│       ├── commit-convention.md   # Commit convention
│       └── release-guide.md       # Release guide
│
├── tutorial/                  # Tutorials (detailed examples)
│   ├── zh/                    # 中文教程
│   │   ├── README.md          # 教程索引
│   │   ├── basics.md          # 快速入门
│   │   ├── types.md           # 类型系统
│   │   ├── functions.md       # 函数与闭包
│   │   ├── control-flow.md    # 控制流
│   │   └── modules.md         # 模块系统
│   │
│   └── en/                    # English Tutorials
│       ├── README.md          # Tutorial Index
│       ├── basics.md          # Quick Start
│       ├── types.md           # Type System
│       ├── functions.md       # Functions and Closures
│       ├── control-flow.md    # Control Flow
│       └── modules.md         # Module System
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

## Directory Responsibilities

| Directory | Responsibility | Content Type |
|-----------|----------------|--------------|
| `design/` | Completed design decisions | Manifestos, specs, whitepapers, design trade-offs |
| `design/discussion/` | Designs under discussion | Open issues, drafts in discussion |
| `design/rfc/` | RFC-style design proposals | Proposed designs |
| `guides/` | Usage guides and tutorials | Quick start, language guide, developer guide |
| `tutorial/` | Detailed tutorials | Step-by-step examples, best practices |
| `plans/` | Implementation plans to be done | Implementation roadmap, task breakdown |
| `implementation/` | Completed/in-progress implementation details | Technical details, phase reports |
| `maintenance/` | Documentation maintenance rules | Version management, review process, archiving rules |

## Reading Order

### Getting Started

1. [Quick Start](guides/getting-started.md) - Get up and running in 5 minutes
2. [YaoXiang Guide](guides/YaoXiang-book.md) - Learn core concepts systematically
3. [Tutorial: Basics](tutorial/en/basics.md) - Variables, types, operators

### Advanced Learning

4. [Tutorial: Type System](tutorial/en/types.md) - Deep understanding of types
5. [Tutorial: Functions and Closures](tutorial/en/functions.md) - Function definitions and higher-order functions
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

## Contributing

Contributions are welcome! Please submit a Pull Request or Issue.

## License

MIT License

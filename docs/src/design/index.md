# YaoXiang 设计文档

> 道生一，一生二，二生三，三生万物。

本目录包含 YaoXiang 编程语言的设计决策、提案和讨论。

## 核心设计理念

| 理念 | 描述 |
|------|------|
| **一切皆类型** | 值、函数、模块都是类型；类型是一等公民 |
| **自然语法** | Python 般的可读性，接近自然语言 |
| **所有权模型** | 零成本抽象，无 GC，高性能 |
| **并作模型** | 同步语法，异步本质，自动并行 |
| **AI 友好** | 严格结构化，清晰的 AST |

## 设计文档结构

```
design/
├── index.md              # 本索引
├── accepted/             # 已接受的设计提案
│   └── *.md
├── rfc/                  # RFC 提案（待审议）
│   ├── *.md
│   └── RFC_TEMPLATE.md
└── discussion/           # 设计讨论区（开放讨论）
    └── *.md
```

## 已接受的设计提案

| 文档 | 状态 | 描述 |
|------|------|------|
| [008-并发模型](./accepted/008-runtime-concurrency-model.md) | ✅ 正式 | 并作模型与任务调度器设计 |

> 查看 [`accepted/`](./accepted/) 目录获取完整列表。

## RFC 提案

> RFC（Request for Comments）是新特性和重大变更的提案流程。

### 活跃提案

| 编号 | 标题 | 状态 |
|------|------|------|
| RFC-003 | 版本规划 | 待审议 |
| RFC-005 | 自动化 CVE 扫描 | 待审议 |
| RFC-006 | 文档站点优化 | 待审议 |
| RFC-012 | f-string 模板字符串 | 待审议 |

### RFC 模板

提交新提案前，请参考：
- [RFC_TEMPLATE.md](./rfc/RFC_TEMPLATE.md)
- [完整示例](./rfc/EXAMPLE_full_feature_proposal.md)

## 参与设计讨论

### 提案流程

```
1. 起草提案（使用 RFC 模板）
   → 放入 rfc/ 目录

2. 社区讨论
   → 在 rfc/REPO 对应 issue 中讨论

3. 核心团队评审
   → 接受 → 移入 accepted/
   → 拒绝 → 移入 archived/ 或删除
```

### 设计原则

- **明确边界**：每个设计决策应该有清晰的适用范围
- **实用优先**：解决实际问题，而不是假想威胁
- **渐进透明**：并发模型的分层设计（L1-L3）
- **用户可见行为不变**：Never break userspace

## 代码示例

```yaoxiang
# 类型定义
type Point = { x: Float, y: Float }
type Result[T, E] = { ok(T) | err(E) }

# 函数定义
add: (a: Int, b: Int) -> Int = a + b

# 并作函数（自动并发）
fetch_data: (url: String) -> JSON spawn = {
    HTTP.get(url).json()
}

# 主函数
main: () -> Void = {
    print("Hello, YaoXiang!")
}
```

## 关键设计决策

### 1. 类型系统

- **统一类型语法**：废除 `enum`、`struct`、`union`，统一用 `type`
- **构造器即类型**：消除"类型"与"值"的鸿沟
- **泛型支持**：编译期单态化，零运行时开销

### 2. 并作模型

```yaoxiang
# 三层并发抽象

# L1: @blocking 同步（禁用并行）
fetch: (String) -> JSON @blocking = (url) => { ... }

# L2: spawn 显式并发
process: () -> Void spawn = () => {
    users = fetch_users()
    posts = fetch_posts()  # 自动并行
}

# L3: 完全透明（默认）
compute: (Int) -> Int = (n) => {
    a = heavy_calc(1)  # 系统自动分析依赖
    b = heavy_calc(2)
    c = heavy_calc(3)
    a + b + c
}
```

### 3. 错误处理

```yaoxiang
type Result[T, E] = ok(T) | err(E)

process: () -> Result[Data, Error] = {
    data = fetch_data()?      # ? 运算符透明传播
    transformed = transform(data)?
    save(transformed)?
}
```

## 相关资源

- [教程](../tutorial/) - 学习使用 YaoXiang
- [参考文档](../reference/) - API 和标准库
- [GitHub Discussions](https://github.com/ChenXu233/YaoXiang/discussions)
- [贡献指南](../tutorial/contributing.md)

## 历史归档

设计过程中的历史文档已移至 [`docs/old/`](../../old/) 目录，包括：
- 早期架构设计
- 已废弃的提案
- 过时的实现计划

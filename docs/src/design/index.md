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
├── deprecated/           # 已废弃（被新设计取代）
│   └── *.md
├── rejected/             # 已拒绝
│   └── *.md
├── rfc/
│   ├── draft/            # 草案（工作进行中）
│   ├── review/           # 审核中（开放讨论）
│   ├── accepted/         # 已接受（设计通过）
│   ├── deprecated/       # 已废弃（被取代）
│   └── rejected/         # 已拒绝（不通过）
└── discussion/           # 设计讨论区（开放讨论）
    └── *.md
```

## 已接受的设计提案

| 文档 | 状态 | 描述 |
|------|------|------|
| [RFC-010 统一类型语法](./rfc/accepted/010-unified-type-syntax.md) | ✅ 已接受 | 统一类型定义语法 |
| [RFC-011 泛型类型系统](./rfc/accepted/011-generic-type-system.md) | ✅ 已接受 | 泛型类型系统设计 |
| [RFC-009 所有权模型](./rfc/accepted/009-ownership-model.md) | ✅ 已接受 | 所有权与借用系统设计 |
| [RFC-024 并发模型](./rfc/accepted/024-concurrency-model.md) | ✅ 已接受 | spawn并发原语语义 |
| [RFC-027 编译期断言](./rfc/accepted/027-compile-time-evaluation-types.md) | ✅ 已接受 | 编译期谓词与静态验证 |
|
| > 查看 [`rfc/accepted/`](./rfc/accepted/) 目录获取完整列表（共16个），及 [`rfc/index.md`](./rfc/index.md) 查看最新状态。

## RFC 提案

> RFC（Request for Comments）是新特性和重大变更的提案流程。


### 活跃提案
| 编号 | 标题 | 状态 |
|------|------|------|
| RFC-019 | 类型化同像性 | 草案 |
| RFC-028 | JIT编译器 | 草案 |
| RFC-029 | 模块语义系统 | 草案 |
| RFC-031 | 优化级别 | 草案 |
| RFC-033 | ^^反射运算符 | 草案 |
| RFC-034 | 调试工具链 | 草案 |
| RFC-035 | MCP Server | 草案 |
| RFC-002 | 跨平台IO(libuv) | 草案 |
| RFC-026b | yx-bindgen | 草案 |
| RFC-011a | 接口实现与动态分发 | 审核中 |
| RFC-014a | Registry协议 | 审核中 |
| RFC-014b | 构建系统 | 审核中 |
| RFC-014c | 工作空间 | 审核中 |
| RFC-026a | 可扩展FFI | 审核中 |
| RFC-032 | spawn统一表达式 | 审核中 |

### 已接受提案
| 编号 | 标题 | 状态 |
|------|------|------|
| RFC-004 | 柯里化多位置绑定 | 已接受 |
| RFC-006 | 文档站点优化 | 已接受 |
| RFC-007 | 函数语法统一 | 已接受 |
| RFC-008 | 运行时并发模型 | 已接受 |
| RFC-009 | 所有权模型 | 已接受 |
| RFC-009a | 令牌生命期分析 | 已接受 |
| RFC-010 | 统一类型语法 | 已接受 |
| RFC-011 | 泛型系统 | 已接受 |
| RFC-012 | f-string | 已接受 |
| RFC-013 | 错误码规范 | 已接受 |
| RFC-014 | 包管理器 | 已接受 |
| RFC-015 | 配置系统 | 已接受 |
| RFC-017 | LSP支持 | 已接受 |
| RFC-018 | LLVM AOT编译器 | 已接受 |
| RFC-024 | 并发模型 | 已接受 |
| RFC-026 | FFI核心机制 | 已接受 |
| RFC-027 | 编译期断言 | 已接受 |
| RFC-030 | assert断言机制 | 已接受 |

### 已拒绝提案
| 编号 | 标题 | 状态 |
|------|------|------|
| RFC-003 | 版本规划 | 已拒绝 |
| RFC-005 | CVE扫描 | 已拒绝 |
| RFC-016 | 量子原生支持 | 已拒绝 |
| RFC-025 | 原语类型扩展 | 已拒绝 |
### RFC 模板

提交新提案前，请参考：
- [RFC_TEMPLATE.md](./rfc/RFC_TEMPLATE.md)
- [完整示例](./rfc/EXAMPLE_full_feature_proposal.md)

## 参与设计讨论

### RFC 生命周期

RFC 提案有 5 个状态：

| 状态 | 含义 |
|------|------|
| 草案 | 工作进行中 |
| 审核中 | 开放讨论 |
| 已接受 | 设计通过 |
| 已废弃 | 曾被接受，被新设计取代 |
| 已拒绝 | 不通过 |

完整生命周期：
```
草案 → 审核中 → 已接受 → 已废弃（被取代）
                  ↓
               已拒绝（不通过）
```

### 提案流程

```
1. 起草提案（使用 RFC 模板）
   → 放入 rfc/draft/

2. 提交审核
   → 移入 rfc/review/，开放社区讨论

3. 核心团队评审
   → 接受 → 移入 rfc/accepted/
   → 拒绝 → 移入 rfc/rejected/

4. 后续维护
   → 被取代 → 移入 rfc/deprecated/
```

### 设计原则

- **明确边界**：每个设计决策应该有清晰的适用范围
- **实用优先**：解决实际问题，而不是假想威胁
- **用户可见行为不变**：Never break userspace

## 代码示例

```yaoxiang
// 类型定义
Point: Type = { x: Float, y: Float }
Result: Type(T, E) = { ok(T) | err(E) }

// 函数定义
add: (a: Int, b: Int) -> Int = a + b

// 主函数
main: () -> Void = {
    print("Hello, YaoXiang!")
}
```

## 关键设计决策

### 1. 类型系统

- **统一类型语法**：废除 `enum`、`struct`、`union`，统一用 `Name: Type = {...}`
- **构造器即类型**：消除"类型"与"值"的鸿沟
- **泛型支持**：编译期单态化，零运行时开销

### 2. 并作模型

```yaoxiang
// 并作模型：默认顺序执行，spawn 引入数据流并行

// 默认顺序执行
compute: (Int) -> Int = (n) => {
    a = heavy_calc(1)
    b = heavy_calc(2)  // 顺序执行，等 a 完成
    c = heavy_calc(3)  // 顺序执行，等 b 完成
    a + b + c
}

// spawn 块引入数据流并行
process: () -> Void = () => {
    spawn {
        users = fetch_users()   // 并行
        posts = fetch_posts()   // 并行
    }
    // 调用方同步阻塞等待结果
    render(users, posts)
}
```

### 3. 错误处理

```yaoxiang
Result: Type(T, E) = { ok(T) | err(E) }

process: () -> Result(Data, Error) = {
    data = fetch_data()?      // ? 运算符透明传播
    transformed = transform(data)?
    save(transformed)?
}
```

## 相关资源

- [教程](../tutorial/) - 学习使用 YaoXiang
- [参考文档](../reference/) - API 和标准库
- [语言规范](../reference/language-spec/index.md) - 完整的语言规范
- [GitHub Discussions](https://github.com/ChenXu233/YaoXiang/discussions)
- [贡献指南](../tutorial/contributing.md)

## 历史归档

设计过程中的历史文档已移至 [`docs/old/`](../../old/) 目录，包括：
- 早期架构设计
- 已废弃的提案
- 过时的实现计划

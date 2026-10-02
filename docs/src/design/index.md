# YaoXiang 设计文档

> 道生一，一生二，二生三，三生万物。

本目录包含 YaoXiang 编程语言的设计决策、提案和讨论。

## 核心设计理念

| 理念           | 描述                                   |
| -------------- | -------------------------------------- |
| **一切皆类型** | 值、函数、模块都是类型；类型是一等公民 |
| **自然语法**   | Python 般的可读性，接近自然语言        |
| **所有权模型** | 零成本抽象，无 GC，高性能              |
| **并作模型**   | 同步语法，异步本质，自动并行           |
| **AI 友好**    | 严格结构化，清晰的 AST                 |

## 设计文档结构

```
design/
├── index.md                        # 本索引
├── manifesto.md                    # 爻象宣言
├── manifesto-wtf.md                # 宣言 WTF 版
├── 2006-born-language-design.md    # 一个 2006 年出生者的语言设计观
├── rfc/                            # RFC 提案，状态即目录
│   ├── draft/                      # 草案（工作进行中）
│   ├── review/                     # 审核中（开放讨论）
│   ├── accepted/                   # 已接受（设计通过）
│   ├── deprecated/                 # 已废弃（被取代）
│   └── rejected/                   # 已拒绝（不通过）
├── check/                          # yx check 静态检查设计规范
└── formatter/                      # yx format 格式化规范
```

## RFC 提案

> RFC（Request for Comments）是新特性与重大变更的提案流程。

**状态以 [RFC 追踪表](./rfc/TRACKING.md) 为唯一权威。** 该文件由 `scripts/rfc/check_tracking.py`
在 CI 中依据每个 RFC 所在的目录自动生成，状态由目录反推，因此永远与磁盘上的实际文件一致。本页
不再重复维护状态列表——手写列表必然过时。

| 用途                       | 入口                                                        |
| -------------------------- | ----------------------------------------------------------- |
| 完整状态列表（自动生成）   | [rfc/TRACKING.md](./rfc/TRACKING.md)                        |
| 按状态分组的索引与提交指引 | [rfc/index.md](./rfc/index.md)                              |
| 新提案模板                 | [rfc/RFC_TEMPLATE.md](./rfc/RFC_TEMPLATE.md)                |
| 完整写法示例               | [rfc/EXAMPLE_full_feature_proposal.md](./rfc/EXAMPLE_full_feature_proposal.md) |

## 参与设计讨论

### RFC 生命周期

RFC 提案有 5 个状态：

| 状态   | 含义                   |
| ------ | ---------------------- |
| 草案   | 工作进行中             |
| 审核中 | 开放讨论               |
| 已接受 | 设计通过               |
| 已废弃 | 曾被接受，被新设计取代 |
| 已拒绝 | 不通过                 |

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
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

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
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

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
- [贡献指南](../dev/contributing.md)

## 历史归档

设计过程中的历史文档已移至 `docs/src/archive/` 目录（该目录不参与站点构建），包括：

- 早期架构设计
- 已废弃的提案
- 过时的实现计划

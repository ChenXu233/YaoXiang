# RFC-001：并作模型与错误处理系统

> **状态**: 已接受
> **作者**: 晨煦
> **创建日期**: 2025-01-05
> **最后更新**: 2025-01-06

## 设计来源与参考

本文档的设计基于以下文档，并作为 language-spec 的详细设计来源：

| 文档 | 关系 | 说明 |
|------|------|------|
| [async-whitepaper](../async-whitepaper.md) | **设计源头** | 并作模型的理论基础和核心概念 |
| [language-spec](../language-spec.md) | **规范目标** | 本RFC的设计将整合到语言规范中 |

> **说明**：本RFC是对 [async-whitepaper](../async-whitepaper.md) 中提出的并作模型的具体化和规范化，将其转化为可实现的语言设计。

## 摘要

提出YaoXiang的并作模型（三层并发架构）、副作用处理机制、DAG依赖分析、Result类型系统和错误图可视化。核心设计理念源自《易经》"万物并作，吾以观复"——以同步语法描述逻辑，运行时自动并发执行。

## 快速选择

| 场景 | 写法 | 说明 |
|------|------|------|
| 自动并行 | 不写注解 | 默认行为，最大化并行 |
| 显式声明并行 | `@auto` | 与默认行为相同，可读性 |
| 同步等待 | `@eager` | 强制等待依赖完成 |
| 完全顺序 | `@block` | 无并发，调试用 |
| 局部并发 | `spawn` | @block 作用域内并发 |

## 动机

### 为什么需要并作模型？

当前主流语言的并发模型存在明显缺陷：

| 语言 | 并发模型 | 问题 |
|------|----------|------|
| Rust | async/await + tokio | 异步传染、学习曲线陡峭 |
| Go | goroutine | 无类型安全、逃逸分析困难 |
| Python | asyncio | GIL限制、性能差 |
| JavaScript | Promise/async | 回调地狱虽缓解但仍复杂 |

### 核心矛盾

1. **透明性 vs 可控性**：完全透明但不可控 vs 完全可控但不透明
2. **并发 vs 可调试**：并发程序难调试 vs 可调试程序难并发
3. **安全 vs 性能**：GC安全但有开销 vs 手动管理高效但危险

### 并作模型的解决思路

```
传统困境 ────────────── 并作模型 ────────────── 解决方案
完全透明但不可控 ────→ 三层模型 ────→ 从L1到L3渐进式采用
并发程序难调试 ──────→ 图调试 ──────→ 错误图可视化
GC安全但有开销 ──────→ 所有权 ──────→ Rust式编译时检查
```

## 提案

### 1. 并作模型：三层并发架构

并作模型允许开发者以同步、顺序的思维描述逻辑，而语言运行时令其中的计算单元如万物并作般自动、高效地并发执行。

#### 三层抽象

> **说明**：L1/L2/L3 是**心智模型**，帮助用户理解不同场景下的并发行为。实际实现只有一套机制：资源类型 + DAG 自动分析 + @block/@eager/@auto 注解控制。

| 层级 | 心智模型 | 语法 | 执行方式 | 并行度 | 适用场景 |
|------|----------|------|----------|--------|----------|
| **L1** | 禁止并发 | `@block` | 无 DAG，纯顺序执行 | ❌ 无 | 调试、新手、关键代码段 |
| **L2** | 部分并发 | `spawn` | 开发者可控 DAG | ⚠️ 部分 | 中级用户、需要控制并发 |
| **L3** | 完全并发 | 默认 | 自动分析 DAG | ✅ 完整 | 专家、自动并行优化 |

#### L1: `@block` 同步模式

```yaoxiang
# L1: @block 同步模式（无 DAG，纯顺序执行）
fetch_sync: (String) -> JSON @block = (url) => {
    HTTP.get(url).json()
}

main: () -> Void @block = () => {
    # 严格顺序执行，无任何并发
    data1 = fetch_sync("https://api.example.com/data1")
    data2 = fetch_sync("https://api.example.com/data2")
    process(data1, data2)
}
```

#### L2: 显式 spawn 并发

```yaoxiang
# L2: 显式 spawn 并发
fetch_data: (String) -> JSON spawn = (url) => {
    HTTP.get(url).json()
}

process_users_and_posts: () -> Void spawn = () => {
    users = fetch_data("https://api.example.com/users")
    posts = fetch_data("https://api.example.com/posts")
    # users 和 posts 自动并行执行
    print("Users: " + users.length.to_string())
    print("Posts: " + posts.length.to_string())
}
```

#### L3: 完全透明（默认）

```yaoxiang
# L3: 完全透明（默认模式）
heavy_calc: (Int) -> Int = (n) => {
    fibonacci(n)
}

auto_parallel: (Int) -> Int = (n) => {
    a = heavy_calc(1)
    b = heavy_calc(2)
    c = heavy_calc(3)
    a + b + c  # 在此处等待所有结果
}
```

### 2. 三种注解完整对比

| 维度 | `@auto`（默认） | `@eager` | `@block` |
|------|-----------------|----------|----------|
| **spawn 执行** | 异步执行，调度器响应 | 异步执行 + 挂起等待 | 强制同步执行 |
| **普通调用** | 异步执行 | 同步执行 | 同步执行 |
| **并行度** | ✅ 完全并行 | ⚠️ 部分并行 | ❌ 无并行 |
| **调度器参与** | ✅ 完整参与 | ✅ 完整参与 | ❌ 不参与 |
| **DAG 构建** | ✅ 构建 | ✅ 构建 | ❌ 不构建 |

**选择指南**：
- 最大并发优化 → `@auto`（默认）
- 需要有序副作用 → `@eager`
- 调试/新手/关键代码 → `@block`

```yaoxiang
# @auto（默认）：最大化并行
heavy_calc: (Int) -> Int = (n) => {
    fibonacci(n)  # 默认自动并行
}

async_heavy_calc: spawn (Int) -> Int = (n) => {
    fibonacci(n)  # 默认自动并行
}

calc_all: () -> Int = () => {
    a = heavy_calc(1)           # 异步执行
    b = heavy_calc(2)           # 异步执行
    c = async_heavy_calc(3)     # 异步执行
    d = async_heavy_calc(4)     # 异步执行
    a + b + c + d               # 需要值时，调度器保证完成
}

# @eager：同步等待结果
@eager
calc_sequential: () -> Int = () => {
    a = heavy_calc(1)         # 同步执行
    b = heavy_calc(2)         # 同步执行
    c = async_heavy_calc(3)   # 异步执行 + 挂起等待
    d = async_heavy_calc(4)   # 异步执行 + 挂起等待
    a + b + c + d             # 需要值时，调度器保证完成
}

# @block：纯顺序执行
@block
calc_simple: () -> Int = () => {
    a = heavy_calc(1)   # 强制同步执行
    b = heavy_calc(2)   # 同步执行
    a + b
}
```

### 3. DAG 依赖分析

#### 3.1 核心原则：同步语法，异步本质

```
程序 → 编译时分析 → DAG（有向无环图）
           ↓
       运行时调度 → 自动并行
```

#### 3.2 资源冲突与副作用处理

**核心观点**：DAG 依赖 = 数据流依赖。资源标识应通过变量传递。

```yaoxiang
# ✅ 正确：变量传递资源标识，DAG 自动保证顺序
filename: String = "data.txt"
File.write(filename, x)     # 依赖 filename 变量
File.write(filename, y)     # 依赖 filename 变量
# DAG 认为两个操作都读取 filename，自动按顺序并发执行

# ⚠️ 用户责任：字面量直接使用
File.write("data.txt", x)   # 无变量依赖
File.write("data.txt", y)   # 无变量依赖
# 并行执行导致数据竞争，用户负责避免此模式
```

#### 3.3 资源类型与副作用抽象（v0.6+）

> **设计变更**：原 "Void 函数急切规则" 已退役，由资源类型统一管理副作用。

**核心思想**：所有副作用都是"资源操作"，通过资源类型标记，编译器自动构建 DAG 依赖。

```yaoxiang
# 资源类型声明
type Resource             # 资源标记
type FilePath: Resource   # 文件路径是资源
type HttpUrl: Resource    # URL 是资源
type DBUrl: Resource      # 数据库 URL 是资源
type Console: Resource    # 控制台是资源

# 资源操作自动构建 DAG
main: () -> Void = () => {
    log("开始")              # Console 资源操作
    save("data.json", data)  # FilePath 资源操作
    log("完成")              # Console 资源操作
    # DAG: log → save → log，自动串行
}

# 不同资源自动并行
fetch_users: () -> JSON = () => HTTP.get("api/users")
fetch_posts: () -> JSON = () => HTTP.get("api/posts")

main: () -> Void = () => {
    users = fetch_users()    # HttpUrl 资源
    posts = fetch_posts()    # HttpUrl 资源，不同 URL → 并行
    render(users, posts)
}
```

##### 3.3.1 资源操作识别机制

**资源操作通过类型约束自动识别**，无需特殊语法标记：

1. **底层类型约束**：参数类型为资源类型的函数自动识别为资源操作
   ```yaoxiang
   write: (FilePath, Data) -> Void   # FilePath 是 Resource，自动识别
   log: (String) -> Void             # Console 是 Resource，自动识别
   ```

2. **标准库强制约束**：标准库提供的资源操作函数已内置类型约束
   ```yaoxiang
   # 标准库实现
   write: [R: FileSystem](FilePath[R], Data) -> Void

   # 用户调用时自动识别为资源操作
   write("data.txt", data)  # FilePath 字面量，编译器识别为资源
   ```

3. **用户自定义资源**：用户定义的资源类型自动继承资源操作识别
   ```yaoxiang
   type Database: Resource

   query: (Database, String) -> Result[Row, Error]
   # 参数 Database 是 Resource，自动识别为资源操作
   ```

##### 3.3.2 运行时资源处理

对于运行时才确定的资源标识，通过**变量传递 + 数据流分析**处理：

```yaoxiang
# 运行时资源路径
path: String = get_config_path()    # 运行时获取
file: File = open(path)             # 打开文件资源

# 数据流分析自动追踪依赖
write(file, data1)                  # 依赖 file 变量
write(file, data2)                  # 依赖 file 变量
# 编译器分析：两个 write 都依赖 file，自动串行

# 复杂情况：变量来源追踪
path1 = compute_path("file1")       # 运行时计算
path2 = compute_path("file2")       # 运行时计算
# 编译器保守处理：无法确认是否同一资源，默认串行
# 用户可显式标记优化：@unsafe_parallel(path1, path2)
```

**资源类型操作规则**：
- 对同一资源的操作 → 自动添加 DAG 依赖边 → 串行
- 对不同资源的操作 → 无依赖边 → 可并行

### 4. Result 类型与错误处理

```yaoxiang
# 标准 Result 类型
type Result[T, E] = ok(T) | err(E)

# ? 运算符透明错误传播
process: () -> Result[Data, Error] = {
    data = fetch_data()?
    processed = transform(data)?
    save(processed)?
}
```

### 5. 编译期优化策略

#### 5.1 静态 DAG 构建

| 层级 | 策略 | 适用场景 |
|------|------|----------|
| L0 | 完全动态 | 复杂运行时条件 |
| L1 | 静态节点识别 | 纯函数、无依赖计算 |
| L2 | 静态边分析 | 固定依赖结构 |
| L3 | 完全静态 | 编译期生成执行计划 |

#### 5.2 L1 自动回退

对于小函数，自动回退到 `@block` 模式，避免调度开销。

| 条件 | 阈值 | 说明 |
|------|------|------|
| 指令数 | < 50 | 少于 50 条指令回退 |
| 节点数 | < 10 | 少于 10 个 DAG 节点回退 |

```bash
# 可调整阈值
yaoxiangc --l1-threshold=100  # 提高阈值
yaoxiangc --no-l1-fallback    # 禁用自动回退
```

### 6. DAG 详细设计

#### 6.1 节点类型

```rust
// 节点类型
enum NodeKind {
    Task,      // 任务节点
    Value,     // 值节点
    Control,   // 控制流节点
}

// 节点结构
struct Node {
    id: NodeId,
    kind: NodeKind,
    inputs: Vec<ValueNodeId>,  // 输入依赖
    outputs: Vec<ValueNodeId>, // 输出值
    span: Span,                // 源码位置
}
```

#### 6.2 边类型

| 边类型 | 符号 | 语义 |
|--------|------|------|
| DataEdge | ──► | 数据依赖（值流动） |
| ControlEdge | ──● | 控制依赖（顺序执行） |
| SpawnEdge | ──◎ | 并发入口（可并行起点） |

**冲突规则**：
- Data + Data = 可并行
- Data + Control = 串行化
- Control + Control = 串行化

#### 6.3 构建算法

```rust
fn build_dag(ast: &AST) -> DAG {
    // 1. 遍历函数，收集 TaskNode
    for fn in ast.functions {
        create_task_node(fn)
    }

    // 2. 分析数据依赖（Def-Use 链）
    for (def, uses) in def_use_chains {
        add_data_edge(def, use)
    }

    // 3. 分析控制依赖（Statement 顺序）
    for i in 0..statements.len() - 1 {
        add_control_edge(i, i + 1)
    }

    // 4. 验证 DAG 无环
    if !is_acyclic() {
        error("循环依赖")
    }

    dag
}
```

#### 6.4 拓扑排序与调度

```rust
fn topological_sort(dag: &DAG) -> Vec<NodeId> {
    in_degree = dag.nodes.map(|n| n.incoming_edges.len())
    ready_queue: Vec<NodeId> = in_degree.filter(|d| d == 0)

    while ready_queue not empty {
        node = ready_queue.pop_front()
        result.push(node)

        for succ in node.successors {
            in_degree[succ] -= 1
            if in_degree[succ] == 0 {
                ready_queue.push_back(succ)
            }
        }
    }

    result
}
```

### 7. 编译器与运行时改动

| 组件 | 改动 |
|------|------|
| 编译器 | 新增 `@block`、`@eager`、`spawn` 语法支持 |
| Standard 运行时 | DAG Builder + 拓扑排序 + 调度器 |
| Full 运行时 | Standard + Work Stealing + @eager |

### 8. 类型系统

```
Send ──► 可安全跨线程传输
  └──► Sync ──► 可安全跨线程共享
       └──► 满足 Send + Sync 的类型可自动并发

Arc[T] 实现 Send + Sync（线程安全引用计数）
Mutex[T] 提供内部可变性
```

### 9. 语法 BNF

```
FunctionDecl ::= Identifier ':' Type ('spawn' | '@block')? '=' Expr
ConcurrentBlock ::= 'spawn' '{' Expr (',' Expr)* '}'
Pattern ::= 'ok' '(' Pattern ')' | 'err' '(' Pattern ')' | '_'
ErrorPropagate ::= Expr '?'
```

### 10. 向后兼容性

- ✅ 完全向后兼容
- 仅新增语法，原有代码无需修改

错误处理详细设计移至单独的错误处理规范文档。

## 权衡

### 优点

1. **渐进式采用**：三层模型适应不同技能水平
2. **自然语法**：同步代码获得并行性能
3. **编译时安全**：Send/Sync 约束消除数据竞争
4. **可调试性**：错误图提供清晰的错误传播视图
5. **资源安全**：编译时资源冲突检测防止死锁

### 缺点

1. **学习曲线**：需要理解 DAG 依赖概念
2. **编译时间**：全程序 DAG 分析可能较慢
3. **工具链复杂度**：需要全新的调试和可视化工具
4. **调试挑战**：L3 模式的调试需要特殊工具支持

## 替代方案

| 方案 | 为什么不选择 |
|------|--------------|
| 仅支持显式 async/await | 无法实现真正的透明并发 |
| 仅支持完全透明并发 | 用户失去控制权，无法调试 |
| Go式 goroutine | 无类型安全、无法编译时检查 |
| 仅 L1 模式 | 放弃并作模型的核心价值 |

## 实现策略

### 阶段划分

1. **阶段1 (v0.1)**： `@block` 同步模式、基础类型
2. **阶段2 (v0.2)**： FlowScheduler 调度器实现
3. **阶段3 (v0.3)**： `spawn` 块、显式并发
4. **阶段4 (v0.5)**： L3 完全透明、DAG 自动分析
5. **阶段5 (v0.6)**： 错误图、图调试器
6. **阶段6 (v1.0)**： 生产可用优化

### 实际实现状态

#### ✅ 已完成（2025-01-23）

**基础任务系统**：
- Task/TaskId/TaskState 定义完成
- 任务配置与优先级系统
- 基础调度器框架
- 位置：`src/backends/runtime/task.rs`

**三层运行时架构**：
- Embedded Runtime 设计完成
- Standard Runtime 设计完成
- Full Runtime 设计完成
- 位置：`src/backends/runtime/mod.rs`

#### 🚧 进行中

**FlowScheduler 调度器**：
- 设计文档完成（2026-01-04）
- 架构设计：`src/runtime/scheduler/`
- DAG 调度器、工作窃取、libuv IO
- 位置：`.claude/plan/flow-scheduler-implementation.md`

#### 📋 计划中

**错误处理系统实现**：
- Result 类型系统
- 错误传播机制
- 错误图可视化
- DAG 错误分析

### 依赖关系

- RFC-001 无外部依赖（基础核心）
- RFC-008（Runtime 并发模型）→ 设计完成
- RFC-003（版本规划）→ 已更新
- RFC-011（泛型系统）→ 设计完成

### 风险

1. **DAG 分析性能**：全程序分析可能 O(n²)，需要优化算法
2. **工具链缺失**：调试器需要从零开发
3. **用户接受度**：透明并发违反直觉，需要良好文档
4. **FlowScheduler 实现复杂度**：工业级调度器实现难度高

## 开放问题

> 以下问题需要在 RFC 进入"已接受"状态前确定

| 议题 | 状态 | 说明 |
|------|------|------|
| @ordered 在循环中的语义定义 | ✅ 已解决 | 删除 `@ordered`，使用 `@eager` 替代 |
| @block 注解位置 | ✅ 已决定 | 方案A：返回类型后，与 `spawn` 位置一致 |
| 资源冲突检测规则 | ✅ 已解决 | DAG 数据流依赖，用户通过变量传递 |
| Send/Sync 约束检查 | ✅ 已解决 | 编译时强制检查：跨节点值必须 Send，共享引用必须 Sync |
| 副作用一致性保证 | ✅ 已解决 | 不引入副作用系统，用户负责 |



### 实现问题（实现时注意）

> 本节记录 RFC-001 设计中可能影响实现的问题及其缓解建议，供实现阶段参考。

#### 严重问题（高优先级）

- [x] **DAG 构建与执行顺序冲突** ✅ 已解决
  - 解决方案：错误沿依赖边向上游传播，不考虑执行顺序
  - 确保与错误图报错结合，可追踪错误引发的函数

#### 中等问题（中优先级）

- [x] **DAG 构建时间复杂度** ✅ 已解决
  - 解决方案：使用增量构建 + 缓存

- [x] **资源冲突检测规则** ✅ 已解决（DAG 数据流依赖）
  - **核心观点**：DAG 依赖 = 数据流依赖，资源标识应通过变量传递
  - **解决方案**：
    - DAG 仅分析变量间的数据流依赖
    - 资源操作应通过变量传递标识符，DAG 自动构建顺序依赖
    - 字面量直接使用导致的问题是用户设计问题，非语言责任
  ```yaoxiang
  # ✅ 正确：变量传递资源标识，DAG 自动保证顺序
  filename: String = "data.txt"
  File.write(filename, x)     # 依赖 filename 变量
  File.write(filename, y)     # 依赖 filename 变量
  # DAG 认为两个操作都读取 filename，自动按顺序执行

  # ⚠️ 用户责任：字面量直接使用
  File.write("data.txt", x)   # 无变量依赖
  File.write("data.txt", y)   # 无变量依赖
  # DAG 中视为独立节点，可能并行执行
  # 这是用户设计问题，语言不自动保护

  # ❌ 用户问题：故意创建重复资源
  db1: DB = connect_database()  # 变量 db1
  db2: DB = connect_database()  # 变量 db2，值相同但变量不同
  # db1 和 db2 是不同变量，DAG 认为无依赖
  # 这是用户设计错误，非语言问题
  ```

- [x] **副作用一致性保证** ✅ 已解决（信任用户 + 文档引导）
  - **核心观点**：副作用依赖 = 数据流依赖，通过变量传递构建 DAG
  - **解决方案**：
    - 不引入复杂副作用系统
    - 用户通过变量传递资源句柄，DAG 自动管理顺序
    - 用户对自己设计负责，语言提供机制而非保护
  ```yaoxiang
  # ✅ 正确：通过变量传递副作用依赖
  db: DB = connect_database()      # 获取连接
  db.insert(user1)                 # 依赖 db 变量
  db.insert(user2)                 # 依赖 db 变量
  # 两个 insert 都读取 db，DAG 保证顺序执行

  # ⚠️ 用户责任：不用变量传递
  insert_to_db("data1")            # 无变量依赖
  insert_to_db("data2")            # 无变量依赖
  # 并行执行可能导致数据库状态不确定

  # 文档引导：向用户说明如何正确构建数据流
  # - 使用变量传递资源句柄
  # - 让 DAG 自动管理执行顺序
  # - 避免字面量直接使用同一资源
  ```


#### 低优先级问题

- [x] **错误图内存占用** ✅ 已解决
  - 解决方案：DAG 只在单个函数内构建，不会深度构建函数内的函数调用

#### 与 RFC-008 的交互问题

- [x] **问题1：三层运行时如何选择？** ✅ 已解决
  - 解决方案：使用泛型 + 编译时注入
  ```yaoxiang
  # 通过泛型声明运行时，可在运行时切换
  # 声明了标准式，可在标准/嵌入式之间切换
  # 未声明完整式，无法切换到完整
  main: [R: Runtime] => (RuntimeContext[R]) -> Void = (ctx) => {
      ...
  }
  ```

- [x] **问题2：DAG 节点与运行时的接口是什么？** ✅ 已解决
  - 解决方案：使用泛型 + 函数注入（自举友好），不使用 trait
  ```yaoxiang
  # 函数式接口设计
  dag_builder: (DAG, Node) -> Node
  scheduler: (DAG, RuntimeContext) -> ExecutionResult
  executor: (Node, RuntimeContext) -> Result[Value, Error]
  ```

#### 已删除问题（不适用）

| 问题 | 删除原因 |
|------|----------|
| L3 透明模式回退机制缺失 | 不需要降级，通过 `@eager`、`@auto` 可强制切换运行模式 |
| 嵌套 spawn 依赖分析复杂度 | spawn 仅作为传统异步回退方案，标准运行时无此问题 |
| @ordered 性能开销 | 删除 `@ordered`，使用 `@eager` 替代（语义一致） |

#### 实现优先级建议

根据 MVP 路线（P1-P4 → P8 → P11），建议实现顺序：

- [ ] **第一阶段（P1-P4）**：编译前端
  - [ ] 实现 L1 `@block` 注解
  - [ ] 实现基础的 spawn 语法解析

- [ ] **第二阶段（P8-P11）**：标准运行时
  - [ ] 实现 DAG Builder（P9）
  - [ ] 实现 Scheduler（P10）
  - [ ] 实现 VM 解释器（P11）

- [ ] **第三阶段（P13-P14）**：Full 运行时
  - [ ] 实现 Work Stealing（P13）
  - [ ] 实现 @eager（P14）

---

## 附录

### 附录A：@block注解位置讨论

> **讨论状态**: 开放中
> **发起者**: @晨煦
> **日期**: 2025-01-05

#### 问题描述

`@block`注解放在函数签名的哪个位置？需要平衡**可读性**和**一致性**。

#### 备选方案

| 方案 | 语法示例 | 优点 | 缺点 |
|------|----------|------|------|
| **A（当前）** | `() -> Void @block` | 与`spawn`位置一致 | 类型后紧跟注解，稍显拥挤 |
| **B** | `@block () -> Void` | 注解醒目、一眼可见 | 与`spawn`不一致 |
| **C** | `() @block -> Void` | 视觉分割清晰 | 破坏类型语法结构 |
| **D** | `fn @block name() -> Void { }` | 关键字开头 | 关键字+注解组合冗长 |

#### 各方案分析

**方案A（推荐）**：
- 与`spawn`保持同一位置，降低学习成本
- 可通过IDE高亮和代码风格规范弥补可读性

**方案B**：
- 注解更醒目，但与`spawn`位置不一致

**方案C**：
- 破坏`参数列表 -> 返回类型`的结构完整性

**方案D**：
- 冗长，与YaoXiang简洁语法风格不符

#### 推荐代码风格（缓解方案A的可读性问题）

```yaoxiang
# 将注解放在单独一行
main:
    () -> Void @block
= () => {
    ...
}
```

#### 社区讨论记录

> **讨论日期**: 2025-01-06
> **参与者**: @晨煦

##### 讨论议题 1：L3 透明模式回退机制

**问题**：L3 透明模式回退机制缺失，用户无法从 L3 降级到 L2/L1

**决策**：不需要降级机制，通过 `@eager`、`@auto` 等语法注解可强制某个函数作用域切换运行模式。

##### 讨论议题 2：嵌套 spawn 依赖分析复杂度

**问题**：嵌套 spawn 依赖分析可能导致 O(n²) 复杂度

**决策**：spawn 仅作为强制并发与异步的回退方案。在标准运行时中，所有函数默认惰性求值，不存在 spawn 解析问题。

##### 讨论议题 3：DAG 构建与执行顺序冲突

**问题**：错误传播可能与并行执行顺序不一致

**决策**：错误沿依赖边向上游传播，不考虑执行顺序，确保与错误图报错结合，可追踪错误引发的函数。

##### 讨论议题 4：DAG 构建时间复杂度

**问题**：大型项目编译缓慢

**决策**：使用增量构建 + 缓存机制优化编译性能。

##### 讨论议题 5：@ordered 与 @eager 语义统一

**问题**：@ordered 性能开销问题

**决策**：删除 `@ordered`，使用 `@eager` 替代（语义一致）。@eager 既是同步执行保证，也是结果顺序收集的保证。

##### 讨论议题 6：三层运行时选择机制

**问题**：如何在三种运行时之间选择

**决策**：使用泛型 + 编译时注入。通过泛型声明运行时，可在运行时切换：
- 声明了标准式，可在标准/嵌入式之间切换
- 未声明完整式，无法切换到完整

##### 讨论议题 7：DAG 节点与运行时接口

**问题**：节点与运行时的接口设计

**决策**：使用泛型 + 函数注入（自举友好），不使用 trait：
```yaoxiang
dag_builder: (DAG, Node) -> Node
scheduler: (DAG, RuntimeContext) -> ExecutionResult
executor: (Node, RuntimeContext) -> Result[Value, Error]
```

##### 讨论议题 8：错误图内存占用

**问题**：大型 DAG 可能产生大量错误节点

**决策**：DAG 只在单个函数内构建，不会深度构建函数内的函数调用，避免极端情况。

##### 待决策议题

| 议题 | 状态 | 说明 |
|------|------|------|
| 资源冲突检测规则 | ✅ 已解决 | DAG 数据流依赖，用户通过变量传递资源标识 |
| Send/Sync 约束检查 | 待考虑 | 信任开发者 + 运行时断言 |
| 副作用一致性保证 | ✅ 已解决 | 不引入副作用系统，用户通过变量传递构建 DAG |

#### 决议

> **状态**: ✅ 已决定
> **日期**: 2025-01-05

采用 **方案A**：注解放在返回类型后，与 `spawn` 位置一致。

理由：
1. 与 `spawn` 保持同一位置，降低学习成本
2. 可通过 IDE 高亮和代码风格规范弥补可读性

#### 讨论：@ordered 在循环中的语义（已删除）

> **状态**: ❌ 已删除
> **日期**: 2025-01-06

**问题**：`@ordered` 在循环中的语义是什么？

**决策**：删除 `@ordered`，使用 `@eager` 替代。

```yaoxiang
# @ordered 已删除，使用 @eager 替代
# @eager 既是同步执行保证，也是结果顺序收集的保证
@eager
results = spawn for i in 0..100 {
    heavy_calc(i)  # 顺序执行
}
# results 按原始顺序 [0, 1, 2, ..., 99] 收集
```

**理由**：
1. `@ordered` 与 `@eager` 语义一致，无需重复设计
2. 减少语法复杂度，统一使用 `@eager`
3. `@eager` 同时保证同步执行和结果顺序收集

---

### 附录B：设计决策记录

> 本附录记录RFC中已确定的设计决策及其理由。

| 决策 | 决定 | 日期 | 记录人 |
|------|------|------|--------|
| 采用三层并发架构 | L1/L2/L3 渐进式 | 2025-01-05 | @晨煦 |
| @block与spawn位置一致 | 方案A | 2025-01-05 | @晨煦 |
| @ordered 循环语义 | 已删除，用 @eager 替代 | 2025-01-06 | @晨煦 |
| DAG 错误传播 | 沿依赖边向上游传播 | 2025-01-06 | @晨煦 |
| DAG 性能优化 | 增量构建 + 缓存 | 2025-01-06 | @晨煦 |
| 运行时选择机制 | 泛型 + 编译时注入 | 2025-01-06 | @晨煦 |
| 节点接口设计 | 泛型 + 函数注入（无 trait） | 2025-01-06 | @晨煦 |
| 错误图内存优化 | DAG 仅在单函数内构建 | 2025-01-06 | @晨煦 |
| 静态 DAG 优化 | 编译期预构建 DAG 结构 | 2025-01-06 | @晨煦 |
| L1 自动回退 | 小函数自动回退到 @block | 2025-01-06 | @晨煦 |
| 资源冲突检测 | DAG 数据流依赖，用户变量传递 | 2025-01-06 | @晨煦 |
| 副作用一致性 | 资源类型统一管理，自动 DAG 构建 | 2025-01-06 | @晨煦 |
| Void 急切规则 | 已退役（v0.6） | 2026-01-06 | @晨煦 |
| 资源类型系统 | Resource 标记 + DAG 自动依赖分析 | 2026-01-06 | @晨煦 |
| 资源操作识别 | 类型约束自动识别 | 2026-01-06 | @晨煦 |
| 运行时资源处理 | 变量传递 + 数据流分析 | 2026-01-06 | @晨煦 |
| L1/L2/L3 心智模型 | 三层抽象帮助用户理解，非实现机制 | 2026-01-06 | @晨煦 |
| Send/Sync 约束检查 | 编译时强制检查 | 2026-01-06 | @GitHub Copilot |

---

### 附录C：术语表

| 术语 | 定义 |
|------|------|
| 并作模型 | YaoXiang的并发范式，同步语法、异步本质 |
| DAG | 有向无环图，描述计算依赖关系 |
| spawn | 并作函数标记，表示可并发执行 |
| @block | 同步注解，禁用并发优化（无 DAG，无调度器参与） |
| @eager | 急切求值注解，强制等待依赖完成（调度器参与，但限制并行度） |
| @auto | 自动并行注解，默认行为（调度器完整参与，最大化并行） |
| Resource | 资源类型标记，用于副作用抽象 |
| 资源类型 | 标记为 Resource 的类型，其操作自动构建 DAG 依赖 |
| 错误图 | 可视化的错误传播路径 |
| RuntimeContext[R] | 运行时上下文泛型，用于注入不同运行时 |

## 参考文献

- [Rust async book](https://rust-lang.github.io/async-book/)
- [Go并发模式](https://golang.org/doc/effective_go#concurrency)
- [工作窃取调度](https://en.wikipedia.org/wiki/Work_stealing)
- [《易经》并作思想](https://en.wikipedia.org/wiki/I_Ching)
- [并作模型白皮书](../async-whitepaper.md)
- [YaoXiang 指南](../YaoXiang-book.md)
- [YaoXiang 语言规范](../language-spec.md)
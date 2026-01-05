# **《并作：基于惰性求值的无感异步并发模型》技术白皮书**

## 🏛️ 一、核心定义：并作模型

**并作模型**，取意《易·复卦》"万物并作，吾以观复"。它是一种编程语言并发范式，允许开发者以同步、顺序的思维描述逻辑，而语言运行时令其中的计算单元如万物并作般自动、高效地并发执行，并在最终统一协同。

### 核心设计理念：默认惰性 + spawn 类型标记

| 设计原则 | 说明 |
|----------|------|
| **默认惰性求值** | 所有函数默认惰性（类似 Haskell），返回 Lazy[T] |
| **核心数量配置** | 脚本头声明 `// @cores: N` 自动启用并行化 |
| **spawn 类型标记** | `-> T spawn` 标记函数为严格可异步与并发的，其余默认可并发 |
| **混合求值模式** | `@eager`（装饰器，强制急切）、`@lazy`（装饰器，保持惰性） |
| **Void 自动急切** | 返回 `Void` 的函数自动急切求值（副作用必须执行） |

### 核心三原则

| 核心原则 | 阐释 |
|----------|------|
| **同步语法** | 所见即所得的顺序代码，所写即所得的执行流程 |
| **并发本质** | 运行时自动提取并行性，挖掘数据依赖中的并发机会 |
| **统一协同** | 结果在需要时自动汇聚，保证逻辑正确性 |

**它通过两个根本性转变达成此目标：**

1. **将"控制流"转化为"数据流"**：程序被视作一个纯函数式的惰性求值数据流图
2. **将"异步传染"转化为"依赖解析"**：异步性不再是函数签名的效应，而成为运行时在数据依赖点上自动执行的等待操作

---

## 📚 二、术语体系：统一的概念地图

围绕"并作"，我们构建了一套清晰、自洽的术语体系，将所有设计串联起来：

| 官方术语 | 对应语法/概念 | 阐释 |
|----------|---------------|------|
| **并作函数** | `-> T spawn` | 返回类型标记，表示这是一个严格可参与"并作"并发执行的计算单元 |
| **并作块** | `spawn { a(), b() }` | 开发者显式声明的并发疆域，块内任务将"并作"执行 |
| **并作循环** | `spawn for x in xs { ... }` | 数据并行范式，循环体在所有数据元素上"并作"执行 |
| **并作值** | `Async[T]` 代理类型 | 一个正在并作中的"未来值"，在使用时自动等待其"作"完 |
| **并作图** | 惰性计算图（DAG） | "并作"发生的舞台，描述了所有计算单元间的依赖与并行关系 |
| **并作调度器** | 运行时任务调度器 | 负责协调"万物"，让它们在正确时机"并作"的智能中枢 |

> **技术交流示例**："这里我们用个并作块来并发调用两个并作函数，就能自动获得它们的并作值。"

---

## 二、核心概念

### 2.1 并作图：万物并作的舞台

所有程序在编译时被转化为一个**有向无环计算图（DAG）**，我们称之为**并作图**。

| 元素 | 说明 |
|------|------|
| **节点** | 代表表达式计算单元 |
| **边** | 代表数据依赖关系（A → B 表示 B 依赖 A 的结果） |
| **惰性** | 节点仅在其输出被**真正需要**时才被求值 |

### 2.2 默认惰性求值

所有函数默认采用**惰性求值**策略：

```yaoxiang
# 脚本头配置并行核心数
# @cores: 4

# 所有函数默认惰性求值（默认可并发）
heavy_computation: (Int) -> Int = (x) => {
    # 这个函数不会立即执行
    # 只有当结果被使用时才执行
    fibonacci(x)
}

main: () -> Void = () => {
    # heavy_computation 返回 Int，类型是 Lazy[Int]
    result = heavy_computation(100)

    # 在这里，result 被用于加法，触发求值
    # 系统自动找到最佳时机并行执行
    total = result + heavy_computation(200)
}
```

### 2.3 混合求值注解（装饰器风格）

YaoXiang 的注解类似于 Python 的装饰器（decorator），用于修改函数或表达式的行为：

| 注解（装饰器） | 行为 |
|----------------|------|
| `@eager` | **装饰器**：强制急切求值，立即执行 |
| `@lazy` | **装饰器**：保持惰性（默认，可省略） |

**Void 自动急切规则：** 返回 `Void` 的函数自动急切求值（无需任何注解），因为副作用必须执行。

```yaoxiang
# @eager 装饰器：强制急切求值
heavy_computation: (Int) -> Int = (x) => {
    fibonacci(x)
}

# 返回 Void 的函数自动急切求值（副作用函数）
log: (String) -> Void = (message) => {
    print(message)
}

main: () -> Void = () => {
    # log 自动急切执行，因为返回 Void
    log("Processing started")

    # 使用 @eager 强制急切
    @eager heavy_computation(100)
}
```

### 2.4 并作值：Async[T] 惰性代理类型

任何返回类型标记为 `-> T spawn` 的函数会立即返回一个 `Async[T]` 类型的值，我们称之为**并作值**。

```yaoxiang
# 并作函数：返回类型标记为 -> JSON spawn
# 表明这是一个严格可并作执行的计算单元
fetch: (String) -> JSON spawn = (url) => {
    HTTP.get(url).json()
}

main: () -> Void = () => {
    # fetch 返回的是并作值 Async[JSON]
    # 但使用时不需任何额外语法
    data = fetch("https://api.example.com")  # Async[JSON]

    # 在这里，data 自动等待并解包为 JSON
    print(data.name)  # 如同步代码般自然
}
```

#### 并作值的核心特性

| 特性 | 说明 |
|------|------|
| **语法透明** | `Async[T]` 在类型系统中是 `T` 的子类型，可在任何期望 `T` 的上下文中使用 |
| **按需等待** | 当必须使用 `T` 类型具体值时（如字段访问、算术运算），运行时自动挂起并等待 |
| **错误传播** | 内部实际为 `Result<T, E>`，错误沿数据流自然传播 |

### 2.7 并作构造：从"修饰符"到"类型标记"

`spawn` 关键字是连接同步思维与异步实现的唯一桥梁，具有三重语义：

| 语法形式 | 官方术语 | 语义 | 运行时行为 |
|:---------|:---------|:-----|:----------|
| **`-> T spawn`** | 并作函数 | 返回类型标记，表明这是一个严格可参与并作的计算单元 | 其调用返回 `Async[T]`，标志着一个并作图节点的创建 |
| **`spawn { ... }`** | 并作块 | 显式声明的并发疆域 | 运行时**激进地**将块内各表达式作为独立任务并发执行，并在块结束时隐式等待所有结果 |
| **`spawn for`** | 并作循环 | 数据并行循环 | 将循环体转化为多个并行任务，自动进行数据分片、调度和结果收集 |

---

## 三、工作原理：从代码到执行

### 3.1 编译时：构建并作图

```yaoxiang
# 并作函数定义：返回类型标记为 spawn
fetch: (String) -> String spawn = (url) => { ... }
parse: (String) -> Model spawn = (data) => { ... }

process: () -> Report = () => {
    # 编译器在此创建并作图节点
    data_a = fetch("url1")  # 节点 A: Async[String]
    data_b = fetch("url2")  # 节点 B: Async[String]

    # 并作块：显式并发疆域
    (model_a, model_b) = spawn {
        parse(data_a),  # 节点 C: 依赖 A
        parse(data_b)   # 节点 D: 依赖 B
    }

    # 最终汇入节点
    generate_report(model_a, model_b)  # 节点 E
}
```

**编译器操作：**
1. 解析源代码，构建全局并作图
2. 为每个表达式创建计算节点
3. 分析数据依赖，建立边关系
4. `spawn { }` 和 `spawn for` 块内的子图被打上 **"并行求值"** 标记

### 4.2 运行时：并作调度器

一个智能的、支持工作窃取的**并作调度器**负责执行并作图：

```rust
// 并作调度器核心逻辑
impl FlowScheduler {
    fn execute_node(&self, node_id: NodeId) {
        let node = self.get_node(node_id);
        
        match &node.kind {
            NodeKind::AsyncCompute => {
                // 并作函数：提交到协程池
                self.submit_async(node_id);
            }
            NodeKind::ParallelBlock => {
                // 并作块：激进并行执行所有直接子节点
                self.submit_parallel(node_id);
            }
            NodeKind::DataParallel { iterator, body } => {
                // 并作循环：自动分片
                self.submit_data_parallel(node_id, iterator, body);
            }
            _ => { /* 同步执行 */ }
        }
    }
}
```

#### 执行流程

```
1. 为求值 [E]，需要 [C] 和 [D]
2. [C] 依赖 [A]，[D] 依赖 [B]
3. 并作调度器发现 [A] 与 [B] 无依赖 → 立即并行执行
4. [A]、[B] 完成后，由于并作块标记 → 立即并行执行 [C] 和 [D]
5. [C]、[D] 完成后，执行 [E]
```

**关键机制：**

| 机制 | 说明 |
|------|------|
| **惰性触发** | 执行从请求最终结果开始，反向追踪依赖 |
| **自动等待** | 遇到 `Async[T]` 时自动挂起，执行其他就绪任务 |
| **工作窃取** | 线程从其他线程队列窃取任务，提高 CPU 利用率 |

---

## 四、关键机制详解

### 4.1 副作用与求值保证

纯粹的惰性求值可能导致副作用（如日志、写入）永不执行。并作模型采用**基于返回类型的自动推导**：

| 规则 | 条件 | 行为 |
|------|------|------|
| **规则一** | 返回 `Void` 的函数 | **自动急切求值**（副作用必须执行） |
| **规则二** | 使用 `@eager` 装饰器的表达式 | 无论返回类型如何，都**强制急切求值** |
| **规则三** | 返回非 Void 类型 | **惰性求值**（默认） |

```yaoxiang
# 返回 Void 的函数自动急切执行（副作用）
log: (String) -> Void = (message) => {
    print(message)
}

# @eager 装饰器：强制急切求值
cache_compute: (Int) -> Int = (x) => {
    # 即使返回 Int，也强制立即执行
    expensive_calculation(x)
}

main: () -> Void = () => {
    # log 自动急切执行（返回 Void）
    log("Processing started")

    # @eager 强制急切执行
    @eager
    cache_compute(100)

    # 普通函数惰性执行（返回 Int）
    result = heavy_computation(200)  # 此时不执行
    print(result)  # 在这里才执行
}
```

### 4.2 错误处理

错误沿数据流自然传播，如同同步代码般直观：

```yaoxiang
might_fail: () -> Result[Data, Error] spawn = () => { ... }

main: () -> Void = () => {
    # 并作值内部实际为 Result
    data = might_fail()  # Async[Result[Data, Error]]

    # 等待点：若底层计算失败，异常在此抛出
    processed = data.transform()  # 如同步代码般处理错误
}
```

### 4.3 纯函数与 @block 同步保证

**核心洞察：纯函数不会阻塞！**

因为：
- 纯函数无 I/O，只有 CPU 计算
- 计算再久也不阻塞调度器，只占用 CPU 时间

**执行策略：**

| 函数类型 | 执行策略 | 阻塞？ |
|----------|----------|--------|
| 纯函数（无 I/O） | 同步执行 | 否（仅 CPU 占用） |
| 异步函数（返回 `Async[T]`） | 异步执行 | 否 |
| `@block` 注解函数 | 同步执行，内部调度 | 否 |

**@block 注解：同步执行保证**

`@block` 注解保证函数以同步姿态执行：
- 函数返回时结果已准备好
- 内部如有异步调用，在内部完成调度
- 适合需要同步语义但内部可能包含异步操作的场景

```yaoxiang
# @block：同步执行，内部异步调度完成后再返回
heavy_compute: (List[Int]) -> Int = (data) => {
    # 内部可能有异步操作，但在返回前完成
    processed = data.map(x => async_transform(x))
    processed.sum()
}

# 普通异步函数：返回 Async[T]
fetch_user: (Int) -> Async[User] = (id) => {
    database.query("SELECT * FROM users WHERE id = ?", id)
}

# 纯函数：自动同步（无 I/O）
factorial: (Int) -> Int = (n) => {
    if n <= 1 then 1 else n * factorial(n - 1)
}

main: () -> Void = () => {
    # @block 函数：同步执行
    result = heavy_compute([1, 2, 3, 4, 5])  # 立即返回结果
    print(result)  # 15

    # 异步函数：返回 Async[User]
    user = fetch_user(123)  # Async[User]
    print(user.name)  # 自动等待并解包
}
```

**运行时策略：**

```rust
fn execute_function(node: &DAGNode) {
    match node.execution_mode {
        ExecutionMode::Pure => {
            // 纯函数：同步执行
            node.execute();
        }
        ExecutionMode::Async => {
            // 异步函数：提交到 async 调度器
            async_runtime.submit(node);
        }
        ExecutionMode::Blocking => {
            // @block 函数：同步执行，内部调度异步操作
            execute_blocking(node);
        }
    }
}

fn execute_blocking(node: &DAGNode) {
    // 执行函数体
    let result = node.execute_body();
    
    // 收集内部所有异步操作
    let internal_async_ops = collect_async_ops(node);
    
    // 等待所有内部异步操作完成
    if !internal_async_ops.is_empty() {
        async_runtime.wait_all(internal_async_ops);
    }
    
    // 返回结果
    result
}
```

**设计优势：**
- **简洁**：无需复杂的 effect 系统
- **灵活**：`@block` 可选，需要同步语义时使用
- **高效**：纯函数自动同步执行
- **安全**：主调度器永远不阻塞

### 4.4 并行竞争控制：类型系统保证原子性

**核心思想：用类型系统标记并发访问的数据，编译器检查同步正确性。**

**类型标记体系：**

| 类型 | 语义 | 并发安全 | 说明 |
|------|------|----------|------|
| `T` | 不可变数据 | ✅ 安全 | 默认类型，多任务读取无竞争 |
| `Ref[T]` | 可变引用 | ⚠️ 需同步 | 标记为可并发修改，编译检查锁使用 |
| `Atomic[T]` | 原子类型 | ✅ 安全 | 底层原子操作，无锁并发 |
| `Mutex[T]` | 互斥锁包装 | ✅ 安全 | 自动加锁解锁，编译保证 |
| `RwLock[T]` | 读写锁包装 | ✅ 安全 | 读多写少场景优化 |

**类型安全性保证：**

```yaoxiang
# 默认不可变 - 天然无竞争
data: List[Int] = [1, 2, 3, 4, 5]
spawn for x in data { process(x) }  # ✅ 安全，只读无竞争

# 可变引用 - 需要同步
counter: Ref[Int] = Ref.new(0)

# 错误示例：未加锁访问 Ref（编译错误）
spawn for i in 1..10 {
    # ❌ 编译错误：Ref 必须通过同步原语访问
    counter.value = counter.value + i
}

# 正确示例：使用 with 语法糖自动加锁
spawn for i in 1..10 {
    # ✅ with 块自动获取和释放锁
    with counter.lock() {
        counter.value = counter.value + i
    }
}

# 原子类型 - 无锁并发
atomic_counter: Atomic[Int] = Atomic.new(0)
spawn for i in 1..10 {
    # ✅ 原子操作，无锁安全
    atomic_counter.fetch_add(i)
}
```

**Mutex[T] 类型 - 编译期锁保证：**

```yaoxiang
# 创建互斥锁包装的数据
shared_state: Mutex[Map[String, Int]] = Mutex.new(Map.empty())

# 使用 with 语法糖（类似 Go 的 defer）
main: () -> Void = () => {
    spawn for i in 1..100 {
        # with 自动获取锁，块结束后自动释放
        with shared_state.lock() {
            # 临界区：受 Mutex 保护
            current = shared_state.get("count").or(0)
            shared_state.set("count", current + 1)
        }
    }

    # 等待所有任务完成
    print(shared_state.get("count"))  # 100
}
```

**类型推断与锁检查：**

```rust
// 编译器在编译时检查
fn compile_check_locks(func: &Function) {
    for node in func.nodes {
        match node {
            NodeKind::ReadRef(ref_var) => {
                // 检查是否在锁的保护范围内
                if !is_inside_lock_guard(ref_var) {
                    compile_error!("Ref 访问必须在 lock() 保护范围内");
                }
            }
            NodeKind::WriteRef(ref_var, _) => {
                // 双重检查：锁 + 唯一写入者
                if !is_inside_lock_guard(ref_var) {
                    compile_error!("Ref 修改必须在 lock() 保护范围内");
                }
                if has_multiple_writers(func, ref_var) {
                    compile_error!("Mutex[T] 只能有一个写入者，需用 RwLock[T]");
                }
            }
            _ => {}
        }
    }
}
```

**设计优势：**

| 优势 | 说明 |
|------|------|
| **编译期检查** | 锁遗漏在编译期捕获，而非运行时死锁 |
| **零运行时开销** | Mutex 包装在无竞争时无开销 |
| **语法简洁** | `with lock() { ... }` 语法糖，自动管理生命周期 |
| **类型安全** | 误用 Ref 而非 Atomic 会在类型层面报错 |

---

## 五、优势总结

| 优势 | 说明 |
|------|------|
| **零传染性** | 异步代码与同步代码在语法和类型签名上无区别，彻底根除"async/await"传染 |
| **高性能并行** | 惰性并作图与显式 `spawn` 标记相结合，既允许运行时自动发掘并行性，也赋予程序员进行极限性能优化的明确工具 |
| **心智模型简单** | 开发者只需关注数据流向和业务逻辑，无需理解复杂的并发原语和回调 |
| **易于重构** | 在顺序逻辑和并发逻辑之间切换成本极低，仅需增减 `spawn {}` 包装 |
| **术语直观** | "并作函数"、"并作块"、"并作值"让技术讨论变得极其直观 |

---

## 六、实现考量

### 6.1 编译器

- [ ] 实现数据流分析，构建并作图
- [ ] 实现 `spawn` 返回类型标记的解析和类型推断
- [ ] 将 `spawn {}` 和 `spawn for` 脱糖为运行时并行原语
- [ ] 支持装饰器风格的注解（`@eager`、`@lazy`）
- [ ] 实现 Void 返回类型自动急切求值逻辑

### 6.2 运行时

- [ ] 实现支持工作窃取的并作调度器
- [ ] 实现计算图依赖感知的任务调度
- [ ] 实现 `Async[T]` 类型的自动解包机制
- [ ] 实现 Void 函数的自动急切执行

### 6.3 调试工具 ⚠️ 必须

**计算图可视化调试器**是理解复杂程序行为的关键：

| 功能 | 说明 |
|------|------|
| **节点状态可视化** | 观察每个计算节点的 Pending/Running/Completed 状态 |
| **依赖关系展示** | 显示节点间的数据依赖边 |
| **任务流动追踪** | 观察任务在各个线程间的流转 |
| **性能瓶颈定位** | 识别长链路和热点节点 |

---

## 七、代码示例

### 7.1 基础并作函数

```yaoxiang
use std.net

# 并作函数定义：返回类型标记为 spawn
fetch_user: (Int) -> User spawn = (id) => {
    response = net.HTTP.get("/users/" + id.to_string())
    response.json()
}

fetch_posts: (Int) -> List[Post] spawn = (user_id) => {
    response = net.HTTP.get("/users/" + user_id.to_string() + "/posts")
    response.json()
}

main: () -> Void = () => {
    # 自动并行执行（无依赖）
    user = fetch_user(123)      # Async[User]
    posts = fetch_posts(123)    # Async[List[Post]]

    # 在这里自动等待并解包
    print(user.name)            # 如同步代码般自然
    print(posts.length)
}
```

### 7.2 并作块

```yaoxiang
fetch: (String) -> JSON spawn = (url) => { ... }
parse: (JSON) -> Model spawn = (json) => { ... }

parallel_fetch: () -> (Model, Model) = () => {
    # 并作块：显式并发疆域
    (model_a, model_b) = spawn {
        parse(fetch("https://api1.com/data")),
        parse(fetch("https://api2.com/data"))
    }
    # 模型 a 和 b 在这里都已就绪
    (model_a, model_b)
}
```

### 7.3 并作循环

```yaoxiang
process_item: (Item) -> Result[Processed, Error] spawn = (item) => { ... }

batch_process: (List[Item]) -> List[Result[Processed, Error]] = (items) => {
    # 并作循环：数据并行
    results = [spawn for item in items {
        process_item(item)
    }]
    # results 在这里是一个 List，包含所有处理结果
    results
}
```

---

> *"万物并作，吾以观复。"*
> —— 《易·复卦》
>
> 并作模型将惰性求值的声明式优雅与高性能并发的要求相结合，旨在为系统编程提供一种既安全又极具表达力的全新范式。

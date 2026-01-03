# **《并作：基于惰性求值的无感异步并发模型》技术白皮书**

## 🏛️ 一、核心定义：并作模型

**并作模型**，取意《易·复卦》"万物并作，吾以观复"。它是一种编程语言并发范式，允许开发者以同步、顺序的思维描述逻辑，而语言运行时令其中的计算单元如万物并作般自动、高效地并发执行，并在最终统一协同。

###

| 核心三原则 原则 | 阐释 |
|------|------|
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
| **并作函数** | `spawn fn` | 定义了一个可参与"并作"并发执行的计算单元 |
| **并作块** | `spawn { a(), b() }` | 开发者显式声明的并发疆域，块内任务将"并作"执行 |
| **并作循环** | `spawn for x in xs { ... }` | 数据并行范式，循环体在所有数据元素上"并作"执行 |
| **并作值** | `Async[T]` 代理类型 | 一个正在并作中的"未来值"，在使用时自动等待其"作"完 |
| **并作图** | 惰性计算图（DAG） | "并作"发生的舞台，描述了所有计算单元间的依赖与并行关系 |
| **并作调度器** | 运行时任务调度器 | 负责协调"万物"，让它们在正确时机"并作"的智能中枢 |

> **技术交流示例**："这里我们用个并作块来并发调用两个并作函数，就能自动获得它们的并作值。"

---

## 三、核心概念

### 3.1 并作图：万物并作的舞台

所有程序在编译时被转化为一个**有向无环计算图（DAG）**，我们称之为**并作图**。

| 元素 | 说明 |
|------|------|
| **节点** | 代表表达式计算单元 |
| **边** | 代表数据依赖关系（A → B 表示 B 依赖 A 的结果） |
| **惰性** | 节点仅在其输出被**真正需要**时才被求值 |

```
        [A: fetch]       [B: fetch]      ← 并作图节点
           │                │
           ▼                ▼
        [C: parse]       [D: parse]      ← 依赖边
            \                /
             ▼              ▼
            [E: generate_report]         ← 汇入最终结果
```

### 3.2 并作值：Async[T] 惰性代理类型

任何标记为 `spawn fn` 的函数调用会立即返回一个 `Async[T]` 类型的值，我们称之为**并作值**。

```yaoxiang
# 并作函数：定义了一个可并作执行的计算单元
spawn fn fetch(url: String) -> JSON = (url) => {
    HTTP.get(url).json()
}

fn main() -> Void = () => {
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

### 3.3 并作构造：从"修饰符"到"构造器"

`spawn` 关键字是连接同步思维与异步实现的唯一桥梁，具有三重语义：

| 语法形式 | 官方术语 | 语义 | 运行时行为 |
|:---------|:---------|:-----|:----------|
| **`spawn fn`** | 并作函数 | 定义一个可参与并作的计算单元 | 其调用返回 `Async[T]`，标志着一个并作图节点的创建 |
| **`spawn { ... }`** | 并作块 | 显式声明的并发疆域 | 运行时**激进地**将块内各表达式作为独立任务并发执行，并在块结束时隐式等待所有结果 |
| **`spawn for`** | 并作循环 | 数据并行循环 | 将循环体转化为多个并行任务，自动进行数据分片、调度和结果收集 |

---

## 四、工作原理：从代码到执行

### 4.1 编译时：构建并作图

```yaoxiang
# 并作函数定义
spawn fn fetch(url: String) -> String = (url) => { ... }
spawn fn parse(data: String) -> Model = (data) => { ... }

fn process() -> Report = () => {
    # 编译器在此创建并作图节点
    let data_a = fetch("url1")  # 节点 A: Async[String]
    let data_b = fetch("url2")  # 节点 B: Async[String]
    
    # 并作块：显式并发疆域
    let (model_a, model_b) = spawn {
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

## 五、关键机制详解

### 5.1 副作用与求值保证

纯粹的惰性求值可能导致副作用（如日志、写入）永不执行。并作模型引入**求值保证规则**：

| 规则 | 条件 | 行为 |
|------|------|------|
| **规则一** | 不返回值的函数调用（`Unit` 类型） | 默认为包含副作用，**急切求值** |
| **规则二** | 使用 `@eager` 注解的表达式 | 无论是否有返回值，都**急切求值** |

```yaoxiang
# 急切求值示例
@eager log("Processing started")  # 确保立即执行

fn main() -> Void = () => {
    @eager side_effect()  # 即使返回 Unit，也立即执行
}
```

### 5.2 错误处理

错误沿数据流自然传播，如同同步代码般直观：

```yaoxiang
spawn fn might_fail() -> Result[Data, Error] = () => { ... }

fn main() -> Void = () => {
    # 并作值内部实际为 Result
    let data = might_fail()  # Async[Result[Data, Error]]
    
    # 等待点：若底层计算失败，异常在此抛出
    let processed = data.transform()  # 如同步代码般处理错误
}
```

### 5.3 与阻塞世界的交互

通过 `@blocking` 注解标记会阻塞操作系统线程的函数：

```yaoxiang
# 阻塞操作：提交到专用线程池
@blocking fn read_large_file(path: String) -> String = (path) => {
    std::fs.read_to_string(path)  # 可能阻塞的操作
}

fn main() -> Void = () => {
    # 不会阻塞并作调度器
    content = read_large_file("bigdata.txt")
}
```

**运行时分流：**

| 操作类型 | 执行池 | 原因 |
|----------|--------|------|
| 并作函数 | 协程调度器 | 非阻塞，可高效并发 |
| 并作循环 | 协程调度器 | 数据分片，任务并行 |
| `@blocking` | 阻塞线程池 | 隔离阻塞操作，保护调度器 |

---

## 六、优势总结

| 优势 | 说明 |
|------|------|
| **零传染性** | 异步代码与同步代码在语法和类型签名上无区别，彻底根除"async/await"传染 |
| **高性能并行** | 惰性并作图与显式 `spawn` 标记相结合，既允许运行时自动发掘并行性，也赋予程序员进行极限性能优化的明确工具 |
| **心智模型简单** | 开发者只需关注数据流向和业务逻辑，无需理解复杂的并发原语和回调 |
| **易于重构** | 在顺序逻辑和并发逻辑之间切换成本极低，仅需增减 `spawn {}` 包装 |
| **术语直观** | "并作函数"、"并作块"、"并作值"让技术讨论变得极其直观 |

---

## 七、实现考量

### 7.1 编译器

- [ ] 实现数据流分析，构建并作图
- [ ] 实现 `spawn` 语法的解析和类型推断
- [ ] 将 `spawn {}` 和 `spawn for` 脱糖为运行时并行原语
- [ ] 支持 `@eager` 和 `@blocking` 注解

### 7.2 运行时

- [ ] 实现支持工作窃取的并作调度器
- [ ] 实现计算图依赖感知的任务调度
- [ ] 实现 `Async[T]` 类型的自动解包机制
- [ ] 实现阻塞线程池隔离机制

### 7.3 调试工具 ⚠️ 必须

**计算图可视化调试器**是理解复杂程序行为的关键：

| 功能 | 说明 |
|------|------|
| **节点状态可视化** | 观察每个计算节点的 Pending/Running/Completed 状态 |
| **依赖关系展示** | 显示节点间的数据依赖边 |
| **任务流动追踪** | 观察任务在各个线程间的流转 |
| **性能瓶颈定位** | 识别长链路和热点节点 |

---

## 八、代码示例

### 8.1 基础并作函数

```yaoxiang
use std.net

# 并作函数定义
spawn fn fetch_user(id: Int) -> User = (id) => {
    response = net.HTTP.get("/users/" + id.to_string())
    response.json()
}

spawn fn fetch_posts(user_id: Int) -> List[Post] = (user_id) => {
    response = net.HTTP.get("/users/" + user_id.to_string() + "/posts")
    response.json()
}

fn main() -> Void = () => {
    # 自动并行执行（无依赖）
    user = fetch_user(123)      # Async[User]
    posts = fetch_posts(123)    # Async[List[Post]]
    
    # 在这里自动等待并解包
    print(user.name)            # 如同步代码般自然
    print(posts.length)
}
```

### 8.2 并作块

```yaoxiang
spawn fn fetch(url: String) -> JSON = (url) => { ... }
spawn fn parse(json: JSON) -> Model = (json) => { ... }

fn parallel_fetch() -> (Model, Model) = () => {
    # 并作块：显式并发疆域
    let (model_a, model_b) = spawn {
        parse(fetch("https://api1.com/data")),
        parse(fetch("https://api2.com/data"))
    }
    # 模型 a 和 b 在这里都已就绪
    (model_a, model_b)
}
```

### 8.3 并作循环

```yaoxiang
spawn fn process_item(item: Item) -> Result[Processed, Error] = (item) => { ... }

fn batch_process(items: List[Item]) -> List[Result[Processed, Error]] = () => {
    # 并作循环：数据并行
    let results = spawn for item in items {
        process_item(item)
    }
    # results 在这里是一个 List，包含所有处理结果
    results
}
```

---

> *"万物并作，吾以观复。"*
> —— 《易·复卦》
>
> 并作模型将惰性求值的声明式优雅与高性能并发的要求相结合，旨在为系统编程提供一种既安全又极具表达力的全新范式。

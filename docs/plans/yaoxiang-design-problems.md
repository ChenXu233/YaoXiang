# YaoXiang 语言设计问题清单与解决方案

> 创建日期：2025-01-02
> 状态：进行中
> 目标：解决"一切皆类型"及相关的设计问题

---

## 一、类型系统问题

### 1.1 类型开销问题

**问题描述**：
当前 `MonoType` 枚举直接存储所有类型信息，导致：
- 内存开销大（每个类型变体都有完整结构）
- 拷贝成本高（类型值拷贝时复制整个结构）
- 字符串重复存储（类型名称多次存储）

**当前实现**：
```rust
pub enum MonoType {
    Struct(StructType),  // 包含 name: String, fields: Vec<(String, MonoType)>
    Enum(EnumType),      // 包含 name: String, variants: Vec<String>
    Tuple(Vec<MonoType>),
    List(Box<MonoType>),
    // ...
}
```

**解决方案**：

| 方案 | 描述 | 优点 | 缺点 |
|------|------|------|------|
| **类型表** | 使用整数 ID 引用类型 | 省内存 | 需要间接查找 |
| **类型共享** | 相同类型复用同一实例 | 省内存 | 需要注册表 |
| **Arc 包装** | 使用 Arc<StructType> | 共享引用 | 运行时开销 |

**推荐方案**：混合策略
- 基本类型（Int, Float, String）直接存储
- 复合类型使用类型表 + 整数 ID
- 共享结构体使用 Arc

### 1.2 递归类型处理

**问题描述**：
自引用类型如链表无法直接表示：

```yaoxiang
type List[T] = nil | cons(T, List[T])
```

**当前状态**：
- 当前 `StructType` 和 `EnumType` 不支持递归
- 需要间接处理（如 Box 或引用）

**解决方案**：

```rust
// 方案1：使用 Box 延迟初始化
pub enum MonoType {
    // ...
    Recursive {
        type_id: usize,      // 类型标识符
        definition: Box<MonoType>,  // 类型定义
    },
    // ...
}

// 方案2：类型引用（指向已定义的类型）
pub enum MonoType {
    // ...
    TypeRef(String),  // 引用已定义的类型
    // ...
}
```

### 1.3 类型比较效率

**问题描述**：
运行时类型比较需要递归遍历所有字段，性能较低。

**当前实现**：
```rust
(MonoType::Struct(s1), MonoType::Struct(s2)) => {
    if s1.fields.len() != s2.fields.len() { return Err(...); }
    for ((_, f1), (_, f2)) in s1.fields.iter().zip(s2.fields.iter()) {
        self.unify(f1, f2)?;
    }
}
```

**优化方案**：

```rust
// 方案1：类型指纹
impl MonoType {
    pub fn fingerprint(&self) -> u64 {
        match self {
            MonoType::Int(n) => fingerprint_int(*n),
            MonoType::Struct(s) => {
                let mut fp = FINGERPRINT_STRUCT;
                for (name, ty) in &s.fields {
                    fp = fp.wrapping_mul(31)
                        .wrapping_add(name_fingerprint(name));
                    fp = fp.wrapping_mul(31)
                        .wrapping_add(ty.fingerprint());
                }
                fp
            }
            // ...
        }
    }
}

// 方案2：类型缓存
struct TypeCache {
    types: HashMap<MonoType, usize>,  // 类型 -> ID
    fingerprints: HashMap<u64, MonoType>,  // 指纹 -> 类型
}
```

### 1.4 类型反射能力

**问题描述**：
"一切皆类型"要求完整的运行时类型信息，但当前实现不完整。

**当前能力**：
- `type_name()` - 获取类型名称
- `is_numeric()`, `is_indexable()` - 基本检查

**缺失能力**：
- 字段列表访问
- 变体列表访问
- 类型层次查询
- 类型相等性深度比较

**解决方案**：

```rust
// 完整的类型反射接口
pub trait TypeReflection {
    fn type_name(&self) -> String;
    fn type_id(&self) -> usize;
    fn fields(&self) -> Option<Vec<(String, &MonoType)>>;
    fn variants(&self) -> Option<Vec<String>>;
    fn is_subtype_of(&self, other: &MonoType) -> bool;
    fn can_cast_to(&self, target: &MonoType) -> bool;
}
```

---

## 二、异步编程问题

### 2.1 spawn 调度问题

**设计决策**：
- spawn 函数阻塞直到调度
- 协作式调度（Yield）
- 编译器自动插入 await

**待解决问题**：
1. 任务队列的实现
2. 调度策略（优先级、FIFO、抢占）
3. 任务状态管理（pending, running, completed, failed）

**解决方案**：

```rust
// 任务状态
enum TaskState {
    Pending,
    Running,
    Completed,
    Failed(String),
}

// 任务
struct Task<T> {
    state: TaskState,
    result: Option<T>,
    receiver: oneshot::Sender<T>,
}

// 调度器
struct Scheduler {
    ready_queue: Vec<TaskId>,
    waiting_map: HashMap<TaskId, Task>,
    wakeup_tx: channel::Sender<TaskId>,
}
```

### 2.2 自动 await 实现

**设计决策**：
编译器自动分析依赖，插入 await 指令

**挑战**：
1. 依赖分析的正确性
2. 控制流复杂时的处理
3. 错误传播

**解决方案**：

```rust
// 编译器前端：依赖分析
fn analyze_dependencies(expr: &Expr) -> DependencyGraph {
    let mut graph = DependencyGraph::new();
    // 遍历表达式，识别 spawn 调用
    // 建立数据流图
    graph
}

// 代码生成：插入 await
fn insert_await(expr: &Expr, dependencies: &DependencyGraph) -> Expr {
    // 在使用 spawn 结果的地方插入 await
    // 处理并行调用的优化
    expr
}
```

---

## 三、线程安全问题

### 3.1 Send/Sync 约束实现

**设计决策**：
类似 Rust 的 Send/Sync 约束

**待解决问题**：
1. 类型约束的定义和检查
2. 复合类型的自动派生
3. 与 spawn 的集成

**实现方案**：

```rust
// 类型约束标记
trait Send {}
trait Sync {}

impl<T> Send for Arc<T> where T: Send + Sync {}
impl<T> Send for Mutex<T> where T: Send {}
impl<T> Sync for Mutex<T> where T: Send + Sync {}

// 类型检查器扩展
struct TypeChecker {
    // ...
    fn check_send(&self, ty: &MonoType) -> Result<(), TypeError> {
        // 递归检查所有字段
        // 检查是否为 Send 类型
    }
}
```

---

## 四、内存管理问题

### 4.1 借用检查实现

**设计决策**：
类似 Rust 的借用检查器

**待解决问题**：
1. 借用生命周期的追踪
2. 借用冲突检测
3. 错误消息质量

### 4.2 生命周期标注

**设计决策**：
支持显式生命周期标注 `'a`

**待解决问题**：
1. 生命周期省略规则
2. 生命周期推导
3. 生命周期错误消息

---

## 五、问题优先级

### 5.1 高优先级（核心问题）

| 问题 | 影响 | 解决难度 |
|------|------|----------|
| 类型开销 | 运行时性能 | 中 |
| 递归类型 | 语言表达能力 | 高 |
| Send/Sync | 线程安全 | 高 |
| spawn 调度 | 异步正确性 | 高 |

### 5.2 中优先级（重要问题）

| 问题 | 影响 | 解决难度 |
|------|------|----------|
| 类型比较 | 运行时性能 | 低 |
| 类型反射 | AI 友好性 | 中 |
| 自动 await | 异步易用性 | 中 |

### 5.3 低优先级（优化问题）

| 问题 | 影响 | 解决难度 |
|------|------|----------|
| 错误消息 | 用户体验 | 低 |
| 编译时间 | 开发效率 | 中 |

---

## 六、测试需求

### 6.1 类型系统测试

```rust
// 类型推断测试
#[test]
fn test_type_inference() { ... }

// 类型相等测试
#[test]
fn test_type_equality() { ... }

// 泛型特化测试
#[test]
fn test_generic_specialization() { ... }
```

### 6.2 异步测试

```rust
// spawn 测试
#[test]
fn test_spawn() { ... }

// 自动 await 测试
#[test]
fn test_auto_await() { ... }

// 并发测试
#[test]
fn test_concurrent() { ... }
```

### 6.3 线程安全测试

```rust
// Send 测试
#[test]
fn test_send() { ... }

// Sync 测试
#[test]
fn test_sync() { ... }

// 数据竞争检测
#[test]
fn test_race_condition() { ... }
```

---

## 七、参考实现

### 7.1 类型系统参考

- **Rust**：Hindley-Milner 类型推断
- **Idris**：依赖类型实现
- **TypeScript**：运行时类型反射

### 7.2 异步参考

- **Rust async**：async/await 实现
- **Kotlin**：协程调度器
- **Go**：goroutine 调度器

### 7.3 线程安全参考

- **Rust**：Send/Sync trait
- **Java**：synchronized 和 concurrent 包

---

## 八、下一步行动

1. 确定类型表示策略（类型表 vs 直接存储）
2. 实现递归类型支持
3. 设计 Send/Sync 约束系统
4. 实现 spawn 调度器
5. 添加类型反射能力
6. 编写相关测试

---

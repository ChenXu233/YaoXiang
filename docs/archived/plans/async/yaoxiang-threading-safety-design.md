# YaoXiang 线程安全设计方案

> 设计决策：Send/Sync 类型约束（类似 Rust）
> 创建日期：2025-01-02
> 状态：待实现

---

## 一、设计概述

### 1.1 核心概念

| 约束 | 说明 | 类比 |
|------|------|------|
| **Send** | 类型可以安全地跨线程**转移所有权** | Rust Send |
| **Sync** | 类型可以安全地跨线程**共享引用** | Rust Sync |

### 1.2 设计原则

- **类型即约束**：Send/Sync 作为类型约束
- **自动派生**：复合类型自动继承字段的 Send/Sync 属性
- **编译时检查**：类型检查器验证线程安全性
- **与现有系统集成**：与 ref/mut、spawn 协同工作

---

## 二、类型约束设计

### 2.1 Send 约束

```yaoxiang
# Send 标记类型可以跨线程转移

# 基本类型自动满足 Send
# Int, Float, Bool, String, Char 都是 Send

# 结构体自动派生 Send
type Point = Point(x: Int, y: Float)
# Point 是 Send，因为 Int 和 Float 都是 Send

# 包含非 Send 字段的类型不是 Send
type NonSend = NonSend(data: Rc[Int])
# Rc 不是 Send（引用计数不是原子的）
# 因此 NonSend 不是 Send

# 泛型自动推导
type Container[T] = Container(value: T)
# Container[Int] 是 Send
# Container[Rc[Int]] 不是 Send（取决于 T）
```

### 2.2 Sync 约束

```yaoxiang
# Sync 标记类型可以安全地跨线程共享引用

# 基本类型都是 Sync
# &Int, &Float, &String 都可以安全共享

# 结构体自动派生 Sync
type Point = Point(x: Int, y: Float)
# &Point 是 Sync，因为 &Int 和 &Float 都是 Sync

# 包含内部可变性的类型
type Counter = Counter(value: Int, mutex: Mutex[Int])
# &Counter 是 Sync，因为 Mutex 提供内部可变性
```

### 2.3 互斥关系

```yaoxiang
# 如果 T: Sync，则 &T: Send
# （共享引用可以安全发送到其他线程）

# 如果 T: Send，则 &T 不是 Send
# （借用规则：同一资源不能有多个所有者）

# 如果 T: Sync，则 &T: Sync
# （共享引用本身可以安全共享）
```

---

## 三、与现有特性集成

### 3.1 spawn 函数约束

```yaoxiang
# spawn 要求参数和返回值满足 Send
spawn[T: Send](fn() -> T) -> Task[T] = (f) => {
    # T 必须可安全转移
    # f 可以安全发送到新线程
}

# 示例
type Data = Data(value: Int)

# 有效：Data 是 Send
task = spawn(|| => Data(42))

# 无效：Rc 不是 Send
type SharedData = SharedData(rc: Rc[Int])
# task = spawn(|| => SharedData(Rc.new(42))  # 编译错误！
```

### 3.2 ref/mut 与 Sync

```yaoxiang
# 跨线程借用需要 Sync 约束

# 有效：Point 是 Sync
process_point(ref Point) -> Void = (p) => {
    print(p.x)
}

# 无效：NonSync 不是 Sync
type NonSync = NonSync(rc: Rc[Int])
# process_non_sync(ref NonSync) -> Void  # 编译错误！

# 需要显式使用线程安全版本
process_safe(ref ThreadSafe[NonSync]) -> Void = (p) => {
    print(p.value)
}
```

### 3.3 Arc 自动包装

```yaoxiang
# 对于非 Send 类型，可以使用 Arc 包装

type NonSend = NonSend(data: Rc[Int])

# Arc 实现了 Send（原子引用计数）
# Arc[NonSend] 是 Send

create_task(Arc[NonSend]) -> Task[Void] = (data) => {
    spawn(|| => {
        # Arc 可以安全发送到线程
        print(data.data)
    })
}
```

---

## 四、标准库的 Send/Sync 实现

### 4.1 已实现 Send + Sync 的类型

```yaoxiang
# 原子类型
ThreadSafeInt, ThreadSafeBool, ThreadSafeUsize
# 都是 Send + Sync

# 锁和同步原语
Mutex[T], RwLock[T], Condvar
# T: Send => Mutex[T]: Send + Sync
# T: Send => RwLock[T]: Send + Sync

# 原子引用计数
Arc[T]
# T: Send + Sync => Arc[T]: Send + Sync

# 通道
Channel[T], Receiver[T], Sender[T]
# T: Send => Channel[T]: Send
# T: Send => Receiver[T]: Send
# T: Send => Sender[T]: Send
```

### 4.2 未实现 Send 的类型

```yaoxiang
# 非原子引用计数
Rc[T]
# 不是 Send，也不是 Sync

# 原始指针
Ptr[T]
# 不是 Send，也不是 Sync

# RefCell[T]
# 不是 Send，也不是 Sync（运行时借用检查）
```

---

## 五、类型检查规则

### 5.1 复合类型的派生规则

```yaoxiang
# 结构体类型
type Struct[T1, T2] = Struct(f1: T1, f2: T2)

# Send 派生
Struct[T1, T2]: Send
    ⇐ T1: Send 且 T2: Send

# Sync 派生
Struct[T1, T2]: Sync
    ⇐ T1: Sync 且 T2: Sync

# 联合类型
type Union[T1, T2] = variant1(T1) | variant2(T2)

# Send 派生
Union[T1, T2]: Send
    ⇐ T1: Send 且 T2: Send

# Sync 派生
Union[T1, T2]: Sync
    ⇐ T1: Sync 且 T2: Sync
```

### 5.2 函数类型的约束

```yaoxiang
type Fn[T, R] = fn(T) -> R

# Send
Fn[T, R]: Send
    ⇐ R: Send  (返回值可发送)
    ⇐ T: Send  (参数可发送)

# Sync
Fn[T, R]: Sync
    ⇐ R: Sync  (返回引用可安全共享)
```

---

## 六、使用示例

### 6.1 基本用法

```yaoxiang
# 数据类型
type Point = Point(x: Int, y: Float)

# 并发函数
process_point(Point) -> Float spawn = (p) => {
    p.x.to_float() + p.y
}

# 跨线程共享
main() -> Void = () => {
    p = Point(1, 2.0)
    
    # spawn 自动检查 Send
    task = spawn(|| => process_point(p))
    
    result = task.await()
    print(result)
}
```

### 6.2 使用 Arc 共享

```yaoxiang
type Counter = Counter(value: Int)

create_counters(Int) -> List[Arc[Counter]] = (n) => {
    counters = []
    for i in 0..n {
        counters = counters + [Arc.new(Counter(i))]
    }
    counters
}

# 并发更新
increment_all(List[Arc[Counter]]) -> Void spawn = (counters) => {
    for counter in counters {
        # Arc 可安全跨线程共享
        ref_count = ref counter.value
        ref_count = ref_count + 1  # 需要 Mutex！
    }
}
```

### 6.3 使用 Mutex

```yaoxiang
type SafeCounter = SafeCounter(mutex: Mutex[Int])

new_counter() -> Arc[SafeCounter] = () => {
    Arc.new(SafeCounter(Mutex.new(0)))
}

increment(Arc[SafeCounter]) -> Void spawn = (counter) => {
    guard = counter.mutex.lock()
    guard.value = guard.value + 1
    # guard 自动释放
}

main() -> Void = () => {
    c = new_counter()
    
    spawn increment(c)
    spawn increment(c)
    spawn increment(c)
    
    # 等待完成
    sleep(100)
    
    guard = c.mutex.lock()
    print("Final: " + guard.value.to_string())
}
```

---

## 七、与 spawn 的协作

### 7.1 spawn 的类型签名

```yaoxiang
# spawn 函数定义
pub spawn[T: Send](fn() -> T) -> Task[T]

# spawn_with
pub spawn_with[T: Send, R: Send](R, fn(R) -> T) -> Task[T]

# 限制：
# - T 必须是 Send（结果可转移到调用线程）
# - 捕获的环境必须是 Send（可发送到新线程）
```

### 7.2 环境捕获

```yaoxiang
# 有效：捕获的环境是 Send
counter: Arc[Int] = Arc.new(0)
task = spawn(|| => {
    ref_counter = ref counter
    ref_counter = ref_counter + 1
})

# 无效：捕获的环境不是 Send
rc_counter: Rc[Int] = Rc.new(0)
# task = spawn(|| => {  # 编译错误！
#     ref_counter = ref rc_counter  # Rc 不是 Send
# })
```

---

## 八、错误消息

### 8.1 Send 错误

```
错误：类型 Rc[Int] 不满足 Send 约束

  --> example.yx:10:5
   |
10 |     spawn(|| => use_rc(rc))
   |     ^^^^^^^^^^^^^^^^^^^^^^
   |
   = 帮助：Rc 使用非原子引用计数，不能安全跨线程发送
   = 帮助：考虑使用 Arc 替代 Rc
```

### 8.2 Sync 错误

```
错误：类型 NonSync 不满足 Sync 约束

  --> example.yx:5:20
   |
5 |     ref_data = ref non_sync_data
   |                    ^^^^^^^^^^^^
   |
   = 帮助：NonSync 包含非线程安全字段
   = 帮助：使用 ThreadSafe[NonSync] 或 Mutex[NonSync]
```

---

## 九、实现路线图

### 阶段1：基础类型标记

- [ ] 定义 Send 和 Sync 类型约束
- [ ] 实现基本类型的 Send/Sync 派生
- [ ] 添加类型检查规则

### 阶段2：复合类型支持

- [ ] 实现结构体类型的 Send/Sync 派生
- [ ] 实现联合类型的 Send/Sync 派生
- [ ] 实现泛型类型的 Send/Sync 推导

### 阶段3：标准库集成

- [ ] 为 Arc、Mutex、Channel 实现 Send/Sync
- [ ] 标记 Rc、RefCell 为非 Send/Sync
- [ ] 添加线程安全版本的数据结构

### 阶段4：与 spawn 集成

- [ ] 修改 spawn 函数添加 Send 约束
- [ ] 实现环境捕获的 Send 检查
- [ ] 添加更好的错误消息

---

## 十、与文档的集成

此设计应集成到以下文档章节：

1. **第四章 内存管理** - 添加 Send/Sync 约束说明
2. **第五章 异步编程** - spawn 的 Send 约束
3. **第十章 标准库** - Arc、Mutex 的 Send/Sync 实现
4. **第十一章 故障排除** - 常见 Send/Sync 错误

---

## 附录：完整的 Send/Sync 派生表

| 类型 | Send | Sync | 条件 |
|------|------|------|------|
| Int | ✅ | ✅ | 无 |
| Float | ✅ | ✅ | 无 |
| Bool | ✅ | ✅ | 无 |
| String | ✅ | ✅ | 无 |
| Struct[T] | ✅ | ✅ | T: Send / T: Sync |
| Union[T] | ✅ | ✅ | T: Send / T: Sync |
| Vec[T] | ✅ | ✅ | T: Send / T: Sync |
| Option[T] | ✅ | ✅ | T: Send / T: Sync |
| Result[T, E] | ✅ | ✅ | T, E: Send / Sync |
| Arc[T] | ✅ | ✅ | T: Send + Sync |
| Mutex[T] | ✅ | ✅ | T: Send |
| Channel[T] | ✅ | - | T: Send |
| Rc[T] | ❌ | ❌ | 无 |
| RefCell[T] | ❌ | ❌ | 无 |
| &T | - | ✅ | T: Sync |
| fn(T) -> R | ✅ | ✅ | R: Send, T: Send |

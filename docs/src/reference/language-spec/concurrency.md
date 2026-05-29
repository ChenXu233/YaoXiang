# 并发模型规范

本文件定义 YaoXiang 编程语言的并发模型规范，包括异步编程、并发原语和内存模型。

---

## 第一章：并作函数

### 1.1 spawn 函数（并作函数）

```
SpawnFn     ::= Identifier ':' FnType 'spawn' '=' Lambda
FnType      ::= '(' ParamTypes? ')' '->' TypeExpr ('@' Annotation)?
Annotation  ::= 'block' | 'eager'
```

**函数注解**：

| 注解 | 位置 | 行为 |
|------|------|------|
| `@block` | 返回类型后 | 禁用并发优化，完全顺序执行 |
| `@eager` | 返回类型后 | 强制急切求值 |

**语法示例**：

```
// 并作函数：可并发执行
fetch_data: (url: String) -> JSON spawn = { ... }

// @block 同步函数：完全顺序执行
main: () -> Void @block = { ... }

// @eager 急切函数：立即执行
compute: (n: Int) -> Int @eager = { ... }
```

### 1.2 spawn 块

显式声明的并发疆域，块内任务将并作执行：

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
```

**示例**：

```
// 并作块：显式并发
(result_a, result_b) = spawn {
    parse(fetch("url1")),
    parse(fetch("url2"))
}
```

### 1.3 spawn 循环

数据并行循环，循环体在所有数据元素上并作执行：

```
SpawnFor    ::= Identifier '=' 'spawn' 'for' Identifier 'in' Expr '{' Expr '}'
```

**示例**：

```
// 并作循环：数据并行
results = spawn for item in items {
    process(item)
}
```

### 1.4 错误传播运算符

```
ErrorPropagate ::= Expr '?'
```

**示例**：

```
process: (p: Point) -> Result[Data, Error] = {
    data = fetch_data()?      // 自动传播错误
    transform(data)?
}
```

---

## 第二章：内存管理

### 2.1 所有权模型

YaoXiang 采用**所有权模型**管理内存，每个值有唯一的所有者：

| 语义 | 说明 | 语法 |
|------|------|------|
| **Move** | 默认语义，所有权转移 | `p2 = p` |
| **ref** | 共享（Arc 引用计数） | `shared = ref p` |
| **clone()** | 显式复制 | `p2 = p.clone()` |

### 2.2 Move 语义（默认）

```yaoxiang
// 赋值 = Move（零拷贝）
p: Point = Point(1.0, 2.0)
p2 = p              // Move，p 失效

// 函数传参 = Move
process: (p: Point) -> Void = {
    // p 的所有权转移进来
}

// 返回值 = Move
create: () -> Point = {
    p = Point(1.0, 2.0)
    return p        // Move，所有权转移
}
```

### 2.3 ref 关键字（Arc）

`ref` 关键字创建**引用计数指针**（Arc），用于安全共享：

```yaoxiang
// 创建 Arc
p: Point = Point(1.0, 2.0)
shared = ref p      // Arc，线程安全

// 共享访问
spawn(() => print(shared.x))   // 安全

// Arc 自动管理生命周期
// shared 离开作用域时，计数归零自动释放
```

**特点**：
- 线程安全引用计数
- 自动管理生命周期
- 跨 spawn 边界安全

### 2.4 clone() 显式复制

```yaoxiang
// 显式复制值
p: Point = Point(1.0, 2.0)
p2 = p.clone()      // p 和 p2 独立

// 两者都可修改，互不影响
p.x = 0.0           // 正确
p2.x = 0.0          // 正确
```

### 2.5 unsafe 代码块

`unsafe` 代码块允许使用裸指针，用于系统级编程：

```yaoxiang
// 裸指针类型
PtrType ::= '*' TypeExpr

// unsafe 代码块
UnsafeBlock ::= 'unsafe' '{' Stmt* '}'
```

**示例**：

```yaoxiang
p: Point = Point(1.0, 2.0)

// 裸指针只能在 unsafe 块中使用
unsafe {
    ptr: *Point = &p     // 获取裸指针
    (*ptr).x = 0.0       // 解引用
}
```

**限制**：
- 裸指针只能在 `unsafe` 块中使用
- 用户保证不悬空、不释放后使用
- 不参与 Send/Sync 检查

### 2.6 所有权语法 BNF

```bnf
// === 所有权表达式 ===

// Move（默认）
MoveExpr     ::= Expr

// ref Arc
RefExpr      ::= 'ref' Expr

// clone
CloneExpr    ::= Expr '.clone' '(' ')'

// === 裸指针（仅 unsafe） ===

PtrType       ::= '*' TypeExpr
UnsafeBlock   ::= 'unsafe' '{' Stmt* '}'
```

---

## 第三章：并发安全

### 3.1 Send / Sync 约束

| 约束 | 语义 | 说明 |
|------|------|------|
| **Send** | 可安全跨线程传输 | 值可以移动到另一个线程 |
| **Sync** | 可安全跨线程共享 | 不可变引用可以共享到另一个线程 |

**自动派生**：

```
// Send 派生规则
Struct[T1, T2]: Send ⇐ T1: Send 且 T2: Send

// Sync 派生规则
Struct[T1, T2]: Sync ⇐ T1: Sync 且 T2: Sync
```

**类型约束**：

| 类型 | Send | Sync | 说明 |
|------|------|------|------|
| `T`（值） | ✅ | ✅ | 不可变数据 |
| `ref T` | ✅ | ✅ | Arc 线程安全 |
| `*T` | ❌ | ❌ | 裸指针不安全 |

### 3.2 Send/Sync 约束层次

```
Send ──► 可安全跨线程传输
  │
  └──► Sync ──► 可安全跨线程共享
       │
       └──► 满足 Send + Sync 的类型可自动并发

Arc[T] 实现 Send + Sync（线程安全引用计数）
Mutex[T] 提供内部可变性
```

### 3.3 并发安全类型

| 类型 | 语义 | 并发安全 | 说明 |
|------|------|----------|------|
| `T` | 不可变数据 | ✅ 安全 | 默认类型，多任务读取无竞争 |
| `Ref[T]` | 可变引用 | ⚠️ 需同步 | 标记为可并发修改，编译检查锁使用 |
| `Atomic[T]` | 原子类型 | ✅ 安全 | 底层原子操作，无锁并发 |
| `Mutex[T]` | 互斥锁包装 | ✅ 安全 | 自动加锁解锁，编译保证 |
| `RwLock[T]` | 读写锁包装 | ✅ 安全 | 读多写少场景优化 |

**语法**：

```
Mutex[T]    // 互斥锁包装的可变数据
Atomic[T]   // 原子类型（仅限 Int、Float 等）
RwLock[T]   // 读写锁包装
```

**with 语法糖**：

```
with mutex.lock() {
    // 临界区：受 Mutex 保护
    ...
}
```

---

## 附录：并发语法速查

### A.1 spawn 语法

```yaoxiang
// 并作函数
fetch_data: (url: String) -> JSON spawn = { ... }

// 并作块
(result_a, result_b) = spawn {
    parse(fetch("url1")),
    parse(fetch("url2"))
}

// 并作循环
results = spawn for item in items {
    process(item)
}
```

### A.2 所有权语法

```yaoxiang
// Move（默认）
p2 = p

// ref Arc
shared = ref p

// clone
p2 = p.clone()

// unsafe
unsafe {
    ptr: *Point = &p
    (*ptr).x = 0.0
}
```

### A.3 并发安全类型

```yaoxiang
// 互斥锁
mutex: Mutex[Int] = Mutex(0)
with mutex.lock() {
    // 临界区
}

// 原子类型
counter: Atomic[Int] = Atomic(0)
counter.increment()

// 读写锁
data: RwLock[Data] = RwLock(data)
with data.read() {
    // 读取操作
}
```

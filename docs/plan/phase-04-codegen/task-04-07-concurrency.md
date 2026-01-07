# Task 4.7: 并发原语字节码

> **优先级**: P2（依赖运行时调度器）
> **状态**: ⏳ 待实现（实现依赖于运行时调度器）

## 功能描述

生成并发相关（spawn 标记）的字节码。

## 设计原则（基于 RFC-008 决策）

**核心原则**：**并发是运行时问题，不是字节码问题**

根据 RFC-008《Runtime 并发模型》决策：
- **spawn** 是注解标记，生成普通函数调用 + 任务创建（运行时处理）
- **await** 不是关键字，是值使用时的自动等待（调度器职责）
- **channel/mutex** 是标准库类型，不是字节码指令
- **DAG** 是运行时数据结构，不是字节码

## 代码生成策略

根据 RFC-008，**并发不需要专用字节码指令**：

| IR 指令 | 字节码 | 原因 |
|---------|-------|------|
| `spawn` 标记的函数调用 | `CallStatic` | 普通函数调用，调度器检测 Async[T] 并创建任务 |
| `Yield` | `Yield` | 让出执行权，用于异步调度 |
| `TailCall` | `TailCall` | 尾调用优化，用于异步任务切换 |
| `Spawn` IR 指令 | `Nop` | 根据 RFC-008，spawn 是运行时问题，codegen 不生成特殊字节码 |
| `Await` | **不存在** | await 不是关键字，运行时自动处理 |

## 当前实现状态

```rust
// src/middle/codegen/mod.rs

// 并发操作（基于 RFC-008）
// spawn 是注解标记，await 不是关键字（运行时自动处理）
Spawn { func: _ } => {
    // 根据 RFC-008，spawn 标记由运行时处理
    // 编译产物是普通函数调用，调度器负责创建 Async[T]
    // codegen 不需要生成特殊字节码
    Ok(BytecodeInstruction::new(TypedOpcode::Nop, vec![]))
}

Yield => Ok(BytecodeInstruction::new(TypedOpcode::Yield, vec![])),
```

**说明**：`Spawn → Nop` 是符合 RFC-008 的预期行为，因为：
1. 并发是运行时问题，不需要特殊字节码
2. 调度器在运行时检测 spawn 标记，创建 Async[T] 任务
3. await 不是关键字，值使用时自动等待

## 运行时代码生成（不在 codegen 中）

根据 RFC-008，调度器负责以下工作：

1. **任务创建**：检测 spawn 标记，创建 Async[T] 代理
2. **DAG 构建**：跟踪任务依赖关系
3. **自动等待**：值使用时检测 Async[T] 类型，插入等待点
4. **任务调度**：在 I/O 或计算完成后恢复执行

```rust
// 调度器伪代码（不在 codegen 中）
fn eval_async(async_val: Async<T>) -> T {
    match async_val.state {
        State::Ready(t) => t,
        State::Pending(task) => {
            scheduler.schedule(task);
            yield();  // 让出执行权
            eval_async(async_val)  // 恢复时重新检查
        }
    }
}
```

## 字节码指令复用

根据 RFC-008，并发复用现有指令，不添加新 opcode：

| Opcode | 值 | 说明 |
|--------|-----|------|
| `CallStatic` | 0x80 | 用于调用 spawn 的函数 |
| `Yield` | 0x0A | 暂停执行（异步调度） |
| `TailCall` | 0x09 | 尾调用优化（异步任务切换） |

## 生成规则示例

### spawn 标记的函数调用
```yaoxiang
fetch_data: spawn (String) -> JSON = (url) => {
    HTTP.get(url).json()
}

main: () -> Void = () => {
    data = fetch_data("https://api.example.com")
    print(data.name)
}
```
生成字节码：
```
# fetch_data 函数体
CallStatic r1, func_id=HTTP.get, base_arg=?, arg_count=1
CallStatic r2, func_id=JSON.json, base_arg=r1, arg_count=1
ReturnValue r2

# main 函数
CallStatic r3, func_id=fetch_data, base_arg=?, arg_count=1
# r3 是 Async[JSON] 类型（调度器创建）

# 使用 data.name 时自动等待
# 运行时检测 Async[JSON] 被使用，插入等待点
GetField r4, r3, 0
CallStatic print, r4
```

### 并作块（spawn { }）
```yaoxiang
result = spawn {
    a = compute_a()
    b = compute_b()
    a + b
}
```
生成字节码：
```
# 并作块展开为函数 + spawn
CallStatic r1, func_id=compute_a, base_arg=?, arg_count=0
CallStatic r2, func_id=compute_b, base_arg=?, arg_count=0
I64Add r1, r2 -> r3
ReturnValue r3

# spawn 标记：调度器在运行时创建 Async[T]
# 这是运行时的职责，不是字节码的一部分
```

## 验收测试

```yaoxiang
# test_concurrency_bytecode.yx

# 基础 spawn（没有 await 关键字）
task = spawn { 1 + 2 }
result = task  # 自动等待
assert(result == 3)

# 并行计算（调度器负责并行）
tasks = [
    spawn { fib(20) },
    spawn { fib(25) },
    spawn { fib(30) }
]
# 等待所有任务完成（自动等待）
r1 = tasks[0]
r2 = tasks[1]
r3 = tasks[2]
assert(r1 == 6765)
assert(r2 == 75025)
assert(r3 == 832040)

# 异步数据流（自动等待链）
fetch_user: spawn (Int) -> User = (id) => { ... }
fetch_posts: spawn (User) -> Posts = (user) => { ... }

user = fetch_user(1)        # Async[User]
posts = fetch_posts(user)   # Async[Posts]，依赖 user
result = posts.title         # 自动等待两个任务
assert(result != "")

print("Concurrency bytecode tests passed!")
```

## 相关文件

- **src/vm/opcode.rs**: TypedOpcode 枚举定义
- **src/runtime/scheduler/**: 运行时调度器（不在 codegen 中）
- **src/middle/codegen/mod.rs**: spawn 标记处理（Spawn → Nop，符合 RFC-008）
- **src/middle/ir.rs**: IR 定义（已移除 CallAsync 和 Await）
- **RFC-008**: Runtime 并发模型详细设计

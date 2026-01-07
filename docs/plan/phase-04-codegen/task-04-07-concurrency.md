# Task 4.7: 并发原语字节码

> **优先级**: P2（依赖运行时调度器）
> **状态**: ⏳ 待实现

## 功能描述

生成并发相关（spawn、await）的字节码。

## 设计原则（基于 RFC-008 决策）

**并发是运行时问题，不是字节码问题**：

根据 RFC-008《Runtime 并发模型》决策：
- **spawn** 是注解标记，生成普通函数调用 + 任务创建
- **await** 是值使用时的自动等待，调度器职责
- **channel/mutex** 是标准库类型，不是字节码指令
- **DAG** 是运行时数据结构，不是字节码

## 字节码指令

| Opcode | 值 | 操作 | 说明 |
|--------|-----|------|------|
| `CallStatic` | 0x80 | 函数调用 | 用于调用 spawn 函数 |
| `Yield` | 0x0A | 暂停执行 | 异步调度（让出执行权） |
| `TailCall` | 0x09 | 尾调用优化 | 用于异步任务切换 |

**说明**：并发不需要专用字节码指令，核心机制：
1. `spawn fn` → 普通函数调用 + 运行时创建 Async[T]
2. `await` → 值使用时自动插入等待点
3. 调度器在值使用点挂起/恢复任务

## 生成规则

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
# r3 是 Async[JSON] 类型

# 使用 data.name 时自动等待
# 这是调度器的职责，不是字节码
# 运行时检测到 Async[JSON] 被使用，插入等待点
GetField r4, r3, 0  # 获取 name 字段（触发 await）
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
# 并作块是语法糖，编译器展开为多个 spawn 调用
CallStatic r1, func_id=compute_a, base_arg=?, arg_count=0
CallStatic r2, func_id=compute_b, base_arg=?, arg_count=0
I64Add r1, r2 -> r3
ReturnValue r3
```

### 显式 await（未来兼容性）
```yaoxiang
task = spawn { heavy_compute() }
result = await task
```
生成字节码：
```
CallStatic r1, func_id=heavy_compute, base_arg=?, arg_count=0
STORE r1 -> task

# await task 是语法糖，实际是获取 task 的值
# 调度器在 GetField 或字段访问时自动处理
GetField r2, r1, 0  # 等待并获取结果
STORE r2 -> result
```

## 调度器职责（不在字节码中）

调度器是运行时组件，负责：

1. **任务创建**：`spawn fn` 调用时创建 Async[T] 代理
2. **DAG 构建**：跟踪任务依赖关系
3. **自动等待**：值使用时检测 Async[T] 类型，插入等待点
4. **任务调度**：在 I/O 或计算完成后恢复执行

```rust
// 调度器伪代码
fn eval_async(async_val: Async<T>) -> T {
    match async_val.state {
        State::Ready(t) => t,
        State::Pending(task) => {
            scheduler.schedule(task);
            // 挂起当前任务
            yield();
            // 恢复时重新检查状态
            eval_async(async_val)
        }
    }
}
```

## 验收测试

```yaoxiang
# test_concurrency_bytecode.yx

# 基础 spawn
task = spawn { 1 + 2 }
result = await task
assert(result == 3)

# 并行计算（调度器负责并行）
tasks = [
    spawn { fib(20) },
    spawn { fib(25) },
    spawn { fib(30) }
]
# 等待所有任务完成
r1 = await tasks[0]
r2 = await tasks[1]
r3 = await tasks[2]
assert(r1 == 6765)
assert(r2 == 75025)
assert(r3 == 832040)

# 异步数据流
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
- **src/vm/scheduler.rs**: 运行时调度器（不在 codegen 中）
- **src/middle/codegen/generator.rs**: spawn 标记处理
- **RFC-008**: Runtime 并发模型详细设计

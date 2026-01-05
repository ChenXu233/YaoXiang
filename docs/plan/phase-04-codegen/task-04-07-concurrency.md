# Task 4.7: 并发原语字节码

> **优先级**: P1
> **状态**: ⏳ 待实现

## 功能描述

生成并发相关（spawn、await、channel、mutex）的字节码。

## 字节码指令

| Opcode | 操作 | 说明 |
|--------|------|------|
| `SPAWN` | 创建任务 | 启动异步任务 |
| `AWAIT` | 等待任务 | 等待任务完成 |
| `CHANNEL_NEW` | 创建通道 | |
| `CHANNEL_SEND` | 发送消息 | |
| `CHANNEL_RECV` | 接收消息 | |
| `MUTEX_NEW` | 创建互斥锁 | |
| `MUTEX_LOCK` | 加锁 | |
| `MUTEX_UNLOCK` | 解锁 | |

## 字节码格式

```rust
struct Spawn {
    task: FunctionRef,
    args: Vec<Reg>,
    task_id: Reg,
}

struct Await {
    task_id: Reg,
    result: Reg,
}

struct ChannelSend {
    channel: Reg,
    value: Reg,
}

struct ChannelRecv {
    channel: Reg,
    result: Reg,
}
```

## 生成规则

### spawn/await
```yaoxiang
task = spawn { compute_heavy_task() }
result = await task
```
生成字节码：
```
SPAWN compute_heavy_task() -> task_id
AWAIT task_id -> result
```

### channel 通信
```yaoxiang
ch = channel()
spawn { ch.send(42) }
value = ch.recv()
```
生成字节码：
```
CHANNEL_NEW -> ch
SPAWN send_task -> task_id
CHANNEL_RECV ch -> value
```

## 验收测试

```yaoxiang
# test_concurrency_bytecode.yx

# 基础 spawn/await
task = spawn { 1 + 2 }
result = await task
assert(result == 3)

# 并行计算
tasks = [
    spawn { fib(20) },
    spawn { fib(25) },
    spawn { fib(30) }
]
results = tasks.map(t => await t)
assert(results.length == 3)

# Channel 通信
ch = channel()
spawn {
    ch.send("hello")
}
msg = ch.recv()
assert(msg == "hello")

print("Concurrency bytecode tests passed!")
```

## 相关文件

- **bytecode.rs**: 并发指令定义
- **generator.rs**: 并发生成逻辑

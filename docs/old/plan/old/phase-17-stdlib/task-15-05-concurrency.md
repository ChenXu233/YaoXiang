# Task 15.5: 并发原语

> **优先级**: P1
> **状态**: ⏳ 待实现

## 功能描述

提供线程、Channel、Mutex、WaitGroup 等并发原语。

## 并发 API

```yaoxiang
# 线程
use std::thread

# 创建线程
handle = thread::spawn(|| {
    print("In thread")
    42
})

# 等待线程
result = thread::join(handle)
assert(result == 42)

# Channel
ch = channel()
ch.send("message")
msg = ch.recv()

# Mutex
mutex = mutex::new(0)
mutex.lock()
value = mutex.get()
mutex.set(value + 1)
mutex.unlock()

# WaitGroup
wg = wait_group()
for i in 0..4 {
    wg.add(1)
    spawn(|| {
        # do work
        wg.done()
    })
}
wg.wait()
```

## 验收测试

```yaoxiang
# test_concurrency.yx

use std::thread
use std::sync

# 并行计算
results = []
for i in 0..4 {
    handle = spawn || i * i
    results = results.push(handle)
}

# 等待所有结果
final = results.map(h => await h)
# final = [0, 1, 4, 9]

# Channel 通信
ch = channel()
spawn || ch.send("hello")
msg = ch.recv()
assert(msg == "hello")

print("Concurrency tests passed!")
```

## 相关文件

- **thread/mod.rs**
- **sync/mod.rs**

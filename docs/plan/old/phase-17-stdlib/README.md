# Phase 17: 标准库

> **模块路径**: `src/std/`
> **状态**: ⏳ 待实现
> **依赖**: P8（Core Runtime）

## 概述

标准库提供常用功能模块，包括 IO、集合、并发、字符串处理等。

## 文件结构

```
phase-17-stdlib/
├── README.md                       # 本文档
├── task-17-01-core.md              # 核心模块
├── task-17-02-collections.md       # 集合类型
├── task-17-03-io.md                # IO 操作
├── task-17-04-string.md            # 字符串处理
├── task-17-05-concurrency.md       # 并发原语
├── task-17-06-async.md             # 异步支持
├── task-17-07-net.md               # 网络
└── task-17-08-time.md              # 时间日期
```

## 完成状态

| Task | 名称 | 状态 | 依赖 |
|------|------|------|------|
| task-17-01 | 核心模块 | ⏳ 待实现 | P8 |
| task-17-02 | 集合类型 | ⏳ 待实现 | task-17-01 |
| task-17-03 | IO 操作 | ⏳ 待实现 | task-17-01 |
| task-17-04 | 字符串处理 | ⏳ 待实现 | task-17-01 |
| task-17-05 | 并发原语 | ⏳ 待实现 | task-17-01 |
| task-17-06 | 异步支持 | ⏳ 待实现 | task-17-05 |
| task-17-07 | 网络 | ⏳ 待实现 | task-17-03 |
| task-17-08 | 时间日期 | ⏳ 待实现 | task-17-01 |

## 模块结构

```
std/
├── mod.rs              # 主模块，导出所有内容
├── core/               # 核心模块
│   ├── option.yx       # Option 类型
│   ├── result.yx       # Result 类型
│   ├── panic.yx        #  panic 处理
│   └── prelude.yx      # 自动导入
├── collections/        # 集合模块
│   ├── list.yx         # 列表
│   ├── dict.yx         # 字典
│   ├── set.yx          # 集合
│   └── deque.yx        # 双端队列
├── io/                 # IO 模块
│   ├── file.yx         # 文件操作
│   ├── buffer.yx       # 缓冲 IO
│   └── reader.yx       #  Reader/Writer
├── string/             # 字符串模块
│   ├── string.yx       # 字符串
│   └── builder.yx      # 字符串构建
├── concurrency/        # 并发模块
│   ├── channel.yx      # 通道
│   ├── mutex.yx        # 互斥锁
│   └── atomic.yx       # 原子类型
├── net/                # 网络模块
│   ├── tcp.yx          # TCP
│   ├── udp.yx          # UDP
│   └── http.yx         # HTTP
└── time/               # 时间模块
    ├── time.yx         # 时间点
    └── duration.yx     # 时间段
```

## 核心模块示例

```yaoxiang
use std::core::*;
use std::collections::*;

# Option 类型
let value: Option[Int] = some(42)
let result = match value {
    some(n) => n * 2,
    none => 0,
}

# Result 类型
let result: Result[Int, String] = ok(42)
let value = result?  # 错误传播

# 集合操作
let list = [1, 2, 3, 4, 5]
let doubled = list.map(|x| x * 2)
let filtered = list.filter(|x| x > 2)
let sum = list.fold(0, |acc, x| acc + x)
```

## IO 模块示例

```yaoxiang
use std::io::*;

# 读取文件
let content = File::read_all("example.txt")?

# 写入文件
File::write_all("output.txt", "Hello, YaoXiang!")?

# 缓冲读写
let file = File::open("data.txt")?
let reader = BufferedReader::new(file)
for line in reader.lines() {
    print(line)
}
```

## 并发模块示例

```yaoxiang
use std::concurrency::*;

# 通道
let (sender, receiver) = channel()
spawn {
    sender.send("Hello from spawned task!")
}
let message = receiver.recv()

# 互斥锁
let mutex = Mutex::new(0)
spawn {
    let mut value = mutex.lock()
    *value = *value + 1
}
```

## 网络模块示例

```yaoxiang
use std::net::*;

# TCP 连接
let stream = TcpStream::connect("127.0.0.1:8080")?
stream.write_all(b"GET / HTTP/1.0\r\n\r\n")?

# TCP 监听
let listener = TcpListener::bind("0.0.0.0:8080")?
for connection in listener.incoming() {
    spawn {
        handle_connection(connection?)
    }
}
```

## 相关文件

- **mod.rs**: 标准库主模块
- **io/**: IO 模块
- **collections/**: 集合模块
- **string/**: 字符串模块

## 相关文档

- [Phase 8: Core Runtime](../phase-08-core-runtime/README.md)
- [Phase 18: Bootstrap](../phase-18-bootstrap/README.md)

---
title: "RFC-034: 统一调试工具链"
status: "草案"
author: "晨煦"
created: "2026-07-06"
updated: "2026-07-06"
---

# RFC-034: 统一调试工具链

## 摘要

为 YaoXiang 引入统一的调试工具链。核心设计是**一个源头，三种消费**：编译前端将源码位置、变量名、类型信息作为一等公民嵌入 YaoXiang IR，解释器、JIT、LLVM 三个后端各自消费同一套元数据。用户通过 `yaoxiang run --debug` 启动 DAP（Debug Adapter Protocol）服务器，VS Code 通过 stdio 连接，获得断点、单步、变量查看、调用栈、表达式求值、并发调试的统一体验——无论底层是哪种执行引擎。

## 动机

### 为什么需要这个特性？

当前排查 YaoXiang 程序错误的手段极度原始：

```yaoxiang
io.println("DEBUG: x = " + x.to_string())
io.println("DEBUG: entered branch A")
```

三个致命问题：

1. **编译器开发自举受阻**：用 YaoXiang 写 YaoXiang 编译器，但写编译器的人 debug 不了自己写的代码。自举阶段缺乏交互式调试手段是死胡同。
2. **三种引擎，零调试**：解释器、JIT、LLVM 各自跑各自的，出了问题用户只能看 `ALL TESTS PASSED` 有没有出现在 stdout 里。断言失败？不知道在哪行、不知道变量值。
3. **并发是黑盒**：`spawn` 创建了多个任务，哪个任务挂了？变量被谁 move 了？全靠运气猜。

### 设计目标

- **统一体验**：解释器能过的断点，JIT 也能过，LLVM 也有一致的源码映射。用户不感知底层引擎差异。
- **一个源头**：调试元数据随 IR 流动，不重复定义，不维护两套映射。
- **零侵入**：`yaoxiang run --debug` 一个参数，不添加此参数时编译和执行行为完全不变。
- **DAP 标准**：直接对接 VS Code 生态，不重新发明编辑器协议。

## 提案

### 核心设计

架构总览：

```
┌──────────────────────────────────────────────────────┐
│                    VS Code / 编辑器                    │
│              DAP Client (launch.json)                 │
└────────────────────────┬─────────────────────────────┘
                         │ stdio
┌────────────────────────▼─────────────────────────────┐
│                 DAP 服务器 (yx-core)                   │
│  ┌─────────┐  ┌──────────┐  ┌───────────────────┐    │
│  │ 会话管理 │  │ 断点管理  │  │ 表达式求值引擎     │    │
│  └─────────┘  └──────────┘  └───────────────────┘    │
└────────────────────────┬─────────────────────────────┘
                         │ 查询/控制
┌────────────────────────▼─────────────────────────────┐
│                  运行时调试接口 (trait)                 │
│  pause / resume / step / get_frames / eval / ...    │
└────┬──────────────────┬──────────────────┬──────────┘
     │                  │                  │
┌────▼────┐    ┌───────▼───────┐    ┌─────▼──────┐
│ 解释器   │    │ JIT (RFC-028)│    │ LLVM AOT   │
│ 直接消费  │    │ 生成轻量     │    │ IR元数据    │
│ IR元数据  │    │ 调试表       │    │ → DWARF   │
└─────────┘    └───────────────┘    └────────────┘
```

**关键设计决策**：

1. **DAP 服务器与运行时通过 trait 解耦**。服务器不关心底层是解释器还是 JIT——它只通过 `DebugEngine` trait 发号施令。每个引擎独立实现同一个 trait。
2. **`yaoxiang run --debug` 强制使用解释器**。调试需要可控性，不需要性能。LLVM 模式下仅生成 DWARF 用于事后回溯（core dump / crash report），不做交互式调试。
3. **从 `yaoxiang run` 复用入口发现逻辑**。不引入新子命令，心智模型就是"以调试模式运行我的程序"。

### IR 调试元数据

在现有 YaoXiang IR 上附加元数据，不新增 IR 种类。所有元数据在**编译前端**一处生成，后端消费是只读的：

| 元数据 | 附着点 | 说明 |
|--------|--------|------|
| `SourceLocation` | 每个 IR 节点 | 源文件:行号:列号 |
| `VarName` | 变量声明/绑定节点 | 源码中的变量名 |
| `TypeAnnotation` | 变量/表达式节点 | 推断出的类型（含编译期谓词） |
| `ScopeBoundary` | 块/函数入口出口 | 变量作用域的生命周期 |
| `SpanInfo` | spawn 节点 | spawn 块内任务边界 |

### 启动流程

```
yaoxiang run --debug file.yx
    │
    ├── Phase 1: 编译（带调试元数据）
    │   ├── 解析 → AST
    │   ├── 类型检查 + 编译期谓词验证
    │   └── 降级到 IR（附加调试元数据）
    │
    ├── Phase 2: 使用解释器引擎
    │   └── --debug 模式下无论是否 --release，都使用解释器
    │
    ├── Phase 3: 启动 DAP 服务器
    │   ├── 初始化 stdio 传输通道
    │   ├── 等待 VS Code attach
    │   ├── attach 成功后暂停在程序入口
    │   └── 转入交互调试循环
    │
    └── Phase 4: 程序结束 / 调试会话结束 → 退出
```

### 各模式差异

| 模式 | 调试方式 |
|------|---------|
| `yaoxiang run --debug` | 强制解释器，全功能 DAP 交互调试 |
| `yaoxiang run --release` | 生成 DWARF，用于事后回溯（core dump / crash report），不启动 DAP |
| `yaoxiang run`（普通） | 无调试元数据，无调试支持 |

## 详细设计

### 1. 断点

```
断点类型：
├── 源码行断点    → 编译前端生成位置元数据，后端查询匹配
├── 函数入口断点  → 函数调用时触发（第二阶段）
├── 条件断点      → 表达式求值为 true 时触发
└── 数据断点      → 变量被修改时触发（第二阶段）
```

**源码行断点核心逻辑**：

```
VS Code 发送: "在 file.yx:42 设置断点"
    │
DAP 服务器:
    ├── 查询 IR 中所有 SourceLocation == (file.yx, 42) 的节点
    ├── 转发给运行时: "在这些 IR 地址暂停"
    └── 运行时返回: 断点 ID

程序运行到 IR 节点 → 运行时检查: 这个节点在断点列表里吗？
    ├── 普通断点 → 暂停，通知 DAP 服务器
    └── 条件断点 → 求值条件表达式 → true 才暂停
```

**三个引擎实现**：

| | 解释器 | JIT | LLVM |
|---|---|---|---|
| 断点插入方式 | 执行循环中检查 IR 节点 ID | JIT 在机器码中插 `int3` | 利用 LLVM DWARF + 硬件断点 |
| 条件断点求值 | 直接解释表达式 | 临时 JIT 编译条件表达式 | DWARF 表达式栈 + 求值 |
| 性能开销 | 每个 IR 节点多一次查表 | 仅断点处有开销 | 几乎零开销（硬件断点） |

### 2. 单步执行

```
Step Over    → 执行当前行，跳过函数调用内部，停在下一行
Step Into    → 进入当前行的函数调用内部
Step Out     → 执行到当前函数返回
Continue     → 恢复执行直到下一个断点或程序结束
```

**实现逻辑**：单步操作本质上是**临时断点**。和用户显式设的断点共享同一套机制，不是两个系统，是一个系统的两种用法。

```
Step Over:
    当前源码行号 = query_line(frame)
    → 设置临时断点在下一行
    → 如果当前行是函数调用: 在调用点之后设临时断点
    → Continue → 命中临时断点 → 删除 → 暂停

Step Into:
    当前调用目标的第一行可执行位置
    → 找到函数体第一个 IR 节点的源码位置
    → 设置临时断点 → Continue → 命中 → 暂停

Step Out:
    当前栈帧的返回地址
    → 找到 caller 的下一行
    → 设置临时断点 → Continue → 命中 → 暂停
```

**临时断点的四个边界情况处理**：

1. **并发归属**：临时断点绑定到当前任务 ID，其他任务命中后直接忽略。
2. **Step Over spawn 块**：在 spawn 外部按 Step Over 等于跑完整个 spawn 块，跳转到之后。进入 spawn 内部调试应使用 Step Into。
3. **临时断点未命中**：设看门狗超时（30 秒无任何断点命中）→ 强制暂停 → 通知 VS Code。同时监听程序退出事件 → 立即清理。
4. **同一行多个 IR 节点**：Step Over 的临时断点标记 `ignore_current_line`，命中后若源码行号等于当前行号 → 忽略，继续。

### 3. 变量检查与作用域

```
VS Code 请求: "当前帧的变量列表"
    │
DAP 服务器:
    ├── 查询当前暂停点的 IR 节点
    ├── 遍历当前 ScopeBoundary 内的变量绑定
    │   └── 每个绑定返回: (名称, 类型, 运行时值引用)
    └── 组装 VariablesResponse → VS Code
```

**作用域分层**：

```
┌─ Globals ───────────────────────────┐
│  模块级绑定: 常量、类型别名、全局   │
├─ Locals ────────────────────────────┤
│  当前函数内可见的局部变量           │
│  ├── 参数 (函数参数)                │
│  └── 局部绑定 (let / 赋值)          │
├─ Captured ──────────────────────────┤
│  spawn 块 / 闭包捕获的外部变量      │
│  显示所有权状态: 已 move / ref 共享  │
└─────────────────────────────────────┘
```

**引擎差异**：

| 引擎 | 变量值获取 |
|------|-----------|
| 解释器 | 直接读 VM 栈帧和堆。每个值在内存里有明确表示 |
| JIT | 寄存器和栈上的值 → 需要 JIT 编译时记录"变量 → 寄存器/栈槽"映射表 |
| LLVM | DWARF 的 `.debug_info` 段 → `DW_AT_location` → LLDB 原生支持 |

**特殊类型显示**：编译期谓词精化类型展示对调试有价值的信息：

```
x: Positive(x)  →  显示 "Int (x > 0 = True)"
y: Sorted(y)    →  显示 "Array(Int) (排序保证)"
result: T       →  显示运行时的具体类型
```

### 4. 调用栈

```
DAP 请求: StackTrace
    │
返回:
┌──────────────────────────────────┐
│ #0  process_item()  file.yx:42  │  ← 当前暂停点
│     locals: item = "hello"       │
│     spawn 任务 ID: task-3        │
├──────────────────────────────────┤
│ #1  main()          file.yx:67  │  ← caller
│     locals: data = ["hello", ...]│
├──────────────────────────────────┤
│ #2  <entry>         file.yx:1   │  ← 根
└──────────────────────────────────┘
```

每个帧记录：函数签名、调用位置（源文件+行号）、局部变量（延迟求值）、spawn 上下文（任务 ID）。

解释器和 JIT 各自维护帧链表。帧不是零成本获取的——但调试模式不追求零开销。

### 5. 表达式求值（Watch / REPL）

用户在断点处输入任意 YaoXiang 表达式：

```
Watch: x + y         → 返回计算结果
Watch: items[2].name → 访问复杂结构
Watch: f(x)          → 调用函数（有副作用风险）
```

**求值策略**：

```
用户输入表达式
    │
├── 编译器前端解析表达式
├── 在当前帧上下文中做类型检查
├── 变量值从当前帧获取（只读引用）
├── 表达式作为独立微程序执行
│   └── 不允许修改外部变量
│   └── 不允许 spawn
│   └── 不允许 IO（或可选开启）
└── 返回结果值 → 原始帧状态完全不变
```

**解释器天然沙箱**：表达式求值不是新建沙箱——解释器本身就是沙箱。表达式求值只是临时 push 一个帧，用完就销毁。和正常执行共享同一个 VM，但不提交任何副作用。

**函数调用求值**：默认允许，但警告用户"此表达式可能有副作用"，需用户确认后执行。

**引擎差异**：

| 引擎 | 表达式求值 |
|------|-----------|
| 解释器 | 复用现有 eval 代码路径，注入当前帧环境 |
| JIT | 临时编译表达式 → 链接到当前帧 → 执行 → 废弃临时代码 |
| LLVM | 不支持——LLVM 模式不做交互调试 |

### 6. 并发调试

**任务模型可见性**：

DAP 的 `threads` 概念映射为 YaoXiang 的 `spawn` 任务。每个任务有自己的栈帧链表和运行状态。

```
┌─ Threads ───────────────────────────┐
│  ● task-1  main()     file.yx:10   │ ← 当前聚焦
│  ▶ task-2  fetch()    file.yx:34   │ ← 运行中
│  ⏸ task-3  process()  file.yx:56   │ ← 断点暂停
│  ◼ task-4  write()    已结束         │
└─────────────────────────────────────┘
```

**断点在并发上下文**：

| 暂停模式 | 行为 | 适用场景 |
|----------|------|---------|
| `stop-all`（默认） | 一个任务命中 → 所有任务暂停 | 调试数据竞争、全局状态 |
| `stop-this-only` | 只暂停命中任务，其他继续 | 调试独立任务逻辑 |

**spawn 块的单步语义**：

```
spawn {          // Step Over → 跑完整个 spawn 块
    task_a()     // Step Into → 进入 task_a
    task_b()     // 并行跑，不受单独 step 影响
}
```

### 7. DAP 协议映射

#### 第一阶段：核心请求

| DAP 请求 | YaoXiang 语义 |
|----------|--------------|
| `initialize` | 能力协商：支持断点、单步、变量、栈帧 |
| `launch` / `attach` | 启动/附加到 YaoXiang 程序（`--debug` 走 attach 模式） |
| `setBreakpoints` | 设置源码行断点 |
| `configurationDone` | 断点就绪，开始执行 |
| `threads` | 返回所有活跃 spawn 任务列表 |
| `stackTrace` | 返回指定任务的栈帧列表 |
| `scopes` | 返回当前帧的变量作用域 |
| `variables` | 返回指定作用域的变量列表 |
| `continue` | 恢复执行 |
| `next` | Step Over |
| `stepIn` | Step Into |
| `stepOut` | Step Out |
| `pause` | 中断所有任务 |
| `evaluate` | 在当前帧求值表达式 |
| `disconnect` | 结束调试会话 |

#### 第二阶段：增强请求

| DAP 请求 | YaoXiang 语义 |
|----------|--------------|
| `setFunctionBreakpoints` | 函数名断点 |
| `setExceptionBreakpoints` | 错误/panic 时暂停 |
| `dataBreakpointInfo` | 数据断点（变量修改触发） |

## 实现策略

### 阶段零：基础设施（前置于所有阶段）

**目标**：编译前端附加调试元数据到 IR。

| 组件 | 改动 |
|------|------|
| IR 定义 | 新增 `SourceLocation`、`VarName`、`TypeAnnotation` 等元数据字段 |
| Parser | 每个 AST 节点记录源码位置 |
| TypeChecker | 类型信息附着到 IR 节点 |
| 测试 | 验证 IR dump 包含位置和变量信息 |

**不涉及运行时。**

### 第一阶段：解释器 DAP MVP

**目标**：`yaoxiang run --debug file.yx` 可以设置断点、单步、查看变量。

| 组件 | 改动 |
|------|------|
| DAP 服务器 (yx-core 新模块) | stdio 传输层、核心请求处理、断点管理器（源码行 → IR 节点映射） |
| 运行时调试 trait (yx-core) | `DebugEngine` trait 定义（pause, resume, step, get_frames, eval, get_variables） |
| 解释器 | 执行循环中断点检查、暂停/恢复机制、帧链表维护、`InterpreterDebugEngine` 实现 |
| CLI | `yaoxiang run --debug` 参数 |

**验收标准**：对 `tests/yaoxiang/` 下任意 `.yx` 文件，能用 VS Code 设置断点、Step Over、看变量值。

### 第二阶段：高级调试能力

**目标**：表达式求值、函数断点、并发调试、异常断点。

| 组件 | 改动 |
|------|------|
| 表达式求值引擎 | 微程序编译（复用 parser + typechecker）、临时帧推入 VM、副作用隔离 |
| 并发调试 | spawn 任务列表映射、断点绑定任务 ID、stop-all / stop-this-only 暂停策略 |
| 函数/异常断点 | `setFunctionBreakpoints`、`setExceptionBreakpoints` 映射 |
| VS Code 扩展 | 提供默认的 `launch.json` 模板 |

### 第三阶段：JIT 调试 & LLVM DWARF

**目标**：JIT 引擎复用 DAP，LLVM 产出 DWARF 用于崩溃回溯。

| 组件 | 改动 |
|------|------|
| JIT | 实现 `DebugEngine` trait、编译时生成变量→寄存器映射表、运行时帧链表、表达式临时编译 |
| LLVM | IR 调试元数据 → LLVM `DILocation` / `DISubprogram` → DWARF（无 DAP 交互） |

### 依赖关系

```
阶段零（IR 元数据）
    ↓
阶段一（解释器 DAP MVP）  ← 从这里开始就能用了
    ↓
阶段二（高级能力）
    ↓
阶段三（JIT + LLVM DWARF）
```

### 风险

| 风险 | 缓解 |
|------|------|
| 解释器暂停机制的复杂度 | 用简单的 channel/signal 而非复杂状态机，暂停就是不让 fetch 下一条指令 |
| 表达式求值的类型安全 | 复用现有 typechecker，只读引用，不提交副作用 |
| DAP 协议细节 | 参考 debugpy / delve 的实现，协议是成熟的 |
| 并发调试的 stop-all 活锁 | 超时机制 + 强制暂停 |

## 权衡

### 优点

- **一个源头**：调试元数据只生成一次，三个引擎共享。不会出现"解释器调试信息是对的但 LLVM 的错了"
- **零侵入**：`--debug` 一个参数，不加此参数的行为完全不变
- **DAP 标准**：直接接入 VS Code 生态，不需要自定义编辑器协议或调试器 UI
- **解释器优先**：调试天然适合解释器——灵活、可控、表达式求值简单。LLVM 模式不做交互调试是最务实的选择

### 缺点

- **调试模式性能差**：解释器比 JIT/LLVM 慢很多。但调试不需要性能——没人指望 debug 模式跑生产负载
- **LLVM 调试受限**：AOT 编译无法交互调试，只能用 GDB/LLDB + DWARF。但这是权衡：LLVM 模式下本就不该有调试行为差异
- **并发暂停复杂**：stop-all 语义在解释器上实现需要遍历所有活跃任务

### 替代方案

| 方案 | 为什么不选择 |
|------|-------------|
| 三种引擎各自实现 DAP | 三倍工作，三套 bug。违反"好品味" |
| 只用 DWARF，不搞自有 DAP | 解释器和 JIT 没有 DWARF 概念，LLDB 进不了 VM 内部 |
| 比照 Python pdb 做命令行调试器 | VS Code 体验完爆命令行调试器 |
| 把 DAP 塞进 LSP 进程 | 生命周期完全不同——LSP 跟随项目，DAP 跟随调试会话。进程隔离是硬需求 |

## 开放问题

- [ ] 条件断点的表达式语法与正常 YaoXiang 完全一致？（建议：完全一致，复用 parser）
- [ ] `spawn` 块内的 Step Into 行为：用户按下 Step Into 进入 spawn 块时，多个并行任务应展示哪个？（建议：暂停在第一个已创建的任务上）
- [ ] VS Code 扩展：调试配置是放在现有 `vscode-extension/` 目录下还是独立仓库？

## 参考文献

- [RFC-024：基于 spawn 块的并发模型](../accepted/024-concurrency-model.md)
- [RFC-027：编译期谓词与统一静态验证](../accepted/027-compile-time-evaluation-types.md)
- [RFC-028：JIT 编译器 — VM 内多级执行引擎](../draft/028-jit-compiler.md)
- [RFC-030：assert 断言机制](../review/030-assert-mechanism.md)
- [DAP 协议规范](https://microsoft.github.io/debug-adapter-protocol/)
- [debugpy — Python DAP 实现参考](https://github.com/microsoft/debugpy)
- [Delve — Go 调试器参考](https://github.com/go-delve/delve)

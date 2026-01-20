# Task 8.3: 嵌入式运行时（即时执行器）

> **优先级**: P1
> **状态**: ⬜ 待开始
> **模块**: `src/embedded/executor.rs`
> **依赖**: task-08-01-value-type, task-08-02-allocator

## 功能描述

实现嵌入式运行时的即时执行器，用于 WASM/游戏脚本等资源受限场景。

### 核心特性

- **即时执行**：读取字节码后立即执行，无调度开销
- **同步执行**：按顺序执行所有操作
- **无 DAG**：不使用依赖图
- **无调度器**：没有任务调度逻辑
- **忽略 spawn**：spawn 标记被当作普通函数调用

### 文件结构

```
src/embedded/
├── mod.rs              # 导出入口
└── executor.rs         # 即时执行器
```

## 实现内容

### 1. EmbeddedRuntime 结构

```rust
/// 嵌入式运行时
pub struct EmbeddedRuntime {
    /// 字节码解释器
    interpreter: Interpreter,
    /// 内存分配器
    allocator: BumpAllocator,
    /// 全局变量
    globals: HashMap<GlobalId, RuntimeValue>,
    /// 模块
    modules: HashMap<ModuleId, CompiledModule>,
    /// 函数表
    functions: Vec<CompiledFunction>,
}
```

### 2. Interpreter（字节码解释器）

```rust
struct Interpreter {
    ip: usize,              // 指令指针
    stack: Vec<RuntimeValue>, // 操作数栈
    call_stack: Vec<Frame>,  // 调用栈
}

struct Frame {
    func_id: FunctionId,
    return_ip: usize,
    locals: Vec<RuntimeValue>,
}
```

### 3. 指令执行（所有操作码）

需要实现完整的字节码指令集，包括：
- 基础操作：Nop, Const, Load, Store
- 算术运算：I64Add, I64Sub, I64Mul, I64Div, F64Add...
- 比较运算：I64Eq, I64Lt...
- 函数调用：CallStatic, CallIndirect, Return
- 控制流：Branch, BranchIf, BranchTable
- 数据操作：GetField, SetField, NewStruct, NewEnum...
- 模式匹配：Match
- 并发相关：Spawn（忽略，视为普通调用）, Await（不存在）

### 4. 错误处理

```rust
enum RuntimeError {
    StackUnderflow,
    InvalidLocal(usize),
    InvalidField(usize),
    TypeMismatch,
    FunctionNotFound(FunctionId),
    MissingMain,
    UnsupportedOpcode(TypedOpcode),
    DivisionByZero,
    // ...
}
```

## 验收测试

```rust
#[test]
fn test_basic_execution() {
    let mut runtime = EmbeddedRuntime::new(64 * 1024);
    let result = runtime.load_and_run(hello_world_module);
    assert!(result.is_ok());
}

#[test]
fn test_spawn_ignored() {
    // spawn 标记在嵌入式模式下被忽略
    let mut runtime = EmbeddedRuntime::new(64 * 1024);
    let result = runtime.load_and_run(spawn_module);
    // 同步执行，结果不是 Async 值
    assert_eq!(result, Ok(RuntimeValue::Int(42)));
}

#[test]
fn test_all_opcodes() {
    // 测试所有指令的执行
}
```

## 与 RFC-008 对照

| RFC-008 设计 | 实现 |
|-------------|------|
| 即时执行器 | ✅ EmbeddedRuntime |
| 同步执行 | ✅ 顺序执行 |
| 无 DAG | ✅ 不构建依赖图 |
| 无调度器 | ✅ 解释器直接执行 |
| spawn 忽略 | ✅ 视为普通调用 |

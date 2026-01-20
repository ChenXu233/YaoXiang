# Task 12.1: 执行器

> **优先级**: P0
> **状态**: ✅ 已实现

## 功能描述

虚拟机的核心执行引擎，负责解释执行字节码。

## 执行器结构

```rust
/// 虚拟机执行器
struct VM {
    /// 字节码
    bytecode: Bytecode,
    /// 寄存器文件
    regs: RegisterFile,
    /// 栈帧栈
    frames: Vec<Frame>,
    /// 当前帧索引
    frame_index: usize,
    /// 程序计数器
    pc: usize,
    /// VM 配置
    config: VMConfig,
    /// 错误信息
    error: Option<VMError>,
}

/// 寄存器文件
struct RegisterFile {
    /// 通用寄存器
    regs: Vec<RuntimeValue>,
}

/// VM 配置
struct VMConfig {
    /// 初始栈大小
    stack_size: usize,
    /// 最大调用深度
    max_call_depth: usize,
    /// 启用调试模式
    trace_execution: bool,
}
```

## 与 Runtime 的关系

```rust
// 使用 Runtime 提供的值类型
use crate::runtime::value::RuntimeValue;

// 使用 Runtime 提供的外部函数
use crate::runtime::extfunc;
```

## 执行循环

```rust
impl VM {
    /// 执行字节码
    pub fn execute_module(&mut self, module: &CompiledModule) -> VMResult<()> {
        // 加载主函数
        // 执行主函数
        // 返回结果
    }

    /// 执行单条指令
    fn step(&mut self) -> VMResult<()> {
        // 获取指令
        let opcode = self.fetch_opcode()?;

        // 执行指令
        self.execute(opcode)?;

        // 检查中断
        self.check_interrupt()?;

        Ok(())
    }
}
```

## 外部函数调用

```rust
impl VM {
    /// 调用外部函数
    fn call_external(&mut self, name: &str, args: &[RuntimeValue]) -> VMResult<RuntimeValue> {
        if let Some(ext_func) = EXTERNAL_FUNCTIONS.get(name) {
            Ok((ext_func.func)(args))
        } else {
            Err(VMError::ExternalFunctionNotFound(name.to_string()))
        }
    }
}
```

## 相关文件

- `src/vm/executor.rs` - 执行器实现
- `src/vm/mod.rs` - 模块入口
- `src/runtime/extfunc.rs` - 外部函数

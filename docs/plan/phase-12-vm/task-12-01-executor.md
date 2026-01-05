# Task 12.1: 执行器

> **优先级**: P0
> **状态**: ⚠️ 需重构

## 功能描述

虚拟机的核心执行引擎，负责解释执行字节码。

## 执行器结构

```rust
/// 虚拟机执行器
struct VM {
    /// 字节码
    bytecode: Bytecode,
    /// 寄存器
    registers: Vec<Value>,
    /// 栈帧栈
    frames: Vec<Frame>,
    /// 当前帧索引
    frame_index: usize,
    /// 程序计数器
    pc: usize,
    /// 运行时接口
    runtime: RuntimeInterface,
    /// VM 配置
    config: VMConfig,
}

struct VMConfig {
    /// 最大栈大小
    max_stack_size: usize,
    /// 最大帧深度
    max_frame_depth: usize,
    /// 启用调试模式
    debug_mode: bool,
    /// 启用 JIT（可选）
    enable_jit: bool,
}
```

## 执行循环

```rust
impl VM {
    /// 执行字节码
    pub fn run(&mut self) -> VMResult<Value> {
        loop {
            // 获取指令
            let instruction = self.fetch_instruction()?;

            // 译码
            let opcode = instruction.opcode;

            // 执行
            self.execute(opcode, &instruction)?;

            // 检查中断
            self.check_interrupt()?;

            // 检查是否结束
            if self.is_halted() {
                break;
            }
        }

        self.pop_result()
    }

    fn fetch_instruction(&self) -> Result<&Instruction, VMError> {
        let pc = self.pc;
        self.bytecode.get(pc).ok_or(VMError::InvalidPC(pc))
    }
}
```

## 相关文件

- `src/vm/executor.rs`

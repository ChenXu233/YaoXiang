# Task 13.2: 机器码生成

> **优先级**: P1
> **状态": ⏳ 待实现

## 功能描述

将字节码编译为机器码。

## 机器码生成器

```rust
/// 机器码生成器
struct MachineCodeGenerator {
    /// 目标平台
    target: Target,
    /// 代码缓冲区
    code_buffer: CodeBuffer,
    /// 寄存器分配器
    reg_alloc: RegisterAllocator,
    /// 指令选择器
    isel: InstructionSelector,
}

impl MachineCodeGenerator {
    /// 编译函数
    pub fn compile_function(&mut self, bytecode: &FunctionBytecode) -> CompiledCode {
        // 1. 构建控制流图
        let cfg = self.build_cfg(bytecode);

        // 2. 寄存器分配
        let reg_alloc = self.allocate_registers(&cfg);

        // 3. 指令选择
        let masm = self.select_instructions(&cfg, &reg_alloc);

        // 4. 汇编
        let code = self.assemble(&masm);

        CompiledCode {
            code,
            entry_point: code.ptr(),
        }
    }
}
```

## 相关文件

- **machine_code.rs**: MachineCodeGenerator

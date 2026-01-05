# Task 12.2: 栈帧管理

> **优先级**: P0
> **状态**: ⚠️ 需重构

## 功能描述

管理函数调用的栈帧，包括局部变量和操作数栈。

## 栈帧结构

```rust
/// 栈帧
struct Frame {
    /// 函数信息
    function: FunctionInfo,
    /// 返回地址
    return_address: usize,
    /// 返回值寄存器
    return_register: Option<Reg>,
    /// 局部变量
    locals: Vec<Value>,
    /// 操作数栈
    operand_stack: Vec<Value>,
    /// 捕获变量
    captures: Vec<Value>,
    /// 调用者帧索引
    caller_frame: Option<usize>,
}

struct FunctionInfo {
    /// 函数名称
    name: String,
    /// 参数数量
    param_count: usize,
    /// 局部变量数量
    local_count: usize,
    /// 最大栈深度
    max_stack_depth: usize,
    /// 函数入口地址
    entry_pc: usize,
}
```

## 栈帧操作

```rust
impl Frame {
    /// 推送操作数
    pub fn push(&mut self, value: Value) {
        self.operand_stack.push(value);
    }

    /// 弹出操作数
    pub fn pop(&mut self) -> Option<Value> {
        self.operand_stack.pop()
    }

    /// 获取局部变量
    pub fn get_local(&self, index: Reg) -> &Value {
        &self.locals[index as usize]
    }

    /// 设置局部变量
    pub fn set_local(&mut self, index: Reg, value: Value) {
        self.locals[index as usize] = value;
    }
}
```

## 相关文件

- `src/vm/frame.rs`

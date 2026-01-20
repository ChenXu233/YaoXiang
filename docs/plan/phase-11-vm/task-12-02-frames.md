# Task 12.2: 栈帧管理

> **优先级**: P0
> **状态**: ✅ 已实现

## 功能描述

管理函数调用的栈帧，包括局部变量和操作数栈。

## 栈帧结构

```rust
/// 栈帧
#[derive(Debug, Clone)]
struct Frame {
    /// 函数名称
    name: String,
    /// 返回地址
    return_addr: usize,
    /// 保存的帧指针
    saved_fp: usize,
    /// 局部变量（使用 RuntimeValue）
    locals: Vec<RuntimeValue>,
}
```

## 与 Runtime 的关系

```rust
// 使用 Runtime 提供的值类型
use crate::runtime::value::RuntimeValue;
```

## 栈帧操作

```rust
impl Frame {
    /// 创建新帧
    pub fn new(
        name: String,
        return_addr: usize,
        saved_fp: usize,
        locals: Vec<RuntimeValue>,
    ) -> Self {
        Self {
            name,
            return_addr,
            saved_fp,
            locals,
        }
    }

    /// 获取局部变量
    pub fn get_local(&self, index: usize) -> Option<&RuntimeValue> {
        self.locals.get(index)
    }

    /// 设置局部变量
    pub fn set_local(&mut self, index: usize, value: RuntimeValue) {
        if let Some(loc) = self.locals.get_mut(index) {
            *loc = value;
        }
    }
}
```

## 相关文件

- `src/vm/frames.rs` - 栈帧实现

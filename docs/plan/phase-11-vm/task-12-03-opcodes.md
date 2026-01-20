# Task 12.3: 指令集

> **优先级**: P0
> **状态**: ✅ 已实现（TypedOpcode）

## 功能描述

定义和实现所有字节码指令。

## 指令分类

| 分类 | 前缀 | 说明 |
|------|------|------|
| 控制流 | `Nop`, `Jump`, `Return` | 基本控制 |
| 函数 | `Call`, `CallNative`, `ReturnValue` | 函数调用 |
| 局部变量 | `LoadLocal`, `StoreLocal` | 局部变量访问 |
| 常量 | `LoadConst` | 加载常量 |
| 运算 | `Add`, `Sub`, `Mul`, ... | 算术运算 |
| 比较 | `Eq`, `Ne`, `Lt`, ... | 比较运算 |
| 类型 | `Cast`, `TypeCheck` | 类型操作 |
| 并发 | `Spawn`, `Await` | 并发原语 |

## TypedOpcode 枚举

```rust
/// 字节码操作码（强类型）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TypedOpcode {
    // 控制流
    Nop = 0x00,
    Return = 0x01,
    ReturnValue = 0x02,
    Jump = 0x03,
    JumpIfFalse = 0x04,
    JumpIfTrue = 0x05,

    // 函数调用
    Call = 0x10,
    CallNative = 0x11,
    CallMethod = 0x12,

    // 局部变量
    LoadLocal = 0x20,
    StoreLocal = 0x21,
    LoadConst = 0x22,
    LoadField = 0x23,
    StoreField = 0x24,

    // 算术运算
    Add = 0x30,
    Sub = 0x31,
    Mul = 0x32,
    Div = 0x33,
    Mod = 0x34,
    Neg = 0x35,

    // 比较运算
    Eq = 0x40,
    Ne = 0x41,
    Lt = 0x42,
    Le = 0x43,
    Gt = 0x44,
    Ge = 0x45,

    // 类型操作
    Cast = 0x50,
    TypeCheck = 0x51,

    // 并发原语
    Spawn = 0x60,
    Await = 0x61,

    // 对象操作
    NewObject = 0x70,
    NewArray = 0x71,
    NewMap = 0x72,

    // 内存操作
    Alloc = 0x80,
    Free = 0x81,
    LoadPtr = 0x82,
    StorePtr = 0x83,

    // 扩展（用于自定义操作）
    Extension0 = 0xE0,
    Extension1 = 0xE1,
    Extension2 = 0xE2,
    Extension3 = 0xE3,

    // 保留
    Reserved = 0xFF,
}
```

## 相关文件

- `src/vm/opcode.rs` - 操作码实现

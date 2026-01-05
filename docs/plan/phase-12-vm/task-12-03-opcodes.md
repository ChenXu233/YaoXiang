# Task 12.3: 指令集

> **优先级**: P0
> **状态**: ⚠️ 需重构

## 功能描述

定义和实现所有字节码指令。

## 指令分类

| 分类 | 前缀 | 说明 |
|------|------|------|
| 常量 | `CONST` | 加载常量 |
| 加载 | `LOAD` | 加载变量 |
| 存储 | `STORE` | 存储变量 |
| 算术 | `ADD`, `SUB`, ... | 算术运算 |
| 比较 | `EQ`, `NE`, `LT`, ... | 比较运算 |
| 逻辑 | `AND`, `OR`, `NOT` | 逻辑运算 |
| 控制 | `JUMP`, `JMPF`, `JMPT` | 控制流 |
| 函数 | `CALL`, `RET` | 函数调用 |
| 对象 | `NEW`, `GET`, `SET` | 对象操作 |
| 并发 | `SPAWN`, `AWAIT` | 并发原语 |

## 指令格式

```rust
/// 字节码指令
struct Instruction {
    /// 操作码
    opcode: Opcode,
    /// 操作数
    operands: Vec<Operand>,
    /// 指令位置
    pc: usize,
}

enum Opcode {
    // 常量指令
    Const(ConstValue),

    // 加载/存储指令
    Load(Reg, LoadSource),
    Store(Reg, StoreTarget),

    // 算术指令
    Add(Reg, Reg, Reg),        // dst = src1 + src2
    Sub(Reg, Reg, Reg),
    Mul(Reg, Reg, Reg),
    Div(Reg, Reg, Reg),
    Mod(Reg, Reg, Reg),
    Neg(Reg, Reg),             // dst = -src

    // 比较指令
    Eq(Reg, Reg, Reg),         // dst = src1 == src2
    Ne(Reg, Reg, Reg),
    Lt(Reg, Reg, Reg),
    Le(Reg, Reg, Reg),
    Gt(Reg, Reg, Reg),
    Ge(Reg, Reg, Reg),

    // 逻辑指令
    And(Reg, Reg, Reg),
    Or(Reg, Reg, Reg),
    Not(Reg, Reg),

    // 控制流指令
    Jump(Label),
    JumpIfFalse(Reg, Label),

    // 函数指令
    Call {
        func: FunctionRef,
        args: Vec<Reg>,
        result: Option<Reg>,
    },
    Return(Option<Reg>),

    // 对象指令
    New(HandleType, ResultReg),
    GetField(Reg, FieldName, ResultReg),
    SetField(Reg, FieldName, Reg),
    Index(Reg, Reg, ResultReg),

    // 并发指令
    Spawn(FunctionRef, TaskIdReg),
    Await(TaskIdReg, ResultReg),
}
```

## 相关文件

- `src/vm/opcodes.rs`

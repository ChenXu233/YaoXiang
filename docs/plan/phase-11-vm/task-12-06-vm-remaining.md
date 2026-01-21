# Task 12-06: VM 剩余指令实现

> **优先级**: 🔴 高
> **状态**: 🚧 进行中
> **预估工时**: 2-3 天

## 问题背景

在 task-12-05 完成后，VM 核心指令已实现，但仍需完善数据结构操作指令。

---

## 一、指令实现状态总览

### ✅ 已完整实现

| 类别 | 数量 | 指令 |
|------|------|------|
| 基础控制 | 6 | `Nop`, `Return`, `ReturnValue`, `Switch`, `Label`, `TailCall` |
| 跳转指令 | 3 | `Jmp`, `JmpIf`, `JmpIfNot` |
| 循环控制 | 2 | `LoopStart`, `LoopInc` |
| 数据移动 | 5 | `Mov`, `LoadConst`, `LoadLocal`, `StoreLocal`, `LoadArg` |
| I64 运算 | 16 | Add/Sub/Mul/Div/Rem/And/Or/Xor/Shl/Sar/Shr/Neg + Load/Store/Const + 6比较 |
| I32 运算 | 16 | Add/Sub/Mul/Div/Rem/And/Or/Xor/Shl/Sar/Shr/Neg + Load/Store/Const |
| F64 运算 | 11 | Add/Sub/Mul/Div/Rem/Sqrt/Neg + Load/Store/Const + 6比较 |
| F32 运算 | 16 | Add/Sub/Mul/Div/Rem/Sqrt/Neg + Load/Store/Const + 6比较 |
| 字符串 | 6 | `StringLength`, `StringConcat`, `StringEqual`, `StringGetChar`, `StringFromInt`, `StringFromFloat` |
| 元素访问 | 1 | `LoadElement` |
| 函数调用 | 1 | `CallStatic` |

### ⚠️ 简化处理（需完整实现）

| # | 指令 | 编码 | 说明 |
|---|------|------|------|
| 1 | `StoreElement` | 0x78 | 列表元素存储 |
| 2 | `GetField` | 0x75 | 结构体字段读取 |
| 3 | `SetField` | 0x76 | 结构体字段写入 |
| 4 | `NewListWithCap` | 0x7A | 带容量列表创建 |
| 5 | `MakeClosure` | 0x83 | 闭包创建 |
| 6 | `LoadUpvalue` | 0x84 | 加载闭包变量 |
| 7 | `ArcNew` | 0x7B | Arc 创建 |
| 8 | `ArcClone` | 0x7C | Arc 克隆 |
| 9 | `ArcDrop` | 0x7D | Arc 释放 |
| 10 | `Cast` | 0xC1 | 类型转换 |

### ❌ 废弃/移除指令

| 指令 | 编码 | 原因 |
|------|------|------|
| `TryBegin/TryEnd/Throw/Rethrow` | 0xA0-0xA3 | 使用 Result + ?，无异常机制 |
| `CallVirt/CallDyn` | 0x81-0x82 | CallStatic 已满足当前需求 |
| `StackAlloc/HeapAlloc` | 0x72-0x73 | 所有权模型未完善，暂缓 |

---

## 二、实现优先级

### 🔴 P0 - 编译器后端必需

| 指令 | 原因 |
|------|------|
| `StoreElement` | 列表写入，编译器生成 for 循环需要 |
| `NewListWithCap` | 列表字面量编译需要 |
| `GetField/SetField` | 结构体/元组字段访问 |

**验收标准**：
- [ ] StoreElement 完整实现（支持 List 类型）
- [ ] NewListWithCap 使用指定容量
- [ ] GetField 支持 List/Object/Tuple
- [ ] SetField 支持 List/Object

### 🟡 P1 - 函数式编程

| 指令 | 原因 |
|------|------|
| `MakeClosure` | 匿名函数/lambda 支持 |
| `LoadUpvalue` | 闭包变量捕获 |

### 🟢 P2 - 高级特性

| 指令 | 原因 |
|------|------|
| `ArcNew/Clone/Drop` | 所有权模型，`ref` 关键字支持 |
| `Cast` | 数值类型转换（Int ↔ Float） |

---

## 三、实现方案

### 3.1 StoreElement 指令

**操作数**：[array_reg, index_reg, src_reg]

```rust
StoreElement => {
    let array_reg = self.read_u8()?;
    let index_reg = self.read_u8()?;
    let src_reg = self.read_u8()?;

    let value = self.regs.read(src_reg).clone();
    match self.regs.read(array_reg) {
        RuntimeValue::List(items) => {
            // 简化：先转 vec 再修改
            let mut new_items = items.to_vec();
            let idx = match self.regs.read(index_reg) {
                RuntimeValue::Int(n) => *n as usize,
                _ => return Err(VMError::TypeError("int".into())),
            };
            if idx >= new_items.len() {
                return Err(VMError::IndexOutOfBounds { index: idx, size: new_items.len() });
            }
            new_items[idx] = value;
            self.regs.write(array_reg, RuntimeValue::List(new_items));
        }
        _ => return Err(VMError::TypeError("list".into())),
    }
    Ok(())
}
```

### 3.2 GetField/SetField 指令

**GetField 操作数**：[dst, obj_reg, field_offset]

```rust
GetField => {
    let dst = self.read_u8()?;
    let obj_reg = self.read_u8()?;
    let offset = self.read_u16()? as usize;

    match self.regs.read(obj_reg).clone() {
        RuntimeValue::List(items) => {
            if let Some(val) = items.get(offset) {
                self.regs.write(dst, val.clone());
            }
        }
        RuntimeValue::Tuple(items) => {
            if let Some(val) = items.get(offset) {
                self.regs.write(dst, val.clone());
            }
        }
        _ => return Err(VMError::TypeError("list/tuple/object".into())),
    }
    Ok(())
}
```

### 3.3 NewListWithCap 指令

**操作数**：[dst, capacity]

```rust
NewListWithCap => {
    let dst = self.read_u8()?;
    let capacity = self.read_u16()? as usize;
    self.regs.write(dst, RuntimeValue::List(Vec::with_capacity(capacity)));
    Ok(())
}
```

---

## 四、测试计划

| 测试类型 | 覆盖指令 | 说明 |
|---------|---------|------|
| 列表操作 | StoreElement, LoadElement, NewListWithCap | 元素读写、容量 |
| 结构体 | GetField, SetField | 字段访问 |
| 闭包 | MakeClosure, LoadUpvalue | 环境捕获 |

---

## 五、语言设计一致性

### 5.1 异常处理（移除）

YaoXiang 使用 **Result + ?** 运算符，无异常机制：

```yaoxiang
# 正确方式
process: () -> Result[Int, String] = () => {
    data = fetch_data()?  # ? 自动传播错误
}
```

### 5.2 引用计数（Arc 指令）

`ArcNew/Clone/Drop` 与 `ref` 关键字对齐：

```yaoxiang
p: Point = Point(1.0, 2.0)
shared = ref p  # 创建 Arc
```

### 5.3 内存分配

所有权模型下：
- 默认 **Move**（零拷贝）
- `ref` 关键字触发堆分配（Arc）
- 栈分配优先，小对象不堆分配

---

## 六、风险与依赖

| 风险 | 缓解措施 |
|------|---------|
| StoreElement 涉及所有权 | 当前用 clone，后续可优化 |
| Arc 实现依赖所有权模型 | 先实现基础功能 |
| 闭包环境捕获复杂度 | 先支持简单场景 |

---

## 参考资料

- [src/vm/opcode.rs](src/vm/opcode.rs) - 指令定义
- [src/vm/executor.rs](src/vm/executor.rs) - 执行器
- [language-spec.md](../../design/language-spec.md) - 语言规范
- [RFC-009 所有权模型](../../design/accepted/009-ownership-model.md)

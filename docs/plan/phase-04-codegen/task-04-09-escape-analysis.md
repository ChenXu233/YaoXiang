# Task 4.9: 逃逸分析集成

> **优先级**: P2
> **状态**: ✅ 已实现

## 功能描述

与逃逸分析模块集成，优化内存分配策略：栈分配 vs 堆分配。

## 设计原则

**逃逸分析是编译器优化阶段**，在字节码生成之前完成。字节码层面只需要：
- 使用 `StackAlloc` 分配确定不逃逸的值
- 使用 `HeapAlloc` 分配可能逃逸的值
- 运行时检查（可选）：在不确定时插入逃逸检查

## 字节码指令（复用现有指令）

| Opcode | 值 | 操作 | 说明 |
|--------|-----|------|------|
| `StackAlloc` | 0x70 | 栈上分配 | size (u16) |
| `HeapAlloc` | 0x71 | 堆分配 | dst, type_id (u16) |
| `Drop` | 0x72 | 释放所有权 | reg |

**说明**：`StackAlloc` 用于**确定不逃逸**的值，`HeapAlloc` 用于**可能逃逸**的值。

## 集成机制

逃逸分析在 IR → 字节码转换之前完成：

```rust
// src/middle/codegen/mod.rs

fn generate_function(
    &mut self,
    func: &FunctionIR,
    code_section: &mut CodeSection,
) -> Result<(), CodegenError> {
    self.current_function = Some(func.clone());
    self.register_allocator = RegisterAllocator::new();

    // 运行逃逸分析（在生成字节码之前）
    let mut escape_analyzer = EscapeAnalyzer::new();
    self.escape_analysis = Some(escape_analyzer.analyze_function(func));

    // 生成函数体
    let instructions = self.generate_instructions(func)?;

    // 清除逃逸分析结果（避免影响下一个函数）
    self.escape_analysis = None;
    ...
}
```

字节码生成器根据逃逸分析结果选择分配指令：

| EscapeInfo | 分配策略 | 字节码指令 |
|------------|----------|-----------|
| 不逃逸、不捕获、不修改 | 栈分配 | `StackAlloc` |
| 不逃逸、捕获 | 栈分配 + 关闭 | `StackAlloc` + `CloseUpvalue` |
| 逃逸 | 堆分配 | `HeapAlloc` |
| 可能逃逸（不确定） | 堆分配（保守） | `HeapAlloc` |
| 可变且不逃逸 | 栈分配 + 写时复制 | `StackAlloc` |

## 当前实现状态

```rust
// src/middle/codegen/stmt.rs

/// 检查变量是否需要堆分配（综合考虑类型和逃逸分析）
fn should_heap_allocate_for_var(
    &self,
    local_idx: usize,
    ty: &MonoType,
) -> bool {
    // 1. 首先检查逃逸分析结果
    if let Some(ref escape) = self.escape_analysis {
        let local_id = LocalId::new(local_idx);
        if escape.should_heap_allocate(local_id) {
            return true;
        }
    }

    // 2. 根据类型决定（回退策略）
    self.should_heap_allocate_for_type(ty)
}
```

**说明**：集成工作流程：
1. `generate_function` 调用 `EscapeAnalyzer::analyze_function(func)`
2. 结果存储在 `CodegenContext.escape_analysis`
3. `generate_var_decl` 调用 `should_heap_allocate_for_var` 检查逃逸分析结果
4. 决定使用 `StackAlloc` 或 `HeapAlloc`

## 生成规则

### 栈分配优化（不逃逸）
```yaoxiang
fn foo() {
    x = 42  # x 不逃逸，栈分配
    bar(x)
    # x 在此处释放
}
```
生成字节码（优化后）：
```
StackAlloc 8 -> r1     # 栈上分配 8 bytes
CONST 42 -> r2
StoreLocal r2, x       # 或直接 StoreElement r1, 0, r2
LOAD x -> r3
CallStatic bar(r3)
Drop r1                # 函数结束，栈帧释放时自动 Drop
```

### 逃逸到外部（堆分配）
```yaoxiang
fn may_escape() -> List[Int] {
    data = [1, 2, 3]  # data 逃逸（作为返回值），堆分配
    return data
}
```
生成字节码：
```
# 逃逸分析确定 data 会逃逸，使用 HeapAlloc
HeapAlloc r1, type_id=List[Int]

# 填充列表
CONST 1 -> r2
StoreElement r1, r2, ???
CONST 2 -> r3
StoreElement r1, r3, ???
CONST 3 -> r4
StoreElement r1, r4, ???

ReturnValue r1
```

### 被闭包捕获（需要关闭 Upvalue）
```yaoxiang
fn create_closure() {
    x = 42
    closure = || x + 1  # x 被捕获，可能逃逸
    call(closure)
}
```
生成字节码：
```
StackAlloc 8 -> r1     # 初始栈分配 x
CONST 42 -> r2
StoreLocal r2, x

# 创建闭包
MakeClosure r3, func_id=closure_closure, upvalue_count=1

# 关闭 upvalue（x 从栈搬迁到堆）
CloseUpvalue r1

CallStatic call(r3)
```

### 运行时逃逸检查（保守策略）
```yaoxiang
fn process() {
    data = [1, 2, 3]  # 编译时无法确定是否逃逸
    if condition {
        return data  # 逃逸
    }
    sum(data)        # 不逃逸
}
```
生成字节码（保守，假设逃逸）：
```
# 编译时无法确定，使用堆分配（保守但正确）
HeapAlloc r1, type_id=List[Int]

# 填充列表
CONST 1 -> r2
StoreElement r1, r2, ???
CONST 2 -> r3
StoreElement r1, r3, ???
CONST 3 -> r4
StoreElement r1, r4, ???

# 条件检查
LOAD condition -> r5
JmpIfNot r5, no_return

# 返回 data（逃逸路径）
ReturnValue r1

no_return:
CallStatic sum(r1)
Drop r1  # data 不需要，释放
```

### 可变局部变量（栈分配）
```yaoxiang
fn counter() -> () {
    count = 0  # 可变但不逃逸
    count = count + 1
    print(count)
}
```
生成字节码：
```
StackAlloc 8 -> r1     # 栈上分配 count
CONST 0 -> r2
StoreLocal r2, count

CONST 1 -> r3
LOAD count -> r4
I64Add r4, r3 -> r5
StoreLocal r5, count

LOAD count -> r6
CallStatic print(r6)
Drop r1                # 栈帧释放时自动 Drop
```

## 验收测试

```yaoxiang
# test_escape_analysis.yx

# 不逃逸 - 栈分配优化
fn local_only() {
    x = compute_value()  # 栈分配优化
    process(x)
    # x 在此处释放
}

# 逃逸到返回值 - 堆分配
fn may_escape() -> List[Int] {
    data = [1, 2, 3]
    return data  # data 逃逸，堆分配
}

# 闭包捕获 - 堆分配
fn create_closure() {
    x = 42
    closure = || x + 1  # x 被捕获，堆分配
    call(closure)
}

# 条件逃逸 - 保守策略
fn conditional_escape(do_return: Bool) -> Option[List[Int]] {
    data = [1, 2, 3]
    if do_return {
        some(data)  # 可能逃逸，堆分配
    } else {
        none
    }
}

# 循环中的逃逸分析
fn collect_in_loop(n: Int) -> List[Int] {
    result = []
    for i in 0..n {
        item = i * 2
        result = result + [item]  # result 逃逸
    }
    return result
}

print("Escape analysis integration tests passed!")
```

## 相关文件

- **src/vm/opcode.rs**: TypedOpcode 枚举定义（StackAlloc, HeapAlloc, Drop, CloseUpvalue）
- **src/middle/escape.rs**: 逃逸分析模块
- **src/middle/codegen/bytecode.rs**: BytecodeInstruction 结构
- **src/middle/codegen/generator.rs**: 逃逸分析集成逻辑

# 闭包调用修复方案

> **状态**: 🔄 进行中
>
> **目标**: 修复闭包调用，使高阶函数（map/filter/reduce）能正常工作

## 一、问题背景

### 1.1 当前问题

当用户使用高阶函数时：

```yao
std.list.map([1, 2], x => x * 2)
```

会报错：`"Cannot call YaoXiang functions from this native context"`

### 1.2 根因分析

问题有两层：

#### 问题 1：MakeClosure 生成的 func_id 错误

**文件**：`src/backends/interpreter/executor.rs` 第 999-1020 行

```rust
BytecodeInstr::MakeClosure {
    dst,
    func: func_ref,
    env: _,
} => {
    let func_name = ...;
    let func_id = crate::backends::common::value::FunctionId(
        self.functions
            .get(&func_name)
            .map(|_| self.functions.len() as u32)  // ❌ 错误！
            .unwrap_or(0),
    );
    let closure = RuntimeValue::Function(FunctionValue {
        func_id,
        env: Vec::new(),  // env 被忽略
    });
    ...
}
```

问题：`self.functions.len()` 是 HashMap 的当前大小，不是函数的实际索引。

#### 问题 2：没有通过 func_id 调用函数的机制

- `Interpreter` 使用 `HashMap<String, BytecodeFunction>` 存储函数
- 没有 `Vec<ByteCodeFunction>` 按索引存储
- 无法通过 `func_id` 查找并调用函数

#### 问题 3：NativeContext 没有传入 call_fn 回调

在 `CallStatic` 和 `CallNative` 处理中：

```rust
let mut ctx = NativeContext::new(&mut self.heap);  // ❌ 没有 call_fn
let result = self.ffi.call(&func_name, &call_args, &mut ctx)?;
```

这导致 `ctx.call_function()` 返回错误。

---

## 二、修复方案

### 2.1 方案设计

需要三个改动：

| 改动 | 文件 | 描述 |
|------|------|------|
| A | executor.rs | 添加 `Vec<BytecodeFunction>` 函数表 |
| B | executor.rs | 修复 MakeClosure，使用正确的函数索引 |
| C | executor.rs | 添加 `call_function_by_id` 方法 + 在调用 native 时传入回调 |

### 2.2 详细设计

#### 改动 A：添加函数表

```rust
// src/backends/interpreter/executor.rs

pub struct Interpreter {
    // ... 现有字段 ...
    /// Function table by index (for closure calls)
    functions_by_id: Vec<BytecodeFunction>,
}
```

加载模块时，同时填充两个结构：

```rust
// 加载模块时
for func in &module.functions {
    self.functions.insert(func.name.clone(), func.clone());
    self.functions_by_id.push(func.clone());  // 按顺序添加
}
```

#### 改动 B：修复 MakeClosure

```rust
BytecodeInstr::MakeClosure { ... } => {
    let func_name = ...;

    // 找到函数在 Vec 中的索引
    let func_id = if let Some((idx, _)) = self.functions_by_id
        .iter()
        .enumerate()
        .find(|(_, f)| f.name == func_name)
    {
        FunctionId(idx as u32)
    } else {
        FunctionId(0)  // fallback
    };

    let closure = RuntimeValue::Function(FunctionValue {
        func_id,
        env: Vec::new(),  // TODO: 后续实现 env 捕获
    });
    ...
}
```

#### 改动 C：实现 call_fn 回调

```rust
// 在 Interpreter 中添加方法
impl Interpreter {
    /// 通过 func_id 调用 YaoXiang 函数
    fn call_function_by_id(
        &mut self,
        func_id: FunctionId,
        args: &[RuntimeValue],
    ) -> Result<RuntimeValue, ExecutorError> {
        let idx = func_id.0 as usize;
        if idx >= self.functions_by_id.len() {
            return Err(ExecutorError::FunctionNotFound(format!(
                "Function with id {} not found",
                idx
            )));
        }
        let func = &self.functions_by_id[idx];
        self.execute_function(func, args)
    }
}
```

然后在调用 native 函数时传入回调：

```rust
// CallStatic / CallNative 处理中
let mut ctx = NativeContext::new(&mut self.heap);

// 创建回调闭包
let interp_ptr = std::ptr::addr_of_mut!(*self);
let call_fn = move |func: &RuntimeValue, args: &[RuntimeValue]| -> Result<RuntimeValue, ExecutorError> {
    // 从 func 提取 func_id 并调用
    if let RuntimeValue::Function(fv) = func {
        let mut interpreter = unsafe { &mut *interp_ptr };
        interpreter.call_function_by_id(fv.func_id, args)
    } else {
        Err(ExecutorError::Type("Expected function value".to_string()))
    }
};

let mut ctx = NativeContext::with_call_fn(&mut self.heap, call_fn);
let result = self.ffi.call(&func_name, &call_args, &mut ctx)?;
```

---

## 三、验收标准

### 3.1 编译验收

- [ ] `cargo check` 通过
- [ ] `cargo build` 通过

### 3.2 功能验收

- [ ] `std.list.map([1, 2], x => x * 2)` 返回 `[2, 4]`
- [ ] `std.list.filter([1, 2, 3], x => x > 1)` 返回 `[2, 3]`
- [ ] `std.list.reduce([1, 2, 3], (acc, x) => acc + x, 0)` 返回 `6`

### 3.3 边界验收

- [ ] 空列表的高阶函数调用正常工作
- [ ] 闭包捕获外部变量正常工作（后续实现）
- [ ] 嵌套函数调用正常工作

---

## 四、测试方案

### 4.1 单元测试

创建测试文件 `tests/closure.yx`：

```yao
// 测试 map
let doubled = std.list.map([1, 2, 3], x => x * 2);
assert(std.list.get(doubled, 0) == 2);
assert(std.list.get(doubled, 1) == 4);
assert(std.list.get(doubled, 2) == 6);

// 测试 filter
let filtered = std.list.filter([1, 2, 3, 4, 5], x => x > 2);
assert(std.list.len(filtered) == 3);

// 测试 reduce
let sum = std.list.reduce([1, 2, 3, 4], (acc, x) => acc + x, 0);
assert(sum == 10);

// 测试链式调用
let result = std.list.map(
    std.list.filter([1, 2, 3, 4], x => x % 2 == 0),
    x => x * 10
);
assert(std.list.get(result, 0) == 20);
assert(std.list.get(result, 1) == 40);
```

### 4.2 运行测试

```bash
# 编译项目
cargo build

# 运行测试
cargo run -- tests/closure.yx

# 或者使用 yaoxiang cli
yaoxiang run tests/closure.yx
```

---

## 五、风险与回滚

### 5.1 风险

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| 改动影响现有函数调用 | 可能破坏 CallStatic | 逐步测试，每步编译 |
| 回调闭包生命周期 | 借用检查复杂 | 使用原始指针方案 |

### 5.2 回滚方案

```bash
git checkout -- src/backends/interpreter/executor.rs
```

---

## 六、时间估算

| 改动 | 预计时间 |
|------|----------|
| 改动 A：添加函数表 | 30 分钟 |
| 改动 B：修复 MakeClosure | 20 分钟 |
| 改动 C：实现回调 + 测试 | 1 小时 |
| **总计** | **1.5-2 小时** |

---

## 七、后续工作

完成本次修复后，可进一步优化：

1. **闭包 env 捕获**：实现 `MakeClosure` 中的 `env` 字段
2. **TailCall 优化**：添加尾调用优化
3. **性能优化**：缓存函数查找结果

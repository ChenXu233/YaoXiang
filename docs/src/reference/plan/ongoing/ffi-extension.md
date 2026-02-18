# FFI 扩展设计方案

## 一、背景与目标

### 1.1 现状

当前 FFI 架构：

```rust
type NativeHandler = fn(&[RuntimeValue]) -> Result<RuntimeValue, ExecutorError>;
```

**问题**：
- native 函数无法访问 heap，无法返回 List/Dict
- native 函数无法调用用户传入的 YaoXiang 函数（高阶函数无法实现）
- 解释器中散落着硬编码的特殊处理（len, dict_keys 等）

### 1.2 目标

1. 让 native 函数能访问 heap，返回 List/Dict
2. 让 native 函数能调用 YaoXiang 函数（支持高阶函数）
3. 统一架构，消除解释器硬编码

---

## 二、总体设计

### 2.1 核心类型定义

```rust
// 执行上下文 - 传递给 native 函数
pub struct NativeContext<'a> {
    /// 堆内存管理
    pub heap: &'a mut Heap,
    /// 解释器（用于调用 YaoXiang 函数）
    pub interpreter: &'a mut Interpreter,
}

// Native 函数签名变更
pub type NativeHandler = fn(args: &[RuntimeValue], ctx: &mut NativeContext) -> Result<RuntimeValue, ExecutorError>;
```

### 2.2 模块结构

```
src/backends/interpreter/
├── ffi.rs          # 修改：NativeHandler 类型，调用约定
└── executor.rs    # 修改：调用 native 时构造 Context

src/std/
├── mod.rs         # 修改：NativeHandler 类型定义
├── io.rs          # 修改：所有函数签名
├── math.rs        # 修改：所有函数签名
├── string.rs      # 修改：实现 heap 访问
├── list.rs        # 修改：实现 heap 访问 + 高阶函数
├── dict.rs        # 修改：实现 heap 访问
└── ... 其他模块   # 修改：所有函数签名
```

### 2.3 调用流程

```
用户代码调用 native 函数
    ↓
BytecodeExecutor 执行 CallNative
    ↓
从 FFIRegistry 获取 NativeHandler
    ↓
构造 NativeContext { heap, interpreter }
    ↓
调用 handler(args, &mut ctx)
    ↓
handler 内部可以：
  - 访问 heap 分配/修改 List/Dict
  - 调用 interpreter.call_fn() 执行用户函数
    ↓
返回 RuntimeValue
```

---

## 三、详细实施步骤

### 步骤 1：修改 FFI 类型定义

**文件**：`src/std/mod.rs`

**改动内容**：
1. 添加 `NativeContext` 结构体定义
2. 修改 `NativeHandler` 类型别名
3. 修改 `NativeExport` 结构体（可选）

**验收标准**：
- [ ] `NativeContext` 结构体包含 `heap` 和 `interpreter` 字段
- [ ] `NativeHandler` 类型为 `fn(args: &[RuntimeValue], ctx: &mut NativeContext) -> Result<RuntimeValue, ExecutorError>`
- [ ] 编译通过

**测试方案**：
- 编译测试：`cargo check` 通过

---

### 步骤 2：修改 FFI Registry

**文件**：`src/backends/interpreter/ffi.rs`

**改动内容**：
1. 修改 `register()` 方法签名
2. 修改 `call()` 方法，调用时传入 ctx

**验收标准**：
- [ ] `register(name, handler)` 接受新签名的 handler
- [ ] `call(name, args, ctx)` 将 ctx 传递给 handler
- [ ] 编译通过

**测试方案**：
- 编译测试：`cargo check` 通过

---

### 步骤 3：修改解释器调用点

**文件**：`src/backends/interpreter/executor.rs`

**改动内容**：
1. 找到 `CallNative` 字节码处理位置（约 600 行）
2. 在调用 native 函数前构造 `NativeContext`
3. 将 ctx 传递给 `ffi.call()`

**验收标准**：
- [ ] 调用 native 函数时创建 NativeContext
- [ ] NativeContext 包含有效的 heap 引用
- [ ] NativeContext 包含有效的 interpreter 引用
- [ ] 编译通过

**测试方案**：
- 编译测试：`cargo check` 通过

---

### 步骤 4：更新 std.io 模块

**文件**：`src/std/io.rs`

**改动内容**：
1. 更新所有 native 函数签名
2. 添加 `ctx` 参数

**涉及函数**：
- `native_print`
- `native_println`
- `native_read_line`
- `native_read_file`
- `native_write_file`
- `native_append_file`

**验收标准**：
- [ ] 所有函数签名符合新 `NativeHandler` 类型
- [ ] 函数内部不使用 ctx（向后兼容）
- [ ] 编译通过

**测试方案**：
- [ ] `std.io.print("test")` 正常工作
- [ ] `std.io.println("test")` 正常工作

---

### 步骤 5：更新 std.math 模块

**文件**：`src/std/math.rs`

**改动内容**：
1. 更新所有 native 函数签名
2. 添加 `ctx` 参数

**涉及函数**：
- `native_abs`, `native_max`, `native_min`, `native_clamp`
- `native_fabs`, `native_fmax`, `native_fmin`, `native_pow`
- `native_sqrt`, `native_floor`, `native_ceil`, `native_round`
- `native_sin`, `native_cos`, `native_tan`
- `native_pi`, `native_e`, `native_tau`

**验收标准**：
- [ ] 所有函数签名符合新类型
- [ ] 编译通过

**测试方案**：
- [ ] `std.math.abs(-5)` 返回 5
- [ ] `std.math.sqrt(4)` 返回 2

---

### 步骤 6：实现 std.string 完整功能

**文件**：`src/std/string.rs`

**改动内容**：
1. 修改函数签名
2. 实现 heap 访问，返回真正的 List

**涉及函数**：
| 函数 | 实现方式 |
|------|----------|
| `split` | 使用 ctx.heap 分配 List |
| `chars` | 使用 ctx.heap 分配 List |
| `trim/upper/lower/replace` | 已实现（无需 heap）|
| `contains/starts_with/ends_with` | 已实现（无需 heap）|

**验收标准**：
- [ ] `std.string.split("a,b", ",")` 返回 `["a", "b"]`
- [ ] `std.string.chars("abc")` 返回 `["a", "b", "c"]`
- [ ] 编译通过

**测试方案**：
```yao
// 测试 split
let result = std.string.split("hello,world", ",");
assert(std.list.len(result) == 2);

// 测试 chars
let chars = std.string.chars("abc");
assert(std.list.len(chars) == 3);
```

---

### 步骤 7：实现 std.list 完整功能（含高阶函数）

**文件**：`src/std/list.rs`

**改动内容**：
1. 修改所有函数签名
2. 实现 heap 访问
3. 实现高阶函数调用

**涉及函数**：

| 函数 | 实现方式 |
|------|----------|
| `push` | 使用 ctx.heap 分配新 List |
| `pop` | 从 heap 取元素 |
| `prepend` | 使用 ctx.heap 分配新 List |
| `reverse` | 使用 ctx.heap 分配新 List |
| `concat` | 使用 ctx.heap 分配新 List |
| `map` | **调用用户函数** |
| `filter` | **调用用户函数** |
| `reduce` | **调用用户函数** |
| `get/set/first/last/slice` | heap 访问 |

**高阶函数实现要点**：
```rust
fn native_map(args: &[RuntimeValue], ctx: &mut NativeContext) -> Result<RuntimeValue, ExecutorError> {
    // args[0] 是列表，args[1] 是用户函数
    let list_handle = /* 从 args[0] 提取 */;
    let func_value = /* 从 args[1] 提取 */;

    // 获取列表元素
    let items = ctx.heap.get(list_handle).unwrap();
    let HeapValue::List(original_items) = items else { /* error */ };

    // 对每个元素调用用户函数
    let mut result_items = Vec::new();
    for item in original_items {
        // 调用 interpreter 执行用户函数
        let mapped = ctx.interpreter.call_fn_value(&func_value, &[item])?;
        result_items.push(mapped);
    }

    // 返回新列表
    let new_handle = ctx.heap.allocate(HeapValue::List(result_items));
    Ok(RuntimeValue::List(new_handle))
}
```

**验收标准**：
- [ ] `std.list.push([1, 2], 3)` 返回 `[1, 2, 3]`
- [ ] `std.list.pop([1, 2, 3])` 返回 `3` 和剩余 `[1, 2]`
- [ ] `std.list.map([1, 2], x => x * 2)` 返回 `[2, 4]`
- [ ] `std.list.filter([1, 2, 3], x => x > 1)` 返回 `[2, 3]`
- [ ] `std.list.reduce([1, 2, 3], (acc, x) => acc + x, 0)` 返回 `6`
- [ ] 编译通过

**测试方案**：
```yao
// 测试 push
let list1 = std.list.push([1, 2], 3);
assert(std.list.len(list1) == 3);

// 测试 map
let doubled = std.list.map([1, 2, 3], x => x * 2);
assert(std.list.get(doubled, 0) == 2);

// 测试 filter
let filtered = std.list.filter([1, 2, 3, 4], x => x > 2);
assert(std.list.len(filtered) == 2);

// 测试 reduce
let sum = std.list.reduce([1, 2, 3], (acc, x) => acc + x, 0);
assert(sum == 6);
```

---

### 步骤 8：实现 std.dict 完整功能

**文件**：`src/std/dict.rs`

**改动内容**：
1. 修改所有函数签名
2. 实现 heap 访问
3. 支持 Any 类型键

**涉及函数**：

| 函数 | 实现方式 |
|------|----------|
| `get` | 从 heap 取 Dict，查找键 |
| `set` | 使用 ctx.heap 分配新 Dict |
| `has` | 从 heap 取 Dict，检查键 |
| `keys/values/entries` | 使用 ctx.heap 分配 List |
| `delete` | 使用 ctx.heap 分配新 Dict |
| `merge` | 使用 ctx.heap 合并两个 Dict |

**验收标准**：
- [ ] `std.dict.get({a: 1}, "a")` 返回 `1`
- [ ] `std.dict.set({a: 1}, "b", 2)` 返回 `{a: 1, b: 2}`
- [ ] `std.dict.keys({a: 1, b: 2})` 返回 `["a", "b"]`
- [ ] `std.dict.has({a: 1}, "a")` 返回 `true`
- [ ] 编译通过

**测试方案**：
```yao
// 测试 get
let d = {name: "tom", age: 20};
assert(std.dict.get(d, "name") == "tom");

// 测试 set
let d1 = {a: 1};
let d2 = std.dict.set(d1, "b", 2);
assert(std.dict.has(d2, "b") == true);

// 测试 keys
let keys = std.dict.keys({x: 1, y: 2});
assert(std.list.len(keys) == 2);
```

---

### 步骤 9：更新其他 std 模块

**涉及文件**：
- `src/std/net.rs`
- `src/std/time.rs`
- `src/std/os.rs`
- `src/std/concurrent.rs`
- `src/std/weak.rs`
- `src/std/ffi.rs`（如有测试代码）

**改动内容**：
- 更新所有 native 函数签名，添加 ctx 参数
- 不需要使用 ctx 的函数可以忽略

**验收标准**：
- [ ] 所有 std 模块编译通过
- [ ] 现有功能不受影响

---

### 步骤 10：清理解释器硬编码

**文件**：`src/backends/interpreter/executor.rs`

**待移除代码**：
- `len()` 特殊处理（约 609-634 行）
- `dict_keys()` 特殊处理（约 637-666 行）

**注意**：
- 先完成步骤 6-8，确保 std 库函数正常工作
- 然后用 `std.list.len()` 替代内置 `len()`
- 用 `std.dict.keys()` 替代内置 `dict_keys()`

**验收标准**：
- [ ] 移除 len() 硬编码后，`len([1,2,3])` 仍然工作（通过 std.list.len）
- [ ] 移除 dict_keys() 硬编码后，`dict_keys({a:1})` 仍然工作（通过 std.dict.keys）
- [ ] 编译通过

---

## 四、测试方案

### 4.1 单元测试

在 `src/std/` 目录下添加测试：

```rust
#[cfg(test)]
mod tests {
    // string tests
    #[test]
    fn test_split() { ... }

    // list tests
    #[test]
    fn test_push() { ... }
    #[test]
    fn test_map() { ... }

    // dict tests
    #[test]
    fn test_get() { ... }
}
```

### 4.2 集成测试

创建测试文件 `tests/std_primitives.yx`：

```yao
// 字符串测试
let s1 = std.string.trim("  hello  ");
assert(s1 == "hello");

let s2 = std.string.split("a,b,c", ",");
assert(std.list.len(s2) == 3);

// 列表测试
let l1 = std.list.push([1, 2], 3);
assert(std.list.len(l1) == 3);

let doubled = std.list.map([1, 2, 3], x => x * 2);
assert(std.list.get(doubled, 0) == 2);

// 字典测试
let d = std.dict.set({a: 1}, "b", 2);
assert(std.dict.has(d, "b") == true);

// 高阶函数测试
let filtered = std.list.filter([1, 2, 3, 4, 5], x => x > 2);
assert(std.list.len(filtered) == 3);

let sum = std.list.reduce([1, 2, 3, 4], (acc, x) => acc + x, 0);
assert(sum == 10);
```

### 4.3 回归测试

确保现有功能不受影响：

```bash
# 运行现有测试
cargo test

# 运行集成测试
cargo run -- tests/std_primitives.yx
```

---

## 五、风险与回滚

### 5.1 风险

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| 改动量大 | 可能引入 bug | 分步骤，每步编译测试 |
| 破坏现有 native 函数 | 运行时错误 | 更新所有 std 模块签名 |
| 高阶函数调用复杂 | 实现难度高 | 参考现有 interpreter 调用逻辑 |

### 5.2 回滚方案

如果出问题，可以用 git 回滚：

```bash
git checkout -- src/std/ src/backends/interpreter/ffi.rs src/backends/interpreter/executor.rs
```

---

## 六、时间估算

| 步骤 | 预计时间 |
|------|----------|
| 步骤 1-3（FFI 核心） | 1-2 小时 |
| 步骤 4-5（更新 io/math）| 30 分钟 |
| 步骤 6（string 完整）| 30 分钟 |
| 步骤 7（list + 高阶函数）| 1-2 小时 |
| 步骤 8（dict）| 1 小时 |
| 步骤 9-10（清理）| 30 分钟 |
| **总计** | **5-6 小时** |

---

## 七、总结

**完成后能力**：

```yao
// 字符串
std.string.split("a,b,c", ",")  // ["a", "b", "c"]
std.string.chars("hi")          // ["h", "i"]

// 列表
std.list.push([1,2], 3)         // [1, 2, 3]
std.list.map([1,2], x => x*2)   // [2, 4]
std.list.filter([1,2,3], x => x>1)  // [2, 3]
std.list.reduce([1,2,3], (a,x)=>a+x, 0)  // 6

// 字典
std.dict.get({a:1}, "a")       // 1
std.dict.keys({a:1, b:2})      // ["a", "b"]
```

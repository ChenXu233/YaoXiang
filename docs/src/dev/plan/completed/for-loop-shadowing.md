# For 循环与遮蔽检查实现文档

## 概述

本文档描述 YaoXiang 语言中 for 循环变量可变性设计和遮蔽检查的实现方案。

**背景问题**：
- 当前 `for i in 1..5` 语法在 IR 阶段报错，因为循环变量未被标记为可变
- 语言需要实现禁止遮蔽（shadowing）规则
- 循环变量在循环内部的可变性需要显式声明，与 `let mut`的语法规则保持一致，但语言上没有 `let` 语法，因此需要禁止遮蔽。

## 设计原则

1. **禁止遮蔽**：任何命名空间（for、if、{}代码块等）中新声明的变量不能遮蔽外部已存在的变量
2. **显式可变性**：可变性必须通过 `mut` 关键字显式声明，循环变量默认不可变
3. **每次迭代新绑定**：for 循环的变量在每次迭代中创建新的绑定，不是修改同一个变量

## 实现内容

### 1. 禁止遮蔽检查

#### 1.1 类型检查阶段

在所有创建新变量的位置检测遮蔽：

- **for 循环**：检测循环变量名是否已在当前或外层作用域声明
- **let 声明**：检测变量名是否已在当前作用域声明
- **if/while 等块语句**：检测内部声明的变量是否遮蔽外部

#### 1.2 实现位置

修改以下文件：
- `src/frontend/typecheck/checking/mod.rs` - 添加遮蔽检测逻辑

#### 1.3 错误码

新增错误码 `E2xxx`（待确定）：
```
[E2xxx] 变量遮蔽错误
error: cannot shadow existing variable 'x'
 --> example.yx:3:5
  |
3 |     for x in 1..5 {
  |     ^ variable 'x' is already declared in outer scope
```

### 2. for 循环变量可变性

#### 2.1 语法设计

```yaoxiang
for i in 1..5 {      // i 不可变（默认）
    print(i)         // OK
    i = i + 1        // 错误：cannot assign to immutable variable
}

for mut i in 1..5 {  // i 可变
    i = i + 1        // OK
}
```

#### 2.2 语法分析

修改 `src/frontend/core/parser/statements/control_flow.rs`：
- 在解析 for 语句时，检查 `for` 关键字后是否有 `mut`
- 如果有 `mut`，记录到 AST 节点中

AST 结构变更：
```rust
StmtKind::For {
    var,           // 变量名
    var_mut: bool, // 新增：变量是否可变
    iterable,
    body,
    label,
}
```

#### 2.3 IR 生成

修改 `src/middle/core/ir_gen.rs` 的 `generate_for_loop_ir`：
- 如果 `var_mut` 为 true，将循环变量加入 `current_mut_locals`
- 这样 MutChecker 就会允许对该变量的重复赋值

```rust
// 在 generate_for_loop_ir 中
if var_mut {
    self.current_mut_locals.insert(var_reg);
}
```

## 实现效果（示例）

### 示例 1：基本 for 循环

```yaoxiang
// 输入
for i in 1..5 {
    print(i)
}

// 输出
1
2
3
4
```

### 示例 2：for 循环变量修改

```yaoxiang
// 输入
for mut i in 1..3 {
    i = i + 10
    print(i)
}

// 输出
11
12
13
```

### 示例 3：禁止遮蔽 - for 循环

```yaoxiang
// 输入
i = 10
for i in 1..5 {
    print(i)
}

// 错误输出
error [E2xxx] cannot shadow existing variable 'i'
 --> example.yx:2:5
  |
2 |     for i in 1..5 {
  |         ^ variable 'i' is already declared in outer scope
help: consider renaming the inner variable or outer variable to avoid shadowing
```

### 示例 4：{}代码块禁止遮蔽

```yaoxiang
// 输入
x = 1
{
    x = 2  // 错误！
    print(x)
}

// 错误输出
error [E2xxx] cannot shadow existing variable 'x'
 --> example.yx:3:1
  |
3 |     x = 2
  |     ^ variable 'x' is already declared in outer scope
help: consider renaming the inner variable or outer variable to avoid shadowing
```

### 示例 5：if 块中的遮蔽

```yaoxiang
// 输入
x = 1
if true {
    x = 2  // 错误！
    print(x)
}

// 错误输出
error [E2xxx] cannot shadow existing variable 'x'
 --> example.yx:4:5
  |
4 |         x = 2
  |         ^ variable 'x' is already declared in outer scope
help: consider renaming the inner variable or outer variable to avoid shadowing
```

### 示例 6：循环体内修改不可变变量

```yaoxiang
// 输入
for i in 1..5 {
    i = i + 1
}

// 错误输出
error [E2010] Cannot assign to immutable variable 'i'
 --> example.yx:2:5
  |
2 |     i = i + 1
  |     ^ cannot assign to immutable variable 'i'
help: Use 'mut' to declare a mutable variable
```

## 2. for 循环变量每次迭代新绑定

```yaoxiang
// 输入
for i in 1..3 {
    print(i)
}

// 这里面的 i 在每次迭代中都是一个新的绑定，因为每次循环体结束后循环体内的 i 都被销毁，下一次迭代时会创建一个新的 i 绑定，不是修改同一个变量。
```

## 验收方案

### 功能验收

1. **for 循环基础功能**
   - [ ] `for i in 1..5 { print(i) }` 正常输出 1-4
   - [ ] `for mut i in 1..5 { i = i + 1; print(i) }` 正常输出 2-5

2. **禁止遮蔽**
   - [ ] 外部存在变量时，for 循环使用同名变量报错
   - [ ] 外部存在变量时，声明同名变量报错
   - [ ] if/while 块内声明遮蔽外部变量报错
   - [ ] 跨函数作用域的遮蔽检测

3. **可变性检查**
   - [ ] for 循环变量默认不可变，修改报错
   - [ ] for mut 变量可修改，正常工作

### 错误信息验收

- [ ] 遮蔽错误信息清晰，指出被遮蔽的变量和位置
- [ ] 可变性错误信息与现有 E2010 风格一致

## 测试方案

### 单元测试

在 `src/frontend/typecheck/tests/` 中添加：

```rust
#[test]
fn test_for_loop_basic() {
    // 测试基本 for 循环
}

#[test]
fn test_for_loop_mut() {
    // 测试 for mut
}

#[test]
fn test_shadowing_for_loop() {
    // 测试 for 循环遮蔽检测
}

#[test]
fn test_shadowing_block() {
    // 测试遮蔽检测
}

#[test]
fn test_shadowing_if_block() {
    // 测试 if 块遮蔽检测
}
```

### 集成测试

在 `docs/src/tutorial/examples/` 中添加测试用例：

1. `test_for_loop.yx` - for 循环基本测试
2. `test_shadowing.yx` - 遮蔽检测测试

### 手动测试

```bash
# 测试基本功能
cargo run -- run docs/src/tutorial/examples/std_io_examples.yx

# 测试遮蔽报错
echo 'i = 10; for i in 1..5 { print(i) }' | cargo run -- run -
```

### 回归测试

确保现有测试不因本次修改而失败：
```bash
cargo test
```

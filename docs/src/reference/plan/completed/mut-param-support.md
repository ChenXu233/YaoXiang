# 函数参数 mut 语法支持实现计划

> **状态**：已实现
> **日期**：2026-02-19

---

## 概述

### 问题背景

当前 YaoXiang 语言的函数参数不支持 `mut` 关键字，参数默认不可变。这导致以下问题：

1. **闭包参数不可变**：如 `list.map([1,2,3], x => x * 2)` 中，闭包参数 `x` 不可变，闭包体内无法修改 `x`
2. **函数参数不可变**：常规函数参数也无法修改，无法实现"参数原地修改"的模式

### 目标

实现函数参数的 `mut` 语法支持，使函数参数可以声明为可变的。

### 语法设计

```yaoxiang
// 普通函数参数
fn foo(mut x: Int) -> Int {
    x = x + 1  // 合法，可以修改
    x
}

// Lambda 参数
f = (mut x) => x + 1

// 高阶函数调用
list.map([1, 2, 3], (mut x) => x * 2)  // 合法
```

---

## 错误码设计

### E2010 vs E2011 区分

| 错误码 | 描述 | 场景 |
|--------|------|------|
| E2010 | cannot assign to immutable variable | `x = 1; x = 2`（用户显式修改变量） |
| **E2011** | **closure parameter requires mut** | `list.map([..., x => ...])`（闭包参数需要 mut） |

### E2011 设计

**错误信息**：
```
error[E2011]: closure parameter '{param_name}' requires 'mut' to be modified
 --> example.yx:1:20
  |
1 | list.map([1,2,3], x => x * 2);
  |                    ^ consider adding 'mut' to parameter: (mut x) => ...
```

**触发条件**：
1. 闭包作为参数传递给高阶函数
2. 高阶函数内部尝试修改闭包参数
3. 闭包参数未声明 `mut`

**修复提示**：
- 建议将 `x => ...` 改为 `(mut x) => ...`

---

## 验收标准

### 解析器层

- [x] 解析器支持解析 `(mut x: Type)` 形式的参数
- [x] 解析器支持解析 `(mut x)` 形式的参数（省略类型）
- [x] 解析器支持 Lambda 的 `(mut x) => body` 语法

### AST 层

- [x] `Param` 结构体新增 `is_mut: bool` 字段
- [x] 类型检查正确识别可变参数

### 语义分析层

- [x] 类型检查正确处理可变参数
- [x] 可变参数可以在函数体内被修改

### IR 生成层

- [x] IR 生成器正确处理可变参数（注册为可变局部变量）
- [x] 闭包的可变参数正确传递到闭包函数

### 测试要求

- [x] 测试用例：带 mut 参数的普通函数
- [x] 测试用例：带 mut 参数的 Lambda
- [x] 测试用例：高阶函数中使用 mut 参数闭包
- [x] 测试用例：报错场景 - 不可变参数被修改

---

## 实现步骤

### Phase 1: AST 修改

#### 1.1 修改 Param 结构体

**文件**：`src/frontend/core/parser/ast.rs`

```rust
pub struct Param {
    pub name: String,
    pub ty: Option<Type>,
    pub is_mut: bool,  // 新增
    pub span: Span,
}
```

**验收**：
- [x] AST 中 Param 包含 is_mut 字段

---

### Phase 2: 解析器修改

#### 2.1 修改参数解析逻辑

**文件**：`src/frontend/core/parser/statements/declarations.rs`

**函数**：`parse_fn_params`

在解析参数名之前检测 `mut` 关键字：

```rust
// 检测 mut 关键字
let is_mut = state.skip(&TokenKind::KwMut);

let name = match state.current().map(|t| &t.kind) {
    Some(TokenKind::Identifier(n)) => n.clone(),
    _ => break,
};
state.bump();

// 解析类型注解
let ty = if state.skip(&TokenKind::Colon) {
    parse_type_annotation(state)
} else {
    None
};

params.push(Param {
    name,
    ty,
    is_mut,  // 新增
    span: param_span,
});
```

**验收**：
- [x] `(mut x: Int)` 正确解析
- [x] `(mut x)` 正确解析（无类型注解）
- [x] `(x: Int)` 解析为不可变（is_mut = false）

---

### Phase 3: 类型检查

#### 3.1 类型检查器传递 is_mut 信息

**文件**：`src/frontend/typecheck/`

需要在类型检查阶段将参数的可变性信息传递下去。

**验收**：
- [x] 类型检查通过可变参数的函数定义
- [x] 类型检查拒绝不可变参数被修改的代码

---

### Phase 4: IR 生成

#### 4.1 修改 generate_function_ir

**文件**：`src/middle/core/ir_gen.rs`

修改参数注册逻辑，根据 `is_mut` 决定是否注册为可变：

```rust
for (i, param) in params.iter().enumerate() {
    // 注册参数
    self.register_local(&param.name, i);
    // 只有 mut 参数才注册为可变
    if param.is_mut {
        self.current_mut_locals.insert(i);
    }
}
```

#### 4.2 修改 generate_lambda_body_ir

**文件**：`src/middle/core/ir_gen.rs`

同样修改闭包参数的处理：

```rust
for (i, param) in params.iter().enumerate() {
    self.register_local(&param.name, i);
    // 只有 mut 参数才注册为可变
    if param.is_mut {
        self.current_mut_locals.insert(i);
    }
}
```

**验收**：
- [x] 可变参数在 IR 中正确标记为可变
- [x] 闭包的可变参数正确传递

---

## 涉及文件

| 模块 | 文件 | 修改内容 |
|------|------|----------|
| AST | `src/frontend/core/parser/ast.rs` | Param 结构体新增 is_mut 字段 |
| 解析器 | `src/frontend/core/parser/statements/declarations.rs` | 参数解析支持 mut 关键字，函数类型注解识别 mut 参数 |
| 解析器 | `src/frontend/core/parser/statements/bindings.rs` | binding 参数解析支持 mut 关键字 |
| 解析器 | `src/frontend/core/parser/pratt/nud.rs` | 类型化参数列表支持 mut 前缀 |
| 解析器 | `src/frontend/core/parser/pratt/led.rs` | Lambda 参数转换支持 Expr::Lambda |
| 解析器 | `src/frontend/core/parser/pratt/mod.rs` | Lambda 参数添加 is_mut: false 默认值 |
| IR 生成 | `src/middle/core/ir_gen.rs` | 根据 is_mut 注册可变局部变量，修复 lambda 状态隔离 |
| 测试 | `src/frontend/typecheck/tests/*.rs` | 更新 Param 构造添加 is_mut 字段 |
| 测试 | `tests/mut_param_test.yx` | mut 参数通过场景测试 |
| 测试 | `tests/mut_param_error_test.yx` | 不可变参数修改报错测试 |

---

## 测试用例

### 通过场景

```yaoxiang
// 1. 普通函数的可变参数
fn increment(mut x: Int) -> Int {
    x = x + 1
}
main = {
    result = increment(5);
}

// 2. Lambda 的可变参数
main = {
    f = (mut x) => {
        x = x + 1
        x
    };
    result = f(5);
}

// 3. 高阶函数中使用可变参数闭包
use std.{io, list}
main = {
    result = list.map([1, 2, 3], (mut x) => {
        x = x * 2
        x
    });
}
```

### 报错场景

#### E2010 - 普通不可变变量修改

```yaoxiang
// 不可变参数被修改 - E2010
fn foo(x: Int) -> Int {
    x = x + 1  // E2010: cannot assign to immutable variable
}
```

#### E2011 - 闭包参数需要 mut（新增）

```yaoxiang
// 闭包参数未声明 mut - E2011
list.map([1, 2, 3], x => x * 2)
// E2011: closure parameter 'x' requires 'mut' to be modified
// help: consider adding 'mut' to parameter: (mut x) => ...

list.filter([1, 2, 3], x => x > 2)
// E2011: closure parameter 'x' requires 'mut' to be modified

list.reduce([1, 2, 3], (acc, x) => acc + x, 0)
// E2011: closure parameter 'acc' requires 'mut' to be modified
// E2011: closure parameter 'x' requires 'mut' to be modified
```
```

---

## 风险与注意事项

1. **向后兼容**：现有代码不写 `mut` 参数，保持不可变行为
2. **类型推导**：省略类型注解时 `(mut x)` 应能自动推断类型
3. **闭包场景**：确保可变参数正确传递到闭包函数体

---

## 实现过程中发现并修复的额外问题

### 1. Lambda IR 生成状态隔离

**问题**：`generate_lambda_body_ir` 在生成闭包函数体时会清空 `current_mut_locals` 和 `current_local_names`，导致父函数的可变性信息丢失。

**修复**：在进入 lambda body IR 生成前保存父函数状态（`current_mut_locals`、`current_local_names`、`next_temp`），退出后恢复。

### 2. Lambda 返回值寄存器冲突

**问题**：`generate_lambda_body_ir` 使用固定寄存器 0 作为返回值寄存器，与参数寄存器 0 冲突，导致 MutChecker 误报。

**修复**：改为使用 `self.next_temp_reg()` 分配独立的返回值寄存器。

### 3. 列表字面量 StoreIndex 可变性

**问题**：列表字面量 `[1, 2, 3]` 通过多次 `StoreIndex` 写入元素，第二次及后续写入被 MutChecker 视为对不可变变量的修改。

**修复**：在 `AllocArray` 后将列表临时寄存器注册为可变（`current_mut_locals.insert(result_reg)`）。

### 4. 函数类型注解中 `mut` 参数识别

**问题**：RFC-010 函数类型注解解析 `(mut x: Int) -> Int` 时，`looks_like_named_params` 检测无法识别 `KwMut` 开头的参数列表，导致被误判为旧语法。

**修复**：在两处 `looks_like_named_params` 检测中添加 `state.at(&TokenKind::KwMut)` 判断。

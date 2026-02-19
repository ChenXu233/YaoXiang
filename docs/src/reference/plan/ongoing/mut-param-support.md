# 函数参数 mut 语法支持实现计划

> **状态**：待实现
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

## 验收标准

### 解析器层

- [ ] 解析器支持解析 `(mut x: Type)` 形式的参数
- [ ] 解析器支持解析 `(mut x)` 形式的参数（省略类型）
- [ ] 解析器支持 Lambda 的 `(mut x) => body` 语法

### AST 层

- [ ] `Param` 结构体新增 `is_mut: bool` 字段
- [ ] 类型检查正确识别可变参数

### 语义分析层

- [ ] 类型检查正确处理可变参数
- [ ] 可变参数可以在函数体内被修改

### IR 生成层

- [ ] IR 生成器正确处理可变参数（注册为可变局部变量）
- [ ] 闭包的可变参数正确传递到闭包函数

### 测试要求

- [ ] 测试用例：带 mut 参数的普通函数
- [ ] 测试用例：带 mut 参数的 Lambda
- [ ] 测试用例：高阶函数中使用 mut 参数闭包
- [ ] 测试用例：报错场景 - 不可变参数被修改

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
- [ ] AST 中 Param 包含 is_mut 字段

---

### Phase 2: 解析器修改

#### 2.1 修改参数解析逻辑

**文件**：`src/frontend/core/parser/statements/declarations.rs`

**函数**：`parse_fn_params`

在解析参数名之前检测 `mut` 关键字：

```rust
// 检测 mut 关键字
let is_mut = state.skip(&TokenKind::Mut);

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
- [ ] `(mut x: Int)` 正确解析
- [ ] `(mut x)` 正确解析（无类型注解）
- [ ] `(x: Int)` 解析为不可变（is_mut = false）

---

### Phase 3: 类型检查

#### 3.1 类型检查器传递 is_mut 信息

**文件**：`src/frontend/typecheck/`

需要在类型检查阶段将参数的可变性信息传递下去。

**验收**：
- [ ] 类型检查通过可变参数的函数定义
- [ ] 类型检查拒绝不可变参数被修改的代码

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
- [ ] 可变参数在 IR 中正确标记为可变
- [ ] 闭包的可变参数正确传递

---

## 涉及文件

| 模块 | 文件 | 修改内容 |
|------|------|----------|
| AST | `src/frontend/core/parser/ast.rs` | Param 结构体新增 is_mut 字段 |
| 解析器 | `src/frontend/core/parser/statements/declarations.rs` | 参数解析支持 mut 关键字 |
| 类型检查 | `src/frontend/typecheck/inference/*.rs` | 传递参数可变性信息 |
| IR 生成 | `src/middle/core/ir_gen.rs` | 根据 is_mut 注册可变局部变量 |

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

```yaoxiang
// 不可变参数被修改 - 应报错
fn foo(x: Int) -> Int {
    x = x + 1  // E2010: cannot assign to immutable variable
}
```

---

## 风险与注意事项

1. **向后兼容**：现有代码不写 `mut` 参数，保持不可变行为
2. **类型推导**：省略类型注解时 `(mut x)` 应能自动推断类型
3. **闭包场景**：确保可变参数正确传递到闭包函数体

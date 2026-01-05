# Task 3.7: 缺陷修复

> **优先级**: P0
> **状态**: ⏳ 待实现

## 当前已知缺陷

### 缺陷 1: 函数定义返回类型检查不完整

**位置**: `infer.rs:278-315` (`infer_fn_def_expr`)

**问题描述**:
函数定义表达式未完整验证返回语句类型与声明返回类型的一致性。当前实现忽略了 body 返回类型与预期返回类型的约束。

**当前代码**:
```rust
fn infer_fn_def_expr(
    &mut self,
    params: &[ast::Param],
    return_type: Option<&ast::Type>,
    body: &ast::Block,
    _span: Span,
) -> TypeResult<MonoType> {
    // ...
    let _body_ty = self.infer_block(body, false, None)?;  // 忽略了返回类型检查！
    // ...
}
```

**修复方案**:
```rust
let body_ty = self.infer_block(body, false, Some(&return_ty))?;
// 现在会添加 return_ty == body_ty 的约束
```

---

### 缺陷 2: match 表达式模式推断简化

**位置**: `infer.rs:358-377` (`infer_match`)

**问题描述**:
match 表达式的类型推断未使用被匹配表达式的类型来约束模式。当前实现忽略了模式与表达式类型的关联。

**当前代码**:
```rust
fn infer_match(
    &mut self,
    expr: &ast::Expr,
    arms: &[ast::MatchArm],
    _span: Span,
) -> TypeResult<MonoType> {
    let _expr_ty = self.infer_expr(expr)?;  // 推断但未使用！
    // ...
}
```

**修复方案**:
```rust
let expr_ty = self.infer_expr(expr)?;
for arm in arms {
    let pattern_ty = self.infer_pattern(&arm.pattern, Some(&expr_ty))?;  // 传入 expr_ty
    self.solver.add_constraint(pattern_ty, result_ty.clone(), arm.span);
}
```

---

### 缺陷 3: 结构体/联合体模式匹配未完整实现

**位置**: `infer.rs:400-411` (`infer_pattern`)

**问题描述**:
`Pattern::Struct` 和 `Pattern::Union` 的处理返回简化类型变量，未验证字段是否匹配。

**修复方案**:
```rust
ast::Pattern::Struct { name, fields } => {
    // 从 env 获取结构体类型定义
    let struct_ty = self.get_type(name)?
        .instantiate(self.solver());

    // 验证模式字段与类型定义匹配
    // 收集字段绑定类型
    Ok(struct_ty)
}
```

---

### 缺陷 4: 泛型类型变量作用域管理

**位置**: `infer.rs` 多个方法

**问题描述**:
泛型参数在实例化时可能泄漏到外部作用域，导致类型污染。

**修复方案**:
- 在 `instantiate` 时创建新的类型变量域
- 确保泛型参数不泄漏到外部作用域

---

### 缺陷 5: 错误位置信息不完整

**位置**: `errors.rs`

**问题描述**:
某些错误缺少行号、列号等位置信息，或上下文信息不足。

**修复方案**:
- 为所有错误添加 span 信息
- 增加错误上下文提示
- 提供修复建议

---

## 验收测试

```yaoxiang
# test_defect_fixes.yx

# 缺陷 1 测试: 返回类型检查
fn add(a: Int, b: Int): Int = a + b
assert(add(1, 2) == 3)

# 缺陷 2 测试: match 类型推断
result = match Option::Some(42) {
    Some(n) => n * 2
    None => 0
}
assert(result == 84)

# 缺陷 3 测试: 结构体模式匹配
Point = struct { x: Int, y: Int }
p = Point(x: 1, y: 2)
match p {
    Point(x, y) => x + y
}
assert(true)

# 缺陷 4 测试: 泛型作用域
map[T](f: (T) -> Int, list: List[T]): Int = 0

# 缺陷 5 测试: 错误信息
# x: Int = "hello"  # 期望错误: expected `Int`, found `String`
#                              --> test.yaoxiang:5:11

print("Defect fix tests passed!")
```

## 修复优先级

| 缺陷 | 优先级 | 影响范围 |
|------|--------|----------|
| 缺陷 1 | P0 | 函数返回类型检查 |
| 缺陷 2 | P0 | match 类型推断 |
| 缺陷 3 | P1 | 结构体模式匹配 |
| 缺陷 4 | P1 | 泛型作用域 |
| 缺陷 5 | P2 | 错误信息质量 |

## 相关文件

- `src/frontend/typecheck/infer.rs`
- `src/frontend/typecheck/errors.rs`

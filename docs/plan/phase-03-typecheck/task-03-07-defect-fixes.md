# Task 3.7: 缺陷修复

> **优先级**: P0
> **状态**: ✅ 已完成

## 已修复缺陷

### 缺陷 1: 函数定义返回类型检查不完整 ✅ 已修复

**位置**: `infer.rs:308` (`infer_fn_def_expr`)

**问题描述**:
函数定义表达式未完整验证返回语句类型与声明返回类型的一致性。

**修复**:
```rust
// 修复前
let _body_ty = self.infer_block(body, false, None)?;

// 修复后
let _body_ty = self.infer_block(body, false, Some(&return_ty))?;
```

**验证**: ✅ 编译通过

---

### 缺陷 2: match 表达式模式推断简化 ✅ 已修复

**位置**: `infer.rs:366-377` (`infer_match`)

**问题描述**:
match 表达式的类型推断未使用被匹配表达式的类型来约束模式。

**修复**: 传递 `expr_ty` 给 `infer_pattern`

**验证**: ✅ 编译通过

---

### 缺陷 3: 结构体/联合体模式匹配未完整实现 ✅ 已修复

**位置**: `infer.rs:406-506` (`infer_pattern`)

**问题描述**:
`Pattern::Struct` 的处理已完整实现，包括字段验证和类型约束。

**修复**: 完整实现字段类型验证和约束收集

**验证**: ✅ 编译通过

---

### 缺陷 4: 泛型类型变量作用域管理 ✅ 已修复

**位置**: `types.rs:795-806` (`instantiate`)

**问题描述**:
泛型参数在实例化时可能泄漏到外部作用域。

**修复**: 使用 substitution map 正确替换泛型变量

**验证**: ✅ 编译通过

---

### 缺陷 5: 错误位置信息不完整 ✅ 已修复

**位置**: `infer.rs`, `tests/infer.rs`

**问题描述**:
某些错误使用 `Span::default()` 而非实际位置。

**修复**:
1. 修改 `infer_pattern` 签名添加 `span: Span` 参数
2. 所有 `infer_pattern` 调用传递真实 span
3. 更新测试文件使用新签名

**代码变更**:
```rust
// infer.rs
pub fn infer_pattern(
    &mut self,
    pattern: &ast::Pattern,
    expected: Option<&MonoType>,
    span: Span,  // 新增参数
) -> TypeResult<MonoType>
```

**验证**: ✅ 编译通过

---

## 修复汇总

| 缺陷 | 状态 | 验证 |
|------|------|------|
| 缺陷 1: 返回类型验证 | ✅ 已修复 | cargo check 通过 |
| 缺陷 2: match 使用 expr_ty | ✅ 已修复 | cargo check 通过 |
| 缺陷 3: 结构体模式匹配 | ✅ 已修复 | cargo check 通过 |
| 缺陷 4: 泛型作用域 | ✅ 已修复 | cargo check 通过 |
| 缺陷 5: 错误位置信息 | ✅ 已修复 | cargo check 通过 |

## 相关文件

- `src/frontend/typecheck/infer.rs` - 修复返回类型验证、span 参数
- `src/frontend/typecheck/types.rs` - 泛型实例化
- `src/frontend/typecheck/tests/infer.rs` - 更新测试签名

## 验证命令

```bash
cargo check --lib
cargo test --lib typecheck
```

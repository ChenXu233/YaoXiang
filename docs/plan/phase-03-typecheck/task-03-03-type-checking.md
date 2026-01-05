# Task 3.3: 类型检查

> **优先级**: P0
> **状态**: ⚠️ 部分实现

## 功能描述

在类型推断的基础上，检查类型注解和约束是否正确。

## 检查类型

| 检查项 | 说明 |
|--------|------|
| 类型注解匹配 | 检查声明类型与初始化表达式类型是否兼容 |
| 函数返回类型 | 检查 return 语句类型与函数返回类型是否一致 |
| 参数类型 | 检查函数调用参数类型与声明参数类型是否匹配 |
| 可变性约束 | 检查 mut 修饰符使用是否正确 |

## 检查方法

```rust
impl TypeChecker {
    /// 检查整个模块
    pub fn check_module(&mut self, module: &ast::Module) -> Result<ModuleIR, Vec<TypeError>> { ... }

    /// 检查函数定义
    pub fn check_fn_def(...) -> Result<FunctionIR, TypeError> { ... }

    /// 检查变量声明
    pub fn check_var_decl(...) -> Result<(), TypeError> { ... }

    /// 检查类型定义
    pub fn check_type_def(...) -> Result<(), TypeError> { ... }
}
```

## 验收测试

```yaoxiang
# test_type_checking.yx

# 类型注解检查
x: Int = 42          # ✓
y: Float = "hello"   # ✗ 类型不匹配

# 函数返回类型检查
add(a: Int, b: Int): Int = a + b  # ✓
bad_fn(): Int = "hello"           # ✗ 返回类型不匹配

# 参数类型检查
print_int(n: Int) = n
print_int("hello")    # ✗ 参数类型不匹配

print("Type checking tests passed!")
```

## 相关文件

- **check.rs**: TypeChecker 实现

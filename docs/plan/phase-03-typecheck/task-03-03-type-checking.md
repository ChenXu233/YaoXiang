# Task 3.3: 类型检查

> **优先级**: P0
> **状态**: ✅ 已实现

## 功能描述

在类型推断的基础上，检查类型注解和约束是否正确。

## 架构设计

**采用合并式架构**：类型推断与检查合并在 `TypeInferrer` 结构体中。

```
TypeInferrer
├── 类型推断 (infer_* 方法)
│   ├── infer_expr - 推断表达式类型
│   ├── infer_pattern - 推断模式类型
│   └── ...
├── 类型检查 (通过约束收集)
│   └── 在推断过程中收集类型约束
└── 约束求解 (TypeConstraintSolver)
    └── 统一求解所有约束
```

**设计理由**：
1. **简单性** - 一个结构体处理所有类型相关逻辑
2. **Hindley-Milner 风格** - 推断和约束收集天然合一
3. **代码复用** - 共享 `solver` 和 `scopes`
4. **减少类型传递** - 不需要在推断器和检查器之间传递类型环境

## 检查类型

| 检查项 | 说明 |
|--------|------|
| 类型注解匹配 | 检查声明类型与初始化表达式类型是否兼容 |
| 函数返回类型 | 检查 return 语句类型与函数返回类型是否一致 |
| 参数类型 | 检查函数调用参数类型与声明参数类型是否匹配 |
| 可变性约束 | 检查 mut 修饰符使用是否正确 |
| 模式匹配 | 检查模式与被匹配值的类型是否兼容 |
| 模式穷尽性 | 检查所有可能情况是否被覆盖 |

## 检查方法

```rust
impl TypeInferrer {
    /// 检查整个模块
    pub fn check_module(&mut self, module: &ast::Module) -> Result<ModuleIR, Vec<TypeError>> { ... }

    /// 检查函数定义
    pub fn check_fn_def(...) -> Result<FunctionIR, TypeError> { ... }

    /// 检查变量声明
    pub fn check_var_decl(...) -> Result<(), TypeError> { ... }

    /// 检查类型定义
    pub fn check_type_def(...) -> Result<(), TypeError> { ... }

    /// 检查模式穷尽性 ⏳ 待实现
    /// 
    /// 验证 match 表达式的所有可能情况是否被覆盖
    pub fn check_exhaustive(&self, pattern: &ast::Pattern, value_type: &MonoType) -> Result<(), TypeError> { ... }
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

# 模式匹配检查
type Option[T] = some(T) | none
result = match some(42) {
    some(n) => n * 2  # ✓ n 类型为 Int
    none => 0
}

# 结构体模式匹配
type Point = Point(x: Int, y: Int)
p = Point(x: 1, y: 2)
match p {
    Point(x, y) => x + y  # ✓ x, y 类型为 Int
}

# 模式类型兼容性检查 ⏳ 待实现
# match 42 {
#     some(n) => n  # ✗ Int 与 Option 不兼容
# }

# 模式穷尽性检查 ⏳ 待实现
# match some(42) {
#     some(n) => n
#     # ✗ 缺少 none 分支，非穷尽
# }

print("Type checking tests passed!")
```

## 相关文件

- **infer.rs**: TypeInferrer 实现（类型推断与检查合并）

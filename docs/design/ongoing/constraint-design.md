# RFC-010 / RFC-011 约束（Constraint）实现设计

> **状态**: 实现中
> **创建日期**: 2026-02-02
> **最后更新**: 2026-02-03

## 核心设计

### 约束 = 接口

约束在 YaoXiang 中定义为**所有字段都是函数类型的记录类型**：

```yaoxiang
# 约束（接口）定义
type Drawable = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}
```

### 约束只能在泛型上下文中使用

```yaoxiang
# ✅ 正确：泛型约束
draw: [T: Drawable](item: T, surface: Surface) -> Void = (item, surface) => {
    item.draw(surface)
}

# ❌ 错误：约束类型直接赋值
d: Drawable = some_circle  # 编译错误！
```

### 结构化匹配（鸭子类型）

类型只要包含约束要求的所有方法（签名兼容），就满足约束：

```yaoxiang
# 满足 Drawable 约束的类型
type Circle = {
    radius: Int,
    draw: (Circle, Surface) -> Void,  # 包含 draw 方法，签名兼容
    bounding_box: (Circle) -> Rect     # 包含 bounding_box 方法，签名兼容
}
```

## 实现状态

### 已完成 ✅

1. **MonoType 扩展** (`src/frontend/core/type_system/mono.rs`)
   - `is_constraint()`: 判断是否是约束类型
   - `constraint_fields()`: 获取约束的所有要求字段

2. **错误类型** (`src/frontend/typecheck/errors.rs`)
   - `ConstraintCheck`: 约束检查失败错误 (E0022)
   - `ConstraintInNonGenericContext`: 约束在非泛型上下文使用错误 (E0023)

3. **BoundsChecker 扩展** (`src/frontend/typecheck/checking/bounds.rs`)
   - `check_constraint()`: 检查类型是否满足约束
   - `fn_signatures_compatible()`: 检查函数签名兼容性

4. **赋值检查** (`src/frontend/typecheck/checking/assignment.rs`)
   - 拒绝约束类型直接赋值

5. **泛型推断器扩展** (`src/frontend/typecheck/inference/generics.rs`)
   - `check_type_constraint()`: 在泛型实例化时检查约束

6. **单元测试** (`src/frontend/typecheck/tests/constraint.rs`)
   - 约束类型识别测试
   - 约束匹配测试（成功/失败）
   - 函数签名兼容性测试
   - 空约束测试

### 待实现 🔲

1. **解析器支持**: 解析 `[T: Drawable]` 语法
2. **类型环境集成**: 从类型环境获取约束类型定义
3. **交集约束支持**: `T: Drawable & Serializable`

## 代码结构

```
src/frontend/
├── core/
│   └── type_system/
│       └── mono.rs          # MonoType 扩展（is_constraint, constraint_fields）
└── typecheck/
    ├── errors.rs            # 新增约束检查错误
    ├── checking/
    │   ├── bounds.rs        # BoundsChecker 扩展（check_constraint）
    │   └── assignment.rs    # 拒绝约束类型赋值
    ├── inference/
    │   └── generics.rs      # 泛型推断器扩展
    └── tests/
        ├── mod.rs           # 添加 constraint 模块
        └── constraint.rs    # 单元测试
```

## 关键算法

### 约束检查 (`check_constraint`)

```rust
fn check_constraint(ty: &MonoType, constraint: &MonoType) -> Result<()> {
    // 1. 获取约束的所有函数字段
    let constraint_fields = constraint.constraint_fields();

    // 2. 获取类型的函数字段
    let type_fn_fields = ty.get_fn_fields();

    // 3. 检查每个约束字段是否存在且签名兼容
    for (field_name, constraint_fn) in constraint_fields {
        match type_fn_fields.get(field_name) {
            Some(found_fn) => {
                if !fn_signatures_compatible(found_fn, constraint_fn) {
                    return Err(SignatureMismatch);
                }
            }
            None => return Err(MissingMethod(field_name)),
        }
    }

    Ok(())
}
```

### 函数签名兼容性

```rust
fn fn_signatures_compatible(found: &Fn, required: &Fn) -> bool {
    // 返回类型必须相同
    if found.return_type != required.return_type {
        return false;
    }

    // 参数数量比较：
    // - 相同：直接比较
    // - found 多一个参数（self）：跳过第一个参数比较
    match (found.params.len(), required.params.len()) {
        (n, n) => found.params == required.params,
        (n+1, n) => found.params[1..] == required.params,
        _ => false,
    }
}
```

## 错误代码

| 代码 | 错误类型 | 说明 |
|------|----------|------|
| E0022 | ConstraintCheck | 类型不满足约束 |
| E0023 | ConstraintInNonGenericContext | 约束类型在非泛型上下文使用 |

## 测试用例

### 约束识别

```rust
#[test]
fn test_constraint_recognition() {
    // 函数字段组成的类型是约束类型
    let drawable = MonoType::Struct(...);  // 只有 draw 方法
    assert!(drawable.is_constraint());

    // 包含非函数字段的类型不是约束类型
    let point = MonoType::Struct(...);  // 有 x, y 字段
    assert!(!point.is_constraint());
}
```

### 约束匹配

```rust
#[test]
fn test_type_satisfies_constraint() {
    let mut checker = BoundsChecker::new();

    // Circle 有 draw 方法，满足 Drawable 约束
    let circle = Circle { radius: 1, draw: fn(...) => ... };
    assert!(checker.check_constraint(&circle, &Drawable).is_ok());

    // Rect 没有 draw 方法，不满足 Drawable 约束
    let rect = Rect { width: 1, height: 1 };
    assert!(checker.check_constraint(&rect, &Drawable).is_err());
}
```

### 拒绝约束类型赋值

```rust
#[test]
fn test_reject_constraint_assignment() {
    let checker = AssignmentChecker::new();

    // 约束类型直接赋值应该被拒绝
    let result = checker.check_assignment(&Drawable, &Circle, span);
    assert!(result.is_err());
}
```

## 下一步

1. 解析器支持 `[T: Drawable]` 语法
2. 在泛型函数调用时集成约束检查
3. 支持 TypeRef 类型的约束检查（需要类型环境）
4. 交集约束 `T: Drawable & Serializable` 支持

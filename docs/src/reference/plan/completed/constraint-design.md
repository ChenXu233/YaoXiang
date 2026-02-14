# RFC-010 / RFC-011 约束（Constraint）设计决策

> **状态**: 已确定
> **创建日期**: 2026-02-02
> **最后更新**: 2026-02-02
> **相关 RFC**: [RFC-010 统一类型语法](../design/accepted/010-unified-type-syntax.md), [RFC-011 泛型系统](../design/accepted/011-generic-type-system.md)

---

## 一、核心设计原则

### 1.1 约束 = 能力要求

约束定义了类型必须提供的能力（字段和方法）：

```yaoxiang
# 定义约束（要求什么能力）
type Drawable = {
    draw: (Self, Surface) -> Void,
    bounding_box: (Self) -> Rect
}

type Serializable = {
    serialize: (Self) -> String
}
```

### 1.2 约束只能在泛型上下文中使用

**❌ 不允许**：直接鸭子类型赋值

```yaoxiang
let d: Drawable = Circle(1)  # 不允许！
```

**✅ 允许**：泛型约束

```yaoxiang
draw: [T: Drawable](item: T) -> Void = (item) => {
    item.draw(screen)
}

# 调用时自动检查
draw(Circle(1))  # ✅ Circle 有 draw，通过
draw(Rect(2))    # ❌ Rect 没有 draw，编译错误
```

**原因**：
- 直接赋值是"事后诸葛"，可能意外匹配
- 泛型约束是"事前验证"，意图清晰

---

## 二、使用场景

### 2.1 泛型函数参数

```yaoxiang
# 处理任何可绘制的对象
process: [T: Drawable](items: List[T]) -> Void = (items) => {
    for item in items {
        item.draw(screen)
    }
}
```

### 2.2 泛型返回类型

```yaoxiang
# 返回可序列化对象
serialize_all: [T: Serializable](items: List[T]) -> List[String] = {
    items.map((item) => item.serialize())
}
```

### 2.3 泛型数据容器

```yaoxiang
# 容器元素必须是可绘制的 - 错误写法
let shapes: List[Drawable] = []  # 错误！约束不能在非泛型上下文使用

# 正确：使用泛型参数
let shapes: List[Circle] = []
```

---

## 三、结构化子类型规则

### 3.1 匹配规则

```yaoxiang
# 约束定义
type Config = {
    load: () -> String,
    save: (String) -> Void,
    name: String
}

# 类型定义
type File = {
    filename: String,
    load: () -> String,
    save: (String) -> Void,
    size: Int,          # 额外字段，忽略
}

# 检查 File 是否满足 Config：
#   - load: () -> String ✅ 匹配
#   - save: (String) -> Void ✅ 匹配
#   - name: String ❌ 不匹配（名字是 filename，不是 name）
#
# 结果：❌ File 不满足 Config
```

### 3.2 匹配算法

| 要求 | 类型提供 | 结果 |
|------|---------|------|
| `x: Int` | `x: Int` | ✅ 匹配 |
| `x: Int` | `y: Int` | ❌ 不匹配 |
| `x: Int` | `x: String` | ❌ 不匹配 |
| `fn: (A) -> B` | `fn: (Self, A) -> B` | ✅ 匹配（去掉 Self） |
| 要求的字段/方法 | 额外的字段/方法 | ✅ 忽略 |

### 3.3 编译器检查流程

```rust
fn check_type_satisfies_constraint(
    typ: &MonoType,
    constraint: &MonoType,
) -> Result<(), ConstraintCheckError> {
    // 1. 验证 constraint 是有效约束（所有字段都是函数类型）
    if !constraint.is_valid_constraint() {
        return Err(ConstraintCheckError::NotAConstraint);
    }

    // 2. 遍历约束的所有要求
    for (name, required_type) in constraint.required_fields() {
        match lookup_type_field(typ, name) {
            Some(found_type) => {
                // 3. 检查类型兼容性
                if !types_compatible(found_type, required_type, typ) {
                    return Err(ConstraintCheckError::TypeMismatch {
                        field: name,
                        expected: required_type,
                        found: found_type,
                    });
                }
            }
            None => {
                // 4. 缺少必需字段
                return Err(ConstraintCheckError::MissingField {
                    field: name,
                    constraint: constraint.name(),
                });
            }
        }
    }

    Ok(())
}
```

---

## 四、类型定义时的约束声明（可选）

类型定义时可以声明实现哪些约束，便于代码阅读和 IDE 提示：

```yaoxiang
# 声明类型时实现约束
type Point = {
    x: Int,
    y: Int,
    draw: (Point, Surface) -> Void,
    bounding_box: (Point) -> Rect,
    serialize: (Point) -> String,
    Drawable,      # 声明实现 Drawable
    Serializable   # 声明实现 Serializable
}
```

**效果**：
- ✅ 代码自文档化
- ✅ IDE 可以提示"Point 实现了 Drawable"
- ✅ 编译器会验证声明是否正确

---

## 五、错误处理

### 5.1 错误类型

```rust
pub enum ConstraintCheckError {
    #[error("'{0}' is not a valid constraint (must have function fields only)")]
    NotAConstraint(String),

    #[error("Type '{type}' does not satisfy constraint '{constraint}': missing field '{field}'")]
    MissingField {
        type_name: String,
        constraint: String,
        field: String,
        span: Span,
    },

    #[error("Type '{type}' does not satisfy constraint '{constraint}': field '{field}' type mismatch")]
    TypeMismatch {
        type_name: String,
        constraint: String,
        field: String,
        expected: String,
        found: String,
        span: Span,
    },

    #[error("Constraint '{0}' can only be used in generic context")]
    NotInGenericContext {
        constraint_name: String,
        span: Span,
    },
}
```

### 5.2 错误示例

```
Error: Type 'Rect' does not satisfy constraint 'Drawable'
  Required method 'draw: (Rect, Surface) -> Void' not found
  Note: Add 'draw' method to Rect to satisfy Drawable

Error: Constraint 'Serializable' can only be used in generic context
  Did you mean: 'serialize_all: [T: Serializable](List[T]) -> List[String]'
```

---

## 六、为什么这样设计？

### 6.1 对比其他方案

| 方案 | 问题 |
|------|------|
| `let d: Drawable = Circle(1)` | 意外匹配，鸭子类型过于宽松 |
| `impl Drawable for Circle` | 需要新关键字，违反 RFC-010 设计原则 |
| `as Config` 转换 | 增加语法复杂度 |
| **当前方案：泛型约束** | 意图清晰，编译期检查，无意外匹配 |

### 6.2 设计原则

1. **约束 = 能力要求**：只定义"需要什么"
2. **泛型 = 事前验证**：调用前检查，不允许意外
3. **零新关键字**：复用现有语法
4. **编译期安全**：所有检查在编译期完成

---

## 七、文件结构

```
src/frontend/
├── core/
│   └── type_system/
│       └── mono.rs              # MonoType 扩展（is_constraint）
│
└── typecheck/
    ├── checking/
    │   ├── mod.rs               # 导出 constraint 模块
    │   └── constraint.rs        # ⬅️ 约束检查器（新增）
    ├── errors.rs                # TypeError 扩展
    └── ...
        └── tests/
            └── test_constraint.rs # ⬅️ 约束检查测试（新增）
```

---

## 八、验收标准

- [ ] 约束定义语法正常工作
- [ ] 泛型约束 `[T: Drawable]` 正常工作
- [ ] `let d: Drawable = ...` 被正确拒绝
- [ ] 结构化匹配规则正确实现
- [ ] 错误信息清晰准确
- [ ] 所有现有测试通过
- [ ] 新增 10+ 单元测试

---

## 九、问答

### Q: 为什么不允许 `let d: Drawable = Circle(1)`？

A: 这是"事后验证"，Circle 碰巧有 draw 方法就被接受，可能不是设计意图。泛型约束是"事前验证"，代码明确说"我需要 Drawable 能力"。

### Q: 如何存储一组 Drawable 对象？

A: 使用泛型容器或接口模式：

```yaoxiang
# 方法1：统一具体类型
let shapes: List[Circle] = []
shapes.push(Circle(1))

# 方法2：使用特质/接口模式（需要运行时派发）
# 如果确实需要异构集合，未来可以添加 trait object 支持
```

### Q: 这和 Rust 的 Trait 有什么不同？

A: 本质相似，但：
- 没有 `impl` 关键字
- 没有显式声明要求（可选）
- 只在泛型上下文中使用

### Q: 约束可以包含数据字段吗？

A: 可以。YaoXiang 的约束不限于方法：

```yaoxiang
type HasPosition = {
    x: Int,
    y: Int
}

move: [T: HasPosition](item: T, dx: Int, dy: Int) -> T = (item, dx, dy) => {
    # item.x 和 item.y 必须存在
    item
}
```

---

## 十、相关文档

- [RFC-010 统一类型语法](../design/accepted/010-unified-type-syntax.md)
- [RFC-011 泛型系统](../design/accepted/011-generic-type-system.md)

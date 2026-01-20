# 实现计划：统一类型语法

> **任务 ID**: impl-type-syntax
> **RFC**: [RFC-010 统一类型语法](../design/rfc/010-unified-type-syntax.md)
> **状态**: 待开始
> **优先级**: P0
> **预计工时**: 2-3 周

---

## 目标

实现 YaoXiang 的**统一类型语法**（RFC-010），统一数据结构体和枚举的定义方式：

- `{}` 定义数据类型（结构体、枚举、联合）
### 旧语法 → 新语法

```yaoxiang
# === 旧语法（构造器即类型）===
type Point = Point(x: Float, y: Float)
type Color = red | green | blue
type Result[T, E] = ok(T) | err(E)

# === 新语法（花括号）===
type Point = { x: Float, y: Float }
type Color = { red | green | blue }
type Result[T, E] = { ok(T) | err(E) }
```

---

## 设计规格

### 数据类型语法 (花括号)

```yaoxiang
# 结构体
type Point = { x: Float, y: Float }

# 枚举（无载荷变体）
type Color = { red | green | blue }

# 枚举（有载荷变体）
type Result[T, E] = { ok(T) | err(E) }
type Option[T] = { some(T) | none }

# 混合类型
type Shape = { circle(Float) | rect(Float, Float) }
```

### 接口类型语法 (方括号)

```yaoxiang
# 单方法接口
type Serializable = [ serialize() -> String ]

# 多方法接口
type Drawable = [
    draw(Surface) -> Void,
    bounding_box() -> Rect
]
```

### 与 RFC-004（多位置绑定）的集成

绑定语法 `func[position]` 中的位置索引从 **0** 开始：

```yaoxiang
# 原始函数：distance(Point, Point) -> Float
distance(Point, Point) -> Float = (a, b) => { ... }

# 绑定到 Point 类型（第 0 位）
Point.distance = distance[0]

# 使用：p1.distance(p2) → distance(p1, p2)
```

---

## 实现步骤

### Phase 1: 词法分析器扩展

**文件**: `src/frontend/lexer/`

| 任务 | 描述 | 状态 | 文件 |
|------|------|------|------|
| Token 扩展 | 添加 `LBRACE`(`{`), `RBRACE`(`}`), `LBRACK`(`[`), `RBRACK`(`]`) | 待开始 | `token.rs` |
| Token 识别 | 更新 `tokenize` 函数识别新分隔符 | 待开始 | `mod.rs` |
| 位置信息 | 确保新 token 有正确的 span 信息 | 待开始 | `span.rs` |

**测试用例**:
```yaoxiang
# 应该成功解析
type A = { x: Int }
type B = { a | b | c }
type C = [ foo() -> Int ]

# 应该报错
type D = { x: Int      # 缺少右花括号
type E = [ foo() -> Int  # 缺少右方括号
```

---

### Phase 2: 语法分析器重构

**文件**: `src/frontend/parser/`

#### 2.1 AST 结构调整

**文件**: `src/frontend/ast/types.rs`

```rust
// 新的 TypeDefBody 枚举
pub enum TypeDefBody {
    Struct { fields: Vec<Field> },
    Enum { variants: Vec<Variant> },
    Interface { methods: Vec<FnSignature> },
}

pub struct Field {
    pub name: Ident,
    pub type_expr: TypeExpr,
}

pub struct Variant {
    pub name: Ident,
    pub payload: Option<TypeExpr>,  // None 表示无载荷
}

pub struct FnSignature {
    pub name: Ident,
    pub params: Vec<TypeExpr>,
    pub return_type: Box<TypeExpr>,
}
```

#### 2.2 解析函数更新

| 任务 | 描述 | 状态 | 相关文件 |
|------|------|------|---------|
| `parse_type_def` | 重构类型定义解析函数 | 待开始 | `type_def.rs` (新建) |
| `parse_struct_body` | 解析 `{ x: Float, y: Float }` | 待开始 | `type_def.rs` |
| `parse_enum_body` | 解析 `{ ok(T) \| err(E) }` | 待开始 | `type_def.rs` |
| `parse_interface_body` | 解析 `[ serialize() -> String ]` | 待开始 | `type_def.rs` |
| `parse_variant` | 解析变体 `ok(T)` 或 `red` | 待开始 | `type_def.rs` |

**参考现有实现**: [task-02-05-types.md](../phase-02-parser/task-02-05-types.md)

#### 2.3 模式匹配更新

```yaoxiang
# 新语法下的模式匹配
let p: Point = Point(3.0, 4.0)
match p {
    { x, y } => x + y,  # 解构
    _ => 0
}
```

---

### Phase 3: 类型检查器更新

**文件**: `src/frontend/typecheck/`

| 任务 | 描述 | 状态 |
|------|------|------|
| 类型注册 | 更新 `TypeDef` 注册逻辑 | 待开始 |
| 字段访问 | 更新 `.field` 访问检查 | 待开始 |
| 构造器检查 | 更新构造器调用检查 | 待开始 |
| 模式匹配 | 更新解构模式匹配 | 待开始 |
| 接口约束 | 实现接口类型约束检查 | 待开始 |

#### 3.1 类型注册更新

```rust
// 在 type_env.rs 中

pub struct TypeEnvironment {
    pub types: HashMap<String, TypeDef>,
    // ...
}

pub struct TypeDef {
    pub name: String,
    pub body: TypeDefBody,  // Struct | Enum | Interface
    // ...
}
```

#### 3.2 构造器验证

```rust
// 验证类型构造调用
fn check_constructor_call(
    constructor: Ident,
    args: Vec<Expr>,
    expected_type: Option<Type>,
) -> Type {
    // 查找类型定义
    let type_def = lookup_type(constructor.name)?;

    match type_def.body {
        TypeDefBody::Struct { fields } => {
            // 验证字段数量和类型
        }
        TypeDefBody::Enum { variants } => {
            // 验证变体存在性和载荷类型
        }
        _ => error!("Cannot construct interface type"),
    }
}
```

---

### Phase 4: 单态化器更新

**文件**: `src/frontend/monomorphize/`

| 任务 | 描述 | 状态 |
|------|------|------|
| 泛型特化 | 更新泛型类型的单态化 | 待开始 |
| 变体特化 | 更新枚举变体的单态化 | 待开始 |

---

### Phase 5: 代码生成更新

**文件**: `src/middle/codegen/`

| 任务 | 描述 | 状态 |
|------|------|------|
| 结构体布局 | 生成结构体的内存布局信息 | 待开始 |
| 枚举布局 | 生成枚举的 tag + payload 布局 | 待开始 |
| 接口虚表 | 生成接口的虚表 (vtable) | 待开始 |

---

### Phase 6: 测试覆盖

**文件**: `tests/unit/`

| 测试文件 | 覆盖场景 |
|----------|----------|
| `type_parsing.rs` | 花括号/方括号解析 |
| `struct_types.rs` | 结构体定义和使用 |
| `enum_types.rs` | 枚举定义和模式匹配 |
| `interface_types.rs` | 接口定义和实现 |
| `type_checking.rs` | 类型检查边界条件 |

---

## 验收标准

- [ ] 词法分析器识别 `{`, `}`, `[`, `]` 作为分隔符
- [ ] `type Point = { x: Float, y: Float }` 正确解析
- [ ] `type Result[T, E] = { ok(T) | err(E) }` 正确解析
- [ ] `type Serializable = [ serialize() -> String ]` 正确解析
- [ ] 模式匹配解构 `{ x, y }` 正常工作
- [ ] 所有现有测试通过
- [ ] 新增 20+ 单元测试

---

## 风险和依赖

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| 语法冲突 | `{}` 同时用于 block 和类型定义 | 根据上下文解析，无歧义 |
| 现有代码兼容 | 现有代码使用旧语法 | 渐进式迁移，先警告后移除 |
| 性能开销 | 虚表派发可能有额外开销 | 单态化优化接口调用 |
| 测试覆盖 | 新语法边界条件多 | 全面单元测试 |

---

## 兼容性策略

```yaoxiang
# 旧语法 (废弃但兼容)
type Point = Point(x: Float, y: Float)   # 编译警告
type Result[T, E] = ok(T) | err(E)       # 编译警告

# 新语法
type Point = { x: Float, y: Float }
type Result[T, E] = { ok(T) | err(E) }
```

**迁移策略**:
- v0.3: 新语法支持，提示旧语法废弃警告
- v0.4: 移除旧语法支持

---

## 进度追踪

| 阶段 | 状态 | 完成日期 | 负责人 |
|------|------|----------|--------|
| 词法分析器 | ⏳ | - | - |
| 语法分析器 | ⏳ | - | - |
| 类型检查器 | ⏳ | - | - |
| 单态化器 | ⏳ | - | - |
| 代码生成 | ⏳ | - | - |
| 测试验收 | ⏳ | - | - |

---

## 相关文档

- [RFC-010 统一类型语法](../design/rfc/010-unified-type-syntax.md)
- [RFC-004 多位置联合绑定](../design/rfc/004-curry-multi-position-binding.md)
- [类型解析任务](../phase-02-parser/task-02-05-types.md)
- [语言规范](../design/language-spec.md)

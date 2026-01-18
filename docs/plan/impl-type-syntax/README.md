# 实现计划：统一类型语法

> **任务 ID**: impl-type-syntax
> **状态**: 待开始
> **优先级**: P0
> **预计工时**: 2-3 周

---

## 目标

实现 YaoXiang 的统一类型语法：
- `{}` 定义数据结构（结构体、枚举、联合）
- `[]` 定义接口类型

---

## 设计规格

### 数据类型语法 (花括号)

```yaoxiang
# 结构体
type Point = { x: Float, y: Float }

# 枚举
type Result[T, E] = { ok(T) | err(E) }
type Color = { red | green | blue }

# 混合类型
type Shape = { circle(Float) | rect(Float, Float) }
```

### 接口类型语法 (方括号)

```yaoxiang
# 接口定义：方法签名集合
type Serializable = [ serialize() -> String ]

type Drawable = [
    draw(Surface) -> Void,
    bounding_box() -> Rect
]
```

---

## 接口与数据的组合

### 核心概念

接口 `[ serialize() -> String ]` 本质上是一组**方法签名**的集合。数据结构通过声明绑定来实现接口。

```
接口定义 Drawable:
  draw(Surface) -> Void     → 需要: fn(Drawable, Surface) -> Void
  bounding_box() -> Rect    → 需要: fn(Drawable) -> Rect

数据结构 Point 实现 Drawable:
  Point.draw = draw_function[0]        # this 在第 0 位
  Point.bounding_box = bbox_function[0]
```

### 完整组合示例

```yaoxiang
# 1. 定义数据结构
type Point = { x: Float, y: Float }

# 2. 定义接口
type Drawable = [
    draw(Surface) -> Void,
    bounding_box() -> Rect
]

# 3. 定义自由函数（底层实现）
draw_point(Point, Surface) -> Void = (p, surface) => {
    surface.draw_rect(p.x, p.y, 10, 10)
}

bbox_point(Point) -> Rect = (p) => {
    Rect(p.x - 5, p.y - 5, 10, 10)
}

# 4. 实现接口：声明绑定（编译时检查）
impl Point: Drawable

# 编译器自动推导绑定：
# Point.draw = draw_point[0]
# Point.bounding_box = bbox_point[0]

# 5. 使用
let p = Point(100, 200)
let surface = Surface.new()

# 函数视角
draw_point(p, surface)
bbox_point(p)

# OOP 视角（语法糖，自动展开）
p.draw(surface)     # → draw_point(p, surface)
p.bounding_box()    # → bbox_point(p)
```

### 多位置联合绑定（RFC-004）

当函数参数顺序不匹配时，使用 `[n]` 语法指定绑定位置：

```yaoxiang
# 底层函数：Point 在第 1 位
render_thing(Surface, Point) -> Void = (surface, p) => { ... }

# 接口要求：draw(this, Surface)
type Renderable = [ draw(Surface) -> Void ]

# 绑定：翻转参数顺序
impl Point: Renderable
# 编译器推导：Point.draw = render_thing[1]
# p.draw(s) → render_thing(s, p)

# 多位置联合绑定
distance(Point, Point) -> Float = (a, b) => { ... }
type Distanceable = [ distance_to(Point) -> Float ]
# Point.distance_to = distance[0, 1]
# p1.distance_to(p2) → distance(p1, p2)

# 占位符绑定
process(Point, Int, Point) -> Point = (p1, _, p2) => { ... }
Point.combine = process[0, _, 2]
# p1.combine(5, p2) → process(p1, 5, p2)
```

### 编译时检查流程

```
impl Point: Drawable
    │
    ▼
展开接口定义：需要 draw(Surface) -> Void, bounding_box() -> Rect
    │
    ▼
查找 Point 上的绑定：
  Point.draw = draw_point[0] ✅ 存在
  Point.bounding_box = bbox_point[0] ✅ 存在
    │
    ▼
类型检查：
  draw_point(p: Point, surface: Surface) -> Void
  参数匹配：Point 在第 0 位 ✅
  返回类型：Void ✅
    │
    ├─ ✅ 通过
    └─ ❌ 编译错误：Point 没有实现 Drawable.draw
```

**检查规则**：
1. 接口的所有方法必须在数据类型上有对应绑定
2. 绑定位置的参数类型必须与数据类型匹配
3. 方法返回类型必须与接口定义一致
4. 任一不满足 → 编译错误

### 泛型约束

```yaoxiang
fn serialize_all[T: Serializable](items: List[T]) -> List[String] {
    items.map(fn(item) => serialize(item))
}

type Point = { x: Float, y: Float }
# 如果 Point 没有 serialize → 编译错误
let result = serialize_all([Point(1, 2), Point(3, 4)])
```

---

## 实现步骤

### Phase 1: 词法分析器扩展

**文件**: `src/frontend/lexer/`

| 任务 | 描述 | 状态 |
|------|------|------|
| Token 扩展 | 添加 `LBRACE`(`{`), `RBRACE`(`}`), `LBRACK`(`[`), `RBRACK`(`]`) | 待开始 |
| Token 识别 | 更新 tokenize 函数识别新分隔符 | 待开始 |
| 位置信息 | 确保新 token 有正确的 span 信息 | 待开始 |

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

### Phase 2: 语法分析器重构

**文件**: `src/frontend/parser/`

#### 2.1 更新类型定义解析

```rust
// 原来的构造器语法
// type Point = Point(x: Float, y: Float)
// type Result[T, E] = ok(T) | err(E)

// 新语法
// type Point = { x: Float, y: Float }
// type Result[T, E] = { ok(T) | err(E) }
```

| 任务 | 描述 | 状态 |
|------|------|------|
| parse_type_def | 重构类型定义解析函数 | 待开始 |
| parse_struct_body | 解析 `{ x: Float, y: Float }` | 待开始 |
| parse_enum_body | 解析 `{ ok(T) \| err(E) }` | 待开始 |
| parse_interface_body | 解析 `[ serialize() -> String ]` | 待开始 |

#### 2.2 AST 结构调整

```rust
// src/frontend/ast/types.rs

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
    pub payload: Option<TypeExpr>,
}

pub struct FnSignature {
    pub name: Ident,
    pub params: Vec<TypeExpr>,
    pub return_type: Box<TypeExpr>,
}
```

### Phase 3: 类型检查器更新

**文件**: `src/frontend/typecheck/`

| 任务 | 描述 | 状态 |
|------|------|------|
| 类型注册 | 更新类型注册逻辑 | 待开始 |
| 字段访问 | 更新 `.field` 访问检查 | 待开始 |
| 构造器检查 | 更新构造器调用检查 | 待开始 |
| 模式匹配 | 更新解构模式匹配 | 待开始 |
| 接口约束 | 实现接口类型约束检查 | 待开始 |

#### 3.1 接口类型检查

```rust
// 当 T 出现在需要接口类型的位置时
fn check_interface_constraint(value_type: Type, interface_type: InterfaceType) -> bool {
    // 检查 value_type 是否实现了 interface_type 中所有方法
    for method in interface_type.methods {
        if !value_type.has_method(method.name, method.signature) {
            return false;
        }
    }
    true
}
```

### Phase 4: IR 和代码生成

**文件**: `src/middle/codegen/`

| 任务 | 描述 | 状态 |
|------|------|------|
| 结构体布局 | 生成结构体的内存布局信息 | 待开始 |
| 枚举布局 | 生成枚举的 tag + payload 布局 | 待开始 |
| 接口虚表 | 生成接口的虚表 (vtable) | 待开始 |
| 方法派发 | 实现接口方法的动态派发 | 待开始 |

### Phase 5: 测试覆盖

**文件**: `tests/unit/`

| 测试文件 | 覆盖场景 |
|----------|----------|
| `type_parsing.rs` | 花括号/方括号解析 |
| `struct_types.rs` | 结构体定义和使用 |
| `enum_types.rs` | 枚举定义和模式匹配 |
| `interface_types.rs` | 接口定义和实现 |
| `type_checking.rs` | 类型检查边界条件 |

---

## 兼容性

### 废弃旧语法

```yaoxiang
# 旧语法 (废弃)
type Point = Point(x: Float, y: Float)
type Result[T, E] = ok(T) | err(E)

# 新语法
type Point = { x: Float, y: Float }
type Result[T, E] = { ok(T) | err(E) }
```

**策略**:
- v0.3: 新语法支持，提示旧语法废弃警告
- v0.4: 移除旧语法支持

---

## 风险和依赖

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| 语法冲突 | `{}` 同时用于 block 和类型定义 | 根据上下文解析，无歧义 |
| 性能开销 | 虚表派发可能有额外开销 | 单态化优化接口调用 |
| 测试覆盖 | 新语法边界条件多 | 全面单元测试 |

---

## 验收标准

- [ ] 词法分析器识别 `{`, `}`, `[`, `]` 作为分隔符
- [ ] `type Point = { x: Float, y: Float }` 正确解析
- [ ] `type Result[T, E] = { ok(T) | err(E) }` 正确解析
- [ ] `type Serializable = [ serialize() -> String ]` 正确解析
- [ ] 模式匹配解构 `Point(x, y)` 正常工作
- [ ] 所有现有测试通过
- [ ] 新增 20+ 单元测试

---

## 进度追踪

| 阶段 | 状态 | 完成日期 |
|------|------|----------|
| 词法分析器 | ⏳ | - |
| 语法分析器 | ⏳ | - |
| 类型检查器 | ⏳ | - |
| IR/代码生成 | ⏳ | - |
| 测试验收 | ⏳ | - |

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

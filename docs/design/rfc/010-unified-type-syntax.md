# RFC-010: 统一类型语法 - name: type = value 模型

> **状态**: 审核中
> **作者**: 晨煦
> **创建日期**: 2025-01-20
> **最后更新**: 2025-01-25（集成RFC-011泛型系统，补充泛型依赖说明）

## 摘要

本 RFC 提出一种极简统一的类型语法模型：**一切皆 `name: type = value`**。

核心思想：
- 变量/函数：`name: type = value`
- 类型定义：`type Name = { ... }`
- 接口定义：`type InterfaceName = { method: (...) -> ... }`
- 类型方法：`Type.method: (Type, ...) -> ReturnType = (self, ...) => ...`
- 普通方法：`name: (Type, ...) -> ReturnType = (param, ...) => ...`
- 自动绑定：`pub name: (Type, ...) -> ReturnType = ...` → 自动绑定到类型

```yaoxiang
# 核心语法：统一 + 区分

# 变量
x: Int = 42

# 函数
add: (Int, Int) -> Int = (a, b) => a + b

# 类型定义（type 关键字前置，更直观）
type Point = {
    x: Float,
    y: Float,
    Drawable,
    Serializable
}

# 接口定义
type Drawable = {
    draw: (Surface) -> Void
}

type Serializable = {
    serialize: () -> String
}

# 方法定义
Point.draw: (Point, Surface) -> Void = (self, surface) => {
    surface.plot(self.x, self.y)
}

Point.serialize: (Point) -> String = (self) => {
    "Point(${self.x}, ${self.y})"
}

# 使用
p: Point = Point(1.0, 2.0)
p.draw(screen)           # 语法糖 → Point.draw(p, screen)
let s: Drawable = p      # 接口赋值
```

## 动机

### 为什么需要这个特性？

当前类型系统存在多个分离的概念：
- 变量声明语法
- 函数定义语法
- 类型定义语法（不同语法）
- 接口定义语法
- 方法绑定语法

这些概念之间缺乏统一性，导致语法碎片化，学习成本高。

### 设计目标

1. **极致统一**：一个语法规则覆盖所有情况
2. **简洁优雅**：`name: type = value` 对称美学
3. **无需新关键字**：复用现有语法元素
4. **理论优雅**：类型本身也是 Type 类型的值
5. **泛型友好**：与泛型系统（RFC-011）无缝集成

### 与泛型系统的集成

RFC-010的统一语法模型与RFC-011的泛型系统设计**天然契合**，泛型参数可以无缝融入统一模型：

```yaoxiang
# 基础泛型（RFC-011 Phase 1）
type List[T] = { data: Array[T], length: Int }

# 泛型函数
map: [T, R](list: List[T], f: Fn(T) -> R) -> List[R] = ...

# 类型约束（RFC-011 Phase 2）
clone: [T: Clone](value: T) -> T = value.clone()

# Const泛型（RFC-011 Phase 4）
type Array[T, N: Int] = { data: T[N], length: N }
```

**依赖关系**：
- RFC-011 Phase 1（基础泛型）是RFC-010的**强依赖**
- 无基础泛型，RFC-010的泛型示例无法编译
- 建议：RFC-011 Phase 1 与 RFC-010 同步实现

## 提案

### 核心原则

```
统一模型 + 类型定义区分

├── 变量/函数：name: type = value
│   ├── x: Int = 42
│   └── add: (Int, Int) -> Int = (a, b) => a + b
│
├── 类型定义：type Name = ...
│   ├── type Point = { x: Float, y: Float }
│   └── type Drawable = { draw: (Surface) -> Void }
│
├── 类型方法：Type.method: (Type, ...) -> Return = ...
│   └── Point.draw: (Point, Surface) -> Void = (self, s) => ...
│
├── 普通方法：name: (Type, ...) -> Return = ...
│   └── distance: (Point, Point) -> Float = (p1, p2) => ...
│
└── 自动绑定：pub name: (Type, ...) -> Return = ...
    └── pub draw: (Point, Surface) -> Void = ...  → 自动绑定到 Point.draw
```

### 语法定义

#### 1. 变量声明

```yaoxiang
# 基本语法
x: Int = 42
name: String = "Alice"
flag: Bool = true

# 类型推导（可省略）
y = 100  # 推断为 Int
```

#### 2. 函数定义

```yaoxiang
# 完整语法
add: (Int, Int) -> Int = (a, b) => { return a + b }

# 带参数名
greet: (String) -> String = (name) => { return "Hello, ${name}!" }

# 多参数
calc: (x: Float, y: Float, op: String) -> Float = (x, y, op) => {
    match op {
        "+" -> return x + y,
        "-" -> return x - y,
        _ -> return 0.0
    }
}

# 多行函数体
calc2: (x: Float, y: Float) -> Float = (x, y) => {
    if x > y {
        return x
    }
    return y
}
```

#### 返回规则

所有函数必须显式使用 `return` 关键字返回值（除返回 `()` 的函数外）：

```yaoxiang
# 非 Void 返回类型 - 必须使用 return
add: (Int, Int) -> Int = (a, b) => { return a + b }

# Void 返回类型 - 可选使用 return（通常省略）
print: (String) -> () = (msg) => {
    # 不需要 return，或者可以写 return
}

# 单行表达式（推荐使用花括号和 return）
greet: (String) -> String = (name) => { return "Hello, ${name}!" }

# 多行函数体 - 必须使用 return
max: (Int, Int) -> Int = (a, b) => {
    if a > b {
        return a
    } else {
        return b
    }
}
```

#### 3. 类型定义

```yaoxiang
# 简单类型
type Point = {
    x: Float,
    y: Float
}

# 实现接口的类型（接口名写在类型体末尾）
type Point = {
    x: Float,
    y: Float,
    Drawable,     # 实现 Drawable 接口
    Serializable  # 实现 Serializable 接口
}

# 空类型（仅实现接口）
type EmptyType = {}
```

#### 4. 接口定义

```yaoxiang
# 接口 = 记录类型，字段都是函数类型
type Drawable = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}

type Serializable = {
    serialize: () -> String
}

# 空接口
type Empty = {}
```

#### 5. 方法定义

```yaoxiang
# 类型方法：关联到特定类型
Point.draw: (Point, Surface) -> Void = (self, surface) => {
    surface.plot(self.x, self.y)
}

Point.serialize: (Point) -> String = (self) => {
    "Point(${self.x}, ${self.y})"
}

# 普通方法：不关联类型，作为独立函数
distance: (Point, Point) -> Float = (p1, p2) => {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

# 类型推导（可省略）
Point.draw = (self, surface) => surface.plot(self.x, self.y)
Point.serialize = (self) => "Point(${self.x}, ${self.y})"
```

#### 6. 方法绑定：普通方法 ↔ 类型方法

普通方法可以通过 `[position]` 语法绑定到类型，反之亦然（参考 RFC-004）。

**自动绑定**：`pub` 声明的函数自动绑定到同文件定义的类型

```yaoxiang
# 使用 pub 声明，编译器自动绑定
pub distance: (Point, Point) -> Float = (p1, p2) => {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

# 编译器自动推断：
# 1. Point 在当前文件定义
# 2. 函数参数包含 Point
# 3. 执行 Point.distance = distance[0]

# 现在可以这样调用：
# 函数式
d1 = distance(p1, p2)

# OOP 语法糖
d2 = p1.distance(p2)  # → distance(p1, p2)
```

**手动绑定**（需要精确控制位置时）：

```yaoxiang
# 显式绑定（用于非 pub 或需要指定位置）
distance: (Point, Point) -> Float = (p1, p2) => ...
Point.distance = distance[0]

# 或指定绑定位置
# Point.transform = transform[1]  # this 绑定到第 1 位
```

**多位置绑定**：

```yaoxiang
# 函数接收多个 Point 参数
transform_points: (Point, Point, Float) -> Point = (p1, p2, factor) => {
    # ...
}

# 绑定多个位置（自动柯里化）
Point.transform = transform_points[0, 1]

# 调用
p1.transform(p2)(2.0)  # → transform_points(p1, p2, 2.0)
```

**反向绑定**（类型方法转普通函数）：

```yaoxiang
# 类型方法
Point.draw: (Point, Surface) -> Void = (self, surface) => {
    surface.plot(self.x, self.y)
}

# 提取为普通函数（不绑定 this）
draw_point: (Point, Surface) -> Void = Point.draw

# 或绑定到特定位置
# 如果 transform(Vector, Point) 的签名是 transform(v, p)
# 可以绑定 Point 到第 1 位
# Point.transform = transform[1]
```

#### 7. 接口组合

```yaoxiang
# 接口组合 = 类型交集
type DrawableSerializable = Drawable & Serializable

# 使用交集类型
func process[T: Drawable & Serializable](item: T) -> String {
    item.draw(screen)
    return item.serialize()
}
```

#### 8. 泛型类型

```yaoxiang
# 基础泛型（RFC-011 Phase 1）
type List[T] = {
    data: Array[T],
    length: Int,
    push: [T](List[T], T) -> Void,
    get: [T](List[T], Int) -> Maybe[T]
}

# 具体实例化（RFC-011语法）
type IntList = List[Int]

# 泛型方法（RFC-011语法）
List.push[T]: (List[T], T) -> Void = (self, item) => {
    self.data.append(item)
    self.length = self.length + 1
}

List.get[T]: (List[T], Int) -> Maybe[T] = (self, index) => {
    if index >= 0 && index < self.length {
        Maybe.Just(self.data[index])
    } else {
        Maybe.Nothing
    }
}
```

### 示例

#### 完整示例

```yaoxiang
# ======== 1. 接口定义 ========

type Drawable = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}

type Serializable = {
    serialize: () -> String
}

type Transformable = {
    translate: (Transformable, Float, Float) -> Transformable,
    scale: (Transformable, Float) -> Transformable
}

# ======== 2. 类型定义 ========

type Point = {
    x: Float,
    y: Float,
    Drawable,
    Serializable,
    Transformable
}

type Rect = {
    x: Float,
    y: Float,
    width: Float,
    height: Float,
    Drawable,
    Serializable,
    Transformable
}

# ======== 3. 方法定义 ========

# 使用 pub 声明，编译器自动绑定到类型
# 绑定规则：第一个 Point 参数 → 方法名取函数名

pub draw: (Point, Surface) -> Void = (self, surface) => {
    surface.plot(self.x, self.y)
}

pub bounding_box: (Point) -> Rect = (self) => {
    Rect(self.x - 1, self.y - 1, 2, 2)
}

pub serialize: (Point) -> String = (self) => {
    "Point(${self.x}, ${self.y})"
}

pub translate: (Point, Float, Float) -> Point = (self, dx, dy) => {
    Point(self.x + dx, self.y + dy)
}

pub scale: (Point, Float) -> Point = (self, factor) => {
    Point(self.x * factor, self.y * factor)
}

# 普通方法（pub，自动绑定到 Point.distance）
pub distance: (Point, Point) -> Float = (p1, p2) => {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

# Rect 的方法
pub draw: (Rect, Surface) -> Void = (self, surface) => {
    surface.draw_rect(self.x, self.y, self.width, self.height)
}

pub bounding_box: (Rect) -> Rect = (self) => self

pub serialize: (Rect) -> String = (self) => {
    "Rect(${self.x}, ${self.y}, ${self.width}, ${self.height})"
}

pub translate: (Rect, Float, Float) -> Rect = (self, dx, dy) => {
    Rect(self.x + dx, self.y + dy, self.width, self.height)
}

pub scale: (Rect, Float) -> Rect = (self, factor) => {
    Rect(self.x * factor, self.y * factor, self.width * factor, self.height * factor)
}

# ======== 4. 使用 ========

# 创建实例
p: Point = Point(1.0, 2.0)
r: Rect = Rect(0.0, 0.0, 10.0, 20.0)

# 方法调用（语法糖）
p.draw(screen)
r.draw(screen)

# 普通方法调用（直接调用）
d: Float = distance(p, Point(0.0, 0.0))

# 链式调用
p2: Point = p.translate(1.0, 1.0).scale(2.0)

# 接口赋值
drawables: List[Drawable] = [p, r]
for d in drawables {
    d.draw(screen)
}

# 泛型函数
process_all[T: Serializable](items: List[T]) {
    for item in items {
        print(item.serialize())
    }
}

process_all([p, r])
```

### 运行时表示

```yaoxiang
# Point 实例在内存中的表示
p: Point = Point(1.0, 2.0)

# 内部结构：
# p = {
#     x: 1.0,
#     y: 2.0,
#     __vtable__: {
#         draw: &Point.draw,
#         bounding_box: &Point.bounding_box,
#         serialize: &Point.serialize,
#         translate: &Point.translate,
#         scale: &Point.scale
#     }
# }

# 方法调用 p.draw(screen) 编译为：
# 1. 查找 p.__vtable__.draw
# 2. 调用 p.__vtable__.draw(p, screen)
```

## 详细设计

### 接口检查算法

```rust
fn check_type_implements_interface(
    typ: &Type,
    iface: &Type
) -> Result<(), TypeError> {
    // 对于接口的每个字段（函数字段）
    for (field_name, iface_field) in &iface.fields {
        // 检查类型是否有同名方法
        if let Some(method) = typ.methods.get(field_name) {
            // 检查方法签名是否兼容
            // 接口字段: (Surface) -> Void
            // 方法签名: (Point, Surface) -> Void
            // 比较：去掉 self 参数后应该匹配
            if !method_signature_matches(method, iface_field.type_) {
                return Err(TypeError::MethodSignatureMismatch {
                    type_name: typ.name,
                    interface_name: iface.name,
                    method_name: field_name,
                });
            }
        } else {
            return Err(TypeError::MissingMethod {
                type_name: typ.name,
                interface_name: iface.name,
                method_name: field_name,
            });
        }
    }
    Ok(())
}
```

### 鸭子类型支持

```yaoxiang
# 只要有相同方法，就可以赋值给接口类型
type CustomPoint = {
    draw: (CustomPoint, Surface) -> Void,
    x: Float,
    y: Float
}

custom: CustomPoint = CustomPoint(
    (self, s) => s.plot(self.x, self.y),
    1.0,
    2.0
)

# 可以赋值给 Drawable（鸭子类型）
let d: Drawable = custom  # 只要有 draw 方法即可
```

### 语法变化

| 之前 | 之后 |
|------|------|
| `type Point = Point(x: Float, y: Float)` | `type Point = { x: Float, y: Float }` |
| `type Result[T, E] = ok(T) \| err(E)` | `type Result[T, E] = { ok: (T) -> Self, err: (E) -> Self }` |
| 需要 `impl` 关键字 | 无需关键字，接口名写在类型体后 |

## 权衡

### 优点

| 优点 | 说明 |
|------|------|
| 极致统一 | 一个语法规则覆盖所有情况 |
| 理论优雅 | 完美对称的 `name: type = value` |
| 无新关键字 | 复用现有语法元素 |
| 易于实现 | 编译器只需要处理一种声明形式 |
| 易于学习 | 记住一个模式就能写所有代码 |
| 易于扩展 | 新特性可以自然地融入这个模型 |

### 缺点

| 缺点 | 说明 |
|------|------|
| 命名规范 | 方法需遵循 `Type.method` 命名 |
| 冗长 | 完整语法比简化语法长，但可推导 |
| 学习曲线 | 需要理解统一模型 |

### 缓解措施

```yaoxiang
# 1. 清晰的错误信息
# 编译错误示例：
# Error: Point does not implement Serializable
#   Required method 'serialize: () -> String' not found
#   Note: Define Point.serialize to implement Serializable

# 2. 类型推导
# 可以省略类型，由编译器推导
Point.draw = (self, surface) => surface.plot(self.x, self.y)

# 3. IDE 提示
# IDE 自动提示缺失的方法
```

## 替代方案

| 方案 | 描述 | 为什么不选 |
|------|------|-----------|
| 分离语法 | 变量、函数、类型用不同语法 | 碎片化，不统一 |
| impl 关键字 | `impl Point: Drawable` | 增加关键字 |
| with 关键字 | `type Point with Drawable = ...` | 增加关键字 |

## 实现策略

### 阶段划分

1. **Phase 1: 基础语法**
   - [ ] 解析 `name: type = value` 声明
   - [ ] 实现基础类型检查
   - [ ] 支持变量和简单函数

2. **Phase 2: 记录类型**
   - [ ] 解析 `{ ... }` 记录类型
   - [ ] 实现记录类型检查
   - [ ] 支持字段访问

3. **Phase 3: 接口系统**
   - [ ] 解析接口定义 `Interface: Type = { method: (...) -> ... }`
   - [ ] 实现接口检查算法
   - [ ] 支持鸭子类型

4. **Phase 4: 方法系统**
   - [ ] 解析 `Type.method: (...) -> ... = ...` 方法定义
   - [ ] 实现方法关联
   - [ ] 方法调用语法糖

5. **Phase 5: 泛型和高级特性**
   - [ ] 支持泛型类型构造器
   - [ ] 支持接口组合 `&`
   - [ ] 优化和测试

### 依赖关系

- 无外部依赖
- 可独立实现

### 风险

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| 解析复杂度 | 统一语法可能增加解析复杂度 | 使用递归下降解析器 |
| 性能开销 | vtable 查找可能有额外开销 | 编译期单态化优化 |

## 开放问题

- [ ] 泛型语法是否需要简化？
- [ ] 默认方法实现的优先级规则？

## 附录

### 语法 BNF

```bnf
program ::= statement*

statement ::= type_declaration | declaration | expression

# 类型定义：type Name = type_expression
type_declaration ::= 'type' identifier type_params? '=' type_expression

type_params ::= '[' identifier (',' identifier)* ']'

type_expression ::= identifier
                  | '(' type_expression (',' type_expression)* ')' '->' type_expression
                  | '{' type_field* '}'

type_field ::= identifier ':' type_expression
             | identifier   # 接口约束

# 变量/函数声明：name: type = value
declaration ::= identifier ':' type '=' expression

type ::= identifier
       | '(' type (',' type)* ')' '->' type
       | '{' field* '}'

field ::= identifier ':' type
        | identifier

expression ::= literal
              | identifier
              | '(' expression (',' expression)* ')'
              | expression '.' identifier '(' arguments? ')'
              | lambda
              | '{' field ':' expression (',' field ':' expression)* '}'

lambda ::= '(' parameters? ')' '=>' block

parameters ::= parameter (',' parameter)*

parameter ::= identifier ':' type

block ::= expression | '{' expression* '}'
```

### 术语表

| 术语 | 定义 |
|------|------|
| 声明 | `name: type = value` 形式的赋值语句 |
| 记录类型 | 包含命名字段 `{ ... }` 的类型 |
| 接口 | 字段全为函数类型的记录类型 |
| 类型方法 | `Type.method` 形式的方法，关联到特定类型 |
| 普通方法 | 不关联类型的独立函数，可直接调用 |
| 自动绑定 | `pub` 声明的函数自动绑定到同文件定义的类型 |
| 方法关联 | 将 `Type.method` 函数自动关联为类型的函数字段 |
| vtable | 虚表，存储方法指针的数据结构 |

---

## 生命周期与归宿

```
┌─────────────┐
│   草案      │  ← 当前状态
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  审核中     │  ← 开放社区讨论和反馈
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  已接受     │    │  已拒绝     │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   accepted/ │    │    rfc/     │
│ (正式设计)  │    │ (保留原位)  │
└─────────────┘    └─────────────┘
```

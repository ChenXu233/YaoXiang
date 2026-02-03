# RFC-010: 统一类型语法 - name: type = value 模型

> **状态**: 已接受
> **作者**: 晨煦
> **创建日期**: 2025-01-20
> **最后更新**: 2025-02-03（统一语法风格：参数名在签名中声明，函数体用 = { ... }）

## 摘要

本 RFC 提出一种极简统一的类型语法模型：**一切皆 `name: type = value`**。

核心思想：
- 变量/函数：`name: type = value`
- 类型定义：`type Name = { ... }`
- 接口定义：`type InterfaceName = { method: (param: Type) -> Ret }`
- 类型方法：`Type.method: (self: Type, param: Type) -> Ret = { ... }`
- 普通方法：`name: (param: Type) -> Ret = { ... }`
- 自动绑定：`pub name: (param: Type) -> Ret = ...` → 自动绑定到类型

```yaoxiang
# 核心语法：统一 + 区分

# 变量
x: Int = 42

# 函数（参数名在签名中）
add: (a: Int, b: Int) -> Int = a + b

# 类型定义（type 关键字前置，更直观）
type Point = {
    x: Float,
    y: Float,
    Drawable,
    Serializable
}

# 接口定义
type Drawable = {
    draw: (self: Self, surface: Surface) -> Void
}

type Serializable = {
    serialize: (self: Self) -> String
}

# 方法定义（使用 Type.method 语法糖）
Point.draw: (self: Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}

Point.serialize: (self: Point) -> String = {
    return "Point(${self.x}, ${self.y})"
}

# 使用
p: Point = Point(1.0, 2.0)
p.draw(screen)           # 语法糖 → Point.draw(p, screen)
s: Drawable = p      # 接口赋值
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
├── 类型方法：Type.method: (self: Type, ...) -> Return = ...
│   └── Point.draw: (self: Point, surface: Surface) -> Void = ...
│
├── 普通方法：name: (param: Type, ...) -> Return = ...
│   └── distance: (p1: Point, p2: Point) -> Float = ...
│
└── 自动绑定：pub name: (param: Type, ...) -> Return = ...
    └── pub draw: (self: Point, surface: Surface) -> Void = ...  → 自动绑定到 Point.draw
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
# 完整语法（参数名在签名中声明）
add: (a: Int, b: Int) -> Int = {
    return a + b
}

# 带参数名
greet: (name: String) -> String = {
    return "Hello, ${name}!"
}

# 多参数
calc: (x: Float, y: Float, op: String) -> Float = {
    return match op {
        "+" -> x + y,
        "-" -> x - y,
        _ -> 0.0
    }
}

# 多行函数体
calc2: (x: Float, y: Float) -> Float = {
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
add: (a: Int, b: Int) -> Int = {
    return a + b
}

# Void 返回类型 - 可选使用 return（通常省略）
print: (msg: String) -> Void = {
    # 不需要 return
}

# 单行表达式（直接返回值，无需 return）
greet: (name: String) -> String = "Hello, ${name}!"

# 多行函数体 - 必须使用 return
max: (a: Int, b: Int) -> Int = {
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
# 类型方法：关联到特定类型（使用 Type.method 语法）
Point.draw: (self: Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}

Point.serialize: (self: Point) -> String = {
    return "Point(${self.x}, ${self.y})"
}

# 普通方法：不关联类型，作为独立函数
distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
}

# 类型推导（可省略）
Point.draw = (self: Point, surface: Surface) => surface.plot(self.x, self.y)
Point.serialize = (self: Point) => "Point(${self.x}, ${self.y})"
```

#### 6. 方法绑定：普通方法 ↔ 类型方法

普通方法可以通过 `[position]` 语法绑定到类型，反之亦然（参考 RFC-004）。

**自动绑定**：`pub` 声明的函数自动绑定到同文件定义的类型

```yaoxiang
# 使用 pub 声明，编译器自动绑定
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
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
distance: (p1: Point, p2: Point) -> Float = ...
Point.distance = distance[0]

# 或指定绑定位置
# Point.transform = transform[1]  # this 绑定到第 1 位
```

**多位置绑定**：

```yaoxiang
# 函数接收多个 Point 参数
transform_points: (p1: Point, p2: Point, factor: Float) -> Point = {
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
Point.draw: (self: Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}

# 提取为普通函数（不绑定 this）
draw_point: (p: Point, surface: Surface) -> Void = Point.draw

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
    push: [T](self: List[T], item: T) -> Void,
    get: [T](self: List[T], index: Int) -> Maybe[T]
}

# 具体实例化（RFC-011语法）
type IntList = List[Int]

# 泛型方法（RFC-011语法）
List.push: [T](self: List[T], item: T) -> Void = {
    self.data.append(item)
    self.length = self.length + 1
}

List.get: [T](self: List[T], index: Int) -> Maybe[T] = {
    if index >= 0 && index < self.length {
        return Maybe.Just(self.data[index])
    } else {
        return Maybe.Nothing
    }
}
```

### 示例

#### 完整示例

```yaoxiang
# ======== 1. 接口定义 ========

type Drawable = {
    draw: (self: Self, surface: Surface) -> Void,
    bounding_box: (self: Self) -> Rect
}

type Serializable = {
    serialize: (self: Self) -> String
}

type Transformable = {
    translate: (self: Self, dx: Float, dy: Float) -> Self,
    scale: (self: Self, factor: Float) -> Self
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

pub draw: (self: Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}

pub bounding_box: (self: Point) -> Rect = {
    return Rect(self.x - 1, self.y - 1, 2, 2)
}

pub serialize: (self: Point) -> String = {
    return "Point(${self.x}, ${self.y})"
}

pub translate: (self: Point, dx: Float, dy: Float) -> Point = {
    return Point(self.x + dx, self.y + dy)
}

pub scale: (self: Point, factor: Float) -> Point = {
    return Point(self.x * factor, self.y * factor)
}

# 普通方法（pub，自动绑定到 Point.distance）
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
}

# Rect 的方法
pub draw: (self: Rect, surface: Surface) -> Void = {
    surface.draw_rect(self.x, self.y, self.width, self.height)
}

pub bounding_box: (self: Rect) -> Rect = self

pub serialize: (self: Rect) -> String = {
    return "Rect(${self.x}, ${self.y}, ${self.width}, ${self.height})"
}

pub translate: (self: Rect, dx: Float, dy: Float) -> Rect = {
    return Rect(self.x + dx, self.y + dy, self.width, self.height)
}

pub scale: (self: Rect, factor: Float) -> Rect = {
    return Rect(self.x * factor, self.y * factor, self.width * factor, self.height * factor)
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
    draw: (self: CustomPoint, surface: Surface) -> Void,
    x: Float,
    y: Float
}

custom: CustomPoint = CustomPoint(
    (self, surface) => surface.plot(self.x, self.y),
    1.0,
    2.0
)
```

### 语法变化

| 之前 | 之后 |
|------|------|
| `type Point = Point(x: Float, y: Float)` | `type Point = { x: Float, y: Float }` |
| `type Result[T, E] = ok(T) \| err(E)` | `type Result[T, E] = { ok: (T) -> Self, err: (E) -> Self }` |
| 需要 `impl` 关键字 | 无需关键字，接口名写在类型体后 |

## 语法设计说明：具名函数本质是 Lambda 的语法糖

### 核心理解

**具名函数和 Lambda 表达式是同一个东西！** 唯一的区别是：具名函数给 Lambda 取了个名字。

```yaoxiang
# 这两者本质完全相同
add: (a: Int, b: Int) -> Int = a + b           # 具名函数（推荐）
add: (Int, Int) -> Int = (a, b) => a + b        # Lambda 形式（完全等价）
```

### 语法糖模型

```
# 具名函数 = Lambda + 名字
name: (Params) -> ReturnType = body

# 本质上是
name: (Params) -> ReturnType = (params) => body
```

**关键点**：当签名完整声明了参数类型，Lambda 头部的参数名就变成了冗余，可以省略。

### 参数作用域规则

**参数覆盖外层变量**：签名中的参数作用域覆盖函数体，内部作用域优先级更高。

```yaoxiang
x = 10  # 外层变量

double: (x: Int) -> Int = x * 2  # ✅ 参数 x 覆盖外层 x，结果为 20
```

### 标注位置灵活

类型标注可以在以下任一位置，**至少标注一处即可**：

| 标注位置 | 形式 | 说明 |
|----------|------|------|
| 仅签名 | `double: (x: Int) -> Int = x * 2` | ✅ 推荐 |
| 仅 Lambda 头 | `double = (x: Int) => x * 2` | ✅ 合法 |
| 两边都标 | `double: (x: Int) -> Int = (x) => x * 2` | ✅ 冗余但允许 |

### 完整示例

```yaoxiang
# ✅ 推荐：签名完整，Lambda 头部省略
add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1
main: () -> Void = { print("hi") }

# ✅ 合法：签名只声明类型，Lambda 补全参数名
add: (Int, Int) -> Int = (a, b) => a + b
inc: Int -> Int = (x) => x + 1

# ✅ 合法：Lambda 头中标注类型
double = (x: Int) => x * 2

# ✅ 合法：两边都标注
double: (x: Int) -> Int = (x) => x * 2
```

### 设计优势

| 特性 | 优势 |
|------|------|
| **简洁** | 签名完整时无需重复写参数名 |
| **灵活** | 保留 Lambda 形式，喜欢哪个用哪个 |
| **一致** | 与变量声明 `x: Int = 42` 保持统一模式 |
| **直观** | `name: Type = body` 直接对应"名为 name，类型 Type，值为 body" |

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
#   Required method 'serialize: (self: Point) -> String' not found
#   Note: Define Point.serialize to implement Serializable

# 2. 类型推导
# 可以省略类型，由编译器推导
Point.draw = (self: Point, surface: Surface) => surface.plot(self.x, self.y)

# 3. IDE 提示
# IDE 自动提示缺失的方法
```

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

statement ::= type_declaration | function | expression

# 类型定义：type Name = { ... }
type_declaration ::= 'type' identifier type_params? '=' type_expression

type_params ::= '[' identifier (',' identifier)* ']'

type_expression ::= identifier
                  | '(' type_expression (',' type_expression)* ')' '->' type_expression
                  | '{' type_field* '}'

type_field ::= identifier ':' type_expression
             | identifier   # 接口约束

# 函数声明：name: (param: Type, ...) -> Ret = { ... }
# 参数名在签名中声明，函数体直接使用参数名
function ::= identifier generic_params? '(' parameters? ')' '->' type '=' (expression | block)

generic_params ::= '[' identifier (',' identifier)* ']'

parameters ::= parameter (',' parameter)*

parameter ::= identifier ':' type

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

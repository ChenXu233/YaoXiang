---
title: "RFC-010: 统一类型语法 - name: type = value 模型"
status: "已接受"
author: "晨煦"
created: "2025-01-20"
updated: "2026-06-05（更新返回规则和 {} 语义）"
---

# RFC-010: 统一类型语法 - name: type = value 模型


## 摘要

本 RFC 提出一种极简统一的类型语法模型：**一切皆 `name: type = value`**。

YaoXiang 只有一种声明形式：

```
identifier : type = expression
```

其中 `type` 可以是任意类型表达式，`expression` 可以是任意值表达式。
**没有 `fn`，没有 `struct`，没有 `trait`，没有 `impl`，没有小写 `type` 关键字（但有 `Type` 作为元类型关键字）**。

> **核心设计**：`Type` 本身就是一个泛型类型。`(T: Type) -> Type` 表示"接受类型参数 T 的类型"。

| 概念       | 代码写法                                      |
|------------|-----------------------------------------------|
| 变量       | `x: Int = 42`                                |
| 函数       | `add: (a: Int, b: Int) -> Int = a + b`       |
| 记录类型   | `Point: Type = { x: Float, y: Float }`       |
| 接口       | `Drawable: Type = { draw: (Surface) -> Void }` |
| 泛型类型   | `List: (T: Type) -> Type = { data: Array(T), length: Int }` |
| 泛型类型   | `Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }` |
| 方法       | `Point.draw: (p: Point, s: Surface) -> Void = ...`<br>`Point.draw = draw[0]` |
| 泛型函数   | `map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))` |

**`Type` 是语言中唯一的元类型关键字**。

> **命名空间 vs 方法绑定**：`Type.name` 前缀表示**命名空间归属**，仅此而已。
> 它不触发任何隐式绑定。要让 `p.draw(screen)` 这种 `.` 调用语法生效，
> 必须显式绑定：`Point.draw = draw[0]`。
> 详见下文"命名空间与方法绑定"一节。
它用于标注类型层级，编译器自动处理 Type0、Type1、Type2... 的区分，对用户透明。

```yaoxiang
// 核心语法：统一 + 区分

// 变量
x: Int = 42

// 函数（参数名在签名中）
add: (a: Int, b: Int) -> Int = a + b

// 记录类型
Point: Type = {
    x: Float,
    y: Float,
    draw: (Surface) -> Void,
    serialize: () -> String
}

// 接口（本质是字段全为函数的记录类型）
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

// 方法定义（使用 Type.method 语法）
Point.draw: (self: Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}

Point.serialize: (self: Point) -> String = {
    return "Point(${self.x}, ${self.y})"
}

// 泛型类型（(T: Type) -> Type = 接受类型参数的泛型类型）
List: (T: Type) -> Type = {
    data: Array(T),
    length: Int
}

Map: (K: Type, V: Type) -> Type = {
    keys: Array(K),
    values: Array(V)
}

// 使用
p: Point = Point(1.0, 2.0)
p.draw(screen)           // 语法糖 → Point.draw(p, screen)
s: Drawable = p           // 结构子类型：Point 实现 Drawable
drawables: List(Drawable) = [p, r]
process_all(drawables)
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
// 基础泛型（RFC-011 Phase 1）
List: (T: Type) -> Type = { data: Array(T), length: Int }

// 泛型函数（RFC-023 语法：签名中 Type 位置可省略，调用时自动推断）
map: (: Type, R: Type) -> (( list: List(T), f: (T) -> R) -> List(R)) = ...

// 类型约束（RFC-011 Phase 2）
clone: (value: T) -> T = value.clone()  // T: Clone 约束由参数类型携带

// Const泛型（RFC-011 Phase 4）
Array: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }
```

**依赖关系**：
- RFC-011 Phase 1（基础泛型）是RFC-010的**强依赖**
- 无基础泛型，RFC-010的泛型示例无法编译
- 建议：RFC-011 Phase 1 与 RFC-010 同步实现

## 提案

### 核心原则：类型构造器 vs 函数/变量

**这是一个关键的设计选择，决定了语法的歧义消除规则：**

| 写法 | 含义 | 规则 |
|------|------|------|
| **`x: Type = ...`** | 类型构造器 | `: Type` 显式声明 → 强制为类型 |
| **`f = ...`** | 函数或变量 | 无 `: Type` → HM 主动推断为函数/变量 |

**为什么这样设计？**

`{ ... }` 语法本身有歧义：
- `{ x: Float, y: Float }` 可以是**类型字面量**（记录类型）
- `{ a = 1 + 1 }` 可以是**代码块**（执行语句，返回 Void）

**消除歧义的规则**：
- **有** `: Type` → 强制解析为类型构造器，`{ ... }` 是类型字面量
- **没有** `: Type` → HM 主动将 `{ ... }` 解析为代码块，推断为函数类型

```yaoxiang
# ✅ 类型构造器：有 : Type
Point: Type = { x: Float, y: Float }

# ✅ 函数：没有 : Type，HM 推断为 () -> Void
main = { println("Hello") }

# ❌ 错误：没有 : Type，编译器无法将 { ... } 解析为类型
Point = { x: Float, y: Float }  // HM 推断为函数，不是类型！
```

---

**统一模型：identifier : type = expression**

```
├── 变量
│   └── x: Int = 42
│
├── 函数
│   └── add: (a: Int, b: Int) -> Int = a + b  # 无 : Type，HM 推断为函数
│
├── 记录类型
│   └── Point: Type = { x: Float, y: Float }  # 必须返回： Type
│
├── 接口
│   └── Drawable: Type = { draw: (Surface) -> Void }  # 必须返回： Type
│
├── 泛型类型
│   └── List: (T: Type) -> Type = { data: Array(T), length: Int }  # 必须返回： Type
│
├── 泛型类型（多参数）
│   └── Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }  # 必须返回： Type
│
├── 命名空间函数
│   └── draw: (p: Point, surface: Surface) -> Void = ...
│       Point.draw = draw[0]  # 显式绑定后才有点调用语法
│
└── 泛型函数
    └── map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))  # 不返回 Type，HM 推断为函数
```

### 元类型层级（编译器内部）

**编译器内部**维护一个宇宙层级 `level: selfpointnum`（用字符串存储，理论上可无限延伸）。

| Level | 说明 |
|-------|------|
| `Type0` | 日常类型（`Int`、`Float`、`Point`） |
| `Type1` | 类型构造器（`List`、`Maybe`） |
| `Type2+` | 高阶构造器 |

**用户从不看见这些数字**，只看见 `: Type`。

> **Curry-Howard 同构**：宇宙层级的存在不是工程实现细节，而是逻辑一致性的必要条件。Curry-Howard 同构将类型等同于命题，如果允许 `Type: Type`（即"类型的类型也是类型"），就会产生类似"这句话是假的"的 Russell 悖论——在类型系统中表现为 Girard 悖论。YaoXiang 的 `Type0 / Type1 / Type2…` 分层（即 Martin-Löf 类型论中的累积宇宙），确保每个类型只属于某一层级，`Typeₙ : Typeₙ₊₁` 形成永不闭合的上升链条，从根本上避免了悖论。这意味着，YaoXiang 的类型系统在 Curry-Howard 意义上是 **逻辑一致的**。

### 语法定义

#### 1. 变量声明

```yaoxiang
// 基本语法
x: Int = 42
name: String = "Alice"
flag: Bool = true

// 类型推导（可省略）
y = 100  // 推断为 Int
```

#### 2. 函数定义

```yaoxiang
// 单表达式形式（直接返回值，无需 return）
add: (a: Int, b: Int) -> Int = a + b
greet: (name: String) -> String = "Hello, ${name}!"

// 代码块形式（必须用 return 返回值）
process: (x: Int) -> Int = {
    a = x * 2
    b = a + 1
    return b
}

// 多行代码块
calc: (x: Float, y: Float, op: String) -> Float = {
    return match op {
        "+" -> x + y,
        "-" -> x - y,
        _ -> 0.0
    }
}

// Void 函数（代码块内不需要 return）
print: (msg: String) -> Void = {
    console.write(msg)
}
```

#### 返回规则

返回值取决于 `=` 右侧的形式：

| 写法 | 返回值 |
|------|--------|
| `= expr`（无花括号） | 直接返回 `expr` |
| `= { ... }`（有花括号） | 必须用 `return`，否则返回 `Void` |

```yaoxiang
# 单表达式：直接返回值，不需要 return
add: (a: Int, b: Int) -> Int = a + b

# 代码块：必须用 return 返回值
process: (x: Int) -> Int = {
    a = x * 2
    b = a + 1
    return b
}

# Void 函数：不需要 return
print: (msg: String) -> Void = {
    console.write(msg)
}
```

> **设计理由**：`{ ... }` 是依赖驱动计算单元（见下文），其返回语义与单表达式不同。花括号引入了多语句上下文，因此需要显式 `return` 来消除"最后一个表达式是否是返回值"的歧义。

#### `{}` 语义：依赖驱动计算单元

`{ ... }` 在 YaoXiang 中不仅仅是代码块——它是**依赖驱动计算单元**。这个语义在函数体、变量初始化和 `spawn` 中保持一致：

**核心规则**：
- `{}` 内的赋值语句按依赖关系自动排序，而非书写顺序
- 依赖齐备则立即执行，缺失则阻塞等待
- 使用 `return` 显式返回值（见返回规则）

```yaoxiang
# 依赖驱动：b 依赖 a，编译器自动排序
result: Int = {
    b = a + 1      # 依赖 a → 自动排在 a 之后
    a = 10         # 无依赖 → 可以先执行
    return b       # 返回 11
}
```

> **与单表达式的区别**：`= expr`（无花括号）是直接返回值的简单绑定；`= { ... }`（有花括号）引入依赖驱动计算上下文，允许多语句和显式 `return`。

#### `spawn` 块

`spawn { ... }` 是 YaoXiang 的唯一并行原语。它利用 `{}` 的依赖驱动语义实现自动并行化：

- `spawn { ... }` 内的直接子赋值自动创建并行任务
- 依赖齐备的任务立即并发执行
- 调用方阻塞等待所有子任务完成

```yaoxiang
result = spawn {
    a = fetch_data("url1")    # 任务 1
    b = fetch_data("url2")    # 任务 2（与 a 无依赖，并行执行）
    c = process(a, b)         # 依赖 a, b → 等待两者完成后执行
    return c
}
// 调用方在此阻塞，直到 spawn 块内所有任务完成
```

> **详细定义**：`spawn` 的完整语义、任务创建规则和阻塞模型详见 `008-runtime-concurrency-model.md`。

#### `unsafe` 块

`unsafe { ... }` 用于定义不透明类型和操作裸指针。它利用 `{}` 的 return 语义将类型定义返回给上一作用域：

**核心规则**：
- `unsafe {}` 中可以定义类型和操作裸指针
- 使用 `return` 将类型定义返回给上一作用域
- 返回的类型在 `unsafe {}` 外可用
- 类型的字段访问需要 unsafe 权限

```yaoxiang
# 在 unsafe 块中定义不透明类型
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  # 裸指针
    }
    return SqliteDb
}

# SqliteDb 在 unsafe 块外可用
db = sqlite3_open("test.db")

# ❌ 编译错误：handle 字段需要 unsafe 权限
handle = db.handle

# ✅ 通过方法调用
db.close()
```

> **详细定义**：`unsafe` 的完整语义、FFI 类型定义和方法绑定详见 `ffi.md`。

#### 3. 类型定义

类型定义是 YaoXiang 统一语法的核心，包含字段、默认值、绑定方法、接口实现：

##### 基础类型

**记录类型**：字段列表，字段类型可以是任意类型表达式。

```yaoxiang
Point: Type = {
    x: Float,
    y: Float
}
```

**有默认值的字段**：字段可以有默认值，构造时可选。

```yaoxiang
Point: Type = {
    x: Float = 0,
    y: Float = 0
}
```

使用：

```yaoxiang
Point() → Point(x=0, y=0)
Point(x=1) → Point(x=1, y=0)
Point(x=1, y=2) → Point(x=1, y=2)
```

**无默认值的字段**：必须在构造时提供。

```yaoxiang
Point2: Type = {
    x: Float,
    y: Float
}
```

使用：
```yaoxiang
Point2(x=1, y=2) //✓
Point2() //✗
Point2(x=1) //✗
```

##### 绑定方法

**方式1：在类型定义体内直接绑定外部函数**

```yaoxiang
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]           // 绑定到位置0，柯里化后 method: (b: Point) -> Float
}
// 调用：p1.distance(p2) → distance(p1, p2)
```

**方式2：匿名函数 + 位置绑定**

```yaoxiang
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance: ((a: Point, b: Point) -> Float)[0] = ((a, b) => {
        dx = a.x - b.x
        dy = a.y - b.y
        return (dx * dx + dy * dy).sqrt()
    })
}
// 语法：((params) => body)[position]
// 调用：p1.distance(p2) → distance(p1, p2)
```

##### 接口实现

**接口名写在类型体内，编译器自动检查其实现**

```yaoxiang
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

Point: Type = {
    x: Float,
    y: Float,
    Drawable,          // 实现 Drawable 接口
    Serializable      // 实现 Serializable 接口
}
```

##### 接口定义

**接口 = 字段全为函数的记录类型**

```yaoxiang
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

// 空类型/空接口
EmptyType: Type = {}
Empty: Type = {}
```

##### 命名空间函数定义

**`Type.name` 前缀表示命名空间归属**，仅此而已。它不触发任何隐式绑定。

```yaoxiang
// 命名空间函数：在 Point 命名空间下的普通函数
Point.draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

Point.serialize: (p: &Point) -> String = {
    return "Point(${p.x}, ${p.y})"
}

// 调用：就是普通函数调用
Point.draw(p, screen)
Point.serialize(p)
```

> **注意**：`self` 不是关键字，只是参数名的约定俗成。写成 `p`、`this`、`x` 效果完全一样。
> 编译器不看参数名，看类型。

##### 方法绑定（唯一方式）

要让 `p.draw(screen)` 这种 `.` 方法调用语法生效，**必须显式绑定**。
`[position]` 语法是将函数绑定为"方法"的唯一机制（详细语法见 RFC-004）。

```yaoxiang
// 定义函数
draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

// 显式绑定 — 这之后才有 p.draw(screen) 语法
Point.draw = draw[0]   // 位置 0 的参数（&Point）由调用者填充

// 使用
p.draw(screen)          // 语法糖 → draw(&p, screen)
Point.draw(p, screen)   // 两种调用方式等价

// 不写 [0] = 不绑定。Point.draw 就是普通函数别名，没有 . 语法
Point.draw = draw       // 不绑定：只能 Point.draw(p, screen)
```

**默认行为**：不写 `[n]` = 不绑定任何参数。用户必须显式决定哪些参数由调用者填充。

**多位置绑定**：

```yaoxiang
// 绑定多个位置（自动柯里化）
Point.transform = transform_points[0, 1]
// 调用：p1.transform(p2)(2.0) → transform_points(p1, p2, 2.0)
```

**反向操作**（方法转普通函数）：

```yaoxiang
// 从绑定中取出函数
draw_point: (p: &Point, surface: Surface) -> Void = Point.draw
```

#### 4. 接口组合

```yaoxiang
// 接口组合 = 类型交集
DrawableSerializable: Type = Drawable & Serializable

// 使用交集类型
process: (T: Drawable & Serializable) -> ((item: T, screen: Surface) -> String) = {
    item.draw(screen)
    return item.serialize()
}
```

#### 5. 泛型类型

```yaoxiang
// 基础泛型（RFC-011 Phase 1）
List: (T: Type) -> Type = {
    data: Array(T),
    length: Int,
    push: (T:Type)-((self: List(T), item: T) -> Void),
    get: (T:Type)->((self: List(T), index: Int) -> Maybe(T))
}

// 具体实例化（RFC-023 语法）
IntList: Type = List(Int)

IntList.push = {
    self.data.append(item)
    self.length = self.length + 1
}

List.push = (type: Type) -> {
    return (self: List(type), item: type) -> {
        self.data.append(item)
        self.length = self.length + 1
    }
}

IntList.push(Int)(self, item)  // 调用示例

// 泛型方法（RFC-023 语法：类型参数由调用处自动推断）
List.push: (self: List(T), item: T) -> Void = {
    self.data.append(item)
    self.length = self.length + 1
}

List.get: (self: List(T), index: Int) -> Maybe(T) = {
    if index >= 0 && index < self.length {
        return Maybe.Just(self.data[index])
    } else {
        return Maybe.Nothing
    }
}
```

#### 6. 泛型调用语法

泛型类型和泛型函数的调用统一使用 `()` 语法。`[]` 不在任何泛型上下文中使用。

**核心规则**：

1. **`()` 做一切应用**：类型应用、函数调用、值构造全部用 `()`

```yaoxiang
# 类型标注
numbers: List(Int) = List(1, 2, 3)

# 空容器：T 从左侧来
empty: List(Int) = List()

# 泛型函数调用——类型从参数自动流动
strings = map(numbers, f)
// T=Int 来自 numbers: List(Int)
// R=String 来自 f: (Int) -> String
```

2. **Type 在左，值在右**：`name: type = value`——Type 参数在左侧声明，右侧永远是具体值。空容器 `List()` 的 `T` 必须从左侧类型注解获取。

3. **类型信息只需写一次**——在参数声明时，编译器带它流动：

```yaoxiang
numbers: List(Int) = List(1, 2, 3)  // Int 在左边写一次
f: (Int) -> String = (x) => x.to_string()
strings = map(numbers, f)   // T=Int, R=String 自动从 numbers 和 f 的类型来
```

4. **值构造从元素推断类型**：

```yaoxiang
x = List(1, 2, 3)       // 推断为 List(Int)
y = List("a", "b")      // 推断为 List(String)
z = List()              // ❌ 编译错误：无法推断 T
z: List(Int) = List()   // ✅ T=Int 来自左侧注解
```

5. **类型别名**：

```yaoxiang
IntList: Type = List(Int)
StringToInt: Type = (String) -> Int
Matrix3x3: Type = Matrix(Float, 3, 3)
```

> **与旧语法对比**：`List[Int]` → `List(Int)`，`List[Int]()` → `List()`，`List[Int](1,2,3)` → `List(1,2,3)`。
> 旧的 `[]` 泛型语法已彻底移除。`[]` 仅用于数组/列表字面量和索引访问。

### 示例

#### 完整示例

```yaoxiang
// ======== 1. 接口定义 ========
// 接口 = 字段全是函数类型的记录类型
// 接口中不需要 self 参数 — 接口只定义"去掉调用者位置后的函数签名"

Drawable: Type = {
    draw: (surface: Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

Transformable: Type = {
    translate: (dx: Float, dy: Float) -> Transformable,  // 返回接口类型，具体实现返回自己的类型
    scale: (factor: Float) -> Transformable
}

// ======== 2. 类型定义 ========

Point: Type = {
    x: Float,
    y: Float,
    Drawable,
    Serializable,
    Transformable
}

Rect: Type = {
    x: Float,
    y: Float,
    width: Float,
    height: Float,
    Drawable,
    Serializable,
    Transformable
}

// ======== 3. 方法实现（普通函数 + 显式绑定）========

// 定义函数（self 只是约定名，不是关键字）
draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

bounding_box: (p: &Point) -> Rect = {
    return Rect(p.x - 1, p.y - 1, 2, 2)
}

serialize: (p: &Point) -> String = {
    return "Point(${p.x}, ${p.y})"
}

translate: (p: &Point, dx: Float, dy: Float) -> Point = {
    return Point(p.x + dx, p.y + dy)
}

scale: (p: &Point, factor: Float) -> Point = {
    return Point(p.x * factor, p.y * factor)
}

distance: (p1: &Point, p2: &Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
}

// 显式绑定 — 绑定后才有点调用语法
Point.draw = draw[0]
Point.bounding_box = bounding_box[0]
Point.serialize = serialize[0]
Point.translate = translate[0]
Point.scale = scale[0]
Point.distance = distance[0]

// Rect 的方法也类似
draw: (r: &Rect, surface: Surface) -> Void = {
    surface.draw_rect(r.x, r.y, r.width, r.height)
}
Rect.draw = draw[0]

bounding_box: (r: &Rect) -> Rect = r
Rect.bounding_box = bounding_box[0]

serialize: (r: &Rect) -> String = {
    return "Rect(${r.x}, ${r.y}, ${r.width}, ${r.height})"
}
Rect.serialize = serialize[0]

translate: (r: &Rect, dx: Float, dy: Float) -> Rect = {
    return Rect(r.x + dx, r.y + dy, r.width, r.height)
}
Rect.translate = translate[0]

scale: (r: &Rect, factor: Float) -> Rect = {
    return Rect(r.x * factor, r.y * factor, r.width * factor, r.height * factor)
}
Rect.scale = scale[0]

// ======== 4. 使用 ========

// 创建实例
p: Point = Point(1.0, 2.0)
r: Rect = Rect(0.0, 0.0, 10.0, 20.0)

// 方法调用（语法糖）
p.draw(screen)
r.draw(screen)

// 普通方法调用（直接调用）
d: Float = distance(p, Point(0.0, 0.0))

// 链式调用
p2: Point = p.translate(1.0, 1.0).scale(2.0)

// 接口赋值
drawables: List(Drawable) = [p, r]
for d in drawables {
    d.draw(screen)
}

// 泛型函数（RFC-023 语法：调用时省略类型参数，自动推断）
process_all: (items: List(T)) -> Void = {
    for item in items {
        print(item.serialize())
    }
}

process_all([p, r])
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

### 接口直接赋值与编译期优化

接口类型支持直接赋值，编译器会根据赋值的右值类型自动选择最优的调用策略：

```yaoxiang
// 直接赋值具体类型 → 编译期可确定具体类型，零开销调用
d: Drawable = Circle(1)
d.draw(screen)  // 编译后：直接调用 circle_draw(screen)，无 vtable

// 函数返回值 → 编译期无法确定具体类型，使用 vtable
d: Drawable = get_shape()
d.draw(screen)  // 通过 vtable 查找方法

// 异构集合 → 使用 vtable
shapes: List(Drawable) = [Circle(1), Rect(2, 3)]
for s in shapes {
    s.draw(screen)  // 通过 vtable 查找方法
}
```

**编译期优化策略**：

| 场景 | 推断结果 | 调用方式 |
|------|----------|----------|
| `d: Drawable = Circle(1)` | 具体类型 Circle | 直接调用（零开销） |
| `d: Drawable = get_shape()` | 未知 | vtable |
| `shapes: List(Drawable) = [...]` | 异构 | vtable |

**规则**：
1. 当右值是具体类型构造器且编译期可确定时，生成直接调用 IR
2. 当右值类型无法在编译期确定时，回退到 vtable 机制
3. vtable 兜底保证运行时多态的正确性

### 鸭子类型支持

```yaoxiang
// 只要有相同方法，就可以赋值给接口类型
CustomPoint: Type = {
    draw: (self: CustomPoint, surface: Surface) -> Void,
    x: Float,
    y: Float
}

custom: CustomPoint = CustomPoint(
    (self: CustomPoint, surface: Surface) => surface.plot(self.x, self.y),
    1.0,
    2.0
)
```

### 语法变化

| 之前 | 之后 |
|------|------|
| `type Point = Point(x: Float, y: Float)` | `type Point = { x: Float, y: Float }` |
| `type Result(T, E) = ok(T) \| err(E)` | `Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }` |
| 需要 `impl` 关键字 | 无需关键字，接口名写在类型体后 |

## 语法设计说明：具名函数本质是 Lambda 的语法糖

### 核心理解

**具名函数和 Lambda 表达式是同一个东西！** 唯一的区别是：具名函数给 Lambda 取了个名字。

```yaoxiang
// 这两者本质完全相同
add: (a: Int, b: Int) -> Int = a + b           // 具名函数（推荐）
add: (a: Int, b: Int) -> Int = (a, b) => a + b        // Lambda 形式（完全等价）
```

### 语法糖模型

```
// 具名函数 = Lambda + 名字
name: (Params) -> ReturnType = body

// 本质上是
name: (Params) -> ReturnType = (params) => body
```

**关键点**：当签名完整声明了参数类型，Lambda 头部的参数名就变成了冗余，可以省略。

### 参数作用域规则

**参数覆盖外层变量**：签名中的参数作用域覆盖函数体，内部作用域优先级更高。

```yaoxiang
x = 10  // 外层变量

double: (x: Int) -> Int = x * 2  // ✅ 参数 x 覆盖外层 x，结果为 20
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
// ✅ 推荐：签名完整，Lambda 头部省略
add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1
main: () -> Void = { print("hi") }

// ✅ 合法：Lambda 头中标注类型
double = (x: Int) => x * 2

// ✅ 合法：两边都标注
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
// 1. 清晰的错误信息
// 编译错误示例：
// Error: Point does not implement Serializable
//   Required method 'serialize: (self: Point) -> String' not found
//   Note: Define Point.serialize to implement Serializable

// 2. 类型推导
// 可以省略类型，由编译器推导
Point.draw = (self: Point, surface: Surface) => surface.plot(self.x, self.y)

// 3. IDE 提示
// IDE 自动提示缺失的方法
```

### 风险

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| 解析复杂度 | 统一语法可能增加解析复杂度 | 使用递归下降解析器 |
| 性能开销 | vtable 查找可能有额外开销 | 编译期单态化优化 |

---

## 彩蛋 🎮：语言之源

> ✨ **Type: Type = Type** ✨

```yaoxiang
// 尝试定义类型的类型...
Type: Type = Type
```

**警告**：此乃**不可名状**之物！

```
╔══════════════════════════════════════════════════════════════╗
║                                                              ║
║   一生二，二生三，三生万物。                                   ║
║   易有太极，是生两仪。                                         ║
║                                                              ║
║   Type: Type = Type                                          ║
║   此乃爻象之源，语言之边界。                                   ║
║   编译器在此沉默，哲学在此驻足。                               ║
║                                                              ║
║   感谢你触达语言的哲学边界。                                   ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝
```

> **注**：编译器无法正确处理 `Type: Type = Type`（会导致 Type0/Type1 宇宙悖论），但我们特意保留这个"彩蛋"——当你尝试编译它时，会收到一条来自语言创始人的禅意消息。这不仅是技术边界，更是 YaoXiang 对类型哲学的致敬。

---

## 附录

### 语法 BNF

```bnf
program ::= statement*

statement ::= declaration | expression

# 统一声明：name: Type = expression
declaration ::= identifier ':' type_expr '=' expression

# 类型表达式
type_expr ::= identifier
       | identifier '(' type_expr (',' type_expr)* ')'      # 类型应用
       | '(' type_expr (',' type_expr)* ')' '->' type_expr       # 函数类型
       | '{' type_field* '}'                       # 记录/接口类型
       | 'Type'                                    # 元类型

type_field ::= identifier ':' type_expr
             | identifier                           # 接口约束

# 泛型参数：作为函数类型的一部分，如 (T: Type, R: Type) -> (...)
# 无需独立的 BNF 规则——: Type 参数就是普通函数参数

# 表达式
expression ::= literal
              | identifier
              | identifier '(' expression (',' expression)* ')'  # 函数调用 / 构造器调用
              | '(' expression (',' expression)* ')'              # 元组
              | expression '.' identifier '(' arguments? ')'    # 方法调用
              | lambda
              | '{' field ':' expression (',' field ':' expression)* '}'

arguments ::= expression (',' expression)*

lambda ::= '(' parameter_list? ')' '=>' block

block ::= expression | '{' expression* '}'
```

### 术语表

| 术语 | 定义 |
|------|------|
| 声明 | `name: type = value` 形式的赋值语句 |
| 记录类型 | 包含命名字段的 `{ ... }` 类型 |
| 接口 | 字段全为函数类型的记录类型 |
| 泛型类型 | 定义为 `Name: (T: Type) -> Type = { ... }` 的类型，接受类型参数 |
| 命名空间函数 | `Type.name` 形式的函数，属于 Type 命名空间。不隐含任何绑定 |
| 方法绑定 | `Type.name = func[n]`，将 func 的位置 n 绑定为调用者，使 `obj.name(args)` 语法可用 |
| 泛型函数 | 使用 `(T: Type)` 语法的函数，类型参数作为第一个参数组 |
| 元类型 | `Type`，语言中唯一的类型层级标记 |

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

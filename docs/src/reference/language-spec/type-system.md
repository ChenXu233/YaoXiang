# 类型系统规范

本文件定义 YaoXiang 编程语言的类型系统规范，包括基本类型、复合类型、泛型和 trait。

---

## 第零章：理论基础

### 0.1 Curry-Howard 同构

Curry-Howard 同构（Curry-Howard correspondence）是 YaoXiang 类型系统的理论基础。它揭示了编程语言的类型系统与数理逻辑之间的深层对应关系：

| 逻辑学 | 编程语言 |
|--------|----------|
| 命题 \(P\) | 类型 `Type` |
| 证明 \(p: P\) | 程序 `x: T = ...` |
| 蕴含 \(P \rightarrow Q\) | 函数类型 `(P) -> Q` |
| 合取 \(P \wedge Q\) | 积类型 `{ a: P, b: Q }` |
| 析取 \(P \vee Q\) | 和类型 `{ a(P) \| b(Q) }` |
| 全称量化 \(\forall x:T. P(x)\) | 泛型 `(T: Type) -> ...` |
| 真 \(\top\) | 空类型 `{}` |
| 假 \(\bot\) | `Void` / `Never` |
| 类型宇宙 \(Type_n : Type_{n+1}\) | 宇宙分层（防 Russell 悖论） |
| 数学归纳法 | 类型级 `match` |

### 0.2 类型即命题，程序即证明

在 YaoXiang 中，这一对应关系是设计的一等原则：

- **一个类型就是一个逻辑命题**。`Int` 是"整数存在"的命题，`fn(a: Int, b: Int) -> Int` 是"给定两个整数，存在一个整数"的命题。
- **类型检查就是验证证明**。当一个程序通过类型检查，相当于一个逻辑命题被构造性证明。
- **终止的类型级计算对应正确的归纳推理**。YaoXiang 的类型族（如 `Add` 在 `Nat` 上的模式匹配）本质上是数学归纳法的类型级编码。

### 0.3 对语言设计的影响

Curry-Howard 同构在 YaoXiang 中的具体体现：

1. **宇宙分层**（RFC-010）：`Type₀ : Type₁ : Type₂ …` 避免 `Type: Type` 导致的逻辑悖论（Girard 悖论）
2. **类型族**（RFC-011）：自然数 `Nat(Zero/Succ)` 的类型级模式匹配对应 Peano 公理下的归纳证明
3. **条件类型**（RFC-011）：`If: (C: Bool, T: Type, E: Type) -> Type` 对应逻辑中的 case 析取
4. **值依赖类型**（RFC-011）：`Vec: (n: Int) -> Type` 对应"对每个整数 n 存在一个类型"的有穷量化

---

## 第一章：类型分类

### 1.1 类型表达式

```
TypeExpr    ::= PrimitiveType
              | StructType
              | EnumType
              | InterfaceType
              | TupleType
              | FnType
              | GenericType
              | TypeRef
              | TypeUnion
              | TypeIntersection
              | ConstrainedType
              | AssociatedType
```

---

## 第二章：基本类型

### 2.1 原类型

| 类型 | 描述 | 默认大小 |
|------|------|----------|
| `Type` | 元类型 | 0 字节 |
| `Void` | 空值 | 0 字节 |
| `Bool` | 布尔值 | 1 字节 |
| `Int` | 有符号整数 | 8 字节 |
| `Uint` | 无符号整数 | 8 字节 |
| `Float` | 浮点数 | 8 字节 |
| `String` | UTF-8 字符串 | 可变 |
| `Char` | Unicode 字符 | 4 字节 |
| `Bytes` | 原始字节 | 可变 |

带位宽的整数：`Int8`, `Int16`, `Int32`, `Int64`, `Int128`
带位宽的浮点：`Float32`, `Float64`

---

## 第三章：复合类型

### 3.1 记录类型

**统一语法**：`Name: Type = { field1: Type1, field2: Type2, ... }`

```
RecordType  ::= '{' FieldList? '}'
FieldList   ::= Field (',' Field)* ','?
Field       ::= Identifier ':' TypeExpr
            |  Identifier                 // 接口约束
```

```yaoxiang
// 简单记录类型
Point: Type = { x: Float, y: Float }

// 空记录类型
Empty: Type = {}

// 带泛型的记录类型
Pair: (T: Type) -> Type = { first: T, second: T }

// 实现接口的记录类型
Point: Type = {
    x: Float,
    y: Float,
    Drawable,
    Serializable
}
```

**规则**：
- 记录类型使用花括号 `{}` 定义
- 字段名后直接跟冒号和类型
- 接口名写在类型体内表示实现该接口

#### 3.1.1 字段默认值

类型字段可以指定默认值，构造时可选提供：

```yaoxiang
// 有默认值的字段 - 构造时可选
Point: Type = {
    x: Float = 0,
    y: Float = 0
}

// 使用
Point()           // -> Point(x=0, y=0)
Point(x=1)       // -> Point(x=1, y=0)
Point(x=1, y=2) // -> Point(x=1, y=2)

// 无默认值的字段 - 构造时必填
Point2: Type = {
    x: Float,
    y: Float
}

// 使用
Point2(x=1, y=2) // 正确
Point2()          // 错误
```

**规则**：
- `field: Type = expression` -> 有默认值，构造时可选
- `field: Type` -> 无默认值，构造时必填

#### 3.1.2 内置绑定

在类型定义体内可以直接绑定方法：

```yaoxiang
// 方式1：引用外部函数绑定
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]    // 绑定到位置0
}
// 调用：p1.distance(p2) -> distance(p1, p2)

// 方式2：匿名函数 + 位置绑定
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
// 调用：p1.distance(p2) -> distance(p1, p2)
```

### 3.2 枚举类型（变体类型）

```
EnumType    ::= '{' Variant ('|' Variant)* '}'
Variant     ::= Identifier (':' TypeExpr)?
```

**语法**：`Name: Type = { Variant1 | Variant2(params) | ... }`

```yaoxiang
// 无参变体
Color: Type = { red | green | blue }

// 有参变体
Option: (T: Type) -> Type = { some(T) | none }

// 混合
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

// 无参变体等价于无参构造器
Bool: Type = { true | false }
```

### 3.3 接口类型

```
InterfaceType ::= '{' FnField (',' FnField)* ','?
FnField       ::= Identifier ':' FnType
FnType        ::= '(' ParamTypes? ')' '->' TypeExpr
```

**语法**：接口是字段全为函数类型的记录类型

```yaoxiang
// 接口定义
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

// 空接口
EmptyInterface: Type = {}
```

**接口实现**：类型通过在定义末尾列出接口名来实现接口

```yaoxiang
// 实现接口的类型
Point: Type = {
    x: Float,
    y: Float,
    Drawable,        // 实现 Drawable 接口
    Serializable     // 实现 Serializable 接口
}
```

**接口直接赋值**：具体类型可以直接赋值给接口类型变量（结构化子类型）

```yaoxiang
// 直接赋值（编译期可确定具体类型 -> 零开销调用）
d: Drawable = Circle(1)
d.draw(screen)        // 编译后：直接调用 circle_draw，无 vtable

// 函数返回值（编译期无法确定 -> vtable 调用）
d: Drawable = get_shape()
d.draw(screen)        // 通过 vtable 查找方法

// 接口作为函数参数
process: (d: Drawable) -> Void = d.draw(screen)
```

**编译期优化策略**：

| 场景 | 推断结果 | 调用方式 |
|------|----------|----------|
| 直接赋值具体类型 | 具体类型可确定 | 直接调用（零开销） |
| 函数返回值 | 未知 | vtable |
| 异构集合 | 多个类型 | vtable |

### 3.4 元组类型

```
TupleType   ::= '(' TypeList? ')'
TypeList    ::= TypeExpr (',' TypeExpr)* ','?
```

### 3.5 函数类型

```
FnType      ::= '(' ParamList? ')' '->' TypeExpr
ParamList   ::= TypeExpr (',' TypeExpr)*
```

---

## 第四章：泛型

### 4.1 泛型参数语法

```
GenericType     ::= Identifier '[' TypeArgList ']'
TypeArgList     ::= TypeExpr (',' TypeExpr)* ','?
GenericParams   ::= '[' Identifier (',' Identifier)* ']'
                 |  '[' Identifier ':' TypeBound (',' Identifier ':' TypeBound)* ']'
TypeBound       ::= Identifier
                 |  Identifier '+' Identifier ('+' Identifier)*
```

### 4.2 泛型类型定义

```yaoxiang
// 基础泛型类型
Option: (T: Type) -> Type = {
    some: (T) -> Self,
    none: () -> Self
}

Result: (T: Type, E: Type) -> Type = {
    ok: (T) -> Self,
    err: (E) -> Self
}

List: (T: Type) -> Type = {
    data: Array(T),
    length: Int,
    push: (self: List(T), item: T) -> Void,
    get: (self: List(T), index: Int) -> Option(T)
}
```

### 4.3 类型推导

```yaoxiang
// 编译器自动推导泛型参数
numbers: List(Int) = List(1, 2, 3)  // 编译器推导 List(Int)
```

---

## 第五章：类型约束

### 5.1 单一约束

```
ConstrainedType ::= '[' Identifier ':' TypeBound ']' TypeExpr
```

```yaoxiang
// 接口类型定义（作为约束）
Clone: Type = {
    clone: (Self) -> Self
}

// 使用约束
clone: (T: Clone)(value: T) -> T = value.clone()
```

### 5.2 多重约束

```yaoxiang
// 多重约束语法
combine: (T: Clone + Add)(a: T, b: T) -> T = {
    a.clone() + b
}

// 泛型容器的排序
sort: (T: Clone + PartialOrd)(list: List(T)) -> List(T) = {
    result = list.clone()
    quicksort(&mut result)
    return result
}
```

### 5.3 函数类型约束

```yaoxiang
// 高阶函数约束
call_twice: (T: Type, F: Fn() -> T)(f: F) -> (T, T) = (f(), f())

compose: (A: Type, B: Type, C: Type, F: Fn(A) -> B, G: Fn(B) -> C)(a: A, f: F, g: G) -> C = g(f(a))
```

---

## 第六章：关联类型

### 6.1 关联类型定义

```
AssociatedType ::= Identifier ':' TypeExpr
```

```yaoxiang
// Iterator trait（使用记录类型语法）
Iterator: (T: Type) -> Type = {
    Item: T,                    // 关联类型
    next: (Self) -> Option(T),
    has_next: (Self) -> Bool
}

// 使用关联类型
collect: (T: Type, I: Iterator(T))(iter: I) -> List(T) = {
    result = List(T)()
    while iter.has_next() {
        if let Some(item) = iter.next() {
            result.push(item)
        }
    }
    return result
}
```

### 6.2 泛型关联类型（GAT）

```yaoxiang
// 更复杂的关联类型
Container: (T: Type) -> Type = {
    Item: T,
    IteratorType: Iterator(T),  // 关联类型也是泛型的
    iter: (Self) -> IteratorType
}
```

---

## 第七章：编译期泛型

### 7.1 字面量类型约束

```
LiteralType   ::= Identifier ':' Int          // 编译期常量
CompileTimeFn ::= '[' Identifier ':' Int ']' '(' Identifier ')' '->' TypeExpr
```

**核心设计**：用 `(n: Int)` 泛型参数 + `(n: n)` 值参数，区分编译期常量与运行时值。

```yaoxiang
// 编译期阶乘：参数必须是编译期已知的字面量
factorial: (n: Int)(n: n) -> Int = {
    match n {
        0 => 1,
        _ => n * factorial(n - 1)
    }
}

// 编译期常量数组
StaticArray: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // 编译期已知大小的数组
    length: N
}

// 使用方式
arr: StaticArray(Int, factorial(5))  // 编译器在编译期计算 factorial(5) = 120
```

### 7.2 编译期常量数组

```yaoxiang
// 矩阵类型使用
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows)
}

// 编译期维度验证
identity_matrix: (T: Add + Zero + One, N: Int)(size: N) -> Matrix(T, N, N) = {
    // ...
}
```

---

## 第八章：条件类型

### 8.1 If 条件类型

```
IfType        ::= 'If' '[' BoolExpr ',' TypeExpr ',' TypeExpr ']'
```

```yaoxiang
// 类型级 If
If: (C: Bool, T: Type, E: Type) -> Type = match C {
    True => T,
    False => E
}

// 示例：编译期分支
NonEmpty: (T: Type) -> Type = If(T != Void, T, Never)

// 编译期验证
Assert: (C: Bool) -> Type = match C {
    True => Void,
    False => compile_error("Assertion failed")
}
```

### 8.2 类型族

```yaoxiang
// 编译期类型转换
AsString: (T: Type) -> Type = match T {
    Int => String,
    Float => String,
    Bool => String,
    _ => String
}
```

---

## 第九章：类型联合与交集

### 9.1 类型联合

```
TypeUnion     ::= TypeExpr '|' TypeExpr
```

### 9.2 类型交集

```
TypeIntersection ::= TypeExpr '&' TypeExpr
```

**语法**：类型交集 `A & B` 表示同时满足 A 和 B 的类型

```yaoxiang
// 接口组合 = 类型交集
DrawableSerializable: Type = Drawable & Serializable

// 使用交集类型
process: (T: Drawable & Serializable)(item: T, screen: Surface) -> String = {
    item.draw(screen)
    return item.serialize()
}
```

---

## 第十章：函数重载与特化

### 10.1 函数重载

```yaoxiang
// 基本特化：使用函数重载（编译器自动选择）
sum: (arr: Array(Int)) -> Int = {
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Array(Float)) -> Float = {
    return simd_sum_float(arr.data, arr.length)
}

// 通用实现
sum: (T: Add)(arr: Array(T)) -> T = {
    result = Zero::zero()
    for item in arr {
        result = result + item
    }
    return result
}
```

### 10.2 平台特化

```yaoxiang
// 平台类型枚举（标准库定义）
Platform: Type = X86_64 | AArch64 | RISC_V | ARM | X86

// P 是预定义泛型参数名，代表当前编译平台
sum: (P: X86_64)(arr: Array(Float)) -> Float = {
    return avx2_sum(arr.data, arr.length)
}

sum: (P: AArch64)(arr: Array(Float)) -> Float = {
    return neon_sum(arr.data, arr.length)
}
```

---

## 第十一章：类型属性

YaoXiang 通过类型属性（type properties）标记类型的复制和并发语义。类型属性由编译器自动推导，用户不直接标注。

### 11.1 Dup（隐式浅复制）

**Dup**（Duplicable）是隐式浅复制标记。实现 Dup 的类型，赋值和传参时自动进行浅复制（bitwise copy），原值和新值完全独立。

**Dup 类型**：
| 类型 | 说明 |
|------|------|
| `Int`, `Int8`..`Int128` | 所有整数 |
| `Float`, `Float32`, `Float64` | 所有浮点 |
| `Bool` | 布尔值 |
| `Char` | Unicode 字符 |
| `String` | UTF-8 字符串（浅复制） |
| `Bytes` | 原始字节（浅复制） |
| `&T` | 读取令牌（见第十二章） |

**非 Dup 类型**（默认 Move）：
| 类型 | 说明 |
|------|------|
| `&mut T` | 写入令牌，线性类型 |
| 大多数 struct | 默认 Move，除非所有字段都是 Dup |
| enum（有参变体） | 若承载的数据非 Dup，则整个变体非 Dup |

```yaoxiang
// Dup 类型：自由复制
a: Int = 42
b = a           // 浅复制，a 仍然可用
c = a           // 可多次复制

// 非 Dup 类型（Move 默认）
p: Point = Point(1.0, 2.0)
q = p           // Move，p 不可再读
// r = p        // ❌ 编译错误：p 已被移动
```

**Dup 的自动推导**：
- 基本类型（Int, Float, Bool, Char, String, Bytes）自动实现 Dup
- 结构体：当且仅当所有字段类型都是 Dup 时，自动实现 Dup
- 枚举变体：当且仅当所有变体承载的类型都是 Dup 时，自动实现 Dup

### 11.2 Clone（显式深复制）

**Clone** 是显式深复制接口。所有类型都可以实现 Clone，提供 `.clone()` 方法。

```yaoxiang
// Clone 接口定义（标准库）
Clone: Type = {
    clone: (Self) -> Self
}

// 使用
p: Point = Point(1.0, 2.0)
backup = p.clone()    // 深复制，p 仍然可用
p2 = p.clone()        // 可多次克隆
```

### 11.3 Dup 与 Clone 的关系

**Dup 蕴含 Clone，但 Clone 不蕴含 Dup**：

```
Dup ⇒ Clone（字段逐位复制即可实现 .clone()）
Clone ⇏ Dup（显式深复制不妨碍默认 Move 语义）
```

```yaoxiang
// Dup ⇒ Clone：Int 既是 Dup 也是 Clone
x: Int = 42
y = x              // Dup：隐式浅复制
z = x.clone()      // Clone：显式深复制（效果相同）

// Clone ⇏ Dup：Point 可以 Clone，但默认 Move
p: Point = Point(1.0, 2.0)
q = p.clone()      // Clone：显式深复制，p 仍然可用
r = p              // Move：所有权转移，因为 Point 不是 Dup
```

**设计意图**：
- Dup 是"这个类型复制起来很便宜/自然"的承诺
- Clone 是"我可以给你一个独立的副本"的能力
- 大多数 struct 不自动实现 Dup，保持 Move 默认——零拷贝高性能

### 11.4 Send / Sync（非用户可见）

**Send** 和 **Sync** 不是用户可见的类型属性，由编译器和 `ref` 关键字自动处理。

| 属性 | 含义 | 用户如何触发 |
|------|------|-------------|
| **Send** | 可安全跨任务传递 | `ref` 跨任务时编译器自动选 Arc |
| **Sync** | 可安全跨任务共享 | `ref` 跨任务时编译器自动选 Arc |

**自动推导规则（编译器内部）**：

| 类型 | Send | Sync | 说明 |
|------|------|------|------|
| 值类型（Int, Float, Point...） | 是 | 是 | 值传递天然安全 |
| `ref T` | 是 | 是 | 编译器自动选 Rc（单任务）/ Arc（跨任务） |
| `&T` / `&mut T` | 否 | 否 | 令牌不能跨任务边界 |
| `*T` | 否 | 否 | 裸指针单线程 |

```yaoxiang
// 跨任务共享：ref 自动处理 Send/Sync
@block
main: () -> Void = {
    data = ref heavy_data
    spawn { use(data) }    // 编译器：跨任务 → Arc（Send + Sync）
    spawn { use(data) }    // 编译器：跨任务 → Arc（Send + Sync）
}

// 令牌不能跨任务（非 Send）
bad_task: (p: &Point) -> Void = {
    spawn { print(p.x) }   // ❌ 编译错误：&T 未实现 Send
}
```

**用户不需要关心 Send/Sync**：`ref` 关键字封装了所有并发安全逻辑。

---

## 第十二章：借用令牌类型

### 12.1 核心概念

`&T` 和 `&mut T` 是**零大小的编译期令牌类型**。它们不是"引用"，而是"访问权限的类型级证明"。

```
&T      →  零大小，Dup（可复制），授予只读权限
&mut T  →  零大小，Linear（非 Dup），授予独占读写权限
```

**关键特性**：
- 令牌是**普通类型**，遵循和所有其他类型一样的作用域规则
- 不需要生命周期标注 `'a`
- 不需要专用借用检查器——类型属性（Dup/Linear）自然推导权限
- 编译后完全消失，零运行时开销

### 12.2 基本使用

```yaoxiang
// 方法端：声明参数类型，决定需要的权限
Point.print: (self: &Point) -> Void = {
    print(self.x)               // &Point 令牌授予读权限
    print(self.y)
}

Point.shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
    self.x = self.x + dx        // &mut Point 令牌授予写权限
    self.y = self.y + dy
}

// 调用端：编译器自动选择借用或 Move
p = Point(1.0, 2.0)
p.print()                       // 编译器自动创建 &Point 令牌
p.shift(1.0, 1.0)               // 编译器自动创建 &mut Point 令牌
p.print()                       // OK，上一个令牌已随 shift 调用结束而释放

// 多个 &T 令牌共存——Dup 类型允许自由复制
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 令牌的作用域与传播

令牌是普通类型，因此支持所有普通类型的操作：

**返回令牌**——令牌随返回值一起传播：

```yaoxiang
// ✅ 子令牌和父令牌一起返回
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // 令牌返回给调用者
print(px_ref)                    // OK，令牌仍在作用域
```

**存结构体**——结构体可以携带令牌字段：

```yaoxiang
// ✅ 结构体携带令牌作为字段
Window: Type = {
    target: Point,
    view: &Point,              // 令牌字段——持有对 target 的只读视图
}
```

**闭包捕获**——闭包捕获令牌就像捕获任何值：

```yaoxiang
// ✅ 闭包捕获 &Float 令牌（Dup 类型，自由复制到闭包中）
filter_by_threshold: (items: List(Point), threshold: &Float) -> List(Point) = {
    items.filter(|p| p.x > threshold)
}
```

### 12.4 自动借用选择

调用端编译器按以下优先级自动选择：

```
1. 如果实参后续还有使用 → 优先创建令牌（&T 或 &mut T，根据方法签名）
2. 如果实参后续不再使用 → Move
3. 优先匹配顺序：&T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // print 声明 &self → 编译器创建 &Point 令牌
p.shift(1.0, 1.0)  // shift 声明 &mut self → 编译器创建 &mut Point 令牌
p2 = p             // 后续不再使用 → Move
```

### 12.5 冻结机制

`&mut T` 令牌可以临时"冻结"以产生 `&T` 令牌：

```yaoxiang
modify_and_read: (p: &mut Point) -> Void = {
    p.x = 10.0                   // 使用 &mut Point 修改
    
    // 冻结 &mut，获取只读视图
    view: &Point = freeze(p)     // p 在此处被冻结
    print(view.x)                // 通过 &Point 读取
    print(view.y)
    // view 离开作用域，冻结解除
    
    p.y = 20.0                   // &mut Point 恢复可用
}
```

`freeze` 的语义：
- 接受 `&mut T`，返回 `&T`
- 在 `&T` 存活期间，原 `&mut T` 不可用
- `&T` 离开作用域后，`&mut T` 自动恢复
- 这是**流敏感活性分析**——编译器在函数体内追踪令牌状态

### 12.6 令牌冲突检测

编译器对令牌值做**流敏感活性分析**，追踪每个令牌的状态（活跃/冻结/已移动）：

```yaoxiang
// ❌ &mut 和派生的 &T 不能同时活跃
bad_alias: (p: &mut Point) -> Void = {
    view: &Point = freeze(p)     // p 被冻结
    p.x = 10.0                   // ❌ 编译错误：WriteToken 在冻结状态
    print(view.x)
}

// ✅ 冻结解除后可继续使用 &mut
good_seq: (p: &mut Point) -> Void = {
    view: &Point = freeze(p)     // p 被冻结
    print(view.x)                // 使用 &T
    // view 离开作用域，冻结解除
    p.x = 10.0                   // ✅ WriteToken 已恢复
}

// ❌ 同一实参不能同时创建 &mut 令牌和其他令牌
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p 同时派生 &mut 和 & 令牌
```

### 12.7 编译器内部：品牌机制

用户从不接触品牌。编译器在内部为每个令牌分配编译期唯一标识：

```
用户看到的           编译器内部表示
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N 是编译期唯一整数
&mut Point     →  WriteToken(Point, #M)   // #M 是编译期唯一整数
```

品牌的用途：
- **防伪造**：令牌只能从所有者胶囊或 freeze 操作获得，不能凭空构造
- **关联追踪**：字段访问派生的 `&Float` 携带派生品牌（`#N.field_x`），编译器可追踪到父令牌
- **冲突检测**：同源 WriteToken 和派生 ReadToken 不能同时活跃

品牌在单态化和内联后完全消失，生成的机器码中不存在。**零运行时开销。**

### 12.8 令牌 Sum 类型

```
&BorrowToken ::= &T          // ReadToken（Dup，可复制）
               | &mut T      // WriteToken（Linear，独占）
```

### 12.9 借用令牌 vs ref

| | `&T` / `&mut T` | `ref` |
|------|------|------|
| 做什么 | 看一眼/原地改 | 共享持有 |
| 范围 | 随令牌值的作用域 | 跨作用域 |
| 成本 | 零开销（零大小类型，编译后消失） | Rc 或 Arc（编译器选） |
| 逃逸 | 可（令牌随返回值/结构体/闭包传播） | 本来就是用来逃逸的 |
| 跨任务 | 不可（令牌未实现 Send） | 可（编译器自动选 Arc） |
| 环检测 | 不涉及 | 任务内静默，跨任务 lint |

---

## 附录：类型定义速查

### A.1 类型定义

```
// === 记录类型（花括号） ===

// 结构体
Point: Type = { x: Float, y: Float }

// 枚举（变体类型）
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }
Status: Type = { pending | processing | completed }

// === 接口类型（花括号，字段全为函数） ===

// 接口定义
Serializable: Type = { serialize: () -> String }

// 实现接口的类型
Point: Type = {
    x: Float,
    y: Float,
    Serializable    // 实现 Serializable 接口
}

// === 函数类型 ===

Adder: Type = (Int, Int) -> Int
```

### A.2 泛型语法

```
// 泛型类型
List: (T: Type) -> Type = { data: Array(T), length: Int }
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

// 泛型函数
map: (T: Type, R: Type)(list: List(T), f: Fn(T) -> R) -> List(R) = { ... }

// 类型约束
clone: (T: Clone)(value: T) -> T = value.clone()
combine: (T: Clone + Add)(a: T, b: T) -> T = body

// 关联类型
Iterator: (T: Type) -> Type = { Item: T, next: () -> Option(T) }

// 编译期泛型
factorial: (n: Int)(n: n) -> Int = { ... }
StaticArray: (T: Type, N: Int) -> Type = { data: T(N), length: N }

// 条件类型
If: (C: Bool, T: Type, E: Type) -> Type = match C { True => T, False => E }

// 函数特化
sum: (arr: Array(Int)) -> Int = { ... }
sum: (arr: Array(Float)) -> Float = { ... }
```

### A.3 类型属性速查

```
// === Dup（隐式浅复制）===
// 基本类型自动 Dup
Int, Float, Bool, Char, String, Bytes   // Dup
&T                                      // Dup（共享读取令牌）

// 非 Dup（默认 Move）
&mut T                                  // Linear（独占写入令牌）
大多数 struct                            // Move 默认

// Dup 蕴含 Clone（字段逐位复制），但 Clone 不蕴含 Dup

// === Clone（显式深复制）===
value.clone()                           // 显式深复制

// === Send / Sync（非用户可见）===
// 由 ref 关键字和编译器自动处理
// 值类型：Send + Sync
// ref T：Send + Sync（编译器自动选 Rc/Arc）
// &T / &mut T：非 Send（不能跨任务）
// *T：非 Send（裸指针单线程）
```

### A.4 借用令牌速查

```
// === 借用令牌 ===
&T              // 零大小编译期读令牌，Dup（可复制）
&mut T          // 零大小编译期写令牌，Linear（不可复制）

// 调用端自动选择
// 1. 实参后续还有使用 → 创建令牌
// 2. 实参后续不再使用 → Move
// 3. 优先匹配：&T < &mut T < Move

// 令牌传播
// ✅ 可返回、可存结构体、可被闭包捕获
// ❌ 不可跨任务（未实现 Send）

// 冻结
view: &T = freeze(mut_ref)   // &mut T → &T（冻结期间 &mut T 不可用）
```

# 类型系统规范

本文件定义 YaoXiang 编程语言的类型系统规范，包括基本类型、复合类型、泛型和 trait。

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
Pair: Type[T] = { first: T, second: T }

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
Option: Type[T] = { some(T) | none }

// 混合
Result: Type[T, E] = { ok(T) | err(E) }

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
Option: Type[T] = {
    some: (T) -> Self,
    none: () -> Self
}

Result: Type[T, E] = {
    ok: (T) -> Self,
    err: (E) -> Self
}

List: Type[T] = {
    data: Array[T],
    length: Int,
    push: [T](self: List[T], item: T) -> Void,
    get: [T](self: List[T], index: Int) -> Option[T]
}
```

### 4.3 类型推导

```yaoxiang
// 编译器自动推导泛型参数
numbers: List[Int] = List(1, 2, 3)  // 编译器推导 List[Int]
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
clone: [T: Clone](value: T) -> T = value.clone()
```

### 5.2 多重约束

```yaoxiang
// 多重约束语法
combine: [T: Clone + Add](a: T, b: T) -> T = {
    a.clone() + b
}

// 泛型容器的排序
sort: [T: Clone + PartialOrd](list: List[T]) -> List[T] = {
    result = list.clone()
    quicksort(&mut result)
    return result
}
```

### 5.3 函数类型约束

```yaoxiang
// 高阶函数约束
call_twice: [T, F: Fn() -> T](f: F) -> (T, T) = (f(), f())

compose: [A, B, C, F: Fn(A) -> B, G: Fn(B) -> C](a: A, f: F, g: G) -> C = g(f(a))
```

---

## 第六章：关联类型

### 6.1 关联类型定义

```
AssociatedType ::= Identifier ':' TypeExpr
```

```yaoxiang
// Iterator trait（使用记录类型语法）
Iterator: Type[T] = {
    Item: T,                    // 关联类型
    next: (Self) -> Option[T],
    has_next: (Self) -> Bool
}

// 使用关联类型
collect: [T, I: Iterator[T]](iter: I) -> List[T] = {
    result = List[T]()
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
Container: Type[T] = {
    Item: T,
    IteratorType: Iterator[T],  // 关联类型也是泛型的
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

**核心设计**：用 `[n: Int]` 泛型参数 + `(n: n)` 值参数，区分编译期常量与运行时值。

```yaoxiang
// 编译期阶乘：参数必须是编译期已知的字面量
factorial: [n: Int](n: n) -> Int = {
    match n {
        0 => 1,
        _ => n * factorial(n - 1)
    }
}

// 编译期常量数组
StaticArray: Type[T, N: Int] = {
    data: T[N],      // 编译期已知大小的数组
    length: N
}

// 使用方式
arr: StaticArray[Int, factorial(5)]  // 编译器在编译期计算 factorial(5) = 120
```

### 7.2 编译期常量数组

```yaoxiang
// 矩阵类型使用
Matrix: Type[T, Rows: Int, Cols: Int] = {
    data: Array[Array[T, Cols], Rows]
}

// 编译期维度验证
identity_matrix: [T: Add + Zero + One, N: Int](size: N) -> Matrix[T, N, N] = {
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
If: Type[C: Bool, T, E] = match C {
    True => T,
    False => E
}

// 示例：编译期分支
NonEmpty: Type[T] = If[T != Void, T, Never]

// 编译期验证
Assert: Type[C: Bool] = match C {
    True => Void,
    False => compile_error("Assertion failed")
}
```

### 8.2 类型族

```yaoxiang
// 编译期类型转换
AsString: Type[T] = match T {
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
process: [T: Drawable & Serializable](item: T, screen: Surface) -> String = {
    item.draw(screen)
    return item.serialize()
}
```

---

## 第十章：函数重载与特化

### 10.1 函数重载

```yaoxiang
// 基本特化：使用函数重载（编译器自动选择）
sum: (arr: Array[Int]) -> Int = {
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Array[Float]) -> Float = {
    return simd_sum_float(arr.data, arr.length)
}

// 通用实现
sum: [T: Add](arr: Array[T]) -> T = {
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
sum: [P: X86_64](arr: Array[Float]) -> Float = {
    return avx2_sum(arr.data, arr.length)
}

sum: [P: AArch64](arr: Array[Float]) -> Float = {
    return neon_sum(arr.data, arr.length)
}
```

---

## 附录：类型定义速查

### A.1 类型定义

```
// === 记录类型（花括号） ===

// 结构体
Point: Type = { x: Float, y: Float }

// 枚举（变体类型）
Result: Type[T, E] = { ok(T) | err(E) }
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
List: Type[T] = { data: Array[T], length: Int }
Result: Type[T, E] = { ok(T) | err(E) }

// 泛型函数
map: [T, R](list: List[T], f: Fn(T) -> R) -> List[R] = { ... }

// 类型约束
clone: [T: Clone](value: T) -> T = value.clone()
combine: [T: Clone + Add](a: T, b: T) -> T = body

// 关联类型
Iterator: Type[T] = { Item: T, next: () -> Option[T] }

// 编译期泛型
factorial: [n: Int](n: n) -> Int = { ... }
StaticArray: Type[T, N: Int] = { data: T[N], length: N }

// 条件类型
If: Type[C: Bool, T, E] = match C { True => T, False => E }

// 函数特化
sum: (arr: Array[Int]) -> Int = { ... }
sum: (arr: Array[Float]) -> Float = { ... }
```

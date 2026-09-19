---
title: 'RFC-010: Unified Type Syntax - name: type = value Model'
status: 'Accepted'
author: 'Chenxu'
updated: '2026-07-14 (Never built-in type implemented, #157 closed)'
issue: '#127'
---

# RFC-010: Unified Type Syntax - name: type = value Model

## Summary

This RFC proposes a minimalist unified type syntax model: **everything is `name: type = value`**.

YaoXiang has only one declaration form:

```
identifier : type = expression
```

Where `type` can be any type expression, and `expression` can be any value expression. **There is no
`fn`, no `struct`, no `trait`, no `impl`, and no lowercase `type` keyword (but there is `Type` as
the meta type keyword)**.

> **Core design**: `Type` itself is a generic type. `(T: Type) -> Type` means "a type that accepts a
> type parameter T".

| Concept          | Code                                                                         |
| ---------------- | ---------------------------------------------------------------------------- |
| Variable         | `x: Int = 42`                                                                |
| Function         | `add: (a: Int, b: Int) -> Int = a + b`                                       |
| Record type      | `Point: Type = { x: Float, y: Float }`                                       |
| Interface        | `Drawable: Type = { draw: (Surface) -> Void }`                               |
| Generic type     | `List: (T: Type) -> Type = { data: Array(T), length: Int }`                  |
| Generic type     | `Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }`     |
| Method           | `Point.draw: (p: Point, s: Surface) -> Void = ...`<br>`Point.draw = draw[0]` |
| Generic function | `map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))`    |

**`Type` is the only meta type keyword in the language**.

> **Namespace vs method binding**: The `Type.name` prefix indicates **namespace affiliation**,
> nothing more. It does not trigger any implicit binding. For `.` call syntax like `p.draw(screen)`
> to work, you must explicitly bind: `Point.draw = draw[0]`. See the "Namespace and method binding"
> section below. It is used to mark the type level, and the compiler automatically handles the
> distinction of `Type0`, `Type1`, `Type2`..., which is transparent to the user.

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
    "Point(${self.x}, ${self.y})"
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

## Motivation

### Why is this feature needed?

The current type system has several separate concepts:

- Variable declaration syntax
- Function definition syntax
- Type definition syntax (different syntax)
- Interface definition syntax
- Method binding syntax

These concepts lack unity, resulting in fragmented syntax and a high learning cost.

### Design goals

1. **Ultimate unity**: one syntax rule covers all cases
2. **Concise and elegant**: the symmetric aesthetic of `name: type = value`
3. **No new keywords**: reuse existing syntax elements
4. **Theoretically elegant**: types themselves are values of type `Type`
5. **Generics-friendly**: seamless integration with the generics system (RFC-011)

### Integration with the generics system

The unified syntax model of RFC-010 and the generics system design of RFC-011 are **naturally
compatible**, and generics parameters can seamlessly fit into the unified model:

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

**Dependencies**:

- RFC-011 Phase 1 (basic generics) is a **strong dependency** of RFC-010
- Without basic generics, the generics examples in RFC-010 cannot compile
- Suggestion: implement RFC-011 Phase 1 in sync with RFC-010

## Proposal

### Core principle: type constructor vs function/variable

**This is a key design choice that determines the disambiguation rules of the syntax:**

| Syntax              | Meaning              | Rule                                                                   |
| ------------------- | -------------------- | ---------------------------------------------------------------------- |
| **`x: Type = ...`** | Type constructor     | Explicit `: Type` declaration → forced to be a type                    |
| **`f = ...`**       | Function or variable | No `: Type` → HM (Hindley–Milner) actively infers as function/variable |

**Why this design?**

The `{ ... }` syntax itself is ambiguous:

- `{ x: Float, y: Float }` can be a **type literal** (record type)
- `{ a = 1 + 1 }` can be a **code block** (executable statement, returning `Void`)

**Disambiguation rule**:

- **Has** `: Type` → forced to parse as a type constructor, `{ ... }` is a type literal
- **No** `: Type` → HM actively parses `{ ... }` as a code block, and infers it as a function type

```yaoxiang
# ✅ 类型构造器：有 : Type
Point: Type = { x: Float, y: Float }

# ✅ 函数：没有 : Type，HM 推断为 () -> Void
main: () -> Void = { println("Hello") }

# ❌ 错误：没有 : Type，编译器无法将 { ... } 解析为类型
Point = { x: Float, y: Float }  // HM 推断为函数，不是类型！
```

---

**Unified model: identifier : type = expression**

```
├── Variable
│   └── x: Int = 42
│
├── Function
│   └── add: (a: Int, b: Int) -> Int = a + b  # no : Type, HM infers as function
│
├── Record type
│   └── Point: Type = { x: Float, y: Float }  # must return: Type
│
├── Interface
│   └── Drawable: Type = { draw: (Surface) -> Void }  # must return: Type
│
├── Generic type
│   └── List: (T: Type) -> Type = { data: Array(T), length: Int }  # must return: Type
│
├── Generic type (multi-parameter)
│   └── Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }  # must return: Type
│
├── Namespace function
│   └── draw: (p: Point, surface: Surface) -> Void = ...
│       Point.draw = draw[0]  # only after explicit binding can the . call syntax be used
│
└── Generic function
    └── map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))  # does not return Type, HM infers as function
```

### Meta type hierarchy (compiler internal)

The compiler internally maintains a universe hierarchy `level: selfpointnum` (stored as a string,
theoretically infinitely extensible).

| Level    | Description                              |
| -------- | ---------------------------------------- |
| `Type0`  | Everyday types (`Int`, `Float`, `Point`) |
| `Type1`  | Type constructors (`List`, `Maybe`)      |
| `Type2+` | Higher-order constructors                |

**Users never see these numbers**, only `: Type`.

### Curry-Howard correspondence: types as propositions, programs as proofs

YaoXiang's unified syntax `name: type = value` is not chosen arbitrarily — it happens to be a direct
mapping of the Curry-Howard correspondence. This correspondence reveals a profound fact: **the type
system and the logic system are two sides of the same thing**.

| Logic (proposition)                | Type system (YaoXiang)              | Example                              |
| ---------------------------------- | ----------------------------------- | ------------------------------------ |
| Proposition P                      | Type T                              | `Int`, `Bool`                        |
| Proof that P is true               | A value of type T                   | `42: Int`, `true: Bool`              |
| P → Q (implication)                | Function type `(P) -> Q`            | `(x: Int) -> Bool`                   |
| P ∧ Q (conjunction)                | Record type `{ p: P, q: Q }`        | `{ x: Int, y: Bool }`                |
| ∀x.P(x) (universal quantification) | Generic function `(T: Type) -> ...` | `map: (T: Type, R: Type) -> ...`     |
| P ⊕ Q (disjunction)                | enum / tagged union                 | `Maybe: (T: Type) -> Type = { ... }` |

**Meaning of `name: type = value` under Curry-Howard**:

```yaoxiang
// "x: Int = 42" reads: "there exists a proof of type Int, named x, with value 42"
x: Int = 42

// "add: (a: Int, b: Int) -> Int = a + b" reads:
// "there exists an implication proof: given proofs a and b of type Int, we can construct a proof of type Int"
add: (a: Int, b: Int) -> Int = a + b

// "Point: Type = { x: Float, y: Float }" reads:
// "Point is a proposition whose proof requires simultaneously providing a proof x of type Float and a proof y of type Float"
Point: Type = { x: Float, y: Float }
```

**Why does this matter?**

1. **Logical consistency = type safety**: if the type system allows constructing a value of type `T`
   without any legal runtime representation, that is like allowing a false proposition to be proven
   in logic — the system collapses. Curry-Howard tells us: **a type-safe language is naturally a
   consistent logic system**.

2. **Universe hierarchy is a necessary condition**: as detailed below, if `Type: Type` were allowed
   (i.e., "the type of a type is also a type"), it would produce Russell's paradox (which in type
   theory manifests as Girard's paradox). YaoXiang's stratification `Type₀ : Type₁ : Type₂ : ...`
   ensures that each type belongs to only one level, forming an ever-rising chain that never closes,
   fundamentally avoiding paradoxes. This means that YaoXiang's type system is **logically
   consistent** in the Curry-Howard sense.

3. **Theoretical basis of the unified syntax**: the reason `name: type = value` can cover all
   concepts — variables, functions, types, interfaces, generics — with a single syntax is precisely
   that under Curry-Howard they are all the same thing: **providing a proof for a proposition**.
   Variables are evidence of propositions; functions are evidence of implications; records are
   evidence of conjunctions; generics are evidence of universal quantification. Unified syntax is
   not an artificial coincidence but a natural consequence of the Curry-Howard correspondence.

> **Further reading**: Wadler, P. (2015). _"Propositions as Types."_ Communications of the ACM,
> 58(12), 75–84. This article explains the history and significance of the Curry-Howard
> correspondence in accessible language.

### Syntax definition

#### 1. Variable declaration

```yaoxiang
// 基本语法
x: Int = 42
name: String = "Alice"
flag: Bool = true

// 类型推导（可省略）
y = 100  // 推断为 Int
```

#### 2. Function definition

> **Errata (2026-09-15, RFC-010a)**: This section and the "Return rules" below originally used
> `return` as the value exit of a block, which conflicts with the early-return semantics of RFC-007
> (see [RFC-010a](010a-tail-expression-and-return.md) for details). This has now been changed: **the
> value of a block = the tail expression, `return` exits the function (type `Never`)**. The
> erroneous example "you must use `return` to return a value" has been removed and is not preserved
> in this errata.

```yaoxiang
// 单表达式形式
add: (a: Int, b: Int) -> Int = a + b
greet: (name: String) -> String = "Hello, ${name}!"

// 代码块形式：值是尾表达式
process: (x: Int) -> Int = {
    a = x * 2
    b = a + 1
    b                    // 尾表达式 → 块的值
}

// 多行代码块
calc: (x: Float, y: Float, op: String) -> Float = {
    match op {
        "+" -> x + y,
        "-" -> x - y,
        _ -> 0.0
    }                    // match 作尾表达式
}

// Void 函数：尾表达式为 Void
print: (msg: String) -> Void = {
    console.write(msg)   // console.write : Void
}
```

#### Return rules

> **Errata (2026-09-15, RFC-010a)**: The original rule "`= { ... }` must use `return`, otherwise it
> returns `Void`" and the rationale "explicit `return` is needed to eliminate the ambiguity of
> 'whether the last expression is the return value'" **are deprecated**. The tail expression does
> not produce this ambiguity, and `return` does not interfere with it (Rust has validated the same
> design).

**The value of a block = the tail expression (the only exit)**:

| Form                                            | Value                                         |
| ----------------------------------------------- | --------------------------------------------- |
| `= expr` (no braces)                            | `expr`                                        |
| `= { ...; e }` (with braces)                    | the tail expression `e`                       |
| `= { ...; s }` (last is a statement/assignment) | `Void` (the value of an assignment is `Void`) |
| `= {}` (empty block)                            | `Void`                                        |

**Semantics of `return`**: non-local exit, **exits the nearest function boundary** (does not "return
to the block"), type is `Never`. `Never <: T` holds for any type (principle of explosion), so
`return` can appear at any return-type position.

```yaoxiang
# 单表达式：直接返回值
add: (a: Int, b: Int) -> Int = a + b

# 代码块：值是尾表达式
process: (x: Int) -> Int = {
    a = x * 2
    b = a + 1
    b
}

# 提前返回：return 穿透块，退出函数（类型 Never）
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    n * factorial(n - 1)     # 尾表达式
}
```

> **Design rationale (Errata 2026-09-15, RFC-010a)**: `{ ... }` is a dependency-driven computation
> unit (see below), and its evaluation semantics differ from a single expression — braces introduce
> a multi-statement context, **whose value is given by the tail expression**. The original argument
> "explicit `return` is needed to eliminate ambiguity" is deprecated: the tail expression does not
> produce that ambiguity.

#### `{}` semantics: a dependency-driven computation unit

`{ ... }` in YaoXiang is not just a code block — it is a **dependency-driven computation unit**.
This semantics is consistent in function bodies, variable initializers, and `spawn`:

**Core rules**:

- Assignment statements inside `{}` are automatically ordered by dependency, not by textual order
- When dependencies are satisfied, the statement executes immediately; otherwise it blocks waiting
- **The value of a block = the tail expression** (see Return rules); `return` is a non-local exit of
  type `Never`, exiting the function

```yaoxiang
# 依赖驱动：b 依赖 a，编译器自动排序
result: Int = {
    b = a + 1      # 依赖 a → 自动排在 a 之后
    a = 10         # 无依赖 → 可以先执行
    b              # 尾表达式 → 块的值 11
}
```

> **Difference from a single expression**: `= expr` (without braces) is a simple binding that
> directly returns the value; `= { ... }` (with braces) introduces a dependency-driven computation
> context that allows multiple statements, and its value is given by the tail expression.

#### `spawn` block

`spawn { ... }` is the only parallelism primitive in YaoXiang. It exploits the dependency-driven
semantics of `{}` to achieve automatic parallelization:

- Direct sub-assignments inside `spawn { ... }` automatically create parallel tasks
- Tasks whose dependencies are ready immediately run concurrently
- The caller blocks until all sub-tasks have completed

```yaoxiang
result = spawn {
    a = fetch_data("url1")    # 任务 1
    b = fetch_data("url2")    # 任务 2（与 a 无依赖，并行执行）
    c = process(a, b)         # 依赖 a, b → 等待两者完成后执行
    c                         # 尾表达式 → spawn 的值
}
// 调用方在此阻塞，直到 spawn 块内所有任务完成
```

> **Detailed definition**: for the complete semantics of `spawn`, task creation rules, and blocking
> model, see `008-runtime-concurrency-model.md`.

#### `unsafe` block

`unsafe { ... }` is used to define opaque types and to operate on raw pointers. It exploits the
evaluation semantics of `{}` so that the type definition is handed to the enclosing scope (the value
exit is the tail expression):

**Core rules**:

- Inside `unsafe {}` you can define types and operate on raw pointers
- The **tail expression** gives the value of `unsafe {}` (the type definition is handed to the
  enclosing scope)
- The returned type is available outside the `unsafe {}` block
- Accessing the fields of such a type requires unsafe permission

```yaoxiang
# 在 unsafe 块中定义不透明类型
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  # 裸指针
    }
    SqliteDb           # 尾表达式 → unsafe 块的值
}

# SqliteDb 在 unsafe 块外可用
db = sqlite3_open("test.db")

# ❌ 编译错误：handle 字段需要 unsafe 权限
handle = db.handle

# ✅ 通过方法调用
db.close()
```

> **Detailed definition**: for the complete semantics of `unsafe`, FFI type definition, and method
> binding, see `ffi.md`.

#### 3. Type definition

Type definition is the core of YaoXiang's unified syntax, covering fields, default values, bound
methods, and interface implementations:

##### Basic types

**Record type**: a list of fields whose field types may be any type expression.

```yaoxiang
Point: Type = {
    x: Float,
    y: Float
}
```

**Fields with default values**: a field can have a default value, which is optional at construction
time.

```yaoxiang
Point: Type = {
    x: Float = 0,
    y: Float = 0
}
```

Usage:

```yaoxiang
Point() → Point(x=0, y=0)
Point(x=1) → Point(x=1, y=0)
Point(x=1, y=2) → Point(x=1, y=2)
```

**Fields without default values**: must be provided at construction time.

```yaoxiang
Point2: Type = {
    x: Float,
    y: Float
}
```

Usage:

```yaoxiang
Point2(x=1, y=2) //✓
Point2() //✗
Point2(x=1) //✗
```

##### Built-in types

YaoXiang's identifier system is divided into three layers, recognized by different compiler phases:

1. **Keywords** (parser independent token) — control structures and declaration keywords such as
   `if`, `match`, `pub`, `return`
2. **Literal reserved words** (parser independent token) — `true`, `false`, `void`, `Type`; cannot
   be used as ordinary identifiers
3. **Built-in type names** (pre-registered by the type checker) — the parser treats them as ordinary
   identifiers, and the type checker resolves them. **Not reserved words; can be shadowed (not
   recommended)**

The difference between `void` (lowercase, a literal reserved word) and `Void` (uppercase, a built-in
type name): `void` is a value literal (equal to the unique value of Unit), whereas `Void` is a type
name (equal to the Unit type, logical ⊤). `let x: Void = void` is legal.

Predefined built-in type names:

| Type     | Logical counterpart    | Description                                                                                                                                                                                                                                                                          |
| -------- | ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `Never`  | ⊥ (false / empty type) | Zero constructor; no value can inhabit this type. Represents "impossibility" — divergence, panic, dead code. `Never <: T` holds for any `T` (principle of explosion). A function returning `Never` means it will never return normally. **Not a keyword, but a built-in type name.** |
| `Void`   | ⊤ (true / Unit)        | Exactly one inhabitant (the default `void` value). `x: Void = <default>` is legal. The unit of sum types corresponds to the unit of product types — `Void` is a zero-field product type (Unit), `Never` is a zero-variant sum type.                                                  |
| `Int`    | —                      | Signed integer                                                                                                                                                                                                                                                                       |
| `Float`  | —                      | Floating-point number                                                                                                                                                                                                                                                                |
| `Bool`   | —                      | Boolean value: `true` / `false`                                                                                                                                                                                                                                                      |
| `Char`   | —                      | Unicode character                                                                                                                                                                                                                                                                    |
| `String` | —                      | String                                                                                                                                                                                                                                                                               |

##### Bound methods

**Method 1: bind an external function directly within the type definition body**

```yaoxiang
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]           // 绑定到位置0，柯里化后 method: (b: Point) -> Float
}
// 调用：p1.distance(p2) → distance(p1, p2)
```

**Method 2: anonymous function + positional binding**

```yaoxiang
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance: ((a: Point, b: Point) -> Float)[0] = ((a, b) => {
        dx = a.x - b.x
        dy = a.y - b.y
        (dx * dx + dy * dy).sqrt()      # 尾表达式
    })
}
// 语法：((params) => body)[position]
// 调用：p1.distance(p2) → distance(p1, p2)
```

##### Interface implementation

**Interface names are written inside the type body; the compiler automatically checks the
implementation**

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

##### Interface definition

**An interface is a record type whose fields are all functions**

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

##### Namespace function definition

**The `Type.name` prefix indicates namespace affiliation**, nothing more. It triggers no implicit
binding.

```yaoxiang
// 命名空间函数：在 Point 命名空间下的普通函数
Point.draw: (p: &Point, surface: Surface) -> Void = {
    surface.plot(p.x, p.y)
}

Point.serialize: (p: &Point) -> String = {
    "Point(${p.x}, ${p.y})"
}

// 调用：就是普通函数调用
Point.draw(p, screen)
Point.serialize(p)
```

> **Note**: `self` is not a keyword, just a conventional name for the parameter. Writing it as `p`,
> `this`, or `x` is exactly equivalent. The compiler does not look at the parameter name, it looks
> at the type.

##### Method binding (the only way)

For `.` method-call syntax like `p.draw(screen)` to take effect, **explicit binding is required**.
The `[position]` syntax is the only mechanism for binding a function as a "method" (see RFC-004 for
the detailed syntax).

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

**Default behavior**: omitting `[n]` = do not bind any parameter. The user must explicitly decide
which parameters are filled by the caller.

**Multi-position binding**:

```yaoxiang
// 绑定多个位置（自动柯里化）
Point.transform = transform_points[0, 1]
// 调用：p1.transform(p2)(2.0) → transform_points(p1, p2, 2.0)
```

**Reverse operation** (method to ordinary function):

```yaoxiang
// 从绑定中取出函数
draw_point: (p: &Point, surface: Surface) -> Void = Point.draw
```

#### 4. Interface composition

```yaoxiang
// 接口组合 = 类型交集
DrawableSerializable: Type = Drawable & Serializable

// 使用交集类型
process: (T: Drawable & Serializable) -> ((item: T, screen: Surface) -> String) = {
    item.draw(screen)
    item.serialize()
}
```

#### 5. Generic types

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
    (self: List(type), item: type) -> {
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
    if index >= 0 and index < self.length {
        Maybe.Just(self.data[index])
    } else {
        Maybe.Nothing
    }
}
```

#### 6. Generic call syntax

Both generic types and generic function calls use the `()` syntax uniformly. `[]` is never used in
any generic context.

**Core rules**:

1. **`()` does everything for application**: type application, function calls, and value
   construction all use `()`

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

2. **Type on the left, value on the right**: `name: type = value` — Type parameters are declared on
   the left; the right side is always concrete values. The `T` of an empty container like `List()`
   must be obtained from the left-side type annotation.

3. **Type information only needs to be written once** — in the parameter declaration, the compiler
   carries it through:

```yaoxiang
numbers: List(Int) = List(1, 2, 3)  // Int 在左边写一次
f: (Int) -> String = (x) => x.to_string()
strings = map(numbers, f)   // T=Int, R=String 自动从 numbers 和 f 的类型来
```

4. **Value construction infers the type from the elements**:

```yaoxiang
x = List(1, 2, 3)       // 推断为 List(Int)
y = List("a", "b")      // 推断为 List(String)
z = List()              // ❌ 编译错误：无法推断 T
z: List(Int) = List()   // ✅ T=Int 来自左侧注解
```

5. **Type aliases**:

```yaoxiang
IntList: Type = List(Int)
StringToInt: Type = (String) -> Int
Matrix3x3: Type = Matrix(Float, 3, 3)
```

> **Comparison with the old syntax**: `List[Int]` → `List(Int)`, `List[Int]()` → `List()`,
> `List[Int](1,2,3)` → `List(1,2,3)`. The old `[]` generics syntax has been completely removed. `[]`
> is now used only for array/list literals and index access.

### Examples

#### Complete example

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
    Rect(p.x - 1, p.y - 1, 2, 2)
}

serialize: (p: &Point) -> String = {
    "Point(${p.x}, ${p.y})"
}

translate: (p: &Point, dx: Float, dy: Float) -> Point = {
    Point(p.x + dx, p.y + dy)
}

scale: (p: &Point, factor: Float) -> Point = {
    Point(p.x * factor, p.y * factor)
}

distance: (p1: &Point, p2: &Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
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
    "Rect(${r.x}, ${r.y}, ${r.width}, ${r.height})"
}
Rect.serialize = serialize[0]

translate: (r: &Rect, dx: Float, dy: Float) -> Rect = {
    Rect(r.x + dx, r.y + dy, r.width, r.height)
}
Rect.translate = translate[0]

scale: (r: &Rect, factor: Float) -> Rect = {
    Rect(r.x * factor, r.y * factor, r.width * factor, r.height * factor)
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

## Detailed design

### Interface check algorithm

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

### Interface direct assignment and compile-time optimization

Interface types support direct assignment; the compiler automatically picks the best call strategy
based on the right-hand side's type:

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

**Compile-time optimization strategy**:

| Scenario                         | Inferred result      | Call method                 |
| -------------------------------- | -------------------- | --------------------------- |
| `d: Drawable = Circle(1)`        | Concrete type Circle | Direct call (zero overhead) |
| `d: Drawable = get_shape()`      | Unknown              | vtable                      |
| `shapes: List(Drawable) = [...]` | Heterogeneous        | vtable                      |

**Rules**:

1. When the right-hand side is a concrete type constructor and is determinable at compile time,
   generate direct-call IR
2. When the right-hand side's type cannot be determined at compile time, fall back to the vtable
   mechanism
3. The vtable fallback guarantees correctness of runtime polymorphism

### Duck typing support

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

### Syntax changes

| Before                                   | After                                                                                        |
| ---------------------------------------- | -------------------------------------------------------------------------------------------- |
| `type Point = Point(x: Float, y: Float)` | `type Point = { x: Float, y: Float }`                                                        |
| `type Result(T, E) = ok(T) \| err(E)`    | `Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }` |
| `impl` keyword required                  | No keyword needed; interface names are written after the type body                           |

### Deprecated: `|` variant syntax

> **Deprecation announcement (2026-07-25)**: the `|` variant syntax is officially deprecated and has
> been removed from the implementation.

The following forms are **no longer supported**:

```
type Color = red | green | blue                # ❌ 废弃
type Result(T, E) = ok(T) | err(E)             # ❌ 废弃
type Option(T) = some(T) | none                # ❌ 废弃
```

Sum types are now expressed uniformly via record types. When all fields of a record type are
functions that all return the type itself, it is a sum type:

```yaoxiang
Color: Type = {
    red: () -> Color,
    green: () -> Color,
    blue: () -> Color
}

Result: (T: Type, E: Type) -> Type = {
    ok: (T) -> Result(T, E),
    err: (E) -> Result(T, E)
}

Option: (T: Type) -> Type = {
    some: (T) -> Option(T),
    none: () -> Option(T)
}
```

**Design rationale**:

1. **Eliminate special cases**: `|` is the only non-`name: type = value` form of syntax in the BNF.
   After removing it, the `type_expr` production is completely unified, and the parser no longer
   needs to maintain a separate path and lookahead fallback for variant types.
2. **Mathematical equivalence**: under the Curry-Howard correspondence, the sum type corresponding
   to the disjunction P ⊕ Q is equivalent to a "record type whose fields are all functions returning
   the type itself". They express the same semantics, and two syntaxes are unnecessary.
3. **Zero destruction**: before removal, the `|` syntax was only half-supported in the parser
   (parameterless variants could be parsed, but parameter types were lost during monomorphization),
   and no user code depended on it.
4. **AST simplification**: the `Type::Variant(Vec<VariantDef>)` node is removed; all variant types
   now uniformly take the `Type::Struct` path, and the special branches in downstream
   typecheck/mono/formatter are all eliminated.

> **Note**: the semantic properties of sum types (such as match exhaustiveness checking and
> tagged-union memory layout) are inferred by the typecheck layer from the `Type::Struct` structure,
> and do not depend on a dedicated AST node.

### Logical operators: `and` / `or` / `!` (authoritative definition, Zig-style)

> **Definition announcement (2026-08-03)**: the authoritative form of the logical operators is the
> keywords `and` / `or` plus the symbolic unary `!` (consistent with the precedence table in SPEC
> `syntax.md` §2.2). This design aligns with Zig: **short-circuit control flow uses keywords; pure
> unary operations use symbols**. The early implementation drifting toward C's `&&` / `||`, and the
> intermediate keyword `not`, have both been removed.

**Semantics**:

| Operator | Precedence (SPEC §2.2)            | Associativity | Semantics                                    |
| -------- | --------------------------------- | ------------- | -------------------------------------------- |
| `!`      | 3 (unary prefix, tightly binding) | right-to-left | Logical NOT (pure function, no control flow) |
| `and`    | 10                                | left-to-right | Short-circuit logical AND                    |
| `or`     | 10                                | left-to-right | Short-circuit logical OR                     |

```yaoxiang
# 短路求值：and 左侧为 false / or 左侧为 true 时，右侧不执行
if x != 0 and y / x > 1 { ... }   # x == 0 时不会除零

# 紧绑定：!a == b ≡ (!a) == b（Zig 式；与 Python 的 not a == b ≡ not (a == b) 相反）
!3 == 4          # false：(!3) == 4 → false
!(3 == 4)        # true
!x != 0          # ≡ (!x) != 0
!list.is_empty(xs)   # ≡ !(list.is_empty(xs))，调用后取反
```

The following forms are **no longer supported** (the lexer reports an error and suggests the
corresponding form):

```
x && y     # ❌ 已移除，用 x and y
x || y     # ❌ 已移除，用 x or y
not x      # ❌ 已移除，用 !x（not 恢复为普通标识符；!= 不受影响）
```

**Design rationale** (aligning with Zig, ziglang/zig#272 / #6625):

1. **Short-circuit is control flow → keywords; pure function is operation → symbols**. `and` / `or`
   change the order of evaluation (the right side is skipped when not needed), the same nature as
   `if`, so keywords; `!` performs pure negation on an already-evaluated operand, the same nature as
   `-` `+`, so symbols. YaoXiang uses `?` for error propagation (§2.11), so `!` has no conflict.
2. **Tight binding eliminates ambiguity**: `!` visually "sticks" to its operand, and the high
   precedence is immediately obvious; the keyword `not` is forced to leave a space from its operand,
   and where it binds (`not a == b`) easily causes mental ambiguity.
3. **Disambiguation**: `&` already serves two roles — the borrow token (`&p` / `&mut p`, RFC-009)
   and bitwise AND (SPEC §2.2 precedence 8). Introducing `&&` would make a single symbol carry three
   meanings. `and` / `or` / `!` visually separate the three concepts of borrow, bitwise operation,
   and logic completely.
4. **Precedent**: Zig (a modern system language in the same ecological niche) is exactly the
   combination of `and` / `or` keywords plus the `!` symbol; Python / Lua / Ada / SQL use
   all-keyword (including `not`); the C family uses all-symbol — YaoXiang takes Zig's mix, getting
   the best of both.
5. **Curry-Howard consistency**: types are propositions (see the isomorphism section above); the
   logical connectives in refinement types are written as `and` / `or` (e.g.
   `{ 0 <= idx and idx < arr.len }`), which is a natural expression of propositions; `!` as a unary
   negation symbol corresponds to ¬.

> **Implementation**: in the IR layer, `and` / `or` expand to short-circuit jump sequences
> (`a and b ≡ if a { b } else { false }`); `!` is parsed as a unary tight binding (the operand is
> parsed at `BP_UNARY + 1`). Regression tests: `tests/yaoxiang/01-syntax/basics/logical_ops.yx`,
> `logical_not.yx`.

## Syntax design notes: named functions are essentially syntactic sugar for Lambdas

### Core insight

**Named functions and Lambda expressions are the same thing!** The only difference is that a named
function gives the Lambda a name.

```yaoxiang
// 这两者本质完全相同
add: (a: Int, b: Int) -> Int = a + b           // 具名函数（推荐）
add: (a: Int, b: Int) -> Int = (a, b) => a + b        // Lambda 形式（完全等价）
```

### Syntactic sugar model

```
// 具名函数 = Lambda + 名字
name: (Params) -> ReturnType = body

// 本质上是
name: (Params) -> ReturnType = (params) => body
```

**Key point**: when the signature fully declares the parameter types, the parameter names in the
Lambda header become redundant and can be omitted.

### Parameter scope rules

**Parameters override outer-scope variables**: the parameters in the signature shadow the function
body, and the inner scope has higher priority.

```yaoxiang
x = 10  // 外层变量

double: (x: Int) -> Int = x * 2  // ✅ 参数 x 覆盖外层 x，结果为 20
```

### Flexible annotation position

The type annotation may appear at any of the following positions; **at least one annotation is
sufficient**:

| Annotation position | Form                                     | Note                     |
| ------------------- | ---------------------------------------- | ------------------------ |
| Signature only      | `double: (x: Int) -> Int = x * 2`        | ✅ Recommended           |
| Lambda header only  | `double = (x: Int) => x * 2`             | ✅ Valid                 |
| Both sides          | `double: (x: Int) -> Int = (x) => x * 2` | ✅ Redundant but allowed |

### Complete example

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

### Design advantages

| Feature        | Advantage                                                                             |
| -------------- | ------------------------------------------------------------------------------------- |
| **Concise**    | No need to repeat parameter names when the signature is complete                      |
| **Flexible**   | The Lambda form is preserved; use whichever you like                                  |
| **Consistent** | Maintains the unified pattern with variable declaration `x: Int = 42`                 |
| **Intuitive**  | `name: Type = body` directly corresponds to "named `name`, type `Type`, value `body`" |

## Trade-offs

### Advantages

| Advantage             | Description                                            |
| --------------------- | ------------------------------------------------------ |
| Ultimate unity        | One syntax rule covers all cases                       |
| Theoretically elegant | Perfectly symmetric `name: type = value`               |
| No new keywords       | Reuse existing syntax elements                         |
| Easy to implement     | The compiler only needs to handle one declaration form |
| Easy to learn         | Remember one pattern and you can write all code        |
| Easy to extend        | New features can naturally fit into this model         |

### Disadvantages

| Disadvantage      | Description                                                           |
| ----------------- | --------------------------------------------------------------------- |
| Naming convention | Methods need to follow the `Type.method` naming convention            |
| Verbose           | The full syntax is longer than a simplified form, but can be inferred |
| Learning curve    | You need to understand the unified model                              |

### Mitigations

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

### Risks

| Risk                 | Impact                                             | Mitigation                                 |
| -------------------- | -------------------------------------------------- | ------------------------------------------ |
| Parsing complexity   | The unified syntax may increase parsing complexity | Use a recursive-descent parser             |
| Performance overhead | vtable lookup may incur extra overhead             | Compile-time monomorphization optimization |

---

## Easter egg 🎮: the source of the language

> ✨ **Type: Type = Type** ✨

```yaoxiang
// 尝试定义类型的类型...
Type: Type = Type
```

**Warning**: this is an **unspeakable** thing!

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

> **Note**: the compiler cannot correctly handle `Type: Type = Type` (it would cause the Type0/Type1
> universe paradox), but we deliberately keep this "easter egg" — when you try to compile it, you
> will receive a Zen message from the language's founder. This is not just a technical boundary, but
> also YaoXiang's tribute to the philosophy of types.

---

## Appendix

### Syntax BNF

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

### Glossary

| Term               | Definition                                                                                                                     |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------ |
| Declaration        | An assignment statement in the form `name: type = value`                                                                       |
| Record type        | A `{ ... }` type that contains named fields                                                                                    |
| Interface          | A record type whose fields are all function types                                                                              |
| Generic type       | A type defined as `Name: (T: Type) -> Type = { ... }`, accepting type parameters                                               |
| Namespace function | A function of the form `Type.name`, belonging to the `Type` namespace. Implies no binding                                      |
| Method binding     | `Type.name = func[n]`, binding parameter at position `n` of `func` as the caller, making the `obj.name(args)` syntax available |
| Generic function   | A function using the `(T: Type)` syntax, with type parameters as the first parameter group                                     |
| Meta type          | `Type`, the only type-level marker in the language                                                                             |

---

## Lifecycle and destination

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

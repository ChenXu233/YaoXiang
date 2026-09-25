---
title: 'RFC-011b: 运算符重载与接口驱动运算符'
status: '已接受'
author: '晨煦'
created: '2026-09-15'
updated: '2026-09-22'
group: 'rfc-011'
issue: '#341'
---

# RFC-011b: 运算符重载与接口驱动运算符

> **参考**:
>
> - [RFC-011: 泛型系统设计](./011-generic-type-system.md) — 类型约束
>   `T: Add + Multiply`、关联类型
> - [RFC-011a: 接口实现与动态分发](./011a-interface-implementation.md)
>   — 接口声明/实例化机制
> - [RFC-009: 所有权模型设计](./009-ownership-model.md) — `&mut T` 线性令牌
> - [RFC-004: 柯里化方法的多位置联合绑定](./004-curry-multi-position-binding.md) — `f[0]`
>   位置绑定语法
> - [RFC-010: 统一类型语法](./010-unified-type-syntax.md)
>   — 和类型 = 字段全返回自身类型的记录
> - [RFC-013: 错误码规范](./013-error-code-specification.md) — `Result`
>   归 std 的既有定位、E108x
> - [RFC-010b: 模式匹配完备化（变体解构与穷尽性）](../draft/010b-pattern-matching-completeness.md) — 变体解构（依赖）

## 摘要

为 YaoXiang 补齐**运算符重载**能力，使 `a + b` / `a == b` / `a[i]` / `e?`
可由用户自定义类型实现，并让 RFC-011 已写入的约束语法中的**运算符约束**（`T: Add + Multiply`）从
**纸面能力**变为可落地机制（同句中的 `Zero` 不属运算符，见开放问题）。

设计采用**三层职责分离**：固定的运算符→方法映射表（Layer 0）、按名派发的基座（Layer 1，复用既有
`method_bindings`）、接口契约层（Layer
2，用于泛型约束）。运算符的**优先级与结合性保持语言固定**，用户只重载语义。

首批范围：`Add` `Subtract` `Multiply` `Divide` `Modulo` `Equal` `Index`
七个接口。算术接口采用**三类型参数** `(Self, R, O)`——结果类型 `O` 显式化，使 `1 + 2.5`（返回
`Float`）与 `point * 2.0`（缩放）这类异型运算可表达。`Equal`
**默认自动派生**（全字段可比的记录自动获得逐字段 `==`），显式实例化可覆盖。

`Try`（`?`
的接口化）整体移至阶段 2：它依赖 RFC-010（构造）与 RFC-010b（解构）落地，且接口形状尚未定案（见开放问题）。

**无新语法、无新关键字**，全部复用 RFC-011a 既有机制（接口声明 / 实例化 / 外部方法声明 / 重载）。

## 动机

### 为什么需要这个特性

#### 1. RFC-011 的核心示例依赖它，且现状比「无法兑现」更糟

RFC-011（已接受）在 8 处使用运算符名作为类型约束：

```yaoxiang
multiply: (T: Add + Multiply + Zero, Rows: Int, Cols: Int, M: Int) -> (
    (a: Matrix(T, Rows, Cols), b: Matrix(T, Cols, M)) -> Matrix(T, Rows, M)
)
```

全套 RFC 中从无一篇定义 `Add` / `Multiply` 从何而来、`+` 如何绑定到它们。实测确认现状更糟：`T: Add`
**今天直接编译报错**——约束求解查询旧 trait 表（`trait_data.rs`），表里没有 `Add`。约束名 `Zero` /
`One` / `PartialOrd` / `Fn` / `FnMut` 同样悬空（本 RFC 的处理见「与其他 RFC 的协同」）。

#### 2. 用户自定义类型无法参与基本运算（实测）

```
Point: Type = { x: Int, y: Int }
a = Point(1, 2)
b = Point(1, 2)
a == b
```

```
error [E6007] Runtime error: type mismatch in comparison Eq:
    Struct { type_id: TypeId(0), ... } vs Struct { type_id: TypeId(0), ... }
```

连带后果：`list.contains(list_of_structs, p)` **完全不可用**——它内部依赖 `==`。而 `(1, 2) == (1, 2)`
与 `[1] == [1]` 走运行时逐元素比较**一直可用**——结构体是唯一缺口。

#### 3. `?` 把类型名焊死在编译器里，阻断 `Result` 归 std

`?` 的实现同时硬编码了类型名与变体编号：

```rust
// typecheck：硬编码构造 Result 类型
let expected_result = MonoType::make_result(ok_ty, expected_err);

// ir_gen：硬编码 group 名与 variant 编号
Instruction::VariantTag { group: "Result".to_string(), .. }
variant 0 = ok, variant 1 = err
```

这使 `Result` 必须留在 core。若 `?` 改为**接口驱动**，任何实现了该接口的类型（含用户自定义）都能被
`?` 使用，`Result` 即可归属 std（RFC-013 已有「std 库 `Result(T, Error)`」的定位）。

注意这只是问题的一半：`ok(...)` / `err(...)` / `some(...)`
构造写法目前也焊在解析器里（语言规范§1.4.2 列为「由解析器识别的构造子」）。「Result 归 std」需要把
`?` 与构造子**两半一起解**，均归本 RFC 阶段 2。

#### 4. 用户自定义容器无法索引

`Index` 的类型检查是硬编码白名单：

```rust
// expressions.rs
MonoType::Generic { name, args } if name == "List"  => Ok(args[0].clone()),
MonoType::Generic { name, args } if name == "Array" => Ok(args[0].clone()),
MonoType::Generic { name, args } if name == "Dict"  => Ok(args[1].clone()),
// 其余 → "类型层不认的容器宁拒不静默"
```

用户定义任何容器类型，`c[0]` 一律报错。

#### 5. `%` 的实现违反已发布文档

实测 `-7 % 3` 返回
`-1`（截断余数）。但语言参考的运算符优先级表（`reference/index.md`）早已写着「`* / %`
乘除**取模**」——文档承诺的是取模，实现给的是余数。这不是设计变更，是**实现违反文档的缺陷**，本 RFC 顺手修正。

### 已有的半成品基础

调研发现机制**大部分已在**，缺的是接线：

| 机制                                      | 位置                                                                                              | 状态                                            |
| ----------------------------------------- | ------------------------------------------------------------------------------------------------- | ----------------------------------------------- |
| 接口声明 `(Self: Type) -> Type`           | RFC-011a Phase 1                                                                                  | ✅ 可运行                                       |
| 接口实例化 `Animal(Dog)` + 外部方法声明   | RFC-011a Phase 2–3                                                                                | ✅ 可运行（有单测）                             |
| 方法派发 `method_bindings["Type.method"]` | `expressions.rs`（注册于 `environment.rs`，查询于 `expressions.rs` 调用解析与字段 fallback 路径） | ✅ 可运行                                       |
| 关联类型（= 接口类型参数）                | RFC-011 §3.1；RFC-011a 已采纳此方案                                                               | ✅ 机制已定                                     |
| 类型族求值 `AssociatedTypeDef`            | `dependent_types.rs`                                                                              | ✅ 生产用例 `std.assert` 的 `IsTrue`            |
| `Equal` / `Dup` / `Clone` / `Debug` trait | `trait_data.rs` 已注册                                                                            | ⚠️ `Equal` 零消费点；旧自动派生只登记签名无实现 |
| 运算符 → 方法名映射                       | —                                                                                                 | ❌ 不存在                                       |

**结论**：这不是从零造特性，而是把已有零件接线并成文。

## 提案

### 核心设计：三层职责分离

```
Layer 0  固定映射表（语言级常量，用户不可改）
         + → add    == → equal    [] → index    ? → residual
         │  编译期内置，不参与类型推导，不暴露给用户
         ▼
Layer 1  派发基座（按名查方法并调用）        ← 复用既有机制
         method_bindings["Point.add"]
         ▼
Layer 2  接口契约（泛型约束用）
         Add / Equal / Index / Try …
         使 T: Add 约束成立，且作为运算符的放行条件
```

**分层理由**：

- **Layer 0 与 1 让运算符「能用」**，Layer 2 让运算符「能被约束」。合并会导致任何名为 `add`
  的方法都被 `+` 调用，RFC-011 的 `T: Add` 与运算符脱节。
- **Layer 1 不新造**：`method_bindings` 已在按 `Type.method`
  键查表（字段查找失败后的 fallback 路径），运算符走同一条路即可。
- **Layer
  2 是放行条件**：运算符放行前**必须**确认类型实现了对应接口（见 §Equal 的自动派生例外——推导与登记是同一条判据）。

### 名字与登记：两条独立通道

**运算符查询的是「接口实现登记表」，不经过普通名字解析。**

- 登记表归核心所有，聚合两处登记：核心为基本类型做的默认登记（`Add(Int, Int, Int)`
  等），和用户在类型体内写的接口实例化（`Add(Point, Point, Point)`）。无论实现代码物理上在哪个模块，登记汇入同一张表，`+`
  / `==` / `[]` 只看这张表。
- 本地模块定义一个叫 `Add` 的**同名绑定**（例如 RFC-011 §5.2 的 Peano 类型级加法
  `Add: (A: Type, B: Type) -> Type = match ...`）走的是**名字查找通道**，只影响该模块内 `Add`
  这个名字的解析，**碰不到登记表，不影响运算符可用性**。同名不同物、互不遮蔽。

这同时回答了「RFC-011 的类型级 `Add` 与本 RFC 的接口 `Add` 是否冲突」：不冲突。类型级 `Add`
是纯类型层面的计算（Zero/Succ 是类型不是值，永远不会有 `Zero + Succ(...)`
的值运算），与值层面的运算符接口是同一名字的两个层面。

### Layer 0：固定映射表

| 运算符            | 接口                          | 方法               | 首批      |
| ----------------- | ----------------------------- | ------------------ | --------- |
| `+`               | `Add`                         | `add`              | ✅        |
| `-`               | `Subtract`                    | `subtract`         | ✅        |
| `*`               | `Multiply`                    | `multiply`         | ✅        |
| `/`               | `Divide`                      | `divide`           | ✅        |
| `%`               | `Modulo`                      | `modulo`           | ✅        |
| `==` `!=`         | `Equal`                       | `equal`            | ✅        |
| `[]`              | `Index`                       | `index`            | ✅        |
| `?`               | `Try`（四方法，阶段 2 定案）  | `is_failure` 等    | ✅ 已落地 |
| `<` `<=` `>` `>=` | —（保留原生指令）             | —                  | ❌        |
| `and` `or`        | —（短路是语言语义，不可重载） | —                  | ❌        |
| 位运算 5 个       | —                             | —                  | ❌        |
| 一元 `-` `!`      | —                             | —                  | ❌        |

**接口名用全拼而非缩写**（`Multiply` 而非 `Mul`）：与 RFC-011 正文的 `T: Add + Multiply + Zero`
保持一致，**不改动已接受 RFC**。

**`%` 采用 `Modulo`
语义**（数学取模，结果符号跟随除数）。动机一节已确认当前 remainder 实现违反已发布文档（`reference/index.md`「乘除取模」），本条按缺陷修正处理，不留兼容期。

现状备注：`%` 的类型检查白名单目前只有 Int/Float（与 `+`
的 Int/Float/String/List 白名单不同），附录 A.2 有实测记录。

### Layer 2：接口定义

#### 算术接口（三类型参数）

```yaoxiang
Add: (Self: Type, R: Type, O: Type) -> Type = {
    add: (self: &Self, other: &R) -> O
}

Subtract: (Self: Type, R: Type, O: Type) -> Type = {
    subtract: (self: &Self, other: &R) -> O
}

Multiply: (Self: Type, R: Type, O: Type) -> Type = {
    multiply: (self: &Self, other: &R) -> O
}

Divide: (Self: Type, R: Type, O: Type) -> Type = {
    divide: (self: &Self, other: &R) -> O
}

Modulo: (Self: Type, R: Type, O: Type) -> Type = {
    modulo: (self: &Self, other: &R) -> O
}
```

三个类型参数各司其职：

- `Self`：左操作数类型（接收者，`&Self` 借用，见 RFC-009 借用令牌与 RFC-011a 接收者约定）；
- `R`：右操作数类型——**左右允许异型**；
- `O`：**结果类型**，显式声明。

**为什么结果类型必须是显式参数**（而不是写死 `-> Self`）：

1. 写死 `Self` 则 `Add(Int, Float)` 的方法必须返回 `Int`，`1 + 2.5` 无法正确表达；
2. 结果类型一旦显式化，RFC-011 §8.3 的提升类型族
   `Add: (A, B) -> Type = match (A, B) { (Int, Float) => Float, ... }`
   与本 RFC 的接口登记**成为同一张表的两个视角**——核心登记 `Add(Int, Float, Float)` 就是
   `(Int, Float) => Float` 那一行，用户的每次实例化都是往这张表里加行；
3. `Index` 接口本来就是三参数 `(Self, Key, Value)`、返回类型 `Value` 作参数——算术接口补上 `O`
   之后，整个运算符接口家族形状统一，写死 `Self` 的反而是异类。

**核心默认登记**（原生指令路径，不走方法调用）：`Add(Int, Int, Int)`、`Add(Int, Float, Float)`、
`Add(Float, Int, Float)`、`Add(Float, Float, Float)`、`Add(String, String, String)`（拼接）、
`Add(List(T), List(T), List(T))`（元素拼接）等，五个算术接口对基本类型同理。`1 + 2.5`
从当前的编译错误（白名单要求两侧同型）变为返回 `3.5: Float` 的合法运算。

**约束语法糖**：`T: Add` ≜ 已登记 `Add(T, T, T)`——同型自复合，结果仍为该型。RFC-011矩阵乘法示例中
`a * b + c` 全程类型为 `T`，要求的正是这个含义。`T: Equal` 同理 ≜ `Equal(T, T)`。

**异型示例**——向量缩放：

```yaoxiang
Point: Type = {
    x: Float,
    y: Float,
    Multiply(Point, Float, Point),
}

Point.multiply: (self: &Point, other: &Float) -> Point =
    Point(self.x * other, self.y * other)

main: () -> Void = {
    p = Point(1.0, 2.0)
    q = p * 3.0              # Point(3.0, 6.0)
}
```

#### 相等接口（默认自动派生）

```yaoxiang
Equal: (Self: Type, R: Type) -> Type = {
    equal: (self: &Self, other: &R) -> Bool
}
```

**`Equal` 默认自动派生**，规则五条：

1. **默认派生**：定义记录类型时，若所有字段都可比（基础类型、`String`、或本身可比的记录/元组/列表，且不含
   `&mut` 字段），编译器自动生成**逐字段比较**的
   `==`，走原生代码路径，不生成用户可见的方法。用户自己写的类型天然可比，无需任何仪式。
2. **显式覆盖**：类型体里写了 `Equal(Point, Point)` 并提供 `Point.equal`
   方法的，用用户的版本（如带容差的浮点比较），不再自动派生。
3. **字段不可比时**：自动派生失败，`==` 不可用，诊断指明是哪个字段拖了后腿；此时用户仍可手写 `Equal`
   自定义比较（如含函数字段就按名字比）。
4. **前置条件**：含 `&mut T`
   线性令牌（字段递归）的类型不参与比较——线性令牌读一次即消耗，无法同时取出两个值来比。注意前提是「非线性」而非「Dup」：原语值类型（Int/Float/Bool/Char）按 RFC-011
   §2.4 不属于 Dup（它们是编译器内置值复制），若以 Dup 为前提则 `Point { x: Float, y: Float }`
   会被误拒。
5. **约束同判据**：`T: Equal` 的求解与 `==`
   的放行走同一条「查登记或结构推导」规则——约束和运算符永远给出同一个答案。

**一致性依据**：`(1, 2) == (1, 2)` 与 `[1] == [1]` 今天就走运行时逐元素比较（`executor.rs`
的比较白名单含 Tuple/List/Array），记录类型是唯一被排除的复合类型。自动派生不是新增静默默认，而是把语言内部既有行为补齐到最后一块。

**旧机制退役**：`trait_data.rs` 中 `Equal`
的旧自动派生（只登记签名、无实现代码、零消费点）停用；`Equal`
的全部判定（基本类型默认登记、结构推导、显式实例化）走接口登记表。旧 trait 表保留 Clone/Dup/Debug 的既有职责（名字与运算符接口不重叠，将来统一另议）。

#### 索引接口

```yaoxiang
Index: (Self: Type, Key: Type, Value: Type) -> Type = {
    index: (self: &Self, key: &Key) -> Value
}
```

**`Value`
作为类型参数而非关联类型成员**：RFC-011a 的开放问题已定案「关联类型通过泛型接口参数实现」（`Iterator: (Item: Type) -> Type`
是同构先例），不需要引入 `type` 成员语法。

**所有权说明**：`index` 从 `&Self` 借用中返回完整 `Value`。标准库的 `list.get: (&Vec(A), Int) -> A`
已是同一形态，本接口**与 std 现状同待遇**；对移动语义元素类型「从借用中取出完整值」的精确语义，归 RFC-009 全面执法时统一处理，本 RFC 不为此发明新规则。

**多位置索引靠元组打包 + 重载**，不引入变参接口：

```yaoxiang
// 一维容器
List(T) 实例化 Index(List(T), Int, T)                   → arr[0]

// 多维容器
Grid    实例化 Index(Grid, Tuple(Int, Int), Float)       → g[0, 1]
//                    └─ Key 是元组

// 两个实例化签名不同 → 共存
```

**同一类型的同名接口允许多个实例化**，按其注入方法的签名区分、遵循 RFC-011a 方法级重载规则共存。RFC-011a 的重载明文只到方法级，本条在其上补一层实例化级规则：同名接口实例化共存的合法性由展开后方法签名是否冲突决定（冲突即 E1097，与字段/方法命名空间规则同源）。

#### 传播接口 Try（阶段 2 已定案并落地）

`?` 的接口化接口名为 `Try`，四方法形状（2026-09-22 阶段 2 定案）：

```yaoxiang
Try: (Self: Type, T: Type, E: Type) -> Type = {
    is_failure: (self: &Self) -> Bool,
    success:    (self: &Self) -> T,
    residual:   (self: &Self) -> E,
    from_error: (E) -> Self,
}
```

- **语义分工**：`is_failure` 判定成败、`success` 取成功载荷、`residual` 取失败
  载荷、`from_error` 做跨类型传播的桥（`f()?` 的 `T` ≠ 外层 `U` 时从 `E` 重建
  失败值）。`?` 的 lowering 统一生成四方法调用链——`is_failure(t)` 为真则
  `Ret from_error(residual(t))`，否则表达式值为 `success(t)`；不再手写变体
  检查序列，`Result` / `Option`（std yx 实现）与用户自定义 Try 类型同一路径。
- **死路分支**：`success` 的失败臂与 `residual` 的成功臂契约上不可达，实现用
  `assert(false)` 发散（`assert` 返回 `Never`，爆炸原理 `Never <: T` 放行，
  type-system.md §2.2）。
- **检查**：typecheck 查接口实现登记表（Self 位名义匹配，抽象条目按 scrutinee
  实参实例化）；外层函数返回类型必须也实现 `Try` 且 `E` 位可接住失败值
  （E1081/E1082/E1083 语义随之接口化）。
- **Result 归 std**：`Result` / `Option` 的类型定义与 Try 实现迁入
  `std/result.yx` / `std/option.yx`（纯 YaoXiang），native `ok`/`err` 构造器
  退役——变体构造语法 `Result(T, E).ok(v)` 是唯一构造通道（构造子焊死问题
  随 parser 特判退役一并解决）。Option 的 Try 残余类型取 `Void`（对应 Rust
  Try 实验的 NoneT 语义）。

### 示例

#### 用户自定义类型：算术与相等开箱即用

```yaoxiang
Point: Type = {
    x: Float,
    y: Float,
    Add(Point, Point, Point),
}

Point.add: (self: &Point, other: &Point) -> Point =
    Point(self.x + other.x, self.y + other.y)

main: () -> Void = {
    a = Point(1.0, 2.0)
    b = Point(3.0, 4.0)
    c = a + b                   # Point(4.0, 6.0)
    println(a == b)             # false —— Equal 自动派生，无需实例化
    println(a == a)             # true
}
```

#### 自定义相等（覆盖自动派生）

```yaoxiang
Vec3: Type = {
    x: Float,
    y: Float,
    z: Float,
    Equal(Vec3, Vec3),
}

# 浮点比较带容差，覆盖逐字段自动派生
Vec3.equal: (self: &Vec3, other: &Vec3) -> Bool =
    abs(self.x - other.x) < 0.000001
    and abs(self.y - other.y) < 0.000001
    and abs(self.z - other.z) < 0.000001
```

#### 自定义容器索引

```yaoxiang
Box: (T: Type) -> Type = {
    data: List(T),
    Index(Box(T), Int, T)
}

Box.index: (T: Type)(self: &Box(T), key: &Int) -> T =
    list.get(self.data, key)

main: () -> Void = {
    b = Box([10, 20, 30])
    println(b[1])               // 20
}
```

#### 泛型约束终于可兑现

```yaoxiang
// RFC-011 的示例从此可落地
// T: Add ≜ Add(T, T, T) 已登记；T: Multiply ≜ Multiply(T, T, T) 已登记
combine: (T: Add + Multiply)(a: T, b: T, c: T) -> T =
    a * b + c
```

（RFC-011 招牌示例 `T: Add + Multiply + Zero` 中的 `Zero` / `One`
不在本 RFC 范围——它们是常量成员不是运算符，「无接收者的接口成员」在 RFC-011a 无先例，见开放问题。）

### 语法变化

**无新语法、无新关键字**。所有能力由既有机制组合而成：

| 能力         | 复用的既有机制                             |
| ------------ | ------------------------------------------ |
| 接口声明     | RFC-011a Phase 1                           |
| 接口实例化   | RFC-011a Phase 2（`Dog: { Animal(Dog) }`） |
| 方法实现     | RFC-011a Phase 3（外部声明 `Point.add`）   |
| 关联类型     | RFC-011 §3.1（接口类型参数）               |
| 多位置索引   | 既有元组打包解析 + 实例化级重载规则        |
| 运算符优先级 | **语言固定**，不开放用户自定义             |

## 详细设计

### 类型系统影响

**新增接口**（Layer 2）：`Add` `Subtract` `Multiply` `Divide` `Modulo` `Equal` `Index`（`Try`
阶段 2）。

**接口实现登记表**：运算符的判定中枢。聚合核心默认登记（基本类型）与用户实例化（`check_interface_instantiation`
产出的 `ImplementationProof`），`+` / `==` / `[]` 的放行检查与 `T: Add` 约束求解（`bounds.rs` 的
`check_trait_bounds`）**查询同一张表**——不存在「约束说行、运算符说不行」的缝隙。

**`Equal`
的结构推导**：登记表之上叠加一条结构规则（全字段可比 ⇒ 可比），基本类型由核心登记兜底，递归闭合。原
`trait_data.rs` 的 `Equal` 注册与旧自动派生停用。

**运算符查询与名字解析分离**：`+` 等运算符只查登记表，不走普通名字解析；本地同名绑定（如类型级 `Add`
家族）不影响运算符（见 §名字与登记）。

**孤儿规则**：运算符实现只能写在**类型的定义模块**里——`Int` 的定义在核心，故 `Int`
的登记只能核心写；`Point`
的定义在用户模块，故只有其定义者能给它登记。所有类型服从同一条规则，内建类型无特权。

### 运行时行为

**零运行时开销**：运算符在编译期确定调用目标（静态派发）。对基本类型（`Int`/`Float`/`String`/`List`）保留**原生指令快路径**，不走接口派发；自动派生的结构体
`==` 生成原生逐字段比较；`Any`（动态类型）位置的 `==` / `!=` 保持现有运行时比较（RFC-036 测试框架
`assert_eq` 依赖此行为，不受影响）。

**`%` 语义修正**：`-7 % 3` 从 `-1`（截断余数）变为
`2`（数学取模）。解释器（`checked_rem`）、常量折叠（`a % b`）、字节码（`I64_REM`）三处同步修改。

### 编译器改动

| 组件                                 | 改动                                                                                                                                                  |
| ------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| `typecheck/inference/expressions.rs` | `infer_binary` 白名单改为「原生快路径 + 登记表查询」双路；`Expr::Index` 白名单增加接口查询分支                                                        |
| `typecheck/inference/bounds.rs`      | `check_trait_bounds` 对运算符接口名改查接口登记表（`Equal` 含结构推导）                                                                               |
| `typecheck/checker.rs`               | 新增 `Equal` 结构推导（记录定义后）；实例化级重载规则（同名接口多实例化按方法签名共存）                                                               |
| `middle/core/ir_gen.rs`              | `Expr::Index` 增加方法调用派发分支；无显式 `equal` 的结构体 `==` 生成原生逐字段比较；`Mod` 指令改数学取模                                             |
| 接口登记表（新增）                   | Layer 0 映射表常量 + 运算符→接口查询函数 + 核心默认登记（`Int`/`Float`/`String`/`List` 等）                                                           |
| `trait_data.rs`                      | `Equal` 注册与旧自动派生停用（`Clone`/`Dup`/`Debug` 维持现状）                                                                                        |
| 诊断                                 | `Equal` 前置（线性令牌）不满足复用 `E1101`（类型未实现接口）族；`E1081`/`E1082` 文案去 "Result" 字样（阶段 2 随 `Try` 落地，按 RFC-013 流程三方同步） |

### 向后兼容性

**基本类型运算不变**：`1 + 2`、`"a" + "b"`、`[1] + [2]`、`1 < 2` 全部保留原生路径，行为与性能不变。

**`1 + 2.5` 从编译错误变为合法**：核心登记 `Add(Int, Float, Float)`
后，原先被白名单拒绝的混合算术可用，返回 `Float`。这是新增能力，无既有代码受影响。

**`%` 语义修正**：`-7 % 3` 从 `-1` 变为
`2`。定性为**缺陷修正**（文档早已承诺取模，见动机 §5），不留兼容期；现有测试语料无依赖负数 `%`
的用例（已核实）。

**结构体 `==` 从运行时错误变为可用**：此前 `Struct == Struct`
一律 E6007 运行时错误，接线后自动派生即可用——从坏到好，无既有合法代码受影响。

**`Any` 的 `==` 不受影响**：动态类型位置保持运行时比较（RFC-036 的 `assert_eq`
断言族依赖，此前可用、此后仍可用）。

**`f[0]` 位置绑定不变**：`distance[0]` 是 RFC-004 的编译器内置能力，**不走 `Index`
接口**，不受影响。

**`?` 对现有代码透明**（阶段 2）：`Result` 补上 `Try` 实例化后，现有 `?` 用法行为完全一致。

## 权衡

### 优点

- **兑现 RFC-011 的运算符约束**：`T: Add + Multiply`
  从纸面（实为编译错误）变为可落地，不动已接受 RFC 正文
- **解除 `Result` 的 core 绑定**（阶段 2）：`?` 与构造子接口化后 `Result`
  可归 std，与 RFC-013 既有定位对齐
- **修复实测缺陷**：`Point == Point` 开箱即用，连带修复 `list.contains` 对结构体不可用
- **零新语法**：全部复用 RFC-011a 既有机制，不触碰 parser 语法规则
- **零运行时开销**：静态派发 + 基本类型原生快路径
- **用户自定义容器可用**：`Box(T)[0]` 从「一律报错」变为可用
- **语言内部一致**：Tuple/List 已是逐元素比较，记录类型补齐；算术接口三参数与 Index 形状统一；RFC-011
  §8.3 提升表与接口登记合一

### 缺点

- **`Equal`
  自动派生是静默默认**：将来若想收回「记录默认可比」是破坏性变更。接受此立场——与 Tuple/List 既有行为一致，且显式实例化始终可覆盖
- **`Equal` 前置会拒绝含线性令牌的类型**：含 `&mut` 字段的类型无法
  `==`，这是语义正确的代价，需清晰诊断（指明哪个字段）
- **接口实例化是显式成本**：算术运算符每个要写一行实例化 + 一个方法（`Equal`
  已豁免——自动派生）。RFC-011a 的语法决定了无法隐式推导（换来的是 `Self` 类型参数无魔法）
- **`%` 语义修正是行为变更**：虽定性为缺陷修正，仍需在迁移说明中标注

## 替代方案

### 方案 A：只做 Layer 1（按方法名派发），不做接口层

`+` 只查是否有名为 `add` 的方法，不要求实现 `Add` 接口。

**否决理由**：RFC-011 的 `T: Add`
约束将与运算符脱节——约束查接口，运算符查方法名，两者可能给出不一致的答案。且无法在编译期给「`+`
用于不可加类型」提供准确诊断。

### 方案 B：引入构造子语法 `Ok(x)` / `Some(x)` 解决 `?` 问题

不接口化 `?`，而是给和类型加构造子语法。

**否决理由**：与 RFC-010（已接受）冲突。RFC-010 明确规定「统一使用记录类型表达和类型，**不需要两套语法**」，并显式废弃了
`|` 语法。引入构造子是引入第二套表达。且它只解决 `?`，不解决 `Point == Point` 与自定义容器索引。

### 方案 C：比较运算符也首批接口化（引入 `Ordering`）

`<` `<=` `>` `>=` 走 `Compare` 接口，返回三值 `Ordering`。

**否决理由**：`<`
在 YaoXiang 已是**一等 IR 指令**（`Instruction::Lt/Le/Gt/Ge`），接口化会让基本类型被迫绕行。且引入
`Ordering` 会带出 `Ordering` 自身的比较/排序、浮点 `NaN` 的 `PartialOrd` vs `Ord`
等一整套问题，属独立 RFC 的量。当前实测暴露的需求（`Point == Point`、`list.contains`）**只需
`Equal`**。

### 方案 D：运算符名用缩写（`Add` 接口的方法叫 `add`，接口叫 `Mul`）

**否决理由**：RFC-011 正文已写 `T: Add + Multiply + Zero`，用缩写需改动已接受 RFC。

### 方案 E：`Equal` 只做显式实例化，不自动派生

**否决理由**：用户自己写的类型居然不能 `==`，体验不可接受；且 `(1, 2) == (1, 2)`、`[1] == [1]`
今天已是逐元素比较，唯独记录类型被排除，本来就不一致。显式实例化保留为覆盖手段，兼顾自定义需求。

### 方案 F：算术接口返回类型写死 `-> Self`

**否决理由**：`Add(Int, Float)` 的方法将被迫返回 `Int`，`1 + 2.5` 无法正确表达；向量缩放
`Multiply(Point, Float)` 同理写不出来。且 RFC-011 §8.3 的提升类型族 `(Int, Float) => Float`
将失去落地位置。三参数 `(Self, R, O)` 与 `Index(Key, Value)` 形状统一。

## 实现策略

### 依赖关系

| 依赖                        | 状态      | 影响本 RFC 的哪部分                                   |
| --------------------------- | --------- | ----------------------------------------------------- |
| RFC-011a 接口机制 Phase 1–3 | ✅ 已落地 | Layer 2 的地基（已实测可运行）                        |
| RFC-010 记录式构造路径      | ❌ 未落地 | **阶段 2**：`?` 的构造侧 + 构造子 parser 特判退役     |
| RFC-010b 变体解构          | ❌ 纸面   | **阶段 2**：`Result.residual` 的 `match` 写法、穷尽性 |
| RFC-009 线性令牌推导        | 部分      | `Equal` 的前置约束检查                                |

### 分阶段

按**接口依赖**（设计约束，非排期）分为两组：

**阶段 1 — 不依赖 RFC-010/010b**：

- Layer 0 映射表 + 接口实现登记表 + Layer 1 派发接线
- `Add` `Subtract` `Multiply` `Divide` `Modulo` `Equal` `Index` 七个接口（三类型参数形态）
- `Equal` 自动派生 + 线性前置约束 + 约束求解改道登记表
- `%` 语义修正（含解释器/常量折叠/字节码三处）
- `Any` 位置 `==` 保持运行时比较
- **收益**：`Point + Point`、`Point == Point`（免仪式）、`Box(T)[0]`、`1 + 2.5`、`T: Add`
  约束全部可用

**阶段 2 — 依赖 RFC-010 / RFC-010b，开工前须定案 `Try` 形状**：

- `Try` 接口形状定案（见开放问题）
- `?` 接口化 + 构造子（`ok`/`err`/`some`）parser 特判退役，改走 RFC-010 记录构造路径
- `Result` 从 core 迁至 std（依据 RFC-013 既有定位）

### 风险

| 风险                                 | 缓解                                                     |
| ------------------------------------ | -------------------------------------------------------- |
| `%` 语义变更影响现有用户代码         | 定性为缺陷修正 + 已核实测试语料无负数 `%` 依赖           |
| `Equal` 自动派生的静默默认将来难收回 | 立场已定：与 Tuple/List 既有行为一致，显式实例化可覆盖   |
| 基本类型双路径（原生 + 登记）不一致  | 门禁：核心登记的语义须与原生指令一致（同一张表两个视角） |
| 登记表查询拖慢编译                   | 表按类型名索引 + 实例化结果缓存（复用 RFC-011a proof）   |
| 实例化级重载引入歧义                 | 与方法级重载同规则：签名冲突即 E1097                     |

## 与其他 RFC 的协同

本 RFC 的定位是**消费方需求提出者**，需同步更新以下 RFC 以保证协同一致（已获授权修订）：

| RFC                   | 需更新的内容                                                                                                                                                                                        |
| --------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **RFC-011**（已接受） | §约束处注明 `Add` / `Multiply` 由 011b 定义并落地（`T: Add` ≜ `Add(T, T, T)`）；标注 `Zero` / `One` / `PartialOrd` / `Fn` / `FnMut` 为悬空约束名（待后续 RFC）；§8.3 提升类型族注明与接口登记表合一 |
| **RFC-010**（已接受） | 明确「记录字段即构造函数」的落地要求——它是阶段 2 构造子 parser 特判退役的前置                                                                                                                       |
| **RFC-010b**（草案，原 RFC-039） | 构造与解构必须成对；穷尽性判定不依赖 `Result` 的 core/std 归属（011b 阶段 2 将迁移）                                                                                                                |
| **RFC-009**（已接受） | §类型属性处交叉引用：`Equal` 前置为「不含 `&mut` 线性令牌」（非 Dup）                                                                                                                               |
| **RFC-013**（已接受） | 阶段 2 落地时：`E1081` / `E1082` 文案去 "Result" 字样；`Equal` 前置诊断复用 `E1101` 族——按 RFC-013 流程三方同步（codes/*.rs ↔ locales ↔ 码表）                                                      |
| **RFC-018**（已接受） | `%` 改数学取模后，`Mod → srem/urem` 映射表失效，需改为 `srem` + 符号修正（或 `sdiv`+`mul`+`sub` 合成），并更正「取模/余数」术语混用                                                                 |
| **RFC-036**（已接受） | 无需改动。注明关系：`Any` 位置 `==` 保持运行时比较，`assert_eq` 断言族不受闸门影响                                                                                                                  |

> `Result` 归 std 的依据：RFC-013 已定位「std 库
> `Result(T, Error)`」，RFC-014 的分层把 std 归入核心源。本 RFC 阶段 2 是其落地路径之一，不再引用其他编号。

## 开放问题

- [x] ~~`Modulo` 的语义修正是否需要兼容期？~~ →
      **关闭**：语言参考早已写明「乘除取模」，当前 remainder 实现违反已发布文档，按缺陷修正处理，不留兼容期（2026-09-22）
- [x] ~~用户能否为已存在的类型补充运算符实现（孤儿规则）？~~ →
      **定案**：运算符实现只能写在类型的定义模块里。`Int`
      的定义在核心，故只有核心能给它登记；用户只能为自己的类型登记。所有类型一视同仁，内建类型无特权（2026-09-22）
- [ ] `Index` 是否要区分可变索引（类似 Rust 的 `IndexMut`）？（首批只读；可变索引涉及 RFC-009 的
      `WriteToken`，且 RFC-011a 已有 `&mut Self` 接收者先例可循，留待后续）
- [ ] 多位置索引的 Key 用元组打包（现状）还是改为多参数（Swift 风格）？（沿用元组打包，不改解析器）
- [ ] `Zero` / `One`
      的形态：常量成员不是运算符，「无接收者的接口成员」在 RFC-011a 无先例，需单独定案后 RFC-011 的
      `T: Add + Multiply + Zero` 全句才可兑现
- [ ] `Try` 接口的完整形状：`?` 需要成败判定、成功载荷提取、失败路径产出外层返回值三件事，单方法
      `residual` 不足以覆盖；与构造子 parser 特判退役一起，阶段 2 开工前定案
- [ ] `?T` 前缀类型（RFC-026 FFI 可空标注、RFC-018）与 `e?` 后缀运算符共用 `?`
      符号：位置不同（类型位 vs 表达式位）不构成冲突，成文说明即可
- [ ] `PartialOrd` / `Ordering`（比较运算符接口化）：独立 RFC，本 RFC 明确不做

---

## 附录A：调研证据

以下实测均在 **0.8.0**（`target/debug/yaoxiang-rs.exe`）上复现。

### A.1 接口机制可用性（本 RFC 的地基）

| 能力                                                       | 实测                                             |
| ---------------------------------------------------------- | ------------------------------------------------ |
| 接口声明 `Animal: (Self: Type) -> Type = {...}`            | ✅ 可定义                                        |
| 接口实例化 + 外部方法 `Dog: { Animal(Dog) }` + `Dog.speak` | ✅ 可运行，输出正确（`tests/rfc011a.rs` 有单测） |
| 接口实例化 + **内部**方法声明                              | ❌ `E1097`（字段与方法共用命名空间冲突）         |
| 方法派发 `d.speak()`                                       | ✅ 可运行                                        |

**注**：`E1097`
意味着运算符方法必须用**外部声明**形态（`Point.add: (self: &Point, ...)`），与 RFC-011a 的示例一致。`method_bindings`
的注册在 `environment.rs`（`add_method_binding`），查询在 `expressions.rs`
的调用目标解析与字段查找失败 fallback 路径。

### A.2 运算符现状

| 表达式                              | 现状                                                          |
| ----------------------------------- | ------------------------------------------------------------- |
| `1 + 2` / `"a" + "b"` / `[1] + [2]` | ✅ 硬编码白名单（Int/Float/String/List，**要求两侧同型**）    |
| `1 + 2.5`（Int + Float）            | ❌ 编译错误（白名单要求两侧同型）                             |
| `-7 % 3`                            | `-1`（截断余数；`%` 白名单仅 Int/Float，与 `+` 的白名单不同） |
| `Point(1,2) == Point(1,2)`          | ❌ `E6007`（Eq 在 Struct 上运行时失败）                       |
| `(1,2) == (1,2)` / `[1] == [1]`     | ✅ 运行时逐元素比较（Struct 是唯一缺口）                      |
| Any 位置 `a == b`（`assert_eq`）    | ✅ 运行时比较（RFC-036 实证）                                 |
| `f[0]`（函数位置绑定）              | ⚠️ 仅绑定声明内合法，作表达式报 `E3006`                       |
| `arr[0, 1]`（多位置）               | ✅ 元组打包，`list([1, 2])`                                   |

### A.3 `?` 与构造子的硬编码位置

```rust
// src/frontend/core/typecheck/inference/expressions.rs
let expected_result = MonoType::make_result(ok_ty.clone(), expected_err.clone());

// src/middle/core/ir_gen.rs
Instruction::VariantTag { group: "Result".to_string(), .. }
// variant 0 = ok, variant 1 = err
```

构造子侧：`ok(T)` / `err(E)` / `some(T)` 由解析器识别（语言规范 `syntax.md` §1.4.2）；
`std/result.rs` 的 `is_ok` / `unwrap` 等按 `variant_id 0/1`
模式匹配。「Result 归 std」需两半（`?` + 构造子）一起解，均归阶段 2。

## 附录B：设计决策记录

| 决策                 | 决定                                                 | 理由                                                                                                    | 日期       |
| -------------------- | ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------- | ---------- |
| 架构                 | 三层分离（映射表 / 派发基座 / 接口契约）             | 合并会让 RFC-011 约束与运算符脱节                                                                       | 2026-09-15 |
| 运算符放行条件       | 必须实现接口（Layer 2 为放行条件）                   | 保证 `T: Add` 与 `+` 严格对应                                                                           | 2026-09-15 |
| 接口命名             | 全拼（`Multiply` 而非 `Mul`）                        | 不改 RFC-011 已接受的正文                                                                               | 2026-09-15 |
| `%` 接口名           | `Modulo`（数学取模）                                 | 名字把语义钉死                                                                                          | 2026-09-15 |
| 比较运算符           | 首批**不**接口化，保留原生 IR 指令                   | 已是一等指令；`Ordering` 带出一整套独立问题                                                             | 2026-09-15 |
| `Ordering`           | 首批不引入                                           | 无真实需求驱动，属独立 RFC 的量                                                                         | 2026-09-15 |
| 关联类型             | 用接口类型参数，不引入 `type` 成员语法               | RFC-011a 已定案此方案（`Iterator: (Item: Type)`）                                                       | 2026-09-15 |
| 方法绑定与索引的统一 | 概念统一，接口只管容器索引                           | 位置绑定的键是编译期常量，结果类型需类型族求值，无法用户实现                                            | 2026-09-15 |
| 多位置索引           | 元组打包 + 按 Key 类型重载，不变参接口               | 不改解析器                                                                                              | 2026-09-15 |
| 位运算 / 一元运算符  | 首批不做                                             | 对自定义类型罕见，YAGNI                                                                                 | 2026-09-15 |
| 算术接口形状         | 三类型参数 `(Self, R, O)`；`T: Add` ≜ `Add(T, T, T)` | 结果类型显式化：`1 + 2.5`、缩放可表达；RFC-011 §8.3 提升表与登记表合一；与 `Index(Key, Value)` 形状统一 | 2026-09-22 |
| `Equal` 派生         | 默认自动派生 + 显式实例化覆盖；约束求解同判据        | 「自己写的类型不能 ==」不可接受；Tuple/List 已是逐元素比较，Struct 是唯一缺口                           | 2026-09-22 |
| `Equal` 前置约束     | 「不含 `&mut` 线性令牌」，**非 Dup**                 | RFC-011 §2.4 明文原语不属于 Dup，以 Dup 为前提会误拒 `Point{Float,Float}`                               | 2026-09-22 |
| 名字与登记分离       | 运算符只查接口实现登记表，不走名字解析               | 本地同名绑定（类型级 `Add` 家族等）与运算符互不干扰；RFC-011 §5.2 示例无需改动                          | 2026-09-22 |
| 孤儿规则             | 实现跟随类型的定义模块                               | 所有类型一视同仁，内建类型无特权                                                                        | 2026-09-22 |
| `Any` 的 `==`        | 保持运行时比较，不查登记表                           | RFC-036 `assert_eq` 断言族已依赖此行为                                                                  | 2026-09-22 |
| `Try` 形状           | 定案：四方法（is_failure/success/residual/from_error）+ assert-Never 死路语义；已落地 | `?` 需三件事，单方法 `residual` 不够；构造子 parser 特判随 Result 归 std 一并退役                          | 2026-09-22 |
| `%` 语义定性         | 缺陷修正（文档已承诺取模），不留兼容期               | `reference/index.md`「乘除取模」为先证                                                                  | 2026-09-22 |
| `Result` 归 std 依据 | 引 RFC-013 既有定位，不再引用不存在的编号            | RFC-013 已写「std 库 `Result(T, Error)`」                                                               | 2026-09-22 |

## 附录C：术语表

| 术语           | 定义                                                                                   |
| -------------- | -------------------------------------------------------------------------------------- |
| Layer 0        | 运算符 → 方法名的固定映射表，语言级常量，用户不可改                                    |
| Layer 1        | 按 `Type.method` 键查表并调用的派发基座                                                |
| Layer 2        | 接口契约层，为泛型约束提供依据，同时是运算符的放行条件                                 |
| 接口实现登记表 | 聚合核心默认登记与用户实例化的实现总表；运算符查询与约束求解的唯一判据，与名字解析无关 |
| 原生快路径     | 基本类型保留的硬编码运算路径，不走接口派发                                             |
| 位置绑定       | RFC-004 的 `f[0]` 语法，把函数参数位绑定为方法，编译期行为                             |
| 自动派生       | 编译器为全字段可比的记录生成的逐字段 `==`，显式实例化可覆盖                            |

## 参考文献

- [RFC-011: 泛型系统设计](./011-generic-type-system.md) — `T: Add + Multiply + Zero`
  约束、关联类型
- [RFC-011a: 接口实现与动态分发](./011a-interface-implementation.md)
  — 接口声明/实例化/重载规则
- [RFC-009: 所有权模型设计](./009-ownership-model.md) — `&mut T` 线性令牌
- [RFC-004: 柯里化方法的多位置联合绑定](./004-curry-multi-position-binding.md) — `f[0]`
  语法
- [RFC-010: 统一类型语法](./010-unified-type-syntax.md) — 和类型表达
- [RFC-013: 错误码规范](./013-error-code-specification.md) — `Result`
  归 std 定位、错误码流程
- [RFC-010b: 模式匹配完备化（变体解构与穷尽性）](../draft/010b-pattern-matching-completeness.md)
- [Rust `std::ops::Index`](https://doc.rust-lang.org/std/ops/trait.Index.html) — 关联类型 `Output`
  设计
- [Swift Subscripts](https://docs.swift.org/swift-book/documentation/the-swift-programming-language/subscripts/)
  — 多参数下标

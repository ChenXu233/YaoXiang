---
title: 'RFC-011b: 运算符重载与接口驱动运算符'
status: '草案'
author: '晨煦'
created: '2026-09-15'
updated: '2026-09-15'
group: 'rfc-011'
issue: '#341'
---

# RFC-011b: 运算符重载与接口驱动运算符

> **参考**:
>
> - [RFC-011: 泛型系统设计](./011-generic-type-system.md) — 类型约束 `T: Add + Multiply`、关联类型
> - [RFC-011a: 接口实现与动态分发](./011a-interface-implementation.md) — 接口声明/实例化机制
> - [RFC-009: 所有权模型设计](./009-ownership-model.md) — `Dup` / `Linear` 类型属性
> - [RFC-004: 柯里化方法的多位置联合绑定](./004-curry-multi-position-binding.md) — `f[0]`
>   位置绑定语法
> - [RFC-010: 统一类型语法](./010-unified-type-syntax.md) — 和类型 = 字段全返回自身类型的记录
> - [RFC-039: 模式匹配完备化](../draft/039-pattern-matching-completeness.md) — 变体解构（依赖）

## 摘要

为 YaoXiang 补齐**运算符重载**能力，使 `a + b` / `a == b` / `a[i]` / `e?`
可由用户自定义类型实现，并让 RFC-011 已写入的 `T: Add + Multiply + Zero`
约束从**纸面能力**变为可落地机制。

设计采用**三层职责分离**：固定的运算符→方法映射表（Layer 0）、按名派发的基座（Layer 1，复用既有
`method_bindings`）、接口契约层（Layer
2，用于泛型约束）。运算符的**优先级与结合性保持语言固定**，用户只重载语义。

首批范围：`Add` `Subtract` `Multiply` `Divide` `Modulo` `Equal` `Index` `Try`
八个接口。不引入新关键字、不引入新语法结构。

## 动机

### 为什么需要这个特性

#### 1. RFC-011 的核心示例依赖它，但目前是纸面能力

RFC-011（已接受）在 8 处使用运算符名作为类型约束：

```yaoxiang
multiply: (T: Add + Multiply + Zero, Rows: Int, Cols: Int, M: Int) -> (
    (a: Matrix(T, Rows, Cols), b: Matrix(T, Cols, M)) -> Matrix(T, Rows, M)
)
```

这是 RFC-011「值依赖类型 + 编译期维度验证」的招牌示例。但**全套 RFC 中从无一篇定义 `Add` /
`Multiply` 从何而来、`+` 如何绑定到它们**。`T: Add` 当前是一句无法兑现的断言。

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

连带后果：`list.contains(list_of_structs, p)` **完全不可用**——它内部依赖 `==`。

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
`?` 使用，`Result` 即可归属 std。

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

### 已有的半成品基础

调研发现机制**大部分已在**，缺的是接线：

| 机制                                      | 位置                                | 状态                                 |
| ----------------------------------------- | ----------------------------------- | ------------------------------------ |
| 接口声明 `(Self: Type) -> Type`           | RFC-011a Phase 1                    | ✅ 可运行                            |
| 接口实例化 `Animal(Dog)` + 外部方法声明   | RFC-011a Phase 2–3                  | ✅ 可运行                            |
| 方法派发 `method_bindings["Type.method"]` | `expressions.rs:1299`               | ✅ 可运行                            |
| 关联类型（= 接口类型参数）                | RFC-011 §3.1；RFC-011a 已采纳此方案 | ✅ 机制已定                          |
| 类型族求值 `AssociatedTypeDef`            | `dependent_types.rs`                | ✅ 生产用例 `std.assert` 的 `IsTrue` |
| `Equal` / `Dup` / `Clone` / `Debug` trait | `trait_data.rs` 已注册              | ⚠️ `Equal` **零消费点**              |
| 运算符 → 方法名映射                       | —                                   | ❌ 不存在                            |

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
         使 T: Add 约束成立，且作为运算符的放行闸门
```

**分层理由**：

- **Layer 0 与 1 让运算符「能用」**，Layer 2 让运算符「能被约束」。合并会导致任何名为 `add`
  的方法都被 `+` 调用，RFC-011 的 `T: Add` 与运算符脱节。
- **Layer 1 不新造**：`method_bindings` 已在按 `Type.method`
  键查表（字段查找失败后的 fallback 路径），运算符走同一条路即可。
- **Layer 2 是闸门**：运算符放行前**必须**确认类型实现了对应接口（见 §"为什么必须实现接口"）。

### Layer 0：固定映射表

| 运算符            | 接口                          | 方法       | 首批 |
| ----------------- | ----------------------------- | ---------- | ---- |
| `+`               | `Add`                         | `add`      | ✅   |
| `-`               | `Subtract`                    | `subtract` | ✅   |
| `*`               | `Multiply`                    | `multiply` | ✅   |
| `/`               | `Divide`                      | `divide`   | ✅   |
| `%`               | `Modulo`                      | `modulo`   | ✅   |
| `==` `!=`         | `Equal`                       | `equal`    | ✅   |
| `[]`              | `Index`                       | `index`    | ✅   |
| `?`               | `Try`                         | `residual` | ✅   |
| `<` `<=` `>` `>=` | —（保留原生指令）             | —          | ❌   |
| `and` `or`        | —（短路是语言语义，不可重载） | —          | ❌   |
| 位运算 5 个       | —                             | —          | ❌   |
| 一元 `-` `!`      | —                             | —          | ❌   |

**接口名用全拼而非缩写**（`Multiply` 而非 `Mul`）：与 RFC-011 正文的 `T: Add + Multiply + Zero`
保持一致，**不改动已接受 RFC**。

**`%` 采用 `Modulo` 语义**（数学取模，结果符号跟随除数），而非当前实现的 remainder 行为（`-7 % 3`
现返回 `-1`）。这是**有意的语义修正**——名字把语义钉死为数学取模。

### Layer 2：接口定义

#### 算术接口

```yaoxiang
Add: (Self: Type, R: Type) -> Type = {
    add: (self: &Self, other: &R) -> Self
}

Subtract: (Self: Type, R: Type) -> Type = {
    subtract: (self: &Self, other: &R) -> Self
}

Multiply: (Self: Type, R: Type) -> Type = {
    multiply: (self: &Self, other: &R) -> Self
}

Divide: (Self: Type, R: Type) -> Type = {
    divide: (self: &Self, other: &R) -> Self
}

Modulo: (Self: Type, R: Type) -> Type = {
    modulo: (self: &Self, other: &R) -> Self
}
```

`R` 是右操作数类型，允许左右异型（如 `Int + Float`）。返回值固定为 `Self` （`&Self`
借用接收者，见 RFC-009 借用令牌与 RFC-011a 接收者约定）。

#### 相等接口

```yaoxiang
Equal: (Self: Type, R: Type) -> Type = {
    equal: (self: &Self, other: &R) -> Bool
}
```

**`Equal` 隐含要求 `Dup`**：`==` 的语义是「比较两个值」，需要双方可读。线性（非
`Dup`）类型的值只能读一次，无法参与比较。RFC-009 已给出因果链：

> **因果关系不能倒：冻结是原因，Dup 是结果。**

因此 `Equal` 的实现前提是 `Self` 具备 `Dup` 属性；不具备时编译器拒绝 `Equal` 实例化。

#### 索引接口

```yaoxiang
Index: (Self: Type, Key: Type, Value: Type) -> Type = {
    index: (self: &Self, key: &Key) -> Value
}
```

**`Value`
作为类型参数而非关联类型成员**：RFC-011a 的开放问题已定案「关联类型通过泛型接口参数实现」（`Iterator: (Item: Type) -> Type`
是同构先例），不需要引入 `type` 成员语法。

**多位置索引靠元组打包 + 重载**，不引入变参接口：

```yaoxiang
// 一维容器
List(T) 实例化 Index(List(T), Int, T)                   → arr[0]

// 多维容器
Grid    实例化 Index(Grid, Tuple(Int, Int), Float)       → g[0, 1]
//                    └─ Key 是元组

// 两个实例化签名不同 → 按 RFC-011a 重载规则共存
```

#### 传播接口

```yaoxiang
Try: (Self: Type, T: Type, E: Type) -> Type = {
    residual: (self: &Self) -> E
}
```

`T`（成功类型）与 `E`（错误类型）同为接口类型参数。

**`?` 的类型规则**：

```yaoxiang
// f: () -> Result(T, E)
// 函数体内余下部分要求 Result(U, E)（错误类型必须一致）
x = f()?        // x: T，等价于：
                //   match f() {
                //       ok(v)  => v
                //       err(e) => return err(e)
                //   }
```

错误类型不一致时编译报错（对应既有 `E1083`）；外层函数返回类型不是可传播类型时报
`E1081`；表达式未实现 `Try` 时替换 `E1082` 的文案（不再提 "Result" 这个具体名字）。

### 示例

#### 用户自定义类型的算术与相等

```yaoxiang
Point: Type = {
    x: Float,
    y: Float,
    Add(Point, Point),
    Equal(Point, Point)
}

Point.add: (self: &Point, other: &Point) -> Point =
    Point(self.x + other.x, self.y + other.y)

Point.equal: (self: &Point, other: &Point) -> Bool =
    self.x == other.x and self.y == other.y

main: () -> Void = {
    a = Point(1.0, 2.0)
    b = Point(3.0, 4.0)
    c = a + b                   // Point(4.0, 6.0)
    println(a == b)             // false
    println(a == a)             // true
}
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

#### 让 `Result` 归 std（设计目标）

`Result` 实现 `Try` 后，`?` 不再依赖类型名硬编码：

```yaoxiang
// std.result 内（不再需要 core 特判）
Result: (T: Type, E: Type) -> Type = {
    ok: (T) -> Result(T, E),
    err: (E) -> Result(T, E),
    Try(Result(T, E), T, E)
}

Result.residual: (T: Type, E: Type)(self: &Result(T, E)) -> E =
    match self {
        ok(_) => abort_invalid_residual(),   // ok 路径不应走到 residual
        err(e) => e
    }
```

#### 泛型约束终于可兑现

```yaoxiang
// RFC-011 的示例从此可落地
combine: (T: Add + Multiply)(a: T, b: T, c: T) -> T =
    a * b + c
```

### 语法变化

**无新语法、无新关键字**。所有能力由既有机制组合而成：

| 能力         | 复用的既有机制                             |
| ------------ | ------------------------------------------ |
| 接口声明     | RFC-011a Phase 1                           |
| 接口实例化   | RFC-011a Phase 2（`Dog: { Animal(Dog) }`） |
| 方法实现     | RFC-011a Phase 3（外部声明 `Point.add`）   |
| 关联类型     | RFC-011 §3.1（接口类型参数）               |
| 多位置索引   | 既有元组打包解析 + RFC-011a 重载规则       |
| 运算符优先级 | **语言固定**，不开放用户自定义             |

## 详细设计

### 类型系统影响

**新增接口**（Layer 2）：`Add` `Subtract` `Multiply` `Divide` `Modulo` `Equal` `Index` `Try`。

**复用已注册 trait**：`Equal` 已在 `trait_data.rs` 注册但**零消费点**（`==` 当前只做 `unify`
后硬编码返回 `Bool`）。本 RFC 将其接为 `==` 的实际契约。

**`Dup` 前置约束**：`Equal` 实例化时检查 `Self` 是否为 `Dup`。`Dup` / `Linear`
属性由 RFC-009 推导（`&T` 冻结 ⇒ Dup；`&mut T` 独占 ⇒ Linear）。

**约束求解**：`T: Add` 的求解 = 查 `T` 是否实例化了 `Add`
接口。机制复用 RFC-011a 的编译期类型收集，无新增求解器。

### 运行时行为

**零运行时开销**：运算符在编译期确定调用目标（静态派发）。对基本类型（`Int`/`Float`/
`String`/`List`）保留**原生指令快路径**，不走接口派发。

`?` 的运行时行为不变（解包 + 提前返回），只是判定依据从「类型名 == "Result"」变为「类型实现了
`Try`」。

### 编译器改动

| 组件                                 | 改动                                                                                      |
| ------------------------------------ | ----------------------------------------------------------------------------------------- |
| `typecheck/inference/expressions.rs` | `infer_binary` 的白名单改为「原生快路径 + 接口查询」双路；新增 `Index` / `Try` 的接口查询 |
| 同上（`Try` 臂）                     | 移除 `MonoType::make_result` 硬编码，改为查询 `Try` 实例化                                |
| `middle/core/ir_gen.rs`              | `Expr::Try` 移除 `group: "Result"` 硬编码；`Expr::Index` 增加接口派发分支                 |
| `typecheck/types/trait_data.rs`      | 注册 8 个接口的默认实例化（`Int` / `Float` / `String` / `List` 等）                       |
| 新增                                 | Layer 0 映射表常量 + 运算符→接口的查询函数                                                |
| 诊断                                 | `E1082` 文案从 "requires Result" 改为 "requires Try"（`E1081`/`E1083` 沿用）              |

### 向后兼容性

**基本类型运算不变**：`1 + 2`、`"a" + "b"`、`[1] + [2]`、`1 < 2` 全部保留原生路径，行为与性能不变。

**`%` 语义变更**：`-7 % 3` 从 `-1`（remainder）变为
`2`（modulo）。这是**有意的修正**，需在迁移说明中标注；现有测试语料无依赖负数 `%` 的用例（已核实）。

**`f[0]` 位置绑定不变**：`distance[0]` 是 RFC-004 的编译器内置能力，**不走 `Index`
接口**，不受影响。

**`?` 对现有代码透明**：`Result` 补上 `Try` 实例化后，现有 `?` 用法行为完全一致。

## 权衡

### 优点

- **兑现 RFC-011**：`T: Add + Multiply + Zero` 从纸面变为可落地，不动已接受 RFC 正文
- **解除 `Result` 的 core 绑定**：`?` 接口化后 `Result` 可归 std，承接分层目标
- **修复实测缺陷**：`Point == Point` 可用，连带修复 `list.contains` 对结构体不可用
- **零新语法**：全部复用 RFC-011a 既有机制，不触碰 parser 语法规则（遵守 RFC-036 零语法变更原则）
- **零运行时开销**：静态派发 + 基本类型原生快路径
- **用户自定义容器可用**：`Box(T)[0]` 从「一律报错」变为可用

### 缺点

- **`%` 语义变更是破坏性变更**：虽有真实的语义理由，仍需迁移说明
- **`Equal` 隐含 `Dup` 会拒绝部分类型**：线性类型的值无法
  `==`，这是语义正确的代价，但用户可能感到意外，需清晰的诊断信息
- **接口实例化是显式成本**：每个运算符要写一行接口实例化 + 一个方法。RFC-011a 的语法决定了无法隐式推导（换来的是`Self`
  类型参数无魔法）

## 替代方案

### 方案 A：只做 Layer 1（按方法名派发），不做接口层

`+` 只查是否有名为 `add` 的方法，不要求实现 `Add` 接口。

**否决理由**：RFC-011 的 `T: Add`
约束将与运算符脱节——约束查接口，运算符查方法名，两者可能给出不一致的答案。且无法在编译期给「`+`
用于不可加类型」提供准确诊断。

### 方案 B：引入构造子语法 `Ok(x)` / `Some(x)` 解决 `?` 问题

不接口化 `?`，而是给和类型加构造子语法。

**否决理由**：与 RFC-010（已接受）冲突。RFC-010 明确规定「统一使用记录类型表达和类型，
**不需要两套语法**」，并显式废弃了 `|` 语法。引入构造子是引入第二套表达。且它只解决 `?`，不解决
`Point == Point` 与自定义容器索引。

### 方案 C：比较运算符也首批接口化（引入 `Ordering`）

`<` `<=` `>` `>=` 走 `Compare` 接口，返回三值 `Ordering`。

**否决理由**：`<`
在 YaoXiang 已是**一等 IR 指令**（`Instruction::Lt/Le/Gt/Ge`），接口化会让基本类型被迫绕行。且引入
`Ordering` 会带出 `Ordering` 自身的比较/排序、浮点 `NaN` 的 `PartialOrd` vs `Ord`
等一整套问题，属独立 RFC 的量。当前实测暴露的需求（`Point == Point`、`list.contains`）**只需
`Equal`**。

### 方案 D：运算符名用缩写（`Add` 接口的方法叫 `add`，接口叫 `Mul`）

**否决理由**：RFC-011 正文已写 `T: Add + Multiply + Zero`，用缩写需改动已接受 RFC。

## 实现策略

### 依赖关系

| 依赖                        | 状态      | 影响本 RFC 的哪部分                                        |
| --------------------------- | --------- | ---------------------------------------------------------- |
| RFC-011a 接口机制 Phase 1–3 | ✅ 已落地 | Layer 2 的地基（已实测可运行）                             |
| RFC-010 记录式构造路径      | ❌ 未落地 | **`?` 的构造侧**（造不出 `Result` 值则无法完整测试 `Try`） |
| RFC-039 变体解构            | ❌ 纸面   | `Result.residual` 的 `match` 写法                          |
| RFC-009 `Dup`/`Linear` 推导 | 部分      | `Equal` 的前置约束检查                                     |

### 分阶段

按**接口依赖**（设计约束，非排期）分为两组：

**阶段 1 — 不依赖 RFC-010/039**：

- Layer 0 映射表 + Layer 1 派发接线
- `Add` `Subtract` `Multiply` `Divide` `Modulo` `Equal` `Index` 七个接口
- `Equal` ⇒ `Dup` 前置约束
- 基本类型原生快路径保持
- **收益**：`Point + Point`、`Point == Point`、`Box(T)[0]` 全部可用

**阶段 2 — 依赖 RFC-010 / RFC-039**：

- `Try` 接口 + `?` 接口化
- `Result` 从 core 迁至 std
- **阻塞原因**：需要能构造 `Result` 值（RFC-010）并解构它（RFC-039）

### 风险

| 风险                                | 缓解                                                   |
| ----------------------------------- | ------------------------------------------------------ |
| `%` 语义变更破坏现有用户代码        | 迁移说明 + 已核实测试语料无负数 `%` 依赖               |
| 接口实例化的显式成本让用户嫌繁琐    | 诊断信息提示「需实现 X 接口」并给出实例化示例          |
| 基本类型双路径（原生 + 接口）不一致 | 门禁：`Int` 等实现接口但其方法语义须与原生指令一致     |
| Layer 2 闸门导致既有代码编译失败    | `Equal` 现零消费点，接线后影响的只有此前本就报错的用法 |

## 与其他 RFC 的协同

本 RFC 的定位是**消费方需求提出者**，需同步更新以下 RFC 以保证协同一致（已授权修订）：

| RFC                   | 需更新的内容                                                 |
| --------------------- | ------------------------------------------------------------ |
| **RFC-011**（已接受） | 在 §约束 处注明 `Add` / `Multiply` 等由 RFC-011b 定义并落地  |
| **RFC-010**（已接受） | 明确「记录字段即构造函数」的落地要求——它是 `?` 阶段 2 的前置 |
| **RFC-039**（草案）   | 构造与解构必须成对；穷尽性检查对 `std` 变体集的处理          |
| **RFC-009**（已接受） | `Equal` ⇒ `Dup` 前置在 §类型属性 处交叉引用                  |

## 开放问题

- [ ] `Modulo` 的语义修正是否需要兼容期？（@晨煦：倾向直接改，因为当前行为是 bug 而非特性）
- [ ] `Index` 是否要区分可变索引（类似 Rust 的
      `IndexMut`）？（@晨煦：首批只读，可变索引涉及 RFC-009 的 `WriteToken`，留待后续）
- [ ] 多位置索引的 Key 用元组打包（现状）还是改为多参数（Swift 风格）？（@晨煦：沿用现状元组，避免改解析器）
- [ ] 用户能否为**已存在的类型**补充运算符实现？（如给 `List` 加自定义 `Add`）——涉及孤儿规则

---

## 附录A：调研证据

以下实测均在 **0.8.0**（`target/debug/yaoxiang-rs.exe`）上复现。

### A.1 接口机制可用性（本 RFC 的地基）

| 能力                                                       | 实测                                     |
| ---------------------------------------------------------- | ---------------------------------------- |
| 接口声明 `Animal: (Self: Type) -> Type = {...}`            | ✅ 可定义                                |
| 接口实例化 + 外部方法 `Dog: { Animal(Dog) }` + `Dog.speak` | ✅ 可运行，输出正确                      |
| 接口实例化 + **内部**方法声明                              | ❌ `E1097`（字段与方法共用命名空间冲突） |
| 方法派发 `d.speak()`                                       | ✅ 可运行                                |

**注**：`E1097`
意味着运算符方法必须用**外部声明**形态（`Point.add: (self: &Point, ...)`），与 RFC-011a 的示例一致。

### A.2 运算符现状

| 表达式                              | 现状                                     |
| ----------------------------------- | ---------------------------------------- |
| `1 + 2` / `"a" + "b"` / `[1] + [2]` | ✅ 硬编码白名单（Int/Float/String/List） |
| `Point(1,2) == Point(1,2)`          | ❌ `E6007`（Eq 在 Struct 上不成立）      |
| `f[0]`（函数位置绑定）              | ⚠️ 仅绑定声明内合法，作表达式报 `E3006`  |
| `arr[0, 1]`（多位置）               | ✅ 元组打包，`list([1, 2])`              |
| `-7 % 3`                            | `-1`（remainder，非 modulo）             |

### A.3 `?` 的硬编码位置

```rust
// src/frontend/core/typecheck/inference/expressions.rs
let expected_result = MonoType::make_result(ok_ty.clone(), expected_err.clone());

// src/middle/core/ir_gen.rs
Instruction::VariantTag { group: "Result".to_string(), .. }
// variant 0 = ok, variant 1 = err
```

两处均需改为接口查询。

## 附录B：设计决策记录

| 决策                 | 决定                                     | 理由                                                         | 日期       |
| -------------------- | ---------------------------------------- | ------------------------------------------------------------ | ---------- |
| 架构                 | 三层分离（映射表 / 派发基座 / 接口契约） | 合并会让 RFC-011 约束与运算符脱节                            | 2026-09-15 |
| 运算符放行条件       | 必须实现接口（Layer 2 是闸门）           | 保证 `T: Add` 与 `+` 严格对应                                | 2026-09-15 |
| `Equal` 与 `Dup`     | `Equal` 定义里写死 `Dup` 前置            | RFC-009 因果链：冻结是原因，Dup 是结果                       | 2026-09-15 |
| 接口命名             | 全拼（`Multiply` 而非 `Mul`）            | 不改 RFC-011 已接受的正文                                    | 2026-09-15 |
| `%` 接口名           | `Modulo`（数学取模）                     | 名字把语义钉死，当前 remainder 行为是缺陷                    | 2026-09-15 |
| 比较运算符           | 首批**不**接口化，保留原生 IR 指令       | 已是一等指令；`Ordering` 带出一整套独立问题                  | 2026-09-15 |
| `Ordering`           | 首批不引入                               | 无真实需求驱动，属独立 RFC 的量                              | 2026-09-15 |
| `?` 接口名           | `Try`，方法 `residual`                   | 不预设与未来三元语法糖的统一，避免过度设计                   | 2026-09-15 |
| 关联类型             | 用接口类型参数，不引入 `type` 成员语法   | RFC-011a 已定案此方案（`Iterator: (Item: Type)`）            | 2026-09-15 |
| 方法绑定与索引的统一 | 概念统一，接口只管容器索引（Layer A）    | 位置绑定的键是编译期常量，结果类型需类型族求值，无法用户实现 | 2026-09-15 |
| 多位置索引           | 元组打包 + 按 Key 类型重载，不变参接口   | 不改解析器；靠 RFC-011a 重载规则共存                         | 2026-09-15 |
| 位运算 / 一元运算符  | 首批不做                                 | 对自定义类型罕见，YAGNI                                      | 2026-09-15 |

## 附录C：术语表

| 术语       | 定义                                                            |
| ---------- | --------------------------------------------------------------- |
| Layer 0    | 运算符 → 方法名的固定映射表，语言级常量，用户不可改             |
| Layer 1    | 按 `Type.method` 键查表并调用的派发基座                         |
| Layer 2    | 接口契约层，为泛型约束提供依据，同时是运算符的放行闸门          |
| 原生快路径 | 基本类型（Int/Float/String/List）保留的硬编码运算路径，不走接口 |
| 位置绑定   | RFC-004 的 `f[0]` 语法，把函数参数位绑定为方法，编译期行为      |

## 参考文献

- [RFC-011: 泛型系统设计](./011-generic-type-system.md) — `T: Add + Multiply + Zero` 约束、关联类型
- [RFC-011a: 接口实现与动态分发](./011a-interface-implementation.md) — 接口声明/实例化/重载规则
- [RFC-009: 所有权模型设计](./009-ownership-model.md) — `Dup` / `Linear`
- [RFC-004: 柯里化方法的多位置联合绑定](./004-curry-multi-position-binding.md) — `f[0]` 语法
- [RFC-010: 统一类型语法](./010-unified-type-syntax.md) — 和类型表达
- [RFC-039: 模式匹配完备化](../draft/039-pattern-matching-completeness.md)
- [Rust `std::ops::Index`](https://doc.rust-lang.org/std/ops/trait.Index.html) — 关联类型 `Output`
  设计
- [Swift Subscripts](https://docs.swift.org/swift-book/documentation/the-swift-programming-language/subscripts/)
  — 多参数下标

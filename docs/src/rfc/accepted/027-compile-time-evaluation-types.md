---
title: 'RFC-027：编译期谓词与统一静态验证'
status: '已接受'
author: '晨煦'
created: '2026-06-07'
updated: '2026-11-14'
impl_status: 'in-progress'
impl_detail:
  'Phase 1-2 完成，Phase 3 部分完成，Phase 4 部分完成。assert/Assert 统一方案 6 Phase
  全部实现（#157-#162 已关闭）：Never 类型、IsTrue 桥接、流敏感 Γ + kill
  set、类型级递归、宇宙分层弱检查。§11 的 dispatch 分派**语义**已由调用点义务实现
  （`checker.rs` 三处直接调 `check_predicate`：绑定位 / 调用点实参 / 返回位），但**独立的
  `layers/dispatch.rs` 模块已删除**——它零生产调用点，且对 `Unproven` 的处置（降级为 W1080
  警告）与生产实做（一律报错）相冲突（#377-2）。证明内核、逻辑签名、证书与 SMT 角色按
  RFC-039 D62 的统一项世界重写（§4、§8、§13）：SMT/tactics/proof search 是**不可信
  certificate producer**，`unsat` 只是搜索结果，必须构造出证书经 `Kernel::check` 验证才得
  Proved。`predicate.rs` 当前的 `SMTResult::Unsat => Ok(())` 是裸信 SMT 的缺口，见 §8.6。
  §14 就地补完依赖记录 Σ：proof-space 字段判据、构造器 data-only + 自动合成证据、proof family 参数 `P: T -> Proposition`、
  `TypeDefPayload` 跨模块证据槽、`ConstExpr::FieldPath` 类型级 `b.w`、5A 不可变与四码分工。'
impl_percent: 82
issue_number: 90
issue_url: 'https://github.com/ChenXu233/YaoXiang/issues/90'

issue: '#90'
---

# RFC-027：编译期谓词与统一静态验证

> **参考**:
>
> - [RFC-009: 所有权模型](009-ownership-model.md)
> - [RFC-010: 统一类型语法 - name: type = value 模型](010-unified-type-syntax.md)
> - [RFC-011: 泛型系统设计](011-generic-type-system.md)
> - [RFC-024：基于 spawn 块的并发模型](024-concurrency-model.md)
>
> **取代**：[RFC-022: 霍尔逻辑静态验证支持（规约注释与规约类型）](../deprecated/022-hoare-logic-static-verification.md)
> — 已废弃

## 摘要

本文提出为 YaoXiang 引入**编译期谓词**作为一等公民，将所有编译期静态验证统一为一条**证明管道**。编译期谓词不是外挂的规约注释——它就是函数。一个返回 Type 的函数，可在类型位置使用，编译器在编译期调用它并检查返回值。类型即命题，编译期求值即证明。

**核心论点**：类型检查在编译期唯一的工作就是构造和验证证明项。类型等式、令牌冲突、依赖类型归约、
编译期谓词求值、霍尔逻辑蕴含——全部是编译期证明管道中的不同类型检查，共享同一条管道、同一个证明
内核 `Kernel::check`。证明项与命题都住在同一套**统一项世界**（RFC-039 D62）；命题由**逻辑签名**
`LogicSignature` 赋予意义，证明项以**证书** `Certificate` 表示，证书相对签名而言，只由内核按签名
的规则验证。SMT 求解器、tactic、proof search 都是**不可信的 certificate producer**：它们只负责找
证书，`unsat` 不是 `Proved`。当内核返回 Unproven 时，程序员写一个 YaoXiang 证明函数作为显式证据
——它就是一个证明项，内核以与验证任何函数返回类型完全相同的方式验证它。一切都是 YaoXiang 代码，
一切由证明内核验证。

## 动机

### 为什么废弃 RFC-022？

RFC-022 将规约设计为 `//!` 注释形式：

```yaoxiang
max: (T: Ord) -> ((arr: Array(T, n)) -> T) = {
    //! requires: NonEmpty(n) = n > 0          ← 这是独立于类型的注释
    //! ensures: ExistsMax(result, arr[0..n])   ← 这是独立于类型的注释
}
```

这犯了 Curry-Howard 同构的根本性错误：**将规约和类型分裂为两层**。注释不是类型。注释不参与类型检查。注释是"外部工具"的心智模型。

白皮书说得很清楚：

> "没有 `//!` 注释。没有独立的规约语言。一切都在类型系统内。"

### 当前的问题

- RFC-022 的 `//!` 注释是独立于类型系统的外挂语法
- 规约类型和普通类型是两套体系，造成概念冗余
- Debug Build 验证 / Release Build 忽略的分裂模式破坏了统一性
- SMT 求解器在传统认知中被定位为外部工具或「信任 Z3 的 unsat」——YaoXiang 将其定位为不可信的
  certificate producer：产出候选证书，由证明内核验证
- 类型检查、借用验证、编译期谓词检查、宏展开各自走不同的路径

### 正确的心智模型

类型检查可以抽象为一个函数：

```
Kernel::check(signature, certificate, proposition) → Proved | Disproved(Model) | Unproven
```

所有编译期检查——简单类型匹配、借用冲突检测、编译期谓词验证——都是这个函数的子任务。它们共享同一条证明管道，区别仅在于证明项复杂度和构造策略。

每个义务都携带当前生效的 `LogicSignature`。当内核返回 Unproven 时，程序员补显式证据：
提供一个证明函数（其返回类型等于待证命题）作为证书。内核用同一签名验证它——这和平常的
类型检查是同一个操作。签名可替换：换签名就是换一套推理规则，但内核与三值结果不变（§4.1）。

## 提案

### 1. `{}` 是证明空间：类型即断言，验证即类型检查

YaoXiang 的 `{}`
是编译期证明空间。里面的一切都是断言，证明内核按当前 `LogicSignature` 保证每一项有可验证的证书——要么由 producer 自动找到，要么由程序员补显式证据（§4.3）。

```
Point: Type = { x: Float, y: Float }
#               ^^^^^^^^^^^^^^^^^^^^^  编译器保证 x 是 Float，y 是 Float

List: (T: Type) -> Type = { data: Array(T) }
#                           ^^^^^^^^^^^^^^^  编译器保证 data 是 Array(T)
```

**泛型是编译期谓词的特例。**

```yaoxiang
Positive: (x: Int) -> Type = { x > 0 }
#          ^^^^^^              ^^^^^^
#          参数在签名处          {} 内只有断言
#          编译器在编译期调用时验证 x > 0

List: (T: Type) -> Type = { data: Array(T) }
#      ^^^^^^^^              ^^^^^^^^^^^^^^^
#      参数在签名处           编译器验证 type_of(T) == Type，type_of(data) == Array(T)
```

同一个模式：`name: (params) -> Type = { 断言 }`。编译器不区分"类型断言"和"值断言"——都是证明管道中的求值目标。

**循环不变式不需要单独写。变量上的类型标注就是 Floyd-Hoare 不变式。**

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # 标注引用 i——告诉编译器 s 的类型依赖 i
    mut i: UpTo(arr.len) = 0     # 初始化时 i=0，验证：0 == sum(arr[0..0]) → True
    while i < arr.len {
        s += arr[i]  # 编译器验证：s_new == sum(arr[0..i+1])
        i += 1       # i 变更→触发 s 依赖重验证：s 满足 SumUpTo(arr, i_new)
    }
    return s  # s: SumUpTo(arr, arr.len) = sum(arr[0..arr.len])
}
```

编译器对循环体生成一次验证条件——归纳假设（类型标注）→ 赋值操作 → 新值是否满足类型标注。producer 为归纳步产出候选证书、`Kernel::check` 验证成立后，所有迭代自动覆盖。不需要 `: decreases`，不需要 `: Invariant`，不需要归纳证明——归纳被分解为每个赋值的局部 VC。

### 2. 前置/后置条件：参数类型和返回类型上的编译期谓词

放弃 RFC-022 的 `//! requires`/`//! ensures`。编译期谓词作为参数或返回的类型标注。

**参数侧是函数调用。** 编译期谓词是返回 Type 的函数，参数侧的使用方式就是调用它——和 `factorial(5)`
一样。返回值侧引入了一个新概念：返回值形参。

```yaoxiang
# 前置条件：参数类型中显式调用编译期谓词
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive(b)) -> Int = a / b
#                       ^^^^^^^^^^  b 是当前形参名，传给 Positive 做参数
#                       编译器在调用点提取实参值，代入 b，验证 Positive(实参)
#                       例：divide(10, 2) → 验证 Positive(2) = { 2 > 0 } → True
#                       例：divide(10, 0) → 验证 Positive(0) = { 0 > 0 } → False → 编译错误

# 后置条件：返回值形参 + 编译期谓词
IsMax: (T: Ord, arr: Array(T), result: T) -> Type = {
    forall j in 0..arr.len: result >= arr[j]
}

NonEmpty: (arr: Array(T)) -> Type = { arr.len > 0 }

max: (T: Ord) -> ((arr: NonEmpty(arr))) -> (result: IsMax(T, arr, result)) = {
#                                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
#                                            result 是返回值形参，值由 return 提供
#                                            编译器在 return 点代入返回值，验证后置条件
    candidate = arr[0]
    for i in 1..arr.len {
        if arr[i] > candidate { candidate = arr[i] }
    }
    return candidate
}
```

**关键规则**：

- **参数侧**：`b: Positive(b)` —— `b` 是当前形参名，传给 `Positive` 做参数。函数调用语法，零隐式。
- **返回侧**：`-> (result: IsMax(T, arr, result))` —— `result` 是返回值形参，值由 `return`
  语句提供。`result` 仅存在于类型签名中、仅被谓词引用，不进入函数体作用域、不出现在调用方。
- **返回值形参可选**：无后置条件时不写，签名和普通函数完全一致（`-> Int`）。
- **统一性**：参数和返回值形参是同一个概念——`形参名: 谓词调用(实参)`，区别仅在于值由调用方提供还是由
  `return` 提供。

> **落地注记（2026-10-02）**：返回位形式参数的识别**严格按声明的名字**——`-> (r: P(...))` 里的
> `r` 是唯一的返回值形参。约束里其余**不在作用域**的自由变量不再被代入返回值，而是报未定义
> 标识符（实测：`g: () -> (r: Eq2(m, r)) = { return 7 }`（`m` 未定义）→
> `error [E1001] Unknown variable: 'm'`）。谓词调用的实参按**实参**真正代入约束：
> `P(r + 100)` 代入后是「返回值 + 100」，而不是「返回值」。
>
> **已知缺口（2026-10-02）**：**返回位的多参数谓词 + 符号实参暂不支持**。触发形态：
> `f: (b: Int) -> (r: SumUpTo(b, r)) = { return b * 2 }`（`SumUpTo: (n: Int, s: Int) -> Type = { s == n * 2 }`）
> → `error [E2031] … the refinement type constraint of 'r': SumUpTo(b, (b * 2)) cannot be statically proven`。
> 替代写法：把实参改成编译期可折叠的字面量（如 `SumUpTo(3, r)`），或为该谓词提供返回 `Type`
> 的证明函数；解除条件：证明内核支持「多参数谓词 + 符号实参」的实参代入。详见
> [RFC-027a](../accepted/027a-termination-explicit-measure.md) §实现落地记录（2026-10-02，本轮硬化）。

### 3. 路径条件传播：运行时值的编译期验证

编译期谓词在绑定位置使用时，参数由程序员显式传入。当运行时值进入精化类型参数时，producer
收集路径条件并产出候选证书，由 `Kernel::check` 按当前 `LogicSignature` 验证——无需程序员
隐式传递证明，显式证据是补证据时的一等形态（§4.3）。

#### 3.1 显式函数调用

编译期谓词在绑定位置使用时，参数由程序员显式传入——就是函数调用，零隐式。

`Positive: (x: Int) -> Type = { x > 0 }`
是一个编译期谓词构造器。当它出现在绑定位置（参数声明、变量声明、返回类型）时，程序员显式传入已绑定的变量名：

```yaoxiang
b: Positive(b)
// b 已被声明为当前形参，Positive(b) 是函数调用
// 正格化后：b: { b > 0 }
```

不需要编译器隐式填入参数——`b: Positive(b)` 和 `f(5)` 一样，就是函数调用。`b`
作为参数名被绑定，其类型标注 `Positive(b)` 引用了 `b`
自身——这是依赖类型的标准模式，不是隐式展开规则。

**与 RFC-010 `self` 的统一**：RFC-010 确立 `self` 不是关键字，只是参数名的约定俗成（"写成
`p`、`this`、`x` 效果完全一样"）。`b: Positive(b)`
共享同一个机制——参数名在类型标注中可被引用。`self` 出现在 `self: Point` 的位置，`b` 出现在
`b: Positive(b)`
的位置，两者的类型标注都引用了参数自身。区别仅在于类型标注的复杂度，机制完全一样——名字绑定后，类型可以依赖这个名字。

返回类型同样使用显式函数调用：

```yaoxiang
Sorted: (arr: Array(T)) -> Type = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }

sort: (arr: Array(T)) -> (result: Sorted(result)) = { ... }
//                        ^^^^^^^^^^^^^^^^^^^^^^^
//                        result 是返回值形参，Sorted(result) 是函数调用
//                        编译器在 return 点将返回值代入 result，验证 Sorted(返回值)
```

同样适用于局部变量声明：

```yaoxiang
let x: Positive(x) = 5
// x 绑定为 5，Positive(5) → { 5 > 0 } → True → 通过

// let y: Positive(y) = 0
// y 绑定为 0，Positive(0) → { 0 > 0 } → False → 编译错误
```

#### 3.2 路径条件收集

当运行时值出现在条件分支中时，编译器自动收集路径条件，形成当前作用域的**假设集合**。这些假设作为编译期 Bool 求值的背景知识参与验证。

```yaoxiang
if y > 0 {
    // 编译器在此分支内自动获得假设：{ y > 0 }
    let result = divide(x, y)
    // 验证条件：(y > 0) ⇒ (y > 0)
    // 证明管道判蕴含成立 → Proved
} else {
// 此分支假设：{ !(y > 0) }
// 如果要调用 divide(x, y)，验证条件为 !(y > 0) ⇒ y > 0
    // 证明管道判不蕴含 → Disproved
}
```

这不是编译器写死特殊模式——这是编译期证明管道的自然行为。每次类型检查调用点向管道发送：

```
{背景假设} ⇒ {验证目标}
```

证明管道判断蕴含性。Proved → 通过，Disproved → 编译错误 + 反例，Unproven
→ 编译错误 + 未解命题。背景假设来自当前程序点的路径条件。

#### 3.3 假设栈

编译器在分析控制流时，为每个基本块维护一个假设集合：

- **if-guard**：`if y > 0` → true 分支压入 `y > 0`，false 分支压入 `!(y > 0)`（如果使用了 else）
- **match 模式**：`if let Some(v) = opt` → 分支内压入 `opt == Some(v)`
- **逻辑连接**：`if x > 0 and y < 10` → 分支内压入 `x > 0` 和 `y < 10`
- **函数前置条件**：调用 `divide(a, b)` 时，`b` 必须满足 `Positive`
  的证据要么来自当前假设，要么来自实参自身的精化类型标注（`b` 已被标注为 `Positive` 则其类型携带
  `b > 0`）
- **赋值**：`let z = y` 时，`y` 上已有的精化条件传递到 `z`

所有假设进入 `Certificate` 的 `context` 字段（`Γ`），随候选证书一并交给 `Kernel::check`；producer 需要 SMT 时，才把同一份 `Γ` 翻译为 SMT-LIB 背景断言。

#### 3.4 无静态证据则编译错误

如果程序员直接写：

```yaoxiang
divide_user_input: (x: Int, y: Int) -> Int = divide(x, y)
```

当前程序点没有 `y > 0` 的假设，实参 `y` 自身也没有 `Positive` 类型标注。验证条件为：

```
{} ⇒ { y > 0 }
```

管道返回 `Disproved`（不蕴含）→ 编译错误：

> 无法证明参数 `b` 在 `divide` 调用中满足 `Positive`。 `y`
> 来自函数输入，无已证明的界。考虑用 if 分支守卫调用：`if y > 0 { divide(x, y) }`。

YaoXiang 不接受运行时值直接进入精化类型参数而不提供静态证据。这不是限制——这是硬安全哲学的核心：凡 `Kernel::check` 在当前签名下验证不出证书的代码，不得通过编译。

#### 3.5 与统一管道的关系

路径条件传播不是额外的机制。它是编译期证明管道在控制流分析上的直接延伸：

| 阶段         | 职责                                                                                                     |
| ------------ | -------------------------------------------------------------------------------------------------------- |
| 路径条件收集 | 编译器控制流分析阶段，为每个基本块标注假设集                                                             |
| 验证条件生成 | 遇到需验证的类型约束时，合并路径条件 + 实参类型信息                                                      |
| 证明管道判定 | certificate producer 产出候选证书 → `Kernel::check(signature, certificate, proposition)` → 三值 |
| 结果         | `Proved` → 通过；`Disproved` → 编译错误 + 反例；`Unproven` → 编译错误 + 未解命题 + 补证据提示       |

没有新部件。没有特殊规则。路径条件就是 `LogicSignature` 的局部假设集 `Γ`，它与类型等式、借用约束共享同一条管道、同一套预算系统。

### 4. 证明内核：`Kernel::check`

所有编译期检查共享同一条管道、同一个**证明内核**。内核的唯一判定入口是：

```text
Kernel::check(
    signature   : LogicSignature,
    certificate : Certificate,
    proposition : Proposition,
) -> CheckVerdict<Certificate>
```

`check` 是**纯函数**：给定同一签名、同一证书、同一命题，它永远返回同一结果；不读求解器状态、
不读时钟、不读文件路径。它只做一件事——按 `signature` 的 `rules` 逐步重放 `certificate`
的推导步，检查每一步的规则实例、前提、上下文与替换是否合法，并核对最终结论是否就是
`proposition`。内核不搜索、不猜测、不补全证书；证书缺一步、用错规则、或结论对不上，都不得出
`Proved`。

这与 RFC-039 D62 的**统一项世界**是同一幅图：类型位置与值位置共享同一套编译期项；命题
`Proposition`、证明项 `Certificate`、以及签名里的 sorts/rules/axioms 全是这一套项，不另立
第二套证明语言。判等（§12.1）是纯判定，`q=0` 槽的证据（RFC-011 §4.1）是调用侧义务，
Σ witness（§13）是构造点义务——三者都汇入同一个 `check`，不各开一条信任通道。

`CheckVerdict<Certificate>` 的三值与 §4.1 一致：`Proved(checked)` 携带经内核验证的证书，
`Disproved(model)` 携带反例，`Unproven` 携带未解命题与原因。只有 `Proved(checked)` 能使义务
通过；`checked` 是可被编译期缓存与增量复用的那一份证书。

```
编译期义务（命题 + 当前 LogicSignature）
        │
        ├── producer 编排（都不可信，见 §8）
        │   ├── 内核直判规则：结构等价、βδι-归约、令牌活性事实
        │   ├── Γ 等式桥 / 调用侧 q=0 证据（RFC-011 §4.1）
        │   ├── 显式证据：证明函数、axiom library / proof library 条目
        │   └── SMT / tactics / proof search：产出候选 Certificate
        │
        ▼
Kernel::check(signature, certificate, proposition)
        │  按 signature.rules 重放每一步；不用「unsat」顶替检查
        ▼
 ┌──────────────────────────────────────────────┐
 │ Proved(checked)   → 编译通过（证书编译期留存）│
 │ Disproved(model)  → 编译错误 + 反例           │
 │ Unproven          → 未解命题 + 原因 + 补证据  │
 └──────────────────────────────────────────────┘
        │
        ▼
Unproven → 提示补显式证据（证明函数 / 换签名 / 补公理）
        │
        ▼
再次 check ──→ Proved(checked) ──→ 编译通过
```

#### 4.1 `CheckVerdict`：三值代数与逻辑签名

`check` 返回三种结果——这是停机问题的必然结论，也是证明论的自然分划：

```text
CheckVerdict<C> = Proved(checked: C) | Disproved(model: Model) | Unproven(reason, subject)
```

- **`Proved(checked)`** → 停机，证书已按 `signature` 逐步验证，结论等于 `proposition`。义务通过；`checked` 是编译期可缓存的那一份证书（§13）。
- **`Disproved(model)`** → 停机，存在反例 `model`。编译错误 + 反例 + 源码位置。
- **`Unproven(reason, subject)`** → 在当前签名、证据与资源上限内，producer 没交出可验证的证书。编译错误 + 未解命题 + 原因 + **补证据提示**（§12.3 的诊断形状）。

**Unproven ≠ False。** 内核说「算不出来」不等价于命题为假——它只说明当前 `signature` 下、当前证据集里没有一份能通过检查的证书。这是诚实，不是缺陷。

**逻辑签名 `LogicSignature`** 决定「什么算一步有效推导」。它是一个可替换的数据：

```text
LogicSignature {
    sorts        : Map<SortName, SortDecl>          // 项与命题的类别
    rules        : Map<RuleId, InferenceRule>       // 内核可重放的推导规则
    axioms       : Map<AxiomId, Proposition>        // 本签名接受的公理
    libraries    : [LibraryRef]                     // 全部 axiom/proof 库，见下
    hash         : ContentHash                     // 覆盖 sorts/rules/axioms + libraries 全部条目
}
LibraryRef {
    kind         : LibraryKind                     // Axiom | Proof
    id           : LibraryId                       // 库内条目 id 的命名空间
    manifest     : Manifest                        // 条目 id + 命题/规则形态 + 来源
    hash         : ContentHash                     // 对 manifest 规范字节的内容哈希
}
SortDecl {
    space        : SortSpace                       // Proof | Data，见下
    arity        : [SortName]                      // 该 sort 构造子接受的实参 sort
}
SortSpace = Proof | Data
```

`sorts` 里每个 `SortDecl` 带一个 `space`，它是**「一个类型族应用是命题还是数据」的唯一判据**。`space = Proof` 表示该 sort 的 inhabitant 是证明项（命题 / 精化类型 / proof family 的结果），`space = Data` 表示 inhabitant 是运行期数据（`Int`、`Array`、`Vec`、普通记录）。`Pos`（`{ v > 0 }` 一类精化谓词）、`Assert`、`Terminates`，以及返回 `space=Proof` 类型的 `proof/type family`，其 `space=Proof`；`Array: (T: Type, N: Int) -> Type` 这类**数据类型族**的 `space=Data`。`space` 由库 manifest / 谓词声明登记，不由字段名、实参形态或「看起来像谓词」的语法猜；§14.1 的 evidence 分类只读这一个字段。数据类型族即使依赖严格前序字段，结果仍是数据字段。

`libraries` 是**列表**：一个签名可同时采用多个 axiom library 与多个 proof library（例：内置 `linarith` 公理库 + 用户 `zfc-core` 公理库 + `std-proofs` 证明库 + 项目私有证明库）。每个库自带 `kind`、`id`、`manifest` 与 `hash`；`axioms` 里的每一条按 `LibraryId` 归属到某个 `kind=Axiom` 的库，证明库条目按 `LibraryId` 归属到 `kind=Proof` 的库。`signature.hash` 覆盖 `sorts/rules/axioms` **与 `libraries` 的全部条目**（不止一个）；增删任一库、改任一 manifest 的一个字节，都换一个签名，缓存键、证书与增量复用全部按 `hash` 失效。

- `sorts`、`rules`、`axioms` 是签名本体；`axioms` 里的每一条都必须归属到 `libraries` 中某个 `kind=Axiom` 库的 `manifest` 登记条目，证书引用任一库登记外的公理 ⇒ `check` 拒绝。
- `libraries` 里每个 `LibraryRef` 各自有 manifest 与内容哈希：manifest 列出条目 id、命题/规则
  形态与来源，`hash` 对 manifest 的规范字节计算；`kind` 区分 axiom library 与 proof library。
  签名 `hash` 覆盖 `sorts/rules/axioms` 与 `libraries` **全部** `LibraryRef`（不是两个）。
- 内置库（`Int` 线性算术、`Array` 长度、`Assert` 桥接等）也是 `libraries` 里的普通 `LibraryRef`，
  只是随编译器分发；它们与用户库、项目库同列，**不在内核里硬编码信任根**。信任来自
  「该签名及其 `libraries` 列表被本次编译采用」这一事实，不来自某个库或求解器的名字。

签名可替换，且替换是数据替换、不是换内核：

| 签名 | sorts / rules 的取值 | 典型用途 |
| --- | --- | --- |
| 经典算术片段 | 线性算术 sorts + 无量词规则 | `x > 0`、`0 <= idx < arr.len` |
| 经典一阶（含排中） | 经典 sequent 规则 + 排中公理 | 经典蕴含、反证 |
| 构造主义 | 构造规则，无排中、无双否消去 | 提取见证、避免非构造证明 |
| ZFC 片段 | 集合论 sorts + 选择/外延等登记公理 | 需要集合论词汇的命题 |

`Proved` 永远是**签名相对**的：同一命题在经典签名下 `Proved`、在构造主义签名下可能 `Unproven`。
证书携带 `signature_hash`，`check` 先验哈希一致再重放；哈希不符直接 `Unproven`，不尝试
「大致兼容」。

**跨签名必须显式。** 设 `A`/`B` 为两个签名、`P` 为 `A` 中的命题、`certA` 为 `A` 下已验证的证书，要把结论用到 `B`，只有三条合法路径，每条都必须留下可检查的痕迹：

1. **翻译项**：给 `translate: A.Proposition → B.Proposition` 及其在 `B` 下经 `check` 验证的证书；
   得到 `Q = translate(P)`，再在 `B` 下证 `Q`。
2. **桥接公理**：在 `B.libraries` 的某个 `kind=Axiom` 库里登记一条显式桥接公理（如「`A` 的该规则在 `B` 的该 sort 上有对应解释」），证书引用 `(LibraryId, AxiomId)`；`B.hash` 随该登记变化。
3. **重新证明**：在 `B` 下对 `P` 直接构造新证书。

绝不允许「`A` 下 `Proved` ⇒ `B` 下 `Proved`」的隐式搬运，也不允许把 `certA` 换个签名哈希就复用。Σ 的跨模块导出（§13）与 `q=0` 证据的跨模块形态同样走这三条路径。

预算硬限制是停机问题的工程解（数值见 §8.4）。不给 knob——给了就是在问用户「你觉得你的程序
会不会停机」，用户不知道，编译器也不知道。预算耗尽归入 `Unproven(reason=BudgetExceeded)`，
不是 `Disproved`。

#### 4.2 `Certificate`：签名相对的证明项

**证书**是「证明项」在编译期的表示，相对某个 `LogicSignature` 才有意义：

```text
Certificate {
    signature_hash : ContentHash            // 必须等于 signature.hash
    proposition    : Proposition           // 结论，统一项世界里的项
    steps          : [ProofStep]           // 按 signature.rules 的推导序列
    evidence_refs  : [EvidenceRef]         // 显式证据的引用
}
ProofStep {
    rule       : RuleId                   // 必须命中 signature.rules
    premises   : [StepId | AxiomId | GammaId | EvidenceId]
    context    : Γ                       // 局部假设（路径条件、等式桥、q=0 证据）
    conclusion : Proposition
    subst      : Substitution            // 项替换，统一项世界的项
}
EvidenceRef = ExplicitProof(AstNodeId)     // 证明函数 / 用户写的显式证据
            | LibraryAxiom(LibraryId, AxiomId)
            | LibraryProof(LibraryId, ProofId)
            | ProducerHint(bytes)          // SMT/tactics/proof search 的提示，不直接信任
```

`check` 对每个 `ProofStep` 做四件机械的事：① `rule` 在 `signature.rules` 中存在；
② `premises` 指向早先步骤、登记公理、`Γ` 事实或显式证据，且各自可解析；
③把 `subst` 应用到规则模式后与 `conclusion` 同一（判等按 §12.1 的纯判定）；
④最后一步的 `conclusion` 与 `certificate.proposition` 同一。任何一件不成立 ⇒ `Unproven`，
并指出是哪一步；**不因为 producer 说「unsat」就跳过这四件**。

`ProducerHint` 只是提示：producer 可以把它附在候选证书里（例如 SMT-LIB 脚本、模型、tactic
轨迹），内核既不执行它，也不把它当规则；提示只服务于「如何把候选补全成可检查的 `steps`」。
补全失败就还是 `Unproven`。

#### 4.3 Unproven 之后：补显式证据（证明函数是第一类）

当内核返回 `Unproven` 时，诊断必须提示**补证据**，而不是把「算不出」渲染成「不成立」（诊断形状见 §12.3）。补证据的合法形态按「第一类」对待——它们就是统一项世界里的证明项，不是外挂：

- **证明函数**：一个 YaoXiang 函数，其返回类型等于待证命题。它是 `ExplicitProof(AstNodeId)`，内核验证它与验证 `add(a, b): Int` 是同一个机制。
- **axiom library 条目**：在签名登记的公理，`EvidenceRef::LibraryAxiom`。
- **proof library 条目**：命名证明（可复用、可单测、带 manifest hash），`EvidenceRef::LibraryProof`。
- **已有内核证书**：同一 `signature_hash` 下已 `Proved(checked)` 的证书，直接复用或作为前提。
- **换签名 / 加公理 / 写翻译项**：按 §4.1 的跨签名三路径，显式登记后重新 `check`。

```text
命题    = 类型（统一项世界里的 Proposition）
证明    = 程序（该类型的一个值，即 Certificate 的 steps）
验证    = Kernel::check(signature, certificate, proposition)
信任    = 该签名被本次编译采用 + 证书逐步可重放；不来自求解器
```

SMT/tactics/proof search 产出的候选证书若补全不出 `steps`，回退到 `Unproven`——程序员补显式证据，或让 producer 换一条搜索路径。不需要 AI，不需要导出到 Coq，不需要新概念。

```yaoxiang
# 命题：编译器无法自动证明的精化属性
FirstIsMin: (T: Ord, arr: Sorted(T)) -> Type = {
    forall i in 0..arr.len: arr[0] <= arr[i]
}

# 显式证据：程序员写一个函数，返回类型就是上面的命题
# 内核验证它——和验证 add(a,b): Int 完全一致
first_is_min: (T: Ord, arr: Sorted(T)) -> FirstIsMin(T, arr) = {
    ...
}
```

**编译期无法自动证明的属性 → 程序员补显式证据（一类证明项）→ 内核按签名验证。** 整个过程是平滑的梯度——producer 替你找简单的证书，把脑子留给难的。

#### 4.4 管道内的分层依赖

以上 producer 共享同一个 `Kernel::check` 接口，但存在编排顺序。类型等式是所有后续分析的先决条件；
所有权/令牌检查依赖类型信息；精化谓词验证依赖前两层的结果。编译器按层编排，底层失败的表达式
不进入上层——避免在类型错误的程序上浪费搜索预算；**每一层的结论仍必须由 `check` 验证，层序只
决定先向 producer 要哪份候选证书**。

```
编排顺序（同一内核，分层调度）
├── 第 0 层：类型等式（T1 == T2）
│   └── 内核直判规则（defeq）→ 产出候选证书 → check
├── 第 1 层：所有权/令牌冲突
│   └── 流敏感活性事实 → 产出候选证书 → check
└── 第 2 层：精化谓词/霍尔蕴含
    └── producer（Γ 证据 / proof library / SMT 搜索）→ 候选证书 → check
```

每一层仍返回 `Proved/Disproved/Unproven`，共享同一接口、同一 `signature` 与同一预算系统。

### 5. 三层函数统一

| 层级       | 运行时机 | 输入    | 输出 | 示例                                           |
| ---------- | -------- | ------- | ---- | ---------------------------------------------- |
| 值级函数   | 运行时   | 值      | 值   | `add: (a: Int, b: Int) -> Int = a + b`         |
| 类型构造器 | 编译期   | 类型/值 | Type | `List: (T: Type) -> Type = { data: Array(T) }` |
| 编译期谓词 | 编译期   | 值      | Type | `Positive: (x: Int) -> Type = { x > 0 }`       |

全部使用相同的 `name: type = value` 语法。编译期谓词和类型构造器走同一条编译期证明管道——`{}`
是证明空间。

### 6. 循环：Floyd-Hoare 验证条件生成

循环不需要单独的 `: Invariant(...)` 或 `: decreases(...)`
标注。变量上的编译期谓词类型标注定义了 Floyd-Hoare 风格的断言——编译器从类型标注生成验证条件，证明管道检查每个赋值是否保持类型。

核心机制：每个赋值操作对应一个 Hoare 三元组 `{P} x := e {Q}`，验证条件是
`P ⇒ Q[e/x]`。编译器对循环体生成一次验证条件——证明管道验证归纳步成立后，所有迭代自动覆盖。

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # 标注引用 i；初始化时 i=0，验证：0 == sum(arr[0..0]) → True
    mut i: UpTo(arr.len) = 0     # 验证：0 <= 0 <= arr.len → True
    while i < arr.len {
        # 编译器对循环体生成一次 VC。前提：s 满足 SumUpTo(arr, i)，i 满足 UpTo(arr.len)。
        #
        # s += arr[i]：
        #   验证义务：s_new 满足 SumUpTo(arr, i)（当前 i 不变）
        #   代入 s_new = s_old + arr[i]：
        #     需要 s_old + arr[i] == sum(arr[0..i+1])
        #     从归纳假设 s_old == sum(arr[0..i])，两边加 arr[i]：
        #     sum(arr[0..i]) + arr[i] == sum(arr[0..i+1])
        #   编译器 + SMT：线性算术，毫秒级 → Proved
        #
        # i += 1：
        #   i 变更 → 依赖图中 s 的类型标注引用了 i → 触发重验证
        #   新的验证目标：s 满足 SumUpTo(arr, i_new)
        #   即 s == sum(arr[0..i_new])，由上一步保证 → Proved
        s += arr[i]
        i += 1
    }
    return s  # 此时 s: SumUpTo(arr, arr.len)，即 s == sum(arr[0..arr.len])
}
```

循环不变式就是变量上的类型标注——程序员写类型，编译器检查归纳步。编译器不需要"发现"不变式，也不需要"自动做归纳"——它把归纳证明分解为每个赋值操作的局部验证条件，交给证明管道分而治之。

#### 6.1 依赖追踪：可变变量上的依赖类型

上述机制的前提是：编译器知道 `s` 的类型标注 `SumUpTo(arr, i)` 引用了 `i`——当 `i` 变更时，`s`
的类型约束也随之变化。这需要编译器维护**变量间的类型依赖图**。

**数据结构**：

```
TypeDepGraph: Map<VarName, Set<VarName>>
# 键是被依赖的变量，值是类型标注中引用了该变量的变量集合
# 例：{ i: {s}, j: {s, t}, ... }
```

**构建**：类型检查器在处理 `mut v: Pred(... x ...) = init` 时，解析 `Pred(...)`
参数中的自由变量引用。如果参数中引用了当前作用域内的其他可变变量 `x`，则在依赖图中记录 `x → v`。

**触发**：当被依赖的变量 `x` 被赋值时，编译器：

1. 查找依赖图中所有依赖 `x` 的变量 `{v₁, v₂, ...}`
2. 对每个 `v`，生成验证条件：`v 的当前值是否满足更新后的类型 Pred(... x_new ...)`
3. 将 VC 送入证明管道

**赋值顺序敏感**：依赖追踪自然强制执行正确的赋值顺序。以 `SumUpTo(arr, i)` 为例：

```yaoxiang
# 正确顺序
s += arr[i]   # s_new 满足 SumUpTo(arr, i+1)
i += 1        # i 变更 → 重验证 s 满足 SumUpTo(arr, i_new) → True

# 错误顺序——编译器拒绝
i += 1        # i 变更 → 重验证 s 满足 SumUpTo(arr, i_new)
              # s 尚未更新，s_old == sum(arr[0..i_old]) ≠ sum(arr[0..i_new])
              # → 编译错误：变量 s 不满足类型 SumUpTo(arr, i_new)
s += arr[i]   # 不可达
```

**组合依赖**：一个变量可以依赖多个变量。类型标注 `{ v: Int; v == x + y }` 同时依赖 `x` 和
`y`——任一变更都触发重验证。

**与证明管道的关系**：依赖追踪是 VC 生成的触发器，不是独立的验证机制。它回答"什么时候需要生成 VC"——证明管道回答"VC 是否成立"。

### 7. 终止检查

**适用范围：精化类型。**
终止性不是独立开关，而是**验证模式**的一部分：类型一旦被精化（`Refined { base, constraint }`），它所标注的那段计算就进入验证模式，模式内要求终止性；未被精化的普通类型不进验证模式，不生成任何终止义务。

由此得到两条推论：

- **循环**：裸 `while` 不进验证模式；度量变量带精化标注（如 `i: UpTo(n)`）时进验证模式，须证终止。
- **递归**：函数签名带精化（参数精化或返回类型含精化）时进验证模式，须证每个递归调用点的测度严格递减。

模式内全自动优先：编译器先自动探索测度，能证的通过；探索不出且未显式给出测度则编译错误。无**标注语法**口子——测度与终止命题都写在类型位，不引入
`decreases` 类新语法。测度与显式兜底的形态见 §6.9。

#### 6.1 设计原则

编译器从两处自动提取终止证明所需的信息：

1. **变量类型标注**：精化类型中的边界约束（如 `UpTo(n)` 给出上界 `n` 和下界 `0`）
2. **循环体操作**：每次迭代对变量施加的操作

编译器按优先级尝试四种度量合成策略，找到一个即停止。四策略是**测度探索的受限模板序列**，输入是精化约束（策略 1–4 均以「有界类型的变量」为起点），不是「编译期求值的代码」；它们与 §6.9 的显式测度是同一件事的自动与人工两面。

> **测度探索是探索，不是推断。**
> 探索只枚举模板（线性秩、违反计数、有界模式、乘法缩放），不保证存在解——通用测度推断一般不可判定（归约到停机问题）。因此模板外的情形必须能由程序员显式给出测度（§6.9），否则编译错误。

#### 6.2 策略 1：线性秩函数自动合成

当变量有线性界标注时，producer 枚举候选线性度量并产出递减义务的候选证书，由 `Kernel::check` 验证。

```
输入：
  变量 v₁: UpTo(u₁), v₂: UpTo(u₂), ...（有上下界的变量）
  循环条件 cond
  循环体中的赋值集合

算法：
  1. 从类型标注提取每个变量的界：[low_i, high_i]
  2. 枚举候选度量：v_i, u_i - v_i, v_i - v_j, 等线性组合
  3. 对每个候选度量 m：
     - 产出 m ≥ 0 的候选证书（从类型界推导）→ check
     - 对循环体的每条执行路径，产出 m' < m 的候选证书 → check
  4. 找到符合条件的线性组合 → 终止得证
```

覆盖范围：任意变量被赋值为线性表达式（`v = a·v + b`）且有界类型标注的循环。包括
`i += const`、`i -= const`、以及二分查找式的区间收缩：

```yaoxiang
# 二分查找：low = mid + 1 或 high = mid
# 度量 high - low 在两条路径上都严格递减
binary_search: (arr: Sorted(Int, arr), key: Int) -> Option(Int) = {
    mut low: UpTo(arr.len) = 0
    mut high: UpTo(arr.len) = arr.len
    while low < high {
        let mid = (low + high) / 2
        if arr.data[mid] < key { low = mid + 1 }
        else if arr.data[mid] > key { high = mid }
        else { return Some(mid) }
    }
    return None
}
```

#### 6.3 策略 2：谓词违反计数——从目标类型自动提取度量 <span style="color:orange">【实验性策略】</span>

> ⚠️ **当前状态：实验性策略，阶段 3 实现时根据实际可行性决定是否包含。**
> 此策略对相邻交换操作（冒泡排序、插入排序）有效，对非相邻操作（快速排序 partition、堆排序 sift-down）无法自动证明。覆盖边界见下表。如阶段 3 验证不可行，此策略将被移除或降级为未来工作。

核心洞察：**用户写的规约是编译器推理的素材。** 编译器不需要内置"什么是排序"——它读 `Sorted`
的定义，从定义中自动提取度量。

```
输入：
  目标类型：Sorted(arr) = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }
  循环体操作：相邻元素交换

算法：
  1. 解析谓词定义：forall i in range: cond(i, arr)
  2. 自动生成度量：violation_count = |{ i | ¬cond(i, arr) }|
  3. 分析操作对度量的影响：
     - 相邻交换 arr[j], arr[j+1] = arr[j+1], arr[j]
     - 只影响索引 j-1, j, j+1 三对
     - 如果 arr[j] > arr[j+1]（违反谓词），交换后这对满足谓词
     - violation_count 至少减 1
  4. 上界：n·(n-1)/2（最大相邻逆序数），下界：0
  → 终止得证
```

**当前覆盖范围**：

| 算法     | 操作模式       | 策略 2 可证明？ | 原因                             |
| -------- | -------------- | :-------------: | -------------------------------- |
| 冒泡排序 | 相邻交换       |       ✅        | violation_count 每次交换严格递减 |
| 插入排序 | 相邻移动       |       ✅        | 每次移位消除一个违反对           |
| 选择排序 | 非相邻交换     |       ❌        | 单次交换可能增加 violation_count |
| 快速排序 | partition 分区 |       ❌        | 非相邻交换，不保证单调递减       |
| 堆排序   | sift-down      |       ❌        | 树形操作，violation_count 非单调 |

**互补策略**：对于快速排序，`low < high`
区间收缩可由策略 1（线性秩函数）覆盖——外层的 partition 递归，每次区间减半。策略 1 和策略 2 互补覆盖，多数实际算法的终止性可由两者之一证明。但策略 2 的通用化（非相邻操作、树形操作）仍是开放问题。

```yaoxiang
sort: (arr: Array(Int)) -> (result: Sorted(result)) = {
    mut i: UpTo(arr.len) = 0
    while i < arr.len - 1 {
        mut j: UpTo(arr.len - i - 1) = 0
        while j < arr.len - i - 1 {
            if arr.data[j] > arr.data[j+1] {
                arr.data[j], arr.data[j+1] = arr.data[j+1], arr.data[j]
            }
            j += 1
        }
        i += 1
    }
    return arr
}
```

#### 6.4 策略 3：有界递增/递减模式

`v += const`（正常数），变量有上界类型标注 → 度量 `upper_bound - v` 每次递减
`const`，下界 0。这是策略 1 的退化情况，编译器在最前面快速处理。

#### 6.5 策略 4：乘法缩放度量模板

`v *= const`（const > 1），变量有上下界类型标注。编译器内置对数度量模板
`ceil(log_const(upper/v))`，每次乘 const 度量减 1。

```yaoxiang
mut i: Positive(i) = 1
while i < n {
    # 编译器自动推导：度量 ceil(log₂(n/i))，每次乘 2 度量减 1
    i *= 2
}
```

#### 6.6 终止性与正确性分离

终止性证明和正确性证明是独立的：

- **终止性**：以上四种策略自动证明循环在有限步内退出；探索不出时由程序员在类型位给出测度（§6.9）
- **正确性**：循环体是否朝目标类型推进，由编译期证明管道通过验证条件检查

两者都通过 → 编译通过。终止性得证但正确性失败 → 编译错误 + 反例。正确性得证但终止性无法证明 → 编译错误并指出无法分析的变量或操作。两者都失败 → 编译错误分别报告两项失败原因。

#### 6.7 递归函数的终止检查

对带精化签名的递归函数，编译器检查每个递归调用点的形参递减：

```yaoxiang
gcd: (a: Int, b: NonNegative(b)) -> Int = {
    if b == 0 { return a }
    return gcd(b, a % b)  // 编译器探索：(b, a % b) 的适定序测度递减 → 终止
}
```

形参递减是最强的自动路径（结构递归）。探索不出时，程序员在类型位显式给出测度（§6.9）。

#### 6.8 硬边界

`i = f(i)` 且 `f` 不可逆、不封闭、不保持任何单调性——数学上不可能自动证终止。编译错误：

> 此循环无法自动证明终止。循环变量依赖于不可分析的函数
> `f`。请使用可被编译器分析的迭代模式，或为该循环绑定一个名字并在类型位提供测度（§6.9）。

这不是编译器的失败。凡无法静态证明安全的代码，不得通过编译。即使给出显式测度，也必须为递减与良基性各交出一份经 `Kernel::check` 验证的证书——测度写得不对会被反例打回，人只能证不出来，不能证错。

#### 6.9 显式测度：`Terminates`

自动探索不出测度时，程序员把测度写在**类型位**——与 `Positive(b)`、`IsMax(T, arr, result)`
同一机制（谓词应用），零新语法：

```yaoxiang
// 测度：普通函数，可单测、可复用、不参与运行时
gcd_measure: (a: Int, b: Int) -> Int = { b }

// 终止分量长在函数自己的类型位
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

// 循环：绑定名即锚点，测度按名字获得作用域内的量
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n {
        i = i + 1
        i
    }
    return acc
}
```

循环这里的理由：`Terminates(m)` 精化的就是循环体尾表达式的值类型，锚点由绑定名 `acc`
提供——循环因此可被指称，不再有「匿名构造无从指称」的死角。

> **体尾须给出值**：循环的值 = 循环体块的值（§2.9），而块的值 = 尾表达式。
> 体尾若是赋值语句（如 `{ i = i + 1 }`），块值为 `Void`，`return acc` 便是从
> `-> Int` 返回 `Void`，类型不成立。故本示例的体尾显式写出 `i`。这不改变测度的
> 语义——测度 `n - i` 是**状态表达式**，与循环的值类型无关。

> **落地注记（2026-10-02 登记；2026-10-04 更新）**：本节的循环示例**类型侧**成立
> （`check` 0 error、回边义务 Proved）。**运行期取值缺陷（#409）已修复**：`while` 作值现在
> 取循环体末次迭代的体块尾表达式的值，与类型侧 `block_value_ty` 同口径——
> `loop(4) == 4`、`countdown(3) == 30` 实跑成立（`for` 同款修复）。
>
> **已知残留（零迭代）**：循环体一次都没求值时，没有「末次迭代的体值」，循环值仍取
> `Void`——绑定到非 `Void` 类型位并在运行期使用即报
> `E6007 Runtime error: type mismatch in comparison Eq: Void vs Int(0)`（响亮的类型不匹配，
> 不是静默错值）。这是**静态近似边界**：`block_value_ty` 判据给的是体尾表达式的类型，
> 对「循环是否至少执行一次」没有判定能力（需要数据流/可证性分析），故类型侧不擅自降级
> （降级会破坏本节 `Terminates` 循环绑定所依赖的值类型）。
> 该残留的契约由语料 `tests/yaoxiang/02-type-system/termination/while_zero_iteration_void_err.yx`
> （`// expect: runtime-error E6007`）钉住。**该残留已被确认为有意保留的设计边界**
> （2026-10-04 裁定，暂不修复），故 issue
> [#409](https://github.com/ChenXu233/YaoXiang/issues/409) 已随主形态修复关闭，不再单开跟踪项。
> 最小复现与影响面的历史记录见
> [RFC-027a](../accepted/027a-termination-explicit-measure.md) §示例 循环节「已知缺陷 D6」。
> 另：终止策略 1（线性秩函数）本轮恢复生效，故本节「自动探索不出才写显式测度」的顺序不变——
> 显式测度仍是探索失败后的兜底。

**`Terminates` 是内置谓词**，与 `Int`、`Never`
同属核心原语。它是唯一**由编译器代写函数体**的谓词——它的断言（「每个递归调用点/循环回边的测度严格递减」）长在计算结构里，用户写的谓词引用不到函数体或循环体，因此无法用「语法」节的谓词定义语法表达。内置面收敛为这一个名字。

**元数**。`Terminates(FnType, m)` 与 `Terminates(m)` 是同一个谓词的两个元数，不是两种构造：

| 形态                    | 锚点           | 用途                       |
| ----------------------- | -------------- | -------------------------- |
| `Terminates(m)`         | 所在绑定的名字 | 自递归函数、循环——默认形态 |
| `Terminates(FnType, m)` | 显式函数类型   | 互递归等锚点不唯一的场景   |

两者本质同一：终止义务总是落在「精化所在的类型位所标注的那段计算」上。

**测度不限制返回类型。**
测度可以是任意类型的表达式（不强制自然数）；其上的「严格递减」由该类型上可用的适定序给出。测度是否良基（如返回
`Int` 时是否
`>= 0`）是一条**独立的义务**，与递减义务同样由 producer 产出候选证书、交 `Kernel::check` 验证；两者都推不出时，诊断**不直接拒绝**，而是建议检查方向（测度是否下界成立、递归参数是否真的朝该方向走）。

**与自动探索的关系**：显式测度不是另一条管线，而是 producer 探索失败后的输入。给出测度后仍由 producer 产出候选证书、交同一个 `Kernel::check` 验证递减与良基性；不成立则报错并附反例。

义务生成、判定管线、诊断方向、互递归的测度共享（SCC）等落地机制见
[RFC-027a: 终止检查的显式测度](../accepted/027a-termination-explicit-measure.md)。

### 8. Certificate Producer：SMT / tactics / proof search 都不可信

SMT 求解器、tactic 脚本、proof search 过程在本设计中归属同一个角色：**certificate producer
（证书生产者）**。它们不可信：编译器不把它们的返回值当证明，只把它们当「找到一份候选证书」
的提示。唯一可信的是 `Kernel::check(signature, certificate, proposition)`（§4）。

#### 8.1 信任模型

- **内核是验证者**：`check` 按 `signature.rules` 重放 `certificate.steps`（§4.2）。它不调用求解器、不解析 `unsat`、不读 producer 的日志。
- **producer 是搜索者**：给定 `signature` 与 `proposition`，producer 尽力交出一份 `Certificate`。它返回什么、说什么，都不构成 `Proved`。
- **禁止 `Unsat ⇒ Proved`**：`unsat` 至多说明「在 producer 自己的翻译与理论下，没找到否定模型」。
  它既不是一份证书，也不保证 `certificate.steps` 存在。把 `unsat` 直接映射成 `Proved` 等于把
  信任根外包给求解器，与 §4 的纯函数内核正面冲突。**本 RFC 明确禁止该映射**。
- **`sat` 也不是 `Disproved` 的充分条件**：`sat + model` 只是 producer 给出的候选反例；
  它必须变成 `Kernel::check` 可判的证伪证书（按 `signature` 的证伪规则重放，或由内核直判
  规则算出「两侧归一化后不同」，见 §12.1），才能出 `Disproved(model)`。翻译失真
  （例：整数字面量保真问题，issue #561）会让 `sat` 携带伪模型，裸信它会假拒绝合法程序。
- **`unknown` / 超时 / 预算耗尽 ⇒ `Unproven`**：诊断带未解命题、原因与补证据提示（§12.3），不得降级为内部编译器错误（E8001）。

#### 8.2 producer 到内核的数据流

```text
proposition + signature
        │
        ▼
┌───────────────────────────────────────────────┐
│ certificate producer（不可信）                  │
│  · 内核直判规则（结构等价、βδι、defeq）         │
│  · Γ 等式桥 / q=0 证据（RFC-011 §4.1）         │
│  · axiom library / proof library 条目           │
│  · 证明函数（显式证据，§4.3）                   │
│  · SMT-LIB 2.6 / Z3 / CVC5                     │
│  · tactics / proof search                       │
└───────────────────────────────────────────────┘
        │  候选 Certificate（steps 可能为空 / 只带 ProducerHint）
        ▼
Kernel::check(signature, certificate, proposition)
        │
        ├── steps 齐备且逐步合法、结论同一 → Proved(checked)
        ├── 证伪证书经 check 确认（或内核直判）→ Disproved(model)
        └── 证书缺步 / 规则未登记 / 哈希不符 → Unproven（补证据提示）
```

producer 可以并行跑、可以缓存、可以换后端；这些自由都不改变「必须交出可重放证书」这一条。
SMT-LIB 2.6 仍是翻译目标（ISO 标准，Z3 / CVC5 / MathSAT / Yices 原生支持），默认后端 Z3、
备选 CVC5、编译时切换——**SMT-LIB 是 producer 的输入格式，不是信任边界**。不做「通用求解器
抽象层」，也不做「通用 producer 抽象层」：producer 只有一条出口契约「交出 `Certificate`」。

#### 8.3 从 `unsat` 到合法证书：翻译规则登记

`unsat` 要变得有用，必须先把 producer 的理论折叠成 `signature` 登记过的规则。例如线性算术
片段在 `libraries` 的某个 `kind=Axiom` 库的 `manifest` 里登记一组 `linarith` 规则（含其适用 sort 与前提形态）；SMT-LIB
的 `unsat` 结果经一个 **proof-producing 适配器**展开成这组规则的 `ProofStep` 序列，再交
`check`。适配器本身也是不可信 producer：它产出的步骤照样逐步重放。

这条要求把「信任 Z3 的 unsat」改造成「信任 `linarith` 规则在签名里登记、且证书逐步可检查」。
翻译 bug 不再靠「其他测试会暴露」这种祈祷兜底——它产出的证书缺步或用错规则，`check` 当场给
`Unproven`。未登记理论的 `unsat` 一律不得进 `Proved` 通道。

在 proof-producing 适配器落地之前，允许**保守降级**：算术片段走内核直判规则（结构等价、常量
折叠、βδι-归约）与已登记的证明库条目；producer 的 `unsat` 只作为 `ProducerHint` 附在候选证书里，
不单独触发 `Proved`。降级面比裸信 `unsat` 小，不 silent pass。

#### 8.4 预算（生产者的，不是内核的）

预算约束 producer 的搜索，不约束 `check` 的正确性——`check` 是确定性的重放，步数由证书长度决定。

| 预算维度       | 默认值 | 说明                                                     |
| -------------- | ------ | -------------------------------------------------------- |
| 搜索步数       | 10,000 | producer 的求解/搜索步数上限；超了返回 `Unproven`      |
| 时间           | 100ms  | 单义务搜索时间上限                                       |
| 量词实例化深度 | 3      | producer 翻译的嵌套量词深度                              |
| 证书重放步数   | 硬上限 | `check` 的最大 `steps.len()`；超限 ⇒ `Unproven`    |

超预算返回 `Unproven`，编译错误 + 谓词位置 + 一行预算摘要（§12.3）。没有降级、没有运行时检查、
没有 silent pass。依赖类型预归约（`factorial(5) → 120`、`append([1,2],[3]) → [1,2,3]`）
是内核直判规则，不消耗 producer 预算。

#### 8.5 与三值结果、Γ、q=0 证据的关系

`Γ` 中的常量等式事实（§12）与 `q=0` 槽的四源证据（RFC-011 §4.1：字面量、值环境、单值钉住、
Γ 等式）都是 producer 可引用的 `EvidenceRef` 来源；它们不是另一条判定通道。§3.5 的「路径条件
就是背景知识」在此精确化为「路径条件是 `signature` 在本地的 `Γ` 上下文，随 `Certificate` 的
`context` 字段一并交给 `check`」。

#### 8.6 当前实现的缺口：`predicate.rs` 裸信 SMT

本节的设计与当前生产实现不一致，缺口如实登记，不假装已落地：

- `src/frontend/core/sema/layers/predicate.rs` 的 `try_implication` 把 `SMTResult::Unsat` 直接映射为 `Some(Ok(()))`；`try_smt_solve` 把 `SMTResult::Unsat` 直接映射为 `Ok(())`。`Ok(())` 就是 `ProofResult` 的 `Proved` 臂——这就是**裸信 `unsat`**，没有构造、也没有重放任何 `Certificate`。
- 同文件的 `SMTResult::Sat { model }` 直接构造 `ProofFailure::Disproved`，反例未经内核按签名验证；翻译失真时（issue #561）会假拒绝合法程序。
- `ProofContext`（`src/frontend/core/sema/proof/context.rs`）目前只携带 `env / assumptions / dep_graph / budget / dep_env`，**没有 `LogicSignature` 字段**；`ProofFailure`、`ProofResult` 是三值结果，但没有 `Certificate` 类型。
- 因此当前管道是「producer 的返回值 = 判定结果」，而目标管道是「producer 交证书 → `Kernel::check` → 判定结果」。二者不是措辞差异，是信任模型差异：前者把 Z3 的结果当信任根，后者把签名与内核当信任根。
- 修复方向（不在本文写代码）：① 引入 `LogicSignature` 与 `Certificate` 表示，并在 `ProofContext` 携带当前签名；② 把 `try_implication` / `try_smt_solve` 改造成「产出候选证书」，`unsat` 只作为 `ProducerHint`；③ 让 `Kernel::check` 成为 `Proved` 的唯一出口，`Sat` 反例也经 `check` 确认；④ 按 §8.3 登记 `linarith` 等翻译规则。落地按 RFC-039 D62 的「证据强制 / Π/Σ 完全体」阶段推进，本轮只定契约。

#### 8.7 wasm 与后端差异

wasm 下无 Z3（SMT 代码 `cfg(not(target_arch = "wasm32"))` 排除）时，producer 少一条搜索路径，
降级为 `Unproven`（保守、少报）。这与「签名是否一致」无关：同一 `signature` 下，native 多一个
producer 候选，wasm 少一个，`Kernel::check` 的行为完全相同。既有 native/wasm 差异表（§开放问题）
按「producer 可得性」解释，不再按「判定精度」解释。

程序员不需要知道 SMT 的存在。心智模型是：**编译器能按当前签名验证证书就通过，不能就报 Unproven 并提示补证据——如果自动 producer 找不到，你写个证明函数（一类显式证据）交给内核看。**

### 9. 编译期谓词组合

编译期谓词是返回 Type 的函数，组合通过函数组合自然实现：

```yaoxiang
SortedNonEmpty: (T: Ord, arr: Array(T)) -> Type = {
    Sorted(T, arr) and NonEmpty(arr)
}
```

### 10. 代码示例

#### 9.1 除法安全

```yaoxiang
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive(b)) -> Int = a / b

result = divide(10, 2)   # ✅ 编译器验证 Positive(2) = { 2 > 0 } → True
# result = divide(10, 0)  # ❌ 编译器验证 Positive(0) = { 0 > 0 } → False
```

#### 9.2 数组访问安全

```yaoxiang
InBounds: (idx: Int, arr: Array(T)) -> Type = { 0 <= idx and idx < arr.len }

get: (arr: Array(T), idx: InBounds(idx, arr)) -> T = arr.data[idx]

arr = Array(Int)(1, 2, 3)
x = get(arr, 1)   # ✅ 编译器验证 InBounds(1, arr) = { 0 <= 1 and 1 < 3 } → True
# y = get(arr, 5)  # ❌ 编译器验证 InBounds(5, arr) = { 0 <= 5 and 5 < 3 } → False
```

#### 9.3 排序正确性

```yaoxiang
Sorted: (T: Ord, arr: Array(T)) -> Type = {
    forall i in 0..arr.len-1: arr[i] <= arr[i+1]
}

sort: (T: Ord) -> ((arr: Array(T))) -> (result: Sorted(T, result)) = {
    result = arr.clone()
    # ... 排序算法实现 ...
    return result
}
```

#### 9.4 循环：编译器 VC 生成

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0
    mut i: UpTo(arr.len) = 0
    while i < arr.len {
        s += arr[i]
        i += 1
    }
    return s
}
```

### 11. dispatch 分派管道：编译期与运行时的统一分派

> **落地注记（2026-10-03 登记，#377-2）**：本节的**语义**成立且已接入生产，但**结构**不是独立的
> `layers/dispatch.rs` 模块。生产的分派由 `checker.rs` 的**三处调用点义务**实现——绑定位重验证
> （`revalidate_refined`）、调用点实参（`check_call_arg_refinements`）、返回位后置条件
> （`check_return_refinement`）；其中**直接调用** `layers::predicate::check_predicate` 的是**两处**：调用点实参与
> 返回位后置条件共用唯一判定单元 `judge_refined_obligation`（`checker/refinement.rs`），
> `revalidate_refined` 自带 E2030 口径。**第四处义务面在证明执行器**（`proof_execution.rs`，
> 2026-11-14 接线，issue #537）：执行证明调用时先展开**被调证明函数自身的形参精化**（RFC-027 §3.3
> 调用点前置条件的嵌套义务——广度优先工作清单，去重集 + §8 预算收口；闭合 Bool 约束判伪出
> E4018，Call 形态入队继续执行）。原
> `layers/dispatch.rs` 零生产调用点，且其对 `Unproven { ProofFunctionRequired }` 的处置是
> 「降级为 W1080 警告并注入 Γ」，与生产实做（`Unproven` 一律报错，见 `refined_unproven`）
> 相冲突，故已整体删除。另需注意：本节表中 Runtime 行的「插入运行时 check」在生产**尚无**
> 发射点（`RuntimeOutcome::InsertCheck` 无消费方），Γ 注入则由 `ownership.rs` 的分支守卫与
> `inference/expressions.rs` 独立完成。

`assert` 和 `Assert` 是同一个精化类型原语的两面。分派管道 `dispatch`
按**谓词的自由变量在编译期是否可及**自动决定走编译期证明还是运行时检查：

| 判据                                                   | 模式            | 行为                                                                |
| ------------------------------------------------------ | --------------- | ------------------------------------------------------------------- |
| 所有自由变量编译期已知（泛型参数、编译期常量）         | **CompileTime** | 进证明管道：certificate producer 交候选证书 → `Kernel::check` → Proved 擦除 / Disproved 编译错误 / Unproven 补证据 |
| 存在自由变量来自运行时（函数参数、外部输入、mut 变量） | **Runtime**     | 插入运行时 check，并向流敏感假设集 Γ 注入精化事实                   |

**关键**：「判不了」≠「证伪」。CompileTime 模式下 Unknown 要求证明（不静默降级），Runtime 模式下命题在编译期根本没有真假可言——prover 再强也无法为「用户可能输入了负数」写出恒真证明，运行时 check 是唯一 sound 选择。这不是 prover 不够强，是理论必然。

### 12. 流敏感假设集 Γ ：最强后置条件传播

编译器维护一个流敏感（flow-sensitive）假设集 Γ，跟踪每个控制流点已知成立的命题。

**SP（最强后置条件）传播**：

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP 传播
```

**mut 变量的 kill set**：`mut` 变量重新赋值后，涉及该变量的所有假设从 Γ 中移除：

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
mut x = x - 5       // Γ = {}  ← x > 0 被 kill
```

这是 soundness 的硬要求——变量值变了，旧假设无效。

**分支合流**：IF/ELSE 或 match 分支合并时，Γ 取各分支假设的交集。只有所有路径都成立的命题才带出分支。

**等式桥（q=0 槽的自动证据）**：Γ 中的常量等式事实是编译期值参数（RFC-011 §4.1，q=0 槽）的
合法证据来源。`if y == 42 { f(y)(y) }` 在守卫分支内，Γ 记 `y == 42` ⇒ y 在 q=0 槽处视作 42，
调用按 `f(42)` 的既有路径检查。等式的两端一侧为编译期常量；无等式证据的运行期值填入 q=0 槽
仍在调用点报告。这不是新机制——验证条件 `{y == 42} ⇒ {y ≡ 42}` 走的就是既有蕴含判定。

#### 12.1 判等的定义（defeq）

本节由 RFC-039 决议 D62 增补：上段的等式桥给出 q=0 槽的证据来源，本节补全判等本身的定义、Unproven 在类型等式上的落点与诊断形状。

判等是类型上的唯一等价关系。它只由一条规则给出：两侧各自归一化后**同一**。

- **归一化**由三类归约组成：δ 展开类型别名与定义；β 归约类型族与依赖类型应用；ι 把字面量类型规范化——字面量类型的身份只由值决定，源文本的拼写不参与（`03` 与 `3` 是同一类型）。
- **同一性**是全结构的：类型构造器同名、实参逐一同一。判等不读缓存键、不读渲染文本，也不对精化谓词做任何推理。
- **子类型是独立关系**：`A <: B` 由子类型规则判定；`A <: B` 与 `B <: A` 同时成立也不作为判等的定义。双向子类型会把协变与接口鸭子类型规则掺进等价，使「可赋值」与「等价」互相污染。
- **`Refined` 的谓词不同 ⇒ Unproven**：两侧基类型相同而谓词不同时，编译器返回 Unproven，不返回 Disproved。谓词等价属假设集 Γ 层（上段的等式桥与其后的 Γ 等式规则），该层落地前编译器只能说「判不出」。

判等的失败面与 §4.1 的三值代数一致：两侧归一化成功且同一 ⇒ `Proved`；两侧归一化成功但不同 ⇒ `Disproved`；任一侧归一化失败 ⇒ `Unproven`（预算耗尽或能力边界）。**归约失败不产生 `Disproved`**——算不出不等于不相等。

判等是纯判定：它**不绑定**类型变量，成功即接受，不留实例化。判定点（赋值、实参、返回位）先走统一；统一失败后允许改用判等，但必须先回滚统一尝试产生的部分绑定。不回滚，被判定为不等价的尝试会留下绑定并污染后续推断。

#### 12.2 Unproven ≠ False：类型等式的落点

§4.1 的三值代数适用于全部义务点，类型等式是其中之一。本节把「Unproven ≠ False」落到这一层。

- 义务点返回 `Unproven` 时，编译器**必须**报编译错误，不得放行。诊断必须提示补证据：精化域写证明函数或引用 proof library 条目（§4.3），等式域给出可归一化的形态；换签名 / 加桥接公理按 §4.1 的跨签名三路径。
- `Unproven` 与 `Disproved` 必须可区分：`Disproved` 带反例（具体赋值或不等价的两侧），`Unproven` 带未解命题与其原因。把 `Unproven` 渲染成「不成立」是错误报告。
- `Unproven` **不得**降级为内部编译器错误（E8001）。E8001 只表示编译器自身的一致性故障；预算耗尽与能力边界是编译器推理能力的边界，属用户域诊断（RFC-013「消息质量规范」5）。
- `Unproven` **不得**静默放行。判定点以失败收场时，用户看到的诊断只能来自既有失败路径；不新增第二条诊断，也不因判不出而接受。

#### 12.3 诊断形状

`Unproven` 的诊断必须携带三样，缺一不算完整：

- **未解命题**：可渲染的命题文本（精化约束、等式两侧的类型、终止测度）；
- **原因**：能力边界（超出内核推理规则）、预算耗尽、含未绑定符号名——三者之一；
- **位置**：源码位置。确无位置的判定点用默认位置，并在实现注释中写明理由。

预算细节按**一行摘要**呈现（例：「预算 10000 步，已用 10000 步」）。完整步数、求解器内部轨迹与失败形态进 verbose 通道或日志，不进默认输出。

本节是对 §4.1「Unproven → 编译错误 + 未解命题 + 预算消耗报告」的**形式放松**：实质不变——用户仍必须能区分「算不动」与「不成立」；放松的只是「预算消耗报告」的呈现粒度。理由：完整步数对用户无行动价值（用户无法据此修改程序），且随实现细节漂移；限缩为一行摘要后，诊断的稳定性与可读性提高，可区分性由「原因」一栏保证。

`Unproven` 的「补证据提示」属于诊断的**原因**一栏：能力边界说明超出当前 `signature.rules`，预算耗尽给一行摘要，含未绑定符号名指出待补证据的名字。一般性 `Unproven` 与精化专用码的分工见 RFC-013 的码表草案节（E4023 / E2031）。

### 13. 擦除模型：证书编译期留存、运行期擦除

三种「擦除」必须分清，它们作用在不同对象上：

| 对象 | 编译期 | 运行期 | 依据 |
| --- | --- | --- | --- |
| `Certificate`（经 `Kernel::check` 验证的证明项） | 留存：`Proved(checked)` 进编译期缓存，供增量复用与 Σ 构造义务核对 | 完全擦除：不产生运行时代码、不占数据槽 | §4.1、§4.2 |
| `q=0` 编译期值参数 / Σ witness 字段 | 作为量词事实与证据义务存在（§14） | 不占槽、`CreateStruct` 只含数据字段、值级 `b.w` 拒绝（E1114） | RFC-011 §4.1、D62 I4/I6、§14 |
| Runtime 模式下 dispatch 插入的 Bool check | 不是 witness、不是证书 | 保留：在值层面执行 | §11 |

**证书是编译期对象。** `check` 通过后，`Proved(checked)` 只活在编译期：它钉住「该义务已按 `signature.hash` 验证」，缓存键含 `signature_hash + proposition + certificate`，签名一换即失效（§4.1）。运行期不需要、也不得携带证书——类型层面的证明已擦除，`q=0` 槽与 Σ witness 同样不占运行期布局（D62 I4「擦除完备」、I6「witness 擦除 ≠ runtime check」）。

**Runtime check 与证书无关。** §11 分派到 Runtime 模式时插入的是值层面的 Bool 检查，它检查运行期值，不是内核验证过的证明项。保留它不违反「证书运行期擦除」；两者作用对象不同，互不替代：witness 擦除、证书擦除、check 保留。

**与 D62 统一项世界一致**：命题、证书、`q=0` 项、Σ witness 都是同一套编译期项；擦除是「编译期项不进入运行期布局」这一条后果，不是为证明项另开一套擦除规则。Σ 记录字段如何成为 evidence 义务、构造器如何产出证书、`b.w` 如何类型级使用，见 §14。§13 与 §8.6 的实现缺口并列记录：当前 `predicate.rs` 尚未产出 `Certificate`，故「证书编译期留存」尚未落地。

### 14. 依赖记录 Σ：evidence 字段、构造与类型级使用

Σ 依赖记录是统一项世界里的**记录类型**：后面的字段可以引用前面字段的名字。本节的记录是 §4 证明内核的一个义务来源——构造器必须为每条 evidence 字段交出经 `Kernel::check(signature, certificate, proposition)` 验证的证书；证书按 §13 编译期留存、运行期擦除。

本节给出完整语义：位置规则（§14.1）、构造与布局（§14.2）、泛型（§14.3）、跨模块导出（§14.4）、类型级使用（§14.5）、不可变与诊断（§14.6）。泛型、跨模块、类型级使用都是本节当场定义的语义，不登记为后续。

#### 14.1 字段位置规则：evidence 的 proof-space 判据

设记录声明 `R` 的字段按源码顺序为 `f₀, f₁, …, fₙ₋₁`，字段 `fᵢ` 的类型为 `Tᵢ`。判定分三步：**先定 `space`（是命题族还是数据族），再定实参位置（前序 / 自身 / 前向），最后定字段类别**，不做跨字段推断、不按字段名猜：

**第一步（proof-space 判据，唯一判据）。** 把 `Tᵢ` 归一化，取其结果类型族的 `SortDecl.space`（§4.1）：

- `space = Proof`（`Tᵢ` 归约后是命题 / 精化类型 / `Assert` / `Terminates` / proof family 结果）⇒ 进入 evidence 判定；
- `space = Data`（`Tᵢ` 是 `Int`、`Array`、`Vec`、普通记录，或**数据类型族应用**）⇒ **数据字段**，无论它引用了几个前序字段。

`Array(Int, n)` 的 `n` 引用前序数据字段，改变的是「数据槽的长度」，不是「生成一条证明」。把它判成 witness 会把缓冲区从运行期布局里抹掉——这是本节要根除的误分类。`space` 由 `signature.sorts` / 库 manifest 登记，`Kernel::check` 与字段分类读同一份登记。

**第二步（位置判据）。** 仅对 `space = Proof` 的 `Tᵢ = P(a₀,…,aₖ)` 生效，按实参指向的位置分三种：

- 实参全是**严格前序数据字段** `fₕ (h < i)`、**编译期常量**、**编译期值参数**（RFC-011 §4.1 / SPEC §4.3 的 `Matrix` 的 `Rows`、`Cols` 一类 q=0 参数）或**类型参数** ⇒ 第三步定为 evidence。编译期值参数在实例化后取到具体常量，按编译期常量计（与 SPEC §8.3.1 逐字一致）；未实例化的开项留到构造点判 `Unproven`（§14.3）；
- 实参**就是 `fᵢ` 自身**（自引用精化，如 `v: Pos(v)`）⇒ 第三步定为**数据字段（admission）**：该字段是承载值的变量，`Refined` 约束只是对它自身取值的检查，不是对别处事实的证明；
- 前向引用后序字段、引用另一个 evidence 字段、或把 `fᵢ` 自身当 evidence 实参 ⇒ 声明期拒绝。

**第三步（类别判据）。** `space=Data` ⇒ 数据字段；`space=Proof` + 前序实参（含编译期值参数 / 编译期常量 / 类型参数）⇒ evidence；`space=Proof` + 自引用 ⇒ 数据字段（admission）。

| # | 形态 | 判定 | 量词 | 运行期布局 |
| --- | --- | --- | --- | --- |
| 1 | `space=Proof` 且实参全为严格前序数据字段 / 编译期常量 / 编译期值参数 / 类型参数（正例 `w: Pos(v)`） | **evidence / witness 字段** | `Some(Quantity::Zero)` | 不占槽、不进 `CreateStruct`、`base` 不进运行期布局 |
| 2 | `space=Data`：普通值类型（`n: Int`）、数据类型族应用（正例 `data: Array(Int, n)`）、普通记录 | **数据字段** | `None` | 占数据槽、进 `CreateStruct` |
| 3 | `space=Proof` 但实参就是 `fᵢ` 自身（自引用精化 `v: Pos(v)`） | **数据字段（admission）** | `None` | 占数据槽；`Refined` 约束在构造点按 §3.1 / §3.3 既有义务检查 |
| 4 | `space=Proof` 但**前向引用**后序字段 `fₕ (h > i)`、引用另一个 evidence 字段、或把 `fᵢ` 自身当 evidence 实参 | **非法**：声明期拒绝，不静默降级 | — | 不产生任何字段 |

四行互不混淆：第 2 行是「普通值类型 / 数据族」一律数据字段；第 3 行是「命题族但自引用」的 admission 数据字段；第 4 行是「命题族但位置不合法」一律声明期拒绝。**不存在「前向引用也算数据字段」这条路径**——前向引用既不占数据槽，也不进 `runtime_field_index`。

**witness 只能依赖严格前序数据字段。** evidence 字段 `fᵢ: P(fₕ,…)` 的实参不得引用另一个 evidence 字段、不得前向引用后序字段、不得引用 `fᵢ` 自身；`P` 的 kind 判据由 §14.3 给出。这条「proof-space + 严格前序 + 只吃数据字段」保证构造点按声明序走到 `fᵢ` 时，所有实参已有编译期值，命题是闭项，可交 `check`。

**正例（evidence）与反例（数据族）：**

```yaoxiang
Pos: (v: Int) -> Type = { v > 0 }        // space=Proof

Box: Type = {                            // 正例：w 是 evidence
    v: Int,          // f0 数据字段
    w: Pos(v),       // f1 evidence：P=Pos 是 Proof, 实参 v 是严格前序数据字段 => q=0
    z: Int,          // f2 数据字段；运行期索引 = 1 (跳过 w) 
}

Buf: Type = {                            // 反例：data 仍是数据字段
    n: Int,                 // f0 数据字段
    data: Array(Int, n),    // f1 数据字段：Array 是 Data 族, n 只决定长度, 不生成证明
                            // 运行期索引 = 1, 占槽、进 CreateStruct
}

Admission: Type = {                      // 自引用精化：数据字段, 不是 evidence
    v: Pos(v),              // Proof 族但实参是自身 => admission (表第 3 行) 
}

// 编译期值参数作 evidence 实参 (RFC-011 §4.1 / SPEC §4.3、§8.3.1) 
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),   // Rows 是编译期值参数 (q=0) => evidence
    data: Array(Array(T, Cols), Rows), // Array 是 Data 族 => 数据字段
}
```

`Matrix` 的 `Rows`、`Cols` 是编译期值参数（实例化后取具体常量），`Assert(Rows > 0)` 的 `space=Proof` 且实参为编译期值参数 ⇒ `_assert_rows` 是 evidence 字段；`data` 引用同样的编译期值参数，但 `Array` 是 `space=Data` ⇒ 仍是数据字段。B.2 判定器对「编译期值参数」的判据因此明确：看它是否在类型位被引用（RFC-011 §4.1 的强度声明），不按字段名猜。

**`base` 不进运行期布局**：`w: Pos(v)` 的 `w` 不是「一个 Int 值」，而是「`Pos(v)` 的证明」。它的 `Refined.base`（`Int`）只是内核判等/归一化的类型信息，绝不为 `w` 分配运行期槽位。`StructType.fields` 用 `Vec<StructFieldInfo>{ name, ty, quantity, is_mut, has_default }` 单列表示：`quantity == Some(Quantity::Zero)` 为 witness，`None` 为数据字段；`None` 在此表示「数据字段」，不是「未解析」。

表示层接口：`ast::StructField.quantity`、`mono::StructFieldInfo.quantity`、`ir::TypeDeclField.quantity` 三处同型；`mono::quantity_is_witness` / `StructFieldInfo::is_witness` / `is_witness_bearing` 是唯一判定；`mono::runtime_field_index` 是声明序 → 运行期数据序的**唯一映射点**（witness 命中返回 `None`）。分类前的 `space` 查询读 `signature.sorts` / `LibraryRef.manifest`，不是第二套分类表。

#### 14.2 构造器：源级 data-only，编译期自动合成证据

**源级构造只有一个形态：`Box(v, z)`（数据参数，按声明序）。** evidence 字段**不占任何构造实参位**——既没有「把 witness 当值传进去」的写法，也没有「证据参数位」；`Box` 的 arity 只数数据字段 `v` / `z`，不数 `w`。**证据不来自构造实参，来自构造点自动生成的证明义务**。三个视图由此分清：

| 视图 | 形态 | arity | 谁产出证据 |
| --- | --- | --- | --- |
| 源级 / 编译期构造视图 | `Box(3, 5)` | **compile-time arity = runtime arity = 数据字段数（此处 2：v、z）** | 构造点自动生成 `FieldObligation`，producer 交候选 `Certificate` → `Kernel::check` |
| 运行期构造视图（`translator::synthesize_constructor` / `CreateStruct`） | `Box(3, 5)` | runtime arity = 2 | 无：`CreateStruct.operands` 只含数据字段局部槽 `v=3, z=5`（`z` 的运行期索引 = 1，跳过 `w`），`local_count` 按数据字段数计，没有 `w` |

自动合成的路径：把严格前序数据字段的编译期值代入 `P` 得到闭命题，producer（§8：内核直判规则 / `Γ` 证据 / proof library / SMT 搜索）交候选 `Certificate`，`Kernel::check(signature, certificate, proposition)` 验证；`Proved(checked)` 按 §13 编译期留存、运行期擦除。

**显式证据不是构造实参，是独立声明。** 当自动 producer 交不出可验证证书（`Unproven`）时，程序员按 §4.3 补显式证据——写一个**独立证明函数**（返回类型等于待证命题）或在 `proof library` 登记命名证明，作为 `EvidenceRef::ExplicitProof` / `LibraryProof` 被 `FieldObligation` 引用；不改写构造表达式、不新增 `Box(..., proof)` 形态。**源级构造多传一个证据实参**（`Box(3, 5, evidence)`）**不允许**：按 D60 六条绑定规则，多出的实参落不到任何数据形参槽，报 E1010 / E1014 一族参数错误，不静默吞掉。

`FieldObligation` 必须能同时表示「还没产出证书」与「已验证证书」两种状态，故把原 `evidence: EvidenceRef`（只能指显式证据，装不下 producer 合成的 `Certificate`）拆成两槽：

```text
FieldObligation {
    record         : TypeId,
    field          : FieldId,
    proposition    : Proposition,
    evidence_ref   : Option<EvidenceRef>,   // 程序员补的显式证据；自动合成时为 None
    certificate_ref: Option<CertificateRef>,// producer 合成并经 check 的证书；未验证时为 None
    signature_hash : ContentHash,
}
CertificateRef { id: CertId, signature_hash: ContentHash }  // 指向 §13 编译期缓存里的 Proved(checked)
```

两槽的合法组合：`(Some(ev), None)` = 显式证据待重放；`(None, Some(cr))` = 自动合成且已验证；`(None, None)` = 待 producer 搜索；`(Some, Some)` 非法（一条义务不消费两份证明）。`check` 通过后写 `certificate_ref`，`evidence_ref` 保留为来源标注。

`FieldObligation` 在 `checker/refinement.rs` 的 refined walk 里生成；字段绑定复用 `bind_args_to_params`（D60 六条规则），判定复用唯一单元 `judge_refined_obligation`。**证不出时不补值**：按 §4.3 补显式证据或返回 `Unproven`（E2031 / 一般性 E4023），没有「写错的 witness 值」这回事。

自引用精化字段（`v: Pos(v)`）是数据字段：它的 `Refined` 约束在构造点按既有调用点义务判定（§3.1 / §3.3），不是 evidence 字段，不产生 `FieldObligation`，也不改变 `runtime_field_index`。

#### 14.3 泛型记录：proof family 参数 `P`（kind `T -> Proposition`）

类型是「参数化记录 + **proof family 参数**」，不是「泛型 witness 留待后续」。`P` 的 kind 与 §14.1 的 proof-space 判据同源：**结果 sort 必须是 `space=Proof`**，即 `P` 是 proof/type family（命题族），不是 data family（数据类型族）。`T -> Proposition` 是 kind 记法（`Proposition` 是 §4.1 `SortSpace::Proof` 的 sort 别名），**不是源级类型名**：下面的 `yaoxiang` 块用当前解析器可接受的 `P: (T) -> Type` 形态，kind 约束在声明与实例化处判定，不写进可解析代码块。

```yaoxiang
Pos: (v: Int) -> Type = { v > 0 }

// 源级形态：P 是类型级函数参数；kind 约束 (结果 space=Proof) 在正文说明
Cell: (T: Type, P: (T) -> Type) -> Type = {
    v: T,            // 数据字段
    w: P(v),         // evidence：P 是 Proof 族, 实参 v 是严格前序数据字段
}
```

- **kind 判据**（正文记法，不是源级语法）：`P: T -> Proposition` 中 `Proposition` 是 `SortSpace::Proof` 的 sort 别名（§4.1）；等价于 `P` 的应用结果 `space=Proof`。`Array: (T: Type, N: Int) -> Type` 的 `space=Data`，把它传给 `Cell` 是 **kind 不符，声明期拒绝**（参数声明的 kind 就是 `T -> Proposition`，绑定 data family 是类型错误，不是「`w` 退化成数据字段」；§14.1 第 2 行只管**直接写出**数据类型族的字段）。多元形态 `(T, U) -> Proposition` 同理，实参按声明序全部落在严格前序数据字段上。
- `P` 与 `T: Type` 同属编译期参数位（q=0，RFC-011 §4.1）。实例化 `Cell(Int, Pos)` 时 `P` 绑定到具体谓词 `Pos`（`space=Proof`），`w` 的命题变成 `Pos(v)`，位置规则与 §14.1 逐字相同；`Cell(Int, Array)` 在声明/实例化处因 kind 不符拒绝。
- `P` 不是值参数、不占运行期槽；`P` 未绑定（泛型未实例化）时 `w` 的结果 sort 仍是 `Proof`（kind 已声明），但命题是开项，**构造点返回 `Unproven`**（不是声明期拒绝），不静默放行、也不退化成数据字段。实例化后判定按 §14.1 重跑。
- 泛型记录的 evidence 判定在**实例化后**按 §14.1 重跑一次：`StructType` 的 `quantity` 随 `substitute` 代入 `P` 后仍为 `Some(Zero)`；`mono::substitute` 与 `types::solver` 的 Struct 臂按字段透传 `quantity`，不另造第二份判定。
- 泛型 `Cell` 的导出随 `TypeDefPayload` 携带 `P` 的 kind（`T -> Proposition` 记法）与字段 obligation 模板（§14.4）；源级 `(T) -> Type` 与 kind 记法的对应由声明/实例化处的判据给出。

#### 14.4 跨模块导出：`TypeDefPayload` 携带命题/证据槽

含 evidence 字段的记录跨模块使用时，命题与证据槽必须随类型定义一起导出，不能让导入方「按字段名重猜」。载体是 `frontend::module::TypeDefPayload`：

```text
TypeDefPayload {
    // …既有字段…
    type_params      : [TypeParam { name, kind }]        // 含 P: T -> Proposition（§14.3）
    field_obligations: [FieldObligation]                 // 见 §14.2；无 witness 的记录为 []
    evidence_slots   : [EvidenceSlot {
        field          : FieldId,
        quantity       : Quantity,                       // 恒为 Zero
        proposition    : Proposition,                    // 可含 FieldPath 前序引用
        evidence_ref   : Option<EvidenceRef>,            // 显式证据来源；自动合成时为 None
        certificate_ref: Option<CertificateRef>,         // 已验证证书引用；未验证时为 None
        signature_hash : ContentHash,                    // 随 §4.1 签名失效
    }]
    libraries        : [LibraryRef]                      // 命题/证据引用到的库，跨模块随导出携带
}
```

- 导出面：`checker.rs` 构造 `TypeDefPayload` 时，对每个 `quantity == Some(Zero)` 的字段填一条 `EvidenceSlot`；`field_obligations` 与 `evidence_slots` 同源，不各写一份。
- 导入面：导入方拿到 `EvidenceSlot` 后，用**自己的当前 `LogicSignature`** 验证——`signature_hash` 不符时按 §4.1 的跨签名三路径处理（翻译项 / 桥接公理 / 重新证明），不得直接复用导出方的 `Proved`；`libraries` 里引用到的库必须出现在导入方 `libraries` 列表里，否则 `Unproven` 并提示登记该库或写桥接。
- 多文件/多模块构造点复用同一 `judge_refined_obligation`；跨模块不新增第二套判定入口（RFC-039 D55 静默通道禁令）。

#### 14.5 类型级 `b.w`：`ConstExpr::FieldPath` + evidence term

witness 字段**只能类型级使用**（`b.w` 是用户可写的类型级语法，见本节末对 4A 的取代说明），值级读取一律拒绝：

- **项表示**：`ConstExpr::FieldPath { base: Box<ConstExpr>, field: FieldId }` 是统一项世界里的字段路径项。`b.w` 在类型位置解析为 `FieldPath(NamedVar("b"), "w")`；它指称的不是运行期值，而是 `EvidenceSlot.evidence_ref` 对应的 **evidence term**（证明项）。
- **evidence term**：`EvidenceTerm(FieldPath, EvidenceRef, signature_hash)` 是 `Certificate` 的一个 `EvidenceRef` 来源（§4.2），对应 `FieldObligation.evidence_ref` / `certificate_ref` 两槽（§14.2）里的显式证据或已验证证书。把 `b.w` 用于另一个命题（如 `Q(b.w)`）时，producer 把 `EvidenceTerm` 作为候选证书的前提，`Kernel::check` 按签名重放；`signature_hash` 不符则不复用。
- **类型级用途**：`FieldPath` 可出现在谓词实参、类型构造实参位、另一命题的 evidence term 前提、以及 `TypeDefPayload.evidence_slots[i].proposition`（逐槽索引）。注意：`FieldPath` 指向 witness 时是**证明项前提**，不是「数据字段实参」——§14.1 的「evidence 字段实参只吃严格前序数据字段」约束的是 `P` 的数据实参，不禁止把前序 evidence 的证明项当作新命题的前提（后者经 `EvidenceTerm` 交 `check` 重放）。`b.w` 不产生运行期值、不占数据槽。
- **值级拒绝**：`b.w` 出现在值表达式（`FieldAccess` 的 `Struct` 臂与 `TypeRef→Struct` 臂）报 **E1114**（`witness_field_not_readable`）。`FieldPath` 不是值项，`Evaluator` 与 IR 生成都不为它产物化值。
- `ConstExpr` 的穷尽 `match` 补臂点（`const_eval::substitute`、`free_names`、`Display`、`translate::` 系列、`proof_execution`、`fold_proof_call_arg` 等）必须处理 `FieldPath`；未支持项语言的求值器遇到 `FieldPath` 返回 `Unproven`，不静默当 `NamedVar`。

```text
// 设计示意（非可编译片段：Proved / FieldPath / evidence term 是内核项，见本节的接口定义）
// 类型级使用：把 b.w 当证据前提, 不是取值
useWitness: (b: Box) -> Type = { Proved(b.w) }   // b.w 解析为 FieldPath + evidence term
// 值级 b.w => E1114
```

> **取代 4A 的「类型级无用户语法」**。4A 裁决（09a §P6 6.9.5 H0 裁决，2026-11-13）的前提是「`b.w` 需要 `ConstExpr::FieldPath` 与 proof token，属独立 term 语言设计，现在加会造第二 term world」，故裁定 V1 不提供用户 `b.w` 投影。`ConstExpr::FieldPath`、`EvidenceTerm`、`TypeDefPayload.evidence_slots` 都属于**同一套统一项世界**（§4，不是第二 term world），该前提不成立：**类型级 `b.w` 是用户可用语法**（上例 `useWitness`），4A 的「类型级 `b.w` 无用户语法」一条由本节取代，不再有效。4A 的其余两条——witness 是编译期字段、构造时证明、运行期擦除；字段表示 `quantity` + 命题类型——继续成立。值级 `b.w` 仍按 4A 拒绝（E1114）。`FieldPath` 只活在类型/命题位置，不是运行期字段路径，不构成「A 式运行期字段」。

#### 14.6 不可变、擦除与诊断码

- **5A 整体不可变**：含任一 evidence 字段的记录整体不可变——数据字段不得声明 `mut`、不得字段赋值、不得 `&mut` 借用、不得作为写方法接收者（含自动 `WriteBorrow`）。否则 witness 命题可能因字段改写失效，且当前无 runtime check / revalidation；违反报 **E1115**（`witness_record_immutable`）。更细粒度的线性/唯一性设计另案。
- **擦除**：evidence 字段的 `quantity=Zero` ⇒ 不占运行期槽；`CreateStruct` / `GetField` / `SetField` / `resolve_field_index` 只认 `runtime_field_index` 的数据序；`Proved(checked)` 证书按 §13 编译期留存、运行期擦除；Runtime 模式的 Bool check 与 witness 无关。
- **诊断码分工**（四码各管一面，不借码、不静默）：

| 码 | 触发 | 语义 |
| --- | --- | --- |
| **E4018** | 构造点 evidence 命题被判伪（经 `check` 确认的反例 / 内核直判） | 精化谓词违反，带反例 |
| **E2031** | 构造点 evidence 命题 `Unproven`（精化通道，带 `assigned`/`var`/`constraint` 三槽） | 精化约束无法证明，提示补显式证据（§4.3） |
| **E1114** | 值级读取 witness 字段 `b.w` | witness 字段不可值级读取 |
| **E1115** | witness-bearing 记录的 `mut` / 字段赋值 / `&mut` / 写接收者 | 记录整体不可变 |
| **E4023** | 非精化通道的一般性 `Unproven`（如 `FieldPath` 求值器不支持） | 一般性无法证明，带命题/原因/位置（§12.3） |

`ProofFunctionRequired` 仍走 E4020；一般性 `Unproven` 与 E2031 的分工不变（§12.3、RFC-013）。**声明期拒绝只保留三类**：`space=Proof` 但前向引用后序字段、引用另一 evidence 字段、`P` 的 kind 为 data family（§14.3）。它们在声明处直接拒绝，不进构造点、不静默降级为数据字段。`space=Data` 的数据类型族应用（含 `Array(Int, n)`）与 `space=Proof` 的自引用精化（`v: Pos(v)`，admission）都是合法数据字段。**泛型未实例化不属声明期拒绝**：按 §14.3，`P` 未绑定时 `w` 的结果 sort 仍是 `Proof`、命题为开项，在**构造点**返回 `Unproven`（E2031 / E4023），不静默放行、也不退化成数据字段。

## 详细设计

### 语法变化

| 之前 (RFC-022)                        | 之后 (本 RFC)                                                                |
| ------------------------------------- | ---------------------------------------------------------------------------- |
| `//! requires: NonEmpty(n) = n > 0`   | 编译期谓词作为参数类型 `(b: Positive(b))`                                    |
| `//! ensures: ExistsMax(result, arr)` | 返回类型使用返回值形参 `-> (result: IsMax(T, arr, result))`                  |
| `/*! invariant: ... !*/`              | 变量上的编译期谓词类型标注——Floyd-Hoare 不变式                               |
| `//! decreases: n`                    | 测度写在精化类型位（`Terminates`）；编译器先自动探索，探索不出才要求显式给出 |
| 规约是注释                            | 规约是类型系统                                                               |
| `/*! invariant: ... !*/` 之外无记录证据 | 依赖记录 Σ：按 proof-space 判据区分 evidence 字段（q=0）与数据族字段，见 §14   |

### 语法

**编译期谓词没有新关键字。** `{}`
是证明空间，和现有类型定义语法完全一致。编译期谓词就是返回 Type 的函数——`name: (params) -> Type = { 断言 }`。使用时就是函数调用——`Positive(b)`、`IsMax(T, arr, result)`。

```bnf
# 编译期谓词 = 返回 Type 的函数，{} 内为编译器验证的断言
# 使用现有函数/类型语法，无需新增 BNF 规则
predicate ::= identifier ':' params '->' 'Type' '=' '{' assertions '}'
```

**谓词应用的实参必须是编译期常量形态**——字面量、变量（按名绑定）、类型应用（递归提取），或**编译期可指称的函数引用**（函数名）。实参形态不可转换为常量表达式报
**E1092**，实参个数与谓词声明的形参个数不匹配报
**E1093**。实参按位置绑定到形参列表，谓词元数由声明决定——`Positive(x)` 一元、`IsMax(T, arr, result)`
三元、`Terminates(m)` 与 `Terminates(FnType, m)`
一元与二元——精化约束**绝不静默丢弃**（此前不可转换实参会导致约束无声消失，违反约束的绑定静默通过）。

**新增语法概念：返回值形参**——`-> (name: Type)` 中 `name` 是返回值形参。

返回值形参是 YaoXiang 在现有函数语法上引入的语法概念之一（记录 evidence 字段复用既有字段语法，见 §14）。它的语义：

- `name` 的值由 `return` 语句提供
- `name` 仅存在于类型签名中，被后置条件谓词引用（如 `-> (result: IsMax(T, arr, result))`）
- `name` 不进入函数体作用域，不出现在调用方
- 返回值形参**可选**——无后置条件时签名与普通函数完全一致（`-> Int`），不引入任何额外负担

引入它的理由：后置条件需要引用"函数将要返回的值"。在没有返回值形参的情况下，编译器只能通过特殊规则（如隐式变量
`$result` 或 `__retval__`）来让谓词引用返回值。返回值形参将这种引用显式化——它就是一个形参，只是值由
`return` 而非调用方提供。

**证明函数**不是新概念——它就是一个 YaoXiang 函数，其返回类型是被断言的命题。当编译器返回 Unproven 时，程序员提供证明函数，类型检查器以与验证任何函数返回类型完全相同的方式验证它。不需要新语法、新关键字、新规则。

> **两者的边界。**
> 正确性域（谓词 Unproven）在**体**里补证明：写一个函数，其返回类型就是待证命题。终止域（§6.9）在**类型位**补测度：写
> `Terminates(测度)`
> 作为绑定或函数签名的类型。前者是「证不出的命题写一个证明」，后者是「探索不出的测度显式声明」——机制同一（皆为精化类型应用），落点不同。终止域**不需要**写一个
> `_proof` 函数。

### 类型系统影响

- **类型宇宙**：编译期谓词位于 Type₂ 层——接受值返回 Type 的函数，与类型构造器层级相同
- **泛型交互**：编译期谓词可带泛型参数，如 `NonEmpty: (T: Type) -> (arr: Array(T)) -> Type`
- **所有权交互**：编译期谓词中的表达式遵守所有权规则，只能读不能写
- **类型推导**：编译期谓词的参数参与 HM 类型推导
- **依赖记录 Σ**：记录字段按 §14 的 proof-space + 位置规则分为数据字段与 evidence 字段（`Quantity::Zero`）；proof family 参数 `P: T -> Proposition` 与 `TypeDefPayload.evidence_slots` 给出泛型与跨模块的完整语义

### 运行时表示

编译期谓词在运行时**按 dispatch 分派结果处理**：

- **CompileTime 模式**（所有自由变量编译期已知）：`Kernel::check` 验证证书后，`Proved(checked)` 在编译期留存、运行期完全擦除。`Positive: (x: Int) -> Type = { x > 0 }`——参数 `b: Positive(5)` 在运行时的表示就是 `Int`；精化条件 `{ 5 > 0 }` 已通过，证书不进入运行期布局。Σ evidence 字段按 §14 同此：`base` 不进布局、`CreateStruct` data-only、值级 `b.w` 报 E1114。
- **Runtime 模式**（存在运行时自由变量）：保留运行时 check——在值层面执行 Bool 检查，注入流敏感假设集 Γ。详见 §11 的 dispatch 分派管道与 §13 的擦除模型。

将编译期谓词放在类型位置（如 `f(x: Positive(x))`）不产生包装类型、不分配额外内存。但 `x`
来自运行时输入时，**会**插入运行时 Bool 检查。

**与 `ref`
的交互约束**：编译期谓词只能引用不可变借用或所有权已转移的值。引用可变借用的编译期谓词，编译器无法在编译期保证验证结果在运行时仍然成立——此类用法直接报编译错误。

### 编译器改动

1. **解析器**：编译期谓词与证明函数使用标准函数语法，无需额外解析规则
2. **证明内核**：`Kernel::check(signature, certificate, proposition)` 与 `CheckVerdict`；按 `signature.rules` 重放 `Certificate.steps`，是 `Proved` 的唯一出口
3. **逻辑签名与库**：`LogicSignature`（sorts / rules / axioms / `libraries: [LibraryRef]` / hash），每个 `LibraryRef` 带 `kind/id/manifest/hash`，签名 hash 覆盖全部库；签名可替换，跨签名走显式翻译或桥接公理
4. **Certificate producer**（不可信）：内核直判规则、Γ/`q=0` 证据、证明函数、SMT-LIB 2.6 翻译（默认 Z3、备选 CVC5）、tactics、proof search；全部只交候选 `Certificate`，`unsat` 不直达 `Proved`（§8）
5. **验证条件生成**：WP/SP 演算、循环不变式、以及 §14 记录构造的 `FieldObligation`（字段位置规则 / `bind_args_to_params` / `judge_refined_obligation`），产出命题与候选证书，不自行判定
6. **错误报告**：反例（经 `check` 确认）+ 未解命题 + 原因 + 补证据提示 + 源码位置关联（§12.3）

### 向后兼容性

- ✅ 未使用编译期谓词的代码完全不变
- ✅ 编译期谓词在 CompileTime 模式下零运行时开销，Runtime 模式下仅保留必要的 Bool 检查
- ⚠️ RFC-022 的 `//!` 语法不再支持——但 022 从未被实现，无迁移负担
- ⚠️ 记录类型新增 §14 的 evidence 字段位置规则：原先「字段名恰好是谓词应用、静默当数据字段」的程序转为编译期 witness 判定（值级读取 E1114、witness-bearing 记录不可变 E1115），属对外行为变更，按 RFC-039 D59 的 D/E 类登记

## 权衡

### 优点

- **Curry-Howard 同构完整兑现**：类型即命题，程序即证明，`name: Proposition = Proof`；命题与证书同住 D62 的统一项世界
- **统一性**：编译期谓词与普通函数使用完全相同的语法，无概念分裂
- **producer 透明**：程序员不需要知道 SMT 的存在，心智模型是「补显式证据 → 内核按签名验证」；后端可替换
- **渐进式采用**：可从一个编译期谓词开始，逐步增加覆盖
- **最小运行时开销**：证书与 `q=0` 项编译期留存、运行期擦除；Runtime 模式仅保留必要的 Bool 检查

### 缺点

- **编译时间**：certificate producer 的搜索与内核重放都增加编译时间，但 producer 预算（§8.4）与证书步数上限保证可控
- **自动证明边界**：producer 交不出可验证证书时内核返回 `Unproven` 并提示补证据。这不是语言缺陷——这是停机问题的必然结论。内核诚实区分「算不出」与「不成立」
- **学习曲线**：编写有效的编译期谓词、证明函数与选择逻辑签名需要理解 Curry-Howard 同构与「签名相对」的基本直觉
- **实现复杂度**：编译期证明管道的统一需要精心设计

### 风险缓解

- producer 预算硬限制（搜索步数 10,000 / 时间 100ms / 实例化深度 3）与证书重放步数上限，超预算返回 `Unproven`（§8.4）
- 依赖类型预归约：确定性的值计算由内核直判规则先吃掉，producer 只啃非确定部分
- `Unproven` 不是死路：补显式证据（证明函数、proof library 条目、换签名 / 桥接公理，§4.3）；终止域在类型位给出测度（§6.9）——全部经 `Kernel::check` 验证
- 增量验证：`Proved(checked)` 按 `signature_hash + proposition + certificate` 缓存，仅重算签名变更或证书失效的模块
- 清晰的错误信息 + 经 `check` 确认的反例 + 未解命题 + 原因 + 补证据提示 + 一行预算摘要（§12.3）

## 替代方案

| 方案                               | 为什么不选择                                                                                                |
| ---------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| RFC-022：`//!` 注释式规约          | 规约与类型分裂，违反 Curry-Howard 同构                                                                      |
| 独立规约文件（如 CVL）             | 规约与代码分离，增加维护成本                                                                                |
| 仅运行时断言                       | 无法静态保证正确性                                                                                          |
| 外部证明助手（如 Coq）             | 与编译器割裂，需要独立的证明语言和信任边界。YaoXiang 的选择：证明就是 YaoXiang 代码，证书经 `Kernel::check` 按可替换签名验证，不硬编码信任根 |
| **本方案：编译期谓词作为一等公民** | ✅                                                                                                          |

## 实现策略

### 阶段划分

| 阶段       | 内容                                                                                                       |
| ---------- | ---------------------------------------------------------------------------------------------------------- |
| **阶段 1** | `Kernel::check` + `LogicSignature` + `Certificate` + `CheckVerdict`；内核直判规则（结构等价、βδι-归约、defeq） |
| **阶段 2** | axiom library / proof library 的 manifest 与哈希；证明函数与显式证据接入；proof-producing 适配器把 `unsat` 展开成登记规则的 `ProofStep` |
| **阶段 3** | 循环不变式 VC 生成 + 终止检查（§6、§6.9）+ 依赖记录 Σ 的字段位置规则、构造义务、`FieldPath` 类型级使用（§14）走同一 `check` |
| **阶段 4** | 按 `signature_hash` 的证书缓存 + 增量验证 + `TypeDefPayload` 跨模块证据槽 + 跨签名翻译/桥接公理 + IDE 支持（§14.4） |

### 依赖关系

- RFC-010: 统一类型语法 — 编译期谓词基于 `name: type = value`
- RFC-011: 泛型系统 — 编译期谓词可带泛型参数
- RFC-009: 所有权模型 — 编译期谓词表达式遵守所有权规则

## 开放问题

- [x] **wasm32 目标的 Z3 可用性**：wasm 下无 Z3（SMT 代码全部 `cfg(not(target_arch = "wasm32"))` 排除）。
      影响范围与降级方向：
      - `ownership.rs` `smt_cut` 恒返回 `false` → 回边穿越 → **保守拒绝**（sound 方向，只是少收窄）
      - 终止检查策略 1、谓词第 2b/3 级整体跳过

  **修正（2026-10-01）**：原文称这些路径「当前**不可达**」（理由是 `predicate_defs`
  生产从无填充、`parser` 不产生 `MonoType::Refined`、`with_solver` 生产从不调用），
  已不成立——三处都已落地（谓词定义注册 #377-3；精化类型正格化；生产管线注入
  求解器）。故 wasm 与 native 的差异不再仅限于 `smt_cut` 的精度：

  | 路径 | native | wasm | 降级方向 |
  | --- | --- | --- | --- |
  | 终止检查显式测度判定 | 判定 | `Unjudged` | 不报 E4022（少报） |
  | 良基性判定 | 判定 | `Unjudged` | 不报 E4022（少报） |
  | 精化谓词的 SMT 蕴含级 | 执行 | 跳过 | 落到 `Unproven` |

  均为**保守**方向（producer 少一条搜索路径 ⇒ 候选证书少 ⇒ `Unproven`，`Kernel::check` 本身不变），不影响 soundness。按 §8.7 的口径，下表按「producer 可得性」解释，不再按「判定精度」解释。若要精度一致，
  方案与代价见 issue #376（结论：优先 JS 侧 Z3 实例，而非把 Z3 链进主 wasm——
  后者需换 emcc 构建体系且产物从 3 MB 涨到 ~20 MB）。
- [x] **SMT 求解器选择**：默认 Z3（MIT 协议，最广泛验证）。CVC5 作为 SMT-LIB 兼容备选，编译器标志切换。SMT-LIB 2.6 是 producer 的翻译目标——它是候选证书的输入格式，不是信任边界；`Proved` 的唯一出口仍是 `Kernel::check`（§8）。不做自定义通用求解器接口，也不把 `unsat` 当判定结果。
- [x] **求解预算的具体数值**：producer 搜索步数 10,000 / 时间 100ms / 量词实例化深度 3，另设证书重放步数上限（§8.4）。编译器内部固定，不给 knob。实际使用中如有真实用例证明不够（非「用户写错了」），再调整。
- [x] **量词支持范围**：语言层面不限制量词阶数。编译期谓词接受 Type 参数——Type 包括函数类型——因此高阶量词是类型系统的自然推论，不需要特殊语法。producer 可处理一阶量词（forall/exists，支持交错嵌套，由预算深度 3 限制）并交出候选证书；高阶量词交不出可验证证书时，内核返回 `Unproven` 并提示补显式证据——写一个返回类型等于该命题的 YaoXiang 证明函数（§4.3），由 `Kernel::check` 按当前签名验证。不需要外部导出，不需要 AI，不需要交互式证明模式。一切都是 YaoXiang 代码，一切由证明内核验证。
- [x] **反例格式化**：源码变量名直接用作 SMT 变量名（加上模块前缀避免冲突）。producer 返回的 `sat + model` 是候选反例，必须经 `Kernel::check` 确认使命题为假后才渲染。输出格式：变量名 = 具体值 + 源码位置 + 谓词定义位置。不做复杂映射层。
- [x] ~~**与 `ref` 智能指针的编译期谓词交互？**~~
      → 已决策：编译期谓词只允许不可变借用或已转移所有权的值。可变借用的值不可出现在编译期谓词中。
- [x] **`forall` 谓词违反计数量度对非相邻操作的扩展？**
      → 不扩展。当前覆盖范围（相邻交换、相邻移动）由策略 1（线性秩函数）互补覆盖——快排外层区间收缩由策略 1 兜底，堆排序由策略 1（数组索引模式）兜底。不能由任何策略证明终止的循环，编译器直接报错——这是硬安全哲学，不是缺陷。如果未来有真实场景（非学术构造）的算法四种策略都无法覆盖，再重新讨论。→
      **再讨论已触发（2026-09-14，#318）**：非结构递归（gcd 类非直接递减、相互递归、归并分区）即本条款预期的真实场景——测度探索的模板序列无法自动拿下它们，且它们无法在不破坏可读性的前提下改写为可分析的迭代模式。结论：硬安全哲学不变（测度写得不对仍被经 `check` 确认的反例打回，人只能证不出来、不能证错），但「探索不出即拒绝」放宽为「探索不出可由程序员在类型位显式给出测度」——见 §6.9。
- [x] **线性秩函数枚举组合爆炸**：候选枚举上限为 3 个有界变量。≤3 时枚举所有线性组合、逐个产出候选证书交 `Kernel::check`。>3 时只尝试单变量度量（`v_i`、`u_i - v_i`），失败直接报编译错误——提示程序员「循环有 >3 个有界变量，自动 producer 无法合成多变量度量，请补显式证据」。这不是工程妥协——是迫使程序员写更简单的循环。

## 参考文献

- [RFC-010: 统一类型语法](010-unified-type-syntax.md)
- [RFC-011: 泛型系统设计](011-generic-type-system.md)
- [RFC-009: 所有权模型](009-ownership-model.md)
- Howard, W. A. (1969). The Formulae-as-Types Notion of Construction.
- Swamy, N. et al. (2016). Dependent Types and Multi-Monadic Effects in F\*. _POPL 2016_.
- Vazou, N. et al. (2014). Refinement Types for Haskell. _ICFP 2014_.
- Leino, K. R. M. (2010). Dafny: An Automatic Program Verifier for Functional Correctness. _LPAR
  2010_.
- De Moura, L. & Bjørner, N. (2008). Z3: An Efficient SMT Solver. _TACAS 2008_.

---

## 生命周期与归宿

```
┌─────────────┐
│   草案      │  ← 作者创建
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  审核中     │  ← 当前状态：社区讨论
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
│ accepted/   │    │  rejected/  │
│ (正式设计)  │    │ (保留原位)  │
└─────────────┘    └─────────────┘
```

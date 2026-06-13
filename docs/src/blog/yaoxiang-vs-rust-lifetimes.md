# 我们把 `'a` 藏到编译器里了

*——关于 YaoXiang 所有权模型的一个诚实笔记*

---

你第一次看到这段 Rust 代码时，花了多久才真正理解？

```rust
struct Parser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn advance(&mut self) -> &'a str {
        let start = self.pos;
        self.pos += 1;
        &self.input[start..self.pos]
    }
}
```

三个 `'a`。一个在结构体上，两个在 impl 块里。它们说的是同一件事：`Parser` 不能活得比它借用的 `input` 更久。这是正确的。Rust 的安全性就建立在这套机制上。

但我们写这类代码时常有一个念头：**这件事，编译器自己不能搞定吗？** Rust 的答案是"不能"——至少在不改变借用模型的前提下不能。

**这篇文章不是"YaoXiang 解决了这个问题"。是"YaoXiang 在试另一条路——把 `'a` 藏到编译器里，让编译器替你写它。走到哪了，还没解决的硬骨头是什么。"**

---

## Rust 为什么需要 `'a`

Rust 的 `&T` 和 `&mut T` 是指针——指向数据的指针。借用一个值，就是创建一个指向它的引用。这个引用有自己的生命周期。当引用跨函数边界传播（作为返回值、存入结构体）时，编译器无法在单个函数内推断引用能活多久——需要程序员用 `'a` 提供"这个返回值和这个参数共享生命周期"的信息。

Rust 社区没有停在原地。生命周期省略规则让大多数简单函数免于标注。NLL 在 2018 版落地，让借用不再被词法作用域束缚。但当引用需要存入结构体、从函数返回、被闭包捕获时——**模型本身决定了这些场景必须由程序员标注引用之间的关系。**

---

## 换个角度：借用不是指针，是令牌

YaoXiang 的核心设计记录在 [RFC-009（所有权模型）](/design/rfc/accepted/009-ownership-model) 中。它没有改变默认语义（都是 Move），而是改变了**借用的本体**。

在 YaoXiang 中，`&T` 和 `&mut T` **不是指针**。它们是**零大小的编译期令牌**——类型级别的访问权限证明。借用一个值，不是创建指向它的指针，而是创建"我被允许访问它"的证明：

```
&T     →  保证数据不可变。实现 Dup（可复制），多个只读令牌共存安全
&mut T →  保证独占可变。不实现 Dup（线性），同一来源只能有一个
```

在 Rust 中你写 `&` 在调用点（`distance(&p1, &p2)`）。在 YaoXiang 中，编译器看到函数签名要求 `&Point`，在调用点自动创建令牌——调用点变成 `distance(p1, p2)`。代价是定义者的签名必须声明 `&`，否则默认 Move 会吃掉所有权：

```yaoxiang
# 签名中需要 &——编译器看到 &Point，调用时自动创建令牌
check_dimensions: (v: &Vec3) -> Bool = { ... }
check_bounds: (v: &Vec3) -> Bool = { ... }

v = Vec3(1.0, 2.0, 3.0)
if check_dimensions(v) && check_bounds(v) {  # 每次都自动创建 &Vec3 令牌
    # v 仍然可用
}
```

Rust 中你把 `&` 放在调用点；YaoXiang 中你把 `&` 放在定义点。不是标注消失了——是标注的位置变了。

---

## 品牌机制：藏在编译器里的 `'a`

用户从不接触这个——但理解它才能理解 YaoXiang 到底做了什么。

编译器内部给每个借用令牌分配编译期唯一编号：

```
用户看到的         编译器内部表示
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)
&mut Point     →  WriteToken(Point, #M)
```

当你从 `&Point` 访问字段得到 `&Float` 时，后者携带派生品牌：`#N.field_x`。当你把 `&mut` 令牌 Move 给另一个变量时，编译器知道原变量不再持有——这是 Move 语义的基础能力。

**`#N` 就是 `'a`。** 前缀关系——`#N` 是 `#N.field_x` 的前缀——就是 outlives 约束。同一个信息。Rust 的程序员写 `'a`，YaoXiang 的编译器写 `#N`。

区别只有推导成功率。Rust 的省略规则和 NLL 覆盖了约 80% 的场景。YaoXiang 的赌注是：**如果语言设计给编译器更干净的输入，它能不能覆盖更多？**

这个赌注由几条语言约束支撑：

- **禁止变量遮蔽**——`x` 在这个作用域里只有一个身份，编译器不需要区分"你说的是哪个 x"
- **显式 `return`**——什么东西逃逸出块，是写明的，编译器不需要推断"最后一行是不是返回值"
- **`for` 每次迭代新建绑定**——迭代间的变量互不干扰，编译器不需要追踪"上次迭代改了什么"

这几条不是"规范"。和 Java 的 getter/setter 那种无意义仪式不同——每一条都是**把编译器需要推断的信息变成程序里已经写明的信息**。编译器不需要猜"你指的是哪个变量"、"这个东西逃逸了没有"、"循环变量跨迭代怎么变"——答案在代码里。

---

## 品牌能做到的事——和 Rust 一样

因为令牌是普通类型，它遵守所有普通类型的规则。没有"引用不能返回"、"引用不能存结构体"、"闭包不能捕获引用"的特殊禁令。**但 Rust 也一样——Rust 全部能做到。** 区别不是能力，是谁写标注。

**返回引用——Rust 程序员写 `'a`，YaoXiang 编译器写 `#N`：**

```yaoxiang
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()   # 令牌传播到调用方，编译器追踪品牌派生链
```

Rust 做同样的事，只是程序员需要写 `'a` 连接输入和输出。YaoXiang 中编译器通过品牌路径自动派生——`px_ref`（`#N.field_x`）从 `p`（`#N`）派生。同一个约束，不同的记录方式。

**结构体持有引用——无生命周期参数：**

```yaoxiang
Window: Type = {
    target: Point,
    view: &Point,   # 令牌字段，和其他字段没有区别
}
```

Rust 中结构体有引用字段时，`'a` 需要出现在结构体定义和所有 impl 块中——程序员显式标注 `Window<'a>` 的生命周期约束。YaoXiang 里 `view: &Point` 不写 `'a`，但品牌编号仍然在编译器内部扮演同样的角色——`Window` 实例销毁时，内部的令牌随之死亡。同一个保证，不同的可见性。

**闭包捕获——Dup 令牌零成本复制：**

```yaoxiang
filter_by_threshold: (items: List(Point), threshold: &Float) -> List(Point) = {
    items.filter(|p| p.x > threshold)   # threshold 令牌复制进闭包，零开销
}
```

`&Float` 实现 Dup（可复制），闭包捕获它就像捕获一个零大小的整数。和函数调用的自动借用共用同一套规则。用户零标注。

---

## 代价：标注从签名中消失

Rust 的 `'a` 有一个常被提起的价值：它也是文档。`fn split_at_mut<'a>(slice: &'a mut [T], mid: usize) -> (&'a mut [T], &'a mut [T])`——`'a` 告诉读者两个返回的切片引用同一个原始数据。

现实中这个论点的力度有限——大多数 Rust 初学者并不把 `'a` 当文档读，而是当编译器要求的咒语抄。但公平地说：在复杂借用场景中，Rust 的 `'a` 至少给了追溯数据流的一个起点。YaoXiang 中你需要理解品牌派生链——品牌对用户不可见，这依赖工具链，而工具链尚未成熟。

---

## 令牌冲突检测：同一套证明管道

Rust 有独立的"借用检查器"。YaoXiang 的**设计方向**是把借用冲突统一进类型检查的证明管道（[RFC-027（编译期谓词与统一静态验证）](/design/rfc/accepted/027-compile-time-evaluation-types)）。

令牌冲突是一条霍尔命题：

```
{ 冲突的 ReadToken 已死 } data.push(4) { WriteToken 安全获取 }
```

```yaoxiang
# &mut T 是线性类型——Move 后原变量不再持有
bad: (p: &mut Point) -> Void = {
    p2: &mut Point = p    # WriteToken 从 p 转移到 p2
    p.x = 10.0            # { p 持有 WriteToken } p.x = 10.0 { 安全 }
}                          # → p 的 WriteToken 已 Move → Disproved

# &T 是 Dup——可复制
good: (p: &Point) -> Void = {
    p2: &Point = p        # 复制只读令牌
    print(p.x)            # OK，两个只读令牌共存
}
```

和类型错误、谓词验证失败共享同一条错误报告路径。你不需要学两套诊断系统。**但代价是：** 一个复杂的令牌冲突在 Rust 中产生的是精心措辞的借用检查错误；在 YaoXiang 中可能表现为"WriteToken(#7.field_x) conflicts with WriteToken(#7)"——技术上准确，但品牌编号对人类读者没有意义。错误信息的可解释性是尚未验证的领域。

---

## `ref` 关键字：自动选择 Rc/Arc

令牌不能跨任务（跨线程）——它们是编译期证明，不是运行时值。跨作用域共享用 `ref`：

```yaoxiang
shared_data = ref Point(1.0, 2.0)   # 编译器逃逸分析自动选 Rc 或 Arc

spawn {
    print(shared_data.x)   # 跨任务 → 编译器选 Arc
}
```

- 不逃逸到 spawn 块 → `Rc`（非原子引用计数）
- 逃逸到 spawn 块 → `Arc`（原子引用计数）

代价：阅读代码时无法从局部知道 `ref` 是 Rc 还是 Arc。一次重构（把代码包进 spawn）可能悄悄改变引用计数的实现——你不会收到编译器提醒。性能变化是隐式的。

---

## 当前的硬骨头：RAII 太粗糙了

前面说了令牌是值、生命周期由 RAII 管理。但普通值的 RAII 规则是：**值存活直到作用域结束。** 这恰恰是 Rust 在 NLL 之前的问题——借用持续到整个块结束，即使你早已不用它了。

```yaoxiang
process: (data: &mut Data) -> Void = {
    header_view: &Header = data.header()    # 从 &mut Data 派生 &Header
    header_info = parse_header(header_view) # ← header_view 的最后使用
    # header_view 没必要活到函数结束了——
    # 但 RAII 让它活到 }

    data.modify(header_info)   # ❌ ReadToken 还"活着"，WriteToken 被阻止
}
```

Rust 的 NLL 分析最后一次使用而不是词法作用域。YaoXiang 需要同样的能力。正在做的方案是把令牌活性分析也接入证明管道——三层：

1. **快速通道**——复用现有的 BorrowChecker（线性扫描，IR 指令级）。同一基本块内令牌用完就释放的场景直接通过
2. **结构分析**——品牌树前缀匹配（判断谁和谁冲突）+ DAG 消费者查询（判断令牌的最后消费者是否在当前节点之后）
3. **SMT 求解**——循环条件等需要逻辑推理时才激活

证明管道基础设施（`Proved/Disproved/Unproven` 三值返回、Z3 SMT 后端、假设栈）已经部分实现在 `src/frontend/core/typecheck/proof/`。所有权层（`layers/ownership.rs`）还是骨架——直接返回 Proved，没做实际检查。正在填。

当前版本的解决方案是手动嵌套块来缩短令牌作用域：

```yaoxiang
process: (data: &mut Data) -> Void = {
    header_info = {
        header_view: &Header = data.header()
        parse_header(header_view)
    }   # header_view 随块结束释放
    data.modify(header_info)   # ✅
}
```

这是真实摩擦。每个从 Rust 过来的人都会碰到。能不能消除——取决于管道接入所有权层的效果。

---

## 最难的问题：兜底

Rust 的 `'a` 不仅是负担——它也是兜底。编译器推断不出来，程序员标注生命周期关系，编译器验证。**你有一支笔。**

YaoXiang 的兜底理论上应该是编译期**证明函数**（RFC-027 §4.2）：编译器自动推导失败 → 程序员写一个函数，返回类型就是"令牌不冲突"这个命题 → 编译器验证这个函数的类型。但——

"令牌不冲突"的证明函数长什么样？用户怎么构造一个类型为 `WriteTokenAvailable` 的值？要不要理解品牌编号 `#N` 和 `#N.field_x` 的前缀关系？

**如果证明函数要求用户理解品牌编号——那我们只是把 `'a` 换了个写法，叫 `#1`。没省任何事。**

这个问题还没有答案。这是整个实验最可能卡死的地方。

---

## RFC-009 的九次迭代

这不是象牙塔里想出来的设计。RFC-009 经历了九次大版本：

| 版本 | 关键变化 | 被推翻的原因 |
|------|----------|------------|
| v1–v7 | 基于 Rust 所有权模型，逐步增加消费分析、逆函数、字段级可变性 | 过度设计，复杂性失控 |
| **v8** | "丐版借用"——`&T`/`&mut T` 只能做参数，不能返回、存结构体、闭包捕获 | 三条硬编码禁令。表达力严重受限 |
| **v9** | 借用令牌系统——`&T`/`&mut T` 是普通类型，遵守普通规则 | 消除特殊规则，但将品牌追踪下沉到编译器内部 |

v8 到 v9 的跃迁是真正的突破：从三条禁令到零条特殊规则。但 v9 消除的是用户可见的规则，不是系统的内在复杂性——品牌机制、派生追踪、同源冲突检测，这些仍然存在，只是换到了编译器里。把借用检查统一进证明管道是一个方向，但能不能在真实代码上跑通、兜底怎么设计——还在验证。

---

## 写在最后

我们没有消灭 `'a`。`#1` 就是 `'a`——同一个信息，不同的位置。

实验的赌注是：语言设计约束（禁止遮蔽、显式 return、`for` 新绑定、`{}` DAG 语义）给了编译器更干净的输入，品牌推导也许能在 Rust 的生命周期省略规则失效的大部分场景下自动成功。如果能——用户不再需要写 `'a`，不需要区分标注和省略，不需要学借用检查器。如果不能——或者说兜底机制（证明函数）要求用户理解品牌编号——那只是一种重新发明。

正在做。有结果了再写。

---

*YaoXiang 是一个正在开发中的编程语言。所有权模型见 [RFC-009](/design/rfc/accepted/009-ownership-model)，闭包捕获见 [RFC-023](/design/rfc/accepted/023-closure-capture-model)，并发模型见 [RFC-024](/design/rfc/accepted/024-concurrency-model)。*

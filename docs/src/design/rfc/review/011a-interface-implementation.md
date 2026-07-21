---
title: "RFC-011a: 接口实现与动态分发"
status: "审核中"
author: "晨煦"
created: "2026-06-14"
updated: "2026-07-05"
group: "rfc-011"
---

# RFC-011a: 接口实现与动态分发  

> **父 RFC**: [RFC-011: 泛型系统设计](../accepted/011-generic-type-system.md)
>
> **本 RFC 补充并替代 RFC-011 §2.1-2.4 的接口约束部分。**

## 摘要

RFC-011 定义了泛型系统，但没有详细说明接口实现机制。本文档补充：

1. **接口声明**：类型定义中直接写接口名，不需要 `impl` 关键字
2. **方法实现**：内部声明和外部声明都支持
3. **重载规则**：签名不同允许重载，签名相同报错（覆盖禁止）
4. **默认值**：字段后直接写 `= value`
5. **动态分发**：编译期类型收集 + 接口匹配，无虚表

**核心设计**：

```yaoxiang
# 接口定义
Animal: Type = {
    speak: (Self) -> String,
}

# 类型定义（内部声明）
Dog: Type = {
    x: Int = 10,
    Animal,  # 接口声明
    speak: (Self) -> String = "Woof",
}

# 外部声明（重载）
Dog.speak: (Self, volume: Int) -> String = "WOOF"

# 异构容器（动态分发）
animals: List(Animal) = [Dog.new(), Cat.new()]
animals[0].speak()  # "Woof"
```

**消除的复杂性**：
- ❌ 无 `impl` 关键字
- ❌ 无 `dyn Trait + 'a` 标注
- ❌ 无虚表（编译期类型收集 + 枚举包装）
- ❌ 无覆盖（重载规则统一）

---

## 动机

### RFC-011 的不足

RFC-011 定义了泛型系统，但没有详细说明：

| 问题 | 说明 |
|------|------|
| 接口声明语法 | 如何声明类型实现了接口？ |
| 方法实现位置 | 内部声明还是外部声明？ |
| 重载规则 | 同名方法如何处理？ |
| 默认值语法 | 字段如何设置默认值？ |
| 动态分发 | 异构容器如何实现？ |

### 设计目标

1. **简洁**：不需要 `impl` 关键字
2. **灵活**：方法实现内部或外部都支持
3. **统一**：重载规则一致
4. **方便**：默认值语法简洁
5. **零开销**：无虚表，编译期类型收集

### 与 Rust 的对比

| 特性 | Rust | YaoXiang |
|------|------|----------|
| 接口声明 | `impl Animal for Dog { ... }` | `Dog: Type = { Animal, ... }` |
| 方法实现 | 在 `impl` 块中 | 内部或外部 |
| 重载 | 不支持 | 支持（签名不同） |
| 默认值 | 需要 `#[default]` | 直接写 `= value` |
| 异构容器 | `Vec<Box<dyn Animal + 'a>>` | `List(Animal)` |
| 动态分发 | 虚表查找 | 编译期类型收集 |

---

## 提案

### 1. 接口声明

**核心规则**：在类型定义中直接写接口名，不需要 `impl` 关键字。

```yaoxiang
# 接口定义
Animal: Type = {
    speak: (Self) -> String,
}

# 类型声明实现接口
Dog: Type = {
    x: Int,
    Animal,  # 接口声明
}
```

**编译器处理**：
1. 识别 `Animal` 是接口类型
2. 检查 `Dog` 是否有 `Animal` 要求的所有方法
3. 如果通过 → 生成实现证明
4. 如果失败 → 编译错误

**语法糖等价**：

```yaoxiang
Dog: Type = {
    x: Int,
    Animal,  # 等价于展开 Animal 的方法，但保留来源标记
}

# 等价于（但保留来源信息）
Dog: Type = {
    x: Int,
    speak: (Self) -> String,  # 来自 Animal
}
```

**为什么需要来源标记**：
- 直接展开会丢失来源信息
- 来源标记用于生成实现证明
- 运行时通过证明找到正确的方法

### 2. 方法实现

**核心规则**：方法实现内部声明和外部声明都支持。

#### 2.1 内部声明

```yaoxiang
Dog: Type = {
    x: Int = 10,
    Animal,
    speak: (Self) -> String = "Woof",  # 方法实现在内部
}
```

#### 2.2 外部声明

```yaoxiang
Dog: Type = {
    x: Int,
    Animal,
}

# 方法实现在外部
Dog.speak: (Self) -> String = "Woof"
```

#### 2.3 混合声明

```yaoxiang
Dog: Type = {
    x: Int = 10,
    Animal,
    speak: (Self) -> String = "Woof",  # 部分方法在内部
}

# 部分方法在外部
Dog.play: (Self) -> Void = { ... }
```

**编译器处理**：
1. 收集所有定义（内部和外部）
2. 按签名分组（重载）
3. 检查是否有覆盖（报错）
4. 检查接口完整性
5. 生成实现证明

### 3. 重载与覆盖

**核心规则**：
- 签名不同 → 重载 → 允许
- 签名相同 → 覆盖 → 报错

#### 3.1 重载（允许）

```yaoxiang
# 参数类型不同，允许重载
Dog.speak: (Self) -> String = "Woof"
Dog.speak: (Self, volume: Int) -> String = "WOOF"
```

#### 3.2 覆盖（禁止）

```yaoxiang
# 签名完全相同，禁止覆盖
Dog.speak: (Self) -> String = "Woof"
Dog.speak: (Self) -> String = "Bark"  # ❌ 报错：覆盖不允许
```

**错误信息**：

```
错误：Dog.speak(Self) -> String 重复定义
  --> 文件2:5:1
  |
5 | Dog.speak: (Self) -> String = "Bark"
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ 重复定义
  |
  --> 文件1:3:1
  |
3 | Dog.speak: (Self) -> String = "Woof"
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ 第一个定义
```

#### 3.3 规则统一

**内部声明和外部声明遵循相同的重载/覆盖规则**：

```yaoxiang
# 内部声明
Dog: Type = {
    x: Int,
    Animal,
    speak: (Self) -> String = "Woof",
}

# 外部声明（重载，允许）
Dog.speak: (Self, volume: Int) -> String = "WOOF"

# 外部声明（覆盖，禁止）
Dog.speak: (Self) -> String = "Bark"  # ❌ 报错
```

### 4. 默认值

**核心规则**：字段后直接写 `= value`，省去构造函数。

```yaoxiang
Dog: Type = {
    x: Int = 10,  # 默认值
    y: Int = 20,  # 默认值
    Animal,
}
```

**编译器生成构造函数**：

```yaoxiang
# 所有字段都有默认值 → 生成无参构造函数
Dog.new: () -> Dog = { x: 10, y: 20 }

# 部分字段有默认值 → 生成部分参数构造函数
Dog.new: (x: Int) -> Dog = { x: x, y: 20 }
Dog.new: (y: Int) -> Dog = { x: 10, y: y }

# 全参数构造函数
Dog.new: (x: Int, y: Int) -> Dog = { x: x, y: y }
```

**外部声明默认值**：

```yaoxiang
Dog: Type = {
    x: Int,
    y: Int,
    Animal,
}

# 外部声明默认值
Dog.x: Int = 10
Dog.y: Int = 20
```

**等价于内部声明**。

### 5. 编译器实现

#### 5.1 接口描述符

```rust
// 编译器内部：接口描述符
struct InterfaceDescriptor {
    name: String,
    methods: Vec<MethodSignature>,
}
```

#### 5.2 类型定义

```rust
// 编译器内部：类型定义
struct TypeDefinition {
    name: String,
    fields: Vec<Field>,
    interface_implementations: Vec<InterfaceImplementation>,
}

// 接口实现（保留来源信息）
struct InterfaceImplementation {
    interface: InterfaceId,
    methods: HashMap<MethodId, FunctionBody>,
}
```

#### 5.3 实现证明

```rust
// 编译器内部：实现证明
struct ImplementationProof {
    type_id: TypeId,
    interface_id: InterfaceId,
    methods: Vec<MethodPointer>,
}
```

#### 5.4 编译流程

```
1. 解析类型定义，收集接口声明
2. 收集所有方法定义（内部和外部）
3. 按签名分组（重载）
4. 检查覆盖（报错）
5. 检查接口完整性
6. 生成实现证明
7. 运行时，值携带实现证明
```

### 6. 动态分发

**核心设计**：编译期类型收集 + 接口匹配，无虚表。

#### 6.1 异构容器

```yaoxiang
# 接口定义
Animal: Type = {
    speak: (Self) -> String,
}

# 类型定义
Dog: Type = {
    x: Int,
    Animal,
    speak: (Self) -> String = "Woof",
}

Cat: Type = {
    y: Int,
    Animal,
    speak: (Self) -> String = "Meow",
}

# 异构容器
animals: List(Animal) = [Dog.new(), Cat.new()]
animals[0].speak()  # "Woof"
animals[1].speak()  # "Meow"
```

#### 6.2 编译期类型收集

**核心策略：所有权追踪，增量构建。** 不是在编译期扫描所有实现了接口的类型——而是在每个 `List(Animal)` 的**所有权操作点**增量收集：

```yaoxiang
// 构造点
animals: List(Animal) = [Dog.new()]       // AnimalGroup = { Dog(Dog) }

// append 点
animals.append(Cat.new())                  // 编译器在 append 处看到 Cat → 扩展为 { Dog, Cat }
animals.append(Bird.new())                 // 再扩展 { Dog, Cat, Bird }
```

**编译器处理**（增量）：

1. 遇到 `List(I)` 第一次被构造 → 生成初始枚举（当前编译单元内已知的所有构造类型）
2. 每次 `append` / `push` / 索引赋值 → 检查值类型是否已在枚举中；不在则扩展枚举变体
3. 为最终枚举生成单态化 `match` 分发代码
4. 跨编译单元：链接时合并各单元的枚举变体集合

**自动生成的枚举**：

```yaoxiang
# 编译器自动生成（用户不感知）
AnimalGroup: Type = {
    Dog(Dog),
    Cat(Cat),
    Bird(Bird),    # ← append(Bird.new()) 触发增量扩展
}

# List(Animal) 内部等价于 List(AnimalGroup)
```

#### 6.3 接口匹配检查

**关键洞见**：接口匹配是编译期检查的，即使类型来自动态加载的插件。

```yaoxiang
# 插件系统
plugin = load_plugin("bird.so")

# 编译器检查：plugin.create_bird() 返回类型必须实现 Animal
bird: Animal = plugin.create_bird()  # 编译期检查

# 放入异构容器 —— append 点触发枚举扩展
animals: List(Animal) = [Dog.new(), Cat.new()]
animals.append(bird)                 # 编译器：(1) 验证 bird 实现了 Animal (2) 扩展枚举
```

**编译器处理**：
1. 检查 `append` 参数的返回类型
2. 验证该类型是否实现了目标接口
3. 如果通过 → 扩展枚举、允许放入
4. 如果失败 → 编译错误

#### 6.4 运行时分发

**调用流程（编译期枚举 match，ImplementationProof 已擦除）：**

```
animals[0].speak()
  ↓
编译器生成的 match:
  match animals[0] {
    AnimalGroup.Dog(d) => d.speak(),
    AnimalGroup.Cat(c) => c.speak(),
    AnimalGroup.Bird(b) => b.speak(),
  }
```

**与虚表的对比**：

| | 虚表（Rust） | 编译期枚举（YaoXiang） |
|---|---|---|
| 查找方式 | 虚表指针 → 方法指针 | 枚举 match → 直接调用 |
| 运行时开销 | 一次间接寻址 | 字符串比较/branch（可被 CPU 分支预测优化） |
| 编译期生成 | 虚表 | 枚举 + match |
| 用户标注 | 需要 `dyn Trait + 'a` | 不需要 |
| ImplementationProof | 不适用 | 编译期擦除，运行时不存在 |

**YaoXiang 的优势**：
- 不需要品牌标注
- 编译期类型安全
- 用户透明（不需要写 `dyn Animal`）
- ImplementationProof 是纯编译期概念，零运行时开销

#### 6.5 限制与范围

**当期内（单个编译单元）：** 完整支持。所有权追踪覆盖所有 `append`/构造点，枚举增量构建。

**跨编译单元：** 链接时合并各单元的枚举变体集合。设计与链接时单态化共用同一套机制（各单元生成部分枚举，链接器合并）。

**不支持：** 运行时动态类型（完全的鸭子类型）。类型集合在编译期完全已知。
---

## 用例分析

### 基本接口实现

```yaoxiang
# 接口定义
Animal: Type = {
    speak: (Self) -> String,
}

# 类型定义
Dog: Type = {
    x: Int = 10,
    Animal,
    speak: (Self) -> String = "Woof",
}

# 使用
dog = Dog.new()
dog.speak()  # "Woof"
```

### 多重接口实现

```yaoxiang
# 多个接口
Animal: Type = {
    speak: (Self) -> String,
}

Pet: Type = {
    name: (Self) -> String,
}

# 类型实现多个接口
Dog: Type = {
    x: Int = 10,
    Animal,
    Pet,
    speak: (Self) -> String = "Woof",
    name: (Self) -> String = "Buddy",
}

# 使用
dog = Dog.new()
dog.speak()  # "Woof"
dog.name()   # "Buddy"
```

### 泛型接口

```yaoxiang
# 泛型接口
Container: (T: Type) -> Type = {
    add: (self: &mut Self, item: T) -> Void,
    get: (self: &Self, index: Int) -> T,
}

# 实现泛型接口
IntList: Type = {
    data: Array(Int),
    Container(Int),
    add: (self: &mut Self, item: Int) -> Void = ...,
    get: (self: &Self, index: Int) -> Int = ...,
}
```

### 异构容器

```yaoxiang
# 接口定义
Animal: Type = {
    speak: (Self) -> String,
}

# 类型定义
Dog: Type = {
    x: Int,
    Animal,
    speak: (Self) -> String = "Woof",
}

Cat: Type = {
    y: Int,
    Animal,
    speak: (Self) -> String = "Meow",
}

# 异构容器
animals: List(Animal) = [Dog.new(), Cat.new()]

# 使用
for animal in animals {
    print(animal.speak())
}
# 输出：
# Woof
# Meow
```

### 插件系统

```yaoxiang
# 接口定义
Plugin: Type = {
    name: (Self) -> String,
    execute: (Self) -> Void,
}

# 主程序
main: () -> Void = {
    # 加载插件
    plugin1 = load_plugin("plugin1.so")
    plugin2 = load_plugin("plugin2.so")

    # 编译器检查：plugin1 和 plugin2 必须实现 Plugin 接口
    plugins: List(Plugin) = [plugin1, plugin2]

    # 执行所有插件
    for plugin in plugins {
        print(plugin.name())
        plugin.execute()
    }
}
```

---

## 权衡

### 优点

1. **简洁**：不需要 `impl` 关键字
2. **灵活**：方法实现内部或外部都支持
3. **统一**：重载规则一致
4. **方便**：默认值语法简洁
5. **零开销**：无虚表，编译期类型收集
6. **类型安全**：接口匹配是编译期检查
7. **用户透明**：不需要写 `dyn Animal + 'a`

### 缺点

1. **限制**：不支持运行时动态类型（完全的鸭子类型）
2. **编译期开销**：需要为每个接口生成枚举变体和 match 分发代码
3. **类型集合**：必须在编译期完全已知（单个编译单元内）

### 缓解措施

1. **插件系统**：通过编译期接口匹配检查支持
2. **类型集合**：所有权追踪，增量构建——在每个 `append`/构造点收集，不是全局扫描
3. **跨编译单元**：链接时合并枚举变体集合，与链接时单态化共用机制

---

## 替代方案

| 方案 | 为什么不选择 |
|------|--------------|
| `impl` 关键字 | 增加语法复杂度 |
| 虚表（`dyn Trait`） | 需要品牌标注（`'a`） |
| 完全鸭子类型 | 运行时开销，类型不安全 |
| 枚举包装（手动） | 用户负担重 |

---

## 与 RFC-009 的关系

**品牌与接口实现**：
- 接口实现在类型层，不涉及品牌
- 品牌在借用证明层（RFC-009a）
- 两者正交，互不影响

**动态分发与品牌**：
- 动态分发使用实现证明，不需要品牌标注
- 实现证明是编译期生成的，运行时零查找
- 避免了 `dyn Trait + 'a` 的复杂性


## 接口继承

接口可以包含另一个接口。**不引入新语法**——和类型声明接口使用完全相同的语法位置：

```yaoxiang
Animal: Type = {
    speak: (Self) -> String,
}

Pet: Type = {
    Animal,                       # Pet 继承 Animal — 无新关键字
    name: (Self) -> String,
}

# Dog 实现 Pet 时，必须同时满足 Animal 和 Pet 的所有方法
Dog: Type = {
    x: Int,
    Pet,
    speak: (Self) -> String = "Woof",  # 来自 Animal
    name: (Self) -> String = "Buddy",  # 来自 Pet
}
```

**设计原则：** 继承存在，但不鼓励滥用。主要组合方式是通过多个接口声明（`Dog: Type = { Animal, Pet, ... }`）。一个类型可以直接声明它满足的所有接口，不需要通过继承树来表达。接口继承仅在有明确"is-a"层级时使用。

**编译器处理：** 展开继承链。`Pet` 展开为 `{ Animal 的所有方法, name: ... }`。`Dog` 声明 `Pet` 时，编译器验证 `Dog` 同时满足 `Animal` 和 `Pet` 的全部方法。

## 默认方法实现

接口可以提供方法的默认实现。实现类型可以选择覆盖或继承默认实现：

```yaoxiang
fmt: Type = {
    display: (Self) -> String,                      # 必须实现
    debug: (Self) -> String = Self.display(),       # ✅ 引用同接口方法
    summary: (Self) -> String = f"<{Self.name}>",   # ❌ 编译错误：Self.name 不在 fmt 里
}
```

**核心约束：接口不能假设上级实现。** 默认方法只能引用同一个接口中已声明的方法。具体类型的字段或其他接口的方法对默认方法不可见——接口是一个闭合的契约，不能伸手去摸实现类型的口袋。违反此约束在**接口定义时**直接报错。

**继承可以假设下级实现：** 当接口 `Pet` 继承 `Animal` 时，`Pet` 的默认方法可以使用 `Animal` 声明的方法——因为继承了，所以保证有。

```yaoxiang
Animal: Type = {
    speak: (Self) -> String,
}

Pet: Type = {
    Animal,                                              # 继承
    name: (Self) -> String,
    introduce: (Self) -> String = Self.name() + " says " + Self.speak(),  # ✅ speak 来自继承的 Animal
}
```

**编译期行为：** 类型实现接口时，对每个方法：
1. 类型有提供 → 使用类型的方法
2. 类型未提供、接口有默认 → 编译器内联默认实现到类型上（零虚表开销）
3. 类型未提供、接口无默认 → 编译错误

**设计原则：** 默认方法类似 `Copy`/`Clone` 的自动派生机制——编译器在需要时自动生成，用户可覆盖。不引入 `virtual`/`override`/`super` 关键字。
---

## 实现阶段

| 阶段 | 内容 | 依赖 |
|------|------|------|
| Phase 1 | 接口声明语法 | RFC-011 |
| Phase 2 | 方法实现的内部/外部声明 | Phase 1 |
| Phase 3 | 重载与覆盖规则 | Phase 2 |
| Phase 4 | 默认值语法 | Phase 2 |
| Phase 5 | 接口继承 | Phase 3 |
| Phase 6 | 默认方法实现 | Phase 5 |
| Phase 7 | 实现证明生成 | Phase 6 |
| Phase 8 | 编译期类型收集 | Phase 7 |
| Phase 9 | 动态分发实现 | Phase 8 |

---

## 设计决策记录

| 决策 | 决定 | 原因 | 日期 |
|------|------|------|------|
| 接口声明语法 | 类型体内直接写接口名 | 消除 `impl` 关键字，接口声明是类型定义的自然组成部分 | 2026-06-14 |
| 动态分发 | 编译期类型收集 + 自动枚举生成 | 无虚表，零运行时查找，用户透明 | 2026-06-14 |
| 外部方法声明 | 支持 | 灵活性与内部声明等价，编译器负责跨文件收集 | 2026-06-14 |
| 覆盖 | 禁止（同签名报错） | 覆盖导致不可预测的行为，重载覆盖率所有情况 | 2026-06-14 |
| 接口继承 | 支持，无新语法 | 和类型声明接口相同的语法位置。鼓励组合（多接口声明），不鼓励深层继承树 | 2026-07-03 |
| 默认方法实现 | 支持，类似 Copy/Clone 自动派生 | 接口提供默认体，编译器在实现类型上内联；用户可覆盖。不引入 virtual/override | 2026-07-03 |
| 默认方法约束 | 接口定义时验证：只能引用同接口方法，不可假设上级实现 | 接口是闭合契约。继承可以假设下级实现，但接口不能假设实现类型的字段/方法 | 2026-07-03 |
| 类型收集策略 | 所有权追踪，增量构建——在每个 append/构造点收集 | 不是全局扫描所有实现者，是按所有权操作点增量扩展枚举 | 2026-07-03 |
| ImplementationProof | 纯编译期概念，运行时擦除 | 运行时走枚举 match 分发，证明仅用于编译期验证 | 2026-07-03 |
| 跨编译单元 | 链接时合并各单元枚举变体 | 与链接时单态化共用机制，各单元生成部分枚举，链接器合并 | 2026-07-03 |

## 开放问题

- [x] ~~接口继承（接口可以继承其他接口）~~ → 支持，无新语法。`Pet: Type = { Animal, ... }`
- [x] ~~默认方法实现（接口可以提供默认实现）~~ → 支持，类似 Copy 自动派生。接口提供 body，编译器按需内联
- [ ] 接口约束的高级用法（关联类型、GAT）
- [ ] 与闭包的交互（闭包实现接口）

---

## 参考文献

- [RFC-011: 泛型系统设计](../accepted/011-generic-type-system.md) — 父 RFC
- [RFC-009: 所有权模型设计](../accepted/009-ownership-model.md) — 所有权系统
- [RFC-009a: 借用证明管道](../accepted/009a-borrow-proof-pipeline.md) — 品牌机制
- [RFC-010: 统一类型语法](../accepted/010-unified-type-syntax.md) — 统一语法

---

## 生命周期与归宿

| 状态 | 位置 | 说明 |
|------|------|------|
| **审核中** | `docs/design/rfc/review/` | 开放社区讨论 |

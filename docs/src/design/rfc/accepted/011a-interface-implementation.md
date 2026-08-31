---
title: 'RFC-011a: 接口实现与动态分发'
status: '已接受'
author: '晨煦'
created: '2026-06-14'
updated: '2026-08-19'
group: 'rfc-011'
---

# RFC-011a: 接口实现与动态分发

> **父 RFC**: [RFC-011: 泛型系统设计](../accepted/011-generic-type-system.md)
>
> **本 RFC 补充并替代 RFC-011 §2.1-2.4 的接口约束部分。**

## 摘要

RFC-011 定义了泛型系统，但没有详细说明接口实现机制。本文档补充：

1. **接口声明**：接口是参数化类型——`(Self: Type) -> Type`，实现时传入具体类型
2. **方法实现**：内部声明和外部声明都支持
3. **重载规则**：签名不同允许重载，签名相同报错（覆盖禁止）
4. **默认值**：字段后直接写 `= value`
5. **动态分发**：编译期类型收集 + 接口匹配，无虚表

**核心设计**：

```yaoxiang
# 接口定义（参数化类型，Self 是显式类型参数）
Animal: (Self: Type) -> Type = {
    speak: (self: &Self) -> String,
}

# 类型定义（内部声明）
Dog: Type = {
    x: Int = 10,
    Animal(Dog),  # 接口实例化，Self ↦ Dog
    speak: (self: &Dog) -> String = "Woof",
}

# 外部声明（重载）
Dog.speak: (self: &Dog, volume: Int) -> String = "WOOF"

# 异构容器（动态分发）
animals: List(Animal) = [Dog.new(), Cat.new()]
animals[0].speak()  # "Woof"
```

**接收者拼写约定**（勘误 2026-08-30，配合 RFC-009 所有权语义）：

- 方法接收者跟随签名语义：`&Self` = 借用（接口的默认约定——方法调用不消费接收者），
  `&mut Self` = 可变借用，按值 `Self` = 消费接收者（Move，RFC-009）。
- impl 侧签名中的 `Self` 是 impl 类型的别名：接口 `speak: (self: &Self)` 与
  impl `(self: &Dog)` / `(self: &Self)` 均匹配（Self↦impl 类型替换后完全一致，§3）。
- 历史示例中的按值接收者拼写（`(self: Self)`）意为借用，本文档已统一迁移为
  显式 `&Self`；按值拼写从此保留"消费"语义，不再混用。

**消除的复杂性**：

- ❌ 无 `impl` 关键字
- ❌ 无 `Self` 魔法关键字（`Self` 是显式类型参数，和 `T` 没区别）
- ❌ 无 `dyn Trait + 'a` 标注
- ❌ 无虚表（编译期类型收集 + 枚举包装）
- ❌ 无覆盖（重载规则统一）

---

## 动机

### RFC-011 的不足

RFC-011 定义了泛型系统，但没有详细说明：

| 问题         | 说明                     |
| ------------ | ------------------------ |
| 接口声明语法 | 如何声明类型实现了接口？ |
| 方法实现位置 | 内部声明还是外部声明？   |
| 重载规则     | 同名方法如何处理？       |
| 默认值语法   | 字段如何设置默认值？     |
| 动态分发     | 异构容器如何实现？       |

### 设计目标

1. **简洁**：不需要 `impl` 关键字
2. **灵活**：方法实现内部或外部都支持
3. **统一**：重载规则一致
4. **方便**：默认值语法简洁
5. **零开销**：无虚表，编译期类型收集

### 与 Rust 的对比

| 特性     | Rust                          | YaoXiang                      |
| -------- | ----------------------------- | ----------------------------- |
| 接口声明 | `impl Animal for Dog { ... }` | `Dog: Type = { Animal(Dog), ... }` |
| 方法实现 | 在 `impl` 块中                | 内部或外部                    |
| 重载     | 不支持                        | 支持（签名不同）              |
| 默认值   | 需要 `#[default]`             | 直接写 `= value`              |
| 异构容器 | `Vec<Box<dyn Animal + 'a>>`   | `List(Animal)`                |
| 动态分发 | 虚表查找                      | 编译期类型收集                |
| Self 关键字 | 魔法关键字，隐式量化       | 显式类型参数，和 T 平等       |

---

## 提案

### 1. 接口声明

**核心规则**：接口是参数化类型 `(Self: Type) -> Type`，`Self` 是显式类型参数，不是魔法关键字。实现时调用接口并传入具体类型。

```yaoxiang
# 接口定义（与 RFC-011 泛型类型完全一致）
Animal: (Self: Type) -> Type = {
    speak: (self: &Self) -> String,
}

# 类型声明实现接口
Dog: Type = {
    x: Int,
    Animal(Dog),  # 实例化接口，Self ↦ Dog
}
```

**编译器处理**：

1. 识别 `Animal(Dog)` 是 `(Self: Type) -> Type` 的实例化调用
2. 执行 `Self ↦ Dog` 替换：展开 `Animal(Dog)` → `{ speak: (self: &Dog) -> String }`
3. 检查 `Dog` 是否提供了所有要求的方法（签名匹配）
4. 如果通过 → 生成实现证明
5. 如果失败 → 编译错误

**展开等价**：

```yaoxiang
Dog: Type = {
    x: Int,
    Animal(Dog),  # 展开为 Animal 的方法，保留来源标记
}

# 等价于（保留来源信息）
Dog: Type = {
    x: Int,
    speak: (self: &Dog) -> String,  # 来自 Animal，Self 已替换为 Dog
}
```

**为什么需要来源标记**：

- 直接展开会丢失来源信息
- 来源标记用于生成实现证明
- 运行时通过证明找到正确的方法

#### 1.1 Self 类型参数与类型检查时机

`Self` 是接口的显式类型参数，不是魔法关键字。`Animal: (Self: Type) -> Type` 和 `List: (T: Type) -> Type` 是同一种东西——`(Type) -> Type` 类型构造器。

**类型检查时机**：

- **接口定义时**：`{ speak: (self: &Self) -> String }` 中的 `Self` 是抽象类型参数，只做语法检查。
- **实例化点**：`Animal(Dog)` 时执行 `Self ↦ Dog`，展开后做完整类型检查（签名匹配、方法存在性）。

这避免了 RFC-011 中 `Self` 作为隐式魔法关键字的问题——`Self` 不出现在类型定义中，它只在接口参数列表中出现一次，和 `T` 完全平等。

#### 1.2 字段名与方法名的命名空间

类型的字段名和方法名共享同一个命名空间。接口展开后，如果接口方法名与类型字段名冲突，**编译报错**：

```yaoxiang
Drawable: (Self: Type) -> Type = {
    x: (self: &Self) -> Int,    // 方法叫 x
}

Point: Type = {
    x: Int,                     // 字段也叫 x
    Drawable(Point),            // ❌ 编译错误：Drawable 要求方法 x，与字段 x 冲突
}
```

字段访问 `point.x` 和方法调用 `point.x()` 在语法上无法区分。统一命名空间避免歧义。

### 2. 方法实现

**核心规则**：方法实现内部声明和外部声明都支持。

#### 2.1 内部声明

```yaoxiang
Dog: Type = {
    x: Int = 10,
    Animal(Dog),
    speak: (self: &Dog) -> String = "Woof",  # 方法实现在内部
}
```

#### 2.2 外部声明

```yaoxiang
Dog: Type = {
    x: Int,
    Animal(Dog),
}

# 方法实现在外部
Dog.speak: (self: &Dog) -> String = "Woof"
```

#### 2.3 混合声明

```yaoxiang
Dog: Type = {
    x: Int = 10,
    Animal(Dog),
    speak: (self: &Dog) -> String = "Woof",  # 部分方法在内部
}

# 部分方法在外部
Dog.play: (self: &Dog) -> Void = { ... }
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
Dog.speak: (self: &Dog) -> String = "Woof"
Dog.speak: (self: &Dog, volume: Int) -> String = "WOOF"
```

#### 3.2 覆盖（禁止）

```yaoxiang
# 签名完全相同，禁止覆盖
Dog.speak: (self: &Dog) -> String = "Woof"
Dog.speak: (self: &Dog) -> String = "Bark"  # ❌ 报错：覆盖不允许
```

**错误信息**：

```
错误：Dog.speak(self: &Dog) -> String 重复定义
  --> 文件2:5:1
  |
5 | Dog.speak: (self: &Dog) -> String = "Bark"
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ 重复定义
  |
  --> 文件1:3:1
  |
3 | Dog.speak: (self: &Dog) -> String = "Woof"
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ 第一个定义
```

#### 3.3 规则统一

**内部声明和外部声明遵循相同的重载/覆盖规则**：

```yaoxiang
# 内部声明
Dog: Type = {
    x: Int,
    Animal(Dog),
    speak: (self: &Dog) -> String = "Woof",
}

# 外部声明（重载，允许）
Dog.speak: (self: &Dog, volume: Int) -> String = "WOOF"

# 外部声明（覆盖，禁止）
Dog.speak: (self: &Dog) -> String = "Bark"  # ❌ 报错
```

### 4. 默认值

**核心规则**：字段后直接写 `= value`，省去构造函数。

```yaoxiang
Dog: Type = {
    x: Int = 10,  # 默认值
    y: Int = 20,  # 默认值
    Animal(Dog),
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
    Animal(Dog),
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
    self_param: TypeParam,     // Self 类型参数
    methods: Vec<MethodSignature>,
}
```

#### 5.2 类型定义

```rust
// 编译器内部：类型定义
struct TypeDefinition {
    name: String,
    fields: Vec<Field>,
    interface_instantiations: Vec<InterfaceInstantiation>,
}

// 接口实例化（Self ↦ ConcreteType）
struct InterfaceInstantiation {
    interface: InterfaceId,
    self_type: TypeId,          // Self 被替换为的具体类型
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
1. 解析类型定义，收集接口实例化声明（Animal(Dog)）
2. 对每个接口实例化执行 Self ↦ ConcreteType 替换
3. 展开接口方法签名，检查签名匹配
4. 收集所有方法定义（内部和外部）
5. 按签名分组（重载）
6. 检查覆盖（报错）
7. 检查接口完整性
8. 生成实现证明
```

### 6. 动态分发

**核心设计**：编译期类型收集 + 接口匹配，无虚表。

#### 6.1 异构容器

`Animal` 是 `(Self: Type) -> Type`。`List(Animal)` 将未实例化的接口类型构造器用作**存在类型**（existential）：`∃S. Animal(S)`——"存在某个类型 S，S 实现了 Animal(S)"。

```yaoxiang
# 接口定义
Animal: (Self: Type) -> Type = {
    speak: (self: &Self) -> String,
}

# 类型定义
Dog: Type = {
    x: Int,
    Animal(Dog),
    speak: (self: &Dog) -> String = "Woof",
}

Cat: Type = {
    y: Int,
    Animal(Cat),
    speak: (self: &Cat) -> String = "Meow",
}

# 异构容器 — Animal 未实例化 = 存在类型
animals: List(Animal) = [Dog.new(), Cat.new()]
animals[0].speak()  # "Woof"
animals[1].speak()  # "Meow"
```

**所有权语义**：放入异构容器是 Move 语义（RFC-009）。`Dog.new()` 被移入 `AnimalGroup::Dog` 枚举变体，原始变量不再可用。

```yaoxiang
dog = Dog.new()
animals: List(Animal) = [dog]
# dog.speak()  ← ❌ 编译错误：dog 已被 move
```

#### 6.2 编译期类型收集

**核心策略：所有权追踪，增量构建。** 不是在编译期扫描所有实现了接口的类型——而是在每个 `List(Animal)`
的**所有权操作点**增量收集：

```yaoxiang
// 构造点
animals: List(Animal) = [Dog.new()]       // AnimalGroup = { Dog(Dog) }

// append 点
animals.append(Cat.new())                  // 编译器在 append 处看到 Cat → 扩展为 { Dog, Cat }
animals.append(Bird.new())                 // 再扩展 { Dog, Cat, Bird }
```

**编译器处理**（增量）：

1. 遇到 `List(Animal)` 第一次被构造 → 生成初始枚举（当前编译单元内已知的所有构造类型）
2. 每次 `append` / `push` / 索引赋值 → 检查值类型是否已在枚举中；不在则扩展枚举变体
3. 为最终枚举生成单态化 `match` 分发代码
4. 跨编译单元：依赖 LTO（链接时优化）合并枚举变体。`Animal` 作为存在类型在编译单元边界传递时，各单元生成部分枚举变体，链接阶段合并为完整枚举。

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
bird: Animal = plugin.create_bird()  # 编译期检查，存在类型

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

**品牌投影**（与 RFC-009a 的交互）：match 的模式绑定 `AnimalGroup.Dog(d)` 在品牌树中产生 `#animals[0].Dog` 子品牌，与字段投影（`#42.field_x`）等价。`d.speak()` 创建的 `ReadToken(d)` 品牌链为 `animals → animals[0] → d → ReadToken(d)`，借用检查器通过品牌树前缀匹配验证冲突。

**下标访问的类型**：`animals[0]` 返回 `&AnimalGroup`（编译器生成的枚举类型），用户不能直接获取 `&mut Animal`。可变访问通过接口方法间接实现（如 `animals[0].mutate()` 内部展开为 `AnimalGroup::Dog(d) => d.mutate()`）。

**与虚表的对比**：

|                     | 虚表（Rust）          | 编译期枚举（YaoXiang）                     |
| ------------------- | --------------------- | ------------------------------------------ |
| 查找方式            | 虚表指针 → 方法指针   | 枚举 match → 直接调用                      |
| 运行时开销          | 一次间接寻址          | branch（可被 CPU 分支预测优化）            |
| 编译期生成          | 虚表                  | 枚举 + match                               |
| 用户标注            | 需要 `dyn Trait + 'a` | 不需要                                     |
| ImplementationProof | 不适用                | 编译期擦除，运行时不存在                   |

**YaoXiang 的优势**：

- 不需要品牌标注
- 编译期类型安全
- 用户透明（不需要写 `dyn Animal`）
- ImplementationProof 是纯编译期概念，零运行时开销

#### 6.5 限制与范围

**当期内（单个编译单元）：** 完整支持。所有权追踪覆盖所有 `append`/构造点，枚举增量构建。

**跨编译单元：**
依赖 LTO（链接时优化）合并枚举变体。`Animal` 作为存在类型（`∃S. Animal(S)`）在编译单元边界传递。各单元生成部分枚举变体，链接阶段合并。

**不支持：** 运行时动态类型（完全的鸭子类型）。类型集合在编译期完全已知。

#### 6.6 实现注记（#307 阶段 3，v1 已落地）

§6 的语义（异构容器、编译期成员检查、按实际类型分发、类型集合编译期封闭）已全部落地，
实现形态在机制层做了如下具体化：

- **类型收集**：按 `ImplementationProof` 对整个编译单元一次性收集实现类型集，替代 §6.2 的
  「逐所有权操作点增量收集」。单编译单元内两者语义等价（多出的死变体无害）；增量收集的
  价值在跨单元场景，归入 v2（见下）。
- **表示**：编译器合成 `Animal$Group` 变体类型，纯 IR/字节码/运行时工件（指令
  `CreateVariant`/`VariantTag`/`VariantPayload`，运行时值 `RuntimeValue::Enum`），
  MonoType 不感知——typecheck 层用户可见类型仍是接口名。每个进入存在类型位置的具体值
  自动包装为变体值（统一不透明表示，§6.4 语义）。
- **包装点**：typecheck 在「具体 vs 存在」判定的位置（带注解 let/调用实参/return/列表
  字面量元素）做定向走查，产出 span 键强制表；IR 生成按 span 注入包装。漏包装由运行时
  守卫响亮拒绝（`VariantTag`/`VariantPayload` 校验值必须是命名组的变体值），最坏情况是
  测试期显式运行时错误，绝不静默产出错误数据。
- **分发**：变体号比较跳转链，每臂解包负载后静态调用具体方法；RFC-004 重绑定形式
  （`Type.method = fn[n]`）按绑定位置重排后同样参与分发。
- **隔离**：遗留 trait 约束（`Drawable: Type = {..}` 式，无泛型参数）不经变体分发，行为不变。

**v1 边界（后续阶段）**：跨单元 LTO 变体合并（§6.5）；对 Group 值的模式匹配（依赖
match 对变体模式的 IR 支持）；反射交互；Move 进容器语义；`Any`/类型变量中转流与推断型
lambda 边界（兜底 = 运行时守卫）。

---

## 用例分析

### 基本接口实现

```yaoxiang
# 接口定义
Animal: (Self: Type) -> Type = {
    speak: (self: &Self) -> String,
}

# 类型定义
Dog: Type = {
    x: Int = 10,
    Animal(Dog),
    speak: (self: &Dog) -> String = "Woof",
}

# 使用
dog = Dog.new()
dog.speak()  # "Woof"
```

### 多重接口实现

```yaoxiang
# 多个接口
Animal: (Self: Type) -> Type = {
    speak: (self: &Self) -> String,
}

Pet: (Self: Type) -> Type = {
    name: (self: &Self) -> String,
}

# 类型实现多个接口
Dog: Type = {
    x: Int = 10,
    Animal(Dog),
    Pet(Dog),
    speak: (self: &Dog) -> String = "Woof",
    name: (self: &Dog) -> String = "Buddy",
}

# 使用
dog = Dog.new()
dog.speak()  # "Woof"
dog.name()   # "Buddy"
```

### 泛型接口

```yaoxiang
# 泛型接口
Container: (Self: Type, T: Type) -> Type = {
    add: (self: &mut Self, item: T) -> Void,
    get: (self: &Self, index: Int) -> T,
}

# 实现泛型接口
IntList: Type = {
    data: Array(Int),
    Container(IntList, Int),
    add: (self: &mut IntList, item: Int) -> Void = ...,
    get: (self: &IntList, index: Int) -> Int = ...,
}
```

### 异构容器

```yaoxiang
# 接口定义
Animal: (Self: Type) -> Type = {
    speak: (self: &Self) -> String,
}

# 类型定义
Dog: Type = {
    x: Int,
    Animal(Dog),
    speak: (self: &Dog) -> String = "Woof",
}

Cat: Type = {
    y: Int,
    Animal(Cat),
    speak: (self: &Cat) -> String = "Meow",
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
Plugin: (Self: Type) -> Type = {
    name: (self: &Self) -> String,
    execute: (self: &Self) -> Void,
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

| 方案                | 为什么不选择           |
| ------------------- | ---------------------- |
| `impl` 关键字       | 增加语法复杂度         |
| 虚表（`dyn Trait`） | 需要品牌标注（`'a`）   |
| 完全鸭子类型        | 运行时开销，类型不安全 |
| 枚举包装（手动）    | 用户负担重             |

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

**异构容器的所有权**：

- 放入 `List(Animal)` 是 Move 语义（RFC-009），原始变量不可再访问
- 下标访问 `animals[0]` 返回 `&AnimalGroup`（编译器生成的枚举），品牌投影链为 `animals → animals[0] → enum_variant → field`
- 可变访问通过接口方法间接实现，不暴露 `&mut AnimalGroup` 给用户

## 接口继承

接口可以包含另一个接口。**不引入新语法**——和类型声明接口使用完全相同的语法位置：

```yaoxiang
Animal: (Self: Type) -> Type = {
    speak: (self: &Self) -> String,
}

Pet: (Self: Type) -> Type = {
    Animal(Self),                       # Pet 继承 Animal — 无新关键字
    name: (self: &Self) -> String,
}

# Dog 实现 Pet 时，必须同时满足 Animal 和 Pet 的所有方法
Dog: Type = {
    x: Int,
    Pet(Dog),
    speak: (self: &Dog) -> String = "Woof",  # 来自 Animal
    name: (self: &Dog) -> String = "Buddy",  # 来自 Pet
}
```

**设计原则：**
继承存在，但不鼓励滥用。主要组合方式是通过多个接口实例化（`Dog: Type = { Animal(Dog), Pet(Dog), ... }`）。一个类型可以直接声明它满足的所有接口，不需要通过继承树来表达。接口继承仅在有明确"is-a"层级时使用。

**编译器处理：** 展开继承链。`Pet(Self)` 展开为 `{ Animal(Self) 的所有方法, name: ... }`。`Dog` 声明 `Pet(Dog)`
时，`Self ↦ Dog`，编译器验证 `Dog` 同时满足 `Animal(Dog)` 和 `Pet(Dog)` 的全部方法。

**接口继承中的 Self 替换**：`Pet: (Self: Type) -> Type = { Animal(Self), ... }` 中，`Animal(Self)` 的 `Self` 是 `Pet` 的 `Self` 参数——它会被延迟替换。当 `Dog` 实现 `Pet(Dog)` 时，`Self ↦ Dog`，`Animal(Self)` 变为 `Animal(Dog)`。这和泛型函数的参数传递语义完全一致。

## 默认方法实现

接口可以提供方法的默认实现。实现类型可以选择覆盖或继承默认实现：

```yaoxiang
fmt: (Self: Type) -> Type = {
    display: (self: &Self) -> String,                      # 必须实现
    debug: (self: &Self) -> String = self.display(),       # ✅ 引用同接口方法
    summary: (self: &Self) -> String = f"<{self.name}>",  # ❌ 编译错误：self.name 不在 fmt 里
}
```

**核心约束：接口不能假设上级实现。**
默认方法只能引用同一个接口中已声明的方法。具体类型的字段或其他接口的方法对默认方法不可见——接口是一个闭合的契约，不能伸手去摸实现类型的口袋。违反此约束在**接口定义时**直接报错。

**继承可以假设下级实现：** 当接口 `Pet(Self)` 继承 `Animal(Self)` 时，`Pet` 的默认方法可以使用 `Animal`
声明的方法——因为继承了，所以保证有。

```yaoxiang
Animal: (Self: Type) -> Type = {
    speak: (self: &Self) -> String,
}

Pet: (Self: Type) -> Type = {
    Animal(Self),                                              # 继承
    name: (self: &Self) -> String,
    introduce: (self: &Self) -> String = self.name() + " says " + self.speak(),  # ✅ speak 来自继承的 Animal
}
```

**编译期行为：** 类型实现接口时，对每个方法：

1. 类型有提供 → 使用类型的方法
2. 类型未提供、接口有默认 → 编译器内联默认实现到类型上（零虚表开销）
3. 类型未提供、接口无默认 → 编译错误

**设计原则：** 默认方法类似 `Copy`/`Clone`
的自动派生机制——编译器在需要时自动生成，用户可覆盖。不引入 `virtual`/`override`/`super` 关键字。
---

## 实现阶段

| 阶段    | 内容                    | 依赖    |
| ------- | ----------------------- | ------- |
| Phase 1 | 接口声明语法（`(Self: Type) -> Type`） + Self 类型参数 | RFC-011 |
| Phase 2 | 接口实例化（`Animal(Dog)`） + Self ↦ ConcreteType 替换 | Phase 1 |
| Phase 3 | 方法实现的内部/外部声明 | Phase 2 |
| Phase 4 | 重载与覆盖规则          | Phase 3 |
| Phase 5 | 默认值语法              | Phase 3 |
| Phase 6 | 接口继承                | Phase 4 |
| Phase 7 | 默认方法实现            | Phase 6 |
| Phase 8 | 实现证明生成            | Phase 7 |
| Phase 9 | 编译期类型收集          | Phase 8 |
| Phase 10| 动态分发实现            | Phase 9 |

---

## 设计决策记录

| 决策                | 决定                                                 | 原因                                                                        | 日期       |
| ------------------- | ---------------------------------------------------- | --------------------------------------------------------------------------- | ---------- |
| 接口声明语法        | 接口是参数化类型 `(Self: Type) -> Type`，实现时实例化 | 消除 `Self` 魔法关键字，与 RFC-011 泛型系统完全统一                         | 2026-06-14 |
| Self 类型参数       | 显式类型参数，接口定义时仅语法检查，实例化点完整检查  | 避免 HM 推断中的自由类型变量                                                | 2026-06-14 |
| 动态分发            | 编译期类型收集 + 自动枚举生成                        | 无虚表，零运行时查找，用户透明                                              | 2026-06-14 |
| 外部方法声明        | 支持                                                 | 灵活性与内部声明等价，编译器负责跨文件收集                                  | 2026-06-14 |
| 覆盖                | 禁止（同签名报错）                                   | 覆盖导致不可预测的行为，重载覆盖率所有情况                                  | 2026-06-14 |
| 接口继承            | 支持，无新语法                                       | 和类型声明接口相同的语法位置。鼓励组合（多接口实例化），不鼓励深层继承树    | 2026-07-03 |
| 默认方法实现        | 支持，类似 Copy/Clone 自动派生                       | 接口提供默认体，编译器在实现类型上内联；用户可覆盖。不引入 virtual/override | 2026-07-03 |
| 默认方法约束        | 接口定义时验证：只能引用同接口方法，不可假设上级实现 | 接口是闭合契约。继承可以假设下级实现，但接口不能假设实现类型的字段/方法     | 2026-07-03 |
| 类型收集策略        | 所有权追踪，增量构建——在每个 append/构造点收集       | 不是全局扫描所有实现者，是按所有权操作点增量扩展枚举                        | 2026-07-03 |
| ImplementationProof | 纯编译期概念，运行时擦除                             | 运行时走枚举 match 分发，证明仅用于编译期验证                               | 2026-07-03 |
| 跨编译单元          | LTO 合并枚举变体                                     | 存在类型在编译单元边界传递，各单元生成部分枚举，LTO 阶段合并                | 2026-07-03 |
| 字段/方法命名空间   | 统一命名空间，冲突报错                               | 字段访问 `point.x` 和方法调用 `point.x()` 无法语法区分，统一避免歧义       | 2026-07-03 |
| 异构容器所有权      | Move 语义，放入容器后原始变量不可用                  | 与 RFC-009 所有权模型一致                                                   | 2026-07-03 |
| 品牌投影            | match 模式绑定产生子品牌，与字段投影等价             | 与 RFC-009a 品牌树机制一致，enum 变体投影是品牌树的合法路径                 | 2026-07-03 |
| 接收者拼写约定      | `&Self` 借用 / `&mut Self` 可变借用 / 按值 = Move    | 接收者跟随签名语义（RFC-009），接口默认借用；历史按值拼写迁移为 &Self       | 2026-08-30 |

## 开放问题

- [x] ~~接口继承（接口可以继承其他接口）~~ → 支持，无新语法。`Pet: (Self: Type) -> Type = { Animal(Self), ... }`
- [x] ~~默认方法实现（接口可以提供默认实现）~~
      → 支持，类似 Copy 自动派生。接口提供 body，编译器按需内联
- [x] ~~Self 作为隐式魔法关键字~~ → 消除。`Self` 是显式类型参数，接口即 `(Self: Type) -> Type`
- [ ] 接口约束的高级用法（关联类型、GAT）—— 关联类型通过泛型接口参数实现（`Container: (Self: Type, T: Type) -> Type`），GAT 需要进一步设计
- [ ] 与闭包的交互（闭包实现接口）—— 初始策略：闭包不支持直接实现接口，需要 wrapper 类型。匿名类型的接口实现留待后续 RFC

---

## 参考文献

- [RFC-011: 泛型系统设计](../accepted/011-generic-type-system.md) — 父 RFC
- [RFC-009: 所有权模型设计](../accepted/009-ownership-model.md) — 所有权系统
- [RFC-009a: 借用证明管道](../accepted/009a-borrow-proof-pipeline.md) — 品牌机制
- [RFC-010: 统一类型语法](../accepted/010-unified-type-syntax.md) — 统一语法

---

## 生命周期与归宿

| 状态       | 位置                        | 说明         |
| ---------- | --------------------------- | ------------ |
| **已接受** | `docs/design/rfc/accepted/` | 正式设计文档 |

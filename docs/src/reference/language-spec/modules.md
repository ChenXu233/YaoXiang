# 模块系统规范

本文件定义 YaoXiang 的模块系统规范：模块如何定义、`use` 如何导入、导出面是什么、作用域如何划分。

设计依据：[RFC-029 模块语义](../../design/rfc/accepted/029-module-semantics.md)、[RFC-029g 移除 pub 关键字与自动绑定](../../design/rfc/accepted/029g-remove-pub-and-auto-bind.md)。

面向使用者的操作指南见[模块系统](../../guide/modules)。

---

## 第一章：模块定义

### 1.1 模块即文件

模块使用文件作为边界。每个 `.yx` 文件就是一个模块，没有 `module` / `mod` 声明关键字。

```yaoxiang
// math/geometry.yx
Point: Type = { x: Float, y: Float }

distance: (a: Point, b: Point) -> Float = { ... }
```

### 1.2 模块 = 一组顶层绑定

一个模块的「内容」就是它的顶层绑定集合。上例中 `math.geometry` 模块的内容等同于：

```
{ Point: Type, distance: (Point, Point) -> Float }
```

模块因此是一个**开放的 record**——没有需要逐一声明的导出清单，模块即其全部顶层绑定。

### 1.3 路径映射

模块路径按段映射到文件，**先文件后目录**：

| 模块路径       | 查找顺序                                     |
| -------------- | -------------------------------------------- |
| `a.b`          | `<base>/a/b.yx` → 未命中则 `<base>/a/b/mod.yx` |
| `a`            | `<base>/a.yx` → 未命中则 `<base>/a/mod.yx`    |

`mod.yx` 是目录入口文件的**约定**（门牌号），不是唯一入口（锁）——`a.yx` 与 `a/mod.yx` 都可承载模块 `a`。

::: danger 不要用 index.yx
Rust 习惯的 `index.yx` 在 YaoXiang 中**不被识别**。写成 `src/helper/index.yx` 时 `use helper` 会报 E5001。目录入口必须命名 `mod.yx`。
:::

两种形态同时存在会触发模块路径歧义并报错（RFC-029 §4）。

### 1.4 查找起点

`<base>` 按以下顺序尝试：

1. 导入者所在目录
2. 项目根

---

## 第二章：模块导入

`use` 是 record 解构：把目标模块的绑定引入当前作用域。

### 2.1 文法

```
Import       ::= 'use' ModuleRef ImportSpec?
ImportSpec   ::= 'as' Identifier
              |  '{' ImportItems '}'
ImportItems  ::= ImportItem (',' ImportItem)* ','?
ImportItem   ::= Identifier ('as' Identifier)?
ModuleRef    ::= Identifier ('.' Identifier)*
```

### 2.2 形态

| 语法                          | 语义                             | 示例                                      |
| ----------------------------- | -------------------------------- | ----------------------------------------- |
| `use path`                    | 引入 `path` 命名空间，用 `path.x` 访问 | `use std.io` → `io.print(...)`        |
| `use path.{x, y}`             | 把 `x`、`y` 直接绑定到当前作用域  | `use std.math.{sqrt}` → `sqrt(...)`      |
| `use path.{x as y}`           | 把 `x` 绑定为 `y`                | `use std.io.{print as say}` → `say(...)` |
| `use path as alias`           | 引入 `path` 命名空间并重命名      | `use helper as h` → `h.label(...)`        |

`use path.{a, b}` 绑定的是**同一批顶层绑定**的两条路径，因此 `use a.{Point}` 与 `use a.geometry.{Point}` 指向同一个绑定。

### 2.3 明确不采用的语法

以下写法在 RFC-029 中被显式否决，实现同样拒绝：

| 语法                             | 处置              | 实际行为                                       |
| -------------------------------- | ----------------- | ---------------------------------------------- |
| `use path.*`                     | 通配导入，不采用  | 解析失败                                       |
| `from path use item`             | Python 式，不采用 | 解析失败                                       |
| `use path.{item} as alias`       | 位置式别名，不采用 | 解析失败，提示改用内联形式 `use path.{item as alias}` |
| `use .relative`                  | 相对导入，未列入设计 | 解析失败（E0011）                           |

::: warning 位置式别名与内联别名不是一回事
`use std.io.{print} as p`（花括号**之后**加别名）被显式拒绝；`use std.io.{print as p}`（花括号**之内**）是支持的形态。编译器报错文案会指明这一点。
:::

### 2.4 名字冲突

两个模块路径指向同一名字的绑定时，导入会失败，需改用内联别名区分。

---

## 第三章：导出面

### 3.1 没有可见性机制

**本语言不引入任何可见性机制。** 没有 `pub`，没有 `private`，没有 `export`。

RFC-029 作出该裁定，RFC-029g 进一步把 `pub` 关键字从词法、AST、类型检查、死码豁免、格式化器与 LSP 中整体删除。

### 3.2 所有顶层绑定默认可导入

模块的每个顶层绑定——无论是否标注过任何修饰——对所有能写出该模块路径的代码可见。

```yaoxiang
// helper/mod.yx —— 注意 plain 没有任何修饰
plain: () -> string = "no pub keyword at all"

// 另一个模块可以直接导入它
use helper

main = () => {
    print(helper.plain())
}
```

因此**不存在「默认私有」这回事**。为导出与否争论一个绑定的可见性，在 YaoXiang 里是无效问题。

> 关于 `pub` 关键字的历史：029g 裁定删除前，`pub` 仅对死码豁免（W1001 族）起作用，**从不参与导出判定**——写 `pub` 不影响能否被导入。旧文档中「默认所有项都是私有的」的描述与实现相反。

### 3.3 方法绑定是显式的

方法不通过 `pub` 自动绑定。方法形式是**显式组合**：

```yaoxiang
Point: Type = { x: Float, y: Float }

// 显式方法：首参是 self
Point.distance: (self: &Point, other: &Point) -> Float = {
    dx = self.x - other.x
    dy = self.y - other.y
    (dx * dx + dy * dy).sqrt()
}
```

显式形式表达力严格更强（支持多位置绑定、单位绑定），是唯一的方法形式。

---

## 第四章：作用域

### 4.1 作用域层次

- **模块作用域**：文件顶层的绑定
- **块作用域**：每个 `{}` 建立一层，内层声明不泄漏到外层
- **函数作用域**：函数体内

### 4.2 声明与遮蔽

YaoXiang 没有 `let` 关键字。`x = value` 是声明还是赋值？遵循一个原则：

**赋值优先。** 声明只有一次，赋值却有百次。让高频操作走最短路径。

```
x = value:
    沿作用域链向外查找 x
      → 找到 mut x          ：赋值，OK（通过 &mut 令牌）
      → 找到 x（不可变，存活）：E2010 不可重新赋值
      → 找不到              ：在当前作用域新声明（唯一的声明路径）

mut x = value:
    → 当前作用域已存在 x ：E2002 重复定义
    → 外层作用域存在 x   ：E2013 禁止遮蔽（显式新声明不能与外层同名）
    → 无冲突              ：新可变声明
```

- **同作用域**：任何名字只能声明一次（E2002）
- **内层无 `mut`**：优先查找外层，赋值或报错
- **内层有 `mut`**：显式新声明，禁止与外层同名（E2013）

> **块是真作用域**：「沿作用域链查找」的前提是每个 `{}` 块确实建立一层——内层新声明的名字**不泄漏到外层**。

#### 同作用域

```yaoxiang
x = 10
x = 20              // E2010：'x' 不可变，不能重新赋值

mut y = 10
y = 20              // OK：同一绑定，重新赋值
mut y = 30          // E2002：'y' 已在此作用域定义（显式新声明撞名）

z = 10
mut z = 20          // E2002：'z' 已在此作用域定义（mut 不能覆盖已有声明）
```

注意 `x = 20` 报的是 **E2010**（不可重赋值）而非 E2002（重复定义）：
不写 `mut` 的 `x = value` 语义上是**赋值**（沿作用域链查找），
不是「再声明一个 x」。只有 `mut x = value` 才是显式新声明，撞名才报 E2002。

#### Move 后重新绑定

不可变变量如果拥有所有权，当它的值被 move（消耗）后，原绑定进入 **moved**
状态——名字仍然占据作用域槽位，但值已不可访问。

**moved 不是「声明判定」的输入。** 重新获得该名字的办法是**显式重声明**：

```yaoxiang
// 管道式数据流：每一步消耗旧值，产生新值
mut data = fetch()           // 显式可变声明
mut data = transform(data)   // E2002：同作用域已有 data
```

> **为何不用「已 moved → 可重声明」**：
>
> move 是**路径相关**的数据流属性——`if c { move p }` 之后 `p` 在一条路径上
> 已 move、另一条尚未，是「可能已 move」而非布尔真假。一个绑定级标记
> 结构上无法表达分支合流，把它当作「名字可重新声明」的开关会给出错误的
> 诊断（单分支 move 被当成已 move）。
>
> 真正的 move 分析在 `layers/ownership.rs`：构建函数体 CFG，以
> `Alive < Moved < Dropped` 格做数据流，**分支汇合取 max（保守）**，
> 在读取检查点报 E2014/E2018。它回答的是「这里能不能读」，
> 而「这个名字能不能再声明」由 4.2 的赋值优先规则独立回答。
>
> 两者职责不同，不应耦合。这也是旧文「已 moved 分支」从未可达的原因：
> 它依赖一个永不写入的标记。

**重新绑定靠 `mut` 显式声明：**

```yaoxiang
// 等价的显式写法
mut data1 = fetch()
data2 = transform(data1)  // data1 被 move，不可再用
mut data3 = filter(data2)
```

**语义分离：**

| 操作         | 含义                 | 机制            | 语法                          |
| ------------ | -------------------- | --------------- | ----------------------------- |
| **重新绑定** | 旧值消失，新值诞生   | move + 新声明   | `mut x = f(x)`（新名或 `mut`） |
| **原地修改** | 同一内存位置的值变化 | mut 赋值        | `mut x = v`; `x = w`          |

**约束：**

- 只有拥有所有权的值才能被 move。引用（`&T`、`&mut T`）被复制而非移动
- Move 检查是编译期完成的（CFG + 数据流），moved 状态的变量在任何表达式中被读取都会报 E2014
- 声明必须带初值：`LetStmt ::= ('mut')? Identifier (':' TypeExpr)? '=' Expr`——`x: Int` 这种纯注解声明不合文法，报 E0012

```yaoxiang
// move 后读取 → 错误
data = fetch()
result = process(data)   // data 被 move
print(data)              // E2014：'data' 已被 move，不可再使用

// 引用不会触发 move
ref_data = &value
copy1 = ref_data         // 复制引用，ref_data 仍可用
copy2 = ref_data         // OK

// 跨作用域：moved 状态穿透
data = fetch()
{
    data = transform(data)  // move 外层 data → 重新绑定（内层新声明）
    print(data)             // OK：使用内层 data
}
print(data)                 // E2014：外层 data 已被 move
```

#### 跨作用域

```yaoxiang
// 外层不可变，内层赋值 → 不可变变量不能重新赋值
x = 10
{
    x = 20          // E2010：'x' 不可变，不能重新赋值
}
{
    mut x = 20      // E2013：不能遮蔽已有变量 'x'（显式声明新绑定）
}

// 外层 mut，内层赋值 → 修改同一绑定
mut y = 10
{
    y = 20          // OK：同一绑定，通过 &mut 令牌修改
}
print(y)            // 20

// 外层 mut，内层不能声明同名
mut z = 10
{
    z = 30          // OK：同一绑定
}
{
    mut z = 30      // E2013：不能遮蔽已有变量 'z'
}

// 多层嵌套：mut 穿透所有层级
mut a = 0
{
    {
        a = 10      // OK
    }
}
print(a)            // 10

// 不可变穿透所有层级也不能重新赋值
b = 0
{
    {
        b = 10      // E2010：'b' 不可变，不能重新赋值
    }
}
```

#### for 循环

```yaoxiang
// 循环变量每次迭代是新绑定，不是修改
for i in 1..5 {
    print(i)        // OK：每次迭代绑定新值
    i = 10          // E2010：不可变循环变量，不能重新赋值
}

for mut i in 1..5 {
    i = 10          // OK：可变循环变量
}

// 循环变量不能遮蔽外层
i = 0
for i in 1..5 {     // E2013：不能遮蔽已有变量 'i'
}

// mut 外层累加器在循环体内可修改
mut sum = 0
for i in 1..5 {
    sum = sum + i   // OK：同一绑定，通过 &mut 令牌修改
}
print(sum)          // 15

// 不可变外层在循环体内不能修改
sum2 = 0
for i in 1..5 {
    sum2 = sum2 + i // E2010：'sum2' 不可变，不能重新赋值
}
```

#### 相关错误码

| 错误码 | 消息                                           | 触发场景                                       |
| ------ | ---------------------------------------------- | ---------------------------------------------- |
| E2002  | `'{name}' is already defined in this scope`    | 同作用域重复声明（无论 mut 与否）              |
| E2010  | `Cannot assign to immutable variable '{name}'` | 内层无 `mut` 赋值时，外层变量不可变且未 moved  |
| E2013  | `Cannot shadow existing variable '{name}'`     | 内层显式声明（`mut x` 或 `x: Type`）与外层同名 |
| E2014  | `'{name}' has been moved and cannot be used`   | 读取已 moved 的变量                            |

---

## 第五章：目录组织

### 5.1 结构

```
src/
├── main.yx          // 顶层文件
├── math/
│   ├── mod.yx       // 目录入口（模块 math）
│   ├── vector.yx    // 模块 math.vector
│   └── matrix.yx    // 模块 math.matrix
└── utils/
    ├── mod.yx       // 目录入口（模块 utils）
    └── string.yx    // 模块 utils.string
```

### 5.2 目录入口

目录中的 `mod.yx` 作为该目录的模块入口：

```yaoxiang
// math/mod.yx
use math.vector
use math.matrix

Vector = vector.Vector
Matrix = matrix.Matrix
```

注意这里**没有 `pub`**——`Vector`、`Matrix` 是普通顶层绑定，导入方直接可见（见 §3.2）。

---

## 第六章：诊断

| 错误码 | 含义                       | 典型触发                                     |
| ------ | -------------------------- | -------------------------------------------- |
| E5001  | 模块未找到                 | 路径写错；用了 `index.yx` 当目录入口；依赖未安装 |
| E5003  | 模块中不存在该导出项       | 导入名拼错（如 `read` 而非 `read_line`）       |

E5001 在项目模式下会附带提示：依赖缺失时建议运行 `yx install`。

# 模块系统规范

本文件定义 YaoXiang 编程语言的模块系统规范，包括模块定义、导入导出和d'd。

---

## 第一章：模块定义

### 1.1 模块基础

模块使用文件作为边界。每个 `.yx` 文件就是一个模块。

```
// 文件名即为模块名
// Math.yx
pub pi: Float = 3.14159
pub sqrt(Float) -> Float = (x) => { ... }
```

### 1.2 模块命名规则

- 模块名由文件名决定
- 文件扩展名 `.yx` 不参与模块名
- 模块名使用 PascalCase 命名

---

## 第二章：模块导入

### 2.1 导入语法

```
Import       ::= 'use' ModuleRef ImportSpec?
ImportSpec   ::= ('{' ImportItems '}') ('as' AliasList)?
              |  'as' AliasList
ImportItems  ::= Identifier (',' Identifier)* ','?
AliasList    ::= Identifier (',' Identifier)*
```

### 2.2 导入方式

| 语法 | 说明 | 示例 |
|------|------|------|
| `use path;` | 导入模块，使用最后部分访问 | `use std.io;` -> `io.print` |
| `use path.{a, b};` | 导入指定项 | `use std.io.{print};` -> `print` |
| `use path as alias;` | 导入并重命名 | `use std.io as io;` -> `io.print` |
| `use path.{i1, i2} as a, b;` | 导入指定项并重命名 | `use std.io.{print, read} as p, r;` -> `p`, `r` |

### 2.3 导入示例

```yaoxiang
// 导入整个模块
use std.io
io.print("Hello")

// 导入指定项
use std.io.{print, read}
print("Hello")

// 导入并重命名
use std.io as io_module
io_module.print("Hello")

// 导入指定项并重命名
use std.io.{print, read} as p, r
p("Hello")
```

---

## 第三章：模块导出

### 3.1 pub 关键字

使用 `pub` 关键字声明导出项：

```yaoxiang
// Math.yx
pub pi: Float = 3.14159
pub sqrt: (x: Float) -> Float = (x) => { ... }

// 私有项（不导出）
internal_value: Int = 42
```

### 3.2 导出规则

- 默认所有项都是私有的
- 使用 `pub` 声明的项可以被其他模块访问
- 私有项只能在当前模块内访问

### 3.3 pub 自动绑定

使用 `pub` 声明的函数，编译器自动绑定到同文件定义的类型：

```yaoxiang
// 使用 pub 声明，编译器自动绑定
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

// 编译器自动推断：
// 1. Point 在当前文件定义
// 2. 函数参数包含 Point
// 3. 执行 Point.distance = distance[0]

// 调用
d = distance(p1, p2)           // 函数式
d2 = p1.distance(p2)           // OOP 语法糖
```

---

## 第四章：作用域

### 4.1 模块作用域

每个模块都有自己的作用域，模块内的项默认对外不可见。

### 4.2 嵌套作用域

```yaoxiang
// 块作用域
{
    x = 10
    // x 在此作用域内可见
}
// x 在此作用域外不可见

// 函数作用域
add: (a: Int, b: Int) -> Int = {
    result = a + b
    return result
}
// result 在函数外不可见
```

### 4.3 变量声明与遮蔽

YaoXiang 没有 `let` 关键字。`x = value` 是声明还是赋值？遵循一个原则：

**赋值优先。** 声明只有一次，赋值却有百次。让高频操作走最短路径。

```
x = value:
  沿作用域链向外查找 x
    → 找到 mut x    ：赋值，OK（通过 &mut 令牌）
    → 找到 x（不可变）：E2010 不可重新赋值
    → 找不到        ：在当前作用域新声明（唯一的声明路径）

mut x = value:
    → 当前作用域已存在 x ：E2002 重复定义
    → 外层作用域存在 x   ：E2013 禁止遮蔽（显式新声明不能与外层同名）
    → 无冲突              ：新可变声明
```

- **同作用域**：任何名字只能声明一次（E2002）
- **内层无 `mut`**：优先查找外层，赋值或报错
- **内层有 `mut`**：显式新声明，禁止与外层同名（E2013）

#### 同作用域

```yaoxiang
x = 10
x = 20              // E2002：'x' 已在此作用域定义

mut y = 10
y = 20              // OK：同一绑定，重新赋值
mut y = 30          // E2002：'y' 已在此作用域定义

z = 10
mut z = 20          // E2002：'z' 已在此作用域定义（mut 不能覆盖已有声明）
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

| 错误码 | 消息 | 触发场景 |
|--------|------|----------|
| E2002 | `'{name}' is already defined in this scope` | 同作用域重复声明（无论 mut 与否） |
| E2010 | `Cannot assign to immutable variable '{name}'` | 内层无 `mut` 赋值时，外层变量不可变 |
| E2013 | `Cannot shadow existing variable '{name}'` | 内层显式声明（`mut x` 或 `x: Type`）与外层同名 |

---

## 第五章：模块组织

### 5.1 目录结构

```
src/
├── main.yx          // 主模块
├── math/
│   ├── index.yx     // 数学模块入口
│   ├── vector.yx    // 向量模块
│   └── matrix.yx    // 矩阵模块
└── utils/
    ├── index.yx     // 工具模块入口
    └── string.yx    // 字符串工具
```

### 5.2 模块入口

目录中的 `index.yx` 文件作为模块入口：

```yaoxiang
// math/index.yx
use math.vector
use math.matrix

pub Vector = vector.Vector
pub Matrix = matrix.Matrix
```

### 5.3 相对导入

```yaoxiang
// 在 math/vector.yx 中
use math.matrix  // 绝对导入
use .matrix      // 相对导入（同目录）
```

---

## 附录：模块语法速查

### A.1 模块即文件

```
// 文件名.yx 即为模块名
Import ::= 'use' ModuleRef
```

### A.2 导入导出

```yaoxiang
// 导入
use std.io
use std.io.{print, read}
use std.io as io

// 导出
pub pi: Float = 3.14159
pub sqrt: (x: Float) -> Float = (x) => { ... }
```

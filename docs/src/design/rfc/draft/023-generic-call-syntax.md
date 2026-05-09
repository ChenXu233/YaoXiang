# RFC-023：泛型调用语法 - 统一括号应用

> **状态**: 设计完成，待实现
> **作者**: 晨煦
> **创建日期**: 2026-04-22
> **最后更新**: 2026-05-09（统一 `()` 语法，消灭 `[]` 用于泛型）

## 背景

在统一类型语法（RFC-010）和泛型系统（RFC-011）的设计中，泛型调用存在语法分裂问题：类型标注用 `[]`，值构造用 `()`，用户需要记两套语法。

**本 RFC 的目标：一刀砍掉 `[]` 在泛型中的所有使用，让 `()` 成为唯一的应用语法。**

## 核心设计

### 1. 一条规则：`()` 做一切

签名里怎么写，调用就怎么写。签名用 `()`，调用也用 `()`：

```yaoxiang
# 类型构造器
List: (T: Type) -> Type = { data: Array(T), length: Int }

# 泛型函数
map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R)) = ...
```

### 2. 类型在左，值在右

`name: type = value` — Type 参数是类型信息，住在冒号左边；右边永远是具体值：

```yaoxiang
# 类型标注用 ()
numbers: List(Int) = List(1, 2, 3)
//       ^^^^^^^^   从左边的 Int 知道 T=Int
//                   右边只给值，Type 不参与

strings: List(String) = List("a", "b", "c")

# 空容器：T 从左侧来
empty: List(Int) = List()
```

### 3. 类型参数自动流动

签名中标注了 `: Type` 的参数，编译器从调用参数的已标注类型中自动获取。不需要用户手写，不需要"Type 自描述"这类概念——就是普通的类型推导：

```yaoxiang
map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R)) = ...

numbers: List(Int) = List(1, 2, 3)
f: (Int) -> String = (x) => x.to_string()

strings = map(numbers, f)
// T=Int 来自 numbers: List(Int)
// R=String 来自 f: (Int) -> String
// 用户什么都没多写
```

### 4. 值构造：从元素推断类型

```yaoxiang
// 有元素：T 从元素类型推断
x = List(1, 2, 3)       // 推断为 List(Int)
y = List("a", "b")      // 推断为 List(String)

// 空容器：T 必须从左侧注解来
z: List(Int) = List()    // T=Int 来自左侧
z2 = List()              // ❌ 编译错误：无法推断 T
```

### 5. 多层嵌套

```yaoxiang
// 类型构造
IntList: Type = List(Int)
Matrix3x3: Type = Matrix(Float, 3, 3)

// 嵌套
matrix: List(List(Int)) = List(
    List(1, 2, 3),
    List(4, 5, 6)
)
```

## 与旧语法的对比

| 场景 | 旧语法 | 新语法 |
|------|--------|--------|
| 类型标注 | `List[Int]` | `List(Int)` |
| 值构造（有元素） | `List[Int](1, 2, 3)` | `List(1, 2, 3)` |
| 值构造（空容器） | `List[Int]()` | `empty: List(Int) = List()` |
| 泛型函数调用 | `map[Int, String](list, f)` | `map(list, f)` |
| 类型构造 | `List[Int]` | `List(Int)` |

## 设计原则

### 签名即文档

用户看到签名就知道怎么调用：

```yaoxiang
# 签名
List: (T: Type) -> Type = { ... }

# 自然推导出用法
numbers: List(Int) = List(1, 2, 3)
//       ^^^^^^^^   从签名 (T: Type) 来
//                   ^^^^^^^^^^^ 从签名 = { data: Array(T), length: Int } 来
```

### 显式优于隐式——但有边界

```yaoxiang
# ✅ 类型信息在参数上已经写了
numbers: List(Int) = List(1, 2, 3)
f: (Int) -> String = (x) => x.to_string()
strings = map(numbers, f)      # T, R 从 numbers 和 f 的类型来——已经显式

# ❌ 参数类型未知时必须显式
process = map(numbers, (x) => x)  # ❌ R 无法从 (x) => x 推断，编译错误

# ✅ 解决：给参数加类型标注
process = map(numbers, (x: Int) => x.to_string())  # ✅ R=String 从返回类型推断
```

**规则：类型信息只需写一次——在参数声明时。编译器带它流动。**

## 实现要点

### 编译器处理流程

1. **解析签名**：识别签名中 `: Type` 的参数位置
2. **解析调用**：获取实参的类型
3. **类型填充**：从实参类型填充 `: Type` 参数
4. **类型检查**：验证填充后的完整类型

### 推断失败处理

```yaoxiang
# 无法推断时，给出清晰的编译错误
map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R)) = ...

x = map(List(1, 2, 3), (x) => x)
# Error: Cannot infer R from (x) => x
# Note: Add a type annotation to the function parameter or return type,
#       e.g. (x: Int) => x.to_string()
```

## 示例

### 泛型类型

```yaoxiang
# 定义
List: (T: Type) -> Type = { data: Array(T), length: Int }

Vector: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }

# 类型标注 + 值构造
numbers: List(Int) = List(1, 2, 3)
arr: Vector(Float, 3) = Vector(1.0, 2.0, 3.0)

# 空容器
empty: List(Int) = List()
```

### 泛型函数

```yaoxiang
# 定义
map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R)) = ...

zip: (T: Type, R: Type) -> ((a: T, b: R) -> (T, R)) = ...

# 使用——类型从参数自动流动
strings = map(List(1, 2, 3), (x: Int) => x.to_string())
pair = zip(1, "hello")
```

### 类型别名

```yaoxiang
IntList: Type = List(Int)
StringToInt: Type = (String) -> Int
IntMatrix3x3: Type = Matrix(Int, 3, 3)
```

## 依赖关系

- RFC-010（统一类型语法）：语法基础
- RFC-011（泛型系统）：泛型参数和实例化机制

## 验收标准

1. ✅ `()` 是唯一的泛型应用语法
2. ✅ `[]` 不在任何泛型上下文中使用
3. ✅ `name: type = value` 中 Type 在左，值在右
4. ✅ 泛型函数调用时 `: Type` 参数自动从实参类型获取
5. ✅ 无法推断时产生清晰的编译错误
6. ✅ 与 RFC-010/011 语法完全一致

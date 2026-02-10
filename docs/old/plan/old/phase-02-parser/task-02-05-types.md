# Task 2.5: 类型解析

> **优先级**: P0
> **状态**: ✅ 已实现

## 功能描述

解析类型注解和类型表达式。

## 类型语法

### 基础类型

```yaoxiang
# 标量类型
x: Int = 42
y: Float = 3.14
flag: Bool = true
name: String = "hello"
char: Char = 'a'

# 复合类型
point: Point = Point(x: 1, y: 2)
list: List[Int] = [1, 2, 3]
map: Map[String, Int] = {}
```

### 类型构造

```yaoxiang
# 函数类型
callback: (Int) -> Bool

# 元组类型
pair: (Int, String) = (42, "answer")

# 选项类型
maybe: Option[Int] = Some(42)

# 结果类型
result: Result[Int, String] = Ok(42)

# 泛型
boxed: Box[String] = Box("value")
```

### 类型定义

```yaoxiang
# 类型别名
type MyInt = Int

# 联合类型
type Color = red | green | blue

# 泛型联合类型
type Result[T, E] = ok(T) | err(E)

# 结构体类型
type Point = Point(x: Float, y: Float)
```

## 类型语法规则

```ebnf
type_annotation = ":" type_expression
type_expression  = function_type | compound_type | base_type

function_type    = "(" [type_list] ")" "->" type_expression
compound_type    = type_name "[" type_list "]"
base_type        = identifier | "(" type_expression ")"

type_list        = type_expression { "," type_expression }
type_name        = identifier ["[" type_list "]"]
```

## Type 枚举

```rust
pub enum Type {
    Name(String),                    // 基本类型名
    Int(usize),                      // Int(n) - n 位整数
    Float(usize),                    // Float(n) - n 位浮点
    Char,                            // 字符类型
    String,                          // 字符串类型
    Bool,                            // 布尔类型
    Void,                            // 空类型
    Struct(Vec<(String, Type)>),     // 结构体
    NamedStruct {                    // 命名结构体
        name: String,
        fields: Vec<(String, Type)>,
    },
    Union(Vec<(String, Option<Type>)>), // 联合类型
    Enum(Vec<String>),               // 枚举
    Variant(Vec<VariantDef>),        // 变体类型
    Tuple(Vec<Type>),                // 元组
    List(Box<Type>),                 // 列表
    Dict(Box<Type>, Box<Type>),      // 字典
    Set(Box<Type>),                  // 集合
    Fn {                             // 函数类型
        params: Vec<Type>,
        return_type: Box<Type>,
    },
    Option(Box<Type>),               // Option[T]
    Result(Box<Type>, Box<Type>),    // Result[T, E]
    Generic {                        // 泛型类型
        name: String,
        args: Vec<Type>,
    },
    Sum(Vec<Type>),                  // 和类型
}
```

## 输入示例

```rust
// Token 序列
Colon, LParen, KwInt, RParen, Arrow, KwBool
```

## 输出示例

```rust
Type::Fn {
    params: vec![Type::Int],
    return_type: Box::new(Type::Bool),
}
```

## 验收测试

```yaoxiang
# test_types.yx

# 基础类型注解
x: Int = 42
y: Float = 3.14
s: String = "hello"

# 函数类型
fn_type: (Int, Int) -> Int
fn_type = add

# 泛型类型
list: List[Int] = [1, 2, 3]
maybe: Option[String] = Some("value")
result: Result[Int, String] = Ok(42)

# 复杂泛型
map: Map[String, List[Int]] = {}

# 元组类型
pair: (Int, String) = (42, "answer")

# 类型定义
type Color = red | green | blue
type Point = Point(x: Float, y: Float)

print("Type parsing tests passed!")
```

## 相关文件

- **[`type_parser.rs`](type_parser.rs)**: 类型解析实现
- **[`ast.rs`](ast.rs:167)**: `Type`, `VariantDef` 定义
- **[`stmt.rs`](stmt.rs:107)**: `parse_type_stmt()`, `parse_type_definition()`

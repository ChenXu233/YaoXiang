# Task 2.5: 类型解析

> **优先级**: P0
> **状态**: ⏳ 待实现

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

# 泛型
result: Result[Int, String] = Ok(42)
```

## 类型语法规则

```ebnf
type_annotation = ":" type_expression
type_expression  = function_type | compound_type | base_type
function_type    = "(" [type_list] ")" "->" type_expression
compound_type    = type_name "[" type_list "]"
base_type        = identifier | "(" type_expression ")"
type_list        = type_expression { "," type_expression }
```

## 输入示例

```rust
// Token 序列
Colon, LParen, KwInt, RParen, Arrow, KwBool
```

## 输出示例

```rust
Type::Function {
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

# 复杂泛型
map: Map[String, List[Int]] = {}

print("Type parsing tests passed!")
```

## 相关文件

- **mod.rs**: parse_type_annotation(), parse_type_expression()
- **ast.rs**: Type, TypeName, GenericType

---
title: 'Lambda 式'
---

# Lambda 式

Lambda は**匿名で、手軽に定義できる関数**です。YaoXiang では、通常の関数は本質的に名前付き Lambda です。

## 構文

構文仕様に従い：

```
Lambda      ::= '(' ParamList? ')' '=>' Expr
            |  '(' ParamList? ')' '=>' Block
```

最も簡単な Lambda：

```yaoxiang
// 表达式形式的 Lambda
double = (x) => x * 2

print(double(5))   // 10
print(double(10))  // 20
```

## Lambda と関数の統一

YaoXiang の核心設計哲学は構文の統一です。**関数とは、Lambda に名前を付けたもの**です：

```yaoxiang
// 这两者完全等价：

// Lambda 形式
add = (a, b) => a + b

// 函数形式（语法糖）
add: (a: Int, b: Int) -> Int = a + b
```

最初の行は「Lambda を変数 `add` に代入する」ということで、2 行目は「`add`
という名前の関数を定義する」ということです。コンパイラはほぼ同じ方法でこれらを処理します。

## いつ Lambda を使うか

Lambda は次の二つの場面で最も適しています：

### 1. 高階関数——関数を引数として渡す

```yaoxiang
// 对列表的每个元素应用一个操作
apply_to_all: (list: List(Int), op: (Int) -> Int) -> List(Int) = {
    mut result = []
    for item in list {
        result.append(op(item))
    }
    return result
}

numbers = [1, 2, 3, 4, 5]

// 传入 Lambda
doubled = apply_to_all(numbers, (x) => x * 2)
squared = apply_to_all(numbers, (x) => x * x)

print(doubled)  // [2, 4, 6, 8, 10]
print(squared)  // [1, 4, 9, 16, 25]
```

### 2. 一時的な一回限りの操作

一度しか使わないロジックのために関数をわざわざ定義する必要はありません：

```yaoxiang
// 排序——临时定义排序规则
students = [
    {"name": "Alice", "score": 90},
    {"name": "Bob", "score": 85},
    {"name": "Charlie", "score": 92},
]

sorted_students = students.sort_by((a, b) => a["score"].compare(b["score"]))
```

## ブロック形式の Lambda

Lambda が複数行のロジックを必要とするときは、ブロック形式を使用します：

```yaoxiang
// 代码块 Lambda：可以包含多条语句
process = (data) => {
    cleaned = data.trim()
    lower = cleaned.lowercase()
    return lower
}

result = process("  Hello World  ")
print(result)  // "hello world"
```

ブロック形式では `return`
を使って値を返す必要があることに注意してください。この点は関数とまったく同じです。

## 複数引数 Lambda

```yaoxiang
// 三个参数
add_three = (x, y, z) => x + y + z
print(add_three(1, 2, 3))  // 6

// 无参 Lambda
greet = () => "Hello, YaoXiang!"
print(greet())  // "Hello, YaoXiang!"
```

## 型推論

Lambda の引数の型はコンテキストから推論できます：

```yaoxiang
// 类型从使用处推断——不需要写 (x: Int) => x * 2
apply: (op: (Int) -> Int, value: Int) -> Int = op(value)

result = apply((x) => x + 10, 5)
print(result)  // 15
```

コンパイラは `op` の型が `(Int) -> Int` であることを知っているので、Lambda `(x) => x + 10` の中の
`x` は自動的に `Int` として推論されます。

> **注意**：関数定義の規則により、引数の型はシグネチャまたは Lambda ヘッダの少なくとも一方で指定する必要があります。Lambda が引数として渡される場合、型は通常受け手のシグネチャによって提供されます。

## まとめ

| 要点         | 説明                                                   |
| ------------ | ------------------------------------------------------ |
| 構文         | `(params) => expr` または `(params) => { return ... }` |
| 本質         | 関数 = 名前付き Lambda                                 |
| 高階関数     | Lambda は引数として渡すことができる                    |
| ブロック形式 | 複数行のロジックには `{}` + `return`                   |
| 型推論       | 引数の型はコンテキストから自動推論                     |

Lambda は YaoXiang で「一時的なロジック」を表現する最も簡潔な方法です。これをマスターすれば、コードはより柔軟でコンパクトになります。

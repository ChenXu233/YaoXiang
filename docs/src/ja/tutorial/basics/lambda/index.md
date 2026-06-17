---
title: Lambda 式
---

# Lambda 式

Lambda とは**匿名の、いつでも定義できる関数**です。YaoXiang では、通常の関数は本質的に名前付きの Lambda です。

## 構文

文法仕様によると：

```
Lambda      ::= '(' ParamList? ')' '=>' Expr
            |  '(' ParamList? ')' '=>' Block
```

最も単純な Lambda：

```yaoxiang
# 式形式の Lambda
double = (x) => x * 2

println(double(5))   # 10
println(double(10))  # 20
```

## Lambda と関数の統一

YaoXiang の核となる設計哲学は構文の統一です。**関数は Lambda に名前を付けたもの**です：

```yaoxiang
# この二つは完全に等価です：

# Lambda 形式
add = (a, b) => a + b

# 関数形式（糖衣構文）
add: (a: Int, b: Int) -> Int = a + b
```

一行目は「Lambda を変数 `add` に代入する」、二行目は「`add` という名前の関数を定義する」という意味です。コンパイラは両者をほぼ同じ方法で処理します。

## どんなときに Lambda を使うか

Lambda は特に二つのシーンに最適です：

### 1. 高階関数——関数を引数として渡す

```yaoxiang
# リストの全要素に操作を適用する
apply_to_all: (list: List(Int), op: (Int) -> Int) -> List(Int) = {
    mut result = []
    for item in list {
        result.append(op(item))
    }
    return result
}

numbers = [1, 2, 3, 4, 5]

# Lambda を渡す
doubled = apply_to_all(numbers, (x) => x * 2)
squared = apply_to_all(numbers, (x) => x * x)

println(doubled)  # [2, 4, 6, 8, 10]
println(squared)  # [1, 4, 9, 16, 25]
```

### 2. 一時的な一度きりの操作

一度しか使わないロジックのためにわざわざ関数を定義する必要はありません：

```yaoxiang
# ソート——ソートルールを一時的に定義
students = [
    {"name": "Alice", "score": 90},
    {"name": "Bob", "score": 85},
    {"name": "Charlie", "score": 92},
]

sorted_students = students.sort_by((a, b) => a["score"].compare(b["score"]))
```

## ブロック形式の Lambda

Lambda が複数行のロジックを必要とするときは、ブロック形式を使います：

```yaoxiang
# ブロック Lambda：複数の文を含められる
process = (data) => {
    cleaned = data.trim()
    lower = cleaned.lowercase()
    return lower
}

result = process("  Hello World  ")
println(result)  # "hello world"
```

ブロック形式では `return` で値を返す必要がある点に注意してください。これは関数とまったく同じです。

## 多引数 Lambda

```yaoxiang
# 三つの引数
add_three = (x, y, z) => x + y + z
println(add_three(1, 2, 3))  # 6

# 無引数 Lambda
greet = () => "Hello, YaoXiang!"
println(greet())  # "Hello, YaoXiang!"
```

## 型推論

Lambda の引数の型はコンテキストから推論できます：

```yaoxiang
# 型は使用箇所から推論される——(x: Int) => x * 2 と書く必要はない
apply: (op: (Int) -> Int, value: Int) -> Int = op(value)

result = apply((x) => x + 10, 5)
println(result)  # 15
```

コンパイラは `op` の型が `(Int) -> Int` であることを知っているため、Lambda `(x) => x + 10` の中の `x` は自動的に `Int` として推論されます。

> **注意**：関数定義のルールによれば、引数の型はシグネチャまたは Lambda ヘッダの少なくとも一方で指定する必要があります。Lambda が引数として渡される場合、型は通常受け側のシグネチャから提供されます。

## まとめ

| ポイント | 説明 |
|------|------|
| 構文 | `(params) => expr` または `(params) => { return ... }` |
| 本質 | 関数 = 名前付きの Lambda |
| 高階関数 | Lambda は引数として渡せる |
| ブロック形式 | 複数行のロジックは `{}` + `return` を使う |
| 型推論 | 引数の型はコンテキストから自動推論される |

Lambda は YaoXiang で「一時的なロジック」を表現する最も簡潔な方法です。これを習得すれば、コードはより柔軟に、よりコンパクトになります。
---
title: Lambda式
---

# Lambda式

Lambdaは**匿名の、手軽に定義できる関数**です。YaoXiangでは、通常の関数は本質的に具名Lambdaです。

## 構文

構文規則によると：

```
Lambda      ::= '(' ParamList? ')' '=>' Expr
            |  '(' ParamList? ')' '=>' Block
```

最もシンプルなLambda：

```yaoxiang
// 式形式のLambda
double = (x) => x * 2

print(double(5))   // 10
print(double(10))  // 20
```

## Lambdaと関数の統一

YaoXiangのコア設計哲学は構文の統一です。**関数は名前にバインドされたLambdaです**：

```yaoxiang
// これら二つは完全に同等：

// Lambda形式
add = (a, b) => a + b

// 関数形式（糖衣構文）
add: (a: Int, b: Int) -> Int = a + b
```

1行目は「Lambdaを変数`add`に代入する」で、2行目は「`add`という名前の関数を定義する」です。コンパイラ，它们的処理方式几乎相同です。

## Lambdaを使う場面

Lambdaに最適な二つの場面があります：

### 1. 高階関数——関数をパラメータとして渡す

```yaoxiang
// リストの各要素に操作を適用する
apply_to_all: (list: List(Int), op: (Int) -> Int) -> List(Int) = {
    mut result = []
    for item in list {
        result.append(op(item))
    }
    return result
}

numbers = [1, 2, 3, 4, 5]

// Lambdaを渡す
doubled = apply_to_all(numbers, (x) => x * 2)
squared = apply_to_all(numbers, (x) => x * x)

print(doubled)  // [2, 4, 6, 8, 10]
print(squared)  // [1, 4, 9, 16, 25]
```

### 2. 一時的な一度だけの操作

一度しか使わないロジックのために特意的に関数を定義する必要はありません：

```yaoxiang
// ソート——一時的なソートルールを定義
students = [
    {"name": "Alice", "score": 90},
    {"name": "Bob", "score": 85},
    {"name": "Charlie", "score": 92},
]

sorted_students = students.sort_by((a, b) => a["score"].compare(b["score"]))
```

## コードブロック形式のLambda

Lambdaが複数行のロジックを必要とする場合、コードブロック形式を使います：

```yaoxiang
// コードブロックLambda：複数のステートメントを含むことができる
process = (data) => {
    cleaned = data.trim()
    lower = cleaned.lowercase()
    return lower
}

result = process("  Hello World  ")
print(result)  // "hello world"
```

コードブロック形式では`return`を使って値を返す必要があることに注意してください。これは関数と全く同じです。

## 複数パラメータのLambda

```yaoxiang
// 3つのパラメータ
add_three = (x, y, z) => x + y + z
print(add_three(1, 2, 3))  // 6

// パラメータなしのLambda
greet = () => "Hello, YaoXiang!"
print(greet())  // "Hello, YaoXiang!"
```

## 型推論

Lambdaのパラメータ型は文脈から推論できます：

```yaoxiang
// 型は使用箇所から推論——(x: Int) => x * 2と書く必要はない
apply: (op: (Int) -> Int, value: Int) -> Int = op(value)

result = apply((x) => x + 10, 5)
print(result)  // 15
```

コンパイラは`op`の型が`(Int) -> Int`であることを知っているため、Lambda`(x) => x + 10`の`x`は自動的に`Int`と推論されます。

> **注意**：関数定義のルールにより、パラメータ型はシグネチャまたはLambdaヘッダーの少なくとも一方で标注する必要があります。Lambdaがパラメータとして渡される場合、型は通常、受取側のシグネチャから提供されます。

## まとめ

| 要点       | 説明                                               |
| ---------- | -------------------------------------------------- |
| 構文       | `(params) => expr` または `(params) => { return ... }` |
| 本質       | 関数 = 具名Lambda                               |
| 高階関数   | Lambdaはパラメータとして渡すことができる                            |
| コードブロック形式 | 複数行のロジックは `{}` と `return` を使用                         |
| 型推論   | パラメータ型は文脈から自動的に推論                           |

LambdaはYaoXiangにおける「一時的なロジック」を表現する最も簡潔な方法です。マスターすれば、コードはより柔軟かつコンパクトになります。

---
title: Lambda 式
---

# Lambda 式

Lambda は**匿名で、その場で定義できる関数**です。YaoXiang では、通常の関数は本質的に名前付きの Lambda です。

## 構文

文法仕様に基づくと：

```
Lambda      ::= '(' ParamList? ')' '=>' Expr
            |  '(' ParamList? ')' '=>' Block
```

最もシンプルな Lambda：

```yaoxiang
// 式の形式の Lambda
double = (x) => x * 2

print(double(5))   // 10
print(double(10))  // 20
```

## Lambda と関数の統一

YaoXiang の中核となる設計思想は構文の統一です。**関数は名前に束縛された Lambda です**：

```yaoxiang
// この二つは完全に等価です：

// Lambda 形式
add = (a, b) => a + b

// 関数形式（シンタックスシュガー）
add: (a: Int, b: Int) -> Int = a + b
```

最初の行は「Lambda を変数 `add` に代入する」、二行目は「`add` という名前の関数を定義する」です。コンパイラはほぼ同じ方法でこれらを処理します。

## いつ Lambda を使うか

Lambda は特に次の二つの場面に適しています：

### 1. 高階関数——関数を引数として渡す

```yaoxiang
// リストの全要素にある操作を適用する
apply_to_all: (list: List(Int), op: (Int) -> Int) -> List(Int) = {
    mut result = []
    for item in list {
        result.append(op(item))
    }
    return result
}

numbers = [1, 2, 3, 4, 5]

// Lambda を渡す
doubled = apply_to_all(numbers, (x) => x * 2)
squared = apply_to_all(numbers, (x) => x * x)

print(doubled)  // [2, 4, 6, 8, 10]
print(squared)  // [1, 4, 9, 16, 25]
```

### 2. 一時的な一回限りの操作

一度しか使わないロジックのために関数をわざわざ定義する必要はありません：

```yaoxiang
// ソート——ソートルールを一時的に定義
students = [
    {"name": "Alice", "score": 90},
    {"name": "Bob", "score": 85},
    {"name": "Charlie", "score": 92},
]

sorted_students = students.sort_by((a, b) => a["score"].compare(b["score"]))
```

## ブロック形式の Lambda

Lambda が複数行のロジックを必要とする場合は、ブロック形式を使用します：

```yaoxiang
// ブロック Lambda：複数の文を含むことができる
process = (data) => {
    cleaned = data.trim()
    lower = cleaned.lowercase()
    return lower
}

result = process("  Hello World  ")
print(result)  // "hello world"
```

ブロック形式では `return` を使って値を返す必要がある点に注意してください。これは関数とまったく同じです。

## 複数パラメータの Lambda

```yaoxiang
// 3つのパラメータ
add_three = (x, y, z) => x + y + z
print(add_three(1, 2, 3))  // 6

// パラメータなしの Lambda
greet = () => "Hello, YaoXiang!"
print(greet())  // "Hello, YaoXiang!"
```

## 型推論

Lambda のパラメータ型はコンテキストから推論できます：

```yaoxiang
// 型は使用箇所から推論される——(x: Int) => x * 2 と書く必要はない
apply: (op: (Int) -> Int, value: Int) -> Int = op(value)

result = apply((x) => x + 10, 5)
print(result)  // 15
```

コンパイラは `op` の型が `(Int) -> Int` であることを知っているため、Lambda `(x) => x + 10` の中の `x` は自動的に `Int` と推論されます。

> **注意**：関数定義のルールによれば、パラメータ型はシグネチャまたは Lambda ヘッダの少なくとも一方で指定されている必要があります。Lambda が引数として渡される場合、型は通常受け手のシグネチャから提供されます。

## まとめ

| 要点 | 説明 |
|------|------|
| 構文 | `(params) => expr` または `(params) => { return ... }` |
| 本質 | 関数 = 名前付きの Lambda |
| 高階関数 | Lambda は引数として渡すことができる |
| ブロック形式 | 複数行のロジックには `{}` + `return` |
| 型推論 | パラメータ型はコンテキストから自動推論される |

Lambda は YaoXiang において「一時的なロジック」を表現する最も簡潔な方法です。これを習得すれば、コードはより柔軟でコンパクトなものになります。
---
title: 関数の定義と呼び出し
---

# 関数の定義と呼び出し

前の章では、変数の宣言方法を学びました。この章では、YaoXiang のコアである関数について学びます。YaoXiang の関数構文は変数宣言と同じ `name: type = value` モデルを共有しているので、馴染みやすいはずです。

## 関数は Lambda

最も重要な概念から説明します：**YaoXiang では、関数は本質的に lambda 式です**。特別な `fn` キーワードはなく、複雑な儀式もありません。関数を定義するとは、lambda に名前を付けることです。

```
#  любой関数は本質的に以下の4つの組み合わせ：
name: (params) -> Return = body
 ^       ^        ^        ^
 |       |        |        +-- 関数本体（lambda 式またはコードブロック）
 |       |        +-- 戻り値の型
 |       +-- パラメータリスト（シグネチャ）
 +-- 関数名
```

これは前の章で学んだ `name: type = value` と全く同じです——ただ，这里的「型」がたまたま関数型なだけです。

---

##  式形式：直接値を返す

最もシンプルな関数には `return` キーワードは不要です。関数体が単一の式である場合、それは直接戻り値として機能します：

```yaoxiang
//  式形式——直接値を返す、return 不要
add: (a: Int, b: Int) -> Int = a + b
square: (x: Int) -> Int = x * x
greet: (name: String) -> String = "你好, " + name
```

呼び出し：

```yaoxiang
sum = add(3, 5)          // sum = 8
sq = square(4)           // sq = 16
msg = greet("世界")       // msg = "你好, 世界"
```

これを**式形式**と呼びます。関数体が式（`{ }` コードブロックではない）である場合、その値が直接関数の戻り値になります。`return` を書く必要はなく、書いたらかえってエラーになります。

```yaoxiang
// 正しい：式が直接戻り値として機能
double: (x: Int) -> Int = x * 2

// 誤り：式形式で return を書くのは構文エラー
// double: (x: Int) -> Int = return x * 2   // ❌
```

---

## コードブロック形式：明示的な return

複数ステップの計算を含む関数の場合は、`{ }` コードブロックで関数体を囲みます。**コードブロック内では、`return` 文で値を返す必要があります**：

```yaoxiang
// コードブロック形式——return で値を返す必要がある
factorial: (n: Int) -> Int = {
    if n <= 1 {
        return 1
    }
    return n * factorial(n - 1)
}

// 計算結果
f5 = factorial(5)        // f5 = 120
```

ルールはシンプルです：**式形式は直接値を返す；コードブロック形式は明示的に `return` する必要がある**。コードブロック内で `return` を忘れると、関数はデフォルトで `Void` を返します。

```yaoxiang
// 注意：この関数にはバグがある
// bad_add: (a: Int, b: Int) -> Int = {
//     a + b   // return がない！ブロックはデフォルトで Void を返すが、シグネチャは Int を要求 → 型エラー
// }

// 正しい書き方
good_add: (a: Int, b: Int) -> Int = {
    return a + b
}
```

まとめ：

| 形式 | 構文 | 戻り値の返し方 |
|------|------|----------------|
| 式形式 | `name: ... = expr` | 式の値が直接戻り値になる |
| コードブロック形式 | `name: ... = { ... }` | `return` で明示的に返す必要がある |

---

## パラメータ定義

### 基本パラメータ

パラメータは関数シグネチャ内に書き、各パラメータに型を注釈できます：

```yaoxiang
// 2つのパラメータ、両方とも型を注釈
multiply: (a: Int, b: Int) -> Int = a * b
```

### パラメータの型は、シグネチャまたは Lambda 頭の少なくとも一方に注釈が必要

YaoXiang のルールは：**入力パラメータがある場合、パラメータの型はシグネチャまたは Lambda 頭の少なくとも一方に明示的に出現する必要があります**。両方を省略するとコンパイラに拒否されます。

```yaoxiang
// 方式1：シグネチャにパラメータの型を書く（Lambda 頭を省略）
add: (a: Int, b: Int) -> Int = a + b

// 方式2：Lambda 頭にパラメータの型を書く（シグネチャを省略）
add = (a: Int, b: Int) => a + b

// 方式3：完全形式（シグネチャ + Lambda 頭、両方ある）
add: (a: Int, b: Int) -> Int = (a, b) => a + b

// 誤り：両方に型を書かない
// add = (a, b) => a + b   // ❌ コンパイラはパラメータの型を推論できない
```

**方式1 推荐使用**——シグネチャにパラメータの型を書き、Lambda 頭を省略します。これが最も簡潔で明確な書き方です。

---

## 戻り値

関数の戻り値の型は `->` の後に書きます。`->` は関数型の印であり、省略できません（省略すると他の型としてパースされます）。

```yaoxiang
// Int を返す
add_one: (x: Int) -> Int = x + 1

// String を返す
to_string: (n: Int) -> String = n.to_string()

// Void を返す（戻り値なし）
log: (msg: String) -> Void = {
    print(msg)    // return なし、デフォルトで Void を返す
}
```

戻り値の型も省略でき、HM 型推論に任せられます：

```yaoxiang
// コンパイラが戻り値の型を Int と推論
add = (a: Int, b: Int) => a + b

// コンパイラが戻り値の型を String と推論
greet = (name: String) => "你好, " + name
```

---

## 関数呼び出し

### 位置パラメータ

最も基本的な呼び出し方法——順序대로引数を渡します：

```yaoxiang
add: (a: Int, b: Int) -> Int = a + b

result = add(1, 2)        // result = 3
```

構文規範では、関数呼び出しの形式定義は：

```
Expr '(' ArgList? ')'
```

日常言語に翻訳すると：式の後に括弧のペアを続け、括弧の中にはパラメータリストを配置できます。

### 名前付きパラメータ

順序대로引数を渡す他に、YaoXiang は**名前付きパラメータ**もサポートしています——パラメータ名で値を指定でき、順序は自由です：

```yaoxiang
// 名前付きパラメータ——パラメータ名の後にコロンを書き、その後に値を書く
result = add(a: 3, b: 5)     // result = 8
result = add(b: 5, a: 3)     // 順序は自由、結果は同じ

// 位置パラメータと混在可能だが、位置パラメータは前に書く必要がある
result = add(3, b: 5)        // OK
```

名前付きパラメータは呼び出しをより読みやすくし、パラメータが多い場合に特に便利です：

```yaoxiang
// 関数シグネチャ
send: (to: String, title: String, body: String) -> Void = {
    print("发送给: " + to)
    print("标题: " + title)
    print("正文: " + body)
}

// 名前付きパラメータで呼び出し意図が一目瞭然
send(
    to: "alice@example.com",
    title: "会议通知",
    body: "明天下午 3 点开会"
)
```

---

## パラメータなし関数

パラメータが不要な関数は、パラメータリストを省略できます：

```yaoxiang
// 完全形式：空のパラメータを明示的に宣言
hello: () -> Void = {
    print("Hello!")
}

// 最も簡潔な形式：シグネチャを省略、コンパイラが () -> Void と自動推論
hello = {
    print("Hello!")
}

// パラメータなし関数を呼び出す
hello()
```

`main` 関数が最も一般的なパラメータなし関数です：

```yaoxiang
// main 関数のいくつかの書き方

// 完全形式
main: () -> Void = {
    print("Hello, YaoXiang!")
}

// 最も簡潔な形式（推荐）
main = {
    print("Hello, YaoXiang!")
}
```

---

## 複数行関数

関数のロジックが複雑な場合は、コードブロック形式でコードを整理します。YaoXiang はスペース4つ分のインデントを強制します：

```yaoxiang
// 複数ステップの計算
calculate_stats: (numbers: List(Int)) -> Float = {
    // ローカル変数を宣言
    mut total = 0
    mut count = 0

    // ループで累積
    for n in numbers {
        total = total + n
        count = count + 1
    }

    // ゼロ除算を回避
    if count == 0 {
        return 0.0
    }

    // 平均値を返す
    return total:as(Float) / count:as(Float)
}
```

複数行関数では `#` でコメントを書いたり、`mut` ローカル変数を宣言したり、`for` や `if` でロジックを構築したりできます。

---

## `pub` と自動バインディング

モジュール内で、`pub` キーワードで宣言された関数は他のモジュールからインポートして使用できます。更有趣的是、**`pub` 関数は自動的に同じファイルで定義された型にバインディングされ**、OOP スタイルで呼び出すことができます。

```yaoxiang
// point.yx

// 型を定義
Point: Type = { x: Float, y: Float }

// pub 関数：コンパイラが自動的に Point.distance としてバインディング
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
}

// 2つの呼び出し方法がどちらも可能
p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

d1 = distance(p1, p2)       // 関数型呼び出し
d2 = p1.distance(p2)        // OOP スタイル呼び出し（糖衣構文）
```

コンパイラが `pub distance(p1: Point, p2: Point)` を見ると、`Point` が同じファイルで定義されていることを発見し、自動的に `Point.distance` のバインディングを作成します。追加の `impl` コードを書く必要はありません。

---

## クイックリファレンス

```yaoxiang
// ── 関数定義構文一覧 ──

// 式形式（最も使用頻度が高い）
add: (a: Int, b: Int) -> Int = a + b

// コードブロック形式（複数ステップのロジック）
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

// パラメータなし関数（最も簡潔）
main = { print("Hello!") }

// パラメータあり—シグネチャを省略
double = (x: Int) => x * 2

// パラメータあり—Lambda 頭を省略（推荐）
triple: (x: Int) -> Int = x * 3

// pub エクスポート + 自動バインディング
pub add: (a: Int, b: Int) -> Int = a + b

// ── 呼び出し構文 ──

result = add(1, 2)          // 位置パラメータ
result = add(a: 1, b: 2)    // 名前付きパラメータ
result = add(1, b: 2)       // 混在（位置パラメータが前）
```

---

## この章のまとめ

YaoXiang 関数のコア知識をマスターしました：

- **統一された構文**：`name: (params) -> Return = body`、変数宣言の `name: type = value` と同じ系統
- **式形式**：`= expr`、式の値が直接戻り値になる、`return` は不要
- **コードブロック形式**：`= { ...; return expr }`、ブロック内では `return` で明示的に返す必要がある
- **パラメータの型注釈**：シグネチャまたは Lambda 頭の少なくとも一方に型を書く、シグネチャに書くのが推荐
- **呼び出し**：位置パラメータまたは名前付きパラメータ、名前付きパラメータは順序が自由
- **`pub` 自動バインディング**：`pub` 関数は同じファイルの型に自動的にバインディングされ、`obj.method()` 呼び出しをサポート
- **パラメータなしが最簡**：`name = { ... }`、コンパイラが自動的に `() -> Void` と推論

次のステップでは、[制御フロー](./control-flow.md) 章引いて続き、`if`、`for`、`while` を関数で使用する方法を学べます。
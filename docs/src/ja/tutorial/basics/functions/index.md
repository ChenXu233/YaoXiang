---
title: '関数定義と呼び出し'
---

# 関数定義と呼び出し

前の章では、変数を宣言する方法を学びました。本章では YaoXiang の核心である関数を習得します。YaoXiang の関数構文は変数宣言と同じ
`name: type = value` モデルを共有しているため、見覚えがあると感じるはずです。

## 関数は Lambda

最も重要な概念を先に述べます。**YaoXiang では、関数は本質的に lambda 式です**。特別な `fn`
キーワードも、複雑な儀式もありません。関数を定義するとは、lambda に名前を付けるだけです。

```
# どんな関数も本質的には以下の 4 つの組み合わせです：
name: (params) -> Return = body
 ^       ^        ^        ^
 |       |        |        +-- 関数本体（lambda 式またはコードブロック）
 |       |        +-- 戻り値の型
 |       +-- パラメータリスト（シグネチャ）
 +-- 関数名
```

これは前の章で学んだ `name: type = value`
と完全に一致します。ここでの「型」がちょうど関数型であるというだけです。

---

## 式形式：直接的に値を返す

最も簡単な関数には `return`
キーワードは不要です。関数本体が単一の式である場合、それが直接戻り値となります：

```yaoxiang
// 表达式形式——直接返回值，无需 return
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

これを**式形式**と呼びます。関数本体が式（`{ }`
コードブロックではない）である場合、その値が直接関数の戻り値となります。`return`
を書く必要はなく、書くとむしろエラーになります。

```yaoxiang
// 正确：表达式直接作为返回值
double: (x: Int) -> Int = x * 2

// 错误：表达式形式中写 return 是语法错误
// double: (x: Int) -> Int = return x * 2   // ❌
```

---

## コードブロック形式：明示的な return

関数が複数ステップの計算を含む場合は、`{ }`
コードブロックで関数本体を囲みます。**コードブロック内では、必ず `return`
文を使って値を返す必要があります**：

```yaoxiang
// 代码块形式——必须用 return 返回值
factorial: (n: Int) -> Int = {
    if n <= 1 {
        return 1
    }
    return n * factorial(n - 1)
}

// 计算结果
f5 = factorial(5)        // f5 = 120
```

ルールはシンプルです。**式形式は直接的に値を返し、コードブロック形式は明示的に `return`
しなければならない**。コードブロック内で `return` を書き忘れた場合、関数はデフォルトで `Void`
を返します。

```yaoxiang
// 注意：这个函数有 bug
// bad_add: (a: Int, b: Int) -> Int = {
//     a + b   // 没有 return！块默认返回 Void，但签名要求 Int → 类型错误
// }

// 正确写法
good_add: (a: Int, b: Int) -> Int = {
    return a + b
}
```

まとめ：

| 形式               | 構文                  | 戻り値の方法                      |
| ------------------ | --------------------- | --------------------------------- |
| 式形式             | `name: ... = expr`    | 式の値が直接戻り値となる          |
| コードブロック形式 | `name: ... = { ... }` | `return` で明示的に返す必要がある |

---

## パラメータ定義

### 基本的なパラメータ

パラメータは関数シグネチャに記述し、それぞれのパラメータに型を注釈できます：

```yaoxiang
// 两个参数，都标注了类型
multiply: (a: Int, b: Int) -> Int = a * b
```

### パラメータの型はシグネチャまたは Lambda ヘッダのいずれかに必ず注釈する

YaoXiang のルールは次のとおりです。**入力パラメータがある場合、パラメータの型はシグネチャまたは Lambda ヘッダの少なくとも一方で明示的に記述しなければならない**。両方を省略するとコンパイラに拒否されます。

```yaoxiang
// 方式一：参数类型写在签名中（省略 Lambda 头）
add: (a: Int, b: Int) -> Int = a + b

// 方式二：参数类型写在 Lambda 头中（省略签名）
add = (a: Int, b: Int) => a + b

// 方式三：完整形式（签名 + Lambda 头都有）
add: (a: Int, b: Int) -> Int = (a, b) => a + b

// 错误：两边都不写类型
// add = (a, b) => a + b   // ❌ 编译器无法推断参数类型
```

**方式一を推奨します**――パラメータの型はシグネチャに記述し、Lambda ヘッダは省略します。これが最も簡潔で明確な書き方です。

---

## 戻り値

関数の戻り値の型は `->` の後に書きます。`->`
は関数型の記号であり、省略できません（省略すると他の型として解釈されます）。

```yaoxiang
// 返回 Int
add_one: (x: Int) -> Int = x + 1

// 返回 String
to_string: (n: Int) -> String = n.to_string()

// 返回 Void（无返回值）
log: (msg: String) -> Void = {
    print(msg)    // 无 return，默认返回 Void
}
```

戻り値の型も省略でき、HM 型推論に任せることもできます：

```yaoxiang
// 编译器推断返回类型为 Int
add = (a: Int, b: Int) => a + b

// 编译器推断返回类型为 String
greet = (name: String) => "你好, " + name
```

---

## 関数呼び出し

### 位置引数

最も基本的な呼び出し方法――順番に渡す：

```yaoxiang
add: (a: Int, b: Int) -> Int = a + b

result = add(1, 2)        // result = 3
```

言語仕様における関数呼び出しの形式定義は次のとおりです：

```
Expr '(' ArgList? ')'
```

日常用語に翻訳すると、式の後に丸括弧が続き、括弧内には引数リストを置くことができます。

### 名前付き引数

位置による渡し方の他にも、YaoXiang は**名前付き引数**をサポートしています――引数名で値を指定し、順序は問いません：

```yaoxiang
// 命名参数——参数名后面跟等号，然后是值
result = add(a = 3, b = 5)     // result = 8
result = add(b = 5, a = 3)     // 顺序任意，结果相同

// 可以和位置参数混用，但位置参数必须在前面
result = add(3, b = 5)        // OK
```

名前付き引数は呼び出しをより読みやすくし、引数が多い場合に特に有用です：

```yaoxiang
// 函数签名
send: (to: String, title: String, body: String) -> String = to + "|" + title + "|" + body

// 命名参数让调用意图一目了然
msg = send(
    to = "alice@example.com",
    title = "会议通知",
    body = "明天下午 3 点开会"
)
```

引数名の誤りや重複指定はコンパイル時にエラーとなり、暗黙的に位置引数として扱われることはありません：

```yaoxiang
// ❌ add 没有名为 c 的形参 → E1014
result = add(b = 5, c = 1)

// ❌ a 既由位置传入又由命名传入 → E1015
result = add(1, a = 2)

// ❌ 少传一个参数 → E1010
result = add(a = 1)
```

---

## 引数なし関数

引数を必要としない関数は、引数リストを省略できます：

```yaoxiang
// 完整形式：显式声明空参数
hello: () -> Void = {
    print("Hello!")
}

// 最简形式：省略签名，编译器自动推断为 () -> Void
hello = {
    print("Hello!")
}

// 调用无参函数
hello()
```

`main` 関数が最も一般的な引数なし関数です：

```yaoxiang
// main 函数的几种写法

// 完整形式
main: () -> Void = {
    print("Hello, YaoXiang!")
}

// 最简形式（推荐）
main: () -> Void = {
    print("Hello, YaoXiang!")
}
```

---

## 複数行関数

関数のロジックが複雑な場合は、コードブロック形式でコードを構成します。YaoXiang は 4 スペースのインデントを強制します：

```yaoxiang
// 多步计算
calculate_stats: (numbers: List(Int)) -> Float = {
    // 声明局部变量
    mut total = 0
    mut count = 0

    // 循环累加
    for n in numbers {
        total = total + n
        count = count + 1
    }

    // 避免除零
    if count == 0 {
        return 0.0
    }

    // 返回平均值
    return total:as(Float) / count:as(Float)
}
```

複数行関数内では `#` でコメントを書け、`mut` ローカル変数を宣言でき、`for` と `if`
でロジックを構成できます。

---

## pub と自動バインディング

モジュール内では、`pub`
キーワードで宣言された関数は他のモジュールからインポートして使用できます。さらに興味深いことに、**`pub`
関数は同じファイルで定義された型に自動的にバインドされ**、OOP スタイルで呼び出せるようになります。

```yaoxiang
// point.yx

// 定义类型
Point: Type = { x: Float, y: Float }

// pub 函数：编译器自动将其绑定为 Point.distance
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
}

// 两种调用方式都可以
p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

d1 = distance(p1, p2)       // 函数式调用
d2 = p1.distance(p2)        // OOP 风格调用（语法糖）
```

コンパイラが `pub distance(p1: Point, p2: Point)` を見て、`Point`
が同じファイルで定義されていることを認識すると、自動的に `Point.distance`
のバインディングを作成します。追加の `impl` コードを書く必要はありません。

---

## 早見表

```yaoxiang
// ── 函数定义语法一览 ──

// 表达式形式（最常用）
add: (a: Int, b: Int) -> Int = a + b

// 代码块形式（多步逻辑）
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

// 无参函数（最简）
main: () -> Void = { print("Hello!") }

// 有参—省略签名
double = (x: Int) => x * 2

// 有参—省略 Lambda 头（推荐）
triple: (x: Int) -> Int = x * 3

// pub 导出 + 自动绑定
pub add: (a: Int, b: Int) -> Int = a + b

// ── 调用语法 ──

result = add(1, 2)          // 位置参数
result = add(a = 1, b = 2)   // 命名参数
result = add(1, b = 2)      // 混用（位置在前）
```

---

## まとめ

YaoXiang 関数の核心知識を習得しました：

- **統一構文**：`name: (params) -> Return = body`、変数宣言の `name: type = value` と同源
- **式形式**：`= expr`、式の値が直接戻り値となり `return` 不要
- **コードブロック形式**：`= { ...; return expr }`、ブロック内では `return` で明示的に返す必要がある
- **パラメータの型注釈**：シグネチャまたは Lambda ヘッダの少なくとも一方で型を記述、シグネチャに書くことを推奨
- **呼び出し**：位置引数または名前付き引数、名前付き引数は順序自由
- **pub 自動バインディング**：`pub` 関数は同ファイルの型に自動バインドされ、`obj.method()`
  呼び出しをサポート
- **引数なし最簡**：`name = { ... }`、コンパイラが自動的に `() -> Void` と推論

次のステップとして、[制御フロー](./control-flow.md) の章に進み、関数内で `if`、`for`、`while`
を如何使用するかを学びましょう。

---
title: '関数定義と呼び出し'
---

# 関数定義と呼び出し

前の章では、変数の宣言方法を学びました。この章では、YaoXiang の核心である関数について説明します。YaoXiang の関数構文は変数宣言と同じ `name: type = value` モデルを共有しているので、親しみを感じることでしょう。

## 関数は Lambda

最も重要な概念を先に説明します：**YaoXiang では、関数は本质上 lambda 式です**。特殊な `fn` キーワードはなく、複雑な儀式もありません。関数を定義するとは、lambda に名前を付けることです。

```
# 任意の関数は本质上この4つの組み合わせです：
name: (params) -> Return = body
 ^       ^        ^        ^
 |       |        |        +-- 関数本体（lambda 式またはコードブロック）
 |       |        +-- 戻り値の型
 |       +-- 引数リスト（シグネチャ）
 +-- 関数名
```

これは前の章で学んだ `name: type = value` と完全に一致していますmdash;ここで言う「型」がたまたま関数型であるだけです。

---

##  式形式：直接値を返す

最も単純な関数には `return` キーワードは不要です。関数本体が単一の式である場合、それは直接戻り値として機能します：

```yaoxiang
// 式形式mdash;直接値を返す、return 不要
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

これを**式形式**と呼びます。関数本体が式（`{ }` コードブロックではない）である場合、その値が関数の戻り値として直接使用されます。`return` を書く必要はなく、書くとむしろエラーになります。

```yaoxiang
// 正しい：式が直接戻り値として機能する
double: (x: Int) -> Int = x * 2

// 誤り：式形式で return を書くと構文エラー
// double: (x: Int) -> Int = return x * 2   // ❌
```

---

## コードブロック形式：明示的な return

複数ステップの計算を含む関数の場合は、`{ }` コードブロックで関数本体を囲みます。**コードブロック内では、`return`  文で値を返さなければなりません**：

```yaoxiang
// コードブロック形式mdash;return で値を返さなければならない
factorial: (n: Int) -> Int = {
    if n <= 1 {
        return 1
    }
    return n * factorial(n - 1)
}

// 計算結果
f5 = factorial(5)        // f5 = 120
```

規則は簡単です：**式形式は直接値を返す；コードブロック形式は明示的な `return` が必要**。コードブロックで `return` を忘れた場合、関数はデフォルトで `Void` を返します。

```yaoxiang
// 注意：这个函数有 bug
// bad_add: (a: Int, b: Int) -> Int = {
//     a + b   // 没有 return！块默认返回 Void，但签名要求 Int → 类型错误
// }

// 正しい書き方
good_add: (a: Int, b: Int) -> Int = {
    return a + b
}
```

まとめ：

| 形式       | 構文                  | 戻り値の返し方               |
| ---------- | --------------------- | ------------------------ |
| 式形式 | `name: ... = expr`    | 式の値が直接戻り値として機能   |
| コードブロック形式 | `name: ... = { ... }` | `return` で明示的に返さなければならない |

---

## 引数定義

### 基本引数

引数は関数シグネチャ内に書き、各引数に型を注釈できます：

```yaoxiang
// 2つの引数、両方とも型を注釈
multiply: (a: Int, b: Int) -> Int = a * b
```

### 引数型はシグネチャまたは Lambda 頭の少なくとも一方に注釈しなければならない

YaoXiang の規則：**入力引数がある場合、引数型はシグネチャまたは Lambda 頭の少なくとも一方に明示的に出現しなければならない**。両方を省略するとコンパイラに拒否されます。

```yaoxiang
// 方法1：引数型をシグネチャに書く（Lambda 頭を省略）
add: (a: Int, b: Int) -> Int = a + b

// 方法2：引数型を Lambda 頭に書く（シグネチャを省略）
add = (a: Int, b: Int) => a + b

// 方法3：完全形式（シグネチャ + Lambda 頭両方）
add: (a: Int, b: Int) -> Int = (a, b) => a + b

// 誤り：両方に型を書かない
// add = (a, b) => a + b   // ❌ コンパイラは引数型を推論できない
```

**方法1を推奨**mdash;引数型をシグネチャに書き、Lambda 頭を省略。これが最も簡潔で明確な書き方です。

---

## 戻り値

関数の戻り値型は `->` の後に書きます。`->` は関数型の印であり、省略できません（省略すると他の型として解析されます）。

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

戻り値型も省略でき、HM 型推論に任せることもできます：

```yaoxiang
// コンパイラが戻り値型を Int と推論
add = (a: Int, b: Int) => a + b

// コンパイラが戻り値型を String と推論
greet = (name: String) => "你好, " + name
```

---

## 関数呼び出し

### 位置引数

最も基本的な呼び出し方法mdash;順序대로引数を渡す：

```yaoxiang
add: (a: Int, b: Int) -> Int = a + b

result = add(1, 2)        // result = 3
```

構文定義における関数呼び出しの形式は：

```
Expr '(' ArgList? ')'
```

日常言語に翻訳すると：式の後に括弧のペアを続け、括弧の中には引数リストを置いても良い。

### 名前付き引数

位置による引数渡しの他に、YaoXiang は**名前付き引数**をサポートmdash;引数名で値を指定、順序は自由：

```yaoxiang
// 名前付き引数mdash;引数名の後に等号、その後に値
result = add(a = 3, b = 5)     // result = 8
result = add(b = 5, a = 3)     // 順序は任意、結果は同じ

// 位置引数と混在可能だが、位置引数は前に書かなければならない
result = add(3, b = 5)        // OK
```

名前付き引数は呼び出しをより読みやすくし、引数が多いときに特に便利です：

```yaoxiang
// 関数シグネチャ
send: (to: String, title: String, body: String) -> String = to + "|" + title + "|" + body

// 名前付き引数で呼び意图が一目でわかる
msg = send(
    to = "alice@example.com",
    title = "会议通知",
    body = "明天下午 3 点开会"
)
```

引数名を間違えたり重複して指定したりするとコンパイル時にエラーになり、位置で silenciosamente 使用されることはありません：

```yaoxiang
// ❌ add には c という引数はない → E1014
result = add(b = 5, c = 1)

// ❌ a は位置でも名前付きでも渡されている → E1015
result = add(1, a = 2)

// ❌ 引数が1つ足りない → E1010
result = add(a = 1)
```

---

## 引数なし関数

引数が必要ない関数は引数リストを省略できます：

```yaoxiang
// 完全形式：空引数を明示的に宣言
hello: () -> Void = {
    print("Hello!")
}

// 最も簡潔な形式：シグネチャを省略、コンパイラが () -> Void と自動推論
hello = {
    print("Hello!")
}

// 引数なし関数を呼び出す
hello()
```

`main` 関数が最も一般的な引数なし関数です：

```yaoxiang
// main 関数のいくつかの書き方

// 完全形式
main: () -> Void = {
    print("Hello, YaoXiang!")
}

// 最も簡潔な形式（推奨）
main: () -> Void = {
    print("Hello, YaoXiang!")
}
```

---

## 複数行関数

関数のロジックが複雑な場合、コードブロック形式でコードを整理します。YaoXiang は強制的に4つのスペースによるインデントを使用します：

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

    // ゼロ除算を避ける
    if count == 0 {
        return 0.0
    }

    // 平均値を返す
    return total:as(Float) / count:as(Float)
}
```

複数行関数では `#` でコメントを書いたり、`mut` ローカル変数を宣言したり、`for` や `if` でロジックを構築したりできます。

---

## pub と自動バインディング

モジュール内で `pub` キーワードで宣言された関数は、他のモジュールからインポートして使用できます。さらに興味深いことに、**`pub` 関数は自動的に同じファイルで定義された型にバインディングされ**、OOP スタイルで呼び出すことができます。

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

// 2つの呼び出し方が可能
p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

d1 = distance(p1, p2)       // 関数型呼び出し
d2 = p1.distance(p2)        // OOP スタイル呼び出し（糖衣構文）
```

コンパイラが `pub distance(p1: Point, p2: Point)` を見ると、同じファイルに `Point` が定義されていることを発見し、自動的に `Point.distance` のバインディングを作成します。追加の `impl` コードを書く必要はありません。

---

## クイックリファレンス

```yaoxiang
// ── 関数定義構文一覧 ──

// 式形式（最も一般的）
add: (a: Int, b: Int) -> Int = a + b

// コードブロック形式（複数ステップのロジック）
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

// 引数なし関数（最も簡潔）
main: () -> Void = { print("Hello!") }

// 引数ありmdash;シグネチャを省略
double = (x: Int) => x * 2

// 引数ありmdash;Lambda 頭を省略（推奨）
triple: (x: Int) -> Int = x * 3

// pub エクスポート + 自動バインディング
pub add: (a: Int, b: Int) -> Int = a + b

// ── 呼び出し構文 ──

result = add(1, 2)          // 位置引数
result = add(a = 1, b = 2)   // 名前付き引数
result = add(1, b = 2)      // 混在（位置が前）
```

---

## まとめ

YaoXiang 関数の核心知識を習得しました：

- **統一構文**：`name: (params) -> Return = body`、変数宣言の `name: type = value` と同じ起源
- **式形式**：`= expr`、式の値が直接戻り値として機能、`return` は不要
- **コードブロック形式**：`= { ...; return expr }`、ブロック内では `return` で明示的に返さなければならない
- **引数型の注釈**：シグネチャまたは Lambda 頭の少なくとも一方に型を書く、シグネチャに書くことを推奨
- **呼び出し**：位置引数または名前付き引数、名前付き引数は順序が自由
- **pub 自動バインディング**：`pub` 関数は同じファイルの型に自動バインディングされ、`obj.method()` 呼び出しをサポート
- **引数なしの最簡形式**：`name = { ... }`、コンパイラが自動的に `() -> Void` と推論

次のステップとして、[制御フロー](../../../design/formatter/formatting-rules/control-flow.md)章進んで、`if`、`for`、`while` を関数で使用する方法を学びましょう。
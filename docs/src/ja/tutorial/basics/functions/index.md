---
title: 関数の定義と呼び出し
---

# 関数の定義と呼び出し

前の章では、変数の宣言方法を学びました。この章では、YaoXiang の核となる関数について説明します。YaoXiang の関数構文は変数宣言と同じ
`name: type = value` モデルを共有しているので、亲しみを感じることでしょう。

## 関数は Lambda

最も重要な概念を先に説明します：**YaoXiang では、関数は本质上 lambda 式です**。特別な `fn`
キーワードはなく、複雑な準備も不要です。関数を定義するとは、lambda に名前を付けることです。

```
# すべての関数は本質的にこの4つの要素の組み合わせです：
name: (params) -> Return = body
 ^       ^        ^        ^
 |       |        |        +-- 関数本体（lambda 式またはコードブロック）
 |       |        +-- 戻り値の型
 |       +-- パラメータリスト（シグネチャ）
 +-- 関数名
```

これは前の章で学んだ `name: type = value` と完全に一致していますmdash;ここでの「型」がたまたま関数型なだけです。

---

## 式形式：直接値を返す

最も単純な関数は `return` キーワードを必要としません。関数本体が単一の式である場合、それは直接戻り値として機能します：

```yaoxiang
// 式形式mdash;直接値を返す、return は不要
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

これは**式形式**と呼ばれます。関数本体が式（`{ }`
コードブロックではない）である場合、その値が関数の戻り値として直接使用されます。`return`
を書く必要はなく、書くとむしろエラーになります。

```yaoxiang
// 正しい：式が直接戻り値として機能する
double: (x: Int) -> Int = x * 2

// 間違い：式形式で return を書くと構文エラー
// double: (x: Int) -> Int = return x * 2   // ❌
```

---

## ブロック形式：明示的な return

複数ステップの計算を含む関数には、`{ }` コードブロックで関数本体を囲みます。**ブロック内では、`return`
文を使用して値を返す必要があります**：

```yaoxiang
// ブロック形式mdash;return で値を返さなければならない
factorial: (n: Int) -> Int = {
    if n <= 1 {
        return 1
    }
    return n * factorial(n - 1)
}

// 計算結果
f5 = factorial(5)        // f5 = 120
```

ルールは単純です：**式形式は直接値を返す；ブロック形式は明示的な `return` が必要**。ブロック内で
`return` を忘れると、関数はデフォルトで `Void` を返します。

```yaoxiang
// 注意：这个函数有 bug
// bad_add: (a: Int, b: Int) -> Int = {
//     a + b   // return がない！ブロックはデフォルトで Void を返すが、シグネチャは Int を要求 → 型エラー
// }

// 正しい書き方
good_add: (a: Int, b: Int) -> Int = {
    return a + b
}
```

まとめ：

| 形式       | 構文                  | 戻り値の返し方               |
| ---------- | --------------------- | ------------------------ |
| 式形式 | `name: ... = expr`    | 式の値が直接戻り値になる   |
| ブロック形式 | `name: ... = { ... }` | `return` で明示的に返す必要がある |

---

## パラメータ定義

### 基本パラメータ

パラメータは関数シグネチャ内に書き、各パラメータに型を标注できます：

```yaoxiang
// 2つのパラメータ、両方とも型を标注
multiply: (a: Int, b: Int) -> Int = a * b
```

### パラメータの型はシグネチャまたは Lambda 頭の少なくとも一方に标注する必要がある

YaoXiang のルールは：**入力パラメータがある場合、パラメータの型はシグネチャまたは Lambda 頭の少なくとも一方に明示的に出现する必要があります**。両方を省略するとコンパイラに拒否されます。

```yaoxiang
// 方法1：シグネチャにパラメータの型を書く（Lambda 頭を省略）
add: (a: Int, b: Int) -> Int = a + b

// 方法2：Lambda 頭にパラメータの型を書く（シグネチャを省略）
add = (a: Int, b: Int) => a + b

// 方法3：完全形式（シグネチャ + Lambda 頭の両方）
add: (a: Int, b: Int) -> Int = (a, b) => a + b

// 間違い：両方に型を書かない
// add = (a, b) => a + b   // ❌ コンパイラはパラメータの型を推断できない
```

**方法1を使用することを推奨します**mdash;シグネチャにパラメータの型を書き、Lambda 頭を省略します。これが最も簡潔で明確な書き方です。

---

## 戻り値

関数の戻り値の型は `->` の後に書きます。`->` は関数型の印であり、省略できません（省略すると他の型として解析されます）。

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

戻り値の型も省略でき、HM 型推論に任せることもできます：

```yaoxiang
// コンパイラが戻り値の型を Int と推断
add = (a: Int, b: Int) => a + b

// コンパイラが戻り値の型を String と推断
greet = (name: String) => "你好, " + name
```

---

## 関数呼び出し

### 位置引数

最も基本的な呼び出し方法mdash;順番に引数を渡します：

```yaoxiang
add: (a: Int, b: Int) -> Int = a + b

result = add(1, 2)        // result = 3
```

構文仕様における関数呼び出しの形式定義：

```
Expr '(' ArgList? ')'
```

日常の言葉に翻译すると：式の後に一対の括弧、その中に引数リストを置きます。

### 名前付き引数

位置による引数渡しの他に、YaoXiang は**名前付き引数**もサポートしていますmdash;パラメータ名で値を指定し、順番は自由です：

```yaoxiang
// 名前付き引数mdash;パラメータ名の後にコロンを書き、その後に値を書く
result = add(a: 3, b: 5)     // result = 8
result = add(b: 5, a: 3)     // 順番は自由、同じ結果

// 位置引数と混在可能だが、位置引数は前に書かなければならない
result = add(3, b: 5)        // OK
```

名前付き引数は呼び出しをより読みやすくし、パラメータが多いときに特に便利です：

```yaoxiang
// 関数シグネチャ
send: (to: String, title: String, body: String) -> Void = {
    print("发送给: " + to)
    print("标题: " + title)
    print("正文: " + body)
}

// 名前付き引数により呼び出しの意図が一目でわかる
send(
    to: "alice@example.com",
    title: "会议通知",
    body: "明天下午 3 点开会"
)
```

---

## 引数なし関数

パラメータを必要としない関数は、パラメータリストを省略できます：

```yaoxiang
// 完全形式：明示的に空のパラメータを宣言
hello: () -> Void = {
    print("Hello!")
}

// 最も簡略な形式：シグネチャを省略、コンパイラが () -> Void と自动推断
hello = {
    print("Hello!")
}

// 引数なし関数の呼び出し
hello()
```

`main` 関数は最も一般的な引数なし関数です：

```yaoxiang
// main 関数のいくつかの書き方

// 完全形式
main: () -> Void = {
    print("Hello, YaoXiang!")
}

// 最も簡略な形式（推奨）
main = {
    print("Hello, YaoXiang!")
}
```

---

## 複数行関数

関数ロジックが複雑な場合は、ブロック形式でコードを構成します。YaoXiang は4つのスペースによるインデントを強制します：

```yaoxiang
// 複数ステップの計算
calculate_stats: (numbers: List(Int)) -> Float = {
    // ローカル変数の宣言
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

モジュール内で `pub` キーワードで宣言された関数は、他のモジュールからインポートして使用できます。さらにおもしろいことに、**`pub`
関数は自動的に同じファイルに定義された型にバインディングされ**、OOP スタイルの呼び出しが可能になります。

```yaoxiang
// point.yx

// 型の定義
Point: Type = { x: Float, y: Float }

// pub 関数：コンパイラが Point.distance に自動バインディング
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
}

// 2種類の呼び出し方法が可能
p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

d1 = distance(p1, p2)       // 関数型呼び出し
d2 = p1.distance(p2)        // OOP スタイル呼び出し（糖衣構文）
```

コンパイラは `pub distance(p1: Point, p2: Point)` を見ると、`Point` が同じファイルで定義されていることを發現し、`Point.distance`
のバインディングを自動生成します。追加の `impl` コードを書く必要はありません。

---

## クイックリファレンス

```yaoxiang
// ── 関数定義構文一覧 ──

// 式形式（最もよく使用）
add: (a: Int, b: Int) -> Int = a + b

// ブロック形式（複数ステップのロジック）
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

// 引数なし関数（最も簡略）
main = { print("Hello!") }

// 引数ありmdash;シグネチャを省略
double = (x: Int) => x * 2

// 引数ありmdash;Lambda 頭を省略（推奨）
triple: (x: Int) -> Int = x * 3

// pub エクスポート + 自動バインディング
pub add: (a: Int, b: Int) -> Int = a + b

// ── 呼び出し構文 ──

result = add(1, 2)          // 位置引数
result = add(a: 1, b: 2)    // 名前付き引数
result = add(1, b: 2)       // 混在（位置引数は前に）
```

---

## 小結

YaoXiang 関数の核となる知識をマスターしました：

- **統一構文**：`name: (params) -> Return = body`、変数宣言の `name: type = value` と同じ起源
- **式形式**：`= expr`、式の値が直接戻り値になる、`return` は不要
- **ブロック形式**：`= { ...; return expr }`、ブロック内では `return` で明示的に返す必要がある
- **パラメータの型标注**：シグネチャまたは Lambda 頭の少なくとも一方に型を書く、シグネチャに書くことを推奨
- **呼び出し**：位置引数または名前付き引数、名前付き引数は順番が自由
- **pub 自動バインディング**：`pub` 関数は同じファイルの型に自動バインディングされ、`obj.method()` 呼び出しをサポート
- **引数なしが最も簡略**：`name = { ... }`、コンパイラが自動的に `() -> Void` と推断

次のステップとして、[制御フロー](./control-flow.md) 章進んで、関数で `if`、`for`、`while` をどのように使用するか学びましょう。

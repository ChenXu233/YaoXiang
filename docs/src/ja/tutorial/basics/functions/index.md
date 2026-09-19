---
title: '関数の定義と呼び出し'
---

# 関数の定義と呼び出し

前の章では、変数の宣言方法を学びました。この章では、YaoXiang の中核である関数を習得します。YaoXiang の関数構文は変数宣言と同じ
`name: type = value` モデルを共有しているため、既視感があるはずです。

## 関数は Lambda である

最も重要な概念を先に述べます。**YaoXiang では、関数は本質的に lambda 式です**。特別な `fn`
キーワードもなく、複雑な儀式もありません。関数を定義するとは、lambda に名前を付けるだけです。

```
# あらゆる関数は本質的に次の 4 つの組み合わせです：
name: (params) -> Return = body
 ^       ^        ^        ^
 |       |        |        +-- 関数本体（lambda 式またはコードブロック）
 |       |        +-- 戻り値の型
 |       +-- パラメータリスト（シグネチャ）
 +-- 関数名
```

これは前の章で学んだ `name: type = value`
と完全に一致します。ただ、ここでの「型」がちょうど関数型であるだけです。

---

## 式形式：直接的に値を返す

最も簡単な関数には `return`
キーワードは不要です。関数本体が単一の式である場合、それが直接的に戻り値となります。

```yaoxiang
// 式形式——直接的に値を返し、return 不要
add: (a: Int, b: Int) -> Int = a + b
square: (x: Int) -> Int = x * x
greet: (name: String) -> String = "你好, " + name
```

これらを呼び出します。

```yaoxiang
sum = add(3, 5)          // sum = 8
sq = square(4)           // sq = 16
msg = greet("世界")       // msg = "你好, 世界"
```

これは**式形式**と呼ばれます。関数本体が式である（`{ }`
コードブロックではない）場合、その値が直接的に関数の戻り値となります。`return`
を書く必要はなく、書くと逆にエラーになります。

```yaoxiang
// 正しい：式が直接的に戻り値となる
double: (x: Int) -> Int = x * 2

// 誤り：式形式で return を書くと構文エラー
// double: (x: Int) -> Int = return x * 2   // ❌
```

---

## ブロック形式：明示的な return

関数が複数ステップの計算を含む場合、`{ }` ブロックで関数本体を囲みます。**ブロック内では、`return`
文を使って値を返さなければなりません**。

```yaoxiang
// ブロック形式——return で値を返さなければならない
factorial: (n: Int) -> Int = {
    if n <= 1 {
        return 1
    }
    return n * factorial(n - 1)
}

// 計算結果
f5 = factorial(5)        // f5 = 120
```

ルールはシンプルです。**式形式は直接的に値を返し、ブロック形式は明示的に `return`
しなければならない**。ブロック内で `return` を書き忘れた場合、関数はデフォルトで `Void` を返します。

```yaoxiang
// 注意：この関数にはバグがあります
// bad_add: (a: Int, b: Int) -> Int = {
//     a + b   // return がない！ブロックはデフォルトで Void を返すが、シグネチャは Int を要求 → 型エラー
// }

// 正しい書き方
good_add: (a: Int, b: Int) -> Int = {
    return a + b
}
```

まとめ：

| 形式         | 構文                  | 戻り値の返し方                          |
| ------------ | --------------------- | --------------------------------------- |
| 式形式       | `name: ... = expr`    | 式の値が直接的に戻り値となる            |
| ブロック形式 | `name: ... = { ... }` | `return` で明示的に返さなければならない |

---

## パラメータ定義

### 基本パラメータ

パラメータは関数のシグネチャに記述し、それぞれのパラメータに型を注釈できます。

```yaoxiang
// 2 つのパラメータ、どちらも型を注釈
multiply: (a: Int, b: Int) -> Int = a * b
```

### パラメータ型はシグネチャまたは Lambda ヘッダのいずれかに注釈する必要がある

YaoXiang のルールは次のとおりです。**入力パラメータがある場合、パラメータ型はシグネチャまたは Lambda ヘッダの少なくとも一方で明示的に記述しなければならない**。両方を省略するとコンパイラに拒否されます。

```yaoxiang
// 方法 1：パラメータ型をシグネチャに記述（Lambda ヘッダを省略）
add: (a: Int, b: Int) -> Int = a + b

// 方法 2：パラメータ型を Lambda ヘッダに記述（シグネチャを省略）
add = (a: Int, b: Int) => a + b

// 方法 3：完全形式（シグネチャと Lambda ヘッダ両方にある）
add: (a: Int, b: Int) -> Int = (a, b) => a + b

// 誤り：両方に型を書かない
// add = (a, b) => a + b   // ❌ コンパイラがパラメータ型を推論できない
```

**方法 1 の使用を推奨します**——パラメータ型をシグネチャに記述し、Lambda ヘッダを省略する。これが最も簡潔で明確な書き方です。

---

## 戻り値

関数の戻り値の型は `->` の後に書きます。`->`
は関数型の記号であり、省略できません（省略すると他の型として解析されます）。

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

戻り値の型も省略でき、HM 型推論に任せられます。

```yaoxiang
// コンパイラが戻り値型を Int と推論
add = (a: Int, b: Int) => a + b

// コンパイラが戻り値型を String と推論
greet = (name: String) => "你好, " + name
```

---

## 関数呼び出し

### 位置引数

最も基本的な呼び出し方法——順番どおりに引数を渡します。

```yaoxiang
add: (a: Int, b: Int) -> Int = a + b

result = add(1, 2)        // result = 3
```

言語仕様における関数呼び出しの形式定義は次のとおりです。

```
Expr '(' ArgList? ')'
```

日常語に翻訳すると、式の後に丸括弧が続き、括弧内に引数リストを配置できます。

### 名前付き引数

位置に従って引数を渡すほか、YaoXiang は**名前付き引数**もサポートしています——引数名で値を指定し、順序は問いません。

```yaoxiang
// 名前付き引数——引数名の後にコロン、続いて値
result = add(a: 3, b: 5)     // result = 8
result = add(b: 5, a: 3)     // 順序は任意、結果は同じ

// 位置引数と混在可能、ただし位置引数を前に置く必要がある
result = add(3, b: 5)        // OK
```

名前付き引数は呼び出しをより読みやすくし、引数が多い場合に特に有用です。

```yaoxiang
// 関数のシグネチャ
send: (to: String, title: String, body: String) -> Void = {
    print("发送给: " + to)
    print("标题: " + title)
    print("正文: " + body)
}

// 名前付き引数で呼び出しの意図が一目瞭然
send(
    to: "alice@example.com",
    title: "会议通知",
    body: "明天下午 3 点开会"
)
```

---

## 引数なしの関数

引数を必要としない関数はパラメータリストを省略できます。

```yaoxiang
// 完全形式：空のパラメータを明示的に宣言
hello: () -> Void = {
    print("Hello!")
}

// 最も簡潔な形式：シグネチャを省略、コンパイラが自動的に () -> Void と推論
hello = {
    print("Hello!")
}

// 引数なしの関数を呼び出す
hello()
```

`main` 関数が最も一般的な引数なしの関数です。

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

関数のロジックが複雑な場合は、ブロック形式でコードを組み立てます。YaoXiang は 4 スペースのインデントを強制します。

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

複数行関数内では `#` でコメントを書け、`mut` ローカル変数を宣言でき、`for` と `if`
でロジックを構築できます。

---

## pub と自動バインディング

モジュール内では、`pub`
キーワードで宣言された関数を他のモジュールからインポートして使用できます。さらに興味深いことに、**`pub`
関数は同じファイルで定義された型に自動的にバインドされ**、OOP スタイルで呼び出せるようになります。

```yaoxiang
// point.yx

// 型を定義
Point: Type = { x: Float, y: Float }

// pub 関数：コンパイラが自動的に Point.distance としてバインド
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
}

// どちらの呼び出し方法も使用可能
p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

d1 = distance(p1, p2)       // 関数型呼び出し
d2 = p1.distance(p2)        // OOP スタイル呼び出し（糖衣構文）
```

コンパイラは `pub distance(p1: Point, p2: Point)` を見て、`Point`
が同じファイルで定義されていることを認識すると、自動的に `Point.distance`
のバインディングを作成します。追加の `impl` コードを書く必要はありません。

---

## クイックリファレンス

```yaoxiang
// ── 関数定義構文一覧 ──

// 式形式（最も一般的）
add: (a: Int, b: Int) -> Int = a + b

// ブロック形式（複数ステップのロジック）
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

// 引数なしの関数（最も簡潔）
main: () -> Void = { print("Hello!") }

// パラメータあり——シグネチャを省略
double = (x: Int) => x * 2

// パラメータあり——Lambda ヘッダを省略（推奨）
triple: (x: Int) -> Int = x * 3

// pub エクスポート + 自動バインディング
pub add: (a: Int, b: Int) -> Int = a + b

// ── 呼び出し構文 ──

result = add(1, 2)          // 位置引数
result = add(a: 1, b: 2)    // 名前付き引数
result = add(1, b: 2)       // 混在（位置を先に）
```

---

## まとめ

YaoXiang の関数の核となる知識を習得しました。

- **統一構文**：`name: (params) -> Return = body`、変数宣言の `name: type = value` と同源
- **式形式**：`= expr`、式の値が直接的に戻り値となり、`return` 不要
- **ブロック形式**：`= { ...; return expr }`、ブロック内では `return` で明示的に返す必要がある
- **パラメータ型の注釈**：シグネチャまたは Lambda ヘッダの少なくとも一方で型を記述、シグネチャに記述することを推奨
- **呼び出し**：位置引数または名前付き引数、名前付き引数の順序は任意
- **pub 自動バインディング**：`pub` 関数は同じファイルの型に自動的にバインドされ、`obj.method()`
  呼び出しをサポート
- **引数なし最簡**：`name = { ... }`、コンパイラが自動的に `() -> Void` と推論

次のステップとして、[制御フロー](./control-flow.md) の章に進み、関数内で `if`、`for`、`while`
をどのように使用するかを学べます。

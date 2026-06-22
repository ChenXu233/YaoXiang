---
title: 関数定義と呼び出し
---

# 関数定義と呼び出し

前章では、変数を宣言する方法を学びました。本章では YaoXiang の核心である関数を扱います。YaoXiang の関数構文は変数宣言と同じ `name: type = value` モデルを共有しているため、見覚えがあると感じるはずです。

## 関数とは Lambda である

まず最も重要な概念を述べます。**YaoXiang では、関数は本質的に lambda 式です**。特別な `fn` キーワードはなく、複雑な儀式もありません。関数を定義するとは、lambda に名前を付けるだけです。

```
# あらゆる関数は本質的に次の 4 つの組み合わせです：
name: (params) -> Return = body
 ^       ^        ^        ^
 |       |        |        +-- 関数本体（lambda 式またはコードブロック）
 |       |        +-- 戻り値の型
 |       +-- パラメータリスト（シグネチャ）
 +-- 関数名
```

これは前章で学んだ `name: type = value` と完全に一致します — 単にここの「型」がたまたま関数型であるだけです。

---

## 式形式：直接値を返す

最も単純な関数は `return` キーワードを必要としません。関数本体が単一の式であるとき、それが直接戻り値になります：

```yaoxiang
# 式形式 — 直接値を返す、return 不要
add: (a: Int, b: Int) -> Int = a + b
square: (x: Int) -> Int = x * x
greet: (name: String) -> String = "你好, " + name
```

呼び出し：

```yaoxiang
sum = add(3, 5)          # sum = 8
sq = square(4)           # sq = 16
msg = greet("世界")       # msg = "你好, 世界"
```

これを**式形式**と呼びます。関数本体が式（`{ }` コードブロックでない）である場合、その値がそのまま関数の戻り値になります。`return` を書く必要はなく、書くと逆にエラーになります。

```yaoxiang
# 正しい：式が直接戻り値になる
double: (x: Int) -> Int = x * 2

# エラー：式形式で return を書くと構文エラー
# double: (x: Int) -> Int = return x * 2   // ❌
```

---

## コードブロック形式：明示的な return

関数が複数ステップの計算を含む場合、`{ }` コードブロックで関数本体を囲みます。**コードブロック内では、`return` 文を使って値を返す必要があります**：

```yaoxiang
# コードブロック形式 — return で値を返す必要がある
factorial: (n: Int) -> Int = {
    if n <= 1 {
        return 1
    }
    return n * factorial(n - 1)
}

# 計算結果
f5 = factorial(5)        # f5 = 120
```

ルールは単純です。**式形式は直接値を返す；コードブロック形式は明示的に `return` しなければならない**。コードブロック内で `return` を書き忘れた場合、関数はデフォルトで `Void` を返します。

```yaoxiang
# 注意：この関数にはバグがある
# bad_add: (a: Int, b: Int) -> Int = {
#     a + b   // return がない！ブロックはデフォルトで Void を返すが、シグネチャは Int を要求 → 型エラー
# }

# 正しい書き方
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

### 基本的なパラメータ

パラメータは関数シグネチャ内に書き、それぞれのパラメータに型を注釈できます：

```yaoxiang
# 2 つのパラメータ、どちらも型を注釈
multiply: (a: Int, b: Int) -> Int = a * b
```

### パラメータ型はシグネチャまたは Lambda ヘッダのいずれかに注釈する必要がある

YaoXiang のルールはこうです：**入力パラメータがある場合、パラメータ型はシグネチャまたは Lambda ヘッダの少なくとも一方で明示的に出現しなければならない**。両方を省略するとコンパイラに拒否されます。

```yaoxiang
# 方法 1：パラメータ型をシグネチャに書く（Lambda ヘッダを省略）
add: (a: Int, b: Int) -> Int = a + b

# 方法 2：パラメータ型を Lambda ヘッダに書く（シグネチャを省略）
add = (a: Int, b: Int) => a + b

# 方法 3：完全な形式（シグネチャ + Lambda ヘッダ両方）
add: (a: Int, b: Int) -> Int = (a, b) => a + b

# エラー：両側に型を書かない
# add = (a, b) => a + b   // ❌ コンパイラはパラメータ型を推論できない
```

**方法 1 の使用を推奨します** — パラメータ型をシグネチャに書き、Lambda ヘッダを省略する。これは最も簡潔で明確な書き方です。

---

## 戻り値

関数の戻り値型は `->` の後に書きます。`->` は関数型の記号であり、省略できません（省略すると他の型として解釈されます）。

```yaoxiang
# Int を返す
add_one: (x: Int) -> Int = x + 1

# String を返す
to_string: (n: Int) -> String = n.to_string()

# Void を返す（戻り値なし）
log: (msg: String) -> Void = {
    println(msg)    # return なし、デフォルトで Void を返す
}
```

戻り値型も省略可能で、HM 型推論に任せることもできます：

```yaoxiang
# コンパイラが戻り値型を Int と推論
add = (a: Int, b: Int) => a + b

# コンパイラが戻り値型を String と推論
greet = (name: String) => "你好, " + name
```

---

## 関数呼び出し

### 位置引数

最も基本的な呼び出し方法 — 順番に引数を渡します：

```yaoxiang
add: (a: Int, b: Int) -> Int = a + b

result = add(1, 2)        # result = 3
```

言語仕様における関数呼び出しの形式定義は次のとおりです：

```
Expr '(' ArgList? ')'
```

日常語に翻訳すると：式の後に括弧が続き、括弧内には引数リストを入れることができます。

### 名前付き引数

位置による引数渡しに加え、YaoXiang は**名前付き引数**もサポートしています — 引数名で値を指定し、順序は自由です：

```yaoxiang
# 名前付き引数 — 引数名 + コロン + 値
result = add(a: 3, b: 5)     # result = 8
result = add(b: 5, a: 3)     # 順序自由、結果は同じ

# 位置引数と混在可能だが、位置引数は前に置く必要がある
result = add(3, b: 5)        # OK
```

名前付き引数は呼び出しをより読みやすくし、引数が多い場合に特に有用です：

```yaoxiang
# 関数シグネチャ
send: (to: String, title: String, body: String) -> Void = {
    println("发送给: " + to)
    println("标题: " + title)
    println("正文: " + body)
}

# 名前付き引数で呼び出し意図が一目瞭然
send(
    to: "alice@example.com",
    title: "会议通知",
    body: "明天下午 3 点开会"
)
```

---

## 引数なし関数

引数を必要としない関数はパラメータリストを省略できます：

```yaoxiang
# 完全形式：空のパラメータを明示的に宣言
hello: () -> Void = {
    println("Hello!")
}

# 最も簡潔な形式：シグネチャを省略、コンパイラが自動的に () -> Void と推論
hello = {
    println("Hello!")
}

# 引数なし関数の呼び出し
hello()
```

`main` 関数が最も一般的な引数なし関数です：

```yaoxiang
# main 関数のいくつかの書き方

# 完全形式
main: () -> Void = {
    println("Hello, YaoXiang!")
}

# 最も簡潔な形式（推奨）
main = {
    println("Hello, YaoXiang!")
}
```

---

## 複数行関数

関数のロジックが複雑な場合、コードブロック形式でコードを整理します。YaoXiang は 4 スペースのインデントを強制します：

```yaoxiang
# 複数ステップの計算
calculate_stats: (numbers: List(Int)) -> Float = {
    # 局所変数の宣言
    mut total = 0
    mut count = 0

    # ループで累積
    for n in numbers {
        total = total + n
        count = count + 1
    }

    # ゼロ除算を回避
    if count == 0 {
        return 0.0
    }

    # 平均値を返す
    return total:as(Float) / count:as(Float)
}
```

複数行関数内では `#` でコメントを書け、`mut` 局所変数を宣言でき、`for` や `if` でロジックを構築できます。

---

## pub と自動バインディング

モジュール内では、`pub` キーワードで宣言された関数は他のモジュールからインポートして使用できます。さらに興味深いことに、**`pub` 関数は同じファイルで定義された型に自動的にバインドされ**、OOP スタイルで呼び出せるようになります。

```yaoxiang
# point.yx

# 型を定義
type Point = { x: Float, y: Float }

# pub 関数：コンパイラが自動的に Point.distance としてバインド
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
}

# どちらの呼び出し方も可能
p1 = Point(x: 3.0, y: 4.0)
p2 = Point(x: 1.0, y: 2.0)

d1 = distance(p1, p2)       # 関数型呼び出し
d2 = p1.distance(p2)        # OOP スタイル呼び出し（シンタックスシュガー）
```

コンパイラが `pub distance(p1: Point, p2: Point)` を見て、`Point` が同じファイルで定義されていることを発見すると、自動的に `Point.distance` のバインディングを作成します。追加の `impl` コードを書く必要はありません。

---

## クイックリファレンス

```yaoxiang
# ── 関数定義構文一覧 ──

# 式形式（最も一般的）
add: (a: Int, b: Int) -> Int = a + b

# コードブロック形式（複数ステップのロジック）
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

# 引数なし関数（最も簡潔）
main = { println("Hello!") }

# パラメータあり — シグネチャ省略
double = (x: Int) => x * 2

# パラメータあり — Lambda ヘッダ省略（推奨）
triple: (x: Int) -> Int = x * 3

# pub エクスポート + 自動バインディング
pub add: (a: Int, b: Int) -> Int = a + b

# ── 呼び出し構文 ──

result = add(1, 2)          # 位置引数
result = add(a: 1, b: 2)    # 名前付き引数
result = add(1, b: 2)       # 混在（位置が前）
```

---

## まとめ

これで YaoXiang 関数の核となる知識を習得しました：

- **統一構文**：`name: (params) -> Return = body`、変数宣言の `name: type = value` と同じ出自
- **式形式**：`= expr`、式の値が直接戻り値になり、`return` 不要
- **コードブロック形式**：`= { ...; return expr }`、ブロック内では `return` で明示的に返す必要がある
- **パラメータ型注釈**：シグネチャまたは Lambda ヘッダの少なくとも一方で型を書く、シグネチャに書くことを推奨
- **呼び出し**：位置引数または名前付き引数、名前付き引数は順序自由
- **pub 自動バインディング**：`pub` 関数は同じファイルの型に自動バインドされ、`obj.method()` 呼び出しをサポート
- **引数なし最簡**：`name = { ... }`、コンパイラが自動的に `() -> Void` と推論

次のステップとして、[制御フロー](./control-flow.md) 章に進み、関数内で `if`、`for`、`while` を如何使用するかを学べます。
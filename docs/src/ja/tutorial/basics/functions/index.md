```markdown
---
title: 関数定義と呼び出し
---
```

# 関数定義と呼び出し

前の章では、変数の宣言方法を学びました。この章では、YaoXiang の核心である**関数**を習得します。YaoXiang の関数構文は変数宣言と同じ `name: type = value` モデルを共有しているため、既視感を覚えるはずです。

## 関数は Lambda である

最も重要な概念から始めましょう：**YaoXiang では、関数は本质上 lambda 式です**。特別な `fn` キーワードはなく、複雑な儀式もありません。関数を定義するとは、lambda に名前を付けるだけです。

```
# あらゆる関数は本质上次の 4 つの要素の組み合わせです：
name: (params) -> Return = body
 ^       ^        ^        ^
 |       |        |        +-- 関数本体（lambda 式またはコードブロック）
 |       |        +-- 戻り値の型
 |       +-- パラメータリスト（シグネチャ）
 +-- 関数名
```

これは前の章で学んだ `name: type = value` と完全に一致します——ただここの「型」がちょうど関数型であるだけです。

---

## 式形式：直接的に値を返す

最も簡単な関数には `return` キーワードは不要です。関数本体が単一の式である場合、それが直接戻り値となります：

```yaoxiang
// 式形式——直接的に値を返し、return は不要
add: (a: Int, b: Int) -> Int = a + b
square: (x: Int) -> Int = x * x
greet: (name: String) -> String = "こんにちは, " + name
```

呼び出し：

```yaoxiang
sum = add(3, 5)          // sum = 8
sq = square(4)           // sq = 16
msg = greet("世界")       // msg = "こんにちは, 世界"
```

これを**式形式**と呼びます。関数本体が式（`{ }` コードブロックではない）の場合、その値が直接関数の戻り値となります。`return` を書く必要はなく、書くとむしろ構文エラーになります。

```yaoxiang
// 正しい：式が直接戻り値となる
double: (x: Int) -> Int = x * 2

// エラー：式形式で return を書くと構文エラー
// double: (x: Int) -> Int = return x * 2   // ❌
```

---

## コードブロック形式：明示的な return

関数が複数ステップの計算を含む場合、`{ }` コードブロックで関数本体を囲みます。**コードブロック内では、`return` 文を使って値を返す必要があります**：

```yaoxiang
// コードブロック形式——return で値を返す必要がある
factorial: (n: Int) -> Int = {
    if n <= 1 {
        return 1
    }
    return n * factorial(n - 1)
}

// 結果の計算
f5 = factorial(5)        // f5 = 120
```

ルールは単純です：**式形式は直接的に値を返す；コードブロック形式は明示的に `return` しなければならない**。コードブロック内で `return` を書き忘れた場合、関数はデフォルトで `Void` を返します。

```yaoxiang
// 注意：この関数にはバグがあります
// bad_add: (a: Int, b: Int) -> Int = {
//     a + b   // return がない！ブロックはデフォルトで Void を返すが、シグネチャは Int を要求する → 型エラー
// }

// 正しい書き方
good_add: (a: Int, b: Int) -> Int = {
    return a + b
}
```

まとめ：

| 形式 | 構文 | 戻り値の返し方 |
|------|------|----------------|
| 式形式 | `name: ... = expr` | 式の値が直接戻り値となる |
| コードブロック形式 | `name: ... = { ... }` | 明示的に `return` する必要がある |

---

## パラメータ定義

### 基本的なパラメータ

パラメータは関数シグネチャ内に記述し、各パラメータに型を注釈できます：

```yaoxiang
// 2 つのパラメータ、両方に型を注釈
multiply: (a: Int, b: Int) -> Int = a * b
```

### パラメータ型はシグネチャまたは Lambda ヘッダのいずれかに注釈する必要がある

YaoXiang のルールは次の通りです：**入力パラメータがある場合、パラメータ型はシグネチャまたは Lambda ヘッダの少なくとも一方で明示的に出現しなければならない**。両方を省略するとコンパイラに拒否されます。

```yaoxiang
// 方法 1：パラメータ型をシグネチャに書く（Lambda ヘッダを省略）
add: (a: Int, b: Int) -> Int = a + b

// 方法 2：パラメータ型を Lambda ヘッダに書く（シグネチャを省略）
add = (a: Int, b: Int) => a + b

// 方法 3：完全形式（シグネチャ + Lambda ヘッダの両方がある）
add: (a: Int, b: Int) -> Int = (a, b) => a + b

// エラー：両方に型を書かない
// add = (a, b) => a + b   // ❌ コンパイラはパラメータ型を推論できない
```

**方法 1 を使用することをお勧めします**——パラメータ型をシグネチャに書き、Lambda ヘッダを省略する。これは最も簡潔で明確な書き方です。

---

## 戻り値

関数の戻り値型は `->` の後に書きます。`->` は関数型の特徴であり、省略できません（省略すると他の型として解析されます）。

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

戻り値型も省略可能で、HM 型推論に処理させることができます：

```yaoxiang
// コンパイラが戻り値型を Int と推論
add = (a: Int, b: Int) => a + b

// コンパイラが戻り値型を String と推論
greet = (name: String) => "こんにちは, " + name
```

---

## 関数呼び出し

### 位置引数

最も基本的な呼び出し方法——順番に渡す：

```yaoxiang
add: (a: Int, b: Int) -> Int = a + b

result = add(1, 2)        // result = 3
```

構文仕様における関数呼び出しの形式定義は次の通りです：

```
Expr '(' ArgList? ')'
```

日常言語に翻訳すると：式の後に括弧が続き、括弧内に引数リストを置くことができます。

### 名前付き引数

位置で渡すだけでなく、YaoXiang は**名前付き引数**もサポートしています——パラメータ名で値を指定し、順序は自由です：

```yaoxiang
// 名前付き引数——パラメータ名の後にコロン、そして値
result = add(a: 3, b: 5)     // result = 8
result = add(b: 5, a: 3)     // 順序自由、結果は同じ

// 位置引数と混在可能だが、位置引数は前に置く必要がある
result = add(3, b: 5)        // OK
```

名前付き引数は呼び出しをより読みやすくし、パラメータが多い場合に特に有用です：

```yaoxiang
// 関数シグネチャ
send: (to: String, title: String, body: String) -> Void = {
    print("发送给: " + to)
    print("标题: " + title)
    print("正文: " + body)
}

// 名前付き引数で呼び出し意図が一目でわかる
send(
    to: "alice@example.com",
    title: "会议通知",
    body: "明天下午 3 点开会"
)
```

---

## 引数なし関数

引数を必要としない関数は、パラメータリストを省略できます：

```yaoxiang
// 完全形式：明示的に空のパラメータを宣言
hello: () -> Void = {
    print("Hello!")
}

// 最も簡潔な形式：シグネチャを省略、コンパイラが自動的に () -> Void と推論
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

// 最も簡潔な形式（推奨）
main = {
    print("Hello, YaoXiang!")
}
```

---

## 複数行関数

関数のロジックが複雑な場合は、コードブロック形式を使用してコードを整理します。YaoXiang は 4 つのスペースインデントを強制します：

```yaoxiang
// 複数ステップの計算
calculate_stats: (numbers: List(Int)) -> Float = {
    // ローカル変数の宣言
    mut total = 0
    mut count = 0

    // 累積ループ
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

複数行関数では `#` でコメントを書け、`mut` ローカル変数を宣言でき、`for` と `if` でロジックを構築できます。

---

## pub と自動バインディング

モジュール内で、`pub` キーワードで宣言された関数は他のモジュールからインポートして使用できます。さらに興味深いことに、**`pub` 関数は同じファイルで定義された型に自動的にバインドされ**、OOP スタイルで呼び出すことができます。

```yaoxiang
// point.yx

// 型の定義
Point: Type = { x: Float, y: Float }

// pub 関数：コンパイラが自動的に Point.distance としてバインド
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
}

// 両方の呼び出し方法が使用可能
p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

d1 = distance(p1, p2)       // 関数型呼び出し
d2 = p1.distance(p2)        // OOP スタイル呼び出し（構文糖）
```

コンパイラが `pub distance(p1: Point, p2: Point)` を見て、`Point` が同じファイルで定義されていることを発見すると、自動的に `Point.distance` バインディングを作成します。追加の `impl` コードを書く必要はありません。

---

## クイックリファレンス

```yaoxiang
// ── 関数定義構文一覧 ──

// 式形式（最も一般的）
add: (a: Int, b: Int) -> Int = a + b

// コードブロック形式（複数ステップロジック）
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

// 引数なし関数（最も簡潔）
main = { print("Hello!") }

// 引数あり—シグネチャ省略
double = (x: Int) => x * 2

// 引数あり—Lambda ヘッダ省略（推奨）
triple: (x: Int) -> Int = x * 3

// pub エクスポート + 自動バインディング
pub add: (a: Int, b: Int) -> Int = a + b

// ── 呼び出し構文 ──

result = add(1, 2)          // 位置引数
result = add(a: 1, b: 2)    // 名前付き引数
result = add(1, b: 2)       // 混在（位置が前）
```

---

## まとめ

YaoXiang 関数のコア知識を習得しました：

- **統一構文**：`name: (params) -> Return = body`、変数宣言の `name: type = value` と同じ出自
- **式形式**：`= expr`、式の値が直接戻り値となり、`return` は不要
- **コードブロック形式**：`= { ...; return expr }`、ブロック内で `return` を使った明示的な戻りが必要
- **パラメータ型注釈**：シグネチャまたは Lambda ヘッダの少なくとも一方で型を書く、シグネチャに書くことを推奨
- **呼び出し**：位置引数または名前付き引数、名前付き引数は順序自由
- **pub 自動バインディング**：`pub` 関数は同じファイルの型に自動バインドされ、`obj.method()` 呼び出しをサポート
- **引数なし最簡**：`name = { ... }`、コンパイラが自動的に `() -> Void` と推論

次のステップでは、[制御フロー](./control-flow.md)の章に進み、関数内で `if`、`for`、`while` を使用する方法を学ぶことができます。
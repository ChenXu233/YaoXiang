---
title: 型システム
---

# 型システム

基礎チュートリアルでは `Int`、`String`、`Bool` などの組み込み型の使い方を学びました。本章では YaoXiang の型システムを深く理解し、**独自の型を定義する**方法を学びます。

## 統一文法モデル

YaoXiang の型システムは RFC-010 で定義された統一文法の上に構築されています：**すべてが `name: type = value`** です。

| 概念 | 書き方 |
|------|------|
| 変数 | `x: Int = 42` |
| 関数 | `add: (a: Int, b: Int) -> Int = a + b` |
| レコード型 | `Point: Type = { x: Float, y: Float }` |
| インタフェース | `Drawable: Type = { draw: (Surface) -> Void }` |
| ジェネリック型 | `List: (T: Type) -> Type = { ... }` |

注意：**型定義自体も `name: Type = value` です**。

## レコード型

レコード型（他の言語では「構造体」と呼ばれます）は YaoXiang における最も基本的なデータ組織方法です：

```yaoxiang
# レコード型の定義
Point: Type = { x: Float, y: Float }

# インスタンスの作成
origin = Point(x: 0.0, y: 0.0)
p = Point(x: 3.0, y: 4.0)

# フィールドへのアクセス
println(p.x)  # 3.0
println(p.y)  # 4.0
```

### フィールドのデフォルト値

フィールドにはデフォルト値を指定でき、構築時には任意で指定できます：

```yaoxiang
User: Type = {
    name: String,
    age: Int = 0,
    active: Bool = true,
}

alice = User(name: "Alice", age: 25)        # active はデフォルト値 true
bob = User(name: "Bob")                      # age=0, active=true
anonymous = User(name: "guest", active: false)  # age=0
```

### メソッド定義

`Type.method` 構文を使用して型にメソッドを定義します：

```yaoxiang
Point: Type = { x: Float, y: Float }

# メソッドの定義：Point.method 構文
Point.length: (self: Point) -> Float = {
    return (self.x * self.x + self.y * self.y).sqrt()
}

p = Point(x: 3.0, y: 4.0)

# 二つの呼び出し方は等価
println(Point.length(p))  # 5.0 — 関数呼び出し
println(p.length())       # 5.0 — .呼び出し構文
```

### pub 自動バインディング

同じファイル内では、`pub` 宣言された関数は同じファイルで定義された型に自動的にバインドされます：

```yaoxiang
Point: Type = { x: Float, y: Float }

# pub 関数は自動的に Point にバインドされる
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
}

p1 = Point(x: 0.0, y: 0.0)
p2 = Point(x: 3.0, y: 4.0)

# 自動バインディングされたメソッドは . で呼び出す
println(p1.distance(p2))  # 5.0
```

## 列挙型

列挙は互いに排他的なバリアントの集合を定義します。データを持たないバリアントは小文字で、データを持つバリアントは関数型構文で記述します：

```yaoxiang
# 単純な列挙
type Color = red | green | blue

# データ付きの列挙
type Result(T, E) = ok(T) | err(E)

# ネストした列挙
type Shape = circle(Float) | rect(Float, Float) | point
```

列挙の中核となる考え：**各バリアント自体が型でもあります**。

```yaoxiang
area: (s: Shape) -> Float = match s {
    circle(r) => 3.14159 * r * r,
    rect(w, h) => w * h,
    point => 0,
}

println(area(circle(5.0)))    # 78.53975
println(area(rect(3.0, 4.0))) # 12.0
```

## インタフェース

インタフェースは**フィールドがすべて関数型であるレコード型**です。インタフェースを実装するには、レコードにそのインタフェース名を含めます：

```yaoxiang
# インタフェースの定義
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect,
}

# インタフェースの実装：レコード型にインタフェース名を含める
Circle: Type = {
    x: Float,
    y: Float,
    radius: Float,
    Drawable,       # Drawable インタフェースを実装
}

# インタフェースが要求するメソッドを提供
Circle.draw: (self: Circle, surface: Surface) -> Void = {
    surface.draw_circle(self.x, self.y, self.radius)
}

Circle.bounding_box: (self: Circle) -> Rect = {
    return Rect(
        x: self.x - self.radius,
        y: self.y - self.radius,
        width: self.radius * 2.0,
        height: self.radius * 2.0,
    )
}
```

インタフェースはポリモーフィズムを実現します — `Drawable` を実装した任意の型は `Drawable` を受け取る関数に渡すことができます。

## ジェネリック型

ジェネリックにより、**特定の型に限定されない**型定義を記述できます：

```yaoxiang
# ジェネリック Pair
Pair: (T: Type, U: Type) -> Type = { first: T, second: U }

# 使用
string_pair = Pair(Int, String)(first: 1, second: "hello")
float_pair = Pair(Float, Float)(first: 3.14, second: 2.71)
```

ジェネリック関数：

```yaoxiang
# ジェネリック map：リストのすべての要素に関数を適用
map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R)) = {
    mut result: List(R) = []
    for item in list {
        result.append(f(item))
    }
    return result
}

numbers = [1, 2, 3, 4]
doubled = map(Int, Int)(numbers, (x) => x * 2)
println(doubled)  # [2, 4, 6, 8]
```

## まとめ

| 概念 | 構文 | 用途 |
|------|------|------|
| レコード型 | `Point: Type = { x: Float, y: Float }` | 関連データの組織化 |
| 列挙 | `type Color = red \| green \| blue` | 多者択一 |
| インタフェース | `Drawable: Type = { draw: ... }` | ポリモーフィック抽象 |
| ジェネリック | `List: (T: Type) -> Type = { ... }` | 型の引数化 |
| メソッド | `Type.method: (self: Type, ...) -> ...` | 振る舞いの追加 |
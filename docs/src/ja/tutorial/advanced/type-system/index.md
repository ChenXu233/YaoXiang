---
title: '型システム'
---

# 型システム

基礎チュートリアルでは `Int`、`String`、`Bool`
などの組み込み型の使い方を学びました。この章では YaoXiang の型システムを深く理解し、**独自の型を定義する方法**を学びます。

## 統一構文モデル

YaoXiang の型システムは RFC-010 で定義された統一構文の上に構築されています: **すべては
`name: type = value`**。

| 概念           | 書き方                                         |
| -------------- | ---------------------------------------------- |
| 変数           | `x: Int = 42`                                  |
| 関数           | `add: (a: Int, b: Int) -> Int = a + b`         |
| レコード型     | `Point: Type = { x: Float, y: Float }`         |
| インタフェース | `Drawable: Type = { draw: (Surface) -> Void }` |
| ジェネリック型 | `List: (T: Type) -> Type = { ... }`            |

注意: **型定義自体も `name: Type = value`** です。

## レコード型

レコード型（他の言語では「構造体」と呼ばれます）は YaoXiang における最も基本的なデータ編成方法です:

```yaoxiang
// レコード型の定義
Point: Type = { x: Float, y: Float }

// インスタンスの作成
origin = Point(x: 0.0, y: 0.0)
p = Point(x: 3.0, y: 4.0)

// フィールドへのアクセス
print(p.x)  // 3.0
print(p.y)  // 4.0
```

### フィールドのデフォルト値

フィールドにはデフォルト値を指定でき、構築時には任意の指定が可能です:

```yaoxiang
User: Type = {
    name: String,
    age: Int = 0,
    active: Bool = true,
}

alice = User(name: "Alice", age: 25)        // active はデフォルト値の true
bob = User(name: "Bob")                      // age=0, active=true
anonymous = User(name: "guest", active: false)  // age=0
```

### メソッド定義

`Type.method` 構文を使用して型にメソッドを定義します:

```yaoxiang
Point: Type = { x: Float, y: Float }

// メソッドの定義: Point.method 構文
Point.length: (self: Point) -> Float = {
    return (self.x * self.x + self.y * self.y).sqrt()
}

p = Point(x: 3.0, y: 4.0)

// 2 つの呼び出し方は等価
print(Point.length(p))  // 5.0 — 関数呼び出し
print(p.length())       // 5.0 — .呼び出し構文
```

### pub 自動バインディング

同一ファイル内では、`pub` 宣言された関数は自動的に同ファイルで定義された型にバインドされます:

```yaoxiang
Point: Type = { x: Float, y: Float }

// pub 関数は自動的に Point にバインドされる
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
}

p1 = Point(x: 0.0, y: 0.0)
p2 = Point(x: 3.0, y: 4.0)

// 自動バインドされたメソッドは . で呼び出す
print(p1.distance(p2))  // 5.0
```

## enum 型

enum は互いに排他的な変種（バリアント）の集合を定義します。データを持たないバリアントは小文字で、データを持つバリアントは関数型構文で記述します:

```yaoxiang
// シンプルな enum
Color: Type = { red: () -> Color, green: () -> Color, blue: () -> Color }

// データを持つ enum
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// ネストされた enum
Shape: Type = { circle: (Float) -> Shape, rect: (Float, Float) -> Shape, point: () -> Shape }
```

enum の中心思想: **各バリアント自体もまた型である**。

```yaoxiang
area: (s: Shape) -> Float = match s {
    circle(r) => 3.14159 * r * r,
    rect(w, h) => w * h,
    point => 0,
}

print(area(circle(5.0)))    // 78.53975
print(area(rect(3.0, 4.0))) // 12.0
```

## インタフェース

インタフェースとは、**フィールドがすべて関数型であるレコード型**のことです。インタフェースを実装するとは、レコードにそのインタフェース名を含めることです:

```yaoxiang
// インタフェースの定義
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect,
}

// インタフェースの実装: レコード型にインタフェース名を含める
Circle: Type = {
    x: Float,
    y: Float,
    radius: Float,
    Drawable,       // Drawable インタフェースを実装
}

// インタフェースで要求されるメソッドを提供
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

インタフェースは多態性を実現します — `Drawable` を実装した任意の型を `Drawable`
を受け取る関数に渡すことができます。

## ジェネリック型

ジェネリックを使用すると、**特定の型に限定されない**型定義を記述できます:

```yaoxiang
// ジェネリックな Pair
Pair: (T: Type, U: Type) -> Type = { first: T, second: U }

// 使用
string_pair = Pair(Int, String)(first: 1, second: "hello")
float_pair = Pair(Float, Float)(first: 3.14, second: 2.71)
```

ジェネリック関数:

```yaoxiang
// ジェネリック map: リストの全要素に関数を適用
map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R)) = {
    mut result: List(R) = []
    for item in list {
        result.append(f(item))
    }
    return result
}

numbers = [1, 2, 3, 4]
doubled = map(Int, Int)(numbers, (x) => x * 2)
print(doubled)  // [2, 4, 6, 8]
```

## まとめ

| 概念           | 構文                                                                        | 用途                          |
| -------------- | --------------------------------------------------------------------------- | ----------------------------- |
| レコード型     | `Point: Type = { x: Float, y: Float }`                                      | 関連データの編成              |
| enum           | `Color: Type = { red: () -> Color, green: () -> Color, blue: () -> Color }` | 多者択一                      |
| インタフェース | `Drawable: Type = { draw: ... }`                                            | 多態性の抽象                  |
| ジェネリック   | `List: (T: Type) -> Type = { ... }`                                         | 型の引数化                    |
| Never          | `Never` はシステム組み込みの底型                                            | 発散/決して返らないコードパス |
| メソッド       | `Type.method: (self: Type, ...) -> ...`                                     | 振る舞いの付与                |

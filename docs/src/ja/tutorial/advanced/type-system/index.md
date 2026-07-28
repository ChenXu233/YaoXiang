---
title: 型システム
---

# 型システム

基礎チュートリアルでは `Int`、`String`、`Bool`
などの組み込み型の使い方を学びました。この章では YaoXiang の型システムを深く理解し、**独自の型を定義する**方法を学びます。

## 統一構文モデル

YaoXiang の型システムは RFC-010 で定義された統一構文に基づいています：**すべてが
`name: type = value`** です。

| 概念             | 構文                                           |
| ---------------- | ---------------------------------------------- |
| 変数             | `x: Int = 42`                                  |
| 関数             | `add: (a: Int, b: Int) -> Int = a + b`         |
| 記録型           | `Point: Type = { x: Float, y: Float }`         |
| インターフェース | `Drawable: Type = { draw: (Surface) -> Void }` |
| ジェネリック型   | `List: (T: Type) -> Type = { ... }`            |

注意：**型定義自体も `name: Type = value`** です。

## 記録型

記録型（他の言語では「構造体」）は YaoXiang において最も基本的なデータ組織方式です：

```yaoxiang
// 記録型を定義する
Point: Type = { x: Float, y: Float }

// インスタンスを生成する
origin = Point(x: 0.0, y: 0.0)
p = Point(x: 3.0, y: 4.0)

// フィールドにアクセスする
print(p.x)  // 3.0
print(p.y)  // 4.0
```

### フィールドのデフォルト値

フィールドにはデフォルト値を指定でき、生成時に省略可能です：

```yaoxiang
User: Type = {
    name: String,
    age: Int = 0,
    active: Bool = true,
}

alice = User(name: "Alice", age: 25)        // active はデフォルト値の true を取る
bob = User(name: "Bob")                      // age=0, active=true
anonymous = User(name: "guest", active: false)  // age=0
```

### メソッドの定義

`Type.method` 構文を使用して型にメソッドを定義します：

```yaoxiang
Point: Type = { x: Float, y: Float }

// メソッドを定義する：Point.method 構文
Point.length: (self: Point) -> Float = {
    return (self.x * self.x + self.y * self.y).sqrt()
}

p = Point(x: 3.0, y: 4.0)

// 2つの呼び出し方法は等価
print(Point.length(p))  // 5.0 — 関数型呼び出し
print(p.length())       // 5.0 — .呼び出し構文
```

### pub 自動バインディング

同一ファイル内で、`pub` 宣言された関数は同じファイルで定義された型に自動的にバインディングされます：

```yaoxiang
Point: Type = { x: Float, y: Float }

// pub 関数が Point に自動バインディング
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
}

p1 = Point(x: 0.0, y: 0.0)
p2 = Point(x: 3.0, y: 4.0)

// 自動バインディングされたメソッドは . で呼び出す
print(p1.distance(p2))  // 5.0
```

## 列挙型

列挙型は一組の排他的なバリアントを定義します。データのないバリアントは小文字で、データのあるバリアントは関数型構文を使用します：

```yaoxiang
// 単純な列挙型
Color: Type = { red: () -> Color, green: () -> Color, blue: () -> Color }

// データ付き列挙型
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// ネストされた列挙型
Shape: Type = { circle: (Float) -> Shape, rect: (Float, Float) -> Shape, point: () -> Shape }
```

列挙型の核心理念：**各バリアント自体が也是一个型です**。

```yaoxiang
area: (s: Shape) -> Float = match s {
    circle(r) => 3.14159 * r * r,
    rect(w, h) => w * h,
    point => 0,
}

print(area(circle(5.0)))    // 78.53975
print(area(rect(3.0, 4.0))) // 12.0
```

## インターフェース

インターフェースは**フィールドがすべて関数型である記録型**です。インターフェースを実装するには、記録型にそのインターフェース名を含めます：

```yaoxiang
// インターフェースを定義する
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect,
}

// インターフェースを実装する：記録型にインターフェース名を含める
Circle: Type = {
    x: Float,
    y: Float,
    radius: Float,
    Drawable,       // Drawable インターフェースを実装
}

// インターフェースが要求するメソッドを提供する
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

インターフェースは多態を実現します—`Drawable` を実装した型はすべて、`Drawable`
を受け取る関数に渡すことができます。

## ジェネリック型

ジェネリック型を使用すると、**具体的な型に限定されない**型定義を記述できます：

```yaoxiang
// ジェネリック Pair
Pair: (T: Type, U: Type) -> Type = { first: T, second: U }

// 使用例
string_pair = Pair(Int, String)(first: 1, second: "hello")
float_pair = Pair(Float, Float)(first: 3.14, second: 2.71)
```

ジェネリック関数：

```yaoxiang
// ジェネリック map：リストの各要素に関数を適用する
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

## 小括

| 概念             | 構文                                                                        | 用途                      |
| ---------------- | --------------------------------------------------------------------------- | ------------------------- |
| 記録型           | `Point: Type = { x: Float, y: Float }`                                      | 関連データをまとめる      |
| 列挙型           | `Color: Type = { red: () -> Color, green: () -> Color, blue: () -> Color }` | 複数選択から1つ           |
| インターフェース | `Drawable: Type = { draw: ... }`                                            | 多態的な抽象化            |
| ジェネリック型   | `List: (T: Type) -> Type = { ... }`                                         | 型をパラメータ化する      |
| Never            | `Never` はシステム組み込みの底型                                            | 発散/永不返回のコードパス |
| メソッド         | `Type.method: (self: Type, ...) -> ...`                                     | 振る舞いを付随させる      |

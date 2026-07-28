---
title: 型システム
---

# 型システム

基礎編では `Int`、`String`、`Bool`
などの組み込み型の使い方を学びました。この章では YaoXiang の型システムを深く掘り下げ、**独自の型を定義する**方法を学びます。

## 統一された構文モデル

YaoXiang の型システムは、RFC-010 で定義された統一構文の上に構築されています：**すべてが
`name: type = value` という形**。

| 概念             | 書き方                                         |
| ---------------- | ---------------------------------------------- |
| 変数             | `x: Int = 42`                                  |
| 関数             | `add: (a: Int, b: Int) -> Int = a + b`         |
| レコード型       | `Point: Type = { x: Float, y: Float }`         |
| インターフェース | `Drawable: Type = { draw: (Surface) -> Void }` |
| ジェネリクス型   | `List: (T: Type) -> Type = { ... }`            |

注意：**型定義自体も `name: Type = value` という形**。

## レコード型

レコード型（他の言語では「構造体」と呼ばれる）は、YaoXiang における最も基本的なデータ構成方法です：

```yaoxiang
// 定义记录类型
Point: Type = { x: Float, y: Float }

// 创建实例
origin = Point(x: 0.0, y: 0.0)
p = Point(x: 3.0, y: 4.0)

// 访问字段
print(p.x)  // 3.0
print(p.y)  // 4.0
```

### フィールドのデフォルト値

フィールドにはデフォルト値を指定でき、構築時には省略可能です：

```yaoxiang
User: Type = {
    name: String,
    age: Int = 0,
    active: Bool = true,
}

alice = User(name: "Alice", age: 25)        // active 取默认值 true
bob = User(name: "Bob")                      // age=0, active=true
anonymous = User(name: "guest", active: false)  // age=0
```

### メソッド定義

`Type.method` 構文を使って型にメソッドを定義します：

```yaoxiang
Point: Type = { x: Float, y: Float }

// 定义方法：Point.method 语法
Point.length: (self: Point) -> Float = {
    return (self.x * self.x + self.y * self.y).sqrt()
}

p = Point(x: 3.0, y: 4.0)

// 两种调用方式等价
print(Point.length(p))  // 5.0 — 函数式调用
print(p.length())       // 5.0 — .调用语法
```

### pub 自動バインディング

同一ファイル内では、`pub` 宣言された関数は同じファイルで定義された型に自動的にバインドされます：

```yaoxiang
Point: Type = { x: Float, y: Float }

// pub 函数自动绑定到 Point
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
}

p1 = Point(x: 0.0, y: 0.0)
p2 = Point(x: 3.0, y: 4.0)

// 自动绑定的方法用 . 调用
print(p1.distance(p2))  // 5.0
```

## 列挙型

列挙型は互いに排他的なバリアントの集合を定義します。データを持たないバリアントは小文字で、データを持つバリアントは関数構文で書きます：

```yaoxiang
// 简单枚举
Color: Type = { red: () -> Color, green: () -> Color, blue: () -> Color }

// 带数据的枚举
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// 嵌套枚举
Shape: Type = { circle: (Float) -> Shape, rect: (Float, Float) -> Shape, point: () -> Shape }
```

列挙型の中核となる考え方は、**各バリアント自体が型でもある**ということです。

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

インターフェースとは、**フィールドがすべて関数型であるようなレコード型**のことです。インターフェースの実装は、レコードにインターフェース名を含めることで行います：

```yaoxiang
// 定义接口
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect,
}

// 实现接口：在记录类型中包含接口名
Circle: Type = {
    x: Float,
    y: Float,
    radius: Float,
    Drawable,       // 实现 Drawable 接口
}

// 提供接口要求的方法
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

インターフェースはポリモーフィズムを実現します。`Drawable` を実装したあらゆる型を、`Drawable`
を受け取る関数に渡すことができます。

## ジェネリクス型

ジェネリクスを使うと、**特定の型に縛られない**型定義を記述できます：

```yaoxiang
// 泛型 Pair
Pair: (T: Type, U: Type) -> Type = { first: T, second: U }

// 使用
string_pair = Pair(Int, String)(first: 1, second: "hello")
float_pair = Pair(Float, Float)(first: 3.14, second: 2.71)
```

ジェネリクス関数：

```yaoxiang
// 泛型 map：对列表的每个元素应用函数
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

| 概念             | 構文                                                                        | 用途                           |
| ---------------- | --------------------------------------------------------------------------- | ------------------------------ |
| レコード型       | `Point: Type = { x: Float, y: Float }`                                      | 関連データの組織化             |
| 列挙型           | `Color: Type = { red: () -> Color, green: () -> Color, blue: () -> Color }` | 多者択一                       |
| インターフェース | `Drawable: Type = { draw: ... }`                                            | 多態性の抽象化                 |
| ジェネリクス     | `List: (T: Type) -> Type = { ... }`                                         | 型の抽象化                     |
| Never            | `Never` はシステムが組み込みで提供するボトム型                              | 発散／決して戻らないコード経路 |
| メソッド         | `Type.method: (self: Type, ...) -> ...`                                     | 振る舞いの付与                 |

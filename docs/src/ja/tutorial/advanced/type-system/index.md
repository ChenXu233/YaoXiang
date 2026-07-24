---
title: 型システム
---

# 型システム

基本チュートリアルでは `Int`、`String`、`Bool` などの組み込み型の使い方を学びました。この章では YaoXiang の型システムをより深く学び、**独自の型を定義する**方法を習得します。

## 統一構文モデル

YaoXiang の型システムは RFC-010 で定義された統一構文の上に構築されています：**すべてが `name: type = value`**。

| 概念 | 構文 |
|------|------|
| 変数 | `x: Int = 42` |
| 関数 | `add: (a: Int, b: Int) -> Int = a + b` |
| レコード型 | `Point: Type = { x: Float, y: Float }` |
| インターフェース | `Drawable: Type = { draw: (Surface) -> Void }` |
| ジェネリック型 | `List: (T: Type) -> Type = { ... }` |

注意：**型定義自体も `name: Type = value`** です。

## レコード型

レコード型（他の言語では「構造体」と呼ばれます）は、YaoXiang における最も基本的なデータ組織化の手段です：

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

フィールドにはデフォルト値を指定でき、構築時には省略可能です：

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

`Type.method` 構文を使用して型にメソッドを定義します：

```yaoxiang
Point: Type = { x: Float, y: Float }

// メソッドの定義：Point.method 構文
Point.length: (self: Point) -> Float = {
    return (self.x * self.x + self.y * self.y).sqrt()
}

p = Point(x: 3.0, y: 4.0)

// 2 つの呼び出し方は等価
print(Point.length(p))  // 5.0 — 関数呼び出し
print(p.length())       // 5.0 — . 呼び出し構文
```

### pub 自動バインディング

同じファイル内では、`pub` 宣言された関数は自動的に同じファイルで定義された型にバインドされます：

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

## 列挙型

列挙型は相互に排他的なバリアントの集合を定義します。データを持たないバリアントは小文字で、データを持つバリアントは関数型構文で記述します：

```yaoxiang
// 単純な列挙型
Color: Type = { red | green | blue }

// データを持つ列挙型
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

// ネストした列挙型
Shape: Type = { circle(Float) | rect(Float, Float) | point }
```

列挙型の中核となる理念は：**各バリアント自体も型である**ということです。

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

インターフェースは**フィールドがすべて関数型であるレコード型**です。インターフェースを実装するには、レコードにインターフェース名を含めます：

```yaoxiang
// インターフェースの定義
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect,
}

// インターフェースの実装：レコード型内にインターフェース名を含める
Circle: Type = {
    x: Float,
    y: Float,
    radius: Float,
    Drawable,       // Drawable インターフェースを実装
}

// インターフェースが要求するメソッドを提供
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

インターフェースはポリモーフィズムを実現します — `Drawable` を実装する任意の型を、`Drawable` を受け取る関数に渡すことができます。

## ジェネリック型

ジェネリックを使うと、**特定の型に限定されない**型定義を記述できます：

```yaoxiang
// ジェネリック Pair
Pair: (T: Type, U: Type) -> Type = { first: T, second: U }

// 使用例
string_pair = Pair(Int, String)(first: 1, second: "hello")
float_pair = Pair(Float, Float)(first: 3.14, second: 2.71)
```

ジェネリック関数：

```yaoxiang
// ジェネリック map：リストの各要素に関数を適用
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

| 概念 | 構文 | 用途 |
|------|------|------|
| レコード型 | `Point: Type = { x: Float, y: Float }` | 関連データの組織化 |
| 列挙型 | `Color: Type = { red \| green \| blue }` | 択一 |
| インターフェース | `Drawable: Type = { draw: ... }` | ポリモーフィックな抽象化 |
| ジェネリック | `List: (T: Type) -> Type = { ... }` | 型の引数化 |
| Never | `Never` はシステム組み込みのボトム型 | 発散 / 返らないコードパス |
| メソッド | `Type.method: (self: Type, ...) -> ...` | 動作の付加 |
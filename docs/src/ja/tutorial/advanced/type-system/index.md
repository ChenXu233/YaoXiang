```markdown
---
title: 型システム
---

# 型システム

基本チュートリアルでは `Int`、`String`、`Bool` などの組み込み型の使い方を学びました。本章では YaoXiang の型システムをより深く理解し、**独自の型を定義する**方法を学びます。

## 統一構文モデル

YaoXiang の型システムは RFC-010 で定義された統一構文の上に構築されています：**すべては `name: type = value`**。

| 概念 | 書き方 |
|------|------|
| 変数 | `x: Int = 42` |
| 関数 | `add: (a: Int, b: Int) -> Int = a + b` |
| レコード型 | `Point: Type = { x: Float, y: Float }` |
| インターフェース | `Drawable: Type = { draw: (Surface) -> Void }` |
| ジェネリック型 | `List: (T: Type) -> Type = { ... }` |

注意：**型定義自体も `name: Type = value`** です。

## レコード型

レコード型（他の言語では「構造体」と呼ばれます）は、YaoXiang における最も基本的なデータ構造の表現方法です：

```yaoxiang
// レコード型を定義
Point: Type = { x: Float, y: Float }

// インスタンスを作成
origin = Point(x: 0.0, y: 0.0)
p = Point(x: 3.0, y: 4.0)

// フィールドにアクセス
print(p.x)  // 3.0
print(p.y)  // 4.0
```

### フィールドのデフォルト値

フィールドにはデフォルト値を指定でき、コンストラクタ呼び出し時には省略が可能です：

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

`Type.method` 構文を使って型にメソッドを定義します：

```yaoxiang
Point: Type = { x: Float, y: Float }

// メソッドを定義：Point.method 構文
Point.length: (self: Point) -> Float = {
    return (self.x * self.x + self.y * self.y).sqrt()
}

p = Point(x: 3.0, y: 4.0)

// 二つの呼び出し方は等価
print(Point.length(p))  // 5.0 — 関数型呼び出し
print(p.length())       // 5.0 — .呼び出し構文
```

### pub 自動バインディング

同一ファイル内では、`pub` で宣言された関数は同じファイルで定義された型に自動的にバインドされます：

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

列挙型は互いに排他的なバリアントの集合を定義します。データを持たないバリアントは小書きで、データを持つバリアントは関数型構文で記述します：

```yaoxiang
// シンプルな列挙
Color: Type = { red | green | blue }

// データ付きの列挙
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

// ネストした列挙
Shape: Type = { circle(Float) | rect(Float, Float) | point }
```

列挙の核となる考え方は：**各バリアント自体もまた型である**ということです。

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

インターフェースとは、**全フィールドが関数型であるレコード型**のことです。インターフェースを実装するとは、そのレコードを実装型のフィールドに含めることを意味します：

```yaoxiang
// インターフェースを定義
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect,
}

// インターフェースを実装：レコード型内にインターフェース名を含める
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

インターフェースは多態性を実現します——`Drawable` を実装する任意の型を、`Drawable` を受け取る関数に渡すことができます。

## ジェネリック型

ジェネリックを使用すると、**特定の型に限定されない**型定義を記述できます：

```yaoxiang
// ジェネリックな Pair
Pair: (T: Type, U: Type) -> Type = { first: T, second: U }

// 使用例
string_pair = Pair(Int, String)(first: 1, second: "hello")
float_pair = Pair(Float, Float)(first: 3.14, second: 2.71)
```

ジェネリック関数：

```yaoxiang
// ジェネリックな map：リストの全要素に関数を適用
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
| レコード型 | `Point: Type = { x: Float, y: Float }` | 関連データをまとめる |
| 列挙 | `Color: Type = { red \| green \| blue }` | 複数の選択肢のうち一つ |
| インターフェース | `Drawable: Type = { draw: ... }` | 多態性の抽象化 |
| ジェネリック | `List: (T: Type) -> Type = { ... }` | 型の引数化 |
| メソッド | `Type.method: (self: Type, ...) -> ...` | 振る舞いの付加 |
```
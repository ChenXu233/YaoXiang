---
title: '構文早見表'
---

# 構文早見表

5分でYaoXiangの核となる構文を理解。詳細については [チュートリアル](../tutorial/index.md) を参照。

## 変数

```yaoxiang
x = 42                    // 不変（デフォルト）
mut y = 0                 // 可変

name: String = "hello"    // 明示的な型
count: Int = 100          // 型注釈

pub version = "1.0"       // 公開エクスポート
```

## 関数

すべては `name: type = value`。関数も値である。

```yaoxiang
// 式形式（直接値を返す）
add: (a: Int, b: Int) -> Int = a + b

// ブロック形式（明示的な return）
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

// Lambda（シグネチャが完全な場合は引数名を省略可能）
double = (x) => x * 2
add = (a, b) => a + b
inc = x => x + 1            // 単一の引数は括弧を省略可能

// ブロック内では return が必要
process: (x: Int) -> Int = {
    a = x * 2
    b = a + 1
    return b
}

// Void 関数は return 不要
greet: (name: String) -> Void = {
    io.println("Hello, " + name)
}
```

## 型

`type`、`struct`、`trait`、`impl` キーワードは存在しない。統一された宣言ですべてを完結。

```yaoxiang
// レコード型
Point: Type = { x: Float, y: Float }
p = Point(1.0, 2.0)            // 位置引数
p = Point(x=1.0, y=2.0)        // 名前付き引数

// デフォルト値を持つフィールド
Point: Type = { x: Float = 0, y: Float = 0 }
Point()                        // OK: x=0, y=0
Point(x=1.0)                   // OK: x=1.0, y=0

// バリアント型（enum）
Color: Type = { red: () -> Color, green: () -> Color, blue: () -> Color }

Option: (T: Type) -> Type = { some: (T) -> Option(T), none: () -> Option(T) }
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// インターフェース（フィールドがすべて関数型のレコード型）
Drawable: Type = { draw: (Surface) -> Void }

// インターフェース合成
DrawableSerializable: Type = Drawable & Serializable

// 型内でのインターフェース実装宣言
Circle: Type = {
    radius: Float,
    Drawable,              // Drawable インターフェースを実装
    Serializable,          // Serializable インターフェースを実装
}

// ジェネリック型
List: (T: Type) -> Type = {
    data: Array(T),
    length: Int,
    push: (self: List(T), item: T) -> Void,
    map: (R: Type) -> ((self: List(T), f: (T) -> R) -> List(R)),
}

// ジェネリック制約
clone: (T: Clone)(value: T) -> T = value.clone()
sort: (T: Clone + PartialOrd)(list: List(T)) -> List(T)
```

## メソッド

```yaoxiang
// 名前空間関数（Type.method は所属を示すマーカーであり、バインディングではない）
Point.distance: (a: &Point, b: &Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    return (dx * dx + dy * dy).sqrt()
}

// 明示的にバインドした後にのみ `.` 呼び出し構文が使える
Point.distance = distance[0]
// この後 p1.distance(p2) → distance(p1, p2)

// クイック定義 + バインド
Point.draw: (self: &Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}
```

## 制御フロー

```yaoxiang
// if は式
grade = if score >= 90 { "A" } else if score >= 60 { "B" } else { "C" }

// match
result = match value {
    ok(v) => "success: {v}",
    err(e) => "error: {e}",
    _ => "unknown",
}

// ループ
for i in 0..5 { io.println(i) }
for item in items { io.println(item) }

mut n = 0
while n < 5 { io.println(n); n = n + 1 }
```

## データ構造

```yaoxiang
// リスト
nums = [1, 2, 3, 4, 5]
first = nums[0]           // 1

// 辞書
scores = {"Alice": 90, "Bob": 85}
a = scores["Alice"]       // 90

// リスト内包表記
evens = [x for x in nums if x % 2 == 0]
doubled = [x * 2 for x in nums]
```

## パターン照合

```yaoxiang
match shape {
    circle(r) => pi * r * r,
    rect(w, h) => w * h,
    point => 0,
}

// 構造体/タプルパターン
match p {
    { x: 0, y: 0 } => "origin",
    { x, y } => "({x}, {y})",
}
match t {
    (0, 0) => "origin",
    (x, y) => "({x}, {y})",
}

// 分割代入
a, b = (1, 2)              // a=1, b=2

// ガード式
match age {
    n if n >= 18 => true,
    _ => false,
}
```

## モジュールとインポート

```yaoxiang
use std.io
use std.math.{sqrt, sin, cos}
use std.{io, list}

io.println("hello")
result = sqrt(16)         // 4.0

// エイリアス
use std.math as math
use std.{io as print}

// 公開エクスポート
pub add: (a: Int, b: Int) -> Int = a + b
pub Point: Type = { x: Float, y: Float }
```

## 所有権

```yaoxiang
// Move：デフォルトで所有権が移転
p1 = Point(1.0, 2.0)
p2 = p1                   // p1 は移される

// 借用 `&`：自動的にトークンを作成（手動の `&` 不要）
distance: (a: &Point, b: &Point) -> Float = ...
d = distance(p1, p2)      // コンパイラが自動的に借用トークンを作成

// 可変借用 `&mut`
update: (p: &mut Point, x: Float) -> Void = { p.x = x }

// ref：共有保持（コンパイラが自動的に Rc/Arc を選択）
shared = ref data

// clone：明示的なディープコピー
backup = data.clone()
```

## 並行処理

spawn は唯一の並列プリミティブ。async/await も Send/Sync もない。

```yaoxiang
// spawn ブロック：サブ式が自動的に並列実行
result = spawn {
    user = fetch_user(1)
    posts = fetch_posts()
    return (user, posts)
}

// spawn for：データ並列
results = spawn for item in items {
    return process(item)
}

// spawn + ref：タスク間で共有
main: () -> Void = {
    shared = ref data
    result = spawn {
        a = shared
        return a
    }
}
```

## F-string

```yaoxiang
name = "YaoXiang"
io.println(f"Hello {name}")          // Hello YaoXiang
io.println(f"Sum: {10 + 20}")        // Sum: 30
io.println(f"Pi: {pi:.2f}")          // Pi: 3.14
```

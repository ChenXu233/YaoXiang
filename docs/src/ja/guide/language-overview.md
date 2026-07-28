---
title: 文法クイックリファレンス
---

# 文法クイックリファレンス

5分で YaoXiang のコア文法を理解できます。詳細を学ぶには [チュートリアル](/tutorial/)
をご覧ください。

## 変数

```yaoxiang
x = 42                    // 不可変（デフォルト）
mut y = 0                 // 可変

name: String = "hello"    // 明示的な型
count: Int = 100          // 型注釈

pub version = "1.0"       // 公開エクスポート
```

## 関数

すべては `name: type = value`。関数も値です。

```yaoxiang
// 式形式（直接値を返す）
add: (a: Int, b: Int) -> Int = a + b

// コードブロック形式（明示的 return）
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

// Lambda（シグネチャが完整时可省略引数名）
double = (x) => x * 2
add = (a, b) => a + b
inc = x => x + 1            // 単引数では括弧を省略可

// コードブロック内では return が必要
process: (x: Int) -> Int = {
    a = x * 2
    b = a + 1
    return b
}

// Void 関数では return が不要
greet: (name: String) -> Void = {
    io.println("Hello, " + name)
}
```

## 型

`type`、`struct`、`trait`、`impl` キーワードはありません。统一された宣言で全てを記述します。

```yaoxiang
// 記録タイプ
Point: Type = { x: Float, y: Float }
p = Point(1.0, 2.0)            // 位置引数
p = Point(x=1.0, y=2.0)        // 名前付き引数

// デフォルト値を持つフィールド
Point: Type = { x: Float = 0, y: Float = 0 }
Point()                        // OK: x=0, y=0
Point(x=1.0)                   // OK: x=1.0, y=0

// 変体タイプ（enum type）
Color: Type = { red: () -> Color, green: () -> Color, blue: () -> Color }

Option: (T: Type) -> Type = { some: (T) -> Option(T), none: () -> Option(T) }
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// インターフェース（フィールドが全て関数型の記録タイプ）
Drawable: Type = { draw: (Surface) -> Void }

// インターフェース組合
DrawableSerializable: Type = Drawable & Serializable

// 型内宣言でインターフェース実装
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
// 名前空間関数（Type.method は所属标记而非绑定）
Point.distance: (a: &Point, b: &Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    return (dx * dx + dy * dy).sqrt()
}

// 明示的バインディング後にのみ . 呼び出し構文が使用可能
Point.distance = distance[0]
// 以降 p1.distance(p2) → distance(p1, p2)

// 快速定義 + バインディング
Point.draw: (self: &Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}
```

## 制御流

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

// 解構代入
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
// Move：デフォルトの所有権移転
p1 = Point(1.0, 2.0)
p2 = p1                   // p1 は移動される

// 借用 &：トークンは自動的に作成される（手動の & は不要）
distance: (a: &Point, b: &Point) -> Float = ...
d = distance(p1, p2)      // コンパイラが自動的に借用トークンを作成

// 可変借用 &mut
update: (p: &mut Point, x: Float) -> Void = { p.x = x }

// ref：共有所有権（コンパイラが Rc/Arc を自動選択）
shared = ref data

// clone：明示的なディープコピー
backup = data.clone()
```

## 並行性

spawn は唯一の並行プリミティブです。async/await も Send/Sync も不要です。

```yaoxiang
// spawn ブロック：サブ式は自動的に並列化
result = spawn {
    user = fetch_user(1)
    posts = fetch_posts()
    return (user, posts)
}

// spawn for：データ並列性
results = spawn for item in items {
    return process(item)
}

// spawn + ref：タスク間共有
main = {
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

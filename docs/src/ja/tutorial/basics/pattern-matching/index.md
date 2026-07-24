---
title: パターン照合
---

# パターン照合

[match の基礎](../control-flow/match.md) では、`match` の基本的な使い方——リテラル、識別子、ワイルドカード——を学びました。ここでは、YaoXiang のパターン照合のすべての能力を深く探求します。

## 完全なパターン型

構文仕様に基づくと、`Pattern` の完全な定義は次のとおりです：

```
Pattern     ::= Literal       # リテラルパターン：42, "hello"
            | Identifier      # 識別子パターン：値をキャプチャ
            | Wildcard        # ワイルドカード：_
            | StructPattern   # 構造体パターン：レコードの分解
            | TuplePattern    # タプルパターン：タプルの分解
            | EnumPattern     # 列挙型パターン：バリアントの分解
            | OrPattern       # またはパターン：pattern1 | pattern2
```

前の章で最初の3つの基本パターンを学びました。本章では、残りの4つの発展パターンを取り上げます。

## 列挙型パターン

列挙型パターンは、`match` で最もよく使われる高度な機能です。列挙型のバリアントを分解し、内部データを抽出できます。

### 基本的な列挙型のマッチ

```yaoxiang
// Result 型を定義
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

// 関数で match を使用して Result を処理
handle: (result: Result(Int, String)) -> String = match result {
    ok(value) => "成功！得到的值是: {value}",
    err(msg) => "出错啦: {msg}",
}

a = ok(42)
b = err("连接超时")

print(handle(a))  // 成功！得到的值是: 42
print(handle(b))  // 出错啦: 连接超时
```

### Option 型

```yaoxiang
// null を避けるために Option を使用
// 組み込み型: Option: (T: Type) -> Type = some(T) | none

describe: (opt: Option(Int)) -> String = match opt {
    some(n) => "有值: {n}",
    none => "什么也没有",
}

print(describe(some(100)))  // 有值: 100
print(describe(none))       // 什么也没有
```

### カスタム列挙型

```yaoxiang
// 颜色枚举を定義
Color: Type = { red | green | blue | rgb(Int, Int, Int) }

to_hex: (c: Color) -> String = match c {
    red => "#FF0000",
    green => "#00FF00",
    blue => "#0000FF",
    rgb(r, g, b) => "#{r.to_hex()}{g.to_hex()}{b.to_hex()}",
}

print(to_hex(red))                // #FF0000
print(to_hex(rgb(128, 128, 128))) // #808080
```

`rgb(r, g, b)` の `r`、`g`、`b` は識別子パターンです——これらは `rgb` バリアント内部の3つの値をキャプチャします。

## 構造体パターン（レコードの分解）

構造体パターンを使用すると、構造体から関心のあるフィールドを直接抽出できます：

```yaoxiang
Point: Type = { x: Float, y: Float }
Rect: Type = { x: Float, y: Float, width: Float, height: Float }

// 構造体パターンの分解
area: (shape: Rect) -> Float = match shape {
    { x: _, y: _, width: w, height: h } => w * h,
}

r = Rect(0.0, 0.0, 10.0, 20.0)
print(area(r))  // 200.0
```

`{ width: w, height: h }` は、「レコードから `width` フィールドを取り出して変数 `w` にバインドし、`height` フィールドを取り出して変数 `h` にバインドする」ことを意味します。`x: _` と `y: _` は「これらのフィールドは存在するが値は気にしない」ことを表します。

**簡略記法**：フィールド名と変数名が同じ場合、略記できます——コンパイラが自動的に同名変数に分解します：

```yaoxiang
describe_point: (p: Point) -> String = match p {
    { x: 0.0, y: 0.0 } => "原点",
    { x, y } => "坐标 ({x}, {y})",
}

print(describe_point(Point(0.0, 0.0)))  // 原点
print(describe_point(Point(3.0, 4.0)))  // 坐标 (3.0, 4.0)
```

## タプルパターン

タプルパターンは、タプルの各要素を分解します：

```yaoxiang
Pair: Type = (Int, String)

first: (p: Pair) -> Int = match p {
    (n, _) => n,
}

second: (p: Pair) -> String = match p {
    (_, s) => s,
}

p = (42, "hello")
print(first(p))   // 42
print(second(p))  // "hello"
```

## またはパターン

`|` を使用して複数のパターンを組み合わせ、いずれかに一致させます：

```yaoxiang
Token: Type = { number(Int) | plus | minus | times | divide | eof }

// 複数のバリアントを「演算子」カテゴリにまとめる
is_operator: (t: Token) -> Bool = match t {
    plus | minus | times | divide => true,
    _ => false,
}

print(is_operator(plus))      // true
print(is_operator(number(5))) // false
```

## ガード式（if ガード）

マッチアームの後に `if 条件` を追加すると、パターンが一致し**かつ**条件が満たされた場合にのみ、マッチが有効になります：

```yaoxiang
Age: Type = { adult(Int) | child(Int) }

// ガード式で追加の条件を指定
can_drive: (a: Age) -> Bool = match a {
    adult(n) if n >= 18 => true,
    adult(n) if n < 18 => false,
    child(_) => false,
}

print(can_drive(adult(20)))  // true
print(can_drive(adult(16)))  // false
```

ガード式の変数は前述のパターンから来ます——`adult(n) if n >= 18` は、まず `n` で値をキャプチャし、次に `n >= 18` でチェックします。

## 網羅性チェック

YaoXiang コンパイラは、`match` がすべての考えられるケースをカバーすることを保証します。分支を見落とすと、コンパイラはエラーを出します：

```yaoxiang
Direction: Type = { north | south | east | west }

// ✅ 正しい：4つの方向をすべてカバー
turn: (d: Direction) -> Direction = match d {
    north => east,
    east => south,
    south => west,
    west => north,
}

// ❌ コンパイルエラー：west が欠落
// broken: (d: Direction) -> Direction = match d {
//     north => east,
//     east => south,
//     south => west,
//     // west が未処理 → コンパイルエラー
// }
```

これは、YaoXiang がランタイムの予期せぬ動作を防ぐ重要な仕組みです——新しいバリアントを追加すると、すべての `match` 箇所でコンパイラが更新を提醒します。

## ネストされたパターン

パターンの真の力は**ネスト**にあります——1つのパターン内に別のパターンをネストできます：

```yaoxiang
Expr: Type = { literal(Int) | add(Expr, Expr) | mul(Expr, Expr) }

// ネストされたパターン：add の中で再度 literal を照合
simplify: (e: Expr) -> Expr = match e {
    add(literal(0), right) => right,  // 0 + x = x
    add(left, literal(0)) => left,    // x + 0 = x
    mul(literal(1), right) => right,  // 1 * x = x
    mul(left, literal(1)) => left,    // x * 1 = x
    other => other,
}

e = add(literal(0), literal(5))
print(simplify(e))  // literal(5)
```

`add(literal(0), right)` では、外側は `add` 列挙型パターン、内側は `literal(0)` リテラルパターン——2階のネストで、1回の照合ですべて 처리됩니다。

## 小まとめ

| パターン型 | 構文 | 用途 |
|----------|------|------|
| リテラル | `42`, `"hi"` | 値を正確に照合 |
| 識別子 | `x` | 照合した値をキャプチャ |
| ワイルドカード | `_` | フォールバック照合 |
| 列挙型 | `ok(value)` | 列挙型バリアントを分解 |
| 構造体 | `{ x, y }` | レコードフィールドを分解 |
| タプル | `(a, b)` | タプル要素を分解 |
| または | `a \| b \| c` | 複数選択の照合 |
| ガード式 | `pattern if cond` | 条件判断を追加 |

`match` + パターン照合 = YaoXiang における最も強力な制御フローツールです。使い方をマスターすれば、より安全性が高く、より明確なコードが書けるようになります。
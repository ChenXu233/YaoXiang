---
title: パターンマッチ
---

# パターンマッチ

[match の基礎](../control-flow/match.md) では、`match`
の基本的な使い方——リテラル、識別子、ワイルドカード——を学びました。ここでは YaoXiang のパターンマッチの全機能を深く探求します。

## 完全なパターン型

文法仕様によると、`Pattern` の完全な定義は次のとおりです：

```
Pattern     ::= Literal       # リテラルパターン：42, "hello"
            | Identifier      # 識別子パターン：値を捕捉
            | Wildcard        # ワイルドカード：_
            | StructPattern   # 構造体パターン：レコードを分解
            | TuplePattern    # タプルパターン：タプルを分解
            | EnumPattern     # 列挙パターン：バリアントを分解
            | OrPattern       # ORパターン：pattern1 | pattern2
```

前章では最初の3つの基本パターンを学びました。この章では残り4つの発展的なパターンに焦点を当てます。

## 列挙パターン

列挙パターンは `match`
で最もよく使われる高度な機能です。enum バリアントを分解し、内部のデータを抽出できます。

### 基本的な列挙マッチング

```yaoxiang
// Result 型を定義
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// 関数が match を使って Result を処理する
handle: (result: Result(Int, String)) -> String = match result {
    ok(value) => "成功！取得した値: {value}",
    err(msg) => "エラー発生: {msg}",
}

a = ok(42)
b = err("接続タイムアウト")

print(handle(a))  // 成功！取得した値: 42
print(handle(b))  // エラー発生: 接続タイムアウト
```

### Option 型

```yaoxiang
// Option を使って null を回避
// 組み込み型: Option: (T: Type) -> Type = { some: (T) -> Option(T), none: () -> Option(T) }

describe: (opt: Option(Int)) -> String = match opt {
    some(n) => "値あり: {n}",
    none => "何もない",
}

print(describe(some(100)))  // 値あり: 100
print(describe(none))       // 何もない
```

### カスタム列挙

```yaoxiang
// 色 enum を定義
Color: Type = { red: () -> Color, green: () -> Color, blue: () -> Color, rgb: (Int, Int, Int) -> Color }

to_hex: (c: Color) -> String = match c {
    red => "#FF0000",
    green => "#00FF00",
    blue => "#0000FF",
    rgb(r, g, b) => "#{r.to_hex()}{g.to_hex()}{b.to_hex()}",
}

print(to_hex(red))                // #FF0000
print(to_hex(rgb(128, 128, 128))) // #808080
```

`rgb(r, g, b)` の `r`、`g`、`b` は識別子パターンであり、`rgb`
バリアント内の3つの値を捕捉しています。

## 構造体パターン（レコード分解）

構造体パターンを使うと、構造体から必要なフィールドを直接抽出できます：

```yaoxiang
Point: Type = { x: Float, y: Float }
Rect: Type = { x: Float, y: Float, width: Float, height: Float }

// 構造体パターンで分解
area: (shape: Rect) -> Float = match shape {
    { x: _, y: _, width: w, height: h } => w * h,
}

r = Rect(0.0, 0.0, 10.0, 20.0)
print(area(r))  // 200.0
```

`{ width: w, height: h }` は「レコードから `width` フィールドを取り出して変数 `w` に束縛し、`height`
フィールドを取り出して変数 `h` に束縛する」という意味です。`x: _` と `y: _`
は「これらのフィールドは存在するが値は気にしない」という意味です。

**簡略記法**：フィールド名と変数名が同じ場合、簡略化できます——コンパイラが自動的に同名の変数へ分解します：

```yaoxiang
describe_point: (p: Point) -> String = match p {
    { x: 0.0, y: 0.0 } => "原点",
    { x, y } => "座標 ({x}, {y})",
}

print(describe_point(Point(0.0, 0.0)))  // 原点
print(describe_point(Point(3.0, 4.0)))  // 座標 (3.0, 4.0)
```

## タプルパターン

タプルパターンはタプルの各要素を分解します：

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

## ORパターン

`|` を使って複数のパターンを組み合わせ、そのいずれかにマッチさせます：

```yaoxiang
Token: Type = { number: (Int) -> Token, plus: () -> Token, minus: () -> Token, times: () -> Token, divide: () -> Token, eof: () -> Token }

// 複数のバリアントを「演算子」グループにまとめる
is_operator: (t: Token) -> Bool = match t {
    plus | minus | times | divide => true,
    _ => false,
}

print(is_operator(plus))      // true
print(is_operator(number(5))) // false
```

## ガード式（if ガード）

マッチアームの後に `if 条件`
を付けると、パターンがマッチし、**かつ**条件が満たされた場合にのみマッチが成立します：

```yaoxiang
Age: Type = { adult: (Int) -> Age, child: (Int) -> Age }

// ガード式で追加条件を付ける
can_drive: (a: Age) -> Bool = match a {
    adult(n) if n >= 18 => true,
    adult(n) if n < 18 => false,
    child(_) => false,
}

print(can_drive(adult(20)))  // true
print(can_drive(adult(16)))  // false
```

ガード式の中の変数は前述のパターンから来ています——`adult(n) if n >= 18` では、まず `n`
で値を捕捉し、それから `n >= 18` でチェックします。

## 網羅性チェック

YaoXiang コンパイラは `match`
がすべての可能性をカバーしていることを保証します。分岐が欠けていると、コンパイラはエラーを出します：

```yaoxiang
Direction: Type = { north: () -> Direction, south: () -> Direction, east: () -> Direction, west: () -> Direction }

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

これは YaoXiang がランタイムの予期せぬ事態を防ぐ重要な仕組みです——新しいバリアントを追加すると、すべての
`match` 箇所でコンパイラが更新を促します。

## ネストパターン

パターンの真の力は**ネスト**にあります——パターンの中に別のパターンを入れ子にできます：

```yaoxiang
Expr: Type = { literal: (Int) -> Expr, add: (Expr, Expr) -> Expr, mul: (Expr, Expr) -> Expr }

// ネストパターン：add の中でさらに literal をマッチ
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

`add(literal(0), right)` では、外側が `add` の列挙パターン、内側が `literal(0)`
のリテラルパターン——2階層のネストで一度にマッチングします。

## まとめ

| パターン型     | 構文              | 用途                     |
| -------------- | ----------------- | ------------------------ |
| リテラル       | `42`, `"hi"`      | 値を正確にマッチ         |
| 識別子         | `x`               | マッチした値を捕捉       |
| ワイルドカード | `_`               | フォールバックマッチ     |
| 列挙           | `ok(value)`       | enum バリアントを分解    |
| 構造体         | `{ x, y }`        | レコードフィールドを分解 |
| タプル         | `(a, b)`          | タプル要素を分解         |
| OR             | `a \| b \| c`     | 複数候補のマッチ         |
| ガード式       | `pattern if cond` | 追加条件の判定           |

`match` + パターンマッチ =
YaoXiang における最強の制御フローツール。これをマスターすれば、より安全でより明確なコードが書けるようになります。

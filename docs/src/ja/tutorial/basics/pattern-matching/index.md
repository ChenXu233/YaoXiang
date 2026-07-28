---
title: パターン照合
---

# パターン照合

[match の基礎](../control-flow/match.md)では、`match`の基本的な使い方——リテラル、識別子、ワイルドカード——を学びましたここではYaoXiangのパターン照合のすべての機能を深く探ります。

## 完全なパターン型

文法仕様によると、`Pattern`の完全な定義は次のとおりです：

```
Pattern     ::= Literal       # リテラルパターン: 42, "hello"
            | Identifier      # 識別子パターン: 値をキャプチャ
            | Wildcard        # ワイルドカード: _
            | StructPattern   # 構造体パターン: レコードを分解
            | TuplePattern    # タプルパターン: タプルを分解
            | EnumPattern     # 列挙パターン: バリアントを分解
            | OrPattern       # 或パターン: pattern1 | pattern2
```

前章で前三つの基本的なパターンを学びました本章では後四つの進んだパターンを取り上げます。

## 列挙パターン

列挙パターンは`match`で最も常用的advanced featuresです。これは列挙バリアントを分解し、内部データを抽出できます。

### 基本的な列挙マッチング

```yaoxiang
// Result 型を定義
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// 関数で match を使用して Result を処理
handle: (result: Result(Int, String)) -> String = match result {
    ok(value) => "成功！得られた値は: {value}",
    err(msg) => "エラー発生: {msg}",
}

a = ok(42)
b = err("接続タイムアウト")

print(handle(a))  // 成功！得られた値は: 42
print(handle(b))  // エラー発生: 接続タイムアウト
```

### Option 型

```yaoxiang
// Option を使用して null を避ける
// 組み込み型: Option: (T: Type) -> Type = { some: (T) -> Option(T), none: () -> Option(T) }

describe: (opt: Option(Int)) -> String = match opt {
    some(n) => "値あり: {n}",
    none => "何もない",
}

print(describe(some(100)))  // 値あり: 100
print(describe(none))       // 何もない
```

### カスタム列挙型

```yaoxiang
// 色 列挙型を定義
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

`rgb(r, g, b)`の中の`r`、`g`、`b`は識別子パターンです——これらは`rgb`バリアント内部の3つの値をキャプチャします。

## 構造体パターン（レコード分解）

構造体パターンを使用すると、構造体から直接関心のあるフィールドを抽出できます：

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

`{ width: w, height: h }`は「レコードから`width`フィールドを取り出して変数`w`にバインドし、`height`フィールドを取り出して変数`h`にバインドする」を意味します。`x: _`と`y: _`は「これらのフィールドは存在するが値は無視する」を表します。

**簡略記法**：フィールド名と変数名が同じ場合、縮めて書けます——コンパイラは自動的に同名変数に分解します：

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

## 或パターン

`|`を使用して複数のパターンを組み合わせ、いずれか一つにマッチさせます：

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

マッチアームの後に`if 条件`を追加すると、パターン匹配**かつ**条件が満たされたときにのみマッチングが成功します：

```yaoxiang
Age: Type = { adult: (Int) -> Age, child: (Int) -> Age }

// ガード式で追加条件
can_drive: (a: Age) -> Bool = match a {
    adult(n) if n >= 18 => true,
    adult(n) if n < 18 => false,
    child(_) => false,
}

print(can_drive(adult(20)))  // true
print(can_drive(adult(16)))  // false
```

ガード式の中の変数は前述のパターンから来ます——`adult(n) if n >= 18`はまず`n`で値をキャプチャし、次に`n >= 18`でチェックします。

## 窮尽性チェック

YaoXiangコンパイラは`match`がすべての可能なケースをカバーしていることを確認します。分支が不足している場合、コンパイラはエラーを出します：

```yaoxiang
Direction: Type = { north: () -> Direction, south: () -> Direction, east: () -> Direction, west: () -> Direction }

// ✅ 正しい：四つの方向をすべてカバー
turn: (d: Direction) -> Direction = match d {
    north => east,
    east => south,
    south => west,
    west => north,
}

// ❌ コンパイルエラー：west がない
// broken: (d: Direction) -> Direction = match d {
//     north => east,
//     east => south,
//     south => west,
//     // west が未処理 → コンパイルエラー
// }
```

これはYaoXiangが実行時の予期しないエラーを防ぐ重要な機構です——新しいバリアントが追加されると、すべての`match`箇所でコンパイラが更新を促します。

## ネストパターン

パターンの真の威力は**ネスト**にあります——一つのパターンの中に別のパターンをネストできます：

```yaoxiang
Expr: Type = { literal: (Int) -> Expr, add: (Expr, Expr) -> Expr, mul: (Expr, Expr) -> Expr }

// ネストパターン: add の中でさらに literal にマッチング
simplify: (e: Expr) -> Expr = match e {
    add(literal(0), right) => right,  // 0 + x = x
    add(left, literal(0)) => left,   // x + 0 = x
    mul(literal(1), right) => right,  // 1 * x = x
    mul(left, literal(1)) => left,    // x * 1 = x
    other => other,
}

e = add(literal(0), literal(5))
print(simplify(e))  // literal(5)
```

`add(literal(0), right)`では、外側は`add`列挙パターン、内側は`literal(0)`リテラルパターンです——二層のネスト、一度のマッチング。

## 小まとめ

| パターン型 | 構文              | 用途           |
| ---------- | ----------------- | -------------- |
| リテラル   | `42`, `"hi"`      | 値と精密マッチ |
| 識別子     | `x`               | マッチした値をキャプチャ |
| ワイルドカード | `_`           | フォールバックマッチ |
| 列挙       | `ok(value)`       | 列挙バリアントを分解 |
| 構造体     | `{ x, y }`        | レコードフィールドを分解 |
| タプル     | `(a, b)`          | タプル要素を分解 |
| 或         | `a \| b \| c`     | 複数選択マッチ |
| ガード式   | `pattern if cond` | 条件判断を追加 |

`match`+ パターン照合 = YaoXiangにおける最強の制御フローツール。これらを習得すれば、より安全で明確なコードが書けます。

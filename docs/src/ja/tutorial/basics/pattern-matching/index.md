---
title: パターンマッチング
---

# パターンマッチング

[match 基礎](../control-flow/match.md) では、`match` の基本的な使い方——リテラル、識別子、ワイルドカード——を学びました。ここからは YaoXiang のパターンマッチングの全機能を探求していきます。

## 完全なパターン型

文法仕様によると、`Pattern` の完全な定義は次の通りです：

```
Pattern     ::= Literal       # リテラルパターン：42, "hello"
            | Identifier      # 識別子パターン：値をキャプチャ
            | Wildcard        # ワイルドカード：_
            | StructPattern   # 構造体パターン：レコードを分解
            | TuplePattern    # タプルパターン：タプルを分解
            | EnumPattern     # 列挙型パターン：バリアントを分解
            | OrPattern       # ORパターン：pattern1 | pattern2
```

前の章で最初の3つの基礎パターンを学びました。この章では残り4つの応用パターンに焦点を当てます。

## 列挙型パターン

列挙型パターンは `match` で最もよく使われる高度な機能です。列挙型のバリアントを分解し、内部のデータを取り出せます。

### 基本的な列挙型の照合

```yaoxiang
# Result 型を定義
Result: (T: Type, E: Type) -> Type = ok(T) | err(E)

# match を使って Result を処理する関数
handle: (result: Result(Int, String)) -> String = match result {
    ok(value) => "成功！取得した値: ${value}",
    err(msg) => "エラー発生: ${msg}",
}

a = ok(42)
b = err("接続タイムアウト")

println(handle(a))  # 成功！取得した値: 42
println(handle(b))  # エラー発生: 接続タイムアウト
```

### Option 型

```yaoxiang
# null を避けるために Option を使用
# 組み込み型: Option: (T: Type) -> Type = some(T) | none

describe: (opt: Option(Int)) -> String = match opt {
    some(n) => "値あり: ${n}",
    none => "何もなし",
}

println(describe(some(100)))  # 値あり: 100
println(describe(none))       # 何もなし
```

### カスタム列挙型

```yaoxiang
# 色を表す列挙型を定義
type Color = red | green | blue | rgb(Int, Int, Int)

to_hex: (c: Color) -> String = match c {
    red => "#FF0000",
    green => "#00FF00",
    blue => "#0000FF",
    rgb(r, g, b) => "#${r.to_hex()}${g.to_hex()}${b.to_hex()}",
}

println(to_hex(red))                # #FF0000
println(to_hex(rgb(128, 128, 128))) # #808080
```

`rgb(r, g, b)` の `r`、`g`、`b` は識別子パターンで、`rgb` バリアント内の3つの値をキャプチャしています。

## 構造体パターン（レコード分解）

構造体パターンを使うと、構造体から必要なフィールドを直接取り出せます：

```yaoxiang
type Point = { x: Float, y: Float }
type Rect = { x: Float, y: Float, width: Float, height: Float }

# 構造体パターンで分解
area: (shape: Rect) -> Float = match shape {
    { x: _, y: _, width: w, height: h } => w * h,
}

r = Rect(x: 0.0, y: 0.0, width: 10.0, height: 20.0)
println(area(r))  # 200.0
```

`{ width: w, height: h }` は「レコードから `width` フィールドを変数 `w` に、`height` フィールドを変数 `h` に取り出す」という意味です。`x: _` と `y: _` は「これらのフィールドは存在するが値は気にしない」という意味になります。

**簡略記法**：フィールド名と変数名が同じ場合、省略形で書けます——コンパイラが自動的に同名の変数に分解します：

```yaoxiang
describe_point: (p: Point) -> String = match p {
    { x: 0.0, y: 0.0 } => "原点",
    { x, y } => "座標 (${x}, ${y})",
}

println(describe_point(Point(x: 0.0, y: 0.0)))  # 原点
println(describe_point(Point(x: 3.0, y: 4.0)))  # 座標 (3.0, 4.0)
```

## タプルパターン

タプルパターンでタプルの各要素を分解します：

```yaoxiang
type Pair = (Int, String)

first: (p: Pair) -> Int = match p {
    (n, _) => n,
}

second: (p: Pair) -> String = match p {
    (_, s) => s,
}

p = (42, "hello")
println(first(p))   # 42
println(second(p))  # "hello"
```

## ORパターン

`|` を使って複数のパターンを組み合わせ、そのいずれかにマッチさせます：

```yaoxiang
type Token = number(Int) | plus | minus | times | divide | eof

# 複数のバリアントを「演算子」カテゴリとしてまとめる
is_operator: (t: Token) -> Bool = match t {
    plus | minus | times | divide => true,
    _ => false,
}

println(is_operator(plus))      # true
println(is_operator(number(5))) # false
```

## ガード式（if ガード）

マッチアームの後に `if 条件` を付けると、パターンがマッチし**かつ**条件を満たす場合のみそのアームが有効になります：

```yaoxiang
type Age = adult(Int) | child(Int)

# ガード式で追加の条件を付ける
can_drive: (a: Age) -> Bool = match a {
    adult(n) if n >= 18 => true,
    adult(n) if n < 18 => false,
    child(_) => false,
}

println(can_drive(adult(20)))  # true
println(can_drive(adult(16)))  # false
```

ガード式内の変数は前のパターンから来ています——`adult(n) if n >= 18` はまず `n` で値をキャプチャし、それから `n >= 18` でチェックします。

## 網羅性チェック

YaoXiang のコンパイラは `match` がすべての可能なケースをカバーしていることを確認します。ブランチが欠けているとコンパイラはエラーを出します：

```yaoxiang
type Direction = north | south | east | west

# ✅ 正しい：4つの方向をすべてカバー
turn: (d: Direction) -> Direction = match d {
    north => east,
    east => south,
    south => west,
    west => north,
}

# ❌ コンパイルエラー：west が欠落
# broken: (d: Direction) -> Direction = match d {
#     north => east,
#     east => south,
#     south => west,
#     # west が未処理 → コンパイルエラー
# }
```

これは YaoXiang がランタイムの予期せぬ事態を防ぐ重要な仕組みです——新しいバリアントが追加されると、すべての `match` 箇所で更新を促すリマインダがコンパイラから出ます。

## ネストパターン

パターンの真の力は**ネスト**から生まれます——パターンの中に別のパターンを入れ子にできます：

```yaoxiang
type Expr = literal(Int) | add(Expr, Expr) | mul(Expr, Expr)

# ネストパターン：add の内側で literal をさらにマッチ
simplify: (e: Expr) -> Expr = match e {
    add(literal(0), right) => right,  # 0 + x = x
    add(left, literal(0)) => left,    # x + 0 = x
    mul(literal(1), right) => right,  # 1 * x = x
    mul(left, literal(1)) => left,    # x * 1 = x
    other => other,
}

e = add(literal(0), literal(5))
println(simplify(e))  # literal(5)
```

`add(literal(0), right)` では、外側が `add` の列挙型パターン、内側が `literal(0)` のリテラルパターンで、2層のネストを1回のマッチで行っています。

## まとめ

| パターン型 | 文法 | 用途 |
|------------|------|------|
| リテラル | `42`, `"hi"` | 値の厳密マッチ |
| 識別子 | `x` | マッチした値をキャプチャ |
| ワイルドカード | `_` | フォールバックマッチ |
| 列挙型 | `ok(value)` | 列挙型バリアントの分解 |
| 構造体 | `{ x, y }` | レコードフィールドの分解 |
| タプル | `(a, b)` | タプル要素の分解 |
| OR | `a \| b \| c` | 複数候補のマッチ |
| ガード式 | `pattern if cond` | 追加の条件判定 |

`match` + パターンマッチング = YaoXiang における最強の制御フローツール。これを習得すれば、より安全でより明確なコードが書けるようになります。
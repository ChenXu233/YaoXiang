```markdown
---
title: パターンマッチング
---

# パターンマッチング

[match の基礎](../control-flow/match.md) では、`match` の基本的な使い方——リテラル、識別子、ワイルドカード——を学びました。ここから YaoXiang のパターンマッチングの全能力を深掘りしていきましょう。

## 完全なパターン型

文法仕様によると、`Pattern` の完全な定義は次のとおりです：

```
Pattern     ::= Literal       # リテラルパターン: 42, "hello"
            | Identifier      # 識別子パターン: 値をキャプチャ
            | Wildcard        # ワイルドカード: _
            | StructPattern   # 構造体パターン: レコードを分解
            | TuplePattern    # タプルパターン: タプルを分解
            | EnumPattern     # 列挙パターン: バリアントを分解
            | OrPattern       # OR パターン: pattern1 | pattern2
```

前 3 種類の基本パターンは前章で学びました。本章では残り 4 種類の応用パターンに焦点を当てます。

## 列挙パターン

列挙パターンは `match` で最もよく使われる応用機能です。列挙バリアントを分解し、内部のデータを抽出できます。

### 基本的な列挙マッチ

```yaoxiang
// Result 型を定義
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

// match を使って Result を処理する関数
handle: (result: Result(Int, String)) -> String = match result {
    ok(value) => "成功！取得した値: {value}",
    err(msg) => "エラーが発生しました: {msg}",
}

a = ok(42)
b = err("接続タイムアウト")

print(handle(a))  // 成功！取得した値: 42
print(handle(b))  // エラーが発生しました: 接続タイムアウト
```

### Option 型

```yaoxiang
// Option を使って null を回避
// 組み込み型: Option: (T: Type) -> Type = some(T) | none

describe: (opt: Option(Int)) -> String = match opt {
    some(n) => "値あり: {n}",
    none => "値なし",
}

print(describe(some(100)))  // 値あり: 100
print(describe(none))       // 値なし
```

### カスタム列挙

```yaoxiang
// 色の列挙を定義
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

`rgb(r, g, b)` 内の `r`、`g`、`b` は識別子パターンであり、`rgb` バリアント内の 3 つの値をキャプチャします。

## 構造体パターン（レコード分解）

構造体パターンを使うと、構造体から必要なフィールドを直接抽出できます：

```yaoxiang
Point: Type = { x: Float, y: Float }
Rect: Type = { x: Float, y: Float, width: Float, height: Float }

// 構造体パターンによる分解
area: (shape: Rect) -> Float = match shape {
    { x: _, y: _, width: w, height: h } => w * h,
}

r = Rect(0.0, 0.0, 10.0, 20.0)
print(area(r))  // 200.0
```

`{ width: w, height: h }` は「レコードから `width` フィールドを取り出して変数 `w` に束縛し、`height` フィールドを変数 `h` に束縛する」という意味です。`x: _` と `y: _` は「これらのフィールドは存在するが値には興味がない」ことを表します。

**簡略記法**：フィールド名と変数名が同じ場合、省略形が使えます——コンパイラが自動的に同名の変数に分解します：

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

## OR パターン

`|` を使って複数のパターンを組み合わせ、そのうちのいずれかにマッチさせます：

```yaoxiang
Token: Type = { number(Int) | plus | minus | times | divide | eof }

// 複数のバリアントを「演算子」グループとして組み合わせる
is_operator: (t: Token) -> Bool = match t {
    plus | minus | times | divide => true,
    _ => false,
}

print(is_operator(plus))      // true
print(is_operator(number(5))) // false
```

## ガード式（if ガード）

マッチアームの後ろに `if 条件` を追加すると、パターンが一致し**かつ**条件も満たされた場合にのみマッチが成立します：

```yaoxiang
Age: Type = { adult(Int) | child(Int) }

// ガード式で追加条件を付ける
can_drive: (a: Age) -> Bool = match a {
    adult(n) if n >= 18 => true,
    adult(n) if n < 18 => false,
    child(_) => false,
}

print(can_drive(adult(20)))  // true
print(can_drive(adult(16)))  // false
```

ガード式内の変数は直前のパターンから来ています——`adult(n) if n >= 18` はまず `n` で値をキャプチャし、それから `n >= 18` でチェックします。

## 網羅性検査

YaoXiang のコンパイラは `match` がすべての可能性あるケースをカバーしていることを保証します。分岐が欠けていると、コンパイラはエラーを報告します：

```yaoxiang
Direction: Type = { north | south | east | west }

// ✅ 正しく: 4 つの方向をすべてカバー
turn: (d: Direction) -> Direction = match d {
    north => east,
    east => south,
    south => west,
    west => north,
}

// ❌ コンパイルエラー: west が欠落
// broken: (d: Direction) -> Direction = match d {
//     north => east,
//     east => south,
//     south => west,
//     // west 未処理 → コンパイルエラー
// }
```

これは YaoXiang が実行時の予期せぬ事態を防ぐ重要な仕組みです——バリアントが追加されると、すべての `match` 箇所でコンパイラが更新を促します。

## ネストパターン

パターンの真の威力は**ネスト**から生まれます——あるパターンの中に別のパターンを入れ子にできます：

```yaoxiang
Expr: Type = { literal(Int) | add(Expr, Expr) | mul(Expr, Expr) }

// ネストパターン: add の内側で literal をさらにマッチ
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

`add(literal(0), right)` では、外側が `add` の列挙パターン、内側が `literal(0)` のリテラルパターンとなっており、2 層のネストを 1 回のマッチで処理しています。

## まとめ

| パターン種別 | 構文 | 用途 |
|--------------|------|------|
| リテラル | `42`, `"hi"` | 値を厳密に一致させる |
| 識別子 | `x` | 一致した値をキャプチャする |
| ワイルドカード | `_` | デフォルトマッチ |
| 列挙 | `ok(value)` | 列挙バリアントを分解する |
| 構造体 | `{ x, y }` | レコードのフィールドを分解する |
| タプル | `(a, b)` | タプルの要素を分解する |
| OR | `a \| b \| c` | 複数候補のいずれかに一致 |
| ガード | `pattern if cond` | 追加条件を判定する |

`match` + パターンマッチング = YaoXiang における最強の制御フローツール。これをマスターすれば、より安全でより明快なコードが書けるようになります。
```
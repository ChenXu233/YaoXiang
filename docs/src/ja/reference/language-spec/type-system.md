# 型システム仕様

本ドキュメントでは、YaoXiang プログラミング言語の型システム仕様を定義します。基本型、複合型、ジェネリクス、トレイトを含みます。

---

## 第零章：理論的基盤

### 0.1 Curry-Howard 同型対応

Curry-Howard 同型対応（Curry-Howard correspondence）は、YaoXiang の型システムの理論的基盤です。プログラミング言語の型システムと数理論理学の間の深い対応関係を示しています：

| 論理学 | プログラミング言語 |
|--------|----------|
| 命題 \(P\) | 型 `Type` |
| 証明 \(p: P\) | プログラム `x: T = ...` |
| 含意 \(P \rightarrow Q\) | 関数型 `(P) -> Q` |
| 論理積 \(P \wedge Q\) | 積型 `{ a: P, b: Q }` |
| 論理和 \(P \vee Q\) | 和型 `{ a(P) \| b(Q) }` |
| 全称量化 \(\forall x:T. P(x)\) | ジェネリクス `(T: Type) -> ...` |
| 真 \(\top\) | 空型 `{}` |
| 偽 \(\bot\) | `Void` / `Never` |
| 型宇宙 \(Type_n : Type_{n+1}\) | 宇宙分层（Russell 逆理の回避） |
| 数学的帰納法 | 型レベル `match` |

### 0.2 型は命題、プログラムは証明

YaoXiang では、この対応関係は設計上の第一原則です：

- **型は論理命題である**。`Int` は「整数が存在する」という命題であり、`fn(a: Int, b: Int) -> Int` は「2つの整数が与えられたとき、ある整数が存在する」という命題である。
- **型チェックは証明の検証である**。プログラムが型チェックをパスすることは、論理命題が構成的に証明されたことを意味する。
- **終了する型レベル計算は正しい帰納推理に対応する**。YaoXiang の型族（`Nat` 上の `Add` のパターンマッチングなど）は、数学的帰納法の型レベルエンコーディングである。

### 0.3 言語設計への影響

Curry-Howard 同型対応は YaoXiang では以下のように具現化されています：

1. **宇宙分层**（RFC-010）：`Type₀ : Type₁ : Type₂ …` により `Type: Type` に起因する論理逆理（Girard 逆理）を回避
2. **型族**（RFC-011）：自然数 `Nat(Zero/Succ)` の型レベルパターンマッチングは、Peano 公理下での帰納証明に対応
3. **条件型**（RFC-011）：`If: (C: Bool, T: Type, E: Type) -> Type` は論理における場合分けの選言に対応
4. **値依存型**（RFC-011）：`Vec: (n: Int) -> Type` は「各整数 n に対してある型が存在する」という有限量化に対応

---

## 第一章：型の分類

### 1.1 型式

```
TypeExpr    ::= PrimitiveType
              | RecordType
              | InterfaceType
              | TupleType
              | FnType
              | GenericType
              | TypeRef
              | TypeUnion
              | TypeIntersection
```

---

## 第二章：基本型

### 2.1 プリミティブ型

| 型 | 説明 | デフォルトサイズ |
|------|------|----------|
| `Type` | メタ型 | 0 バイト |
| `Void` | 空値 | 0 バイト |
| `Bool` | 真理値 | 1 バイト |
| `Int` | 符号付き整数 | 8 バイト |
| `Uint` | 符号なし整数 | 8 バイト |
| `Float` | 浮動小数点 | 8 バイト |
| `String` | UTF-8 文字列 | 可変 |
| `Char` | Unicode 文字 | 4 バイト |
| `Bytes` | 生バイト | 可変 |

ビット幅付き整数：`Int8`, `Int16`, `Int32`, `Int64`, `Int128`
ビット幅付き浮動小数点：`Float32`, `Float64`

---

## 第三章：複合型

### 3.1 レコード型

**統一構文**：`Name: Type = { field1: Type1, field2: Type2, ... }`

```
RecordType  ::= '{' FieldList? '}'
FieldList   ::= Field (',' Field)* ','?
Field       ::= Identifier ':' TypeExpr
            |  Identifier                 // インターフェース制約
```

```yaoxiang
// 単純なレコード型
Point: Type = { x: Float, y: Float }

// 空レコード型
Empty: Type = {}

// ジェネリックレコード型
Pair: (T: Type) -> Type = { first: T, second: T }

// インターフェースを実装するレコード型
Point: Type = {
    x: Float,
    y: Float,
    Drawable,
    Serializable
}
```

**ルール**：

- レコード型は波括弧 `{}` で定義する
- フィールド名の直後にコロンと型を続ける
- 型本体内にインターフェース名を記述するとそのインターフェースを実装する

> **名前空間帰属**：`Type.name` 接頭辞（例：`Point.draw`）は、その関数が `Point` の名前空間属することを示します。
> これは暗黙的なバインディングをトリガーしません。`p.draw()` のような `.` 呼び出し構文を機能させるには、明示的にバインドする必要があります：
> `Point.draw = draw[0]`。
> 詳細は RFC-004 と RFC-010 を参照してください。

#### 3.1.1 フィールドデフォルト値

型フィールドにはデフォルト値を指定でき、構築時に省略可能です：

```yaoxiang
// デフォルト値を持つフィールド - 構築時に省略可能
Point: Type = {
    x: Float = 0,
    y: Float = 0
}

// 使用例
Point()           // -> Point(x=0, y=0)
Point(x=1)       // -> Point(x=1, y=0)
Point(x=1, y=2) // -> Point(x=1, y=2)

// デフォルト値を持たないフィールド - 構築時に必須
Point2: Type = {
    x: Float,
    y: Float
}

// 使用例
Point2(x=1, y=2) // 正しい
Point2()          // エラー
```

**ルール**：

- `field: Type = expression` -> デフォルト値あり、構築時に省略可能
- `field: Type` -> デフォルト値なし、構築時に必須

#### 3.1.2 組込みバインディング

型定義体内で直接メソッドをバインドできます：

```yaoxiang
// 方式1：外部関数を参照してバインディング
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]    // 位置0にバインディング
}
// 呼び出し：p1.distance(p2) -> distance(p1, p2)

// 方式2：匿名関数 + 位置バインディング
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance: ((a: Point, b: Point) -> Float)[0] = ((a, b) => {
        dx = a.x - b.x
        dy = a.y - b.y
        return (dx * dx + dy * dy).sqrt()
    })
}
// 構文：((params) => body)[position]
// 呼び出し：p1.distance(p2) -> distance(p1, p2)
```

### 3.2 インターフェース型

```
InterfaceType ::= '{' FnField (',' FnField)* ','?
FnField       ::= Identifier ':' FnType
FnType        ::= '(' ParamTypes? ')' '->' TypeExpr
```

**構文**：インターフェースはフィールドがすべて関数型であるレコード型です

```yaoxiang
// インターフェース定義
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

// 空インターフェース
EmptyInterface: Type = {}
```

**インターフェース実装**：型は定義末尾にインターフェース名を列出ことでインターフェースを実装します

```yaoxiang
// インターフェースを実装する型
Point: Type = {
    x: Float,
    y: Float,
    Drawable,        // Drawable インターフェースを実装
    Serializable     // Serializable インターフェースを実装
}
```

**インターフェースへの直接代入**：具象型はインターフェース型の変数に直接代入できます（構造的サブタイピング）

```yaoxiang
// 直接代入（コンパイル時に具象型が確定 -> ゼロオーバーヘッド呼び出し）
d: Drawable = Circle(1)
d.draw(screen)        // コンパイル後：直接 circle_draw を呼び出し、vtable なし

// 関数戻り値（コンパイル時に確定できない -> vtable 呼び出し）
d: Drawable = get_shape()
d.draw(screen)        // vtable を介してメソッドを検索

// インターフェースを関数パラメータとして使用
process: (d: Drawable) -> Void = d.draw(screen)
```

**コンパイル時最適化戦略**：

| シナリオ | 推論結果 | 呼び出し方式 |
|------|----------|----------|
| 具象型への直接代入 | 具象型が確定可能 | 直接呼び出し（ゼロオーバーヘッド） |
| 関数戻り値 | 不明 | vtable |
| 不均一コレクション | 複数の型 | vtable |

### 3.4 タプル型

```
TupleType   ::= '(' TypeList? ')'
TypeList    ::= TypeExpr (',' TypeExpr)* ','?
```

### 3.5 関数型

```
FnType      ::= '(' ParamList? ')' '->' TypeExpr
ParamList   ::= TypeExpr (',' TypeExpr)*
```

---

## 第四章：ジェネリクス

### 4.1 ジェネリックパラメータ構文

ジェネリックパラメータは関数型の一部であり、通常の引数と統一された `()` 構文を使用します：

```
GenericType     ::= Identifier '(' TypeArgList ')'
TypeArgList     ::= TypeExpr (',' TypeExpr)* ','?
TypeBound       ::= Identifier
                 |  Identifier '+' Identifier ('+' Identifier)*
```

ジェネリック型定義において、`(T: Type)` は型構築子の引数シグネチャであり、`-> Type` は戻り型を表します：

```yaoxiang
List: (T: Type) -> Type = { ... }
Map: (K: Type, V: Type) -> Type = { ... }
```

ジェネリック関数では、型パラメータもシグネチャで宣言され、コンパイラが実引数から自動的に推論します：

```yaoxiang
map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R)) = ...
```

### 4.2 ジェネリック型定義

```yaoxiang
// 基本ジェネリック型
Option: (T: Type) -> Type = {
    some: (T) -> Option(T),
    none: () -> Option(T)
}

Result: (T: Type, E: Type) -> Type = {
    ok: (T) -> Result(T, E),
    err: (E) -> Result(T, E)
}

List: (T: Type) -> Type = {
    data: Array(T),
    length: Int,
    push: (self: List(T), item: T) -> Void,   # self は単なる約束名であり、キーワードではない
    get: (self: List(T), index: Int) -> Option(T)
}
```

### 4.3 型推論

```yaoxiang
// コンパイラがジェネリックパラメータを自動推論
numbers: List(Int) = List(1, 2, 3)  // コンパイラが List(Int) を推論
```

---

## 第五章：型制約

### 5.1 単一制約

```
ConstrainedType ::= '(' Identifier ':' TypeBound ')' TypeExpr
```

```yaoxiang
// インターフェース型定義（制約として使用）
Clone: Type = {
    clone: () -> Clone
}

// 制約の使用
clone: (T: Clone)(value: T) -> T = value.clone()
```

### 5.2 複数制約

```yaoxiang
// 複数制約構文
combine: (T: Clone + Add)(a: T, b: T) -> T = {
    a.clone() + b
}

// ジェネリックコンテナのソート
sort: (T: Clone + PartialOrd)(list: List(T)) -> List(T) = {
    result = list.clone()
    quicksort(&mut result)
    return result
}
```

### 5.3 関数型制約

```yaoxiang
// 高階関数制約
call_twice: (T: Type, F: () -> T)(f: F) -> (T, T) = (f(), f())

compose: (A: Type, B: Type, C: Type, F: (A) -> B, G: (B) -> C)(a: A, f: F, g: G) -> C = g(f(a))
```

---

## 第六章：関連型

### 6.1 関連型定義

```
AssociatedType ::= Identifier ':' TypeExpr
```

```yaoxiang
// Iterator トレイト（レコード型構文を使用）
Iterator: (T: Type) -> Type = {
    Item: T,                    // 関連型
    next: () -> Option(T),
    has_next: () -> Bool
}

// 関連型の使用
collect: (T: Type, I: Iterator(T))(iter: I) -> List(T) = {
    result = List(T)()
    while iter.has_next() {
        if let Some(item) = iter.next() {
            result.push(item)
        }
    }
    return result
}
```

### 6.2 ジェネリック関連型（GAT）

```yaoxiang
// より複雑な関連型
Container: (T: Type) -> Type = {
    Item: T,
    IteratorType: Iterator(T),  // 関連型もジェネリック
    iter: () -> IteratorType
}
```

---

## 第七章：コンパイル時ジェネリクス

### 7.1 コンパイル時定数パラメータ

```
LiteralType   ::= Identifier ':' Int          // コンパイル時定数
CompileTimeFn ::= '(' Identifier ':' Int ')' '(' Identifier ')' '->' TypeExpr
```

**コア設計**：`(n: Int)` ジェネリックパラメータ + `(n: n)` 値パラメータを使用して、コンパイル時定数と実行時値を区別します。

```yaoxiang
// コンパイル時階乗：パラメータはコンパイル時に既知のリテラルでなければならない
factorial: (n: Int)(n: n) -> Int = {
    match n {
        0 => 1,
        _ => n * factorial(n - 1)
    }
}

// コンパイル時定数配列
StaticArray: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // コンパイル時にサイズが既知の配列
    length: N
}

// 使用例
arr: StaticArray(Int, factorial(5))  // コンパイラがコンパイル時に factorial(5) = 120 を計算
```

### 7.2 コンパイル時定数配列

```yaoxiang
// 行列型での使用
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows)
}

// コンパイル時次元検証
identity_matrix: (T: Add + Zero + One, N: Int)(size: N) -> Matrix(T, N, N) = {
    // ...
}
```

---

## 第八章：条件型

### 8.1 If 条件型

```
IfType        ::= 'If' '(' BoolExpr ',' TypeExpr ',' TypeExpr ')'
```

```yaoxiang
// 型レベル If
If: (C: Bool, T: Type, E: Type) -> Type = match C {
    True => T,
    False => E
}

// 例：コンパイル時分岐
NonEmpty: (T: Type) -> Type = If(T != Void, T, Never)

// コンパイル時検証
Assert: (C: Bool) -> Type = match C {
    True => Void,
    False => compile_error("Assertion failed")
}
```

### 8.2 型族

```yaoxiang
// コンパイル時型変換
AsString: (T: Type) -> Type = match T {
    Int => String,
    Float => String,
    Bool => String,
    _ => String
}
```

---

## 第九章：型和集合と交集

### 9.1 型和集合

```
TypeUnion     ::= TypeExpr '|' TypeExpr
```

### 9.2 型交集

```
TypeIntersection ::= TypeExpr '&' TypeExpr
```

**構文**：型交集 `A & B` は、A と B の両方を同時に満たす型を表します

```yaoxiang
// インターフェース合成 = 型交集
DrawableSerializable: Type = Drawable & Serializable

// 型交集の使用
process: (T: Drawable & Serializable)(item: T, screen: Surface) -> String = {
    item.draw(screen)
    return item.serialize()
}
```

---

## 第十章：関数オーバーロードと特殊化

### 10.1 関数オーバーロード

```yaoxiang
// 基本特殊化：関数オーバーロードを使用（コンパイラが自動選択）
sum: (arr: Array(Int)) -> Int = {
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Array(Float)) -> Float = {
    return simd_sum_float(arr.data, arr.length)
}

// 汎用実装
sum: (T: Add)(arr: Array(T)) -> T = {
    result = Zero::zero()
    for item in arr {
        result = result + item
    }
    return result
}
```

### 10.2 プラットフォーム特殊化

```yaoxiang
// プラットフォーム型列挙型（標準ライブラリで定義）
Platform: Type = X86_64 | AArch64 | RISC_V | ARM | X86

// P は現在のコンパイルプラットフォームを表す事前定義ジェネリックパラメータ名
sum: (P: X86_64)(arr: Array(Float)) -> Float = {
    return avx2_sum(arr.data, arr.length)
}

sum: (P: AArch64)(arr: Array(Float)) -> Float = {
    return neon_sum(arr.data, arr.length)
}
```

---

## 第十一章：型属性

YaoXiang では、Linear と複製可能を区別する1つの型属性のみがあります。コンパイラが自動的に推論します。

### 11.1 Move（デフォルト所有権移動）

すべての型はデフォルトで Move セマンティクスを遵循します。代入、引数渡し、戻り値 = 所有権移動。

```yaoxiang
p: Point = Point(1.0, 2.0)
q = p           // Move、p はこれ以降使用不可
```

### 11.2 Dup（シャローコピー：ハンドルの複製、データ共有）

**Dup 属性は参照/トークン型に使用します**。Dup 型の代入 = シャローコピー——ハンドルトークンを複製し、基盤となるデータを共有します。複数の所有者が同じデータブロックを指します。

| 型 | 属性 | 説明 |
|------|------|------|
| `&T` | Dup | ゼロサイズの読み取りトークン、トークンの複製 = 複数の視点で同じデータを指す |
| `ref T` | Dup | Rc/Arc の複製 = 参照カウント+1、ヒープデータを共有 |
| `&mut T` | Linear | ゼロサイズの書き込みトークン、排他的で複製不可 |
| その他のすべての型 | Move | デフォルト所有権移動 |

**プリミティブ値型**（Int, Float, Bool, Char）はコンパイラの組込み特別な処理です：代入時に自動値複製が行われ、2つの値は完全に独立します。これはコンパイラのネイティブ動作であり、Dup 型属性には属しません。

```yaoxiang
// &T: Dup、自由なエイリアスが可能
view: &Point = &p
view2 = view     // Dup：トークンを複製、両方が有効
print(view.x)    // 使用可能
print(view2.x)   // 使用可能

// &mut T: Linear、複製不可
mut_ref: &mut Point = &mut p
// r2 = mut_ref  // ❌ &mut T は Dup ではないため複製不可
```

### 11.3 Clone（明示的なディープコピー）と Dup の関係

**Clone** は明示的なディープコピーインターフェースです。すべての型は Clone を実装でき、`.clone()` メソッドを提供します。

```yaoxiang
// Clone インターフェース定義（標準ライブラリ）
Clone: Type = {
    clone: () -> Clone
}

// 使用例
p: Point = Point(1.0, 2.0)
backup = p.clone()    // ディープコピー、p は仍然使用可能
p2 = p.clone()        // 複数回のクローンが可能
```

**Dup と Clone の違い**：

| | Dup | Clone |
|---|---|---|
| **セマンティクス** | シャローコピー：ハンドル/トークンを複製、基盤となるデータを共有 | ディープコピー：完全に独立したコピーを作成 |
| **呼び出し方式** | 暗黙的（代入/引数渡しが自動） | 明示的（`.clone()`） |
| **変更の影響** | 相互に影響（基盤となるデータを共有） | 相互に影響しない（独立したコピー） |
| **適用型** | `&T` トークン、`ref T` | Clone インターフェースを実装する任意の型 |
| **コスト** | ゼロオーバーヘッド（トークンはゼロサイズ型） | 型によって異なる |

**Dup は Clone を蕴含せず、Clone は Dup を蕴含しません**——これらは直交する2つの概念です：

```yaoxiang
// Dup 型：トークンを複製、基盤となるデータを共有
view: &Point = &p
view2 = view        // Dup：トークンを複製、両者が同じ p を指す
print(view.x)       // 使用可能
print(view2.x)      // 使用可能、同じデータを参照

// プリミティブ値型：コンパイラが自動値複製（Dupe ではない）
x: Int = 42
y = x               // 値複製、x と y は完全に独立
print(x)            // 使用可能

// Clone：明示的なディープコピー、独立コピーを作成
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone：ディープコピー、p は仍然使用可能
r = p               // Move：所有権移動、Point は Dup でもプリミティブ値型でもないため
```

**設計意図**：

- Dup はトークン/参照型に使用し、「同じデータに対する複数の視点」という問題を解決する
- Clone は独立コピーが必要なシナリオに使用し、明示的な呼び出しでコストを可視化する
- プリミティブ値型（Int/Float/Bool/Char）の複製はコンパイラの組込み動作であり、Dup には属さない
- ほとんどのカスタム型はデフォルトで Move、ゼロコピーで高性能

## 第十二章：借用トークン型

### 12.1 コアコンセプト

`&T` と `&mut T` は**ゼロサイズのコンパイル時トークン型**です。これらは「参照」ではなく、「アクセス権限の型レベル証明」です。

```
&T      →  ゼロサイズ、Dup（複製可能）、読み取り専用権限を付与
&mut T  →  ゼロサイズ、Linear（非 Dup）、排他的な読み書き権限を付与
```

**主要特性**：

- トークンは**通常の型**であり、他のすべての型と同じスコープルールを遵循する
- ライフタイム注釈 `'a` が不要
- 専用借用チェッカー不要——型属性（Dup/Linear）が自然に権限を推論する
- コンパイル後に完全に消失、ランタイムオーバーヘッドゼロ

### 12.2 基本使用

```yaoxiang
// メソッド側：パラメータ型を宣言し、必要な権限を決定
Point.print: (self: &Point) -> Void = {
    print(self.x)               // &Point トークンが読み取り権限を付与
    print(self.y)
}

Point.shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
    self.x = self.x + dx        // &mut Point トークンが書き込み権限を付与
    self.y = self.y + dy
}

// 呼び出し側：コンパイラが自動的に借用または Move を選択
p = Point(1.0, 2.0)
p.print()                       // コンパイラが自動的に &Point トークンを作成
p.shift(1.0, 1.0)               // コンパイラが自動的に &mut Point トークンを作成
p.print()                       // OK、前のトークンは shift 呼び出し終了時に解放済み

// 複数の &T トークンの共存——Dup 型により自由に複製可能
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 トークンのスコープと伝播

トークンは通常の型であるため、すべての通常の型の操作をサポートしています：

**トークンの戻り**——トークンは戻り値とともに伝播します：

```yaoxiang
// ✅ 子トークンと親トークンが一緒に返される
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // トークンが呼び出し元に返される
print(px_ref)                    // OK、トークンは仍然スコープ内
```

**構造体への格納**——構造体はトークンフィールドを持てます：

```yaoxiang
// ✅ 構造体がトークンをフィールドとして携带
Window: Type = {
    target: Point,
    view: &Point,              // トークンフィールド——target への読み取り専用ビューを保持
}
```

**クロージャのキャプチャ**——クロージャは他の値と同じようにトークンをキャプチャします：

```yaoxiang
// ✅ クロージャが &Float トークンをキャプチャ（Dup 型、クロージャに自由に複製可能）
filter_by_threshold: (items: List(Point), threshold: &Float) -> List(Point) = {
    items.filter(|p| p.x > threshold)
}
```

### 12.4 自動借用選択

呼び出し側コンパイラは以下の優先順位で自動的に選択します：

```
1. 実引数が後続でも使用される場合 → トークン作成を優先（&T または &mut T、メソッドシグネチャに応じて）
2. 実引数が後続で使用されない場合 → Move
3. 優先一致順序：&T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // print のパラメータ型は &Point → コンパイラが &Point トークンを作成
p.shift(1.0, 1.0)  // shift のパラメータ型は &mut Point → コンパイラが &mut Point トークンを作成
p2 = p             // 後続で使用しない → Move
```

### 12.5 トークン衝突検出

コンパイラはトークン値に対して**フロー感受性ライブネス分析**を行い、各トークンの状態（ライブ/移動済み）を追跡します：

```yaoxiang
// ❌ &mut と派生 &T が同時にライブになれない
bad_alias: (p: &mut Point) -> Void = {
    p.x = 10.0                   // ✅ 正常使用 WriteToken
    print(p.y)
}

// ✅ トークンスコープ終了後に自動的に解放
good_seq: (p: &mut Point) -> Void = {
    {
        // 内部スコープ
        print(p.x)               // &mut Point を使用
    }
    // 内部スコープ終了
    p.x = 10.0                   // ✅ WriteToken は仍然使用可能
}

// ❌ 同じ実引数に対して同時に &mut トークンとその他のトークンを作成できない
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p が同時に &mut と & トークンを派生させる
```

### 12.6 コンパイラ内部：ブランド機構

ユーザーはブランドに触れません。コンパイラは内部的に各トークンにコンパイル時一意の識別子を割り当てます：

```
ユーザーが見るもの           コンパイラの内部表現
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N はコンパイル時一意の整数
&mut Point     →  WriteToken(Point, #M)   // #M はコンパイル時一意の整数
```

ブランドの用途：

- **偽造防止**：トークンは所有者のコンテナからのみ取得でき、凭空構築できない
- **関連追跡**：フィールドアクセスから派生した `&Float` は派生ブランド（`#N.field_x`）を携带し、コンパイラは親トークンに追跡可能
- **衝突検出**：同一來源の WriteToken と派生 ReadToken は同時にライブになれない

ブランドは単態化とインライン展開後に完全に消失し、生成される機械語に存在しません。**ランタイムオーバーヘッドゼロ。**

### 12.7 トークン Sum 型

```
&BorrowToken ::= &T          // ReadToken（Dup、複製可能）
               | &mut T      // WriteToken（Linear、排他的）
```

### 12.8 借用トークン vs ref

| | `&T` / `&mut T` | `ref` |
|------|------|------|
| 役割 | 一時参照/原地変更 | 共有所有 |
| 範囲 | トークン値のスコープに従う | スコープを跨ぐ |
| コスト | ゼロオーバーヘッド（ゼロサイズ型、コンパイル後に消失） | Rc または Arc（コンパイラが選択） |
| エスケープ | 可能（トークンは戻り値/構造体/クロージャで伝播可能） | 元々エスケープ用 |
| タスク間 | 不可（トークンは Send 未実装） | 可能（コンパイラが自動的に Arc を選択） |
| 環検出 | 関係なし | タスク内でサイレント、跨タスクで lint |

---

## 付録：型定義早見表

### A.1 型定義

```
// === レコード型（波括弧） ===

// レコード型
Point: Type = { x: Float, y: Float }

// 変体付きレコード型（函数字段を使用）
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// === インターフェース型（波括弧、フィールドがすべて関数） ===

// インターフェース定義
Serializable: Type = { serialize: () -> String }

// インターフェースを実装する型
Point: Type = {
    x: Float,
    y: Float,
    Serializable    // Serializable インターフェースを実装
}

// === 関数型 ===

Adder: Type = (Int, Int) -> Int
```

### A.2 ジェネリック構文

```
// ジェネリック型
List: (T: Type) -> Type = { data: Array(T), length: Int }
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// ジェネリック関数
map: (T: Type, R: Type)(list: List(T), f: (T) -> R) -> List(R) = { ... }

// 型制約
clone: (T: Clone)(value: T) -> T = value.clone()
combine: (T: Clone + Add)(a: T, b: T) -> T = body

// 関連型
Iterator: (T: Type) -> Type = { Item: T, next: () -> Option(T) }

// コンパイル時ジェネリクス
factorial: (n: Int)(n: n) -> Int = { ... }
StaticArray: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }

// 条件型
If: (C: Bool, T: Type, E: Type) -> Type = match C { True => T, False => E }

// 関数特殊化
sum: (arr: Array(Int)) -> Int = { ... }
sum: (arr: Array(Float)) -> Float = { ... }
```

### A.3 型属性早見表

```
// === Move（デフォルト） ===
// すべての型はデフォルトで Move。代入、引数渡し、戻り値 = 所有権移動

// === プリミティブ値型（コンパイラの組込み） ===
Int, Float,     // 代入時に自動値複製、2つの値が完全に独立
Bool, Char      // Dup ではなく、コンパイラのプリミティブに対する組込み処理

// === Dup（シャローコピー：ハンドルを複製、基盤となるデータを共有） ===
&T              // ゼロサイズ読み取りトークン、トークンの複製 = 複数の視点で同じデータを指す
ref T           // Rc/Arc の複製 = 参照カウント+1、ヒープデータを共有

// === Linear ===
&mut T          // ゼロサイズ書き込みトークン、Linear（排他的で複製不可）

// === Clone（明示的なディープコピー） ===
value.clone()   // 独立コピーを作成、変更は原値に影響しない
```

### A.4 借用トークン早見表

```
// === 借用トークン ===
&T              // ゼロサイズコンパイル時読み取りトークン、Dup（複製可能）
&mut T          // ゼロサイズコンパイル時書き込みトークン、Linear（複製不可）

// 呼び出し側の自動選択
// 1. 実引数が後続で使用される → トークン作成
// 2. 実引数が後続で使用されない → Move
// 3. 優先一致：&T < &mut T < Move

// トークン伝播
// ✅ 戻り可能、構造体格納可能、クロージャでキャプチャ可能
// ❌ タスク間を跨げない（Send 未実装）
```
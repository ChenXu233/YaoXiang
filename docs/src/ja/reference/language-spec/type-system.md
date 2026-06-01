# タイプシステム仕様

本ドキュメントは、YaoXiang プログラミング言語のタイプシステムを定義する仕様書であり、基本型、複合型、generics、trait を含む。

---

## 第零章：理論的基礎

### 0.1 Curry-Howard 同型対応

Curry-Howard 同型対応（Curry-Howard correspondence）は、YaoXiang のタイプシステムの理論的基盤である。これはプログラミング言語のタイプシステムと数理論理学の間の深い対応関係を明らかにする：

| 論理学 | プログラミング言語 |
|--------|----------|
| 命題 \(P\) | 型 `Type` |
| 証明 \(p: P\) | プログラム `x: T = ...` |
| 含意 \(P \rightarrow Q\) | 関数型 `(P) -> Q` |
| 論理積 \(P \wedge Q\) | 積型 `{ a: P, b: Q }` |
| 論理和 \(P \vee Q\) | 和型 `{ a(P) \| b(Q) }` |
| 全称量化 \(\forall x:T. P(x)\) | generics `(T: Type) -> ...` |
| 真 \(\top\) | 空型 `{}` |
| 偽 \(\bot\) | `Void` / `Never` |
| 型宇宙 \(Type_n : Type_{n+1}\) | 宇宙分层（Russell 逆理の回避） |
| 数学的帰納法 | 型レベル `match` |

### 0.2 型は命題、プログラムは証明

YaoXiang では、この対応関係は設計の一等原則である：

- **型は論理命題である**。`Int` は「整数が存在する」という命題であり、`fn(a: Int, b: Int) -> Int` は「2つの整数が与えられたとき、1つの整数が存在する」という命題である。
- **型チェックは証明の検証である**。プログラムが型チェックをパスすることは、論理命題が構成的に証明されたことを意味する。
- **終了する型レベル計算は正しい帰納的推論に対応する**。YaoXiang の type family（例：`Nat` 上の `Add` の pattern matching）は数学的帰納法の型レベルエンコードである。

### 0.3 言語設計への影響

Curry-Howard 同型対応は YaoXiang で以下のように具体化している：

1. **宇宙分层**（RFC-010）：`Type₀ : Type₁ : Type₂ …` は `Type: Type` がもたらす論理的逆理（Girard 逆理）を回避する
2. **型族**（RFC-011）：自然数 `Nat(Zero/Succ)` の型レベル pattern matching は Peano 公理系下での帰納的証明に対応する
3. **条件型**（RFC-011）：`If: (C: Bool, T: Type, E: Type) -> Type` は論理における case 分配に対応する
4. **値依存型**（RFC-011）：`Vec: (n: Int) -> Type` は「各整数 n に対して型が存在する」の有限量化に対応する

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

### 2.1 原始型

| 型 | 説明 | デフォルトサイズ |
|------|------|----------|
| `Type` | メタ型 | 0 バイト |
| `Void` | 空値 | 0 バイト |
| `Bool` | 真理値 | 1 バイト |
| `Int` | 符号付き整数 | 8 バイト |
| `Uint` | 符号なし整数 | 8 バイト |
| `Float` | 浮動小数点数 | 8 バイト |
| `String` | UTF-8 文字列 | 可変 |
| `Char` | Unicode 文字 | 4 バイト |
| `Bytes` | 生バイト | 可変 |

ビット幅付き整数：`Int8`, `Int16`, `Int32`, `Int64`, `Int128`
ビット幅付き浮動小数点：`Float32`, `Float64`

---

## 第三章：複合型

### 3.1 record type

**統一的構文**：`Name: Type = { field1: Type1, field2: Type2, ... }`

```
RecordType  ::= '{' FieldList? '}'
FieldList   ::= Field (',' Field)* ','?
Field       ::= Identifier ':' TypeExpr
            |  Identifier                 // インターフェース制約
```

```yaoxiang
// 単純な record type
Point: Type = { x: Float, y: Float }

// 空 record type
Empty: Type = {}

// generics を持つ record type
Pair: (T: Type) -> Type = { first: T, second: T }

// インターフェースを実装する record type
Point: Type = {
    x: Float,
    y: Float,
    Drawable,
    Serializable
}
```

**規則**：

- record type は波括弧 `{}` で定義する
- フィールド名の直後にコロンと型を続ける
- インターフェース名は型本体に記述してそのインターフェースを実装を表す

#### 3.1.1 フィールドデフォルト値

型フィールドにはデフォルト値を指定でき、構築時に省略可能：

```yaoxiang
// デフォルト値のあるフィールド - 構築時に省略可能
Point: Type = {
    x: Float = 0,
    y: Float = 0
}

// 使用例
Point()           // -> Point(x=0, y=0)
Point(x=1)       // -> Point(x=1, y=0)
Point(x=1, y=2) // -> Point(x=1, y=2)

// デフォルト値のないフィールド - 構築時に必須
Point2: Type = {
    x: Float,
    y: Float
}

// 使用例
Point2(x=1, y=2) // 正しい
Point2()          // エラー
```

**規則**：

- `field: Type = expression` -> デフォルト値あり、構築時に省略可能
- `field: Type` -> デフォルト値なし、構築時に必須

#### 3.1.2 組み込み束縛

型定義体内で直接メソッドを束縛できる：

```yaoxiang
// 方法1：外部関数を束縛
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]    // 位置0に束縛
}
// 呼び出し：p1.distance(p2) -> distance(p1, p2)

// 方法2：匿名関数 + 位置束縛
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

**構文**：インターフェースとは、フィールドがすべて関数型である record type

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

**インターフェース実装**：型は定義末尾にインターフェース名を列挙してインターフェースを実装する

```yaoxiang
// インターフェースを実装する型
Point: Type = {
    x: Float,
    y: Float,
    Drawable,        // Drawable インターフェースを実装
    Serializable     // Serializable インターフェースを実装
}
```

**インターフェースへの直接代入**：具体的な型はインターフェース型の変数に直接代入可能（構造的サブタイプ）

```yaoxiang
// 直接代入（compile-time に具体的な型が確定 -> ゼロオーバーヘッド呼び出し）
d: Drawable = Circle(1)
d.draw(screen)        // コンパイル後：直接 circle_draw を呼び出し、vtable なし

// 関数戻り値（compile-time に確定不能 -> vtable 呼び出し）
d: Drawable = get_shape()
d.draw(screen)        // vtable 経由でメソッドを検索

// インターフェースを関数パラメータとして使用
process: (d: Drawable) -> Void = d.draw(screen)
```

**コンパイル時最適化戦略**：

| シナリオ | 推論結果 | 呼び出し方式 |
|------|----------|----------|
| 具体的な型への直接代入 | 具体的な型が確定 | 直接呼び出し（ゼロオーバーヘッド） |
| 関数戻り値 | 不明 | vtable |
| 異種混合コレクション | 複数の型 | vtable |

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

## 第四章：generics

### 4.1 generics パラメータ構文

generics パラメータは関数型の一部であり、通常の引数と統一された `()` 構文を使用する：

```
GenericType     ::= Identifier '(' TypeArgList ')'
TypeArgList     ::= TypeExpr (',' TypeExpr)* ','?
TypeBound       ::= Identifier
                 |  Identifier '+' Identifier ('+' Identifier)*
```

generics 型定義では、`(T: Type)` は型構成子のパラメータシグネチャであり、`-> Type` は戻り値の型を表す：

```yaoxiang
List: (T: Type) -> Type = { ... }
Map: (K: Type, V: Type) -> Type = { ... }
```

generics 関数では、型パラメータもシグネチャで宣言し、コンパイラが実引数から自動的に推論する：

```yaoxiang
map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R)) = ...
```

### 4.2 generics 型定義

```yaoxiang
// 基本的な generics 型
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
    push: (self: List(T), item: T) -> Void,
    get: (self: List(T), index: Int) -> Option(T)
}
```

### 4.3 型推論

```yaoxiang
// コンパイラが自動的に generics パラメータを推論
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

// generics コンテナのソート
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
// Iterator trait（record type 構文を使用）
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

### 6.2 Generics 関連型（GAT）

```yaoxiang
// より複雑な関連型
Container: (T: Type) -> Type = {
    Item: T,
    IteratorType: Iterator(T),  // 関連型も generics
    iter: () -> IteratorType
}
```

---

## 第七章：コンパイル時 generics

### 7.1 コンパイル時定数パラメータ

```
LiteralType   ::= Identifier ':' Int          // コンパイル時定数
CompileTimeFn ::= '(' Identifier ':' Int ')' '(' Identifier ')' '->' TypeExpr
```

**コア設計**：`compile-time 定数と runtime 値を区別するために、`(n: Int)` generics パラメータと `(n: n)` 値パラメータを使用する。

```yaoxiang
// compile-time 階乗：パラメータは compile-time に既知の literal である必要がある
factorial: (n: Int)(n: n) -> Int = {
    match n {
        0 => 1,
        _ => n * factorial(n - 1)
    }
}

// compile-time 定数配列
StaticArray: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // compile-time にサイズが確定している配列
    length: N
}

// 使用方法
arr: StaticArray(Int, factorial(5))  // コンパイラが compile-time に factorial(5) = 120 を計算
```

### 7.2 コンパイル時定数配列

```yaoxiang
// 行列型の使用
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows)
}

// compile-time 次元検証
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

// 例：compile-time 分岐
NonEmpty: (T: Type) -> Type = If(T != Void, T, Never)

// compile-time 検証
Assert: (C: Bool) -> Type = match C {
    True => Void,
    False => compile_error("Assertion failed")
}
```

### 8.2 型族

```yaoxiang
// compile-time 型変換
AsString: (T: Type) -> Type = match T {
    Int => String,
    Float => String,
    Bool => String,
    _ => String
}
```

---

## 第九章：型の合併と交叉

### 9.1 型の合併

```
TypeUnion     ::= TypeExpr '|' TypeExpr
```

### 9.2 型の交叉

```
TypeIntersection ::= TypeExpr '&' TypeExpr
```

**構文**：型の交叉 `A & B` は A と B の両方を同時に満たす型を表す

```yaoxiang
// インターフェース合成 = 型の交叉
DrawableSerializable: Type = Drawable & Serializable

// 交叉型の使用
process: (T: Drawable & Serializable)(item: T, screen: Surface) -> String = {
    item.draw(screen)
    return item.serialize()
}
```

---

## 第十章：関数のオーバーロードと特殊化

### 10.1 関数オーバーロード

```yaoxiang
// 基本的な特殊化：関数オーバーロードを使用（コンパイラが自動選択）
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
// プラットフォーム型 enum（標準ライブラリで定義）
Platform: Type = X86_64 | AArch64 | RISC_V | ARM | X86

// P は現在コンパイル中のプラットフォームを表す事前定義 generics パラメータ名
sum: (P: X86_64)(arr: Array(Float)) -> Float = {
    return avx2_sum(arr.data, arr.length)
}

sum: (P: AArch64)(arr: Array(Float)) -> Float = {
    return neon_sum(arr.data, arr.length)
}
```

---

## 第十一章：型の属性

YaoXiang では区別すべき型属性は1つのみ：Linear vs 複製可能。コンパイラが自動的に推論する。

### 11.1 Move（デフォルト所有権移転）

すべての型はデフォルトで Move セマンティクスを遵守する。代入、引数渡し、戻り値 = 所有権移転。

```yaoxiang
p: Point = Point(1.0, 2.0)
q = p           // Move、p はこれ以降使用不可
```

### 11.2 Dup（浅複製：ハンドルの複製、データ共有）

**Dup 属性は参照/トークン型に使用する**。Dup 型の代入 = 浅複製——ハンドルを複製、基底データは共有。複数の所有者が同じデータを指す。

| 型 | 属性 | 説明 |
|------|------|------|
| `&T` | Dup | ゼロサイズの読み取りトークン、トークン複製 = 同じデータを指す複数のビュー |
| `ref T` | Dup | Rc/Arc 複製 = 参照カウント+1、 힙データを共有 |
| `&mut T` | Linear | ゼロサイズの書き込みトークン、排他的、使用不可複製 |
| その他すべての型 | Move | デフォルト所有権移転 |

**原始値型**（Int, Float, Bool, Char）はコンパイラの組み込み的特殊処理である：代入時に自動的に値を複製し、2つの値は完全に独立する。これはコンパイラのネイティブ動作であり、Dup 型属性の一部ではない。

```yaoxiang
// &T: Dup、自由にエイリアス可能
view: &Point = &p
view2 = view     // Dup：トークンを複製、両方有効
print(view.x)    // 使用可能
print(view2.x)   // 使用可能

// &mut T: Linear、複製不可
mut_ref: &mut Point = &mut p
// r2 = mut_ref  // ❌ &mut T は Dup ではないため複製不可
```

### 11.3 Clone（明示的深複製）と Dup の関係

**Clone** は明示的深複製インターフェースである。すべての型は Clone を実装でき、`.clone()` メソッドを提供する。

```yaoxiang
// Clone インターフェース定義（標準ライブラリ）
Clone: Type = {
    clone: () -> Clone
}

// 使用
p: Point = Point(1.0, 2.0)
backup = p.clone()    // 深複製、p は引き続き使用可能
p2 = p.clone()        // 複数回のクローン作成が可能
```

**Dup と Clone の違い**：

| | Dup | Clone |
|---|---|---|
| **セマンティクス** | 浅複製：ハンドル/トークンを複製、基底データは共有 | 深複製：完全に独立したコピーを作成 |
| **呼び出し方式** | 暗黙的（代入/引数渡しが自動） | 明示的（`.clone()`） |
| **変更の影響** | 相互に影響（基底データを共有） | 相互に影響なし（独立したコピー） |
| **適用型** | `&T` トークン、`ref T` | Clone インターフェースを実装する任意の型 |
| **コスト** | ゼロオーバーヘッド（トークンはゼロサイズ型） | 型によって異なる |

**Dup は Clone を意味せず、Clone は Dup を意味しない**——これらは2つの直交する概念である：

```yaoxiang
// Dup 型：トークンを複製、基底データを共有
view: &Point = &p
view2 = view        // Dup：トークンを複製、两者とも同じ p を指す
print(view.x)       // 使用可能
print(view2.x)      // 使用可能、同一データを見ている

// 原始値型：コンパイラが自動的に値を複製（Dup ではない）
x: Int = 42
y = x               // 値を複製、x と y は完全に独立
print(x)            // 使用可能

// Clone：明示的深複製、独立したコピーを作成
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone：深複製、p は引き続き使用可能
r = p               // Move：所有権移転、Point は Dup でも原始値型でもないため
```

**設計意図**：

- Dup はトークン/参照型に使用し、「同じデータを見る複数のビュー」という問題を解決する
- Clone は独立したコピーが必要なシナリオに使用し、コストが見えるようにする
- 原始値型（Int/Float/Bool/Char）の複製はコンパイラの組み込み動作であり、Dup の一部ではない
- ほとんどのカスタム型はデフォルトで Move、ゼロコピーで高性能

## 第十二章：借用トークン型

### 12.1 コア概念

`&T` と `&mut T` は**ゼロサイズの compile-time トークン型**である。これらは「参照」ではなく、「アクセス権限の型レベル証明」である。

```
&T      →  ゼロサイズ、Dup（複製可能）、読み取り専用権限を付与
&mut T  →  ゼロサイズ、Linear（非 Dup）、排他的読み書き権限を付与
```

**主要特性**：

- トークンは**通常の型**であり、他のすべての型と同じスコープ規則に従う
- ライフタイム注釈 `'a` が不要
- 専用の借用チェッカー不要——型属性（Dup/Linear）が自然に権限を推論する
- コンパイル後に完全に消失、ゼロ runtime オーバーヘッド

### 12.2 基本的な使用

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
p.print()                       // OK、前回のトークンは shift 呼び出し終了時に解放済み

// 複数の &T トークンの共存——Dup 型により自由に複製可能
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 トークンのスコープと伝播

トークンは通常の型であるため、すべての通常型の操作をサポートしている：

**トークンの 반환**——トークンは戻り値と一緒に伝播する：

```yaoxiang
// ✅ サブトークンと親トークンが一緒に返される
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // トークンが呼び出し元に返される
print(px_ref)                    // OK、トークンはスコープ内に残っている
```

**構造体への格納**——構造体はトークンフィールドを保持できる：

```yaoxiang
// ✅ 構造体がトークンをフィールドとして保持
Window: Type = {
    target: Point,
    view: &Point,              // トークンフィールド——target への読み取り専用ビューを保持
}
```

**クロージャによるキャプチャ**——クロージャは他の値と同じようにトークンをキャプチャする：

```yaoxiang
// ✅ クロージャが &Float トークンをキャプチャ（Dup 型、自由にクロージャに複製可能）
filter_by_threshold: (items: List(Point), threshold: &Float) -> List(Point) = {
    items.filter(|p| p.x > threshold)
}
```

### 12.4 自動借用選択

呼び出し側コンパイラは以下の優先順位で自動的に選択する：

```
1. 実引数がその後も使用される場合 → トークンを作成優先（&T または &mut T、メソッドシグネチャに従う）
2. 実引数がその後使用されない場合 → Move
3. 優先一致順位：&T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // print は &self を宣言 -> コンパイラが &Point トークンを作成
p.shift(1.0, 1.0)  // shift は &mut self を宣言 -> コンパイラが &mut Point トークンを作成
p2 = p             // その後使用しない -> Move
```

### 12.5 冻结メカニズム

`&mut T` トークンは一時的に「冻结」して `&T` トークンを生成できる：

```yaoxiang
modify_and_read: (p: &mut Point) -> Void = {
    p.x = 10.0                   // &mut Point を使用して変更
    
    // &mut を冻结し、読み取り専用ビューを取得
    view: &Point = freeze(p)     // ここで p が冻结される
    print(view.x)                // &Point を通じて読み取り
    print(view.y)
    // view がスコープを離れる、冻结解除
    
    p.y = 20.0                   // &mut Point が再び使用可能に
}
```

`freeze` のセマンティクス：

- `&mut T` を受け取り、`&T` を返す
- `&T` が生存している間、元の `&mut T` は使用不可
- `&T` がスコープを離れた後、`&mut T` は自動的に回復する
- これは**フロー依存生存解析**——コンパイラは関数本体でトークンの状態を追跡する

### 12.6 トークン衝突検出

コンパイラはトークン値に対して**フロー依存生存解析**を行い、各トークンの状態（生存中/冻结済み/移動済み）を追跡する：

```yaoxiang
// ❌ &mut と派生 &T を同時に生存させることはできない
bad_alias: (p: &mut Point) -> Void = {
    view: &Point = freeze(p)     // p が冻结される
    p.x = 10.0                   // ❌ コンパイルエラー：WriteToken が冻结状態
    print(view.x)
}

// ✅ 冻结解除後に &mut を続行使用可能
good_seq: (p: &mut Point) -> Void = {
    view: &Point = freeze(p)     // p が冻结される
    print(view.x)                // &T を使用
    // view がスコープを離れる、冻结解除
    p.x = 10.0                   // ✅ WriteToken が回復済み
}

// ❌ 同じ実引数から同時に &mut トークン和其他トークンを作成することは不可
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p から同時に &mut と & トークンが派生
```

### 12.7 コンパイラ内部：ブランドメカニズム

ユーザーはブランドに直接触れることはない。コンパイラは内部で各トークンに compile-time で一意の識別子を割り当てる：

```
ユーザーが見るもの           コンパイラの内部表現
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N は compile-time で一意の整数
&mut Point     →  WriteToken(Point, #M)   // #M は compile-time で一意の整数
```

ブランドの用途：

- **偽造防止**：トークンは所有者胶囊または freeze 操作からのみ取得でき、空から構築不可
- **関連追跡**：フィールドアクセスから派生した `&Float` は派生ブランド（`#N.field_x`）を携带し、コンパイラは親トークンに追跡可能
- **衝突検出**：同一来源の WriteToken と派生 ReadToken を同時に生存させることはできない

ブランドは単態化とインライン展開後に完全に消失し、生成される機械語に存在しない。**ゼロ runtime オーバーヘッド。**

### 12.8 トークン Sum 型

```
&BorrowToken ::= &T          // ReadToken（Dup、複製可能）
               | &mut T      // WriteToken（Linear、排他的）
```

### 12.9 借用トークン vs ref

| | `&T` / `&mut T` | `ref` |
|------|------|------|
| 役割 | 一瞥/その場で変更 | 共有所有 |
| 範囲 | トークン値のスコープに従う | スコープをまたぐ |
| コスト | ゼロオーバーヘッド（ゼロサイズ型、コンパイル後に消失） | Rc または Arc（コンパイラが選択） |
| エスケープ | 可能（トークンは戻り値/構造体/クロージャを通じて伝播可能） | そもそもエスケープ用 |
| タスク間 | 不可（トークンは Send を実装していない） | 可能（コンパイラが自動的に Arc を選択） |
| 循環検出 | 関係ない | タスク内は無視、タスク間は lint |

---

## 付録：型定義早見表

### A.1 型定義

```
// === record type（波括弧） ===

// record type
Point: Type = { x: Float, y: Float }

// 変体を持つ record type（関数フィールドを使用）
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// === インターフェース型（波括弧、フィールドはすべて関数） ===

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

### A.2 generics 構文

```
// generics 型
List: (T: Type) -> Type = { data: Array(T), length: Int }
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// generics 関数
map: (T: Type, R: Type)(list: List(T), f: (T) -> R) -> List(R) = { ... }

// 型制約
clone: (T: Clone)(value: T) -> T = value.clone()
combine: (T: Clone + Add)(a: T, b: T) -> T = body

// 関連型
Iterator: (T: Type) -> Type = { Item: T, next: () -> Option(T) }

// compile-time generics
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
// すべての型はデフォルトで Move。代入、引数渡し、戻り値 = 所有権移転

// === 原始値型（コンパイラの組み込み処理） ===
Int, Float,     // 代入時に自動的に値を複製、2つの値は完全に独立
Bool, Char      // Dup ではなく、コンパイラの原始型に対する組み込み処理

// === Dup（浅複製：ハンドルを複製、基底データを共有） ===
&T              // ゼロサイズ読み取りトークン、トークン複製 = 同じデータを指す複数のビュー
ref T           // Rc/Arc 複製 = 参照カウント+1、 힙データを共有

// === Linear ===
&mut T          // ゼロサイズ書き込みトークン、Linear（排他的、複製不可）

// === Clone（明示的深複製） ===
value.clone()   // 独立したコピーを作成、変更は元の値に影響しない
```

### A.4 借用トークン早見表

```
// === 借用トークン ===
&T              // ゼロサイズ compile-time 読み取りトークン、Dup（複製可能）
&mut T          // ゼロサイズ compile-time 書き込みトークン、Linear（複製不可）

// 呼び出し側の自動選択
// 1. 実引数がその後使用 -> トークンを作成
// 2. 実引数がその後不使用 -> Move
// 3. 優先一致：&T < &mut T < Move

// トークンの伝播
// ✅ 戻り値可能、構造体に格納可能、クロージャにキャプチャ可能
// ❌ タスク間をまたげない（Send を実装していない）

// 冻结
view: &T = freeze(mut_ref)   // &mut T -> &T（冻结中は &mut T が使用不可）
```
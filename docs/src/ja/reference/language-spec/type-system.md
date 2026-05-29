# タイプシステム仕様

本ドキュメントはYaoXiangプログラミング言語のタイプシステム仕様を定義ものであり、基本型、複合型、ジェネリクス、traitを含む。

---

## 第零章：理論的基盤

### 0.1 Curry-Howard同形対応

Curry-Howard同形対応（Curry-Howard correspondence）は、YaoXiangのタイプシステムの理論的基盤である。これはプログラミング言語のタイプシステムと数理論理の間の深い対応関係を示す：

| 論理 学 | プログラミング言語 |
|--------|----------|
| 命題 \(P\) | 型 `Type` |
| 証明 \(p: P\) | プログラム `x: T = ...` |
| 含意 \(P \rightarrow Q\) | 関数型 `(P) -> Q` |
| 連言 \(P \wedge Q\) | 積型 `{ a: P, b: Q }` |
| 選言 \(P \vee Q\) | 和型 `{ a(P) \| b(Q) }` |
| 全称量化 \(\forall x:T. P(x)\) | ジェネリクス `(T: Type) -> ...` |
| 真 \(\top\) | 空型 `{}` |
| 偽 \(\bot\) | `Void` / `Never` |
| 型宇宙 \(Type_n : Type_{n+1}\) | 宇宙分层（Russellのパラドックス回避） |
| 数学的帰納法 | 型レベル `match` |

### 0.2 型は命題であり、プログラムは証明である

YaoXiangでは、この対応関係は設計の一級原則である：

- **型は論理命題である**。`Int`は「整数が存在する」という命題であり、`fn(a: Int, b: Int) -> Int`は「2つの整数が与えられたとき、整数が一つ存在する」という命題である。
- **型チェックは証明の検証である**。プログラムが型チェックをパスすることは、論理命題が構成的に証明されたことに相当する。
- **終了する型レベル計算は正しい帰納的推論に対応する**。YaoXiangのtype family（`Nat`上の`Add`のようなpattern matching）は、数学的帰納法の型レベルでの符号化である。

### 0.3 言語設計への影響

Curry-Howard同形対応がYaoXiangにおける具体的な体现：

1. **宇宙分层**（RFC-010）：`Type₀ : Type₁ : Type₂ …` は `Type: Type` がもたらす論理的パラドックス（Girardのパラドックス）を回避する
2. **type family**（RFC-011）：自然数 `Nat(Zero/Succ)` の型レベルpattern matchingは、Peano公理下での帰納的証明に対応する
3. **条件型**（RFC-011）：`If: (C: Bool, T: Type, E: Type) -> Type` は論理における場合分けの選言に対応する
4. **値依存型**（RFC-011）：`Vec: (n: Int) -> Type` は「各整数nに対して型が存在する」という有限量化に対応する

---

## 第一章：型の分類

### 1.1 型式

```
TypeExpr    ::= PrimitiveType
              | StructType
              | EnumType
              | InterfaceType
              | TupleType
              | FnType
              | GenericType
              | TypeRef
              | TypeUnion
              | TypeIntersection
              | ConstrainedType
              | AssociatedType
```

---

## 第二章：基本型

### 2.1 primitive type

| 型 | 説明 | デフォルトサイズ |
|------|------|----------|
| `Type` | meta type | 0バイト |
| `Void` | void | 0バイト |
| `Bool` | boolean type | 1バイト |
| `Int` | 符号付きinteger type | 8バイト |
| `Uint` | 符号なしinteger type | 8バイト |
| `Float` | float type | 8バイト |
| `String` | UTF-8文字列型 | 可変 |
| `Char` | Unicode文字 | 4バイト |
| `Bytes` | 生バイト | 可変 |

ビット幅付き整数：`Int8`, `Int16`, `Int32`, `Int64`, `Int128`
ビット幅付き浮動小数：`Float32`, `Float64`

---

## 第三章：複合型

### 3.1 record type

**統一構文**：`Name: Type = { field1: Type1, field2: Type2, ... }`

```
RecordType  ::= '{' FieldList? '}'
FieldList   ::= Field (',' Field)* ','?
Field       ::= Identifier ':' TypeExpr
            |  Identifier                 // インターフェース制約
```

```yaoxiang
// 単純なrecord type
Point: Type = { x: Float, y: Float }

// 空record type
Empty: Type = {}

// ジェネリクス付きrecord type
Pair: (T: Type) -> Type = { first: T, second: T }

// インターフェースを実装するrecord type
Point: Type = {
    x: Float,
    y: Float,
    Drawable,
    Serializable
}
```

**規則**：

- record typeは波括弧 `{}` で定義する
- フィールド名の後にコロンと型を続ける
- インターフェース名は型本体に記述してそのインターフェースを実装したことを示す

#### 3.1.1 フィールドデフォルト値

型フィールドにはデフォルト値を指定でき、構築時に省略可能：

```yaoxiang
// デフォルト値のあるフィールド - 構築時に省略可能
Point: Type = {
    x: Float = 0,
    y: Float = 0
}

// 使用
Point()           // -> Point(x=0, y=0)
Point(x=1)       // -> Point(x=1, y=0)
Point(x=1, y=2) // -> Point(x=1, y=2)

// デフォルト値のないフィールド - 構築時に必須
Point2: Type = {
    x: Float,
    y: Float
}

// 使用
Point2(x=1, y=2) // 正しい
Point2()          // エラー
```

**規則**：

- `field: Type = expression` -> デフォルト値あり、構築時に省略可能
- `field: Type` -> デフォルト値なし、構築時に必須

#### 3.1.2 builtin binding

型定義本体内で直接メソッドをバインディングできる：

```yaoxiang
// 方法1：外部関数を参照してバインディング
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]    // 位置0にバインディング
}
// 呼び出し：p1.distance(p2) -> distance(p1, p2)

// 方法2：匿名関数 + 位置バインディング
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

### 3.2 enum type（値variant）

```
EnumType    ::= '{' Variant ('|' Variant)* '}'
Variant     ::= Identifier (':' TypeExpr)?
```

**構文**：`Name: Type = { Variant1 | Variant2(params) | ... }`

```yaoxiang
// パラメータなしvariant
Color: Type = { red | green | blue }

// パラメータ付きvariant
Option: (T: Type) -> Type = { some(T) | none }

// 混合
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

// パラメータなしvariantはパラメータなしコンストラクタと同等
Bool: Type = { true | false }
```

### 3.3 interface type

```
InterfaceType ::= '{' FnField (',' FnField)* ','?
FnField       ::= Identifier ':' FnType
FnType        ::= '(' ParamTypes? ')' '->' TypeExpr
```

**構文**：インターフェースはフィールドがすべて関数型であるrecord typeである

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

**インターフェース実装**：型は定義の末尾にインターフェース名を列挙してインターフェースを実装する

```yaoxiang
// インターフェースを実装する型
Point: Type = {
    x: Float,
    y: Float,
    Drawable,        // Drawableインターフェースを実装
    Serializable     // Serializableインターフェースを実装
}
```

**インターフェースへの直接代入**：具象型はインターフェース型変数に直接代入できる（構造的部分型）

```yaoxiang
// 直接代入（コンパイル時に具象型が特定可能 -> ゼロオーバーヘッド呼び出し）
d: Drawable = Circle(1)
d.draw(screen)        // コンパイル後：直接circle_drawを呼び出し、vtableなし

// 関数の戻り値（コンパイル時に特定不可 -> vtable呼び出し）
d: Drawable = get_shape()
d.draw(screen)        // vtable経由でメソッドを検索

// インターフェースを関数パラメータとして使用
process: (d: Drawable) -> Void = d.draw(screen)
```

**コンパイル時最適化戦略**：

| シナリオ | 推論結果 | 呼び出し方式 |
|------|----------|----------|
| 具象型を直接代入 | 具象型を特定可能 | 直接呼び出し（ゼロオーバーヘッド） |
| 関数の戻り値 | 不明 | vtable |
| 不均一コレクション | 複数 型 | vtable |

### 3.4 tuple type

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

### 4.1 ジェネリクス引数構文

```
GenericType     ::= Identifier '[' TypeArgList ']'
TypeArgList     ::= TypeExpr (',' TypeExpr)* ','?
GenericParams   ::= '[' Identifier (',' Identifier)* ']'
                 |  '[' Identifier ':' TypeBound (',' Identifier ':' TypeBound)* ']'
TypeBound       ::= Identifier
                 |  Identifier '+' Identifier ('+' Identifier)*
```

### 4.2 ジェネリクス型定義

```yaoxiang
// 基本ジェネリクス型
Option: (T: Type) -> Type = {
    some: (T) -> Self,
    none: () -> Self
}

Result: (T: Type, E: Type) -> Type = {
    ok: (T) -> Self,
    err: (E) -> Self
}

List: (T: Type) -> Type = {
    data: Array(T),
    length: Int,
    push: (self: List(T), item: T) -> Void,
    get: (self: List(T), index: Int) -> Option(T)
}
```

### 4.3 type inference

```yaoxiang
// コンパイラが自動的にジェネリクス引数を推論
numbers: List(Int) = List(1, 2, 3)  // コンパイラがList(Int)を推論
```

---

## 第五章：type constraint

### 5.1 single constraint

```
ConstrainedType ::= '[' Identifier ':' TypeBound ']' TypeExpr
```

```yaoxiang
// インターフェース型定義（制約として）
Clone: Type = {
    clone: (Self) -> Self
}

// 制約を使用
clone: (T: Clone)(value: T) -> T = value.clone()
```

### 5.2 multiple constraint

```yaoxiang
// multiple constraint構文
combine: (T: Clone + Add)(a: T, b: T) -> T = {
    a.clone() + b
}

// ジェネリクスコンテナのソート
sort: (T: Clone + PartialOrd)(list: List(T)) -> List(T) = {
    result = list.clone()
    quicksort(&mut result)
    return result
}
```

### 5.3 関数型constraint

```yaoxiang
// 高階関数constraint
call_twice: (T: Type, F: Fn() -> T)(f: F) -> (T, T) = (f(), f())

compose: (A: Type, B: Type, C: Type, F: Fn(A) -> B, G: Fn(B) -> C)(a: A, f: F, g: G) -> C = g(f(a))
```

---

## 第六章：associated type

### 6.1 associated type定義

```
AssociatedType ::= Identifier ':' TypeExpr
```

```yaoxiang
// Iterator trait（record type構文を使用）
Iterator: (T: Type) -> Type = {
    Item: T,                    // associated type
    next: (Self) -> Option(T),
    has_next: (Self) -> Bool
}

// associated typeを使用
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

### 6.2 ジェネリクスassociated type（GAT）

```yaoxiang
// より複雑なassociated type
Container: (T: Type) -> Type = {
    Item: T,
    IteratorType: Iterator(T),  // associated typeもジェネリクス
    iter: (Self) -> IteratorType
}
```

---

## 第七章：コンパイルタイムジェネリクス

### 7.1 literal typeconstraint

```
LiteralType   ::= Identifier ':' Int          // コンパイルタイム定数
CompileTimeFn ::= '[' Identifier ':' Int ']' '(' Identifier ')' '->' TypeExpr
```

**核心設計**：`(n: Int)` ジェネリクス引数 + `(n: n)` 値引数を使用して、コンパイルタイム定数とruntime値を区別する。

```yaoxiang
// コンパイルタイム階乗：引数はコンパイルタイム時点で既知のliteralでなければならない
factorial: (n: Int)(n: n) -> Int = {
    match n {
        0 => 1,
        _ => n * factorial(n - 1)
    }
}

// コンパイルタイム定数配列
StaticArray: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // コンパイルタイムでサイズが既知の配列
    length: N
}

// 使用方法
arr: StaticArray(Int, factorial(5))  // コンパイラがコンパイルタイムにfactorial(5) = 120を計算
```

### 7.2 コンパイルタイム定数配列

```yaoxiang
// 行列型で使用
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows)
}

// コンパイルタイム次元検証
identity_matrix: (T: Add + Zero + One, N: Int)(size: N) -> Matrix(T, N, N) = {
    // ...
}
```

---

## 第八章：条件型

### 8.1 If条件型

```
IfType        ::= 'If' '[' BoolExpr ',' TypeExpr ',' TypeExpr ']'
```

```yaoxiang
// 型レベルIf
If: (C: Bool, T: Type, E: Type) -> Type = match C {
    True => T,
    False => E
}

// 例：コンパイルタイム分岐
NonEmpty: (T: Type) -> Type = If(T != Void, T, Never)

// コンパイルタイム検証
Assert: (C: Bool) -> Type = match C {
    True => Void,
    False => compile_error("Assertion failed")
}
```

### 8.2 type family

```yaoxiang
// コンパイルタイム型変換
AsString: (T: Type) -> Type = match T {
    Int => String,
    Float => String,
    Bool => String,
    _ => String
}
```

---

## 第九章：型のUnionとIntersect

### 9.1 型Union

```
TypeUnion     ::= TypeExpr '|' TypeExpr
```

### 9.2 型Intersect

```
TypeIntersection ::= TypeExpr '&' TypeExpr
```

**構文**：型Intersect `A & B` はAとBの両方を同時に満たす型を表す

```yaoxiang
// インターフェース合成 = 型Intersect
DrawableSerializable: Type = Drawable & Serializable

// 型Intersectを使用
process: (T: Drawable & Serializable)(item: T, screen: Surface) -> String = {
    item.draw(screen)
    return item.serialize()
}
```

---

## 第十章：関数のオーバーロードと特化

### 10.1 関数オーバーロード

```yaoxiang
// 基本特化：関数オーバーロードを使用（コンパイラが自動的に選択）
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

### 10.2 プラットフォーム特化

```yaoxiang
// プラットフォーム型enum（標準ライブラリで定義）
Platform: Type = X86_64 | AArch64 | RISC_V | ARM | X86

// Pは現在のコンパイルプラットフォームを表す事前定義済みジェネリクス引数名
sum: (P: X86_64)(arr: Array(Float)) -> Float = {
    return avx2_sum(arr.data, arr.length)
}

sum: (P: AArch64)(arr: Array(Float)) -> Float = {
    return neon_sum(arr.data, arr.length)
}
```

---

## 付録：型定義早見表

### A.1 型定義

```yaoxiang
// === record type（波括弧） ===

// 構造体
Point: Type = { x: Float, y: Float }

// enum（値variant型）
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }
Status: Type = { pending | processing | completed }

// === interface type（波括弧、フィールドはすべて関数） ===

// インターフェース定義
Serializable: Type = { serialize: () -> String }

// インターフェースを実装する型
Point: Type = {
    x: Float,
    y: Float,
    Serializable    // Serializableインターフェースを実装
}

// === 関数型 ===

Adder: Type = (Int, Int) -> Int
```

### A.2 ジェネリクス構文

```yaoxiang
// ジェネリクス型
List: (T: Type) -> Type = { data: Array(T), length: Int }
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

// ジェネリクス関数
map: (T: Type, R: Type)(list: List(T), f: Fn(T) -> R) -> List(R) = { ... }

// type constraint
clone: (T: Clone)(value: T) -> T = value.clone()
combine: (T: Clone + Add)(a: T, b: T) -> T = body

// associated type
Iterator: (T: Type) -> Type = { Item: T, next: () -> Option(T) }

// コンパイルタイムジェネリクス
factorial: (n: Int)(n: n) -> Int = { ... }
StaticArray: (T: Type, N: Int) -> Type = { data: T(N), length: N }

// 条件型
If: (C: Bool, T: Type, E: Type) -> Type = match C { True => T, False => E }

// 関数特化
sum: (arr: Array(Int)) -> Int = { ... }
sum: (arr: Array(Float)) -> Float = { ... }
```
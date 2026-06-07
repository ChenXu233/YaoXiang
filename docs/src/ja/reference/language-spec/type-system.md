# 型システム仕様

本書はYaoXiangプログラミング言語の型システム仕様を定義ものであり、基本型、複合型、泛型、およびtraitを含む。

---

## 第零章：理論的基盤

### 0.1 Curry-Howard 同型

Curry-Howard 同型（Curry-Howard correspondence）はYaoXiangの型システムの理論的基盤である。これはプログラミング言語の型システムと数理論理学の間の深い対応関係を示すものである：

| 論理學 | プログラミング言語 |
|--------|----------|
| 命題 \(P\) | 型 `Type` |
| 証明 \(p: P\) | プログラム `x: T = ...` |
| 含意 \(P \rightarrow Q\) | 関数型 `(P) -> Q` |
| 論理積 \(P \wedge Q\) | 積型 `{ a: P, b: Q }` |
| 論理和 \(P \vee Q\) | 和型 `{ a(P) \| b(Q) }` |
| 全称量化 \(\forall x:T. P(x)\) | 泛型 `(T: Type) -> ...` |
| 真 \(\top\) | 空型 `{}` |
| 偽 \(\bot\) | `Void` / `Never` |
| 型宇宙 \(Type_n : Type_{n+1}\) | 宇宙分层（Russell 逆理の防止） |
| 数学的帰納法 | 型レベル `match` |

### 0.2 型即ち命題、プログラム即ち証明

YaoXiangにおいて、この対応関係は設計の一級原則である：

- **型は論理命題である**。`Int` は「整数が存在する」という命題であり、`fn(a: Int, b: Int) -> Int` は「2つの整数が与えられたとき、ある整数が存在する」という命題である。
- **型検査は証明の検証である**。プログラムが型検査を通過することは、論理命題が構成的に証明されたことに相当する。
- **終了する型レベル計算は正しい帰納的推論に対応する**。YaoXiang の型族（`Nat` 上の `Add` のような）是認納法的証明の型レベルエンコーディング本质上である。

### 0.3 言語設計への影響

Curry-Howard 同型がYaoXiangにおける具現化：

1. **宇宙分层**（RFC-010）：`Type₀ : Type₁ : Type₂ …` は `Type: Type` がもたらす論理逆理（Girard 逆理）を回避する
2. **型族**（RFC-011）：自然数 `Nat(Zero/Succ)` の型レベルパターンマッチングはPeano公理下の帰納的証明に対応する
3. **条件型**（RFC-011）：`If: (C: Bool, T: Type, E: Type) -> Type` は論理におけるcase分析和に対応する
4. **値従属型**（RFC-011）：`Vec: (n: Int) -> Type` は「各整数 n に対してある型が存在する」の有限量化に対応する

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

### 2.1 原型

| 型 | 説明 | デフォルトサイズ |
|------|------|----------|
| `Type` | 元型 | 0 バイト |
| `Void` | 空値 | 0 バイト |
| `Bool` | 布尔値 | 1 バイト |
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

### 3.1 記録型

**統一構文**：`Name: Type = { field1: Type1, field2: Type2, ... }`

```
RecordType  ::= '{' FieldList? '}'
FieldList   ::= Field (',' Field)* ','?
Field       ::= Identifier ':' TypeExpr
            |  Identifier                 // インターフェース制約
```

```yaoxiang
// 単純記録型
Point: Type = { x: Float, y: Float }

// 空記録型
Empty: Type = {}

// 泛型記録型
Pair: (T: Type) -> Type = { first: T, second: T }

// インターフェースを実装する記録型
Point: Type = {
    x: Float,
    y: Float,
    Drawable,
    Serializable
}
```

**規則**：

- 記録型は波括弧 `{}` で定義する
- フィールド名の直後にコロンと型を続ける
- 型本体内にインターフェース名を記述するとそのインターフェースを実装したことになる

#### 3.1.1 フィールドデフォルト値

型のフィールドにはデフォルト値を指定でき、構築時に省略可能：

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

型定義本体内で直接メソッドを束縛できる：

```yaoxiang
// 方法1：外部関数を参照して束縛
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

**構文**：インターフェースとは、すべてのフィールドが関数型である記録型である

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

**インターフェース実装**：型は定義末尾にインターフェース名を列挙することでインターフェースを実装する

```yaoxiang
// インターフェースを実装する型
Point: Type = {
    x: Float,
    y: Float,
    Drawable,        // Drawable インターフェースを実装
    Serializable     // Serializable インターフェースを実装
}
```

**インターフェースへの直接代入**：具象型はインターフェース型の変数に直接代入可能（構造的サブタイピング）

```yaoxiang
// 直接代入（コンパイル時に具象型が確定 -> ゼロオーバーヘッド呼び出し）
d: Drawable = Circle(1)
d.draw(screen)        // コンパイル後：vtableなしでcircle_drawを直接呼び出し

// 関数戻り値（コンパイル時に確定不可 -> vtable呼び出し）
d: Drawable = get_shape()
d.draw(screen)        // vtableでメソッドを検索

// インターフェースを関数パラメータとして使用
process: (d: Drawable) -> Void = d.draw(screen)
```

**コンパイル時最適化戦略**：

| シナリオ | 推論結果 | 呼び出し方式 |
|------|----------|----------|
| 具象型への直接代入 | 具象型が確定可能 | 直接呼び出し（ゼロオーバーヘッド） |
| 関数戻り値 | 不明 | vtable |
| 異種混合コレクション | 複数型 | vtable |

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

## 第四章：泛型

### 4.1 泛型パラメータ構文

泛型パラメータは関数型の一部であり、通常のパラメータと統一的に `()` 構文を使用する：

```
GenericType     ::= Identifier '(' TypeArgList ')'
TypeArgList     ::= TypeExpr (',' TypeExpr)* ','?
TypeBound       ::= Identifier
                 |  Identifier '+' Identifier ('+' Identifier)*
```

泛型型定義において、`(T: Type)` は型構築子のパラメータシグネチャであり、`-> Type` は戻り型である：

```yaoxiang
List: (T: Type) -> Type = { ... }
Map: (K: Type, V: Type) -> Type = { ... }
```

泛型関数においても、型パラメータはシグネチャ内で宣言し、コンパイラは実引数から自動的に推論する：

```yaoxiang
map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R)) = ...
```

### 4.2 泛型型定義

```yaoxiang
// 基本泛型型
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
// コンパイラが泛型パラメータを自動的に推論
numbers: List(Int) = List(1, 2, 3)  // コンパイラがList(Int)を推論
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

// 泛型コンテナのソート
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
// Iterator trait（記録型構文を使用）
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

### 6.2 泛型関連型（GAT）

```yaoxiang
// より複雑な関連型
Container: (T: Type) -> Type = {
    Item: T,
    IteratorType: Iterator(T),  // 関連型も泛型
    iter: () -> IteratorType
}
```

---

## 第七章：コンパイル時泛型

### 7.1 コンパイル時定数パラメータ

```
LiteralType   ::= Identifier ':' Int          // コンパイル時定数
CompileTimeFn ::= '(' Identifier ':' Int ')' '(' Identifier ')' '->' TypeExpr
```

**核心設計**：`(n: Int)` 泛型パラメータ + `(n: n)` 値パラメータを使用して、コンパイル時定数と実行時値を区別する。

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

// 使用方法
arr: StaticArray(Int, factorial(5))  // コンパイラがコンパイル時にfactorial(5) = 120を計算
```

### 7.2 コンパイル時定数配列

```yaoxiang
// 行列型の使用
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
// 型レベルIf
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

## 第九章：型和共用と交差

### 9.1 型和共用

```
TypeUnion     ::= TypeExpr '|' TypeExpr
```

### 9.2 型交差

```
TypeIntersection ::= TypeExpr '&' TypeExpr
```

**構文**：型交差 `A & B` は A と B の両方を同時に満たす型を表す

```yaoxiang
// インターフェース合成 = 型交差
DrawableSerializable: Type = Drawable & Serializable

// 交差型の使用
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
// プラットフォーム型列挙体（標準ライブラリ定義）
Platform: Type = X86_64 | AArch64 | RISC_V | ARM | X86

// P は現在コンパイル中のプラットフォームを表す事前定義済み泛型パラメータ名
sum: (P: X86_64)(arr: Array(Float)) -> Float = {
    return avx2_sum(arr.data, arr.length)
}

sum: (P: AArch64)(arr: Array(Float)) -> Float = {
    return neon_sum(arr.data, arr.length)
}
```

---

## 第十一章：型属性

YaoXiangは区別すべき型属性は1種類のみである：線形 vs 複製可能。コンパイラが自動的に推論する。

### 11.1 Move（デフォルト所有権移動）

すべての型はデフォルトでMoveセマンティクスに従う。代入、引数渡しまたは戻り値 = 所有権移動。

```yaoxiang
p: Point = Point(1.0, 2.0)
q = p           // Move、pはこれ以降読み取り不可
```

### 11.2 Dup（シャローコピー：ハンドルの複製、データの共有）

**Dup属性は参照/トークン型に使用する**。Dup型の代入 = シャローコピー——ハンドルトークンを複製し、基底データを共有する。複数の保持者が同じデータブロックを指す。

| 型 | 属性 | 説明 |
|------|------|------|
| `&T` | Dup | ゼロサイズの読み取りトークン、トークンの複製 = 複数の視点で同じデータを指す |
| `ref T` | Dup | Rc/Arcの複製 = 参照カウント+1、ヒープデータを共有 |
| `&mut T` | Linear | ゼロサイズの書き込みトークン、排他的で複製不可 |
| その他すべての型 | Move | デフォルト所有権移動 |

**原語値型**（Int, Float, Bool, Char）はコンパイラの組み込みの特殊処理である：代入時に自動的に値が複製され、2つの値が完全に独立する。これはコンパイラのネイティブ動作であり、Dup型属性には属さない。

```yaoxiang
// &T: Dup、自由にエイリアス可能
view: &Point = &p
view2 = view     // Dup：トークンを複製、両方有効
print(view.x)    // 使用可能
print(view2.x)   // 使用可能

// &mut T: Linear、複製不可
mut_ref: &mut Point = &mut p
// r2 = mut_ref  // ❌ &mut TはDupではないため複製不可
```

### 11.3 Clone（明示的ディープコピー）とDupの関係

**Clone** は明示的ディープコピーインターフェースである。すべての型はCloneを実装でき、`.clone()` メソッドを提供する。

```yaoxiang
// Cloneインターフェース定義（標準ライブラリ）
Clone: Type = {
    clone: () -> Clone
}

// 使用
p: Point = Point(1.0, 2.0)
backup = p.clone()    // ディープコピー、pはまだ使用可能
p2 = p.clone()        // 複数回クローン可能
```

**DupとCloneの違い**：

| | Dup | Clone |
|---|---|---|
| **セマンティクス** | シャローコピー：ハンドルトークンを複製し、基底データを共有 | ディープコピー：完全に独立したコピーを作成 |
| **呼び出し方式** | 暗黙的（代入/引数渡しが自動） | 明示的（`.clone()`） |
| **変更の影響** | 互いに影響（基底データを共有） | 互いに影響なし（独立コピー） |
| **適用型** | `&T` トークン、`ref T` | Cloneインターフェースを実装する任意の型 |
| **コスト** | ゼロオーバーヘッド（トークンはゼロサイズ型） | 型により異なる |

**DupはCloneを含意せず、CloneはDupを含意しない**——これらは2つの直交する概念である：

```yaoxiang
// Dup型：トークンを複製、基底データを共有
view: &Point = &p
view2 = view        // Dup：トークンを複製、両方とも同じpを指す
print(view.x)       // 使用可能
print(view2.x)      // 使用可能、同じデータを参照

// 原語値型：コンパイラが自動的に値を複製（Dupではない）
x: Int = 42
y = x               // 値を複製、xとyは完全に独立
print(x)            // 使用可能

// Clone：明示的なディープコピー、独立コピーを作成
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone：ディープコピー、pはまだ使用可能
r = p               // Move：所有権移動、PointはDupでも原語値型でもないため
```

**設計意図**：

- Dupはトークン/参照型に使用し、「複数視点で同じデータをみる」問題を解決する
- Cloneは独立コピーが必要なシナリオに使用し、コストが目に見える形で明示的になる
- 原語値型（Int/Float/Bool/Char）のコピーはコンパイラの組み込み動作であり、Dupには属さない
- ほとんどのカスタム型はデフォルトでMove、ゼロコピーで高性能

## 第十二章：借用トークン型

### 12.1 核心概念

`&T` と `&mut T` は**ゼロサイズのコンパイル時トークン型**である。これらは「参照」ではなく、「アクセス権限の型レベル証明」である。

```
&T      →  ゼロサイズ、Dup（複製可能）、読み取り専用権限を付与
&mut T  →  ゼロサイズ、Linear（非Dup）、排他的読み書き権限を付与
```

**核心特性**：

- トークンは**通常の型**であり、他のすべての型と同じスコープ規則に従う
- ライフタイム注釈 `'a` は不要
- 専用の借用検査器は不要——型属性（Dup/Linear）が自然に権限を推論する
- コンパイル後完全に消失、実行時オーバーヘッドゼロ

### 12.2 基本使用

```yaoxiang
// メソッド側：パラメータ型を宣言し、必要な権限を決定
Point.print: (self: &Point) -> Void = {
    print(self.x)               // &Pointトークンが読み取り権限を付与
    print(self.y)
}

Point.shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
    self.x = self.x + dx        // &mut Pointトークンが書き込み権限を付与
    self.y = self.y + dy
}

// 呼び出し側：コンパイラが自動的に借用またはMoveを選択
p = Point(1.0, 2.0)
p.print()                       // コンパイラが自動的に&Pointトークンを作成
p.shift(1.0, 1.0)               // コンパイラが自動的に&mut Pointトークンを作成
p.print()                       // OK、前回のトークンはshift呼び出し終了時に解放済み

// 複数の&Tトークンの共存——Dup型なので自由に複製可能
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 トークンのスコープと伝播

トークンは通常の型であるため、すべての通常型の操作をサポート：

**トークンの 반환**——トークンは戻り値とともに呼び出し元に伝播する：

```yaoxiang
// ✅ サブトークンとペアレントトークンが一緒に返される
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // トークンが呼び出し元に返される
print(px_ref)                    // OK、トークンはまだスコープ内
```

**構造体への格納**——構造体はトークンフィールドを携带できる：

```yaoxiang
// ✅ 構造体がトークンをフィールドとして携带
Window: Type = {
    target: Point,
    view: &Point,              // トークンフィールド——targetへの読み取り専用ビューを保持
}
```

**クロージャキャプチャ**——クロージャは任意の値をキャプチャするのと同様にトークンをキャプチャ：

```yaoxiang
// ✅ クロージャが&Floatトークンをキャプチャ（Dup型なので自由にクロージャに複製可能）
filter_by_threshold: (items: List(Point), threshold: &Float) -> List(Point) = {
    items.filter(|p| p.x > threshold)
}
```

### 12.4 自動借用選択

呼び出し側コンパイラは次の優先順位で自動的に選択：

```
1. 実引数が以降も使用される場合 → トークンを作成優先（&Tまたは&mut T、メソッドシグネチャに基づく）
2. 実引数が以降使用されない場合 → Move
3. 優先マッチング順序：&T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // printは&selfを宣言 -> コンパイラが&Pointトークンを作成
p.shift(1.0, 1.0)  // shiftは&mut selfを宣言 -> コンパイラが&mut Pointトークンを作成
p2 = p             // 以降使用しない -> Move
```

### 12.5 トークン競合検出

コンパイラはトークン値に対して**フロー感受性ライフネス分析**を行い、各トークンの状態（アクティブ/移動済み）を追跡：

```yaoxiang
// ❌ &mutと派生&Tトークンは同時にアクティブになれない
bad_alias: (p: &mut Point) -> Void = {
    p.x = 10.0                   // ✅ 正常使用WriteToken
    print(p.y)
}

// ✅ トークンスコープ終了後に自動解放
good_seq: (p: &mut Point) -> Void = {
    {
        // 内部スコープ
        print(p.x)               // &mut Pointを使用
    }
    // 内部スコープ終了
    p.x = 10.0                   // ✅ WriteTokenは引き続き使用可能
}

// ❌ 同じ実引数に対して同時に&mutトークン以外のトークンを作成不可
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ pが&mutと&Tトークンを同時に派生させる
```

### 12.6 コンパイラ内部：ブランド機構

ユーザーはブランドに直接触れない。コンパイラは内部で各トークンにコンパイル時一意の識別子を割り当て：

```
ユーザーが見るもの           コンパイラ内部表現
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #Nはコンパイル時一意の整数
&mut Point     →  WriteToken(Point, #M)   // #Mはコンパイル時一意の整数
```

ブランドの用途：

- **偽造防止**：トークンは所有者のカプセルからのみ取得でき、空から構築できない
- **関連追跡**：フィールドアクセスから派生した `&Float` は派生ブランド（`#N.field_x`）を携带し、コンパイラは親トークンまで追跡可能
- **競合検出**：同一来源のWriteTokenと派生のReadTokenは同時にアクティブになれない

ブランドは単態化とインライン展開後に完全に消失し、生成された機械語には存在しない。**実行時オーバーヘッドゼロ。**

### 12.7 トークンSum型

```
&BorrowToken ::= &T          // ReadToken（Dup、複製可能）
               | &mut T      // WriteToken（Linear、排他的）
```

### 12.8 借用トークン vs ref

| | `&T` / `&mut T` | `ref` |
|------|------|------|
| 役割 | 一時参照/インプレイスは変更 | 共有所有 |
| 範囲 | トークン値のスコープに従う | スコープ間 |
| コスト | ゼロオーバーヘッド（ゼロサイズ型、コンパイル後消失） | RcまたはArc（コンパイラが選択） |
| エスケープ | 可能（トークンは戻り値/構造体/クロージャ跟着して伝播） | 本来エスケープ用 |
| タスク間 | 不可（トークンはSend未実装） | 可能（コンパイラが自動的にArcを選択） |
| サイクル検出 | 関係なし | タスク内は無視、タスク間をlintで検出 |

---

## 付録：型定義早見表

### A.1 型定義

```
// === 記録型（波括弧） ===

// 記録型
Point: Type = { x: Float, y: Float }

// 变体のある記録型（函数字段を使用）
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// === インターフェース型（波括弧、フィールドがすべて関数） ===

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

### A.2 泛型構文

```
// 泛型型
List: (T: Type) -> Type = { data: Array(T), length: Int }
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// 泛型関数
map: (T: Type, R: Type)(list: List(T), f: (T) -> R) -> List(R) = { ... }

// 型制約
clone: (T: Clone)(value: T) -> T = value.clone()
combine: (T: Clone + Add)(a: T, b: T) -> T = body

// 関連型
Iterator: (T: Type) -> Type = { Item: T, next: () -> Option(T) }

// コンパイル時泛型
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
// すべての型はデフォルトでMove。代入、引数渡し、戻り値 = 所有権移動

// === 原語値型（コンパイラの組み込み） ===
Int, Float,     // 代入時に自動的に値が複製され、2つの値が完全に独立
Bool, Char      // Dupではなく、原語のコンパイラの組み込み処理

// === Dup（シャローコピー：ハンドルを複製、基底データを共有） ===
&T              // ゼロサイズ読み取りトークン、トークンの複製 = 複数視点で同じデータを指す
ref T           // Rc/Arcの複製 = 参照カウント+1、ヒープデータを共有

// === Linear ===
&mut T          // ゼロサイズ書き込みトークン、Linear（排他的、複製不可）

// === Clone（明示的ディープコピー） ===
value.clone()   // 独立コピーを作成、変更は原値に影響しない
```

### A.4 借用トークン早見表

```
// === 借用トークン ===
&T              // ゼロサイズコンパイル時読み取りトークン、Dup（複製可能）
&mut T          // ゼロサイズコンパイル時書き込みトークン、Linear（複製不可）

// 呼び出し側の自動選択
// 1. 実引数が以降も使用 -> トークンを作成
// 2. 実引数が以降使用しない -> Move
// 3. 優先マッチング：&T < &mut T < Move

// トークン伝播
// ✅ 戻り値可、構造体格納可、クロージャキャプチャ可
// ❌ タスク間不可（Send未実装）
```
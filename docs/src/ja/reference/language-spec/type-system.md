# タイプシステム仕様

本ドキュメントでは、YaoXiang プログラミング言語のタイプシステム仕様を定義します。基本型、複合型、ジェネリクス、traitを含みます。

---

## 第零章：理論的基盤

### 0.1 Curry-Howard 同型対応

Curry-Howard 同型対応（Curry-Howard correspondence）は、YaoXiang のタイプシステムの理論的基盤です。これはプログラミング言語のタイプシステムと数理論理学の間の深い対応関係を明らかにします：

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
| 型の宇宙 \(Type_n : Type_{n+1}\) | 宇宙階層化（Russell 逆理の回避） |
| 数学的帰納法 | 型レベル `match` |

### 0.2 型即ち命題、プログラム即ち証明

YaoXiang では、この対応関係は設計の第一原則です：

- **型は論理命題そのものである**。`Int` は「整数が存在する」という命題であり、`fn(a: Int, b: Int) -> Int` は「2つの整数が与えられたとき、整数が一つ存在する」という命題である。
- **型チェックは証明の検証である**。プログラムが型チェックをパスすることは、論理命題が構成的に証明されたことに相当する。
- **項自由な型レベル計算は正しい帰納推論に対応する**。YaoXiang の型族（`Nat` 上の `Add` のようなパターンマッチング）は、数学的帰納法の型レベル符号化本質である。

### 0.3 言語設計への影響

Curry-Howard 同型対応は YaoXiang において以下のように具体化している：

1. **宇宙階層化**（RFC-010）：`Type₀ : Type₁ : Type₂ …` により `Type: Type` に起因する論理逆理（Girard 逆理）を回避
2. **型族**（RFC-011）：自然数 `Nat(Zero/Succ)` の型レベルパターンマッチングは Peano 公理下での帰納証明に対応
3. **条件型**（RFC-011）：`If: (C: Bool, T: Type, E: Type) -> Type` は論理における case による選言に対応
4. **値依存型**（RFC-011）：`Vec: (n: Int) -> Type` は「各整数 n に対して型が存在する」の有限量化に対応

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

### 2.1 原型（Primitive Types）

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

### 3.1 記録型（Record Type）

**統一構文**：`Name: Type = { field1: Type1, field2: Type2, ... }`

```
RecordType  ::= '{' FieldList? '}'
FieldList   ::= Field (',' Field)* ','?
Field       ::= Identifier ':' TypeExpr
            |  Identifier                 // インターフェース制約
```

```yaoxiang
// 単純な記録型
Point: Type = { x: Float, y: Float }

// 空記録型
Empty: Type = {}

// ジェネリックな記録型
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
- フィールド名の直後にコロンと型を記述する
- インターフェース名は型本体に記述することでそのインターフェースを実装する

#### 3.1.1 フィールドのデフォルト値

型フィールドにはデフォルト値を指定でき、構築時に省略可能：

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

**規則**：
- `field: Type = expression` -> デフォルト値あり、構築時に省略可能
- `field: Type` -> デフォルト値なし、構築時に必須

#### 3.1.2 内束縛（Builtin Binding）

型定義本体の中で直接メソッドを束縛できる：

```yaoxiang
// 方法1：外部関数を参照して束縛
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]    // 位置0に束縛
}
// 呼び出し：p1.distance(p2) -> distance(p1, p2)

// 方法2：無名関数 + 位置束縛
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

### 3.2 列挙型（Variant Types）

```
EnumType    ::= '{' Variant ('|' Variant)* '}'
Variant     ::= Identifier (':' TypeExpr)?
```

**構文**：`Name: Type = { Variant1 | Variant2(params) | ... }`

```yaoxiang
// パラメータなしバリアント
Color: Type = { red | green | blue }

// パラメータ付きバリアント
Option: (T: Type) -> Type = { some(T) | none }

// 混合
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

// パラメータなしバリアントはパラメータなしコンストラクタと等価
Bool: Type = { true | false }
```

### 3.3 インターフェース型

```
InterfaceType ::= '{' FnField (',' FnField)* ','?
FnField       ::= Identifier ':' FnType
FnType        ::= '(' ParamTypes? ')' '->' TypeExpr
```

**構文**：インターフェースはフィールドがすべて関数型である記録型である

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

**インターフェースへの直接代入**：具体型はインターフェース型の変数に直接代入可能（構造的サブタイピング）

```yaoxiang
// 直接代入（コンパイル時に具体型が確定 -> ゼロオーバーヘッド呼び出し）
d: Drawable = Circle(1)
d.draw(screen)        // コンパイル後：circle_draw を直接呼び出し、vtable なし

// 関数の戻り値（コンパイル時に確定できない -> vtable 呼び出し）
d: Drawable = get_shape()
d.draw(screen)        // vtable でメソッドを検索

// インターフェースを関数パラメータとして使用
process: (d: Drawable) -> Void = d.draw(screen)
```

**コンパイル時最適化戦略**：

| シナリオ | 推論結果 | 呼び出し方式 |
|------|----------|----------|
| 具体型を直接代入 | 具体型が確定可能 | 直接呼び出し（ゼロオーバーヘッド） |
| 関数の戻り値 | 不明 | vtable |
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

## 第四章：ジェネリクス

### 4.1 ジェネリックパラメータ構文

```
GenericType     ::= Identifier '[' TypeArgList ']'
TypeArgList     ::= TypeExpr (',' TypeExpr)* ','?
GenericParams   ::= '[' Identifier (',' Identifier)* ']'
                 |  '[' Identifier ':' TypeBound (',' Identifier ':' TypeBound)* ']'
TypeBound       ::= Identifier
                 |  Identifier '+' Identifier ('+' Identifier)*
```

### 4.2 ジェネリック型定義

```yaoxiang
// 基本的なジェネリック型
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

### 4.3 型推論

```yaoxiang
// コンパイラがジェネリックパラメータを自動推論
numbers: List(Int) = List(1, 2, 3)  // コンパイラが List(Int) を推論
```

---

## 第五章：型制約

### 5.1 単一制約

```
ConstrainedType ::= '[' Identifier ':' TypeBound ']' TypeExpr
```

```yaoxiang
// インターフェース型定義（制約として使用）
Clone: Type = {
    clone: (Self) -> Self
}

// 制約を使用
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
call_twice: (T: Type, F: Fn() -> T)(f: F) -> (T, T) = (f(), f())

compose: (A: Type, B: Type, C: Type, F: Fn(A) -> B, G: Fn(B) -> C)(a: A, f: F, g: G) -> C = g(f(a))
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
    next: (Self) -> Option(T),
    has_next: (Self) -> Bool
}

// 関連型を使用
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
    iter: (Self) -> IteratorType
}
```

---

## 第七章：コンパイル時ジェネリクス

### 7.1 リテラル型制約

```
LiteralType   ::= Identifier ':' Int          // コンパイル時定数
CompileTimeFn ::= '[' Identifier ':' Int ']' '(' Identifier ')' '->' TypeExpr
```

**コア設計**：(n: Int) ジェネリックパラメータ + (n: n) 値パラメータを使用して、コンパイル時定数と実行時値を区別する。

```yaoxiang
// コンパイル時階乗：パラメータはコンパイル時に既知のリテラルである必要がある
factorial: (n: Int)(n: n) -> Int = {
    match n {
        0 => 1,
        _ => n * factorial(n - 1)
    }
}

// コンパイル時定数配列
StaticArray: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // コンパイル時にサイズが確定している配列
    length: N
}

// 使用方法
arr: StaticArray(Int, factorial(5))  // コンパイラがコンパイル時に factorial(5) = 120 を計算
```

### 7.2 コンパイル時定数配列

```yaoxiang
// 行列型の使用
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows)
}

// コンパイル時の次元検証
identity_matrix: (T: Add + Zero + One, N: Int)(size: N) -> Matrix(T, N, N) = {
    // ...
}
```

---

## 第八章：条件型

### 8.1 If 条件型

```
IfType        ::= 'If' '[' BoolExpr ',' TypeExpr ',' TypeExpr ']'
```

```yaoxiang
// 型レベル If
If: (C: Bool, T: Type, E: Type) -> Type = match C {
    True => T,
    False => E
}

// 示例：コンパイル時分岐
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

## 第九章：型和共用と交叉

### 9.1 型和共用

```
TypeUnion     ::= TypeExpr '|' TypeExpr
```

### 9.2 型交叉

```
TypeIntersection ::= TypeExpr '&' TypeExpr
```

**構文**：型の交叉 `A & B` は A と B の両方を同時に満たす型を表す

```yaoxiang
// インターフェース組合 = 型交叉
DrawableSerializable: Type = Drawable & Serializable

// 交叉型を使用
process: (T: Drawable & Serializable)(item: T, screen: Surface) -> String = {
    item.draw(screen)
    return item.serialize()
}
```

---

## 第十章：関数オーバーロードと特殊化

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
// プラットフォーム型列挙型（標準ライブラリ定義）
Platform: Type = X86_64 | AArch64 | RISC_V | ARM | X86

// P は事前定義されたジェネリックパラメータ名で、現在のコンパイルプラットフォームを表す
sum: (P: X86_64)(arr: Array(Float)) -> Float = {
    return avx2_sum(arr.data, arr.length)
}

sum: (P: AArch64)(arr: Array(Float)) -> Float = {
    return neon_sum(arr.data, arr.length)
}
```

---

## 第十一章：型の属性

YaoXiang は型のコピーと並行性のセマンティクスをマークする**型の属性**（type properties）を持つ。型の属性はコンパイラが自動推論し、ユーザーが直接マークすることはない。

### 11.1 Dup（暗黙的浅コピー）

**Dup**（Duplicable）は暗黙的浅コピーマークである。Dup を実装する型は、代入と引数渡しの際に自動的に浅コピー（ビット単位コピー）を実行し、元の値と新しい値は完全に独立する。

**Dup 型**：
| 型 | 説明 |
|------|------|
| `Int`, `Int8`..`Int128` | すべての整数 |
| `Float`, `Float32`, `Float64` | すべての浮動小数点 |
| `Bool` | 真理値 |
| `Char` | Unicode 文字 |
| `String` | UTF-8 文字列（浅コピー） |
| `Bytes` | 生バイト（浅コピー） |
| `&T` | 読み取りトークン（第十二章参照） |

**非 Dup 型**（デフォルト Move）：
| 型 | 説明 |
|------|------|
| `&mut T` | 書き込みトークン、線形型 |
| ほとんどの struct | デフォルト Move、すべてのフィールドが Dup の場合は例外 |
| enum（パラメータ付きバリアント） | 保持するデータが非 Dup の場合、バリアント全体が非 Dup |

```yaoxiang
// Dup 型：自由にコピー可能
a: Int = 42
b = a           // 浅コピー、a は引き続き使用可能
c = a           // 複数回コピー可能

// 非 Dup 型（デフォルト Move）
p: Point = Point(1.0, 2.0)
q = p           // Move、p はこれ以上読み取り不可
// r = p        // ❌ コンパイルエラー：p はすでに移動済み
```

**Dup の自動推論**：
- 基本型（Int, Float, Bool, Char, String, Bytes）は自動的に Dup を実装
- 構造体：すべてのフィールド型が Dup の場合のみ、自動的に Dup を実装
- 列挙型バリアント：すべてのバリアントが保持する型が Dup の場合のみ、自動的に Dup を実装

### 11.2 Clone（明示的深コピー）

**Clone** は明示的深コピーインターフェースである。すべての型は Clone を実装でき、`.clone()` メソッドを提供する。

```yaoxiang
// Clone インターフェース定義（標準ライブラリ）
Clone: Type = {
    clone: (Self) -> Self
}

// 使用例
p: Point = Point(1.0, 2.0)
backup = p.clone()    // 深コピー、p は引き続き使用可能
p2 = p.clone()        // 複数回クローン可能
```

### 11.3 Dup と Clone の関係

**Dup は Clone を蕴含するが、Clone は Dup を蕴含しない**：

```
Dup ⇒ Clone（フィールドをビット単位でコピーすれば .clone() を実装できる）
Clone ⇏ Dup（明示的深コピーはデフォルト Move セマンティクスを妨げない）
```

```yaoxiang
// Dup ⇒ Clone：Int は Dup でもあり Clone でもある
x: Int = 42
y = x              // Dup：暗黙的浅コピー
z = x.clone()      // Clone：明示的深コピー（効果は同じ）

// Clone ⇏ Dup：Point は Clone できるがデフォルトは Move
p: Point = Point(1.0, 2.0)
q = p.clone()      // Clone：明示的深コピー、p は引き続き使用可能
r = p              // Move：所有権移転，因為 Point は Dup ではない
```

**設計意図**：
- Dup は「この型はコピーするのが安い/自然である」という約束
- Clone は「独立したコピーを作成できる」という能力
- ほとんどの struct は Dup を自動実装せず、Move をデフォルトとする——ゼロコピーの高性能

### 11.4 Send / Sync（ユーザーには非表示）

**Send** と **Sync** はユーザーには見えない型の属性であり、コンパイラと `ref` キーワードによって自動的に処理される。

| 属性 | 意味 | ユーザーのトリガー方法 |
|------|------|-------------|
| **Send** | タスク間で安全に传递可能 | `ref` がタスク間を移動する際にコンパイラが Arc を自動選択 |
| **Sync** | タスク間で安全に共有可能 | `ref` がタスク間を移動する際にコンパイラが Arc を自動選択 |

**自動推論規則（コンパイラ内部）**：

| 型 | Send | Sync | 説明 |
|------|------|------|------|
| 値型（Int, Float, Point...） | はい | はい | 値渡しは本質的に安全 |
| `ref T` | はい | はい | コンパイラが Rc（単一タスク）/ Arc（タスク間）を自動選択 |
| `&T` / `&mut T` | いいえ | いいえ | トークンはタスク境界をまたげない |
| `*T` | いいえ | いいえ | 生ポインタはシングルスレッド |

```yaoxiang
// タスク間共有：ref が Send/Sync を自動処理
@block
main: () -> Void = {
    data = ref heavy_data
    spawn { use(data) }    // コンパイラ：タスク間 -> Arc（Send + Sync）
    spawn { use(data) }    // コンパイラ：タスク間 -> Arc（Send + Sync）
}

// トークンはタスク間を移動できない（非 Send）
bad_task: (p: &Point) -> Void = {
    spawn { print(p.x) }   // ❌ コンパイルエラー：&T は Send を実装していない
}
```

**ユーザーは Send/Sync を気にする必要はない**：`ref` キーワードがすべての並行性安全ロジックをカプセル化している。

---

## 第十二章：借用トークン型

### 12.1 コア概念

`&T` と `&mut T` は**ゼロサイズのコンパイル時トークン型**である。これらは「参照」ではなく、「アクセス権限の型レベル証明」である。

```
&T      →  ゼロサイズ、Dup（コピー可能）、読み取り専用権限を付与
&mut T  →  ゼロサイズ、Linear（非 Dup）、排他的読み書き権限を付与
```

**主な特性**：
- トークンは**通常の型**であり、他のすべての型と同じスコープ規則に従う
- ライフタイム注記 `'a` が不要
- 専用の借用チェッカーが不要——型の属性（Dup/Linear）が自然に権限を推論
- コンパイル後は完全に消失し、実行時オーバーヘッドゼロ

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

// 複数の &T トークンの共存——Dup 型は自由にコピー可能
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 トークンのスコープと伝播

トークンは通常の型であるため、すべての通常の型の操作をサポート：

**トークンの戻り値**——トークンは戻り値と一緒に伝播：

```yaoxiang
// ✅ 子トークンと親トークンは一緒に返される
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // トークンが呼び出し元に返される
print(px_ref)                    // OK、トークンはまだスコープ内
```

**構造体への格納**——構造体はトークンフィールドを持てる：

```yaoxiang
// ✅ 構造体はトークンをフィールドとして保持可能
Window: Type = {
    target: Point,
    view: &Point,              // トークンフィールド——target への読み取り専用ビューを保持
}
```

**クロージャによるキャプチャ**——クロージャは他の値と同様にトークンをキャプチャ：

```yaoxiang
// ✅ クロージャが &Float トークンをキャプチャ（Dup 型、自由롭게クロージャにコピー可能）
filter_by_threshold: (items: List(Point), threshold: &Float) -> List(Point) = {
    items.filter(|p| p.x > threshold)
}
```

### 12.4 自動借用選択

呼び出し側コンパイラは次の優先順位で自動選択：

```
1. 実引数が後続で使用される場合 → トークンを作成優先（&T または &mut T、メソッド署名に基づく）
2. 実引数が後続で使用されない場合 → Move
3. 優先マッチ順序：&T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // print は &self を宣言 -> コンパイラが &Point トークンを作成
p.shift(1.0, 1.0)  // shift は &mut self を宣言 -> コンパイラが &mut Point トークンを作成
p2 = p             // 後続で使用されない -> Move
```

### 12.5 凍結機構

`&mut T` トークンは一時的に「凍結」して `&T` トークンを生成できる：

```yaoxiang
modify_and_read: (p: &mut Point) -> Void = {
    p.x = 10.0                   // &mut Point を使用して変更
    
    // &mut を凍結して読み取り専用ビューを取得
    view: &Point = freeze(p)     // p はここで凍結される
    print(view.x)                // &Point を通じて読み取り
    print(view.y)
    // view がスコープを離れる、凍結が解除される
    
    p.y = 20.0                   // &mut Point が再び使用可能に
}
```

`freeze` のセマンティクス：
- `&mut T` を受け取り、`&T` を返す
- `&T` が存続する間、元の `&mut T` は使用不可
- `&T` がスコープを離れると、`&mut T` が自動的に回復
- これは**フロー依存生存分析**——コンパイラが関数本体でトークン状態を追跡

### 12.6 トークン衝突検出

コンパイラはトークン値に対して**フロー依存生存分析**を実行し、各トークンの状態（生存中/凍結済み/移動済み）を追跡：

```yaoxiang
// ❌ &mut と派生の &T を同時に生存させることはできない
bad_alias: (p: &mut Point) -> Void = {
    view: &Point = freeze(p)     // p が凍結される
    p.x = 10.0                   // ❌ コンパイルエラー：WriteToken は凍結状態
    print(view.x)
}

// ✅ 凍結解除後に &mut を引き続き使用可能
good_seq: (p: &mut Point) -> Void = {
    view: &Point = freeze(p)     // p が凍結される
    print(view.x)                // &T を使用
    // view がスコープを離れる、凍結が解除される
    p.x = 10.0                   // ✅ WriteToken が回復
}

// ❌ 同一の実引数から同時に &mut トークンと他のトークンを作成することはできない
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p から同時に &mut と & トークンが派生
```

### 12.7 コンパイラ内部：ブランド機構

ユーザーはブランドに触れることはない。コンパイラは内部で各トークンにコンパイル時一意の識別子を割り当て：

```
ユーザーが見るもの         コンパイラの内部表現
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N はコンパイル時一意の整数
&mut Point     →  WriteToken(Point, #M)   // #M はコンパイル時一意の整数
```

ブランドの用途：
- **偽造防止**：トークンは所有者のポインタまたは freeze 操作からのみ取得でき凭空に構築不可
- **関連追跡**：フィールドアクセスから派生した `&Float` は派生ブランド（`#N.field_x`）を携带し、コンパイラは親トークンへ追跡可能
- **衝突検出**：同源的 WriteToken と派生的 ReadToken は同時に生存不可

ブランドは単態化とインライン展開後に完全に消失し、生成された機械語には存在しない。**実行時オーバーヘッドゼロ。**

### 12.8 トークンの Sum 型

```
&BorrowToken ::= &T          // ReadToken（Dup、コピー可能）
               | &mut T      // WriteToken（Linear、排他的）
```

### 12.9 借用トークン vs ref

| | `&T` / `&mut T` | `ref` |
|------|------|------|
| 何をするか | 一時的な参照 / その場で変更 | 共有所有 |
| 範囲 | トークン値のスコープに従う | スコープをまたぐ |
| コスト | ゼロオーバーヘッド（ゼロサイズ型、コンパイル後に消失） | Rc または Arc（コンパイラが選択） |
| エスケープ | 可能（トークンは戻り値/構造体/クロージャ伴随でエスケープ可能） | そもそもエスケープ用 |
| タスク間 | 不可（トークンは Send を実装していない） | 可能（コンパイラが自動的に Arc を選択） |
| 循環検出 | 関係なし | タスク内は無視、タスク間を lint |

---

## 付録：型定義クイックリファレンス

### A.1 型定義

```
// === 記録型（波括弧） ===

// 構造体
Point: Type = { x: Float, y: Float }

// 列挙型（バリアント型）
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }
Status: Type = { pending | processing | completed }

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

### A.2 ジェネリック構文

```
// ジェネリック型
List: (T: Type) -> Type = { data: Array(T), length: Int }
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

// ジェネリック関数
map: (T: Type, R: Type)(list: List(T), f: Fn(T) -> R) -> List(R) = { ... }

// 型制約
clone: (T: Clone)(value: T) -> T = value.clone()
combine: (T: Clone + Add)(a: T, b: T) -> T = body

// 関連型
Iterator: (T: Type) -> Type = { Item: T, next: () -> Option(T) }

// コンパイル時ジェネリック
factorial: (n: Int)(n: n) -> Int = { ... }
StaticArray: (T: Type, N: Int) -> Type = { data: T(N), length: N }

// 条件型
If: (C: Bool, T: Type, E: Type) -> Type = match C { True => T, False => E }

// 関数特殊化
sum: (arr: Array(Int)) -> Int = { ... }
sum: (arr: Array(Float)) -> Float = { ... }
```

### A.3 型の属性クイックリファレンス

```
// === Dup（暗黙的浅コピー）===
// 基本型は自動的に Dup
Int, Float, Bool, Char, String, Bytes   // Dup
&T                                      // Dup（共有読み取りトークン）

// 非 Dup（デフォルト Move）
&mut T                                  // Linear（排他的書き込みトークン）
ほとんどの struct                            // Move デフォルト

// Dup は Clone を蕴含（フィールドをビット単位でコピーすれば .clone() を実装可能）、だが Clone は Dup を蕴含しない

// === Clone（明示的深コピー）===
value.clone()                           // 明示的深コピー

// === Send / Sync（ユーザーには非表示）===
// ref キーワードとコンパイラが自動処理
// 値型：Send + Sync
// ref T：Send + Sync（コンパイラが Rc/Arc を自動選択）
// &T / &mut T：非 Send（タスク間をまたげない）
// *T：非 Send（生ポインタはシングルスレッド）
```

### A.4 借用トークンクイックリファレンス

```
// === 借用トークン ===
&T              // ゼロサイズコンパイル時読み取りトークン、Dup（コピー可能）
&mut T          // ゼロサイズコンパイル時書き込みトークン、Linear（コピー不可）

// 呼び出し側の自動選択
// 1. 実引数が後続で使用される -> トークンを作成
// 2. 実引数が後続で使用されない -> Move
// 3. 優先マッチ：&T < &mut T < Move

// トークンの伝播
// ✅ 戻り値可能、構造体に格納可能、クロージャにキャプチャ可能
// ❌ タスク間をまたげない（Send を実装していない）

// 凍結
view: &T = freeze(mut_ref)   // &mut T -> &T（凍結期間中は &mut T が使用不可）
```
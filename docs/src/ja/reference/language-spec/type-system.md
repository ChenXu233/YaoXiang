# 型システム仕様

本文書は YaoXiang プログラミング言語の型システム仕様を定義ものであり、基本型、複合型、ジェネリクス、traitを含む。

---

## 第零章：理論的基礎

### 0.1 Curry-Howard 同型

Curry-Howard 同型（Curry-Howard
correspondence）は YaoXiang の型システムの理論的基盤である。これはプログラミング言語の型システムと数理論理学の間の深い対応関係を示すものである：

| 論理学                           | プログラミング言語                    |
| -------------------------------- | ------------------------------------- |
| 命題 \(P\)                       | 型 `Type`                             |
| 証明 \(p: P\)                    | プログラム `x: T = ...`                |
| 含意 \(P \rightarrow Q\)        | 関数型 `(P) -> Q`                     |
| 連言 \(P \wedge Q\)              | 積型 `{ a: P, b: Q }`                  |
| 選言 \(P \vee Q\)                | 和型 `{ a(P) \| b(Q) }`                |
| 全称量化 \(\forall x:T. P(x)\)   | ジェネリクス `(T: Type) -> ...`        |
| 真 \(\top\)                      | `Void`（Unit、既定値あり）             |
| 偽 \(\bot\)                      | `Never`（零コンストラクタ、居住不可） |
| 型宇宙 \(Type_n : Type_{n+1}\)   | 宇宙階層（Russell 逆理の回避）         |
| case 分析                        | 型レベル `match`                      |

> **注意**：型レベル `match` は case 分析であり、数学的帰納法ではない。帰納法には型レベル再帰関数 +
> コンパイラの停止性検査が必要である。

### 0.2 型は命題、プログラムは証明

YaoXiang において、この対応関係は設計の一級原則である：

- **停止する型レベル計算は正しい構成的証明に対応する**。YaoXiang の型族（例：`Nat` 上の `Add` の case
  分析 +
  再帰呼び出し）は数学的帰納法の型レベル符号化本质上——前提としてコンパイラが停止性検査を行えること。
- **型検査は証明の検証である**。プログラムが型検査を通るとは、論理命題が構成的に証明されたことに相当する。

### 0.3 言語設計への影響

Curry-Howard 同型は YaoXiang において以下に具体化される：

1. **宇宙階層**（RFC-010）：`Type₀ : Type₁ : Type₂ …` により `Type: Type`
   による論理的逆理（Girard 逆理）を回避
2. **型族**（RFC-011）：自然数 `Nat(Zero/Succ)` の型レベル case 分析 +
   再帰呼び出しは Peano
   公理に対応——前提としてコンパイラが停止性検査を行うこと
3. **条件型**（RFC-011）：`If: (C: Bool, T: Type, E: Type) -> Type` は論理における case 選言に対応
4. **値依存型**（RFC-011）：`Vec: (n: Int) -> Type` は「各整数 n
   に対して型が存在する」有限量化に対応

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

> **設計上の注意**：RFC-010 で「すべてが代入である」という統一モデル（`name: type = value`）が提案されているが、構文レベルでは型と値は区別する必要がある。コンパイラ実装では
> `Type` と `Expr` は独立した2つの AST 列挙型（`ast.rs:406` と `ast.rs:25`）であり、`TypeExpr`
> は BNF のプレースホルダとして実装の `Type` 列挙型に対応し、「この位置には型が来る」を意味する。

---

## 第二章：基本型

### 2.1 プリミティブ型

| 型        | 論理的対応       | 説明                                                                              | 既定サイズ |
| --------- | ---------------- | --------------------------------------------------------------------------------- | --------- |
| `Type`    | —                | メタ型                                                                            | 0 バイト  |
| `Never`   | ⊥（偽/空型）     | 零コンストラクタ、値なし。発散/panic 戻り値型。任意の T に対して `Never <: T`。  | 0 バイト  |
| `Void`    | ⊤（真/Unit）     | 既定の void 値あり、零フィールド積型。`x: Void = <デフォルト>` は合法。          | 0 バイト  |
| `Bool`    | —                | 真偽値：`true` / `false`                                                          | 1 バイト  |
| `Int`     | —                | 符号付き整数                                                                      | 8 バイト  |
| `Uint`    | —                | 符号なし整数                                                                      | 8 バイト  |
| `Float`   | —                | 浮動小数点数                                                                      | 8 バイト  |
| `String`  | —                | UTF-8 文字列                                                                      | 可変      |
| `Char`    | —                | Unicode 文字                                                                      | 4 バイト  |
| `Bytes`   | —                | 生バイト                                                                          | 可変      |

ビット幅付き整数：`Int8`, `Int16`, `Int32`, `Int64`, `Int128`
ビット幅付き浮動小数点：`Float32`, `Float64`

### 2.2 Never と Void：⊥ と ⊤

`Never` と `Void` は型システムの論理的原元であり、それぞれ偽（⊥）と真（⊤）に対応する。

**Never（⊥、偽/空型）** — 交渉の余地のない3つの性質：

1. **零コンストラクタ**：リテラルや式が `Never` 型の値を生成することはできない。`x: Never = ...` には右辺を書けない。
2. **爆発原理**：任意の型 `T` に対して `Never <: T` が成立する。`assert(false)` は
   `Never` を返すので、その先のコードは型検査を通る（実行はされないが）。
3. **発散マーク**：`f: (...) -> Never` は `f` が戻らないことを保証する。コンパイラはこれに基づいて dead
   code 分析と `match` 枝合流を行う。

`Never` は組み込み型名（`Int`/`Bool` と同じ登録パス）であり、キーワードではない。

**Void（⊤、真/Unit）** — 丁度1つの居住者（既定 void 値）。`Void`
は零フィールド積型の単位元である。`x: Void = <デフォルト>` は合法であり、関数がデフォルトで `return`
を持たない場合は `Void` を返す。

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

// ジェネリクス付きレコード型
Pair: (T: Type) -> Type = { first: T, second: T }

// インターフェースを実装するレコード型
Point: Type = {
    x: Float,
    y: Float,
    Drawable,
    Serializable
}
```

**規則**：

- レコード型は波括弧 `{}` で定義する
- フィールド名の直後にコロンと型を書く
- 型本体内にインターフェース名を書くとそのインターフェースを実装する

> **名前空間所属**：`Type.name` プレフィックス（例：`Point.draw`）は関数が `Point`
> の名前空間に属すことを示す。暗黙のバインディングは発生しない。`p.draw()` のような `.`
> 呼び出し構文を動作させるには、明示的にバインディングする必要がある：
> `Point.draw = draw[0]`。詳細は RFC-004 と RFC-010 を参照。

#### 3.1.1 フィールド既定値

型フィールドには既定値を指定でき、構築時に省略可能：

```yaoxiang
// 既定値のあるフィールド - 構築時に省略可能
Point: Type = {
    x: Float = 0,
    y: Float = 0
}

// 使用
Point()           // -> Point(x=0, y=0)
Point(x=1)       // -> Point(x=1, y=0)
Point(x=1, y=2) // -> Point(x=1, y=2)

// 既定値のないフィールド - 構築時に必須
Point2: Type = {
    x: Float,
    y: Float
}

// 使用
Point2(x=1, y=2) // 正しい
Point2()          // エラー
```

**規則**：

- `field: Type = expression` -> 既定値あり、構築時に省略可能
- `field: Type` -> 既定値なし、構築時に必須

#### 3.1.2 組み込みバインディング

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

// 方法2：無名関数 + 位置バインディング
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

**構文**：インターフェースはフィールドがすべて関数型であるレコード型

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

**インターフェースへの直接代入**：具体型はインターフェース型変数に直接代入可能（構造的サブ型）

```yaoxiang
// 直接代入（コンパイル時に具体型を特定可能 -> ゼロオーバーヘッド呼び出し）
d: Drawable = Circle(1)
d.draw(screen)        // コンパイル後：vtableなしで直接 circle_draw を呼び出す

// 関数戻り値（コンパイル時に特定不可 -> vtable呼び出し）
d: Drawable = get_shape()
d.draw(screen)        // vtableでメソッドを検索

// インターフェースを関数パラメータとして使用
process: (d: Drawable) -> Void = d.draw(screen)
```

**コンパイル時最適化戦略**：

| シナリオ         | 推論結果         | 呼び出し方式           |
| ---------------- | ---------------- | ---------------------- |
| 具体型を直接代入 | 具体型を特定可能 | 直接呼び出し（ゼロオーバーヘッド） |
| 関数戻り値       | 不明             | vtable                 |
| 不均一集合       | 複数型           | vtable                 |

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

### 4.1 ジェネリクス構文

ジェネリクス引数は関数型の一部であり、通常の引数と同様に `()` 構文を使用する：

```
GenericType     ::= Identifier '(' TypeArgList ')'
TypeArgList     ::= TypeExpr (',' TypeExpr)* ','?
TypeBound       ::= Identifier
                 |  Identifier '+' Identifier ('+' Identifier)*
```

ジェネリクス型定義において、`(T: Type)` は型コンストラクタの引数シグネチャであり、`-> Type` は戻り値の型を表す：

```yaoxiang
List: (T: Type) -> Type = { ... }
Map: (K: Type, V: Type) -> Type = { ... }
```

ジェネリクス関数では、型引数もシグネチャ内で宣言し、コンパイラは実引数から自動的に推論する：

```yaoxiang
map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R)) = ...
```

### 4.2 ジェネリクス型定義

```yaoxiang
// 基本ジェネリクス型
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
    push: (self: List(T), item: T) -> Void,   // self は約束名でありキーワードではない
    get: (self: List(T), index: Int) -> Option(T)
}
```

### 4.3 型推論

```yaoxiang
// コンパイラがジェネリクス引数を自動推論
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

// 制約を使用
clone: (T: Clone)(value: T) -> T = value.clone()
```

### 5.2 複数制約

```yaoxiang
// 複数制約構文
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
// Iterator trait（レコード型構文を使用）
Iterator: (T: Type) -> Type = {
    Item: T,                    // 関連型
    next: () -> Option(T),
    has_next: () -> Bool
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

### 6.2 ジェネリクス関連型（GAT）

```yaoxiang
// より複雑な関連型
Container: (T: Type) -> Type = {
    Item: T,
    IteratorType: Iterator(T),  // 関連型もジェネリクス
    iter: () -> IteratorType
}
```

---

## 第七章：コンパイル時ジェネリクス

### 7.1 コンパイル時定数引数

```
LiteralType   ::= Identifier ':' Int          // コンパイル時定数
CompileTimeFn ::= '(' Identifier ':' Int ')' '(' Identifier ')' '->' TypeExpr
```

**中心的設計**：`(n: Int)` ジェネリクス引数 + `(n: n)` 値引数により、コンパイル時定数と実行時値を区別する。

```yaoxiang
// コンパイル時階乗：引数はコンパイル時に既知のリテラルでなければならない
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
arr: StaticArray(Int, factorial(5))  // コンパイラがコンパイル時に factorial(5) = 120 を計算
```

### 7.2 コンパイル時定数配列

```yaoxiang
// 行列型で使用
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
// IsTrue は Assert への橋渡しと型精緻化（§8.3 参照）
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      // ⊤、プログラム続行
    false => Never,    // ⊥、発散/コンパイルエラー
}
Assert: (cond: Bool) -> Type = IsTrue(cond)
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

### 8.3 Assert 精緻化型と assert アサーション

`assert` と `Assert`
は同じ精緻化原語の両面であり、「述語の自由変数がコンパイル時に到達可能か」に基づいて dispatch
ディスパッチパイプラインが自動的に選択する。

**中心的シグネチャ**：`assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**dispatch ディスパッチ規則**：

| 判別基準                                       | モード        | 動作                                                               |
| ---------------------------------------------- | ------------- | ------------------------------------------------------------------ |
| すべての自由変数がコンパイル時に既知（ジェネリクス引数、コンパイル時定数） | CompileTime | 証明パイプラインに進む：true → Void に消去、false → コンパイルエラー（Never は居住不可） |
| 実行時の自由変数が存在（関数パラメータ、外部入力）       | Runtime     | 実行時 Bool チェックを挿入し、フロー感度仮定セット Γ に精緻化事实を導入 |

**フロー感度仮定セット Γ**：

コンパイラは各制御フロー点における既知命題のセットを維持する：

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP 伝播
mut x = x - 5       // Γ = {}  ← mut キルセット：古い仮定が無効になる
```

`mut` 変数への代入後、その変数に関わるすべての仮定が削除される（キルセット）。枝合流時に Γ
は各枝の共通部分を取る。

---

## 第九章：型の合併と交叉

### 9.1 型合併

```
TypeUnion     ::= TypeExpr '|' TypeExpr
```

### 9.2 型交叉

```
TypeIntersection ::= TypeExpr '&' TypeExpr
```

**構文**：型交叉 `A & B` は A と B の両方を満たす型を表す

```yaoxiang
// インターフェース合成 = 型交叉
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
Platform: Type = { X86_64: () -> Platform, AArch64: () -> Platform, RISC_V: () -> Platform, ARM: () -> Platform, X86: () -> Platform }

// P は現在コンパイル中のプラットフォームを表す事前定義ジェネリクス引数
sum: (P: X86_64)(arr: Array(Float)) -> Float = {
    return avx2_sum(arr.data, arr.length)
}

sum: (P: AArch64)(arr: Array(Float)) -> Float = {
    return neon_sum(arr.data, arr.length)
}
```

---

## 第十一章：型属性

YaoXiang には区別が必要な型属性は1種類だけある：Linear と 複製可能。これはコンパイラが自動推論する。

### 11.1 Move（既定の所有権移動）

すべての型は既定で Move セマンティクスに従う。代入、引数渡し、戻り値 = 所有権移動。

```yaoxiang
p: Point = Point(1.0, 2.0)
q = p           // Move、p はこれ以上読めない
```

### 11.2 Dup（浅コピー：ハンドルの複製、データの共有）

**Dup 属性は参照/トークン型に使用する**。Dup 型での代入 = 浅コピー——ハンドルの複製またはトークンの複製、基礎データは共有する。複数の保持者が同じデータブロックを指す。

| 型        | 属性   | 説明                                              |
| --------- | ------ | ------------------------------------------------- |
| `&T`      | Dup    | 零サイズの読み取りトークン、トークン複製 = 同一データを指す複数のビュー |
| `ref T`   | Dup    | Rc/Arc の複製 = 参照カウント+1、ヒープデータを共有 |
| `&mut T`  | Linear | 零サイズの書き込みトークン、排他的、使用不可複製     |
| その他全型 | Move   | 既定の所有権移動                                   |

**プリミティブ値型**（Int, Float, Bool,
Char）はコンパイラの組み込み的特殊処理である：代入時に自動値コピーされ、2つの値は完全に独立する。これはコンパイラのネイティブ動作であり、Dup
型属性には属さない。

```yaoxiang
// &T: Dup、自由にエイリアス可能
view: &Point = &p
view2 = view     // Dup：トークン複製、両方有効
print(view.x)    // 使用可能
print(view2.x)   // 使用可能

// &mut T: Linear、複製不可
mut_ref: &mut Point = &mut p
// r2 = mut_ref  // ❌ &mut T は Dup ではない、複製不可
```

### 11.3 Clone（明示的ディープコピー）と Dup の関係

**Clone** は明示的ディープコピーインターフェースである。すべての型は Clone を実装でき、`.clone()` メソッドを提供する。

```yaoxiang
// Clone インターフェース定義（標準ライブラリ）
Clone: Type = {
    clone: () -> Clone
}

// 使用
p: Point = Point(1.0, 2.0)
backup = p.clone()    // ディープコピー、p は引き続き使用可能
p2 = p.clone()        // 複数回クローン可能
```

**Dup と Clone の違い**：

|              | Dup                                 | Clone                     |
| ------------ | ----------------------------------- | ------------------------- |
| **セマンティクス** | 浅コピー：ハンドル/トークンを複製、基礎データは共有 | ディープコピー：完全な独立コピーを作成 |
| **呼び出し方式** | 暗黙的（代入/引数渡しが自動）       | 明示的（`.clone()`）        |
| **変更の影響** | 互いに影響（基礎データを共有）      | 互いに影響しない（独立コピー） |
| **適用型**   | `&T` トークン、`ref T`              | Clone インターフェースを実装する任意の型 |
| **コスト**   | ゼロオーバーヘッド（トークンは零サイズ型） | 型による                  |

**Dup は Clone を含意せず、Clone は Dup を含意しない**——これらは直交する2つの概念である：

```yaoxiang
// Dup 型：トークンを複製、基礎データを共有
view: &Point = &p
view2 = view        // Dup：トークン複製、両方とも同じ p を指す
print(view.x)       // 使用可能
print(view2.x)      // 使用可能、同一データを表示

// プリミティブ値型：コンパイラが自動値コピー（Dup ではない）
x: Int = 42
y = x               // 値コピー、x と y は完全に独立
print(x)            // 使用可能

// Clone：明示的ディープコピー、独立コピーを作成
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone：ディープコピー、p は引き続き使用可能
r = p               // Move：所有権移動、Point は Dup でもプリミティブ値型でもないため
```

**設計意図**：

- Dup はトークン/参照型に使用し、「同じデータを見る複数のビュー」という問題を解決する
- Clone は独立コピーが必要なシナリオに使用し、コストを明示的にする
- プリミティブ値型（Int/Float/Bool/Char）のコピーはコンパイラの組み込み動作であり、Dup
  には属さない
- ほとんどのカスタム型は既定で Move、ゼロコピーで高性能

## 第十二章：借用トークン型

### 12.1 中心的概念

`&T` と `&mut T` は**零サイズのコンパイル時トークン型**である。これらは「参照」ではなく、「アクセス権限の型レベル証明」である。

```
&T      →  零サイズ、ソースデータを凍結（書き込みトークンの取得を禁止）、
          凍結保証下で複数の読み取りが安全 → Dup（複製可能）
&mut T  →  零サイズ、排他的読み書き（他のトークンをすべて禁止）、
          排他的アクセス下では複製に意味がない → Linear（非 Dup）
```

**主要な特性**：

- トークンは**普通の型**であり、他のすべての型と同じスコープ規則に従う
- ライフタイム注記 `'a` は不要
- 専用の借用検査器は不要——型属性（Dup/Linear）が権限を自然に推論する
- コンパイル後になくなる、実行時オーバーヘッドゼロ

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
p.shift(1.0, 1.0)              // コンパイラが自動的に &mut Point トークンを作成
p.print()                       // OK、前回のトークンは shift 呼び出し終了時に解放済み

// 複数の &T トークンの共存——Dup 型により自由に複製可能
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 トークンのスコープと伝播

トークンは普通の型であるため、すべての普通の型の操作をサポート：

**トークンの戻り**——トークンは戻り値と一緒に伝播する：

```yaoxiang
// ✅ サブトークンと親トークンが一緒に戻る
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // トークンが呼び出し元に返される
print(px_ref)                    // OK、トークンはまだスコープ内
```

**構造体への格納**——構造体はトークンフィールドを持てる：

```yaoxiang
// ✅ 構造体がトークンをフィールドとして持つ
Window: Type = {
    target: Point,
    view: &Point,              // トークンフィールド——target への読み取り専用ビューを保持
}
```

**クロージャのキャプチャ**——クロージャは他の値と同様にトークンをキャプチャする：

```yaoxiang
// ✅ クロージャが &Float トークンをキャプチャ（Dup 型、自由にクロージャ内に複製可能）
filter_by_threshold: (items: List(Point), threshold: &Float) -> List(Point) = {
    items.filter(|p| p.x > threshold)
}
```

### 12.4 自動借用選択

呼び出し側コンパイラは以下の優先順位で自動選択：

```
1. 実引数が後続でも使用される場合 → トークンを作成優先（&T または &mut T、メソッドシグネチャによる）
2. 実引数が後続で使用されない場合 → Move
3. 優先マッチング順序：&T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // print のパラメータ型が &Point → コンパイラが &Point トークンを作成
p.shift(1.0, 1.0)  // shift のパラメータ型が &mut Point → コンパイラが &mut Point トークンを作成
p2 = p             // 後続で使用しない → Move
```

### 12.5 トークン競合検出

コンパイラはトークン値に対して**フロー感度活性分析**を行い、各トークンの状態（アクティブ/移動済み）を追跡：

```yaoxiang
// ❌ &mut と派生 &T が同時にアクティブになれない
bad_alias: (p: &mut Point) -> Void = {
    p.x = 10.0                   // ✅ 正常な WriteToken の使用
    print(p.y)
}

// ✅ トークンスコープ終了後に自動解放
good_seq: (p: &mut Point) -> Void = {
    {
        // 内部スコープ
        print(p.x)               // &mut Point を使用
    }
    // 内部スコープ終了
    p.x = 10.0                   // ✅ WriteToken はまだ使用可能
}

// ❌ 同一の実引数に対して &mut トークンと他のトークンを同時に作成不可
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p が同時に &mut と & トークンを派生させる
```

### 12.6 コンパイラの内部：ブランド機構

ユーザーはブランドに触れない。コンパイラは内部的に各トークンにコンパイル時一意の識別子を割り当て：

```
ユーザーが見るもの         コンパイラの内部表現
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N はコンパイル時一意の整数
&mut Point     →  WriteToken(Point, #M)   // #M はコンパイル時一意の整数
```

ブランドの用途：

- **偽造防止**：トークンは所有者のポインタからのみ取得でき、空中浮遊は不可能
- **関連追跡**：フィールドアクセスから派生した `&Float` は派生ブランド（`#N.field_x`）を携带し、コンパイラは親トークンまで追跡可能
- **競合検出**：同一来源の WriteToken と派生 ReadToken は同時にアクティブになれない

ブランドは単態化とインライン展開後に完全に消滅し、生成される機械語には存在しない。**ゼロ実行時オーバーヘッド。**

### 12.7 トークン Sum 型

```
&BorrowToken ::= &T          // ReadToken（ソースデータを冻结 → Dup 安全）
               | &mut T      // WriteToken（排他的読み書き → Linear）
```

### 12.8 借用トークン vs ref

|        | `&T` / `&mut T`                    | `ref`                   |
| ------ | ---------------------------------- | ----------------------- |
| 役割   | 一時参照/その場で変更             | 共有所有                |
| 範囲   | トークン値のスコープに従う         | スコープを越える        |
| コスト | ゼロオーバーヘッド（零サイズ型、コンパイル後に消滅）   | Rc または Arc（コンパイラ選択）   |
| 「エスケープ」   | 可能（トークンは戻り値/構造体/クロージャを通じて伝播可能） | もともとはエスケープ用   |
| タスク間 | 不可（トークンはタスク間伝播未実装）       | 可（コンパイラが自動的に Arc を選択）  |
| 環検出 | 関係なし                             | タスク内は無通知、クロスタスクで lint |

---

## 付録：型定義早見表

### A.1 型定義

```
// === レコード型（波括弧） ===

// レコード型
Point: Type = { x: Float, y: Float }

// バリアント付きレコード型（関数フィールドを使用）
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

### A.2 ジェネリクス構文

```
// ジェネリクス型
List: (T: Type) -> Type = { data: Array(T), length: Int }
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// ジェネリクス関数
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
// === Move（既定） ===
// すべての型は既定で Move。代入、引数渡し、戻り値 = 所有権移動

// === プリミティブ値型（コンパイラの組み込み） ===
Int, Float,     // 代入時に自動値コピー、2つの値は完全に独立
Bool, Char      // Dup ではなく、コンパイラのプリミティブに対する組み込み処理

// === Dup（浅コピー：ハンドルを複製、基礎データを共有） ===
&T              // 零サイズ読み取りトークン、トークン複製 = 同一データを指す複数のビュー
ref T           // Rc/Arc の複製 = 参照カウント+1、ヒープデータを共有

// === Linear ===
&mut T          // 零サイズ書き込みトークン、Linear（排他的、複製不可）

// === Clone（明示的ディープコピー） ===
value.clone()   // 独立コピーを作成、変更は原値に影響しない
```

### A.4 借用トークン早見表

```
// === 借用トークン ===
&T              // 零サイズコンパイル時読み取りトークン、ソースデータを冻结 → Dup（複製可能）
&mut T          // 零サイズコンパイル時書き込みトークン、排他的読み書き → Linear（複製不可）

// 呼び出し側の自動選択
// 1. 実引数が後続で使用される → トークンを作成
// 2. 実引数が後続で使用されない → Move
// 3. 優先マッチング：&T < &mut T < Move

// トークン伝播
// ✅ 戻り可能、構造体に格納可能、クロージャにキャプチャ可能
// ❌ タスク間不可（トークンはタスク間伝播未実装）
```

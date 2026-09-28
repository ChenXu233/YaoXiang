# type system 仕様

本ドキュメントは YaoXiang プログラミング言語の type
system 仕様を定義する。基本型、複合型、generics、trait を含む。

---

## 第零章：理論的基礎

### 0.1 Curry-Howard 同型

Curry-Howard 対応 (Curry-Howard correspondence) は YaoXiang の type
system の理論的基礎である。これはプログラミング言語の type
system と数理論理学の間の深い対応関係を明らかにする：

| 論理学                         | プログラミング言語                              |
| ------------------------------ | ----------------------------------------------- |
| 命題 \(P\)                     | 型 `Type`                                       |
| 証明 \(p: P\)                  | プログラム `x: T = ...`                         |
| 含意 \(P \rightarrow Q\)       | 関数型 `(P) -> Q`                               |
| 連言 \(P \wedge Q\)            | 積型 `{ a: P, b: Q }`                           |
| 選言 \(P \vee Q\)              | 和型 `{ a(P) \| b(Q) }`                         |
| 全称量化 \(\forall x:T. P(x)\) | generics `(T: Type) -> ...`                     |
| 真 \(\top\)                    | `Void`（Unit、デフォルト値あり）                |
| 偽 \(\bot\)                    | `Never`（ゼロコンストラクタ、居住可能な値なし） |
| 型宇宙 \(Type_n : Type_{n+1}\) | 宇宙階層（Russell のパラドックスを防ぐ）        |
| case 分析                      | 型レベル `match`                                |

> **注意**：型レベル `match` は場合分け (case
> analysis) であり、数学的帰納法ではない。帰納法には型レベル再帰関数 + コンパイラの停止性検査が必要。

### 0.2 型は命題、プログラムは証明

YaoXiang では、この対応関係は設計の第一級の原則である：

- **停止する型レベル計算は正しい構成的証明に対応する**。YaoXiang の type family（例：`Nat` 上の
  `Add`
  の case 分析 + 再帰呼び出し）は本質的に数学的帰納法の型レベルエンコーディングである——ただしコンパイラが停止性検査を行えることが前提である。
- **型検査は証明の検証である**。プログラムが型検査を通ると、論理的命題が構成的に証明されたことになる。

### 0.3 言語設計への影響

Curry-Howard 対応が YaoXiang で具体的に現れている：

1. **宇宙階層**（RFC-010）：`Type₀ : Type₁ : Type₂ …` は `Type: Type`
   から生じる論理的矛盾（Girard のパラドックス）を回避する
2. **type family**（RFC-011）：自然数 `Nat(Zero/Succ)`
   の型レベル case 分析 + 再帰呼び出しは Peano 公理に対応する——コンパイラが停止性検査を行うことが前提
3. **conditional type**（RFC-011）：`If: (C: Bool, T: Type, E: Type) -> Type`
   は論理における case 選言に対応する
4. **値依存型**（RFC-011）：`Array: (T: Type, N: Int) -> Type`
   は「各整数 N に対して型が存在する」という有界量化に対応する

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

> **設計説明**：RFC-010 は「すべてが代入である」統一モデル（`name: type = value`）を提案しているが、構文レベルでは型と値を区別する必要がある。コンパイラ実装では
> `Type` と `Expr` は独立した二つの AST 列挙型（`ast.rs:406` と `ast.rs:25`）であり、`TypeExpr`
> は BNF のプレースホルダーとして実装の `Type`
> 列挙型に対応し、「この位置は型を期待する」ことを示す。

---

## 第二章：基本型

### 2.1 primitive type

| 型       | 論理的対応   | 説明                                                                                | デフォルトサイズ |
| -------- | ------------ | ----------------------------------------------------------------------------------- | ---------------- |
| `Type`   | —            | meta type                                                                           | 0 バイト         |
| `Never`  | ⊥（偽/空型） | ゼロコンストラクタ、値なし。発散/panic 戻り型。`Never <: T` は任意の T に対し成立。 | 0 バイト         |
| `Void`   | ⊤（真/Unit） | デフォルト void 値あり、ゼロフィールド積型。`x: Void = <デフォルト>` は合法。       | 0 バイト         |
| `Bool`   | —            | ブール値：`true` / `false`                                                          | 1 バイト         |
| `Int`    | —            | 符号付き整数                                                                        | 8 バイト         |
| `Uint`   | —            | 符号なし整数                                                                        | 8 バイト         |
| `Float`  | —            | 浮動小数点数                                                                        | 8 バイト         |
| `String` | —            | UTF-8 文字列                                                                        | 可変             |
| `Char`   | —            | Unicode 文字                                                                        | 4 バイト         |
| `Bytes`  | —            | 生バイト                                                                            | 可変             |

ビット幅付き整数：`Int8`, `Int16`, `Int32`, `Int64`, `Int128`。ビット幅付き浮動小数点：`Float32`,
`Float64`

### 2.2 Never と Void：⊥ と ⊤

`Never` と `Void` は type system の論理的プリミティブであり、それぞれ偽（⊥）と真（⊤）に対応する。

**Never（⊥、偽/空型）** — 譲歩できない三つの性質：

1. **ゼロコンストラクタ**：`Never` 型の値を生成する literal や式はない。`x: Never = ...`
   の右辺は書けない。
2. **爆発原理**：`Never <: T` は任意の型 `T` に対し成立。`assert(false)` は `Never`
   を返し、その後のコードは型検査を通る（実際には実行されないが）。
3. **発散マーカー**：`f: (...) -> Never` は `f`
   が決して戻らないことを示す。コンパイラはこれに基づき dead code 分析と `match` 分岐合流を行う。

`Never` は組み込み型名（`Int`/`Bool` と同じ登録パス）であり、キーワードではない。

**Void（⊤、真/Unit）** — ちょうど一つの居住者（デフォルト void 値）を持つ。`Void`
はゼロフィールド積型の単位元である。`x: Void = <デフォルト>`
は合法。ブロックの値は**末尾式**によって与えられる（空ブロック `{}` は `Void`）、詳細は
[RFC-010a](../../design/rfc/accepted/010a-tail-expression-and-return.md) を参照。

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
// 简单记录类型
Point: Type = { x: Float, y: Float }

// 空记录类型
Empty: Type = {}

// 带泛型的记录类型
Pair: (T: Type) -> Type = { first: T, second: T }

// 实现接口的记录类型
Point: Type = {
    x: Float,
    y: Float,
    Drawable,
    Serializable
}
```

**規則**：

- record type は中括弧 `{}` で定義する
- フィールド名の後にコロンと型を続ける
- インターフェース名は型本体に書くと、そのインターフェースの実装を示す

> **名前空間の所属**：`Type.name` プレフィックス（例：`Point.draw`）は関数が `Point`
> の名前空間に属することを示す。これは暗黙のバインディングを引き起こさない。`p.draw()` のような `.`
> 呼び出し構文を有効にするには、明示的なバインディングが必要：
> `Point.draw = draw[0]`。詳細は RFC-004 と RFC-010 を参照。

#### 3.1.1 フィールドのデフォルト値

型フィールドにはデフォルト値を指定でき、構築時にオプションで提供できる：

```yaoxiang
// 有默认值的字段 - 构造时可选
Point: Type = {
    x: Float = 0,
    y: Float = 0
}

// 使用
Point()           // -> Point(x=0, y=0)
Point(x=1)       // -> Point(x=1, y=0)
Point(x=1, y=2) // -> Point(x=1, y=2)

// 无默认值的字段 - 构造时必填
Point2: Type = {
    x: Float,
    y: Float
}

// 使用
Point2(x=1, y=2) // 正确
Point2()          // 错误
```

**規則**：

- `field: Type = expression` -> デフォルト値あり、構築時オプション
- `field: Type` -> デフォルト値なし、構築時必須

#### 3.1.2 組み込みバインディング

型定義本体内で直接メソッドをバインドできる：

```yaoxiang
// 方式1：引用外部函数绑定
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]    // 绑定到位置0
}
// 调用：p1.distance(p2) -> distance(p1, p2)

// 方式2：匿名函数 + 位置绑定
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance: ((a: Point, b: Point) -> Float)[0] = ((a, b) => {
        dx = a.x - b.x
        dy = a.y - b.y
        return (dx * dx + dy * dy).sqrt()
    })
}
// 语法：((params) => body)[position]
// 调用：p1.distance(p2) -> distance(p1, p2)
```

### 3.2 interface type

```
InterfaceType ::= '{' FnField (',' FnField)* ','?
FnField       ::= Identifier ':' FnType
FnType        ::= '(' ParamTypes? ')' '->' TypeExpr
```

**構文**：インターフェースはフィールドがすべて関数型である record type である

```yaoxiang
// 接口定义
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

// 空接口
EmptyInterface: Type = {}
```

**インターフェースの実装**：型は定義の最後にインターフェース名を列挙することでインターフェースを実装する

```yaoxiang
// 实现接口的类型
Point: Type = {
    x: Float,
    y: Float,
    Drawable,        // 实现 Drawable 接口
    Serializable     // 实现 Serializable 接口
}
```

**インターフェースへの直接代入**：具体型は interface
type の変数に直接代入できる（構造的サブタイピング）

```yaoxiang
// 直接赋值（编译期可确定具体类型 -> 零开销调用）
d: Drawable = Circle(1)
d.draw(screen)        // 编译后：直接调用 circle_draw，无 vtable

// 函数返回值（编译期无法确定 -> vtable 调用）
d: Drawable = get_shape()
d.draw(screen)        // 通过 vtable 查找方法

// 接口作为函数参数
process: (d: Drawable) -> Void = d.draw(screen)
```

**コンパイル時最適化戦略**：

| シーン           | 推論結果       | 呼び出し方式       |
| ---------------- | -------------- | ------------------ |
| 直接代入具体类型 | 具体类型可确定 | 直接调用（零开销） |
| 函数返回值       | 未知           | vtable             |
| 异构集合         | 多个类型       | vtable             |

**コヒーレンスとオーファンルール（適用なし、収束説明）**：YaoXiang のインターフェースは構造的型（インターフェース = フィールドがすべて関数型である record）であり、名目的な trait ではない——クレート/モジュールを横断した「誰が誰のために実装するか」という帰属問題は存在せず、Rust 式のオーファンルールとコヒーレンス検査は適用対象がない（裁決記録は RFC-011
§2.1）。構造的世界の対応する保障は**重複実装の拒否**である：同一メソッドシグネチャの型上重複定義はコンパイルエラー（RFC-011a
§3、上書き禁止；オーバーロードは合法）。

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

## 第四章：generics

### 4.1 generics パラメータの構文

generics パラメータは関数型の一部であり、通常のパラメータと同様に `()` 構文を使用する：

```
GenericType     ::= Identifier '(' TypeArgList ')'
TypeArgList     ::= TypeExpr (',' TypeExpr)* ','?
TypeBound       ::= Identifier
                 |  Identifier '+' Identifier ('+' Identifier)*
```

generics 型定義では、`(T: Type)` は型コンストラクタのパラメータシグネチャであり、`-> Type`
は戻り型を示す：

```yaoxiang
List: (T: Type) -> Type = { ... }
Map: (K: Type, V: Type) -> Type = { ... }
```

#### 4.1.1 コンテナ型

コンテナ型は generics 型コンストラクタであり、組み込みプリミティブではない——ユーザー定義 generics と同じ扱いを受け、統一された generics インスタンス化パスを経る。長さ情報の帰属は三つのコンテナ概念の根本的な違いである：

| 型            | 長さ       | セマンティクス                       | 基盤                                        |
| ------------- | ---------- | ------------------------------------ | ------------------------------------------- |
| `Array(T, N)` | 型         | 定長配列（const generics N）         | コアプリミティブ（スタック/インライン優先） |
| `Vec(T)`      | runtime 値 | ランタイム長さの生バッファ、拡張可能 | コアプリミティブ（ヒープ上の連続バッファ）  |
| `List(T)`     | runtime 値 | 標準ライブラリ型（拡張可能リスト）   | ライブラリ：`{ data: Vec(T), length: Int }` |
| `Dict(K, V)`  | runtime 値 | キー値マッピング                     | `HeapValue::Dict`                           |

> `List(T)` は**標準ライブラリ型であり、コンパイラプリミティブではない**：YaoXiang 自身によって
> `std.list`
> で定義され、ユーザー定義 generics 記録と同じ扱いを受ける。拡張可能セマンティクスのすべての戦略（いつ拡張するか、どれだけ拡張するか、共有可能か）はライブラリ内にあり、コンパイラは関与しない。
> `Vec(T)` はそれが依存する最小限の基礎プリミティブである。
>
> Set(T) は削除済み：literal なし、runtime 表現なし、std.set なし。需要が発生した際に Dict パターンで完成させる。

主要な規則：

- **literal の振り分けはコンテキストが決める**：`[...]` 生リテラルと `List(T)`
  注釈は拡張可能リストに振り分けられる；`Array(T, N)`
  注釈がリテラルに直接作用する場合は定長配列に振り分けられる。振り分け検証：要素数 ==
  N、要素型が T と互換、不一致は compile-time
  E1002；N がシンボリック定数（const パラメータ）の場合、個数検証は精化型フェーズまで延期される。
- **暗黙の List→Array 変換を禁止**：定長性は型層で保証される——push は `List(A)`
  レシーバのみ受け付ける。
- **パフォーマンス階層**：底から上に向かってパフォーマンスは低下し、柔軟性は向上：`Array` > `Vec` >
  `List`。
- **インデックス失敗の契約**（runtime エラーは過渡状態、目標状態は compile-time 精化でカバー、値依存型、§8.4 参照）：
  - インデックス範囲外（負のインデックスを含む）→ `E6003`
  - Dict キー欠落 → `E6008`
- **membership `in` 述語**：`Bool`
  を返しエラーにならず、右オペランドは List/Array/Dict(キー)/Tuple/String/Range をカバー。一等ホーア述語であり、精化型の compile-time 証明可能な命題の基底である。`

generics 関数では、型パラメータも同様にシグネチャで宣言され、コンパイラは実引数から自動的に推論する：

```yaoxiang
map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R)) = ...
```

### 4.2 generics 型定義

```yaoxiang
// 基础泛型类型
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
    push: (self: List(T), item: T) -> Void,   // self 只是约定名，不是关键字
    get: (self: List(T), index: Int) -> Option(T)
}
```

**`?` 伝播と `Try` インターフェース**：`expr?` はインターフェース駆動のエラー伝播である——受信者型は
`Try` インターフェース（4 メンバー `is_failure` / `success` / `residual` /
`from_error`）を実装している必要があり、失敗時は `from_error(residual(t))`
の結果を現在の関数から早期に返し、成功時は式の値が `success(t)` となる。外側関数の戻り型も同様に
`Try` を実装し、その失敗残余型は受信者と一致する必要がある（`E1081` / `E1082` /
`E1083`）。`Result(T, E)` と `Option(T)` は `std.result` / `std.option` によって `Try`
実装を提供する（`Option` の失敗残余型は `Void`）；ユーザー定義の和型は型本体内に `Try(自身, T, E)`
と書くだけで `?` に接続できる。 `?`
の lowering は組み込みとユーザー型を区別しない——同一インターフェース、同一パス。

### 4.3 generics 構築呼び出しと type inference

generics 型定義のフィールドリストは**コンストラクタを自動生成する**：各フィールドがコンストラクタパラメータに対応し、フィールド名がパラメータ名となる；デフォルト値を持つフィールドは構築時に省略可能で、デフォルト値のないフィールドは必須。関数型フィールド（メソッド）はコンストラクタパラメータを生成しない。

```yaoxiang
// 类型定义
Container: (T: Type) -> Type = {
    value: T,        // 无默认值 → 构造参数必填
    extra: T,
}
// 自动展开的完整形式（编译器内部视图，不要求用户手写）：
// Container: (T: Type) -> (value: T, extra: T) -> Type = {
//     value: T = value,
//     extra: T = extra,
// }

// 调用：调用自动生成的构造函数
c  = Container(42, 43)            // 构造参数按字段顺序填；T 从元素自动解包 = Int
c2 = Container("a", "b")          // T = String
c3 = Container(Int)(42, 43)       // 显式类型参数 + 位置式构造参数
c4 = Container(Int)(extra=43, value=42)  // 字段名式，顺序任意
c5 = Container(Int)()             // 空构造：字段取默认值/零值（数据事后赋值）

// 字段默认值 → 构造参数可省略
Point: (T: Type) -> Type = { x: T = 0, y: T = 0 }
p  = Point(1.5, 2.5)              // T = Float，x←1.5, y←2.5
p2 = Point(Int)()                 // x=0, y=0
```

**呼び出し規則**（単一括弧、宣言パラメータとビット単位で照合、左から右）：

1. 実引数はビット単位で型宣言パラメータとの照合を試みる：`Type`
   位置は型実引数を受け付け、compile-time 値パラメータ位置（例：`Int`）は compile-time 定数を受け付ける。
2. compile-time 値パラメータ位置の照合が成功する場合（部分照合）、型構築として処理する：すべてのパラメータ位置を順に検査し、エラー時は宣言順序に従って**最初に不一致/欠落したパラメータを報告**する。
3. 実引数が宣言パラメータにまったく対応しない（すべて値、compile-time 値パラメータ位置の照合なし）の場合、コンストラクタパラメータとして処理する：位置式はフィールド順に従い、型パラメータは要素型から自動展開される。

```yaoxiang
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),
    data: Array(Array(T, Cols), Rows),
}

m: Matrix(Int, 3, 4)              // 类型位置：一层类型构造
m2 = Matrix(Int, 3, 4)(data=[[1,2,3,4],[5,6,7,8],[9,10,11,12]])  // 两层：类型 + 构造参数
m3 = Matrix(Int, 3, 4)()          // 空构造（RFC-011 §9.3 模式，数据事后赋值）

Matrix(42)    // ❌ 位0: T←42 不匹配（42 不是类型）；位1: Rows←42 匹配；
              //    位2: Cols 缺失 → 先报第一个错误：T 期望 Type，找到 42
Container(42) // ❌ 缺构造参数 extra
Container(42, 43, 44)  // ❌ 构造参数超数
```

**type
inference**：generics 型コンストラクタの型パラメータはコンストラクタパラメータ要素から自動展開される（`Container(42, 43)`
→ T=Int）；generics 関数の型パラメータは実引数型から自動展開される（`map(numbers, f)` → T=Int,
R=String、§4.1 参照）。展開できない場合は明示的に指定する必要がある。

---

## 第五章：type constraint

### 5.1 single constraint

```
ConstrainedType ::= '(' Identifier ':' TypeBound ')' TypeExpr
```

```yaoxiang
// 接口类型定义（作为约束）
Clone: Type = {
    clone: () -> Clone
}

// 使用约束
clone: (T: Clone)(value: T) -> T = value.clone()
```

### 5.2 multiple constraint

> **制約の解決ソース（RFC-011b）**：演算子制約名（`Add` / `Subtract` / `Multiply` / `Divide` /
> `Modulo` / `Equal` / `Index`）の解決 = インターフェース実装登録テーブルを照会する—— `T: Add`
> ≜ 登録済み `Add(T, T, T)` インスタンス化；`Equal`
> には別の構造的推論（全フィールドが比較可能な record は自動比較可能）。`Zero` / `One` /
> `PartialOrd` などの名前はまだ定義ソースがなく、宙ぶらりんの制約名である。

```yaoxiang
// 多重约束语法
combine: (T: Clone + Add)(a: T, b: T) -> T = {
    a.clone() + b
}

// 泛型容器的排序
sort: (T: Clone + PartialOrd)(list: List(T)) -> List(T) = {
    result = list.clone()
    quicksort(&mut result)
    return result
}
```

### 5.3 関数型制約

```yaoxiang
// 高阶函数约束
call_twice: (T: Type, F: () -> T)(f: F) -> (T, T) = (f(), f())

compose: (A: Type, B: Type, C: Type, F: (A) -> B, G: (B) -> C)(a: A, f: F, g: G) -> C = g(f(a))
```

---

## 第六章：associated type

### 6.1 associated type 定義

```
AssociatedType ::= Identifier ':' TypeExpr
```

```yaoxiang
// Iterator trait（使用记录类型语法）
Iterator: (T: Type) -> Type = {
    Item: T,                    // 关联类型
    next: () -> Option(T),
    has_next: () -> Bool
}

// 使用关联类型
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

### 6.2 generics associated type (GAT)

```yaoxiang
// 更复杂的关联类型
Container: (T: Type) -> Type = {
    Item: T,
    IteratorType: Iterator(T),  // 关联类型也是泛型的
    iter: () -> IteratorType
}
```

---

## 第七章：compile-time generics

### 7.1 compile-time 値パラメータ

```
LiteralType   ::= Identifier ':' Int          // compile-time 常量（候选）
```

> 判定基準は**型位置で参照されるか**であり、「具体的な型で注釈されているか」ではない：`add: (a: Int, b: Int) -> Int = a + b`
> の中で `a`/`b` は runtime 値パラメータである（どちらもどの型位置にも現れない）。

**用語**：`Type` 以外の具体型（例：
`Int`）で注釈された generics パラメータを**compile-time 値パラメータ候補**と呼び、compile-time 値パラメータになるかどうかは値が型位置で参照されるか（値依存）による。**`const`
キーワードは不要** （実装内部ではかつて「const
generics」という用語を使用していたが、ドキュメントでは統一して「compile-time 値パラメータ」を使用する）。

**判定規則（二段階）**：

1. **形態粗選別**：パラメータが `Type` 以外の具体型（`Int`/`Bool`/`Float`）で注釈されている → 候補。
2. **用途精選別**：候補名が**型位置**（型本体のフィールド型、内部 `Fn` パラメータ型、`Assert`
   述語、`Array(T, N)`
   型構築実引数位置）に現れる → 真の compile-time 値パラメータ；そうでない場合は**runtime 値パラメータ**。

| 書き方                                                     | 判定                        | 理由                            |
| ---------------------------------------------------------- | --------------------------- | ------------------------------- |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b runtime 値パラメータ    | 値位置にのみ現れる              |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N compile-time 値パラメータ | N が型構築実引数位置に現れる    |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N compile-time 値パラメータ | N が内部パラメータ k の型となる |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N 落空→runtime 値パラメータ | N が型本体で参照されていない    |

**中核設計**：`(N: Int)` compile-time 値パラメータ + `(k: N)`
値パラメータで、compile-time 定数と runtime 値を区別する。落空候補（形態は候補だが用途が未一致）は runtime 値パラメータに退化する——関数レベルと型コンストラクタパスの両方がこれに従って処理される。

```yaoxiang
// 编译期值参数：N 在类型位置（Array 长度槽）被引用
Measure: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // N 出现在类型构造实参位 → 编译期值参数
    length: N
}

// 使用方式：factorial(5) 在类型位置求值（编译期），结果 120 嵌入类型
arr: Measure(Int, factorial(5))  // 编译器在编译期计算 factorial(5) = 120

// 值依赖：N 作为内层参数 k 的类型
// N 是编译期值参数（出现在 (k: N) 的类型位）；
// k 是运行时值参数，其类型为字面量类型 N（单值类型）。
factorial: (N: Int) -> (k: N) -> Int = {
    match k {
        0 => 1,
        _ => k * factorial(k - 1)
    }
}
```

### 7.2 compile-time 定数配列

```yaoxiang
// 矩阵类型使用
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows)
}

// 编译期维度验证
identity_matrix: (T: Add + Zero + One, N: Int)(size: N) -> Matrix(T, N, N) = {
    // ...
}
```

---

## 第八章：conditional type

### 8.1 If conditional type

```
IfType        ::= 'If' '(' BoolExpr ',' TypeExpr ',' TypeExpr ')'
```

```yaoxiang
// 类型级 If
If: (C: Bool, T: Type, E: Type) -> Type = match C {
    True => T,
    False => E
}

// 示例：编译期分支
NonEmpty: (T: Type) -> Type = If(T != Void, T, Never)
// IsTrue 桥接与 Assert 精化类型（详见 §8.3）
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      // ⊤，程序继续
    false => Never,    // ⊥，发散/编译错误
}
Assert: (cond: Bool) -> Type = IsTrue(cond)
```

### 8.2 type family

```yaoxiang
// 编译期类型转换
AsString: (T: Type) -> Type = match T {
    Int => String,
    Float => String,
    Bool => String,
    _ => String
}
```

### 8.3 Assert 精化型と assert アサーション

`assert` と `Assert`
は同じ精化プリミティブの二面である——dispatch 分派パイプラインが「述語の自由変数が compile-time にアクセス可能か」に従って自動的に選択する。

**中核シグネチャ**：`assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**dispatch 分派規則**：

| 判定基準                                                                         | モード      | 動作                                                                                          |
| -------------------------------------------------------------------------------- | ----------- | --------------------------------------------------------------------------------------------- |
| すべての自由変数が compile-time に既知（generics パラメータ、compile-time 定数） | CompileTime | 証明パイプラインに入る：true → void に消去、false → compile-time エラー（Never は居住不可能） |
| runtime 自由変数が存在する（関数パラメータ、外部入力）                           | Runtime     | runtime Bool 検査を挿入し、フロー感度仮説集合 Γ に精化事実を注入する                          |

**フロー感度仮説集合 Γ**：

コンパイラは各制御フロー点の既知の命題集合を保持する：

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP 传播
mut x = x - 5       // Γ = {}  ← mut kill set：旧假设失效
```

`mut` 変数の代入後、その変数に関するすべての仮説が削除される（kill
set）。分岐合流時、Γ は各分岐の交差を取る。

### 8.4 Terminates：停止測度述語

`Terminates` は**組み込み述語**であり、`Int`、`Never`
と同じくコアプリミティブ（組み込み名、キーワードではない）に属する。これは**測度**をある計算にバインドし、その計算が停止することを宣言し、停止の証人を与える。

**形態**：二つのアリティ、同じ述語：

| 形態                    | アンカー                 | 用途                                                                                                               |
| ----------------------- | ------------------------ | ------------------------------------------------------------------------------------------------------------------ |
| `Terminates(m)`         | あるバインディングの名前 | デフォルト形態——自己再帰関数、ループ                                                                               |
| `Terminates(FnType, m)` | 明示的な関数型           | 測度の帰属を明示的に指定する必要がある場合（測度が別所で定義されている、同じ測度が複数の計算にサービスを提供する） |

```yaoxiang
// 测度：普通函数，可单测、可复用，不参与运行时
gcd_measure: (a: Int, b: Int) -> Int = { b }

// 二元形态：测度定义在别处，显式指明归属
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

// 一元形态：锚点即绑定名，测度是作用域内的表达式
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n {
        i = i + 1
    }
    return acc
}
```

**セマンティクス**：`Terminates(m)`
が精化するのは、それが型位置に注釈されたその計算の値型である。義務はその計算にかかる——関数は各再帰呼び出し点、ループは後退辺——いずれも「次の状態の測度が現在の状態の測度より厳密に小さい」であり、その点のパスガード下で判定される。

呼び出し点の義務は `m(callee_args) < m(caller_args)`；後退辺の義務は
`m(次のラウンド) < m(本ラウンド)`。両者は同じ形式である。

> **なぜループをカバーするか**：ループは無名構造であり、通常は指示することができない。バインディング名がすなわち名前である——`acc: Terminates(n - i) = while ...`
> の中の `acc` がアンカーを提供し、ループはそれ故に指示可能となる。これが `Terminates`
> の一項形態がループに作用する理由である。

**測度**：戻り型を制限しない（自然数を強制しない）；その上の「厳密減少」はその型で利用可能な適格順序によって与えられる。測度が well-founded であるか（例：`Int`
を返すときに `>= 0`
であるか）は**独立した義務**であり、減少義務と同様に compile-time 証明パイプラインで判定される。

**トリガー**：停止検査は**精化型**によってトリガーされる——型が精化されるとすぐに検証モードに入る。精化されていない通常の型（例：裸の
`while` ループ、精化シグネチャのない関数）は検証モードに入らず、停止義務を生成しない。

**自動探索優先**：コンパイラはまず測度を自動探索し（線形ランク関数、述語違反カウント、有界増減、乗算スケーリングの 4 テンプレート）、探索できない場合にのみ明示的に
`Terminates` を指定する必要がある。

**runtime 表現**：純粋に compile-time のエンティティであり、witness とともに消去され、runtime バイナリには入らない。

> 完全な設計は
> [RFC-027 §6.9](../../design/rfc/accepted/027-compile-time-evaluation-types.md)（セマンティクス）と
> [RFC-027a](../../design/rfc/review/027a-termination-explicit-measure.md)（実装メカニズム）を参照。

---

## 第九章：型ユニオンと交差

### 9.1 型ユニオン

```
TypeUnion     ::= TypeExpr '|' TypeExpr
```

### 9.2 型交差

```
TypeIntersection ::= TypeExpr '&' TypeExpr
```

**構文**：型交差 `A & B` は A と B の両方を満たす型を表す

```yaoxiang
// 接口组合 = 类型交集
DrawableSerializable: Type = Drawable & Serializable

// 使用交集类型
process: (T: Drawable & Serializable)(item: T, screen: Surface) -> String = {
    item.draw(screen)
    return item.serialize()
}
```

---

## 第十章：関数オーバーロードと特殊化

### 10.1 関数オーバーロード

```yaoxiang
// 基本特化：使用函数重载（编译器自动选择）
sum: (arr: Array(Int)) -> Int = {
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Array(Float)) -> Float = {
    return simd_sum_float(arr.data, arr.length)
}

// 通用实现
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
// 平台类型枚举（标准库定义）
Platform: Type = { X86_64: () -> Platform, AArch64: () -> Platform, RISC_V: () -> Platform, ARM: () -> Platform, X86: () -> Platform }

// P 是预定义泛型参数名，代表当前编译平台
sum: (P: X86_64)(arr: Array(Float)) -> Float = {
    return avx2_sum(arr.data, arr.length)
}

sum: (P: AArch64)(arr: Array(Float)) -> Float = {
    return neon_sum(arr.data, arr.length)
}
```

---

## 第十一章：型属性

YaoXiang には区別すべき型属性が一つだけある：linear
vs コピー可能。コンパイラによって自動推論される。

### 11.1 Move（デフォルトの ownership 移転）

すべての型はデフォルトで Move セマンティクスに従う。代入、引数渡し、戻り値 = ownership 移転。

```yaoxiang
p: Point = Point(1.0, 2.0)
q = p           // Move，p 不可再读
```

### 11.2 Dup（浅いコピー：ハンドルをコピー、データを共有）

**Dup 属性は参照/トークン型に使用される**。Dup 型の代入 = 浅いコピー——ハンドル/トークンをコピーし、基礎データを共有。複数の保持者が同じデータブロックを指す。

| 型             | 属性   | 説明                                                                        |
| -------------- | ------ | --------------------------------------------------------------------------- |
| `&T`           | Dup    | ゼロサイズの読み取りトークン、トークンのコピー = 同じデータを指す複数の視点 |
| `ref T`        | Dup    | Rc/Arc のコピー = 参照カウント+1、ヒープデータを共有                        |
| `&mut T`       | Linear | ゼロサイズの書き込みトークン、排他的、コピー不可                            |
| 他のすべての型 | Move   | デフォルトの ownership 移転                                                 |

**プリミティブ値型**（Int, Float, Bool,
Char）はコンパイラが組み込む特別な処理である：代入時に自動的に値コピーされ、二つの値は完全に独立している。これはコンパイラのネイティブ動作であり、Dup 型属性には属さない。

```yaoxiang
// &T: Dup，可自由别名
view: &Point = &p
view2 = view     // Dup：复制令牌，两者均有效
print(view.x)    // 可用
print(view2.x)   // 可用

// &mut T: Linear，不可复制
mut_ref: &mut Point = &mut p
// r2 = mut_ref  // ❌ &mut T 不是 Dup，不能复制
```

### 11.3 Clone（明示的な深いコピー）と Dup の関係

**Clone** は明示的な深いコピーインターフェースである。すべての型は Clone を実装でき、`.clone()`
メソッドを提供する。

```yaoxiang
// Clone 接口定义（标准库）
Clone: Type = {
    clone: () -> Clone
}

// 使用
p: Point = Point(1.0, 2.0)
backup = p.clone()    // 深复制，p 仍然可用
p2 = p.clone()        // 可多次克隆
```

**Dup と Clone の違い**：

|                    | Dup                                                     | Clone                                    |
| ------------------ | ------------------------------------------------------- | ---------------------------------------- |
| **セマンティクス** | 浅いコピー：ハンドル/トークンをコピー、基礎データを共有 | 深いコピー：完全で独立したコピー作成     |
| **呼び出し方式**   | 暗黙的（代入/引数渡しで自動）                           | 明示的（`.clone()`）                     |
| **変更影響**       | 互いに影響（基礎データを共有）                          | 互いに独立（独立したコピー）             |
| **適用型**         | `&T` トークン、`ref T`                                  | Clone インターフェースを実装する任意の型 |
| **コスト**         | ゼロオーバーヘッド（トークンはゼロサイズ型）            | 型による                                 |

**Dup は Clone を含意せず、Clone も Dup を含意しない**——これらは二つの直交する概念である：

```yaoxiang
// Dup 类型：复制令牌，底层数据共享
view: &Point = &p
view2 = view        // Dup：复制令牌，两者指向同一个 p
print(view.x)       // 可用
print(view2.x)      // 可用，看到的是同一份数据

// 原语值类型：编译器自动值复制（不是 Dup）
x: Int = 42
y = x               // 值复制，x 和 y 完全独立
print(x)            // 可用

// Clone：显式深拷贝，创建独立副本
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone：深复制，p 仍然可用
r = p               // Move：所有权转移，因为 Point 不是 Dup 也不是原语值类型
```

**設計意図**：

- Dup はトークン/参照型に使用され、「同じデータの複数の視点」問題を解決する
- Clone は独立したコピーが必要なシナリオに使用され、明示的な呼び出しでコストを可視化する
- プリミティブ値型（Int/Float/Bool/Char）のコピーはコンパイラの組み込み動作であり、Dup には属さない
- ほとんどのカスタム型はデフォルトで Move であり、ゼロコピーで高性能

## 第十二章：借用トークン型

### 12.1 中核概念

`&T` と `&mut T`
は**ゼロサイズの compile-time トークン型**である。これらは「参照」ではなく、「アクセス権の型レベル証明」である。

```
&T      →  ゼロサイズ、ソースデータを凍結（WriteToken のこの間の取得を禁止）、
          凍結保証下で複数の読み取り専用が安全 → Dup（コピー可能）
&mut T  →  ゼロサイズ、排他的読み書き（他のすべてのトークンを禁止）、
          排他的アクセス下ではコピーは無意味 → Linear（Dup ではない）
```

**主要な特性**：

- トークンは**通常の型**であり、他のすべての型と同じスコープ規則に従う
- ライフタイム注釈 `'a` は不要
- 専用の借用チェッカーは不要——型属性（Dup/Linear）が自然に権限を推論する
- コンパイル後完全に消滅し、runtime オーバーヘッドはゼロ

### 12.2 基本的な使用

```yaoxiang
// 方法端：声明参数类型，决定需要的权限
Point.print: (self: &Point) -> Void = {
    print(self.x)               // &Point 令牌授予读权限
    print(self.y)
}

Point.shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
    self.x = self.x + dx        // &mut Point 令牌授予写权限
    self.y = self.y + dy
}

// 调用端：编译器自动选择借用或 Move
p = Point(1.0, 2.0)
p.print()                       // 编译器自动创建 &Point 令牌
p.shift(1.0, 1.0)               // 编译器自动创建 &mut Point 令牌
p.print()                       // OK，上一个令牌已随 shift 调用结束而释放

// 多个 &T 令牌共存——Dup 类型允许自由复制
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 トークンのスコープと伝播

トークンは通常の型であるため、通常の型のすべての操作をサポートする：

**トークンの返却**——トークンは戻り値と一緒に伝播する：

```yaoxiang
// ✅ 子令牌和父令牌一起返回
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // 令牌返回给调用者
print(px_ref)                    // OK，令牌仍在作用域
```

**存结构体**——構造体はトークンフィールドを保持できる：

```yaoxiang
// ✅ 结构体携带令牌作为字段
Window: Type = {
    target: Point,
    view: &Point,              // 令牌字段——持有对 target 的只读视图
}
```

**闭包不捕获，上下文在创建点固化**——クロージャは自分のパラメータのみを取り込み、外部データが必要な場合はカリー化によって作成時点で値をクロージャに固定する：

```yaoxiang
// ✅ 上下文经柯里化固化：threshold 是参数，gt_point(threshold) 在创建点把值固化进闭包
gt_point: (t: Float) -> (p: Point) -> Bool = (p) => p.x > t
filter_by_threshold: (items: List(Point), threshold: Float) -> List(Point) = {
    items.filter(gt_point(threshold))
}
```

> 注：クロージャ（関数値）がエスケープした後、その定義箇所のスコープはすでに死んでいる可能性があるため、外側の変数を暗黙的にキャプチャしてはならない；しかし呼び出し点（作成点）のスコープは必ず生存しており、その時点でコンテキストが値としてクロージャに固定されることは安全である。

### 12.4 自動借用選択

呼び出し端で、コンパイラは以下の優先順位に従って自動的に選択する：

```
1. 如果实参后续还有使用 → 优先创建令牌（&T 或 &mut T，根据方法签名）
2. 如果实参后续不再使用 → Move
3. 优先匹配顺序：&T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // print 的参数类型为 &Point → 编译器创建 &Point 令牌
p.shift(1.0, 1.0)  // shift 的参数类型为 &mut Point → 编译器创建 &mut Point 令牌
p2 = p             // 后续不再使用 → Move
```

**メソッド受信者はシグネチャセマンティクスに従う**（RFC-011a 受信者スペル規約と同款）：受信者が `&T`
→ 読み取り専用借用トークン；`&mut T` → 可変借用トークン；値渡し →
Move（受信者を消費）。呼び出し点で生成された借用トークンは呼び出し終了とともに解放される（transient、§12.5 区間セマンティクス）；インターフェースの借用受信者はインターフェース作成者が明示的に
`&Self` を宣言し、impl シグネチャは `Self ↦ impl 型`
置換後にインターフェースと完全に一致する必要がある（RFC-011a §3）。

### 12.5 トークン競合検出

トークン競合検出は**借用ホーア命題**（RFC-009a）であり、独立したフロー感度分析ではない。コンパイラは借用命題（`borrow_conflict`/`use_after_move`/`use_after_drop`/`mut_violation`）を自動生成し、証明パイプラインに送って検証する；トークンの生存性は区間
`[created_at, last_use]`（RFC-009a §逆 BFS 活性分析参照）：

```yaoxiang
// ❌ &mut 和派生的 &T 不能同时活跃
bad_alias: (p: &mut Point) -> Void = {
    p.x = 10.0                   // ✅ 正常使用 WriteToken
    print(p.y)
}

// ✅ 令牌作用域结束后自动释放
good_seq: (p: &mut Point) -> Void = {
    {
        // 内部作用域
        print(p.x)               // 使用 &mut Point
    }
    // 内部作用域结束
    p.x = 10.0                   // ✅ WriteToken 仍可用
}

// ❌ 同一实参不能同时创建 &mut 令牌和其他令牌
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p 同时派生 &mut 和 & 令牌
```

### 12.6 コンパイラ内部：ブランド機構

ユーザーがブランドに触れることは決してない。コンパイラは内部で各トークンに compile-time 一意の識別子を割り当てる：

```
用户看到的           编译器内部表示
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N 是编译期唯一整数
&mut Point     →  WriteToken(Point, #M)   // #M 是编译期唯一整数
```

ブランドの用途：

- **偽造防止**：トークンは所有者カプセルからのみ取得でき、凭空に構築できない
- **関連追跡**：フィールドアクセスから派生した `&Float`
  は派生ブランド（`#N.field_x`）を持ち、コンパイラは親トークンまで追跡できる
- **競合検出**：同源の WriteToken と派生 ReadToken は同時にアクティブにできない

ブランドは単態化とインライン化後に完全に消滅し、生成されたマシンコードには存在しない。**runtime オーバーヘッドはゼロ。**

### 12.7 トークン Sum 型

```
&BorrowToken ::= &T          // ReadToken（ソースデータを凍結 → Dup 安全）
               | &mut T      // WriteToken（排他的読み書き → Linear）
```

### 12.8 借用トークン vs ref

|        | `&T` / `&mut T`                                      | `ref`                                 |
| ------ | ---------------------------------------------------- | ------------------------------------- |
| 做什么 | 看一眼/原地改                                        | 共享持有                              |
| 范围   | 随令牌值的作用域                                     | 跨作用域                              |
| 成本   | ゼロオーバーヘッド（ゼロサイズ型、compile 後に消滅） | Rc または Arc（コンパイラが選択）     |
| 逃逸   | 可（トークンは戻り値/構造体で伝播）                  | 本来就是用来逃逸的                    |
| 跨任务 | 不可（トークンはタスク間渡し未実装）                 | 可（コンパイラが自動的に Arc を選択） |
| 环检测 | 不涉及                                               | タスク内静默，跨タスク lint           |

> 注（未定義）：ref 作成後に内容を読み取る方法（解参照/メソッド/自動）はまだ仕様で定義されておらず、実装現状では
> `*a` は E1052 を報告する。定義後に本節を補完する。

---

## 付録：型定義クイックリファレンス

### A.1 型定義

```
// === 记录类型（花括号） ===

// 记录类型
Point: Type = { x: Float, y: Float }

// 带变体的记录类型（使用函数字段）
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// === 接口类型（花括号，字段全为函数） ===

// 接口定义
Serializable: Type = { serialize: () -> String }

// 实现接口的类型
Point: Type = {
    x: Float,
    y: Float,
    Serializable    // 实现 Serializable 接口
}

// === 函数类型 ===

Adder: Type = (Int, Int) -> Int

// === 终止测度（内置谓词，见 §8.4） ===

// 一元：锚点即绑定名（自递归函数、循环）
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n { i = i + 1 }
    return acc
}

// 二元：显式指明测度归属（测度定义在别处）
gcd_measure: (a: Int, b: Int) -> Int = { b }
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}
```

### A.2 generics 構文

```
// 泛型类型
List: (T: Type) -> Type = { data: Array(T), length: Int }
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// 泛型函数
map: (T: Type, R: Type)(list: List(T), f: (T) -> R) -> List(R) = { ... }

// 类型约束
clone: (T: Clone)(value: T) -> T = value.clone()
combine: (T: Clone + Add)(a: T, b: T) -> T = body

// 关联类型
Iterator: (T: Type) -> Type = { Item: T, next: () -> Option(T) }

// 编译期泛型：N 在类型位置 (k: N) 被引用 → 编译期值参数
factorial: (N: Int)(k: N) -> Int = { ... }
Measure: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }

// 条件类型
If: (C: Bool, T: Type, E: Type) -> Type = match C { True => T, False => E }

// 函数特化
sum: (arr: Array(Int)) -> Int = { ... }
sum: (arr: Array(Float)) -> Float = { ... }
```

### A.3 型属性クイックリファレンス

```
// === Move（默认） ===
// 所有类型默认 Move。赋值、传参、返回 = 所有权转移

// === 原语值类型（编译器内置） ===
Int, Float,     // 赋值时自动值复制，两个值完全独立
Bool, Char      // 不是 Dup，是编译器对原语的内置处理

// === Dup（浅拷贝：复制句柄，共享底层数据） ===
&T              // 零大小读取令牌，复制令牌 = 多个视角指向同一数据
ref T           // Rc/Arc 复制 = 引用计数+1，共享堆数据

// === Linear ===
&mut T          // 零大小写入令牌，Linear（独占，不可复制）

// === Clone（显式深复制） ===
value.clone()   // 创建独立副本，修改不影响原值
```

### A.4 借用トークンクイックリファレンス

```
// === 借用令牌 ===
&T              // 零大小编译期读令牌，冻结源数据 → Dup（可复制）
&mut T          // 零大小编译期写令牌，独占读写 → Linear（不可复制）

// 调用端自动选择
// 1. 实参后续还有使用 → 创建令牌
// 2. 实参后续不再使用 → Move
// 3. 优先匹配：&T < &mut T < Move

// 令牌传播
// ✅ 可返回、可存结构体、可被闭包捕获
// ❌ 不可跨任务（令牌未实现跨任务传递）
```

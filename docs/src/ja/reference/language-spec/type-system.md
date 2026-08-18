# 型システム仕様

本ドキュメントは YaoXiang プログラミング言語の型システム仕様を定義する。基本型、複合型、ジェネリクス、trait を含む。

---

## 第零章：理論的基礎

### 0.1 Curry-Howard 対応

Curry-Howard 対応（Curry-Howard
correspondence）は、YaoXiang 型システムの理論的基礎である。これはプログラミング言語の型システムと数理論理学の間の深層対応を明らかにする：

| 論理学                         | プログラミング言語                                    |
| ------------------------------ | ----------------------------------------------------- |
| 命題 \(P\)                     | 型 `Type`                                             |
| 証明 \(p: P\)                  | プログラム `x: T = ...`                               |
| 含意 \(P \rightarrow Q\)       | 関数型 `(P) -> Q`                                     |
| 連言 \(P \wedge Q\)            | 積型 `{ a: P, b: Q }`                                 |
| 選言 \(P \vee Q\)              | 和型 `{ a(P) \| b(Q) }`                               |
| 全称量化 \(\forall x:T. P(x)\) | ジェネリクス `(T: Type) -> ...`                       |
| 真 \(\top\)                    | `Void`（Unit、デフォルト値あり）                      |
| 偽 \(\bot\)                    | `Never`（ゼロコンストラクタ、いかなる値も居住不可能） |
| 型宇宙 \(Type_n : Type_{n+1}\) | 宇宙階層（Russell のパラドックス防止）                |
| case 分析                      | 型レベル `match`                                      |

> **注意**：型レベル `match` は分類討論（case
> analysis）であり、数学的帰納法ではない。帰納法には型レベル再帰関数とコンパイラの停止性検査が必要。

### 0.2 型は命題、プログラムは証明

YaoXiang では、この対応関係が設計の第一級原則として位置づけられる：

- **停止する型レベル計算は正しい構成的証明に対応する**。YaoXiang の型族（例：`Nat(Zero/Succ)` 上の
  `Add`
  の case 分析 + 再帰呼び出し）は本質的に数学的帰納法の型レベルエンコーディングである——ただし、コンパイラが停止性検査を行えることが前提である。
- **型検査は証明の検証である**。あるプログラムが型検査を通過するということは、論理的命題が構成的に証明されたことを意味する。

### 0.3 言語設計への影響

Curry-Howard 対応の YaoXiang における具体的な体現：

1. **宇宙階層**（RFC-010）：`Type₀ : Type₁ : Type₂ …` は `Type: Type`
   によって生じる論理的パラドックス（Girard のパラドックス）を回避する
2. **型族**（RFC-011）：自然数 `Nat(Zero/Succ)`
   の型レベル case 分析 + 再帰呼び出しは Peano 公理に対応する——ただし、コンパイラが停止性検査を行うことが前提である
3. **条件型**（RFC-011）：`If: (C: Bool, T: Type, E: Type) -> Type` は論理学の case 選言に対応する
4. **値依存型**（RFC-011）：`Vec: (n: Int) -> Type`
   は「各整数 n に対して型が存在する」という有界量化に対応する

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

> **設計説明**：RFC-010 は「すべてが代入である」という統一モデル（`name: type = value`）を提案しているが、構文レベルでは型と値を区別する必要がある。コンパイラ実装では
> `Type` と `Expr` は二つの独立した AST enum（`ast.rs:406` および `ast.rs:25`）であり、`TypeExpr`
> は BNF のプレースホルダとして実装の `Type` enum に対応し、「この位置に型が期待される」ことを表す。

---

## 第二章：基本型

### 2.1 プリミティブ型

| 型       | 論理対応     | 説明                                                                                              | デフォルトサイズ |
| -------- | ------------ | ------------------------------------------------------------------------------------------------- | ---------------- |
| `Type`   | —            | メタ型                                                                                            | 0 バイト         |
| `Never`  | ⊥（偽/空型） | ゼロコンストラクタ、いかなる値もなし。発散/panic の戻り型。`Never <: T` は任意の T に対して成立。 | 0 バイト         |
| `Void`   | ⊤（真/Unit） | デフォルトの void 値を持ち、ゼロフィールドの積型。`x: Void = <デフォルト>` が合法。               | 0 バイト         |
| `Bool`   | —            | ブール値：`true` / `false`                                                                        | 1 バイト         |
| `Int`    | —            | 符号付き整数                                                                                      | 8 バイト         |
| `Uint`   | —            | 符号なし整数                                                                                      | 8 バイト         |
| `Float`  | —            | 浮動小数点数                                                                                      | 8 バイト         |
| `String` | —            | UTF-8 文字列                                                                                      | 可変             |
| `Char`   | —            | Unicode 文字                                                                                      | 4 バイト         |
| `Bytes`  | —            | 生バイト                                                                                          | 可変             |

ビット幅指定の整数：`Int8`, `Int16`, `Int32`, `Int64`, `Int128` ビット幅指定の浮動小数点：`Float32`,
`Float64`

### 2.2 Never と Void：⊥ と ⊤

`Never` と `Void` は型システムの論理プリミティブであり、それぞれ偽（⊥）と真（⊤）に対応する。

**Never（⊥、偽/空型）** — 譲歩できない三つの性質：

1. **ゼロコンストラクタ**：リテラルや式を問わず、いかなる値も `Never`
   型を生成できない。`x: Never = ...` の右辺に書けるものはない。
2. **爆発原理**：`Never <: T` は任意の型 `T` に対して成立する。`assert(false)` は `Never`
   を返し、以降のコードは型検査を通過する（実際には実行されないが）。
3. **発散マーカー**：`f: (...) -> Never` は `f`
   が絶対に return しないことを示す。コンパイラはこれに基づき dead code 解析と `match`
   分岐合流を行う。

`Never` は組み込み型名（`Int`/`Bool` と同じ登録パス）であり、キーワードではない。

**Void（⊤、真/Unit）** — ちょうど一つの居住者（デフォルト void 値）を有する。`Void`
はゼロフィールド積型の単位元である。`x: Void = <デフォルト>` が合法であり、関数がデフォルトで
`return` を持たない場合は `Void` を返す。

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

// 空のレコード型
Empty: Type = {}

// ジェネリックを含むレコード型
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
- フィールド名の後にコロンと型を続ける
- 型本体内のインターフェース名は当該インターフェースの実装を表す

> **名前空間帰属**：`Type.name` 接頭辞（例：`Point.draw`）は関数が `Point`
> の名前空間に属することを示すのみであり、暗黙のバインディングを一切発動しない。`p.draw()` のような
> `.`
> 呼び出し構文を有効化するには、明示的なバインディングが必要である：`Point.draw = draw[0]`。詳細は RFC-004 および RFC-010 を参照。

#### 3.1.1 フィールドデフォルト値

型のフィールドにはデフォルト値を指定でき、構築時には省略可能となる：

```yaoxiang
// デフォルト値を持つフィールド - 構築時に省略可能
Point: Type = {
    x: Float = 0,
    y: Float = 0
}

// 使用
Point()           // -> Point(x=0, y=0)
Point(x=1)       // -> Point(x=1, y=0)
Point(x=1, y=2) // -> Point(x=1, y=2)

// デフォルト値を持たないフィールド - 構築時に必須
Point2: Type = {
    x: Float,
    y: Float
}

// 使用
Point2(x=1, y=2) // 正しい
Point2()          // エラー
```

**ルール**：

- `field: Type = expression` -> デフォルト値あり、構築時に省略可能
- `field: Type` -> デフォルト値なし、構築時に必須

#### 3.1.2 組み込みバインディング

型定義本体内で直接メソッドをバインドできる：

```yaoxiang
// 方式1：外部関数の参照によるバインディング
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]    // 位置 0 にバインド
}
// 呼び出し：p1.distance(p2) -> distance(p1, p2)

// 方式2：無名関数 + 位置バインディング
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

**構文**：インターフェースはフィールドがすべて関数型であるレコード型である

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

**インターフェース実装**：型は定義の末尾にインターフェース名を列挙することでインターフェースを実装する

```yaoxiang
// インターフェースを実装する型
Point: Type = {
    x: Float,
    y: Float,
    Drawable,        // Drawable インターフェースを実装
    Serializable     // Serializable インターフェースを実装
}
```

**インターフェースへの直接代入**：具体型はインターフェース型変数に直接代入可能（構造的サブタイピング）

```yaoxiang
// 直接代入（コンパイル時に具体型を決定可能 → ゼロオーバーヘッド呼び出し）
d: Drawable = Circle(1)
d.draw(screen)        // コンパイル後：circle_draw を直接呼び出し、vtable なし

// 関数の戻り値（コンパイル時に決定不可能 → vtable 呼び出し）
d: Drawable = get_shape()
d.draw(screen)        // vtable 経由でメソッドを検索

// 関数の引数としてのインターフェース
process: (d: Drawable) -> Void = d.draw(screen)
```

**コンパイル時最適化戦略**：

| シナリオ         | 推論結果         | 呼び出し方式                       |
| ---------------- | ---------------- | ---------------------------------- |
| 具体型の直接代入 | 具体型を決定可能 | 直接呼び出し（ゼロオーバーヘッド） |
| 関数の戻り値     | 不明             | vtable                             |
| 異種集合         | 複数型           | vtable                             |

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

### 4.1 ジェネリック引数構文

ジェネリック引数は関数型の一部であり、通常の引数と統一的に `()` 構文を用いる：

```
GenericType     ::= Identifier '(' TypeArgList ')'
TypeArgList     ::= TypeExpr (',' TypeExpr)* ','?
TypeBound       ::= Identifier
                 |  Identifier '+' Identifier ('+' Identifier)*
```

ジェネリック型定義では、`(T: Type)` は型コンストラクタの引数シグネチャであり、`-> Type`
は戻り型を表す：

```yaoxiang
List: (T: Type) -> Type = { ... }
Map: (K: Type, V: Type) -> Type = { ... }
```

ジェネリック関数では、型引数も同様にシグネチャ内で宣言され、コンパイラは実引数から自動的に推論する：

```yaoxiang
map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R)) = ...
```

### 4.2 ジェネリック型定義

```yaoxiang
// 基本的なジェネリック型
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
    push: (self: List(T), item: T) -> Void,   // self は単なる慣習名でありキーワードではない
    get: (self: List(T), index: Int) -> Option(T)
}
```

### 4.3 ジェネリック構築呼び出しと型推論

ジェネリック型定義のフィールドリストは**自動的に構築関数を生成する**：各フィールドは構築引数に対応し、フィールド名が引数名となる。デフォルト値を持つフィールドは構築時に省略可能で、デフォルト値を持たないフィールドは必須である。関数型フィールド（メソッド）は構築引数を生成しない。

```yaoxiang
// 型定義
Container: (T: Type) -> Type = {
    value: T,        // デフォルト値なし → 構築引数は必須
    extra: T,
}
// 自動的に展開された完全形式（コンパイラの内部ビューであり、ユーザーが手書きする必要はない）：
// Container: (T: Type) -> (value: T, extra: T) -> Type = {
//     value: T = value,
//     extra: T = extra,
// }

// 呼び出し：自動生成された構築関数の呼び出し
c  = Container(42, 43)            // 構築引数はフィールド順に埋める；T は要素から自動的に Int へ
c2 = Container("a", "b")          // T = String
c3 = Container(Int)(42, 43)       // 明示的な型引数 + 位置式構築引数
c4 = Container(Int)(extra=43, value=42)  // フィールド名式、順序は任意
c5 = Container(Int)()             // 空構築：フィールドはデフォルト値/ゼロ値を取る（データは後で代入）

// フィールドデフォルト値 → 構築引数は省略可能
Point: (T: Type) -> Type = { x: T = 0, y: T = 0 }
p  = Point(1.5, 2.5)              // T = Float、x←1.5, y←2.5
p2 = Point(Int)()                 // x=0, y=0
```

**呼び出しルール**（単一括弧、宣言引数をビット単位でマッチング、左から右へ）：

1. 実引数はビット単位で型宣言引数とのマッチングを試行する：`Type`
   位置は型実引数を受け入れ、コンパイル時値引数位置（例：`Int`）はコンパイル時定数を受け入れる。
2. コンパイル時値引数位置でのマッチングが成功した場合（部分マッチング）、型構築として処理する：全引数位置をビット単位でチェックし、エラー時は宣言順に従って**最初の一致しない/欠落している引数を最初に報告する**。
3. 実引数が宣言引数に完全に対応しない場合（すべて値で、コンパイル時値引数位置とマッチする箇所がない）、構築引数として処理する：位置式はフィールド順に埋め、型引数は要素型から自動的にアンラップする。

```yaoxiang
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),
    data: Array(Array(T, Cols), Rows),
}

m: Matrix(Int, 3, 4)              // 型位置：一階型構築
m2 = Matrix(Int, 3, 4)(data=[[1,2,3,4],[5,6,7,8],[9,10,11,12]])  // 二階：型 + 構築引数
m3 = Matrix(Int, 3, 4)()          // 空構築（RFC-011 §9.3 パターン、データは後で代入）

Matrix(42)    // ❌ 位置0: T←42 が一致しない（42 は型ではない）；位置1: Rows←42 が一致；
              //    位置2: Cols が欠落 → 最初のエラーを報告：T は Type を期待するが 42 が見つかった
Container(42) // ❌ 構築引数 extra が欠落
Container(42, 43, 44)  // ❌ 構築引数が超過
```

**型推論**：ジェネリック型コンストラクタの型引数は構築引数の要素から自動的にアンラップされる（`Container(42, 43)`
→ T=Int）；ジェネリック関数の型引数は実引数の型から自動的にアンラップされる（`map(numbers, f)` →
T=Int, R=String、§4.1 参照）。アンラップできない場合は明示的に指定する必要がある。

---

## 第五章：型制約

### 5.1 単一制約

```
ConstrainedType ::= '(' Identifier ':' TypeBound ')' TypeExpr
```

```yaoxiang
// インターフェース型定義（制約として）
Clone: Type = {
    clone: () -> Clone
}

// 制約の使用
clone: (T: Clone)(value: T) -> T = value.clone()
```

### 5.2 複数制約

```yaoxiang
// 複数制約の構文
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
// Iterator trait（レコード型構文を使用）
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

### 7.1 コンパイル時定数引数

```
LiteralType   ::= Identifier ':' Int          // コンパイル時定数
```

**用語**：`Type`
以外の具体型（例：`Int`）でアノテーションされたジェネリック引数は**コンパイル時値引数**（compile-time
value parameter）と呼ばれ、デフォルトでコンパイル時に決定され、**`const`
キーワードは不要である**（実装内部ではかつて「const ジェネリクス」と呼称されていたが、ドキュメントでは統一して「コンパイル時値引数」を使用する）。

**中核設計**：`(n: Int)` コンパイル時値引数と `(n: n)`
値引数を用いて、コンパイル時定数とランタイム値を区別する。

```yaoxiang
// コンパイル時階乗：引数はコンパイル時に既知のリテラルでなければならない
factorial: (n: Int) -> (n: n) -> Int = {
    match n {
        0 => 1,
        _ => n * factorial(n - 1)
    }
}

// コンパイル時定数配列
StaticArray: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // コンパイル時に既知のサイズを持つ配列
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
// IsTrue ブリッジと Assert 精緻化型（詳細は §8.3）
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      // ⊤、プログラムは継続
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
は同じ精緻化プリミティブの二面であり、dispatch ディスパッチパイプラインが「述語の自由変数がコンパイル時に到達可能か」に基づいて自動的に選択する。

**中核シグネチャ**：`assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**dispatch ディスパッチルール**：

| 判定基準                                                                   | モード      | 動作                                                                                           |
| -------------------------------------------------------------------------- | ----------- | ---------------------------------------------------------------------------------------------- |
| すべての自由変数がコンパイル時に既知（ジェネリック引数、コンパイル時定数） | CompileTime | 証明パイプラインへ進む：true → Void として消去、false → コンパイルエラー（Never は居住不可能） |
| ランタイム自由変数が存在する（関数の引数、外部入力）                       | Runtime     | ランタイム Bool 検査を挿入し、フロー敏感的仮定集合 Γ に精緻化事実を注入する                    |

**フロー敏感的仮定集合 Γ**：

コンパイラは各制御流点の既知命題集合を保守する：

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP 伝播
mut x = x - 5       // Γ = {}  ← mut kill set：古い仮定が無効化される
```

`mut` 変数の代入後、その変数に関するすべての仮定が削除される（kill
set）。分岐合流時には Γ は各分岐の交差を取る。

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

**構文**：型交差 `A & B` は A と B を同時に満たす型を表す

```yaoxiang
// インターフェース組み合わせ = 型交差
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
// プラットフォーム型 enum（標準ライブラリで定義）
Platform: Type = { X86_64: () -> Platform, AArch64: () -> Platform, RISC_V: () -> Platform, ARM: () -> Platform, X86: () -> Platform }

// P は事前定義されたジェネリック引数名で、現在のコンパイルプラットフォームを表す
sum: (P: X86_64)(arr: Array(Float)) -> Float = {
    return avx2_sum(arr.data, arr.length)
}

sum: (P: AArch64)(arr: Array(Float)) -> Float = {
    return neon_sum(arr.data, arr.length)
}
```

---

## 第十一章：型属性

YaoXiang には区別すべき型属性が一つだけある：線形 vs コピー可能。コンパイラが自動的に推論する。

### 11.1 Move（デフォルトの所有権移転）

すべての型はデフォルトで Move セマンティクスに従う。代入、引数渡し、戻り値 = 所有権移転。

```yaoxiang
p: Point = Point(1.0, 2.0)
q = p           // Move、p は以降読み取れない
```

### 11.2 Dup（浅いコピー：ハンドルコピー、共有データ）

**Dup 属性は参照/トークン型に使用される**。Dup 型の代入 = 浅いコピー——ハンドル/トークンをコピーし、基盤データを共有する。複数の保持者が同じデータを指す。

| 型               | 属性   | 説明                                                                      |
| ---------------- | ------ | ------------------------------------------------------------------------- |
| `&T`             | Dup    | ゼロサイズ読み取りトークン、トークンコピー = 複数の視点が同じデータを指す |
| `ref T`          | Dup    | Rc/Arc コピー = 参照カウント +1、ヒープデータを共有                       |
| `&mut T`         | Linear | ゼロサイズ書き込みトークン、排他的、コピー不可                            |
| その他すべての型 | Move   | デフォルトの所有権移転                                                    |

**プリミティブ値型**（Int, Float, Bool,
Char）はコンパイラによる組み込み特殊処理：代入時に自動的に値がコピーされ、二つの値は完全に独立する。これはコンパイラのネイティブ動作であり、Dup 型属性には属さない。

```yaoxiang
// &T: Dup、自由な別名付けが可能
view: &Point = &p
view2 = view     // Dup：トークンをコピーし、両者ともに有効
print(view.x)    // 使用可能
print(view2.x)   // 使用可能

// &mut T: Linear、コピー不可
mut_ref: &mut Point = &mut p
// r2 = mut_ref  // ❌ &mut T は Dup ではない、コピー不可
```

### 11.3 Clone（明示的な深いコピー）と Dup の関係

**Clone** は明示的な深いコピーインターフェースである。すべての型は Clone を実装可能で、`.clone()`
メソッドを提供する。

```yaoxiang
// Clone インターフェース定義（標準ライブラリ）
Clone: Type = {
    clone: () -> Clone
}

// 使用
p: Point = Point(1.0, 2.0)
backup = p.clone()    // 深いコピー、p は引き続き使用可能
p2 = p.clone()        // 複数回クローン可能
```

**Dup と Clone の違い**：

|                    | Dup                                                     | Clone                                    |
| ------------------ | ------------------------------------------------------- | ---------------------------------------- |
| **セマンティクス** | 浅いコピー：ハンドル/トークンをコピー、基盤データを共有 | 深いコピー：完全で独立したコピーを作成   |
| **呼び出し方式**   | 暗黙的（代入/引数渡しで自動）                           | 明示的（`.clone()`）                     |
| **変更の影響**     | 相互に影響する（基盤データを共有）                      | 相互に影響しない（独立コピー）           |
| **適用型**         | `&T` トークン、`ref T`                                  | Clone インターフェースを実装する任意の型 |
| **コスト**         | ゼロオーバーヘッド（トークンはゼロサイズ型）            | 型に依存                                 |

**Dup は Clone を蕴含せず、Clone も Dup を蕴含しない**——これらは二つの直交する概念である：

```yaoxiang
// Dup 型：トークンコピー、基盤データを共有
view: &Point = &p
view2 = view        // Dup：トークンコピー、両者が同じ p を指す
print(view.x)       // 使用可能
print(view2.x)      // 使用可能、同じデータが見える

// プリミティブ値型：コンパイラによる自動値コピー（Dup ではない）
x: Int = 42
y = x               // 値コピー、x と y は完全に独立
print(x)            // 使用可能

// Clone：明示的な深いコピー、独立コピー作成
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone：深いコピー、p は引き続き使用可能
r = p               // Move：所有権移転、Point は Dup でもプリミティブ値型でもないため
```

**設計意図**：

- Dup はトークン/参照型に使用され、「複数の視点で同じデータを見る」問題を解決する
- Clone は独立したコピーが必要なシナリオに使用され、明示的な呼び出しによりコストを可視化する
- プリミティブ値型（Int/Float/Bool/Char）のコピーはコンパイラの組み込み動作であり、Dup には属さない
- ほとんどのカスタム型はデフォルトで Move であり、ゼロコピーで高性能

## 第十二章：借用トークン型

### 12.1 中核概念

`&T` と `&mut T`
は**ゼロサイズのコンパイル時トークン型**である。これらは「参照」ではなく、「アクセス権の型レベル証明」である。

```
&T      →  ゼロサイズ、ソースデータを凍結（この期間の WriteToken 取得を禁止）、
          凍結保証下で複数の読み取り専用が安全 → Dup（コピー可能）
&mut T  →  ゼロサイズ、排他的読み書き（他のあらゆるトークンを禁止）、
          排他的アクセス下ではコピーは無意味 → Linear（Dup ではない）
```

**主要特性**：

- トークンは**通常の型**であり、他のすべての型と同じスコープルールに従う
- ライフタイム注釈 `'a` は不要
- 専用の借用チェッカーは不要——型属性（Dup/Linear）が自然に権限を推論する
- コンパイル後に完全に消え、ゼロランタイムオーバーヘッド

### 12.2 基本使用

```yaoxiang
// メソッド側：引数型を宣言し、必要な権限を決定
Point.print: (self: &Point) -> Void = {
    print(self.x)               // &Point トークンが読み取り権限を付与
    print(self.y)
}

Point.shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
    self.x = self.x + dx        // &mut Point トークンが書き込み権限を付与
    self.y = self.y + dy
}

// 呼び出し側：コンパイラが借用または Move を自動選択
p = Point(1.0, 2.0)
p.print()                       // コンパイラが自動的に &Point トークンを作成
p.shift(1.0, 1.0)               // コンパイラが自動的に &mut Point トークンを作成
p.print()                       // OK、前のトークンは shift 呼び出し終了と共に解放済み

// 複数の &T トークンが共存——Dup 型は自由なコピーを許可
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 トークンのスコープと伝播

トークンは通常の型であるため、通常の型がサポートするすべての操作をサポートする：

**トークンの返却**——トークンは戻り値と共に伝播する：

```yaoxiang
// ✅ 子トークンと親トークンを一緒に返す
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // トークンは呼び出し元に返される
print(px_ref)                    // OK、トークンはまだスコープ内
```

**構造体に格納**——構造体はトークンフィールドを保持可能：

```yaoxiang
// ✅ 構造体がトークンをフィールドとして保持
Window: Type = {
    target: Point,
    view: &Point,              // トークンフィールド——target への読み取り専用ビューを保持
}
```

**クロージャキャプチャ**——クロージャは任意の値をキャプチャするようにトークンをキャプチャする：

```yaoxiang
// ✅ クロージャが &Float トークンをキャプチャ（Dup 型なので自由にコピー可能）
filter_by_threshold: (items: List(Point), threshold: &Float) -> List(Point) = {
    items.filter(|p| p.x > threshold)
}
```

### 12.4 自動借用選択

呼び出し側コンパイラは以下の優先度で自動選択する：

```
1. 実引数が後に使用される場合 → トークンの作成を優先（&T または &mut T、メソッドシグネチャに応じて）
2. 実引数が後に使用されない場合 → Move
3. 優先マッチング順序：&T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // print の引数型は &Point → コンパイラが &Point トークンを作成
p.shift(1.0, 1.0)  // shift の引数型は &mut Point → コンパイラが &mut Point トークンを作成
p2 = p             // 以降使用されない → Move
```

### 12.5 トークン衝突検出

コンパイラはトークン値に対して**フロー敏感的活性解析**を行い、各トークンの状態（アクティブ/移動済み）を追跡する：

```yaoxiang
// ❌ &mut と派生した &T は同時にアクティブにできない
bad_alias: (p: &mut Point) -> Void = {
    p.x = 10.0                   // ✅ WriteToken を正常に使用
    print(p.y)
}

// ✅ トークンのスコープ終了後に自動解放
good_seq: (p: &mut Point) -> Void = {
    {
        // 内部スコープ
        print(p.x)               // &mut Point を使用
    }
    // 内部スコープ終了
    p.x = 10.0                   // ✅ WriteToken は引き続き使用可能
}

// ❌ 同じ実引数から &mut トークンと他のトークンを同時に作成することはできない
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p から &mut トークンと & トークンを同時に派生
```

### 12.6 コンパイラ内部：ブランド機構

ユーザーがブランドに触れることは決してない。コンパイラは内部で各トークンにコンパイル時一意識別子を割り当てる：

```
ユーザーから見える           コンパイラ内部表現
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N はコンパイル時一意整数
&mut Point     →  WriteToken(Point, #M)   // #M はコンパイル時一意整数
```

ブランドの用途：

- **偽造防止**：トークンは所有者カプセルからのみ取得可能で、凭空には構築できない
- **関連追跡**：フィールドアクセス由来の `&Float`
  は派生ブランド（`#N.field_x`）を運び、コンパイラは親トークンまで追跡可能
- **衝突検出**：同じソースの WriteToken と派生 ReadToken は同時にアクティブにできない

ブランドは単態化とインライン化後に完全に消え、生成された機械語には存在しない。**ゼロランタイムオーバーヘッド**。

### 12.7 トークン Sum 型

```
&BorrowToken ::= &T          // ReadToken（ソースデータ凍結 → Dup 安全）
               | &mut T      // WriteToken（排他的読み書き → Linear）
```

### 12.8 借用トークン vs ref

|              | `&T` / `&mut T`                                      | `ref`                                 |
| ------------ | ---------------------------------------------------- | ------------------------------------- |
| 機能         | 一目見る/インプレース変更                            | 共有保持                              |
| 範囲         | トークン値のスコープに従う                           | スコープを超える                      |
| コスト       | ゼロオーバーヘッド（ゼロサイズ型、コンパイル後消失） | Rc または Arc（コンパイラが選択）     |
| エスケープ   | 可（トークンは戻り値/構造体/クロージャと共に伝播）   | 本来エスケープ用                      |
| タスク間     | 不可（トークンはタスク間転送を実装していない）       | 可（コンパイラが Arc を自動選択）     |
| サイクル検出 | 関与しない                                           | タスク内では静かに、タスク間では lint |

---

## 付録：型定義クイックリファレンス

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

### A.3 型属性クイックリファレンス

```
// === Move（デフォルト） ===
// すべての型はデフォルトで Move。代入、引数渡し、戻り値 = 所有権移転

// === プリミティブ値型（コンパイラ組み込み） ===
Int, Float,     // 代入時に自動的に値コピー、二つの値は完全に独立
Bool, Char      // Dup ではなく、コンパイラによるプリミティブの組み込み処理

// === Dup（浅いコピー：ハンドルコピー、基盤データを共有） ===
&T              // ゼロサイズ読み取りトークン、トークンコピー = 複数の視点が同じデータを指す
ref T           // Rc/Arc コピー = 参照カウント +1、ヒープデータを共有

// === Linear ===
&mut T          // ゼロサイズ書き込みトークン、Linear（排他的、コピー不可）

// === Clone（明示的な深いコピー） ===
value.clone()   // 独立したコピーを作成、変更は元の値に影響しない
```

### A.4 借用トークンクイックリファレンス

```
// === 借用トークン ===
&T              // ゼロサイズコンパイル時読み取りトークン、ソースデータ凍結 → Dup（コピー可能）
&mut T          // ゼロサイズコンパイル時書き込みトークン、排他的読み書き → Linear（コピー不可）

// 呼び出し端の自動選択
// 1. 実引数が後に使用される → トークン作成
// 2. 実引数が後に使用されない → Move
// 3. 優先マッチング：&T < &mut T < Move

// トークン伝播
// ✅ 返却可能、構造体に格納可能、クロージャにキャプチャ可能
// ❌ タスク間は不可（トークンはタスク間転送を実装していない）
```

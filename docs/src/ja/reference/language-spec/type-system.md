# 型システム仕様

本文書は YaoXiang プログラミング言語の型システム仕様を定義する。基本型、複合型、ジェネリクス、trait を含む。

---

## 第零章：理論的基礎

### 0.1 Curry-Howard 同型

Curry-Howard 同型（Curry-Howard
correspondence）は YaoXiang 型システムの理論的基礎である。これはプログラミング言語の型システムと数理論理学の間の深層対応の関係を明らかにする：

| 論理学                         | プログラミング言語                   |
| ------------------------------ | ------------------------------------ |
| 命題 \(P\)                     | 型 `Type`                            |
| 証明 \(p: P\)                  | プログラム `x: T = ...`              |
| 含意 \(P \rightarrow Q\)       | 関数型 `(P) -> Q`                    |
| 連言 \(P \wedge Q\)            | 積型 `{ a: P, b: Q }`                |
| 選言 \(P \vee Q\)              | 和型 `{ a(P) \| b(Q) }`              |
| 全称量化 \(\forall x:T. P(x)\) | ジェネリクス `(T: Type) -> ...`      |
| 真 \(\top\)                    | `Void`（Unit、デフォルト値あり）     |
| 偽 \(\bot\)                    | `Never`（零コンストラクタ、値なし）  |
| 型宇宙 \(Type_n : Type_{n+1}\) | 宇宙階層（ラッセルの逆理を防ぐため） |
| case 分析                      | 型レベル `match`                     |

> **注意**：型レベル `match` は分類討論（case
> analysis）であり、数学的帰納法ではない。帰納法は型レベル再帰関数 + コンパイラの停止性検査を必要とする。

### 0.2 型が命題、プログラムが証明

YaoXiang において、この対応関係は設計の一級原則として位置づけられる：

- **停止する型レベル計算は正しい構成的証明に対応する**。YaoXiang の型族（例えば `Nat` 上の `Add`
  の case 分析 + 再帰呼び出し）は本質的に数学的帰納法の型レベル符号化である——ただし、コンパイラが停止性検査を行えることが前提である。
- **型検査は証明の検証である**。あるプログラムが型検査を通過するということは、論理的命題が構成的に証明されたことに相当する。

### 0.3 言語設計への影響

Curry-Howard 同型が YaoXiang において具体的に体现される：

1. **宇宙階層**（RFC-010）：`Type₀ : Type₁ : Type₂ …` により、`Type: Type`
   に起因する論理的逆理（Girard の逆理）を回避する
2. **型族**（RFC-011）：自然数 `Nat(Zero/Succ)`
   の型レベル case 分析 + 再帰呼び出しは Peano 公理に対応する——コンパイラの停止性検査が前提
3. **条件型**（RFC-011）：`If: (C: Bool, T: Type, E: Type) -> Type` は論理の case 選言に対応する
4. **値依存型**（RFC-011）：`Vec: (n: Int) -> Type`
   は「各整数 n に対し型が存在する」という有界限量に対応する

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

> **設計説明**：RFC-010 は「すべてが代入である」統一モデル（`name: type = value`）を提案しているが、構文の層では型と値を区別する必要がある。コンパイラ実装では
> `Type` と `Expr` は二つの独立した AST 列挙（`ast.rs:406` と `ast.rs:25`）であり、`TypeExpr`
> は BNF プレースホルダとして実装の `Type` 列挙に対応し、「この位置に型が期待される」ことを表す。

---

## 第二章：基本型

### 2.1 プリミティブ型

| 型       | 論理的対応   | 説明                                                                                 | デフォルトサイズ |
| -------- | ------------ | ------------------------------------------------------------------------------------ | ---------------- |
| `Type`   | —            | メタ型                                                                               | 0 バイト         |
| `Never`  | ⊥（偽/空型） | 零コンストラクタ、値なし。発散/panic の戻り型。任意の T に対し `Never <: T` が成立。 | 0 バイト         |
| `Void`   | ⊤（真/Unit） | デフォルト void 値を持つ零フィールド積型。`x: Void = <デフォルト>` が合法。          | 0 バイト         |
| `Bool`   | —            | ブール値：`true` / `false`                                                           | 1 バイト         |
| `Int`    | —            | 符号付き整数                                                                         | 8 バイト         |
| `Uint`   | —            | 符号なし整数                                                                         | 8 バイト         |
| `Float`  | —            | 浮動小数点数                                                                         | 8 バイト         |
| `String` | —            | UTF-8 文字列                                                                         | 可変             |
| `Char`   | —            | Unicode 文字                                                                         | 4 バイト         |
| `Bytes`  | —            | 生バイト列                                                                           | 可変             |

ビット幅指定の整数：`Int8`, `Int16`, `Int32`, `Int64`,
`Int128`。ビット幅指定の浮動小数点：`Float32`, `Float64`。

### 2.2 Never と Void：⊥ と ⊤

`Never` と `Void` は型システムの論理的プリミティブであり、それぞれ偽（⊥）と真（⊤）に対応する。

**Never（⊥、偽/空型）** — 譲歩できない三性質：

1. **零コンストラクタ**：リテラルや式で `Never` 型の値を生成することはできない。`x: Never = ...`
   の右辺は記述不能。
2. **爆発原理**：任意の型 `T` に対し `Never <: T` が成立する。`assert(false)` は `Never`
   を返し、後続コードは型検査を通過できる（実際には到達不能だが）。
3. **発散マーカー**：`f: (...) -> Never` は `f` が戻らないことを示す。コンパイラはこれに基づき dead
   code 解析と `match` 分岐合流を行う。

`Never` は組込型名（`Int`/`Bool` と同じ登録経路）であり、キーワードではない。

**Void（⊤、真/Unit）** — ちょうど一つの居住者（デフォルト void 値）を持つ。`Void`
は零フィールド積型の単位元である。`x: Void = <デフォルト>` が合法であり、関数がデフォルトで `return`
を持たない場合は `Void` を返す。

---

## 第三章：複合型

### 3.1 レコード型

**統一構文**：`Name: Type = { field1: Type1, field2: Type2, ... }`

```
RecordType  ::= '{' FieldList? '}'
FieldList   ::= Field (',' Field)* ','?
Field       ::= Identifier ':' TypeExpr
            |  Identifier                 // インタフェース制約
```

```yaoxiang
// 単純なレコード型
Point: Type = { x: Float, y: Float }

// 空のレコード型
Empty: Type = {}

// ジェネリクス付きレコード型
Pair: (T: Type) -> Type = { first: T, second: T }

// インタフェースを実装するレコード型
Point: Type = {
    x: Float,
    y: Float,
    Drawable,
    Serializable
}
```

**規則**：

- レコード型は波括弧 `{}` で定義する
- フィールド名の直後にコロンと型を続ける
- 型本体内のインタフェース名は当該インタフェースの実装を表す

> **名前空間の所属**：`Type.name` 接頭辞（例：`Point.draw`）は関数が `Point`
> の名前空間に属することを示す。これはいかなる暗黙のバインディングも引き起こさない。`p.draw()`
> のような `.`
> 呼び出し構文を有効にするには、明示的なバインディングが必要である：`Point.draw = draw[0]`。詳細は RFC-004 および RFC-010 を参照。

#### 3.1.1 フィールドのデフォルト値

型フィールドにはデフォルト値を指定でき、構築時には任意の引数で提供できる：

```yaoxiang
// デフォルト値を持つフィールド - 構築時に任意
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

- `field: Type = expression` → デフォルト値あり、構築時は省略可
- `field: Type` → デフォルト値なし、構築時は必須

#### 3.1.2 組み込みバインディング

型定義体内で直接メソッドをバインドできる：

```yaoxiang
// 方式1：外部関数を参照してバインド
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]    // 位置0にバインド
}
// 呼び出し：p1.distance(p2) -> distance(p1, p2)

// 方式2：無名関数 + 位置バインド
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

### 3.2 インタフェース型

```
InterfaceType ::= '{' FnField (',' FnField)* ','?
FnField       ::= Identifier ':' FnType
FnType        ::= '(' ParamTypes? ')' '->' TypeExpr
```

**構文**：インタフェースは全フィールドが関数型であるレコード型である

```yaoxiang
// インタフェース定義
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

// 空インタフェース
EmptyInterface: Type = {}
```

**インタフェース実装**：型は定義末尾にインタフェース名を列挙することでインタフェースを実装する

```yaoxiang
// インタフェースを実装する型
Point: Type = {
    x: Float,
    y: Float,
    Drawable,        // Drawable インタフェースを実装
    Serializable     // Serializable インタフェースを実装
}
```

**インタフェース直接代入**：具象型はインタフェース型変数に直接代入できる（構造的部分型付け）

```yaoxiang
// 直接代入（コンパイル時に具象型を決定可能 → ゼロオーバーヘッド呼び出し）
d: Drawable = Circle(1)
d.draw(screen)        // コンパイル後：直接 circle_draw を呼び出す、vtable なし

// 関数の戻り値（コンパイル時に決定不能 → vtable 呼び出し）
d: Drawable = get_shape()
d.draw(screen)        // vtable でメソッドを検索

// 関数の引数としてのインタフェース
process: (d: Drawable) -> Void = d.draw(screen)
```

**コンパイル時最適化戦略**：

| シナリオ         | 推論結果         | 呼び出し方式               |
| ---------------- | ---------------- | -------------------------- |
| 具象型の直接代入 | 具象型を決定可能 | 直接呼び出し（ゼロコスト） |
| 関数の戻り値     | 不明             | vtable                     |
| 異種集合         | 複数の型         | vtable                     |

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

### 4.1 ジェネリクス引数の構文

ジェネリクス引数は関数型の一部であり、通常引数と同様に `()` 構文を用いる：

```
GenericType     ::= Identifier '(' TypeArgList ')'
TypeArgList     ::= TypeExpr (',' TypeExpr)* ','?
TypeBound       ::= Identifier
                 |  Identifier '+' Identifier ('+' Identifier)*
```

ジェネリック型定義において、`(T: Type)` は型コンストラクタの引数シグネチャであり、`-> Type`
は戻り型を表す：

```yaoxiang
List: (T: Type) -> Type = { ... }
Map: (K: Type, V: Type) -> Type = { ... }
```

ジェネリック関数においても、型引数はシグネチャで宣言し、コンパイラが実引数から自動推論する：

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
    push: (self: List(T), item: T) -> Void,   // self は単なる規約名、キーワードではない
    get: (self: List(T), index: Int) -> Option(T)
}
```

### 4.3 ジェネリック構築呼び出しと型推論

ジェネリック型定義のフィールドリストは**自動的にコンストラクタを生成する**：各フィールドが構築引数に対応し、フィールド名が引数名となる。デフォルト値を持つフィールドは構築時に省略可能で、デフォルト値を持たないフィールドは必須である。関数型フィールド（メソッド）は構築引数を生成しない。

```yaoxiang
// 型定義
Container: (T: Type) -> Type = {
    value: T,        // デフォルト値なし → 構築引数必須
    extra: T,
}
// 自動展開された完全形式（コンパイラの内部ビュー、ユーザの手書きは不要）：
// Container: (T: Type) -> (value: T, extra: T) -> Type = {
//     value: T = value,
//     extra: T = extra,
// }

// 呼び出し：自動生成されたコンストラクタの呼び出し
c  = Container(42, 43)            // 構築引数はフィールド順に渡す；T は要素から自動展開 = Int
c2 = Container("a", "b")          // T = String
c3 = Container(Int)(42, 43)       // 明示的な型引数 + 位置指定の構築引数
c4 = Container(Int)(extra=43, value=42)  // フィールド名指定、順序任意
c5 = Container(Int)()             // 空構築：フィールドはデフォルト値/零値を取得（データは事後代入）

// フィールドのデフォルト値 → 構築引数の省略可
Point: (T: Type) -> Type = { x: T = 0, y: T = 0 }
p  = Point(1.5, 2.5)              // T = Float、x←1.5, y←2.5
p2 = Point(Int)()                 // x=0, y=0
```

**呼び出し規則**（単一丸括弧、宣言引数と逐次照合、左から右へ）：

1. 実引数は宣言引数と逐次照合される：`Type`
   位置は型実引数を受け付け、コンパイル時値引数位置（例：`Int`）はコンパイル時定数を受け取る。
2. コンパイル時値引数位置への照合が成功した場合（部分一致）、型構築として処理する：全引数位置を逐次検査し、エラーは宣言順に従って**最初に不一致/欠落した引数**を報告する。
3. 実引数が宣言引数と完全に対応しない場合（すべて値で、コンパイル時値引数位置に一致しない）、構築引数として処理する：位置指定はフィールド順に従い、型引数は要素型から自動展開される。

```yaoxiang
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),
    data: Array(Array(T, Cols), Rows),
}

m: Matrix(Int, 3, 4)              // 型位置：一層型構築
m2 = Matrix(Int, 3, 4)(data=[[1,2,3,4],[5,6,7,8],[9,10,11,12]])  // 二層：型 + 構築引数
m3 = Matrix(Int, 3, 4)()          // 空構築（RFC-011 §9.3 パターン、データは事後代入）

Matrix(42)    // ❌ 位置0: T←42 が不一致（42 は型ではない）；位置1: Rows←42 が一致；
              //    位置2: Cols 欠落 → 最初のエラーを報告：T は Type を期待、42 が見つかった
Container(42) // ❌ 構築引数 extra が不足
Container(42, 43, 44)  // ❌ 構築引数の超過
```

**型推論**：ジェネリック型コンストラクタの型引数は構築引数の要素から自動展開される（`Container(42, 43)`
→ T=Int）；ジェネリック関数の型引数は実引数の型から自動展開される（`map(numbers, f)` → T=Int,
R=String、§4.1 を参照）。展開できない場合は明示的な指定が必須である。

---

## 第五章：型制約

### 5.1 単一制約

```
ConstrainedType ::= '(' Identifier ':' TypeBound ')' TypeExpr
```

```yaoxiang
// インタフェース型定義（制約として使用）
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

### 6.1 関連型の定義

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
以外の具体型（例：`Int`）で注釈されたジェネリクス引数を**コンパイル時値引数**（compile-time value
parameter）と呼ぶ。デフォルトでコンパイル時に決定され、**`const`
キーワードは不要**（実装内部ではかつて「const ジェネリクス」と呼ばれていたが、ドキュメントでは統一して「コンパイル時値引数」を使用する）。

**核心設計**：`(n: Int)` コンパイル時値引数 + `(n: n)`
値引数で、コンパイル時定数とランタイム値を区別する。

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

// 使用例
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
// IsTrue ブリッジと Assert 精錬型（詳細は §8.3）
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

### 8.3 Assert 精錬型と assert 表明

`assert` と `Assert`
は同一の精錬プリミティブの二面であり、dispatch 分岐パイプラインにより「述語の自由変数がコンパイル時に到達可能か」に基づいて自動選択される。

**核心シグネチャ**：`assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**dispatch 分岐規則**：

| 判定基準                                                                   | モード      | 振る舞い                                                                                 |
| -------------------------------------------------------------------------- | ----------- | ---------------------------------------------------------------------------------------- |
| すべての自由変数がコンパイル時に既知（ジェネリクス引数、コンパイル時定数） | CompileTime | 証明パイプラインに入る：true → Void に消去、false → コンパイルエラー（Never に居住不能） |
| ランタイム自由変数が存在（関数の引数、外部入力）                           | Runtime     | ランタイム Bool 検査を挿入し、フロー依存仮定集合 Γ に精錬事実を注入する                  |

**フロー依存仮定集合 Γ**：

コンパイラは各制御フロー点の既知命題集合を維持する：

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP 伝播
mut x = x - 5       // Γ = {}  ← mut kill set：古い仮定が無効化される
```

`mut` 変数への代入後、当該変数に関するすべての仮定が削除される（kill
set）。分岐合流時、Γ は各分岐の交差を取る。

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
// インタフェース組み合わせ = 型交差
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
// プラットフォーム型列挙（標準ライブラリで定義）
Platform: Type = { X86_64: () -> Platform, AArch64: () -> Platform, RISC_V: () -> Platform, ARM: () -> Platform, X86: () -> Platform }

// P は事前定義済みジェネリクス引数名で、現在のコンパイルプラットフォームを表す
sum: (P: X86_64)(arr: Array(Float)) -> Float = {
    return avx2_sum(arr.data, arr.length)
}

sum: (P: AArch64)(arr: Array(Float)) -> Float = {
    return neon_sum(arr.data, arr.length)
}
```

---

## 第十一章：型属性

YaoXiang には区別すべき型属性が一つだけある：線形 vs コピー可能。コンパイラにより自動推論される。

### 11.1 Move（デフォルトの所有権移転）

すべての型はデフォルトで Move セマンティクスに従う。代入、引数渡し、戻り値 = 所有権の移転。

```yaoxiang
p: Point = Point(1.0, 2.0)
q = p           // Move、p は以降読み取り不可
```

### 11.2 Dup（浅いコピー：ハンドルをコピー、データを共有）

**Dup 属性は参照/トークン型に用いる**。Dup 型の代入 = 浅いコピー——ハンドル/トークンをコピーし、底层データは共有される。複数の所有者が同一のデータブロックを指す。

| 型               | 属性   | 説明                                                                |
| ---------------- | ------ | ------------------------------------------------------------------- |
| `&T`             | Dup    | 零サイズ読み取りトークン、トークンコピー = 同じデータへの複数の視点 |
| `ref T`          | Dup    | Rc/Arc コピー = 参照カウント+1、ヒープデータを共有                  |
| `&mut T`         | Linear | 零サイズ書き込みトークン、排他的、コピー不可                        |
| その他すべての型 | Move   | デフォルトの所有権移転                                              |

**プリミティブ値型**（Int, Float, Bool,
Char）はコンパイラに組み込まれた特別な処理を受ける：代入時に自動的に値がコピーされ、二つの値は完全に独立する。これはコンパイラのネイティブ動作であり、Dup 型属性には含まれない。

```yaoxiang
// &T: Dup、自由に別名化可能
view: &Point = &p
view2 = view     // Dup：トークンをコピー、両者とも有効
print(view.x)    // 使用可
print(view2.x)   // 使用可

// &mut T: Linear、コピー不可
mut_ref: &mut Point = &mut p
// r2 = mut_ref  // ❌ &mut T は Dup ではない、コピー不可
```

### 11.3 Clone（明示的な深いコピー）と Dup の関係

**Clone** は明示的な深いコピーインタフェースである。すべての型は Clone を実装でき、`.clone()`
メソッドを提供する。

```yaoxiang
// Clone インタフェース定義（標準ライブラリ）
Clone: Type = {
    clone: () -> Clone
}

// 使用例
p: Point = Point(1.0, 2.0)
backup = p.clone()    // 深いコピー、p は引き続き使用可能
p2 = p.clone()        // 複数回クローン可能
```

**Dup と Clone の違い**：

|                    | Dup                                                     | Clone                                  |
| ------------------ | ------------------------------------------------------- | -------------------------------------- |
| **セマンティクス** | 浅いコピー：ハンドル/トークンをコピー、底层データを共有 | 深いコピー：完全な独立複製を作成       |
| **呼び出し方式**   | 暗黙的（代入/引数渡しで自動）                           | 明示的（`.clone()`）                   |
| **変更の影響**     | 相互に影響する（底层データを共有）                      | 互いに影響しない（独立した複製）       |
| **適用型**         | `&T` トークン、`ref T`                                  | Clone インタフェースを実装する任意の型 |
| **コスト**         | ゼロコスト（トークンは零サイズ型）                      | 型に依存                               |

**Dup は Clone を含意せず、Clone も Dup を含意しない**——これらは二つの直交する概念である：

```yaoxiang
// Dup 型：トークンをコピー、底层データを共有
view: &Point = &p
view2 = view        // Dup：トークンをコピー、両者は同じ p を指す
print(view.x)       // 使用可
print(view2.x)      // 使用可、見えるのは同じデータ

// プリミティブ値型：コンパイラが自動的に値をコピー（Dup ではない）
x: Int = 42
y = x               // 値コピー、x と y は完全に独立
print(x)            // 使用可

// Clone：明示的な深いコピー、独立した複製を作成
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone：深いコピー、p は引き続き使用可能
r = p               // Move：所有権移転、Point は Dup でもプリミティブ値型でもないため
```

**設計意図**：

- Dup はトークン/参照型に用い、「同じデータの複数の視点」という問題を解決する
- Clone は独立した複製が必要なシナリオに用い、明示的呼び出しによりコストを可視化する
- プリミティブ値型（Int/Float/Bool/Char）のコピーはコンパイラの組込動作であり、Dup には含まれない
- ほとんどのカスタム型はデフォルトで Move であり、ゼロコピーで高性能である

## 第十二章：借用トークン型

### 12.1 核心概念

`&T` と `&mut T`
は**零サイズのコンパイル時トークン型**である。これらは「参照」ではなく、「アクセス権限の型レベル証明」である。

```
&T      →  零サイズ、ソースデータを凍結（この期間中 WriteToken の取得を禁止）、
          凍結保証の下で複数の読み取りが安全 → Dup（コピー可）
&mut T  →  零サイズ、排他的読み書き（他のすべてのトークンを禁止）、
          排他的アクセス下ではコピーが無意味 → Linear（Dup でない）
```

**主要な特性**：

- トークンは**通常の型**であり、他のすべての型と同じスコープ規則に従う
- ライフタイム注釈 `'a` は不要
- 専用の借用検査器は不要——型属性（Dup/Linear）が自然に権限を推論する
- コンパイル後完全に消滅し、ランタイムオーバーヘッドはゼロ

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

// 呼び出し側：コンパイラが自動的に借用または Move を選択
p = Point(1.0, 2.0)
p.print()                       // コンパイラが自動的に &Point トークンを作成
p.shift(1.0, 1.0)               // コンパイラが自動的に &mut Point トークンを作成
p.print()                       // OK、前のトークンは shift 呼び出し終了とともに解放済み

// 複数の &T トークンが共存可能——Dup 型は自由にコピー可能
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 トークンのスコープと伝播

トークンは通常の型であるため、通常の型のすべての操作をサポートする：

**トークンの返却**——トークンは戻り値とともに伝播する：

```yaoxiang
// ✅ サブトークンと親トークンを一緒に返す
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // トークンは呼び出し元に返される
print(px_ref)                    // OK、トークンはまだスコープ内
```

**構造体への格納**——構造体はトークンフィールドを保持できる：

```yaoxiang
// ✅ 構造体がトークンをフィールドとして保持
Window: Type = {
    target: Point,
    view: &Point,              // トークンフィールド——target への読み取り専用ビューを保持
}
```

**クロージャはキャプチャせず、コンテキストは作成時点で固定化**——クロージャは自身の引数のみを取り、外側のデータを必要とする場合はカリー化により作成時点で値を固定化してクロージャ内に取り込む：

```yaoxiang
// ✅ コンテキストはカリー化により固定化：threshold は引数、gt_point(threshold) は作成時点で値をクロージャに固定化
gt_point: (t: Float) -> (p: Point) -> Bool = (p) => p.x > t
filter_by_threshold: (items: List(Point), threshold: Float) -> List(Point) = {
    items.filter(gt_point(threshold))
}
```

> 注：クロージャ（関数値）がエスケープした後、その定義箇所のスコープはすでに死んでいる可能性があるため、外側の変数を暗黙的にキャプチャしてはならない。ただし、呼び出し点（作成点）のスコープは必ず生存しているため、コンテキストがその時点で値として固定化されクロージャに入ることは安全である。

### 12.4 自動借用選択

呼び出し側では、コンパイラが以下の優先順位で自動的に選択する：

```
1. 実引数の後続使用がある場合 → 優先的にトークンを作成（メソッドシグネチャに応じて &T または &mut T）
2. 実引数の後続使用がない場合 → Move
3. 優先照合順序：&T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // print の引数型は &Point → コンパイラが &Point トークンを作成
p.shift(1.0, 1.0)  // shift の引数型は &mut Point → コンパイラが &mut Point トークンを作成
p2 = p             // 後続使用なし → Move
```

### 12.5 トークン衝突検出

トークン衝突検出は**借用ホーア命題**（RFC-009a）であり、独立したフロー依存解析ではない。コンパイラが借用命題（`borrow_conflict`/`use_after_move`/`use_after_drop`/`mut_violation`）を自動生成し、証明パイプラインに送って検証する。トークン活性は区間
`[created_at, last_use]` である（RFC-009a §逆 BFS 活性解析を参照）：

```yaoxiang
// ❌ &mut と派生した &T を同時にアクティブにできない
bad_alias: (p: &mut Point) -> Void = {
    p.x = 10.0                   // ✅ WriteToken の通常使用
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

// ❌ 同じ実引数から同時に &mut トークンと他のトークンを作成できない
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p から同時に &mut と & トークンを派生
```

### 12.6 コンパイラ内部：ブランド機構

ユーザはブランドに一切触れない。コンパイラが内部で各トークンにコンパイル時一意識別子を割り当てる：

```
ユーザに見える表現     コンパイラ内部表現
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N はコンパイル時一意整数
&mut Point     →  WriteToken(Point, #M)   // #M はコンパイル時一意整数
```

ブランドの用途：

- **偽造防止**：トークンは所有者カプセルからのみ取得でき、凭空に構築できない
- **関連追跡**：フィールドアクセスから派生した `&Float`
  は派生ブランド（`#N.field_x`）を携带し、コンパイラが親トークンまで追跡可能
- **衝突検出**：同源の WriteToken と派生 ReadToken は同時にアクティブにできない

ブランドは単態化とインライン化後に完全に消滅し、生成された機械語には存在しない。**ランタイムオーバーヘッドはゼロ**。

### 12.7 トークン Sum 型

```
&BorrowToken ::= &T          // ReadToken（ソースデータを凍結 → Dup 安全）
               | &mut T      // WriteToken（排他的読み書き → Linear）
```

### 12.8 借用トークン vs ref

|            | `&T` / `&mut T`                            | `ref`                                 |
| ---------- | ------------------------------------------ | ------------------------------------- |
| 役割       | 一目見る/その場で変更                      | 共有所有                              |
| 範囲       | トークン値のスコープに従う                 | スコープを超える                      |
| コスト     | ゼロコスト（零サイズ型、コンパイル後消滅） | Rc または Arc（コンパイラが選択）     |
| エスケープ | 可（トークンは戻り値/構造体で伝播）        | 本来エスケープ用                      |
| タスク間   | 不可（トークンはタスク間伝播を未実装）     | 可（コンパイラが自動的に Arc を選択） |
| 環検出     | 関与しない                                 | タスク内は静かに処理、タスク間は lint |

> 注（未定義）：ref 作成後の内容の読み取り方法（参照解除/メソッド/自動）については仕様未定義。実装現状では
> `*a` は E1052 を報告する。定義後に本節に補完予定。

---

## 付録：型定義クイックリファレンス

### A.1 型定義

```
// === レコード型（波括弧） ===

// レコード型
Point: Type = { x: Float, y: Float }

// バリアント付きレコード型（関数フィールドを使用）
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// === インタフェース型（波括弧、フィールドは全関数） ===

// インタフェース定義
Serializable: Type = { serialize: () -> String }

// インタフェースを実装する型
Point: Type = {
    x: Float,
    y: Float,
    Serializable    // Serializable インタフェースを実装
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
// すべての型はデフォルトで Move。代入、引数渡し、戻り値 = 所有権の移転

// === プリミティブ値型（コンパイラ組込） ===
Int, Float,     // 代入時に自動的に値がコピーされ、二つの値は完全に独立
Bool, Char      // Dup ではなく、コンパイラのプリミティブ組込処理

// === Dup（浅いコピー：ハンドルをコピー、底层データを共有） ===
&T              // 零サイズ読み取りトークン、トークンコピー = 同じデータへの複数の視点
ref T           // Rc/Arc コピー = 参照カウント+1、ヒープデータを共有

// === Linear ===
&mut T          // 零サイズ書き込みトークン、Linear（排他的、コピー不可）

// === Clone（明示的な深いコピー） ===
value.clone()   // 独立した複製を作成、変更は元の値に影響しない
```

### A.4 借用トークン クイックリファレンス

```
// === 借用トークン ===
&T              // 零サイズコンパイル時読み取りトークン、ソースデータを凍結 → Dup（コピー可）
&mut T          // 零サイズコンパイル時書き込みトークン、排他的読み書き → Linear（コピー不可）

// 呼び出し側の自動選択
// 1. 実引数の後続使用がある → トークン作成
// 2. 実引数の後続使用がない → Move
// 3. 優先照合：&T < &mut T < Move

// トークン伝播
// ✅ 返却可、構造体格納可、クロージャキャプチャ可
// ❌ タスク間不可（トークンはタスク間伝播を未実装）
```

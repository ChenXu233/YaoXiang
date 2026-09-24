# 型システム仕様

本文書は YaoXiang プログラミング言語の型システム仕様を定義する。基本型、複合型、ジェネリクス、トレイトを含む。

---

## 第零章：理论基础

### 0.1 Curry-Howard 対応

Curry-Howard 対応（Curry-Howard
correspondence）は、YaoXiang 型システムの理论基础である。プログラミング言語の型システムと数理論理の深層対応関係を明らかにする：

| 論理学                         | プログラミング言語                        |
| ------------------------------ | ----------------------------------------- |
| 命題 \(P\)                     | 型 `Type`                                 |
| 証明 \(p: P\)                  | プログラム `x: T = ...`                   |
| 含意 \(P \rightarrow Q\)       | 関数型 `(P) -> Q`                         |
| 連言 \(P \wedge Q\)            | 積型 `{ a: P, b: Q }`                     |
| 選言 \(P \vee Q\)              | 和型 `{ a(P) \| b(Q) }`                   |
| 全称量化 \(\forall x:T. P(x)\) | ジェネリクス `(T: Type) -> ...`           |
| 真 \(\top\)                    | `Void`（Unit、デフォルト値あり）          |
| 偽 \(\bot\)                    | `Never`（コンストラクタゼロ、居住値なし） |
| 型宇宙 \(Type_n : Type_{n+1}\) | 宇宙階層（Russell パラドックス防止）      |
| case 分析                      | 型レベル `match`                          |

> **注意**：型レベル `match` は場合分け（case
> analysis）であり、数学的帰納法ではない。帰納法は型レベル再帰関数 + コンパイラの停止性チェックを必要とする。

### 0.2 型は命題、プログラムは証明

YaoXiang において、この対応関係は設計の最上位原則である：

- **停止する型レベル計算は正しい構成的証明に対応する**。YaoXiang の型族（`Nat` 上の `Add`
  の case 分析 + 再帰呼び出しなど）は、本質的に数学的帰納法の型レベルエンコーディングである——ただし、コンパイラが停止性チェックを行えることが前提となる。
- **型検査は証明の検証である**。あるプログラムが型検査を通過するということは、論理的命題が構成的に証明されたことと同義である。

### 0.3 言語設計への影響

YaoXiang における Curry-Howard 対応の具体的な現れ：

1. **宇宙階層**（RFC-010）：`Type₀ : Type₁ : Type₂ …` により、`Type: Type`
   が引き起こす論理的パラドックス（Girard パラドックス）を回避
2. **型族**（RFC-011）：自然数 `Nat(Zero/Succ)`
   の型レベル case 分析 + 再帰呼び出しは Peano 公理に対応——コンパイラの停止性チェックが前提
3. **条件型**（RFC-011）：`If: (C: Bool, T: Type, E: Type) -> Type` は論理における case 選言に対応
4. **値依存型**（RFC-011）：`Array: (T: Type, N: Int) -> Type`
   は「各整数 N に対し型が存在する」という有限量化に対応

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

> **設計説明**：RFC-010 は「すべて代入である」という統一モデル（`name: type = value`）を提案しているが、文法レベルでは型と値を区別する必要がある。コンパイラ実装では
> `Type` と `Expr` は独立した AST enum であり（`ast.rs:406` と `ast.rs:25`）、`TypeExpr`
> は BNF プレースホルダーとして実装の `Type` enum に対応し、「この位置は型を期待する」を表す。

---

## 第二章：基本型

### 2.1 プリミティブ型

| 型       | 論理的対応   | 説明                                                                                    | デフォルトサイズ |
| -------- | ------------ | --------------------------------------------------------------------------------------- | ---------------- |
| `Type`   | —            | メタ型                                                                                  | 0 バイト         |
| `Never`  | ⊥（偽/空型） | コンストラクタゼロ、値なし。発散/panic の戻り型。`Never <: T` が任意の T に対して成立。 | 0 バイト         |
| `Void`   | ⊤（真/Unit） | デフォルト void 値を持つゼロフィールド積型。`x: Void = <デフォルト>` が合法。           | 0 バイト         |
| `Bool`   | —            | ブール値：`true` / `false`                                                              | 1 バイト         |
| `Int`    | —            | 符号付き整数                                                                            | 8 バイト         |
| `Uint`   | —            | 符号なし整数                                                                            | 8 バイト         |
| `Float`  | —            | 浮動小数点数                                                                            | 8 バイト         |
| `String` | —            | UTF-8 文字列                                                                            | 可変             |
| `Char`   | —            | Unicode 文字                                                                            | 4 バイト         |
| `Bytes`  | —            | 生バイト                                                                                | 可変             |

ビット幅付き整数：`Int8`, `Int16`, `Int32`, `Int64`, `Int128` ビット幅付き浮動小数点：`Float32`,
`Float64`

### 2.2 Never と Void：⊥ と ⊤

`Never` と `Void` は型システムの論理的プリミティブであり、偽（⊥）と真（⊤）にそれぞれ対応する。

**Never（⊥、偽/空型）** — 譲歩不可能な 3 つの性質：

1. **コンストラクタゼロ**：`Never` 型の値を生成するリテラルや式は存在しない。`x: Never = ...`
   の右辺は書けない。
2. **爆発原理**：`Never <: T` が任意の型 `T` に対して成立する。`assert(false)` は `Never`
   を返し、その後のコードは型検査を通過できる（実際には実行されないが）。
3. **発散マーカー**：`f: (...) -> Never` は `f`
   が決して戻らないことを示す。コンパイラはこれにより dead code 分析と `match` 分岐合流を行う。

`Never` は組み込み型名であり（`Int`/`Bool` と同じ登録パス）、キーワードではない。

**Void（⊤、真/Unit）** — ちょうど 1 つの居住者（デフォルト void 値）を持つ。`Void`
はゼロフィールド積型の単位元である。`x: Void = <デフォルト>`
が合法。ブロックの値は**末尾式**によって与えられる（空ブロック `{}` は `Void`）、詳細は
[RFC-010a](../design/rfc/accepted/010a-tail-expression-and-return.md) を参照。

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

**ルール**：

- レコード型は波括弧 `{}` で定義する
- フィールド名の後にコロンと型を続ける
- インターフェース名は型本体内に記述することで、そのインターフェースの実装を示す

> **名前空間の所属**：`Type.name` 接頭辞（例：`Point.draw`）は関数が `Point`
> の名前空間に属することを示す。これは暗黙のバインディングを引き起こさない。`p.draw()` のような `.`
> 呼び出し構文を有効にするには、明示的なバインディングが必要：`Point.draw = draw[0]`。詳細は RFC-004 と RFC-010 を参照。

#### 3.1.1 フィールドのデフォルト値

型のフィールドにはデフォルト値を設定でき、構築時にはオプションで提供できる：

```yaoxiang
// デフォルト値を持つフィールド - 構築時はオプション
Point: Type = {
    x: Float = 0,
    y: Float = 0
}

// 使用
Point()           // -> Point(x=0, y=0)
Point(x=1)       // -> Point(x=1, y=0)
Point(x=1, y=2) // -> Point(x=1, y=2)

// デフォルト値のないフィールド - 構築時は必須
Point2: Type = {
    x: Float,
    y: Float
}

// 使用
Point2(x=1, y=2) // 正しい
Point2()          // エラー
```

**ルール**：

- `field: Type = expression` -> デフォルト値あり、構築時はオプション
- `field: Type` -> デフォルト値なし、構築時は必須

#### 3.1.2 組み込みバインディング

型定義本体内で直接メソッドをバインドできる：

```yaoxiang
// 方法1：外部関数の参照によるバインディング
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]    // 位置 0 にバインド
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

### 3.2 インターフェース型

```
InterfaceType ::= '{' FnField (',' FnField)* ','?
FnField       ::= Identifier ':' FnType
FnType        ::= '(' ParamTypes? ')' '->' TypeExpr
```

**構文**：インターフェースは全フィールドが関数型であるレコード型である

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

**インターフェースの実装**：型は定義の末尾にインターフェース名を列挙することでインターフェースを実装する

```yaoxiang
// インターフェースを実装する型
Point: Type = {
    x: Float,
    y: Float,
    Drawable,        // Drawable インターフェースを実装
    Serializable     // Serializable インターフェースを実装
}
```

**インターフェースへの直接代入**：具体型はインターフェース型変数に直接代入できる（構造的サブタイピング）

```yaoxiang
// 直接代入（コンパイル時に具体型が確定 -> ゼロオーバーヘッド呼び出し）
d: Drawable = Circle(1)
d.draw(screen)        // コンパイル後：circle_draw を直接呼び出し、vtable なし

// 関数の戻り値（コンパイル時に確定できない -> vtable 呼び出し）
d: Drawable = get_shape()
d.draw(screen)        // vtable 経由でメソッドを検索

// インターフェースを関数の引数として
process: (d: Drawable) -> Void = d.draw(screen)
```

**コンパイル時最適化戦略**：

| シナリオ         | 推論結果         | 呼び出し方式                       |
| ---------------- | ---------------- | ---------------------------------- |
| 具体型の直接代入 | 具体型が確定可能 | 直接呼び出し（ゼロオーバーヘッド） |
| 関数の戻り値     | 不明             | vtable                             |
| 異種コレクション | 複数型           | vtable                             |

**コヒーレンスとオーファンルール（不適用、収束説明）**：YaoXiang のインターフェースは構造的型（インターフェース = 全フィールドが関数型のレコード）であり、名目的なトレイトではない——crate/モジュール横断の「誰が誰のために実装できる」という帰属問題は存在せず、Rust 式のオーファンルールとコヒーレンスチェックは適用対象を持たない（裁定記録は RFC-011
§2.1）。構造的世界の対応する保障は**重複実装の拒否**：同じメソッドシグネチャを型に重複定義するとコンパイルエラー（RFC-011a
§3、上書き禁止；オーバーロードは合法）。

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

### 4.1 ジェネリクスパラメータの構文

ジェネリクスパラメータは関数型の一部であり、通常のパラメータと統一して `()` 構文を使用する：

```
GenericType     ::= Identifier '(' TypeArgList ')'
TypeArgList     ::= TypeExpr (',' TypeExpr)* ','?
TypeBound       ::= Identifier
                 |  Identifier '+' Identifier ('+' Identifier)*
```

ジェネリック型定義では、`(T: Type)` は型コンストラクタのパラメータシグネチャであり、`-> Type`
は戻り型を表す：

```yaoxiang
List: (T: Type) -> Type = { ... }
Map: (K: Type, V: Type) -> Type = { ... }
```

### 4.1.1 コンテナ型

コンテナ型はジェネリック型コンストラクタであり、組み込みプリミティブではない——ユーザー定義ジェネリクスと同等に扱われ、统一されたジェネリクスインスタンス化パスで処理される。長さ情報の帰属は 3 つのコンテナ概念の根本的な違いである：

| 型            | 長さ     | セマンティクス                     | 基底                                        |
| ------------- | -------- | ---------------------------------- | ------------------------------------------- |
| `Array(T, N)` | 型       | 固定長配列（const ジェネリクス N） | 中核プリミティブ（スタック/インライン優先） |
| `Vec(T)`      | 実行時値 | 実行時長さの生バッファ、拡張可能   | 中核プリミティブ（ヒープ上の連続バッファ）  |
| `List(T)`     | 実行時値 | 標準ライブラリ型（拡張可能リスト） | ライブラリ：`{ data: Vec(T), length: Int }` |
| `Dict(K, V)`  | 実行時値 | キー値マッピング                   | `HeapValue::Dict`                           |

> `List(T)` は**標準ライブラリ型であり、コンパイラプリミティブではない**：YaoXiang 自身が `std.list`
> で定義し、ユーザー定義ジェネリクレコードと同等に扱われる。拡張可能セマンティクスのすべての戦略（いつ拡張するか、どれだけ拡張するか、共有可能か）はライブラリ内にあり、コンパイラは関与しない。`Vec(T)`
> はそれが依存する最小限の基底プリミティブである。
>
> `Set(T)` は削除済み：リテラル、実行時表現、`std.set`
> のいずれも存在しない。需要が発生した場合は Dict パターンに倣って補完する。

重要なルール：

- **リテラルの着地点はコンテキストが決定**：`[...]` 生リテラルと `List(T)`
  注釈は拡張可能リストに着地し；`Array(T, N)`
  注釈がリテラルに直接作用する場合は固定長配列に着地する。着地点検証：要素数 ==
  N、要素型が T と互換性あり、不一致はコンパイル時 E1002；N がシンボル定数（const パラメータ）の場合、個数検証は型精化段階まで延期される。
- **暗黙の List→Array 変換を禁止**：固定長性は型層で保証される——push は `List(A)`
  レシーバのみ受け入れる。
- **パフォーマンス階層**：下から上に向かってパフォーマンスが減少し、柔軟性が増加：`Array` > `Vec` >
  `List`。
- **インデックス失敗契約**（実行時エラーは過渡状態、目標状態はコンパイル時精化によるカバー、値依存型へ）：
  - インデックス範囲外（負のインデックスを含む）→ `E6003`
  - Dict キー欠落 → `E6008`
- **membership `in` 述語**：`Bool`
  を返しエラーを出さない、右オペランドは List/Array/Dict(キー)/Tuple/String/Range をカバー。第一級の Hoare 述語であり、精化型のコンパイル時証明可能命題の基底である。`

ジェネリック関数では、型パラメータもシグネチャで宣言され、コンパイラは実引数から自動的に推論する：

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
    push: (self: List(T), item: T) -> Void,   // self は単なる規約名であり、キーワードではない
    get: (self: List(T), index: Int) -> Option(T)
}
```

### 4.3 ジェネリック構築呼び出しと型推論

ジェネリック型定義のフィールドリストは**コンストラクタを自動生成する**：各フィールドが構築パラメータに対応し、フィールド名がパラメータ名となる；デフォルト値を持つフィールドは構築時に省略可能で、デフォルト値のないフィールドは必須である。関数型フィールド（メソッド）は構築パラメータを生成しない。

```yaoxiang
// 型定義
Container: (T: Type) -> Type = {
    value: T,        // デフォルト値なし -> 構築パラメータ必須
    extra: T,
}
// 自動展開された完全形式（コンパイラ内部ビュー、ユーザーが手書きする必要はない）：
// Container: (T: Type) -> (value: T, extra: T) -> Type = {
//     value: T = value,
//     extra: T = extra,
// }

// 呼び出し：自動生成されたコンストラクタを呼び出す
c  = Container(42, 43)            // 構築パラメータをフィールド順に埋める；T は要素から自動推論 = Int
c2 = Container("a", "b")          // T = String
c3 = Container(Int)(42, 43)       // 明示的な型パラメータ + 位置式構築パラメータ
c4 = Container(Int)(extra=43, value=42)  // フィールド名式、順序任意
c5 = Container(Int)()             // 空構築：フィールドはデフォルト値/ゼロ値を取得（データは後で代入）

// フィールドデフォルト値 -> 構築パラメータ省略可能
Point: (T: Type) -> Type = { x: T = 0, y: T = 0 }
p  = Point(1.5, 2.5)              // T = Float、x←1.5, y←2.5
p2 = Point(Int)()                 // x=0, y=0
```

**呼び出しルール**（単一括弧、宣言パラメータの逐次マッチング、左から右）：

1. 実引数を宣言パラメータと逐次マッチング：`Type`
   位置は型実引数を受け入れ、コンパイル時値パラメータ位置（例：`Int`）はコンパイル時定数を受け入れる。
2. コンパイル時値パラメータ位置のマッチングが成功した場合（部分マッチング）、型構築として処理：すべてのパラメータ位置を逐次チェックし、エラー時は宣言順序に従って**最初に一致しない/欠落しているパラメータ**を報告する。
3. 実引数が宣言パラメータに完全に対応しない場合（全部値で、コンパイル時値パラメータ位置にマッチングしない）、構築パラメータとして処理：位置式はフィールド順に埋め、型パラメータは要素型から自動推論する。

```yaoxiang
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),
    data: Array(Array(T, Cols), Rows),
}

m: Matrix(Int, 3, 4)              // 型位置：一層型構築
m2 = Matrix(Int, 3, 4)(data=[[1,2,3,4],[5,6,7,8],[9,10,11,12]])  // 二層：型 + 構築パラメータ
m3 = Matrix(Int, 3, 4)()          // 空構築（RFC-011 §9.3 パターン、データは後で代入）

Matrix(42)    // ❌ 位置 0: T←42 が一致しない（42 は型ではない）；位置 1: Rows←42 が一致；
              //    位置 2: Cols 欠落 -> 最初のエラーを報告：T は Type を期待、42 が見つかった
Container(42) // ❌ 構築パラメータ extra が欠落
Container(42, 43, 44)  // ❌ 構築パラメータ過多
```

**型推論**：ジェネリック型コンストラクタの型パラメータは構築パラメータ要素から自動推論される（`Container(42, 43)`
→ T=Int）；ジェネリック関数の型パラメータは実引数型から自動推論される（`map(numbers, f)` → T=Int,
R=String、§4.1 を参照）。推論できない場合は明示的に指定する必要がある。

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

> **制約の求解ソース（RFC-011b）**：演算子制約名（`Add` / `Subtract` / `Multiply` / `Divide` /
> `Modulo` / `Equal` / `Index`）の求解 = インターフェース実装登録表を検索——`T: Add` ≜ `Add(T, T, T)`
> インスタンス化が登録済み；`Equal`
> には他に構造推論（全フィールドが比較可能なレコードは自動比較可能）がある。`Zero` / `One` /
> `PartialOrd` などの名前はまだ定義ソースがなく、宙ぶらりん制約名である。

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

### 7.1 コンパイル時値パラメータ

```
LiteralType   ::= Identifier ':' Int          // コンパイル時定数（候補）
```

> 判断の根拠は**型位置で参照されているか**であり、「具体的な型が注釈されているか」ではない：`add: (a: Int, b: Int) -> Int = a + b`
> において `a`/`b` は実行時値パラメータである（どちらもどの型位置にも現れない）。

**用語**：`Type`
以外の具体型（例：`Int`）で注釈されたジェネリクスパラメータを**コンパイル時値パラメータ候補**と呼び、それがコンパイル時値パラメータになるかどうかは、その値が型位置で参照されているかどうかに依存する（値依存）。**`const`
キーワードは不要**（実装内部ではかつて「const ジェネリクス」と呼んでいたが、ドキュメントは「コンパイル時値パラメータ」に統一する）。

**判定ルール（二ステップ）**：

1. **形態粗選別**：パラメータが `Type`
   以外の具体型（`Int`/`Bool`/`Float`）で注釈されている -> 候補。
2. **用途精選別**：候補名が**型位置**（型本体のフィールド型、内部 `Fn` パラメータ型、`Assert`
   述語、`Array(T, N)`
   型構築実引数位置）に現れる -> 真のコンパイル時値パラメータ；そうでなければ**実行時値パラメータ**。

| 書き方                                                     | 判定                           | 理由                                    |
| ---------------------------------------------------------- | ------------------------------ | --------------------------------------- |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b は実行時値パラメータ       | 値位置にのみ現れる                      |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N はコンパイル時値パラメータ   | N が型構築実引数位置に現れる            |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N はコンパイル時値パラメータ   | N が内部パラメータ k の型として使われる |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N は落空 -> 実行時値パラメータ | N が型本体で参照されていない            |

**核心設計**：`(N: Int)` コンパイル時値パラメータと `(k: N)`
値パラメータを使って、コンパイル時定数と実行時値を区別する。落空候補（形態は候補だが用途が命中しない）は実行時値パラメータに退化する——関数レベルと型コンストラクタパスの両方がこの規則に従う。

```yaoxiang
// コンパイル時値パラメータ：N が型位置（Array 長さスロット）で参照される
Measure: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // N が型構築実引数位置に現れる -> コンパイル時値パラメータ
    length: N
}

// 使用方法：factorial(5) は型位置で評価され（コンパイル時）、結果 120 が型に埋め込まれる
arr: Measure(Int, factorial(5))  // コンパイラはコンパイル時に factorial(5) = 120 を計算

// 値依存：N が内部パラメータ k の型として使われる
// N はコンパイル時値パラメータ（(k: N) の型位置に現れる）；
// k は実行時値パラメータ、その型はリテラル型 N（単一値型）。
factorial: (N: Int) -> (k: N) -> Int = {
    match k {
        0 => 1,
        _ => k * factorial(k - 1)
    }
}
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
// IsTrue ブリッジと Assert 精化型（詳細は §8.3）
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

### 8.3 Assert 精化型と assert アサーション

`assert` と `Assert`
は同じ精化プリミティブの二面であり、dispatch 分派パイプラインが「述語の自由変数がコンパイル時に到達可能か」に基づいて自動選択する。

**核心シグネチャ**：`assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**dispatch 分派ルール**：

| 判定基準                                                                         | モード      | 挙動                                                                                   |
| -------------------------------------------------------------------------------- | ----------- | -------------------------------------------------------------------------------------- |
| すべての自由変数がコンパイル時に既知（ジェネリクスパラメータ、コンパイル時定数） | CompileTime | 証明パイプラインへ：true -> Void に消去、false -> コンパイルエラー（Never は居住不能） |
| 実行時自由変数が存在する（関数パラメータ、外部入力）                             | Runtime     | 実行時 Bool チェックを挿入し、フロー依存仮説集合 Γ に精化事実を注入                    |

**フロー依存仮説集合 Γ**：

コンパイラは各制御流点の既知命題集合を維持する：

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP 伝播
mut x = x - 5       // Γ = {}  ← mut kill set：古い仮説が無効化される
```

`mut` 変数代入後、その変数に関するすべての仮説が削除される（kill
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
// インターフェース合成 = 型交差
DrawableSerializable: Type = Drawable & Serializable

// 交差型の使用
process: (T: Drawable & Serializable)(item: T, screen: Surface) -> String = {
    item.draw(screen)
    return item.serialize()
}
```

---

## 第十章：関数オーバーロードと特化

### 10.1 関数オーバーロード

```yaoxiang
// 基本特化：関数オーバーロードを使用（コンパイラが自動選択）
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
// プラットフォーム型 enum（標準ライブラリ定義）
Platform: Type = { X86_64: () -> Platform, AArch64: () -> Platform, RISC_V: () -> Platform, ARM: () -> Platform, X86: () -> Platform }

// P は事前定義されたジェネリクスパラメータ名で、現在のコンパイルプラットフォームを表す
sum: (P: X86_64)(arr: Array(Float)) -> Float = {
    return avx2_sum(arr.data, arr.length)
}

sum: (P: AArch64)(arr: Array(Float)) -> Float = {
    return neon_sum(arr.data, arr.length)
}
```

---

## 第十一章：型属性

YaoXiang には区別すべき型属性が 1 つだけある：線形 vs コピー可能。コンパイラが自動推論する。

### 11.1 Move（デフォルトの所有権移転）

すべての型はデフォルトで Move セマンティクスに従う。代入、引数渡し、戻り値 = 所有権移転。

```yaoxiang
p: Point = Point(1.0, 2.0)
q = p           // Move、p は再読込不可
```

### 11.2 Dup（浅いコピー：ハンドル複製、データ共有）

**Dup 属性は参照/トークン型に使用される**。Dup 型の代入 = 浅いコピー——ハンドル/トークンを複製し、底层のデータは共有する。複数の所有者が同じデータブロックを指す。

| 型               | 属性   | 説明                                                                    |
| ---------------- | ------ | ----------------------------------------------------------------------- |
| `&T`             | Dup    | ゼロサイズ読み取りトークン、トークン複製 = 複数の視点が同じデータを指す |
| `ref T`          | Dup    | Rc/Arc 複製 = 参照カウント+1、ヒープデータを共有                        |
| `&mut T`         | Linear | ゼロサイズ書き込みトークン、独占、コピー不可                            |
| その他すべての型 | Move   | デフォルトの所有権移転                                                  |

**プリミティブ値型**（Int, Float, Bool,
Char）はコンパイラ組み込みの特殊処理：代入時に自動的に値コピーされ、2 つの値は完全に独立する。これはコンパイラのネイティブ動作であり、Dup 型属性には属さない。

```yaoxiang
// &T: Dup、自由なエイリアシング可能
view: &Point = &p
view2 = view     // Dup：トークン複製、両方とも有効
print(view.x)    // 使用可能
print(view2.x)   // 使用可能

// &mut T: Linear、コピー不可
mut_ref: &mut Point = &mut p
// r2 = mut_ref  // ❌ &mut T は Dup ではない、コピー不可
```

### 11.3 Clone（明示的な深いコピー）と Dup の関係

**Clone** は明示的な深いコピーインターフェースである。すべての型が Clone を実装でき、`.clone()`
メソッドを提供する。

```yaoxiang
// Clone インターフェース定義（標準ライブラリ）
Clone: Type = {
    clone: () -> Clone
}

// 使用
p: Point = Point(1.0, 2.0)
backup = p.clone()    // 深いコピー、p はまだ使用可能
p2 = p.clone()        // 複数回クローン可能
```

**Dup と Clone の違い**：

|                    | Dup                                               | Clone                                    |
| ------------------ | ------------------------------------------------- | ---------------------------------------- |
| **セマンティクス** | 浅いコピー：ハンドル/トークン複製、底层データ共有 | 深いコピー：完全独立の副本作成           |
| **呼び出し方式**   | 暗黙的（代入/引数渡しで自動）                     | 明示的（`.clone()`）                     |
| **変更影響**       | 相互に影響（底层データ共有）                      | 互いに影響しない（独立副本）             |
| **適用型**         | `&T` トークン、`ref T`                            | Clone インターフェースを実装する任意の型 |
| **コスト**         | ゼロオーバーヘッド（トークンはゼロサイズ型）      | 型による                                 |

**Dup は Clone を含意せず、Clone は Dup を含意しない**——これらは 2 つの直交する概念である：

```yaoxiang
// Dup 型：トークン複製、底层データ共有
view: &Point = &p
view2 = view        // Dup：トークン複製、両方とも同じ p を指す
print(view.x)       // 使用可能
print(view2.x)      // 使用可能、同じデータが見える

// プリミティブ値型：コンパイラが自動値コピー（Dup ではない）
x: Int = 42
y = x               // 値コピー、x と y は完全に独立
print(x)            // 使用可能

// Clone：明示的な深いコピー、独立副本作成
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone：深いコピー、p はまだ使用可能
r = p               // Move：所有権移転、Point は Dup でもプリミティブ値型でもないため
```

**設計意図**：

- Dup はトークン/参照型に使用され、「同じデータに対する複数の視点」の問題を解決する
- Clone は独立した副本が必要なシナリオに使用され、明示的な呼び出しによってコストを可視化する
- プリミティブ値型（Int/Float/Bool/Char）のコピーはコンパイラの組み込み動作であり、Dup には属さない
- ほとんどのカスタム型はデフォルトで Move、ゼロコピーで高性能

## 第十二章：借用トークン型

### 12.1 核心概念

`&T` と `&mut T`
は**ゼロサイズのコンパイル時トークン型**である。これらは「参照」ではなく、「アクセス権限の型レベル証明」である。

```
&T      →  ゼロサイズ、源データを凍結（期間中 WriteToken 取得を禁止）、
          凍結保証の下で複数の読み取り専用は安全 -> Dup（コピー可能）
&mut T  →  ゼロサイズ、独占読み書き（他のすべてのトークンを禁止）、
          独占アクセス下ではコピーは無意味 -> Linear（Dup ではない）
```

**重要な特性**：

- トークンは**通常の型**であり、他のすべての型と同じスコープ規則に従う
- ライフタイム注釈 `'a` は不要
- 専用の借用チェッカーは不要——型属性（Dup/Linear）が自然に権限を推論する
- コンパイル後に完全に消滅し、ゼロランタイムオーバーヘッド

### 12.2 基本使用

```yaoxiang
// メソッド端：パラメータ型を宣言し、必要な権限を決定
Point.print: (self: &Point) -> Void = {
    print(self.x)               // &Point トークンが読み取り権限を付与
    print(self.y)
}

Point.shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
    self.x = self.x + dx        // &mut Point トークンが書き込み権限を付与
    self.y = self.y + dy
}

// 呼び出し端：コンパイラが自動的に借用または Move を選択
p = Point(1.0, 2.0)
p.print()                       // コンパイラが自動的に &Point トークンを作成
p.shift(1.0, 1.0)               // コンパイラが自動的に &mut Point トークンを作成
p.print()                       // OK、前のトークンは shift 呼び出し終了とともに解放された

// 複数の &T トークン共存——Dup 型は自由なコピーが許可される
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 トークンのスコープと伝播

トークンは通常の型であるため、通常の型のすべての操作をサポートする：

**トークンを返す**——トークンは戻り値と一緒に伝播する：

```yaoxiang
// ✅ サブトークンと親トークンを一緒に返す
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // トークンは呼び出し元に返される
print(px_ref)                    // OK、トークンはまだスコープ内
```

**構造体に格納**——構造体はトークンフィールドを携带できる：

```yaoxiang
// ✅ 構造体がトークンをフィールドとして携带
Window: Type = {
    target: Point,
    view: &Point,              // トークンフィールド——target への読み取り専用ビューを保持
}
```

**クロージャはキャプチャしない、コンテキストは作成時点で固定される**——クロージャは自分のパラメータのみを消費し、外部データが必要な場合はカリー化を通じて作成時点で値をクロージャに固定する：

```yaoxiang
// ✅ コンテキストがカリー化により固定される：threshold はパラメータ、gt_point(threshold) は作成時点で値をクロージャに固定
gt_point: (t: Float) -> (p: Point) -> Bool = (p) => p.x > t
filter_by_threshold: (items: List(Point), threshold: Float) -> List(Point) = {
    items.filter(gt_point(threshold))
}
```

> 注：クロージャ（関数値）がエスケープした後、その定義箇所のスコープはすでに死んでいる可能性があるため、外側の変数を暗黙にキャプチャしてはならない。ただし、呼び出し点（作成点）のスコープは必ず生存しており、その時点でコンテキストが値として固定されてクロージャに入ることは安全である。

### 12.4 自動借用選択

呼び出し端で、コンパイラは以下の優先順位に従って自動的に選択する：

```
1. 実引数が後にまだ使用される -> トークン作成を優先（メソッドシグネチャに応じて &T または &mut T）
2. 実引数が後に使用されない -> Move
3. 優先マッチング順序：&T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // print のパラメータ型は &Point -> コンパイラが &Point トークンを作成
p.shift(1.0, 1.0)  // shift のパラメータ型は &mut Point -> コンパイラが &mut Point トークンを作成
p2 = p             // 後は使用されない -> Move
```

**メソッドレシーバはシグネチャセマンティクスに従う**（RFC-011a のレシーバスペル约定と同じ）：レシーバが
`&T` -> 読み取り専用借用トークン；`&mut T` -> 可変借用トークン；値 ->
Move（レシーバ消費）。呼び出し点で生成された借用トークンは呼び出し終了とともに解放される（transient、§12.5 区間セマンティクス）；インターフェースの借用レシーバはインターフェース作者が
`&Self` と明示的に宣言し、impl シグネチャは `Self ↦ impl 型`
置換後にインターフェースと完全に一致する必要がある（RFC-011a §3）。

### 12.5 トークン衝突検出

トークン衝突検出は**借用 Hoare 命題**（RFC-009a）であり、独立したフロー依存分析ではない。コンパイラは借用命題（`borrow_conflict`/`use_after_move`/`use_after_drop`/`mut_violation`）を自動生成して証明パイプラインに送って検証する；トークン活性は区間
`[created_at, last_use]` である（RFC-009a §逆 BFS 活性分析を参照）：

```yaoxiang
// ❌ &mut と派生した &T は同時にアクティブになれない
bad_alias: (p: &mut Point) -> Void = {
    p.x = 10.0                   // ✅ WriteToken を正常使用
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
alias_bad(p, p)                  // ❌ p 同時に &mut と & トークンを派生
```

### 12.6 コンパイラ内部：ブランド機構

ユーザーはブランドに一切触れない。コンパイラは内部で各トークンにコンパイル時一意識別子を割り当てる：

```
ユーザーが見るもの        コンパイラ内部表現
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N はコンパイル時一意整数
&mut Point     →  WriteToken(Point, #M)   // #M はコンパイル時一意整数
```

ブランドの用途：

- **偽造防止**：トークンは所有者カプセルからのみ取得でき、凭空に構築できない
- **関連追跡**：フィールドアクセスから派生した `&Float`
  は派生ブランド（`#N.field_x`）を携带し、コンパイラは親トークンまで追跡可能
- **衝突検出**：同源 WriteToken と派生 ReadToken は同時にアクティブになれない

ブランドは単態化とインライン化後に完全に消滅し、生成された機械語には存在しない。**ゼロランタイムオーバーヘッド。**

### 12.7 トークン Sum 型

```
&BorrowToken ::= &T          // ReadToken（源データ凍結 -> Dup 安全）
               | &mut T      // WriteToken（独占読み書き -> Linear）
```

### 12.8 借用トークン vs ref

|            | `&T` / `&mut T`                                      | `ref`                                 |
| ---------- | ---------------------------------------------------- | ------------------------------------- |
| 做什么     | ちょっと見る/その場で変更                            | 共有保持                              |
| 範囲       | トークン値のスコープに従う                           | スコープをまたぐ                      |
| コスト     | ゼロオーバーヘッド（ゼロサイズ型、コンパイル後消滅） | Rc または Arc（コンパイラが選択）     |
| エスケープ | 可（トークンは戻り値/構造体で伝播）                  | そもそもエスケープ用                  |
| タスク間   | 不可（トークンはタスク間転送未実装）                 | 可（コンパイラが自動的に Arc を選択） |
| 環検出     | 関与しない                                           | タスク内は静かに、タスク間は lint     |

> 注（未定義）：ref 作成後に内容をどう読むか（参照解除/メソッド/自動）はまだ仕様で定義されておらず、実装現状では
> `*a` は E1052 を報告する。定義後に本節を補足する。

---

## 付録：型定義クイックリファレンス

### A.1 型定義

```
// === レコード型（波括弧） ===

// レコード型
Point: Type = { x: Float, y: Float }

// バリアント付きレコード型（関数フィールドを使用）
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// === インターフェース型（波括弧、全フィールドが関数）===

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

// コンパイル時ジェネリクス：N が型位置 (k: N) で参照される -> コンパイル時値パラメータ
factorial: (N: Int)(k: N) -> Int = { ... }
Measure: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }

// 条件型
If: (C: Bool, T: Type, E: Type) -> Type = match C { True => T, False => E }

// 関数特化
sum: (arr: Array(Int)) -> Int = { ... }
sum: (arr: Array(Float)) -> Float = { ... }
```

### A.3 型属性クイックリファレンス

```
// === Move（デフォルト）===
// すべての型はデフォルトで Move。代入、引数渡し、戻り値 = 所有権移転

// === プリミティブ値型（コンパイラ組み込み）===
Int, Float,     // 代入時に自動値コピー、2 つの値は完全に独立
Bool, Char      // Dup ではなく、コンパイラのプリミティブ組み込み処理

// === Dup（浅いコピー：ハンドル複製、底层データ共有）===
&T              // ゼロサイズ読み取りトークン、トークン複製 = 複数の視点が同じデータを指す
ref T           // Rc/Arc 複製 = 参照カウント+1、ヒープデータを共有

// === Linear ===
&mut T          // ゼロサイズ書き込みトークン、Linear（独占、コピー不可）

// === Clone（明示的な深いコピー）===
value.clone()   // 独立副本を作成、変更は元の値に影響しない
```

### A.4 借用トークンクイックリファレンス

```
// === 借用トークン ===
&T              // ゼロサイズコンパイル時読み取りトークン、源データ凍結 -> Dup（コピー可能）
&mut T          // ゼロサイズコンパイル時書き込みトークン、独占読み書き -> Linear（コピー不可）

// 呼び出し端の自動選択
// 1. 実引数が後にまだ使用される -> トークン作成
// 2. 実引数が後に使用されない -> Move
// 3. 優先マッチング：&T < &mut T < Move

// トークン伝播
// ✅ 戻り値として、構造体に格納、クロージャでキャプチャ可能
// ❌ タスク間は不可（トークンはタスク間転送未実装）
```

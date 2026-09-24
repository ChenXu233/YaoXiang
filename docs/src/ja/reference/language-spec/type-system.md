# 型システム仕様

本文書は YaoXiang プログラミング言語の型システム仕様を定義する。基本的な型、複合型、ジェネリクス、trait を含む。

---

## 第零章：理論的基礎

### 0.1 Curry-Howard 同型

Curry-Howard 同型（Curry-Howard
correspondence）は YaoXiang 型システムの理論的基礎である。これはプログラミング言語の型システムと数理論理学の間の深層対応を明らかにする。

| 論理学                         | プログラミング言語                    |
| ------------------------------ | ------------------------------------- |
| 命題 \(P\)                     | 型 `Type`                             |
| 証明 \(p: P\)                  | プログラム `x: T = ...`               |
| 含意 \(P \rightarrow Q\)       | 関数型 `(P) -> Q`                     |
| 連言 \(P \wedge Q\)            | 積型 `{ a: P, b: Q }`                 |
| 選言 \(P \vee Q\)              | 和型 `{ a(P) \| b(Q) }`               |
| 全称量化 \(\forall x:T. P(x)\) | ジェネリクス `(T: Type) -> ...`       |
| 真 \(\top\)                    | `Void`（Unit、デフォルト値を持つ）    |
| 偽 \(\bot\)                    | `Never`（零コンストラクタ、居住不能） |
| 型宇宙 \(Type_n : Type_{n+1}\) | 宇宙階層（Russell 悖論を防ぐ）        |
| case 分析                      | 型レベル `match`                      |

> **注意**：型レベル `match` は分類討論（case
> analysis）であり、数学的帰納法ではない。帰納法は型レベル再帰関数 + コンパイラの停止性チェックを必要とする。

### 0.2 型は命題、プログラムは証明

YaoXiang において、この対応関係は設計の第一級の原則である：

- **停止する型レベル計算は正しい構成的証明に対応する**。YaoXiang の型族（例：`Nat` 上の `Add`
  の case 分析 + 再帰呼び出し）は本質的に数学的帰納法の型レベルエンコーディングである——ただし、コンパイラが停止性チェックを行えることが前提となる。
- **型検査は証明の検証である**。あるプログラムが型検査を通過するということは、論理命題が構成的に証明されたことと等価である。

### 0.3 言語設計への影響

Curry-Howard 同型の YaoXiang における具体的な体現：

1. **宇宙階層**（RFC-010）：`Type₀ : Type₁ : Type₂ …` は `Type: Type`
   に起因する論理的矛盾（Girard パラドックス）を回避する
2. **型族**（RFC-011）：自然数 `Nat(Zero/Succ)`
   の型レベル case 分析 + 再帰呼び出しは Peano 公理に対応する——ただし、コンパイラが停止性チェックを行うことが前提
3. **条件型**（RFC-011）：`If: (C: Bool, T: Type, E: Type) -> Type` は論理の case 選言に対応する
4. **値依存型**（RFC-011）：`Array: (T: Type, N: Int) -> Type`
   は「各整数 N に対して型が存在する」という有量化に対応する

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

> **設計説明**：RFC-010 は「すべて代入である」という統一モデル（`name: type = value`）を提案しているが、文法レベルでは型と値は依然として区別される必要がある。コンパイラ実装において
> `Type` と `Expr` は二つの独立した AST 列挙型（`ast.rs:406` と `ast.rs:25`）であり、`TypeExpr`
> は BNF プレースホルダとして実装の `Type` 列挙型に対応し、「この位置に型が期待される」ことを表す。

---

## 第二章：基本型

### 2.1 プリミティブ型

| 型       | 論理的対応   | 説明                                                                                        | デフォルトサイズ |
| -------- | ------------ | ------------------------------------------------------------------------------------------- | ---------------- |
| `Type`   | —            | メタ型                                                                                      | 0 バイト         |
| `Never`  | ⊥（偽/空型） | 零コンストラクタ、いかなる値も持たない。発散/panic の戻り型。`Never <: T` は任意の T で成立 | 0 バイト         |
| `Void`   | ⊤（真/Unit） | デフォルト void 値を持つ零フィールド積型。`x: Void = <デフォルト>` は合法                   | 0 バイト         |
| `Bool`   | —            | ブール値：`true` / `false`                                                                  | 1 バイト         |
| `Int`    | —            | 符号付き整数                                                                                | 8 バイト         |
| `Uint`   | —            | 符号なし整数                                                                                | 8 バイト         |
| `Float`  | —            | 浮動小数点数                                                                                | 8 バイト         |
| `String` | —            | UTF-8 文字列                                                                                | 可変             |
| `Char`   | —            | Unicode 文字                                                                                | 4 バイト         |
| `Bytes`  | —            | 生バイト                                                                                    | 可変             |

ビット幅付き整数：`Int8`, `Int16`, `Int32`, `Int64`, `Int128`。ビット幅付き浮動小数点：`Float32`,
`Float64`

### 2.2 Never と Void：⊥ と ⊤

`Never` と `Void` は型システムの論理的プリミティブであり、偽（⊥）と真（⊤）にそれぞれ対応する。

**Never（⊥、偽/空型）** — 譲歩できない三つの性質：

1. **零コンストラクタ**：`Never` 型の値を生成できるリテラルや式は存在しない。`x: Never = ...`
   の右辺には何も書けない。
2. **爆発原理**：`Never <: T` は任意の型 `T` に対して成立する。`assert(false)` は `Never`
   を返し、その後のコードは型検査を通過できる（実際には決して実行されないが）。
3. **発散マーカー**：`f: (...) -> Never` は `f`
   が返らないことを保証する。コンパイラはこれに基づき dead code 分析と `match` 分岐合流を行う。

`Never` は組み込み型名（`Int`/`Bool` と同じ登録パス）であり、キーワードではない。

**Void（⊤、真/Unit）** — ちょうど一つの居住者（デフォルト void 値）を持つ。`Void`
は零フィールド積型の単位元である。`x: Void = <デフォルト>`
は合法。ブロックの値は**末尾式**によって与えられる（空ブロック `{}` は `Void`）、詳細は
[RFC-010a](../../design/rfc/accepted/010a-tail-expression-and-return.md) を参照。

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

**規則**：

- レコード型は波括弧 `{}` で定義する
- フィールド名の直後にコロンと型を続ける
- インターフェース名は型本体内に記述することでそのインターフェースの実装を表す

> **名前空間の所属**：`Type.name` 接頭辞（例：`Point.draw`）は関数が `Point`
> の名前空間に属することを示す。これは暗黙のバインディングを引き起こさない。`p.draw()` のような `.`
> 呼び出し構文を有効にするには、明示的なバインディングが必要である：`Point.draw = draw[0]`。詳細は RFC-004 および RFC-010 を参照。

#### 3.1.1 フィールドのデフォルト値

型フィールドはデフォルト値を指定でき、構築時には任意で提供できる：

```yaoxiang
// デフォルト値を持つフィールド - 構築時には任意
Point: Type = {
    x: Float = 0,
    y: Float = 0
}

// 使用
Point()           // -> Point(x=0, y=0)
Point(x=1)       // -> Point(x=1, y=0)
Point(x=1, y=2) // -> Point(x=1, y=2)

// デフォルト値を持たないフィールド - 構築時には必須
Point2: Type = {
    x: Float,
    y: Float
}

// 使用
Point2(x=1, y=2) // 正しい
Point2()          // エラー
```

**規則**：

- `field: Type = expression` -> デフォルト値を持つ、構築時には任意
- `field: Type` -> デフォルト値を持たない、構築時には必須

#### 3.1.2 組み込みバインディング

型定義本体内で直接メソッドをバインドできる：

```yaoxiang
// 方式1：外部関数の参照によるバインディング
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]    // 位置0にバインド
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

**インターフェースへの直接代入**：具象型はインターフェース型変数に直接代入できる（構造的サブタイピング）

```yaoxiang
// 直接代入（具象型をコンパイル時に決定可能 -> ゼロオーバーヘッド呼び出し）
d: Drawable = Circle(1)
d.draw(screen)        // コンパイル後：circle_draw を直接呼び出す、vtable なし

// 関数の戻り値（コンパイル時に決定不能 -> vtable 呼び出し）
d: Drawable = get_shape()
d.draw(screen)        // vtable 経由でメソッドを検索

// 関数の引数としてのインターフェース
process: (d: Drawable) -> Void = d.draw(screen)
```

**コンパイル時最適化戦略**：

| シナリオ         | 推論結果         | 呼び出し方式                       |
| ---------------- | ---------------- | ---------------------------------- |
| 具象型の直接代入 | 具象型を決定可能 | 直接呼び出し（ゼロオーバーヘッド） |
| 関数の戻り値     | 不明             | vtable                             |
| 異種コレクション | 複数の型         | vtable                             |

**コヒーレンスとオーファンルール（不適用、収束説明）**：YaoXiang のインターフェースは構造的型（インターフェース = フィールドがすべて関数型のレコード）であり、名目的な trait ではない——crate/モジュール横断の「誰が誰に対して実装できる」という帰属問題は存在せず、Rust 式のオーファンルールとコヒーレンスチェックは適用対象を持たない（裁決記録は RFC-011
§2.1）。構造的世界の対応する保障は**重複実装の拒否**である：同一メソッドシグネチャの型上での重複定義はコンパイルエラーとなる（RFC-011a
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

ジェネリクスパラメータは関数型の一部であり、通常のパラメータと統一的に `()` 構文を使用する：

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

コンテナ型はジェネリック型コンストラクタであり、組み込みプリミティブではない——ユーザー定義ジェネリクスと同じ扱いを受け、統一されたジェネリクスインスタンス化パスを経由する。長さ情報の帰属が三つのコンテナ概念の根本的な違いである：

| 型            | 長さ     | セマンティクス                     | 基底                                        |
| ------------- | -------- | ---------------------------------- | ------------------------------------------- |
| `Array(T, N)` | 型       | 固定長配列（const ジェネリクス N） | コアプリミティブ（スタック/インライン優先） |
| `Vec(T)`      | 実行時値 | 実行時長さの生バッファ、伸長可能   | コアプリミティブ（ヒープ上連続バッファ）    |
| `List(T)`     | 実行時値 | 標準ライブラリ型（伸長可能リスト） | ライブラリ：`{ data: Vec(T), length: Int }` |
| `Dict(K, V)`  | 実行時値 | キーバリュー写像                   | `HeapValue::Dict`                           |

> `List(T)` は**標準ライブラリ型であり、コンパイラプリミティブではない**：YaoXiang 自身によって
> `std.list`
> 内で定義され、ユーザー定義ジェネリクスのレコードと同じ扱いを受ける。伸長可能セマンティクスの全戦略（いつ拡張するか、どの程度拡張するか、共有可能か）はライブラリ内にあり、コンパイラは関与しない。`Vec(T)`
> はそれが依存する最小基盤プリミティブである。
>
> Set(T) は除名済み：リテラルなし、実行時表現なし、std.set なし。要件が発生した際に Dict パターンに従って補完する。

主要な規則：

- **リテラルの振り分けは文脈が決定する**：`[...]` 生リテラルと `List(T)`
  注釈は伸長可能リストに振り分けられる；`Array(T, N)`
  注釈がリテラルに直接作用する場合は固定長配列に振り分けられる。振り分け検証：要素数 ==
  N、要素型が T と互換、不一致はコンパイル時 E1002；N が記号定数（const パラメータ）の場合、個数検証は型精化フェーズまで延期される。
- **暗黙の List→Array 変換禁止**：固定長性は型層で保証される——push は `List(A)`
  レシーバのみ受け付ける。
- **パフォーマンス階層**：底から上に向かってパフォーマンスは減少し、柔軟性は増加する：`Array` >
  `Vec` > `List`。
- **インデックス失敗契約**（実行時レポートは過渡的、目標状態はコンパイル時精化カバレッジ、値依存型経由、§8.4 参照）：
  - インデックス範囲外（負のインデックスを含む）→ `E6003`
  - Dict キー欠落 → `E6008`
- **membership `in` 述語**：`Bool`
  を返しエラーを発生させない、右オペランドは List/Array/Dict(キー)/Tuple/String/Range をカバー。第一級ホーア述語であり、精化型のコンパイル時証明可能命題の基底である。

ジェネリック関数では、型パラメータも同様にシグネチャで宣言され、コンパイラが実引数から自動推論する：

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
    push: (self: List(T), item: T) -> Void,   // self は単なる慣例名であり、キーワードではない
    get: (self: List(T), index: Int) -> Option(T)
}
```

### 4.3 ジェネリック構築呼び出しと型推論

ジェネリック型定義のフィールドリストは**コンストラクタを自動生成する**：各フィールドが構築パラメータに対応し、フィールド名がパラメータ名となる；デフォルト値を持つフィールドは構築時に省略可能で、デフォルト値を持たないフィールドは必須である。関数型フィールド（メソッド）は構築パラメータを生成しない。

```yaoxiang
// 型定義
Container: (T: Type) -> Type = {
    value: T,        // デフォルト値なし → 構築パラメータ必須
    extra: T,
}
// 自動展開された完全形式（コンパイラ内部ビュー、ユーザーが手書きする必要なし）：
// Container: (T: Type) -> (value: T, extra: T) -> Type = {
//     value: T = value,
//     extra: T = extra,
// }

// 呼び出し：自動生成されたコンストラクタを呼び出す
c  = Container(42, 43)            // 構築パラメータはフィールド順に格納；T は要素から自動展開 = Int
c2 = Container("a", "b")          // T = String
c3 = Container(Int)(42, 43)       // 明示的な型パラメータ + 位置式構築パラメータ
c4 = Container(Int)(extra=43, value=42)  // フィールド名式、順序任意
c5 = Container(Int)()             // 空構築：フィールドはデフォルト値/零値を取得（データは事後代入）

// フィールドデフォルト値 → 構築パラメータは省略可能
Point: (T: Type) -> Type = { x: T = 0, y: T = 0 }
p  = Point(1.5, 2.5)              // T = Float、x←1.5, y←2.5
p2 = Point(Int)()                 // x=0, y=0
```

**呼び出し規則**（単一括弧、宣言パラメータに位置ごとに一致、左から右へ）：

1. 実引数を位置ごとに型宣言パラメータとの照合を試みる：`Type`
   の位置は型実引数を受け入れ、コンパイル時値パラメータの位置（例：`Int`）はコンパイル時定数を受け入れる。
2. コンパイル時値パラメータ位置の一致が成功した場合（部分一致）、型構築として処理する：すべてのパラメータ位置を順にチェックし、エラー時は宣言順序に従って**最初に一致しない/欠落しているパラメータを報告する**。
3. 実引数が宣言パラメータに完全に対応しない（すべて値、コンパイル時値パラメータ位置に一致するものがない）場合、構築パラメータとして処理する：位置式はフィールド順に格納、型パラメータは要素型から自動展開する。

```yaoxiang
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),
    data: Array(Array(T, Cols), Rows),
}

m: Matrix(Int, 3, 4)              // 型位置：一層型構築
m2 = Matrix(Int, 3, 4)(data=[[1,2,3,4],[5,6,7,8],[9,10,11,12]])  // 二層：型 + 構築パラメータ
m3 = Matrix(Int, 3, 4)()          // 空構築（RFC-011 §9.3 パターン、データは事後代入）

Matrix(42)    // ❌ 位置0: T←42 不一致（42 は型ではない）；位置1: Rows←42 一致；
              //    位置2: Cols 欠落 → 最初のエラーを先に報告：T は Type を期待、42 が見つかった
Container(42) // ❌ 構築パラメータ extra 欠落
Container(42, 43, 44)  // ❌ 構築パラメータ超過
```

**型推論**：ジェネリック型コンストラクタの型パラメータは構築パラメータ要素から自動展開される（`Container(42, 43)`
→ T=Int）；ジェネリック関数の型パラメータは実引数型から自動展開される（`map(numbers, f)` → T=Int,
R=String、§4.1 参照）。展開できない場合は明示的に指定する必要がある。

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

> **制約の解決ソース（RFC-011b）**：演算子制約名（`Add` / `Subtract` / `Multiply` / `Divide` /
> `Modulo` / `Equal` / `Index`）の解決 = インターフェース実装登録表の検索——`T: Add` ≜ `Add(T, T, T)`
> インスタンス化が登録済み；`Equal`
> には別途構造推論がある（全フィールドが比較可能なレコードは自動比較可能）。`Zero` / `One` /
> `PartialOrd` などの名前は未定義ソースであり、宙ぶらりん制約名である。

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

### 7.1 コンパイル時値パラメータ

```
LiteralType   ::= Identifier ':' Int          // コンパイル時定数（候補）
```

> 判断基準は**型位置で参照されるかどうか**であり、「具体的な型で注釈されているか」ではない：`add: (a: Int, b: Int) -> Int = a + b`
> において `a`/`b` は実行時値パラメータである（どちらも型位置に現れない）。

**用語**：`Type`
以外の具体的な型（例：`Int`）で注釈されたジェネリクスパラメータは**コンパイル時値パラメータ候補**と呼ばれ、コンパイル時値パラメータになるかどうかは、その値が型位置で参照されるかどうか（値依存）によって決まる。**`const`
キーワードは不要**（実装内部ではかつて「const ジェネリクス」という用語を使用していたが、ドキュメントでは統一して「コンパイル時値パラメータ」を使用する）。

**判定規則（二段階）**：

1. **形態粗選別**：パラメータが `Type`
   以外の具体的な型（`Int`/`Bool`/`Float`）で注釈されている → 候補。
2. **用途精選別**：候補名が**型位置**（型本体のフィールド型、内側 `Fn` パラメータ型、`Assert`
   述語、`Array(T, N)`
   型構築実引数位置）に現れる → 真のコンパイル時値パラメータ；そうでなければ**実行時値パラメータ**。

| 書き方                                                     | 判定                       | 理由                         |
| ---------------------------------------------------------- | -------------------------- | ---------------------------- |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b 実行時値パラメータ     | 値位置にのみ出現             |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N コンパイル時値パラメータ | N が型構築実引数位置に出現   |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N コンパイル時値パラメータ | N が内側パラメータ k の型    |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N 落空→実行時値パラメータ  | N が型本体で参照されていない |

**中核設計**：`(N: Int)` コンパイル時値パラメータ + `(k: N)`
値パラメータを用いて、コンパイル時定数と実行時値を区別する。落空候補（形態は候補だが用途が命中しない）は実行時値パラメータに退化する——関数レベルと型コンストラクタパスの両方がこの規則に従う。

```yaoxiang
// コンパイル時値パラメータ：N が型位置（Array 長さスロット）で参照される
Measure: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // N が型構築実引数位置に出現 → コンパイル時値パラメータ
    length: N
}

// 使用方法：factorial(5) は型位置で評価（コンパイル時）され、結果 120 が型に埋め込まれる
arr: Measure(Int, factorial(5))  // コンパイラはコンパイル時に factorial(5) = 120 を計算

// 値依存：N は内側パラメータ k の型として機能
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

### 8.3 Assert 精化型と assert 命題

`assert` と `Assert`
は同じ精化プリミティブの二面であり、dispatch 分派パイプラインが「述語の自由変数がコンパイル時に到達可能か」に基づき自動選択する。

**中核シグネチャ**：`assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**dispatch 分派規則**：

| 判定基準                                                                       | モード      | 振る舞い                                                                           |
| ------------------------------------------------------------------------------ | ----------- | ---------------------------------------------------------------------------------- |
| すべての自由変数がコンパイル時既知（ジェネリクスパラメータ、コンパイル時定数） | CompileTime | 証明パイプラインへ：true → Void に消去、false → コンパイルエラー（Never 居住不能） |
| 実行時自由変数が存在（関数パラメータ、外部入力）                               | Runtime     | 実行時 Bool チェックを挿入し、フロー感度仮定集合 Γ に精化事実を注入                |

**フロー感度仮定集合 Γ**：

コンパイラは各制御フロー点の既知命題集合を保持する：

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP 伝播
mut x = x - 5       // Γ = {}  ← mut kill set：旧仮定失効
```

`mut` 変数代入後、当該変数に関するすべての仮定が削除される（kill
set）。分岐合流時 Γ は各分岐の交差を取る。

### 8.4 Terminates：停止測度述語

`Terminates` は**組み込み述語**であり、`Int`、`Never`
と同じくコアプリミティブに属する（組み込み名、キーワードではない）。これは**測度**を一つの計算にバインドし、当該計算の停止を宣言し、停止の証人を与える。

**形態**：二つのアリティ、同じ述語：

| 形態                    | アンカー             | 用途                                                                                                               |
| ----------------------- | -------------------- | ------------------------------------------------------------------------------------------------------------------ |
| `Terminates(m)`         | 所在バインディング名 | デフォルト形態——自己再帰関数、ループ                                                                               |
| `Terminates(FnType, m)` | 明示的関数型         | 測度の帰属を明示的に指定する必要がある場合（測度が他所で定義されている、同じ測度が複数の計算にサービスを提供する） |

```yaoxiang
// 測度：通常関数、単体テスト可能、再利用可能、実行時には参加しない
gcd_measure: (a: Int, b: Int) -> Int = { b }

// 二項形態：測度が他所で定義されている、帰属を明示的に指定
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

// 一項形態：アンカーがバインディング名、測度はスコープ内の式
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n {
        i = i + 1
    }
    return acc
}
```

**セマンティクス**：`Terminates(m)`
はそれが型位置に注釈したその計算の値型を精化する。義務はその計算にかかる——関数は各再帰呼び出し点、ループは backedge——いずれも「次の状態の測度は現在の状態の測度より厳密に小さい」であり、その点のパスガード下で判定される。

呼び出し点の義務は `m(callee_args) < m(caller_args)`；backedge の義務は
`m(次のラウンド) < m(現在のラウンド)`。両者の形式は同一である。

> **なぜループもカバーするか**：ループは匿名構造体であり、通常は指示できない。バインディング名が名前である——`acc: Terminates(n - i) = while ...`
> における `acc` がアンカーを提供し、ループは指示可能になる。これが `Terminates`
> の一項形態がループに作用する理由である。

**測度**：戻り型を制限しない（自然数を強制しない）；その上の「厳密減少」は当該型で利用可能な整列順序によって与えられる。測度が well-founded であるか（`Int`
を返す場合に `>= 0`
か）は**独立した義務**であり、減少義務と同じくコンパイル時証明パイプラインが判定する。

**トリガー**：停止チェックは**精化型**によってトリガーされる——型が精化され次第、検証モードに入る。精化されていない通常の型（例：生の
`while` ループ、精化シグネチャなしの関数）は検証モードに入らず、停止義務を生成しない。

**自動探索優先**：コンパイラはまず測度を自動探索（線形ランク関数、述語違反カウント、有界増減、乗法的スケールの四つのテンプレート）し、探索できない場合に限り明示的な
`Terminates` が必要となる。

**実行時表現**：純粋なコンパイル時エンティティであり、witness と共に消去され、実行時バイナリには参加しない。

> 完全な設計については
> [RFC-027 §6.9](../../design/rfc/accepted/027-compile-time-evaluation-types.md)（セマンティクス）と
> [RFC-027a](../../design/rfc/review/027a-termination-explicit-measure.md)（実装機構）を参照。

---

## 第九章：型の和と交差

### 9.1 型の和集合

```
TypeUnion     ::= TypeExpr '|' TypeExpr
```

### 9.2 型の交差

```
TypeIntersection ::= TypeExpr '&' TypeExpr
```

**構文**：型交差 `A & B` は A と B を同時に満たす型を表す

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
// プラットフォーム型列挙（標準ライブラリで定義）
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

YaoXiang には区別が必要な型属性が一つだけある：線形 vs 複製可能。コンパイラが自動推論する。

### 11.1 Move（デフォルトの所有権移転）

すべての型はデフォルトで Move セマンティクスに従う。代入、引数渡し、戻り値 = 所有権移転。

```yaoxiang
p: Point = Point(1.0, 2.0)
q = p           // Move、p はもう読み取れない
```

### 11.2 Dup（浅いコピー：ハンドルのコピー、データの共有）

**Dup 属性は参照/トークン型に使用される**。Dup 型の代入 = 浅いコピー——ハンドル/トークンをコピーし、底层データが共有される。複数の所有者が同じデータブロックを指す。

| 型               | 属性   | 説明                                                                    |
| ---------------- | ------ | ----------------------------------------------------------------------- |
| `&T`             | Dup    | 零サイズ読み取りトークン、トークンのコピー = 同じデータへの複数のビュー |
| `ref T`          | Dup    | Rc/Arc コピー = 参照カウント+1、ヒープデータを共有                      |
| `&mut T`         | Linear | 零サイズ書き込みトークン、独占、複製不可                                |
| その他すべての型 | Move   | デフォルトの所有権移転                                                  |

**プリミティブ値型**（Int, Float, Bool,
Char）はコンパイラ組み込みの特別な処理である：代入時に自動的に値コピーされ、二つの値は完全に独立する。これはコンパイラのネイティブ動作であり、Dup 型属性には属さない。

```yaoxiang
// &T: Dup、自由にエイリアス可能
view: &Point = &p
view2 = view     // Dup：トークンをコピー、どちらも有効
print(view.x)    // 使用可能
print(view2.x)   // 使用可能

// &mut T: Linear、複製不可
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

|                    | Dup                                                     | Clone                                    |
| ------------------ | ------------------------------------------------------- | ---------------------------------------- |
| **セマンティクス** | 浅いコピー：ハンドル/トークンをコピー、底层データが共有 | 深いコピー：完全独立の副本を作成         |
| **呼び出し方式**   | 暗黙的（代入/引数渡しで自動）                           | 明示的（`.clone()`）                     |
| **変更の影響**     | 相互に影響（底层データを共有）                          | 相互に影響しない（独立副本）             |
| **適用型**         | `&T` トークン、`ref T`                                  | Clone インターフェースを実装する任意の型 |
| **コスト**         | ゼロオーバーヘッド（トークンは零サイズ型）              | 型による                                 |

**Dup は Clone を含意せず、Clone も Dup を含意しない**——これらは二つの直交する概念である：

```yaoxiang
// Dup 型：トークンをコピー、底层データが共有
view: &Point = &p
view2 = view        // Dup：トークンをコピー、どちらも同じ p を指す
print(view.x)       // 使用可能
print(view2.x)      // 使用可能、同じデータが見える

// プリミティブ値型：コンパイラが自動的に値コピー（Dup ではない）
x: Int = 42
y = x               // 値コピー、x と y は完全に独立
print(x)            // 使用可能

// Clone：明示的な深いコピー、独立副本を作成
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone：深いコピー、p はまだ使用可能
r = p               // Move：所有権移転、Point は Dup でもプリミティブ値型でもない
```

**設計意図**：

- Dup はトークン/参照型に使用され、「同じデータに対する複数のビュー」という問題を解決する
- Clone は独立副本が必要なシナリオに使用され、明示的な呼び出しによりコストを可視化する
- プリミティブ値型（Int/Float/Bool/Char）のコピーはコンパイラの組み込み動作であり、Dup には属さない
- ほとんどのユーザー定義型はデフォルトで Move、ゼロコピーで高性能

## 第十二章：借用トークン型

### 12.1 中核概念

`&T` と `&mut T`
は**零サイズのコンパイル時トークン型**である。これらは「参照」ではなく、「アクセス権限の型レベル証明」である。

```
&T      →  零サイズ、ソースデータを凍結（期間中 WriteToken 取得禁止）、
          凍結保証下で複数読み取りが安全 → Dup（複製可能）
&mut T  →  零サイズ、排他的読み書き（他のトークンすべて禁止）、
          排他アクセス下ではコピーが無意味 → Linear（非 Dup）
```

**主要な特性**：

- トークンは**通常の型**であり、他のすべての型と同じスコープ規則に従う
- ライフタイム注釈 `'a` は不要
- 専用の借用チェッカーは不要——型属性（Dup/Linear）が自然に権限を推論する
- コンパイル後完全に消滅、ゼロ実行時オーバーヘッド

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
p.print()                       // OK、前のトークンは shift 呼び出し終了とともに解放済み

// 複数の &T トークンが共存——Dup 型は自由にコピー可能
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
(px_ref, p) = p.get_x()        // トークンが呼び出し元に返される
print(px_ref)                    // OK、トークンはまだスコープ内
```

**構造体に格納**——構造体はトークンフィールドを保持できる：

```yaoxiang
// ✅ 構造体がトークンをフィールドとして保持
Window: Type = {
    target: Point,
    view: &Point,              // トークンフィールド——target への読み取り専用ビューを保持
}
```

**クロージャはキャプチャしない、コンテキストは作成時点で固定**——クロージャは自分のパラメータのみを取り込み、外部データが必要な場合はカリー化を通じて作成時点で値をクロージャに固定する：

```yaoxiang
// ✅ コンテキストがカリー化によって固定：threshold はパラメータ、gt_point(threshold) は作成時点で値をクロージャに固定
gt_point: (t: Float) -> (p: Point) -> Bool = (p) => p.x > t
filter_by_threshold: (items: List(Point), threshold: Float) -> List(Point) = {
    items.filter(gt_point(threshold))
}
```

> 注：クロージャ（関数値）がエスケープした後、その定義箇所のスコープは既に死んでいる可能性があるため、外側の変数を暗黙的にキャプチャしてはならない；しかし呼び出し点（作成点）のスコープは必ず生存しており、コンテキストはその時点で値としてクロージャに固定されるのが安全である。

### 12.4 自動借用選択

呼び出し側コンパイラは以下の優先順位で自動選択する：

```
1. 実引数が後に使用される場合 → トークン作成を優先（&T または &mut T、メソッドシグネチャによる）
2. 実引数がもう使用されない場合 → Move
3. 優先一致順序：&T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // print のパラメータ型は &Point → コンパイラが &Point トークンを作成
p.shift(1.0, 1.0)  // shift のパラメータ型は &mut Point → コンパイラが &mut Point トークンを作成
p2 = p             // 後は使用されない → Move
```

**メソッドレシーバはシグネチャセマンティクスに従う**（RFC-011a レシーバスペリング規約と同じ）：レシーバが
`&T` → 読み取り借用トークン；`&mut T` → 可変借用トークン；値渡し →
Move（レシーバを消費）。呼び出し点で生成された借用トークンは呼び出し終了とともに解放される（transient、§12.5 インターバルセマンティクス）；インターフェースの借用レシーバはインターフェース作者が
`&Self` を明示的に宣言し、impl シグネチャは `Self ↦ impl 型`
置換後にインターフェースと完全一致する必要がある（RFC-011a §3）。

### 12.5 トークン競合検出

トークン競合検出は**借用ホーア命題**（RFC-009a）であり、独立したフロー感度分析ではない。コンパイラが借用命題を自動生成（`borrow_conflict`/`use_after_move`/`use_after_drop`/`mut_violation`）し、証明パイプラインに送って検証する；トークン活性はインターバル
`[created_at, last_use]` である（RFC-009a §逆 BFS 活性分析 参照）：

```yaoxiang
// ❌ &mut と派生した &T は同時にアクティブにできない
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

// ❌ 同じ実引数から &mut トークンと他のトークンを同時に作成できない
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p から &mut と & トークンを同時に派生
```

### 12.6 コンパイラ内部：ブランド機構

ユーザーはブランドに一切触れない。コンパイラは内部で各トークンにコンパイル時一意識別子を割り当てる：

```
ユーザーから見える        コンパイラ内部表現
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N はコンパイル時一意整数
&mut Point     →  WriteToken(Point, #M)   // #M はコンパイル時一意整数
```

ブランドの用途：

- **偽造防止**：トークンは所有者カプセルからのみ取得可能、凭空構築不可
- **関連追跡**：フィールドアクセスから派生した `&Float`
  は派生ブランド（`#N.field_x`）を携带し、コンパイラは親トークンまで追跡可能
- **競合検出**：同源 WriteToken と派生 ReadToken は同時にアクティブにできない

ブランドは単態化とインライン化後に完全に消滅し、生成される機械語には存在しない。**ゼロ実行時オーバーヘッド。**

### 12.7 トークン Sum 型

```
&BorrowToken ::= &T          // ReadToken（ソースデータを凍結 → Dup 安全）
               | &mut T      // WriteToken（排他読み書き → Linear）
```

### 12.8 借用トークン vs ref

|              | `&T` / `&mut T`                                    | `ref`                                 |
| ------------ | -------------------------------------------------- | ------------------------------------- |
| 役割         | 一目見る/その場で変更                              | 共有所有                              |
| 範囲         | トークン値のスコープに従う                         | スコープ横断                          |
| コスト       | ゼロオーバーヘッド（零サイズ型、コンパイル後消滅） | Rc または Arc（コンパイラが選択）     |
| エスケープ   | 可（トークンは戻り値/構造体で伝播）                | 本来エスケープ用                      |
| タスク横断   | 不可（トークンはタスク横断渡し未実装）             | 可（コンパイラが自動的に Arc を選択） |
| サイクル検出 | 該当なし                                           | タスク内は静的、タスク横断は lint     |

> 注（未定義）：ref 作成後の内容の読み取り方法（デリファレンス/メソッド/自動）はまだ仕様で定義されておらず、実装現状の
> `*a` は E1052 を報告する。定義後に本節に補完する。

---

## 付録：型定義クイックリファレンス

### A.1 型定義

```
// === レコード型（波括弧） ===

// レコード型
Point: Type = { x: Float, y: Float }

// バリアント付きレコード型（関数字段を使用）
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

// === 停止測度（組み込み述語、§8.4 参照） ===

// 一項：アンカーがバインディング名（自己再帰関数、ループ）
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n { i = i + 1 }
    return acc
}

// 二項：測度の帰属を明示的に指定（測度が他所で定義）
gcd_measure: (a: Int, b: Int) -> Int = { b }
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}
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

// コンパイル時ジェネリクス：N が型位置 (k: N) で参照される → コンパイル時値パラメータ
factorial: (N: Int)(k: N) -> Int = { ... }
Measure: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }

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
Bool, Char      // Dup ではなく、コンパイラがプリミティブに対して組み込む処理

// === Dup（浅いコピー：ハンドルをコピー、底层データを共有） ===
&T              // 零サイズ読み取りトークン、トークンのコピー = 同じデータへの複数のビュー
ref T           // Rc/Arc コピー = 参照カウント+1、ヒープデータを共有

// === Linear ===
&mut T          // 零サイズ書き込みトークン、Linear（排他、複製不可）

// === Clone（明示的な深いコピー） ===
value.clone()   // 独立副本を作成、変更は原値に影響しない
```

### A.4 借用トークンクイックリファレンス

```
// === 借用トークン ===
&T              // 零サイズコンパイル時読み取りトークン、ソースデータを凍結 → Dup（複製可能）
&mut T          // 零サイズコンパイル時書き込みトークン、排他読み書き → Linear（複製不可）

// 呼び出し側の自動選択
// 1. 実引数が後に使用される場合 → トークン作成
// 2. 実引数がもう使用されない場合 → Move
// 3. 優先一致：&T < &mut T < Move

// トークン伝播
// ✅ 戻り値として返せる、構造体に格納できる、クロージャにキャプチャされる
// ❌ タスク横断不可（トークンはタスク横断渡し未実装）
```

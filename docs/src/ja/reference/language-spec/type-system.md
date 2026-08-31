# 型システム仕様

この文書は YaoXiang プログラミング言語の型システム仕様を定義する。基本型、複合型、ジェネリクス、trait を含む。

---

## 第零章：理論的基礎

### 0.1 Curry-Howard 対応

Curry-Howard 対応（Curry-Howard
correspondence）は YaoXiang 型システムの理論的基礎である。プログラミング言語の型システムと数理論理の間の深い対応関係を明らかにする：

| 論理学                         | プログラミング言語                          |
| ------------------------------ | ------------------------------------------- |
| 命題 \(P\)                     | 型 `Type`                                   |
| 証明 \(p: P\)                  | プログラム `x: T = ...`                     |
| 含意 \(P \rightarrow Q\)       | 関数型 `(P) -> Q`                           |
| 連言 \(P \wedge Q\)            | 積型 `{ a: P, b: Q }`                       |
| 選言 \(P \vee Q\)              | 和型 `{ a(P) \| b(Q) }`                     |
| 全称量化 \(\forall x:T. P(x)\) | ジェネリクス `(T: Type) -> ...`             |
| 真 \(\top\)                    | `Void`（Unit、デフォルト値を持つ）          |
| 偽 \(\bot\)                    | `Never`（零コンストラクタ、値は存在しない） |
| 型宇宙 \(Type_n : Type_{n+1}\) | 宇宙階層（Russell パラドックス防止）        |
| case 分析                      | 型レベル `match`                            |

> **注意**：型レベル `match` は場合分け（case
> analysis）であり、数学的帰納法ではない。帰納法には型レベル再帰関数とコンパイラの停止性検査が必要。

### 0.2 型は命題、プログラムは証明

YaoXiang では、この対応関係は設計の第一級の原則である：

- **停止する型レベル計算は正しい構成的証明に対応する**。YaoXiang の型族（`Nat` 上の `Add`
  の case 分析 + 再帰呼び出しなど）は、本質的に数学的帰納法の型レベル符号化である——ただし、コンパイラが停止性検査を行えることが前提。
- **型検査は証明の検証である**。プログラムが型検査を通過するということは、論理的命題が構成的に証明されたことを意味する。

### 0.3 言語設計への影響

YaoXiang における Curry-Howard 対応の具体的な現れ：

1. **宇宙階層**（RFC-010）：`Type₀ : Type₁ : Type₂ …` により、`Type: Type`
   に起因する論理的パラドックス（Girard パラドックス）を回避
2. **型族**（RFC-011）：自然数 `Nat(Zero/Succ)`
   の型レベル case 分析 + 再帰呼び出しは Peano 公理に対応する——ただし、コンパイラが停止性検査を行うことが前提
3. **条件型**（RFC-011）：`If: (C: Bool, T: Type, E: Type) -> Type` は論理の case 選言に対応する
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

> **設計説明**：RFC-010 では「すべてが代入である」という統一モデル（`name: type = value`）を提案しているが、構文のレベルでは型と値は依然として区別される必要がある。コンパイラの実装では
> `Type` と `Expr` は二つの独立した AST 列挙型（`ast.rs:406` と `ast.rs:25`）であり、`TypeExpr`
> は BNF プレースホルダとして実装の `Type` 列挙型に対応し、「この位置は型を期待する」ことを表す。

---

## 第二章：基本型

### 2.1 プリミティブ型

| 型       | 論理的対応   | 説明                                                                                        | デフォルトサイズ |
| -------- | ------------ | ------------------------------------------------------------------------------------------- | ---------------- |
| `Type`   | —            | メタ型                                                                                      | 0 バイト         |
| `Never`  | ⊥（偽/空型） | 零コンストラクタ、値を持たない。発散/panic の戻り型。`Never <: T` は任意の T に対して成立。 | 0 バイト         |
| `Void`   | ⊤（真/Unit） | デフォルト void 値を持つ零フィールド積型。`x: Void = <デフォルト>` は合法。                 | 0 バイト         |
| `Bool`   | —            | ブール値：`true` / `false`                                                                  | 1 バイト         |
| `Int`    | —            | 符号付き整数                                                                                | 8 バイト         |
| `Uint`   | —            | 符号なし整数                                                                                | 8 バイト         |
| `Float`  | —            | 浮動小数点数                                                                                | 8 バイト         |
| `String` | —            | UTF-8 文字列                                                                                | 可変             |
| `Char`   | —            | Unicode 文字                                                                                | 4 バイト         |
| `Bytes`  | —            | 生バイト列                                                                                  | 可変             |

ビット幅付き整数：`Int8`, `Int16`, `Int32`, `Int64`, `Int128` ビット幅付き浮動小数点：`Float32`,
`Float64`

### 2.2 Never と Void：⊥ と ⊤

`Never` と `Void` は型システムの論理的プリミティブであり、それぞれ偽（⊥）と真（⊤）に対応する。

**Never（⊥、偽/空型）** — 三つの交渉不可能な性質：

1. **零コンストラクタ**：`Never` 型の値を生成するリテラルや式は存在しない。`x: Never = ...`
   の右辺は書けない。
2. **爆発原理**：`Never <: T` は任意の型 `T` に対して成立する。`assert(false)` は `Never`
   を返し、その後のコードは型検査を通過する（実際には実行されないが）。
3. **発散マーカー**：`f: (...) -> Never` は `f`
   が戻らないことを保証する。コンパイラはこれに基づいて dead code 分析と `match` 分岐合流を行う。

`Never` は（`Int`/`Bool` と同じ登録パスの）組み込み型名であり、キーワードではない。

**Void（⊤、真/Unit）** — 唯一の居住者（デフォルト void 値）を持つ。`Void`
は零フィールド積型の単位元である。`x: Void = <デフォルト>` は合法であり、`return`
文を持たない関数のデフォルト戻り型は `Void`。

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

- レコード型は波括弧 `{}` を使って定義する
- フィールド名の後にコロンと型を続ける
- インタフェース名は型本体内に記述することで実装を示す

> **名前空間の所属**：`Type.name` 接頭辞（例：`Point.draw`）は関数が `Point`
> の名前空間に属することを示すだけで、いかなる暗黙の束縛も引き起こさない。`p.draw()` のような `.`
> 呼び出し構文を有効にするには、明示的な束縛が必要：
> `Point.draw = draw[0]`。詳細は RFC-004 および RFC-010 を参照。

#### 3.1.1 フィールドのデフォルト値

型のフィールドにはデフォルト値を指定でき、構築時には任意の指定が可能：

```yaoxiang
// デフォルト値を持つフィールド - 構築時は任意
Point: Type = {
    x: Float = 0,
    y: Float = 0
}

// 使用方法
Point()           // -> Point(x=0, y=0)
Point(x=1)       // -> Point(x=1, y=0)
Point(x=1, y=2) // -> Point(x=1, y=2)

// デフォルト値を持たないフィールド - 構築時に必須
Point2: Type = {
    x: Float,
    y: Float
}

// 使用方法
Point2(x=1, y=2) // 正しい
Point2()          // エラー
```

**規則**：

- `field: Type = expression` -> デフォルト値を持つ、構築時は任意
- `field: Type` -> デフォルト値を持たない、構築時に必須

#### 3.1.2 組み込み束縛

型定義の本体内では直接メソッドを束縛できる：

```yaoxiang
// 方法1：外部関数の参照による束縛
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

### 3.2 インタフェース型

```
InterfaceType ::= '{' FnField (',' FnField)* ','?
FnField       ::= Identifier ':' FnType
FnType        ::= '(' ParamTypes? ')' '->' TypeExpr
```

**構文**：インタフェースはフィールドがすべて関数型であるレコード型

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

**インタフェース実装**：型は定義の末尾にインタフェース名を列挙することで実装する

```yaoxiang
// インタフェースを実装する型
Point: Type = {
    x: Float,
    y: Float,
    Drawable,        // Drawable インタフェースを実装
    Serializable     // Serializable インタフェースを実装
}
```

**インタフェースへの直接代入**：具象型はインタフェース型変数に直接代入可能（構造的サブタイピング）

```yaoxiang
// 直接代入（コンパイル時に具象型が確定 -> ゼロオーバーヘッド呼び出し）
d: Drawable = Circle(1)
d.draw(screen)        // コンパイル後：circle_draw を直接呼び出し、vtable なし

// 関数の戻り値（コンパイル時に確定不可 -> vtable 呼び出し）
d: Drawable = get_shape()
d.draw(screen)        // vtable 経由でメソッド検索

// 関数のパラメータとしてのインタフェース
process: (d: Drawable) -> Void = d.draw(screen)
```

**コンパイル時最適化戦略**：

| シナリオ         | 推論結果         | 呼び出し方式                       |
| ---------------- | ---------------- | ---------------------------------- |
| 具象型の直接代入 | 具象型が確定可能 | 直接呼び出し（ゼロオーバーヘッド） |
| 関数の戻り値     | 不明             | vtable                             |
| 異種集合         | 複数の型         | vtable                             |

**コヒーレンスとオーファンルール（適用不可、結論説明）**：YaoXiang のインタフェースは構造的型（インタフェース = フィールドがすべて関数型であるレコード）であり、名目的な trait ではない——クレート/モジュールをまたぐ「誰が何に対して実装できるか」という帰属問題が存在しないため、Rust 式のオーファンルールとコヒーレンス検査には適用対象がない（裁定記録は #46、RFC-011
§2.1）。構造的世界の対応する保証は**重複実装の拒否**である：同じメソッドシグネチャを型の上で重複定義するとコンパイルエラー（RFC-011a
§3、上書き禁止；オーバーロードは合法）。TraitResolver の名目的解決メカニズムは #46 と共に削除済み。

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

````yaoxiang
List: (T: Type) -> Type = { ... }
Map: (K: Type, V: Type) -> Type = { ... }
``

### 4.1.1 コンテナ型（#299）

コンテナ型はジェネリック型コンストラクタであり、組み込みプリミティブではない——ユーザ定義ジェネリクスと同一の扱いを受け、統一されたジェネリクスのインスタンス化パスを経由して処理される：

| 型            | セマンティクス            | 基底表現             |
| ------------- | ------------------------- | -------------------- |
| `List(T)`     | 可変長リスト              | `HeapValue::List`    |
| `Array(T, N)` | 固定長配列（const ジェネリクス N）| `HeapValue::Array` |
| `Dict(K, V)`  | キー値マッピング          | `HeapValue::Dict`    |

> `Set(T)` は廃止済み（#300 決定4）：リテラルも、ランタイム表現も、std.set も存在しない。
> 必要が生じた時点で Dict パターンに倣って補完する。

主要規則：

- **リテラルの着地点は文脈が決定する**：`[...]` の生リテラルと `List(T)` 注釈は可変長
  リストに着地する；`Array(T, N)` 注釈がリテラルに直接作用する場合は固定長配列に着地する。
  着地点検証（#300）：要素数 == N、要素型が T と互換、不一致はコンパイル時 E1002；
  N がシンボル定数（const パラメータ）の場合、要素数検証は型精化フェーズまで延期される。
- **暗黙の List→Array 変換を禁止**：固定長は型層で保証される——push は
  `List(A)` の receiver のみ受け付ける。
- **インデックス失敗の契約**（ランタイムエラーは過渡的状態、目標はコンパイル時精化によるカバー、
  値依存型へ）：
  - インデックス範囲外（負のインデックスを含む）→ `E6003`
  - Dict のキー欠落 → `E6008`
- **membership `in` 述語**：`Bool` を返しエラーとならない、右オペランドは
  List/Array/Dict(キー)/Tuple/String/Range をカバーする。第一級のホール述語であり、
  精化型のコンパイル時証明可能な命題の基底である。`

ジェネリック関数では、型パラメータもシグネチャで宣言され、コンパイラが実引数から自動的に推論する：

```yaoxiang
map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R)) = ...
````

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
    push: (self: List(T), item: T) -> Void,   // self は単なる慣習名であり、キーワードではない
    get: (self: List(T), index: Int) -> Option(T)
}
```

### 4.3 ジェネリック構築呼び出しと型推論

ジェネリック型定義のフィールドリストは**自動的にコンストラクタを生成する**：各フィールドが構築パラメータに対応し、フィールド名がパラメータ名となる；デフォルト値を持つフィールドは構築時に省略可能で、デフォルト値を持たないフィールドは必須。関数型のフィールド（メソッド）は構築パラメータを生成しない。

```yaoxiang
// 型定義
Container: (T: Type) -> Type = {
    value: T,        // デフォルト値なし → 構築パラメータ必須
    extra: T,
}
// 自動的に展開された完全な形式（コンパイラ内部ビュー、ユーザの手書きは不要）：
// Container: (T: Type) -> (value: T, extra: T) -> Type = {
//     value: T = value,
//     extra: T = extra,
// }

// 呼び出し：自動生成されたコンストラクタの呼び出し
c  = Container(42, 43)            // 構築パラメータをフィールド順に埋める；T は要素から自動アンパック = Int
c2 = Container("a", "b")          // T = String
c3 = Container(Int)(42, 43)       // 明示的な型パラメータ + 位置指定構築パラメータ
c4 = Container(Int)(extra=43, value=42)  // フィールド名指定、順序任意
c5 = Container(Int)()             // 空構築：フィールドはデフォルト値/ゼロ値を取得（データは後で代入）

// フィールドデフォルト値 → 構築パラメータ省略可能
Point: (T: Type) -> Type = { x: T = 0, y: T = 0 }
p  = Point(1.5, 2.5)              // T = Float, x←1.5, y←2.5
p2 = Point(Int)()                 // x=0, y=0
```

**呼び出し規則**（単一括弧、宣言パラメータと位置ごとにマッチング、左から右）：

1. 実引数を位置ごとに型宣言パラメータとマッチングを試みる：`Type`
   位置は型実引数を受け付け、コンパイル時値パラメータ位置（例：`Int`）はコンパイル時定数を受け付ける。
2. コンパイル時値パラメータ位置へのマッチが成功した場合（部分マッチ）、型構築として処理する：全てのパラメータ位置を順にチェックし、エラー時は宣言順に**最初にマッチしない/欠落したパラメータ**を報告する。
3. 実引数が宣言パラメータに完全に対応しない場合（全て値、コンパイル時値パラメータ位置へのマッチなし）、構築パラメータとして処理する：位置指定はフィールド順に、型パラメータは要素型から自動アンパック。

```yaoxiang
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),
    data: Array(Array(T, Cols), Rows),
}

m: Matrix(Int, 3, 4)              // 型位置：一段の型構築
m2 = Matrix(Int, 3, 4)(data=[[1,2,3,4],[5,6,7,8],[9,10,11,12]])  // 二段：型 + 構築パラメータ
m3 = Matrix(Int, 3, 4)()          // 空構築（RFC-011 §9.3 モード、データは後で代入）

Matrix(42)    // ❌ 位置0: T←42 がマッチしない（42 は型ではない）；位置1: Rows←42 がマッチ；
              //    位置2: Cols 欠落 → 最初のエラーを報告：T は Type を期待、42 が見つかった
Container(42) // ❌ 構築パラメータ extra が欠落
Container(42, 43, 44)  // ❌ 構築パラメータ過多
```

**型推論**：ジェネリック型コンストラクタの型パラメータは構築パラメータ要素から自動アンパック（`Container(42, 43)`
→ T=Int）；ジェネリック関数の型パラメータは実引数型から自動アンパック（`map(numbers, f)` → T=Int,
R=String、§4.1 参照）。アンパックできない場合は明示的に指定する必要がある。

---

## 第五章：型制約

### 5.1 単一制約

```
ConstrainedType ::= '(' Identifier ':' TypeBound ')' TypeExpr
```

```yaoxiang
// インタフェース型定義（制約として）
Clone: Type = {
    clone: () -> Clone
}

// 制約の使用
clone: (T: Clone)(value: T) -> T = value.clone()
```

### 5.2 多重制約

```yaoxiang
// 多重制約構文
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

> **訂正（#296）**：原文ではコンパイル時値パラメータを「デフォルトでコンパイル時確定」と述べていたが、この表現は**厳密に誤り**である——
> `add: (a: Int, b: Int) -> Int = a + b` における `a`/`b`
> はランタイム値パラメータである。**型位置で参照される**
> 具体的な型パラメータのみがコンパイル時値パラメータとなる。正しい定義は以下。

**用語**：`Type`
以外の具体型（例：`Int`）で注釈されたジェネリクスパラメータを**コンパイル時値パラメータ候補**と呼び、コンパイル時値パラメータになるかどうかは、その値が型位置で参照されるか（値依存）による。**`const`
キーワードは不要**
（実装内部では一時的に「const ジェネリクス」という用語を使用していたが、ドキュメントでは「コンパイル時値パラメータ」に統一）。

**判定規則（二段階）**：

1. **形態による粗選別**：パラメータが `Type`
   以外の具体型（`Int`/`Bool`/`Float`）で注釈されている → 候補。
2. **用途による精選別**：候補名が**型位置**に現れる（型本体のフィールド型、内部 `Fn` パラメータ型、
   `Assert` 述語、`Array(T, N)`
   型構築実引数位置）→ 真のコンパイル時値パラメータ；そうでなければ**ランタイム値パラメータ**。

| 記述                                                       | 判定                              | 理由                         |
| ---------------------------------------------------------- | --------------------------------- | ---------------------------- |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b はランタイム値パラメータ      | 値位置にのみ出現             |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N はコンパイル時値パラメータ      | N が型構築実引数位置に出現   |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N はコンパイル時値パラメータ      | N が内部パラメータ k の型    |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N は空振り→ランタイム値パラメータ | N が型本体で参照されていない |

**核となる設計**：(N: Int) コンパイル時値パラメータと (k:
N) 値パラメータを用いて、コンパイル時定数とランタイム値を区別する。空振り候補（形態は候補だが用途が該当しない）はランタイム値パラメータに退化する——関数レベルではすでにこの扱いがされているが、型コンストラクタパスは
[issue #297](https://github.com/ChenXu233/YaoXiang/issues/297) を参照。

```yaoxiang
// コンパイル時値パラメータ：N が型位置（Array 長さスロット）で参照される
StaticArray: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // N が型構築実引数位置に出現 → コンパイル時値パラメータ
    length: N
}

// 使用方法：factorial(5) が型位置で評価され（コンパイル時）、結果 120 が型に埋め込まれる
arr: StaticArray(Int, factorial(5))  // コンパイラがコンパイル時に factorial(5) = 120 を計算

// 値依存：N が内部パラメータ k の型として使われる
// N はコンパイル時値パラメータ（(k: N) の型位置に出現）；
// k はランタイム値パラメータであり、その型はリテラル型 N（単一値型）。
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
は同じ精化プリミティブの二つの側面であり、dispatch 分岐パイプラインが「述語の自由変数がコンパイル時に到達可能か」に基づいて自動選択する。

**核となるシグネチャ**：`assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**dispatch 分岐規則**：

| 判定基準                                                                       | モード      | 振る舞い                                                                             |
| ------------------------------------------------------------------------------ | ----------- | ------------------------------------------------------------------------------------ |
| 全ての自由変数がコンパイル時に既知（ジェネリクスパラメータ、コンパイル時定数） | CompileTime | 証明パイプラインへ：true → Void に消去、false → コンパイルエラー（Never は居住不可） |
| ランタイム自由変数が存在（関数パラメータ、外部入力）                           | Runtime     | ランタイム Bool 検査を挿入し、フロー依存仮定集合 Γ に精化事実を注入                  |

**フロー依存仮定集合 Γ**：

コンパイラは各制御フロー点の既知命題集合を維持する：

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP 伝播
mut x = x - 5       // Γ = {}  ← mut キリングセット：旧仮定が無効化される
```

`mut`
変数の代入後、その変数に関する全ての仮定が削除される（キリングセット）。分岐合流時、Γ は各分岐の積集合を取る。

---

## 第九章：型の結合と交差

### 9.1 型の結合

```
TypeUnion     ::= TypeExpr '|' TypeExpr
```

### 9.2 型の交差

```
TypeIntersection ::= TypeExpr '&' TypeExpr
```

**構文**：型の交差 `A & B` は A と B の両方を満たす型を表す

```yaoxiang
// インタフェース組み合わせ = 型の交差
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

YaoXiang には区別すべき型属性は一つしかない：線形 vs 複製可能。コンパイラが自動推論する。

### 11.1 Move（デフォルトの所有権移転）

全ての型はデフォルトで Move セマンティクスに従う。代入、引数渡し、戻り値 = 所有権移転。

```yaoxiang
p: Point = Point(1.0, 2.0)
q = p           // Move、p はもはや読み取れない
```

### 11.2 Dup（シャローコピー：ハンドル複製、データ共有）

**Dup 属性は参照/トークン型に使用される**。Dup 型の代入 = シャローコピー——ハンドル/トークンを複製し、基底データを共有する。複数の所有者が同じデータブロックを指す。

| 型             | 属性   | 説明                                                                    |
| -------------- | ------ | ----------------------------------------------------------------------- |
| `&T`           | Dup    | ゼロサイズ読み取りトークン、トークン複製 = 複数の視点が同じデータを指す |
| `ref T`        | Dup    | Rc/Arc 複製 = 参照カウント+1、ヒープデータ共有                          |
| `&mut T`       | Linear | ゼロサイズ書き込みトークン、独占、複製不可                              |
| その他全ての型 | Move   | デフォルトの所有権移転                                                  |

**プリミティブ値型**（Int, Float, Bool,
Char）はコンパイラ組み込みの特殊処理：代入時に自動値コピーされ、二つの値は完全に独立。これはコンパイラのネイティブ動作であり、Dup 型属性には含まれない。

```yaoxiang
// &T: Dup、自由なエイリアスが可能
view: &Point = &p
view2 = view     // Dup：トークン複製、両者とも有効
print(view.x)    // 使用可能
print(view2.x)   // 使用可能

// &mut T: Linear、複製不可
mut_ref: &mut Point = &mut p
// r2 = mut_ref  // ❌ &mut T は Dup ではない、複製できない
```

### 11.3 Clone（明示的なディープコピー）と Dup の関係

**Clone** は明示的なディープコピーインタフェースである。全ての型は Clone を実装でき、`.clone()`
メソッドを提供する。

```yaoxiang
// Clone インタフェース定義（標準ライブラリ）
Clone: Type = {
    clone: () -> Clone
}

// 使用
p: Point = Point(1.0, 2.0)
backup = p.clone()    // ディープコピー、p は依然として使用可能
p2 = p.clone()        // 複数回クローン可能
```

**Dup と Clone の違い**：

|                    | Dup                                                   | Clone                                  |
| ------------------ | ----------------------------------------------------- | -------------------------------------- |
| **セマンティクス** | シャローコピー：ハンドル/トークン複製、基底データ共有 | ディープコピー：完全な独立コピー作成   |
| **呼び出し方式**   | 暗黙的（代入/引数渡しで自動）                         | 明示的（`.clone()`）                   |
| **変更影響**       | 相互に影響する（基底データ共有）                      | 相互に影響しない（独立コピー）         |
| **適用型**         | `&T` トークン、`ref T`                                | Clone インタフェースを実装する任意の型 |
| **コスト**         | ゼロオーバーヘッド（トークンはゼロサイズ型）          | 型による                               |

**Dup は Clone を含意せず、Clone は Dup を含意しない**——これらは直交する二つの概念である：

```yaoxiang
// Dup 型：トークン複製、基底データ共有
view: &Point = &p
view2 = view        // Dup：トークン複製、両者は同じ p を指す
print(view.x)       // 使用可能
print(view2.x)      // 使用可能、同じデータが見える

// プリミティブ値型：コンパイラによる自動値コピー（Dup ではない）
x: Int = 42
y = x               // 値コピー、x と y は完全に独立
print(x)            // 使用可能

// Clone：明示的なディープコピー、独立コピー作成
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone：ディープコピー、p は依然として使用可能
r = p               // Move：所有権移転、Point は Dup でもプリミティブ値型でもないため
```

**設計意図**：

- Dup はトークン/参照型に使用され、「複数の視点で同じデータを見る」問題を解決する
- Clone は独立コピーが必要なシナリオに使用され、明示的な呼び出しによりコストを可視化する
- プリミティブ値型（Int/Float/Bool/Char）のコピーはコンパイラの組み込み動作であり、Dup には含まれない
- ほとんどのカスタム型はデフォルトで Move であり、ゼロコピーで高性能

## 第十二章：借用トークン型

### 12.1 核となる概念

`&T` と `&mut T`
は**ゼロサイズのコンパイル時トークン型**である。これらは「参照」ではなく、「アクセス権の型レベル証明」である。

```
&T      →  ゼロサイズ、ソースデータを凍結（この間の WriteToken 取得を禁止）、
          凍結保証の下で複数読み取りが安全 → Dup（複製可能）
&mut T  →  ゼロサイズ、排他的読み書き（他の全てのトークンを禁止）、
          排他的アクセス下では複製が無意味 → Linear（非 Dup）
```

**主要な特性**：

- トークンは**通常の型**であり、他の全ての型と同じスコープ規則に従う
- ライフタイム注釈 `'a` は不要
- 専用の借用チェッカーは不要——型属性（Dup/Linear）が自然に権限を推論する
- コンパイル後に完全に消滅し、ランタイムオーバーヘッドはゼロ

### 12.2 基本的な使用方法

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
p.print()                       // OK、前のトークンは shift 呼び出しの終了と共に解放済み

// 複数の &T トークンの共存——Dup 型は自由なコピーを許可
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 トークンのスコープと伝播

トークンは通常の型であるため、通常の型がサポートする全ての操作をサポートする：

**トークンの返却**——トークンは戻り値と共に伝播する：

```yaoxiang
// ✅ 子トークンと親トークンを一緒に返す
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // トークンは呼び出し元に返却される
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

**クロージャはキャプチャしない、文脈は作成時点で固定される**——クロージャは自分のパラメータのみを消費し、外側のデータが必要な場合はカリー化により作成時点で値として固定する：

```yaoxiang
// ✅ 文脈はカリー化により固定される：threshold はパラメータ、gt_point(threshold) は作成時点で値としてクロージャに固定
gt_point: (t: Float) -> (p: Point) -> Bool = (p) => p.x > t
filter_by_threshold: (items: List(Point), threshold: Float) -> List(Point) = {
    items.filter(gt_point(threshold))
}
```

> 注：クロージャ（関数値）がエスケープした後、定義箇所のスコープは既に死んでいる可能性があるため、外側の変数の暗黙的キャプチャは禁止；しかし呼び出し点（作成点）のスコープは必ず生存しており、その時点で文脈が値として固定されてクロージャに入ることは安全。

### 12.4 自動借用選択

呼び出し側コンパイラは以下の優先順位で自動選択する：

```
1. 実引数が後続でも使用される → トークン作成を優先（&T または &mut T、メソッドシグネチャによる）
2. 実引数が後続で未使用 → Move
3. 優先マッチング順序：&T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // print のパラメータ型が &Point → コンパイラが &Point トークンを作成
p.shift(1.0, 1.0)  // shift のパラメータ型が &mut Point → コンパイラが &mut Point トークンを作成
p2 = p             // 後続で未使用 → Move
```

**メソッドレシーバはシグネチャセマンティクスに従う**（2026-08-30 訂正、RFC-011a レシーバ命名規約と同種）：レシーバが
`&T` → 読み取り借用トークン；`&mut T` → 可変借用トークン；値渡し →
Move（レシーバを消費）。呼び出し点で生成された借用トークンは呼び出し終了と共に解放される（transient、§12.5 スコープセマンティクス）；インタフェースの借用レシーバはインタフェース設計者が明示的に
`&Self` を宣言し、impl シグネチャは `Self ↦ impl 型`
置換後にインタフェースと完全一致する必要がある（RFC-011a §3）。

### 12.5 トークン競合検出

トークン競合検出は**借用ホール命題**（RFC-009a）であり、独立したフロー依存分析ではない。コンパイラが自動的に借用命題（`borrow_conflict`/`use_after_move`/`use_after_drop`/`mut_violation`）を生成して証明パイプラインに送り検証する；トークン活性はスコープ
`[created_at, last_use]`（RFC-009a §逆 BFS 活性分析を参照）：

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

// ❌ 同じ実引数から同時に &mut トークンと他のトークンを作成できない
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p が同時に &mut と & トークンを派生
```

### 12.6 コンパイラ内部：ブランド機構

ユーザはブランドに一切触れない。コンパイラが内部で各トークンにコンパイル時一意識別子を割り当てる：

```
ユーザから見える       コンパイラ内部表現
─────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N はコンパイル時一意整数
&mut Point     →  WriteToken(Point, #M)   // #M はコンパイル時一意整数
```

ブランドの用途：

- **偽造防止**：トークンは所有者カプセルからのみ取得可能、凭空には構築できない
- **関連追跡**：フィールドアクセスから派生した `&Float`
  は派生ブランド（`#N.field_x`）を携带し、コンパイラが親トークンまで追跡可能
- **競合検出**：同源 WriteToken と派生 ReadToken は同時にアクティブにできない

ブランドは単態化とインライン化後に完全に消滅し、生成された機械語には存在しない。**ランタイムオーバーヘッドはゼロ**。

### 12.7 トークン Sum 型

```
&BorrowToken ::= &T          // ReadToken（ソースデータ凍結 → Dup 安全）
               | &mut T      // WriteToken（排他的読み書き → Linear）
```

### 12.8 借用トークン vs ref

|              | `&T` / `&mut T`                                      | `ref`                             |
| ------------ | ---------------------------------------------------- | --------------------------------- |
| 動作         | 一目見る/その場で変更                                | 共有保有                          |
| 範囲         | トークン値のスコープに従う                           | スコープをまたぐ                  |
| コスト       | ゼロオーバーヘッド（ゼロサイズ型、コンパイル後消滅） | Rc または Arc（コンパイラ選択）   |
| エスケープ   | 可能（トークンは戻り値/構造体で伝播）                | 本来エスケープ用                  |
| タスク間     | 不可（トークンはタスク間渡し未実装）                 | 可能（コンパイラが自動選択 Arc）  |
| サイクル検出 | 関与しない                                           | タスク内では無音、タスク間で lint |

> 注（未定義）：ref 作成後の内容読み取り（デリファレンス/メソッド/自動）はまだ仕様で定義されておらず、実装現状では
> `*a` は E1052 を報告する。定義後に本節に補完する。

---

## 付録：型定義早見表

### A.1 型定義

```
// === レコード型（波括弧） ===

// レコード型
Point: Type = { x: Float, y: Float }

// バリアント付きレコード型（関数フィールドを使用）
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// === インタフェース型（波括弧、フィールドは全て関数） ===

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

// コンパイル時ジェネリクス：N が型位置 (k: N) で参照される → コンパイル時値パラメータ
factorial: (N: Int)(k: N) -> Int = { ... }
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
// 全ての型はデフォルトで Move。代入、引数渡し、戻り値 = 所有権移転

// === プリミティブ値型（コンパイラ組み込み） ===
Int, Float,     // 代入時に自動値コピー、二つの値は完全に独立
Bool, Char      // Dup ではなく、コンパイラのプリミティブ組み込み処理

// === Dup（シャローコピー：ハンドル複製、基底データ共有） ===
&T              // ゼロサイズ読み取りトークン、トークン複製 = 複数の視点が同じデータを指す
ref T           // Rc/Arc 複製 = 参照カウント+1、ヒープデータ共有

// === Linear ===
&mut T          // ゼロサイズ書き込みトークン、Linear（独占、複製不可）

// === Clone（明示的ディープコピー） ===
value.clone()   // 独立コピー作成、変更は元値に影響しない
```

### A.4 借用トークン早見表

```
// === 借用トークン ===
&T              // ゼロサイズコンパイル時読み取りトークン、ソースデータ凍結 → Dup（複製可能）
&mut T          // ゼロサイズコンパイル時書き込みトークン、排他的読み書き → Linear（複製不可）

// 呼び出し側自動選択
// 1. 実引数が後続でも使用される → トークン作成
// 2. 実引数が後続で未使用 → Move
// 3. 優先マッチング：&T < &mut T < Move

// トークン伝播
// ✅ 返却可能、構造体に格納可能、クロージャでキャプチャ可能
// ❌ タスク間は不可（トークンはタスク間渡し未実装）
```

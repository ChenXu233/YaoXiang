# 型システム仕様

本文書は YaoXiang プログラミング言語の型システム仕様を定義する。基本型、複合型、ジェネリクス、trait を含む。

---

## 第零章：理論的基礎

### 0.1 Curry-Howard 同型

Curry-Howard 同型（Curry-Howard
correspondence）は YaoXiang 型システムの理論的基礎である。これはプログラミング言語の型システムと数理論理との深層対応関係を明らかにする：

| 論理学                         | プログラミング言語                          |
| ------------------------------ | ------------------------------------------- |
| 命題 \(P\)                     | 型 `Type`                                   |
| 証明 \(p: P\)                  | プログラム `x: T = ...`                     |
| 含意 \(P \rightarrow Q\)       | 関数型 `(P) -> Q`                           |
| 連言 \(P \wedge Q\)            | 積型 `{ a: P, b: Q }`                       |
| 選言 \(P \vee Q\)              | 和型 `{ a(P) \| b(Q) }`                     |
| 全称量化 \(\forall x:T. P(x)\) | ジェネリクス `(T: Type) -> ...`             |
| 真 \(\top\)                    | `Void`（Unit、デフォルト値あり）            |
| 偽 \(\bot\)                    | `Never`（ゼロコンストラクタ、値は存在せず） |
| 型宇宙 \(Type_n : Type_{n+1}\) | 宇宙階層（Russell のパラドックス防止）      |
| case 分析                      | 型レベル `match`                            |

> **注意**：型レベル `match` は場合分け（case
> analysis）であり、数学的帰納法ではない。帰納法には型レベル再帰関数とコンパイラの停止性検査が必要である。

### 0.2 型は命題、プログラムは証明

YaoXiang において、この対応関係は設計の最上位原則である：

- **停止する型レベル計算は正しい構成的証明に対応する**。YaoXiang の型族（例えば `Nat` に対する `Add`
  の case 分析 + 再帰呼び出し）は本質的に数学的帰納法の型レベル符号化である——ただし、コンパイラが停止性検査を行えることが前提となる。
- **型検査は証明の検証である**。プログラムが型検査を通過するということは、論理命題が構成的に証明されたことと等価である。

### 0.3 言語設計への影響

Curry-Howard 同型の YaoXiang における具体的な体現：

1. **宇宙階層**（RFC-010）：`Type₀ : Type₁ : Type₂ …` により `Type: Type`
   に起因する論理矛盾（Girard のパラドックス）を回避
2. **型族**（RFC-011）：自然数 `Nat(Zero/Succ)`
   の型レベル case 分析 + 再帰呼び出しは Peano 公理に対応する——ただし、コンパイラが停止性検査を行うことが前提
3. **条件型**（RFC-011）：`If: (C: Bool, T: Type, E: Type) -> Type` は論理の case 選言に対応する
4. **値依存型**（RFC-011）：`Vec: (n: Int) -> Type`
   は「各整数 n に対し一つの型が存在する」という有量化に対応する

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

> **設計説明**：RFC-010 は「すべて代入である」という統一モデル（`name: type = value`）を提案しているが、構文レベルでは型と値を区別する必要がある。コンパイラ実装では
> `Type` と `Expr` は二つの独立した AST 列挙型（`ast.rs:406` と `ast.rs:25`）であり、`TypeExpr`
> は BNF プレースホルダとして実装の `Type` 列挙型に対応し、「この位置は型を期待する」ことを表す。

---

## 第二章：基本型

### 2.1 primitive type

| 型       | 論理対応     | 説明                                                                                        | デフォルトサイズ |
| -------- | ------------ | ------------------------------------------------------------------------------------------- | ---------------- |
| `Type`   | —            | メタ型                                                                                      | 0 バイト         |
| `Never`  | ⊥（偽/空型） | ゼロコンストラクタ、値を持たない。発散/panic の戻り型。`Never <: T` は任意の T に対し成立。 | 0 バイト         |
| `Void`   | ⊤（真/Unit） | デフォルトの void 値を持つゼロフィールド積型。`x: Void = <デフォルト>` は合法。             | 0 バイト         |
| `Bool`   | —            | ブール値：`true` / `false`                                                                  | 1 バイト         |
| `Int`    | —            | 符号付き整数                                                                                | 8 バイト         |
| `Uint`   | —            | 符号なし整数                                                                                | 8 バイト         |
| `Float`  | —            | 浮動小数点数                                                                                | 8 バイト         |
| `String` | —            | UTF-8 文字列                                                                                | 可変             |
| `Char`   | —            | Unicode 文字                                                                                | 4 バイト         |
| `Bytes`  | —            | 生バイト列                                                                                  | 可変             |

ビット幅付き整数：`Int8`, `Int16`, `Int32`, `Int64`, `Int128`。ビット幅付き浮動小数点：`Float32`,
`Float64`

### 2.2 Never と Void：⊥ と ⊤

`Never` と `Void` は型システムの論理プリミティブであり、それぞれ偽（⊥）と真（⊤）に対応する。

**Never（⊥、偽/空型）** — 譲歩できない三つの性質：

1. **ゼロコンストラクタ**：リテラルも式も `Never` 型の値を生成できない。`x: Never = ...`
   の右辺は書けない。
2. **爆発原理**：`Never <: T` は任意の型 `T` に対し成立する。`assert(false)` は `Never`
   を返し、以降のコードは型検査を通過できる（実際には実行されないが）。
3. **発散マーカー**：`f: (...) -> Never` は `f`
   が戻らないことを保証する。コンパイラはこれにより dead code 分析と `match` 分岐合流を行う。

`Never` は組み込み型名（`Int`/`Bool` と同じ登録パス）であり、キーワードではない。

**Void（⊤、真/Unit）** — ちょうど一つの居住者（デフォルトの void 値）を持つ。`Void`
はゼロフィールド積型の単位元である。`x: Void = <デフォルト>`
は合法。ブロックの値は**末尾式**によって与えられる（空ブロック `{}` は `Void`）、詳細は
[RFC-010a](../design/rfc/accepted/010a-tail-expression-and-return.md) を参照。

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
// 単純な記録型
Point: Type = { x: Float, y: Float }

// 空の記録型
Empty: Type = {}

// ジェネリクス付きの記録型
Pair: (T: Type) -> Type = { first: T, second: T }

// インターフェースを実装する記録型
Point: Type = {
    x: Float,
    y: Float,
    Drawable,
    Serializable
}
```

**ルール**：

- 記録型は波括弧 `{}` で定義する
- フィールド名の後にコロンと型を続ける
- インターフェース名は型本体内に記述することでそのインターフェースの実装を表す

> **名前空間の所属**：`Type.name` プレフィックス（例：`Point.draw`）は関数が `Point`
> の名前空間に属することを示す。これは暗黙の束縛を発生させない。`p.draw()` のような `.`
> 呼び出し構文を有効にするには、明示的な束縛が必要：`Point.draw = draw[0]`。詳細は RFC-004 と RFC-010 を参照。

#### 3.1.1 フィールドのデフォルト値

型のフィールドにはデフォルト値を指定でき、構築時に省略可能：

```yaoxiang
// デフォルト値を持つフィールド - 構築時は省略可能
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
Point2(x=1, y=2) // 正
Point2()          // エラー
```

**ルール**：

- `field: Type = expression` -> デフォルト値あり、構築時は省略可能
- `field: Type` -> デフォルト値なし、構築時は必須

#### 3.1.2 組み込み束縛

型定義本体内で直接メソッドを束縛できる：

```yaoxiang
// 方式1：外部関数を参照して束縛
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]    // 位置0に束縛
}
// 呼び出し：p1.distance(p2) -> distance(p1, p2)

// 方式2：無名関数 + 位置束縛
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

**インターフェースへの直接代入**：具体型はインターフェース型変数に直接代入できる（構造的サブタイピング）

```yaoxiang
// 直接代入（具体型がコンパイル時に決定可能 -> ゼロオーバーヘッド呼び出し）
d: Drawable = Circle(1)
d.draw(screen)        // コンパイル後：直接 circle_draw を呼び出し、vtable なし

// 関数の戻り値（コンパイル時に決定不可 -> vtable 呼び出し）
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
| 異種コレクション | 複数の型         | vtable                             |

**コヒーレンスとオーファンルール（不適用、決着説明）**：YaoXiang のインターフェースは構造的型（インターフェース = フィールドがすべて関数型である記録）であり、名目的 trait ではない——crate/モジュール横断の「誰が誰のために実装するか」という帰属問題は存在せず、Rust 式のオーファンルールとコヒーレンス検査は適用対象がない（裁定記録は RFC-011
§2.1）。構造的世界の対応する保証は**重複実装の拒否**である：同じメソッドシグネチャを型上で重複定義するとコンパイルエラー（RFC-011a
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

### 4.1 ジェネリックパラメータの構文

ジェネリックパラメータは関数型の一部であり、通常の引数と同じく `()` 構文を使用する：

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

コンテナ型はジェネリックな型コンストラクタであり、組み込み primitive ではない——ユーザ定義ジェネリクスと同様に扱われ、統一的なジェネリクスインスタンス化パスを通じて処理される：

| 型            | 意味                               | 基盤               |
| ------------- | ---------------------------------- | ------------------ |
| `List(T)`     | 拡張可能リスト                     | `HeapValue::List`  |
| `Array(T, N)` | 固定長配列（const ジェネリクス N） | `HeapValue::Array` |
| `Dict(K, V)`  | キーバリュー映射                   | `HeapValue::Dict`  |

> `Set(T)`
> は既に廃止：リテラルなし、ランタイム表現なし、std.set なし。需要が発生した時点で Dict パターンに従って補完する。

重要なルール：

- **リテラルの振り分けは文脈が決定**：`[...]` 素のリテラルと `List(T)`
  注釈は拡張可能リストへ；`Array(T, N)`
  注釈がリテラルに直接作用する場合は固定長配列へ。振り分け検査：要素数 ==
  N、要素型が T と互換、不一致はコンパイル時 E1002；N がシンボル定数（const パラメータ）の場合、要素数検査は型精化フェーズまで遅延される。
- **暗黙の List→Array 変換を禁止**：固定長性は型層で保証される——push は `List(A)`
  レシーバのみ受け付ける。
- **インデックス失敗契約**（ランタイムエラーは過渡的、目標はコンパイル時型精化によるカバレッジ、値依存型経由）：
  - インデックス範囲外（負のインデックスを含む）→ `E6003`
  - Dict キー欠落 → `E6008`
- **membership `in` 述語**：`Bool`
  を返しエラーなし、右オペランドは List/Array/Dict(キー)/Tuple/String/Range をカバー。一等ホーア述語であり、型精化のコンパイル時証明可能命題の基底である。`

ジェネリック関数では、型パラメータも同様にシグネチャで宣言され、コンパイラが実引数から自動推論する：

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
    push: (self: List(T), item: T) -> Void,   // self は単なる慣習名であり、キーワードではない
    get: (self: List(T), index: Int) -> Option(T)
}
```

### 4.3 ジェネリック構築呼び出しと型推論

ジェネリック型定義のフィールドリストは**コンストラクタを自動生成する**：各フィールドが構築パラメータに対応し、フィールド名がパラメータ名となる；デフォルト値を持つフィールドは構築時に省略可能、デフォルト値のないフィールドは必須。関数型フィールド（メソッド）は構築パラメータを生成しない。

```yaoxiang
// 型定義
Container: (T: Type) -> Type = {
    value: T,        // デフォルト値なし -> 構築パラメータ必須
    extra: T,
}
// 自動展開された完全形式（コンパイラ内部ビュー、ユーザの手書き不要）：
// Container: (T: Type) -> (value: T, extra: T) -> Type = {
//     value: T = value,
//     extra: T = extra,
// }

// 呼び出し：自動生成されたコンストラクタを呼び出す
c  = Container(42, 43)            // 構築パラメータはフィールド順に渡す；T は要素から自動展開 = Int
c2 = Container("a", "b")          // T = String
c3 = Container(Int)(42, 43)       // 明示的型パラメータ + 位置指定構築パラメータ
c4 = Container(Int)(extra=43, value=42)  // フィールド名指定、順序任意
c5 = Container(Int)()             // 空構築：フィールドはデフォルト値/ゼロ値を取る（データは後で代入）

// フィールドデフォルト値 -> 構築パラメータは省略可能
Point: (T: Type) -> Type = { x: T = 0, y: T = 0 }
p  = Point(1.5, 2.5)              // T = Float、x←1.5, y←2.5
p2 = Point(Int)()                 // x=0, y=0
```

**呼び出しルール**（単一括弧、宣言パラメータと位置ごとにマッチング、左から右へ）：

1. 実引数を位置ごとに型宣言パラメータとのマッチングを試行：`Type`
   位置は型実引数を受け入れ、コンパイル時値パラメータ位置（例：`Int`）はコンパイル時定数を受け入れる。
2. コンパイル時値パラメータ位置でのマッチングが部分的に成功した場合、型構築として処理：すべてのパラメータ位置を順番に検査し、エラー時は宣言順に**最初に一致しない/欠落したパラメータを報告**する。
3. 実引数が宣言パラメータに完全対応しない（すべて値で、コンパイル時値パラメータ位置にマッチングしない）場合、構築パラメータとして処理：位置指定はフィールド順、型パラメータは要素型から自動展開。

```yaoxiang
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),
    data: Array(Array(T, Cols), Rows),
}

m: Matrix(Int, 3, 4)              // 型位置：一層型構築
m2 = Matrix(Int, 3, 4)(data=[[1,2,3,4],[5,6,7,8],[9,10,11,12]])  // 二層：型 + 構築パラメータ
m3 = Matrix(Int, 3, 4)()          // 空構築（RFC-011 §9.3 パターン、データは後で代入）

Matrix(42)    // ❌ 位置0: T←42 が不一致（42 は型ではない）；位置1: Rows←42 が一致；
              //    位置2: Cols 欠落 -> 最初のエラーを報告：T は Type を期待、42 を受け取り
Container(42) // ❌ 構築パラメータ extra 欠落
Container(42, 43, 44)  // ❌ 構築パラメータ過剰
```

**型推論**：ジェネリック型コンストラクタの型パラメータは構築パラメータ要素から自動展開（`Container(42, 43)`
→ T=Int）；ジェネリック関数の型パラメータは実引数型から自動展開（`map(numbers, f)` → T=Int,
R=String、§4.1 を参照）。展開できない場合は明示的に指定しなければならない。

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

> **訂正**：原文ではコンパイル時値パラメータを「デフォルトでコンパイル時確定」と述べていたが、この表現は**厳密に誤り**である——
> `add: (a: Int, b: Int) -> Int = a + b` の `a`/`b`
> はランタイム値パラメータである。**型位置で参照される**具体型パラメータのみがコンパイル時値パラメータである。正しい定義は以下に示す。

**用語**：`Type`
以外の具体型（例：`Int`）で注釈されたジェネリックパラメータを**コンパイル時値パラメータ候補**と呼び、コンパイル時値パラメータになるかどうかは値が型位置で参照されるか（値依存）による。**`const`
キーワードは不要**（実装内部ではかつて「const ジェネリクス」と呼んでいたが、ドキュメントは統一して「コンパイル時値パラメータ」を使用する）。

**判定ルール（二段階）**：

1. **形態粗選別**：パラメータが `Type` 以外の具体型（`Int`/`Bool`/`Float`）で注釈されている → 候補。
2. **用途精選別**：候補名が**型位置**（型本体のフィールド型、内層 `Fn` パラメータ型、`Assert`
   述語、`Array(T, N)`
   型構築実引数位置）に現れる → 真のコンパイル時値パラメータ；そうでない場合は**ランタイム値パラメータ**。

| 書き方                                                     | 判定                          | 理由                            |
| ---------------------------------------------------------- | ----------------------------- | ------------------------------- |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b ランタイム値パラメータ    | 値位置にのみ出現                |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N コンパイル時値パラメータ    | N が型構築実引数位置に出現      |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N コンパイル時値パラメータ    | N が内層パラメータ k の型になる |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N 落空→ランタイム値パラメータ | N が型本体で参照されていない    |

**核心設計**：`(N: Int)` コンパイル時値パラメータと `(k: N)`
値パラメータを用いて、コンパイル時定数とランタイム値を区別する。落空候補（形態は候補だが用途が未命中）はランタイム値パラメータに退化する——関数レベルと型コンストラクタパスの両方でこの通り処理される。

```yaoxiang
// コンパイル時値パラメータ：N が型位置（Array の長さスロット）で参照される
StaticArray: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // N が型構築実引数位置に出現 -> コンパイル時値パラメータ
    length: N
}

// 使用方式：factorial(5) は型位置で評価され（コンパイル時）、結果 120 が型に埋め込まれる
arr: StaticArray(Int, factorial(5))  // コンパイラがコンパイル時に factorial(5) = 120 を計算

// 値依存：N が内層パラメータ k の型になる
// N はコンパイル時値パラメータ（(k: N) の型位置に出現）；
// k はランタイム値パラメータ、その型はリテラル型 N（単一値型）。
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

### 8.3 Assert 精化型と assert 宣言

`assert` と `Assert`
は同じ精化プリミティブの二つの側面である——dispatch 分岐パイプラインが「述語の自由変数がコンパイル時に到達可能か」によって自動選択する。

**核心シグネチャ**：`assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**dispatch 分岐ルール**：

| 判定基準                                                                       | モード      | 挙動                                                                                 |
| ------------------------------------------------------------------------------ | ----------- | ------------------------------------------------------------------------------------ |
| すべての自由変数がコンパイル時既知（ジェネリックパラメータ、コンパイル時定数） | CompileTime | 証明パイプラインへ：true → Void に消去、false → コンパイルエラー（Never は居住不可） |
| ランタイム自由変数が存在（関数パラメータ、外部入力）                           | Runtime     | ランタイム Bool 検査を挿入し、フロー依存仮定集合 Γ に精化事実を注入する              |

**フロー依存仮定集合 Γ**：

コンパイラは各制御フロー点の既知命題集合を保持する：

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP 伝播
mut x = x - 5       // Γ = {}  ← mut kill set：古い仮定が無効化される
```

`mut` 変数への代入後、その変数に関するすべての仮定が削除される（kill
set）。分岐合流時、Γ は各分岐の交差を取る。

---

## 第九章：型の和と積

### 9.1 型の和

```
TypeUnion     ::= TypeExpr '|' TypeExpr
```

### 9.2 型の積

```
TypeIntersection ::= TypeExpr '&' TypeExpr
```

**構文**：型の積 `A & B` は A と B の両方を満たす型を表す

```yaoxiang
// インターフェース合成 = 型の積
DrawableSerializable: Type = Drawable & Serializable

// 積型の使用
process: (T: Drawable & Serializable)(item: T, screen: Surface) -> String = {
    item.draw(screen)
    return item.serialize()
}
```

---

## 第十章：関数のオーバーロードと特殊化

### 10.1 関数のオーバーロード

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
// プラットフォーム型列挙（標準ライブラリ定義）
Platform: Type = { X86_64: () -> Platform, AArch64: () -> Platform, RISC_V: () -> Platform, ARM: () -> Platform, X86: () -> Platform }

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

YaoXiang には区別すべき型の属性は一つしかない：線形 vs 複製可能。コンパイラが自動推論する。

### 11.1 Move（デフォルトの所有権移転）

すべての型はデフォルトで Move セマンティクスに従う。代入、引数渡し、戻り値 = 所有権移転。

```yaoxiang
p: Point = Point(1.0, 2.0)
q = p           // Move、p はもはや読めない
```

### 11.2 Dup（シャローコピー：ハンドルコピー、データ共有）

**Dup 属性は参照/トークン型に使用される**。Dup 型の代入 = シャローコピー——ハンドル/トークンをコピーし、底层のデータを共有する。複数の保有者が同一のデータブロックを指す。

| 型                 | 属性   | 説明                                                                      |
| ------------------ | ------ | ------------------------------------------------------------------------- |
| `&T`               | Dup    | ゼロサイズ読み取りトークン、トークンコピー = 複数の視点が同一データを指す |
| `ref T`            | Dup    | Rc/Arc コピー = 参照カウント+1、ヒープデータ共有                          |
| `&mut T`           | Linear | ゼロサイズ書き込みトークン、排他、コピー不可                              |
| その他のすべての型 | Move   | デフォルトの所有権移転                                                    |

**primitive 値型**（Int, Float, Bool,
Char）はコンパイラ組み込みの特殊処理：代入時に自動的に値コピーされ、二つの値は完全に独立。これはコンパイラのネイティブ動作であり、Dup 型属性には属さない。

```yaoxiang
// &T: Dup、自由にエイリアス可能
view: &Point = &p
view2 = view     // Dup：トークンコピー、両方とも有効
print(view.x)    // 使用可能
print(view2.x)   // 使用可能

// &mut T: Linear、コピー不可
mut_ref: &mut Point = &mut p
// r2 = mut_ref  // ❌ &mut T は Dup ではない、コピー不可
```

### 11.3 Clone（明示的ディープコピー）と Dup の関係

**Clone** は明示的ディープコピーインターフェース。すべての型が Clone を実装でき、`.clone()`
メソッドを提供する。

```yaoxiang
// Clone インターフェース定義（標準ライブラリ）
Clone: Type = {
    clone: () -> Clone
}

// 使用
p: Point = Point(1.0, 2.0)
backup = p.clone()    // ディープコピー、p は依然として使用可能
p2 = p.clone()        // 複数回クローン可能
```

**Dup と Clone の違い**：

|                  | Dup                                                     | Clone                                    |
| ---------------- | ------------------------------------------------------- | ---------------------------------------- |
| **意味**         | シャローコピー：ハンドル/トークンコピー、底层データ共有 | ディープコピー：完全独立の複製を作成     |
| **呼び出し方式** | 暗黙的（代入/引数渡しで自動）                           | 明示的（`.clone()`）                     |
| **変更の影響**   | 互いに影響する（底层データ共有）                        | 互いに影響しない（独立コピー）           |
| **適用型**       | `&T` トークン、`ref T`                                  | Clone インターフェースを実装する任意の型 |
| **コスト**       | ゼロオーバーヘッド（トークンはゼロサイズ型）            | 型による                                 |

**Dup は Clone を意味せず、Clone は Dup を意味しない**——これらは二つの直交する概念である：

```yaoxiang
// Dup 型：トークンコピー、底层データ共有
view: &Point = &p
view2 = view        // Dup：トークンコピー、両方とも同じ p を指す
print(view.x)       // 使用可能
print(view2.x)      // 使用可能、見るのは同じデータ

// primitive 値型：コンパイラが自動値コピー（Dup ではない）
x: Int = 42
y = x               // 値コピー、x と y は完全に独立
print(x)            // 使用可能

// Clone：明示的ディープコピー、独立コピー作成
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone：ディープコピー、p は依然として使用可能
r = p               // Move：所有権移転、Point は Dup でも primitive 値型でもないため
```

**設計意図**：

- Dup はトークン/参照型に使用され、「複数の視点で同一データを見る」問題を解決する
- Clone は独立コピーが必要なシナリオに使用され、明示的呼び出しによりコストを可視化する
- primitive 値型（Int/Float/Bool/Char）のコピーはコンパイラの組み込み動作であり、Dup には属さない
- ほとんどのカスタム型はデフォルトで Move となり、ゼロコピーで高性能

## 第十二章：借用トークン型

### 12.1 核心概念

`&T` と `&mut T`
は**ゼロサイズのコンパイル時トークン型**である。これらは「参照」ではなく、「アクセス権の型レベル証明」である。

```
&T      →  ゼロサイズ、ソースデータを凍結（期間中 WriteToken の取得を禁止）、
          凍結保証の下で複数の読み取り専用が安全 -> Dup（コピー可能）
&mut T  →  ゼロサイズ、排他的読み書き（他のすべてのトークンを禁止）、
          排他アクセス下ではコピー無意味 -> Linear（非 Dup）
```

**重要な特性**：

- トークンは**通常の型**であり、他のすべての型と同じスコープルールに従う
- ライフタイム注釈 `'a` は不要
- 専用の借用検査器は不要——型属性（Dup/Linear）が自然に権限を推論する
- コンパイル後完全に消滅し、ランタイムオーバーヘッドはゼロ

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
p.shift(1.0, 1.0)               // コンパイラが自動的に &mut Point トークンを作成
p.print()                       // OK、前のトークンは shift 呼び出し終了と共に解放済み

// 複数の &T トークンの共存——Dup 型は自由にコピー可能
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 トークンのスコープと伝播

トークンは通常の型であるため、通常の型のすべての操作をサポートする：

**トークンの返却**——トークンは戻り値と共に伝播する：

```yaoxiang
// ✅ 子トークンと親トークンは一緒に返される
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // トークンは呼び出し元に返される
print(px_ref)                    // OK、トークンはまだスコープ内
```

**構造体に格納**——構造体はトークンフィールドを保持できる：

```yaoxiang
// ✅ 構造体はトークンをフィールドとして保持する
Window: Type = {
    target: Point,
    view: &Point,              // トークンフィールド——target への読み取り専用ビューを保持
}
```

**クロージャはキャプチャせず、文脈は作成時点で固定**——クロージャは自身のパラメータのみを取り込み、外層データを必要とする場合はカリー化によって作成時点で値として固定される：

```yaoxiang
// ✅ 文脈はカリー化により固定：threshold はパラメータ、gt_point(threshold) は作成時点で値としてクロージャに固定される
gt_point: (t: Float) -> (p: Point) -> Bool = (p) => p.x > t
filter_by_threshold: (items: List(Point), threshold: Float) -> List(Point) = {
    items.filter(gt_point(threshold))
}
```

> 注：クロージャ（関数値）がエスケープした後、その定義箇所のスコープは既に死んでいる可能性があるため、外層変数を暗黙にキャプチャしてはならない；ただし呼び出し点（作成点）のスコープは必ず生存しており、その時点で文脈が値として固定されてクロージャに入るのは安全である。

### 12.4 自動借用選択

呼び出し側コンパイラは以下の優先順位で自動選択する：

```
1. 実引数が後にまだ使用される場合 -> 優先的にトークンを作成（&T または &mut T、メソッドシグネチャによる）
2. 実引数が後に使用されない場合 -> Move
3. 優先マッチング順序：&T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // print のパラメータ型は &Point -> コンパイラが &Point トークンを作成
p.shift(1.0, 1.0)  // shift のパラメータ型は &mut Point -> コンパイラが &mut Point トークンを作成
p2 = p             // 後に使用されない -> Move
```

**メソッドレシーバはシグネチャセマンティクスに従う**（訂正 2026-08-30、RFC-011a レシーバスペル規則と同種）：レシーバが
`&T` → 読み取り専用借用トークン；`&mut T` → 可変借用トークン；値渡し →
Move（レシーバ消費）。呼び出し点で生成された借用トークンは呼び出し終了と共に解放される（transient、§12.5 区間セマンティクス）；インターフェースの借用レシーバはインターフェース作者が
`&Self` を明示宣言し、impl シグネチャは `Self ↦ impl 型`
置換後インターフェースと完全一致する必要がある（RFC-011a §3）。

### 12.5 トークン衝突検出

トークン衝突検出は**借用ホーア命題**（RFC-009a）であり、独立したフロー依存分析ではない。コンパイラが借用命題（`borrow_conflict`/`use_after_move`/`use_after_drop`/`mut_violation`）を自動生成して証明パイプラインに送り検証する；トークン活性は区間
`[created_at, last_use]`（RFC-009a §逆 BFS 活性分析を参照）：

```yaoxiang
// ❌ &mut と派生の &T は同時にアクティブになれない
bad_alias: (p: &mut Point) -> Void = {
    p.x = 10.0                   // ✅ WriteToken 通常使用
    print(p.y)
}

// ✅ トークンスコープ終了後に自動解放
good_seq: (p: &mut Point) -> Void = {
    {
        // 内側スコープ
        print(p.x)               // &mut Point を使用
    }
    // 内側スコープ終了
    p.x = 10.0                   // ✅ WriteToken はまだ使用可能
}

// ❌ 同じ実引数から &mut トークンと他のトークンを同時に作成できない
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p から同時に &mut と & トークンを派生
```

### 12.6 コンパイラ内部：ブランド機構

ユーザはブランドに決して触れない。コンパイラが内部で各トークンにコンパイル時一意識別子を割り当てる：

```
ユーザが見るもの        コンパイラ内部表現
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N はコンパイル時一意整数
&mut Point     →  WriteToken(Point, #M)   // #M はコンパイル時一意整数
```

ブランドの用途：

- **偽造防止**：トークンは所有カプセルからのみ取得でき、凭空に構築できない
- **関連追跡**：フィールドアクセスから派生した `&Float`
  は派生ブランド（`#N.field_x`）を運び、コンパイラは親トークンまで追跡可能
- **衝突検出**：同源の WriteToken と派生 ReadToken は同時にアクティブになれない

ブランドは単態化とインライン化後に完全に消滅し、生成された機械語には存在しない。**ランタイムオーバーヘッドはゼロ。**

### 12.7 トークン Sum 型

```
&BorrowToken ::= &T          // ReadToken（ソースデータ凍結 -> Dup 安全）
               | &mut T      // WriteToken（排他読み書き -> Linear）
```

### 12.8 借用トークン vs ref

|            | `&T` / `&mut T`                                      | `ref`                               |
| ---------- | ---------------------------------------------------- | ----------------------------------- |
| すること   | 一目見る/その場で変更                                | 共有保持                            |
| 範囲       | トークン値のスコープに従う                           | スコープ横断                        |
| コスト     | ゼロオーバーヘッド（ゼロサイズ型、コンパイル後消滅） | Rc または Arc（コンパイラが選択）   |
| エスケープ | 可（トークンは戻り値/構造体で伝播）                  | 本来エスケープ用                    |
| タスク横断 | 不可（トークンはタスク横断渡し未実装）               | 可（コンパイラが自動的に Arc 選択） |
| 環検出     | 関与しない                                           | タスク内は静默、タスク横断は lint   |

> 注（未定義）：ref 作成後にどのように内容を読み取るか（デリファレンス/メソッド/自動）はまだ規範で未定義、実装現状では
> `*a` は E1052 を報告する。定義後に本節に追記する。

---

## 付録：型定義クイックリファレンス

### A.1 型定義

```
// === 記録型（波括弧） ===

// 記録型
Point: Type = { x: Float, y: Float }

// バリアント付き記録型（関数フィールドを使用）
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

// === primitive 値型（コンパイラ組み込み） ===
Int, Float,     // 代入時に自動値コピー、二つの値は完全に独立
Bool, Char      // Dup ではなく、コンパイラの primitive に対する組み込み処理

// === Dup（シャローコピー：ハンドルコピー、底层データ共有） ===
&T              // ゼロサイズ読み取りトークン、トークンコピー = 複数の視点が同一データを指す
ref T           // Rc/Arc コピー = 参照カウント+1、ヒープデータ共有

// === Linear ===
&mut T          // ゼロサイズ書き込みトークン、Linear（排他、コピー不可）

// === Clone（明示的ディープコピー） ===
value.clone()   // 独立コピーを作成、変更は元値に影響しない
```

### A.4 借用トークンクイックリファレンス

```
// === 借用トークン ===
&T              // ゼロサイズコンパイル時読み取りトークン、ソースデータ凍結 -> Dup（コピー可能）
&mut T          // ゼロサイズコンパイル時書き込みトークン、排他読み書き -> Linear（コピー不可）

// 呼び出し側の自動選択
// 1. 実引数が後にまだ使用される -> トークン作成
// 2. 実引数が後に使用されない -> Move
// 3. 優先マッチング：&T < &mut T < Move

// トークン伝播
// ✅ 返却可、構造体格納可、クロージャキャプチャ可
// ❌ タスク横断不可（トークンはタスク横断渡し未実装）
```

# 型システム仕様

本文書は YaoXiang プログラミング言語の型システム仕様を定義する。基本型、複合型、ジェネリクス、trait を含む。

---

## 第零章：基礎理論

### 0.1 Curry-Howard 同型

Curry-Howard 同型（Curry-Howard
correspondence）は YaoXiang 型システムの理論的基礎である。これはプログラミング言語の型システムと数理論理学の間の深い対応関係を明らかにする：

| 論理学                         | プログラミング言語                              |
| ------------------------------ | ----------------------------------------------- |
| 命題 \(P\)                     | 型 `Type`                                       |
| 証明 \(p: P\)                  | プログラム `x: T = ...`                         |
| 含意 \(P \rightarrow Q\)       | 関数型 `(P) -> Q`                               |
| 連言 \(P \wedge Q\)            | 積型 `{ a: P, b: Q }`                           |
| 選言 \(P \vee Q\)              | 和型 `{ a(P) \| b(Q) }`                         |
| 全称量化 \(\forall x:T. P(x)\) | ジェネリクス `(T: Type) -> ...`                 |
| 真 \(\top\)                    | `Void`（Unit、デフォルト値を持つ）              |
| 偽 \(\bot\)                    | `Never`（ゼロコンストラクタ、居住可能な値なし） |
| 型宇宙 \(Type_n : Type_{n+1}\) | 宇宙階層化（Russell パラドックスを防ぐ）        |
| case 分析                      | 型レベル `match`                                |

> **注意**：型レベル `match` は場合分け（case
> analysis）であり、数学的帰納法ではない。帰納法には型レベル再帰関数 + コンパイラの停止性検査が必要。

### 0.2 型は命題、プログラムは証明

YaoXiang において、この対応関係は設計の最上位原則である：

- **停止する型レベル計算は正しい構成的証明に対応する**。YaoXiang の型族（例えば `Nat` 上の `Add`
  の case 分析 + 再帰呼び出し）は本質的に数学的帰納法の型レベルエンコーディングである——ただし、コンパイラが停止性検査を行えることが前提。
- **型検査は証明の検証である**。プログラムが型検査を通過するとき、論理命題が構成的に証明されたことと等価である。

### 0.3 言語設計への影響

Curry-Howard 同型の YaoXiang における具体的な現れ：

1. **宇宙階層化**（RFC-010）：`Type₀ : Type₁ : Type₂ …` は `Type: Type`
   に起因する論理矛盾（Girard パラドックス）を回避
2. **型族**（RFC-011）：自然数 `Nat(Zero/Succ)`
   の型レベル case 分析 + 再帰呼び出しは Peano 公理に対応——コンパイラが停止性検査を行うことが前提
3. **条件型**（RFC-011）：`If: (C: Bool, T: Type, E: Type) -> Type` は論理の case 析取に対応
4. **値依存型**（RFC-011）：`Array: (T: Type, N: Int) -> Type`
   は「各整数 N に対し型が存在する」の有量化に対応

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

> **設計説明**：RFC-010 は「全てが代入」の統一モデル（`name: type = value`）を提案しているが、構文レベルでは型と値を区別する必要がある。コンパイラ実装では
> `Type` と `Expr` は独立した 2 つの AST 列挙型（`ast.rs:406` と `ast.rs:25`）であり、`TypeExpr`
> は BNF のプレースホルダとして実装の `Type` 列挙型に対応し、「この位置は型を期待する」を表す。

---

## 第二章：基本型

### 2.1 プリミティブ型

| 型       | 論理対応     | 説明                                                                                          | デフォルトサイズ |
| -------- | ------------ | --------------------------------------------------------------------------------------------- | ---------------- |
| `Type`   | —            | メタ型                                                                                        | 0 バイト         |
| `Never`  | ⊥（偽/空型） | ゼロコンストラクタ、値は存在しない。発散/panic の戻り型。`Never <: T` は任意の T に対し成立。 | 0 バイト         |
| `Void`   | ⊤（真/Unit） | デフォルト void 値を持つゼロフィールド積型。`x: Void = <デフォルト>` が合法。                 | 0 バイト         |
| `Bool`   | —            | ブール値：`true` / `false`                                                                    | 1 バイト         |
| `Int`    | —            | 符号付き整数                                                                                  | 8 バイト         |
| `Uint`   | —            | 符号なし整数                                                                                  | 8 バイト         |
| `Float`  | —            | 浮動小数点数                                                                                  | 8 バイト         |
| `String` | —            | UTF-8 文字列                                                                                  | 可変             |
| `Char`   | —            | Unicode 文字                                                                                  | 4 バイト         |
| `Bytes`  | —            | 生のバイト列                                                                                  | 可変             |

ビット幅付き整数：`Int8`, `Int16`, `Int32`, `Int64`, `Int128` ビット幅付き浮動小数点：`Float32`,
`Float64`

### 2.2 Never と Void：⊥ と ⊤

`Never` と `Void` は型システムの論理プリミティブであり、偽（⊥）と真（⊤）に対応する。

**Never（⊥、偽/空型）** — 譲歩不能な 3 つの性質：

1. **ゼロコンストラクタ**：リテラルも式も `Never` 型の値を生成できない。`x: Never = ...`
   には右辺が書けない。
2. **爆発原理**：`Never <: T` は任意の型 `T` に対し成立。`assert(false)` は `Never`
   を返し、後続コードは型検査を通る（実際には到達しないが）。
3. **発散マーカー**：`f: (...) -> Never` は `f`
   が戻らないことを保証する。コンパイラはこれにより dead code 解析と `match` 分岐合流を行う。

`Never` は組み込み型名（`Int`/`Bool` と同じ登録パス）であり、キーワードではない。

**Void（⊤、真/Unit）** — ちょうど 1 つの居住者（デフォルト void 値）を持つ。`Void`
はゼロフィールド積型の単位元である。`x: Void = <デフォルト>`
が合法。ブロックの値は**末尾式**で与えられる（空ブロック `{}` は `Void`）。詳細は
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

- レコード型は中括弧 `{}` で定義
- フィールド名の後にコロンと型を続ける
- 型本体内でのインターフェース名は当該インターフェースの実装を表す

> **名前空間归属**：`Type.name` 接頭辞（例：`Point.draw`）は関数が `Point`
> の名前空間に属することを示すだけで、いかなる暗黙の束縛も引き起こさない。`p.draw()` のような `.`
> 呼び出し構文を有効にするには、明示的な束縛が必要：`Point.draw = draw[0]`。RFC-004 と RFC-010 を参照。

#### 3.1.1 フィールドのデフォルト値

型フィールドにはデフォルト値を指定でき、構築時には任意で提供可能：

```yaoxiang
// デフォルト値を持つフィールド - 構築時は任意
Point: Type = {
    x: Float = 0,
    y: Float = 0
}

// 使用
Point()           // -> Point(x=0, y=0)
Point(x=1)       // -> Point(x=1, y=0)
Point(x=1, y=2) // -> Point(x=1, y=2)

// デフォルト値なしのフィールド - 構築時必須
Point2: Type = {
    x: Float,
    y: Float
}

// 使用
Point2(x=1, y=2) // 正しい
Point2()          // エラー
```

**ルール**：

- `field: Type = expression` -> デフォルト値あり、構築時は任意
- `field: Type` -> デフォルト値なし、構築時必須

#### 3.1.2 組み込みバインディング

型定義本体内で直接メソッドを束縛できる：

```yaoxiang
// 方法 1：外部関数を参照してバインディング
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]    // 位置 0 にバインディング
}
// 呼び出し：p1.distance(p2) -> distance(p1, p2)

// 方法 2：無名関数 + 位置バインディング
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

**構文**：インターフェースは全フィールドが関数型であるレコード型

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

**インターフェース実装**：型は定義末尾にインターフェース名を列挙することで実装する

```yaoxiang
// インターフェースを実装する型
Point: Type = {
    x: Float,
    y: Float,
    Drawable,        // Drawable インターフェースを実装
    Serializable     // Serializable インターフェースを実装
}
```

**インターフェースへの直接代入**：具象型はインターフェース型変数に直接代入可能（構造的部分型）

```yaoxiang
// 直接代入（コンパイル時に具象型が確定 -> ゼロオーバーヘッド呼び出し）
d: Drawable = Circle(1)
d.draw(screen)        // コンパイル後：circle_draw を直接呼び出し、vtable なし

// 関数の戻り値（コンパイル時に確定不能 -> vtable 呼び出し）
d: Drawable = get_shape()
d.draw(screen)        // vtable でメソッドを検索

// 関数の引数としてのインターフェース
process: (d: Drawable) -> Void = d.draw(screen)
```

**コンパイル時最適化戦略**：

| シナリオ         | 推論結果     | 呼び出し方式                       |
| ---------------- | ------------ | ---------------------------------- |
| 具象型の直接代入 | 具象型が確定 | 直接呼び出し（ゼロオーバーヘッド） |
| 関数の戻り値     | 不明         | vtable                             |
| 異種コレクション | 複数型       | vtable                             |

**コヒーレンスと孤立ルール（不適用、収束説明）**：YaoXiang のインターフェースは構造的型（インターフェース = 全フィールドが関数型のレコード）であり、名目的 trait ではない——crate/モジュール横断の「誰が誰に対して実装可能か」という帰属問題が存在しないため、Rust 式の孤立ルールとコヒーレンス検査は適用対象を持たない（裁決記録は RFC-011
§2.1）。構造的世界の対応する保障は**重複実装の拒否**である：同一のメソッドシグネチャの型上での重複定義はコンパイルエラーとなる（RFC-011a
§3、上書き禁止；オーバーロードのみ合法）。

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

ジェネリックパラメータは関数型の一部であり、通常のパラメータと統一して `()` 構文を用いる：

```
GenericType     ::= Identifier '(' TypeArgList ')'
TypeArgList     ::= TypeExpr (',' TypeExpr)* ','?
TypeBound       ::= Identifier
                 |  Identifier '+' Identifier ('+' Identifier)*
```

ジェネリック型定義において、`(T: Type)` は型コンストラクタのパラメータシグネチャであり、`-> Type`
は戻り型を表す：

```yaoxiang
List: (T: Type) -> Type = { ... }
Map: (K: Type, V: Type) -> Type = { ... }
```

### 4.1.1 コンテナ型

コンテナ型はジェネリック型コンストラクタであり、組み込みプリミティブではない——ユーザ定義ジェネリックと同一の扱いを受け、统一されたジェネリックインスタンス化経路で処理される。長さ情報の帰属が 3 つのコンテナ概念の根本的な違いである：

| 型            | 長さ         | セマンティクス                       | 基盤                                        |
| ------------- | ------------ | ------------------------------------ | ------------------------------------------- |
| `Array(T, N)` | 型           | 固定長配列（const ジェネリック N）   | コアプリミティブ（スタック/インライン優先） |
| `Vec(T)`      | ランタイム値 | ランタイム長さの生バッファ、伸長可能 | コアプリミティブ（ヒープ上の連続バッファ）  |
| `List(T)`     | ランタイム値 | 標準ライブラリ型（伸長可能リスト）   | ライブラリ：`{ data: Vec(T), length: Int }` |
| `Dict(K, V)`  | ランタイム値 | キー値マッピング                     | `HeapValue::Dict`                           |

> `List(T)` は**標準ライブラリ型であり、コンパイラプリミティブではない**：YaoXiang 自身により
> `std.list`
> で定義され、ユーザ定義ジェネリックレコードと同等の扱いを受ける。伸長可能セマンティクスの全戦略（いつ拡張するか、いくつ拡張するか、共有可能か）はすべてライブラリ内にあり、コンパイラは関与しない。
> `Vec(T)` はそれが依存する最小の基盤プリミティブである。
>
> Set(T) は既に削除：リテラルなし、ランタイム表現なし、std.set なし。要件が発生した時点で Dict パターンに従い補完する。

重要なルール：

- **リテラルの帰属はコンテキストが決定**：`[...]` 素のリテラルと `List(T)`
  注釈は伸長可能リストに帰属；`Array(T, N)`
  注釈がリテラルに直接作用する場合は固定長配列に帰属する。帰属検証：要素数 ==
  N、要素型が T と互換。不適合はコンパイル時 E1002；N がシンボル定数（const パラメータ）の場合、要素数検証は型精化段階まで延期。
- **暗黙の List→Array 変換を禁止**：固定長性は型層で保証される——push は `List(A)`
  レシーバのみ受け付ける。
- **パフォーマンス階層**：底辺から上に向かってパフォーマンス递减、柔軟性递增：`Array` > `Vec` >
  `List`。
- **インデックス失敗契約**（ランタイムエラーは過渡状態、目標状態はコンパイル時精化でカバー、値依存型に従う、§8.4 参照）：
  - インデックス範囲外（負のインデックスを含む）→ `E6003`
  - Dict キー欠落 → `E6008`
- **membership `in` 述語**：`Bool`
  を返しエラーなし、右オペランドは List/Array/Dict(キー)/Tuple/String/Range をカバー。一級ホール述語であり、精化型のコンパイル時証明可能命題の基底である。`

ジェネリック関数において、型パラメータも同様にシグネチャで宣言され、コンパイラは実引数から自動推論する：

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

**`?` 伝播と `Try` インターフェース**：`expr?` はインターフェース駆動のエラー伝播である——受信者型は
`Try` インターフェース（4 メンバ `is_failure` / `success` / `residual` /
`from_error`）を実装している必要があり、失敗時は `from_error(residual(t))`
の結果を現在の関数から早期リターンし、成功時は式の値が `success(t)` となる。外側関数の戻り型も `Try`
を実装し、その失敗残余型は受信者と一致する必要がある（`E1081` / `E1082` / `E1083`）。`Result(T, E)`
と `Option(T)` は `std.result` / `std.option` により `Try` 実装を提供する（`Option` の失敗残余型は
`Void`）；ユーザ定義の和型は型本体内に `Try(自身, T, E)` と書くだけで `?` に接続できる。 `?`
の lowering は組み込みとユーザ型を区別しない——同一インターフェース、同一経路。

和型の**変種集合は定義または `use` インポートによりチェッカに登録される**：`Result`/`Option`
は使用前に `use std.result` / `use std.option` が必要（モジュール全体とグループ `use std.{...}`
も同権）、変種構築（型修飾形式）、match 変種分解、網羅性判定はすべてこの登録を唯一の判拠とする；native 関数シグネチャが返す
`Result(Float, Error)` は同名の `Generic` を生成し、 `std.result`
の和型と名前で同一アイデンティティを持ち、`use` 後に match 分解可能。

### 4.3 ジェネリック構築呼び出しと型推論

ジェネリック型定義のフィールドリストは**コンストラクタを自動生成する**：各フィールドが構築パラメータに対応し、フィールド名がそのままパラメータ名になる；デフォルト値を持つフィールドは構築時に省略可能で、デフォルト値なしは必須。関数型フィールド（メソッド）は構築パラメータを生成しない。

```yaoxiang
// 型定義
Container: (T: Type) -> Type = {
    value: T,        // デフォルト値なし -> 構築パラメータ必須
    extra: T,
}
// 自動展開された完全形式（コンパイラ内部ビュー、ユーザの手書きは不要）：
// Container: (T: Type) -> (value: T, extra: T) -> Type = {
//     value: T = value,
//     extra: T = extra,
// }

// 呼び出し：自動生成されたコンストラクタを呼び出す
c  = Container(42, 43)            // 構築パラメータをフィールド順に渡す；T は要素から自動展開 = Int
c2 = Container("a", "b")          // T = String
c3 = Container(Int)(42, 43)       // 明示的な型パラメータ + 位置式構築パラメータ
c4 = Container(Int)(extra=43, value=42)  // フィールド名式、順序任意
c5 = Container(Int)()             // 空構築：フィールドはデフォルト値/ゼロ値を取る（データは事後代入）

// フィールドデフォルト値 -> 構築パラメータは省略可能
Point: (T: Type) -> Type = { x: T = 0, y: T = 0 }
p  = Point(1.5, 2.5)              // T = Float、x←1.5, y←2.5
p2 = Point(Int)()                 // x=0, y=0
```

**呼び出しルール**（単一括弧、宣言パラメータを逐次マッチング、左から右）：

1. 実引数を逐次的に型宣言パラメータにマッチング試行：`Type`
   位置は型実引数を受け付け、コンパイル時値パラメータ位置（例：`Int`）はコンパイル時定数を受け付ける。
2. コンパイル時値パラメータ位置への部分マッチングが成功した場合、型構築として処理：全パラメータ位置を逐次チェックし、エラー時は宣言順に**最初の不整合/欠落パラメータを先に報告**。
3. 実引数が宣言パラメータに完全対応しない場合（全値が対象、コンパイル時値パラメータ位置へのマッチングなし）、構築パラメータとして処理：位置式はフィールド順に代入、型パラメータは要素型から自動展開。

```yaoxiang
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),
    data: Array(Array(T, Cols), Rows),
}

m: Matrix(Int, 3, 4)              // 型位置：一層型構築
m2 = Matrix(Int, 3, 4)(data=[[1,2,3,4],[5,6,7,8],[9,10,11,12]])  // 二層：型 + 構築パラメータ
m3 = Matrix(Int, 3, 4)()          // 空構築（RFC-011 §9.3 モード、データは事後代入）

Matrix(42)    // ❌ 位置 0: T←42 不整合（42 は型ではない）；位置 1: Rows←42 整合；
              //    位置 2: Cols 欠落 -> 最初のエラーを先に報告：T は Type を期待、42 を受領
Container(42) // ❌ 構築パラメータ extra 欠落
Container(42, 43, 44)  // ❌ 構築パラメータ超過
```

**型推論**：ジェネリック型コンストラクタの型パラメータは構築パラメータ要素から自動展開（`Container(42, 43)`
→ T=Int）；ジェネリック関数の型パラメータは実引数型から自動展開（`map(numbers, f)` → T=Int,
R=String、§4.1 参照）。展開不能時は明示的指定が必須。

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

> **制約の解法ソース（RFC-011b）**：演算子制約名（`Add` / `Subtract` / `Multiply` / `Divide` /
> `Modulo` / `Equal` / `Index`）の解法 = インターフェース実装登録表の検索—— `T: Add` ≜ 登録済み
> `Add(T, T, T)` インスタンス化；`Equal`
> は別途構造推論（全フィールドが比較可能なレコードは自動比較可能）。`Zero` / `One` / `PartialOrd`
> 等の名前はまだ定義ソースがなく、宙ぶらりん制約名である。

```yaoxiang
// 多重制約の構文
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

> 判定基準は**型位置で参照されるか**であり、「具体的な型が注釈されているか」ではない：`add: (a: Int, b: Int) -> Int = a + b`
> において `a`/`b` はランタイム値パラメータである（どちらも型位置に登場しない）。

**用語**：`Type`
以外の具体型（例：`Int`）で注釈されたジェネリックパラメータを**コンパイル時値パラメータ候補**と呼び、コンパイル時値パラメータになるかは値が型位置で参照されるか（値依存）による。**`const`
キーワードは不要**
（実装内部でかつて「const ジェネリクス」と呼んでいたが、ドキュメントは「コンパイル時値パラメータ」に統一する）。

**判定ルール（2 ステップ）**：

1. **形態粗選別**：パラメータ注釈が `Type` 以外の具体型（`Int`/`Bool`/`Float`）-> 候補。
2. **用途精選別**：候補名が**型位置**に登場（型本体フィールド型、内層 `Fn` パラメータ型、`Assert`
   述語、`Array(T, N)`
   型構築実引数位置）-> 真のコンパイル時値パラメータ；そうでなければ**ランタイム値パラメータ**。

| 書き方                                                     | 判定                          | 理由                            |
| ---------------------------------------------------------- | ----------------------------- | ------------------------------- |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b ランタイム値パラメータ    | 値位置にのみ登場                |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N コンパイル時値パラメータ    | N が型構築実引数位置に登場      |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N コンパイル時値パラメータ    | N が内層パラメータ k の型になる |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N 落空→ランタイム値パラメータ | N が型本体で参照されない        |

**中核設計**：`(N: Int)` コンパイル時値パラメータ + `(k: N)`
値パラメータにより、コンパイル時定数とランタイム値を区別する。落空候補（形態は候補だが用途が未命中）はランタイム値パラメータに退化する——関数レベルと型コンストラクタパスの両方がこの処理に従う。

```yaoxiang
// コンパイル時値パラメータ：N が型位置（Array 長さスロット）で参照される
Measure: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // N が型構築実引数位置に登場 -> コンパイル時値パラメータ
    length: N
}

// 使用方法：factorial(5) は型位置で評価（コンパイル時）され、結果 120 が型に埋め込まれる
arr: Measure(Int, factorial(5))  // コンパイラがコンパイル時に factorial(5) = 120 を計算

// 値依存：N が内層パラメータ k の型になる
// N はコンパイル時値パラメータ（(k: N) の型位置に登場）；
// k はランタイム値パラメータ、その型はリテラル型 N（単値型）。
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

### 8.3 Assert 精化型と assert 表明

`assert` と `Assert`
は同一精化プリミティブの二面であり、dispatch 分岐パイプラインにより「述語の自由変数がコンパイル時に到達可能か」で自動選択される。

**中核シグネチャ**：`assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**dispatch 分岐ルール**：

| 判定基準                                                                 | モード      | 振る舞い                                                                           |
| ------------------------------------------------------------------------ | ----------- | ---------------------------------------------------------------------------------- |
| 全自由変数がコンパイル時既知（ジェネリックパラメータ、コンパイル時定数） | CompileTime | 証明パイプラインへ：true → Void に消去、false → コンパイルエラー（Never 居住不能） |
| ランタイム自由変数が存在（関数パラメータ、外部入力）                     | Runtime     | ランタイム Bool 検査を挿入し、フロー敏感仮説集合 Γ に精化事実を注入                |

**フロー敏感仮説集合 Γ**：

コンパイラは各制御点での既知命題集合を保持する：

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP 伝播
mut x = x - 5       // Γ = {}  ← mut kill set：旧仮説失効
```

`mut` 変数代入後、当該変数に関する全仮説が削除される（kill
set）。分岐合流時 Γ は各分岐の交差を取る。

### 8.4 Terminates：停止測度述語

`Terminates` は**組み込み述語**であり、`Int`、`Never`
と同じくコアプリミティブ（組み込み名であり、キーワードではない）に属する。これは**測度**を計算に紐付け、当該計算が停止することと、その停止の証人を表明する。

**形態**：同じ述語の 2 つのアリティ：

| 形態                    | アンカー             | 用途                                                                             |
| ----------------------- | -------------------- | -------------------------------------------------------------------------------- |
| `Terminates(m)`         | 所在バインディング名 | デフォルト形態——自己再帰関数、ループ                                             |
| `Terminates(FnType, m)` | 明示的関数型         | 測度の帰属を明示する必要がある場合（測度が別所で定義、同じ測度が複数計算に奉仕） |

```yaoxiang
// 測度：通常関数、ユニットテスト可能、再利用可能、ランタイムに参加しない
gcd_measure: (a: Int, b: Int) -> Int = { b }

// 二項形態：測度が別所で定義、帰属を明示
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

// 一項形態：アンカーがバインディング名そのもの、測度はスコープ内の式
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n {
        i = i + 1
    }
    return acc
}
```

**セマンティクス**：`Terminates(m)`
が精化するのは、それが所在する型位置に注釈された計算の値型である。義務はその計算に落ちる——関数の場合は各再帰呼び出し点に、ループの場合は後退辺に——いずれも「次の状態の測度が現在の状態の測度より厳密に小さい」ことであり、当該点のパスガード下で判定される。

呼び出し点の義務は `m(callee_args) < m(caller_args)`；後退辺の義務は
`m(次のラウンド) < m(本ラウンド)`。両者は形式上同一。

> **なぜループもカバーするか**：ループは匿名構造であり、通常は指示できない。バインディング名がすなわち名前である——`acc: Terminates(n - i) = while ...`
> の `acc` がアンカーを提供し、ループはこうして指示可能になる。これが `Terminates`
> の一項形態がループに作用する理由である。

**測度**：戻り型を制限しない（自然数を強制しない）；その上の「厳密減少」は当該型で利用可能な整礎順序により与えられる。測度が整礎であるか（例：`Int`
を返すときに `>= 0`
か）は**独立した義務**であり、減少義務と同様にコンパイル時証明パイプラインで判定される。

**トリガ**：停止検査は**精化型**によりトリガされる——型が一度精化されれば即座に検証モードに入る。精化されていない通常の型（例：素の
`while` ループ、精化シグネチャのない関数）は検証モードに入らず、停止義務を生成しない。

**自動探索優先**：コンパイラはまず測度を自動探索（線形ランク関数、述語違反カウンタ、有界増減、乗法スケールの 4 テンプレート）し、探索失敗時にのみ明示的な
`Terminates` が必要。

**ランタイム表現**：純粋にコンパイル時エンティティであり、witness 消去と共にランタイムバイナリに参加しない。

> 完整設計は
> [RFC-027 §6.9](../../design/rfc/accepted/027-compile-time-evaluation-types.md)（セマンティクス）と
> [RFC-027a](../../design/rfc/review/027a-termination-explicit-measure.md)（実装メカニズム）を参照。

---

## 第九章：型の和集合と交差

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
// インターフェース組み合わせ = 型交差
DrawableSerializable: Type = Drawable & Serializable

// 交差型の使用
process: (T: Drawable & Serializable)(item: T, screen: Surface) -> String = {
    item.draw(screen)
    return item.serialize()
}
```

---

## 第十章：関数のオーバーロードと特殊化

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
// プラットフォーム型列挙（標準ライブラリ定義）
Platform: Type = { X86_64: () -> Platform, AArch64: () -> Platform, RISC_V: () -> Platform, ARM: () -> Platform, X86: () -> Platform }

// P は事前定義済みジェネリックパラメータ名、現在のコンパイルプラットフォームを表す
sum: (P: X86_64)(arr: Array(Float)) -> Float = {
    return avx2_sum(arr.data, arr.length)
}

sum: (P: AArch64)(arr: Array(Float)) -> Float = {
    return neon_sum(arr.data, arr.length)
}
```

---

## 第十一章：型の属性

YaoXiang には区別すべき型属性が 1 種類だけある：線形 vs 複製可能。コンパイラが自動推論する。

### 11.1 Move（デフォルトの所有権移転）

全型はデフォルトで Move セマンティクスに従う。代入、引数渡し、戻り値 = 所有権移転。

```yaoxiang
p: Point = Point(1.0, 2.0)
q = p           // Move、p は以降読み取り不可
```

### 11.2 Dup（シャローコピー：ハンドル複製、データ共有）

**Dup 属性は参照/トークン型に用いる**。Dup 型の代入 = シャローコピー——ハンドル/トークンを複製し、底层のデータを共有する。複数の保持者が同一データブロックを指す。

| 型         | 属性   | 説明                                                              |
| ---------- | ------ | ----------------------------------------------------------------- |
| `&T`       | Dup    | ゼロサイズ読み取りトークン、トークン複製 = 同じデータへの複数視点 |
| `ref T`    | Dup    | Rc/Arc 複製 = 参照カウント+1、ヒープデータ共有                    |
| `&mut T`   | Linear | ゼロサイズ書き込みトークン、排他的、複製不可                      |
| その他全型 | Move   | デフォルト所有権移転                                              |

**プリミティブ値型**（Int, Float, Bool,
Char）はコンパイラ組み込みの特殊処理：代入時に自動で値コピーされ、2 つの値は完全に独立。これはコンパイラのネイティブ動作であり、Dup 型属性には属さない。

```yaoxiang
// &T: Dup、自由なエイリアシングが可能
view: &Point = &p
view2 = view     // Dup：トークン複製、両者とも有効
print(view.x)    // 使用可能
print(view2.x)   // 使用可能

// &mut T: Linear、複製不可
mut_ref: &mut Point = &mut p
// r2 = mut_ref  // ❌ &mut T は Dup ではないため複製不可
```

### 11.3 Clone（明示的ディープコピー）と Dup の関係

**Clone** は明示的ディープコピーインターフェース。すべての型は Clone を実装でき、`.clone()`
メソッドを提供する。

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

|                    | Dup                                                   | Clone                                    |
| ------------------ | ----------------------------------------------------- | ---------------------------------------- |
| **セマンティクス** | シャローコピー：ハンドル/トークン複製、底层データ共有 | ディープコピー：完全独立複製を作成       |
| **呼び出し方式**   | 暗黙的（代入/引数渡しで自動）                         | 明示的（`.clone()`）                     |
| **変更影響**       | 相互に影響（底层データ共有）                          | 互いに影響なし（独立複製）               |
| **適用型**         | `&T` トークン、`ref T`                                | Clone インターフェースを実装する任意の型 |
| **コスト**         | ゼロオーバーヘッド（トークンはゼロサイズ型）          | 型による                                 |

**Dup は Clone を蕴含せず、Clone も Dup を蕴含しない**——両者は直交する概念：

```yaoxiang
// Dup 型：トークン複製、底层データ共有
view: &Point = &p
view2 = view        // Dup：トークン複製、両者は同じ p を指す
print(view.x)       // 使用可能
print(view2.x)      // 使用可能、見るのは同じデータ

// プリミティブ値型：コンパイラが自動値コピー（Dup ではない）
x: Int = 42
y = x               // 値コピー、x と y は完全に独立
print(x)            // 使用可能

// Clone：明示的ディープコピー、独立複製を作成
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone：ディープコピー、p は引き続き使用可能
r = p               // Move：所有権移転、Point は Dup でもプリミティブ値型でもない
```

**設計意図**：

- Dup はトークン/参照型に用い、「同一データに対する複数視点」の問題を解決
- Clone は独立複製が必要なシナリオに用い、明示的呼び出しでコストを可視化
- プリミティブ値型（Int/Float/Bool/Char）の複製のコピーはコンパイラの組み込み動作であり、Dup に属さない
- 大多数のカスタム型はデフォルトで Move、ゼロコピーで高性能

## 第十二章：借用トークン型

### 12.1 中核概念

`&T` と `&mut T`
は**ゼロサイズのコンパイル時トークン型**である。これは「参照」ではなく、「アクセス権限の型レベル証明」である。

```
&T      →  ゼロサイズ、ソースデータを凍結（期間中 WriteToken 取得禁止）、
          凍結保証下で複数読取が安全 -> Dup（複製可能）
&mut T  →  ゼロサイズ、排他的読み書き（他の全トークン禁止）、
          排他的アクセス下で複製は無意味 -> Linear（非 Dup）
```

**キー特性**：

- トークンは**通常の型**であり、他のすべての型と同じスコープルールに従う
- ライフタイム注釈 `'a` は不要
- 専用の借用チェッカー不要——型属性（Dup/Linear）が自然に権限を推論
- コンパイル後完全に消滅、ランタイムオーバーヘッドゼロ

### 12.2 基本使用

```yaoxiang
// メソッド端：パラメータ型を宣言し、必要な権限を決定
Point.print: (self: &Point) -> Void = {
    print(self.x)               // &Point トークンが読取権限を付与
    print(self.y)
}

Point.shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
    self.x = self.x + dx        // &mut Point トークンが書込権限を付与
    self.y = self.y + dy
}

// 呼び出し端：コンパイラが借用または Move を自動選択
p = Point(1.0, 2.0)
p.print()                       // コンパイラが自動的に &Point トークンを作成
p.shift(1.0, 1.0)               // コンパイラが自動的に &mut Point トークンを作成
p.print()                       // OK、前のトークンは shift 呼び出し終了と共に解放済み

// 複数の &T トークン共存——Dup 型は自由複製を許可
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 トークンのスコープと伝播

トークンは通常の型であるため、通常の型の全操作をサポートする：

**トークンを返す**——トークンは戻り値と共に伝播：

```yaoxiang
// ✅ 子トークンと親トークンを一緒に返す
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // トークンを呼び出し元に返す
print(px_ref)                    // OK、トークンは依然スコープ内
```

**構造体に格納**——構造体はトークンフィールドを持てる：

```yaoxiang
// ✅ 構造体がトークンをフィールドとして保持
Window: Type = {
    target: Point,
    view: &Point,              // トークンフィールド——target への読取ビューを保持
}
```

**クロージャはキャプチャせず、コンテキストは作成時点で固定**——クロージャは自分のパラメータのみを取り込み、外部データを必要とするときはカリー化により作成時点で値をクロージャ内に固定する：

```yaoxiang
// ✅ コンテキストはカリー化で固定：threshold はパラメータ、gt_point(threshold) は作成時点で値をクロージャ内に固定
gt_point: (t: Float) -> (p: Point) -> Bool = (p) => p.x > t
filter_by_threshold: (items: List(Point), threshold: Float) -> List(Point) = {
    items.filter(gt_point(threshold))
}
```

> 注：クロージャ（関数値）がエスケープした後、その定義箇所のスコープは既に死んでいる可能性があるため、外側の変数を暗黙的にキャプチャしてはならない；ただし呼び出し点（作成点）のスコープは必ず生存しているため、コンテキストがその時点で値としてクロージャに固定されることは安全である。

### 12.4 自動借用選択

呼び出し端のコンパイラは以下の優先順位で自動選択する：

```
1. 実引数が後に使用される -> トークン作成を優先（&T または &mut T、メソッドシグネチャに応じて）
2. 実引数が以降使用されない -> Move
3. 優先マッチング順序：&T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // print のパラメータ型は &Point -> コンパイラが &Point トークンを作成
p.shift(1.0, 1.0)  // shift のパラメータ型は &mut Point -> コンパイラが &mut Point トークンを作成
p2 = p             // 以降使用されない -> Move
```

**メソッドレシーバはシグネチャセマンティクスに従う**（RFC-011a レシーバスペル约定に同じ）：レシーバが
`&T` -> 読取借用トークン；`&mut T` -> 可変借用トークン；値渡し ->
Move（レシーバ消費）。呼び出し点で生成された借用トークンは呼び出し終了と共に解放される（transient、§12.5 区間セマンティクス）；インターフェースの借用レシーバはインターフェース作者が
`&Self` を明示宣言し、impl シグネチャは `Self ↦ impl 型`
置換後にインターフェースと完全一致する必要がある（RFC-011a §3）。

### 12.5 トークン衝突検出

トークン衝突検出は**借用ホール命題**（RFC-009a）であり、独立したフロー敏感分析ではない。コンパイラは借用命題を自動生成（`borrow_conflict`/`use_after_move`/`use_after_drop`/`mut_violation`）し、証明パイプラインに投入して検証する；トークン活性は区間
`[created_at, last_use]`（RFC-009a §逆 BFS 活性分析参照）：

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
    p.x = 10.0                   // ✅ WriteToken は依然使用可能
}

// ❌ 同一実引数から &mut トークンと他のトークンを同時に生成不可
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p から同時に &mut と & トークンを派生
```

### 12.6 コンパイラ内部：ブランド機構

ユーザはブランドに一切触れない。コンパイラは内部で各トークンにコンパイル時一意識別子を割り当てる：

```
ユーザに見える           コンパイラ内部表現
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N はコンパイル時一意整数
&mut Point     →  WriteToken(Point, #M)   // #M はコンパイル時一意整数
```

ブランドの用途：

- **偽造防止**：トークンは所有者カプセルからのみ取得でき、凭空に構築不可
- **関連追跡**：フィールドアクセスから派生した `&Float`
  は派生ブランド（`#N.field_x`）を運び、コンパイラは親トークンまで追跡可能
- **衝突検出**：同源 WriteToken と派生 ReadToken は同時にアクティブにできない

ブランドは単態化とインライン展開後に完全に消滅し、生成された機械語には存在しない。**ランタイムオーバーヘッドゼロ。**

### 12.7 トークン Sum 型

```
&BorrowToken ::= &T          // ReadToken（ソースデータ凍結 -> Dup 安全）
               | &mut T      // WriteToken（排他的読み書き -> Linear）
```

### 12.8 借用トークン vs ref

|            | `&T` / `&mut T`                                      | `ref`                             |
| ---------- | ---------------------------------------------------- | --------------------------------- |
| 役割       | 一目見る/その場で変更                                | 共有保持                          |
| 範囲       | トークン値のスコープに従う                           | スコープをまたぐ                  |
| コスト     | ゼロオーバーヘッド（ゼロサイズ型、コンパイル後消滅） | Rc または Arc（コンパイラが選択） |
| エスケープ | 可能（トークンは戻り値/構造体で伝播）                | そもそもエスケープ用              |
| タスク横断 | 不可（トークンはタスク横断未実装）                   | 可（コンパイラが自動 Arc 選択）   |
| 環検出     | 関与なし                                             | タスク内静默、タスク横断 lint     |

> 注（未定義）：ref 作成後の内容読み取り（逆参照/メソッド/自動）方法はまだ規範で未定義、実装現状の
> `*a` は E1052 を報告。定義後に本節に補完。

---

## 付録：型定義クイックリファレンス

### A.1 型定義

```
// === レコード型（中括弧） ===

// レコード型
Point: Type = { x: Float, y: Float }

// バリアント付きレコード型（関数字段を使用）
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// === インターフェース型（中括弧、全フィールドが関数） ===

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

// 一項：アンカーがバインディング名そのもの（自己再帰関数、ループ）
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n { i = i + 1 }
    return acc
}

// 二項：測度帰属を明示（測度が別所で定義）
gcd_measure: (a: Int, b: Int) -> Int = { b }
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}
```

### A.2 ジェネリック構文

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

// コンパイル時ジェネリクス：N が型位置 (k: N) で参照 -> コンパイル時値パラメータ
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
// 全型はデフォルトで Move。代入、引数渡し、戻り値 = 所有権移転

// === プリミティブ値型（コンパイラ組み込み） ===
Int, Float,     // 代入時に自動値コピー、2 つの値は完全に独立
Bool, Char      // Dup ではなく、コンパイラのプリミティブ組み込み処理

// === Dup（シャローコピー：ハンドル複製、底层データ共有） ===
&T              // ゼロサイズ読取トークン、トークン複製 = 同じデータへの複数視点
ref T           // Rc/Arc 複製 = 参照カウント+1、ヒープデータ共有

// === Linear ===
&mut T          // ゼロサイズ書込トークン、Linear（排他的、複製不可）

// === Clone（明示的ディープコピー） ===
value.clone()   // 独立複製を作成、変更は原値に影響しない
```

### A.4 借用トークンクイックリファレンス

```
// === 借用トークン ===
&T              // ゼロサイズコンパイル時読取トークン、ソースデータ凍結 -> Dup（複製可能）
&mut T          // ゼロサイズコンパイル時書込トークン、排他的読み書き -> Linear（複製不可）

// 呼び出し端の自動選択
// 1. 実引数が後に使用される -> トークン作成
// 2. 実引数が以降使用されない -> Move
// 3. 優先マッチング：&T < &mut T < Move

// トークン伝播
// ✅ 戻り値可、構造体格納可、クロージャで取得可
// ❌ タスク横断不可（トークンはタスク横断未実装）
```

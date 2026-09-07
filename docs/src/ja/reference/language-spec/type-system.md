# 型システム仕様

本文書は YaoXiang プログラミング言語の型システム仕様を定義する。基本型、複合型、ジェネリクス、trait を含む。

---

## 第零章：理論的基礎

### 0.1 Curry-Howard 同型

Curry-Howard 同型（Curry-Howard
correspondence）は YaoXiang 型システムの理論的基礎である。これはプログラミング言語の型システムと数理論理の間の深層対応関係を明らかにする：

| 論理学                         | プログラミング言語                              |
| ------------------------------ | ----------------------------------------------- |
| 命題 \(P\)                     | 型 `Type`                                       |
| 証明 \(p: P\)                  | プログラム `x: T = ...`                         |
| 含意 \(P \rightarrow Q\)       | 関数型 `(P) -> Q`                               |
| 連言 \(P \wedge Q\)            | 積型 `{ a: P, b: Q }`                           |
| 選言 \(P \vee Q\)              | 和型 `{ a(P) \| b(Q) }`                         |
| 全称量化 \(\forall x:T. P(x)\) | ジェネリクス `(T: Type) -> ...`                 |
| 真 \(\top\)                    | `Void`（Unit、デフォルト値あり）                |
| 偽 \(\bot\)                    | `Never`（ゼロコンストラクタ、居住可能な値なし） |
| 型宇宙 \(Type_n : Type_{n+1}\) | 宇宙階層（Russell のパラドックス防止）          |
| case 分析                      | 型レベル `match`                                |

> **注意**：型レベル `match` は場合分け（case
> analysis）であり、数学的帰納法ではない。帰納法には型レベル再帰関数とコンパイラの停止性検査が必要となる。

### 0.2 型は命題、プログラムは証明

YaoXiang において、この対応関係は設計の最上位原則である：

- **停止する型レベル計算は正しい構成的証明に対応する**。YaoXiang の型族（`Nat` 上の `Add`
  における case 分析 + 再帰呼び出しなど）は本質的に数学的帰納法の型レベルエンコーディングである——ただしコンパイラが停止性検査を行えることが前提となる。
- **型検査は証明の検証である**。プログラムが型検査を通過するということは、論理命題が構成的に証明されたことに相当する。

### 0.3 言語設計への影響

YaoXiang における Curry-Howard 同型の具体例：

1. **宇宙階層**（RFC-010）：`Type₀ : Type₁ : Type₂ …` は `Type: Type`
   による論理的パラドックス（Girard のパラドックス）を回避する
2. **型族**（RFC-011）：自然数 `Nat(Zero/Succ)`
   の型レベル case 分析 + 再帰呼び出しは Peano 公理に対応する——ただしコンパイラが停止性検査を行うことが前提
3. **条件型**（RFC-011）：`If: (C: Bool, T: Type, E: Type) -> Type` は論理の case 選言に対応する
4. **値依存型**（RFC-011）：`Vec: (n: Int) -> Type`
   は「各整数 n に対して型が存在する」という有界量化に対応する

---

## 第一章：型分類

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

> **設計説明**：RFC-010 は「すべて代入である」という統一モデル（`name: type = value`）を提案しているが、構文の層では型と値を区別する必要がある。コンパイラ実装では
> `Type` と `Expr` は独立した二つの AST enum であり（`ast.rs:406` と `ast.rs:25`）、`TypeExpr`
> は BNF プレースホルダとして実装の `Type` enum に対応し、「この位置は型を期待する」ことを表す。

---

## 第二章：基本型

### 2.1 プリミティブ型

| 型       | 論理対応     | 説明                                                                                    | デフォルトサイズ |
| -------- | ------------ | --------------------------------------------------------------------------------------- | ---------------- |
| `Type`   | —            | メタ型                                                                                  | 0 バイト         |
| `Never`  | ⊥（偽/空型） | ゼロコンストラクタ、値なし。発散/panic の戻り型。`Never <: T` が任意の T に対して成立。 | 0 バイト         |
| `Void`   | ⊤（真/Unit） | デフォルト void 値を持つ、ゼロフィールド積型。`x: Void = <デフォルト>` が合法。         | 0 バイト         |
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

`Never` と `Void` は型システムの論理プリミティブであり、偽（⊥）と真（⊤）にそれぞれ対応する。

**Never（⊥、偽/空型）** — 譲歩できない三つの性質：

1. **ゼロコンストラクタ**：リテラルや式で `Never` 型の値を生成することはできない。`x: Never = ...`
   の右辺には何も書けない。
2. **爆発原理**：`Never <: T` が任意の型 `T` に対して成立する。`assert(false)` は `Never`
   を返し、その後のコードは型検査を通過できる（実際には実行されないが）。
3. **発散マーカー**：`f: (...) -> Never` は `f`
   が決して戻らないことを示す。コンパイラはこれに基づきデッドコード解析と `match` 分岐合流を行う。

`Never` は組み込み型名（`Int`/`Bool` と同じ登録パス）であり、キーワードではない。

**Void（⊤、真/Unit）** — ちょうど一つの居住者（デフォルト void 値）。`Void`
はゼロフィールド積型の単位元である。`x: Void = <デフォルト>` が合法であり、関数がデフォルトで
`return` を持たない場合は `Void` を返す。

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

// ジェネリクス付き記録型
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

- 記録型は波括弧 `{}` で定義される
- フィールド名の後にコロンと型を続ける
- インターフェース名は型体内に書くことで、そのインターフェースの実装を表す

> **名前空間の所属**：`Type.name` プレフィックス（例：`Point.draw`）は関数が `Point`
> の名前空間に属することを示す。これは暗黙のバインディングを引き起こさない。`p.draw()` のような `.`
> 呼び出し構文を有効にするには、明示的なバインディングが必要：`Point.draw = draw[0]`。詳細は RFC-004 および RFC-010 を参照。

#### 3.1.1 フィールドのデフォルト値

型のフィールドにはデフォルト値を指定でき、構築時にオプションで提供できる：

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

// デフォルト値のないフィールド - 構築時に必須
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

型定義体内では直接メソッドをバインドできる：

```yaoxiang
// 方法1：外部関数の参照バインディング
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]    // 位置0にバインド
}
// 呼び出し：p1.distance(p2) -> distance(p1, p2)

// 方法2：無名関数 + 位置バインディング
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance: ((a: Point, b: Point) -> Float)[0] = ((a, b) => {
        dx = a.x - b.x
        dy = b.y - a.y
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

**インターフェースへの直接代入**：具象型はインターフェース型変数に直接代入できる（構造的サブタイピング）

```yaoxiang
// 直接代入（コンパイル時に具象型を決定可能 -> ゼロオーバーヘッド呼び出し）
d: Drawable = Circle(1)
d.draw(screen)        // コンパイル後：circle_draw を直接呼び出し、vtable なし

// 関数の戻り値（コンパイル時に決定不可能 -> vtable 呼び出し）
d: Drawable = get_shape()
d.draw(screen)        // vtable 経由でメソッドを検索

// 関数引数としてのインターフェース
process: (d: Drawable) -> Void = d.draw(screen)
```

**コンパイル時最適化戦略**：

| シナリオ         | 推論結果         | 呼び出し方式                       |
| ---------------- | ---------------- | ---------------------------------- |
| 具象型の直接代入 | 具象型を決定可能 | 直接呼び出し（ゼロオーバーヘッド） |
| 関数の戻り値     | 不明             | vtable                             |
| 異種コレクション | 複数の型         | vtable                             |

**コヒーレンスとオーファンルール（適用外、結論の明記）**：YaoXiang のインターフェースは構造的型（インターフェース = フィールドがすべて関数型である記録）であり、名义的な trait ではない——クレート/モジュールを跨ぐ「誰が何のために実装できるか」という帰属問題は存在せず、Rust 式のオーファンルールとコヒーレンス検査は適用対象を持たない（裁決記録は RFC-011
§2.1）。構造的世界の対応する保証は**重複実装の拒否**である：同じメソッドシグネチャを型上で重複定義するとコンパイルエラーとなる（RFC-011a
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

### 4.1 ジェネリック引数の構文

ジェネリック引数は関数型の一部であり、通常の引数と統一して `()` 構文を用いる：

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

コンテナ型はジェネリック型コンストラクタであり、組み込みプリミティブではない——ユーザーが定義したジェネリクスと同じ扱いを受け、統一されたジェネリクスインスタンス化パスで処理される：

| 型            | セマンティクス                     | ベース             |
| ------------- | ---------------------------------- | ------------------ |
| `List(T)`     | 拡張可能なリスト                   | `HeapValue::List`  |
| `Array(T, N)` | 固定長配列（const ジェネリクス N） | `HeapValue::Array` |
| `Dict(K, V)`  | キーバリュー写像                   | `HeapValue::Dict`  |

> Set(T) は既に削除：リテラルなし、ランタイム表現なし、std.set なし。必要性が出現したら Dict のパターンに従って補完する。

重要なルール：

- **リテラルの配置は文脈が決定**：`[...]` 生のリテラルと `List(T)`
  注釈は拡張可能リストに配置；`Array(T, N)`
  注釈がリテラルに直接作用する場合は固定長配列に配置。配置検証：要素数 ==
  N、要素型が T と互換、不一致はコンパイル時 E1002；N がシンボル定数（const 引数）の場合、個数検査は型精化フェーズまで延期される。
- **暗黙の List→Array 変換を禁止**：固定長は型層で保証される——push は `List(A)`
  レシーバのみ受け入れる。
- **インデックス失敗契約**（ランタイムエラーは過渡的、目標はコンパイル時型精化カバー、値依存型に向かう）：
  - インデックス範囲外（負のインデックスを含む）→ `E6003`
  - Dict キー欠落 → `E6008`
- **membership `in` 述語**：`Bool`
  を返しエラーは出さない、右オペランドは List/Array/Dict(キー)/Tuple/String/Range をカバー。一級ホール述語、型精化コンパイル時証明可能命題の基底。`

ジェネリック関数では、型引数も同様にシグネチャで宣言され、コンパイラが実引数から自動推論する：

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
    push: (self: List(T), item: T) -> Void,   // self は単なる慣習名であり、キーワードではない
    get: (self: List(T), index: Int) -> Option(T)
}
```

### 4.3 ジェネリック構築呼び出しと型推論

ジェネリック型定義のフィールドリストは**自動的にコンストラクタを生成する**：各フィールドがコンストラクト引数に対応し、フィールド名が引数名となる；デフォルト値を持つフィールドは構築時に省略可能で、デフォルト値のないフィールドは必須。関数型フィールド（メソッド）はコンストラクト引数を生成しない。

```yaoxiang
// 型定義
Container: (T: Type) -> Type = {
    value: T,        // デフォルト値なし -> コンストラクト引数は必須
    extra: T,
}
// 自動展開された完全形式（コンパイラの内部ビュー、ユーザーが手書きする必要はない）：
// Container: (T: Type) -> (value: T, extra: T) -> Type = {
//     value: T = value,
//     extra: T = extra,
// }

// 呼び出し：自動生成されたコンストラクタの呼び出し
c  = Container(42, 43)            // コンストラクト引数はフィールド順；T は要素から自動推論 = Int
c2 = Container("a", "b")          // T = String
c3 = Container(Int)(42, 43)       // 明示的な型引数 + 位置式コンストラクト引数
c4 = Container(Int)(extra=43, value=42)  // フィールド名式、順序任意
c5 = Container(Int)()             // 空構築：フィールドはデフォルト値/ゼロ値を取る（データは後で代入）

// フィールドデフォルト値 -> コンストラクト引数は省略可能
Point: (T: Type) -> Type = { x: T = 0, y: T = 0 }
p  = Point(1.5, 2.5)              // T = Float、x←1.5, y←2.5
p2 = Point(Int)()                 // x=0, y=0
```

**呼び出しルール**（単一括弧、宣言引数と順番にマッチング、左から右）：

1. 実引数は位置ごとに型宣言引数との照合を試みる：`Type`
   の位置は型実引数を受け入れ、コンパイル時値引数位置（例：`Int`）はコンパイル時定数を受け入れる。
2. コンパイル時値引数位置のマッチが成功した場合（部分マッチ）、型構築として処理する：すべての引数位置を順番に検査し、エラー時は宣言順に従って**最初にマッチしない/欠落した引数を先に報告**する。
3. 実引数が宣言引数に完全に対応しない場合（すべて値で、コンパイル時値引数位置がない）、コンストラクト引数として処理する：位置式はフィールド順に従い、型引数は要素型から自動推論される。

```yaoxiang
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),
    data: Array(Array(T, Cols), Rows),
}

m: Matrix(Int, 3, 4)              // 型位置：一層型構築
m2 = Matrix(Int, 3, 4)(data=[[1,2,3,4],[5,6,7,8],[9,10,11,12]])  // 二層：型 + コンストラクト引数
m3 = Matrix(Int, 3, 4)()          // 空構築（RFC-011 §9.3 パターン、データは後で代入）

Matrix(42)    // ❌ 位置0: T←42 がマッチしない（42 は型ではない）；位置1: Rows←42 がマッチ；
              //    位置2: Cols が欠落 -> 最初のエラーを先に報告：T は Type を期待、42 が見つかった
Container(42) // ❌ コンストラクト引数 extra が欠落
Container(42, 43, 44)  // ❌ コンストラクト引数が超過
```

**型推論**：ジェネリック型コンストラクタの型引数はコンストラクト引数の要素から自動推論される（`Container(42, 43)`
→ T=Int）；ジェネリック関数の型引数は実引数の型から自動推論される（`map(numbers, f)` → T=Int,
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

### 7.1 コンパイル時値引数

```
LiteralType   ::= Identifier ':' Int          // コンパイル時定数（候補）
```

> **訂正**：原文ではコンパイル時値引数を「デフォルトでコンパイル時確定」と表現しているが、これは**厳密には誤り**である——
> `add: (a: Int, b: Int) -> Int = a + b` における `a`/`b`
> はランタイム値引数である。**型位置で参照される**具体的な型引数のみがコンパイル時値引数となる。正しい定義は以下を参照。

**用語**：`Type`
以外の具象型（例：`Int`）で注釈されたジェネリック引数は**コンパイル時値引数候補**と呼ばれ、コンパイル時値引数になるかどうかは、その値が型位置で参照されるか（値依存）によって決まる。**`const`
キーワードは不要**（実装内部では「const ジェネリクス」と呼んでいたが、ドキュメントでは統一して「コンパイル時値引数」を使用する）。

**判定ルール（二段階）**：

1. **形態による粗選別**：`Type` 以外の具象型（`Int`/`Bool`/`Float`）で注釈された引数 → 候補。
2. **用途による精選別**：候補名が**型位置**（型体内フィールド型、内層 `Fn` 引数型、`Assert`
   述語、`Array(T, N)`
   型構築実引数位置）に現れる → 真のコンパイル時値引数；そうでなければ**ランタイム値引数**。

| 書き方                                                     | 判定                      | 理由                          |
| ---------------------------------------------------------- | ------------------------- | ----------------------------- |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b はランタイム値引数    | 値位置にのみ出現              |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N はコンパイル時値引数    | N は型構築実引数位置          |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N はコンパイル時値引数    | N は内層引数 k の型として機能 |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N は落空→ランタイム値引数 | N は型体内で参照されていない  |

**核心的設計**：`(N: Int)` コンパイル時値引数と `(k: N)`
値引数を用いて、コンパイル時定数とランタイム値を区別する。落空した候補（形態は候補だが用途が未一致）はランタイム値引数に退化する——関数レベルと型コンストラクタパスの両方がこの扱いに従う。

```yaoxiang
// コンパイル時値引数：N は型位置（Array の長さスロット）で参照される
StaticArray: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // N は型構築実引数位置に現れる -> コンパイル時値引数
    length: N
}

// 使用方法：factorial(5) は型位置で評価され（コンパイル時）、結果 120 が型に埋め込まれる
arr: StaticArray(Int, factorial(5))  // コンパイラはコンパイル時に factorial(5) = 120 を計算

// 値依存：N は内層引数 k の型として機能
// N はコンパイル時値引数（(k: N) の型位置に現れる）；
// k はランタイム値引数、その型はリテラル型 N（単一値型）。
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
// IsTrue ブリッジと Assert 型精化（詳細は §8.3）
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

### 8.3 Assert 型精化と assert 表明

`assert` と `Assert`
は同じ精化プリミティブの二面であり、dispatch 分派パイプラインが「述語の自由変数がコンパイル時に到達可能か」に基づいて自動選択する。

**核心的シグネチャ**：`assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**dispatch 分派ルール**：

| 判定基準                                                                   | モード      | 振る舞い                                                                                 |
| -------------------------------------------------------------------------- | ----------- | ---------------------------------------------------------------------------------------- |
| すべての自由変数がコンパイル時に既知（ジェネリック引数、コンパイル時定数） | CompileTime | 証明パイプラインへ進む：true → Void に消去、false → コンパイルエラー（Never は居住不可） |
| ランタイム自由変数が存在する（関数引数、外部入力）                         | Runtime     | ランタイム Bool 検査を挿入し、フロー敏感仮定集合 Γ に精化事実を注入                      |

**フロー敏感仮定集合 Γ**：

コンパイラは各制御フロー点の既知命題集合を維持する：

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP 伝播
mut x = x - 5       // Γ = {}  ← mut kill set：古い仮定が無効化される
```

`mut` 変数への代入後、その変数に関するすべての仮定が削除される（kill
set）。分岐合流時、Γ は各分岐の積集合を取る。

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

**構文**：型積 `A & B` は A と B の両方を満たす型を表す

```yaoxiang
// インターフェース合成 = 型積
DrawableSerializable: Type = Drawable & Serializable

// 積型の使用
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

YaoXiang には区別すべき型属性が一つだけある：線形 vs コピー可能。コンパイラが自動推論する。

### 11.1 Move（デフォルトの所有権移転）

すべての型はデフォルトで Move セマンティクスに従う。代入、引数渡し、戻り値 = 所有権移転。

```yaoxiang
p: Point = Point(1.0, 2.0)
q = p           // Move、p はもはや読み取れない
```

### 11.2 Dup（シャローコピー：ハンドルのコピー、データの共有）

**Dup 属性は参照/トークン型に使用される**。Dup 型の代入 = シャローコピー——ハンドル/トークンをコピーし、基盤となるデータを共有する。複数の保持者が同じデータを指す。

| 型               | 属性   | 説明                                                                      |
| ---------------- | ------ | ------------------------------------------------------------------------- |
| `&T`             | Dup    | ゼロサイズ読み取りトークン、トークンコピー = 複数の視点が同じデータを指す |
| `ref T`          | Dup    | Rc/Arc コピー = 参照カウント+1、ヒープデータ共有                          |
| `&mut T`         | Linear | ゼロサイズ書き込みトークン、排他的、コピー不可                            |
| その他すべての型 | Move   | デフォルトの所有権移転                                                    |

**プリミティブ値型**（Int, Float, Bool,
Char）はコンパイラが組み込みで特別扱いする：代入時に自動的に値がコピーされ、二つの値は完全に独立する。これはコンパイラのネイティブ動作であり、Dup 型属性には属さない。

```yaoxiang
// &T: Dup、自由にエイリアス可能
view: &Point = &p
view2 = view     // Dup：トークンをコピー、両者とも有効
print(view.x)    // 使用可能
print(view2.x)   // 使用可能

// &mut T: Linear、コピー不可
mut_ref: &mut Point = &mut p
// r2 = mut_ref  // ❌ &mut T は Dup ではない、コピー不可
```

### 11.3 Clone（明示的なディープコピー）と Dup の関係

**Clone** は明示的なディープコピーインターフェースである。すべての型は Clone を実装でき、`.clone()`
メソッドを提供する。

```yaoxiang
// Clone インターフェース定義（標準ライブラリ）
Clone: Type = {
    clone: () -> Clone
}

// 使用
p: Point = Point(1.0, 2.0)
backup = p.clone()    // ディープコピー、p はまだ使用可能
p2 = p.clone()        // 複数回クローン可能
```

**Dup と Clone の違い**：

|                    | Dup                                                         | Clone                                    |
| ------------------ | ----------------------------------------------------------- | ---------------------------------------- |
| **セマンティクス** | シャローコピー：ハンドル/トークンをコピー、基盤データを共有 | ディープコピー：完全独立の複製を作成     |
| **呼び出し方式**   | 暗黙的（代入/引数渡しで自動）                               | 明示的（`.clone()`）                     |
| **変更の影響**     | 互いに影響（基盤データを共有）                              | 互いに影響しない（独立コピー）           |
| **適用型**         | `&T` トークン、`ref T`                                      | Clone インターフェースを実装する任意の型 |
| **コスト**         | ゼロオーバーヘッド（トークンはゼロサイズ型）                | 型による                                 |

**Dup は Clone を含意せず、Clone は Dup を含意しない**——これらは二つの直交する概念である：

```yaoxiang
// Dup 型：トークンをコピー、基盤データを共有
view: &Point = &p
view2 = view        // Dup：トークンをコピー、両者は同じ p を指す
print(view.x)       // 使用可能
print(view2.x)      // 使用可能、同じデータを見る

// プリミティブ値型：コンパイラが自動値コピー（Dup ではない）
x: Int = 42
y = x               // 値コピー、x と y は完全に独立
print(x)            // 使用可能

// Clone：明示的ディープコピー、独立した複製を作成
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone：ディープコピー、p はまだ使用可能
r = p               // Move：所有権移転、Point は Dup でもプリミティブ値型でもないため
```

**設計意図**：

- Dup はトークン/参照型に使用され、「同じデータの複数の視点」問題を解決する
- Clone は独立コピーが必要なシナリオに使用され、明示的な呼び出しによりコストを可視化する
- プリミティブ値型（Int/Float/Bool/Char）のコピーはコンパイラの組み込み動作であり、Dup には属さない
- ほとんどのカスタム型はデフォルトで Move となり、ゼロコピーの高性能を実現

## 第十二章：借用トークン型

### 12.1 核心的概念

`&T` と `&mut T`
は**ゼロサイズのコンパイル時トークン型**である。これらは「参照」ではなく、「アクセス権限の型レベル証明」である。

```
&T      →  ゼロサイズ、ソースデータを凍結（この期間の WriteToken 取得を禁止）、
          凍結保証下で複数の読み取り専用は安全 -> Dup（コピー可能）
&mut T  →  ゼロサイズ、排他的読み書き（他のすべてのトークンを禁止）、
          排他的アクセス下ではコピーは無意味 -> Linear（非 Dup）
```

**重要な特性**：

- トークンは**通常の型**であり、他のすべての型と同じスコープルールに従う
- ライフタイム注釈 `'a` は不要
- 専用の借用チェッカーは不要——型属性（Dup/Linear）が自然に権限を推論する
- コンパイル後に完全に消滅し、ランタイムオーバーヘッドはゼロ

### 12.2 基本使用

```yaoxiang
// メソッド端：引数型を宣言し、必要な権限を決定
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
p.print()                       // OK、前のトークンは shift 呼び出し終了とともに解放

// 複数の &T トークンの共存——Dup 型は自由にコピー可能
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 トークンのスコープと伝播

トークンは通常の型であるため、通常の型のすべての操作をサポートする：

**トークンの返却**——トークンは戻り値とともに伝播する：

```yaoxiang
// ✅ 子トークンと親トークンを一緒に返す
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // トークンは呼び出し元に返される
print(px_ref)                    // OK、トークンはまだスコープ内
```

**構造体への保存**——構造体はトークンフィールドを保持できる：

```yaoxiang
// ✅ 構造体がトークンをフィールドとして保持
Window: Type = {
    target: Point,
    view: &Point,              // トークンフィールド——target への読み取り専用ビューを保持
}
```

**クロージャはキャプチャせず、コンテキストは作成時点で固定化**——クロージャは自身の引数のみを取得し、外側データが必要な場合はカリー化により作成時点で値として固定化する：

```yaoxiang
// ✅ コンテキストがカリー化で固定化：threshold は引数、gt_point(threshold) は作成時点で値を固定化
gt_point: (t: Float) -> (p: Point) -> Bool = (p) => p.x > t
filter_by_threshold: (items: List(Point), threshold: Float) -> List(Point) = {
    items.filter(gt_point(threshold))
}
```

> 注：クロージャ（関数値）がエスケープした後、その定義箇所のスコープは既に死んでいる可能性があるため、外側変数を暗黙的にキャプチャしてはならない；しかし呼び出し点（作成点）のスコープは必ず生存しており、その時点でコンテキストが値として固定化されてクロージャに入ることは安全である。

### 12.4 自動借用選択

呼び出し端で、コンパイラは以下の優先順位に従って自動的に選択する：

```
1. 実引数の後に使用がある場合 -> トークン作成を優先（&T または &mut T、メソッドシグネチャによる）
2. 実引数の後に使用がない場合 -> Move
3. 優先マッチ順序：&T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // print の引数型は &Point -> コンパイラが &Point トークンを作成
p.shift(1.0, 1.0)  // shift の引数型は &mut Point -> コンパイラが &mut Point トークンを作成
p2 = p             // 後の使用なし -> Move
```

**メソッドレシーバはシグネチャセマンティクスに従う**（訂正 2026-08-30、RFC-011a レシーバスペル约定と同型）：レシーバが
`&T` -> 読み取り専用借用トークン；`&mut T` -> 可変借用トークン；値渡し ->
Move（レシーバを消費）。呼び出し点で生成された借用トークンは呼び出し終了とともに解放される（transient、§12.5 区間セマンティクス）；インターフェースの借用レシーバはインターフェース作者が
`&Self` と明示的に宣言し、impl シグネチャは `Self ↦ impl 型`
置換後、インターフェースと完全一致しなければならない（RFC-011a §3）。

### 12.5 トークン競合検出

トークン競合検出は**借用ホール命題**（RFC-009a）であり、独立したフロー敏感解析ではない。コンパイラが借用命題を自動生成し（`borrow_conflict`/`use_after_move`/`use_after_drop`/`mut_violation`）、証明パイプラインに送って検証する；トークン活性は区間
`[created_at, last_use]` である（RFC-009a §逆 BFS 活性解析を参照）：

```yaoxiang
// ❌ &mut と派生の &T は同時にアクティブにできない
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
alias_bad(p, p)                  // ❌ p は同時に &mut と & トークンを派生
```

### 12.6 コンパイラ内部：ブランド機構

ユーザーはブランドに決して触れない。コンパイラは内部で各トークンにコンパイル時一意の識別子を割り当てる：

```
ユーザーが見る           コンパイラ内部表現
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N はコンパイル時一意の整数
&mut Point     →  WriteToken(Point, #M)   // #M はコンパイル時一意の整数
```

ブランドの用途：

- **偽造防止**：トークンは所有者カプセルからのみ取得でき、凭空に構築できない
- **関連追跡**：フィールドアクセスから派生した `&Float`
  は派生ブランド（`#N.field_x`）を持ち、コンパイラは親トークンまで追跡できる
- **競合検出**：同源の WriteToken と派生 ReadToken は同時にアクティブにできない

ブランドは単態化とインライン化後に完全に消滅し、生成された機械語には存在しない。**ランタイムオーバーヘッドはゼロ。**

### 12.7 トークン Sum 型

```
&BorrowToken ::= &T          // ReadToken（ソースデータを凍結 -> Dup 安全）
               | &mut T      // WriteToken（排他的読み書き -> Linear）
```

### 12.8 借用トークン vs ref

|              | `&T` / `&mut T`                                      | `ref`                                 |
| ------------ | ---------------------------------------------------- | ------------------------------------- |
| 役割         | 一目見る/その場で変更                                | 共有所有                              |
| 範囲         | トークン値のスコープに従う                           | スコープを跨ぐ                        |
| コスト       | ゼロオーバーヘッド（ゼロサイズ型、コンパイル後消滅） | Rc または Arc（コンパイラが選択）     |
| エスケープ   | 可（トークンは戻り値/構造体で伝播）                  | 本来エスケープ用                      |
| タスクを跨ぐ | 不可（トークンはタスク間渡し未実装）                 | 可（コンパイラが自動的に Arc を選択） |
| 環検出       | 該当なし                                             | タスク内では無音、タスク間は lint     |

> 注（未定義）：ref 作成後の内容の読み取り（デリファレンス/メソッド/自動）はまだ仕様で定義されておらず、実装現状では
> `*a` は E1052 を報告する。定義後に本節に補足する。

---

## 付録：型定義クイックリファレンス

### A.1 型定義

```
// === 記録型（波括弧） ===

// 記録型
Point: Type = { x: Float, y: Float }

// バリアント付き記録型（関数フィールドを使用）
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

// コンパイル時ジェネリクス：N は型位置 (k: N) で参照される -> コンパイル時値引数
factorial: (N: Int)(k: N) -> Int = { ... }
StaticArray: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }

// 条件型
If: (C: Bool, T: Type, E: Type) -> Type = match C { True => T, False => E }

// 関数特化
sum: (arr: Array(Int)) -> Int = { ... }
sum: (arr: Array(Float)) -> Float = { ... }
```

### A.3 型属性クイックリファレンス

```
// === Move（デフォルト） ===
// すべての型はデフォルトで Move。代入、引数渡し、戻り値 = 所有権移転

// === プリミティブ値型（コンパイラ組み込み） ===
Int, Float,     // 代入時に自動的に値がコピーされ、二つの値は完全に独立
Bool, Char      // Dup ではなく、コンパイラのプリミティブに対する組み込み処理

// === Dup（シャローコピー：ハンドルをコピー、基盤データを共有） ===
&T              // ゼロサイズ読み取りトークン、トークンコピー = 複数の視点が同じデータを指す
ref T           // Rc/Arc コピー = 参照カウント+1、ヒープデータ共有

// === Linear ===
&mut T          // ゼロサイズ書き込みトークン、Linear（排他的、コピー不可）

// === Clone（明示的ディープコピー） ===
value.clone()   // 独立した複製を作成、変更は元の値に影響しない
```

### A.4 借用トークンクイックリファレンス

```
// === 借用トークン ===
&T              // ゼロサイズコンパイル時読み取りトークン、ソースデータを凍結 -> Dup（コピー可能）
&mut T          // ゼロサイズコンパイル時書き込みトークン、排他的読み書き -> Linear（コピー不可）

// 呼び出し端の自動選択
// 1. 実引数の後に使用がある場合 -> トークン作成
// 2. 実引数の後に使用がない場合 -> Move
// 3. 優先マッチ：&T < &mut T < Move

// トークン伝播
// ✅ 返却可能、構造体に保存可能、クロージャにキャプチャ可能
// ❌ タスクを跨げない（トークンはタスク間渡し未実装）
```

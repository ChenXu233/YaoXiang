# 型システム仕様

本文書はYaoXiangプログラミング言語の型システム仕様を定義する。基本型、複合型、ジェネリクス、traitを含む。

---

## 第零章：理論的基礎

### 0.1 Curry-Howard対応

Curry-Howard対応（Curry-Howard
correspondence）はYaoXiangの型システムの理論的基礎である。これはプログラミング言語の型システムと数理論理学の間の深層対応関係を明らかにする：

| 論理学                           | プログラミング言語                        |
| -------------------------------- | ----------------------------------------- |
| 命題 \(P\)                       | 型 `Type`                                 |
| 証明 \(p: P\)                    | プログラム `x: T = ...`                   |
| 含意 \(P \rightarrow Q\)         | 関数型 `(P) -> Q`                         |
| 連言 \(P \wedge Q\)              | 積型 `{ a: P, b: Q }`                     |
| 選言 \(P \vee Q\)                | 和型 `{ a(P) \| b(Q) }`                   |
| 全称量化 \(\forall x:T. P(x)\)   | ジェネリクス `(T: Type) -> ...`           |
| 真 \(\top\)                      | `Void`（Unit、デフォルト値を持つ）        |
| 偽 \(\bot\)                      | `Never`（零コンストラクタ、値を持たない） |
| 型の宇宙 \(Type_n : Type_{n+1}\) | 宇宙階層（Russellパラドックス防止）       |
| 場合分け                         | 型レベル `match`                          |

> **注意**：型レベル `match` は場合分け（case
> analysis）であり、数学的帰納法ではない。帰納法には型レベル再帰関数とコンパイラの停止性検査が必要である。

### 0.2 型は命題、プログラムは証明

YaoXiangにおいて、この対応関係は設計の第一級の原則である：

- **停止する型レベル計算は正しい構成的証明に対応する**。YaoXiangの型族（例えば `Nat` 上の `Add`
  のcase分析 + 再帰呼び出し）は本質的に数学的帰納法の型レベルエンコードである——前提としてコンパイラが停止性検査を行えること。
- **型検査は証明の検証である**。あるプログラムが型検査を通過することは、論理的命題が構成的に証明されたことに相当する。

### 0.3 言語設計への影響

Curry-Howard対応のYaoXiangにおける具体的な現れ：

1. **宇宙階層**（RFC-010）：`Type₀ : Type₁ : Type₂ …` は `Type: Type`
   に起因する論理的パラドックス（Girardのパラドックス）を回避する
2. **型族**（RFC-011）：自然数 `Nat(Zero/Succ)`
   の型レベルcase分析 + 再帰呼び出しはPeano公理に対応する——前提としてコンパイラが停止性検査を行うこと
3. **条件型**（RFC-011）：`If: (C: Bool, T: Type, E: Type) -> Type` は論理におけるcase選言に対応する
4. **値依存型**（RFC-011）：`Array: (T: Type, N: Int) -> Type`
   は「各整数Nに対して型が存在する」という有界量化に対応する

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

> **設計説明**：RFC-010は「すべて代入である」という統一モデル（`name: type = value`）を提案しているが、構文レベルでは型と値は依然として区別される必要がある。コンパイラ実装では
> `Type` と `Expr` は2つの独立したAST列挙型（`ast.rs:406` と `ast.rs:25`）であり、`TypeExpr`
> はBNFプレースホルダとして実装中の `Type` 列挙型に対応し、「この位置は型を期待する」を表す。

---

## 第二章：基本型

### 2.1 プリミティブ型

| 型       | 論理的対応   | 説明                                                                          | デフォルトサイズ |
| -------- | ------------ | ----------------------------------------------------------------------------- | ---------------- |
| `Type`   | —            | メタ型                                                                        | 0バイト          |
| `Never`  | ⊥（偽/空型） | 零コンストラクタ、値なし。発散/panicの戻り型。`Never <: T` は任意のTで成立。  | 0バイト          |
| `Void`   | ⊤（真/Unit） | デフォルトのvoid値を持つ、零フィールド積型。`x: Void = <デフォルト>` は合法。 | 0バイト          |
| `Bool`   | —            | ブール値：`true` / `false`                                                    | 1バイト          |
| `Int`    | —            | 符号付き整数                                                                  | 8バイト          |
| `Uint`   | —            | 符号なし整数                                                                  | 8バイト          |
| `Float`  | —            | 浮動小数点数                                                                  | 8バイト          |
| `String` | —            | UTF-8文字列                                                                   | 可変             |
| `Char`   | —            | Unicode文字                                                                   | 4バイト          |
| `Bytes`  | —            | 生バイト                                                                      | 可変             |

ビット幅付き整数：`Int8`, `Int16`, `Int32`, `Int64`, `Int128` ビット幅付き浮動小数点：`Float32`,
`Float64`

### 2.2 NeverとVoid：⊥と⊤

`Never` と `Void` は型システムの論理的プリミティブ——それぞれ偽（⊥）と真（⊤）に対応する。

**Never（⊥、偽/空型）** — 譲歩できない3つの性質：

1. **零コンストラクタ**：リテラルや式で `Never` 型の値を生成することはできない。`x: Never = ...`
   の右辺に書けるものはない。
2. **爆発原理**：`Never <: T` は任意の型 `T` に対して成立する。`assert(false)` は `Never`
   を返し、その後のコードは型検査を通過できる（実際には実行されないが）。
3. **発散マーカー**：`f: (...) -> Never` は `f`
   が戻らないことを保証する。コンパイラはこれに基づいてデッドコード解析と `match` 分岐合流を行う。

`Never` は組み込み型名（`Int`/`Bool` と同じ登録パス）であり、キーワードではない。

**Void（⊤、真/Unit）** — 正確に1つの居住者（デフォルトvoid値）を持つ。`Void`
は零フィールド積型の単位元である。`x: Void = <デフォルト>`
は合法。ブロックの値は**末尾式**で与えられる（空ブロック `{}` は `Void`）、詳細は
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
- フィールド名の直後にコロンと型を続ける
- インターフェース名は型本体内に書いてそのインターフェースの実装を表す

> **名前空間の帰属**：`Type.name` 接頭辞（例：`Point.draw`）は関数が `Point`
> の名前空間に属することを示す。これは暗黙のバインディングを発生させない。`p.draw()` のような `.`
> 呼び出し構文を有効にするには、明示的なバインディングが必要：`Point.draw = draw[0]`。詳細はRFC-004およびRFC-010を参照。

#### 3.1.1 フィールドのデフォルト値

型フィールドにはデフォルト値を指定でき、構築時にはオプションで提供できる：

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

// デフォルト値がないフィールド - 構築時に必須
Point2: Type = {
    x: Float,
    y: Float
}

// 使用
Point2(x=1, y=2) // 正
Point2()          // エラー
```

**ルール**：

- `field: Type = expression` -> デフォルト値あり、構築時はオプション
- `field: Type` -> デフォルト値なし、構築時は必須

#### 3.1.2 組み込みバインディング

型定義本体内で直接メソッドをバインドできる：

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
    Drawable,        // Drawableインターフェースを実装
    Serializable     // Serializableインターフェースを実装
}
```

**インターフェース直接代入**：具象型はインターフェース型変数に直接代入できる（構造的部分型付け）

```yaoxiang
// 直接代入（コンパイル時に具象型を決定可能 -> ゼロオーバーヘッド呼び出し）
d: Drawable = Circle(1)
d.draw(screen)        // コンパイル後：circle_drawを直接呼び出し、vtableなし

// 関数の戻り値（コンパイル時に決定不可 -> vtable呼び出し）
d: Drawable = get_shape()
d.draw(screen)        // vtableでメソッドを検索

// インターフェースを関数の引数として
process: (d: Drawable) -> Void = d.draw(screen)
```

**コンパイル時最適化戦略**：

| シナリオ           | 推論結果         | 呼び出し方式                       |
| ------------------ | ---------------- | ---------------------------------- |
| 具象型への直接代入 | 具象型を決定可能 | 直接呼び出し（ゼロオーバーヘッド） |
| 関数の戻り値       | 不明             | vtable                             |
| 異種コレクション   | 複数型           | vtable                             |

**コヒーレンスとオーファンルール（適用外、収束説明）**：YaoXiangのインターフェースは構造的型（インターフェース = フィールドがすべて関数型であるレコード）であり、名目的なtraitではない——クレート/モジュール横断の「誰が何のために実装できるか」という帰属問題は存在せず、Rustのオーファンルールとコヒーレンスチェックには適用対象がない（裁定記録はRFC-011
§2.1参照）。構造的世界の対応する保障は**重複実装の拒否**：同一メソッドシグネチャを型に重複定義するとコンパイルエラー（RFC-011a
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

### 4.1 ジェネリクス引数の構文

ジェネリクス引数は関数型の一部であり、通常の引数と統一して `()` 構文を使用する：

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

コンテナ型はジェネリック型コンストラクタであり、組み込みプリミティブではない——ユーザー定義のジェネリクスと同じ扱いを受け、统一されたジェネリクスインスタンス化パスで処理される。長さ情報の帰属が3つのコンテナ概念の根本的な違いである：

| 型            | 長さ         | セマンティクス                       | 基盤                                        |
| ------------- | ------------ | ------------------------------------ | ------------------------------------------- |
| `Array(T, N)` | 型           | 固定長配列（constジェネリクスN）     | コアプリミティブ（スタック/インライン優先） |
| `Vec(T)`      | ランタイム値 | ランタイム長さの生バッファ、拡張可能 | コアプリミティブ（ヒープ上の連続バッファ）  |
| `List(T)`     | ランタイム値 | 標準ライブラリ型（拡張可能リスト）   | ライブラリ：`{ data: Vec(T), length: Int }` |
| `Dict(K, V)`  | ランタイム値 | キー値マッピング                     | `HeapValue::Dict`                           |

> `List(T)` は**標準ライブラリ型であり、コンパイラプリミティブではない**：YaoXiang自身によって
> `std.list`
> で定義され、ユーザー定義ジェネリックレコードと同じ扱いを受ける。拡張可能セマンティクスの全戦略（いつ拡張するか、どれだけ拡張するか、共有可能か）はすべてライブラリにあり、コンパイラは関与しない。`Vec(T)`
> はそれが依存する最小基盤プリミティブである。
>
> Set(T)は抹消済み：リテラルなし、ランタイム表現なし、std.setなし。需要が発生した時にDictパターンに従って補完する。

重要なルール：

- **リテラルの配置は文脈が決定**：`[...]` 素のリテラルと `List(T)`
  注釈は拡張可能リストに配置；`Array(T, N)`
  注釈がリテラルに直接作用する場合は固定長配列に配置。配置検証：要素数 ==
  N、要素型がTと互換性、不一致はコンパイル時E1002；Nが記号定数（const引数）の場合、個数検証は精化型フェーズまで延期される。
- **暗黙のList→Array変換を禁止**：固定長性は型層で保証される——pushは `List(A)`
  レシーバのみ受け入れる。
- **パフォーマンス階層**：底から上に向かってパフォーマンスが減少し柔軟性が増加：`Array` > `Vec` >
  `List`。
- **インデックス失敗契約**（ランタイムエラーは過渡的、目標状態はコンパイル時精化カバレッジ、値依存型経由）：
  - インデックス範囲外（負のインデックスを含む）→ `E6003`
  - Dictキー欠落 → `E6008`
- **membership `in` 述語**：`Bool`
  を返しエラーにならない、右オペランドはList/Array/Dict(キー)/Tuple/String/Rangeをカバー。第一級ホーア述語であり、精化型のコンパイル時証明可能命題の基底である。`

ジェネリック関数では、型引数も同様にシグネチャで宣言され、コンパイラが実引数から自動的に推論する：

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
    push: (self: List(T), item: T) -> Void,   // selfは単なる慣習名であってキーワードではない
    get: (self: List(T), index: Int) -> Option(T)
}
```

### 4.3 ジェネリック構築呼び出しと型推論

ジェネリック型定義のフィールドリストは**自動的にコンストラクタを生成する**：各フィールドが構築引数に対応し、フィールド名が引数名となる；デフォルト値を持つフィールドは構築時に省略可能で、デフォルト値がないフィールドは必須。関数型フィールド（メソッド）は構築引数を生成しない。

```yaoxiang
// 型定義
Container: (T: Type) -> Type = {
    value: T,        // デフォルト値なし -> 構築引数必須
    extra: T,
}
// 自動的に展開された完全形式（コンパイラ内部ビュー、ユーザーが手書きする必要はない）：
// Container: (T: Type) -> (value: T, extra: T) -> Type = {
//     value: T = value,
//     extra: T = extra,
// }

// 呼び出し：自動生成されたコンストラクタを呼び出す
c  = Container(42, 43)            // 構築引数をフィールド順に入力；Tは要素から自動展開 = Int
c2 = Container("a", "b")          // T = String
c3 = Container(Int)(42, 43)       // 明示的な型引数 + 位置式構築引数
c4 = Container(Int)(extra=43, value=42)  // フィールド名式、順不同
c5 = Container(Int)()             // 空構築：フィールドはデフォルト値/ゼロ値を取る（データは事後代入）

// フィールドデフォルト値 -> 構築引数は省略可能
Point: (T: Type) -> Type = { x: T = 0, y: T = 0 }
p  = Point(1.5, 2.5)              // T = Float、x←1.5, y←2.5
p2 = Point(Int)()                 // x=0, y=0
```

**呼び出しルール**（単一括弧、宣言された引数を位置ごとにマッチング、左から右）：

1. 実引数を位置ごとに型宣言引数とのマッチングを試みる：`Type`
   位置は型実引数を受け入れ、コンパイル時値引数位置（例：`Int`）はコンパイル時定数を受け入れる。
2. コンパイル時値引数位置マッチングが成功した場合（部分マッチング）、型構築として処理する：全引数位置を順にチェックし、エラー時は宣言順に従って**最初に一致しない/欠落している引数を先に報告**する。
3. 実引数が宣言引数に完全に対応しない場合（全部値で、コンパイル時値引数位置がマッチングしない）、構築引数として処理する：位置式はフィールド順に入力し、型引数は要素型から自動展開する。

```yaoxiang
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),
    data: Array(Array(T, Cols), Rows),
}

m: Matrix(Int, 3, 4)              // 型位置：一層型構築
m2 = Matrix(Int, 3, 4)(data=[[1,2,3,4],[5,6,7,8],[9,10,11,12]])  // 二層：型 + 構築引数
m3 = Matrix(Int, 3, 4)()          // 空構築（RFC-011 §9.3パターン、データは事後代入）

Matrix(42)    // ❌ 位置0: T←42 は一致しない（42は型ではない）；位置1: Rows←42 が一致；
              //    位置2: Cols が欠落 -> 最初のエラーを先に報告：T は Type を期待するが、42 が見つかった
Container(42) // ❌ 構築引数 extra が欠落
Container(42, 43, 44)  // ❌ 構築引数過剰
```

**型推論**：ジェネリック型コンストラクタの型引数は構築引数の要素から自動展開される（`Container(42, 43)`
→ T=Int）；ジェネリック関数の型引数は実引数の型から自動展開される（`map(numbers, f)` → T=Int,
R=String、§4.1参照）。展開できない場合は明示的に指定する必要がある。

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
// 複数制約構文
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

### 7.1 コンパイル時値引数

```
LiteralType   ::= Identifier ':' Int          // コンパイル時定数（候補）
```

> **訂正**：原文ではコンパイル時値引数を「デフォルトでコンパイル時に決定」と記載していたが、この記述は**厳密に誤り**である——
> `add: (a: Int, b: Int) -> Int = a + b` の `a`/`b`
> はランタイム値引数である。**型位置で参照される**特定の型引数のみがコンパイル時値引数である。正しい定義は以下を参照。

**用語**：`Type` 以外の具体的な型（例：
`Int`）で注釈されたジェネリクス引数を**コンパイル時値引数候補**と呼び、それがコンパイル時値引数になるかどうかは、その値が型位置で参照されるか（値依存）による。**`const`
キーワードは不要**
（実装内部ではかつて「constジェネリクス」と呼んでいたが、ドキュメントでは統一して「コンパイル時値引数」を使用する）。

**判定ルール（2ステップ）**：

1. **形態粗選別**：`Type` 以外の具体的な型（`Int`/`Bool`/`Float`）で注釈された引数 → 候補。
2. **用途精選別**：候補名が**型位置**（型本体のフィールド型、内側 `Fn` 引数型、`Assert`
   述語、`Array(T, N)`
   型構築実引数位置）に現れる → 真のコンパイル時値引数；そうでなければ**ランタイム値引数**。

| 書き方                                                     | 判定                    | 理由                        |
| ---------------------------------------------------------- | ----------------------- | --------------------------- |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b ランタイム値引数    | 値位置にのみ出現            |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N コンパイル時値引数    | Nが型構築実引数位置に出現   |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N コンパイル時値引数    | Nが内側引数kの型になる      |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N 落空→ランタイム値引数 | Nが型本体で参照されていない |

**コア設計**：`(N: Int)` コンパイル時値引数 + `(k: N)`
値引数を用いて、コンパイル時定数とランタイム値を区別する。落空候補（形態は候補だが用途が命中しない）はランタイム値引数に退化する——関数レベルと型コンストラクタパスの両方がこれに従って処理される。

```yaoxiang
// コンパイル時値引数：Nが型位置（Array長さスロット）で参照される
Measure: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // Nが型構築実引数位置に出現 -> コンパイル時値引数
    length: N
}

// 使用方法：factorial(5)は型位置で評価され（コンパイル時）、結果120が型に埋め込まれる
arr: Measure(Int, factorial(5))  // コンパイラはコンパイル時にfactorial(5) = 120を計算

// 値依存：Nが内側引数kの型になる
// Nはコンパイル時値引数（(k: N)の型位置に現れる）；
// kはランタイム値引数、その型はリテラル型N（単一値型）。
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

### 8.1 If条件型

```
IfType        ::= 'If' '(' BoolExpr ',' TypeExpr ',' TypeExpr ')'
```

```yaoxiang
// 型レベルIf
If: (C: Bool, T: Type, E: Type) -> Type = match C {
    True => T,
    False => E
}

// 例：コンパイル時分岐
NonEmpty: (T: Type) -> Type = If(T != Void, T, Never)
// IsTrueブリッジとAssert精化型（詳細は§8.3）
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

### 8.3 Assert精化型とassertアサーション

`assert` と `Assert`
は同じ精化プリミティブの2つの面である——ディスパッチパイプラインが「述語の自由変数がコンパイル時に到達可能か」によって自動的に選択する。

**コアシグネチャ**：`assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**ディスパッチルール**：

| 判定基準                                                                   | モード      | 動作                                                                                       |
| -------------------------------------------------------------------------- | ----------- | ------------------------------------------------------------------------------------------ |
| すべての自由変数がコンパイル時に既知（ジェネリクス引数、コンパイル時定数） | CompileTime | 証明パイプラインに入る：true → Voidとして消去、false → コンパイルエラー（Neverは居住不能） |
| ランタイム自由変数が存在する（関数引数、外部入力）                         | Runtime     | ランタイムBool検査を挿入し、フロー敏感仮説集合Γに精化事実を注入                            |

**フロー敏感仮説集合Γ**：

コンパイラは各制御フロー点の既知の命題集合を維持する：

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP伝播
mut x = x - 5       // Γ = {}  ← mut kill set：古い仮説が無効化される
```

`mut` 変数代入後、その変数に関連するすべての仮説が削除される（kill
set）。分岐合流時、Γは各分岐の交差を取る。

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

**構文**：型交差 `A & B` はAとBを同時に満たす型を表す

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
// プラットフォーム型列挙（標準ライブラリ定義）
Platform: Type = { X86_64: () -> Platform, AArch64: () -> Platform, RISC_V: () -> Platform, ARM: () -> Platform, X86: () -> Platform }

// Pは事前定義されたジェネリクス引数名で、現在のコンパイルプラットフォームを表す
sum: (P: X86_64)(arr: Array(Float)) -> Float = {
    return avx2_sum(arr.data, arr.length)
}

sum: (P: AArch64)(arr: Array(Float)) -> Float = {
    return neon_sum(arr.data, arr.length)
}
```

---

## 第十一章：型属性

YaoXiangには区別すべき型属性は1種類しかない：線形 vs コピー可能。コンパイラが自動推論する。

### 11.1 Move（デフォルトの所有権移転）

すべての型はデフォルトでMoveセマンティクスに従う。代入、引数渡し、戻り値 = 所有権移転。

```yaoxiang
p: Point = Point(1.0, 2.0)
q = p           // Move、pは再読込不可
```

### 11.2 Dup（浅いコピー：ハンドルをコピー、データを共有）

**Dup属性は参照/トークン型に使用される**。Dup型の代入 = 浅いコピー——ハンドル/トークンをコピーし、基盤データを共有する。複数の保有者が同じデータブロックを指す。

| 型                 | 属性   | 説明                                                                      |
| ------------------ | ------ | ------------------------------------------------------------------------- |
| `&T`               | Dup    | ゼロサイズ読み取りトークン、トークンコピー = 複数の視点が同じデータを指す |
| `ref T`            | Dup    | Rc/Arcコピー = 参照カウント+1、ヒープデータを共有                         |
| `&mut T`           | Linear | ゼロサイズ書き込みトークン、排他的、コピー不可                            |
| その他のすべての型 | Move   | デフォルトの所有権移転                                                    |

**プリミティブ値型**（Int, Float, Bool,
Char）はコンパイラ組み込みの特殊処理：代入時に自動的に値コピーされ、2つの値は完全に独立。これはコンパイラのネイティブ動作であり、Dup型属性には属さない。

```yaoxiang
// &T: Dup、自由に別名化可能
view: &Point = &p
view2 = view     // Dup：トークンコピー、両方とも有効
print(view.x)    // 使用可能
print(view2.x)   // 使用可能

// &mut T: Linear、コピー不可
mut_ref: &mut Point = &mut p
// r2 = mut_ref  // ❌ &mut TはDupではない、コピー不可
```

### 11.3 Clone（明示的な深いコピー）とDupの関係

**Clone** は明示的な深いコピーインターフェース。すべての型がCloneを実装でき、`.clone()`
メソッドを提供する。

```yaoxiang
// Cloneインターフェース定義（標準ライブラリ）
Clone: Type = {
    clone: () -> Clone
}

// 使用
p: Point = Point(1.0, 2.0)
backup = p.clone()    // 深いコピー、pはまだ使用可能
p2 = p.clone()        // 複数回クローン可能
```

**DupとCloneの違い**：

|                    | Dup                                                     | Clone                                   |
| ------------------ | ------------------------------------------------------- | --------------------------------------- |
| **セマンティクス** | 浅いコピー：ハンドル/トークンをコピー、基盤データを共有 | 深いコピー：完全独立の複製を作成        |
| **呼び出し方式**   | 暗黙的（代入/引数渡し自動）                             | 明示的（`.clone()`）                    |
| **変更の影響**     | 相互に影響（基盤データを共有）                          | 互いに影響しない（独立複製）            |
| **適用型**         | `&T` トークン、`ref T`                                  | Cloneインターフェースを実装する任意の型 |
| **コスト**         | ゼロオーバーヘッド（トークンはゼロサイズ型）            | 型による                                |

**DupはCloneを含意せず、CloneはDupを含意しない**——これらは2つの直交する概念である：

```yaoxiang
// Dup型：トークンコピー、基盤データ共有
view: &Point = &p
view2 = view        // Dup：トークンコピー、両方とも同じpを指す
print(view.x)       // 使用可能
print(view2.x)      // 使用可能、同じデータが見える

// プリミティブ値型：コンパイラが自動値コピー（Dupではない）
x: Int = 42
y = x               // 値コピー、xとyは完全に独立
print(x)            // 使用可能

// Clone：明示的な深いコピー、独立複製を作成
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone：深いコピー、pはまだ使用可能
r = p               // Move：所有権移転、PointはDupでもプリミティブ値型でもない
```

**設計意図**：

- Dupはトークン/参照型用、「同じデータの複数の視点」問題を解決する
- Cloneは独立コピーが必要なシナリオ用、明示的呼び出しでコストを見える化する
- プリミティブ値型（Int/Float/Bool/Char）のコピーはコンパイラの組み込み動作であり、Dupには属さない
- ほとんどのユーザー定義型はデフォルトでMove、ゼロコピーで高性能

## 第十二章：借用トークン型

### 12.1 コア概念

`&T` と `&mut T`
は**ゼロサイズのコンパイル時トークン型**である。これらは「参照」ではなく、「アクセス権限の型レベル証明」である。

```
&T      →  ゼロサイズ、ソースデータを凍結（WriteTokenの取得期間中）、
          凍結保証下で複数読み取りが安全 -> Dup（コピー可能）
&mut T  →  ゼロサイズ、排他的読み書き（他のトークンを禁止）、
          排他アクセス下ではコピーは無意味 -> Linear（Dupでない）
```

**主要な特性**：

- トークンは**通常の型**であり、他のすべての型と同じスコープルールに従う
- ライフタイム注釈 `'a` は不要
- 専用の借用チェッカーは不要——型属性（Dup/Linear）が権限を自然に推論する
- コンパイル後に完全に消滅、ゼロランタイムオーバーヘッド

### 12.2 基本使用

```yaoxiang
// メソッド端：引数型を宣言し、必要な権限を決定
Point.print: (self: &Point) -> Void = {
    print(self.x)               // &Pointトークンが読み取り権限を付与
    print(self.y)
}

Point.shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
    self.x = self.x + dx        // &mut Pointトークンが書き込み権限を付与
    self.y = self.y + dy
}

// 呼び出し端：コンパイラが自動的に借用またはMoveを選択
p = Point(1.0, 2.0)
p.print()                       // コンパイラが自動的に&Pointトークンを作成
p.shift(1.0, 1.0)               // コンパイラが自動的に&mut Pointトークンを作成
p.print()                       // OK、前のトークンはshift呼び出し終了とともに解放済み

// 複数の&Tトークンの共存——Dup型は自由にコピー可能
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
(px_ref, p) = p.get_x()        // トークンが呼び出し元に返される
print(px_ref)                    // OK、トークンはまだスコープ内
```

**構造体への格納**——構造体はトークンフィールドを保持できる：

```yaoxiang
// ✅ 構造体がトークンをフィールドとして保持
Window: Type = {
    target: Point,
    view: &Point,              // トークンフィールド——targetへの読み取り専用ビューを保持
}
```

**クロージャはキャプチャしない、文脈は作成時点で固定**——クロージャは自分の引数のみを食べ、外側データが必要な場合はカリー化によって作成時点で値をクロージャに固定する：

```yaoxiang
// ✅ 文脈はカリー化によって固定される：thresholdは引数、gt_point(threshold)が作成時点で値をクロージャに固定
gt_point: (t: Float) -> (p: Point) -> Bool = (p) => p.x > t
filter_by_threshold: (items: List(Point), threshold: Float) -> List(Point) = {
    items.filter(gt_point(threshold))
}
```

> 注：クロージャ（関数値）がエスケープした後、その定義箇所のスコープは死んでいる可能性があるため、外側変数を暗黙的にキャプチャしてはならない；しかし呼び出し箇所（作成箇所）のスコープは必ず生存しており、その時点で文脈が値として固定されクロージャに入ること安全性がある。

### 12.4 自動借用選択

呼び出し端でコンパイラは以下の優先順位に従って自動的に選択する：

```
1. 実引数が後にまだ使用される -> トークン作成を優先（&T または &mut T、メソッドシグネチャによる）
2. 実引数が後に使用されない -> Move
3. 優先マッチング順序：&T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // printの引数型は&Point -> コンパイラが&Pointトークンを作成
p.shift(1.0, 1.0)  // shiftの引数型は&mut Point -> コンパイラが&mut Pointトークンを作成
p2 = p             // 後は使用されない -> Move
```

**メソッドレシーバはシグネチャセマンティクスに従う**（訂正 2026-08-30、RFC-011a レシーバスペル约定と同じ）：レシーバが
`&T` → 読み取り借用トークン；`&mut T` → 可変借用トークン；値渡し →
Move（レシーバを消費）。呼び出し箇所で生成された借用トークンは呼び出し終了とともに解放される（transient、§12.5 区間セマンティクス）；インターフェースの借用レシーバはインターフェース作者が
`&Self` を明示的に宣言し、impl シグネチャは `Self ↦ impl型`
置換後インターフェースと完全一致する必要がある（RFC-011a §3）。

### 12.5 トークン衝突検出

トークン衝突検出は**借用ホーア命題**（RFC-009a）であり、独立したフロー敏感分析ではない。コンパイラが借用命題を自動生成（`borrow_conflict`/`use_after_move`/`use_after_drop`/`mut_violation`）し証明パイプラインに送って検証する；トークン活性は区間
`[created_at, last_use]`（RFC-009a §逆BFS活性分析参照）：

```yaoxiang
// ❌ &mutと派生&Tは同時にアクティブになれない
bad_alias: (p: &mut Point) -> Void = {
    p.x = 10.0                   // ✅ WriteTokenの正常使用
    print(p.y)
}

// ✅ トークンスコープ終了後に自動解放
good_seq: (p: &mut Point) -> Void = {
    {
        // 内部スコープ
        print(p.x)               // &mut Pointを使用
    }
    // 内部スコープ終了
    p.x = 10.0                   // ✅ WriteTokenはまだ使用可能
}

// ❌ 同じ実引数から同時に&mutトークンと他のトークンを作成できない
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ pから同時に&mutと&トークンを派生
```

### 12.6 コンパイラ内部：ブランド機構

ユーザーがブランドに触れることはない。コンパイラは内部で各トークンにコンパイル時一意識別子を割り当てる：

```
ユーザーが見るもの    コンパイラ内部表現
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #Nはコンパイル時一意整数
&mut Point     →  WriteToken(Point, #M)   // #Mはコンパイル時一意整数
```

ブランドの用途：

- **偽造防止**：トークンは所有者カプセルからのみ取得でき、凭空に構築できない
- **関連追跡**：フィールドアクセスから派生した `&Float`
  は派生ブランド（`#N.field_x`）を持ち、コンパイラは親トークンまで追跡できる
- **衝突検出**：同源WriteTokenと派生ReadTokenは同時にアクティブになれない

ブランドは単態化とインライン化後に完全に消滅し、生成された機械語には存在しない。**ゼロランタイムオーバーヘッド。**

### 12.7 トークンSum型

```
&BorrowToken ::= &T          // ReadToken（ソースデータ凍結 -> Dup安全）
               | &mut T      // WriteToken（排他的読み書き -> Linear）
```

### 12.8 借用トークン vs ref

|            | `&T` / `&mut T`                                      | `ref`                            |
| ---------- | ---------------------------------------------------- | -------------------------------- |
| 機能       | 一目見る/インプレース変更                            | 共有保持                         |
| 範囲       | トークン値のスコープに従う                           | スコープをまたぐ                 |
| コスト     | ゼロオーバーヘッド（ゼロサイズ型、コンパイル後消滅） | RcまたはArc（コンパイラが選択）  |
| エスケープ | 可（トークンは戻り値/構造体で伝播）                  | 本来エスケープ用                 |
| タスク横断 | 不可（トークンはタスク横断未実装）                   | 可（コンパイラが自動でArc選択）  |
| 環検出     | 関与しない                                           | タスク内は静粛、タスク横断はlint |

> 注（未定義）：ref作成後に内容をどう読むか（デリファレンス/メソッド/自動）はまだ仕様で定義されておらず、実装現状の
> `*a` はE1052を報告する。定義後に本節に補完する。

---

## 付録：型定義クイックリファレンス

### A.1 型定義

```
// === レコード型（波括弧） ===

// レコード型
Point: Type = { x: Float, y: Float }

// バリアント付きレコード型（関数フィールドを使用）
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// === インターフェース型（波括弧、フィールドがすべて関数） ===

// インターフェース定義
Serializable: Type = { serialize: () -> String }

// インターフェースを実装する型
Point: Type = {
    x: Float,
    y: Float,
    Serializable    // Serializableインターフェースを実装
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

// コンパイル時ジェネリクス：Nが型位置 (k: N) で参照される -> コンパイル時値引数
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
// すべての型はデフォルトでMove。代入、引数渡し、戻り値 = 所有権移転

// === プリミティブ値型（コンパイラ組み込み） ===
Int, Float,     // 代入時に自動値コピー、2つの値は完全に独立
Bool, Char      // Dupではなく、コンパイラのプリミティブ組み込み処理

// === Dup（浅いコピー：ハンドルをコピー、基盤データを共有） ===
&T              // ゼロサイズ読み取りトークン、トークンコピー = 複数の視点が同じデータを指す
ref T           // Rc/Arcコピー = 参照カウント+1、ヒープデータを共有

// === Linear ===
&mut T          // ゼロサイズ書き込みトークン、Linear（排他的、コピー不可）

// === Clone（明示的な深いコピー） ===
value.clone()   // 独立コピー作成、変更は元値に影響しない
```

### A.4 借用トークンクイックリファレンス

```
// === 借用トークン ===
&T              // ゼロサイズコンパイル時読み取りトークン、ソースデータ凍結 -> Dup（コピー可能）
&mut T          // ゼロサイズコンパイル時書き込みトークン、排他的読み書き -> Linear（コピー不可）

// 呼び出し端自動選択
// 1. 実引数が後にまだ使用される -> トークン作成
// 2. 実引数が後に使用されない -> Move
// 3. 優先マッチング：&T < &mut T < Move

// トークン伝播
// ✅ 返却可、構造体格納可、クロージャキャプチャ可
// ❌ タスク横断不可（トークンはタスク横断未実装）
```

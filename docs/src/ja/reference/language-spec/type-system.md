# 型システム仕様

本ドキュメントは YaoXiang プログラミング言語の型システム仕様を定義ものであり、基本型、複合型、ジェネリクス、トレイトを含む。

---

## 第零章：理論的基盤

### 0.1 カリー＝ハワード同型対応

カリー＝ハワード同型対応（Curry-Howard correspondence）は YaoXiang の型システムの理論的基盤である。これはプログラミング言語の型システムと数理論理学の奥深い対応関係を示すものである：

| 論理学                          | プログラミング言語                           |
| ------------------------------- | -------------------------------------------- |
| 命題 \(P\)                      | 型 `Type`                                    |
| 証明 \(p: P\)                   | プログラム `x: T = ...`                      |
| 含意 \(P \rightarrow Q\)       | 関数型 `(P) -> Q`                            |
| 連言 \(P \wedge Q\)             | 直積型 `{ a: P, b: Q }`                      |
| 選言 \(P \vee Q\)               | 直和型 `{ a(P) \| b(Q) }`                    |
| 全称量化 \(\forall x:T. P(x)\)  | ジェネリクス `(T: Type) -> ...`              |
| 真 \(\top\)                     | `Void`（ユニット型、デフォルト値あり）       |
| 偽 \(\bot\)                     | `Never`（零コンストラクタ、居留する値なし）  |
| 型宇宙 \(Type_n : Type_{n+1}\)  | 宇宙分层（ラッセルの逆理防止）                |
| case 分析                       | 型レベル `match`                             |

> **注意**：型レベル `match` は場合分け（case analysis）であり、数学的帰納法ではない。帰納法には型レベル再帰関数 + コンパイラの終了性検査が必要である。

### 0.2 型は命題、プログラムは証明

YaoXiang では、この対応関係は設計の一等原則である：

- **終了する型レベル計算は正しい構成的証明に対応する**。YaoXiang の型族（`Nat` 上の `Add` の case 分析 + 再帰呼び出しなど）は数学的帰納法の型レベル符号化である——前提としてコンパイラが終了性検査できること。
- **型検査は証明の検証である**。プログラムが型検査を通るとは、論理的命題が構成的に証明されたことに相当する。

### 0.3 言語設計への影響

カリー＝ハワード同型対応の YaoXiang における具体的体現：

1. **宇宙分层**（RFC-010）：`Type₀ : Type₁ : Type₂ …` は `Type: Type` が招く論理的逆理（Girard 逆理）を回避する
2. **型族**（RFC-011）：自然数 `Nat(Zero/Succ)` の型レベル case 分析 + 再帰呼び出しは Peano 公理系に対応する——前提としてコンパイラが終了性検査を行うこと
3. **条件型**（RFC-011）：`If: (C: Bool, T: Type, E: Type) -> Type` は論理学における場合分け選言に対応する
4. **値依存型**（RFC-011）：`Array: (T: Type, N: Int) -> Type` は「各整数 N に対して型が存在する」の有限量化に対応する

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

> **設計上の注意**：RFC-010 で「すべては代入である」という統一モデル（`name: type = value`）が提唱されているが、構文レベルでは型と値は区別する必要がある。コンパイラの實現では `Type` と `Expr` は独立した2つの AST 列挙型（`ast.rs:406` と `ast.rs:25`）であり、`TypeExpr` は BNF のプレースホルダとして実装の `Type` 列挙型に対応し、「この位置には型が来る」ことを示す。

---

## 第二章：基本型

### 2.1 原型

| 型        | 論理対応          | 説明                                                                   | デフォルトサイズ |
| --------- | ----------------- | ---------------------------------------------------------------------- | ---------------- |
| `Type`    | —                 | 元型                                                                   | 0 バイト         |
| `Never`   | ⊥（偽/空型）      | 零コンストラクタ、値が存在しない。発散/panic の返り型。`Never <: T`    | 0 バイト         |
| `Void`    | ⊤（真/ユニット）  | デフォルト void 値 あり、零フィールド積型。`x: Void = <デフォルト>` 可  | 0 バイト         |
| `Bool`    | —                 | 真理値：`true` / `false`                                               | 1 バイト         |
| `Int`     | —                 | 符号付き整数                                                           | 8 バイト         |
| `Uint`    | —                 | 符号なし整数                                                           | 8 バイト         |
| `Float`   | —                 | 浮動小数点数                                                            | 8 バイト         |
| `String`  | —                 | UTF-8 文字列                                                           | 可変             |
| `Char`    | —                 | Unicode 文字                                                           | 4 バイト         |
| `Bytes`   | —                 | 生バイト                                                               | 可変             |

ビット幅付き整数：`Int8`, `Int16`, `Int32`, `Int64`, `Int128`
ビット幅付き浮動小数点：`Float32`, `Float64`

### 2.2 Never と Void：⊥ と ⊤

`Never` と `Void` は型システムの論理素子である——それぞれ偽（⊥）と真（⊤）に対応する。

**Never（⊥、偽/空型）** — 三つの不容商量な性質：

1. **零コンストラクタ**：`Never` 型の値を生成できるリテラルや式は一切存在しない。`x: Never = ...` の右辺は書けない。
2. **爆発原理**：`Never <: T` は任意の型 `T` に対して成立する。`assert(false)` は `Never` を返し、以後のコードは型検査を通る（ただし決して実行されない）。
3. **発散マーク**：`f: (...) -> Never` は `f` が戻らないことを保証する。コンパイラはこれを使って dead code 分析と `match` 枝合流を行う。

`Never` は組み込み型名（`Int`/`Bool` と同じ登録パス）であり、キーワードではない。

**Void（⊤、真/ユニット）** — ちょうど一つの居留者（デフォルト void 値）。`Void` は零フィールド積型の単位元である。`x: Void = <デフォルト>` は合法である。ブロックの値は**末尾式**によって与えられる（空ブロック `{}` は `Void`）。詳細は [RFC-010a](../../design/rfc/accepted/010a-tail-expression-and-return.md) を参照。

---

## 第三章：複合型

### 3.1 記録型

**統一的構文**：`Name: Type = { field1: Type1, field2: Type2, ... }`

```
RecordType  ::= '{' FieldList? '}'
FieldList   ::= Field (',' Field)* ','?
Field       ::= Identifier ':' TypeExpr
            |  Identifier                 // インターフェース制約
```

```yaoxiang
// 単純な記録型
Point: Type = { x: Float, y: Float }

// 空記録型
Empty: Type = {}

// ジェネリクスを持つ記録型
Pair: (T: Type) -> Type = { first: T, second: T }

// インターフェースを実装する記録型
Point: Type = {
    x: Float,
    y: Float,
    Drawable,
    Serializable
}
```

**規則**：

- 記録型は波括弧 `{}` で定義する
- フィールド名の直後にコロンと型を続ける
- インターフェース名は型本体内に記述してそのインターフェースを実装することを示す

> **名前空間への帰属**：`Type.name` プレフィックス（例：`Point.draw`）は、その関数が `Point` の名前空間属することを示す。これは何の暗黙的束縛もトリガーしない。`p.draw()` のような `.` 呼び出し構文を動作させるには、明示的に束縛する必要がある：`Point.draw = draw[0]`。詳細は RFC-004 と RFC-010 を参照。

#### 3.1.1 フィールドデフォルト値

型のフィールドにはデフォルト値を指定でき、構築時に省略可能である：

```yaoxiang
// デフォルト値のあるフィールド - 構築時に省略可能
Point: Type = {
    x: Float = 0,
    y: Float = 0
}

// 使用例
Point()           // -> Point(x=0, y=0)
Point(x=1)       // -> Point(x=1, y=0)
Point(x=1, y=2) // -> Point(x=1, y=2)

// デフォルト値のないフィールド - 構築時に必須
Point2: Type = {
    x: Float,
    y: Float
}

// 使用例
Point2(x=1, y=2) // 正しい
Point2()          // 錯誤
```

**規則**：

- `field: Type = expression` -> デフォルト値あり、構築時に省略可能
- `field: Type` -> デフォルト値なし、構築時に必須

#### 3.1.2 組み込み束縛

型定義体内で直接メソッドを束縛できる：

```yaoxiang
// 方法1：外部関数を参照して束縛
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]    // 位置0に束縛
}
// 呼び出し：p1.distance(p2) -> distance(p1, p2)

// 方法2：匿名関数 + 位置束縛
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

**インターフェース実装**：型は定義末尾にインターフェース名を列挙してインターフェースを実装する

```yaoxiang
// インターフェースを実装する型
Point: Type = {
    x: Float,
    y: Float,
    Drawable,        // Drawable インターフェースを実装
    Serializable     // Serializable インターフェースを実装
}
```

**インターフェースへの直接代入**：具体型はインターフェース型変数に直接代入可能（構造的下位型）

```yaoxiang
// 直接代入（コンパイル時に具体型確定 -> ゼロオーバーヘッド呼び出し）
d: Drawable = Circle(1)
d.draw(screen)        // コンパイル後：circle_draw を直接呼び出し、vtable なし

// 関数の返り値（コンパイル時に判断不能 -> vtable 呼び出し）
d: Drawable = get_shape()
d.draw(screen)        // vtable 経由でメソッド查找

// インターフェースを関数引数として受け取る
process: (d: Drawable) -> Void = d.draw(screen)
```

**コンパイル時最適化戦略**：

| シナリオ           | 推論結果         | 呼び出し方式           |
| ------------------ | ---------------- | ---------------------- |
| 具体型への直接代入 | 具体型確定可能   | 直接呼び出し（ゼロコスト） |
| 関数返り値         | 不明             | vtable                 |
| 不均一コレクション  | 複数型           | vtable                 |

**整合性と孤児規則（適用外、終結説明）**：YaoXiang のインターフェースは構造的型（インターフェース = 全フィールドが関数型の記録）であり、名目的トレイトではない——「誰が誰のために実装できるか」の帰属問題が存在しないため、Rust 式の孤児規則と整合性チェックの適用対象がない（裁定記録は RFC-011 §2.1 を参照）。構造的世界の対応保障は**重複実装拒否**である：同一メソッドシグネチャが型上で重複定義されるとコンパイルエラー（RFC-011a §3、override 禁止；オーバーロードは合法）。

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

### 4.1 ジェネリクス仮引数構文

ジェネリクス仮引数は関数型の一部であり、普通のパラメータと統一的に `()` 構文を使用する：

```
GenericType     ::= Identifier '(' TypeArgList ')'
TypeArgList     ::= TypeExpr (',' TypeExpr)* ','?
TypeBound       ::= Identifier
                 |  Identifier '+' Identifier ('+' Identifier)*
```

ジェネリクス型定義において、`(T: Type)` は型コンストラクタの仮引数シグネチャであり、`-> Type` は返り型を示す：

```yaoxiang
List: (T: Type) -> Type = { ... }
Map: (K: Type, V: Type) -> Type = { ... }
```

### 4.1.1 コンテナ型

コンテナ型はジェネリクスク型コンストラクタであり、組み込みプリミティブではない——ユーザー定義ジェネリクスと同じ待遇であり、統一されたジェネリクスインスタンス化経路を通じて処理される。長さと情報の帰属は三つのコンテナ概念の根本的違いである：

| 型              | 長さ       | 意味                           | ベース                           |
| --------------- | ---------- | ------------------------------ | -------------------------------- |
| `Array(T, N)`   | 型         | 固定長配列（const ジェネリクス N）| コアプリミティブ（スタック/インライン優先）|
| `Vec(T)`        | 実行時値   | 実行時可変長の生バッファ、成長可能| コアプリミティブ（ヒープ連続バッファ）|
| `List(T)`       | 実行時値   | 標準ライブラリ型（成長可能リスト）| ライブラリ：`{ data: Vec(T), length: Int }`|
| `Dict(K, V)`    | 実行時値   | キー値マッピング               | `HeapValue::Dict`                |

> `List(T)` は**標準ライブラリ型であり、コンパイラプリミティブではない**：YaoXiang 自身により `std.list` で定義されており、ユーザー定義ジェネリクス記録と同じ待遇である。成長可能セマンティクスの全戦略（いつ拡張するか、どの程度拡張するか、共有可能か否か）はすべてライブラリ内にあり、コンパイラは関与しない。`Vec(T)` が依存する最小基盤プリミティブである。
>
> Set(T) は除外済み：リテラルなし、実行時表現なし、std.set なし。需求が生じたら Dict パターンに従って追加する。

重要な規則：

- **リテラルの落点は文脈で決定**：`[...]` ベアリテラルは `List(T)` 注釈があると成長可能リストになる；`Array(T, N)` 注釈が直接リテラルに作用すると固定長配列になる。落点検証：要素数 == N、要素型が T と互換、不一致はコンパイル時 E1002；N がシンボル定数（const パラメータ）の場合は要素数検証を精化型段階に延期。
- **暗黙の List→Array 変換禁止**：固定長性は型レベルで保証される——push は `List(A)` レシーバのみ受け付ける。
- **パフォーマンス階層**：下から上へ性能低下、柔軟性増加：`Array` > `Vec` > `List`。
- **索引失敗契約**（実行時エラーは移行状態、目標はコンパイル時精化で覆盖、値依存型に移行、見 §8.4）：
  - 索引範囲外（含负索引）→ `E6003`
  - Dict でキー不在 → `E6008`
- **membership `in` 述語**：例外を投げず `Bool` を返す、右オペランドはList/Array/Dict(キー)/Tuple/String/Range を覆盖。一等功能ホーア述語であり、精化型コンパイル時証明可能命題の基底である。`

ジェネリクス関数においても、型パラメータはシグネチャで宣言し、コンパイラが実引数から自動的に推論する：

```yaoxiang
map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R)) = ...
```

### 4.2 ジェネリクス型定義

```yaoxiang
// 基本ジェネリクス型
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
    push: (self: List(T), item: T) -> Void,   // self は約束名であり、キーワードではない
    get: (self: List(T), index: Int) -> Option(T)
}
```

### 4.3 ジェネリクス構築呼び出しと型推論

ジェネリクス型定義のフィールドリストは**自動的にコンストラクタを生成**する：各フィールドが1つの構築パラメータに対応し、フィールド名がパラメータ名となる；デフォルト値のあるフィールドは構築時に省略可能で、デフォルト値のないフィールドは必須である。関数型フィールド（メソッド）は構築パラメータを生成しない。

```yaoxiang
// 型定義
Container: (T: Type) -> Type = {
    value: T,        // デフォルト値なし -> 構築パラメータ必須
    extra: T,
}
// 自動展開された完全形式（コンパイラの内部ビュー、ユーザーが手書きする必要はない）：
// Container: (T: Type) -> (value: T, extra: T) -> Type = {
//     value: T = value,
//     extra: T = extra,
// }

// 呼び出し：自動生成されたコンストラクタを呼び出す
c  = Container(42, 43)            // 構築パラメータはフィールド順;T は要素から自動解凍 = Int
c2 = Container("a", "b")          // T = String
c3 = Container(Int)(42, 43)       // 明示的型パラメータ + 位置式構築パラメータ
c4 = Container(Int)(extra=43, value=42)  // フィールド名式、順序任意
c5 = Container(Int)()             // 空構築：フィールドはデフォルト値/零値（データは後で代入）

// フィールドデフォルト値 -> 構築パラメータ省略可能
Point: (T: Type) -> Type = { x: T = 0, y: T = 0 }
p  = Point(1.5, 2.5)              // T = Float、x←1.5, y←2.5
p2 = Point(Int)()                 // x=0, y=0
```

**呼び出し規則**（単括弧、宣言パラメータを位置ごとに匹配、左から右へ）：

1. 実引数を位置ごとに型宣言パラメータとの匹配を試みる：`Type` 位は型実引数を受け付け、実行時値パラメータ位（例：`Int`）は実行時定数を受け付ける。
2. 実行時値パラメータ位での匹配が成功した場合（部分匹配）、型構築として処理する：全パラメータ位を位置ごとに検査し、エラー時は宣言順で**最初不一致/欠落のパラメータを報告**する。
3. 実引数が宣言パラメータに全く対応できない場合（すべて値、実引数なし）は、構築パラメータとして処理する：位置式はフィールド順、型パラメータは要素型から自動解凍。

```yaoxiang
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),
    data: Array(Array(T, Cols), Rows),
}

m: Matrix(Int, 3, 4)              // 型位置：一層目型構築
m2 = Matrix(Int, 3, 4)(data=[[1,2,3,4],[5,6,7,8],[9,10,11,12]])  // 二層：型 + 構築パラメータ
m3 = Matrix(Int, 3, 4)()          // 空構築（RFC-011 §9.3 モード、データは後で代入）

Matrix(42)    // ❌ 位0: T←42 不一致（42 は型ではない）；位1: Rows←42 匹配；
              //    位2: Cols 欠落 -> 最初エラーを報告：T は Type を期待、42 を得た
Container(42) // ❌ 構築パラメータ extra 欠落
Container(42, 43, 44)  // ❌ 構築パラメータ過多
```

**型推論**：ジェネリクスク型コンストラクタの型パラメータは構築パラメータ要素から自動解凍（`Container(42, 43)` → T=Int）；ジェネリクス関数の型パラメータは実引数型から自動解凍（`map(numbers, f)` → T=Int, R=String、§4.1 参照）。解凍できない場合は明示的に指定する必要がある。

---

## 第五章：型制約

### 5.1 単一制約

```
ConstrainedType ::= '(' Identifier ':' TypeBound ')' TypeExpr
```

```yaoxiang
// インターフェース型定義（制約として使用）
Clone: Type = {
    clone: () -> Clone
}

// 制約を使用
clone: (T: Clone)(value: T) -> T = value.clone()
```

### 5.2 複数制約

> **制約の解決源**（RFC-011b）：演算子制約名（`Add` / `Subtract` / `Multiply` / `Divide` / `Modulo` / `Equal` / `Index`）の解決 = インターフェース実装登録簿の検索——
> `T: Add` ≜ `Add(T, T, T)` インスタンスが登録済み；`Equal` は構造的推論も追加（全フィールドが比較可能な記録は自動的に比較可能）。`Zero` / `One` / `PartialOrd` などの名前は定義源がまだなく、宙吊り制約名である。

```yaoxiang
// 複数制約構文
combine: (T: Clone + Add)(a: T, b: T) -> T = {
    a.clone() + b
}

// ジェネリクスコンテナのソート
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
// Iterator トレイト（記録型構文を使用）
Iterator: (T: Type) -> Type = {
    Item: T,                    // 関連型
    next: () -> Option(T),
    has_next: () -> Bool
}

// 関連型を使用
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

### 6.2 ジェネリクス関連型（GAT）

```yaoxiang
// より複雑な関連型
Container: (T: Type) -> Type = {
    Item: T,
    IteratorType: Iterator(T),  // 関連型もジェネリクス
    iter: () -> IteratorType
}
```

---

## 第七章：コンパイル時ジェネリクス

### 7.1 コンパイル時値パラメータ

```
LiteralType   ::= Identifier ':' Int          // コンパイル時定数（候補）
```

> 判定根拠は**型位置で参照されているか**であり、「具体的な型が标注されているか」ではない：`add: (a: Int, b: Int) -> Int = a + b` において
> `a`/`b` は実行時値パラメータである（両方ともいかなる型位置にも現れない）。

**用語**：`Type` 以外の具体的な型（例：`Int`）を标注したジェネリクスパラメータは**コンパイル時値パラメータ候補**と呼ばれ、コンパイル時値パラメータになるかどうかは型位置で参照されるか（値依存）による。**`const` キーワードは不要**（実装内部ではかつて「const ジェネリクス」を使用していたため、ドキュメントでは統一的に「コンパイル時値パラメータ」を使用する）。

**判定規則（二段階）**：

1. **形態粗筛**：パラメータが `Type` 以外の具体型（`Int`/`Bool`/`Float`）を标注 → 候補。
2. **用途精筛**：候補名が**型位置**に現れる（型本体フィールド型、内側 `Fn` パラメータ型、 `Assert` 述語、`Array(T, N)` 型構築実引位置）→ 真的コンパイル時値パラメータ；そうでなければ**実行時値パラメータ**。

| 写法                                                     | 判定               | 理由                   |
| -------------------------------------------------------- | ------------------ | ---------------------- |
| `add: (a: Int, b: Int) -> Int = a + b`                   | a/b 実行時値パラメータ | 値位置のみに出現       |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N コンパイル時値パラメータ | N が型構築実引位置に出現 |
| `factorial: (N: Int) -> (k: N) -> Int`                   | N コンパイル時値パラメータ | N が内側パラメータ k の型 |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`              | N 落空→実行時値パラメータ | N が型本体で参照されていない |

**核心設計**：(N: Int) コンパイル時値パラメータ + (k: N) 値パラメータ 사용하여 컴파일 타임 상수와 실행 타임 값을 구분한다. 落空候補（形態は候補、用途は未命中）は実行時値パラメータに退化——関数レベルも型コンストラクタ経路も共にこの處理に従う。

```yaoxiang
// コンパイル時値パラメータ：N が型位置（Array 長さスロット）で参照されている
Measure: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // N が型構築実引位置に出現 -> コンパイル時値パラメータ
    length: N
}

// 使用方法：factorial(5) が型位置で評価（コンパイル時）、結果 120 が型に埋め込まれる
arr: Measure(Int, factorial(5))  // コンパイラはコンパイル時に factorial(5) = 120 を計算

// 値依存：N が内側パラメータ k の型として
// N はコンパイル時値パラメータ（(k: N) の型位に出現）；
// k は実行時値パラメータ、その型はリテラル型 N（単値型）である。
factorial: (N: Int) -> (k: N) -> Int = {
    match k {
        0 => 1,
        _ => k * factorial(k - 1)
    }
}
```

### 7.2 コンパイル時定数配列

```yaoxiang
// 行列型で使用
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
// IsTrue は Assert 精化型のブリッジ（§8.3 参照）
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

### 8.3 Assert 精化型と assert 述語

`assert` と `Assert` は同一精化素子の两面である——「述語の自由変数がコンパイル時にアクセス可能か」に基づいて dispatch 分派管が自動的に選択する。

**核心シグネチャ**：`assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**dispatch 分派規則**：

| 判据                                             | モード       | 動作                                                               |
| ------------------------------------------------ | ------------ | ------------------------------------------------------------------ |
| 全自由変数がコンパイル時既知（ジェネリクスパラメータ、コンパイル時定数） | CompileTime  | 証明管道に進む：true → Void に消去、false → コンパイルエラー（Never は居留不可） |
| 実行時自由変数が存在（関数パラメータ、外部入力） | Runtime      | 実行時 Bool 検査 を挿入し、流感性仮定集合 Γ に精化事実を注入        |

**流感性仮定集合 Γ**：

コンパイラは各制御流点の既知命題集合を維持する：

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP 伝播
mut x = x - 5       // Γ = {}  ← mut kill set：古い仮定が無効
```

`mut` 変数代入後、その変数に関わる全仮定が削除される（kill set）。枝合流時に Γ は各枝の共通部分を取る。

### 8.4 Terminates：終了測度述語

`Terminates` は**組み込み述語**であり、`Int`、`Never` と同様にコアプリミティブ（組み込み名、キーワードではない）。これは**測度**を計算の断片に結びつけ、その計算が停止すること、および停止の証拠を宣言する。

**形態**：二項、一項、同一述語：

| 形態                     | アンカー         | 用途                                                         |
| ------------------------ | ---------------- | ------------------------------------------------------------ |
| `Terminates(m)`          | 所在束縛の名前   | デフォルト形態——自己再帰関数、ループ                         |
| `Terminates(FnType, m)`  | 明示的関数型     | 測度を明示的に指定する必要がある場合（測度が他で定義、同じ測度が複数の計算に提供服务） |

```yaoxiang
// 測度：普通の関数、単体テスト可能、再利用可、実行時に関与しない
gcd_measure: (a: Int, b: Int) -> Int = { b }

// 二項形態：測度が他で定義、明示的に帰属を指定
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

// 一項形態：アンカーが束縛名、測度はスコープ内の式
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n {
        i = i + 1
    }
    return acc
}
```

**意味**：`Terminates(m)` が精化するのは、それがある型位置に标注された計算断片の型である。義務はその計算にある——関数なら各再帰呼び出し点、ループならバックエッジ——すべて「次状態の測度が現在の測度より厳密に小さい」であり、その点の路径守卫下で判定する。

呼び出し点の義務は `m(callee_args) < m(caller_args)`；バックエッジの義務は `m(次週) < m(今週)`。両者は形式的に同一である。

> **なぜループを覆盖するか**：ループは匿名構造であり、通常は参照不能である。束縛名が名前を提供する——`acc: Terminates(n - i) = while ...` における `acc` がアンカーであり、ループはこれにより参照可能になる。これが `Terminates` の一項形態がループに作用する理由である。

**測度**：返り型に制限なし（自然数を強制しない）；その上の「厳密減少」はその型上で利用可能な適切な全順序によって与えられる。測度が整序可能か（`Int` 返りでの >= 0 是否か）は**独立した義務**であり、減少義務と同様にコンパイル時証明管道が判定する。

**トリガー**：終了検査は**精化型**によってトリガーされる——型が精化되면検証モードに入る。精化されていない普通の型（ベア `while` ループ、精化署名のない関数など）は検証モードに入らず、終了義務も生成しない。

**自動探索優先**：コンパイラはまず自動探索測度（線形順位関数、述語違反カウント、有界増減、乗法スケール四つのテンプレート）を行い，探索不能な場合のみ明示的に `Terminates` を指定する必要がある。

**実行時表現**：純粋なコンパイル時実体であり、証拠跟着いて消去され、実行時バイナリに含まれない。

> 完全な設計は
> [RFC-027 §6.9](../../design/rfc/accepted/027-compile-time-evaluation-types.md)（意味論）と
> [RFC-027a](../../design/rfc/review/027a-termination-explicit-measure.md)（実装メカニズム）を参照。

---

## 第九章：型の合併と交叉

### 9.1 型合併

```
TypeUnion     ::= TypeExpr '|' TypeExpr
```

### 9.2 型交叉

```
TypeIntersection ::= TypeExpr '&' TypeExpr
```

**構文**：型交叉 `A & B` は A と B の両方を同時に満たす型を表す

```yaoxiang
// インターフェース合成 = 型交叉
DrawableSerializable: Type = Drawable & Serializable

// 交叉型を使用
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
// プラットフォーム型列挙型（標準ライブラリで定義）
Platform: Type = { X86_64: () -> Platform, AArch64: () -> Platform, RISC_V: () -> Platform, ARM: () -> Platform, X86: () -> Platform }

// P は事前定義のジェネリクスパラメータ名、現在のコンパイルプラットフォームを表す
sum: (P: X86_64)(arr: Array(Float)) -> Float = {
    return avx2_sum(arr.data, arr.length)
}

sum: (P: AArch64)(arr: Array(Float)) -> Float = {
    return neon_sum(arr.data, arr.length)
}
```

---

## 第十一章：型属性

YaoXiang には区別が必要な型属性は一つだけである：線形 vs 複製可能。コンパイラが自動的に推論する。

### 11.1 Move（デフォルト所有権移動）

すべての型はデフォルトで Move セマンティクスを遵循する。代入、引数渡し、返り = 所有権移動。

```yaoxiang
p: Point = Point(1.0, 2.0)
q = p           // Move、p はこれ以降読めない
```

### 11.2 Dup（シャローコピー：ハンドルの複製、データの共有）

**Dup 属性は参照/トークン型に使用する**。Dup 型の代入 = シャローコピー——ハンドル/トークンを複製し、基盤データを共有する。複数の保持者が同一データブロックを指す。

| 型          | 属性    | 説明                                              |
| ----------- | ------- | ------------------------------------------------- |
| `&T`        | Dup     | 零サイズ読み取りトークン、トークン複製 = 複数視点で同一データを指す |
| `ref T`     | Dup     | Rc/Arc 複製 = 参照カウント+1、共有ヒープデータ  |
| `&mut T`    | Linear  | 零サイズ書き込みトークン、排他的、使用不可複製    |
| その他全型   | Move    | デフォルト所有権移動                              |

**原型値型**（Int, Float, Bool, Char）はコンパイラの組み込み特殊處理である：代入時に自動値コピーされ、2つの値が完全に独立する。これはコンパイラの原生的な動作であり、Dup 型属性には属さない。

```yaoxiang
// &T: Dup、自由なエイリアス可
view: &Point = &p
view2 = view     // Dup：トークン複製、両者とも有効
print(view.x)    // 使用可
print(view2.x)   // 使用可

// &mut T: Linear、複製不可
mut_ref: &mut Point = &mut p
// r2 = mut_ref  // ❌ &mut T は Dup ではない、複製不可
```

### 11.3 Clone（明示的ディープコピー）と Dup の関係

**Clone** は明示的ディープコピーインターフェースである。全型が Clone を実装でき、`.clone()` メソッドを提供する。

```yaoxiang
// Clone インターフェース定義（標準ライブラリ）
Clone: Type = {
    clone: () -> Clone
}

// 使用
p: Point = Point(1.0, 2.0)
backup = p.clone()    // ディープコピー、p は 여전히使用可能
p2 = p.clone()        // 複数回クローン可
```

**Dup と Clone の違い**：

|              | Dup                                 | Clone                     |
| ------------ | ----------------------------------- | ------------------------- |
| **意味**     | シャローコピー：ハンドル/トークンを複製、基盤データが共有 | ディープコピー：完全独立コピーを作成 |
| **呼び出し方式** | 暗黙（代入/引数渡しが自動）        | 明示的（`.clone()`）      |
| **変更の影響** | 互いに影響（共有基盤データ）       | 互いに無影響（独立コピー）|
| **適用型**   | `&T` トークン、`ref T`              | Clone インターフェースを実装する任意型 |
| **コスト**   | ゼロオーバーヘッド（トークンは零サイズ型）| 型による                  |

**Dup は Clone を蕴含せず、Clone は Dup を蕴含しない**——これらは直交する2つの概念である：

```yaoxiang
// Dup 型：トークン複製、基盤データが共有
view: &Point = &p
view2 = view        // Dup：トークン複製、両者が同じ p を指す
print(view.x)       // 使用可
print(view2.x)      // 使用可、同一データを見ている

// 原型値型：コンパイラが自動値コピー（Dup ではない）
x: Int = 42
y = x               // 値コピー、x と y は完全に独立
print(x)            // 使用可

// Clone：明示的ディープコピー、独立コピーを作成
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone：ディープコピー、p は 여전히使用可能
r = p               // Move：所有権移動、Point は Dup でも原型値型でもないため
```

**設計意図**：

- Dup はトークン/参照型に使用し、「複数の視点で同一データを見る」問題を解決する
- Clone は独立コピーが必要なシナリオに使用し、コストを明示的にする
- 原型値型（Int/Float/Bool/Char）のコピーはコンパイラの組み込み動作であり、Dup ではない
- ほとんどのユーザー定義型はデフォルトで Move、ゼロコピーで高性能

## 第十二章：借用トークン型

### 12.1 核心概念

`&T` と `&mut T` は**ゼロサイズのコンパイル時トークン型**である。これらは「参照」ではなく、「アクセス権限の型レベル証明」である。

```
&T      →  ゼロサイズ、ソースデータを冻结（WriteToken の一時取得を禁止）、
          冻结保証下で複数読み取りが安全 → Dup（複製可）
&mut T  →  ゼロサイズ、排他的読み書き（他のトークンをすべて禁止）、
          排他アクセス下で複製は無意味 → Linear（非 Dup）
```

**主要特性**：

- トークンは**普通の型**であり、他のすべての型と同じスコープ規則を遵循する
- ライフタイム标注 `'a` が不要である
- 専用の借用検査器が不要——型属性（Dup/Linear）が権限を自然に推論する
- コンパイル後に完全に消失し、実行時オーバーヘッドゼロ

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
p.print()                       // OK、前のトークンは shift 呼び出し終了時に解放済み

// 複数 &T トークン共存——Dup 型は自由に複製可
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 トークンのスコープと伝播

トークンは普通の型であるため、すべての普通の型の操作をサポートしている：

**トークンを返す**——トークンは返り値とともに伝播する：

```yaoxiang
// ✅ 副トークンと親トークンを一緒に返す
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // トークンが呼び出し元に返される
print(px_ref)                    // OK、トークンはまだスコープ内
```

**構造体に格納**——構造体はトークンフィールドを持てる：

```yaoxiang
// ✅ 構造体がトークンをフィールドとして持つ
Window: Type = {
    target: Point,
    view: &Point,              // トークンフィールド——target への読み取り専用ビューを保持
}
```

**クロージャはキャプチャせず、文脈は作成点で固定**——クロージャは自分のパラメータのみを食べ、外層データが必要な場合はカリー化により作成点で値をクロージャに固定する：

```yaoxiang
// ✅ 文脈をカリー化で固定：threshold はパラメータ、gt_point(threshold) が作成点で値をクロージャに固定
gt_point: (t: Float) -> (p: Point) -> Bool = (p) => p.x > t
filter_by_threshold: (items: List(Point), threshold: Float) -> List(Point) = {
    items.filter(gt_point(threshold))
}
```

> 注：クロージャ（関数値）がエスケープ後、その定義处スコープが既に死んでいる可能性があるため、暗黙的に外層変数をキャプチャしてはならない；ただし呼び出し点（作成点）スコープは必ず生存しており、文脈はその点で値としてクロージャに固定するのが安全である。

### 12.4 自動借用選択

呼び出し側コンパイラは以下の優先順位で自動選択する：

```
1. 実引数が後続も使用される場合 → トークン作成を優先（&T または &mut T、メソッド署名による）
2. 実引数が後続使用されない場合 → Move
3. 優先匹配順位：&T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // print のパラメータ型は &Point → コンパイラが &Point トークンを作成
p.shift(1.0, 1.0)  // shift のパラメータ型は &mut Point → コンパイラが &mut Point トークンを作成
p2 = p             // 後続使用なし → Move
```

**メソッドレシーバは署名意味に従う**（RFC-011a レシーバスペル約束と同系統）：レシーバが `&T` → 読み取り借用トークン；`&mut T` → 変更借用トークン；値 →
Move（レシーバを消費）。呼び出し点で生成された借用トークンは呼び出し終了時に解放（過渡的、§12.5 区間意味論）；インターフェースの借用レシーバはインターフェース作者が `&Self` を明示的に宣言し、impl 署名は `Self ↦ impl 型` 置換後インターフェースと完全に一致する必要がある（RFC-011a §3）。

### 12.5 トークン衝突検出

トークン衝突検出は**借用のホール命題**（RFC-009a）であり、独立した流感性分析ではない。コンパイラは借用命題（`borrow_conflict`/`use_after_move`/`use_after_drop`/`mut_violation`）を自動生成し、証明管道に送信して検証する；トークン生存は区間 `[created_at, last_use]` である（RFC-009a §逆 BFS 生存分析参照）：

```yaoxiang
// ❌ &mut と派生 &T を同時に生存させられない
bad_alias: (p: &mut Point) -> Void = {
    p.x = 10.0                   // ✅ 正常使用 WriteToken
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

// ❌ 同一実引数で同時に &mut トークン和其他トークンを作成できない
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p が同時に &mut と & トークンを派生
```

### 12.6 コンパイラ内部：ブランド機構

ユーザーがブランドに触れることはない。コンパイラは内部で各トークンにコンパイル時一意識別子を割り当てている：

```
ユーザーが見るもの         コンパイラの内部表現
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N はコンパイル時一意整数
&mut Point     →  WriteToken(Point, #M)   // #M はコンパイル時一意整数
```

ブランドの用途：

- **偽造防止**：トークンは所有者のカプセルからのみ取得でき、凭空に構築できない
- **関連追跡**：フィールドアクセスから派生した `&Float` は派生ブランド（`#N.field_x`）を携带し、コンパイラが親トークンに追跡可能
- **衝突検出**：同源 WriteToken と派生 ReadToken は同時に生存できない

ブランドは単態化とインライン展開後に完全に消失し、生成された機械語中には存在しない。**実行時オーバーヘッドゼロ。**

### 12.7 トークン Sum 型

```
&BorrowToken ::= &T          // ReadToken（ソースデータを冻结 → Dup 安全）
               | &mut T      // WriteToken（排他的読み書き → Linear）
```

### 12.8 借用トークン vs ref

|        | `&T` / `&mut T`                  | `ref`                   |
| ------ | -------------------------------- | ----------------------- |
| 何をするか | 一瞥/その場で変更               | 共有所有                |
| 範囲   | トークン値のスコープに跟着        | スコープを跨ぐ          |
| コスト | ゼロオーバーヘッド（零サイズ型、コンパイル後消失）| Rc または Arc（コンパイラが選択）|
| エスケープ | 可（トークンは返り値/構造体を伝播）| そもそもエスケープ用    |
| 跨タスク | 不可（トークンは跨タスク传递未実装）| 可（コンパイラが Arc を自動選択）|
| 環検出 | 関係ない                           | タスク内は無言、跨タスクは lint |

> 注（未定義）：ref 作成後の内容の読み方（逆参照/メソッド/自動）はまだ仕様で定義されておらず、実装現状では `*a` で E1052 報エラー。定義後に本節に補完する。

---

## 付録：型定義早見表

### A.1 型定義

```
// === 記録型（波括弧） ===

// 記録型
Point: Type = { x: Float, y: Float }

// 変体付き記録型（函数字段を使用）
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// === インターフェース型（波括弧、全フィールドが関数） ===

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

// === 終了測度（組み込み述語、§8.4 参照） ===

// 一項：アンカーが束縛名（自己再帰関数、ループ）
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n { i = i + 1 }
    return acc
}

// 二項：測度の帰属を明示（測度が他で定義）
gcd_measure: (a: Int, b: Int) -> Int = { b }
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}
```

### A.2 ジェネリクス構文

```
// ジェネリクス型
List: (T: Type) -> Type = { data: Array(T), length: Int }
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// ジェネリクス関数
map: (T: Type, R: Type)(list: List(T), f: (T) -> R) -> List(R) = { ... }

// 型制約
clone: (T: Clone)(value: T) -> T = value.clone()
combine: (T: Clone + Add)(a: T, b: T) -> T = body

// 関連型
Iterator: (T: Type) -> Type = { Item: T, next: () -> Option(T) }

// コンパイル時ジェネリクス：N が型位置 (k: N) で参照 → コンパイル時値パラメータ
factorial: (N: Int)(k: N) -> Int = { ... }
Measure: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }

// 条件型
If: (C: Bool, T: Type, E: Type) -> Type = match C { True => T, False => E }

// 関数特殊化
sum: (arr: Array(Int)) -> Int = { ... }
sum: (arr: Array(Float)) -> Float = { ... }
```

### A.3 型属性早見表

```
// === Move（デフォルト） ===
// 全型はデフォルトで Move。代入、引数渡し、返り = 所有権移動

// === 原型値型（コンパイラの組み込み） ===
Int, Float,     // 代入時に自動値コピー、2つの値が完全に独立
Bool, Char      // Dup ではなく、コンパイラの原型に対する組み込み處理

// === Dup（シャローコピー：ハンドルを複製、基盤データを共有） ===
&T              // 零サイズ読み取りトークン、トークン複製 = 複数視点で同一データを指す
ref T           // Rc/Arc 複製 = 参照カウント+1、共有ヒープデータ

// === Linear ===
&mut T          // 零サイズ書き込みトークン、Linear（排他、複製不可）

// === Clone（明示的ディープコピー） ===
value.clone()   // 独立コピーを作成、変更は原値に影響しない
```

### A.4 借用トークン早見表

```
// === 借用トークン ===
&T              // 零サイズコンパイル時読み取りトークン、ソースデータを冻结 → Dup（複製可）
&mut T          // 零サイズコンパイル時書き込みトークン、排他的読み書き → Linear（複製不可）

// 呼び出し側自動選択
// 1. 実引数が後続も使用 → トークン作成
// 2. 実引数が後続使用なし → Move
// 3. 優先匹配：&T < &mut T < Move

// トークン伝播
// ✅ 返り可能、構造体に格納可能、クロージャにキャプチャ可能
// ❌ 跨タスク不可（トークンは跨タスク传递未実装）
```
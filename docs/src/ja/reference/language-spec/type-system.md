# 型システム仕様

本ドキュメントは、YaoXiang プログラミング言語の型システム仕様を定義する。基本型、複合型、generics、trait を含む。

---

## 第零章：理論的基礎

### 0.1 Curry-Howard 対応

Curry-Howard 対応（Curry-Howard
correspondence）は YaoXiang 型システムの理論的基礎である。これはプログラミング言語の型システムと数理論理学の間の深い対応関係を明らかにする：

| 論理学                         | プログラミング言語                            |
| ------------------------------ | --------------------------------------------- |
| 命題 \(P\)                     | 型 `Type`                                     |
| 証明 \(p: P\)                  | プログラム `x: T = ...`                       |
| 含意 \(P \rightarrow Q\)       | 関数型 `(P) -> Q`                             |
| 連言 \(P \wedge Q\)            | 積型 `{ a: P, b: Q }`                         |
| 選言 \(P \vee Q\)              | 和型 `{ a(P) \| b(Q) }`                       |
| 全称量化 \(\forall x:T. P(x)\) | generics `(T: Type) -> ...`                   |
| 真 \(\top\)                    | `Void`（Unit、デフォルト値を持つ）            |
| 偽 \(\bot\)                    | `Never`（零コンストラクタ、居住可能な値なし） |
| 型宇宙 \(Type_n : Type_{n+1}\) | 宇宙階層（Russell パラドックス防止）          |
| case 分析                      | 型レベル `match`                              |

> **注意**：型レベル `match` は分類討論（case
> analysis）であり、数学的帰納法ではない。帰納法には型レベル再帰関数 + コンパイラの停止性チェックが必要である。

### 0.2 型は命題、プログラムは証明

YaoXiang において、この対応関係は設計の最上位原則である：

- **停止する型レベル計算は正しい構成的証明に対応する**。YaoXiang の型族（`Nat` 上の `Add`
  の case 分析 + 再帰呼び出しなど）は本質的に数学的帰納法の型レベル符号化である——前提としてコンパイラが停止性チェックを行えること。
- **型検査は証明の検証である**。あるプログラムが型検査を通過するということは、論理的命題が構成的に証明されたことに相当する。

### 0.3 言語設計への影響

YaoXiang における Curry-Howard 対応の具体化：

1. **宇宙階層**（RFC-010）：`Type₀ : Type₁ : Type₂ …` により `Type: Type`
   が引き起こす論理的矛盾（Girard パラドックス）を回避
2. **型族**（RFC-011）：自然数 `Nat(Zero/Succ)`
   の型レベル case 分析 + 再帰呼び出しは Peano 公理に対応——前提としてコンパイラが停止性チェックを行うこと
3. **条件型**（RFC-011）：`If: (C: Bool, T: Type, E: Type) -> Type` は論理学の case 選言に対応
4. **値依存型**（RFC-011）：`Vec: (n: Int) -> Type`
   は「各整数 n に対して型が存在する」という有界量化に対応

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
> `Type` と `Expr` は独立した二つの AST 列挙型（`ast.rs:406` と `ast.rs:25`）であり、`TypeExpr`
> は BNF プレースホルダとして実装の `Type` 列挙型に対応し、「この位置は型を期待する」ことを示す。

---

## 第二章：基本型

### 2.1 primitive type

| 型       | 論理的対応   | 説明                                                                                                | デフォルトサイズ |
| -------- | ------------ | --------------------------------------------------------------------------------------------------- | ---------------- |
| `Type`   | —            | メタ型                                                                                              | 0 バイト         |
| `Never`  | ⊥（偽/空型） | 零コンストラクタ、いかなる値も持たない。発散/panic の返り型。`Never <: T` が任意の T に対して成立。 | 0 バイト         |
| `Void`   | ⊤（真/Unit） | デフォルト void 値を持つ零フィールド積型。`x: Void = <デフォルト>` は合法。                         | 0 バイト         |
| `Bool`   | —            | ブール値：`true` / `false`                                                                          | 1 バイト         |
| `Int`    | —            | 符号付き整数                                                                                        | 8 バイト         |
| `Uint`   | —            | 符号なし整数                                                                                        | 8 バイト         |
| `Float`  | —            | 浮動小数点数                                                                                        | 8 バイト         |
| `String` | —            | UTF-8 文字列                                                                                        | 可変             |
| `Char`   | —            | Unicode 文字                                                                                        | 4 バイト         |
| `Bytes`  | —            | 生のバイト列                                                                                        | 可変             |

ビット幅付き整数：`Int8`, `Int16`, `Int32`, `Int64`, `Int128` ビット幅付き浮動小数点：`Float32`,
`Float64`

### 2.2 Never と Void：⊥ と ⊤

`Never` と `Void` は型システムの論理的原始要素であり、それぞれ偽（⊥）と真（⊤）に対応する。

**Never（⊥、偽/空型）** — 譲歩不可能な三つの性質：

1. **零コンストラクタ**：いかなる literal や式も `Never` 型の値を生成できない。`x: Never = ...`
   には右辺として書けるものがない。
2. **爆発原理**：`Never <: T` が任意の型 `T` に対して成立する。`assert(false)` は `Never`
   を返し、その後のコードは型検査を通過できる（実際には実行されないが）。
3. **発散マーカー**：`f: (...) -> Never` は `f`
   が戻らないことを保証する。コンパイラはこれにより dead code 分析と `match` 分岐合流を行う。

`Never` は組み込み型名（`Int`/`Bool` と同じ登録パス）であり、キーワードではない。

**Void（⊤、真/Unit）** — ちょうど一つの居住者（デフォルト void 値）を持つ。`Void`
は零フィールド積型の単位元である。`x: Void = <デフォルト>` は合法であり、関数がデフォルトで `return`
を持たない場合は `Void` を返す。

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

// generics 付きの記録型
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
- フィールド名の後にコロンと型を続ける
- インターフェース名は型本体内に記述することでそのインターフェースの実装を示す

> **名前空間の所属**：`Type.name` 接頭辞（例：`Point.draw`）は関数が `Point`
> の名前空間に属することを示す。これはいかなる暗黙の束縛も引き起こさない。`p.draw()` のような `.`
> 呼び出し構文を有効にするには、明示的な束縛が必要：`Point.draw = draw[0]`。詳細は RFC-004 および RFC-010 を参照。

#### 3.1.1 フィールドのデフォルト値

型のフィールドにはデフォルト値を設定でき、構築時にはオプションで指定できる：

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

**規則**：

- `field: Type = expression` -> デフォルト値あり、構築時はオプション
- `field: Type` -> デフォルト値なし、構築時は必須

#### 3.1.2 組み込みバインディング

型定義本体内で直接メソッドを束縛できる：

```yaoxiang
// 方法 1：外部関数の参照による束縛
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]    // 位置 0 にバインド
}
// 呼び出し：p1.distance(p2) -> distance(p1, p2)

// 方法 2：無名関数 + 位置バインディング
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
// 直接代入（コンパイル時に具象型を決定可能 -> ゼロオーバーヘッド呼び出し）
d: Drawable = Circle(1)
d.draw(screen)        // コンパイル後：circle_draw を直接呼び出し、vtable なし

// 関数の戻り値（コンパイル時に決定不能 -> vtable 呼び出し）
d: Drawable = get_shape()
d.draw(screen)        // vtable 経由でメソッドを検索

// 関数の引数としてのインターフェース
process: (d: Drawable) -> Void = d.draw(screen)
```

**コンパイル時最適化戦略**：

| シナリオ         | 推論結果       | 呼び出し方式                       |
| ---------------- | -------------- | ---------------------------------- |
| 具象型の直接代入 | 具象型決定可能 | 直接呼び出し（ゼロオーバーヘッド） |
| 関数の戻り値     | 不明           | vtable                             |
| 異種コレクション | 複数型         | vtable                             |

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

## 第四章：generics

### 4.1 generics パラメータ構文

generics パラメータは関数型の一部であり、通常の引数と統一的に `()` 構文を使用する：

```
GenericType     ::= Identifier '(' TypeArgList ')'
TypeArgList     ::= TypeExpr (',' TypeExpr)* ','?
TypeBound       ::= Identifier
                 |  Identifier '+' Identifier ('+' Identifier)*
```

generics 型定義において、`(T: Type)` は型コンストラクタのパラメータシグネチャであり、`-> Type`
は返り型を示す：

````yaoxiang
List: (T: Type) -> Type = { ... }
Map: (K: Type, V: Type) -> Type = { ... }
``

### 4.1.1 コンテナ型（#299）

コンテナ型は generics 型コンストラクタであり、組み込み primitive ではない——ユーザーが定義した generics と同様に扱われ、統一された generics インスタンス化パスを経由して処理される：

| 型 | 意味 | バックエンド |
| --- | --- | --- |
| `List(T)` | 拡張可能リスト | `HeapValue::List` |
| `Array(T, N)` | 固定長配列（const generics N）| `HeapValue::Array` |
| `Dict(K, V)` | キー値マッピング | `HeapValue::Dict` |
| `Set(T)` | 集合（literal なし、`std.set` で構築）| — |

重要な規則：

- **literal の着地点は文脈が決定**：`[...]` 生 literal と `List(T)` 注釈は拡張可能リストに配置；`Array(T, N)` 注釈が literal に直接作用する場合は固定長配列に配置。
- **暗黙的な List→Array 変換を禁止**：固定長は型層で保証される——push は `List(A)` レシーバのみ受け入れる。
- **インデックス失敗契約**（実行時エラーは過渡期、目標状態はコンパイル時精密化でカバー、値依存型を経由）：
  - インデックス範囲外（負のインデックスを含む）→ `E6003`
  - Dict のキー欠落 → `E6008`
- **membership `in` 述語**：`Bool` を返しエラーを出さない、右オペランドは List/Array/Dict(キー)/Set/Tuple/String/Range をカバー。第一級ホーア述語であり、精密化型のコンパイル時証明可能命題の基底である。`

generics 関数において、型パラメータも同様にシグネチャで宣言され、コンパイラが実引数から自動的に推論する：

```yaoxiang
map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R)) = ...
````

### 4.2 generics 型定義

```yaoxiang
// 基本的な generics 型
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
    push: (self: List(T), item: T) -> Void,   // self は単なる規約上の名前であり、キーワードではない
    get: (self: List(T), index: Int) -> Option(T)
}
```

### 4.3 generics 構築呼び出しと型推論

generics 型定義のフィールドリストは**コンストラクタを自動生成する**：各フィールドが構築パラメータに対応し、フィールド名がパラメータ名となる。デフォルト値を持つフィールドは構築時に省略可能で、デフォルト値のないフィールドは必須である。関数型フィールド（メソッド）は構築パラメータを生成しない。

```yaoxiang
// 型定義
Container: (T: Type) -> Type = {
    value: T,        // デフォルト値なし -> 構築パラメータ必須
    extra: T,
}
// 自動展開された完全な形式（コンパイラ内部ビュー、ユーザー手書き不要）：
// Container: (T: Type) -> (value: T, extra: T) -> Type = {
//     value: T = value,
//     extra: T = extra,
// }

// 呼び出し：自動生成されたコンストラクタを呼び出す
c  = Container(42, 43)            // 構築パラメータはフィールド順に代入；T は要素から自動展開 = Int
c2 = Container("a", "b")          // T = String
c3 = Container(Int)(42, 43)       // 明示的型パラメータ + 位置式構築パラメータ
c4 = Container(Int)(extra=43, value=42)  // フィールド名式、順序は任意
c5 = Container(Int)()             // 空構築：フィールドはデフォルト値/ゼロ値を取得（データは後で代入）

// フィールドデフォルト値 -> 構築パラメータは省略可能
Point: (T: Type) -> Type = { x: T = 0, y: T = 0 }
p  = Point(1.5, 2.5)              // T = Float、x←1.5, y←2.5
p2 = Point(Int)()                 // x=0, y=0
```

**呼び出し規則**（単一括弧、宣言パラメータに逐次マッチング、左から右）：

1. 実引数を逐次的に型宣言パラメータにマッチング試行：`Type`
   位置は型実引数を受け入れ、コンパイル時値パラメータ位置（例：`Int`）はコンパイル時定数を受け入れる。
2. コンパイル時値パラメータ位置へのマッチングが成功した場合（部分マッチング）、型構築として処理：すべてのパラメータ位置を逐次検査し、エラー時は宣言順序に従って**最初にマッチしない/欠落しているパラメータを先に報告**。
3. 実引数が宣言パラメータに完全に対応しない場合（すべて値、コンパイル時値パラメータ位置にマッチング可なし）、構築パラメータとして処理：位置式はフィールド順で代入、型パラメータは要素型から自動展開。

```yaoxiang
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    _assert_rows: Assert(Rows > 0),
    data: Array(Array(T, Cols), Rows),
}

m: Matrix(Int, 3, 4)              // 型位置：一層型構築
m2 = Matrix(Int, 3, 4)(data=[[1,2,3,4],[5,6,7,8],[9,10,11,12]])  // 二層：型 + 構築パラメータ
m3 = Matrix(Int, 3, 4)()          // 空構築（RFC-011 §9.3 パターン、データは後で代入）

Matrix(42)    // ❌ 位置 0: T←42 不一致（42 は型ではない）；位置 1: Rows←42 マッチ；
              //    位置 2: Cols 欠落 -> 最初のエラーを先に報告：T は Type を期待、42 を受信
Container(42) // ❌ 構築パラメータ extra 欠落
Container(42, 43, 44)  // ❌ 構築パラメータ超過
```

**型推論**：generics 型コンストラクタの型パラメータは構築パラメータの要素から自動展開される（`Container(42, 43)`
→ T=Int）；generics 関数の型パラメータは実引数の型から自動展開される（`map(numbers, f)` → T=Int,
R=String、§4.1 参照）。展開不可能な場合は明示的に指定する必要がある。

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

// generics コンテナのソート
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

### 6.2 generics 関連型（GAT）

```yaoxiang
// より複雑な関連型
Container: (T: Type) -> Type = {
    Item: T,
    IteratorType: Iterator(T),  // 関連型も generics
    iter: () -> IteratorType
}
```

---

## 第七章：コンパイル時 generics

### 7.1 コンパイル時値パラメータ

```
LiteralType   ::= Identifier ':' Int          // コンパイル時定数（候補）
```

> **訂正（#296）**：原文ではコンパイル時値パラメータを「デフォルトでコンパイル時確定」と記載していたが、この記述は**厳密に誤り**である——
> `add: (a: Int, b: Int) -> Int = a + b` において `a`/`b`
> は実行時値パラメータである。**型位置で参照される**具体的な型パラメータのみがコンパイル時値パラメータとなる。正しい定義は下記参照。

**用語**：`Type`
以外の具体的な型（例：`Int`）で标注された generics パラメータを**コンパイル時値パラメータ候補**と呼び、コンパイル時値パラメータになるかどうかは値が型位置で参照されるか（値依存）による。**`const`
キーワードは不要**（実装内部ではかつて「const
generics」を使用していたが、ドキュメントは統一して「コンパイル時値パラメータ」を使用）。

**判定規則（二段階）**：

1. **形態粗筛**：パラメータ标注が `Type` 以外の具体的な型（`Int`/`Bool`/`Float`）→ 候補。
2. **用途精筛**：候補名が**型位置**に現れる（型本体のフィールド型、内側 `Fn` パラメータ型、`Assert`
   述語、`Array(T, N)`
   型構築実引数位置）→ 真のコンパイル時値パラメータ；それ以外は**実行時値パラメータ**。

| 書き方                                                     | 判定                           | 理由                                |
| ---------------------------------------------------------- | ------------------------------ | ----------------------------------- |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b は実行時値パラメータ       | 値位置にのみ出現                    |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N はコンパイル時値パラメータ   | N が型構築実引数位置に出現          |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N はコンパイル時値パラメータ   | N が内側パラメータ k の型として使用 |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N は落空 -> 実行時値パラメータ | N が型本体で参照されていない        |

**核心設計**：`(N: Int)` コンパイル時値パラメータ + `(k: N)`
値パラメータを使用して、コンパイル時定数と実行時値を区別する。落空候補（形態は候補だが用途が未命中）は実行時値パラメータに退化する——関数レベルでは既にこの通り処理されているが、型コンストラクタパスは
[issue #297](https://github.com/ChenXu233/YaoXiang/issues/297) 参照。

```yaoxiang
// コンパイル時値パラメータ：N が型位置（Array の長さスロット）で参照される
StaticArray: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // N が型構築実引数位置に出現 -> コンパイル時値パラメータ
    length: N
}

// 使用方法：factorial(5) は型位置で評価（コンパイル時）され、結果 120 が型に埋め込まれる
arr: StaticArray(Int, factorial(5))  // コンパイラがコンパイル時に factorial(5) = 120 を計算

// 値依存：N が内側パラメータ k の型として使用
// N はコンパイル時値パラメータ（(k: N) の型位置に出現）；
// k は実行時値パラメータ、その型は literal 型 N（単一値型）。
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
// IsTrue ブリッジと Assert 精密化型（詳細は §8.3）
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

### 8.3 Assert 精密化型と assert アサーション

`assert` と `Assert`
は同じ精密化原始要素の二つの側面であり、dispatch 分派パイプラインにより「述語の自由変数がコンパイル時に到達可能か」で自動選択される。

**核心シグネチャ**：`assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**dispatch 分派規則**：

| 判定基準                                                                    | モード      | 振る舞い                                                                             |
| --------------------------------------------------------------------------- | ----------- | ------------------------------------------------------------------------------------ |
| すべての自由変数がコンパイル時既知（generics パラメータ、コンパイル時定数） | CompileTime | 証明パイプラインへ：true → Void に消去、false → コンパイルエラー（Never は居住不可） |
| 実行時自由変数が存在（関数パラメータ、外部入力）                            | Runtime     | 実行時 Bool チェックを挿入し、フロー敏感仮定集合 Γ に精密化事実を注入                |

**フロー敏感仮定集合 Γ**：

コンパイラは各制御流点の既知命題集合を保持する：

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP 伝播
mut x = x - 5       // Γ = {}  ← mut kill set：古い仮定が無効化される
```

`mut` 変数代入後、その変数に関するすべての仮定が削除される（kill
set）。分岐合流時 Γ は各分岐の積集合を取る。

---

## 第九章：型のユニオンと交差

### 9.1 型のユニオン

```
TypeUnion     ::= TypeExpr '|' TypeExpr
```

### 9.2 型の交差

```
TypeIntersection ::= TypeExpr '&' TypeExpr
```

**構文**：型交差 `A & B` は A と B を同時に満たす型を表す

```yaoxiang
// インターフェース合成 = 型の交差
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
// プラットフォーム型列挙（標準ライブラリ定義）
Platform: Type = { X86_64: () -> Platform, AArch64: () -> Platform, RISC_V: () -> Platform, ARM: () -> Platform, X86: () -> Platform }

// P は事前定義された generics パラメータ名で、現在のコンパイルプラットフォームを表す
sum: (P: X86_64)(arr: Array(Float)) -> Float = {
    return avx2_sum(arr.data, arr.length)
}

sum: (P: AArch64)(arr: Array(Float)) -> Float = {
    return neon_sum(arr.data, arr.length)
}
```

---

## 第十一章：型属性

YaoXiang には区別が必要な型属性は一つしかない：linear vs コピー可能。コンパイラが自動推論する。

### 11.1 Move（デフォルトの所有権移転）

すべての型はデフォルトで Move セマンティクスに従う。代入、引数渡し、戻り値 = 所有権移転。

```yaoxiang
p: Point = Point(1.0, 2.0)
q = p           // Move、p は二度と読めない
```

### 11.2 Dup（シャローコピー：ハンドルをコピー、データを共有）

**Dup 属性は参照/トークン型に使用される**。Dup 型の代入 = シャローコピー——ハンドル/トークンをコピーし、底层のデータを共有する。複数の保持者が同じデータブロックを指す。

| 型               | 属性   | 説明                                                                      |
| ---------------- | ------ | ------------------------------------------------------------------------- |
| `&T`             | Dup    | ゼロサイズ読み取りトークン、トークンコピー = 同じデータに対する複数の視点 |
| `ref T`          | Dup    | Rc/Arc コピー = 参照カウント+1、ヒープデータ共有                          |
| `&mut T`         | Linear | ゼロサイズ書き込みトークン、独占、コピー不可                              |
| その他すべての型 | Move   | デフォルト所有権移転                                                      |

**primitive 値型**（Int, Float, Bool,
Char）はコンパイラ組み込みの特殊処理：代入時に自動的に値コピーされ、二つの値は完全に独立している。これはコンパイラのネイティブ動作であり、Dup 型属性には属さない。

```yaoxiang
// &T: Dup、自由なエイリアス可能
view: &Point = &p
view2 = view     // Dup：トークンをコピー、両方とも有効
print(view.x)    // 使用可能
print(view2.x)   // 使用可能

// &mut T: Linear、コピー不可
mut_ref: &mut Point = &mut p
// r2 = mut_ref  // ❌ &mut T は Dup ではない、コピー不可
```

### 11.3 Clone（明示的ディープコピー）と Dup の関係

**Clone** は明示的ディープコピーインターフェースである。すべての型は Clone を実装でき、`.clone()`
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

|                    | Dup                                                       | Clone                                    |
| ------------------ | --------------------------------------------------------- | ---------------------------------------- |
| **セマンティクス** | シャローコピー：ハンドル/トークンをコピー、底层データ共有 | ディープコピー：完全に独立したコピー作成 |
| **呼び出し方式**   | 暗黙的（代入/引数渡しで自動）                             | 明示的（`.clone()`）                     |
| **変更影響**       | 互いに影響（底层データ共有）                              | 互いに影響なし（独立コピー）             |
| **適用型**         | `&T` トークン、`ref T`                                    | Clone インターフェースを実装する任意の型 |
| **コスト**         | ゼロオーバーヘッド（トークンはゼロサイズ型）              | 型による                                 |

**Dup は Clone を意味せず、Clone は Dup を意味しない**——これらは直交する二つの概念である：

```yaoxiang
// Dup 型：トークンをコピー、底层データを共有
view: &Point = &p
view2 = view        // Dup：トークンをコピー、両方とも同じ p を指す
print(view.x)       // 使用可能
print(view2.x)      // 使用可能、同じデータが見える

// primitive 値型：コンパイラが自動値コピー（Dup ではない）
x: Int = 42
y = x               // 値コピー、x と y は完全に独立
print(x)            // 使用可能

// Clone：明示的ディープコピー、独立コピー作成
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone：ディープコピー、p はまだ使用可能
r = p               // Move：所有権移転、Point は Dup でも primitive 値型でもない
```

**設計意図**：

- Dup はトークン/参照型に使用され、「同じデータに対する複数の視点」という問題を解決する
- Clone は独立コピーが必要なシナリオに使用され、明示的呼び出しによりコストを可視化する
- primitive 値型（Int/Float/Bool/Char）のコピーはコンパイラの組み込み動作であり、Dup には属さない
- ほとんどのユーザー定義型はデフォルトで Move であり、ゼロコピーで高性能

## 第十二章：借用トークン型

### 12.1 核心概念

`&T` と `&mut T`
は**ゼロサイズのコンパイル時トークン型**である。これらは「参照」ではなく、「アクセス権限の型レベル証明」である。

```
&T      →  ゼロサイズ、ソースデータを凍結（期間中 WriteToken の取得を禁止）、
          凍結保証下で複数読み取り専用が安全 -> Dup（コピー可能）
&mut T  →  ゼロサイズ、排他的読み書き（他のすべてのトークンを禁止）、
          排他アクセス下ではコピーが無意味 -> Linear（Dup ではない）
```

**重要な特性**：

- トークンは**通常の型**であり、他のすべての型と同じスコープ規則に従う
- ライフタイム注釈 `'a` は不要
- 専用の借用チェッカーは不要——型属性（Dup/Linear）が自然に権限を推論する
- コンパイル後完全に消滅し、ゼロ実行時オーバーヘッド

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
p.print()                       // OK、前のトークンは shift 呼び出し終了とともに解放済み

// 複数の &T トークンが共存——Dup 型は自由なコピーを許可
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 トークンのスコープと伝播

トークンは通常の型であるため、通常の型のすべての操作をサポートする：

**トークンを返す**——トークンは戻り値とともに伝播する：

```yaoxiang
// ✅ 子トークンと親トークンを一緒に返す
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // トークンは呼び出し元に返される
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

**クロージャはキャプチャせず、コンテキストは作成時点で固定化される**——クロージャは独自のパラメータのみを受け取り、外側データが必要な場合はカリー化により作成時点で値をクロージャ内に固定化する：

```yaoxiang
// ✅ コンテキストはカリー化で固定化：threshold はパラメータ、gt_point(threshold) は作成時点で値をクロージャに固定化
gt_point: (t: Float) -> (p: Point) -> Bool = (p) => p.x > t
filter_by_threshold: (items: List(Point), threshold: Float) -> List(Point) = {
    items.filter(gt_point(threshold))
}
```

> 注：クロージャ（関数値）がエスケープした後、その定義箇所のスコープはすでに死んでいる可能性があるため、外側の変数を暗黙的にキャプチャしてはならない；しかし呼び出し箇所（作成箇所）のスコープは確実に生存しており、その時点でコンテキストが値として固定化されてクロージャに入ることは安全である。

### 12.4 自動借用選択

呼び出し側コンパイラは以下の優先順位で自動選択する：

```
1. 実引数が後に使用される場合 -> トークン作成を優先（メソッドシグネチャに応じて &T または &mut T）
2. 実引数が後に使用されない場合 -> Move
3. マッチング優先順位：&T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // print のパラメータ型は &Point -> コンパイラが &Point トークンを作成
p.shift(1.0, 1.0)  // shift のパラメータ型は &mut Point -> コンパイラが &mut Point トークンを作成
p2 = p             // 後は使用されない -> Move
```

### 12.5 トークン衝突検出

トークン衝突検出は**借用ホーア命題**（RFC-009a）であり、独立したフロー敏感分析ではない。コンパイラが借用命題を自動生成（`borrow_conflict`/`use_after_move`/`use_after_drop`/`mut_violation`）し、証明パイプラインに送って検証する。トークンの生存性は区間
`[created_at, last_use]`（RFC-009a §逆 BFS 生存性分析参照）：

```yaoxiang
// ❌ &mut と派生した &T は同時に生存できない
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

ユーザーはブランドに一切触れない。コンパイラは内部で各トークンにコンパイル時一意識別子を割り当てる：

```
ユーザーが見るもの       コンパイラ内部表現
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N はコンパイル時一意整数
&mut Point     →  WriteToken(Point, #M)   // #M はコンパイル時一意整数
```

ブランドの用途：

- **偽造防止**：トークンは所有者カプセルからのみ取得でき、凭空に構築できない
- **関連追跡**：フィールドアクセスから派生した `&Float`
  は派生ブランド（`#N.field_x`）を携带し、コンパイラが親トークンまで追跡可能
- **衝突検出**：同源 WriteToken と派生 ReadToken は同時に生存できない

ブランドは単態化とインライン化後に完全に消滅し、生成される機械語には存在しない。**ゼロ実行時オーバーヘッド。**

### 12.7 トークン Sum 型

```
&BorrowToken ::= &T          // ReadToken（ソースデータ凍結 -> Dup 安全）
               | &mut T      // WriteToken（排他的読み書き -> Linear）
```

### 12.8 借用トークン vs ref

|            | `&T` / `&mut T`                                      | `ref`                               |
| ---------- | ---------------------------------------------------- | ----------------------------------- |
| 機能       | 一目見る/その場で変更                                | 共有保持                            |
| 範囲       | トークン値のスコープに従う                           | スコープを超える                    |
| コスト     | ゼロオーバーヘッド（ゼロサイズ型、コンパイル後消滅） | Rc または Arc（コンパイラが選択）   |
| エスケープ | 可（トークンは戻り値/構造体で伝播）                  | 本来エスケープ用                    |
| タスク越え | 不可（トークンはタスク越え渡し未実装）               | 可（コンパイラが自動的に Arc 選択） |
| 環検出     | 関与しない                                           | タスク内は静的、タスク越えは lint   |

> 注（未定義）：ref 作成後の内容読み取り（dereference/メソッド/自動）についてはまだ仕様で定義されておらず、実装現状では
> `*a` は E1052 を報告する。定義後に本節に補足予定。

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

### A.2 generics 構文

```
// generics 型
List: (T: Type) -> Type = { data: Array(T), length: Int }
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// generics 関数
map: (T: Type, R: Type)(list: List(T), f: (T) -> R) -> List(R) = { ... }

// 型制約
clone: (T: Clone)(value: T) -> T = value.clone()
combine: (T: Clone + Add)(a: T, b: T) -> T = body

// 関連型
Iterator: (T: Type) -> Type = { Item: T, next: () -> Option(T) }

// コンパイル時 generics：N が型位置 (k: N) で参照される -> コンパイル時値パラメータ
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
Bool, Char      // Dup ではなく、コンパイラによる primitive の組み込み処理

// === Dup（シャローコピー：ハンドルをコピー、底层データを共有） ===
&T              // ゼロサイズ読み取りトークン、トークンコピー = 同じデータに対する複数の視点
ref T           // Rc/Arc コピー = 参照カウント+1、ヒープデータ共有

// === Linear ===
&mut T          // ゼロサイズ書き込みトークン、Linear（独占、コピー不可）

// === Clone（明示的ディープコピー） ===
value.clone()   // 独立コピー作成、変更は元値に影響しない
```

### A.4 借用トークンクイックリファレンス

```
// === 借用トークン ===
&T              // ゼロサイズコンパイル時読み取りトークン、ソースデータ凍結 -> Dup（コピー可能）
&mut T          // ゼロサイズコンパイル時書き込みトークン、排他的読み書き -> Linear（コピー不可）

// 呼び出し側の自動選択
// 1. 実引数が後に使用される -> トークン作成
// 2. 実引数が後に使用されない -> Move
// 3. マッチング優先順位：&T < &mut T < Move

// トークン伝播
// ✅ 戻り値可能、構造体格納可能、クロージャキャプチャ可能
// ❌ タスク越え不可（トークンはタスク越え渡し未実装）
```

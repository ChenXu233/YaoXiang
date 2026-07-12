# 型システム仕様

本ドキュメントは YaoXiang プログラミング言語の型システム仕様を定義する。基本型、複合型、ジェネリクス、trait を含む。

---

## 第零章：理論的基礎

### 0.1 カリー・ハワード同型対応

カリー・ハワード同型対応（Curry-Howard correspondence）は YaoXiang の型システムの理論的基礎である。これはプログラミング言語の型システムと数理論理の間の深い対応関係を明らかにする：

| 論理学 | プログラミング言語 |
|--------|----------|
| 命題 \(P\) | 型 `Type` |
| 証明 \(p: P\) | プログラム `x: T = ...` |
| 蕴含 \(P \rightarrow Q\) | 関数型 `(P) -> Q` |
| 連言 \(P \wedge Q\) | 積型 `{ a: P, b: Q }` |
| 選言 \(P \vee Q\) | 和型 `{ a(P) \| b(Q) }` |
| 全称量化 \(\forall x:T. P(x)\) | ジェネリクス `(T: Type) -> ...` |
| 真 \(\top\) | `Void`（Unit、デフォルト値あり） |
| 偽 \(\bot\) | `Never`（零コンストラクタ、いかなる値も居住できない） |
| 型宇宙 \(Type_n : Type_{n+1}\) | 宇宙階層（ラッセルのパラドックス防止） |
| ケース分析 | 型レベル `match` |

> **注意**：型レベル `match` は場合分け（case analysis）であり、数学的帰納法ではない。帰納法には型レベル再帰関数とコンパイラの停止性検査が必要である。

### 0.2 型は命題、プログラムは証明

YaoXiang において、この対応関係は設計の第一級の原則である：

- **停止する型レベル計算は正しい構成的証明に対応する**。YaoXiang の型族（`Nat` 上の `Add` など）の case 分析 + 再帰呼び出しは本質的に数学的帰納法の型レベル符号化である——前提としてコンパイラが停止性検査を行うこと。
- **型検査は証明の検証である**。プログラムが型検査を通るということは、論理命題が構成的に証明されたことに相当する。

### 0.3 言語設計への影響

YaoXiang におけるカリー・ハワード同型対応の具体化：

1. **宇宙階層**（RFC-010）：`Type₀ : Type₁ : Type₂ …` により `Type: Type` が引き起こす論理的パラドックス（Girard のパラドックス）を回避
2. **型族**（RFC-011）：自然数 `Nat(Zero/Succ)` の型レベル case 分析 + 再帰呼び出しは Peano 公理に対応する——前提としてコンパイラが停止性検査を行うこと
3. **条件型**（RFC-011）：`If: (C: Bool, T: Type, E: Type) -> Type` は論理における case 選言に対応する
4. **値依存型**（RFC-011）：`Vec: (n: Int) -> Type` は「すべての整数 n に対して型が存在する」という有量化に対応する

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

> **設計説明**：RFC-010 は「すべてが代入である」という統一モデル（`name: type = value`）を提案しているが、構文レベルでは型と値を区別する必要がある。コンパイラ実装において `Type` と `Expr` は二つの独立した AST enum（`ast.rs:406` および `ast.rs:25`）であり、`TypeExpr` は BNF 上のプレースホルダとして実装の `Type` enum に対応し、「この位置に型が期待される」ことを表す。

---

## 第二章：基本型

### 2.1 プリミティブ型
| 型 | 論理的対応 | 説明 | デフォルトサイズ |
|------|---------|------|----------|
| `Type` | — | メタ型 | 0 バイト |
| `Never` | ⊥（偽/空型） | 零コンストラクタ、いかなる値もない。発散/panic の戻り型。`Never <: T` は任意の T に対し成立。 | 0 バイト |
| `Void` | ⊤（真/Unit） | デフォルト void 値を持つ零フィールド積型。`x: Void = <デフォルト>` が合法。 | 0 バイト |
| `Bool` | — | ブール値：`true` / `false` | 1 バイト |
| `Int` | — | 符号付き整数 | 8 バイト |
| `Uint` | — | 符号なし整数 | 8 バイト |
| `Float` | — | 浮動小数点数 | 8 バイト |
| `String` | — | UTF-8 文字列 | 可変 |
| `Char` | — | Unicode 文字 | 4 バイト |
| `Bytes` | — | 生バイト列 | 可変 |

ビット幅付き整数：`Int8`, `Int16`, `Int32`, `Int64`, `Int128`
ビット幅付き浮動小数点：`Float32`, `Float64`

### 2.2 Never と Void：⊥ と ⊤

`Never` と `Void` は型システムの論理的プリミティブであり、それぞれ偽（⊥）と真（⊤）に対応する。

**Never（⊥、偽/空型）** — 譲歩できない三性質：

1. **零コンストラクタ**：`Never` 型の値を生成できるリテラルや式は一切ない。`x: Never = ...` の右辺に書けるものがない。
2. **爆発原理**：`Never <: T` は任意の型 `T` に対し成立する。`assert(false)` は `Never` を返し、後続コードは型検査を通る（実際には決して実行されないが）。
3. **発散マーカー**：`f: (...) -> Never` は `f` が返らないことを保証する。コンパイラはこれに基づき dead code 分析と `match` 分岐合流を行う。

`Never` は組込型名（`Int`/`Bool` と同じ登録パス）であり、キーワードではない。

**Void（⊤、真/Unit）** — 正確に一つの居住者（デフォルト void 値）を持つ。`Void` は零フィールド積型の単位元である。`x: Void = <デフォルト>` が合法であり、`return` 文を持たない関数のデフォルト戻り型は `Void` である。


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

// ジェネリック付きレコード型
Pair: (T: Type) -> Type = { first: T, second: T }

// インタフェース実装レコード型
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
- インタフェース名を型本体に記述することでそのインタフェースを実装する

> **名前空間归属**：`Type.name` プレフィックス（例：`Point.draw`）は関数が `Point` の名前空間に属することを示す。これは暗黙的束縛を引き起こさない。`p.draw()` のような `.` 呼び出し構文を有効化するには、明示的な束縛が必要である：
> `Point.draw = draw[0]`。
> 詳細は RFC-004 および RFC-010 を参照。

#### 3.1.1 フィールドデフォルト値

型のフィールドにデフォルト値を指定でき、構築時にはオプションで提供できる：

```yaoxiang
// デフォルト値を持つフィールド - 構築時はオプション
Point: Type = {
    x: Float = 0,
    y: Float = 0
}

// 使用例
Point()           // -> Point(x=0, y=0)
Point(x=1)       // -> Point(x=1, y=0)
Point(x=1, y=2) // -> Point(x=1, y=2)

// デフォルト値のないフィールド - 構築時は必須
Point2: Type = {
    x: Float,
    y: Float
}

// 使用例
Point2(x=1, y=2) // 正しい
Point2()          // エラー
```

**ルール**：
- `field: Type = expression` -> デフォルト値あり、構築時オプション
- `field: Type` -> デフォルト値なし、構築時必須

#### 3.1.2 組込束縛

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

**インタフェース実装**：型は定義末尾にインタフェース名を列挙することで実装する

```yaoxiang
// インタフェースを実装する型
Point: Type = {
    x: Float,
    y: Float,
    Drawable,        // Drawable インタフェースを実装
    Serializable     // Serializable インタフェースを実装
}
```

**インタフェース直接代入**：具象型はインタフェース型変数に直接代入できる（構造的サブタイピング）

```yaoxiang
// 直接代入（コンパイル時に具象型が判明 -> ゼロオーバーヘッド呼び出し）
d: Drawable = Circle(1)
d.draw(screen)        // コンパイル後：circle_draw を直接呼び出し、vtable なし

// 関数の戻り値（コンパイル時に不明 -> vtable 呼び出し）
d: Drawable = get_shape()
d.draw(screen)        // vtable 経由でメソッド検索

// インタフェースを関数引数に
process: (d: Drawable) -> Void = d.draw(screen)
```

**コンパイル時最適化戦略**：

| シナリオ | 推論結果 | 呼び出し方式 |
|------|----------|----------|
| 具象型を直接代入 | 具象型が判明 | 直接呼び出し（ゼロオーバーヘッド） |
| 関数の戻り値 | 不明 | vtable |
| 異種コレクション | 複数型 | vtable |

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

### 4.1 ジェネリック引数構文

ジェネリック引数は関数型の一部であり、通常引数と同様に `()` 構文を用いる：

```
GenericType     ::= Identifier '(' TypeArgList ')'
TypeArgList     ::= TypeExpr (',' TypeExpr)* ','?
TypeBound       ::= Identifier
                 |  Identifier '+' Identifier ('+' Identifier)*
```

ジェネリック型定義において、`(T: Type)` は型コンストラクタのパラメータシグネチャであり、`-> Type` は戻り型を表す：

```yaoxiang
List: (T: Type) -> Type = { ... }
Map: (K: Type, V: Type) -> Type = { ... }
```

ジェネリック関数においても、型引数はシグネチャで宣言され、コンパイラが実引数から自動推論する：

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
    push: (self: List(T), item: T) -> Void,   # self は単なる慣習名でありキーワードではない
    get: (self: List(T), index: Int) -> Option(T)
}
```

### 4.3 型推論

```yaoxiang
// コンパイラがジェネリック引数を自動推論
numbers: List(Int) = List(1, 2, 3)  // コンパイラが List(Int) を推論
```

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

### 6.1 関連型定義

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
CompileTimeFn ::= '(' Identifier ':' Int ')' '(' Identifier ')' '->' TypeExpr
```

**核心設計**：`(n: Int)` ジェネリック引数 + `(n: n)` 値引数で、コンパイル時定数とランタイム値を区別する。

```yaoxiang
// コンパイル時階乗：引数はコンパイル時に既知のリテラルでなければならない
factorial: (n: Int)(n: n) -> Int = {
    match n {
        0 => 1,
        _ => n * factorial(n - 1)
    }
}

// コンパイル時定数配列
StaticArray: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // コンパイル時にサイズ既知の配列
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
# IsTrue ブリッジと Assert 精錬型（詳細は §8.3）
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      # ⊤、プログラム続行
    false => Never,    # ⊥、発散/コンパイルエラー
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

`assert` と `Assert` は同一の精錬プリミティブの二面であり、dispatch 分岐パイプラインが「述語の自由変数がコンパイル時に到達可能か」によって自動選択する。

**核心シグネチャ**：`assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**dispatch 分岐ルール**：

| 判別基準 | モード | 振る舞い |
|------|------|------|
| すべての自由変数がコンパイル時既知（ジェネリック引数、コンパイル時定数） | CompileTime | 証明パイプラインへ：true → Void に消去、false → コンパイルエラー（Never は居住不可） |
| ランタイム自由変数が存在する（関数引数、外部入力） | Runtime | ランタイム Bool 検査を挿入し、フロー敏感仮定集合 Γ に精錬事実を注入 |

**フロー敏感仮定集合 Γ**：

コンパイラは各制御フロー点における既知命題集合を保持する：

```yaoxiang
assert(x > 0)       # Γ = {x > 0}
y = x + 1           # Γ = {x > 0, y > 1}  ← SP 伝播
mut x = x - 5       # Γ = {}  ← mut kill set：旧仮定失効
```

`mut` 変数代入後、当該変数に関するすべての仮定が除去される（kill set）。分岐合流時 Γ は各分岐の交差集合を取る。

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

**構文**：型交差 `A & B` は A と B を同時に満たす型を表す

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
// プラットフォーム型 enum（標準ライブラリで定義）
Platform: Type = X86_64 | AArch64 | RISC_V | ARM | X86

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

YaoXiang には区別すべき型属性が一つだけある：線形 vs 複製可能。コンパイラが自動推論する。

### 11.1 Move（デフォルト所有権移転）

すべての型はデフォルトで Move セマンティクスに従う。代入、引数渡し、戻り値 = 所有権移転。

```yaoxiang
p: Point = Point(1.0, 2.0)
q = p           // Move、p は以降読み取り不可
```

### 11.2 Dup（シャローコピー：ハンドル複製、データ共有）

**Dup 属性は参照/トークン型に用いる**。Dup 型の代入 = シャローコピー——ハンドル/トークンを複製し、底层データを共有する。複数の保持者が同一データブロックを指す。

| 型 | 属性 | 説明 |
|------|------|------|
| `&T` | Dup | ゼロサイズ読み取りトークン。トークン複製 = 同一データへの複数視点 |
| `ref T` | Dup | Rc/Arc 複製 = 参照カウント+1、ヒープデータ共有 |
| `&mut T` | Linear | ゼロサイズ書き込みトークン、排他的、複製不可 |
| その他すべての型 | Move | デフォルト所有権移転 |

**プリミティブ値型**（Int, Float, Bool, Char）はコンパイラが内蔵する特殊処理：代入時に自動値コピーされ、二つの値は完全に独立。これはコンパイラのネイティブ動作であり、Dup 型属性には属さない。

```yaoxiang
// &T: Dup、自由に別名付け可能
view: &Point = &p
view2 = view     // Dup：トークン複製、両者ともに有効
print(view.x)    // 使用可
print(view2.x)   // 使用可

// &mut T: Linear、複製不可
mut_ref: &mut Point = &mut p
// r2 = mut_ref  // ❌ &mut T は Dup ではないため複製不可
```

### 11.3 Clone（明示的深コピー）と Dup の関係

**Clone** は明示的深コピーインタフェースである。すべての型は Clone を実装可能で、`.clone()` メソッドを提供する。

```yaoxiang
// Clone インタフェース定義（標準ライブラリ）
Clone: Type = {
    clone: () -> Clone
}

// 使用例
p: Point = Point(1.0, 2.0)
backup = p.clone()    // 深コピー、p は依然として使用可能
p2 = p.clone()        // 複数回クローン可能
```

**Dup と Clone の違い**：

| | Dup | Clone |
|---|---|---|
| **意味** | シャローコピー：ハンドル/トークン複製、底层データ共有 | 深コピー：完全に独立した副本を作成 |
| **呼び出し方式** | 暗黙的（代入/引数渡しで自動） | 明示的（`.clone()`） |
| **変更影響** | 相互に影響（底层データ共有） | 互いに影響せず（独立副本） |
| **適用型** | `&T` トークン、`ref T` | Clone インタフェースを実装する任意の型 |
| **コスト** | ゼロオーバーヘッド（トークンはゼロサイズ型） | 型に依存 |

**Dup は Clone を蕴含せず、Clone も Dup を蕴含しない**——これらは二つの直交概念である：

```yaoxiang
// Dup 型：トークン複製、底层データ共有
view: &Point = &p
view2 = view        // Dup：トークン複製、両者が同一の p を指す
print(view.x)       // 使用可
print(view2.x)      // 使用可、同一データを参照

// プリミティブ値型：コンパイラが自動値コピー（Dup ではない）
x: Int = 42
y = x               // 値コピー、x と y は完全に独立
print(x)            // 使用可

// Clone：明示的深コピー、独立副本作成
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone：深コピー、p は依然として使用可能
r = p               // Move：所有権移転、Point は Dup でもプリミティブ値型でもないため
```

**設計意図**：
- Dup はトークン/参照型に用い、「同一データの複数視点」問題を解決する
- Clone は独立副本が必要なシナリオに用い、明示的呼び出しによりコストを可視化する
- プリミティブ値型（Int/Float/Bool/Char）の複製はコンパイラの組込動作であり、Dup に属さない
- ほとんどのカスタム型はデフォルト Move で、ゼロコピー高性能

## 第十二章：借用トークン型

### 12.1 核心概念

`&T` と `&mut T` は**ゼロサイズのコンパイル時トークン型**である。これらは「参照」ではなく、「アクセス権限の型レベル証明」である。

```
&T      →  ゼロサイズ、元データを凍結（期間中 WriteToken 取得を禁止）、
          凍結保証下では複数読み取りが安全 → Dup（複製可能）
&mut T  →  ゼロサイズ、排他的読み書き（他トークンすべて禁止）、
          排他的アクセス下では複製が無意味 → Linear（非 Dup）
```

**主要特性**：
- トークンは**通常の型**であり、他のすべての型と同じスコープルールに従う
- ライフタイム注釈 `'a` は不要
- 専用借用チェッカーは不要——型属性（Dup/Linear）が自然に権限を推論する
- コンパイル後完全に消滅し、ゼロランタイムオーバーヘッド

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

// 呼び出し側：コンパイラが借用または Move を自動選択
p = Point(1.0, 2.0)
p.print()                       // コンパイラが &Point トークンを自動生成
p.shift(1.0, 1.0)               // コンパイラが &mut Point トークンを自動生成
p.print()                       // OK、前のトークンは shift 呼び出し終了に伴い解放済み

// 複数の &T トークン共存——Dup 型は自由に複製可能
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 トークンのスコープと伝播

トークンは通常の型であるため、通常型のすべての操作をサポートする：

**トークンを返す**——トークンは戻り値とともに伝播する：

```yaoxiang
// ✅ 子トークンと親トークンを一緒に返す
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // トークンが呼び出し元に返される
print(px_ref)                    // OK、トークンは依然としてスコープ内
```

**構造体に格納**——構造体はトークンフィールドを保持できる：

```yaoxiang
// ✅ 構造体がトークンをフィールドとして保持
Window: Type = {
    target: Point,
    view: &Point,              // トークンフィールド——target への読み取りビューを保持
}
```

**クロージャ捕捉**——クロージャは他の値を捕捉するようにトークンを捕捉する：

```yaoxiang
// ✅ クロージャが &Float トークンを捕捉（Dup 型のためクロージャへ自由に複製可能）
filter_by_threshold: (items: List(Point), threshold: &Float) -> List(Point) = {
    items.filter(|p| p.x > threshold)
}
```

### 12.4 自動借用選択

呼び出し側で、コンパイラは以下の優先度で自動選択する：

```
1. 実引数が後続も使用される → トークン生成を優先（&T または &mut T、メソッドシグネチャに応じて）
2. 実引数が後続未使用 → Move
3. 優先マッチング順序：&T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // print のパラメータ型は &Point -> コンパイラが &Point トークン生成
p.shift(1.0, 1.0)  // shift のパラメータ型は &mut Point -> コンパイラが &mut Point トークン生成
p2 = p             // 後続未使用 -> Move
```

### 12.5 トークン衝突検出

コンパイラはトークン値に対し**フロー敏感活性分析**を行い、各トークンの状態（アクティブ/移動済み）を追跡する：

```yaoxiang
// ❌ &mut と派生した &T は同時にアクティブ不可
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
    p.x = 10.0                   // ✅ WriteToken は依然として使用可能
}

// ❌ 同一実引数から &mut トークンと他トークンを同時生成不可
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p から &mut トークンと & トークンを同時派生
```

### 12.6 コンパイラ内部：ブランド機構

ユーザがブランドに触れることは決してない。コンパイラが内部で各トークンにコンパイル時一意識別子を割り当てる：

```
ユーザが見るもの         コンパイラ内部表現
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N はコンパイル時一意整数
&mut Point     →  WriteToken(Point, #M)   // #M はコンパイル時一意整数
```

ブランドの用途：
- **偽造防止**：トークンは所有者カプセルからのみ取得可能、凭空構築不可
- **関連追跡**：フィールドアクセスから派生した `&Float` は派生ブランド（`#N.field_x`）を携带し、コンパイラは親トークンまで追跡可能
- **衝突検出**：同源の WriteToken と派生 ReadToken は同時にアクティブ不可

ブランドは単態化とインライン化後に完全に消滅し、生成されたマシンコードには存在しない。**ゼロランタイムオーバーヘッド**。

### 12.7 トークン Sum 型

```
&BorrowToken ::= &T          // ReadToken（元データ凍結 -> Dup 安全）
               | &mut T      // WriteToken（排他的読み書き -> Linear）
```

### 12.8 借用トークン vs ref

| | `&T` / `&mut T` | `ref` |
|------|------|------|
| 役割 | 一目見る/その場で変更 | 共有保持 |
| 範囲 | トークン値のスコープに従う | スコープ跨越 |
| コスト | ゼロオーバーヘッド（ゼロサイズ型、コンパイル後消滅） | Rc または Arc（コンパイラが選択） |
| エスケープ | 可（トークンは戻り値/構造体/クロージャで伝播可能） | 元々エスケープ用 |
| タスク跨越 | 不可（トークンはタスク間転送未実装） | 可（コンパイラが自動的に Arc を選択） |
| 環検出 | 関与しない | タスク内は静かに許容、タスク跨越は lint |

---

## 付録：型定義早見表

### A.1 型定義

```
// === レコード型（波括弧） ===

// レコード型
Point: Type = { x: Float, y: Float }

// バリアント付きレコード型（関数をフィールドに使用）
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// === インタフェース型（波括弧、フィールドはすべて関数） ===

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

### A.3 型属性早見表

```
// === Move（デフォルト） ===
// すべての型はデフォルト Move。代入、引数渡し、戻り値 = 所有権移転

// === プリミティブ値型（コンパイラ内蔵） ===
Int, Float,     // 代入時に自動値コピー、二つの値は完全に独立
Bool, Char      // Dup ではなく、コンパイラのプリミティブ組込処理

// === Dup（シャローコピー：ハンドル複製、底层データ共有） ===
&T              // ゼロサイズ読み取りトークン、トークン複製 = 同一データへの複数視点
ref T           // Rc/Arc 複製 = 参照カウント+1、ヒープデータ共有

// === Linear ===
&mut T          // ゼロサイズ書き込みトークン、Linear（排他的、複製不可）

// === Clone（明示的深コピー） ===
value.clone()   // 独立副本作成、修正は原値に影響しない
```

### A.4 借用トークン早見表

```
// === 借用トークン ===
&T              // ゼロサイズコンパイル時読み取りトークン、元データ凍結 -> Dup（複製可能）
&mut T          // ゼロサイズコンパイル時書き込みトークン、排他的読み書き -> Linear（複製不可）

// 呼び出し側の自動選択
// 1. 実引数が後続も使用される -> トークン生成
// 2. 実引数が後続未使用 -> Move
// 3. 優先マッチング：&T < &mut T < Move

// トークン伝播
// ✅ 戻り値可、構造体格納可、クロージャ捕捉可
// ❌ タスク跨越不可（トークンはタスク間転送未実装）
```
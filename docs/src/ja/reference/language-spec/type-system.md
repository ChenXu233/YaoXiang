# 型システム仕様

このドキュメントは YaoXiang プログラミング言語の型システム仕様を定義する。基本型、複合型、ジェネリクス、trait を含む。

---

## 第零章：理論的基礎

### 0.1 Curry-Howard 対応

Curry-Howard 対応（Curry-Howard correspondence）は YaoXiang 型システムの理論的基礎である。これはプログラミング言語の型システムと数理論理学の間の深い対応関係を明らかにする：

| 論理学 | プログラミング言語 |
|--------|----------|
| 命題 \(P\) | 型 `Type` |
| 証明 \(p: P\) | プログラム `x: T = ...` |
| 含意 \(P \rightarrow Q\) | 関数型 `(P) -> Q` |
| 連言 \(P \wedge Q\) | 積型 `{ a: P, b: Q }` |
| 選言 \(P \vee Q\) | 和型 `{ a(P) \| b(Q) }` |
| 全称量化 \(\forall x:T. P(x)\) | ジェネリクス `(T: Type) -> ...` |
| 真 \(\top\) | `Void`（Unit、デフォルト値あり） |
| 偽 \(\bot\) | `Never`（零コンストラクタ、居留可能な値なし） |
| 型宇宙 \(Type_n : Type_{n+1}\) | 宇宙階層（Russell のパラドックス防止） |
| 場合分け | 型レベル `match` |

> **注意**：型レベル `match` は場合分け（case analysis）であり、数学的帰納法ではない。帰納法は型レベル再帰関数 + コンパイラの停止性検査を必要とする。

### 0.2 型は命題、プログラムは証明

YaoXiang では、この対応関係が設計の第一級の原則である：

- **停止する型レベル計算は正しい構成的証明に対応する**。YaoXiang の型族（`Nat` 上の `Add` のような case 分析 + 再帰呼び出し）は本質的に数学的帰納法の型レベルエンコーディングである——ただしコンパイラが停止性検査を行えることが前提となる。
- **型検査は証明の検証である**。あるプログラムが型検査を通過することは、論理的命題が構成的に証明されたことに相当する。

### 0.3 言語設計への影響

YaoXiang における Curry-Howard 対応の具体的な体現：

1. **宇宙階層**（RFC-010）：`Type₀ : Type₁ : Type₂ …` は `Type: Type` が引き起こす論理的パラドックス（Girard のパラドックス）を回避する
2. **型族**（RFC-011）：自然数 `Nat(Zero/Succ)` の型レベル case 分析 + 再帰呼び出しは Peano 公理に対応する——ただしコンパイラが停止性検査を行うことが前提
3. **条件型**（RFC-011）：`If: (C: Bool, T: Type, E: Type) -> Type` は論理学の case による選言に対応する
4. **値依存型**（RFC-011）：`Vec: (n: Int) -> Type` は「各整数 n に対して型が存在する」という有界量化に対応する

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

> **設計説明**：RFC-010 は「すべてが代入である」という統一モデル（`name: type = value`）を提案しているが、構文レベルでは型と値を区別する必要がある。コンパイラ実装では `Type` と `Expr` は独立した AST enum であり（`ast.rs:406` と `ast.rs:25`）、`TypeExpr` は BNF プレースホルダとして実装の `Type` enum に対応し、「この位置には型が期待される」を表す。

---

## 第二章：基本型

### 2.1 プリミティブ型
| 型 | 論理的対応 | 説明 | デフォルトサイズ |
|------|---------|------|----------|
| `Type` | — | メタ型 | 0 バイト |
| `Never` | ⊥（偽/空型） | 零コンストラクタ、値なし。発散/panic の戻り型。`Never <: T` は任意の T に対して成立。 | 0 バイト |
| `Void` | ⊤（真/Unit） | デフォルト void 値あり、零フィールド積型。`x: Void = <デフォルト>` が合法。 | 0 バイト |
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

**Never（⊥、偽/空型）** — 譲歩不可能な三性質：

1. **零コンストラクタ**：リテラルや式は一切 `Never` 型の値を生成できない。`x: Never = ...` の右辺は書けない。
2. **爆発原理**：`Never <: T` は任意の型 `T` に対して成立する。`assert(false)` は `Never` を返し、後続のコードは型検査を通過できる（実際には実行されないが）。
3. **発散マーカー**：`f: (...) -> Never` は `f` が決して返らないことを示す。コンパイラはこれに基づいて dead code 解析と `match` 分岐合流を行う。

`Never` は組み込み型名（`Int`/`Bool` と同じ登録パス）であり、キーワードではない。

**Void（⊤、真/Unit）** — 正確に一つの居留者（デフォルト void 値）。`Void` は零フィールド積型の単位元である。`x: Void = <デフォルト>` が合法で、関数がデフォルトで `return` を持たない場合 `Void` を返す。


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
- レコード型は中括弧 `{}` で定義される
- フィールド名の後にコロンと型を続ける
- インターフェース名は型本体内に記述することで、そのインターフェースの実装を表す

> **名前空間归属**：`Type.name` プレフィックス（例：`Point.draw`）は関数が `Point` の名前空間に属することを示す。
> これは暗黙のバインディングを引き起こさない。`p.draw()` のような `.` 呼び出し構文を有効にするには、明示的なバインディングが必要：
> `Point.draw = draw[0]`。
> 詳細は RFC-004 と RFC-010 を参照。

#### 3.1.1 フィールドのデフォルト値

型のフィールドにはデフォルト値を指定でき、構築時にはオプションで提供できる：

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
- `field: Type = expression` → デフォルト値あり、構築時はオプション
- `field: Type` → デフォルト値なし、構築時は必須

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

**構文**：インターフェースはフィールドがすべて関数型であるレコード型

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

**インターフェースの直接代入**：具象型はインターフェース型変数に直接代入できる（構造的サブタイピング）

```yaoxiang
// 直接代入（コンパイル時に具象型を決定可能 → ゼロオーバーヘッド呼び出し）
d: Drawable = Circle(1)
d.draw(screen)        // コンパイル後：circle_draw を直接呼び出し、vtable なし

// 関数の戻り値（コンパイル時に決定不能 → vtable 呼び出し）
d: Drawable = get_shape()
d.draw(screen)        // vtable でメソッドをルックアップ

// 関数の引数としてのインターフェース
process: (d: Drawable) -> Void = d.draw(screen)
```

**コンパイル時最適化戦略**：

| シナリオ | 推論結果 | 呼び出し方式 |
|------|----------|----------|
| 具象型の直接代入 | 具象型を決定可能 | 直接呼び出し（ゼロオーバーヘッド） |
| 関数の戻り値 | 未知 | vtable |
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

### 4.1 ジェネリクスパラメータの構文

ジェネリクスパラメータは関数型の一部であり、通常の引数と統一して `()` 構文を使用する：

```
GenericType     ::= Identifier '(' TypeArgList ')'
TypeArgList     ::= TypeExpr (',' TypeExpr)* ','?
TypeBound       ::= Identifier
                 |  Identifier '+' Identifier ('+' Identifier)*
```

ジェネリック型定義では、`(T: Type)` は型コンストラクタのパラメータシグネチャであり、`-> Type` は戻り型を表す：

```yaoxiang
List: (T: Type) -> Type = { ... }
Map: (K: Type, V: Type) -> Type = { ... }
```

ジェネリック関数では、型パラメータも同様にシグネチャで宣言され、コンパイラが実引数から自動的に推論する：

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
    push: (self: List(T), item: T) -> Void,   // self は単なる慣習名でキーワードではない
    get: (self: List(T), index: Int) -> Option(T)
}
```

### 4.3 型推論

```yaoxiang
// コンパイラが自動的にジェネリクスパラメータを推論
numbers: List(Int) = List(1, 2, 3)  // コンパイラが List(Int) を推論
```

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

### 7.1 コンパイル時定数パラメータ

```
LiteralType   ::= Identifier ':' Int          // コンパイル時定数
CompileTimeFn ::= '(' Identifier ':' Int ')' '(' Identifier ')' '->' TypeExpr
```

**核心設計**：`(n: Int)` ジェネリクスパラメータ + `(n: n)` 値パラメータで、コンパイル時定数とランタイム値を区別する。

```yaoxiang
// コンパイル時階乗：パラメータはコンパイル時に既知のリテラルでなければならない
factorial: (n: Int)(n: n) -> Int = {
    match n {
        0 => 1,
        _ => n * factorial(n - 1)
    }
}

// コンパイル時定数配列
StaticArray: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // コンパイル時にサイズが既知の配列
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
// IsTrue ブリッジと Assert 精製型（詳細は §8.3）
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


### 8.3 Assert 精製型と assert アサーション

`assert` と `Assert` は同じ精製プリミティブの二つの面であり、dispatch ディスパッチパイプラインが「述語の自由変数がコンパイル時にアクセス可能か」によって自動的に選択する。

**核心シグネチャ**：`assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**dispatch ディスパッチルール**：

| 判定基準 | モード | 振る舞い |
|------|------|------|
| すべての自由変数がコンパイル時に既知（ジェネリクスパラメータ、コンパイル時定数） | CompileTime | 証明パイプラインへ：true → Void として消去、false → コンパイルエラー（Never は居留不可） |
| ランタイム自由変数が存在（関数の引数、外部入力） | Runtime | ランタイム Bool 検査を挿入し、フロー敏感仮定集合 Γ に精製事実を注入 |

**フロー敏感仮定集合 Γ**：

コンパイラは各制御フロー点で既知の命題集合を管理する：

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP 伝播
mut x = x - 5       // Γ = {}  ← mut kill set：古い仮定が無効化される
```

`mut` 変数への代入後、その変数に関するすべての仮定が削除される（kill set）。分岐合流時、Γ は各分岐の交差を取る。

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

**構文**：型の積 `A & B` は A と B を同時に満たす型を表す

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

## 第十章：関数のオーバーロードと特化

### 10.1 関数のオーバーロード

```yaoxiang
// 基本的な特化：関数のオーバーロードを使用（コンパイラが自動選択）
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
// プラットフォーム型 enum（標準ライブラリで定義）
Platform: Type = X86_64 | AArch64 | RISC_V | ARM | X86

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

YaoXiang には区別すべき型属性が一つだけある：線形 vs 複製可能。コンパイラが自動的に推論する。

### 11.1 Move（デフォルトの所有権移転）

すべての型はデフォルトで Move セマンティクスに従う。代入、引数渡し、戻り値 = 所有権移転。

```yaoxiang
p: Point = Point(1.0, 2.0)
q = p           // Move、p はもう読み取れない
```

### 11.2 Dup（浅いコピー：ハンドルをコピー、データを共有）

**Dup 属性は参照/トークン型に使用される**。Dup 型の代入 = 浅いコピー——ハンドル/トークンをコピーし、底层データは共有される。複数の保持者が同じデータブロックを指す。

| 型 | 属性 | 説明 |
|------|------|------|
| `&T` | Dup | 零サイズ読み取りトークン、トークンコピー = 同じデータを指す複数のビュー |
| `ref T` | Dup | Rc/Arc コピー = 参照カウント+1、ヒープデータを共有 |
| `&mut T` | Linear | 零サイズ書き込みトークン、排他的、複製不可 |
| その他すべての型 | Move | デフォルトの所有権移転 |

**プリミティブ値型**（Int, Float, Bool, Char）はコンパイラ組み込みの特殊処理：代入時に自動的に値コピーされ、二つの値は完全に独立。これはコンパイラのネイティブ動作であり、Dup 型属性には属さない。

```yaoxiang
// &T: Dup、自由にエイリアス可能
view: &Point = &p
view2 = view     // Dup：トークンをコピー、両方とも有効
print(view.x)    // 使用可能
print(view2.x)   // 使用可能

// &mut T: Linear、複製不可
mut_ref: &mut Point = &mut p
// r2 = mut_ref  // ❌ &mut T は Dup ではない、コピー不可
```

### 11.3 Clone（明示的な深いコピー）と Dup の関係

**Clone** は明示的な深いコピーインターフェース。すべての型が Clone を実装でき、`.clone()` メソッドを提供する。

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

| | Dup | Clone |
|---|---|---|
| **セマンティクス** | 浅いコピー：ハンドル/トークンをコピー、底层データを共有 | 深いコピー：完全に独立した複製を作成 |
| **呼び出し方式** | 暗黙的（代入/引数渡しで自動） | 明示的（`.clone()`） |
| **変更の影響** | 互いに影響する（底层データを共有） | 互いに影響しない（独立した複製） |
| **適用型** | `&T` トークン、`ref T` | Clone インターフェースを実装する任意の型 |
| **コスト** | ゼロオーバーヘッド（トークンは零サイズ型） | 型による |

**Dup は Clone を意味せず、Clone は Dup を意味しない**——これらは二つの直交する概念である：

```yaoxiang
// Dup 型：トークンをコピー、底层データを共有
view: &Point = &p
view2 = view        // Dup：トークンをコピー、両方とも同じ p を指す
print(view.x)       // 使用可能
print(view2.x)      // 使用可能、同じデータを参照

// プリミティブ値型：コンパイラが自動的に値コピー（Dup ではない）
x: Int = 42
y = x               // 値コピー、x と y は完全に独立
print(x)            // 使用可能

// Clone：明示的な深いコピー、独立した複製を作成
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone：深いコピー、p はまだ使用可能
r = p               // Move：所有権移転、Point は Dup でもプリミティブ値型でもない
```

**設計意図**：
- Dup はトークン/参照型に使用され、「同じデータの複数のビュー」問題を解決する
- Clone は独立した複製が必要なシナリオに使用され、明示的な呼び出しでコストを可視化する
- プリミティブ値型（Int/Float/Bool/Char）のコピーはコンパイラの組み込み動作であり、Dup には属さない
- ほとんどのカスタム型はデフォルトで Move であり、ゼロコピーで高性能

## 第十二章：借用トークン型

### 12.1 核心概念

`&T` と `&mut T` は**零サイズのコンパイル時トークン型**である。これらは「参照」ではなく、「アクセス権限の型レベル証明」である。

```
&T      →  零サイズ、元データを凍結（この期間中 WriteToken の取得を禁止）、
          凍結保証の下で複数の読み取りが安全 → Dup（複製可能）
&mut T  →  零サイズ、排他的読み書き（他のすべてのトークンを禁止）、
          排他的アクセス下ではコピーは無意味 → Linear（非 Dup）
```

**重要な特性**：
- トークンは**通常の型**であり、他のすべての型と同じスコープルールに従う
- ライフタイム注釈 `'a` は不要
- 専用の借用チェッカーは不要——型属性（Dup/Linear）が自然に権限を推論する
- コンパイル後に完全に消滅し、ランタイムオーバーヘッドはゼロ

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
p.print()                       // OK、前のトークンは shift 呼び出し終了と共に解放済み

// 複数の &T トークンが共存——Dup 型は自由にコピーを許可
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 トークンのスコープと伝播

トークンは通常の型であるため、通常の型のすべての操作をサポートする：

**トークンの返却**——トークンは戻り値と共に伝播：

```yaoxiang
// ✅ サブトークンと親トークンを一緒に返却
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // トークンは呼び出し元に返される
print(px_ref)                    // OK、トークンはまだスコープ内
```

**構造体への格納**——構造体はトークンフィールドを保持できる：

```yaoxiang
// ✅ 構造体がトークンをフィールドとして保持
Window: Type = {
    target: Point,
    view: &Point,              // トークンフィールド——target への読み取りビューを保持
}
```

**クロージャのキャプチャ**——クロージャは他の値をキャプチャするのと同様にトークンをキャプチャ：

```yaoxiang
// ✅ クロージャが &Float トークンをキャプチャ（Dup 型、自由にクロージャへコピー）
filter_by_threshold: (items: List(Point), threshold: &Float) -> List(Point) = {
    items.filter(|p| p.x > threshold)
}
```

### 12.4 自動借用選択

呼び出し側では、コンパイラが以下の優先順位で自動的に選択する：

```
1. 実引数が後続も使用される → トークン作成を優先（&T または &mut T、メソッドシグネチャによる）
2. 実引数が後続使用されない → Move
3. 優先マッチ順序：&T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // print の引数型は &Point → コンパイラが &Point トークンを作成
p.shift(1.0, 1.0)  // shift の引数型は &mut Point → コンパイラが &mut Point トークンを作成
p2 = p             // 後続使用されない → Move
```

### 12.5 トークン衝突検出

コンパイラはトークン値に対して**フロー敏感活性解析**を行い、各トークンの状態（アクティブ/移動済み）を追跡する：

```yaoxiang
// ❌ &mut と派生 &T を同時にアクティブにできない
bad_alias: (p: &mut Point) -> Void = {
    p.x = 10.0                   // ✅ WriteToken を正常使用
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

// ❌ 同じ実引数から同時に &mut トークンと他のトークンを作成できない
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p から同時に &mut と & トークンを派生
```

### 12.6 コンパイラ内部：ブランド機構

ユーザーはブランドに一切触れない。コンパイラは内部で各トークンにコンパイル時一意識別子を割り当てる：

```
ユーザーに見える         コンパイラ内部表現
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N はコンパイル時一意整数
&mut Point     →  WriteToken(Point, #M)   // #M はコンパイル時一意整数
```

ブランドの用途：
- **偽造防止**：トークンは所有者のカプセルからのみ取得でき、凭空には構築できない
- **関連追跡**：フィールドアクセスから派生する `&Float` は派生ブランド（`#N.field_x`）を持ち、コンパイラは親トークンまで追跡可能
- **衝突検出**：同源 WriteToken と派生 ReadToken は同時にアクティブになれない

ブランドは単態化とインライン化後に完全に消滅し、生成される機械語には存在しない。**ランタイムオーバーヘッドはゼロ。**

### 12.7 トークン Sum 型

```
&BorrowToken ::= &T          // ReadToken（元データを凍結 → Dup 安全）
               | &mut T      // WriteToken（排他的読み書き → Linear）
```

### 12.8 借用トークン vs ref

| | `&T` / `&mut T` | `ref` |
|------|------|------|
| 機能 | ちら見/その場変更 | 共有保持 |
| 範囲 | トークン値のスコープに従う | スコープをまたぐ |
| コスト | ゼロオーバーヘッド（零サイズ型、コンパイル後消滅） | Rc または Arc（コンパイラが選択） |
| エスケープ | 可（トークンは戻り値/構造体/クロージャで伝播） | 本来エスケープ用 |
| タスク間 | 不可（トークンはタスク間渡し未実装） | 可（コンパイラが自動的に Arc を選択） |
| 環検出 | 関与しない | タスク内はサイレント、タスク間は lint |

---

## 付録：型定義チートシート

### A.1 型定義

```
// === レコード型（中括弧） ===

// レコード型
Point: Type = { x: Float, y: Float }

// 変種付きレコード型（関数フィールドを使用）
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// === インターフェース型（中括弧、フィールドはすべて関数） ===

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

// コンパイル時ジェネリクス
factorial: (n: Int)(n: n) -> Int = { ... }
StaticArray: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }

// 条件型
If: (C: Bool, T: Type, E: Type) -> Type = match C { True => T, False => E }

// 関数特化
sum: (arr: Array(Int)) -> Int = { ... }
sum: (arr: Array(Float)) -> Float = { ... }
```

### A.3 型属性チートシート

```
// === Move（デフォルト） ===
// すべての型はデフォルトで Move。代入、引数渡し、戻り値 = 所有権移転

// === プリミティブ値型（コンパイラ組み込み） ===
Int, Float,     // 代入時に自動的に値コピー、二つの値は完全に独立
Bool, Char      // Dup ではなく、コンパイラのプリミティブ組み込み処理

// === Dup（浅いコピー：ハンドルをコピー、底层データを共有） ===
&T              // 零サイズ読み取りトークン、トークンコピー = 同じデータを指す複数のビュー
ref T           // Rc/Arc コピー = 参照カウント+1、ヒープデータを共有

// === Linear ===
&mut T          // 零サイズ書き込みトークン、Linear（排他的、複製不可）

// === Clone（明示的な深いコピー） ===
value.clone()   // 独立した複製を作成、変更は元値に影響しない
```

### A.4 借用トークンチートシート

```
// === 借用トークン ===
&T              // 零サイズコンパイル時読み取りトークン、元データを凍結 → Dup（複製可能）
&mut T          // 零サイズコンパイル時書き込みトークン、排他的読み書き → Linear（複製不可）

// 呼び出し側の自動選択
// 1. 実引数が後続も使用される → トークン作成
// 2. 実引数が後続使用されない → Move
// 3. 優先マッチ：&T < &mut T < Move

// トークン伝播
// ✅ 返却可、構造体への格納可、クロージャでキャプチャ可
// ❌ タスク間渡し不可（トークンはタスク間渡し未実装）
```
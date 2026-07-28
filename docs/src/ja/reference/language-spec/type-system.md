# 型システム仕様

この文書は YaoXiang プログラミング言語の型システム仕様を定義する。基本型、複合型、ジェネリクス、trait を含む。

---

## 第零章：理論的基礎

### 0.1 Curry-Howard 対応

Curry-Howard 対応（Curry-Howard
correspondence）は YaoXiang 型システムの理論的基礎である。これはプログラミング言語の型システムと数理論理学の間の深い対応関係を明らかにする：

| 論理学                           | プログラミング言語                              |
| -------------------------------- | ----------------------------------------------- |
| 命題 \(P\)                       | 型 `Type`                                       |
| 証明 \(p: P\)                    | プログラム `x: T = ...`                         |
| 含意 \(P \rightarrow Q\)         | 関数型 `(P) -> Q`                               |
| 連言 \(P \wedge Q\)              | 直積型 `{ a: P, b: Q }`                         |
| 選言 \(P \vee Q\)                | 直和型 `{ a(P) \| b(Q) }`                       |
| 全称量化 \(\forall x:T. P(x)\)   | ジェネリック `(T: Type) -> ...`                 |
| 真 \(\top\)                      | `Void`（Unit、デフォルト値を持つ）              |
| 偽 \(\bot\)                      | `Never`（零コンストラクタ、居住可能な値がない） |
| 型の宇宙 \(Type_n : Type_{n+1}\) | 宇宙階層化（Russell のパラドックスを防ぐ）      |
| ケース分析                       | 型レベル `match`                                |

> **注意**：型レベル `match` は場合分け（case
> analysis）であり、数学的帰納法ではない。帰納法には型レベル再帰関数とコンパイラの停止性検査が必要である。

### 0.2 型は命題、プログラムは証明

YaoXiang において、この対応関係は設計の第一級の原則である：

- **停止する型レベル計算は正しい構成的証明に対応する**。YaoXiang の型族（`Nat` 上の `Add`
  のケース分析 + 再帰呼び出しなど）は本質的に数学的帰納法の型レベルエンコードである——前提として、コンパイラが停止性検査を行うこと。
- **型検査は証明の検証である**。プログラムが型検査を通過することは、論理命題が構成的に証明されたことに相当する。

### 0.3 言語設計への影響

Curry-Howard 対応の YaoXiang における具体的な現れ：

1. **宇宙階層化**（RFC-010）：`Type₀ : Type₁ : Type₂ …` により `Type: Type`
   がもたらす論理的な矛盾（Girard のパラドックス）を回避
2. **型族**（RFC-011）：自然数 `Nat(Zero/Succ)`
   の型レベルケース分析 + 再帰呼び出しはペアノの公理に対応する——前提として、コンパイラが停止性検査を行うこと
3. **条件型**（RFC-011）：`If: (C: Bool, T: Type, E: Type) -> Type`
   は論理における case の選言に対応する
4. **値依存型**（RFC-011）：`Vec: (n: Int) -> Type`
   は「各整数 n に対して1つの型が存在する」という有界量化に対応する

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

> **設計説明**：RFC-010 は「すべてが代入である」という統一モデル（`name: type = value`）を提案しているが、構文レベルでは型と値を区別する必要がある。コンパイラの実装では
> `Type` と `Expr` は2つの独立した AST 列挙型（`ast.rs:406` と `ast.rs:25`）であり、`TypeExpr`
> は BNF のプレースホルダとして実装の `Type` 列挙型に対応し、「この位置には型が期待される」を表す。

---

## 第二章：基本型

### 2.1 プリミティブ型

| 型       | 論理的対応   | 説明                                                                                        | デフォルトサイズ |
| -------- | ------------ | ------------------------------------------------------------------------------------------- | ---------------- |
| `Type`   | —            | メタ型                                                                                      | 0 バイト         |
| `Never`  | ⊥（偽/空型） | 零コンストラクタ、値を持たない。発散/panic の戻り型。`Never <: T` は任意の T に対して成立。 | 0 バイト         |
| `Void`   | ⊤（真/Unit） | デフォルトの void 値を持つ、零フィールド直積型。`x: Void = <デフォルト>` は有効。           | 0 バイト         |
| `Bool`   | —            | ブール値：`true` / `false`                                                                  | 1 バイト         |
| `Int`    | —            | 符号付き整数                                                                                | 8 バイト         |
| `Uint`   | —            | 符号なし整数                                                                                | 8 バイト         |
| `Float`  | —            | 浮動小数点数                                                                                | 8 バイト         |
| `String` | —            | UTF-8 文字列                                                                                | 可変             |
| `Char`   | —            | Unicode 文字                                                                                | 4 バイト         |
| `Bytes`  | —            | 生のバイト列                                                                                | 可変             |

ビット幅付き整数：`Int8`, `Int16`, `Int32`, `Int64`, `Int128` ビット幅付き浮動小数点：`Float32`,
`Float64`

### 2.2 Never と Void：⊥ と ⊤

`Never` と `Void` は型システムの論理的な基本要素であり、それぞれ偽（⊥）と真（⊤）に対応する。

**Never（⊥、偽/空型）** — 譲歩できない3つの性質：

1. **零コンストラクタ**：リテラルや式で `Never` 型の値を生成することはできない。`x: Never = ...`
   には右辺が書けない。
2. **爆発原理**：`Never <: T` は任意の型 `T` に対して成立する。`assert(false)` は `Never`
   を返し、その後のコードは型検査を通過できる（実際には決して実行されないが）。
3. **発散マーカー**：`f: (...) -> Never` は `f`
   が決して戻らないことを示す。コンパイラはこれに基づいてデッドコード解析と `match`
   分岐の合流を行う。

`Never` は組み込み型名（`Int`/`Bool` と同じ登録パス）であり、キーワードではない。

**Void（⊤、真/Unit）** — ちょうど1つの居住者（デフォルトの void 値）を持つ。`Void`
は零フィールド直積型の単位元である。`x: Void = <デフォルト>` は有効であり、関数がデフォルトで
`return` を持たない場合は `Void` を返す。

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
// 简单记录类型
Point: Type = { x: Float, y: Float }

// 空记录类型
Empty: Type = {}

// 带泛型的记录类型
Pair: (T: Type) -> Type = { first: T, second: T }

// 实现接口的记录类型
Point: Type = {
    x: Float,
    y: Float,
    Drawable,
    Serializable
}
```

**規則**：

- レコード型は中括弧 `{}` で定義される
- フィールド名の直後にコロンと型を続ける
- 型本体内にインターフェース名を書くとそのインターフェースを実装することを示す

> **名前空間の所属**：`Type.name` 接頭辞（例：`Point.draw`）は関数が `Point`
> の名前空間に属することを示す。これは暗黙的なバインディングを発生させない。`p.draw()` のような `.`
> 呼び出し構文を有効にするには、明示的なバインディングが必要：
> `Point.draw = draw[0]`。詳細は RFC-004 と RFC-010 を参照。

#### 3.1.1 フィールドのデフォルト値

型フィールドはデフォルト値を指定でき、構築時には任意で提供できる：

```yaoxiang
// 有默认值的字段 - 构造时可选
Point: Type = {
    x: Float = 0,
    y: Float = 0
}

// 使用
Point()           // -> Point(x=0, y=0)
Point(x=1)       // -> Point(x=1, y=0)
Point(x=1, y=2) // -> Point(x=1, y=2)

// 无默认值的字段 - 构造时必填
Point2: Type = {
    x: Float,
    y: Float
}

// 使用
Point2(x=1, y=2) // 正确
Point2()          // 错误
```

**規則**：

- `field: Type = expression` -> デフォルト値を持つ、構築時は任意
- `field: Type` -> デフォルト値なし、構築時は必須

#### 3.1.2 組み込みバインディング

型定義の本体内でメソッドを直接バインドできる：

```yaoxiang
// 方式1：引用外部函数绑定
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]    // 绑定到位置0
}
// 调用：p1.distance(p2) -> distance(p1, p2)

// 方式2：匿名函数 + 位置绑定
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance: ((a: Point, b: Point) -> Float)[0] = ((a, b) => {
        dx = a.x - b.x
        dy = a.y - b.y
        return (dx * dx + dy * dy).sqrt()
    })
}
// 语法：((params) => body)[position]
// 调用：p1.distance(p2) -> distance(p1, p2)
```

### 3.2 インターフェース型

```
InterfaceType ::= '{' FnField (',' FnField)* ','?
FnField       ::= Identifier ':' FnType
FnType        ::= '(' ParamTypes? ')' '->' TypeExpr
```

**構文**：インターフェースは全フィールドが関数型であるレコード型である

```yaoxiang
// 接口定义
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

// 空接口
EmptyInterface: Type = {}
```

**インターフェースの実装**：型は定義の最後にインターフェース名を列挙することでインターフェースを実装する

```yaoxiang
// 实现接口的类型
Point: Type = {
    x: Float,
    y: Float,
    Drawable,        // 实现 Drawable 接口
    Serializable     // 实现 Serializable 接口
}
```

**インターフェースへの直接代入**：具象型はインターフェース型変数に直接代入できる（構造的サブタイピング）

```yaoxiang
// 直接赋值（编译期可确定具体类型 -> 零开销调用）
d: Drawable = Circle(1)
d.draw(screen)        // 编译后：直接调用 circle_draw，无 vtable

// 函数返回值（编译期无法确定 -> vtable 调用）
d: Drawable = get_shape()
d.draw(screen)        // 通过 vtable 查找方法

// 接口作为函数参数
process: (d: Drawable) -> Void = d.draw(screen)
```

**コンパイル時最適化戦略**：

| シナリオ         | 推論結果         | 呼び出し方式                       |
| ---------------- | ---------------- | ---------------------------------- |
| 具象型を直接代入 | 具象型が確定可能 | 直接呼び出し（ゼロオーバーヘッド） |
| 関数の戻り値     | 不明             | vtable                             |
| 異種コレクション | 複数の型         | vtable                             |

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

ジェネリック引数は関数型の一部であり、通常の引数と統一的に `()` 構文を使用する：

```
GenericType     ::= Identifier '(' TypeArgList ')'
TypeArgList     ::= TypeExpr (',' TypeExpr)* ','?
TypeBound       ::= Identifier
                 |  Identifier '+' Identifier ('+' Identifier)*
```

ジェネリック型定義では、`(T: Type)` は型コンストラクタの引数シグネチャであり、`-> Type`
は戻り型を表す：

```yaoxiang
List: (T: Type) -> Type = { ... }
Map: (K: Type, V: Type) -> Type = { ... }
```

ジェネリック関数では、型引数も同様にシグネチャで宣言され、コンパイラが実引数から自動的に推論する：

```yaoxiang
map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R)) = ...
```

### 4.2 ジェネリック型定義

```yaoxiang
// 基础泛型类型
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
    push: (self: List(T), item: T) -> Void,   // self 只是约定名，不是关键字
    get: (self: List(T), index: Int) -> Option(T)
}
```

### 4.3 型推論

```yaoxiang
// 编译器自动推导泛型参数
numbers: List(Int) = List(1, 2, 3)  // 编译器推导 List(Int)
```

---

## 第五章：型制約

### 5.1 単一制約

```
ConstrainedType ::= '(' Identifier ':' TypeBound ')' TypeExpr
```

```yaoxiang
// 接口类型定义（作为约束）
Clone: Type = {
    clone: () -> Clone
}

// 使用约束
clone: (T: Clone)(value: T) -> T = value.clone()
```

### 5.2 複数制約

```yaoxiang
// 多重约束语法
combine: (T: Clone + Add)(a: T, b: T) -> T = {
    a.clone() + b
}

// 泛型容器的排序
sort: (T: Clone + PartialOrd)(list: List(T)) -> List(T) = {
    result = list.clone()
    quicksort(&mut result)
    return result
}
```

### 5.3 関数型制約

```yaoxiang
// 高阶函数约束
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
// Iterator trait（使用记录类型语法）
Iterator: (T: Type) -> Type = {
    Item: T,                    // 关联类型
    next: () -> Option(T),
    has_next: () -> Bool
}

// 使用关联类型
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
// 更复杂的关联类型
Container: (T: Type) -> Type = {
    Item: T,
    IteratorType: Iterator(T),  // 关联类型也是泛型的
    iter: () -> IteratorType
}
```

---

## 第七章：コンパイル時ジェネリクス

### 7.1 コンパイル時定数引数

```
LiteralType   ::= Identifier ':' Int          // 编译期常量
CompileTimeFn ::= '(' Identifier ':' Int ')' '(' Identifier ')' '->' TypeExpr
```

**核心設計**：`(n: Int)` ジェネリック引数と `(n: n)`
値引数を用いて、コンパイル時定数とランタイム値を区別する。

```yaoxiang
// 编译期阶乘：参数必须是编译期已知的字面量
factorial: (n: Int)(n: n) -> Int = {
    match n {
        0 => 1,
        _ => n * factorial(n - 1)
    }
}

// 编译期常量数组
StaticArray: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // 编译期已知大小的数组
    length: N
}

// 使用方式
arr: StaticArray(Int, factorial(5))  // 编译器在编译期计算 factorial(5) = 120
```

### 7.2 コンパイル時定数配列

```yaoxiang
// 矩阵类型使用
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows)
}

// 编译期维度验证
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
// 类型级 If
If: (C: Bool, T: Type, E: Type) -> Type = match C {
    True => T,
    False => E
}

// 示例：编译期分支
NonEmpty: (T: Type) -> Type = If(T != Void, T, Never)
// IsTrue 桥接与 Assert 精化类型（详见 §8.3）
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      // ⊤，程序继续
    false => Never,    // ⊥，发散/编译错误
}
Assert: (cond: Bool) -> Type = IsTrue(cond)
```

### 8.2 型族

```yaoxiang
// 编译期类型转换
AsString: (T: Type) -> Type = match T {
    Int => String,
    Float => String,
    Bool => String,
    _ => String
}
```

### 8.3 Assert 精錬型と assert 表明

`assert` と `Assert`
は同一の精錬プリミティブの二面であり、「述語の自由変数がコンパイル時に到達可能か」に基づいて dispatch 分配パイプラインにより自動的に選択される。

**核心シグネチャ**：`assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))`

**dispatch 分配規則**：

| 判定基準                                                                   | モード      | 動作                                                                                       |
| -------------------------------------------------------------------------- | ----------- | ------------------------------------------------------------------------------------------ |
| すべての自由変数がコンパイル時に既知（ジェネリック引数、コンパイル時定数） | CompileTime | 証明パイプラインへ進む：true → Void に消去、false → コンパイルエラー（Never は居住不可能） |
| ランタイム自由変数が存在（関数引数、外部入力）                             | Runtime     | ランタイム Bool 検査を挿入し、フロー敏感仮定集合 Γ に精錬事実を注入                        |

**フロー敏感仮定集合 Γ**：

コンパイラは各制御流点における既知の命題集合を維持する：

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP 伝播
mut x = x - 5       // Γ = {}  ← mut kill set：旧仮定失效
```

`mut` 変数への代入後、その変数に関連するすべての仮定が削除される（kill
set）。分岐合流時、Γ は各分岐の交差集合を取る。

---

## 第九章：型共用と交差

### 9.1 型共用

```
TypeUnion     ::= TypeExpr '|' TypeExpr
```

### 9.2 型交差

```
TypeIntersection ::= TypeExpr '&' TypeExpr
```

**構文**：型交差 `A & B` は A と B の両方を満たす型を表す

```yaoxiang
// 接口组合 = 类型交集
DrawableSerializable: Type = Drawable & Serializable

// 使用交集类型
process: (T: Drawable & Serializable)(item: T, screen: Surface) -> String = {
    item.draw(screen)
    return item.serialize()
}
```

---

## 第十章：関数オーバーロードと特化

### 10.1 関数オーバーロード

```yaoxiang
// 基本特化：使用函数重载（编译器自动选择）
sum: (arr: Array(Int)) -> Int = {
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Array(Float)) -> Float = {
    return simd_sum_float(arr.data, arr.length)
}

// 通用实现
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
// 平台类型枚举（标准库定义）
Platform: Type = { X86_64: () -> Platform, AArch64: () -> Platform, RISC_V: () -> Platform, ARM: () -> Platform, X86: () -> Platform }

// P 是预定义泛型参数名，代表当前编译平台
sum: (P: X86_64)(arr: Array(Float)) -> Float = {
    return avx2_sum(arr.data, arr.length)
}

sum: (P: AArch64)(arr: Array(Float)) -> Float = {
    return neon_sum(arr.data, arr.length)
}
```

---

## 第十一章：型属性

YaoXiang で区別が必要な型属性は1種類のみ：リニア vs コピー可能。コンパイラにより自動的に推論される。

### 11.1 Move（デフォルトの所有権移転）

すべての型はデフォルトで Move セマンティクスに従う。代入、関数引数渡し、戻り値 = 所有権移転。

```yaoxiang
p: Point = Point(1.0, 2.0)
q = p           // Move，p 不可再读
```

### 11.2 Dup（浅いコピー：ハンドルをコピーし、データを共有）

**Dup 属性は参照/トークン型に使用される**。Dup 型の代入 = 浅いコピー（ハンドル/トークンをコピーし、基礎となるデータを共有する）。複数の所有者が同じデータブロックを指す。

| 型               | 属性   | 説明                                                                      |
| ---------------- | ------ | ------------------------------------------------------------------------- |
| `&T`             | Dup    | 零サイズ読み取りトークン、トークンをコピー = 複数の視点が同じデータを指す |
| `ref T`          | Dup    | Rc/Arc コピー = 参照カウント+1、ヒープデータを共有                        |
| `&mut T`         | Linear | 零サイズ書き込みトークン、排他的、コピー不可                              |
| その他すべての型 | Move   | デフォルトの所有権移転                                                    |

**プリミティブ値型**（Int、Float、Bool、Char）はコンパイラに組み込まれた特別な処理である：代入時に自動的に値がコピーされ、2つの値は完全に独立している。これはコンパイラのネイティブ動作であり、Dup 型属性には属さない。

```yaoxiang
// &T: Dup，可自由别名
view: &Point = &p
view2 = view     // Dup：复制令牌，两者均有效
print(view.x)    // 可用
print(view2.x)   // 可用

// &mut T: Linear，不可复制
mut_ref: &mut Point = &mut p
// r2 = mut_ref  // ❌ &mut T 不是 Dup，不能复制
```

### 11.3 Clone（明示的な深いコピー）と Dup の関係

**Clone** は明示的な深いコピーインターフェースである。すべての型は Clone を実装でき、`.clone()`
メソッドを提供する。

```yaoxiang
// Clone 接口定义（标准库）
Clone: Type = {
    clone: () -> Clone
}

// 使用
p: Point = Point(1.0, 2.0)
backup = p.clone()    // 深复制，p 仍然可用
p2 = p.clone()        // 可多次克隆
```

**Dup と Clone の違い**：

|                    | Dup                                                                 | Clone                                    |
| ------------------ | ------------------------------------------------------------------- | ---------------------------------------- |
| **セマンティクス** | 浅いコピー：ハンドル/トークンをコピーし、基礎となるデータを共有する | 深いコピー：完全で独立した副本を作成する |
| **呼び出し方法**   | 暗黙的（代入/関数引数渡しで自動）                                   | 明示的（`.clone()`）                     |
| **変更の影響**     | 互いに影響する（基礎となるデータを共有）                            | 互いに影響しない（独立した副本）         |
| **適用型**         | `&T` トークン、`ref T`                                              | Clone インターフェースを実装する任意の型 |
| **コスト**         | ゼロオーバーヘッド（トークンは零サイズ型）                          | 型に依存                                 |

**Dup は Clone を包含せず、Clone は Dup を包含しない** — これらは2つの直交する概念である：

```yaoxiang
// Dup 类型：复制令牌，底层数据共享
view: &Point = &p
view2 = view        // Dup：复制令牌，两者指向同一个 p
print(view.x)       // 可用
print(view2.x)      // 可用，看到的是同一份数据

// 原语值类型：编译器自动值复制（不是 Dup）
x: Int = 42
y = x               // 值复制，x 和 y 完全独立
print(x)            // 可用

// Clone：显式深拷贝，创建独立副本
p: Point = Point(1.0, 2.0)
q = p.clone()       // Clone：深复制，p 仍然可用
r = p               // Move：所有权转移，因为 Point 不是 Dup 也不是原语值类型
```

**設計意図**：

- Dup はトークン/参照型に使用され、「複数の視点で同じデータを見る」問題を解決する
- Clone は独立した副本が必要なシナリオに使用され、明示的な呼び出しによりコストを可視化する
- プリミティブ値型（Int/Float/Bool/Char）のコピーはコンパイラの組み込み動作であり、Dup には属さない
- ほとんどのカスタム型はデフォルトで Move であり、ゼロコピーで高性能

## 第十二章：借用トークン型

### 12.1 核心概念

`&T` と `&mut T`
は**零サイズのコンパイル時トークン型**である。これらは「参照」ではなく、「アクセス権限の型レベル証明」である。

```
&T      →  零サイズ、ソースデータを凍結（期間中 WriteToken の取得を禁止）、
          凍結保証下では複数の読み取り専用が安全 → Dup（コピー可能）
&mut T  →  零サイズ、排他的読み書き（他のすべてのトークンを禁止）、
          排他的アクセス下ではコピーに意味がない → Linear（Dup ではない）
```

**重要な特性**：

- トークンは**通常の型**であり、他のすべての型と同じスコープルールに従う
- ライフタイム注釈 `'a` は不要
- 専用の借用チェッカーは不要 — 型属性（Dup/Linear）が権限を自然に推論する
- コンパイル後に完全に消滅し、ランタイムオーバーヘッドがゼロ

### 12.2 基本的な使用

```yaoxiang
// 方法端：声明参数类型，决定需要的权限
Point.print: (self: &Point) -> Void = {
    print(self.x)               // &Point 令牌授予读权限
    print(self.y)
}

Point.shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
    self.x = self.x + dx        // &mut Point 令牌授予写权限
    self.y = self.y + dy
}

// 调用端：编译器自动选择借用或 Move
p = Point(1.0, 2.0)
p.print()                       // 编译器自动创建 &Point 令牌
p.shift(1.0, 1.0)               // 编译器自动创建 &mut Point 令牌
p.print()                       // OK，上一个令牌已随 shift 调用结束而释放

// 多个 &T 令牌共存——Dup 类型允许自由复制
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)
}
d = distance(p, p2)
```

### 12.3 トークンのスコープと伝播

トークンは通常の型であるため、通常の型のすべての操作をサポートする：

**トークンの返却** — トークンは戻り値と一緒に伝播する：

```yaoxiang
// ✅ 子令牌和父令牌一起返回
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)
}

p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()        // 令牌返回给调用者
print(px_ref)                    // OK，令牌仍在作用域
```

**構造体への格納** — 構造体はトークンフィールドを保持できる：

```yaoxiang
// ✅ 结构体携带令牌作为字段
Window: Type = {
    target: Point,
    view: &Point,              // 令牌字段——持有对 target 的只读视图
}
```

**クロージャによるキャプチャ**
— クロージャは任意の値をキャプチャするのと同様にトークンをキャプチャする：

```yaoxiang
// ✅ 闭包捕获 &Float 令牌（Dup 类型，自由复制到闭包中）
filter_by_threshold: (items: List(Point), threshold: &Float) -> List(Point) = {
    items.filter(|p| p.x > threshold)
}
```

### 12.4 自動借用の選択

呼び出し側コンパイラは以下の優先順位で自動的に選択する：

```
1. 実引数が後続でも使用される → トークン（&T または &mut T、メソッドシグネチャによる）の作成を優先
2. 実引数が後続で不要 → Move
3. 優先マッチング順序：&T < &mut T < Move
```

```yaoxiang
p = Point(1.0, 2.0)
p.print()          // print 的参数类型为 &Point → 编译器创建 &Point 令牌
p.shift(1.0, 1.0)  // shift 的参数类型为 &mut Point → 编译器创建 &mut Point 令牌
p2 = p             // 后续不再使用 → Move
```

### 12.5 トークン競合の検出

コンパイラはトークン値に対して**フロー敏感生存解析**を行い、各トークンの状態（アクティブ/移動済み）を追跡する：

```yaoxiang
// ❌ &mut 和派生的 &T 不能同时活跃
bad_alias: (p: &mut Point) -> Void = {
    p.x = 10.0                   // ✅ 正常使用 WriteToken
    print(p.y)
}

// ✅ 令牌作用域结束后自动释放
good_seq: (p: &mut Point) -> Void = {
    {
        // 内部作用域
        print(p.x)               // 使用 &mut Point
    }
    // 内部作用域结束
    p.x = 10.0                   // ✅ WriteToken 仍可用
}

// ❌ 同一实参不能同时创建 &mut 令牌和其他令牌
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)                  // ❌ p 同时派生 &mut 和 & 令牌
```

### 12.6 コンパイラ内部：ブランド機構

ユーザーはブランドを決して目にしない。コンパイラは内部で各トークンにコンパイル時一意識別子を割り当てる：

```
ユーザーが見るもの         コンパイラ内部表現
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N はコンパイル時一意の整数
&mut Point     →  WriteToken(Point, #M)   // #M はコンパイル時一意の整数
```

ブランドの用途：

- **偽造防止**：トークンは所有者カプセルからのみ取得でき、凭空に構築できない
- **関連追跡**：フィールドアクセスから派生した `&Float`
  は派生ブランド（`#N.field_x`）を持ち、コンパイラは親トークンまで追跡できる
- **競合検出**：同源の WriteToken と派生 ReadToken は同時にアクティブにできない

ブランドは単相化とインライン化後に完全に消滅し、生成された機械語には存在しない。**ランタイムオーバーヘッドがゼロ。**

### 12.7 トークン Sum 型

```
&BorrowToken ::= &T          // ReadToken（ソースデータを凍結 → Dup 安全）
               | &mut T      // WriteToken（排他的読み書き → Linear）
```

### 12.8 借用トークン vs ref

|              | `&T` / `&mut T`                                      | `ref`                                     |
| ------------ | ---------------------------------------------------- | ----------------------------------------- |
| 何をするか   | 一目見る/その場で変更                                | 共有保持                                  |
| 範囲         | トークン値のスコープに従う                           | スコープをまたぐ                          |
| コスト       | ゼロオーバーヘッド（零サイズ型、コンパイル後に消滅） | Rc または Arc（コンパイラが選択）         |
| エスケープ   | 可（トークンは戻り値/構造体/クロージャで伝播する）   | そもそもエスケープ用である                |
| タスク間     | 不可（トークンはタスク間渡しを実装していない）       | 可（コンパイラが自動的に Arc を選択）     |
| サイクル検出 | 関与しない                                           | タスク内ではサイレント、タスク間では lint |

---

## 付録：型定義クイックリファレンス

### A.1 型定義

```
// === 记录类型（花括号） ===

// 记录类型
Point: Type = { x: Float, y: Float }

// 带变体的记录类型（使用函数字段）
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// === 接口类型（花括号，字段全为函数） ===

// 接口定义
Serializable: Type = { serialize: () -> String }

// 实现接口的类型
Point: Type = {
    x: Float,
    y: Float,
    Serializable    // 实现 Serializable 接口
}

// === 函数类型 ===

Adder: Type = (Int, Int) -> Int
```

### A.2 ジェネリック構文

```
// 泛型类型
List: (T: Type) -> Type = { data: Array(T), length: Int }
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// 泛型函数
map: (T: Type, R: Type)(list: List(T), f: (T) -> R) -> List(R) = { ... }

// 类型约束
clone: (T: Clone)(value: T) -> T = value.clone()
combine: (T: Clone + Add)(a: T, b: T) -> T = body

// 关联类型
Iterator: (T: Type) -> Type = { Item: T, next: () -> Option(T) }

// 编译期泛型
factorial: (n: Int)(n: n) -> Int = { ... }
StaticArray: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }

// 条件类型
If: (C: Bool, T: Type, E: Type) -> Type = match C { True => T, False => E }

// 函数特化
sum: (arr: Array(Int)) -> Int = { ... }
sum: (arr: Array(Float)) -> Float = { ... }
```

### A.3 型属性クイックリファレンス

```
// === Move（默认） ===
// 所有类型默认 Move。赋值、传参、返回 = 所有权转移

// === 原语值类型（编译器内置） ===
Int, Float,     // 赋值时自动值复制，两个值完全独立
Bool, Char      // 不是 Dup，是编译器对原语的内置处理

// === Dup（浅拷贝：复制句柄，共享底层数据） ===
&T              // 零大小读取令牌，复制令牌 = 多个视角指向同一数据
ref T           // Rc/Arc 复制 = 引用计数+1，共享堆数据

// === Linear ===
&mut T          // 零大小写入令牌，Linear（独占，不可复制）

// === Clone（显式深复制） ===
value.clone()   // 创建独立副本，修改不影响原值
```

### A.4 借用トークンクイックリファレンス

```
// === 借用令牌 ===
&T              // 零大小编译期读令牌，冻结源数据 → Dup（可复制）
&mut T          // 零大小编译期写令牌，独占读写 → Linear（不可复制）

// 调用端自动选择
// 1. 实参后续还有使用 → 创建令牌
// 2. 实参后续不再使用 → Move
// 3. 优先匹配：&T < &mut T < Move

// 令牌传播
// ✅ 可返回、可存结构体、可被闭包捕获
// ❌ 不可跨任务（令牌未实现跨任务传递）
```

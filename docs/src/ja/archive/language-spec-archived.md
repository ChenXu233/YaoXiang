> **注意：本文档はアーカイブ済みであり、メンテナンスされていません。**
> **新しい言語仕様ドキュメントを参照してください：[言語仕様](../reference/language-spec/index.md)**

---

# YaoXiang（爻象）プログラミング言語仕様

> バージョン：v1.8.0
> 状態：仕様
> 著者：晨煦
> 日付：2024-12-31
> 更新：2026-02-22 - meta typeはキーワードではない。

---

## 第1章：序論

### 1.1 範囲

本書はYaoXiangプログラミング言語の構文と意味論を定義しています。これは言語の権威ある参照文書であり、コンパイラおよびツール実装者を対象としています。

チュートリアルとサンプルコードについては、[YaoXiangガイド](../guide/YaoXiang-book.md)および[tutorial/](../tutorial/)ディレクトリを参照してください。

### 1.2 適合性

本書に定義されたすべての規則を満たすプログラムまたは実装は、YaoXiang仕様に適合するものとみなされます。

---

## 第2章：字句構造

### 2.1 ソースファイル

YaoXiangソースファイルはUTF-8エンコーディングを使用する必要があります。ソースファイルは通常、`.yx`拡張子を持ちます。

### 2.2 字句トークンの分類

| カテゴリ | 説明 | 例 |
|------|------|------|
| 識別子 | 文字またはアンダースコアで始まる | `x`, `_private`, `my_var` |
| キーワード | 言語が事前定義した予約語 | `Type`, `pub`, `use` |
| リテラル | 固定値 | `42`, `"hello"`, `true` |
| 演算子 | 算術記号 | `+`, `-`, `*`, `/` |
| 区切り文字 | 構文区切り文字 | `(`, `)`, `{`, `}`, `,` |

### 2.3 キーワード

YaoXiangは非常に少数のキーワードを定義しています：

```
pub    use    spawn
ref    mut    if     elif
else   match  while  for    return
break  continue as     in     unsafe
```

これらのキーワードはどのようなコンテキストでも特殊な意味を持ち、識別子として使用することはできません。

### 2.4 予約語

| 予約語 | 型 | 説明 |
|--------|------|------|
| `Type` | Type | meta type |
| `true` | Bool | ブール値の真 |
| `false` | Bool | ブール値の偽 |
| `void` | Void | 空値 |
| `some(T)` | Option | Optionの値variant |
| `ok(T)` | Result | Resultの成功variant |
| `err(E)` | Result | Resultのエラーvariant |

### 2.5 識別子

識別子は文字またはアンダースコアで始まり、後続の文字は文字、数字、またはアンダースコアにできます。識別子は大小文字を区別します。

特別な識別子：
- `_` はプレースホルダーとして使用され、ある値を無視することを示します
- アンダースコアで始まる識別子はプライベートメンバーを示します

### 2.6 リテラル

#### 2.6.1 整数

```
Decimal     ::= [0-9][0-9_]*
Octal       ::= 0o[0-7][0-7_]*
Hex         ::= 0x[0-9a-fA-F][0-9a-fA-F_]*
Binary      ::= 0b[01][01_]*
```

#### 2.6.2 浮動小数点数

```
Float       ::= [0-9][0-9_]* '.' [0-9][0-9_]* ([eE][+-]?[0-9][0-9_]*)?
```

#### 2.6.3 文字列

```
String      ::= '"' ([^"\\] | EscapeSequence)* '"'
Escape      ::= '\\' ([nrt'"\\] | UnicodeEscape)
Unicode     ::= 'u' '{' HexDigit+ '}'
```

#### 2.6.4 コレクション

```
List        ::= '[' Expr (',' Expr)* ']'
Dict        ::= '{' String ':' Expr (',' String ':' Expr)* '}'
Set         ::= '{' Expr (',' Expr)* '}'
```

#### 2.6.5 リスト内包表記

```
ListComp    ::= '[' Expr 'for' Identifier 'in' Expr (',' Expr)* ('if' Expr)? ']'
```

#### 2.6.6 メンバーシップ検査

```
Membership  ::= Expr 'in' Expr
```

### 2.7 コメント

```
// 単一行コメント

/* 複数行コメント
   複数行にまたがる */
```

### 2.8 インデント規則

コードは4スペースのインデントを使用する必要があります。Tab文字の使用は禁止です。これは強制的な構文規則です。

---

## 第3章：型

### 3.1 型の分類

```
TypeExpr    ::= PrimitiveType
              | StructType
              | EnumType
              | InterfaceType
              | TupleType
              | FnType
              | GenericType
              | TypeRef
              | TypeUnion
              | TypeIntersection
              | ConstrainedType
              | AssociatedType
```

### 3.2 primitive type

| 型 | 説明 | デフォルトサイズ |
|------|------|----------|
| `Type` | meta type | 0バイト |
| `Void` | 空値 | 0バイト |
| `Bool` | ブール値 | 1バイト |
| `Int` | 符号付き整数 | 8バイト |
| `Uint` | 符号なし整数 | 8バイト |
| `Float` | 浮動小数点数 | 8バイト |
| `String` | UTF-8文字列 | 可変 |
| `Char` | Unicode文字 | 4バイト |
| `Bytes` | 生バイト | 可変 |

ビット幅付き整数：`Int8`, `Int16`, `Int32`, `Int64`, `Int128`
ビット幅付き浮動小数点：`Float32`, `Float64`

### 3.3 record type

**統一構文**：`Name: Type = { field1: Type1, field2: Type2, ... }`

```
RecordType  ::= '{' FieldList? '}'
FieldList   ::= Field (',' Field)* ','?
Field       ::= Identifier ':' TypeExpr
            |  Identifier                 # インターフェース制約
```

```yaoxiang
// 単純なrecord type
Point: Type = { x: Float, y: Float }

// 空のrecord type
Empty: Type = {}

// ジェネリクスを持つrecord type
Pair: Type[T] = { first: T, second: T }

// インターフェースを実装するrecord type
Point: Type = {
    x: Float,
    y: Float,
    Drawable,
    Serializable
}
```

**規則**：
- record typeは波括弧`{}`で定義されます
- フィールド名の直後にコロンと型を続けます
- 型本体内にインターフェース名を記述するとそのインターフェースを実装します

#### 3.3.1 フィールドのデフォルト値

型のフィールドにはデフォルト値を指定でき、構築時に省略可能です：

```yaoxiang
// デフォルト値のあるフィールド - 構築時に省略可能
Point: Type = {
    x: Float = 0,
    y: Float = 0
}

// 使用例
Point()           # → Point(x=0, y=0)
Point(x=1)       # → Point(x=1, y=0)
Point(x=1, y=2) # → Point(x=1, y=2)

// デフォルト値のないフィールド - 構築時に必須
Point2: Type = {
    x: Float,
    y: Float
}

// 使用例
Point2(x=1, y=2) # ✓
Point2()          # ✗ エラー
```

**規則**：
- `field: Type = expression` → デフォルト値あり、構築時に省略可能
- `field: Type` → デフォルト値なし、構築時に必須

#### 3.3.2 組み込みバインディング

型定義本体内で直接メソッドをバインディングできます：

```yaoxiang
// 方法1：外部関数を参照してバインディング
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]    # 位置0にバインディング
}
// 呼び出し：p1.distance(p2) → distance(p1, p2)

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
// 呼び出し：p1.distance(p2) → distance(p1, p2)
```

### 3.4 列挙型（variant型）

```
EnumType    ::= '{' Variant ('|' Variant)* '}'
Variant     ::= Identifier (':' TypeExpr)?
```

**構文**：`Name: Type = { Variant1 | Variant2(params) | ... }`

```yaoxiang
// パラメータなしのvariant
Color: Type = { red | green | blue }

// パラメータ付きvariant
Option: Type[T] = { some(T) | none }

// 混合
Result: Type[T, E] = { ok(T) | err(E) }

// パラメータなしのvariantはパラメータなしのコンストラクタに相当
Bool: Type = { true | false }
```

### 3.5 インターフェース型

```
InterfaceType ::= '{' FnField (',' FnField)* ','?
FnField       ::= Identifier ':' FnType
FnType        ::= '(' ParamTypes? ')' '->' TypeExpr
```

**構文**：インターフェースはフィールドがすべて関数型であるrecord typeです

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

**インターフェース実装**：型は定義の末尾にインターフェース名を列挙することでインターフェースを実装します

```yaoxiang
// インターフェースを実装する型
Point: Type = {
    x: Float,
    y: Float,
    Drawable,        # Drawableインターフェースを実装
    Serializable     # Serializableインターフェースを実装
}
```

**インターフェースへの直接代入**：具象型はインターフェース型の変数に直接代入できます（構造的サブタイピング）

```yaoxiang
// 直接代入（コンパイル時に具象型を決定可能 → ゼロオーバーヘッド呼び出し）
d: Drawable = Circle(1)
d.draw(screen)        # コンパイル後：vtableなしでcircle_drawを直接呼び出し

// 関数の戻り値（コンパイル時に判断不可 → vtable呼び出し）
d: Drawable = get_shape()
d.draw(screen)        # vtableでメソッドを検索

// インターフェースを関数パラメータとして使用
process: (d: Drawable) -> Void = d.draw(screen)
```

**コンパイル時最適化戦略**：

| シナリオ | 推論結果 | 呼び出し方式 |
|------|----------|----------|
| 具象型を直接代入 | 具象型を決定可能 | 直接呼び出し（ゼロオーバーヘッド） |
| 関数の戻り値 | 不明 | vtable |
| 異種混合コレクション | 複数型 | vtable |

### 3.6 タプル型

```
TupleType   ::= '(' TypeList? ')'
TypeList    ::= TypeExpr (',' TypeExpr)* ','?
```

### 3.7 関数型

```
FnType      ::= '(' ParamList? ')' '->' TypeExpr
ParamList   ::= TypeExpr (',' TypeExpr)*
```

### 3.8 ジェネリクス型

#### 3.8.1 ジェネリックパラメータ構文

```
GenericType     ::= Identifier '[' TypeArgList ']'
TypeArgList     ::= TypeExpr (',' TypeExpr)* ','?
GenericParams   ::= '[' Identifier (',' Identifier)* ']'
                 |  '[' Identifier ':' TypeBound (',' Identifier ':' TypeBound)* ']'
TypeBound       ::= Identifier
                 |  Identifier '+' Identifier ('+' Identifier)*
```

#### 3.8.2 ジェネリック型定義

```yaoxiang
// 基本的なジェネリック型
Option: Type[T] = {
    some: (T) -> Self,
    none: () -> Self
}

Result: Type[T, E] = {
    ok: (T) -> Self,
    err: (E) -> Self
}

List: Type[T] = {
    data: Array[T],
    length: Int,
    push: [T](self: List[T], item: T) -> Void,
    get: [T](self: List[T], index: Int) -> Option[T]
}
```

#### 3.8.3 型推論

```yaoxiang
// コンパイラが自動的にジェネリックパラメータを推論
numbers: List[Int] = List(1, 2, 3)  # コンパイラがList[Int]を推論
```

### 3.9 型制約

#### 3.9.1 単一constraint

```
ConstrainedType ::= '[' Identifier ':' TypeBound ']' TypeExpr
```

```yaoxiang
// インターフェース型定義（constraintとして使用）
Clone: Type = {
    clone: (Self) -> Self
}

// 制約を使用
clone: [T: Clone](value: T) -> T = value.clone()
```

#### 3.9.2 複数constraint

```yaoxiang
// 複数constraint構文
combine: [T: Clone + Add](a: T, b: T) -> T = {
    a.clone() + b
}

// ジェネリックコンテナのソート
sort: [T: Clone + PartialOrd](list: List[T]) -> List[T] = {
    result = list.clone()
    quicksort(&mut result)
    return result
}
```

#### 3.9.3 関数型constraint

```yaoxiang
// 高階関数constraint
call_twice: [T, F: Fn() -> T](f: F) -> (T, T) = (f(), f())

compose: [A, B, C, F: Fn(A) -> B, G: Fn(B) -> C](a: A, f: F, g: G) -> C = g(f(a))
```

### 3.10 関連型

#### 3.10.1 関連型定義

```
AssociatedType ::= Identifier ':' TypeExpr
```

```yaoxiang
// Iterator trait（record type構文を使用）
Iterator: Type[T] = {
    Item: T,                    # 関連型
    next: (Self) -> Option[T],
    has_next: (Self) -> Bool
}

// 関連型を使用
collect: [T, I: Iterator[T]](iter: I) -> List[T] = {
    result = List[T]()
    while iter.has_next() {
        if let Some(item) = iter.next() {
            result.push(item)
        }
    }
    return result
}
```

#### 3.10.2 ジェネリック関連型（GAT）

```yaoxiang
// より複雑な関連型
Container: Type[T] = {
    Item: T,
    IteratorType: Iterator[T],  # 関連型もジェネリック
    iter: (Self) -> IteratorType
}
```

### 3.11 コンパイル時ジェネリクス

#### 3.11.1 リテラル型制約

```
LiteralType   ::= Identifier ':' Int          # コンパイル時定数
CompileTimeFn ::= '[' Identifier ':' Int ']' '(' Identifier ')' '->' TypeExpr
```

**コア設計**：`[n: Int]`ジェネリックパラメータ + `(n: n)`値パラメータを使用して、コンパイル時定数と実行時値を区別します。

```yaoxiang
// コンパイル時階乗：パラメータはコンパイル時に既知のリテラルでなければならない
factorial: [n: Int](n: n) -> Int = {
    match n {
        0 => 1,
        _ => n * factorial(n - 1)
    }
}

// コンパイル時定数配列
StaticArray: Type[T, N: Int] = {
    data: T[N],      # コンパイル時にサイズが判明している配列
    length: N
}

// 使用方法
arr: StaticArray[Int, factorial(5)]  # コンパイラがコンパイル時にfactorial(5) = 120を計算
```

#### 3.11.2 コンパイル時定数配列

```yaoxiang
// 行列型で使用
Matrix: Type[T, Rows: Int, Cols: Int] = {
    data: Array[Array[T, Cols], Rows]
}

// コンパイル時次元検証
identity_matrix: [T: Add + Zero + One, N: Int](size: N) -> Matrix[T, N, N] = {
    // ...
}
```

### 3.12 条件型

#### 3.12.1 If条件型

```
IfType        ::= 'If' '[' BoolExpr ',' TypeExpr ',' TypeExpr ']'
```

```yaoxiang
// 型レベルIf
If: Type[C: Bool, T, E] = match C {
    True => T,
    False => E
}

// 例：コンパイル時分岐
NonEmpty: Type[T] = If[T != Void, T, Never]

// コンパイル時検証
Assert: Type[C: Bool] = match C {
    True => Void,
    False => compile_error("Assertion failed")
}
```

#### 3.12.2 型族

```yaoxiang
// コンパイル時型変換
AsString: Type[T] = match T {
    Int => String,
    Float => String,
    Bool => String,
    _ => String
}
```

### 3.13 型union

```
TypeUnion     ::= TypeExpr '|' TypeExpr
```

### 3.14 型intersection

```
TypeIntersection ::= TypeExpr '&' TypeExpr
```

**構文**：型intersection `A & B`はAとBの両方を同時に満たす型を表します

```yaoxiang
// インターフェース合成 = 型intersection
DrawableSerializable: Type = Drawable & Serializable

// intersection型を使用
process: [T: Drawable & Serializable](item: T, screen: Surface) -> String = {
    item.draw(screen)
    return item.serialize()
}
```

### 3.15 関数オーバーロードと特殊化

```yaoxiang
// 基本的な特殊化：関数オーバーロードを使用（コンパイラが自動選択）
sum: (arr: Array[Int]) -> Int = {
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Array[Float]) -> Float = {
    return simd_sum_float(arr.data, arr.length)
}

// 汎用実装
sum: [T: Add](arr: Array[T]) -> T = {
    result = Zero::zero()
    for item in arr {
        result = result + item
    }
    return result
}
```

### 3.16 プラットフォーム特殊化

```yaoxiang
// プラットフォーム型列挙型（標準ライブラリ定義）
Platform: Type = X86_64 | AArch64 | RISC_V | ARM | X86

// Pは事前定義されたジェネリックパラメータ名で、現在のコンパイルプラットフォームを表す
sum: [P: X86_64](arr: Array[Float]) -> Float = {
    return avx2_sum(arr.data, arr.length)
}

sum: [P: AArch64](arr: Array[Float]) -> Float = {
    return neon_sum(arr.data, arr.length)
}
```

---

## 第3章（続き）：構文設計の説明

### 3.17 名前付き関数とLambdaの関係

**コア理解**：名前付き関数とLambda式は同じものです！唯一のの違いは、名前付き関数に名前があるということです。

```yaoxiang
// 这两者本质完全相同
add: (a: Int, b: Int) -> Int = a + b           # 具名関数（推薦）
add: (a: Int, b: Int) -> Int = (a, b) => a + b  # Lambda形式（完全同等）
```

**糖衣構文モデル**：

```
// 具名関数 = Lambda + 名前
name: (Params) -> ReturnType = body

// 本質的には
name: (Params) -> ReturnType = (params) => body
```

**要点**：シグネチャがパラメータ型を完全に宣言している場合、Lambdaヘッダーのパラメータ名は冗長になり、省略可能です。

### 3.18 パラメータスコープ規則

**パラメータが外側変数をオーバーライド**：シグネチャ内のパラメータスコープは関数本体をオーバーライドし、内部スコープの方が優先順位が高くなります。

```yaoxiang
x = 10  # 外側変数
double: (x: Int) -> Int = x * 2  # ✅ パラメータxが外側のxをオーバーライド、結果は20
```

### 3.19 型注釈の位置

型注釈は次のいずれかの位置に配置でき、**少なくとも1箇所に注釈を付ける必要があります**：

| 注釈位置 | 形式 | 説明 |
|----------|------|------|
| シグネチャのみ | `double: (x: Int) -> Int = x * 2` | ✅ 推奨 |
| Lambdaヘッダーのみ | `double = (x: Int) => x * 2` | ✅ 有効 |
| 両方に注釈 | `double: (x: Int) -> Int = (x: Int) => x * 2` | ✅ 冗長だが許可 |

### 4.1 式分類

```
Expr        ::= Literal
              | Identifier
              | FnCall
              | MemberAccess
              | IndexAccess
              | UnaryOp
              | BinaryOp
              | TypeCast
              | IfExpr
              | MatchExpr
              | Block
              | Lambda
```

### 4.2 演算子の優先順位

| 優先度 | 演算子 | 結合性 |
|--------|--------|--------|
| 1 | `()` `[]` `.` | 左から右 |
| 2 | `as` | 左から右 |
| 3 | `*` `/` `%` | 左から右 |
| 4 | `+` `-` | 左から右 |
| 5 | `<<` `>>` | 左から右 |
| 6 | `&` `\|` `^` | 左から右 |
| 7 | `==` `!=` `<` `>` `<=` `>=` | 左から右 |
| 8 | `not` | 右から左 |
| 9 | `and` `or` | 左から右 |
| 10 | `if...else` | 右から左 |
| 11 | `=` `+=` `-=` `*=` `/=` | 右から左 |

### 4.3 関数呼び出し

```
FnCall      ::= Expr '(' ArgList? ')'
ArgList     ::= Expr (',' Expr)* (',' NamedArg)* | NamedArg (',' NamedArg)*
NamedArg    ::= Identifier ':' Expr
```

### 4.4 メンバーアクセス

```
MemberAccess::= Expr '.' Identifier
```

### 4.5 インデックスアクセス

```
IndexAccess ::= Expr '[' Expr ']'
```

### 4.6 型変換

```
TypeCast    ::= Expr 'as' TypeExpr
```

### 4.7 条件式

```
IfExpr      ::= 'if' Expr Block ('elif' Expr Block)* ('else' Block)?
```

### 4.8 pattern matching

```
MatchExpr   ::= 'match' Expr '{' MatchArm+ '}'
MatchArm    ::= Pattern ('|' Pattern)* ('if' Expr)? '=>' Expr ','
Pattern     ::= Literal
              | Identifier
              | Wildcard
              | StructPattern
              | TuplePattern
              | EnumPattern
              | OrPattern
```

### 4.9 ブロック式

```
Block       ::= '{' Stmt* Expr? '}'
```

### 4.10 Lambda式

```
Lambda      ::= '(' ParamList? ')' '=>' Expr
            |  '(' ParamList? ')' '=>' Block
```

---

## 第5章：文

### 5.1 文分類

```
Stmt        ::= LetStmt
              | ExprStmt
              | ReturnStmt
              | BreakStmt
              | ContinueStmt
              | IfStmt
              | MatchStmt
              | LoopStmt
              | WhileStmt
              | ForStmt
```

### 5.2 変数宣言

```
LetStmt     ::= ('mut')? Identifier (':' TypeExpr)? '=' Expr
```

### 5.3 return文

```
ReturnStmt  ::= 'return' Expr?
```

### 5.4 break文

```
BreakStmt   ::= 'break' Identifier?
```

### 5.5 continue文

```
ContinueStmt::= 'continue'
```

### 5.6 if文

```
IfStmt      ::= 'if' Expr Block ('elif' Expr Block)* ('else' Block)?
```

### 5.7 match文

```
MatchStmt   ::= 'match' Expr '{' MatchArm+ '}'
```

### 5.8 while文

```
WhileStmt   ::= 'while' Expr Block
```

### 5.9 for文

```
ForStmt     ::= 'for' 'mut'? Identifier 'in' Expr Block
```

#### 5.9.1 意味論：各反復は新しい値へのバインディング

YaoXiangのforループの意味論は従来の言語とは異なります：**各反復は新しい値へのバインディングであり、同じ変数を変更することではありません**。

```yaoxiang
// 例：for i in 1..5
for i in 1..5 {
    print(i)
}
```

**実行過程**：

| 反復 | ループ変数の動作 |
|------|----------------|
| 1回目 | 新規バインディング `i = 1` を作成、ループ本体実行、1を出力 |
| 2回目 | 新規バインディング `i = 2` を作成（以前のバインディングは破棄）、ループ本体実行、2を出力 |
| 3回目 | 新規バインディング `i = 3` を作成、ループ本体実行、3を出力 |
| 4回目 | 新規バインディング `i = 4` を作成、ループ本体実行、4を出力 |
| 終了 | ループ本体終了、バインディング破棄 |

**要点**：各反復終了後、その反復で作成されたバインディングは破棄されます。次の反復は完全に新しいバインディングであり、前の反復のバインディングとは一切関係ありません。

#### 5.9.2 forとfor mutの違い

| 構文 | ループ変数の可変性 | 説明 |
|------|----------------|------|
| `for i in 1..5` | 不変 | ループ本体内でバインディングを変更できない |
| `for mut i in 1..5` | 可変 | ループ本体内でバインディングを変更できる |

```yaoxiang
// ✅ 有効：各反復で新しい値にバインディングするため、変更は不要
for i in 1..5 {
    print(i)  # iの値を読み取り
}

// ❌ エラー：不変バインディングは変更不可
for i in 1..5 {
    i = i + 1  # エラー：不変バインディングは変更不可
}

// ✅ 有効：for mutを使用するとバインディングを変更可能
for mut i in 1..5 {
    i = i + 1  # 変更可
}
```

#### 5.9.3 シャドーイング検査

forループ変数は外側スコープに既に存在する変数をシャドーイングできません：

```yaoxiang
// ❌ エラー：iは外部で既に宣言されている
i = 10
for i in 1..5 {
    print(i)
}

// ✅ 正しい：異なる変数名を使用
i = 10
for j in 1..5 {
    print(j)
}
```

エラーコード：`E2013 - Cannot shadow existing variable`

#### 5.9.4 他の言語との比較

| 言語 | forループ変数の意味論 |
|------|------------------|
| YaoXiang | 各反復で新しい値にバインディング |
| Rust | 同じ変数を変更（mutが必要） |
| Python | 同じ変数を変更（mut不要） |
| C/C++ | 同じ変数を変更（ポインタまたは参照が必要） |

**設計理由**：YaoXiangがバインディング意味論を採用した理由は：
1. 各反復終了後、ループ本体内の変数は破棄される
2. 次の反復は完全に新しいバインディングである
3. これにより反復間の状態を考える必要がなくなり、より安全になる

---

## 第6章：関数

### 6.1 統一関数モデル

**コア構文**：`name: type = value`

YaoXiangは**統一声明モデル**を採用しています：変数、関数、メソッドはすべて同じ形式 `name: type = value` を使用します。

```
Declaration   ::= Identifier ':' Type '=' Expression
FunctionDef   ::= Identifier GenericParams? '(' Parameters? ')' '->' Type '=' (Expression | Block)
GenericParams ::= '[' Identifier (',' Identifier)* ']'
Parameters    ::= Parameter (',' Parameter)*
Parameter     ::= Identifier ':' TypeExpr
```

### 6.2 変数宣言

```yaoxiang
// 基本構文
x: Int = 42
name: String = "YaoXiang"
mut counter: Int = 0

// 型推論
y = 100  # Intとして推論
```

### 6.3 関数定義

#### 6.3.1 完全構文

```yaoxiang
// パラメータ名はシグネチャで宣言
add: (a: Int, b: Int) -> Int = {
    return a + b
}

// 単一パラメータ
inc: (x: Int) -> Int = x + 1

// パラメータなし関数
main: () -> Void = {
    print("Hello")
}

// 複数行関数本体
calc: (x: Float, y: Float, op: String) -> Float = {
    return match op {
        "+" => x + y,
        "-" => x - y,
        _ => 0.0
    }
}
```

#### 6.3.2 戻り値規則

```yaoxiang
// 非Void戻り型 - returnを使用する必要がある
add: (a: Int, b: Int) -> Int = {
    return a + b
}

// Void戻り型 - returnの使用は任意
print: (msg: String) -> Void = {
    // return不要
}

// 単一行式 - 直接値を返す、return不要
greet: (name: String) -> String = "Hello, ${name}!"
```

### 6.4 ジェネリック関数

```yaoxiang
// ジェネリック関数定義
map: [T, R](list: List[T], f: Fn(T) -> R) -> List[R] = {
    result = List[R]()
    for item in list {
        result.push(f(item))
    }
    return result
}

// ジェネリックconstraintを使用
clone: [T: Clone](value: T) -> T = value.clone()

// 複数型パラメータ
combine: [T, U](a: T, b: U) -> (T, U) = (a, b)
```

### 6.5 メソッド定義

#### 6.5.1 型メソッド

**構文**：`Type.method: (self: Type, ...) -> Return = ...`

```yaoxiang
// 型メソッド：特定の型に関連付け
Point.draw: (self: Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}

Point.serialize: (self: Point) -> String = {
    return "Point(${self.x}, ${self.y})"
}

// メソッド糖衣構文を使用
p: Point = Point(1.0, 2.0)
p.draw(screen)           # 糖衣構文 → Point.draw(p, screen)
```

#### 6.5.2 通常メソッド

**構文**：`name: (Type, ...) -> Return = ...`（型に関連付けない）

```yaoxiang
// 通常メソッド：型に関連付けず、独立関数として
distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}
```

### 6.6 メソッドバインディング

#### 6.6.1 手動バインディング

**構文**：`Type.method = function[positions]`

```yaoxiang
// 位置0にバインディング（デフォルト）
Point.distance = distance[0]

// 位置1にバインディング
Point.transform = transform[1]

// 複数位置バインディング
Point.scale = scale[0, 1]

// プレースホルダーを使用
Point.calc = func[0, _, 2]
```

#### 6.6.2 pub自動バインディング

`pub`で宣言された関数について、コンパイラは同一ファイルで定義された型に自動バインディングします：

```yaoxiang
// pubで宣言すると、コンパイラが自動バインディング
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

// コンパイラが自動推論：
// 1. Pointが現在のファイルで定義されている
// 2. 関数パラメータがPointを含む
// 3. Point.distance = distance[0]を実行

// 呼び出し
d = distance(p1, p2)           # 関数形式
d2 = p1.distance(p2)           # OOP糖衣構文
```

### 6.7 メソッドバインディング規則

| 規則 | 説明 |
|------|------|
| 位置は0から開始 | `func[0]`は第1パラメータ（インデックス0）をバインディング |
| 最大位置 | 関数のパラメータ数より小さい必要がある |
| 負数インデックス | `[-1]`は最後のパラメータを意味する |
| プレースホルダー | `_`はその位置をスキップし、用户提供に委ねる |

### 6.8 柯里化サポート

バインディングは自然に柯里化をサポートします。呼び出し時に提供されたパラメータが残りパラメータより少ない場合、残りのパラメータを受け取る関数を返します：

```yaoxiang
// 元の関数：5つのパラメータ
calculate: (scale: Float, a: Point, b: Point, x: Float, y: Float) -> Float = ...

// バインディング：Point.calc = calculate[1, 2]
// バインディング後の残りパラメータ：scale, x, y

// 呼び出しシナリオ
p1.calc(2.0, 10.0, 20.0)       # 3つのパラメータを提供 → 直接呼び出し
p1.calc(2.0)                    # 1つのパラメータを提供 → (Float, Float) -> Floatを返す
p1.calc()                       # 0個のパラメータを提供 → (Float, Float, Float) -> Floatを返す
```

### 6.9 spawn関数と注釈

#### 6.9.1 spawn関数（並作関数）

```
SpawnFn     ::= Identifier ':' FnType 'spawn' '=' Lambda
FnType      ::= '(' ParamTypes? ')' '->' TypeExpr ('@' Annotation)?
Annotation  ::= 'block' | 'eager'
```

**関数注釈**：

| 注釈 | 位置 | 動作 |
|------|------|------|
| `@block` | 戻り型の後 | 並行最適化を無効化、完全な逐次実行 |
| `@eager` | 戻り型の後 | 先行評価を強制 |

**構文例**：

```
// spawn関数：並行実行可能
fetch_data: (url: String) -> JSON spawn = { ... }

// @block同期関数：完全な逐次実行
main: () -> Void @block = { ... }

// @eager先行関数：即時実行
compute: (n: Int) -> Int @eager = { ... }
```

#### 6.9.2 spawnブロック

明示的に宣言された並行領域で、ブロック内のタスクはspawn実行されます：

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
```

**例**：

```
// spawnブロック：明示的並行
(result_a, result_b) = spawn {
    parse(fetch("url1")),
    parse(fetch("url2"))
}
```

#### 6.9.3 spawnループ

データ並列ループで、ループ本体がすべてのデータ要素にspawn実行されます：

```
SpawnFor    ::= Identifier '=' 'spawn' 'for' Identifier 'in' Expr '{' Expr '}'
```

**例**：

```
// spawnループ：データ並列
results = spawn for item in items {
    process(item)
}
```

#### 6.9.4 エラー伝播演算子

```
ErrorPropagate ::= Expr '?'
```

**例**：

```
process: (p: Point) -> Result[Data, Error] = {
    data = fetch_data()?      # エラーを自動伝播
    transform(data)?
}
```

---

## 第7章：モジュール

### 7.1 モジュール定義

モジュールはファイルを境界として使用します。各`.yx`ファイルは1つのモジュールです。

```
// ファイル名がモジュール名になる
// Math.yx
pub pi: Float = 3.14159
pub sqrt(Float) -> Float = (x) => { ... }
```

### 7.2 モジュールのインポート

```
Import       ::= 'use' ModuleRef ImportSpec?
ImportSpec   ::= ('{' ImportItems '}') ('as' AliasList)?
              |  'as' AliasList
ImportItems  ::= Identifier (',' Identifier)* ','?
AliasList    ::= Identifier (',' Identifier)*
```

| 構文 | 説明 | 例 |
|------|------|------|
| `use path;` | モジュールをインポート、最後の部分でアクセス | `use std.io;` → `io.print` |
| `use path.{a, b};` | 指定項目をインポート | `use std.io.{print};` → `print` |
| `use path as alias;` | インポートして名前変更 | `use std.io as io;` → `io.print` |
| `use path.{i1, i2} as a, b;` | 指定項目をインポートして名前変更 | `use std.io.{print, read} as p, r;` → `p`, `r` |

---

## 第8章：メモリ管理

### 8.1 所有権モデル

YaoXiangは**所有権モデル**を使用してメモリを管理し、各値には一意の所有者がいます：

| 意味論 | 説明 | 構文 |
|------|------|------|
| **Move** | デフォルト意味論、所有権移転 | `p2 = p` |
| **ref** | 共有（Arc参照カウント） | `shared = ref p` |
| **clone()** | 明示的コピー | `p2 = p.clone()` |

### 8.2 Move意味論（デフォルト）

```yaoxiang
// 代入 = Move（ゼロコピー）
p: Point = Point(1.0, 2.0)
p2 = p              # Move、pは失效

// 関数引数 = Move
process: (p: Point) -> Void = {
    // pの所有権が移転
}

// 戻り値 = Move
create: () -> Point = {
    p = Point(1.0, 2.0)
    return p        # Move、所有権移転
}
```

### 8.3 refキーワード（Arc）

`ref`キーワードは**参照カウントポインタ**（Arc）を作成し、安全な共有に使用します：

```yaoxiang
// Arcを作成
p: Point = Point(1.0, 2.0)
shared = ref p      # Arc、スレッドセーフ

// 共有アクセス
spawn(() => print(shared.x))   # ✅ 安全

// Arcは自動的にライフサイクルを管理
// sharedがスコープを離れると、カウンタがゼロになり自動解放
```

**特徴**：
- スレッドセーフ参照カウント
- ライフサイクル自動管理
- spawn境界を越えて安全

### 8.4 clone()明示的コピー

```yaoxiang
// 明示的に値をコピー
p: Point = Point(1.0, 2.0)
p2 = p.clone()      # pとp2は独立

// 両方を変更可能で、互いに影響しない
p.x = 0.0           # ✅
p2.x = 0.0          # ✅
```

### 8.5 unsafeコードブロック

`unsafe`コードブロックは生ポインタの使用を許可し、システムレベルプログラミングに使用します：

```yaoxiang
// 生ポインタ型
PtrType ::= '*' TypeExpr

// unsafeコードブロック
UnsafeBlock ::= 'unsafe' '{' Stmt* '}'
```

**例**：

```yaoxiang
p: Point = Point(1.0, 2.0)

// 生ポインタはunsafeブロック内でのみ使用可能
unsafe {
    ptr: *Point = &p     # 生ポインタを取得
    (*ptr).x = 0.0       # 逆参照
}
```

**制限**：
- 生ポインタは`unsafe`ブロック内でのみ使用可能
- ユーザーはダングリング、解放後使用がないことを保証
- Send/Syncチェックに参加しない

### 8.7 所有権構文BNF

```bnf
// === 所有権式 ===

// Move（デフォルト）
MoveExpr     ::= Expr

// ref Arc
RefExpr      ::= 'ref' Expr

// clone
CloneExpr    ::= Expr '.clone' '(' ')'

// === 生ポインタ（unsafeのみ）===

PtrType       ::= '*' TypeExpr
UnsafeBlock   ::= 'unsafe' '{' Stmt* '}'
```

### 8.8 Send / Sync制約

| 制約 | 意味論 | 説明 |
|------|------|------|
| **Send** | スレッド間を安全に传输可能 | 値を別のスレッドに移動できる |
| **Sync** | スレッド間を安全に共有可能 | 不変参照を別のスレッドに共有できる |

**自動導出**：

```
// Send導出規則
Struct[T1, T2]: Send ⇐ T1: Send かつ T2: Send

// Sync導出規則
Struct[T1, T2]: Sync ⇐ T1: Sync かつ T2: Sync
```

**型制約**：

| 型 | Send | Sync | 説明 |
|------|------|------|------|
| `T`（値） | ✅ | ✅ | 不変データ |
| `ref T` | ✅ | ✅ | Arcスレッドセーフ |
| `*T` | ❌ | ❌ | 生ポインタは不安全 |

---

## 第8章（続き）：型システム制約

### 8.7 Send/Sync制約

YaoXiangはRustスタイルの型制約を使用して並行安全性を保証します：

| 制約 | 意味論 | 説明 |
|------|------|------|
| **Send** | スレッド間を安全に传输可能 | 値を別のスレッドに移動できる |
| **Sync** | スレッド間を安全に共有可能 | 不変参照を別のスレッドに共有できる |

**制約階層**：

```
Send ──► スレッド間を安全に传输可能
  │
  └──► Sync ──► スレッド間を安全に共有可能
       │
       └──► Send + Syncを満たす型は自動的に並行可能

Arc[T] はSend + Syncを実装（スレッドセーフ参照カウント）
Mutex[T] は内部可変性を提供
```

### 8.8 並行安全型

| 型 | 意味論 | 並行安全 | 説明 |
|------|------|----------|------|
| `T` | 不変データ | ✅ 安全 | デフォルト型、 멀티タスク読み取りは競合なし |
| `Ref[T]` | 可変参照 | ⚠️ 同期が必要 | 並行変更可能、マクロによるロック使用をコンパイル検査 |
| `Atomic[T]` | アトミック型 | ✅ 安全 | 低レベルアトミック操作、ロックフリー並行 |
| `Mutex[T]` | ミューテックスラップ | ✅ 安全 | 自動ロック解除解除、コンパイルが保証 |
| `RwLock[T]` | 読み取り書きロックラップ | ✅ 安全 | 読み取り多書き込み少シナリオを最適化 |

**構文**：

```
Mutex[T]    # ミューテックスラップされた可変データ
Atomic[T]   # アトミック型（Int、Floatのみなど）
RwLock[T]   # 読み取り書きロックラップ
```

**with糖衣構文**：

```
with mutex.lock() {
    // 臨界区間：Mutexで保護
    ...
}
```

---

## 第9章：エラー処理

### 9.1 Result型

```
Result: Type[T, E] = ok(T) | err(E)
```

**variantコンストラクタ**：

| variant | 構文 | 説明 |
|------|------|------|
| `ok(T)` | `ok(value)` | 成功値 |
| `err(E)` | `err(error)` | エラー値 |

### 9.2 Option型

```
Option: Type[T] = some(T) | none
```

**variantコンストラクタ**：

| variant | 構文 | 説明 |
|------|------|------|
| `some(T)` | `some(value)` | 値あり |
| `none` | `none` | 値なし |

### 9.3 エラー伝播

```
ErrorPropagate ::= Expr '?'
```

`?`演算子はResult型のエラーを自動的に伝播します：

```
// 成功時は値を返し、失敗時はerrを上に返す
data = fetch_data()?

// 以下と同等
data = match fetch_data() {
    ok(v) => v
    err(e) => return err(e)
}
```

---

## 付録A：構文早見表

### A.1 型定義

```
// === record type（波括弧） ===

// 構造体
Point: Type = { x: Float, y: Float }

// 列挙型（variant型）
Result: Type[T, E] = { ok(T) | err(E) }
Status: Type = { pending | processing | completed }

// === インターフェース型（波括弧、フィールドがすべて関数） ===

// インターフェース定義
Serializable: Type = { serialize: () -> String }

// インターフェースを実装する型
Point: Type = {
    x: Float,
    y: Float,
    Serializable    # Serializableインターフェースを実装
}

// === 関数型 ===

Adder: Type = (Int, Int) -> Int
```

### A.2 関数定義

```
// 形式1：型集中式（推薦）
name: (param1: Type1, param2: Type2) -> ReturnType = body

// 形式2：省略式（パラメータ名省略）
name: (Type1, Type2) -> ReturnType = (params) => body

// ジェネリック関数
name: [T, R](param: T) -> R = body

// ジェネリックconstraint
name: [T: Clone + Add](a: T, b: T) -> T = body
```

### A.3 メソッド定義

```
// 型メソッド
Type.method: (self: Type, ...) -> ReturnType = { ... }

// 通常メソッド
name: (Type, ...) -> ReturnType = { ... }
```

### A.4 メソッドバインディング

```
// 単位置バインディング
Type.method = func[0]

// 複数位置バインディング
Type.method = func[0, 1]

// pub自動バインディング
pub name: (Type, ...) -> ReturnType = { ... }  # Typeに自動バインディング
```

### A.5 ジェネリック構文

```
// ジェネリック型
List: Type[T] = { data: Array[T], length: Int }
Result: Type[T, E] = { ok(T) | err(E) }

// ジェネリック関数
map: [T, R](list: List[T], f: Fn(T) -> R) -> List[R] = { ... }

// 型constraint
clone: [T: Clone](value: T) -> T = value.clone()
combine: [T: Clone + Add](a: T, b: T) -> T = body

// 関連型
Iterator: Type[T] = { Item: T, next: () -> Option[T] }

// コンパイル時ジェネリック
factorial: [n: Int](n: n) -> Int = { ... }
StaticArray: Type[T, N: Int] = { data: T[N], length: N }

// 条件型
If: Type[C: Bool, T, E] = match C { True => T, False => E }

// 関数特殊化
sum: (arr: Array[Int]) -> Int = { ... }
sum: (arr: Array[Float]) -> Float = { ... }
```

### A.6 モジュール

```
// モジュールはファイル
// ファイル名.yxがモジュール名
Import ::= 'use' ModuleRef
```

### A.7 制御フロー

```
if Expr Block (elif Expr Block)* (else Block)?
match Expr { MatchArm+ }
while Identifier in Expr Block Expr Block
for
```

### A.8 match構文

```
match value {
    pattern1 => expr1,
    pattern2 if guard => expr2,
    _ => default_expr,
}
```

---

## 付録B：コード実装との違いの説明

> 本節では、言語仕様と現在のコード実装の間の既知の違いを説明します。

### B.1 キーワード

| キーワード | 仕様の状態 | コード実装 | 説明 |
|--------|---------|---------|------|
| `struct` | 削除済み | ❌ なし | 統一構文 `Name: Type = {...}` を使用 |
| `enum` | 削除済み | ❌ なし | variant構文 `Name: Type = { A \| B \| C }` を使用 |
| `type` | 削除済み | ❌ なし | meta typeキーワードとして `Type`（大文字）を使用 |

### B.2 構文の違い

| 構文要素 | 仕様 | コード実装 | 説明 |
|---------|------|---------|------|
| match arm 区切り文字 | `->` | `=>` | `=>`（FatArrow）を使用 |
| 関数定義 | `name(types) -> type = (params) => body` | 2つの形式 | 型集中式 `name: Type = (params) =>` をサポート |
| インターフェース型 | `type Serializable = [ serialize() -> String ]` | ❌ 未実装 | 角括弧構文は未実装 |

### B.3 未実装機能

以下の仕様で説明されている機能は、まだコードに実装されていません：

| 機能 | 優先度 | 説明 |
|------|--------|------|
| 統一型構文 `Name: Type = {...}` | P0 | RFC-010：統一構文で `type Name = ...` を置換 |
| 波括弧型構文 | P0 | `Point: Type = { x: Float, y: Float }` |
| インターフェース型 | P1 | `Drawable: Type = { draw() -> Void }` |
| リスト内包表記 | P2 | `[x for x in list if condition]` |
| `?` エラー伝播 | P1 | Result型の自動エラー伝播 |
| `ref` キーワード | P1 | Arc参照カウント共有 |
| `unsafe`コードブロック | P1 | 生ポインタとシステムレベルプログラミング |
| `*T` 生ポインタ型 | P1 | 生ポインタ型構文 |
| `clone()` 意味論 | P1 | 明示的コピー |
| `@block` 注釈 | P1 | 同期実行保証 |
| `spawn`関数 | P1 | 並作関数マーク |
| `spawn {}`ブロック | P1 | 明示的並行領域 |
| `spawn for`ループ | P1 | データ並列ループ |
| Send/Sync制約 | P2 | 並行安全型検査 |
| Mutex/Atomic型 | P2 | 並行安全データ型 |
| エラーグラフ可視化 | P3 | 並行エラー伝播追跡 |
| **ジェネリック型システム** | P1 | RFC-011 |
| 基本的なgenerics `[T]` | P1 | ジェネリック型パラメータと単態化 |
| 型constraint `[T: Clone]` | P2 | 単一/複数constraintシステム |
| 関連型 `Item: T` | P3 | GATサポート |
| コンパイル時generics `[N: Int]` | P3 | リテラル型constraint |
| 条件型 `If[C, T, E]` | P3 | 型レベル計算 |
| 関数オーバーロード特殊化 | P2 | プラットフォーム特殊化と型特殊化 |
| メソッド構文 `Type.method` | P1 | `Point.draw: (...) -> ... = ...` |

### B.4 非実装機能

以下のRustスタイル機能は**実装されません**：

| 機能 | 理由 |
|------|------|
| ライフタイム `'a` | 参照概念がないため、ライフタイム不要 |
| 借用検査器 | ref = Arcで代替 |
| `&T` 借用構文 | Move意味論を使用 |
| `&mut T` 可変借用 | mut + Moveを使用 |

---

## 第10章：メソッドバインディング

### 10.1 バインディング概要

YaoXiangは**純粋関数型設計**を採用しており、すべての操作は関数によって実装されます。バインディング機構は関数を型に関連付け、呼び出し元がまるでメソッドを呼び出すように関数を呼び出せるようにします。

```
バインディング宣言 ::= 型 '.' 識別子 '=' 関数名 '[' 位置リスト ']'
位置リスト ::= 位置 (',' 位置)* ','?
位置     ::= 整数（0から開始） | 負整数 | プレースホルダー
```

**コア規則**：
- 位置インデックスは**0**から開始
- デフォルトは**0**位（最初のパラメータ）にバインディング
- 負数インデックス `[-1]` は最後のパラメータをサポート
- 複数位置同時バインディング `[0, 1, 2]`
- プレースホルダー `_` はその位置をスキップ

### 10.2 バインディング構文

**構文**：
```
Type.method = func[position]
Type.method = func[0, 1, 2]    # 複数位置バインディング
Type.method = func[0, _, 2]   # プレースホルダーを使用
Type.method = func[-1]        # 負数インデックス（最後のパラメータ）
```

**意味論**：
- `Type.method = func[0]` は `obj.method(arg)` 呼び出し時、`obj`が`func`の0番目のパラメータにバインディングされることを意味する
- 残りのパラメータは元の順序で埋める

### 10.3 バインディング例

```yaoxiang
// === 基本的なバインディング ===

// 元の関数
distance: (a: Point, b: Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    (dx * dx + dy * dy).sqrt()
}

// Point型にバインディング（位置0）
Point.distance = distance[0]

// 使用
p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)
d = p1.distance(p2)  # → distance(p1, p2)

// === 複数位置バインディング ===

// 元の関数
calculate: (scale: Float, a: Point, b: Point, x: Float, y: Float) -> Float = ...

// 複数位置をバインディング
Point.calc_scale = calculate[0]      # scaleのみバインディング
Point.calc_both = calculate[1, 2]    # 2つのPointパラメータをバインディング

// 使用
f = p1.calc_scale(2.0)  # → calculate(2.0, p1, _, _, _)
result = f(p2, 10.0, 20.0)  # → calculate(2.0, p1, p2, 10.0, 20.0)

// === 柯里化（パラメータ不足時は自動的に関数を返す）===

// 1つのパラメータをバインディング
Point.distance_to = distance[0]

// 使用 - 2番目のパラメータを提供しない場合、柯里化関数を返す
f = p1.distance_to(p2)  # → distance(p1, p2) 直接呼び出し
f2 = p1.distance_to()   # → distance(p1, _) 関数 (Point) -> Float を返す
result = f2(p2)         # → distance(p1, p2)
```

### 10.4 バインディング規則

**位置規則**：
| 規則 | 説明 |
|------|------|
| 位置は0から開始 | `func[0]`は第1パラメータ（インデックス0）をバインディング |
| 最大位置 | 関数のパラメータ数より小さい必要がある |
| 負数インデックス | `[-1]`は最後のパラメータを意味する |
| プレースホルダー | `_`はその位置をスキップし、用户提供に委ねる |

**型検査**：
```yaoxiang
// ✅ 有効なバインディング
Point.distance = distance[0]          # distance(Point, Point)
Point.calc = calculate[1, 2]          # calculate(Float, Point, Point, ...)

// ❌ 無効なバインディング（コンパイルエラー）
Point.wrong = distance[5]             # 5 >= 2（パラメータ数）
Point.wrong = distance[0, 0]          # 位置重複（許可されていない場合）
Point.wrong = distance[-2]            # -2が範囲外
```

### 10.5 自動バインディング

モジュールで定義され最初のパラメータがモジュール型である関数については、メソッド呼び出し構文が自動サポートされます：

```yaoxiang
// === Point.yx ===
Point: Type = { x: Float, y: Float }

// 最初のパラメータがPointで、自動メソッド呼び出しサポート
distance: (a: Point, b: Point) -> Float = { ... }
add: (a: Point, b: Point) -> Point = { ... }

// === main.yx ===
use Point

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

// ✅ 自動バインディング：p1.distance(p2) → distance(p1, p2)
d = p1.distance(p2)
// ✅ p1.add(p2) → add(p1, p2)
p3 = p1.add(p2)
```

**自動バインディング規則**：
- 関数はモジュールファイルで定義されている
- 関数の0番目のパラメータ型がモジュール名と一致する
- 関数はモジュール外で自動バインディングするために`pub`でなければならない

### 10.6 バインディングと柯里化の関係

バインディングは自然に柯里化をサポートします。呼び出し時に提供されたパラメータが残りパラメータより少ない場合、残りのパラメータを受け取る関数を返します：

```yaoxiang
// 元の関数：5つのパラメータ
calculate: (scale: Float, a: Point, b: Point, x: Float, y: Float) -> Float = ...

// バインディング：Point.calc = calculate[1, 2]
// バインディング後の残りパラメータ：scale, x, y

// 呼び出しシナリオ
p1.calc(2.0, 10.0, 20.0)              # 3つのパラメータを提供 → 直接呼び出し
p1.calc(2.0)                          # 1つのパラメータを提供 → (Float, Float) -> Floatを返す
p1.calc()                             # 0個のパラメータを提供 → (Float, Float, Float) -> Floatを返す
```

---

## 付録C：バインディング構文早見表

### C.1 バインディング宣言

```
// 単位置バインディング（デフォルトで位置0にバインディング）
Type.method = func[0]

// 複数位置バインディング
Type.method = func[0, 1, 2]

// プレースホルダーを使用
Type.method = func[0, _, 2]

// 負数インデックス（最後のパラメータ）
Type.method = func[-1]
```

### C.2 位置インデックス説明

```
関数パラメータ：    (p0, p1, p2, p3, p4)
              ↑  ↑  ↑  ↑  ↑
インデックス：        0  1  2  3  4

// バインディング [1, 3]
Type.method = func[1, 3]
// 呼び出し：obj.method(p0, p2, p4)
// マッピング：func(p0_bound, obj, p2, p3_bound, p4)
```

### C.3 呼び出し形式

```yaoxiang
// 直接呼び出し（すべての残りパラメータを提供）
result = p.method(arg1, arg2, arg3)

// 柯里化（残りパラメータを提供しないか部分的に提供）
f = p.method(arg1)          # 残りパラメータを受け取る関数を返す
result = f(arg2, arg3)
```

---

## バージョン履歴

| バージョン | 日付 | 著者 | 変更説明 |
|------|------|------|---------|
| v1.0.0 | 2024-12-31 | 晨煦 | 初期バージョン |
| v1.1.0 | 2025-01-04 | 沫郁酱 | match armが`=>`而不是`->`を使用するよう修正；関数定義構文を更新；型定義構文を更新；コード実装との違いの説明を追加 |
| v1.2.0 | 2025-01-05 | 沫郁酱 | 純粋仕様に精简化し、サンプルコードをtutorial/ディレクトリに移動 |
| v1.3.0 | 2025-01-05 | 沫郁酱 | 並作モデル仕様を追加（3層並行アーキテクチャ、spawn構文、注釈）；型システム制約を追加（Send/Sync）；並行安全型を追加（Mutex、Atomic）；エラー処理を更新（？演算子）；未実装機能リストを更新 |
| v1.4.0 | 2025-01-15 | 晨煦 | 所有権モデルを更新（デフォルトMove + 明示的ref=Arc）；unsafeキーワードを追加；ライフタイム`'a`と借用検査器を削除；未実装機能リストを更新 |
| v1.5.0 | 2025-01-20 | 晨煦 | メソッドバインディング仕様を追加（RFC-004）：位置インデックスは0から開始；デフォルトで位置0にバインディング；負数インデックスと複数位置バインディングをサポート |
| v1.6.0 | 2025-02-06 | 晨煦 | RFC-010（統一型構文）を統合：`type Name = {...}`構文を更新；シグネチャ内のパラメータ名；Type.methodメソッド構文；RFC-011（ジェネリックシステム）を統合：ジェネリック型`[T]`、型constraint`[T: Clone]`、関連型`Item: T`、コンパイル時generics`[N: Int]`、条件型`If[C, T, E]`、関数オーバーロード特殊化、プラットフォーム特殊化を追加 |
| v1.7.0 | 2026-02-13 | 晨煦 | RFC-010 更新：`Name: Type = {...}`で`type Name = {...}`を置換；`Type`（大文字）のみがmeta typeキーワード；すべての宣言が統一構文を使用 |
| v1.8.0 | 2026-02-18 | 晨煦 | RFC-010 新規：デフォルト値初期化、組み込みバインディング構文；RFC-004 新規：組み込みバインディング、無名関数バインディング |
| v1.8.1 | 2026-02-20 | 晨煦 | meta typeはキーワードではない。 |

---

> 本仕様はYaoXiangプログラミング言語のコア構文と意味論を定義しています。
> チュートリアルとサンプルコードについては、[YaoXiangガイド](../guide/YaoXiang-book.md)および[tutorial/](../tutorial/)ディレクトリを参照してください。
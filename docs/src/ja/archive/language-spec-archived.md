> **注意：本文檔已存檔，不再維護。**
> **請參考新的語言規範文檔：[語言規範](../reference/language-spec/index.md)**

---

# YaoXiang（爻象）プログラミング言語仕様

> バージョン：v1.8.0
> 状態：仕様
> 著者：晨煦
> 日付：2024-12-31
> 更新：2026-02-22 - メタ型はキーワードではない。

---

## 第1章：はじめに

### 1.1 範囲

このドキュメントはYaoXiangプログラミング言語の構文と意味論を定義ものである。これは言語の権威あるリファレンスであり、コンパイラおよびツールの実装者を対象としている。

チュートリアルとサンプルコードについては、[YaoXiang ガイド](../guide/YaoXiang-book.md)および[tutorial/](../tutorial/)ディレクトリを参照のこと。

### 1.2 適合性

プログラムまたは実装が本ドキュメントで定義されたすべての規則を満たす場合、YaoXiang仕様に適合するとみなされる。

---

## 第2章：字句構造

### 2.1 ソースファイル

YaoXiangソースファイルはUTF-8エンコーディングを使用しなければならない。ソースファイルの拡張子は通常`.yx`である。

### 2.2 トークン分類

| 分類 | 説明 | 例 |
|------|------|------|
| 識別子 | 文字またはアンダースコアで始まる | `x`, `_private`, `my_var` |
| キーワード | 言語によって予約された単語 | `Type`, `pub`, `use` |
| リテラル | 固定値 | `42`, `"hello"`, `true` |
| 演算子 | 演算記号 | `+`, `-`, `*`, `/` |
| 区切り文字 | 構文区切り | `(`, `)`, `{`, `}`, `,` |

### 2.3 キーワード

YaoXiangは極めて少数のキーワードを定義する：

```
pub    use    spawn
ref    mut    if     elif
else   match  while  for    return
break  continue as     in     unsafe
```

これらのキーワードはどのようなコンテキストでも特別な意味を持ち、識別子として使用できない。

### 2.4 予約語

| 予約語 | 型 | 説明 |
|--------|------|------|
| `Type` | Type | メタ型 |
| `true` | Bool | 真理値真 |
| `false` | Bool | 真理値偽 |
| `void` | Void | 空値 |
| `some(T)` | Option | Option 値variant |
| `ok(T)` | Result | Result 成功variant |
| `err(E)` | Result | Result エラーvariant |

### 2.5 識別子

識別子は文字またはアンダースコアで始まり、後続の文字は文字、数字、またはアンダースコアにできる。識別子は大文字小文字を区別する。

特殊識別子：
- `_` はプレースホルダーとして使用され、ある値を無視することを示す
- アンダースコアで始まる識別子はプライベートメンバーを示す

### 2.6 リテラル

#### 2.6.1 整数

```
Decimal     ::= [0-9][0-9_]*
Octal       ::= 0o[0-7][0-7_]*
Hex         ::= 0x[0-9a-fA-F][0-9a-fA-F_]*
Binary      ::= 0b[01][01_]*
```

#### 2.6.2 浮動小数点

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
   複数行にまたかれる */
```

### 2.8 インデント規則

コードは4つのスペースを使用してインデントしなければならず、タブ文字の使用は禁止。これは必須の構文規則である。

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

### 3.2 プリミティブ型

| 型 | 説明 | デフォルトサイズ |
|------|------|----------|
| `Type` | メタ型 | 0 バイト |
| `Void` | 空値 | 0 バイト |
| `Bool` | 真理値 | 1 バイト |
| `Int` | 符号付き整数 | 8 バイト |
| `Uint` | 符号なし整数 | 8 バイト |
| `Float` | 浮動小数点 | 8 バイト |
| `String` | UTF-8 文字列 | 可変 |
| `Char` | Unicode 文字 | 4 バイト |
| `Bytes` | 生バイト | 可変 |

ビット幅付き整数：`Int8`, `Int16`, `Int32`, `Int64`, `Int128`
ビット幅付き浮動小数点：`Float32`, `Float64`

### 3.3 レコード型

**統一構文**：`Name: Type = { field1: Type1, field2: Type2, ... }`

```
RecordType  ::= '{' FieldList? '}'
FieldList   ::= Field (',' Field)* ','?
Field       ::= Identifier ':' TypeExpr
            |  Identifier                 # インターフェース制約
```

```yaoxiang
// 単純なレコード型
Point: Type = { x: Float, y: Float }

// 空レコード型
Empty: Type = {}

// ジェネリクス付きレコード型
Pair: Type[T] = { first: T, second: T }

// インターフェースを実装するレコード型
Point: Type = {
    x: Float,
    y: Float,
    Drawable,
    Serializable
}
```

**規則**：
- レコード型は波括弧`{}`を使用して定義する
- フィールド名の直後にコロンと型を続ける
- インターフェース名は型体内でそのインターフェースを実装することを示す

#### 3.3.1 フィールドデフォルト値

型フィールドにはデフォルト値を指定でき、構築時に省略可能：

```yaoxiang
// デフォルト値を持つフィールド - 構築時に省略可能
Point: Type = {
    x: Float = 0,
    y: Float = 0
}

// 使用
Point()           # → Point(x=0, y=0)
Point(x=1)       # → Point(x=1, y=0)
Point(x=1, y=2) # → Point(x=1, y=2)

// デフォルト値を持たないフィールド - 構築時に必須
Point2: Type = {
    x: Float,
    y: Float
}

// 使用
Point2(x=1, y=2) # ✓
Point2()          # ✗ エラー
```

**規則**：
- `field: Type = expression` → デフォルト値あり、構築時に省略可能
- `field: Type` → デフォルト値なし、構築時に必須

#### 3.3.2 組み込みバインディング

型定義体内で直接メソッドをバインディングできる：

```yaoxiang
// 方式1：外部関数を参照してバインディング
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]    # 位置0にバインディング
}
// 呼び出し：p1.distance(p2) → distance(p1, p2)

// 方式2：無名関数 + 位置バインディング
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
// パラメータなしvariant
Color: Type = { red | green | blue }

// パラメータ付きvariant
Option: Type[T] = { some(T) | none }

// 混合
Result: Type[T, E] = { ok(T) | err(E) }

// パラメータなしvariantはパラメータなしコンストラクタと等価
Bool: Type = { true | false }
```

### 3.5 インターフェース型

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

**インターフェース実装**：型は定義の末尾にインターフェース名を列出することでインターフェースを実装する

```yaoxiang
// インターフェースを実装する型
Point: Type = {
    x: Float,
    y: Float,
    Drawable,        # Drawable インターフェースを実装
    Serializable     # Serializable インターフェースを実装
}
```

**インターフェースへの直接代入**：具体型はインターフェース型変数に直接代入可能（構造的サブタイピング）

```yaoxiang
// 直接代入（コンパイル時に具体型が確定 → ゼロオーバーヘッド呼び出し）
d: Drawable = Circle(1)
d.draw(screen)        # コンパイル後：vtableなしでcircle_drawを直接呼び出し

// 関数戻り値（コンパイル時に確定できない → vtable呼び出し）
d: Drawable = get_shape()
d.draw(screen)        # vtableでメソッドを検索

// インターフェースを関数パラメータとして使用
process: (d: Drawable) -> Void = d.draw(screen)
```

**コンパイル時最適化戦略**：

| シナリオ | 推論結果 | 呼び出し方式 |
|------|----------|------|
| 具体型を直接代入 | 具体型が確定 | 直接呼び出し（ゼロオーバーヘッド） |
| 関数戻り値 | 不明 | vtable |
| 不均一コレクション | 複数の型 | vtable |

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

### 3.8 ジェネリック型

#### 3.8.1 ジェネリック引数構文

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
// 基本ジェネリック型
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
// コンパイラがジェネリック引数を自動推論
numbers: List[Int] = List(1, 2, 3)  # コンパイラがList[Int]を推論
```

### 3.9 型制約

#### 3.9.1 単一制約

```
ConstrainedType ::= '[' Identifier ':' TypeBound ']' TypeExpr
```

```yaoxiang
// インターフェース型定義（制約として）
Clone: Type = {
    clone: (Self) -> Self
}

// 制約を使用
clone: [T: Clone](value: T) -> T = value.clone()
```

#### 3.9.2 複数制約

```yaoxiang
// 複数制約構文
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

#### 3.9.3 関数型制約

```yaoxiang
// 高階関数制約
call_twice: [T, F: Fn() -> T](f: F) -> (T, T) = (f(), f())

compose: [A, B, C, F: Fn(A) -> B, G: Fn(B) -> C](a: A, f: F, g: G) -> C = g(f(a))
```

### 3.10 関連型

#### 3.10.1 関連型定義

```
AssociatedType ::= Identifier ':' TypeExpr
```

```yaoxiang
// Iterator トレイト（レコード型構文を使用）
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

**コア設計**：`[n: Int]` ジェネリック引数 + `(n: n)` 値引数を使用し、コンパイル時定数とランタイム値を区別する。

```yaoxiang
// コンパイル時階乗：引数はコンパイル時に既知のリテラルでなければならない
factorial: [n: Int](n: n) -> Int = {
    match n {
        0 => 1,
        _ => n * factorial(n - 1)
    }
}

// コンパイル時定数配列
StaticArray: Type[T, N: Int] = {
    data: T[N],      # コンパイル時にサイズが確定している配列
    length: N
}

// 使用方法
arr: StaticArray[Int, factorial(5)]  # コンパイラがコンパイル時にfactorial(5) = 120を計算
```

#### 3.11.2 コンパイル時定数配列

```yaoxiang
// 行列型の使用
Matrix: Type[T, Rows: Int, Cols: Int] = {
    data: Array[Array[T, Cols], Rows]
}

// コンパイル時次元検証
identity_matrix: [T: Add + Zero + One, N: Int](size: N) -> Matrix[T, N, N] = {
    // ...
}
```

### 3.12 条件型

#### 3.12.1 If 条件型

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

### 3.13 型合併

```
TypeUnion     ::= TypeExpr '|' TypeExpr
```

### 3.14 型交差

```
TypeIntersection ::= TypeExpr '&' TypeExpr
```

**構文**：型交差 `A & B` はAとBの両方を同時に満たす型を表す

```yaoxiang
// インターフェース合成 = 型交差
DrawableSerializable: Type = Drawable & Serializable

// 交差型を使用
process: [T: Drawable & Serializable](item: T, screen: Surface) -> String = {
    item.draw(screen)
    return item.serialize()
}
```

### 3.15 関数オーバーロードと特殊化

```yaoxiang
// 基本特殊化：関数オーバーロードを使用（コンパイラが自動選択）
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
// プラットフォーム型列挙型（標準ライブラリで定義）
Platform: Type = X86_64 | AArch64 | RISC_V | ARM | X86

// Pは現在のコンパイルプラットフォームを表す事前定義ジェネリック引数名
sum: [P: X86_64](arr: Array[Float]) -> Float = {
    return avx2_sum(arr.data, arr.length)
}

sum: [P: AArch64](arr: Array[Float]) -> Float = {
    return neon_sum(arr.data, arr.length)
}
```

---

## 第3章（続）：構文設計の説明

### 3.17 名前付き関数とLambdaの関係

**コア理解**：名前付き関数とLambda式は同じものである！唯一のの違いは、名前付き関数がLambdaに名前を付けたものである。

```yaoxiang
// この2つは本質的に完全に同じ
add: (a: Int, b: Int) -> Int = a + b           # 名前付き関数（推奨）
add: (a: Int, b: Int) -> Int = (a, b) => a + b  # Lambda形式（完全に同等）
```

**糖衣構文モデル**：

```
// 名前付き関数 = Lambda + 名前
name: (Params) -> ReturnType = body

// 本質的には
name: (Params) -> ReturnType = (params) => body
```

**重要ポイント**：シグネチャがパラメータ型を完全に宣言している場合、Lambdaヘッダのパラメータ名は冗長になり、省略可能。

### 3.18 パラメータスコープ規則

**シグネチャ内のパラメータが外側の変数をシャドウ**：シグネチャ内のパラメータスコープは関数本体を覆い、内部スコープが優先順位が高い。

```yaoxiang
x = 10  # 外側変数
double: (x: Int) -> Int = x * 2  # ✅ パラメータxが外側のxをシャドウ、結果は20
```

### 3.19 型注釈の位置

型注釈は以下のいずれかの位置に配置でき、**少なくとも1箇所に注釈をつければよい**：

| 注釈位置 | 形式 | 説明 |
|----------|------|------|
| シグネチャのみ | `double: (x: Int) -> Int = x * 2` | ✅ 推奨 |
| Lambdaヘッダのみ | `double = (x: Int) => x * 2` | ✅ 有効 |
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

### 4.2 演算子優先順位

| 優先順位 | 演算子 | 結合性 |
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

### 4.8 パターンマッチング

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

#### 5.9.1 意味論：各反復は新しい値のバインディング

YaoXiangのforループの意味論は従来の言語と異なる：**各反復は新しい値のバインディングであり、同じ変数を変更するわけではない**。

```yaoxiang
// 例：for i in 1..5
for i in 1..5 {
    print(i)
}
```

**実行過程**：

| 反復 | ループ変数の動作 |
|------|----------------|
| 1回目 | 新しいバインディング`i = 1`を作成、ループ本体を実行、1を出力 |
| 2回目 | 新しいバインディング`i = 2`を作成（以前のバインディングは破棄）、ループ本体を実行、2を出力 |
| 3回目 | 新しいバインディング`i = 3`を作成、ループ本体を実行、3を出力 |
| 4回目 | 新しいバインディング`i = 4`を作成、ループ本体を実行、4を出力 |
| 終了 | ループ本体終了、バインディング破棄 |

**重要ポイント**：各反復の終了後、その反復で作成されたバインディングは破棄される。次に反復は前回の反復のバインディングとは全く関係ない新しいバインディングである。

#### 5.9.2 for と for mut の違い

| 構文 | ループ変数の可変性 | 説明 |
|------|----------------|------|
| `for i in 1..5` | 不変 | ループ本体でバインディングを変更できない |
| `for mut i in 1..5` | 可変 | ループ本体でバインディングを変更できる |

```yaoxiang
// ✅ 有効：各反復で新しい値をバインディングするため、変更は不要
for i in 1..5 {
    print(i)  # iの値を読み取る
}

// ❌ 誤り：不変バインディングは変更できない
for i in 1..5 {
    i = i + 1  # エラー：不変バインディングは変更できない
}

// ✅ 有効：for mutを使用するとバインディングを変更できる
for mut i in 1..5 {
    i = i + 1  # 変更が許可される
}
```

#### 5.9.3 シャドウチェック

forループ変数は外側のスコープに既に存在する変数をシャドウできない：

```yaoxiang
// ❌ 誤り：iは外部で既に宣言されている
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
| YaoXiang | 各反復で新しい値をバインディング |
| Rust | 同じ変数を変更（mutが必要） |
| Python | 同じ変数を変更（mut不要） |
| C/C++ | 同じ変数を変更（ポインタまたは参照が必要） |

**設計理由**：YaoXiangがバインディング意味論を採用したのは：
1. 各反復の終了後、ループ本体内の変数は破棄される
2. 次に反復は完全に新しいバインディングである
3. これによりより安全になり、反復間の状態を考える必要がない

---

## 第6章：関数

### 6.1 統一関数モデル

**コア構文**：`name: type = value`

YaoXiangは**統一宣言モデル**を採用する：変数、関数、メソッドはすべて同じ形式`name: type = value`を使用する。

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
y = 100  # Intと推論される
```

### 6.3 関数定義

#### 6.3.1 完全構文

```yaoxiang
// シグネチャでパラメータ名を宣言
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
// Void以外の戻り値型 - returnを使用する必要がある
add: (a: Int, b: Int) -> Int = {
    return a + b
}

// Void戻り値型 - returnの使用は省略可能
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

// ジェネリック制約を使用
clone: [T: Clone](value: T) -> T = value.clone()

// 複数型引数
combine: [T, U](a: T, b: U) -> (T, U) = (a, b)
```

### 6.5 メソッド定義

#### 6.5.1 型メソッド

**構文**：`Type.method: (self: Type, ...) -> Return = ...`

```yaoxiang
// 型メソッド：特定の型に関連付けられる
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

#### 6.5.2 通常のメソッド

**構文**：`name: (Type, ...) -> Return = ...`（型に関連付けない）

```yaoxiang
// 通常メソッド：型に関連付けず、独立した関数として
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

#### 6.6.2 pub 自動バインディング

`pub`で宣言された関数について、コンパイラは同ファイルで定義された型に自動バインディングする：

```yaoxiang
// pubで宣言し、コンパイラが自動バインディング
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

// コンパイラが自動推論：
// 1. Pointが現在のファイルで定義されている
// 2. 関数パラメータにPointが含まれる
// 3. Point.distance = distance[0]を実行

// 呼び出し
d = distance(p1, p2)           # 関数型
d2 = p1.distance(p2)           # OOP糖衣構文
```

### 6.7 メソッドバインディング規則

| 規則 | 説明 |
|------|------|
| 位置は0から開始 | `func[0]`は最初のパラメータ（インデックス0）をバインディング |
| 最大位置 | 関数のパラメータ数未満でなければならない |
| 負数インデックス | `[-1]`は最後尾のパラメータを示す |
| プレースホルダー | `_`はその位置をスキップし、ユーザーが提供する |

### 6.8 。カリー化サポート

バインディングは自然にカリー化をサポート。呼び出し時に提供された引数が残りの引数より少ない場合、残りのパラメータを受け取る関数を返す：

```yaoxiang
// 元の関数：5つのパラメータ
calculate: (scale: Float, a: Point, b: Point, x: Float, y: Float) -> Float = ...

// バインディング：Point.calc = calculate[1, 2]
// バインディング後の残りのパラメータ：scale, x, y

// 呼び出しシナリオ
p1.calc(2.0, 10.0, 20.0)       # 3つの引数を提供 → 直接呼び出し
p1.calc(2.0)                    # 1つの引数を提供 → (Float, Float) -> Floatを返す
p1.calc()                       # 0個の引数を提供 → (Float, Float, Float) -> Floatを返す
```

### 6.9 並作関数と注解

#### 6.9.1 spawn関数（並行関数）

```
SpawnFn     ::= Identifier ':' FnType 'spawn' '=' Lambda
FnType      ::= '(' ParamTypes? ')' '->' TypeExpr ('@' Annotation)?
Annotation  ::= 'block' | 'eager'
```

**関数注解**：

| 注解 | 位置 | 動作 |
|------|------|------|
| `@block` | 戻り値型の後 | 並行最適化を無効化、完全な逐次実行 |
| `@eager` | 戻り値型の後 | 先行評価を強制 |

**構文例**：

```
// 並行関数：並行実行可能
fetch_data: (url: String) -> JSON spawn = { ... }

// @block 同期関数：完全逐次実行
main: () -> Void @block = { ... }

// @eager 先行関数：直ちに実行
compute: (n: Int) -> Int @eager = { ... }
```

#### 6.9.2 spawnブロック

明示的に宣言された並行領域、ブロック内のタスクは並行実行される：

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
```

**例**：

```
// 並行ブロック：明示的並行
(result_a, result_b) = spawn {
    parse(fetch("url1")),
    parse(fetch("url2"))
}
```

#### 6.9.3 spawnループ

データ並列ループ、ループ本体がすべてのデータ要素で並行実行される：

```
SpawnFor    ::= Identifier '=' 'spawn' 'for' Identifier 'in' Expr '{' Expr '}'
```

**例**：

```
// 並行ループ：データ並列
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
    data = fetch_data()?      # 自動的にエラーを伝播
    transform(data)?
}
```

---

## 第7章：モジュール

### 7.1 モジュール定義

モジュールはファイルを境界として使用する。各`.yx`ファイルは1つのモジュールである。

```
// ファイル名がモジュール名となる
// Math.yx
pub pi: Float = 3.14159
pub sqrt(Float) -> Float = (x) => { ... }
```

### 7.2 モジュールインポート

```
Import       ::= 'use' ModuleRef ImportSpec?
ImportSpec   ::= ('{' ImportItems '}') ('as' AliasList)?
              |  'as' AliasList
ImportItems  ::= Identifier (',' Identifier)* ','?
AliasList    ::= Identifier (',' Identifier)*
```

| 構文 | 説明 | 例 |
|------|------|------|
| `use path;` | モジュールをインポートし、最後の部分でアクセス | `use std.io;` → `io.print` |
| `use path.{a, b};` | 指定項目をインポート | `use std.io.{print};` → `print` |
| `use path as alias;` | インポートして名前変更 | `use std.io as io;` → `io.print` |
| `use path.{i1, i2} as a, b;` | 指定項目をインポートして名前変更 | `use std.io.{print, read} as p, r;` → `p`, `r` |

---

## 第8章：メモリ管理

### 8.1 所有権モデル

YaoXiangは**所有権モデル**を使用してメモリを管理し、各値には所有者が1人だけ存在する：

| 意味論 | 説明 | 構文 |
|------|------|------|
| **Move** | デフォルト意味論、所有権转移 | `p2 = p` |
| **ref** | 共有（Arc参照カウント） | `shared = ref p` |
| **clone()** | 明示的コピー | `p2 = p.clone()` |

### 8.2 Move意味論（デフォルト）

```yaoxiang
// 代入 = Move（ゼロコピー）
p: Point = Point(1.0, 2.0)
p2 = p              # Move、pは無効化

// 関数引数 = Move
process: (p: Point) -> Void = {
    // pの所有権が转移
}

// 戻り値 = Move
create: () -> Point = {
    p = Point(1.0, 2.0)
    return p        # Move、所有権转移
}
```

### 8.3 refキーワード（Arc）

`ref`キーワードは**参照カウントポインタ**（Arc）を作成し、安全な共有に使用する：

```yaoxiang
// Arcを作成
p: Point = Point(1.0, 2.0)
shared = ref p      # Arc、スレッド安全

// 共有アクセス
spawn(() => print(shared.x))   # ✅ 安全

// Arcは自動的にライフサイクルを管理
// sharedがスコープを離れると、カウントがゼロになり自動解放
```

**特徴**：
- スレッド安全参照カウント
- 自動ライフサイクル管理
- spawn境界を越えて安全

### 8.4 clone() 明示的コピー

```yaoxiang
// 明示的に値をコピー
p: Point = Point(1.0, 2.0)
p2 = p.clone()      # pとp2は独立

// どちらも変更可能で、互いに影響しない
p.x = 0.0           # ✅
p2.x = 0.0          # ✅
```

### 8.5 unsafeコードブロック

`unsafe`コードブロックはベアポインタの使用を許可し、システムレベルのプログラミングに使用する：

```yaoxiang
// ベアポインタ型
PtrType ::= '*' TypeExpr

// unsafeコードブロック
UnsafeBlock ::= 'unsafe' '{' Stmt* '}'
```

**例**：

```yaoxiang
p: Point = Point(1.0, 2.0)

// ベアポインタはunsafeブロック内でのみ使用可能
unsafe {
    ptr: *Point = &p     # ベアポインタを取得
    (*ptr).x = 0.0       # 逆参照
}
```

**制限**：
- ベアポインタは`unsafe`ブロック内でのみ使用可能
- ユーザーはダングリング、使用後の解放がないことを保証
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

// === ベアポインタ（unsafeのみ） ===

PtrType       ::= '*' TypeExpr
UnsafeBlock   ::= 'unsafe' '{' Stmt* '}'
```

### 8.8 Send / Sync制約

| 制約 | 意味論 | 説明 |
|------|------|------|
| **Send** | スレッド間を安全に传输可能 | 値を別のスレッドに移動可能 |
| **Sync** | スレッド間を安全に共有可能 | 不変参照を別のスレッドに共有可能 |

**自動導出**：

```
// Send導出規則
Struct[T1, T2]: Send ⇐ T1: Send 且つ T2: Send

// Sync導出規則
Struct[T1, T2]: Sync ⇐ T1: Sync 且つ T2: Sync
```

**型制約**：

| 型 | Send | Sync | 説明 |
|------|------|------|------|
| `T`（値） | ✅ | ✅ | 不変データ |
| `ref T` | ✅ | ✅ | Arcスレッド安全 |
| `*T` | ❌ | ❌ | ベアポインタ不安全 |

---

## 第8章（続）：型システム制約

### 8.7 Send/Sync制約

YaoXiangはRustスタイルの型制約を使用して並行安全を保証する：

| 制約 | 意味論 | 説明 |
|------|------|------|
| **Send** | スレッド間を安全に传输可能 | 値を別のスレッドに移動可能 |
| **Sync** | スレッド間を安全に共有可能 | 不変参照を別のスレッドに共有可能 |

**制約階層**：

```
Send ──► スレッド間を安全に传输可能
  │
  └──► Sync ──► スレッド間を安全に共有可能
       │
       └──► Send + Syncを満たす型は自動的に並行可能

Arc[T]はSend + Syncを実装（スレッド安全参照カウント）
Mutex[T]は内部可変性を提供
```

### 8.8 並行安全型

| 型 | 意味論 | 並行安全 | 説明 |
|------|------|----------|------|
| `T` | 不変データ | ✅ 安全 | デフォルト型、マルチタスク読み取り競合なし |
| `Ref[T]` | 可変参照 | ⚠️ 同期が必要 | 並行変更可能、マーク、ロック使用のコンパイル時チェック |
| `Atomic[T]` | 原子型 | ✅ 安全 | 基盤原子操作、ロックフリー並行 |
| `Mutex[T]` | 相互排除ロック包装 | ✅ 安全 | 自動ロック解除、コンパイル時保証 |
| `RwLock[T]` | 読取書込ロック包装 | ✅ 安全 | 読み取り多く書き込み少ないシナリオ最適化 |

**構文**：

```
Mutex[T]    # 相互排除ロック包装の可変データ
Atomic[T]   # 原子型（Int、Floatなど専用）
RwLock[T]   # 読取書込ロック包装
```

**with糖衣構文**：

```
with mutex.lock() {
    // 臨界区間：Mutexで保護
    ...
}
```

---

## 第9章：错误処理

### 9.1 Result型

```
Result: Type[T, E] = ok(T) | err(E)
```

**variant構築**：

| variant | 構文 | 説明 |
|------|------|------|
| `ok(T)` | `ok(value)` | 成功値 |
| `err(E)` | `err(error)` | エラー値 |

### 9.2 Option型

```
Option: Type[T] = some(T) | none
```

**variant構築**：

| variant | 構文 | 説明 |
|------|------|------|
| `some(T)` | `some(value)` | 値あり |
| `none` | `none` | 値なし |

### 9.3 エラー伝播

```
ErrorPropagate ::= Expr '?'
```

`?`演算子はResult型のエラーを自動的に伝播する：

```
// 成功時は値を返し、失敗時は上方向にerrを返す
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
// === レコード型（波括弧） ===

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
    Serializable    # Serializable インターフェースを実装
}

// === 関数型 ===

Adder: Type = (Int, Int) -> Int
```

### A.2 関数定義

```
// 形式1：型集中式（推奨）
name: (param1: Type1, param2: Type2) -> ReturnType = body

// 形式2：省略式（パラメータ名省略）
name: (Type1, Type2) -> ReturnType = (params) => body

// ジェネリック関数
name: [T, R](param: T) -> R = body

// ジェネリック制約
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

// 複数置バインディング
Type.method = func[0, 1]

// pub自動バインディング
pub name: (Type, ...) -> ReturnType = { ... }  # Typeに自動バインディング
```

### A.5 ジェネリクス構文

```
// ジェネリック型
List: Type[T] = { data: Array[T], length: Int }
Result: Type[T, E] = { ok(T) | err(E) }

// ジェネリック関数
map: [T, R](list: List[T], f: Fn(T) -> R) -> List[R] = { ... }

// 型制約
clone: [T: Clone](value: T) -> T = value.clone()
combine: [T: Clone + Add](a: T, b: T) -> T = body

// 関連型
Iterator: Type[T] = { Item: T, next: () -> Option[T] }

// コンパイル時ジェネリクス
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

> 本節では、言語仕様と現在のコード実装の間の既知の違いを説明する。

### B.1 キーワード

| キーワード | 仕様状態 | コード実装 | 説明 |
|--------|---------|---------|------|
| `struct` | 削除済み | ❌ なし | 統一構文`Name: Type = {...}`を使用 |
| `enum` | 削除済み | ❌ なし | variant構文`Name: Type = { A \| B \| C }`を使用 |
| `type` | 削除済み | ❌ なし | メタ型キーワードとして`Type`（大文字）を使用 |

### B.2 構文の違い

| 構文要素 | 仕様 | コード実装 | 説明 |
|---------|------|---------|------|
| match arm区切り | `->` | `=>` | `=>`（FatArrow）を使用 |
| 関数定義 | `name(types) -> type = (params) => body` | 2つの形式 | 型集中式`name: Type = (params) =>`をサポート |
| インターフェース型 | `type Serializable = [ serialize() -> String ]` | ❌ 未実装 | 角括弧構文は未実装 |

### B.3 未実装機能

以下の仕様で説明されている機能は、まだコードに実装されていない：

| 機能 | 優先度 | 説明 |
|------|--------|------|
| 統一型構文`Name: Type = {...}` | P0 | RFC-010：統一構文で`type Name = ...`を置き換え |
| 波括弧型構文 | P0 | `Point: Type = { x: Float, y: Float }` |
| インターフェース型 | P1 | `Drawable: Type = { draw() -> Void }` |
| リスト内包表記 | P2 | `[x for x in list if condition]` |
| `?`エラー伝播 | P1 | Result型の自動エラー伝播 |
| `ref`キーワード | P1 | Arc参照カウント共有 |
| `unsafe`コードブロック | P1 | ベアポインタとシステムレベルプログラミング |
| `*T`ベアポインタ型 | P1 | ベアポインタ型構文 |
| `clone()`意味論 | P1 | 明示的コピー |
| `@block`注解 | P1 | 同期実行保証 |
| `spawn`関数 | P1 | 並行関数マーク |
| `spawn {}`ブロック | P1 | 明示的並行領域 |
| `spawn for`ループ | P1 | データ並列ループ |
| Send/Sync制約 | P2 | 並行安全型チェック |
| Mutex/Atomic型 | P2 | 並行安全データ型 |
| エラーグラフ可視化 | P3 | 並行エラー伝播追跡 |
| **ジェネリック型システム** | P1 | RFC-011 |
| 基本ジェネリクス`[T]` | P1 | ジェネリック型引数と単態化 |
| 型制約`[T: Clone]` | P2 | 単一/複数制約システム |
| 関連型`Item: T` | P3 | GATサポート |
| コンパイル時ジェネリクス`[N: Int]` | P3 | リテラル型制約 |
| 条件型`If[C, T, E]` | P3 | 型レベル計算 |
| 関数オーバーロード特殊化 | P2 | プラットフォーム特殊化と型特殊化 |
| メソッド構文`Type.method` | P1 | `Point.draw: (...) -> ... = ...` |

### B.4 非実装機能

以下のRustスタイルの機能は**実装しない**：

| 機能 | 理由 |
|------|------|
| ライフタイム`'a` | 参照概念がないため、ライフタイム不要 |
| 借用チェッカー | ref = Arcで代替 |
| `&T`借用構文 | Move意味論を使用 |
| `&mut T`可変借用 | mut + Moveを使用 |

---

## 第10章：メソッドバインディング

### 10.1 バインディング概要

YaoXiangは**純粋関数型設計**を採用し、すべての操作は関数で実装される。バインディング機構は関数を型に関連付け、呼び出し元がメソッドを呼び出すかのように関数を呼び出せるようにする。

```
バインディング宣言 ::= 型 '.' 識別子 '=' 関数名 '[' 位置リスト ']'
位置リスト ::= 位置 (',' 位置)* ','?
位置     ::= 整数（0から開始） | 負整数 | プレースホルダー
```

**コア規則**：
- 位置インデックスは**0**から開始
- デフォルトは**0**位（最初のパラメータ）にバインディング
- 負数インデックス`[-1]`サポートは最後尾のパラメータを示す
- 複数位置聯合バインディング`[0, 1, 2]`サポート
- プレースホルダー`_`はその位置をスキップすることを示す

### 10.2 バインディング構文

**構文**：
```
Type.method = func[position]
Type.method = func[0, 1, 2]    # 複数位置バインディング
Type.method = func[0, _, 2]   # プレースホルダーを使用
Type.method = func[-1]        # 負数インデックス（最後尾のパラメータ）
```

**意味論**：
- `Type.method = func[0]`は`obj.method(arg)`呼び出し時、`obj`が`func`の0位パラメータにバインディングされることを示す
- 残りのパラメータは元の順序で埋める

### 10.3 バインディング例

```yaoxiang
// === 基本バインディング ===

// 元の関数
distance: (a: Point, b: Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    (dx * dx + dy * dy).sqrt()
}

// Point型にバインディング（0位）
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

// === カリー化（引数不足時は自動的に関数を返す）===

// 1つのパラメータをバインディング
Point.distance_to = distance[0]

// 使用 - 2番目のパラメータを提供しない、カリー化関数を返す
f = p1.distance_to(p2)  # → distance(p1, p2) 直接呼び出し
f2 = p1.distance_to()   # → distance(p1, _) 関数を返す (Point) -> Float
result = f2(p2)         # → distance(p1, p2)
```

### 10.4 バインディング規則

**位置規則**：
| 規則 | 説明 |
|------|------|
| 位置は0から開始 | `func[0]`は最初のパラメータ（インデックス0）をバインディング |
| 最大位置 | 関数のパラメータ数未満でなければならない |
| 負数インデックス | `[-1]`は最後尾のパラメータを示す |
| プレースホルダー | `_`はその位置をスキップし、ユーザーが提供する |

**型チェック**：
```yaoxiang
// ✅ 有効なバインディング
Point.distance = distance[0]          # distance(Point, Point)
Point.calc = calculate[1, 2]          # calculate(Float, Point, Point, ...)

// ❌ 無効なバインディング（コンパイルエラー）
Point.wrong = distance[5]             # 5 >= 2（パラメータ数）
Point.wrong = distance[0, 0]          # 位置の重複（許可されていない場合）
Point.wrong = distance[-2]            # -2が範囲外
```

### 10.5 自動バインディング

モジュールで定義され最初のパラメータがモジュール型である関数について、メソッド呼び出し構文が自動サポートされる：

```yaoxiang
// === Point.yx ===
Point: Type = { x: Float, y: Float }

// 最初のパラメータはPoint、メソッド呼び出し構文が自動サポート
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
- 関数定義はモジュールファイル内で行われている
- 関数の0位パラメータ型とモジュール名が一致している
- 関数はモジュール外で自動バインディングするために`pub`でなければならない

### 10.6 バインディングとカリー化の関係

バインディングは自然にカリー化をサポート。呼び出し時に提供された引数が残りの引数より少ない場合、残りのパラメータを受け取る関数を返す：

```yaoxiang
// 元の関数：5つのパラメータ
calculate: (scale: Float, a: Point, b: Point, x: Float, y: Float) -> Float = ...

// バインディング：Point.calc = calculate[1, 2]
// バインディング後の残りのパラメータ：scale, x, y

// 呼び出しシナリオ
p1.calc(2.0, 10.0, 20.0)              # 3つの引数を提供 → 直接呼び出し
p1.calc(2.0)                          # 1つの引数を提供 → (Float, Float) -> Floatを返す
p1.calc()                             # 0個の引数を提供 → (Float, Float, Float) -> Floatを返す
```

---

## 付録C：バインディング構文早見表

### C.1 バインディング宣言

```
// 単位置バインディング（デフォルトで0位にバインディング）
Type.method = func[0]

// 複数置バインディング
Type.method = func[0, 1, 2]

// プレースホルダーを使用
Type.method = func[0, _, 2]

// 負数インデックス（最後尾のパラメータ）
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
// 直接呼び出し（すべての残りの引数を提供）
result = p.method(arg1, arg2, arg3)

// カリー化（残りの引数を提供しない、または一部のみ提供）
f = p.method(arg1)          # 残りの引数を受け取る関数を返す
result = f(arg2, arg3)
```

---

## バージョン履歴

| バージョン | 日付 | 著者 | 変更内容 |
|------|------|------|---------|
| v1.0.0 | 2024-12-31 | 晨煦 | 初期バージョン |
| v1.1.0 | 2025-01-04 | 沫郁酱 | match armは`=>`而非`->`を修正；関数定義構文を更新；型定義構文を更新；コード実装との違いの説明を追加 |
| v1.2.0 | 2025-01-05 | 沫郁酱 | 仕様のみに精简化、サンプルコードはtutorial/ディレクトリに移動 |
| v1.3.0 | 2025-01-05 | 沫郁酱 | 並行モデル仕様を追加（三層並行アーキテクチャ、spawn構文、注解）；型システム制約を追加（Send/Sync）；並行安全型を追加（Mutex、Atomic）；エラー処理を更新（?演算子）；未実装機能リストを更新 |
| v1.4.0 | 2025-01-15 | 晨煦 | 所有権モデルを更新（デフォルトMove + 明示的ref=Arc）；unsafeキーワードを追加；ライフタイム`'a`と借用チェッカーを削除；未実装機能リストを更新 |
| v1.5.0 | 2025-01-20 | 晨煦 | メソッドバインディング仕様を追加（RFC-004）：位置インデックスは0から開始；デフォルトで0位にバインディング；負数インデックスと複数位置バインディングをサポート |
| v1.6.0 | 2025-02-06 | 晨煦 | RFC-010（統一型構文）を統合：`type Name = {...}`構文を更新；シグネチャ内のパラメータ名の関数定義；Type.methodメソッド構文；RFC-011（ジェネリックシステム）を統合：ジェネリック型`[T]`を追加；型制約`[T: Clone]`；関連型`Item: T`；コンパイル時ジェネリクス`[N: Int]`；条件型`If[C, T, E]`；関数オーバーロード特殊化；プラットフォーム特殊化 |
| v1.7.0 | 2026-02-13 | 晨煦 | RFC-010を更新：`Name: Type = {...}`で`type Name = {...}`を置き換え；`Type`（大文字）のみがメタ型キーワード；すべての宣言は統一構文を使用 |
| v1.8.0 | 2026-02-18 | 晨煦 | RFC-010でデフォルト値初期化を追加；組み込みバインディング構文；RFC-004で組み込みバインディングを追加；無名関数バインディング |
| v1.8.1 | 2026-02-20 | 晨煦 | メタ型はキーワードではない。 |

---

> この仕様はYaoXiangプログラミング言語のコア構文と意味論を定義ものである。
> チュートリアルとサンプルコードについては、[YaoXiang ガイド](../guide/YaoXiang-book.md)および[tutorial/](../tutorial/)ディレクトリを参照のこと。
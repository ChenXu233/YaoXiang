> **注意：このドキュメントはアーカイブされており、保守されていません。**
> **新しい言語仕様ドキュメントを参照してください：[言語仕様](../reference/language-spec/index.md)**

---

# YaoXiang（爻象）プログラミング言語仕様

> バージョン：v1.8.0
> ステータス：仕様
> 著者：晨煦
> 日付：2024-12-31
> 更新：2026-02-22 - メタ型はキーワードに含まれない。

---

## 第1章：はじめに

### 1.1 範囲

本文書は YaoXiang プログラミング言語の構文と意味を定義する。これは言語の権威あるリファレンスであり、コンパイラとツールの実装者を対象としている。

チュートリアルとサンプルコードは [tutorial/](../tutorial/) ディレクトリを参照のこと。

### 1.2 適合性

本文書で定義されたすべてのルールを満たすプログラムまたは実装は、YaoXiang 仕様に準拠しているものと見なされる。

---

## 第2章：字句構造

### 2.1 ソースファイル

YaoXiang ソースファイルは UTF-8 エンコーディングを使用しなければならない。ソースファイルの拡張子は通常 `.yx` である。

### 2.2 字句単位の分類

| カテゴリ | 説明 | 例 |
|------|------|------|
| 識別子 | 文字またはアンダースコアで始まる | `x`, `_private`, `my_var` |
| キーワード | 言語で事前定義された予約語 | `Type`, `pub`, `use` |
| リテラル | 固定値 | `42`, `"hello"`, `true` |
| 演算子 | 演算記号 | `+`, `-`, `*`, `/` |
| 区切り記号 | 構文区切り | `(`, `)`, `{`, `}`, `,` |

### 2.3 キーワード

YaoXiang は非常に少数のキーワードを定義する：

```
pub    use    spawn
ref    mut    if     elif
else   match  while  for    return
break  continue as     in     unsafe
```

これらのキーワードはあらゆる文脈で特別な意味を持ち、識別子として使用することはできない。

### 2.4 予約語

| 予約語 | 型 | 説明 |
|--------|------|------|
| `Type` | Type | メタ型 |
| `true` | Bool | ブール真値 |
| `false` | Bool | ブール偽値 |
| `void` | Void | 空値 |
| `some(T)` | Option | Option 値変体 |
| `ok(T)` | Result | Result 成功変体 |
| `err(E)` | Result | Result エラー変体 |

### 2.5 識別子

識別子は文字またはアンダースコアで始まり、後続の文字は文字、数字、またはアンダースコアであることができる。識別子は大文字小文字を区別する。

特殊な識別子：
- `_` は値を無視することを示すプレースホルダとして使用される
- アンダースコアで始まる識別子はプライベートメンバーを表す

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

#### 2.6.6 メンバー検査

```
Membership  ::= Expr 'in' Expr
```

### 2.7 コメント

```
// 単一行コメント

/* 複数行コメント
   複数行にまたがることができる */
```

### 2.8 インデントルール

コードは 4 つのスペースでインデントしなければならず、Tab 文字の使用は禁止されている。これは強制構文ルールである。

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
| `Bool` | ブール値 | 1 バイト |
| `Int` | 符号付き整数 | 8 バイト |
| `Uint` | 符号なし整数 | 8 バイト |
| `Float` | 浮動小数点数 | 8 バイト |
| `String` | UTF-8 文字列 | 可変 |
| `Char` | Unicode 文字 | 4 バイト |
| `Bytes` | 生バイト | 可変 |

ビット幅指定の整数：`Int8`, `Int16`, `Int32`, `Int64`, `Int128`
ビット幅指定の浮動小数点数：`Float32`, `Float64`

### 3.3 レコード型

**統一構文**：`Name: Type = { field1: Type1, field2: Type2, ... }`

```
RecordType  ::= '{' FieldList? '}'
FieldList   ::= Field (',' Field)* ','?
Field       ::= Identifier ':' TypeExpr
            |  Identifier                 # インターフェース制約
```

```yaoxiang
// シンプルなレコード型
Point: Type = { x: Float, y: Float }

// 空のレコード型
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

**ルール**：
- レコード型は波括弧 `{}` を使用して定義する
- フィールド名の後にコロンと型を続ける
- インターフェース名は型本体内に記述することで、そのインターフェースの実装を示す

#### 3.3.1 フィールドのデフォルト値

型フィールドはデフォルト値を指定でき、構築時にはオプションで提供できる：

```yaoxiang
// デフォルト値を持つフィールド - 構築時はオプション
Point: Type = {
    x: Float = 0,
    y: Float = 0
}

// 使用
Point()           # → Point(x=0, y=0)
Point(x=1)       # → Point(x=1, y=0)
Point(x=1, y=2) # → Point(x=1, y=2)

// デフォルト値のないフィールド - 構築時は必須
Point2: Type = {
    x: Float,
    y: Float
}

// 使用
Point2(x=1, y=2) # ✓
Point2()          # ✗ エラー
```

**ルール**：
- `field: Type = expression` → デフォルト値あり、構築時はオプション
- `field: Type` → デフォルト値なし、構築時は必須

#### 3.3.2 組み込みバインディング

型定義本体内で直接メソッドをバインドできる：

```yaoxiang
// 方法1：外部関数の参照バインディング
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]    # 位置 0 にバインド
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

### 3.4 列挙型（変体型）

```
EnumType    ::= '{' Variant ('|' Variant)* '}'
Variant     ::= Identifier (':' TypeExpr)?
```

**構文**：`Name: Type = { Variant1 | Variant2(params) | ... }`

```yaoxiang
// 引数なし変体
Color: Type = { red | green | blue }

// 引数あり変体
Option: Type[T] = { some(T) | none }

// 混合
Result: Type[T, E] = { ok(T) | err(E) }

// 引数なし変体は引数なしコンストラクタと等価
Bool: Type = { true | false }
```

### 3.5 インターフェース型

```
InterfaceType ::= '{' FnField (',' FnField)* ','?
FnField       ::= Identifier ':' FnType
FnType        ::= '(' ParamTypes? ')' '->' TypeExpr
```

**構文**：インターフェースは全フィールドが関数型であるレコード型である

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

**インターフェース実装**：型は定義の最後にインターフェース名を列挙することでインターフェースを実装する

```yaoxiang
// インターフェースを実装する型
Point: Type = {
    x: Float,
    y: Float,
    Drawable,        # Drawable インターフェースを実装
    Serializable     # Serializable インターフェースを実装
}
```

**インターフェース直接代入**：具象型はインターフェース型変数に直接代入できる（構造的サブタイピング）

```yaoxiang
// 直接代入（コンパイル時に具象型が判明 → ゼロオーバーヘッド呼び出し）
d: Drawable = Circle(1)
d.draw(screen)        # コンパイル後：直接 circle_draw を呼び出し、vtable なし

// 関数戻り値（コンパイル時に不明 → vtable 呼び出し）
d: Drawable = get_shape()
d.draw(screen)        # vtable 経由でメソッドを検索

// インターフェースを関数の引数として
process: (d: Drawable) -> Void = d.draw(screen)
```

**コンパイル時最適化戦略**：

| シナリオ | 推論結果 | 呼び出し方式 |
|------|----------|----------|
| 具象型への直接代入 | 具象型が判明 | 直接呼び出し（ゼロオーバーヘッド） |
| 関数の戻り値 | 不明 | vtable |
| 異種コレクション | 複数の型 | vtable |

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

#### 3.8.1 ジェネリック引数の構文

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
// コンパイラがジェネリック引数を自動推論
numbers: List[Int] = List(1, 2, 3)  # コンパイラが List[Int] を推論
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

// 制約の使用
clone: [T: Clone](value: T) -> T = value.clone()
```

#### 3.9.2 複数制約

```yaoxiang
// 複数制約の構文
combine: [T: Clone + Add](a: T, b: T) -> T = {
    return a.clone() + b
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

#### 3.10.1 関連型の定義

```
AssociatedType ::= Identifier ':' TypeExpr
```

```yaoxiang
// Iterator trait（レコード型構文を使用）
Iterator: Type[T] = {
    Item: T,                    # 関連型
    next: (Self) -> Option[T],
    has_next: (Self) -> Bool
}

// 関連型の使用
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

**中核設計**：`[n: Int]` ジェネリック引数 + `(n: n)` 値引数で、コンパイル時定数とランタイム値を区別する。

```yaoxiang
// コンパイル時階乗：引数はコンパイル時に既知のリテラルでなければならない
factorial: [n: Int](n: n) -> Int = {
    return match n {
        0 => 1,
        _ => n * factorial(n - 1)
    }
}

// コンパイル時定数配列
StaticArray: Type[T, N: Int] = {
    data: T[N],      # コンパイル時に既知のサイズの配列
    length: N
}

// 使用方法
arr: StaticArray[Int, factorial(5)]  # コンパイラがコンパイル時に factorial(5) = 120 を計算
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
// 型レベル If
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

### 3.13 型ユニオン

```
TypeUnion     ::= TypeExpr '|' TypeExpr
```

### 3.14 型交差

```
TypeIntersection ::= TypeExpr '&' TypeExpr
```

**構文**：型交差 `A & B` は A と B の両方を満たす型を表す

```yaoxiang
// インターフェース合成 = 型交差
DrawableSerializable: Type = Drawable & Serializable

// 交差型の使用
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
// プラットフォーム型列挙（標準ライブラリで定義）
Platform: Type = X86_64 | AArch64 | RISC_V | ARM | X86

// P は事前定義されたジェネリック引数名で、現在のコンパイルプラットフォームを表す
sum: [P: X86_64](arr: Array[Float]) -> Float = {
    return avx2_sum(arr.data, arr.length)
}

sum: [P: AArch64](arr: Array[Float]) -> Float = {
    return neon_sum(arr.data, arr.length)
}
```

---

## 第3章（続き）：構文設計の説明

### 3.17 名前付き関数と Lambda の関係

**中核理解**：名前付き関数と Lambda 式は同じものである！唯一の違いは、名前付き関数が Lambda に名前を付けたことである。

```yaoxiang
// これらは本質的に完全に同じ
add: (a: Int, b: Int) -> Int = a + b           # 名前付き関数（推奨）
add: (a: Int, b: Int) -> Int = (a, b) => a + b  # Lambda 形式（完全に等価）
```

**構文糖衣モデル**：

```
// 名前付き関数 = Lambda + 名前
name: (Params) -> ReturnType = body

// 本質的には
name: (Params) -> ReturnType = (params) => body
```

**要点**：シグネチャに引数型が完全に宣言されている場合、Lambda ヘッダの引数名は冗長になり、省略可能である。

### 3.18 引数スコープルール

**引数が外側の変数を上書き**：シグネチャの引数スコープは関数本体を覆い、内部スコープの優先度が高い。

```yaoxiang
x = 10  # 外側の変数
double: (x: Int) -> Int = x * 2  # ✅ 引数 x が外側の x を上書き、結果は 20
```

### 3.19 型注釈位置

型注釈は以下のいずれかの位置に置くことができ、**少なくとも 1 箇所に注釈が必要**：

| 注釈位置 | 形式 | 説明 |
|----------|------|------|
| シグネチャのみ | `double: (x: Int) -> Int = x * 2` | ✅ 推奨 |
| Lambda ヘッダのみ | `double = (x: Int) => x * 2` | ✅ 合法 |
| 両方に注釈 | `double: (x: Int) -> Int = (x: Int) => x * 2` | ✅ 冗長だが許可される |

### 4.1 式の分類

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

### 4.8 パターン照合

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

### 4.10 Lambda 式

```
Lambda      ::= '(' ParamList? ')' '=>' Expr
            |  '(' ParamList? ')' '=>' Block
```

---

## 第5章：文

### 5.1 文の分類

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

### 5.3 return 文

```
ReturnStmt  ::= 'return' Expr?
```

### 5.4 break 文

```
BreakStmt   ::= 'break' Identifier?
```

### 5.5 continue 文

```
ContinueStmt::= 'continue'
```

### 5.6 if 文

```
IfStmt      ::= 'if' Expr Block ('elif' Expr Block)* ('else' Block)?
```

### 5.7 match 文

```
MatchStmt   ::= 'match' Expr '{' MatchArm+ '}'
```

### 5.8 while 文

```
WhileStmt   ::= 'while' Expr Block
```

### 5.9 for 文

```
ForStmt     ::= 'for' 'mut'? Identifier 'in' Expr Block
```

#### 5.9.1 セマンティクス：各反復は新しい値をバインドする

YaoXiang の for ループのセマンティクスは従来の言語とは異なる：**各反復は同じ変数を変更するのではなく、新しい値をバインドする**。

```yaoxiang
// 例：for i in 1..5
for i in 1..5 {
    print(i)
}
```

**実行プロセス**：

| 反復 | ループ変数の動作 |
|------|----------------|
| 1 回目 | 新しいバインド `i = 1` を作成、ループ本体を実行、1 を出力 |
| 2 回目 | 新しいバインド `i = 2` を作成（前のバインドは破棄済み）、ループ本体を実行、2 を出力 |
| 3 回目 | 新しいバインド `i = 3` を作成、ループ本体を実行、3 を出力 |
| 4 回目 | 新しいバインド `i = 4` を作成、ループ本体を実行、4 を出力 |
| 終了 | ループ本体の終了、バインド破棄 |

**要点**：各反復が終了すると、その反復で作成されたバインドは破棄される。次の反復は前の反復のバインドとは関係のない、完全に新しいバインドである。

#### 5.9.2 for と for mut の違い

| 構文 | ループ変数の可変性 | 説明 |
|------|----------------|------|
| `for i in 1..5` | 不変 | ループ本体内でバインドを変更できない |
| `for mut i in 1..5` | 可変 | ループ本体内でバインドを変更できる |

```yaoxiang
// ✅ 合法：各反復で新しい値をバインドし、変更不要
for i in 1..5 {
    print(i)  # i の値を読み取る
}

// ❌ エラー：不変バインド、変更不可
for i in 1..5 {
    i = i + 1  # エラー：不変バインドは変更不可
}

// ✅ 合法：for mut を使用するとバインドの変更が許可される
for mut i in 1..5 {
    i = i + 1  # 変更が許可される
}
```

#### 5.9.3 シャドーイングチェック

for ループ変数は外側スコープに既存の変数をシャドーイングできない：

```yaoxiang
// ❌ エラー：i はすでに外部で宣言されている
i = 10
for i in 1..5 {
    print(i)
}

// ✅ 正解：異なる変数名を使用
i = 10
for j in 1..5 {
    print(j)
}
```

エラーコード：`E2013 - Cannot shadow existing variable`

#### 5.9.4 他の言語との比較

| 言語 | for ループ変数のセマンティクス |
|------|------------------|
| YaoXiang | 各反復で新しい値をバインド |
| Rust | 同じ変数を変更（mut が必要） |
| Python | 同じ変数を変更（mut 不要） |
| C/C++ | 同じ変数を変更（ポインタまたは参照が必要） |

**設計理由**：YaoXiang がバインディングセマンティクスを採用する理由：
1. 各反復終了後、ループ本体内の変数は破棄される
2. 次の反復は完全に新しいバインドである
3. これによりより安全であり、反復間の状態を考慮する必要がない

---

## 第6章：関数

### 6.1 統一関数モデル

**中核構文**：`name: type = value`

YaoXiang は**統一宣言モデル**を採用する：変数、関数、メソッドはすべて同じ形式 `name: type = value` を使用する。

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
y = 100  # Int と推論される
```

### 6.3 関数定義

#### 6.3.1 完全な構文

```yaoxiang
// 引数名はシグネチャで宣言
add: (a: Int, b: Int) -> Int = {
    return a + b
}

// 単一引数
inc: (x: Int) -> Int = x + 1

// 引数なしの関数
main: () -> Void = {
    print("Hello")
}

// 複数行の関数本体
calc: (x: Float, y: Float, op: String) -> Float = {
    return match op {
        "+" => x + y,
        "-" => x - y,
        _ => 0.0
    }
}
```

#### 6.3.2 返却ルール

```yaoxiang
// Void 以外の戻り型 - return を使用しなければならない
add: (a: Int, b: Int) -> Int = {
    return a + b
}

// Void 戻り型 - return はオプション
print: (msg: String) -> Void = {
    // return 不要
}

// 単一行式 - 直接値を返し、return 不要
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

// ジェネリック制約の使用
clone: [T: Clone](value: T) -> T = value.clone()

// 複数型引数
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

// メソッド構文糖衣の使用
p: Point = Point(1.0, 2.0)
p.draw(screen)           # 構文糖衣 → Point.draw(p, screen)
```

#### 6.5.2 通常メソッド

**構文**：`name: (Type, ...) -> Return = ...`（型に関連付けない）

```yaoxiang
// 通常メソッド：型に関連付けず、独立した関数として
distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p1.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
}
```

### 6.6 メソッドバインディング

#### 6.6.1 手動バインディング

**構文**：`Type.method = function[positions]`

```yaoxiang
// 位置 0 にバインド（デフォルト）
Point.distance = distance[0]

// 位置 1 にバインド
Point.transform = transform[1]

// 複数位置のバインディング
Point.scale = scale[0, 1]

// プレースホルダの使用
Point.calc = func[0, _, 2]
```

#### 6.6.2 pub 自動バインディング

`pub` で宣言された関数は、コンパイラが同ファイルで定義された型に自動バインドする：

```yaoxiang
// pub で宣言し、コンパイラが自動バインド
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p1.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
}

// コンパイラが自動推論：
// 1. Point は現在のファイルで定義
// 2. 関数の引数に Point が含まれる
// 3. Point.distance = distance[0] を実行

// 呼び出し
d = distance(p1, p2)           # 関数形式
d2 = p1.distance(p2)           # OOP 構文糖衣
```

### 6.7 メソッドバインディングルール

| ルール | 説明 |
|------|------|
| 位置は 0 から開始 | `func[0]` は最初の引数（インデックス 0）にバインド |
| 最大位置 | 関数の引数の数 < でなければならない |
| 負のインデックス | `[-1]` は最後の引数を表す |
| プレースホルダ | `_` はその位置をスキップし、用户提供 |

### 6.8 カリー化サポート

バインディングは本質的にカリー化をサポートする。呼び出し時に提供された引数が残り引数より少ない場合、残りの引数を受け付ける関数を返す：

```yaoxiang
// 元の関数：5 つの引数
calculate: (scale: Float, a: Point, b: Point, x: Float, y: Float) -> Float = ...

// バインディング：Point.calc = calculate[1, 2]
// バインド後の残り引数：scale, x, y

// 呼び出しシナリオ
p1.calc(2.0, 10.0, 20.0)       # 3 つの引数を提供 → 直接呼び出し
p1.calc(2.0)                    # 1 つの引数を提供 → (Float, Float) -> Float を返す
p1.calc()                       # 0 つの引数を提供 → (Float, Float, Float) -> Float を返す
```

### 6.9 spawn 関数とアノテーション

#### 6.9.1 spawn 関数

```
SpawnFn     ::= Identifier ':' FnType 'spawn' '=' Lambda
FnType      ::= '(' ParamTypes? ')' '->' TypeExpr ('@' Annotation)?
Annotation  ::= 'block' | 'eager'
```

**関数アノテーション**：

| アノテーション | 位置 | 動作 |
|------|------|------|
| `@block` | 戻り型の後 | 並行最適化を無効化し、完全に順次実行 |
| `@eager` | 戻り型の後 | 即時評価を強制 |

**構文例**：

```
// spawn 関数：並行実行可能
fetch_data: (url: String) -> JSON spawn = { ... }

// @block 同期関数：完全に順次実行
main: () -> Void @block = { ... }

// @eager 急切関数：即時実行
compute: (n: Int) -> Int @eager = { ... }
```

#### 6.9.2 spawn ブロック

明示的に宣言された並行スコープ。ブロック内のタスクは spawn で実行される：

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
```

**例**：

```
// spawn ブロック：明示的な並行性
(result_a, result_b) = spawn {
    parse(fetch("url1")),
    parse(fetch("url2"))
}
```

#### 6.9.3 spawn ループ

データ並列ループ。ループ本体はすべてのデータ要素に対して spawn で実行される：

```
SpawnFor    ::= Identifier '=' 'spawn' 'for' Identifier 'in' Expr '{' Expr '}'
```

**例**：

```
// spawn ループ：データ並列
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
    data = fetch_data()?      # 自動エラー伝播
    return transform(data)?
}
```

---

## 第7章：モジュール

### 7.1 モジュール定義

モジュールはファイル境界を使用する。各 `.yx` ファイルは 1 つのモジュールである。

```
// ファイル名がモジュール名
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
| `use path;` | モジュールをインポート、最後の部分を使用してアクセス | `use std.io;` → `io.print` |
| `use path.{a, b};` | 指定した項目をインポート | `use std.io.{print};` → `print` |
| `use path as alias;` | インポートして名前変更 | `use std.io as io;` → `io.print` |
| `use path.{i1, i2} as a, b;` | 指定した項目をインポートして名前変更 | `use std.io.{print, read} as p, r;` → `p`, `r` |

---

## 第8章：メモリ管理

### 8.1 所有権モデル

YaoXiang は**所有権モデル**を採用してメモリを管理する。各値には一意の所有者がある：

| セマンティクス | 説明 | 構文 |
|------|------|------|
| **Move** | デフォルトセマンティクス、所有権の移転 | `p2 = p` |
| **ref** | 共有（Arc 参照カウント） | `shared = ref p` |
| **clone()** | 明示的なコピー | `p2 = p.clone()` |

### 8.2 Move セマンティクス（デフォルト）

```yaoxiang
// 代入 = Move（ゼロコピー）
p: Point = Point(1.0, 2.0)
p2 = p              # Move、p は無効化

// 関数引数渡し = Move
process: (p: Point) -> Void = {
    // p の所有権が転送される
}

// 戻り値 = Move
create: () -> Point = {
    p = Point(1.0, 2.0)
    return p        # Move、所有権の移転
}
```

### 8.3 ref キーワード（Arc）

`ref` キーワードは**参照カウントポインタ**（Arc）を作成し、安全な共有に使用される：

```yaoxiang
// Arc の作成
p: Point = Point(1.0, 2.0)
shared = ref p      # Arc、スレッドセーフ

// 共有アクセス
spawn(() => print(shared.x))   # ✅ 安全

// Arc はライフサイクルを自動管理
// shared がスコープを出ると、カウントがゼロになり自動解放
```

**特徴**：
- スレッドセーフな参照カウント
- ライフサイクルの自動管理
- spawn 境界を跨ぐ安全な共有

### 8.4 clone() 明示的コピー

```yaoxiang
// 値を明示的にコピー
p: Point = Point(1.0, 2.0)
p2 = p.clone()      # p と p2 は独立

// どちらも変更可能、相互に影響しない
p.x = 0.0           # ✅
p2.x = 0.0          # ✅
```

### 8.5 unsafe コードブロック

`unsafe` コードブロックは生ポインタの使用を許可し、システムレベルプログラミングに使用される：

```yaoxiang
// 生ポインタ型
PtrType ::= '*' TypeExpr

// unsafe コードブロック
UnsafeBlock ::= 'unsafe' '{' Stmt* '}'
```

**例**：

```yaoxiang
p: Point = Point(1.0, 2.0)

// 生ポインタは unsafe ブロック内でのみ使用可能
unsafe {
    ptr: *Point = &p     # 生ポインタを取得
    (*ptr).x = 0.0       # デリファレンス
}
```

**制限**：
- 生ポインタは `unsafe` ブロック内でのみ使用可能
- ユーザーはダングリングや解放後の使用がないことを保証する
- Send/Sync チェックに参加しない

### 8.7 所有権構文 BNF

```bnf
// === 所有権式 ===

// Move（デフォルト）
MoveExpr     ::= Expr

// ref Arc
RefExpr      ::= 'ref' Expr

// clone
CloneExpr    ::= Expr '.clone' '(' ')'

// === 生ポインタ（unsafe のみ） ===

PtrType       ::= '*' TypeExpr
UnsafeBlock   ::= 'unsafe' '{' Stmt* '}'
```

### 8.8 Send / Sync 制約

| 制約 | セマンティクス | 説明 |
|------|------|------|
| **Send** | スレッド境界を安全に越えて転送可能 | 値を別のスレッドに移動可能 |
| **Sync** | スレッド境界を安全に越えて共有可能 | 不変参照を別のスレッドに共有可能 |

**自動派生**：

```
// Send 派生ルール
Struct[T1, T2]: Send ⇐ T1: Send かつ T2: Send

// Sync 派生ルール
Struct[T1, T2]: Sync ⇐ T1: Sync かつ T2: Sync
```

**型制約**：

| 型 | Send | Sync | 説明 |
|------|------|------|------|
| `T`（値） | ✅ | ✅ | 不変データ |
| `ref T` | ✅ | ✅ | Arc はスレッドセーフ |
| `*T` | ❌ | ❌ | 生ポインタは安全ではない |

---

## 第8章（続き）：型システム制約

### 8.7 Send/Sync 制約

YaoXiang は Rust スタイルの型制約を使用して並行安全性を保証する：

| 制約 | セマンティクス | 説明 |
|------|------|------|
| **Send** | スレッド境界を安全に越えて転送可能 | 値を別のスレッドに移動可能 |
| **Sync** | スレッド境界を安全に越えて共有可能 | 不変参照を別のスレッドに共有可能 |

**制約階層**：

```
Send ──► スレッド境界を安全に越えて転送可能
  │
  └──► Sync ──► スレッド境界を安全に越えて共有可能
       │
       └──► Send + Sync を満たす型は自動的に並行可能

Arc[T] は Send + Sync を実装（スレッドセーフ参照カウント）
Mutex[T] は内部可変性を提供
```

### 8.8 並行安全型

| 型 | セマンティクス | 並行安全 | 説明 |
|------|------|----------|------|
| `T` | 不変データ | ✅ 安全 | デフォルト型、複数タスクの読み取りで競合なし |
| `Ref[T]` | 可変参照 | ⚠️ 同期必要 | 並行変更可能とマーク、ロック使用をコンパイル時チェック |
| `Atomic[T]` | アトミック型 | ✅ 安全 | 基盤となるアトミック操作、ロックフリー並行性 |
| `Mutex[T]` | ミューテックスラッパー | ✅ 安全 | 自動ロック・アンロック、コンパイル時保証 |
| `RwLock[T]` | 読み書きロックラッパー | ✅ 安全 | 読み取り多書き込み少シナリオの最適化 |

**構文**：

```
Mutex[T]    # ミューテックスでラップされた可変データ
Atomic[T]   # アトミック型（Int、Float などのみ）
RwLock[T]   # 読み書きロックラッパー
```

**with 構文糖衣**：

```
with mutex.lock() {
    // クリティカルセクション：Mutex で保護
    ...
}
```

---

## 第9章：エラー処理

### 9.1 Result 型

```
Result: Type[T, E] = ok(T) | err(E)
```

**変体構築**：

| 変体 | 構文 | 説明 |
|------|------|------|
| `ok(T)` | `ok(value)` | 成功値 |
| `err(E)` | `err(error)` | エラー値 |

### 9.2 Option 型

```
Option: Type[T] = some(T) | none
```

**変体構築**：

| 変体 | 構文 | 説明 |
|------|------|------|
| `some(T)` | `some(value)` | 値あり |
| `none` | `none` | 値なし |

### 9.3 エラー伝播

```
ErrorPropagate ::= Expr '?'
```

`?` 演算子は Result 型のエラーを自動伝播する：

```
// 成功時は値を返し、失敗時は err を上に返す
data = fetch_data()?

// 以下と等価
data = match fetch_data() {
    ok(v) => v
    err(e) => return err(e)
}
```

---

## 付録A：構文クイックリファレンス

### A.1 型定義

```
// === レコード型（波括弧） ===

// 構造体
Point: Type = { x: Float, y: Float }

// 列挙型（変体型）
Result: Type[T, E] = { ok(T) | err(E) }
Status: Type = { pending | processing | completed }

// === インターフェース型（波括弧、フィールドは全関数） ===

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
// 形式 1：型集中方式（推奨）
name: (param1: Type1, param2: Type2) -> ReturnType = body

// 形式 2：短縮方式（引数名省略）
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
// 単一位置バインディング
Type.method = func[0]

// 複数位置バインディング
Type.method = func[0, 1]

// pub 自動バインディング
pub name: (Type, ...) -> ReturnType = { ... }  # Type に自動バインド
```

### A.5 ジェネリック構文

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
// ファイル名.yx がモジュール名
Import ::= 'use' ModuleRef
```

### A.7 制御フロー

```
if Expr Block (elif Expr Block)* (else Block)?
match Expr { MatchArm+ }
while Identifier in Expr Block Expr Block
for
```

### A.8 match 構文

```
match value {
    pattern1 => expr1,
    pattern2 if guard => expr2,
    _ => default_expr,
}
```

---

## 付録B：コード実装との差異説明

> 本節では言語仕様と現在のコード実装の間の既知の差異を説明する。

### B.1 キーワード

| キーワード | 仕様のステータス | コード実装 | 説明 |
|--------|---------|---------|------|
| `struct` | 削除済み | ❌ なし | 統一構文 `Name: Type = {...}` を使用 |
| `enum` | 削除済み | ❌ なし | 変体構文 `Name: Type = { A \| B \| C }` を使用 |
| `type` | 削除済み | ❌ なし | メタ型キーワードとして `Type`（大文字）を使用 |

### B.2 構文差異

| 構文要素 | 仕様 | コード実装 | 説明 |
|---------|------|---------|------|
| match arm 区切り記号 | `->` | `=>` | `=>`（FatArrow）を使用 |
| 関数定義 | `name(types) -> type = (params) => body` | 2 つの形式 | 型集中方式 `name: Type = (params) =>` をサポート |
| インターフェース型 | `type Serializable = [ serialize() -> String ]` | ❌ 未実装 | 角括弧構文は実装待ち |

### B.3 未実装の機能

以下は仕様で記述されているが、まだコードで実装されていない機能：

| 機能 | 優先度 | 説明 |
|------|--------|------|
| 統一型構文 `Name: Type = {...}` | P0 | RFC-010：統一構文で `type Name = ...` を置換 |
| 波括弧型構文 | P0 | `Point: Type = { x: Float, y: Float }` |
| インターフェース型 | P1 | `Drawable: Type = { draw() -> Void }` |
| リスト内包表記 | P2 | `[x for x in list if condition]` |
| `?` エラー伝播 | P1 | Result 型の自動エラー伝播 |
| `ref` キーワード | P1 | Arc 参照カウント共有 |
| `unsafe` コードブロック | P1 | 生ポインタとシステムレベルプログラミング |
| `*T` 生ポインタ型 | P1 | 生ポインタ型構文 |
| `clone()` セマンティクス | P1 | 明示的コピー |
| `@block` アノテーション | P1 | 同期実行保証 |
| `spawn` 関数 | P1 | spawn 関数マーカー |
| `spawn {}` ブロック | P1 | 明示的並行スコープ |
| `spawn for` ループ | P1 | データ並列ループ |
| Send/Sync 制約 | P2 | 並行安全型チェック |
| Mutex/Atomic 型 | P2 | 並行安全データ型 |
| エラーグラフ可視化 | P3 | 並行エラー伝播追跡 |
| **ジェネリック型システム** | P1 | RFC-011 |
| 基本ジェネリクス `[T]` | P1 | ジェネリック型引数と単相化 |
| 型制約 `[T: Clone]` | P2 | 単一/複数制約システム |
| 関連型 `Item: T` | P3 | GAT サポート |
| コンパイル時ジェネリクス `[N: Int]` | P3 | リテラル型制約 |
| 条件型 `If[C, T, E]` | P3 | 型レベル計算 |
| 関数オーバーロード特殊化 | P2 | プラットフォーム特殊化と型特殊化 |
| メソッド構文 `Type.method` | P1 | `Point.draw: (...) -> ... = ...` |

### B.4 実装しない機能

以下の Rust スタイル機能は**実装されない**：

| 機能 | 理由 |
|------|------|
| ライフタイム `'a` | 参照の概念がないため、ライフタイム不要 |
| 借用チェッカー | ref = Arc で代替 |
| `&T` 借用構文 | Move セマンティクスを使用 |
| `&mut T` 可変借用 | mut + Move を使用 |

---

## 第10章：メソッドバインディング

### 10.1 バインディング概要

YaoXiang は**純粋関数型設計**を採用しており、すべての操作は関数を通じて実装される。バインディングメカニズムは関数を型に関連付け、呼び出し側がメソッドのように関数を呼び出せるようにする。

```
バインディング宣言 ::= 型 '.' 識別子 '=' 関数名 '[' 位置リスト ']'
位置リスト ::= 位置 (',' 位置)* ','?
位置     ::= 整数（0 から開始） | 負の整数 | プレースホルダ
```

**中核ルール**：
- 位置インデックスは **0** から開始する
- デフォルトで位置 **0**（最初の引数）にバインドする
- 負のインデックス `[-1]` は最後の引数を表す
- 複数位置の結合バインディング `[0, 1, 2]`
- プレースホルダ `_` はその位置をスキップする

### 10.2 バインディング構文

**構文**：
```
Type.method = func[position]
Type.method = func[0, 1, 2]    # 複数位置バインディング
Type.method = func[0, _, 2]   # プレースホルダの使用
Type.method = func[-1]        # 負のインデックス（最後の引数）
```

**セマンティクス**：
- `Type.method = func[0]` は `obj.method(arg)` 呼び出し時、`obj` が `func` の位置 0 の引数にバインドされることを表す
- 残りの引数は元の順序で埋められる

### 10.3 バインディング例

```yaoxiang
// === 基本バインディング ===

// 元の関数
distance: (a: Point, b: Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    (dx * dx + dy * dy).sqrt()
}

// Point 型にバインド（位置 0）
Point.distance = distance[0]

// 使用
p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)
d = p1.distance(p2)  # → distance(p1, p2)

// === 複数位置バインディング ===

// 元の関数
calculate: (scale: Float, a: Point, b: Point, x: Float, y: Float) -> Float = ...

// 複数位置のバインディング
Point.calc_scale = calculate[0]      # scale のみバインド
Point.calc_both = calculate[1, 2]    # 2 つの Point 引数をバインド

// 使用
f = p1.calc_scale(2.0)  # → calculate(2.0, p1, _, _, _)
result = f(p2, 10.0, 20.0)  # → calculate(2.0, p1, p2, 10.0, 20.0)

// === カリー化（引数不足時に自動的に関数を返す）===

// 1 つの引数をバインド
Point.distance_to = distance[0]

// 使用 - 2 番目の引数を提供しない場合、カリー化された関数を返す
f = p1.distance_to(p2)  # → distance(p1, p2) 直接呼び出し
f2 = p1.distance_to()   # → distance(p1, _) 関数 (Point) -> Float を返す
result = f2(p2)         # → distance(p1, p2)
```

### 10.4 バインディングルール

**位置ルール**：
| ルール | 説明 |
|------|------|
| 位置は 0 から開始 | `func[0]` は最初の引数（インデックス 0）にバインド |
| 最大位置 | 関数の引数の数 < でなければならない |
| 負のインデックス | `[-1]` は最後の引数を表す |
| プレースホルダ | `_` はその位置をスキップし、用户提供 |

**型チェック**：
```yaoxiang
// ✅ 合法なバインディング
Point.distance = distance[0]          # distance(Point, Point)
Point.calc = calculate[1, 2]          # calculate(Float, Point, Point, ...)

// ❌ 不正なバインディング（コンパイルエラー）
Point.wrong = distance[5]             # 5 >= 2（引数の数）
Point.wrong = distance[0, 0]          # 位置の重複（許可されない場合）
Point.wrong = distance[-2]            # -2 は範囲外
```

### 10.5 自動バインディング

モジュール内で定義され、最初の引数がモジュール型である関数は、メソッド呼び出し構文を自動サポートする：

```yaoxiang
// === Point.yx ===
Point: Type = { x: Float, y: Float }

// 最初の引数は Point、メソッド呼び出しを自動サポート
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

**自動バインディングルール**：
- 関数はモジュールファイル内で定義される
- 関数の位置 0 の引数型がモジュール名と一致する
- 関数はモジュール外で自動バインディングされるために `pub` でなければならない

### 10.6 バインディングとカリー化の関係

バインディングは本質的にカリー化をサポートする。呼び出し時に提供された引数が残り引数より少ない場合、残りの引数を受け付ける関数を返す：

```yaoxiang
// 元の関数：5 つの引数
calculate: (scale: Float, a: Point, b: Point, x: Float, y: Float) -> Float = ...

// バインディング：Point.calc = calculate[1, 2]
// バインド後の残り引数：scale, x, y

// 呼び出しシナリオ
p1.calc(2.0, 10.0, 20.0)              # 3 つの引数を提供 → 直接呼び出し
p1.calc(2.0)                          # 1 つの引数を提供 → (Float, Float) -> Float を返す
p1.calc()                             # 0 つの引数を提供 → (Float, Float, Float) -> Float を返す
```

---

## 付録C：バインディング構文クイックリファレンス

### C.1 バインディング宣言

```
// 単一位置バインディング（デフォルトで位置 0 にバインド）
Type.method = func[0]

// 複数位置バインディング
Type.method = func[0, 1, 2]

// プレースホルダの使用
Type.method = func[0, _, 2]

// 負のインデックス（最後の引数）
Type.method = func[-1]
```

### C.2 位置インデックスの説明

```
関数引数：    (p0, p1, p2, p3, p4)
              ↑  ↑  ↑  ↑  ↑
インデックス： 0  1  2  3  4

// バインディング [1, 3]
Type.method = func[1, 3]
// 呼び出し：obj.method(p0, p2, p4)
// マッピング：func(p0_bound, obj, p2, p3_bound, p4)
```

### C.3 呼び出し形式

```yaoxiang
// 直接呼び出し（すべての残り引数を提供）
result = p.method(arg1, arg2, arg3)

// カリー化（残り引数の一部または全部を提供しない）
f = p.method(arg1)          # 残り引数を受け付ける関数を返す
result = f(arg2, arg3)
```

---

## バージョン履歴

| バージョン | 日付 | 著者 | 変更内容 |
|------|------|------|---------|
| v1.0.0 | 2024-12-31 | 晨煦 | 初期バージョン |
| v1.1.0 | 2025-01-04 | 沫郁酱 | match arm で `->` ではなく `=>` を使用するよう修正；関数定義構文を更新；型定義構文を更新；コード実装との差異説明を追加 |
| v1.2.0 | 2025-01-05 | 沫郁酱 | 純粋仕様に簡素化、サンプルコードを tutorial/ ディレクトリに移動 |
| v1.3.0 | 2025-01-05 | 沫郁酱 | 並作モデル仕様を追加（3 層並行アーキテクチャ、spawn 構文、アノテーション）；型システム制約を追加（Send/Sync）；並行安全型を追加（Mutex、Atomic）；エラー処理を更新（? 演算子）；未実装機能リストを更新 |
| v1.4.0 | 2025-01-15 | 晨煦 | 所有権モデルを更新（デフォルト Move + 明示的 ref=Arc）；unsafe キーワードを追加；ライフタイム `'a` と借用チェッカーを削除；未実装機能リストを更新 |
| v1.5.0 | 2025-01-20 | 晨煦 | メソッドバインディング仕様を追加（RFC-004）：位置インデックスは 0 から開始；デフォルトで位置 0 にバインド；負のインデックスと複数位置バインディングをサポート |
| v1.6.0 | 2025-02-06 | 晨煦 | RFC-010（統一型構文）を統合：`type Name = {...}` 構文を更新、シグネチャ内の引数名を持つ関数定義、Type.method メソッド構文；RFC-011（ジェネリックシステム）を統合：ジェネリック型 `[T]`、型制約 `[T: Clone]`、関連型 `Item: T`、コンパイル時ジェネリクス `[N: Int]`、条件型 `If[C, T, E]`、関数オーバーロード特殊化、プラットフォーム特殊化を追加 |
| v1.7.0 | 2026-02-13 | 晨煦 | RFC-010 更新：`Name: Type = {...}` が `type Name = {...}` を置換；`Type`（大写）のみがメタ型キーワード；すべての宣言は統一構文を使用 |
| v1.8.0 | 2026-02-18 | 晨煦 | RFC-010 新規：デフォルト値初期化、組み込みバインディング構文；RFC-004 新規：組み込みバインディング、無名関数バインディング |
| v1.8.1 | 2026-02-20 | 晨煦 | メタ型はキーワードに含まれない。 |

---

> チュートリアルとサンプルコードは [tutorial/](../tutorial/) ディレクトリを参照してください。
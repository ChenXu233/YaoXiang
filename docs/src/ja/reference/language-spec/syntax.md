# 構文仕様

本文書は YaoXiang プログラミング言語の構文仕様を定義する。字句構造、構文規則、演算子の優先順位を含む。

---

## 第一章：字句構造

### 1.1 ソースファイル

YaoXiang ソースファイルは UTF-8 エンコーディングを使用しなければならない。ソースファイルの拡張子は通常 `.yx` である。

### 1.2 トークンの分類

| カテゴリー | 説明 | 例 |
|------|------|------|
| 識別子 | 文字またはアンダースコアで始まる | `x`, `_private`, `my_var` |
| キーワード | 言語で事前定義された予約語 | `Type`, `pub`, `use` |
| リテラル | 固定値 | `42`, `"hello"`, `true` |
| 演算子 | 演算記号 | `+`, `-`, `*`, `/` |
| 区切り記号 | 構文区切り記号 | `(`, `)`, `{`, `}`, `,` |

### 1.3 キーワード

YaoXiang はごく少数のキーワードを定義する：

```
pub    use    spawn
ref    mut    if     elif
else   match  while  for    return
break  continue as     in     unsafe
```

これらのキーワードはあらゆる文脈で特別な意味を持ち、識別子として使用することはできない。

### 1.4 予約語

YaoXiang の「予約語」は 3 層に分かれ、それぞれパーサ(parser)と型検査器(type checker)が異なる段階で識別する：

#### 1.4.1 リテラル予約語

パーサが独立したトークンを持つリテラル識別子は、通常の識別子として使用できない：

| 識別子 | 所属する型 | 説明 |
|--------|---------|------|
| `Type` | — | メタ型(meta type)キーワード |
| `true` | Bool | ブール真値 |
| `false` | Bool | ブール偽値 |
| `void` | Void | Void リテラル（Unit 値）。小文字の `void` は値リテラル；大文字の `Void` は型名（§1.4.3 参照）。|

#### 1.4.2 コンストラクタ式

以下のコンストラクタはパターンマッチング(pattern matching)と式の文脈でパーサによって識別される：

| コンストラクタ | 所属する型 | 説明 |
|--------|---------|------|
| `some(T)` | Option | Option 値バリアント(value variant)構築 |
| `ok(T)` | Result | Result 成功バリアント |
| `err(E)` | Result | Result エラーバリアント |

#### 1.4.3 組み込み型名

以下の型名は型検査器(type checker)によって事前登録されており、インポートなしでも型の位置で使用できる。パーサはこれらを通常の識別子として扱う——**予約語ではないため、ローカルバインディング(builtin binding)でシャドウイング可能（推奨されない）**。

| 型名 | 論理的対応 | 説明 |
|--------|---------|------|
| `Void` | ⊤（真/Unit） | 零フィールドの積型で、ちょうど 1 つの居住者を持つ（`void` リテラル、§1.4.1 参照） |
| `Never` | ⊥（偽/空型） | 零バリアント和型で、居住者を持たない。`Never` 値を生成する式は存在しない。`Never <: T` はすべての `T` に対して成立する（爆発原理）。|
| `Int` | — | 符号付き整数 |
| `Float` | — | 浮動小数点数 |
| `Bool` | — | ブール値：`true` / `false` |
| `Char` | — | Unicode 文字 |
| `String` | — | 文字列 |

### 1.5 識別子

識別子は文字またはアンダースコアで始まり、後続の文字は文字、数字、またはアンダースコアが使用できる。識別子は大文字小文字を区別する。

特別な識別子：
- `_` はプレースホルダとして使用され、何らかの値を無視することを示す
- アンダースコアで始まる識別子はプライベートメンバーを示す

### 1.6 リテラル

#### 1.6.1 整数

```
Decimal     ::= [0-9][0-9_]*
Octal       ::= 0o[0-7][0-7_]*
Hex         ::= 0x[0-9a-fA-F][0-9a-fA-F_]*
Binary      ::= 0b[01][01_]*
```

#### 1.6.2 浮動小数点数

```
Float       ::= [0-9][0-9_]* '.' [0-9][0-9_]* ([eE][+-]?[0-9][0-9_]*)?
```

#### 1.6.3 文字列

```
String      ::= '"' ([^"\\] | EscapeSequence)* '"'
Escape      ::= '\\' ([nrt'"\\] | UnicodeEscape)
Unicode     ::= 'u' '{' HexDigit+ '}'
```

#### 1.6.4 コレクション

```
List        ::= '[' Expr (',' Expr)* ']'
Dict        ::= '{' String ':' Expr (',' String ':' Expr)* '}'
Set         ::= '{' Expr (',' Expr)* '}'
```

#### 1.6.5 リスト内包表記

```
ListComp    ::= '[' Expr 'for' Identifier 'in' Expr (',' Expr)* ('if' Expr)? ']'
```

#### 1.6.6 メンバーシップ検査

```
Membership  ::= Expr 'in' Expr
```

### 1.7 コメント

```
// 単一行コメント

/* 複数行コメント
   複数行に渡ることができる */
```

### 1.8 インデント規則

コードは 4 つのスペースでインデントしなければならず、Tab 文字の使用は禁止されている。これは強制的な構文規則である。

---

## 第二章：構文規則

### 2.1 式の分類

```
Expr        ::= Literal
              | Identifier
              | FnCall
              | MemberAccess
              | IndexAccess
              | UnaryOp
              | BinaryOp
              | TypeCast
              | RangeExpr
              | ErrorPropagate
              | RefExpr
              | IfExpr
              | MatchExpr
              | Block
              | Lambda
```

### 2.2 演算子の優先順位

| 優先順位 | 演算子 | 結合性 |
|--------|--------|--------|
| 1 | `()` `[]` `.` `?` | 左から右 |
| 2 | `as` | 左から右 |
| 3 | `*` `/` `%` | 左から右 |
| 4 | `+` `-` | 左から右 |
| 5 | `..` | 左から右 |
| 6 | `<<` `>>` | 左から右 |
| 7 | `&` `\|` `^` | 左から右 |
| 8 | `==` `!=` `<` `>` `<=` `>=` | 左から右 |
| 9 | `not` | 右から左 |
| 10 | `and` `or` | 左から右 |
| 11 | `if...else` | 右から左 |
| 12 | `=` `+=` `-=` `*=` `/=` | 右から左 |

### 2.3 関数呼び出し

```
FnCall      ::= Expr '(' ArgList? ')'
ArgList     ::= Expr (',' Expr)* (',' NamedArg)* | NamedArg (',' NamedArg)*
NamedArg    ::= Identifier ':' Expr
```

### 2.4 メンバーアクセス

```
MemberAccess::= Expr '.' Identifier
```

### 2.5 インデックスアクセス

```
IndexAccess ::= Expr '[' Expr ']'
```

### 2.6 型変換

```
TypeCast    ::= Expr 'as' TypeExpr
```

### 2.7 条件式

```
IfExpr      ::= 'if' Expr Block ('elif' Expr Block)* ('else' Block)?
```

### 2.8 パターンマッチング

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

### 2.9 ブロック式

```
Block       ::= '{' Stmt* Expr? '}'
```

**統一意味論**：すべての `{}` ブロックの `return` 意味論は一貫している：

| ブロックタイプ | return 意味論 | デフォルト戻り値 |
|--------|------------|----------|
| 通常の `{}` | 値を返す | Void |
| `unsafe {}` | 型定義を返す | Void |
| `spawn {}` | 結果を返す | Void |

**核心原則**：
- `{}` 内の `return` は常に内容を外側のスコープに返す
- デフォルトでは `return` がない場合は `Void` を返す
- 式の形式 `= expr` は直接値を返す

```yaoxiang
// 通常の {} ブロック：return は値を返す
result = {
    x = compute()
    return x  // 値を外側のスコープに返す
}

// unsafe {} ブロック：return は型定義を返す
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void
    }
    return SqliteDb  // 型定義を外側のスコープに返す
}

// spawn {} ブロック：return は結果を返す
(a, b) = spawn {
    result1 = fetch("url1"),
    result2 = fetch("url2")
    return (result1, result2)  // 結果を外側のスコープに返す
}
```

### 2.10 ラムダ式

```
Lambda      ::= '(' ParamList? ')' '=>' Expr
            |  '(' ParamList? ')' '=>' Block
```

### 2.11 エラー伝播演算子

```
ErrorPropagate ::= Expr '?'
```

`?` 演算子は後置演算子で、優先順位は `.` と同階層である。`Result(T, E)` 型に対して：
- `Ok(v)` の場合は値 `v` を抽出して実行を続ける
- `Err(e)` の場合はエラーを上位に伝播する（`return Err(e)`）

```yaoxiang
process: (data: Data) -> Result(Data, Error) = {
    validated = validate(data)?     // 成功時は値を抽出し、失敗時は上位に伝播する
    transform(validated)
}
```

### 2.12 範囲式

```
RangeExpr   ::= Expr '..' Expr
```

`..` は範囲型を作成し、`for` ループやスライスに使用される。

```yaoxiang
for i in 0..10 { print(i) }
slice = array[0..5]
```

### 2.13 ref 式

```
RefExpr     ::= 'ref' Expr
```

`ref` は共有所有(ownership)を作成する。コンパイラが自動的に Rc（単一タスク）または Arc（タスク間）を選択し、ユーザーは実装の詳細を気にする必要はない。

```yaoxiang
data = ref heavy_data
spawn { use(data) }   // タスク間：コンパイラが自動的に Arc を選択
```

### 2.14 unsafe 式

```
UnsafeExpr  ::= 'unsafe' Block
```

`unsafe` ブロックは不透明型の定義と生ポインタ(raw pointer)の操作に使用される。`return` を使用して型定義を外側のスコープに返す。

**意味論**：
- `unsafe {}` 内で型を定義し、生ポインタを操作できる
- 返される型は `unsafe {}` の外側でも使用可能
- 型のフィールドアクセスには unsafe 権限が必要

```yaoxiang
// unsafe ブロック内で不透明型を定義
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  // 生ポインタ
    }
    return SqliteDb
}

// SqliteDb は unsafe ブロック外でも使用可能
db = sqlite3_open("test.db")
```

### 2.15 スコープ

**基本規則**：
- 各 `{}` ブロックは 1 つのスコープを作成する
- 内側のスコープは外側のスコープの変数にアクセスできる
- 外側のスコープは内側のスコープの変数にアクセスできない
- 変数宣言は「代入優先」原則に従う

```yaoxiang
// ブロックスコープ
{
    x = 10
    // x はこのスコープ内で可視
}
// x はこのスコープ外では不可視

// 関数スコープ
add: (a: Int, b: Int) -> Int = {
    result = a + b
    return result
}
// result は関数の外では不可視
```

**変数宣言とシャドウイング**：
- `x = value`：スコープチェーンを遡って x を検索し、見つかれば代入、見つからなければ新規宣言
- `mut x = value`：明示的な新しい可変宣言で、外側と同じ名前は禁止
- 同じスコープ内では任意の名前は 1 回だけしか宣言できない

> **詳細な定義**：スコープの完全な規則、変数宣言、シャドウイング機構の詳細は [モジュールシステム仕様](./modules.md#第四章スコープ) を参照。

---

## 第三章：文

### 3.1 文の分類

```
Stmt        ::= LetStmt
              | ExprStmt
              | ReturnStmt
              | BreakStmt
              | ContinueStmt
              | IfStmt
              | MatchStmt
              | WhileStmt
              | ForStmt
              | SpawnStmt
```

### 3.2 変数宣言

```
LetStmt     ::= ('mut')? Identifier (':' TypeExpr)? '=' Expr
```

### 3.3 return 文

```
ReturnStmt  ::= 'return' Expr?
```

**意味論**：`return` はコードブロックから値を返すために使用される。`return` がない場合、コードブロックはデフォルトで `Void` を返す。

### 3.4 break 文

```
BreakStmt   ::= 'break' Identifier?
```

### 3.5 continue 文

```
ContinueStmt::= 'continue'
```

### 3.6 if 文

```
IfStmt      ::= 'if' Expr Block ('elif' Expr Block)* ('else' Block)?
```

### 3.7 match 文

```
MatchStmt   ::= 'match' Expr '{' MatchArm+ '}'
```

### 3.8 while 文

```
WhileStmt   ::= 'while' Expr Block
```

### 3.9 for 文

```
ForStmt     ::= 'for' 'mut'? Identifier 'in' Expr Block
```

#### 3.9.1 意味論：各反復は新しい値を束縛する

YaoXiang の for ループ意味論は従来の言語と異なる：**各反復は新しい値を束縛し、同じ変数を変更するのではない**。

```yaoxiang
// 例：for i in 1..5
for i in 1..5 {
    print(i)
}
```

**実行プロセス**：

| 反復 | ループ変数の動作 |
|------|----------------|
| 1 回目 | 新しい束縛 `i = 1` を作成し、ループ本体を実行、1 を表示 |
| 2 回目 | 新しい束縛 `i = 2` を作成し（以前の束縛は破棄済み）、ループ本体を実行、2 を表示 |
| 3 回目 | 新しい束縛 `i = 3` を作成し、ループ本体を実行、3 を表示 |
| 4 回目 | 新しい束縛 `i = 4` を作成し、ループ本体を実行、4 を表示 |
| 終了 | ループ本体終了、束縛を破棄 |

**要点**：各反復終了後、その反復で作成された束縛は破棄される。次の反復は完全に新しい束縛であり、前の反復の束縛とは一切関係がない。

#### 3.9.2 for と for mut の違い

| 構文 | ループ変数の可变性 | 説明 |
|------|----------------|------|
| `for i in 1..5` | 不変 | ループ本体内で束縛を変更できない |
| `for mut i in 1..5` | 可変 | ループ本体内で束縛を変更できる |

```yaoxiang
// 合法：各反復で新しい値を束縛し、変更は不要
for i in 1..5 {
    print(i)  // i の値を読み取る
}

// エラー：不変の束縛、変更不可
for i in 1..5 {
    i = i + 1  // エラー：不変の束縛は変更できない
}

// 合法：for mut を使用すれば束縛の変更が許可される
for mut i in 1..5 {
    i = i + 1  // 変更が許可される
}
```

#### 3.9.3 シャドウイング検査

YaoXiang は変数のシャドウイングを禁止する。for ループ変数は外側スコープの変数と同じ名前にできない：

```yaoxiang
// エラー：i はすでに外部で宣言されている
i = 10
for i in 1..5 {
    print(i)
}

// 正しい：異なる変数名を使用
i = 10
for j in 1..5 {
    print(j)
}
```

この規則はすべてのコードブロックに適用される。詳細は [4.3 シャドウイング規則](./modules.md#43-シャドウイング規則) を参照。

#### 3.9.4 他の言語との比較

| 言語 | for ループ変数の意味論 |
|------|------------------|
| YaoXiang | 各反復で新しい値を束縛 |
| Rust | 同じ変数を変更（mut が必要） |
| Python | 同じ変数を変更（mut 不要） |
| C/C++ | 同じ変数を変更（ポインタまたは参照が必要） |

**設計理由**：YaoXiang が束縛意味論を採用するのは以下の理由による：

1. **自然意味論により適合**
   自然言語では、「集合の各要素 x について」は各 x が独立した個体であることを意味する。YaoXiang の `for i in 1..5` は「1 から 5 の各 i について」と読まれ、各反復の i は完全に新しい束縛であり、これは人間の直感的理解と一致する。

2. **偶発的な変更を回避**
   デフォルトの不変な束縛意味論は、ループ本体内でループ変数を偶発的に変更できないことを意味する。複雑なループ本体のどこかで誤って `i = ...` と書いてしまい、追跡困難なバグが発生する心配がない。

3. **高性能なソリューションに簡単に手が届く**
   反復間で変数を再利用する必要が実際にある場合（アキュムレータやキャッシュなど）、`for mut` 宣言を使用すれば可変束縛モードに切り替えられる。これは暗黙的な共有状態よりも明確である——意図は構文で明示的に表現され、実行時動作(runtime)に隠されているのではない。

### 3.10 spawn 文

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
SpawnFor    ::= Identifier '=' 'spawn' 'for' 'mut'? Identifier 'in' Expr '{' Expr '}'
SpawnStmt   ::= SpawnBlock | SpawnFor
```

**spawn ブロック**：並列領域を明示的に宣言し、ブロック内の式を並列実行する。

```yaoxiang
(result_a, result_b) = spawn {
    parse(fetch("url1")),
    parse(fetch("url2"))
}
```

**spawn ループ**：データ並列ループ。

```yaoxiang
results = spawn for item in items {
    process(item)
}
```

---

## 付録：構文早見表

### A.1 制御フロー

```
if Expr Block (elif Expr Block)* (else Block)?
match Expr { MatchArm+ }
while Expr Block
for 'mut'? Identifier 'in' Expr Block
```

### A.2 エラーハンドリング

```
Expr '?'              // エラー伝播（Result 型）
```

### A.3 match 構文

```
match value {
    pattern1 => expr1,
    pattern2 if guard => expr2,
    _ => default_expr,
}
```
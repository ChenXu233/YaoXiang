# 構文仕様書

本文書は YaoXiang プログラミング言語の構文仕様を定義ものであり、字句構造、構文規則、演算子の優先順位を含む。

---

## 第1章：字句構造

### 1.1 ソースファイル

YaoXiang ソースファイルは UTF-8 エンコーディングを使用する必要がある。ソースファイルは通常 `.yx` を拡張子とする。

### 1.2 字句单元の分類

| カテゴリ | 説明 | 例 |
|------|------|------|
| 識別子 | 文字またはアンダースコアで始まる | `x`, `_private`, `my_var` |
| キーワード | 言語が予約する単語 | `Type`, `pub`, `use` |
| リテラル | 固定値 | `42`, `"hello"`, `true` |
| 演算子 | 演算記号 | `+`, `-`, `*`, `/` |
| 区切り記号 | 構文区切り | `(`, `)`, `{`, `}`, `,` |

### 1.3 キーワード

YaoXiang は非常に少数のキーワードを定義する：

```
pub    use    spawn
ref    mut    if     elif
else   match  while  for    return
break  continue as     in     unsafe
```

これらのキーワードはどのようなコンテキストにおいても特別な意味を持ち、識別子として使用することはできない。

### 1.4 予約語

| 予約語 | 型 | 説明 |
|--------|------|------|
| `Type` | Type | メタ型 |
| `true` | Bool | ブール値の真 |
| `false` | Bool | ブール値の偽 |
| `void` | Void | 空値 |
| `some(T)` | Option | Option 値バリアント |
| `ok(T)` | Result | Result 成功バリアント |
| `err(E)` | Result | Result エラーンティант |

### 1.5 識別子

識別子は文字またはアンダースコアで始まり、後続の文字は文字、数字、またはアンダースコア可以使用。識別子は大文字と小文字を区別する。

特殊識別子：
- `_` はプレースホルダーとして使用され、ある値を無視することを意味する
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

#### 1.6.6 メンバー検査

```
Membership  ::= Expr 'in' Expr
```

### 1.7 コメント

```
// 単一行コメント

/* 複数行コメント
   複数行にまたがることもできる */
```

### 1.8 インデント規則

コードはスペース 4 個でインデントする必要があります。Tab 文字の使用は禁止です。これは強制的な構文規則です。

---

## 第2章：構文規則

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

### 2.8 パターン照合

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

**統一的セマンティクス**：すべての `{}` ブロックの return セマンティクスは一貫しています：

| ブロック型 | return セマンティクス | デフォルト返り値 |
|--------|------------|----------|
| 通常の `{}` | 値を返す | Void |
| `unsafe {}` | 型定義を返す | Void |
| `spawn {}` | 結果を返す | Void |

**基本原則**：
- `{}` 内での `return` は常に内容を外側のスコープに返す
- `return` がない場合のデフォルトは `Void` を返す
- 式形式 `= expr` は直接値を返す

```yaoxiang
# 通常の {} ブロック：return は値を返す
result = {
    x = compute()
    return x  # 値を外側のスコープに返す
}

# unsafe {} ブロック：return は型定義を返す
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void
    }
    return SqliteDb  # 型定義を外側のスコープに返す
}

# spawn {} ブロック：return は結果を返す
(a, b) = spawn {
    result1 = fetch("url1"),
    result2 = fetch("url2")
    return (result1, result2)  # 結果を外側のスコープに返す
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

`?` 演算子は後置演算子であり、優先順位は `.` と同じ。`Result(T, E)` 型に対して：
- `Ok(v)` の場合は値 `v` を抽出して実行継続
- `Err(e)` の場合はエラーを上に伝播（`return Err(e)`）

```yaoxiang
process: (data: Data) -> Result(Data, Error) = {
    validated = validate(data)?     // 成功時は値を抽出、失敗時は上に伝播
    transform(validated)
}
```

### 2.12 範囲式

```
RangeExpr   ::= Expr '..' Expr
```

`..` は範囲型を作成し、`for` ループやスライスに使用する。

```yaoxiang
for i in 0..10 { print(i) }
slice = array[0..5]
```

### 2.13 ref 式

```
RefExpr     ::= 'ref' Expr
```

`ref` は共有所有を作成する。コンパイラは自動的に Rc（単一タスク）または Arc（タスク間）を選択するため、ユーザーは実装の詳細を気にする必要はない。

```yaoxiang
data = ref heavy_data
spawn { use(data) }   // タスク間：コンパイラが自動的に Arc を選択
```

### 2.14 unsafe 式

```
UnsafeExpr  ::= 'unsafe' Block
```

`unsafe` ブロックは不透明型和型和裸ポインタの操作を定義するために使用される。`return` を使用して型定義を外側のスコープに返す。

**セマンティクス**：
- `unsafe {}` 内では型定義と裸ポインタの操作が可能
- 返された型は `unsafe {}` の外でも使用可能
- 型のフィールドアクセスには unsafe 権限が必要

```yaoxiang
# unsafe ブロック内で不透明型を定義
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  # 裸ポインタ
    }
    return SqliteDb
}

# SqliteDb は unsafe ブロックの外で使用可能
db = sqlite3_open("test.db")
```

### 2.15 スコープ

**基本規則**：
- 各 `{}` ブロックはスコープを作成する
- 内側のスコープは外側のスコープの変数にアクセス可能
- 外側のスコープは内側のスコープの変数にアクセス不可
- 変数宣言は「代入優先」原則に従う

```yaoxiang
# ブロックスコープ
{
    x = 10
    # x はこのスコープ内で可視
}
# x はこのスコープの外では不可視

# 関数スコープ
add: (a: Int, b: Int) -> Int = {
    result = a + b
    return result
}
# result は関数外では不可視
```

**変数宣言とシャドーイング**：
- `x = value`：スコープチェーンを外に向かって x を探査、見つかれば代入、見つからなければ新規宣言
- `mut x = value`：明示的な新規可変宣言、外側と同名を禁止
- 同一スコープ内で любой 名前を二度宣言することはできない

> **詳細定義**：スコープの完全な規則、変数宣言、シャドーイング機構については [モジュールシステム仕様](./modules.md#第四章作用域) を参照。

---

## 第3章：文

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

**セマンティクス**：`return` はコードブロックから値を返すために使用する。`return` がない場合、コードブロックはデフォルトで `Void` を返す。

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

#### 3.9.1 セマンティクス：各反復は新しい値へのバインディング

YaoXiang の for ループのセマンティクスは従来の言語と異なる：**各反復は、同じ変数を変更するのではなく、新しい値へのバインディングである**。

```yaoxiang
// 例：for i in 1..5
for i in 1..5 {
    print(i)
}
```

**実行過程**：

| 反復 | ループ変数の動作 |
|------|----------------|
| 第1回 | 新規バインディング `i = 1` を作成し、ループ本体を実行、1 を出力 |
| 第2回 | 新規バインディング `i = 2` を作成し（前回のバインディングは破棄済み）、ループ本体を実行、2 を出力 |
| 第3回 | 新規バインディング `i = 3` を作成し、ループ本体を実行、3 を出力 |
| 第4回 | 新規バインディング `i = 4` を作成し、ループ本体を実行、4 を出力 |
| 終了 | ループ本体終了、バインディング破棄 |

**要点**：各反復の終了後、その反復で作成されたバインディングは破棄される。次の反復は前回の反復のバインディングとは何の関係もない совершенно 新規のバインディングである。

#### 3.9.2 for と for mut の違い

| 構文 | ループ変数の可変性 | 説明 |
|------|----------------|------|
| `for i in 1..5` | 不可変 | ループ本体内でバインディングを変更不可 |
| `for mut i in 1..5` | 可変 | ループ本体内でバインディングを変更可能 |

```yaoxiang
// 有効：各反復で新しい値にバインディングするため、変更は不要
for i in 1..5 {
    print(i)  // i の値を読み取る
}

// 誤り：不可変バインディングは変更不可
for i in 1..5 {
    i = i + 1  // 誤り：不可変バインディングは変更不可
}

// 有効：for mut を使用すればバインディングの変更を許可
for mut i in 1..5 {
    i = i + 1  // 許可
}
```

#### 3.9.3 シャドーイング検査

YaoXiang は変数のシャドーイングを禁止する。for ループ変数は外側のスコープの変数と同じ名前を使用できない：

```yaoxiang
// 誤り：i はすでに外部で宣言されている
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

この規則はすべてのコードブロックに適用される。詳細については [4.3 シャドーイング規則](./modules.md#43-遮蔽规则) を参照。

#### 3.9.4 他の言語との比較

| 言語 | for ループ変数のセマンティクス |
|------|------------------|
| YaoXiang | 各反復で新しい値にバインディング |
| Rust | 同じ変数を変更（mut が必要） |
| Python | 同じ変数を変更（mut 不要） |
| C/C++ | 同じ変数を変更（ポインタまたは参照が必要） |

**設計理由**：YaoXiang がバインディングセマンティクスを選択したのは以下のためである：

1. **より自然なセマンティクスに適合**
   自然言語では「集合内の各要素 x について」とは、各 x が独立した个体であることを意味する。YaoXiang の `for i in 1..5` は「1 から 5 の中の各 i について」と読み、各反復の i は совершенно 新しいバインディングであり、これは人間の直感的な理解と一致する。

2. **偶発的な変更を回避**
   デフォルトで不可変のバインディングセマンティクスは、ループ本体内でループ変数を偶発的に変更できないことを意味する。複雑なループ本体の中で誤って `i = ...` を書いて追跡困難なバグが発生する心配がない。

3. **高性能な解決策が簡単に手に届く**
   反復間で変数を再利用する必要がある場合（例えばアキュムレータ、キャッシュ）、`for mut` を使用して可変バインディングモードに切り替えることができる。これは暗黙の共有状態よりも清晰である——意図は構文で明示的に表現され、ランタイム動作に隠されていない。

### 3.10 spawn 文

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
SpawnFor    ::= Identifier '=' 'spawn' 'for' 'mut'? Identifier 'in' Expr '{' Expr '}'
SpawnStmt   ::= SpawnBlock | SpawnFor
```

**spawn ブロック**：并发制御範囲を明示的に宣言し、ブロック内の式并发的に実行する。

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

### A.2 エラー処理

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
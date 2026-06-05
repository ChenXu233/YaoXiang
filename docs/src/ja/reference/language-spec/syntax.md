# 構文仕様

本ファイルでは、YaoXiang プログラミング言語の構文仕様を定義する。字句構造、構文規則、演算子の優先順位を含む。

---

## 第1章：字句構造

### 1.1 ソースファイル

YaoXiang のソースファイルは UTF-8 エンコーディングを使用する必要がある。ソースファイルの拡張子は通常 `.yx` である。

### 1.2 トークンの分類

| カテゴリ | 説明 | 例 |
|------|------|------|
| 識別子 | 文字またはアンダースコアで始まる | `x`, `_private`, `my_var` |
| キーワード | 言語が予約している単語 | `Type`, `pub`, `use` |
| リテラル | 固定値 | `42`, `"hello"`, `true` |
| 演算子 | 演算記号 | `+`, `-`, `*`, `/` |
| 区切り文字 | 構文区切り文字 | `(`, `)`, `{`, `}`, `,` |

### 1.3 キーワード

YaoXiang は非常に少数のキーワードを定義している：

```
pub    use    spawn
ref    mut    if     elif
else   match  while  for    return
break  continue as     in     unsafe
```

これらのキーワードは任意のコンテキストで特別な意味を持ち、識別子として使用することはできない。

### 1.4 予約語

| 予約語 | 型 | 説明 |
|--------|------|------|
| `Type` | Type | メタ型 |
| `true` | Bool | ブール真値 |
| `false` | Bool | ブール偽値 |
| `void` | Void | 空値 |
| `some(T)` | Option | Option 値变体 |
| `ok(T)` | Result | Result 成功变体 |
| `err(E)` | Result | Result エラー变体 |

### 1.5 識別子

識別子は文字またはアンダースコアで始まり、後続の文字には文字、数字、アンダースコアを使用できる。識別子は大文字小文字を区別する。

特殊識別子：
- `_` はプレースホルダとして使用され、特定の値を無視することを意味する
- アンダースコアで始まる識別子はプライベートメンバを表す

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
// 单行コメント

/* 多行コメント
   可以跨越多行 */
```

### 1.8 インデント規則

コードは4スペースのインデントを使用する必要がある。Tab 文字の使用は禁止。これは強制的な構文規則である。

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

**意味**：コードブロック内では `return` を使用して値を返さなければならない。`return` がない場合はデフォルトで `Void` を返す。式形式 `= expr` は直接値を返す。

### 2.10 ラムダ式

```
Lambda      ::= '(' ParamList? ')' '=>' Expr
            |  '(' ParamList? ')' '=>' Block
```

### 2.11 エラー伝播演算子

```
ErrorPropagate ::= Expr '?'
```

`?` 演算子は後置演算子で、優先順位は `.` と同レベルである。`Result(T, E)` 型に対して：
- `Ok(v)` の場合、値 `v` を抽出して続行
- `Err(e)` の場合、エラーを上に伝播（`return Err(e)`）

```yaoxiang
process: (data: Data) -> Result(Data, Error) = {
    validated = validate(data)?     // 成功時に値を抽出し、失敗時に上に伝播
    transform(validated)
}
```

### 2.12 範囲式

```
RangeExpr   ::= Expr '..' Expr
```

`..` は範囲型を作成する。`for` ループとスライスに使用される。

```yaoxiang
for i in 0..10 { print(i) }
slice = array[0..5]
```

### 2.13 ref 式

```
RefExpr     ::= 'ref' Expr
```

`ref` は共有所有を作成する。コンパイラは Rc（単一タスク）または Arc（タスク間）を自動的に選択し、ユーザーは実装の詳細を気にする必要はない。

```yaoxiang
data = ref heavy_data
spawn { use(data) }   // タスク間：コンパイラが自動的に Arc を選択
```

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

**意味**：`return` はコードブロックから値を返すために使用する。`return` がない場合、コードブロックはデフォルトで `Void` を返す。

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

#### 3.9.1 意味：各反復は新しい束縛である

YaoXiang の for ループの意味は従来の言語とは異なる：**各反復は新しい束縛であり、同じ変数を変更するわけではない**。

```yaoxiang
// 例：for i in 1..5
for i in 1..5 {
    print(i)
}
```

**実行プロセス**：

| 反復 | ループ変数の動作 |
|------|------------------|
| 第1回 | 新しい束縛 `i = 1` を作成し、ループ本体を実行、1 を出力 |
| 第2回 | 新しい束縛 `i = 2` を作成（以前の束縛は破棄済み）、ループ本体を実行、2 を出力 |
| 第3回 | 新しい束縛 `i = 3` を作成し、ループ本体を実行、3 を出力 |
| 第4回 | 新しい束縛 `i = 4` を作成し、ループ本体を実行、4 を出力 |
| 終了 | ループ本体終了、束縛破棄 |

**重要なポイント**：各反復の終了後、その反復で作成された束縛は破棄される。次の反復は前回の反復の束縛とは関係のない新しい束縛である。

#### 3.9.2 for と for mut の違い

| 構文 | ループ変数の可変性 | 説明 |
|------|----------------|------|
| `for i in 1..5` | 不変 | ループ本体で束縛を変更できない |
| `for mut i in 1..5` | 可変 | ループ本体で束縛を変更できる |

```yaoxiang
// 有効：各反復で新しい束縛するため、変更は不要
for i in 1..5 {
    print(i)  // i の値を読み取る
}

// 誤り：不変束縛のため変更不可
for i in 1..5 {
    i = i + 1  // 誤り：不変束縛は変更できない
}

// 有効：for mut を使用すると束縛を変更できる
for mut i in 1..5 {
    i = i + 1  // 許可
}
```

#### 3.9.3 シャドウイング検査

YaoXiang は変数のシャドウイングを禁止している。for ループ変数は外側のスコープの変数と同じ名前を使用できない：

```yaoxiang
// 誤り：i はすでに外部で宣言されている
i = 10
for i in 1..5 {
    print(i)
}

// 正しい：別の変数名を使用
i = 10
for j in 1..5 {
    print(j)
}
```

この規則はすべてのコードブロックに適用される。詳細については [4.3 シャドウイング規則](./modules.md#43-遮蔽規則) を参照。

#### 3.9.4 他の言語との比較

| 言語 | for ループ変数の意味 |
|------|---------------------|
| YaoXiang | 各反復で新しい束縛 |
| Rust | 同じ変数を変更（mut が必要） |
| Python | 同じ変数を変更（mut は不要） |
| C/C++ | 同じ変数を変更（ポインタまたは参照が必要） |

**設計理由**：YaoXiang が束縛セマンティクス採用している理由は：

1. **より自然な意味論**
   自然言語では、「集合内の各要素 x について」とは、各 x が独立した存在であることを意味する。YaoXiang の `for i in 1..5` は「1 から 5 までの各 i について」と読み、各反復の i はまったく新しい束縛であり、人間の直感と一致する。

2. **予期せぬ変更を回避**
   デフォルトで不変の束縛セマンティクスにより、ループ本体でループ変数を予期せぬ変更が発生する心配がない。複雑なループ本体内で誤って `i = ...` と書いて追跡困難なバグが発生する心配をしなくてよい。

3. **高性能ソリューションが簡単に実現可能**
   反復間で変数を再利用する必要がある場合（例：アキュムレータ、キャッシュ）、`for mut` を使用して可変束縛モードに切り替えることができる。これは暗黙的な共有状態よりも明確である—意図が構文で明示的に表現され、実行時の動作に隠されない。

### 3.10 spawn 文

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
SpawnFor    ::= Identifier '=' 'spawn' 'for' 'mut'? Identifier 'in' Expr '{' Expr '}'
SpawnStmt   ::= SpawnBlock | SpawnFor
```

**spawn ブロック**：并发範囲を明示的に宣言し、ブロック内の式が并发に実行される。

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

## 付録：構文クイックリファレンス

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
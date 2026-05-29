# 構文仕様

本ドキュメントでは、YaoXiang プログラミング言語の構文仕様を定義します。これには字句構造、構文規則、演算子の優先順位が含まれます。

---

## 第1章：字句構造

### 1.1 ソースファイル

YaoXiang ソースファイルは UTF-8 エンコーディングを使用する必要があります。ソースファイルの拡張子は通常 `.yx` です。

### 1.2 字句トークンの分類

| カテゴリ | 説明 | 例 |
|------|------|------|
| 識別子 | 文字またはアンダースコアで始まる | `x`, `_private`, `my_var` |
| キーワード | 言語定義の予約語 | `Type`, `pub`, `use` |
| リテラル | 固定値 | `42`, `"hello"`, `true` |
| 演算子 | 演算記号 | `+`, `-`, `*`, `/` |
| 区切り記号 | 構文区切り | `(`, `)`, `{`, `}`, `,` |

### 1.3 キーワード

YaoXiang は非常に少数のキーワードを定義しています：

```
pub    use    spawn
ref    mut    if     elif
else   match  while  for    return
break  continue as     in     unsafe
```

これらのキーワードは任意のコンテキストで特別な意味を持ち、識別子として使用することはできません。

### 1.4 予約語

| 予約語 | 型 | 説明 |
|--------|------|------|
| `Type` | Type | メタ型 |
| `true` | Bool | ブール真値 |
| `false` | Bool | ブール偽値 |
| `void` | Void | 空値 |
| `some(T)` | Option | Option 値ヴァリアント |
| `ok(T)` | Result | Result 成功ヴァリアント |
| `err(E)` | Result | Result エラーヴァリアント |

### 1.5 識別子

識別子は文字またはアンダースコアで始まり、後続の文字は文字、数字、またはアンダースコアにできます。識別子は大文字と小文字を区別します。

特殊識別子：
- `_` はプレースホルダーとして使用され、ある値を無視することを表します
- アンダースコアで始まる識別子はプライベートメンバーを表します

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
   複数行にまたがる可能 */
```

### 1.8 インデント規則

コードは4つのスペースを使用してインデントする必要があります。Tab 文字の使用は禁止です。これは強制的な構文規則です。

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
              | IfExpr
              | MatchExpr
              | Block
              | Lambda
```

### 2.2 演算子の優先順位

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

### 2.10 Lambda 式

```
Lambda      ::= '(' ParamList? ')' '=>' Expr
            |  '(' ParamList? ')' '=>' Block
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
              | LoopStmt
              | WhileStmt
              | ForStmt
```

### 3.2 変数宣言

```
LetStmt     ::= ('mut')? Identifier (':' TypeExpr)? '=' Expr
```

### 3.3 return 文

```
ReturnStmt  ::= 'return' Expr?
```

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

#### 3.9.1 意味：各イテレーションは新しいバインディング

YaoXiang の for ループの意味は従来の言語とは異なります：**各イテレーションは新しいバインディングであり、同じ変数を変更するのではありません**。

```yaoxiang
// 例：for i in 1..5
for i in 1..5 {
    print(i)
}
```

**実行プロセス**：

| イテレーション | ループ変数の動作 |
|------|----------------|
| 第1回 | 新しいバインディング `i = 1` を作成、ループ本体を実行、1 を印字 |
| 第2回 | 新しいバインディング `i = 2` を作成（前のバインディングは破棄済み）、ループ本体を実行、2 を印字 |
| 第3回 | 新しいバインディング `i = 3` を作成、ループ本体を実行、3 を印字 |
| 第4回 | 新しいバインディング `i = 4` を作成、ループ本体を実行、4 を印字 |
| 終了 | ループ本体終了、バインディング破棄 |

**重要なポイント**：各イテレーションの終了後、そのイテレーションで作成されたバインディングは破棄されます。次のイテレーションは完全に新しいバインディングであり、前のイテレーションのバインディングとは関係ありません。

#### 3.9.2 for と for mut の違い

| 構文 | ループ変数の可変性 | 説明 |
|------|----------------|------|
| `for i in 1..5` | 不変 | ループ本体内でバインディングを変更できない |
| `for mut i in 1..5` | 可変 | ループ本体内でバインディングを変更できる |

```yaoxiang
// 有効：各イテレーションで新しいバインディングを作成するため、変更は不要
for i in 1..5 {
    print(i)  // i の値を読み取る
}

// 誤り：不変バインディングのため、変更不可
for i in 1..5 {
    i = i + 1  // 誤り：不変バインディングは変更不可
}

// 有効：for mut を使用するとバインディングの変更を許可
for mut i in 1..5 {
    i = i + 1  // 許可
}
```

#### 3.9.3 シャドウイング検査

for ループ変数は、外側のスコープに既に存在する変数をシャドウできません：

```yaoxiang
// 誤り：i は既に外部で宣言されている
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

エラーコード：`E2013 - Cannot shadow existing variable`

#### 3.9.4 他の言語との比較

| 言語 | for ループ変数の意味 |
|------|------------------|
| YaoXiang | 各イテレーションで新しいバインディング |
| Rust | 同じ変数を変更（mut が必要） |
| Python | 同じ変数を変更（mut は不要） |
| C/C++ | 同じ変数を変更（ポインタまたは参照が必要） |

**設計理由**：YaoXiang がバインディング意味論を採用したのは：
1. 各イテレーションの終了後、ループ本体内の変数は破棄される
2. 次のイテレーションは完全に新しいバインディングである
3. これにより、イテレーション間の状態を考える必要がなくなり、より安全である

---

## 付録：構文早見表

### A.1 制御フロー

```
if Expr Block (elif Expr Block)* (else Block)?
match Expr { MatchArm+ }
while Identifier in Expr Block Expr Block
for
```

### A.2 match 構文

```
match value {
    pattern1 => expr1,
    pattern2 if guard => expr2,
    _ => default_expr,
}
```
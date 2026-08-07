# 構文仕様

本文書は YaoXiang プログラミング言語の構文仕様を定義する。字句構造、構文規則、演算子の優先順位を含む。

---

## 第一章：字句構造

### 1.1 ソースファイル

YaoXiang のソースファイルは UTF-8 エンコーディングを使用しなければならない。ソースファイルは通常
`.yx` 拡張子を持つ。

### 1.2 字句単位の分類

| カテゴリ   | 説明                             | 例                        |
| ---------- | -------------------------------- | ------------------------- |
| 識別子     | 文字またはアンダースコアで始まる | `x`, `_private`, `my_var` |
| キーワード | 言語の事前定義された予約語       | `Type`, `pub`, `use`      |
| リテラル   | 固定値                           | `42`, `"hello"`, `true`   |
| 演算子     | 演算記号                         | `+`, `-`, `*`, `/`        |
| 区切り文字 | 構文の区切り                     | `(`, `)`, `{`, `}`, `,`   |

### 1.3 キーワード

YaoXiang は最小限のキーワードしか定義しない：

```
pub    use    spawn
ref    mut    if     else
else   match  while  for    return
break  continue as     in     unsafe
```

これらのキーワードはあらゆるコンテキストで特別な意味を持ち、識別子として使用することはできない。

### 1.4 予約語

YaoXiang の「予約語」は 3 つのレベルに分かれ、それぞれパーサ（parser）と型検査器（type
checker）によって異なる段階で識別される：

#### 1.4.1 リテラル予約語

パーサが独立したトークンとして持つリテラル識別子は、通常の識別子として使用できない：

| 識別子  | 所属型 | 説明                                                                                              |
| ------- | ------ | ------------------------------------------------------------------------------------------------- |
| `Type`  | —      | メタ型キーワード                                                                                  |
| `true`  | Bool   | ブール真値                                                                                        |
| `false` | Bool   | ブール偽値                                                                                        |
| `void`  | Void   | Void リテラル（Unit 値）。小文字の `void` は値リテラル；大文字の `Void` は型名（§1.4.3 を参照）。 |

#### 1.4.2 コンストラクタ式

以下のコンストラクタはパターン照合と式のコンテキストでパーサによって識別される：

| コンストラクタ | 所属型 | 説明                    |
| -------------- | ------ | ----------------------- |
| `some(T)`      | Option | Option 値バリアント構築 |
| `ok(T)`        | Result | Result 成功バリアント   |
| `err(E)`       | Result | Result エラーバリアント |

#### 1.4.3 組み込み型名

以下の型名は型検査器によって事前登録されており、インポートなしでも型の位置で使用できる。パーサはこれらを通常の識別子として扱う——**予約語ではなく、局所束縛でシャドウイング可能（非推奨）**。

| 型名     | 論理対応     | 説明                                                                                                                             |
| -------- | ------------ | -------------------------------------------------------------------------------------------------------------------------------- |
| `Void`   | ⊤（真/Unit） | ゼロフィールドの積型。ちょうど 1 つの居住者を持つ（`void` リテラル、§1.4.1 を参照）。                                            |
| `Never`  | ⊥（偽/空型） | ゼロバリアントの和型、居住者ゼロ。`Never` 値を生成する式は存在しない。`Never <: T` はすべての `T` に対して成立する（爆発原理）。 |
| `Int`    | —            | 符号付き整数                                                                                                                     |
| `Float`  | —            | 浮動小数点数                                                                                                                     |
| `Bool`   | —            | ブール値：`true` / `false`                                                                                                       |
| `Char`   | —            | Unicode 文字                                                                                                                     |
| `String` | —            | 文字列                                                                                                                           |

### 1.5 識別子

識別子は文字またはアンダースコアで始まり、続く文字には文字、数字、アンダースコアが使用できる。識別子は大文字小文字を区別する。

特殊識別子：

- `_` はプレースホルダとして使用され、値を無視することを示す
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

#### 1.6.6 メンバーシップテスト

```
Membership  ::= Expr 'in' Expr
```

### 1.7 コメント

```
// 单行注释

/* 多行注释
   可以跨越多行 */
```

### 1.8 インデント規則

コードは 4 つのスペースでインデントしなければならず、Tab 文字の使用は禁止。これは強制的な構文規則である。

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

| 優先順位 | 演算子                      | 結合性 |
| -------- | --------------------------- | ------ |
| 1        | `()` `[]` `.` `?`           | 左結合 |
| 2        | `as`                        | 左結合 |
| 3        | 単項前置 `!` `-` `+`        | 右結合 |
| 4        | `*` `/` `%`                 | 左結合 |
| 5        | `+` `-`                     | 左結合 |
| 6        | `..`                        | 左結合 |
| 7        | `<<` `>>`                   | 左結合 |
| 8        | `&` `\|` `^`                | 左結合 |
| 9        | `==` `!=` `<` `>` `<=` `>=` | 左結合 |
| 10       | `and` `or`                  | 左結合 |
| 11       | `if...else`                 | 右結合 |
| 12       | `=` `+=` `-=` `*=` `/=`     | 右結合 |

> **単項前置演算子**（`!` `-`
> `+`）は強結合：呼び出しとメンバーアクセスより低く、すべての二項演算子より高い。したがって
> `!a == b` ≡ `(!a) == b`（Zig 式のセマンティクス）；`!`
> は純粋な単項演算であり、短絡制御フローには参加しない。 `and`/`or`
> キーワード（短絡）とは直交する（RFC-010 による規範定義）。

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
IfExpr      ::= 'if' Expr Block ('else' 'if' Expr Block)* ('else' Block)?
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

> **文の終了規則**：Stmt 間の区切りと改行の振る舞い（`;`
> による明示的区切り、改行による終了、継続行の例外、行頭の `(`/`[` は決して結合しない）は
> [RFC-038](../design/rfc/draft/038-statement-termination.md) によって定義される。

**統一セマンティクス**：すべての `{}` ブロックの return セマンティクスは一貫している：

| ブロック型  | return のセマンティクス | デフォルト戻り値 |
| ----------- | ----------------------- | ---------------- |
| 通常の `{}` | 値を返す                | Void             |
| `unsafe {}` | 型定義を返す            | Void             |
| `spawn {}`  | 結果を返す              | Void             |

**核心原則**：

- `{}` 内の `return` は常に内容を外側のスコープに返す
- デフォルトでは `return` がない場合 `Void` を返す
- 式の形式 `= expr` は直接値を返す

```yaoxiang
// 普通 {} 块：return 返回值
result = {
    x = compute()
    return x  // 返回值给上一作用域
}

// unsafe {} 块：return 返回类型定义
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void
    }
    return SqliteDb  // 返回类型定义给上一作用域
}

// spawn {} 块：return 返回结果
(a, b) = spawn {
    result1 = fetch("url1"),
    result2 = fetch("url2")
    return (result1, result2)  // 返回结果给上一作用域
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

`?` 演算子は後置演算子で、優先順位は `.` と同等である。`Result(T, E)` 型に対して：

- `Ok(v)` の場合は値 `v` を抽出して実行を続ける
- `Err(e)` の場合はエラーを上位へ伝播する（`return Err(e)`）

```yaoxiang
process: (data: Data) -> Result(Data, Error) = {
    validated = validate(data)?     // 成功时提取值，失败时向上传播
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

`ref`
は共有所有を作成する。コンパイラは自動的に Rc（単一タスク）または Arc（タスク間）を選択し、ユーザは実装の詳細を気にする必要はない。

```yaoxiang
data = ref heavy_data
spawn { use(data) }   // 跨任务：编译器自动选 Arc
```

### 2.14 unsafe 式

```
UnsafeExpr  ::= 'unsafe' Block
```

`unsafe` ブロックは不透明型の定義と生ポインタの操作に使用される。`return`
を使用して型定義を外側のスコープに返す。

**セマンティクス**：

- `unsafe {}` 内では型の定義と生ポインタの操作が可能
- 返された型は `unsafe {}` 外で使用可能
- 型のフィールドアクセスには unsafe 権限が必要

```yaoxiang
// 在 unsafe 块中定义不透明类型
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  // 裸指针
    }
    return SqliteDb
}

// SqliteDb 在 unsafe 块外可用
db = sqlite3_open("test.db")
```

### 2.15 スコープ

**基本規則**：

- 各 `{}` ブロックはスコープを作成する
- 内側のスコープは外側のスコープの変数にアクセス可能
- 外側のスコープは内側のスコープの変数にアクセスできない
- 変数宣言は「代入優先」原則に従う

```yaoxiang
// 块作用域
{
    x = 10
    // x 在此作用域内可见
}
// x 在此作用域外不可见

// 函数作用域
add: (a: Int, b: Int) -> Int = {
    result = a + b
    return result
}
// result 在函数外不可见
```

**変数宣言とシャドウイング**：

- `x = value`：スコープチェーンに沿って外側に x を検索し、見つかれば代入、見つからなければ新規宣言
- `mut x = value`：明示的な新規可変宣言、外側と同名は禁止
- 同じスコープ内では任意の名前は 1 回しか宣言できない

> **詳細定義**：スコープの完全な規則、変数宣言とシャドウイングのメカニズムについては
> [モジュールシステム仕様](./modules.md#第四章作用域) を参照。

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

**セマンティクス**：`return` はコードブロックから値を返すために使用される。`return`
がない場合、コードブロックはデフォルトで `Void` を返す。

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
IfStmt      ::= 'if' Expr Block ('else' 'if' Expr Block)* ('else' Block)?
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

#### 3.9.1 セマンティクス：各イテレーションは新しい値を束縛する

YaoXiang の for ループのセマンティクスは従来の言語とは異なる：**各イテレーションは新しい値を束縛し、同じ変数を変更するのではない**。

```yaoxiang
// 示例：for i in 1..5
for i in 1..5 {
    print(i)
}
```

**実行過程**：

| イテレーション | ループ変数の動作                                                          |
| -------------- | ------------------------------------------------------------------------- |
| 1 回目         | 新規束縛 `i = 1` を作成し、ループ本体を実行、1 を出力                     |
| 2 回目         | 新規束縛 `i = 2` を作成（前の束縛は破棄済み）、ループ本体を実行、2 を出力 |
| 3 回目         | 新規束縛 `i = 3` を作成し、ループ本体を実行、3 を出力                     |
| 4 回目         | 新規束縛 `i = 4` を作成し、ループ本体を実行、4 を出力                     |
| 終了           | ループ本体終了、束縛破棄                                                  |

**重要点**：各イテレーションの終了後、そのイテレーションで作成された束縛は破棄される。次のイテレーションは完全に新しい束縛であり、前のイテレーションの束縛とは一切関係がない。

#### 3.9.2 for と for mut の違い

| 構文                | ループ変数の可変性 | 説明                             |
| ------------------- | ------------------ | -------------------------------- |
| `for i in 1..5`     | 不変               | ループ本体内で束縛を変更できない |
| `for mut i in 1..5` | 可変               | ループ本体内で束縛を変更可能     |

```yaoxiang
// 合法：每次迭代绑定新值，不需要修改
for i in 1..5 {
    print(i)  // 读取 i 的值
}

// 错误：不可变绑定，不能修改
for i in 1..5 {
    i = i + 1  // 错误：不能修改不可变绑定
}

// 合法：使用 for mut 允许修改绑定
for mut i in 1..5 {
    i = i + 1  // 允许修改
}
```

#### 3.9.3 シャドウイング検査

YaoXiang は変数のシャドウイングを禁止する。for ループ変数は外側のスコープの変数と同名にできない：

```yaoxiang
// 错误：i 已经在外部声明
i = 10
for i in 1..5 {
    print(i)
}

// 正确：使用不同的变量名
i = 10
for j in 1..5 {
    print(j)
}
```

この規則はすべてのコードブロックに適用される。詳細は
[4.3 シャドウイング規則](./modules.md#43-遮蔽规则) を参照。

#### 3.9.4 他の言語との比較

| 言語     | for ループ変数のセマンティクス             |
| -------- | ------------------------------------------ |
| YaoXiang | 各イテレーションで新しい値を束縛           |
| Rust     | 同じ変数を変更（mut が必要）               |
| Python   | 同じ変数を変更（mut 不要）                 |
| C/C++    | 同じ変数を変更（ポインタまたは参照が必要） |

**設計理由**：YaoXiang が束縛セマンティクスを採用するのは以下の理由による：

1. **より自然なセマンティクスに合致する**
   自然言語では「集合の各要素 x について」は各 x が独立した存在であることを意味する。YaoXiang の
   `for i in 1..5`
   は「1 から 5 の各 i について」と読み、各イテレーションの i は完全に新しい束縛である。これは人間の直感的理解と一致する。

2. **予期しない変更を回避する**
   デフォルトで不変な束縛セマンティクスは、ループ本体内でループ変数を誤って変更できないことを意味する。複雑なループ本体内のどこかで誤って
   `i = ...` と書いてしまい、追跡困難なバグが発生する心配がない。

3. **高性能ソリューションへの近道**
   イテレーション間で変数を再利用する必要が実際にある場合（アキュムレータやキャッシュなど）、`for mut`
   宣言を使用すれば可変束縛モードに切り替えられる。これは暗黙の共有状態よりも明確である——意図が構文で明示的に表現され、実行時の動作に隠されない。

### 3.10 spawn 文

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
SpawnFor    ::= Identifier '=' 'spawn' 'for' 'mut'? Identifier 'in' Expr '{' Expr '}'
SpawnStmt   ::= SpawnBlock | SpawnFor
```

**spawn ブロック**：並行領域を明示的に宣言し、ブロック内の式は並行に実行される。

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
if Expr Block (else if Expr Block)* (else Block)?
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

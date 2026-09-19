---
title: 'RFC-007: 関数定義構文統一案'
issue: '#131'
status: '承認済み'
author: '沫郁酱'
created: '2025-01-05'
updated: '2026-09-15（返却セマンティクスを RFC-010a で訂正）'
---

# RFC-007: 関数定義構文統一案

> **訂正（2026-09-15、RFC-010a）**：本文ではかつて「コードブロック内では `return`
> を使って値を返す必要があり、`return` がない場合はデフォルトで `Void`
> を返す」としていた。この記述は廃止された——**ブロックの値 = 末尾式**であり、`return` は `Never`
> 型の非局所脱出である。詳細は [RFC-010a](./010a-tail-expression-and-return.md)
> を参照。本 RFC の関数形式定義（完全形式 /
> Lambda 頭省略 / 署名省略 / 空引数最簡の4種の書き方）は**変更なし**であり、「早期リターン」のセマンティクスも元より本 RFC と整合している。
>
> **補足（裁定 C）**：本文の「空引数最簡」`name = { ... }`
> は RFC-010 の「値を持つブロック」構文と位置が競合するため、**注釈優先・既定で関数**と裁定された——注釈なしの場合、本文の「空引数最簡」が成立し（関数として扱われ）、`Fn`
> 以外の注釈を書いた場合のみブロック値（`x: Int = { ... }`）となる。RFC-010a 付録D を参照。
>
> **関連補足**：関数本体（`{ ... }` コードブロック）内の文の終端と改行ルール（`;`
> による明示的な区切り、改行による終端、続行の例外）は
> [RFC-038（ドラフト）](../draft/038-statement-termination.md)
> で定義されており、本 RFC の範囲外である。

## 概要

本 RFC は YaoXiang 言語の**関数定義構文**の最終案を定める。統一構文
`name: (params) -> Return = body` を採用し、RFC-010 の `name: type = value`
モデルと完全に一致させる。

曖昧さを避けるため：関数が入力引数を持つ場合、引数型は「署名」または「lambda 頭」の少なくとも一方で明示的に注釈付けしなければならず、両方を省略した場合は拒否される。

コードブロック `{ ... }` の値は**末尾式**によって与えられる；`return` は `Never`
型の非局所脱出である（[RFC-010a](./010a-tail-expression-and-return.md) 参照）。式形式 `= expr`
は直接値を与える。

## 動機

### なぜこの機能が必要か？

1. **構文の一貫性**：旧構文の歴史的負の遺産を除去し、スタイルを統一する
2. **簡潔性**：HM アルゴリズムによる自動型推論で、ボイラープレートコードを削減
3. **型安全性**：HM アルゴリズムにより型安全性を保証し、推論できない場合のみ明示的に注釈
4. **言語の成熟度**：HM アルゴリズムは現代的な関数型言語の成熟した手法である

### 統一構文モデル

**中核原則**：`name: Signature = LambdaBody`

- **完全形式**：署名（引数名 + 型 + `->` + 戻り型を含む） + Lambda 頭（引数名を含む）
- **短縮ルール**：曖昧さを生まない範囲で極力省略する
  - `->` は省略不可（関数型の目印であり、省略するとタプルとして解釈される）
  - **入力引数がある場合**、引数型は署名または lambda 頭の少なくとも一方で明示的に出現しなければならない
  - Lambda 頭は省略可能 → 署名で既に引数名と型が宣言されている場合
  - 戻り型は明示的に注釈可能であり、推論可能なら省略可能

```yaoxiang
# 完整形式（签名完整 + Lambda头完整）
add: (a: Int, b: Int) -> Int = (a, b) => a + b

# 简写：省略 Lambda 头（签名已声明参数）
add: (a: Int, b: Int) -> Int = a + b

# 简写：省略签名（lambda 头标注参数类型）
add = (a: Int, b: Int) => a + b

# ❌ 错误：两边都没标注参数类型
# add = (a, b) => a + b
```

### 設計目標

```yaoxiang
# === 完整形式 ===
add: (a: Int, b: Int) -> Int = (a, b) => { a + b }

# === 简写形式 ===
add: (a: Int, b: Int) -> Int = a + b                 # 省略 Lambda 头
add = (a: Int, b: Int) => a + b                      # 省略签名

# === 空参函数 ===
main: () -> Void = () => { println("Hello") }          # 完整形式
main: () -> Void = { println("Hello") }                # 省略 Lambda 头
main: () -> Void = { println("Hello") }                            # 最简形式（推断为 () -> Void）

# === 泛型函数（使用 RFC-010 统一语法）===
identity: (T: Type) -> ((x: T) -> T) = (x) => x         # 完整形式
identity: (T: Type) -> ((x: T) -> T) = x                # 省略 Lambda 头
identity = (x: T) => x                                  # 省略签名（lambda 头标注类型）

# === 递归函数 ===
factorial: (n: Int) -> Int = (n) => {
    if n <= 1 { return 1 } else { return n * factorial(n - 1) }
}
```

### 構文ルール

| シナリオ          | 構文                                                   | 説明                    |
| ----------------- | ------------------------------------------------------ | ----------------------- |
| **完全形式**      | `name: (a: Type, b) -> Ret = (a, b) => { return ... }` | 署名 + Lambda 頭が完全  |
| **Lambda 頭省略** | `name: (a: Type, b: Type) -> Ret = { ... }`            | 署名で引数を宣言済み    |
| **署名省略**      | `name = (a: Type, b: Type) => { ... }`                 | lambda 頭で引数型を注釈 |
| **空引数完全**    | `name: () -> Void = () => { return ... }`              | 空引数関数の完全形      |
| **空引数短縮**    | `name: () -> Void = { return ... }`                    | Lambda 頭を省略         |
| **空引数最簡**    | `name = { return ... }`                                | 無引数・無戻り最簡      |

**注意**：コードブロック `{ ... }` の値は**末尾式**によって与えられる（唯一の出口）；`return` は
`Never` 型の非局所脱出であり、最も近い関数境界から脱出する。式形式 `= expr` は直接値を与える。詳細は
[RFC-010a](./010a-tail-expression-and-return.md) を参照。

**注意**：`->` は関数型の目印であり、省略不可（省略するとタプルとして解釈される）。

**重要**：`if` 式は分岐を中括弧 `{}` で囲み、`then/else` キーワードはサポートされない：

```yaoxiang
# 正确：使用花括号
if n <= 1 { return 1 } else { return n * factorial(n - 1) }

# 错误：不支持 then/else 关键字
# if n <= 1 then return 1 else return n * factorial(n - 1)
```

## 提案

### HM アルゴリズムと高階多相のサポート

**中核機能**：HM アルゴリズムはジェネリック型注釈を通じて高階多相（Higher-rank
polymorphism）をサポートする。

**設計原理**：

- **高階関数**：関数を引数として渡す際、ジェネリック制約によりその関数型を束縛する必要がある
- **型注釈の形式**：`(T: Type) -> ((f: (T) -> T, x: T) -> T)` — ジェネリック引数による関数型の制約
- **HM の動作フロー**：ジェネリック引数により関数型を推論し、多相的な関数合成を実現

**説明用サンプル**：

```yaoxiang
# ✅ 支持高阶多态：泛型约束函数类型参数
call_twice: (T: Type) -> ((f: (T) -> T, x: T) -> T) = {
    return f(f(x))
}
# 使用：call_twice((x) => x + 1, 5)  # 推断 T=Int

compose: (A: Type, B: Type, C: Type) -> ((f: (B) -> C, g: (A) -> B, x: A) -> C) = {
    return f(g(x))
}
# 使用：compose((x) => x * 2, (x) => x + 1, 5)  # 推断 A=Int, B=Int, C=Int

# ❌ 不支持：缺少泛型约束的高阶函数
# bad_hof: (f, x) => f(f(x))  # HM无法推断，缺少泛型参数
```

**HM 推論プロセス**：

1. 高階関数の引数を識別：`f: (T) -> T`
2. ジェネリック制約を生成：`(T: Type)`
3. ジェネリックインスタンス化により具体型を推論
4. 多相的な関数合成を実現

### Lambda 式の構文ルール

**重要なルール**：コードブロック `{ ... }`
の値は**末尾式**によって与えられる（唯一の出口）；`return` は `Never`
型の非局所脱出であり、最も近い関数境界から脱出する。式形式 `= expr` は直接値を与える。詳細は
[RFC-010a](./010a-tail-expression-and-return.md) を参照。

| 構文形式               | 構文             | 値の出口                            |
| ---------------------- | ---------------- | ----------------------------------- |
| **コードブロック形式** | `{ statements }` | 末尾式（空ブロック `{}` は `Void`） |
| **式形式**             | `expression`     | 式の値                              |
| **`return`**           | `return e`       | 関数からの非局所脱出、型は `Never`  |

**例**：

```yaoxiang
main: () -> Void = { println("Hello") }         # 尾表达式为 Void
add: (a: Int, b: Int) -> Int = { a + b }        # 尾表达式给出值
empty: () -> Void = {}                          # 空块 → Void

# 提前返回：使用 return
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

# 表达式形式：直接给出值
add: (a: Int, b: Int) -> Int = a + b            # 正确：表达式形式
main: () -> Void = println("Hello")               # 正确：表达式形式
```

**中核となる考え方**：

1. 関数定義は HM アルゴリズムにより型推論を行い、極力推論する。推論できない場合は明示的にエラー
2. **HM アルゴリズムの動作原理**：演算子の型制約や関数呼び出し関係などの文脈情報から自動的に型を推論
3. **ジェネリクスのサポート**：多相関数はジェネリック構文 `(T: Type)`
   で型引数を明示的に制約する（RFC-010/011）
4. **推論の境界**：戻り型とローカル変数は推論可能；有引数関数の引数型は明示的な注釈が必要（署名または lambda 頭のいずれか）
5. 空引数・無戻り関数は `name: () -> Void = { ... }` を使用し、RFC-010 と統一
6. 旧構文は廃止し、移行ツールを提供

**型推論の例**：

```yaoxiang
# 泛型函数：显式类型参数（使用RFC-010统一语法）
identity: (T: Type) -> ((x: T) -> T) = x
map: (T: Type, R: Type) -> ((f: (T) -> R, list: List(T)) -> List(R)) = {
    result = List(R)()
    for item in list { result.push(f(item)) }
    return result
}

# 多态函数：通过显式泛型约束定义（RFC-010/011）
add: (T: Add) -> ((a: T, b: T) -> T) = a + b
print_sum: (a: Int, b: Int) -> Void = { println(a + b) }  # 推断为 (Int, Int) -> Void

# 高阶多态：通过泛型类型注解实现HM支持高阶多态
call_twice: (T: Type) -> ((f: (T) -> T, x: T) -> T) = { return f(f(x)) }
compose: (A: Type, B: Type, C: Type) -> ((f: (B) -> C, g: (A) -> B, x: A) -> C) = { return f(g(x)) }
```

```yaoxiang
# === 函数定义：HM算法类型推断 ===

# 标准函数：HM算法推断返回类型（参数类型需显式）
add = (a: Int, b: Int) => a + b            # 推断为 (a: Int, b: Int) -> Int
main: () -> Void = { println("Hello") }                # 推断为 () -> Void

# 部分显式参数：HM算法推断剩余部分
print_sum: (a: Int, b: Int) -> Void = { println(a + b) }  # 推断为 (Int, Int) -> Void
greet: (name: String) -> Void = { println("Hello " + name) }  # 推断为 (String) -> Void

# 泛型函数：明确约束多态类型参数（使用RFC-010统一语法）
identity: (T: Type) -> ((x: T) -> T) = x
map: (T: Type, R: Type) -> ((f: (T) -> R, list: List(T)) -> List(R)) = {
    # 实现 map 函数
    return List(R)()
}

# 递归函数：通过HM算法和递归约束推断
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 } else { return n * factorial(n - 1) }
}

# === 变量赋值：HM算法类型推断 ===

# 显式类型
x: Int = 42

# HM算法自动推断为 Int
y = 42                               # 推断为 Int

# HM算法自动推断为 String
name = "YaoXiang"                    # 推断为 String

# HM算法自动推断为 Float
pi = 3.14159                         # 推断为 Float
```

**HM 型推論ルール**：

| シナリオ          | 構文                                              | 省略可能部分 | 例                      |
| ----------------- | ------------------------------------------------- | ------------ | ----------------------- |
| **完全形式**      | `name: (a: Type, b: Type) -> Ret = (a, b) => ...` | なし         | 署名 + Lambda 頭が完全  |
| **Lambda 頭省略** | `name: (a: Type, b: Type) -> Ret = ...`           | Lambda 頭    | 署名で引数を宣言済み    |
| **署名省略**      | `name = (a: Type, b: Type) => ...`                | 署名         | lambda 頭が引数型を提供 |
| **戻り Ret 省略** | `name: (a: Type, b: Type) -> = ...`               | 戻り型       | HM が戻り型を推論       |
| **空引数完全**    | `name: () -> Void = () => { ... }`                | なし         | 空引数関数の完全形      |
| **空引数短縮**    | `name: () -> Void = { ... }`                      | Lambda 頭    | `() =>` を省略          |
| **空引数最簡**    | `name = { ... }`                                  | すべて       | 無引数・無戻り最簡      |
| **変数代入**      | `name = value`                                    | 型           | HM が型を推論           |
| **明示的変数**    | `name: Type = value`                              | なし         | 明示的な型注釈          |

**中核原則**：

- `->` は関数型の目印であり、省略不可（省略するとタプルとして解釈される）
- 戻り型 `Ret` は省略可能であり、HM が関数本体から推論する
- 入力引数が存在する場合、引数型は明示的に出現しなければならない（署名または lambda 頭のいずれか）
- その他の部分は推論可能かつ曖昧さを生まない場合に省略可能
- 暗黙の型変換は行わず、JavaScript のような混乱を避ける

## 詳細設計

### 構文糖の展開

省略の有無に関わらず、最終的には統一中間表現に正規化される：

```rust
// 完整形式
add: (a: Int, b: Int) -> Int = (a, b) => a + b

// 展开后 IR
let add: (Int, Int) -> Int = |a: Int, b: Int| -> Int {
    return a + b
};

// 省略 Lambda 头
add: (a: Int, b: Int) -> Int = a + b

// 展开后 IR（与完整形式相同）
let add: (Int, Int) -> Int = |a: Int, b: Int| -> Int {
    return a + b
};

// 省略签名（lambda 头标注参数类型）
add = (a: Int, b: Int) => a + b

// 展开后 IR
let add: (Int, Int) -> Int = |a: Int, b: Int| -> Int {
    a + b
};
```

### 構文定義

```bnf
function_def ::= identifier ':' type_expr '=' expression
               | identifier '=' expression
               | identifier '=' block                    # 最简形式：无参无返回

identifier ::= [a-zA-Z_][a-zA-Z0-9_]*

type_expr ::= identifier                     # 类型引用
       | '()'                          # 空类型
       | '(' parameters ')' '->' type_expr   # 函数类型（参数名在签名中）
       | type_expr '->' type_expr            # 简单函数类型
       | identifier '(' type_expr (',' type_expr)* ')'  # 类型应用

expression ::= '(' parameters ')' '=>' block
             | '(' ')' '=>' block
             | '(' parameters ')' '=>' expression

parameters ::= parameter (',' parameter)*
parameter ::= identifier                # 类型推断
            | identifier ':' type_expr      # 部分显式类型

block ::= '{' statement (',' statement)* '}'
        | expression

statement ::= identifier ':' expression  # 赋值语句
           | expression                  # 表达式语句（执行但不返回）
           | 'return' expression         # 返回语句（返回指定值）

# 注意：代码块内必须使用 return 返回值；无 return 时默认返回 Void
# 例如：{ return 1 + 1 } 返回 Int；{ println("Hello") } 返回 Void
# 注意：泛型参数使用 (T: Type) 语法，作为函数类型的一部分，无需独立 BNF 规则
```

### エラー処理

```yaoxiang
# === 编译错误示例 ===

# 错误1：代码块返回类型不匹配
add: (a: Int, b: Int) -> Int = { println(a + b) }
// 错误：块内无 return，默认返回 Void，但签名期望 Int
// 正确：add: (a: Int, b: Int) -> Int = a + b
// 或者：add: (a: Int, b: Int) -> Int = { return a + b }

# 错误2：使用未声明的类型参数
identity: (x: T) -> T = x
// 错误：T 未声明；需要显式泛型参数（RFC-010）
// 正确：identity: (T: Type) -> ((x: T) -> T) = x

# 正确：HM算法推断返回类型
double = (x: Int) => x + x

# 完整形式（逐步简写）
double: (x: Int) -> Int = (x) => x + x                # 完整
double: (x: Int) -> Int = x + x                       # 省略 Lambda 头
double = (x: Int) => x + x                            # 省略返回类型（HM 推断返回）
# double = (x) => x + x                               # ❌ 参数类型不允许两边都省略
```

## トレードオフ

### 利点

- **構文の統一**：`name: Signature = LambdaBody` モデルがあらゆるシナリオをカバー
- **柔軟な短縮**：任意の部分は HM で推論可能なら省略可能
- **型安全性**：HM アルゴリズムが型安全性を保証し、暗黙の型変換を回避
- **再帰のサポート**：HM アルゴリズムと再帰制約により自動的に型を推論
- **ゼロ負担**：完全形から最簡形までスムーズに遷移

### 欠点

- **移行コスト**：旧コードは移行ツールによる変換が必要
- **学習コスト**：「完全形 + 任意の短縮」モデルの理解が必要

## 代替案

| 案                    | 説明                                    | 採用しなかった理由                             |
| --------------------- | --------------------------------------- | ---------------------------------------------- |
| HM アルゴリズム型推論 | Hindley-Milner アルゴリズムによる型推論 | ✅ **採用済み**、現代的な関数型言語の標準      |
| 明示的な型宣言        | すべての型を明示的に書く                | 簡潔な構文の原則に反し、ボイラープレートが増加 |
| 旧構文の維持          | 新旧両方の構文をサポート                | 構文の分裂により、保守コストが高い             |
| `fn` キーワード       | 関数と変数を区別するために `fn` を導入  | 「関数はすなわち lambda」という設計に反する    |

## 実装戦略

### 段階分け

1. **Phase 1: 構文解析と HM アルゴリズム**（v0.3）
   - 新構文 `name = lambda` + HM アルゴリズム型推論を実装
   - 空引数・無戻りのデフォルト補完を実装

2. **Phase 2: 移行ツール**（v0.3）
   - `yaoxiang-migrate --old-to-new` ツールを開発
   - 旧構文のコードを自動変換

3. **Phase 3: 検証とドキュメント**（v0.3）
   - 旧コードの移行完了検証
   - ドキュメントの更新

### 移行ツール

```bash
# 迁移单个文件
yaoxiang-migrate --old-to-new src/main.yaoxiang

# 迁移整个项目
yaoxiang-migrate --old-to-new --recursive src/

# 预览迁移（不修改文件）
yaoxiang-migrate --old-to-new --dry-run src/main.yaoxiang
```

移行ルール：

```yaoxiang
# 旧语法
add(Int, Int) -> Int = (a, b) => { a + b }
main() -> Int = { println("Hello"); 0 }
main() = { println("Hello") }

# === 新语法：完整形式（签名完整 + Lambda 头完整）===
add: (a: Int, b: Int) -> Int = (a, b) => a + b
main: () -> Void = () => { println("Hello") }

# === 简写：省略 Lambda 头 ===
add: (a: Int, b: Int) -> Int = a + b
main: () -> Void = { println("Hello") }

# === 简写：HM 推断 ===
add = (a: Int, b: Int) => a + b              # 推断为 (a: Int, b: Int) -> Int
main: () -> Void = { println("Hello") }                  # 推断为 () -> Void

# === 最简形式 ===
main: () -> Void = {                                      # 等价于 main: () -> Void = { ... }
    println("Hello")
}
```

### 依存関係

- 外部依存なし
- 単独で実装可能

### リスク

| リスク       | 影響                     | 緩和策                                           |
| ------------ | ------------------------ | ------------------------------------------------ |
| 移行漏れ     | 旧コードのコンパイル失敗 | 移行ツールを提供し、すべての旧構文パターンを網羅 |
| パーサのバグ | 構文解析の不安定さ       | 十分なテストカバレッジ                           |

## オープンな問題

> 以下の問題は設計中に解決済みであり、付録A に記録されている。

- ~~Q1: `main() = body` のような極簡記法を保持すべきか？~~ → 解決済み：`main: () -> Void = { ... }` として保持
- ~~Q2: 関数名の後の `:` を保持するか？~~
  → 解決済み：保持は任意；但し有引数関数は引き続き署名または lambda 頭で引数型を注釈する必要がある
- ~~Q3: HM アルゴリズムは引数型の推論をサポートするか？~~
  → 解決済み：戻り値/ローカルは推論可能；有引数関数の引数型は明示的な注釈が必要
- ~~Q4: `fn` キーワードを導入するか？~~ → 解決済み：導入しない、関数はすなわち lambda
- ~~Q5: 旧コードの移行戦略は？~~ → 解決済み：`yaoxiang-migrate` ツールを提供
- ~~Q6: ジェネリック関数の使い方は？~~ → 解決済み：RFC-010 統一構文 `(T: Type)` を使用

---

## 付録

### 付録A：各言語の関数定義構文参考

| 言語         | 構文スタイル                                        | 特徴                                 |
| ------------ | --------------------------------------------------- | ------------------------------------ |
| Rust         | `fn add(a: i32, b: i32) -> i32 { ... }`             | キーワード + 型注釈                  |
| Haskell      | `add a b = ...` / `add :: Int -> Int -> Int`        | 型署名が分離                         |
| OCaml        | `let add a b = ...`                                 | 引数型は省略可能                     |
| MoonBit      | `fn add(a: Int, b: Int): Int { ... }`               | 簡潔な型注釈                         |
| TypeScript   | `const add = (a: number, b: number): number => ...` | Lambda スタイル                      |
| Scala        | `def add(a: Int, b: Int): Int = { ... }`            | `def` キーワード                     |
| **YaoXiang** | `name = (a: Int, b: Int) => a + b`                  | **関数 = lambda、HM が戻り値を推論** |

### 付録B：設計決定の記録

| 決定            | 決定内容                                                     | 日付       | 記録者  |
| --------------- | ------------------------------------------------------------ | ---------- | ------- |
| 構文スタイル    | 新構文 `name: (params) -> Return = body` + HM 推論           | 2026-02-03 | @沫郁酱 |
| 引数の位置      | 引数名は署名で宣言し、RFC-010 と統一                         | 2026-02-03 | @沫郁酱 |
| デフォルト補完  | 空引数関数は署名を省略可能、空ブロック `{}` は `Void` と推論 | 2026-02-03 | @沫郁酱 |
| 型推論          | HM アルゴリズムにより自動推論、推論できない場合は明示        | 2026-01-06 | @沫郁酱 |
| 旧構文          | 廃止し、移行ツールを提供                                     | 2026-01-06 | @沫郁酱 |
| `fn` キーワード | 導入しない                                                   | 2026-01-06 | @沫郁酱 |
| 再帰宣言        | HM アルゴリズムと再帰制約により自動推論                      | 2026-01-06 | @沫郁酱 |

### 付録C：用語集

| 用語             | 定義                                                                                                   |
| ---------------- | ------------------------------------------------------------------------------------------------------ |
| HM アルゴリズム  | Hindley-Milner 型推論アルゴリズム。関数と変数の型を自動的に推論する                                    |
| ジェネリクス     | 型引数 `(T: Type)` により多相関数を制約する（例：`identity: (T: Type) -> ((x: T) -> T) = x`、RFC-010） |
| デフォルト型補完 | 空引数・無戻り関数が `-> Void` を省略した場合、コンパイラが自動的に補完する                            |
| 構文糖           | コードを読みやすくするための構文の簡略化記法                                                           |
| 正規化           | 構文形式を統一内部表現に変換すること                                                                   |
| 関数 = lambda    | 関数は本質的に lambda 変数であり、型は HM アルゴリズムにより自動推論される                             |

---

## 参考文献

- [MoonBit 言語設計](https://moonbitlang.com/)
- [Rust 関数構文](https://doc.rust-lang.org/book/ch03-03-how-functions-work.html)
- [Haskell 型システム](https://www.haskell.org/tutorial/patterns.html)
- [OCaml 型推論](https://v2.ocaml.org/manual/)

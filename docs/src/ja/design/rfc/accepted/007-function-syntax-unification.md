---
title: 'RFC-007: 関数定義構文の統一案'
issue: '#131'
status: '承認済み'
author: '沫郁酱'
created: '2025-01-05'
updated: '2026-09-15'
---

# RFC-007: 関数定義構文の統一案

> **関連**：「空パラメータ最簡」`name = { ... }`とRFC-010の「値を持つブロック」構文の位置が重なるところでは、**内容が型を決定する**ことと裁定する——`=>`は常に関数、`Fn`注釈は関数、`Fn`注釈なしはブロック値、注釈がない場合は内容で推論する。[RFC-010a](./010a-tail-expression-and-return.md)
> 付録Dを参照。
>
> **関連補足**：関数本体（`{ ... }`コードブロック）内の文の終端と改行規則（`;`明示的区切り、改行終端、継続行例外）は[RFC-038（ドラフト）](./038-statement-termination.md)で定義されており、本RFCでは扱わない。

## 概要

本RFCはYaoXiang言語の**関数定義構文**の最終案を確定する。統一構文
`name: (params) -> Return = body`を使用し、RFC-010の`name: type = value`モデルと完全に一致する。

曖昧さを避けるため：関数が入力パラメータを持つ場合、パラメータ型は「シグネチャ」または「lambda頭」の少なくとも一方で明示的に注釈を付ける必要がある；両方を省略すると拒否される。

コードブロック`{ ... }`の値は**末尾式**で与えられる；`return`は`Never`型の非局所的脱出である（[RFC-010a](./010a-tail-expression-and-return.md)参照）。式形式`= expr`は直接値を与える。

## 動機

### なぜこの特性が必要か？

1. **構文の一貫性**：旧構文の歴史的負債を排除し、スタイルを統一する
2. **簡潔性**：HMアルゴリズムが自動的に型を推論し、ボイラープレートを削減する
3. **型安全**：HMアルゴリズムが型安全を保証し、推論できない場合のみ明示的に注釈を付ける
4. **言語の成熟度**：HMアルゴリズムは現代的な関数型言語の成熟した解である

### 統一構文モデル

**核心原則**：`name: Signature = LambdaBody`

- **完全形式**：シグネチャ（パラメータ名 + 型 + `->` + 戻り型を含む） +
  Lambda頭（パラメータ名を含む）
- **省略規則**：曖昧さを導入しない範囲でできるだけ省略する
  - `->`は省略できない（関数型の目印であり、省略するとタプルとして解析される）
  - **入力パラメータがある場合**、パラメータ型はシグネチャまたはlambda頭の少なくとも一方で明示的に出現しなければならない
  - Lambda頭は省略可能 → シグネチャが既にパラメータ名と型を宣言している場合
  - 戻り型は明示的に注釈可能、推論可能な場合は省略可能

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

### 構文規則

| 場面                 | 構文                                                   | 説明                             |
| -------------------- | ------------------------------------------------------ | -------------------------------- |
| **完全形式**         | `name: (a: Type, b) -> Ret = (a, b) => { return ... }` | シグネチャ + Lambda頭完全        |
| **Lambda頭省略**     | `name: (a: Type, b: Type) -> Ret = { ... }`            | シグネチャがパラメータを宣言済み |
| **シグネチャ省略**   | `name = (a: Type, b: Type) => { ... }`                 | lambda頭がパラメータ型を注釈     |
| **空パラメータ完全** | `name: () -> Void = () => { return ... }`              | 空パラメータ関数完全             |
| **空パラメータ省略** | `name: () -> Void = { return ... }`                    | Lambda頭を省略                   |
| **空パラメータ最簡** | `name = { return ... }`                                | パラメータなし戻りなし最簡       |

**注意**：コードブロック`{ ... }`の値は**末尾式**で与えられる（唯一の出口）；`return`は`Never`型の非局所的脱出であり、最近接の関数境界から抜け出す。式形式`= expr`は直接値を与える。詳細については[RFC-010a](./010a-tail-expression-and-return.md)を参照。

**注意**：`->`は関数型の目印であり、省略できない（省略するとタプルとして解析される）。

**重要**：`if`式は波括弧`{}`で分岐を囲み、`then/else`キーワードはサポートされない：

```yaoxiang
# 正确：使用花括号
if n <= 1 { return 1 } else { return n * factorial(n - 1) }

# 错误：不支持 then/else 关键字
# if n <= 1 then return 1 else return n * factorial(n - 1)
```

## 提案

### HMアルゴリズムと高階多相のサポート

**核心特性**：HMアルゴリズムはgenerics型注釈を通じて高階多相（Higher-rank
polymorphism）をサポートする

**設計原理**：

- **高階関数**：関数を引数として渡す場合、genericsで関数型を制約する必要がある
- **型注釈形式**：`(T: Type) -> ((f: (T) -> T, x: T) -> T)` - generics引数が関数型を制約
- **HMのワークフロー**：generics引数を通じて関数型を推論し、多相関数の合成を実現

**例の説明**：

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

**HM推論過程**：

1. 高階関数の引数を識別：`f: (T) -> T`
2. generics制約を作成：`(T: Type)`
3. genericsインスタンス化を通じて具体的な型を推論
4. 多相関数の合成を実現

### Lambda式構文規則

**重要規則**：コードブロック`{ ... }`の値は**末尾式**で与えられる（唯一の出口）；`return`は`Never`型の非局所的脱出であり、最近接の関数境界から抜け出す。式形式`= expr`は直接値を与える。詳細については[RFC-010a](./010a-tail-expression-and-return.md)を参照。

| 構文形式               | 構文             | 値の出口                          |
| ---------------------- | ---------------- | --------------------------------- |
| **コードブロック形式** | `{ statements }` | 末尾式（空ブロック`{}`は`Void`）  |
| **式形式**             | `expression`     | 式の値                            |
| **`return`**           | `return e`       | 関数から非局所的脱出、型は`Never` |

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

**核心思想**：

1. 関数定義はHMアルゴリズムによって型推論され、推論できる場合はできるだけ推論し、推論できない場合は明示的にエラーを報告する
2. **HMアルゴリズムの動作原理**：演算子の型制約や関数呼び出し関係などのコンテキスト情報から自動的に型を推論する
3. **genericsサポート**：多相関数はgenerics構文`(T: Type)`で明確に型引数を制約する（RFC-010/011）
4. **推論の境界**：戻り型とローカル変数は推論可能；有引数関数のパラメータ型は明示的に注釈が必要（シグネチャまたはlambda頭のいずれか）
5. 空パラメータ戻りなし関数は`name: () -> Void = { ... }`を使用し、RFC-010と統一
6. 旧構文は廃止し、移行ツールを提供する

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

**HM型推論規則**：

| 場面                 | 構文                                              | 省略可能部分 | 例                               |
| -------------------- | ------------------------------------------------- | ------------ | -------------------------------- |
| **完全形式**         | `name: (a: Type, b: Type) -> Ret = (a, b) => ...` | なし         | シグネチャ + Lambda頭完全        |
| **Lambda頭省略**     | `name: (a: Type, b: Type) -> Ret = ...`           | Lambda頭     | シグネチャがパラメータを宣言済み |
| **シグネチャ省略**   | `name = (a: Type, b: Type) => ...`                | シグネチャ   | lambda頭がパラメータ型を提供     |
| **戻り型Ret省略**    | `name: (a: Type, b: Type) -> = ...`               | 戻り型       | HMが戻り型を推論                 |
| **空パラメータ完全** | `name: () -> Void = () => { ... }`                | なし         | 空パラメータ関数完全             |
| **空パラメータ省略** | `name: () -> Void = { ... }`                      | Lambda頭     | `() =>`を省略                    |
| **空パラメータ最簡** | `name = { ... }`                                  | 全部         | パラメータなし戻りなし最簡       |
| **変数代入**         | `name = value`                                    | 型           | HMが型を推論                     |
| **明示的変数**       | `name: Type = value`                              | なし         | 明示的型注釈                     |

**核心原則**：

- `->`は関数型の目印であり、省略できない（省略するとタプルとして解析される）
- 戻り型`Ret`は省略可能、HMが関数本体から推論する
- 入力パラメータがある場合、パラメータ型は明示的に出現しなければならない（シグネチャまたはlambda頭のいずれか）
- その他の部分は推論可能で曖昧さを導入しない場合に省略可能
- 暗黙的な型変換はなし、JavaScriptのような混乱を避ける

## 詳細設計

### 糖衣構文の展開

省略の有無に関わらず、最終的にはすべて統一中間表現に正規化される：

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

### メリット

- **構文の統一**：`name: Signature = LambdaBody`モデルがすべての場面をカバー
- **柔軟な省略**：任意の部分はHMで推論可能なら省略可能
- **型安全**：HMアルゴリズムが型安全を保証し、暗黙的な型変換を避ける
- **再帰サポート**：HMアルゴリズムと再帰制約が自動的に型を推論
- **ゼロ負担**：完全から最簡までスムーズな移行

### デメリット

- **移行コスト**：旧コードは移行ツールによる変換が必要
- **学習コスト**：「完全形式 + 任意の省略」モデルの理解が必要

## 代替案

| 案                   | 説明                               | 選択しない理由                                 |
| -------------------- | ---------------------------------- | ---------------------------------------------- |
| HMアルゴリズム型推論 | Hindley-Milnerアルゴリズムで型推論 | ✅ **採用済み**、現代的な関数型言語の標準      |
| 明示的型宣言         | すべての型を明示的に記述           | 簡素化構文の原則に違反、ボイラープレートを増加 |
| 旧構文の保持         | 新旧構文を同時にサポート           | 構文の分裂、保守コスト高                       |
| fnキーワード         | fnを導入して関数と変数を区別       | 「関数はlambdaである」という設計に違反         |

## 実装戦略

### フェーズ区分

1. **フェーズ1：構文解析とHMアルゴリズム**（v0.3）
   - 新構文`name = lambda` + HMアルゴリズム型推論を実装
   - 空パラメータ戻りなしのデフォルト充填を実装

2. **フェーズ2：移行ツール**（v0.3）
   - `yaoxiang-migrate --old-to-new`ツールを開発
   - 旧構文コードを自動変換

3. **フェーズ3：検証とドキュメント**（v0.3）
   - 旧コード移行完了の検証
   - ドキュメント更新

### 移行ツール

```bash
# 迁移单个文件
yaoxiang-migrate --old-to-new src/main.yaoxiang

# 迁移整个项目
yaoxiang-migrate --old-to-new --recursive src/

# 预览迁移（不修改文件）
yaoxiang-migrate --old-to-new --dry-run src/main.yaoxiang
```

移行規則：

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
- 独立して実装可能

### リスク

| リスク       | 影響                     | 緩和策                                           |
| ------------ | ------------------------ | ------------------------------------------------ |
| 移行漏れ     | 旧コードのコンパイル失敗 | 移行ツールを提供、すべての旧構文パターンをカバー |
| パーサエラー | 構文解析の不安定性       | 十分なテストカバレッジ                           |

## オープン問題

> 以下の問題は既に設計で解決されており、付録Aに記録されている。

- ~~Q1: 是否应该保留 `main() = body` 这种极简写法？~~
  → 解決済み：`main: () -> Void = { ... }`として保持
- ~~Q2: 函数名后的 `:` 是否保留？~~
  → 解決済み：オプションで保持；但し有引数関数は依然としてシグネチャまたはlambda頭でパラメータ型を注釈する必要あり
- ~~Q3: HM算法是否支持参数类型推断？~~
  → 解決済み：戻り値/ローカルは推論可能；有引数関数のパラメータ型は明示的注釈が必要
- ~~Q4: 是否引入 `fn` 关键字？~~ → 解決済み：導入しない、関数はlambdaである
- ~~Q5: 旧代码的迁移策略是什么？~~ → 解決済み：`yaoxiang-migrate`ツールを提供
- ~~Q6: 泛型函数如何使用？~~ → 解決済み：RFC-010統一構文`(T: Type)`を使用

---

## 付録

### 付録A：各言語の関数定義構文リファレンス

| 言語         | 構文スタイル                                        | 特徴                                |
| ------------ | --------------------------------------------------- | ----------------------------------- |
| Rust         | `fn add(a: i32, b: i32) -> i32 { ... }`             | キーワード + 型注釈                 |
| Haskell      | `add a b = ...` / `add :: Int -> Int -> Int`        | 型シグネチャ分離                    |
| OCaml        | `let add a b = ...`                                 | パラメータ型省略可能                |
| MoonBit      | `fn add(a: Int, b: Int): Int { ... }`               | 簡潔な型注釈                        |
| TypeScript   | `const add = (a: number, b: number): number => ...` | Lambdaスタイル                      |
| Scala        | `def add(a: Int, b: Int): Int = { ... }`            | defキーワード                       |
| **YaoXiang** | `name = (a: Int, b: Int) => a + b`                  | **関数 = lambda、HMが戻り値を推論** |

### 付録B：設計決定記録

| 決定           | 内容                                                             | 日付       | 記録者  |
| -------------- | ---------------------------------------------------------------- | ---------- | ------- |
| 構文スタイル   | 新構文`name: (params) -> Return = body` + HM推論                 | 2026-02-03 | @沫郁酱 |
| パラメータ位置 | パラメータ名はシグネチャで宣言、RFC-010と統一                    | 2026-02-03 | @沫郁酱 |
| デフォルト充填 | 空パラメータ関数はシグネチャ省略可、空ブロック`{}`は`Void`と推論 | 2026-02-03 | @沫郁酱 |
| 型推論         | HMアルゴリズムが自動推論、推論不可時は明示的                     | 2026-01-06 | @沫郁酱 |
| 旧構文         | 廃止、移行ツールを提供                                           | 2026-01-06 | @沫郁酱 |
| fnキーワード   | 導入しない                                                       | 2026-01-06 | @沫郁酱 |
| 再帰宣言       | HMアルゴリズムと再帰制約が自動推論                               | 2026-01-06 | @沫郁酱 |

### 付録C：用語集

| 用語             | 定義                                                                                                     |
| ---------------- | -------------------------------------------------------------------------------------------------------- |
| HMアルゴリズム   | Hindley-Milner型推論アルゴリズム、関数と変数の型を自動推論                                               |
| generics         | 型引数`(T: Type)`を使用して多相関数を制約する、例：`identity: (T: Type) -> ((x: T) -> T) = x`（RFC-010） |
| デフォルト型充填 | 空パラメータ戻りなし関数は`-> Void`を省略、コンパイラが自動充填                                          |
| 糖衣構文         | コードをより読みやすくする構文の簡略化写法                                                               |
| 正規化           | 構文形式を統一内部表現に変換                                                                             |
| 関数=lambda      | 関数は本質的にlambda変数、型はHMアルゴリズムで自動推論                                                   |

---

## 参考文献

- [MoonBit 言語設計](https://moonbitlang.com/)
- [Rust 関数構文](https://doc.rust-lang.org/book/ch03-03-how-functions-work.html)
- [Haskell 型システム](https://www.haskell.org/tutorial/patterns.html)
- [OCaml 型推論](https://v2.ocaml.org/manual/)

---
title: "RFC-007：関数定義構文統一方案"
---

# RFC-007: 関数定義構文統一方案

> **状態**: 承認済み
> **著者**: 沫郁酱
> **作成日**: 2025-01-05
> **最終更新**: 2026-03-21（型構築子ルールとコードブロック戻り値セマンティクスの整合）

## 要約

本 RFC は YaoXiang 言語の**関数定義構文**の最終方案を定める。統一構文 `name: (params) -> Return = body` を使用し、RFC-010 の `name: type = value` モデルと完全に整合する。

曖昧さを避けるため：関数に 입력 매개변수가存在する場合、パラメータ型は「シグネチャ」または「lambda 頭」の少なくとも片方に明示的に标注しなければならない；両方を省略した場合は拒否される。

コードブロック `{ ... }` 内では `return` を使用して値を返さなければならない；`return` がない場合はデフォルトで `Void` を返す。式形式 `= expr` は直接値を返す。

## 動機

### なぜこの機能が必要か？

1. **構文の一貫性**：旧構文の歴史的包袱を消除し、スタイルを統一する
2. **簡潔性**：HMアルゴリズムが型を自動推定し、ボイラープレートコードを削減する
3. **型安全性**：HMアルゴリズムが型安全性を保証し、推定できない場合のみ明示的に标注する
4. **言語成熟度**：HMアルゴリズムは現代関数型言語の成熟した方案である

### 統一構文モデル

**基本原則**：`name: Signature = LambdaBody`

- **完全形式**：シグネチャ（パラメータ名 + 型 + `->` + 戻り型） + Lambda頭（パラメータ名を含む）
- **省略ルール**：曖昧さを引入しない的前提下可能な限り省略
  - `->` は省略不可（関数型の印であり、省略するとタプルとしてパースされる）
  - **입력 매개변수가存在する場合**、パラメータ型はシグネチャまたは lambda 頭の少なくとも片方に明示的に出现しなければならない
  - Lambda 頭は省略可 → シグネチャが既にパラメータ名と型を宣言している場合
  - 戻り型は明示的に标注可能であり、推定可能な場合は省略可

```yaoxiang
# 完全形式（シグネチャ完全 + Lambda頭完全）
add: (a: Int, b: Int) -> Int = (a, b) => a + b

# 省略：Lambda 頭を省略（シグネチャがパラメータを宣言済み）
add: (a: Int, b: Int) -> Int = a + b

# 省略：シグネチャを省略（lambda 頭がパラメータ型を标注）
add = (a: Int, b: Int) => a + b

# ❌ 誤り：両方がパラメータ型を标注していない
# add = (a, b) => a + b
```

### 設計目標

```yaoxiang
# === 完全形式 ===
add: (a: Int, b: Int) -> Int = (a, b) => { a + b }

# === 省略形式 ===
add: (a: Int, b: Int) -> Int = a + b                 # Lambda 頭を省略
add = (a: Int, b: Int) => a + b                      # シグネチャを省略

# === 空パラメータ関数 ===
main: () -> Void = () => { println("Hello") }          # 完全形式
main: () -> Void = { println("Hello") }                # Lambda 頭を省略
main = { println("Hello") }                            # 最小形式（() -> Void と推定）

# === ジェネリック関数（RFC-010 統一構文を使用）===
identity: (T: Type) -> ((x: T) -> T) = (x) => x         # 完全形式
identity: (T: Type) -> ((x: T) -> T) = x                # Lambda 頭を省略
identity = (x: T) => x                                  # シグネチャを省略（lambda 頭が型を标注）

# === 再帰関数 ===
factorial: (n: Int) -> Int = (n) => {
    if n <= 1 { return 1 } else { return n * factorial(n - 1) }
}
```

### 構文ルール

| シナリオ | 構文 | 説明 |
|------|------|----------|
| **完全形式** | `name: (a: Type, b) -> Ret = (a, b) => { return ... }` | シグネチャ + Lambda頭 完全 |
| **Lambda 頭を省略** | `name: (a: Type, b: Type) -> Ret = { ... }` | シグネチャがパラメータを宣言済み |
| **シグネチャを省略** | `name = (a: Type, b: Type) => { ... }` | lambda 頭がパラメータ型を标注 |
| **空パラメータ完全** | `name: () -> Void = () => { return ... }` | 空パラメータ関数 完全 |
| **空パラメータ省略** | `name: () -> Void = { return ... }` | Lambda 頭を省略 |
| **空パラメータ最小** | `name = { return ... }` | パラメータなし・戻りなし 最小 |

**注意**：コードブロック `{ ... }` 内では `return` を使用して値を返さなければならない；`return` がない場合はデフォルトで `Void` を返す。式形式 `= expr` は直接値を返す。

**注意**：`->` は関数型の印であり、省略不可（省略するとタプルとしてパースされる）。

**重要**：`if` 式は波括弧 `{}` で分岐を包裹し、`then/else` キーワードをサポートしない：
```yaoxiang
# 正しい：波括弧を使用
if n <= 1 { return 1 } else { return n * factorial(n - 1) }

# 誤り：then/else キーワードはサポートしない
# if n <= 1 then return 1 else return n * factorial(n - 1)
```

## 提案

### HMアルゴリズムと高階多态サポート

**コア機能**：HMアルゴリズムはジェネリック型注釈を通じて高階多态（Higher-rank polymorphism）をサポートする

**設計原理**：
- **高階関数**：関数をパラメータとして渡す際、その関数型をジェネリックに制約する必要がある
- **型注釈形式**：`(T: Type) -> ((f: (T) -> T, x: T) -> T)` - ジェネリックパラメータが関数型を制約する
- **HMワークフロー**：ジェネリックパラメータを通じて関数型を推定し、多态関数合成を実現する

**サンプル説明**：
```yaoxiang
# ✅ 高階多态をサポート：ジェネリックが関数型パラメータを制約
call_twice: (T: Type) -> ((f: (T) -> T, x: T) -> T) = {
    return f(f(x))
}
# 使用：call_twice((x) => x + 1, 5)  # T=Int と推定

compose: (A: Type, B: Type, C: Type) -> ((f: (B) -> C, g: (A) -> B, x: A) -> C) = {
    return f(g(x))
}
# 使用：compose((x) => x * 2, (x) => x + 1, 5)  # A=Int, B=Int, C=Int と推定

# ❌ サポート外：高階関数にジェネリック制約がない
# bad_hof: (f, x) => f(f(x))  # HMが推定できず、ジェネリックパラメータが不足
```

**HM推定プロセス**：
1. 高階関数パラメータを識別：`f: (T) -> T`
2. ジェネリック制約を生成：`(T: Type)`
3. ジェネリックインスタンス化を通じて具体型を推定
4. 多态関数合成を実現

### Lambda 式構文ルール

**重要ルール**：コードブロック `{ ... }` 内では `return` を使用して値を返さなければならない；`return` がない場合はデフォルトで `Void` を返す。式形式 `= expr` は直接値を返す。

| 構文形式 | 構文 | 戻り方式 |
|---------|------|----------|
| **コードブロック形式** | `{ statements }` | `return` で値を返さなければならない；`return` がない場合はデフォルトで `Void` |
| **式形式** | `expression` | 式の値を直接返す |

**サンプル**：
```yaoxiang
main: () -> Void = { println("Hello") }         # Void を返す（return なし）
add: (a: Int, b: Int) -> Int = { return a + b }  # Int を返す（明示的な return）
empty: () -> Void = {}                          # 空ブロックはデフォルトで Void を返す

# 早期返回：return を使用
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

# 式形式：直接値を返す（return 不要）
add: (a: Int, b: Int) -> Int = a + b            # 正しい：式形式
main: () -> Void = println("Hello")               # 正しい：式形式
```

**コア思想**：
1. 関数定義はHMアルゴリズムを通じて型推定を行い、尽量推定し、推定できない場合は明示的にエラーを報告する
2. **HMアルゴリズム動作原理**：演算子型制約、関数呼び出し関係などのコンテキスト情報を通じて型を自動推定する
3. **ジェネリックサポート**：多态関数はジェネリック構文 `(T: Type)` を使用して型パラメータを明示的に制約する（RFC-010/011）
4. **推定境界**：戻り型とローカル変数は推定可能；パラメータ付き関数のパラメータ型は明示的に标注が必要（シグネチャまたは lambda 頭の少なくとも片方）
5. 空パラメータ戻りなし関数は `name: () -> Void = { ... }` を使用し、RFC-010 と統一する
6. 旧構文は退役し、移行ツールを提供する

**型推定サンプル**：
```yaoxiang
# ジェネリック関数：明示的型パラメータ（RFC-010 統一構文を使用）
identity: (T: Type) -> ((x: T) -> T) = x
map: (T: Type, R: Type) -> ((f: (T) -> R, list: List(T)) -> List(R)) = {
    result = List(R)()
    for item in list { result.push(f(item)) }
    return result
}

# 多态関数：明示的ジェネリック制約を通じて定義（RFC-010/011）
add: (T: Add) -> ((a: T, b: T) -> T) = a + b
print_sum: (a: Int, b: Int) -> Void = { println(a + b) }  # (Int, Int) -> Void と推定

# 高階多态：ジェネリック型注釈を通じてHMが高階多态をサポート
call_twice: (T: Type) -> ((f: (T) -> T, x: T) -> T) = { return f(f(x)) }
compose: (A: Type, B: Type, C: Type) -> ((f: (B) -> C, g: (A) -> B, x: A) -> C) = { return f(g(x)) }
```

```yaoxiang
# === 関数定義：HMアルゴリズム型推定 ===

# 標準関数：HMアルゴリズムが戻り型を推定（パラメータ型は明示が必要）
add = (a: Int, b: Int) => a + b            # (a: Int, b: Int) -> Int と推定
main = { println("Hello") }                # () -> Void と推定

# 部分的に明示的参数：HMアルゴリズムが 나머지部分を推定
print_sum: (a: Int, b: Int) -> Void = { println(a + b) }  # (Int, Int) -> Void と推定
greet: (name: String) -> Void = { println("Hello " + name) }  # (String) -> Void と推定

# ジェネリック関数：多态型パラメータを明示的に制約（RFC-010 統一構文を使用）
identity: (T: Type) -> ((x: T) -> T) = x
map: (T: Type, R: Type) -> ((f: (T) -> R, list: List(T)) -> List(R)) = {
    # map 関数を実装
    return List(R)()
}

# 再帰関数：HMアルゴリズムと再帰制約を通じて推定
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 } else { return n * factorial(n - 1) }
}

# === 変数代入：HMアルゴリズム型推定 ===

# 明示的型
x: Int = 42

# HMアルゴリズムが自動的に Int と推定
y = 42                               # Int と推定

# HMアルゴリズムが自動的に String と推定
name = "YaoXiang"                    # String と推定

# HMアルゴリズムが自動的に Float と推定
pi = 3.14159                         # Float と推定
```

**HM型推定ルール**：

| シナリオ | 構文 | 省略可能部分 | サンプル |
|------|------|----------|------|
| **完全形式** | `name: (a: Type, b: Type) -> Ret = (a, b) => ...` | なし | シグネチャ + Lambda頭 完全 |
| **Lambda 頭を省略** | `name: (a: Type, b: Type) -> Ret = ...` | Lambda 頭 | シグネチャがパラメータを宣言済み |
| **シグネチャを省略** | `name = (a: Type, b: Type) => ...` | シグネチャ | lambda 頭がパラメータ型を提供 |
| **戻り Ret を省略** | `name: (a: Type, b: Type) -> = ...` | 戻り型 | HM が戻り型を推定 |
| **空パラメータ完全** | `name: () -> Void = () => { ... }` | なし | 空パラメータ関数 完全 |
| **空パラメータ省略** | `name: () -> Void = { ... }` | Lambda 頭 | `() =>` を省略 |
| **空パラメータ最小** | `name = { ... }` | すべて | パラメータなし・戻りなし 最小 |
| **変数代入** | `name = value` | 型 | HM が型を推定 |
| **明示的変数** | `name: Type = value` | なし | 明示的型注釈 |

**コア原則**：
- `->` は関数型の印であり、省略不可（省略するとタプルとしてパースされる）
- 戻り型 `Ret` は省略可能で、HM に基づき関数本体から推定する
- 入力パラメータが存在する場合、パラメータ型は明示的に出现しなければならない（シグネチャまたは lambda 頭の少なくとも片方）
- 그以外 부분は推定可能かつ曖昧さを引入しない場合に省略可能
- 暗黙的な型変換はなく、JavaScript 的な混乱を避ける

## 詳細な設計

### 糖衣構文展開

省略の有無に関わらず、最終的にはすべて統一中間表現に正規化される：

```rust
// 完全形式
add: (a: Int, b: Int) -> Int = (a, b) => a + b

// 展開後 IR
let add: (Int, Int) -> Int = |a: Int, b: Int| -> Int {
    return a + b
};

// Lambda 頭を省略
add: (a: Int, b: Int) -> Int = a + b

// 展開後 IR（完全形式と同じ）
let add: (Int, Int) -> Int = |a: Int, b: Int| -> Int {
    return a + b
};

// シグネチャを省略（lambda 頭がパラメータ型を标注）
add = (a: Int, b: Int) => a + b

// 展開後 IR
let add: (Int, Int) -> Int = |a: Int, b: Int| -> Int {
    a + b
};
```

### 構文定義

```bnf
function_def ::= identifier ':' type_expr '=' expression
               | identifier '=' expression
               | identifier '=' block                    # 最小形式：パラメータなし・戻りなし

identifier ::= [a-zA-Z_][a-zA-Z0-9_]*

type_expr ::= identifier                     # 型参照
       | '()'                          # 空型
       | '(' parameters ')' '->' type_expr   # 関数型（シグネチャ中のパラメータ名）
       | type_expr '->' type_expr            # 単純関数型
       | identifier '(' type_expr (',' type_expr)* ')'  # 型応用

expression ::= '(' parameters ')' '=>' block
             | '(' ')' '=>' block
             | '(' parameters ')' '=>' expression

parameters ::= parameter (',' parameter)*
parameter ::= identifier                # 型推定
            | identifier ':' type_expr      # 部分的に明示的な型

block ::= '{' statement (',' statement)* '}'
        | expression

statement ::= identifier ':' expression  # 代入文
           | expression                  # 式文（実行するが返さない）
           | 'return' expression         # 返回文（指定された値を返す）

# 注意：コードブロック内では return で値を返さなければならない；return がない場合はデフォルトで Void を返す
# 例：{ return 1 + 1 } は Int を返す；{ println("Hello") } は Void を返す
# 注意：ジェネリックパラメータは (T: Type) 構文を使用し、関数型の一部となるため、独立した BNF ルールは不要
```

### エラー処理

```yaoxiang
# === コンパイルエラーサンプル ===

# エラー1：コードブロック戻り型が一致しない
add: (a: Int, b: Int) -> Int = { println(a + b) }
// エラー：ブロック内に return がなく、デフォルトで Void を返すが、シグネチャは Int を期待
// 正しい：add: (a: Int, b: Int) -> Int = a + b
// または：add: (a: Int, b: Int) -> Int = { return a + b }

# エラー2：未宣言の型パラメータを使用
identity: (x: T) -> T = x
// エラー：T が未宣言；明示的なジェネリックパラメータが必要（RFC-010）
// 正しい：identity: (T: Type) -> ((x: T) -> T) = x

# 正しい：HMアルゴリズムが戻り型を推定
double = (x: Int) => x + x

# 完全形式（段階的に省略）
double: (x: Int) -> Int = (x) => x + x                # 完全
double: (x: Int) -> Int = x + x                       # Lambda 頭を省略
double = (x: Int) => x + x                            # 戻り型を省略（HM が推定）
# double = (x) => x + x                               # ❌ パラメータ型は両方を省略できない
```

## トレードオフ

### 优点

- **構文の統一**：`name: Signature = LambdaBody` モデルがすべてのシナリオをカバー
- **柔軟な省略**：HM が推定可能な任意のパートを省略可能
- **型安全性**：HMアルゴリズムが型安全性を保証し、暗黙的な型変換を避ける
- **再帰サポート**：HMアルゴリズムと再帰制約が自動的に型を推定
- **ゼロ负担**：完全から最小まで滑らかな移行

### 缺点

- **移行コスト**：古いコードは変換ツールで変換する必要がある
- **学習コスト**：「完全形式 + 任意の省略」モデルを理解する必要がある

## 代替案

| 方案 | 説明 | なぜ選択しないか |
|------|------|-----------|
| HMアルゴリズム型推定 | Hindley-Milnerアルゴリズムを使用して型を推定 | ✅ **採用済み**、現代関数型言語の標準 |
| 明示的型宣言 | すべての型を明示的に記述 | 簡略化構文原则に违反、ボイラープレートコードが増加 |
| 旧構文を維持 | 新旧構文を同時にサポート | 構文分裂、メンテ成本が高い |
| fn キーワード | 関数を区別するための fn を導入 | 「関数は lambda である」という設計に违反 |

## 実装戦略

### フェーズ分け

1. **Phase 1: 構文解析とHMアルゴリズム**（v0.3）
   - 新構文 `name = lambda` + HMアルゴリズム型推定を実装
   - 空パラメータ戻りなしの場合のデフォルト填充を実装

2. **Phase 2: 移行ツール**（v0.3）
   - `yaoxiang-migrate --old-to-new` ツールを開発
   - 旧構文コードを自動的に変換

3. **Phase 3: 検証とドキュメント**（v0.3）
   - 旧コードの移行完了を検証
   - ドキュメントを更新

### 移行ツール

```bash
# 単一ファイルの移行
yaoxiang-migrate --old-to-new src/main.yaoxiang

# プロジェクト全体の移行
yaoxiang-migrate --old-to-new --recursive src/

# 移行をプレビュー（ファイルは変更しない）
yaoxiang-migrate --old-to-new --dry-run src/main.yaoxiang
```

移行ルール：
```yaoxiang
# 旧構文
add(Int, Int) -> Int = (a, b) => { a + b }
main() -> Int = { println("Hello"); 0 }
main() = { println("Hello") }

# === 新構文：完全形式（シグネチャ完全 + Lambda頭完全）===
add: (a: Int, b: Int) -> Int = (a, b) => a + b
main: () -> Void = () => { println("Hello") }

# === 省略：Lambda 頭を省略 ===
add: (a: Int, b: Int) -> Int = a + b
main: () -> Void = { println("Hello") }

# === 省略：HM 推定 ===
add = (a: Int, b: Int) => a + b              # (a: Int, b: Int) -> Int と推定
main = { println("Hello") }                  # () -> Void と推定

# === 最小形式 ===
main = {                                      # main: () -> Void = { ... } と同等
    println("Hello")
}
```

### 依存関係

- 外部依存なし
- 独立して実装可能

### リスク

| リスク | 影響 | 軽減措施 |
|------|------|---------|
| 移行もれ | 旧コードがコンパイル失敗 | 移行ツールを提供、すべての旧構文パターンをカバー |
| パーザーエラー | 構文解析が不安定 | 十分なテストカバレッジ |

## 未解決の問題

> 以下的问题是已在设计中解决，记录在付録A中。

- ~~Q1: `main() = body` 这种极简写法是否应该保留？~~ → 已解决：保留为 `main = { ... }`
- ~~Q2: 函数名后的 `:` 是否保留？~~ → 已解决：可选保留；但有参函数仍需在签名或 lambda 头标注参数类型
- ~~Q3: HM算法是否支持参数类型推断？~~ → 已解决：返回值/局部可推断；有参函数的参数类型需显式标注
- ~~Q4: 是否引入 `fn` 关键字？~~ → 已解决：不引入，函数就是 lambda
- ~~Q5: 旧代码的迁移策略是什么？~~ → 已解决：提供 `yaoxiang-migrate` 工具
- ~~Q6: 泛型函数如何使用？~~ → 已解决：使用RFC-010统一语法 `(T: Type)`

---

## 付録

### 付録A：各言語関数定義構文参照

| 言語 | 構文スタイル | 特徴 |
|------|---------|------|
| Rust | `fn add(a: i32, b: i32) -> i32 { ... }` | キーワード + 型注釈 |
| Haskell | `add a b = ...` / `add :: Int -> Int -> Int` | 型シグネチャ分離 |
| OCaml | `let add a b = ...` | パラメータ型は省略可能 |
| MoonBit | `fn add(a: Int, b: Int): Int { ... }` | 簡潔な型注釈 |
| TypeScript | `const add = (a: number, b: number): number => ...` | Lambda スタイル |
| Scala | `def add(a: Int, b: Int): Int = { ... }` | def キーワード |
| **YaoXiang** | `name = (a: Int, b: Int) => a + b` | **関数 = lambda、HM が戻り値を推定** |

### 付録B：設計意思決定記録

| 意思決定 | 決定 | 日付 | 記録人 |
|------|------|------|--------|
| 構文スタイル | 新構文 `name: (params) -> Return = body` + HM推定 | 2026-02-03 | @沫郁酱 |
| パラメータ位置 | パラメータ名はシグネチャで宣言し、RFC-010 と統一 | 2026-02-03 | @沫郁酱 |
| デフォルト填充 | 空パラメータ関数はシグネチャを省略可能、空ブロック `{}` は `Void` と推定 | 2026-02-03 | @沫郁酱 |
| 型推定 | HMアルゴリズムが自動推定、推定できない場合は明示 | 2026-01-06 | @沫郁酱 |
| 旧構文 | 退役、移行ツールを提供 | 2026-01-06 | @沫郁酱 |
| fn キーワード | 引入しない | 2026-01-06 | @沫郁酱 |
| 再帰宣言 | HMアルゴリズムと再帰制約が自動推定 | 2026-01-06 | @沫郁酱 |

### 付録C：用語集

| 用語 | 定義 |
|------|------|
| HMアルゴリズム | Hindley-Milner型推定アルゴリズム、関数と変数の型を自動推定 |
| ジェネリック | 型パラメータ `(T: Type)` を使用して多态関数を制約、例：`identity: (T: Type) -> ((x: T) -> T) = x`（RFC-010） |
| デフォルト型填充 | 空パラメータ戻りなし関数で `-> Void` を省略でき、コンパイラが自動填充 |
| 糖衣構文 | コードを読みやすくする構文簡略化写法 |
| 正規化 | 構文形式を統一内部表現に変換 |
| 関数即 lambda | 関数は本質的に lambda 変数であり、型はHMアルゴリズムを通じて自動推定 |

---

## 参考文献

- [MoonBit 言語設計](https://moonbitlang.com/)
- [Rust 関数構文](https://doc.rust-lang.org/book/ch03-03-how-functions-work.html)
- [Haskell 型システム](https://www.haskell.org/tutorial/patterns.html)
- [OCaml 型推定](https://v2.ocaml.org/manual/)
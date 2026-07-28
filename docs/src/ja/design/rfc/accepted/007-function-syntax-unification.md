---
title: 'RFC-007: 関数定義構文統一方案'
issue: '#131'
status: '承認済み'
author: '沫郁酱'
created: '2025-01-05'
updated: '2026-07-05（GH Issue #131に同期）'
---

# RFC-007: 関数定義構文統一方案

## 摘要

本 RFC は YaoXiang 言語の**関数定義構文**の最終方案を確定する。統一構文
`name: (params) -> Return = body`を使用し、RFC-010の `name: type = value` モデルと完全に一致させる。

曖昧さを避けるため：関数に入力パラメータがある場合、パラメータの型は「シグネチャ」または「lambda 頭」の少なくとも 片方に明示的に标注する必要がある。両方を省略した場合は拒否される。

コードブロック `{ ... }` 内では `return` を使用して値を返さなければならない。`return` がない場合はデフォルトで `Void` を返す。式形式 `= expr` は直接値を返す。

## 動機

### なぜこの機能が必要なのか？

1. **構文の一貫性**：古い構文の歴史的包袱を消除し、スタイルを統一する
2. **簡潔性**：HMアルゴリズムが自動的に型を推断し、ボイラープレートコードを削減する
3. **型安全性**：HMアルゴリズムが型安全性を保証し、推断できない場合にのみ明示的に标注する
4. **言語の成熟度**：HMアルゴリズムは現代関数型言語の成熟した方案である

### 統一構文モデル

**核心原則**：`name: Signature = LambdaBody`

- **完全形式**：シグネチャ（パラメータ名 + 型 + `->` + 戻り値の型） + Lambda頭（パラメータ名を含む）
- **省略規則**：曖昧さを招かない的前提下で最も省略する
  - `->` は省略不可（関数型の印であり、省略するとタプルとしてパースされる）
  - **入力パラメータがある場合**、パラメータの型はシグネチャまたは lambda 頭の少なくとも 片方に明示的に出現する必要がある
  - Lambda 頭は省略可能 → シグネチャがパラメータ名と型を既に宣言している場合
  - 戻り値の型は明示的に标注可能，也可省略可，也可省略可能な場合は省略する

```yaoxiang
# 完全形式（シグネチャ完全 + Lambda頭完全）
add: (a: Int, b: Int) -> Int = (a, b) => a + b

# 省略：Lambda 頭を省略（シグネチャがパラメータを宣言済み）
add: (a: Int, b: Int) -> Int = a + b

# 省略：シグネチャを省略（lambda 頭がパラメータの型を标注）
add = (a: Int, b: Int) => a + b

# ❌ エラー：両方がパラメータの型を标注していない
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
main = { println("Hello") }                            # 最も省略形式（() -> Voidと推断）

# === ジェネリクス関数（RFC-010統一構文を使用）===
identity: (T: Type) -> ((x: T) -> T) = (x) => x         # 完全形式
identity: (T: Type) -> ((x: T) -> T) = x                # Lambda 頭を省略
identity = (x: T) => x                                  # シグネチャを省略（lambda 頭が型を标注）

# === 再帰関数 ===
factorial: (n: Int) -> Int = (n) => {
    if n <= 1 { return 1 } else { return n * factorial(n - 1) }
}
```

### 構文規則

| シナリオ           | 構文                                                   | 説明                  |
| ------------------ | ------------------------------------------------------ | --------------------- |
| **完全形式**       | `name: (a: Type, b) -> Ret = (a, b) => { return ... }` | シグネチャ + Lambda 頭完全 |
| **Lambda 頭を省略** | `name: (a: Type, b: Type) -> Ret = { ... }`            | シグネチャがパラメータを宣言済み |
| **シグネチャを省略** | `name = (a: Type, b: Type) => { ... }`                 | lambda 頭がパラメータの型を标注 |
| **空パラメータ完全** | `name: () -> Void = () => { return ... }`              | 空パラメータ関数完全          |
| **空パラメータ省略** | `name: () -> Void = { return ... }`                    | Lambda 頭を省略              |
| **空パラメータ最省略** | `name = { return ... }`                                | 無パラメータ無返最も省略       |

**注意**：コードブロック `{ ... }` 内では `return` を使用して値を返さなければならない。`return` がない場合はデフォルトで `Void` を返す。式形式 `= expr` は直接値を返す。

**注意**：`->` は関数型の印であり、省略不可（省略するとタプルとしてパースされる）。

**重要**：`if` 式は波括弧 `{}` を使用して分支を包み、`then/else` キーワードをサポートしない：

```yaoxiang
# 正しい：波括弧を使用
if n <= 1 { return 1 } else { return n * factorial(n - 1) }

# エラー：then/else キーワードはサポートされていない
# if n <= 1 then return 1 else return n * factorial(n - 1)
```

## 提案

### HMアルゴリズムと高階多態サポート

**核心特性**：HMアルゴリズムはジェネリクス型注釈により高階多態（Higher-rank polymorphism）をサポートする

**設計原理**：

- **高階関数**：関数をパラメータとして渡す場合、関数型を制約するジェネリクスが必要
- **型注釈形式**：`(T: Type) -> ((f: (T) -> T, x: T) -> T)` - ジェネリクスパラメータが関数型を制約
- **HMワークフロー**：ジェネリクスパラメータを通じて関数型を推断し、多態関数の合成を実現する

**示例説明**：

```yaoxiang
# ✅ 高階多態をサポート：ジェネリクスが関数型パラメータを制約
call_twice: (T: Type) -> ((f: (T) -> T, x: T) -> T) = {
    return f(f(x))
}
# 使用：call_twice((x) => x + 1, 5)  # T=Intと推断

compose: (A: Type, B: Type, C: Type) -> ((f: (B) -> C, g: (A) -> B, x: A) -> C) = {
    return f(g(x))
}
# 使用：compose((x) => x * 2, (x) => x + 1, 5)  # A=Int, B=Int, C=Intと推断

# ❌ サポート外：ジェネリクス制約のない高階関数
# bad_hof: (f, x) => f(f(x))  # HMが推断できず、ジェネリクスパラメータが欠落
```

**HM推断プロセス**：

1. 高階関数パラメータを識別：`f: (T) -> T`
2. ジェネリクス制約を作成：`(T: Type)`
3. ジェネリクスインスタンス化を通じて具体的な型を推断
4. 多態関数の合成を実現

### Lambda 式構文規則

**重要な規則**：コードブロック `{ ... }` 内では `return` を使用して値を返さなければならない。`return` がない場合はデフォルトで `Void` を返す。式形式 `= expr` は直接値を返す。

| 構文形式         | 構文             | 返し方                                            |
| -------------- | ---------------- | --------------------------------------------------- |
| **コードブロック形式** | `{ statements }` | `return` を使用して値を返さなければならない；`return` がない場合はデフォルトで `Void` |
| **式形式**       | `expression`     | 式の値を直接返す                                    |

**示例**：

```yaoxiang
main: () -> Void = { println("Hello") }         # Voidを返す（returnなし）
add: (a: Int, b: Int) -> Int = { return a + b }  # Intを返す（明示的な return）
empty: () -> Void = {}                          # 空ブロックはデフォルトでVoidを返す

# 早期返回：return を使用
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

# 式形式：直接値を返す（return 不要）
add: (a: Int, b: Int) -> Int = a + b            # 正しい：式形式
main: () -> Void = println("Hello")               # 正しい：式形式
```

**核心思想**：

1. 関数定義はHMアルゴリズムを通じて型推断を行い、尽量推断し、推断できない場合は明示的にエラーを出す
2. **HMアルゴリズム動作原理**：演算子の型制約、関数呼び出し関係などのコンテキスト情報により自動的に型を推断
3. **ジェネリクスサポート**：多態関数はジェネリクス構文 `(T: Type)` を使用して型パラメータを明示的に制約（RFC-010/011）
4. **推断境界**：戻り値の型とローカル変数は推断可能；パラメータを持つ関数のパラメータの型は明示的に标注が必要（シグネチャまたは lambda 頭の一方）
5. 空パラメータ無返関数は `name: () -> Void = { ... }` を使用し、RFC-010と統一
6. 古い構文は退役し、移行ツールを提供

**型推断示例**：

```yaoxiang
# ジェネリクス関数：明示的な型パラメータ（RFC-010統一構文を使用）
identity: (T: Type) -> ((x: T) -> T) = x
map: (T: Type, R: Type) -> ((f: (T) -> R, list: List(T)) -> List(R)) = {
    result = List(R)()
    for item in list { result.push(f(item)) }
    return result
}

# 多態関数：明示的なジェネリクス制約により定義（RFC-010/011）
add: (T: Add) -> ((a: T, b: T) -> T) = a + b
print_sum: (a: Int, b: Int) -> Void = { println(a + b) }  # (Int, Int) -> Voidと推断

# 高階多態：ジェネリクス型注釈によりHMが高階多態をサポート
call_twice: (T: Type) -> ((f: (T) -> T, x: T) -> T) = { return f(f(x)) }
compose: (A: Type, B: Type, C: Type) -> ((f: (B) -> C, g: (A) -> B, x: A) -> C) = { return f(g(x)) }
```

```yaoxiang
# === 関数定義：HMアルゴリズム型推断 ===

# 標準関数：HMアルゴリズムが戻り値の型を推断（パラメータの型は明示的に必要）
add = (a: Int, b: Int) => a + b            # (a: Int, b: Int) -> Intと推断
main = { println("Hello") }                # () -> Voidと推断

# 部分的に明示的なパラメータ：HMアルゴリズムが残りを推断
print_sum: (a: Int, b: Int) -> Void = { println(a + b) }  # (Int, Int) -> Voidと推断
greet: (name: String) -> Void = { println("Hello " + name) }  # (String) -> Voidと推断

# ジェネリクス関数：多態型パラメータを明示的に制約（RFC-010統一構文を使用）
identity: (T: Type) -> ((x: T) -> T) = x
map: (T: Type, R: Type) -> ((f: (T) -> R, list: List(T)) -> List(R)) = {
    # map 関数を実装
    return List(R)()
}

# 再帰関数：HMアルゴリズムと再帰制約により推断
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 } else { return n * factorial(n - 1) }
}

# === 変数代入：HMアルゴリズム型推断 ===

# 明示的な型
x: Int = 42

# HMアルゴリズムが自動的に Int と推断
y = 42                               # Intと推断

# HMアルゴリズムが自動的に String と推断
name = "YaoXiang"                    # Stringと推断

# HMアルゴリズムが自動的に Float と推断
pi = 3.14159                         # Floatと推断
```

**HM型推断規則**：

| シナリオ             | 構文                                              | 省略可能部分 | 示例                  |
| ------------------ | ------------------------------------------------- | ---------- | --------------------- |
| **完全形式**       | `name: (a: Type, b: Type) -> Ret = (a, b) => ...` | なし         | シグネチャ + Lambda 頭完全 |
| **Lambda 頭を省略** | `name: (a: Type, b: Type) -> Ret = ...`           | Lambda 頭  | シグネチャがパラメータを宣言済み |
| **シグネチャを省略** | `name = (a: Type, b: Type) => ...`                | シグネチャ       | lambda 頭がパラメータの型を提供 |
| **Ret を省略**   | `name: (a: Type, b: Type) -> = ...`               | 戻り値の型   | HM が戻り値の型を推断       |
| **空パラメータ完全** | `name: () -> Void = () => { ... }`                | なし         | 空パラメータ関数完全          |
| **空パラメータ省略** | `name: () -> Void = { ... }`                      | Lambda 頭  | `() =>` を省略          |
| **空パラメータ最省略** | `name = { ... }`                                  | すべて       | 無パラメータ無返最も省略       |
| **変数代入**       | `name = value`                                    | 型       | HM が型を推断           |
| **明示的な変数**   | `name: Type = value`                              | なし         | 明示的な型注釈          |

**核心原則**：

- `->` は関数型の印であり、省略不可（省略するとタプルとしてパースされる）
- 戻り値の型 `Ret` は省略可能で、HM が関数本体に基づいて推断する
- 入力パラメータが存在する場合、パラメータの型は明示的に出現する必要がある（シグネチャまたは lambda 頭の一方）
- 残りの部分は推断可能かつ曖昧さを招かない場合に省略可能
- 暗黙の型変換はなく、JavaScript のような混乱を避ける

## 詳細設計

### 構文糖衣展開

省略の有無に関係なく、最終的にはすべて統一中間表現に正規化される：

```rust
// 完全形式
add: (a: Int, b: Int) -> Int = (a, b) => a + b

// 展開後の IR
let add: (Int, Int) -> Int = |a: Int, b: Int| -> Int {
    return a + b
};

// Lambda 頭を省略
add: (a: Int, b: Int) -> Int = a + b

// 展開後の IR（完全形式と同じ）
let add: (Int, Int) -> Int = |a: Int, b: Int| -> Int {
    return a + b
};

// シグネチャを省略（lambda 頭がパラメータの型を标注）
add = (a: Int, b: Int) => a + b

// 展開後の IR
let add: (Int, Int) -> Int = |a: Int, b: Int| -> Int {
    a + b
};
```

### 構文定義

```bnf
function_def ::= identifier ':' type_expr '=' expression
               | identifier '=' expression
               | identifier '=' block                    # 最も省略形式：無パラメータ無返

identifier ::= [a-zA-Z_][a-zA-Z0-9_]*

type_expr ::= identifier                     # 型参照
       | '()'                          # 空型
       | '(' parameters ')' '->' type_expr   # 関数型（シグネチャにパラメータ名あり）
       | type_expr '->' type_expr            # 単純な関数型
       | identifier '(' type_expr (',' type_expr)* ')'  # 型応用

expression ::= '(' parameters ')' '=>' block
             | '(' ')' '=>' block
             | '(' parameters ')' '=>' expression

parameters ::= parameter (',' parameter)*
parameter ::= identifier                # 型推断
            | identifier ':' type_expr      # 部分的に明示的な型

block ::= '{' statement (',' statement)* '}'
        | expression

statement ::= identifier ':' expression  # 代入文
           | expression                  # 式文（実行するが返さない）
           | 'return' expression         # 返回文（指定した値を返す）

# 注意：コードブロック内では return を使用して値を返さなければならない；return がない場合はデフォルトで Void を返す
# 例：{ return 1 + 1 } は Int を返す；{ println("Hello") } は Void を返す
# 注意：ジェネリクスパラメータは (T: Type) 構文を使用し、関数型の一部であるため、独立した BNF 規則は不要
```

### エラー処理

```yaoxiang
# === コンパイルエラー示例 ===

# エラー1：コードブロックの戻り値の型が一致しない
add: (a: Int, b: Int) -> Int = { println(a + b) }
// エラー：ブロック内に return がなく、デフォルトで Void を返すが、シグネチャは Int を期待
// 正しい：add: (a: Int, b: Int) -> Int = a + b
// または：add: (a: Int, b: Int) -> Int = { return a + b }

# エラー2：未宣言の型パラメータを使用
identity: (x: T) -> T = x
// エラー：T が未宣言；明示的なジェネリクスパラメータが必要（RFC-010）
// 正しい：identity: (T: Type) -> ((x: T) -> T) = x

# 正しい：HMアルゴリズムが戻り値の型を推断
double = (x: Int) => x + x

# 完全形式（逐步的に省略）
double: (x: Int) -> Int = (x) => x + x                # 完全
double: (x: Int) -> Int = x + x                       # Lambda 頭を省略
double = (x: Int) => x + x                            # 戻り値の型を省略（HM が推断）
# double = (x) => x + x                               # ❌ パラメータの型は両方を省略不可
```

## 权衡

### 利点

- **構文の統一**：`name: Signature = LambdaBody` モデルがすべてのシナリオをカバー
- **柔軟な省略**：HM が推断可能な任意的部分は省略可能
- **型安全性**：HMアルゴリズムが型安全性を保証し、暗黙の型変換を避ける
- **再帰サポート**：HMアルゴリズムと再帰制約が自動的に型を推断
- **ゼロオーバーヘッド**：完全から最も省略まで滑らかに移行

### 欠点

- **移行コスト**：古いコードは変換ツールが必要
- **学習コスト**：「完全形式 + 任意の省略」モデルを理解する必要がある

## 代替方案

| 方案           | 説明                           | なぜ選ばないか                        |
| -------------- | ------------------------------ | --------------------------------- |
| HMアルゴリズム型推断 | Hindley-Milnerアルゴリズムを使用して型を推断 | ✅ **採用済み**、現代関数型言語標準 |
| 明示的な型宣言   | すべての型を明示的に記述             | 構文簡略化の原則に違反、ボイラープレートが増加    |
| 古い構文を保留     | 新旧両方の構文を同時にサポート           | 構文分裂、メンテナンスコストが高い              |
| fn キーワード      | 関数と変数を区別するために fn を導入         | 「関数は lambda である」という設計に反する       |

## 実装策略

### 段階分け

1. **Phase 1: 構文解析とHMアルゴリズム**（v0.3）
   - 新しい構文 `name = lambda` + HMアルゴリズム型推断を実装
   - 空パラメータ無返のデフォルト填充を実装

2. **Phase 2: 移行ツール**（v0.3）
   - `yaoxiang-migrate --old-to-new` ツールを開発
   - 古い構文のコードを自動的に変換

3. **Phase 3: 検証とドキュメント**（v0.3）
   - 古いコードの移行完了検証
   - ドキュメント更新

### 移行ツール

```bash
# 単一ファイルを移行
yaoxiang-migrate --old-to-new src/main.yaoxiang

# プロジェクト全体を移行
yaoxiang-migrate --old-to-new --recursive src/

# 移行をプレビュー（ファイルは変更しない）
yaoxiang-migrate --old-to-new --dry-run src/main.yaoxiang
```

移行規則：

```yaoxiang
# 古い構文
add(Int, Int) -> Int = (a, b) => { a + b }
main() -> Int = { println("Hello"); 0 }
main() = { println("Hello") }

# === 新しい構文：完全形式（シグネチャ完全 + Lambda 頭完全）===
add: (a: Int, b: Int) -> Int = (a, b) => a + b
main: () -> Void = () => { println("Hello") }

# === 省略：Lambda 頭を省略 ===
add: (a: Int, b: Int) -> Int = a + b
main: () -> Void = { println("Hello") }

# === 省略：HM が推断 ===
add = (a: Int, b: Int) => a + b              # (a: Int, b: Int) -> Intと推断
main = { println("Hello") }                  # () -> Voidと推断

# === 最も省略形式 ===
main = {                                      # main: () -> Void = { ... } と同等
    println("Hello")
}
```

### 依存関係

- 外部依存なし
- 独立して実装可能

### リスク

| リスク       | 影響           | 軽減措施                         |
| ---------- | -------------- | -------------------------------- |
| 移行漏れ   | 古いコードがコンパイル失敗 | 移行ツールを提供し、すべての古い構文パターンをカバー |
| パーサーエラー | 構文解析が不安定 | 十分なテストカバレッジ                   |

## 開放問題

> 以下の問題は既に設計で解決済みであり、付録Aに記録されている。

- ~~Q1: `main() = body` 這種極簡寫法是否應該保留？~~ → 解決済み：`main = { ... }` として保留
- ~~Q2: 関数名の後の `:` は保留するか？~~ → 解決済み：オプションとして保留可能；但しパラメータを持つ関数はシグネチャまたは lambda 頭でパラメータの型を标注する必要がある
- ~~Q3: HMアルゴリズムはパラメータの型推断をサポートするか？~~ → 解決済み：戻り値/ローカル变量は推断可能；パラメータを持つ関数のパラメータの型は明示的に标注が必要
- ~~Q4: `fn` キーワードを導入するか？~~ → 解決済み：導入しない、関数は lambda である
- ~~Q5: 古いコードの移行戦略は何か？~~ → 解決済み：`yaoxiang-migrate` ツールを提供
- ~~Q6: ジェネリクス関数の使い方は？~~ → 解決済み：RFC-010統一構文 `(T: Type)` を使用

---

## 付録

### 付録A：各言語の関数定義構文参照

| 言語         | 構文スタイル                                            | 特徴                             |
| ------------ | --------------------------------------------------- | -------------------------------- |
| Rust         | `fn add(a: i32, b: i32) -> i32 { ... }`             | キーワード + 型注釈                |
| Haskell      | `add a b = ...` / `add :: Int -> Int -> Int`        | 型シグネチャが分離                     |
| OCaml        | `let add a b = ...`                                 | パラメータの型は省略可能                   |
| MoonBit      | `fn add(a: Int, b: Int): Int { ... }`               | 簡潔な型注釈                     |
| TypeScript   | `const add = (a: number, b: number): number => ...` | Lambda スタイル                      |
| Scala        | `def add(a: Int, b: Int): Int = { ... }`            | def キーワード                       |
| **YaoXiang** | `name = (a: Int, b: Int) => a + b`                  | **関数 = lambda、HM が戻り値を推断** |

### 付録B：設計意思決定記録

| 意思決定      | 決定                                              | 日付       | 記録者  |
| --------- | ------------------------------------------------- | ---------- | ------- |
| 構文スタイル  | 新しい構文 `name: (params) -> Return = body` + HM推断 | 2026-02-03 | @沫郁酱 |
| パラメータ位置  | パラメータ名はシグネチャで宣言し、RFC-010と統一               | 2026-02-03 | @沫郁酱 |
| デフォルト填充  | 空パラメータ関数はシグネチャを省略でき、空ブロック `{}` は `Void` と推断       | 2026-02-03 | @沫郁酱 |
| 型推断  | HMアルゴリズムが自動的に型を推断し、推断できない場合は明示的に                    | 2026-01-06 | @沫郁酱 |
| 古い構文    | 退役、移行ツールを提供                                | 2026-01-06 | @沫郁酱 |
| fn キーワード | 導入しない                                            | 2026-01-06 | @沫郁酱 |
| 再帰宣言  | HMアルゴリズムと再帰制約が自動的に推断                          | 2026-01-06 | @沫郁酱 |

### 付録C：用語集

| 用語          | 定義                                                                                            |
| ------------- | ----------------------------------------------------------------------------------------------- |
| HMアルゴリズム        | Hindley-Milner型推断アルゴリズム、関数と変数の型を自動的に推断                                              |
| ジェネリクス          | 型パラメータ `(T: Type)` を使用して多態関数を制約、例：`identity: (T: Type) -> ((x: T) -> T) = x`（RFC-010） |
| デフォルト型填充  | 空パラメータ無返関数は `-> Void` を省略でき、コンパイラが自動的に填充                                                    |
| 構文糖衣        | コードをより読みやすくする構文の簡略化                                                                          |
| 正規化        | 構文形式を統一内部表現に変換                                                                    |
| 関数はlambda | 関数は本質的に lambda 変数であり、型はHMアルゴリズムにより自動的に推断                                                  |

---

## 参考文献

- [MoonBit 言語設計](https://moonbitlang.com/)
- [Rust 関数構文](https://doc.rust-lang.org/book/ch03-03-how-functions-work.html)
- [Haskell 型システム](https://www.haskell.org/tutorial/patterns.html)
- [OCaml 型推断](https://v2.ocaml.org/manual/)

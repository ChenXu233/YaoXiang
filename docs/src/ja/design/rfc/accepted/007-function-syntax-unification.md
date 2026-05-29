---
title: RFC-007：関数定義構文の統一方案
---

# RFC-007: 関数定義構文の統一方案

> **状態**: 承認済み
> **著者**: 沫郁酱
> **作成日**: 2025-01-05
> **最終更新**: 2026-03-21（型コンストラクタ規則とコードブロックの戻り値セマンティクスの整合）

## 概要

本 RFC は YaoXiang 言語の**関数定義構文**の最終方案を定めます。統一構文 `name: (params) -> Return = body` を使用し、RFC-010 の `name: type = value` モデルと完全に一致させます。

曖昧さを避けるため：関数に輸入パラメータがある場合、パラメータの型は「シグネチャ」または「lambda 頭」の少なくとも片方に明示的に标注する必要があります。両方を省略した場合は拒否されます。

コードブロック `{ ... }` の最後の式が戻り値となります。空ブロック `{}` は `Void` を返します。

## 動機

### この機能が必要な理由

1. **構文の一貫性**：旧構文の歴史的包袱を排除し、スタイルを統一
2. **簡潔性**：HM算法が自動的に型を推断し、ボイラープレートコードを削減
3. **型安全性**：HM算法が型安全性を保証し、推断できない場合のみ明示的に标注
4. **言語の成熟度**：HM算法は現代関数型言語の成熟した方案

### 統一構文モデル

**基本原則**：`name: Signature = LambdaBody`

- **完全形**：シグネチャ（パラメータ名 + 型 + `->` + 戻り値の型を含む）+ Lambda頭（パラメータ名を含む）
- **省略規則**：曖昧さを引入しない前提下で可能な限り省略
  - `->` は省略不可（関数型の識別子，否则はタプルとして解析される）
  - **パラメータが存在する場合**、パラメータの型はシグネチャまたは lambda 頭の少なくとも片方に明示的に出现
  - Lambda 頭は省略可能 → シグネチャがパラメータ名と型を既に宣言している場合
  - 戻り値の型は明示的に标注可能，也可根据推断情况省略

```yaoxiang
# 完全形（シグネチャ完全 + Lambda頭完全）
add: (a: Int, b: Int) -> Int = (a, b) => a + b

# 省略：Lambda 頭を省略（シグネチャがパラメータを宣言済み）
add: (a: Int, b: Int) -> Int = a + b

# 省略：シグネチャを省略（lambda 頭がパラメータの型を标注）
add = (a: Int, b: Int) => a + b

# ❌ エラー：両方がパラメータの型を省略
# add = (a, b) => a + b
```

### 設計目標

```yaoxiang
# === 完全形 ===
add: (a: Int, b: Int) -> Int = (a, b) => { a + b }

# === 省略形 ===
add: (a: Int, b: Int) -> Int = a + b                 # Lambda 頭を省略
add = (a: Int, b: Int) => a + b                      # シグネチャを省略

# === 空パラメータ関数 ===
main: () -> Void = () => { println("Hello") }          # 完全形
main: () -> Void = { println("Hello") }                # Lambda 頭を省略
main = { println("Hello") }                            # 最も簡潔な形（() -> Void と推断）

# === ジェネリクス関数（RFC-010 統一構文を使用）===
identity: (T: Type) -> ((x: T) -> T) = (x) => x         # 完全形
identity: (T: Type) -> ((x: T) -> T) = x                # Lambda 頭を省略
identity = (x: T) => x                                  # シグネチャを省略（lambda 頭が型を标注）

# === 再帰関数 ===
factorial: (n: Int) -> Int = (n) => {
    if n <= 1 { 1 } else { n * factorial(n - 1) }
}
```

### 構文規則

| シナリオ | 構文 | 説明 |
|------|------|------|
| **完全形** | `name: (a: Type, b) -> Ret = (a, b) => { return ... }` | シグネチャ + Lambda 頭 完全 |
| **Lambda 頭を省略** | `name: (a: Type, b: Type) -> Ret = { ... }` | シグネチャがパラメータを宣言済み |
| **シグネチャを省略** | `name = (a: Type, b: Type) => { ... }` | lambda 頭がパラメータの型を标注 |
| **空パラメータ完全形** | `name: () -> Void = () => { return ... }` | 空パラメータ関数完全 |
| **空パラメータ省略形** | `name: () -> Void = { return ... }` | Lambda 頭を省略 |
| **空パラメータ最簡形** | `name = { return ... }` | パラメータなし・戻り値なし最簡形 |

**注意**：ブロック `{ ... }` の最後の式が戻り値となります。途中で終了する必要がある場合は `return` を使用します。空ブロック `{}` は `Void` と推断されます。

**注意**：`->` は関数型の識別子であり、省略できません（否则はタプルとして解析されます）。

**重要**：`if` 式は分岐を波括弧 `{}` で包裹し、`then/else` キーワードはサポートしません：
```yaoxiang
# 正しい：波括弧を使用
if n <= 1 { return 1 } else { return n * factorial(n - 1) }

# エラー：then/else キーワードはサポート外
# if n <= 1 then return 1 else return n * factorial(n - 1)
```

## 提案

### HM算法と高階多态支持

**核心的特性**：HM算法はジェネリック型注釈を通じて高階多态（Higher-rank polymorphism）を支持します。

**設計原理**：
- **高階関数**：関数をパラメータとして渡す際、関数型をジェネリックに制約する必要があります
- **型注釈形式**：`(T: Type) -> ((f: (T) -> T, x: T) -> T)` - ジェネリックパラメータが関数型を制約
- **HM のワークフロー**：ジェネリックパラメータを通じて関数型を推断し、多态関数合成を実現

**示例説明**：
```yaoxiang
# ✅ 高階多态を支持：ジェネリックが関数型パラメータを制約
call_twice: (T: Type) -> ((f: (T) -> T, x: T) -> T) = {
    return f(f(x))
}
# 使用：call_twice((x) => x + 1, 5)  # T=Int と推断

compose: (A: Type, B: Type, C: Type) -> ((f: (B) -> C, g: (A) -> B, x: A) -> C) = {
    return f(g(x))
}
# 使用：compose((x) => x * 2, (x) => x + 1, 5)  # A=Int, B=Int, C=Int と推断

# ❌ 非支持：ジェネリック制約がない高階関数
# bad_hof: (f, x) => f(f(x))  # HMが推断できず、ジェネリックパラメータが不足
```

**HM推断プロセス**：
1. 高階関数パラメータを識別：`f: (T) -> T`
2. ジェネリック制約を生成：`(T: Type)`
3. ジェネリクスインスタンス化を通じて具体的な型を推断
4. 多态関数合成を実現

### Lambda 式構文規則

**重要な規則**：コードブロック `{ ... }` は最後の式の値を返します。途中で終了する必要がある場合は `return` を使用します。空ブロック `{}` は `Void` を返します。

| 構文形式 | 構文 | 戻り方式 |
|---------|------|----------|
| **コードブロック形式** | `{ statements }` | 最後の式が戻り値；`return` で途中で返回可能 |
| **式形式** | `expression` | 式の値を直接返回 |

**示例**：
```yaoxiang
main: () -> Void = { println("Hello") }         # Void を返回（最後の式は println）
add: (a: Int, b: Int) -> Int = { a + b }        # Int を返回（最後の式は a + b）
empty: () -> Void = {}                          # 空ブロックは Void を返回

# 途中で返回：return を使用
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    n * factorial(n - 1)
}

# 式形式：値を直接返回（return 不要）
add: (a: Int, b: Int) -> Int = a + b            # 正しい：式形式
main: () -> Void = println("Hello")               # 正しい：式形式
```

**核心的な考え方**：
1. 関数定義はHM算法を通じて型推断を行い、尽量推断し、できない場合は明示的にエラー
2. **HM算法の動作原理**：演算子の型制約、関数呼び出し関係などのコンテキスト情報から型を自動的に推断
3. **ジェネリクス支持**：多态関数はジェネリック構文 `(T: Type)` を使用して型パラメータを明示的に制約（RFC-010/011）
4. **推断の境界**：戻り値の型とローカル変数は推断可能；パラメータを持つ関数のパラメータの型は明示的に标注必要（シグネチャまたは lambda 頭の少なくとも片方）
5. 空パラメータ・戻り値なし関数は `name: () -> Void = { ... }` を使用し、RFC-010 と統一
6. 旧構文は退役し、移行ツールを提供

**型推断示例**：
```yaoxiang
# ジェネリック関数：明示的な型パラメータ（RFC-010 統一構文を使用）
identity: (T: Type) -> ((x: T) -> T) = x
map: (T: Type, R: Type) -> ((f: (T) -> R, list: List(T)) -> List(R)) = {
    result = List(R)()
    for item in list { result.push(f(item)) }
    return result
}

# 多态関数：明示的なジェネリック制約を通じて定義（RFC-010/011）
add: (T: Add) -> ((a: T, b: T) -> T) = a + b
print_sum: (a: Int, b: Int) -> Void = { println(a + b) }  # (Int, Int) -> Void と推断

# 高階多态：ジェネリック型注釈を通じてHMが高階多态を支持
call_twice: (T: Type) -> ((f: (T) -> T, x: T) -> T) = { return f(f(x)) }
compose: (A: Type, B: Type, C: Type) -> ((f: (B) -> C, g: (A) -> B, x: A) -> C) = { return f(g(x)) }
```

```yaoxiang
# === 関数定義：HM算法 型推断 ===

# 標準関数：HM算法が戻り値の型を推断（パラメータの型は明示的に必要）
add = (a: Int, b: Int) => a + b            # (a: Int, b: Int) -> Int と推断
main = { println("Hello") }                # () -> Void と推断

# 一部の明示的パラメータ：HM算法が残りを推断
print_sum: (a: Int, b: Int) -> Void = { println(a + b) }  # (Int, Int) -> Void と推断
greet: (name: String) -> Void = { println("Hello " + name) }  # (String) -> Void と推断

# ジェネリック関数：多态型パラメータを明示的に制約（RFC-010 統一構文を使用）
identity: (T: Type) -> ((x: T) -> T) = x
map: (T: Type, R: Type) -> ((f: (T) -> R, list: List(T)) -> List(R)) = {
    # map 関数の実装
    return List(R)()
}

# 再帰関数：HM算法と再帰制約を通じて推断
factorial: (n: Int) -> Int = {
    if n <= 1 { 1 } else { n * factorial(n - 1) }
}

# === 変数代入：HM算法 型推断 ===

# 明示的な型
x: Int = 42

# HM算法が自動的に Int と推断
y = 42                               # Int と推断

# HM算法が自動的に String と推断
name = "YaoXiang"                    # String と推断

# HM算法が自動的に Float と推断
pi = 3.14159                         # Float と推断
```

**HM型推断規則**：

| シナリオ | 構文 | 省略可能部分 | 示例 |
|------|------|----------|------|
| **完全形** | `name: (a: Type, b: Type) -> Ret = (a, b) => ...` | なし | シグネチャ + Lambda 頭 完全 |
| **Lambda 頭を省略** | `name: (a: Type, b: Type) -> Ret = ...` | Lambda 頭 | シグネチャがパラメータを宣言済み |
| **シグネチャを省略** | `name = (a: Type, b: Type) => ...` | シグネチャ | lambda 頭がパラメータの型を提供 |
| **Ret を省略** | `name: (a: Type, b: Type) -> = ...` | 戻り値の型 | HM が戻り値の型を推断 |
| **空パラメータ完全形** | `name: () -> Void = () => { ... }` | なし | 空パラメータ関数完全 |
| **空パラメータ省略形** | `name: () -> Void = { ... }` | Lambda 頭 | `() =>` を省略 |
| **空パラメータ最簡形** | `name = { ... }` | すべて | パラメータなし・戻り値なし最簡形 |
| **変数代入** | `name = value` | 型 | HM が型を推断 |
| **明示的変数** | `name: Type = value` | なし | 明示的な型注釈 |

**基本原則**：
- `->` は関数型の識別子であり、省略できません（否则はタプルとして解析されます）
- 戻り値の型 `Ret` は省略可能で、HM に基づいて関数本体から推断されます
- パラメータが存在する場合、パラメータの型は明示的に出现する必要があります（シグネチャまたは lambda 頭の少なくとも片方）
- その他の部分は、推断可能かつ曖昧さを引入しない場合に省略可能
- 暗黙的な型変換はなく、JavaScript のような混乱を避ける

## 詳細な設計

### 構文糖の展開

省略の有無に関わらず、最終的には統一的な中間表現に正規化されます：

```rust
// 完全形
add: (a: Int, b: Int) -> Int = (a, b) => a + b

// 展開後の IR
let add: (Int, Int) -> Int = |a: Int, b: Int| -> Int {
    return a + b
};

// Lambda 頭を省略
add: (a: Int, b: Int) -> Int = a + b

// 展開後の IR（完全形と同じ）
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
               | identifier '=' block                    # 最も簡潔な形：パラメータなし・戻り値なし

identifier ::= [a-zA-Z_][a-zA-Z0-9_]*

type_expr ::= identifier                     # 型参照
       | '()'                          # 空型
       | '(' parameters ')' '->' type_expr   # 関数型（シグネチャ内にパラメータ名）
       | type_expr '->' type_expr            # 単純な関数型
       | identifier '(' type_expr (',' type_expr)* ')'  # 型応用

expression ::= '(' parameters ')' '=>' block
             | '(' ')' '=>' block
             | '(' parameters ')' '=>' expression

parameters ::= parameter (',' parameter)*
parameter ::= identifier                # 型推断
            | identifier ':' type_expr      # 一部の明示的な型

block ::= '{' statement (',' statement)* '}'
        | expression

statement ::= identifier ':' expression  # 代入文
           | expression                  # 式文（実行するが返回なし）
           | 'return' expression         # 返回文（指定した値を返す）

# 注意：コードブロックは最後の式の値を返す；空ブロック {} は Void と推断
# 例：{ 1 + 1 } は Int を返す；{ println("Hello") } は Void を返す
# 注意：ジェネリックパラメータは (T: Type) 構文を使用し、関数型の一部として独立した BNF 規則は不要
```

### エラー処理

```yaoxiang
# === コンパイルエラー示例 ===

# エラー1：コードブロックの戻り値の型が一致しない
add: (a: Int, b: Int) -> Int = { println(a + b) }
// エラー：ブロックの最後の式は println(...) であり、Void を返すが、シグネチャは Int を期待
// 正しい：add: (a: Int, b: Int) -> Int = a + b
// または：add: (a: Int, b: Int) -> Int = { a + b }

# エラー2：未宣言の型パラメータを使用
identity: (x: T) -> T = x
// エラー：T が未宣言；明示的なジェネリックパラメータが必要（RFC-010）
// 正しい：identity: (T: Type) -> ((x: T) -> T) = x

# 正しい：HM算法が戻り値の型を推断
double = (x: Int) => x + x

# 完全形（段階的に省略）
double: (x: Int) -> Int = (x) => x + x                # 完全
double: (x: Int) -> Int = x + x                       # Lambda 頭を省略
double = (x: Int) => x + x                            # 戻り値の型を省略（HM が推断）
# double = (x) => x + x                               # ❌ パラメータの型は両方の省略不可
```

## 权衡

### 利点

- **構文の統一**：`name: Signature = LambdaBody` モデルがすべてのシナリオをカバー
- **柔軟な省略**：HM が推断可能な任意の部分，省略可能
- **型安全性**：HM算法が型安全性を保証し、暗黙的な型変換を回避
- **再帰支持**：HM算法と再帰制約が自動的に型を推断
- **ゼロ负担**：完全形から最簡形への滑らかな移行

### 欠点

- **移行コスト**：旧コードは変換ツールで移行必要
- **学習コスト**：「完全形 + 任意の省略」モデルを理解する必要がある

## 代替方案

| 方案 | 説明 | 選定外の理由 |
|------|------|-----------|
| HM算法 型推断 | Hindley-Milner算法を使用して型を推断 | ✅ **既に採用**、現代関数型言語の標準 |
| 明示的な型宣言 | すべての型を明示的に記述必須 | 構文簡略化の原則に反し、ボイラープレートコードが増加 |
| 旧構文の維持 | 新旧両方の構文を同時に支持 | 構文の分裂、维护コストが増加 |
| fn キーワード | 関数を区別するための fn キーワードを引入 | 「関数は lambda」という設計に反する |

## 実装策略

### 段階的划分

1. **Phase 1: 構文解析とHM算法**（v0.3）
   - 新構文 `name = lambda` + HM算法 型推断を実装
   - 空パラメータ・戻り値なしのデフォルト埋込みを実装

2. **Phase 2: 移行ツール**（v0.3）
   - `yaoxiang-migrate --old-to-new` ツールを開発
   - 旧構文コードの自動変換

3. **Phase 3: 検証とドキュメンテーション**（v0.3）
   - 旧コードの移行完了検証
   - ドキュメンテーション更新

### 移行ツール

```bash
# 単一ファイルの移行
yaoxiang-migrate --old-to-new src/main.yaoxiang

# プロジェクト全体の移行
yaoxiang-migrate --old-to-new --recursive src/

# 移行のプレビュー（ファイルは変更しない）
yaoxiang-migrate --old-to-new --dry-run src/main.yaoxiang
```

移行規則：
```yaoxiang
# 旧構文
add(Int, Int) -> Int = (a, b) => { a + b }
main() -> Int = { println("Hello"); 0 }
main() = { println("Hello") }

# === 新構文：完全形（シグネチャ完全 + Lambda 頭完全）===
add: (a: Int, b: Int) -> Int = (a, b) => a + b
main: () -> Void = () => { println("Hello") }

# === 省略形：Lambda 頭を省略 ===
add: (a: Int, b: Int) -> Int = a + b
main: () -> Void = { println("Hello") }

# === 省略形：HM が推断 ===
add = (a: Int, b: Int) => a + b              # (a: Int, b: Int) -> Int と推断
main = { println("Hello") }                  # () -> Void と推断

# === 最も簡潔な形 ===
main = {                                      # main: () -> Void = { ... } と等価
    println("Hello")
}
```

### 依存関係

- 外部依存なし
- 独立して実装可能

### リスク

| リスク | 影響 | 軽減措施 |
|------|------|---------|
| 移行の遗漏 | 旧コードのコンパイル失敗 | 移行ツールを提供、すべての旧構文パターンをカバー |
| パーサーのエラー | 構文解析の不安定さ | 十分なテストカバレッジ |

## オープン問題

> 以下の問題は既に設計で解決済みで、付録Aに記録されています。

- ~~Q1: `main() = body` 这种极简写法是否应该保留？~~ → 解決済み：`main = { ... }` として保留
- ~~Q2: 関数名の後の `:` 是否保留？~~ → 解決済み：オプションとして保留；但し、パラメータを持つ関数はシグネチャまたは lambda 頭でパラメータの型を标注必要
- ~~Q3: HM算法是否支持パラメータ型推断？~~ → 解決済み：戻り値/ローカル变量的推断可能；パラメータを持つ関数のパラメータの型は明示的に标注必要
- ~~Q4: 是否引入 `fn` キーワード？~~ → 解決済み：引入なし、関数は lambda
- ~~Q5: 旧代码的迁移策略是什么？~~ → 解決済み：`yaoxiang-migrate` ツールを提供
- ~~Q6: ジェネリック関数の使用方法は？~~ → 解決済み：RFC-010 統一構文 `(T: Type)` を使用

---

## 付録

### 付録A：各言語の関数定義構文参照

| 言語 | 構文スタイル | 特徴 |
|------|---------|------|
| Rust | `fn add(a: i32, b: i32) -> i32 { ... }` | キーワード + 型注釈 |
| Haskell | `add a b = ...` / `add :: Int -> Int -> Int` | 型シグネチャの分離 |
| OCaml | `let add a b = ...` | パラメータの型を省略可能 |
| MoonBit | `fn add(a: Int, b: Int): Int { ... }` | 簡潔な型注釈 |
| TypeScript | `const add = (a: number, b: number): number => ...` | Lambda スタイル |
| Scala | `def add(a: Int, b: Int): Int = { ... }` | def キーワード |
| **YaoXiang** | `name = (a: Int, b: Int) => a + b` | **関数 = lambda、HM が戻り値を推断** |

### 付録B：設計意思決定記録

| 意思決定 | 決定 | 日付 | 記録者 |
|------|------|------|--------|
| 構文スタイル | 新構文 `name: (params) -> Return = body` + HM推断 | 2026-02-03 | @沫郁酱 |
| パラメータの位置 | シグネチャ内でパラメータ名を宣言し、RFC-010 と統一 | 2026-02-03 | @沫郁酱 |
| デフォルト埋込み | 空パラメータ関数はシグネチャを省略可能、空ブロック `{}` は `Void` と推断 | 2026-02-03 | @沫郁酱 |
| 型推断 | HM算法が自動的に推断、できない場合は明示的に | 2026-01-06 | @沫郁酱 |
| 旧構文 | 退役、移行ツールを提供 | 2026-01-06 | @沫郁酱 |
| fn キーワード | 引入なし | 2026-01-06 | @沫郁酱 |
| 再帰宣言 | HM算法と再帰制約が自動的に推断 | 2026-01-06 | @沫郁酱 |

### 付録C：用語集

| 用語 | 定義 |
|------|------|
| HM算法 | Hindley-Milner型推断算法、関数と変数の型を自動的に推断 |
| ジェネリクス | 型パラメータ `(T: Type)` を使用して多态関数を制約、例：`identity: (T: Type) -> ((x: T) -> T) = x`（RFC-010） |
| デフォルト型埋込み | 空パラメータ・戻り値なし関数は `-> Void` を省略可能、コンパイラが自動的に埋込み |
| 構文糖 | コードをより読みやすくする構文の単純化 |
| 正規化 | 構文形式を統一的な内部表現に変換 |
| 関数は lambda | 関数の本質は lambda 変数であり、型はHM算法を通じて自動的に推断 |

---

## 参考文献

- [MoonBit 言語設計](https://moonbitlang.com/)
- [Rust 関数構文](https://doc.rust-lang.org/book/ch03-03-how-functions-work.html)
- [Haskell 型システム](https://www.haskell.org/tutorial/patterns.html)
- [OCaml 型推断](https://v2.ocaml.org/manual/)
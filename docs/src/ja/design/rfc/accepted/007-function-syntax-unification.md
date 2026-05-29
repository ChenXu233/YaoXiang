---
title: "RFC-007：関数定義構文の統一方案"
---

# RFC-007: 関数定義構文の統一方案

> **状態**: 承認済み
> **作者**: 沫郁酱
> **作成日**: 2025-01-05
> **最終更新**: 2026-03-21（型コンストラクタ規則とコードブロック戻り値意味の整合）

## 要約

本 RFC は YaoXiang 言語の**関数定義構文**の最終方案を定義します。統一構文 `name: (params) -> Return = body` を使用し、RFC-010 の `name: type = value` モデルと完全一致的ます。

曖昧さを避けるため：関数に引数がある場合は、パラメータ型は「シグネチャ」または「lambda 頭」の少なくとも片方に明示的に注釈を付ける必要があり、両方省略した場合は拒否されます。

コードブロック `{ ... }` の最後の式が戻り値となります。空ブロック `{}` は `Void` を返します。

## 動機

### なぜこの機能が必要인가？

1. **構文の一貫性**: 旧構文の歴史的包袱を排除し、スタイルを統一
2. **簡潔性**: HM アルゴリズムが自動的に型を推論し、ボイラープレートコードを削減
3. **型安全性**: HM アルゴリズムが型安全性を保証し、推論できない場合のみ明示的に注釈
4. **言語の成熟度**: HM アルゴリズムは現代関数型言語の成熟した方案

### 統一構文モデル

**基本原則**: `name: Signature = LambdaBody`

- **完全形式**: シグネチャ（パラメータ名 + 型 + `->` + 戻り型）+ Lambda 頭（パラメータ名を含む）
- **省略規則**: 曖昧さを導入しない前提下での省略を許可
  - `->` は省略不可（関数型の印。否则会被解析为元组）
  - **引数がある場 合**、パラメータ型はシグネチャまたは lambda 頭の少なくとも片方に明示的に出現する必要がある
  - Lambda 頭は省略可能 → シグネチャがパラメータ名と型を宣言している場合
  - 戻り型は明示的に注釈可能で、推論可能な場合は省略可能

```yaoxiang
# 完全形式（シグネチャ完全 + Lambda 頭完全）
add: (a: Int, b: Int) -> Int = (a, b) => a + b

# 省略：Lambda 頭を省略（シグネチャがパラメータを宣言済み）
add: (a: Int, b: Int) -> Int = a + b

# 省略：シグネチャを省略（lambda 頭がパラメータ型を注釈）
add = (a: Int, b: Int) => a + b

# ❌ エラー：両方にパラメータ型を注釈していない
# add = (a, b) => a + b
```

### 設計目標

```yaoxiang
# === 完全形式 ===
add: (a: Int, b: Int) -> Int = (a, b) => { a + b }

# === 省略形式 ===
add: (a: Int, b: Int) -> Int = a + b                 # Lambda 頭を省略
add = (a: Int, b: Int) => a + b                      # シグネチャを省略

# === 空引数関数 ===
main: () -> Void = () => { println("Hello") }          # 完全形式
main: () -> Void = { println("Hello") }                # Lambda 頭を省略
main = { println("Hello") }                            # 最小形式（() -> Void と推論）

# === ジェネリクス関数（RFC-010 統一構文を使用）===
identity: (T: Type) -> ((x: T) -> T) = (x) => x         # 完全形式
identity: (T: Type) -> ((x: T) -> T) = x                # Lambda 頭を省略
identity = (x: T) => x                                  # シグネチャを省略（lambda 頭が型を注釈）

# === 再帰関数 ===
factorial: (n: Int) -> Int = (n) => {
    if n <= 1 { 1 } else { n * factorial(n - 1) }
}
```

### 構文規則

| シナリオ | 構文 | 説明 |
|------|------|------|
| **完全形式** | `name: (a: Type, b) -> Ret = (a, b) => { return ... }` | シグネチャ + Lambda 頭 完全 |
| **Lambda 頭省略** | `name: (a: Type, b: Type) -> Ret = { ... }` | シグネチャがパラメータを宣言済み |
| **シグネチャ省略** | `name = (a: Type, b: Type) => { ... }` | lambda 頭がパラメータ型を注釈 |
| **空引数完全** | `name: () -> Void = () => { return ... }` | 空引数関数完全 |
| **空引数省略** | `name: () -> Void = { return ... }` | Lambda 頭を省略 |
| **空引数最小** | `name = { return ... }` | 引数なし戻り値なし最小 |

**注意**: ブロック `{ ... }` の最後の式が戻り値となります。途中で終了する必要がある場合は `return` を使用します。空ブロック `{}` は `Void` と推論されます。

**注意**: `->` は関数型の印であり、省略できません（否则会被解析为元组）。

**重要**: `if` 式は分支を囲むために波括弧 `{}` を使用하며、`then/else` キーワードはサポートしません：
```yaoxiang
# 正しい：波括弧を使用
if n <= 1 { return 1 } else { return n * factorial(n - 1) }

# エラー：then/else キーワードはサポートしない
# if n <= 1 then return 1 else return n * factorial(n - 1)
```

## 提案

### HM アルゴリズムと高階多态サポート

**核 心機能**: HM アルゴリズムはジェネリック型注釈 통해高階多态（Higher-rank polymorphism）をサポート

**設計原理**:
- **高階関数**: 関数をパラメータとして渡す場合、関数型をジェネリックに制約する必要がある
- **型注釈形式**: `(T: Type) -> ((f: (T) -> T, x: T) -> T)` - ジェネリックパラメータが関数型を制約
- **HM ワークフロー**: ジェネリックパラメータを通じて関数型を推論し、多态関数合成を実現

**示例説明**:
```yaoxiang
# ✅ 高階多态サポート：ジェネリック制約関数型パラメータ
call_twice: (T: Type) -> ((f: (T) -> T, x: T) -> T) = {
    return f(f(x))
}
# 使用：call_twice((x) => x + 1, 5)  # T=Int と推論

compose: (A: Type, B: Type, C: Type) -> ((f: (B) -> C, g: (A) -> B, x: A) -> C) = {
    return f(g(x))
}
# 使用：compose((x) => x * 2, (x) => x + 1, 5)  # A=Int, B=Int, C=Int と推論

# ❌ サポート外：ジェネリック制約缺失の高階関数
# bad_hof: (f, x) => f(f(x))  # HM が推論できず、ジェネリックパラメータ缺失
```

**HM 推論プロセス**:
1. 高階関数パラメータを識別: `f: (T) -> T`
2. ジェネリック制約を生成: `(T: Type)`
3. ジェネリックインスタンス化を通じて具体型を推論
4. 多态関数合成を実現

### Lambda 式構文規則

**重要規則**: コードブロック `{ ... }` は最後の式の値を返します。途中で終了する必要がある場合は `return` を使用します。空ブロック `{}` は `Void` を返します。

| 構文形式 | 構文 | 戻り方式 |
|---------|------|----------|
| **コードブロック形式** | `{ statements }` | 最後の式が戻り値; `return` で早期返回 가능 |
| **式形式** | `expression` | 式値を直接返回 |

**示例**:
```yaoxiang
main: () -> Void = { println("Hello") }         # Void を返す（最後の式が println）
add: (a: Int, b: Int) -> Int = { a + b }        # Int を返す（最後の式が a + b）
empty: () -> Void = {}                          # 空ブロックは Void を返す

# 早期返回：return を使用
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    n * factorial(n - 1)
}

# 式形式：値を直接返す（return は不要）
add: (a: Int, b: Int) -> Int = a + b            # 正しい：式形式
main: () -> Void = println("Hello")               # 正しい：式形式
```

**核心理念**:
1. 関数定義はHMアルゴリズムを通じて型推論を行い、尽量推論し、推論できない場合は明示的にエラーを報告
2. **HM アルゴリズム動作原理**: 演算子の型制約、関数呼び出し関係などのコンテキスト情報によって自動的に型を推論
3. **ジェネリクスサポート**: 多态関数はジェネリック構文 `(T: Type)` を使用して型パラメータを明示的に制約（RFC-010/011）
4. **推論境界**: 戻り型と局所変数は推論可能; 引数付き関数のパラメータ型は明示的に注釈が必要（シグネチャまたは lambda 頭のいずれか）
5. 空引数戻り値なし関数は `name: () -> Void = { ... }` を使用하며、RFC-010 と統一
6. 旧構文は退役し、移行ツールを提供

**型推論示例**:
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
print_sum: (a: Int, b: Int) -> Void = { println(a + b) }  # (Int, Int) -> Void と推論

# 高階多态：ジェネリック型注釈を通じて HM がより高阶多态をサポート
call_twice: (T: Type) -> ((f: (T) -> T, x: T) -> T) = { return f(f(x)) }
compose: (A: Type, B: Type, C: Type) -> ((f: (B) -> C, g: (A) -> B, x: A) -> C) = { return f(g(x)) }
```

```yaoxiang
# === 関数定義：HM アルゴリズム型推論 ===

# 標準関数：HM アルゴリズムが戻り型を推論（パラメータ型は明示が必要）
add = (a: Int, b: Int) => a + b            # (a: Int, b: Int) -> Int と推論
main = { println("Hello") }                # () -> Void と推論

# 一部明示パラメータ：HM アルゴリズムが残余部分を推論
print_sum: (a: Int, b: Int) -> Void = { println(a + b) }  # (Int, Int) -> Void と推論
greet: (name: String) -> Void = { println("Hello " + name) }  # (String) -> Void と推論

# ジェネリック関数：多态型パラメータを明示的に制約（RFC-010 統一構文を使用）
identity: (T: Type) -> ((x: T) -> T) = x
map: (T: Type, R: Type) -> ((f: (T) -> R, list: List(T)) -> List(R)) = {
    # map 関数の実装
    return List(R)()
}

# 再帰関数：HM アルゴリズムと再帰制約を通じて推論
factorial: (n: Int) -> Int = {
    if n <= 1 { 1 } else { n * factorial(n - 1) }
}

# === 変数代入：HM アルゴリズム型推論 ===

# 明示的型
x: Int = 42

# HM アルゴリズムが自動的に Int と推論
y = 42                               # Int と推論

# HM アルゴリズムが自動的に String と推論
name = "YaoXiang"                    # String と推論

# HM アルゴリズムが自動的に Float と推論
pi = 3.14159                         # Float と推論
```

**HM 型推論規則**:

| シナリオ | 構文 | 省略可能部分 | 示例 |
|------|------|----------|------|
| **完全形式** | `name: (a: Type, b: Type) -> Ret = (a, b) => ...` | なし | シグネチャ + Lambda 頭 完全 |
| **Lambda 頭省略** | `name: (a: Type, b: Type) -> Ret = ...` | Lambda 頭 | シグネチャがパラメータを宣言済み |
| **シグネチャ省略** | `name = (a: Type, b: Type) => ...` | シグネチャ | lambda 頭がパラメータ型を提供 |
| **Ret 省略** | `name: (a: Type, b: Type) -> = ...` | 戻り型 | HM が戻り型を推論 |
| **空引数完全** | `name: () -> Void = () => { ... }` | なし | 空引数関数完全 |
| **空引数省略** | `name: () -> Void = { ... }` | Lambda 頭 | `() =>` を省略 |
| **空引数最小** | `name = { ... }` | 全部 | 引数なし戻り値なし最小 |
| **変数代入** | `name = value` | 型 | HM が型を推論 |
| **明示的変数** | `name: Type = value` | なし | 明示的型注釈 |

**基本原則**:
- `->` は関数型の印であり、省略できません（否则会被解析为元组）
- 戻り型 `Ret` は省略可能で、HM が関数体に基づいて推論
- 引数がある場合は、パラメータ型は明示的に出現する必要があります（シグネチャまたは lambda 頭のいずれか）
- 其余部分是可推断且不引入歧义时可省略
- 暗黙的型変換なし、JavaScript 式の混乱を回避

## 詳細設計

### 構文糖の展開

省略の有無に関わらず、最終的には統一中間表現に正規化されます：

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

// シグネチャを省略（lambda 頭がパラメータ型を注釈）
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
               | identifier '=' block                    # 最小形式：引数なし戻り値なし

identifier ::= [a-zA-Z_][a-zA-Z0-9_]*

type_expr ::= identifier                     # 型参照
       | '()'                          # 空型
       | '(' parameters ')' '->' type_expr   # 関数型（シグネチャ内にパラメータ名）
       | type_expr '->' type_expr            # 単純関数型
       | identifier '(' type_expr (',' type_expr)* ')'  # 型応用

expression ::= '(' parameters ')' '=>' block
             | '(' ')' '=>' block
             | '(' parameters ')' '=>' expression

parameters ::= parameter (',' parameter)*
parameter ::= identifier                # 型推論
            | identifier ':' type_expr      # 一部明示的型

block ::= '{' statement (',' statement)* '}'
        | expression

statement ::= identifier ':' expression  # 代入文
           | expression                  # 式文（実行するが返回しない）
           | 'return' expression         # 返回文（指定した値を返回）

# 注意：コードブロックは最後の式の値を返す；空ブロック {} は Void と推論
# 例：{ 1 + 1 } は Int を返す；{ println("Hello") } は Void を返す
# 注意：ジェネリックパラメータは (T: Type) 構文を使用し、関数型の一部として、独立した BNF 規則は不要
```

### エラー処理

```yaoxiang
# === コンパイルエラー示例 ===

# エラー1：コードブロック戻り型が不一致
add: (a: Int, b: Int) -> Int = { println(a + b) }
// エラー：ブロックの最後の式は println(...) であり、Void を返すが、シグネチャは Int を期待
// 正しい：add: (a: Int, b: Int) -> Int = a + b
// または：add: (a: Int, b: Int) -> Int = { a + b }

# エラー2：未宣言の型パラメータを使用
identity: (x: T) -> T = x
// エラー：T が未宣言；明示的ジェネリックパラメータが必要（RFC-010）
// 正しい：identity: (T: Type) -> ((x: T) -> T) = x

# 正しい：HM アルゴリズムが戻り型を推論
double = (x: Int) => x + x

# 完全形式（段階的に省略）
double: (x: Int) -> Int = (x) => x + x                # 完全
double: (x: Int) -> Int = x + x                       # Lambda 頭を省略
double = (x: Int) => x + x                            # 戻り型を省略（HM が推論）
# double = (x) => x + x                               # ❌ パラメータ型は両 方省略不可
```

## トレードオフ

### メリット

- **構文の統一**: `name: Signature = LambdaBody` モデルですべてのシナリオをカバー
- **柔軟な省略**: HM アルゴリズムが推論可能な任意の部分で省略可能
- **型安全性**: HM アルゴリズムが型安全性を保証し、暗黙的型変換を回避
- **再帰サポート**: HM アルゴリズムと再帰制約が自動的に型を推論
- **ゼロ負担**: 完全から最小まで滑らかな移行

### デメリット

- **移行コスト**: 旧コードは変換ツールで変換必要
- **学習コスト**: 「完全形式 + 任意の省略」モデルを理解する必要がある

## 代替方案

| 方案 | 説明 | 为什么不选 |
|------|------|-----------|
| HM アルゴリズム型推論 | Hindley-Milner アルゴリズムを使用して型を推論 | ✅ **已采用**、現代関数型言語標準 |
| 明示的型宣言 | すべての型を明示的に書く必要 | 簡略化構文原則に違反、ボイラープレートコードが増加 |
| 旧構文の保持 | 新旧両方の構文をサポート | 構文分裂、メンテコスト高 |
| fn キーワード | 関数と変数を区別するために fn を導入 | 「関数は lambda」の設計に違反 |

## 実装戦略

### 段階的划分

1. **Phase 1: 構文解析と HM アルゴリズム**（v0.3）
   - 新構文 `name = lambda` + HM アルゴリズム型推論を実装
   - 空引数戻り値なしのデフォルト填充を実装

2. **Phase 2: 移行ツール**（v0.3）
   - `yaoxiang-migrate --old-to-new` ツールを開発
   - 旧構文コードを自動的に変換

3. **Phase 3: 検証とドキュメンテーション**（v0.3）
   - 旧コード移行完了検証
   - ドキュメンテーション更新

### 移行ツール

```bash
# 単一ファイルの移行
yaoxiang-migrate --old-to-new src/main.yaoxiang

# プロジェクト全体の移行
yaoxiang-migrate --old-to-new --recursive src/

# 移行プレビュー（ファイルは変更しない）
yaoxiang-migrate --old-to-new --dry-run src/main.yaoxiang
```

移行規則：
```yaoxiang
# 旧構文
add(Int, Int) -> Int = (a, b) => { a + b }
main() -> Int = { println("Hello"); 0 }
main() = { println("Hello") }

# === 新構文：完全形式（シグネチャ完全 + Lambda 頭完全）===
add: (a: Int, b: Int) -> Int = (a, b) => a + b
main: () -> Void = () => { println("Hello") }

# === 省略：Lambda 頭を省略 ===
add: (a: Int, b: Int) -> Int = a + b
main: () -> Void = { println("Hello") }

# === 省略：HM が推論 ===
add = (a: Int, b: Int) => a + b              # (a: Int, b: Int) -> Int と推論
main = { println("Hello") }                  # () -> Void と推論

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
| 移行遗漏 | 旧コードのコンパイル失敗 | 移行ツールを提供し、すべての旧構文パターンをカバー |
| パーサーエラー | 構文解析が不安定 | 十分なテストカバー |

## 開放問題

> 以下的问题是已在设计中解决，记录在付録A。

- ~~Q1: `main() = body` 这种极简写法是否应该保留？~~ → 已解决：保留为 `main = { ... }`
- ~~Q2: 函数名后的 `:` 是否保留？~~ → 已解决：可选保留；但有参函数仍需在签名或 lambda 头标注参数类型
- ~~Q3: HM 算法是否支持参数类型推断？~~ → 已解决：返回值/局部可推断；有参函数的参数类型需显式标注
- ~~Q4: 是否引入 `fn` 关键字？~~ → 已解决：不引入，函数就是 lambda
- ~~Q5: 旧代码的迁移策略是什么？~~ → 已解决：提供 `yaoxiang-migrate` 工具
- ~~Q6: 泛型函数如何使用？~~ → 已解决：使用RFC-010统一语法 `(T: Type)`

---

## 付録

### 付録A：各言語関数定義構文参考

| 言語 | 構文スタイル | 特徴 |
|------|---------|------|
| Rust | `fn add(a: i32, b: i32) -> i32 { ... }` | キーワード + 型注釈 |
| Haskell | `add a b = ...` / `add :: Int -> Int -> Int` | 型シグネチャ分離 |
| OCaml | `let add a b = ...` | パラメータ型を省略可能 |
| MoonBit | `fn add(a: Int, b: Int): Int { ... }` | 簡潔な型注釈 |
| TypeScript | `const add = (a: number, b: number): number => ...` | Lambda スタイル |
| Scala | `def add(a: Int, b: Int): Int = { ... }` | def キーワード |
| **YaoXiang** | `name = (a: Int, b: Int) => a + b` | **関数 = lambda、HM が戻り値を推論** |

### 付録B：設計决策記録

| 决策 | 決定 | 日付 | 記録人 |
|------|------|------|--------|
| 構文スタイル | 新構文 `name: (params) -> Return = body` + HM推論 | 2026-02-03 | @沫郁酱 |
| パラメータ位置 | パラメータ名はシグネチャ内に宣言し、RFC-010 と統一 | 2026-02-03 | @沫郁酱 |
| デフォルト填充 | 空引数関数はシグネチャを省略可能、空ブロック `{}` は `Void` と推論 | 2026-02-03 | @沫郁酱 |
| 型推論 | HM アルゴリズムが自動的に推論、推論できない場合は明示 | 2026-01-06 | @沫郁酱 |
| 旧構文 | 退役、移行ツールを提供 | 2026-01-06 | @沫郁酱 |
| fn キーワード | 導入しない | 2026-01-06 | @沫郁酱 |
| 再帰宣言 | HM アルゴリズムと再帰制約が自動的に推論 | 2026-01-06 | @沫郁酱 |

### 付録C：用語集

| 用語 | 定義 |
|------|------|
| HM アルゴリズム | Hindley-Milner 型推論アルゴリズムで、関数と変数の型を自動的に推論 |
| ジェネリクス | 型パラメータ `(T: Type)` を使用して多态関数を制約、例：`identity: (T: Type) -> ((x: T) -> T) = x`（RFC-010） |
| デフォルト型填充 | 空引数戻り値なし関数は `-> Void` を省略可能、コンパイラが自動的に填充 |
| 構文糖 | コードをより読みやすくする構文簡略化写法 |
| 正規化 | 構文形式を統一内部表現に変換 |
| 関数は lambda | 関数の本質は lambda 変数であり、型は HM アルゴリズムによって自動的に推論 |

---

## 参考文献

- [MoonBit 言語設計](https://moonbitlang.com/)
- [Rust 関数構文](https://doc.rust-lang.org/book/ch03-03-how-functions-work.html)
- [Haskell 型システム](https://www.haskell.org/tutorial/patterns.html)
- [OCaml 型推論](https://v2.ocaml.org/manual/)
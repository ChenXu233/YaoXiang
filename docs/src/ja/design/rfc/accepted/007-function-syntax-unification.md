---
title: RFC-007：関数定義構文統一方案
---

# RFC-007: 関数定義構文統一方案

> **ステータス**: 承認済み
> **著者**: 沫郁酱
> **作成日**: 2025-01-05
> **最終更新**: 2026-03-21（型コンストラクタ規則とコードブロック戻り値セマンティクスの整合）

## 概要

本 RFC は YaoXiang 言語の**関数定義構文**の最終方案を定めます。統一構文 `name: (params) -> Return = body` を使用し、RFC-010 の `name: type = value` モデルと完全に一致させます。

曖昧さを避けるため：関数に輸入パラメータがある場合、パラメータの型は「シグネチャ」または「lambda 頭」の少なくとも片方に明示的に标注されなければならず、両方省略した場合は拒否されます。

コードブロック `{ ... }` の最後の式が戻り値となります。空ブロック `{}` は `Void` を返します。

## 動機

### なぜこの機能が必要인가

1. **構文の一貫性**：旧構文の歴史的包袱を消除し、スタイルを統一する
2. **簡潔性**：HM算法が型を自動推論し、样板コードを減らす
3. **型安全性**：HM算法が型安全性を保証し、推論できない場合のみ明示的に标注する
4. **言語成熟度**：HM算法は現代関数型言語の成熟した方案である

### 統一構文モデル

**核心原則**：`name: Signature = LambdaBody`

- **完全形式**：シグネチャ（パラメータ名 + 型 + `->` + 戻り型） + Lambda頭（パラメータ名を含む）
- **省略規則**：曖昧さを引入しない的前提下尽量省略
  - `->` は省略不可（関数型の印であり、省略するとタプルとしてパースされる）
  - **入力パラメータがある場合**、パラメータの型はシグネチャまたは lambda 頭の少なくとも片方に明示的に出現する必要がある
  - Lambda 頭は省略可能 → シグネチャが既にパラメータ名と型を宣言している場合
  - 戻り型は明示的に标注可能であり、推論可能な場合は省略可能

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
main = { println("Hello") }                            # 最も省略形式（推論で () -> Void）

# === ジェネリック関数（RFC-010 統一構文を使用）===
identity: (T: Type) -> ((x: T) -> T) = (x) => x         # 完全形式
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
| **完全形式** | `name: (a: Type, b) -> Ret = (a, b) => { return ... }` | シグネチャ + Lambda 頭 完全 |
| **Lambda 頭を省略** | `name: (a: Type, b: Type) -> Ret = { ... }` | シグネチャがパラメータを宣言済み |
| **シグネチャを省略** | `name = (a: Type, b: Type) => { ... }` | lambda 頭がパラメータの型を标注 |
| **空パラメータ完全** | `name: () -> Void = () => { return ... }` | 空パラメータ関数 完全 |
| **空パラメータ省略** | `name: () -> Void = { return ... }` | Lambda 頭を省略 |
| **空パラメータ最省略** | `name = { return ... }` | パラメータなし・返り値なし 最省略 |

**注意**：ブロック `{ ... }` の最後の式が戻り値となります。途中で終了する必要がある場合は `return` を使用します。空ブロック `{}` は `Void` と推論されます。

**注意**：`->` は関数型の印であり、省略できません（省略するとタプルとしてパースされます）。

**重要**：`if` 式は分岐を波括弧 `{}` で包み、`then/else` キーワードはサポートしません：

```yaoxiang
# 正しい：波括弧を使用
if n <= 1 { return 1 } else { return n * factorial(n - 1) }

# エラー：then/else キーワードはサポートしない
# if n <= 1 then return 1 else return n * factorial(n - 1)
```

## 提案

### HM算法と高階多态支持

**核心機能**：HM算法はジェネリック型注釈を通じて高階多态（Higher-rank polymorphism）をサポートする

**設計原理**：

- **高階関数**：関数をパラメータとして渡す際、関数型を制約するジェネリックが必要
- **型注釈形式**：`(T: Type) -> ((f: (T) -> T, x: T) -> T)` - ジェネリックパラメータが関数型を制約
- **HM動作フロー**：ジェネリックパラメータを通じて関数型を推論し、多态関数合成を実現

**示例説明**：

```yaoxiang
# ✅ 高階多态をサポート：ジェネリックが関数型パラメータを制約
call_twice: (T: Type) -> ((f: (T) -> T, x: T) -> T) = {
    return f(f(x))
}
# 使用：call_twice((x) => x + 1, 5)  # T=Int と推論

compose: (A: Type, B: Type, C: Type) -> ((f: (B) -> C, g: (A) -> B, x: A) -> C) = {
    return f(g(x))
}
# 使用：compose((x) => x * 2, (x) => x + 1, 5)  # A=Int, B=Int, C=Int と推論

# ❌ サポート外：高階関数にジェネリック制約がない
# bad_hof: (f, x) => f(f(x))  # HMが推論できず、ジェネリックパラメータが不足
```

**HM推論プロセス**：

1. 高階関数パラメータを識別：`f: (T) -> T`
2. ジェネリック制約を生成：`(T: Type)`
3. ジェネリックインスタンス化を通じて具体型を推論
4. 多态関数合成を実現

### Lambda 式構文規則

**重要な規則**：コードブロック `{ ... }` はその最後の式の値を返す。途中で終了する必要がある場合は `return` を使用します。空ブロック `{}` は `Void` を返します。

| 構文形式 | 構文 | 戻り方式 |
|---------|------|----------|
| **コードブロック形式** | `{ statements }` | 最後の式が戻り値。`return` で早期返回可能 |
| **式形式** | `expression` | 式値を直接返す |

**示例**：

```yaoxiang
main: () -> Void = { println("Hello") }         # Void を返す（最後の式が println）
add: (a: Int, b: Int) -> Int = { a + b }        # Int を返す（最後の式が a + b）
empty: () -> Void = {}                          # 空ブロックは Void を返す

# 早期返回：return を使用
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    n * factorial(n - 1)
}

# 式形式：直接値を返す（return 不要）
add: (a: Int, b: Int) -> Int = a + b            # 正しい：式形式
main: () -> Void = println("Hello")               # 正しい：式形式
```

**核心思想**：

1. 関数定義はHM算法を通じて型推論を行い、尽量推論し、推論できない場合は明示的にエラーを報告
2. **HM算法の動作原理**：演算子の型制約、関数呼び出し関係などのコンテキスト情報を通じて型を自動推論
3. **ジェネリックサポート**：多态関数はジェネリック構文 `(T: Type)` を使用して型パラメータを明示的に制約（RFC-010/011）
4. **推論境界**：戻り型と局所変数は推論可能。パラメータ付き関数のパラメータ型は明示的に标注が必要（シグネチャまたは lambda 頭のいずれか）
5. 空パラメータ・返り値なし関数は `name: () -> Void = { ... }` を使用し、RFC-010 と統一
6. 旧構文は退役し、移行ツールを提供

**型推論示例**：

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

# 高階多态：ジェネリック型注釈を通じてHMが高階多态をサポート
call_twice: (T: Type) -> ((f: (T) -> T, x: T) -> T) = { return f(f(x)) }
compose: (A: Type, B: Type, C: Type) -> ((f: (B) -> C, g: (A) -> B, x: A) -> C) = { return f(g(x)) }
```

```yaoxiang
# === 関数定義：HM算法型推論 ===

# 標準関数：HM算法が戻り型を推論（パラメータ型は明示が必要）
add = (a: Int, b: Int) => a + b            # (a: Int, b: Int) -> Int と推論
main = { println("Hello") }                # () -> Void と推論

# 一部明示パラメータ：HM算法が残りを推論
print_sum: (a: Int, b: Int) -> Void = { println(a + b) }  # (Int, Int) -> Void と推論
greet: (name: String) -> Void = { println("Hello " + name) }  # (String) -> Void と推論

# ジェネリック関数：多态型パラメータを明示的に制約（RFC-010 統一構文を使用）
identity: (T: Type) -> ((x: T) -> T) = x
map: (T: Type, R: Type) -> ((f: (T) -> R, list: List(T)) -> List(R)) = {
    # map 関数の実装
    return List(R)()
}

# 再帰関数：HM算法と再帰制約を通じて推論
factorial: (n: Int) -> Int = {
    if n <= 1 { 1 } else { n * factorial(n - 1) }
}

# === 変数代入：HM算法型推論 ===

# 明示的型
x: Int = 42

# HM算法が自動的に Int と推論
y = 42                               # Int と推論

# HM算法が自動的に String と推論
name = "YaoXiang"                    # String と推論

# HM算法が自動的に Float と推論
pi = 3.14159                         # Float と推論
```

**HM型推論規則**：

| シナリオ | 構文 | 省略可能部分 | 示例 |
|------|------|----------|------|
| **完全形式** | `name: (a: Type, b: Type) -> Ret = (a, b) => ...` | なし | シグネチャ + Lambda 頭 完全 |
| **Lambda 頭を省略** | `name: (a: Type, b: Type) -> Ret = ...` | Lambda 頭 | シグネチャがパラメータを宣言済み |
| **シグネチャを省略** | `name = (a: Type, b: Type) => ...` | シグネチャ | lambda 頭がパラメータの型を提供 |
| **戻り Ret を省略** | `name: (a: Type, b: Type) -> = ...` | 戻り型 | HM が戻り型を推論 |
| **空パラメータ完全** | `name: () -> Void = () => { ... }` | なし | 空パラメータ関数 完全 |
| **空パラメータ省略** | `name: () -> Void = { ... }` | Lambda 頭 | `() =>` を省略 |
| **空パラメータ最省略** | `name = { ... }` | 全部 | パラメータなし・返り値なし 最省略 |
| **変数代入** | `name = value` | 型 | HM が型を推論 |
| **明示的変数** | `name: Type = value` | なし | 明示的型注釈 |

**核心原則**：

- `->` は関数型の印であり、省略不可（省略するとタプルとしてパースされる）
- 戻り型 `Ret` は省略可能であり、HM が関数ボディに基づいて推論
- 入力パラメータが存在する場合、パラメータの型は明示的に出現する必要がある（シグネチャまたは lambda 頭のいずれか）
- その余の部分は推論可能かつ曖昧さを引入しない場合に省略可能
- 暗黙的型変換なし、JavaScript 式の混乱を避ける

## 詳細設計

### 構文糖衣展開

省略の有無に関わらず、最終的にはすべて統一中間表現に正規化されます：

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

// シグネチャを省略（lambda 頭がパラメータの型を标注）
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
               | identifier '=' block                    # 最省略形式：パラメータなし・返り値なし

identifier ::= [a-zA-Z_][a-zA-Z0-9_]*

type_expr ::= identifier                     # 型参照
       | '()'                          # 空型
       | '(' parameters ')' '->' type_expr   # 関数型（シグネチャ内にパラメータ名）
       | type_expr '->' type_expr            # 単純関数型
       | identifier '(' type_expr (',' type_expr)* ')'  # 型適用

expression ::= '(' parameters ')' '=>' block
             | '(' ')' '=>' block
             | '(' parameters ')' '=>' expression

parameters ::= parameter (',' parameter)*
parameter ::= identifier                # 型推論
            | identifier ':' type_expr      # 一部明示的型

block ::= '{' statement (',' statement)* '}'
        | expression

statement ::= identifier ':' expression  # 代入文
           | expression                  # 式文（実行するが返らない）
           | 'return' expression         # 返回文（指定した値を返す）

# 注意：コードブロックは最後の式の値を返す。空ブロック {} は Void と推論
# 例：{ 1 + 1 } は Int を返す；{ println("Hello") } は Void を返す
# 注意：ジェネリックパラメータは (T: Type) 構文を使用します。関数型の一部であり、独立した BNF 規則は不要
```

### エラー処理

```yaoxiang
# === コンパイルエラー示例 ===

# エラー1：コードブロック戻り型が一致しない
add: (a: Int, b: Int) -> Int = { println(a + b) }
// エラー：ブロックの最後の式は println(...) であり Void を返すが、シグネチャは Int を期待
// 正しい：add: (a: Int, b: Int) -> Int = a + b
// または：add: (a: Int, b: Int) -> Int = { a + b }

# エラー2：未宣言の型パラメータを使用
identity: (x: T) -> T = x
// エラー：T が未宣言。明示的ジェネリックパラメータが必要（RFC-010）
// 正しい：identity: (T: Type) -> ((x: T) -> T) = x

# 正しい：HM算法が戻り型を推論
double = (x: Int) => x + x

# 完全形式（段階的に省略）
double: (x: Int) -> Int = (x) => x + x                # 完全
double: (x: Int) -> Int = x + x                       # Lambda 頭を省略
double = (x: Int) => x + x                            # 戻り型を省略（HM が推論）
# double = (x) => x + x                               # ❌ パラメータ型は両端省略不可
```

## トレードオフ

### 利点

- **構文の統一**：`name: Signature = LambdaBody` モデルがすべてのシナリオをカバー
- **柔軟な省略**：任意部分を HM 推論可能時に省略可能
- **型安全性**：HM算法が型安全性を保証し、暗黙的型変換を避ける
- **再帰サポート**：HM算法と再帰制約が自動的に型を推論
- **ゼロ負担**：完全から最省略へ滑らかに移行

### 欠点

- **移行コスト**：旧コードは移行ツールで変換が必要
- **学習コスト**：「完全形式 + 任意省略」モデルを理解する必要がある

## 替代方案

| 方案 | 説明 | なぜ選択しない |
|------|------|-----------|
| HM算法型推論 | Hindley-Milner算法を使用して型を推論 | ✅ **採用済み**、現代関数型言語の標準 |
| 明示的型宣言 | すべての型を明示的に記述 | 構文簡略化の原則に反し、样板コードが増加 |
| 旧構文を保持 | 新旧構文を同時にサポート | 構文分裂、メンテコスト増大 |
| fn キーワード | 関数を区別するために fn を導入 | 「関数は lambda」という設計に反する |

## 実装策略

### 段階的划分

1. **Phase 1: 構文解析とHM算法**（v0.3）
   - 新構文 `name = lambda` + HM算法型推論を実装
   - 空パラメータ・返り値なしのデフォルト填充を実装

2. **Phase 2: 移行ツール**（v0.3）
   - `yaoxiang-migrate --old-to-new` ツールを開発
   - 旧構文コードを自動変換

3. **Phase 3: 検証とドキュメント**（v0.3）
   - 旧コード移行完了検証
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

# === 省略：HM 推論 ===
add = (a: Int, b: Int) => a + b              # (a: Int, b: Int) -> Int と推論
main = { println("Hello") }                  # () -> Void と推論

# === 最も省略形式 ===
main = {                                      # main: () -> Void = { ... } と同等
    println("Hello")
}
```

### 依存関係

- 外部依存なし
- 独立して実装可能

### リスク

| リスク | 影響 | 緩和措施 |
|------|------|---------||
| 移行遗漏 | 旧コードのコンパイル失敗 | 移行ツールを提供し、すべての旧構文パターンをカバー |
| パーサーエラー | 構文解析が不安定 | 十分なテストカバレッジ |

## 開放問題

> 以下の問題は設計で既に解決済みであり、付録Aに記録。

- ~~Q1: `main() = body` のような極限省略写法を残すか？~~ → 解決済み：`main = { ... }` として残す
- ~~Q2: 関数名の後の `:` を残すか？~~ → 解決済み：任意で残す可能。ただしパラメータ付き関数はシグネチャまたは lambda 頭でパラメータ型を标注必要
- ~~Q3: HM算法はパラメータ型推論をサポートするか？~~ → 解決済み：戻り値/局所変数は推論可能。パラメータ付き関数のパラメータ型は明示的に标注必要
- ~~Q4: `fn` キーワードを導入するか？~~ → 解決済み：導入せず、関数は lambda
- ~~Q5: 旧コードの移行戦略は？~~ → 解決済み：`yaoxiang-migrate` ツールを提供
- ~~Q6: ジェネリック関数の使用方法は？~~ → 解決済み：RFC-010 統一構文 `(T: Type)` を使用

---

## 付録

### 付録A：各言語の関数定義構文参照

| 言語 | 構文スタイル | 特徴 |
|------|---------|------|
| Rust | `fn add(a: i32, b: i32) -> i32 { ... }` | キーワード + 型注釈 |
| Haskell | `add a b = ...` / `add :: Int -> Int -> Int` | 型シグネチャ分離 |
| OCaml | `let add a b = ...` | パラメータ型を省略可能 |
| MoonBit | `fn add(a: Int, b: Int): Int { ... }` | 簡潔な型注釈 |
| TypeScript | `const add = (a: number, b: number): number => ...` | Lambda スタイル |
| Scala | `def add(a: Int, b: Int): Int = { ... }` | def キーワード |
| **YaoXiang** | `name = (a: Int, b: Int) => a + b` | **関数 = lambda、HM が戻り値を推論** |

### 付録B：設計意思決定記録

| 意思決定 | 決定 | 日付 | 記録者 |
|------|------|------|--------|
| 構文スタイル | 新構文 `name: (params) -> Return = body` + HM推論 | 2026-02-03 | @沫郁酱 |
| パラメータ位置 | シグネチャ内にパラメータ名を宣言、RFC-010 と統一 | 2026-02-03 | @沫郁酱 |
| デフォルト填充 | 空パラメータ関数はシグネチャを省略可能、空ブロック `{}` は `Void` と推論 | 2026-02-03 | @沫郁酱 |
| 型推論 | HM算法が自動推論、推論できない場合は明示的に | 2026-01-06 | @沫郁酱 |
| 旧構文 | 退役、移行ツールを提供 | 2026-01-06 | @沫郁酱 |
| fn キーワード | 導入せず | 2026-01-06 | @沫郁酱 |
| 再帰宣言 | HM算法と再帰制約が自動推論 | 2026-01-06 | @沫郁酱 |

### 付録C：用語集

| 用語 | 定義 |
|------|------|
| HM算法 | Hindley-Milner 型推論算法。関数と変数の型を自動推論 |
| ジェネリック | 型パラメータ `(T: Type)` を使用して多态関数を制約。<br>例：`identity: (T: Type) -> ((x: T) -> T) = x`（RFC-010） |
| デフォルト型填充 | 空パラメータ・返り値なし関数で `-> Void` を省略可能、コンパイラが自動填充 |
| 構文糖衣 | コードを読みやすくする構文簡略化写法 |
| 正規化 | 構文形式を統一内部表現に変換 |
| 関数即 lambda | 関数の本質は lambda 変数であり、型はHM算法を通じて自動推論 |

---

## 参考文献

- [MoonBit 言語設計](https://moonbitlang.com/)
- [Rust 関数構文](https://doc.rust-lang.org/book/ch03-03-how-functions-work.html)
- [Haskell 型システム](https://www.haskell.org/tutorial/patterns.html)
- [OCaml 型推論](https://v2.ocaml.org/manual/)
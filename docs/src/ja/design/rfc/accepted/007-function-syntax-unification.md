```yaml
---
title: "RFC-007: 関数定義構文統一案"
issue: "#131"
status: "受諾済"
author: "沫郁酱"
created: "2025-01-05"
updated: "2026-07-05（GH Issue #131 に同期）"
---
```

# RFC-007: 関数定義構文統一案

## 概要

本 RFC は YaoXiang 言語の**関数定義構文**の最終案を確定するものである。統一構文 `name: (params) -> Return = body` を採用し、RFC-010 の `name: type = value` モデルと完全に一致する。

曖昧さを避けるため：関数が入力パラメータを持つ場合、パラメータ型は「シグネチャ」または「ラムダ頭（lambda head）」の少なくとも一方で明示的に注釈付けされなければならない；両方を省略することは拒否される。

コードブロック `{ ... }` の中では値返却に `return` を使用しなければならない；`return` がない場合、デフォルトで `Void` を返す。式形式 `= expr` は直接値を返す。

## 動機

### なぜこの機能が必要か？

1. **構文の一貫性**：古い構文の歴史的負債を排除し、スタイルを統一する
2. **簡潔性**：HM アルゴリズム（type inference）が自動的に型を推論し、定型コードを削減する
3. **型安全性**：HM アルゴリズムが型安全性を保証し、推論できない場合のみ明示的に注釈する
4. **言語の成熟度**：HM アルゴリズムは現代的な関数型言語における成熟したソリューションである

### 統一構文モデル

**核心原則**：`name: Signature = LambdaBody`

- **完全形式**：シグネチャ（パラメータ名 + 型 + `->` + 戻り型を含む）+ ラムダ頭（パラメータ名を含む）
- **短縮規則**：曖昧さを導入しない範囲で可能な限り省略する
  - `->` は省略不可（関数型の目印であり、省略するとタプル（tuple type）として解析される）
  - **入力パラメータがある場合**、パラメータ型はシグネチャまたはラムダ頭の少なくとも一方で明示的に出現しなければならない
  - ラムダ頭は省略可能 → シグネチャが既にパラメータ名と型を宣言している場合
  - 戻り型は明示的に注釈可能、推論可能な場合は省略可能

```yaoxiang
# 完全形式（シグネチャ完全 + ラムダ頭完全）
add: (a: Int, b: Int) -> Int = (a, b) => a + b

# 短縮：ラムダ頭を省略（シグネチャがパラメータを宣言）
add: (a: Int, b: Int) -> Int = a + b

# 短縮：シグネチャを省略（ラムダ頭がパラメータ型を注釈）
add = (a: Int, b: Int) => a + b

# ❌ エラー：両側でパラメータ型が注釈されていない
# add = (a, b) => a + b
```

### 設計目標

```yaoxiang
# === 完全形式 ===
add: (a: Int, b: Int) -> Int = (a, b) => { a + b }

# === 短縮形式 ===
add: (a: Int, b: Int) -> Int = a + b                 # ラムダ頭を省略
add = (a: Int, b: Int) => a + b                      # シグネチャを省略

# === 空パラメータ関数 ===
main: () -> Void = () => { println("Hello") }          # 完全形式
main: () -> Void = { println("Hello") }                # ラムダ頭を省略
main = { println("Hello") }                            # 最短形式（() -> Void と推論）

# === ジェネリック関数（RFC-010 統一構文を使用）===
identity: (T: Type) -> ((x: T) -> T) = (x) => x         # 完全形式
identity: (T: Type) -> ((x: T) -> T) = x                # ラムダ頭を省略
identity = (x: T) => x                                  # シグネチャを省略（ラムダ頭が型を注釈）

# === 再帰関数 ===
factorial: (n: Int) -> Int = (n) => {
    if n <= 1 { return 1 } else { return n * factorial(n - 1) }
}
```

### 構文規則

| シナリオ | 構文 | 説明 |
|------|------|------|
| **完全形式** | `name: (a: Type, b) -> Ret = (a, b) => { return ... }` | シグネチャ + ラムダ頭完全 |
| **ラムダ頭省略** | `name: (a: Type, b: Type) -> Ret = { ... }` | シグネチャがパラメータを宣言 |
| **シグネチャ省略** | `name = (a: Type, b: Type) => { ... }` | ラムダ頭がパラメータ型を注釈 |
| **空パラメータ完全** | `name: () -> Void = () => { return ... }` | 空パラメータ関数完全 |
| **空パラメータ短縮** | `name: () -> Void = { return ... }` | ラムダ頭を省略 |
| **空パラメータ最短** | `name = { return ... }` | 無パラメータ・無戻り最短 |

**注意**：コードブロック `{ ... }` の中では値返却に `return` を使用しなければならない；`return` がない場合、デフォルトで `Void` を返す。式形式 `= expr` は直接値を返す。

**注意**：`->` は関数型（function type）の目印であり、省略不可（省略するとタプルとして解析される）。

**重要**：`if` 式は中括弧 `{}` で分岐を囲み、`then/else` キーワードはサポートしない：
```yaoxiang
# 正しい：中括弧を使用
if n <= 1 { return 1 } else { return n * factorial(n - 1) }

# エラー：then/else キーワードはサポートされない
# if n <= 1 then return 1 else return n * factorial(n - 1)
```

## 提案

### HM アルゴリズムと高階多相サポート

**核心特性**：HM アルゴリズムはジェネリック型注釈を通じて高階多相（higher-rank polymorphism）をサポートする

**設計原理**：
- **高階関数**：関数を引数として渡す場合、その関数型をジェネリック制約（type constraint）する必要がある
- **型注釈形式**：`(T: Type) -> ((f: (T) -> T, x: T) -> T)` - ジェネリックパラメータが関数型を制約
- **HM のワークフロー**：ジェネリックパラメータを通じて関数型を推論し、多相関数の合成を実現する

**例示説明**：
```yaoxiang
# ✅ 高階多相をサポート：ジェネリック制約による関数型パラメータ
call_twice: (T: Type) -> ((f: (T) -> T, x: T) -> T) = {
    return f(f(x))
}
# 使用：call_twice((x) => x + 1, 5)  # T=Int と推論

compose: (A: Type, B: Type, C: Type) -> ((f: (B) -> C, g: (A) -> B, x: A) -> C) = {
    return f(g(x))
}
# 使用：compose((x) => x * 2, (x) => x + 1, 5)  # A=Int, B=Int, C=Int と推論

# ❌ サポートされない：ジェネリック制約のない高階関数
# bad_hof: (f, x) => f(f(x))  # HM が推論できない、ジェネリックパラメータが欠如
```

**HM 推論プロセス**：
1. 高階関数パラメータを認識する：`f: (T) -> T`
2. ジェネリック制約を作成する：`(T: Type)`
3. ジェネリックインスタンス化を通じて具象型を推論する
4. 多相関数の合成を実現する

### ラムダ式の構文規則

**重要規則**：コードブロック `{ ... }` の中では値返却に `return` を使用しなければならない；`return` がない場合、デフォルトで `Void` を返す。式形式 `= expr` は直接値を返す。

| 構文形式 | 構文 | 返却方式 |
|---------|------|----------|
| **コードブロック形式** | `{ statements }` | 値返却に `return` を使用しなければならない；`return` がない場合、デフォルトで `Void` |
| **式形式** | `expression` | 式値を直接返す |

**例**：
```yaoxiang
main: () -> Void = { println("Hello") }         # Void を返す（return なし）
add: (a: Int, b: Int) -> Int = { return a + b }  # Int を返す（明示的 return）
empty: () -> Void = {}                          # 空ブロックはデフォルトで Void を返す

# 早期リターン：return を使用
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

# 式形式：値を直接返す（return 不要）
add: (a: Int, b: Int) -> Int = a + b            # 正しい：式形式
main: () -> Void = println("Hello")               # 正しい：式形式
```

**核心思想**：
1. 関数定義は HM アルゴリズムを通じて型推論を行い、推論できない場合は明示的にエラーとする
2. **HM アルゴリズムの動作原理**：演算子型制約、関数呼び出し関係などのコンテキスト情報から自動的に型を推論する
3. **generics サポート**：多相関数はジェネリック構文 `(T: Type)` を使用して型パラメータを明示的に制約する（RFC-010/011）
4. **推論境界**：戻り型とローカル変数は推論可能；有パラメータ関数のパラメータ型は明示的な注釈が必要（シグネチャまたはラムダ頭のいずれか）
5. 空パラメータ・無戻り関数は `name: () -> Void = { ... }` を使用し、RFC-010 と統一する
6. 古い構文は廃止し、移行ツールを提供する

**型推論の例**：
```yaoxiang
# ジェネリック関数：明示的な型パラメータ（RFC-010 統一構文を使用）
identity: (T: Type) -> ((x: T) -> T) = x
map: (T: Type, R: Type) -> ((f: (T) -> R, list: List(T)) -> List(R)) = {
    result = List(R)()
    for item in list { result.push(f(item)) }
    return result
}

# 多相関数：明示的な generics 制約による定義（RFC-010/011）
add: (T: Add) -> ((a: T, b: T) -> T) = a + b
print_sum: (a: Int, b: Int) -> Void = { println(a + b) }  # (Int, Int) -> Void と推論

# 高階多相：ジェネリック型注釈による HM の高階多相サポート
call_twice: (T: Type) -> ((f: (T) -> T, x: T) -> T) = { return f(f(x)) }
compose: (A: Type, B: Type, C: Type) -> ((f: (B) -> C, g: (A) -> B, x: A) -> C) = { return f(g(x)) }
```

```yaoxiang
# === 関数定義：HM アルゴリズム型推論 ===

# 標準関数：HM アルゴリズムが戻り型を推論（パラメータ型は明示的必要）
add = (a: Int, b: Int) => a + b            # (a: Int, b: Int) -> Int と推論
main = { println("Hello") }                # () -> Void と推論

# 部分的な明示的パラメータ：HM アルゴリズムが残余を推論
print_sum: (a: Int, b: Int) -> Void = { println(a + b) }  # (Int, Int) -> Void と推論
greet: (name: String) -> Void = { println("Hello " + name) }  # (String) -> Void と推論

# ジェネリック関数：多相型パラメータを明示的に制約（RFC-010 統一構文を使用）
identity: (T: Type) -> ((x: T) -> T) = x
map: (T: Type, R: Type) -> ((f: (T) -> R, list: List(T)) -> List(R)) = {
    # map 関数の実装
    return List(R)()
}

# 再帰関数：HM アルゴリズムと再帰制約による推論
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 } else { return n * factorial(n - 1) }
}

# === 変数代入：HM アルゴリズム型推論 ===

# 明示的な型
x: Int = 42

# HM アルゴリズムが Int と自動推論
y = 42                               # Int と推論

# HM アルゴリズムが String と自動推論
name = "YaoXiang"                    # String と推論

# HM アルゴリズムが Float と自動推論
pi = 3.14159                         # Float と推論
```

**HM 型推論規則**：

| シナリオ | 構文 | 省略可能部分 | 例 |
|------|------|----------|------|
| **完全形式** | `name: (a: Type, b: Type) -> Ret = (a, b) => ...` | なし | シグネチャ + ラムダ頭完全 |
| **ラムダ頭省略** | `name: (a: Type, b: Type) -> Ret = ...` | ラムダ頭 | シグネチャがパラメータを宣言 |
| **シグネチャ省略** | `name = (a: Type, b: Type) => ...` | シグネチャ | ラムダ頭がパラメータ型を提供 |
| **戻り型 Ret 省略** | `name: (a: Type, b: Type) -> = ...` | 戻り型 | HM が戻り型を推論 |
| **空パラメータ完全** | `name: () -> Void = () => { ... }` | なし | 空パラメータ関数完全 |
| **空パラメータ短縮** | `name: () -> Void = { ... }` | ラムダ頭 | `() =>` を省略 |
| **空パラメータ最短** | `name = { ... }` | 全部 | 無パラメータ・無戻り最短 |
| **変数代入** | `name = value` | 型 | HM が型を推論 |
| **明示的変数** | `name: Type = value` | なし | 明示的な型注釈 |

**核心原則**：
- `->` は関数型の目印であり、省略不可（省略するとタプルとして解析される）
- 戻り型 `Ret` は省略可能、HM が関数体から推論する
- 入力パラメータが存在する場合、パラメータ型は明示的に出現しなければならない（シグネチャまたはラムダ頭のいずれか）
- その他の部分は推論可能かつ曖昧さを導入しない場合に省略可能
- 暗黙的な型変換はなし、JavaScript のような混乱を回避する

## 詳細設計

### 構文糖の展開

省略の有無に関わらず、最終的に統一中間表現（IR）に正規化される：

```rust
// 完全形式
add: (a: Int, b: Int) -> Int = (a, b) => a + b

// 展開後 IR
let add: (Int, Int) -> Int = |a: Int, b: Int| -> Int {
    return a + b
};

// ラムダ頭省略
add: (a: Int, b: Int) -> Int = a + b

// 展開後 IR（完全形式と同じ）
let add: (Int, Int) -> Int = |a: Int, b: Int| -> Int {
    return a + b
};

// シグネチャ省略（ラムダ頭がパラメータ型を注釈）
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
               | identifier '=' block                    # 最短形式：無パラメータ・無戻り

identifier ::= [a-zA-Z_][a-zA-Z0-9_]*

type_expr ::= identifier                     # 型参照
       | '()'                          # 空型
       | '(' parameters ')' '->' type_expr   # 関数型（パラメータ名はシグネチャで）
       | type_expr '->' type_expr            # 単純な関数型
       | identifier '(' type_expr (',' type_expr)* ')'  # 型適用

expression ::= '(' parameters ')' '=>' block
             | '(' ')' '=>' block
             | '(' parameters ')' '=>' expression

parameters ::= parameter (',' parameter)*
parameter ::= identifier                # 型推論
            | identifier ':' type_expr      # 部分的な明示的型

block ::= '{' statement (',' statement)* '}'
        | expression

statement ::= identifier ':' expression  # 代入文
           | expression                  # 式文（実行するが返さない）
           | 'return' expression         # return 文（指定値を返す）

# 注意：コードブロック内では値返却に return を使用しなければならない；return がない場合、デフォルトで Void を返す
# 例：{ return 1 + 1 } は Int を返す；{ println("Hello") } は Void を返す
# 注意：ジェネリックパラメータは (T: Type) 構文を使用し、関数型の一部であり、独立した BNF 規則は不要
```

### エラー処理

```yaoxiang
# === コンパイルエラー例 ===

# エラー1：コードブロックの戻り型不一致
add: (a: Int, b: Int) -> Int = { println(a + b) }
// エラー：ブロック内に return がない、デフォルトで Void を返すが、シグネチャは Int を期待
// 正しい：add: (a: Int, b: Int) -> Int = a + b
// または：add: (a: Int, b: Int) -> Int = { return a + b }

# エラー2：未宣言の型パラメータの使用
identity: (x: T) -> T = x
// エラー：T が未宣言；明示的な generics パラメータが必要（RFC-010）
// 正しい：identity: (T: Type) -> ((x: T) -> T) = x

# 正しい：HM アルゴリズムが戻り型を推論
double = (x: Int) => x + x

# 完全形式（段階的短縮）
double: (x: Int) -> Int = (x) => x + x                # 完全
double: (x: Int) -> Int = x + x                       # ラムダ頭省略
double = (x: Int) => x + x                            # 戻り型省略（HM が戻り値を推論）
# double = (x) => x + x                               # ❌ パラメータ型は両側の省略を許可しない
```

## トレードオフ

### 利点

- **構文の統一**：`name: Signature = LambdaBody` モデルが全シナリオをカバーする
- **柔軟な短縮**：任意の部分は HM で推論可能なら省略可能
- **型安全性**：HM アルゴリズムが型安全性を保証し、暗黙的型変換を回避する
- **再帰サポート**：HM アルゴリズムと再帰制約が自動的に型を推論する
- **ゼロ負担**：完全形式から最短形式までスムーズに移行できる

### 欠点

- **移行コスト**：古いコードは移行ツールによる変換が必要
- **学習コスト**：「完全形式 + 任意の短縮」モデルの理解が必要

## 代替案

| 案 | 説明 | 選択しない理由 |
|------|------|-----------|
| HM アルゴリズム型推論 | Hindley-Milner アルゴリズムで型を推論 | ✅ **採用済**、現代的な関数型言語の標準 |
| 明示的な型宣言 | 全ての型を明示的に記述 | 簡素化構文の原則に違反し、定型コードが増加する |
| 古い構文の保持 | 新旧構文を同時にサポート | 構文分裂を引き起こし、保守コストが高い |
| fn キーワード | 関数と変数を区別するために fn を導入 | 「関数（function）＝ lambda」という設計に違反する |

## 実装戦略

### 段階区分

1. **Phase 1: 構文解析と HM アルゴリズム**（v0.3）
   - 新構文 `name = lambda` + HM アルゴリズム型推論を実装する
   - 空パラメータ・無戻りのデフォルト充填を実装する

2. **Phase 2: 移行ツール**（v0.3）
   - `yaoxiang-migrate --old-to-new` ツールを開発する
   - 古い構文コードを自動変換する

3. **Phase 3: 検証とドキュメント**（v0.3）
   - 古いコードの移行完了検証
   - ドキュメント更新

### 移行ツール

```bash
# 単一ファイルを移行
yaoxiang-migrate --old-to-new src/main.yaoxiang

# プロジェクト全体を移行
yaoxiang-migrate --old-to-new --recursive src/

# 移行プレビュー（ファイルを変更しない）
yaoxiang-migrate --old-to-new --dry-run src/main.yaoxiang
```

移行規則：
```yaoxiang
# 古い構文
add(Int, Int) -> Int = (a, b) => { a + b }
main() -> Int = { println("Hello"); 0 }
main() = { println("Hello") }

# === 新構文：完全形式（シグネチャ完全 + ラムダ頭完全）===
add: (a: Int, b: Int) -> Int = (a, b) => a + b
main: () -> Void = () => { println("Hello") }

# === 短縮：ラムダ頭省略 ===
add: (a: Int, b: Int) -> Int = a + b
main: () -> Void = { println("Hello") }

# === 短縮：HM 推論 ===
add = (a: Int, b: Int) => a + b              # (a: Int, b: Int) -> Int と推論
main = { println("Hello") }                  # () -> Void と推論

# === 最短形式 ===
main = {                                      # main: () -> Void = { ... } と等価
    println("Hello")
}
```

### 依存関係

- 外部依存なし
- 独立して実装可能

### リスク

| リスク | 影響 | 緩和策 |
|------|------|---------|
| 移行漏れ | 古いコードのコンパイル失敗 | 移行ツールを提供し、全古い構文パターンをカバーする |
| パーサーエラー | 構文解析が不安定 | 十分なテストカバレッジを確保する |

## 未解決問題

> 以下の問題は設計で既に解決済みであり、付録 A に記録されている。

- ~~Q1: `main() = body` のような極簡記法を保持すべきか？~~ → 解決済：`main = { ... }` として保持する
- ~~Q2: 関数名後の `:` を保持するか？~~ → 解決済：任意で保持；但し有パラメータ関数はシグネチャまたはラムダ頭でパラメータ型を注釈する必要あり
- ~~Q3: HM アルゴリズムはパラメータ型推論をサポートするか？~~ → 解決済：戻り値/ローカルは推論可能；有パラメータ関数のパラメータ型は明示的な注釈が必要
- ~~Q4: `fn` キーワードを導入するか？~~ → 解決済：導入しない、関数（function）は lambda である
- ~~Q5: 古いコードの移行戦略は？~~ → 解決済：`yaoxiang-migrate` ツールを提供する
- ~~Q6: ジェネリック関数の使用方法は？~~ → 解決済：RFC-010 統一構文 `(T: Type)` を使用する

---

## 付録

### 付録 A：各言語の関数定義構文リファレンス

| 言語 | 構文スタイル | 特徴 |
|------|---------|------|
| Rust | `fn add(a: i32, b: i32) -> i32 { ... }` | キーワード + 型注釈 |
| Haskell | `add a b = ...` / `add :: Int -> Int -> Int` | 型シグネチャ分離 |
| OCaml | `let add a b = ...` | パラメータ型は省略可能 |
| MoonBit | `fn add(a: Int, b: Int): Int { ... }` | 簡潔な型注釈 |
| TypeScript | `const add = (a: number, b: number): number => ...` | ラムダスタイル |
| Scala | `def add(a: Int, b: Int): Int = { ... }` | def キーワード |
| **YaoXiang** | `name = (a: Int, b: Int) => a + b` | **関数（function）= lambda、HM が戻り値を推論** |

### 付録 B：設計決定記録

| 決定 | 決定内容 | 日付 | 記録者 |
|------|------|------|--------|
| 構文スタイル | 新構文 `name: (params) -> Return = body` + HM 推論 | 2026-02-03 | @沫郁酱 |
| パラメータ位置 | パラメータ名はシグネチャで宣言、RFC-010 と統一 | 2026-02-03 | @沫郁酱 |
| デフォルト充填 | 空パラメータ関数はシグネチャ省略可能、空ブロック `{}` は `Void` と推論 | 2026-02-03 | @沫郁酱 |
| 型推論 | HM アルゴリズムによる自動推論、推論できない場合は明示 | 2026-01-06 | @沫郁酱 |
| 古い構文 | 廃止、移行ツールを提供 | 2026-01-06 | @沫郁酱 |
| fn キーワード | 導入しない | 2026-01-06 | @沫郁酱 |
| 再帰宣言 | HM アルゴリズムと再帰制約による自動推論 | 2026-01-06 | @沫郁酱 |

### 付録 C：用語集

| 用語 | 定義 |
|------|------|
| HM アルゴリズム | Hindley-Milner 型推論アルゴリズム、関数（function）と変数の型を自動推論する |
| generics | 型パラメータ `(T: Type)` を使用して多相関数を制約する仕組み、例：`identity: (T: Type) -> ((x: T) -> T) = x`（RFC-010） |
| デフォルト型充填 | 空パラメータ・無戻り関数の `-> Void` を省略し、コンパイラが自動充填する仕組み |
| 構文糖 | コードをより読みやすくする構文の簡略記法 |
| 正規化 | 構文形式を統一内部表現に変換すること |
| 関数（function）即 lambda | 関数は本質的に lambda 変数であり、型は HM アルゴリズムが自動推論する |

---

## 参考文献

- [MoonBit 言語設計](https://moonbitlang.com/)
- [Rust 関数（function）構文](https://doc.rust-lang.org/book/ch03-03-how-functions-work.html)
- [Haskell 型システム（type system）](https://www.haskell.org/tutorial/patterns.html)
- [OCaml 型推論（type inference）](https://v2.ocaml.org/manual/)
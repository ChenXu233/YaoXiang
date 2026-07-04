title: "RFC-007: 関数定義構文統一方案"
status: "承認済み"
author: "沫郁酱"
created: "2025-01-05"
updated: "2026-03-21（型コンストラクタルールとコードブロック返り値セマンティクスの整合）"
```

# RFC-007: 関数定義構文統一方案

## 概要

本 RFC は YaoXiang 言語の**関数定義構文**の最終方案を確定する。統一構文 `name: (params) -> Return = body` を使用し、RFC-010 の `name: type = value` モデルと完全に一致する。

曖昧さを避けるため、関数に輸入パラメータがある場合は、パラメータ型は「シグネチャ」または「lambda 頭」の少なくとも片方に明示的に标注する必要がある。両方を省略した場合は拒否される。

コードブロック `{ ... }` 内では `return` を使用して返り値を返す必要がある。`return` がない場合はデフォルトで `Void` を返す。式形式 `= expr` は直接値を返す。

## 動機

### この機能が必要な理由

1. **構文の一貫性**：旧構文の歴史的包袱を排除し、スタイルを統一
2. **簡潔性**：HM算法が自動的に型を推論し、样板コード减少了
3. **型安全**：HM算法が型安全を保証し、推論できない場合にのみ明示的に标注
4. **言語成熟度**：HM算法は現代関数型言語の成熟した方案

### 統一構文モデル

**基本原則**：`name: Signature = LambdaBody`

- **完全形式**：シグネチャ（パラメータ名 + 型 + `->` + 返り型） + Lambda頭（パラメータ名を含む）
- **省略規則**：曖昧さを導入しない前提下 максимально 省略
  - `->` は省略不可（関数型の印，否则会被解析为元组）
  - **有輸入パラメータの場合**、パラメータ型はシグネチャまたは lambda 頭の少なくとも片方に明示的に出現する必要がある
  - Lambda 頭は省略可能 → シグネチャがパラメータ名と型をすでに宣言している場合
  - 返り型は明示的に标注可能、推論可能な場合は省略可能

```yaoxiang
# 完全形式（シグネチャ完全 + Lambda頭完全）
add: (a: Int, b: Int) -> Int = (a, b) => a + b

# 省略：Lambda 頭を省略（シグネチャがパラメータをすでに宣言）
add: (a: Int, b: Int) -> Int = a + b

# 省略：シグネチャを省略（lambda 頭がパラメータ型を标注）
add = (a: Int, b: Int) => a + b

# ❌ エラー：両側にパラメータ型が标注されていない
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
main = { println("Hello") }                            # 最小形式（推論で () -> Void）

# === ジェネリック関数（RFC-010 統一構文を使用）===
identity: (T: Type) -> ((x: T) -> T) = (x) => x         # 完全形式
identity: (T: Type) -> ((x: T) -> T) = x                # Lambda 頭を省略
identity = (x: T) => x                                  # シグネチャを省略（lambda 頭が型を标注）

# === 再帰関数 ===
factorial: (n: Int) -> Int = (n) => {
    if n <= 1 { return 1 } else { return n * factorial(n - 1) }
}
```

### 構文規則

| シーン | 構文 | 説明 |
|------|------|------|
| **完全形式** | `name: (a: Type, b) -> Ret = (a, b) => { return ... }` | シグネチャ + Lambda 頭完全 |
| **Lambda 頭を省略** | `name: (a: Type, b: Type) -> Ret = { ... }` | シグネチャがパラメータを宣言済み |
| **シグネチャを省略** | `name = (a: Type, b: Type) => { ... }` | lambda 頭がパラメータ型を标注 |
| **空パラメータ完全** | `name: () -> Void = () => { return ... }` | 空パラメータ関数完全 |
| **空パラメータ省略** | `name: () -> Void = { return ... }` | Lambda 頭を省略 |
| **空パラメータ最小** | `name = { return ... }` | 無パラメータ無返り最小 |

**注意**：コードブロック `{ ... }` 内では `return` を使用して返り値を返す必要がある。`return` がない場合はデフォルトで `Void` を返す。式形式 `= expr` は直接値を返す。

**注意**：`->` は関数型の印であり、省略不可（否则会被解析为元组）。

**重要**：`if` 式は分支を包裹するために波括弧 `{}` を使用し、`then/else` キーワードをサポートしない：
```yaoxiang
# 正しい：波括弧を使用
if n <= 1 { return 1 } else { return n * factorial(n - 1) }

# エラー：then/else キーワードはサポートしない
# if n <= 1 then return 1 else return n * factorial(n - 1)
```

## 提案

### HM算法と高階多态サポート

**コア機能**：HM算法はジェネリック型注釈を通じて高階多态（Higher-rank polymorphism）をサポート

**設計原理**：
- **高階関数**：関数がパラメータとして渡されるとき、関数型を制約するジェネリックが必要
- **型注釈形式**：`(T: Type) -> ((f: (T) -> T, x: T) -> T)` - ジェネリックパラメータが関数型を制約
- **HM ワークフロー**：ジェネリックパラメータを通じて関数型を推論し、多态関数合成を実現

**示例説明**：
```yaoxiang
# ✅ 高階多态サポート：ジェネリックが関数型パラメータを制約
call_twice: (T: Type) -> ((f: (T) -> T, x: T) -> T) = {
    return f(f(x))
}
# 使用：call_twice((x) => x + 1, 5)  # 推論 T=Int

compose: (A: Type, B: Type, C: Type) -> ((f: (B) -> C, g: (A) -> B, x: A) -> C) = {
    return f(g(x))
}
# 使用：compose((x) => x * 2, (x) => x + 1, 5)  # 推論 A=Int, B=Int, C=Int

# ❌ サポート外：ジェネリック制約の欠落した高階関数
# bad_hof: (f, x) => f(f(x))  # HMが推論不能、ジェネリックパラメータ欠落
```

**HM推論プロセス**：
1. 高階関数パラメータを識別：`f: (T) -> T`
2. ジェネリック制約を生成：`(T: Type)`
3. ジェネリックインスタンス化を通じて具象型を推論
4. 多态関数合成を実現

### Lambda 式構文規則

**重要な規則**：コードブロック `{ ... }` 内では `return` を使用して返り値を返す必要がある。`return` がない場合はデフォルトで `Void` を返す。式形式 `= expr` は直接値を返す。

| 構文形式 | 構文 | 返り方式 |
|---------|------|----------|
| **コードブロック形式** | `{ statements }` | `return` を使用して返り値を返す必要がある；`return` がない場合はデフォルトで `Void` |
| **式形式** | `expression` | 直接式の値を返す |

**示例**：
```yaoxiang
main: () -> Void = { println("Hello") }         # Void を返す（return なし）
add: (a: Int, b: Int) -> Int = { return a + b }  # Int を返す（明示的 return）
empty: () -> Void = {}                          # 空ブロックはデフォルトで Void を返す

# 早期返り：return を使用
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

# 式形式：直接値を返す（return 不要）
add: (a: Int, b: Int) -> Int = a + b            # 正しい：式形式
main: () -> Void = println("Hello")               # 正しい：式形式
```

**コア思想**：
1. 関数定義はHM算法を通じて型推論を行い，尽量推論、推論できない場合は明示的にエラーを報告
2. **HM算法動作原理**：演算子の型制約、関数呼び出し関係などのコンテキスト情報に基づいて型を自動的に推論
3. **ジェネリックサポート**：多态関数はジェネリック構文 `(T: Type)` を使用して型パラメータを明示的に制約（RFC-010/011）
4. **推論境界**：返り型と局所変数は推論可能；パラメータを持つ関数のパラメータ型は明示的に标注が必要（シグネチャまたは lambda 頭のいずれか）
5. 空パラメータ無返り関数は `name: () -> Void = { ... }` を使用し、RFC-010 と統一
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
print_sum: (a: Int, b: Int) -> Void = { println(a + b) }  # 推論で (Int, Int) -> Void

# 高階多态：ジェネリック型注釈を通じてHMが高階多态をサポート
call_twice: (T: Type) -> ((f: (T) -> T, x: T) -> T) = { return f(f(x)) }
compose: (A: Type, B: Type, C: Type) -> ((f: (B) -> C, g: (A) -> B, x: A) -> C) = { return f(g(x)) }
```

```yaoxiang
# === 関数定義：HM算法型推論 ===

# 標準関数：HM算法が返り型を推論（パラメータ型は明示が必要）
add = (a: Int, b: Int) => a + b            # 推論で (a: Int, b: Int) -> Int
main = { println("Hello") }                # 推論で () -> Void

# 部分的に明示的パラメータ：HM算法が残りを推論
print_sum: (a: Int, b: Int) -> Void = { println(a + b) }  # 推論で (Int, Int) -> Void
greet: (name: String) -> Void = { println("Hello " + name) }  # 推論で (String) -> Void

# ジェネリック関数：多态型パラメータを明示的に制約（RFC-010 統一構文を使用）
identity: (T: Type) -> ((x: T) -> T) = x
map: (T: Type, R: Type) -> ((f: (T) -> R, list: List(T)) -> List(R)) = {
    # map 関数を実装
    return List(R)()
}

# 再帰関数：HM算法と再帰制約を通じて推論
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 } else { return n * factorial(n - 1) }
}

# === 変数代入：HM算法型推論 ===

# 明示的型
x: Int = 42

# HM算法が自動的に Int に推論
y = 42                               # Int に推論

# HM算法が自動的に String に推論
name = "YaoXiang"                    # String に推論

# HM算法が自動的に Float に推論
pi = 3.14159                         # Float に推論
```

**HM型推論規則**：

| シーン | 構文 | 省略可能部分 | 示例 |
|------|------|----------|------|
| **完全形式** | `name: (a: Type, b: Type) -> Ret = (a, b) => ...` | なし | シグネチャ + Lambda 頭完全 |
| **Lambda 頭を省略** | `name: (a: Type, b: Type) -> Ret = ...` | Lambda 頭 | シグネチャがパラメータを宣言済み |
| **シグネチャを省略** | `name = (a: Type, b: Type) => ...` | シグネチャ | lambda 頭がパラメータ型を提供 |
| **返り Ret を省略** | `name: (a: Type, b: Type) -> = ...` | 返り型 | HM が返り型を推論 |
| **空パラメータ完全** | `name: () -> Void = () => { ... }` | なし | 空パラメータ関数完全 |
| **空パラメータ省略** | `name: () -> Void = { ... }` | Lambda 頭 | `() =>` を省略 |
| **空パラメータ最小** | `name = { ... }` | 全部 | 無パラメータ無返り最小 |
| **変数代入** | `name = value` | 型 | HM が型を推論 |
| **明示的変数** | `name: Type = value` | なし | 明示的型注釈 |

**基本原則**：
- `->` は関数型の印であり、省略不可（否则会被解析为元组）
- 返り型 `Ret` は省略可能で、HM が関数本体に基づいて推論
- 輸入パラメータが存在する場合、パラメータ型は明示的に出現する必要がある（シグネチャまたは lambda 頭のいずれか）
- 残余部分は推論可能かつ曖昧さを導入しない場合に省略可能
- 暗黙の型変換なし、JavaScript 式の混乱を回避

## 詳細設計

### 構文糖衣展開

省略の有無に関係なく、最終的にはすべて統一中間表現に正規化される：

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
               | identifier '=' block                    # 最小形式：無パラメータ無返り

identifier ::= [a-zA-Z_][a-zA-Z0-9_]*

type_expr ::= identifier                     # 型参照
       | '()'                          # 空型
       | '(' parameters ')' '->' type_expr   # 関数型（シグネチャ内のパラメータ名）
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
           | expression                  # 式文（実行するが返らない）
           | 'return' expression         # 返り文（指定値を返す）

# 注意：コードブロック内では return を使用して返り値を返す必要がある；return がない場合はデフォルトで Void を返す
# 例：{ return 1 + 1 } は Int を返す；{ println("Hello") } は Void を返す
# 注意：ジェネリックパラメータは (T: Type) 構文を使用し、関数型の一部として、独立した BNF 規則は不要
```

### エラー処理

```yaoxiang
# === コンパイルエラー示例 ===

# エラー1：コードブロック返り型が一致しない
add: (a: Int, b: Int) -> Int = { println(a + b) }
// エラー：ブロック内に return がなく、デフォルトで Void を返す，然而シグネチャは Int を期待
// 正しい：add: (a: Int, b: Int) -> Int = a + b
// または：add: (a: Int, b: Int) -> Int = { return a + b }

# エラー2：未宣言の型パラメータを使用
identity: (x: T) -> T = x
// エラー：T が未宣言；明示的ジェネリックパラメータが必要（RFC-010）
// 正しい：identity: (T: Type) -> ((x: T) -> T) = x

# 正しい：HM算法が返り型を推論
double = (x: Int) => x + x

# 完全形式（段階的に省略）
double: (x: Int) -> Int = (x) => x + x                # 完全
double: (x: Int) -> Int = x + x                       # Lambda 頭を省略
double = (x: Int) => x + x                            # 返り型を省略（HM が返り型を推論）
# double = (x) => x + x                               # ❌ パラメータ型は両側での省略不允许
```

## トレードオフ

### メリット

- **構文の統一**：`name: Signature = LambdaBody` モデルがすべてのシーンをカバー
- **柔軟な省略**：HM が推論可能な任意的部分は省略可能
- **型安全**：HM算法が型安全を保証し、暗黙の型変換を回避
- **再帰サポート**：HM算法と再帰制約が自動的に型を推論
- **ゼロ負担**：完全から最小まで滑らかに移行

### デメリット

- **移行コスト**：旧コードは移行ツールで変換が必要
- **学習コスト**：「完全形式 + 任意の省略」モデルを理解する必要がある

## 代替方案

| 方案 | 説明 | なぜ選択しないか |
|------|------|-----------|
| HM算法型推論 | Hindley-Milner算法を使用して型を推論 | ✅ **すでに採用**、現代関数型言語の標準 |
| 明示的型宣言 | すべての型を明示的に書く必要がある | 構文簡略化の原則に違反、样板コードが増加 |
| 旧構文を保持 | 新旧構文を同時にサポート | 構文分裂、维护コストが高い |
| fn キーワード | 関数を区別するために fn を導入 | 「関数は lambda である」という設計に違反 |

## 実装策略

### 段階的区分

1. **Phase 1: 構文解析とHM算法**（v0.3）
   - 新構文 `name = lambda` + HM算法型推論を実装
   - 空パラメータ無返りのデフォルト填充を実装

2. **Phase 2: 移行ツール**（v0.3）
   - `yaoxiang-migrate --old-to-new` ツールを開発
   - 旧構文コードを自動的に変換

3. **Phase 3: 検証と文書**（v0.3）
   - 旧コード移行完了検証
   - 文書の更新

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
add = (a: Int, b: Int) => a + b              # 推論で (a: Int, b: Int) -> Int
main = { println("Hello") }                  # 推論で () -> Void

# === 最小形式 ===
main = {                                      # main: () -> Void = { ... } と等価
    println("Hello")
}
```

### 依存関係

- 外部依存なし
- 独立して実装可能

### リスク

| リスク | 影響 | 緩和措施 |
|------|------|---------|
| 移行漏れ | 旧コードがコンパイル失敗 | 移行ツールを提供し、すべての旧構文パターンをカバー |
| パーサーエラー | 構文解析が不安定 | 十分なテストカバレッジ |

## 開放問題

> 以下的问题是已经在设计中解决的，记录在付録A。

- ~~Q1: `main() = body` 这种極限省略写法是否应该保留？~~ → 解決済み：`main = { ... }` として保留
- ~~Q2: 関数名の後の `:` は保持するか？~~ → 解決済み：オプションとして保持；但し有パラメータ関数は引き続きシグネチャまたは lambda 頭にパラメータ型を标注する必要がある
- ~~Q3: HM算法はパラメータ型推論をサポートするか？~~ → 解決済み：返り値/局所变量は推論可能；有パラメータ関数のパラメータ型は明示的に标注が必要
- ~~Q4: `fn` キーワードを導入するか？~~ → 解決済み：導入しない、関数は lambda である
- ~~Q5: 旧コードの移行戦略は何か？~~ → 解決済み：`yaoxiang-migrate` ツールを提供
- ~~Q6: ジェネリック関数は如何使用か？~~ → 解決済み：RFC-010 統一構文 `(T: Type)` を使用

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
| **YaoXiang** | `name = (a: Int, b: Int) => a + b` | **関数 = lambda、HM が返り値を推論** |

### 付録B：設計意思決定記録

| 意思決定 | 決定 | 日付 | 記録人 |
|------|------|------|--------|
| 構文スタイル | 新構文 `name: (params) -> Return = body` + HM推論 | 2026-02-03 | @沫郁酱 |
| パラメータ位置 | シグネチャにパラメータ名を宣言、RFC-010 と統一 | 2026-02-03 | @沫郁酱 |
| デフォルト填充 | 空パラメータ関数はシグネチャを省略可能、空ブロック `{}` は `Void` に推論 | 2026-02-03 | @沫郁酱 |
| 型推論 | HM算法が自動的に型を推論、推論できない場合は明示的に | 2026-01-06 | @沫郁酱 |
| 旧構文 | 退役、移行ツールを提供 | 2026-01-06 | @沫郁酱 |
| fn キーワード | 導入しない | 2026-01-06 | @沫郁酱 |
| 再帰宣言 | HM算法と再帰制約が自動的に推論 | 2026-01-06 | @沫郁酱 |

### 付録C：用語集

| 用語 | 定義 |
|------|------|
| HM算法 | Hindley-Milner型推論算法、関数と変数の型を自動的に推論 |
| ジェネリック | 型パラメータ `(T: Type)` を使用して多态関数を制約、例：`identity: (T: Type) -> ((x: T) -> T) = x`（RFC-010） |
| デフォルト型填充 | 空パラメータ無返り関数で `-> Void` を省略可能、コンパイラが自動的に填充 |
| 構文糖衣 | コードを読みやすくする構文簡略化写法 |
| 正規化 | 構文形式を統一内部表現に変換 |
| 関数即ち lambda | 関数は本質的に lambda 変数であり、型はHM算法を通じて自動的に推論 |

---

## 参考文献

- [MoonBit 言語設計](https://moonbitlang.com/)
- [Rust 関数構文](https://doc.rust-lang.org/book/ch03-03-how-functions-work.html)
- [Haskell 型システム](https://www.haskell.org/tutorial/patterns.html)
- [OCaml 型推論](https://v2.ocaml.org/manual/)
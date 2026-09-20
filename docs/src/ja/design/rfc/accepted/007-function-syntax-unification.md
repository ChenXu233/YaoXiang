---
title: 'RFC-007: 関数定義構文統一案'
issue: '#131'
status: '承認済み'
author: '沫郁酱'
created: '2025-01-05'
updated: '2026-09-15'
---

# RFC-007: 関数定義構文統一案

> **関連**：「空パラメータ最簡」`name = { ... }`
> と RFC-010 の「値ブロック」構文位置が重なる部分について、 **内容が型を決める**と裁定する——`=>`
> は常に関数、`Fn` 注釈は関数、`Fn` 注釈なしはブロック値、注釈なしの場合は内容により推論する。詳細は
> [RFC-010a](./010a-tail-expression-and-return.md) 付録Dを参照。
>
> **関連補足**：関数本体（`{ ... }` コードブロック）内の文終端と改行ルール（`;`
> 明示的区切り、改行終端、続き行例外）は
> [RFC-038（ドラフト）](../draft/038-statement-termination.md) で定義され、本RFCでは扱わない。

## 概要

本RFCは YaoXiang 言語の**関数定義構文**の最終案を確定する。統一構文
`name: (params) -> Return = body` を使用し、RFC-010 の `name: type = value` モデルと完全に一致する。

曖昧さを避けるため：関数が入力パラメータを持つ場合、パラメータ型は「シグネチャ」または「lambda ヘッダ」の少なくとも一方で明示的に注釈付けする必要がある；両方を省略することは拒否される。

コードブロック `{ ... }` の値は**末尾式**で与えられる；`return` は `Never`
型の非局所的脱出である（[RFC-010a](./010a-tail-expression-and-return.md) 参照）。式形式 `= expr`
は値を直接与える。

## 動機

### なぜこの機能が必要か？

1. **構文の一貫性**：旧構文の歴史的負担を排除し、スタイルを統一する
2. **簡潔性**：HMアルゴリズムが自動推論し、ボイラープレートコードを削減
3. **型安全性**：HMアルゴリズムが型安全を保証し、推論できない場合のみ明示的に注釈
4. **言語の成熟度**：HMアルゴリズムは現代的な関数型言語の成熟した手法

### 統一構文モデル

**核となる原則**：`name: Signature = LambdaBody`

- **完全形式**：シグネチャ（パラメータ名 + 型 + `->` + 戻り型を含む）+
  Lambdaヘッダ（パラメータ名を含む）
- **短縮ルール**：曖昧さを導入しない範囲でできる限り省略
  - `->` は省略不可（関数型の目印、省略するとタプルとして解釈される）
  - **入力パラメータがある場合**、パラメータ型はシグネチャまたは lambda ヘッダの少なくとも一方で明示的に出現する必要がある
  - Lambda ヘッダは省略可能 → シグネチャが既にパラメータ名と型を宣言している場合
  - 戻り型は明示的に注釈可能、推論可能な場合は省略可能

```yaoxiang
# 完全形式（シグネチャ完全 + Lambdaヘッダ完全）
add: (a: Int, b: Int) -> Int = (a, b) => a + b

# 短縮：Lambda ヘッダを省略（シグネチャがパラメータを宣言）
add: (a: Int, b: Int) -> Int = a + b

# 短縮：シグネチャを省略（lambda ヘッダがパラメータ型を注釈）
add = (a: Int, b: Int) => a + b

# ❌ エラー：両方でパラメータ型を注釈していない
# add = (a, b) => a + b
```

### 設計目標

```yaoxiang
# === 完全形式 ===
add: (a: Int, b: Int) -> Int = (a, b) => { a + b }

# === 短縮形式 ===
add: (a: Int, b: Int) -> Int = a + b                 # Lambda ヘッダを省略
add = (a: Int, b: Int) => a + b                      # シグネチャを省略

# === 空パラメータ関数 ===
main: () -> Void = () => { println("Hello") }          # 完全形式
main: () -> Void = { println("Hello") }                # Lambda ヘッダを省略
main: () -> Void = { println("Hello") }                            # 最簡形式（() -> Void と推論）

# === ジェネリック関数（RFC-010 統一構文を使用）===
identity: (T: Type) -> ((x: T) -> T) = (x) => x         # 完全形式
identity: (T: Type) -> ((x: T) -> T) = x                # Lambda ヘッダを省略
identity = (x: T) => x                                  # シグネチャを省略（lambda ヘッダが型を注釈）

# === 再帰関数 ===
factorial: (n: Int) -> Int = (n) => {
    if n <= 1 { return 1 } else { return n * factorial(n - 1) }
}
```

### 構文ルール

| シナリオ              | 構文                                                   | 説明                              |
| --------------------- | ------------------------------------------------------ | --------------------------------- |
| **完全形式**          | `name: (a: Type, b) -> Ret = (a, b) => { return ... }` | シグネチャ + Lambda ヘッダ完全    |
| **Lambda ヘッダ省略** | `name: (a: Type, b: Type) -> Ret = { ... }`            | シグネチャがパラメータを宣言      |
| **シグネチャ省略**    | `name = (a: Type, b: Type) => { ... }`                 | lambda ヘッダがパラメータ型を注釈 |
| **空パラメータ完全**  | `name: () -> Void = () => { return ... }`              | 空パラメータ関数完全              |
| **空パラメータ短縮**  | `name: () -> Void = { return ... }`                    | Lambda ヘッダを省略               |
| **空パラメータ最簡**  | `name = { return ... }`                                | 無パラメータ無戻り最簡            |

**注意**：コードブロック `{ ... }` の値は**末尾式**で与えられる（唯一の出口）；`return` は `Never`
型の非局所的脱出であり、最も近い関数境界から脱出する。式形式 `= expr` は値を直接与える。詳細は
[RFC-010a](./010a-tail-expression-and-return.md) を参照。

**注意**：`->` は関数型の目印であり、省略不可（省略するとタプルとして解釈される）。

**重要**：`if` 式は中括弧 `{}` で分岐を囲み、`then/else` キーワードはサポートされない：

```yaoxiang
# 正しい：中括弧を使用
if n <= 1 { return 1 } else { return n * factorial(n - 1) }

# エラー：then/else キーワードはサポートされない
# if n <= 1 then return 1 else return n * factorial(n - 1)
```

## 提案

### HMアルゴリズムと高階多相サポート

**核となる機能**：HMアルゴリズムはジェネリック型注釈を通じて高階多相（Higher-rank
polymorphism）をサポートする

**設計原理**：

- **高階関数**：関数を引数として渡す際、ジェネリック制約で関数型を制約する必要がある
- **型注釈形式**：`(T: Type) -> ((f: (T) -> T, x: T) -> T)` - ジェネリックパラメータが関数型を制約
- **HMワークフロー**：ジェネリックパラメータを通じて関数型を推論し、多相関数の合成を実現

**説明例**：

```yaoxiang
# ✅ 高階多相サポート：ジェネリック制約で関数型パラメータを制約
call_twice: (T: Type) -> ((f: (T) -> T, x: T) -> T) = {
    return f(f(x))
}
# 使用：call_twice((x) => x + 1, 5)  # T=Int と推論

compose: (A: Type, B: Type, C: Type) -> ((f: (B) -> C, g: (A) -> B, x: A) -> C) = {
    return f(g(x))
}
# 使用：compose((x) => x * 2, (x) => x + 1, 5)  # A=Int, B=Int, C=Int と推論

# ❌ サポート外：ジェネリック制約のない高階関数
# bad_hof: (f, x) => f(f(x))  # HMで推論不可、ジェネリックパラメータ不足
```

**HM推論プロセス**：

1. 高階関数パラメータを識別：`f: (T) -> T`
2. ジェネリック制約を作成：`(T: Type)`
3. ジェネリックインスタンス化を通じて具体型を推論
4. 多相関数合成を実現

### Lambda 式構文ルール

**重要なルール**：コードブロック `{ ... }` の値は**末尾式**で与えられる（唯一の出口）；`return` は
`Never` 型の非局所的脱出であり、最も近い関数境界から脱出する。式形式 `= expr`
は値を直接与える。詳細は [RFC-010a](./010a-tail-expression-and-return.md) を参照。

| 構文形式               | 構文             | 値の出口                            |
| ---------------------- | ---------------- | ----------------------------------- |
| **コードブロック形式** | `{ statements }` | 末尾式（空ブロック `{}` は `Void`） |
| **式形式**             | `expression`     | 式の値                              |
| **`return`**           | `return e`       | 関数の非局所的脱出、型 `Never`      |

**例**：

```yaoxiang
main: () -> Void = { println("Hello") }         # 末尾式は Void
add: (a: Int, b: Int) -> Int = { a + b }        # 末尾式が値を与える
empty: () -> Void = {}                          # 空ブロック → Void

# 早期リターン：return を使用
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

# 式形式：値を直接与える
add: (a: Int, b: Int) -> Int = a + b            # 正しい：式形式
main: () -> Void = println("Hello")               # 正しい：式形式
```

**核となる考え方**：

1. 関数定義はHMアルゴリズムで型推論を行い、できる限り推論し、推論できない場合は明示的にエラー
2. **HMアルゴリズムの動作原理**：演算子型制約、関数呼び出し関係などのコンテキスト情報から自動推論
3. **ジェネリックサポート**：多相関数はジェネリック構文 `(T: Type)`
   で型パラメータを明確に制約（RFC-010/011）
4. **推論境界**：戻り型とローカル変数は推論可能；引数あり関数のパラメータ型は明示的に注釈が必要（シグネチャまたは lambda ヘッダのいずれか）
5. 空パラメータ無戻り関数は `name: () -> Void = { ... }` を使用し、RFC-010 と統一
6. 旧構文は廃止、移行ツールを提供

**型推論の例**：

```yaoxiang
# ジェネリック関数：明示的な型パラメータ（RFC-010統一構文を使用）
identity: (T: Type) -> ((x: T) -> T) = x
map: (T: Type, R: Type) -> ((f: (T) -> R, list: List(T)) -> List(R)) = {
    result = List(R)()
    for item in list { result.push(f(item)) }
    return result
}

# 多相関数：明示的なジェネリック制約で定義（RFC-010/011）
add: (T: Add) -> ((a: T, b: T) -> T) = a + b
print_sum: (a: Int, b: Int) -> Void = { println(a + b) }  # (Int, Int) -> Void と推論

# 高階多相：ジェネリック型注釈によるHM高階多相サポート
call_twice: (T: Type) -> ((f: (T) -> T, x: T) -> T) = { return f(f(x)) }
compose: (A: Type, B: Type, C: Type) -> ((f: (B) -> C, g: (A) -> B, x: A) -> C) = { return f(g(x)) }
```

```yaoxiang
# === 関数定義：HMアルゴリズムによる型推論 ===

# 標準関数：HMアルゴリズムが戻り型を推論（パラメータ型は明示必須）
add = (a: Int, b: Int) => a + b            # (a: Int, b: Int) -> Int と推論
main: () -> Void = { println("Hello") }                # () -> Void と推論

# 部分的に明示的なパラメータ：HMアルゴリズムが残りを推論
print_sum: (a: Int, b: Int) -> Void = { println(a + b) }  # (Int, Int) -> Void と推論
greet: (name: String) -> Void = { println("Hello " + name) }  # (String) -> Void と推論

# ジェネリック関数：多相型パラメータを明確に制約（RFC-010統一構文を使用）
identity: (T: Type) -> ((x: T) -> T) = x
map: (T: Type, R: Type) -> ((f: (T) -> R, list: List(T)) -> List(R)) = {
    # map 関数の実装
    return List(R)()
}

# 再帰関数：HMアルゴリズムと再帰制約による推論
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 } else { return n * factorial(n - 1) }
}

# === 変数代入：HMアルゴリズムによる型推論 ===

# 明示的な型
x: Int = 42

# HMアルゴリズムが自動推論し Int
y = 42                               # Int と推論

# HMアルゴリズムが自動推論し String
name = "YaoXiang"                    # String と推論

# HMアルゴリズムが自動推論し Float
pi = 3.14159                         # Float と推論
```

**HM型推論ルール**：

| シナリオ              | 構文                                              | 省略可能部分  | 例                                |
| --------------------- | ------------------------------------------------- | ------------- | --------------------------------- |
| **完全形式**          | `name: (a: Type, b: Type) -> Ret = (a, b) => ...` | なし          | シグネチャ + Lambda ヘッダ完全    |
| **Lambda ヘッダ省略** | `name: (a: Type, b: Type) -> Ret = ...`           | Lambda ヘッダ | シグネチャがパラメータを宣言      |
| **シグネチャ省略**    | `name = (a: Type, b: Type) => ...`                | シグネチャ    | lambda ヘッダがパラメータ型を提供 |
| **戻り型 Ret 省略**   | `name: (a: Type, b: Type) -> = ...`               | 戻り型        | HM が戻り型を推論                 |
| **空パラメータ完全**  | `name: () -> Void = () => { ... }`                | なし          | 空パラメータ関数完全              |
| **空パラメータ短縮**  | `name: () -> Void = { ... }`                      | Lambda ヘッダ | `() =>` を省略                    |
| **空パラメータ最簡**  | `name = { ... }`                                  | すべて        | 無パラメータ無戻り最簡            |
| **変数代入**          | `name = value`                                    | 型            | HM が型を推論                     |
| **明示的変数**        | `name: Type = value`                              | なし          | 明示的な型注釈                    |

**核となる原則**：

- `->` は関数型の目印であり、省略不可（省略するとタプルとして解釈される）
- 戻り型 `Ret` は省略可能、HM が関数本体から推論
- 入力パラメータが存在する場合、パラメータ型は明示的に出現する必要がある（シグネチャまたは lambda ヘッダのいずれか）
- その他の部分は推論可能かつ曖昧さを導入しない場合に省略可能
- 暗黙的な型変換はなく、JavaScript のような混乱を避ける

## 詳細設計

### 構文糖の展開

省略の有無に関わらず、最終的に統一中間表現に正規化される：

```rust
// 完全形式
add: (a: Int, b: Int) -> Int = (a, b) => a + b

// 展開後 IR
let add: (Int, Int) -> Int = |a: Int, b: Int| -> Int {
    return a + b
};

// Lambda ヘッダ省略
add: (a: Int, b: Int) -> Int = a + b

// 展開後 IR（完全形式と同じ）
let add: (Int, Int) -> Int = |a: Int, b: Int| -> Int {
    return a + b
};

// シグネチャ省略（lambda ヘッダがパラメータ型を注釈）
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
               | identifier '=' block                    # 最簡形式：無パラメータ無戻り

identifier ::= [a-zA-Z_][a-zA-Z0-9_]*

type_expr ::= identifier                     # 型参照
       | '()'                          # 空型
       | '(' parameters ')' '->' type_expr   # 関数型（パラメータ名はシグネチャ内）
       | type_expr '->' type_expr            # 単純な関数型
       | identifier '(' type_expr (',' type_expr)* ')'  # 型適用

expression ::= '(' parameters ')' '=>' block
             | '(' ')' '=>' block
             | '(' parameters ')' '=>' expression

parameters ::= parameter (',' parameter)*
parameter ::= identifier                # 型推論
            | identifier ':' type_expr      # 部分的に明示的な型

block ::= '{' statement (',' statement)* '}'
        | expression

statement ::= identifier ':' expression  # 代入文
           | expression                  # 式文（実行するが値は返さない）
           | 'return' expression         # return文（指定値を返す）

# 注意：コードブロック内では return を使用して値を返す必要がある；return なしの場合デフォルトで Void を返す
# 例：{ return 1 + 1 } は Int を返す；{ println("Hello") } は Void を返す
# 注意：ジェネリックパラメータは (T: Type) 構文を使用し、関数型の一部として、独立した BNF ルールは不要
```

### エラー処理

```yaoxiang
# === コンパイルエラー例 ===

# エラー1：コードブロックの戻り型不一致
add: (a: Int, b: Int) -> Int = { println(a + b) }
// エラー：ブロック内に return がない、デフォルトで Void を返すが、シグネチャは Int を期待
// 正しい：add: (a: Int, b: Int) -> Int = a + b
// または：add: (a: Int, b: Int) -> Int = { return a + b }

# エラー2：未宣言の型パラメータ使用
identity: (x: T) -> T = x
// エラー：T が未宣言；明示的なジェネリックパラメータが必要（RFC-010）
// 正しい：identity: (T: Type) -> ((x: T) -> T) = x

# 正しい：HMアルゴリズムが戻り型を推論
double = (x: Int) => x + x

# 完全形式（段階的短縮）
double: (x: Int) -> Int = (x) => x + x                # 完全
double: (x: Int) -> Int = x + x                       # Lambda ヘッダ省略
double = (x: Int) => x + x                            # 戻り型省略（HM が戻り型を推論）
# double = (x) => x + x                               # ❌ パラメータ型の両側省略は不可
```

## トレードオフ

### 利点

- **構文の統一**：`name: Signature = LambdaBody` モデルが全シナリオをカバー
- **柔軟な短縮**：任意の部分は HM で推論可能なら省略可能
- **型安全性**：HMアルゴリズムが型安全を保証し、暗黙的な型変換を回避
- **再帰サポート**：HMアルゴリズムと再帰制約による自動型推論
- **ゼロ負荷**：完全形式から最簡形式へのスムーズな移行

### 欠点

- **移行コスト**：旧コードは移行ツールによる変換が必要
- **学習コスト**：「完全形式 + 任意の短縮」モデルの理解が必要

## 代替案

| 案                   | 説明                                 | 採用しない理由                             |
| -------------------- | ------------------------------------ | ------------------------------------------ |
| HMアルゴリズム型推論 | Hindley-Milner アルゴリズムで型推論  | ✅ **採用済み**、現代的関数型言語の標準    |
| 明示的な型宣言       | すべての型を明示的に記述             | 簡潔な構文原則に違反、ボイラープレート増加 |
| 旧構文の保持         | 新旧構文の両方をサポート             | 構文分裂、保守コスト高                     |
| fn キーワード        | 関数と変数を区別するために fn を導入 | 「関数は lambda」という設計に違反          |

## 実装戦略

### 段階分け

1. **Phase 1: 構文解析と HM アルゴリズム**（v0.3）
   - 新構文 `name = lambda` + HM アルゴリズム型推論を実装
   - 空パラメータ無戻りのデフォルト充填を実装

2. **Phase 2: 移行ツール**（v0.3）
   - `yaoxiang-migrate --old-to-new` ツールを開発
   - 旧構文コードを自動変換

3. **Phase 3: 検証とドキュメント**（v0.3）
   - 旧コード移行完了の検証
   - ドキュメント更新

### 移行ツール

```bash
# 単一ファイルの移行
yaoxiang-migrate --old-to-new src/main.yaoxiang

# プロジェクト全体の移行
yaoxiang-migrate --old-to-new --recursive src/

# 移行のプレビュー（ファイルを変更しない）
yaoxiang-migrate --old-to-new --dry-run src/main.yaoxiang
```

移行ルール：

```yaoxiang
# 旧構文
add(Int, Int) -> Int = (a, b) => { a + b }
main() -> Int = { println("Hello"); 0 }
main() = { println("Hello") }

# === 新構文：完全形式（シグネチャ完全 + Lambda ヘッダ完全）===
add: (a: Int, b: Int) -> Int = (a, b) => a + b
main: () -> Void = () => { println("Hello") }

# === 短縮：Lambda ヘッダ省略 ===
add: (a: Int, b: Int) -> Int = a + b
main: () -> Void = { println("Hello") }

# === 短縮：HM 推論 ===
add = (a: Int, b: Int) => a + b              # (a: Int, b: Int) -> Int と推論
main: () -> Void = { println("Hello") }                  # () -> Void と推論

# === 最簡形式 ===
main: () -> Void = {                                      # main: () -> Void = { ... } と等価
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
| パーサエラー | 構文解析の不安定         | 十分なテストカバレッジ                           |

## 未解決問題

> 以下の問題は設計で解決済み、付録Aに記録。

- ~~Q1: `main() = body` のような極簡記述を保持すべきか？~~ → 解決済み：`main: () -> Void = { ... }`
  として保持
- ~~Q2: 関数名後の `:` を保持するか？~~
  → 解決済み：オプションで保持；ただし引数あり関数は依然としてシグネチャまたは lambda ヘッダでパラメータ型を注釈する必要あり
- ~~Q3: HMアルゴリズムはパラメータ型推論をサポートするか？~~
  → 解決済み：戻り値/ローカルは推論可能；引数あり関数のパラメータ型は明示的に注釈が必要
- ~~Q4: `fn` キーワードを導入するか？~~ → 解決済み：導入しない、関数は lambda
- ~~Q5: 旧コードの移行戦略は？~~ → 解決済み：`yaoxiang-migrate` ツールを提供
- ~~Q6: ジェネリック関数の使用方法？~~ → 解決済み：RFC-010 統一構文 `(T: Type)` を使用

---

## 付録

### 付録A：各言語の関数定義構文参考

| 言語         | 構文スタイル                                        | 特徴                                 |
| ------------ | --------------------------------------------------- | ------------------------------------ |
| Rust         | `fn add(a: i32, b: i32) -> i32 { ... }`             | キーワード + 型注釈                  |
| Haskell      | `add a b = ...` / `add :: Int -> Int -> Int`        | 型シグネチャ分離                     |
| OCaml        | `let add a b = ...`                                 | パラメータ型省略可能                 |
| MoonBit      | `fn add(a: Int, b: Int): Int { ... }`               | 簡潔な型注釈                         |
| TypeScript   | `const add = (a: number, b: number): number => ...` | Lambda スタイル                      |
| Scala        | `def add(a: Int, b: Int): Int = { ... }`            | def キーワード                       |
| **YaoXiang** | `name = (a: Int, b: Int) => a + b`                  | **関数 = lambda、HM が戻り値を推論** |

### 付録B：設計決定記録

| 決定           | 決定内容                                                             | 日付       | 記録者  |
| -------------- | -------------------------------------------------------------------- | ---------- | ------- |
| 構文スタイル   | 新構文 `name: (params) -> Return = body` + HM 推論                   | 2026-02-03 | @沫郁酱 |
| パラメータ位置 | パラメータ名はシグネチャで宣言、RFC-010 と統一                       | 2026-02-03 | @沫郁酱 |
| デフォルト充填 | 空パラメータ関数はシグネチャ省略可、空ブロック `{}` は `Void` と推論 | 2026-02-03 | @沫郁酱 |
| 型推論         | HMアルゴリズムによる自動推論、推論不可時は明示                       | 2026-01-06 | @沫郁酱 |
| 旧構文         | 廃止、移行ツールを提供                                               | 2026-01-06 | @沫郁酱 |
| fn キーワード  | 導入しない                                                           | 2026-01-06 | @沫郁酱 |
| 再帰宣言       | HMアルゴリズムと再帰制約による自動推論                               | 2026-01-06 | @沫郁酱 |

### 付録C：用語集

| 用語             | 定義                                                                                                         |
| ---------------- | ------------------------------------------------------------------------------------------------------------ |
| HMアルゴリズム   | Hindley-Milner 型推論アルゴリズム、関数と変数の型を自動推論                                                  |
| ジェネリック     | 型パラメータ `(T: Type)` を使用して多相関数を制約、例：`identity: (T: Type) -> ((x: T) -> T) = x`（RFC-010） |
| デフォルト型充填 | 空パラメータ無戻り関数は `-> Void` を省略可能、コンパイラが自動充填                                          |
| 構文糖           | コードをより読みやすくする構文の簡略記述                                                                     |
| 正規化           | 構文形式を統一内部表現に変換                                                                                 |
| 関数即 lambda    | 関数は本質的に lambda 変数であり、型は HM アルゴリズムで自動推論                                             |

---

## 参考文献

- [MoonBit 言語設計](https://moonbitlang.com/)
- [Rust 関数構文](https://doc.rust-lang.org/book/ch03-03-how-functions-work.html)
- [Haskell 型システム](https://www.haskell.org/tutorial/patterns.html)
- [OCaml 型推論](https://v2.ocaml.org/manual/)

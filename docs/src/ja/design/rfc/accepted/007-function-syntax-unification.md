---
title: 'RFC-007: 関数定義構文統一案'
issue: '#131'
status: '承認済み'
author: '沫郁酱'
created: '2025-01-05'
updated: '2026-09-15（returnのセマンティクスをRFC-010aの正誤表に従って修正）'
---

# RFC-007: 関数定義構文統一案

> **正誤表（2026-09-15、RFC-010a）**：本文では「コードブロック内では必ず `return`
> を使って値を返す必要があり、`return` がない場合はデフォルトで `Void`
> を返す」としていたが、この記述は廃止された——**ブロックの値 = 末尾式**であり、`return` は `Never`
> 型の非局所退出である。詳細は [RFC-010a](./010a-tail-expression-and-return.md)
> を参照。本RFCの関数形式定義（完全形式 /
> Lambda 頭省略 / 署名省略 / 空引数という4つの書き方）は**変更なし**であり、「早期return」のセマンティクスも本RFCと一致する。
>
> **補足（裁定 C）**：本文の「空引数最簡」`name = { ... }`
> は RFC-010 の「値を持つブロック」構文と位置が重なるため、**注釈優先・デフォルトは関数**と裁定された——注釈がない場合は本文の「空引数最簡」が有効（関数扱い）となり、`Fn`
> 以外の注釈を書いた場合のみブロック値（`x: Int = { ... }`）となる。RFC-010a 付録Dを参照。
>
> **関連補足**：関数本体（`{ ... }` コードブロック）内の文の終端と改行ルール（`;`
> による明示的区切り、改行による終端、続き行の例外）は
> [RFC-038（草案）](../draft/038-statement-termination.md) で定義されており、本RFCでは扱わない。

## 概要

本RFCはYaoXiang言語の**関数定義構文**の最終案を確定するものである。統一構文
`name: (params) -> Return = body` を使用し、RFC-010 の `name: type = value` モデルと完全に一致する。

曖昧さを避けるため：関数が入力パラメータを持つ場合、パラメータの型は「署名」または「lambda 頭」の少なくとも一方で明示的に注釈付けする必要がある。両方を省略した場合は拒否される。

コードブロック `{ ... }` の値は**末尾式**によって決まる；`return` は `Never`
型の非局所退出である（[RFC-010a](./010a-tail-expression-and-return.md) を参照）。式形式 `= expr`
は値を直接与える。

## 動機

### なぜこの機能が必要なのか？

1. **構文の一貫性**：旧構文の歴史的負債を排除し、スタイルを統一する
2. **簡潔性**：HMアルゴリズムが型を自動推論し、ボイラープレートコードを削減する
3. **型安全性**：HMアルゴリズムは型安全性を保証し、推論できない場合のみ明示的に注釈する
4. **言語の成熟度**：HMアルゴリズムは現代的な関数型言語の成熟した手法である

### 統一構文モデル

**核心原則**：`name: Signature = LambdaBody`

- **完全形式**：署名（パラメータ名 + 型 + `->` + 戻り型を含む） + Lambda 頭（パラメータ名を含む）
- **省略ルール**：曖昧さを導入しない範囲で極力省略する
  - `->` は省略不可（関数型の目印であり、否则タプルとして解釈される）
  - **入力パラメータがある場合**、パラメータの型は署名または lambda 頭の少なくとも一方で明示的に出現する必要がある
  - Lambda 頭は省略可能 → 署名で既にパラメータ名と型が宣言されている場合
  - 戻り型は明示的に注釈可能、推論可能な場合は省略可能

```yaoxiang
# 完全形式（署名完全 + Lambda 頭完全）
add: (a: Int, b: Int) -> Int = (a, b) => a + b

# 省略：Lambda 頭を省略（署名がパラメータを宣言済み）
add: (a: Int, b: Int) -> Int = a + b

# 省略：署名を省略（lambda 頭でパラメータ型を注釈）
add = (a: Int, b: Int) => a + b

# ❌ エラー：両方でパラメータ型が注釈されていない
# add = (a, b) => a + b
```

### 設計目標

```yaoxiang
# === 完全形式 ===
add: (a: Int, b: Int) -> Int = (a, b) => { a + b }

# === 省略形式 ===
add: (a: Int, b: Int) -> Int = a + b                 # Lambda 頭を省略
add = (a: Int, b: Int) => a + b                      # 署名を省略

# === 空引数関数 ===
main: () -> Void = () => { println("Hello") }          # 完全形式
main: () -> Void = { println("Hello") }                # Lambda 頭を省略
main: () -> Void = { println("Hello") }                            # 最簡形式（() -> Void と推論される）

# === ジェネリック関数（RFC-010 の統一構文を使用）===
identity: (T: Type) -> ((x: T) -> T) = (x) => x         # 完全形式
identity: (T: Type) -> ((x: T) -> T) = x                # Lambda 頭を省略
identity = (x: T) => x                                  # 署名を省略（lambda 頭で型を注釈）

# === 再帰関数 ===
factorial: (n: Int) -> Int = (n) => {
    if n <= 1 { return 1 } else { return n * factorial(n - 1) }
}
```

### 構文ルール

| シナリオ          | 構文                                                   | 説明                          |
| ----------------- | ------------------------------------------------------ | ----------------------------- |
| **完全形式**      | `name: (a: Type, b) -> Ret = (a, b) => { return ... }` | 署名 + Lambda 頭完全          |
| **Lambda 頭省略** | `name: (a: Type, b: Type) -> Ret = { ... }`            | 署名がパラメータを宣言済み    |
| **署名省略**      | `name = (a: Type, b: Type) => { ... }`                 | lambda 頭でパラメータ型を注釈 |
| **空引数完全**    | `name: () -> Void = () => { return ... }`              | 空引数関数の完全形            |
| **空引数省略**    | `name: () -> Void = { return ... }`                    | Lambda 頭を省略               |
| **空引数最簡**    | `name = { return ... }`                                | 引数なし・戻り値なしの最簡形  |

**注意**：コードブロック `{ ... }` の値は**末尾式**によって決まる（唯一の出口）；`return` は `Never`
型の非局所退出であり、最も近い関数の境界を抜ける。式形式 `= expr` は値を直接与える。詳細は
[RFC-010a](./010a-tail-expression-and-return.md) を参照。

**注意**：`->` は関数型の目印であり、省略不可（否则タプルとして解釈される）。

**重要**：`if` 式は中括弧 `{}` で分岐を囲み、`then/else` キーワードはサポートされない：

```yaoxiang
# 正しい：中括弧を使用
if n <= 1 { return 1 } else { return n * factorial(n - 1) }

# エラー：then/else キーワードはサポートされない
# if n <= 1 then return 1 else return n * factorial(n - 1)
```

## 提案

### HMアルゴリズムと高階多相サポート

**核心的特徴**：HMアルゴリズムはジェネリック型注釈を通じて高階多相（Higher-rank
polymorphism）をサポートする。

**設計原理**：

- **高階関数**：関数を引数として渡す際、ジェネリック制約によりその関数型を束縛する必要がある
- **型注釈の形式**：`(T: Type) -> ((f: (T) -> T, x: T) -> T)` - ジェネリックパラメータが関数型を束縛
- **HMの動作フロー**：ジェネリックパラメータを通じて関数型を推論し、多相関数の合成を実現

**例示**：

```yaoxiang
# ✅ 高階多相をサポート：ジェネリックで関数型パラメータを束縛
call_twice: (T: Type) -> ((f: (T) -> T, x: T) -> T) = {
    return f(f(x))
}
# 使用：call_twice((x) => x + 1, 5)  # T=Int と推論

compose: (A: Type, B: Type, C: Type) -> ((f: (B) -> C, g: (A) -> B, x: A) -> C) = {
    return f(g(x))
}
# 使用：compose((x) => x * 2, (x) => x + 1, 5)  # A=Int, B=Int, C=Int と推論

# ❌ 非サポート：ジェネリック制約がない高階関数
# bad_hof: (f, x) => f(f(x))  # HMが推論できず、ジェネリックパラメータが欠如
```

**HM推論プロセス**：

1. 高階関数のパラメータを認識：`f: (T) -> T`
2. ジェネリック制約を生成：`(T: Type)`
3. ジェネリックのインスタンス化により具体型を推論
4. 多相関数の合成を実現

### Lambda 式の構文ルール

**重要ルール**：コードブロック `{ ... }` の値は**末尾式**によって決まる（唯一の出口）；`return` は
`Never` 型の非局所退出であり、最も近い関数の境界を抜ける。式形式 `= expr` は値を直接与える。詳細は
[RFC-010a](./010a-tail-expression-and-return.md) を参照。

| 構文形式               | 構文             | 値の出口                            |
| ---------------------- | ---------------- | ----------------------------------- |
| **コードブロック形式** | `{ statements }` | 末尾式（空ブロック `{}` は `Void`） |
| **式形式**             | `expression`     | 式の値                              |
| **`return`**           | `return e`       | 関数からの非局所退出、型 `Never`    |

**例**：

```yaoxiang
main: () -> Void = { println("Hello") }         # 末尾式が Void
add: (a: Int, b: Int) -> Int = { a + b }        # 末尾式が値を与える
empty: () -> Void = {}                          # 空ブロック → Void

# 早期return：return を使用
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

# 式形式：値を直接与える
add: (a: Int, b: Int) -> Int = a + b            # 正しい：式形式
main: () -> Void = println("Hello")               # 正しい：式形式
```

**核心思想**：

1. 関数定義はHMアルゴリズムにより型推論を行い、極力推論し、推論できない場合は明示的にエラー
2. **HMアルゴリズムの動作原理**：演算子の型制約、関数呼び出し関係などの文脈情報により自動的に型を推論
3. **ジェネリックサポート**：多相関数はジェネリック構文 `(T: Type)`
   により型パラメータを明示的に束縛する（RFC-010/011）
4. **推論の境界**：戻り型とローカル変数は推論可能；引数あり関数のパラメータ型は明示的に注釈が必要（署名または lambda 頭のいずれか）
5. 引数なし・戻り値なし関数は `name: () -> Void = { ... }` を使用し、RFC-010 と統一
6. 旧構文を廃止し、移行ツールを提供

**型推論の例**：

```yaoxiang
# ジェネリック関数：明示的な型パラメータ（RFC-010 の統一構文を使用）
identity: (T: Type) -> ((x: T) -> T) = x
map: (T: Type, R: Type) -> ((f: (T) -> R, list: List(T)) -> List(R)) = {
    result = List(R)()
    for item in list { result.push(f(item)) }
    return result
}

# 多相関数：明示的なジェネリック制約による定義（RFC-010/011）
add: (T: Add) -> ((a: T, b: T) -> T) = a + b
print_sum: (a: Int, b: Int) -> Void = { println(a + b) }  # (Int, Int) -> Void と推論

# 高階多相：ジェネリック型注釈によりHMが高階多相をサポート
call_twice: (T: Type) -> ((f: (T) -> T, x: T) -> T) = { return f(f(x)) }
compose: (A: Type, B: Type, C: Type) -> ((f: (B) -> C, g: (A) -> B, x: A) -> C) = { return f(g(x)) }
```

```yaoxiang
# === 関数定義：HMアルゴリズムによる型推論 ===

# 標準関数：HMアルゴリズムが戻り型を推論（パラメータ型は明示が必要）
add = (a: Int, b: Int) => a + b            # (a: Int, b: Int) -> Int と推論
main: () -> Void = { println("Hello") }                # () -> Void と推論

# 部分的に明示的なパラメータ：HMアルゴリズムが残りを推論
print_sum: (a: Int, b: Int) -> Void = { println(a + b) }  # (Int, Int) -> Void と推論
greet: (name: String) -> Void = { println("Hello " + name) }  # (String) -> Void と推論

# ジェネリック関数：多相型パラメータを明示的に束縛（RFC-010 の統一構文を使用）
identity: (T: Type) -> ((x: T) -> T) = x
map: (T: Type, R: Type) -> ((f: (T) -> R, list: List(T)) -> List(R)) = {
    # map 関数の実装
    return List(R)()
}

# 再帰関数：HMアルゴリズムと再帰制約により推論
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 } else { return n * factorial(n - 1) }
}

# === 変数代入：HMアルゴリズムによる型推論 ===

# 明示的な型
x: Int = 42

# HMアルゴリズムにより自動的に Int と推論
y = 42                               # Int と推論

# HMアルゴリズムにより自動的に String と推論
name = "YaoXiang"                    # String と推論

# HMアルゴリズムにより自動的に Float と推論
pi = 3.14159                         # Float と推論
```

**HM型推論ルール**：

| シナリオ            | 構文                                              | 省略可能部分 | 例                            |
| ------------------- | ------------------------------------------------- | ------------ | ----------------------------- |
| **完全形式**        | `name: (a: Type, b: Type) -> Ret = (a, b) => ...` | なし         | 署名 + Lambda 頭完全          |
| **Lambda 頭省略**   | `name: (a: Type, b: Type) -> Ret = ...`           | Lambda 頭    | 署名がパラメータを宣言済み    |
| **署名省略**        | `name = (a: Type, b: Type) => ...`                | 署名         | lambda 頭がパラメータ型を提供 |
| **戻り型 Ret 省略** | `name: (a: Type, b: Type) -> = ...`               | 戻り型       | HM が戻り型を推論             |
| **空引数完全**      | `name: () -> Void = () => { ... }`                | なし         | 空引数関数の完全形            |
| **空引数省略**      | `name: () -> Void = { ... }`                      | Lambda 頭    | `() =>` を省略                |
| **空引数最簡**      | `name = { ... }`                                  | 全部         | 引数なし・戻り値なしの最簡形  |
| **変数代入**        | `name = value`                                    | 型           | HM が型を推論                 |
| **明示的変数**      | `name: Type = value`                              | なし         | 明示的な型注釈                |

**核心原則**：

- `->` は関数型の目印であり、省略不可（否则タプルとして解釈される）
- 戻り型 `Ret` は省略可能で、HMが関数本体から推論する
- 入力パラメータが存在する場合、パラメータ型は明示的に出現する必要がある（署名または lambda 頭のいずれか）
- その他の部分は推論可能かつ曖昧さを導入しない場合に省略可能
- 暗黙的な型変換はなく、JavaScript のような混乱を回避

## 詳細設計

### シンタックスシュガーの展開

省略の有無に関わらず、最終的には統一中間表現に正規化される：

```rust
// 完全形式
add: (a: Int, b: Int) -> Int = (a, b) => a + b

// 展開後のIR
let add: (Int, Int) -> Int = |a: Int, b: Int| -> Int {
    return a + b
};

// Lambda 頭を省略
add: (a: Int, b: Int) -> Int = a + b

// 展開後のIR（完全形式と同じ）
let add: (Int, Int) -> Int = |a: Int, b: Int| -> Int {
    return a + b
};

// 署名を省略（lambda 頭でパラメータ型を注釈）
add = (a: Int, b: Int) => a + b

// 展開後のIR
let add: (Int, Int) -> Int = |a: Int, b: Int| -> Int {
    a + b
};
```

### 構文定義

```bnf
function_def ::= identifier ':' type_expr '=' expression
               | identifier '=' expression
               | identifier '=' block                    # 最簡形式：引数なし・戻り値なし

identifier ::= [a-zA-Z_][a-zA-Z0-9_]*

type_expr ::= identifier                     # 型参照
       | '()'                          # 空型
       | '(' parameters ')' '->' type_expr   # 関数型（パラメータ名は署名内）
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

# 注意：コードブロック内では return で値を返す必要がある；return がない場合はデフォルトで Void を返す
# 例：{ return 1 + 1 } は Int を返す；{ println("Hello") } は Void を返す
# 注意：ジェネリックパラメータは (T: Type) 構文を使用し、関数型の一部として、独立した BNF ルールは不要
```

### エラー処理

```yaoxiang
# === コンパイルエラー例 ===

# エラー1：コードブロックの戻り型が一致しない
add: (a: Int, b: Int) -> Int = { println(a + b) }
// エラー：ブロック内に return がない、デフォルトで Void を返すが、署名は Int を期待
// 正しい：add: (a: Int, b: Int) -> Int = a + b
// または：add: (a: Int, b: Int) -> Int = { return a + b }

# エラー2：未宣言の型パラメータを使用
identity: (x: T) -> T = x
// エラー：T が未宣言；明示的なジェネリックパラメータが必要（RFC-010）
// 正しい：identity: (T: Type) -> ((x: T) -> T) = x

# 正しい：HMアルゴリズムが戻り型を推論
double = (x: Int) => x + x

# 完全形式（段階的に省略）
double: (x: Int) -> Int = (x) => x + x                # 完全
double: (x: Int) -> Int = x + x                       # Lambda 頭を省略
double = (x: Int) => x + x                            # 戻り型を省略（HM が推論）
# double = (x) => x + x                               # ❌ パラメータ型は両方の省略を許可しない
```

## トレードオフ

### 利点

- **構文の統一**：`name: Signature = LambdaBody` モデルがすべてのシナリオをカバー
- **柔軟な省略**：任意の部分をHMで推論できる場合は省略可能
- **型安全性**：HMアルゴリズムが型安全性を保証し、暗黙的な型変換を回避
- **再帰サポート**：HMアルゴリズムと再帰制約により自動的に型を推論
- **ゼロ負荷**：完全形式から最簡形式までスムーズに遷移

### 欠点

- **移行コスト**：旧コードは移行ツールによる変換が必要
- **学習コスト**：「完全形式 + 任意の省略」モデルを理解する必要がある

## 代替案

| 案                   | 説明                                         | 採用しない理由                                       |
| -------------------- | -------------------------------------------- | ---------------------------------------------------- |
| HMアルゴリズム型推論 | Hindley-Milnerアルゴリズムによる型推論を使用 | ✅ **採用済み**、現代的な関数型言語の標準            |
| 明示的な型宣言       | すべての型を明示的に書く                     | 簡素化構文の原則に反し、ボイラープレートコードが増加 |
| 旧構文の保持         | 新旧構文を同時にサポート                     | 構文の分裂、保守コストが高い                         |
| fn キーワード        | 関数と変数を区別するために fn を導入         | 「関数とは lambda である」という設計に反する         |

## 実装戦略

### 段階分け

1. **Phase 1: 構文解析とHMアルゴリズム**（v0.3）
   - 新構文 `name = lambda` + HMアルゴリズムによる型推論を実装
   - 引数なし・戻り値なしのデフォルト補完を実装

2. **Phase 2: 移行ツール**（v0.3）
   - `yaoxiang-migrate --old-to-new` ツールを開発
   - 旧構文のコードを自動変換

3. **Phase 3: 検証とドキュメント**（v0.3）
   - 旧コードの移行完了検証
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

# === 新構文：完全形式（署名完全 + Lambda 頭完全）===
add: (a: Int, b: Int) -> Int = (a, b) => a + b
main: () -> Void = () => { println("Hello") }

# === 省略：Lambda 頭を省略 ===
add: (a: Int, b: Int) -> Int = a + b
main: () -> Void = { println("Hello") }

# === 省略：HM 推論 ===
add = (a: Int, b: Int) => a + b              # (a: Int, b: Int) -> Int と推論
main: () -> Void = { println("Hello") }                  # () -> Void と推論

# === 最簡形式 ===
main: () -> Void = {                                      # main: () -> Void = { ... } と同等
    println("Hello")
}
```

### 依存関係

- 外部依存なし
- 独立して実装可能

### リスク

| リスク         | 影響                     | 緩和策                                           |
| -------------- | ------------------------ | ------------------------------------------------ |
| 移行漏れ       | 旧コードのコンパイル失敗 | 移行ツールを提供し、すべての旧構文パターンを網羅 |
| パーサーエラー | 構文解析の不安定さ       | 十分なテストカバレッジ                           |

## 未解決問題

> 以下の問題は設計で既に解決済みであり、付録Aに記録されている。

- ~~Q1: `main() = body` のような極簡記法を保持すべきか？~~ → 解決済み：`main: () -> Void = { ... }`
  として保持
- ~~Q2: 関数名の後の `:` を保持するか？~~
  → 解決済み：保持は任意；ただし引数あり関数は引き続き署名または lambda 頭でパラメータ型を注釈する必要あり
- ~~Q3: HMアルゴリズムはパラメータ型推論をサポートするか？~~
  → 解決済み：戻り値/ローカルは推論可能；引数あり関数のパラメータ型は明示的に注釈が必要
- ~~Q4: `fn` キーワードを導入するか？~~ → 解決済み：導入しない、関数は lambda そのもの
- ~~Q5: 旧コードの移行戦略は？~~ → 解決済み：`yaoxiang-migrate` ツールを提供
- ~~Q6: ジェネリック関数の使用方法は？~~ → 解決済み：RFC-010 の統一構文 `(T: Type)` を使用

---

## 付録

### 付録A：各言語の関数定義構文リファレンス

| 言語         | 構文スタイル                                        | 特徴                                 |
| ------------ | --------------------------------------------------- | ------------------------------------ |
| Rust         | `fn add(a: i32, b: i32) -> i32 { ... }`             | キーワード + 型注釈                  |
| Haskell      | `add a b = ...` / `add :: Int -> Int -> Int`        | 型署名の分離                         |
| OCaml        | `let add a b = ...`                                 | パラメータ型は省略可能               |
| MoonBit      | `fn add(a: Int, b: Int): Int { ... }`               | 簡潔な型注釈                         |
| TypeScript   | `const add = (a: number, b: number): number => ...` | Lambda スタイル                      |
| Scala        | `def add(a: Int, b: Int): Int = { ... }`            | def キーワード                       |
| **YaoXiang** | `name = (a: Int, b: Int) => a + b`                  | **関数 = lambda、HM が戻り値を推論** |

### 付録B：設計決定の記録

| 決定           | 内容                                                         | 日付       | 記録者  |
| -------------- | ------------------------------------------------------------ | ---------- | ------- |
| 構文スタイル   | 新構文 `name: (params) -> Return = body` + HM推論            | 2026-02-03 | @沫郁酱 |
| パラメータ位置 | パラメータ名は署名内で宣言、RFC-010 と統一                   | 2026-02-03 | @沫郁酱 |
| デフォルト補完 | 空引数関数は署名を省略可能、空ブロック `{}` は `Void` と推論 | 2026-02-03 | @沫郁酱 |
| 型推論         | HMアルゴリズムにより自動推論、推論できない場合は明示         | 2026-01-06 | @沫郁酱 |
| 旧構文         | 廃止、移行ツールを提供                                       | 2026-01-06 | @沫郁酱 |
| fn キーワード  | 導入しない                                                   | 2026-01-06 | @沫郁酱 |
| 再帰宣言       | HMアルゴリズムと再帰制約により自動推論                       | 2026-01-06 | @沫郁酱 |

### 付録C：用語集

| 用語                 | 定義                                                                                                     |
| -------------------- | -------------------------------------------------------------------------------------------------------- |
| HMアルゴリズム       | Hindley-Milner型推論アルゴリズム、関数と変数の型を自動推論                                               |
| ジェネリック         | 型パラメータ `(T: Type)` により多相関数を束縛、例：`identity: (T: Type) -> ((x: T) -> T) = x`（RFC-010） |
| デフォルト型補完     | 引数なし・戻り値なし関数では `-> Void` を省略、コンパイラが自動補完                                      |
| シンタックスシュガー | コードをより読みやすくする構文の簡略記法                                                                 |
| 正規化               | 構文形式を統一内部表現に変換                                                                             |
| 関数 = lambda        | 関数は本質的に lambda 変数であり、型はHMアルゴリズムにより自動推論                                       |

---

## 参考文献

- [MoonBit 言語設計](https://moonbitlang.com/)
- [Rust 関数構文](https://doc.rust-lang.org/book/ch03-03-how-functions-work.html)
- [Haskell 型システム](https://www.haskell.org/tutorial/patterns.html)
- [OCaml 型推論](https://v2.ocaml.org/manual/)

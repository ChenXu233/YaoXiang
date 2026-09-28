---
title: 'RFC-007: 関数定義構文統一方案'
issue: '#131'
status: '受領済み'
author: '沫郁酱'
created: '2025-01-05'
updated: '2026-09-15'
---

# RFC-007: 関数定義構文統一方案

> **関連**：「空引数最小形」`name = { ... }`
> と RFC-010 の「値ブロック」構文位置が重複する箇所においては、裁定は**内容が型を決定する**となる——`=>`
> は常に関数、`Fn` 注釈は関数、非 `Fn` 注釈はブロック値、注釈なし時は内容から推論。参见
> [RFC-010a](./010a-tail-expression-and-return.md) 付録D。
>
> **関連補足**：関数本体（`{ ... }` コードブロック）内の文終端と改行規則（`;`
> 明示的分離、改行終端、継続行例外）は [RFC-038（草案）](./038-statement-termination.md)
> で定義されており、本 RFC の範囲外とする。

## 概要

本 RFC は YaoXiang 言語の**関数定義構文**の最終方案を確定する。統一構文
`name: (params) -> Return = body` を使用し、RFC-010 の `name: type = value`
モデルと完全に一致させる。

曖昧さを避けるため：関数が入力パラメータを持つ場合、パラメータ型は「シグネチャ」または「lambda ヘッダ」の少なくとも片方で明示的に标注する必要がある；両方を省略した場合は拒否される。

コードブロック `{ ... }` の値は**末尾式**によって与えられる；`return` は `Never`
型の非局所退出である（参见 [RFC-010a](./010a-tail-expression-and-return.md)）。式形式 `= expr`
は直接的に値を与える。

## 動機

### なぜこの特性が必要か？

1. **構文の一貫性**：旧構文の歴史的包袱を排除し、スタイルを統一する
2. **簡潔性**：HM アルゴリズムが型を自動推論し、ボイラープレートを削減する
3. **型安全性**：HM アルゴリズムが型安全性を保証し、推論できない時だけ明示的に标注する
4. **言語成熟度**：HM アルゴリズムは現代関数型言語の成熟した方案である

### 統一構文モデル

**核心原則**：`name: Signature = LambdaBody`

- **完全形**：シグネチャ（パラメータ名 + 型 + `->` + 返り値型）+ Lambda ヘッダ（パラメータ名を含む）
- **省略規則**：曖昧さを導入しない前提下でできるだけ省略する
  - `->` は省略不可（関数型の印であり、省略するとタプルとして解析される）
  - **入力パラメータがある場合**、パラメータ型はシグネチャまたは lambda ヘッダの少なくとも片方で明示的に出現する必要がある
  - Lambda ヘッダは省略可能 → シグネチャがパラメータ名と型を既に宣言している場合
  - 返り値型は明示的に标注可能であり、推論可能な場合は省略可能

```yaoxiang
# 完全形（シグネチャ完全 + Lambda ヘッダ完全）
add: (a: Int, b: Int) -> Int = (a, b) => a + b

# 省略：Lambda ヘッダを省略（シグネチャがパラメータを宣言済み）
add: (a: Int, b: Int) -> Int = a + b

# 省略：シグネチャを省略（lambda ヘッダがパラメータ型を标注）
add = (a: Int, b: Int) => a + b

# ❌ エラー：両方にパラメータ型が标注されていない
# add = (a, b) => a + b
```

### 設計目標

```yaoxiang
# === 完全形 ===
add: (a: Int, b: Int) -> Int = (a, b) => { a + b }

# === 省略形 ===
add: (a: Int, b: Int) -> Int = a + b                 # Lambda ヘッダを省略
add = (a: Int, b: Int) => a + b                      # シグネチャを省略

# === 空引数関数 ===
main: () -> Void = () => { println("Hello") }          # 完全形
main: () -> Void = { println("Hello") }                # Lambda ヘッダを省略
main: () -> Void = { println("Hello") }                            # 最小形（() -> Void と推論）

# === ジェネリック関数（RFC-010 統一構文を使用）===
identity: (T: Type) -> ((x: T) -> T) = (x) => x         # 完全形
identity: (T: Type) -> ((x: T) -> T) = x                # Lambda ヘッダを省略
identity = (x: T) => x                                  # シグネチャを省略（lambda ヘッダが型を标注）

# === 递归函数 ===
factorial: (n: Int) -> Int = (n) => {
    if n <= 1 { return 1 } else { return n * factorial(n - 1) }
}
```

### 構文規則

| シナリオ              | 構文                                                   | 説明                              |
| --------------------- | ------------------------------------------------------ | --------------------------------- |
| **完全形**            | `name: (a: Type, b) -> Ret = (a, b) => { return ... }` | シグネチャ + Lambda ヘッダ完全    |
| **Lambda ヘッダ省略** | `name: (a: Type, b: Type) -> Ret = { ... }`            | シグネチャがパラメータを宣言済み  |
| **シグネチャ省略**    | `name = (a: Type, b: Type) => { ... }`                 | lambda ヘッダがパラメータ型を标注 |
| **空引数完全形**      | `name: () -> Void = () => { return ... }`              | 空引数関数完全形                  |
| **空引数省略形**      | `name: () -> Void = { return ... }`                    | Lambda ヘッダを省略               |
| **空引数最小形**      | `name = { return ... }`                                | 引数なし返り値なし最小形          |

**注意**：コードブロック `{ ... }` の値は**末尾式**によって与えられる（唯一の出口）；`return` は
`Never` 型の非局所退出であり、最も近い関数境界を脱出する。式形式 `= expr`
は直接的に値を与える。詳細については [RFC-010a](./010a-tail-expression-and-return.md) を参照。

**注意**：`->` は関数型の印であり、省略不可（省略するとタプルとして解析される）。

**重要**：`if` 式はブランチを波括弧 `{}` で囲み、`then/else` キーワードはサポートしない：

```yaoxiang
# 正しい：波括弧を使用
if n <= 1 { return 1 } else { return n * factorial(n - 1) }

# 間違い：then/else キーワードはサポートしない
# if n <= 1 then return 1 else return n * factorial(n - 1)
```

## 提案

### HM アルゴリズムと高階多态サポート

**核心的特性**：HM アルゴリズムはジェネリック型注釈を通じて高階多态（Higher-rank
polymorphism）をサポートする

**設計原理**：

- **高階関数**：関数をパラメータとして渡す際、その関数型をジェネリックに制約する必要がある
- **型注釈形式**：`(T: Type) -> ((f: (T) -> T, x: T) -> T)` - ジェネリックパラメータが関数型を制約する
- **HM ワークフロー**：ジェネリックパラメータを通じて関数型を推論し、多态関数合成を実現する

**サンプル説明**：

```yaoxiang
# ✅ 高階多态サポート：ジェネリックが関数型パラメータを制約
call_twice: (T: Type) -> ((f: (T) -> T, x: T) -> T) = {
    return f(f(x))
}
# 使用：call_twice((x) => x + 1, 5)  # 推断 T=Int

compose: (A: Type, B: Type, C: Type) -> ((f: (B) -> C, g: (A) -> B, x: A) -> C) = {
    return f(g(x))
}
# 使用：compose((x) => x * 2, (x) => x + 1, 5)  # 推断 A=Int, B=Int, C=Int

# ❌ サポート外：ジェネリック制約がない高階関数
# bad_hof: (f, x) => f(f(x))  # HM が推論できず、ジェネリックパラメータが不足
```

**HM 推論プロセス**：

1. 高階関数パラメータの識別：`f: (T) -> T`
2. ジェネリック制約の生成：`(T: Type)`
3. ジェネリックインスタンス化を通じて具体的な型を推論する
4. 多态関数合成の実現

### Lambda 式構文規則

**重要な規則**：コードブロック `{ ... }` の値は**末尾式**によって与えられる（唯一の出口）；`return`
は `Never` 型の非局所退出であり、最も近い関数境界を脱出する。式形式 `= expr`
は直接的に値を与える。詳細については [RFC-010a](./010a-tail-expression-and-return.md) を参照。

| 構文形式             | 構文             | 値出口                              |
| -------------------- | ---------------- | ----------------------------------- |
| **コードブロック形** | `{ statements }` | 末尾式（空ブロック `{}` は `Void`） |
| **式形式**           | `expression`     | 式値                                |
| **`return`**         | `return e`       | 非局所退出関数、型 `Never`          |

**サンプル**：

```yaoxiang
main: () -> Void = { println("Hello") }         # 尾表达式为 Void
add: (a: Int, b: Int) -> Int = { a + b }        # 尾表达式给出值
empty: () -> Void = {}                          # 空块 → Void

# 提前返回：使用 return
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

# 式形式：直接的に値を与える
add: (a: Int, b: Int) -> Int = a + b            # 正しい：式形式
main: () -> Void = println("Hello")               # 正しい：式形式
```

**核心的な考え方**：

1. 関数定義は HM アルゴリズムを通じて型推論を行い、尽量推論し、推論できない時は明示的にエラーを出す
2. **HM アルゴリズムの動作原理**：演算子の型制約、関数呼び出し関係などのコンテキスト情報に基づいて型を自動推論する
3. **ジェネリックサポート**：多态関数は `(T: Type)`
   ジェネリック構文を使用して型パラメータを明示的に制約する（RFC-010/011）
4. **推論境界**：返り値と局所変数は推論可能；有引数関数のパラメータ型は明示的に标注する必要がある（シグネチャまたは lambda ヘッダのいずれか）
5. 空引数返り値なし関数は `name: () -> Void = { ... }` を使用し、RFC-010 と統一する
6. 旧構文は退役し、移行ツールを提供する

**型推論サンプル**：

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
print_sum: (a: Int, b: Int) -> Void = { println(a + b) }  # 推断为 (Int, Int) -> Void

# 高階多态：ジェネリック型注釈を通じて HM がより高階多态をサポート
call_twice: (T: Type) -> ((f: (T) -> T, x: T) -> T) = { return f(f(x)) }
compose: (A: Type, B: Type, C: Type) -> ((f: (B) -> C, g: (A) -> B, x: A) -> C) = { return f(g(x)) }
```

```yaoxiang
# === 関数定義：HM アルゴリズム型推論 ===

# 標準関数：HM アルゴリズムが返り値型を推論（パラメータ型は明示的必要）
add = (a: Int, b: Int) => a + b            # (a: Int, b: Int) -> Int と推論
main: () -> Void = { println("Hello") }                # () -> Void と推論

# 部分的に明示的なパラメータ：HM アルゴリズムが残りを推論
print_sum: (a: Int, b: Int) -> Void = { println(a + b) }  # (Int, Int) -> Void と推論
greet: (name: String) -> Void = { println("Hello " + name) }  # (String) -> Void と推論

# ジェネリック関数：多态型パラメータを明示的に制約（RFC-010 統一構文を使用）
identity: (T: Type) -> ((x: T) -> T) = x
map: (T: Type, R: Type) -> ((f: (T) -> R, list: List(T)) -> List(R)) = {
    # 实现 map 函数
    return List(R)()
}

# 再帰関数：HM アルゴリズムと再帰制約を通じて推論
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 } else { return n * factorial(n - 1) }
}

# === 変数代入：HM アルゴリズム型推論 ===

# 显式类型
x: Int = 42

# HM アルゴリズムが自動的に Int と推論
y = 42                               # Int と推論

# HM アルゴリズムが自動的に String と推論
name = "YaoXiang"                    # String と推論

# HM アルゴリズムが自動的に Float と推論
pi = 3.14159                         # Float と推論
```

**HM 型推論規則**：

| シナリオ              | 構文                                              | 省略可能部分  | サンプル                          |
| --------------------- | ------------------------------------------------- | ------------- | --------------------------------- |
| **完全形**            | `name: (a: Type, b: Type) -> Ret = (a, b) => ...` | なし          | シグネチャ + Lambda ヘッダ完全    |
| **Lambda ヘッダ省略** | `name: (a: Type, b: Type) -> Ret = ...`           | Lambda ヘッダ | シグネチャがパラメータを宣言済み  |
| **シグネチャ省略**    | `name = (a: Type, b: Type) => ...`                | シグネチャ    | lambda ヘッダがパラメータ型を提供 |
| **返り値 Ret 省略**   | `name: (a: Type, b: Type) -> = ...`               | 返り値型      | HM が返り値型を推論               |
| **空引数完全形**      | `name: () -> Void = () => { ... }`                | なし          | 空引数関数完全形                  |
| **空引数省略形**      | `name: () -> Void = { ... }`                      | Lambda ヘッダ | `() =>` を省略                    |
| **空引数最小形**      | `name = { ... }`                                  | 全部          | 引数なし返り値なし最小形          |
| **変数代入**          | `name = value`                                    | 型            | HM が型を推論                     |
| **明示的な変数**      | `name: Type = value`                              | なし          | 明示的な型注釈                    |

**核心的な原則**：

- `->` は関数型の印であり、省略不可（省略するとタプルとして解析される）
- 返り値型 `Ret` は省略可能であり、HM が関数本体に基づいて推論する
- 入力パラメータが存在する場合、パラメータ型は明示的に出現する必要がある（シグネチャまたは lambda ヘッダのいずれか）
- 残りの部分は推論可能かつ曖昧さを導入しない場合に省略可能
- 暗黙的な型変換なし、JavaScript 的な混乱を避ける

## 詳細な設計

### シンタックスシュガー展開

省略の有無に関わらず、最終的には統一された中間表現に正規化される：

```rust
// 完全形
add: (a: Int, b: Int) -> Int = (a, b) => a + b

// 展开后 IR
let add: (Int, Int) -> Int = |a: Int, b: Int| -> Int {
    return a + b
};

// Lambda ヘッダを省略
add: (a: Int, b: Int) -> Int = a + b

// 展開後 IR（完全形と同じ）
let add: (Int, Int) -> Int = |a: Int, b: Int| -> Int {
    return a + b
};

// シグネチャを省略（lambda ヘッダがパラメータ型を标注）
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
               | identifier '=' block                    # 最小形：引数なし返り値なし

identifier ::= [a-zA-Z_][a-zA-Z0-9_]*

type_expr ::= identifier                     # 型参照
       | '()'                          # 空型
       | '(' parameters ')' '->' type_expr   # 関数型（シグネチャにパラメータ名あり）
       | type_expr '->' type_expr            # 単純関数型
       | identifier '(' type_expr (',' type_expr)* ')'  # 型応用

expression ::= '(' parameters ')' '=>' block
             | '(' ')' '=>' block
             | '(' parameters ')' '=>' expression

parameters ::= parameter (',' parameter)*
parameter ::= identifier                # 型推論
            | identifier ':' type_expr      # 一部明示的な型

block ::= '{' statement (',' statement)* '}'
        | expression

statement ::= identifier ':' expression  # 代入文
           | expression                  # 式文（実行するが返さない）
           | 'return' expression         # return 文（指定した値を返す）

# 注意：コードブロック内では return を使用して値を返する必要がある；return がない場合はデフォルトで Void を返す
# 例：{ return 1 + 1 } は Int を返す；{ println("Hello") } は Void を返す
# 注意：ジェネリックパラメータは (T: Type) 構文を使用し、関数型の一部となるため、独立した BNF 規則は不要
```

### エラー処理

```yaoxiang
# === コンパイルエラーサンプル ===

# エラー1：コードブロックの返り値型が一致しない
add: (a: Int, b: Int) -> Int = { println(a + b) }
// エラー：ブロック内に return がない、デフォルトで Void を返すが、シグネチャは Int を期待している
// 正しい：add: (a: Int, b: Int) -> Int = a + b
// または：add: (a: Int, b: Int) -> Int = { return a + b }

# エラー2：未宣言の型パラメータを使用
identity: (x: T) -> T = x
// 错误：T 未声明；需要显式泛型参数（RFC-010）
// 正确：identity: (T: Type) -> ((x: T) -> T) = x

# 正しい：HM アルゴリズムが返り値型を推論
double = (x: Int) => x + x

# 完全形（段階的に省略）
double: (x: Int) -> Int = (x) => x + x                # 完全
double: (x: Int) -> Int = x + x                       # Lambda ヘッダを省略
double = (x: Int) => x + x                            # 返り値型を省略（HM が返り値を推論）
# double = (x) => x + x                               # ❌ パラメータ型は両方を同時に省略不可
```

## トレードオフ

### 优点

- **構文の統一**：`name: Signature = LambdaBody` モデルがすべてのシナリオをカバー
- **柔軟な省略**：HM で推論可能な任意のパートを省略可能
- **型安全性**：HM アルゴリズムが型安全性を保証し、暗黙的な型変換を回避
- **再帰サポート**：HM アルゴリズムと再帰制約が自動的に型を推論
- **ゼロ負担**：完全形から最小形への滑らかな移行

### 缺点

- **移行コスト**：旧コードは移行ツールで変換する必要がある
- **学習コスト**：「完全形 + 任意の省略」モデルを理解する必要がある

## 代替方案

| 方案                  | 説明                                          | なぜ選択しないか                               |
| --------------------- | --------------------------------------------- | ---------------------------------------------- |
| HM アルゴリズム型推論 | Hindley-Milner アルゴリズムを使用して型を推論 | ✅ **採用済み**、現代関数型言語の標準          |
| 明示的な型宣言        | すべての型を明示的に記述                      | 簡略化構文の原則に反し、ボイラープレートを増加 |
| 旧構文の保持          | 新旧両方の構文を同時にサポート                | 構文の分裂、维护コストが高い                   |
| fn キーワード         | 関数と変数を区別するために fn を導入          | 「関数は lambda である」という設計に反         |

## 実装戦略

### フェーズ分け

1. **Phase 1: 構文解析と HM アルゴリズム**（v0.3）
   - 新構文 `name = lambda` + HM アルゴリズム型推論を実装
   - 空引数返り値なしのデフォルト埋めを実装

2. **Phase 2: 移行ツール**（v0.3）
   - `yaoxiang-migrate --old-to-new` ツールを開発
   - 旧構文コードを自動的に変換

3. **フェーズ3：検証とドキュメント**（v0.3）
   - 旧コード移行完了の検証
   - ドキュメント更新

### 移行ツール

```bash
# 迁移单个文件
yaoxiang-migrate --old-to-new src/main.yaoxiang

# 迁移整个项目
yaoxiang-migrate --old-to-new --recursive src/

# 移行プレビュー（ファイルは変更しない）
yaoxiang-migrate --old-to-new --dry-run src/main.yaoxiang
```

移行規則：

```yaoxiang
# 旧语法
add(Int, Int) -> Int = (a, b) => { a + b }
main() -> Int = { println("Hello"); 0 }
main() = { println("Hello") }

# === 新構文：完全形（シグネチャ完全 + Lambda ヘッダ完全）===
add: (a: Int, b: Int) -> Int = (a, b) => a + b
main: () -> Void = () => { println("Hello") }

# === 省略：Lambda ヘッダを省略 ===
add: (a: Int, b: Int) -> Int = a + b
main: () -> Void = { println("Hello") }

# === 省略：HM 推論 ===
add = (a: Int, b: Int) => a + b              # (a: Int, b: Int) -> Int と推論
main: () -> Void = { println("Hello") }                  # () -> Void と推論

# === 最小形 ===
main: () -> Void = {                                      # main: () -> Void = { ... } と同等
    println("Hello")
}
```

### 依存関係

- 外部依存なし
- 独立して実装可能

### リスク

| リスク         | 影響                     | 軽減措置                                         |
| -------------- | ------------------------ | ------------------------------------------------ |
| 移行漏れ       | 旧コードがコンパイル失敗 | 移行ツールを提供、すべての旧構文パターンをカバー |
| パーサーエラー | 構文解析が不安定         | 十分なテストカバレッジ                           |

## 開放問題

> 以下の問題は設計で既に解決済みであり、付録Aに記録されている。

- ~~Q1: `main() = body` のような極限簡略表記を保持すべきか？~~
  → 解決済み：`main: () -> Void = { ... }` として保持
- ~~Q2: 関数名の後の `:` を保持するか？~~
  → 解決済み：オプションで保持可能；但し有引数関数はシグネチャまたは lambda ヘッダのいずれかでパラメータ型を标注する必要がある
- ~~Q3: HM アルゴリズムはパラメータ型推論をサポートするか？~~
  → 解決済み：返り値/局所変数は推論可能；有引数関数のパラメータ型は明示的に标注が必要
- ~~Q4: `fn` キーワードを導入するか？~~ → 解決済み：導入しない、関数は lambda である
- ~~Q5: 旧コードの移行策略は？~~ → 解決済み：`yaoxiang-migrate` ツールを提供
- ~~Q6: ジェネリック関数の使用方法は？~~ → 解決済み：RFC-010 統一構文 `(T: Type)` を使用

---

## 付録

### 付録A：各言語の関数定義構文参照

| 言語         | 構文スタイル                                        | 特徴                                 |
| ------------ | --------------------------------------------------- | ------------------------------------ |
| Rust         | `fn add(a: i32, b: i32) -> i32 { ... }`             | キーワード + 型注釈                  |
| Haskell      | `add a b = ...` / `add :: Int -> Int -> Int`        | 型シグネチャが分離                   |
| OCaml        | `let add a b = ...`                                 | パラメータ型は省略可能               |
| MoonBit      | `fn add(a: Int, b: Int): Int { ... }`               | 簡潔な型注釈                         |
| TypeScript   | `const add = (a: number, b: number): number => ...` | Lambda スタイル                      |
| Scala        | `def add(a: Int, b: Int): Int = { ... }`            | def キーワード                       |
| **YaoXiang** | `name = (a: Int, b: Int) => a + b`                  | **関数 = lambda、HM が返り値を推論** |

### 付録B：設計意思決定記録

| 意思決定       | 決定                                                               | 日付       | 記録人  |
| -------------- | ------------------------------------------------------------------ | ---------- | ------- |
| 構文スタイル   | 新構文 `name: (params) -> Return = body` + HM推論                  | 2026-02-03 | @沫郁酱 |
| パラメータ位置 | シグネチャにパラメータ名を宣言、RFC-010 と統一                     | 2026-02-03 | @沫郁酱 |
| デフォルト埋め | 空引数関数はシグネチャを省略可能、空ブロック `{}` は `Void` と推論 | 2026-02-03 | @沫郁酱 |
| 型推論         | HM アルゴリズムが自動的に推論、推論できない時は明示的に            | 2026-01-06 | @沫郁酱 |
| 旧構文         | 退役、移行ツールを提供                                             | 2026-01-06 | @沫郁酱 |
| fn キーワード  | 導入しない                                                         | 2026-01-06 | @沫郁酱 |
| 再帰宣言       | HM アルゴリズムと再帰制約が自動的に推論                            | 2026-01-06 | @沫郁酱 |

### 付録C：用語集

| 用語                 | 定義                                                                                                             |
| -------------------- | ---------------------------------------------------------------------------------------------------------------- |
| HM アルゴリズム      | Hindley-Milner 型推論アルゴリズム、関数と変数の型を自動的に推論                                                  |
| ジェネリック         | 型パラメータ `(T: Type)` を使用して多态関数を制約する、例：`identity: (T: Type) -> ((x: T) -> T) = x`（RFC-010） |
| デフォルト型埋め     | 空引数返り値なし関数は `-> Void` を省略可能、コンパイラが自動的に埋める                                          |
| シンタックスシュガー | コードを読みやすくする構文簡略化                                                                                 |
| 正規化               | 構文形式を統一された内部表現に変換                                                                               |
| 関数即 lambda        | 関数は本質的には lambda 変数であり、型は HM アルゴリズムを通じて自動的に推論                                     |

---

## 参考文献

- [MoonBit 言語設計](https://moonbitlang.com/)
- [Rust 関数構文](https://doc.rust-lang.org/book/ch03-03-how-functions-work.html)
- [Haskell 型システム](https://www.haskell.org/tutorial/patterns.html)
- [OCaml 型推論](https://v2.ocaml.org/manual/)

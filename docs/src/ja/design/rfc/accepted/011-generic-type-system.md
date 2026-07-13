```yaml
---
title: "RFC-011: ジェネリクスシステム設計 - ゼロコスト抽象とマクロの代替"
status: "承認済み"
author: "晨煦"
updated: "2026-07-14（IsTrue/Assert 型族 + 型レベル再帰が実装済み、#158/#160 がクローズ済み）"
issue: "#128"
issues_impl:
  - "#45"
  - "#46"
  - "#73"
  - "#90"
  - "#96"
  - "#40"
  - "#151"
pr_impl:
  - "#122"
---
```

# RFC-011: ジェネリクスシステム設計 - ゼロコスト抽象とマクロの代替

## 概要

本ドキュメントはYaoXiang言語の**ジェネリクスシステム設計**を定義する。強力なジェネリクス能力によりゼロコスト抽象を実現し、コンパイル時最適化によってマクロへの依存を削減し、デッドコード除去機構を提供する。

**コア設計**:
- **統一シグネチャ構文**: `(T: Type, R: Type) -> ...` ジェネリクス引数と通常の引数の統一
- **Type 自己記述機構**: `Type` は言語レベルの特別な存在であり、シグネチャ内の `Type` 位置は自動推論されて填充される
- **型制約**: `T: Dup + Add` の多重制約、関数型制約
- **関連型**: `Iterator: (Item: Type) -> Type = { next: () -> Option(Item), has_next: () -> Bool }`
- **コンパイル時ジェネリクス**: `N: Int` ジェネリクス値引数、コンパイル時定数インスタンス化
- **条件型**: `If: (C: Bool, T: Type, E: Type) -> Type` 型レベル計算、型族

**価値**:
- ゼロコスト抽象: コンパイル時単態化、ランタイムオーバーヘッドなし
- デッドコード除去: インスタンス化グラフ解析 + LLVM最適化
- マクロ代替: ジェネリクスにより90%のマクロ使用シーンを代替
- 型安全性: コンパイル時チェック、IDEフレンドリー
- **明示は暗黙より優れる**: `Type` の自己記述、コンパイラによる自動推論

## 参考ドキュメント

本ドキュメントの設計は以下のドキュメントに基づいている:

| ドキュメント | 関係 | 説明 |
|------|------|------|
| [RFC-010: 統一型構文](./010-unified-type-syntax.md) | **構文基盤** | ジェネリクス構文と統一 `name: type = value` モデルの統合 |
| [RFC-010: 統一型構文](./010-unified-type-syntax.md) | **呼び出し構文** | 第6節: ジェネリクス呼び出し構文 - 統一 `()` 適用、`[]` の完全削除 |
| [RFC-009: 所有権モデル](./accepted/009-ownership-model.md) | **型システム** | Move セマンティクスとジェネリクスの自然な結合 |
| [RFC-001: spawn モデル](./accepted/001-concurrent-model-error-handling.md) | **実行モデル** | DAG 解析とジェネリクス型検査 |
| [RFC-008: ランタイムモデル](./accepted/008-runtime-concurrency-model.md) | **コンパイラアーキテクチャ** | ジェネリクス単態化とコンパイル時最適化戦略 |
| [型ユニバース思想](../reference/plan/ongoing/类型宇宙思想.md) | **理論的核心** | 型ユニバース階層モデルと値依存型設計 |
| [RFC-022: ホーア論理静的検証](./draft/022-hol-logic-verification.md) | **停止性検査** | decreases 仕様とコンパイル時評価の安全保証 |

## 型ユニバース思想と値依存型

YaoXiang のジェネリクスシステムは**型ユニバース思想**の上に構築されている。このメンタルモデルは言語のすべての概念を階層化された構造として統一するものであり、その中核となる革新は**値依存型**を Type2 層の第一級市民に格上げした点にある。

### 値依存型とは?

**値依存型**は、1つ以上の**値**(他の型だけでなく)に依存する型である。これらの値はコンパイル時に評価可能であり、コンパイル段階で型安全保証を提供する。

```yaoxiang
# 従来のジェネリクス: 型引数
List: (T: Type) -> Type

# 値依存型: 値引数
Vec: (n: Int) -> Type  # ベクトル型は長さの値 n に依存する
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type  # 行列型は行数と列数に依存する
```

### 値依存型の核心的優位性

従来のジェネリクスと比較して、YaoXiang の値依存型には以下の核心的優位性がある:

| 特性 | 従来のジェネリクス (C++/Rust) | YaoXiang 値依存型 |
|------|-------------------|---------------------|
| 型が依存する値 | 型引数のみ | 関数呼び出し結果を含む任意の値に依存可能 |
| コンパイル時評価 | C++テンプレートは手動特殊化、Rust は不可 | 自動コンパイル時評価、停止を保証 |
| 型レベル計算 | テンプレートメタプログラミング(複雑/危険) | 統一された型レベル計算エンジン |
| 型安全性 | C++ なし、Rust 制限あり | 完全な型安全性、コンパイル時チェック |
| 次元検証 | ランタイムチェックまたは手動特殊化 | コンパイル時次元検証、ランタイムオーバーヘッドなし |

### 型ユニバース階層と値依存型

型ユニバース思想は言語の概念を意味的役割に従って異なる階層に分割する。値依存型は **Type2 層**に位置する:

| 階層 | 役割 | 例 |
|------|------|------|
| Type-1 | 値 | `42`, `factorial(5)`, 関数そのもの |
| Type0 | メタ型キーワード | `Type` |
| Type1 | 具象型 | `Int`, `String`, `Vec(3)` |
| **Type2** | **関数/型コンストラクタ/値依存型** | `add: (Int, Int) -> Int`, `Vec: (n: Int) -> Type`, `Matrix: (T: Type, Rows: Int, Cols: Int) -> Type` |

**重要な設計**: Type2 層の関数、型コンストラクタ、値依存型は**統一構文**を持ち、すべて `(params) -> result` の形式である:
- 通常関数: `(Int, Int) -> Int` → 戻り値は値
- 型コンストラクタ: `(T: Type) -> Type` → 戻り値は型
- 値依存型: `(n: Int) -> Type` → 戻り値は型だが、値引数に依存する

> **Curry-Howard 同型**: この統一は偶然ではない。Curry-Howard 同型は「型は命題、プログラムは証明」であると指摘する - 関数型 `A → B` は論理的含意「A ならば B」に対応し、ジェネリクス `(T: Type) -> Type` は全称量化「すべての型 T について」に対応し、値依存型 `(n: Int) -> Type` は「各整数 n に対して型が存在する」に対応する。YaoXiang は関数、型コンストラクタ、値依存型を Type2 層に統一する。これにより、本質的に「証明」と「計算」を同じ概念 - **構成的証明** - として統一する。これは Curry-Howard 同型の言語設計における直接的な現れである: 一つの形式 (`(params) -> result`) が論理的命題と計算プロセスの両方を担う。

### コンパイル時決定性保証

YaoXiang の型ユニバース思想は要求する: **Type 階層のすべてはコンパイル時に決定されなければならない**。

```yaoxiang
# コンパイル時次元検証の例
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),
    # コンパイル時チェック: 次元は正でなければならない
    _assert: Assert[Rows > 0],
    _assert: Assert[Cols > 0],
}

# 3x3 単位行列の作成 - コンパイル時に完了
identity: (T: Add + Zero + One, N: Int) -> ((size: N) -> Matrix(T, N, N)) = {
    matrix = Matrix(T, N, N)()
    # ...
}

# コンパイル時計算: factorial(3) = 6、ベクトルサイズはコンパイル時に決定
vec: Vec(factorial(3)) = Vec(6)()
```

コンパイラは自動的に:
1. 型位置での関数呼び出しを検出する
2. 関数が `decreases` 仕様でマークされているかを検証する(後述の停止性検査機構を参照)
3. コンパイル時に評価を実行する
4. 結果を生成される型に埋め込む

### 値依存型の適用シナリオ

#### コンパイル時次元検証
```yaoxiang
# 行列乗算: コンパイル時に次元一致を検証
multiply: (T: Add + Multiply + Zero,
           Rows: Int, Cols: Int, M: Int) -> ((
    a: Matrix(T, Rows, Cols),
    b: Matrix(T, Cols, M)
) -> Matrix(T, Rows, M)) = {
    # コンパイル時チェック: a.Cols == b.Rows、そうでなければコンパイルエラー
    result = Matrix(T, Rows, M)()
    # ...
}

# エラーはコンパイル時に捕捉される:
# multiply(matrix_2x3, matrix_4x2)  # コンパイルエラー: 2 != 4
```

#### 型安全な配列サイズ
```yaoxiang
# 配列サイズはコンパイル時定数
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    length: N,
}

# N はコンパイル時定数であり、型レベル計算に使用可能
first_three: Array(Int, 3) = Array(Int, 3)(1, 2, 3)
# first_three.length == 3(コンパイル時既知)
```

#### 条件型
```yaoxiang
# 型レベル If
If: (C: Bool, T: Type, E: Type) -> Type = match C {
    True => T,
    False => E,
}

# 型族
AsString: (T: Type) -> Type = match T {
    Int => String,
    Float => String,
    Bool => String,
    _ => String,
}
```

#### ジェネリック関数
```yaoxiang
# map: ジェネリック関数、型引数 T, R はコンパイル時に決定
map: (T: Type, R: Type) -> (
    (list: List(T), f: (x: T) -> R) -> List(R)
) = (list, f) => {
    result = List(R)()
    for x in list {
        result.push(f(x))
    }
    return result
}

# 使用時は完全に透過的、型は自動推論
numbers = List(1, 2, 3)
doubled = map(numbers, (x) => x * 2)  # map[Int, Int] として推論
```

### 他言語との比較

| 特性 | C++テンプレート | Rust ジェネリクス | Haskell GADT | **YaoXiang** |
|------|---------|----------|--------------|--------------|
| 型引数 | ✅ | ✅ | ✅ | ✅ |
| 値依存型 | ❌ | ❌ | ✅ | ✅ |
| コンパイル時評価 | テンプレートインスタンス化 | ❌ | ✅ | ✅ |
| 停止性保証 | ❌ | ❌ | ❌ (危険) | ✅ (decreases 仕様) |
| 型安全性 | ❌(マクロ展開) | ✅ | ✅ | ✅ |
| 統一構文 | ❌ | ❌ | ❌ | ✅ |
| コンパイル時次元検証 | 手動特殊化 | ランタイムチェック | 型族 | コンパイル時自動検証 |
| decreases 仕様 | ❌ | ❌ | ❌ | ✅ |

### 停止性検査機構(RFC-022との統合)

値依存型のコンパイル時評価は**停止を保証**しなければならない。さもないと型システムが無限ループに陥る。YaoXiang は **decreases 仕様** によってこれを保証し、RFC-022 とシームレスに統合する。

#### 再帰関数の停止性仕様
```yaoxiang
# コンパイル時階乗: 停止を証明しなければならない
factorial: (n: Int) -> Int = {
    //! requires: n >= 0
    //! ensures: result == n!
    //! decreases: n    # 再帰ごとに n は厳密に減少する
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

# 使用: 型位置での呼び出し
vec: Vec(factorial(5)) = Vec(120)()  # コンパイル時評価 factorial(5) = 120
```

#### ループの停止性仕様
```yaoxiang
sum: (arr: Array(Int, n)) -> Int = {
    s = 0; i = 0
    while i < n {
        /*! invariant: s == sum(arr[0..i]) && 0 <= i <= n !*/
        /*! decreases: n - i !*/
        s += arr[i]; i += 1
    }
    return s
}
```

#### 停止性検査のワークフロー

```
┌─────────────────────────────────────────────────────────────┐
│  型検査フェーズ                                              │
│  型位置での関数呼び出しに遭遇(例: Vec(factorial(5)))          │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  1. decreases 仕様をチェック                                  │
│     - decreases あり: すべての再帰パスで減少条件を検証         │
│     - decreases なしだが明らかに停止: 直接評価                 │
│     - decreases なしで停止しない可能性: コンパイルエラー       │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  2. コンパイル時評価(組み込みインタプリタが実行)             │
│     - 純粋関数: 直接評価                                       │
│     - 副作用: コンパイルエラー(型位置は副作用なしが必須)       │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  3. 結果を型に埋め込み                                         │
│     - Vec(factorial(5)) → Vec(120)                          │
│     - Matrix(Float, 3, 3) → 具象型                            │
└─────────────────────────────────────────────────────────────┘
```

#### 優位性

- **安全性**: コンパイル時評価が必ず停止することを保証し、型システムの無限ループを回避
- **統一性**: 停止性検査と部分正当性検証が同じ仕様機構を共有
- **漸進的強化**: ランタイムチェックから完全静的証明へ段階的に移行可能

## 動機

### なぜ強力なジェネリクスシステムが必要か?

現行の主流言語のジェネリクスには限界がある:

| 言語 | ジェネリクス能力 | 問題 |
|------|----------|------|
| Java | 境界型 | コンパイル時単態化、ジェネリクス特殊化なし |
| C# | ジェネリクス制約 | ランタイム型チェック、パフォーマンスオーバーヘッドあり |
| Rust | ジェネリクス + Trait | Trait システムが複雑、学習曲線が急峻 |
| C++ | テンプレート | テンプレート特殊化が複雑、コンパイルエラーが悪い |
| **YaoXiang** | **値依存型** | **型が値に依存可能、コンパイル時次元検証、停止性保証** |

### 核心的矛盾

1. **パフォーマンス vs 柔軟性**: ランタイム柔軟性 vs コンパイル時最適化
2. **複雑 vs 簡潔**: 強力な型システム vs 使いやすさ
3. **マクロ vs ジェネリクス**: マクロコード生成 vs ジェネリクス型安全性
4. **値依存 vs 型安全性**: 従来のジェネリクスはコンパイル時に次元を検証できない

### 値依存型の核心的優位性

YaoXiang の**値依存型**は従来のジェネリクスに対する核心的優位性である:

| 優位性 | 説明 |
|------|------|
| **型が値に依存** | `Vec: (n: Int) -> Type` により型が具体的な値に依存可能 |
| **コンパイル時評価** | 型位置の関数呼び出しはコンパイル時に評価され、結果が型に直接埋め込まれる |
| **次元検証** | `Matrix(Float, 3, 3)` が行列次元をコンパイル時に検証 |
| **型レベル計算** | `If`, `Match` などの条件型が型レベル計算をサポート |
| **停止性保証** | decreases 仕様がコンパイル時評価の停止を保証 |

```yaoxiang
# C++/Rust では不可能なコンパイル時検証
matrix: Matrix(Float, factorial(3), factorial(2)) = ...
# コンパイル時計算: factorial(3) = 6, factorial(2) = 2
# 型は Matrix(Float, 6, 2)

# 次元不一致はコンパイル時に捕捉される
identity: Matrix(Float, 3, 3) = ...
# multiply(matrix_2x3, identity_3x3)  # コンパイルエラー: 2 != 3
```

### ジェネリクスシステムの価値

```yaoxiang
# 例: 統一 API 設計
# 異なるコンテナ型の map 操作

# 従来のアプローチ: 型ごとに個別実装
map_int_array: (array: Array(Int), f: Fn(Int) -> Int) -> Array(Int) = ...
map_string_array: (array: Array(String), f: Fn(String) -> String) -> Array(String) = ...
map_int_list: (list: List(Int), f: Fn(Int) -> Int) -> List(Int) = ...
map_string_list: (list: List(String), f: Fn(String) -> String) -> List(String) = ...

# ジェネリクスアプローチ: 一つのジェネリック関数で全型をカバー
map: (T: Type, R: Type)(container: Container(T), f: Fn(T) -> R) -> Container(R) = {
    for item in container {
        result.push(f(item))
    }
    result
}
```

## 設計目標

### 核心目標

1. **ゼロコスト抽象** - ジェネリクス呼び出しは具象型呼び出しと等価
2. **デッドコード除去** - コンパイル時解析、使用されるジェネリクスのみインスタンス化
3. **マクロ代替** - ジェネリクスにより90%のマクロ使用シーンを代替
4. **型安全性** - コンパイル時チェック、ランタイム型オーバーヘッドなし
5. **IDE フレンドリー** - スマートヒント、明確なエラーメッセージ
6. **値依存型** - 型が値に依存可能、コンパイル時次元検証をサポート
7. **コンパイル時評価の安全性** - decreases 仕様によりコンパイル時評価の停止を保証

### 設計原則

- **コンパイル時決定**: ジェネリクス引数はコンパイル時に決定
- **単態化優先**: 具象コードを生成、仮想関数呼び出しを回避
- **制約駆動**: 型制約がインスタンス化をガイド
- **プラットフォーム最適化**: 特殊化によるプラットフォーム固有最適化をサポート
- **型ユニバースの統一**: 関数/型コンストラクタ/値依存型を Type2 層に統一
- **停止性保証**: 型位置での関数呼び出しは停止を証明しなければならない

## 提案

### 1. 基本ジェネリクス

#### 1.1 ジェネリック型引数

> **重要なルール**: ジェネリック型定義は**明示的に `: Type` を标注しなければならない**。さもないと HM 推論により関数として扱われる。
>
> | 書き方 | 意味 |
> |------|------|
> | `List: (T: Type) -> Type = {...}` | ✅ 型コンストラクタ |
> | `List = {...}` | ❌ HM が関数として推論し、型ではない |

```yaoxiang
# ジェネリック型定義(必ず : Type が必要)
Option: (T: Type) -> Type = {
    some: (T) -> Self,
    none: () -> Self
}

Result: (T: Type, E: Type) -> Type = {
    ok: (T) -> Self,
    err: (E) -> Self
}

List: (T: Type) -> Type = {
    data: Array(T),
    length: Int,
    push: (self: List(T), item: T) -> Void,   # self は単なる慣例名であり、キーワードではない
    get: (self: List(T), index: Int) -> Option(T),
}

# ジェネリック関数(: Type なし、HM が関数として推論)
map: (T: Type, R: Type) -> ((opt: Option(T), f: Fn(T) -> R) -> Option(R)) = {
    return match opt {
        some => Option.some(f(some)),
        none => Option.none(),
    }
}

# ジェネリクス制約(直接式、単一行は return 省略可)
clone: (T: Clone)(value: T) -> T = value.clone()

# 複数型引数
combine: (T: Type, U: Type) -> ((a: T, b: U) -> (T, U)) = (a, b)
```

### ジェネリック関数呼び出し構文

#### 1.1 統一シグネチャ構文

```yaoxiang
# ジェネリック関数は統一された (T: Type, R: Type) シグネチャ構文を使用
map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R)) = ...

# 複数型引数
combine: (T: Type, U: Type) -> ((a: T, b: U) -> (T, U)) = (a, b)
```

#### 1.2 Type 自己記述機構

`Type` は言語レベルの特別な存在であり、コンパイラは本質的にシグネチャ内の `Type` 位置を認識し、実引数の型から自動推論して填充する。

```yaoxiang
# コンパイラがジェネリクス引数を自動推論
numbers: List(Int) = List(Int)
#         ^^^^^^^^   ^^^^^^
#         型宣言     コンストラクタ呼び出し: Int が T を填充

# 関数呼び出しの推論
numbers: List(Int) = List(Int)
f: (x: Int) -> String = (x) => x.to_string()
strings: List(String) = map(numbers, f)
# コンパイラが推論: T=Int, R=String
```

#### 1.3 単態化

```yaoxiang
# ソースコード
map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R)) = {
    result: List(R) = List(R)()
    for x in list {
        result.push(f(x))
    }
    return result
}

# 使用箇所
int_list: List(Int) = List(Int)
doubled: List(Int) = map(int_list, (x: Int) => x * 2)  # map[Int, Int] をインスタンス化

string_list: List(String) = List(String)
uppercased: List(String) = map(string_list, (s: String) => s.to_uppercase())  # map[String, String] をインスタンス化

# コンパイル後(等価コード)
map_Int_Int: (list: List(Int), f: (Int) -> Int) -> List(Int) = {
    result: List(Int) = List(Int)
    for x in list {
        result.push(f(x))
    }
    return result
}

map_String_String: (list: List(String), f: (String) -> String) -> List(String) = {
    result: List(String) = List(String)
    for s in list {
        result.push(f(s))
    }
    return result
}
```

#### 1.4 明示的填充(推論失敗時)

```yaoxiang
# 推論可能な場合は Type 引数を省略
numbers: List(Int) = List(Int)
strings: List(String) = map(numbers, (x: Int) => x.to_string())

# 推論不可能な場合は明示的填充が必須
# map(numbers, (x) => x)  # ❌ Error: Cannot infer R

### 2. 型制約システム

#### 2.1 単一制約

```yaoxiang
# 基本トレイト定義(インターフェース型)
Clone: Type = {
    clone: (Self) -> Self,
}

Display: Type = {
    fmt: (Self, Formatter) -> Result,
}

Debug: Type = {
    fmt: (Self, Formatter) -> Result,
}

# 制約の使用: シグネチャ内で直接型制約を宣言
clone: (T: Clone) -> (value: T) -> T = value.clone()

debug_print: (T: Debug)(value: T) -> Void = {
    formatter = Formatter.new()
    value.fmt(formatter)
    print(formatter.to_string())
}
```

#### 2.2 多重制約

```yaoxiang
# 多重制約構文
combine: (T: Clone + Add)(a: T, b: T) -> T = {
    a.clone() + b
}

# ジェネリックコンテナのソート
sort: (T: Clone + PartialOrd)(list: List(T)) -> List(T) = {
    # ソートアルゴリズムの実装
    result: List(T) = list.clone()
    quicksort(&mut result)
    return result
}

# 関数型制約
map: (T: Type, R: FnMut(T))(array: Array(T), f: R) -> Array(R) = {
    result: Array(R) = Array()
    for item in array {
        result.push(f(item))
    }
    return result
}

# 使用
doubled: Array(Int) = map(Array(1, 2, 3), (x: Int) => x * 2)  # コンパイラが推論
```

#### 2.3 関数型制約

```yaoxiang
# 高階関数制約
call_twice: (T: Type, F: Fn() -> T)(f: F) -> (T, T) = (f(), f())

call_with_arg: (T: Type, U: Type, F: Fn(T) -> U)(arg: T, f: F) -> U = f(arg)

compose: (A: Type, B: Type, C: Type, F: Fn(A) -> B, G: Fn(B) -> C)(a: A, f: F, g: G) -> C = g(f(a))

# 使用例
result: Int = call_with_arg(42, (x: Int) => x * 2)  # result = 84
composed: String = compose(
    "hello",
    (s: String) => s.to_uppercase(),
    (s: String) => s + " WORLD"
)  # composed = "HELLO WORLD"
```

#### 2.4 組み込みマーカートレイト: Dup と Clone

**三種類のコピーセマンティクス**:

| 型 | 意味 | トリガー方式 | 適用シーン |
|------|------|----------|----------|
| **プリミティブ値コピー** | 代入時に自動で値コピー、二つの値は完全に独立 | 代入/引数渡しで自動 | Int, Float, Bool, Char |
| **Dup** | シャローコピー: ハンドル/トークンをコピー、底层データを共有 | 代入/引数渡しで自動 | `&T` トークン、`ref T`、String/Bytes |
| **Clone** | ディープコピー: 完全な独立した副本を作成 | `value.clone()` | Clone を実装する任意の型 |

**Dup のセマンティクス**: Dup を実装した型は、代入/引数渡し時に所有権を移動しない - コンパイラがハンドル/トークンをコピーし、複数の所有者が同じ底层データを指す。これは RFC-009 所有権モデルのデフォルト Move セマンティクスの補完である。

**Dup と Clone は直交する概念である**:

```
Dup = ハンドルをコピー、データを共有(変更は相互に影響)
Clone = データをコピー、副本は独立(変更は相互に影響しない)
```

**ルール**:

```
1. プリミティブ値型(Int, Float, Bool, Char) — コンパイラ組み込みの値コピー、Dup ではない
2. Dup  — 参照/トークン型および内部参照カウント型にのみ適用
3. Clone — 明示的なディープコピー、任意の型が実装可能
4. デフォルト Move — 他の型はデフォルト Move セマンティクスを維持
```

**どの型が Dup か**:

| 型 | Dup | 理由 |
|------|-----|------|
| `&T`(借用トークン) | ✅ | ゼロサイズトークン、トークンコピー = 同じデータへの複数の視点 |
| `ref T` | ✅ | Rc/Arc コピー = 参照カウント+1、ヒープデータを共有 |
| String, Bytes | ✅ | 内部参照カウント、ハンドル共有で底层 buffer を共有 |
| `&mut T`(可変トークン) | ❌ | 線形排他、コピー不可 |
| struct | 派生 | すべてのフィールドが Dup → struct Dup |
| enum | 派生 | すべてのバリアントのすべてのフィールドが Dup → enum Dup |
| tuple | 派生 | すべての要素が Dup → tuple Dup |
| Fn(クロージャ) | ❌ | キャプチャ環境が Dup でない可能性あり |
| `*T`(生ポインタ) | ❌ | unsafe、所有権システムに参加しない |

**Int/Float/Bool/Char は Dup ではない** - これらは値型であり、代入時にコンパイラが自動で値コピーする(二つの値は完全に独立)。これは「シャローコピー」ではなく、プリミティブに対するコンパイラ組み込み処理であり、Dup 型属性で表現すべきものではない。

```yaoxiang
# プリミティブ値型: コンパイラが自動で値コピー(Dup ではない)
x: Int = 42
y = x          # 値コピー、x と y は完全に独立
print(x)       # ✅

# Dup: シャローコピー、ハンドル共有でデータを共有
view: &Point = &point
view2 = view    # ✅ Dup: トークンコピー、両者は同じ point を指す
print(view.x)   # ✅

# Clone: 明示的なディープコピー、独立した副本を作成
backup = big_struct.clone()  # 明示的呼び出し

# ジェネリクス制約
dup_use: (T: Dup) -> T = x         # T: Dup → シャローコピー可能
clone_use: (T: Clone) -> T = x.clone()  # T: Clone → ディープコピー可能
```

> **注意**: `Send`/`Sync` はユーザ可視トレイトではない。タスク間安全保証は `ref` キーワードとコンパイラの完全自動処理によって行われる - `ref` は Rc または Arc を自動選択し、ユーザは Send/Sync を理解する必要がない。

### 3. 関連型

#### 3.1 関連型定義

```yaoxiang
# Iterator トレイト((Item: Type) -> Type 構文を使用)
Iterator: (Item: Type) -> Type = {
    next: (Self) -> Option(Item),
    has_next: (Self) -> Bool,
    collect: (T: Type)(Self) -> List(T),
}

# 使用
collect_all: (T: Type, I: Iterator(T))(iter: I) -> List(T) = {
    result: List(T) = List(T)
    while iter.has_next() {
        if let Some(item) = iter.next() {
            result.push(item)
        }
    }
    return result
}

# Array の Iterator 実装
# メソッド構文糖を使用: Array.Item, Array.next, Array.has_next
Array.has_next: (T: Type)(self: Array(T)) -> Bool = {
    return self.index < self.length
}

Array.next: (T: Type)(self: Array(T)) -> Option(T) = {
    if has_next(self) {
        item = self.data[self.index]
        self.index = self.index + 1
        return Option.some(item)
    } else {
        return Option.none()
    }
}

Array.Item: (T: Type)(arr: Array(T)) -> T = {
    return arr.data[0]
}
```

#### 3.2 ジェネリック関連型(GAT)

```yaoxiang
# より複雑な関連型
Producer: (Item: Type) -> Type = {
    Item: T,
    produce: (Self) -> Option(Item),
}

# 関連型はジェネリックにすることが可能
Container: (Item: Type) -> Type = {
    Item: T,
    IteratorType: Iterator(Item),  # 関連型もジェネリック
    iter: (Self) -> IteratorType,
}

# 使用
process_container: (T: Type, C: Container(T))(container: C) -> List(T) = {
    container.iter().collect()
}
```

### 4. コンパイル時ジェネリクス

#### 4.1 コンパイル時定数引数

**核心的設計**: ジェネリクスシグネチャ内の `Type` マーカーはコンパイル時型引数をマークし、`Int` などの値引数はジェネリクスコンテキストではデフォルトでコンパイル時に決定可能。`const` キーワードは不要。

```yaoxiang
# ════════════════════════════════════════════════════════
# コンパイル時定数引数: ジェネリクス内の Int はデフォルトでコンパイル時決定
# ════════════════════════════════════════════════════════

# コンパイル時階乗: N はコンパイル時既知のリテラルでなければならない
factorial: (N: Int) -> (n: N) -> Int = {
    return match n {
        0 => 1,
        _ => n * factorial(n - 1)
    }
}

# コンパイル時加算
add: (a: Int, b: Int) -> (a: a, b: b) -> Int = a + b

# ════════════════════════════════════════════════════════
# コンパイル時定数配列
# ════════════════════════════════════════════════════════
StaticArray: (T: Type, N: Int) -> Type = {
    data: Array(T, N),  # コンパイル時既知サイズの配列
    length: N,
}

# 使用方式
arr: StaticArray(Int, factorial(5))  # StaticArray(Int, 120)、コンパイラがコンパイル時に計算
```

#### 4.2 コンパイル時計算

```yaoxiang
# ════════════════════════════════════════════════════════
# コンパイル時計算の例
# ════════════════════════════════════════════════════════

# コンパイラがコンパイル時にリテラル型の関数呼び出しを計算
SIZE: Int = factorial(5)  # コンパイル時に 120

# 行列型の使用
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),
}

# コンパイル時次元検証
identity_matrix: (T: Add + Zero + One, N: Int)(size: N) -> Matrix(T, N, N) = {
    matrix: Matrix(T, N, N) = Matrix(T, N, N)()
    for i in 0..size {
        for j in 0..size {
            if i == j {
                matrix.data[i][j] = One::one()
            } else {
                matrix.data[i][j] = Zero::zero()
            }
        }
    }
    matrix
}

# 使用: コンパイル時計算、Matrix(Float, 3, 3) を生成
identity_3x3: Matrix(Float, 3, 3) = identity_matrix(Float, 3)(3)
```


### Never と Void: 型システムの ⊥ と ⊤

YaoXiang の型システムは Curry-Howard 同型において ⊥(偽/空型)と ⊤(真/Unit)の両方を備え、`Never` と `Void` という二つの組み込み型名で承载する:

**Never(⊥)** — 譲歩不可能な三つの内核的性質:

1. **ゼロコンストラクタ**: `Never` 型の値を生成するリテラルや式は一切存在しない。これはメタレベル性質であり、組み込みでなければならない。
2. **爆発原理**: `Never <: T` は任意の型 `T` に対して成立する。`Never` 値は任意の型として使用可能 - これは `assert(false)` の後のコードが型検査を通る理由である(決して実行されないが)。
3. **発散マーカー**: `f: (...) -> Never` は `f` が絶対に返らないことを示す。コンパイラはこれに基づいてデッドコード解析を行う。

`Never` は組み込み型名でありキーワードではないため、parser は無関心。空と型のリテラル構文は開放しない。

**Void(⊤、すなわち Unit)** — ちょうど一つの居住者(デフォルト void 値)を持ち、真命題「恒真」の载体である。`Void` はゼロフィールド積型の幺元であり、`Never` はゼロバリアント和型の幺元である - 二者は対偶。`x: Void = <デフォルト>` は合法であり、`x: Never = ...` は右辺に書けるものがない。

#### 4.3 コンパイル時検証(標準ライブラリ実装)

```yaoxiang
# ════════════════════════════════════════════════════════
# 標準ライブラリ実装: 条件型を利用
# ════════════════════════════════════════════════════════

# 標準ライブラリ定義
# IsTrue: 値宇宙から型宇宙への橋 - Bool 真値を型にマッピング
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      # ⊤、値を持つ、プログラムは続く
    false => Never,    # ⊥、値なし、発散
}

# Assert: コンパイル時精化型プリミティブ - Bool 命題の型レベル表述
Assert: (cond: Bool) -> Type = IsTrue(cond)
#
# cond が true  → Assert(true)  = Void    (恒真、消去)
# cond が false → Assert(false) = Never   (恒偽、コンパイルエラー/発散)
# cond が判定不能  → 証明パイプラインが dispatch モードで決定:
#                  CompileTime → Unknown、prove を要求
#                  Runtime     → check を挿入、Γ 仮定を注入

# 使用方式1: 型定義内で制約として
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    # コンパイル時チェック: N は 0 より大きくなければならない(Assert は型位置)
    length: Assert(N > 0),
}

# 使用方式2: 式内で使用
IntArray: (N: Int) -> Type = StaticArray(Int, N)
# 検証: IntArray(10) のサイズは sizeof(Int) * 10 と等しい
Assert(size_of(IntArray(10)) == sizeof(Int) * 10)
```

#### 4.4 コンパイル時ジェネリクス特殊化

```yaoxiang
# 小配列最適化: 関数オーバーロードを使用してコンパイル時ジェネリクス特殊化を実装

# 汎用実装
sum: (T: Type, N: Int) -> ((arr: Array(T, N)) -> T) = {
    result = Zero::zero()
    for item in arr.data {
        result = result + item
    }
    return result
}

# N=1 特殊化
sum: (T: Type) -> ((arr: Array(T, 1)) -> T) = arr.data[0]

# N=2 特殊化
sum: (T: Type) -> ((arr: Array(T, 2)) -> T) = arr.data[0] + arr.data[1]

# 小配列ループ展開(N <= 4)
sum: (T: Type, N: Int) -> ((arr: Array(T, N)) -> T) = {
    # コンパイラ最適化: ループを展開
    return arr.data[0] + arr.data[1] + arr.data[2] + arr.data[3]
}
```

### 5. 条件型

> **Curry-Howard 同型**: 条件型は Curry-Howard の視点から見ると論理の **case 分析** である。`Bool` 型は二つの可能な値(True/False)を持つ命題に対応し、`If` はその命題の真偽に応じて異なる結果を選択する - これはまさに論理における case 析取である。`match C { True => T, False => E }` は実際には「命題 C が True のとき結論は T、C が False のとき結論は E」を表現している。

#### 5.1 If 条件型

```yaoxiang
# 型レベル If
If: (C: Bool, T: Type, E: Type) -> Type = match C {
    True => T,
    False => E,
}

# 例: コンパイル時分岐
NonEmpty: (T: Type) -> Type = If(T != Void, T, Never)

Optional: (T: Type) -> Type = If(T != Void, T, Void)

# コンパイル時検証(§4.3 の Assert 定義に統一)
# Assert: (cond: Bool) -> Type = IsTrue(cond)

# 使用
# 型計算: If(True, Int, String) => Int
# 型計算: If(False, Int, String) => String
```

#### 5.2 型族

> **Curry-Howard 同型**: 型族は「命題即型」の最も直接的な現れである。`Add: (A: Type, B: Type) -> Type` は「型レベルで加算関数を書いた」のではなく、自然数加算に関する**命題を構成している**。`(Zero, B) => B` は「命題 Add(Zero, B) は B と等価である」と述べ、`(Succ(A'), B) => Succ(Add(A', B))` は「Add(A', B) が成立するなら Add(Succ(A'), B) も成立する」と述べる。これは Peano 公理における加算定義そのもの。型検査器がこの match 式の通過を検証することは、この定義の論理的一貫性の検証と等価である。

```yaoxiang
# コンパイル時型変換
AsString: (T: Type) -> Type = match T {
    Int => String,
    Float => String,
    Bool => String,
    _ => String,  # デフォルト
}

# 型レベル計算
Length: (T: Type) -> Type = match T.length {
    0 => Zero,
    1 => Succ(Zero),
    2 => Succ(Succ(Zero)),
    _ => TooLong,
}

# 型レベル加算(Curry-Howard: case 分析 + 再帰呼び出し、完全な帰納法には停止性検査が必要)
Add: (A: Type, B: Type) -> Type = match (A, B) {
    (Zero, B) => B,
    (Succ(A'), B) => Succ(Add(A', B)),
}

# 例: コンパイル時計算 2 + 3
Two: Type = Succ(Succ(Zero))
Three: Type = Succ(Succ(Succ(Zero)))
Five: Type = Add[Two, Three]  # Succ(Succ(Succ(Succ(Succ(Zero)))))
```

### 6. 関数オーバーロード特殊化

#### 6.1 基本特殊化

```yaoxiang
# 基本特殊化: 関数オーバーロードを使用(コンパイラが自動選択)
sum: (arr: Array(Int)) -> Int = {
    # より効率的なコードにコンパイル
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Array(Float)) -> Float = {
    # SIMD 命令を使用
    return simd_sum_float(arr.data, arr.length)
}

# 汎用実装
sum: (T: Type) -> ((arr: Array(T)) -> T) = {
    result = Zero::zero()
    for item in arr {
        result = result + item
    }
    return result
}
```

#### 6.2 条件特殊化

```yaoxiang
# RFC-010 構文に完全準拠した特殊化方式: 関数オーバーロード

# 具象型特殊化
sum: (arr: Array(Int)) -> Int = {
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Array(Float)) -> Float = {
    return simd_sum_float(arr.data, arr.length)
}

# ジェネリック実装(コンパイラが自動的に最適を選択)
sum: (T: Type) -> ((arr: Array(T)) -> T) = {
    result = Zero::zero()
    for item in arr {
        result = result + item
    }
    return result
}

# 使用時は完全に透過的
int_arr = Array(Int)(1, 2, 3)
float_arr = Array(Float)(1.0, 2.0, 3.0)

# コンパイラが最適特殊化を自動選択
sum(int_arr)     # sum: (Array(Int)) -> Int を選択
sum(float_arr)    # sum: (Array(Float)) -> Float を選択
```

#### 6.3 関数オーバーロードとインラインの完全結合

**重要な特性**: 関数オーバーロードとインライン最適化は自然に結合し、ゼロコスト抽象を実現する。

```yaoxiang
# ======== ソースコード ========
sum: (arr: Array(Int)) -> Int = {
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Array(Float)) -> Float = {
    return simd_sum_float(arr.data, arr.length)
}

sum: (T: Type) -> ((arr: Array(T)) -> T) = {
    result = Zero::zero()
    for item in arr {
        result = result + item
    }
    return result
}

# 使用
int_arr = Array(Int)(1, 2, 3, 4, 5)
result = sum(int_arr)

# ======== コンパイル後(等価コード)=======
# コンパイラが最適特殊化を自動選択し、インライン化
result = native_sum_int(int_arr.data, int_arr.length)

# 手書き最適化コードと完全等価、関数呼び出しオーバーヘッドなし!
```

**核心的優位性**:

1. **コンパイラの智能選択**
   ```yaoxiang
   sum(int_arr)      # sum: (Array(Int)) -> Int を自動選択
   sum(float_arr)    # sum: (Array(Float)) -> Float を自動選択
   sum(custom_arr)  # sum: (T: Type) -> ((arr: Array(T)) -> T) を自動選択
   ```

2. **インライン最適化**
   - 小さい関数は呼び出し点に自動インライン化
   - 関数呼び出しオーバーヘッドゼロ
   - 手書き最適化コードと完全等価

3. **型安全性**
   - コンパイル時型チェック
   - ランタイムオーバーヘッドゼロ
   - 仮想関数テーブル不要

4. **RFC-010 との完全適合**
   ```yaoxiang
   # 統一構文を完全使用
   name: type = value
   # impl、where などの新しいキーワード不要
   ```

**実際の応用例**:

```yaoxiang
# 性能重視の数値計算
fibonacci: (n: Int) -> Int = {
    if n <= 1 { return n }
    return fibonacci(n - 1) + fibonacci(n - 2)
}

fibonacci: (n: Float) -> Float = {
    # Binet の公式を使用
    phi = (1.0 + 5.0.sqrt()) / 2.0
    return (phi.pow(n) - (-phi).pow(-n)) / 5.0.sqrt()
}

# コンパイラが自動選択しインライン化
fibonacci(10)      # Int バージョンを選択、完全にインライン化
fibonacci(10.5)    # Float バージョンを選択、Binet の公式を使用
```

**これは何を意味するか?**

- ✅ **ジェネリクス特殊化** → 関数オーバーロードで自然に解決
- ✅ **性能最適化** → インライン化が自動完了
- ✅ **コード再利用** → 一つの関数名で複数の実装
- ✅ **ゼロコスト抽象** → コンパイル時多態、ランタイムオーバーヘッドゼロ
- ✅ **新しいキーワード不要** → RFC-010 統一構文に完全適合
```

### 7. デッドコード除去機構

#### 7.1 インスタンス化グラフ解析

```rust
// コンパイラ内部: ジェネリクスインスタンス化依存グラフを構築
struct InstantiationGraph {
    // ノード: ジェネリクスインスタンス化
    nodes: HashMap<InstanceKey, InstanceNode>,

    // エッジ: 使用関係
    edges: HashMap<InstanceKey, Vec<InstanceKey>>,
}

struct InstanceKey {
    generic: FunctionId,  // ジェネリクス関数 ID
    type_args: Vec<TypeId>,  // 型引数
    const_args: Vec<ConstId>,  // Const 引数
}

// アルゴリズム: 到達可能性解析
fn eliminate_dead_instantiations(graph: &InstantiationGraph) {
    let mut reachable = HashSet::new();

    // エントリポイントから開始(main、エクスポート関数など)
    let entry_points = find_entry_points();
    for entry in entry_points {
        dfs_visit(entry, &graph, &mut reachable);
    }

    // 訪問されていないインスタンス化がデッドコード
    for node in &graph.nodes {
        if !reachable.contains(node.key) {
            eliminate(node);
        }
    }
}
```

#### 7.2 使用点解析

```yaoxiang
# ソースコード解析
map: (T: Type, R: Type)(list: List(T), f: Fn(T) -> R) -> List(R) = ...

# 使用点 1: map(Int, Int) をインスタンス化
int_list = List(1, 2, 3)
doubled = map(int_list, (x) => x * 2)  # map[Int, Int] が必要

# 使用点 2: map(String, String) をインスタンス化
string_list = List("a", "b", "c")
uppercased = map(string_list, (s) => s.to_uppercase())  # map[String, String] が必要

# 未使用: map[Float, Float] など
# これらのジェネリクスインスタンスは生成されない

# コンパイル後は使用されるインスタンスのみを含む
map_Int_Int: (list: List(Int), f: Fn(Int) -> Int) -> List(Int) = ...
map_String_String: (list: List(String), f: Fn(String) -> String) -> List(String) = ...
```

#### 7.3 コンパイル時ジェネリクス DCE

```yaoxiang
# コンパイル時解析: コンパイル時ジェネリクスの使用状況
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
}

# 実際の使用状況
arr_10_int = Array(Int, 10)(1, 2, 3, 4, 5, 6, 7, 8, 9, 10)
arr_100_int = Array(Int, 100)(...)

# コンパイル後は使用される Size のみを生成
Array_Int_10: (Array(Int, 10)) = ...
Array_Int_100: (Array(Int, 100)) = ...

# 未使用の Size は生成されない
# Array(Int, 50) は生成されない
```

#### 7.4 モジュール間 DCE

```yaoxiang
# モジュール A
# A.yx
pub map: (T: Type, R: Type)(list: List(T), f: Fn(T) -> R) -> List(R) = ...

# モジュール B
# B.yx
use A.{map}
int_list = List(1, 2, 3)
doubled = map(int_list, (x) => x * 2)  # map(Int, Int) をインスタンス化

# モジュール C
# C.yx
use A.{map}
string_list = List("a", "b", "c")
uppercased = map(string_list, (s) => s.to_uppercase())  # map(String, String) をインスタンス化

# コンパイル解析:
# - モジュール B は map[Int, Int] を使用
# - モジュール C は map[String, String] を使用
# - コンパイル後のバイナリはこれら二つのインスタンスのみを含む
```

#### 7.5 LLVM レベル DCE

```rust
// コンパイルパイプライン
fn optimize_ir(ir: &mut IR) {
    // 1. 単態化(YaoXiang コンパイラ)
    ir.monomorphize();

    // 2. インライン最適化
    ir.inline_small_functions();

    // 3. 定数伝播
    ir.constant_propagation();

    // 4. LLVM IR を生成
    let llvm_ir = ir.to_llvm();

    // 5. LLVM 最適化パス
    llvm_ir.add_pass(Passes::DEAD_CODE_ELIMINATION);
    llvm_ir.add_pass(Passes::INLINE_FUNCTION);
    llvm_ir.add_pass(Passes::GLOBAL_DCE);
    llvm_ir.add_pass(Passes::MERGE_FUNC);

    // 6. 最適化パスを実行
    llvm_ir.run_optimization_passes();
}
```

### 8. マクロ代替戦略

#### 8.1 コード生成の代替

```yaoxiang
# ❌ マクロ方式: コード生成
macro_rules! impl_debug {
    ($($t:ty),*) => {
        $(impl Debug for $t {
            fn fmt(&self, f: &mut Formatter) -> Result {
                write!(f, "{:?}", self)
            }
        })*
    };
}

# ✅ ジェネリクス方式: 自動派生
# 関数オーバーロード方式で自動派生を使用
debug_fmt: (T: fields...) -> ((self: Point(T)) -> String) = {
    return "Point { x: " + self.x.to_string() + ", y: " + self.y.to_string() + " }"
}

# 使用
p = Point { x: 1, y: 2 }
p.debug_fmt(&formatter)  # 自動生成呼び出し
```

#### 8.2 DSL の代替

```yaoxiang
# ❌ マクロ方式: HTML DSL
html! {
    <div class="container">
        <h1> { title } </h1>
        <ul>
            { for item in items {
                <li> { item } </li>
            }}
        </ul>
    </div>
}

# ✅ ジェネリクス方式: 型安全ビルダー
Element: Type = {
    tag: String,
    attrs: HashMap(String, String),
    children: List(Element),
    text: Option(String),
}

create_element: (tag: String) -> Element = {
    return Element(tag, HashMap::new(), List::new(), None)
}

with_class: [E: Element](elem: E, class: String) -> E = {
    elem.attrs.insert("class", class)
    return elem
}

with_text: [E: Element](elem: E, text: String) -> E = {
    return E { text: Some(text), ..elem }
}

# DOM を構築
container = create_element("div")
    |> with_class("container")
    |> with_children(List::new())

title_elem = create_element("h1") |> with_text(title)
items_li = items.map((item) =>
    create_element("li") |> with_text(item)
)
root = container |> with_children(List::new() + [title_elem, ul_elem])
```

#### 8.3 型レベルプログラミングの代替

```yaoxiang
# ❌ マクロ方式: 型レベル計算
macro_rules! add_types {
    ($a:ty, $b:ty) => {
        ($a, $b)
    };
}

# ✅ ジェネリクス方式: 条件型
Add: (A: Type, B: Type) -> Type = match (A, B) {
    (Int, Int) => Int,
    (Float, Float) => Float,
    (Int, Float) => Float,
    (Float, Int) => Float,
    _ => TypeError,
}

# コンパイル時検証
AssertAddable: (A: Type, B: Type) -> Type = If(Add(A, B) != TypeError, (A, B), compile_error("Cannot add"))

# 使用
result_type = Add[Int, Float]  # Float として推論
```

### 9. 例

#### 9.1 完全なジェネリックコンテナ例

```yaoxiang
# ======== 1. ジェネリックコンテナ定義 ========
# (T: Type) -> Type 構文を使用
Result: (T: Type, E: Type) -> Type = {
    ok: (T) -> Self,
    err: (E) -> Self,
}

Option: (T: Type) -> Type = {
    some: (T) -> Self,
    none: () -> Self,
}

List: (T: Type) -> Type = {
    data: Array(T),
    length: Int,

    # ジェネリックメソッド(T は外側の List(T) から自動的にスコープに引き込まれる)
    push: (self: List(T), item: T) -> Void,
    pop: (self: List(T)) -> Option(T),
    map: (R: Type) -> ((self: List(T), f: (T) -> R) -> List(R)),
    filter: (self: List(T), predicate: (T) -> Bool) -> List(T),
    fold: (U: Type) -> ((self: List(T), initial: U, f: (U, T) -> U) -> U),
}

# ======== 2. ジェネリックメソッドの実装 ========
# 関数は List 名前空間の下で定義される(List. プレフィックス = 名前空間帰属)
# list.push(item) のような . 呼び出し構文を有効にするには、明示的バインディングが必要: List.push = push[0]
# self は単なる慣例引数名であり、コンパイラは名前ではなく型を見る

List.push: (T: Type) -> ((self: List(T), item: T) -> Void) = {
    if self.length >= self.data.length {
        # 拡張
        new_data = Array(T)(self.data.length * 2)
        for i in 0..self.length {
            new_data[i] = self.data[i]
        }
        self.data = new_data
    }
    self.data[self.length] = item
    self.length = self.length + 1
}

List.pop: (T: Type) -> ((self: List(T)) -> Option(T)) = {
    if self.length > 0 {
        self.length = self.length - 1
        return Option.some(self.data[self.length])
    } else {
        return Option.none()
    }
}

List.map: (T: Type, R: Type) -> ((self: List(T), f: (T) -> R) -> List(R)) = {
    result = List(R)()
    for i in 0..self.length {
        result.push(f(self.data[i]))
    }
    return result
}

List.filter: (T: Type) -> ((self: List(T), predicate: (T) -> Bool) -> List(T)) = {
    result = List(T)()
    for i in 0..self.length {
        if predicate(self.data[i]) {
            result.push(self.data[i])
        }
    }
    return result
}

List.fold: (T: Type, U: Type) -> ((self: List(T), initial: U, f: (U, T) -> U) -> U) = {
    result = initial
    for i in 0..self.length {
        result = f(result, self.data[i])
    }
    return result
}

# ======== 3. 型制約の使用 ========
# List の Clone を実装
List.clone: (T: Clone) -> ((self: List(T)) -> List(T)) = {
    result = List(T)()
    for i in 0..self.length {
        result.push(self.data[i].clone())
    }
    return result
}

# ======== 4. 使用例 ========
# ジェネリック List を作成
numbers = List(Int)()
numbers.push(1)
numbers.push(2)
numbers.push(3)

# ジェネリックメソッドを使用
doubled = numbers.map((x) => x * 2)
evens = numbers.filter((x) => x % 2 == 0)

# fold を使用して計算
sum = numbers.fold(0, (acc, x) => acc + x)  # sum = 6

# ジェネリクス組み合わせ
sum_of_evens = numbers
    .filter((x) => x % 2 == 0)
    .map((x) => x * 2)
    .fold(0, (acc, x) => acc + x)  # sum_of_evens = 8
```

#### 9.2 ジェネリクスアルゴリズム例

```yaoxiang
# ======== 1. ジェネリックソートアルゴリズム ========
Comparator: (T: Type) -> Type = {
    compare: (T, T) -> Int,  # a < b なら -1、a == b なら 0、a > b なら 1
}

# ジェネリック quicksort
quicksort: (T: Clone) -> ((array: Array(T), cmp: Comparator(T)) -> Array(T)) = {
    if array.length <= 1 {
        return array.clone()
    }

    pivot = array[array.length / 2]
    left = Array(T)()
    right = Array(T)()

    for i in 0..array.length {
        if i == array.length / 2 {
            continue
        }
        item = array[i]
        comparison = cmp.compare(item, pivot)
        if comparison < 0 {
            left.push(item)
        } else {
            right.push(item)
        }
    }

    sorted_left = quicksort(left, cmp)
    sorted_right = quicksort(right, cmp)

    result = sorted_left.clone()
    result.push(pivot)
    result.extend(sorted_right)
    return result
}

# ======== 2. IntComparator 実装 ========
# 関数オーバーロードで実装
compare: (a: Int, b: Int) -> Int = {
    if a < b {
        return -1
    } else if a > b {
        return 1
    } else {
        return 0
    }
}

# ======== 3. 使用例 ========
# Int 配列をソート
numbers = Array(Int)(3, 1, 4, 1, 5, 9, 2, 6)
sorted = quicksort(numbers, Comparator(Int)())

# String 配列をソート(StringComparator が必要)
strings = Array(String)("hello", "world", "foo", "bar")
sorted_strings = quicksort(strings, Comparator(String)())
```

#### 9.3 コンパイル時ジェネリクス例

```yaoxiang
# ======== 1. コンパイル時行列型 ========
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),

    # コンパイル時次元検証: Assert 標準ライブラリ型を利用
    _assert: Assert[Rows > 0],  # Rows > 0、そうでなければコンパイルエラー
    _assert: Assert[Cols > 0],  # Cols > 0、そうでなければコンパイルエラー

    # 行列演算
    multiply: (M: Int) -> ((self: Matrix(T, Rows, Cols), other: Matrix(T, Cols, M)) -> Matrix(T, Rows, M)) = {
        result = Matrix(T, Rows, M)()
        for i in 0..Rows {
            for j in 0..M {
                sum = Zero::zero()
                for k in 0..Cols {
                    sum = sum + self.data[i][k] * other.data[k][j]
                }
                result.data[i][j] = sum
            }
        }
        return result
    }
}

# ======== 2. コンパイル時行列作成 ========
identity: (T: Add + Multiply + One, N: Int) -> ((size: N) -> Matrix(T, N, N)) = {
    matrix = Matrix(T, N, N)()
    for i in 0..N {
        for j in 0..N {
            if i == j {
                matrix.data[i][j] = One::one()
            } else {
                matrix.data[i][j] = Zero::zero()
            }
        }
    }
    return matrix
}

# ======== 3. 使用例 ========
# コンパイル時既知サイズの行列を作成
# 2x3 行列
matrix_2x3 = Matrix(Float, 2, 3)()
matrix_2x3.data[0][0] = 1.0
matrix_2x3.data[0][1] = 2.0
matrix_2x3.data[0][2] = 3.0
matrix_2x3.data[1][0] = 4.0
matrix_2x3.data[1][1] = 5.0
matrix_2x3.data[1][2] = 6.0

# 3x2 行列
matrix_3x2 = Matrix(Float, 3, 2)()
matrix_3x2.data[0][0] = 7.0
matrix_3x2.data[0][1] = 8.0
matrix_3x2.data[1][0] = 9.0
matrix_3x2.data[1][1] = 10.0
matrix_3x2.data[2][0] = 11.0
matrix_3x2.data[2][1] = 12.0

# 行列乗算: 2x3 * 3x2 = 2x2
result = matrix_2x3.multiply(matrix_3x2)

# コンパイル時検証: result の型は Matrix(Float, 2, 2)
# 2x2 単位行列
identity_3x3 = identity(Float, 3)()

# 次元不一致: コンパイルエラー
# bad_multiply = matrix_2x3.multiply(identity_3x3)  # コンパイルエラー: 3x3 != 2x3
```

## トレードオフ

### 優位性

1. **ゼロコスト抽象**
   - コンパイル時単態化、ランタイムオーバーヘッドなし
   - 仮想関数不要、RTTI 不要

2. **デッドコード除去**
   - コンパイル時解析、使用されるジェネリクスのみインスタンス化
   - コード膨張を抑制可能

3. **マクロ代替**
   - 型安全なコード生成
   - IDE フレンドリー、明確なエラーメッセージ

4. **コンパイル時計算**
   - コンパイル時ジェネリクスがコンパイル時計算をサポート
   - 次元検証などの特性
   - `const` キーワード不要、純粋な型制約

### 欠点

1. **コンパイル時間**
   - ジェネリクスインスタンス化がコンパイル時間を増加
   - 制約解決が遅くなる可能性

2. **メモリ使用量**
   - コンパイラのメモリ使用量が増加
   - キャッシュ機構にメモリが必要

3. **実装複雑度**
   - 制約ソルバが複雑
   - 型レベル計算エンジンが複雑

4. **エラー診断**
   - ジェネリクスエラーが複雑になる可能性
   - 明確なエラーヒントが必要

### 緩和策

1. **キャッシュ戦略**
   - インスタンス化結果のキャッシュ
   - LRU キャッシュによるメモリ制限

2. **インクリメンタルコンパイル**
   - コンパイル結果のキャッシュ
   - インクリメンタルインスタンス化

3. **エラーヒント**
   - 明確なエラーメッセージ
   - ジェネリクス引数推論のヒント

4. **並列コンパイル**
   - ジェネリクスの並列インスタンス化
   - マルチスレッド制約解決

## 代替案

| 案 | なぜ選択しないか |
|------|--------------|
| 基本ジェネリクスのみ | 複雑なマクロを代替できない |
| 純粋マクロシステム | 型安全性なし、エラーメッセージが悪い |
| 依存制約のみ | 柔軟性不足 |
| ランタイムジェネリクス | パフォーマンスオーバーヘッドあり |

### リスク

| リスク | 影響 | 緩和策 |
|------|------|------|
| 制約解決の複雑度 | コンパイル時間が長すぎる | インクリメンタル解決 + キャッシュ |
| コード膨張 | バイナリファイルが大きすぎる | DCE + 閾値制御 |
| 実装複雑度 | 開発期間が長くなる | 段階的実装 |
| エラー診断 | ユーザー体験が悪い | 詳細なエラーメッセージ |

## オープンイシュー

### 決議待ちの問題

| 議題 | 説明 | 状態 |
|------|------|------|
| インスタンス化戦略 | Eager vs Lazy vs Threshold | 議論待ち |
| キャッシュサイズ | LRU キャッシュ容量設定 | 議論待ち |
| エラー診断 | ジェネリクスエラーメッセージの詳細度 | 議論待ち |

### 今後の最適化

| 最適化項目 | 価値 | 実装難度 |
|--------|------|------|
| インスタンス化グラフ解析 | 高 | 中 |
| 型レベルプログラミング DSL | 中 | 高 |
| ジェネリクスパフォーマンスベンチマーク | 中 | 低 |

## 付録

### 構文 BNF

```bnf
# ジェネリクス引数は統一 () 構文を使用し、関数型の一部
# 例: map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R))

# 型制約(ジェネリクス引数内)
type_bound ::= identifier
             | identifier '+' identifier ('+' identifier)*

# 引数宣言(型 + 名前)
parameter ::= identifier ':' type

parameters ::= parameter (',' parameter)*

# 関数宣言: name: type = expression
# ジェネリクス引数は関数型の最初の引数グループ: (T: Type) -> ((params) -> return)
function ::= identifier ':' type '=' (expression | block)

# メソッド宣言: Type.method: type = expression
method ::= identifier '.' identifier ':' type '=' (expression | block)

# 型定義(統一 Binding 構文)
# ジェネリック型例: List: (T: Type) -> Type = { ... }
generic_type ::= identifier ':' type '=' type_expression

# ジェネリクス引数の Type はコンパイラにより実引数型から自動填充
# 例: map(numbers, f)、T は numbers: List(Int) から抽出、R は f: (Int) -> String から抽出
```

## ライフサイクルと帰属

```
┌─────────────┐
│   ドラフト    │  ← 現在の状態
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  レビュー中  │  ← オープンなコミュニティ議論とフィードバック
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  承認済み    │    │  拒否済み    │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   accepted/ │    │    rfc/     │
│ (正式設計)  │    │ (原位置保持) │
└─────────────┘    └─────────────┘
```

---

## 参考文献

### YaoXiang 公式ドキュメント

- [RFC-010: 統一型構文](./010-unified-type-syntax.md)
- [RFC-009: 所有権モデル](./accepted/009-ownership-model.md)
- [RFC-001: spawn モデル](./accepted/001-concurrent-model-error-handling.md)
- [RFC-008: ランタイムモデル](./accepted/008-runtime-concurrency-model.md)
- [言語仕様](../language-spec.md)
- [YaoXiang ガイド](../guides/YaoXiang-book.md)

### 外部参考

- [Rust ジェネリクスシステム](https://doc.rust-lang.org/book/ch10-01-syntax.html)
- [C++ テンプレート特殊化](https://en.cppreference.com/w/cpp/language/template_specialization)
- [Haskell 型クラス](https://www.haskell.org/tutorial/classes.html)
- [Swift ジェネリクス](https://docs.swift.org/swift-book/LanguageGuide/Generics.html)
- [単態化最適化](https://llvm.org/docs/Monomorphization.html)
- [デッドコード除去](https://en.wikipedia.org/wiki/Dead_code_elimination)
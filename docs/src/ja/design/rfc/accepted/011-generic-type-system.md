```markdown
---
title: "RFC-011: ジェネリクスシステム設計 - ゼロコスト抽象とマクロ代替"
status: "承認済み"
author: "晨煦"
updated: "2026-07-15（型本体コードブロック + コンパイル時仕様 + エフェクトシードが実装済み）"
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

# RFC-011: ジェネリクスシステム設計 - ゼロコスト抽象とマクロ代替

## 概要

本文書はYaoXiang言語の**ジェネリクスシステム設計**を定義する。強力なジェネリクス機能によりゼロコスト抽象を実現し、コンパイル時最適化を通じてマクロへの依存を削減し、デッドコード除去機構を提供する。

**核心設計**：
- **統一シグネチャ構文**：`(T: Type, R: Type) -> ...` ジェネリックパラメータと通常パラメータの統一
- **Type 自己記述機構**：`Type` は言語レベルの特別な存在であり、シグネチャ中の `Type` 位置は自動的に推論・補完される
- **型制約**：`T: Dup + Add` のような複数制約、関数型制約
- **関連型**：`Iterator: (Item: Type) -> Type = { next: () -> Option(Item), has_next: () -> Bool }`
- **コンパイル時ジェネリクス**：`N: Int` ジェネリック値パラメータ、コンパイル時定数インスタンス化
- **条件型**：`If: (C: Bool, T: Type, E: Type) -> Type` 型レベル計算、型族

**価値**：
- ゼロコスト抽象：コンパイル時単態化、ランタイムオーバーヘッドなし
- デッドコード除去：インスタンス化グラフ解析 + LLVM最適化
- マクロ代替：ジェネリクスがマクロ使用シーンの90%を代替
- 型安全性：コンパイル時検査、IDEフレンドリー
- **明示は暗黙に優る**：`Type` 自己記述、コンパイラによる自動推論

## 参考ドキュメント

本文書の設計は以下に基づく：

| ドキュメント | 関係 | 説明 |
|------|------|------|
| [RFC-010: 統一型構文](./010-unified-type-syntax.md) | **構文基盤** | ジェネリクス構文と統一 `name: type = value` モデルの統合 |
| [RFC-010: 統一型構文](./010-unified-type-syntax.md) | **呼び出し構文** | 第6節：ジェネリック呼び出し構文——統一 `()` 適用、`[]` 完全削除 |
| [RFC-009: 所有権モデル](./accepted/009-ownership-model.md) | **型システム** | Moveセマンティクスとジェネリクスの自然な統合 |
| [RFC-024: spawnベースの並行ランタイムセマンティクス](./024-concurrency-model.md) | **実行モデル** | DAG解析とジェネリック型検査 |
| [RFC-008: ランタイムモデル](./accepted/008-runtime-concurrency-model.md) | **コンパイラアーキテクチャ** | ジェネリック単態化とコンパイル時最適化戦略 |
| [型ユニバース思想](../reference/plan/ongoing/类型宇宙思想.md) | **理論的核** | 型ユニバース階層モデルと値依存型の設計 |
| [RFC-027: コンパイル時述語と統一静的検証](./027-compile-time-evaluation-types.md) | **終了性検査** | decreases仕様とコンパイル時評価の安全保証 |

## 型ユニバース思想と値依存型

YaoXiangのジェネリクスシステムは**型ユニバース思想**の上に構築されており、このメンタルモデルは言語のすべての概念を階層構造として統一する。核心的な革新は、**値依存型**をType2層の一級市民として昇格させることにある。

### 値依存型とは何か？

**値依存型**は、一つ以上の**値**（他の型だけでなく）に依存する型である。これらの値はコンパイル時に評価され、コンパイル段階で型安全保証を提供する。

```yaoxiang
# 传统泛型：类型参数
List: (T: Type) -> Type

# 值依赖类型：值参数
Vec: (n: Int) -> Type  # 向量类型依赖于长度值 n
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type  # 矩阵类型依赖于行数和列数
```

### 値依存型の核心的な優位性

従来のジェネリクスと比較して、YaoXiangの値依存型には以下の核心的な優位性がある：

| 特性 | 従来ジェネリクス (C++/Rust) | YaoXiang 値依存型 |
|------|-------------------|---------------------|
| 型が依存する値 | 型パラメータのみ | 任意の値（関数呼び出し結果を含む）に依存可能 |
| コンパイル時評価 | C++テンプレート手動特殊化、Rustは不可 | 自動コンパイル時評価、停止保証 |
| 型レベル計算 | テンプレートメタプログラミング（複雑/危険） | 統一された型レベル計算エンジン |
| 型安全性 | C++はなし、Rustは制限あり | 完全な型安全性、コンパイル時検査 |
| 次元検証 | ランタイム検査または手動特殊化 | コンパイル時次元検証、ランタイムオーバーヘッドなし |

### 型ユニバース階層と値依存型

型ユニバース思想は言語概念を意味的役割によって異なる階層に分割する。値依存型は **Type2層** に位置する：

| 階層 | 役割 | 例 |
|------|------|------|
| Type-1 | 値 | `42`, `factorial(5)`, 関数そのもの |
| Type0 | メタ型キーワード | `Type` |
| Type1 | 具象型 | `Int`, `String`, `Vec(3)` |
| **Type2** | **関数/型コンストラクタ/値依存型** | `add: (Int, Int) -> Int`, `Vec: (n: Int) -> Type`, `Matrix: (T: Type, Rows: Int, Cols: Int) -> Type` |

**重要な設計**：Type2層の関数、型コンストラクタ、値依存型は**統一構文**であり、すべて `(params) -> result` の形式である：
- 通常関数：`(Int, Int) -> Int` → 戻り値は値
- 型コンストラクタ：`(T: Type) -> Type` → 戻り値は型
- 値依存型：`(n: Int) -> Type` → 戻り値は型だが、値パラメータに依存する

> **カリー・ハワード同型対応**：この統一は偶然ではない。カリー・ハワード同型対応は「型は命題、プログラムは証明」であることを示す——関数型 `A → B` は論理的蕴含「A ならば B」に対応し、ジェネリクス `(T: Type) -> Type` は全称量化「すべての型Tについて」に対応する。YaoXiangが関数、型コンストラクタ、値依存型をType2層に統一することは、本質的に「証明」と「計算」を同一の概念——**構成的証明**——に統一することである。これはカリー・ハワード同型対応の言語設計における直接的な具現化であり、一つの形式（`(params) -> result`）が同時に論理的命題と計算過程を担う。

### コンパイル時決定性保証

YaoXiangの型ユニバース思想は要求する：**Type階層のすべてはコンパイル時に決定される**。

```yaoxiang
# 编译期维度验证示例
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),
    # 编译期检查：维度必须为正
    _assert: Assert[Rows > 0],
    _assert: Assert[Cols > 0],
}

# 创建 3x3 单位矩阵 - 编译期完成
identity: (T: Add + Zero + One, N: Int) -> ((size: N) -> Matrix(T, N, N)) = {
    matrix = Matrix(T, N, N)()
    # ...
}

# 编译期计算：factorial(3) = 6，向量大小在编译期确定
vec: Vec(factorial(3)) = Vec(6)()
```

コンパイラは自動的に：
1. 型位置での関数呼び出しを検出する
2. 関数が `decreases` 仕様でマークされているかを検証する（後述の終了性検査機構を参照）
3. コンパイル時に評価を実行する
4. 結果を生成された型に埋め込む

### 値依存型の応用シーン

#### コンパイル時次元検証
```yaoxiang
# 矩阵乘法：编译期验证维度匹配
multiply: (T: Add + Multiply + Zero,
           Rows: Int, Cols: Int, M: Int) -> ((
    a: Matrix(T, Rows, Cols),
    b: Matrix(T, Cols, M)
) -> Matrix(T, Rows, M)) = {
    # 编译期检查：a.Cols == b.Rows，否则编译错误
    result = Matrix(T, Rows, M)()
    # ...
}

# 错误在编译期捕获：
# multiply(matrix_2x3, matrix_4x2)  # 编译错误：2 != 4
```

#### 型安全な配列サイズ
```yaoxiang
# 数组大小是编译期常量
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    length: N,
}

# N 是编译期常量，可以用于类型级计算
first_three: Array(Int, 3) = Array(Int, 3)(1, 2, 3)
# first_three.length == 3（编译期已知）
```

#### 条件型
```yaoxiang
# 类型级If
If: (C: Bool, T: Type, E: Type) -> Type = match C {
    True => T,
    False => E,
}

# 类型族
AsString: (T: Type) -> Type = match T {
    Int => String,
    Float => String,
    Bool => String,
    _ => String,
}
```

#### ジェネリック関数
```yaoxiang
# map: 泛型函数，类型参数 T, R 在编译期确定
map: (T: Type, R: Type) -> (
    (list: List(T), f: (x: T) -> R) -> List(R)
) = (list, f) => {
    result = List(R)()
    for x in list {
        result.push(f(x))
    }
    return result
}

# 使用时完全透明，类型自动推导
numbers = List(1, 2, 3)
doubled = map(numbers, (x) => x * 2)  # 推导为 map[Int, Int]
```

### 他の言語との比較

| 特性 | C++テンプレート | Rustジェネリクス | Haskell GADT | **YaoXiang** |
|------|---------|----------|--------------|--------------|
| 型パラメータ | ✅ | ✅ | ✅ | ✅ |
| 値依存型 | ❌ | ❌ | ✅ | ✅ |
| コンパイル時評価 | テンプレートインスタンス化 | ❌ | ✅ | ✅ |
| 停止保証 | ❌ | ❌ | ❌（危険） | ✅（decreases仕様） |
| 型安全性 | ❌（マクロ展開） | ✅ | ✅ | ✅ |
| 統一構文 | ❌ | ❌ | ❌ | ✅ |
| コンパイル時次元検証 | 手動特殊化 | ランタイム検査 | 型族 | コンパイル時自動検証 |
| decreases仕様 | ❌ | ❌ | ❌ | ✅ |

### 終了性検査機構（RFC-022との統合）

値依存型のコンパイル時評価は**停止を保証**しなければならない。さもないと型システムが無限ループに陥る。YaoXiangは **decreases 仕様** を通じてこれを保証し、RFC-022とシームレスに統合される。

#### 再帰関数の終了性仕様
```yaoxiang
# 编译期阶乘：必须证明终止
factorial: (n: Int) -> Int = {
    //! requires: n >= 0
    //! ensures: result == n!
    //! decreases: n    # 每次递归 n 严格递减
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

# 使用：在类型位置调用
vec: Vec(factorial(5)) = Vec(120)()  # 编译期求值 factorial(5) = 120
```

#### ループの終了性仕様
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

#### 終了性検査のワークフロー

```
┌─────────────────────────────────────────────────────────────┐
│  型検査フェーズ                                              │
│  型位置での関数呼び出しに遭遇（例：Vec(factorial(5))）       │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  1. decreases 仕様を検査                                    │
│     - decreases あり：すべての再帰経路で減少条件を検証        │
│     - decreases なしだが明らかに停止：直接評価                │
│     - decreases なしで停止しない可能性：コンパイルエラー      │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  2. コンパイル時評価（組み込みインタプリタで実行）           │
│     - 純粋関数：直接評価                                     │
│     - 副作用：コンパイルエラー（型位置は副作用なしが必須）    │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  3. 結果を型に埋め込み                                      │
│     - Vec(factorial(5)) → Vec(120)                          │
│     - Matrix(Float, 3, 3) → 具象型                          │
└─────────────────────────────────────────────────────────────┘
```

#### 優位性

- **安全性**：コンパイル時評価が必ず停止することを保証し、型システムの無限ループを回避
- **統一性**：終了性検査と部分正当性検証が同じ仕様機構を共有
- **段階的強化**：ランタイム検査から完全な静的証明へと段階的に移行可能

## 動機

### なぜ強力なジェネリクスシステムが必要か？

現在の主流言語のジェネリクスには限界がある：

| 言語 | ジェネリクス機能 | 問題 |
|------|----------|------|
| Java | 境界型 | コンパイル時単態化、ジェネリクス特殊化なし |
| C# | ジェネリック制約 | ランタイム型検査、パフォーマンスオーバーヘッドあり |
| Rust | ジェネリクス + Trait | Traitシステムが複雑、学習曲線が急峻 |
| C++ | テンプレート | テンプレート特殊化が複雑、コンパイルエラーメッセージが悪い |
| **YaoXiang** | **値依存型** | **型が値に依存可能、コンパイル時次元検証、停止保証** |

### 核心的矛盾

1. **パフォーマンス vs 柔軟性**：ランタイム柔軟性 vs コンパイル時最適化
2. **複雑さ vs 簡潔さ**：強力な型システム vs 使いやすさ
3. **マクロ vs ジェネリクス**：マクロコード生成 vs ジェネリクス型安全性
4. **値依存 vs 型安全性**：従来ジェネリクスはコンパイル時に次元を検証できない

### 値依存型の核心的な優位性

YaoXiangの**値依存型**は従来ジェネリクスに対する核心的な優位性である：

| 優位性 | 説明 |
|------|------|
| **型の値依存** | `Vec: (n: Int) -> Type` により型が具体的な値に依存可能 |
| **コンパイル時評価** | 型位置での関数呼び出しはコンパイル時に評価され、結果が型に直接埋め込まれる |
| **次元検証** | `Matrix(Float, 3, 3)` はコンパイル時に行列次元を検証 |
| **型レベル計算** | `If`, `Match` などの条件型による型レベル計算をサポート |
| **停止保証** | decreases 仕様によりコンパイル時評価の停止を保証 |

```yaoxiang
# C++/Rust 无法做到的编译期验证
matrix: Matrix(Float, factorial(3), factorial(2)) = ...
# 编译期计算：factorial(3) = 6, factorial(2) = 2
# 类型为 Matrix(Float, 6, 2)

# 维度不匹配在编译期捕获
identity: Matrix(Float, 3, 3) = ...
# multiply(matrix_2x3, identity_3x3)  # 编译错误：2 != 3
```

### ジェネリクスシステムの価値

```yaoxiang
# 示例：统一API设计
# 不同容器类型的map操作

# 传统方案：每个类型单独实现
map_int_array: (array: Array(Int), f: Fn(Int) -> Int) -> Array(Int) = ...
map_string_array: (array: Array(String), f: Fn(String) -> String) -> Array(String) = ...
map_int_list: (list: List(Int), f: Fn(Int) -> Int) -> List(Int) = ...
map_string_list: (list: List(String), f: Fn(String) -> String) -> List(String) = ...

# 泛型方案：一个泛型函数覆盖所有类型
map: (T: Type, R: Type)(container: Container(T), f: Fn(T) -> R) -> Container(R) = {
    for item in container {
        result.push(f(item))
    }
    result
}
```

## 設計目標

### 核心目標

1. **ゼロコスト抽象** - ジェネリック呼び出しは具象型呼び出しと等価
2. **デッドコード除去** - コンパイル時解析、使用されるジェネリクスのみをインスタンス化
3. **マクロ代替** - ジェネリクスがマクロ使用シーンの90%を代替
4. **型安全性** - コンパイル時検査、ランタイム型オーバーヘッドなし
5. **IDEフレンドリー** - スマートヒント、明確なエラーメッセージ
6. **値依存型** - 型が値に依存可能、コンパイル時次元検証をサポート
7. **コンパイル時評価の安全性** - decreases 仕様によりコンパイル時評価の停止を保証

### 設計原則

- **コンパイル時決定**：ジェネリックパラメータはコンパイル時に決定
- **単態化優先**：具象コードを生成、仮想関数呼び出しを回避
- **制約駆動**：型制約がインスタンス化を指導
- **プラットフォーム最適化**：プラットフォーム固有最適化の特殊化サポート
- **型ユニバースの統一**：関数/型コンストラクタ/値依存型をType2層に統一
- **停止保証**：型位置での関数呼び出しは停止を証明しなければならない

## 提案

### 1. 基本ジェネリクス

#### 1.1 ジェネリック型パラメータ

> **重要なルール**：ジェネリック型定義は**明示的に `: Type` を标注しなければならない**。さもないとHM推論により関数として扱われる。
>
> | 書き方 | 意味 |
> |------|------|
> | `List: (T: Type) -> Type = {...}` | ✅ 型コンストラクタ |
> | `List = {...}` | ❌ HM推論により関数として扱われ、型ではない |

```yaoxiang
# 泛型类型定义（必须有 : Type）
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
    push: (self: List(T), item: T) -> Void,   # self 只是约定名，不是关键字
    get: (self: List(T), index: Int) -> Option(T),
}

# 泛型函数（无 : Type，HM 推断为函数）
map: (T: Type, R: Type) -> ((opt: Option(T), f: Fn(T) -> R) -> Option(R)) = {
    return match opt {
        some => Option.some(f(some)),
        none => Option.none(),
    }
}

# 泛型约束（直接表达式，单行可省略 return）
clone: (T: Clone)(value: T) -> T = value.clone()

# 多类型参数
combine: (T: Type, U: Type) -> ((a: T, b: U) -> (T, U)) = (a, b)
```

### ジェネリック関数呼び出し構文

#### 1.1 統一シグネチャ構文

```yaoxiang
# 泛型函数使用统一的 (T: Type, R: Type) 签名语法
map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R)) = ...

# 多类型参数
combine: (T: Type, U: Type) -> ((a: T, b: U) -> (T, U)) = (a, b)
```

#### 1.2 Type 自己記述機構

`Type` は言語レベルの特別な存在であり、コンパイラはシグネチャ中の `Type` 位置を自然に認識し、実際の引数の型から自動的に推論・補完する。

```yaoxiang
# 编译器自动推断泛型参数
numbers: List(Int) = List(Int)
#         ^^^^^^^^   ^^^^^^
#         类型声明   构造调用：Int 填充 T

# 函数调用推断
numbers: List(Int) = List(Int)
f: (x: Int) -> String = (x) => x.to_string()
strings: List(String) = map(numbers, f)
# 编译器推断：T=Int, R=String
```

#### 1.3 単態化

```yaoxiang
# 源代码
map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R)) = {
    result: List(R) = List(R)()
    for x in list {
        result.push(f(x))
    }
    return result
}

# 使用点
int_list: List(Int) = List(Int)
doubled: List(Int) = map(int_list, (x: Int) => x * 2)  # 实例化 map[Int, Int]

string_list: List(String) = List(String)
uppercased: List(String) = map(string_list, (s: String) => s.to_uppercase())  # 实例化 map[String, String]

# 编译后（等价代码）
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

#### 1.4 明示的補完（推論失敗時）

```yaoxiang
# 可推断时省略 Type 参数
numbers: List(Int) = List(Int)
strings: List(String) = map(numbers, (x: Int) => x.to_string())

# 无法推断时必须显式填充
# map(numbers, (x) => x)  # ❌ Error: Cannot infer R
```

### 2. 型制約システム

#### 2.1 単一制約

```yaoxiang
# 基本trait定义（接口类型）
Clone: Type = {
    clone: (Self) -> Self,
}

Display: Type = {
    fmt: (Self, Formatter) -> Result,
}

Debug: Type = {
    fmt: (Self, Formatter) -> Result,
}

# 使用约束：在签名中直接声明类型约束
clone: (T: Clone) -> (value: T) -> T = value.clone()

debug_print: (T: Debug)(value: T) -> Void = {
    formatter = Formatter.new()
    value.fmt(formatter)
    print(formatter.to_string())
}
```

#### 2.2 複数制約

```yaoxiang
# 多重约束语法
combine: (T: Clone + Add)(a: T, b: T) -> T = {
    a.clone() + b
}

# 泛型容器的排序
sort: (T: Clone + PartialOrd)(list: List(T)) -> List(T) = {
    # 实现排序算法
    result: List(T) = list.clone()
    quicksort(&mut result)
    return result
}

# 函数类型约束
map: (T: Type, R: FnMut(T))(array: Array(T), f: R) -> Array(R) = {
    result: Array(R) = Array()
    for item in array {
        result.push(f(item))
    }
    return result
}

# 使用
doubled: Array(Int) = map(Array(1, 2, 3), (x: Int) => x * 2)  # 编译器推断
```

#### 2.3 関数型制約

```yaoxiang
# 高阶函数约束
call_twice: (T: Type, F: Fn() -> T)(f: F) -> (T, T) = (f(), f())

call_with_arg: (T: Type, U: Type, F: Fn(T) -> U)(arg: T, f: F) -> U = f(arg)

compose: (A: Type, B: Type, C: Type, F: Fn(A) -> B, G: Fn(B) -> C)(a: A, f: F, g: G) -> C = g(f(a))

# 使用示例
result: Int = call_with_arg(42, (x: Int) => x * 2)  # result = 84
composed: String = compose(
    "hello",
    (s: String) => s.to_uppercase(),
    (s: String) => s + " WORLD"
)  # composed = "HELLO WORLD"
```

#### 2.4 組み込みマーカートレイト：DupとClone

**3種類のコピーセマンティクス**：

| 型 | 意味 | トリガー方式 | 適用シーン |
|------|------|----------|----------|
| **プリミティブ値コピー** | 代入時に自動値コピー、2つの値が完全に独立 | 代入/引数渡し自動 | Int, Float, Bool, Char |
| **Dup** | シャローコピー：ハンドル/トークンをコピー、基底データを共有 | 代入/引数渡し自動 | `&T` トークン、`ref T`、String/Bytes |
| **Clone** | ディープコピー：完全独立なコピーを作成 | `value.clone()` | Cloneを実装する任意の型 |

**Dupのセマンティクス**：Dupを実装した型は、代入/引数渡し時に所有権を移転しない——コンパイラがハンドル/トークンをコピーし、複数の所有者が同じ基底データを指す。これはRFC-009所有権モデルにおけるデフォルトMoveセマンティクスの補完関係にある。

**DupとCloneは直交する概念**：

```
Dup = ハンドルのコピー、データを共有（変更が互いに影響）
Clone = データのコピー、コピーは独立（変更が互いに影響しない）
```

**ルール**：

```
1. プリミティブ値型（Int, Float, Bool, Char） — コンパイラ組み込み値コピー、Dupには属さない
2. Dup  — 参照/トークン型および内部参照カウント型のみに適用
3. Clone — 明示的ディープコピー、任意の型で実装可能
4. デフォルトMove — 他の型はデフォルトMoveセマンティクスを維持
```

**どの型がDupか**：

| 型 | Dup | 理由 |
|------|-----|------|
| `&T`（借用トークン） | ✅ | ゼロサイズトークン、トークンコピー = 複数の視点が同じデータを指す |
| `ref T` | ✅ | Rc/Arcのコピー = 参照カウント+1、ヒープデータを共有 |
| String, Bytes | ✅ | 内部参照カウント、ハンドルコピーにより基底bufferを共有 |
| `&mut T`（可変トークン） | ❌ | 線形排他、コピー不可 |
| struct | 派生 | 全フィールドDup → struct Dup |
| enum | 派生 | 全variantの全フィールドDup → enum Dup |
| tuple | 派生 | 全要素Dup → tuple Dup |
| Fn（クロージャ） | ❌ | キャプチャ環境が非Dupの可能性あり |
| `*T`（生ポインタ） | ❌ | unsafe、所有権システムに参加しない |

**Int/Float/Bool/CharはDupではない**——これらは値型であり、代入時にコンパイラが自動値コピーを行う（2つの値が完全に独立）。これは「シャローコピー」ではなく、コンパイラのプリミティブに対する組み込み処理であり、Dup型属性を通じて表現すべきものではない。

```yaoxiang
# 原语值类型：编译器自动值复制（不是 Dup）
x: Int = 42
y = x          # 值复制，x 和 y 完全独立
print(x)       # ✅

# Dup：浅拷贝，复制句柄共享数据
view: &Point = &point
view2 = view    # ✅ Dup：复制令牌，两者指向同一个 point
print(view.x)   # ✅

# Clone：显式深拷贝，创建独立副本
backup = big_struct.clone()  # 显式调用

# 泛型约束
dup_use: (T: Dup) -> T = x         # T: Dup → 可以浅拷贝
clone_use: (T: Clone) -> T = x.clone()  # T: Clone → 可以深拷贝
```

> **注意**：`Send`/`Sync` はユーザー可視traitとして提供されない。タスク横断的安全性保証は `ref` キーワードとコンパイラの完全自動処理によって実現される——`ref` が自動的にRcまたはArcを選択し、ユーザーはSend/Syncを理解する必要がない。

### 3. 関連型

#### 3.1 関連型定義

```yaoxiang
# Iterator trait（使用 (Item: Type) -> Type 语法）
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

# Array的Iterator实现
# 使用方法语法糖：Array.Item, Array.next, Array.has_next
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

#### 3.2 ジェネリック関連型（GAT）

```yaoxiang
# 更复杂的关联类型
Producer: (Item: Type) -> Type = {
    Item: T,
    produce: (Self) -> Option(Item),
}

# 关联类型可以是泛型的
Container: (Item: Type) -> Type = {
    Item: T,
    IteratorType: Iterator(Item),  # 关联类型也是泛型的
    iter: (Self) -> IteratorType,
}

# 使用
process_container: (T: Type, C: Container(T))(container: C) -> List(T) = {
    container.iter().collect()
}
```

### 4. コンパイル時ジェネリクス

#### 4.1 コンパイル時定数パラメータ

**核心設計**：ジェネリックシグネチャ中の `Type` はコンパイル時型パラメータを示し、`Int` などの値パラメータはジェネリックコンテキストではデフォルトでコンパイル時に決定可能。`const` キーワードは不要。

```yaoxiang
# ════════════════════════════════════════════════════════
# 编译期常量参数：泛型中的 Int 默认编译期确定
# ════════════════════════════════════════════════════════

# 编译期阶乘：N 必须是编译期已知的字面量
factorial: (N: Int) -> (n: N) -> Int = {
    return match n {
        0 => 1,
        _ => n * factorial(n - 1)
    }
}

# 编译期加法
add: (a: Int, b: Int) -> (a: a, b: b) -> Int = a + b

# ════════════════════════════════════════════════════════
# 编译期常量数组
# ════════════════════════════════════════════════════════
StaticArray: (T: Type, N: Int) -> Type = {
    data: Array(T, N),  # 编译期已知大小的数组
    length: N,
}

# 使用方式
arr: StaticArray(Int, factorial(5))  # StaticArray(Int, 120)，编译器在编译期计算
```

#### 4.2 コンパイル時計算

```yaoxiang
# ════════════════════════════════════════════════════════
# 编译期计算示例
# ════════════════════════════════════════════════════════

# 编译器在编译期计算字面量类型的函数调用
SIZE: Int = factorial(5)  # 编译期为 120

# 矩阵类型使用
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),
}

# 编译期维度验证
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

# 使用：编译期计算，生成 Matrix(Float, 3, 3)
identity_3x3: Matrix(Float, 3, 3) = identity_matrix(Float, 3)(3)
```

### NeverとVoid：型システムの ⊥ と ⊤

YaoXiangの型システムはカリー・ハワード同型対応において ⊥（偽/空型）と ⊤（真/Unit）を同時に備え、`Never` と `Void` という2つの組み込み型名がそれを担う：

**Never（⊥）** — 交渉不可の3つのコア性質：

1. **ゼロコンストラクタ**：リテラルも式も `Never` 型の値を生成できない。これはメタレベル性質であり、組み込み必須。
2. **爆発原理**：`Never <: T` は任意の型 `T` について成立する。`Never` 値は任意の型として使用可能——これが `assert(false)` 後のコードが型検査を通る理由である（決して実行されないが）。
3. **発散マーカー**：`f: (...) -> Never` は `f` が返らないことを保証する。コンパイラはこれに基づきデッドコード解析を行う。

`Never` は組み込み型名でありキーワードではないためパーサーは影響を受けない。空や型リテラル構文は開放しない。

**Void（⊤、すなわちUnit）** — ちょうど1つの居住者（デフォルトのvoid値）を持ち、真命題「常に真」の担い手である。`Void` はゼロフィールド積型の単位元、`Never` はゼロバリアント和型の単位元——両者は双対。`x: Void = <デフォルト>` は合法、`x: Never = ...` は右辺に書けるものがない。

#### 4.3 コンパイル時検証（標準ライブラリ実装）

```yaoxiang
# ════════════════════════════════════════════════════════
# 标准库实现：利用条件类型
# ════════════════════════════════════════════════════════

# 标准库定义
# IsTrue：值宇宙到类型宇宙的桥——Bool 真值映射为类型
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      # ⊤，有值，程序继续
    false => Never,    # ⊥，无值，发散
}

# Assert：编译期精化类型原语——对 Bool 命题的类型级表述
Assert: (cond: Bool) -> Type = IsTrue(cond)
#
# cond 为 true  → Assert(true)  = Void    （恒真，擦除）
# cond 为 false → Assert(false) = Never   （恒假，编译错误/发散）
# cond 判不了   → 由证明管道按 dispatch 模式决定：
#                  CompileTime → Unknown，要求 prove
#                  Runtime     → 插入 check，注入 Γ 假设

# 使用方式1：在类型定义中作为约束
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    # 编译期检查：N 必须大于 0（Assert 在类型位置）
    length: Assert(N > 0),
}

# 使用方式2：在表达式中使用
IntArray: (N: Int) -> Type = StaticArray(Int, N)
# 验证：IntArray(10) 的大小等于 sizeof(Int) * 10
Assert(size_of(IntArray(10)) == sizeof(Int) * 10)
```

#### 4.4 コンパイル時ジェネリクス特殊化

```yaoxiang
# 小数组优化：使用函数重载实现编译期泛型特化

# 通用实现
sum: (T: Type, N: Int) -> ((arr: Array(T, N)) -> T) = {
    result = Zero::zero()
    for item in arr.data {
        result = result + item
    }
    return result
}

# N=1 特化
sum: (T: Type) -> ((arr: Array(T, 1)) -> T) = arr.data[0]

# N=2 特化
sum: (T: Type) -> ((arr: Array(T, 2)) -> T) = arr.data[0] + arr.data[1]

# 小数组循环展开（N <= 4）
sum: (T: Type, N: Int) -> ((arr: Array(T, N)) -> T) = {
    # 编译器优化：展开循环
    return arr.data[0] + arr.data[1] + arr.data[2] + arr.data[3]
}
```

### 5. 条件型

> **カリー・ハワード同型対応**：条件型をカリー・ハワードの視点から見ると、論理における **ケース分析** である。`Bool` 型は2つの可能な値（True/False）を持つ命題に対応し、`If` はその命題の真偽に応じて異なる結果を選択する——これはまさに論理におけるケース選言である。`match C { True => T, False => E }` は実際には「命題CがTrueのとき結論はT、CがFalseのとき結論はE」を表現している。

#### 5.1 If条件型

```yaoxiang
# 类型级If
If: (C: Bool, T: Type, E: Type) -> Type = match C {
    True => T,
    False => E,
}

# 示例：编译期分支
NonEmpty: (T: Type) -> Type = If(T != Void, T, Never)

Optional: (T: Type) -> Type = If(T != Void, T, Void)

# 编译期验证（统一到 §4.3 的 Assert 定义）
# Assert: (cond: Bool) -> Type = IsTrue(cond)

# 使用
# 类型计算：If(True, Int, String) => Int
# 类型计算：If(False, Int, String) => String
```

#### 5.2 型族

> **カリー・ハワード同型対応**：型族は「命題は型である」の最も直接的な体現である。`Add: (A: Type, B: Type) -> Type` は「型レベルに加法関数を記述した」のではなく、**自然数加法に関する命題を構築している**。`(Zero, B) => B` は「命題 Add(Zero, B) は B と同値」を意味し、`(Succ(A'), B) => Succ(Add(A', B))` は「Add(A', B) が成立するなら Add(Succ(A'), B) も成立」を意味する。これがペアノ公理における加法定義そのものである。型検査器がこのmatch式が通過することを検証することは、この定義の論理的一貫性を検証することと同値である。

```yaoxiang
# 编译期类型转换
AsString: (T: Type) -> Type = match T {
    Int => String,
    Float => String,
    Bool => String,
    _ => String,  # 默认
}

# 类型级计算
Length: (T: Type) -> Type = match T.length {
    0 => Zero,
    1 => Succ(Zero),
    2 => Succ(Succ(Zero)),
    _ => TooLong,
}

# 类型级加法（Curry-Howard：case analysis + 递归调用，需要终止性检查才是完整归纳）
Add: (A: Type, B: Type) -> Type = match (A, B) {
    (Zero, B) => B,
    (Succ(A'), B) => Succ(Add(A', B)),
}

# 示例：编译期计算 2 + 3
Two: Type = Succ(Succ(Zero))
Three: Type = Succ(Succ(Succ(Zero)))
Five: Type = Add[Two, Three]  # Succ(Succ(Succ(Succ(Succ(Zero)))))
```

### 6. 関数オーバーロード特殊化

#### 6.1 基本特殊化

```yaoxiang
# 基本特化：使用函数重载（编译器自动选择）
sum: (arr: Array(Int)) -> Int = {
    # 编译为更高效的代码
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Array(Float)) -> Float = {
    # 使用SIMD指令
    return simd_sum_float(arr.data, arr.length)
}

# 通用实现
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
# 完全符合RFC-010语法的特化方式：函数重载

# 具体类型特化
sum: (arr: Array(Int)) -> Int = {
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Array(Float)) -> Float = {
    return simd_sum_float(arr.data, arr.length)
}

# 泛型实现（编译器自动选择最优）
sum: (T: Type) -> ((arr: Array(T)) -> T) = {
    result = Zero::zero()
    for item in arr {
        result = result + item
    }
    return result
}

# 使用时完全透明
int_arr = Array(Int)(1, 2, 3)
float_arr = Array(Float)(1.0, 2.0, 3.0)

# 编译器自动选择最优特化
sum(int_arr)     # 选择 sum: (Array(Int)) -> Int
sum(float_arr)    # 选择 sum: (Array(Float)) -> Float
```

#### 6.3 関数オーバーロードとインライン化の完璧な組み合わせ

**重要な特性**：関数オーバーロードとインライン化最適化は自然に組み合わさり、ゼロコスト抽象を実現する。

```yaoxiang
# ======== 源代码 ========
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

# ======== 编译后（等价代码）=======
# 编译器自动选择最优特化，然后内联
result = native_sum_int(int_arr.data, int_arr.length)

# 完全等价于手写优化代码，无函数调用开销！
```

**核心的な優位性**：

1. **コンパイラのスマート選択**
   ```yaoxiang
   sum(int_arr)      # 自动选择 sum: (Array(Int)) -> Int
   sum(float_arr)    # 自动选择 sum: (Array(Float)) -> Float
   sum(custom_arr)  # 自动选择 sum: (T: Type) -> ((arr: Array(T)) -> T)
   ```

2. **インライン最適化**
   - 小さい関数は呼び出し点に自動インライン化
   - 関数呼び出しオーバーヘッドなし
   - 手書き最適化コードと完全等価

3. **型安全性**
   - コンパイル時型検査
   - ランタイムオーバーヘッドなし
   - 仮想関数テーブル不要

4. **RFC-010に完璧に準拠**
   ```yaoxiang
   # 完全使用统一语法
   name: type = value
   # 无需impl、where等新关键字
   ```

**実際の応用例**：

```yaoxiang
# 性能敏感的数值计算
fibonacci: (n: Int) -> Int = {
    if n <= 1 { return n }
    return fibonacci(n - 1) + fibonacci(n - 2)
}

fibonacci: (n: Float) -> Float = {
    # 使用Binet公式
    phi = (1.0 + 5.0.sqrt()) / 2.0
    return (phi.pow(n) - (-phi).pow(-n)) / 5.0.sqrt()
}

# 编译器自动选择并内联
fibonacci(10)      # 选择 Int 版本，完全内联
fibonacci(10.5)    # 选择 Float 版本，使用Binet公式
```

**これは何を意味するか？**

- ✅ **ジェネリック特殊化** → 関数オーバーロードで自然に解決
- ✅ **パフォーマンス最適化** → インライン化が自動完了
- ✅ **コード再利用** → 1つの関数名で複数の実装
- ✅ **ゼロコスト抽象** → コンパイル時多態、ランタイムオーバーヘッドなし
- ✅ **新規キーワード不要** → RFC-010統一構文に完璧に準拠

### 7. デッドコード除去機構

#### 7.1 インスタンス化グラフ解析

```rust
// 编译器内部：构建泛型实例化依赖图
struct InstantiationGraph {
    // 节点：泛型实例化
    nodes: HashMap<InstanceKey, InstanceNode>,

    // 边：使用关系
    edges: HashMap<InstanceKey, Vec<InstanceKey>>,
}

struct InstanceKey {
    generic: FunctionId,  // 泛型函数ID
    type_args: Vec<TypeId>,  // 类型参数
    const_args: Vec<ConstId>,  // Const参数
}

// 算法：可达性分析
fn eliminate_dead_instantiations(graph: &InstantiationGraph) {
    let mut reachable = HashSet::new();

    // 从入口点开始（main、导出函数等）
    let entry_points = find_entry_points();
    for entry in entry_points {
        dfs_visit(entry, &graph, &mut reachable);
    }

    // 未访问的实例化就是死代码
    for node in &graph.nodes {
        if !reachable.contains(node.key) {
            eliminate(node);
        }
    }
}
```

#### 7.2 使用箇所解析

```yaoxiang
# 源代码分析
map: (T: Type, R: Type)(list: List(T), f: Fn(T) -> R) -> List(R) = ...

# 使用点1：实例化 map(Int, Int)
int_list = List(1, 2, 3)
doubled = map(int_list, (x) => x * 2)  # 需要 map[Int, Int]

# 使用点2：实例化 map(String, String)
string_list = List("a", "b", "c")
uppercased = map(string_list, (s) => s.to_uppercase())  # 需要 map[String, String]

# 未使用：map[Float, Float] 等
# 这些泛型实例不会被生成

# 编译后只包含被使用的实例
map_Int_Int: (list: List(Int), f: Fn(Int) -> Int) -> List(Int) = ...
map_String_String: (list: List(String), f: Fn(String) -> String) -> List(String) = ...
```

#### 7.3 コンパイル時ジェネリクスDCE

```yaoxiang
# 编译期分析：编译期泛型使用情况
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
}

# 实际使用情况
arr_10_int = Array(Int, 10)(1, 2, 3, 4, 5, 6, 7, 8, 9, 10)
arr_100_int = Array(Int, 100)(...)

# 编译后只生成被使用的Size
Array_Int_10: (Array(Int, 10)) = ...
Array_Int_100: (Array(Int, 100)) = ...

# 未使用的Size不会生成
# Array(Int, 50) 不会生成
```

#### 7.4 モジュール横断DCE

```yaoxiang
# 模块A
# A.yx
pub map: (T: Type, R: Type)(list: List(T), f: Fn(T) -> R) -> List(R) = ...

# 模块B
# B.yx
use A.{map}
int_list = List(1, 2, 3)
doubled = map(int_list, (x) => x * 2)  # 实例化 map(Int, Int)

# 模块C
# C.yx
use A.{map}
string_list = List("a", "b", "c")
uppercased = map(string_list, (s) => s.to_uppercase())  # 实例化 map(String, String)

# 编译分析：
# - 模块B使用 map[Int, Int]
# - 模块C使用 map[String, String]
# - 编译后二进制只包含这两个实例
```

#### 7.5 LLVMレベルDCE

```rust
// 编译流水线
fn optimize_ir(ir: &mut IR) {
    // 1. 单态化（YaoXiang编译器）
    ir.monomorphize();

    // 2. 内联优化
    ir.inline_small_functions();

    // 3. 常量传播
    ir.constant_propagation();

    // 4. 生成LLVM IR
    let llvm_ir = ir.to_llvm();

    // 5. LLVM优化pass
    llvm_ir.add_pass(Passes::DEAD_CODE_ELIMINATION);
    llvm_ir.add_pass(Passes::INLINE_FUNCTION);
    llvm_ir.add_pass(Passes::GLOBAL_DCE);
    llvm_ir.add_pass(Passes::MERGE_FUNC);

    // 6. 运行优化
    llvm_ir.run_optimization_passes();
}
```

### 8. マクロ代替戦略

#### 8.1 コード生成代替

```yaoxiang
# ❌ 宏方案：代码生成
macro_rules! impl_debug {
    ($($t:ty),*) => {
        $(impl Debug for $t {
            fn fmt(&self, f: &mut Formatter) -> Result {
                write!(f, "{:?}", self)
            }
        })*
    };
}

# ✅ 泛型方案：自动派生
# 使用函数重载方式自动派生
debug_fmt: (T: fields...) -> ((self: Point(T)) -> String) = {
    return "Point { x: " + self.x.to_string() + ", y: " + self.y.to_string() + " }"
}

# 使用
p = Point { x: 1, y: 2 }
p.debug_fmt(&formatter)  # 自动生成调用
```

#### 8.2 DSL代替

```yaoxiang
# ❌ 宏方案：HTML DSL
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

# ✅ 泛型方案：类型安全构建器
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

# 构建DOM
container = create_element("div")
    |> with_class("container")
    |> with_children(List::new())

title_elem = create_element("h1") |> with_text(title)
items_li = items.map((item) =>
    create_element("li") |> with_text(item)
)
root = container |> with_children(List::new() + [title_elem, ul_elem])
```

#### 8.3 型レベルプログラミング代替

```yaoxiang
# ❌ 宏方案：类型级计算
macro_rules! add_types {
    ($a:ty, $b:ty) => {
        ($a, $b)
    };
}

# ✅ 泛型方案：条件类型
Add: (A: Type, B: Type) -> Type = match (A, B) {
    (Int, Int) => Int,
    (Float, Float) => Float,
    (Int, Float) => Float,
    (Float, Int) => Float,
    _ => TypeError,
}

# 编译期验证
AssertAddable: (A: Type, B: Type) -> Type = If(Add(A, B) != TypeError, (A, B), compile_error("Cannot add"))

# 使用
result_type = Add[Int, Float]  # 推导为 Float
```

### 9. 例

#### 9.1 完全なジェネリックコンテナ例

```yaoxiang
# ======== 1. 定义泛型容器 ========
# 使用 (T: Type) -> Type 语法
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

    # 泛型方法（T 由外层 List(T) 自动带入作用域）
    push: (self: List(T), item: T) -> Void,
    pop: (self: List(T)) -> Option(T),
    map: (R: Type) -> ((self: List(T), f: (T) -> R) -> List(R)),
    filter: (self: List(T), predicate: (T) -> Bool) -> List(T),
    fold: (U: Type) -> ((self: List(T), initial: U, f: (U, T) -> U) -> U),
}

# ======== 2. 实现泛型方法 ========
# 函数定义在 List 命名空间下（List. 前缀 = 命名空间归属）
# 要让 list.push(item) 这种 . 调用语法生效，需要显式绑定：List.push = push[0]
# self 只是约定参数名，编译器不看名字看类型

List.push: (T: Type) -> ((self: List(T), item: T) -> Void) = {
    if self.length >= self.data.length {
        # 扩容
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

# ======== 3. 类型约束使用 ========
# 实现 Clone for List
List.clone: (T: Clone) -> ((self: List(T)) -> List(T)) = {
    result = List(T)()
    for i in 0..self.length {
        result.push(self.data[i].clone())
    }
    return result
}

# ======== 4. 使用示例 ========
# 创建泛型List
numbers = List(Int)()
numbers.push(1)
numbers.push(2)
numbers.push(3)

# 使用泛型方法
doubled = numbers.map((x) => x * 2)
evens = numbers.filter((x) => x % 2 == 0)

# 使用fold计算
sum = numbers.fold(0, (acc, x) => acc + x)  # sum = 6

# 泛型组合
sum_of_evens = numbers
    .filter((x) => x % 2 == 0)
    .map((x) => x * 2)
    .fold(0, (acc, x) => acc + x)  # sum_of_evens = 8
```

#### 9.2 ジェネリックアルゴリズム例

```yaoxiang
# ======== 1. 泛型排序算法 ========
Comparator: (T: Type) -> Type = {
    compare: (T, T) -> Int,  # -1 if a < b, 0 if a == b, 1 if a > b
}

# 泛型quicksort
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

# ======== 2. IntComparator实现 ========
# 使用函数重载实现
compare: (a: Int, b: Int) -> Int = {
    if a < b {
        return -1
    } else if a > b {
        return 1
    } else {
        return 0
    }
}

# ======== 3. 使用示例 ========
# 排序Int数组
numbers = Array(Int)(3, 1, 4, 1, 5, 9, 2, 6)
sorted = quicksort(numbers, Comparator(Int)())

# 排序String数组（需要StringComparator）
strings = Array(String)("hello", "world", "foo", "bar")
sorted_strings = quicksort(strings, Comparator(String)())
```

#### 9.3 コンパイル時ジェネリクス例

```yaoxiang
# ======== 1. 编译期矩阵类型 ========
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),

    # 编译期维度验证：利用 Assert 标准库类型
    _assert: Assert[Rows > 0],  # Rows > 0，否则编译错误
    _assert: Assert[Cols > 0],  # Cols > 0，否则编译错误

    # 矩阵运算
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

# ======== 2. 编译期矩阵创建 ========
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

# ======== 3. 使用示例 ========
# 创建编译期已知大小的矩阵
# 2x3 矩阵
matrix_2x3 = Matrix(Float, 2, 3)()
matrix_2x3.data[0][0] = 1.0
matrix_2x3.data[0][1] = 2.0
matrix_2x3.data[0][2] = 3.0
matrix_2x3.data[1][0] = 4.0
matrix_2x3.data[1][1] = 5.0
matrix_2x3.data[1][2] = 6.0

# 3x2 矩阵
matrix_3x2 = Matrix(Float, 3, 2)()
matrix_3x2.data[0][0] = 7.0
matrix_3x2.data[0][1] = 8.0
matrix_3x2.data[1][0] = 9.0
matrix_3x2.data[1][1] = 10.0
matrix_3x2.data[2][0] = 11.0
matrix_3x2.data[2][1] = 12.0

# 矩阵乘法：2x3 * 3x2 = 2x2
result = matrix_2x3.multiply(matrix_3x2)

# 编译期验证：result类型为 Matrix(Float, 2, 2)
# 2x2 单位矩阵
identity_3x3 = identity(Float, 3)()

# 维度不匹配：编译错误
# bad_multiply = matrix_2x3.multiply(identity_3x3)  # 编译错误：3x3 != 2x3
```

## トレードオフ

### メリット

1. **ゼロコスト抽象**
   - コンパイル時単態化、ランタイムオーバーヘッドなし
   - 仮想関数不要、RTTI不要

2. **デッドコード除去**
   - コンパイル時解析、使用されるジェネリクスのみをインスタンス化
   - コード膨張が制御可能

3. **マクロ代替**
   - 型安全なコード生成
   - IDEフレンドリー、明確なエラーメッセージ

4. **コンパイル時計算**
   - コンパイル時ジェネリクスがコンパイル時計算をサポート
   - 次元検証などの機能
   - `const` キーワード不要、純粋な型制約

### デメリット

1. **コンパイル時間**
   - ジェネリクスインスタンス化がコンパイル時間を増加させる
   - 制約解決が遅くなる可能性

2. **メモリ使用量**
   - コンパイラのメモリ使用量が増加
   - キャッシュ機構がメモリを必要とする

3. **実装の複雑さ**
   - 制約ソルバーが複雑
   - 型レベル計算エンジンが複雑

4. **エラー診断**
   - ジェネリクスエラーが複雑になる可能性
   - 明確なエラーメッセージが必要

### 緩和策

1. **キャッシュ戦略**
   - インスタンス化結果のキャッシュ
   - LRUキャッシュによるメモリ制限

2. **インクリメンタルコンパイル**
   - コンパイル結果のキャッシュ
   - インクリメンタルインスタンス化

3. **エラー通知**
   - 明確なエラーメッセージ
   - ジェネリックパラメータ推論のヒント

4. **並列コンパイル**
   - ジェネリクスの並列インスタンス化
   - マルチスレッド制約解決

## 代替案

| 代替案 | 選択しない理由 |
|------|--------------|
| 基本ジェネリクスのみ | 複雑なマクロを代替できない |
| 純粋なマクロシステム | 型安全性なし、エラーメッセージが悪い |
| 制約依存のみ | 柔軟性不足 |
| ランタイムジェネリクス | パフォーマンスオーバーヘッドあり |

### リスク

| リスク | 影響 | 緩和策 |
|------|------|----------|
| 制約解決の複雑さ | コンパイル時間が過大 | インクリメンタル解決 + キャッシュ |
| コード膨張 | バイナリファイルが大きすぎ | DCE + 閾値制御 |
| 実装の複雑さ | 開発期間が長期化 | 段階的実装 |
| エラー診断 | ユーザー体験が悪い | 詳細なエラーメッセージ |

## オープンな問題

### 未解決問題

| 議題 | 説明 | 状態 |
|------|------|------|
| インスタンス化戦略 | Eager vs Lazy vs Threshold | 議論待ち |
| キャッシュサイズ | LRUキャッシュ容量設定 | 議論待ち |
| エラー診断 | ジェネリクスエラーメッセージの詳細度 | 議論待ち |

### 今後の最適化

| 最適化項目 | 価値 | 実装難易度 |
|--------|------|----------|
| インスタンス化グラフ解析 | 高 | 中 |
| 型レベルプログラミングDSL | 中 | 高 |
| ジェネリクスパフォーマンスベンチマーク | 中 | 低 |

## 付録

### 構文BNF

```bnf
# 泛型参数使用统一 () 语法，作为函数类型的一部分
# 如 map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R))

# 类型约束（在泛型参数中）
type_bound ::= identifier
             | identifier '+' identifier ('+' identifier)*

# 参数声明（类型 + 名字）
parameter ::= identifier ':' type

parameters ::= parameter (',' parameter)*

# 函数声明：name: type = expression
# 泛型参数是函数类型中的第一个参数组：(T: Type) -> ((params) -> return)
function ::= identifier ':' type '=' (expression | block)

# 方法声明：Type.method: type = expression
method ::= identifier '.' identifier ':' type '=' (expression | block)

# 类型定义（统一 Binding 语法）
# 泛型类型如 List: (T: Type) -> Type = { ... }
generic_type ::= identifier ':' type '=' type_expression

# 泛型参数中的 Type 由编译器自动从实参类型填充
# 如 map(numbers, f)，T 从 numbers: List(Int) 提取，R 从 f: (Int) -> String 提取
```

## ライフサイクルと結末

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
│  承認済み   │    │  拒否       │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   accepted/ │    │    rfc/     │
│ (正式設計)  │    │ (現状維持)  │
└─────────────┘    └─────────────┘
```

---

## 参考文献

### YaoXiang公式ドキュメント

- [RFC-010: 統一型構文](./010-unified-type-syntax.md)
- [RFC-009: 所有権モデル](./accepted/009-ownership-model.md)
- [RFC-001: spawnモデル](./accepted/001-concurrent-model-error-handling.md)
- [RFC-008: ランタイムモデル](./accepted/008-runtime-concurrency-model.md)
- [tutorial/ チュートリアル](../../../../../tutorial/)

### 外部参考

- [Rustジェネリクスシステム](https://doc.rust-lang.org/book/ch10-01-syntax.html)
- [C++テンプレート特殊化](https://en.cppreference.com/w/cpp/language/template_specialization)
- [Haskell型クラス](https://www.haskell.org/tutorial/classes.html)
- [Swiftジェネリクス](https://docs.swift.org/swift-book/LanguageGuide/Generics.html)
- [単態化最適化](https://llvm.org/docs/Monomorphization.html)
- [デッドコード除去](https://en.wikipedia.org/wiki/Dead_code_elimination)
```
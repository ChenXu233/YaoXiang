---
title: "RFC-011: ジェネリックシステム設計 - ゼロコスト抽象化とマクロ代替"
status: "受入済み"
author: "晨煦"
updated: "2026-07-15（タイプ体コードブロック + コンパイル時仕様 + エフェクトシード実装済み）"
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

# RFC-011: ジェネリックシステム設計 - ゼロコスト抽象化とマクロ代替

## 摘要

本文書はYaoXiang言語の**ジェネリックシステム設計**を定義し、強力なジェネリック機能によりゼロコスト抽象化を実現し、コンパイル時最適化によりマクロへの依存を軽減し、デッドコードElimination機構を提供する。

**コア設計**：
- **統一シグネチャ構文**：`T: Type, R: Type` ジェネリック引数と通常引数の統一
- **Type 自己記述機構**：`Type` は言語レベルの特殊存在、シグネチャ中の `Type` 位置は自動推論で充填可能
- **型制約**：`T: Dup + Add` 複数制約、関数型制約
- **関連型**：`Iterator: (Item: Type) -> Type = { next: () -> Option(Item), has_next: () -> Bool }`
- **コンパイル時ジェネリック**：`N: Int` ジェネリック値引数、コンパイル時定数インスタンス化
- **条件型**：`If: (C: Bool, T: Type, E: Type) -> Type` 型レベル計算、型族

**価値**：
- ゼロコスト抽象化：コンパイル時単態化、ランタイムオーバーヘッドなし
- デッドコードElimination：インスタンス化グラフ分析 + LLVM最適化
- マクロ代替：ジェネリックでマクロ使用シナリオの90%を代替
- 型安全性：コンパイル時チェック、IDEフレンドリー
- **明示的优于隐式**：`Type` 自己記述、コンパイラ自動推論

## 参照文書

本文書の設計は以下の文書に基づいている：

| 文書 | 関係 | 説明 |
|------|------|------|
| [RFC-010: 統一型構文](./010-unified-type-syntax.md) | **構文基盤** | ジェネリック構文と統一 `name: type = value` モデルの統合 |
| [RFC-010: 統一型構文](./010-unified-type-syntax.md) | **呼び出し構文** | 第6節：ジェネリック呼び出し構文 - 統一 `()` 適用、`[]` 完全削除 |
| [RFC-009: 所有権モデル](./accepted/009-ownership-model.md) | **型システム** | Moveセマンティクスとジェネリックの自然な結合 |
| [RFC-024: spawnベースの並行ランタイムセマンティクス](./024-concurrency-model.md) | **実行モデル** | DAG分析とジェネリック型チェック |
| [RFC-008: ランタイムモデル](./accepted/008-runtime-concurrency-model.md) | **コンパイラアーキテクチャ** | ジェネリック単態化とコンパイル時最適化戦略 |
| [型宇宙の思想](../reference/plan/ongoing/型宇宙の思想.md) | **理論コア** | 型宇宙階層モデルと値依存型設計 |
| [RFC-027: コンパイル時述語と統一静的検証](./027-compile-time-evaluation-types.md) | **終了チェック** | decreases仕様とコンパイル時評価安全保障 |

## 型宇宙の思想と値依存型

YaoXiangのジェネリックシステムは**型宇宙の思想**に基づいており、このメンタルモデルは言語内のすべての概念を階層構造に統一し、コアイノベーションは**値依存型**をType2層の一級市民として昇華させた点にある。

### 値依存型とは？

**値依存型**とは、1つまたは複数の**値**（型だけでなく）に依存する型のことを言う。これらの値はコンパイル時に評価可能であり、 컴파일 단계에서 타입 안전성을 보장할 수 있다.

```yaoxiang
# 従来のジェネリック：型パラメータ
List: (T: Type) -> Type

# 値依存型：値パラメータ
Vec: (n: Int) -> Type  # ベクトル型は長さ値 n に依存
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type  # 行列型は行数と列数に依存
```

### 値依存型のコア優位性

従来のジェネリックと比較して、YaoXiangの値依存型は以下のコア優位性を持つ：

| 特性 | 従来のジェネリック (C++/Rust) | YaoXiang 値依存型 |
|------|-------------------|---------------------|
| 型が依存する値 | 型パラメータのみに依存 | 関数呼び出し結果を含む任意の値に依存可能 |
| コンパイル時評価 | C++テンプレート手動特殊化、Rustなし | 自動コンパイル時評価、終了保証 |
| 型レベル計算 | テンプレートメタプログラミング（複雑/危険） | 統一型レベル計算エンジン |
| 型安全性 | C++なし、Rust制限的 | 完全型安全性、コンパイル時チェック |
| 次元検証 | ランタイムチェックまたは手動特殊化 | コンパイル時次元検証、ランタイムオーバーヘッドなし |

### 型宇宙階層と値依存型

型宇宙の思想は言語概念をセマンティック役割別に異なる階層に分類し、値依存型は **Type2層** に位置する：

| 階層 | 役割 | 例 |
|------|------|------|
| Type-1 | 値 | `42`, `factorial(5)`, 関数自体 |
| Type0 | メタ型キーワード | `Type` |
| Type1 | 具体型 | `Int`, `String`, `Vec(3)` |
| **Type2** | **関数/型構築子/値依存型** | `add: (Int, Int) -> Int`, `Vec: (n: Int) -> Type`, `Matrix: (T: Type, Rows: Int, Cols: Int) -> Type` |

**主要設計**：Type2層の関数、型構築子、値依存型は**統一構文**で、すべて `(params) -> result` の形式：
- 通常関数：`(Int, Int) -> Int` → 戻り値は値
- 型構築子：`(T: Type) -> Type` → 戻り値は型
- 値依存型：`(n: Int) -> Type` → 戻り値は型だが値引数に依存

> **Curry-Howard同型**：この統一は偶然ではない。Curry-Howard同型は「型は命題、プログラムは証明」であることを示す——関数型 `A → B` は論理包含「AならばB」に対応し、ジェネリック `(T: Type) -> Type` は全称量化「すべての型Tについて」に対応し、値依存型 `(n: Int) -> Type` は「各整数nに対して型が存在する」に対応する。YaoXiangが関数、型構築子、値依存型をType2層に統一することは、本質的に「証明」と「計算」を同一概念——**構成的証明**——に統一することである。これはCurry-Howard同型の言語設計への直接的反映である：1つの形式（`(params) -> result`）が論理命題と計算過程を同時に担う。

### コンパイル時決定性保証

YaoXiangの型宇宙の思想は要求する：**Type階層的一切はコンパイル時に決定される**。

```yaoxiang
# コンパイル時次元検証の例
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),
    # コンパイル時チェック：次元は正でなければならない
    _assert: Assert[Rows > 0],
    _assert: Assert[Cols > 0],
}

# 3x3単位行列の作成 - コンパイル時に完了
identity: (T: Add + Zero + One, N: Int) -> ((size: N) -> Matrix(T, N, N)) = {
    matrix = Matrix(T, N, N)()
    # ...
}

# コンパイル時計算：factorial(3) = 6、ベクトルサイズはコンパイル時に決定
vec: Vec(factorial(3)) = Vec(6)()
```

コンパイラは自動的に：
1. 型位置の関数呼び出しを検出
2. 関数に `decreases` 仕様があるかを検証（下部の終了チェック機構を参照）
3. コンパイル時に評価を実行
4. 結果を生成された型に埋め込み

### 値依存型の適用シナリオ

#### コンパイル時次元検証
```yaoxiang
# 行列乗算：コンパイル時に次元一致を検証
multiply: (T: Add + Multiply + Zero,
           Rows: Int, Cols: Int, M: Int) -> ((
    a: Matrix(T, Rows, Cols),
    b: Matrix(T, Cols, M)
) -> Matrix(T, Rows, M)) = {
    # コンパイル時チェック：a.Cols == b.Rows，否则编译错误
    result = Matrix(T, Rows, M)()
    # ...
}

# エラーはコンパイル時に捕获：
# multiply(matrix_2x3, matrix_4x2)  # コンパイルエラー：2 != 4
```

#### 型安全な配列サイズ
```yaoxiang
# 配列サイズはコンパイル時定数
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    length: N,
}

# N はコンパイル時定数で、型レベル計算に使用可能
first_three: Array(Int, 3) = Array(Int, 3)(1, 2, 3)
# first_three.length == 3（コンパイル時既知）
```

#### 条件型
```yaoxiang
# 型レベルIf
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
# map: ジェネリック関数、型パラメータ T, R はコンパイル時に決定
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
doubled = map(numbers, (x) => x * 2)  # map[Int, Int] に推論
```

### 他言語との比較

| 特性 | C++テンプレート | Rustジェネリック | Haskell GADT | **YaoXiang** |
|------|---------|----------|--------------|--------------|
| 型パラメータ | ✅ | ✅ | ✅ | ✅ |
| 値依存型 | ❌ | ❌ | ✅ | ✅ |
| コンパイル時評価 | テンプレートインスタンス化 | ❌ | ✅ | ✅ |
| 終了保証 | ❌ | ❌ | ❌（危険） | ✅（decreases仕様） |
| 型安全性 | ❌（マクロ展開） | ✅ | ✅ | ✅ |
| 統一構文 | ❌ | ❌ | ❌ | ✅ |
| コンパイル時次元検証 | 手動特殊化 | ランタイムチェック | 型族 | コンパイル時自動検証 |
| decreases仕様 | ❌ | ❌ | ❌ | ✅ |

### 終了チェック機構（RFC-022との統合）

値依存型のコンパイル時評価は**終了を保証**しなければならない、さもなくば型システムは無限ループに陥る。YaoXiangは **decreases仕様** を通じてこれを保証し、RFC-022とシームレスに統合する。

#### 再帰関数の終了仕様
```yaoxiang
# コンパイル時階乗：終了を証明する必要がある
factorial: (n: Int) -> Int = {
    //! requires: n >= 0
    //! ensures: result == n!
    //! decreases: n    # 各再帰で n は厳密に減少
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

# 使用：型位置で呼び出し
vec: Vec(factorial(5)) = Vec(120)()  # コンパイル時に factorial(5) = 120 を評価
```

#### ループの終了仕様
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

#### 終了チェックのワークフロー

```
┌─────────────────────────────────────────────────────────────┐
│  型チェックフェーズ                                            │
│  型位置の関数呼び出しに遭遇（例：Vec(factorial(5))）            │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  1. decreases 仕様のチェック                                   │
│     - decreasesあり：全再帰パスで減少条件が成立することを検証    │
│     - decreasesなしだが明らかに終了：直接評価                  │
│     - decreasesなしで終了しない可能性：コンパイルエラー         │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  2. コンパイル時評価（ビルトインインタプリタが実行）             │
│     - 純粋関数：直接評価                                      │
│     - 副作用あり：コンパイルエラー（型位置は副作用禁止）         │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  3. 結果の型への埋め込み                                        │
│     - Vec(factorial(5)) → Vec(120)                          │
│     - Matrix(Float, 3, 3) → 具体型                           │
└─────────────────────────────────────────────────────────────┘
```

#### 優位性

- **安全性**：コンパイル時評価の終了を保証し、型システムの無限ループを回避
- **統一性**：終了チェックと部分的正当性検証は同じ仕様機構を共有
- **漸進的強化**：ランタイムチェックから完全静的証明へ徐々に移行可能

## 動機

### なぜ強力なジェネリックシステムが必要か？

現在の主流言語のジェネリックには限界がある：

| 言語 | ジェネリック機能 | 問題 |
|------|----------|------|
| Java | 境界型 | コンパイル時単態化、ジェネリック特殊化なし |
| C# | ジェネリック制約 | ランタイム型チェック、パフォーマンスオーバーヘッド |
| Rust | ジェネリック + Trait | Traitシステム複雑、学習曲線が急 |
| C++ | テンプレート | テンプレート特殊化複雑、コンパイルエラーメッセージが悪い |
| **YaoXiang** | **値依存型** | **型は値に依存可能、コンパイル時次元検証、終了保証** |

### コア矛盾

1. **パフォーマンス vs 柔軟性**：ランタイム柔軟性 vs コンパイル時最適化
2. **複雑さ vs 簡潔さ**：強力な型システム vs 使いやすさ
3. **マクロ vs ジェネリック**：マクロコード生成 vs ジェネリック型安全性
4. **値依存 vs 型安全性**：従来のジェネリックはコンパイル時に次元を検証できない

### 値依存型のコア優位性

YaoXiangの**値依存型**は従来のジェネリックに対するコア優位性である：

| 優位性 | 説明 |
|------|------|
| **型が値に依存** | `Vec: (n: Int) -> Type` で型は具体的な値に依存可能 |
| **コンパイル時評価** | 型位置の関数呼び出しはコンパイル時に評価され、結果を型に直接埋め込み |
| **次元検証** | `Matrix(Float, 3, 3)` はコンパイル時に行列次元を検証 |
| **型レベル計算** | `If`, `Match` などの条件型で型レベル計算をサポート |
| **終了保証** | decreases仕様でコンパイル時評価の終了を保証 |

```yaoxiang
# C++/Rust ではできないコンパイル時検証
matrix: Matrix(Float, factorial(3), factorial(2)) = ...
# コンパイル時計算：factorial(3) = 6, factorial(2) = 2
# 型は Matrix(Float, 6, 2)

# 次元不一致はコンパイル時に捕获
identity: Matrix(Float, 3, 3) = ...
# multiply(matrix_2x3, identity_3x3)  # コンパイルエラー：2 != 3
```

### ジェネリックシステムの価値

```yaoxiang
# 例：統一API設計
# 異なるコンテナ型のmap操作

# 従来方案：各型ごとに個別実装
map_int_array: (array: Array(Int), f: Fn(Int) -> Int) -> Array(Int) = ...
map_string_array: (array: Array(String), f: Fn(String) -> String) -> Array(String) = ...
map_int_list: (list: List(Int), f: Fn(Int) -> Int) -> List(Int) = ...
map_string_list: (list: List(String), f: Fn(String) -> String) -> List(String) = ...

# ジェネリック方案：1つのジェネリック関数で全型をカバー
map: (T: Type, R: Type)(container: Container(T), f: Fn(T) -> R) -> Container(R) = {
    for item in container {
        result.push(f(item))
    }
    result
}
```

## 設計目標

### コア目標

1. **ゼロコスト抽象化** - ジェネリック呼び出しは具体型呼び出しと等価
2. **デッドコードElimination** - コンパイル時分析、使用されるジェネリックのみインスタンス化
3. **マクロ代替** - ジェネリックでマクロ使用シナリオの90%を代替
4. **型安全性** - コンパイル時チェック、ランタイム型オーバーヘッドなし
5. **IDEフレンドリー** - インテリジェントヒント、明確なエラーメッセージ
6. **値依存型** - 型は値に依存可能、コンパイル時次元検証をサポート
7. **コンパイル時評価安全** - decreases仕様でコンパイル時評価の終了を保証

### 設計原則

- **コンパイル時決定**：ジェネリック引数はコンパイル時に決定
- **単態化優先**：具体コードを生成、仮想関数呼び出しを回避
- **制約駆動**：型制約がインスタンス化を指導
- **プラットフォーム最適化**：特殊化でプラットフォーム固有最適化をサポート
- **型宇宙の統一**：関数/型構築子/値依存型をType2層に統一
- **終了保証**：型位置の関数呼び出しは終了を証明する必要がある

## 提案

### 1. 基礎ジェネリック

#### 1.1 ジェネリック型パラメータ

> **重要ルール**：ジェネリック型定義は**必ず `: Type` を明示的に标注** 해야 하며、さもなくばHM推論により関数として扱われる。
>
> | 書き方 | 意味 |
> |------|------|
> | `List: (T: Type) -> Type = {...}` | ✅ 型構築子 |
> | `List = {...}` | ❌ HM推論により関数、型ではない |

```yaoxiang
# ジェネリック型定義（: Type 必須）
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
    push: (self: List(T), item: T) -> Void,   # self は約束名だけで、キーワードではない
    get: (self: List(T), index: Int) -> Option(T),
}

# ジェネリック関数（: Type なし、HM推論により関数）
map: (T: Type, R: Type) -> ((opt: Option(T), f: Fn(T) -> R) -> Option(R)) = {
    return match opt {
        some => Option.some(f(some)),
        none => Option.none(),
    }
}

# ジェネリック制約（直接式、1行ではreturn省略可能）
clone: (T: Clone)(value: T) -> T = value.clone()

# 複数型パラメータ
combine: (T: Type, U: Type) -> ((a: T, b: U) -> (T, U)) = (a, b)
```

### ジェネリック関数呼び出し構文

#### 1.1 統一シグネチャ構文

```yaoxiang
# ジェネリック関数は統一された (T: Type, R: Type) シグネチャ構文を使用
map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R)) = ...

# 複数型パラメータ
combine: (T: Type, U: Type) -> ((a: T, b: U) -> (T, U)) = (a, b)
```

#### 1.2 Type 自己記述機構

`Type` は言語レベルの特殊存在であり、コンパイラは本能的にシグネチャ中の `Type` 位置を認識し、実際の引数型から自動推論で充填する。

```yaoxiang
# コンパイラは自動的にジェネリック引数を推論
numbers: List(Int) = List(Int)
#         ^^^^^^^^   ^^^^^^
#         型宣言     構築呼び出し：Int が T を充填

# 関数呼び出しの推論
numbers: List(Int) = List(Int)
f: (x: Int) -> String = (x) => x.to_string()
strings: List(String) = map(numbers, f)
# コンパイラが推論：T=Int, R=String
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

# コンパイル後（等価コード）
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

#### 1.4 明示的充填（推論失敗時）

```yaoxiang
# 推論可能な場合は Type 引数を省略可能
numbers: List(Int) = List(Int)
strings: List(String) = map(numbers, (x: Int) => x.to_string())

# 推論不可能な場合は明示的に充填必須
# map(numbers, (x) => x)  # ❌ Error: Cannot infer R

### 2. 型制約システム

#### 2.1 単一制約

```yaoxiang
# 基本trait定義（インターフェース型）
Clone: Type = {
    clone: (Self) -> Self,
}

Display: Type = {
    fmt: (Self, Formatter) -> Result,
}

Debug: Type = {
    fmt: (Self, Formatter) -> Result,
}

# 制約を使用：シグネチャで直接型制約を宣言
clone: (T: Clone) -> (value: T) -> T = value.clone()

debug_print: (T: Debug)(value: T) -> Void = {
    formatter = Formatter.new()
    value.fmt(formatter)
    print(formatter.to_string())
}
```

#### 2.2 複数制約

```yaoxiang
# 複数制約構文
combine: (T: Clone + Add)(a: T, b: T) -> T = {
    a.clone() + b
}

# ジェネリックコンテナのソート
sort: (T: Clone + PartialOrd)(list: List(T)) -> List(T) = {
    # ソートアルゴリズム実装
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

#### 2.4 組み込み marker trait：Dup と Clone

**3種類のコピーセマンティクス**：

| 型 | 意味 | トリガー方式 | 適用シナリオ |
|------|------|----------|----------|
| **原語値コピー** | 代入時に自動値コピー、2つの値は完全に独立 | 代入/引数渡しの自動 | Int, Float, Bool, Char |
| **Dup** | シャローコピー：ハンドラ/トークンをコピー、基底データは共有 | 代入/引数渡しの自動 | `&T` トークン、`ref T`、String/Bytes |
| **Clone** | ディープコピー：完全に独立したコピーを作成 | `value.clone()` | Cloneを実装する任意の方 |

**Dupのセマンティクス**：Dupを実装した型は代入/引数渡し時に所有権を移動しない——コンパイラはハンドラ/トークンをコピーし、複数の所有者が同一の基底データを指す。これはRFC-009所有権モデルのMoveデフォルトセマンティクスの補完である。

**DupとCloneは直交する概念である**：

```
Dup = ハンドラをコピー、データを共有（変更は互いに影響）
Clone = データをコピー、コピーは独立（変更は互いに影響しない）
```

**ルール**：

```
1. 原語値型（Int, Float, Bool, Char） — コンパイラの組み込み値コピー、Dupには属さない
2. Dup  — 参照/トークン型と内部参照カウントの型にのみ適用
3. Clone — 明示的ディープコピー、任意の方が実装可能
4. デフォルト Move — その他の型はデフォルト Move セマンティクスを維持
```

**哪些类型是 Dup**：

| 型 | Dup | 原因 |
|------|-----|------|
| `&T`（借用トークン） | ✅ | ゼロサイズトークン、トークンコピー = 同一データを指す複数の視点 |
| `ref T` | ✅ | Rc/Arc コピー = 参照カウント+1、ヒープデータを共有 |
| String, Bytes | ✅ | 内部参照カウント、ハンドラコピーにより基底bufferを共有 |
| `&mut T`（可変トークン） | ❌ | 線形独占、コピー不可 |
| struct | 派生 | 全フィールド Dup → struct Dup |
| enum | 派生 | 全variantの全フィールド Dup → enum Dup |
| tuple | 派生 | 全要素 Dup → tuple Dup |
| Fn（クロージャ） | ❌ | キャプチャ環境にDupでないものが含まれる可能性 |
| `*T`（裸ポインタ） | ❌ | unsafe、所有権システムに参加しない |

**Int/Float/Bool/Char は Dup ではない**——これらは値型であり、代入時にコンパイラが自動的に値コピーする（2つの値は完全に独立）。これは「シャローコピー」ではなく、コンパイラの原語に対する組み込み処理であり、Dup型属性で表現する必要もすべきでもない。

```yaoxiang
# 原語値型：コンパイラが自動的に値コピー（Dupではない）
x: Int = 42
y = x          # 値コピー、x と y は完全に独立
print(x)       # ✅

# Dup：シャローコピー、ハンドラコピーによりデータを共有
view: &Point = &point
view2 = view    # ✅ Dup：トークンコピー、両者とも同一の point を指す
print(view.x)   # ✅

# Clone：明示的ディープコピー、独立コピーを作成
backup = big_struct.clone()  # 明示的呼び出し

# ジェネリック制約
dup_use: (T: Dup) -> T = x         # T: Dup → シャローコピー可能
clone_use: (T: Clone) -> T = x.clone()  # T: Clone → ディープコピー可能
```

> **注意**：`Send`/`Sync` はユーザー可见traitとしては提供しない。タスク間安全保障は `ref` キーワードとコンパイラの完全自動処理により保障——`ref` は Rc または Arc を自動選択し、ユーザーは Send/Sync を理解する必要がない。

### 3. 関連型

#### 3.1 関連型定義

```yaoxiang
# Iterator trait（(Item: Type) -> Type 構文を使用）
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

# ArrayのIterator実装
# メソッド構文糖衣を使用：Array.Item, Array.next, Array.has_next
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
# より複雑な関連型
Producer: (Item: Type) -> Type = {
    Item: T,
    produce: (Self) -> Option(Item),
}

# 関連型はジェネリックに可能
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

### 4. コンパイル時ジェネリック

#### 4.1 コンパイル時定数パラメータ

**コア設計**：ジェネリックシグネチャ中の `Type` はコンパイル時型パラメータを标记し、`Int` などの値引数はジェネリックコンテキストではデフォルトでコンパイル時に確定可能である。`const` キーワードは不要。

```yaoxiang
# ════════════════════════════════════════════════════════
# コンパイル時定数パラメータ：ジェネリック中の Int はデフォルトでコンパイル時に確定
# ════════════════════════════════════════════════════════

# コンパイル時階乗：N はコンパイル時既知のリテラルである必要がある
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

# 使用方法
arr: StaticArray(Int, factorial(5))  # StaticArray(Int, 120)、コンパイラがコンパイル時に計算
```

#### 4.2 コンパイル時計算

```yaoxiang
# ════════════════════════════════════════════════════════
# コンパイル時計算の例
# ════════════════════════════════════════════════════════

# コンパイラはコンパイル時にリテラル型の関数呼び出しを計算
SIZE: Int = factorial(5)  # コンパイル時は 120

# 行列型を使用
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

# 使用：コンパイル時に計算し、Matrix(Float, 3, 3) を生成
identity_3x3: Matrix(Float, 3, 3) = identity_matrix(Float, 3)(3)
```


### Never と Void：型システムの ⊥ と ⊤

YaoXiangの型システムはCurry-Howard同型において⊥（偽/空型）と⊤（真/Unit）の両方を同時に備え、`Never` と `Void` の2つの組み込み型名で表現する：

**Never（⊥）** — 3つの交渉不可能な内核性質：

1. **ゼロコンストラクタ**：任何のリテラルや式も `Never` 型の値を生成できない。これはメタレベル性質であり、組み込み必要。
2. **爆発原理**：`Never <: T` は任意の方 `T` に対して成立する。`Never` 値は任意の方として使用可能——これが `assert(false)` 以降のコードが型チェックを通過する理由（実行されることはないが）。
3. **発散マーク**：`f: (...) -> Never` は `f` が返さないことを保証する。コンパイラはこの情報を基にdead code分析を行う。

`Never` は組み込み型名でありキーワードではなく、parserには無感知である。空和型リテラル構文は開放しない。

**Void（⊤、即ちUnit）** — 丁度1つの住人（デフォルトvoid値）を持ち、真命题「恒真」の担体である。`Void` はゼロフィールド積型の単位元であり、`Never` はゼロvariant和型の単位元である——二者に対偶関係がある。`x: Void = <デフォルト>` は合法であり、`x: Never = ...` は右辺に书写できるものが何もない。

#### 4.3 コンパイル時検証（標準ライブラリ実装）

```yaoxiang
# ════════════════════════════════════════════════════════
# 標準ライブラリ実装：条件型を利用
# ════════════════════════════════════════════════════════

# 標準ライブラリ定義
# IsTrue：値宇宙から型宇宙への橋——Bool 真値を型にマッピング
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      # ⊤、値あり、プログラム続行
    false => Never,    # ⊥、値なし、発散
}

# Assert：コンパイル時精製型プリミティブ——Bool 命題の型レベル記述
Assert: (cond: Bool) -> Type = IsTrue(cond)
#
# cond が true  → Assert(true)  = Void    （恒真、擦拭）
# cond が false → Assert(false) = Never   （恒偽、コンパイルエラー/発散）
# cond が判断不能 → 証明管道が dispatch モードに応じて決定：
#                  CompileTime → Unknown、prove 要求
#                  Runtime     → check を挿入、Γ 仮定を注入

# 使用方法1：型定義で制約として
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    # コンパイル時チェック：N は 0 より大きくなければならない（Assert は型位置）
    length: Assert(N > 0),
}

# 使用方法2：式で中使用
IntArray: (N: Int) -> Type = StaticArray(Int, N)
# 検証：IntArray(10) のサイズは sizeof(Int) * 10 と等しい
Assert(size_of(IntArray(10)) == sizeof(Int) * 10)
```

#### 4.4 コンパイル時ジェネリック特殊化

```yaoxiang
# 小配列最適化：関数オーバーロードでコンパイル時ジェネリック特殊化を実装

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

# 小配列ループ展開（N <= 4）
sum: (T: Type, N: Int) -> ((arr: Array(T, N)) -> T) = {
    # コンパイラ最適化：ループを展開
    return arr.data[0] + arr.data[1] + arr.data[2] + arr.data[3]
}
```

### 5. 条件型

> **Curry-Howard同型**：条件型はCurry-Howardの視点からは論理における **case分析** である。`Bool` 型は2つの可能な値を持つ命題（True/False）に対応し、`If` はその命題の真偽に応じて異なる結果を選択する——これは論理におけるcase選言に他ならない。`match C { True => T, False => E }` は实际上、「命題 C が True の時結論は T、C が False の時結論は E」を表現している。

#### 5.1 If条件型

```yaoxiang
# 型レベルIf
If: (C: Bool, T: Type, E: Type) -> Type = match C {
    True => T,
    False => E,
}

# 例：コンパイル時分岐
NonEmpty: (T: Type) -> Type = If(T != Void, T, Never)

Optional: (T: Type) -> Type = If(T != Void, T, Void)

# コンパイル時検証（§4.3 の Assert 定義に統一）
# Assert: (cond: Bool) -> Type = IsTrue(cond)

# 使用
# 型計算：If(True, Int, String) => Int
# 型計算：If(False, Int, String) => String
```

#### 5.2 型族

> **Curry-Howard同型**：型族は「命題即型」最も直接的な体现である。`Add: (A: Type, B: Type) -> Type` は「型レベルで加算関数を記述した」ではなく、**自然数加算に関する命題を構成している**。(Zero, B) => B は「命題 Add(Zero, B) は B と同値である」ことを言い、(Succ(A'), B) => Succ(Add(A', B)) は「Add(A', B) が成立するなら、Add(Succ(A'), B) も成立する」ことを言う。これはPeano公理における加算定義そのものである。型チェッカーがこのmatch式を検証することは、この定義の論理的一貫性を検証することと等価である。

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

# 型レベル加算（Curry-Howard：case analysis + 再帰呼び出し、終了性チェックが必要で完全帰納法）
Add: (A: Type, B: Type) -> Type = match (A, B) {
    (Zero, B) => B,
    (Succ(A'), B) => Succ(Add(A', B)),
}

# 例：コンパイル時に 2 + 3 を計算
Two: Type = Succ(Succ(Zero))
Three: Type = Succ(Succ(Succ(Zero)))
Five: Type = Add[Two, Three]  # Succ(Succ(Succ(Succ(Succ(Zero)))))
```

### 6. 関数オーバーロード特殊化

#### 6.1 基本特殊化

```yaoxiang
# 基本特殊化：関数オーバーロードを使用（コンパイラが自動選択）
sum: (arr: Array(Int)) -> Int = {
    # より効率的なコードにコンパイル
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Array(Float)) -> Float = {
    # SIMD命令を使用
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
# RFC-010構文に完全準拠した特殊化方式：関数オーバーロード

# 具体型特殊化
sum: (arr: Array(Int)) -> Int = {
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Array(Float)) -> Float = {
    return simd_sum_float(arr.data, arr.length)
}

# ジェネリック実装（コンパイラが自動的に最適を選択）
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

# コンパイラが自動的に最適特殊化を選択
sum(int_arr)     # sum: (Array(Int)) -> Int を選択
sum(float_arr)    # sum: (Array(Float)) -> Float を選択
```

#### 6.3 関数オーバーロードとインライン最適化の見事な結合

**主要特性**：関数オーバーロードとインライン最適化は自然に結合し、ゼロコスト抽象化を実現する。

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

# ======== コンパイル後（等価コード）=======
# コンパイラが自動的に最適特殊化を選択 ثمインライン展開
result = native_sum_int(int_arr.data, int_arr.length)

# 手書き最適化コードと完全に等価、関数呼び出しオーバーヘッドなし！
```

**コア優位性**：

1. **コンパイラのインテリジェント選択**
   ```yaoxiang
   sum(int_arr)      # 自動的に sum: (Array(Int)) -> Int を選択
   sum(float_arr)    # 自動的に sum: (Array(Float)) -> Float を選択
   sum(custom_arr)  # 自動的に sum: (T: Type) -> ((arr: Array(T)) -> T) を選択
   ```

2. **インライン最適化**
   - 小関数は自動的に呼び出し点にインライン展開
   - 関数呼び出しオーバーヘッドゼロ
   - 手書き最適化コードと完全に等価

3. **型安全性**
   - コンパイル時型チェック
   - ランタイムオーバーヘッドゼロ
   - 仮想関数テーブル不要

4. **RFC-010との完璧な一致**
   ```yaoxiang
   # 統一構文を完全使用
   name: type = value
   # impl、where などの新キーワード不要
   ```

**実際の応用例**：

```yaoxiang
# パフォーマンスが重要な数値計算
fibonacci: (n: Int) -> Int = {
    if n <= 1 { return n }
    return fibonacci(n - 1) + fibonacci(n - 2)
}

fibonacci: (n: Float) -> Float = {
    # Binetの公式を使用
    phi = (1.0 + 5.0.sqrt()) / 2.0
    return (phi.pow(n) - (-phi).pow(-n)) / 5.0.sqrt()
}

# コンパイラが自動的に選択してインライン展開
fibonacci(10)      # Intバージョンを選択、完全インライン展開
fibonacci(10.5)    # Floatバージョンを選択、Binetの公式を使用
```

**これ意味着什么？**

- ✅ **ジェネリック特殊化** → 関数オーバーロードで自然に解決
- ✅ **パフォーマンス最適化** → インライン展開が自動的に完了
- ✅ **コード再利用** → 1つの関数名、複数の実装
- ✅ **ゼロコスト抽象化** → コンパイル時多态、ランタイムオーバーヘッドなし
- ✅ **新キーワード不要** → RFC-010統一構文に完璧に準拠
```

### 7. デッドコードElimination機構

#### 7.1 インスタンス化グラフ分析

```rust
// コンパイラの内部：ジェネリックインスタンス化依存グラフを構築
struct InstantiationGraph {
    // ノード：ジェネリックインスタンス化
    nodes: HashMap<InstanceKey, InstanceNode>,

    // エッジ：使用関係
    edges: HashMap<InstanceKey, Vec<InstanceKey>>,
}

struct InstanceKey {
    generic: FunctionId,  // ジェネリック関数ID
    type_args: Vec<TypeId>,  // 型引数
    const_args: Vec<ConstId>,  // Const引数
}

// アルゴリズム：到達可能性分析
fn eliminate_dead_instantiations(graph: &InstantiationGraph) {
    let mut reachable = HashSet::new();

    // エントリポイントから開始（main、エクスポート関数など）
    let entry_points = find_entry_points();
    for entry in entry_points {
        dfs_visit(entry, &graph, &mut reachable);
    }

    // 未訪問のインスタンス化はデッドコード
    for node in &graph.nodes {
        if !reachable.contains(node.key) {
            eliminate(node);
        }
    }
}
```

#### 7.2 使用箇所分析

```yaoxiang
# ソースコード分析
map: (T: Type, R: Type)(list: List(T), f: Fn(T) -> R) -> List(R) = ...

# 使用箇所1：map(Int, Int) をインスタンス化
int_list = List(1, 2, 3)
doubled = map(int_list, (x) => x * 2)  # map[Int, Int] が必要

# 使用箇所2：map(String, String) をインスタンス化
string_list = List("a", "b", "c")
uppercased = map(string_list, (s) => s.to_uppercase())  # map[String, String] が必要

# 未使用：map[Float, Float] など
# これらのジェネリックインスタンスは生成されない

# コンパイル後、使用されたインスタンスのみ含む
map_Int_Int: (list: List(Int), f: Fn(Int) -> Int) -> List(Int) = ...
map_String_String: (list: List(String), f: Fn(String) -> String) -> List(String) = ...
```

#### 7.3 コンパイル時ジェネリックDCE

```yaoxiang
# コンパイル時分析：コンパイル時ジェネRIC使用状況
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
}

# 実際の使用状況
arr_10_int = Array(Int, 10)(1, 2, 3, 4, 5, 6, 7, 8, 9, 10)
arr_100_int = Array(Int, 100)(...)

# コンパイル後、 사용된 Size のみ生成
Array_Int_10: (Array(Int, 10)) = ...
Array_Int_100: (Array(Int, 100)) = ...

# 未使用のSizeは生成されない
# Array(Int, 50) は生成されない
```

#### 7.4 クロスモジュールDCE

```yaoxiang
# モジュールA
# A.yx
pub map: (T: Type, R: Type)(list: List(T), f: Fn(T) -> R) -> List(R) = ...

# モジュールB
# B.yx
use A.{map}
int_list = List(1, 2, 3)
doubled = map(int_list, (x) => x * 2)  # map(Int, Int) をインスタンス化

# モジュールC
# C.yx
use A.{map}
string_list = List("a", "b", "c")
uppercased = map(string_list, (s) => s.to_uppercase())  # map(String, String) をインスタンス化

# コンパイル分析：
# - モジュールBは map[Int, Int] を使用
# - モジュールCは map[String, String] を使用
# - コンパイル後バイナリはこの2つのインスタンスのみ含む
```

#### 7.5 LLVMレベルDCE

```rust
// コンパイルパイプライン
fn optimize_ir(ir: &mut IR) {
    // 1. 単態化（YaoXiangコンパイラ）
    ir.monomorphize();

    // 2. インライン最適化
    ir.inline_small_functions();

    // 3. 定数伝播
    ir.constant_propagation();

    // 4. LLVM IRを生成
    let llvm_ir = ir.to_llvm();

    // 5. LLVM最適化パス
    llvm_ir.add_pass(Passes::DEAD_CODE_ELIMINATION);
    llvm_ir.add_pass(Passes::INLINE_FUNCTION);
    llvm_ir.add_pass(Passes::GLOBAL_DCE);
    llvm_ir.add_pass(Passes::MERGE_FUNC);

    // 6. 最適化実行
    llvm_ir.run_optimization_passes();
}
```

### 8. マクロ代替策略

#### 8.1 コード生成代替

```yaoxiang
# ❌ マクロ方案：コード生成
macro_rules! impl_debug {
    ($($t:ty),*) => {
        $(impl Debug for $t {
            fn fmt(&self, f: &mut Formatter) -> Result {
                write!(f, "{:?}", self)
            }
        })*
    };
}

# ✅ ジェネリック方案：自動導出
# 関数オーバーロード 방식으로自動導出
debug_fmt: (T: fields...) -> ((self: Point(T)) -> String) = {
    return "Point { x: " + self.x.to_string() + ", y: " + self.y.to_string() + " }"
}

# 使用
p = Point { x: 1, y: 2 }
p.debug_fmt(&formatter)  # 呼び出しを自動生成
```

#### 8.2 DSL代替

```yaoxiang
# ❌ マクロ方案：HTML DSL
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

# ✅ ジェネリック方案：型安全ビルダー
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

# DOM構築
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
# ❌ マクロ方案：型レベル計算
macro_rules! add_types {
    ($a:ty, $b:ty) => {
        ($a, $b)
    };
}

# ✅ ジェネリック方案：条件型
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
result_type = Add[Int, Float]  # Float に推論
```

### 9. 例

#### 9.1 完全ジェネリックコンテナ例

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

    # ジェネリックメソッド（T は外側の List(T) から自動的にスコープに導入）
    push: (self: List(T), item: T) -> Void,
    pop: (self: List(T)) -> Option(T),
    map: (R: Type) -> ((self: List(T), f: (T) -> R) -> List(R)),
    filter: (self: List(T), predicate: (T) -> Bool) -> List(T),
    fold: (U: Type) -> ((self: List(T), initial: U, f: (U, T) -> U) -> U),
}

# ======== 2. ジェネリックメソッド実装 ========
# 関数定義は List 名前空間下（List. プレフィックス = 名前空間帰属）
# list.push(item) のような . 呼び出し構文を動作させるには、明示的バインディングが必要：List.push = push[0]
# self は約束パラメータ名に過ぎず、コンパイラは名前ではなく型を見る

List.push: (T: Type) -> ((self: List(T), item: T) -> Void) = {
    if self.length >= self.data.length {
        # 容量拡張
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
# List に Clone を実装
List.clone: (T: Clone) -> ((self: List(T)) -> List(T)) = {
    result = List(T)()
    for i in 0..self.length {
        result.push(self.data[i].clone())
    }
    return result
}

# ======== 4. 使用例 ========
# ジェネリックListを作成
numbers = List(Int)()
numbers.push(1)
numbers.push(2)
numbers.push(3)

# ジェネリックメソッドを使用
doubled = numbers.map((x) => x * 2)
evens = numbers.filter((x) => x % 2 == 0)

# foldで計算
sum = numbers.fold(0, (acc, x) => acc + x)  # sum = 6

# ジェネリック合成
sum_of_evens = numbers
    .filter((x) => x % 2 == 0)
    .map((x) => x * 2)
    .fold(0, (acc, x) => acc + x)  # sum_of_evens = 8
```

#### 9.2 ジェネリックアルゴリズム例

```yaoxiang
# ======== 1. ジェネリックソートアルゴリズム ========
Comparator: (T: Type) -> Type = {
    compare: (T, T) -> Int,  # -1 if a < b, 0 if a == b, 1 if a > b
}

# ジェネリックquicksort
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

# ======== 2. IntComparator実装 ========
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
# Int配列をソート
numbers = Array(Int)(3, 1, 4, 1, 5, 9, 2, 6)
sorted = quicksort(numbers, Comparator(Int)())

# String配列をソート（StringComparatorが必要）
strings = Array(String)("hello", "world", "foo", "bar")
sorted_strings = quicksort(strings, Comparator(String)())
```

#### 9.3 コンパイル時ジェネリック例

```yaoxiang
# ======== 1. コンパイル時行列型 ========
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),

    # コンパイル時次元検証：Assert 標準ライブラリ型を利用
    _assert: Assert[Rows > 0],  # Rows > 0、さもなくばコンパイルエラー
    _assert: Assert[Cols > 0],  # Cols > 0、さもなくばコンパイルエラー

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

# 行列乗算：2x3 * 3x2 = 2x2
result = matrix_2x3.multiply(matrix_3x2)

# コンパイル時検証：result型は Matrix(Float, 2, 2)
# 2x2 単位行列
identity_3x3 = identity(Float, 3)()

# 次元不一致：コンパイルエラー
# bad_multiply = matrix_2x3.multiply(identity_3x3)  # コンパイルエラー：3x3 != 2x3
```

## トレードオフ

### 優位性

1. **ゼロコスト抽象化**
   - コンパイル時単態化、ランタイムオーバーヘッドなし
   - 仮想関数不要、RTTI不要

2. **デッドコードElimination**
   - コンパイル時分析、使用されるジェネリックののみインスタンス化
   - コード膨張は制御可能

3. **マクロ代替**
   - 型安全なコード生成
   - IDEフレンドリー、エラーメッセージが明確

4. **コンパイル時計算**
   - コンパイル時ジェネリックがコンパイル時計算をサポート
   - 次元検証などの特性
   - `const` キーワード不要、純粋な型制約

### 劣位性

1. **コンパイル時間**
   - ジェネリックインスタンス化はコンパイル時間を増加
   - 制約解決は比較的遅い可能性

2. **メモリ使用量**
   - コンパイラのメモリ使用量が増加
   - キャッシュ機構はメモリを消費

3. **実装複雑さ**
   - 制約解決器が複雑
   - 型レベル計算エンジンが複雑

4. **エラー診断**
   - ジェネリックエラーは複雑な可能性
   - 明確なエラーヒントが必要

### 緩和措施

1. **キャッシュ戦略**
   - インスタンス化結果をキャッシュ
   - LRUキャッシュでメモリ制限

2. **インクリメンタルコンパイル**
   - コンパイル結果をキャッシュ
   - インクリメンタルインスタンス化

3. **エラーヒント**
   - 明確なエラーメッセージ
   - ジェネリックパラメータ推論ヒント

4. **並行コンパイル**
   - ジェネリックを並行インスタンス化
   - マルチスレッド制約解決

## 代替方案

| 方案 | 選択しない理由 |
|------|--------------|
| のみ基礎ジェネリック | 複雑なマクロを代替できない |
| 純粋マクロシステム | 型安全性がなく、エラーメッセージが悪い |
| のみ制約に依存 | 柔軟性が不足 |
| ランタイムジェネリック | パフォーマンスオーバーヘッドあり |

### リスク

| リスク | 影響 | 緩和措施 |
|------|------|----------|
| 制約解決複雑さ | コンパイル時間が長すぎる | インクリメンタル解決 + キャッシュ |
| コード膨張 | バイナリファイルが大きすぎる | DCE + 閾値制御 |
| 実装複雑さ | 開発周期が延長 | フェーズ分け実装 |
| エラー診断 | ユーザー体験が悪い | 詳細なエラーメッセージ |

## 開放問題

### 未決議問題

| 議題 | 説明 | 状態 |
|------|------|------|
| インスタンス化戦略 | Eager vs Lazy vs Threshold | 議論待ち |
| キャッシュサイズ | LRUキャッシュ容量設定 | 議論待ち |
| エラー診断 | ジェネリックエラーメッセージ詳細度 | 議論待ち |

### 今後の最適化

| 最適化項目 | 価値 | 実装難易度 |
|--------|------|----------|
| インスタンス化グラフ分析 | 高 | 中 |
| 型レベルプログラミングDSL | 中 | 高 |
| ジェネリックパフォーマンスベンチマーク | 中 | 低 |

## 付録

### 構文BNF

```bnf
# ジェネリックパラメータは統一 () 構文を使用し、関数型の一部として
# 例：map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R))

# 型制約（ジェネリックパラメータ内）
type_bound ::= identifier
             | identifier '+' identifier ('+' identifier)*

# パラメータ宣言（型 + 名前）
parameter ::= identifier ':' type

parameters ::= parameter (',' parameter)*

# 関数宣言：name: type = expression
# ジェネリック引数は関数型の最初の引数グループ：(T: Type) -> ((params) -> return)
function ::= identifier ':' type '=' (expression | block)

# メソッド宣言：Type.method: type = expression
method ::= identifier '.' identifier ':' type '=' (expression | block)

# 型定義（統一 Binding 構文）
# ジェネリック型如 List: (T: Type) -> Type = { ... }
generic_type ::= identifier ':' type '=' type_expression

# ジェネリックパラメータ内の Type はコンパイラが実引数型から自動充填
# 例：map(numbers, f)、T は numbers: List(Int) から、R は f: (Int) -> String から抽出
```

## ライフサイクルと帰趨

```
┌─────────────┐
│   草案      │  ← 現在の状態
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  審査中     │  ← コミュニティ議論とフィードバックを開放
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  受入済み     │    │  拒否済み     │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   accepted/ │    │    rfc/     │
│ (正式設計)  │    │ (保存位置)  │
└─────────────┘    └─────────────┘
```

---

## 参考文献

### YaoXiang公式文書

- [RFC-010: 統一型構文](./010-unified-type-syntax.md)
- [RFC-009: 所有権モデル](./accepted/009-ownership-model.md)
- [RFC-001: spawnモデル](./accepted/001-concurrent-model-error-handling.md)
- [RFC-008: ランタイムモデル](./accepted/008-runtime-concurrency-model.md)
- [tutorial/ チュートリアル](../../../../../tutorial/)
### 外部参照

- [Rustジェネリックシステム](https://doc.rust-lang.org/book/ch10-01-syntax.html)
- [C++テンプレート特殊化](https://en.cppreference.com/w/cpp/language/template_specialization)
- [Haskell型クラス](https://www.haskell.org/tutorial/classes.html)
- [Swiftジェネリック](https://docs.swift.org/swift-book/LanguageGuide/Generics.html)
- [単態化最適化](https://llvm.org/docs/Monomorphization.html)
- [デッドコードElimination](https://en.wikipedia.org/wiki/Dead_code_elimination)
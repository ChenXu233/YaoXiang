---
title: "RFC-011: 泛型システム設計 - ゼロコスト抽象とマクロ代替"
status: "受け入れ済み"
author: "晨煦"
created: "2025-01-25"
updated: "2026-04-22（Type自己記述機構に更新、泛型呼び出し構文を統一）"
---

# RFC-011: 泛型システム設計 - ゼロコスト抽象とマクロ代替

## 要約

本文書はYaoXiang言語の**泛型システム設計**を定義する。強力な泛型能力によってゼロコスト抽象を実現し、コンパイル時最適化によってマクロへの依存を削減し、デッドコード除去機構を提供する。

**核心設計**：
- **統一シグネチャ構文**：`(T: Type, R: Type) -> ...` 泛型パラメータと通常のパラメータを統一
- **Type自己記述機構**：`Type` は言語レベルの特殊存在であり、シグネチャ中の `Type` の位置は自動推論で埋められる
- **型制約**：`T: Dup + Add` のような多重制約、関数型制約
- **関連型**：`Iterator: (Item: Type) -> Type = { next: () -> Option(Item), has_next: () -> Bool }`
- **コンパイル時泛型**：`N: Int` 泛型値パラメータ、コンパイル時定数インスタンス化
- **条件型**：`If: (C: Bool, T: Type, E: Type) -> Type` 型レベル計算、型族

**価値**：
- ゼロコスト抽象：コンパイル時単態化、ランタイムオーバーヘッドなし
- デッドコード除去：インスタンス化グラフ解析 + LLVM最適化
- マクロ代替：泛型がマクロ使用シーンの90%を代替
- 型安全性：コンパイル時検査、IDEフレンドリー
- **明示は暗黙に優る**：`Type` 自己記述、コンパイラが自動推論

## 参考文書

本文書の設計は以下の文書に基づいている：

| 文書 | 関係 | 説明 |
|------|------|------|
| [RFC-010: 統一型構文](./010-unified-type-syntax.md) | **構文の基礎** | 泛型構文と統一された `name: type = value` モデルの統合 |
| [RFC-010: 統一型構文](./010-unified-type-syntax.md) | **呼び出し構文** | 第6節：泛型呼び出し構文 — 統一された `()` 適用、`[]` は完全削除 |
| [RFC-009: 所有权モデル](./accepted/009-ownership-model.md) | **型システム** | Moveセマンティクスと泛型の自然な結合 |
| [RFC-001: 並行モデル](./accepted/001-concurrent-model-error-handling.md) | **実行モデル** | DAG解析と泛型型検査 |
| [RFC-008: ランタイムモデル](./accepted/008-runtime-concurrency-model.md) | **コンパイラアーキテクチャ** | 泛型単態化とコンパイル時最適化戦略 |
| [型宇宙思想](../reference/plan/ongoing/类型宇宙思想.md) | **理論的核** | 型宇宙階層モデルと値依存型設計 |
| [RFC-022: ホーア論理静的検証](./draft/022-hol-logic-verification.md) | **停止検査** | decreases仕様とコンパイル時評価の安全保証 |

## 型宇宙思想と値依存型

YaoXiang の泛型システムは**型宇宙思想**の上に構築されている。このメンタルモデルは言語のすべての概念を階層構造として統一し、中核的革新は**値依存型**をType2層の第一級市民に引き上げたことにある。

### 値依存型とは何か？

**値依存型**は、一つ以上の**値**（他の型だけでなく）に依存する型である。これらの値はコンパイル時に評価され、コンパイル段階で型安全保証を提供できる。

```yaoxiang
# 伝統的な泛型：型パラメータ
List: (T: Type) -> Type

# 値依存型：値パラメータ
Vec: (n: Int) -> Type  # ベクトル型は長さの値 n に依存する
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type  # 行列型は行数と列数に依存する
```

### 値依存型の核心的優位性

伝統的な泛型と比較して、YaoXiang の値依存型には以下の核心的優位性がある：

| 特性 | 伝統的な泛型 (C++/Rust) | YaoXiang 値依存型 |
|------|-------------------|---------------------|
| 型が依存する値 | 型パラメータのみ | 関数の呼び出し結果を含む任意の値に依存可能 |
| コンパイル時評価 | C++テンプレートの手動特殊化、Rust は不可 | 自動コンパイル時評価、停止を保証 |
| 型レベル計算 | テンプレートメタプログラミング（複雑/危険） | 統一された型レベル計算エンジン |
| 型安全性 | C++はなし、Rustは限定的 | 完全な型安全性、コンパイル時検査 |
| 次元検証 | ランタイム検査または手動特殊化 | コンパイル時次元検証、ランタイムオーバーヘッドなし |

### 型宇宙階層と値依存型

型宇宙思想は言語の概念を意味的役割によって異なる階層に分割し、値依存型は **Type2層** に位置する：

| 階層 | 役割 | 例 |
|------|------|------|
| Type-1 | 値 | `42`, `factorial(5)`, 関数そのもの |
| Type0 | メタ型キーワード | `Type` |
| Type1 | 具体型 | `Int`, `String`, `Vec(3)` |
| **Type2** | **関数/型コンストラクタ/値依存型** | `add: (Int, Int) -> Int`, `Vec: (n: Int) -> Type`, `Matrix: (T: Type, Rows: Int, Cols: Int) -> Type` |

**重要な設計**：Type2層の関数、型コンストラクタ、値依存型は**統一構文**であり、すべて `(params) -> result` の形式である：
- 通常関数：`(Int, Int) -> Int` → 戻り値は値
- 型コンストラクタ：`(T: Type) -> Type` → 戻り値は型
- 値依存型：`(n: Int) -> Type` → 戻り値は型だが、値パラメータに依存する

> **Curry-Howard同型**：この統一は偶然ではない。Curry-Howard同型は「型は命题、プログラムは証明」であることを示す — 関数型 `A → B` は論理的含意「A ならば B」に対応し、泛型 `(T: Type) -> Type` は全称量化「すべての型Tに対して」に対応し、値依存型 `(n: Int) -> Type` は「各整数nに対して型が存在する」に対応する。YaoXiang は関数、型コンストラクタ、値依存型をType2層に統一することで、本質的に「証明」と「計算」を同一の概念 — **構成的証明** — として統合している。これがまさに、Curry-Howard同型を言語設計に直接反映したものである：一つの形式 `(params) -> result` が論理的命题と計算過程を同時に担う。

### コンパイル時決定性の保証

YaoXiang の型宇宙思想は以下を要求する：**Type階層のすべてはコンパイル時に決定される**。

```yaoxiang
# コンパイル時次元検証の例
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),
    # コンパイル時検査：次元は正でなければならない
    _assert: Assert[Rows > 0],
    _assert: Assert[Cols > 0],
}

# 3x3単位行列を作成 — コンパイル時に完了
identity: (T: Add + Zero + One, N: Int) -> ((size: N) -> Matrix(T, N, N)) = {
    matrix = Matrix(T, N, N)()
    # ...
}

# コンパイル時計算：factorial(3) = 6、ベクトルサイズはコンパイル時に決定
vec: Vec(factorial(3)) = Vec(6)()
```

コンパイラは自動的に以下を行う：
1. 型位置での関数呼び出しを検出
2. 関数が `decreases` 仕様でマークされているかを検証（後述の停止検査機構を参照）
3. コンパイル時に評価を実行
4. 結果を生成された型に埋め込む

### 値依存型の応用シーン

#### コンパイル時次元検証
```yaoxiang
# 行列乗算：コンパイル時に次元一致を検証
multiply: (T: Add + Multiply + Zero,
           Rows: Int, Cols: Int, M: Int) -> ((
    a: Matrix(T, Rows, Cols),
    b: Matrix(T, Cols, M)
) -> Matrix(T, Rows, M)) = {
    # コンパイル時検査：a.Cols == b.Rows、そうでなければコンパイルエラー
    result = Matrix(T, Rows, M)()
    # ...
}

# エラーはコンパイル時に捕捉される：
# multiply(matrix_2x3, matrix_4x2)  # コンパイルエラー：2 != 4
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
# first_three.length == 3（コンパイル時に既知）
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

#### 泛型関数
```yaoxiang
# map: 泛型関数、型パラメータ T, R はコンパイル時に決定
map: (T: Type, R: Type) -> (
    (list: List(T), f: (x: T) -> R) -> List(R)
) = (list, f) => {
    result = List(R)()
    for x in list {
        result.push(f(x))
    }
    return result
}

# 使用時は完全に透明、型は自動推論される
numbers = List(1, 2, 3)
doubled = map(numbers, (x) => x * 2)  # map[Int, Int] と推論される
```

### 他言語との比較

| 特性 | C++テンプレート | Rust泛型 | Haskell GADT | **YaoXiang** |
|------|---------|----------|--------------|--------------|
| 型パラメータ | ✅ | ✅ | ✅ | ✅ |
| 値依存型 | ❌ | ❌ | ✅ | ✅ |
| コンパイル時評価 | テンプレートインスタンス化 | ❌ | ✅ | ✅ |
| 停止保証 | ❌ | ❌ | ❌（危険） | ✅（decreases仕様） |
| 型安全性 | ❌（マクロ展開） | ✅ | ✅ | ✅ |
| 統一構文 | ❌ | ❌ | ❌ | ✅ |
| コンパイル時次元検証 | 手動特殊化 | ランタイム検査 | 型族 | コンパイル時自動検証 |
| decreases仕様 | ❌ | ❌ | ❌ | ✅ |

### 停止検査機構（RFC-022との統合）

値依存型のコンパイル時評価は**停止を保証**しなければならない。さもないと型システムが無限ループに陥る。YaoXiang は **decreases仕様** によってこれを保証し、RFC-022 とシームレスに統合する。

#### 再帰関数の停止仕様
```yaoxiang
# コンパイル時階乗：停止を証明しなければならない
factorial: (n: Int) -> Int = {
    //! requires: n >= 0
    //! ensures: result == n!
    //! decreases: n    # 再帰ごとに n は厳密に減少する
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

# 使用例：型位置で呼び出し
vec: Vec(factorial(5)) = Vec(120)()  # コンパイル時に factorial(5) = 120 と評価
```

#### ループの停止仕様
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

#### 停止検査のワークフロー

```
┌─────────────────────────────────────────────────────────────┐
│  型検査段階                                                  │
│  型位置での関数呼び出しに遭遇（例：Vec(factorial(5))）       │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  1. decreases仕様を検査                                       │
│     - decreases あり：すべての再帰パスで減少条件を検証         │
│     - decreases なしだが明らかに停止する：直接評価              │
│     - decreases なし、かつ停止しない可能性：コンパイルエラー     │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  2. コンパイル時評価（組み込みインタプリタが実行）             │
│     - 純関数：直接評価                                        │
│     - 副作用：コンパイルエラー（型位置は副作用なしでなければ）  │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  3. 結果を型に埋め込み                                        │
│     - Vec(factorial(5)) → Vec(120)                          │
│     - Matrix(Float, 3, 3) → 具体型                            │
└─────────────────────────────────────────────────────────────┘
```

#### 優位性

- **安全性**：コンパイル時評価の必然的な停止を保証し、型システムの無限ループを回避
- **統一性**：停止検査と部分正当性検証が同じ仕様機構を共有
- **段階的強化**：ランタイム検査から完全静的証明へと段階的に移行可能

## 動機

### なぜ強い泛型システムが必要なのか？

現在の主流言語の泛型には限界がある：

| 言語 | 泛型能力 | 問題 |
|------|----------|------|
| Java | 境界型 | コンパイル時単態化、泛型特殊化なし |
| C# | 泛型制約 | ランタイム型検査、性能オーバーヘッドあり |
| Rust | 泛型 + Trait | Traitシステムが複雑、学習曲線が急峻 |
| C++ | テンプレート | テンプレート特殊化が複雑、コンパイルエラー情報が悪い |
| **YaoXiang** | **値依存型** | **型が値に依存可能、コンパイル時次元検証、停止保証** |

### 核心的矛盾

1. **性能 vs 柔軟性**：ランタイム柔軟性 vs コンパイル時最適化
2. **複雑 vs 簡潔**：強力な型システム vs 使いやすさ
3. **マクロ vs 泛型**：マクロコード生成 vs 泛型型安全性
4. **値依存 vs 型安全性**：伝統的な泛型ではコンパイル時に次元を検証できない

### 値依存型の核心的優位性

YaoXiang の**値依存型**は伝統的な泛型に対する核心的優位性である：

| 優位性 | 説明 |
|------|------|
| **型が値に依存** | `Vec: (n: Int) -> Type` により型が具体的な値に依存 |
| **コンパイル時評価** | 型位置での関数呼び出しはコンパイル時に評価され、結果が直接型に埋め込まれる |
| **次元検証** | `Matrix(Float, 3, 3)` でコンパイル時に行列次元を検証 |
| **型レベル計算** | `If`, `Match` などの条件型が型レベル計算をサポート |
| **停止保証** | decreases仕様がコンパイル時評価の必然的な停止を保証 |

```yaoxiang
# C++/Rust にはできないコンパイル時検証
matrix: Matrix(Float, factorial(3), factorial(2)) = ...
# コンパイル時計算：factorial(3) = 6, factorial(2) = 2
# 型は Matrix(Float, 6, 2)

# 次元不一致はコンパイル時に捕捉される
identity: Matrix(Float, 3, 3) = ...
# multiply(matrix_2x3, identity_3x3)  # コンパイルエラー：2 != 3
```

### 泛型システムの価値

```yaoxiang
# 例：統一API設計
# 異なるコンテナ型のmap操作

# 従来方案：型ごとに個別実装
map_int_array: (array: Array(Int), f: Fn(Int) -> Int) -> Array(Int) = ...
map_string_array: (array: Array(String), f: Fn(String) -> String) -> Array(String) = ...
map_int_list: (list: List(Int), f: Fn(Int) -> Int) -> List(Int) = ...
map_string_list: (list: List(String), f: Fn(String) -> String) -> List(String) = ...

# 泛型方案：一つの泛型関数がすべての型をカバー
map: (T: Type, R: Type)(container: Container(T), f: Fn(T) -> R) -> Container(R) = {
    for item in container {
        result.push(f(item))
    }
    result
}
```

## 設計目標

### 核心的目標

1. **ゼロコスト抽象** - 泛型呼び出しは具体型呼び出しと等価
2. **デッドコード除去** - コンパイル時分析、使用される泛型のみをインスタンス化
3. **マクロ代替** - 泛型がマクロ使用シーンの90%を代替
4. **型安全性** - コンパイル時検査、ランタイム型オーバーヘッドなし
5. **IDEフレンドリー** - スマートヒント、明確なエラー情報
6. **値依存型** - 型が値に依存可能、コンパイル時次元検証をサポート
7. **コンパイル時評価の安全性** - decreases仕様によってコンパイル時評価の停止を保証

### 設計原則

- **コンパイル時決定**：泛型パラメータはコンパイル時に決定
- **単態化優先**：具体コードを生成、虚関数呼び出しを回避
- **制約駆動**：型制約がインスタンス化をガイド
- **プラットフォーム最適化**：特殊化でプラットフォーム固有の最適化をサポート
- **型宇宙統一**：関数/型コンストラクタ/値依存型をType2層に統一
- **停止保証**：型位置での関数呼び出しは停止を証明しなければならない

## 提案

### 1. 基礎泛型

#### 1.1 泛型型パラメータ

> **重要なルール**：泛型型定義は**明示的に `: Type` を标注しなければならない**。さもないとHMによって関数として推論される。
>
> | 書き方 | 意味 |
> |------|------|
> | `List: (T: Type) -> Type = {...}` | ✅ 型コンストラクタ |
> | `List = {...}` | ❌ HMが関数として推論、型ではない |

```yaoxiang
# 泛型型定義（: Type 必須）
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
    push: (self: List(T), item: T) -> Void,   # self は単なる慣習名であり、キーワードではない
    get: (self: List(T), index: Int) -> Option(T),
}

# 泛型関数（: Type なし、HMが関数として推論）
map: (T: Type, R: Type) -> ((opt: Option(T), f: Fn(T) -> R) -> Option(R)) = {
    return match opt {
        some => Option.some(f(some)),
        none => Option.none(),
    }
}

# 泛型制約（直接表現式、単一行なら return 省略可）
clone: (T: Clone)(value: T) -> T = value.clone()

# 複数型パラメータ
combine: (T: Type, U: Type) -> ((a: T, b: U) -> (T, U)) = (a, b)
```

### 泛型関数の呼び出し構文

#### 1.1 統一シグネチャ構文

```yaoxiang
# 泛型関数は統一された (T: Type, R: Type) シグネチャ構文を使用
map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R)) = ...

# 複数型パラメータ
combine: (T: Type, U: Type) -> ((a: T, b: U) -> (T, U)) = (a, b)
```

#### 1.2 Type自己記述機構

`Type` は言語レベルの特殊存在であり、コンパイラはシグネチャ中の `Type` の位置を自然に認識し、実際のパラメータ型から自動推論して埋める。

```yaoxiang
# コンパイラが泛型パラメータを自動推論
numbers: List(Int) = List(Int)
#         ^^^^^^^^   ^^^^^^
#         型宣言     コンストラクタ呼び出し：Int が T を埋める

# 関数呼び出し推論
numbers: List(Int) = List(Int)
f: (x: Int) -> String = (x) => x.to_string()
strings: List(String) = map(numbers, f)
# コンパイラ推論：T=Int, R=String
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

#### 1.4 明示的指定（推論失敗時）

```yaoxiang
# 推論可能な場合は Type パラメータを省略
numbers: List(Int) = List(Int)
strings: List(String) = map(numbers, (x: Int) => x.to_string())

# 推論不可能な場合は明示的指定が必須
# map(numbers, (x) => x)  # ❌ Error: Cannot infer R
```

### 2. 型制約システム

#### 2.1 単一制約

```yaoxiang
# 基本的な trait 定義（インターフェース型）
Clone: Type = {
    clone: (Self) -> Self,
}

Display: Type = {
    fmt: (Self, Formatter) -> Result,
}

Debug: Type = {
    fmt: (Self, Formatter) -> Result,
}

# 制約の使用：シグネチャ内で型制約を直接宣言
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

# 泛型コンテナのソート
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

#### 2.4 組み込みマーカートレイト：Dup と Clone

**三種類のコピーセマンティクス**：

| 型 | 意味 | トリガー方式 | 適用シーン |
|------|------|----------|----------|
| **原語値コピー** | 代入時に自動値コピー、二つの値は完全に独立 | 代入/引数渡しで自動 | Int, Float, Bool, Char |
| **Dup** | シャローコピー：ハンドル/トークンをコピー、基礎データを共有 | 代入/引数渡しで自動 | `&T` トークン、`ref T`、String/Bytes |
| **Clone** | ディープコピー：完全独立な副本を作成 | `value.clone()` | Clone を実装する任意の型 |

**Dup のセマンティクス**：Dup を実装した型は、代入/引数渡し時に所有権を移転しない — コンパイラがハンドル/トークンをコピーし、複数の所有者が同じ基礎データを指す。これは RFC-009 所有権モデルにおける Move デフォルトセマンティクスの補完である。

**Dup と Clone は直交的な概念である**：

```
Dup = ハンドルをコピー、データを共有（変更は相互に影響する）
Clone = データをコピー、副本は独立（変更は相互に影響しない）
```

**ルール**：

```
1. 原語値型（Int, Float, Bool, Char） — コンパイラ組み込みの値コピー、Dup には属さない
2. Dup  — 参照/トークン型と内部参照カウントの型にのみ適用
3. Clone — 明示的ディープコピー、任意の型が実装可能
4. デフォルト Move — 他の型はデフォルトの Move セマンティクスを維持
```

**どの型が Dup か**：

| 型 | Dup | 理由 |
|------|-----|------|
| `&T`（借用トークン） | ✅ | ゼロサイズトークン、トークンコピー = 同じデータへの複数のビュー |
| `ref T` | ✅ | Rc/Arc コピー = 参照カウント+1、ヒープデータを共有 |
| String, Bytes | ✅ | 内部参照カウント、ハンドル共有で基礎 buffer を共有 |
| `&mut T`（可変トークン） | ❌ | 線形独占、コピー不可 |
| struct | 派生 | すべてのフィールドが Dup → struct は Dup |
| enum | 派生 | すべての variant のすべてのフィールドが Dup → enum は Dup |
| tuple | 派生 | すべての要素が Dup → tuple は Dup |
| Fn（クロージャ） | ❌ | キャプチャ環境が Dup でない可能性 |
| `*T`（生ポインタ） | ❌ | unsafe、所有権システムに参加しない |

**Int/Float/Bool/Char は Dup ではない** — これらは値型であり、代入時にコンパイラが自動的に値コピーを行う（二つの値は完全に独立）。これは「シャローコピー」ではなく、原語に対するコンパイラ組み込み処理であり、Dup 型属性を通じて表現すべきものではない。

```yaoxiang
# 原語値型：コンパイラが自動的に値コピー（Dup ではない）
x: Int = 42
y = x          # 値コピー、x と y は完全に独立
print(x)       # ✅

# Dup：シャローコピー、ハンドル共有でデータを共有
view: &Point = &point
view2 = view    # ✅ Dup：トークンコピー、両者は同じ point を指す
print(view.x)   # ✅

# Clone：明示的ディープコピー、独立した副本を作成
backup = big_struct.clone()  # 明示的呼び出し

# 泛型制約
dup_use: (T: Dup) -> T = x         # T: Dup → シャローコピー可能
clone_use: (T: Clone) -> T = x.clone()  # T: Clone → ディープコピー可能
```

> **注意**：`Send`/`Sync` はユーザー可視のトレイトではない。タスク横断の安全保証は `ref` キーワードとコンパイラによって全自动処理される — `ref` が自動的に Rc または Arc を選択し、ユーザーは Send/Sync を理解する必要がない。

### 3. 関連型

#### 3.1 関連型の定義

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

# Array の Iterator 実装
# メソッド構文糖を使用：Array.Item, Array.next, Array.has_next
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

#### 3.2 泛型関連型（GAT）

```yaoxiang
# より複雑な関連型
Producer: (Item: Type) -> Type = {
    Item: T,
    produce: (Self) -> Option(Item),
}

# 関連型は泛型にすることも可能
Container: (Item: Type) -> Type = {
    Item: T,
    IteratorType: Iterator(Item),  # 関連型も泛型
    iter: (Self) -> IteratorType,
}

# 使用
process_container: (T: Type, C: Container(T))(container: C) -> List(T) = {
    container.iter().collect()
}
```

### 4. コンパイル時泛型

#### 4.1 コンパイル時定数パラメータ

**核心設計**：泛型シグネチャ中の `Type` マーカーはコンパイル時型パラメータを示し、`Int` などの値パラメータは泛型コンテキストではデフォルトでコンパイル時に決定可能。`const` キーワードは不要。

```yaoxiang
# ════════════════════════════════════════════════════════
# コンパイル時定数パラメータ：泛型中の Int はデフォルトでコンパイル時決定
# ════════════════════════════════════════════════════════

# コンパイル時階乗：N はコンパイル時に既知のリテラルでなければならない
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
    data: Array(T, N),  # コンパイル時に既知のサイズの配列
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

# 使用：コンパイル時計算、Matrix(Float, 3, 3) を生成
identity_3x3: Matrix(Float, 3, 3) = identity_matrix(Float, 3)(3)
```

#### 4.3 コンパイル時検証（標準ライブラリ実装）

```yaoxiang
# ════════════════════════════════════════════════════════
# 標準ライブラリ実装：条件型を利用
# ════════════════════════════════════════════════════════

# 標準ライブラリ定義：Assert[C] は型である
# - C が True の時、Void に推論される
# - C が False の時、compile_error("Assertion failed") に推論される
Assert: (C: Type) -> Type = match C {
    True => Void,
    False => compile_error("Assertion failed"),
}

# 使用方式1：型定義中で制約として
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    # コンパイル時検査：N は 0 より大きくなければならない（Assert は型位置）
    length: Assert(N > 0),
}

# 使用方式2：式中で使用
IntArray: (N: Int) -> Type = StaticArray(Int, N)
# 検証：IntArray(10) のサイズは sizeof(Int) * 10 と等しい
Assert(size_of(IntArray(10)) == sizeof(Int) * 10)
```

#### 4.4 コンパイル時泛型特殊化

```yaoxiang
# 小配列最適化：関数オーバーロードでコンパイル時泛型特殊化を実現

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

> **Curry-Howard同型**：条件型はCurry-Howardの視点から論理の **case分析** に対応する。`Bool` 型は二つの可能な値（True/False）を持つ命题に対応し、`If` はその命题の真偽に応じて異なる結果を選択する — これは論理における case 析取そのものである。`match C { True => T, False => E }` は実際には「命题 C が True の時結論は T、False の時結論は E」ということを表現している。

#### 5.1 If 条件型

```yaoxiang
# 型レベル If
If: (C: Bool, T: Type, E: Type) -> Type = match C {
    True => T,
    False => E,
}

# 例：コンパイル時分岐
NonEmpty: (T: Type) -> Type = If(T != Void, T, Never)

Optional: (T: Type) -> Type = If(T != Void, T, Void)

# コンパイル時検証
Assert: (C: Bool) -> Type = match C {
    True => Void,
    False => compile_error("Assertion failed"),
}

# 使用
# 型計算：If(True, Int, String) => Int
# 型計算：If(False, Int, String) => String
```

#### 5.2 型族

> **Curry-Howard同型**：型族は「命题即型」の最も直接的な具現である。`Add: (A: Type, B: Type) -> Type` は「型の層に加法関数を書いた」のではなく、自然数加法に関する **命题を構成している**。`(Zero, B) => B` は「命题 Add(Zero, B) は B と等価である」と言い、`(Succ(A'), B) => Succ(Add(A', B))` は「Add(A', B) が成立するならば、Add(Succ(A'), B) も成立する」と言う。これがまさに Peano 公理における加法の定義そのものである。型検査器がこの match 式が合格することを検証することは、この定義の論理的一貫性を検証することと等価である。

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

# 型レベル加法（Curry-Howard：これは自然数加法の帰納的定義でもある）
Add: (A: Type, B: Type) -> Type = match (A, B) {
    (Zero, B) => B,
    (Succ(A'), B) => Succ(Add(A', B)),
}

# 例：コンパイル時計算 2 + 3
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
# RFC-010 構文に完全準拠した特殊化方式：関数オーバーロード

# 具体型特殊化
sum: (arr: Array(Int)) -> Int = {
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Array(Float)) -> Float = {
    return simd_sum_float(arr.data, arr.length)
}

# 泛型実装（コンパイラが自動的に最適なものを選択）
sum: (T: Type) -> ((arr: Array(T)) -> T) = {
    result = Zero::zero()
    for item in arr {
        result = result + item
    }
    return result
}

# 使用時は完全に透明
int_arr = Array(Int)(1, 2, 3)
float_arr = Array(Float)(1.0, 2.0, 3.0)

# コンパイラが自動的に最適な特殊化を選択
sum(int_arr)     # sum: (Array(Int)) -> Int を選択
sum(float_arr)    # sum: (Array(Float)) -> Float を選択
```

#### 6.3 関数オーバーロードとインライン化の完璧な結合

**重要な特性**：関数オーバーロードとインライン化最適化は自然に結合し、ゼロコスト抽象を実現する。

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
# コンパイラが自動的に最適な特殊化を選択し、インライン化
result = native_sum_int(int_arr.data, int_arr.length)

# 手書き最適化コードと完全に等価、関数呼び出しオーバーヘッドなし！
```

**核心的優位性**：

1. **コンパイラのスマート選択**
   ```yaoxiang
   sum(int_arr)      # sum: (Array(Int)) -> Int を自動選択
   sum(float_arr)    # sum: (Array(Float)) -> Float を自動選択
   sum(custom_arr)  # sum: (T: Type) -> ((arr: Array(T)) -> T) を自動選択
   ```

2. **インライン化最適化**
   - 小さな関数は呼び出し先に自動インライン化
   - 関数呼び出しオーバーヘッドがゼロ
   - 手書き最適化コードと完全に等価

3. **型安全性**
   - コンパイル時型検査
   - ランタイムオーバーヘッドがゼロ
   - 虚関数テーブル不要

4. **RFC-010 と完璧に契合**
   ```yaoxiang
   # 統一構文を完全使用
   name: type = value
   # impl、where などの新しいキーワード不要
   ```

**実際の応用例**：

```yaoxiang
# 性能敏感な数値計算
fibonacci: (n: Int) -> Int = {
    if n <= 1 { return n }
    return fibonacci(n - 1) + fibonacci(n - 2)
}

fibonacci: (n: Float) -> Float = {
    # Binet の公式を使用
    phi = (1.0 + 5.0.sqrt()) / 2.0
    return (phi.pow(n) - (-phi).pow(-n)) / 5.0.sqrt()
}

# コンパイラが自動選択してインライン化
fibonacci(10)      # Int バージョンを選択、完全にインライン化
fibonacci(10.5)    # Float バージョンを選択、Binet の公式を使用
```

**これは何を意味するか？**

- ✅ **泛型特殊化** → 関数オーバーロードで自然に解決
- ✅ **性能最適化** → インライン化が自動完了
- ✅ **コード再利用** → 一つの関数名、複数の実装
- ✅ **ゼロコスト抽象** → コンパイル時多態、ランタイムオーバーヘッドゼロ
- ✅ **新しいキーワード不要** → RFC-010 統一構文に完璧に準拠
```

### 7. デッドコード除去機構

#### 7.1 インスタンス化グラフ解析

```rust
// コンパイラ内部：泛型インスタンス化依存グラフを構築
struct InstantiationGraph {
    // ノード：泛型インスタンス化
    nodes: HashMap<InstanceKey, InstanceNode>,

    // エッジ：使用関係
    edges: HashMap<InstanceKey, Vec<InstanceKey>>,
}

struct InstanceKey {
    generic: FunctionId,  // 泛型関数 ID
    type_args: Vec<TypeId>,  // 型パラメータ
    const_args: Vec<ConstId>,  // Const パラメータ
}

// アルゴリズム：到達可能性解析
fn eliminate_dead_instantiations(graph: &InstantiationGraph) {
    let mut reachable = HashSet::new();

    // エントリポイントから開始（main、エクスポート関数など）
    let entry_points = find_entry_points();
    for entry in entry_points {
        dfs_visit(entry, &graph, &mut reachable);
    }

    // 訪問されていないインスタンス化はデッドコード
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

# 使用点1：map(Int, Int) をインスタンス化
int_list = List(1, 2, 3)
doubled = map(int_list, (x) => x * 2)  # map[Int, Int] が必要

# 使用点2：map(String, String) をインスタンス化
string_list = List("a", "b", "c")
uppercased = map(string_list, (s) => s.to_uppercase())  # map[String, String] が必要

# 未使用：map[Float, Float] など
# これらの泛型インスタンスは生成されない

# コンパイル後は使用されているインスタンスのみを含む
map_Int_Int: (list: List(Int), f: Fn(Int) -> Int) -> List(Int) = ...
map_String_String: (list: List(String), f: Fn(String) -> String) -> List(String) = ...
```

#### 7.3 コンパイル時泛型 DCE

```yaoxiang
# コンパイル時分析：コンパイル時泛型の使用状況
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
}

# 実際の使用状況
arr_10_int = Array(Int, 10)(1, 2, 3, 4, 5, 6, 7, 8, 9, 10)
arr_100_int = Array(Int, 100)(...)

# コンパイル後は使用されている Size のみを生成
Array_Int_10: (Array(Int, 10)) = ...
Array_Int_100: (Array(Int, 100)) = ...

# 未使用の Size は生成されない
# Array(Int, 50) は生成されない
```

#### 7.4 モジュール横断 DCE

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

# コンパイル分析：
# - モジュール B は map[Int, Int] を使用
# - モジュール C は map[String, String] を使用
# - コンパイル後のバイナリにはこの二つのインスタンスのみが含まれる
```

#### 7.5 LLVM レイヤ DCE

```rust
// コンパイルパイプライン
fn optimize_ir(ir: &mut IR) {
    // 1. 単態化（YaoXiang コンパイラ）
    ir.monomorphize();

    // 2. インライン化最適化
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

    // 6. 最適化を実行
    llvm_ir.run_optimization_passes();
}
```

### 8. マクロ代替戦略

#### 8.1 コード生成の代替

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

# ✅ 泛型方案：自動派生
# 関数オーバーロード方式で自動派生
debug_fmt: (T: fields...) -> ((self: Point(T)) -> String) = {
    return "Point { x: " + self.x.to_string() + ", y: " + self.y.to_string() + " }"
}

# 使用
p = Point { x: 1, y: 2 }
p.debug_fmt(&formatter)  # 呼び出しを自動生成
```

#### 8.2 DSL の代替

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

# ✅ 泛型方案：型安全ビルダー
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
# ❌ マクロ方案：型レベル計算
macro_rules! add_types {
    ($a:ty, $b:ty) => {
        ($a, $b)
    };
}

# ✅ 泛型方案：条件型
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
result_type = Add[Int, Float]  # Float と推論
```

### 9. 例

#### 9.1 完全な泛型コンテナの例

```yaoxiang
# ======== 1. 泛型コンテナの定義 ========
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

    # 泛型メソッド（T は外側の List(T) から自動的にスコープに導入される）
    push: (self: List(T), item: T) -> Void,
    pop: (self: List(T)) -> Option(T),
    map: (R: Type) -> ((self: List(T), f: (T) -> R) -> List(R)),
    filter: (self: List(T), predicate: (T) -> Bool) -> List(T),
    fold: (U: Type) -> ((self: List(T), initial: U, f: (U, T) -> U) -> U),
}

# ======== 2. 泛型メソッドの実装 ========
# 関数は List 名前空間下で定義（List. プレフィックス = 名前空間帰属）
# list.push(item) のような . 呼び出し構文を有効にするには、明示的バインディングが必要：List.push = push[0]
# self は単なる慣習パラメータ名であり、コンパイラは名前ではなく型を見る

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
# 泛型 List を作成
numbers = List(Int)()
numbers.push(1)
numbers.push(2)
numbers.push(3)

# 泛型メソッドを使用
doubled = numbers.map((x) => x * 2)
evens = numbers.filter((x) => x % 2 == 0)

# fold を使用して計算
sum = numbers.fold(0, (acc, x) => acc + x)  # sum = 6

# 泛型組み合わせ
sum_of_evens = numbers
    .filter((x) => x % 2 == 0)
    .map((x) => x * 2)
    .fold(0, (acc, x) => acc + x)  # sum_of_evens = 8
```

#### 9.2 泛型アルゴリズムの例

```yaoxiang
# ======== 1. 泛型ソートアルゴリズム ========
Comparator: (T: Type) -> Type = {
    compare: (T, T) -> Int,  # -1 if a < b, 0 if a == b, 1 if a > b
}

# 泛型クイックソート
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

# ======== 2. IntComparator の実装 ========
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

# String 配列をソート（StringComparator が必要）
strings = Array(String)("hello", "world", "foo", "bar")
sorted_strings = quicksort(strings, Comparator(String)())
```

#### 9.3 コンパイル時泛型の例

```yaoxiang
# ======== 1. コンパイル時行列型 ========
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),

    # コンパイル時次元検証：Assert 標準ライブラリ型を利用
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

# ======== 2. コンパイル時行列の作成 ========
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
# コンパイル時に既知サイズの行列を作成
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

# コンパイル時検証：result の型は Matrix(Float, 2, 2)
# 2x2 単位行列
identity_3x3 = identity(Float, 3)()

# 次元不一致：コンパイルエラー
# bad_multiply = matrix_2x3.multiply(identity_3x3)  # コンパイルエラー：3x3 != 2x3
```

## トレードオフ

### 優位性

1. **ゼロコスト抽象**
   - コンパイル時単態化、ランタイムオーバーヘッドなし
   - 虚関数不要、RTTI 不要

2. **デッドコード除去**
   - コンパイル時分析、使用される泛型のみをインスタンス化
   - コード膨張が制御可能

3. **マクロ代替**
   - 型安全なコード生成
   - IDE フレンドリー、エラー情報が明確

4. **コンパイル時計算**
   - コンパイル時泛型がコンパイル時計算をサポート
   - 次元検証などの特性
   - `const` キーワード不要、純粋な型制約

### 欠点

1. **コンパイル時間**
   - 泛型インスタンス化がコンパイル時間を増加させる
   - 制約求解が遅くなる可能性

2. **メモリ使用量**
   - コンパイラのメモリ使用量が増加
   - キャッシュ機構にメモリが必要

3. **実装の複雑さ**
   - 制約ソルバが複雑
   - 型レベル計算エンジンが複雑

4. **エラー診断**
   - 泛型エラーが複雑になる可能性
   - 明確なエラーヒントが必要

### 緩和措置

1. **キャッシュ戦略**
   - インスタンス化結果のキャッシュ
   - LRU キャッシュでメモリ制限

2. **インクリメンタルコンパイル**
   - コンパイル結果のキャッシュ
   - インクリメンタルインスタンス化

3. **エラーヒント**
   - 明確なエラー情報
   - 泛型パラメータ推論のヒント

4. **並列コンパイル**
   - 泛型インスタンス化の並列化
   - マルチスレッド制約求解

## 代替方案

| 方案 | なぜ選択しないか |
|------|--------------|
| 基礎泛型のみ | 複雑なマクロを代替できない |
| 純粋マクロシステム | 型安全性がなく、エラー情報が悪い |
| 依存制約のみ | 柔軟性が不足 |
| ランタイム泛型 | 性能オーバーヘッドあり |

### リスク

| リスク | 影響 | 緩和措置 |
|------|------|----------|
| 制約求解の複雑さ | コンパイル時間が長すぎる | インクリメンタル求解 + キャッシュ |
| コード膨張 | バイナリファイルが大きすぎる | DCE + 閾値制御 |
| 実装の複雑さ | 開発期間が延長 | 段階的実装 |
| エラー診断 | ユーザー体験が悪い | 詳細なエラー情報 |

## 未解決問題

### 決議待ちの問題

| 議題 | 説明 | 状態 |
|------|------|------|
| インスタンス化戦略 | Eager vs Lazy vs Threshold | 議論待ち |
| キャッシュサイズ | LRU キャッシュ容量設定 | 議論待ち |
| エラー診断 | 泛型エラー情報の詳細度 | 議論待ち |

### 今後の最適化

| 最適化項目 | 価値 | 実装難易度 |
|--------|------|----------|
| インスタンス化グラフ分析 | 高 | 中 |
| 型レベルプログラミング DSL | 中 | 高 |
| 泛型性能ベンチマーク | 中 | 低 |

## 付録

### 構文 BNF

```bnf
# 泛型パラメータは統一された () 構文を使用し、関数型の一部である
# 例：map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R))

# 型制約（泛型パラメータ中）
type_bound ::= identifier
             | identifier '+' identifier ('+' identifier)*

# パラメータ宣言（型 + 名前）
parameter ::= identifier ':' type

parameters ::= parameter (',' parameter)*

# 関数宣言：name: type = expression
# 泛型パラメータは関数型の最初のパラメータ群：(T: Type) -> ((params) -> return)
function ::= identifier ':' type '=' (expression | block)

# メソッド宣言：Type.method: type = expression
method ::= identifier '.' identifier ':' type '=' (expression | block)

# 型定義（統一 Binding 構文）
# 泛型型 例：List: (T: Type) -> Type = { ... }
generic_type ::= identifier ':' type '=' type_expression

# 泛型パラメータ中の Type はコンパイラが実引数型から自動的に埋める
# 例：map(numbers, f)、T は numbers: List(Int) から、R は f: (Int) -> String から抽出
```

## ライフサイクルと帰属

```
┌─────────────┐
│   草案      │  ← 現在の状態
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  レビュー中 │  ← オープンなコミュニティ議論とフィードバック
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  受け入れ   │    │  拒否       │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   accepted/ │    │    rfc/     │
│ (正式設計)  │    │ (元の場所)  │
└─────────────┘    └─────────────┘
```

---

## 参考文献

### YaoXiang 公式ドキュメント

- [RFC-010: 統一型構文](./010-unified-type-syntax.md)
- [RFC-009: 所有権モデル](./accepted/009-ownership-model.md)
- [RFC-001: 並行モデル](./accepted/001-concurrent-model-error-handling.md)
- [RFC-008: ランタイムモデル](./accepted/008-runtime-concurrency-model.md)
- [言語仕様](../language-spec.md)
- [YaoXiang ガイド](../guides/YaoXiang-book.md)

### 外部参考

- [Rust 泛型システム](https://doc.rust-lang.org/book/ch10-01-syntax.html)
- [C++ テンプレート特殊化](https://en.cppreference.com/w/cpp/language/template_specialization)
- [Haskell 型クラス](https://www.haskell.org/tutorial/classes.html)
- [Swift 泛型](https://docs.swift.org/swift-book/LanguageGuide/Generics.html)
- [単態化最適化](https://llvm.org/docs/Monomorphization.html)
- [デッドコード除去](https://en.wikipedia.org/wiki/Dead_code_elimination)
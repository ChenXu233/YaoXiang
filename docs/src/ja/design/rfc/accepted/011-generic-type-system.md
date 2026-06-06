---
title: "RFC-011: ジェネリックシステム設計 - ゼロコスト抽象化とマクロ代替"
status: "受入済み"
author: "晨煦"
created: "2025-01-25"
updated: "2026-04-22（Type自己記述機構に更新、ジェネリック呼び出し構文を統一）"
---

# RFC-011: ジェネリックシステム設計 - ゼロコスト抽象化とマクロ代替

## 摘要

本ドキュメントはYaoXiang言語の**ジェネリックシステム設計**を定義し、強力なジェネリック機能を通じてゼロコスト抽象化を実現し、コンパイル時最適化によりマクロへの依存を軽減し、デッドコード除去メカニズムを提供します。

**コア設計**：
- **統一署名構文**：`(T: Type, R: Type) -> ...` ジェネリック引数と通常の引数を統一
- **Type自己記述機構**：`Type`は言語レベルの特殊存在であり、署名内の`Type`位置は自動的に推論・補完可能
- **型制約**：`T: Dup + Add` 複数制約、関数型制約
- **関連型**：`Iterator: (Item: Type) -> Type = { next: () -> Option(Item), has_next: () -> Bool }`
- **コンパイル時ジェネリック**：`N: Int` ジェネリック値パラメータ、コンパイル時定数インスタンス化
- **条件型**：`If: (C: Bool, T: Type, E: Type) -> Type` 型レベル計算、型族

**価値**：
- ゼロコスト抽象化：コンパイル時モノリゼーション、ランタイムオーバーヘッドなし
- デッドコード除去：インスタンス化グラフ分析 + LLVM最適化
- マクロ代替：ジェネリックでマクロ使用シナリオの90%を代替
- 型安全性：コンパイル時チェック、IDEフレンドリー
- **明示的优于隐式**：`Type`自己記述、コンパイラが自動的に推論

## 參照文献

本ドキュメントの設計は以下のドキュメントに基づいています：

| 文献 | 関係 | 説明 |
|------|------|------|
| [RFC-010: 統一型構文](./010-unified-type-syntax.md) | **構文基盤** | ジェネリック構文と統一`name: type = value`モデルの統合 |
| [RFC-010: 統一型構文](./010-unified-type-syntax.md) | **呼び出し構文** | 第6節：ジェネリック呼び出し構文——統一`()`適用、`[]`を完全撤去 |
| [RFC-009: 所有権モデル](./accepted/009-ownership-model.md) | **型システム** | Move意味論とジェネリックの自然な融合 |
| [RFC-001: 並作モデル](./accepted/001-concurrent-model-error-handling.md) | **実行モデル** | DAG分析とジェネリック型チェック |
| [RFC-008: 実行時モデル](./accepted/008-runtime-concurrency-model.md) | **コンパイラアーキテクチャ** | ジェネリックモノリゼーションとコンパイル時最適化戦略 |
| [型宇宙思想](../reference/plan/ongoing/タイプ宇宙思想.md) | **理論コア** | 型宇宙階層モデルと値依存型設計 |
| [RFC-022: ホア論理静的検証](./draft/022-hol-logic-verification.md) | **終了チェック** | decreases契約とコンパイル時評価安全保障 |

## 型宇宙思想と値依存型

YaoXiangのジェネリックシステムは**型宇宙思想**に基づいており、このメンタルモデルは言語内のすべての概念を階層構造に統一し、核心的な革新は**値依存型**をType2層の第一級市民として昇格させた点にあります。

### 値依存型とは？

**値依存型**とは、1つ以上の**値**（他の型のみに依存するのではなく）に依存する型のことです。これらの値はコンパイル時に評価可能であり、コンパイル段階で型安全保証を提供します。

```yaoxiang
# 従来のジェネリック：型パラメータ
List: (T: Type) -> Type

# 値依存型：値パラメータ
Vec: (n: Int) -> Type  # ベクトル型は長さ値nに依存
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type  # 行列型は行数と列数に依存
```

### 値依存型のコア優位性

従来のジェネリックと比較して、YaoXiangの値依存型は以下のコア優位性を持ちます：

| 特性 | 従来のジェネリック (C++/Rust) | YaoXiang 値依存型 |
|------|------------------------------|-------------------|
| 型が依存する値 | 型パラメータのみに依存 | 関数呼び出し結果を含む任意の値に依存可能 |
| コンパイル時評価 | C++テンプレート手動特化、Rust無 | 自動コンパイル時評価、終了保証付き |
| 型レベル計算 | テンプレートメタプログラミング（複雑/危険） | 統一された型レベル計算エンジン |
| 型安全性 | C++無、Rust制限的 | 完全な型安全性、コンパイル時チェック |
| 次元検証 |  런타임チェックまたは手動特化 | コンパイル時次元検証、ランタイムオーバーヘッドなし |

### 型宇宙階層と値依存型

型宇宙思想は言語の概念を意味的役割に応じて異なる階層に分類し、値依存型は**Type2層**に位置します：

| 階層 | 役割 | 例 |
|------|------|------|
| Type-1 | 値 | `42`, `factorial(5)`, 関数自体 |
| Type0 | メタ型キーワード | `Type` |
| Type1 | 具象型 | `Int`, `String`, `Vec(3)` |
| **Type2** | **関数/型構築子/値依存型** | `add: (Int, Int) -> Int`, `Vec: (n: Int) -> Type`, `Matrix: (T: Type, Rows: Int, Cols: Int) -> Type` |

**鍵となる設計**：Type2層の関数、型構築子、値依存型は**統一構文**であり、いずれも`(params) -> result`形式です：
- 通常関数：`(Int, Int) -> Int` → 戻り値は値
- 型構築子：`(T: Type) -> Type` → 戻り値は型
- 値依存型：`(n: Int) -> Type` → 戻り値は型だが、値パラメータに依存

> **Curry-Howard同型対応**：この統一は偶然ではありません。Curry-Howard同型対応は「型即ち命題、プログラム即ち証明」を指摘します——関数型`A → B`は論理包含「AならばB」に対応し、ジェネリック`(T: Type) -> Type`は全称量化「任意の型Tについて」に対応し、値依存型`(n: Int) -> Type`は「各整数nに対して型が存在する」に対応します。YaoXiangは関数、型構築子、値依存型をType2層に統一しましたが、これは本質的に「証明」と「計算」を同一概念——**構成的証明**——に統一しています。これはCurry-Howard同型対応が言語設計において直接具現化されたものであり、1つの形式（`(params) -> result`）が論理命題と計算過程を同時に担います。

### コンパイル時確定的保証

YaoXiangの型宇宙思想は以下の要件を定めます：**Type階層的一切はコンパイル時に確定**。

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

# コンパイル時計算：factorial(3) = 6、ベクトルサイズはコンパイル時に確定
vec: Vec(factorial(3)) = Vec(6)()
```

コンパイラは自動的に以下を実行します：
1. 型位置上にある関数呼び出しを検出
2. 関数が`decreases`契約でマークされているかを検証（以下参照の終了チェックメカニズム）
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
    # コンパイル時チェック：a.Cols == b.Rows，否则はコンパイルエラー
    result = Matrix(T, Rows, M)()
    # ...
}

# エラーはコンパイル時に捕捉：
# multiply(matrix_2x3, matrix_4x2)  # コンパイルエラー：2 != 4
```

#### 型安全な配列サイズ
```yaoxiang
# 配列サイズはコンパイル時定数
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    length: N,
}

# Nはコンパイル時定数であり、型レベル計算に使用可能
first_three: Array(Int, 3) = Array(Int, 3)(1, 2, 3)
# first_three.length == 3（コンパイル時に既知）
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
# map: ジェネリック関数、型パラメータT, Rはコンパイル時に確定
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
doubled = map(numbers, (x) => x * 2)  # map[Int, Int]に推論
```

### 他言語との比較

| 特性 | C++テンプレート | Rustジェネリック | Haskell GADT | **YaoXiang** |
|------|----------------|------------------|--------------|--------------|
| 型パラメータ | ✅ | ✅ | ✅ | ✅ |
| 値依存型 | ❌ | ❌ | ✅ | ✅ |
| コンパイル時評価 | テンプレートインスタンス化 | ❌ | ✅ | ✅ |
| 終了保証 | ❌ | ❌ | ❌（危険） | ✅（decreases契約） |
| 型安全性 | ❌（マクロ展開） | ✅ | ✅ | ✅ |
| 統一構文 | ❌ | ❌ | ❌ | ✅ |
| コンパイル時次元検証 | 手動特化 | ランタイムチェック | 型族 | コンパイル時自動検証 |
| decreases契約 | ❌ | ❌ | ❌ | ✅ |

### 終了チェックメカニズム（RFC-022との統合）

値依存型のコンパイル時評価は**終了を保証**する必要があります、さもなくば型システムは無限ループに陥ります。YaoXiangは**decreases契約**を通じてこれを保証し、RFC-022とシームレスに統合します。

#### 再帰関数の終了契約
```yaoxiang
# コンパイル時階乗：終了を証明する必要がある
factorial: (n: Int) -> Int = {
    //! requires: n >= 0
    //! ensures: result == n!
    //! decreases: n    # 各再帰でnが厳密に減少
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

# 使用：型位置で呼び出し
vec: Vec(factorial(5)) = Vec(120)()  # コンパイル時にfactorial(5) = 120を評価
```

#### ループの終了契約
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
│  型チェック段階                                              │
│  型位置上の関数呼び出しを検出（例：Vec(factorial(5))）       │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  1. decreases契約をチェック                                  │
│     - decreasesあり：递减条件がすべての再帰パスで成立するか検証 │
│     - decreasesなしだが明らかに終了可能：直接評価              │
│     - decreasesなしかつ終了しない可能性：コンパイルエラー      │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  2. コンパイル時評価（ビルトインインタプリタが実行）          │
│     - 純関数：直接評価                                        │
│     - 副作用あり：コンパイルエラー（型位置は副作用禁止）       │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  3. 結果を型に埋め込み                                        │
│     - Vec(factorial(5)) → Vec(120)                          │
│     - Matrix(Float, 3, 3) → 具象型                           │
└─────────────────────────────────────────────────────────────┘
```

#### 優位性

- **安全性**：コンパイル時評価が必ず終了することを保証し、型システムの無限ループを防止
- **統一性**：終了チェックと部分的正当性検証が同一の契約メカニズムを共有
- **漸進的強化**：ランタイムチェックから完全静的証明へ徐々に移行可能

## 動機

### なぜ強力なジェネリックシステムが必要か？

現在主流の言語のジェネリックには限界があります：

| 言語 | ジェネリック機能 | 問題点 |
|------|------------------|--------|
| Java | 境界型 | コンパイル時モノリゼーション、ジェネリック特化なし |
| C# | ジェネリック制約 | ランタイム型チェック、パフォーマンスオーバーヘッドあり |
| Rust | ジェネリック + 特質 | 特質システムが複雑、学習曲線が険しい |
| C++ | テンプレート | テンプレート特化が複雑、コンパイルエラー情報が悪い |
| **YaoXiang** | **値依存型** | **型が値に依存、コンパイル時次元検証、終了保証** |

### コアな矛盾

1. **パフォーマンス vs 柔軟性**：ランタイム柔軟性 vs コンパイル時最適化
2. **複雑さ vs シンプルさ**：強力な型システム vs 使いやすさ
3. **マクロ vs ジェネリック**：マクロコード生成 vs ジェネリック型安全性
4. **値依存 vs 型安全性**：従来のジェネリックではコンパイル時に次元を検証できない

### 値依存型のコア優位性

YaoXiangの**値依存型**は従来のジェネリックに対するコアな優位性です：

| 優位性 | 説明 |
|--------|------|
| **型が値に依存** | `Vec: (n: Int) -> Type`で型が具体的な値に依存可能 |
| **コンパイル時評価** | 型位置の関数呼び出しをコンパイル時に評価し、結果を型に直接埋め込み |
| **次元検証** | `Matrix(Float, 3, 3)`で行列次元をコンパイル時に検証 |
| **型レベル計算** | `If`, `Match`などの条件型が型レベル計算をサポート |
| **終了保証** | decreases契約でコンパイル時評価が必ず終了することを保証 |

```yaoxiang
# C++/Rustでは做不到なコンパイル時検証
matrix: Matrix(Float, factorial(3), factorial(2)) = ...
# コンパイル時計算：factorial(3) = 6, factorial(2) = 2
# 型は Matrix(Float, 6, 2)

# 次元不一致はコンパイル時に捕捉
identity: Matrix(Float, 3, 3) = ...
# multiply(matrix_2x3, identity_3x3)  # コンパイルエラー：2 != 3
```

### ジェネリックシステムの価値

```yaoxiang
# 例：統一API設計
# 異なるコンテナ型のmap操作

# 従来案：各型を個別に実装
map_int_array: (array: Array(Int), f: Fn(Int) -> Int) -> Array(Int) = ...
map_string_array: (array: Array(String), f: Fn(String) -> String) -> Array(String) = ...
map_int_list: (list: List(Int), f: Fn(Int) -> Int) -> List(Int) = ...
map_string_list: (list: List(String), f: Fn(String) -> String) -> List(String) = ...

# ジェネリック案：1つのジェネリック関数ですべての型をカバー
map: (T: Type, R: Type)(container: Container(T), f: Fn(T) -> R) -> Container(R) = {
    for item in container {
        result.push(f(item))
    }
    result
}
```

## 設計目標

### コア目標

1. **ゼロコスト抽象化** - ジェネリック呼び出しは具象型呼び出しと同等
2. **デッドコード除去** - コンパイル時分析、使用されているジェネリックのインスタンスのみ生成
3. **マクロ代替** - ジェネリックでマクロ使用シナリオの90%を代替
4. **型安全性** - コンパイル時チェック、ランタイム型オーバーヘッドなし
5. **IDEフレンドリー** - インテリジェントなヒント、明確なエラー情報
6. **値依存型** - 型が値に依存可能、コンパイル時次元検証をサポート
7. **コンパイル時評価の安全性** - decreases契約を介してコンパイル時評価の終了を保証

### 設計原則

- **コンパイル時確定**：ジェネリックパラメータはコンパイル時に確定
- **モノリゼーション優先**：具象コードを生成、仮想関数呼び出しを回避
- **制約駆動**：型制約がインスタンス化をガイド
- **プラットフォーム最適化**：特化でプラットフォーム固有最適化をサポート
- **型宇宙の統一**：関数/型構築子/値依存型をType2層に統一
- **終了保証**：型位置の関数呼び出しは終了を証明する必要がある

## 提案

### 1. 基本ジェネリック

#### 1.1 ジェネリック型パラメータ

> **重要な規則**：ジェネリック型定義は**必ず`: Type`を明示的にマーク**する必要があります、さもなくばHM推論により関数として扱われます。
>
> | 書き方 | 意味 |
> |------|------|
> | `List: (T: Type) -> Type = {...}` | ✅ 型構築子 |
> | `List = {...}` | ❌ HM推論により関数、型ではない |

```yaoxiang
# ジェネリック型定義（: Typeが必要）
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
    push: (self: List(T), item: T) -> Void,
    get: (self: List(T), index: Int) -> Option(T),
}

# ジェネリック関数（: Typeなし、HM推論により関数）
map: (T: Type, R: Type) -> ((opt: Option(T), f: Fn(T) -> R) -> Option(R)) = {
    return match opt {
        some => Option.some(f(some)),
        none => Option.none(),
    }
}

# ジェネリック制約（直接式、1行はreturnを省略可能）
clone: (T: Clone)(value: T) -> T = value.clone()

# 複数型パラメータ
combine: (T: Type, U: Type) -> ((a: T, b: U) -> (T, U)) = (a, b)
```

### ジェネリック関数呼び出し構文

#### 1.1 統一署名構文

```yaoxiang
# ジェネリック関数は統一された(T: Type, R: Type)署名構文を使用
map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R)) = ...

# 複数型パラメータ
combine: (T: Type, U: Type) -> ((a: T, b: U) -> (T, U)) = (a, b)
```

#### 1.2 Type自己記述機構

`Type`は言語レベルの特殊存在であり、コンパイラは署名内の`Type`位置を本能的に識別し、実引数の型から自動的に推論・補完します。

```yaoxiang
# コンパイラが自動的にジェネリックパラメータを推論
numbers: List(Int) = List(Int)
#         ^^^^^^^^   ^^^^^^
#         型宣言   構築呼び出し：IntでTを補完

# 関数呼び出しの推論
numbers: List(Int) = List(Int)
f: (x: Int) -> String = (x) => x.to_string()
strings: List(String) = map(numbers, f)
# コンパイラが推論：T=Int, R=String
```

#### 1.3 モノリゼーション

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
doubled: List(Int) = map(int_list, (x: Int) => x * 2)  # map[Int, Int]をインスタンス化

string_list: List(String) = List(String)
uppercased: List(String) = map(string_list, (s: String) => s.to_uppercase())  # map[String, String]をインスタンス化

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

#### 1.4 明示的補完（推論失敗時）

```yaoxiang
# 推論可能な時はTypeパラメータを省略
numbers: List(Int) = List(Int)
strings: List(String) = map(numbers, (x: Int) => x.to_string())

# 推論不可能な時は明示的に補完する必要がある
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

# 制約を使用：署名で直接型制約を宣言
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
    # ソートアルゴリズムを実装
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

#### 2.4 組み込みmarker trait：DupとClone

**3種類のコピーセマンティクス**：

| 型 | 意味 | トリガー方式 | 適用シナリオ |
|------|------|-------------|-------------|
| **原語値コピー** | 代入時に自動値コピー、2つの値が完全に独立 | 代入/引数渡しが自動 | Int, Float, Bool, Char |
| **Dup** | 浅コピー：ハンドラ/トークンをコピー、基底データが共有 | 代入/引数渡しが自動 | `&T`トークン、`ref T`、String/Bytes |
| **Clone** | 深コピー：完全に独立したコピーを作成 | `value.clone()` | Cloneを実装する任意の型 |

**Dupのセマンティクス**：Dupを実装した型は代入/引数渡し時に所有権を移動しません——コンパイラがハンドラ/トークンをコピーし、複数の所有者が同じ基底データを参照します。これはRFC-009所有権モデルのMoveデフォルトセマンティクスの補完です。

**DupとCloneは直交する概念です**：

```
Dup = ハンドラをコピー、データを共有（変更は互いに影響）
Clone = データをコピー、コピーは独立（変更は互いに影響しない）
```

**規則**：

```
1. 原語値型（Int, Float, Bool, Char） — コンパイラの組み込み値コピー、Dupには属さない
2. Dup  — 参照/トークン型と内部参照カウントを持つ型にのみ適用
3. Clone — 明示的深コピー、任意の型が実装可能
4. デフォルトMove — 他の型はデフォルトMoveセマンティクスを維持
```

**Dupである型有哪些**：

| 型 | Dup | 理由 |
|------|-----|------|
| `&T`（借用トークン） | ✅ | ゼロサイズトークン、トークンをコピー = 同じデータを複数の視点で参照 |
| `ref T` | ✅ | Rc/Arcをコピー = 参照カウント+1、ヒープデータを共有 |
| String, Bytes | ✅ | 内部参照カウント、ハンドラをコピーして基底bufferを共有 |
| `&mut T`（可変トークン） | ❌ | 線形独占、コピー不可 |
| struct | 派生 | すべてのフィールドDup → struct Dup |
| enum | 派生 | すべてのvariantのすべてのフィールドDup → enum Dup |
| tuple | 派生 | すべての要素Dup → tuple Dup |
| Fn（クロージャ） | ❌ | キャプチャする環境がDupでない可能性がある |
| `*T`（生ポインタ） | ❌ | unsafe、所有権システムに参加しない |

**Int/Float/Bool/CharはDupではありません**——これらは値型であり、代入時にコンパイラが自動的に値コピーします（2つの値が完全に独立）。これは「浅コピー」ではなく、コンパイラの原語に対する組み込み処理であり、Dup型属性を通じて表現する必要もすべきでもない部分です。

```yaoxiang
# 原語値型：コンパイラが自動値コピー（Dupではない）
x: Int = 42
y = x          # 値コピー、xとyは完全に独立
print(x)       # ✅

# Dup：浅コピー、ハンドラをコピーしてデータを共有
view: &Point = &point
view2 = view    # ✅ Dup：トークンをコピー、両者が同じpointを参照
print(view.x)   # ✅

# Clone：明示的深コピー、独立コピーを作成
backup = big_struct.clone()  # 明示的呼び出し

# ジェネリック制約
dup_use: (T: Dup) -> T = x         # T: Dup → 浅コピー可能
clone_use: (T: Clone) -> T = x.clone()  # T: Clone → 深コピー可能
```

> **注意**：`Send`/`Sync`はユーザー可视traitではありません。タスク間安全保障は`ref`キーワードとコンパイラの全自动処理により担保されます——`ref`はRcまたはArcを自動選択し、ユーザーはSend/Syncを理解必要はありません。

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

# 関連型はジェネリックになれる
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

**コア設計**：ジェネリック署名内の`Type`はコンパイル時型パラメータをマークし、`Int`などの値パラメータはジェネリックコンテキストではデフォルトでコンパイル時に確定可能です。`const`キーワードは不要です。

```yaoxiang
# ════════════════════════════════════════════════════════
# コンパイル時定数パラメータ：ジェネリック内のIntはデフォルトでコンパイル時に確定
# ════════════════════════════════════════════════════════

# コンパイル時階乗：Nはコンパイル時に既知のリテラルでなければならない
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
    data: Array(T, N),  # コンパイル時にサイズが既知の配列
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

# コンパイラがコンパイル時にリテラル型の関数呼び出しを計算
SIZE: Int = factorial(5)  # コンパイル時は120

# 行列型で使用
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

# 使用：コンパイル時に計算、Matrix(Float, 3, 3)を生成
identity_3x3: Matrix(Float, 3, 3) = identity_matrix(Float, 3)(3)
```

#### 4.3 コンパイル時検証（標準ライブラリ実装）

```yaoxiang
# ════════════════════════════════════════════════════════
# 標準ライブラリ実装：条件型を利用
# ════════════════════════════════════════════════════════

# 標準ライブラリ定義：Assert[C]は型
# - CがTrueのとき、Voidに導出
# - CがFalseのとき、compile_error("Assertion failed")に導出
Assert: (C: Type) -> Type = match C {
    True => Void,
    False => compile_error("Assertion failed"),
}

# 使用方法1：型定義内で制約として使用
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    # コンパイル時チェック：Nは0より大きくなければならない（型位置のAssert）
    length: Assert(N > 0),
}

# 使用方法2：式内で使用
IntArray: (N: Int) -> Type = StaticArray(Int, N)
# 検証：IntArray(10)のサイズはsizeof(Int) * 10に等しい
Assert(size_of(IntArray(10)) == sizeof(Int) * 10)
```

#### 4.4 コンパイル時ジェネリック特化

```yaoxiang
# 小配列最適化：関数オーバーロードを使用してコンパイル時ジェネリック特化を実装

# 汎用実装
sum: (T: Type, N: Int) -> ((arr: Array(T, N)) -> T) = {
    result = Zero::zero()
    for item in arr.data {
        result = result + item
    }
    return result
}

# N=1特化
sum: (T: Type) -> ((arr: Array(T, 1)) -> T) = arr.data[0]

# N=2特化
sum: (T: Type) -> ((arr: Array(T, 2)) -> T) = arr.data[0] + arr.data[1]

# 小配列ループ展開（N <= 4）
sum: (T: Type, N: Int) -> ((arr: Array(T, N)) -> T) = {
    # コンパイラ最適化：ループを展開
    return arr.data[0] + arr.data[1] + arr.data[2] + arr.data[3]
}
```

### 5. 条件型

> **Curry-Howard同型対応**：条件型はCurry-Howardの視点からは論理の**case分析**に対応します。`Bool`型は2つの可能な値を持つ命題（True/False）に対応し、`If`はその命題の真偽に応じて異なる結果を選びます——これは論理におけるcase選言そのものです。`match C { True => T, False => E }`は実際には「命題CがTrueのとき結論はT、CがFalseのとき結論はEである」を表現しています。

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

> **Curry-Howard同型対応**：型族は「命題即ち型」を最も直接的に体現しています。`Add: (A: Type, B: Type) -> Type`は「型レベルで加算関数を書いた」のではなく、**自然数加算に関する命題を構築**しています。`(Zero, B) => B`は「命題Add(Zero, B)はBと同値である」を意味し、`(Succ(A'), B) => Succ(Add(A', B))`は「Add(A', B)이 성립하면 Add(Succ(A'), B)도 성립한다」を意味しています。これはPeano公理における加算の定義そのものです。型チェッカーがこのmatch式を検証することは、この定義の論理的整合性を検証することと同等です。

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

# 型レベル加算（Curry-Howard：これも自然数加算の帰納的定義）
Add: (A: Type, B: Type) -> Type = match (A, B) {
    (Zero, B) => B,
    (Succ(A'), B) => Succ(Add(A', B)),
}

# 例：コンパイル時に2 + 3を計算
Two: Type = Succ(Succ(Zero))
Three: Type = Succ(Succ(Succ(Zero)))
Five: Type = Add[Two, Three]  # Succ(Succ(Succ(Succ(Succ(Zero)))))
```

### 6. 関数オーバーロード特化

#### 6.1 基本特化

```yaoxiang
# 基本特化：関数オーバーロードを使用（コンパイラが自動選択）
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

#### 6.2 条件特化

```yaoxiang
# RFC-010構文に完全に準拠した特化方式：関数オーバーロード

# 具象型特化
sum: (arr: Array(Int)) -> Int = {
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Array(Float)) -> Float = {
    return simd_sum_float(arr.data, arr.length)
}

# ジェネリック実装（コンパイラが自動選択）
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

# コンパイラが自動選択
sum(int_arr)     # sum: (Array(Int)) -> Intを選択
sum(float_arr)    # sum: (Array(Float)) -> Floatを選択
```

#### 6.3 関数オーバーロードとインライン最適化の完璧な融合

**鍵となる特性**：関数オーバーロードとインライン最適化は自然に融合し、ゼロコスト抽象化を実現します。

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
# コンパイラが自動選択してからインライン化
result = native_sum_int(int_arr.data, int_arr.length)

# 手書き最適化コードと完全に等価、関数呼び出しオーバーヘッドなし！
```

**コアな優位性**：

1. **コンパイラの知的選択**
   ```yaoxiang
   sum(int_arr)      # 自動選択 sum: (Array(Int)) -> Int
   sum(float_arr)    # 自動選択 sum: (Array(Float)) -> Float
   sum(custom_arr)  # 自動選択 sum: (T: Type) -> ((arr: Array(T)) -> T)
   ```

2. **インライン最適化**
   - 小関数は自動的に呼び出し点にインライン化
   - 関数呼び出しオーバーヘッドゼロ
   - 手書き最適化コードと完全に等価

3. **型安全性**
   - コンパイル時の型チェック
   - ランタイムオーバーヘッドゼロ
   - 仮想関数テーブル不要

4. **RFC-010との完璧な整合**
   ```yaoxiang
   # 統一構文を完全に使用
   name: type = value
   # impl、whereなどの新しいキーワードが不要
   ```

**実際の応用例**：

```yaoxiang
# パフォーマンス重視の数値計算
fibonacci: (n: Int) -> Int = {
    if n <= 1 { return n }
    return fibonacci(n - 1) + fibonacci(n - 2)
}

fibonacci: (n: Float) -> Float = {
    # Binetの公式を使用
    phi = (1.0 + 5.0.sqrt()) / 2.0
    return (phi.pow(n) - (-phi).pow(-n)) / 5.0.sqrt()
}

# コンパイラが自動選択してインライン化
fibonacci(10)      # Intバージョンを選択、完全インライン化
fibonacci(10.5)    # Floatバージョンを選択、Binetの公式を使用
```

**これは何を意味するのか？**

- ✅ **ジェネリック特化** → 関数オーバーロードで自然に解決
- ✅ **パフォーマンス最適化** → インライン化が自動完了
- ✅ **コード再利用** → 1つの関数名、複数の実装
- ✅ **ゼロコスト抽象化** → コンパイル時多相、ランタイムオーバーヘッドゼロ
- ✅ **新しいキーワード不要** → RFC-010統一構文に完璧に準拠
```

### 7. デッドコード除去メカニズム

#### 7.1 インスタンス化グラフ分析

```rust
// コンパイラ内部：ジェネリックインスタンス化依存グラフを構築
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

#### 7.2 使用点分析

```yaoxiang
# ソースコード分析
map: (T: Type, R: Type)(list: List(T), f: Fn(T) -> R) -> List(R) = ...

# 使用点1：map(Int, Int)をインスタンス化
int_list = List(1, 2, 3)
doubled = map(int_list, (x) => x * 2)  # map[Int, Int]が必要

# 使用点2：map(String, String)をインスタンス化
string_list = List("a", "b", "c")
uppercased = map(string_list, (s) => s.to_uppercase())  # map[String, String]が必要

# 未使用：map[Float, Float]など
# これらのジェネリックインスタンスは生成されない

# コンパイル後は使用されたインスタンスのみを含む
map_Int_Int: (list: List(Int), f: Fn(Int) -> Int) -> List(Int) = ...
map_String_String: (list: List(String), f: Fn(String) -> String) -> List(String) = ...
```

#### 7.3 コンパイル時ジェネリックDCE

```yaoxiang
# コンパイル時分析：コンパイル時ジェネリックの使用状況
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
}

# 実際の使用状況
arr_10_int = Array(Int, 10)(1, 2, 3, 4, 5, 6, 7, 8, 9, 10)
arr_100_int = Array(Int, 100)(...)

# コンパイル後は使用されたSizeのみを生成
Array_Int_10: (Array(Int, 10)) = ...
Array_Int_100: (Array(Int, 100)) = ...

# 未使用のSizeは生成されない
# Array(Int, 50) は生成されない
```

#### 7.4 モジュール間DCE

```yaoxiang
# モジュールA
# A.yx
pub map: (T: Type, R: Type)(list: List(T), f: Fn(T) -> R) -> List(R) = ...

# モジュールB
# B.yx
use A.{map}
int_list = List(1, 2, 3)
doubled = map(int_list, (x) => x * 2)  # map(Int, Int)をインスタンス化

# モジュールC
# C.yx
use A.{map}
string_list = List("a", "b", "c")
uppercased = map(string_list, (s) => s.to_uppercase())  # map(String, String)をインスタンス化

# コンパイル分析：
# - モジュールBはmap[Int, Int]を使用
# - モジュールCはmap[String, String]を使用
# - コンパイル後のバイナリにはこの2つのインスタンスのみが含まれる
```

#### 7.5 LLVMレベルDCE

```rust
// コンパイルパイプライン
fn optimize_ir(ir: &mut IR) {
    // 1. モノリゼーション（YaoXiangコンパイラ）
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

    // 6. 最適化を実行
    llvm_ir.run_optimization_passes();
}
```

### 8. マクロ代替策略

#### 8.1 コード生成代替

```yaoxiang
# ❌ マクロ案：コード生成
macro_rules! impl_debug {
    ($($t:ty),*) => {
        $(impl Debug for $t {
            fn fmt(&self, f: &mut Formatter) -> Result {
                write!(f, "{:?}", self)
            }
        })*
    };
}

# ✅ ジェネリック案：自動導出
# 関数オーバーロードを使用して自動導出
debug_fmt: (T: fields...) -> ((self: Point(T)) -> String) = {
    return "Point { x: " + self.x.to_string() + ", y: " + self.y.to_string() + " }"
}

# 使用
p = Point { x: 1, y: 2 }
p.debug_fmt(&formatter)  # 自動生成呼び出し
```

#### 8.2 DSL代替

```yaoxiang
# ❌ マクロ案：HTML DSL
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

# ✅ ジェネリック案：型安全ビルダー
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

# DOMを構築
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
# ❌ マクロ案：型レベル計算
macro_rules! add_types {
    ($a:ty, $b:ty) => {
        ($a, $b)
    };
}

# ✅ ジェネリック案：条件型
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
result_type = Add[Int, Float]  # Floatに導出
```

### 9. 例

#### 9.1 完全ジェネリックコンテナ例

```yaoxiang
# ======== 1. ジェネリックコンテナを定義 ========
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

    # ジェネリックメソッド（Tは外側のList(T)から自動的にスコープ導入）
    push: (self: List(T), item: T) -> Void,
    pop: (self: List(T)) -> Option(T),
    map: (R: Type) -> ((self: List(T), f: (T) -> R) -> List(R)),
    filter: (self: List(T), predicate: (T) -> Bool) -> List(T),
    fold: (U: Type) -> ((self: List(T), initial: U, f: (U, T) -> U) -> U),
}

# ======== 2. ジェネリックメソッドを実装 ========
# Type.method 構文糖衣を使用：List型に自動関連付け

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

# ======== 3. 型制約を使用 ========
# ListにCloneを実装
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

# foldを使用して計算
sum = numbers.fold(0, (acc, x) => acc + x)  # sum = 6

# ジェネリック組み合わせ
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
# 関数オーバーロードを使用して実装
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

    # コンパイル時次元検証：Assert標準ライブラリ型を利用
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
# コンパイル時にサイズが既知の行列を作成
# 2x3行列
matrix_2x3 = Matrix(Float, 2, 3)()
matrix_2x3.data[0][0] = 1.0
matrix_2x3.data[0][1] = 2.0
matrix_2x3.data[0][2] = 3.0
matrix_2x3.data[1][0] = 4.0
matrix_2x3.data[1][1] = 5.0
matrix_2x3.data[1][2] = 6.0

# 3x2行列
matrix_3x2 = Matrix(Float, 3, 2)()
matrix_3x2.data[0][0] = 7.0
matrix_3x2.data[0][1] = 8.0
matrix_3x2.data[1][0] = 9.0
matrix_3x2.data[1][1] = 10.0
matrix_3x2.data[2][0] = 11.0
matrix_3x2.data[2][1] = 12.0

# 行列乗算：2x3 * 3x2 = 2x2
result = matrix_2x3.multiply(matrix_3x2)

# コンパイル時検証：resultの型はMatrix(Float, 2, 2)
# 2x2単位行列
identity_3x3 = identity(Float, 3)()

# 次元不一致：コンパイルエラー
# bad_multiply = matrix_2x3.multiply(identity_3x3)  # コンパイルエラー：3x3 != 2x3
```

## トレードオフ

### 優位性

1. **ゼロコスト抽象化**
   - コンパイル時モノリゼーション、ランタイムオーバーヘッドなし
   - 仮想関数不要、RTTI不要

2. **デッドコード除去**
   - コンパイル時分析、使用されたジェネリックのみをインスタンス化
   - コード膨張は制御可能

3. **マクロ代替**
   - 型安全なコード生成
   - IDEフレンドリー、明確なエラー情報

4. **コンパイル時計算**
   - コンパイル時ジェネリックがコンパイル時計算をサポート
   - 次元検証などの特性
   - `const`キーワード不要、純粋な型制約

### 劣位性

1. **コンパイル時間**
   - ジェネリックインスタンス化によりコンパイル時間が増加
   - 制約解決が遅い可能性がある

2. **メモリ使用量**
   - コンパイラのメモリ使用量が増加
   - キャッシュ機構にはメモリが必要

3. **実装複雑性**
   - 制約解決器が複雑
   - 型レベル計算エンジンが複雑

4. **エラー診断**
   - ジェネリックエラーが複雑である可能性がある
   - 明確なエラー提示が必要

### 緩和措施

1. **キャッシュ戦略**
   - インスタンス化結果をキャッシュ
   - LRUキャッシュでメモリを制限

2. **增量コンパイル**
   - コンパイル結果をキャッシュ
   - 增量インスタンス化

3. **エラー提示**
   - 明確なエラー情報
   - ジェネリックパラメータ推論のヒント

4. **並列コンパイル**
   - ジェネリックの並列インスタンス化
   - マルチスレッド制約解決

## 代替方案

| 方案 | 選択しない理由 |
|------|---------------|
| のみ基本ジェネリック | 複雑なマクロを代替できない |
| のみマクロシステム | 型安全性がない、エラー情報が悪い |
| のみ制約に依存 | 柔軟性が不足 |
| ランタイムジェネリック | パフォーマンスオーバーヘッドあり |

### リスク

| リスク | 影響 | 緩和措施 |
|--------|------|----------|
| 制約解決複雑性 | コンパイル時間が長すぎる | 增量解決 + キャッシュ |
| コード膨張 | バイナリファイルが大きすぎる | DCE + しきい値制御 |
| 実装複雑性 | 開発サイクルが延長 | 段階的実装 |
| エラー診断 | ユーザー体験が悪い | 詳細なエラー情報 |

## 開放問題

### 審議待ち問題

| 議題 | 説明 | ステータス |
|------|------|-----------|
| インスタンス化戦略 | Eager vs Lazy vs Threshold | 審議中 |
| キャッシュサイズ | LRUキャッシュ容量設定 | 審議中 |
| エラー診断 | ジェネリックエラー情報の詳細度 | 審議中 |

### 后续最適化

| 最適化項目 | 価値 | 実装難易度 |
|-----------|------|-----------|
| インスタンス化グラフ分析 | 高 | 中 |
| 型レベルプログラミングDSL | 中 | 高 |
| ジェネリックパフォーマンスベンチマーク | 中 | 低 |

## 付録

### 構文BNF

```bnf
# ジェネリックパラメータは統一()構文を使用し、関数型の一部として機能
# 例：map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R))

# 型制約（ジェネリックパラメータ内）
type_bound ::= identifier
             | identifier '+' identifier ('+' identifier)*

# パラメータ宣言（型 + 名前）
parameter ::= identifier ':' type

parameters ::= parameter (',' parameter)*

# 関数宣言：name: type = expression
# ジェネリックパラメータは関数型の最初の引数グループ：(T: Type) -> ((params) -> return)
function ::= identifier ':' type '=' (expression | block)

# メソッド宣言：Type.method: type = expression
method ::= identifier '.' identifier ':' type '=' (expression | block)

# 型定義（統一Binding構文）
# ジェネリック型例：List: (T: Type) -> Type = { ... }
generic_type ::= identifier ':' type '=' type_expression

# ジェネリックパラメータ内のTypeはコンパイラが実引数の型から自動補完
# 例：map(numbers, f)、Tはnumbers: List(Int)から抽出、Rはf: (Int) -> Stringから抽出
```

## ライフサイクルと歸趨

```
┌─────────────┐
│   草案      │  ← 現在のステータス
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  審査中     │  ← コミュニティの議論とフィードバックを募集中
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
│ (正式設計)  │    │ (元の位置を保持) │
└─────────────┘    └─────────────┘
```

---

## 参考文献

### YaoXiang公式ドキュメント

- [RFC-010: 統一型構文](./010-unified-type-syntax.md)
- [RFC-009: 所有権モデル](./accepted/009-ownership-model.md)
- [RFC-001: 並作モデル](./accepted/001-concurrent-model-error-handling.md)
- [RFC-008: 実行時モデル](./accepted/008-runtime-concurrency-model.md)
- [言語仕様](../language-spec.md)
- [YaoXiangガイド](../guides/YaoXiang-book.md)

### 外部参照

- [Rustジェネリックシステム](https://doc.rust-lang.org/book/ch10-01-syntax.html)
- [C++テンプレート特化](https://en.cppreference.com/w/cpp/language/template_specialization)
- [Haskell型クラス](https://www.haskell.org/tutorial/classes.html)
- [Swiftジェネリック](https://docs.swift.org/swift-book/LanguageGuide/Generics.html)
- [モノリゼーション最適化](https://llvm.org/docs/Monomorphization.html)
- [デッドコード除去](https://en.wikipedia.org/wiki/Dead_code_elimination)
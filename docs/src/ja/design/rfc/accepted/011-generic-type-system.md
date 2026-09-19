---
title: 'RFC-011: ジェネリクスシステム設計 - ゼロコスト抽象とマクロ代替'
status: '受理済み'
author: '晨煦'
updated: '2026-07-15（型体コードブロック + コンパイル時規約 + エフェクトシード実装済み）'
issue: '#128'
issues_impl:
  - '#45'
  - '#46'
  - '#73'
  - '#90'
  - '#96'
  - '#40'
  - '#151'
pr_impl:
  - '#122'
---

# RFC-011: ジェネリクスシステム設計 - ゼロコスト抽象とマクロ代替

## 摘要

本文書はYaoXiang言語の**ジェネリクスシステム設計**を定義する。強力なジェネリクス能力によりゼロコスト抽象を実現し、コンパイル時最適化によってマクロへの依存を減らし、デッドコード除去機構を提供する。

**中核設計**：

- **統一シグネチャ構文**：`(T: Type, R: Type) -> ...` ジェネリクス引数と通常引数の統一
- **Type 自己記述機構**：`Type` は言語レベルの特殊存在であり、シグネチャ内の `Type`
  位置は自動的に推論・充填される
- **型制約**：`T: Dup + Add` 多重制約、関数型制約
- **関連型**：`Iterator: (Item: Type) -> Type = { next: () -> Option(Item), has_next: () -> Bool }`
- **コンパイル時ジェネリクス**：`N: Int` ジェネリクス値引数、コンパイル時定数のインスタンス化
- **条件型**：`If: (C: Bool, T: Type, E: Type) -> Type` 型レベル計算、型族

**価値**：

- ゼロコスト抽象：コンパイル時単相化、ランタイムオーバーヘッドなし
- デッドコード除去：インスタンス化グラフ解析 + LLVM最適化
- マクロ代替：ジェネリクスがマクロ使用シーンの90%を代替
- 型安全性：コンパイル時検査、IDE親和性
- **明示は暗黙に優る**：`Type` 自己記述、コンパイラ自動推論

## 参考文書

本文書の設計は以下の文書に基づいている：

| 文書                                                                              | 関係                         | 説明                                                           |
| --------------------------------------------------------------------------------- | ---------------------------- | -------------------------------------------------------------- |
| [RFC-010: 統一型構文](./010-unified-type-syntax.md)                               | **構文基盤**                 | ジェネリクス構文と統一 `name: type = value` モデル統合         |
| [RFC-010: 統一型構文](./010-unified-type-syntax.md)                               | **呼び出し構文**             | 第6節：ジェネリクス呼び出し構文——統一 `()` 適用、`[]` 完全削除 |
| [RFC-009: 所有権モデル](./accepted/009-ownership-model.md)                        | **型システム**               | Move セマンティクスとジェネリクスの自然な結合                  |
| [RFC-024: spawn ベースの並行ランタイムセマンティクス](./024-concurrency-model.md) | **実行モデル**               | DAG解析とジェネリクス型検査                                    |
| [RFC-008: ランタイムモデル](./accepted/008-runtime-concurrency-model.md)          | **コンパイラアーキテクチャ** | ジェネリクス単相化とコンパイル時最適化戦略                     |
| [型宇宙思想](../reference/plan/ongoing/类型宇宙思想.md)                           | **理論的核心**               | 型宇宙階層モデルと値依存型設計                                 |
| [RFC-027: コンパイル時述語と統一静的検証](./027-compile-time-evaluation-types.md) | **停止検査**                 | 自動度量合成とコンパイル時評価の安全保障                       |

## 型宇宙思想と値依存型

YaoXiang のジェネリクスシステムは**型宇宙思想**の上に構築されており、このメンタルモデルは言語内のすべての概念を階層構造として統一する。中核的革新は**値依存型**を Type2 層のファーストクラス市民に引き上げたことにある。

### 値依存型とは何か？

**値依存型**は、一つまたは複数の**値**（他の型だけでなく）に依存する型である。これらの値はコンパイル時に評価でき、コンパイル段階で型安全性の保証を提供する。

```yaoxiang
# 従来ジェネリクス：型引数
List: (T: Type) -> Type

# 値依存型：値引数
Array: (T: Type, N: Int) -> Type  # 配列型は長さの値 N に依存する
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type  # 行列型は行数と列数に依存する
```

### コンテナ型命名階層

言語層には3つのコンテナ概念があり、長さ情報の帰属こそが根本的な違いである：

| 型            | 長さ         | セマンティクス                           | 底层                                        |
| ------------- | ------------ | ---------------------------------------- | ------------------------------------------- |
| `Array(T, N)` | 型           | **固定長**配列、N は型に含まれる         | 中核プリミティブ（スタック/インライン優先） |
| `Vec(T)`      | ランタイム値 | **ランタイム長の原始バッファ**、拡張可能 | 中核プリミティブ（ヒープ上の連続バッファ）  |
| `List(T)`     | ランタイム値 | 標準ライブラリ型                         | ライブラリ：`{ data: Vec(T), length: Int }` |

三者の分業原則：

- **`Array(T, N)`
  は唯一長さを型に含める形式である**——長さはコンパイル時定数なので、境界失敗のコンパイル時拒否が可能（`a[5]`
  が `a: Array(Int, 3)` の場合、直接コンパイル時エラー。下記の「コンパイル時次元検証」を参照）。
- **`Vec(T)`
  はランタイム長の最小基盤である**——「割り当て可能、長さ取得、読み書き、拡張可能」の4つだけを提供し、容量戦略、拡張係数、縮小可否は一切行わない。他のコンテナを構築する原料である。
- **`List(T)` はライブラリ型であり、プリミティブではない**——YaoXiang 自身で `std.list`
  に定義され（`{ data: Vec(T), length: Int }`）、ユーザー定義のジェネリクスレコードと同等の扱いを受ける。拡張可能セマンティクスのすべての戦略（いつ拡張するか、どれだけ拡張するか、共有可能か）はライブラリ内にあり、コンパイラは関与しない。

`Vec(T)` の構築形式（二層：まず型引数、次に構築引数）：

```yaoxiang
# 空構築——長さ 0、要素は事後追加
v = Vec(Int)()

# 要素構築——長さは要素数で決定
w = Vec(Int)(1, 2, 3)          # 長さ 3

# スロット割り当て——n 個のゼロ値スロットを割り当て
buf = Vec(Int)(len=64)         # 長さ 64、要素はすべてゼロ値
```

> スロット割り当ては**フィールド名形式**（`len=`）を使用し、位置形式ではない：位置形式の単一整数は「単一要素ベクトル」と曖昧になる（`Vec(Int)(64)`
> では「長さ 64」と「要素 64 を含む」を区別できない）。これはジェネリクス構築の統一ルールと一致する：フィールド名形式実引数は名前でバインドされ、位置推論の影響を受けない。
>
> これは `List` 拡張に必要な唯一のプリミティブである——`List`
> は必要に応じて新しいスロットを割り当てて要素を移動する：
>
> ```yaoxiang
> new_data = Vec(T)(len=self.data.length * 2)
> ```
>
> いつ拡張するか、どれだけ拡張するか、縮小するかどうかはすべて `List` が決定する。`Vec`
> は容量戦略を行わない。

下から上へ、パフォーマンスは低下し、柔軟性は向上する：`Array` > `Vec` > `List`。

> 命名の根拠：`Vec`/`vector`
> は主流言語（Rust/C++）ではランタイム長の拡張可能シーケンスを指す；`Array` は固定長を指す。

### 値依存型の中核的優位性

従来のジェネリクスと比較して、YaoXiang の値依存型には以下の中核的優位性がある：

| 特性             | 従来ジェネリクス (C++/Rust)                 | YaoXiang 値依存型                                  |
| ---------------- | ------------------------------------------- | -------------------------------------------------- |
| 型が依存する値   | 型引数のみ                                  | 関数呼び出し結果を含む任意の値                     |
| コンパイル時評価 | C++テンプレート手動特殊化、 Rust なし       | 自動コンパイル時評価、停止保証                     |
| 型レベル計算     | テンプレートメタプログラミング（複雑/危険） | 統一された型レベル計算エンジン                     |
| 型安全性         | C++ なし、Rust 制限あり                     | 完全な型安全性、コンパイル時検査                   |
| 次元検証         | ランタイム検査または手動特殊化              | コンパイル時次元検証、ランタイムオーバーヘッドなし |

### 型宇宙階層と値依存型

型宇宙思想は言語概念を意味的役割によって異なる階層に分割し、値依存型は **Type2 層**に位置する：

| 階層      | 役割                               | 例                                                                                                              |
| --------- | ---------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| Type-1    | 値                                 | `42`, `factorial(5)`, 関数本身                                                                                  |
| Type0     | メタ型キーワード                   | `Type`                                                                                                          |
| Type1     | 具体型                             | `Int`, `String`, `Array(Int, 3)`                                                                                |
| **Type2** | **関数/型コンストラクタ/値依存型** | `add: (Int, Int) -> Int`, `Array: (T: Type, N: Int) -> Type`, `Matrix: (T: Type, Rows: Int, Cols: Int) -> Type` |

**鍵となる設計**：Type2 層の関数、型コンストラクタ、値依存型は**統一構文**であり、すべて
`(params) -> result` の形式である：

- 通常関数：`(Int, Int) -> Int` → 戻り値は値
- 型コンストラクタ：`(T: Type) -> Type` → 戻り値は型
- 値依存型：`(T: Type, N: Int) -> Type` → 戻り値は型であり、値引数 N に依存する

> **Curry-Howard 同型**：この統一は偶然ではない。Curry-Howard 同型は「型は命题、プログラムは証明」と指摘する——関数型
> `A → B` は論理的蕴含「A ならば B」に対応し、ジェネリクス `(T: Type) -> Type`
> は全称量化「すべての型 T について」に対応し、値依存型 `(n: Int) -> Type`
> は「各整数 n に対して型が存在する」に対応する。YaoXiang は関数、型コンストラクタ、値依存型を Type2 層に統一し、本質的に「証明」と「計算」を同一の概念——**構成的証明**——として統一する。これは Curry-Howard 同型を言語設計に直接反映したものである：一つの形式（`(params) -> result`）が論理的命题と計算過程を同時に担う。

### コンパイル時決定性の保証

YaoXiang の型宇宙思想は要求する：**Type 階層のすべてはコンパイル時に決定される**。

```yaoxiang
# コンパイル時次元検証の例
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),
    # コンパイル時検査：次元は正でなければならない
    _assert: Assert(Rows > 0),
    _assert: Assert(Cols > 0),
}

# 3x3 単位行列を作成 - コンパイル時に完了
identity: (T: Add + Zero + One, N: Int) -> ((size: N) -> Matrix(T, N, N)) = {
    matrix = Matrix(T, N, N)()
    # ...
}

# コンパイル時計算：factorial(3) = 6、配列サイズはコンパイル時に決定
arr: Array(Int, factorial(3)) = Array(Int, 6)()
```

コンパイラは自動的に：

1. 型位置での関数呼び出しを検出する
2. 関数に対してコンパイル時停止検査を実行する（下記の停止検査機構を参照）
3. コンパイル時に評価を実行する
4. 結果を生成された型に埋め込む

### 値依存型の応用シーン

#### コンパイル時次元検証

```yaoxiang
# 行列乗算：コンパイル時に次元の整合を検証
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

# N はコンパイル時定数なので、型レベル計算に使用可能
first_three: Array(Int, 3) = Array(Int, 3)(1, 2, 3)
# first_three.length == 3（コンパイル時に既知）
```

#### 境界失敗のコンパイル時カバレッジ目標

> **実装状況の説明**：コンテナ型は特殊化解除済み——`Array(T, N)`
> は const ジェネリクスコンストラクタであり、リテラルコンテキスト落点、`in`
> membership 述語はどちらも実装済み。 `Array(T, N)`
> リテラル落点の N と要素型はコンパイル時検査により強制されている（E1002）、**N は信頼できる**——本節の目的メカニズムは「注釈 N
> == ランタイム長」の上に構築可能。現在の `[]`
> インデックス範囲外（E6003）と Dict キー欠落（E6008）は**ランタイムエラー過渡状態**；本節の値依存型はこれらの境界失敗を**コンパイル時**に押し下げる目標メカニズムである：
>
> - const インデックス：`a[5]`（5 はコンパイル時定数）が `a: Array(Int, 3)`
>   の場合、直接コンパイル時拒否；
> - 値インデックス：`a[i]` には前置条件 `i < len(a)` が必要、値依存型契約により証明；
> - `in` 述語はホーア論理前置条件の基底：`n in 1..10`、`x in some_set`
>   はいずれもコンパイル時に証明可能な命题。
>
> 精化型の完全な設計は実装時に別途補足する。

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

#### ジェネリクス関数

```yaoxiang
# map: ジェネリクス関数、型引数 T, R はコンパイル時に決定
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
numbers = List(Int)()   # 訂正：値構築二層形式（§9.1 参照）；要素は push で填充
numbers.push(1)
numbers.push(2)
numbers.push(3)
doubled = map(numbers, (x) => x * 2)  # map[Int, Int] と推論
```

### 他の言語との比較

| 特性                                  | C++テンプレート            | Rust ジェネリクス | Haskell GADT | **YaoXiang**                 |
| ------------------------------------- | -------------------------- | ----------------- | ------------ | ---------------------------- |
| 型引数                                | ✅                         | ✅                | ✅           | ✅                           |
| 値依存型                              | ❌                         | ❌                | ✅           | ✅                           |
| コンパイル時評価                      | テンプレートインスタンス化 | ❌                | ✅           | ✅                           |
| 停止保証                              | ❌                         | ❌                | ❌（危険）   | ✅（自動度量合成、RFC-027）  |
| 型安全性                              | ❌（マクロ展開）           | ✅                | ✅           | ✅                           |
| 統一構文                              | ❌                         | ❌                | ❌           | ✅                           |
| コンパイル時次元検証                  | 手動特殊化                 | ランタイム検査    | 型族         | コンパイル時自動検証         |
| 半自動停止注釈（decreases/invariant） | ❌                         | ❌                | ❌           | ❌（コンパイル時全自动のみ） |

### 停止検査機構（RFC-027 と統一）

値依存型のコンパイル時評価は**停止を保証**しなければならず、そうでなければ型システムが無限ループに陥る。停止検査は RFC-027 のコンパイル時証明パイプラインにより
**全自动**で完了する——コンパイラが自動的に度量を合成し、証明可能な再帰/ループは通過、証明不可能なものは直接コンパイルエラー。**半自動注釈用の余地は設けない**：RFC-022 の
`//! decreases`、`/*! invariant !*/` は RFC-022 廃止に伴い廃止済み、規約とは型注記そのものである。

#### 再帰関数の停止検査

コンパイラはコンパイル時評価前に、再帰呼び出しの引数が各再帰パスで厳密に減少していることを検査する（RFC-027
§6.7）。いかなる規約コメントも不要：

```yaoxiang
# コンパイル時階乗：//! requires/ensures/decreases 不要、コンパイラが自動分析
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)  # コンパイラ分析：n-1 < n → 減少 → 停止
}

# 使用：型位置で呼び出し、コンパイラがまず停止を検証してから評価
arr: Array(Int, factorial(5)) = Array(Int, 120)()  # コンパイル時に factorial(5) = 120 を評価
```

| シーン                                        | 動作             |
| --------------------------------------------- | ---------------- |
| コンパイラが再帰の減少を分析可能（例：`n-1`） | コンパイル時評価 |
| 減少しない/減少判定不可                       | コンパイルエラー |
| ランタイム呼び出し（型位置でない）            | 停止検査不要     |

#### ループの停止検査

ループには `: Invariant(...)` や `: decreases(...)`
注釈は不要。変数の精化型注釈（例：`UpTo(n)`）がループ不変条件と度量境界を同時に提供し、コンパイラは優先順位に従って4つの度量合成戦略を試し、一つ見つかれば停止する（RFC-027
§7）：

1. **線形ランク関数自動合成**——型注釈から変数の界を抽出し、線形組み合わせを列挙、SMT が m ≥
   0 かつすべてのパスで m' < m を検証
2. **述語違反カウント**（実験的）——目標型定義（例：`Sorted`）から violation_count を抽出し、隣接交換/移動をカバー
3. **有界増減/減少パターン**——`v += const` → 度量 `upper - v`（戦略 1 の退化、最速パス）
4. **乗法スケーリング度量テンプレート**——`v *= const`（const > 1）→ 度量
   `ceil(log_const(upper / v))`

```yaoxiang
sum: (arr: Array(Int, n)) -> Int = {
    mut i: UpTo(arr.len) = 0   # 型注釈が上界 arr.len と下界 0 を与える
    while i < arr.len {
        # コンパイラが自動推論：度量 arr.len - i、各反復で厳密に 1 減少 → 停止が証明される
        s += arr[i]; i += 1
    }
    return s
}
```

#### 停止検査のワークフロー

```
┌─────────────────────────────────────────────────────────────┐
│  型検査フェーズ                                            │
│  型位置での関数呼び出しに遭遇（例：factorial(5)）          │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  1. 停止検査（RFC-027 証明パイプライン、全自动）           │
│     - 再帰関数：各再帰パスで引数が厳密に減少しているか検査 │
│     - ループ：4つの度量合成戦略（線形ランク/違反カウント/  │
│       有界パターン/乗法スケーリング）、SMT が減少を検証    │
│     - 証明不能 → コンパイルエラー（硬境界、半自動注釈の     │
│       フォールバックなし）                                  │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  2. コンパイル時評価（組み込みインタプリタで実行）         │
│     - 純粋関数：直接評価                                    │
│     - 副作用：コンパイルエラー（型位置は副作用なしでなければ │
│       ならない）                                            │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  3. 結果を型に埋め込む                                      │
│     - Array(Int, factorial(5)) → Array(Int, 120)           │
│     - Matrix(Float, 3, 3) → 具体型                          │
└─────────────────────────────────────────────────────────────┘
```

#### 優位性

- **安全性**：コンパイル時評価が必ず停止することを保証し、型システムの無限ループを回避
- **統一性**：停止検査と正当性検証（VC 生成）が同一のコンパイル時証明パイプライン（RFC-027）を共有、独立した規約構文なし
- **全自动**：コンパイラが型注釈から自動的に度量を合成し、証明可能なら通過、不可能ならエラー——`decreases`
  の手書きに依存しない

## 動機

### なぜ強力なジェネリクスシステムが必要か？

現在の主流言語のジェネリクスには限界がある：

| 言語         | ジェネリクス能力     | 問題                                                 |
| ------------ | -------------------- | ---------------------------------------------------- |
| Java         | 境界型               | コンパイル時単相化、ジェネリクス特殊化なし           |
| C#           | ジェネリクス制約     | ランタイム型検査、パフォーマンスオーバーヘッド       |
| Rust         | ジェネリクス + Trait | Trait システムが複雑、学習曲線が急峻                 |
| C++          | テンプレート         | テンプレート特殊化が複雑、コンパイルエラー情報が悪い |
| **YaoXiang** | **値依存型**         | **型が値に依存可能、コンパイル時次元検証、停止保証** |

### 中核的矛盾

1. **性能 vs 柔軟性**：ランタイム柔軟性 vs コンパイル時最適化
2. **複雑 vs 簡潔**：強力な型システム vs 使いやすさ
3. **マクロ vs ジェネリクス**：マクロコード生成 vs ジェネリクス型安全性
4. **値依存 vs 型安全性**：従来ジェネリクスはコンパイル時に次元を検証できない

### 値依存型の中核的優位性

YaoXiang の**値依存型**は従来ジェネリクスに対する中核的な優位性である：

| 優位性               | 説明                                                                       |
| -------------------- | -------------------------------------------------------------------------- |
| **型が値に依存**     | `Array: (T: Type, N: Int) -> Type` により型が具体的な値に依存可能          |
| **コンパイル時評価** | 型位置での関数呼び出しはコンパイル時に評価され、結果は型に直接埋め込まれる |
| **次元検証**         | `Matrix(Float, 3, 3)` はコンパイル時に行列次元を検証                       |
| **型レベル計算**     | `If`、`Match` などの条件型が型レベル計算をサポート                         |
| **停止保証**         | コンパイル時停止検査（自動度量合成）がコンパイル時評価の必然的停止を保証   |

```yaoxiang
# C++/Rust には不可能なコンパイル時検証
matrix: Matrix(Float, factorial(3), factorial(2)) = ...
# コンパイル時計算：factorial(3) = 6, factorial(2) = 2
# 型は Matrix(Float, 6, 2)

# 次元不一致はコンパイル時に捕捉される
identity: Matrix(Float, 3, 3) = ...
# multiply(matrix_2x3, identity_3x3)  # コンパイルエラー：2 != 3
```

### ジェネリクスシステムの価値

```yaoxiang
# 例：統一 API 設計
# 異なるコンテナ型の map 操作

# 従来方案：型ごとに個別実装
map_int_array: (array: Vec(Int), f: Fn(Int) -> Int) -> Vec(Int) = ...
map_string_array: (array: Vec(String), f: Fn(String) -> String) -> Vec(String) = ...
map_int_list: (list: List(Int), f: Fn(Int) -> Int) -> List(Int) = ...
map_string_list: (list: List(String), f: Fn(String) -> String) -> List(String) = ...

# ジェネリクス方案：一つのジェネリクス関数ですべての型をカバー
map: (T: Type, R: Type)(container: Container(T), f: Fn(T) -> R) -> Container(R) = {
    for item in container {
        result.push(f(item))
    }
    result
}
```

## 設計目標

### 中核目標

1. **ゼロコスト抽象** - ジェネリクス呼び出しは具体型呼び出しと等価
2. **デッドコード除去** - コンパイル時分析、使用されるジェネリクスのみインスタンス化
3. **マクロ代替** - ジェネリクスがマクロ使用シーンの 90% を代替
4. **型安全性** - コンパイル時検査、ランタイム型オーバーヘッドなし
5. **IDE 親和性** - スマートヒント、明確なエラー情報
6. **値依存型** - 型が値に依存可能、コンパイル時次元検証をサポート
7. **コンパイル時評価の安全性** - コンパイル時停止検査（RFC-027 自動度量合成）により保証

### 設計原則

- **コンパイル時決定**：ジェネリクス引数はコンパイル時に決定
- **単相化優先**：具体コードを生成、仮想関数呼び出しを回避
- **制約駆動**：型制約がインスタンス化を指導
- **プラットフォーム最適化**：特殊化がプラットフォーム固有最適化をサポート
- **型宇宙統一**：関数/型コンストラクタ/値依存型を Type2 層に統一
- **停止保証**：型位置での関数呼び出しは停止を証明しなければならない

## 提案

### 1. 基礎ジェネリクス

#### 1.1 ジェネリクス型引数

> **鍵となるルール**：ジェネリクス型定義は**明示的に `: Type`
> を标注しなければならない**、さもないと HM により関数と推論される。
>
> | 書き方                            | 意味                           |
> | --------------------------------- | ------------------------------ |
> | `List: (T: Type) -> Type = {...}` | ✅ 型コンストラクタ            |
> | `List = {...}`                    | ❌ HM が関数と推論、型ではない |

```yaoxiang
# ジェネリクス型定義（: Type が必須）
Option: (T: Type) -> Type = {
    some: (T) -> Self,
    none: () -> Self
}

Result: (T: Type, E: Type) -> Type = {
    ok: (T) -> Self,
    err: (E) -> Self
}

List: (T: Type) -> Type = {
    data: Vec(T),
    length: Int,
    push: (self: List(T), item: T) -> Void,   # self は単なる慣例名で、キーワードではない
    get: (self: List(T), index: Int) -> Option(T),
}

# ジェネリクス関数（: Type なし、HM が関数と推論）
map: (T: Type, R: Type) -> ((opt: Option(T), f: Fn(T) -> R) -> Option(R)) = {
    return match opt {
        some => Option.some(f(some)),
        none => Option.none(),
    }
}

# ジェネリクス制約（直接式、単行なら return 省略可）
clone: (T: Clone)(value: T) -> T = value.clone()

# 多型引数
combine: (T: Type, U: Type) -> ((a: T, b: U) -> (T, U)) = (a, b)
```

### ジェネリクス関数呼び出し構文

#### 1.1 統一シグネチャ構文

```yaoxiang
# ジェネリクス関数は統一された (T: Type, R: Type) シグネチャ構文を使用
map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R)) = ...

# 多型引数
combine: (T: Type, U: Type) -> ((a: T, b: U) -> (T, U)) = (a, b)
```

#### 1.2 Type 自己記述機構

`Type` は言語レベルの特殊存在であり、コンパイラは元来シグネチャ内の `Type`
位置を認識でき、実際引数の型から自動的に推論・充填する。

```yaoxiang
# コンパイラがジェネリクス引数を自動推論
numbers: List(Int) = List(Int)()
#         ^^^^^^^^   ^^^^^^^^
#         型宣言     構築呼び出し：Int が T を充填、() 値構築

# 関数呼び出し推論
numbers: List(Int) = List(Int)()
f: (x: Int) -> String = (x) => x.to_string()
strings: List(String) = map(numbers, f)
# コンパイラ推論：T=Int, R=String
```

#### 1.3 単相化

```yaoxiang
# ソースコード
map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R)) = {
    result: List(R) = List(R)()
    for x in list {
        result.push(f(x))
    }
    return result
}

# 使用点
int_list: List(Int) = List(Int)()
doubled: List(Int) = map(int_list, (x: Int) => x * 2)  # map[Int, Int] をインスタンス化

string_list: List(String) = List(String)()
uppercased: List(String) = map(string_list, (s: String) => s.to_uppercase())  # map[String, String] をインスタンス化

# コンパイル後（等価コード）
map_Int_Int: (list: List(Int), f: (Int) -> Int) -> List(Int) = {
    result: List(Int) = List(Int)()
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

````yaoxiang
# 推論可能なら Type 引数を省略
numbers: List(Int) = List(Int)()
strings: List(String) = map(numbers, (x: Int) => x.to_string())

# 推論不能時は明示的充填が必須
# map(numbers, (x) => x)  # ❌ Error: Cannot infer R

### 2. 型制約システム

#### 2.1 単一制約

```yaoxiang
# 基本 trait 定義（インターフェース型）
Clone: Type = {
    clone: (Self) -> Self,
}

Display: Type = {
    fmt: (Self, Formatter) -> Result,
}

Debug: Type = {
    fmt: (Self, Formatter) -> Result,
}

# 制約使用：シグネチャ内で直接型制約を宣言
clone: (T: Clone) -> (value: T) -> T = value.clone()

debug_print: (T: Debug)(value: T) -> Void = {
    formatter = Formatter.new()
    value.fmt(formatter)
    print(formatter.to_string())
}
````

#### 2.2 多重制約

```yaoxiang
# 多重制約構文
combine: (T: Clone + Add)(a: T, b: T) -> T = {
    a.clone() + b
}

# ジェネリクスコンテナのソート
sort: (T: Clone + PartialOrd)(list: List(T)) -> List(T) = {
    # ソートアルゴリズムを実装
    result: List(T) = list.clone()
    quicksort(&mut result)
    return result
}

# 関数型制約
map: (T: Type, R: FnMut(T))(array: Vec(T), f: R) -> Vec(R) = {
    result: Vec(R) = Vec()
    for item in array {
        result.push(f(item))
    }
    return result
}

# 使用
doubled: Vec(Int) = map(Vec(1, 2, 3), (x: Int) => x * 2)  # コンパイラが推論
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

**三類のコピーセマンティクス**：

| 型                       | 意味                                              | トリガー方式      | 適用シーン                           |
| ------------------------ | ------------------------------------------------- | ----------------- | ------------------------------------ |
| **プリミティブ値コピー** | 代入時に自動値コピー、二つの値は完全独立          | 代入/引数渡し自動 | Int, Float, Bool, Char               |
| **Dup**                  | 浅いコピー：ハンドル/トークン複製、底层データ共有 | 代入/引数渡し自動 | `&T` トークン、`ref T`、String/Bytes |
| **Clone**                | 深いコピー：完全独立な複製を作成                  | `value.clone()`   | Clone を実装する任意の型             |

**Dup のセマンティクス**：Dup を実装した型は、代入/引数渡し時に所有権を移転しない——コンパイラがハンドル/トークンを複製し、複数の保持者が同一の底层データを指す。これは RFC-009 所有権モデルにおける Move デフォルトセマンティクスの補完である。

**Dup と Clone は直交する概念である**：

```
Dup = ハンドルを複製、データを共有（変更が相互に影響）
Clone = データを複製、コピーは独立（変更が相互に影響しない）
```

**ルール**：

```
1. プリミティブ値型（Int, Float, Bool, Char）— コンパイラ組み込み値コピー、Dup に属さない
2. Dup — 参照/トークン型と内部参照カウント型のみに適用
3. Clone — 明示的深いコピー、任意の型が実装可能
4. デフォルト Move — 他の型はデフォルトの Move セマンティクスを維持
```

**哪些の型が Dup か**：

| 型                       | Dup  | 理由                                                      |
| ------------------------ | ---- | --------------------------------------------------------- |
| `&T`（借用トークン）     | ✅   | ゼロサイズトークン、トークン複製 = 同一データへの複数視点 |
| `ref T`                  | ✅   | Rc/Arc 複製 = 参照カウント+1、ヒープデータ共有            |
| String, Bytes            | ✅   | 内部参照カウント、ハンドル複製で底层 buffer を共有        |
| `&mut T`（可变トークン） | ❌   | 線形独占、複製不可                                        |
| struct                   | 派生 | すべてのフィールドが Dup → struct Dup                     |
| enum                     | 派生 | すべての variant のすべてのフィールドが Dup → enum Dup    |
| tuple                    | 派生 | すべての要素が Dup → tuple Dup                            |
| Fn（クロージャ）         | ❌   | キャプチャ環境が Dup でない可能性あり                     |
| `*T`（生ポインタ）       | ❌   | unsafe、所有権システムに参加しない                        |

**Int/Float/Bool/Char は Dup ではない**——これらは値型であり、代入時にコンパイラが自動的に値コピーする（二つの値は完全独立）。これは「浅いコピー」ではなく、プリミティブに対するコンパイラ組み込み処理であり、Dup 型属性を通じて表現する必要はないし、そうすべきでもない。

```yaoxiang
# プリミティブ値型：コンパイラが自動値コピー（Dup ではない）
x: Int = 42
y = x          # 値コピー、x と y は完全独立
print(x)       # ✅

# Dup：浅いコピー、ハンドル複製でデータ共有
view: &Point = &point
view2 = view    # ✅ Dup：トークン複製、両者は同じ point を指す
print(view.x)   # ✅

# Clone：明示的深いコピー、独立したコピー作成
backup = big_struct.clone()  # 明示的呼び出し

# ジェネリクス制約
dup_use: (T: Dup) -> T = x         # T: Dup → 浅いコピー可能
clone_use: (T: Clone) -> T = x.clone()  # T: Clone → 深いコピー可能
```

> **注意**：`Send`/`Sync` はユーザー可視 trait としない。タスク横断安全保証は `ref`
> キーワードとコンパイラ全自动処理により行われる——`ref`
> が自動的に Rc または Arc を選択し、ユーザーは Send/Sync を理解する必要がない。

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

# Vec の Iterator 実装
# メソッド構文糖を使用：Vec.Item, Vec.next, Vec.has_next
# 反復位置は包装レコードが運ぶ（Vec 自体は原始バッファ、index フィールドなし）
VecIter: (T: Type) -> Type = {
    data: &Vec(T),
    index: Int,
}

VecIter.has_next: (T: Type)(self: &VecIter(T)) -> Bool = {
    return self.index < self.data.length
}

VecIter.next: (T: Type)(self: &mut VecIter(T)) -> Option(T) = {
    if self.index < self.data.length {
        item = self.data[self.index]
        self.index = self.index + 1
        return Option.some(item)
    } else {
        return Option.none()
    }
}

VecIter.Item: (T: Type)(arr: &VecIter(T)) -> T = {
    return arr.data[arr.index]
}
```

#### 3.2 ジェネリクス関連型（GAT）

```yaoxiang
# より複雑な関連型
Producer: (Item: Type) -> Type = {
    Item: T,
    produce: (Self) -> Option(Item),
}

# 関連型はジェネリクスになれる
Container: (Item: Type) -> Type = {
    Item: T,
    IteratorType: Iterator(Item),  # 関連型もジェネリクス
    iter: (Self) -> IteratorType,
}

# 使用
process_container: (T: Type, C: Container(T))(container: C) -> List(T) = {
    container.iter().collect()
}
```

### 4. コンパイル時ジェネリクス

#### 4.1 コンパイル時値引数

> **訂正**：原文「`Int`
> などの値引数はジェネリクスコンテキストでデフォルトでコンパイル時決定可能」という記述は
> **厳密に誤り**である——`add: (a: Int, b: Int) -> Int = a + b` において `a`/`b` はランタイム値引数。
> **型位置で参照される**具体型引数のみがコンパイル時値引数。正しい定義は下を参照。

**中核設計**：ジェネリクスシグネチャ内の `Type` は型引数をマークする；具体型（`Int`/`Bool`/`Float`
など）の注釈付き引数列は**コンパイル時値引数の候補**であり、コンパイル時値引数になるかどうかは値が**型位置で参照される**（値依存）かどうかに依存する。`const`
キーワードは不要。

**判定ルール（二ステップ）**：

1. **形態粗筛**：引数注釈が `Type` 以外の具体型（例：`Int`）→ 候補に列挙。
2. **用途精筛**：候補名が**型位置**（型体フィールド型、内層 `Fn` 引数型、`Assert`
   述語、`Array(T, N)`
   などの型構築実引数位）に登場 → コンパイル時値引数として確認；そうでなければ**ランタイム値引数**と見なす。

| 書き方                                                     | 判定                 | 理由                                           |
| ---------------------------------------------------------- | -------------------- | ---------------------------------------------- |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b ランタイム値引数 | 値位置にのみ登場、型構築に参加しない           |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N コンパイル時値引数 | N が `Array(T, N)` の型構築実引数位に登場      |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N コンパイル時値引数 | N が内層引数 `k` の型                          |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N 落空（以下参照）   | N が型体内で参照されず、ランタイム値引数に退化 |

> **値依存の本質**：コンパイル時値引数すなわち値依存型——値が**型を構築する**ために使用される場合にのみ、コンパイル時決定が必要。形態（`: Int`）は候補資格のみを決定し、用途（型位置での露出）はコンパイル時値引数であるかどうかを決定する。これは §「コンパイル時決定性の保証」における「型位置での関数呼び出しはコンパイル時評価」と同一の判定基準である。

```yaoxiang
# ════════════════════════════════════════════════════════
# コンパイル時値引数：N が型位置（Measure 長さスロット）で参照される
# ════════════════════════════════════════════════════════
Measure: (T: Type, N: Int) -> Type = {
    data: Array(T, N),  # N が型構築実引数位に登場 → コンパイル時値引数
    length: N,
}

# 使用方式：factorial(5) が型位置で評価（コンパイル時）、結果 120 が型に埋め込まれる
m: Measure(Int, factorial(5))  # Measure(Int, 120)

# ════════════════════════════════════════════════════════
# 値依存：N が内層引数 k の型として
# ════════════════════════════════════════════════════════
# N はコンパイル時値引数（(k: N) の型位に登場）；
# k はランタイム値引数、その型はリテラル型 N（単値型）。
factorial: (N: Int) -> (k: N) -> Int = {
    return match k {
        0 => 1,
        _ => k * factorial(k - 1)
    }
}
```

> **落空候補の処理**：具体型を注釈したが型位置で参照されない候補（上表の `Foo` の `N`
> など）はランタイム値引数（関数級パス）に退化する。型コンストラクタパスの落空候補はランタイムスロットを占められない（型コンストラクタはコンパイル時評価）、宣言側で直接 [E1094] エラー：「N はコンパイル時値引数として宣言されたが、型体内で参照されていない」——これまでは静かに破棄され、インスタンス化 arity の不一致を引き起こしていた。

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

### Never と Void：型システムの ⊥ と ⊤

YaoXiang の型システムは Curry-Howard 同型において同時に ⊥（偽/空型）と ⊤（真/Unit）を備え、`Never`
と `Void` の二つの組み込み型名で担う：

**Never（⊥）** — 交渉不可能な三つの内核的性質：

1. **ゼロコンストラクタ**：リテラルも式も `Never`
   型の値を生成できない。これはメタレベル性質であり、組み込みでなければならない。
2. **爆発原理**：`Never <: T` は任意の型 `T` に対して成立する。`Never`
   値は任意の型として使用可能——これが `assert(false)`
   後のコードが型検査を通過する理由である（もちろん実行されることはないが）。
3. **発散マーカー**：`f: (...) -> Never` は `f`
   が返らないことを保証する。コンパイラはこれに基づき dead code 分析を行う。

`Never`
は組み込み型名でありキーワードではなく、parser は感知しない。空和型リテラル構文は開放しない。

**Void（⊤、すなわち Unit）**
— 唯一の居留者（デフォルト void 値）を持ち、真の命題「恒真」の担い手である。`Void`
は零フィールド積型の単位元、`Never` は零変体和型の単位元——両者は対偶をなす。`x: Void = <デフォルト>`
は合法、`x: Never = ...` は書ける右辺がない。

#### 4.3 コンパイル時検証（標準ライブラリ実装）

```yaoxiang
# ════════════════════════════════════════════════════════
# 標準ライブラリ実装：条件型の利用
# ════════════════════════════════════════════════════════

# 標準ライブラリ定義
# IsTrue：値宇宙から型宇宙への橋——Bool 真値を型にマッピング
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      # ⊤、値を持つ、プログラム続行
    false => Never,    # ⊥、値なし、発散
}

# Assert：コンパイル時精化型プリミティブ——Bool 命題の型レベル表現
Assert: (cond: Bool) -> Type = IsTrue(cond)
#
# cond が true  → Assert(true)  = Void    （恒真、消去）
# cond が false → Assert(false) = Never   （恒偽、コンパイルエラー/発散）
# cond が判定不能 → 証明パイプラインが dispatch モードで決定：
#                    CompileTime → Unknown、prove を要求
#                    Runtime     → check を挿入、Γ 仮定を注入

# 使用方式1：型定義内での制約として
Bounded: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    # コンパイル時検査：N は 0 より大きくなければならない（Assert は型位置）
    length: Assert(N > 0),
}

# 使用方式2：式内で使用
IntArray: (N: Int) -> Type = Array(Int, N)
# 検証：IntArray(10) のサイズは sizeof(Int) * 10 と等しい
Assert(size_of(IntArray(10)) == sizeof(Int) * 10)
```

#### 4.4 コンパイル時ジェネリクス特殊化

```yaoxiang
# 小配列最適化：関数オーバーロードを使用してコンパイル時ジェネリクス特殊化を実現

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
    # コンパイラ最適化：ループ展開
    return arr.data[0] + arr.data[1] + arr.data[2] + arr.data[3]
}
```

### 5. 条件型

> **Curry-Howard 同型**：条件型は Curry-Howard 視点から論理の **case 分析** である。`Bool`
> 型は二つの可能な値（True/False）を持つ命題に対応し、`If`
> はその命題の真偽に応じて異なる結果を選択する——これは論理における case 析取そのものである。`match C { True => T, False => E }`
> は実質的に「命題 C が True のとき結論は T、C が False のとき結論は E」を表現している。

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

# コンパイル時検証（§4.3 の Assert 定義に統一）
# Assert: (cond: Bool) -> Type = IsTrue(cond)

# 使用
# 型計算：If(True, Int, String) => Int
# 型計算：If(False, Int, String) => String
```

#### 5.2 型族

> **Curry-Howard 同型**：型族は「命題即型」の最も直接的な体現である。`Add: (A: Type, B: Type) -> Type`
> は「型レベルで加算関数を書いた」のではなく、**自然数加算に関する命題を構成している**。`(Zero, B) => B`
> は「命題 Add(Zero, B) は B と等価である」と言い、`(Succ(A'), B) => Succ(Add(A', B))` は「Add(A',
> B) が成立するなら、Add(Succ(A'),
> B) も成立する」と言う。これは Peano 公理における加算定義そのものである。型検査器がこの match 式の通過を検証することは、この定義の論理的一貫性を検証することと等価である。

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

# 型レベル加算（Curry-Howard：case analysis + 再帰呼び出し、完全帰納のためには停止性検査が必要）
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
sum: (arr: Vec(Int)) -> Int = {
    # より効率的なコードにコンパイル
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Vec(Float)) -> Float = {
    # SIMD 命令を使用
    return simd_sum_float(arr.data, arr.length)
}

# 汎用実装
sum: (T: Type) -> ((arr: Vec(T)) -> T) = {
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
sum: (arr: Vec(Int)) -> Int = {
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Vec(Float)) -> Float = {
    return simd_sum_float(arr.data, arr.length)
}

# ジェネリクス実装（コンパイラが自動的に最適なものを選択）
sum: (T: Type) -> ((arr: Vec(T)) -> T) = {
    result = Zero::zero()
    for item in arr {
        result = result + item
    }
    return result
}

# 使用時は完全に透過的
int_arr = Vec(Int)(1, 2, 3)
float_arr = Vec(Float)(1.0, 2.0, 3.0)

# コンパイラが自動的に最適な特殊化を選択
sum(int_arr)     # sum: (Vec(Int)) -> Int を選択
sum(float_arr)    # sum: (Vec(Float)) -> Float を選択
```

#### 6.3 関数オーバーロードとインラインの完璧な結合

**鍵となる特性**：関数オーバーロードとインライン最適化が自然に結合し、ゼロコスト抽象を実現する。

```yaoxiang
# ======== ソースコード ========
sum: (arr: Vec(Int)) -> Int = {
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Vec(Float)) -> Float = {
    return simd_sum_float(arr.data, arr.length)
}

sum: (T: Type) -> ((arr: Vec(T)) -> T) = {
    result = Zero::zero()
    for item in arr {
        result = result + item
    }
    return result
}

# 使用
int_arr = Vec(Int)(1, 2, 3, 4, 5)
result = sum(int_arr)

# ======== コンパイル後（等価コード）=======
# コンパイラが自動的に最適な特殊化を選択し、インライン化する
result = native_sum_int(int_arr.data, int_arr.length)

# 手書き最適化コードと完全等価、関数呼び出しオーバーヘッドなし！
```

**中核的優位性**：

1. **コンパイラのスマートな選択**

   ```yaoxiang
   sum(int_arr)      # 自動的に sum: (Vec(Int)) -> Int を選択
   sum(float_arr)    # 自動的に sum: (Vec(Float)) -> Float を選択
   sum(custom_arr)  # 自動的に sum: (T: Type) -> ((arr: Vec(T)) -> T) を選択
   ```

2. **インライン最適化**
   - 小さな関数は呼び出し点に自動インライン化
   - 関数呼び出しオーバーヘッドなし
   - 手書き最適化コードと完全等価

3. **型安全性**
   - コンパイル時型検査
   - ランタイムオーバーヘッドなし
   - 仮想関数テーブル不要

4. **RFC-010 と完璧に契合**

   ```yaoxiang
   # 統一構文を完全使用
   name: type = value
   # impl、where などの新しいキーワード不要
   ```

**実際の応用例**：

```yaoxiang
# 性能敏感的数値計算
fibonacci: (n: Int) -> Int = {
    if n <= 1 { return n }
    return fibonacci(n - 1) + fibonacci(n - 2)
}

fibonacci: (n: Float) -> Float = {
    # Binet の公式を使用
    phi = (1.0 + 5.0.sqrt()) / 2.0
    return (phi.pow(n) - (-phi).pow(-n)) / 5.0.sqrt()
}

# コンパイラが自動的に選択しインライン化
fibonacci(10)      # Int バージョン選択、完全にインライン化
fibonacci(10.5)    # Float バージョン選択、Binet の公式を使用
```

**これは何を意味するか？**

- ✅ **ジェネリクス特殊化** → 関数オーバーロードで自然に解決
- ✅ **性能最適化** → インライン化が自動完了
- ✅ **コード再利用** → 一つの関数名で複数の実装
- ✅ **ゼロコスト抽象** → コンパイル時多態、ランタイムオーバーヘッドなし
- ✅ **新しいキーワード不要** → RFC-010 統一構文に完璧準拠

````

### 7. デッドコード除去機構

#### 7.1 インスタンス化グラフ分析

```rust
// コンパイラ内部：ジェネリクスインスタンス化依存グラフを構築
struct InstantiationGraph {
    // ノード：ジェネリクスインスタンス化
    nodes: HashMap<InstanceKey, InstanceNode>,

    // エッジ：使用関係
    edges: HashMap<InstanceKey, Vec<InstanceKey>>,
}

struct InstanceKey {
    generic: FunctionId,  // ジェネリクス関数 ID
    type_args: Vec<TypeId>,  // 型引数
    const_args: Vec<ConstId>,  // Const 引数
}

// アルゴリズム：到達可能性分析
fn eliminate_dead_instantiations(graph: &InstantiationGraph) {
    let mut reachable = HashSet::new();

    // エントリポイントから開始（main、エクスポート関数など）
    let entry_points = find_entry_points();
    for entry in entry_points {
        dfs_visit(entry, &graph, &mut reachable);
    }

    // 未訪問のインスタンス化がデッドコード
    for node in &graph.nodes {
        if !reachable.contains(node.key) {
            eliminate(node);
        }
    }
}
````

#### 7.2 使用点分析

```yaoxiang
# ソースコード分析
map: (T: Type, R: Type)(list: List(T), f: Fn(T) -> R) -> List(R) = ...

# 使用点1：map(Int, Int) をインスタンス化
int_list = List(Int)()
int_list.push(1)
int_list.push(2)
int_list.push(3)
doubled = map(int_list, (x) => x * 2)  # map[Int, Int] を必要とする

# 使用点2：map(String, String) をインスタンス化
string_list = List(String)()
string_list.push("a")
string_list.push("b")
string_list.push("c")
uppercased = map(string_list, (s) => s.to_uppercase())  # map[String, String] を必要とする

# 未使用：map[Float, Float] など
# これらのジェネリクスインスタンスは生成されない

# コンパイル後は使用されたインスタンスのみを含む
map_Int_Int: (list: List(Int), f: Fn(Int) -> Int) -> List(Int) = ...
map_String_String: (list: List(String), f: Fn(String) -> String) -> List(String) = ...
```

#### 7.3 コンパイル時ジェネリクス DCE

```yaoxiang
# コンパイル時分析：コンパイル時ジェネリクスの使用状況
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
}

# 実際の使用状況
arr_10_int = Array(Int, 10)(data=[1, 2, 3, 4, 5, 6, 7, 8, 9, 10])  # 二層：型引数 + 構築引数
# 訂正：初期版は Array(Int, 10)(1, 2, 3, ...)（要素を直接展開）と書かれていたが、
# §9.3 の権威パターン（Type(引数)(フィールド構築引数)/空構築）と一致せず、
# フィールド名式構築引数に統一、SPEC type-system.md §4.3 を参照。
arr_100_int = Array(Int, 100)()   # 空構築、データは事後代入

# コンパイル後は使用されたサイズのみを生成
Array_Int_10: (Array(Int, 10)) = ...
Array_Int_100: (Array(Int, 100)) = ...

# 未使用のサイズは生成されない
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
int_list = List(Int)()
int_list.push(1)
int_list.push(2)
int_list.push(3)
doubled = map(int_list, (x) => x * 2)  # map(Int, Int) をインスタンス化

# モジュール C
# C.yx
use A.{map}
string_list = List(String)()
string_list.push("a")
string_list.push("b")
string_list.push("c")
uppercased = map(string_list, (s) => s.to_uppercase())  # map(String, String) をインスタンス化

# コンパイル分析：
# - モジュール B は map[Int, Int] を使用
# - モジュール C は map[String, String] を使用
# - コンパイル後のバイナリはこれら二つのインスタンスのみを含む
```

#### 7.5 LLVM レベル DCE

```rust
// コンパイルパイプライン
fn optimize_ir(ir: &mut IR) {
    // 1. 単相化（YaoXiang コンパイラ）
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

    // 6. 最適化パスを実行
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

# ✅ ジェネリクス方案：自動派生
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

# ✅ ジェネリクス方案：型安全ビルダー
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

# ✅ ジェネリクス方案：条件型
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

#### 9.1 完全なジェネリクスコンテナ例

```yaoxiang
# ======== 1. ジェネリクスコンテナの定義 ========
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
    data: Vec(T),
    length: Int,

    # ジェネリクスメソッド（T は外層 List(T) から自動的にスコープに導入される）
    push: (self: List(T), item: T) -> Void,
    pop: (self: List(T)) -> Option(T),
    map: (R: Type) -> ((self: List(T), f: (T) -> R) -> List(R)),
    filter: (self: List(T), predicate: (T) -> Bool) -> List(T),
    fold: (U: Type) -> ((self: List(T), initial: U, f: (U, T) -> U) -> U),
}

# ======== 2. ジェネリクスメソッドの実装 ========
# 関数定義は List 名前空間の下（List. プレフィックス = 名前空間帰属）
# list.push(item) のような . 呼び出し構文を有効にするには、明示的バインディングが必要：List.push = push[0]
# self は単なる慣例引数名であり、コンパイラは名前ではなく型を見る

List.push: (T: Type) -> ((self: List(T), item: T) -> Void) = {
    if self.length >= self.data.length {
        # 拡張
        new_data = Vec(T)(len=self.data.length * 2)
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
# List の Clone 実装
List.clone: (T: Clone) -> ((self: List(T)) -> List(T)) = {
    result = List(T)()
    for i in 0..self.length {
        result.push(self.data[i].clone())
    }
    return result
}

# ======== 4. 使用例 ========
# ジェネリクス List を作成
numbers = List(Int)()
numbers.push(1)
numbers.push(2)
numbers.push(3)

# ジェネリクスメソッドを使用
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
# ======== 1. ジェネリクスソートアルゴリズム ========
Comparator: (T: Type) -> Type = {
    compare: (T, T) -> Int,  # a < b なら -1、a == b なら 0、a > b なら 1
}

# ジェネリクス quicksort
quicksort: (T: Clone) -> ((array: Vec(T), cmp: Comparator(T)) -> Vec(T)) = {
    if array.length <= 1 {
        return array.clone()
    }

    pivot = array[array.length / 2]
    left = Vec(T)()
    right = Vec(T)()

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
numbers = Vec(Int)(3, 1, 4, 1, 5, 9, 2, 6)
sorted = quicksort(numbers, Comparator(Int)())

# String 配列をソート（StringComparator が必要）
strings = Vec(String)("hello", "world", "foo", "bar")
sorted_strings = quicksort(strings, Comparator(String)())
```

#### 9.3 コンパイル時ジェネリクス例

```yaoxiang
# ======== 1. コンパイル時行列型 ========
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),

    # コンパイル時次元検証：Assert 標準ライブラリ型を利用
    _assert: Assert(Rows > 0),  # Rows > 0、そうでなければコンパイルエラー
    _assert: Assert(Cols > 0),  # Cols > 0、そうでなければコンパイルエラー

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

### 利点

1. **ゼロコスト抽象**
   - コンパイル時単相化、ランタイムオーバーヘッドなし
   - 仮想関数不要、RTTI 不要

2. **デッドコード除去**
   - コンパイル時分析、使用されるジェネリクスのみインスタンス化
   - コード膨張が制御可能

3. **マクロ代替**
   - 型安全なコード生成
   - IDE 親和性、明確なエラー情報

4. **コンパイル時計算**
   - コンパイル時ジェネリクスがコンパイル時計算をサポート
   - 次元検証などの特性
   - `const` キーワード不要、純粋な型制約

### 欠点

1. **コンパイル時間**
   - ジェネリクスインスタンス化がコンパイル時間を増加させる
   - 制約解決が遅くなる可能性

2. **メモリ使用量**
   - コンパイラのメモリ使用量が増加
   - キャッシュ機構がメモリを必要とする

3. **実装複雑度**
   - 制約ソルバが複雑
   - 型レベル計算エンジンが複雑

4. **エラー診断**
   - ジェネリクスエラーが複雑になる可能性
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
   - ジェネリクス引数推論のヒント

4. **並列コンパイル**
   - ジェネリクスの並列インスタンス化
   - マルチスレッド制約解決

## 代替方案

| 方案                   | 選択しない理由                 |
| ---------------------- | ------------------------------ |
| 基礎ジェネリクスのみ   | 複雑なマクロを代替できない     |
| 純粋マクロシステム     | 型安全性なし、エラー情報が悪い |
| 依存制約のみ           | 柔軟性不足                     |
| ランタイムジェネリクス | パフォーマンスオーバーヘッド   |

### リスク

| リスク         | 影響                 | 緩和措置                          |
| -------------- | -------------------- | --------------------------------- |
| 制約解決複雑度 | コンパイル時間過長   | インクリメンタル解決 + キャッシュ |
| コード膨張     | バイナリファイル過大 | DCE + 閾値制御                    |
| 実装複雑度     | 開発期間延長         | 段階的実装                        |
| エラー診断     | ユーザー体験低下     | 詳細なエラー情報                  |

## 未解決問題

### 決議待ち問題

| 議題               | 説明                           | 状態     |
| ------------------ | ------------------------------ | -------- |
| インスタンス化戦略 | Eager vs Lazy vs Threshold     | 議論待ち |
| キャッシュサイズ   | LRU キャッシュ容量設定         | 議論待ち |
| エラー診断         | ジェネリクスエラー情報の詳細度 | 議論待ち |

### 今後の最適化

| 最適化項目                   | 価値 | 実装難易度 |
| ---------------------------- | ---- | ---------- |
| インスタンス化グラフ分析     | 高   | 中         |
| 型レベルプログラミング DSL   | 中   | 高         |
| ジェネリクス性能ベンチマーク | 中   | 低         |

## 付録

### 構文 BNF

```bnf
# ジェネリクス引数は統一 () 構文を使用し、関数型の一部
# 例：map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R))

# 型制約（ジェネリクス引数内）
type_bound ::= identifier
             | identifier '+' identifier ('+' identifier)*

# 引数宣言（型 + 名前）
parameter ::= identifier ':' type

parameters ::= parameter (',' parameter)*

# 関数宣言：name: type = expression
# ジェネリクス引数は関数型の最初の引数グループ：(T: Type) -> ((params) -> return)
function ::= identifier ':' type '=' (expression | block)

# メソッド宣言：Type.method: type = expression
method ::= identifier '.' identifier ':' type '=' (expression | block)

# 型定義（統一 Binding 構文）
# ジェネリクス型例：List: (T: Type) -> Type = { ... }
generic_type ::= identifier ':' type '=' type_expression

# ジェネリクス引数内の Type はコンパイラが実引数型から自動充填
# 例：map(numbers, f)、T は numbers: List(Int) から抽出、R は f: (Int) -> String から抽出
```

## ライフサイクルと帰属

```
┌─────────────┐
│   草案      │  ← 現在の状態
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  審査中     │  ← オープンコミュニティでの議論とフィードバック
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  受理済み   │    │  拒否済み   │
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
- [RFC-001: 並作モデル](./accepted/001-concurrent-model-error-handling.md)
- [RFC-008: ランタイムモデル](./accepted/008-runtime-concurrency-model.md)
- [tutorial/ チュートリアル](../../../../../tutorial/)

### 外部参考

- [Rust ジェネリクスシステム](https://doc.rust-lang.org/book/ch10-01-syntax.html)
- [C++ テンプレート特殊化](https://en.cppreference.com/w/cpp/language/template_specialization)
- [Haskell 型クラス](https://www.haskell.org/tutorial/classes.html)
- [Swift ジェネリクス](https://docs.swift.org/swift-book/LanguageGuide/Generics.html)
- [単相化最適化](https://llvm.org/docs/Monomorphization.html)
- [デッドコード除去](https://en.wikipedia.org/wiki/Dead_code_elimination)

---
title: 'RFC-011: generics システム設計 - ゼロコスト抽象とマクロの代替'
status: '承認済み'
author: '晨煦'
updated: '2026-07-15（型体コードブロック + compile-time 規約 + エフェクトシード実装済み）'
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

# RFC-011: generics システム設計 - ゼロコスト抽象とマクロの代替

## 概要

本ドキュメントでは、YaoXiang 言語の**generics システム設計**を定義する。強力な generics 機能によりゼロコスト抽象を実現し、compile-time 最適化を活用してマクロへの依存を削減し、デッドコード除去機構を提供する。

**コア設計**：

- **統一シグネチャ構文**：`(T: Type, R: Type) -> ...` generics パラメータと通常パラメータの統一
- **`Type` 自己記述機構**：`Type` は言語レベルの特殊存在であり、シグネチャ内の `Type`
  位置は自動的に推論・充填される
- **type constraint**：`T: Dup + Add` multiple constraint、関数型制約
- **associated
  type**：`Iterator: (Item: Type) -> Type = { next: () -> Option(Item), has_next: () -> Bool }`
- **compile-time generics**：`N: Int` generics 値パラメータ、compile-time 定数インスタンス化
- **conditional type**：`If: (C: Bool, T: Type, E: Type) -> Type` 型レベル計算、type family

**価値**：

- ゼロコスト抽象：compile-time 単態化、runtime オーバーヘッドなし
- デッドコード除去：インスタンス化グラフ解析 + LLVM 最適化
- マクロ代替：generics により 90% のマクロ使用シーンを代替
- 型安全性：compile-time チェック、IDE フレンドリー
- **明示は暗黙に優れる**：`Type` 自己記述、コンパイラが自動推論

## 参考ドキュメント

本ドキュメントの設計は以下のドキュメントに基づいている：

| ドキュメント                                                                       | 関係                         | 説明                                                          |
| ---------------------------------------------------------------------------------- | ---------------------------- | ------------------------------------------------------------- |
| [RFC-010: 統一型構文](./010-unified-type-syntax.md)                                | **構文の基礎**               | generics 構文と統一 `name: type = value` モデルの統合         |
| [RFC-010: 統一型構文](./010-unified-type-syntax.md)                                | **呼び出し構文**             | 第 6 節：generics 呼び出し構文——統一 `()` 適用、`[]` 完全削除 |
| [RFC-009: ownership モデル](./accepted/009-ownership-model.md)                     | **type system**              | Move セマンティクスと generics の自然な結合                   |
| [RFC-024: spawn ベースの並行ランタイムセマンティクス](./024-concurrency-model.md)  | **実行モデル**               | DAG 解析と generics 型検査                                    |
| [RFC-008: ランタイムモデル](./accepted/008-runtime-concurrency-model.md)           | **コンパイラアーキテクチャ** | generics 単態化と compile-time 最適化戦略                     |
| [型ユニバース思想](../reference/plan/ongoing/类型宇宙思想.md)                      | **理論的コア**               | 型ユニバース階層モデルと値依存型設計                          |
| [RFC-027: compile-time 述語と統一静的検証](./027-compile-time-evaluation-types.md) | **停止検査**                 | 自動メトリック合成と compile-time 評価の安全保証              |

## 型ユニバース思想と値依存型

YaoXiang の generics システムは**型ユニバース思想**の上に構築されている。このメンタルモデルでは、言語のすべての概念を階層構造として統一し、核心となる革新は**値依存型**を Type2 層のファーストクラス市民として昇格させることである。

### 値依存型とは？

**値依存型**は、一つまたは複数の**値**（他の型だけでなく）に依存する型である。これらの値は compile-time に評価され、コンパイル段階で型安全保証を提供できる。

```yaoxiang
# 传统泛型：类型参数
List: (T: Type) -> Type

# 值依赖类型：值参数
Array: (T: Type, N: Int) -> Type  # 数组类型依赖于长度值 N
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type  # 矩阵类型依赖于行数和列数
```

### コンテナ型命名の階層化

言語層には 3 つのコンテナ概念があり、長さ情報の帰属が根本的な違いである：

| 型            | 長さ       | セマンティクス                           | 基盤                                           |
| ------------- | ---------- | ---------------------------------------- | ---------------------------------------------- |
| `Array(T, N)` | 型         | **固定長**配列、N は型に含まれる         | コア primitive type（スタック/インライン優先） |
| `Vec(T)`      | runtime 値 | **runtime の長さの生バッファ**、拡張可能 | コア primitive type（ヒープ上連続バッファ）    |
| `List(T)`     | runtime 値 | 標準ライブラリ型                         | ライブラリ：`{ data: Vec(T), length: Int }`    |

3 者の分業原則：

- **`Array(T, N)`
  は長さを型に含める唯一の形**である——長さは compile-time 定数であるため、境界失敗の compile-time 拒否が可能（`a[5]`
  は `a: Array(Int, 3)` のとき直接 compile-time エラー、後述の「compile-time 次元検証」を参照）。
- **`Vec(T)`
  は runtime の長さの最小基盤**——「割り当て可能、長さ取得可能、読み書き可能、拡張可能」の 4 つのみを提供し、容量戦略、増加係数、縮小可否は一切行わない。他のコンテナを構築する原料である。
- **`List(T)` はライブラリ型であり、primitive ではない**——YaoXiang 自身で `std.list`
  内に定義され（`{ data: Vec(T), length: Int }`）、ユーザー定義の generics
  record と同等の扱いを受ける。拡張可能セマンティクスのすべての戦略（いつ拡張するか、いくつ拡張するか、共有可否）はライブラリ内にあり、コンパイラは関与しない。

`Vec(T)` の構築形式（2 層：まず型パラメータ、次に構築パラメータ）：

```yaoxiang
# 空构造——长度 0，元素事后追加
v = Vec(Int)()

# 元素构造——长度由元素个数确定
w = Vec(Int)(1, 2, 3)          # 长度 3

# 槽位分配——分配 n 个零值槽位
buf = Vec(Int)(len=64)         # 长度 64，元素全为零值
```

> スロット割り当ては**フィールド名形式**（`len=`）を使用し、位置形式は使用しない：位置形式の単一整数は「単一要素ベクトル」と曖昧になる（`Vec(Int)(64)`
> は「長さ 64」と「要素 64 を 1 つ含む」を区別できない）。これは generics 構築の統一ルールと一致する：フィールド名形式の実引数は名前でバインドされ、位置推論の影響を受けない。
>
> これは `List` 拡張に必要な唯一の primitive である——`List`
> は必要に応じて新しいスロットを割り当てて要素を移動する：
>
> ```yaoxiang
> new_data = Vec(T)(len=self.data.length * 2)
> ```
>
> いつ拡張するか、いくつ拡張するか、縮小可否はすべて `List` が決定する。`Vec` は容量戦略を行わない。

下から上に向かって、性能は低下し、柔軟性は増加する：`Array` > `Vec` > `List`。

> 命名の根拠：`Vec`/`vector`
> は主流言語（Rust/C++）ではいずれも runtime の長さの拡張可能シーケンスを指し、`Array`
> は固定長を指す。

### 値依存型の核心的な優位性

従来の generics と比較して、YaoXiang の値依存型には以下の核心的な優位性がある：

| 特性              | 従来の generics (C++/Rust)                  | YaoXiang 値依存型                                 |
| ----------------- | ------------------------------------------- | ------------------------------------------------- |
| 型が依存する値    | 型パラメータのみに依存                      | 関数呼び出し結果を含む任意の値に依存可能          |
| compile-time 評価 | C++ テンプレート手動特殊化、Rust なし       | 自動 compile-time 評価、停止保証                  |
| 型レベル計算      | テンプレートメタプログラミング（複雑/危険） | 統一された型レベル計算エンジン                    |
| 型安全性          | C++ なし、Rust 制限あり                     | 完全な型安全性、compile-time チェック             |
| 次元検証          | runtime チェックまたは手動特殊化            | compile-time 次元検証、runtime オーバーヘッドなし |

### 型ユニバース階層と値依存型

型ユニバース思想は、言語概念を意味的役割によって異なる階層に分割し、値依存型は
**Type2 層**に位置する：

| 階層      | 役割                               | 例                                                                                                              |
| --------- | ---------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| Type-1    | 値                                 | `42`, `factorial(5)`, 関数自体                                                                                  |
| Type0     | meta type キーワード               | `Type`                                                                                                          |
| Type1     | 具象型                             | `Int`, `String`, `Array(Int, 3)`                                                                                |
| **Type2** | **関数/型コンストラクタ/値依存型** | `add: (Int, Int) -> Int`, `Array: (T: Type, N: Int) -> Type`, `Matrix: (T: Type, Rows: Int, Cols: Int) -> Type` |

**重要な設計**：Type2 層の関数、型コンストラクタ、値依存型は**構文が統一**されており、いずれも
`(params) -> result` の形式である：

- 通常関数：`(Int, Int) -> Int` → 戻り値は値
- 型コンストラクタ：`(T: Type) -> Type` → 戻り値は型
- 値依存型：`(T: Type, N: Int) -> Type` → 戻り値は型であり、値パラメータ N に依存する

> **Curry-Howard 対応**：この統一は偶然ではない。Curry-Howard 対応は「型は命題、プログラムは証明」であると指摘している——関数型
> `A → B` は論理的含意「A ならば B」に対応し、generics `(T: Type) -> Type`
> は全称量化「すべての型 T について」に対応し、値依存型 `(n: Int) -> Type`
> は「各整数 n について型が存在する」に対応する。YaoXiang は関数、型コンストラクタ、値依存型を Type2 層に統一しており、本質的には「証明」と「計算」を**構成的証明**という同一概念に統一している。これは Curry-Howard 対応の言語設計における直接的な現れである：一つの形式（`(params) -> result`）が論理的命題と計算プロセスの両方を担う。

### compile-time 決定性の保証

YaoXiang の型ユニバース思想は、**Type 階層のすべてが compile-time に決定される**ことを要求する。

```yaoxiang
# 编译期维度验证示例
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),
    # 编译期检查：维度必须为正
    _assert: Assert(Rows > 0),
    _assert: Assert(Cols > 0),
}

# 创建 3x3 单位矩阵 - 编译期完成
identity: (T: Add + Zero + One, N: Int) -> ((size: N) -> Matrix(T, N, N)) = {
    matrix = Matrix(T, N, N)()
    # ...
}

# 编译期计算：factorial(3) = 6，数组大小在编译期确定
arr: Array(Int, factorial(3)) = Array(Int, 6)()
```

コンパイラは自動的に：

1. 型位置での関数呼び出しを検出する
2. 関数に対して compile-time 停止検査を実行する（後述の停止検査機構を参照）
3. compile-time に評価を実行する
4. 結果を生成された型に埋め込む

### 値依存型の応用シナリオ

#### compile-time 次元検証

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

#### 境界失敗の compile-time カバレッジ目標

> **実装状況の説明**：コンテナ型は既に特殊化解除されている——`Array(T, N)` は const
> generics コンストラクタであり、literal コンテキスト落点、`in`
> メンバーシップ述語が既に実装されている。 `Array(T, N)`
> literal 落点の N と要素型は compile-time 検査により強制されている（E1002）、**N は既に信頼できる**——本節の目標機構は「注釈 N
> == runtime 長さ」の上に構築できる。現在の `[]`
> インデックス境界外（E6003）と Dict キー欠落（E6008）は**runtime エラーの過渡状態**である；本節の値依存型はこれらの境界失敗を**compile-time**
> に抑えることを目標とする機構である：
>
> - const インデックス：`a[5]`（5 は compile-time 定数）は `a: Array(Int, 3)`
>   のとき直接 compile-time 拒否；
> - 値インデックス：`a[i]` は前提条件 `i < len(a)` を要求し、値依存型コントラクトにより証明；
> - `in` 述語はホーア論理の前提条件の基底である：`n in 1..10`、`x in some_set`
>   はいずれも compile-time に証明可能な命題。
>
> 精化型の完全な設計は実装時に別途本節を補完する。

#### conditional type

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

#### generics 関数

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
numbers = List(Int)()   # 值构造两层形式（见 §9.1）；元素用 push 填充
numbers.push(1)
numbers.push(2)
numbers.push(3)
doubled = map(numbers, (x) => x * 2)  # 推导为 map[Int, Int]
```

### 他言語との比較

| 特性                                  | C++ テンプレート           | Rust generics    | Haskell GADT | **YaoXiang**                      |
| ------------------------------------- | -------------------------- | ---------------- | ------------ | --------------------------------- |
| 型パラメータ                          | ✅                         | ✅               | ✅           | ✅                                |
| 値依存型                              | ❌                         | ❌               | ✅           | ✅                                |
| compile-time 評価                     | テンプレートインスタンス化 | ❌               | ✅           | ✅                                |
| 停止保証                              | ❌                         | ❌               | ❌（危険）   | ✅（自動メトリック合成、RFC-027） |
| 型安全性                              | ❌（マクロ展開）           | ✅               | ✅           | ✅                                |
| 統一構文                              | ❌                         | ❌               | ❌           | ✅                                |
| compile-time 次元検証                 | 手動特殊化                 | runtime チェック | type family  | compile-time 自動検証             |
| 半自動停止注釈（decreases/invariant） | ❌                         | ❌               | ❌           | ❌（compile-time 全自動のみ）     |

### 停止検査機構（RFC-027 と統合）

値依存型の compile-time 評価は**停止を保証**しなければならず、そうでなければ型システムが無限ループに陥る。停止検査は RFC-027 の compile-time 証明パイプラインにより
**全自動**で実行される——コンパイラは自動的にメトリックを合成し、証明可能な再帰/ループは通過し、証明不可能なものは直接コンパイルエラーを報告する。**半自動注釈の余地を残さない**：RFC-022 の
`//! decreases`、`/*! invariant !*/` は RFC-022 の廃止に伴い廃止され、規約は型注釈自体である。

#### 再帰関数の停止検査

コンパイラは compile-time 評価前に、すべての再帰パスで再帰呼び出しの引数が厳密に減少するかをチェックする（RFC-027
§6.7）。規約コメントは一切不要：

```yaoxiang
# 编译期阶乘：无 //! requires/ensures/decreases，编译器自动分析
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)  # 编译器分析：n-1 < n → 递减 → 终止
}

# 使用：在类型位置调用，编译器先验证终止再求值
arr: Array(Int, factorial(5)) = Array(Int, 120)()  # 编译期求值 factorial(5) = 120
```

| シナリオ                                        | 動作              |
| ----------------------------------------------- | ----------------- |
| コンパイラが再帰的減少（例：`n-1`）を分析できる | compile-time 評価 |
| 減少しない/減少を判定できない                   | コンパイルエラー  |
| runtime 呼び出し（型位置でない）                | 停止検査不要      |

#### ループの停止検査

ループには `: Invariant(...)` や `: decreases(...)`
の注釈は不要である。変数の精化型注釈（例：`UpTo(n)`）がループ不変条件とメトリック境界の両方を提供し、コンパイラは優先順位に従って 4 つのメトリック合成戦略を試し、見つかれば停止する（RFC-027
§7）：

1. **線形ランク関数の自動合成**——型注釈から変数の境界を抽出し、線形組み合わせを列挙し、SMT で m ≥
   0 かつすべてのパスで m' < m を検証
2. **述語違反カウント**（実験的）——目標型定義（例：`Sorted`）から violation_count を抽出し、隣接交換/移動をカバー
3. **有界増減パターン**——`v += const` → メトリック `upper - v`（戦略 1 の縮退、最速パス）
4. **乗法スケールメトリックテンプレート**——`v *= const`（const > 1）→ メトリック
   `ceil(log_const(upper / v))`

```yaoxiang
sum: (arr: Array(Int, n)) -> Int = {
    mut i: UpTo(arr.len) = 0   # 类型标注给出上界 arr.len 与下界 0
    while i < arr.len {
        # 编译器自动推导：度量 arr.len - i，每次迭代严格递减 1 → 终止得证
        s += arr[i]; i += 1
    }
    return s
}
```

#### 停止検査のワークフロー

```
┌─────────────────────────────────────────────────────────────┐
│  类型检查阶段                                                │
│  遇到类型位置上的函数调用（如 factorial(5)）                │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  1. 终止检查（RFC-027 证明管道，全自动）                     │
│     - 递归函数：检查参数在每条递归路径上严格递减             │
│     - 循环：四种度量合成策略（线性秩/违反计数/有界模式/      │
│       乘法缩放），SMT 验证递减                                │
│     - 无法证明 → 编译错误（硬边界，无半自动标注兜底）        │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  2. 编译期求值（由内置解释器执行）                           │
│     - 纯函数：直接求值                                       │
│     - 副作用：编译错误（类型位置必须无副作用）                │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  3. 结果嵌入类型                                            │
│     - Array(Int, factorial(5)) → Array(Int, 120)             │
│     - Matrix(Float, 3, 3) → 具体类型                        │
└─────────────────────────────────────────────────────────────┘
```

#### 優位性

- **安全性**：compile-time 評価が必ず停止することを保証し、型システムが無限ループに陥ることを防ぐ
- **統一性**：停止検査と正当性検証（VC 生成）は同じ compile-time 証明パイプライン（RFC-027）を共有し、独立した規約構文は存在しない
- **全自動**：コンパイラは型注釈から自動的にメトリックを合成し、証明可能なら通過、証明不可能ならエラー——プログラマの手書き
  `decreases` に依存しない

## 動機

### なぜ強力な generics システムが必要なのか？

現在の主流言語の generics には限界がある：

| 言語         | generics 能力    | 問題                                                       |
| ------------ | ---------------- | ---------------------------------------------------------- |
| Java         | 境界型           | compile-time 単態化、generics 特殊化なし                   |
| C#           | generics 制約    | runtime 型チェック、パフォーマンスオーバーヘッドあり       |
| Rust         | generics + Trait | Trait システムが複雑、学習曲線が急峻                       |
| C++          | テンプレート     | テンプレート特殊化が複雑、コンパイルエラーメッセージが悪い |
| **YaoXiang** | **値依存型**     | **型が値に依存可能、compile-time 次元検証、停止保証**      |

### 核心的な矛盾

1. **性能 vs 柔軟性**：runtime 柔軟性 vs compile-time 最適化
2. **複雑 vs 簡潔**：強力な type system vs 使いやすさ
3. **マクロ vs generics**：マクロコード生成 vs generics 型安全性
4. **値依存 vs 型安全性**：従来の generics では compile-time に次元を検証できない

### 値依存型の核心的な優位性

YaoXiang の**値依存型**は従来の generics に対する核心的な優位性である：

| 優位性                | 説明                                                                            |
| --------------------- | ------------------------------------------------------------------------------- |
| **型が値に依存**      | `Array: (T: Type, N: Int) -> Type` により型が具体的な値に依存する               |
| **compile-time 評価** | 型位置の関数呼び出しは compile-time に評価され、結果は直接型に埋め込まれる      |
| **次元検証**          | `Matrix(Float, 3, 3)` は compile-time に行列次元を検証                          |
| **型レベル計算**      | `If`, `Match` などの conditional type が型レベル計算をサポート                  |
| **停止保証**          | compile-time 停止検査（自動メトリック合成）により compile-time 評価の停止を保証 |

```yaoxiang
# C++/Rust 无法做到的编译期验证
matrix: Matrix(Float, factorial(3), factorial(2)) = ...
# 编译期计算：factorial(3) = 6, factorial(2) = 2
# 类型为 Matrix(Float, 6, 2)

# 维度不匹配在编译期捕获
identity: Matrix(Float, 3, 3) = ...
# multiply(matrix_2x3, identity_3x3)  # 编译错误：2 != 3
```

### generics システムの価値

```yaoxiang
# 示例：统一API设计
# 不同容器类型的map操作

# 传统方案：每个类型单独实现
map_int_array: (array: Vec(Int), f: Fn(Int) -> Int) -> Vec(Int) = ...
map_string_array: (array: Vec(String), f: Fn(String) -> String) -> Vec(String) = ...
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

### 核心的な目標

1. **ゼロコスト抽象** - generics 呼び出しは具象型呼び出しと等価
2. **デッドコード除去** - compile-time 解析、使用される generics のみをインスタンス化
3. **マクロ代替** - generics により 90% のマクロ使用シーンを代替
4. **型安全性** - compile-time チェック、runtime 型オーバーヘッドなし
5. **IDE フレンドリー** - スマートヒント、明確なエラーメッセージ
6. **値依存型** - 型が値に依存可能、compile-time 次元検証をサポート
7. **compile-time 評価の安全性** -
   compile-time 停止検査（RFC-027 自動メトリック合成）により compile-time 評価の停止を保証

### 設計原則

- **compile-time 決定**：generics パラメータは compile-time に決定
- **単態化優先**：具象コードを生成し、仮想関数呼び出しを回避
- **制約駆動**：type constraint がインスタンス化をガイド
- **プラットフォーム最適化**：特殊化によりプラットフォーム固有の最適化をサポート
- **型ユニバース統一**：関数/型コンストラクタ/値依存型を Type2 層に統一
- **停止保証**：型位置の関数呼び出しは停止を証明する必要がある

## 提案

### 1. 基礎 generics

#### 1.1 generics 型パラメータ

> **重要なルール**：generics 型定義は**明示的に `: Type`
> を注釈しなければならない**。さもないと HM により関数として推論される。
>
> | 書き方                            | 意味                               |
> | --------------------------------- | ---------------------------------- |
> | `List: (T: Type) -> Type = {...}` | ✅ 型コンストラクタ                |
> | `List = {...}`                    | ❌ HM が関数として推論、型ではない |

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
    data: Vec(T),
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

### generics 関数呼び出し構文

#### 1.1 統一シグネチャ構文

```yaoxiang
# 泛型函数使用统一的 (T: Type, R: Type) 签名语法
map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R)) = ...

# 多类型参数
combine: (T: Type, U: Type) -> ((a: T, b: U) -> (T, U)) = (a, b)
```

#### 1.2 `Type` 自己記述機構

`Type` は言語レベルの特殊存在であり、コンパイラは本質的にシグネチャ内の `Type`
位置を認識し、実引数の型から自動的に推論・充填する。

```yaoxiang
# 编译器自动推断泛型参数
numbers: List(Int) = List(Int)()
#         ^^^^^^^^   ^^^^^^^^
#         类型声明   构造调用：Int 填充 T，() 值构造

# 函数调用推断
numbers: List(Int) = List(Int)()
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
int_list: List(Int) = List(Int)()
doubled: List(Int) = map(int_list, (x: Int) => x * 2)  # 实例化 map[Int, Int]

string_list: List(String) = List(String)()
uppercased: List(String) = map(string_list, (s: String) => s.to_uppercase())  # 实例化 map[String, String]

# 编译后（等价代码）
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

#### 1.4 明示的な充填（推論が失敗したとき）

````yaoxiang
# 可推断时省略 Type 参数
numbers: List(Int) = List(Int)()
strings: List(String) = map(numbers, (x: Int) => x.to_string())

# 无法推断时必须显式填充
# map(numbers, (x) => x)  # ❌ Error: Cannot infer R

### 2. 型制約システム

#### 2.1 单一约束

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
````

#### 2.2 multiple constraint

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
map: (T: Type, R: FnMut(T))(array: Vec(T), f: R) -> Vec(R) = {
    result: Vec(R) = Vec()
    for item in array {
        result.push(f(item))
    }
    return result
}

# 使用
doubled: Vec(Int) = map(Vec(1, 2, 3), (x: Int) => x * 2)  # 编译器推断
```

> **制約名の由来（2026-09-22 付記）**：`Add` / `Subtract` / `Multiply` / `Divide` / `Modulo`
> などの演算子制約は
> [RFC-011b: 演算子オーバーロードとインタフェース駆動演算子](./011b-operator-overloading.md)
> で定義・実装されている—— `T: Add` ≜ `Add(T, T, T)`
> インタフェースインスタンス化が登録済み（3 つの型パラメータ、結果型 `O` を明示）。 `Zero` / `One` /
> `PartialOrd` / `Fn` / `FnMut`
> は現時点で**定義の出典がなく**、宙に浮いた制約名であり、今後の RFC で個別に実装される予定；それまでは、これらの名前を含む例は紙面の示意である。

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

#### 2.4 組み込みマーカー trait：Dup と Clone

**3 種類のコピーセマンティクス**：

| 型                     | 意味                                                          | トリガー方法        | 適用シナリオ                         |
| ---------------------- | ------------------------------------------------------------- | ------------------- | ------------------------------------ |
| **primitive 値コピー** | 代入時に自動で値をコピーし、2 つの値は完全に独立              | 代入/引数渡しで自動 | Int, Float, Bool, Char               |
| **Dup**                | シャローコピー：ハンドル/トークンをコピーし、基礎データを共有 | 代入/引数渡しで自動 | `&T` トークン、`ref T`、String/Bytes |
| **Clone**              | ディープコピー：完全で独立したコピー作成                      | `value.clone()`     | Clone を実装する任意の型             |

**Dup のセマンティクス**：Dup を実装した型は、代入/引数渡し時に ownership を移動しない——コンパイラがハンドル/トークンをコピーし、複数の所有者が同じ基礎データを指す。これは RFC-009
ownership モデルにおける Move デフォルトセマンティクスの補完である。

**Dup と Clone は直交する概念である**：

```
Dup = 复制句柄，共享数据（修改互相影响）
Clone = 复制数据，副本独立（修改互不影响）
```

```
Dup = ハンドルをコピー、データを共有（変更が相互に影響）
Clone = データをコピー、コピーは独立（変更が相互に影響しない）
```

**ルール**：

```
1. 原语值类型（Int, Float, Bool, Char） — 编译器内置值复制，不属于 Dup
2. Dup  — 只适用于引用/令牌类型和内部引用计数的类型
3. Clone — 显式深拷贝，任何类型可实现
4. 默认 Move — 其他类型保持默认 Move 语义
```

```
1. primitive 値型（Int, Float, Bool, Char） — コンパイラ組み込みの値コピー、Dup には属さない
2. Dup  — 参照/トークン型および内部参照カウント型にのみ適用
3. Clone — 明示的なディープコピー、任意の型が実装可能
4. デフォルト Move — 他の型はデフォルトの Move セマンティクスを維持
```

**どの型が Dup か**：

| 型                       | Dup  | 理由                                                              |
| ------------------------ | ---- | ----------------------------------------------------------------- |
| `&T`（借用トークン）     | ✅   | ゼロサイズトークン、トークンコピー = 複数の視点が同じデータを指す |
| `ref T`                  | ✅   | Rc/Arc コピー = 参照カウント+1、ヒープデータを共有                |
| String, Bytes            | ✅   | 内部参照カウント、ハンドルコピーで基礎バッファを共有              |
| `&mut T`（可変トークン） | ❌   | 線形排他、コピー不可                                              |
| struct                   | 派生 | すべてのフィールドが Dup → struct Dup                             |
| enum                     | 派生 | すべての variant のすべてのフィールドが Dup → enum Dup            |
| tuple                    | 派生 | すべての要素が Dup → tuple Dup                                    |
| Fn（クロージャ）         | ❌   | キャプチャ環境が Dup でない可能性                                 |
| `*T`（生ポインタ）       | ❌   | unsafe、ownership システムに参加しない                            |

**Int/Float/Bool/Char は Dup ではない**——これらは値型であり、代入時にコンパイラが自動的に値をコピーする（2 つの値は完全に独立）。これは「シャローコピー」ではなく、コンパイラによる primitive の組み込み処理であり、Dup 型属性を通じて表現する必要はないし、すべきではない。

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

> **注意**：`Send`/`Sync` はユーザーに見える trait ではない。タスク間安全性保証は `ref`
> キーワードとコンパイラの全自動処理により行われる——`ref`
> は自動的に Rc または Arc を選択し、ユーザーは Send/Sync を理解する必要がない。

### 3. associated type

#### 3.1 associated type 定義

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

# Vec的Iterator实现
# 使用方法语法糖：Vec.Item, Vec.next, Vec.has_next
# 迭代位置由包装记录携带（Vec 本身是原始缓冲，无 index 字段）
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

#### 3.2 generics associated type（GAT）

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

### 4. compile-time generics

#### 4.1 compile-time 値パラメータ

**核心的な設計**：generics シグネチャの `Type` は型パラメータを示す；具象型（`Int`/`Bool`/`Float`
など）で注釈されたパラメータは**compile-time 値パラメータの候補**であり、compile-time 値パラメータになるかどうかは、その値が**型位置で参照されるか**（値依存）による。`const`
キーワードは不要。

> 判断の根拠は**型位置で参照されるか**であり、「具象型で注釈されているか」ではない：`add: (a: Int, b: Int) -> Int = a + b`
> において `a`/`b` は runtime 値パラメータである。両方ともどの型位置にも現れないため。

**判定ルール（2 ステップ）**：

1. **形態の粗選別**：パラメータが `Type` 以外の具象型（例：`Int`）で注釈されている → 候補に列挙。
2. **用途の精選別**：候補名が**型位置**（型体のフィールド型、内側 `Fn` パラメータ型、 `Assert`
   述語、`Array(T, N)` などの型構築実引数位置）に現れる →
   compile-time 値パラメータとして確認；そうでなければ **runtime 値パラメータ**と見なす。

| 書き方                                                     | 判定                           | 理由                                                 |
| ---------------------------------------------------------- | ------------------------------ | ---------------------------------------------------- |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b は runtime 値パラメータ    | 値位置にのみ現れ、型構築に参加しない                 |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N は compile-time 値パラメータ | N は `Array(T, N)` の型構築実引数位置に現れる        |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N は compile-time 値パラメータ | N は内側パラメータ `k` の型として機能                |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N は落空（以下参照）           | N は型体で参照されず、runtime 値パラメータに退化する |

> **値依存の本質**：compile-time 値パラメータはすなわち値依存型である——値が**型を構築する**ために使われる場合にのみ、compile-time 決定が必要となる。形態（`: Int`）は候補資格を決定するのみで、用途（型位置での露出）が compile-time 値パラメータであるかを決定する。これは §「compile-time 決定性の保証」における「型位置での関数呼び出しは compile-time に評価される」と同じ根拠である。

```yaoxiang
# ════════════════════════════════════════════════════════
# 编译期值参数：N 在类型位置（Measure 长度槽）被引用
# ════════════════════════════════════════════════════════
Measure: (T: Type, N: Int) -> Type = {
    data: Array(T, N),  # N 出现在类型构造实参位 → 编译期值参数
    length: N,
}

# 使用方式：factorial(5) 在类型位置求值（编译期），结果 120 嵌入类型
m: Measure(Int, factorial(5))  # Measure(Int, 120)

# ════════════════════════════════════════════════════════
# 值依赖：N 作为内层参数 k 的类型
# ════════════════════════════════════════════════════════
# N 是编译期值参数（出现在 (k: N) 的类型位）；
# k 是运行时值参数，其类型为字面量类型 N（单值类型）。
factorial: (N: Int) -> (k: N) -> Int = {
    return match k {
        0 => 1,
        _ => k * factorial(k - 1)
    }
}
```

> **落空候補の処理**：具象型で注釈されているが型位置で参照されない候補（上記表の `Foo` の `N`
> など）は runtime 値パラメータ（関数レベルの経路）に退化する。型コンストラクタ経路の落空候補は runtime スロットを占有できず（型コンストラクタは compile-time に評価される）、宣言側で直接エラー [E1094] を報告する：「N は compile-time 値パラメータとして宣言されているが、型体で参照されていない」——以前は静かに破棄されていたため、インスタンス化の arity が不整合になっていた。

#### 4.2 compile-time 計算

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

### Never と Void：type system の ⊥ と ⊤

YaoXiang の type
system は Curry-Howard 対応において ⊥（偽/空型）と ⊤（真/Unit）の両方を備え、`Never` と `Void`
の 2 つの組み込み型名でそれらを担う：

**Never（⊥）** — 交渉不可の 3 つの核となる性質：

1. **ゼロコンストラクタ**：`Never`
   型の値を生成する literal や式は存在しない。これはメタレベルの性質であり、組み込みでなければならない。
2. **爆発原理**：`Never <: T` は任意の型 `T` に対して成立する。`Never`
   値は任意の型として使用できる——これが `assert(false)`
   の後のコードが型チェックを通過する理由である（決して実行されないが）。
3. **発散マーカー**：`f: (...) -> Never` は `f`
   が return しないことを保証する。コンパイラはこれに基づいて dead code 解析を行う。

`Never`
は組み込み型名であり、キーワードではないため parser は感知しない。空和型 literal 構文は公開しない。

**Void（⊤、すなわち Unit）**
— ちょうど 1 つの居住者（デフォルトの void 値）を持ち、真命題「恒真式」の担い手である。`Void`
はゼロフィールドの積型の単位元であり、`Never`
はゼロ variant の和型の単位元である——両者は双対。`x: Void = <デフォルト>`
は合法であり、`x: Never = ...` は右辺が書けない。

#### 4.3 compile-time 検証（標準ライブラリ実装）

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
Bounded: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    # 编译期检查：N 必须大于 0（Assert 在类型位置）
    length: Assert(N > 0),
}

# 使用方式2：在表达式中使用
IntArray: (N: Int) -> Type = Array(Int, N)
# 验证：IntArray(10) 的大小等于 sizeof(Int) * 10
Assert(size_of(IntArray(10)) == sizeof(Int) * 10)
```

#### 4.4 compile-time generics 特殊化

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

### 5. conditional type

> **Curry-Howard 対応**：conditional type は Curry-Howard の観点から見ると論理における **case 分析**
> である。`Bool` 型は 2 つの可能な値を持つ命題（True/False）に対応し、`If`
> はその命題の真偽に応じて異なる結果を選択する——これはまさに論理における case 選言である。`match C { True => T, False => E }`
> は実際にはこう表現している：「命題 C が True であるとき結論は T、C が False であるとき結論は E」。

#### 5.1 If conditional type

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

#### 5.2 type family

> **Curry-Howard 対応**：type
> family は「命題即型」の最も直接的な具現化である。`Add: (A: Type, B: Type) -> Type`
> は「型レベルで加算関数を記述した」のではなく、**自然数の加算に関する命題を構成している**。`(Zero, B) => B`
> は「命題 Add(Zero, B) は B と等価である」といい、`(Succ(A'), B) => Succ(Add(A', B))` は「Add(A',
> B) が成り立つならば、Add(Succ(A'),
> B) も成り立つ」という。これが Peano の公理における加算の定義そのものである。型チェッカーがこの match 式が通過することを検証することは、この定義の論理的一貫性を検証することと等価である。

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
sum: (arr: Vec(Int)) -> Int = {
    # 编译为更高效的代码
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Vec(Float)) -> Float = {
    # 使用SIMD指令
    return simd_sum_float(arr.data, arr.length)
}

# 通用实现
sum: (T: Type) -> ((arr: Vec(T)) -> T) = {
    result = Zero::zero()
    for item in arr {
        result = result + item
    }
    return result
}
```

#### 6.2 条件付き特殊化

```yaoxiang
# 完全符合RFC-010语法的特化方式：函数重载

# 具体类型特化
sum: (arr: Vec(Int)) -> Int = {
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Vec(Float)) -> Float = {
    return simd_sum_float(arr.data, arr.length)
}

# 泛型实现（编译器自动选择最优）
sum: (T: Type) -> ((arr: Vec(T)) -> T) = {
    result = Zero::zero()
    for item in arr {
        result = result + item
    }
    return result
}

# 使用时完全透明
int_arr = Vec(Int)(1, 2, 3)
float_arr = Vec(Float)(1.0, 2.0, 3.0)

# 编译器自动选择最优特化
sum(int_arr)     # 选择 sum: (Vec(Int)) -> Int
sum(float_arr)    # 选择 sum: (Vec(Float)) -> Float
```

#### 6.3 関数オーバーロードとインラインの完璧な組み合わせ

**重要な特性**：関数オーバーロードはインライン最適化と本質的に組み合わせ可能で、ゼロコスト抽象を実現する。

```yaoxiang
# ======== 源代码 ========
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

# ======== 编译后（等价代码）=======
# 编译器自动选择最优特化，然后内联
result = native_sum_int(int_arr.data, int_arr.length)

# 完全等价于手写优化代码，无函数调用开销！
```

**核心的な優位性**：

1. **コンパイラのスマートな選択**

   ```yaoxiang
   sum(int_arr)      # 自动选择 sum: (Vec(Int)) -> Int
   sum(float_arr)    # 自动选择 sum: (Vec(Float)) -> Float
   sum(custom_arr)  # 自动选择 sum: (T: Type) -> ((arr: Vec(T)) -> T)
   ```

2. **インライン最適化**
   - 小さい関数は自動的に呼び出し点にインライン化される
   - 関数呼び出しのオーバーヘッドがゼロ
   - 手書きの最適化コードと完全に等価

3. **型安全性**
   - compile-time 型チェック
   - runtime オーバーヘッドゼロ
   - 仮想関数テーブル不要

4. **RFC-010 との完璧な適合**

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

- ✅ **generics 特殊化** → 関数オーバーロードで自然に解決
- ✅ **性能最適化** → インラインが自動的に完了
- ✅ **コード再利用** → 1 つの関数名、複数の実装
- ✅ **ゼロコスト抽象** → compile-time 多態、runtime オーバーヘッドゼロ
- ✅ **新しいキーワード不要** → RFC-010 統一構文に完璧に適合

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

#### 7.2 使用点解析

```yaoxiang
# 源代码分析
map: (T: Type, R: Type)(list: List(T), f: Fn(T) -> R) -> List(R) = ...

# 使用点1：实例化 map(Int, Int)
int_list = List(Int)()
int_list.push(1)
int_list.push(2)
int_list.push(3)
doubled = map(int_list, (x) => x * 2)  # 需要 map[Int, Int]

# 使用点2：实例化 map(String, String)
string_list = List(String)()
string_list.push("a")
string_list.push("b")
string_list.push("c")
uppercased = map(string_list, (s) => s.to_uppercase())  # 需要 map[String, String]

# 未使用：map[Float, Float] 等
# 这些泛型实例不会被生成

# 编译后只包含被使用的实例
map_Int_Int: (list: List(Int), f: Fn(Int) -> Int) -> List(Int) = ...
map_String_String: (list: List(String), f: Fn(String) -> String) -> List(String) = ...
```

#### 7.3 compile-time generics DCE

```yaoxiang
# 编译期分析：编译期泛型使用情况
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
}

# 实际使用情况
arr_10_int = Array(Int, 10)(data=[1, 2, 3, 4, 5, 6, 7, 8, 9, 10])  # 两层：类型参数 + 构造参数
arr_100_int = Array(Int, 100)()   # 空构造，数据事后赋值

# 编译后只生成被使用的Size
Array_Int_10: (Array(Int, 10)) = ...
Array_Int_100: (Array(Int, 100)) = ...

# 未使用的Size不会生成
# Array(Int, 50) 不会生成
```

#### 7.4 モジュール間 DCE

```yaoxiang
# 模块A
# A.yx
pub map: (T: Type, R: Type)(list: List(T), f: Fn(T) -> R) -> List(R) = ...

# 模块B
# B.yx
use A.{map}
int_list = List(Int)()
int_list.push(1)
int_list.push(2)
int_list.push(3)
doubled = map(int_list, (x) => x * 2)  # 实例化 map(Int, Int)

# 模块C
# C.yx
use A.{map}
string_list = List(String)()
string_list.push("a")
string_list.push("b")
string_list.push("c")
uppercased = map(string_list, (s) => s.to_uppercase())  # 实例化 map(String, String)

# 编译分析：
# - 模块B使用 map[Int, Int]
# - 模块C使用 map[String, String]
# - 编译后二进制只包含这两个实例
```

#### 7.5 LLVM レベル DCE

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

#### 8.1 コード生成の代替

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

#### 8.2 DSL の代替

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

#### 8.3 型レベルプログラミングの代替

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

> **RFC-011b との関係（2026-09-22 付記）**：本節の持ち上げ type family `Add(A, B)` は
> [RFC-011b](./011b-operator-overloading.md)
> 演算子インタフェース登録表の型レベルの視点である——中核登録 `Add(Int, Float, Float)` と本表の
> `(Int, Float) => Float`
> は同一のルールであり、ユーザの各インタフェースインスタンス化はこの表への 1 行追加である。§5.2 の Peano 型レベル
> `Add`
> は純粋に型レベルの計算（同名異物）であり、値レベルの演算子インタフェースと互いに干渉しない——演算子のクエリは実装登録表にアクセスし、名前解決を経由しない。

### 9. 例

#### 9.1 完全な generics コンテナの例

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
    data: Vec(T),
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

#### 9.2 generics アルゴリズムの例

```yaoxiang
# ======== 1. 泛型排序算法 ========
Comparator: (T: Type) -> Type = {
    compare: (T, T) -> Int,  # -1 if a < b, 0 if a == b, 1 if a > b
}

# 泛型quicksort
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
numbers = Vec(Int)(3, 1, 4, 1, 5, 9, 2, 6)
sorted = quicksort(numbers, Comparator(Int)())

# 排序String数组（需要StringComparator）
strings = Vec(String)("hello", "world", "foo", "bar")
sorted_strings = quicksort(strings, Comparator(String)())
```

#### 9.3 compile-time generics の例

```yaoxiang
# ======== 1. 编译期矩阵类型 ========
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),

    # 编译期维度验证：利用 Assert 标准库类型
    _assert: Assert(Rows > 0),  # Rows > 0，否则编译错误
    _assert: Assert(Cols > 0),  # Cols > 0，否则编译错误

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

### 利点

1. **ゼロコスト抽象**
   - compile-time 単態化、runtime オーバーヘッドなし
   - 仮想関数不要、RTTI 不要

2. **デッドコード除去**
   - compile-time 解析、使用される generics のみをインスタンス化
   - コード膨張を管理可能

3. **マクロ代替**
   - 型安全なコード生成
   - IDE フレンドリー、明確なエラーメッセージ

4. **compile-time 計算**
   - compile-time generics が compile-time 計算をサポート
   - 次元検証などの機能
   - `const` キーワード不要、純粋な型制約

### 欠点

1. **コンパイル時間**
   - generics インスタンス化がコンパイル時間を増加させる
   - 制約解決が遅くなる可能性がある

2. **メモリ使用量**
   - コンパイラのメモリ使用量が増加
   - キャッシュ機構にメモリが必要

3. **実装の複雑さ**
   - 制約ソルバが複雑
   - 型レベル計算エンジンが複雑

4. **エラー診断**
   - generics エラーが複雑になる可能性がある
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
   - generics パラメータ推論のヒント

4. **並列コンパイル**
   - generics の並列インスタンス化
   - マルチスレッド制約解決

## 代替案

| 案                 | 選択しない理由                       |
| ------------------ | ------------------------------------ |
| 基礎 generics のみ | 複雑なマクロを代替できない           |
| 純粋マクロシステム | 型安全性なし、エラーメッセージが悪い |
| 依存制約のみ       | 柔軟性不足                           |
| runtime generics   | パフォーマンスオーバーヘッドあり     |

### リスク

| リスク           | 影響                         | 緩和策                            |
| ---------------- | ---------------------------- | --------------------------------- |
| 制約解決の複雑さ | コンパイル時間の長期化       | インクリメンタル解決 + キャッシュ |
| コード膨張       | バイナリファイルが大きくなる | DCE + 閾値制御                    |
| 実装の複雑さ     | 開発期間の長期化             | 段階的実装                        |
| エラー診断       | ユーザー体験の低下           | 詳細なエラーメッセージ            |

## 未解決問題

### 決議待ちの問題

| 議題               | 説明                              | 状態     |
| ------------------ | --------------------------------- | -------- |
| インスタンス化戦略 | Eager vs Lazy vs Threshold        | 議論待ち |
| キャッシュサイズ   | LRU キャッシュ容量設定            | 議論待ち |
| エラー診断         | generics エラーメッセージの詳細度 | 議論待ち |

### 今後の最適化

| 最適化項目                 | 価値 | 実装難易度 |
| -------------------------- | ---- | ---------- |
| インスタンス化グラフ解析   | 高   | 中         |
| 型レベルプログラミング DSL | 中   | 高         |
| generics 性能ベンチマーク  | 中   | 低         |

## 付録

### 構文 BNF

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

## ライフサイクルと帰結

```
┌─────────────┐
│   草案      │  ← 当前状态
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  审核中     │  ← 开放社区讨论和反馈
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  已接受     │    │  已拒绝     │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   accepted/ │    │    rfc/     │
│ (正式设计)  │    │ (保留原位)  │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
```

```
┌─────────────┐
│   草案      │  ← 現在の状態
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  審査中     │  ← コミュニティの議論とフィードバックを公開
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
│ (正式設計)  │    │ (元の場所)  │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
```

---

## 参考文献

### YaoXiang 公式ドキュメント

- [RFC-010: 統一型構文](./010-unified-type-syntax.md)
- [RFC-009: ownership モデル](./accepted/009-ownership-model.md)
- [RFC-001: spawn モデル](./accepted/001-concurrent-model-error-handling.md)
- [RFC-008: ランタイムモデル](./accepted/008-runtime-concurrency-model.md)
- [tutorial/ チュートリアル](../../../../../tutorial/)

### 外部参考

- [Rust generics システム](https://doc.rust-lang.org/book/ch10-01-syntax.html)
- [C++ テンプレート特殊化](https://en.cppreference.com/w/cpp/language/template_specialization)
- [Haskell 型クラス](https://www.haskell.org/tutorial/classes.html)
- [Swift generics](https://docs.swift.org/swift-book/LanguageGuide/Generics.html)
- [単態化最適化](https://llvm.org/docs/Monomorphization.html)
- [デッドコード除去](https://en.wikipedia.org/wiki/Dead_code_elimination)

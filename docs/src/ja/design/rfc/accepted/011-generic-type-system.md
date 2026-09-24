---
title: 'RFC-011: 泛型システム設計 - ゼロコスト抽象化とマクロ代替'
status: '已接受'
author: '晨煦'
updated: '2026-07-15（タイプ体コードブロック + コンパイル時仕様 + エフェクトシード実装済み）'
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

# RFC-011: 泛型システム設計 - ゼロコスト抽象化とマクロ代替

## 摘要

本文档定义了YaoXiang语言の**泛型システム設計**を、強力な泛型能力を通じてゼロコスト抽象化を実現し、コンパイル時最適化によりマクロへの依存を軽減し、デッドコード除去メカニズムを提供します。

**核心設計**：

- **統一署名構文**：`(T: Type, R: Type) -> ...` 泛型パラメータと通常パラメータの統一
- **Type 自己記述メカニズム**：`Type` は言語レベルで特別な存在であり、署名中の `Type` 位置は自動的に推論・填充可能
- **型制約**：`T: Dup + Add` マルチ制約、関数型制約
- **関連型**：`Iterator: (Item: Type) -> Type = { next: () -> Option(Item), has_next: () -> Bool }`
- **コンパイル時泛型**：`N: Int` 泛型値パラメータ、コンパイル時定数インスタンス化
- **条件型**：`If: (C: Bool, T: Type, E: Type) -> Type` 型レベル計算、型族

**価値**：

- ゼロコスト抽象化：コンパイル時単態化、 런타임オーバーヘッドなし
- デッドコード除去：インスタンス化グラフ分析 + LLVM最適化
- マクロ代替：泛型でマクロ使用シナリオの90%を代替
- 型安全：コンパイル時チェック、IDEフレンドリー
- **明示的优于隐式**：`Type` 自己記述、コンパイラ自動推論

## 参考文档

本文档の設計は以下ドキュメントに基づいています：

| ドキュメント                                                                      | 関係           | 説明                                               |
| --------------------------------------------------------------------------------- | -------------- | -------------------------------------------------- |
| [RFC-010: 統一型構文](./010-unified-type-syntax.md)                               | **構文基盤**   | 泛型構文と統一 `name: type = value` モデル統合       |
| [RFC-010: 統一型構文](./010-unified-type-syntax.md)                               | **呼び出し構文** | 第6節：泛型呼び出し構文——統一 `()` 適用、`[]` 完全撤去 |
| [RFC-009: 所有権モデル](./009-ownership-model.md)                                 | **型システム** | Moveセマンティクスと泛型の自然な組み合わせ         |
| [RFC-024: spawnベース并发実行时意味論](./024-concurrency-model.md)                | **実行モデル** | DAG分析と泛型型チェック                            |
| [RFC-008: 実行時モデル](./008-runtime-concurrency-model.md)                       | **コンパイラアーキテクチャ** | 泛型単態化とコンパイル時最適化戦略         |
| タイプ宇宙思想（下文同名章节参照）                                                | **理論コア**   | タイプ宇宙階層モデルと値依存型設計                  |
| [RFC-027: コンパイル時述語と統一静的検証](./027-compile-time-evaluation-types.md) | **終了チェック** | 自動メジャース合成とコンパイル時評価安全保障     |

## タイプ宇宙思想と値依存型

YaoXiangの泛型システムは**タイプ宇宙思想**に基づいており、このメンタルモデルは言語内すべての概念を階層構造に統一し、コアイノベーションは**値依存型**をType2層の第一級市民として昇華することです。

### 値依存型とは何か？

**値依存型**とは、1つまたは複数の**値**（型だけでなく）に依存する型のことです。これらの値はコンパイル時に評価可能であり、コンパイル段階で型安全保証を提供します。

```yaoxiang
# 伝統的な泛型：型パラメータ
List: (T: Type) -> Type

# 値依存型：値パラメータ
Array: (T: Type, N: Int) -> Type  # 配列型は長さ値 N に依存
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type  # 行列型は行数と列数に依存
```

### コンテナ型命名階層

言語レベルには3つのコンテナ概念があり、长度情報の帰属が根本的な違いです：

| 型            | 長さ     | セマンティクス                     | 下層                                |
| ------------- | -------- | ---------------------------------- | ----------------------------------- |
| `Array(T, N)` | 型       | **固定長**配列、N は型に含まれる   | コアプリミティブ（スタック/インライン優先） |
| `Vec(T)`      | 运行時値 | **運行時長さの生バッファ**、成長可能 | コアプリミティブ（ヒープ上連続バッファ） |
| `List(T)`     | 运行時値 | 標準ライブラリ型                   | ライブラリ：`{ data: Vec(T), length: Int }` |

三者の分担原則：

- **`Array(T, N)` は唯一、長さを型に入れる形式**——長さはコンパイル時定数なので、境界失敗のコンパイル時拒否が可能（`a[5]` 当 `a: Array(Int, 3)` は直接コンパイル時エラー、以下「コンパイル時次元検証」参照）。
- **`Vec(T)` は运行時長さの最小基盤**——「割り当て可能、长度取得可能、読み書き可能、成長可能」の4つのみを提供、キャパシティ戦略、成長係数、縮小の有無は一切行わない。他のコンテナを構築する原材料です。
- **`List(T)` はライブラリ型であり、プリミティブではない**——YaoXiang自身で `std.list` に定義（`{ data: Vec(T), length: Int }`）、ユーザ定義泛型レコードと同待遇。成長可能セマンティクスの全戦略（いつ扩容、どれだけ扩容、共有可能か否か）はすべてライブラリにあり、コンパイラは関与しない。

`Vec(T)` の構築形式（2層：型パラメータ＋構築パラメータ）：

```yaoxiang
# 空構築——長さ0、要素は事後に追加
v = Vec(Int)()

# 要素構築——長さは要素数で確定
w = Vec(Int)(1, 2, 3)          # 長さ 3

# スロット確保——n個のゼロ値スロットを確保
buf = Vec(Int)(len=64)         # 長さ 64、要素はすべてゼロ値
```

> スロット確保は**フィールド名式**（`len=`）而非位置式：位置式の単一整数は「単一要素ベクトル」と曖昧になる（`Vec(Int)(64)` は「長さ64」と「要素64を1つ含む」の区別がつかない）。これは泛型構築の統一規則と一致：フィールド名式引数は名前バインディング、位置推論の影響を受けない。
>
> これは `List` 扩容に必要な唯一のプリミティブ——`List` は必要時に新しいスロットを確保し要素を移動する：
>
> ```yaoxiang
> new_data = Vec(T)(len=self.data.length * 2)
> ```
>
> いつ扩容、どれだけ扩容、縮小するかどうかは `List` が決める。`Vec` はキャパシティ戦略を行わない。

下から上看ると、パフォーマンスは逓減、柔軟性は逓増：`Array` > `Vec` > `List`。

> 命名根拠：`Vec`/`vector` は主流言語（Rust/C++）ではいずれも运行時長さの成長可能シーケンスを指す；`Array` は固定長を指定。

### 値依存型のコア優位性

伝統的な泛型相比、YaoXiangの値依存型は以下のコア優位性を持ちます：

| 特性         | 伝統的な泛型 (C++/Rust)     | YaoXiang 値依存型              |
| ------------ | --------------------------- | ------------------------------ |
| 型依存の値   | 型パラメータのみに依存      | 関数呼び出し結果を含む任意の値に依存可能 |
| コンパイル時評価 | C++テンプレート手動特殊化、Rustなし | 自動コンパイル時評価、終了保証       |
| 型レベル計算 | テンプレートメタプログamming（複雑/危険） | 統一型レベル計算エンジン       |
| 型安全       | C++なし、Rust制限付き        | 完全型安全、コンパイル時チェック     |
| 次元検証     | 运行時チェックまたは手動特殊化 | コンパイル時次元検証、运行時オーバーヘッドなし |

### タイプ宇宙階層と値依存型

タイプ宇宙思想は言語概念をセマンティックロールで異なる階層に分類し、値依存型は **Type2 層** に位置します：

| 階層      | 役割                           | 例                                                                                                            |
| --------- | ------------------------------ | --------------------------------------------------------------------------------------------------------------- |
| Type-1    | 値                             | `42`, `factorial(5)`, 関数自体                                                                                  |
| Type0     | メタ型キーワード               | `Type`                                                                                                          |
| Type1     | 具体型                         | `Int`, `String`, `Array(Int, 3)`                                                                                |
| **Type2** | **関数/型構築子/値依存型** | `add: (Int, Int) -> Int`, `Array: (T: Type, N: Int) -> Type`, `Matrix: (T: Type, Rows: Int, Cols: Int) -> Type` |

**鍵設計**：Type2層の関数、型構築子、値依存型は**統一的構文**で、すべて `(params) -> result` の形式：

- 通常関数：`(Int, Int) -> Int` → 戻り値は値
- 型構築子：`(T: Type) -> Type` → 戻り値は型
- 値依存型：`(T: Type, N: Int) -> Type` → 戻り値は型、かつ値パラメータ N に依存

> **Curry-Howard同型**：この統一は偶然ではない。Curry-Howard同型は「型は命題、プログラムは証明」を指摘——関数型 `A → B` は論理包含「AならばB」に対応、泛型 `(T: Type) -> Type` は全称量化「任意の型Tに対して」に対応、値依存型 `(n: Int) -> Type` は「各整数nに対してある型が存在する」に対応。YaoXiangが関数、型構築子、値依存型をType2層に統一するのは、本質的に「証明」と「計算」を同一概念——**構成的証明**——に統一することです。これはCurry-Howard同型の言語設計への直接的な具現化：1つの形式（`(params) -> result`)が論理命題と計算過程を同時に担う。

### コンパイル時確定性保証

YaoXiangのタイプ宇宙思想は要求：**Type 階層のすべてはコンパイル時に確定する**。

```yaoxiang
# コンパイル時次元検証例
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),
    # コンパイル時チェック：次元は正でなければならない
    _assert: Assert(Rows > 0),
    _assert: Assert(Cols > 0),
}

# 3x3単位行列の作成 - コンパイル時に完了
identity: (T: Add + Zero + One, N: Int) -> ((size: N) -> Matrix(T, N, N)) = {
    matrix = Matrix(T, N, N)()
    # ...
}

# コンパイル時計算：factorial(3) = 6、配列サイズはコンパイル時に確定
arr: Array(Int, factorial(3)) = Array(Int, 6)()
```

コンパイラは自動的に：

1. 型位置の関数呼び出しを検出
2. 関数にコンパイル時終了チェックを実行（以下の終了チェック機構参照）
3. コンパイル時に評価を実行
4. 結果を生成された型に埋め込み

### 値依存型の応用シナリオ

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

# N はコンパイル時定数なので型レベル計算に使用可能
first_three: Array(Int, 3) = Array(Int, 3)(1, 2, 3)
# first_three.length == 3（コンパイル時に既知）
```

#### 境界失敗のコンパイル時覆写ターゲット

> **実装状態説明**：コンテナ型は特殊化解除済み——`Array(T, N)` は const 泛型構築子、リテラルコンテキスト落点、`in` メンバーシップ述語はすでに実装済み。 `Array(T, N)` リテラル落点の N と要素型はコンパイル時検証で強制済み（E1002）、**N は信頼できる**——本節の目的は「N == 运行時長さ」という注釈の上にこの機構を構築すること。今の `[]` インデックス境界外れ（E6003）とDict欠落キー（E6008）は**運行時エラー移行態**；本節の値依存型はこれらの境界失敗を**コンパイル時**に押し込む目的機構：
>
> - const インデックス：`a[5]`（5 はコンパイル時定数）当 `a: Array(Int, 3)` は直接コンパイル時拒否；
> - 値インデックス：`a[i]` は前置条件 `i < len(a)` を要求、値依存型契約で証明；
> - `in` 述語はホア論理前置条件の基底：`n in 1..10`、`x in some_set` はどちらもコンパイル時証明可能命題。
>
> リファイン型の完全設計は実装時に別途本節を補足。

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

#### 泛型関数

```yaoxiang
# map: 泛型関数、型パラメータ T, R はコンパイル時に確定
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
numbers = List(Int)()   # 値構築2層形式（§9.1参照）；要素は push で填充
numbers.push(1)
numbers.push(2)
numbers.push(3)
doubled = map(numbers, (x) => x * 2)  # map[Int, Int] に推論
```

### 他言語との比較

| 特性                                  | C++テンプレート | Rust泛型   | Haskell GADT | **YaoXiang**                        |
| ------------------------------------- | --------------- | ---------- | ------------ | ----------------------------------- |
| 型パラメータ                          | ✅             | ✅         | ✅           | ✅                                  |
| 値依存型                              | ❌             | ❌         | ✅           | ✅                                  |
| コンパイル時評価                      | テンプレートインスタンス化 | ❌         | ✅           | ✅                                  |
| 終了保証                              | ❌             | ❌         | ❌（危険）   | ✅（自動メジャース探索 + 明示的測度、RFC-027） |
| 型安全                                | ❌（マクロ展開） | ✅         | ✅           | ✅                                  |
| 統一構文                              | ❌             | ❌         | ❌           | ✅                                  |
| コンパイル時次元検証                  | 手動特殊化     | 运行時チェック | 型族         | コンパイル時自動検証                  |
| 半自動終了注釈（decreases/invariant） | ❌             | ❌         | ❌           | ❌（注釈構文なし；明示的測度は型位置に記述） |

### 終了チェック機構（RFC-027との統一）

値依存型のコンパイル時評価は**終了を保証**しなければならず、そうでなければ型システムは無限ループに陥ります。終了チェックはRFC-027のコンパイル時証明パイプラインが**全自动優先**で完了——コンパイラはまず自動的に測度を探索し、証明できる再帰/ループは通過；探索できずかつ明示的測度がない場合はコンパイルエラー（RFC-027 §6.9は型位置の明示的測度フォールバックを提供）。**注釈構文の余地を残さない**：RFC-022の `//! decreases`、`/*! invariant !*/` はRFC-022と共に廃妾済み、仕様は型注釈自体。

> **トリガー判据（RFC-027 §7）**：終了義務は**リファイン型**によってトリガー——型がリファインされると検証モードに入る。リファインされていない通常の型は検証モードに入らず、終了義務を生成しない。

#### 再帰関数の終了チェック

コンパイラはリファイン署名を持つ再帰関数に対し、再帰呼び出しのパラメータが各再帰パスで厳密に減少するかをチェック（RFC-027 §6.7）。任何の仕様コメント不要：

```yaoxiang
# リファイン署名付き再帰：//! requires/ensures/decreases なし、コンパイラが自動的に減少を探索
factorial: (n: NonNegative(n)) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)  # コンパイラが探索：n-1 < n → 減少 → 終了
}
```

| シナリオ                                   | 動作                 |
| ------------------------------------------ | -------------------- |
| コンパイラが減少を探索できる（例：`n-1`） | 通過                 |
| 探索できないが型位置に測度がある（§6.9）  | SMTが判定、成立すれば通過 |
| 探索できないかつ明示的測度なし / SMTが偽と判定 | コンパイルエラー     |
| 署名にリファインなし（検証モード外）       | 終了義務を生成しない  |

#### ループの終了チェック

ループには `: Invariant(...)` や `: decreases(...)` 注釈は不要。変数上のリファイン型注釈（例：`UpTo(n)`）はループ不変式と測度境界の両方を提供し、コンパイラは優先順位順に4つの測度探索戦略を試み、いずれかを発見したら停止（RFC-027 §6.1–6.5）：

1. **線形階関数の自動合成**——型注釈から変数境界を抽出し、線形結合を列挙、SMTが m ≥ 0 かつ全パスで m' < m を検証
2. **述語違反カウント**（実験的）——目標型定義（例：`Sorted`）から violation_count を抽出し、隣接交換/移動をカウント
3. **有界増減パターン**——`v += const` → 測度 `upper - v`（戦略1の退化版、最速パス）
4. **乗法スケール測度テンプレート**——`v *= const`（const > 1）→ 測度 `ceil(log_const(upper / v))`

```yaoxiang
sum: (arr: Array(Int, n)) -> Int = {
    mut i: UpTo(arr.len) = 0   # 型注釈が上限 arr.len と下限 0 を提供 → 検証モードに入る
    while i < arr.len {
        # コンパイラが自動的に探索：測度 arr.len - i、各イテレーションで厳密に1減少 → 終了を証明
        s += arr[i]; i += 1
    }
    return s
}
```

探索できない場合、ループに名前を付けて型位置に測度を与えることができる（RFC-027 §6.9）：

```yaoxiang
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n { i = i + 1 }
    return acc
}
```

#### 終了チェックのワークフロー

```
┌─────────────────────────────────────────────────────────────┐
│  型チェック段階                                             │
│  リファイン型位置に出くわす（パラメータリファイン、        │
│  返り値リファイン、変数リファイン）                        │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  1. 終了チェック（RFC-027 証明パイプライン、全自動優先）    │
│     - 再帰関数：パラメータが各再帰パスで厳密に減少するかチェック │
│     - ループ：4つの測度探索戦略（線形階/違反カウント/      │
│       有界パターン/乗法スケール）、SMTが減少を検証          │
│     - 探索できない → プログラマが型位置に測度を与えられる   │
│     - 測度なし / SMTが偽と判定 → コンパイルエラー（硬境界） │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  2. コンパイル時評価（組込みインタープリタが実行）         │
│     - 純粋関数：直接評価                                     │
│     - 副作用：コンパイルエラー（型位置は副作用禁止）        │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  3. 結果の型への埋め込み                                     │
│     - Array(Int, factorial(5)) → Array(Int, 120)            │
│     - Matrix(Float, 3, 3) → 具体型                          │
└─────────────────────────────────────────────────────────────┘
```

#### 優位性

- **安全性**：コンパイル時評価が必ず終了することを保証、型システムが無限ループに陥ることを回避
- **統一性**：終了チェックと正当性検証（VC生成）は同一のコンパイル時証明パイプライン（RFC-027）を共有、独立した仕様構文なし
- **全自动優先**：コンパイラは型注釈から自動的に測度を探索、証明できれば通過；探索できなくても型位置に測度（`Terminates`）を与えられる、SMTが判定——プログラマの手による `decreases` 構文に依存しない

## 動機

### なぜ強力な泛型システムが必要か？

現在の主流言語の泛型には限界があります：

| 言語         | 泛型能力       | 問題                                       |
| ------------ | -------------- | ------------------------------------------ |
| Java         | 境界型         | コンパイル時単態化、泛型特殊化なし           |
| C#           | 泛型制約       | 运行時型チェック、パフォーマンスオーバーヘッド |
| Rust         | 泛型 + Trait   | Traitシステム複雑、学習曲線が険しい        |
| C++          | テンプレート   | テンプレート特殊化複雑、コンパイルエラー情報が悪い |
| **YaoXiang** | **値依存型** | **型は値に依存可能、コンパイル時次元検証、終了保証** |

### 核心的矛盾

1. **パフォーマンス vs 柔軟性**：运行時柔軟性 vs コンパイル時最適化
2. **複雑 vs 簡潔**：強力な型システム vs 使いやすさ
3. **マクロ vs 泛型**：マクロコード生成 vs 泛型型安全
4. **値依存 vs 型安全**：伝統的な泛型ではコンパイル時に次元を検証できない

### 値依存型の核心的优势

YaoXiangの**値依存型**は伝統的な泛型に対する核心的优势です：

| 優位性           | 説明                                                    |
| -------------- | ------------------------------------------------------- |
| **型依存の値** | `Array: (T: Type, N: Int) -> Type` で型を具体的な値に依存させる |
| **コンパイル時評価** | 型位置の関数呼び出しはコンパイル時に評価され、結果は直接型に埋め込み |
| **次元検証**   | `Matrix(Float, 3, 3)` はコンパイル時に行列次元を検証   |
| **型レベル計算** | `If`, `Match` 等の条件型で型レベル計算をサポート        |
| **終了保証**   | コンパイル時終了チェック（自動メジャース合成）でコンパイル時評価が必ず終了することを保証 |

```yaoxiang
# C++/Rust では做不到のコンパイル時検証
matrix: Matrix(Float, factorial(3), factorial(2)) = ...
# コンパイル時計算：factorial(3) = 6, factorial(2) = 2
# 型は Matrix(Float, 6, 2)

# 次元不一致はコンパイル時に捕捉
identity: Matrix(Float, 3, 3) = ...
# multiply(matrix_2x3, identity_3x3)  # コンパイルエラー：2 != 3
```

### 泛型システムの価値

```yaoxiang
# 例：統一API設計
# 異なるコンテナ型のmap操作

# 伝統的な方案：型ごとに個別実装
map_int_array: (array: Vec(Int), f: Fn(Int) -> Int) -> Vec(Int) = ...
map_string_array: (array: Vec(String), f: Fn(String) -> String) -> Vec(String) = ...
map_int_list: (list: List(Int), f: Fn(Int) -> Int) -> List(Int) = ...
map_string_list: (list: List(String), f: Fn(String) -> String) -> List(String) = ...

# 泛型方案：1つの泛型関数ですべての型をカバー
map: (T: Type, R: Type)(container: Container(T), f: Fn(T) -> R) -> Container(R) = {
    for item in container {
        result.push(f(item))
    }
    result
}
```

## 設計目標

### コア目標

1. **ゼロコスト抽象化** - 泛型呼び出しは具体型呼び出しと等価
2. **デッドコード除去** - コンパイル時分析、使用された泛型のみをインスタンス化
3. **マクロ代替** - 泛型でマクロ使用シナリオの90%を代替
4. **型安全** - コンパイル時チェック、运行時型オーバーヘッドなし
5. **IDEフレンドリー** - Intelligent補完、明確なエラー情報
6. **値依存型** - 型は値に依存可能、コンパイル時次元検証をサポート
7. **コンパイル時評価安全** - コンパイル時終了チェック（RFC-027 自動メジャース合成）でコンパイル時評価の終了を保証

### 設計原則

- **コンパイル時確定**：泛型パラメータはコンパイル時に確定
- **単態化優先**：具象コードを生成、仮想関数呼び出しを回避
- **制約駆動**：型制約がインスタンス化を指導
- **プラットフォーム最適化**：特殊化でプラットフォーム固有最適化をサポート
- **タイプ宇宙統一**：関数/型構築子/値依存型をType2層に統一
- **終了保証**：型位置の関数呼び出しは終了を証明しなければならない

## 提案

### 1. 基礎泛型

#### 1.1 泛型型パラメータ

> **重要ルール**：泛型型定義は**必ず `: Type` を明示的に注釈**でなければならず、さもなければHM推論で関数として解釈される。
>
> | 書き方                              | 意味                       |
> | ----------------------------------- | -------------------------- |
> | `List: (T: Type) -> Type = {...}` | ✅ 型構築子               |
> | `List = {...}`                    | ❌ HM推論で関数、型ではない |

```yaoxiang
# 泛型型定義（: Type 必须）
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
    push: (self: List(T), item: T) -> Void,   # self は約束名に過ぎず、キーワードではない
    get: (self: List(T), index: Int) -> Option(T),
}

# 泛型関数（: Type なし、HM推論で関数）
map: (T: Type, R: Type) -> ((opt: Option(T), f: Fn(T) -> R) -> Option(R)) = {
    return match opt {
        some => Option.some(f(some)),
        none => Option.none(),
    }
}

# 泛型制約（直接式、1行では return 省可能）
clone: (T: Clone)(value: T) -> T = value.clone()

# 複数型パラメータ
combine: (T: Type, U: Type) -> ((a: T, b: U) -> (T, U)) = (a, b)
```

### 泛型関数呼び出し構文

#### 1.1 統一署名構文

```yaoxiang
# 泛型関数は統一的な (T: Type, R: Type) 署名構文を使用
map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R)) = ...

# 複数型パラメータ
combine: (T: Type, U: Type) -> ((a: T, b: U) -> (T, U)) = (a, b)
```

#### 1.2 Type 自己記述メカニズム

`Type` は言語レベルで特別な存在であり、コンパイラは署名中の `Type` 位置を自然に認識し、実際の引数型から自動的に推論・填充を行う。

```yaoxiang
# コンパイラが自動的に泛型パラメータを推論
numbers: List(Int) = List(Int)()
#         ^^^^^^^^   ^^^^^^^^
#         型宣言     構築呼び出し：Int が T を填充、() は値構築

# 関数呼び出し推論
numbers: List(Int) = List(Int)()
f: (x: Int) -> String = (x) => x.to_string()
strings: List(String) = map(numbers, f)
# コンパイラが推論：T=Int, R=String
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

# 使用箇所
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

#### 1.4 明示的填充（推論失敗時）

```yaoxiang
# 推論可能時は Type パラメータを省略
numbers: List(Int) = List(Int)()
strings: List(String) = map(numbers, (x: Int) => x.to_string())

# 推論不可能時は明示的に填充必須
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

# 制約を使用：署名に直接型制約を宣言
clone: (T: Clone) -> (value: T) -> T = value.clone()

debug_print: (T: Debug)(value: T) -> Void = {
    formatter = Formatter.new()
    value.fmt(formatter)
    print(formatter.to_string())
}
```

#### 2.2 マルチ制約

```yaoxiang
# マルチ制約構文
combine: (T: Clone + Add)(a: T, b: T) -> T = {
    a.clone() + b
}

# 泛型コンテナのソート
sort: (T: Clone + PartialOrd)(list: List(T)) -> List(T) = {
    # ソートアルゴリズム実装
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

> **制約名の出典（2026-09-22 注）**：`Add` / `Subtract` / `Multiply` / `Divide` / `Modulo` 等の演算子制約は [RFC-011b: 演算子オーバーロードとインターフェース駆動演算子](./011b-operator-overloading.md) で定義・実装済み——`T: Add` ≜ `Add(T, T, T)` インターフェースインスタンス化を登録済み（3型パラメータ、結果型 `O` は明示的）。 `Zero` / `One` / `PartialOrd` / `Fn` / `FnMut` は現在**定義ソースなし**、宙吊り制約名であり、後続RFCでそれぞれ実装予定；それまでの間、これらの名前を含む例は紙上の示意に過ぎない。

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

#### 2.4 組込みマーカtrait：Dup と Clone

**3種類のコピーセマンティクス**：

| 型           | 意味                                | トリガー方式        | 適用シナリオ                         |
| -------------- | ----------------------------------- | ------------------- | ------------------------------------ |
| **原語値コピー** | 代入時に自動値コピー、2つの値は完全に独立 | 代入/引数渡しが自動 | Int, Float, Bool, Char              |
| **Dup**        | 浅いコピー：ハンドラ/トークンをコピー、底层データは共有 | 代入/引数渡しが自動 | `&T` トークン、`ref T`、String/Bytes |
| **Clone**      | 深いコピー：完全な独立コピーを作成    | `value.clone()`     | Clone を実装する任意の型             |

**Dup のセマンティクス**：Dup を実装する型は代入/引数渡し時に所有権を移動しない——コンパイラがハンドラ/トークンをコピーし、複数の保有者が同一の底层データを指す。これはRFC-009所有権モデルにおけるMoveデフォルトセマンティクスの相補。

**Dup と Clone は直交する概念**：

```
Dup = ハンドラをコピー、データは共有（変更は互いに影響）
Clone = データをコピー、コピーは独立（変更は互いに影響しない）
```

**ルール**：

```
1. 原語値型（Int, Float, Bool, Char） — コンパイラが組込み値コピーを実行、Dup に属さない
2. Dup  — 参照/トークン型と内部参照カウントを持つ型にのみ適用
3. Clone — 明示的な深いコピー、任意の型が実装可能
4. デフォルト Move — その他の型はデフォルト Move セマンティクスを維持
```

**Dup である型**：

| 型                 | Dup  | 理由                                        |
| -------------------- | ---- | ------------------------------------------- |
| `&T`（借用トークン） | ✅   | ゼロサイズトークン、トークンコピー = 複数視点で同一データを指す |
| `ref T`              | ✅   | Rc/Arc コピー = 参照カウント+1、堆データを共有 |
| String, Bytes        | ✅   | 内部参照カウント、ハンドラコピーして底层 buffer を共有 |
| `&mut T`（可变トークン） | ❌   | 線形独占、コピー不可                          |
| struct               | 派生 | 全フィールド Dup → struct Dup               |
| enum                 | 派生 | 全 variant の全フィールド Dup → enum Dup      |
| tuple                | 派生 | 全要素 Dup → tuple Dup                      |
| Fn（クロージャ）     | ❌   | 捕獲環境が Dup でない可能性がある            |
| `*T`（生ポインタ）   | ❌   | unsafe、所有権システムに参加しない           |

**Int/Float/Bool/Char は Dup ではない**——これらは値型であり、代入時にコンパイラが自動的に値コピーを行う（2つの値は完全に独立）。これは「浅いコピー」ではなく、コンパイラの原語に対する組込み処理であり、也不要かつ不当にも Dup 型属性で表現すべきではない。

```yaoxiang
# 原語値型：コンパイラが自動的に値コピー（Dup ではない）
x: Int = 42
y = x          # 値コピー、x と y は完全に独立
print(x)       # ✅

# Dup：浅いコピー、ハンドラをコピーしてデータを共有
view: &Point = &point
view2 = view    # ✅ Dup：トークンをコピー、両者が同一の point を指す
print(view.x)   # ✅

# Clone：明示的な深いコピー、独立コピーを作成
backup = big_struct.clone()  # 明示的呼び出し

# 泛型制約
dup_use: (T: Dup) -> T = x         # T: Dup → 浅いコピー可能
clone_use: (T: Clone) -> T = x.clone()  # T: Clone → 深いコピー可能
```

> **注意**：`Send`/`Sync` はユーザに見せる trait ではない。タスク間安全保障は `ref` キーワードとコンパイラの全自动処理で実現——`ref` が Rc または Arc を自動的に選択し、ユーザは Send/Sync を理解する必要がない。

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
# メソッド構文糖衣を使用：Vec.Item, Vec.next, Vec.has_next
# イテレーション位置はラッパーレコードが携带（Vec 自体は生バッファ、index フィールドなし）
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

#### 3.2 泛型関連型（GAT）

```yaoxiang
# より複雑な関連型
Producer: (Item: Type) -> Type = {
    Item: T,
    produce: (Self) -> Option(Item),
}

# 関連型は泛型にできる
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

#### 4.1 コンパイル時値パラメータ

**コア設計**：泛型署名中の `Type` 印は型パラメータを示す；具体的な型（`Int`/`Bool`/`Float` 等）を注釈するパラメータ列は**コンパイル時値パラメータの候補**であり、コンパイル時値パラメータになるかどうかは、その値が**型位置で参照されているか**（値依存）に依存する。`const` キーワードは不要。

> 判斷根拠は**型位置で参照されているか**であり、「具体的な型を注釈したか」ではない：`add: (a: Int, b: Int) -> Int = a + b` では `a`/`b` は运行時値パラメータであり、両者が型構築に参加していないため。

**判定ルール（2ステップ）**：

1. **形態粗選別**：パラメータが非 `Type` の具体型（例：`Int`）を注釈 → 候補に列入。
2. **用途精選別**：候補名が**型位置**に出現（型体フィールド型、内側 `Fn` パラメータ型、 `Assert` 述語、`Array(T, N)` 等の型構築実引数位）→ コンパイル時値パラメータとして確認；さもなければ**运行時値パラメータ**として視る。

| 書き方                                                       | 判定             | 理由                                    |
| ---------------------------------------------------------- | ---------------- | --------------------------------------- |
| `add: (a: Int, b: Int) -> Int = a + b`                     | a/b 运行時値パラメータ | 値位置のみに出現、型構築に参加せず          |
| `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }` | N コンパイル時値パラメータ | N が `Array(T, N)` の型構築実引数位に出現 |
| `factorial: (N: Int) -> (k: N) -> Int`                     | N コンパイル時値パラメータ | N が内側パラメータ `k` の型として出現      |
| `Foo: (T: Type, N: Int) -> Type = { x: T }`                | N 空（下記参照） | N が型体で参照されていない、运行時値パラメータに退化 |

> **値依存本質**：コンパイル時値パラメータは値依存型である——値が**型を構築する**ために使われる場合にのみ、コンパイル時に確定する必要がある。形態（`: Int`）は候補資格を決定し、用途（型位置での出現）がコンパイル時値パラメータかどうかをを決定する。これは §「コンパイル時確定性保証」の「型位置での関数呼び出しはコンパイル時に評価される」と同一の判定根拠である。

```yaoxiang
# ════════════════════════════════════════════════════════
# コンパイル時値パラメータ：N は型位置（Measure の長さスロット）で参照されている
# ════════════════════════════════════════════════════════
Measure: (T: Type, N: Int) -> Type = {
    data: Array(T, N),  # N が型構築実引数位に出現 → コンパイル時値パラメータ
    length: N,
}

# 使用方法：factorial(5) は型位置で評価（コンパイル時）、結果 120 が型に埋め込まれる
m: Measure(Int, factorial(5))  # Measure(Int, 120)

# ════════════════════════════════════════════════════════
# 値依存：N は内側パラメータ k の型として出現
# ════════════════════════════════════════════════════════
# N はコンパイル時値パラメータ（(k: N) の型位に出現）；
# k は运行時値パラメータ、その型はリテラル型 N（単一値型）。
factorial: (N: Int) -> (k: N) -> Int = {
    return match k {
        0 => 1,
        _ => k * factorial(k - 1)
    }
}
```

> **空候補の処理**：具体的な型を注釈したが型位置で参照されていない候補（上表 `Foo` の `N`）は运行時値パラメータに退化（関数级パス）。型構築子パスの空候補は运行時スロットを占めることができず（型構築子はコンパイル時に評価）、宣言側で直接エラー [E1094] を送出——「N はコンパイル時値パラメータとして宣言されたが型体で参照されていない」——これまで黙って破棄していたためインスタンス化アリティ不一致が発生していた。

#### 4.2 コンパイル時計算

```yaoxiang
# ════════════════════════════════════════════════════════
# コンパイル時計算例
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

# 使用：コンパイル時に計算され、Matrix(Float, 3, 3) が生成
identity_3x3: Matrix(Float, 3, 3) = identity_matrix(Float, 3)(3)
```

### Never と Void：型システムの ⊥ と ⊤

YaoXiangの型システムはCurry-Howard同型において ⊥（偽/空型）と ⊤（真/Unit）の両方を同時に持ち、`Never` と `Void` の2つの組込み型名がこれを担います：

**Never（⊥）** — 3つの交渉不可なカーネル性質：

1. **零構築子**：いかなるリテラルも式も `Never` 型の値を生成できない。これはメタレベルの性質であり、組込みが必要。
2. **爆発原理**：`Never <: T` は任意の型 `T` に対して成立。`Never` 値は任意の型として使用可能——これが `assert(false)` の後のコードが型チェックを通る理由（実行されることはないが）。
3. **発散マーク**：`f: (...) -> Never` は `f` が戻らないことを保証。コンパイラはこれを使って dead code 分析を行う。

`Never` は組込み型名であり、キーワードではなく、parser は無感知。空和型リテラル構文は開放しない。

**Void（⊤、即ち Unit）** — 丁度1つの住人（デフォルト void 値）があり、真命題「恒真」の担い手である。`Void` は零フィールド積型の単位元、`Never` は零variant和型の単位元——二者はお対偶。`x: Void = <デフォルト>` は合法、`x: Never = ...` は右辺を書けない。

#### 4.3 コンパイル時検証（標準ライブラリ実装）

```yaoxiang
# ════════════════════════════════════════════════════════
# 標準ライブラリ実装：条件型を利用
# ════════════════════════════════════════════════════════

# 標準ライブラリ定義
# IsTrue：値宇宙から型宇宙への橋——Bool 真値を型にマッピング
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      # ⊤、値あり、プログラム継続
    false => Never,    # ⊥、値なし、発散
}

# Assert：コンパイル時リファイン型プリミティブ——Bool 命題の型レベル表述
Assert: (cond: Bool) -> Type = IsTrue(cond)
#
# cond が true  → Assert(true)  = Void    （恒真、擦除）
# cond が false → Assert(false) = Never   （恒偽、コンパイルエラー/発散）
# cond が判定不能   → 証明パイプラインが dispatch モードに従い決定：
#                  CompileTime → Unknown、prove を要求
#                  Runtime     → チェックを挿入し、Γ 仮定を注入

# 使用方法1：型定義内で制約として使用
Bounded: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    # コンパイル時チェック：N は 0 より大きい必要がある（Assert は型位置）
    length: Assert(N > 0),
}

# 使用方法2：式中で使用
IntArray: (N: Int) -> Type = Array(Int, N)
# 検証：IntArray(10) のサイズは sizeof(Int) * 10 に等しい
Assert(size_of(IntArray(10)) == sizeof(Int) * 10)
```

#### 4.4 コンパイル時泛型特殊化

```yaoxiang
# 小配列最適化：関数オーバーロードでコンパイル時泛型特殊化を実装

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

> **Curry-Howard同型**：条件型はCurry-Howardの視点から見ると論理における **case 解析**。`Bool` 型は2つの可能な値（True/False）を持つ命題に対応し、`If` はその命題の真偽に応じて異なる結果を選択——これは論理における case 選言そのものである。`match C { True => T, False => E }` は実際には「命題 C が True のとき結論は T、C が False のとき結論は E」を表現している。

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

> **Curry-Howard同型**：型族は「命題即ち型」を最も直接的に体現。`Add: (A: Type, B: Type) -> Type` は「型レベルで加算関数を書いた」のではなく、**自然数加算についての命題を構築**している。`(Zero, B) => B` は「命題 Add(Zero, B) は B と同値」、`(Succ(A'), B) => Succ(Add(A', B))` は「Add(A', B) が成立すれば Add(Succ(A'), B) も成立」を意味する。これはPeano公理における加算定義そのものである。型チェッカーがこの match 式が通過することを検証することは、この定義の論理的一貫性を検証することと同値。

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

# 型レベル加算（Curry-Howard：case analysis + 再帰呼び出し、完全な帰納のため終了性チェックが必要）
Add: (A: Type, B: Type) -> Type = match (A, B) {
    (Zero, B) => B,
    (Succ(A'), B) => Succ(Add(A', B)),
}

# 例：コンパイル時に 2 + 3 を計算
Two: Type = Succ(Succ(Zero))
Three: Type = Succ(Succ(Succ(Zero)))
Five: Type = Add[Two, Three]  # Succ(Succ(Succ(Succ(Succ(Zero)))))
```

### 6. 関数オーバーロード特化

#### 6.1 基本特化

```yaoxiang
# 基本特殊化：関数オーバーロードを使用（コンパイラが自動選択）
sum: (arr: Vec(Int)) -> Int = {
    # より効率的なコードにコンパイル
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Vec(Float)) -> Float = {
    # SIMD命令を使用
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
# RFC-010構文に完全に準拠した特殊化方式：関数オーバーロード

# 具体型特殊化
sum: (arr: Vec(Int)) -> Int = {
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Vec(Float)) -> Float = {
    return simd_sum_float(arr.data, arr.length)
}

# 泛型実装（コンパイラが自動的に最適をを選択）
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

# コンパイラが自動的に最適をを選択
sum(int_arr)     # sum: (Vec(Int)) -> Int を選択
sum(float_arr)    # sum: (Vec(Float)) -> Float を選択
```

#### 6.3 関数オーバーロードとインライン最適化の見事な組み合わせ

**鍵特性**：関数オーバーロードとインライン最適化は自然に組み合わせ、ゼロコスト抽象化を実現。

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
# コンパイラが自動的に最適をを選択し、次にインライン展開
result = native_sum_int(int_arr.data, int_arr.length)

# 手書き最適化コードと完全等価！関数呼び出しオーバーヘッドなし！
```

**コア優位性**：

1. **コンパイラのIntelligent選択**

   ```yaoxiang
   sum(int_arr)      # 自動的に sum: (Vec(Int)) -> Int を選択
   sum(float_arr)    # 自動的に sum: (Vec(Float)) -> Float を選択
   sum(custom_arr)  # 自動的に sum: (T: Type) -> ((arr: Vec(T)) -> T) を選択
   ```

2. **インライン最適化**
   - 小関数は自動的に呼び出し点にインライン展開
   - ゼロ関数呼び出しオーバーヘッド
   - 手書き最適化コードと完全等価

3. **型安全**
   - コンパイル時型チェック
   - 実行時ゼロオーバーヘッド
   - 仮想関数テーブル不要

4. **RFC-010との完璧な整合**

   ```yaoxiang
   # 統一構文を完全に使用
   name: type = value
   # impl、where等の新しいキーワード不要
   ```

**実際の応用例**：

```yaoxiang
# パフォーマンス敏感な数値計算
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
fibonacci(10)      # Int 版を選択、完全インライン展開
fibonacci(10.5)    # Float 版を選択、Binetの公式を使用
```

**これは何を意味するのか？**

- ✅ **泛型特殊化** → 関数オーバーロードで自然に解決
- ✅ **パフォーマンス最適化** → インライン展開が自動完了
- ✅ **コード再利用** → 1つの関数名、複数の実装
- ✅ **ゼロコスト抽象化** → コンパイル時多相、ゼロ実行時オーバーヘッド
- ✅ **新しいキーワード不要** → RFC-010統一構文に完璧に準拠

### 7. デッドコード除去メカニズム

#### 7.1 インスタンス化グラフ分析

```rust
// コンパイラ内部：泛型インスタンス化依存グラフを構築
struct InstantiationGraph {
    // ノード：泛型インスタンス化
    nodes: HashMap<InstanceKey, InstanceNode>,

    // エッジ：使用関係
    edges: HashMap<InstanceKey, Vec<InstanceKey>>,
}

struct InstanceKey {
    generic: FunctionId,  // 泛型関数ID
    type_args: Vec<TypeId>,  // 型引数
    const_args: Vec<ConstId>,  // Const引数
}

// アルゴリズム：到達可能性分析
fn eliminate_dead_instantiations(graph: &InstantiationGraph) {
    let mut reachable = HashSet::new();

    // エントリーポイントから開始（main、エクスポート関数等）
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
````

#### 7.2 使用箇所分析

```yaoxiang
# ソースコード分析
map: (T: Type, R: Type)(list: List(T), f: Fn(T) -> R) -> List(R) = ...

# 使用箇所1：map(Int, Int) をインスタンス化
int_list = List(Int)()
int_list.push(1)
int_list.push(2)
int_list.push(3)
doubled = map(int_list, (x) => x * 2)  # map[Int, Int] が必要

# 使用箇所2：map(String, String) をインスタンス化
string_list = List(String)()
string_list.push("a")
string_list.push("b")
string_list.push("c")
uppercased = map(string_list, (s) => s.to_uppercase())  # map[String, String] が必要

# 未使用：map[Float, Float] 等
# これらの泛型インスタンスは生成されない

# コンパイル後は使用されたインスタンスのみを含む
map_Int_Int: (list: List(Int), f: Fn(Int) -> Int) -> List(Int) = ...
map_String_String: (list: List(String), f: Fn(String) -> String) -> List(String) = ...
```

#### 7.3 コンパイル時泛型DCE

```yaoxiang
# コンパイル時分析：コンパイル時泛型使用状況
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
}

# 実際の使用状況
arr_10_int = Array(Int, 10)(data=[1, 2, 3, 4, 5, 6, 7, 8, 9, 10])  # 2層：型パラメータ + 構築パラメータ
arr_100_int = Array(Int, 100)()   # 空構築、データは事後に代入

# コンパイル後は使用されたSizeのみを生成
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
int_list = List(Int)()
int_list.push(1)
int_list.push(2)
int_list.push(3)
doubled = map(int_list, (x) => x * 2)  # map(Int, Int) をインスタンス化

# モジュールC
# C.yx
use A.{map}
string_list = List(String)()
string_list.push("a")
string_list.push("b")
string_list.push("c")
uppercased = map(string_list, (s) => s.to_uppercase())  # map(String, String) をインスタンス化

# コンパイル分析：
# - モジュールBは map[Int, Int] を使用
# - モジュールCは map[String, String] を使用
# - コンパイル後バイナリはこの2つのインスタンスのみを含む
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

    // 6. 最適化を実行
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

# ✅ 泛型方案：自動導出
# 関数オーバーロード方式で自動導出
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
result_type = Add[Int, Float]  # Float に推論
```

> **RFC-011bとの関係（2026-09-22 注）**：本節の型族 `Add(A, B)` は [RFC-011b](./011b-operator-overloading.md) 演算子インターフェース登録台の型レベル視点——コア登録 `Add(Int, Float, Float)` と本表 `(Int, Float) => Float` は同一のルールであり、ユーザのインターフェースインスタンス化たびにこの表に行が追加される。§5.2 のPeano型レベル `Add` は純粋な型レベルの計算（同名別物）であり、値レベルの演算子インターフェースと干渉しない——演算子は実装登録台をクエリし、名前解決を経由しない。

### 9. 例

#### 9.1 完全な泛型コンテナ例

```yaoxiang
# ======== 1. 泛型コンテナを定義 ========
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

    # 泛型メソッド（T は外側の List(T) から自動的にスコープに入る）
    push: (self: List(T), item: T) -> Void,
    pop: (self: List(T)) -> Option(T),
    map: (R: Type) -> ((self: List(T), f: (T) -> R) -> List(R)),
    filter: (self: List(T), predicate: (T) -> Bool) -> List(T),
    fold: (U: Type) -> ((self: List(T), initial: U, f: (U, T) -> U) -> U),
}

# ======== 2. 泛型メソッドを実装 ========
# 関数定義は List 名前空間下（List. プレフィックス = 名前空間帰属）
# list.push(item) のような . 呼び出し構文を動作させるには、明示的バインディングが必要：List.push = push[0]
# self は約束パラメータ名に過ぎず、コンパイラは名前ではなく型を見る

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

# ======== 3. 型制約を使用 ========
# Clone for List を実装
List.clone: (T: Clone) -> ((self: List(T)) -> List(T)) = {
    result = List(T)()
    for i in 0..self.length {
        result.push(self.data[i].clone())
    }
    return result
}

# ======== 4. 使用例 ========
# 泛型Listを作成
numbers = List(Int)()
numbers.push(1)
numbers.push(2)
numbers.push(3)

# 泛型メソッドを使用
doubled = numbers.map((x) => x * 2)
evens = numbers.filter((x) => x % 2 == 0)

# foldを使用して計算
sum = numbers.fold(0, (acc, x) => acc + x)  # sum = 6

# 泛型組み合わせ
sum_of_evens = numbers
    .filter((x) => x % 2 == 0)
    .map((x) => x * 2)
    .fold(0, (acc, x) => acc + x)  # sum_of_evens = 8
```

#### 9.2 泛型アルゴリズム例

```yaoxiang
# ======== 1. 泛型ソートアルゴリズム ========
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

# ======== 2. IntComparator実装 ========
# 関数オーバーロードを使用
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
numbers = Vec(Int)(3, 1, 4, 1, 5, 9, 2, 6)
sorted = quicksort(numbers, Comparator(Int)())

# String配列をソート（StringComparatorが必要）
strings = Vec(String)("hello", "world", "foo", "bar")
sorted_strings = quicksort(strings, Comparator(String)())
```

#### 9.3 コンパイル時泛型例

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
# コンパイル時にサイズが既知の行列を作成
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
   - コンパイル時単態化、実行時オーバーヘッドなし
   - 仮想関数不要、RTTIなし

2. **デッドコード除去**
   - コンパイル時分析、使用された泛型のみをインスタンス化
   - コード膨張が制御可能

3. **マクロ代替**
   - 型安全なコード生成
   - IDEフレンドリー、エラー情報が明確

4. **コンパイル時計算**
   - コンパイル時泛型がコンパイル時計算をサポート
   - 次元検証等の特性
   - `const` キーワード不要、純粋な型制約

### 劣位性

1. **コンパイル時間**
   - 泛型インスタンス化でコンパイル時間が増加
   - 制約解決が遅い可能性

2. **メモリ使用量**
   - コンパイラのメモリ使用量が増加
   - キャッシュ機構にメモリが必要

3. **実装複雑度**
   - 制約解決器が複雑
   - 型レベル計算エンジンが複雑

4. **エラー診断**
   - 泛型エラーが複雑になる可能性
   - 明確なエラー提示が必要

### 緩和措置

1. **キャッシング戦略**
   - インスタンス化結果をキャッシュ
   - LRUキャッシュでメモリを制限

2. **インクリメンタルコンパイル**
   - コンパイル結果をキャッシュ
   - インクリメンタルインスタンス化

3. **エラー提示**
   - 明確なエラー情報
   - 泛型パラメータ推論ヒント

4. **パラレルコンパイル**
   - 泛型を並列にインスタンス化
   - マルチスレッド制約解決

## 代替方案

| 方案       | なぜ選択しないか           |
| ---------- | -------------------------- |
| 基礎泛型のみ | 複雑なマクロを代替できない |
| 純粋マクロシステム | 型安全がなく、エラー情報が悪い |
| 制約のみ依存 | 柔軟性が不足               |
| 実行時泛型 | パフォーマンスオーバーヘッドがある |

### リスク

| リスク           | 影響           | 緩和措置        |
| -------------- | -------------- | --------------- |
| 制約解決複雑度 | コンパイル時間が長すぎる | インクリメンタル解決 + キャッシュ |
| コード膨張       | バイナリファイルが大きすぎる | DCE + 閾値制御  |
| 実装複雑度     | 開発サイクルが延長       | 段階的実装      |
| エラー診断       | ユーザ体験が悪い         | 詳細なエラー情報 |

## 開放問題

### 审议中の問題

| 议题       | 説明                       | 状態   |
| ---------- | -------------------------- | ------ |
| インスタンス化戦略 | Eager vs Lazy vs Threshold | 审议中 |
| キャッシュサイズ   | LRUキャッシュ容量設定      | 审议中 |
| エラー診断   | 泛型エラー情報の詳細度     | 审议中 |

### 後続最適化

| 最適化項目        | 価値 | 実装難易度 |
| ------------- | ---- | -------- |
| インスタンス化グラフ分析  | 高   | 中       |
| 型レベルプログラミングDSL | 中   | 高       |
| 泛型パフォーマンスベンチマーク | 中   | 低       |

## 付録

### 構文BNF

```bnf
# 泛型パラメータは統一的な () 構文を使用し、関数型の一部として
# 例：map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R))

# 型制約（泛型パラメータ内）
type_bound ::= identifier
             | identifier '+' identifier ('+' identifier)*

# パラメータ宣言（型 + 名前）
parameter ::= identifier ':' type

parameters ::= parameter (',' parameter)*

# 関数宣言：name: type = expression
# 泛型パラメータは関数型の最初の引数グループ：(T: Type) -> ((params) -> return)
function ::= identifier ':' type '=' (expression | block)

# メソッド宣言：Type.method: type = expression
method ::= identifier '.' identifier ':' type '=' (expression | block)

# 型定義（統一 Binding 構文）
# 泛型型は List: (T: Type) -> Type = { ... } のように記述
generic_type ::= identifier ':' type '=' type_expression

# 泛型パラメータの Type はコンパイラが実引数型から自動的に填充
# 例：map(numbers, f)、T は numbers: List(Int) から抽出、R は f: (Int) -> String から抽出
```

## ライフサイクルと归宿

```
┌─────────────┐
│   草案      │  ← 現在の状態
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  審査中     │  ← コミュニティ讨论とフィードバックを募集中
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  已受諾     │    │  却下       │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   accepted/ │    │    rfc/     │
│ (正式設計)  │    │ (原件保持)  │
└─────────────┘    └─────────────┘
```

---

## 参考文献

### YaoXiang公式ドキュメント

- [RFC-010: 統一型構文](./010-unified-type-syntax.md)
- [RFC-009: 所有権モデル](./009-ownership-model.md)
- [RFC-001: 並作モデル](../deprecated/001-concurrent-model-error-handling.md)
- [RFC-008: 実行時モデル](./008-runtime-concurrency-model.md)
- [tutorial/ チュートリアル](../../../tutorial/index.md)

### 外部参考文献

- [Rust泛型システム](https://doc.rust-lang.org/book/ch10-01-syntax.html)
- [C++テンプレート特殊化](https://en.cppreference.com/w/cpp/language/template_specialization)
- [Haskell型クラス](https://www.haskell.org/tutorial/classes.html)
- [Swift泛型](https://docs.swift.org/swift-book/LanguageGuide/Generics.html)
- [単態化最適化](https://llvm.org/docs/Monomorphization.html)
- [デッドコード除去](https://en.wikipedia.org/wiki/Dead_code_elimination)
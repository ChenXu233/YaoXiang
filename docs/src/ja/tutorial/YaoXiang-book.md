# YaoXiang（爻象）プログラミング言語ガイド

> バージョン：v1.2.0
> 状態：草稿
> 著者：晨煦
> 日付：2024-12-31
> 更新：2025-01-20 - 位置インデックスが0から開始（RFC-004）；統一タイプ構文（RFC-010）

---

## 目次

1. [言語概述](#一言語概述)
2. [コア機能](#二コア機能)
3. [タイプシステム](#三タイプシステム)
4. [メモリ管理](#四メモリ管理)
5. [非同期プログラミングと並行処理](#五非同期プログラミングと並行処理)
6. [モジュールシステム](#六モジュールシステム)
7. [メソッドバインディングとカリー化](#七メソッドバインディングとカリー化)
8. [AIフレンドリ設計](#八aiフレンドリ設計)
9. [タイプ集中約束（コア設計哲学）](#九タイプ集中約束コア設計哲学)
10. [クイックスタート](#十クイックスタート)

---

**拡張ドキュメント**：
- [高度なバインディング機能とコンパイラ実装](../works/plans/bind/YaoXiang-bind-advanced.md) - 詳細なバインディング機構、高度な機能、コンパイラ実装、エッジケース処理

---

## 一、言語概述

### 1.1 YaoXiangとは？

YaoXiang（爻象）は『易経』における「爻」と「象」の核心概念に着想を得た実験的な汎用プログラミング言語です。「爻」は卦象を構成する基本記号であり、陰陽の変化を象徴します。「象」は事物の本質的な外在的表現であり、万象万物，代表します。

YaoXiang はこの哲学的思考をプログラミング言語のタイプシステムに統合し、**「すべてがタイプである」**という核心理念を提唱しています。YaoXiang の世界観では：

- **値**はタイプのインスタンスである
- **タイプ**自体もタイプのインスタンスである（メタタイプ）
- **関数**は入力タイプから出力タイプへのマッピングである
- **モジュール**はタイプの名前空間組合である

### 1.2 設計目標

YaoXiang の設計目標は以下几个方面に要約できます：

| 目標 | 説明 |
|------|------|
| **統一されたタイプ抽象化** | タイプは最上位の抽象ユニットであり、言語の意味論を簡素化する |
| **自然なプログラミング体験** | Python 風の構文、読みやすさを重視 |
| **安全なメモリ管理** | Rust 風の所有権モデル、GC なし |
| **無感知な非同期プログラミング** | 非同期を自動管理、明示的な await 不要 |
| **完全なタイプリフレクション** | 実行時のタイプ情報が完全に利用可能 |
| **AI フレンドリな構文** | 厳密な構造化、AI が処理しやすい |

### 1.3 言語位置づけ

| 次元 | 位置づけ |
|------|----------|
| パラダイム | マルチパラダイム（関数型 + 命令型 + オブジェクト指向） |
| タイプシステム | 依存タイプ + パラメータ多相 |
| メモリ管理 | 所有権 + RAII（GC なし） |
| コンパイルモデル | AOT コンパイル（オプション JIT） |
| 対象シナリオ | システムプログラミング、応用開発、AI 支援プログラミング |

### 1.4 コード例

```yaoxiang
# 自動タイプ推断
x: Int = 42                           # 明示的なタイプ
y = 42                                # Int に推断
name = "YaoXiang"                     # String に推断

# デフォルトで不変
x: Int = 10
x = 20                                # ❌ コンパイルエラー！不変

# 統一宣言構文：識別子: タイプ = 式
add: (a: Int, b: Int) -> Int = a + b  # 関数宣言
inc: (x: Int) -> Int = x + 1               # 単一パラメータ関数

# 統一タイプ構文：コンストラクタ即タイプ
type Point = { x: Float, y: Float }
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

# 無感知な非同期（並作関数）
fetch_data: (url: String) -> JSON spawn = {
    HTTP.get(url).json()
}

main: () -> Void = {
    # 値構築：関数呼び出しと完全に同じ
    p = Point(3.0, 4.0)
    r = ok("success")

    data = fetch_data("https://api.example.com")
    # 自動待機、await 不要
    print(data.name)
}

# ジェネリック関数
identity: (T: Type) -> ((x: T) -> T) = x

# 高階関数
apply: (f: (Int) -> Int, x: Int) -> Int = f(x)

# カリー化
add_curried: (a: Int) -> (b: Int) -> Int = a + b
```

---

## 二、コア機能

### 2.1 すべてがタイプ

YaoXiang のコア設計哲学は**すべてがタイプ**です。这意味着在 YaoXiang 中：

1. **値はタイプのインスタンスである**：`42` は `Int` タイプのインスタンスである
2. **タイプはタイプのインスタンスである**：`Int` は `type` メタタイプのインスタンスである
3. **関数はタイプマッピングである**：`add: (Int, Int) -> Int` は関数タイプである
4. **モジュールはタイプ組合である**：モジュールは関数とタイプを含む名前空間である

```yaoxiang
# 値はタイプのインスタンス
x: Int = 42

# タイプはタイプのインスタンス
MyList: type = List(Int)

# 関数はタイプ間のマッピング
add: (a: Int, b: Int) -> Int = a + b

# モジュールはタイプの組合（ファイルを使用してモジュールとする）
# Math.yx
pi: Float = 3.14159
sqrt: (x: Float) -> Float = { ... }
```

### 2.2 数学的抽象化

YaoXiang のタイプシステムはタイプリズムと圏論に基づいており、以下を提供します：

- **依存タイプ**：タイプは値に依存できる
- **ジェネリックプログラミング**：タイプパラメータ化
- **タイプ組合**：合併タイプ、交差タイプ

```yaoxiang
# 依存タイプ：固定長ベクトル
Vector: (T: Type, n: Int) -> Type = vector(T, n)

# タイプ合併
type Number = Int | Float

# タイプ交差
type Printable = printable(fn() -> String)
type Serializable = serializable(fn() -> String)
type Versatile = Printable & Serializable
```

### 2.3 ゼロコスト抽象化

YaoXiang はゼロコスト抽象化を保証します。つまり、高レベルの抽象化は実行時のパフォーマンスオーバーヘッドをもたらしません：

- **単態化**：ジェネリック関数はコンパイル時に具体的なバージョンに展開される
- **インライン最適化**：単純な関数は自動的にインライン展開される
- **スタック割り当て**：小さなオブジェクトはデフォルトでスタックに割り当てられる

```yaoxiang
# ジェネリック展開（単態化）
identity: (T: Type) -> ((x: T) -> T) = x

# 使用
int_val = identity(42)      # identity(Int) -> Int に展開
str_val = identity("hello") # identity(String) -> String に展開

# コンパイル後、追加オーバーヘッドなし
```

### 2.4 自然な構文

YaoXiang は Python 風の構文を採用し、読みやすさと自然言語感を追求しています：

```yaoxiang
# 自動タイプ推断
x = 42
name = "YaoXiang"

# 簡潔な関数定義
greet: (name: String) -> String = "Hello, " + name

# パターン照合
classify: (n: Int) -> String = {
    match n {
        0 -> "zero"
        1 -> "one"
        _ if n < 0 -> "negative"
        _ -> "many"
    }
}
```

### 2.5 完全な構文仕様

YaoXiang は統一された宣言構文を採用しています：**識別子: タイプ = 式**。同時に後方互換性のある古い構文も提供します。

#### 2.5.1 二重構文戦略とタイプ集中約束

革新と互換性のバランスを取るため、YaoXiang は2つの構文形式をサポートしますが、統一された**タイプ集中标注約束**を採用します。

**構文形式の比較：**

| 構文タイプ | 形式 | 状態 | 説明 |
|-----------|------|------|------|
| **新構文（標準）** | `name: Type = Lambda` | ✅ 推奨 | 公式標準、すべての新しいコードはこの形式を使用すべき |
| **旧構文（互換）** | `name(Types) -> Ret = Lambda` | ⚠️ 互換のみ | 歴史的コードのために維持、新プロジェクトでは推奨しない |

**コア約束：タイプ集中标注**

YaoXiang は**「宣言優先、タイプ集中」**の設計約束を採用します：

```yaoxiang
# ✅ 正しい：タイプ情報が宣言行に統一
add: (a: Int, b: Int) -> Int = a + b
#   └─────────────────┘   └─────────────┘
#       完全な型署名         実装ロジック

# ❌ 避ける：タイプ情報が実装に分散
add: (a: Int, b: Int) -> Int = a + b
#     └───────────────┘
#     型が実装体に混在
```

**約束好处：**

1. **構文一貫性**：すべての宣言が `識別子: タイプ = 式` に従う
2. **宣言と実装の分離**：タイプ情報が明確、実装体はロジックに集中
3. **AIフレンドリ性**：AIは宣言行を読むだけで完全な関数シグネチャを理解できる
4. **より安全な変更**：タイプの変更は宣言を変更するだけでよく、実装体に影響しない
5. **カリー化フレンドリ**：明確なカリー化型署名をサポート

**選択の提案**：
- **新プロジェクト**：新構文 + タイプ集中約束を使用する必要がある
- **移行プロジェクト**：新構文とタイプ集中約束に逐步的に移行
- **古いコードの保守**：旧構文，可以使用，但建议采用タイプ集中約束

#### 2.5.2 基本宣言構文

```yaoxiang
# === 新構文（推奨）===
# すべての宣言は：識別子: タイプ = 式

# 変数宣言
x: Int = 42
name: String = "YaoXiang"
mut counter: Int = 0

# 関数宣言
add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1
getAnswer: () -> Int = 42
log: (msg: String) -> Void = print(msg)

# === 旧構文（互換）===
# 関数のみに使用、形式：name(Types) -> Ret = Lambda
add(Int, Int) -> Int = (a, b) => a + b
square(Int) -> Int = (x) => x * x
empty() -> Void = () => {}
getRandom() -> Int = () => 42
```

#### 2.5.3 関数タイプ構文

```
関数タイプ ::= '(' パラメータタイプリスト ')' '->' 戻り値タイプ
           | パラメータタイプ '->' 戻り値タイプ              # 単一パラメータ省略

パラメータタイプリスト ::= [タイプ (',' タイプ)*]
戻り値タイプ ::= タイプ | 関数タイプ | 'Void'

# 関数タイプは第一級市民、ネスト可能
# 高階関数タイプ ::= '(' 関数タイプ ')' '->' 戻り値タイプ
```

| 例 | 意味 |
|------|------|
| `Int -> Int` | 単一パラメータ関数タイプ |
| `(Int, Int) -> Int` | 2パラメータ関数タイプ |
| `() -> Void` | パラメータなし関数タイプ |
| `(Int -> Int) -> Int` | 高階関数：関数を受け取り、Int を返す |
| `Int -> Int -> Int` | カリー化関数（右結合） |

#### 2.5.4 ジェネリック構文（タイプパラメータのみに使用）

```yaoxiang
# ジェネリック関数：<タイプパラメータ> プレフィックス
identity: (T: Type) -> ((x: T) -> T) = x
map: (A: Type, B: Type) -> ((f: (A) -> B, xs: List(A)) -> List(B)) = case xs of
  [] => []
  (x :: rest) => f(x) :: map(f, rest)

# ジェネリックタイプ
List: (T: Type) -> Type
```

#### 2.5.5 Lambda 式構文

```
Lambda ::= '(' パラメータリスト ')' '=>' 式
         | パラメータ '=>' 式              # 単一パラメータ省略

パラメータリスト ::= [パラメータ (',' パラメータ)*]
パラメータ ::= 識別子 [':' タイプ]               # オプションのタイプ注釈
```

| 例 | 意味 | 説明 |
|------|------|------|
| `(a, b) => a + b` | 複数パラメータ Lambda | 宣言と組み合わせて使用：<br>`add: (Int, Int) = (a, b) => a + b` |
| `x => x + 1` | 単一パラメータ省略 | 宣言と組み合わせて使用：<br>`inc: Int = x => x + 1` |
| `(x: Int) => x + 1` | タイプ注釈付き | Lambda 内部の一時的な必要性のためだけに使用 |
| `() => 42` | パラメータなし Lambda | 宣言と組み合わせて使用：<br>`get: () = () => 42` |

**注意**：Lambda 式内のタイプ注釈 `(x: Int) => ...` は**一時的、ローカル**なものであり、主に：
- Lambda 内部でタイプ情報が必要な場合
- 宣言構文と組み合わせて使用する場合（タイプは宣言で既に提供）
- 主なタイプ宣言方式として使用すべきではない

#### 2.5.6 完全な例

```yaoxiang
# === 基本関数宣言 ===

# 基礎関数（新構文）
add: (a: Int, b: Int) -> Int = a + b

# 単一パラメータ関数（2つの形式）
inc: (x: Int) -> Int = x + 1
inc2: (x: Int) -> Int = x + 1

# パラメータなし関数
getAnswer: () -> Int = 42

# 戻り値なし関数
log: (msg: String) -> Void = print(msg)

# === 再帰関数 ===
# 再帰は lambda で自然にサポート
fact: (n: Int) -> Int =
  if n <= 1 then 1 else n * fact(n - 1)

# === 高階関数と関数タイプ代入 ===

# 関数タイプは第一級市民
IntToInt: Type = (Int) -> Int
IntBinaryOp: Type = (Int, Int) -> Int

# 高階関数宣言
applyTwice: (f: IntToInt, x: Int) -> Int = f(f(x))

# カリー化関数
addCurried: (a: Int) -> (b: Int) -> Int = a + b

# 関数合成
compose: (f: Int -> Int, g: Int -> Int) -> (x: Int) -> Int =
  f(g(x))

# 関数を返す関数
makeAdder: (x: Int) -> (y: Int) -> Int =
  x + y

# === ジェネリック関数 ===

# ジェネリック関数
identity: (T: Type) -> ((x: T) -> T) = x

# ジェネリック高階関数
map: (A: Type, B: Type) -> ((f: (A) -> B, xs: List(A)) -> List(B)) =
  case xs of
    [] => []
    (x :: rest) => f(x) :: map(f, rest)

# ジェネリック関数タイプ
Transformer: Type = (A: Type, B: Type) -> ((A) -> B)

# ジェネリックタイプを使用
applyTransformer: (A: Type, B: Type) -> ((f: Transformer(A, B), x: A) -> B) =
  f(x)

# === 複雑なタイプ例 ===

# ネストされた関数タイプ
higherOrder: (A: Type) -> ((f: (A) -> Int) -> (A) -> Int) =
  f => x => f(x) + 1

# 複数パラメータ高階関数
zipWith: (A: Type, B: Type, C: Type) -> ((f: (A, B) -> C, xs: List(A), ys: List(B)) -> List(C)) =
  case (xs, ys) of
    ([], _) => []
    (_, []) => []
    (x::xs', y::ys') => f(x, y) :: zipWith(f, xs', ys')

# 関数タイプエイリアス
Predicate: (T: Type) -> Type = { apply: (T) -> Bool }
Mapper: Type = (A: Type, B: Type) -> ((A) -> B)
Reducer: Type = (A: Type, B: Type) -> ((B, A) -> B)

# === 旧構文例（後方互換のみ）===
# 新コードでは推奨しない

mul(Int, Int) -> Int = (a, b) => a * b    # 複数パラメータ
square(Int) -> Int = (x) => x * x          # 単一パラメータ
empty() -> Void = () => {}                  # パラメータなし
get_random() -> Int = () => 42              # 戻り値あり

# 同等の新構文（推奨）
mul: (a: Int, b: Int) -> Int = a * b
square: (x: Int) -> Int = x * x
empty: () -> Void = {}
get_random: () -> Int = 42
```

#### 2.5.7 構文解析ルール

**タイプ解析優先度：**

| 優先度 | タイプ | 説明 |
|--------|------|------|
| 1 (最高) | ジェネリック適用 `List(T)` | 左結合 |
| 2 | 括弧 `(T)` | 結合性の変更 |
| 3 | 関数タイプ `->` | 右結合 |
| 4 (最低) | 基本タイプ `Int, String` | アトミックタイプ |

**タイプ解析例：**

```yaoxiang
# (A -> B) -> C -> D
# 解析: ((A -> B) -> (C -> D))

# A -> B -> C
# 解析: (A -> (B -> C))  # 右結合

# (Int -> Int) -> Int
# 解析: 関数を受け取り、Int -> Int を返す

# List<Int -> Int>
# 解析: List の要素タイプは Int -> Int
```

**Lambda 解析例：**

```yaoxiang
# a => b => a + b
# 解析: a => (b => (a + b))  # 右結合、カリー化

# (a, b) => a + b
# 解析: 2つのパラメータを受け取り、a + b を返す
```

#### 2.5.8 タイプ推断ルール

YaoXiang は**二層処理**戦略を採用します：解析層は寛容に放过し、タイプ検査層は厳密に推断します。

**解析層ルール：**
- パーザは構文構造のみを検証し、タイプ推断を行わない
- タイプ标注のない宣言は、タイプ标注フィールドが `None`
- 基本的な構文構造に従うすべての宣言は解析を通過できる
- **关键点**：`add: (a: Int, b: Int) -> Int = a + b` は解析層では**合法**である

**タイプ検査層ルール：**
- セマンティックの正しさを検証、タイプの完全性を含む
- **パラメータにはタイプ标注が必要**：これは必須要件
- 戻り値タイプは推断可能だが、パラメータタイプは明示的に宣言する必要がある

**完全なタイプ推断ルール：**

| シナリオ | パラメータ推断 | 戻り値推断 | 解析結果 | タイプ検査結果 | 推奨度 |
|------|---------|---------|----------|-------------|---------|
| **標準関数** | 标注タイプを使用 | 标注タイプを使用 | ✅ | ✅ | ⭐⭐⭐⭐⭐ |
| `add: (a: Int, b: Int) -> Int = a + b` | | | | | |
| **部分推断** | 标注タイプを使用 | 式から推断 | ✅ | ✅ | ⭐⭐⭐⭐ |
| `add: (Int, Int) = (a, b) => a + b` | | | | | |
| `inc: (x: Int) -> Int = x + 1` | | | | | |
| `get: () = () => 42` | | | | | |
| **旧構文部分推断** | 标注タイプを使用 | 式から推断 | ✅ | ✅ | ⭐⭐⭐ (互換) |
| `add(Int, Int) = (a, b) => a + b` | | | | | |
| `square(Int) = (x) => x * x` | | | | | |
| **パラメータ無标注** | **推断不可** | - | ✅ | ❌ エラー | ❌ 禁止 |
| `add: (a: Int, b: Int) -> Int = a + b` | | | | | |
| `identity: (T: Type) -> ((x: T) -> T) = x` | | | | | |
| **块の戻り値标注なし** | - | 块内容から推断 | ✅ | ✅ | ⭐⭐⭐⭐ |
| `main = () => {}` | | | | | |
| `get = () => { return 42; }` | | | | | |
| **块の戻り値标注なし（明示的戻り値なし）** | - | `Void` に推断 | ✅ | ✅ 非推奨 | ⚠️ 避ける |
| `bad = (x: Int) => { x }` | | | | | |

**詳細な推断ルール：**

```
解析層：構文構造のみを見る
├── 構造が正しい → 通過
└── 構造がエラー → エラー

タイプ検査層：セマンティックを検証
├── パラメータタイプ推断
│   ├── パラメータにタイプ标注がある → 标注タイプを使用 ✅
│   ├── パラメータにタイプ标注がない → 拒否 ❌
│   └── Lambda パラメータには标注が必要 → 必須要件
│
├── 戻り値タイプ推断
│   ├── return expr がある → expr から推断 ✅
│   ├── return がなく、式がある → 式から推断 ✅
│   ├── return がなく、块 `{ ... }` がある
│   │   ├── 块が空 `{}` → Void ✅
│   │   ├── 块に return がある → return から推断 ✅
│   │   └── 块に return がなく、明示的戻り値もない → Void に推断 ✅（だが非推奨）
│   └── 推断不可 → 拒否 ❌
│
└── 完全推断不可 → 拒否 ❌
```

**注意**：`bad = (x: Int) => { x }` 这种形式可以推断返回类型为 `Void`，但非常不推荐，因为：
- 代码意图不明确
- 容易造成理解错误
- 不符合函数式编程的表达式风格

**推断示例：**

```yaoxiang
# === 推断成功 ===

# 標準形式
main: () -> Void = () => {}                    # 完全标注
num: () -> Int = () => 42                      # 完全标注
inc: (x: Int) -> Int = x + 1                   # 単一パラメータ省略

# 部分推断（新構文）
add: (Int, Int) = (a, b) => a + b              # パラメータに标注、戻り値を推断
square: (x: Int) -> Int = x * x                # パラメータに标注、戻り値を推断
get_answer: () = () => 42                      # パラメータに标注（空）、戻り値を推断

# 部分推断（旧構文、互換）
add2(Int, Int) = (a, b) => a + b               # パラメータに标注、戻り値を推断
square2(Int) = (x) => x * x                    # パラメータに标注、戻り値を推断

# return から推断
fact: Int -> Int = (n) => {
    if n <= 1 { return 1 }
    return n * fact(n - 1)
}

# === 推断失敗 ===

# パラメータが推断不可（解析通過、タイプ検査失敗）
add: (a: Int, b: Int) -> Int = a + b                          # ✗ パラメータにタイプなし
identity: (T: Type) -> ((x: T) -> T) = x                              # ✗ パラメータにタイプなし

# 明示的戻り値のない块
no_return = (x: Int) => { x }                  # ✗ 块に return がなく、隐黙的戻り値を推断不可

# 完全推断不可
bad_fn: (T: Type) -> ((x: T) -> T) = x                                # ✗ パラメータと戻り値が推断不可
```

#### 2.5.9 旧構文（後方互換）

YaoXiang は歴史的コードとの互換性のために旧構文サポートを提供します、**新コードでは推奨しません**。

```
旧構文 ::= 識別子 '(' [パラメータタイプリスト] ')' '->' 戻り値タイプ '=' Lambda
```

| 機能 | 標準構文 | 旧構文 |
|------|---------|--------|
| 宣言形式 | `name: Type = ...` | `name(Types) -> Type = ...` |
| パラメータタイプ位置 | タイプ标注内 | 関数名の後の括弧内 |
| 空パラメータ | `()` を書く必要がある | `()` を省略可能 |
| **推奨度** | ✅ **公式推奨** | ⚠️ **後方互換のみ** |
| **使用シナリオ** | すべての新コード | 歴史的コードの保守 |

**非推奨理由：**
1. **学習コスト**：標準構文と一貫性がなく、言語の複雑さを增加
2. **一貫性**：パラメータタイプ位置が統一されていない（一方はタイプ标注内、もう一方は関数名後）
3. **保守コスト**：パーザは2つの形式を追加で処理する必要がある
4. **AIフレンドリ性**：AI がコードを理解和生成する难度を增加

**移行提案：**
```yaoxiang
# 古いコード（推奨しない）
mul(Int, Int) -> Int = (a, b) => a * b
square(Int) -> Int = (x) => x * x
empty() -> Void = () => {}

# 新しいコード（推奨）
mul: (Int, Int) -> Int = (a, b) => a * b
square: (Int) -> Int = (x) => x * x
empty: () -> Void = () => {}
```

---

## 三、タイプシステム

### 3.1 タイプ階層

YaoXiang のタイプシステムは階層的です：

```
┌─────────────────────────────────────────────────────────────┐
│                    YaoXiang タイプ階層                       │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  type (メタタイプ)                                           │
│    │                                                        │
│    ├── 原タイプ (Primitive Types)                           │
│    │   ├── Void, Bool                                       │
│    │   ├── Int, Uint, Float                                 │
│    │   ├── Char, String, Bytes                              │
│    │                                                        │
│    ├── コンストラクタタイプ (Constructor Types)             │
│    │   ├── Name(args)              # 単一コンストラクタ（構造体）      │
│    │   ├── A(T) | B(U)             # 複数コンストラクタ（合併/列挙型）   │
│    │   ├── A | B | C               # ゼロパラメータコンストラクタ（列挙型）      │
│    │   ├── tuple (T1, T2, ...)                            │
│    │   ├── list(T), dict(K, V)                           │
│    │                                                        │
│    ├── 関数タイプ (Function Types)                          │
│    │   fn (T1, T2, ...) -> R                               │
│    │                                                        │
│    ├── ジェネリックタイプ (Generic Types)                    │
│    │   List(T), Map(K, V), etc.                            │
│    │                                                        │
│    ├── 依存タイプ (Dependent Types)                         │
│    │   (n: Int) -> Type                               │
│    │                                                        │
│    └── モジュールタイプ (Module Types)                      │
│        ファイル即モジュール                                           │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 タイプ定義

```yaoxiang
# 統一タイプ構文：コンストラクタのみ、enum/struct/union キーワードなし
# ルール：| で分隔的都是コンストラクタ、コンストラクタ名(パラメータ) 即タイプ

# === ゼロパラメータコンストラクタ（列挙型スタイル）===
type Color = { red | green | blue }              # red() | green() | blue() と同等

# === 複数パラメータコンストラクタ（構造体スタイル）===
type Point = { x: Float, y: Float }       # コンストラクタ即タイプ

# === ジェネリックコンストラクタ ===
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }           # ジェネリック合併

# === 混合コンストラクタ ===
type Shape = circle(Float) | rect(Float, Float)

# === 値構築（関数呼び出しと完全に同じ）===
c: Color = green                              # green() と同等
p: Point = Point(1.0, 2.0)
r: Result(Int, String) = ok(42)
s: Shape = circle(5.0)

# === インターフェース定義（フィールドがすべて関数のレコードタイプ）===
type Drawable = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}

type Serializable = {
    serialize: () -> String
}

# === インターフェース実装（タイプ末尾にインターフェース名を列出）===
type Point = {
    x: Float,
    y: Float,
    Drawable,        # Drawable インターフェースを実装
    Serializable     # Serializable インターフェースを実装
}
```

### 3.3 タイプ操作

```yaoxiang
# タイプを値として
MyInt = Int
MyList = List(Int)

# タイプリフレクション（コンストラクタパターン照合）
describe_type: (type) -> String = (t) => {
    match t {
        Point(x, y) -> "Point with x=" + x + ", y=" + y
        red -> "Red color"
        ok(value) -> "Ok value"
        _ -> "Other type"
    }
}
```

### 3.4 タイプ推断

YaoXiang は強力なタイプ推断能力を持っています：

```yaoxiang
# 基本推断
x = 42                    # Int に推断
y = 3.14                  # Float に推断
z = "hello"               # String に推断

# 関数戻り値の推断
add: (a: Int, b: Int) -> Int = a + b

# ジェネリック推断
first: (T: Type) -> ((list: List(T)) -> Option(T)) = (list) => {
    if list.length > 0 { some(list[0]) } else { none }
}
```

---

## 四、メモリ管理

### 4.1 所有権モデル

YaoXiang は**所有権モデル**を採用してメモリを管理し、各値には一意の所有者がいます：

```yaoxiang
# === デフォルト Move（ゼロコピー）===
p: Point = Point(1.0, 2.0)
p2 = p              # Move、所有権移転、p は無効

# === ref キーワード = Arc（安全な共有）===
shared = ref p      # Arc、スレッド安全

spawn(() => print(shared.x))   # ✅ 安全

# === clone() 明示的複製 ===
p3 = p.clone()      # p と p3 は独立
```

### 4.2 Move セマンティクス（デフォルト）

```yaoxiang
# 代入 = Move（ゼロコピー）
p: Point = Point(1.0, 2.0)
p2 = p              # Move、p は無効

# 関数引数渡渡 = Move
process: (p: Point) -> Void = {
    # p の所有権が移転してくる
}

# 戻り値 = Move
create: () -> Point = {
    p = Point(1.0, 2.0)
    return p        # Move、所有権移転
}
```

### 4.3 ref キーワード（Arc）

```yaoxiang
# ref キーワードは Arc を作成（参照カウント）
p: Point = Point(1.0, 2.0)
shared = ref p      # Arc、スレッド安全

spawn(() => print(shared.x))   # ✅ 安全

# Arc は自動的にライフサイクルを管理
# shared がスコープを離れるとき、カウントがゼロになり自動的に解放
```

### 4.4 clone() 明示的複製

```yaoxiang
# 元の値を保持する必要がある場合、clone() を使用
p: Point = Point(1.0, 2.0)
p2 = p.clone()   # p と p2 は独立

p.x = 0.0        # ✅
p2.x = 0.0       # ✅ 互いに影響しない
```

### 4.5 unsafe コードブロック（システムレベル）

```yaoxiang
# 裸ポインタは unsafe ブロック内でのみ使用可能
p: Point = Point(1.0, 2.0)

unsafe {
    ptr: *Point = &p     # 裸ポインタ
    (*ptr).x = 0.0       # ユーザーが安全を保証
}
```

### 4.6 RAII

```yaoxiang
# RAII 自動解放
with_file: (path: String) -> String = {
    file = File.open(path)  # 自動的に開く
    content = file.read_all()
    # 関数終了、file は自動的に閉じる
    content
}
```

### 4.7 Send / Sync 制約

| 制約 | セマンティクス | 説明 |
|------|------|------|
| **Send** | スレッド間安全な転送が可能 | 値を別のスレッドに移動できる |
| **Sync** | スレッド間安全な共有が可能 | 不変参照を別のスレッドに共有できる |

```yaoxiang
# ref T は自動的に Send + Sync を満たす（Arc はスレッド安全）
p: Point = Point(1.0, 2.0)
shared = ref p

spawn(() => print(shared.x))   # ✅ Arc はスレッド安全

# 裸ポインタ *T は Send/Sync を満たさない
unsafe {
    ptr: *Point = &p         # 単一スレッド内でのみ使用可能
}
```

### 4.9 実装していないもの

| 機能 | 理由 |
|------|------|
| ライフタイム `'a` | 参照概念がないため、ライフタイムが不要 |
| 借用検査器 | ref = Arc で代替 |
| `&T` 借用構文 | Move セマンティクスを使用 |

---

## 五、非同期プログラミングと並行処理

> 「万物並作，吾以觀復。」——《易·復卦》
>
> YaoXiang は**並作モデル**を採用しており、これは**遅延評価**に基づく無感知な非同期並行パラダイムです。そのコア設計理念は：**開発者が同期적이고逐次的な思考でロジックを記述し、言語ランタイムがその中の計算ユニットを万物並作のように自動的かつ効率的に並行実行させ、最終的に統一的に協調させる**ことです。

> 詳細は [『並作モデル白書』](YaoXiang-async-whitepaper.md) と [非同期実装方案](YaoXiang-async-implementation.md) を参照してください。

### 5.1 並作モデルコアコンセプト

#### 5.1.1 並作図：万物並作のステージ

すべてのプログラムはコンパイル時に**有向非巡回計算グラフ（DAG）**に変換され、これを**並作図**と呼びます。ノードは式計算を表し、エッジはデータ依存関係を表します。このグラフは遅延的であり、ノードはその出力が**実際に必要とされる**ときにのみ評価されます。

```yaoxiang
# コンパイラは自動的に並作図を構築
fetch_user: spawn () -> User = (id) => { ... }
fetch_posts: spawn (User) -> Posts = (user) => { ... }

main:() -> Void = () => {
    user = fetch_user(1)     # ノード A (Async(User))
    posts = fetch_posts(user) # ノード B (Async(Posts))、A に依存

    # ノード C は A と B の結果を必要とする
    print(posts.title)       # 自動待機：A と B が完了することを保証
}
```

#### 5.1.2 並作値：Async(T)

`spawn fn` とマークされた関数呼び出しは、型 `Async(T)` 型の値を 즉시返します。これを**並作値**と呼びます。これは軽量プロキシであり、実際の結果ではなく、**並作中の未来値**を表します。

**コア機能**：
- **タイプ透過性**：`Async(T)` はタイプシステムでは `T` のサブタイプであり、`T` が期待される任意のコンテキストで使用可能
- **自動待機**：プログラムが `T` タイプの具体的な値を使用する必要がある操作を実行すると、ランタイムは現在のタスクを自動的に中断し、計算の完了を待つ
- **ゼロ伝染**：非同期コードと同期コードは構文と型署名に違いがない

```yaoxiang
# 並作値使用例
fetch_data: spawn (String) -> JSON = (url) => { ... }

main: () -> Void = () => {
    data = fetch_data("url")  # Async(JSON)

    # Async(JSON) は直接 JSON として使用可能
    # 自動待機はフィールドアクセス時に発生
    print(data.name)          # data.await().name と同等
}
```

### 5.2 並作構文体系

`spawn` キーワードは三重のセマンティクスを持ち、同期思考と非同期実装を接続する唯一のブリッジです：

| 公式用語 | 構文形式 | セマンティクス | ランタイム動作 |
|----------|----------|------|------------|
| **並作関数** | `spawn fn` | 並作実行に参加できる計算ユニットを定義 | その呼び出しは `Async(T)` を返す |
| **並作ブロック** | `spawn { a(), b() }` | 明示的に宣言された並行領域 | ブロック内のタスクは強制的に並行実行 |
| **並作ループ** | `spawn for x in xs { ... }` | データ並行パラダイム | ループ体がすべての要素に並作実行 |

#### 5.2.1 並作関数

```yaoxiang
# spawn を使用して並作関数をマーク
# 構文は通常の関数と完全に同じ、追加負担なし

fetch_api: spawn (String) -> JSON = (url) => {
    response = HTTP.get(url)
    JSON.parse(response.body)
}

# ネストされた並作呼び出し
process_user: (Int) -> Report = (user_id) => {
    user = fetch_user(user_id)     # Async(User)
    profile = fetch_profile(user)  # Async(Profile)、user に依存
    generate_report(user, profile) # profile に依存
}
```

#### 5.2.2 並作ブロック

```yaoxiang
# spawn { } - 明示的並列構築
# ブロック内のすべての式は独立したタスクとして並行実行

compute_all: (Int, Int) -> (Int, Int, Int) spawn = (a, b) => {
    # 3つの独立した計算が並行実行
    (x, y, z) = spawn {
        heavy_calc(a),        # タスク 1
        heavy_calc(b),        # タスク 2
        another_calc(a, b)    # タスク 3
    }
    (x, y, z)
}
```

#### 5.2.3 並作ループ

```yaoxiang
# spawn for - データ並行ループ
# 各反復は独立したタスクとして並行実行

parallel_sum: (Int) -> Int spawn = (n) => {
    total = spawn for i in 0..n {
        fibonacci(i)          # 各反復を並行
    }
    total
}
```

#### 5.2.4 データ並行ループ

```yaoxiang
# spawn for - データ並行ループ
# 各反復は独立したタスクとして並行実行

parallel_sum: (Int) -> Int spawn = (n) => {
    total = spawn for i in 0..n {
        fibonacci(i)          # 各反復を並行
    }
    total
}

# 行列乗算の並列化
matmul: spawn [[A: Matrix], [B: Matrix]] -> Matrix = (A, B) => {
    result = spawn for i in 0..A.rows {
        row = spawn for j in 0..B.cols {
            dot_product(A.row(i), B.col(j))
        }
        row
    }
    result
}
```

### 5.3 自動待機メカニズム

```yaoxiang
# 明示的な await 不要、コンパイラは自動的に待機点を挿入

main: () -> Void = () => {
    # 自動並列：2つの独立したリクエストが並行実行
    users = fetch_users()      # Async(List(User))
    posts = fetch_posts()      # Async(List(Post))

    # 待機点は「+」操作の位置に自動的に挿入
    count = users.length + posts.length

    # フィールドアクセスが待機をトリガー
    first_user = users[0]      # users が準備完了するのを待つ
    print(first_user.name)
}

# 条件分岐内の待機
process_data: spawn () -> Void = () => {
    data = fetch_data()        # Async(Data)

    if data.is_valid {         # data が準備完了するのを待つ
        process(data)
    } else {
        log("Invalid data")
    }
}
```

### 5.4 並行制御ツール

```yaoxiang
# すべてのタスクの完了を待つ
await_all: (T: Type) -> ((tasks: List(Async(T))) -> List(T)) = {
    # Barrier 待機
}

# いずれか1つが完了するのを待つ
await_any: (T: Type) -> ((tasks: List(Async(T))) -> T) = {
    # 最初の完了結果を返す
}

# タイムアウト制御
with_timeout: (T: Type) -> ((task: Async(T), timeout: Duration) -> Option(T)) = {
    # タイムアウト時は None を返す
}
```

### 5.5 スレッド安全：Send/Sync 制約

YaoXiang は Rust と同様の **Send/Sync タイプ制約**を採用し、コンパイル時にデータ競合を排除します。

#### 5.5.1 Send 制約

**Send**：タイプはスレッド間を安全に**所有権を移転**できます。

```yaoxiang
# 基本タイプは自動的に Send を満たす
# Int, Float, Bool, String はすべて Send

# 構造体は自動的に Send を派生
type Point = { x: Int, y: Float }
# Point は Send、Int と Float が Send だから

# 非 Send フィールドを含むタイプは Send ではない
type NonSend = NonSend(data: Rc(Int))
# Rc は Send ではない（参照カウントがアトミックでない）、したがって NonSend は Send ではない
```

#### 5.5.2 Sync 制約

**Sync**：タイプはスレッド間を安全に**参照を共有**できます。

```yaoxiang
# 基本タイプはすべて Sync
type Point = { x: Int, y: Float }
# &Point は Sync、&Int と &Float が Sync だから

# 内部可変性を含むタイプ
type Counter = Counter(value: Int, mutex: Mutex(Int))
# &Counter は Sync、Mutex が内部可変性を提供するから
```

#### 5.5.3 spawn とスレッド安全

```yaoxiang
# spawn はパラメータと戻り値が Send を満たすことを要求

# 有効：Data は Send
type Data = Data(value: Int)
task = spawn(|| => Data(42))

# 無効：Rc は Send ではない
type SharedData = SharedData(rc: Rc(Int))
# task = spawn(|| => SharedData(Rc.new(42)))  # コンパイルエラー！

# 解決策：Arc（アトミック参照カウント）を使用
type SafeData = SafeData(value: Arc(Int))
task = spawn(|| => SafeData(Arc.new(42)))  # Arc は Send + Sync
```

#### 5.5.4 スレッド安全タイプ派生ルール

```yaoxiang
# 構造体タイプ
type Struct(T1, T2) = Struct(f1: T1, f2: T2)

# Send 派生
Struct(T1, T2): Send ⇐ T1: Send かつ T2: Send

# Sync 派生
Struct(T1, T2): Sync ⇐ T1: Sync かつ T2: Sync

# 合併タイプ
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

# Send 派生
Result(T, E): Send ⇐ T: Send かつ E: Send
```

#### 5.5.5 標準ライブラリスレッド安全実装

| タイプ | Send | Sync | 説明 |
|------|:----:|:----:|------|
| `Int`, `Float`, `Bool` | ✅ | ✅ | 原タイプ |
| `Arc(T)` | ✅ | ✅ | T: Send + Sync |
| `Mutex(T)` | ✅ | ✅ | T: Send |
| `RwLock(T)` | ✅ | ✅ | T: Send |
| `Channel(T)` | ✅ | ❌ | 送信側のみ Send |
| `Rc(T)` | ❌ | ❌ | 非アトミック参照カウント |
| `RefCell(T)` | ❌ | ❌ | 実行時借用検査 |


```yaoxiang
# スレッド安全カウンタ例
type SafeCounter = SafeCounter(mutex: Mutex(Int))

main: () -> Void = () => {
    counter: Arc(SafeCounter) = Arc.new(SafeCounter(Mutex.new(0)))

    # 並行更新
    spawn(|| => {
        guard = counter.mutex.lock()  # Mutex がスレッド安全を提供
        guard.value = guard.value + 1
    })

    spawn(|| => {
        guard = counter.mutex.lock()
        guard.value = guard.value + 1
    })
}
```

### 5.6 阻塞操作

```yaoxiang
# @block アノテーションを使用して OS スレッドをブロックする操作をマーク
# ランタイムはこれらの操作を専用のブロックスレッドプールに割り当てる

@block
read_large_file: (path: String) -> String = {
    # この呼び出しはコアスケジューラをブロックしない
    file = File.open(path)
    content = file.read_all()
    content
}
```

---

## 六、モジュールシステム

### 6.1 モジュールの定義

```yaoxiang
# モジュールはファイルを使用して境界とする
# Math.yx ファイル
pub pi: Float = 3.14159
pub sqrt: (x: Float) -> Float = { ... }
```

### 6.2 モジュールのインポート

```yaoxiang
# モジュール全体をインポート
use std.io

# インポートして名前変更
use std.io as IO

# 具体的な関数をインポート
use std.io.{ read_file, write_file }
```

---

## 七、メソッドバインディングとカリー化

YaoXiang は**純粋関数型設計**を採用し、先进的なバインディングメカニズムを通じてシームレスなメソッド呼び出しとカリー化を実現し、`struct`、`class` などのキーワードを導入する必要がありません。

### 7.1 コア関数定義

すべての操作は通常の関数を通じて実装され、最初の引数は操作の主体と約束されています：

```yaoxiang
# === Point.yx (モジュール) ===

# 統一構文：コンストラクタ即タイプ
type Point = { x: Float, y: Float }

# コア関数：最初の引数は操作の主体
distance: (a: Point, b: Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    (dx * dx + dy * dy).sqrt()
}

add: (a: Point, b: Point) -> Point = {
    Point(a.x + b.x, a.y + b.y)
}

scale: (p: Point, s: Float) -> Point = {
    Point(p.x * s, p.y * s)
}

# より複雑な関数
distance_with_scale: (s: Float, p1: Point, p2: Point) -> Float = {
    dx = (p1.x - p2.x) * s
    dy = (p1.y - p2.y) * s
    (dx * dx + dy * dy).sqrt()
}
```

### 7.2 基本メソッドバインディング

#### 7.2.1 自動バインディング（MoonBit スタイル）

YaoXiang は名前空間に基づく自動バインディングをサポートし、**追加の宣言不要**です：

```yaoxiang
# === Point.yx ===

type Point = { x: Float, y: Float }

# コア関数
distance: (a: Point, b: Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    (dx * dx + dy * dy).sqrt()
}

# === main.yx ===

use Point

main: () -> Void = {
    p1 = Point(3.0, 4.0)
    p2 = Point(1.0, 2.0)

    # ✅ 自動バインディング：メソッド構文で直接呼び出し
    result = p1.distance(p2)  # distance(p1, p2) に解決
}
```

**自動バインディングルール**：
- モジュール内で定義された関数
- 最初の引数タイプがモジュール名と一致する場合
- メソッド呼び出し構文が自動的にサポートされる

#### 7.2.2 バインディングなしオプション（デフォルト動作）

```yaoxiang
# === Vector.yx ===

type Vector = Vector(x: Float, y: Float, z: Float)

# 内部ヘルパー関数、自動バインディング我不想
dot_product_internal: (a: Vector, b: Vector) -> Float = {
    a.x * b.x + a.y * b.y + a.z * b.z
}

# === main.yx ===

use Vector

main: () -> Void = {
    v1 = Vector(1.0, 0.0, 0.0)
    v2 = Vector(0.0, 1.0, 0.0)

    # ❌ バインディング不可：非 pub 関数は自動バインディングされない
    # v1.dot_product_internal(v2)  # コンパイルエラー！

    # ✅ 直接呼び出しが必要（モジュール外からは見えない）
}
```

### 7.3 位置に基づくバインディング構文

YaoXiang は**最もエレガントなバインディング構文**を提供し、位置マーク `[n]` を使用してバインディング位置を精密に制御します：

#### 7.3.1 基本構文

```yaoxiang
# === Point.yx ===

type Point = { x: Float, y: Float }

# コア関数
distance: (a: Point, b: Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    (dx * dx + dy * dy).sqrt()
}
add: (a: Point, b: Point) -> Point = {
    Point(a.x + b.x, a.y + b.y)
}
scale: (p: Point, s: Float) -> Point = {
    Point(p.x * s, p.y * s)
}

# バインディング構文：Type.method = func[position]
# 意味：メソッド呼び出し時、呼び出し元を func の [position] パラメータにバインディング

Point.distance = distance[0]      # 第1パラメータにバインディング
Point.add = add[0]                 # 第1パラメータにバインディング
Point.scale = scale[0]             # 第1パラメータにバインディング
```

**セマンティクス解析**：
- `Point.distance = distance[0]`
  - `distance` 関数は2つのパラメータを持つ：`distance(Point, Point)`
  - `[0]` は呼び出し元が第1パラメータにバインディングされることを意味する
  - 使用：`p1.distance(p2)` → `distance(p1, p2)`

#### 7.3.2 複数位置結合バインディング

```yaoxiang
# === Math.yx ===

# 関数：scale, point1, point2, extra1, extra2
calculate: (scale: Float, a: Point, b: Point, x: Float, y: Float) -> Float = { ... }

# === Point.yx ===

type Point = { x: Float, y: Float }

# 複数位置のバインディング
Point.calc1 = calculate[1, 2]      # scale と point1 をバインディング
Point.calc2 = calculate[1, 3]      # scale と point2 をバインディング
Point.calc3 = calculate[2, 3]      # point1 と point2 をバインディング

# === main.yx ===

use Point

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

# 1. バインディング[1,2] - 残り3,4,5
f1 = p1.calc1(2.0)  # scale=2.0, point1=p1 をバインディング
# f1 は現在 p2, x, y を必要とする
result1 = f1(p2, 10.0, 20.0)  # calculate(2.0, p1, p2, 10.0, 20.0)

# 2. バインディング[1,3] - 残り2,4,5
f2 = p2.calc2(2.0)  # scale=2.0, point2=p2 をバインディング
# f2 は現在 point1, x, y を必要とする
result2 = f2(p1, 10.0, 20.0)  # calculate(2.0, p1, p2, 10.0, 20.0)

# 3. バインディング[2,3] - 残り1,4,5
f3 = p1.calc3(p2)  # point1=p1, point2=p2 をバインディング
# f3 は現在 scale, x, y を必要とする
result3 = f3(2.0, 10.0, 20.0)  # calculate(2.0, p1, p2, 10.0, 20.0)
```

#### 7.3.3 残りパラメータの埋め込み順序

**コアルール**：バインディング後、残りパラメータは**元の関数の順序**で埋め込まれ、バインディングされた位置はスキップされます。

```yaoxiang
# 関数 suppose: func(p1, p2, p3, p4, p5)

# 第1と第3パラメータをバインディング
Type.method = func[1, 3]

# 呼び出し時：
method(p2_value, p4_value, p5_value)

# マッピング：
func(p1_bound, p2_value, p3_bound, p4_value, p5_value)
# 残りパラメータ：2,4,5 は元の順序で埋め込まれる
```

#### 7.3.4 タイプ検査の優位性

```yaoxiang
# ✅ 合法なバインディング
Point.distance = distance[0]          # distance(Point, Point)
Point.calc = calculate[1, 2]          # calculate(scale, Point, Point, ...)

# ❌ 違法なバインディング（コンパイラがエラーを報告）
Point.wrong = distance[5]             # 第5パラメータが存在しない
Point.wrong = distance[0]             # パラメータは1から始まる
Point.wrong = distance[1, 2, 3, 4]    # 関数パラメータ数をオーバー
```

### 7.4 カリー化バインディングの細粒度制御

```yaoxiang
# === Math.yx ===

distance_with_scale: (scale: Float, a: Point, b: Point) -> Float = { ... }

# === Point.yx ===

type Point = { x: Float, y: Float }

# バインディング戦略：各位置を柔軟に制御
Point.distance = distance[0]                    # 基本バインディング
Point.distance_scaled = distance_with_scale[2]  # 第2パラメータにバインディング

# === main.yx ===

use Point
use Math

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

# 1. 基本自動バインディング
d1 = p1.distance(p2)  # distance(p1, p2)

# 2. 異なる位置へのバインディング
f = p1.distance_scaled(2.0)  # 第2パラメータをバインディング、残りは第1,3
result = f(p2)               # distance_with_scale(2.0, p1, p2)

# 3. チェーンバインディング
d2 = p1.distance(p2).distance_scaled(2.0)  # チェーン呼び出し
```

### 7.5 完全なバインディングシステム

```yaoxiang
# === Point.yx ===

type Point = { x: Float, y: Float }

# コア関数
distance: (a: Point, b: Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    (dx * dx + dy * dy).sqrt()
}
add: (a: Point, b: Point) -> Point = {
    Point(a.x + b.x, a.y + b.y)
}
scale: (p: Point, s: Float) -> Point = {
    Point(p.x * s, p.y * s)
}

# 自動バインディング（コア）
Point.distance = distance[0]
Point.add = add[0]
Point.scale = scale[0]

# === Math.yx ===

# グローバル関数
multiply_by_scale: (scale: Float, a: Point, b: Point) -> Float = { ... }

# === main.yx ===

use Point
use Math

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

# 使用
d = p1.distance(p2)          # distance(p1, p2)
r = p1.add(p2)               # add(p1, p2)
s = p1.scale(2.0)            # scale(p1, 2.0)

# グローバル関数バインディング
Point.multiply = multiply_by_scale[2]  # 第2パラメータにバインディング
m = p1.multiply(2.0, p2)     # multiply_by_scale(2.0, p1, p2)
```

### 7.6 バインディングのスコープとルール

#### 7.6.1 pub の作用

```yaoxiang
# === Point.yx ===

type Point = { x: Float, y: Float }

# 非 pub 関数
internal_distance: (a: Point, b: Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    (dx * dx + dy * dy).sqrt()
}

# pub 関数
pub distance: (a: Point, b: Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    (dx * dx + dy * dy).sqrt()
}

# === main.yx ===

use Point

# 自動バインディングは pub 関数に対してのみ有効
p1.distance(p2)      # ✅ distance は pub、自動バインディング可能
# p1.internal_distance(p2)  # ❌ pub ではない、バインディング不可
```

#### 7.6.2 pub 自動バインディングメカニズム

`pub` で宣言された関数は、コンパイラが同じファイルで定義されたタイプに自動的にバインディングします：

```yaoxiang
# === Point.yx ===

type Point = { x: Float, y: Float }

# pub を使用して宣言、コンパイラが自動的にバインディング
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

pub translate: (self: Point, dx: Float, dy: Float) -> Point = {
    Point(self.x + dx, self.y + dy)
}

# コンパイラが自動的に推断してバインディングを実行：
# Point.distance = distance[0]
# Point.translate = translate[0]

# === main.yx ===

use Point

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

# ✅ 関数型呼び出し
d = distance(p1, p2)

# ✅ OOP シンタックスシュガー（自動バインディング）
d2 = p1.distance(p2)
p3 = p1.translate(1.0, 1.0)
```

**自動バインディングルール**：
1. 関数はモジュールファイルで定義（タイプと同じファイル）
2. 関数のパラメータがそのタイプを含む
3. `pub` を使用してエクスポート
4. コンパイラが自動的に `Type.method = function[0]` を実行

**好处**：
- バインディング宣言を手動で書く必要がない
- コードがより簡潔
- バインディングの見落としまたは誤りを回避

#### 7.6.3 モジュール内バインディング

```yaoxiang
# === Point.yx ===

type Point = { x: Float, y: Float }

distance: (a: Point, b: Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    (dx * dx + dy * dy).sqrt()
}

# モジュール内部では、すべての関数が見える
# しかし自動バインディングは pub でエクスポートされた関数のみが外部で有効

pub distance  # エクスポート、外面で自動バインディング可能
```

### 7.7 設計優位性のまとめ

| 機能 | 説明 |
|------|------|
| **ゼロ構文負担** | 自動バインディングに宣言不要 |
| **位置の精密制御** | `[n]` でバインディング位置を精密指定 |
| **複数位置結合** | `[1, 2, 3]` 複数パラメータバインディングをサポート |
| **タイプ安全** | コンパイラがバインディング位置の有効性を検証 |
| **キーワード不要** | `bind` または他のキーワード不要 |
| **柔軟なカリー化** | 任意位置パラメータバインディングをサポート |
| **pub 制御** | pub 関数の外面バインディングのみ有効 |

### 7.8 従来のメソッドバインディングとの違い

| 従来の言語 | YaoXiang |
|---------|----------|
| `obj.method(arg)` | `obj.method(arg)` |
| class/メソッド定義が必要 | 関数 + バインディング宣言のみが必要 |
| 構文 `class { method() {} }` | 構文 `Type.method = func[n]` |
| 継承、多態 | 純粋関数型、継承なし |
| メソッドテーブルルックアップ | コンパイル時バインディング、実行時オーバーヘッドなし |

**コア優位性**：YaoXiang のバインディングは**コンパイル時メカニズム**であり、ゼロ実行時コストであり、関数型プログラミングの純粋さと柔軟性を維持しています。

---

## 八、AI フレンドリ設計

YaoXiang の構文設計は AI コード生成と変更のニーズを特に考慮しています：

### 8.1 設計原則

```yaoxiang
# AI フレンドリ設計目標：
# 1. 厳密な構造化、あいまいさのない構文
# 2. AST が明確、位置決めが容易
# 3. セマンティクスが明確、隠れた動作がない
# 4. コードブロック境界が明確
# 5. タイプ情報が完全
```

### 8.2 厳密な構造化構文

#### 8.2.1 宣言構文の AI フレンドリ戦略

```yaoxiang
# === AI コード生成ベストプラクティス ===

# ✅ 推奨：完全な新構文宣言 + タイプ集中約束を使用
# AI は意図を正確に理解し、完全なタイプ情報を生成可能

add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1
empty: () -> Void = {}

# ❌ 避ける：タイプ标注を省略またはタイプが分散
# AI はパラメータタイプを確定できず、誤ったコードを生成する可能性がある
add: (a: Int, b: Int) -> Int = a + b          # パラメータにタイプなし
identity: (T: Type) -> ((x: T) -> T) = x              # パラメータにタイプなし
add2: (a: Int, b: Int) -> Int = a + b  # タイプが実装に分散

# ⚠️ 互換：旧構文は保守のみに使用
# AI は新構文 + タイプ集中約束を優先的に生成すべき
mul(Int, Int) -> Int = (a, b) => a * b  # 新コードでは推奨しない
```

**タイプ集中約束の AI 優位性：**

1. **シグネチャが一目でわかる**：AI は宣言行を読むだけで完全な関数シグネチャを理解できる
2. **より安全な変更**：タイプの変更は宣言を変更するだけでよく、実装体に影響しない
3. **生成がより簡単**：AI はまず宣言を生成し、それから実装を記入できる
4. **カリー化フレンドリ**：明確なカリー化型署名により AI が処理しやすい

```yaoxiang
# AI 処理例
# 入力：実装体 (a, b) => a + b
# AI が宣言を見る：add: (Int, Int) -> Int
# 結論：パラメータタイプは Int, Int、戻り値タイプは Int

# 比較：タイプが分散している場合
# 入力：実装体 (a: Int, b: Int) => a + b
# AI がする必要がある：実装体を分析してタイプ情報を抽出
# 結果：より複雑な処理ロジック、エラーが発生しやすい
```

#### 8.2.2 二重構文戦略と AI

| 構文タイプ | AI 生成戦略 | 使用シナリオ |
|---------|-----------|---------| 
| **新構文** | ✅ 優先生成、完全なタイプ情報 | すべての新コード生成 |
| **旧構文** | ⚠️ 旧コード保守時にのみ使用 | 歴史的コードの変更 |
| **無标注** | ❌ 生成を避ける | 任何情况下都不应生成 |

#### 8.2.3 構文境界が明確

```yaoxiang
# AI フレンドリなコードブロック境界

# ✅ 明確な開始と終了マーク
function_name: (Type1, Type2) -> ReturnType = (param1, param2) => {
    # 関数体
    if condition {
        do_something()
    } else {
        do_other()
    }
}

# ✅ 条件文には必ず波括弧が必要
if condition {
    # 条件体
}

# ✅ タイプ定義が明確
type MyType = Type1 | Type2

# ❌ 避けるべき曖昧な写法
if condition    # 波括弧がない
    do_something()
```

#### 8.2.4 あいまいさのない構文制約

```yaoxiang
# AI 生成時に遵守する必要がある制約

# 1. 括弧の省略を禁止
# ✅ 正しい
foo: (T: Type) -> ((x: T) -> T) = x
my_list = [1, 2, 3]

# ❌ エラー（禁止）
foo T { T }             # パラメータには括弧が必要
my_list = [1 2 3]       # リストにはカンマが必要

# 2. 戻り値タイプまたは推断可能な形式を明示する必要がある
# ✅ 正しい
get_num: () -> Int = 42
get_num2: () = 42          # 戻り値タイプが推断可能

# ❌ エラー
get_bad = () => { 42 }           # 块に return がなく、推断できない

# 3. パラメータには（新構文で）タイプ标注が必要
# ✅ 正しい
add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1

# ❌ エラー
add: (a: Int, b: Int) -> Int = a + b            # パラメータにタイプなし
identity: (T: Type) -> ((x: T) -> T) = x                # パラメータにタイプなし
```

#### 8.2.5 AI 生成推奨パターン

```yaoxiang
# AI が関数を生成する際の標準テンプレート

# パターン1：完全なタイプ标注
function_name: (param1: ParamType1, param2: ParamType2, ...) -> ReturnType = {
    # 関数体
    return expression
}

# パターン2：戻り値タイプの推断
function_name: (param1: ParamType1, param2: ParamType2) = {
    # 関数体
    return expression
}

# パターン3：単一パラメータ省略
function_name: (param: ParamType) -> ReturnType = expression

# パターン4：パラメータなし関数
function_name: () -> ReturnType = expression

# パターン5：空関数
function_name: () -> Void = {}
```

### 8.3 エラーメッセージの AI フレンドリ性

```yaoxiang
# エラーメッセージは明確な修正提案を提供するべき

# フレンドリでないエラー
# Syntax error at token 'a'

# AI フレンドリなエラー
# Missing type annotation for parameter 'a'
# Suggestion: add ': Int' or similar type to '(a, b) => a + b'
# Correct version: add: (a: Int, b: Int) -> Int = a + b
```

---

## 九、タイプ集中約束（コア設計哲学）

### 9.1 約束概述

YaoXiang のコア設計約束は**「宣言優先、タイプ集中」**です。この約束は言語の AI フレンドリ性と開発効率の基礎です。

```yaoxiang
# ✅ コア約束：タイプ情報が宣言行に統一
add: (a: Int, b: Int) -> Int = a + b

# ❌ 避ける：タイプ情報が実装に分散
add: (a: Int, b: Int) -> Int = a + b
```

### 9.2 約束の5つのコア優位性

#### 1. 構文一貫性
```yaoxiang
# すべての宣言が同じ形式に従う
x: Int = 42                           # 変数
name: String = "YaoXiang"             # 変数
add: (a: Int, b: Int) -> Int = a + b  # 関数
inc: (x: Int) -> Int = x + 1          # 関数
type Point = { x: Float, y: Float } # タイプ
```

#### 2. 宣言と実装の分離
```yaoxiang
# 宣言行が完全なタイプ情報を提供
add: (a: Int, b: Int) -> Int = a + b
# └────────────────────┘
#   完全な関数シグネチャ

# 実装体はビジネスロジックに集中
# (a, b) => a + b  タイプは気にせず、功能のみを実装
```

#### 3. AI フレンドリ性
```yaoxiang
# AI 処理フロー：
# 1. 宣言行を読む → 完全に関数シグネチャを理解
# 2. 実装を生成 → タイプ分析不要
# 3. タイプを変更 → 宣言行のみを変更、実装体に影響なし

# 比較：タイプが分散している場合
add: (a: Int, b: Int) -> Int = a + b
# AI がする必要がある：実装体を分析してタイプ情報を抽出 → より複雑、エラーが発生しやすい
```

#### 4. より安全な変更
```yaoxiang
# パラメータタイプを変更
# 原来: add: (a: Int, b: Int) -> Int = a + b
# 修改: add: (Float, Float) -> Float = (a, b) => a + b
# 実装体: (a, b) => a + b  変更不要！

# タイプが分散している場合：
# 原来: add: (a: Int, b: Int) -> Int = a + b
# 修改: add: (a: Float, b: Float) -> Float = a + b  # 2箇所変更が必要
```

#### 5. カリー化フレンドリ
```yaoxiang
# カリー化タイプが一目でわかる
add_curried: (a: Int) -> (b: Int) -> Int = a + b
#              └─────────────┘
#              カリー化シグネチャ

# 関数合成が第一級市民
compose: (Int -> Int, Int -> Int) -> Int -> Int = (f, g) => x => f(g(x))
```

### 9.3 約束の実施ルール

#### ルール1：パラメータは宣言でタイプを指定する必要がある
```yaoxiang
# ✅ 正しい
add: (a: Int, b: Int) -> Int = a + b

# ❌ エラー
add: (a: Int, b: Int) -> Int = a + b            # パラメータタイプが欠落
identity: (T: Type) -> ((x: T) -> T) = x                # パラメータタイプが欠落
```

#### ルール2：戻り値タイプは推断可能だが标注を推奨
```yaoxiang
# ✅ 推奨：完全标注
get_num: () -> Int = () => 42

# ✅ 許容可能：戻り値タイプが推断
get_num: () = () => 42

# ✅ 空関数は Void に推断
empty: () = () => {}
```

#### ルール3：Lambda 内部のタイプ注釈は一時のもの
```yaoxiang
# ✅ 正しい：宣言のタイプに依存
add: (a: Int, b: Int) -> Int = a + b

# ⚠️ 可能だが推奨しない：Lambda 内で繰り返し标注
add: (Int, Int) -> Int = (a: Int, b: Int) => a + b

# ❌ エラー：宣言标注がない
add: (a: Int, b: Int) -> Int = a + b
```

#### ルール4：旧構文は同じ理念に従う
```yaoxiang
# 旧構文も宣言位置でタイプ情報を提供するように努めるべき
# 形式は異なるが、理念は一貫している：
# - 宣言行が主なタイプ情報を含む
# - 実装体は比較的簡潔
add(Int, Int) -> Int = (a, b) => a + b
```

### 9.4 約束とタイプ推断の関係

```yaoxiang
# 約束はタイプ推断を妨げず、推断方向を案内する

# 1. 完全标注（推断なし）
add: (a: Int, b: Int) -> Int = a + b

# 2. 部分推断（宣言がパラメータタイプを提供）
add: (Int, Int) = (a, b) => a + b  # 戻り値タイプが推断

# 3. 空関数の推断
empty: () = () => {}  # () -> Void に推断
```

### 9.5 約束の AI 実装優位性

**AI コード生成フロー：**

1. **ニーズを読む** → 宣言を生成
   ```
   ニーズ：加算関数
   生成：add: (Int, Int) -> Int = (a, b) => ???
   ```

2. **実装を記入** → タイプ分析不要
   ```
   実装：add: (a: Int, b: Int) -> Int = a + b
   ```

3. **タイプを変更** → 宣言のみを変更
   ```
   変更：add: (Float, Float) -> Float = (a, b) => a + b
   実装：(a, b) => a + b  そのまま
   ```

**約束のない AI 処理との比較：**
```
ニーズ：加算関数
AI がする必要がある：
  1. パラメータタイプを推断
  2. 戻り値タイプを推断
  3. 実装体を生成
  4. 一貫性を検証
  5. タイプ変更時の複雑な更新を処理

結果：より複雑、よりエラーが発生しやすい
```

### 9.6 約束の哲学的意味

この約束は YaoXiang の核心理念を体現しています：

- **宣言即ドキュメント**：宣言行は完全な関数ドキュメントである
- **タイプ即契約**：タイプ情報は呼び出し側と実装者の間の契約である
- **ロジック即実装**：実装体は「何をするか」のみに集中し、「何タイプか」は気にしない
- **ツール即支援**：タイプシステム、AI ツールは明確な宣言に基づいて動作できる

### 9.7 実際の適用比較

#### 完全な例：計算機モジュール

```yaoxiang
# === 推奨做法：タイプ集中約束 ===

# モジュール宣言
pub add: (a: Int, b: Int) -> Int = a + b
pub multiply: (a: Int, b: Int) -> Int = a * b

# 高階関数
pub apply_twice: (f: Int -> Int, x: Int) -> Int = f(f(x))

# カリー化関数
pub make_adder: (x: Int) -> (Int) -> Int = y => x + y

# ジェネリック関数
pub map: (A: Type, B: Type) -> ((f: (A) -> B, xs: List(A)) -> List(B)) = case xs of
  [] => []
  (x :: rest) => f(x) :: map(f, rest)

# タイプ定義
type Point = { x: Float, y: Float }
pub distance: (a: Point, b: Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    (dx * dx + dy * dy).sqrt()
}

# === 推奨しない做法：タイプが分散 ===

# パラメータタイプが Lambda 内にある
add: (a: Int, b: Int) -> Int = a + b
multiply = (a: Int, b: Int) => a * b

# 高階関数のタイプが分散
apply_twice = (f: (Int) -> Int, x: Int) => f(f(x))

# カリー化のタイプが分散
make_adder = (x: Int) => (y: Int) => x + y

# ジェネリックタイプが分散
map: (A: Type, B: Type) -> ((f: (A) -> B, xs: List(A)) -> List(B)) = case xs of
  [] => []
  (x :: rest) => f(x) :: map(f, rest)
```

#### コード保守の比較

```yaoxiang
# ニーズ：add を Int から Float に変更

# === 推奨做法：宣言行のみを変更 ===
# 原来
add: (a: Int, b: Int) -> Int = a + b

# 変更後
add: (a: Float, b: Float) -> Float = a + b
#              ↑↑↑↑↑↑↑↑↑          ↑↑↑↑↑↑↑
#              宣言行を変更          実装体はそのまま

# === 推奨しない做法：複数箇所を変更 ===
# 原来
add: (a: Int, b: Int) -> Int = a + b

# 変更後
add: (a: Float, b: Float) -> Float = a + b
#     ↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑
#     すべてのパラメータタイプを変更する必要がある
```

#### AI 支援プログラミングの比較

```yaoxiang
# AI ニーズ：2点間のマンハッタン距離を計算する関数を実装

# === AI が推奨写法を見た場合 ===
type Point = { x: Float, y: Float }
pub manhattan: (a: Point, b: Point) -> Float = ???  # AI は完全なシグネチャを直接知っている

# AI が生成：
pub manhattan: (a: Point, b: Point) -> Float = {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}

# === AI が推奨しない写法を見た場合 ===
type Point = { x: Float, y: Float }
pub manhattan = ???  # AI が推断する必要がある：パラメータタイプ？戻り値タイプ？

# AI が生成する可能性：
pub manhattan = (a: Point, b: Point) => Float => {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}
# またはタイプ情報が不完全なためエラーが発生する可能性がある
```

### 9.8 約束実施チェックリスト

YaoXiang コードを编写する際に、以下のチェックリストを使用できます：

- [ ] すべての関数宣言に宣言行に完全なタイプ标注がある
- [ ] パラメータタイプは宣言で指定、Lambda 内ではない
- [ ] 戻り値タイプはできるだけ宣言で标注
- [ ] 変数宣言は `name: Type = value` 形式を使用
- [ ] Lambda 本体は簡潔に保ち、タイプの繰り返し情報を含めない
- [ ] 新構文を使用、旧構文ではない
- [ ] 複雑なタイプは type を使用して定義し、宣言を明確に保つ

---

## 十、クイックスタート

### 10.1 Hello World

```yaoxiang
# hello.yx
use std.io

main: () -> Void = {
    println("Hello, YaoXiang!")
}
```

実行方法：`yaoxiang hello.yx`

出力：
```
Hello, YaoXiang!
```

### 10.2 基本構文

```yaoxiang
# 変数とタイプ
x = 42                    # 自動的に Int に推断
name = "YaoXiang"         # 自動的に String に推断
pi = 3.14159              # 自動的に Float に推断

# 関数（新構文を使用）
add: (a: Int, b: Int) -> Int = a + b

# 条件
if x > 0 {
    "positive"
} elif x == 0 {
    "zero"
} else {
    "negative"
}

# ループ
for i in 0..10 {
    print(i)
}
```

### 10.3 メソッドバインディング例

```yaoxiang
# === Point.yx ===

type Point = { x: Float, y: Float }

# コア関数
distance: (a: Point, b: Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    (dx * dx + dy * dy).sqrt()
}

# 自動バインディング
Point.distance = distance[0]

# === main.yx ===

use Point

main: () -> Void = {
    p1 = Point(3.0, 4.0)
    p2 = Point(1.0, 2.0)

    # バインディングを使用
    d = p1.distance(p2)  # distance(p1, p2)
    print(d)
}
```

### 10.4 カリー化バインディング例

```yaoxiang
# === Math.yx ===

distance_with_scale: (scale: Float, a: Point, b: Point) -> Float = {
    dx = (p1.x - p2.x) * scale
    dy = (p1.y - p2.y) * scale
    (dx * dx + dy * dy).sqrt()
}

# === Point.yx ===

type Point = { x: Float, y: Float }

Point.distance_scaled = distance_with_scale[2]  # 第2パラメータにバインディング

# === main.yx ===

use Point

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

# バインディングを使用
f = p1.distance_scaled(2.0)  # scale と p1 をバインディング
result = f(p2)               # 最終呼び出し

# または直接使用
result2 = p1.distance_scaled(2.0, p2)
```

### 10.5 次のステップ

- 完全な構文については [言語仕様](./YaoXiang-language-specification.md) を参照
- 一般的なパターンについては [サンプルコード](./examples/) を参照
- 技術的詳細については [実装計画](./YaoXiang-implementation.md) を参照

---

## 付録

### A. キーワードとアノテーション

| キーワード | 作用 |
|----------|------|
| `type` | タイプ定義 |
| `pub` | パブリックエクスポート |
| `use` | モジュールのインポート |
| `spawn` | 非同期マーク（関数/ブロック/ループ） |
| `ref` | 不変参照 |
| `mut` | 可変参照 |
| `if/elif/else` | 条件分岐 |
| `match` | パターン照合 |
| `while/for` | ループ |
| `return/break/continue` | 制御フロー |
| `as` | タイプ変換 |
| `in` | メンバーアクセス |

| アノテーション | 作用 |
|------|------|
| `@block` | 完全同期コードとしてマーク |
| `@eager` | 急切評価が必要な式としてマーク |
| `@Send` | Send 制約を明示的に宣言 |
| `@Sync` | Sync 制約を明示的に宣言 |

### B. 設計インスピレーション

- **Rust**：所有権モデル、ゼロコスト抽象化
- **Python**：構文スタイル、読みやすさ
- **Idris/Agda**：依存タイプ、タイプ駆動開発
- **TypeScript**：タイプ注釈、実行時タイプ

---

## バージョン履歴

| バージョン | 日付 | 著者 | 変更説明 |
|------|------|------|---------| 
| v1.0.0 | 2024-12-31 | 晨煦 | 初期バージョン |
| v1.1.0 | 2025-01-04 | 沫郁酱 | ジェネリック構文を `<T>` から `[T]` に修正（）；`fn` キーワードを削除；関数定義例を更新 |
| v1.2.0 | 2025-01-06 | 晨煦 | 新構文形式に統一：name: type -> type = lambda |
| v1.3.0 | 2025-01-20 | 晨煦 | 統一タイプ構文を追加（RFC-010）：インターフェース定義は波括弧 `{ serialize: () -> String }` を使用；タイプ末尾にインターフェース名を列出してインターフェースを実装；`pub` 自動バインディングメカニズム |

---

> 「道生一，一生二，二生三，三生万物。」
> —— 《道德経》
>
> タイプは道であり、万物はここから生まれる。
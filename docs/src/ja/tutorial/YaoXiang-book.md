# YaoXiang（爻象）プログラミング言語ガイド

> バージョン：v1.2.0
> 状態：草稿
> 著者：晨煦
> 日付：2024-12-31
> 更新：2025-01-20 - 位置インデックスは0から開始（RFC-004）；統一型構文（RFC-010）

---

## 目次

1. [言語概述](#一言語概述)
2. [コア機能](#二コア機能)
3. [型システム](#三型システム)
4. [メモリ管理](#四メモリ管理)
5. [非同期プログラミングと並行処理](#五非同期プログラミングと並行処理)
6. [モジュールシステム](#六モジュールシステム)
7. [メソッドバインディングとカリー化](#七メソッドバインディングとカリー化)
8. [AIフレンドリー設計](#八aiフレンドリー設計)
9. [型集中約束（コア設計哲学）](#九型集中約束コア設計哲学)
10. [クイックスタート](#十クイックスタート)

---

**拡張ドキュメント**：
- [高度なバインディング機能とコンパイラ実装](../works/plans/bind/YaoXiang-bind-advanced.md) - 深入したバインディングメカニズム、高度な機能、コンパイラ実装、エッジケース処理

---

## 一、言語概述

### 1.1 YaoXiangとは？

YaoXiang（爻象）は『易経』における「爻」と「象」という核心概念に着想を得た実験的な汎用プログラミング言語である。「爻」は卦を構成する基本記号であり、陰陽の変化を象徴する。「象」は事物の本質の外在的表現であり万象万物，代表する。

YaoXiangはこの哲学的思考をプログラミング言語の型システムに溶け込ませ、**「一切皆型」**という核心的理念を提唱する。YaoXiangの世界観において：

- **値**は型のインスタンスである
- **型**自体も型のインスタンスである（メタ型）
- **関数**は入力型から出力型への写像である
- **モジュール**は型の名前空間組み合わせである

### 1.2 設計目標

YaoXiangの設計目標は以下に概括できる：

| 目標 | 説明 |
|------|------|
| **統一された型の抽象化** | 型は最上層の抽象ユニットであり、言語の意味論を簡素化する |
| **自然なプログラミング体験** | Python風の構文、読みやすさを重視 |
| **安全なメモリ管理** | Rust風の所有権モデル、GCなし |
| **無感の非同期プログラミング** | 非同期を自動管理、明示的なawait不要 |
| **完全な型リフレクション** | 実行時の型情報が完全に利用可能 |
| **AIフレンドリーな構文** | 厳密に構造化され、AIが処理しやすい |

### 1.3 言語の位置づけ

| 次元 | 位置づけ |
|------|----------|
| パラダイム | マルチパラダイム（関数型 + 命令型 + オブジェクト指向） |
| 型システム | 依存型 + パラメータ多態 |
| メモリ管理 | 所有権 + RAII（GCなし） |
| コンパイルモデル | AOTコンパイル（オプションでJIT） |
| ターゲットシナリオ | システムプログラミング、アプリケーション開発、AI支援プログラミング |

### 1.4 コード例

```yaoxiang
# 自動型推論
x: Int = 42                           # 明示的な型
y = 42                                # Intと推論
name = "YaoXiang"                     # Stringと推論

# デフォルトで不変
x: Int = 10
x = 20                                # ❌ コンパイルエラー！不変

# 統一宣言構文：識別子: 型 = 式
add: (a: Int, b: Int) -> Int = a + b  # 関数宣言
inc: (x: Int) -> Int = x + 1               # 単一パラメータ関数

# 統一型構文：コンストラクタは型
type Point = { x: Float, y: Float }
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

# 無感非同期（並作関数）
fetch_data: (url: String) -> JSON spawn = {
    HTTP.get(url).json()
}

main: () -> Void = {
    # 値構築：関数呼び出しと完全に同じ
    p = Point(3.0, 4.0)
    r = ok("success")

    data = fetch_data("https://api.example.com")
    # 自動待機、await不要
    print(data.name)
}

# ジェネリクス関数
identity: (T: Type) -> ((x: T) -> T) = x

# 高階関数
apply: (f: (Int) -> Int, x: Int) -> Int = f(x)

# カリー化
add_curried: (a: Int) -> (b: Int) -> Int = a + b
```

---

## 二、コア機能

### 2.1 一切皆型

YaoXiangのコア設計哲学は**一切皆型**である。これはYaoXiangにおいて：

1. **値は型のインスタンス**：`42`は`Int`型のインスタンスである
2. **型は型のインスタンス**：`Int`は`type`メタ型のインスタンスである
3. **関数は型の写像**：`add: (Int, Int) -> Int`は関数型である
4. **モジュールは型の組み合わせ**：モジュールは関数と型を含む名前空間である

```yaoxiang
# 値は型のインスタンス
x: Int = 42

# 型は型のインスタンス
MyList: type = List(Int)

# 関数は型間の写像
add: (a: Int, b: Int) -> Int = a + b

# モジュールは型の組み合わせ（ファイルをモジュールとして使用）
# Math.yx
pi: Float = 3.14159
sqrt: (x: Float) -> Float = { ... }
```

### 2.2 数学的抽象化

YaoXiangの型システムは型理論と圏論に基づいており、以下のを提供する：

- **依存型**：型は値に依存できる
- **ジェネリクスプログラミング**：型パラメータ化
- **型の組み合わせ**：合併型、交差型

```yaoxiang
# 依存型：固定長ベクトル
Vector: (T: Type, n: Int) -> Type = vector(T, n)

# 型合併
type Number = Int | Float

# 型交差
type Printable = printable(fn() -> String)
type Serializable = serializable(fn() -> String)
type Versatile = Printable & Serializable
```

### 2.3 ゼロコスト抽象化

YaoXiangはゼロコスト抽象化を保証する。高レベルの抽象化は実行時のパフォーマンスオーバーヘッドをもたらさない：

- **単態化**：ジェネリクス関数はコンパイル時に具体的なバージョンに展開される
- **インライン最適化**：単純な関数は自動的にインライン展開される
- **スタック確保**：小さなオブジェクトはデフォルトでスタックに確保される

```yaoxiang
# ジェネリクス展開（単態化）
identity: (T: Type) -> ((x: T) -> T) = x

# 使用
int_val = identity(42)      # identity(Int) -> Intに展開
str_val = identity("hello") # identity(String) -> Stringに展開

# コンパイル後、追加オーバーヘッドなし
```

### 2.4 自然構文

YaoXiangはPython風の構文を採用し、読みやすさと自然言語感を追求する：

```yaoxiang
# 自動型推論
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

YaoXiangは統一された宣言構文：**識別子: 型 = 式**を採用する。下位互換性のある古い構文も提供する。

#### 2.5.1 二重構文戦略と型集中約束

革新と互換性のバランスを取るため、YaoXiangは2つの構文形式をサポート하지만、統一された**型集中标注約束**を採用する。

**構文形式比較：**

| 構文タイプ | 形式 | 状態 | 説明 |
|-----------|------|------|------|
| **新構文（標準）** | `name: Type = Lambda` | ✅ 推奨 | 公式標準、新規コードはすべてこの形式を使用すべき |
| **旧構文（互換）** | `name(Types) -> Ret = Lambda` | ⚠️ 互換のみ | 履歴コード向け推奨、新規プロジェクトでは使用禁止 |

**核心約束：型集中标注**

YaoXiangは**「宣言優先、型集中」**の設計約束を採用する：

```yaoxiang
# ✅ 正しい：型情報は宣言行に統一
add: (a: Int, b: Int) -> Int = a + b
#   └─────────────────┘   └─────────────┘
#       完全な型署名         実装ロジック

# ❌ 避ける：型情報が実装に分散
add: (a: Int, b: Int) -> Int = a + b
#     └───────────────┘
#     型が実装本体に混在
```

**約束好处：**

1. **構文一貫性**：すべての宣言が`識別子: 型 = 式`に従う
2. **宣言と実装の分離**：型情報が明確で、実装本体はロジックに集中
3. **AIフレンドリー性**：AIは宣言行を読むだけで完全な関数署名を理解できる
4. **より安全な修正**：型の修正は宣言を変更するだけで、実装本体に影響なし
5. **カリー化フレンドリー**：明確なカリー化型署名をサポート

**選択提案**：
- **新規プロジェクト**：新構文 + 型集中約束の使用必須
- **移行プロジェクト**：新構文と型集中約束に段階的に移行
- **旧コード保守**：旧構文の継続使用可ただし、型集中約束の採用推奨

#### 2.5.2 基本宣言構文

```yaoxiang
# === 新構文（推奨）===
# すべての宣言は：識別子: 型 = 式

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
# 関数のみ使用可能、形式：name(Types) -> Ret = Lambda
add(Int, Int) -> Int = (a, b) => a + b
square(Int) -> Int = (x) => x * x
empty() -> Void = () => {}
getRandom() -> Int = () => 42
```

#### 2.5.3 関数型構文

```
関数型 ::= '(' パラメータ型リスト ')' '->' 戻り値型
           | パラメータ型 '->' 戻り値型              # 単一パラメータ省略形

パラメータ型リスト ::= [型 (',' 型)*]
戻り値型 ::= 型 | 関数型 | 'Void'

# 関数型は一級市民であり、ネスト可能
# 高階関数型 ::= '(' 関数型 ')' '->' 戻り値型
```

| 例 | 意味 |
|------|------|
| `Int -> Int` | 単一パラメータ関数型 |
| `(Int, Int) -> Int` | 2パラメータ関数型 |
| `() -> Void` | パラメータなし関数型 |
| `(Int -> Int) -> Int` | 高階関数：関数を受け取り、Intを返す |
| `Int -> Int -> Int` | カリー化関数（右結合） |

#### 2.5.4 ジェネリクス構文（型パラメータのみ）

```yaoxiang
# ジェネリクス関数：<型パラメータ> 接頭辞
identity: (T: Type) -> ((x: T) -> T) = x
map: (A: Type, B: Type) -> ((f: (A) -> B, xs: List(A)) -> List(B)) = case xs of
  [] => []
  (x :: rest) => f(x) :: map(f, rest)

# ジェネリクス型
List: (T: Type) -> Type
```

#### 2.5.5 Lambda式構文

```
Lambda ::= '(' パラメータリスト ')' '=>' 式
         | パラメータ '=>' 式              # 単一パラメータ省略形

パラメータリスト ::= [パラメータ (',' パラメータ)*]
パラメータ ::= 識別子 [':' 型]               # オプションの型注釈
```

| 例 | 意味 | 説明 |
|------|------|------|
| `(a, b) => a + b` | 複数パラメータLambda | 宣言と組み合わせて使用：<br>`add: (Int, Int) = (a, b) => a + b` |
| `x => x + 1` | 単一パラメータ省略形 | 宣言と組み合わせて使用：<br>`inc: Int = x => x + 1` |
| `(x: Int) => x + 1` | 型注釈付き | Lambda内部で一時的に型情報が必要な場合のみ |
| `() => 42` | パラメータなしLambda | 宣言と組み合わせて使用：<br>`get: () = () => 42` |

**注意**：Lambda式内の型注釈`(x: Int) => ...`は**一時的、ローカル**なものであり、主に：
- Lambda内部で型情報が必要な場合
- 宣言構文と組み合わせて使用する場合（宣言で型が既に提供済み）
- 主な型宣言方法として使用すべきでない

#### 2.5.6 完全な例

```yaoxiang
# === 基本関数宣言 ===

# 基本関数（新構文）
add: (a: Int, b: Int) -> Int = a + b

# 単一パラメータ関数（2つの形式）
inc: (x: Int) -> Int = x + 1
inc2: (x: Int) -> Int = x + 1

# パラメータなし関数
getAnswer: () -> Int = 42

# 戻り値なし関数
log: (msg: String) -> Void = print(msg)

# === 再帰関数 ===
# 再帰はlambda内で自然にサポート
fact: (n: Int) -> Int =
  if n <= 1 then 1 else n * fact(n - 1)

# === 高階関数と関数型代入 ===

# 関数型は一級市民
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

# === ジェネリクス関数 ===

# ジェネリクス関数
identity: (T: Type) -> ((x: T) -> T) = x

# ジェネリクス高階関数
map: (A: Type, B: Type) -> ((f: (A) -> B, xs: List(A)) -> List(B)) =
  case xs of
    [] => []
    (x :: rest) => f(x) :: map(f, rest)

# ジェネリクス関数型
Transformer: Type = (A: Type, B: Type) -> ((A) -> B)

# ジェネリクス型を使用
applyTransformer: (A: Type, B: Type) -> ((f: Transformer(A, B), x: A) -> B) =
  f(x)

# === 複雑な型例 ===

# ネスト関数型
higherOrder: (A: Type) -> ((f: (A) -> Int) -> (A) -> Int) =
  f => x => f(x) + 1

# 複数パラメータ高階関数
zipWith: (A: Type, B: Type, C: Type) -> ((f: (A, B) -> C, xs: List(A), ys: List(B)) -> List(C)) =
  case (xs, ys) of
    ([], _) => []
    (_, []) => []
    (x::xs', y::ys') => f(x, y) :: zipWith(f, xs', ys')

# 関数型エイリアス
Predicate: (T: Type) -> Type = { apply: (T) -> Bool }
Mapper: Type = (A: Type, B: Type) -> ((A) -> B)
Reducer: Type = (A: Type, B: Type) -> ((B, A) -> B)

# === 旧構文例（下位互換のみ）===
# 新コードでは推奨しない

mul(Int, Int) -> Int = (a, b) => a * b    # 複数パラメータ
square(Int) -> Int = (x) => x * x          # 単一パラメータ
empty() -> Void = () => {}                  # パラメータなし
get_random() -> Int = () => 42              # 戻り値あり

# 等価な新構文（推奨）
mul: (a: Int, b: Int) -> Int = a * b
square: (x: Int) -> Int = x * x
empty: () -> Void = {}
get_random: () -> Int = 42
```

#### 2.5.7 構文解析ルール

**型解析優先度：**

| 優先度 | 型 | 説明 |
|--------|------|------|
| 1 (最高) | ジェネリクス適用 `List(T)` | 左結合 |
| 2 | 括弧 `(T)` | 結合性の変更 |
| 3 | 関数型 `->` | 右結合 |
| 4 (最低) | 基本型 `Int, String` | 原子型 |

**型解析例：**

```yaoxiang
# (A -> B) -> C -> D
# 解析結果: ((A -> B) -> (C -> D))

# A -> B -> C
# 解析結果: (A -> (B -> C))  # 右結合

# (Int -> Int) -> Int
# 解析結果: 関数を受け取り、Int -> Intを返す

# List<Int -> Int>
# 解析結果: Listの要素型は Int -> Int
```

**Lambda解析例：**

```yaoxiang
# a => b => a + b
# 解析結果: a => (b => (a + b))  # 右結合、カリー化

# (a, b) => a + b
# 解析結果: 2つのパラメータを受け取り、a + bを返す
```

#### 2.5.8 型推論ルール

YaoXiangは**二層処理**戦略を採用する：解析層は寛容に放过し、型チェック層は厳格に推論する。

**解析層ルール：**
- パーサは構文構造のみ検証し、型推論を行わない
- 型注釈が欠落している宣言は、型注釈フィールドが`None`
- 基本構文構造に準拠するすべての宣言は解析を通過できる
- **重要点**：`add: (a: Int, b: Int) -> Int = a + b`は解析層では**合法**

**型チェック層ルール：**
- 意味論的正確性を検証、型の完全性を含む
- **パラメータには型注釈が必要**：これは必須要件
- 戻り値型は推論可能だが、パラメータ型は明示的に宣言する必要がある

**完全な型推論ルール：**

| シナリオ | パラメータ推論 | 戻り値推論 | 解析結果 | 型チェック結果 | 推奨度 |
|------|---------|---------|----------|-------------|---------|
| **標準関数** | 注釈型を使用 | 注釈型を使用 | ✅ | ✅ | ⭐⭐⭐⭐⭐ |
| `add: (a: Int, b: Int) -> Int = a + b` | | | | | |
| **部分推論** | 注釈型を使用 | 式から推論 | ✅ | ✅ | ⭐⭐⭐⭐ |
| `add: (Int, Int) = (a, b) => a + b` | | | | | |
| `inc: (x: Int) -> Int = x + 1` | | | | | |
| `get: () = () => 42` | | | | | |
| **旧構文部分推論** | 注釈型を使用 | 式から推論 | ✅ | ✅ | ⭐⭐⭐ (互換) |
| `add(Int, Int) = (a, b) => a + b` | | | | | |
| `square(Int) = (x) => x * x` | | | | | |
| **パラメータ無注釈** | **推論不可** | - | ✅ | ❌ エラー | ❌ 禁止 |
| `add: (a: Int, b: Int) -> Int = a + b` | | | | | |
| `identity: (T: Type) -> ((x: T) -> T) = x` | | | | | |
| **戻り値無注釈のブロック** | - | ブロック内容から推論 | ✅ | ✅ | ⭐⭐⭐⭐ |
| `main = () => {}` | | | | | |
| `get = () => { return 42; }` | | | | | |
| **戻り値無注釈のブロック（明示的戻り値なし）** | - | `Void`に推論 | ✅ | ✅ 非推奨 | ⚠️ 避ける |
| `bad = (x: Int) => { x }` | | | | | |

**詳細推論ルール：**

```
解析層：構文構造のみ見る
├── 構造が正しい → 通過
└── 構造が間違っている → エラー

型チェック層：意味論を検証
├── パラメータ型推論
│   ├── パラメータに型注釈あり → 注釈型を使用 ✅
│   ├── パラメータに型注釈なし → 拒否 ❌
│   └── Lambdaパラメータには注釈必須 → 必須要件
│
├── 戻り値型推論
│   ├── return exprあり → exprから推論 ✅
│   ├── returnなし、式あり → 式から推論 ✅
│   ├── returnなし、ブロック`{ ... }`あり
│   │   ├── ブロックが空`{}` → Void ✅
│   │   ├── ブロックにreturnあり → returnから推論 ✅
│   │   └── ブロックにreturnなく明示的戻り値なし → Voidに推論 ✅（ただし非推奨）
│   └── 推論不可 → 拒否 ❌
│
└── 完全推論不可 → 拒否 ❌
```

**注意**：`bad = (x: Int) => { x }`这种形式は戻り値型を`Void`に推論可能だが、とても非推奨である、なぜなら：
- コードの意図が不明確
- 誤解を招きやすい
- 関数型プログラミングの式スタイル不符

**推論例：**

```yaoxiang
# === 推論成功 ===

# 標準形式
main: () -> Void = () => {}                    # 完全注釈
num: () -> Int = () => 42                      # 完全注釈
inc: (x: Int) -> Int = x + 1                   # 単一パラメータ省略形

# 部分推論（新構文）
add: (Int, Int) = (a, b) => a + b              # パラメータに注釈あり、戻り値は推論
square: (x: Int) -> Int = x * x                # パラメータに注釈あり、戻り値は推論
get_answer: () = () => 42                      # パラメータに注釈あり（空）、戻り値は推論

# 部分推論（旧構文、互換）
add2(Int, Int) = (a, b) => a + b               # パラメータに注釈あり、戻り値は推論
square2(Int) = (x) => x * x                    # パラメータに注釈あり、戻り値は推論

# returnからの推論
fact: Int -> Int = (n) => {
    if n <= 1 { return 1 }
    return n * fact(n - 1)
}

# === 推論失敗 ===

# パラメータ推論不可（解析通過、型チェック失敗）
add: (a: Int, b: Int) -> Int = a + b                          # ✗ パラメータに型なし
identity: (T: Type) -> ((x: T) -> T) = x                              # ✗ パラメータに型なし

# 明示的戻り値のないブロック
no_return = (x: Int) => { x }                  # ✗ ブロックにreturnなし、暗黙的戻り値推論不可

# 完全推論不可
bad_fn: (T: Type) -> ((x: T) -> T) = x                                # ✗ パラメータと戻り値の両方推論不可
```

#### 2.5.9 旧構文（下位互換）

YaoXiangは履歴コードとの互換のため旧構文サポートを提供する、**新コードでは推奨しない**。

```
旧構文 ::= 識別子 '(' [パラメータ型リスト] ')' '->' 戻り値型 '=' Lambda
```

| 機能 | 標準構文 | 旧構文 |
|------|---------|--------|
| 宣言形式 | `name: Type = ...` | `name(Types) -> Type = ...` |
| パラメータ型位置 | 型注釈内 | 関数名の後の括弧内 |
| 空パラメータ | `()`を書く必要がある | `()`省略可能 |
| **推奨度** | ✅ **公式推奨** | ⚠️ **下位互換のみ** |
| **使用シナリオ** | すべての新コード | 履歴コード保守 |

**非推奨理由：**
1. **学習コスト**：標準構文と不一致、言語複雑性が増加
2. **一貫性**：パラメータ型の位置が統一されていない（一方は型注釈内、もう一方は関数名の後）
3. **保守コスト**：パーサが追加の2つの形式を処理する必要がある
4. **AIフレンドリー性**：AIのコード理解と生成難易度が増加

**移行提案：**
```yaoxiang
# 旧コード（推奨しない）
mul(Int, Int) -> Int = (a, b) => a * b
square(Int) -> Int = (x) => x * x
empty() -> Void = () => {}

# 新コード（推奨）
mul: (Int, Int) -> Int = (a, b) => a * b
square: (Int) -> Int = (x) => x * x
empty: () -> Void = () => {}
```

---

## 三、型システム

### 3.1 型階層

YaoXiangの型システムは階層化されている：

```
┌─────────────────────────────────────────────────────────────┐
│                    YaoXiang 型階層                           │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  type (メタ型)                                               │
│    │                                                        │
│    ├── プリミティブ型 (Primitive Types)                     │
│    │   ├── Void, Bool                                       │
│    │   ├── Int, Uint, Float                                 │
│    │   ├── Char, String, Bytes                              │
│    │                                                        │
│    ├── コンストラクタ型 (Constructor Types)                │
│    │   ├── Name(args)              # 単一コンストラクタ（構造体）      │
│    │   ├── A(T) | B(U)             # 複数コンストラクタ（合併/列挙型）   │
│    │   ├── A | B | C               # ゼロパラメータコンストラクタ（列挙型）      │
│    │   ├── tuple (T1, T2, ...)                            │
│    │   ├── list(T), dict(K, V)                           │
│    │                                                        │
│    ├── 関数型 (Function Types)                              │
│    │   fn (T1, T2, ...) -> R                               │
│    │                                                        │
│    ├── ジェネリクス型 (Generic Types)                       │
│    │   List(T), Map(K, V), etc.                            │
│    │                                                        │
│    ├── 依存型 (Dependent Types)                             │
│    │   (n: Int) -> Type                               │
│    │                                                        │
│    └── モジュール型 (Module Types)                           │
│        ファイルはモジュール                                   │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 型定義

```yaoxiang
# 統一型構文：コンストラクタのみ、enum/struct/unionキーワードなし
# ルール：|で区切的都是コンストラクタ、コンストラクタ名(パラメータ)は型

# === ゼロパラメータコンストラクタ（列挙型スタイル）===
type Color = { red | green | blue }              # red() | green() | blue()と同等

# === 複数パラメータコンストラクタ（構造体スタイル）===
type Point = { x: Float, y: Float }       # コンストラクタは型

# === ジェネリクスコンストラクタ ===
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }           # ジェネリクス合併

# === 混合コンストラクタ ===
type Shape = circle(Float) | rect(Float, Float)

# === 値構築（関数呼び出しと完全に同じ）===
c: Color = green                              # green()と同等
p: Point = Point(1.0, 2.0)
r: Result(Int, String) = ok(42)
s: Shape = circle(5.0)

# === インターフェース定義（フィールドがすべて関数のレコード型）===
type Drawable = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}

type Serializable = {
    serialize: () -> String
}

# === インターフェース実装（型の末尾にインターフェース名を追加）===
type Point = {
    x: Float,
    y: Float,
    Drawable,        # Drawable インターフェースを実装
    Serializable     # Serializable インターフェースを実装
}
```

### 3.3 型操作

```yaoxiang
# 型は値として扱える
MyInt = Int
MyList = List(Int)

# 型リフレクション（コンストラクタパターン照合）
describe_type: (type) -> String = (t) => {
    match t {
        Point(x, y) -> "Point with x=" + x + ", y=" + y
        red -> "Red color"
        ok(value) -> "Ok value"
        _ -> "Other type"
    }
}
```

### 3.4 型推論

YaoXiangは強力な型推論能力を持つ：

```yaoxiang
# 基本推論
x = 42                    # Intと推論
y = 3.14                  # Floatと推論
z = "hello"               # Stringと推論

# 関数戻り値推論
add: (a: Int, b: Int) -> Int = a + b

# ジェネリクス推論
first: (T: Type) -> ((list: List(T)) -> Option(T)) = (list) => {
    if list.length > 0 { some(list[0]) } else { none }
}
```

---

## 四、メモリ管理

### 4.1 所有権モデル

YaoXiangは**所有権モデル**を採用してメモリを管理し、各値には一意の所有者がいる：

```yaoxiang
# === デフォルトMove（ゼロコピー）===
p: Point = Point(1.0, 2.0)
p2 = p              # Move、所有権移転、pは失效

# === refキーワード = Arc（安全共有）===
shared = ref p      # Arc、スレッドセーフ

spawn(() => print(shared.x))   # ✅ 安全

# === clone()明示的複製===
p3 = p.clone()      # pとp3は独立
```

### 4.2 Move意味論（デフォルト）

```yaoxiang
# 代入 = Move（ゼロコピー）
p: Point = Point(1.0, 2.0)
p2 = p              # Move、pは失效

# 関数引数渡しまたはMove
process: (p: Point) -> Void = {
    # pの所有権が移譲込まれる
}

# 戻り値 = Move
create: () -> Point = {
    p = Point(1.0, 2.0)
    return p        # Move、所有権移転
}
```

### 4.3 refキーワード（Arc）

```yaoxiang
# refキーワードはArcを作成（参照カウント）
p: Point = Point(1.0, 2.0)
shared = ref p      # Arc、スレッドセーフ

spawn(() => print(shared.x))   # ✅ 安全

# Arcはライフタイムを自動管理
# sharedがスコープを 벗어나ると、カウンタがゼロになり自動解放
```

### 4.4 clone()明示的複製

```yaoxiang
# 元の値を保持する必要がある場合はclone()を使用
p: Point = Point(1.0, 2.0)
p2 = p.clone()   # pとp2は独立

p.x = 0.0        # ✅
p2.x = 0.0       # ✅ 互いに影響なし
```

### 4.5 unsafeコードブロック（システムレベル）

```yaoxiang
# ベアポインタはunsafeブロック内でのみ使用可能
p: Point = Point(1.0, 2.0)

unsafe {
    ptr: *Point = &p     # ベアポインタ
    (*ptr).x = 0.0       # ユーザーが安全を保証
}
```

### 4.6 RAII

```yaoxiang
# RAIIは自動解放
with_file: (path: String) -> String = {
    file = File.open(path)  # 自動オープン
    content = file.read_all()
    # 関数終了、fileは自動クローズ
    content
}
```

### 4.7 Send / Sync制約

| 制約 | 意味 | 説明 |
|------|------|------|
| **Send** | スレッド間安全な転送可能 | 値は別のスレッドに移動可能 |
| **Sync** | スレッド間安全な共有可能 | 不変参照は別のスレッドに共有可能 |

```yaoxiang
# ref Tは自動的にSend + Syncを満たす（Arcはスレッドセーフ）
p: Point = Point(1.0, 2.0)
shared = ref p

spawn(() => print(shared.x))   # ✅ Arcはスレッドセーフ

# ベアポインタ*TはSend/Syncを満たさない
unsafe {
    ptr: *Point = &p         # 単一スレッド内でのみ使用可能
}
```

### 4.9 未実装

| 機能 | 理由 |
|------|------|
| ライフタイム `'a` | 参照概念がなく、ライフタイム不要 |
| 借用チェッカー | ref = Arcで代替 |
| `&T` 借用構文 | Move意味論を使用 |

---

## 五、非同期プログラミングと並行処理

> 「万物并作，吾以观复。」——《易·复卦》
>
> YaoXiangは**並作モデル**を採用、これは**怠惰評価**に基づく無感非同期並行パラダイムである。そのコア設計理念は：**開発者が同期、順序的な思考でロジックを記述し、言語ランタイムがその中の計算ユニットを万物並作のように自動効率的に並行実行させ、最終的に統一的に協力させる**。

> 詳細は[『並作モデル白書』](YaoXiang-async-whitepaper.md)と[非同期実装方案](YaoXiang-async-implementation.md)を参照。

### 5.1 並作モデルコアコンセプト

#### 5.1.1 並作グラフ：万物並作舞台

すべてのプログラムはコンパイル時に**有向非環計算グラフ(DAG)**に変換され、これは**並作グラフ**と呼ばれる。ノードは式計算を表し、エッジはデータ依存を表す。このグラフは怠惰であり、ノードは出力が**本当に必要**とされる 때만評価される。

```yaoxiang
# コンパイラは自動的に並作グラフを構築
fetch_user: spawn () -> User = (id) => { ... }
fetch_posts: spawn (User) -> Posts = (user) => { ... }

main:() -> Void = () => {
    user = fetch_user(1)     # ノードA (Async(User))
    posts = fetch_posts(user) # ノードB (Async(Posts))、Aに依存

    # ノードCはAとBの結果が必要
    print(posts.title)       # 自動待機：AとBが完了したことを確認
}
```

#### 5.1.2 並作値：Async(T)

`spawn fn`としてマークされた関数呼び出しは、即座に`Async(T)`型の値を返す、これは**並作値**と呼ばれる。これは実際の結果ではなく、**並作中の未来値**を表す軽量プロキシである。

**コア機能**：
- **型の透明性**：`Async(T)`は型システムにおいて`T`のサブ型であり、`T`が期待される任意のコンテキストで使用可能
- **自動待機**：プログラムが`T`型の具体的な値を使用する必要がある操作を実行するとき、ランタイムは自動的に現在のタスクを停止し、計算完了を待機
- **ゼロ伝染**：非同期コードと同期コードは構文と型署名の点で差がない

```yaoxiang
# 並作値使用例
fetch_data: spawn (String) -> JSON = (url) => { ... }

main: () -> Void = () => {
    data = fetch_data("url")  # Async(JSON)

    # Async(JSON)は直接JSONとして使用可能
    # 自動待期はフィールドアクセス時に発生
    print(data.name)          # data.await().nameと同等
}
```

### 5.2 並作構文体系

`spawn`キーワードは三重の意味を持ち、これは同期思考と非同期実装を繋ぐ唯一のアリ地である：

| 公式用語 | 構文形式 | 意味 | ランタイム動作 |
|----------|----------|------|----------------|
| **並作関数** | `spawn fn` | 並作実行に参加可能な計算ユニットを定義 | その呼び出しは`Async(T)`を返す |
| **並作ブロック** | `spawn { a(), b() }` | 明示的に宣言された並行領域 | ブロック内のタスクは強制的に並列実行 |
| **並作ループ** | `spawn for x in xs { ... }` | データ並列パラダイム | ループ体はすべての要素で並作実行 |

#### 5.2.1 並作関数

```yaoxiang
# spawnを使用して並作関数をマーク
# 構文は通常関数と完全に同じ、追加負担なし

fetch_api: spawn (String) -> JSON = (url) => {
    response = HTTP.get(url)
    JSON.parse(response.body)
}

# ネスト並作呼び出し
process_user: (Int) -> Report = (user_id) => {
    user = fetch_user(user_id)     # Async(User)
    profile = fetch_profile(user)  # Async(Profile)、userに依存
    generate_report(user, profile) # profileに依存
}
```

#### 5.2.2 並作ブロック

```yaoxiang
# spawn { } - 明示的並列構築
# ブロック内のすべての式は独立したタスクとして並行実行

compute_all: (Int, Int) -> (Int, Int, Int) spawn = (a, b) => {
    # 3つの独立計算が並列実行
    (x, y, z) = spawn {
        heavy_calc(a),        # タスク1
        heavy_calc(b),        # タスク2
        another_calc(a, b)    # タスク3
    }
    (x, y, z)
}
```

#### 5.2.3 並作ループ

```yaoxiang
# spawn for - データ並列ループ
# 各反復は独立したタスクとして並列実行

parallel_sum: (Int) -> Int spawn = (n) => {
    total = spawn for i in 0..n {
        fibonacci(i)          # 各反復が並列
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
# 明示的なawaitは不要、コンパイラが自動的に待機ポイントを挿入

main: () -> Void = () => {
    # 自動並列：2つの独立リクエストが並列実行
    users = fetch_users()      # Async(List(User))
    posts = fetch_posts()      # Async(List(Post))

    # 待機ポイントは"+"演算子処に自動的に挿入
    count = users.length + posts.length

    # フィールドアクセスが待機をトリガー
    first_user = users[0]      # users準備完了を待機
    print(first_user.name)
}

# 条件分岐内の待機
process_data: spawn () -> Void = () => {
    data = fetch_data()        # Async(Data)

    if data.is_valid {         # data準備完了を待機
        process(data)
    } else {
        log("Invalid data")
    }
}
```

### 5.4 並行制御ツール

```yaoxiang
# すべてのタスク完了を待機
await_all: (T: Type) -> ((tasks: List(Async(T))) -> List(T)) = {
    # Barrier待機
}

# いずれか1つの完了を待機
await_any: (T: Type) -> ((tasks: List(Async(T))) -> T) = {
    # 最初完了の結果を返す
}

# タイムアウト制御
with_timeout: (T: Type) -> ((task: Async(T), timeout: Duration) -> Option(T)) = {
    # タイムアウトはNoneを返す
}
```

### 5.5 スレッド安全性：Send/Sync制約

YaoXiangはRustに似た**Send/Sync型制約**を採用してスレッド安全性を保証し、コンパイル時にデータ競合を排除する。

#### 5.5.1 Send制約

**Send**：型はスレッド間を安全に**所有権を移転**できる。

```yaoxiang
# 基本型は自動的にSendを満たす
# Int, Float, Bool, StringはすべてSend

# 構造体は自動的にSendを導出
type Point = { x: Int, y: Float }
# PointはSend、IntとFloatはすべてSendだから

# 非Sendフィールドを含む型はSendではない
type NonSend = NonSend(data: Rc(Int))
# RcはSendではない（参照カウントが非原子）、したがってNonSendはSendではない
```

#### 5.5.2 Sync制約

**Sync**：型はスレッド間を安全に**参照を共有**できる。

```yaoxiang
# 基本型はすべてSync
type Point = { x: Int, y: Float }
# &PointはSync、&Intと&FloatはすべてSyncだから

# 内部的可変性を含む型
type Counter = Counter(value: Int, mutex: Mutex(Int))
# &CounterはSync、Mutexが内部的可変性を提供するから
```

#### 5.5.3 spawnとスレッド安全性

```yaoxiang
# spawnはパラメータと戻り値がSendを満たすことを要求

# 有効：DataはSend
type Data = Data(value: Int)
task = spawn(|| => Data(42))

# 無効：RcはSendではない
type SharedData = SharedData(rc: Rc(Int))
# task = spawn(|| => SharedData(Rc.new(42)))  # コンパイルエラー！

# 解決策：Arcを使用（原子参照カウント）
type SafeData = SafeData(value: Arc(Int))
task = spawn(|| => SafeData(Arc.new(42)))  # ArcはSend + Sync
```

#### 5.5.4 スレッド安全性型導出ルール

```yaoxiang
# 構造体型
type Struct(T1, T2) = Struct(f1: T1, f2: T2)

# Send導出
Struct(T1, T2): Send ⇐ T1: Send かつ T2: Send

# Sync導出
Struct(T1, T2): Sync ⇐ T1: Sync かつ T2: Sync

# 合併型
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

# Send導出
Result(T, E): Send ⇐ T: Send かつ E: Send
```

#### 5.5.5 標準ライブラリスレッド安全性実装

| 型 | Send | Sync | 説明 |
|------|:----:|:----:|------|
| `Int`, `Float`, `Bool` | ✅ | ✅ | プリミティブ型 |
| `Arc(T)` | ✅ | ✅ | T: Send + Sync |
| `Mutex(T)` | ✅ | ✅ | T: Send |
| `RwLock(T)` | ✅ | ✅ | T: Send |
| `Channel(T)` | ✅ | ❌ | 送信側のみSend |
| `Rc(T)` | ❌ | ❌ | 非原子参照カウント |
| `RefCell(T)` | ❌ | ❌ | 実行時借用チェック |


```yaoxiang
# スレッド安全カウンター例
type SafeCounter = SafeCounter(mutex: Mutex(Int))

main: () -> Void = () => {
    counter: Arc(SafeCounter) = Arc.new(SafeCounter(Mutex.new(0)))

    # 並行更新
    spawn(|| => {
        guard = counter.mutex.lock()  # Mutexはスレッド安全を提供
        guard.value = guard.value + 1
    })

    spawn(|| => {
        guard = counter.mutex.lock()
        guard.value = guard.value + 1
    })
}
```

### 5.6 ブロッキング操作

```yaoxiang
# @blockアノテーションを使用してOSスレッドをブロックする操作をマーク
# ランタイムはそれを专用ブロッキングスレッドプールに配置

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

### 6.1 モジュール定義

```yaoxiang
# モジュールはファイルを境界として使用する
# Math.yxファイル
pub pi: Float = 3.14159
pub sqrt: (x: Float) -> Float = { ... }
```

### 6.2 モジュールインポート

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

YaoXiangは**純粋関数型設計**を採用し、先進的なバインディングメカニズムを通じてシームレスなメソッド呼び出しとカリー化を実現し、`struct`、`class`などのキーワードを導入する必要がない。

### 7.1 コア関数定義

すべての操作は通常関数を通じて実装され、最初の引数は操作の主体と約束される：

```yaoxiang
# === Point.yx (モジュール) ===

# 統一構文：コンストラクタは型
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

#### 7.2.1 自動バインディング（MoonBitスタイル）

YaoXiangは名前空間に基づく自動バインディングをサポートし、**追加の宣言は不要**：

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
    result = p1.distance(p2)  # distance(p1, p2)として解決
}
```

**自動バインディングルール**：
- モジュール内で定義された関数
- 最初の引数型がモジュール名と一致する場合
- 自動的にメソッド呼び出し構文をサポート

#### 7.2.2 バインディングなしオプション（デフォルト動作）

```yaoxiang
# === Vector.yx ===

type Vector = Vector(x: Float, y: Float, z: Float)

# 内部ヘルパー関数、自动バインディング不希望
dot_product_internal: (a: Vector, b: Vector) -> Float = {
    a.x * b.x + a.y * b.y + a.z * b.z
}

# === main.yx ===

use Vector

main: () -> Void = {
    v1 = Vector(1.0, 0.0, 0.0)
    v2 = Vector(0.0, 1.0, 0.0)

    # ❌ バインディング不可：非pub関数は自動バインディングされない
    # v1.dot_product_internal(v2)  # コンパイルエラー！

    # ✅ 直接呼び出しが必要（モジュール外部からは不可）
}
```

### 7.3 位置ベースのバインディング構文

YaoXiangは**最も洗練されたバインディング構文**を提供し、位置マーク`[n]`を使用してバインディング位置を正確に制御する：

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
# 意味：メソッド呼び出し時、呼び出し元をfuncの[position]パラメータにバインディング

Point.distance = distance[0]      # 第1パラメータにバインディング
Point.add = add[0]                 # 第1パラメータにバインディング
Point.scale = scale[0]             # 第1パラメータにバインディング
```

**意味論解析**：
- `Point.distance = distance[0]`
  - `distance`関数は2つのパラメータを持つ：`distance(Point, Point)`
  - `[0]`は呼び出し元を第1パラメータにバインディングすることを意味する
  - 使用：`p1.distance(p2)` → `distance(p1, p2)`

#### 7.3.2 複数位置統合バインディング

```yaoxiang
# === Math.yx ===

# 関数：scale, point1, point2, extra1, extra2
calculate: (scale: Float, a: Point, b: Point, x: Float, y: Float) -> Float = { ... }

# === Point.yx ===

type Point = { x: Float, y: Float }

# 複数位置をバインディング
Point.calc1 = calculate[1, 2]      # scaleとpoint1をバインディング
Point.calc2 = calculate[1, 3]      # scaleとpoint2をバインディング
Point.calc3 = calculate[2, 3]      # point1とpoint2をバインディング

# === main.yx ===

use Point

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

# 1. バインディング[1,2] - 残り3,4,5
f1 = p1.calc1(2.0)  # scale=2.0, point1=p1をバインディング
# f1は今p2, x, yが必要
result1 = f1(p2, 10.0, 20.0)  # calculate(2.0, p1, p2, 10.0, 20.0)

# 2. バインディング[1,3] - 残り2,4,5
f2 = p2.calc2(2.0)  # scale=2.0, point2=p2をバインディング
# f2は今point1, x, yが必要
result2 = f2(p1, 10.0, 20.0)  # calculate(2.0, p1, p2, 10.0, 20.0)

# 3. バインディング[2,3] - 残り1,4,5
f3 = p1.calc3(p2)  # point1=p1, point2=p2をバインディング
# f3は今scale, x, yが必要
result3 = f3(2.0, 10.0, 20.0)  # calculate(2.0, p1, p2, 10.0, 20.0)
```

#### 7.3.3 残りパラメータ入力順序

**コアルール**：バインディング後、残りパラメータは**元の関数の順序**で入力され、バインディングされた位置はスキップされる。

```yaoxiang
# 関数：func(p1, p2, p3, p4, p5)

# 第1と第3パラメータをバインディング
Type.method = func[1, 3]

# 呼び出し時：
method(p2_value, p4_value, p5_value)

# マッピング：
func(p1_bound, p2_value, p3_bound, p4_value, p5_value)
# 残りパラメータ：2,4,5は元の順序で入力
```

#### 7.3.4 型チェック利点

```yaoxiang
# ✅ 合法バインディング
Point.distance = distance[0]          # distance(Point, Point)
Point.calc = calculate[1, 2]          # calculate(scale, Point, Point, ...)

# ❌ 非法バインディング（コンパイラがエラー）
Point.wrong = distance[5]             # 第5パラメータが存在しない
Point.wrong = distance[0]             # パラメータは1から始まる
Point.wrong = distance[1, 2, 3, 4]    # 関数のパラメータ数を超えている
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

# 2. 異なる位置にバインディング
f = p1.distance_scaled(2.0)  # 第2パラメータをバインディング、残り第1,3
result = f(p2)               # distance_with_scale(2.0, p1, p2)

# 3. チェーンバインディング
d2 = p1.distance(p2).distance_scaled(2.0)  # チェイン呼び出し
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

#### 7.6.1 pubの効果

```yaoxiang
# === Point.yx ===

type Point = { x: Float, y: Float }

# 非pub関数
internal_distance: (a: Point, b: Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    (dx * dx + dy * dy).sqrt()
}

# pub関数
pub distance: (a: Point, b: Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    (dx * dx + dy * dy).sqrt()
}

# === main.yx ===

use Point

# 自動バインディングはpub関数にのみ有効
p1.distance(p2)      # ✅ distanceはpub、自動バインディング可能
# p1.internal_distance(p2)  # ❌ pubではない、バインディング不可
```

#### 7.6.2 pub自動バインディングメカニズム

`pub`で宣言された関数は、コンパイラが同じファイルで定義された型に自動的にバインディングする：

```yaoxiang
# === Point.yx ===

type Point = { x: Float, y: Float }

# pubを使用して宣言、コンパイラが自動的にバインディング
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

pub translate: (self: Point, dx: Float, dy: Float) -> Point = {
    Point(self.x + dx, self.y + dy)
}

# コンパイラは自動的にバインディングを推測して実行：
# Point.distance = distance[0]
# Point.translate = translate[0]

# === main.yx ===

use Point

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

# ✅ 関数型呼び出し
d = distance(p1, p2)

# ✅ OOP糖衣構文（自動バインディング）
d2 = p1.distance(p2)
p3 = p1.translate(1.0, 1.0)
```

**自動バインディングルール**：
1. 関数はモジュールファイルで定義（型と同じファイル）
2. 関数の引数にその型が含まれる
3. `pub`でエクスポート
4. コンパイラが自動的に`Type.method = function[0]`を実行

**利点**：
- 手動でバインディング宣言を書く必要がない
- コードがより簡潔
- バインディングの見落としやミスを回避

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
# しかし自動バインディングはpubエクスポートされた関数のみが外部で有効

pub distance  # エクスポート、外部で自動バインディング可能
```

### 7.7 設計利点まとめ

| 機能 | 説明 |
|------|------|
| **ゼロ構文負担** | 自動バインディングに宣言不要 |
| **位置精密制御** | `[n]`でバインディング位置を精密指定 |
| **複数位置統合** | `[1, 2, 3]`複数パラメータバインディングをサポート |
| **型安全** | コンパイラがバインディング位置の有効性を検証 |
| **キーワード不要** | `bind`などのキーワード不要 |
| **柔軟カリー化** | 任意位置パラメータバインディングをサポート |
| **pub制御** | pub関数のみが外部バインディング可能 |

### 7.8 従来のメソッドバインディングとの違い

| 伝統言語 | YaoXiang |
|---------|----------|
| `obj.method(arg)` | `obj.method(arg)` |
| class/メソッド定義が必要 | 関数 + バインディング宣言のみ |
| 構文 `class { method() {} }` | 構文 `Type.method = func[n]` |
| 継承、多態 | 純粋関数型、継承なし |
| メソッドテーブル検索 | コンパイル時バインディング、実行時オーバーヘッドなし |

**コア優位性**：YaoXiangのバインディングは**コンパイル時メカニズム**であり、ゼロ実行時コストで、関数型プログラミングの純粋さと柔軟性を維持している。

---

## 八、AIフレンドリー設計

YaoXiangの構文設計はAIコード生成と変更のニーズを特に考慮している：

### 8.1 設計原則

```yaoxiang
# AIフレンドリー設計目標：
# 1. 厳密に構造化、曖昧さのない構文
# 2. ASTが清晰、位置決めが容易
# 3. 意味論が明確、隠れた動作なし
# 4. コードブロック境界が明確
# 5. 型情報が完全
```

### 8.2 厳密に構造化された構文

#### 8.2.1 宣言構文のAIフレンドリー戦略

```yaoxiang
# === AIコード生成ベストプラクティス ===

# ✅ 推奨：完全な新構文宣言 + 型集中約束を使用
# AIは意図を正確に理解し、完全な型情報を生成

add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1
empty: () -> Void = {}

# ❌ 避ける：型注釈を省略または型が分散
# AIはパラメータ型を決定できず、誤ったコードを生成する可能性
add: (a: Int, b: Int) -> Int = a + b          # パラメータに型なし
identity: (T: Type) -> ((x: T) -> T) = x              # パラメータに型なし
add2: (a: Int, b: Int) -> Int = a + b  # 型が実装に分散

# ⚠️ 互換：旧構文は旧コード保守のみ
# AIは新構文 + 型集中約束を優先して生成すべき
mul(Int, Int) -> Int = (a, b) => a * b  # 新コードでは使用禁止
```

**型集中約束のAI利点：**

1. **署名が一目でわかる**：AIは宣言行を読むだけで完全な関数署名を理解できる
2. **より安全な修正**：型の修正は宣言を変更するだけで、実装本体に影響なし
3. **より簡単な生成**：AIはまず宣言を生成し、実装を埋めることができる
4. **カリー化フレンドリー**：明確なカリー化型署名によりAIが処理しやすい

```yaoxiang
# AI処理例
# 入力：実装本体 (a, b) => a + b
# AIは宣言を見る：add: (Int, Int) -> Int
# 結論：パラメータ型はInt, Int、戻り値型はInt

# 比較：型が分散している場合
# 入力：実装本体 (a: Int, b: Int) => a + b
# AIが必要：実装本体を分析して型情報を抽出
# 結果：より複雑な処理ロジック、エラーが発生しやすい
```

#### 8.2.2 二重構文戦略とAI

| 構文タイプ | AI生成戦略 | 使用シナリオ |
|---------|-----------|---------|
| **新構文** | ✅ 優先生成、完全な型情報 | すべての新コード生成 |
| **旧構文** | ⚠️ 旧コード保守時のみ使用 | 履歴コード変更 |
| **無注釈** | ❌ 生成禁止 | 任何情况都应避免 |

#### 8.2.3 構文境界が明確

```yaoxiang
# AIフレンドリーなコードブロック境界

# ✅ 明確な開始と終了マーク
function_name: (Type1, Type2) -> ReturnType = (param1, param2) => {
    # 関数本体
    if condition {
        do_something()
    } else {
        do_other()
    }
}

# ✅ 条件文には必ず波括弧が必要
if condition {
    # 条件本体
}

# ✅ 型定義が明確
type MyType = Type1 | Type2

# ❌ 避ける曖昧な構文
if condition    # 波括弧がない
    do_something()
```

#### 8.2.4 曖昧さのない構文制約

```yaoxiang
# AI生成時に守るべき制約

# 1. 括弧の省略禁止
# ✅ 正しい
foo: (T: Type) -> ((x: T) -> T) = x
my_list = [1, 2, 3]

# ❌ エラー（禁止）
foo T { T }             # パラメータには括弧が必要
my_list = [1 2 3]       # リストにはカンマが必要

# 2. 戻り値型または推論可能な形式を明示する必要がある
# ✅ 正しい
get_num: () -> Int = 42
get_num2: () = 42          # 戻り値型が推論可能

# ❌ エラー
get_bad = () => { 42 }           # ブロックにreturnがない、推論不可

# 3. パラメータには型注釈が必要（新構文）
# ✅ 正しい
add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1

# ❌ エラー
add: (a: Int, b: Int) -> Int = a + b            # パラメータに型なし
identity: (T: Type) -> ((x: T) -> T) = x                # パラメータに型なし
```

#### 8.2.5 AI生成推奨パターン

```yaoxiang
# AI関数生成時の標準テンプレート

# パターン1：完全な型注釈
function_name: (param1: ParamType1, param2: ParamType2, ...) -> ReturnType = {
    # 関数本体
    return expression
}

# パターン2：戻り値型推論
function_name: (param1: ParamType1, param2: ParamType2) = {
    # 関数本体
    return expression
}

# パターン3：単一パラメータ省略形
function_name: (param: ParamType) -> ReturnType = expression

# パターン4：パラメータなし関数
function_name: () -> ReturnType = expression

# パターン5：空関数
function_name: () -> Void = {}
```

### 8.3 エラーメッセージのAIフレンドリー性

```yaoxiang
# エラーメッセージは明確な修正提案を提供するべき

# フレンドリーでないエラー
# Syntax error at token 'a'

# AIフレンドリーなエラー
# Missing type annotation for parameter 'a'
# Suggestion: add ': Int' or similar type to '(a, b) => a + b'
# Correct version: add: (a: Int, b: Int) -> Int = a + b
```

---

## 九、型集中約束（コア設計哲学）

### 9.1 約束概述

YaoXiangのコア設計約束は**「宣言優先、型集中」**である。この約束は言語のAIフレンドリー性と開発効率の基盤である。

```yaoxiang
# ✅ コア約束：型情報は宣言行に統一
add: (a: Int, b: Int) -> Int = a + b

# ❌ 避ける：型情報が実装に分散
add: (a: Int, b: Int) -> Int = a + b
```

### 9.2 約束の5つのコア利点

#### 1. 構文一貫性
```yaoxiang
# すべての宣言は同じ形式に従う
x: Int = 42                           # 変数
name: String = "YaoXiang"             # 変数
add: (a: Int, b: Int) -> Int = a + b  # 関数
inc: (x: Int) -> Int = x + 1          # 関数
type Point = { x: Float, y: Float } # 型
```

#### 2. 宣言と実装の分離
```yaoxiang
# 宣言行は完全な型情報を提供
add: (a: Int, b: Int) -> Int = a + b
# └────────────────────┘
#   完全な関数署名

# 実装本体はビジネスロジックに集中
# (a, b) => a + b  型を気にする必要はなく、機能実装のみ
```

#### 3. AIフレンドリー性
```yaoxiang
# AI処理フロー：
# 1. 宣言行を読む → 完全に関数署名を理解
# 2. 実装を生成 → 型推論が不要
# 3. 型を修正 → 宣言行のみ変更、実装本体に影響なし

# 比較：型が分散している場合
add: (a: Int, b: Int) -> Int = a + b
# AIが必要：実装本体を分析して型情報を抽出 → より複雑、エラーが発生しやすい
```

#### 4. より安全な修正
```yaoxiang
# パラメータ型を修正
# 原来: add: (a: Int, b: Int) -> Int = a + b
# 修正: add: (Float, Float) -> Float = (a, b) => a + b
# 実装本体: (a, b) => a + b  変更不要！

# 型が分散している場合：
# 原来: add: (a: Int, b: Int) -> Int = a + b
# 修正: add: (a: Float, b: Float) -> Float = a + b  # 2箇所変更が必要
```

#### 5. カリー化フレンドリー
```yaoxiang
# カリー化型は一目でわかる
add_curried: (a: Int) -> (b: Int) -> Int = a + b
#              └─────────────┘
#              カリー化署名

# 関数合成は一級市民
compose: (Int -> Int, Int -> Int) -> Int -> Int = (f, g) => x => f(g(x))
```

### 9.3 約束の実施ルール

#### ルール1：パラメータは宣言で型を指定する必要がある
```yaoxiang
# ✅ 正しい
add: (a: Int, b: Int) -> Int = a + b

# ❌ エラー
add: (a: Int, b: Int) -> Int = a + b            # パラメータ型が欠落
identity: (T: Type) -> ((x: T) -> T) = x                # パラメータ型が欠落
```

#### ルール2：戻り値型は推論可能だが推奨は注釈
```yaoxiang
# ✅ 推奨：完全注釈
get_num: () -> Int = () => 42

# ✅ 許容：戻り値型推論
get_num: () = () => 42

# ✅ 空関数はVoidに推論
empty: () = () => {}
```

#### ルール3：Lambda内部の型注釈は一時的なもの
```yaoxiang
# ✅ 正しい：宣言の型に依存
add: (a: Int, b: Int) -> Int = a + b

# ⚠️ 可能だが非推奨：Lambda内で重複注釈
add: (Int, Int) -> Int = (a: Int, b: Int) => a + b

# ❌ エラー：宣言注釈が欠落
add: (a: Int, b: Int) -> Int = a + b
```

#### ルール4：旧構文は同じ理念に従う
```yaoxiang
# 旧構文も宣言位置で型情報を提供するべき
# 形式は異なるが、理念は一貫している：
# - 宣言行には主要な型情報が含まれる
# - 実装本体は比較的簡潔
add(Int, Int) -> Int = (a, b) => a + b
```

### 9.4 約束と型推論の関係

```yaoxiang
# 約束は型推論を妨げるものではなく、推論方向を誘導する

# 1. 完全注釈（推論なし）
add: (a: Int, b: Int) -> Int = a + b

# 2. 部分推論（宣言がパラメータ型を提供）
add: (Int, Int) = (a, b) => a + b  # 戻り値型は推論

# 3. 空関数は推論
empty: () = () => {}  # () -> Voidに推論
```

### 9.5 約束のAI実装利点

**AIコード生成フロー：**

1. **ニーズを読む** → 宣言を生成
   ```
   ニーズ：加算関数
   生成：add: (Int, Int) -> Int = (a, b) => ???
   ```

2. **実装を埋める** → 型分析が不要
   ```
   実装：add: (a: Int, b: Int) -> Int = a + b
   ```

3. **型を修正** → 宣言のみ変更
   ```
   修正：add: (Float, Float) -> Float = (a, b) => a + b
   実装：(a, b) => a + b  変更なし
   ```

**約束なしのAI処理との比較：**
```
ニーズ：加算関数
AIが必要：
  1. パラメータ型を推論
  2. 戻り値型を推論
  3. 実装本体を生成
  4. 一致性を検証
  5. 型変化時の複雑な更新を処理

結果：より複雑、よりエラーが発生しやすい
```

### 9.6 約束の哲学的意味

この約束はYaoXiangの核心的理念を体現している：

- **宣言即ドキュメント**：宣言行は完全な関数ドキュメントである
- **型即契約**：型情報は呼び出し元と実装者の間の契約である
- **ロジック即実装**：実装本体は「何をするか」のみを関心し、「何の型か」を関知しない
- **ツール即補助**：型システム、AIツールは明確な宣言に基づいて作業できる

### 9.7 実際の応用比較

#### 完全な例：電卓モジュール

```yaoxiang
# === 推奨方法：型集中約束 ===

# モジュール宣言
pub add: (a: Int, b: Int) -> Int = a + b
pub multiply: (a: Int, b: Int) -> Int = a * b

# 高階関数
pub apply_twice: (f: Int -> Int, x: Int) -> Int = f(f(x))

# カリー化関数
pub make_adder: (x: Int) -> (Int) -> Int = y => x + y

# ジェネリクス関数
pub map: (A: Type, B: Type) -> ((f: (A) -> B, xs: List(A)) -> List(B)) = case xs of
  [] => []
  (x :: rest) => f(x) :: map(f, rest)

# 型定義
type Point = { x: Float, y: Float }
pub distance: (a: Point, b: Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    (dx * dx + dy * dy).sqrt()
}

# === 非推奨方法：型が分散 ===

# パラメータ型がLambda内
add: (a: Int, b: Int) -> Int = a + b
multiply = (a: Int, b: Int) => a * b

# 高階関数の型が分散
apply_twice = (f: (Int) -> Int, x: Int) => f(f(x))

# カリー化型が分散
make_adder = (x: Int) => (y: Int) => x + y

# ジェネリクス型が分散
map: (A: Type, B: Type) -> ((f: (A) -> B, xs: List(A)) -> List(B)) = case xs of
  [] => []
  (x :: rest) => f(x) :: map(f, rest)
```

#### コード保守比較

```yaoxiang
# ニーズ：addをIntからFloatに変更

# === 推奨方法：宣言行のみ変更 ===
# 原来
add: (a: Int, b: Int) -> Int = a + b

# 修正後
add: (a: Float, b: Float) -> Float = a + b
#              ↑↑↑↑↑↑↑↑↑          ↑↑↑↑↑↑↑
#              宣言行変更          実装本体は変更なし

# === 非推奨方法：複数箇所変更が必要 ===
# 原来
add: (a: Int, b: Int) -> Int = a + b

# 修正後
add: (a: Float, b: Float) -> Float = a + b
#     ↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑
#     すべてのパラメータ型を変更する必要がある
```

#### AI支援プログラミング比較

```yaoxiang
# AIニーズ：2点間のマンハッタン距離を計算する関数を実装

# === AIが推奨写法，看到时 ===
type Point = { x: Float, y: Float }
pub manhattan: (a: Point, b: Point) -> Float = ???  # AIは完全な署名を知る

# AI生成：
pub manhattan: (a: Point, b: Point) -> Float = {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}

# === AIが非推奨写法，看到时 ===
type Point = { x: Float, y: Float }
pub manhattan = ???  # AIが必要：パラメータ型？戻り値型？

# AIが可能に生成：
pub manhattan = (a: Point, b: Point) => Float => {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}
# または型情報が不完全なためエラーが発生する可能性
```

### 9.8 約束実施チェックリスト

YaoXiangコードを書く際に、以下のチェックリストを使用できる：

- [ ] すべての関数宣言は宣言行に完全な型注釈を持つ
- [ ] パラメータ型は宣言で指定し、Lambda内ではない
- [ ] 戻り値型は可能な限り宣言で注釈する
- [ ] 変数宣言は`name: Type = value`形式を使用
- [ ] Lambda本体は簡潔に保ち、重複した型情報を避ける
- [ ] 新構文を使用し、旧構文を避ける
- [ ] 複雑な型はtype定義を使用して宣言を明晰に保つ

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
# 変数と型
x = 42                    # Intと自動推論
name = "YaoXiang"         # Stringと自動推論
pi = 3.14159              # Floatと自動推論

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
f = p1.distance_scaled(2.0)  # scaleとp1をバインディング
result = f(p2)               # 最終呼び出し

# または直接使用
result2 = p1.distance_scaled(2.0, p2)
```

### 10.5 次のステップ

- 完全な構文については[言語仕様](./YaoXiang-language-specification.md)を参照
- 一般的なパターンを学習するには[サンプルコード](./examples/)を参照
- 技術的詳細については[実装計画](./YaoXiang-implementation.md)を参照

---

## 付録

### A. キーワードとアノテーション

| キーワード | 機能 |
|--------|------|
| `type` | 型定義 |
| `pub` | 公開エクスポート |
| `use` | モジュールインポート |
| `spawn` | 非同期マーク（関数/ブロック/ループ） |
| `ref` | 不変参照 |
| `mut` | 可変参照 |
| `if/elif/else` | 条件分岐 |
| `match` | パターン照合 |
| `while/for` | ループ |
| `return/break/continue` | 制御フロー |
| `as` | 型変換 |
| `in` | メンバーアクセス |

| アノテーション | 機能 |
|------|------|
| `@block` | 完全同期コードとしてマーク |
| `@eager` | 怠惰評価が必要な式としてマーク |
| `@Send` | Send制約を満たすことを明示的に宣言 |
| `@Sync` | Sync制約を満たすことを明示的に宣言 |

### B. 設計インスピレーション

- **Rust**：所有権モデル、ゼロコスト抽象化
- **Python**：構文スタイル、読みやすさ
- **Idris/Agda**：依存型、型駆動開発
- **TypeScript**：型注釈、実行時型

---

## バージョン履歴

| バージョン | 日付 | 著者 | 変更説明 |
|------|------|------|---------|
| v1.0.0 | 2024-12-31 | 晨煦 | 初期バージョン |
| v1.1.0 | 2025-01-04 | 沫郁酱 | ジェネリクス構文を`<T>`から`[T]`に修正；`fn`キーワードを削除；関数定義示例を更新 |
| v1.2.0 | 2025-01-06 | 晨煦 | 新構文形式に統一：name: type -> type = lambda |
| v1.3.0 | 2025-01-20 | 晨煦 | 統一型構文を追加（RFC-010）：インターフェース定義は波括弧`{ serialize: () -> String }`を使用；型末尾にインターフェース名を追加してインターフェースを実装；`pub`自動バインディングメカニズムを追加 |

---

> 「道生一，一生二，二生三，三生万物。」
> —— 『道徳経』
>
> 型は道なり，万物はここから生まれる。
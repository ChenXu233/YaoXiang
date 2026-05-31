# YaoXiang（爻象）プログラミング言語ガイド

> バージョン：v1.2.0
> ステータス：草稿
> 著者：晨煦
> 日付：2024-12-31
> 更新：2025-01-20 - 位置インデックスが0から開始（RFC-004）；統一型構文（RFC-010）

---

## 目次

1. [言語概述](#一言語概述)
2. [コア機能](#二コア機能)
3. [型システム](#三型システム)
4. [メモリ管理](#四メモリ管理)
5. [非同期プログラミングと並行処理](#五非同期プログラミングと並行処理)
6. [モジュールシステム](#六モジュールシステム)
7. [メソッドバインディングとカリー化](#七メソッドバインディングとカリー化)
8. [AI友好的設計](#八ai友好的設計)
9. [型集中約束（コア設計哲学）](#九型集中約束コア設計哲学)
10. [クイックスタート](#十クイックスタート)

---

**拡張ドキュメント**：

- [高度なバインディング機能とコンパイラ実装](../works/plans/bind/YaoXiang-bind-advanced.md) - 深いバインディング機構、高度な機能、コンパイラ実装、エッジケース処理

---

## 一、言語概述

### 1.1 YaoXiangとは？

YaoXiang（爻象）は実験的な汎用プログラミング言語であり、『易経』における「爻」と「象」の核心理念から設計されました。「爻」は卦象を構成する基本記号であり、陰陽の変化を象徴します。「象」は事物の本質的な外在的表現であり、万象万物を表します。

YaoXiang はこの哲学的思考をプログラミング言語の型システムに組み込み、**「すべてが型である」**という核心理念を提唱しています。YaoXiang の世界観では：

- **値**は型のインスタンスである
- **型**自体も型のインスタンスである（元型）
- **関数**は入力型から出力型への写像である
- **モジュール**は型の名前空間组合せである

### 1.2 設計目標

YaoXiang の設計目標は次のとおり要約できます：

| 目標 | 説明 |
|------|------|
| **統一された型抽象** | 型は最上位の抽象ユニットであり、言語意味を簡略化する |
| **自然なプログラミング体験** | Python 風の構文、読みやすさを重視 |
| **安全なメモリ管理** | Rust 風の所有権モデル、GC なし |
| **無痛の非同期プログラミング** | 非同期を自動管理、明示的な await 不要 |
| **完全な型リフレクション** | 実行時の型情報が完全に利用可能 |
| **AI 友好的な構文** | 厳密な構造化、AI による処理が容易 |

### 1.3 言語の位置づけ

| 次元 | 位置づけ |
|------|----------|
| パラダイム | マルチパラダイム（関数型 + 命令型 + オブジェクト指向） |
| 型システム | 依存型 + パラメータ多相 |
| メモリ管理 | 所有権 + RAII（GC なし） |
| コンパイルモデル | AOT コンパイル（オプションで JIT） |
| ターゲットシナリオ | システムプログラミング、アプリケーション開発、AI 支援プログラミング |

### 1.4 コード例

```yaoxiang
# 自動型推論
x: Int = 42                           # 明示的な型
y = 42                                # Int と推論
name = "YaoXiang"                     # String と推論

# デフォルトで不変
x: Int = 10
x = 20                                # ❌ コンパイルエラー！不変

# 統一宣言構文：識別子: 型 = 式
add: (a: Int, b: Int) -> Int = a + b  # 関数宣言
inc: (x: Int) -> Int = x + 1               # 単一パラメータ関数

# 統一型構文：コンストラクタが型である
type Point = { x: Float, y: Float }
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

# 無痛非同期（並作関数）
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

### 2.1 すべてが型である

YaoXiang のコア設計哲学は**すべてが型である**です。これは YaoXiang において：

1. **値は型のインスタンスである**：`42` は `Int` 型のインスタンス
2. **型は型のインスタンスである**：`Int` は `type` 元型のインスタンス
3. **関数は型の写像である**：`add: (Int, Int) -> Int` は関数型である
4. **モジュールは型の組み合わせである**：モジュールは関数と型を含む名前空間

```yaoxiang
# 値は型のインスタンスである
x: Int = 42

# 型は型のインスタンスである
MyList: type = List(Int)

# 関数は型間の写像である
add: (a: Int, b: Int) -> Int = a + b

# モジュールは型の組み合わせ（ファイルをモジュールとして使用）
# Math.yx
pi: Float = 3.14159
sqrt: (x: Float) -> Float = { ... }
```

### 2.2 数学的抽象

YaoXiang の型システムは型理論と圏論に基づいており、次のを提供します：

- **依存型**：型は値に依存できる
- **ジェネリックプログラミング**：型パラメータ化
- **型組み合わせ**：合併型、交差型

```yaoxiang
# 依存型：固定長ベクトル
Vector: (T: Type, n: Int) -> Type = vector(T, n)

# 型の合併
type Number = Int | Float

# 型の交差
type Printable = printable(fn() -> String)
type Serializable = serializable(fn() -> String)
type Versatile = Printable & Serializable
```

### 2.3 ゼロコスト抽象

YaoXiang はゼロコスト抽象を保証します：高水準の抽象は実行時のパフォーマンスオーバーヘッドをもたらしません：

- **単態化**：ジェネリック関数はコンパイル時に具体バージョンに展開される
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

YaoXiang は Python 風の構文を採用し、読みやすさと自然言語感覚を追求します：

```yaoxiang
# 自動型推論
x = 42
name = "YaoXiang"

# 簡潔な関数定義
greet: (name: String) -> String = "Hello, " + name

# パターンマッチング
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

YaoXiang は統一された宣言構文を採用：**識別子: 型 = 式**。後方互換性のための古い構文も提供します。

#### 2.5.1 二重構文戦略と型集中約束

革新と互換性のバランスを取るため、YaoXiang は2つの構文形式をサポートしますが、统一された**型集中表記約束**を採用します。

**構文形式の比較：**

| 構文タイプ | フォーマット | ステータス | 説明 |
|-----------|-------------|-----------|------|
| **新構文（標準）** | `name: Type = Lambda` | ✅ 推奨 | 公式標準、すべての新規コードはこの形式を使用すべき |
| **旧構文（互換）** | `name(Types) -> Ret = Lambda` | ⚠️ 互換のみ | 歴史的コードのために維持、新規プロジェクトでは非推奨 |

**コア約束：型集中表記**

YaoXiang は**「宣言優先、型集中」**という設計約束を採用します：

```yaoxiang
# ✅ 正しい：型情報が宣言行に統一されている
add: (a: Int, b: Int) -> Int = a + b
#   └─────────────────┘   └─────────────┘
#       完全な型署名         実装ロジック

# ❌ 避ける：型情報が実装に分散している
add: (a: Int, b: Int) -> Int = a + b
#     └───────────────┘
#     型が実装ボディに混在
```

**約束好处：**

1. **構文の一貫性**：すべての宣言が `識別子: 型 = 式` に従う
2. **宣言と実装の分離**：型情報が明確、実装ボディはロジックに集中
3. **AI友好性**：AIは宣言行を読むだけで完全な関数シグネチャを理解できる
4. **より安全な修正**：型の修正は宣言を変更するだけでよく、実装ボディに影響しない
5. **カリー化友好**：明確なカリー化型シグネチャをサポート

**選択のアドバイス**：

- **新規プロジェクト**：新しい構文 + 型集中約束の使用必須
- **移行プロジェクト**：新しい構文と型集中約束に徐々に移行
- **古いコードのメンテナンス**：古い構文の使用を継続できるが、型集中約束を採用することを推奨

#### 2.5.2 基本宣言構文

```yaoxiang
# === 新構文（推奨）===
# すべての宣言が従う：識別子: 型 = 式

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
# 関数のみに使用、フォーマット：name(Types) -> Ret = Lambda
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

# 関数型は第一級市民、ネスト可能
# 高階関数型 ::= '(' 関数型 ')' '->' 戻り値型
```

| 例 | 意味 |
|------|------|
| `Int -> Int` | 単一パラメータ関数型 |
| `(Int, Int) -> Int` | 2パラメータ関数型 |
| `() -> Void` | パラメータなし関数型 |
| `(Int -> Int) -> Int` | 高階関数：関数を受け取り、Int を返す |
| `Int -> Int -> Int` | カリー化関数（右結合） |

#### 2.5.4 ジェネリック構文（型パラメータのみに使用）

```yaoxiang
# ジェネリック関数：<型パラメータ> プレフィックス
identity: (T: Type) -> ((x: T) -> T) = x
map: (A: Type, B: Type) -> ((f: (A) -> B, xs: List(A)) -> List(B)) = case xs of
  [] => []
  (x :: rest) => f(x) :: map(f, rest)

# ジェネリック型
List: (T: Type) -> Type
```

#### 2.5.5 Lambda 式構文

```
Lambda ::= '(' パラメータリスト ')' '=>' 式
         | パラメータ '=>' 式              # 単一パラメータ省略形

パラメータリスト ::= [パラメータ (',' パラメータ)*]
パラメータ ::= 識別符 [':' 型]               # オプションの型注釈
```

| 例 | 意味 | 説明 |
|------|------|------|
| `(a, b) => a + b` | 複数パラメータ Lambda | 宣言と組み合わせて使用：<br>`add: (Int, Int) = (a, b) => a + b` |
| `x => x + 1` | 単一パラメータ省略形 | 宣言と組み合わせて使用：<br>`inc: Int = x => x + 1` |
| `(x: Int) => x + 1` | 型注釈付き | Lambda 内部で一時的に必要な場合のみ |
| `() => 42` | パラメータなし Lambda | 宣言と組み合わせて使用：<br>`get: () = () => 42` |

**注意**：Lambda 式内の型注釈 `(x: Int) => ...` は**一時的、ローカルなもの**であり、主に：

- Lambda 内部で型情報が必要な場合
- 宣言構文と組み合わせて使用する場合（型は宣言ですでにある）
- メインの型宣言方法として使用すべきではない

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
# 再帰は lambda 内で自然にサポート
fact: (n: Int) -> Int =
  if n <= 1 then 1 else n * fact(n - 1)

# === 高階関数と関数型代入 ===

# 関数型を第一級市民として
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

# ジェネリック関数型
Transformer: Type = (A: Type, B: Type) -> ((A) -> B)

# ジェネリック型の使用
applyTransformer: (A: Type, B: Type) -> ((f: Transformer(A, B), x: A) -> B) =
  f(x)

# === 複雑な型の例 ===

# ネストされた関数型
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

# === 旧構文の例（後方互換のみ）===
# 新コードでは非推奨

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

**型解析の優先順位：**

| 優先順位 | 型 | 説明 |
|----------|------|------|
| 1（最高） | ジェネリック適用 `List(T)` | 左結合 |
| 2 | 括弧 `(T)` | 結合性の変更 |
| 3 | 関数型 `->` | 右結合 |
| 4（最低） | 基本型 `Int, String` | 原子型 |

**型解析の例：**

```yaoxiang
# (A -> B) -> C -> D
# 解析：((A -> B) -> (C -> D))

# A -> B -> C
# 解析：(A -> (B -> C))  # 右結合

# (Int -> Int) -> Int
# 解析：関数を受け取り、Int -> Int を返す

# List<Int -> Int>
# 解析：List の要素型は Int -> Int
```

**Lambda 解析の例：**

```yaoxiang
# a => b => a + b
# 解析：a => (b => (a + b))  # 右結合、カリー化

# (a, b) => a + b
# 解析：2つのパラメータを受け取り、a + b を返す
```

#### 2.5.8 型推論ルール

YaoXiang は**二层処理**戦略を採用：解析層は寛容に放过し、型チェック層は厳密に推論します。

**解析層のルール：**

- パーサは構文構造のみを検証し、型推論を行わない
- 型注釈がない宣言は、型注釈フィールドが `None`
- 基本構文構造に準拠するすべての宣言が解析を通る
- **重要点**：`add: (a: Int, b: Int) -> Int = a + b` は解析層では**合法**

**型チェック層のルール：**

- 意味的正确性を検証、型の完全性を含む
- **パラメータには型注釈が必要**：これは必須要件
- 戻り値型は推論可能だが、パラメータ型は明示的に宣言する必要がある

**完全な型推論ルール：**

| シナリオ | パラメータ推論 | 戻り値推論 | 解析結果 | 型チェック結果 | 推奨度 |
|----------|---------------|-----------|----------|---------------|--------|
| **標準関数** | 注釈された型を使用 | 注釈された型を使用 | ✅ | ✅ | ⭐⭐⭐⭐⭐ |
| `add: (a: Int, b: Int) -> Int = a + b` | | | | | |
| **部分推論** | 注釈された型を使用 | 式から推論 | ✅ | ✅ | ⭐⭐⭐⭐ |
| `add: (Int, Int) = (a, b) => a + b` | | | | | |
| `inc: (x: Int) -> Int = x + 1` | | | | | |
| `get: () = () => 42` | | | | | |
| **旧構文部分推論** | 注釈された型を使用 | 式から推論 | ✅ | ✅ | ⭐⭐⭐ (互換) |
| `add(Int, Int) = (a, b) => a + b` | | | | | |
| `square(Int) = (x) => x * x` | | | | | |
| **パラメータ無注釈** | **推論不可** | - | ✅ | ❌ エラー | ❌ 禁止 |
| `add: (a: Int, b: Int) -> Int = a + b` | | | | | |
| `identity: (T: Type) -> ((x: T) -> T) = x` | | | | | |
| **戻り値無注釈のブロック** | - | ブロック内容から推論 | ✅ | ✅ | ⭐⭐⭐⭐ |
| `main = () => {}` | | | | | |
| `get = () => { return 42; }` | | | | | |
| **戻り値無注釈ブロック（明示的戻り値なし）** | - | `Void` と推論 | ✅ | ✅ 非推奨 | ⚠️ 避ける |
| `bad = (x: Int) => { x }` | | | | | |

**詳細推論ルール：**

```
解析層：構文構造のみを見る
├── 構造が正しい → 通過
└── 構造が間違う → エラー

型チェック層：意味を検証
├── パラメータ型推論
│   ├── パラメータに型注釈あり → 注釈された型を使用 ✅
│   ├── パラメータに型注釈なし → 拒否 ❌
│   └── Lambda パラメータには注釈必須 → 必須要件
│
├── 戻り値型推論
│   ├── return expr あり → expr から推論 ✅
│   ├── return なし、式あり → 式から推論 ✅
│   ├── return なし、ブロック `{ ... }` あり
│   │   ├── ブロックが空 `{}` → Void ✅
│   │   ├── ブロックに return あり → return から推論 ✅
│   │   └── ブロックに return なし、明示的戻り値なし → Void と推論 ✅（だが非推奨）
│   └── 推論不可 → 拒否 ❌
│
└── 完全推論不可 → 拒否 ❌
```

**注意**：`bad = (x: Int) => { x }` 这种形式は戻り値型を `Void` と推論できますが 매우 不推荐，因为：

- 代码意图不明确
- 容易造成理解错误
- 不符合函数式编程的表达式风格

**推論例：**

```yaoxiang
# === 推論成功 ===

# 標準形式
main: () -> Void = () => {}                    # 完全な注釈
num: () -> Int = () => 42                      # 完全な注釈
inc: (x: Int) -> Int = x + 1                   # 単一パラメータ省略形

# 部分推論（新構文）
add: (Int, Int) = (a, b) => a + b              # パラメータに注釈あり、戻り値は推論
square: (x: Int) -> Int = x * x                # パラメータに注釈あり、戻り値は推論
get_answer: () = () => 42                      # パラメータに注釈あり（空）、戻り値は推論

# 部分推論（旧構文、互換）
add2(Int, Int) = (a, b) => a + b               # パラメータに注釈あり、戻り値は推論
square2(Int) = (x) => x * x                    # パラメータに注釈あり、戻り値は推論

# return から推論
fact: Int -> Int = (n) => {
    if n <= 1 { return 1 }
    return n * fact(n - 1)
}

# === 推論失敗 ===

# パラメータが推論不可（解析は通過、型チェックは失敗）
add: (a: Int, b: Int) -> Int = a + b                          # ✗ パラメータに型なし
identity: (T: Type) -> ((x: T) -> T) = x                              # ✗ パラメータに型なし

# 明示的戻り値のないブロック
no_return = (x: Int) => { x }                  # ✗ ブロックに return なし、隠蔽的戻り値が推論不可

# 完全推論不可
bad_fn: (T: Type) -> ((x: T) -> T) = x                                # ✗ パラメータと戻り値が推論不可
```

#### 2.5.9 旧構文（後方互換）

YaoXiang は歴史的コードとの互換性のために旧構文サポートを提供します、**新コードでは非推奨**。

```
旧構文 ::= 識別子 '(' [パラメータ型リスト] ')' '->' 戻り値型 '=' Lambda
```

| 機能 | 標準構文 | 旧構文 |
|------|---------|--------|
| 宣言フォーマット | `name: Type = ...` | `name(Types) -> Type = ...` |
| パラメータ型の位置 | 型注釈内 | 関数名の後の括弧内 |
| 空パラメータ | `()` を書く必要がある | `()` を省略可能 |
| **推奨度** | ✅ **公式推奨** | ⚠️ **後方互換のみ** |
| **使用シナリオ** | すべての新規コード | 歴史的コードのメンテナンス |

**非推奨の理由：**

1. **学習コスト**：標準構文と一致せず、言語複雑度が増加
2. **一貫性**：パラメータ型の位置が統一されていない（一方は型注釈内、他方は関数名の後）
3. **メンテナンスコスト**：パーサは2つの形式を追加処理する必要がある
4. **AI友好性**：AIがコードを理解和生成更难

**移行の提案：**

```yaoxiang
# 古いコード（非推奨）
mul(Int, Int) -> Int = (a, b) => a * b
square(Int) -> Int = (x) => x * x
empty() -> Void = () => {}

# 新しいコード（推奨）
mul: (Int, Int) -> Int = (a, b) => a * b
square: (Int) -> Int = (x) => x * x
empty: () -> Void = () => {}
```

---

## 三、型システム

### 3.1 型の階層

YaoXiang の型システムは階層化されています：

```
┌─────────────────────────────────────────────────────────────┐
│                    YaoXiang 型階層                          │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  type (元型)                                                 │
│    │                                                        │
│    ├── プリミティブ型 (Primitive Types)                      │
│    │   ├── Void, Bool                                       │
│    │   ├── Int, Uint, Float                                  │
│    │   ├── Char, String, Bytes                               │
│    │                                                        │
│    ├── コンストラクタ型 (Constructor Types)                  │
│    │   ├── Name(args)              # 単一コンストラクタ（構造体）      │
│    │   ├── A(T) | B(U)             # 複数コンストラクタ（合併/列挙）   │
│    │   ├── A | B | C               # ゼロパラメータコンストラクタ（列挙）      │
│    │   ├── tuple (T1, T2, ...)                            │
│    │   ├── list(T), dict(K, V)                           │
│    │                                                        │
│    ├── 関数型 (Function Types)                              │
│    │   fn (T1, T2, ...) -> R                               │
│    │                                                        │
│    ├── ジェネリック型 (Generic Types)                        │
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
# 統一型構文：コンストラクタのみ、enum/struct/union キーワードなし
# ルール：| で区切的都是コンストラクタ、コンストラクタ名(引数) が型

# === ゼロパラメータコンストラクタ（列挙スタイル）===
type Color = { red | green | blue }              # red() | green() | blue() と等価

# === 複数パラメータコンストラクタ（構造体スタイル）===
type Point = { x: Float, y: Float }       # コンストラクタが型

# === ジェネリックコンストラクタ ===
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }           # ジェネリック合併

# === 混合コンストラクタ ===
type Shape = circle(Float) | rect(Float, Float)

# === 値構築（関数呼び出しと完全に同じ）===
c: Color = green                              # green() と等価
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

# === インターフェース実装（型の末尾にインターフェース名を列出）===
type Point = {
    x: Float,
    y: Float,
    Drawable,        # Drawable インターフェースを実装
    Serializable     # Serializable インターフェースを実装
}
```

### 3.3 型操作

```yaoxiang
# 型を値として
MyInt = Int
MyList = List(Int)

# 型リフレクション（コンストラクタパターンマッチング）
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

YaoXiang は強力な型推論能力を备えています：

```yaoxiang
# 基本推論
x = 42                    # Int と推論
y = 3.14                  # Float と推論
z = "hello"               # String と推論

# 関数戻り値の推論
add: (a: Int, b: Int) -> Int = a + b

# ジェネリック推論
first: (T: Type) -> ((list: List(T)) -> Option(T)) = (list) => {
    if list.length > 0 { some(list[0]) } else { none }
}
```

---

## 四、メモリ管理

### 4.1 所有権モデル

YaoXiang は**所有権モデル**を採用してメモリを管理し、各値に一意の所有者がいます：

```yaoxiang
# === デフォルト Move（ゼロコピー）===
p: Point = Point(1.0, 2.0)
p2 = p              # Move、所有権移転、p は無効

# === ref キーワード = Arc（安全共有）===
shared = ref p      # Arc、スレッドセーフ

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
    # p の所有権が移転
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
shared = ref p      # Arc、スレッドセーフ

spawn(() => print(shared.x))   # ✅ 安全

# Arc はライフタイムを自動管理
# shared がスコープを離れるとき、カウンタがゼロになり自動解放
```

### 4.4 clone() 明示的複製

```yaoxiang
# 元の値を保持する必要がある場合 clone() を使用
p: Point = Point(1.0, 2.0)
p2 = p.clone()   # p と p2 は独立

p.x = 0.0        # ✅
p2.x = 0.0       # ✅ 互いに影響なし
```

### 4.5 unsafe コードブロック（システムレベル）

```yaoxiang
# 裸ポインタは unsafe ブロック内でのみ使用可能
p: Point = Point(1.0, 2.0)

unsafe {
    ptr: *Point = &p     # 裸ポインタ
    (*ptr).x = 0.0       # ユーザーは安全を保証
}
```

### 4.6 RAII

```yaoxiang
# RAII 自動解放
with_file: (path: String) -> String = {
    file = File.open(path)  # 自動オープン
    content = file.read_all()
    # 関数終了、file は自動クローズ
    content
}
```

### 4.7 Send / Sync 制約

| 制約 | セマンティクス | 説明 |
|------|--------------|------|
| **Send** | スレッド間安全転送可能 | 値は別のスレッドに移動可能 |
| **Sync** | スレッド間安全共有可能 | 不変参照は別のスレッドに共有可能 |

```yaoxiang
# ref T は自動的に Send + Sync を満たす（Arc はスレッドセーフ）
p: Point = Point(1.0, 2.0)
shared = ref p

spawn(() => print(shared.x))   # ✅ Arc はスレッドセーフ

# 裸ポインタ *T は Send/Sync を満たさない
unsafe {
    ptr: *Point = &p         # 単一スレッド内でのみ使用可能
}
```

### 4.9 未実装

| 機能 | 理由 |
|------|------|
| ライフタイム `'a` | 参照概念がない、ライフタイム不要 |
| 借用チェッカー | ref = Arc で代替 |
| `&T` 借用構文 | Move セマンティクスを使用 |

---

## 五、非同期プログラミングと並行処理

> 「万物并作，吾以观复。」——《易·复卦》
>
> YaoXiang は**並作モデル**を採用します。これは**遅延評価**に基づく無感非同期並行パラダイムです。そのコア設計理念は：**開発者が同期的で顺序的な思考でロジックを記述し、言語ランタイムはその中の計算ユニットを万物並作のように自動的かつ効率的に並行実行させ、最終的に統一的に協調させる**ことです。

> 詳細については [『並作モデル白書』](YaoXiang-async-whitepaper.md) と [非同期実装方案](YaoXiang-async-implementation.md) を参照してください。

### 5.1 並作モデルコアコンセプト

#### 5.1.1 並作図：万物並作のステージ

すべてのプログラムはコンパイル時に**有向非巡回計算グラフ（DAG）**に変換されます。これを**並作図**と呼びます。ノードは式計算を表し、エッジはデータ依存関係を表します。このグラフは遅延的であり、ノードは出力が**実際に必要**になったときのみ評価されます。

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

`spawn fn` でマークされた関数呼び出しは、 즉시 `Async(T)` 型の値を返します。これは**並作値**と呼ばれます。これは実際の結果ではなく、**並作中の未来値**を表します。

**コア機能**：

- **型透過性**：`Async(T)` は型システムにおいて `T` のサブ型であり、`T` が期待される任意のコンテキストで使用可能
- **自動待機**：プログラムが `T` 型の具体的な値を使用する操作を実行する必要があるとき、ランタイムは現在のタスクを自動的に中断し、計算の完了を待機
- **ゼロ感染**：非同期コードと同期コードは構文と型シグネチャに違いなし

```yaoxiang
# 並作値使用例
fetch_data: spawn (String) -> JSON = (url) => { ... }

main: () -> Void = () => {
    data = fetch_data("url")  # Async(JSON)

    # Async(JSON) は直接 JSON として使用可能
    # 自動待機はフィールドアクセス時に発生
    print(data.name)          # data.await().name と等価
}
```

### 5.2 並作構文体系

`spawn` キーワードは三重の意味を持ち、同期思考と非同期実装を接続する唯一のブリッジです：

| 公式用語 | 構文形式 | セマンティクス | ランタイム動作 |
|----------|----------|--------------|---------------|
| **並作関数** | `spawn fn` | 並作実行に参加できる計算ユニットを定義 | その呼び出しは `Async(T)` を返す |
| **並作ブロック** | `spawn { a(), b() }` | 明示的に宣言された並行領域 | ブロック内タスクは強制的に並列実行 |
| **並作ループ** | `spawn for x in xs { ... }` | データ並列パラダイム | ループボディはすべての要素で並作実行 |

#### 5.2.1 並作関数

```yaoxiang
# spawn を使用して並作関数をマーク
# 構文は通常の関数と完全に一致、追加負担なし

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
    # 3つの独立計算を並行実行
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
# spawn for - データ並列ループ
# 各反復は独立したタスクとして並列実行

parallel_sum: (Int) -> Int spawn = (n) => {
    total = spawn for i in 0..n {
        fibonacci(i)          # 各反復を並列
    }
    total
}
```

#### 5.2.4 データ並列ループ

```yaoxiang
# spawn for - データ並列ループ
# 各反復は独立したタスクとして並列実行

parallel_sum: (Int) -> Int spawn = (n) => {
    total = spawn for i in 0..n {
        fibonacci(i)          # 各反復を並列
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
# 明示的な await なし、コンパイラが自動的に待機点を挿入

main: () -> Void = () => {
    # 自動並列：2つの独立したリクエストを並行実行
    users = fetch_users()      # Async(List(User))
    posts = fetch_posts()      # Async(List(Post))

    # 待機点は "+" 操作處に自動的に挿入
    count = users.length + posts.length

    # フィールドアクセスが待機をトリガー
    first_user = users[0]      # users が準備完了するまで待機
    print(first_user.name)
}

# 条件分岐内の待機
process_data: spawn () -> Void = () => {
    data = fetch_data()        # Async(Data)

    if data.is_valid {         # data が準備完了まで待機
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
    # Barrier 待機
}

# いずれか1つが完了したら待機
await_any: (T: Type) -> ((tasks: List(Async(T))) -> T) = {
    # 最初に完了した結果を返す
}

# タイムアウト制御
with_timeout: (T: Type) -> ((task: Async(T), timeout: Duration) -> Option(T)) = {
    # タイムアウトは None を返す
}
```

### 5.5 スレッドセーフティ：Send/Sync 制約

YaoXiang は Rust のような **Send/Sync 型制約**を採用してコンパイル時にデータ競合を排除し、スレッドセーフティを保証します。

#### 5.5.1 Send 制約

**Send**：型はスレッド間で安全に**所有権を移転**できます。

```yaoxiang
# 基本型は自動的に Send を満たす
# Int, Float, Bool, String はすべて Send

# 構造体は自動的に Send を導出
type Point = { x: Int, y: Float }
# Point は Send、Int と Float は Send だから

# 非 Send フィールドを含む型は Send ではない
type NonSend = NonSend(data: Rc(Int))
# Rc は Send ではない（参照カウントはアトミックではない）、したがって NonSend は Send ではない
```

#### 5.5.2 Sync 制約

**Sync**：型はスレッド間で安全に**参照を共有**できます。

```yaoxiang
# 基本型はすべて Sync
type Point = { x: Int, y: Float }
# &Point は Sync、&Int と &Float は Sync だから

# 内部的可変性を含む型
type Counter = Counter(value: Int, mutex: Mutex(Int))
# &Counter は Sync、Mutex が内部的可変性を 제공하는から
```

#### 5.5.3 spawn とスレッドセーフティ

```yaoxiang
# spawn はパラメータと戻り値が Send を満たすことを要求

# 有効：Data は Send
type Data = Data(value: Int)
task = spawn(|| => Data(42))

# 無効：Rc は Send ではない
type SharedData = SharedData(rc: Rc(Int))
# task = spawn(|| => SharedData(Rc.new(42))  # コンパイルエラー！

# 解決策：Arc（アトミック参照カウント）を使用
type SafeData = SafeData(value: Arc(Int))
task = spawn(|| => SafeData(Arc.new(42)))  # Arc は Send + Sync
```

#### 5.5.4 スレッドセーフティ型導出ルール

```yaoxiang
# 構造体型
type Struct(T1, T2) = Struct(f1: T1, f2: T2)

# Send 導出
Struct(T1, T2): Send ⇐ T1: Send かつ T2: Send

# Sync 導出
Struct(T1, T2): Sync ⇐ T1: Sync かつ T2: Sync

# 合併型
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

# Send 導出
Result(T, E): Send ⇐ T: Send かつ E: Send
```

#### 5.5.5 標準ライブラリスレッドセーフティ実装

| 型 | Send | Sync | 説明 |
|------|:----:|:----:|------|
| `Int`, `Float`, `Bool` | ✅ | ✅ | プリミティブ型 |
| `Arc(T)` | ✅ | ✅ | T: Send + Sync |
| `Mutex(T)` | ✅ | ✅ | T: Send |
| `RwLock(T)` | ✅ | ✅ | T: Send |
| `Channel(T)` | ✅ | ❌ | 送信側のみ Send |
| `Rc(T)` | ❌ | ❌ | 非アトミック参照カウント |
| `RefCell(T)` | ❌ | ❌ | 実行時借用チェック |

```yaoxiang
# スレッドセーフカウンタの例
type SafeCounter = SafeCounter(mutex: Mutex(Int))

main: () -> Void = () => {
    counter: Arc(SafeCounter) = Arc.new(SafeCounter(Mutex.new(0)))

    # 並行更新
    spawn(|| => {
        guard = counter.mutex.lock()  # Mutex はスレッドセーフティを提供
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
# ランタイムはそれらを専用のブロッキングスレッドプールに配置

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
# モジュールはファイルを境界として使用
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

# 特定の関数をインポート
use std.io.{ read_file, write_file }
```

---

## 七、メソッドバインディングとカリー化

YaoXiang は**純粋関数型設計**を採用し、先进のバインディングメカニズムを通じてシームレスなメソッド呼び出しとカリー化を実現します。`struct`、`class` などのキーワードを導入する必要がありません。

### 7.1 コア関数定義

すべての操作は通常の関数を通じて実装され、最初の引数は操作の主体と约定されます：

```yaoxiang
# === Point.yx (モジュール) ===

# 統一構文：コンストラクタが型
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

YaoXiang は名前空間ベースの自動バインディングをサポートし、**追加の宣言不要**です：

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

    # ✅ 自動バインディング：直接メソッド呼び出し
    result = p1.distance(p2)  # distance(p1, p2) に解決
}
```

**自動バインディングルール**：

- モジュール内で定義された関数
- 最初の引数型がモジュール名と一致する場合
- メソッド呼び出し構文を自動的にサポート

#### 7.2.2 バインディングなしオプション（デフォルト動作）

```yaoxiang
# === Vector.yx ===

type Vector = Vector(x: Float, y: Float, z: Float)

# 内部ヘルパー関数、自動バインディング不希望
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

    # ✅ 直接呼び出し必須（モジュール外部からは不可視）
}
```

### 7.3 位置ベースのバインディング構文

YaoXiang は**最もエレガントなバインディング構文**を提供し、位置マーク `[n]` を使用してバインディング位置を精确に制御します：

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
# 意味：メソッド呼び出し時、呼び出し元を func の [position] 引数にバインド

Point.distance = distance[0]      # 第1引数にバインド
Point.add = add[0]                 # 第1引数にバインド
Point.scale = scale[0]             # 第1引数にバインド
```

**セマンティクス解析**：

- `Point.distance = distance[0]`
  - `distance` 関数は2つの引数を持つ：`distance(Point, Point)`
  - `[0]` は呼び出し元を第1引数にバインド
  - 使用：`p1.distance(p2)` → `distance(p1, p2)`

#### 7.3.2 複数位置結合バインディング

```yaoxiang
# === Math.yx ===

# 関数：scale, point1, point2, extra1, extra2
calculate: (scale: Float, a: Point, b: Point, x: Float, y: Float) -> Float = { ... }

# === Point.yx ===

type Point = { x: Float, y: Float }

# 複数位置のバインディング
Point.calc1 = calculate[1, 2]      # scale と point1 をバインド
Point.calc2 = calculate[1, 3]      # scale と point2 をバインド
Point.calc3 = calculate[2, 3]      # point1 と point2 をバインド

# === main.yx ===

use Point

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

# 1. バインディング[1,2] - 残りは3,4,5
f1 = p1.calc1(2.0)  # scale=2.0, point1=p1 をバインド
# f1 は今は p2, x, y を必要とする
result1 = f1(p2, 10.0, 20.0)  # calculate(2.0, p1, p2, 10.0, 20.0)

# 2. バインディング[1,3] - 残りは2,4,5
f2 = p2.calc2(2.0)  # scale=2.0, point2=p2 をバインド
# f2 は今は point1, x, y を必要とする
result2 = f2(p1, 10.0, 20.0)  # calculate(2.0, p1, p2, 10.0, 20.0)

# 3. バインディング[2,3] - 残りは1,4,5
f3 = p1.calc3(p2)  # point1=p1, point2=p2 をバインド
# f3 は今は scale, x, y を必要とする
result3 = f3(2.0, 10.0, 20.0)  # calculate(2.0, p1, p2, 10.0, 20.0)
```

#### 7.3.3 残り引数の投入順序

**コアルール**：バインディング後、残りの引数は**元の関数の順序**で投入され、バインドされた位置はスキップされます。

```yaoxiang
# 関数 suppose: func(p1, p2, p3, p4, p5)

# 第1と第3引数をバインド
Type.method = func[1, 3]

# 呼び出し時：
method(p2_value, p4_value, p5_value)

# マッピング：
func(p1_bound, p2_value, p3_bound, p4_value, p5_value)
# 残り引数：2,4,5 は元の順序で投入
```

#### 7.3.4 型チェックの優位性

```yaoxiang
# ✅ 有効なバインディング
Point.distance = distance[0]          # distance(Point, Point)
Point.calc = calculate[1, 2]          # calculate(scale, Point, Point, ...)

# ❌ 無効なバインディング（コンパイラがエラーを報告）
Point.wrong = distance[5]             # 第5引数は存在しない
Point.wrong = distance[0]             # 引数は1から始まる
Point.wrong = distance[1, 2, 3, 4]    # 関数引数个数を超える
```

### 7.4 カリー化バインディングの細粒度制御

```yaoxiang
# === Math.yx ===

distance_with_scale: (scale: Float, a: Point, b: Point) -> Float = { ... }

# === Point.yx ===

type Point = { x: Float, y: Float }

# バインディング戦略：各位置を柔軟に制御
Point.distance = distance[0]                    # 基本バインディング
Point.distance_scaled = distance_with_scale[2]  # 第2引数にバインド

# === main.yx ===

use Point
use Math

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

# 1. 基本自動バインディング
d1 = p1.distance(p2)  # distance(p1, p2)

# 2. 異なる位置へのバインディング
f = p1.distance_scaled(2.0)  # 第2引数をバインド、残りは第1,3
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
Point.multiply = multiply_by_scale[2]  # 第2引数にバインド
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

# 自動バインディングは pub 関数のみに有効
p1.distance(p2)      # ✅ distance は pub、自動バインディング可能
# p1.internal_distance(p2)  # ❌ pub ではない、バインディング不可
```

#### 7.6.2 pub 自動バインディングメカニズム

`pub` で宣言された関数に対し、コンパイラは自動的に同ファイルで定義された型にバインドします：

```yaoxiang
# === Point.yx ===

type Point = { x: Float, y: Float }

# pub を使用して宣言、コンパイラが自動バインディング
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

pub translate: (self: Point, dx: Float, dy: Float) -> Point = {
    Point(self.x + dx, self.y + dy)
}

# コンパイラが自動的にバインディングを推断して実行：
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

1. 関数はモジュールファイルで定義（型と同じファイル）
2. 関数の引数にその型が含まれている
3. `pub` を使用してエクスポート
4. コンパイラが自動的に `Type.method = function[0]` を実行

**好处**：

- バインディング宣言を手動で記述する必要がない
- コードがより簡潔
- バインディングの忘れや間違いを避ける

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
# しかし自動バインディングは pub エクスポートされた関数のみが外部で有効

pub distance  # エクスポート、外部で自動バインディング可能
```

### 7.7 設計優位性のまとめ

| 機能 | 説明 |
|------|------|
| **ゼロ構文負担** | 自動バインディングに追加の宣言不要 |
| **位置精确制御** | `[n]` でバインディング位置を正確指定 |
| **複数位置結合** | `[1, 2, 3]` 複数パラメータバインディングをサポート |
| **型安全** | コンパイラがバインディング位置の有効性を検証 |
| **キーワード不要** | `bind` やその他のキーワード不要 |
| **柔軟カリー化** | 任意の引数位置バインディングをサポート |
| **pub 制御** | pub 関数のみ外部バインディング可能 |

### 7.8 従来のメソッドバインディングとの違い

| 従来の言語 | YaoXiang |
|-----------|----------|
| `obj.method(arg)` | `obj.method(arg)` |
| class/メソッド定義が必要 | 関数 + バインディング宣言のみ |
| 構文 `class { method() {} }` | 構文 `Type.method = func[n]` |
| 継承、多相 | 純粋関数型、継承なし |
| メソッドテーブル検索 | コンパイル時バインディング、実行時オーバーヘッドなし |

**コア優位性**：YaoXiang のバインディングは**コンパイル時メカニズム**であり、ゼロ実行時コストであり、同時に関数型プログラミングの純粋さと柔軟性を維持しています。

---

## 八、AI友好的設計

YaoXiang の構文設計は AI コード生成と修正のニーズを特に考慮しています：

### 8.1 設計原則

```yaoxiang
# AI友好設計目標：
# 1. 厳密な構造化、曖昧さのない構文
# 2. AST が清晰、位置特定が容易
# 3. 意味が明確、隠蔽動作なし
# 4. コードブロック境界が明確
# 5. 型情報が完全
```

### 8.2 厳密な構造化構文

#### 8.2.1 宣言構文のAI友好戦略

```yaoxiang
# === AI コード生成ベストプラクティス ===

# ✅ 推奨：完全な新構文宣言 + 型集中約束を使用
# AI は意図を正確に理解し、完全な型情報を生成

add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1
empty: () -> Void = {}

# ❌ 避ける：型注釈を省略または型が分散
# AI はパラメータ型を確定できず、誤ったコードを生成する可能性
add: (a: Int, b: Int) -> Int = a + b          # パラメータに型なし
identity: (T: Type) -> ((x: T) -> T) = x              # パラメータに型なし
add2: (a: Int, b: Int) -> Int = a + b  # 型が実装に分散

# ⚠️ 互換：旧構文は古いコードのメンテナンスのみに使用
# AI は新構文 + 型集中約束を優先的に生成すべき
mul(Int, Int) -> Int = (a, b) => a * b  # 新コードでは非推奨
```

**型集中約束のAI優位性：**

1. **シグネチャが一目で了然**：AI は宣言行を読むだけで完全な関数シグネチャを理解できる
2. **より安全な修正**：型の修正は宣言を変更するだけでよく、実装ボディに影響しない
3. **より簡単な生成**：AI はまず宣言を生成し、次に実装を埋めることができる
4. **カリー化友好**：明確なカリー化型シグネチャは AI の処理に適している

```yaoxiang
# AI 処理例
# 入力：実装ボディ (a, b) => a + b
# AI は宣言を見る：add: (Int, Int) -> Int
# 結論：パラメータ型は Int, Int、戻り値型は Int

# 比較：型が分散している場合
# 入力：実装ボディ (a: Int, b: Int) => a + b
# AI が必要：実装ボディを解析して型情報を抽出
# 結果：より複雑な処理ロジック、エラーが発生しやすい
```

#### 8.2.2 二重構文戦略とAI

| 構文タイプ | AI 生成戦略 | 使用シナリオ |
|-----------|-----------|------------|
| **新構文** | ✅ 優先生成、完全な型情報 | すべての新規コード生成 |
| **旧構文** | ⚠️ 古いコードのメンテナンス時のみ使用 | 歴史的コードの修正 |
| **無注釈** | ❌ 生成を避ける | 任何情況でも生成すべきではない |

#### 8.2.3 構文境界が明確

```yaoxiang
# AI 友好なコードブロック境界

# ✅ 明確な開始と終了マーク
function_name: (Type1, Type2) -> ReturnType = (param1, param2) => {
    # 関数ボディ
    if condition {
        do_something()
    } else {
        do_other()
    }
}

# ✅ 条件文には必ず波括弧が必要
if condition {
    # 条件ボディ
}

# ✅ 型定義が明確
type MyType = Type1 | Type2

# ❌ 避けるべき曖昧な写法
if condition    # 波括弧がない
    do_something()
```

#### 8.2.4 曖昧さのない構文制約

```yaoxiang
# AI 生成時に遵守すべき制約

# 1. 括弧の省略禁止
# ✅ 正しい
foo: (T: Type) -> ((x: T) -> T) = x
my_list = [1, 2, 3]

# ❌ エラー（禁止）
foo T { T }             # パラメータには括弧が必要
my_list = [1 2 3]       # リストにはカンマが必要

# 2. 戻り値型または推論可能な形式を明示的に指定
# ✅ 正しい
get_num: () -> Int = 42
get_num2: () = 42          # 戻り値型が推論可能

# ❌ エラー
get_bad = () => { 42 }           # ブロックに return がない、推論不可

# 3. パラメータには型注釈が必要（新構文）
# ✅ 正しい
add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1

# ❌ エラー
add: (a: Int, b: Int) -> Int = a + b            # パラメータに型なし
identity: (T: Type) -> ((x: T) -> T) = x                # パラメータに型なし
```

#### 8.2.5 AI 生成推奨パターン

```yaoxiang
# AI が関数を生成するときの標準テンプレート

# パターン1：完全な型注釈
function_name: (param1: ParamType1, param2: ParamType2, ...) -> ReturnType = {
    # 関数ボディ
    return expression
}

# パターン2：戻り値型推論
function_name: (param1: ParamType1, param2: ParamType2) = {
    # 関数ボディ
    return expression
}

# パターン3：単一パラメータ省略形
function_name: (param: ParamType) -> ReturnType = expression

# パターン4：パラメータなし関数
function_name: () -> ReturnType = expression

# パターン5：空関数
function_name: () -> Void = {}
```

### 8.3 エラーメッセージのAI友好性

```yaoxiang
# エラーメッセージは明確な修正提案を提供するべき

# AI 友好でないエラー
# Syntax error at token 'a'

# AI 友好的エラー
# Missing type annotation for parameter 'a'
# Suggestion: add ': Int' or similar type to '(a, b) => a + b'
# Correct version: add: (a: Int, b: Int) -> Int = a + b
```

---

## 九、型集中約束（コア設計哲学）

### 9.1 約束の概要

YaoXiang のコア設計約束は**「宣言優先、型集中」**です。この約束は言語の AI 友好性と開発効率の基盤です。

```yaoxiang
# ✅ コア約束：型情報が宣言行に統一されている
add: (a: Int, b: Int) -> Int = a + b

# ❌ 避ける：型情報が実装に分散している
add: (a: Int, b: Int) -> Int = a + b
```

### 9.2 約束の5つのコア優位性

#### 1. 構文の一貫性

```yaoxiang
# すべての宣言が同じフォーマットに従う
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
#   完全な関数シグネチャ

# 実装ボディはビジネスロジックに集中
# (a, b) => a + b  型を気にする必要はなく、機能の実装에만 集中
```

#### 3. AI友好性

```yaoxiang
# AI 処理フロー：
# 1. 宣言行を読む → 完全に関数シグネチャを理解
# 2. 実装を生成 → 型推論分析が不要
# 3. 型を修正 → 宣言行のみ変更、実装に影響なし

# 比較：型が分散している場合
add: (a: Int, b: Int) -> Int = a + b
# AI が必要：実装ボディを解析して型情報を抽出 → より複雑、エラーが発生しやすい
```

#### 4. より安全な修正

```yaoxiang
# パラメータ型の修正
# 原来: add: (a: Int, b: Int) -> Int = a + b
# 修正: add: (Float, Float) -> Float = (a, b) => a + b
# 実装ボディ: (a, b) => a + b  変更不要！

# 型が分散している場合：
# 原来: add: (a: Int, b: Int) -> Int = a + b
# 修正: add: (a: Float, b: Float) -> Float = a + b  # 2箇所修正が必要
```

#### 5. カリー化友好

```yaoxiang
# カリー化型シグネチャが一目で了然
add_curried: (a: Int) -> (b: Int) -> Int = a + b
#              └─────────────┘
#              カリー化シグネチャ

# 関数合成を第一級市民として
compose: (Int -> Int, Int -> Int) -> Int -> Int = (f, g) => x => f(g(x))
```

### 9.3 約束の実施ルール

#### ルール1：パラメータは宣言で型を指定する必要がある

```yaoxiang
# ✅ 正しい
add: (a: Int, b: Int) -> Int = a + b

# ❌ エラー
add: (a: Int, b: Int) -> Int = a + b            # パラメータ型がない
identity: (T: Type) -> ((x: T) -> T) = x                # パラメータ型がない
```

#### ルール2：戻り値型は推論可能だが推奨は注釈付き

```yaoxiang
# ✅ 推奨：完全な注釈
get_num: () -> Int = () => 42

# ✅ 許容可能：戻り値型推論
get_num: () = () => 42

# ✅ 空関数は Void と推論
empty: () = () => {}
```

#### ルール3：Lambda 内部の型注釈は一時的なもの

```yaoxiang
# ✅ 正しい：宣言の型に依存
add: (a: Int, b: Int) -> Int = a + b

# ⚠️ 可能だが非推奨：Lambda 内で重複注釈
add: (Int, Int) -> Int = (a: Int, b: Int) => a + b

# ❌ エラー：宣言注釈がない
add: (a: Int, b: Int) -> Int = a + b
```

#### ルール4：旧構文は同じ理念に従う

```yaoxiang
# 旧構文も宣言位置で型情報を提供するよう尽量
# フォーマットは異なるが、理念は一貫：
# - 宣言行が主要な型情報を含む
# - 実装ボディは比較的簡潔
add(Int, Int) -> Int = (a, b) => a + b
```

### 9.4 約束と型推論の関係

```yaoxiang
# 約束は型推論を妨げるのではなく、推論の方向を導く

# 1. 完全な注釈（推論なし）
add: (a: Int, b: Int) -> Int = a + b

# 2. 部分推論（宣言がパラメータ型を提供）
add: (Int, Int) = (a, b) => a + b  # 戻り値型は推論

# 3. 空関数の推論
empty: () = () => {}  # () -> Void と推論
```

### 9.5 約束のAI実装優位性

**AI コード生成フロー：**

1. **要件を読む** → 宣言を生成
   ```
   要件：加算関数
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

**約束のない AI 処理との比較：**

```
要件：加算関数
AI に必要：
  1. パラメータ型を推論
  2. 戻り値型を推論
  3. 実装ボディを生成
  4. 一貫性を検証
  5. 型変更時の複雑な更新を処理

結果：より複雑、よりエラーが発生しやすい
```

### 9.6 約束の哲学的意味

この約束は YaoXiang の核心理念を体現しています：

- **宣言即ドキュメント**：宣言行が完全な関数ドキュメント
- **型即契約**：型情報は呼び出し側と実装者の間の契約
- **ロジック即実装**：実装ボディは「何をすべきか」のみに集中し、「何の型か」を気にしない
- **ツール即補助**：型システム、AI ツールは明確な宣言に基づいて動作可能

### 9.7 實際应用の比較

#### 完全な例：計算機モジュール

```yaoxiang
# === 推奨方法：型集中約束 ===

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

# 型定義
type Point = { x: Float, y: Float }
pub distance: (a: Point, b: Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    (dx * dx + dy * dy).sqrt()
}

# === 非推奨方法：型が分散 ===

# パラメータ型が Lambda 内にある
add: (a: Int, b: Int) -> Int = a + b
multiply = (a: Int, b: Int) => a * b

# 高階関数の型が分散
apply_twice = (f: (Int) -> Int, x: Int) => f(f(x))

# カリー化の型が分散
make_adder = (x: Int) => (y: Int) => x + y

# ジェネリック型が分散
map: (A: Type, B: Type) -> ((f: (A) -> B, xs: List(A)) -> List(B)) = case xs of
  [] => []
  (x :: rest) => f(x) :: map(f, rest)
```

#### コードメンテナンスの比較

```yaoxiang
# 要件：add を Int から Float に変更

# === 推奨方法：宣言行のみ変更 ===
# 原来
add: (a: Int, b: Int) -> Int = a + b

# 修正後
add: (a: Float, b: Float) -> Float = a + b
#              ↑↑↑↑↑↑↑↑↑          ↑↑↑↑↑↑↑
#              宣言行修正          実装ボディは変更なし

# === 非推奨方法：複数箇所変更が必要 ===
# 原来
add: (a: Int, b: Int) -> Int = a + b

# 修正後
add: (a: Float, b: Float) -> Float = a + b
#     ↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑
#     すべてのパラメータ型を変更する必要がある
```

#### AI 支援プログラミングの比較

```yaoxiang
# AI 要件：2点間のマンハッタン距離を計算する関数を実装

# === AI が推奨写法を見ると ===
type Point = { x: Float, y: Float }
pub manhattan: (a: Point, b: Point) -> Float = ???  # AI は完全なシグネチャを直接理解

# AI 生成：
pub manhattan: (a: Point, b: Point) -> Float = {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}

# === AI が非推奨写法を見ると ===
type Point = { x: Float, y: Float }
pub manhattan = ???  # AI は推論が必要：パラメータ型？戻り値型？

# AI が生成する可能性：
pub manhattan = (a: Point, b: Point) => Float => {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}
# または型情報が完全でないためエラー
```

### 9.8 約束実施チェックリスト

YaoXiang コードを编写的时候，可以使用以下のチェックリスト：

- [ ] すべての関数宣言に完全な型注釈が宣言行にある
- [ ] パラメータ型は宣言で指定、Lambda 内にはない
- [ ] 戻り値型はできるだけ宣言で注釈する
- [ ] 変数宣言は `name: Type = value` フォーマットを使用
- [ ] Lambda ボディは簡潔に保ち、型情報を重複させない
- [ ] 新構文を使用、旧構文は使用しない
- [ ] 複雑な型は type 定義を使用して宣言を明晰に保つ

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
x = 42                    # Int と自動推論
name = "YaoXiang"         # String と自動推論
pi = 3.14159              # Float と自動推論

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

### 10.3 メソッドバインディングの例

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

### 10.4 カリー化バインディングの例

```yaoxiang
# === Math.yx ===

distance_with_scale: (scale: Float, a: Point, b: Point) -> Float = {
    dx = (p1.x - p2.x) * scale
    dy = (p1.y - p2.y) * scale
    (dx * dx + dy * dy).sqrt()
}

# === Point.yx ===

type Point = { x: Float, y: Float }

Point.distance_scaled = distance_with_scale[2]  # 第2引数にバインド

# === main.yx ===

use Point

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

# バインディングを使用
f = p1.distance_scaled(2.0)  # scale と p1 をバインド
result = f(p2)               # 最終呼び出し

# または直接使用
result2 = p1.distance_scaled(2.0, p2)
```

### 10.5 次へ

- [言語仕様](./YaoXiang-language-specification.md) を参照して完全な構文を学ぶ
- [サンプルコード](./examples/) を参照して常用パターンを学ぶ
- [実装計画](./YaoXiang-implementation.md) を参照して技術的詳細を学ぶ

---

## 付録

### A. キーワードとアノテーション

| キーワード | 作用 |
|-----------|------|
| `type` | 型定義 |
| `pub` | 公開エクスポート |
| `use` | モジュールのインポート |
| `spawn` | 非同期マーク（関数/ブロック/ループ） |
| `ref` | 不変参照 |
| `mut` | 可変参照 |
| `if/elif/else` | 条件分岐 |
| `match` | パターンマッチング |
| `while/for` | ループ |
| `return/break/continue` | 制御フロー |
| `as` | 型変換 |
| `in` | メンバーアクセス |

| アノテーション | 作用 |
|---------------|------|
| `@block` | 完全同期コードとしてマーク |
| `@eager` | 緊急評価が必要な式としてマーク |
| `@Send` | Send 制約を満たすことを明示的に宣言 |
| `@Sync` | Sync 制約を満たすことを明示的に宣言 |

### B. 設計インスピレーション

- **Rust**：所有権モデル、ゼロコスト抽象
- **Python**：構文スタイル、読みやすさ
- **Idris/Agda**：依存型、型駆動開発
- **TypeScript**：型注釈、実行時型

---

## バージョン履歴

| バージョン | 日付 | 著者 | 変更説明 |
|------------|------|------|----------|
| v1.0.0 | 2024-12-31 | 晨煦 | 初期バージョン |
| v1.1.0 | 2025-01-04 | 沫郁酱 | ジェネリック構文を `<T>` から `[T]` に修正；`fn` キーワードを移除；関数定義の例をを更新 |
| v1.2.0 | 2025-01-06 | 晨煦 | 新構文フォーマットに統一：name: type -> type = lambda |
| v1.3.0 | 2025-01-20 | 晨煦 | 統一型構文を追加（RFC-010）：インターフェース定義には波括弧 `{ serialize: () -> String }` を使用；型の末尾にインターフェース名を列出してインターフェースを実装；`pub` 自動バインディングメカニズムを追加 |

---

> 「道生一，一生二，二生三，三生万物。」
> —— 『道徳経』
>
> 型は道であり、万物はこれから生まれる。
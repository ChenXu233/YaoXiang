# YaoXiang（爻象）プログラミング言語ガイド

> バージョン: v1.2.0
> ステータス: ドラフト
> 著者: 晨煦
> 日付: 2024-12-31
> 更新: 2025-01-20 - 位置インデックスを 0 から開始（RFC-004）；型構文の統一（RFC-010）

---

## 目次

1. [言語概要](#一言語概要)
2. [コア機能](#二コア機能)
3. [型システム](#三型システム)
4. [メモリ管理](#四メモリ管理)
5. [非同期プログラミングと並行処理](#五非同期プログラミングと並行処理)
6. [モジュールシステム](#六モジュールシステム)
7. [メソッドバインディングとカリー化](#七メソッドバインディングとカリー化)
8. [AIフレンドリー設計](#八aiフレンドリー設計)
9. [型集中規約](#九型集中規約中核設計思想)
10. [クイックスタート](#十クイックスタート)

---

**拡張ドキュメント**:
- [高度なバインディング機能とコンパイラ実装](../works/plans/bind/YaoXiang-bind-advanced.md) - バインディング機構の詳細、高度な機能、コンパイラ実装、エッジケース処理

---

## 一、言語概要

### 1.1 YaoXiangとは？

YaoXiang（爻象）は実験的な汎用プログラミング言語であり、その設計理念は『易経』の「爻」と「象」の中核概念に由来します。「爻」は卦象を構成する基本記号であり、陰陽変化を象徴します。「象」は事物の本質の外在的表現であり、万象万物を代表します。

YaoXiang はこの哲学的思考をプログラミング言語の型システムに融合させ、**「すべては型である」**という中核理念を提起します。YaoXiang の世界観では：

- **値**は型のインスタンスである
- **型**自体も型のインスタンスである（メタ型）
- **関数**は入力型から出力型への写像である
- **モジュール**は型の名前空間の組み合わせである

### 1.2 設計目標

YaoXiang の設計目標は以下の側面にまとめられます：

| 目標 | 説明 |
|------|------|
| **統一された型抽象** | 型は最高層の抽象単位であり、言語セマンティクスを簡素化する |
| **自然なプログラミング体験** | Python スタイルの構文、可読性を重視 |
| **安全なメモリ管理** | Rust スタイルの所有権モデル、GC なし |
| **無感覚な非同期プログラミング** | 非同期を自動管理、明示的な await 不要 |
| **完全な型リフレクション** | ランタイム型情報は完全に利用可能 |
| **AI フレンドリーな構文** | 厳密に構造化、 AI 処理が容易 |

### 1.3 言語の位置付け

| 次元 | 位置付け |
|------|------|
| パラダイム | マルチパラダイム（関数型 + 命令型 + オブジェクト指向） |
| 型システム | 依存型 + パラメータ化多相 |
| メモリ管理 | 所有権 + RAII（GC なし） |
| コンパイルモデル | AOT コンパイル（JIT オプション） |
| 対象シナリオ | システムプログラミング、アプリケーション開発、AI 支援プログラミング |

### 1.4 コード例

```yaoxiang
// 自動型推論
x: Int = 42 // 明示的な型
y = 42 // Int として推論
name = "YaoXiang" // String として推論

// デフォルトでイミュータブル
x: Int = 10
x = 20 // ❌ コンパイルエラー！イミュータブル

// 統一された宣言構文：識別子: 型 = 式
add: (a: Int, b: Int) -> Int = a + b // 関数宣言
inc: (x: Int) -> Int = x + 1 // 単一パラメータ関数

// 統一された型構文：コンストラクタがそのまま型
Point: Type = { x: Float, y: Float }
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

// 無感覚な非同期（spawn関数）
fetch_data: (url: String) -> JSON spawn = {
 HTTP.get(url).json()
}

main: () -> Void = {
 // 値構築：関数呼び出しと完全に同じ
 p = Point(3.0, 4.0)
 r = ok("success")

 data = fetch_data("https://api.example.com")
 // 自動待機、await 不要
 print(data.name)
}

// ジェネリック関数
identity: (T: Type) -> ((x: T) -> T) = x

// 高階関数
apply: (f: (Int) -> Int, x: Int) -> Int = f(x)

// カリー化
add_curried: (a: Int) -> (b: Int) -> Int = a + b
```

---

## 二、コア機能

### 2.1 すべては型である

YaoXiang の中核設計哲学は**すべては型である**ということです。これは YaoXiang において以下を意味します：

1. **値は型のインスタンス**：`42` は `Int` 型のインスタンスである
2. **型は型のインスタンス**：`Int` は `type` メタ型のインスタンスである
3. **関数は型の間の写像**：`add: (Int, Int) -> Int` は関数型である
4. **モジュールは型の組み合わせ**：モジュールは関数と型を含む名前空間である

```yaoxiang
// 値は型のインスタンス
x: Int = 42

// 型は型のインスタンス
MyList: type = List(Int)

// 関数は型の間の写像
add: (a: Int, b: Int) -> Int = a + b

// モジュールは型の組み合わせ（ファイルがモジュールになる）
// Math.yx
pi: Float = 3.14159
sqrt: (x: Float) -> Float = { ... }
```

### 2.2 数学的抽象

YaoXiang の型システムは型論と圏論に基づき、以下を提供します：

- **依存型**：型は値に依存できる
- **ジェネリックプログラミング**：型のパラメータ化
- **型の組み合わせ**：ユニオン型、交差型

```yaoxiang
// 依存型：固定長ベクトル
Vector: (T: Type, n: Int) -> Type = vector(T, n)

// ユニオン型
Number: Type = { Int | Float }

// 交差型
Printable: Type = printable(() -> String)
Serializable: Type = serializable(() -> String)
Versatile: Type = Printable & Serializable
```

### 2.3 ゼロコスト抽象

YaoXiang はゼロコスト抽象を保証します。すなわち、高水準の抽象はランタイム性能のオーバーヘッドをもたらしません：

- **単態化**：ジェネリック関数はコンパイル時に具体バージョンに展開される
- **インライン化最適化**：単純な関数は自動的にインライン化される
- **スタック割り当て**：小さなオブジェクトはデフォルトでスタック割り当て

```yaoxiang
// ジェネリックの展開（単態化）
identity: (T: Type) -> ((x: T) -> T) = x

// 使用
int_val = identity(42) // identity(Int) -> Int に展開
str_val = identity("hello") // identity(String) -> String に展開

// コンパイル後の追加オーバーヘッドなし
```

### 2.4 自然な構文

YaoXiang は Python スタイルの構文を採用し、可読性と自然言語感を追求します：

```yaoxiang
// 自動型推論
x = 42
name = "YaoXiang"

// 簡潔な関数定義
greet: (name: String) -> String = "Hello, " + name

// パターンマッチング
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

YaoXiang は統一された宣言構文を採用します：**識別子: 型 = 式**。同時に後方互換性のある旧構文も提供します。

#### 2.5.1 二重構文戦略と型集中規約

革新性と互換性のバランスを取るため、YaoXiang は2つの構文形式をサポートしますが、統一された**型集中注釈規約**を採用します。

**構文形式の比較：**

| 構文タイプ | フォーマット | ステータス | 説明 |
|---------|------|------|------|
| **新構文（標準）** | `name: Type = Lambda` | ✅ 推奨 | 公式標準、新規コードはすべてこの形式を使用すべき |
| **旧構文（互換）** | `name(Types) -> Ret = Lambda` | ⚠️ 互換のみ | 歴史的コード用に保持、新規プロジェクトでは非推奨 |

**中核規約：型集中注釈**

YaoXiang は**「宣言優先、型集中」**という設計規約を採用します：

```yaoxiang
// ✅ 正しい：型情報は宣言行で統一
add: (a: Int, b: Int) -> Int = a + b
// └─────────────────┘ └─────────────┘
// 完全な型シグネチャ     実装ロジック

// ❌ 避ける：型情報が実装に分散
add: (a: Int, b: Int) -> Int = a + b
// └───────────────┘
// 型が実装本体に混在
```

**規約の利点：**

1. **構文の一貫性**：すべての宣言が `識別子: 型 = 式` に従う
2. **宣言と実装の分離**：型情報が一目でわかる、実装本体はロジックに集中
3. **AI フレンドリー性**：AI は宣言行を読むだけで完全な関数シグネチャを理解できる
4. **より安全な変更**：型を変更するには宣言を変更するだけで実装本体に影響しない
5. **カリー化フレンドリー**：明確なカリー化型シグネチャをサポート

**選択の推奨**：
- **新規プロジェクト**：新構文 + 型集中規約を必ず使用
- **移行プロジェクト**：新構文と型集中規約に段階的に移行
- **旧コードの保守**：旧構文を使い続けてもよいが、型集中規約の採用を推奨

#### 2.5.2 基本宣言構文

```yaoxiang
// === 新構文（推奨）===
// すべての宣言は次に従う：識別子: 型 = 式

// 変数宣言
x: Int = 42
name: String = "YaoXiang"
mut counter: Int = 0

// 関数宣言
add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1
getAnswer: () -> Int = 42
log: (msg: String) -> Void = print(msg)

// === 旧構文（互換）===
// 関数のみ使用、フォーマット：name(Types) -> Ret = Lambda
add(Int, Int) -> Int = (a, b) => a + b
square(Int) -> Int = (x) => x * x
empty() -> Void = () => {}
getRandom() -> Int = () => 42
```

#### 2.5.3 関数型構文

```
関数型 ::= '(' パラメータ型リスト ')' '->' 戻り型
         | パラメータ型 '->' 戻り型              # 単一パラメータ短縮形

パラメータ型リスト ::= [型 (',' 型)*]
戻り型 ::= 型 | 関数型 | 'Void'

# 関数型はファーストクラス市民であり、ネストできる
# 高階関数型 ::= '(' 関数型 ')' '->' 戻り型
```

| 例 | 意味 |
|------|------|
| `Int -> Int` | 単一パラメータ関数型 |
| `(Int, Int) -> Int` | 二重パラメータ関数型 |
| `() -> Void` | パラメータなし関数型 |
| `(Int -> Int) -> Int` | 高階関数：関数を受け取り、Int を返す |
| `Int -> Int -> Int` | カリー化関数（右結合） |

#### 2.5.4 ジェネリック構文（型パラメータのみ）

```yaoxiang
// ジェネリック関数：<型パラメータ> 接頭辞
identity: (T: Type) -> ((x: T) -> T) = x
map: (A: Type, B: Type) -> ((f: (A) -> B, xs: List(A)) -> List(B)) = case xs of
 [] => []
 (x :: rest) => f(x) :: map(f, rest)

// ジェネリック型
List: (T: Type) -> Type
```

#### 2.5.5 Lambda 式構文

```
Lambda ::= '(' パラメータリスト ')' '=>' 式
         | パラメータ '=>' 式              # 単一パラメータ短縮形

パラメータリスト ::= [パラメータ (',' パラメータ)*]
パラメータ ::= 識別子 [':' 型]               # オプションの型注釈
```

| 例 | 意味 | 説明 |
|------|------|------|
| `(a, b) => a + b` | 複数パラメータ Lambda | 宣言と組み合わせて使用：<br>`add: (Int, Int) = (a, b) => a + b` |
| `x => x + 1` | 単一パラメータ短縮形 | 宣言と組み合わせて使用：<br>`inc: Int = x => x + 1` |
| `(x: Int) => x + 1` | 型注釈付き | Lambda 内部で一時的に型情報が必要な場合のみ |
| `() => 42` | パラメータなし Lambda | 宣言と組み合わせて使用：<br>`get: () = () => 42` |

**注意**：Lambda 式の型注釈 `(x: Int) => ...` は**一時的な、局所的な**ものであり、主に以下に使用されます：
- Lambda 内部で型情報が必要な場合
- 宣言構文と組み合わせて使用する場合（型は宣言で既に提供）
- 主要な型宣言方式として使用するべきではない

#### 2.5.6 完全な例

```yaoxiang
// === 基本関数宣言 ===

// 基本関数（新構文）
add: (a: Int, b: Int) -> Int = a + b

// 単一パラメータ関数（2つの形式）
inc: (x: Int) -> Int = x + 1
inc2: (x: Int) -> Int = x + 1

// パラメータなし関数
getAnswer: () -> Int = 42

// 戻り値なし関数
log: (msg: String) -> Void = print(msg)

// === 再帰関数 ===
// 再帰は lambda で自然にサポートされる
fact: (n: Int) -> Int =
 if n <= 1 then 1 else n * fact(n - 1)

// === 高階関数と関数型代入 ===

// ファーストクラス市民としての関数型
IntToInt: Type = (Int) -> Int
IntBinaryOp: Type = (Int, Int) -> Int

// 高階関数宣言
applyTwice: (f: IntToInt, x: Int) -> Int = f(f(x))

// カリー化関数
addCurried: (a: Int) -> (b: Int) -> Int = a + b

// 関数合成
compose: (f: Int -> Int, g: Int -> Int) -> (x: Int) -> Int =
 f(g(x))

// 関数を返す関数
makeAdder: (x: Int) -> (y: Int) -> Int =
 x + y

// === ジェネリック関数 ===

// ジェネリック関数
identity: (T: Type) -> ((x: T) -> T) = x

// ジェネリック高階関数
map: (A: Type, B: Type) -> ((f: (A) -> B, xs: List(A)) -> List(B)) =
 case xs of
 [] => []
 (x :: rest) => f(x) :: map(f, rest)

// ジェネリック関数型
Transformer: Type = (A: Type, B: Type) -> ((A) -> B)

// ジェネリック型の使用
applyTransformer: (A: Type, B: Type) -> ((f: Transformer(A, B), x: A) -> B) =
 f(x)

// === 複雑な型の例 ===

// ネストした関数型
higherOrder: (A: Type) -> ((f: (A) -> Int) -> (A) -> Int) =
 f => x => f(x) + 1

// 複数パラメータ高階関数
zipWith: (A: Type, B: Type, C: Type) -> ((f: (A, B) -> C, xs: List(A), ys: List(B)) -> List(C)) =
 case (xs, ys) of
 ([], _) => []
 (_, []) => []
 (x::xs', y::ys') => f(x, y) :: zipWith(f, xs', ys')

// 関数型エイリアス
Predicate: (T: Type) -> Type = { apply: (T) -> Bool }
Mapper: Type = (A: Type, B: Type) -> ((A) -> B)
Reducer: Type = (A: Type, B: Type) -> ((B, A) -> B)

// === 旧構文の例（後方互換のみ） ===
// 新規コードでは非推奨

mul(Int, Int) -> Int = (a, b) => a * b // 複数パラメータ
square(Int) -> Int = (x) => x * x // 単一パラメータ
empty() -> Void = () => {} // パラメータなし
get_random() -> Int = () => 42 // 戻り値あり

// 同等の新構文（推奨）
mul: (a: Int, b: Int) -> Int = a * b
square: (x: Int) -> Int = x * x
empty: () -> Void = {}
get_random: () -> Int = 42
```

#### 2.5.7 構文解析ルール

**型解析の優先順位：**

| 優先順位 | 型 | 説明 |
|--------|------|------|
| 1 (最高) | ジェネリック適用 `List(T)` | 左結合 |
| 2 | 括弧 `(T)` | 結合性を変更 |
| 3 | 関数型 `->` | 右結合 |
| 4 (最低) | 基本型 `Int, String` | 原子型 |

**型解析の例：**

```yaoxiang
// (A -> B) -> C -> D
// 次のように解析される: ((A -> B) -> (C -> D))

// A -> B -> C
// 次のように解析される: (A -> (B -> C)) // 右結合

// (Int -> Int) -> Int
// 次のように解析される: 関数を受け取り、Int -> Int を返す

// List<Int -> Int>
// 次のように解析される: List の要素型は Int -> Int
```

**Lambda 解析の例：**

```yaoxiang
// a => b => a + b
// 次のように解析される: a => (b => (a + b)) // 右結合、カリー化

// (a, b) => a + b
// 次のように解析される: 2つのパラメータを受け取り、a + b を返す
```

#### 2.5.8 型推論ルール

YaoXiang は**二重処理**戦略を採用します：解析層は寛容に、型検査層は厳密に推論します。

**解析層のルール：**
- パーサは構文構造のみを検証し、型推論を行わない
- 型注釈のない宣言では、型注釈フィールドは `None` になる
- 基本的な構文構造に従うすべての宣言は解析を通過する
- **重要**：`add: (a: Int, b: Int) -> Int = a + b` は解析層では**合法**である

**型検査層のルール：**
- パラメータには型注釈が必須：これは強制要件
- 戻り型は推論可能だが、パラメータ型は明示的に宣言しなければならない

**完全な型推論ルール：**

| シナリオ | パラメータ推論 | 戻り推論 | 解析結果 | 型検査結果 | 推奨度 |
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
| **戻り注釈なしのブロック** | - | ブロック内容から推論 | ✅ | ✅ | ⭐⭐⭐⭐ |
| `main = () => {}` | | | | | |
| `get = () => { return 42; }` | | | | | |
| **return なしコードブロック** | - | デフォルトで `Void` | ✅ | ✅ 正しい | ✅ 正しい |
| `add: (a: Int, b: Int) -> Int = { return a + b }` | | | | | |

**詳細な推論ルール：**

```
解析層：構文構造のみを見る
├── 構造が正しい → 通過
└── 構造が誤り → エラー

型検査層：セマンティクスを検証
├── パラメータ型推論
│   ├── パラメータに型注釈あり → 注釈型を使用 ✅
│   ├── パラメータに型注釈なし → 拒否 ❌
│   └── Lambda パラメータは注釈が必須 → 強制要件
│
├── 戻り型推論
│   ├── return expr あり → expr から推論 ✅
│   ├── return なし、式あり → 式から推論 ✅
│   ├── コードブロックに return なし → デフォルトで Void を返す ✅
│   └── 推論不可 → 拒否 ❌
│
└── 完全に推論不可 → 拒否 ❌
```

**注意**：コードブロック内に `return` がない場合、デフォルトで `Void` を返します。例えば：
- `() => { 42 }` は `() -> Void` として推論される（ブロックに return なし、デフォルトで Void を返す）
- `() => { return 42 }` は `() -> Int` として推論される（return あり、return から推論）
- `() => 42` は `() -> Int` として推論される（式形式、直接値を返す）

**推論の例：**

```yaoxiang
// === 推論成功 ===

// 標準形式
main: () -> Void = () => {} // 完全な注釈
num: () -> Int = () => 42 // 完全な注釈
inc: (x: Int) -> Int = x + 1 // 単一パラメータ短縮形

// 部分推論（新構文）
add: (Int, Int) = (a, b) => a + b // パラメータ注釈あり、戻り推論
square: (x: Int) -> Int = x * x // パラメータ注釈あり、戻り推論
get_answer: () = () => 42 // パラメータ注釈あり（空）、戻り推論

// 部分推論（旧構文、互換）
add2(Int, Int) = (a, b) => a + b // パラメータ注釈あり、戻り推論
square2(Int) = (x) => x * x // パラメータ注釈あり、戻り推論

// return から推論
fact: Int -> Int = (n) => {
 if n <= 1 { return 1 }
 return n * fact(n - 1)
}

// === 推論失敗 ===

// パラメータ推論不可（解析通過、型検査失敗）
add: (a: Int, b: Int) -> Int = a + b // ✗ パラメータに型なし
identity: (T: Type) -> ((x: T) -> T) = x // ✗ パラメータに型なし

// コードブロックに return なし
no_return = (x: Int) => { x } // ✓ Void として推論（ブロックに return なし、デフォルトで Void を返す）

// 全体が推論不可
bad_fn: (T: Type) -> ((x: T) -> T) = x // ✗ パラメータと戻り値がどちらも推論不可
```

#### 2.5.9 旧構文（後方互換）

YaoXiang は歴史的コードとの互換性のために旧構文をサポートしますが、**新規コードでは非推奨**です。

```
旧構文 ::= 識別子 '(' [パラメータ型リスト] ')' '->' 戻り型 '=' Lambda
```

| 特性 | 標準構文 | 旧構文 |
|------|---------|--------|
| 宣言フォーマット | `name: Type = ...` | `name(Types) -> Type = ...` |
| パラメータ型の位置 | 型注釈内 | 関数名後の括弧内 |
| 空パラメータ | `()` を必ず記述 | `()` は省略可能 |
| **推奨度** | ✅ **公式推奨** | ⚠️ **後方互換のみ** |
| **使用シナリオ** | すべての新規コード | 歴史的コードの保守 |

**非推奨の理由：**
1. **学習コスト**：標準構文と一貫性がないため、言語の複雑さが増す
2. **一貫性**：パラメータ型の位置が統一されていない（一方は型注釈内、もう一方は関数名後）
3. **保守コスト**：パーサが2つの形式を余分に処理する必要がある
4. **AI フレンドリー性**：AI がコードを理解・生成する難易度が増す

**移行の提案：**
```yaoxiang
// 旧コード（非推奨）
mul(Int, Int) -> Int = (a, b) => a * b
square(Int) -> Int = (x) => x * x
empty() -> Void = () => {}

// 新コード（推奨）
mul: (Int, Int) -> Int = (a, b) => a * b
square: (Int) -> Int = (x) => x * x
empty: () -> Void = () => {}
```

---

## 三、型システム

### 3.1 型階層

YaoXiang の型システムは階層化されています：

```
┌─────────────────────────────────────────────────────────────┐
│                    YaoXiang 型階層                          │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  type (メタ型)                                               │
│    │                                                        │
│    ├── プリミティブ型 (Primitive Types)                        │
│    │   ├── Void, Bool                                       │
│    │   ├── Int, Uint, Float                                 │
│    │   ├── Char, String, Bytes                              │
│    │                                                        │
│    ├── コンストラクタ型 (Constructor Types)                    │
│    │   ├── Name(args)              # 単一コンストラクタ（構造体）│
│    │   ├── A(T) | B(U)             # 複数コンストラクタ（ユニオン/列挙）│
│    │   ├── A | B | C               # ゼロ引数コンストラクタ（列挙）│
│    │   ├── tuple (T1, T2, ...)                            │
│    │   ├── list(T), dict(K, V)                           │
│    │                                                        │
│    ├── 関数型 (Function Types)                                │
│    │   fn (T1, T2, ...) -> R                               │
│    │                                                        │
│    ├── ジェネリック型 (Generic Types)                         │
│    │   List(T), Map(K, V), etc.                            │
│    │                                                        │
│    ├── 依存型 (Dependent Types)                               │
│    │   (n: Int) -> Type                               │
│    │                                                        │
│    └── モジュール型 (Module Types)                            │
│        ファイルがモジュール                                    │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 型定義

```yaoxiang
// 統一された型構文：コンストラクタのみ、enum/struct/union キーワードなし
// ルール：| で区切られたものはすべてコンストラクタ、コンストラクタ名(パラメータ) が型になる

// === ゼロ引数コンストラクタ（列挙スタイル）===
Color: Type = { red | green | blue } // red() | green() | blue() と等価

// === 複数引数コンストラクタ（構造体スタイル）===
Point: Type = { x: Float, y: Float } // コンストラクタがそのまま型

// === ジェネリックコンストラクタ ===
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) } // ジェネリックユニオン

// === 混合コンストラクタ ===
Shape: Type = { circle(Float) | rect(Float, Float) }

// === 値構築（関数呼び出しと完全に同じ）===
c: Color = green // green() と等価
p: Point = Point(1.0, 2.0)
r: Result(Int, String) = ok(42)
s: Shape = circle(5.0)

// === インターフェース定義（フィールドがすべて関数のレコード型）===
Drawable: Type = {
 draw: (Surface) -> Void,
 bounding_box: () -> Rect
}

Serializable: Type = {
 serialize: () -> String
}

// === インターフェース実装（型の末尾にインターフェース名を列挙）===
Point: Type = {
 x: Float,
 y: Float,
 Drawable, // Drawable インターフェースを実装
 Serializable // Serializable インターフェースを実装
}
```

### 3.3 型操作

```yaoxiang
// 値としての型
MyInt = Int
MyList = List(Int)

// 型リフレクション（コンストラクタパターンマッチング）
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

YaoXiang は強力な型推論能力を備えています：

```yaoxiang
// 基本推論
x = 42 // Int として推論
y = 3.14 // Float として推論
z = "hello" // String として推論

// 関数戻り値の推論
add: (a: Int, b: Int) -> Int = a + b

// ジェネリック推論
first: (T: Type) -> ((list: List(T)) -> Option(T)) = (list) => {
 if list.length > 0 { some(list[0]) } else { none }
}
```

---

## 四、メモリ管理

### 4.1 所有権モデル

YaoXiang は**所有権モデル**を採用してメモリを管理し、各値は唯一の所有者を持ちます：

```yaoxiang
// === デフォルト Move（ゼロコピー） ===
p: Point = Point(1.0, 2.0)
p2 = p // Move、所有権が移転、p は無効になる

// === ref キーワード = Arc（安全な共有） ===
shared = ref p // Arc、スレッドセーフ

spawn(() => print(shared.x)) // ✅ 安全

// === clone() 明示的コピー ===
p3 = p.clone() // p と p3 は独立
```

### 4.2 Move セマンティクス（デフォルト）

```yaoxiang
// 代入 = Move（ゼロコピー）
p: Point = Point(1.0, 2.0)
p2 = p // Move、p は無効になる

// 関数引数渡し = Move
process: (p: Point) -> Void = {
 // p の所有権が内部に移転する
}

// 戻り値 = Move
create: () -> Point = {
 p = Point(1.0, 2.0)
 return p // Move、所有権が移転
}
```

### 4.3 ref キーワード（Arc）

```yaoxiang
// ref キーワードは Arc（参照カウント）を作成
p: Point = Point(1.0, 2.0)
shared = ref p // Arc、スレッドセーフ

spawn(() => print(shared.x)) // ✅ 安全

// Arc は自動的にライフサイクルを管理
// shared がスコープを出ると、カウントがゼロになり自動的に解放される
```

### 4.4 clone() 明示的コピー

```yaoxiang
// 元の値を保持する必要がある場合は clone() を使用
p: Point = Point(1.0, 2.0)
p2 = p.clone() // p と p2 は独立

p.x = 0.0 // ✅
p2.x = 0.0 // ✅ 互いに影響しない
```

### 4.5 unsafe コードブロック（システムレベル）

```yaoxiang
// 生ポインタは unsafe ブロック内でのみ使用可能
p: Point = Point(1.0, 2.0)

unsafe {
 ptr: *Point = &p // 生ポインタ
 (*ptr).x = 0.0 // ユーザーが安全性を保証する
}
```

### 4.6 RAII

```yaoxiang
// RAII による自動解放
with_file: (path: String) -> String = {
 file = File.open(path) // 自動的に開く
 content = file.read_all()
 // 関数終了時、file は自動的に閉じる
 content
}
```

### 4.7 Send / Sync 制約

| 制約 | セマンティクス | 説明 |
|------|------|------|
| **Send** | スレッド間を安全に通過できる | 値を別のスレッドに移動できる |
| **Sync** | スレッド間を安全に共有できる | 不変参照を別のスレッドに共有できる |

```yaoxiang
// ref T は自動的に Send + Sync を満たす（Arc はスレッドセーフ）
p: Point = Point(1.0, 2.0)
shared = ref p

spawn(() => print(shared.x)) // ✅ Arc はスレッドセーフ

// 生ポインタ *T は Send/Sync を満たさない
unsafe {
 ptr: *Point = &p // 単一スレッドでのみ使用可能
}
```

### 4.9 実装しないもの

| 特性 | 理由 |
|------|------|
| ライフタイム `'a` | 参照の概念がないため、ライフタイム不要 |
| 借用チェッカー | ref = Arc で代替 |
| `&T` 借用構文 | Move セマンティクスを採用 |

---

## 五、非同期プログラミングと並行処理

> 「万物并作、吾以観復。」——《易経・復卦》
>
> YaoXiang は**並作（spawn）モデル**を採用します。これは**遅延評価**に基づいた無感覚な非同期並行パラダイムです。その中核設計理念は：**開発者が同期・順次的な思考でロジックを記述でき、言語ランタイムがその中の計算ユニットを万物並作のように自動的かつ効率的に並行実行し、最終的に統一的に協調させる**ことです。

> 詳細は [《並作モデル白書》](YaoXiang-async-whitepaper.md) と [非同期実装方案](YaoXiang-async-implementation.md) を参照してください。

### 5.1 並作モデルの基本概念

#### 5.1.1 並作グラフ：万物並作の舞台

すべてのプログラムはコンパイル時に**有向非巡回計算グラフ（DAG）**に変換され、**並作グラフ**と呼ばれます。ノードは式の計算を表し、エッジはデータ依存を表します。このグラフは遅延的であり、すなわちノードはその出力が**実際に必要とされる**ときにのみ評価されます。

```yaoxiang
// コンパイラが自動的に並作グラフを構築
fetch_user: spawn () -> User = (id) => { ... }
fetch_posts: spawn (User) -> Posts = (user) => { ... }

main:() -> Void = () => {
 user = fetch_user(1) // ノード A (Async(User))
 posts = fetch_posts(user) // ノード B (Async(Posts))、A に依存

 // ノード C は A と B の結果を必要とする
 print(posts.title) // 自動待機：A と B の完了を先に保証
}
```

#### 5.1.2 並作値：Async(T)

`spawn fn` とマークされた関数呼び出しは即座に `Async(T)` 型の値を返し、これを**並作値**と呼びます。これは軽量なプロキシであり、実際の結果ではなく、**並作中の将来の値**を表します。

**中核特性**：
- **型透過性**：`Async(T)` は型システムにおいて `T` のサブタイプであり、`T` を期待する任意のコンテキストで使用可能
- **自動待機**：プログラムが `T` 型の具体的な値を使用する必要がある操作に到達したとき、ランタイムは自動的に現在のタスクを中断し、計算の完了を待つ
- **ゼロ伝染**：非同期コードと同期コードは構文と型シグネチャに違いがない

```yaoxiang
// 並作値の使用例
fetch_data: spawn (String) -> JSON = (url) => { ... }

main: () -> Void = () => {
 data = fetch_data("url") // Async(JSON)

 // Async(JSON) は JSON として直接使用可能
 // フィールドアクセス時に自動待機が発生
 print(data.name) // data.await().name と等価
}
```

### 5.2 並作構文体系

`spawn` キーワードは三重の意味を持ち、同期思考と非同期実装を結ぶ唯一の架け橋です：

| 公用語 | 構文形式 | セマンティクス | ランタイム動作 |
|----------|----------|------|------------|
| **並作関数** | `spawn fn` | 並作実行に参加可能な計算ユニットを定義 | その呼び出しは `Async(T)` を返す |
| **並作ブロック** | `spawn { a(), b() }` | 明示的に宣言された並行範囲 | ブロック内のタスクは強制的に並列実行 |
| **並作ループ** | `spawn for x in xs { ... }` | データ並列パラダイム | ループ本体はすべての要素で並作実行 |

#### 5.2.1 並作関数

```yaoxiang
// spawn を使用して並作関数をマーク
// 構文は通常の関数と完全に同じ、余分な負担なし

fetch_api: spawn (String) -> JSON = (url) => {
 response = HTTP.get(url)
 JSON.parse(response.body)
}

// ネストした並作呼び出し
process_user: (Int) -> Report = (user_id) => {
 user = fetch_user(user_id) // Async(User)
 profile = fetch_profile(user) // Async(Profile)、user に依存
 generate_report(user, profile) // profile に依存
}
```

#### 5.2.2 並作ブロック

```yaoxiang
// spawn { } - 明示的な並列構造
// ブロック内のすべての式が独立したタスクとして並行実行される

compute_all: (Int, Int) -> (Int, Int, Int) spawn = (a, b) => {
 // 3つの独立した計算が並列実行
 (x, y, z) = spawn {
 heavy_calc(a), // タスク 1
 heavy_calc(b), // タスク 2
 another_calc(a, b) // タスク 3
 }
 (x, y, z)
}
```

#### 5.2.3 並作ループ

```yaoxiang
// spawn for - データ並列ループ
// 各反復が独立したタスクとして並列実行される

parallel_sum: (Int) -> Int spawn = (n) => {
 total = spawn for i in 0..n {
 fibonacci(i) // 各反復が並列
 }
 total
}
```

#### 5.2.4 データ並列ループ

```yaoxiang
// spawn for - データ並列ループ
// 各反復が独立したタスクとして並列実行される

parallel_sum: (Int) -> Int spawn = (n) => {
 total = spawn for i in 0..n {
 fibonacci(i) // 各反復が並列
 }
 total
}

// 行列乗算の並列化
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
// 明示的な await 不要、コンパイラが自動的に待機点を挿入

main: () -> Void = () => {
 // 自動並列：2つの独立したリクエストが並列実行
 users = fetch_users() // Async(List(User))
 posts = fetch_posts() // Async(List(Post))

 // 待機点は"+"操作箇所に自動挿入
 count = users.length + posts.length

 // フィールドアクセスが待機を引き起こす
 first_user = users[0] // users の準備を待つ
 print(first_user.name)
}

// 条件分岐内の待機
process_data: spawn () -> Void = () => {
 data = fetch_data() // Async(Data)

 if data.is_valid { // data の準備を待つ
 process(data)
 } else {
 log("Invalid data")
 }
}
```

### 5.4 並行制御ツール

```yaoxiang
// すべてのタスクの完了を待つ
await_all: (T: Type) -> ((tasks: List(Async(T))) -> List(T)) = {
 // Barrier 待機
}

// 任意の一つの完了を待つ
await_any: (T: Type) -> ((tasks: List(Async(T))) -> T) = {
 // 最初に完了した結果を返す
}

// タイムアウト制御
with_timeout: (T: Type) -> ((task: Async(T), timeout: Duration) -> Option(T)) = {
 // タイムアウト時は None を返す
}
```

### 5.5 タスク間共有：ref キーワード

YaoXiang は手動の同期プリミティブ（`Send`/`Sync` trait、`Mutex`、`RwLock` など）を必要としません。所有権 + `ref` が並行安全を自動的に処理します。

#### 5.5.1 ref：スコープ間共有

```yaoxiang
// ref はスコープ間共有の唯一の方法
// コンパイラが自動的に Rc（単一タスク）または Arc（タスク間）を選択

data = load_data()
shared = ref data // コンパイラが自動的に実装を選択

result = spawn {
 process_a(shared), // 共有参照、タスク間 → Arc
 process_b(shared) // 共有参照
}
```

#### 5.5.2 コンパイラの自動最適化

```
ref のデータフロー解析：

他のタスクにエスケープしない → Rc（非アトミック参照カウント、低オーバーヘッド）
他のタスクにエスケープする     → Arc（アトミック参照カウント、スレッドセーフ）
```

ユーザーは基盤が Rc か Arc かを気にする必要はありません——`ref` が共有保有であり、それで十分です。

#### 5.5.3 リソースタイプの自動シリアライズ

```yaoxiang
// コンパイラはリソースタイプの使用を追跡し、並行安全を保証
// 同じリソースの操作は自動的にシリアライズされる

(a, b) = spawn {
 read_file("data.txt"), // 先に実行
 write_file("data.txt", x) // 読み取りの完了を待つ
}
```

---

## 六、モジュールシステム

### 6.1 モジュール定義

```yaoxiang
// モジュールはファイルを境界として使用
// Math.yx ファイル
pub pi: Float = 3.14159
pub sqrt: (x: Float) -> Float = { ... }
```

### 6.2 モジュールのインポート

```yaoxiang
// モジュール全体をインポート
use std.io

// インポートして名前変更
use std.io as IO

// 特定の関数をインポート
use std.io.{ read_file, write_file }
```

---

## 七、メソッドバインディングとカリー化

YaoXiang は**純粋関数型設計**を採用しており、高度なバインディング機構によりシームレスなメソッド呼び出しとカリー化を実現します。`struct`、`class` などのキーワードを導入する必要はありません。

### 7.1 中核関数定義

すべての操作は通常の関数で実装され、最初のパラメータが操作の主体として規約化されます：

```yaoxiang
// === Point.yx (モジュール) ===

// 統一構文：コンストラクタがそのまま型
Point: Type = { x: Float, y: Float }

// 中核関数：最初のパラメータが操作の主体
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

// より複雑な関数
distance_with_scale: (s: Float, p1: Point, p2: Point) -> Float = {
 dx = (p1.x - p2.x) * s
 dy = (p1.y - p2.y) * s
 (dx * dx + dy * dy).sqrt()
}
```

### 7.2 基本メソッドバインディング

#### 7.2.1 自動バインディング（MoonBit スタイル）

YaoXiang は名前空間に基づく自動バインディングをサポートし、**追加の宣言は一切不要**です：

```yaoxiang
// === Point.yx ===

Point: Type = { x: Float, y: Float }

// 中核関数
distance: (a: Point, b: Point) -> Float = {
 dx = a.x - b.x
 dy = a.y - b.y
 (dx * dx + dy * dy).sqrt()
}

// === main.yx ===

use Point

main: () -> Void = {
 p1 = Point(3.0, 4.0)
 p2 = Point(1.0, 2.0)

 // ✅ 自動バインディング：メソッドを直接呼び出す
 result = p1.distance(p2) // distance(p1, p2) として解析
}
```

**自動バインディングルール**：
- モジュール内で定義された関数
- 最初のパラメータ型がモジュール名と一致する場合
- 自動的にメソッド呼び出し構文をサポート

#### 7.2.2 バインディングなしオプション（デフォルト動作）

```yaoxiang
// === Vector.yx ===

Vector: Type = Vector(x: Float, y: Float, z: Float)

// 内部ヘルパー関数、自動バインディングを希望しない
dot_product_internal: (a: Vector, b: Vector) -> Float = {
 a.x * b.x + a.y * b.y + a.z * b.z
}

// === main.yx ===

use Vector

main: () -> Void = {
 v1 = Vector(1.0, 0.0, 0.0)
 v2 = Vector(0.0, 1.0, 0.0)

 // ❌ バインディング不可：非 pub 関数は自動バインディングされない
 // v1.dot_product_internal(v2) // コンパイルエラー！

 // ✅ 直接呼び出しが必須（モジュールの外部では不可視）
}
```

### 7.3 位置ベースのバインディング構文

YaoXiang は**最もエレガントなバインディング構文**を提供し、位置マーカー `[n]` を使用してバインディング位置を正確に制御します：

#### 7.3.1 基本構文

```yaoxiang
// === Point.yx ===

Point: Type = { x: Float, y: Float }

// 中核関数
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

// バインディング構文：Type.method = func[position]
// 意味：メソッド呼び出し時、呼び出し元を func の [position] パラメータにバインド

Point.distance = distance[0] // 最初のパラメータにバインド
Point.add = add[0] // 最初のパラメータにバインド
Point.scale = scale[0] // 最初のパラメータにバインド
```

**セマンティック解析**：
- `Point.distance = distance[0]`
  - `distance` 関数には2つのパラメータがある：`distance(Point, Point)`
  - `[0]` は呼び出し元が最初のパラメータにバインドされることを示す
  - 使用法：`p1.distance(p2)` → `distance(p1, p2)`

#### 7.3.2 複数位置の結合バインディング

```yaoxiang
// === Math.yx ===

// 関数：scale, point1, point2, extra1, extra2
calculate: (scale: Float, a: Point, b: Point, x: Float, y: Float) -> Float = { ... }

// === Point.yx ===

Point: Type = { x: Float, y: Float }

// 複数位置のバインディング
Point.calc1 = calculate[1, 2] // scale と point1 をバインド
Point.calc2 = calculate[1, 3] // scale と point2 をバインド
Point.calc3 = calculate[2, 3] // point1 と point2 をバインド

// === main.yx ===

use Point

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

// 1. バインディング[1,2] - 残り3,4,5
f1 = p1.calc1(2.0) // scale=2.0, point1=p1 をバインド
// f1 は現在 p2, x, y を必要とする
result1 = f1(p2, 10.0, 20.0) // calculate(2.0, p1, p2, 10.0, 20.0)

// 2. バインディング[1,3] - 残り2,4,5
f2 = p2.calc2(2.0) // scale=2.0, point2=p2 をバインド
// f2 は現在 point1, x, y を必要とする
result2 = f2(p1, 10.0, 20.0) // calculate(2.0, p1, p2, 10.0, 20.0)

// 3. バインディング[2,3] - 残り1,4,5
f3 = p1.calc3(p2) // point1=p1, point2=p2 をバインド
// f3 は現在 scale, x, y を必要とする
result3 = f3(2.0, 10.0, 20.0) // calculate(2.0, p1, p2, 10.0, 20.0)
```

#### 7.3.3 残りのパラメータの埋め込み順序

**中核ルール**：バインディング後、残りパラメータは**元の関数の順序**で埋め込まれ、バインド済みの位置をスキップします。

```yaoxiang
// 関数を仮定：func(p1, p2, p3, p4, p5)

// 1番目と3番目のパラメータをバインド
Type.method = func[1, 3]

// 呼び出し時：
method(p2_value, p4_value, p5_value)

// 写像：
func(p1_bound, p2_value, p3_bound, p4_value, p5_value)
// 残りのパラメータ：2,4,5 は元の順序で埋め込まれる
```

#### 7.3.4 型検査の優位性

```yaoxiang
// ✅ 合法なバインディング
Point.distance = distance[0] // distance(Point, Point)
Point.calc = calculate[1, 2] // calculate(scale, Point, Point, ...)

// ❌ 違法なバインディング（コンパイラがエラー）
Point.wrong = distance[5] // 5番目のパラメータは存在しない
Point.wrong = distance[0] // パラメータは1から開始
Point.wrong = distance[1, 2, 3, 4] // 関数のパラメータ数を超える
```

### 7.4 カリー化バインディングのきめ細かい制御

```yaoxiang
// === Math.yx ===

distance_with_scale: (scale: Float, a: Point, b: Point) -> Float = { ... }

// === Point.yx ===

Point: Type = { x: Float, y: Float }

// バインディング戦略：各位置を柔軟に制御
Point.distance = distance[0] // 基本バインディング
Point.distance_scaled = distance_with_scale[2] // 2番目のパラメータにバインド

// === main.yx ===

use Point
use Math

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

// 1. 基本的な自動バインディング
d1 = p1.distance(p2) // distance(p1, p2)

// 2. 異なる位置にバインド
f = p1.distance_scaled(2.0) // 2番目のパラメータをバインド、残り1,3
result = f(p2) // distance_with_scale(2.0, p1, p2)

// 3. チェーン呼び出し
d2 = p1.distance(p2).distance_scaled(2.0) // チェーン呼び出し
```

### 7.5 完全なバインディングシステム

```yaoxiang
// === Point.yx ===

Point: Type = { x: Float, y: Float }

// 中核関数
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

// 自動バインディング（中核）
Point.distance = distance[0]
Point.add = add[0]
Point.scale = scale[0]

// === Math.yx ===

// グローバル関数
multiply_by_scale: (scale: Float, a: Point, b: Point) -> Float = { ... }

// === main.yx ===

use Point
use Math

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

// 使用
d = p1.distance(p2) // distance(p1, p2)
r = p1.add(p2) // add(p1, p2)
s = p1.scale(2.0) // scale(p1, 2.0)

// グローバル関数のバインディング
Point.multiply = multiply_by_scale[2] // 2番目のパラメータにバインド
m = p1.multiply(2.0, p2) // multiply_by_scale(2.0, p1, p2)
```

### 7.6 バインディングのスコープとルール

#### 7.6.1 pub の役割

```yaoxiang
// === Point.yx ===

Point: Type = { x: Float, y: Float }

// 非 pub 関数
internal_distance: (a: Point, b: Point) -> Float = {
 dx = a.x - b.x
 dy = a.y - b.y
 (dx * dx + dy * dy).sqrt()
}

// pub 関数
pub distance: (a: Point, b: Point) -> Float = {
 dx = a.x - b.x
 dy = a.y - b.y
 (dx * dx + dy * dy).sqrt()
}

// === main.yx ===

use Point

// 自動バインディングは pub 関数に対してのみ有効
p1.distance(p2) // ✅ distance は pub、自動バインディング可能
// p1.internal_distance(p2) // ❌ pub ではないため、バインディング不可
```

#### 7.6.2 pub 自動バインディング機構

`pub` で宣言された関数は、コンパイラが同じファイルで定義された型に自動バインドします：

```yaoxiang
// === Point.yx ===

Point: Type = { x: Float, y: Float }

// pub で宣言すると、コンパイラが自動バインディング
pub distance: (p1: Point, p2: Point) -> Float = {
 dx = p1.x - p2.x
 dy = p1.y - p2.y
 (dx * dx + dy * dy).sqrt()
}

pub translate: (self: Point, dx: Float, dy: Float) -> Point = {
 Point(self.x + dx, self.y + dy)
}

// コンパイラが自動推論し、バインディングを実行：
// Point.distance = distance[0]
// Point.translate = translate[0]

// === main.yx ===

use Point

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

// ✅ 関数型呼び出し
d = distance(p1, p2)

// ✅ OOP 構文糖（自動バインディング）
d2 = p1.distance(p2)
p3 = p1.translate(1.0, 1.0)
```

**自動バインディングルール**：
1. 関数がモジュールファイルで定義されている（型と同じファイル）
2. 関数のパラメータにその型が含まれている
3. `pub` でエクスポートされている
4. コンパイラが自動的に `Type.method = function[0]` を実行

**利点**：
- 手動のバインディング宣言が不要
- コードがより簡潔
- バインディングの忘れやエラーを回避

#### 7.6.3 モジュール内バインディング

```yaoxiang
// === Point.yx ===

Point: Type = { x: Float, y: Float }

distance: (a: Point, b: Point) -> Float = {
 dx = a.x - b.x
 dy = a.y - b.y
 (dx * dx + dy * dy).sqrt()
}

// モジュール内部では、すべての関数が可視
// しかし自動バインディングは、pub でエクスポートされた関数のみが外部で有効

pub distance // エクスポート、外部で自動バインディング可能
```

### 7.7 設計上の優位性のまとめ

| 特性 | 説明 |
|------|------|
| **ゼロ構文負担** | 自動バインディングに追加の宣言不要 |
| **位置の正確な制御** | `[n]` でバインディング位置を正確に指定 |
| **複数位置結合** | `[1, 2, 3]` のような複数パラメータバインディングをサポート |
| **型安全性** | コンパイラがバインディング位置の有効性を検証 |
| **キーワード不要** | `bind` などのキーワード不要 |
| **柔軟なカリー化** | 任意の位置パラメータのバインディングをサポート |
| **pub 制御** | pub 関数のみが外部バインディング可能 |

### 7.8 従来のメソッドバインディングとの違い

| 従来言語 | YaoXiang |
|---------|----------|
| `obj.method(arg)` | `obj.method(arg)` |
| クラス/メソッド定義が必要 | 関数 + バインディング宣言のみ必要 |
| 構文 `class { method() {} }` | 構文 `Type.method = func[n]` |
| 継承、多態性 | 純粋関数型、継承なし |
| メソッドテーブルルックアップ | コンパイル時バインディング、ランタイムオーバーヘッドなし |

**中核優位性**：YaoXiang のバインディングは**コンパイル時機構**であり、ランタイムコストがゼロであると同時に、関数型プログラミングの純粋性と柔軟性を保持しています。

---

## 八、AI フレンドリー設計

YaoXiang の構文設計は AI コード生成と変更のニーズを特に考慮しています：

### 8.1 設計原則

```yaoxiang
// AI フレンドリー設計目標：
// 1. 厳密に構造化、曖昧さのない構文
// 2. AST が明確、位置特定が容易
// 3. セマンティクスが明確、隠された動作がない
// 4. コードブロックの境界が明確
// 5. 型情報が完全
```

### 8.2 厳密に構造化された構文

#### 8.2.1 宣言構文の AI フレンドリー戦略

```yaoxiang
// === AI コード生成のベストプラクティス ===

// ✅ 推奨：完全な新構文宣言 + 型集中規約を使用
// AI は意図を正確に理解し、完全な型情報を生成できる

add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1
empty: () -> Void = {}

// ❌ 避ける：型注釈の省略や型の分散
// AI はパラメータ型を判定できず、誤ったコードを生成する可能性がある
add: (a: Int, b: Int) -> Int = a + b // パラメータに型なし
identity: (T: Type) -> ((x: T) -> T) = x // パラメータに型なし
add2: (a: Int, b: Int) -> Int = a + b // 型が実装に分散

// ⚠️ 互換：旧構文は保守のみ
// AI は新構文 + 型集中規約を優先的に生成すべき
mul(Int, Int) -> Int = (a, b) => a * b // 新規コードでは非推奨
```

**型集中規約の AI 優位性：**

1. **シグネチャが一目でわかる**：AI は宣言行を読むだけで完全な関数シグネチャを理解できる
2. **より安全な変更**：型を変更するには宣言を変更するだけで実装本体に影響しない
3. **生成がより簡単**：AI はまず宣言を生成し、次に実装を埋めることができる
4. **カリー化フレンドリー**：明確なカリー化型シグネチャは AI 処理に適している

```yaoxiang
// AI 処理の例
// 入力：実装本体 (a, b) => a + b
// AI は宣言を見る：add: (Int, Int) -> Int
// 結論：パラメータ型は Int, Int、戻り型は Int

// 比較：型が分散している場合
// 入力：実装本体 (a: Int, b: Int) => a + b
// AI が必要とすること：実装本体を分析して型情報を抽出
// 結果：より複雑な処理ロジック、エラーが発生しやすい
```

#### 8.2.2 二重構文戦略と AI

| 構文タイプ | AI 生成戦略 | 使用シナリオ |
|---------|-----------|---------|
| **新構文** | ✅ 優先生成、完全な型情報 | すべての新規コード生成 |
| **旧構文** | ⚠️ 旧コード保守時のみ使用 | 歴史的コードの変更 |
| **無注釈** | ❌ 生成を避ける | いかなる状況でも生成すべきでない |

#### 8.2.3 構文境界の明確化

```yaoxiang
// AI フレンドリーなコードブロック境界

// ✅ 明確な開始と終了マーカー
function_name: (Type1, Type2) -> ReturnType = (param1, param2) => {
 // 関数本体
 if condition {
 do_something()
 } else {
 do_other()
 }
}

// ✅ 条件文には中括弧が必須
if condition {
 // 条件本体
}

// ✅ 型定義が明確
MyType: Type = { Type1 | Type2 }

// ❌ 曖昧な書き方を避ける
if condition // 中括弧が欠落
 do_something()
```

#### 8.2.4 曖昧さのない構文制約

```yaoxiang
// AI 生成時に遵守すべき制約

// 1. 括弧の省略禁止
// ✅ 正しい
foo: (T: Type) -> ((x: T) -> T) = x
my_list = [1, 2, 3]

// ❌ 誤り（禁止）
foo T { T } // パラメータには括弧が必須
my_list = [1 2 3] // リストにはカンマが必須

// 2. 明示的な戻り型または推論可能な形式が必須
// ✅ 正しい
get_num: () -> Int = 42
get_num2: () = 42 // 戻り型は推論可能
get_void = () => { 42 } // ✓ Void として推論（ブロックに return なし、デフォルトで Void を返す）

// 3. パラメータには型注釈が必須（新構文）
// ✅ 正しい
add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1

// ❌ 誤り
add: (a: Int, b: Int) -> Int = a + b // パラメータに型なし
identity: (T: Type) -> ((x: T) -> T) = x // パラメータに型なし
```

#### 8.2.5 AI 生成の推奨パターン

```yaoxiang
// AI が関数を生成する際の標準テンプレート

// パターン1：完全な型注釈
function_name: (param1: ParamType1, param2: ParamType2, ...) -> ReturnType = {
 // 関数本体
 return expression
}

// パターン2：戻り型推論
function_name: (param1: ParamType1, param2: ParamType2) = {
 // 関数本体
 return expression
}

// パターン3：単一パラメータ短縮形
function_name: (param: ParamType) -> ReturnType = expression

// パターン4：パラメータなし関数
function_name: () -> ReturnType = expression

// パターン5：空関数
function_name: () -> Void = {}
```

### 8.3 エラーメッセージの AI フレンドリー性

```yaoxiang
// エラーメッセージは明確な修正提案を提供するべき

// フレンドリーでないエラー
// Syntax error at token 'a'

// AI フレンドリーなエラー
// Missing type annotation for parameter 'a'
// Suggestion: add ': Int' or similar type to '(a, b) => a + b'
// Correct version: add: (a: Int, b: Int) -> Int = a + b
```

---

## 九、型集中規約（中核設計思想）

### 9.1 規約概要

YaoXiang の中核設計規約は**「宣言優先、型集中」**です。この規約は言語の AI フレンドリー性と開発効率の礎石です。

```yaoxiang
// ✅ 中核規約：型情報は宣言行で統一
add: (a: Int, b: Int) -> Int = a + b

// ❌ 避ける：型情報が実装に分散
add: (a: Int, b: Int) -> Int = a + b
```

### 9.2 規約の5つの中核優位性

#### 1. 構文の一貫性
```yaoxiang
// すべての宣言が同じフォーマットに従う
x: Int = 42 // 変数
name: String = "YaoXiang" // 変数
add: (a: Int, b: Int) -> Int = a + b // 関数
inc: (x: Int) -> Int = x + 1 // 関数
Point: Type = { x: Float, y: Float } // 型
```

#### 2. 宣言と実装の分離
```yaoxiang
// 宣言行が完全な型情報を提供
add: (a: Int, b: Int) -> Int = a + b
// └────────────────────┘
// 完全な関数シグネチャ

// 実装本体はビジネスロジックに集中
// (a, b) => a + b は型を気にする必要がなく、機能の実装のみを行う
```

#### 3. AI フレンドリー性
```yaoxiang
// AI 処理フロー：
// 1. 宣言行を読む → 関数シグネチャを完全に理解
// 2. 実装を生成 → 型推論の分析不要
// 3. 型を変更 → 宣言行のみを変更、実装に影響しない

// 比較：型が分散している方法
add: (a: Int, b: Int) -> Int = a + b
// AI が必要とすること：実装本体を分析して型情報を抽出 → より複雑、エラーが発生しやすい
```

#### 4. より安全な変更
```yaoxiang
// パラメータ型を変更
// 元：add: (a: Int, b: Int) -> Int = a + b
// 変更：add: (Float, Float) -> Float = (a, b) => a + b
// 実装本体：(a, b) => a + b 変更不要！

// 型が分散している場合：
// 元：add: (a: Int, b: Int) -> Int = a + b
// 変更：add: (a: Float, b: Float) -> Float = a + b // 2箇所を変更する必要がある
```

#### 5. カリー化フレンドリー
```yaoxiang
// カリー化型が一目でわかる
add_curried: (a: Int) -> (b: Int) -> Int = a + b
// └─────────────┘
// カリー化シグネチャ

// ファーストクラス市民としての関数合成
compose: (Int -> Int, Int -> Int) -> Int -> Int = (f, g) => x => f(g(x))
```

### 9.3 規約の実施ルール

#### ルール1：パラメータは宣言内で型を指定しなければならない
```yaoxiang
// ✅ 正しい
add: (a: Int, b: Int) -> Int = a + b

// ❌ 誤り
add: (a: Int, b: Int) -> Int = a + b // パラメータ型が欠落
identity: (T: Type) -> ((x: T) -> T) = x // パラメータ型が欠落
```

#### ルール2：戻り型は推論可能だが注釈を推奨
```yaoxiang
// ✅ 推奨：完全な注釈
get_num: () -> Int = () => 42

// ✅ 許容可能：戻り型推論
get_num: () = () => 42

// ✅ 空関数は Void として推論
empty: () = () => {}
```

#### ルール3：Lambda 内部の型注釈は一時的
```yaoxiang
// ✅ 正しい：宣言内の型に依存
add: (a: Int, b: Int) -> Int = a + b

// ⚠️ 可能だが非推奨：Lambda 内で重複注釈
add: (Int, Int) -> Int = (a: Int, b: Int) => a + b

// ❌ 誤り：宣言注釈が欠落
add: (a: Int, b: Int) -> Int = a + b
```

#### ルール4：旧構文も同じ理念に従う
```yaoxiang
// 旧構文も宣言位置に型情報を提供すべき
// フォーマットは異なるが、理念は一貫：
// - 宣言行が主要な型情報を含む
// - 実装本体は比較的簡潔
add(Int, Int) -> Int = (a, b) => a + b
```

### 9.4 規約と型推論の関係

```yaoxiang
// 規約は型推論を妨げず、推論の方向を導く

// 1. 完全な注釈（推論なし）
add: (a: Int, b: Int) -> Int = a + b

// 2. 部分推論（宣言がパラメータ型を提供）
add: (Int, Int) = (a, b) => a + b // 戻り型推論

// 3. 空関数の推論
empty: () = () => {} // () -> Void として推論
```

### 9.5 規約の AI 実装優位性

**AI コード生成フロー：**

1. **要件を読む** → 宣言を生成
   ```
   要件：加算関数
   生成：add: (Int, Int) -> Int = (a, b) => ???
   ```

2. **実装を埋める** → 型分析不要
   ```
   実装：add: (a: Int, b: Int) -> Int = a + b
   ```

3. **型を変更** → 宣言のみを変更
   ```
   変更：add: (Float, Float) -> Float = (a, b) => a + b
   実装：(a, b) => a + b  変更なし
   ```

**規約なしの場合の AI 処理との比較：**
```
要件：加算関数
AI が必要とすること：
  1. パラメータ型を推論
  2. 戻り型を推論
  3. 実装本体を生成
  4. 一貫性を検証
  5. 型変更時の複雑な更新を処理

結果：より複雑、エラーが発生しやすい
```

### 9.6 規約の哲学的意義

この規約は YaoXiang の中核理念を体現しています：

- **宣言はドキュメントである**：宣言行が完全な関数ドキュメントである
- **型は契約である**：型情報は呼び出し元と実装者の間の契約である
- **ロジックは実装である**：実装本体は「何をするか」のみに関心を払う
- **ツールは補助である**：型システム、AI ツールはどちらも明確な宣言に基づいて機能する

### 9.7 実際の応用比較

#### 完全な例：計算機モジュール

```yaoxiang
// === 推奨される方法：型集中規約 ===

// モジュール宣言
pub add: (a: Int, b: Int) -> Int = a + b
pub multiply: (a: Int, b: Int) -> Int = a * b

// 高階関数
pub apply_twice: (f: Int -> Int, x: Int) -> Int = f(f(x))

// カリー化関数
pub make_adder: (x: Int) -> (Int) -> Int = y => x + y

// ジェネリック関数
pub map: (A: Type, B: Type) -> ((f: (A) -> B, xs: List(A)) -> List(B)) = case xs of
 [] => []
 (x :: rest) => f(x) :: map(f, rest)

// 型定義
Point: Type = { x: Float, y: Float }
pub distance: (a: Point, b: Point) -> Float = {
 dx = a.x - b.x
 dy = a.y - b.y
 (dx * dx + dy * dy).sqrt()
}

// === 非推奨の方法：型が分散 ===

// パラメータ型が Lambda 内にある
add: (a: Int, b: Int) -> Int = a + b
multiply = (a: Int, b: Int) => a * b

// 高階関数の型が分散
apply_twice = (f: (Int) -> Int, x: Int) => f(f(x))

// カリー化の型が分散
make_adder = (x: Int) => (y: Int) => x + y

// ジェネリックの型が分散
map: (A: Type, B: Type) -> ((f: (A) -> B, xs: List(A)) -> List(B)) = case xs of
 [] => []
 (x :: rest) => f(x) :: map(f, rest)
```

#### コード保守の比較

```yaoxiang
// 要件：add を Int から Float に変更

// === 推奨される方法：宣言行のみを変更 ===
// 元
add: (a: Int, b: Int) -> Int = a + b

// 変更後
add: (a: Float, b: Float) -> Float = a + b
// ↑↑↑↑↑↑↑↑↑ ↑↑↑↑↑↑↑
// 宣言行を変更        実装本体は変更なし

// === 非推奨の方法：複数箇所を変更する必要がある ===
// 元
add: (a: Int, b: Int) -> Int = a + b

// 変更後
add: (a: Float, b: Float) -> Float = a + b
// ↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑
// すべてのパラメータ型を変更する必要がある
```

#### AI 支援プログラミングの比較

```yaoxiang
// AI 要件：2点間のマンハッタン距離を計算する関数を実装

// === AI が推奨される書き方を見る場合 ===
Point: Type = { x: Float, y: Float }
pub manhattan: (a: Point, b: Point) -> Float = ??? // AI は完全なシグネチャを直接知る

// AI 生成：
pub manhattan: (a: Point, b: Point) -> Float = {
 (a.x - b.x).abs() + (a.y - b.y).abs()
}

// === AI が非推奨の書き方を見る場合 ===
Point: Type = { x: Float, y: Float }
pub manhattan = ??? // AI は推論が必要：パラメータ型？戻り型？

// AI は以下を生成する可能性がある：
pub manhattan = (a: Point, b: Point) => Float => {
 (a.x - b.x).abs() + (a.y - b.y).abs()
}
// または型情報が不完全なためエラーになる可能性がある
```

### 9.8 規約実施のチェックリスト

YaoXiang コードを記述する際、以下のチェックリストを使用できます：

- [ ] すべての関数宣言は宣言行に完全な型注釈を持つ
- [ ] パラメータ型は宣言内で指定され、Lambda 内ではない
- [ ] 戻り型は可能な限り宣言内で注釈される
- [ ] 変数宣言は `name: Type = value` 形式を使用
- [ ] Lambda 本体は簡潔に保ち、型情報を繰り返さない
- [ ] 旧構文ではなく新構文を使用
- [ ] 複雑な型は type 定義を使用し、宣言を明確に保つ

---

## 十、クイックスタート

### 10.1 Hello World

```yaoxiang
// hello.yx
use std.io

main: () -> Void = {
 print("Hello, YaoXiang!")
}
```

実行方法：`yaoxiang hello.yx`

出力：
```
Hello, YaoXiang!
```

### 10.2 基本構文

```yaoxiang
// 変数と型
x = 42 // 自動的に Int として推論
name = "YaoXiang" // 自動的に String として推論
pi = 3.14159 // 自動的に Float として推論

// 関数（新構文を使用）
add: (a: Int, b: Int) -> Int = a + b

// 条件
if x > 0 {
 "positive"
} elif x == 0 {
 "zero"
} else {
 "negative"
}

// ループ
for i in 0..10 {
 print(i)
}
```

### 10.3 メソッドバインディングの例

```yaoxiang
// === Point.yx ===

Point: Type = { x: Float, y: Float }

// 中核関数
distance: (a: Point, b: Point) -> Float = {
 dx = a.x - b.x
 dy = a.y - b.y
 (dx * dx + dy * dy).sqrt()
}

// 自動バインディング
Point.distance = distance[0]

// === main.yx ===

use Point

main: () -> Void = {
 p1 = Point(3.0, 4.0)
 p2 = Point(1.0, 2.0)

 // バインディングを使用
 d = p1.distance(p2) // distance(p1, p2)
 print(d)
}
```

### 10.4 カリー化バインディングの例

```yaoxiang
// === Math.yx ===

distance_with_scale: (scale: Float, a: Point, b: Point) -> Float = {
 dx = (p1.x - p2.x) * scale
 dy = (p1.y - p2.y) * scale
 (dx * dx + dy * dy).sqrt()
}

// === Point.yx ===

Point: Type = { x: Float, y: Float }

Point.distance_scaled = distance_with_scale[2] // 2番目のパラメータにバインド

// === main.yx ===

use Point

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

// バインディングを使用
f = p1.distance_scaled(2.0) // scale と p1 をバインド
result = f(p2) // 最終呼び出し

// または直接使用
result2 = p1.distance_scaled(2.0, p2)
```

### 10.5 次のステップ

- [言語仕様](./YaoXiang-language-specification.md) を読んで完全な構文を理解する
- [サンプルコード](./examples/) を見て一般的なパターンを学ぶ
- [実装計画](./YaoXiang-implementation.md) を参照して技術的詳細を理解する

---

## 付録

### A. キーワードと注釈

| キーワード | 役割 |
|--------|------|
| `type` | 型定義 |
| `pub` | パブリックエクスポート |
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

### B. 設計インスピレーション

- **Rust**：所有権モデル、ゼロコスト抽象
- **Python**：構文スタイル、可読性
- **Idris/Agda**：依存型、型駆動開発
- **TypeScript**：型注釈、ランタイム型

---

## バージョン履歴

| バージョン | 日付 | 著者 | 変更説明 |
|------|------|------|---------|
| v1.0.0 | 2024-12-31 | 晨煦 | 初期バージョン |
| v1.1.0 | 2025-01-04 | 沫郁酱 | ジェネリック構文を `[T]` に修正（`<T>` から）；`fn` キーワードを削除；関数定義の例を更新 |
| v1.2.0 | 2025-01-06 | 晨煦 | 新構文フォーマットに統一：name: type -> type = lambda |
| v1.3.0 | 2025-01-20 | 晨煦 | 統一型構文（RFC-010）を追加：インターフェース定義には中括弧 `{ serialize: () -> String }` を使用；型の末尾にインターフェース名を列挙してインターフェースを実装；`pub` 自動バインディング機構 |

---

> 「道は一を生じ、一は二を生じ、二は三を生じ、三は万物を生ず。」
> —— 『道徳経』
>
> 型は道のごとく、万物はすべてこれより生ず。
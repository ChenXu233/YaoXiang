# 標準ライブラリ仕様

本文書は YaoXiang プログラミング言語の標準ライブラリ仕様を定義する。コアライブラリ、IO ライブラリ、数学ライブラリを含む。

---

## 第一章：コアライブラリ

### 1.1 基本型

標準ライブラリは以下の基本型の実装を提供する：

| 型             | モジュール       | 説明           |
| -------------- | ---------------- | -------------- |
| `Option(T)`    | `std.option`     | オプション型   |
| `Result(T, E)` | `std.result`     | エラー処理型   |
| `List(T)`      | `std.collection` | 動的配列       |
| `Map(K, V)`    | `std.collection` | ハッシュマップ |
| `String`       | `std.string`     | 文字列型       |
| `Array(T, N)`  | `std.array`      | 固定サイズ配列 |

### 1.2 Option 型

```
Option: (T: Type) -> Type = { some: (T) -> Option(T), none: () -> Option(T) }
```

**バリアント構築**：

| バリアント    | 構文                 | 説明   |
| ------------- | -------------------- | ------ |
| `Option.some` | `Option.some(value)` | 値あり |
| `Option.none` | `Option.none()`      | 値なし |

**常用メソッド**：

```yaoxiang
// 値の有無を確認
is_some: (self: Option(T)) -> Bool
is_none: (self: Option(T)) -> Bool

// 値を取得（panic する可能性あり）
unwrap: (self: Option(T)) -> T

// 値またはデフォルト値を取得
unwrap_or: (self: Option(T), default: T) -> T

// 値をマップ
map: (R: Type) -> ((self: Option(T), f: (T) -> R) -> Option(R))
```

### 1.3 Result 型

```
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }
```

**バリアント構築**：

| バリアント   | 構文                | 説明     |
| ------------ | ------------------- | -------- |
| `Result.ok`  | `Result.ok(value)`  | 成功値   |
| `Result.err` | `Result.err(error)` | エラー値 |

**常用メソッド**：

```yaoxiang
// 成功したか確認
is_ok: (self: Result(T, E)) -> Bool
is_err: (self: Result(T, E)) -> Bool

// 値を取得（panic する可能性あり）
unwrap: (self: Result(T, E)) -> T

// 値またはデフォルト値を取得
unwrap_or: (self: Result(T, E), default: T) -> T

// 成功値をマップ
map: (R: Type) -> ((self: Result(T, E), f: (T) -> R) -> Result(R, E))

// エラー値をマップ
map_err: (F: Type) -> ((self: Result(T, E), f: (E) -> F) -> Result(T, F))
```

**Error キャリアとエラーコード（#323 M4）**：

std 各モジュールの Err キャリア `Error`
は正規化されたエラーコードを運ぶ。コードは RFC-013 の E6xxx/E7xxx セグメントを流用する（例：E6009 =
Range のステップが不正）。これはバージョン横断的に安定した契約であり、プログラムはコードで分岐でき、`yx explain E6009`
でドキュメントを参照できる。コード一覧は RFC-013「ランタイムエラー値とコード貫通」セクションを参照。

```yaoxiang
// Error 値の形式：{ code: String, message: String }

// Err キャリアを取り出す（Ok の場合はランタイムエラー）
unwrap_err: (T, E) -> ((self: Result(T, E)) -> E)

// エラーコード / メッセージを取得
code: (self: Error) -> String
message: (self: Error) -> String
```

**コードによる分岐の例**：

```yaoxiang
use std.range
use std.result

r = range.iter(1..10..0)      // step=0 → Err(Error)
if result.is_err(r) {
    e = result.unwrap_err(r)
    if result.code(e) == "E6009" {
        // Range のステップ不正の分岐で処理
        io.println(result.message(e))
    }
}
```

ユーザ定義のエラーモデリングは `Result(T, E)`
の E ジェネリック引数（カスタムバリアント集合）で行う。std の `Error`
は便利なフォールバックキャリアであり、そのコード体系はユーザの E 型を制約しない。

### 1.4 エラー伝播

```
ErrorPropagate ::= Expr '?'
```

`?` 演算子は Result 型のエラーを自動伝播する：

```
// 成功時は値を返し、失敗時は err を上に返す
data = fetch_data()?

// 概念的に等価な形式（注意：バリアント分解 match はまだ実装されていない——RFC-010b 配備前は
// コンパイルエラー E3008 になる。`?` が現在利用可能な唯一のエラー伝播記法）
data = match fetch_data() {
    ok(v) => v
    err(e) => return err(e)
}
```

### 1.5 アサーション（std.assert）

`std.assert` モジュールは統一されたアサーション機構を提供する——ランタイム `assert`
とコンパイル時の精緻化型 `Assert` は同じプリミティブの二つの側面である。

```yaoxiang
// IsTrue：値から型へのブリッジ関数
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      // ⊤、プログラム続行
    false => Never,    // ⊥、発散
}

// Assert：コンパイル時精緻化型プリミティブ
Assert: (cond: Bool) -> Type = IsTrue(cond)

// assert：ランタイムアサーション（Assert の値導入子）
assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))

// Result のオーバーロード
assert: (result: Result) -> Assert(IsTrue(is_ok(result)))
```

**dispatch 分配**：

| 条件                                      | 振る舞い                                                  |
| ----------------------------------------- | --------------------------------------------------------- |
| cond の全ての自由変数がコンパイル時に既知 | コンパイラが評価し、true → 消去、false → コンパイルエラー |
| ランタイム自由変数が存在                  | ランタイム check を挿入し、フロー依存仮説集合 Γ を注入    |

`assert(false, "msg")` は raise と等価である——別途の throw/raise キーワードは不要。

---

## 第二章：IO ライブラリ

### 2.1 標準入出力

```yaoxiang
// 標準出力
print: (msg: String) -> Void
println: (msg: String) -> Void

// 標準入力
read_line: () -> String
read_char: () -> Char
```

### 2.2 ファイル操作

```yaoxiang
// ファイル型
File: Type = {
    path: String,
    read: (self: File) -> Result(String, Error),
    write: (self: File, content: String) -> Result(Void, Error),
    append: (self: File, content: String) -> Result(Void, Error),
    close: (self: File) -> Void
}

// ファイル操作
open: (path: String) -> Result(File, Error)
create: (path: String) -> Result(File, Error)
delete: (path: String) -> Result(Void, Error)
```

### 2.3 ディレクトリ操作

```yaoxiang
// ディレクトリ型
Dir: Type = {
    path: String,
    entries: (self: Dir) -> Result(List(String), Error),
    create: (self: Dir) -> Result(Void, Error),
    delete: (self: Dir) -> Result(Void, Error)
}

// ディレクトリ操作
read_dir: (path: String) -> Result(Dir, Error)
create_dir: (path: String) -> Result(Void, Error)
delete_dir: (path: String) -> Result(Void, Error)
```

---

## 第三章：数学ライブラリ

### 3.1 基本数学関数

```yaoxiang
// 絶対値
abs: (x: Int) -> Int
abs: (x: Float) -> Float

// 最大・最小値
max: (a: Int, b: Int) -> Int
min: (a: Int, b: Int) -> Int
max: (a: Float, b: Float) -> Float
min: (a: Float, b: Float) -> Float

// べき乗
pow: (base: Float, exp: Float) -> Float
sqrt: (x: Float) -> Float

// 対数
log: (x: Float) -> Float
log2: (x: Float) -> Float
log10: (x: Float) -> Float
```

### 3.2 三角関数

```yaoxiang
// 三角関数
sin: (x: Float) -> Float
cos: (x: Float) -> Float
tan: (x: Float) -> Float

// 逆三角関数
asin: (x: Float) -> Float
acos: (x: Float) -> Float
atan: (x: Float) -> Float
atan2: (y: Float, x: Float) -> Float
```

### 3.3 定数

```yaoxiang
// 数学定数
pi: Float = 3.141592653589793
e: Float = 2.718281828459045
```

---

## 第四章：文字列ライブラリ

### 4.1 文字列操作

```yaoxiang
// 文字列長
length: (s: String) -> Int

// 文字列連結
concat: (a: String, b: String) -> String

// 文字列分割
split: (s: String, delimiter: String) -> List(String)

// 文字列検索
find: (s: String, pattern: String) -> Option(Int)
contains: (s: String, pattern: String) -> Bool

// 文字列置換
replace: (s: String, old: String, new: String) -> String

// 文字列トリム
trim: (s: String) -> String
trim_left: (s: String) -> String
trim_right: (s: String) -> String
```

### 4.2 文字列変換

```yaoxiang
// 型変換
to_string: (x: Int) -> String
to_string: (x: Float) -> String
to_string: (x: Bool) -> String

// パース
parse_int: (s: String) -> Result(Int, Error)
parse_float: (s: String) -> Result(Float, Error)
```

---

## 第五章：コレクションライブラリ

### 5.1 List 型

```yaoxiang
// List 型
List: (T: Type) -> Type = {
    data: Array(T),
    length: Int,
    push: (T: Type) -> ((self: List(T), item: T) -> Void),
    pop: (T: Type) -> ((self: List(T)) -> Option(T)),
    get: (T: Type) -> ((self: List(T), index: Int) -> Option(T)),
    set: (T: Type) -> ((self: List(T), index: Int, value: T) -> Void),
    insert: (T: Type) -> ((self: List(T), index: Int, item: T) -> Void),
    remove: (T: Type) -> ((self: List(T), index: Int) -> Option(T)),
    clear: (T: Type) -> ((self: List(T)) -> Void),
    contains: (T: Type) -> ((self: List(T), item: T) -> Bool),
    sort: (T: Type) -> ((self: List(T)) -> List(T)),
    reverse: (T: Type) -> ((self: List(T)) -> List(T)),
    map: (T: Type, R: Type) -> ((self: List(T), f: (T) -> R) -> List(R)),
    filter: (T: Type) -> ((self: List(T), predicate: (T) -> Bool) -> List(T)),
    reduce: (T: Type, R: Type) -> ((self: List(T), initial: R, f: (R, T) -> R) -> R)
}
```

### 5.2 Map 型

```yaoxiang
// Map 型
Map: (K: Type, V: Type) -> Type = {
    data: Array((K, V)),
    length: Int,
    insert: (K: Type, V: Type) -> ((self: Map(K, V), key: K, value: V) -> Void),
    get: (K: Type, V: Type) -> ((self: Map(K, V), key: K) -> Option(V)),
    remove: (K: Type, V: Type) -> ((self: Map(K, V), key: K) -> Option(V)),
    contains_key: (K: Type, V: Type) -> ((self: Map(K, V), key: K) -> Bool),
    keys: (K: Type, V: Type) -> ((self: Map(K, V)) -> List(K)),
    values: (K: Type, V: Type) -> ((self: Map(K, V)) -> List(V)),
    clear: (K: Type, V: Type) -> ((self: Map(K, V)) -> Void)
}
```

---

## 第六章：イテレータライブラリ

### 6.1 Iterator trait

```yaoxiang
// Iterator trait
Iterator: (T: Type) -> Type = {
    Item: T,
    next: () -> Option(T),
    has_next: () -> Bool,
    map: (R: Type) -> ((f: (T) -> R) -> Iterator(R)),
    filter: (predicate: (T) -> Bool) -> Iterator(T),
    collect: () -> List(T),
    reduce: (R: Type) -> ((initial: R, f: (R, T) -> R) -> R),
    for_each: (f: (T) -> Void) -> Void
}
```

### 6.2 イテレータアダプタ

```yaoxiang
// 範囲イテレータ（Range は正式な型であり、ランタイム身元は三スカラー不変レコード。
// Tuple シェル不再是借される；`1..10` / `1..10..2` を表示し、構造等価、名前付きフィールド）
Range: Type = {
    start: Int,
    end: Int,
    step: Int,
    Iterator(Int)
}

// 使用法（イテレータプロトコル：std.range.iter/has_next/next、for は静的型で dispatch）
for i in 0..10 {
    print(i)
}

// step 形式（二点、新しいキーワードなし）
for i in 0..10..2 {
    print(i)
}
```

> **`Range(Int)` は正式に実装済み**——名前付きフィールド `r.start`/`r.end`/`r.step` にアクセス可能。
> `x in r` はランタイムで `std.range.contains`
> を経由し（境界チェック + ステップ調整）、証明パイプラインが区間命題
> `x >= r.start && x < r.end && (x - r.start) % r.step == 0`
> として認識する（区間は区間を保持し、実体化しない）。step=0 リテラルはコンパイル時に拒否される。動的 step=0 は Result 化済み：
> `std.range.iter` → `Result(Iterator, Error)`、`std.range.contains` →
> `Result(Bool, Error)`、消費側は `?` で呼び出しスタックに沿って伝播するか、`result.unwrap`
> で明示的に分岐する。`for`/`in`
> 糖衣は ir_gen で展開され、Err 分岐（動的 step=0）は明示的に失敗する（`abort_invalid_step`）。決して無音で無限ループにはならない。インターフェースの実体化（型本体
> `Iterator(Int)` 宣言）の型構文と静的分配は RFC-011a フェーズ 1-2 で実装済み：型本体の適用項目
> `Iterator(Int)` が `Self ↦ Range`
> の置換展開と完全性チェックを起動し、合格後に実装証明を生成する。動的分配はフェーズ 3 で実装済み：インターフェース名が実体化されずに型として存在し（`List(Animal)`）、具体的な値が存在型位置に流入すると自動的にバリアント値としてラップされ、要素メソッド呼び出しは実際の型に従って分配される（§6）。std.range モジュールのランタイムプロトコル面は依然としてネイティブメソッドで提供され、インターフェース分配への移行は今後の作業である。

---

## 付録：標準ライブラリモジュール索引

| モジュール       | 説明                                                                                                    |
| ---------------- | ------------------------------------------------------------------------------------------------------- |
| `std.assert`     | アサーション機構——ランタイム assert + コンパイル時 Assert 精緻化型                                      |
| `std.option`     | Option 型                                                                                               |
| `std.result`     | Result 型                                                                                               |
| `std.collection` | List、Map などのコレクション型                                                                          |
| `std.string`     | 文字列操作                                                                                              |
| `std.array`      | 配列操作                                                                                                |
| `std.iterator`   | イテレータ（プロトコル面は現在 `std.range` が提供）                                                     |
| `std.range`      | Range イテレータと区間述語・アダプタ                                                                    |
| `std.test`       | テストアサーションライブラリ（値セマンティクス、RFC-036 §3）——初の純粋な YaoXiang dogfooding モジュール |

### A.2 IO モジュール

| モジュール | 説明             |
| ---------- | ---------------- |
| `std.io`   | 標準入出力       |
| `std.file` | ファイル操作     |
| `std.dir`  | ディレクトリ操作 |

### A.3 数学モジュール

| モジュール      | 説明     |
| --------------- | -------- |
| `std.math`      | 数学関数 |
| `std.math.trig` | 三角関数 |
| `std.math.log`  | 対数関数 |

### A.4 ユーティリティモジュール

| モジュール   | 説明                                                                    |
| ------------ | ----------------------------------------------------------------------- |
| `std.random` | 乱数生成                                                                |
| `std.time`   | 日時                                                                    |
| `std.assert` | コンパイル時 `Assert(C)` とランタイム `assert(x > 0)` を統一（RFC-030） |
| `std.regex`  | 正規表現                                                                |

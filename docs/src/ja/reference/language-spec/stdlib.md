# 標準ライブラリ仕様

本ファイルは YaoXiang プログラミング言語の標準ライブラリ仕様を定義する。コアライブラリ、IO ライブラリ、数学ライブラリを含む。

---

## 第一章：コアライブラリ

### 1.1 基本型

標準ライブラリは以下の基本型の実装を提供する：

| 型             | モジュール       | 説明           |
| -------------- | ---------------- | -------------- |
| `Option(T)`    | `std.option`     | オプション値型 |
| `Result(T, E)` | `std.result`     | エラー処理型   |
| `List(T)`      | `std.collection` | 動的配列       |
| `Map(K, V)`    | `std.collection` | ハッシュマップ |
| `String`       | `std.string`     | 文字列型       |
| `Array(T, N)`  | `std.array`      | 固定サイズ配列 |

### 1.2 Option 型

```
Option: (T: Type) -> Type = { some: (T) -> Option(T), none: () -> Option(T) }
```

**バリアントコンストラクタ**：

| バリアント    | 構文                 | 説明   |
| ------------- | -------------------- | ------ |
| `Option.some` | `Option.some(value)` | 値あり |
| `Option.none` | `Option.none()`      | 値なし |

**よく使われるメソッド**：

```yaoxiang
// 値を持つか確認
is_some: (self: Option(T)) -> Bool
is_none: (self: Option(T)) -> Bool

// 値を取得（panic の可能性あり）
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

**バリアントコンストラクタ**：

| バリアント   | 構文                | 説明     |
| ------------ | ------------------- | -------- |
| `Result.ok`  | `Result.ok(value)`  | 成功値   |
| `Result.err` | `Result.err(error)` | エラー値 |

**よく使われるメソッド**：

```yaoxiang
// 成功か確認
is_ok: (self: Result(T, E)) -> Bool
is_err: (self: Result(T, E)) -> Bool

// 値を取得（panic の可能性あり）
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
は正規化されたエラーコードを運ぶ。コードは RFC-013 の E6xxx/E7xxx セグメント（例：E6009 =
Range ステップ長不正）を再利用しており、これはバージョン横断の安定契約である——プログラムはコードで判定可能であり、`yx explain E6009`
でドキュメントを検索できる。コードインデックスは RFC-013「実行時エラー値とコード連結」章を参照。

```yaoxiang
// Error 値形式：{ code: String, message: String }

// Err キャリアを取り出す（Ok 時は実行時エラーを報告）
unwrap_err: (T, E) -> ((self: Result(T, E)) -> E)

// エラーコード / メッセージを読み取る
code: (self: Error) -> String
message: (self: Error) -> String
```

**コードによる判定の例**：

```yaoxiang
use std.range
use std.result

r = range.iter(1..10..0)      // step=0 → Err(Error)
if result.is_err(r) {
    e = result.unwrap_err(r)
    if result.code(e) == "E6009" {
        // Range ステップ長不正の分岐で処理
        io.println(result.message(e))
    }
}
```

ユーザー定義のエラー設計は `Result(T, E)`
の E ジェネリクスパラメータ（ユーザー定義のバリアント集合）経由で行い、std の `Error`
は便利なフォールバックキャリアである。コード体系はユーザー E 型を制約しない。

### 1.4 エラー伝播

```
ErrorPropagate ::= Expr '?'
```

`?` 演算子は Result 型のエラーを自動伝播する：

```
// 成功時は値を返し、失敗時は err を上に返す
data = fetch_data()?

// 以下と等価
data = match fetch_data() {
    ok(v) => v
    err(e) => return err(e)
}
```

### 1.5 アサーション（std.assert）

`std.assert` モジュールは統一されたアサーション機構を提供する——実行時 `assert` とコンパイル時精化型
`Assert` は同じプリミティブの二つの側面である。

```yaoxiang
// IsTrue：値から型へのブリッジ関数
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      // ⊤，プログラムは継続
    false => Never,    // ⊥，発散
}

// Assert：コンパイル時精化型プリミティブ
Assert: (cond: Bool) -> Type = IsTrue(cond)

// assert：実行時アサーション（Assert の値導入子）
assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))

// Result オーバーロード
assert: (result: Result) -> Assert(IsTrue(is_ok(result)))
```

**ディスパッチ**：

| 条件                                      | 挙動                                                    |
| ----------------------------------------- | ------------------------------------------------------- |
| cond のすべての自由変数がコンパイル時既知 | コンパイラが評価、true → 消去、false → コンパイルエラー |
| 実行時自由変数が存在する                  | 実行時 check を挿入、フロー依存仮定集合 Γ を注入        |

`assert(false, "msg")` は raise と等価である——個別の throw/raise キーワードは不要。

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

### 3.1 基本的な数学関数

```yaoxiang
// 絶対値
abs: (x: Int) -> Int
abs: (x: Float) -> Float

// 最大値・最小値
max: (a: Int, b: Int) -> Int
min: (a: Int, b: Int) -> Int
max: (a: Float, b: Float) -> Float
min: (a: Float, b: Float) -> Float

// べき乗演算
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
// 文字列の長さ
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
// 範囲イテレータ（Range は正式な型であり、実行時身元は三スカラーの不変レコードである。
// Tuple の外殻を借用しない。`1..10` / `1..10..2` を出力し、構造的等価性を持つ名前付きフィールド）
Range: Type = {
    start: Int,
    end: Int,
    step: Int,
    Iterator(Int)
}

// 使用（イテレータプロトコル：std.range.iter/has_next/next、for は静的型ディスパッチ経由）
for i in 0..10 {
    print(i)
}

// step 形式（二点、新しいキーワードなし）
for i in 0..10..2 {
    print(i)
}
```

> **`Range(Int)` が正式に実装された**——名前付きフィールド `r.start` / `r.end` / `r.step`
> にアクセス可能； `x in r` は実行時に `std.range.contains`
> を経由し（境界チェック + ステップ長アラインメント）、証明パイプラインは区間命題
> `x >= r.start && x < r.end && (x - r.start) % r.step == 0`
> を識別する（区間は区間を保持し、実体化しない）。step=0 リテラルはコンパイル時に拒否される；動的 step=0 は Result 化されている：
> `std.range.iter` → `Result(Iterator, Error)`、`std.range.contains` →
> `Result(Bool, Error)`、消費側は `?` で呼び出しスタックに沿って伝播するか `result.unwrap`
> で明示的に分岐する；`for` / `in`
> の糖衣構文は ir_gen でアンパックされ、Err 分岐（動的 step=0）は明示的に失敗する（`abort_invalid_step`）。決して無音で無限ループしない。インターフェースの実体化（型本体
> `Iterator(Int)`
> 宣言）の型構文と静的ディスパッチは RFC-011a フェーズ 1-2 とともに実装された：型本体適用項
> `Iterator(Int)` が `Self ↦ Range`
> 置換展開と完全性検査をトリガし、合格後に実装証明を生成する。動的ディスパッチはフェーズ 3 とともに実装された：インターフェース名がインスタンス化されずに型として存在し（`List(Animal)`）、具体値が存在型位置に流入すると自動的にバリアント値としてラップされ、要素メソッド呼び出しは実際の型に従ってディスパッチされる（§6）。std.range モジュールの実行時プロトコル面は暫定的にネイティブメソッドで提供されており、インターフェースディスパッチへの移行は今後の作業である。

---

## 付録：標準ライブラリモジュール索引

| モジュール       | 説明                                                                                              |
| ---------------- | ------------------------------------------------------------------------------------------------- |
| `std.assert`     | アサーション機構——実行時 assert + コンパイル時 Assert 精化型                                      |
| `std.option`     | Option 型                                                                                         |
| `std.result`     | Result 型                                                                                         |
| `std.collection` | List、Map などのコレクション型                                                                    |
| `std.string`     | 文字列操作                                                                                        |
| `std.array`      | 配列操作                                                                                          |
| `std.iterator`   | イテレータ（プロトコル面は現状 `std.range` で提供）                                               |
| `std.range`      | Range イテレータと区間述語、アダプタ                                                              |
| `std.test`       | テストアサーションライブラリ（値意味論、RFC-036 §3）——最初の純粋な YaoXiang dogfooding モジュール |

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

| モジュール   | 説明                                                                |
| ------------ | ------------------------------------------------------------------- |
| `std.random` | 乱数生成                                                            |
| `std.time`   | 日時                                                                |
| `std.assert` | コンパイル時 `Assert(C)` と実行時 `assert(x > 0)` の統一（RFC-030） |
| `std.regex`  | 正規表現                                                            |

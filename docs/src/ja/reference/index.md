# YaoXiang リファレンスドキュメント

> 本ドキュメントは建設中です...

YaoXiang は現在 **実験検証段階** にあり、標準ライブラリと API は徐々に整備されています。

## 言語仕様

- [言語仕様概要](./language-spec/index.md)
- [構文仕様](./language-spec/syntax.md) - 語彙構造、構文規則、演算子優先順位
- [type system](./language-spec/type-system.md) - 基本型、複合型、generics、trait
- [モジュールシステム](./language-spec/modules.md) - モジュール定義、インポート/エクスポート、スコープ
- [並行モデル](./language-spec/concurrency.md) - 非同期プログラミング、並行プリミティブ、メモリモデル
- [標準ライブラリ](./language-spec/stdlib.md) - コアライブラリ、IO ライブラリ、数学ライブラリ

## 現状

| モジュール       | 状態      | 説明                     |
| ---------------- | --------- | ------------------------ |
| `std.io`         | 🔨 工事中 | 入力出力                 |
| `std.string`     | 🔨 工事中 | 文字列操作               |
| `std.list`       | 🔨 工事中 | リスト操作               |
| `std.dict`       | ✅ 実装済 | 辞書操作                 |
| `std.range`      | ✅ 実装済 | 範囲とイテレータ（#302） |
| `std.math`       | 🔨 工事中 | 数学関数                 |
| `std.net`        | 📋 計画中 | ネットワーク操作         |
| `std.concurrent` | 📋 計画中 | 並行プリミティブ         |

## 組み込み型

### プリミティブ型

| 型       | 説明            | 例              |
| -------- | --------------- | --------------- |
| `Void`   | void/戻り値なし | `()`            |
| `Bool`   | ブール値        | `true`, `false` |
| `Int`    | 整数            | `42`, `-10`     |
| `Float`  | 浮動小数点数    | `3.14`, `-0.5`  |
| `Char`   | 文字            | `'a'`, `'中'`   |
| `String` | 文字列          | `"hello"`       |

### 複合型

| 型                   | 説明           | 例             |
| -------------------- | -------------- | -------------- |
| `Tuple(T1, T2, ...)` | 異種要素タプル | `(1, "hello")` |
| `(Args) -> Ret`      | 関数型         | `(Int) -> Int` |

> #299：コンテナ型（`List(T)` / `Vec(T)` / `Array(T, N)` /
> `Dict(K, V)`）は組み込みプリミティブではありません。これらは generics 型コンストラクタであり、ユーザー定義 generics と同等の扱いをされ、統一された generics インスタンス化パスを介して処理されます。literal 構文（`[...]`
> /
> `{...}`）はコアに残され、着地点はコンテキスト注釈によって決定されます。Set は削除されました（#300）。詳しくは
> [言語仕様](language-spec/syntax.md) を参照してください。
>
> 3 つのコンテナ概念は長さ情報の帰属によって区別されます：`Array(T, N)`
> は長さが型に含まれます（固定長）、`Vec(T)`
> は長さが runtime の値です（生のバッファプリミティブ）、`List(T)`
> は標準ライブラリの型です（`{ data: Vec(T), length: Int }`、ポリシーはすべてライブラリに）。

### ユーザー定義型

```yaoxiang
// record type（構造体）
Point: Type = { x: Float, y: Float }

// enum type
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// interface type（すべてのフィールドが関数）
Callable: Type = { call: (String) -> Void }
```

## 組み込み関数

### 出力

```yaoxiang
print(value)           // 出力、末尾改行なし
println(value)         // 出力、末尾改行あり
```

### 変換

```yaoxiang
to_string(value)       // 文字列に変換
to_int(value)          // 整数に変換
to_float(value)        // 浮動小数点数に変換
```

### 型チェック

```yaoxiang
typeof(value)         // 型名を返す
is_type(value, type)  // 型をチェック
```

## キーワード

| キーワード                | 説明               |
| ------------------------- | ------------------ |
| `Type`                    | meta type          |
| `spawn`                   | spawn 関数をマーク |
| `spawn for`               | 並列ループ         |
| `spawn {}`                | spawn ブロック     |
| `if` / `else if` / `else` | 条件分岐           |
| `match`                   | pattern matching   |
| `while` / `for`           | ループ             |
| `return`                  | 値を返す           |
| `ref`                     | 参照を作成         |
| `mut`                     | 可変マーク         |

## 構文早見表

### 変数宣言

```yaoxiang
// 不変変数（デフォルト）
x: Int = 42
y = 42                 // type inference

// 可変変数
mut count: Int = 0
count = count + 1
```

### 関数定義

```yaoxiang
// 通常の関数
add: (a: Int, b: Int) -> Int = a + b

// spawn 関数（自動並行）
fetch: (url: String) -> JSON spawn = HTTP.get(url).json()

// generics 関数
identity: [T](x: T) -> T = x
```

### 制御フロー

```yaoxiang
// 条件
if x > 0 {
    print("positive")
} else if x < 0 {
    print("negative")
} else {
    print("zero")
}

// pattern matching
match result {
    ok(value) => print("success: " + value),
    err(error) => print("error: " + error),
}

// ループ
for i in 0..10 {
    print(i)
}
```

### エラー処理

```yaoxiang
// ? 演算子でエラーを伝播
data = fetch_file(path)?
```

## 演算子優先順位

| 優先順位 | 演算子                 |
| -------- | ---------------------- |
| 最高     | `( )` 関数呼び出し     |
|          | `.` フィールドアクセス |
|          | `[ ]` インデックス     |
|          | `unary -` 単項マイナス |
|          | `* / %` 乗除剰余       |
|          | `+ -` 加減             |
|          | `== != < > <= >=` 比較 |
|          | `and or` 論理演算      |
| 最低     | `=` 代入               |

## 標準ライブラリの使用例

```yaoxiang
// 標準ライブラリをインポート
use std.io.{print, println}

// リスト操作
use std.list.{list_push, list_pop, list_len}

// 数学関数
use std.math.{sqrt, sin, cos, PI}

// 使用
println("Hello, YaoXiang!")
result = sqrt(16.0)  // 4.0
```

## コマンドラインツール

```bash
# スクリプトを実行
yx run hello.yx

# バイトコードを構築
yx build hello.yx -o hello.42

# インタプリタ実行
yx eval 'println("Hello")'

# ヘルプを表示
yaoxiang --help
```

## 完全な例

```yaoxiang
use std.convert
use std.io

// フィボナッチ数列を計算
fib: (n: Int) -> Int = if n <= 1 {
    n
} else {
    fib(n - 1) + fib(n - 2)
}

// メイン関数
main: () -> Void = {
    io.println("Fibonacci(10) = " + convert.to_string(fib(10)))
}
```

## 関連リソース

- [チュートリアル](../tutorial/) - YaoXiang を学ぶ
- [設計文書](../design/) - 言語設計の決定
- [GitHub](https://github.com/ChenXu233/YaoXiang)

## 貢献ガイド

標準ライブラリは建設中です。貢献を歓迎します！

1. モジュールを選択（例：`std.io`, `std.net`）
2. `src/std/` で関数を実装
3. ドキュメントコメントを追加
4. PR を送信

# YaoXiang リファレンスドキュメント

> 本ドキュメントは作成中です...

YaoXiang は現在 **実験検証段階** にあり、标准ライブラリと API は逐步的に整備中です。

## 言語仕様

- [言語仕様概要](./language-spec/index.md)
- [構文仕様](./language-spec/syntax.md) - 字句構造、構文規則、演算子の優先順位
- [型システム](./language-spec/type-system.md) - 基本型、複合型、ジェネリクス、trait
- [モジュールシステム](./language-spec/modules.md) - モジュール定義、インポート/エクスポート、スコープ
- [並行モデル](./language-spec/concurrency.md) - 非同期プログラミング、並行プリミティブ、メモリモデル
- [標準ライブラリ](./language-spec/stdlib.md) - コアライブラリ、IOライブラリ、数学ライブラリ

## 現在の状態

| モジュール | 状態 | 説明 |
|------|------|------|
| `std.io` | 🔨 施工中 | 入出力 |
| `std.string` | 🔨 施工中 | 文字列操作 |
| `std.list` | 🔨 施工中 | リスト操作 |
| `std.dict` | 📋 計画中 | 辞書操作 |
| `std.math` | 🔨 施工中 | 数学関数 |
| `std.net` | 📋 計画中 | ネットワーク操作 |
| `std.concurrent` | 📋 計画中 | 並行プリミティブ |

## 組み込み型

### プリミティブ型

| 型 | 説明 | 例 |
|------|------|------|
| `Void` | 空値/戻り値なし | `()` |
| `Bool` | 真理値 | `true`, `false` |
| `Int` | 整数 | `42`, `-10` |
| `Float` | 浮動小数点数 | `3.14`, `-0.5` |
| `Char` | 文字 | `'a'`, `'中'` |
| `String` | 文字列 | `"hello"` |

### 複合型

| 型 | 説明 | 例 |
|------|------|------|
| `List[T]` | 同種要素のリスト | `[1, 2, 3]` |
| `Tuple(T1, T2, ...)` | 異種要素のタプル | `(1, "hello")` |
| `Dict[K, V]` | キーと値のペアによるマップ | `{"a": 1}` |
| `Fn(Args) -> Ret` | 関数型 | `(Int) -> Int` |

### ユーザー定義型

```yaoxiang
// レコード型（ストラクチャ）
Point: Type = { x: Float, y: Float }

// 列挙型
Result: Type[T, E] = { ok(T) | err(E) }

// インターフェース型（すべてのフィールドが関数）
Callable: Type = { call: (String) -> Void }
```

## 組み込み関数

### 出力

```yaoxiang
print(value)           // 改行なしで出力
println(value)         // 改行ありで出力
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

| キーワード | 説明 |
|--------|------|
| `Type` | メタ型 |
| `spawn` | spawn関数をマーク |
| `spawn for` | 並列ループ |
| `spawn {}` | spawnブロック |
| `if` / `elif` / `else` | 条件分岐 |
| `match` | パターンマッチング |
| `while` / `for` | ループ |
| `return` | 戻り値 |
| `ref` | 参照を作成 |
| `mut` | 変更可能マーク |

## 構文早見表

### 変数の宣言

```yaoxiang
// 不変変数（デフォルト）
x: Int = 42
y = 42                 // 型推論

// 可変変数
mut count: Int = 0
count = count + 1
```

### 関数の定義

```yaoxiang
// 通常関数
add: (a: Int, b: Int) -> Int = a + b

// spawn関数（自動並行化）
fetch: (url: String) -> JSON spawn = HTTP.get(url).json()

// ジェネリクス関数
identity: [T](x: T) -> T = x
```

### 制御フロー

```yaoxiang
// 条件
if x > 0 {
    println("positive")
} elif x < 0 {
    println("negative")
} else {
    println("zero")
}

// パターンマッチング
match result {
    ok(value) => println("success: " + value),
    err(error) => println("error: " + error),
}

// ループ
for i in 0..10 {
    print(i)
}
```

### エラー処理

```yaoxiang
// ? 演算子によるエラー伝播
data = fetch_file(path)?
```

## 演算子の優先順位

| 優先順位 | 演算子 |
|--------|--------|
| 最高 | `( )` 関数呼び出し |
| | `.` フィールドアクセス |
| | `[ ]` インデックス |
| | `unary -` 単項負号 |
| | `* / %` 乗算/除算/剰余 |
| | `+ -` 加算/減算 |
| | `== != < > <= >=` 比較 |
| | `and or` 論理演算 |
| 最低 | `=` 代入 |

## 標準ライブラリの使用例

```yaoxiang
// 標準ライブラリのインポート
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
# スクリプトの実行
yaoxiang run hello.yx

# バイトコードのビルド
yaoxiang build hello.yx -o hello.42

# インタープリタ実行
yaoxiang eval 'println("Hello")'

# ヘルプの表示
yaoxiang --help
```

## 完全な例

```yaoxiang
// フィボナッチ数列の計算
fib: (n: Int) -> Int = if n <= 1 {
    n
} else {
    fib(n - 1) + fib(n - 2)
}

// メイン関数
main: () -> Void = {
    println("Fibonacci(10) = " + fib(10).to_string())
}
```

## 関連リソース

- [チュートリアル](../tutorial/) - YaoXiang の学習
- [設計ドキュメント](../design/) - 言語設計の決定事項
- [GitHub](https://github.com/ChenXu233/YaoXiang)

## 貢献ガイドライン

標準ライブラリは作成中です。ご貢献を歓迎します！

1. モジュールを選択する（`std.io`、`std.net` など）
2. `src/std/` で関数を実装する
3. ドキュメントコメントを追加する
4. PR を提交する
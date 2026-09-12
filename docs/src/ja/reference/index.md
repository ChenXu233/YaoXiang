# YaoXiang リファレンスドキュメント

> 本ドキュメントは建設中です...

YaoXiang は現在 **実験検証段階** にあり、標準ライブラリと API は徐々に整備されています。

## 言語仕様

- [言語仕様概要](./language-spec/index.md)
- [構文仕様](./language-spec/syntax.md) - 語彙構造、構文規則、演算子の優先順位
- [型システム](./language-spec/type-system.md) - 基本型、複合型、generics、trait
- [モジュールシステム](./language-spec/modules.md) - モジュール定義、インポート/エクスポート、スコープ
- [並行モデル](./language-spec/concurrency.md) - 非同期プログラミング、並行プリミティブ、メモリモデル
- [標準ライブラリ](./language-spec/stdlib.md) - コアライブラリ、IOライブラリ、数学ライブラリ

## 現在のステータス

| モジュール       | ステータス  | 説明                     |
| ---------------- | ----------- | ------------------------ |
| `std.io`         | 🔨 建設中   | 入出力                   |
| `std.string`     | 🔨 建設中   | 文字列操作               |
| `std.list`       | 🔨 建設中   | リスト操作               |
| `std.dict`       | ✅ 実装済み | 辞書操作                 |
| `std.range`      | ✅ 実装済み | 範囲とイテレータ（#302） |
| `std.math`       | 🔨 建設中   | 数学関数                 |
| `std.net`        | 📋 計画中   | ネットワーク操作         |
| `std.concurrent` | 📋 計画中   | 並行プリミティブ         |

## 組み込み型

### 原始类型

| 型       | 説明            | 例              |
| -------- | --------------- | --------------- |
| `Void`   | 空値/戻り値なし | `()`            |
| `Bool`   | ブール値        | `true`, `false` |
| `Int`    | 整数            | `42`, `-10`     |
| `Float`  | 浮動小数点数    | `3.14`, `-0.5`  |
| `Char`   | 文字            | `'a'`, `'中'`   |
| `String` | 文字列          | `"hello"`       |

### 複合型

| 型                   | 説明             | 例             |
| -------------------- | ---------------- | -------------- |
| `Tuple(T1, T2, ...)` | 異種要素のタプル | `(1, "hello")` |
| `(Args) -> Ret`      | 関数型           | `(Int) -> Int` |

> #299: コンテナ型（`List(T)` / `Array(T, N)` /
> `Dict(K, V)`）は組み込み原語ではなく、汎用型構築子です。ユーザーが定義した汎用型と同じ扱いとなり、統一された汎用インスタンス化パスによって処理されます。リテラル構文（`[...]`
> /
> `{...}`）はコアに残され、着地点はコンテキスト注釈によって決定されます。Set は削除されました（#300）、詳細は
> [言語仕様](language-spec/syntax.md) を参照してください。

### ユーザー定義型

```yaoxiang
// 记录类型（结构体）
Point: Type = { x: Float, y: Float }

// 枚举类型
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// 接口类型（所有字段为函数）
Callable: Type = { call: (String) -> Void }
```

## 組み込み関数

### 出力

```yaoxiang
print(value)           // 打印，无换行
println(value)         // 打印，有换行
```

### 変換

```yaoxiang
to_string(value)       // 转换为字符串
to_int(value)          // 转换为整数
to_float(value)        // 转换为浮点数
```

### 型チェック

```yaoxiang
typeof(value)         // 返回类型名称
is_type(value, type)  // 检查类型
```

## キーワード

| キーワード                | 説明               |
| ------------------------- | ------------------ |
| `Type`                    | 元タイプ           |
| `spawn`                   | spawn 関数をマーク |
| `spawn for`               | 並列ループ         |
| `spawn {}`                | spawn ブロック     |
| `if` / `else if` / `else` | 条件分岐           |
| `match`                   | パターン照合       |
| `while` / `for`           | ループ             |
| `return`                  | 戻り値             |
| `ref`                     | 参照の作成         |
| `mut`                     | 可変マーキング     |

## 構文早見表

### 変数宣言

```yaoxiang
// 不可变变量（默认）
x: Int = 42
y = 42                 // 类型推断

// 可变变量
mut count: Int = 0
count = count + 1
```

### 関数定義

```yaoxiang
// 普通函数
add: (a: Int, b: Int) -> Int = a + b

// 并作函数（自动并发）
fetch: (url: String) -> JSON spawn = HTTP.get(url).json()

// 泛型函数
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

// 模式匹配
match result {
    ok(value) => print("success: " + value),
    err(error) => print("error: " + error),
}

// 循环
for i in 0..10 {
    print(i)
}
```

### エラー処理

```yaoxiang
// ? 运算符传播错误
data = fetch_file(path)?
```

## 演算子の優先順位

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
// 导入标准库
use std.io.{print, println}

// 列表操作
use std.list.{list_push, list_pop, list_len}

// 数学函数
use std.math.{sqrt, sin, cos, PI}

// 使用
println("Hello, YaoXiang!")
result = sqrt(16.0)  // 4.0
```

## コマンドラインツール

```bash
# 运行脚本
yx run hello.yx

# 构建字节码
yx build hello.yx -o hello.42

# 解释执行
yx eval 'println("Hello")'

# 查看帮助
yaoxiang --help
```

## 完全な例

```yaoxiang
// 计算斐波那契数列
fib: (n: Int) -> Int = if n <= 1 {
    n
} else {
    fib(n - 1) + fib(n - 2)
}

// 主函数
main: () -> Void = {
    print("Fibonacci(10) = " + fib(10).to_string())
}
```

## 関連リソース

- [チュートリアル](../tutorial/) - YaoXiang を学ぶ
- [設計ドキュメント](../design/) - 言語設計の決定事項
- [GitHub](https://github.com/ChenXu233/YaoXiang)

## コントリビューションガイド

標準ライブラリは建設中です、コントリビューションを歓迎します！

1. モジュールを選択する（例: `std.io`, `std.net`）
2. `src/std/` に関数を実装する
3. ドキュメントコメントを追加する
4. PR を提出する

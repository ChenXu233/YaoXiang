# YaoXiang リファレンスドキュメント

> このドキュメントは作成中です...

YaoXiangは現在**実験検証段階**にあり、標準ライブラリとAPIは段階的に整備されています。

## 言語仕様

- [言語仕様概要](./language-spec/index.md)
- [構文仕様](./language-spec/syntax.md) - 字句構造、構文規則、演算子の優先順位
- [type system](./language-spec/type-system.md) - 基本型、複合型、generics、trait
- [モジュールシステム](./language-spec/modules.md) - モジュール定義、インポート/エクスポート、スコープ
- [並行モデル](./language-spec/concurrency.md) - 非同期プログラミング、並行プリミティブ、メモリモデル
- [標準ライブラリ](./language-spec/stdlib.md) - コアライブラリ、IOライブラリ、数学ライブラリ

## 現状

| モジュール       | 状態        | 説明                     |
| ---------------- | ----------- | ------------------------ |
| `std.io`         | 🔨 作成中   | 入力出力                 |
| `std.string`     | 🔨 作成中   | 文字列操作               |
| `std.list`       | 🔨 作成中   | リスト操作               |
| `std.dict`       | ✅ 実装済み | 辞書操作                 |
| `std.range`      | ✅ 実装済み | 範囲とイテレータ（#302） |
| `std.math`       | 🔨 作成中   | 数学関数                 |
| `std.net`        | 📋 計画中   | ネットワーク操作         |
| `std.concurrent` | 📋 計画中   | 並行プリミティブ         |

## 組み込み型

### primitive type

| 型       | 説明            | 例              |
| -------- | --------------- | --------------- |
| `Void`   | void/戻り値なし | `()`            |
| `Bool`   | boolean type    | `true`, `false` |
| `Int`    | integer type    | `42`, `-10`     |
| `Float`  | float type      | `3.14`, `-0.5`  |
| `Char`   | 文字            | `'a'`, `'中'`   |
| `String` | string type     | `"hello"`       |

### 複合型

| 型                   | 説明                 | 例             |
| -------------------- | -------------------- | -------------- |
| `Tuple(T1, T2, ...)` | 異種要素のtuple type | `(1, "hello")` |
| `(Args) -> Ret`      | function type        | `(Int) -> Int` |

> #299：コンテナ型（`List(T)` / `Vec(T)` / `Array(T, N)` /
> `Dict(K, V)`）は組み込みのprimitiveではなく、ユーザー定義のgenericsと同等の扱いを受けるgenericsの型コンストラクタであり、統一されたgenericsインスタンス化パスで処理される。literal構文（`[...]`
> /
> `{...}`）はコアに保持され、解決先はコンテキストの注釈によって決定される。Setは削除された（#300）、詳細は[言語仕様](language-spec/syntax.md)を参照。
>
> 3つのコンテナ概念は長さ情報の所属によって区別される：`Array(T, N)`は長さが型に含まれる（固定長）、
> `Vec(T)`は長さがruntime
> value（生バッファprimitive）、`List(T)`は標準ライブラリ型（`{ data: Vec(T), length: Int }`、ポリシーはすべてライブラリ内）。

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

| キーワード                | 説明                 |
| ------------------------- | -------------------- |
| `Type`                    | meta type            |
| `spawn`                   | spawn functionを示す |
| `spawn for`               | spawn loop           |
| `spawn {}`                | spawn block          |
| `if` / `else if` / `else` | 条件分岐             |
| `match`                   | pattern matching     |
| `while` / `for`           | ループ               |
| `return`                  | 戻り値               |
| `ref`                     | 参照を作成           |
| `mut`                     | mutマーク            |

## 構文クイックリファレンス

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

- [チュートリアル](../tutorial/) - YaoXiangを学ぶ
- [設計ドキュメント](../design/) - 言語設計の決定
- [GitHub](https://github.com/ChenXu233/YaoXiang)

## 貢献ガイド

標準ライブラリは作成中で、貢献を歓迎します！

1. モジュールを選択（例：`std.io`, `std.net`）
2. `src/std/`に関数を実装
3. ドキュメントコメントを追加
4. PRを提出

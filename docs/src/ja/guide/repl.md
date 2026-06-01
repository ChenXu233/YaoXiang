---
title: REPL インタラクティブインタープリタ
description: YaoXiang REPL 使い方ガイド - インタラクティブコード実行環境
---

# REPL インタラクティブインタープリタ

YaoXiang REPL（Read-Eval-Print Loop）は、コード行を入力して実行できるインタラクティブなコード実行環境です。学习、テスト、デバッグに最適です。

## クイックスタート

### REPL の起動

ターミナルで以下のコマンドを実行して REPL を起動します：

```bash
yaoxiang repl
```

または、サブコマンドなしで直接 `yaoxiang` を実行します：

```bash
yaoxiang
```

起動すると、プロンプトが表示されます：

```
YaoXiang REPL - Type :help for assistance
Press Ctrl+D or :quit to exit

>>
```

### 基本使用法

プロンプト `>>` の後に YaoXiang コードを入力して Enter を押すと実行されます：

```rust
>> 1 + 2
3

>> "Hello, World!"
"Hello, World!"

>> let x = 10
>> x * 2
20
```

### REPL の終了

REPL を終了する方法は3通りです：

1. **ショートカット**： `Ctrl+D` を押す
2. **コマンド**： `:quit` または `:q` を入力
3. **中断**： `Ctrl+C` で現在の入力を中断

## コマンドシステム

REPL はコロン `:` で始まる特別なコマンドを提供します。

### ヘルプコマンド

```rust
>> :help
```

すべての使用可能なコマンドのヘルプ情報を表示します。

### 終了コマンド

```rust
>> :quit
```

REPL を終了します。省略形の `:q` も使用できます。

### クリアコマンド

```rust
>> :clear
```

定義済みのすべての変数と関数をクリアし、REPL 状態をリセットします。省略形の `:c` も使用できます。

### 型查看コマンド

```rust
>> :type x
```

シンボル `x` の型情報を表示します。省略形の `:t` も使用できます。

**例**：

```rust
>> let name = "YaoXiang"
>> :type name
name: String

>> fn add(a: Int, b: Int) -> Int = a + b
>> :type add
add: fn(Int, Int) -> Int
```

### シンボルリストコマンド

```rust
>> :symbols
```

現在の REPL で定義されているすべてのシンボル（変数と関数）をリスト表示します。省略形の `:i` または `:info` も使用できます。

**例**：

```rust
>> let x = 10
>> let y = 20
>> fn greet(name: String) -> String = "Hello, " + name
>> :symbols
x: Int
y: Int
greet: fn(String) -> String
```

### ヒストリコマンド

```rust
>> :history
```

コマンド履歴を表示します。省略形の `:hist` も使用できます。

### 統計コマンド

```rust
>> :stats
```

実行統計情報を表示します。評価回数と合計実行時間を含みます。

**例**：

```rust
>> :stats
Eval count: 5
Total time: 12.34ms
```

## コード実行

### 式実行

REPL は有効な YaoXiang 式を実行できます：

```rust
>> 1 + 2
3

>> 10 * 5 + 3
53

>> "Hello" + " " + "World"
"Hello World"

>> true && false
false
```

### 変数定義

`let` キーワードを使用して変数を定義します：

```rust
>> let name = "YaoXiang"
>> let age = 25
>> let pi = 3.14159
```

定義した変数は後続のコードで使用できます：

```rust
>> name
"YaoXiang"

>> age + 5
30
```

### 関数定義

`fn` キーワードを使用して関数を定義します：

```rust
>> fn add(a: Int, b: Int) -> Int = a + b
>> fn greet(name: String) -> String = "Hello, " + name
```

関数の呼び出し：

```rust
>> add(3, 4)
7

>> greet("World")
"Hello World"
```

### 複数行コード

REPL は複数行コード入力をサポートします。コードが不完全な場合（閉じ括弧がないなど）、自動的に継続行モードに入ります：

```rust
>> fn factorial(n: Int) -> Int =
..   if n <= 1 then 1
..   else n * factorial(n - 1)
```

継続行プロンプトは `..` で、現在の複数行入力モードを示します。

### 構造体定義

```rust
>> struct Point {
..   x: Float,
..   y: Float
.. }
```

### 列挙型定義

```rust
>> enum Color {
..   Red,
..   Green,
..   Blue
.. }
```

## 自動補完

REPL はインテリジェントな自動補完機能を提供し、高速なコード入力を支援します。

### トリガー方法

`Tab` キーを押して自動補完をトリガーします。

### 補完内容

1. **キーワード補完**：YaoXiang 言語キーワード
   - `let`, `fn`, `if`, `else`, `match`, `for`, `while`, `return` など

2. **変数補完**：定義済みの変数
   - 変数名の最初の数文字を入力し、Tab で補完

3. **関数補完**：定義済みの関数
   - 関数名の最初の数文字を入力し、Tab で補完

4. **組込み関数補完**：組込み関数
   - `print`, `len`, `range`, `typeof`, `assert` など

### 補完例

```rust
>> let my_variable = 42
>> my_<Tab>
my_variable: Int

>> fn calculate_sum(a: Int, b: Int) -> Int = a + b
>> calc<Tab>
calculate_sum: fn(Int, Int) -> Int
```

## 高度な機能

### エラー処理

コードにエラーがある場合、REPL は詳細なエラー情報を表示します：

```rust
>> let x = 10 / 0
Error: Runtime error: DivisionByZero

>> undefined_variable
Error: Unknown symbol: undefined_variable
```

エラーは REPL セッションを終了しません。新しいコードの入力を続けることができます。

### 履歴記録

REPL は自動的にコマンド履歴を保存し、以下をサポートします：

- **上下矢印**：履歴コマンドの閲覧
- **検索**：部分入力をしてから上下矢印で検索
- **履歴ファイル**：履歴はファイルに保存され、次回起動時に自動ロード

### 実行統計

`:stats` コマンドを使用して実行統計を確認：

```rust
>> :stats
Eval count: 15
Total time: 45.67ms
```

これはコードパフォーマンスの監視に役立ちます。

## ベストプラクティス

### 1. 意味のある変数名を使用する

```rust
// 良い例
let user_name = "YaoXiang"
let max_retries = 3

// 悪い例
let x = "YaoXiang"
let n = 3
```

### 2. 関数を定義してコードを再利用

```rust
>> fn is_even(n: Int) -> Bool = n % 2 == 0
>> is_even(4)
true
>> is_even(7)
false
```

### 3. `:clear` を使用して状態をリセット

REPL 状態が混乱している場合は、 `:clear` を使用してリセット：

```rust
>> :clear
Context cleared
```

### 4. 自動補完を活用して効率を向上

最初の数文字を入力してから Tab を押して、変数名と関数名を素早く補完します。

### 5. 複数行入力を活用して複雑なコードを処理

```rust
>> fn fibonacci(n: Int) -> Int =
..   if n <= 1 then n
..   else fibonacci(n - 1) + fibonacci(n - 2)
```

## よくある質問

### Q: ある関数の定義を查看方法は？

A: `:type` コマンドを使用して関数シグネチャを查看：

```rust
>> :type my_function
my_function: fn(Int, String) -> Bool
```

### Q: すべての定義をクリアする方法は？

A: `:clear` コマンドを使用：

```rust
>> :clear
```

### Q: 複数行コードが実行されない理由は？

A: 閉じられていない括弧、引用符、波括弧がないか確認してください。REPL は完全なコード入力を待ちます。

### Q: 長時間実行中のコードを中断する方法は？

A: `Ctrl+C` を押して現在の実行を中断します。

### Q: REPL はどのデータ型をサポートしていますか？

A: REPL は YaoXiang のすべてのデータ型をサポートしています：
- `Int`：整数
- `Float`：浮動小数点数
- `String`：文字列
- `Bool`：真理値
- `Unit`：ユニット型
- カスタム構造体と列挙型

## サンプルセッション

完全な REPL セッションの例：

```rust
YaoXiang REPL - Type :help for assistance
Press Ctrl+D or :quit to exit

>> let greeting = "Hello"
>> let name = "YaoXiang"
>> greeting + ", " + name + "!"
"Hello, YaoXiang!"

>> fn factorial(n: Int) -> Int =
..   if n <= 1 then 1
..   else n * factorial(n - 1)
..
>> factorial(5)
120

>> :symbols
greeting: String
name: String
factorial: fn(Int) -> Int

>> :stats
Eval count: 4
Total time: 2.34ms

>> :quit
```

## 関連コマンド

| コマンド | 省略形 | 機能 |
|------|------|------|
| `:help` | `:h` | ヘルプ情報を表示 |
| `:quit` | `:q` | REPL を終了 |
| `:clear` | `:c` | すべての状態をクリア |
| `:type` | `:t` | シンボル型を查看 |
| `:symbols` | `:i` | すべてのシンボルをリスト表示 |
| `:history` | `:hist` | コマンド履歴を表示 |
| `:stats` | - | 実行統計を表示 |
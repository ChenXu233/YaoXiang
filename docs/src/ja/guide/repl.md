---
title: REPL インタラクティブインタープリタ
description: YaoXiang REPL 使い方ガイド - インタラクティブコード実行環境
---

# REPL インタラクティブインタープリタ

YaoXiang REPL（Read-Eval-Print
Loop）はインタラクティブなコード実行環境で、YaoXiang コードを1行ずつ入力・実行でき、学習、テスト、デバッグに最適です。

## クイックスタート

### REPL の起動

ターミナルで次のコマンドを実行して REPL を起動します：

```bash
yaoxiang repl
```

またはサブコマンドなしで直接 `yaoxiang` を実行します：

```bash
yaoxiang
```

起動すると、プロンプトが表示されます：

```
YaoXiang REPL - Type :help for assistance
Press Ctrl+D or :quit to exit

>>
```

### 基本的な使用方法

`>>` プロンプトの後に YaoXiang コードを入力して Enter を押すと実行されます：

```yaoxiang
>> 1 + 2
3

>> "Hello, World!"
"Hello, World!"

>> x = 10
>> x * 2
20
```

### REPL の終了

REPL を終了するには3つの方法があります：

1. **ショートカット**：`Ctrl+D` を押す
2. **コマンド**：`:quit` または `:q` を入力
3. **中断**：`Ctrl+C` で現在の入力を中断

## コマンドシステム

REPL はコロン `:` で始まる特殊コマンドを提供します。

### ヘルプコマンド

```yaoxiang
>> :help
```

すべての利用可能なコマンドのヘルプ情報を表示します。

### 終了コマンド

```yaoxiang
>> :quit
```

REPL を終了します。省略形の `:q` も使用できます。

### クリアコマンド

```yaoxiang
>> :clear
```

すべての定義済み変数と関数を消去し、REPL 状態をリセットします。省略形の `:c` も使用できます。

### 型確認コマンド

```yaoxiang
>> :type x
```

シンボル `x` の型情報を表示します。省略形の `:t` も使用できます。

**例**：

```yaoxiang
>> name = "YaoXiang"
>> :type name
name: String

>> add: (a: Int, b: Int) -> Int = a + b
>> :type add
add: fn(Int, Int) -> Int
```

### シンボル一覧コマンド

```yaoxiang
>> :symbols
```

現在の REPL で定義されたすべてのシンボル（変数と関数）を表示します。省略形の `:i` または `:info` も使用できます。

**例**：

```yaoxiang
>> x = 10
>> y = 20
>> greet: (name: String) -> String = "Hello, " + name
>> :symbols
x: Int
y: Int
greet: fn(String) -> String
```

### 履歴コマンド

```yaoxiang
>> :history
```

コマンド履歴を表示します。省略形の `:hist` も使用できます。

### 統計コマンド

```yaoxiang
>> :stats
```

実行統計情報を表示します（評価回数と合計実行時間を含む）。

**例**：

```yaoxiang
>> :stats
Eval count: 5
Total time: 12.34ms
```

## コード実行

### 式を実行する

REPL は任意の有効な YaoXiang 式を実行できます：

```yaoxiang
>> 1 + 2
3

>> 10 * 5 + 3
53

>> "Hello" + " " + "World"
"Hello World"

>> true && false
false
```

### 変数の定義

変数名を直接使用して変数を定義します：

```yaoxiang
>> name = "YaoXiang"
>> age = 25
>> pi = 3.14159
```

型を明示的に注釈することもできます：

```yaoxiang
>> name: String = "YaoXiang"
>> age: Int = 25
```

定義後、変数は後続のコードで使用できます：

```yaoxiang
>> name
"YaoXiang"

>> age + 5
30
```

### 関数の定義

YaoXiang には `fn` キーワードはなく、関数は署名付きの値です：

```yaoxiang
>> add: (a: Int, b: Int) -> Int = a + b
>> greet: (name: String) -> String = "Hello, " + name
```

関数の呼び出し：

```yaoxiang
>> add(3, 4)
7

>> greet("World")
"Hello World"
```

### 複数行コード

REPL は複数行コード入力をサポートしています。コードが不完全な場合（閉じ括弧がないなど）、自動的に續行モードに入ります：

```yaoxiang
>> factorial: (n: Int) -> Int = {
..     if n <= 1 { return 1 }
..     return n * factorial(n - 1)
.. }
```

續行プロンプトは `..` で、現在の複数行入力モードを示します。

### 型定義

```yaoxiang
>> Point: Type = { x: Float, y: Float }
```

### バリアント型定義（列挙型）

```yaoxiang
>> Color: Type = { red: () -> Color, green: () -> Color, blue: () -> Color }
```

## 自動補完

REPL はインテリジェントな自動補完機能を提供し、コードの素早い入力を助けます。

### トリガー方法

`Tab` キーを押して自動補完をトリガーします。

### 補完内容

1. **キーワード補完**：YaoXiang 言語キーワード（Tab で展開可能）
2. **シンボル補完**：定義済みの変数名と関数名
3. **組み込み関数補完**：`print`、`len`、`range`、`typeof`、`assert` などの組み込み関数

### 補完の例

```yaoxiang
>> my_variable = 42
>> my_<Tab>
my_variable: Int

>> calculate_sum: (a: Int, b: Int) -> Int = a + b
>> calc<Tab>
calculate_sum: fn(Int, Int) -> Int
```

## 高度な機能

### エラー処理

コードにエラーがあると、REPL は詳細なエラー情報を表示します：

```yaoxiang
>> x = 10 / 0
Error: Runtime error: DivisionByZero

>> undefined_variable
Error: Unknown symbol: undefined_variable
```

エラーは REPL セッションを終了させるものではなく、新しいコードの入力を続けることができます。

### 履歴記録

REPL は自動的にコマンド履歴を保存し、以下をサポートしています：

- **上下矢印**：履歴コマンドの閲覧
- **検索**：部分的に入力してから上下矢印で検索
- **履歴ファイル**：履歴はファイルに保存され、次回起動時に自動的にロード

### 実行統計

`:stats` コマンドを使用して実行統計を表示します：

```yaoxiang
>> :stats
Eval count: 15
Total time: 45.67ms
```

これはコードのパフォーマンスを監視するのに役立ちます。

## ベストプラクティス

### 1. 意味のある変数名を使用する

```yaoxiang
// 良い例
user_name = "YaoXiang"
max_retries = 3

// 悪い例
x = "YaoXiang"
n = 3
```

### 2. 関数を定義してコードを再利用する

```yaoxiang
>> is_even: (n: Int) -> Bool = n % 2 == 0
>> is_even(4)
true
>> is_even(7)
false
```

### 3. `:clear` を使用して状態をリセットする

REPL 状態が混乱している場合は、`:clear` を使用してリセットします：

```yaoxiang
>> :clear
Context cleared
```

### 4. 自動補完を活用して効率を上げる

最初の数文字を入力してから Tab を押し、変数名と関数名を素早く補完します。

### 5. 複数行入力を活用して複雑なコードを処理する

```yaoxiang
>> fibonacci: (n: Int) -> Int = {
..     if n <= 1 { return n }
..     return fibonacci(n - 1) + fibonacci(n - 2)
.. }
```

## よくある質問

### Q: ある関数の定義を確認するには？

A: `:type` コマンドを使用して関数シグネチャを確認します：

```yaoxiang
>> :type my_function
my_function: fn(Int, String) -> Bool
```

### Q: すべての定義を消去するには？

A: `:clear` コマンドを使用します：

```yaoxiang
>> :clear
```

### Q: 複数行コードが実行されないのはなぜですか？

A: 閉じていない括弧、引用符、波括弧がないか確認してください。REPL は完全なコード入力を待ちます。

### Q: 長時間実行中のコードを中断するには？

A: `Ctrl+C` を押して現在の実行を中断します。

### Q: REPL は哪些数据类型をサポートしていますか？

A: REPL はすべての YaoXiang データ型をサポートしています：

- `Int`：整数
- `Float`：浮動小数点数
- `String`：文字列
- `Bool`：真偽値
- `Void`：空タイプ
- カスタムレコード型とバリアント型

## サンプルセッション

完全な REPL セッションの例：

```yaoxiang
YaoXiang REPL - Type :help for assistance
Press Ctrl+D or :quit to exit

>> greeting = "Hello"
>> name = "YaoXiang"
>> greeting + ", " + name + "!"
"Hello, YaoXiang!"

>> factorial: (n: Int) -> Int = {
..     if n <= 1 { return 1 }
..     return n * factorial(n - 1)
.. }
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

| コマンド     | 省略形  | 機能             |
| ---------- | ------ | -------------- |
| `:help`    | `:h`   | ヘルプ情報を表示   |
| `:quit`    | `:q`   | REPL を終了      |
| `:clear`   | `:c`   | すべての状態をクリア |
| `:type`    | `:t`   | シンボルの型を表示   |
| `:symbols` | `:i`   | すべてのシンボルを一覧表示 |
| `:history` | `:hist`| コマンド履歴を表示   |
| `:stats`   | -      | 実行統計を表示     |

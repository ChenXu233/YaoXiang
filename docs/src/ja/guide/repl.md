---
title: REPL インタラクティブインタプリタ
description: YaoXiang REPL 使用ガイド - インタラクティブなコード実行環境
---

# REPL インタラクティブインタプリタ

YaoXiang REPL（Read-Eval-Print
Loop）は、YaoXiang コードを行ごとに入力・実行できるインタラクティブなコード実行環境で、学習、テスト、デバッグに最適です。

## はじめに

### REPL の起動

ターミナルで以下のコマンドを実行して REPL を起動します：

```bash
yaoxiang repl
```

または、サブコマンドを指定せずに `yaoxiang` を直接実行します：

```bash
yaoxiang
```

起動後、以下のようなプロンプトが表示されます：

```
YaoXiang REPL - Type :help for assistance
Press Ctrl+D or :quit to exit

>>
```

### 基本的な使い方

プロンプト `>>` の後に YaoXiang コードを入力して Enter キーで実行します：

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

REPL を終了するには 3 つの方法があります：

1. **ショートカットキー**：`Ctrl+D` を押す
2. **コマンド**：`:quit` または `:q` を入力する
3. **中断**：`Ctrl+C` を押して現在の入力を中断する

## コマンドシステム

REPL はコロン `:` で始まる一連の特殊コマンドを提供します。

### ヘルプコマンド

```yaoxiang
>> :help
```

利用可能なすべてのコマンドのヘルプ情報を表示します。

### 終了コマンド

```yaoxiang
>> :quit
```

REPL を終了します。短縮形 `:q` も使用できます。

### クリアコマンド

```yaoxiang
>> :clear
```

定義済みのすべての変数と関数をクリアし、REPL の状態をリセットします。短縮形 `:c` も使用できます。

### 型表示コマンド

```yaoxiang
>> :type x
```

シンボル `x` の型情報を表示します。短縮形 `:t` も使用できます。

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

現在の REPL で定義済みのすべてのシンボル（変数と関数）を一覧表示します。短縮形 `:i` または `:info`
も使用できます。

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

コマンド履歴を表示します。短縮形 `:hist` も使用できます。

### 統計コマンド

```yaoxiang
>> :stats
```

評価回数や合計実行時間など、実行統計情報を表示します。

**例**：

```yaoxiang
>> :stats
Eval count: 5
Total time: 12.34ms
```

## コード実行

### 式の実行

REPL は有効な YaoXiang 式であれば何でも実行できます：

```yaoxiang
>> 1 + 2
3

>> 10 * 5 + 3
53

>> "Hello" + " " + "World"
"Hello World"

>> true and false
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

YaoXiang には `fn` キーワードがありません。関数は単なるシグネチャ付きの値です：

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

REPL は複数行コードの入力に対応しています。コードが不完全な場合（閉じられていない括弧など）、自動的に継続行モードに入ります：

```yaoxiang
>> factorial: (n: Int) -> Int = {
..     if n <= 1 { return 1 }
..     return n * factorial(n - 1)
.. }
```

継続行プロンプトは `..` で、複数行入力モードであることを示します。

### 型の定義

```yaoxiang
>> Point: Type = { x: Float, y: Float }
```

### バリアント型（enum）の定義

```yaoxiang
>> Color: Type = { red: () -> Color, green: () -> Color, blue: () -> Color }
```

## 自動補完

REPL はインテリジェントな自動補完機能を提供し、コードの迅速な入力を支援します。

### 起動方法

`Tab` キーを押して自動補完を起動します。

### 補完内容

1. **キーワード補完**：YaoXiang 言語のキーワード（Tab で展開可能）
2. **シンボル補完**：定義済みの変数名と関数名
3. **組み込み関数補完**：`print`, `len`, `range`, `typeof`, `assert` などの組み込み関数

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

### エラーハンドリング

コードにエラーがある場合、REPL は詳細なエラー情報を表示します：

```yaoxiang
>> x = 10 / 0
Error: Runtime error: DivisionByZero

>> undefined_variable
Error: Unknown symbol: undefined_variable
```

エラーは REPL セッションを終了させないため、新しいコードを入力し続けることができます。

### 履歴

REPL は自動的にコマンド履歴を保存し、以下の機能をサポートします：

- **上下矢印キー**：履歴コマンドの閲覧
- **検索**：一部を入力してから上下矢印キーで検索
- **履歴ファイル**：履歴はファイルに保存され、次回起動時に自動的に読み込まれる

### 実行統計

`:stats` コマンドで実行統計を確認します：

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

### 3. `:clear` で状態をリセットする

REPL の状態が混乱した場合は、`:clear` を使用してリセットします：

```yaoxiang
>> :clear
Context cleared
```

### 4. 自動補完を活用して効率を上げる

最初の数文字を入力してから Tab を押し、変数名や関数名を迅速に補完します。

### 5. 複数行入力で複雑なコードに対応する

```yaoxiang
>> fibonacci: (n: Int) -> Int = {
..     if n <= 1 { return n }
..     return fibonacci(n - 1) + fibonacci(n - 2)
.. }
```

## よくある質問

### Q: ある関数の定義を確認するにはどうすればよいですか？

A: `:type` コマンドを使用して関数のシグネチャを確認します：

```yaoxiang
>> :type my_function
my_function: fn(Int, String) -> Bool
```

### Q: すべての定義をクリアするにはどうすればよいですか？

A: `:clear` コマンドを使用します：

```yaoxiang
>> :clear
```

### Q: 複数行コードが実行されないのはなぜですか？

A: 閉じられていない括弧、引用符、または中括弧がないか確認してください。REPL は完全なコード入力を待ちます。

### Q: 長時間実行されているコードを中断するにはどうすればよいですか？

A: `Ctrl+C` を押して現在の実行を中断します。

### Q: REPL はどのデータ型をサポートしていますか？

A: REPL はすべての YaoXiang データ型をサポートします：

- `Int`：整数
- `Float`：浮動小数点数
- `String`：文字列
- `Bool`：ブール値
- `Void`：空の型
- カスタム record type と enum type

## サンプルセッション

以下は完全な REPL セッションの例です：

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

| コマンド   | 短縮形  | 機能                 |
| ---------- | ------- | -------------------- |
| `:help`    | `:h`    | ヘルプ情報を表示     |
| `:quit`    | `:q`    | REPL を終了          |
| `:clear`   | `:c`    | すべての状態をクリア |
| `:type`    | `:t`    | シンボルの型を表示   |
| `:symbols` | `:i`    | すべてのシンボル一覧 |
| `:history` | `:hist` | コマンド履歴を表示   |
| `:stats`   | -       | 実行統計を表示       |

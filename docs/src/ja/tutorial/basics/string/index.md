---
title: 'F-string'
---

# F-string

f-string は YaoXiang における**テンプレート文字列**です。文字列内に直接変数や式を埋め込むことができ、コンパイラが自動的に型変換と連結を行います。

## 基本用法

文字列の前に `f` 接頭辞を付け、`{式}` で値を挿入します：

```yaoxiang
name = "Alice"
age = 25

greeting = f"Hello {name}, you are {age} years old"
print(greeting)  // Hello Alice, you are 25 years old
```

従来の連結方式と比較すると、f-string との違いは一目瞭然です：

```yaoxiang
// ❌ 従来の連結：冗長で間違いやすい
message = "Hello ".concat(name).concat(", age: ").concat(age.to_string())

// ✅ f-string：直感的で簡潔
message = f"Hello {name}, age: {age}"
```

## 式の補間

`{}` には変数だけでなく、任意の式を入れることができます：

```yaoxiang
x = 10
y = 20

print(f"Sum: {x + y}")         // Sum: 30
print(f"Product: {x * y}")     // Product: 200
print(f"Is positive? {x > 0}") // Is positive? true
```

## フォーマット指定子

式の後に `:` とフォーマット指定子を追加して、出力形式を制御します：

```yaoxiang
pi = 3.14159265

print(f"Pi: {pi}")       // Pi: 3.14159265
print(f"Pi: {pi:.2f}")   // Pi: 3.14（小数点以下2桁）
print(f"Pi: {pi:.4f}")   // Pi: 3.1416（小数点以下4桁）
```

よく使われるフォーマット指定子：

| 指定子 | 意味            | 例                 | 出力           |
| ------ | --------------- | ------------------ | -------------- |
| `:.2f` | 浮動小数点、2桁 | `f"{3.14159:.2f}"` | `3.14`         |
| `:d`   | 10進整数        | `f"{42:d}"`        | `42`           |
| `:x`   | 16進数          | `f"{255:x}"`       | `ff`           |
| `:e`   | 科学的記数法    | `f"{1000:e}"`      | `1.000000e+03` |
| `:s`   | 文字列          | `f"{name:s}"`      | `hello`        |

## メソッド呼び出し

`{}` 内でメソッドを呼び出すことができます：

```yaoxiang
name = "alice"

print(f"Upper: {name.uppercase()}")   // Upper: ALICE
print(f"Length: {name.len()}")        // Length: 5
```

## 中括弧のエスケープ

リテラルの `{` または `}` を出力する必要がある場合は、**二重に**記述します：

```yaoxiang
print(f"{{literal braces}}")     // {literal braces}
print(f"Set: {{1, 2, 3}}")       // Set: {1, 2, 3}

// 混合：二重でリテラル { を出力、単一で補間
name = "YaoXiang"
print(f"{{name}} is {name}")     // {name} is YaoXiang
```

## 複数行 f-string

f-string は複数行にまたがることができます：

```yaoxiang
name = "Alice"
age = 25
city = "Beijing"

info = f"""
Name: {name}
Age: {age}
City: {city}
"""

print(info)
// Name: Alice
// Age: 25
// City: Beijing
```

## f-string の仕組み

コンパイラが f-string を検出すると、効率的な文字列連結に変換します：

```yaoxiang
// あなたが書いたコード
f"Hello {name}, age: {age}"

// コンパイラの変換結果
"Hello ".concat(name.to_string()).concat(", age: ").concat(age.to_string())
```

これは、f-string が単に書き方が簡潔になるだけでなく、実行時の性能も手書きの連結と同等であることを意味します——**追加のオーバーヘッドはゼロ**です。

## まとめ

:::: v-pre

| 要点             | 構文                       |
| ---------------- | -------------------------- |
| 基本補間         | `f"text {var}"`            |
| 式               | `f"result: {x + y}"`       |
| フォーマット     | `f"value: {pi:.2f}"`       |
| 括弧のエスケープ | `f"{{not interpolation}}"` |
| 複数行           | `f"""..."""`               |

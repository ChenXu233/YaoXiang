---
title: F-string
---

# F-string

f-string は YaoXiang の**テンプレート文字列**——文字列内に直接変数や式を埋め込み、コンパイラが自動的に型変換と連結を行います。

## 基本的な使い方

文字列の前に `f` プレフィックスを付け、`{式}` で値を挿入します：

```yaoxiang
name = "Alice"
age = 25

greeting = f"Hello {name}, you are {age} years old"
print(greeting)  // Hello Alice, you are 25 years old
```

従来の連結方式と比較すると、f-string の利点は明白です：

```yaoxiang
// ❌ 従来の連結：冗長でエラーしやすい
message = "Hello ".concat(name).concat(", age: ").concat(age.to_string())

// ✅ f-string：直感的で简洁
message = f"Hello {name}, age: {age}"
```

## 式による補間

`{}` 内には変数だけでなく、任意の式を置くことができます：

```yaoxiang
x = 10
y = 20

print(f"Sum: {x + y}")         // Sum: 30
print(f"Product: {x * y}")     // Product: 200
print(f"Is positive? {x > 0}") // Is positive? true
```

## フォーマット指定子

式の後に `:` とフォーマット指定子を追加して、出力フォーマットを制御できます：

```yaoxiang
pi = 3.14159265

print(f"Pi: {pi}")       // Pi: 3.14159265
print(f"Pi: {pi:.2f}")   // Pi: 3.14（小数第2位まで）
print(f"Pi: {pi:.4f}")   // Pi: 3.1416（小数第4位まで）
```

よく使うフォーマット指定子：

| 指定子 | 意味           | 例                | 出力           |
| ------ | -------------- | ----------------- | -------------- |
| `:.2f` | 浮動小数点、2桁 | `f"{3.14159:.2f}"` | `3.14`         |
| `:d`   | 10進整数       | `f"{42:d}"`       | `42`           |
| `:x`   | 16進数         | `f"{255:x}"`      | `ff`           |
| `:e`   | 指数表記       | `f"{1000:e}"`     | `1.000000e+03` |
| `:s`   | 文字列         | `f"{name:s}"`     | `hello`        |

## メソッドの呼び出し

`{}` 内でメソッドを呼び出すことができます：

```yaoxiang
name = "alice"

print(f"Upper: {name.uppercase()}")   // Upper: ALICE
print(f"Length: {name.len()}")        // Length: 5
```

## 波括弧のエスケープ

リテラルの `{` や `}` を出力する必要がある場合は、**2回続けて記述**します：

```yaoxiang
print(f"{{literal braces}}")     // {literal braces}
print(f"Set: {{1, 2, 3}}")       // Set: {1, 2, 3}

// 混合：2回でリテラル {、1回は補間
name = "YaoXiang"
print(f"{{name}} is {name}")     // {name} is YaoXiang
```

## 複数行 f-string

f-string は複数行にまたがることはできません：

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

コンパイラは f-string を見ると、高效な文字列連結に変換します：

```yaoxiang
// 書いたコード
f"Hello {name}, age: {age}"

// コンパイラによる変換結果
"Hello ".concat(name.to_string()).concat(", age: ").concat(age.to_string())
```

つまり、f-string は書く的时候就更简洁，而且运行时性能也与手写拼接相当——**零额外开销**。

## まとめ

:::: v-pre

| 要点       | 構文                       |
| ---------- | -------------------------- |
| 基本補間   | `f"text {var}"`            |
| 式         | `f"result: {x + y}"`       |
| 書式設定   | `f"value: {pi:.2f}"`       |
| 波括弧のエスケープ | `f"{{not interpolation}}"` |
| 複数行     | `f"""..."""`               |

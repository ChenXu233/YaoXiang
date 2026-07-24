---
title: "関数関連のフォーマットルール"
description: 関数定義、関数呼び出し、Lambda式のフォーマットルール
---

# 関数関連のフォーマットルール

---

## §4 関数定義

**§4.1 関数シグネチャ。** 関数名と引数リストの間にスペースを入れない。

```
// ✅ 正确
foo: (a: Int, b: Int) -> Int = a + b

// ❌ 错误
foo : (a: Int, b: Int) -> Int = a + b
```

**§4.2 引数リストの改行。** 引数リストが行幅を超える場合、各引数を1行に配置し、末尾にコンマを付ける。

```
// 超过行宽时
very_long_function_name: (first_param: Int, second_param: Int, third_param: Int) -> Int = first_param + second_param + third_param

// 格式化后
very_long_function_name:
    first_param: Int,
    second_param: Int,
    third_param: Int,
) -> Int = first_param + second_param + third_param
```

**§4.3 戻り値の型。** 戻り値の型と引数リストは ` -> ` で接続し、`->` の前後にそれぞれ1つのスペースを入れる。

```
// ✅ 正确
foo: () -> Int = 1

// ❌ 错误
foo: () ->Int = 1
foo: ()-> Int = 1
foo:()-> Int = 1
```

**§4.4 関数の本体。** 関数の本体と戻り値の型の間は1つのスペースで区切る。

```
// ✅ 正确
foo: () -> Int = 1

// ❌ 错误（两个空格）
foo: () -> Int  = 1
```

---

## §7 関数呼び出し

**§7.1 引数リスト。** 引数はコンマで区切り、コンマの後に1つのスペースを入れる。

```
// ✅ 正确
foo(1, 2, 3)

// ❌ 错误
foo(1,2,3)
foo(1 , 2 , 3)
```

**§7.2 名前付き引数。** 名前付き引数は `name = value` 形式を使用する。

```
// ✅ 正确
foo(x = 1, y = 2)

// ❌ 错误
foo(x=1, y=2)
```

**§7.3 引数の改行。** 引数リストが行幅を超える場合、各引数を1行に配置し、末尾にコンマを付ける。

```
// 超过行宽时
very_long_function_name(first_argument, second_argument, third_argument)

// 格式化后
very_long_function_name(
    first_argument,
    second_argument,
    third_argument,
)
```

---

## §12 Lambda式

**§12.1 Lambda形式。** Lambdaは `(params) => body` 形式を使用する。

```
// ✅ 正确
f = (x) => x + 1

// 单表达式 body
f = (x) => x * 2

// 多语句 body
f = (x) => {
    y = x + 1
    y * 2
}
```